# Recon — TY2026 port, lens: CONSTANTS AND INDEXING

**Headline.** The indexed half of TY2026 is **already done and armed** — `BundledTaxTables` carries a
verified `ty2026()` and the crypto-delta path computes 2026 today — while the whole *full-return*
constant set for 2026 is **unbuilt but almost entirely knowable now** (Rev. Proc. 2025-32 published
Oct 2025); the single genuinely unknown figure I can identify is `AmtParams::mfs_kicker_rate`, and the
single largest *silent-wrong-number* risk is that `schedule_1a_params(2026)` **already returns 2025's
dollar amounts today**, resting on an in-repo claim that nothing in Schedule 1-A is indexed that no
archived primary source in this worktree supports.

Scope note: this lens answers only "which numbers change, by what rule, and when can we know them."
Year-seams in code, form artifacts, AcroForm maps, oracles and the port machinery belong to the other
five lenses and are touched only where a constant lands in them.

---

## 1. The three-way split, measured against the source

### (a) STATUTORY — fixed in the Code, do NOT index, do NOT change for TY2026

All in `crates/btctax-core/src/tax/tables.rs`, which states the separation in its own module header
(`tables.rs:1-9`) and enforces it by keeping these out of any per-year table.

| constant | file:line | value | cite |
|---|---|---|---|
| `NIIT_RATE` | `tables.rs:134` | `0.038` | §1411(a)(1) |
| `EMPLOYEE_OASDI_RATE` | `tables.rs:140` | `0.062` | §3101(a) |
| `SE_RATE_SS` | `tables.rs:146` | `0.124` | §1401(a) |
| `SE_RATE_MEDICARE` | `tables.rs:151` | `0.029` | §1401(b)(1) |
| `SE_RATE_ADDL_MEDICARE` | `tables.rs:158` | `0.009` | §1401(b)(2) |
| `SE_NET_EARNINGS_FACTOR` | `tables.rs:164` | `0.9235` | §1402(a)(12) |
| `se_addl_medicare_threshold` | `tables.rs:178` | 250,000 / 125,000 / 200,000 | §1401(b)(2)(A) |
| `niit_threshold` | `tables.rs:226` | 250,000 / 200,000 / 125,000 | §1411(b) |
| `loss_limit` | `tables.rs:240` | 3,000 / 1,500 MFS | §1211(b) |
| `QUALIFIED_APPRAISAL_THRESHOLD` | `tables.rs:190` | 5,000 | §170(f)(11)(C) |
| `CWA_SUBSTANTIATION_THRESHOLD` | `tables.rs:198` | 250 | §170(f)(8)(A) |
| `APPRAISAL_ATTACHMENT_THRESHOLD` | `tables.rs:215` | 500,000 | §170(f)(11)(D) |

Plus five more statutory dollar amounts that live outside `tables.rs`, found by sweeping every
non-test source line for a money literal (`awk` over `crates/**/src/*.rs`, skipping from the first
`#[cfg(test)]`):

| constant | file:line | value | cite |
|---|---|---|---|
| `SE_FLOOR` | `crates/btctax-forms/src/schedule_se.rs:45` | 400 | §1402(b)(2) |
| `SE_6017_FLOOR` | `crates/btctax-core/src/tax/return_1040.rs:1512` | 400 | §6017 |
| student-loan deduction cap | `return_1040.rs:1279` (`paid.min(dec!(2500))`) | 2,500 | §221(b)(1) |
| `SCHEDULE_B_THRESHOLD` | `return_1040.rs:3479` | 1,500 | Schedule B part-III trigger (form-set, not indexed) |
| `FORM_8283_THRESHOLD` | `crates/btctax-core/src/tax/printed.rs:188` | 500 | §170(f)(11)(B) |

**Verdict for TY2026: none of the above moves.** Two of them, however, are statutory constants stored
*inside* the per-year indexed struct, which is where a mechanical "index everything for the new year"
pass goes wrong:

- `FullReturnParams::ftc_ceiling` (`tables.rs:474`) — §904(j)'s $300/$600, statutory. **No invariant
  protects it.**
- `FullReturnParams::qbi_phase_in_range_{unmarried,married}` (`tables.rs:499`+) — §199A(b)(3)(B)(ii)(II),
  statutory $50,000/$100,000. **This one is protected**: `qbi_phase_in_top()` (`tables.rs:543`) exists
  precisely so `threshold + width` must equal the Rev. Proc.'s published "phase-in range amount",
  which breaks loudly if the width is indexed or if the published top is pasted into the width field.

### (b) INDEXED — moves every year by Rev. Proc. / SSA

**Already bundled and verified for TY2026** (`crates/btctax-adapters/src/tax_tables.rs:580` `ty2026()`,
wired at `:78`):

- ordinary brackets, all four statuses — Rev. Proc. 2025-32 §4.01 Tables 1–4
- §1(h) LTCG breakpoints, all four statuses — §4.03
- `gift_annual_exclusion` = 19,000 — §4.42(1)
- `ss_wage_base` = 184,500 — SSA determination, Fed. Reg. 2025-11-03
- `gift_lifetime_exclusion` = 15,000,000 — flat OBBBA figure, *not* a §1(f) item for 2026 (first
  indexed 2027)

That is **7 of 7 `TaxTable` fields**, so the crypto-delta path is already TY2026-armed; the CLI test
that used to prove "2026 is not bundled" has been re-pointed to 2027
(`crates/btctax-cli/tests/tax_report.rs:837-855`).

**Not yet bundled for any year past 2024** — `FullReturnParams` (17 fields, `tables.rs:453`) plus its
nested `AmtParams` (15 fields, `tables.rs:265`). `BundledFullReturnTables::load()` inserts **2024
only** (`tax_tables.rs:101`), and both later years are held shut by name:
`ty2025_full_return_must_stay_fail_closed_until_complete` (`:814`) and
`ty2026_full_return_must_stay_fail_closed` (`:931`).

Indexed fields needing a TY2026 value, with where the number lives:

| field | rule | TY2026 source |
|---|---|---|
| `std_deduction` ×4 | §63(c), indexed — but **per-field "later of Rev. Proc. or OBBBA" applies** | Rev. Proc. 2025-32 |
| `std_aged_blind_{married,unmarried}` | §63(f), indexed | Rev. Proc. 2025-32 |
| `dependent_std_floor` | §63(c)(5)(A), indexed (2024 1,300 → 2025 1,350) | Rev. Proc. 2025-32 |
| `dependent_std_earned_addon` | §63(c)(5)(B), **indexed but was 450 in BOTH 2024 and 2025** | Rev. Proc. 2025-32 |
| `kiddie_unearned_threshold` | §1(g)(4), indexed (2,600 → 2,700) | Rev. Proc. 2025-32 / `i1040gi--2026` |
| `elective_deferral_limit` | §402(g)(1), indexed (23,000 → 23,500) | the annual retirement-limits Notice (Nov 2025) |
| `qbi_ti_threshold_{unmarried,married}` | §199A(e)(2), indexed (191,950/383,900 → 197,300/394,600) | Rev. Proc. 2025-32 |
| `student_loan_phaseout_{unmarried,married}` | §221(b)(2), indexed | Rev. Proc. 2025-32 / `i1040gi--2026` |
| `AmtParams` exemption / phase-out start / 26–28% breakpoint / subtrahend | §55(d)(4), indexed — **but OBBBA also reset §55(d)(3) for 2026** | Rev. Proc. 2025-32 §55(d); see §2 below |

★ **`dependent_std_earned_addon` is the trap in this table.** It was $450 for TY2024 and $450 for
TY2025 — an *indexed* figure that happened not to move. A copy-forward pass reads "unchanged last
year" as "not indexed" and carries it silently. The same shape occurs benignly in `ty2026()`'s
`gift_annual_exclusion` (19,000 in both 2025 and 2026, §2503(b)(2) rounds to the nearest $1,000).
**A figure being unchanged is not evidence that it is unindexed.**

### (c) EXPIRING — a hard stop, not a lookup

`Schedule1aParams` (`tables.rs:1017`), the four OBBBA additional deductions:

```
crates/btctax-core/src/tax/tables.rs:1088
pub fn schedule_1a_params(year: i32) -> Option<Schedule1aParams> {
    if !(2025..=2028).contains(&year) {
        return None;
    }
```

- §§224, 225, 163(h)(4), 151(d)(5) sunset after **TY2028** (§224(f), §225(f), §163(h)(4)(F),
  §151(d)(5)(D) per the doc comment at `tables.rs:1081-1085`), so TY2029+ must fail closed.
- The doc states plainly that **nothing here is indexed**, and the test
  `schedule_1a_exists_only_for_2025_through_2028` (`tables.rs:1139`) *enforces* it: every year in
  2025..=2028 must be field-for-field identical to 2025.
- **Consequence: TY2026's Schedule 1-A constants already exist and already pass.** This is the one
  place the TY2026 port is finished before it started — and, for the same reason, the one place the
  code will emit a 2026 dollar figure today with nobody having re-read the statute. See §3, W1.

Also expiring / reverting, and relevant to a TY2026 build: §164(b)'s OBBBA regime is a **temporary
schedule with a statutory escalator through 2029 and a reversion after it**, which is why `SaltLimitation`
(`tables.rs:323`) is a `#[non_exhaustive]` enum whose variants are *instruments*, not parameter sets —
`FlatCap` for 2024 and `Worksheet2025` for 2025 ask genuinely different questions. TY2026 will need its
own variant or an explicit proof that the 2026 worksheet is line-for-line the 2025 one with different
constants.

---

## 2. What the archived TY2026 draft Form 6251 actually settles

`design/forms/2026/f6251--2026-DRAFT.pdf` (sha256 `a547fc9d…`, 295,209 bytes) is in the worktree.
Extracted with `pdftotext -layout`, the form face prints:

```
 5  Exemption.
    Single or head of household . . . . . . $ 500,000 . . . . . $ 90,100
    Married filing jointly or qualifying surviving spouse 1,000,000 . . . . . 140,200
    Married filing separately . . . . . . .        500,000 . . . . .    70,100
 4  Alternative minimum taxable income. Combine lines 1b through 3. (If married filing separately and
    line 4 is more than $640,200, see instructions.)
 7  • All others: If line 6 is $244,500 or less ($122,250 or less if married filing separately), multiply
      line 6 by 26% (0.26). Otherwise, multiply line 6 by 28% (0.28) and subtract $4,890 ($2,445 if
      married filing separately) from the result.
```

Mapped onto the 15 `AmtParams` cells: **12 are printed on the form face**, 2 more are pinned by the two
§55(d)(3) identities the repo already asserts executably
(`mfs_kicker_constants_satisfy_the_two_section_55d3_identities`, `tax_tables.rs:859`), and **exactly 1
is genuinely unknown**.

Arithmetic run this session (`python3`, exact `Decimal`):

```
MFS zero-exemption @50%: 640200      <- matches the form's printed line-4 trigger
MFS zero-exemption @25%: 780400      <- does not
0.02*244500 = 4890.00                <- matches the printed 28%-bracket subtrahend
0.02*122250 = 2445.00                <- matches the MFS subtrahend
2*70100 = 140200                     <- matches the printed MFJ exemption
TY2024 check: 609350 + 66650/0.25 = 875950   (the bundled TY2024 kicker start)
TY2025 check: 626350 + 68500/0.25 = 900350   (the hand-worked TY2025 value)
```

So `exemption_phaseout_rate = 0.50` for TY2026 is **not an inferred constant** in the sense
`CLAUDE.md` forbids — it is forced by §55(d)(3)(i) from three amounts the form itself prints, and the
repo already carries the equivalence proof *and* the KAT that would red if it were wrong. Likewise
`mfs_kicker_max = 70,100` is forced by §55(d)(3)(ii).

**The one genuinely open AMT cell is `mfs_kicker_rate`** — the percentage in the flush sentence's
clause (i) ("include 25% of the excess…"). It is a *different* rule from the phase-out rate; the two
were one field until 2026-07-29 and were split precisely because 2026 moves one and not necessarily the
other (`tables.rs:290-296`). The form face does not print it; it lives in the instructions and,
underneath them, in the amended §55(d)(3) text. **This is a statute-reading question, answerable today
— not a wait-on-the-IRS question.** It bites only MFS filers with AMTI above $640,200, and a rate that
is too low understates.

★ Caveat of record: `design/ty2025/SPEC.md:67` classifies this draft as *"DRAFT — evidence only, never
transcribe."* The draft settles **knowability**, not provenance. The figures above must be taken from
Rev. Proc. 2025-32's §55(d) section (published, verifiable today) or from the final `f6251--2026`.

**Bonus corroboration, free:** the draft's Part III lines 19 and 25 print `$98,900 / $49,450 / $66,200`
and `$545,500 / $306,850 / $613,700 / $579,600` — byte-for-byte the `LtcgBreakpoints` already bundled in
`ty2026()` (`tax_tables.rs:648-680`). An IRS document produced independently of Rev. Proc. 2025-32
confirms all eight bundled TY2026 LTCG numbers.

---

## 3. Where TY2026 would produce a WRONG NUMBER rather than refuse

**W1 (highest). `schedule_1a_params(2026)` already returns TY2025's amounts, and the guarding test
enforces the sameness rather than checking it.** `tables.rs:1139`'s
`schedule_1a_exists_only_for_2025_through_2028` asserts every year 2025..=2028 is field-identical to
2025. If any of the four OBBBA deductions *is* inflation-adjusted for 2026, the code prints a wrong
deduction on a signed return **and the test reds on the correction, not on the defect.** The whole
claim rests on the doc comment at `tables.rs:1008-1012` and `design/ty2025/SPEC_schedule_1a.md:80-82`
("caps and thresholds are fixed dollar amounts in the statute"), and **no §224/§225/§163(h)(4)/§151(d)(5)
text is archived in this worktree** (`legal/primary-sources/statute-irc/` holds §§1, 61, 170, 1001,
1011, 1012, 1015, 1016, 1031, 1091, 1211, 1212, 1221, 1222, 1223, 1411 — none of them these). Direction
of error is unknown, which is worse than a known-conservative one.

**W2. The AMT screening worksheet hardcodes the phase-out rate that TY2026 changes.**

```
crates/btctax-core/src/tax/amt.rs:129
        let line10 = (dec!(0.25) * line9).min(exemption);
crates/btctax-core/src/tax/amt.rs:141
    let line12 = dec!(0.26) * line11;
```

`AmtParams::exemption_phaseout_rate` and `rate_26` exist and are read correctly everywhere in
`form6251.rs` (`:489`, `:495`, `:507`, `:509`) — but not here. At TY2026's 50% the literal understates
worksheet line 10, which understates line 11, which makes the screen answer "no AMT" for filers who
owe it. **Mitigating fact, verified: `amt_should_file_6251` has no production caller** — every hit
outside its own definition is in `amt.rs`'s own test module — because Form 6251 is now computed
unconditionally (§G-6). So this is a latent trap, not a live defect; it becomes live the moment anyone
re-wires the screen.

**W3. No invariant pins `ftc_ceiling` across years.** §904(j)'s $300/$600 is statutory and must be
identical in every bundled year; its neighbour `qbi_phase_in_range_*` has `qbi_phase_in_top()` guarding
exactly this failure and `ftc_ceiling` has nothing. A one-line cross-year equality assertion in the same
sweep as `all_bundled_years_are_tax_table_binnable` (`tax_tables.rs:719`) closes it.

**W4. `ty2026()`'s primary sources are not in the archive.** `legal/primary-sources/irs-guidance/`
contains exactly one Rev. Proc. — `RevProc_2024-28.pdf`, the crypto basis safe harbour. **Rev. Proc.
2023-34, 2024-40 and 2025-32 are all absent**, and so is the SSA Federal Register determination, yet
those three documents are the sole authority for every indexed figure in `ty2024()`, `ty2025()` and
`ty2026()`. Nothing in this worktree can re-verify a TY2026 bracket. `design/forms/MANIFEST.json` covers
forms and instructions only.

**W5 (nit, citation decay).** `design/SPEC_tax_tables_2026.md` dates the SSA wage-base determination two
different ways in one document: the section heading says `SSA 2025-10-24`, the bullet under it says
`Federal Register 2025-11-03`. The code (`tax_tables.rs:693`) carries only the second.

**W6 (year-seam boundary, flagged for the code-seams lens).** `conventions.rs:19`
`pub const TY2025_RETURN_DUE: TaxDate = date!(2026-04-15)` is a singular constant, not a per-year
function, and is consumed at `crates/btctax-core/src/project/resolve.rs:1668`. A TY2026 port needs a
TY2026 due date; the constant's shape does not admit one.

---

## 4. Things that HELP, measured

- **Every TY2026 bracket edge below $100,000 is $25-aligned**, so the IRS Tax-Table binning rule holds
  and no per-year Tax-Table data is needed (`method.rs:9-13`, `TAX_TABLE_CEILING` at `method.rs:21`).
  Checked directly:
  `Single [0, 12400, 50400] · MFJ [0, 24800] · HoH [0, 17700, 67450] · MFS [0, 12400, 50400]` — none
  off a $25 multiple. And this is not a one-off measurement: `all_bundled_years_are_tax_table_binnable`
  (`crates/btctax-adapters/src/tax_tables.rs:719-745`) derives its year list from what is bundled, so
  **TY2026 is already inside an executing invariant**.
- **The `Qss → Mfj` aliasing** (`TaxTable::key`, `tables.rs:88`) means a TY2026 table needs four status
  entries, not five, with no drift risk between two identical schedules.
- **The two §55(d)(3) identities already loop over every bundled year**, so the day TY2026 `AmtParams`
  land, five MFS constants check each other (`tax_tables.rs:859-908`). Note the loop's own guard asserts
  `covered == vec![2024]` today and will red — deliberately — when a year is added, forcing the doc
  comment's EXECUTED/hand-worked split to be updated in the same commit.

---

## 5. CAN BE DONE TODAY — no IRS dependency

1. **Archive Rev. Proc. 2025-32 (I.R.B. 2025-45) into `legal/primary-sources/` with a SHA256, and
   extract its text.** It is published (Oct 2025) and it is the single document that supplies almost
   every indexed TY2026 `FullReturnParams` figure. Closes W4 for 2026 and, retroactively, for 2024/2025
   by archiving 2023-34 and 2024-40 alongside it.
2. **Read Rev. Proc. 2025-32 end to end and enumerate every section naming a Code section btctax uses.**
   This is also the decisive test for W1: if any of §224 / §225 / §163(h)(4) / §151(d)(5) is
   inflation-adjusted, the annual inflation Rev. Proc. is where a 2026 amount would appear. A hit there
   is a live wrong-number bug today; a clean sweep converts the Schedule 1-A "nothing is indexed" claim
   from a doc comment into a cited fact.
3. **Archive 26 U.S.C. §55 and §164 as amended by Pub. L. 119-21** (the statute-IRC archive has neither).
   §55(d)(3)'s amended text settles `mfs_kicker_rate` — the one open AMT cell — and §164(b)(6)'s amended
   text settles the TY2026 SALT cap, phase-down threshold and the escalator schedule through the
   reversion. Both are statutory and readable now; neither waits on a form. **Transcribe the figures from
   that text; do not encode them from memory or from a secondary summary.**
4. **Pin `ftc_ceiling` with a cross-year equality assertion** (W3) — one line, in the existing sweep.
5. **Make the AMT screening worksheet read `AmtParams`** instead of `dec!(0.25)` / `dec!(0.26)`
   (`amt.rs:129`, `:141`), and land it with the planted-defect test B1 requires: set
   `exemption_phaseout_rate = 0.50`, assert the screen's answer changes.
6. **Re-shape `schedule_1a_exists_only_for_2025_through_2028`** so it checks the *sameness claim* against
   a cited source rather than enforcing sameness — or, minimally, add the citation for each of the four
   provisions' absence of an inflation subsection to the doc comment, so a future indexing correction is
   an expected red rather than a surprising one.
7. **Do the TY2025 `FullReturnParams` first.** TY2026 cannot be reached over TY2025: the per-year gate is
   `full_return_for(year) → Some` and TY2025 is still `None` (`tax_tables.rs:814-822`). Every field name,
   every citation shape and both changed instruments (the §164(b) worksheet, the 1a/1b Form 6251) are
   TY2026 requirements too — `design/OWNER_DECISIONS_2026-09-04.md:43` records exactly this, that no
   TY2025 work is wasted on TY2026.

## 6. MUST WAIT, AND ON WHAT

| what | blocked on | expected |
|---|---|---|
| `mfs_kicker_rate` **confirmation** (as opposed to the statutory reading) | `i6251--2026` instructions | ~Jan 2027 |
| The TY2026 **§164(b) SALT worksheet as an instrument** — its line structure, not its constants | Schedule A instructions `i1040sca--2026` | ~Jan 2027 |
| `kiddie_unearned_threshold`, `elective_deferral_limit`, `student_loan_phaseout_*` **as printed on the form** (the Rev. Proc. and the retirement-limits Notice give the values now; `i1040gi--2025` is how TY2025 sourced them) | `i1040gi--2026` | ~Jan 2027 |
| Any **transcription** of Form 6251 2026 Part I (1a/1b around Schedule 1-A line 43), and of a 2026 Schedule 1-A | final TY2026 forms — the only 2026 artifact in the repo is one **draft** 6251, and `SUPPORTED_YEARS` is `&[2017, 2024, 2025]` (`crates/btctax-forms/src/lib.rs:68`) | Nov 2026 – Jan 2027 |
| A **second oracle** for any TY2026 AMT figure | OpenTaxSolver has no 2026 release; taxcalc alone is a known-defective AMT witness (`tax_tables.rs:925-929`) | OTS 2026, ~early 2027 |
| **TY2027** tables (already parked as externally-blocked, `FOLLOWUPS.md:159`) | the next inflation Rev. Proc. + SSA determination — **due within weeks of today** | ~Oct 2026 |

**The schedule, in one line.** Nothing about TY2026 *constants* waits on the IRS except confirmation and
transcription; the values are published or statutory today. What waits on the IRS is the **forms**, and
therefore everything that must be transcribed from a form rather than looked up — which is precisely the
work TY2025 is doing right now.
