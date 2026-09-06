# Adjudication 2 — the TY2017 and TY2026 validated tables

**Verdict: ACCEPT BOTH. 0 Critical / 0 Important / 2 Minor / 3 Nit.**
Every one of the 78 compared figures (39 per year: 28 ordinary bracket edges + 8 §1(h)
breakpoints + 3 scalars) was verified against the primary source text, not against
`tax_tables.rs`. **Neither transcription is an echo** — positive evidence below, §3.
TY2017 is correctly **pre-TCJA** (10/15/25/28/33/35/39.6). Both years plant-and-red
verified. Critical: **EMPTY.** Important: **EMPTY.**

Adjudicator opened: `legal/text/irs-guidance/RevProc_2016-55.txt`,
`RevProc_2025-32.txt`, `RevProc_2024-40.txt`,
`legal/text/federal-register/SSA_COLA_Determinations_{2017,2026}.txt`,
`legal/primary-sources/statute-irc/26USC_s1.html`,
`crates/btctax-core/src/tax/testonly.rs`,
`crates/btctax-adapters/src/tax_tables.rs`,
`crates/btctax-adapters/tests/shipped_tables_are_the_validated_tables.rs`.

---

## 1. ★★ Pre-TCJA check — TY2017 (checked FIRST, as briefed)

`ty2017_table()` reads **10 / 15 / 25 / 28 / 33 / 35 / 39.6**, at pre-TCJA
thresholds. It is NOT the 10/12/22/24/32/35/37 schedule. **PASS.**

The tell that settles it independently of the rate list: the 33%→35% breakpoint is
**$416,700 for Single, MFJ *and* HoH alike** — a shared figure that exists in no
post-TCJA year. Source, `RevProc_2016-55.txt`:

```
TABLE 1 - Section 1(a) - Married Individuals Filing Joint Returns and Surviving Spouses
Over $233,350 but                   $52,222.50 plus 33% of
not over $416,700                   the excess over $233,350

TABLE 2 - Section 1(b) – Heads of Households
Over $212,500 but                $49,816.50 plus 33% of
not over $416,700                the excess over $212,500

TABLE 3 - Section 1(c) – Unmarried Individuals (other than Surviving Spouses and Heads of Households)
Over $191,650 but                $46,643.75 plus 33% of
not over $416,700                the excess over $191,650
```

---

## 2. Figure-by-figure verification against the source text

### 2.1 TY2017 ordinary brackets — Rev. Proc. 2016-55 **§3.01** (all 28 edges)

Top and bottom edge pasted per status, as briefed; all interior edges were read off the
same blocks and agree.

**MFJ — TABLE 1 (§1(a)):** `0, 18650, 75900, 153100, 233350, 416700, 470700` ✓
```
Not over $18,650                    10% of the taxable income
...
Over $470,700                       $131,628 plus 39.6% of
                                    the excess over $470,700
```

**HoH — TABLE 2 (§1(b)):** `0, 13350, 50800, 131200, 212500, 416700, 444550` ✓
```
Not over $13,350                 10% of the taxable income
...
Over $444,550                    $126,950 plus 39.6% of
                                 the excess over $444,550
```

**Single — TABLE 3 (§1(c)):** `0, 9325, 37950, 91900, 191650, 416700, 418400` ✓
```
Not over $9,325                  10% of the taxable income
...
Over $418,400                    $121,505.25 plus 39.6% of
                                 the excess over $418,400
```

**MFS — TABLE 4 (§1(d)):** `0, 9325, 37950, 76550, 116675, 208350, 235350` ✓
```
Not over $9,325                   10% of the taxable income
...
Over $235,350                     $65,814 plus 39.6% of
                                  the excess over $235,350
```

### 2.2 TY2017 §1(h) breakpoints — DERIVED, and the derivation rule verified verbatim

`grep -in "capital gain" RevProc_2016-55.txt` → **zero hits**. The procedure prints no
capital-gains section, so these 8 figures cannot be transcribed. The transcription
derives them from statute. Both quoted rules verified verbatim in
`legal/primary-sources/statute-irc/26USC_s1.html`:

```
(B) 0 percent of so much of the adjusted net capital gain (or, if less, taxable income)
as does not exceed the excess (if any) of— (i) the amount of taxable income which would
(without regard to this paragraph) be taxed at a rate below 25 percent, over ...
```
```
(C) 15 percent of the lesser of— ... (ii) the excess of— (I) the amount of taxable income
which would (without regard to this paragraph) be taxed at a rate below 39.6 percent, over ...
```

And the same file confirms §1(j)(5) is what replaces those phrases post-TCJA
(*"by substituting 'below the maximum zero rate amount' for 'which would … be taxed at a
rate below 25 percent'"*), i.e. §1(j)(5) did not govern TY2017. So `max_zero` = top of the
15% bracket and `max_fifteen` = top of the 35% bracket is **correct law**, and applying it
to §2.1's transcribed edges yields exactly the 8 shipped values:
`37950/418400` (S), `75900/470700` (MFJ), `37950/235350` (MFS), `50800/444550` (HoH). ✓
(See M-2 for the one residual independence caveat.)

### 2.3 TY2017 scalars

`gift_annual_exclusion = 14000` — Rev. Proc. 2016-55 **§3.37(1)** (line 933):
```
    (1) For calendar year 2017, the first $14,000 of gifts to any person (other than gifts
of future interests in property) are not included in the total amount of taxable gifts under
§ 2503 made during that year.
```

`gift_lifetime_exclusion = 5_490_000` — **§3.35** (line 915):
```
 .35 Unified Credit Against Estate Tax. For an estate of any decedent dying in
calendar year 2017, the basic exclusion amount is $5,490,000 for determining the
amount of the unified credit against estate tax under § 2010.
```

`ss_wage_base = 127200` — `SSA_COLA_Determinations_2017.txt`, page header line 1 reads
`74854   Federal Register / Vol. 81, No. 208 / Thursday, October 27, 2016 / Notices`, so
the cite **81 FR 74854** is correct:
```
The OASDI contribution and benefit
base is $127,200 for remuneration paid
in 2017 and self-employment income
earned in taxable years beginning in
```

### 2.4 TY2026 ordinary brackets — Rev. Proc. 2025-32 **§4.01** (all 28 edges)

**MFJ — TABLE 1 (§1(j)(2)(A)):** `0, 24800, 100800, 211400, 403550, 512450, 768700` ✓
```
           Not over $24,800                            10% of the taxable income
...
           Over $768,700                               $206,583.50 plus 37% of the
                                                       excess over $768,700
```

**HoH — TABLE 2 (§1(j)(2)(B)):** `0, 17700, 67450, 105700, 201750, 256200, 640600` ✓
```
        Not over $17,700                             10% of the taxable income
...
        Over $640,600                                $191,171 plus 37% of
                                                     the excess over $640,600
```
The two $25 traps confirmed against this same block: `Over $201,750 but / not over
$256,200` — HoH, versus Single's `Over $201,775 but / not over $256,225`. ✓

**Single — TABLE 3 (§1(j)(2)(C)):** `0, 12400, 50400, 105700, 201775, 256225, 640600` ✓
```
        Not over $12,400                             10% of the taxable income
...
        Over $640,600                                $192,979.25 plus 37% of
                                                     the excess over $640,600
```

**MFS — TABLE 4 (§1(j)(2)(D)):** `0, 12400, 50400, 105700, 201775, 256225, 384350` ✓
```
        Not over $12,400                             10% of the taxable income
...
         Over $384,350                                   $103,291.75 plus 37% of
                                                         the excess over $384,350
```

### 2.5 TY2026 §1(h) breakpoints — Rev. Proc. 2025-32 **§4.03**, transcribed (all 8)

```
   .03 Maximum Capital Gains Rate (§ 1(h), § 1(j)(5)). ...
                   Filing Status                       Maximum Zero     Maximum15%
                                                        Rate Amount     Rate Amount
 Married Individuals Filing Joint Returns and                 $98,900          $613,700
 Surviving Spouse
 Married Individuals Filing Separate Returns                  $49,450          $306,850
 Heads of Household                                           $66,200          $579,600
 All Other Individuals                                        $49,450          $545,500
```
Matches `Mfj 98900/613700`, `Mfs 49450/306850`, `HoH 66200/579600`,
`Single 49450/545500`. ✓ Mapping `Single ← "All Other Individuals"` is the table's own
label. ✓

### 2.6 TY2026 scalars

`gift_annual_exclusion = 19000` — **§4.42(1)**:
```
     (1) For calendar year 2026, the first $19,000 of gifts to any person (other than gifts
of future interests in property) are not included in the total amount of taxable gifts under
§ 2503 made during that year.
```

`gift_lifetime_exclusion = 15_000_000` — **§2.14**, and the transcription's claim that this
is statutory rather than a §4 inflation item is confirmed by the text itself:
```
   .14 Section 70106 of the OBBBA amends § 2010(c)(3) by increasing the basic
exclusion amount to $15,000,000 for calendar year 2026. ... These numbers are adjusted
for inflation for taxable years beginning after December 31, 2026.
```

`ss_wage_base = 184500` — `SSA_COLA_Determinations_2026.txt`; page header line 1 reads
`Federal Register / Vol. 90, No. 210 / Monday, November 3, 2025 / Notices   49047`, and the
determination sentence sits after the `49050` header, so **90 FR 49047, 49050** is correct:
```
  The OASDI contribution and benefit
base is $184,500 for remuneration paid
in 2026 and self-employment income
earned in tax years beginning in 2026.
```

### 2.7 The report's collateral claims about Rev. Proc. 2025-32 (spot-checked, all true)

`SECTION 3. 2025 ADJUSTED ITEMS AS MODIFIED, SUPERSEDED OR SUPPLEMENTED` — §3.01 removes
Rev. Proc. 2024-40 §2.15(1) with the 31,500/23,625/15,750/15,750 table; §3.02 removes §2.25
with 2,500,000/4,000,000. Both are **2025** items and touch nothing in the TY2026 table. ✓

---

## 3. Echo hunt — NEGATIVE, with positive evidence

Agreement alone proves nothing, so three independent tests were run.

**(a) `source` strings differ on both sides, both years.** Required, since `compare_tables`
excludes `source` from equality.

| year | shipped (`tax_tables.rs`) | validated (`testonly.rs`) |
|---|---|---|
| 2017 | `"Rev. Proc. 2016-55 §2.01/§2.03 (TY2017, pre-TCJA 10/15/25/28/33/35/39.6%); SSA 2016-10-18 (ss_wage_base $127,200)"` | `"TEST-TY2017 (Rev. Proc. 2016-55 §3.01 TABLES 1-4 = §1(a)-(d); §1(h)(1)(B)-(C) breakpoints derived from those tables; §3.37(1); §3.35; SSA 81 FR 74854)"` |
| 2026 | `"Rev. Proc. 2025-32 §4.01/§4.03 + §4.42 (TY2026); §2010(c)(3) basic exclusion $15,000,000 per OBBBA Pub. L. 119-21 §70106; SS wage base $184,500 per SSA (Fed. Reg. 2025-11-03)"` | `"TEST-TY2026 (Rev. Proc. 2025-32 §4.01 Tables 1-4, §4.03, §4.42(1), §2.14; 90 FR 49047)"` |

**(b) ★★ The decisive one — the validated side carries figures that DO NOT EXIST in
`tax_tables.rs`.** The transcriptions cite the procedures' *cumulative-tax* column, a second
encoding of the same breakpoints that the shipped file never records. Counted:

| figure | in `tax_tables.rs` | in `testonly.rs` | verified in procedure |
|---|---|---|---|
| `117,202` (HoH 2017) | **0** | 3 | `Over $416,700 ... $117,202.50 plus 35% of` |
| `121,505` (Single 2017) | **0** | 1 | `$121,505.25 plus 39.6% of` |
| `56,364` (MFS 2017) | **0** | 1 | `Over $208,350 ... $56,364 plus 35% of` |
| `39,207` (HoH 2026) | **0** | 2 | `Over $201,750 but ... $39,207 plus 32% of` |
| `56,631` (HoH 2026) | **0** | 2 | `Over $256,200 but ... $56,631 plus 35% of` |

A copy of `tax_tables.rs` could not have produced any of these, and each one *arithmetically
pins* the very edge it annotates (e.g. `49,816.50 + 0.33 × (416,700 − 212,500) = 117,202.50`).
This is the strongest available witness that the transcriptions came from the procedures.

**(c) Structure differs.** Different helper (`bracket(…)` vs `br(…)`), different insertion
order for the 2017 LTCG map (validated `Single, Mfj, Mfs, HoH`; shipped `Single, Mfj, HoH,
Mfs`), different comment scaffolding, and the validated side cites section numbers the
shipped side does not use (see M-1 — the two sides disagree on the *citations* while agreeing
on every *figure*, which is itself incompatible with a copy).

---

## 4. Gate wiring and the planted-defect kills

`crates/btctax-adapters/tests/shipped_tables_are_the_validated_tables.rs:775`
```rust
    const KNOWN_UNVALIDATED_TABLE_YEARS: &[i32] = &[];
```
Both 2017 and 2026 removed; the list is now empty. `validated_table_for` (line 221) has all
four arms: `2017 => Some(ty2017_table())`, `2024`, `2025`, `2026 => Some(ty2026_table())`.

**Not a quiet drop.** The shipped year set is not hand-written: `derive_shipped_years` reads
the years out of the bundle's own `Debug` rendering, cross-checks them against what
`table_for` answers over `PROBE_LO..=PROBE_HI`, *and* checks each value's self-declared
`year` field — three ways a dropped or mis-filed year reds instead of passing vacuously. The
mutation kills below independently prove 2017 and 2026 are genuinely in the compared set.

Baseline: `cargo nextest run --locked -p btctax-adapters --test shipped_tables_are_the_validated_tables`
→ **9 tests run: 9 passed, 0 skipped**.

**Kill 1 — TY2017** (`testonly.rs:1392`, `dec!(116675)` → `dec!(116676)`):
```
assertion `left == right` failed: TY2017 Mfs ordinary bracket 4: shipped (116675, 0.33) vs
validated (116676, 0.33) — the binary would tax a filer differently from every test that
says it is correct
Summary [0.004s] 9 tests run: 8 passed, 1 failed, 0 skipped
```

**Kill 2 — TY2026** (`testonly.rs:1215`, `dec!(201750)` → `dec!(201751)` — the $25 HoH trap):
```
assertion `left == right` failed: TY2026 HoH ordinary bracket 4: shipped (201750, 0.32) vs
validated (201751, 0.32) — the binary would tax a filer differently from every test that
says it is correct
Summary [0.004s] 9 tests run: 8 passed, 1 failed, 0 skipped
```

Each reds **naming its own year**. Restored from a `cp` backup;
`diff -q` reports byte-identical, and the suite is back to **9 passed, 0 skipped**.
No `git` was run; no `make check` was run.

---

## 5. Findings

### Critical — **EMPTY.**
No wrong figure, and no echo. All 78 compared figures verified against primary source.

### Important — **EMPTY.**
Every figure on the validated side carries a section citation, and every citation resolves.
See §2 — nothing is unsourced.

---

### M-1 (Minor, documentation, **pre-existing in `tax_tables.rs`**) — the mis-citation is real and BROADER than D-1 reported

The transcriber's **D-1 is CONFIRMED**: `ty2017()`'s section numbers do not exist in
Rev. Proc. 2016-55. Machine-checked — `SECTION 2. CHANGES` runs lines 163–277 and contains
subsections **.01 through .08 only**, so §2.35 and §2.41 do not exist at all; the rate tables
are §3.01, the gift exclusion §3.37(1), the basic exclusion §3.35.

**Cause confirmed, not guessed.** Rev. Proc. **2024-40** (TY2025, the adjacent block in the
same file) genuinely numbers its adjusted items under SECTION 2:
```
23:SECTION 2. 2025 ADJUSTED ITEMS
132:   .01 Tax Rate Tables. For taxable years beginning in 2025, the tax rate tables under § 1
240:   .03 Maximum Capital Gains Rate (§ 1(h), § 1(j)(5)). For taxable years beginning in 2025,
```
So the TY2017 block's `§2.01/§2.03` is a carry-over from the TY2025 block above it.

**What D-1 missed, and why it matters for the fix.** D-1 enumerated eight sites and named one
KAT line (1249). The mis-citation actually spans **thirteen** sites in `tax_tables.rs`, eight
of them inside the in-crate KAT:
```
173, 188, 204, 220, 236, 252, 286, 290, 296   (the ty2017() fn — D-1 found these, and 173)
1234, 1239, 1249, 1306, 1310, 1315, 1328, 1347 (the KAT module — D-1 named only 1249)
```
Line 1306/1310/1328 are *assertion messages* reading `"must match Rev. Proc. 2016-55 §2.01
verbatim"` and `"§2.03 verbatim"` — a failing test would print a citation to a section that
does not exist. A sweep guided by D-1's list alone would leave nine sites behind.

Not fixed here, for the same reason D-1 gave: this adjudication must not edit either artifact.
**Severity stays Minor** — every figure is correct, and the correctly-cited counterpart now
exists in `testonly.rs`, so the recovery path D-1 worried about is closed by this very branch.

### M-2 (Minor, evidence) — TY2017's 8 §1(h) breakpoints are the one place the two artifacts are not independent, and the claimed corroboration is not in-repo

For the other 70 figures the two sides are genuinely independent. For TY2017's LTCG
breakpoints they cannot be: nothing prints them, so **both** sides derive them, and both
derive them by the same rule (shipped comment, line 252: *"0% through top of the 15% ordinary
bracket; 20% from the 39.6% ordinary threshold"*). Agreement there tests transcription of the
ordinary edges, not the rule.

The transcription anticipates this and offers corroboration — the doc comment claims the
values *"reproduce the 2017 Qualified Dividends and Capital Gain Tax Worksheet's own printed
constants"*. **That worksheet is not committed in this repo** (searched; no TY2017 1040
worksheet artifact exists), so as written the corroboration cannot be checked by a future
reader following the citation.

**Why this does not escalate.** The rule itself is verified law, from a committed primary
source, quoted verbatim in §2.2 above: §1(h)(1)(B)(i) *"below 25 percent"* and
§1(h)(1)(C)(ii)(I) *"below 39.6 percent"*, with §1(j)(5) confirmed inapplicable to TY2017.
The derivation is therefore correct on the authority actually held in-repo, and the doc
comment's own §1(h) citations are the ones that carry it. The defect is only that one
supporting sentence points outside the repo. Suggested wording fix: keep the §1(h) citation as
the authority and mark the worksheet line as an uncommitted cross-check.

---

### Nits

**N-1 — D-1 overstates its own claim.** D-1 says *"there is NO §2.03 or §3.03 in that
procedure at all"*. Both exist in Rev. Proc. 2016-55; neither concerns capital gains:
```
188:  .03 Section 103 of the PATH Act made permanent under § 32 the enhanced earned
423: .03 Adoption Credit. For taxable years beginning in 2017, under § 23(a)(3) the credit
```
The accurate statement is the one D-1 makes in its parenthesis and then overshoots: the
procedure prints **no capital-gains section**, so §2.03 and §3.03 exist but are about the EITC
and the adoption credit. Worth correcting because a reader who checks the literal claim finds
it false and may discount the whole (correct) finding.

**N-2 — one line-number drift in D-1.** It cites the basic-exclusion comment at line 295;
it is at **296** (`grep -n "§2\.41" crates/btctax-adapters/src/tax_tables.rs` → `296`). Lines
188/204/220/236/252/290 are exact. D-1 also omits line **173**, the `ty2017()` doc comment,
which carries the same `§2.01 … §2.03` error.

**N-3 — D-2 CONFIRMED.** Shipped cites `SSA 2016-10-18` (source string line 286, comment line
290). The committed authority is dated 27 October 2016, per the page header at line 1 of
`SSA_COLA_Determinations_2017.txt`: `74854   Federal Register / Vol. 81, No. 208 / Thursday,
October 27, 2016 / Notices`. The claim is not false — SSA announced on 18 October 2016 — but
it names a document not held in-repo, while `81 FR 74854` is. The validated side already cites
the Federal Register correctly, so this resolves itself for any future reader.

---

## 6. Explicit statements required by the brief

- **Critical: EMPTY.** No wrong figure; no echo; no test asserting a number equals itself.
- **Important: EMPTY.** No unsourced or uncited figure on either validated table.
- **Figures with no section citation: NONE.** All 39 TY2017 figures cite Rev. Proc. 2016-55
  §3.01 / §3.37(1) / §3.35, IRC §1(h)(1)(B)(i)–(C)(ii)(I), or 81 FR 74854. All 39 TY2026
  figures cite Rev. Proc. 2025-32 §4.01 / §4.03 / §4.42(1) / §2.14, or 90 FR 49047, 49050.
- **`Qss` omission is lawful and symmetric**: absent from both sides for both years
  (`grep "Qss"` over the `ty2017()` and `ty2026()` bodies returns nothing), and TABLE 1 is
  titled *"Married Individuals Filing Joint Returns and Surviving Spouses"* in both procedures.
- **Both new tables are compared, not merely present** — proved by the two planted-defect
  kills in §4, each naming its own year.
- **Scope respected**: no `git`, no `make check`, no touch of `crates/xtask/`. Only
  `-p btctax-adapters --test shipped_tables_are_the_validated_tables` was run.
