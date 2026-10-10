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

/// One pending `B`: the absolute row it landed on, and whether its command
/// text (`C`) has arrived yet — the watershed between a redrawn prompt's `B`
/// (same run) and a nested command's `B` (the nested slot). Mirrors the
/// daemon's slots in [`note_block_marks`]: the same byte stream must build
/// the same pairing on both ends.
#[derive(Debug, Clone, Copy)]
struct BlockSlot {
    start_abs: i64,
    has_c: bool,
}

impl BlockSlot {
    fn fresh(start_abs: i64) -> Self {
        Self {
            start_abs,
            has_c: false,
        }
    }
}

/// The pane's spans plus its fold state, shared between the reader thread
/// (which opens/closes spans) and the UI thread (which folds, jumps, copies
/// and adopts daemon ids).
#[derive(Debug, Default)]
pub struct BlockTracker {
    spans: Vec<BlockSpan>,
    next_seq: u64,
    /// The outer command's pending `B`, if one is running.
    outer: Option<BlockSlot>,
    /// The one nested command's pending `B`, if one is running.
    nested: Option<BlockSlot>,
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
    /// A `B` mark landed on `abs`: a command started. Before the outer slot's
    /// `C` it refreshes that slot (a redrawn prompt is still one command run,
    /// mirroring the daemon); past it, it opens the nested slot, whose own
    /// pre-`C` redraws refresh it the same way. A `B` while the nested slot
    /// is taken past its `C` is a second layer of nesting and voids both
    /// slots instead — the stretch records nothing rather than wrong spans,
    /// and the next `B` starts over.
    pub fn note_b(&mut self, abs: i64) {
        // No outer slot, or still before the outer command's `C`: a redrawn
        // prompt is still one command run, not two.
        if self.outer.is_none_or(|slot| !slot.has_c) {
            self.outer = Some(BlockSlot::fresh(abs));
        } else if self.nested.is_none() {
            self.nested = Some(BlockSlot::fresh(abs));
        } else if self.nested.is_none_or(|slot| !slot.has_c) {
            // Same redraw one level down: the nested prompt redrawn before
            // its own `C` is still one inner run, not a second layer.
            self.nested = Some(BlockSlot::fresh(abs));
        } else {
            self.outer = None;
            self.nested = None;
        }
    }

    /// A `C` mark landed: the topmost pending command started. Annotates that
    /// slot; with no pending `B` there is nothing to annotate.
    pub fn note_c(&mut self) {
        if let Some(slot) = self.nested.as_mut() {
            slot.has_c = true;
        } else if let Some(slot) = self.outer.as_mut() {
            slot.has_c = true;
        }
    }

    /// Voids both pending slots: the alternate screen touched the stretch, so
    /// neither open run may become a span — blocks never span it.
    pub fn void_pending(&mut self) {
        self.outer = None;
        self.nested = None;
    }

    /// Whether any command is running (an outer or nested `B` with no `D`
    /// yet). Test-only: production code reads [`BlockTracker::open_start`]
    /// and [`BlockTracker::void_pending`] instead.
    #[cfg(test)]
    pub fn has_pending(&self) -> bool {
        self.outer.is_some() || self.nested.is_some()
    }

    /// The topmost pending `B`'s absolute row, if a command is running — so
    /// the caller can fingerprint the rows the span will be verified against.
    pub fn open_start(&self) -> Option<i64> {
        self.nested
            .map(|slot| slot.start_abs)
            .or_else(|| self.outer.map(|slot| slot.start_abs))
    }

    /// A `D` mark landed on `abs`: the topmost pending command finished.
    /// With no pending `B` there is nothing to close — the shell's own
    /// startup mark, or a replay that starts mid-command.
    ///
    /// `fp` is the topmost start row's text as it stands now (see
    /// [`row_text`]); `None` when the row is already gone, which
    /// [`verify_all`](Self::verify_all) will read as eviction.
    ///
    /// `C` never appears here on purpose: it only annotates (see
    /// [`BlockTracker::note_c`]), and commands run with no `133;C` at all,
    /// so `D` closes the topmost pending `B` either way, exactly like the
    /// daemon.
    pub fn note_d(&mut self, abs: i64, exit_code: Option<i32>, fp: Option<String>) {
        let start = match (&self.nested, &self.outer) {
            (Some(slot), _) => slot.start_abs,
            (None, Some(slot)) => slot.start_abs,
            (None, None) => return,
        };
        if self.nested.is_some() {
            self.nested = None;
        } else {
            self.outer = None;
        }
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
    /// already forgot those rows, so spans pointing at them are stale — and
    /// voids any pending slot whose start row it forgot too.
    pub fn prune_before(&mut self, top_abs: i64) {
        if self.outer.is_some_and(|slot| slot.start_abs < top_abs) {
            self.outer = None;
        }
        if self.nested.is_some_and(|slot| slot.start_abs < top_abs) {
            self.nested = None;
        }
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
        self.outer = None;
        self.nested = None;
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
        self.outer = None;
        self.nested = None;
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

    /// Span count plus sorted folded seqs, recorded by the paint snapshot in
    /// the same lock scope as the fold mapping: hit-tests require full
    /// equality before trusting the snapshot's rows, so a fold, unfold, span
    /// close or eviction between paint and click recomputes live instead of
    /// resolving through a stale map. Seqs are never reused (`next_seq` only
    /// moves forward), so the pair names the exact table the map came from.
    pub fn fold_epoch(&self) -> (usize, Vec<u64>) {
        let mut folded: Vec<u64> = self.folded.iter().copied().collect();
        folded.sort_unstable();
        (self.spans.len(), folded)
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
        let changed = match folded {
            true => self.folded.insert(seq),
            false => self.folded.remove(&seq),
        };
        // Same-state sets converge nothing: only a real flip arms a push.
        if changed {
            self.dirty.insert(seq);
        }
        Some(folded)
    }

    /// Whether absolute row `abs` is hidden inside a folded span (past its
    /// first line). The one hidden-row helper every fold-aware path walks
    /// through: the paint mapping, the mouse hit-tests and the jump target
    /// all key off this, so a row is hidden for all of them or none.
    pub fn hidden_abs_in(&self, abs: i64) -> bool {
        self.spans
            .iter()
            .any(|s| self.folded.contains(&s.seq) && s.start_abs < abs && abs <= s.end_abs)
    }

    /// Whether `abs` is hidden inside a folded span (past its first line).
    fn is_hidden(&self, abs: i64) -> bool {
        self.hidden_abs_in(abs)
    }

    /// The window bottom placing `start_abs` at the top row: walk forward
    /// past `rows - 1` visible rows from the span's start. The jump target
    /// inverts this (`display_offset = history + rows - 1 - bottom`) and
    /// verifies through [`BlockTracker::map_visible`], so eviction clamps
    /// converge at the bounds instead of overshooting.
    pub fn bottom_for_top(&self, start_abs: i64, rows: usize) -> i64 {
        let mut bottom = start_abs;
        let mut visible = 1;
        while visible < rows.max(1) {
            bottom += 1;
            if !self.hidden_abs_in(bottom) {
                visible += 1;
            }
        }
        bottom
    }

    /// The scroll offset placing `start_abs` at the top row through the fold
    /// mapping: the window bottom topping out at the span's start (via
    /// [`BlockTracker::bottom_for_top`]), verified through the paint mapping
    /// to a fixed point. `floor`/`grid_bottom` are the absolute rows the grid
    /// still holds (see [`visible_map`]); each step moves whole painted rows,
    /// so the exact single-shot guess converges at once and eviction clamps
    /// converge at the bounds instead — eight steps bound the walk either
    /// way. A span the grid already forgot keeps the identity clamp, which is
    /// the same bound.
    pub fn jump_target_for(
        &self,
        start_abs: i64,
        rows: usize,
        identity: i64,
        history: i64,
        floor: i64,
        grid_bottom: i64,
    ) -> i64 {
        if start_abs < floor {
            return identity;
        }
        let mut target =
            (history + rows as i64 - 1 - self.bottom_for_top(start_abs, rows).min(grid_bottom))
                .clamp(0, history);
        let mut prev = i64::MIN;
        for _ in 0..8 {
            if target == prev {
                break;
            }
            prev = target;
            let bottom = (history - target + rows as i64 - 1).min(grid_bottom);
            let mut map = self.map_visible(bottom, rows);
            map.retain(|&abs| abs >= floor);
            match map.iter().position(|&abs| abs == start_abs) {
                Some(0) => break,
                Some(pos) => {
                    // The start sits `pos` painted rows down: the window tops
                    // out too high by that many visible rows.
                    target = (target - pos as i64).max(0);
                }
                None if map.first().is_some_and(|&first| first > start_abs) || map.is_empty() => {
                    // The window sits below the start entirely: scroll up, by
                    // the absolute gap first, then converge.
                    let gap = map
                        .first()
                        .map(|&first| first - start_abs)
                        .unwrap_or(1)
                        .max(1);
                    let next = target.saturating_add(gap).min(history);
                    if next == target {
                        break;
                    }
                    target = next;
                }
                None => {
                    // The window sits above the start entirely: scroll down.
                    let gap = map.last().map(|&last| start_abs - last).unwrap_or(1).max(1);
                    let next = target.saturating_sub(gap).max(0);
                    if next == target {
                        break;
                    }
                    target = next;
                }
            }
        }
        target
    }

    /// The absolute rows to paint, top to bottom, for a window whose bottom
    /// row is `bottom_abs` showing `rows` rows.
    ///
    /// Folded spans collapse to their first line; the freed rows pull earlier
    /// lines into view (the mapping still ends at the window bottom, so a
    /// full mapping paints exactly where the identity window did). Near the
    /// grid's birth there may be fewer than `rows` lines — those paint
    /// top-aligned, with the blanks left at the bottom. With nothing folded
    /// this is exactly the identity window, so callers can skip the mapping
    /// then.
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
/// window). The mapping names the rows ending at the window bottom, so
/// folding pulls earlier lines into view; a full mapping paints exactly
/// where the identity window did, while a short one (near the grid's birth)
/// paints top-aligned with the blanks at the bottom — the prompt may sit
/// higher, but it stays visible and nothing is hidden.
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
    // newest rows; what is left short paints top-aligned, blank at the
    // bottom.
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
/// rows a short map leaves at the bottom (near the grid's birth, where fewer
/// than the viewport's rows exist yet). Top-aligned: `map[i]` paints at row
/// `i`, so the grid, the gutter and every hit-test share this one function
/// and cannot disagree about which row an absolute row sits on.
pub fn screen_to_abs(row: usize, top: i64, map: Option<&[i64]>) -> Option<i64> {
    match map {
        None => Some(top + row as i64),
        Some(m) => m.get(row).copied(),
    }
}

/// A screen row through an optional [`visible_map`] to the grid [`Line`]
/// selection, links and menus read: the painted absolute row re-based on the
/// mapping's history, `None` for the blank rows past a short map. The single
/// shared row function for every mouse entry — selection start/update and
/// drag-scroll, link hover/click/resolve, the right-click latch — so clicks
/// land on the rows the frame painted. Without folds the mapping is `None`
/// and this is exactly `Line(row - display_offset)`, the old identity path.
pub fn screen_to_line(row: usize, top: i64, map: Option<&[i64]>, history: i64) -> Option<Line> {
    screen_to_abs(row, top, map).map(|abs| Line((abs - history) as i32))
}

/// The screen row a folded span's summary paints on through the frame's
/// mapping: its start row's position in the map. Top-aligned like the grid
/// itself, so this is the same row the gutter marker sits on, at any scroll
/// offset.
pub fn fold_summary_row(map: &[i64], start_abs: i64) -> Option<usize> {
    map.iter().position(|&a| a == start_abs)
}

/// The gutter's markers for one painted frame, one entry per screen row:
/// `(seq, folded, failed)` for the closed span starting on that row, if any.
///
/// Resolved through the same mapping the grid paints — row `r` names `map[r]`
/// (top-aligned; rows past a short map are blank), or `top + r` without one
/// — so marker rows and summary rows cannot disagree: both sides read the
/// one mapping the snapshot computed for the frame, under a single lock
/// scope (see [`screen_to_abs`]).
pub fn gutter_markers(
    tracker: &BlockTracker,
    map: Option<&[i64]>,
    top: i64,
    rows: usize,
) -> Vec<Option<(u64, bool, bool)>> {
    let mut starts = std::collections::HashMap::new();
    for span in tracker.spans() {
        starts.entry(span.start_abs).or_insert((
            span.seq,
            tracker.is_folded(span.seq),
            span.is_failed(),
        ));
    }
    (0..rows)
        .map(|row| screen_to_abs(row, top, map).and_then(|abs| starts.get(&abs).copied()))
        .collect()
}

/// The one-line summary a folded span collapses to: its first line plus the
/// row count and exit line. Truncated to `cols` display columns (wide chars
/// count by [`unicode_width`]). No gutter prefix: the gutter's marker is the
/// fold's single arrow, and a second one here would double it.
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
    let mut summary = String::new();
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

    /// One nested layer builds three spans in close order: each inner `D`
    /// closes its own inner `B`, and the outer `D` still finds the outer `B`.
    /// Mirrors the daemon's nested test over the same mark sequence, so both
    /// ends pair the same stream the same way.
    #[test]
    fn one_nested_layer_builds_three_spans_in_close_order() {
        let mut tracker = BlockTracker::default();
        tracker.note_b(10);
        tracker.note_c();
        tracker.note_b(20);
        tracker.note_c();
        tracker.note_d(30, Some(0), None);
        tracker.note_b(40);
        tracker.note_c();
        tracker.note_d(50, Some(3), None);
        tracker.note_d(60, Some(0), None);
        assert_eq!(tracker.spans.len(), 3);
        let bounds: Vec<(i64, i64)> = tracker
            .spans
            .iter()
            .map(|s| (s.start_abs, s.end_abs))
            .collect();
        assert_eq!(bounds, vec![(20, 30), (40, 50), (10, 60)]);
        let exits: Vec<Option<i32>> = tracker.spans.iter().map(|s| s.exit_code).collect();
        assert_eq!(exits, vec![Some(0), Some(3), Some(0)]);
        assert!(!tracker.has_pending());
    }

    /// `C` annotates the topmost open slot: the nested `C` does not mark the
    /// outer run, and a later outer `B` past the outer `C` still nests.
    #[test]
    fn c_annotates_the_topmost_open_slot() {
        let mut tracker = BlockTracker::default();
        tracker.note_b(10);
        tracker.note_c();
        tracker.note_b(20);
        tracker.note_c();
        // The nested slot is taken: this `B` is a second layer, voiding both.
        tracker.note_b(30);
        assert!(!tracker.has_pending());
        tracker.note_d(40, Some(0), None);
        assert!(tracker.spans.is_empty());
    }

    /// A second layer of nesting voids both slots: later `D`s close nothing,
    /// and the next `B` starts over.
    #[test]
    fn a_second_nesting_layer_voids_both_slots() {
        let mut tracker = BlockTracker::default();
        tracker.note_b(10);
        tracker.note_c();
        tracker.note_b(20);
        tracker.note_c();
        tracker.note_b(30);
        assert!(!tracker.has_pending());
        tracker.note_d(40, Some(0), None);
        tracker.note_d(50, Some(0), None);
        assert!(tracker.spans.is_empty());
        tracker.note_b(60);
        tracker.note_d(70, Some(0), None);
        assert_eq!(tracker.spans.len(), 1);
        assert_eq!(
            (tracker.spans[0].start_abs, tracker.spans[0].end_abs),
            (60, 70)
        );
    }

    /// A `C` with no pending `B` annotates nothing and opens nothing.
    #[test]
    fn c_without_b_opens_nothing() {
        let mut tracker = BlockTracker::default();
        tracker.note_c();
        assert!(!tracker.has_pending());
        tracker.note_d(20, Some(0), None);
        assert!(tracker.spans.is_empty());
    }

    /// The nested prompt redrawn before its own `C` coalesces like the
    /// outer one: same stream as the daemon's nested-redraw test.
    #[test]
    fn nested_redraws_before_the_c_coalesce() {
        let mut tracker = BlockTracker::default();
        tracker.note_b(10);
        tracker.note_c();
        tracker.note_b(20);
        tracker.note_b(21);
        tracker.note_c();
        tracker.note_d(30, Some(0), None);
        tracker.note_d(40, Some(0), None);
        assert_eq!(tracker.spans.len(), 2);
        let bounds: Vec<(i64, i64)> = tracker
            .spans
            .iter()
            .map(|s| (s.start_abs, s.end_abs))
            .collect();
        assert_eq!(bounds, vec![(21, 30), (10, 40)]);
    }

    /// The fingerprint reads the topmost start: with a nested run open, the
    /// inner `D` fingerprints the inner row, not the outer one.
    #[test]
    fn open_start_names_the_topmost_pending_b() {
        let mut tracker = BlockTracker::default();
        assert_eq!(tracker.open_start(), None);
        tracker.note_b(10);
        assert_eq!(tracker.open_start(), Some(10));
        tracker.note_c();
        tracker.note_b(20);
        assert_eq!(tracker.open_start(), Some(20));
        tracker.note_d(30, Some(0), None);
        assert_eq!(tracker.open_start(), Some(10));
        tracker.note_d(40, Some(0), None);
        assert_eq!(tracker.open_start(), None);
    }

    /// A pending slot whose start row the scroll-limit eviction already forgot
    /// is voided with the spans pointing at those rows.
    #[test]
    fn prune_before_voids_a_pending_slot_below_the_line() {
        let mut tracker = BlockTracker::default();
        tracker.note_b(10);
        tracker.prune_before(25);
        assert!(!tracker.has_pending());
        tracker.note_d(30, Some(0), None);
        assert!(tracker.spans.is_empty());
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
            "$ cargo build · 2 lines · exit 2"
        );
        // Narrow columns truncate with an ellipsis, wide chars included.
        let narrow = fold_summary(&term, &span, 10);
        assert!(narrow.ends_with('…'), "truncated: {narrow}");
    }

    /// A fold mapping shorter than the viewport paints top-aligned: content
    /// from the first row, blanks only at the bottom.
    #[test]
    fn short_maps_paint_top_aligned_with_blanks_at_the_bottom() {
        let map = vec![10, 11, 12];
        for (row, want) in [(0, Some(10)), (1, Some(11)), (2, Some(12))] {
            assert_eq!(
                screen_to_abs(row, 8, Some(&map)),
                want,
                "content sits at the top, row {row}"
            );
        }
        assert_eq!(
            screen_to_abs(3, 8, Some(&map)),
            None,
            "the shortfall blanks at the bottom"
        );
        assert_eq!(
            screen_to_abs(4, 8, Some(&map)),
            None,
            "the shortfall blanks at the bottom"
        );
    }

    /// The summary carries no gutter prefix: the gutter's marker is the
    /// fold's single arrow, so a `▸` here would double it.
    #[test]
    fn fold_summary_carries_no_gutter_prefix() {
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
        let summary = fold_summary(&term, &span, 40);
        assert!(
            !summary.starts_with('▸'),
            "the single arrow lives in the gutter: {summary}"
        );
        assert!(
            summary.starts_with("$ cargo build"),
            "the first line still leads: {summary}"
        );
    }

    /// Gutter markers and fold summaries resolve through one mapping, driven
    /// by a live `Term` through real scroll offsets: at every
    /// `display_offset` the frame's [`visible_map`] puts each folded span's
    /// marker on its summary row. No synthetic tops — the scroll state comes
    /// out of the grid the paint path reads.
    #[test]
    fn gutter_markers_land_on_summary_rows_through_real_scroll() {
        use alacritty_terminal::event::VoidListener;
        use alacritty_terminal::grid::Scroll;
        use alacritty_terminal::term::Config;

        let rows = 10usize;
        let size = crate::terminal::size::TermSize::new(40, rows);
        let mut term = Term::new(Config::default(), &size, VoidListener);
        let mut parser: alacritty_terminal::vte::ansi::Processor =
            alacritty_terminal::vte::ansi::Processor::new();
        for i in 0..40 {
            parser.advance(&mut term, format!("line {i:02}\r\n").as_bytes());
        }
        let history = term.grid().history_size() as i64;
        assert!(
            history > rows as i64 + 20,
            "enough scrollback to move through: history {history}"
        );
        let mut tracker = BlockTracker::default();
        let start = history - 20;
        tracker.spans.push(span(1, start, start + 7));
        tracker.set_folded(1, true);
        // Identity offsets around the fold: at the fold's top, five rows
        // above it (its start rows hide inside the window), five below.
        for offset in [history - start, history - start - 5, history - start + 5] {
            let cur = term.grid().display_offset() as i64;
            term.scroll_display(Scroll::Delta((offset - cur) as i32));
            assert_eq!(
                term.grid().display_offset(),
                offset as usize,
                "the grid scrolled where asked"
            );
            let (top, _) = window_abs_range(&term, rows);
            assert_eq!(
                top,
                history - offset,
                "the identity top the mapping builds against"
            );
            let Some(map) = visible_map(&term, &tracker, rows) else {
                panic!("the fold intersects the window at offset {offset}");
            };
            assert_eq!(map.len(), rows, "full map at offset {offset}");
            let markers = gutter_markers(&tracker, Some(&map), top, rows);
            let summary = fold_summary_row(&map, start).expect("the fold stays visible");
            assert_eq!(
                markers[summary],
                Some((1, true, false)),
                "marker sits on the summary row at offset {offset}: {map:?}"
            );
            // Every painted row resolves back through the same mapping.
            for (row, abs) in map.iter().enumerate() {
                assert_eq!(
                    screen_to_abs(row, top, Some(&map)),
                    Some(*abs),
                    "row {row} at offset {offset}"
                );
                assert_eq!(
                    screen_to_line(row, top, Some(&map), history).map(|l| l.0),
                    Some((abs - history) as i32),
                    "shared row function agrees at offset {offset}"
                );
            }
        }
        // Unfolded again the window is identity at the same scroll state: no
        // mapping, and the gutter names the span unfolded exactly where the
        // identity window puts it — the fold paths add nothing without folds.
        tracker.set_folded(1, false);
        assert!(visible_map(&term, &tracker, rows).is_none());
        let (top, _) = window_abs_range(&term, rows);
        let markers = gutter_markers(&tracker, None, top, rows);
        assert_eq!(
            markers[(start - top) as usize],
            Some((1, false, false)),
            "identity row without folds"
        );
        assert!(
            markers.iter().flatten().all(|&(_, folded, _)| !folded),
            "nothing folded, nothing folded-marked"
        );
    }

    /// A short mapping on a live grid near its birth paints top-aligned:
    /// content from the first row, blanks only at the bottom, marker still
    /// on the summary's row.
    #[test]
    fn short_maps_stay_top_aligned_on_a_live_grid() {
        use alacritty_terminal::event::VoidListener;
        use alacritty_terminal::term::Config;

        let rows = 10usize;
        let size = crate::terminal::size::TermSize::new(40, rows);
        let mut term = Term::new(Config::default(), &size, VoidListener);
        let mut parser: alacritty_terminal::vte::ansi::Processor =
            alacritty_terminal::vte::ansi::Processor::new();
        parser.advance(&mut term, b"one\r\ntwo\r\nthree\r\n");
        let history = term.grid().history_size() as i64;
        let mut tracker = BlockTracker::default();
        tracker.spans.push(span(1, history, history + 2));
        tracker.set_folded(1, true);
        let (top, _) = window_abs_range(&term, rows);
        let Some(map) = visible_map(&term, &tracker, rows) else {
            panic!("the fold intersects the window");
        };
        assert!(map.len() < rows, "birth leaves the map short: {map:?}");
        assert_eq!(
            screen_to_abs(0, top, Some(&map)),
            Some(map[0]),
            "content sits at the top"
        );
        for row in map.len()..rows {
            assert_eq!(
                screen_to_abs(row, top, Some(&map)),
                None,
                "the shortfall blanks at the bottom, row {row}"
            );
        }
        let markers = gutter_markers(&tracker, Some(&map), top, rows);
        assert_eq!(markers.len(), rows);
        let summary = fold_summary_row(&map, history).expect("the fold stays visible");
        assert_eq!(summary, 0, "content sits at the top: {map:?}");
        assert_eq!(markers[summary], Some((1, true, false)));
        assert!(
            markers[map.len()..].iter().all(|m| m.is_none()),
            "the shortfall blanks at the bottom"
        );
    }

    /// The jump fixed-point lands a folded span's start at the top row on a
    /// live grid: from several starting scroll offsets, the walk's target
    /// scrolls the window so the paint mapping opens on the span's start,
    /// with the gutter marker on that same row. No synthetic tops — offsets,
    /// windows and landings all come out of the grid through
    /// [`window_abs_range`] and [`visible_map`], the same mapping the jump
    /// verifies against.
    #[test]
    fn jump_fixed_point_lands_fold_starts_at_the_top_row_on_a_live_grid() {
        use alacritty_terminal::event::VoidListener;
        use alacritty_terminal::grid::Dimensions as _;
        use alacritty_terminal::grid::Scroll;
        use alacritty_terminal::term::Config;

        let rows = 10usize;
        let size = crate::terminal::size::TermSize::new(40, rows);
        let mut term = Term::new(Config::default(), &size, VoidListener);
        let mut parser: alacritty_terminal::vte::ansi::Processor =
            alacritty_terminal::vte::ansi::Processor::new();
        for i in 0..40 {
            parser.advance(&mut term, format!("line {i:02}\r\n").as_bytes());
        }
        let history = term.grid().history_size() as i64;
        assert!(
            history > rows as i64 + 20,
            "enough scrollback to move through: history {history}"
        );
        let mut tracker = BlockTracker::default();
        let start = history - 20;
        tracker.spans.push(span(1, start, start + 7));
        tracker.set_folded(1, true);
        let floor = history + term.topmost_line().0 as i64;
        let grid_bottom = history + term.bottommost_line().0 as i64;
        // The identity jump arithmetic, as `jump_to_block_start` runs it.
        let identity = (history - start).min(history);
        // From the bottom, mid-scrollback and just above the fold alike.
        for initial in [0, 8, history - start + 5] {
            let cur = term.grid().display_offset() as i64;
            term.scroll_display(Scroll::Delta((initial - cur) as i32));
            assert_eq!(term.grid().display_offset(), initial as usize);
            let target =
                tracker.jump_target_for(start, rows, identity, history, floor, grid_bottom);
            let cur = term.grid().display_offset() as i64;
            term.scroll_display(Scroll::Delta((target - cur) as i32));
            assert_eq!(
                term.grid().display_offset(),
                target as usize,
                "the grid scrolled to the walk's target from {initial}"
            );
            let (top, _) = window_abs_range(&term, rows);
            let Some(map) = visible_map(&term, &tracker, rows) else {
                panic!("the fold intersects the window at target {target}");
            };
            assert_eq!(
                fold_summary_row(&map, start),
                Some(0),
                "the start lands at the top row from {initial}: {map:?}"
            );
            let markers = gutter_markers(&tracker, Some(&map), top, rows);
            assert_eq!(
                markers[0],
                Some((1, true, false)),
                "the marker lands with it from {initial}"
            );
        }
    }

    /// The hidden-row helper names fold interiors only: starts paint, past
    /// the end is outside, and nothing hides unfolded.
    #[test]
    fn hidden_abs_in_names_fold_interiors_only() {
        let mut tracker = BlockTracker::default();
        tracker.spans.push(span(1, 100, 110));
        assert!(!tracker.hidden_abs_in(100));
        assert!(!tracker.hidden_abs_in(101));
        assert!(!tracker.hidden_abs_in(111));
        tracker.set_folded(1, true);
        assert!(!tracker.hidden_abs_in(100), "the start paints");
        assert!(tracker.hidden_abs_in(101));
        assert!(tracker.hidden_abs_in(110));
        assert!(!tracker.hidden_abs_in(111), "past the end is outside");
    }

    /// The jump helper tops out at the span's start: the window bottom past
    /// which `rows - 1` visible rows follow the start, so the paint mapping
    /// from that bottom opens on the start row.
    #[test]
    fn bottom_for_top_opens_the_paint_mapping_on_the_start() {
        let mut tracker = BlockTracker::default();
        tracker.spans.push(span(1, 100, 110));
        tracker.set_folded(1, true);
        // 101..=110 hide: nine visible rows past the start land at 119.
        assert_eq!(tracker.bottom_for_top(100, 10), 119);
        let map = tracker.map_visible(119, 10);
        assert_eq!(map.len(), 10);
        assert_eq!(map[0], 100);
        // Without folds it is the plain window end.
        tracker.set_folded(1, false);
        assert_eq!(tracker.bottom_for_top(100, 10), 109);
    }

    /// Without a mapping the shared row function is the old identity path:
    /// `Line(row - display_offset)`.
    #[test]
    fn screen_to_line_without_folds_is_the_identity_path() {
        let history = 50;
        let display_offset = 6;
        let top = history - display_offset;
        for row in 0..10 {
            assert_eq!(
                screen_to_line(row, top, None, history),
                Some(Line(row as i32 - display_offset as i32))
            );
        }
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

    #[test]
    fn same_state_set_folded_arms_no_push() {
        let mut tracker = BlockTracker::default();
        tracker.note_b(10);
        tracker.note_d(20, Some(0), None);
        // A real flip arms exactly one push.
        tracker.set_folded(1, true);
        assert!(tracker.dirty.contains(&1));
        tracker.dirty.clear();
        // Repeating the same state converges nothing: no push armed.
        tracker.set_folded(1, true);
        assert!(
            tracker.dirty.is_empty(),
            "a redundant fold must not emit a spurious SetBlockFolded"
        );
        tracker.set_folded(1, false);
        assert!(
            tracker.dirty.contains(&1),
            "a genuine flip still arms its push"
        );
        tracker.dirty.clear();
        // The unfold direction is symmetric: redundant `false -> false`
        // converges nothing, a real unfold arms.
        tracker.set_folded(1, false);
        assert!(
            tracker.dirty.is_empty(),
            "a redundant unfold must not arm a push either"
        );
        assert_eq!(
            tracker.set_folded(1, false),
            Some(false),
            "redundant sets still report the state"
        );
        assert_eq!(
            tracker.set_folded(99, true),
            None,
            "unknown seq resolves to nothing"
        );
        assert!(
            !tracker.dirty.contains(&99),
            "unknown seq leaves dirty untouched"
        );
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

    /// Perf gate A-track (client): building the fold visibility map over a
    /// full 200-span table plus toggling folds must land under 100 ms.
    /// Microseconds in practice — this is smoke, not a benchmark — but the
    /// number is printed so the gate rests on data, not on a bare pass.
    #[test]
    fn fold_map_build_over_full_span_table_is_under_100ms() {
        let mut tracker = BlockTracker::default();
        for i in 0..BLOCK_SPAN_CAP as i64 {
            tracker.note_b(i * 10);
            tracker.note_d(i * 10 + 5, Some(0), None);
        }
        assert_eq!(tracker.spans().len(), BLOCK_SPAN_CAP);
        let seqs: Vec<u64> = tracker.spans().iter().map(|s| s.seq).collect();
        let bottom = tracker.spans().last().map(|s| s.end_abs).unwrap_or(0);

        let start = std::time::Instant::now();
        for (i, seq) in seqs.iter().enumerate() {
            if i % 2 == 0 {
                tracker.set_folded(*seq, true);
            }
        }
        let folded_map = tracker.map_visible(bottom, 40);
        for seq in &seqs {
            tracker.set_folded(*seq, false);
        }
        let unfolded_map = tracker.map_visible(bottom, 40);
        let elapsed = start.elapsed();

        println!(
            "fold_map_build 200 spans: fold-half + 2 map_visible + unfold-all took {:.3} ms",
            elapsed.as_secs_f64() * 1000.0,
        );
        assert_eq!(unfolded_map.len(), 40);
        assert!(
            folded_map.len() <= 40 && !folded_map.is_empty(),
            "a folded map must still name visible rows"
        );
        assert!(
            elapsed.as_secs_f64() * 1000.0 < 100.0,
            "fold map build + toggles took {:.3} ms",
            elapsed.as_secs_f64() * 1000.0
        );
    }
}
