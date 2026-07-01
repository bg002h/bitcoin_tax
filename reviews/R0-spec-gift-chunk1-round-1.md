# R0 architect review — SPEC_gift_chunk1_aggregation (round 1)

- **Artifact:** `design/SPEC_gift_chunk1_aggregation.md`
- **Baseline verified against:** HEAD `191ad58` (confirmed `git rev-parse HEAD`).
- **Gate:** mandatory R0, 0 Critical / 0 Important required before implementation.
- **Verdict: NOT GREEN — 1 Critical, 3 Important, 5 Minor, 2 Nit.** Legal grounding is
  sound and independently web-confirmed; the block is an implementation-feasibility
  contradiction in D3 plus TDD/boundary-coverage gaps.

---

## Independent web-verification of the legal claims (did NOT trust the spec)

All four legal pillars are CONFIRMED against primary/authoritative sources
(Cornell LII / IRC §170; e-CFR §1.170A-16 & §1.170A-13; IRS CCA 202302012 PDF;
Journal of Accountancy; Form 8283 instructions 12/2025).

1. **§170(f)(11)(F) aggregation — CONFIRMED.** "For purposes of determining thresholds
   under this paragraph, property and all similar items of property donated to 1 or more
   donees shall be treated as 1 property." The donor "must aggregate the amount claimed as
   a deduction for all similar items of property (as defined in §1.170A-13(c)(7)(iii))
   contributed during the taxable year." Aggregation is across the year, to one OR multiple
   donees. "Similar items of property" = same generic category/type (coins, stamps,
   non-publicly-traded stock, etc.) → all BTC is one generic type → all BTC donations
   aggregate. Also confirmed: only ONE qualified appraisal is needed for the group of
   similar items in the year. The spec's D1 reading is correct.

2. **§170(f)(11)(C) $5,000 boundary — CONFIRMED strictly greater-than.** Statute: "in the
   case of contributions of property for which a deduction of **more than** $5,000 is
   claimed." "More than" = `>`, not `>=`. The code boundary `> QUALIFIED_APPRAISAL_THRESHOLD`
   (forms.rs:308, fold.rs:1111) is correct. Note the parallel $500 filing floor is
   likewise "more than $500" (render.rs `<= $500 → not required` is consistent).

3. **CCA 202302012 — CONFIRMED.** Crypto donated with a claimed deduction exceeding $5,000
   requires a qualified appraisal; the readily-valued exception (cash, publicly-traded
   securities, inventory, etc.) does NOT apply because a digital asset is not a "security"
   under §165(g)(2) (limited to corporate stock / evidence of corporate/gov indebtedness).
   The exchange-reported value does not substitute; reasonable-cause relief was denied.

4. **Aggregation basis = CLAIMED DEDUCTION — CONFIRMED.** The reg aggregates "the amount
   claimed as a deduction," and §170(f)(11)(C) keys off "a deduction of more than $5,000
   is claimed" — not FMV, proceeds, or basis. Summing `Removal.claimed_deduction` (the
   §170(e) LT→FMV / ST→min(FMV,basis) figure already stored) is the correct quantity.
   For ST lots the deduction (min(FMV,basis)) can be below FMV, and the $5k test correctly
   runs on the reduced deduction.

**Bottom line on law:** the spec's legal grounding (D1 rule, set = Donations only, quantity
= claimed_deduction, `>` boundary, uniform all-BTC-similar A/B, CCA-202302012 crypto
appraisal) is CORRECT. The blocking findings are engineering-feasibility, not tax-law.

---

## Recon citation verification vs HEAD 191ad58

| Spec claim | Source | Status |
|---|---|---|
| `form_8283` at forms.rs ~299-342, one row per Donation leg | forms.rs:299-342 | OK |
| Section A/B decided PER-DONATION via `> QUALIFIED_APPRAISAL_THRESHOLD` | forms.rs:306-312 (spec said 306-311) | OK, minor line drift |
| `Form8283Row.fmv_method: String` always `String::new()` | forms.rs:277 (spec said ~270) | OK, minor line drift |
| `QUALIFIED_APPRAISAL_THRESHOLD = dec!(5000)` | tables.rs:172 | OK exact |
| `Removal.claimed_deduction: Option<Usd>`, `Some` for Donation / `None` for Gift | state.rs:175; fold.rs:1146 | OK |
| `RemovalKind { Gift, Donation }` | state.rs:144-147 | OK |
| `FORM_8283_AGGREGATION_CAVEAT` at render.rs:753-755 (disclaims aggregation) | render.rs:749-755 | OK |
| per-donation `QualifiedAppraisalNote` blocker `claimed_deduction > $5k` | fold.rs:1111-1138 | OK; message already carries CCA 202302012 + the "cross-donation aggregation is not considered here" caveat |
| Standalone (does NOT feed engine B / `compute_tax_year`) | confirmed — `form_8283`/`write_form8283_csv` read only `state.removals` | OK |
| **`FmvStatus` variants `PriceDataset`/`ExchangeProvided`/`UserProvided`/`Unpriced`** | event.rs:9-14 → actual = `ExchangeProvided, PriceDataset, ManualEntry, Missing` | **DRIFT — `UserProvided` & `Unpriced` DO NOT EXIST** (see I1) |
| **`RemovalLeg` carries the leg `FmvStatus`** (D3 source) | state.rs:148-163 → fields are lot_id, sat, basis, fmv_at_transfer, term, `basis_source`, acquired_at | **FALSE — there is NO `fmv_status` on the leg** (see C1) |

---

## Findings

### CRITICAL

**C1 — D3 (FMV-method auto-fill) is not implementable under the spec's own "standalone"
constraint, and the fallback fabricates a Form 8283 field.**
D3 says "Populate `Form8283Row.fmv_method` from the row's price provenance (**leg
`FmvStatus`**)." There is no such field. `RemovalLeg` (state.rs:148-163) carries
`basis_source: BasisSource` but NO FMV provenance; `Removal` (state.rs:164-176) has none
either; `make_removal_legs` (fold.rs:219-258) is handed a single `total_fmv: Usd` with no
status and does not capture one. `form_8283` reads only `state.removals`, so the FMV
determination method for a Section-A row is simply not knowable from current data. Two
consequences, both blocking:
- To truly source it requires ADDING a field to `RemovalLeg` (state schema) AND populating
  it in `make_removal_legs`/the Donate path (fold) — exactly the event-schema + fold change
  the spec forbids (Design/Decisions, D2) and that Q8 asserts is unnecessary. So the spec is
  internally contradictory: it cannot deliver D3 while remaining "forms.rs + render.rs only."
- The only way to emit `"exchange spot price"` on Section-A rows WITHOUT that change is to
  hard-code it (or misuse `basis_source` — which is *basis* provenance, not *FMV*
  provenance; e.g. a `ComputedFromCost` lot can still have a dataset FMV). Blanket-labeling
  the Form 8283 "method used to determine the FMV" without knowing it FABRICATES a tax-form
  field, violating this codebase's explicit "honest gaps, never fabricated" contract
  (forms.rs:252) and mis-stating a form line. Because this changes what a tax form asserts,
  it is Critical.

*Fix (pick one):*
(a) **Descope D3 from this chunk.** Route FMV-method through a proper `RemovalLeg` FMV-
    provenance field with its own schema/fold spec + review; keep Chunk 1 to D1/D2/D4
    (which ARE standalone-feasible). Preferred — it keeps the "standalone, no fold change"
    claim honest.
(b) **Restrict D3 to what is derivable with zero fabrication:** Section B → `"qualified
    appraisal"` (derivable from the section alone — legitimate). Section A → leave
    `fmv_method` EMPTY (needs_review already `true`) or a non-asserting label; DROP the
    `"exchange spot price"` claim and any `FmvStatus` dependency entirely. Do NOT use
    `basis_source` as an FMV-method proxy.
Whichever is chosen, the "Section-A dataset-priced → 'exchange spot price'" KAT (Plan
Task 1) must be removed/rewritten — as specified it is un-writable.

### IMPORTANT

**I1 — `FmvStatus` variant-name drift.** Spec (lines 45-46, 72-74) names `UserProvided`
and `Unpriced`; the real variants (event.rs:9-14) are `ManualEntry` and `Missing`
(`PriceDataset`/`ExchangeProvided` are correct). As written any mapping would fail to
compile / match the wrong set. Subsumed if C1(a)/(b) drops the `FmvStatus` dependency, but
must be corrected wherever `FmvStatus` is referenced; a spec that asserts variant names for
the reader to rely on must state them correctly.

**I2 — Missing exact-$5,000-boundary KAT (the load-bearing `>` vs `>=` line).** The plan
has "aggregate ≤ $5k → A" but no test at exactly $5,000, which is the single case that
distinguishes `>` from `>=` — the boundary the gate calls Critical-if-wrong. Add:
aggregate = exactly $5,000 (e.g. $2,500 + $2,500) → all `Section::A` (strict `>`).

**I3 — Missing "Gifts excluded from the aggregate" KAT (correct-set guard).** D1 correctly
sums only `RemovalKind::Donation`, but nothing locks it. Aggregating the wrong set is
Critical-if-wrong, so it deserves a regression test. Add: a year with a Gift removal of
$10,000 + a Donation of $3,000 → the Donation is `Section::A` (the Gift does not pull it
into §170 aggregation; and Gifts still emit no Form 8283 rows).

### MINOR

**M1 — Doc-comment drift after D1.** `Form8283Row.section` doc (forms.rs:257-259) and the
`form_8283` fn doc (forms.rs:291) both state Section A/B is "driven by the donation's
`claimed_deduction` (> $5,000 → B)." After D1 it is the YEAR aggregate; the plan must update
both doc comments so the source doesn't misstate the rule.

**M2 — "applied to EVERY donation's row" is ambiguous vs the carrier-row convention.**
Section is currently emitted on the FIRST leg (smallest `lot_id`) only, `None` on
subsequent legs (forms.rs:313-324), a deliberate no-double-count convention. D1 must change
only the A/B VALUE (now uniform) while preserving carrier-row-only emission; spell this out
so the implementer doesn't start writing `section` on every leg.

**M3 — D2 advisory surface.** The year-aggregate advisory is render-synthesized (computed
from the sum), so it will NOT appear in `state.advisory` (the blocker-derived advisory list,
render.rs:468-472) that machine/JSON consumers read. If "alongside the existing gift/
appraisal advisories" is meant to include that list, note that a render-only line is a
different surface; acceptable given "no fold change," but state it explicitly.

**M4 — Reuse the existing aggregate sum.** `write_form8283_csv` already computes the exact
same `Σ claimed_deduction` for Donation removals in `year` (render.rs:777-782, the $500-floor
note). D1/D2 should share one helper so the CSV floor note, the Section A/B decision, and the
D2 advisory can never diverge on the aggregate.

**M5 — Line-number drift.** `fmv_method` is forms.rs:277 (spec "~270"); the per-donation
section test is forms.rs:306-312 (spec "306-311"). Cosmetic; correct in the plan.

### NIT

**N1 — `[R0-I1]` tag provenance.** The caveat is tagged `[R0-I1]` in both the code comment
(render.rs:749) and the CSV comment prefix (render.rs:772). D4 only needs to change the
const text; keep the tag as provenance (or note the re-tag) rather than silently dropping it.

**N2 — Consistency of the two appraisal messages.** After D4/D1, `form_8283`/the D2 advisory
will say aggregation IS applied, while the fold blocker message (fold.rs:1133-1135) still
says "cross-donation aggregation is not considered here." That is CORRECT for the fold's
per-donation scope and is intentionally out of scope, but the FOLLOWUPS note (already
planned) should call out the wording delta so a future reader isn't confused.

---

## Answers to the posed evaluation questions

- **Q1 §170(f)(11)(F) aggregation / all-BTC-similar:** confirmed (web + reg). Correct.
- **Q2 `>` $5k boundary:** confirmed strictly greater; `> QUALIFIED_APPRAISAL_THRESHOLD` correct.
- **Q3 CCA 202302012:** confirmed; no readily-valued/publicly-traded exception for crypto.
- **Q4 Aggregation basis = claimed deduction:** confirmed; `claimed_deduction` is right.
- **Q5 Correct set (Donations only, not Gifts):** D1 filter is correct; add I3 to lock it.
- **Q6 Uniform A/B for a BTC-only tool:** sound — all BTC is one generic type; §170(f)(11)(F)
  aggregation makes A/B a year-wide property of the similar-item class. No BTC-only case needs
  per-donation A/B (donee type / per-donee split affects Form 8283 sections V/appraiser, not
  the A-vs-B threshold determination, which the appraisal rule aggregates regardless of donee).
- **Q7 FmvStatus mapping matches event.rs:** NO — see C1 (no leg FmvStatus) + I1 (names).
- **Q8 Standalone / no-regression:** D1, D2, D4 are achievable in forms.rs + render.rs with the
  per-donation fold blocker left intact (no regression) — confirmed. **D3 is NOT** achievable
  standalone (C1).
- **Q9 Scope / TDD:** D1/D2/D4 right-sized and standalone. KATs for aggregate→B,
  under-aggregate→A, single-large→B (regression) are genuine. GAPS: the fmv_method KAT is
  un-writable (C1); missing exact-$5k boundary (I2) and Gift-exclusion (I3) KATs.

## Required to reach green (0C/0I)
1. Resolve C1 — descope D3 (preferred) or restrict it to non-fabricated labels with no
   `FmvStatus`/schema dependency; delete/rewrite the "exchange spot price" KAT.
2. Fix I1 (variant names) wherever `FmvStatus` survives.
3. Add I2 (exact-$5,000 → Section A) and I3 (Gift-excluded) KATs.
4. (Recommended before re-review) fold in M1-M5 / N1-N2.
Re-review after the fold (including the last), per the standard workflow.

---

# Round 2 — re-review

- **Artifact:** `design/SPEC_gift_chunk1_aggregation.md` (revised).
- **Baseline re-verified:** `git rev-parse HEAD` = `git rev-parse origin/main` =
  `191ad58a03d69ea65252dc8eba6d5390b933bbb7` — matches the spec's stated baseline. All
  round-1 citations re-checked against current source (not trusted from round 1).
- **Scope:** confirm the four folds (C1, I1, I2, I3) + the render-reuse Minor; check for
  NEW C/I and residual internal consistency. Legal grounding NOT re-litigated (round-1
  web-confirmed; unchanged).
- **Verdict: NOT YET GREEN — 1 Important remains (I1 not fully closed). C1/I2/I3 CLOSED, 0
  new C/I.** The remaining Important is a ~3-line documentation-hygiene fix; the substantive
  design is sound and implementation-ready once it is folded.

## Fold-by-fold

**C1 — CLOSED (adequate).** D3 (spec lines 72-84) now derives `fmv_method` only from the
section, with no fabrication and no schema/fold dependency:
- `Section::B` → `"qualified appraisal"`. Honest: Section B *is* the "qualified appraisal
  required" case, so appraisal genuinely is the FMV-determination method — derived from the
  section alone, no invented data.
- `Section::A` → EMPTY. Correctly refuses to fabricate "exchange spot price."
Verified against source: `RemovalLeg` (state.rs:148-163) carries only `basis_source`, no FMV
provenance — so D3's premise ("`fmv_method` CANNOT be sourced from price status without an
event-schema/fold change") is factually true; and the "honest gaps, never fabricated"
contract is real (forms.rs:252). D3 is now genuinely standalone (forms.rs + render.rs only),
resolving the round-1 internal contradiction. The un-writable "Section-A dataset-priced →
'exchange spot price'" KAT is gone; the replacement KATs (spec lines 116-117) test exactly
the two honest outcomes. Adequate.

**I1 — NOT CLOSED (residual survives; Important).** The *design* (D3) correctly drops the
`FmvStatus` dependency (spec line 84). But the fold left two stale `FmvStatus` references
that were the object of round-1 I1, and both now CONTRADICT D3:
- **Goal, line 9:** "FMV-method auto-fill for Form 8283 rows **(from `FmvStatus`)**,
  replacing the always-empty `fmv_method`." Directly contradicts D3 (section-derived, no
  `FmvStatus`). The spec's headline intent still states the abandoned mechanism.
- **Recon, lines 45-46:** "`FmvStatus` variants … `PriceDataset`/`ExchangeProvided`/
  **`UserProvided`/`Unpriced`** — the source for the fmv_method label." This (a) still carries
  the exact phantom variant names round-1 I1 flagged — real variants (event.rs:9-14) are
  `ExchangeProvided, PriceDataset, ManualEntry, Missing`; (b) contradicts D3; and (c) even
  misdescribes current state (today `fmv_method` is always `String::new()`, with no
  `FmvStatus` involvement at all).
Round-1 I1 required correction "wherever `FmvStatus` is referenced," and rated it Important;
the identical false citation surviving in two places (one of them the Goal) is an incompletely
folded Important, not a fresh Minor. Per CLAUDE.md ("verify citations against current source
at write time"), a phantom-variant citation is not Minor by this project's own bar; and eval
criterion #6 (internal consistency) fails while the Goal states the opposite of the Design on
the very field C1 concerned. **Fix (trivial):** delete "(from `FmvStatus`)" at line 9; delete
or rewrite lines 45-46 to state current `fmv_method` is always-empty with no `FmvStatus`
involvement (or, if kept as forward-looking, cite the real variants and mark it deferred).
The compile-risk that made round-1 I1 dangerous is gone (no code reads these), which is why
this is Important-doc, not Critical.

**I2 — CLOSED (adequate).** Spec line 112 adds the exact-$5,000-boundary KAT (aggregate
EXACTLY $5,000 → all `Section::A`), explicitly labeled "the case that distinguishes `>` from
`>=`." Matches source: forms.rs:308 uses `> QUALIFIED_APPRAISAL_THRESHOLD` (strict), and
§170(f)(11)(C) is "more than $5,000." The load-bearing boundary is now pinned.

**I3 — CLOSED (adequate).** Spec lines 113-115 add the Gift-exclusion KAT (Gift $10k +
Donation $3k same year → the Donation is `Section::A`; aggregate = the $3,000 Donation only).
Matches source: D1 (lines 52-54) sums only `Removal{Donation}`, and `claimed_deduction` is
`None` for a Gift (state.rs:172-175) — the wrong-set failure mode is now regression-locked.

**Render-reuse Minor (round-1 M4) — CLOSED (adequate).** D2 (spec lines 68-70) explicitly
reuses the sum at render.rs:777-782 rather than recomputing, and states the advisory is
render-time only (not in `state.advisory`). Verified render.rs:777-782 is exactly that
`Σ claimed_deduction` over `Donation` removals in `year`. Consistent with the standalone
pattern; single source of truth for the aggregate preserved.

## D1 / D2 / D4 soundness + new-finding sweep

- **D1** (year-aggregate `Σ claimed_deduction` over `Donation`, `> $5k` → uniform `Section::B`)
  is sound and matches the forms.rs structure. No new C/I.
- **D2** (render-time advisory, reuses the sum, not in `state.advisory`) is sound.
- **D4** (caveat → confirmation over render.rs:749-755) is sound; the const is real.
- **No NEW Critical/Important** introduced by any fold. The only blocker is the carried-over
  I1 residual above.

## Residual Minors (advisory — do not block, but fold with I1)

- **m1 (carryover of round-1 M2).** D1 line 56 still says "applied to EVERY donation's row this
  year (uniform)"; source emits `section` on the FIRST leg only (`is_first.then_some(section)`,
  forms.rs:322-324). Change only the A/B VALUE; preserve carrier-row-only emission. Spell this
  out so the implementer doesn't start writing `section` on every leg.
- **m2 (doc-comment drift from the D3 fold).** With `Section::B` → `"qualified appraisal"`,
  the field docs "`fmv_method` … always EMPTY" (forms.rs:252, 276-277) become inaccurate;
  extend the round-1 M1 doc-update task to cover them (`needs_review` stays `true`, so
  forms.rs:282-284 is fine). Also update the section-rule docs (forms.rs:257-259, 291) per M1.

## Bottom line

C1 + I2 + I3 CLOSED; render-reuse Minor folded; 0 new C/I; D1/D2/D4 sound and standalone
(no event-schema/fold/engine-B change). **I1 is NOT fully closed** — residual `FmvStatus`
references survive at spec lines 9 and 45-46 (line 46 still carries the phantom
`UserProvided`/`Unpriced` names and contradicts D3). That one Important keeps it short of
green. It is a ~3-line edit; fold it (plus the two advisory Minors), then a final re-review
should reach 0C/0I and clear the gate.

---

# Round 3 — re-review (focused)

- **Scope:** confirm I1 fully closed; verify carrier-row Minor (round-2 m1) and doc-update
  Minor (round-2 m2); check internal consistency. C1/I2/I3 not re-litigated (closed round 2);
  legal grounding not re-litigated (web-confirmed round 1).
- **Baseline:** unchanged — `191ad58` (spec header confirmed).
- **Verdict: GREEN — 0 Critical / 0 Important. Ready to implement.**

## I1 — CLOSED (fully)

Grep for `FmvStatus`, `UserProvided`, `Unpriced` across the whole spec yields three hits:

1. **Line 10 (Goal):** "no `FmvStatus` dependency; see D3" — the round-2 residual "(from
   `FmvStatus`)" is gone; Goal now says "honest, section-derived label … no `FmvStatus`
   dependency." Correct.
2. **Lines 46-49 (Recon):** "`RemovalLeg` carries NO FMV provenance … D3 derives it from the
   section only (no `FmvStatus` dependency). (`FmvStatus` on inbound events is
   `ExchangeProvided`/`PriceDataset`/`ManualEntry`/`Missing` per `event.rs`, but it does not
   reach `RemovalLeg`.)" — real variant names (no `UserProvided`/`Unpriced`); explicit
   disclaimer that FmvStatus does NOT reach RemovalLeg; no longer presents it as the
   fmv_method source. Correct.
3. **Line 88 (D3):** "No `FmvStatus` dependency (drops the R0-I1 variant-drift concern
   entirely)." — closing confirmation. Correct.

No remaining reference presents `FmvStatus` as the fmv_method source; no phantom variant
names survive. I1 fully closed.

## Carrier-row Minor (round-2 m1) — CLOSED

D1 lines 59-60: "applied to each donation's Section CARRIER row (the first leg, preserving
the existing `forms.rs:322-324` first-leg-only `section` emission convention — do NOT emit
`section` on every leg)." Explicit and correct.

## Doc-update Minor (round-2 m2) — CLOSED

Task 1 lines 108-109: "[R0-Minor] Also update the now-inaccurate doc comments (`forms.rs:252`
and `~276-277`) that say `fmv_method` is 'always EMPTY' — it's now 'qualified appraisal' for
Section B." Explicitly included in the implementation task.

## Internal consistency

Goal (section-derived, no FmvStatus) ↔ D3 (Section B → "qualified appraisal", Section A →
empty, no FmvStatus, derivable standalone) ↔ Recon (RemovalLeg carries no FMV provenance;
fmv_method cannot be sourced from price status; D3 section-only) — all three agree, no
contradiction.

## Final finding sweep

No new Critical or Important. D1/D2/D3/D4 all internally consistent and standalone-feasible.
KAT coverage complete (aggregate→B, under-aggregate→A, single-large→B regression,
exact-$5k→A, Gift-excluded). Carrier-row emission convention explicit. Reuse of
render.rs:777-782 sum stated. Advisory surface (render-only, not state.advisory) stated.

**0 Critical / 0 Important — R0 GREEN. Proceed to implementation (Task 1 TDD).**
