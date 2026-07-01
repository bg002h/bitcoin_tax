# SPEC — P2-A: §170(e) charitable-deduction computation (Phase-2, sub-project 1)

**Source baseline:** `origin/main` @ `0798051` (post appraisal-trigger slug).
**Goal:** Compute, store, and surface the **exact §170(e)(1)(A) claimed charitable-deduction amount** per
donation, and drive the just-shipped appraisal trigger off that exact amount (retiring the "proxy"
framing). Standalone Schedule-A reporting figure — does NOT feed engine B. First Phase-2 sub-project;
foundation for Form 8283 / Schedule A (later sub-projects).

**SemVer:** additive `Removal.claimed_deduction` field (projected type, not a persisted event) + CSV
column + report line ⇒ **MINOR** (pre-1.0). No tax-figure change to B.

## Legal grounding
- **§170(a) + §170(e)(1)(A):** the contribution STARTS at FMV; §170(e) only *reduces* it by the gain that
  would NOT be long-term capital gain if the property were sold. So per leg:
  - **long-term capital-gain property → FMV** (would-be gain is LTCG → no reduction; also FMV when
    depreciated, since a would-be *loss* is not a reduction).
  - **short-term property → `min(FMV, basis)`** — [R0-C1] NOT simply "basis": when APPRECIATED
    (basis<FMV) the ST gain reduces FMV to basis; when DEPRECIATED (basis>FMV) there is no would-be gain
    → no reduction → the deduction is **FMV** (capped at FMV per §170(a)/Pub 526). `min(FMV, basis)`
    yields basis in the appreciated case and FMV in the depreciated case.
  - (ordinary-income-CHARACTER property — dealer/inventory/self-created — also → basis, but is unmodeled;
    see Out of scope + the retained caveat.)
- **§1221 / §1221(a)(1):** the property must be a capital asset; dealer/inventory ("held for sale in a
  trade or business") is excluded and is ordinary-income-character (deducts at basis regardless of
  holding period). Not modeled — see Out of scope.
- **§170(e)(1)(B)(ii) (donee type — [R0-I1]):** LT→FMV assumes a **public charity (50%-limit org)**. Appreciated
  LT property given to a **non-operating private foundation** is reduced to **basis**, except "qualified
  appreciated stock" — and crypto is NOT stock (per the app's own CCA 202302012). Donee type is unmodeled
  → the "exact" claim is scoped to a public-charity donee; retain a donee-type caveat.
- **§170(b) (AGI limits — [R0-I2], out of scope but must be DISCLOSED):** the claimed deduction is capped
  by AGI percentage limits (30% for capital-gain property, 20% to non-50% orgs, 60% cash) with a 5-year
  carryover; for tax years from 2026 the OBBBA 0.5%-of-AGI floor + 35%-bracket cap also apply. This
  sub-project computes the claimed-deduction AMOUNT **before** these limits; the surfaced figure MUST say
  so.
- The deduction is a **Schedule A itemized** figure; it does not change capital-gains tax. Form 8283 (>$5k)
  / the appraisal requirement is the §170(f)(11) trigger already implemented as `QualifiedAppraisalNote`.

## What already exists (recon @ 0798051)
- Each donation folds to a `Removal{kind:Donation}` with `RemovalLeg{basis, fmv_at_transfer, term,
  basis_source}` (`state.rs:142-158`); the Donate arm (`fold.rs:1004-1113`) already computes a local
  `deduction_proxy = Σ(leg.term==LongTerm ? fmv_at_transfer : basis)` over the FINAL legs (after
  `make_removal_legs` + `carry.rehome_onto_removal_leg`), used ONLY to fire `QualifiedAppraisalNote`
  (>`QUALIFIED_APPRAISAL_THRESHOLD` $5k, tables.rs) then dropped (`fold.rs:1067-1104`).
- **For the modeled universe (non-dealer individual investor, capital assets) that sum IS the exact
  §170(e) deduction** — `term` is the only determinant; no dealer/inventory or self-created signal exists
  (`resolve.rs:696` "capital-asset eligibility assumed for a personal investor; no Phase-1 dealer flag").
- No deduction amount is stored or surfaced today: not on `Removal`, not in `render_removal_leg`
  (`render.rs:280-288`), not in `removals.csv` (`render.rs:566-595`), no charitable-deduction total in
  `render_report`. `compute_tax_year` never reads `state.removals`; `TaxProfile.ordinary_taxable_income`
  is post-deduction (`types.rs:31-50`, `tax_tables.rs:20-21`) → the §170 deduction is standalone.

## Design

### D1 — store the exact deduction on `Removal`
Add `pub claimed_deduction: Option<Usd>` to `Removal` (`state.rs`): `Some(amount)` for
`RemovalKind::Donation`, `None` for `RemovalKind::Gift` (a gift is not a charitable deduction). Compute
`amount = Σ over final legs ( leg.term == Term::LongTerm ? leg.fmv_at_transfer : leg.fmv_at_transfer.min(leg.basis) )`
in the Donate arm at the point the local `deduction_proxy` is computed today, and set it on the `Removal`
before `st.removals.push`. **[R0-C1] the ST branch is `min(fmv, basis)`, not `basis`** — this preserves
`= basis` for appreciated ST legs (the previous proxy behavior) and correctly yields `fmv` for a
depreciated ST leg. **[R0-m1] There are NO `Removal { .. }` test literals** — the only construction sites
are the two production pushes; the compile-forcing one is the **Gift arm** (`fold.rs:994`), which must set
`claimed_deduction: None`. Update both production pushes (Donation → the amount; Gift → `None`).

### D2 — drive the appraisal trigger off the exact amount (retire "proxy")
Replace the local `deduction_proxy` with the stored `claimed_deduction` value; `QualifiedAppraisalNote`
fires when `claimed_deduction > QUALIFIED_APPRAISAL_THRESHOLD`. Update the advisory detail: it is the
**claimed deduction $X** (exact for a non-dealer individual investor donating a capital asset to a public
charity), NOT an "estimated proxy". **[R0-C1] Note the ST-depreciated behavior change:** the trigger now
uses `min(fmv,basis)` for ST legs, so a ST *depreciated* donation is scored at FMV (lower) — this
correctly REMOVES the previous proxy's over-flag for that case (no existing appraisal KAT exercises ST
depreciation, so no current KAT changes; add one that locks it). **KEEP** the caveats (all real for
unmodeled cases): (a) **dealer/inventory** — crypto held as inventory/for sale in a trade or business
(§1221(a)(1)) is ordinary-income-character and deducts at basis regardless of holding period; this figure
assumes capital-asset (investor) status and would OVER-state for a dealer — not modeled; verify; (b)
**[R0-I1] donee type** — LT→FMV assumes a **public charity**; a non-operating **private foundation**
reduces appreciated LT crypto to **basis** (§170(e)(1)(B)(ii); crypto is not qualified appreciated stock)
— donee type not modeled; would OVER-state for a private-foundation gift; verify; (c) **§170(f)(11)(F)
aggregation** — per-donation only; the $5k appraisal test aggregates similar items across the year.

### D3 — surface the deduction
- `render_removal_leg` / the donation render (`render.rs:280-288`): for a Donation, show the removal's
  `claimed_deduction` (once per donation, e.g. on the donation header line — not per leg, since it's a
  per-donation total). Gifts show nothing (None).
- `removals.csv` (`render.rs:566-595`): add a `claimed_deduction` column (empty for gifts).
- `render_report` (`render.rs`): add a **per-year charitable-deduction total** = Σ `claimed_deduction`
  over `state.removals` where `kind==Donation` and `removed_at.year()==year`, using the same year-filter
  pattern as disposals/income. **[R0-I2]** Label it: "charitable deduction (Schedule A itemized) — BEFORE
  §170(b) AGI limits / carryover" (informational; not part of the crypto-attributable tax). The
  `removals.csv` `claimed_deduction` column is likewise the pre-§170(b)-limit amount (note in the header
  doc / a column comment where the codebase documents CSV columns).

### Decisions
- **Standalone, does NOT feed B.** `compute_tax_year` is unchanged; `TaxProfile.ordinary_taxable_income`
  remains user-supplied post-deduction. The §170 figure is reported for the user's Schedule A / Form 8283,
  not fed into the capital-gains computation. (Document; do not wire into B.)
- **Exact for the modeled universe; caveats retained for unmodeled character.** Renaming proxy→exact is
  honest ONLY because dealer/inventory/self-created are out of scope — the caveat makes that explicit.

## Plan (TDD)

### Task 1 — `Removal.claimed_deduction` + compute + drive the trigger off it
- **Files:** `crates/btctax-core/src/state.rs` (field + `Removal` literals), `crates/btctax-core/src/project/fold.rs` (Donate arm), tests.
- Add the field; compute + store it in the Donate arm (the existing sum, now persisted on `Removal`);
  Gift → `None`. Replace the local `deduction_proxy` usage with `removal.claimed_deduction` for the
  `QualifiedAppraisalNote` comparison + reframe the detail per D2 (exact + retained caveats). Update
  `Removal` literals.
- KATs: (a) a Donation's `claimed_deduction == Some(Σ ( LT→fmv ; ST→min(fmv,basis) ))` — cover LT-only,
  ST-appreciated (→basis), **ST-DEPRECIATED (basis>fmv → deduction = fmv, NOT basis) [R0-C1 lock]**, and
  mixed; (b) a Gift's `claimed_deduction == None`; (c) the appraisal trigger still fires correctly off the
  stored amount (LT $60k → flagged; ST $10k/$2k appreciated → not; boundary $5000.00 not / $5000.01
  flagged — adapt the existing appraisal KATs to the stored field, do not weaken them) **plus a
  ST-depreciated lock: basis $8k / fmv $3k → deduction $3k → trigger does NOT fire (the old proxy's
  `basis`=$8k would have)**; (d) the detail text is exact ("claimed deduction") + retains the
  dealer/inventory + **donee-type (private foundation)** + aggregation caveats + CCA 202302012.

### Task 2 — surface (render leg/header, CSV column, per-year total)
- **Files:** `crates/btctax-cli/src/render.rs`; CLI tests (`verify_report.rs` / `export`).
- Donation render shows `claimed_deduction`; `removals.csv` gains a `claimed_deduction` column (empty for
  gifts); `render_report` shows the per-year charitable-deduction total (Schedule-A itemized label).
- KATs: CSV `claimed_deduction` column present + correct value for a donation, empty for a gift; the
  per-year total sums exactly the year's donations (two donations in a year → their sum; a prior-year
  donation excluded); **the report total line carries the "before §170(b) AGI limits / carryover"
  qualifier [R0-I2]**.

### Task 3 — whole-diff review (Phase E) + FOLLOWUPS
- Cross-cutting: `claimed_deduction` exact for the modeled universe (non-dealer investor, capital asset,
  public charity, incl. depreciated property via `min(fmv,basis)`); the trigger drives off it and now
  **differs from the old proxy ONLY for ST-depreciated donations** (correctly, fewer false flags — NOT a
  regression); NO change to B / capital-gains tax / removal-basis math; determinism; no float; privacy.
  Confirm the three caveats (dealer, donee-type, aggregation) correctly bound the unmodeled cases and the
  report/CSV carry the "before §170(b) AGI limits" qualifier.
- FOLLOWUPS (OPEN, later Phase-2): precise ordinary-income-character detection (dealer/inventory
  §1221(a)(1), self-created); **donee-type modeling (public charity vs private foundation, §170(e)(1)(B))**;
  **§170(b) AGI percentage limits + 5-yr carryover + OBBBA-2026 0.5% floor / 35% cap**; §170(f)(11)(F)
  aggregation. Note the deduction is standalone (not fed to B) — if a future sub-project auto-reduces
  taxable income by itemized deductions, that is a separate change (double-count trap).

## Out of scope
- **Dealer/inventory + self-created ordinary-income-character detection** (would deduct at basis even
  LT) — unmodeled; the retained caveat discloses the over-state risk for a dealer.
- **Donee-type modeling (§170(e)(1)(B)) — public charity vs private foundation** (a private-foundation
  gift of appreciated LT crypto reduces to basis); unmodeled; retained donee-type caveat discloses it.
- **§170(b) AGI percentage limits (30%/20%/60%), 5-yr carryover, OBBBA-2026 0.5% floor / 35% cap** — the
  surfaced figure is the claimed deduction BEFORE these; computing the limited/allowed amount is deferred.
- Feeding the deduction into B / auto-reducing `ordinary_taxable_income`.
- Form 8283 / Schedule A PDF generation (later Phase-2 sub-project); §170(f)(11)(F) aggregation;
  2026/2027 tax tables.
