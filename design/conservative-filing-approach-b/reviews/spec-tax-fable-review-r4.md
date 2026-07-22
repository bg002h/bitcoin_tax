# Independent TAX review — Approach B sub-project 1 SPEC, round 4 (post r3-fold)

**Artifact:** `design/conservative-filing-approach-b/SPEC.md` (DRAFT, r1+r2+r3 two-lens folds applied, `566c635`).
**Lens:** US federal tax correctness / completeness / honesty. Adversarial; every r3-fold claim re-verified
against statute/reg AND current source (`project/{fold,pools,resolve}.rs`, `tax/{return_1040,charitable,
compute}.rs`, `conservative.rs`, `state.rs`, `forms.rs`, `void.rs`, `btctax-adapters/src/tax_tables.rs`,
`btctax-cli/src/cmd/tax.rs`).
**Provenance honored:** `DESIGN_PROVENANCE.md` + all six prior reviews read; no adjudicated ruling or
verified-resolved finding re-litigated. Parent-spec guarantees (amended Invariant, D-7 re-scope, D-8
exclusion) checked — no violation.
**Reviewer:** Fable (independent; not the author of the fold). **Date:** 2026-07-21.

**Verdict: NOT green. 0 Critical / 1 Important / 1 Minor / 0 Nit.**

The r3 fold is faithful and complete on its own terms: both r3 Importants and both Nits are genuinely
resolved (§V, each re-derived against source), the widened removal-leg predicate really does catch the
donation-reorder scenario, the deduction-Δ is genuinely profile-free, and the current-year Σ inclusion
double-counts nothing. The one new Important is the next — and, per my census, last — ring of the same
whole-surface class: the fold-diff year set stops at leg-diff years, but a flagged year's delta propagates
through two RETURN-level carryover chains the product itself models (§1212(b) capital-loss;
§170(d) charitable — the latter auto-written-back by `write_back_carryover`) into later filed years whose
legs are unchanged. The capital-loss chain appears nowhere in the spec, and the advisory's amendment copy
names an incomplete set. Fix is copy + census + one KAT — no new machinery.

---

## V — Verified resolved (r3 → fold, each checked against source/law)

- **I-1 (consent Σ dropped the current year) → RESOLVED.** BG-D6 bullet 2 now defines the Σ over "every
  year the pre/post fold pair differs in filed content, INCLUDING the current year (equivalently: the
  BG-D9 diff run WITHOUT its `< current` advisory filter)", names the dominant-flow scenario
  (sell-then-promote-before-filing), and the §6 gates KAT pins "the Σ INCLUDES the current year's
  realized delta" with that exact scenario. The "equivalently" claim is true: the unfiltered diff flags
  the current year iff its filed content changed, which is exactly the realized-delta condition.
  **No double-count:** the realized current-year term covers disposed sats; the unrealized line is scoped
  to "sats not yet disposed" — disjoint sat sets; and the ADVISORY keeps `< current`, so the 1040-X copy
  is not duplicated onto the unfiled current year (the file-early edge — promote in Jan Y+1 before filing
  year Y — lands in the advisory's `< current` set with the conditional "if Y was already filed" copy,
  which is correct). **No new Acknowledgment inconsistency:** each term stays flagged computed-tax-Δ vs
  gain/deduction-Δ; heterogeneous terms are never silently summed into one number.
- **I-2 (fold-diff DISPOSAL-scoped, blind to donation/gift reorders) → RESOLVED.** The predicate now
  ranges over "disposal legs (8949) AND removal legs (8283 / Schedule-A donation + §1015 gift carryover)"
  (BG-D9), §3 item 8 wires `state.removals` per prior year, and the §6 lifecycle KAT pins the prior
  DONATION/GIFT-only year with NO disposal-leg change, both directions. Verified the widening genuinely
  catches the r3 scenario: gifts/donations draw through the same `consume_principal` as disposals
  (`fold.rs:625/789/1108/1185` — the four call sites), `make_removal_legs` (`fold.rs:225`) is the only
  removal-leg builder, `Removal`/`RemovalLeg` derive `PartialEq, Eq` (`state.rs:181, 201`) with
  `removed_at` year-scoping (`state.rs:205`), and a reordered draw changes `leg.lot_id` even at equal
  basis — so the per-year set diff fires on exactly the HIFO-reorder mechanism (`hifo_cmp`'s
  `usd_basis == ZERO` sorts-last exit, `pools.rs:275-287`). **The deduction-Δ is genuinely profile-free:**
  `claimed_deduction` is computed in the fold from final legs (`fold.rs:1231-1240, 1276` — LT→FMV,
  ST→`min(FMV, basis)`), needs no profile and no table; both folds share the same price data, so the Δ is
  well-defined for the 2018–2023 table-less audience years. **The honesty clause is present:** BG-D9's
  copy mandates the ~$D clause "must NOT imply the ~$Δ tax figure captures the deduction effect — engine
  B can't price it" — verified true: `compute_tax_year` contains no charitable term and the derive-side
  Schedule A is crypto-donation-free by design (`return_1040.rs:757-760`); only
  `crypto_charitable_gifts` (`return_1040.rs:524-553`, year-scoped over `state.removals`) → `apply_170b`
  prices it. (One precision residue in the "Σ claimed_deduction, never $0" quoting rule for GIFT-flagged
  years → new **M-1**; the donation scenario the finding named is fully caught — this is not a
  reopening.)
- **N-1 (Σ-gain "equivalence") → RESOLVED.** BG-D9 now names the leg-SET diff as the operative predicate
  and demotes Σ-gain to "a usually-visible consequence, not an equivalence", with the equal-basis
  different-date counterexample inline.
- **N-2 (no-current-price unrealized line) → RESOLVED.** BG-D6 defines the fallback: latest bundled
  close + its date, or the explicit "no current price data — the floor itself, $`filed_basis`, is the
  maximum gain reduction" — never a silent $0 or dropped line. The maximum-reduction claim is a true
  upper bound (the clamp can only reduce the realized saving below the floor, never above it).
- **(Continuity) arch r3 N-1 → folded consistently:** BG-D9-ii is now scoped to non-voided promotes and
  the §6 lifecycle KAT carries the no-spurious-Hard clause. No tax-lens impact.

**Citation staleness sweep:** every symbol the r3 fold cites was re-opened on the branch — all current
(`BundledTaxTables::load()` still {2017, 2024, 2025, 2026}, `tax_tables.rs:75-78`; `consume_fifo`
FIFO-pinned, `pools.rs:62-65`; `overpayment_delta_one`, `conservative.rs:284`; `claimed_deduction`
`Some`/`None` split, `fold.rs:1276/1151`). No stale cite found.

---

## NEW findings

### I-1 (BG-D9 / BG-D6 / §3 / §6) — The fold-diff year set stops at leg-diff years, but a flagged year's delta propagates through the return-level carryover chains — §1212(b) capital-loss and §170(d) charitable, BOTH modeled by the product — into LATER filed years whose legs are unchanged: those years are neither flagged, nor in the consent Σ, nor named by the amendment copy, and the capital-loss chain appears nowhere in the spec.

**Defect:** BG-D9 flags exactly the years whose disposal/removal leg sets differ, and its copy asserts the
amendment set ("claiming it requires a Form 1040-X for Y"). But a year's FILED return also contains
carryover-in lines derived from the PRIOR year's actuals, and the codebase models both chains:
(i) **§1212(b) capital-loss** — engine B computes a per-year `carryforward_out` that "feeds next year's
`capital_loss_carryforward_in`" (`types.rs:111-112`), and `carryforward_consistency`
(`compute.rs:436-448`, wired at `cmd/tax.rs:442-466`) warns when year Y+1's declared carryforward-in
mismatches Y's computed carryforward-out; (ii) **§170(d) charitable** — `write_back_carryover`
(`cmd/tax.rs:481-590`) / `apply_carryover_writeback` (`return_1040.rs:1448-1491`) stamps year Y's
computed `charitable_carryover_out` (derived from the removal legs via `crypto_charitable_gifts` →
`apply_170b`) into Y+1's stored `ReturnInputs`, **silently overwriting a Computed-provenance value**. A
promote (or void) that reorders year Y therefore changes what Y+1's correct Schedule D lines 6/14 or
Schedule A line 13 ARE — while Y+1's leg sets are byte-identical between the folds, so nothing flags
Y+1, the consent Σ has no Y+1 term, and the copy tells the filer to amend Y only. The spec's existing
§170(d) sentence ("the collapsed deduction also corrupts the §170(d) carryover chain") acknowledges one
chain's existence but extends neither the year set nor the copy's amendment claim to the affected years,
and the capital-loss chain is entirely absent from the spec. Engine B cannot compensate: it applies the
same `profile.capital_loss_carryforward_in` in both folds (`compute.rs:317`), so the per-year tax-Δ is
structurally blind to the cascade.

**Concrete failure scenario (promote direction, capital-loss chain):** documented lot 2 BTC @ $8k/BTC
(2020); $0 tranche, Dec-2017 window (~$12k floor). 2024: sells 2 BTC @ $5k/BTC — HIFO draws the
documented lot ($8k outranks the $0-sorted-last tranche) → $6k loss → $3k allowed under §1211(b), $3k
carryforward; files 2024 and 2025 (2025 consumes the $3k on Schedule D lines 6/14). 2026: promotes the
undisposed tranche to the $12k floor → the tranche outranks $8k → the re-folded 2024 sale draws 1 BTC
tranche (clamped, $0 gain) + 1 BTC documented → 2024 net loss is now only $3k, fully absorbed in 2024 —
**no carryforward exists**. 2024 is flagged (legs differ, gain-Δ +$3k, amend-to-pay) — correct. 2025's
filed $3k carryover deduction is now unsupported (amend-to-PAY 2025) — **unflagged, unquoted, unnamed**;
the filer who follows the tool's copy amends 2024 alone and leaves a real understatement sitting on
2025. The in-product `carryforward_consistency` advisory would eventually warn — but only on a later
`tax` run, only when BOTH years have profiles and Y computes (never for a 2018–2023 pair — the audience
years), and with copy ("verify your prior return") that does not connect to the promote. The §170(d)
mirror: an over-ceiling donation year whose deduction collapses under BG-D11 zeroes its
`carryover_out`; the next `--write-carryover` run silently rewrites Y+1's stored inputs — the same
tool-generated silent divergence the r3 converged blocker forbade, one derivation step removed. The
VOID direction inherits both (amend-to-refund, §6511-bounded).

**Authority/code fact:** §1212(b) + §1211(b) (the carryforward is a derived line of Y+1's return, not an
independent fact); §170(d)(1) + Reg §1.170A-10(a)(2) (carryover ages/reduces from Y's actuals);
§6662(d) (the Y+1 understatement is real and silent); `cmd/tax.rs:442-466/481-590`,
`return_1040.rs:1448-1491`, `compute.rs:317, 436-448`, `types.rs:111-112` — the chains are the
product's own machinery, not hypothetical filer behavior.

**Fix (in-spec, proportionate — copy + census + one KAT; NO mechanical flagging of cascade years is
demanded):** quantifying the cascade is profile/AGI-gated, so per the spec's own loud-uncomputable
pattern the floor is *naming*, not computing. (a) **BG-D9 copy:** when a flagged year Y's diff changes
its net capital gain/loss or its charitable deduction, the advisory adds one clause — *"carryover-linked
lines of later filed years (Schedule D capital-loss carryforward, §1212(b); Schedule A charitable
carryover, §170(d)) derive from year Y and may also require amendment, even though those years' crypto
transactions are unchanged"* — quoting the Δ only where the machinery computes it (engine B's
`carryforward_out` diff when both folds compute Y; the absolute return's `charitable_carryover_out`
diff when Y's absolute return computes), else named-unquantified. Both directions. (b) **BG-D6:** the
same cascade note on the exposure side, recorded in the `Acknowledgment` as a named-unquantified term
when unpriceable (never silently absent). (c) **§3 census:** add `carryforward_consistency`
(`compute.rs` / `cmd/tax.rs`) and `write_back_carryover`/`apply_carryover_writeback` as
promote-adjacent sites (the plan decides whether their copy becomes promote-aware; the census must at
least list them so the silent-overwrite path is considered). (d) **§6 KAT:** the loss-stealing reorder
scenario above — Y flagged AND the advisory names the Y+1 §1212(b) cascade; the write-back direction
pinned for §170(d).

### M-1 (BG-D9 / BG-D6) — The removal-flagged quoting rule ("Σ `claimed_deduction` … never $0") and the 1040-X copy are wrong for the GIFT-flagged case: gifts carry `claimed_deduction: None`, so the mandated quantity is identically $0, and a gift-only reorder changes no line of the donor's 1040.

**Defect:** `claimed_deduction` is `Some` only for Donations (`fold.rs:1276`); Gifts store `None`
(`fold.rs:1151`). For a prior GIFT-only year the widened predicate correctly fires (the §1015 carryover
basis the tool documents in `removals.csv` changed), but the spec's mandated quote — "the profile-free
deduction-Δ (`Σ claimed_deduction` from the fold pair), never $0" — evaluates to exactly the bare $0 the
sentence forbids, and the copy's "claiming it requires a Form 1040-X for Y" is false for the donor: a
gift changes no income, deduction, or gain line on the donor's 1040 (the changed artifacts are the
donee-basis documentation / `removals.csv`, and the basis column of a Form 709 Schedule A where one was
filed — a form the product does not generate). Same shape for a donation reorder swapping equal-basis
same-term lots (deduction-Δ genuinely $0 while the 8283 acquisition-date content changed).

**Fix (in-spec, two clauses):** for a removal-flagged year, quote the deduction-Δ for donation legs and
the **§1015 carryover-basis-Δ** (`Σ leg.basis` over gift legs — equally profile-free from the fold pair)
for gift legs; when both are $0, name the changed filed content (8283 dates / donee-basis records)
instead of printing a bare "~$0". The copy distinguishes: donation-flagged → the existing 1040-X
clause; gift-flagged → "the recorded donee-basis (§1015 carryover) documentation for year Y changes;
the donor's Form 1040 is unaffected" (and note the Form 709 basis column where applicable), no 1040-X
assertion. §6: extend the removal-reorder KAT with the gift-only variant.

---

## Disposition

The r3 fold survives adversarial re-derivation intact — this is the first round in the series where
every prior finding's resolution held with no reopening, and my fourth-pass surface census found no new
emitter the promote's basis reaches. The one Important is the terminal ring of the whole-surface class:
having closed every same-year surface (disposal legs r1, §170(e)/8283/§1015 r2, removal-leg reorders
r3), the residue is the cross-YEAR propagation through the two carryover chains the product itself
models. It needs no machinery — a naming clause in the advisory/consent, two census entries, and one
KAT — so r5 should converge immediately. The Minor is a quoting/copy precision fix inside the same
bullets. When the taxonomy changes, the sweep must cover the whole filed surface — and the filed
surface of year Y+1 includes lines derived from year Y.

| Severity | Count |
|---|---|
| Critical | 0 |
| Important | 1 |
| Minor | 1 |
| Nit | 0 |
