# Build report — Approach-B cross-surface experimental disclosure

## Status

DONE. Branch `feat/approach-b-experimental-notice`, 6 commits on top of
`d06e765`. `make check` green (2520/2520, 11 skipped) and
`cargo fmt --all -- --check` clean after the final commit.

Final surfaces: CLI stderr (`declare-tranche`, `promote-tranche`,
`export-irs-pdf` crypto-slice + full-return dispatch), a banner row in both
`btctax-tui` (viewer) and `btctax-tui-edit` (editor), and a new section in
the repo-root `NOTICE`. Per a mid-task scope correction from the owner, the
notice is **interface-only** — it is never written to any export directory
(an earlier `EXPERIMENTAL.txt` sibling-file design was built, then fully
removed in commit `7a9d184`).

## Commits

- `83e5751` — `feat(core)`: new `btctax_core::experimental` module —
  `ExperimentalNotice` struct + `NOTICE` const + `plain_text()`, and the
  `uses_approach_b(events)` liveness predicate (reuses
  `tranche_guard::void_targets`).
- `0f3ebe6` — `feat(cli)`: stderr wiring for `declare-tranche` /
  `promote-tranche` (inline `eprintln!`, mirroring the phantom-wallet
  precedent) and `export-irs-pdf` (a new `IrsPdfReport::experimental_notice`
  bool field, printed by `main.rs`, mirroring that command's existing
  report-field convention). This commit also added an export-directory
  write, later removed.
- `9e982ab` — `feat(tui)`: banner row in `btctax-tui`'s `draw_viewer`,
  mirroring `btctax-tui-edit::draw_browse`'s PSEUDO-RECONCILE row mechanism
  (`Constraint::Length(1)` + index bookkeeping). This commit also added an
  export-directory write, later removed.
- `a80dd83` — `feat(tui-edit)`: a second banner row in `draw_browse`,
  composing with the existing PSEUDO banner (pseudo leads).
- `4f37784` — `docs(notice)`: new "EXPERIMENTAL — DEFENSIVE FILING" section
  in the repo-root `NOTICE`, composing with the existing disclaimer text.
- `7a9d184` — `fix(experimental)`: scope correction — removed
  `render::write_experimental_notice_txt` and all four call sites (CLI
  export-snapshot/export-irs-pdf/full-return, TUI CSV export); fixed banner
  wording that referenced the now-nonexistent file; strengthened the guard
  tests to walk the WHOLE export directory rather than checking three named
  files. This is the commit that makes the final state correct — the first
  three `feat` commits are superseded on this one point (git history is
  immutable; the code at `HEAD` is what matters).

## Test summary

`make check`: 2520 passed, 11 skipped, 0 failed (workspace-wide, nextest +
clippy `-D warnings`). `cargo fmt --all -- --check`: clean.

Coverage added: the `uses_approach_b` predicate (live declare/promote →
true; voided-only → false; declare voided but its promote still live →
true; unrelated events → false) in `btctax-core`; CLI stderr-not-stdout for
each of the four commands plus silence-on-refusal and
silence-on-voided-only; the TUI banner row's appearance/absence and its
proven layout shift (content pane down one row, footer pinned to the last
row) in both `btctax-tui` and `btctax-tui-edit`, including composition with
the PSEUDO-RECONCILE banner; and — the load-bearing set — three
directory-wide guards (CLI crypto-slice export, CLI full-return export, TUI
CSV export) asserting the notice text is absent from every byte of every
file the export writes, plus a decoded-AcroForm-field-value check
specifically on the 8275 PDF.

## Mutations (TRUE — both actually run, observed red, then reverted via `cp`)

1. `write_form_8275_txt_named` (render.rs): appended
   `NOTICE.plain_text()` after the disclosure body. Ran
   `notice_text_is_absent_from_every_filed_artifact` (pre-scope-correction
   guard) → **FAILED**, correctly naming `form_8275.txt` and the leaked
   text. Reverted via `cp` from a pre-mutation backup; re-ran green.
2. Same mutation, re-applied after the scope correction. Ran the
   strengthened `notice_text_is_absent_from_every_file_in_the_export_directory`
   and `full_return_export_notice_absent_from_every_file_in_the_export_directory`
   → both **FAILED**, correctly naming `form_8275.txt`. Reverted via `cp`;
   re-ran green (8/8 in `experimental_notice.rs`).

No `git checkout --` was used for any mutation; both used a `cp` backup
taken immediately before the edit, verified byte-identical after restore.

## Concerns (as of the original build, superseded below)

- None blocking. One judgment call worth flagging: the CLI stderr surface
  fires on `export-snapshot` (the raw CSV/sqlite dump) is intentionally
  **not** wired — only `declare-tranche`, `promote-tranche`,
  `export-irs-pdf`, and the full-return dispatch, per the task's explicit
  command list. `export-snapshot` never emits the notice.
  → **Fixed in fix round 1** (item #4 below): `export-snapshot` writes the
  same disclosure files `export-irs-pdf` does and now emits the notice too.
- Commit `9e982ab`'s title still says "+ EXPERIMENTAL.txt" — an artifact of
  the pre-scope-correction commit sequence, corrected in `7a9d184`. Left
  as-is per the "never amend, never `git checkout --`" instructions; the
  code at `HEAD` is authoritative.

## Fix round 1 (independent two-lens review, relayed by the coordinator)

Verbatim review persisted at `reviews/r1.md` before folding. Six blocking
Important items plus four pre-publish items plus three nits, all folded in
one pass. Commits (on top of `2df251d`):

- `df0ce2c` — persist the review verbatim.
- `924162a` — `fix(core)`: #1 (dropped the "two defects" exhaustive count;
  `defects` is now documented/used as exemplary; added the third,
  worse defect — a filed Form 8949 column (e) basis derivable from a stale
  in-editor snapshot, `design/stale-snapshot-latch/DESIGN.md` §0.1, shipped
  in v0.10.0, whose own design doc records the deferral justification was
  "False"); #5 (replaced the unperformable "against your own records"
  action with three concrete checks against what the feature actually
  produces; restated defect 2 by consequence); #2 core half (new
  `ExperimentalNotice::one_line()`, a derived single-line rendering);
  #6 (new `notice_fields_are_presentation_neutral` test); pre-publish
  `#[non_exhaustive]` + `Serialize` + `testonly::leak_guard_needles()`.
- `3cfd721` — `fix(tui)`: #2 (viewer banner now calls `one_line()`; new
  drift test at an 800-col backend).
- `a29a535` — `fix(tui-edit)`: #2 (editor banner now calls `one_line()`
  via a shared `experimental_banner_text()` helper); #3 (new banner row on
  `draw_defensive_filing` — the screen the whole declare/promote/export
  journey runs on, which had no banner at all — composing above the D-7
  stale marker via a nested split; extended the Browse composition KAT to
  this screen).
- `7441dc9` — `fix(cli)`: #4 (`export-snapshot` now computes
  `uses_approach_b`, surfaces it on `ExportReport`, and `main.rs` emits the
  notice beside the existing blocker disclosure; fourth directory-wide leak
  guard for its out_dir); pre-publish (`cmd/tranche.rs`/`cmd/promote.rs`
  post-write re-read no longer uses `?` on a disclosure-only step — it is
  now unconditional, since the predicate is trivially true after a
  successful write); nits (shared `leak_guard_needles()` needle list;
  `experimental_notice` → `experimental_notice_active`; `eprintln!` →
  `eprint!` at all four stderr call sites, since `plain_text()` already
  ends in `\n`).
- `0befac0` — `docs`: pre-publish (`LIMITATIONS.md` + `README.md` EXPERIMENTAL
  sections); regenerated `docs/examples/examples.md` (stale after the
  content rewrite).

### Test summary (fix round 1)

`make check`: 2529 passed, 11 skipped, 0 failed. `cargo fmt --all -- --check`:
clean. Net +9 tests over the pre-round-1 baseline (2520): 3 in
`btctax-core` (`one_line_is_derived_from_title_summary_and_action`,
`notice_fields_are_presentation_neutral`,
`leak_guard_needles_covers_every_field`), 2 in `btctax-cli`
(`export_snapshot_notice_reaches_stderr_not_stdout_and_is_absent_without_approach_b`,
`export_snapshot_notice_absent_from_every_file_in_the_export_directory`), 1
in `btctax-tui`
(`viewer_experimental_banner_is_derived_from_notice_not_hand_copied`), 3 in
`btctax-tui-edit`
(`browse_experimental_banner_is_derived_from_notice_not_hand_copied`,
`defensive_filing_experimental_banner_composes_with_the_stale_marker`,
`defensive_filing_experimental_banner_absent_without_approach_b`).

### Mutations (fix round 1, TRUE — each actually run, observed red, then reverted via `cp`)

1. `experimental.rs::NOTICE.summary`: replaced one `\`-continuation with an
   embedded `\n`. Ran `notice_fields_are_presentation_neutral` → **FAILED**
   (caught the embedded newline). Reverted via `cp`; re-ran green (11/11 in
   `experimental::tests`).
2. `experimental.rs::NOTICE.summary`: reworded ("Two defects…Both are
   fixed." → "Bugs…every known one is now resolved."). Ran
   `viewer_experimental_banner_is_derived_from_notice_not_hand_copied`
   (`btctax-tui`) → **FAILED**. Reverted via `cp`; re-ran green.
3. Same mutation. Ran
   `browse_experimental_banner_is_derived_from_notice_not_hand_copied`
   (`btctax-tui-edit`) → **FAILED**. Reverted via `cp`; re-ran green.
4. `render.rs::write_form_8275_txt_named`: re-appended
   `NOTICE.plain_text()` after the disclosure body (the export-snapshot
   writer, not the crypto-slice/full-return one this function also
   serves). Ran `export_snapshot_notice_absent_from_every_file_in_the_export_directory`
   → **FAILED**, correctly naming `form_8275.txt`. Reverted via `cp`;
   re-ran green (10/10 in `experimental_notice.rs`).

No `git checkout --` was used for any mutation; every one used a `cp`
backup taken immediately before the edit, verified byte-identical (`diff`)
after restore.

## Concerns (current, after fix round 1)

None blocking. `make check` and `cargo fmt --all -- --check` are green at
`HEAD`.

## Merge + release

Per the user's durable pre-authorization (`design/defensive-filing-wizard/CONTINUITY.md`,
2026-07-26: "You may push, merge, tag & release and do crates when
ready."), and the coordinator's "then merge and a crates.io release":

1. Merged `feat/approach-b-experimental-notice` into `main` (`89561c5`,
   `--no-ff`). `make check` (2529/2529) + `cargo fmt --all -- --check`
   green post-merge.
2. Bumped all 12 workspace crates 0.10.0 → 0.11.0 (minor: additive
   feature, pre-1.0) in lockstep — each crate's own `version` field AND
   every inter-crate `version =` requirement — `4455ba1`. Regenerated
   `docs/man/btctax-update-prices.1` + `docs/examples/examples.md` (their
   drift gates would otherwise catch the stale version string).
3. Verified the full pre-authorized gate before publishing: `make check`,
   `cargo fmt --all -- --check`, `cargo check/clippy --workspace --locked`,
   `cargo run -p xtask -- check-isolation` (net-isolation), `bash
   scripts/pii-scan-generic.sh` (pii-scan, clean), `cargo test -p
   btctax-forms --test census --locked` (forms census), `make docs`, and
   `make bundles` — all green. (MSRV against the pinned 1.88 toolchain was
   NOT re-verified locally — only a newer toolchain was available — left
   to CI, which runs on push.)
4. Pushed `main` + tag `v0.11.0` to `origin` (`git@github.com:bg002h/bitcoin_tax.git`).
5. Created the GitHub release
   (https://github.com/bg002h/bitcoin_tax/releases/tag/v0.11.0).
6. `cargo publish --workspace --dry-run` clean, then `cargo publish
   --workspace` for real — all 10 publishable crates (`btctax`,
   `btctax-core`, `btctax-store`, `btctax-adapters`, `btctax-forms`,
   `btctax-input-form`, `btctax-cli`, `btctax-update-prices`,
   `btctax-tui`, `btctax-tui-edit`) uploaded and confirmed available in
   ONE run — no tail failure this time (contrast the v0.7.0/v0.9.0
   "internal-errors after 9/10" precedent this branch's CONTINUITY.md
   warned about). Verified live via the crates.io API (a bare `curl`
   without a descriptive User-Agent gets a 403 from crates.io's API
   policy; `-A "btctax-release-check (goss.brian@gmail.com)"` works) —
   spot-checked `btctax-core`, `btctax`, `btctax-cli`, `btctax-tui-edit`
   all report `newest_version: 0.11.0`.

**★ Per the CONTINUITY.md note: the crates.io token in
`~/.cargo/credentials.toml` was used for this publish and should be
REVOKED now that it is done its job.**
