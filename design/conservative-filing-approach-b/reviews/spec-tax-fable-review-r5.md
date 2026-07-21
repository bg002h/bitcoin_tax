# Independent TAX review — Approach B sub-project 1 SPEC, round 5 (post r4-fold)

**Artifact:** `design/conservative-filing-approach-b/SPEC.md` (DRAFT, r1–r4 two-lens folds applied, `52c76c8`).
**Lens:** US federal tax correctness / completeness / honesty. Convergence round: both r4 findings re-derived
against statute/reg AND current source (`tax/{compute,return_1040,charitable,qbi}.rs`, `cmd/tax.rs`,
`project/fold.rs`, `state.rs`); one final adversarial pass over the cascade naming for under-report /
over-claim and over the carryover-chain census for a missed third chain.
**Provenance honored:** `DESIGN_PROVENANCE.md` + all eight prior reviews read; no adjudicated ruling or
verified-resolved finding re-litigated; r1–r3 fix clauses confirmed still present in the folded text
(nothing reopened by the r4 fold). Parent-spec guarantees untouched — the fold is copy/census/KAT only.
**Reviewer:** Fable (independent; not the author of the fold). **Date:** 2026-07-21.

**Verdict: GREEN — 0 Critical / 0 Important / 0 Minor / 1 Nit. The tax lens CONVERGES; the gate can close.**

The r4 fold is exactly what r4 prescribed — naming, not computing — and every claim in it is true at source.
The cascade clause is neither an under-report (both chains, both directions, trigger condition exactly the
set of flagged years that can move a carryover) nor an over-claim (hedged "may also require amendment";
quotes a Δ only where the machinery genuinely produces one; else named-unquantified, recorded in the
`Acknowledgment`, never silently absent). The gift/donation quoting split is correct and complete. My
fifth-pass census confirms the two named chains are the ONLY crypto-sensitive cross-year carryover chains
the product models. One wording Nit; nothing gates.

---

## V — Verified resolved (r4 → fold, each checked against source/law)

- **I-1 (cross-year carryover cascade unflagged/unquoted/unnamed) → RESOLVED.** The fold landed all four
  prescribed pieces, each verified:
  - **(a) BG-D9 cascade clause — honest and complete.** The new ★ sub-bullet names BOTH chains with the
    correct mechanisms: §1212(b)/§1211(b) — engine B's `carryforward_out` is a WITH-crypto *level* that
    "feeds next year's `capital_loss_carryforward_in`" (`compute.rs:211-212`, built at `:406-410`;
    `carryforward_consistency` at `:436`, wired advisory-only behind both-profiles + prior-year-Computed,
    `cmd/tax.rs:442-466`); and §170(d) — `write_back_carryover` (`cmd/tax.rs:485-590`) →
    `apply_carryover_writeback` (`return_1040.rs:1454-1491`) stamps Y's computed `charitable_carryover_out`
    into Y+1's stored `ReturnInputs`, and the function's own doc confirms the spec's exact characterization:
    it errs only on **User**-provenance; *"A computed (or empty) existing carryover-in is overwritten
    silently"* (`return_1040.rs:1450-1453`). Engine-B blindness re-verified: `compute_tax_year` applies the
    same `profile.capital_loss_carryforward_in` in both folds (`compute.rs:317`). The advisory copy
    (*"…derive from year Y and **may** also require amendment, even though those years' crypto transactions
    are unchanged"*) is hedged and asserts no computation — **no over-claim**. The quoting conditions are
    factually available where claimed: the `carryforward_out` diff sits on the `TaxResult` the consent Σ
    already computes when both folds compute Y; the `charitable_carryover_out` diff comes off
    `assemble_absolute` over the fold-pair states when the absolute return computes (TY2024-only in v1 —
    hence the honest "else named-unquantified" for the 2018–2023 audience years). Both directions present
    (promote = amend-to-pay cascade; VOID = amend-to-refund, §6511-bounded — sign-correct: in the r4 worked
    corner Y's §1211(b) allowed $3k is identical in both folds, so the entire Δ lands on Y+1, and the void
    restores the stranded carryforward → a §6511-time-barred refund claim).
  - **(b) BG-D6 exposure side** — the cascade note is carried per flagged year and recorded in the
    `Acknowledgment` as a **named-unquantified term when the machinery cannot price it (never silently
    absent)**; it stays a heterogeneous named term, never blended into the headline Σ — consistent with the
    r2-settled flagged-terms structure, so the §6664(c) artifact stays honest.
  - **(c) §3 census item 8c landed** — both chains at the correct symbols (`carryforward_consistency`,
    `tax/compute.rs` / `cmd/tax.rs`; `write_back_carryover`/`apply_carryover_writeback`, `cmd/tax.rs` /
    `tax/return_1040.rs`), with the silent-Computed-overwrite fact and the promote-blind "verify your prior
    return" copy named, plan-scoped exactly as r4 prescribed.
  - **(d) §6 KAT landed** — the loss-stealing reorder scenario (Y flagged AND the advisory names the
    §1212(b) cascade into the later filed year whose crypto legs are unchanged) plus the §170(d)
    write-back direction, "quoted where the machinery computes, else named-unquantified (loud, never
    silent)".
  - **Trigger completeness re-derived:** the clause fires on "a change to Y's net capital gain/loss or its
    charitable deduction" — exactly the complete set: `apply_170b`'s `carryover_out` is a function of AGI
    ceilings + gift amounts (`charitable.rs:101-186`), so a pure gain-Δ (AGI path, even a cash-donation
    year) and a deduction-Δ are the only movers; a both-Δs-zero flagged year (equal-basis swap) moves
    neither chain. No under-trigger, no over-trigger.
  - **No third chain (census exhaustion re-confirmed):** the only other field the write-back stamps —
    `qbi.reit_ptp_carryforward_in` — is crypto-INSENSITIVE: `reit_ptp_carryforward_out =
    (carryforward_in − reit_dividends).max(0)` (`qbi.rs:80`), pure non-crypto `ReturnInputs`; a promote
    cannot move it. AMT credit is out of scope (§5 non-goal); no NOL/state modeling exists. A gift
    reorder's pool-composition effect on later years surfaces as leg diffs in the consuming years —
    caught by the r3-widened predicate directly, not a cascade gap.
- **M-1 (gift-only "never $0" / false donor 1040-X) → RESOLVED.** BG-D9's new donation/gift sub-bullet and
  BG-D6's reworked removal-flagged term match the code split exactly: Donation `Removal` stores
  `claimed_deduction: Some(..)` (LT→FMV, ST→`min(FMV, basis)`, `fold.rs` Donate arm), Gift stores `None`
  (`fold.rs` Gift arm) — so a gift-flagged year now quotes the **§1015 carryover-basis-Δ** (`Σ leg.basis`
  over gift removal legs; `RemovalLeg.basis` at `state.rs:185`, `PartialEq, Eq` derives at
  `state.rs:181/201` — equally profile/table-free from the fold pair), with the tax-correct copy: *"the
  recorded donee-basis (§1015 carryover) documentation for year Y changes; the donor's Form 1040 is
  unaffected"* — TRUE (a gift is no income, deduction, or gain line on the donor's return; §1015 moves the
  DONEE's basis; the Form 709 basis-column note is correctly conditional on one having been filed) — and
  **no 1040-X assertion**. The both-Δs-zero fallback (name the changed 8283 acquisition dates / donee-basis
  records, never a bare "~$0") is present in BG-D9 and reachable from BG-D6 via the filed-content-delta
  third arm. The §6 KAT pins the gift-only variant verbatim (fires, quotes carryover-Δ, no 1040-X).

**Nothing reopened:** the r4 fold's touch surface (BG-D6 removal-flagged sentence, BG-D9 two new
sub-bullets, §3 8c, §6 new KAT block + arch-N-1 word-order) leaves every r1–r3 fix clause intact —
current-year Σ inclusion, removal-leg widening, clamped quantification, uncomputable-year loudness,
Σ-gain demotion, no-current-price fallback all re-confirmed present in the folded text.

---

## NEW findings

### N-1 (BG-D9 cascade clause) — quoting-condition wording: "the `charitable_carryover_out` diff when Y's absolute return computes" is singular where the Δ needs the fold PAIR.
The capital-loss side is precise ("when both folds compute Y"); the charitable side says "when Y's absolute
return computes", but the Δ is definitionally pre-fold vs post-fold `assemble_absolute` outputs, and the
post-fold assembly can refuse where the pre-fold computed (e.g. the taxable-income≤0-with-carryforward
refusal, `return_1040.rs:1436-1443`, can flip when the promote moves Y's gain). The governing sentence
("quoting the Δ only where the machinery computes it … else named-unquantified") disambiguates, and any
implementation that can't produce both values falls to named-unquantified — no wrong filing is reachable.
Word-order fix for the plan: "when Y's absolute return computes **over both folds**". Does not gate.

---

## Disposition

The r4 fold holds under adversarial re-derivation: every symbol cited exists and behaves as claimed, the
cascade naming is honest in both directions (no silent later-year understatement left unnamed; no implied
computation the tool doesn't do), the gift/donation split matches the `Some`/`None` code fact, and the
fifth-pass census closes the carryover-chain class (QBI REIT/PTP verified crypto-insensitive; AMT out of
scope). Five rounds: disposal legs (r1) → §170(e)/8283/§1015 (r2) → removal-leg reorders + current-year Σ
(r3) → cross-year carryover cascade (r4) → exhausted (r5). Both lenses are now green (arch r4, tax r5).
**Gate: 0C/0I — the spec review loop converges. One Nit filed for the Phase-1a plan.**

| Severity | Count |
|---|---|
| Critical | 0 |
| Important | 0 |
| Minor | 0 |
| Nit | 1 |
