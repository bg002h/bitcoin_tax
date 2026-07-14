# FOLLOWUPS — full-return v1

Non-blocking items deferred from the spec/plan review loop. Fold at plan time or later; none gates.

## From Phase 2 implementation (open at the P2 review; deviations recorded for traceability)

- **p2-absolute-assembly-deferred-to-P4** (DEVIATION — plan P2 task 1 → P4) — the absolute WITH-crypto 1040
  income-assembly struct (L1a..L11 + the four Schedule-D routing paths + the `L11 = L9 − L10` cross-foot KAT)
  is **deferred to P4**, where it is first *consumed* (the delta-vs-absolute dual report, SPEC §6) AND where
  ½-SE (L15) is available (P2 would have to stub it → a knowingly-incomplete AGI). Building it in P2 would be
  consumer-less, stubbed dead code that P4 rebuilds. **What P2 delivers instead:** the *derivation* side
  (`derive_tax_profile`, the frozen-seam profile the delta engine consumes) + the reusable crypto-figure
  helpers (`crypto_income`, `capital_gain_line7`) the absolute assembly will reuse. YAGNI + no-stub
  justification; the cross-foot invariant is `L11 = L9 − L10` by construction and gets its KAT in P4.
- **p2-consumer-sweep-remaining** (was `p1-consumer-sweep-P2`; RESOLVED in P2 — routing) — every computing
  consumer now goes through the shared, fail-closed `Session::resolve_screened{,_profile}` (→
  `resolve_and_screen`: resolve_profile + input-screen + compute-dependent screen): **report**, **optimize**
  (run/consult/accept), **what-if** sell+harvest fallback (the ad-hoc-arg path stays ad-hoc), **TUI**
  `optimize_proposal`, and **admin/export** + the **prior-year** M4 advisory (both map an uncomputable
  outcome to "skip", never failing a data export / non-gating advisory). All existing consumer tests pass
  unchanged (behavior-identical for non-pseudo, non-ReturnInputs years).
- **p2-provenance-printing** (SCHEDULED → P4, with the dual-report rendering) — the resolver **mechanism** is
  done in P2: `resolve_screened` / `ProfileOutcome::Ready` carry the `Provenance`. PRINTING it + a
  `provenance_label` formatter (§4.12 "provenance printed on every output") is owned by **P4**, where the
  full-return-aware output format
  (delta-vs-absolute dual report, §6) is built — P2 still emits the existing crypto-delta report, so a
  provenance line has no finished output to live in yet and a stderr stopgap would be thrown away. Non-fail-
  open (the number is already correct + fail-closed); this is an audit-trail nicety. Print it as part of the
  P4 report render.

## From Phase 3 implementation (open at the P3 review; deferrals recorded for traceability)

- **p3-carryover-writeback-P4** (DEVIATION — plan P3 task 3d → P4) — the charitable-carryover WRITE-BACK
  (persist the computed `carryover_out` + the R3-M6 precedence: computed overwrites computed, refuses
  user-entered w/o `--force`) is deferred to P4. Reason: the REAL carryover includes crypto-donation excess,
  which needs the ABSOLUTE Schedule A (crypto donations from the ledger) — a P4 piece. The derive-side
  non-crypto `carryover_out` is intentionally discarded (it is not the filed carryover). No fail-open in P3:
  nothing persists a wrong carryover. **P4 riders before trusting `apply_170b`'s `carryover_out` (P3 review):**
  (i) the non-50%-org classes are now REFUSED upstream (review C1 fold) and negative AGI is clamped to zero
  (review M1 fold — `apply_170b` + `schedule_a_deduction`), so `carryover_out` is trustworthy for the
  in-scope (50%-org, non-negative-AGI) input space; do NOT re-open non-50%-org allocation without the
  `p3-non50org-charitable-special-limit` cross-terms. (ii) P4 must **hoist the `apply_170b` call out of the
  `ri.schedule_a.as_ref().map_or(...)` guard** (return_1040.rs) so a filer with `charitable_carryover_in`
  but no Schedule A block still ages/expires carryover (G8, Reg. §1.170A-10(a)(2), review M2) — today a
  std-deduction year silently skips the engine. P4 then wires the absolute Sch A in + persists.
  (iii) `apply_170b` is `pub` and enforces its "50%-org classes only" precondition NOWHERE itself — the
  guarantee lives entirely in the upstream `screen_inputs` refuse (C1). When P4 adds the absolute-Sch-A
  caller, it MUST also route through `screen_inputs` (or `apply_170b` must gain a `debug_assert!`/boundary
  error rejecting any non-50%-org class), else a P4 caller that skips the screen would silently DROP a
  non-50%-org gift → overstate tax (conservative, but a fail-open of the C1 guarantee). Review r2 N1 (Minor).
  **P4.1 progress:** rider (ii) DONE — `assemble_absolute` calls `apply_170b` **unconditionally** on user
  gifts + the ledger's §170(e) crypto donations (`crypto_charitable_gifts`: LT→`CapGainProp30` FMV,
  ST→`OrdinaryProp50` basis) at with-crypto AGI, so `AbsoluteReturn.charitable_carryover_out` ages even in
  a std-deduction year (KAT `crypto_donation_over_ceiling_carries_over_even_in_std_year`). Rider (iii)
  satisfied by routing (the assembly's contract requires the refuse screens; crypto classes are 50%-org by
  construction). **RESOLVED (P4.9):** the persistence + R3-M6 precedence shipped — new `CarryProvenance`
  (User/Computed, `#[serde(default)] User`) on `CharitableCarryItem` + `QbiInputs`; core
  `apply_carryover_writeback(ar, next_year, force)` stamps the computed charitable + QBI carryover-out into
  year (Y+1)'s carryover-in as `Computed`, overwriting a Computed value silently but **refusing a
  User-entered one without `--force`** (atomic across both). CLI: `report --tax-year Y --write-carryover
  [--force]` (opt-in — `report` stays read-only by default). KATs `writeback_*` (core, 3) +
  `carryover_write_back_round_trips_and_respects_user_precedence` (CLI). See `p4-9-capital-loss-writeback`.
- **p3-l16-absolute-P4** (DEVIATION — plan P3 task 4 → P4 — **RESOLVED in P4.1b**) — L16
  (`method.rs::qdcgt_line16` on the WITH-crypto AGI) + the QBI stub are ABSOLUTE-return lines; P3 shipped the
  frozen-DELTA path. **P4.1a** landed QBI (real Form 8995, not a 0-stub); **P4.1b** landed L16:
  `AbsoluteReturn.regular_tax = qdcgt_line16(ordinary_for(status), ltcg_for(status), L15, L3a, net_ltcg)` on
  the WITH-crypto TI, covering all four §7.2 Schedule-D routing paths (KATs `l16_*`, cent-exact vs the
  deep/01 examples). Closed.
- **p3-non50org-charitable-special-limit** (follow-on — now GUARDED by a refuse) — the non-50%-org charitable
  classes (Cash30, OrdinaryProp30, CapGainProp20) are **REFUSED upstream** by `screen_inputs`
  (`RefuseReason::NonPublicCharityContribution`, review C1) whenever they appear as a current gift OR a
  carryover-in. Rationale for the C1 fold: the original P3 impl gave them own-% ceilings under an *independent*
  30%-of-AGI room, which OMITS the statutory cross-terms — §170(b)(1)(B)(ii) caps them at the LESSER of 30%·AGI
  or (50%·AGI − the 50%-org tiers already allowed), and §170(b)(1)(D)(i)(II) caps CapGainProp20 by
  (30%·AGI − the CapGainProp30 class), not by non-50%-org cash/ordinary usage. That let totals reach 90%·AGI
  where the law caps at 60/50% → a SILENT tax UNDERSTATEMENT (the prior "conservative" claim here was FALSE;
  probes: AGI $100k, $50k Cash60 + $30k Cash30 → law $50k / old engine $80k; $30k CGP30 + $20k CGP20 → law
  $30k / old engine $50k). These classes are never produced by the crypto ledger and are "capture-only rare"
  per SPEC §4.6, so refuse (fail-loud) is the correct v1 posture. **To SUPPORT them later:** implement the two
  Pub. 526 Worksheet-2 cross-terms above (same shape as the shipped R2-I1 50%-org line), add KATs pinning both
  probe scenarios to the CORRECT law totals, then drop the refuse. KATs pinning the current refuse:
  `non50org_cash_gift_refuses`, `non50org_capgain_gift_refuses`, `non50org_carryover_in_refuses`.
- **p3-crypto-donation-delta-integration** (design Q — derive-side exclusion RULED CORRECT at P3 review r1 §3.3;
  absolute/delta treatment → P4) — the crypto-donation §170 deduction is today an advisory-only "before §170(b)"
  figure in the report; how (or whether) it enters the frozen DELTA tax vs only the absolute Schedule A. **P3
  reviewer ruling (r1 §3.3):** the derive-side EXCLUSION is correct and must stand — (a) `apply_170b`'s allowed
  total is monotone nondecreasing in gifts, so excluding crypto gifts can only OVERSTATE the reported tax
  (conservative); (b) non-crypto AGI for the derived Sch A is architecturally FORCED by the frozen seam (a
  with-crypto AGI would contaminate `tax(base)` so it no longer equals the no-crypto counterfactual — SPEC §6);
  (c) the one residual anti-conservative channel is the **medical floor** (with-crypto AGI shrinks the true
  7.5% floor; the derivation-fixed deduction cannot re-shrink) — known, documented (SPEC §6), not new in P3.
  **P4 requirements carried from the ruling:** crypto donations MUST enter the ABSOLUTE Schedule A (ledger
  §170(e) classes at with-crypto AGI, G7), and P4's `absolute_with − absolute_without ≠ delta` KAT (plan P4
  task 8) MUST use a **medical-floor fixture** so the one anti-conservative direction is the one pinned.
  **RESOLVED:** crypto donations enter the absolute Sch A (P4.1a `crypto_charitable_gifts` +
  `absolute_schedule_a_includes_lt_crypto_donation_at_fmv` KAT); the medical-floor divergence KAT
  `section6_medical_floor_delta_understates_and_does_not_reconcile` (P4 review r1 I3 fold) computes BOTH
  `absolute_with − absolute_without` AND the frozen delta on a $20k-medical fixture, asserting `delta <
  absolute contribution` (the delta UNDERSTATES — the one anti-conservative channel) and non-reconciliation.

## From Fable IMPL-P4 code review r1 (1C/4I FOLDED → r2 GREEN 0C/0I; **Phase 4 CERTIFIED at `018e199`**)

- **p4-r2-nit-forceitemize-noscheda-label** (FOLDED — r2 Nit) — `itemized_was_chosen` labeled a
  `ForceItemize`-with-no-Schedule-A ($0) deduction "standard" in the dual report; now returns "itemized"
  (matching `choose_deduction`'s §63(e) itemized arm), KAT `deduction_is_itemized_reflects_the_election`.
  Label-only, reviewer-pre-approved fix direction → no r3 gate round (Nits don't gate; cf. P2 r4 precedent).


- **p4-r1-c1-qss-se-addl-medicare** (FOLD C — CRITICAL, shipped) — `se_addl_medicare_threshold` gave QSS the
  $250,000 joint threshold; §1401(b)(2)(A)(iii) + the 2024 Form 8959 chart put a **QSS at $200,000** (not a
  joint return). Fixed in unfrozen `tables.rs` (QSS → $200k arm) + KATs `form_8959_qss_uses_200k_threshold_not_250k`
  and the `statutory_values_are_constant_across_years` pins. **`niit_threshold` LEFT at $250,000 for QSS** —
  §1411(b)(1) expressly includes "a surviving spouse", a deliberate statutory asymmetry (do not "unify" them).
  Frozen `se.rs` only *calls* the fn → files byte-identical.
- **p4-r1-i4-dividend-subset-screen** (FOLD I — shipped) — `screen_inputs` now refuses a 1099-DIV whose box 1b
  (qualified) or box 5 (§199A) exceeds its box 1a (ordinary) — a corrupt import that gave preferential/QBI
  treatment to income never in AGI (`RefuseReason::InconsistentDividendSubset`, KAT `dividend_subset_inconsistency_refuses`).
- **p4-r1-m3-ctc-advisory-P5** (**RESOLVED in P5**) — `Advisory::CtcOdcOmitted` fires whenever dependents
  are captured, printed in the full-return block (KATs: core `advisories`, CLI
  `full_return_report_surfaces_conservative_omission_advisories`). Original entry: — plan P4 task 7's CTC/ODC
  "loud advisory" half. The **compute** is done (L19 = 0, `ctc_odc_conservatively_omitted_l19_zero`); the
  *advisory surfacing* ("you have N dependents; CTC/ODC omitted — Schedule 8812 filed separately") is a render
  concern owned by **P5** ("wire the conservative-omission advisories into report/output", SPEC §9.2). Direction
  is conservative (overstates tax only). P5's entry-sweep must pick this up; nothing understates in the interim.
- **p4-r1-n1-taxyearreport-struct** (**RESOLVED in P5**) — `cmd::tax::TaxYearReport` is now a NAMED STRUCT
  (was a 7-tuple); every call site destructures by field, so a new field can't transpose. Original entry: — `cmd::tax::TaxYearReport` is now a 7-tuple of
  `Option<String>`s; name it a struct (named fields) before P5 adds the advisory field, so an 8th positional
  element can't silently transpose. Non-behavioral.

## From Fable IMPL-P3 code review r1 (C1/I1/I2 + M1 FOLDED at the P3 gate; M2/M3 folded into entries above; deferrals here)

- **p3-i1-dependent-spouse-refuse** (FOLD C — refuse shipped) — `header.can_be_claimed_as_dependent_spouse`
  was captured (`return_inputs.rs`) but had ZERO consumers, so an MFJ return with a claimable-as-dependent
  spouse got the full basic std (understated tax by up to ~$27,900). Now REFUSED by `screen_inputs`
  (`RefuseReason::DependentSpouseUnsupported`, KAT `dependent_spouse_flag_refuses`) rather than mis-computed:
  the 1040 Std-Deduction-Worksheet-for-Dependents limit (spouse box → §63(c)(5) limited basic, household-Σ
  earned income) is unmodeled in v1, and the legally-consistent input space is narrow (the joint-return test
  usually makes a claimable spouse a refund-only filer). **To SUPPORT later:** extend the §63(c)(5) floor
  trigger to taxpayer-OR-spouse on MFJ with MFJ earned income = household Σ, then drop the refuse.
  **SPEC/RECON ERRATUM (record-only, do not re-open the gate):** deep/04 §1.2 lists the dependent-spouse
  checkbox as a CONSUMED input, but §1.3's std-deduction pseudocode and SPEC §4.7 both silently drop it —
  the source of the unconsumed-flag gap. Fix the spec/recon text when §170/§63 is next revised. See also the
  spec-errata section below.
- **p3-m3-dependent-floor-earned-income-G21** (DEVIATION → P4 — **RESOLVED in P4.1**) — the §63(c)(5)
  dependent-floor earned income (SPEC §4.7/G21 = "Σ box1 + Schedule C net − ½SE") passed **wages only** in
  P3 (the conservative interim, pending ½-SE). **P4.1 completes it on the ABSOLUTE side:**
  `assemble_absolute` now passes `dependent_earned = max(0, wages + Schedule C net − ½-SE)` to
  `standard_deduction` (KAT `dependent_floor_uses_g21_with_crypto_earned_income`). The DERIVE side
  intentionally stays wages-only — its non-crypto profile has no Schedule C (crypto is excluded by the
  frozen seam), so wages-only is not just conservative but *exact* there. Closed.
- **p3-m4-none-dob-forfeited-63f-advisory** (**RESOLVED in P5**) — `Advisory::AgedBoxForfeitedNoDob` now
  surfaces the forfeited §63(f) box (naming the $1,550/$1,950 per-box amount) whenever a DOB is absent.
  Original entry: — a `None` DOB is treated as not-aged
  (`is_aged`), which forfeits the §63(f) aged box ($1,550/$1,950) — correct + conservative (never grant an
  unsubstantiated box; honors `p1-r1-m3-dob-option-pin`; the P6 header age checkbox from the same `None`
  stays unchecked, so the filed return is internally consistent). P5's advisories work should SURFACE it:
  "DOB not on file — if 65+, you are forfeiting $1,550/$1,950 per box" so the conservative default is visible
  rather than silent. Non-blocking.

## From Fable IMPL-P2 code review r4 (final — Phase 2 GREEN-certified at `0c73bc9`; 1 record-only Minor)

- **p2-r4-m1-open-profile-form-error-arm-untested** (RECORD-ONLY, fold opportunistically with future tui-edit
  work) — `open_profile_form`'s `Some(Err(e))` arm (surfaces a corrupt-`tax_profile`-blob read error to
  `app.status`, review M-r3-2) has no dedicated KAT; both KAT-F1s exercise only the `Some(Ok)` arm. The
  regression floor is the already-reviewed-Minor r3 "masked-as-empty" state, and the save path is
  independently D-4-guarded + atomic — so this is not a new hazard. r4 explicitly ruled it does NOT warrant a
  gate round; certification did not wait on it. Add a corrupt-blob → status-set KAT when tui-edit is next touched.

## From Fable IMPL-P2 code review r2 (N1/N2/N3 FOLDED into P2 r3; deferred item here)

- **p2-r2-n4-pseudo-year-viewer-gap** (SCHEDULED → P4, with provenance rendering; PRE-EXISTING, non-fail-open)
  — in pseudo-reconcile mode the CLI `report` computes a $0 placeholder for ANY year, but the viewer's
  `resolve_all_screened` enumerates only stored∪ReturnInputs years, so the Tax tab shows `TaxProfileMissing`
  for a pseudo-only year the CLI computes. Pre-dates P2 (the pre-fold snapshot had the same gap) and no two
  NUMBERS diverge (the what-if panel's own single/$0 placeholder matches the pseudo placeholder) — it is a
  number-vs-refusal divergence between two consumers of the one resolver. Fold with the P4 provenance-render
  work (which owns making every consumer's output audit-consistent), e.g. resolve the selected year on demand
  in the viewer or extend the enumerated set under pseudo mode.

## From Fable IMPL-P2 code review r1 (C1/C2/I1 + M2/M3/M4 FOLDED into P2 r2; deferred items here)

- **p2-pref-over-ti-clamp** (RE-SCHEDULED P3 → P4 at the P3 review, review I2) — `derive_tax_profile`'s
  `.max(0)` strip (return_1040.rs) floors the ordinary base to 0 when `TI < qd + cap_gain_distr` (low
  ordinary income + large preferential income) while the FULL pref slice still reaches the frozen engine
  (which stacks `qd + pref_gain` with no min-against-TI cap). The reconstructed TI is then ≥ the true TI ⇒
  the delta/planning number can only OVERSTATE, never understate (conservative — audit-M2, review M1, both
  ranked Minor). Exact fix = cap the pref slice at TI (reduce the LT `other` first, mirroring the QDCGT
  worksheet's min). **Why P4, not P3 (was "SCHEDULED → P3 with the full deduction stack"):** the fix reduces
  the preferential income that FEEDS the frozen engine (the `other_net_capital_gain` + QD channel), which is
  the very channel P4's absolute assembly and crypto-delta stacking rewire — capping it in the P3 derive
  would be undone by P4. The P3 Schedule A deductions make the `TI < qd + cap_gain_distr` region *more*
  reachable (larger deductions eat the ordinary base first) but never flip the conservative sign, so deferral
  is not a fail-open. Code comment at the strip site updated to match this re-schedule. P4's dual report
  (`absolute_with − absolute_without ≠ delta` KAT) is where the min-cap must land + be pinned.
  **P4.1b progress:** the ABSOLUTE-side cap is now landed + pinned — `AbsoluteReturn.regular_tax` uses
  `qdcgt_line16`, whose built-in `min(L1, qd+ltcg)` cap (method.rs F-A) never overstates L16, verified by
  KAT `l16_preferential_over_ti_is_capped` (TI 35,400 / QD 50,000 ⇒ L16 $0, not the uncapped $446).
  **RESOLVED (P4 review r1 I3 fold):** the divergence KAT `section6_pref_over_ti_delta_overstates_and_does_not_reconcile`
  computes BOTH `absolute_with − absolute_without` (= $0, capped) AND the frozen delta (= $1,250, uncapped
  stacking crosses the 0%→15% LTCG breakpoint), asserting `delta > absolute contribution` and non-reconciliation
  — the absolute side is right; the delta overstates. Closed.

## From Fable spec review r4 (5 Minors — spec is GREEN 0C/0I with these open)

- **fr-schedc-27a** — Schedule C fill: the single `expenses` scalar should land on **Part V line 48 → line
  27a** (so the form cross-foots) rather than 27a with Part V blank. Mechanical map detail; resolve when the
  Schedule C map is extracted (SPEC §7.3, phase 6).
- **fr-se-sscap-clamp** — SPEC §5 stage 7 Sch SE L10 paraphrase omits the SS-cap `max(0, SS_base − wages)`
  clamp. Frozen `se.rs` and the existing `schedule_se.rs` filler both already clamp, so this is a spec-text
  paraphrase gap (fail-loud, not file-wrong). Tighten the wording; no code impact.
- **fr-schb-user-forced** — SPEC §7.1 Schedule B trigger lists "or user-forced" but no `force_schedule_b`
  input is named. Either add the input or drop the clause; harmless (always-file is valid).
- **fr-8962-taxonomy** — SPEC §9.2: excess-APTC / Form 8962 (Sch 2 L2) is listed under REFUSALS but is really
  "unrepresentable / would-refuse-if-captured" (no input exists), and is absent from §1.2's out-of-scope list.
  Move to list (iii) and add to §1.2.
- **fr-profile-diagram-nit** — SPEC §2 diagram labels `TaxProfile` "(2 scalars)"; it is the ~9-field struct
  (deep/02 §1.3). Pure diagram nit; the "2 scalars" is the load-bearing *objective*, not the field count.

## From Fable PLAN review r2 (4 Minors — plan is GREEN 0C/0I with these open)

- **pm-r2-m1** — plan KAT-ownership line mislabels the single-employer excess-SS refuse row as compute-
  dependent; it is input-screenable (P1). One-word fix in the ownership block.
- **pm-r2-m2** — the "KAT 9 → P0 (arithmetic + round-mode)" annotation re-blurs the P0 task-1 (mode) vs task-6
  (cross-foot) split; drop "round-mode" from the KAT-9 label (mode is task 1's discriminating cells).
- **pm-r2-m3** — P1 task-3's parenthetical "(no vault can hold ReturnInputs yet)" is false at phase end; the
  stub is fail-closed regardless — reword to "stub is fail-closed."
- **pm-r2-m4** — P0 task 0 FROZEN pin: make explicit that what-if / pseudo-reconcile / existing-crypto-test
  files are "never alter" (would break loudly) but are not content-pinned (only the 3 delta-path files are).

## From Fable IMPL-P0 code review r1 (folded into P0 r2; 2 recorded/deferred here)

- **p0-taxtable-deviation** (RECORDED — no action) — SPEC §8 / plan-task-5 said to add the standard
  deduction to the per-year `TaxTable`; the impl instead put the full-return indexed params in a NEW
  `FullReturnParams` / `BundledFullReturnTables`. Justified on true grounds: `TaxTable` is a published-crate
  API read by the crypto-delta path (which never needs these fields), and v1 bundles TY2024 only, so a
  separate fail-closed-gated table has the smallest blast radius. (The original code comment claiming a
  frozen `se.rs` struct-literal blocked it was WRONG — `se.rs` only calls the unfrozen `synthetic_table` —
  and has been corrected.) Recorded for traceability.
- **p0-cc0-crosscheck** (DEFERRED → Phase 7) — the P0 acceptance "CI cross-check vs a vendored CC0 PSL
  Tax-Calculator param slice" is not yet implemented. Deferred to P7 (where the independent oracles —
  tenforty / PolicyEngine / IRS ATS — live). Justification: P0's numeric values are already
  **primary-source-verified** (Fable re-fetched Rev. Proc. 2023-34; the 5 QDCGT fixtures are cent-exact
  against the official 2024 worksheet), so the CC0 diff is an *additional independent layer*, not a P0
  correctness blocker. Vendor the TY2024 slice + the diff test in P7.

## From Fable IMPL-P1 code review r1 (C1/I1–I5 + M1/M4/M5/M6 FOLDED into P1 r2; deferred items here)

- **p1-per-field-subcommands** (DEFERRED → follow-on) — v1 ships only the TOML bulk-import (`income import`)
  + `income show` (JSON) + `income clear`. Incremental per-field editors (`income add-w2`, `add-1099-div`,
  …) are a usability follow-on, not a v1 gate — the offline TOML round-trip is the complete input surface.
- **p1-show-as-json-not-toml** (DEFERRED → follow-on) — `income show` emits pretty JSON, not TOML, because
  serde-toml requires scalar keys before nested tables and the nested `ReturnInputs` model violates that
  ordering. A faithful TOML round-trip-out (custom serializer or field reorder) is a follow-on; import
  accepts TOML today, which is the load-bearing direction.
- **p1-consumer-sweep-P2** (SUPERSEDED → see `p2-consumer-sweep-remaining` at the top; **RESOLVED in P2**) —
  the original P1 entry. The routing half (route optimize/what-if/export/TUI through the resolver) is DONE;
  provenance PRINTING split out to `p2-provenance-printing` (→ P4). Kept as a stub so the id resolves; the
  live status lives in the P2-section entry.
- **p1-carryover-writeback-P3P4** (PARTIALLY RESOLVED in P4.9) — charitable + QBI carryover write-back +
  R3-M6 precedence shipped (see `p3-carryover-writeback-P4`). **Capital-loss carryforward write-back → see
  `p4-9-capital-loss-writeback`** below (deferred; frozen-type constraint).
- **p4-9-capital-loss-writeback** (DEFERRED — frozen-type constraint; non-fail-open) — the §1212
  capital-loss carryforward-out (net_1222 `st_carry`/`lt_carry`) is NOT yet written back to next year's
  `capital_loss_carryforward_in`. Unlike charitable/QBI, `Carryforward` is a **FROZEN type**
  (`tax/types.rs`), so it cannot carry the `CarryProvenance` flag the R3-M6 precedence needs; a separate
  provenance mechanism (e.g. a sibling field on `ReturnInputs`, not on `Carryforward`) is required. SPEC §4
  R3-M6 itself lists only charitable + QBI for the write-back, so P4.9 is spec-complete; this is the
  `p1-carryover-writeback-P3P4` residue. Non-fail-open: nothing persists a wrong capital-loss carryforward
  today (the filer carries it forward manually, the P1–P3 behavior). `AbsoluteReturn` does not yet expose
  the capital-loss carryover-out; add it + the sibling-provenance mechanism when this lands.
- **p1-se-earners-and-business-interest-rows** (RESOLVED in P2) — **business-flagged crypto Interest** now
  refuses in `screen_compute_dependent` (`RefuseReason::BusinessInterestIncome`, wired into `report_tax_year`
  + the consumer sweep). **≥2-SE-earners** is *structurally impossible* to input in v1: `ReturnInputs` has a
  single `schedule_c: Option<ScheduleCInputs>`, and the ledger's business income isn't per-earner-tagged, so
  there is no representation of a second SE earner to refuse — the row is moot, not skipped. (If a future
  multi-Schedule-C model lands, re-add the ≥2 refuse then.) Closes r1-I6.5 / R2-I3.
- **p1-task4-row-reclassification** (DEFERRED → task-4 follow-on) — reclassifying an imported inbound *ledger*
  row (e.g. income ↔ self-transfer) from inside the full-return flow is out of P1; the existing reconcile
  reclassification commands remain the path. Distinct from the refuse-row reclassification above. Revisit
  when task-4 row editing is specced.
- **p1-r3-m1-negscreen-exhaustive-destructure** (RESOLVED in P2) — `first_negative_amount` now destructures
  `ReturnInputs` + every money-bearing sub-struct with **no `..`**, so a newly-added `Usd` field is a compile
  error until classified (money → checked, non-money → `_`). The hand-maintained-list fail-open risk is gone.
- **p1-ssn-normalization-P6** (SCHEDULED → P6) — `income import` stores the SSN AS ENTERED; only *masking*
  (the security-load-bearing half) ships in P1. Canonicalization to `NNN-NN-NNNN` (or digits-only) is
  deferred to P6, where the PDF filler needs a single on-form format. Person's doc no longer claims
  "normalized" (review R2-M3).
- **p1-r1-m2-excess-aptc** (NOTE — already tracked) — the impl leaves Sch 2 L1a (excess-APTC) with no input,
  consistent with `fr-8962-taxonomy` above (unrepresentable / would-refuse-if-captured). No new action; this
  cross-links the two so the P3 Schedule-2 filler doesn't treat L1a as a live zero.
- **p1-r1-m3-dob-option-pin** (SCHEDULED → P3, with the age-dependent standard deduction) — `Person.dob` is
  `Option<Date>`; the §63(f) age-65 std-deduction path must treat `None` as "not established" (fail-loud / no
  silent age-0), never as a birthdate. **Not a P2 item after all:** P2's `derive_tax_profile` uses BASIC std
  only (no DOB), and the P2 kiddie-tax refuse keys on `can_be_claimed_as_dependent` (a bool, per SPEC §4.10),
  not DOB — so nothing in P2 reads `dob`. Pin the contract + a KAT in P3 when age-dependent std lands.

## From the whole-design Fable audit (Minors — C1/I1/I2/I3 were FOLDED into spec r5/r6; these Minors remain)

- **audit-minors** — the audit's Minors M2–M8, M10, M11 are recorded in
  `reviews/DESIGN-fable-audit-final.md` (the confirmation review noted they weren't transcribed here). Named
  examples: derived-profile `pref>TI` clamp mirror in `derive_tax_profile`; a couple of taxonomy nits. All
  ranked Minor by two independent Fable passes; fold opportunistically during the relevant phase. (spec §8
  KAT-3 mod-25 + the Sch 2 L1a/L2 structure are now FOLDED into spec r6, not open.)

## Open — owned by P7 (golden returns)

- **[✅ RESOLVED — SPEC §3.1 is VINDICATED, no amendment] spec-3.1-crossfoot-vs-round-the-total.**
  The question: btctax cross-foots printed totals (Σround) while the 1040 instructions appear to say
  "include cents when adding the amounts and round off only the total" (round(Σexact)). Fable P7 r2
  caught the original justification citing that instruction BACKWARDS; the r2 fold then over-corrected
  into "we knowingly depart, and it might understate tax". **Both were reasoning from the instruction.
  The instruction is not the authority.**

  **btctax is RIGHT. OTS is the outlier.** Evidence, in full, at
  [`ROUNDING_AUTHORITY.md`](./ROUNDING_AUTHORITY.md):

  - **26 CFR 301.6102-1(a):** "any amount required to be **reported** on such form shall be entered at
    the nearest whole dollar amount." **(c):** those provisions "apply **only** to amounts required to
    be reported … They do **not** apply to items which must be taken into account in making the
    computations." Lines 22 and 23 ARE reported amounts. The instruction's "include cents" sentence
    restates (c) and governs items appearing NOWHERE on the return — not lines summing other lines.
  - **The MeF schema types every 1040 money element as `xsd:integer`.** A cents-carrying return is not
    expressible; every e-filed 1040 rounds at every line.
  - **IRS Direct File** (their own open-source engine) wraps every reported line in `Round()` over
    already-rounded operands, and comments: *"We're intentionally summing rounded numbers because that
    is what Schedule B requires."* Its own fixture diverges from round-the-total by **$7**, and the IRS
    prints the cross-footed figure.
  - **The IRM — partial, and I got this WRONG at first.** It confirms the whole-dollar regime for lines
    **1–23** ("All lines on Form 1040 are edited in dollars only except lines 24 through 38"), which
    genuinely supports cross-footing across most of the return. But it is **adverse on line 24**, which
    IRM 3.11.3.14.2.28 treats as a **dollars-and-cents** line. My first write-up claimed the IRM
    "inverted the exposure" in btctax's favour — **false, and false in the direction that flattered us.**
  - **★ And the $1 is not an exposure in EITHER direction.** IRM 3.12.3.31.15.5: where a filer's rounding
    differs, "**follow the taxpayer's intent**". The math-error tolerance is redacted in the public IRM.
    Neither btctax nor OTS is at risk. We are right on the merits, not rescued by a penalty.

  Standing rule, now stated properly in SPEC §3.1's terms: **round at the point of REPORTING.** An
  amount printed on a line is rounded; a line summing other printed lines sums the rounded values.
  Amounts that appear nowhere on the return (the W-2 box-2 figures behind line 25a) are carried at exact
  cents and rounded once, where they first surface. btctax already does exactly this.

  ⚠️ **The case is SUFFICIENT, not DECISIVE, and the justification was wrong THREE TIMES** (instructions
  cited backwards → over-corrected into "we depart" → a false IRM inversion in our own favour). Each was
  caught by an independent reviewer. The evidence doc keeps the counter-arguments and the sourcing
  caveats (the MeF XSD is from public mirrors — the IRS gates schema distribution). If you are tempted to
  restate this argument more strongly than the doc does: don't.

- **[✅ DONE — the answer is NO] p7-ats-scenario-2 — the IRS scenario is NOT a golden return.**
  The plan's P7 task said "ingest IRS ATS Scenario 2 with a partial-line diff". It cannot be done, and
  recon 05's premise was wrong. We fetched the PDF, rendered it and LOOKED at the pages:

  - The **1040 is BLANK** — watermarked "DRAFT — DO NOT FILE", lines 1a–15 empty. Only identity,
    filing status and dependents are filled.
  - **Schedule A is only half-filled**: the IRS entered the INPUTS (5a 1,068 · 5b 10,509 · 8a 16,854 ·
    11 250 · 12 735) and left every COMPUTED line blank — 5e (the SALT cap), 7, 8e, 10, 14, 17.
  - **Zero AcroForm `/V` values** in the whole file, so recon's "parse the `/V` fields into an
    expected-lines table" has nothing to parse.
  - Recon's **form list is wrong** too: the scenario's cover page lists 1040, W-2 ×2, Sch 1, Sch A,
    Sch C, Sch EIC, 8283, 8867 — not Sch D / 8812 / 8863 / 8995 / 4972.

  **Why:** ATS validates a submitted **MeF XML**, so the PDF only ever had to state the FACTS. It is a
  test-case *specification*, not an answer key. Same lesson as the tenforty mislocalisation, one level
  up: **verify the artifact says what the note claims before building on the note.** Recon 05 is
  corrected in place.

  **What we did instead** — the plan's own stated alternative ("or a v1-envelope synthetic golden"),
  which is stronger anyway because it covers the whole matrix rather than one taxpayer: eleven
  households against **two** independent engines. And the scenario still earned its keep — its SALT
  figures ($1,068 + $10,509 = $11,577, over the $10,000 cap) seeded `mfj_itemized_salt_over_the_cap`,
  closing a real hole: **no golden exercised §164(b)(5) at all**, because the only other itemized
  household carried one lump sum. Both oracles agree with btctax on it, and the cap is asserted on the
  PAPER (Schedule A line 5e prints 10,000, not 11,577).

- **[✅ DONE] p7-se-divergence-tiebreaker — RUN, and it settled in btctax's favour.** The PSL
  Tax-Calculator (CC0, a completely separate lineage from OTS) reproduces btctax's §1402(b)(1) behaviour
  **to the dollar on every row of the sweep**, including the discriminating middle case ($100,000 of wages
  ⇒ $10,649, where the band is partly but not fully consumed):

  | W-2 wages | tenforty/OTS | PSL Tax-Calculator | btctax |
  |---:|---:|---:|---:|
  | 0 | 11,304 | 11,304 | 11,304 |
  | 100,000 | 11,304 | **10,649** | **10,649** |
  | 168,600 | 11,304 | **2,143** | **2,143** |
  | 220,000 | 11,304 | **2,143** | **2,143** |
  | 300,000 | 11,304 | **2,143** | **2,143** |

  btctax is CONFIRMED correct; `tenforty`/OTS is the outlier. Both engines are now baked into the golden
  cross-check, and the rule is: if the oracles AGREE, btctax must match them; if they DISAGREE, btctax must
  match one and a `Divergence` entry must name which and why. **Still open (upstream curiosity, not ours):
  whether the fault is in OTS itself or in the `tenforty` wrapper — driving OTS directly would answer it,
  and if the wrapper is at fault it is worth reporting.** Filed as `p7-ots-vs-wrapper-localisation → post-v1`.

- **[✅ DONE] p7-ots-vs-wrapper-localisation — ANSWERED: the fault is the WRAPPER. OTS is exonerated.**
  Driving `taxsolve_US_1040_Sched_SE_2024` directly (observe-only; no GPL source read into btctax) reproduces
  btctax's figures **to the cent** across the whole sweep — L9=0, L10=0, L12=2142.52 at $220k of wages. OTS's
  Schedule SE reads and honours `L8a`; its 1040 reads `L13` (the §199A deduction). **Both fields exist; the
  `tenforty` wrapper fills neither.** So the earlier shorthand "the oracle is wrong about SE tax" (commit
  `9ebf9a5`) was mislocalised, and `golden_returns.rs` has been corrected accordingly.

  One important nuance, found by reading tenforty's own tests rather than assuming: their omission of line 8a
  is **deliberate for Married/Joint** and correct to be careful about. `w2_income` is a *household aggregate*
  while Schedule SE is a *per-person* form, so for MFJ you cannot tell whose wages are whose, and attributing
  the household total to the self-employed spouse would wrongly wipe out their wage base. Our diverging golden
  household (`mfj_se_over_the_addl_medicare_threshold`) is exactly that MFJ case — so for **that** row the
  disagreement is a **modelling mismatch, not a tenforty bug**: btctax's `se_w2_ss_wages` means *the filer's
  own* box-3 wages, and the fixture says all $220k are the SE person's. taxcalc agrees with btctax once given
  the same per-person split (`e00200p`/`e00900p`). btctax remains correct **by construction** — it reads the
  actual W-2 — but we should not have called tenforty simply "wrong" here.

  Where tenforty **is** unambiguously wrong is the **Single** filer: one person, no aggregate ambiguity, and
  line 8a still unfilled ⇒ a Single filer with $168,600 of wages and $60,000 of Schedule C profit is
  overcharged **$6,870.84**. Reported upstream with a fix.

- **[✅ DONE] p7-tenforty-upstream-report — FILED.** [Issue #278](https://github.com/mmacpherson/tenforty/issues/278)
  (both defects: Schedule SE line 8a, and §199A/1040 line 13 never supplied — the two backends disagree by
  $16,000 of taxable income on identical inputs) and [PR #279](https://github.com/mmacpherson/tenforty/pull/279)
  (fixes the Schedule SE half; 663 passed / 0 failed on their suite, their MFJ regression test untouched and
  still passing). Both disclose that the author is Claude. The QBI half was deliberately **not** patched: its
  taxable-income limitation is circular with taxable income itself, so the fix needs an architecture decision
  that is the maintainer's to make.

- **p7-tenforty-upstream-followup → post-v1.** ★ **Check back on the responses to
  [issue #278](https://github.com/mmacpherson/tenforty/issues/278) and
  [PR #279](https://github.com/mmacpherson/tenforty/pull/279).** Watch for: maintainer review comments, CI
  results (their workflows need maintainer approval for a first-time contributor, so nothing had run at filing
  time), a request to split the issue in two, or a steer on how they want the §199A fix shaped — we offered to
  send that patch and the per-spouse wage split (`e00200p`/`e00200s`-style) if wanted. Nothing here gates
  btctax: we no longer depend on tenforty (see `p7-oracle-swap`).

- **[SUPERSEDED — the original entry, for the record] p7-se-divergence-tiebreaker → P7.** ★ **Break the tie
  on the SE-tax divergence with a SECOND independent engine.**

  P7.1's cross-check found that the oracle (`tenforty`, wrapping Open Tax Solver) computes a
  self-employment tax that is **invariant to W-2 wages** — flat at $11,304 on $80,000 of SE income
  whether the W-2 wages are $0 or $300,000 — where Schedule SE lines 8a/9/10 and §1402(b)(1) say the
  12.4% OASDI portion must fall to ZERO once W-2 social-security wages have consumed the $168,600 band.
  btctax says $2,143 (Medicare only) for that household; the oracle says $11,304, over-taxing it by
  $8,062 all-in. The divergence is DECLARED in `crates/btctax-core/tests/golden_returns.rs` and asserted
  in btctax's favour, on the strength of the form's own printed text.

  **Two things that evidence does NOT settle:**
  1. **Where the fault lives.** `tenforty` is a Python wrapper around OTS. The flat SE tax could be OTS
     itself, or the wrapper failing to pass W-2 wages into OTS's Schedule SE inputs. We deliberately did
     not investigate — the clean-room posture forbids reading their source (OTS is GPL-2.0, incompatible
     with our `MIT OR Unlicense`; recon 05). **Running** it is fine; **reading** it is not.
  2. **Whether btctax is right**, on the strength of a single dissenting implementation. The form text is
     strong evidence and the arithmetic is unambiguous — but "we disagree with the only oracle we have,
     and we are confident we are right" is precisely the posture that should be independently checked
     before anyone files a return on it.

  **The work:** run a SECOND, unrelated implementation over the same household — candidates: **OTS
  directly** (drive its own input file, bypassing the `tenforty` wrapper, which alone answers question 1),
  **PolicyEngine-US**, and/or the **PSL Tax-Calculator** (CC0 — the SPEC §9 CI cross-check candidate).
  Observe-only in every case: run them, compare FIGURES, never read source into btctax.

  **Acceptance:** the SE figure for `mfj_se_over_the_addl_medicare_threshold` ($220,000 W-2 wages +
  $80,000 SE income, MFJ) is confirmed as **$2,143** by at least one further independent engine — or, if a
  second engine agrees with `tenforty`, btctax's own SE model is re-opened and re-derived from §1402(b)
  and the Schedule SE instructions before P7 can close. Record which of the two questions each engine
  answers. If OTS-direct disagrees with `tenforty`, report the wrapper bug upstream.


## Open — filed from the P6 gate review (Fable r1 + r2). NONE is blocking; each has an owning phase.

Recorded per the standing burndown rule (an item whose owning phase is closing must not silently carry).
Full text: `reviews/IMPL-P6-fable-review-r1.md` and `reviews/IMPL-P6-fable-review-r2.md`.

- **p6-r1-m1-ippin-error-noun → P6.7 (cleanup).** `IpPin::canonical` reuses `SsnError`, so a 5-digit PIN
  refuses with "has 5 digits — an SSN has exactly 9". Wrong noun, wrong count. Give the PIN its own error.
- **p6-r1-m2-mfs-without-spouse → P6.7.** An MFS return with no `spouse` captured files with the spouse-SSN
  and spouse-name cells BLANK; the 1040 requires them on MFS. Refuse at screen time, or LIMITATIONS-note it.
- **p6-r1-m3-schedule-c-face-lines → P6.7.** Sch C lines E (address), G (material participation), H, I/J
  (1099 questions) are blank and undocumented. G especially: a mining/staking trade-or-business with SE tax
  is materially participated by construction — answer it on the same authority as the Schedule D QOF "No".
- **p6-r1-m4-forms-filter-ignored → P6.7.** The full path silently ignores `--forms` (the dispatch precedes
  the filter). Reject the flag on a full-return year, or document that the packet is indivisible.
- **p6-r1-m5-must-file-skips-8949 → P6.7.** `sch_d.must_file()` gates the 8949 branch too, so a real disposal
  whose printed cells all round to $0 files neither form while the DA question answers Yes. A disposal is
  reportable regardless of amount: file Schedule D whenever `f8949.is_some()`. (Intersects r2 NEW-I1's
  operand question — fix together.)
- **p6-r1-m6-instruction-level-citations → post-v1.** Form 8960 L1/L2/L5a/L13 and Form 8995 L11/L12 are cited
  from 1040 lines by the INSTRUCTIONS (not on-face text), and print rounded exact values that can sit $1 from
  the printed 1040 cells the Service cross-matches. Spec-conformant under the on-face criterion; consider
  extending §3.1's closed list to instruction-level citations.
- **p6-r1-m8-refusal-names-who / line-2a-blank-when-zero → P6.7.** `SsnError::Missing` surfaces at the packet
  boundary without saying WHICH of four people; and `push_money` writes "0" where the ARCH note asked for a
  BLANK cell when the value is zero.
- **p6-r2-nm1-qof-silent-skip → P6.7.** The full Schedule D's QOF write is `if let Some(qof_no)` (silent blank
  on a map without the cell) where every other mandatory full-path cell uses fail-closed `need()`.
  Unreachable for TY2024; align the convention.
- **p6-r2-nm2-prefix-is-not-lexicographic-order → P6.7.** "the prefix IS the stapling order" is false under a
  lexicographic directory sort (`12A_` sorts before `12_`; `155_` between `12A_` and `17_`). The MANIFEST is
  correct and labelled; zero-pad the prefixes or fix the comment.
- **p6-r2-nm3-no-packet-tie-out-for-sch-a-l2 → P6.7.** No packet-level KAT pins `sch_a.line2 == f1040.line11`;
  the unit KAT cannot see an assembly-wiring revert.
- **p6-r2-nm5-user-only-8283-refusal-unpinned → P6.7.** The user-only >$500 refusal lost its test pin when the
  guard moved screens. The behavior is correct; the coverage regressed.
- **p6-r2-nm6-cli-empty-header-and-untested-output → P6.7.** The full path prints an empty "Filled IRS forms →"
  header before the packet block, and the whole output block has no test coverage.
- **[✅ DONE] p6-r1-n1-extract-lines — BUILT, and it unlocked the packet round-trip.**
  `btctax_forms::testonly::extract_lines(pdf, map_toml) -> BTreeMap<line, printed text>` walks the committed
  map generically (so it transcribes any form in the packet without knowing which it holds), descends nested
  groups (`identity.ssn`) and repeating tables (`part1_rows[0].payer`), and returns **only cells the fill
  actually wrote** — an off checkbox and an unused row slot are ABSENT, because a blank line on a tax form is
  a statement and "written empty" must not read the same as "never written". All 5 KATs fault-injected.

  It is a read-back, **not** an oracle: it goes through the same map the fill used, so it cannot catch a
  mis-mapped cell — that stays `verify.rs`'s job (geometry, map-independent). The two are complementary:
  **geometry says the value landed in the right box; this says the right VALUE is in it.**
- **p6-r1-n2-manifest-w2-copy-b → P6.7.** The manifest should carry the "attach your W-2 Copy B" line.
- **p6-r1-n3-every-schedule-kat-tests-one-form → P6.7.** `every_schedule_carries_the_name_and_ssn_header`
  tests ONE form; the name promises all. Had it iterated the packet, the unnamed 8949 (r1 I3) would have been
  caught mechanically. Also the 1040 L23 ↔ Sch 2 L21 CELL-TEXT leg of the cross-PDF oracle is still missing.
- **p6-r1-n4-zero-idiom → P6.7.** 1040 line 7 prints "0" where the form's idiom is "-0-".
- **p6-r1-n5-foreign-address-row → P6.7.** The 1040's foreign-address row (f1_15–f1_17) is unmapped and
  undocumented.
- **[✅ DONE — not deferred] p6-r3-nm7 + the r3 nit.** The QDCGT KAT now runs END TO END through
  `form_1040_lines` on a bin-straddling fixture (printed Sch D 5,003 vs exact 5,000, printed L15 = 60,000 ⇒
  the worksheet's ordinary remainder is 54,997 vs 55,000 — different $50 bins), asserting the filed L16 IS
  the worksheet on PRINTED operands and is NOT the worksheet on the exact ones. Fault-injected the
  pass-through to watch it fail before claiming it. The disjointness KAT's comment no longer overstates its
  fixture's collision census (Schedule D + 8949; no Schedule SE on that ledger).

- **p6-r1-obs-partial-packet-on-io-error → P6.7 (observation).** All-or-nothing holds against filler refusals
  (every form fills before any byte is written), but an I/O error mid-write-loop can still leave a partial
  packet on disk. LIMITATIONS' "you never find a half-packet" is scoped to fill failures. Write to a temp dir
  and rename, or scope the claim.


## Open — owned by P6 (the PDF fillers)

- **[✅ CLOSED in P6.5] p5-c1-crypto-slice-export-refusal.** `CliError::CryptoSliceExportForFullReturnYear`
  is DELETED and replaced by the dispatch (`export_irs_pdf` → full packet when the year has `ReturnInputs`,
  else the crypto slice), pinned in BOTH directions plus a byte-level no-overwrite KAT. Original entry:

- **p5-c1-crypto-slice-export-refusal → P6.** `export-irs-pdf` now REFUSES for a year with full-return inputs
  (`CliError::CryptoSliceExportForFullReturnYear`), because its Schedule D fills only the crypto totals (no
  line 13 for 1099-DIV box-2a capital-gain distributions, no lines 6/14 for capital-loss carryovers) and its
  1040 fills only the capital-gain cluster — for a full return those are complete-LOOKING forms with income
  missing (§3.4: fail closed). **P6 must REPLACE this refusal with the real full-return export**, and delete
  the guard in `cmd/admin.rs::export_irs_pdf` + the KAT `export_refuses_for_a_full_return_year_p5_c1` when it
  does. Owning phase: **P6** — this is not deferrable past it, since P6's whole purpose is to make the export
  correct.

- **[CLOSED by SPEC §9 amendment, 2026-07-13] p5-i1-full-return-draft-attest-gate.** The user decided the full-return packet exports CLEAN with no attestation; the DRAFT/attest gate stays pseudo-only. There is no always-on gate to build. Original entry:

- **p5-i1-full-return-draft-attest-gate → P6.** The DRAFT watermark + attestation gate is pseudo-mode-only
  today; LIMITATIONS.md now says so in present tense. SPEC §9 wants an **always-on DRAFT gate for full-return
  PDFs**. It lands with the P6 fillers (there is no full-return PDF to stamp before then). Owning phase: **P6**.

- **[✅ LARGELY CLOSED in P6.3b; residual → P6.7] p5-m1-report-lacks-interior-schedule-lines.** Every
  1040-level figure the report prints is now the PRINTED figure, and the packet itself is filled — so the
  original need ("transcribe by hand from the report") is gone: the filer gets the FORMS. What is NOT printed
  is the interior per-line detail of each schedule (a form-by-form transcription aid), which now has no
  consumer. **Residual scope → P6.7: decide whether to print the interior lines at all, or close this as
  obsoleted by the packet.** Original entry:

- **p5-m1-report-lacks-interior-schedule-lines → P6.** The report prints the 1040-level summary, not the
  interior per-line figures of Schedules 1/2/3/A/B/C and Forms 8959/8960/8995, so the "transcribe by hand"
  instruction is not fully actionable from the report alone. Once P6's `*_lines` printed chains exist (see
  `p6-printed-line-chain` below) the report can print them. Owning phase: **P6**.

- **[✅ CLOSED in P6] p6-printed-line-chain.** Every form in the packet now has its core printed chain, and
  the chains COMPOSE on the printed lines (SPEC §3.1's citation-composition rule, now normative with its
  closed list). Original entry:

- **p6-printed-line-chain (architectural, surfaced building Form 8959).** SPEC §3.1's round-all-amounts
  election means every printed form needs a **printed line chain**: each line `round_dollar`ed AT THE LINE,
  and each printed total summing the ALREADY-ROUNDED lines so the filed form cross-foots. This is NOT
  `round_dollar(exact_total)` — KAT-9 (`kat9_printed_lines_round_then_cross_foot`) pins the discriminating
  case. `btctax-forms` must do ZERO tax arithmetic, so each chain is derived in **core** and the filler
  transcribes it (`other_taxes::form_8959_lines` is the pattern). Every remaining P6 form (Sch 1/2/3/A/B/C,
  8960, 8995, the full 1040, Sch D L17–22) needs its own. Owning phase: **P6**.

- **[✅ CLOSED in P6.3b] p5-report-vs-pdf-may-differ-by-rounding.** Resolved BY CONSTRUCTION, per the
  architect's Q3 ruling: the report renders the PRINTED figures, so there is no user-visible divergence left
  to disclose — the terminal and the filed form carry the same characters (KAT reads the figure back out of
  the PDF). LIMITATIONS explains the whole-dollar election. Original entry:

- **p5-report-vs-pdf-may-differ-by-rounding → P6.** A direct consequence of the above: the exact-cents report
  and the whole-dollar cross-footing PDF can differ by a few dollars. That is what the round-all-amounts
  election means, and the PDF is the filed document. P6 must decide how the report surfaces this (print the
  printed-line figures alongside the exact ones, or state the difference) and LIMITATIONS.md must say it.
  Owning phase: **P6**.

- **[✅ CLOSED in P6.3] p5-n5-advisory-line-wrapping.** `wrap_bulleted` wraps advisories at 92 columns with a
  hanging indent (KAT `advisories_wrap_to_the_house_width_with_a_hanging_indent`). Original entry:

- **p5-n5-advisory-line-wrapping → P6 (render).** `render_advisories` (`crates/btctax-cli/src/render.rs`)
  emits each advisory as ONE unwrapped 300–400-char line; the house style wraps everywhere else. Nit — folds
  naturally into P6's render work, which already has to grow (see `p5-m1`). Surfaced by Fable IMPL-P5 r2
  (N-r2-1) after r1-N5's money-format half was fixed.

- **[CLOSED by SPEC §7.4 amendment] p6-schedule-b-overflow-refuses-instead-of-paginating.** The fail-closed refusal is now the SPEC'd behavior: the 8949 continuation pattern does not transfer, because Schedule B IS the aggregator (line 4 → 1040 2b), so two copies leave "which line 4 is 2b?" undefined. The real fix is filed post-v1. Original entry:

- **p6-schedule-b-overflow-refuses-instead-of-paginating → P6.** SPEC §7.4 says "Sch B >14 interest /
  >15 dividend overflow reuses the **8949 continuation pattern**" (i.e. paginate onto additional
  copies via `overflow.rs` / `merge_copies`). I implemented a **fail-closed REFUSAL** instead
  (`fill_schedule_b` errors when the payer count exceeds the row count). Refusing is safe — it can
  never produce a wrong form — but it is NOT what the SPEC specifies, and it turns a filable return
  into a hard stop for a household with 15+ interest payers. Declaring the deviation rather than
  hiding it: either implement the continuation pattern, or amend SPEC §7.4. Owning phase: **P6**.

- **[✅ DONE in P6.1] p6-form8959-must-file-belongs-in-core.** `Form8959Lines::must_file()` now lives in
  core (`other_taxes.rs`); `form8959.rs` only obeys it. Every file-or-don't-file decision in the packet is
  now a core fact the packet KATs can see.

- **[✅ DONE in P6.1] p1-ssn-normalization-P6** — `packet::Ssn::canonical` (strip formatting → require
  exactly nine digits), `digits()` / `hyphenated()` renderers, and a **masked `Debug`** (an SSN in a log
  or panic message is a PII incident). Wired into `screen_inputs` as `RefuseReason::SsnMalformed(who)` —
  which names WHO, never the digits.

  ⚠️ **DECLARED DEVIATION from the architect's guidance** (ARCH-P6 Q1), for the P6.6 reviewer. Fable said
  a non-canonicalizable SSN should refuse at compute time. I split the case in two: a **malformed** SSN
  (captured but not nine digits) refuses at compute time as instructed, but an **uncaptured** (empty) SSN
  does NOT — it refuses at the packet boundary instead (`ReturnHeader::build` → `SsnError::Missing`), so
  no PDF can ever be attempted without an identity. Reason: the tax math never reads an SSN, so refusing
  the computation would block the very report a filer uses to decide whether to file at all, and would buy
  no correctness — there is no number on the return an absent SSN could make wrong. Fail-closed is
  preserved where it matters (the *filable artifact*), and the codebase's own fixtures are the evidence
  the compute path never needed PII. KAT: `an_uncaptured_ssn_does_not_block_the_report`.

- **[✅ CLOSED in P6.2] p6-aged-blind-checkboxes-missing (was GATING).** The four §63(f) boxes are now
  mapped (`c1_9`/`c1_10`/`c1_11`/`c1_12` — dumped and correlated against the printed Age/Blindness row,
  never extrapolated) and printed, and core derives the count ONCE (`AgedBlindBoxes`) with L12 consuming
  that same count. KATs: `the_1040_prints_the_aged_blind_boxes_its_line_12_depends_on` (fill side) +
  `aged_blind_box_count_matches_the_standard_deduction_core_actually_computed` (core side).

- **[✅ CLOSED in P6.2] p6-form-identity-header.** All nine schedules + the full Schedule D + the 1040
  now print their identity. See the two NEW findings below, both surfaced by doing it.

- **[NEW → found in P6.2, FIXED in P6.2] p6-two-more-checkbox-consistency-gaps.** Dumping the 1040 header
  turned up **two more checkboxes of the same class as aged/blind** — flags core's L12 already consumes
  but the form would never have shown:
  - `c1_6` "Someone can claim: **You** as a dependent" ↔ the §63(c)(5) dependent FLOOR that replaces the
    basic standard deduction.
  - `c1_8` "**Spouse itemizes** on a separate return" ↔ the §63(c)(6) MFS coupling.
  A return that claims either arithmetic without ticking its box contradicts itself exactly as a
  nonstandard standard deduction with zero aged/blind boxes does. `ReturnHeader` now carries all of them
  (plus the §6096 presidential-fund election, which is captured input that would otherwise silently fail
  to print), and the filler prints them. KAT:
  `the_header_carries_the_dependent_claim_and_mfs_itemize_flags_that_l12_depends_on`.

  **Lesson for the P6.6 reviewer:** the aged/blind defect was not a one-off. It is what happens when a
  captured input reaches the ARITHMETIC without reaching the FORM. A sweep for any *other* input that
  core consumes but no cell prints is worth doing — the three found so far were all in the same header.

- **[NEW → P6.2] p6-dependents-over-four-refuses.** The 1040's dependents table has exactly FOUR rows.
  More than four now **fails closed** (`FormsError::Overflow`) rather than printing the first four:
  the IRS's own remedy is to tick `c1_13` and attach a continuation statement, which is the same
  synthetic-page-generator machinery Schedule B's >14-payer case needs and v1 does not have (SPEC §7.4 as
  amended). Printing four of five would silently file a return that misstates the household. The real fix
  rides with `p6-schedule-b-continuation-statement` (post-v1). KAT:
  `more_dependents_than_the_form_holds_fails_closed`.

- **[NEW → P6.2] p6-maxlen-comb-guard (infrastructure).** `pdf::Field` now reads `/MaxLen`, and
  `verify_flat` gained a fifth read-back leg: any value longer than its cell's declared capacity is
  `FormsError::CellOverflow`. This applies to EVERY text write in the crate. It caught, before it could
  ship, the assumption recorded in CONTINUITY_P6 that the 1040's SSN cells are `/MaxLen 11` — they are
  **`/MaxLen 9`** (comb), so they take bare digits, while the schedules' are `/MaxLen 11` and take the
  hyphenated form. The forms genuinely disagree; `render_ssn` reads each cell rather than assuming.

- **[GATING — core half DONE in P6.1; the FILL remains → P6.2] p6-aged-blind-checkboxes-missing.** Core
  now derives the four boxes ONCE (`packet::AgedBlindBoxes::for_return`) and **`standard_deduction` L12
  consumes that same count**, so the checkbox count and the claimed deduction cannot drift apart by
  construction. Pinned by `aged_blind_box_count_matches_the_standard_deduction_core_actually_computed`.
  What remains is P6.2: `f1040.map.toml` still has no checkbox FQNs, so nothing is printed yet. Original
  entry:

- **[GATING] p6-aged-blind-checkboxes-missing → P6.** Core folds the §63(f) age-65/blind additions into
  the printed 1040 **line 12**, but `f1040.map.toml` has **no age/blind checkboxes**. A filed 1040 claiming
  a nonstandard standard deduction with ZERO boxes checked fails the IRS's own arithmetic cross-check — the
  checkbox count is how the Service validates it. Same class as P5-C1: a form internally inconsistent with
  itself. Found by the Fable architect pass. **Gating for the packet, same tier as name/SSN.** The exit
  condition is restated: the packet must be filable AND every figure on it internally and mutually
  consistent — not merely "every money line is right".

- **p6-schedule-b-capacity-error-variant → P6 (nit).** The Schedule B overflow refusal is raised as
  `FormsError::Geometry`, which mislabels it — it is a CAPACITY refusal, not a placement failure. Give it
  its own variant so the CLI can render "file Schedule B by hand" actionably.

- **p6-form8959-must-file-belongs-in-core → P6 (minor).** Every other file-or-don't-file decision is a core
  `Option` return; Form 8959's ("L18 and L24 both zero") lives in the FILLER. Hoist it to
  `Form8959Lines::must_file()` so every filing decision is a core fact the packet KATs can see.

- **p6-writeback-persists-cents-not-filed-figures → post-P6 (minor).** `apply_carryover_writeback` persists
  exact cents; strictly, next year's worksheets start from the *filed* whole-dollar figures. Sub-dollar and
  re-rounded next year, so not a gate item — recorded as a decision, not an accident.

- **p6-schedule-b-continuation-statement → post-v1.** The real >14-payer fix: one Schedule B whose line 1
  reads "see attached statement", plus a generated continuation statement (a synthetic page generator,
  outside the geometric oracle). SPEC §7.4 as amended.

- **[core half DONE in P6.1; the FILL remains → P6.2] p6-form-identity-header.** `packet::ReturnHeader`
  now derives the identity ONCE: the MFJ **joint** name line, the Schedule C **proprietor** (the business
  OWNER — a spouse-owned Sch C files under the SPOUSE's name and SSN even on a joint return, which a naive
  shared writer would get wrong), the address, the aged/blind boxes, and the dependents rows. Fillers can
  only transcribe it. What remains is P6.2: the `[identity]` map fragments + the shared `push_identity`
  writer + the 1040's full header block. Original entry:

- **p6-form-identity-header → P6 (packet assembly).** None of the new P6 fillers (8959, 8960, 8995,
  Sch 1/2/3/A/C) writes the **taxpayer name + SSN header** that every IRS form carries at the top. The
  money lines are right, but the forms are not filable as-is: an unnamed Schedule C is not a return.
  This is deliberately cross-cutting rather than per-form — every form has the same two fields, they
  all come from the same `ReturnInputs.header`, and they belong with the packet-assembly step that
  wires `export_irs_pdf` (item 5 in CONTINUITY_P6's remaining-work list). **Not deferrable past P6** —
  the phase's exit condition is a filable packet.

- **p1-ssn-normalization-P6** — (carried; unchanged).

## Spec errata surfaced by the plan review (fix spec text; do not re-open the GREEN gate for these)

- **spec-s8-kat3-mod25** — SPEC §8 / §10 KAT-3 says "no bracket edge < $100k inside a $50 bin". That's a
  **TY2024-only** fact (deep/01:59). TY2017 (9,325) and TY2025 (11,925 / 48,475) have edges at bin **midpoints**
  (≡ 25 mod 50), which are harmless (IRS taxes at the midpoint; TCW continuous). Correct the spec wording to
  "every edge < $100k ≡ 0 (mod $25)". The **plan already implements the corrected assertion** (P0 task 4).
- **spec-s48-l36** — SPEC §5 stage 9 carries "− L36 applied-to-2025" but §4.8 `Payments` has no L36 input. v1
  pins L36 = 0/blank (plan P4 task 6); add the input in a follow-on or note L36-always-0 in §4.8.
- **spec-recon-dependent-spouse-checkbox** (surfaced by IMPL-P3 review I1) — deep/04 §1.2 lists the
  claimable-as-dependent-SPOUSE checkbox as a consumed std-deduction input, but §1.3's pseudocode and SPEC
  §4.7 both drop it, leaving the captured `can_be_claimed_as_dependent_spouse` flag with no consumer. v1
  REFUSES the flag (`p3-i1-dependent-spouse-refuse`); fix the spec/recon text (re-add the spouse box to §1.3's
  §63(c)(5) trigger + §4.7) if/when the dependent-spouse std-deduction limit is actually modeled.

## From earlier reviews (folded, recorded for traceability)

- (r1–r3 findings were all folded into the spec; see `reviews/SPEC-fable-review-r{1,2,3}.md`.)
