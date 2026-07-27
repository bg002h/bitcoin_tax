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

## Concerns

- None blocking. One judgment call worth flagging: the CLI stderr surface
  fires on `export-snapshot` (the raw CSV/sqlite dump) is intentionally
  **not** wired — only `declare-tranche`, `promote-tranche`,
  `export-irs-pdf`, and the full-return dispatch, per the task's explicit
  command list. `export-snapshot` never emits the notice.
- Commit `9e982ab`'s title still says "+ EXPERIMENTAL.txt" — an artifact of
  the pre-scope-correction commit sequence, corrected in `7a9d184`. Left
  as-is per the "never amend, never `git checkout --`" instructions; the
  code at `HEAD` is authoritative.
