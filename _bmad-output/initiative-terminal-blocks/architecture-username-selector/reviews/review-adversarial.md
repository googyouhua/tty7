# Adversarial Review — architecture-username-selector

Scope: spine `architecture-username-selector.md` vs one level down:
`src/ui/instance_picker.rs` (PickerState/InputState/modal), `crates/tty7-core/src/core/instance.rs`,
`crates/tty7-core/src/daemon/singleton.rs` (`claim`/`holder_pid`/`clear_record_if_free`).

Method: for each attack surface, construct Unit A and Unit B that each obey
every AD to the letter yet combine / behave incompatibly. Each pair is a hole;
each hole names an AD tightening. No pair below requires violating any AD as written.

Verdict: **CONDITIONAL FAIL — 6 holes, all closable.** The layering (AD-1),
validation authority (AD-6), and companion non-duplication (AD-9) hold up.
The delete path (AD-2/AD-3), memory ownership (AD-7), and mtime sourcing (AD-4)
do not: each admits two compliant-but-incompatible implementations. Do not build
until AD-2/AD-3/AD-7 are tightened per F1–F3; F4–F6 should ride in the same edit.

---

## F1 — TOCTOU: probe-then-remove deletes a just-started server (AD-2 + AD-3) — SEVERE

**Compliant pair:**

- Unit A (picker + core, sequential): `if holder_pid(dir).is_none() { remove_instance(name) }`.
  Obeys AD-3 ("before remove_instance, probe") and AD-2 ("validates first, then removes whole").
  Nothing in either AD forbids a gap between the two calls; the spine's state-flow row
  ("core call → memory write") even pictures them as separate steps.
- Unit B (server start, concurrent): `tty7 --as alice server start` claims the seat
  (`singleton::claim` → `LOCK_EX`, `record_holder_pid`) in the window between A's probe
  and A's `remove_dir_all`. B obeys every AD — startup is out of the slice's scope
  and no AD constrains it.

**Incompatibility:** A deletes a live server's config dir out from under it
(sockets, state, lock file included). The daemon never took the failure mode the
singleton module was built to prevent — it *is* the split-machine in reverse:
a running server whose endpoint directory vanishes mid-serve. AD-3's "prevents:
deleting an instance whose server is still running" is false under concurrency.

**Why the ADs allow it:** "Before" is not "atomically with". No AD (a) merges
probe+remove into one core operation, (b) requires a re-probe / lock held across
the unlink, or (c) defines the residual-race acceptance. AD-2's `*_in` test seam
also has no time seam, so the race is untestable by construction.

**Suggested tightening (AD-2 + AD-3):**

- Make the delete a single core operation, e.g.
  `remove_instance_in(root, default_dir, name)` performs validate → probe →
  `remove_dir_all` internally (picker never calls probe itself), and re-probes
  immediately before the unlink; on `Some(pid)` after a `None`, abort with the
  same refusal report.
- State the residual window explicitly: even re-probe leaves a µs gap; document
  it as accepted-for-v1 (same style as AD-4's accepted mtime skew) and require
  the warn-level log (Consistency/Logging row) to include both probe outcomes.
- Add a concurrency test obligation: probe-held → remove refuses even when the
  holder started after the first probe (hold `LOCK_EX` on `dir/daemon.lock` in
  the test, then call the single-operation remove).

## F2 — Probe and delete disagree on which directory they mean (AD-1 + AD-3) — SEVERE

**Compliant pair:**

- Unit A (global probe): picker calls existing `singleton::holder_pid()`, which reads
  `config_path("daemon.lock")` — the *globally pinned* config dir. AD-3's parenthetical
  ("`config_path("daemon.lock")` resolves against the global pinned dir") describes
  exactly this function, so A obeys AD-3 to the letter.
- Unit B (dir-scoped delete): `remove_instance_in(root, …, name)` unlinks
  `<root>/tty7-<name>`. Obeys AD-2 ("removes `tty7-<name>` whole").

**Incompatibility:** A probes the wrong seat for every non-default instance.
Deleting `alice` while `alice`'s server runs but the default seat is free →
false "free", deletes live. Conversely default seat held → false "live", refuses
a safe delete of an idle instance with a plausible-but-wrong pid in the report.
The report's stop command (`--as <name>`) and `rm -rf <dir>` then name a different
instance than the pid belongs to.

Symlink/rename variant of the same hole (same ADs, no AD violated): `dir_for`
joins `root + tty7-<name>` with no canonicalization; `list_in` accepts anything
`is_dir()` (follows symlinks). Between probe of `<dir>/daemon.lock` and
`remove_dir_all(<dir>)` a rename/symlink swap redirects the delete. AD-1 forbids
the *view* from touching paths directly but says nothing about the module
resolving the same path twice the same way.

**Suggested tightening (AD-3 + AD-1 + Conventions/Naming):**

- Replace the parenthetical with a hard rule: the probe MUST be dir-scoped —
  `holder_pid_in(dir)` (or a private `instance.rs` wrapper taking `dir`) that never
  consults the globally pinned dir; picker calls only the instance-module wrapper.
  Calling `singleton::holder_pid()` from the picker becomes a named AD-1 violation.
- Require single-resolution: `remove_instance_in` resolves `dir` once (after
  `validate_name`), probes and unlinks that same `PathBuf`; refuse when the resolved
  path is a symlink / not a real dir (`symlink_metadata`, `FileType::is_symlink`
  check) rather than following it.

## F3 — Two owners of `.last-instance` race and neither validates (AD-7) — MAJOR

**Compliant pair:**

- Unit A (picker memory): `apply_selection` → `write_last(memory_name)` and
  post-delete focus write. Obeys AD-7 ("only with a confirmed selection or the
  post-delete focus row").
- Unit B (explicit path memory): `remember_current()` → `memory_name_for(dir)` →
  `write_last`. Also obeys AD-7's letter — AD-7 binds "`.last-instance` writes"
  but only constrains the picker's two call sites; `remember_current` (explicit
  `--config-dir` launch, `instance_picker.rs:90-97`) is an existing second writer
  the spine never names. Both writers issue bare `fs::write` with no locking.

**Incompatibility:** two processes (bare picker confirming `bob` while an explicit
`--as alice` launch remembers `alice`) interleave `create_dir_all + write` —
last-writer-wins, preselect flaps. Worse: `write_last` itself performs zero
validation, so AD-7's "never write unvalidated or deleted names" is enforced (if
at all) at two call sites by convention. Unit A validates in the view, Unit B
trusts `memory_name_for`; a future third caller obeying AD-7's letter writes
anything — including a just-deleted name, resurrecting the dangling preselect
AD-7 claims to prevent.

**Suggested tightening (AD-7):**

- Name both writers: the memory has exactly two producers (`apply_selection` /
  post-delete focus, and `remember_current`); no other call site may call
  `write_last*` — or, better, move enforcement into core: `write_last_in`
  validates (`sentinel || validate_name`) and refuses deleted names
  (resolve-then-check-exists), returning `Err` the callers map to best-effort skip.
- State the race policy: concurrent bare+explicit launches are last-writer-wins,
  accepted for v1 (no file locking); the post-delete focus write MUST be ordered
  after a successful `remove_instance` returns, never before/alongside.

## F4 — mtime is read twice, ordering flips mid-render (AD-4) — MAJOR

**Compliant pair:**

- Unit A (snapshot): `list_in` returns `(name, mtime)` pairs captured in one pass;
  view sorts/renders the snapshot. Obeys AD-4 ("`metadata().modified()` on
  `tty7-<name>`; no new state file").
- Unit B (re-read per render): view calls the mtime helper on every frame for the
  names from an earlier `list_names`. Equally obedient — AD-4 names the source
  but not the read count, timing, or sort key stability.

**Incompatibility:** live sessions write session state under the instance dir
concurrently, so mtimes advance between list and render (and between rows of one
render). A and B display different orders for the same underlying state; B can
reorder rows under the user's cursor between frames, so Enter confirms a different
row than the highlighted one a frame earlier. AD-4's Deferred note ("mtime-skew
hardening … beyond display") concedes restore/copy skew but not this live-update
flip, which is *within* display.

**Suggested tightening (AD-4):**

- Require snapshot semantics: one listing pass captures `(name, mtime-or-None)`;
  sort and render derive from that snapshot only — no per-frame re-read during
  the picker's lifetime (re-list only on explicit refresh, if any).
- Tie-break deterministically (e.g. mtime desc, then name asc) and say mtime
  ties/missing meta keep the `list_in` name order, so A and B cannot diverge.

## F5 — The report names a pid that may already be someone else (AD-3) — MODERATE

**Compliant pair:**

- Unit A (verbatim report): probe returns `pid`; modal renders "holder pid 12345,
  run `tty7 --as alice server stop`, then `rm -rf <dir>`". Obeys AD-3's report
  line list exactly.
- Unit B (verified report): same probe, plus a "pid N (unverified — confirm it is
  still `tty7` before acting)" label and a `kill -0` / `/proc` identity note.
  Also obeys AD-3 — nothing forbids extra caution text.

**Incompatibility:** `daemon.lock` content is written once at claim; `singleton.rs`
already documents the stale-record hazard (`clear_record_if_free`, pre-recording
builds, pid reuse). A crashed-then-reaped holder whose record was not cleared,
or a pid recycled by the OS, puts an innocent pid in a modal next to a
`rm -rf` command and a stop command. A obeys the spine and still coaches the user
toward `kill`-ing or suspecting the wrong process. The singleton docs push the
identity check onto "the caller's identity check on the pid" (`holder_pid`
docs) — AD-3 names no caller check at all.

**Suggested tightening (AD-3):**

- Label the pid advisory in the report spec: "may be stale/reused; the picker
  never kills; verify (e.g. `ps -p <pid>`) that it is still this instance's
  server before stopping anything."
- Forbid the modal from ever growing a kill button (picker never kills — extend
  the existing "user stops the server themselves" sentence to "and the modal
  offers no kill action, only the stop command string").

## F6 — Non-unix warn-and-allow deletes a live server by default (AD-3) — MODERATE

**Compliant pair:**

- Unit A (allow): non-unix `holder_pid()` is hard-coded `None`
  (`singleton.rs:260-268`); view maps `None` → free → deletes after a warning
  line. Obeys AD-3 ("non-unix … warns … and allows the delete").
- Unit B (refuse-or-confirm): same `None`, mapped to *unknown*, requiring an
  explicit typed confirmation (or refusing outright). B's author also claims
  AD-3 compliance: the AD mandates a warning but never defines what "verification
  was skipped" permits, and AD-2's "on Err the modal stays open" shows the
  spine knows how to refuse.

**Incompatibility:** on Windows (share_mode seat, pid unreadable by design) A
deletes live servers 100% of the time with only a warning line; B refuses the
same delete. Both cite the same sentence. The spine's "prevents: deleting an
instance whose server is still running" is silently platform-conditional, and
the modal's `rm -rf <dir>` line is unix-shaped on the platform least able to use it.

**Suggested tightening (AD-3):**

- Distinguish the probe's three outcomes in the rule: `Live(pid)` → refuse +
  report; `Free` → proceed to remove; `Unknown` (non-unix, unreadable lock) →
  warn AND require an explicit second confirmation (or refuse — pick one, and say
  which). Never map `Unknown` to `Free`.
- Fix the report's platform shape: non-unix report must show the platform-native
  removal command, not `rm -rf`.

---

## Minor / consistency nits (no new AD needed, fold into the tightenings)

- AD-3 says the picker "owns no fs paths" (AD-1) yet renders "config dir path"
  and an `rm -rf <dir>` string: clarify the picker may *display* a dir string
  returned by core but never *resolve or construct* one (today `dir_for` +
  string interpolation in the view would both look compliant).
- AD-5's "modal stacks never exceed one deep" collides with AD-2's "on Err the
  modal stays open with the error line": state whether delete-error replaces the
  confirm modal's body (one modal, two states) vs. opens a second layer.
- AD-7's "post-delete focus row" needs the ordering in F3's fix: focus is computed
  from the post-remove list, and the write happens after remove returns `Ok`.

## What held up (no hole found)

- AD-1 layering vs. probe placement survived one probe: the probe *read* path via
  `singleton::holder_pid()` called directly from the picker is arguably AD-1-clean
  (AD-1 bans view-side *mutation*), which is exactly why F2's fix must explicitly
  route the probe through the instance module — the current text is consistent but
  insufficient, not contradictory.
- AD-5/AD-6 single-owner rules admit no compliant-but-incompatible pair beyond the
  already-noted modal-depth nit; typed-wins cannot resurface without violating AD-5.
- AD-8/AD-9 scoping is tight; no builder can obey them and still hardcode or drift.
