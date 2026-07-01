# Whole-branch review — Charitable/gift Chunk 1 (§170(f)(11)(F) year-aggregation + Form 8283 FMV-method)

- **Artifacts under review:** `design/SPEC_gift_chunk1_aggregation.md` (contract),
  `.superpowers/sdd/gift-chunk1-report.md` (report),
  `.superpowers/sdd/review-191ad58..78f7033.diff` (2 commits).
- **Source verified against:** working tree @ `78f7033` (`git rev-parse HEAD` =
  `78f70339788211452f0ff354e76aebe81ea3cf31` — matches the diff's HEAD). All findings
  re-checked against current source, not trusted from the report.
- **Role:** task review AND final merge gate.
- **Verdict: GREEN — 0 Critical / 0 Important (2 Minor, 1 Nit). Chunk 1 is ready to merge.**

---

## Scope / method

Re-derived the six D1/D3 KATs by hand; verified the D2 advisory boundary + surface; verified
standalone/no-regression via `git diff --stat 191ad58..78f7033` (independently run); grepped the
changed code for float and for stale doc references. The full validation gate (590 tests, clippy
-D warnings, fmt, PII) was NOT re-run per instruction.

---

## 1. §170(f)(11)(F) aggregation (highest priority) — CORRECT

`form_8283` (forms.rs:317-379) computes the year aggregate ONCE, before the row loop
(forms.rs:320-325):

```
year_agg_deduction = state.removals.iter()
    .filter(|r| r.kind == RemovalKind::Donation && r.removed_at.year() == year)
    .filter_map(|r| r.claimed_deduction)
    .sum()
```

`section = if year_agg_deduction > QUALIFIED_APPRAISAL_THRESHOLD { B } else { A }`
(forms.rs:328-332), computed once and applied UNIFORMLY. `QUALIFIED_APPRAISAL_THRESHOLD =
dec!(5000)` (tables.rs:172, exact `Decimal`). The section is emitted on the carrier row only
via `is_first.then_some(section)` (forms.rs:358); carrier = smallest `lot_id` (forms.rs:349-354);
subsequent legs get `None`. The prior per-donation threshold test is fully removed.

**Correct set:** filtered to `kind == RemovalKind::Donation` (belt), and Gifts carry
`claimed_deduction == None` (fold.rs:1024 — suspenders), so `filter_map` drops them regardless.
Gifts cannot enter the aggregate by either path.

Hand re-derivation of every KAT (all reproduce):

| KAT | deductions | Σ | `> 5000`? | section | fmv_method |
|---|---|---|---|---|---|
| 1 aggregate→B | 2000+2000+2000 | 6000 | yes | **B** (all 3) | "qualified appraisal" |
| 2 under→A | 1000+1500 | 2500 | no | A | "" |
| 3 single-large (regression) | 8000 | 8000 | yes | B | "qualified appraisal" |
| 4 exact-$5,000 [R0-I2] | 3000+2000 | 5000 | **no** (`5000 > 5000` = false) | **A** | "" |
| 5 Gift-excluded [R0-I3] | Gift 10000 (excl) + Don 3000 | 3000 | no | **A** | "" |

Carrier-row convention preserved: only the smallest-`lot_id` leg carries `section`/
`claimed_deduction`/`fmv_method`; other legs carry `None`/`""` (KAT 6 locks this). No CSV SUM
double-count.

## 2. fmv_method honest (no fabrication) — CORRECT

`carrier_fmv_method` is derived from the section alone (forms.rs:335-338): `Section::B →
"qualified appraisal"`, `Section::A → String::new()`. Emitted on the carrier row only; `""` on
subsequent legs (forms.rs:366-370). No `FmvStatus` read anywhere; no "exchange spot price" or any
FMV-provenance fabrication (`RemovalLeg`, state.rs:149-163, carries only `basis_source` — no FMV
provenance). The three "always EMPTY" doc claims were updated: the enum doc (forms.rs:197-212),
the struct-level doc (forms.rs:254-258), the `fmv_method` field doc (forms.rs:280-283), and the
`form_8283` fn doc (forms.rs:305-309) all now describe the honest section-derived behavior.
`needs_review` remains `true` on every row (donee/appraiser still unmodeled). KAT 7 pins Section B
carrier → "qualified appraisal", Section A carrier → "".

## 3. D2 advisory — CORRECT

`render_donation_appraisal_advisory` (render.rs:792-805) calls the shared `year_donation_deduction`
helper (render.rs:768-775) — the same helper `write_form8283_csv` uses for the $500 floor note
(render.rs:828); no recompute on the render side. Boundary `if agg <= QUALIFIED_APPRAISAL_THRESHOLD
{ return None }` → fires only when `agg > $5,000` (strict `>`, matching the section), so the
advisory fires exactly when Section B; at exactly $5,000 no advisory (matches Section A).
Render-time only: threaded through `report_tax_year` (cmd/tax.rs) into the `TaxYearReport` tuple
and printed in main.rs as a non-gating line — it does NOT enter `state.advisory` / the blocker set
and does not touch the exit code. Message cites §170(f)(11)(F) + CCA 202302012 (render.rs:799-802).
D4: `FORM_8283_AGGREGATION_CAVEAT` (render.rs:754-758) is now a confirmation of the implemented
year-aggregate (not "unimplemented"); the `[R0-I1]` tag was dropped from the CSV header comment
(render.rs:822), consistent with the state being implemented; the export KAT was updated to assert
the new text.

## 4. Standalone / no regression (highest priority) — CORRECT

Independently ran `git diff --stat 191ad58..78f7033`: the 9 changed files are exactly
`cmd/tax.rs`, `main.rs`, `render.rs`, `tests/export.rs`, `tests/tax_report.rs`, `forms.rs`,
`tests/kat_forms.rs`, the SPEC, and the R0 review. **`compute_tax_year`/engine B (tax/mod.rs),
`fold.rs`, the event schema (event.rs), and `state.rs` are NOT in the changed set** — the
`total == ord_delta + ltcg + niit` identity and all tax figures are unchanged. The per-donation
`BlockerKind::QualifiedAppraisalNote` fold blocker is intact (fold.rs:1111-1138, unchanged) — no
regression; it is complemented, not replaced, by the render-time year-aggregate advisory. The
6-tuple `TaxYearReport` wiring is correct: the new `Option<String>` is appended, all 8 existing
destructuring sites in tests + main.rs were updated to the 6-tuple, and the existing report
elements are unchanged.

## 5. Exact Decimal / determinism / KATs not weakened — CORRECT

Grep of the changed source for `as f64|as f32|: f64|: f32|f64::|f32::` finds only NFR5 comments
that forbid float ("NEVER `sat as f64`", "no float") — zero float in logic. The aggregate is
`Usd` (`rust_decimal::Decimal`) summation; CSV values via `Decimal::to_string()` (render.rs:863-868);
the $500 floor uses `Decimal::from(500)`. Ordering unchanged (sort by `removed_at`, event id,
`lot_id` — forms.rs:378). The existing export/Form 8283 KATs were tightened, not weakened: the
export test now asserts the carrier row has `fmv_method == "qualified appraisal"` and the
non-carrier row `""` (previously asserted all blank), and the caveat assertion targets the new
`§170(f)(11)(F) year-aggregate` text.

---

## Findings

### CRITICAL — none.
### IMPORTANT — none.

### MINOR

**M1 — residual doc drift on the `section` FIELD doc.** `Form8283Row.section` (forms.rs:261-262)
still reads "Driven by the donation's `claimed_deduction` (> $5,000 → B)" — the pre-D1
per-donation rule. After D1 it is the YEAR aggregate. The enum doc, struct doc, `fmv_method` field
doc, and `form_8283` fn doc were all correctly updated; this one field-level doc was missed. This
is precisely the field-doc half of R0-M1 (the fn-doc half was fixed). Doc-only; behavior is
correct. Suggest: "Section A/B (uniform across the year) — on the FIRST leg row only; driven by the
§170(f)(11)(F) year-aggregate deduction (> $5,000 → B)."

**M2 — the year-aggregate sum is computed in two places.** `form_8283` computes
`year_agg_deduction` inline in `btctax-core` (forms.rs:320-325); `render.rs` has its own
`year_donation_deduction` helper in `btctax-cli` (render.rs:768-775). The two are byte-for-byte
identical logic against the same const, so the section and the advisory agree by construction
today — but they are a future divergence point across the crate boundary (a change to the
donation-set definition in one place would silently desync the advisory from the shown section).
The spec's reuse requirement (render-side) is satisfied; this is a DRY/maintainability note.
Suggest exposing a single `btctax-core` helper (e.g. next to `form_8283`) that both `form_8283`
and `render.rs` call. Non-blocking.

### NIT

**N1 — money-format cosmetics in the advisory.** The advisory renders the aggregate via
`fmt_money` (e.g. `$6000.00`) then cites the threshold as `$5,000` (render.rs:799-803) — mixed
"6000.00" vs "5,000" styling. Both are unambiguous. Cosmetic only.

---

## Re-derivation of the two spotlighted cases

**Exact-$5,000 → A** (`form8283_exact_5000_aggregate_is_section_a_not_b`): two Donations $3,000 +
$2,000, same year → `year_agg_deduction = 3000 + 2000 = 5000`. `5000 > QUALIFIED_APPRAISAL_
THRESHOLD(5000)` is **false** (5000 is not more than 5000) → `Section::A` on both carrier rows;
`fmv_method == ""`; the D2 advisory returns `None` (`5000 <= 5000`). Had the operator been `>=`,
this would flip to B — the test is the exact `>` vs `>=` discriminator, and `>` matches
§170(f)(11)(C) "more than $5,000." CONFIRMED.

**Gift-exclusion → A** (`form8283_gift_fmv_excluded_from_donation_aggregate`): a Gift with
`fmv_at_transfer = $10,000` (kind Gift, `claimed_deduction == None`, fold.rs:1024) + a Donation
with `claimed_deduction = $3,000` (kind Donation). The aggregate filters `kind == Donation`,
excluding the Gift entirely; `filter_map(claimed_deduction)` would drop the Gift's `None` anyway
(double guard) → `Σ = 3000`. `3000 > 5000` is false → `Section::A`, `fmv_method == ""`. The
row loop likewise filters `kind == Donation`, so the Gift produces NO Form 8283 row →
`rows.len() == 1`, `rows[0].section == Some(A)`. If Gifts were wrongly aggregated (e.g. via the
$10,000 fmv), the aggregate would be $13,000 → Section B — the test locks A, guarding against
aggregating the wrong set. CONFIRMED.

---

## Bottom line

The aggregation math, the honest section-derived `fmv_method`, the render-time D2 advisory + D4
caveat confirmation, and the standalone/no-regression posture are all correct and match the spec
contract. Boundary (`>` at exactly $5,000 → A) and correct-set (Gifts excluded, double-guarded)
are regression-locked. Engine B / fold / event / state are provably untouched (diff --stat). The
only findings are 2 Minor (a stale field doc; a cross-crate duplicated sum) and 1 Nit — none
blocking. **0 Critical / 0 Important — GREEN. Chunk 1 is ready to merge.**
