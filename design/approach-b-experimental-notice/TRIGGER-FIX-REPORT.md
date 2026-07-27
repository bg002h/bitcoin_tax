# Trigger-fix: correct the mis-gated Approach-B experimental warning

Branch: `fix/experimental-warning-trigger`, off `main` @ `6a31e3c` (v0.11.0 released).

## The bug

The owner's requirement, verbatim: *"in the UI there must be a warning about this experimental
feature in the UI if and only if a user uses this feature."*

`btctax_core::experimental::uses_approach_b(&events)` answers "has a live (non-voided)
`DeclareTranche`/`PromoteTranche` decision already been recorded" — i.e. *has used*, not *is
using*. `btctax-tui-edit`'s `draw_defensive_filing` (`crates/btctax-tui-edit/src/draw_edit.rs`)
gated its banner on that predicate, so a filer who pressed `w` and landed on the Defensive Filing
dashboard with **no tranche yet** saw **no warning** — at exactly the moment they were deciding
whether to use the feature. The banner only appeared *after* they had already declared.

## What was already correct (verified, not re-touched)

The CLI side (`crates/btctax-cli/src/cmd/tranche.rs::declare_tranche`,
`crates/btctax-cli/src/cmd/promote.rs::promote_tranche`) was already unconditional — a prior "fix
round 1" (commit `7441dc9`, pre-dating this task) had already removed the post-write re-read and
made the stderr warning fire unconditionally after a successful write, never gated on
`uses_approach_b`. `crates/btctax-cli/tests/experimental_notice.rs::declare_tranche_notice_reaches_
stderr_not_stdout` already drove this on a fresh, empty vault and passed pre-existing. No code
change was needed there; this was verified by reading both call sites and running the existing
suite before touching anything.

Browse (`btctax-tui-edit`), the viewer (`btctax-tui`), and the export reports
(`export-irs-pdf`/`export-snapshot`/full-return, `cmd/admin.rs`) were already correctly gated on
`uses_approach_b` — they only REFLECT the feature (the filer's return depends on it), so they stay
predicate-gated by design and were left untouched.

## The fix

`draw_defensive_filing` (`crates/btctax-tui-edit/src/draw_edit.rs:99-`): `show_experimental` is now
an unconditional `true` (was `app.snapshot.as_ref().map(|s| uses_approach_b(&s.events)).unwrap_or
(false)`), kept as a named `bool` so the mutation record is a one-line revert. This screen — the
dashboard and the declare/promote flows it hosts (they render through the same function's content
area) — now always carries the banner, regardless of ledger contents. `open_defensive_filing`
(`editor.rs`) only ever transitions to this screen with a live snapshot, so the change is safe in
production; test fixtures that bypass that opener and set the screen directly also now correctly
show the banner (matching "being on this screen is using the feature").

Doc-precision follow-through: `crates/btctax-core/src/experimental.rs`'s module doc and
`uses_approach_b`'s own doc comment claimed it was "the single gate every surface consults", which
was already stale (the CLI bypassed it) and is now explicitly wrong for the DefensiveFiling screen
too — reworded to state the two-class rule (reflecting surfaces gate on it; feature surfaces don't).

## Tests added/changed

- `crates/btctax-tui-edit/src/draw_edit.rs`:
  - `defensive_filing_experimental_banner_present_on_an_empty_vault` (replaces the old, wrong
    `defensive_filing_experimental_banner_absent_without_approach_b`, which encoded the bug) —
    written first, confirmed RED against pre-fix source, then GREEN after the fix.
  - `browse_experimental_banner_absent_on_an_empty_vault` — the Browse counterpart, pinning the
    opposite (correct) behavior for the reflecting surface.
  - `defensive_filing_declare_flow_shows_the_experimental_banner_on_an_empty_vault` and
    `defensive_filing_promote_flow_shows_the_experimental_banner_on_an_empty_vault` — the two write
    flows that render through this screen, each its own code path.
  - `defensive_filing_declare_flow_banner_composes_with_the_stale_marker_on_an_empty_vault` — full
    composition on an empty vault: banner + flow content + D-7 stale marker, all three, banner
    leads, content sits between banner and marker — proves the layout index arithmetic holds under
    the new unconditional gate, not just under the pre-existing live-tranche case.
  - `defensive_filing_notice_floors_at_one_row_on_a_very_short_terminal` — recalibrated from height
    3 to height 4: at height 3 (`inner.height == 1`) the single available row now goes to the
    now-mandatory banner (which is peeled off first, a pre-existing composition-order precedent),
    not the marker; height 4 is the smallest height at which both get a row and the original "the
    marker still gets SOME rect" property is observable again.
  - `defensive_filing_banner_wins_the_only_row_at_the_most_extreme_height` (new) — documents the
    height-3 tradeoff directly as an intentional, tested fact rather than leaving it as an
    unexplained assertion change: nothing panics, the banner wins, the marker doesn't.
  - Existing `defensive_filing_experimental_banner_composes_with_the_stale_marker` (live tranche +
    armed) and `browse_experimental_banner_*` tests re-verified green, unchanged.
- `crates/btctax-cli/tests/experimental_notice.rs`:
  - `an_unrelated_command_on_a_fresh_vault_never_emits_the_notice` — `report` on a fresh, empty
    vault emits the notice on neither stream (the "silent elsewhere" half of the biconditional,
    driven through a real, unrelated command rather than inferred from the closed call-site set).
- Goldens regenerated (content-only, mechanical): `docs/examples-tui/btctax-tui-edit-defensive-
  filing-pseudo-stub-export-refused.txt` and `...-stale-armed-no-status.txt` now include the banner
  row (both underlying fixtures carry no snapshot/tranche, so this is the direct, expected
  consequence of the fix). `stale_dashboard_goldens_never_clip_the_export_line` re-verified green
  against the regenerated content — `[x] export` is not clipped by the extra row.

## Mutation proof

Reverted `show_experimental`'s unconditional `true` back to the predicate
(`app.snapshot.as_ref().map(|s| uses_approach_b(&s.events)).unwrap_or(false)`) via a `cp`-backed
edit (never `git checkout --`): `defensive_filing_experimental_banner_present_on_an_empty_vault`
went RED with the exact pre-fix symptom (empty-vault dashboard renders with no banner). Restored
via the `cp` backup and re-ran GREEN.

## Scope discipline

`crates/btctax-forms/` untouched. No export-writing path touched; the four directory-walk
leak-guard tests in `experimental_notice.rs` (`notice_text_is_absent_from_every_file_in_the_export_
directory`, `export_snapshot_notice_absent_from_every_file_in_the_export_directory`,
`full_return_export_notice_absent_from_every_file_in_the_export_directory`, and the AcroForm
field-value check) all re-verified green — the notice still reaches nothing the export directory
produces.

## Gate

`make check`: 2535/2535 pass (baseline 2529 + 6 new tests), 0 clippy warnings.
`cargo fmt --all -- --check`: clean.
