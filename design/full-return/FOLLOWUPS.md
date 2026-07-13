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
  construction). **STILL OPEN (→ P4 report wiring):** persistence of `carryover_out` as year Y+1's
  `charitable_carryover_in` + the **R3-M6 precedence** (computed overwrites computed; refuses user-entered
  w/o `--force`) — needs the CLI side-table, lands with the dual report.
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
- **p4-r1-m3-ctc-advisory-P5** (DEFERRED → **P5**, owner recorded per burndown) — plan P4 task 7's CTC/ODC
  "loud advisory" half. The **compute** is done (L19 = 0, `ctc_odc_conservatively_omitted_l19_zero`); the
  *advisory surfacing* ("you have N dependents; CTC/ODC omitted — Schedule 8812 filed separately") is a render
  concern owned by **P5** ("wire the conservative-omission advisories into report/output", SPEC §9.2). Direction
  is conservative (overstates tax only). P5's entry-sweep must pick this up; nothing understates in the interim.
- **p4-r1-n1-taxyearreport-struct** (NIT → **P5**) — `cmd::tax::TaxYearReport` is now a 7-tuple of
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
- **p3-m4-none-dob-forfeited-63f-advisory** (→ P5 advisories) — a `None` DOB is treated as not-aged
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
- **p1-carryover-writeback-P3P4** (SCHEDULED → P3/P4) — charitable + capital-loss carryovers are read from
  `ReturnInputs` but not yet written back (next-year carryforward_out). The write-back lands with the
  Schedule A / Schedule D compute stages in P3/P4; P1 only stores the declared inputs.
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
