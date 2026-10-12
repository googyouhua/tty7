# Rubric-walker review — architecture-username-selector

Spine: `_bmad-output/initiative-terminal-blocks/architecture-username-selector/architecture-username-selector.md`
Driving inputs: `../ux-username-selector/DESIGN.md` + `EXPERIENCE.md` (status final)
Brownfield: `src/ui/instance_picker.rs`, `crates/tty7-core/src/core/instance.rs`,
`crates/tty7-core/src/daemon/singleton.rs`, `crates/tty7-core/src/core/config.rs`

## Overall verdict

**Adequate, with two high findings to fix before build.** The spine fixes the real
divergence points for this slice (layering, delete seam, liveness gate, filter
ownership, validation authority, memory rule, i18n, UX binding) and ratifies the
brownfield rather than contradicting it. Named tech is verified-current. The gaps
are: (1) the liveness-probe home/signature is left as "either/or", which lets two
units diverge; (2) the probe's non-unix tri-state is unspecified, so the
warn-and-allow rule is not enforceable against `holder_pid() -> None` meaning both
"free" and "unavailable"; (3) the operational envelope (probe→delete TOCTOU,
Windows relaunch-after-delete, concurrent pickers) is silent. None of the Deferred
items leak divergence for v1.

## Per-criterion verdicts

| Criterion | Verdict | Note |
|---|---|---|
| Fixes the real divergence points for the level below, misses none | adequate | All seven true divergence points are bound (AD-1–AD-8). One placement (probe home) is left open — see F-1. |
| Every AD Rule is enforceable and prevents its stated divergence | thin | AD-1, AD-2 (mostly), AD-4–AD-8 enforceable. AD-3's probe rule is not fully enforceable: no fixed signature/home, no tri-state — see F-1, F-2. |
| Nothing under Deferred could let two units diverge | strong | Trash-vs-permanent (permanent pinned), pinyin (substring pinned by UX + spine), mtime-skew (display-only pinned), remote inventory (modal-inline only) — all closed for v1. |
| Named tech is verified-current | strong | Rust edition 2024 matches workspace `Cargo.toml:4`; gpui + gpui-component workspace pin (zed rev / tty7 branch) matches `Cargo.toml:317-327,470-473`. No new deps. |
| Ratifies rather than contradicts the brownfield codebase | adequate | Layering, `*_in` seam convention, `validate_name` authority, memory file, `config_path` global-only analysis all check out. One unmarked override of an EXPERIENCE assumption (memory rewrite ownership) — see F-3. |
| Covers driving inputs' capabilities | adequate | Every EXPERIENCE capability maps (filter/highlight/dropdown, explicit create, delete+liveness report, preselect+memory, last-used meta, strings, surface detail). Relaunch-failed ownership is implicit only — see F-4. |
| Every dimension the altitude (feature) owns is decided, deferred, or open — no silent whole dimension, esp. operational/environmental envelope | thin | Operational envelope is the silent dimension: no TOCTOU rule, no Windows relaunch-after-delete/create rule, no concurrency rule — see F-5. |

## Findings

### F-1 [high] — Liveness-probe home and signature left as either/or (AD-3 + Consistency + Structural Seed)

- **Location:** spine AD-3 Rule ("new dir-scoped probe"); Consistency Conventions
  ("probe reads as `holder_pid_in(dir)`-shaped if added to singleton, else a
  private fn in the instance module"); Structural Seed
  ("dir-scoped probe alongside or in instance.rs").
- **Problem:** three phrasings permit at least two homes (`singleton.rs` as
  `holder_pid_in(dir)` vs. private fn in `instance.rs`) with different
  signatures and different test seams. Two builders implementing "the probe"
  can land in different modules with different names — the exact divergence a
  spine must prevent. AD-1 (core purity) does not disambiguate because both
  homes are inside core.
- **Fix suggestion:** pick one. Recommended: `pub fn holder_pid_in(dir: &Path)
  -> Option<u32>` in `singleton.rs` (unix; non-unix stub per F-2), re-exported
  or called by the instance module; probe takes `&Path` to `<instance-dir>`,
  never a name. Update Consistency + Seed to the single home and name the
  `*_in`-style test seam explicitly.

### F-2 [high] — Non-unix probe outcome is not representable with `Option<u32>` (AD-3)

- **Location:** spine AD-3 Rule ("Non-unix (probe unavailable): modal warns
  verification was skipped and allows the delete").
- **Problem:** brownfield `singleton::holder_pid()` (`singleton.rs:260-268`)
  returns `Option<u32>` where `None` means both "seat free" and "cannot ask"
  (Windows `share_mode(0)` cannot even open the file; the comment says the
  #667 recovery is unix-only). A dir-scoped probe returning `Option<u32>`
  collapses "free, safe to delete" and "unknown, warn-and-allow" into the same
  `None`. The UI cannot implement the warn-vs-proceed distinction from that
  signature, so the rule as written is not enforceable.
- **Fix suggestion:** specify a tri-state return (e.g. an enum
  `Live(u32) / Free / Unknown`, or `Option<Option<u32>>` with documented
  meaning) for the new dir-scoped probe on all platforms, and state the UI
  mapping: `Live(pid)` → refuse + report; `Free` → proceed to confirm path;
  `Unknown` → warn line + allow. Name the non-unix stub behavior explicitly.

### F-3 [medium] — Memory-rewrite ownership on delete contradicts EXPERIENCE without saying so (AD-2 + AD-7 vs EXPERIENCE.md:16,58)

- **Location:** spine AD-2 ("returns `io::Result`; the UI maps… no memory
  rewrite" on Err; silent on who rewrites on Ok) + AD-7 ("`write_last` is
  called only with… the post-delete focus row") vs. EXPERIENCE.md:16,58
  ("new core API `remove_instance(name)` (delete … + rewrite memory)").
- **Problem:** the spine's actual decision (UI owns the post-delete
  `write_last` to the focus row; core removes the dir only) is the right
  layering and matches AD-1, but it silently overrides the driving input's
  `[ASSUMPTION]` that core rewrites memory. A builder reading both docs can
  implement the rewrite twice (core + UI) or, if core's `remove_instance`
  later gains the rewrite, double-write. The conflict-resolution rule (AD-9:
  this spine wins on structural rules) technically covers it, but memory
  ownership is not named as a structural rule winning over that assumption.
- **Fix suggestion:** add one sentence to AD-2 or AD-7: "`remove_instance`
  removes the directory only and never touches `.last-instance`; the
  post-delete memory rewrite is owned solely by the UI per AD-7. This
  supersedes the EXPERIENCE.md `[ASSUMPTION]` that core rewrites memory."

### F-4 [medium] — Windows relaunch-failed state has no owner (EXPERIENCE.md:76 vs spine silence)

- **Location:** EXPERIENCE.md:76 ("Relaunch failed (Windows) …
  `InstancePickerRelaunchFailed` … Unchanged semantics"); spine AD-8 (string
  family) + Capability map (no relaunch row).
- **Problem:** brownfield `instance_picker.rs:219-240` implements
  Windows-only `relaunch_with` + `relaunch_failed` flag. The spine adds delete
  and create paths that also resolve directories but never states whether
  they flow through the same relaunch/error path on Windows, nor who owns the
  `RelaunchFailed` line after the redesign. Low divergence risk (one file,
  one builder) but a whole capability from the driving input has no
  architecture owner.
- **Fix suggestion:** add a Capability-map row ("Windows relaunch +
  relaunch-failed line → `instance_picker.rs` finish path, unchanged; delete
  and create confirmations reuse it") or one sentence in AD-8/Conventions
  assigning it. No new AD needed.

### F-5 [medium] — Operational envelope silent: TOCTOU, concurrency, permissions (AD-3)

- **Location:** spine AD-3 (probe-then-delete with no atomicity rule);
  Consistency Logging row (warn/info only).
- **Problem:** at feature altitude the envelope is owned, not inherited:
  (a) probe→delete is TOCTOU — a server can start between the shared-lock
  probe and `remove_dir_all`; the spine does not say whether delete
  re-checks, ignores, or best-effort-fails; (b) two pickers / CLI `--as`
  racing the same instance have no stated rule; (c) permission failures
  (0700 leaf, shared-box parent, read-only root) map to the generic modal
  error line by implication only. Each is small, but together they are a
  silent dimension, and (a) directly weakens AD-3's "never delete a live
  server" guarantee to "never delete a server live at probe time."
- **Fix suggestion:** add one Conventions or AD-3 sentence: "The probe is a
  pre-delete gate, not a lock: it is evaluated immediately before
  `remove_instance` in the confirm handler with no await/yield between; a
  delete that fails because the directory changed under it surfaces the
  modal error line and rewrites no memory (AD-2). No cross-process delete
  exclusion for v1." That decides the dimension without adding machinery.

### F-6 [low] — `remove_instance_in` takes two dirs without saying why (AD-2)

- **Location:** spine AD-2 ("`remove_instance_in(root, default_dir, name)` …
  following the existing `*_in` convention").
- **Problem:** brownfield convention is one dir per seam (`list_in(root)`,
  `read_last_in(default_dir)`). The two-arg form is presumably root (locate
  `tty7-<name>`) + default_dir (memory adjacency — though F-3 says core does
  not touch memory, so why does the seam need `default_dir`?). A builder
  could "simplify" to one arg and break the test contract, or pass the same
  dir twice.
- **Fix suggestion:** document each parameter's role in one clause, e.g.
  "`root` locates `tty7-<name>`; `default_dir` is accepted for
  symmetry/future memory adjacency but the delete touches only `root`" —
  or drop `default_dir` from the seam if core never touches memory.

## Mechanical notes

- Verified against code (not copied claims): `config_path("daemon.lock")`
  resolves via process-global `config_dir()` (`config.rs:1356-1358`,
  `1241-1248`) — the spine's "cannot address another instance" analysis is
  correct. `holder_pid()` is unix-only with a `None` non-unix stub
  (`singleton.rs:166-193,260-268`) — underlies F-2. `validate_name` reserves
  `default` and rejects leading `-` (`instance.rs:37-50`) — AD-6/AD-2 sentinel
  handling is accurate. `PickerState::confirm` typed-wins
  (`instance_picker.rs:143-152`) is the divergence AD-5 kills — correctly
  identified. Stop command `tty7 --as <name> server stop` matches documented
  CLI shape (`docs/remote/shared-root.mdx:83`, `skills/tty7/references/commands.md:24,445`).
  Stack versions match workspace `Cargo.toml`.
- AD-5's delegation of keyboard/focus/a11y detail to EXPERIENCE.md via AD-9
  (rather than restating) is correct layering, not a gap; the circular
  reference (AD-5→AD-9→companions) resolves cleanly because AD-9's precedence
  rule is explicit.
- No hallucinated APIs: every brownfield symbol cited in the spine
  (`list_names`, `validate_name`, `dir_for`, `ensure_dir`, `read_last`,
  `write_last`, `default_root`, `InputState`, `InstancePicker*` keys)
  exists at the cited paths.
