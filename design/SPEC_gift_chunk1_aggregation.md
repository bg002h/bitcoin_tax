# SPEC — Charitable/gift Chunk 1: §170(f)(11)(F) year-aggregation + Form 8283 FMV-method

**Source baseline:** `origin/main` @ `191ad58`.
**Goal:** Two standalone Form 8283 correctness improvements, no event-schema change:
1. **§170(f)(11)(F) similar-item year-aggregation** — decide Form 8283 Section A vs B (and the qualified-
   appraisal requirement) on the YEAR's aggregate deduction for similar property (all BTC is "similar
   property"), not per-donation; surface a year-aggregate appraisal advisory when the aggregate crosses
   $5k even if no single donation does.
2. **FMV-method: honest, section-derived label** for Form 8283 rows (Section B → "qualified appraisal";
   Section A → empty — no fabrication, no `FmvStatus` dependency; see D3), replacing the always-empty
   `fmv_method`.

First of three chunks in the charitable/gift completion cluster (Chunk 2 = donee identifier + per-donee
Form 709; Chunk 3 = §2505 advisory + Section-B appraiser struct). **Standalone** — does NOT feed
`compute_tax_year` / engine B (the established Phase-2 pattern).

**SemVer:** additive/behavioral change to the standalone `form_8283` Section A/B determination + a new
advisory + a populated field ⇒ **MINOR** (pre-1.0). No struct/API removal; `Form8283Row.fmv_method`
already exists (was empty).

## Legal grounding (R0 to web-verify)
- **§170(f)(11)(C):** a qualified appraisal is required for donated property when the claimed deduction
  exceeds **$5,000**.
- **§170(f)(11)(F):** for purposes of the dollar thresholds, **all "similar items of property" donated
  during the tax year are AGGREGATED** (whether to one or multiple donees). Treas. Reg.
  §1.170A-13(c)(7)(iii) / §1.170A-16(d)(1): "similar items of property" = property of the same generic
  category or type (coins, stamps, land, publicly-traded vs. non-publicly-traded stock, etc.). **All BTC
  donations are similar property** → they aggregate for the $5,000 test.
- **CCA 202302012:** cryptocurrency donated with a claimed deduction > $5,000 requires a qualified
  appraisal; the "readily-valued" exception (which exempts cash and publicly-traded securities) does NOT
  apply to crypto. So a year-aggregate BTC deduction > $5,000 → qualified appraisal required for the
  donation(s) → Form 8283 **Section B**.

## Current-state (recon @ 191ad58)
- `forms.rs:form_8283(state, year) -> Vec<Form8283Row>` (~299-342): builds one row per Donation
  `RemovalLeg`; **Section A/B is decided PER-DONATION** (`forms.rs:306-311`) by comparing that donation's
  `claimed_deduction` against `QUALIFIED_APPRAISAL_THRESHOLD` (`tables.rs:172`, `dec!(5000)`). Misses the
  cross-donation aggregate (e.g. 3 × $2k donations = $6k → all should be Section B; currently all Section A).
- `Form8283Row.fmv_method: String` (`forms.rs:~270`) is **always `String::new()`** (unmodeled). `donee`
  / `appraiser` also empty (Chunks 2/3). `needs_review` always `true`.
- `FORM_8283_AGGREGATION_CAVEAT` (`render.rs:753-755`) currently DISCLAIMS the missing aggregation — it
  should become a confirmation/explanation once implemented.
- The fold's per-donation `BlockerKind::QualifiedAppraisalNote` (`fold.rs:1111`, `claimed_deduction >
  $5k`) stays as a per-donation signal — but it too misses the year-aggregate case (a small-donations
  year whose aggregate > $5k gets no appraisal signal today).
- `RemovalLeg` (`state.rs:148-163`) carries NO FMV provenance — only `basis_source` (basis provenance,
  NOT FMV provenance). So fmv_method CANNOT be sourced from price status here; D3 derives it from the
  section only (no `FmvStatus` dependency). (`FmvStatus` on inbound events is `ExchangeProvided`/
  `PriceDataset`/`ManualEntry`/`Missing` per `event.rs`, but it does not reach `RemovalLeg`.)
- Standalone confirmed (`forms.rs:192-195`; does NOT feed engine B).

## Design

### D1 — §170(f)(11)(F) year-aggregate Section A/B (in `form_8283`)
In `form_8283(state, year)`, first compute the **year aggregate** over ALL `Removal{Donation}` in `year`:
`year_agg_deduction = Σ removal.claimed_deduction` (the §170(e) claimed deduction already on each
`Removal`; sum the `Option<Usd>` treating `None` as 0). Then:
`section = if year_agg_deduction > QUALIFIED_APPRAISAL_THRESHOLD { Section::B } else { Section::A }`
applied to each donation's Section CARRIER row (the first leg, preserving the existing
`forms.rs:322-324` first-leg-only `section` emission convention — do NOT emit `section` on every leg) —
uniform across the year (all BTC is similar property). This REPLACES the per-donation threshold test. (Rationale: §170(f)(11)(F) aggregates similar property across the year; a
per-donation test under-triggers Section B.)

### D2 — year-aggregate appraisal advisory
Add a standalone year-level advisory (render-time, via the forms/tax report path — NOT a fold/schema
change) emitted when `year_agg_deduction > QUALIFIED_APPRAISAL_THRESHOLD`: e.g. "§170(f)(11)(F): your
{year} BTC donations aggregate ${agg} of claimed deduction (> $5,000) — a qualified appraisal is required
for the donated BTC even if no single donation exceeds $5,000 (all BTC is 'similar property'; CCA
202302012 — no readily-valued exception for crypto)." Surface it alongside the existing gift/appraisal
advisories in the tax report. (The per-donation `QualifiedAppraisalNote` fold blocker is left as-is; this
advisory is the additive year-aggregate signal.)
**[R0-Minor] Reuse the identical donation-deduction sum already computed at `render.rs:777-782`** rather
than recomputing it. This advisory is render-time only (does NOT enter `state.advisory`/the blocker set) —
consistent with the standalone-forms pattern; note that in the code comment.

### D3 — FMV-method: honest, section-derived ONLY (no fabrication) [R0-C1]
`RemovalLeg` (`state.rs:148-163`) carries NO FMV provenance — only `basis_source` (BASIS provenance ≠ FMV
provenance) — and `form_8283` reads only `state.removals` (`make_removal_legs` gets a bare `total_fmv`
with no status). So `fmv_method` CANNOT be sourced from price status without an event-schema/fold change,
which is OUT OF SCOPE for Chunk 1 (standalone forms.rs/render.rs only). Populate ONLY what is derivable
without fabrication:
- Section B rows → `fmv_method = "qualified appraisal"` — Section B ⇒ a qualified appraisal is required,
  so that IS the FMV-determination method; honest, derived purely from the section.
- Section A rows → leave `fmv_method` EMPTY. Do NOT fabricate "exchange spot price" — the FMV method isn't
  modeled, and forms.rs:252 mandates "honest gaps, never fabricated".
Keep `needs_review = true` (donee/appraiser still unmodeled until Chunks 2/3). A real per-row FMV-method
needs FMV provenance on `RemovalLeg` — deferred to a schema-touching chunk (e.g. Chunk 3's Section-B work).
No `FmvStatus` dependency (drops the R0-I1 variant-drift concern entirely).

### D4 — caveat → confirmation
Update `FORM_8283_AGGREGATION_CAVEAT` (`render.rs:753-755`) from "aggregation NOT implemented" to a
statement that Section A/B now reflects the §170(f)(11)(F) year-aggregate for similar property (BTC), with
the CCA 202302012 note.

### Decisions
- **All BTC = "similar property"** (single asset class) → the whole year's donations aggregate uniformly;
  no per-item-category partitioning needed (BTC-only tool).
- **Standalone** — forms.rs + render.rs only; NO event-schema, NO fold change, NO engine-B change. The
  §170(e) `claimed_deduction` per Removal is the input (already computed).
- Section A/B is now UNIFORM across the year's donations (all A or all B) — correct for a single similar-
  property class.

## Plan (TDD)

### Task 1 — §170(f)(11)(F) year-aggregate Section A/B + FMV-method + advisory
- **Files:** `crates/btctax-core/src/forms.rs` (form_8283 — year-aggregate section + fmv_method),
  `crates/btctax-cli/src/render.rs` (the caveat text + the year-aggregate advisory + form8283.csv already
  has the fmv_method column). **[R0-Minor] Also update the now-inaccurate doc comments** (`forms.rs:252`
  and `~276-277`) that say `fmv_method` is "always EMPTY" — it's now "qualified appraisal" for Section B.
- Implement D1 + D3 (both in form_8283). Hand-verified KATs (synthetic fixture `LedgerState` with Donation
  removals):
  - **Aggregation triggers B:** three donations $2,000 + $2,000 + $2,000 (each < $5k, aggregate $6,000 >
    $5k) → ALL rows `Section::B` (pre-change: all A). The core §170(f)(11)(F) lock.
  - **Under aggregate → A:** two donations $1,000 + $1,500 (aggregate $2,500 ≤ $5k) → all `Section::A`.
  - **Single large → B** (regression): one donation $8,000 → `Section::B` (unchanged).
  - **[R0-I2] Exact-$5,000 boundary → A:** donations aggregating EXACTLY $5,000 → all `Section::A` (the
    case that distinguishes `>` from `>=`; §170(f)(11)(C) is "more than $5,000").
  - **[R0-I3] Gift excluded from the aggregate:** a Gift removal $10,000 + a Donation removal $3,000 in
    the same year → the Donation row is `Section::A` (aggregate = the $3,000 Donation only; Gifts are
    §2503/Form 709, NOT §170/Form 8283 — they must NOT enter the donation aggregate).
  - **fmv_method (D3, honest):** a `Section::B` row → `fmv_method == "qualified appraisal"`; a `Section::A`
    row → `fmv_method` EMPTY (not fabricated).
- Implement D2 + D4 (render). KATs: the tax report shows the year-aggregate appraisal advisory when the
  aggregate > $5k (and NOT when ≤ $5k); the caveat text no longer says aggregation is unimplemented.

### Task 2 — whole-diff review (Phase E) + FOLLOWUPS
- Cross-cutting: §170(f)(11)(F) aggregate is correct (all-BTC-similar; year-scoped; the $5k boundary is
  `>` not `>=` per §170(f)(11)(C)); Section A/B uniform; fmv_method informative; the advisory fires at the
  aggregate boundary; STANDALONE (engine B / compute_tax_year / the total identity untouched); exact
  Decimal; determinism; the per-donation fold blocker still present (no regression). Confirm form8283.csv
  now carries fmv_method + the (still-empty until Chunk 2/3) donee/appraiser columns.
- FOLLOWUPS: note Chunk 2 (donee identifier + per-donee Form 709) and Chunk 3 (§2505 advisory + Section-B
  appraiser struct) as the next chunks; the per-donation vs year-aggregate fold-blocker duplication (the
  fold blocker could gain a year-aggregate companion — deferred).

## Out of scope
- The donee identifier / per-donee Form 709 (Chunk 2); §2505 lifetime exemption + Section-B appraiser
  struct (Chunk 3); any event-schema/fold change; partitioning "similar property" into sub-categories
  (BTC-only → one class); a year-aggregate fold blocker (render-time advisory suffices here); feeding any
  of this into engine B.
