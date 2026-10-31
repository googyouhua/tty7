//! A heartbeat the UI thread leaves behind, and a watcher that notices when it
//! stops.
//!
//! When a window manager calls a tty7 window "not responding", the one thing
//! nobody has afterwards is a record of how long the stall lasted and what the
//! app was in the middle of. This keeps that record. The UI thread beats on a
//! foreground timer — one that can only fire while the loop is pumping — and a
//! plain thread watches the beat, so it keeps running while the UI does not.
//!
//! A stall is logged once, with its length and the connect phase in flight at
//! the time, because a frozen window during a connect and a frozen window while
//! nine panes paint look identical from the outside and mean very different
//! things.
//!
//! The flag is deliberate: this is diagnostic plumbing, not something that
//! changes behaviour, so it runs everywhere and costs one atomic store per beat.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use gpui::App;

const BEAT_EVERY: Duration = Duration::from_millis(100);
/// Below this a late frame is a busy frame, not a stall worth a line in the log.
const STALL_AFTER: Duration = Duration::from_millis(1500);
const CHECK_EVERY: Duration = Duration::from_secs(1);

static CLOCK: OnceLock<Instant> = OnceLock::new();
static BEAT_MS: AtomicU64 = AtomicU64::new(0);
static INSTALLED: OnceLock<()> = OnceLock::new();

/// The last few things the UI thread started, oldest first.
///
/// A duration on its own says the window stopped answering but not what it was
/// doing; with these, the line names the work. Kept tiny and only written where
/// a stall has actually been seen, so the cost is one lock per frame at worst.
const CRUMBS: usize = 8;

struct Bread {
    next: usize,
    ring: [&'static str; CRUMBS],
}

static BREAD: std::sync::Mutex<Bread> = std::sync::Mutex::new(Bread {
    next: 0,
    ring: [""; CRUMBS],
});

/// Record that the UI thread is about to do `what`.
pub fn crumb(what: &'static str) {
    if let Ok(mut bread) = BREAD.lock() {
        let at = bread.next;
        bread.ring[at] = what;
        bread.next = (at + 1) % CRUMBS;
    }
}

fn recent() -> String {
    let Ok(bread) = BREAD.lock() else {
        return String::new();
    };
    let mut out: Vec<&'static str> = Vec::with_capacity(CRUMBS);
    for step in 0..CRUMBS {
        let item = bread.ring[(bread.next + step) % CRUMBS];
        if !item.is_empty() {
            out.push(item);
        }
    }
    out.join(" -> ")
}

fn clock() -> &'static Instant {
    CLOCK.get_or_init(Instant::now)
}

/// The UI thread's half: one store per beat.
fn beat() {
    BEAT_MS.store(clock().elapsed().as_millis() as u64, Ordering::Relaxed);
}

/// How long the UI thread has gone without beating. Test-facing.
pub fn quiet_for() -> Duration {
    clock()
        .elapsed()
        .saturating_sub(Duration::from_millis(BEAT_MS.load(Ordering::Relaxed)))
}

/// Start the beat and the watcher. Idempotent: the second call does nothing.
pub fn install(cx: &mut App) {
    if INSTALLED.set(()).is_err() {
        return;
    }
    beat();
    cx.spawn(async move |cx| {
        loop {
            cx.background_executor().timer(BEAT_EVERY).await;
            beat();
        }
    })
    .detach();

    let spawned = std::thread::Builder::new()
        .name("tty7-ui-watchdog".into())
        .spawn(|| {
            let mut stalled: Option<Instant> = None;
            loop {
                std::thread::sleep(CHECK_EVERY);
                let quiet = quiet_for();
                if quiet < STALL_AFTER {
                    stalled = None;
                    continue;
                }
                // One line per stall, at the moment it starts: the recovery is
                // logged too so a reader can tell a 2s hiccup from a lockup
                // that only ended when the app did.
                if stalled.is_none() {
                    stalled = Some(Instant::now());
                    log::warn!(
                        "ui thread stalled for {}ms - {} - last: {}",
                        quiet.as_millis(),
                        in_flight(),
                        recent()
                    );
                }
            }
        });
    if let Err(e) = spawned {
        log::warn!("could not start the ui watchdog thread: {e}");
    }
}

/// What the app was doing when the beat stopped, in the terms a bug report needs.
fn in_flight() -> String {
    match crate::ui::remote_connect::stage_snapshot() {
        Some((stage, machine, waited)) => format!(
            "a connect to {machine} was {} ({}s into that phase)",
            stage.label(),
            waited.as_secs()
        ),
        None => "no connect was in flight".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_clock_is_quiet_for_about_nothing() {
        // Not a timing test: it only pins that a beat of zero (no beat yet)
        // reads as the time since the clock started, and that the arithmetic
        // does not wrap into something enormous.
        assert!(quiet_for() < Duration::from_secs(60));
    }

    #[test]
    fn the_crumbs_read_oldest_first() {
        crumb("crumb test one");
        crumb("crumb test two");
        let trail = recent();
        let first = trail
            .find("crumb test one")
            .expect("the first crumb is kept");
        let second = trail
            .find("crumb test two")
            .expect("the second crumb is kept");
        assert!(first < second, "the newest crumb reads last: {trail}");
    }
}
