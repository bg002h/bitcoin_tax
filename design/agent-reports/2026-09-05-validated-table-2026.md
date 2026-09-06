# TY2026 validated tax table — independent transcription

**Agent:** validated-table-2026
**Date:** 2026-09-05
**Deliverable:** `btctax_core::tax::testonly::ty2026_table()` + wiring into
`crates/btctax-adapters/tests/shipped_tables_are_the_validated_tables.rs`
**Outcome:** transcribed, wired, green. **0 disagreements** with the shipped table.

---

## 1. Sources

| what | where | committed at |
|---|---|---|
| Rate tables, §1(h) breakpoints, gift annual exclusion | **Rev. Proc. 2025-32**, §4.01 Tables 1–4, §4.03, §4.42(1) | `legal/text/irs-guidance/RevProc_2025-32.txt` |
| §2010(c)(3) basic exclusion amount | **Rev. Proc. 2025-32 §2.14** (SECTION 2, CHANGES — *not* §4) | same file |
| Social Security wage base | **90 FR 49047, 49050** (Vol. 90 No. 210, Monday 3 November 2025) | `legal/text/federal-register/SSA_COLA_Determinations_2026.txt` |

`tax_tables.rs` was **not opened** until every figure below was written and the
transcription compiled. It was then read once, to produce §6.

---

## 2. What Rev. Proc. 2025-32 supersedes (the note the brief asked for)

It is a **modifying** procedure, not a standalone one. §1: it "modifies certain sections of
Rev. Proc. 2024-40, 2024-45 I.R.B. 1100, to reflect the amendments to the Internal Revenue Code
(Code) by Public Law 119-21, 139 Stat. 72 (July 4, 2025), commonly known as the One, Big,
Beautiful Bill Act (OBBBA)", for the Code "as in effect on October 9, 2025". §6, in full:
"Rev. Proc. 2024-40 is modified."

Three things bear on this table:

1. **§2.01 — the rate tables were made PERMANENT.** OBBBA §70101 amends §1(j) so the post-TCJA
   tables "effective for taxable years beginning after December 31, 2017, and before January 1,
   2026" are permanent; "the existing seven tax rates of 10%, 12%, 22%, 24%, 32%, 35%, and 37%
   remain in effect for individual taxpayers." So TY2026 keeps the seven-bracket shape and there
   is **no sunset back to pre-TCJA §1(a)–(d)** — the single largest thing that could have gone
   wrong in a TY2026 port, and it is settled by a sentence, not by inference.

2. **What §3 REMOVES is retroactive to TY2025, and touches nothing in this table.**
   - **§3.01** removes **§2.15(1) of Rev. Proc. 2024-40** — the 2025 standard deduction —
     because §63(c)(7) as amended by OBBBA sets it directly: MFJ **$31,500**, HoH **$23,625**,
     Single **$15,750**, MFS **$15,750**.
   - **§3.02** removes **§2.25 of Rev. Proc. 2024-40** — the 2025 §179 expensing limits —
     superseded by **$2,500,000** with the phase-down starting at **$4,000,000**; §3.02(2)
     conforms Rev. Proc. 2024-40's Table of Contents.
   - **§2.04** removes the §36B(f)(2)(B) inflation adjustment outright (OBBBA §71305 repealed the
     provision for taxable years beginning after 2025).

   ★ **Neither removal reaches §2.01 (rate tables) or §2.03 (capital gains) of Rev. Proc.
   2024-40.** Those were only ever the TY2025 figures; the TY2026 ones are new text in §4 here.
   Recorded because "what did it supersede" is precisely the question a silent year-port never
   asks — and this year the answer is *the standard deduction and §179 for the PRIOR year*, which
   is exactly the shape that would be mis-read as "the 2026 tables were removed".

3. **§2.14 is where the lifetime gift/estate figure lives this year.** OBBBA §70106 "amends
   §2010(c)(3) by increasing the basic exclusion amount to **$15,000,000** for calendar year
   2026", with §2631(c) GST equal to the same, "adjusted for inflation for taxable years
   beginning after December 31, 2026". It is a **statutory** number, so it is not in §4 with the
   §1(f) inflation adjustments. A transcription that only walked §4 would have left this field
   with no source at all.

---

## 3. Ordinary rate schedules — §4.01, verbatim thresholds

Each figure is the income at which the rate **starts**.

### §4.01 TABLE 1 — §1(j)(2)(A), Married Individuals Filing Joint Returns and Surviving Spouses

| rate | starts at |
|---|---|
| 10% | $0 |
| 12% | $24,800 |
| 22% | $100,800 |
| 24% | $211,400 |
| 32% | $403,550 |
| 35% | $512,450 |
| 37% | $768,700 |

### §4.01 TABLE 2 — §1(j)(2)(B), Heads of Households

| rate | starts at |
|---|---|
| 10% | $0 |
| 12% | $17,700 |
| 22% | $67,450 |
| 24% | $105,700 |
| 32% | **$201,750** |
| 35% | **$256,200** |
| 37% | $640,600 |

### §4.01 TABLE 3 — §1(j)(2)(C), Unmarried Individuals (other than Surviving Spouses and Heads of Households)

| rate | starts at |
|---|---|
| 10% | $0 |
| 12% | $12,400 |
| 22% | $50,400 |
| 24% | $105,700 |
| 32% | **$201,775** |
| 35% | **$256,225** |
| 37% | $640,600 |

### §4.01 TABLE 4 — §1(j)(2)(D), Married Individuals Filing Separate Returns

| rate | starts at |
|---|---|
| 10% | $0 |
| 12% | $12,400 |
| 22% | $50,400 |
| 24% | $105,700 |
| 32% | $201,775 |
| 35% | $256,225 |
| 37% | **$384,350** |

TABLE 5 (§1(j)(2)(E), Estates and Trusts: $3,300 / $11,700 / $16,000 at 10/24/35/37%) is in the
procedure but has no `FilingStatus` and is deliberately not transcribed.

### ★ The independent-rounding trap, TY2026 edition — WORSE than TY2025

TY2025 had exactly one HoH row $25 below Single's. **TY2026 has two**:

| row | Single / MFS | HoH | delta |
|---|---|---|---|
| 24% floor | $105,700 | $105,700 | **equal** |
| 32% floor | $201,775 | $201,750 | **−$25** |
| 35% floor | $256,225 | $256,200 | **−$25** |
| 37% floor | $640,600 | $640,600 | **equal** |

The two divergent rows are **bracketed on both sides by rows that are identical**, which is the
worst possible arrangement for a copy-with-edits port: the eye confirms $105,700 and $640,600
match and stops looking. Every status was read from its own table.

MFS: identical to Single through the 35% bracket, then $384,350 = exactly half the joint
$768,700 per §1(j)(2)(D). The half-relationship is *checked*, never *used* as the source.

### ★★ Machine-check: every threshold re-derived from the procedure's own cumulative-tax column

The "The Tax Is" column is a second, redundant encoding of the same breakpoints. All **24**
interior rows (4 statuses × 6) were recomputed by script from the thresholds and rates and
compared to the printed dollar amounts. **24/24 agree**, exactly:

```
MFJ    24,800 → 2,480      100,800 → 11,600     211,400 → 35,932
       403,550 → 82,048    512,450 → 116,896    768,700 → 206,583.50
HoH    17,700 → 1,770      67,450 → 7,740       105,700 → 16,155
       201,750 → 39,207    256,200 → 56,631     640,600 → 191,171
Single 12,400 → 1,240      50,400 → 5,800       105,700 → 17,966
       201,775 → 41,024    256,225 → 58,448     640,600 → 192,979.25
MFS    12,400 → 1,240      50,400 → 5,800       105,700 → 17,966
       201,775 → 41,024    256,225 → 58,448     384,350 → 103,291.75
```

This is what actually pins the $25 rows. TABLE 2 prints "$56,631 plus 35%"; a mis-keyed HoH 35%
floor of $256,225 would require $56,639. The digit is caught by arithmetic rather than by a
re-read — the failure mode `CLAUDE.md` records for Form 6251 line 33.

---

## 4. §1(h) capital-gain breakpoints — §4.03 Maximum Capital Gains Rate (§1(h), §1(j)(5))

| filing status (procedure's own label) | Maximum Zero Rate Amount | Maximum 15% Rate Amount |
|---|---|---|
| Married Individuals Filing Joint Returns and Surviving Spouse | $98,900 | $613,700 |
| Married Individuals Filing Separate Returns | $49,450 | $306,850 |
| Heads of Household | $66,200 | $579,600 |
| All Other Individuals *(→ `Single`)* | $49,450 | $545,500 |
| Estates and Trusts | $3,300 | $16,250 |

Estates and Trusts is printed but not transcribed (no `FilingStatus`).

★ **A TY2025 trap that is NOT present in TY2026, and must not be inverted into a rule.** In
TY2025 the MFS 15% ceiling was a round $300,000 and *not* half the joint $600,050. In TY2026 both
MFS figures *do* happen to be exactly half the joint ones ($49,450 = 98,900 ÷ 2; $306,850 =
613,700 ÷ 2). That is a coincidence read off the printed table, not a derivation — the halving is
recorded as a check in the doc comment, and the source of both figures is the row of §4.03.

---

## 5. Scalars

| field | value | citation |
|---|---|---|
| `gift_annual_exclusion` | **$19,000** | §4.42(1): "For calendar year 2026, the first $19,000 of gifts to any person (other than gifts of future interests in property) are not included in the total amount of taxable gifts under § 2503 made during that year." (Unchanged from TY2025 — but read for 2026 in its own words, not carried over. §4.42(2) prints $194,000 for a non-citizen spouse; §4.42(3) reuses the $19,000 for §2801.) |
| `ss_wage_base` | **$184,500** | 90 FR 49050: "The OASDI contribution and benefit base is $184,500 for remuneration paid in 2026 and self-employment income earned in tax years beginning in 2026." Summary at 90 FR 49047 prints the same figure; the derivation at 90 FR 49050 notes $184,500 exceeds the current base of $176,100. **Not in the revenue procedure at all.** |
| `gift_lifetime_exclusion` | **$15,000,000** | §2.14 (see §2 above). Statutory under OBBBA §70106, not a §1(f) adjustment — which is why it has no §4 entry. |

`source` field written as:

```
TEST-TY2026 (Rev. Proc. 2025-32 §4.01 Tables 1-4, §4.03, §4.42(1), §2.14; 90 FR 49047)
```

Deliberately **different** from the shipped string (§6). `compare_tables` destructures
`source: _` precisely so the two artifacts cite different provenance; equal strings would be the
signature of a copy.

`Qss` is deliberately **absent** from both maps, matching `ty2024_table`/`ty2025_table`:
§1(j)(2)(A) gives a qualifying surviving spouse the joint schedule (TABLE 1 is titled "…and
Surviving Spouses") and `TaxTable::key` normalises `Qss → Mfj` at lookup. The ratchet checks the
absence is mutual rather than assuming it.

---

## 6. DISAGREEMENTS — read `tax_tables.rs::ty2026()` **last**, after the above was final

### **NONE. Zero disagreements, on all 39 compared figures.**

| group | figures compared | disagreements |
|---|---|---|
| ordinary bracket `(lower, rate)` pairs | 28 (4 statuses × 7) | **0** |
| §1(h) breakpoints | 8 (4 statuses × 2) | **0** |
| `ss_wage_base`, `gift_annual_exclusion`, `gift_lifetime_exclusion` | 3 | **0** |
| status presence (`Qss` absent both sides) | — | **0** (lawful absence, mutual) |

Notes on the shipped side, none of them defects:

- Shipped `ty2026()` inserts statuses in order Single / Mfj / **HoH** / **Mfs**; the validated
  side uses Single / Mfj / **Mfs** / **HoH**. `ordinary` is a `BTreeMap`, so insertion order is
  not observable and this is not a difference in the artifact.
- The shipped table carries its own comment marking the HoH `$201,750` / `$256,200` trap. It was
  found here independently, from TABLE 2's cumulative column, before that comment was read.
- `source` strings differ, as required: shipped is
  `"Rev. Proc. 2025-32 §4.01/§4.03 + §4.42 (TY2026); §2010(c)(3) basic exclusion $15,000,000 per
  OBBBA Pub. L. 119-21 §70106; SS wage base $184,500 per SSA (Fed. Reg. 2025-11-03)"`.
- The shipped module cites the SSA determination only as "Fed. Reg. 2025-11-03". The validated
  side cites **90 FR 49047, 49050 (Vol. 90 No. 210)** — a page-level citation, which is the
  stronger one. Not a disagreement; recorded as an available improvement to the shipped comment.

**A zero-disagreement result is a real outcome here, not a null one.** Before this, none of the
28 TY2026 bracket thresholds a filer's return is computed from had a second, independently
derived witness anywhere in the corpus; the header table in the test file measured TY2026 at
24/28 thresholds asserted, and only by a KAT living in the same file as the data it checks. They
now have one.

---

## 7. Wiring and verification

Edits (all additive; nothing reordered or reformatted):

1. `crates/btctax-core/src/tax/testonly.rs` — appended `pub fn ty2026_table()` (file 1098 →
   1282 lines). Nothing above line 1098 touched.
2. `crates/btctax-adapters/tests/shipped_tables_are_the_validated_tables.rs`
   - line 72, import: `…, ty2025_table}` → `…, ty2025_table, ty2026_table}`
   - `validated_table_for`: added one arm, `2026 => Some(ty2026_table()),`
   - `KNOWN_UNVALIDATED_TABLE_YEARS`: `&[2017, 2026]` → `&[2017]`

`validated_params_for` was **not** touched: TY2026 `FullReturnParams` are a separate artifact and
outside this brief. `full_return_for(2026)` remains governed by
`ty2026_full_return_must_stay_fail_closed`.

```
$ cargo nextest run -p btctax-adapters -E 'binary(shipped_tables_are_the_validated_tables)'
    Starting 9 tests across 1 binary (8 binaries skipped)
        PASS every_shipped_full_return_params_equal_the_ones_the_corpus_validates
        PASS the_counterpart_gate_discriminates
        PASS every_shipped_tax_table_equals_the_one_the_corpus_validates
        PASS a_single_moved_bracket_is_detected
        PASS the_qbi_phase_in_top_invariant_catches_the_published_amount_paste
        PASS the_debug_year_parser_discriminates
        PASS the_year_derivation_reds_when_a_table_is_filed_under_the_wrong_year
        PASS every_shipped_year_has_a_validated_counterpart
        PASS the_year_derivation_reds_on_a_year_outside_the_probe_window
     Summary 9 tests run: 9 passed, 0 skipped
```

`every_shipped_tax_table_equals_the_one_the_corpus_validates` is the one that now compares
TY2026, and `every_shipped_year_has_a_validated_counterpart` is the ratchet — it would have red
on the stale `2026` entry had the excuse list not been trimmed.

## 8. Left for the controller (not fixed here — out of the minimal-edit scope)

Prose in the test file's header still describes the pre-2026 state and is now stale. It was left
alone deliberately: another agent is editing TY2017 into the same two files, and the brief scopes
these edits to one function, one match arm, one list entry.

- File header line 44: "TY2017, TY2025 and TY2026 ship … with **no validated counterpart**" —
  TY2025 and now TY2026 have one.
- Header table lines 57–68 and the "TY2025 is the live exposure" sentence — superseded.
- Comment above the const, lines ~759–761: "only **TY2024** has a validated counterpart" — now
  TY2024, TY2025 and TY2026 do; TY2017 alone remains.
- §2 banner comment, line ~755: "Measured 2026-09-05: `btctax_core::tax::testonly` defines exactly
  `ty2024_params` … and `ty2024_table` … — no other year" — now also `ty2025_table` and
  `ty2026_table`.

One sweep after both year-transcriptions land will fix all of these without either agent
clobbering the other.
