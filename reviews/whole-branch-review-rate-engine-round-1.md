# Whole-branch review — Sub-project B (rate / NIIT / loss-limit engine), round 1

Reviewer: independent final whole-diff reviewer (cross-cutting net over all 10 B tasks).
Diff: `.superpowers/sdd/review-ecc4c35..4e03429.diff` (12 commits).
Contract: `design/SPEC_lot_optimization_program.md` (Sub-project B + Legal grounding rate authorities + Cross-cutting); plan `design/IMPLEMENTATION_PLAN_rate_engine.md`.
Workspace gate: GREEN per prompt (321 tests, clippy -D clean, fmt clean, release builds) — not re-run; code/diff reviewed.

## VERDICT: READY TO MERGE — 0 Critical / 0 Important.

Found 0 Critical, 0 Important, 2 Minor (1 new, 1 disclosed/recorded), several Nits — all DEFER. No new blocking finding beyond what the per-task reviews already recorded. The deltas-vs-levels split is internally consistent end-to-end, the pinned identity holds by construction, the bundled TY2025 figures and statutory constants are correct, the refusal gate is sound, crate boundaries hold, and there is no float anywhere on the money/rate path.

---

## Cross-cutting verification (the 7 dimensions)

### 1. End-to-end tax correctness — PASS
Traced the full path in `tax/compute.rs::compute_tax_year`:
disposals/income → `disposed_at.year()` / `recognized_at.year()` filter → §1222 `net_1222` (+`carryforward_in` as own-character loss) → §1(h) `preferential_tax` (LT+QD stacked on the ordinary bottom, breakpoints vs total taxable) → §1411 NIIT (`min(NII, MAGI−threshold)`) → §1211/§1212(b) ST-first loss limit → two-scenario incremental delta → `TaxResult`.

- **No dropped/double-counted value at any seam.** `carryforward_in` enters via `cf_short`/`cf_long`; crypto ordinary income is added to `bottom_with` **only** (KAT 11 pins the once-only add: delta 600.00, not the double-count 1352.50). QD appears in both scenarios and cancels except for the crypto-induced bracket shift (correct ceteris-paribus attribution; KAT 3 QD-push = 3830.00 verified by hand).
- **Identity `total == ord_delta + ltcg_tax + niit` holds by construction.** `total = (ord_with+pref_with+niit_with) − (ord_without+pref_without+niit_without)`, `ltcg_tax = pref_with−pref_without`, `niit = niit_with−niit_without`; so the residual `total − ltcg_tax − niit` is exactly `ord_with−ord_without`. `render_tax_outcome` recomputes that residual as the displayed "ordinary-rate (attributable)" line — self-consistent. KAT 12 (three-way nonzero) pins 1200+9000+570=10770 with all three nonzero.
- **Deltas vs levels are consistent and labeled.** Deltas: `ltcg_tax`, `niit`, `total`. Levels (WITH-crypto filing position): `st_net`, `lt_net`, `ordinary_from_crypto`, `loss_deduction`, `carryforward_out`. `carryforward_out` is correctly the WITH-scenario level (`with.st_carry/lt_carry`), so it can feed next year's `carryforward_in`. Render tags each "(level)"/"(attributable)"/"(delta)".
- **§1222 netting** (within-character → cross-net, surviving loss keeps its character) and **§1212(b)** ST-first $3k absorption verified against all 7 `net_tests` and the cross-net arms; re-derived by hand — correct (matches Pub 550 mechanics).
- **§1(h) stacking** `preferential_tax` re-derived for the 0→15, 15→20, all-0%, zero-pref, and bottom-above-max_fifteen cases — correct.
- **NIIT MAGI isolation:** `magi_with = magi_excluding_crypto + crypto_agi`, where `crypto_agi` subtracts the without-scenario cap components so the non-crypto cap gain already inside `magi_excluding_crypto` is never double-counted. Verified.

### 2. Tax figures — PASS (spot-verified vs primary)
`btctax-adapters/src/tax_tables.rs` TY2025 from Rev. Proc. 2024-40:
- Ordinary 37% thresholds: Single/MFS-lower $626,350-equiv, MFJ $751,600, MFS $375,800, HoH $626,350 — all correct; HoH 35% at $250,500, Single/MFS 35% at $250,525 correct.
- §1(h) breakpoints: Single 48,350 / 533,400; MFJ 96,700 / 600,050; HoH 64,750 / 566,700; MFS 48,350 / 300,000 — all correct.
- Statutory constants in `tax/tables.rs`: NIIT 3.8%; thresholds 250k MFJ/QSS, 200k Single/HoH, 125k MFS; loss limit 3000 / 1500 MFS — all correct, year-independent, never placed in a `TaxTable` (I4 separation honored). Qss→Mfj aliasing at lookup. Nothing drifted from the per-task verification.

### 3. Refusal gating — PASS
`first_hard_blocker` scans **all** `state.blockers` for `severity()==Hard` (state.rs:57-75 classifier; 13 Hard kinds incl. `ImportConflict`, the 3 new A-blockers, and B's own three). Any Hard blocker anywhere → `TaxYearNotComputable` with the structured offending `EventId` carried (deterministic: `.find` returns first in projection order). Precedence: Hard → TaxTableMissing → TaxProfileMissing (tested incl. the precedence KAT). Cross-year basis contamination via an out-of-year `ImportConflict` is explicitly covered by `refuses_year_with_out_of_year_import_conflict_on_consumed_lot` (this also closes the recorded B-R2-N2). Advisory blockers do not gate; `carryforward_consistency` returns a `String` (never a `Blocker`) and `report_tax_year` keeps it strictly separate from the outcome and exit code (carryforward_mismatch_advisory_rendered: advisory fires even when the main outcome is `NotComputable(TaxTableMissing)`).

### 4. Crate placement / boundaries — PASS
Core (`tax/{types,tables,compute}.rs`) is pure: reads `state.disposals/income_recognized/blockers` + `&dyn TaxTables`; no I/O. `BundledTaxTables` (adapters) is in-memory `dec!` literals — no `include_str!`, no I/O on the compute path; mirrors `BundledPrices`. `tax_profile` is a CLI side-table (projection input, not ledger state); `report_tax_year` does vault I/O in the CLI then calls the pure core. Exports clean across `lib.rs` in all three crates.

### 5. NFR4 determinism + NFR5 exact + no float + federal-only + privacy — PASS
No `f32`/`f64`/`as f64` anywhere in `tax/`, adapters tax tables, or CLI tax code (grep clean; the only "float" hits are doc comments asserting *no* float). All money is `Decimal`, sats `i64`. `round_cents` = `round_dp_with_strategy(2, ROUND_HALF_EVEN)`, applied at the END only. Determinism: BTreeMap tables, Vec iteration in projection order, `.find` first-match — no HashMap iteration; `determinism_same_inputs_same_outcome` pins it. Federal-only (no state tax). Tests use temp vaults + synthetic CSV fixtures only.

### 6. Backward-compat — PASS
New `BlockerKind` variants are appended; `Blocker`/`BlockerKind` carry **no** serde derives and are never persisted (persistence.rs does not touch blockers) — projection-recomputed, so the enum extension cannot break any on-disk format. `TaxProfile` optional fields are `#[serde(default)]` (round-trip + minimal-JSON tests). The tax module is orthogonal to events/projection; B only *reads* `LedgerState`, so A's lot-id substrate is undisturbed.

### 7. Cross-task consistency / dead code / spec drift — see Minor/Nit below.

---

## Minor

- **B-NEW-M1 — `MarginalRates.niit_applies` doc-vs-code mismatch (display-only).** `types.rs` doc: "true iff MAGI (incl. crypto) exceeds the §1411 threshold." `compute.rs:360` actually sets `niit_applies = niit_with > niit_without` ("crypto *increased* NIIT"). These diverge when the taxpayer already pays NIIT without crypto and crypto adds none (e.g. crypto adds only ordinary income while NII is pinned by unchanged QD, MAGI over threshold both ways): code says `false`, the doc implies `true`. **DEFER** — `MarginalRates` is informational, not serde, and feeds **no** tax figure or the delta. The implemented semantics ("crypto increased NIIT") is arguably the more useful one for an incremental tool. Recommend a FOLLOWUP to align the doc to the code (or rename to `niit_increased`).

- **B-M1 (recorded) — NIIT minimal-NII model can understate NIIT.** NII = `QD + surviving net cap gains`; it does **not** subtract the allowed §1211 loss or net capital losses, and excludes crypto ordinary income. In a loss year the benefit is captured only through the MAGI channel, not the NII channel. **DEFER** — disclosed verbatim in `render_tax_outcome` ("MAY UNDERSTATE NIIT") and recorded as a Phase-2 refinement; within the spec's minimal-profile boundary (Resolved Q#1/Q#6).

## Recorded Minor/Nit triage (per prompt)

- **F1 — money "0" vs "0.00" display (Task 9 nit): DEFER — does NOT block a tax report.** The load-bearing figures — `ltcg_tax`, `niit`, and `total_federal_tax_attributable` — are all `round_cents`-scaled (scale 2) and always print with cents. Only the descriptive **level** fields (`st_net`, `lt_net`, `ordinary_from_crypto`, `loss_deduction`, `carryforward_out`) inherit the source Decimal scale and may print "20000" instead of "20000.00". No figure is wrong or ambiguous (a reader cannot misread "7000" as anything but $7,000); it is purely inconsistent styling. The integration test already accommodates it by asserting on stable prefixes. Recommend a FOLLOWUP `fmt_money {:.2}` helper for a polished, authoritative-looking report — but it is not a correctness gate.
- **B-R2-N2 (assert the gating blocker kind): CLOSED** by `refuses_year_with_out_of_year_import_conflict_on_consumed_lot` (asserts `ImportConflict` → `TaxYearNotComputable`).
- **Task 5 NIT — advisory-only→Computed KAT missing: DEFER.** Gate logic is provably correct (`first_hard_blocker` matches only `severity()==Hard`) and the severity() classifier is unit-tested; the explicit "advisory present yet Computed" KAT is a coverage nicety. Add in a sweep.
- **Task 5 NIT — `let _ = events;` unused param: DEFER.** Documented; the `events` slice is part of the planned C-facing signature / determinism tuple.
- **Task 6 NIT — redundant `rust_decimal_macros` dev-dep in adapters Cargo.toml: DEFER** (cosmetic; clippy clean).
- **Task 8 NIT — `tax-profile --show` prints `filing_status` via `{:?}`: DEFER** (cosmetic).
- **B-R2-N1 (stale plan §4.3 tax_date line), Task 2 doc-cite nit, Task 7 KAT7 comment overclaim, F2 (no CLI TaxTableMissing test — core-covered): DEFER** (doc/comment/coverage; no code impact).

## Notes (non-findings, recorded for the reader)
- `other_net_capital_gain` is modeled as **LT-character** (`net_1222(other_lt=...)`); this is the correct default since §1222(11) "net capital gain" is the net LT figure. No non-crypto-ST lever exists in the minimal profile — a documented boundary, not a defect.
- The refusal gate is intentionally **more** conservative than "disposals touched by a blocker": any Hard blocker anywhere blocks every year. This is the desired cross-year-contamination behavior (prompt dimension 3) and is documented at compute.rs:230-234.
