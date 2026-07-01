# R0 architect review — SPEC P2-A §170(e) charitable-deduction (round 1)

- **Artifact:** `design/SPEC_p2a_170e_deduction.md`
- **Baseline verified against:** HEAD `0798051` ("Merge slug: minimal qualified-appraisal trigger")
- **Reviewer role:** independent architect (R0 gate). Author ≠ reviewer.
- **Gate rule:** proceed to plan/implement only at **0 Critical / 0 Important**.
- **Verdict:** ❌ **NOT green — 1 Critical, 2 Important, 3 Minor, 2 Nit.** Blocking.

---

## A. Citation / drift verification (recon vs. current source)

Every path/line the spec cites was checked against HEAD `0798051`. Drift is minimal; the tax formula is the problem, not the recon.

| Spec claim | Current source | Result |
|---|---|---|
| `RemovalLeg{basis, fmv_at_transfer, term, basis_source}` + `Removal` @ `state.rs:142-158` | `RemovalLeg` 142-149, `Removal` 150-158 | ✅ accurate |
| `Removal` is a projection struct, **not** a persisted serde event | `state.rs:150` derives `Debug, Clone, PartialEq, Eq` only — **no `Serialize`/`Deserialize`** | ✅ confirmed — additive `Option<Usd>` field needs **no migration**; it is a deterministic fn of legs so it stays inside `PartialEq`/determinism tests (same treatment as `FoldStats`) |
| Donate arm `fold.rs:1004-1113`; `deduction_proxy` block `1067-1104` | Donate arm 1004-1113; proxy sum 1074-1083; `QualifiedAppraisalNote` 1084-1104; push 1105-1112 | ✅ (comment starts 1067; the *computation* is 1074-1083 — see Nit n1) |
| Proxy = `Σ(leg.term==LongTerm ? fmv_at_transfer : basis)` over FINAL legs (after `make_removal_legs` + `carry.rehome_onto_removal_leg`) | Exactly `fold.rs:1074-1083`; legs finalized 1041-1058 | ✅ accurate |
| No dealer/self-created signal; capital-asset assumed | `resolve.rs:696` "(4) Capital-asset eligibility (§4.02): assumed for a personal investor (no Phase-1 dealer flag)." | ✅ accurate |
| `QUALIFIED_APPRAISAL_THRESHOLD` = $5,000 (tables.rs) | `tables.rs:119 pub const … = dec!(5000)` | ✅ accurate |
| `render_removal_leg` `render.rs:280-288` | matches | ✅ accurate |
| `removals.csv` writer `render.rs:566-595` (cols: event,kind,removed_at,lot,sat,basis,fmv_at_transfer,term) | matches | ✅ accurate |
| `render_report` per-year filter pattern | `render.rs:154-261`; removals filtered by `r.removed_at.year()`, header line `226-232`, leg lines via `render_removal_leg` | ✅ accurate — header line is the correct per-donation hook |
| `compute_tax_year` never reads `state.removals`; `TaxProfile.ordinary_taxable_income` user-supplied | `compute.rs:225-381` reads only `state.disposals` + `state.income_recognized`; `types.rs:31-50` | ✅ confirmed (see Finding on §3 below) |

**One recon inaccuracy (→ Minor m1):** D1/Task 1 say "Update all `Removal { .. }` literals (tests)." There are **no `Removal { .. }` literals in any test** — grep finds exactly two literals in the whole tree, both production pushes in `fold.rs`: the **Gift arm `fold.rs:994`** and the **Donate arm `fold.rs:1105`**. (The `appraisal_required:` literals in tests are `OutflowClass::Donate {…}` / event constructions, not `Removal`.) The compiler-forcing edit is the Gift-arm push (must get `claimed_deduction: None`) — the spec must name it explicitly.

---

## B. Findings

### 🔴 Critical

**C1 — The short-term formula over-states the deduction for depreciated property; the "exact" claim is false in an in-scope case.**

The rule `claimed_deduction = Σ(leg.term==LongTerm ? fmv_at_transfer : basis)` is correct for **appreciated** property only. §170(a)/Reg 1.170A-1(c) start the contribution amount at **FMV**; §170(e)(1)(A) only ever *reduces* it. For a **short-term leg that has declined in value (`basis > fmv_at_transfer`)** a hypothetical sale yields a **loss**, so "the amount of gain which would not have been long-term capital gain" is **zero** → no reduction → the deduction is **FMV, not basis**. IRS Pub 526 states this flatly: *"If you contribute property with a fair market value that is less than your basis in it, your deduction is limited to its fair market value. You cannot claim a deduction for the difference between the property's basis and its fair market value."*

- The spec's `basis` **over-states** by `(basis − fmv)` for any ST-held, depreciated donated lot. This is squarely inside the **modeled universe** (non-dealer investor, capital asset, public charity) — it is *not* one of the disclosed out-of-scope over-state cases (dealer/self-created), and **no caveat covers it**. Promoting this sum from a trigger-only "proxy" (where over-flagging is explicitly acceptable) to a **surfaced, summed, "exact" Schedule-A `claimed_deduction`** turns an acknowledged over-estimate into a silently over-claimable filed figure. That is exactly the honesty defect the proxy→exact rename (evaluation #4) is meant to remove.
- LT is fine: `fmv_at_transfer` is correct even when depreciated (you forfeit the loss). Only the ST branch is wrong.

**Exact fix.** Per-leg deduction is always capped at FMV:
```
let leg_ded = if leg.term == Term::LongTerm { leg.fmv_at_transfer }
              else { leg.fmv_at_transfer.min(leg.basis) };  // ST/ordinary: min(fmv, basis)
```
Update D1, D2, and Task-1 KAT (a) to this formula. `min` equals `basis` in every appreciated/at-par ST case (all existing behavior preserved) and equals `fmv` only when depreciated — strictly correct, never worse. Add a locking KAT (see m3).

*Interaction with the appraisal trigger:* the §170(f)(11) test keys off the **claimed deduction**, so `min(fmv,basis)` is *more* legally correct for the trigger too. It changes the flag **only** for ST-depreciated donations — a case **no existing KAT exercises** (existing KATs are all appreciated: LT $60k, ST $2k-basis, mixed, $5000 boundaries). So all current KATs are unchanged; see Minor m2 re: wording.

---

### 🟠 Important

**I1 — Private-foundation donee is an undisclosed over-state case; the "exact for a non-dealer individual investor" claim needs a donee assumption.**

The LT→FMV branch assumes a §170(b)(1)(A) **public charity**. Under **§170(e)(1)(B)(ii)**, a contribution of appreciated long-term capital-gain property to a **non-operating private foundation** is reduced by the would-be LTCG — i.e. **deducted at basis** — *except* for **"qualified appreciated stock"** (publicly-traded corporate stock). Crypto is **not stock** (confirmed independently and by the app's own cite CCA 202302012: digital assets are not §165(g)(2) securities). So an LT crypto gift to a private foundation deducts at **basis**, and the app's `fmv_at_transfer` **over-states** — the same failure mode as the dealer caveat, on a different axis, and it hits the *same* "non-dealer individual investor" the spec claims to model exactly.

**Fix.** Either (a) add a third retained caveat to the D2 advisory + a spec Out-of-scope bullet: *"assumes a §170(b)(1)(A) public-charity donee; a gift of appreciated crypto to a non-operating private foundation is reduced to basis under §170(e)(1)(B)(ii) (crypto is not qualified appreciated stock) — donee type is not modeled; this figure would over-state,"* or (b) restrict the "exact" wording to "…to a public charity." Do not ship the bare "exact for a non-dealer individual investor" claim without the donee qualifier.

**I2 — The surfaced per-year total and advisory must state "before §170(b) AGI limits / carryover" (and the 2026 OBBBA floor) or it misleads as a deductible amount.**

Putting §170(b) computation out of scope is **acceptable** — the claimed deduction *before* AGI limits is a legitimate, well-defined figure (it is what Form 8283 reports). But the spec surfaces it as a **"Schedule-A itemized figure"** with **no qualifier**, and Schedule A frequently deducts *less*: appreciated capital-gain property to a public charity is capped at **30% of AGI** (20% to non-50% orgs), with a **5-year carryover** — and, because `currentDate` is 2026 and the report can surface 2026 donations, OBBBA adds a **0.5%-of-AGI floor** and a **35% itemized-value cap** for tax years beginning after 2025. A user could read the unqualified "charitable-deduction total" as this year's deductible amount and over-claim.

**Fix.** Label the report line and CSV column as **claimed deduction *before* §170(b) AGI percentage limits and carryover** (and note the 0.5% floor applies to 2026+ years). Cheap, non-code (D3 + advisory text). Keep §170(b) computation out of scope.

---

### 🟡 Minor

**m1 — "update all `Removal { .. }` literals (tests)" is inaccurate.** No test constructs a `Removal` literal. The only two literals are production pushes: **Gift arm `fold.rs:994`** (set `claimed_deduction: None`) and **Donate arm `fold.rs:1105`**. Task 1 must explicitly call out the Gift-arm push (it is the compile-forcing site and the `Gift → None` KAT's basis) and drop the "(tests)" claim.

**m2 — Task 3 invariant wording is now wrong.** "the trigger drives off it identically to before (no appraisal-flag regression)" cannot be literally true once C1 is fixed. Reword to: *"identical for all appreciated donations (every existing appraisal KAT unchanged); the exact figure correctly changes the flag only where the old proxy over-stated (ST-depreciated) — that is a correction, not a regression."*

**m3 — Add a KAT that locks C1.** New Task-1 KAT: ST leg with `basis $8,000 / fmv $3,000` → `claimed_deduction == Some($3,000)` **and** `QualifiedAppraisalNote` does **not** fire (3k ≤ 5k), whereas the retired proxy (`basis $8k`) *would* have fired. Without this KAT nothing prevents regressing the ST branch back to `basis`. Also keep an explicit LT-depreciated KAT (`basis $8k / fmv $3k`, LT → `$3,000`) to pin that LT stays FMV-capped.

### ⚪ Nit

**n1 — Cite precision.** "the `deduction_proxy` 1067-1104" — 1067 is the comment header; the sum is `1074-1083` and the note `1084-1104`. Tighten to `1074-1104` if you want the computation+emit span.

**n2 — Naming consistency.** If you adopt the I2 label, keep the CSV header machine-stable (e.g. `claimed_deduction`) and put the "before §170(b) limits" wording in the human report line / column doc, not the CSV header string, so `export` KATs stay simple.

---

## C. Point-by-point answers to the review charter

1. **Deduction rule (highest priority):** LT→FMV is exactly §170(e)(1)(A) for a public-charity gift of capital-gain property. ST→basis is **wrong for depreciated ST property** (should be `min(fmv,basis)`) → **C1**. Per-leg-then-sum for a mixed LT/ST donation is the right structure (each leg its own character, summed). Web-verified below.
2. **Gift → None:** ✅ correct. A §102 gift to an individual is not a §170 contribution; only `RemovalKind::Donation` gets `Some(amount)`. Confirm the Gift-arm push (`fold.rs:994`) sets `None` (m1).
3. **Standalone / not feeding B:** ✅ **not a hole, and no double-count.** `compute_tax_year` reads only `disposals` + `income_recognized`, never `removals`. Critically, `TaxProfile.ordinary_taxable_income` is *taxable* income "EXCLUDING app-computed crypto items" — i.e. **already net of the user's Schedule A** (post-deduction). So B must **not** subtract the §170 figure again, and it doesn't. Documenting "user applies it on Schedule A" is the right call. The spec's FOLLOWUP correctly flags the trap: a future auto-reduce of `ordinary_taxable_income` by this figure would **double-count** (the user already subtracted it) — good foresight; keep that note.
4. **Proxy→exact honesty:** conditionally OK. The two caveats (dealer/inventory §1221(a)(1); §170(f)(11)(F) aggregation) + CCA 202302012 are retained and the trigger still fires off the (now exact) amount — **but** the "exact" claim is only honest after **C1** (depreciated ST) **and I1** (private-foundation donee) are addressed; today the rename over-reaches. No appraisal-flag regression for existing KATs.
5. **Field/surfacing:** ✅ `Removal.claimed_deduction: Option<Usd>` is a projection field, **no serde/migration** (confirmed). Per-donation display on the header line (`render.rs:226-232`) is the correct hook; CSV column + per-year total via the existing `removed_at.year()` filter are correct. Adjust literal-update wording per m1.
6. **§170(b) AGI limits + carryover out of scope:** **acceptable**, but the reported figure is **misleading without a disclaimer** → **I2** (add "before §170(b) AGI limits / carryover," note 2026 OBBBA 0.5% floor).
7. **Scope / TDD:** 3 tasks are right-sized, testable, and TDD-ordered. KATs are genuine but must add m3 (ST/LT-depreciated) and re-word m2. Legal cites are accurate (all web-verified). Remaining gaps (dealer/self-created character detection, aggregation, donee type, AGI limits) are correctly deferred once disclosed.

---

## D. Independent web-confirmation of the cited law

- **§170(e)(1)(A) FMV reduction — CONFIRMED.** Contribution is reduced by the gain that would *not* be LTCG if sold at FMV → LT capital-gain property deducts at FMV; ordinary/ST property deducts at basis. IRS worked example: inventory FMV $600, basis $400 → deduction $400 = $600 − ($600−$400). (law.cornell.edu/uscode/text/26/170; 26 CFR 1.170A-1(c), 1.170A-4; bradfordtaxinstitute IRC 170(e)(1)(A).)
- **Depreciated property capped at FMV — CONFIRMED (drives C1).** Pub 526: "If you contribute property with a FMV that is less than your basis … your deduction is limited to its fair market value. You cannot claim a deduction for the difference between the property's basis and its fair market value." (irs.gov Publication 526.)
- **§1221(a)(1) dealer/inventory — CONFIRMED.** "Stock in trade … or property held … primarily for sale to customers in the ordinary course of his trade or business" is excluded from capital-asset status; gain is ordinary regardless of holding period → deduct at basis under §170(e). (irc.bloombergtax IRC 1221; 26 CFR 1.1221-1.)
- **§170(e)(1)(B)(ii) private foundation / qualified appreciated stock — CONFIRMED (drives I1).** Appreciated property to a non-operating private foundation is reduced by the would-be LTCG (to basis); exception only for "qualified appreciated stock" = publicly-traded corporate stock. Crypto is not stock (also per CCA 202302012, digital assets are not §165(g)(2) securities). (law.cornell.edu/uscode/text/26/170.)
- **§170(b) AGI limits + carryover — CONFIRMED (drives I2).** 30% AGI for capital-gain property to a public charity (at FMV), 20% to non-50% orgs, 60% cash; 5-year carryover (§170(d)(1)). OBBBA: new 0.5%-of-AGI floor + 35% itemized-value cap for tax years beginning after 12/31/2025. (law.cornell.edu/uscode/text/26/170; nationaltaxtools/gtlaw OBBBA guides.)
- **CCA 202302012 — CONFIRMED as cited.** Crypto donation with claimed deduction >$5,000 requires a qualified appraisal; the readily-valued/exchange-price exception does not apply; no reasonable-cause relief. (irs.gov/pub/irs-wd/202302012.pdf; Journal of Accountancy Jun 2023.)

---

## E. Exit criteria for round 2 (what makes this green)

1. **C1:** ST/ordinary branch → `fmv_at_transfer.min(basis)` in D1, D2, KAT (a); add m3's ST- and LT-depreciated KATs.
2. **I1:** add the private-foundation / public-charity-donee caveat (advisory + Out-of-scope) or qualify "exact" to "…to a public charity."
3. **I2:** label the surfaced total + CSV column "before §170(b) AGI limits / carryover" (note 2026 OBBBA 0.5% floor).
4. Fold m1/m2 (Gift-arm literal at `fold.rs:994`; reword the "identical trigger" invariant).
5. Re-run this review after the fold (author ≠ reviewer; persist round 2 verbatim before folding).

---

# Round 2 — re-review (post-fold)

- **Artifact:** `design/SPEC_p2a_170e_deduction.md` (revised).
- **Scope:** confirm the round-1 fold closed C1 + I1 + I2 + m1 + m2 and introduced no new tax error / no new Critical or Important. Round-1-confirmed items (recon accuracy; standalone / not-feeding-B; Gift→None; `Removal` projection-struct / no-migration) were **not** re-litigated per charter.
- **Verdict:** ✅ **GREEN — C1/I1/I2 closed, 0 residual tax error, 0 new Critical/Important, m1/m2 folded. Ready to implement.** (2 cosmetic nits below, non-blocking.)

## 1. C1 — CLOSED (tax correctness). ✅
Rule is now `LT → fmv`; `ST → min(fmv, basis)` (not `ST → basis`). Verified tax-correct in all four cases against §170(e)(1)(A) (reduce FMV only by would-be non-LTCG gain) with the §170(a)/Pub 526 FMV cap:

| leg | basis vs fmv | correct deduction | rule result |
|---|---|---|---|
| LT appreciated | basis<fmv | FMV (would-be gain is LTCG → no reduction) | `fmv` ✓ |
| LT depreciated | basis>fmv | FMV (would-be loss → no reduction; loss forfeited) | `fmv` ✓ |
| ST appreciated | basis<fmv | basis (ST gain reduces FMV) | `min(fmv,basis)=basis` ✓ |
| ST depreciated | basis>fmv | FMV (would-be loss → no reduction) | `min(fmv,basis)=fmv` ✓ |

Stated consistently in Legal grounding (L14-20), D1 (L57, L60-62), D2 (L69-72), and the KATs — Task-1(a) covers LT-only / ST-appreciated→basis / ST-depreciated→fmv / mixed (L108-110), and the ST-depreciated **lock** is present twice: KAT (a) and the trigger lock (c) `basis $8k / fmv $3k → $3k → QualifiedAppraisalNote does NOT fire` (L112-114), which the retired `basis=$8k` proxy would have fired.

**No residual "ST→basis" over-claim in the modeled universe.** The only remaining `→ basis` statements are the two disclosed *out-of-scope* axes — dealer/ordinary-character (L21-22, L74) and private-foundation LT (L26-29, L76-78) — plus L41-42, which correctly *recites the existing shipped proxy code* in the recon (not the new rule). Clean.

## 2. I1 — CLOSED. ✅
Donee-type caveat added and accurate: appreciated LT crypto to a **non-operating private foundation** reduces to **basis** under §170(e)(1)(B)(ii) (crypto ≠ qualified appreciated stock, per CCA 202302012), correctly scoped to *appreciated* LT. Present in Legal grounding (L26-29), D2 advisory caveat (b) (L76-78), Out-of-scope (L142-143), and Task-1(d) KAT (L115). Consistently placed; the "exact" claim is now scoped to a public-charity donee.

## 3. I2 — CLOSED. ✅
The per-year report total is labeled "charitable deduction (Schedule A itemized) — BEFORE §170(b) AGI limits / carryover" (D3 L88-90), with the OBBBA-2026 0.5% floor / 35% cap noted (Legal L30-34; Out-of-scope L144-145). §170(b) computation stays out of scope. Task-2 KAT asserts the qualifier label (L124). CSV header stays machine-stable `claimed_deduction` with the pre-limit qualifier in the column doc (L90-91) — also satisfies round-1 n2.

## 4. Minors — addressed. ✅
- **m1:** D1 (L62-63) states there are **NO** `Removal { .. }` test literals; the compile-forcing site is the **Gift arm `fold.rs:994`** set to `claimed_deduction: None`; both production pushes updated. The erroneous "(tests)" claim is gone.
- **m2:** Task 3 (L128-130) no longer claims "identical" — it states the trigger "now **differs from the old proxy ONLY for ST-depreciated donations** (correctly, fewer false flags — NOT a regression)." D2 (L69-72) mirrors this.

## 5. No new Critical/Important from the fold. ✅
- **Shipped appraisal trigger intact for tested cases:** every existing appraisal KAT is appreciated (LT $60k → flag; ST $10k/$2k → min=2k → no flag; $5000.00/$5000.01 boundaries); for appreciated ST `min(fmv,basis)=basis`, identical to the old proxy → no KAT weakened (Task-1(c), L112-114).
- **Caveats coherent:** (a) dealer, (b) private-foundation donee, (c) aggregation are complementary negations of the three scope conditions (non-dealer / public-charity / per-donation) — no mutual contradiction and no contradiction with the now-scoped "exact" claim (non-dealer investor / capital asset / public charity / incl. depreciated via `min(fmv,basis)`).
- **Internal consistency holds:** additive `Option<Usd>` projection field, no serde/migration, standalone (no change to B), right-sized 3-task TDD plan with genuine KATs.

## Residual (non-blocking nits, fold at author's discretion)
- **n1-r2 (nit):** Task-1 file line (L103) annotates `state.rs (field + `Removal` literals)`; the two `Removal` literals are the production **pushes in `fold.rs`**, not `state.rs` (which holds the struct def). D1 already states this correctly, so it will not mislead — cosmetic only.
- **n2-r2 (nit):** Round-1 m3's *secondary* ask — a distinct **LT-depreciated** KAT (`basis $8k / fmv $3k`, LT → `$3,000`) — is folded under "LT-only" (L108) rather than called out separately. The ST-depreciated lock (the load-bearing half of m3) is present. Because the formula maps `LT → fmv` unconditionally there is no branch to regress, so this is optional symmetry, not a gap.

**Gate:** 0 Critical / 0 Important. Spec is **R0 GREEN — cleared to plan/implement.**
