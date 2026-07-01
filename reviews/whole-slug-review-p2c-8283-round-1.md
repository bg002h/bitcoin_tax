# Whole-slug review — P2-C: Form 8283 generation + Form 709 gift advisory (round 1)

**Scope:** task-review + whole-diff gate for P2-C (2 commits, `f7159c5..3bee70d`).
**Artifacts:** `design/SPEC_p2c_form8283_709.md`, `.superpowers/sdd/p2c-report.md`, `review-f7159c5..3bee70d.diff`.
**Source verified against current tree:** `crates/btctax-core/src/{state.rs,project/fold.rs,forms.rs,tax/tables.rs}`,
`crates/btctax-adapters/src/tax_tables.rs`, `crates/btctax-cli/src/{render.rs,cmd/tax.rs,main.rs}`, KATs.
**Reviewer note:** author ≠ reviewer — findings persisted here verbatim for folding; I did NOT edit source.

---

## VERDICT: **NOT ready to merge** — 0 Critical / **1 Important** / 1 Minor / 2 Nit.

One Important, verifiable, and mechanically-fixable finding blocks the gate: a **wrong statutory
citation** (`§2.42` should be `§2.43`) for the gift annual exclusion, repeated across 5 code sites +
the spec. The dollar VALUE ($19,000) and all tax behavior are correct; the fix is value-neutral,
touches no test assertion, and requires no re-derivation. Everything else in the slug is correct,
honest, and standalone.

---

## Independent web-confirmation of $19,000 (with a citation correction)

Verified against the **primary source** — IRS Rev. Proc. 2024-40 (rp-24-40.pdf), extracted verbatim:

> **.43 Annual Exclusion for Gifts.** (1) For calendar year 2025, the first **$19,000** of gifts to
> any person (other than gifts of future interests in property) are not included in the total amount
> of taxable gifts under § 2503 made during that year.

- **VALUE $19,000 = CONFIRMED CORRECT** for the TY2025 §2503(b) per-donee annual exclusion (up from
  $18,000 in 2024). `BundledTaxTables` TY2025 `gift_annual_exclusion: dec!(19000)` is right, and the
  assertion `ty2025_gift_annual_exclusion_is_19000` pins it.
- **SECTION CITATION IS WRONG.** The code/spec cite **§2.42**. The authoritative Rev. Proc. 2024-40
  table of contents + body place "Annual Exclusion for Gifts (§§2503; 2523)" at **§2.43**. **§2.42 is
  "Valuation of Qualified Real Property in Decedent's Gross Estate" (§2032A)** — an unrelated
  estate-tax item. This is a factual, not judgment, error. (Also confirmed: §2.43(2) = the $190,000
  non-citizen-spouse exclusion — a future-interest/non-citizen FOLLOWUP already noted, R0-N3.)

---

## Findings

### [IMPORTANT] I1 — Wrong statutory citation for the gift annual exclusion (§2.42 → §2.43)

The gift annual exclusion is cited as **Rev. Proc. 2024-40 §2.42** in five code locations; the
authoritative source is **§2.43**. §2.42 is a different item (qualified-real-property valuation,
§2032A). The value ($19,000) is correct; only the section pointer is wrong.

Sites (current tree):
- `crates/btctax-adapters/src/tax_tables.rs:16` — module header `//! - **Rev. Proc. 2024-40 §2.42** …`
- `crates/btctax-adapters/src/tax_tables.rs:180` — the **user-visible `source` provenance string**:
  `"Rev. Proc. 2024-40 §2.01/§2.03 + §2.42 (TY2025); …"`
- `crates/btctax-adapters/src/tax_tables.rs:184` — field-population comment
- `crates/btctax-adapters/src/tax_tables.rs:277` — test doc comment
- `crates/btctax-core/src/tax/tables.rs:63` — `gift_annual_exclusion` doc comment
- (also `design/SPEC_p2c_form8283_709.md:86,125` — the spec seeded the wrong number; the impl faithfully
  followed it.)

Why Important, not Minor:
- The `source` field (line 180) is **shipped, user-/auditor-visible provenance** on the TaxTable; an
  auditor who follows §2.42 lands on estate real-property valuation, not the gift exclusion.
- `CLAUDE.md`/STANDARD_WORKFLOW mandates **"Verify citations against current source at write time."**
  That gate was not met — the miss is systematic (5 sites), not a typo.
- The whole stated purpose of `tax_tables.rs` is "values encoded verbatim from Rev. Proc. 2024-40 §X";
  a wrong §X defeats that contract in a filing tool.

Fix: mechanical `§2.42 → §2.43` at the 5 code sites (+ spec/report). **Value-neutral; no test
assertion references the string** (`ty2025_gift_annual_exclusion_is_19000` asserts the value only), so
no golden moves and no re-derivation. After the fold, re-run the citation check and the gate is GREEN.

### [MINOR] M1 — Stale positional comment in an untouched removals.csv consumer

`crates/btctax-cli/tests/verify_report.rs:888` still reads
`// Header check: claimed_deduction must be the 9th column (index 8, 0-based).` After D1 inserted
`acquired_at`, `claimed_deduction` is now the **10th** column (index 9). The **test itself does not
break** — it locates the column dynamically via `headers.iter().position(|h| h == "claimed_deduction")`,
which is the correct header-named pattern — but the comment now misstates the layout. This file is not
in the diff; the column shift made its comment stale as a side effect. Correct the comment (or delete
the now-wrong index annotation).

### [NIT] N1 — form8283.csv comment lines require opt-in `#` handling by consumers

The [R0-I1] aggregation caveat and [R0-M1] $500 note are emitted as leading `#` lines *before* the CSV
header (written to the raw `File`, then wrapped by `csv::Writer` — ordering is correct). A strict CSV
parser without `comment(Some(b'#'))` will mis-read the first `#` line as a header/row. This is the
spec-mandated design ([R0-I1] "CSV header comment"), is documented in the fn doc, and the KATs read
with `.comment(Some(b'#'))`. Acceptable; flagged only so downstream tooling knows to enable comment
handling. (No action required.)

### [NIT] N2 — Blank `section` on subsequent legs of a multi-leg donation

The first-leg convention leaves `section` blank on non-carrier leg rows (mirrors `claimed_deduction`).
A reader filtering `section == "B"` correctly counts one; a reader expecting a section on every row
sees blanks. Documented + KAT-covered; consistent with the P2-A `removals.csv` convention. No action.

---

## Checklist verification (D1–D5)

### D1 — `RemovalLeg.acquired_at` — PASS
- `make_removal_legs` (`fold.rs:246-258`) sets `acquired_at: c.gain_hp_start` and
  `term: term_for(c.gain_hp_start, removed)` — **the exact same HP-start argument**, so acquired_at can
  never contradict term.
- **Confirmed NO loss-zone branch:** `make_removal_legs` is a flat loop with pro-rata FMV allocation;
  it has none of the §1015 gain/NoGainNoLoss zoning that `make_disposal_legs` (`fold.rs:~145-199`) has.
  Removals recognize no gain/loss (TP10) ⇒ acquired_at is always `gain_hp_start`.
- KATs (`kat_tax.rs`): (a) ordinary purchased donation → acquired_at == purchase (HP-start), term LT;
  (b) gift-received-then-donated → acquired_at == §1223 tacked donor date (NOT gift date), term LT.
- `removals.csv` gains the `acquired_at` column between `term` and `claimed_deduction` (`render.rs`).

### D2 — Form 8283 (`form_8283` + `form8283.csv`) — PASS
- Section = **B iff** `claimed_deduction > QUALIFIED_APPRAISAL_THRESHOLD` (`= dec!(5000)`,
  `tables.rs:125`), else A — strict `>`, so exactly $5,000 → A (KAT: `dec!(5000)`→A, `dec!(5000.01)`→B).
  Ties to P2-A's `claimed_deduction` (same value that drives `QualifiedAppraisalNote`, `fold.rs:1111`).
- `how_acquired` [R0-N2]: Exchange/ComputedFromCost→Purchased; Gift{Carryover,FmvFallback}→Gift;
  **FmvAtIncome→Other**; Carried/SafeHarbor/Reconstructed→Review. Matches spec; KAT-covered exhaustively.
- `claimed_deduction` + `section` on the **FIRST leg only** (carrier = smallest `lot_id` via `min_by`,
  which returns the first minimum; deterministic sort places that row first). SUM invariant KAT passes
  (no double-count). Multi-leg export KAT confirms one deduction cell = $52,000.
- `donee`/`appraiser`/`fmv_method` always EMPTY; `needs_review` always `true` (Section B superset).
  **Never fabricated** — the honest-placeholder pattern is intact.
- `description` = exact-Decimal `btc_amount_description` (no f64) — NFR5 satisfied (matches form8949).
- [R0-I1] aggregation caveat + [R0-M1] ≤$500 note present as `#` comment lines; the multi-line caveat
  const uses `\`-continuations (no embedded newline) ⇒ a single `#` line — does not corrupt parsing.
  KATs read with `.comment(Some(b'#'))`. Gift(kind=Gift) → NO row (filter `kind==Donation`; KAT).
  Deterministic order `(removed_at, event, lot_id)`; year-filter (KAT excludes prior + future year).
  0o600 via `open_owner_only`; stable snake_case header (export KAT pins the exact column contract).

### D3 — Form 709 advisory + `gift_annual_exclusion` — PASS (value); see I1 (citation)
- `gift_annual_exclusion: Usd` added to `TaxTable`; BundledTaxTables TY2025 = `dec!(19000)`
  (web-verified correct — Rev. Proc. 2024-40 **§2.43**, see I1 for the citation defect).
- `render_gift_advisory`: over-exclusion → advisory with total + donee-not-modeled / total-exposure
  caveat; under → None; no gifts → None; **[R0-m6]** gifts present + no bundled table → `Some("…
  unavailable … Form 709 exposure not evaluated …")`, NOT None (KAT `gifts_present_but_no_table…`).
  Gift total = Σ `fmv_at_transfer` over `Removal{Gift}` (correct 709 basis; gifts carry no §170(e)
  deduction). Wired into `report_tax_year` (4-tuple) + `main.rs` (printed after `render_schedule_d`,
  non-gating). All `report_tax_year` callers updated.

### D4 — No unintended change — PASS
- Diff touches only: `tax_tables.rs`/`tables.rs` (additive `gift_annual_exclusion` field + citation),
  `forms.rs` (new Form8283 items), `state.rs`/`fold.rs` (additive `acquired_at`), `render.rs`
  (new fns), `cmd/tax.rs`+`main.rs` (advisory wiring), `lib.rs` (re-exports), + additive tests.
  **No `compute.rs` / `compute_tax_year` / capital-gains / FMV / basis change** — engine B does not
  read `gift_annual_exclusion` (new field; compute.rs not in the diff). P2-C is standalone.
- **No existing golden moved.** `tax_report.rs` edits are pure 3-tuple→4-tuple destructuring
  (`_gift`); the `1747.50` golden is untouched. Test `TaxTable` literals gained the field
  (compile-required, value-neutral — compiler enforces completeness, and the gate compiles). removals.csv
  column-index shift 8→9 handled: header-named readers safe; the one positional test (`export.rs`)
  updated to `DED_COL=9`; the other consumer (`verify_report.rs`) uses `position()` — safe (see M1).

### D5 — NFRs — PASS
- NFR4 determinism: `form_8283` total-order sort; advisory sum order-independent.
- NFR5 exact Decimal / no float: CSV values via `Decimal::to_string()` (consistent with
  form8949.csv/removals.csv); description exact. `fmt_money` used only on the display/advisory text.
- CSV 0o600 (`open_owner_only`); additive projection field (RemovalLeg is recomputed, not persisted;
  TaxTable is bundled, not user-serialized) ⇒ no serde migration. Privacy: synthetic-only fixtures
  (`bc1qsyntheticcharity`, hand-chosen amounts).

---

## FOLLOWUPS triage (BLOCK vs DEFER)

- **§170(f)(11)(F) similar-item YEAR-AGGREGATION for the Section A/B split** — **DEFER.** The
  per-donation split is spec-mandated; the aggregate-of-small-donations gap is honestly disclosed by
  the STANDING [R0-I1] caveat (in every form8283.csv) + always-true `needs_review`. No silent
  under-classification: the disclosure is unconditional. Legitimate follow-up.
- **Donee identifier on Donate/GiftOut → full Form 709 (per-donee exclusion + lifetime exemption) +
  Form 8283 donee/FMV-method fields** — **DEFER.** Event-schema change, its own sub-project; the gap
  is honestly flagged (blank donee/fmv_method + `needs_review=true`; 709 is a total-exposure advisory
  with an explicit "donee identity is not modeled" caveat). Honest-placeholder pattern.
- **Section B appraiser-info struct** — **DEFER.** Section B rows emit `needs_review=true` + blank
  appraiser; no fabrication. Follow-up.
- **Gift-exclusion tables for TY2024 / 2026+** — **DEFER.** [R0-m6] guarantees a note (not silence)
  for any year lacking a bundled table; correctly not silently skipped.
- **how_acquired origin-loss (Carried/SafeHarbor/Reconstructed → Review)** — **DEFER.** Mapped to
  "Review" (never guessed); honest.

None BLOCK: every deferred gap is disclosed via `needs_review` / blank columns / standing caveats —
the honest-disclosure bar for filing artifacts is met.

---

## Required before merge
1. **[I1]** Correct `§2.42` → `§2.43` at the 5 code sites (esp. the user-visible `source` string,
   `tax_tables.rs:180`) + the spec. Value-neutral; no test/golden change.
2. **[M1]** Fix the stale index comment in `verify_report.rs:888` (or drop the index annotation).
3. Re-run the citation verification + workspace gate after the fold; expect GREEN (0C/0I).
