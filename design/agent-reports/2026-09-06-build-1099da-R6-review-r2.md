# REVIEW r2 — verification of the R6 fold (d81eea8c) against the seam review

Independent read-only verifier. Worktree
`/scratch/code/bitcoin_tax/.claude/worktrees/agent-a4802ae9bcebb07d7`. That worktree's own branch
(`worktree-agent-a4802ae9bcebb07d7`) was stale at an unrelated commit (`2bd04d45`, FR-45 — a leftover
from a prior task on the same worktree name); it was clean (`git status --short` empty), so it was
detached onto **`d81eea8c`** (HEAD of `main`, the commit under review) for this verification and left
there. No branch was altered; no commit, push or edit survives on `d81eea8c` — every plant below was
`cp`-backed-up, run, and restored, confirmed by an empty `git diff --stat` / `git status --short`
after each one and at the end.

Findings under verification: `design/agent-reports/2026-09-06-build-1099da-R6-review.md`
(1C/2I/4M/1N, findings from line 146). Fold under verification:
`design/agent-reports/2026-09-06-build-1099da-R6-fold-implementation.md`. Commit: `d81eea8c` (file
list matches the fold's "Files changed (11)" exactly, confirmed via `git show d81eea8c --stat`).

## Commands run, with summary lines

```
$ export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review

# baseline (matches fold's reported 2451 passed, 7 skipped)
$ cargo nextest run --locked -p btctax-forms -p btctax-core -p btctax-cli -p btctax-tui --no-fail-fast
     Summary [  16.939s] 2451 tests run: 2451 passed, 7 skipped

# C-1 kill, baseline green
$ cargo nextest run --locked -p btctax-cli -E 'test(a_declared_donation_restriction_refuses_the_slice_and_writes_no_8283)'
     Summary [   1.966s] 1 test run: 1 passed

# C-1 PLANT (`working.as_ref().filter(|_| files_from_answers)`) → RED (see table)
# reverted; re-run → green

# I-1 + M-4 kills, baseline green
$ cargo nextest run --locked -p btctax-cli -E 'test(the_slice_clause_is_conditioned_on_the_years_templates_and_on_answers_being_stored) + test(report_does_not_promise_the_slice_on_a_year_with_no_bundled_templates)'
     Summary [   0.150s] 2 tests run: 2 passed

# I-1 PLANT (drop `slice_can_print` term) → 1 RED / 1 still green (see table, expected — different layer)
# reverted; M-4 PLANT (drop `answers_stored` term) → RED
# reverted; re-run → 2 passed

# I-2 baseline green
$ cargo nextest run --locked -p btctax-forms -E 'test(a_pre_2025_slice_keeps_the_whole_part_i_total_on_line_3)'
     Summary [   0.030s] 1 test run: 1 passed

# I-2 + M-1 shared PLANT (`G_LINE3` loses Box C) → BOTH kills RED simultaneously
$ cargo nextest run --locked -p btctax-forms --no-fail-fast
     Summary [   5.896s] 368 tests run: 366 passed, 2 failed, 4 skipped
# reverted; re-run → 368 passed, 4 skipped

# M-2 baseline green
$ cargo nextest run --locked -p btctax-cli -E 'test(the_form_level_gate_runs_on_arm_3_too) + test(unsupported_year_is_refused)'
     Summary [   0.121s] 2 tests run: 2 passed

# M-2 PLANT (`form_level_gate_runs` reduced to `files_from_answers`) → RED
# reverted; re-run → green

# M-3 baseline green
$ cargo nextest run --locked -p btctax-cli -E 'test(an_unanswered_restriction_refuses_a_section_b_8283_and_prints_a_section_a_one)'
     Summary [   0.577s] 1 test run: 1 passed

# M-3 PLANT (`UnansweredSectionB` row disabled in the SHARED gate) → 3 RED at once:
#   the slice kill AND both named full-return kills (return_1040.rs)
$ cargo nextest run --locked -p btctax-cli -p btctax-core -E '...'
     Summary [   0.376s] 3 tests run: 0 passed, 3 failed
# reverted; re-run → green

# N-1 baseline green
$ cargo nextest run --locked -p btctax-cli -E 'test(schedule_d_selected_without_form_8949_is_refused_and_writes_nothing)'
     Summary [   0.199s] 1 test run: 1 passed

# N-1 PLANT (refusal condition forced `false`) → RED
# reverted; re-run → green

# final state check
$ git status --short && git diff --stat        # both EMPTY
$ cargo nextest run --locked -p btctax-forms -p btctax-core -p btctax-cli -p btctax-tui --no-fail-fast
     Summary [  12.245s] 2451 tests run: 2451 passed, 7 skipped   ← identical to baseline
```

Whole-workspace 3185-passed count was **not** re-run (per brief: machine-verified at this commit,
not to be re-established). All scoped counts above match the fold's own reported numbers exactly.

## PROBE 1 reproduction (C-1)

The review's PROBE 1 — a committed `ReturnInputs` row, TY2025's real proceeds-only regime, empty
`broker_reporting`, `donations_had_restrictions = Some(true)`, a donation that emits an 8283, on the
pure production path (`cmd::admin::export_irs_pdf`) — is exactly the
`committed=true with_answers=false restricted=true` cell of the fold's 2×2×2 sweep in
`a_declared_donation_restriction_refuses_the_slice_and_writes_no_8283`
(`crates/btctax-cli/tests/slice_from_answers.rs:1022-1111`). On `d81eea8c` that cell:

- returns `Err` naming `1.170A-7` and `"overstates the gift"`,
- writes no `form_8283.pdf`,
- and `wrote_nothing(&dir)` holds (`!dir.exists() || dir` is empty — no `out_dir` survives), per the
  helper at `slice_from_answers.rs:143-148`.

Planting the pre-fold scoping (`working.as_ref().filter(|_| files_from_answers)`) reproduces PROBE 1's
FAIL-OPEN exactly: the same test then reports `form_8283_path: Some(..)`, `form_8283_needs_review:
true`, `form_8283_section_b: Some(true)` on TY2025's real `{proceeds: true, basis: false}` regime —
the overstated print, in the suite, on the production path.

## Checklist

| Finding | Verdict | The kill | What was planted | Red text (key line) |
|---|---|---|---|---|
| **C-1** (Critical) | **RESOLVED** | `a_declared_donation_restriction_refuses_the_slice_and_writes_no_8283` — 2×2×2 sweep over `{restricted}×{committed,draft}×{answers,none}`, production path, TY2025 real regime | `if let Some(ri) = working.as_ref().filter(\|_\| files_from_answers)` (the pre-fold scoping) | `a declared restriction must refuse: IrsPdfReport { … form_8283_path: Some("…/form_8283.pdf"), form_8283_needs_review: true, form_8283_section_b: Some(true), … }` |
| **I-1** (Important) | **RESOLVED** | `the_slice_clause_is_conditioned_on_the_years_templates_and_on_answers_being_stored`, `report_does_not_promise_the_slice_on_a_year_with_no_bundled_templates` | `slice_clause` stopped consulting `slice_can_print(year)` | `TY2026 bundles no Form 8949 / Schedule D map — the export refuses, so the sentence must not promise a slice: … still prints the crypto slice from the stored answers.` |
| **I-2** (Important) | **RESOLVED** | `a_pre_2025_slice_keeps_the_whole_part_i_total_on_line_3` (now built on C/F rows, asserts both line 3 AND line 10) | `G_LINE3` (`schedule_d.rs`) reduced from `&[C, I]` to `&[I]` | `assertion \`left == right\` failed: the WHOLE Part I total (Box C) on line 3 — left: None right: Some("…")` |
| **M-1** (Minor) | **RESOLVED**, with the recorded deviation confirmed TRUE | `schedule_d::tests::a_box_in_no_group_refuses_and_the_real_groups_partition_every_box`, `all_boxes_is_every_variant` | Same `G_LINE3` plant as I-2 (shared `BOX_GROUPS` constant) | `the real BOX_GROUPS must carry C on exactly one line — geometric read-back FAILED …: the TY2024 Schedule D box→line table carries Form 8949 Box C on 0 of its six lines ([]), …` |
| **M-2** (Minor) | **RESOLVED**, with the recorded deviation confirmed sound | `cmd::admin::slice_broker_tests::the_form_level_gate_runs_on_arm_3_too` + existing `unsupported_year_is_refused` | `form_level_gate_runs` reduced to `files_from_answers` (R6's original condition) | `TY2017 is a bundled slice year: arm (3) must get the form-level gate too` |
| **M-3** (Minor) | **RESOLVED** | `an_unanswered_restriction_refuses_a_section_b_8283_and_prints_a_section_a_one` (slice) + `a_declared_restriction_refuses_at_any_amount_not_just_over_5000`, `an_itemizer_whose_170b_ceiling_zeroes_the_gift_files_no_8283_and_is_not_blocked` (full return) | `UnansweredSectionB` row disabled in the shared `donation_restriction_gate` (`return_refuse.rs`) | All 3 tests failed on the ONE plant: `an unanswered Section B restriction question must refuse: IrsPdfReport { … form_8283_section_b: Some(true), … }` plus 2 `return_1040` FAILs |
| **M-4** (Minor) | **RESOLVED** | `the_slice_clause_is_conditioned_on_the_years_templates_and_on_answers_being_stored` (same test as I-1, different assertion) | `slice_clause` stopped consulting `answers_stored` | `no stored answers ⇒ no "from the stored answers" promise: … still prints the crypto slice from the stored answers. …` |
| **N-1** (Nit) | **RESOLVED** (documentation-only note upgraded to an actual refusal, on both arms) | `schedule_d_selected_without_form_8949_is_refused_and_writes_nothing` | Refusal condition short-circuited to `false` | `called \`Result::unwrap_err()\` on an \`Ok\` value: IrsPdfReport { … f8949_path: None, schedule_d_path: Some("…/schedule_d.pdf"), … }` |

**8/8 RESOLVED.** Every kill went from green (baseline) to red (planted) to green (reverted), on the
exact defect its finding names. Three of the eight plants (I-2/M-1 sharing one constant; M-3's single
plant reaching both the slice and the full return) demonstrate the kills are structurally, not
coincidentally, tied to a shared guarantee.

## The four declared deviations, checked against source

**(a) M-1 — does `Form8949Box` really have eight variants, and does the partition guard cover all of
them?**
Confirmed. `crates/btctax-core/src/forms.rs:43-60`: the enum has exactly eight variants — `C, F, I, L,
G, H, J, K` — no `A`/`B`/`D`/`E`. `BOX_GROUPS` (`schedule_d.rs:116-129`) covers exactly this set once
each (`G_LINE1B=[G]`, `G_LINE2=[H]`, `G_LINE3=[C,I]`, `G_LINE8B=[J]`, `G_LINE9=[K]`,
`G_LINE10=[F,L]`). `box_index` (`schedule_d.rs:418-429`) is an **exhaustive match with no wildcard
arm** over all eight — a ninth variant would not compile until given an index, and
`all_boxes_is_every_variant` catches a variant present-but-omitted from `ALL_BOXES`. The review's
premise (a `Form8949Box::A`) does not exist in this codebase; the deviation's account is accurate.

**(b) M-2 — does keeping the `SUPPORTED_YEARS` term on arm (3) leave any reachable
partially-ported state ungated?**
No. The composition is exhaustive by construction: `form_level_gate_runs(files_from_answers,
tax_year) = files_from_answers || SUPPORTED_YEARS.contains(tax_year)`. On arm (2) it is
unconditionally `true`. On arm (3): if the year is IN `SUPPORTED_YEARS` (≥1 bundled template,
including any partially-ported year), the OR term makes it `true` regardless of arm, so
`slice_map_gate` runs. If the year is NOT in `SUPPORTED_YEARS` (zero templates), `slice_map_gate` is
skipped on arm (3) — but a separate, **unconditional** check at `admin.rs:1011-1015` (runs on both
arms, confirmed by reading it outside any `if files_from_answers`/`form_level_gate_runs` guard, before
`mkdir_out` at `:1017`) refuses that exact case with `CliError::FormFill(FormsError::UnsupportedYear)`
and writes zero bytes — pinned by the existing `unsupported_year_is_refused` test (confirmed run
against a no-answers vault, i.e. arm (3), asserting `!out.path().join("basis_methodology.txt").exists()`
and the same for `form_8275.txt`). So every reachable state — wholly unported or partially ported — is
gated by one check or the other; there is no year for which neither fires. `SUPPORTED_YEARS.contains`
is exactly the boundary between which of the two checks does the gating, not a gap.

**(c) N-1 on both arms — does any existing test or documented journey select Schedule D without
Form 8949?**
No. `grep -rn "FormArg::ScheduleD" --include="*.rs" .` (excluding `target-review`) hits only four
sites: the new gate itself (`admin.rs:947`), two rendering conditionals (`admin.rs:1041`, `:2290`),
and the test file (`export_irs_pdf.rs:481` — the refusal test itself; `:500` — `[F8949, ScheduleD]`
together, still exported). `docs/examples/examples.md:124` and `crates/xtask/src/examples.rs:705`
(the doc/example generator, one production site each) both print `--forms f8949,schedule-d` —
paired, never Schedule D alone. No golden or man page selects the narrowing; the fold's claim that no
test or doc silently refuses on a previously-honored flow is accurate.

**(d) M-3 — is the shared Section-B predicate the SAME expression the full return uses
(`return_1040.rs:2667-2695`), not a copy?**
Confirmed by both reading and mutation. `return_1040.rs:2665-2703` and `admin.rs:823-855` both call
the single function `btctax_core::tax::return_refuse::donation_restriction_gate` (defined once,
`return_refuse.rs:904-925`); only the three call-site arguments (`answer`, `attaches_8283`,
`section_b`) are computed locally, as the doc comment states. A single plant inside that one function
(disabling its `UnansweredSectionB` row) simultaneously reds the slice's own kill AND two full-return
kills (`a_declared_restriction_refuses_at_any_amount_not_just_over_5000`,
`an_itemizer_whose_170b_ceiling_zeroes_the_gift_files_no_8283_and_is_not_blocked`) that were never
touched by this fold — which would be impossible if the two paths carried independent copies of the
rule. This is the strongest evidence in the fold: one line broke three tests across two crates.

## Other checks made

- **Diff scope.** `git show d81eea8c --stat` lists exactly the 11 files (+1 report) the fold's own
  "Files changed" table names; no unlisted file was touched.
- **Ordering on arm (2).** `git diff fb5e7fc3 d81eea8c -- crates/btctax-cli/src/cmd/admin.rs` shows
  the restriction gate is now checked *before* `screen_broker_reporting` on arm (2), where pre-fold it
  ran *after* it (inside the same `if files_from_answers` block, after the screen). This is a genuine
  refusal-priority change for the (declared restriction) ∧ (unresolved broker box) co-occurrence, but
  it is exactly what the original review's own Minimal Change proposed ("before the `if
  files_from_answers` block, or in a shared prelude to both arms") — not an undisclosed side effect —
  and both orderings are fail-closed, zero-byte usage refusals. Not filed as a new finding.
- **Whole-scoped-crate regression check.** After every plant was reverted, `-p btctax-forms -p
  btctax-core -p btctax-cli -p btctax-tui` reproduced the fold's own post-fold count exactly (2451
  passed, 7 skipped) with an empty `git status`/`git diff`.

## New findings

None. No fold defect, no performative kill (every kill discriminated true/false on its named plant),
and no golden or documented flow was found silently broken.

Counts: C=0 I=0 M=0 N=0
