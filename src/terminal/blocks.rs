//! Command blocks: the tracer's per-pane table of `B`→`D` spans.
//!
//! The daemon owns the truth table ([`CommandBlock`], paired the same way),
//! and this is the client's live mirror, built from the same marks as the
//! reader parses them. Fold, jump and copy key off local spans only — a span's
//! `seq` and absolute rows — while the daemon table merely enriches spans
//! with `daemon_id`/`truncated` through [`BlockTracker::adopt`], in close
//! order, best effort. Nothing routes on those ids yet (entry-6 defines the
//! epoch protocol); they are audit, not addressing.
//!
//! [`CommandBlock`]: tty7_core::daemon::protocol::CommandBlock

use std::collections::HashSet;

use alacritty_terminal::event::EventListener;
use alacritty_terminal::grid::Dimensions as _;
use alacritty_terminal::index::{Column, Line, Point};
use alacritty_terminal::term::Term;

use tty7_core::daemon::protocol::CommandBlock;

/// Shared ownership of a pane's [`BlockTracker`]: the reader builds it, the
/// view folds, jumps, copies and adopts into it.
pub type BlockStore = std::sync::Arc<std::sync::Mutex<BlockTracker>>;

/// Reads the exit code out of an OSC 133 `D` payload (`133;D;130` → 130).
/// `None` for a bare `D` or an unparsable code — still a finish, just with an
/// unknown status.
pub fn parse_d_exit_code(payload: &[u8]) -> Option<i32> {
    payload
        .strip_prefix(b"133;D;")
        .and_then(|c| std::str::from_utf8(c).ok())
        .and_then(|s| s.trim().parse::<i32>().ok())
}

/// How many closed spans a pane keeps. Same order as the daemon's
/// `COMMAND_BLOCK_CAP`, so close-order pairing against its table stays
/// aligned: both sides evict oldest-first at the same magnitude.
pub const BLOCK_SPAN_CAP: usize = 200;

/// One closed command run, in absolute grid rows.
///
/// Absolute rows count from the grid's birth (`history_size + line`), so a
/// span stays put while output scrolls. A grid reset (clear, relink) rebirths
/// them, which is why [`BlockTracker::clear_blocks`] drops every span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockSpan {
    /// Local order among spans. Fold, jump and copy key off this plus the
    /// absolute rows — never off `daemon_id`.
    pub seq: u64,
    /// Absolute row of the `B` mark (the prompt row the command ran from).
    pub start_abs: i64,
    /// Absolute row of the `D` mark (the cursor row it landed on).
    pub end_abs: i64,
    /// The `D` mark's exit status, when it carried one.
    pub exit_code: Option<i32>,
    /// Best-effort enrichment from the daemon's table; `None` until adopted.
    pub daemon_id: Option<u64>,
    /// Best-effort enrichment from the daemon's table; always `false` until
    /// the daemon can honestly say otherwise.
    pub truncated: bool,
    /// The start row's text as it stood when the span closed — the fingerprint
    /// [`BlockTracker::verify_all`] re-reads. Empty when the row was already
    /// gone, which is itself proof of eviction.
    pub fp: String,
}

impl BlockSpan {
    /// How many grid rows the span covers, first line included.
    pub fn len(&self) -> u64 {
        self.end_abs
            .saturating_sub(self.start_abs)
            .saturating_add(1) as u64
    }

    /// The exit line closing a block copy — `exit <code>`, alone on its row.
    pub fn exit_line(&self) -> String {
        match self.exit_code {
            Some(code) => format!("exit {code}"),
            None => "exit ?".to_string(),
        }
    }

    /// Whether the span closed with a failure — any non-zero exit status,
    /// including an interrupt (Ctrl-C surfaces as 130). A missing status is
    /// unknown, not failure, so it never reddens.
    pub fn is_failed(&self) -> bool {
        self.exit_code.is_some_and(|code| code != 0)
    }
}

/// The pane's spans plus its fold state, shared between the reader thread
/// (which opens/closes spans) and the UI thread (which folds, jumps, copies
/// and adopts daemon ids).
#[derive(Debug, Default)]
pub struct BlockTracker {
    spans: Vec<BlockSpan>,
    next_seq: u64,
    /// Absolute row of the pending `B`, if a command is running.
    open: Option<i64>,
    /// Folded spans, by `seq`. Fold state is view-local UI state: it rides
    /// the pane (not the view) so a relink keeps it, and it keys off `seq`
    /// so eviction below drops it with its span.
    folded: HashSet<u64>,
    /// Spans whose local fold the daemon table has not echoed back yet
    /// (entry-5, CAP-5). A toggle lands locally first and rides
    /// fire-and-forget to the daemon; until an adoption sees the daemon row
    /// agree, the refresh below keeps the local value instead of clobbering
    /// it with the stale row — so a fold made while the daemon is
    /// unreachable (or its message still in flight) survives the next
    /// adoption instead of flickering off. Keyed off `seq`, dropped with
    /// its span like [`BlockTracker::folded`](Self::folded).
    dirty: HashSet<u64>,
    /// Highest daemon id consumed (or seen across a resync). Pairing only
    /// ever moves forward from here.
    watermark: u64,
    /// Armed by [`BlockTracker::clear_blocks`]: the next [`BlockTracker::adopt`]
    /// only advances the watermark past the stale snapshot instead of pairing
    /// new spans against pre-clear daemon ids.
    resync: bool,
}

impl BlockTracker {
    /// A `B` mark landed on `abs`: a command started. Re-arms a pending open
    /// (a redrawn prompt is still one command run, mirroring the daemon).
    pub fn note_b(&mut self, abs: i64) {
        self.open = Some(abs);
    }

    /// The pending `B`'s absolute row, if a command is running — so the caller
    /// can fingerprint the rows the span will be verified against.
    pub fn open_start(&self) -> Option<i64> {
        self.open
    }

    /// A `D` mark landed on `abs`: the pending command finished. With no
    /// pending `B` there is nothing to close — the shell's own startup mark,
    /// or a replay that starts mid-command.
    ///
    /// `fp` is the start row's text as it stands now (see [`row_text`]);
    /// `None` when the row is already gone, which [`verify_all`](Self::verify_all)
    /// will read as eviction.
    ///
    /// `C` never appears here on purpose: it is commentary (the command's
    /// text), and commands run with no `133;C` at all, so `D` closes for any
    /// pending `B`, exactly like the daemon.
    pub fn note_d(&mut self, abs: i64, exit_code: Option<i32>, fp: Option<String>) {
        let Some(start) = self.open.take() else {
            return;
        };
        // Marks arrive in stream order, so `end` never precedes `start` —
        // but a grid reset between the two (clear, relink) rebirths absolute
        // rows, and clamping beats recording a backwards span.
        let (start_abs, end_abs) = (start.min(abs), start.max(abs));
        self.next_seq += 1;
        self.spans.push(BlockSpan {
            seq: self.next_seq,
            start_abs,
            end_abs,
            exit_code,
            daemon_id: None,
            truncated: false,
            fp: fp.unwrap_or_default(),
        });
        while self.spans.len() > BLOCK_SPAN_CAP {
            let evicted = self.spans.remove(0);
            self.folded.remove(&evicted.seq);
            self.dirty.remove(&evicted.seq);
        }
    }

    /// Re-reads every span's start row through `row_text`: any change — or a
    /// row the grid no longer holds — proves scroll-cap eviction aliased
    /// absolute rows (history sticks at the cap while surviving content
    /// renumbers, so fresh rows reuse forgotten numbers) or a reset the clear
    /// path missed. Either way every span may point at the wrong rows, so
    /// the table clears through the clear path ([`BlockTracker::clear_blocks`]:
    /// spans drop, resync arms) instead of folding and copying wrong rows.
    /// Returns whether every span verified.
    ///
    /// History rows are immutable, so a mismatch is eviction, not editing —
    /// with one benign exception: a span whose start row is still the live
    /// cursor row can be rewritten by the next output, and then reads as a
    /// mismatch. That only hides the block (safe direction), never corrupts.
    pub fn verify_all(&mut self, row_text: &dyn Fn(i64) -> Option<String>) -> bool {
        let ok = self
            .spans
            .iter()
            .all(|s| row_text(s.start_abs).as_deref() == Some(s.fp.as_str()));
        if !ok {
            self.clear_blocks();
        }
        ok
    }

    /// Drops every span fully above `top_abs` — the scroll-limit eviction
    /// already forgot those rows, so spans pointing at them are stale.
    pub fn prune_before(&mut self, top_abs: i64) {
        if self.spans.is_empty() {
            return;
        }
        let mut dropped = Vec::new();
        self.spans.retain(|span| {
            let keep = span.end_abs >= top_abs;
            if !keep {
                dropped.push(span.seq);
            }
            keep
        });
        for seq in dropped {
            self.folded.remove(&seq);
            self.dirty.remove(&seq);
        }
    }

    /// Drops every span: the grid was reset (clear, relink) and absolute rows
    /// were rebirthed. Arms the resync — the next [`BlockTracker::adopt`]
    /// only chases the watermark past the stale snapshot instead of pairing
    /// new spans against pre-clear daemon ids. The watermark itself is kept:
    /// it names daemon ids, which outlive the grid.
    pub fn clear_blocks(&mut self) {
        self.spans.clear();
        self.folded.clear();
        self.dirty.clear();
        self.open = None;
        self.resync = true;
    }

    /// Drops spans for a grid reset that is immediately rebuilt from a replay
    /// (relink): unlike [`BlockTracker::clear_blocks`] the watermark is kept
    /// and no resync is armed — the replayed spans are the table's tail, and
    /// the next adoption should pair them.
    pub fn reset_for_relink(&mut self) {
        self.spans.clear();
        self.folded.clear();
        self.dirty.clear();
        self.open = None;
    }

    /// Enriches unpaired spans with the daemon table's ids. Pairs newest to
    /// newest: the local spans are (up to eviction) the pane's latest
    /// commands, and so are the table's tail — which is what makes a relink
    /// (whose replay rebuilds only the recent spans) land on the right ids
    /// instead of the table's head. Best effort: a span the daemon evicted
    /// (or never saw) simply keeps `daemon_id: None`, and nothing routes on
    /// the ids either way. Idempotent — adopting the same snapshot twice
    /// pairs nothing new.
    ///
    /// Folded flags (entry-5, CAP-5) ride the same call: spans already paired
    /// by an earlier adopt refresh their fold from the daemon row (the daemon
    /// is truth — a restart restores the same folded rows this way), except
    /// spans with a newer local toggle the daemon has not echoed back yet
    /// (see `dirty`): those keep the local value so an unacked toggle does
    /// not flicker off on the next adoption. Newly paired spans keep their
    /// local fold for the same reason. Returns the newly paired spans whose
    /// local fold disagrees with the daemon row, as
    /// `(seq, daemon_id, local_folded)`, so the caller can push them up and
    /// converge the daemon to what the GUI shows.
    pub fn adopt(&mut self, blocks: &[CommandBlock]) -> Vec<(u64, u64, bool)> {
        let max_id = blocks.iter().map(|b| b.id).max().unwrap_or(self.watermark);
        if self.resync {
            // Post-clear: the snapshot may still hold pre-clear rows the GUI
            // no longer shows — chase the watermark past them, pair nothing.
            self.watermark = self.watermark.max(max_id);
            self.resync = false;
            return Vec::new();
        }
        // Refresh already-paired spans from the daemon table first: the rows
        // are keyed by id, so eviction elsewhere in the table cannot shift
        // them. A span with a local toggle the daemon has not echoed back
        // keeps the local value — the push is in flight (or the daemon
        // unreachable), and clobbering it would flicker the user's fold off.
        for span in self.spans.iter_mut() {
            let Some(id) = span.daemon_id else {
                continue;
            };
            if let Some(row) = blocks.iter().find(|b| b.id == id) {
                span.truncated = row.truncated;
                let local = self.folded.contains(&span.seq);
                if row.folded == local {
                    self.dirty.remove(&span.seq);
                } else if !self.dirty.contains(&span.seq) {
                    match row.folded {
                        true => self.folded.insert(span.seq),
                        false => self.folded.remove(&span.seq),
                    };
                }
            }
        }
        let mut fresh: Vec<&CommandBlock> =
            blocks.iter().filter(|b| b.id > self.watermark).collect();
        fresh.sort_by_key(|b| b.id);
        let unpaired: Vec<usize> = self
            .spans
            .iter()
            .enumerate()
            .filter(|(_, s)| s.daemon_id.is_none())
            .map(|(i, _)| i)
            .collect();
        // Suffix pairing: the newest unpaired span takes the newest fresh id.
        let n = unpaired.len().min(fresh.len());
        let mut newly = Vec::with_capacity(n);
        for (span_idx, block) in unpaired[unpaired.len() - n..]
            .iter()
            .zip(&fresh[fresh.len() - n..])
        {
            let span = &mut self.spans[*span_idx];
            span.daemon_id = Some(block.id);
            span.truncated = block.truncated;
            // Newly paired: a local fold the daemon row disagrees with stays
            // dirty and is reported so the caller pushes it up; an agreeing
            // pair is already converged.
            let local = self.folded.contains(&span.seq);
            if local == block.folded {
                self.dirty.remove(&span.seq);
            } else {
                self.dirty.insert(span.seq);
                newly.push((span.seq, block.id, local));
            }
        }
        self.watermark = self.watermark.max(max_id);
        newly
    }

    /// The closed span covering `abs`, if any. A still-running command (open,
    /// no `D` yet) is not a block: no exit line, no fold, no menu.
    pub fn span_at(&self, abs: i64) -> Option<&BlockSpan> {
        self.spans
            .iter()
            .rev()
            .find(|s| s.start_abs <= abs && abs <= s.end_abs)
    }

    /// Whether any span is folded.
    pub fn has_folds(&self) -> bool {
        !self.folded.is_empty()
    }

    /// Whether `seq` is folded.
    pub fn is_folded(&self, seq: u64) -> bool {
        self.folded.contains(&seq)
    }

    /// Folds (`true`) or unfolds a closed span. Returns the new state, or
    /// `None` when `seq` names no closed span. A successful toggle marks the
    /// span dirty (entry-5, CAP-5): the daemon learns about it
    /// fire-and-forget, so adoptions keep the local value until the daemon
    /// row echoes it back.
    pub fn set_folded(&mut self, seq: u64, folded: bool) -> Option<bool> {
        if !self.spans.iter().any(|s| s.seq == seq) {
            return None;
        }
        match folded {
            true => self.folded.insert(seq),
            false => self.folded.remove(&seq),
        };
        self.dirty.insert(seq);
        Some(folded)
    }

    /// Whether `abs` is hidden inside a folded span (past its first line).
    fn is_hidden(&self, abs: i64) -> bool {
        self.spans
            .iter()
            .any(|s| self.folded.contains(&s.seq) && s.start_abs < abs && abs <= s.end_abs)
    }

    /// The absolute rows to paint, top to bottom, for a window whose bottom
    /// row is `bottom_abs` showing `rows` rows.
    ///
    /// Folded spans collapse to their first line; the freed rows pull earlier
    /// lines into view (the window bottom stays put, so the prompt does too).
    /// Near the grid's birth there may be fewer than `rows` lines — the
    /// caller leaves the rest blank. With nothing folded this is exactly the
    /// identity window, so callers can skip the mapping then.
    pub fn map_visible(&self, bottom_abs: i64, rows: usize) -> Vec<i64> {
        let mut out = Vec::with_capacity(rows);
        let mut abs = bottom_abs;
        while out.len() < rows && abs >= 0 {
            if !self.is_hidden(abs) {
                out.push(abs);
            }
            abs -= 1;
        }
        out.reverse();
        out
    }

    /// Every closed span, oldest first — for the gutter and for tests.
    pub fn spans(&self) -> &[BlockSpan] {
        &self.spans
    }
}

/// Absolute row of the grid's topmost row: spans ending above this point at
/// rows the scroll-limit eviction already forgot.
pub fn prune_top_abs<T: EventListener>(term: &Term<T>) -> i64 {
    term.grid().history_size() as i64 + term.topmost_line().0 as i64
}

/// The text of the grid row at `abs`, or `None` when the grid no longer
/// holds it — the fingerprint behind span verification.
pub fn row_text<T: EventListener>(term: &Term<T>, abs: i64) -> Option<String> {
    use alacritty_terminal::grid::Dimensions as _;

    let history = term.grid().history_size() as i64;
    let line = abs - history;
    if line < term.topmost_line().0 as i64 || line > term.bottommost_line().0 as i64 {
        return None;
    }
    Some(term.bounds_to_string(
        Point::new(Line(line as i32), Column(0)),
        Point::new(Line(line as i32), term.last_column()),
    ))
}

/// Absolute grid row of the cursor: `history_size + line`. Cursor lines are
/// grid coordinates (the paint path maps them with `+ display_offset`), so
/// `display_offset` must NOT enter here — a `D` landing mid-scroll would
/// otherwise understate the anchor by the scroll distance and the fold would
/// leak rows while the copy dropped trailing ones.
pub fn cut_anchor<T: EventListener>(term: &Term<T>) -> i64 {
    term.grid().history_size() as i64 + term.grid().cursor.point.line.0 as i64
}

/// Absolute rows of the window's top and bottom rows under the identity
/// mapping: `[history - display_offset, …)`, `rows` rows long.
pub fn window_abs_range<T: EventListener>(term: &Term<T>, rows: usize) -> (i64, i64) {
    let history = term.grid().history_size() as i64;
    let top = history - term.grid().display_offset() as i64;
    (top, top + rows as i64 - 1)
}

/// Visible-row mapping for a window of `rows` rows, or `None` when the
/// identity mapping holds (nothing folded, or no fold intersecting the
/// window). Bottom-anchored: the window's bottom row stays put, so folding
/// pulls earlier lines into view and the prompt stays where it was.
pub fn visible_map<T: EventListener>(
    term: &Term<T>,
    tracker: &BlockTracker,
    rows: usize,
) -> Option<Vec<i64>> {
    if !tracker.has_folds() {
        return None;
    }
    use alacritty_terminal::grid::Dimensions as _;

    let history = term.grid().history_size() as i64;
    let (top, bottom) = window_abs_range(term, rows);
    // The window can name rows the scroll-limit eviction already forgot
    // (absolute row 0 is the grid's birth, not its memory): only mapped rows
    // the grid still holds may paint. Dropping from the front keeps the
    // bottom-anchored alignment — the caller blanks what is left short.
    let bottom = bottom.min(history + term.bottommost_line().0 as i64);
    let floor = history + term.topmost_line().0 as i64;
    let mut map = tracker.map_visible(bottom, rows);
    map.retain(|&abs| abs >= floor);
    if map.len() == rows && map.first() == Some(&top) {
        return None;
    }
    Some(map)
}

/// A screen row through an optional [`visible_map`]: `None` for the blank
/// rows a short map leaves at the top (near the grid's birth).
pub fn screen_to_abs(row: usize, rows: usize, top: i64, map: Option<&[i64]>) -> Option<i64> {
    match map {
        None => Some(top + row as i64),
        Some(m) => {
            let skip = rows.saturating_sub(m.len());
            row.checked_sub(skip).and_then(|i| m.get(i).copied())
        }
    }
}

/// The one-line summary a folded span collapses to: its first line plus the
/// row count and exit line. Truncated to `cols` display columns (wide chars
/// count by [`unicode_width`]).
pub fn fold_summary<T: EventListener>(term: &Term<T>, span: &BlockSpan, cols: usize) -> String {
    use alacritty_terminal::grid::Dimensions as _;

    let history = term.grid().history_size() as i64;
    let line = (span.start_abs - history).clamp(
        term.topmost_line().0 as i64,
        term.bottommost_line().0 as i64,
    );
    let first = term
        .bounds_to_string(
            Point::new(Line(line as i32), Column(0)),
            Point::new(Line(line as i32), term.last_column()),
        )
        .trim()
        .to_string();
    let mut summary = String::from("▸ ");
    if !first.is_empty() {
        summary.push_str(&first);
        summary.push_str(" · ");
    }
    summary.push_str(&format!("{} lines · {}", span.len(), span.exit_line()));
    truncate_cols(&summary, cols.max(1))
}

/// Truncates to `cols` display columns, closing with `…` when cut.
fn truncate_cols(text: &str, cols: usize) -> String {
    use unicode_width::UnicodeWidthStr as _;
    if text.width() <= cols {
        return text.to_string();
    }
    let mut out = String::new();
    let mut width = 0;
    for ch in text.chars() {
        let w = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if width + w > cols.saturating_sub(1) {
            break;
        }
        out.push(ch);
        width += w;
    }
    out.push('…');
    out
}

/// The plain grid text of `span`'s rows, without the exit line — the shared
/// source every block-copy item reads through. `None` when the grid no longer
/// holds the span's rows.
fn block_grid_text<T: EventListener>(
    term: &Term<T>,
    span: &BlockSpan,
    trim_trailing: bool,
) -> Option<String> {
    use alacritty_terminal::grid::Dimensions as _;

    let history = term.grid().history_size() as i64;
    let top = term.topmost_line().0 as i64;
    let bottom = term.bottommost_line().0 as i64;
    let start = (span.start_abs - history).clamp(top, bottom);
    let end = (span.end_abs - history).clamp(top, bottom);
    if start > end || span.end_abs - history < top || span.start_abs - history > bottom {
        return None;
    }
    let text = term.bounds_to_string(
        Point::new(Line(start as i32), Column(0)),
        Point::new(Line(end as i32), term.last_column()),
    );
    Some(match trim_trailing {
        true => trim_trailing_spaces(&text),
        false => text,
    })
}

/// Appends the exit line to grid text, uniformly for every copy item.
fn with_exit_line(mut grid: String, span: &BlockSpan) -> String {
    if !grid.is_empty() {
        grid.push('\n');
    }
    grid.push_str(&span.exit_line());
    grid
}

/// The plain text of `span`'s rows, closed by its exit line — the block copy.
///
/// Read through [`Term::bounds_to_string`], the same function `capture
/// --plain` uses, so wrapped lines join instead of splitting at an invented
/// newline. `trim_trailing` trims trailing spaces per line exactly like
/// `copy_selection` (which it follows when its config is on); blank edges
/// stay either way (the range is the block, verbatim). Reads the grid's
/// absolute rows, never the fold mapping, so a folded span copies in full.
/// Returns `None` when the grid no longer holds the span's rows.
pub fn block_text<T: EventListener>(
    term: &Term<T>,
    span: &BlockSpan,
    trim_trailing: bool,
) -> Option<String> {
    block_grid_text(term, span, trim_trailing).map(|grid| with_exit_line(grid, span))
}

/// The block's command line (its first logical line) plus the exit line.
///
/// The split runs on the already-joined grid text, so a command the terminal
/// wrapped across rows stays one line. Same plain-text source as
/// [`block_text`]; folded-state independent; `None` exactly when it is.
pub fn block_command_text<T: EventListener>(
    term: &Term<T>,
    span: &BlockSpan,
    trim_trailing: bool,
) -> Option<String> {
    block_grid_text(term, span, trim_trailing).map(|grid| {
        let first = grid.lines().next().unwrap_or("").to_string();
        with_exit_line(first, span)
    })
}

/// The block's output lines (everything past the command line) plus the exit
/// line. A command with no output copies just the exit line. Same source,
/// fold-independence and `None` contract as [`block_command_text`].
pub fn block_output_text<T: EventListener>(
    term: &Term<T>,
    span: &BlockSpan,
    trim_trailing: bool,
) -> Option<String> {
    block_grid_text(term, span, trim_trailing).map(|grid| {
        let mut lines = grid.lines();
        let _ = lines.next();
        with_exit_line(lines.collect::<Vec<_>>().join("\n"), span)
    })
}

/// Trims trailing spaces and tabs per line, like `copy_selection`.
fn trim_trailing_spaces(text: &str) -> String {
    text.split('\n')
        .map(|line| line.trim_end_matches([' ', '\t']))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span(seq: u64, start: i64, end: i64) -> BlockSpan {
        BlockSpan {
            seq,
            start_abs: start,
            end_abs: end,
            exit_code: Some(0),
            daemon_id: None,
            truncated: false,
            fp: String::new(),
        }
    }

    fn daemon(id: u64) -> CommandBlock {
        CommandBlock {
            id,
            exit_code: Some(0),
            folded: false,
            truncated: false,
        }
    }

    /// `B … C … D` closes one span — and so does `B … D` with no `C`: `C`
    /// never gates, mirroring the daemon.
    #[test]
    fn b_to_d_pairs_with_and_without_c() {
        for with_c in [true, false] {
            let mut tracker = BlockTracker::default();
            tracker.note_b(10);
            // `C` is commentary: the tracker takes no note of it.
            let _ = with_c;
            tracker.note_d(20, Some(0), None);
            assert_eq!(tracker.spans.len(), 1, "with_c={with_c}");
            let span = &tracker.spans[0];
            assert_eq!((span.start_abs, span.end_abs), (10, 20));
            assert_eq!(span.exit_code, Some(0));
        }
    }

    /// A `D` with no open span closes nothing.
    #[test]
    fn d_without_b_closes_nothing() {
        let mut tracker = BlockTracker::default();
        tracker.note_d(20, Some(0), None);
        assert!(tracker.spans.is_empty());
    }

    /// A redrawn prompt re-arms the pending open: one run, one span.
    #[test]
    fn second_b_rearms_the_pending_open() {
        let mut tracker = BlockTracker::default();
        tracker.note_b(10);
        tracker.note_b(14);
        tracker.note_d(20, Some(1), None);
        assert_eq!(tracker.spans.len(), 1);
        assert_eq!(tracker.spans[0].start_abs, 14);
    }

    /// Ctrl-C still closes, with its real code.
    #[test]
    fn interrupt_exit_code_is_kept() {
        let mut tracker = BlockTracker::default();
        tracker.note_b(10);
        tracker.note_d(20, Some(130), None);
        assert_eq!(tracker.spans[0].exit_code, Some(130));
        assert_eq!(tracker.spans[0].exit_line(), "exit 130");
    }

    /// Only real failures redden: any non-zero status — including Ctrl-C's
    /// 130 and signal codes — reads as failed, while success and a missing
    /// status (bare `D`) never do.
    #[test]
    fn only_nonzero_exit_reads_as_failed() {
        let mut tracker = BlockTracker::default();
        for (end, code) in [(20, Some(1)), (30, Some(130)), (40, Some(0)), (50, None)] {
            tracker.note_b(end - 5);
            tracker.note_d(end, code, None);
        }
        let failed: Vec<bool> = tracker.spans.iter().map(|s| s.is_failed()).collect();
        assert_eq!(failed, vec![true, true, false, false]);
    }

    /// Close-order pairing: oldest unpaired span takes the smallest fresh id,
    /// and adopting twice pairs nothing new.
    #[test]
    fn adopt_pairs_close_order_and_is_idempotent() {
        let mut tracker = BlockTracker::default();
        tracker.note_b(10);
        tracker.note_d(20, Some(0), None);
        tracker.note_b(30);
        tracker.note_d(40, Some(1), None);
        tracker.adopt(&[daemon(1), daemon(2)]);
        assert_eq!(tracker.spans[0].daemon_id, Some(1));
        assert_eq!(tracker.spans[1].daemon_id, Some(2));
        tracker.adopt(&[daemon(1), daemon(2)]);
        assert_eq!(tracker.spans[0].daemon_id, Some(1));
        // A later span pairs against only what came after the watermark.
        tracker.note_b(50);
        tracker.note_d(60, Some(0), None);
        tracker.adopt(&[daemon(1), daemon(2), daemon(3)]);
        assert_eq!(tracker.spans[2].daemon_id, Some(3));
    }

    /// Pairing is newest-to-newest: fewer spans than blocks lands the span
    /// on the table's tail (a relink rebuilds only recent spans).
    #[test]
    fn adopt_pairs_a_lone_span_against_the_newest_id() {
        let mut tracker = BlockTracker::default();
        tracker.note_b(90);
        tracker.note_d(100, Some(0), None);
        tracker.adopt(&[daemon(95), daemon(96), daemon(97)]);
        assert_eq!(tracker.spans[0].daemon_id, Some(97));
        tracker.adopt(&[daemon(95), daemon(96), daemon(97)]);
        assert_eq!(tracker.spans[0].daemon_id, Some(97));
    }

    /// After a clear, the next adoption only chases the watermark past the
    /// stale snapshot — new spans pair against post-clear ids alone.
    #[test]
    fn clear_arms_resync_against_stale_snapshots() {
        let mut tracker = BlockTracker::default();
        tracker.note_b(10);
        tracker.note_d(20, Some(0), None);
        tracker.adopt(&[daemon(1)]);
        tracker.clear_blocks();
        assert!(tracker.spans.is_empty());
        // The daemon still advertises the pre-clear row (and a newer one the
        // cleared grid never saw): neither may pair with the next span.
        tracker.note_b(100);
        tracker.note_d(110, Some(0), None);
        tracker.adopt(&[daemon(1), daemon(2)]);
        assert_eq!(tracker.spans[0].daemon_id, None);
        tracker.note_b(120);
        tracker.note_d(130, Some(0), None);
        tracker.adopt(&[daemon(1), daemon(2), daemon(3)]);
        assert_eq!(tracker.spans[0].daemon_id, None);
        assert_eq!(tracker.spans[1].daemon_id, Some(3));
    }

    /// The table is bounded at the same magnitude as the daemon's.
    #[test]
    fn spans_evict_oldest_first_at_the_cap() {
        let mut tracker = BlockTracker::default();
        for i in 0..(BLOCK_SPAN_CAP + 1) as i64 {
            tracker.note_b(i * 10);
            tracker.note_d(i * 10 + 5, Some(0), None);
        }
        assert_eq!(tracker.spans.len(), BLOCK_SPAN_CAP);
        assert_eq!(tracker.spans[0].seq, 2);
    }

    /// Eviction drops fold state with its span.
    #[test]
    fn prune_before_drops_stale_spans_and_their_folds() {
        let mut tracker = BlockTracker::default();
        tracker.note_b(10);
        tracker.note_d(20, Some(0), None);
        tracker.note_b(30);
        tracker.note_d(40, Some(0), None);
        tracker.set_folded(1, true);
        tracker.prune_before(25);
        assert_eq!(tracker.spans.len(), 1);
        assert!(!tracker.has_folds());
    }

    /// Only closed spans answer: a running command is not a block yet.
    #[test]
    fn span_at_ignores_a_pending_open() {
        let mut tracker = BlockTracker::default();
        tracker.note_b(10);
        assert!(tracker.span_at(12).is_none());
        tracker.note_d(20, Some(0), None);
        assert_eq!(tracker.span_at(12).map(|s| s.seq), Some(1));
        assert!(tracker.span_at(21).is_none());
    }

    /// Unknown seqs refuse to fold.
    #[test]
    fn folding_an_unknown_seq_is_a_no_op() {
        let mut tracker = BlockTracker::default();
        assert_eq!(tracker.set_folded(7, true), None);
        assert!(!tracker.has_folds());
    }

    /// Menu fold and gutter fold share one path: folding then unfolding the
    /// same seq returns to unfolded with no leftover fold state.
    #[test]
    fn fold_toggle_round_trip_returns_to_unfolded() {
        let mut tracker = BlockTracker::default();
        tracker.note_b(10);
        tracker.note_d(20, Some(0), None);
        let seq = tracker.spans()[0].seq;
        assert_eq!(tracker.set_folded(seq, true), Some(true));
        assert!(tracker.is_folded(seq));
        assert!(tracker.has_folds());
        assert_eq!(tracker.set_folded(seq, false), Some(false));
        assert!(!tracker.is_folded(seq));
        assert!(!tracker.has_folds());
    }

    /// The visible mapping is identity without folds, and collapses folded
    /// spans to their first line, pulling earlier rows into view.
    #[test]
    fn map_visible_collapses_folds_against_the_window_bottom() {
        let mut tracker = BlockTracker::default();
        tracker.spans.push(span(1, 100, 110));
        // No folds: the identity window.
        assert_eq!(
            tracker.map_visible(109, 10),
            (100..=109).collect::<Vec<_>>()
        );
        tracker.set_folded(1, true);
        // Folded to its first line: rows 101..=109 hide, 91..=99 slide in.
        assert_eq!(
            tracker.map_visible(109, 10),
            vec![91, 92, 93, 94, 95, 96, 97, 98, 99, 100]
        );
        // The fold frees four rows, which pull earlier lines into view.
        assert_eq!(tracker.map_visible(100, 5), vec![96, 97, 98, 99, 100]);
    }

    /// `block_text` reads through `bounds_to_string`: a soft-wrapped line
    /// joins instead of splitting, and the exit line closes the text.
    #[test]
    fn block_text_joins_wrapped_lines_and_appends_the_exit_line() {
        use alacritty_terminal::event::VoidListener;
        use alacritty_terminal::term::Config;

        let size = crate::terminal::size::TermSize::new(10, 8);
        let mut term = Term::new(Config::default(), &size, VoidListener);
        let mut parser: alacritty_terminal::vte::ansi::Processor =
            alacritty_terminal::vte::ansi::Processor::new();
        // 10 columns: the 14-char line wraps 10+4.
        parser.advance(&mut term, b"0123456789ABCD\r\nsecond\r\n");
        let history = term.grid().history_size() as i64;
        let span = BlockSpan {
            seq: 1,
            start_abs: history,
            end_abs: history + 2,
            exit_code: Some(0),
            daemon_id: None,
            truncated: false,
            fp: String::new(),
        };
        assert_eq!(
            block_text(&term, &span, true).as_deref(),
            Some("0123456789ABCD\nsecond\nexit 0")
        );
    }

    /// A span the grid no longer holds copies as nothing, not a panic.
    #[test]
    fn block_text_of_an_evicted_span_is_none() {
        use alacritty_terminal::event::VoidListener;
        use alacritty_terminal::term::Config;

        let size = crate::terminal::size::TermSize::new(10, 8);
        let term = Term::new(Config::default(), &size, VoidListener);
        let history = term.grid().history_size() as i64;
        let span = BlockSpan {
            seq: 1,
            start_abs: history - 100,
            end_abs: history - 90,
            exit_code: Some(0),
            daemon_id: None,
            truncated: false,
            fp: String::new(),
        };
        assert_eq!(block_text(&term, &span, true), None);
    }

    /// The three copy items share one source: command takes the first logical
    /// line, output takes the rest, both takes all — each closed by the same
    /// exit line, and a wrapped command stays one line.
    #[test]
    fn block_parts_split_command_output_and_both_share_the_exit_line() {
        use alacritty_terminal::event::VoidListener;
        use alacritty_terminal::term::Config;

        let size = crate::terminal::size::TermSize::new(10, 8);
        let mut term = Term::new(Config::default(), &size, VoidListener);
        let mut parser: alacritty_terminal::vte::ansi::Processor =
            alacritty_terminal::vte::ansi::Processor::new();
        // 10 columns: the 14-char command wraps 10+4 but stays one line.
        parser.advance(&mut term, b"0123456789ABCD\r\nsecond\r\n");
        let history = term.grid().history_size() as i64;
        let span = BlockSpan {
            seq: 1,
            start_abs: history,
            end_abs: history + 2,
            exit_code: Some(0),
            daemon_id: None,
            truncated: false,
            fp: String::new(),
        };
        assert_eq!(
            block_command_text(&term, &span, true).as_deref(),
            Some("0123456789ABCD\nexit 0")
        );
        assert_eq!(
            block_output_text(&term, &span, true).as_deref(),
            Some("second\nexit 0")
        );
        assert_eq!(
            block_text(&term, &span, true).as_deref(),
            Some("0123456789ABCD\nsecond\nexit 0")
        );
    }

    /// Output with nothing past the command line copies just the exit line,
    /// and eviction reads as `None` on every item, not a panic.
    #[test]
    fn block_output_without_output_rows_is_just_the_exit_line() {
        use alacritty_terminal::event::VoidListener;
        use alacritty_terminal::term::Config;

        let size = crate::terminal::size::TermSize::new(40, 8);
        let mut term = Term::new(Config::default(), &size, VoidListener);
        let mut parser: alacritty_terminal::vte::ansi::Processor =
            alacritty_terminal::vte::ansi::Processor::new();
        parser.advance(&mut term, b"$ true\r\n");
        let history = term.grid().history_size() as i64;
        let span = BlockSpan {
            seq: 1,
            start_abs: history,
            end_abs: history,
            exit_code: Some(1),
            daemon_id: None,
            truncated: false,
            fp: String::new(),
        };
        assert_eq!(
            block_command_text(&term, &span, true).as_deref(),
            Some("$ true\nexit 1")
        );
        assert_eq!(
            block_output_text(&term, &span, true).as_deref(),
            Some("exit 1")
        );
        let gone = BlockSpan {
            seq: 2,
            start_abs: history - 100,
            end_abs: history - 90,
            exit_code: Some(0),
            daemon_id: None,
            truncated: false,
            fp: String::new(),
        };
        assert_eq!(block_command_text(&term, &gone, true), None);
        assert_eq!(block_output_text(&term, &gone, true), None);
    }

    /// Copies read the grid's absolute rows, never the fold mapping: a folded
    /// span copies exactly what the unfolded one does.
    #[test]
    fn block_copies_of_a_folded_span_match_the_unfolded_text() {
        use alacritty_terminal::event::VoidListener;
        use alacritty_terminal::term::Config;

        let size = crate::terminal::size::TermSize::new(40, 8);
        let mut term = Term::new(Config::default(), &size, VoidListener);
        let mut parser: alacritty_terminal::vte::ansi::Processor =
            alacritty_terminal::vte::ansi::Processor::new();
        parser.advance(&mut term, b"$ cmd\r\nout1\r\nout2\r\n");
        let history = term.grid().history_size() as i64;
        let span = BlockSpan {
            seq: 1,
            start_abs: history,
            end_abs: history + 2,
            exit_code: Some(0),
            daemon_id: None,
            truncated: false,
            fp: String::new(),
        };
        let mut tracker = BlockTracker::default();
        tracker.spans.push(span.clone());
        tracker.set_folded(1, true);
        assert!(tracker.is_folded(1));
        assert_eq!(
            block_text(&term, &span, true).as_deref(),
            Some("$ cmd\nout1\nout2\nexit 0")
        );
        assert_eq!(
            block_command_text(&term, &span, true).as_deref(),
            Some("$ cmd\nexit 0")
        );
        assert_eq!(
            block_output_text(&term, &span, true).as_deref(),
            Some("out1\nout2\nexit 0")
        );
    }

    /// The fold summary names the first line, the row count and the exit.
    #[test]
    fn fold_summary_names_first_line_count_and_exit() {
        use alacritty_terminal::event::VoidListener;
        use alacritty_terminal::term::Config;

        let size = crate::terminal::size::TermSize::new(40, 8);
        let mut term = Term::new(Config::default(), &size, VoidListener);
        let mut parser: alacritty_terminal::vte::ansi::Processor =
            alacritty_terminal::vte::ansi::Processor::new();
        parser.advance(&mut term, b"$ cargo build\r\ncompiling\r\n");
        let history = term.grid().history_size() as i64;
        let span = BlockSpan {
            seq: 1,
            start_abs: history,
            end_abs: history + 1,
            exit_code: Some(2),
            daemon_id: None,
            truncated: false,
            fp: String::new(),
        };
        assert_eq!(
            fold_summary(&term, &span, 40).as_str(),
            "▸ $ cargo build · 2 lines · exit 2"
        );
        // Narrow columns truncate with an ellipsis, wide chars included.
        let narrow = fold_summary(&term, &span, 10);
        assert!(narrow.ends_with('…'), "truncated: {narrow}");
    }

    /// A changed (or vanished) start row proves scroll-cap eviction aliased
    /// absolute rows, so verification fails, the table clears through the
    /// clear path (spans drop, resync arms), and the daemon ids re-pair
    /// going forward instead of against the aliased rows.
    #[test]
    fn changed_start_rows_fail_verification_and_resync() {
        let mut tracker = BlockTracker::default();
        tracker.note_b(10);
        tracker.note_d(20, Some(0), Some("cmd".into()));
        assert!(tracker.verify_all(&|abs| (abs == 10).then(|| "cmd".to_string())));
        assert_eq!(tracker.spans.len(), 1);
        // Cap overflow rewrote the row: verification fails and clears.
        assert!(!tracker.verify_all(&|_| Some("other".into())));
        assert!(tracker.spans.is_empty());
        // Resync armed: the stale snapshot pairs nothing and only advances
        // the watermark, so the next span pairs against post-clear ids.
        tracker.adopt(&[daemon(1)]);
        tracker.note_b(30);
        tracker.note_d(40, Some(0), Some("new".into()));
        assert!(tracker.verify_all(&|abs| (abs == 30).then(|| "new".to_string())));
        tracker.adopt(&[daemon(1), daemon(2)]);
        assert_eq!(tracker.spans.len(), 1);
        assert_eq!(tracker.spans[0].daemon_id, Some(2));
    }

    /// A start row the grid no longer holds reads as eviction, not as a pass.
    #[test]
    fn a_vanished_start_row_fails_verification() {
        let mut tracker = BlockTracker::default();
        tracker.note_b(10);
        tracker.note_d(20, Some(0), Some("cmd".into()));
        assert!(!tracker.verify_all(&|_| None));
        assert!(tracker.spans.is_empty());
    }

    fn folded_daemon(id: u64) -> CommandBlock {
        CommandBlock {
            id,
            exit_code: Some(0),
            folded: true,
            truncated: false,
        }
    }

    /// Entry-5: a restart restores folds — already-paired spans refresh their
    /// fold from the daemon row.
    #[test]
    fn adopt_refreshes_folds_for_already_paired_spans() {
        let mut tracker = BlockTracker::default();
        tracker.note_b(10);
        tracker.note_d(20, Some(0), None);
        tracker.adopt(&[daemon(1)]);
        assert!(!tracker.is_folded(1));
        tracker.adopt(&[folded_daemon(1)]);
        assert!(tracker.is_folded(1), "the daemon is truth after a restore");
        tracker.adopt(&[daemon(1)]);
        assert!(!tracker.is_folded(1), "an unfold propagates the same way");
    }

    /// Entry-5: a toggle made before the first adoption survives pairing, and
    /// the caller learns the newly paired rows so it can push the fold up.
    #[test]
    fn adopt_keeps_pre_adoption_folds_and_reports_new_pairs() {
        let mut tracker = BlockTracker::default();
        tracker.note_b(10);
        tracker.note_d(20, Some(0), None);
        tracker.set_folded(1, true);
        let newly = tracker.adopt(&[daemon(1)]);
        assert!(
            tracker.is_folded(1),
            "pairing must not clobber the local fold"
        );
        assert_eq!(newly, vec![(1, 1, true)]);
        // The same snapshot adopted twice reports nothing new.
        let again = tracker.adopt(&[daemon(1)]);
        assert!(again.is_empty());
        assert!(tracker.is_folded(1));
    }

    /// Entry-5: once the daemon echoes the pushed fold, the span is converged —
    /// a later daemon unfold applies again instead of sticking.
    #[test]
    fn daemon_echo_clears_the_pending_toggle() {
        let mut tracker = BlockTracker::default();
        tracker.note_b(10);
        tracker.note_d(20, Some(0), None);
        tracker.set_folded(1, true);
        assert_eq!(tracker.adopt(&[daemon(1)]), vec![(1, 1, true)]);
        // The push landed: the daemon row agrees, so the span is converged.
        let again = tracker.adopt(&[folded_daemon(1)]);
        assert!(again.is_empty());
        assert!(tracker.is_folded(1));
        // And a genuine daemon-side unfold now propagates.
        tracker.adopt(&[daemon(1)]);
        assert!(!tracker.is_folded(1));
    }

    /// Entry-5: the unfold direction converges too — a span unfolded before
    /// pairing is reported so the caller pushes the unfold up, and re-adopts
    /// keep it until the daemon echoes.
    #[test]
    fn adopt_reports_pre_adoption_unfolds_against_folded_rows() {
        let mut tracker = BlockTracker::default();
        tracker.note_b(10);
        tracker.note_d(20, Some(0), None);
        // Paired while the daemon still shows folded: local unfold is newer.
        let newly = tracker.adopt(&[folded_daemon(1)]);
        assert!(!tracker.is_folded(1));
        assert_eq!(newly, vec![(1, 1, false)]);
        let again = tracker.adopt(&[folded_daemon(1)]);
        assert!(again.is_empty());
        assert!(
            !tracker.is_folded(1),
            "an unacked unfold must not flicker back on"
        );
        // The push landed: the daemon row agrees, so the span is converged —
        // and a genuine daemon-side fold now propagates.
        tracker.adopt(&[daemon(1)]);
        assert!(!tracker.is_folded(1));
        tracker.adopt(&[folded_daemon(1)]);
        assert!(tracker.is_folded(1));
    }
}
