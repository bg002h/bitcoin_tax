# Independent review — Phase 3 "Report surfaces" (UX-P4-6 + UX-P4-10), fold verification (r2)

Scope reviewed: fold commit `648a563` (diff `027a89d..HEAD`), verified against current source, with the r1 review (`design/usage-examples/reviews/ux-p4-6-10-impl-fable-review-r1.md`) as the baseline. Validation run by me: `make check` — **2046 passed / 0 failed / 8 skipped** (r1 baseline 2044; the fold adds exactly the 2 claimed tests). Standing caveat: `make check` is nextest+clippy only, not the CI-only jobs (fmt/msrv/pii-scan/net-isolation).

## What I verified

**Diff scope.** The fold touches exactly two files: `crates/btctax-cli/tests/report_exit_code.rs` (+90/−2) and the persisted r1 review. Zero production code. The three pre-existing KATs are intact and unweakened — the only non-additive change is the module doc-comment, which now correctly names BOTH SPEC §3.5 exit-0 non-triggers (widened, not narrowed).

**I1 fold — the dual-report KAT genuinely hits `screen_absolute` case (c), non-vacuously.** Traced end-to-end:

- *The refusal is real and is case (c).* The fixture ($0 wages, `answered` ReturnInputs, `cf.short = 1000`) yields AGI = −1000 (Sch D line 7 = −min(1000, 3000)); TI clamps to zero (`return_1040.rs:1207`: `(agi - total_deductions).max(Usd::ZERO)`), so `screen_absolute` (c) at `return_1040.rs:1437` fires (`TI == 0 && cf.short > 0`). Cases (a) and (b) are unreachable on this input: (a) requires `has_qbi(...)` (`qbi.rs:90` — business QBI, REIT dividends, and carryforward all zero → false before the threshold is even consulted); (b) stops at `amt.rs`'s first gate (`line5 = −1000 ≤ exemption` → false). So (c) is the *only* reachable refusal — the doc-comment's claim is exact.
- *The resolver screen does NOT also fire.* `screen_compute_dependent` (`return_1040.rs:584`) has no TI/carryforward row — its rows are the 8283 noncash aggregate (no `schedule_a`), business interest (none), Schedule C (none), and kiddie tax (skipped: `not_a_dependent()` sets `can_be_claimed_as_dependent_taxpayer = Some(false)`). Had it fired, `resolve_and_screen` returns `Uncomputable` → `report_tax_year` returns `Err` → the test's `.unwrap()` panics — so the test *structurally cannot* pass with a dead delta. The delta `outcome == Computed` assertion holds for the right reason (ReturnInputs-derived profile, 2024 bundled table, no hard blockers, no disposal).
- *"NOT COMPUTABLE" is a unique marker inside the dual block.* I swept every production emitter of that string: within the `dual_report` string the ONLY uppercase source is the `screen_absolute` refusal branch (`cmd/tax.rs:335`). The computes-branch delta line uses lowercase `"not computable"` (`render.rs` `render_dual_report`) — case-mismatched AND excluded anyway by the `Computed` assert; `render_advisories` messages contain no such string; the other sites (`render_tax_outcome`:1184, schedule-d:1464, harvest:2288, main.rs banners) render outside `dual_report`. No false-positive path.
- *The exit-0 assertion is load-bearing.* The lib-level asserts pin the vault to the refused-absolute/computed-delta state; the binary run traverses the identical (unmutated — `report_tax_year` is read-only) vault.

**Mutation evidence (run by me, not taken on the author's word; main.rs cp-backed-up and restored, tree confirmed clean after):**

- **A (remove exit-1** — predicate → `false`): reds exactly `report_notcomputable_exits_one` + `report_hard_blocker_exits_one`; all exit-0 KATs pass. ✓
- **B (always fire** — predicate → `true`): reds exactly the three exit-0 KATs, including the new dual-report one. ✓
- **C (kind-narrowed** — fire only on `TaxProfileMissing`): reds exactly `report_hard_blocker_exits_one` — the M1 fold's distinct-kind pin is doing real work. ✓
- **D (the precise I1 regression** — predicate extended with `|| dual_report.contains("NOT COMPUTABLE")`): reds exactly `report_dual_report_absolute_refused_delta_computed_exits_zero`, and nothing else could catch it (the other exit-0 KATs have no dual block). This is the regression the plan's KAT was ordered to hold, and it now dies. ✓

**M1 fold — the hard-blocker KAT pins a genuinely distinct kind.** `write_buy_receive_2025` without pseudo/profile: the resolver's arm 4 returns `Ready { profile: None, provenance: Missing }` (`resolve.rs:130-135` — NOT `Uncomputable`, so no exit-2 shortcut), and `compute_tax_year` checks the Hard blocker (clause 1, `compute.rs:246`) BEFORE the missing-profile branch (clause 3) — so the outcome kind is `TaxYearNotComputable`, not `TaxProfileMissing`, despite the profile also being absent. Mutation C confirms empirically. This is exactly r1's prescribed fix; `TaxTableMissing` remains lib-level-only, the residue r1's prescription explicitly accepted.

## CRITICAL

None.

## IMPORTANT

None.

## MINOR

None.

## NIT

**N1 (carried from r1, unchanged, non-gating)** — no vault-level projection-through-render test for the pending line; held by the fold.rs derivation invariant.

**N2 (new, optional)** — the dual-report KAT could additionally assert the refusal reason substring (`TaxableIncomeNonPositiveWithCarryforward`, which the `[{:?}]` render exposes) to pin the fixture to documented case (c). Not needed for the contract — the exit-code non-trigger is deliberately refusal-agnostic, and (a)/(b) are unreachable on this input — so this is documentation-strength only.

## STATUS

- **I1 — RESOLVED.** The plan-mandated dual-report absolute-refused/delta-computed exit-0 KAT exists, hits `screen_absolute` case (c) non-vacuously (lib-level state proof + unique refusal marker), and its binary-level exit-0 assert is the sole killer of the exact predicate-extension regression (mutation D). Module doc widened to name both non-triggers.
- **M1 — RESOLVED** (as prescribed). Binary-level exit-1 now pinned for the hard-blocker kind, distinct from `TaxProfileMissing` by compute.rs clause ordering; a kind-narrowed predicate now dies (mutation C).

## VERDICT

**GREEN — 0 Critical / 0 Important.** Full suite 2046/2046 passed (delta +2, exactly the fold); diff is test-only; all four mutations I ran flip exactly the tests they should.
