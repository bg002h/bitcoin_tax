# TY2017 validated tax table — transcription report

**Date:** 2026-09-05
**Artifact produced:** `btctax_core::tax::testonly::ty2017_table()`
(`crates/btctax-core/src/tax/testonly.rs`, appended at the end of the file)
**Wired into:** `crates/btctax-adapters/tests/shipped_tables_are_the_validated_tables.rs`
— import, `validated_table_for(2017)` match arm, and `KNOWN_UNVALIDATED_TABLE_YEARS`
went `&[2017]` → `&[]`.

**Primary sources, both committed in-repo:**

| what | file |
|---|---|
| Rev. Proc. 2016-55, 2016-45 I.R.B. 707 | `legal/text/irs-guidance/RevProc_2016-55.txt` |
| SSA COLA determinations for 2017, 81 FR 74854 (Vol. 81 No. 208, Thu 27 Oct 2016) | `legal/text/federal-register/SSA_COLA_Determinations_2017.txt` |
| 26 U.S.C. §1 (for §1(h), which the Rev. Proc. does not reproduce) | `legal/primary-sources/statute-irc/26USC_s1.html` |

**Method.** Every figure was read from the `pdftotext -layout` text layer of the
revenue procedure. `crates/btctax-adapters/src/tax_tables.rs` was **not opened
until every figure below was already written**, and is read only in the
DISAGREEMENTS section at the bottom. Nothing was copied from it.

★★ **PRE-TCJA.** The schedule is 10 / 15 / 25 / 28 / 33 / 35 / 39.6 percent, read
from the §1(a)–(d) tables. §1(j) was added by TCJA §11001 for taxable years
beginning **after** 2017 and does not exist for TY2017; a TY2017 transcription
reading 10/12/22/24/32/35/37 would be wrong regardless of what it agreed with.

---

## 1. Ordinary-income schedules — Rev. Proc. 2016-55 **§3.01** ("Tax Rate Tables")

Note the section number: the 2017 adjusted items are in **SECTION 3**, not
section 2 (§2 of this procedure is "CHANGES", about the PATH Act). See
DISAGREEMENTS D-1.

Thresholds below are the value at which the rate **starts**, i.e. the "Over $X"
figure on each row of the procedure.

### §3.01 TABLE 3 — §1(c) — Unmarried Individuals (other than Surviving Spouses and Heads of Households) → `FilingStatus::Single`

| rate | starts at |
|---|---|
| 10% | $0 |
| 15% | $9,325 |
| 25% | $37,950 |
| 28% | $91,900 |
| 33% | $191,650 |
| 35% | $416,700 |
| 39.6% | $418,400 |

### §3.01 TABLE 1 — §1(a) — Married Individuals Filing Joint Returns and Surviving Spouses → `FilingStatus::Mfj`

| rate | starts at |
|---|---|
| 10% | $0 |
| 15% | $18,650 |
| 25% | $75,900 |
| 28% | $153,100 |
| 33% | $233,350 |
| 35% | $416,700 |
| 39.6% | $470,700 |

### §3.01 TABLE 4 — §1(d) — Married Individuals Filing Separate Returns → `FilingStatus::Mfs`

| rate | starts at |
|---|---|
| 10% | $0 |
| 15% | $9,325 |
| 25% | $37,950 |
| 28% | $76,550 |
| 33% | $116,675 |
| 35% | $208,350 |
| 39.6% | $235,350 |

### §3.01 TABLE 2 — §1(b) — Heads of Households → `FilingStatus::HoH`

| rate | starts at |
|---|---|
| 10% | $0 |
| 15% | $13,350 |
| 25% | $50,800 |
| 28% | $131,200 |
| 33% | $212,500 |
| 35% | $416,700 |
| 39.6% | $444,550 |

### Independent confirmation — the procedure's own cumulative-tax column

Each table prints a second, redundant encoding of the same breakpoints ("$X plus
R% of the excess over $T"). All **24 interior rows** were re-derived from the
row above and all 24 agree, so a mis-keyed digit is caught by arithmetic rather
than by re-reading — the failure mode `CLAUDE.md` records for Form 6251 line 33.

TABLE 1 (MFJ):
- `18,650 × 0.10 = 1,865` ✓
- `1,865 + 0.15 × (75,900 − 18,650) = 10,452.50` ✓
- `10,452.50 + 0.25 × (153,100 − 75,900) = 29,752.50` ✓
- `29,752.50 + 0.28 × (233,350 − 153,100) = 52,222.50` ✓
- `52,222.50 + 0.33 × (416,700 − 233,350) = 112,728` ✓
- `112,728 + 0.35 × (470,700 − 416,700) = 131,628` ✓

TABLE 2 (HoH):
- `13,350 × 0.10 = 1,335` ✓
- `1,335 + 0.15 × (50,800 − 13,350) = 6,952.50` ✓
- `6,952.50 + 0.25 × (131,200 − 50,800) = 27,052.50` ✓
- `27,052.50 + 0.28 × (212,500 − 131,200) = 49,816.50` ✓
- `49,816.50 + 0.33 × (416,700 − 212,500) = 117,202.50` ✓
- `117,202.50 + 0.35 × (444,550 − 416,700) = 126,950` ✓

TABLE 3 (Single):
- `9,325 × 0.10 = 932.50` ✓
- `932.50 + 0.15 × (37,950 − 9,325) = 5,226.25` ✓
- `5,226.25 + 0.25 × (91,900 − 37,950) = 18,713.75` ✓
- `18,713.75 + 0.28 × (191,650 − 91,900) = 46,643.75` ✓
- `46,643.75 + 0.33 × (416,700 − 191,650) = 120,910.25` ✓
- `120,910.25 + 0.35 × (418,400 − 416,700) = 121,505.25` ✓ — a **$1,700-wide**
  35% bracket, the narrowest row in the procedure and the easiest to drop.

TABLE 4 (MFS):
- `9,325 × 0.10 = 932.50` ✓
- `932.50 + 0.15 × (37,950 − 9,325) = 5,226.25` ✓
- `5,226.25 + 0.25 × (76,550 − 37,950) = 14,876.25` ✓
- `14,876.25 + 0.28 × (116,675 − 76,550) = 26,111.25` ✓
- `26,111.25 + 0.33 × (208,350 − 116,675) = 56,364` ✓
- `56,364 + 0.35 × (235,350 − 208,350) = 65,814` ✓

★ Every status was read off its own table. Several MFS figures happen to be
exactly half the joint ones in TY2017 ($235,350 = ½ × $470,700; $208,350 = ½ ×
$416,700; $116,675 = ½ × $233,350; $76,550 = ½ × $153,100) — a coincidence of
this year's §1(d) construction, **not** a rule that was applied. HoH shares none
of it. Nothing below was computed from another status.

★ **A pre-TCJA property worth flagging for anyone porting this forward:** the
33%→35% breakpoint is **$416,700 for Single, MFJ *and* HoH alike**. Post-TCJA the
corresponding edge differs per status (in TY2025 HoH's 35% floor sits $25 below
Single's). An identical figure across three statuses here is correct and is not
evidence of a copy.

★ **`Qss` is deliberately absent**, matching `ty2024_table` and `ty2025_table`:
§1(a) gives a surviving spouse the joint schedule (TABLE 1 is titled "Married
Individuals Filing Joint Returns **and Surviving Spouses**") and `TaxTable::key`
normalises `Qss → Mfj` at lookup. The ratchet accepts that only because BOTH
sides omit it; it is checked, not assumed. Confirmed by the green run below.

---

## 2. §1(h) long-term capital-gain breakpoints — **DERIVED, and here is why**

★★ **Rev. Proc. 2016-55 contains NO capital-gains rate section.** `grep -i
"capital gain"` over the whole text returns nothing. That is not an omission: for
pre-TCJA years §1(h) defines its breakpoints **by reference to the ordinary
brackets** rather than by its own inflation-indexed dollar figures. §1(j)(5),
which gives them independent amounts (and which is what §2.03/§3.03 of the
TY2024/TY2025 procedures print), did not yet exist.

So these four rows are the one place in this table where a figure is derived
rather than transcribed. The derivation is a statutory quotation applied to
figures transcribed above, read from
`legal/primary-sources/statute-irc/26USC_s1.html`:

- **§1(h)(1)(B)(i)** — 0% applies up to *"the amount of taxable income which
  would (without regard to this paragraph) be taxed at a rate **below 25
  percent**"* ⇒ `max_zero` = **the top of the 15% bracket** = each table's
  second-row "not over" figure.
- **§1(h)(1)(C)(ii)(I)** — 15% applies up to *"the amount of taxable income which
  would (without regard to this paragraph) be taxed at a rate **below 39.6
  percent**"* ⇒ `max_fifteen` = **the top of the 35% bracket** = each table's
  sixth-row "not over" figure (equivalently, where 39.6% starts).

| status | `max_zero` (0% ceiling) | source row | `max_fifteen` (15% ceiling) | source row |
|---|---|---|---|---|
| Single | $37,950 | TABLE 3 row 2 | $418,400 | TABLE 3 row 6 |
| Mfj | $75,900 | TABLE 1 row 2 | $470,700 | TABLE 1 row 6 |
| Mfs | $37,950 | TABLE 4 row 2 | $235,350 | TABLE 4 row 6 |
| HoH | $50,800 | TABLE 2 row 2 | $444,550 | TABLE 2 row 6 |

**Independent confirmation.** These eight figures are exactly the constants the
IRS itself prints in the 2017 *Qualified Dividends and Capital Gain Tax
Worksheet* (2017 Form 1040 instructions, line 44) — "$37,950 if single or married
filing separately, $75,900 if married filing jointly or qualifying widow(er),
$50,800 if head of household", and $418,400 / $470,700 / $235,350 / $444,550 for
the 20% step. That worksheet is a second, independently-published encoding of the
same statutory derivation.

**Semantics check.** `max_zero` / `max_fifteen` are *inclusive* ceilings — the
same meaning as the "Maximum Zero Rate Amount" / "Maximum 15% Rate Amount"
columns that `ty2024_table` transcribes from Rev. Proc. 2023-34 §3.03. Taxable
income of exactly $37,950 (Single) sits in the 15% bracket, hence at 0% on the
gain; the IRS worksheet's "enter the smaller of … $37,950" carries the same
inclusivity.

---

## 3. Scalars

| field | value | citation |
|---|---|---|
| `gift_annual_exclusion` | **$14,000** | Rev. Proc. 2016-55 **§3.37(1)**: *"For calendar year 2017, the first $14,000 of gifts to any person (other than gifts of future interests in property) are not included in the total amount of taxable gifts under § 2503 made during that year."* |
| `gift_lifetime_exclusion` | **$5,490,000** | Rev. Proc. 2016-55 **§3.35** (*Unified Credit Against Estate Tax*): *"For an estate of any decedent dying in calendar year 2017, the basic exclusion amount is $5,490,000 for determining the amount of the unified credit against estate tax under § 2010."* |
| `ss_wage_base` | **$127,200** | ★ **Not in the revenue procedure at all.** 81 FR 74854 (Vol. 81 No. 208, Thu 27 Oct 2016), Docket No. SSA-2016-0050, *Cost-of-Living Increase and Other Determinations for 2017*, heading "OASDI Contribution and Benefit Base": *"The OASDI contribution and benefit base is $127,200 for remuneration paid in 2017 and self-employment income earned in taxable years beginning in 2017."* The same notice shows the computation (`$60,600 × NAWI 2015 ÷ NAWI 1992 = $127,086.27`, rounded to the nearest $300 ⇒ $127,200, which exceeds the 2016 base of $118,500). |
| `year` | 2017 | — |
| `source` | `"TEST-TY2017 (Rev. Proc. 2016-55 §3.01 TABLES 1-4 = §1(a)-(d); §1(h)(1)(B)-(C) breakpoints derived from those tables; §3.37(1); §3.35; SSA 81 FR 74854)"` | Deliberately **different** from the shipped string; `compare_tables` excludes `source` from the equality precisely so that identical provenance strings would stand out as the signature of a copy. |

---

## 4. Verification run

```
$ cargo nextest run -p btctax-adapters -E 'binary(shipped_tables_are_the_validated_tables)'
    Starting 9 tests across 1 binary (8 binaries skipped)
        PASS every_shipped_tax_table_equals_the_one_the_corpus_validates
        PASS every_shipped_year_has_a_validated_counterpart
        PASS every_shipped_full_return_params_equal_the_ones_the_corpus_validates
        PASS a_single_moved_bracket_is_detected
        PASS the_counterpart_gate_discriminates
        PASS the_debug_year_parser_discriminates
        PASS the_year_derivation_reds_when_a_table_is_filed_under_the_wrong_year
        PASS the_year_derivation_reds_on_a_year_outside_the_probe_window
        PASS the_qbi_phase_in_top_invariant_catches_the_published_amount_paste
     Summary [0.004s] 9 tests run: 9 passed, 0 skipped
```

**B1 — seen red once.** A green comparison proves nothing until it has been
watched discriminating on TY2017 specifically. `dec!(116675)` in
`ty2017_table()`'s MFS schedule was mutated to `dec!(116676)` and the suite red:

```
assertion `left == right` failed: TY2017 Mfs ordinary bracket 4: shipped (116675, 0.33)
vs validated (116676, 0.33) — the binary would tax a filer differently from every test
that says it is correct
```

The mutation was reverted from a file copy (not `git checkout`) and the binary
re-run green. So the TY2017 arm of the equality is live, not merely present.

---

## 5. DISAGREEMENTS

`crates/btctax-adapters/src/tax_tables.rs` was read only after everything above
was written.

### Numeric: **NONE.**

All **28** ordinary bracket edges (7 × 4 statuses), all **8** §1(h) breakpoints
(2 × 4 statuses), and all **3** scalars agree exactly with the shipped
`ty2017()`. Both artifacts also omit `Qss`, which is the one lawful absence.
Two independent transcriptions of the same procedure reproducing the same 39
figures is what says the reading is right.

### D-1 (documentation, **wrong section numbers in the shipped citation**)

`tax_tables.rs::ty2017()` cites section numbers that **do not exist in Rev. Proc.
2016-55**:

| shipped citation | where the figure actually is |
|---|---|
| `source: "Rev. Proc. 2016-55 §2.01/§2.03 …"` (line 286) | rate tables are **§3.01**; **there is no §2.03 or §3.03 in this procedure at all** |
| `// §2.01 Table 3 — Single …` and the three sibling comments (lines 188, 204, 220, 236) | **§3.01** TABLE 3 / 1 / 2 / 4 |
| `// §2.03 — §1(h) LTCG breakpoints` (line 252) | no such section — derived from §1(h)(1)(B)–(C) applied to the §3.01 tables |
| `// … Rev. Proc. 2016-55 §2.35(1) (TY2017 = $14,000)` (line 290) | **§3.37(1)** |
| `// … Rev. Proc. 2016-55 §2.41 (TY2017 = $5,490,000)` (line 295) | **§3.35** |
| `// §2.01 Tables 1–4 — (lower, rate) pairs, verbatim.` in the KAT (line 1249) | **§3.01** |

Section 2 of Rev. Proc. 2016-55 is titled **CHANGES** and discusses the PATH Act;
it contains no dollar tables. The 2017 adjusted items are all in **SECTION 3**.
The numbering appears to have been carried over from the TY2024/TY2025
procedures, which do use §2.01/§2.03 (Rev. Proc. 2024-40) or §3.01/§3.03 (Rev.
Proc. 2023-34).

**Why this is worth fixing even though every number is right.** The §2.03
citation asserts that a "Maximum Capital Gains Rate" section exists in this
procedure, and it does not — §1(h) had no separately-indexed breakpoints
pre-TCJA. A future verifier who follows that citation will find nothing, and the
most likely recovery is to reach for §2.03 of a *different* year's procedure,
which is a wrong-year paste in an artifact whose whole purpose is to prevent
one. Under this repo's own rule — *"a figure with no citation is remembered, not
transcribed"* — a citation that resolves to nothing is worse than none, because
it looks checked.

**Severity: Minor** (documentation only; no figure moves, nothing a filer
receives changes). Suggested fix: `§2.01` → `§3.01`, `§2.35(1)` → `§3.37(1)`,
`§2.41` → `§3.35`, and replace `§2.03` with an explicit note that the
breakpoints are derived from §1(h)(1)(B)(i) and §1(h)(1)(C)(ii)(I) because the
procedure prints none. **Not done here** — this report's mandate was to
transcribe, and edits to `tax_tables.rs` would defeat the independence the
comparison rests on.

### D-2 (documentation, SSA date)

Shipped cites the wage base as *"SSA 2016-10-18"* (source string) and *"SSA
announced 2016-10-18"* (line 292). The committed authority in this repo is the
**Federal Register** determination, **81 FR 74854, published Thursday 27 October
2016**. 2016-10-18 is the date of SSA's press release / fact sheet, so the claim
is not false, but it names the weaker document and one not held in-repo.
**Severity: Nit.** `ty2017_table()` cites the FR notice instead.

### D-3 (observation, not a defect in either artifact)

`tax_tables.rs` already carried a full 28-edge in-crate KAT,
`ty2017_table_matches_rev_proc_2016_55` (line 1245) — per the test file's own
header table, TY2017 was 28/28 pinned in-crate, unlike TY2025's 8/28. So TY2017
was the *least* exposed of the three unwitnessed years on figures. What it
lacked, and what this transcription supplies, is **independence**: that KAT lives
in the same file as the data it checks and was written by whoever wrote the data.
`ty2017_table()` is a second artifact derived from the primary source, which is a
different kind of witness. Worth recording so the closure is not overclaimed.
