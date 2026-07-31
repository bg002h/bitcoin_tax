# Whole-diff review — crypto-slice trio + FBAR reversal — INSTRUMENT-INTEGRITY lens (Sonnet, r1)

**Date:** 2026-07-30 · **Branch:** `feat/no-pen-deferrals` · **Range:** `7bde148..65270db`.

**Brief:** not tax correctness (a separate Opus reviewer had that lens) but **B1**: *for each
guarantee this diff claims, which test reds when the guarantee is removed — and is that test actually
capable of reddening?* The reviewer was required to MUTATE source and run tests, not merely read, and
to restore from a cp backup (never `git checkout --`, which eats uncommitted work). Vacuous-assertion
hunting was named explicitly.

**Reviewer output is reproduced VERBATIM below.**

---

Tree confirmed clean, no lingering diffs.

## Summary

I mutated the source behind each of the six claimed guarantees, ran the targeted tests, and restored every file from backup (never `git checkout --`). Every mutation was killed by an existing test. One boundary case (not one of the six named guarantees, but part of guarantee 1's predicate) survived and is reported below.

`VERDICT: all-guarantees-held`

`MUTATION LOG:`

| guarantee | mutation applied | result | test that fired |
|---|---|---|---|
| 1. `niit_at_margin` predicate, conjunct 1 | drop `nii_with >= 0` → `magi_with > thr` alone | KILLED | `niit_at_margin_is_false_when_nii_has_slack_below_zero` |
| 1. `niit_at_margin` predicate, conjunct 2 | drop `magi_with > thr` → `nii_with >= 0` alone | KILLED | `st_gain_stacks_on_ordinary` |
| 1. boundary resolution (`>` vs `>=` at threshold) | `magi_with >= thr` instead of `>` | **SURVIVED** | none (see finding below) |
| 2/3. `schedule_d_line17` routing | drop the loss/zero → `None` skip, always answer | KILLED | `schedule_d_line17_is_derived_on_every_revision` (+ pre-existing `printed.rs` routing tests: `schedule_d_routing_zero`, `schedule_d_routing_net_loss_line21_is_a_positive_magnitude`, `form_1040_line7_on_a_loss_year_is_the_limited_amount_with_a_leading_minus`) |
| 2. line-17 answer ignores line 15's sign | `then_some(true)` regardless of `line15` | KILLED | `schedule_d_line17_is_derived_on_every_revision` |
| 2. 2017 on-state analogy-copy | 2017 map `"Yes"/"No"` → `"1"/"2"` | KILLED | `schedule_d_line17_is_derived_on_every_revision` |
| 3. crypto slice fills line 18 (a field the shared 2024 map actually has) | added a write+placement for `map.line18` | KILLED (year 2024; 2017/2025 maps structurally lack the field, so nothing to plant there) | `schedule_d_crypto_slice_leaves_lines_18_through_22_blank` |
| 4. crypto-slice 1040 loses its worksheet stamp | removed `stamp_partial_worksheet_watermark` call | KILLED | `crypto_slice_1040_is_watermarked_as_a_worksheet` |
| 4. full-return 1040 gets the worksheet stamp | added `stamp_partial_worksheet_watermark` to the full-return per-form loop | KILLED | `export_dispatches_a_full_return_year_to_the_full_packet` |
| 5. advisory fires on an explicit "No" too | `!= Some(true)` instead of `.is_none()` | KILLED | `the_fbar_sub_question_advises_only_when_it_was_skipped` |
| 5. advisory fires without a 7a "Yes" | dropped the outer `foreign_accounts == Some(true)` gate | KILLED | `the_fbar_sub_question_advises_only_when_it_was_skipped` (+ `a_clean_high_income_return_has_only_the_unconditional_omission`, `omissions_fire_together_in_order`) |
| 6. reintroduce a refusal (bypassing `FORM_QUESTIONS`) | manual `if 7a==Yes && sub.is_none() { refuse }` | KILLED | `the_fbar_sub_question_does_not_refuse_a_return` (first `assert_eq!`) |
| 6. silently stop asking the sub-question | removed `SkippableId::FbarFilingRequired` from `SKIPPABLE_QUESTIONS` | KILLED | `the_fbar_sub_question_does_not_refuse_a_return` (second `assert!`, registry-membership check) |

```
SEVERITY: Minor
WHERE: crates/btctax-core/src/tax/compute.rs:405 (niit_at_margin: magi_with > thr && nii_with >= Usd::ZERO)
CLAIM: types.rs's doc comment claims the predicate is "boundary-resolved... exactly AT the threshold reports the lower answer," matching the strict `>` used elsewhere (ltcg's `top <= max_zero`).
PROOF: changing `magi_with > thr` to `magi_with >= thr` left all 21 tax_compute tests, golden_returns, and the whole `make check` suite green. No test exercises MAGI exactly at the §1411 threshold ($200k/$250k/$125k), so the boundary claim is asserted in prose but not pinned. Severity is Minor because the field is display-only (feeds no tax figure, only the "all-in rate" headline) and the exposure is a single-dollar boundary.
```

Files touched only transiently for mutation testing (all restored, `git status --short` clean, `make check` green at 2508/2508 on the restored tree): `crates/btctax-core/src/tax/compute.rs`, `crates/btctax-core/src/tax/printed.rs`, `crates/btctax-core/src/tax/advisories.rs`, `crates/btctax-core/src/tax/return_refuse.rs`, `crates/btctax-core/src/tax/questions.rs`, `crates/btctax-forms/src/schedule_d.rs`, `crates/btctax-forms/forms/2017/schedule_d.map.toml`, `crates/btctax-cli/src/cmd/admin.rs`.

`TREE CLEAN: yes`

Note on guarantee 6's first `assert!` (the `FORM_QUESTIONS.iter().any(...contains("Fbar"))` check): logically it reduces to "no `FORM_QUESTIONS` entry's `Debug` name contains `Fbar`" — the value-equality conjunct alongside it is decorative (it's trivially satisfiable whenever the name filter matches, since a genuine reintroduction reads the same field). This isn't a false-pass — a real reintroduction is caught earlier, by the preceding `assert_eq!`, as verified by mutation — so it's redundant rather than broken. Not raised as a SURVIVED finding since I did not find a mutation that gets past the `assert_eq!` above it and is only caught (or missed) by this clause.

---

## Disposition (author, same day)

**13 of 14 mutations killed.** The one that survived is exactly the failure B1 exists to catch: a
guarantee stated in a doc comment, believed, and held by nothing — *"a guarantee without a test that
reds when it is removed does not exist."*

**FIXED INLINE:** `niit_at_margin_reports_the_lower_answer_exactly_at_the_threshold` in
`crates/btctax-core/tests/tax_compute.rs` pins all three of $199,999 / $200,000 / $200,001 with a
fixture whose `crypto_agi == 0`, so `magi_with == magi_excluding_crypto` exactly and the boundary is
addressable at all. Re-running the reviewer's own mutation (`>` → `>=`) now reds it. The frozen-engine
hash of `compute.rs` was re-verified byte-identical to its pin after the restore.

The redundant conjunct in `the_fbar_sub_question_does_not_refuse_a_return` is **left as is**: the
reviewer established by mutation that a real reintroduction is caught by the `assert_eq!(reason(&r),
None)` above it, so the clause is belt-and-braces rather than a false PASS. Removing it would buy
nothing and lose a cheap tripwire on the registry.
