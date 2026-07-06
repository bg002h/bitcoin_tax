# SPEC — official IRS PDF form-fill, sub-project 3 (2017 + 2024 full packet, per-year)

**Source baseline:** `main` @ `55f5812` (branch `feat/irs-form-fill-sp3`). **Review status: R0-GREEN (3 rounds; 0C/0I).
Cleared to implement (two-stage: SP3a = T0+T1 engine+2024, then SP3b = T2/2017).** Reviews:
`reviews/R0-spec-irs-form-fill-sp3-round-{1,2,3}.md`. r1 3C/5I (Fable — caught 2017 SE not computable, the 2024
DA-pair fail-closed, the false 8283 recon); r2 0C/3I (Opus — 3 more 2017 engine gaps: 8283 ¢-pairs, the 2nd
grid token, 1040 DA-optional); r3 0C/0I (Opus — 4 doc Minors/Nits synced: SEC_B_CAP, NIIT-not-in-table, docs to
SP3a, 8283 26-¢-fields definitive). SP3 (final) of task #45. **User-chosen scope (2026-07-06): the
FULL 2017 packet** — incl. a new TY2017 tax table + the engine changes 2017's old forms require. **Correction:
SP3 DOES change the engine** (the DRAFT's "data-only" claim was false — R0-verified).

## Goal (SP3)
`export-irs-pdf --tax-year {2017,2024}` fills the full packet (8949, Schedule D, 8283, Schedule SE, 1040) from a
historical vault — so the ReadOnly TY2017 demo fills the REAL 2017 official PDFs. Requires a **new TY2017 tax
table** (for the SE leg) + several **engine changes** (below), all verified per year.

## [★ R0-C1] btctax must gain a TY2017 tax table (or 2017 Schedule SE cannot exist)
`tax_tables.rs:74` ships TY2024/2025/2026 ONLY; `compute_se_tax` needs the year's `TaxTable.ss_wage_base`, so
`--tax-year 2017` produces NO Schedule SE today. 8949/Schedule D/8283/1040 are table-FREE (pure `LedgerState`;
prices cover 2017) — computable now. **Add a TY2017 `TaxTable` to btctax-adapters** (ordinary brackets + LTCG
breakpoints from **Rev. Proc. 2016-55**; **SS wage base $127,200** from SSA; SE rates 12.4%/2.9%). **[★
tax-critical]** the 2017 values MUST be primary-source-verified with a KAT pinning known 2017 figures (like the
existing per-year tables) — a wrong 2017 rate = a wrong return. 2024 is fully computable (`ty2024`, wage base
$168,600).

## [★ engine changes SP3 owns] (the DRAFT was wrong that the engine is untouched)
1. **[R0-C2 — a latent SP2 bug 2024 exposes] the digital-asset-question oracle picks the TOP-MOST same-y
   `{/1,/2}` `/Btn` pair — which on the 2024 1040 is the FILING-STATUS row (c1_3 @ y=588), not the DA pair
   (c1_5 @ y=487).** `verify.rs:406` `topmost_yes_no_pair` fails CLOSED on a correct 2024 map. **Fix:** select
   the DA pair by **horizontal adjacency** (the two boxes ≤~60pt apart: DA = 36pt vs filing-status = 266pt),
   not top-most-y. **Re-verify against 2025** (no regression) — this improves all years.
2. **[R0-I: dollars+cents field PAIRS — SE, 1040, AND 8283 for 2017] the 2017 SE + 1040 + 8283 split every
   amount into a dollars field AND a cents field** (79 cent-fields on the 2017 1040; **[R0-r2-I1] 26 on the
   2017 8283 Rev.12-2014** — NOT just SE+1040; the DRAFT's "2024/2025 stay single-field" is right but "SE+1040
   only" was wrong). The single-fqn map cell + `fmt_money` + strict-descent oracle can't express it. **Add a
   `MoneyPair{dollars_field, cents_field}` cell type**: split into whole-dollars + 2-digit cents; the oracle
   treats the pair as one logical cell at the dollars-field geometry. **[R0-r2-M2] MoneyPair needs a REAL
   2-decimal/zero-pad formatter** (`fmt_money` is raw `Decimal::to_string` — no cents padding). **[R0-r2-I1]
   MoneyPair must survive `merge_copies` overflow** (the 2017 8283 overflows; the per-copy rename must rewrite
   BOTH the dollars and cents field names as a unit — R0-r3-confirmed this fits `overflow.rs:43` root-`/T`
   rename) and **`SEC_A_CAP` becomes per-year (5 for 2017, not the hardcoded 4); [R0-r3-Ma] `SEC_B_CAP` too —
   2017 Section B (the BTC >$5k appreciated-property donation path) holds 4 rows `Line5A-5D`, not the hardcoded
   3**. (All 2017-only; 2024/2025 stay single-field — R0-r3-confirmed 26 cent fields on 2017 8283, 0 on 2024/2025.)
3. **[R0-I: per-year QOF] 2017 Schedule D has NO QOF question** but `ScheduleDMap` requires one (always writes
   "No"). **Make QOF optional per-year** (the map declares whether the field exists; 2017 omits it).
4. **[R0-I: per-year grid tokens — 8949 AND Schedule D] `F8949_TABLE_TOKEN="Table_Line1_Part"` matches neither
   2017 nor 2024's `Table_Line1[0]`**, AND **[R0-r2-I2] `SCHED_D_TABLE_TOKEN` (schedule_d.rs:18, used :159) is
   hardcoded `Table_PartI` but 2017's is `TablePartI` (no underscore)** → band derivation fails closed on a
   correct 2017 Schedule D. **Make BOTH grid tokens per-year map config**, not consts.
5. **[R0-I: pre-filled constants] the blank 2017 SE ships factory `/V` values** ('127,200'/'00', '5,200'/'00')
   → trips `no_unmapped_filled`. **Add a per-year "pre-filled-exempt" field set** (constants the blank already
   carries) so the check ignores them.
6. **[R0-r2-I3: 2017 1040 has no DA question] make `Form1040Map`'s DA fields `Option`** (parallel to the
   QOF-optional item) AND **skip the `topmost_yes_no_pair`/adjacency DA guard when the DA field is absent** —
   else `fill_form_1040_capgains` errors at form1040.rs:118 on the no-DA 2017 form.

## [★ R0-C3] The recon claim was false — 8283 is REVISION-dated; corrected facts
The DRAFT claimed "all 9 forms verified"; the 8283-2017/2024 URLs were IRS 404 HTML (never checked). Corrected:
no year-editioned 8283 exists — use `irs-prior/f8283--{2014,2023}.pdf`:
- **TY2017 → Rev. 12-2014:** XFA hybrid, **NO digital-asset property box** but **"j Other" EXISTS** → BTC
  donation uses "j Other" + a printed note (do NOT scope out); **OLD part numbering II/III/IV**; **5 Section-A
  rows / 4 Section-B rows** (`Line1A-1E` / `Line5A-5D`); **26 dollars+cents-pair fields (R0-confirmed)** — needs
  MoneyPair.
- **TY2024 → Rev. 12-2023:** has **"k Digital assets"**; parts III/IV/V as 2025.
- Bundle by revision string; a filing-year→revision map (2017→12-2014, 2024→12-2023, 2025→12-2025).

## Per-year facts (recon-verified)
| | 8949 BTC box | rows/part | 1040 cap-gain | DA question | Schedule SE | 8283 rev |
|---|---|---|---|---|---|---|
| **2017** | **Box C/F** (`c1_1[2]`/`c2_1[2]` on `/3`) | 14×8 | **line 13** (¢-pair; the IRS-glitched field name `f1-_51[0]` is HERE — [R0-r2-M4]) | **NONE** | **old short+long, §B long, ¢-pairs, pre-filled** | 12-2014 (no DA box) |
| **2024** | **Box C/F** | 14×8 | **line 7** (no glitch fields) | **yes** (c1_5, adjacency-selected) | unified (name-identical to 2025) | 12-2023 (k DA) |
| 2025 (done) | I/L | 11 | 7a | yes | unified | 12-2025 |

- **Box C/F** = the core `Form8949Box::{C,F}` (forms.rs:114) SP1 declined for 2025 — RIGHT for these pre-1099-DA
  years (data-only via the map's box on-state). **14 rows/part** (SP1-I1 per-year data). Schedule D per-year
  line wording verified at extraction.
- **2017 1040:** cap-gain **line 13**, **NO DA question** → the map has no DA field; the produce/skip rule is
  **reportable capital activity only** (no DA gate); an income-only 2017 year with no disposals ⇒ SKIP the 1040
  (no line-13 value) + note. 2024: **line 7** (not 7a) + the DA question (SP2 C4 YES-iff-activity + I★1
  active/inactive rules port to 2024 field ids).
- **2017 Schedule SE:** the OLD short(§A)+long(§B) form; btctax's SE data maps to the **§B long form** (R0:
  chain maps 1:1, strictly descending; line 12 = "Add lines 10 and 11" — the 0.9% addl Medicare has been
  off-Schedule-SE since 2013, so 2017 already excludes it); $400 floor holds.

## KATs (per year + engine)
- **★ TY2017 tax table:** `ty2017_table_matches_rev_proc_2016_55` — **[R0-r2-M1] a FULL-SCHEDULE equality lock**
  (every ordinary bracket edge + rate, every §1(h) LTCG breakpoint, the $127,200 wage base) — not a few
  spot-pins — the tax-critical primary-source gate. **[R0-r3-Mb] NOT the NIIT threshold** — it is the
  year-independent statutory `niit_threshold()` fn, NEVER a `TaxTable` field (do not add it).
- **★ box:** `ty2017_and_2024_bitcoin_use_box_C_F` (`/3`, NOT I/L); `ty2025_still_I_L` (regression).
- **★ DA oracle fix [C2]:** `da_pair_selected_by_adjacency_not_topmost` (the 2024 filing-status row is NOT
  chosen); `ty2025_da_still_correct` (no regression); `ty2024_1040_fills_da_and_line7`.
- **★ dollars+cents:** `money_pair_splits_dollars_and_cents` (2017 SE/1040); the geometric oracle passes on the
  pair; fault-inject a pair swap ⇒ RED.
- **per-year:** `ty2017_1040_line13_no_da_question`; `ty2017_schedule_d_has_no_qof`; `ty2017_8949_14_rows`;
  `ty2017_schedule_se_long_form_section_b`; `ty2017_se_prefilled_constants_are_exempt`.
- **8283 revisions:** `ty2017_8283_rev_2014_uses_j_other_with_note` (no DA box); `ty2024_8283_rev_2023_digital_assets_box`.
- **★ per-form geometric read-back + fault-inject** for EACH new (form,year) — swap two map entries ⇒ RED,
  fails closed (the oracle re-derives per year); `no_unmapped_filled` per (form,year) (minus the pre-filled set).
- **determinism:** golden sha per (form,year); `map_YYYY_matches_bundled_pdf_fieldset` for 2017 + 2024.
- **regression:** the full 2025 + core tax suites stay green (no per-year branch or engine change breaks them).

## Scope / SemVer / lockstep
btctax-adapters (**+TY2017 `TaxTable`**) + `btctax-forms` (engine: MoneyPair cell, adjacency DA-pair, per-year
QOF/table-token/pre-filled config; +`forms/2017/` +`forms/2024/` PDFs & maps) + `export-irs-pdf` per-year
dispatch. Bundled PDFs public domain. MINOR (new years + new table). Man page + README (supported years
2017/2024/2025; 2017 has no DA question + old SE + Box C/F + revision-dated 8283). cargo-tree isolation unchanged.

## Plan (TDD)
- **T0 (engine + table)** — the DA-pair adjacency fix (+2025 regression), the `MoneyPair` cell type, per-year
  config (box/rows/table-token/QOF-optional/pre-filled-exempt/logical-sequence), AND the **TY2017 `TaxTable`**
  (Rev. Proc. 2016-55 + $127,200) with its primary-source KAT. Unit-tested; 2025 + core suites stay green.
- **T1 (2024 — closest to 2025)** — bundle 2024 PDFs; the 2024 maps (Box C/F, 14 rows, 1040 line 7 + DA-by-
  adjacency, unified SE, 8283 Rev. 12-2023); the 2024 fill branches; 2024 KATs + fault-injects; 2025 regression;
  **[R0-r3-Mc] man page + README updated for 2024** (SP3a ships 2024 as a supported year — must not be
  undocumented; SP3b/T2 amends them again for 2017).
- **T2 (2017 — the old forms)** — bundle 2017 PDFs (+ 8283 Rev. 12-2014); the TY2017 tax table + its
  full-schedule KAT; the 2017 maps incl. the old §B Schedule SE (¢-pairs, pre-filled-exempt), the no-DA
  line-13 1040 (¢-pairs), the no-QOF Schedule D, the "j Other" 8283 (¢-pairs, 5 rows, overflow); the 2017 fill
  branches; 2017 KATs + fault-injects; **end-to-end on the ReadOnly TY2017 vault**; man page + README; whole-diff.

**[R0-r2 rec] Two-stage delivery:** MERGE after **T0+T1** (the engine changes + the 2024 full packet — a bounded
diff, and the DA-adjacency fix ships promptly since it de-risks all years); then do **T2 (2017) as a SEPARATE
branch/whole-diff/merge (SP3b)** — the 2017 engine weight (MoneyPair×overflow, the old §B SE, the no-DA 1040,
the new tax table) earns its own review. Keeps each merge reviewable; the spec stays one coherent design.

## Gotchas
- **[★ C1] add the TY2017 tax table** (primary-source-verified) or no 2017 SE.
- **[★ C2] DA-pair by horizontal adjacency**, not top-most-y (2024 filing-status row trap; re-verify 2025).
- **[★ ¢-pairs] MoneyPair cell** for 2017 SE/1040 (dollars field + cents field).
- **[★ C3] 8283 is revision-dated** — 2017 Rev. 12-2014 ("j Other", no DA box, parts II/III/IV, 5 rows); 2024
  Rev. 12-2023 ("k Digital assets").
- **[per-year config — the full set, R0-r2-M3]** box C/F + `/3`, 14 rows, **BOTH** grid tokens (8949 +
  Schedule D), QOF-optional, DA-optional, pre-filled-exempt, `SEC_A_CAP`, MoneyPair-vs-single, the column-x
  clusters + page indexes, and the bundled PDF bytes — all map/config DATA keyed by year, not `if year==`
  ladders.
- **[2017 1040]** no DA question → produce iff reportable capital activity; income-only 2017 ⇒ skip + note.
- **[regression]** keep 2025 + core tax suites green. **[safety]** pseudo ⇒ attestation + DRAFT watermark
  (unchanged); determinism (golden sha) per (form,year).
