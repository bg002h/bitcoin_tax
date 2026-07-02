# SPEC — TY2024 tax-table backfill

**Source baseline:** `main` @ `42ddab8`. The final queue item from SPEC_se_chunkB_expenses §Task 2
FOLLOWUPS ("next queue item = TY2024 tables backfill").

**Goal:** Add a fully-verified `ty2024()` builder to `crates/btctax-adapters/src/tax_tables.rs` and
register it in `BundledTaxTables::load()`, so that `report --tax-year 2024` produces a
`Computed` result rather than `TaxOutcome::NotComputable(TaxTableMissing)`.  No engine logic
changes.  No struct or trait changes.  Pure data addition.

**SemVer:** No public API, struct, or trait change — additive compiled-in data ⇒ **PATCH**.

---

## Legal grounding (R0 to re-verify against primary sources)

All dollar figures below were extracted directly from the official IRS PDF of **Rev. Proc. 2023-34**
(`irs.gov/pub/irs-drop/rp-23-34.pdf`, pdftotext, verified 2026-07-01) and from the SSA official
announcement.  Every figure carries its exact section cite; R0 must re-verify against the same
primary sources before signing off.

### Rev. Proc. 2023-34 section-numbering note

Rev. Proc. 2023-34 differs structurally from Rev. Proc. 2024-40 (TY2025):
- **Rev. Proc. 2024-40:** adjusted items are in **SECTION 2** → cited as `§2.01`, `§2.03`, `§2.41`,
  `§2.43`.
- **Rev. Proc. 2023-34:** SECTION 2 is "CHANGES" (Hazardous Substance Superfund rate); adjusted
  items are in **SECTION 3** → cited as `§3.01`, `§3.03`, `§3.41`, `§3.43`.

The implementer MUST use the §3.xx cites (not §2.xx) in TY2024 comments and the `source` field.

---

### §3.01 — Ordinary-income rate tables (`TaxTable::ordinary`)

Source: **Rev. Proc. 2023-34 §3.01**, Tables 1–4, §1(j)(2)(A)–(D).

**Table 1 — §1(j)(2)(A): Married Filing Jointly / Qualifying Surviving Spouse**

| Rate | Lower bound |
|------|------------|
| 10%  | $0         |
| 12%  | $23,200    |
| 22%  | $94,300    |
| 24%  | $201,050   |
| 32%  | $383,900   |
| 35%  | $487,450   |
| 37%  | $731,200   |

**Table 2 — §1(j)(2)(B): Head of Household**

| Rate | Lower bound |
|------|------------|
| 10%  | $0         |
| 12%  | $16,550    |
| 22%  | $63,100    |
| 24%  | $100,500   |
| 32%  | $191,950   |
| 35%  | $243,700   |
| 37%  | $609,350   |

**Table 3 — §1(j)(2)(C): Unmarried Individuals (Single)**

| Rate | Lower bound |
|------|------------|
| 10%  | $0         |
| 12%  | $11,600    |
| 22%  | $47,150    |
| 24%  | $100,525   |
| 32%  | $191,950   |
| 35%  | $243,725   |
| 37%  | $609,350   |

**Table 4 — §1(j)(2)(D): Married Filing Separately**

| Rate | Lower bound |
|------|------------|
| 10%  | $0         |
| 12%  | $11,600    |
| 22%  | $47,150    |
| 24%  | $100,525   |
| 32%  | $191,950   |
| 35%  | $243,725   |
| 37%  | $365,600   |

Note: MFS lower bands 10%–35% mirror Single; 37% starts at $365,600 (half of MFJ $731,200 —
Rev. Proc. explicitly states $365,600; R0 to confirm).

**[R0-N2] Primary-source rendering discrepancy (resolved):** in the official PDF
(`rp-23-34.pdf`), the 32% rows of Tables 2–4 print "…plus 32% of the excess over **$191,150**"
while the bound column in the same rows reads "Over **$191,950**".  The IRB publication of
record (IRB 2023-48 HTML) reads $191,950 in BOTH columns, and the base-tax arithmetic confirms
it (HoH: 37,417 = 15,469 + 24% × (191,950 − 100,500); Single/MFS: 39,110.50 = 17,168.50 + 24% ×
(191,950 − 100,525)).  The correct 32% lower bound is **$191,950** in all three tables; the PDF's
"$191,150" is a rendering typo.  Future digit-by-digit verifiers should not stall on it.

QSS is not inserted; `TaxTable::key` maps `Qss → Mfj` at lookup time (same as TY2025).

---

### §3.03 — §1(h) maximum capital gains rate breakpoints (`TaxTable::ltcg`)

Source: **Rev. Proc. 2023-34 §3.03**, §1(h)/§1(j)(5)(B).

| Filing status                           | max_zero  | max_fifteen |
|-----------------------------------------|-----------|-------------|
| MFJ / Qualifying Surviving Spouse       | $94,050   | $583,750    |
| MFS                                     | $47,025   | $291,850    |
| Head of Household                       | $63,000   | $551,350    |
| Single (all other individuals)          | $47,025   | $518,900    |

Note: MFS `max_fifteen` = $291,850.  This is NOT exactly half of MFJ ($583,750 / 2 = $291,875);
the Rev. Proc. states $291,850 directly (independent rounding).  R0 must verify $291,850 verbatim
from the primary source.

QSS uses MFJ values via `TaxTable::key`; no QSS entry is inserted.

---

### §3.43 — §2503/§2523 gift-tax annual exclusion per donee (`TaxTable::gift_annual_exclusion`)

Source: **Rev. Proc. 2023-34 §3.43** (§2503; §2523).
Value: **$18,000** per donee for calendar year 2024.
(TY2025 = $19,000 from Rev. Proc. 2024-40 §2.43 — increment of $1,000 reflects continued
inflation adjustment; this $18,000 figure is the one for TY2024.)

---

### §3.41 — §2010(c)(3) basic exclusion amount (`TaxTable::gift_lifetime_exclusion`)

Source: **Rev. Proc. 2023-34 §3.41** (§2010).
Value: **$13,610,000** for an estate of a decedent dying in calendar year 2024.
(TY2025 = $13,990,000 from Rev. Proc. 2024-40 §2.41.)

---

### SSA 2024 — Social Security contribution and benefit base (`TaxTable::ss_wage_base`)

Source: **SSA announcement, October 12, 2023** (§230 of the Social Security Act, 42 U.S.C. §430).
Value: **$168,600** for 2024.
(TY2025 = $176,100 from SSA announcement 2024-10-10.  TY2024 increased from $160,200 in 2023.)

---

### Statutory constants — confirmed NOT in `TaxTable`

The following are fixed in the U.S. Code and must never appear in a `TaxTable` (I4 / Global
Constraints, `tables.rs` module docstring).  Adding TY2024 does NOT change them:

- `NIIT_RATE` = 3.8% (§1411(a)(1))
- `niit_threshold`: MFJ/QSS $250,000; Single/HoH $200,000; MFS $125,000 (§1411(b))
- `loss_limit`: general $3,000; MFS $1,500 (§1211(b))
- `SE_RATE_SS` = 12.4% (§1401(a))
- `SE_RATE_MEDICARE` = 2.9% (§1401(b)(1))
- `SE_RATE_ADDL_MEDICARE` = 0.9% (§1401(b)(2)(A))
- `SE_NET_EARNINGS_FACTOR` = 0.9235 (§1402(a)(12))
- `se_addl_medicare_threshold`: MFJ/QSS $250,000; Single/HoH $200,000; MFS $125,000 (§1401(b)(2)(A))
- `QUALIFIED_APPRAISAL_THRESHOLD` = $5,000 (§170(f)(11)(C))

---

## Current state (recon @ `42ddab8`)

- **`crates/btctax-adapters/src/tax_tables.rs` (the only file that changes)**
  - Line 1 (module docstring): "Bundled per-year tax tables — TY2025 indexed numbers from Rev.
    Proc. 2024-40."
  - Lines 39–40 (`BundledTaxTables` struct comment; the doc comment spans 37–42): "Currently
    contains **TY2025** only (from Rev. Proc. 2024-40).  TY2026 will be added once verified
    against Rev. Proc. 2025-32 + OBBBA structural law."
  - Lines 49–57 (`load()` function): inserts only `2025 → ty2025()`; contains a commented-out
    `by_year.insert(2026, ty2026())` placeholder with "add ONLY when verified" note.
  - Line 49 (`load()` docstring): "Build the compiled-in tables (TY2025 mandatory; later years
    added as their Rev. Procs. are verified)."

- **`report --tax-year 2024` today:** `BundledTaxTables::load()` builds a map containing only
  year 2025.  `compute_tax_year` at `crates/btctax-core/src/tax/compute.rs:258–261` hits the
  `table_for(2024) → None` branch and returns
  `TaxOutcome::NotComputable(Blocker { kind: BlockerKind::TaxTableMissing, detail: "no bundled
  tax table for 2024" })`.  The CLI renders this as a NOT COMPUTABLE result; exit 0.

- **Existing tests unaffected by adding TY2024:**
  - `tax_tables.rs:missing_year_returns_none` — uses year 2099; remains valid.
  - `kat_rate_engine.rs:refusal_and_missing_table_end_to_end` — uses year 2099; remains valid.
  - `tax_report.rs:carryforward_mismatch_advisory_rendered` — uses year 2026 as the no-table
    year for the carryforward mismatch test; remains valid (2026 stays unbundled).
  - `method_election.rs:synth_2024()` — a local `OneTable` test-double for year 2024; it wraps
    its own synthetic `TaxTable` and is completely independent of `BundledTaxTables`; unaffected.
  - **[R0-M2]** `crates/btctax-tui/src/tabs/tests.rs::tax_tab_year_change_updates_figures`
    (~lines 926–958) — navigates Left to year 2024 with no 2024 profile and asserts
    "NOT COMPUTABLE".  The TUI uses `BundledTaxTables` (`app.rs:108`, `unlock.rs:118`), so after
    the backfill the blocker KIND rendered at 2024 **flips from `TaxTableMissing` to
    `TaxProfileMissing`** (refusal precedence (2)→(3), `compute.rs`).  The test still passes —
    it asserts only the "NOT COMPUTABLE" string and the absence of "1500.00" — but this is a
    real behavioral shift at year 2024; the implementer must run it and confirm, and Task 2
    re-verifies.
  - **[R0-M2]** `crates/btctax-cli/tests/optimize_run.rs:385`
    `optimize_run_pre2025_is_usage_error` — calls `optimize::run(…, 2024, …)` and expects an
    error; unaffected because the pre-2025 guard fires before any table lookup.  Checked and
    cleared.

- **No existing test asserts `table_for(2024)` returns `None`.**  There is nothing to "flip" or
  remove — new KATs simply assert the positive case.

- **Comments that reference "TY2025 only" and must be updated (doc-only, no logic change):**
  1. `crates/btctax-adapters/src/tax_tables.rs:1` — module docstring
  2. `crates/btctax-adapters/src/tax_tables.rs:39–40` — struct comment
  3. `crates/btctax-adapters/src/tax_tables.rs:49` — `load()` docstring
  4. `crates/btctax-cli/src/cmd/optimize.rs:162` — inline comment "tables (TY2025 only)"
  5. `crates/btctax-cli/tests/optimize_accept.rs:83` — comment "The bundled tables cover TY2025 only"

---

## Design

### D1 — `ty2024()` builder

Add a `ty2024() -> TaxTable` function in `crates/btctax-adapters/src/tax_tables.rs`, immediately
before `ty2025()`.  Mirror `ty2025()` exactly in structure:

- Use the `br(lower, rate)` helper for bracket construction (already defined in the file).
- Insert `Single`, `Mfj`, `HoH`, `Mfs` into `ordinary`; do **not** insert `Qss` (maps to `Mfj`
  via `TaxTable::key` at lookup time).
- Insert `Single`, `Mfj`, `HoH`, `Mfs` into `ltcg`; do **not** insert `Qss`.
- Set fields from verified values above.
- `source` field:
  ```
  "Rev. Proc. 2023-34 §3.01/§3.03 + §3.43 + §3.41 (TY2024); \
   SSA 2023-10-12 (ss_wage_base $168,600)"
  ```

The function body, values, and citation-comment conventions must follow the exact pattern
established by `ty2025()`.  Each bracket section should carry a matching `// §3.01 — <Status>`
comment.  Each ancillary field should carry an inline `// §... (TY2024 = $X)` comment.

### D2 — `load()` registration

In `BundledTaxTables::load()`, add `by_year.insert(2024, ty2024());` **before** the existing
`by_year.insert(2025, ty2025());` line (BTreeMap ordering is irrelevant to correctness, but
chronological order aids readability).

The existing commented-out TY2026 placeholder (`// by_year.insert(2026, ty2026())`) is left
**unchanged** — TY2026 and TY2027 remain blocked on publication (see Out of scope).

### D3 — Comment updates (doc-only, no logic change)

Update the five comment sites listed in Current state §"Comments that reference 'TY2025 only'" to
reflect that TY2024 and TY2025 are both bundled.  No code logic, no test logic, no signatures
change.

**[R0-M1] Site 1 (the module docstring) is more than the first line.**  The docstring's
"# Source citation" block (`tax_tables.rs:12–18`: "TY2025 values are encoded verbatim from:
Rev. Proc. 2024-40 …") and line 24 ("the TY2025 indexed values are exactly Rev. Proc. 2024-40")
currently attribute ALL bundled values to Rev. Proc. 2024-40 — wrong once TY2024 lands.  The
docstring update MUST:
- (a) add a TY2024 citation block: **Rev. Proc. 2023-34 §3.01** (rate tables), **§3.03** (Maximum
  Capital Gains Rate), **§3.43** (gift annual exclusion $18,000), **§3.41** (basic exclusion
  amount $13,610,000), plus **SSA 2023-10-12** (ss_wage_base $168,600); and
- (b) keep the OBBBA (Pub. L. 119-21) note scoped to **TY2025** — OBBBA is a 2025 enactment and
  says nothing about TY2024 values.

### D4 — Statutory-constants invariant (no change required)

NIIT rate/threshold, §1211 loss limit, SE rates, and all other statutory constants in
`crates/btctax-core/src/tax/tables.rs` are **untouched** — they are NOT year-indexed and do not
appear in `TaxTable`.  The existing `statutory_values_are_constant_across_years` KAT in
`tables.rs` covers this and continues to pass without modification.

---

## Plan (TDD)

### Task 1 — `ty2024()` builder + KATs

**Files:** `crates/btctax-adapters/src/tax_tables.rs` (production code + `mod tests`).

#### KAT-A1 — Single bracket table matches Rev. Proc. 2023-34 §3.01 Table 3
```
let t = BundledTaxTables::load();
let s = t.table_for(2024).unwrap().ordinary_for(FilingStatus::Single);
assert_eq!(s.brackets[1].lower, dec!(11600));  // 12% start
assert_eq!(s.brackets[2].lower, dec!(47150));  // 22% start
assert_eq!(s.brackets[6].lower, dec!(609350)); // 37% start
assert_eq!(s.brackets[6].rate,  dec!(0.37));
```
Hand-derivation: verbatim from Rev. Proc. 2023-34 §3.01 Table 3 (§1(j)(2)(C)).  Fails red before
`ty2024()` is added; green after.

#### KAT-A2 — MFS 37% starts at $365,600 (Table 4); MFJ at $731,200 (Table 1)
```
let t = BundledTaxTables::load();
let tt = t.table_for(2024).unwrap();
assert_eq!(tt.ordinary_for(FilingStatus::Mfs).brackets.last().unwrap().lower, dec!(365600));
assert_eq!(tt.ordinary_for(FilingStatus::Mfj).brackets.last().unwrap().lower, dec!(731200));
```
Verifies the MFS/MFJ 37% boundary divergence (§3.01 Tables 1 and 4).

#### KAT-A3 — LTCG breakpoints all statuses — §3.03
```
let t = BundledTaxTables::load();
let tt = t.table_for(2024).unwrap();
assert_eq!(*tt.ltcg_for(FilingStatus::Single),
    LtcgBreakpoints { max_zero: dec!(47025), max_fifteen: dec!(518900) });
assert_eq!(*tt.ltcg_for(FilingStatus::Mfj),
    LtcgBreakpoints { max_zero: dec!(94050), max_fifteen: dec!(583750) });
// QSS ≡ MFJ (TaxTable::key maps Qss → Mfj)
assert_eq!(*tt.ltcg_for(FilingStatus::Qss),
    LtcgBreakpoints { max_zero: dec!(94050), max_fifteen: dec!(583750) });
assert_eq!(*tt.ltcg_for(FilingStatus::HoH),
    LtcgBreakpoints { max_zero: dec!(63000), max_fifteen: dec!(551350) });
assert_eq!(*tt.ltcg_for(FilingStatus::Mfs),
    LtcgBreakpoints { max_zero: dec!(47025), max_fifteen: dec!(291850) });
```
Note: MFS `max_fifteen` = **$291,850** (NOT $291,875; independently rounded in Rev. Proc.).

#### KAT-A4 — Ancillary fields — §3.43 / §3.41 / SSA
```
let t = BundledTaxTables::load();
let tt = t.table_for(2024).unwrap();
assert_eq!(tt.gift_annual_exclusion,  dec!(18000));
assert_eq!(tt.ss_wage_base,           dec!(168600));
assert_eq!(tt.gift_lifetime_exclusion, dec!(13_610_000));
```

#### KAT-A5 — TY2024 now available: `table_for(2024)` returns `Some`
```
assert!(BundledTaxTables::load().table_for(2024).is_some());
```
This is the direct complement to the pre-existing `missing_year_returns_none` (year 2099) and
demonstrates that the previously-failing `report --tax-year 2024` path is resolved.

#### KAT-A6 — Bracket-edge compute KATs (four statuses, one per status)

Each is an end-to-end `compute_tax_year` golden using `BundledTaxTables::load()` and year=2024.
The `LedgerState` is built directly (same pattern as `kat_rate_engine.rs`).  All figures are
hand-derived and asserted exact.

**Fixture convention [R0-I1]:** in every A6 fixture, `magi_excluding_crypto = OTI` — the
established `kat_rate_engine.rs` convention (e.g. `mfs_profile(270000, 270000)` in KAT 7).
Do NOT embed the crypto gain in `magi_excluding_crypto`; the engine adds the crypto delta
itself: `magi_with = profile.magi_excluding_crypto + crypto_agi` (`compute.rs:357–361`).

**NII convention [R0-C1]:** the engine's NII includes the surviving net **short-term** gain —
`nii_with = qd + with.ordinary_gain + with.preferential_gain − with.loss_deduction +
interest_nii` (`compute.rs:352–353`; module contract at `compute.rs:214`: "NII is `QD +
surviving net capital gains (ST+LT)`").  This is §1411(c)(1)(A)(iii) / Form 8960 line 5a: net
gain from disposition of property is NII regardless of holding period.  ST gains are taxed at
ordinary *rates* but are NOT thereby excluded from NII.

**A6a — Single, 22% bracket entry** (§3.01 Table 3; 22% starts at $47,150):
- OTI = $47,150; Crypto ST gain = $1,000; magi_excl = $47,150 (= OTI).
- WITH: $1,000 falls entirely in the 22% band ($47,150 → $48,150 < $100,525).
  ord_delta = 22% × $1,000 = **$220.00**.
  Cumulative cross-check: tax(47,150) = 1,160 + 12% × (47,150 − 11,600) = $5,426.00 (matches
  the Rev. Proc.'s own base amount at the 22% edge); tax(48,150) = 5,426.00 + 22% × 1,000 =
  $5,646.00; Δ = $220.00.
- NIIT: nii_with = 1,000 (ST gain IS NII); MAGI_with = magi_excl + crypto_agi = 47,150 + 1,000
  = 48,150 < $200,000 Single threshold (margin $151,850) → NIIT = $0.
  Assert `assert_eq!(r.niit, dec!(0))`.
- total = ord_delta + ltcg_tax + niit = 220.00 + 0 + 0 = **$220.00**.

**A6b — MFJ, 22%/24% boundary** (§3.01 Table 1; 22% ends at $201,050):
- OTI = $200,000; Crypto ST gain = $2,000; magi_excl = $200,000 (= OTI).
- WITH: $1,050 at 22% ($200,000→$201,050), $950 at 24% ($201,050→$202,000).
  ord_delta = 22% × $1,050 + 24% × $950 = $231.00 + $228.00 = **$459.00**.
  Cumulative cross-check: tax(200,000) = 10,852 + 22% × (200,000 − 94,300) = $34,106.00;
  tax(202,000) = 34,337 + 24% × (202,000 − 201,050) = $34,565.00; Δ = $459.00.
- NIIT: nii_with = 2,000; MAGI_with = 200,000 + 2,000 = 202,000 < $250,000 MFJ threshold
  (margin $48,000) → NIIT = $0.  Assert `assert_eq!(r.niit, dec!(0))`.
- total = 459.00 + 0 + 0 = **$459.00**.

**A6c — HoH, 12%/22% boundary** (§3.01 Table 2; 22% starts at $63,100):
- OTI = $63,000; Crypto ST gain = $500; magi_excl = $63,000 (= OTI).
- WITH: $100 at 12% ($63,000→$63,100), $400 at 22% ($63,100→$63,500).
  ord_delta = 12% × $100 + 22% × $400 = $12.00 + $88.00 = **$100.00**.
  Cumulative cross-check: tax(63,000) = 1,655 + 12% × (63,000 − 16,550) = $7,229.00;
  tax(63,500) = 7,241 + 22% × (63,500 − 63,100) = $7,329.00; Δ = $100.00.
- NIIT: nii_with = 500; MAGI_with = 63,000 + 500 = 63,500 < $200,000 HoH threshold
  (margin $136,500) → NIIT = $0.  Assert `assert_eq!(r.niit, dec!(0))`.
- total = 100.00 + 0 + 0 = **$100.00**.

**A6d — MFS, 35%/37% boundary** (§3.01 Table 4; 37% starts at $365,600) — carries a NIIT leg:
- OTI = $365,000; Crypto ST gain = $1,000; magi_excl = $365,000 (= OTI).
- WITH: $600 at 35% ($365,000→$365,600), $400 at 37% ($365,600→$366,000).
  ord_delta = 35% × $600 + 37% × $400 = $210.00 + $148.00 = **$358.00**.
  Cumulative cross-check: tax(365,000) = 55,678.50 + 35% × (365,000 − 243,725) = $98,124.75;
  tax(366,000) = 98,334.75 + 37% × (366,000 − 365,600) = $98,482.75; Δ = $358.00.
- NIIT [R0-C1]: ST gain is NII (§1411(c)(1)(A)(iii); engine: `nii_with` includes
  `with.ordinary_gain`, `compute.rs:352–353`); MFS threshold $125,000 exceeded:
  - crypto_agi = 1,000 → MAGI_with = 365,000 + 1,000 = 366,000 > 125,000.
  - nii_with = 1,000; over = 366,000 − 125,000 = 241,000; base = min(1,000, 241,000) = 1,000.
  - niit_with = 3.8% × 1,000 = **$38.00**; niit_without = 0 (nii_without = 0);
    niit_delta = $38.00.
  Assert `assert_eq!(r.niit, dec!(38.00))`.
- total = ord_delta + ltcg_tax + niit = 358.00 + 0 + 38.00 = **$396.00**.
  Assert `assert_eq!(r.total_federal_tax_attributable, dec!(396.00))`.
- Note: there is NO MFS input near the $365,600 bracket edge that avoids NIIT — the statutory
  $125,000 threshold sits far below the edge — so this KAT must carry the NIIT leg.

#### KAT-A7 — TY2024 LTCG threshold KAT, Single crossing 0%→15%

- OTI = $40,000; Crypto LT gain = $10,000; MAGI excl. = $40,000.
- TY2024 Single: max_zero = $47,025; max_fifteen = $518,900.
- pref stack: bottom = $40,000, top = $50,000.
  - at_0  = 47,025 − 40,000 = $7,025 → taxed at 0%
  - at_15 = 10,000 − 7,025 = $2,975 → taxed at 15%
  - ltcg_tax = 2,975 × 0.15 = **$446.25**
- NIIT: MAGI_with = 40,000 + 10,000 = 50,000 < $200,000 threshold → $0.
- ord_delta = 0 (LT gain does not increase ordinary income).
- total = **$446.25**.

These KATs belong in `crates/btctax-adapters/src/tax_tables.rs` in the existing `mod tests`
block (alongside the TY2025 KATs), using `BundledTaxTables::load()`.  Add the bracketing
`// ── TY2024 KATs ─────` comment header for legibility.

#### Regression KAT (mandatory)

Add an explicit check that TY2025 ordinary and LTCG values are UNCHANGED after the TY2024
addition — add a brief assertion (or confirm the existing
`ty2025_single_ordinary_brackets_match_rev_proc_2024_40` KAT still passes without modification).
The existing TY2025 KATs are sufficient; the regression requirement is satisfied by them passing.
Document this in the Task 1 checklist: "run `cargo test -p btctax-adapters` pre-merge; all
TY2025 KATs byte-identical."

---

### Task 2 — Whole-diff review (Phase E) + FOLLOWUPS

Cross-cutting items R0 must verify:

1. **Every bracket lower bound** in `ty2024()` matches the Rev. Proc. 2023-34 primary source
   verbatim (digit-by-digit comparison against §3.01 Tables 1–4).
2. **Every LTCG breakpoint** matches §3.03 verbatim, including MFS $291,850 (not $291,875).
3. **Section cites** in the `source` field and all inline comments use `§3.01`, `§3.03`, `§3.41`,
   `§3.43` (SECTION 3, not SECTION 2 as in Rev. Proc. 2024-40).
4. **QSS not inserted** — no `ordinary.insert(FilingStatus::Qss, ...)` or
   `ltcg.insert(FilingStatus::Qss, ...)` in `ty2024()`.
5. **Statutory constants untouched** — no change to `NIIT_RATE`, `niit_threshold`, `loss_limit`,
   SE constants, `QUALIFIED_APPRAISAL_THRESHOLD` in `tables.rs`.
6. **TY2025 KATs unmodified** — existing `ty2025_single_ordinary_brackets_match_rev_proc_2024_40`,
   `ty2025_ltcg_breakpoints_all_statuses`, `mfs_37_pct_starts_at_375800_and_mfj_at_751600`,
   `missing_year_returns_none`, `ty2025_gift_annual_exclusion_is_19000`,
   `statutory_values_are_not_in_the_table_and_constant_across_years` all pass unchanged.
7. **`missing_year_returns_none`** (year 2099) still returns `None`.
8. **Comment-only updates** (D3) are consistent across all five sites.
9. **Hand-derivations in A6a–A6d and A7** are independently re-verified by R0 (not trusted from
   the spec author).
10. **`load()` insertion order** — TY2024 inserted before TY2025 for readability; `BTreeMap`
    correctness is order-independent.
11. **[R0-M2] TUI year-2024 blocker-kind flip confirmed benign** —
    `tax_tab_year_change_updates_figures` passes with the blocker at 2024 now
    `TaxProfileMissing` (was `TaxTableMissing`); `optimize_run_pre2025_is_usage_error`
    unaffected (pre-2025 guard precedes table lookup).

**FOLLOWUPS after ship:**
- TY2026 remains blocked on Rev. Proc. 2025-32 publication + OBBBA structural law verification
  (the existing commented-out `// by_year.insert(2026, ty2026())` placeholder and its comment
  remain in place and UNCHANGED).
- TY2027 is similarly blocked.
- Pre-2024 years (2023, 2022, …) are out of scope; no Rev. Proc. verification has been done.

---

## Out of scope

- **TY2026 / TY2027:** no published Rev. Proc. verification; the TY2026 placeholder comment in
  `load()` is left exactly as-is.
- **Pre-2024 years (TY2023 and earlier):** no verification work has been done; no plan to add.
- **Any engine change:** `compute_tax_year`, `compute_se_tax`, the NIIT computation, and all
  downstream rendering logic are untouched.  Adding a bundled table for 2024 makes the existing
  engine operate on TY2024 data; no engine modification is required or permitted.
- **`TaxTable` struct fields:** the struct itself (`tables.rs`) is not modified.  All seven
  fields (`year`, `source`, `ordinary`, `ltcg`, `gift_annual_exclusion`, `ss_wage_base`,
  `gift_lifetime_exclusion`) are already present and receive the TY2024 values from this spec.
- **Standard deduction / AMT / EITC:** not fields in `TaxTable` and not modeled; out of scope.
- **State / local taxes:** app charter; federal only.
