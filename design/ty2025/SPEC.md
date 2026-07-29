# TY2025 — SPEC

**Status: DRAFT r2** (r1 folded 2026-07-29 — 5 Critical, 4 Important, 1 Minor, all accepted; plus 4
author findings). Per `STANDARD_WORKFLOW.md` this passes an independent review loop to
0 Critical / 0 Important before any implementation plan is written. **Not yet re-reviewed.**

Prerequisite reading: `design/amt-form6251/CONTINUITY_TY2025.md` — why TY2025 comes before Tier-2 E4,
and what the recon established. **Do not re-derive the facts recorded there.**
Review history: `design/ty2025/reviews/`.

> **★ What r1 changed, in one line.** The r1 draft treated TY2025 as "new constants + Schedule 1-A".
> It is neither: **OBBBA superseded the standard deduction and turned SALT into a phase-down**, the
> **Form 1040 itself** changed shape, Schedule 1-A has a **MAGI surface and a filing-status bar** the
> draft never mentioned, and **OTS 2025's Part IV is defective in three ways** — so the draft's own
> justification for D-1 ("both oracles implement it") was false where it mattered most.

---

## 1. Purpose & guardrails

**Purpose.** Make btctax file a complete, correct **tax year 2025** federal return, to the same
standard as TY2024: every figure two-oracle validated, every form transcribed line by line.

Two things drive it:

1. **Product.** btctax computes TY2024 only. It is mid-2026. TY2025 is the year people actually filed
   this spring.
2. **Evidence.** TY2025 is the only way to witness the last dark region of the AMT computation. MFS
   with the exemption phased to zero is unwitnessed in TY2024 because OTS 2024 carries stale 2023
   §55(d)(3) constants and taxcalc never models the Form 6251 line-4 add-back — and for MFS those
   coincide by statute. **OTS 2025 implements that rule correctly** (verified; smoke-tested).
   Vectors V23/V24/V25 owe AMT with **zero** witnesses today.

**Guardrails, inherited and non-negotiable:**

- **Transcribe, never paraphrase.** One field per numbered line, in the form's own numbering, with the
  official text **verbatim** as its doc comment. ★ *Including in this document* — r1 caught the r1
  draft trimming Schedule 1-A's own words ("decrease the result to the next **lower** whole number").
- **Two oracles, never one**, or a **named and sized** disqualification, **computed** from the
  defect's mechanism — never a list of vector names. Both hand-kept lists in this repo went stale the
  moment new vectors landed.
- **A disagreement is adjudicated against the FORM**, never encoded.
- **Answered-ness is structural.** A value btctax cannot see must refuse, not default to zero.
- **Conformance ⇒ test; judgment ⇒ review, kept scarce.**

---

## 2. Primary sources

**The form is the authority. An oracle is a witness. Transcribe from the text layer
(`pdftotext -layout`), never the rendered page.**

**Archived and hashed** (in `design/amt-form6251/`, fetched 2026-07-29):

| file | sha256 (first 8) | status |
|---|---|---|
| `f6251--2025.pdf` | `6995bfd2` | IRS **final** |
| `i6251--2025.pdf` | `b130e873` | IRS **final** |
| `f1040--2025.pdf` | `3d31c226` | IRS **final** |
| `f1040s1a--2025.pdf` | `64f97b38` | IRS **final** |
| `f6251--2026-DRAFT.pdf` | `a547fc9d` | **DRAFT — evidence only, never transcribe** |
| `crates/btctax-forms/forms/2024/f6251.pdf` | `7fea4e42` | the TY2024 form, already in-repo |

**STILL TO FETCH AND ARCHIVE before implementation** (r1-8, and the SALT structure below):

- **Schedule 1-A instructions** — the form defers to them in at least five places that change the
  number (line 4a's "$176,100" branch, 4c multi-employer, 5 multi-trade-or-business, 22 ">two VINs",
  36a "valid social security number").
- **Schedule A (2025) + instructions** — the §164(b)(6) SALT phase-down parameters below are
  **not yet verified against a primary source**; they are currently only corroborated by taxcalc.
- **Form 1040 instructions (2025)**, **Rev. Proc. 2024-40**, and the **OBBBA (Pub. L. 119-21)** text.

★ **Per-field source rule (r1-2).** For every constant, **the later of Rev. Proc. 2024-40 and OBBBA
controls.** The Rev. Proc. is *not* the blanket source for TY2025. Each field in §5.1 carries its own
citation, and a field without one does not ship.

---

## 3. Binding decisions

**D-1. Schedule 1-A is implemented IN FULL — all five parts.** Owner decision. Not existence-question
refusal, not filer-supplied totals.

★ **Justification corrected by r1.** The r1 draft said "both oracles implement it", which is true only
in outline. Both do — OTS 2025 `sched_1A()` (`taxsolve_US_1040_2025.c:1783`); taxcalc's
`TipIncomeDed_*`, `OvertimeIncomeDed_*`, `AutoLoanInterestDed_*`, `SeniorDed_*` in a live
`@iterate_jit` `MiscDed()`. **But OTS 2025's Part IV is defective three ways** (D-8), and taxcalc
cannot witness the MAGI add-backs at all (D-6). So Schedule 1-A is *two-oracle-backed in Parts II, III
and V, and one-oracle-plus-named-disqualification in Part IV*. That is acceptable; asserting it were
cleaner is not.

**D-2. TY2026 stays FAIL-CLOSED.** Held by `ty2026_full_return_must_stay_fail_closed`
(`btctax-adapters/src/tax_tables.rs`), mutation-verified. **Adding TY2025 deletes the `2025` assertion
beside it; 2026 must survive that.** Reasons: `CONTINUITY_TY2025.md` §5.

**D-3. Form 6251 Part I is re-transcribed for 2025.**

| | 2024 | 2025 |
|---|---|---|
| line 1 | "…Form 1040 line **15**…" | **1a** = 1040 L14 − Sch 1-A L37 · **1b** = 1040 L11b − 1a |
| line 2a | "…otherwise, line **12**" | "…otherwise, line **12e**" |

★ **Line 1a subtracts Schedule 1-A line 37 — the *senior* deduction subtotal, not line 38, the
total.** So the enhanced senior deduction is **added back for AMT** while tips, overtime and car-loan
interest are **allowed**. Verified: Sch 1-A line 37 = "Enhanced deduction for seniors. Add lines 36a
and 36b"; line 38 = "Add lines 13, 21, 30, and 37 … enter on Form 1040 line 13b".

**D-4. Per-year form shape is a TYPE-level distinction — and it fences Form 1040, not only Form 6251
(r1-6).** The **2025 Form 1040 changed shape**: lines 11a/11b, 12e, 13a, **13b**, and
**line 14 = "Add lines 12e, 13a, and 13b"**. `AbsoluteReturn` is transcribed in 2024 numbering
(`line14` documented as "L12 + L13"; `ti_before_qbi = agi − deduction`). Wiring D-3's "line 14" to the
existing field would omit 13b entirely **and** leave Form 8995 line 11 overstating
taxable-income-before-QBI by the whole Schedule 1-A total — overstating the §199A deduction and firing
`qbi_over_threshold` too late. **The QBI ordering for 2025 is `11b − 12e − 13b`.**
"One struct, a year field, branches inside" is **rejected**; the mechanism is the plan's to choose.

**D-5. Answered-ness.** Every Schedule 1-A input is `Option`; `None` **refuses**. Zero is a filer's
answer, never btctax's assumption. This explicitly covers the **MAGI add-backs (D-6)** and the
**valid-SSN predicate (D-7)**, which the r1 draft omitted.

**D-6. MAGI is a collected surface, not AGI (r1-3).** Schedule 1-A Part I is lines 1, **2a–2e**, 3,
where line 3 = "Add lines 1 and 2e" and 2a–2d are excluded Puerto Rico income, Form 2555 lines 45 and
50, and Form 4563 line 15. That difference drives **all four** phase-outs. **taxcalc cannot witness
it** — its `MiscDed` uses `c00100` directly as the MAGI proxy — so a reconciliation that looks clean
is one-oracle here.

**D-7. Filing-status eligibility is part of the transcription (r1-4).** Schedule 1-A Parts II, III and
V each print *"If married, you must file jointly to claim this deduction"* — **so they are zero for
MFS.** Part IV carries **no** such caution.

★ **The oracles split, and it is adjudicated against the form.** OTS bars *all four* parts for MFS
(`taxsolve_US_1040_2025.c:1824`: `if (status != MARRIED_FILING_SEPARAT)`); taxcalc allows Part IV for
MARS=3. **The form governs: Part IV is allowed for MFS**, and OTS is disqualified on it (D-8).
Parts II/III/V additionally require a valid SSN (line 36a) — a predicate DOB cannot answer and btctax
cannot verify, so it is **asked** under D-5.

**D-8. OTS 2025's Part IV defects are named and sized, not improvised.** Read from source, all three
in `taxsolve_US_1040_2025.c:1891-1894`:

1. `sched1A_L[29] = 300.0 * sched1A_L[28]` — the form says **$200** (taxcalc's
   `AutoLoanInterestDed_po_rate_per_step` = 0.2 agrees with the form).
2. `j = sched1A_L[27] / 1000.0` with `j` an int — **truncates**, where the form says "increase the
   result to the next higher whole number".
3. It prints `showline_wlabelnz("S1A_20", …)` — **line 29 is never emitted**, under a Part III label.

Consequence: at MAGI $120,000 with $10,000 of qualified interest the form gives $6,000 and OTS gives
$4,000. **Part IV rides taxcalc plus this named disqualification** — computed from the mechanism,
per §6.2.

---

## 4. Non-goals

- **TY2026 anything** (D-2), including transcribing the draft's constants.
- **State returns.**
- **Tier-2 AMT attachment (E4/E5/E6).** The Tier-1 refusal stays for both years.
- **Amended/prior-year interaction.** **Backfilling TY2017** full-return params.

---

## 5. Scope

### 5.1 Constants: `FullReturnParams` for 2025

Each field carries its own citation under the §2 per-field rule. **Two are OBBBA, not the Rev. Proc.:**

★ **`std_deduction` — OBBBA, verified against the 2025 Form 1040's own margin: 15,750 single /
31,500 MFJ / 23,625 HoH / 15,750 MFS.** *Not* Rev. Proc. 2024-40's 15,000 / 30,000 / 22,500. Both
oracles agree (OTS `S_STD_DEDUC = 15750.0`; taxcalc `STD` 2025 = `[15750, 31500, 15750, 23625,
31500]`). Typing the Rev. Proc. figures overstates **every** filer's taxable income by $750–$1,500,
and the existing `ty2025_..._match_rev_proc_2024_40` test covers **brackets only** and would not
notice. **Requires its own test against the four values.**

★ **`salt_cap` is NOT a scalar (r1-1).** TY2025 §164(b)(6) is a **$40,000 cap ($20,000 MFS), reduced
by 30% of MAGI over $500,000 ($250,000 MFS), floored at $10,000 ($5,000 MFS)**. `salt_cap: Usd`
(`tables.rs:326`, consumed at `return_1040.rs:322-327` and `printed.rs:1145` as a bare `min`) cannot
express that. Replace with a per-status sub-structure: **cap / phase-down rate / MAGI threshold /
floor**. ⚠ **The cap is confirmed (taxcalc `ID_AllTaxes_c` 2025 = `[40000, 40000, 20000, 40000,
40000]`); the rate, threshold and floor are corroborated by taxcalc only and MUST be transcribed from
the 2025 Schedule A instructions before use** (§2).

Remaining fields, each to be sourced: `std_aged_blind_married`/`_unmarried` ·
`dependent_std_floor`/`_earned_addon` · `kiddie_unearned_threshold` · `elective_deferral_limit` ·
`ftc_ceiling` · `qbi_ti_threshold_unmarried`/`_married` ·
`student_loan_phaseout_unmarried`/`_married` · `amt`.

`AmtParams` for 2025 — **all cells verified against `f6251--2025.pdf` in review r1**:

| | single/hoh | mfj/qss | mfs |
|---|---|---|---|
| exemption | 88,100 | 137,000 | 68,500 |
| phase-out start | 626,350 | 1,252,700 | 626,350 |
| 26/28% breakpoint | 239,100 | 239,100 | 119,550 |
| 28% subtrahend | 4,782 | 4,782 | 2,391 |
| line 19 | 48,350 (hoh 64,750) | 96,700 | 48,350 |
| line 25 | 533,400 (hoh 566,700) | 600,050 | 300,000 |

`mfs_kicker_start` 900,350 · `mfs_kicker_max` 68,500 · `exemption_phaseout_rate` 0.25 ·
`mfs_kicker_rate` 0.25.

★ The two §55(d)(3) identities are already asserted over **every bundled year**
(`mfs_kicker_constants_satisfy_the_two_section_55d3_identities`), so a slip in any of five MFS
constants reds the day they are typed.

### 5.2 Schedule 1-A — a new form, transcribed line by line

| part | lines | deduction | cap | phase-out | MFS? |
|---|---|---|---|---|---|
| I | 1, 2a–2e, 3 | MAGI | — | — | — |
| II | 4–13 | No Tax on Tips | 25,000 | $100 per $1,000 over 150,000 / 300,000 | **barred** |
| III | 14–21 | No Tax on Overtime | 12,500 / 25,000 MFJ | $100 per $1,000 over 150,000 / 300,000 | **barred** |
| IV | 22–30 | Car Loan Interest | 10,000 | $200 per $1,000 over 100,000 / 200,000 | allowed (D-7) |
| V | 31–37 | Seniors | 6,000 per person | 6% of MAGI over 75,000 / 150,000 | **barred** |

Line 38 = "Add lines 13, 21, 30, and 37" → 1040 line 13b. Line 37 → Form 6251 line 1a (D-3).

★ **Rounding direction differs per part and is quoted verbatim** — line 11: *"Divide line 10 by
$1,000. If the resulting number isn't a whole number, **decrease** the result to the next **lower**
whole number."* Line 28: *"…**increase** the result to the next **higher** whole number."*

**Input surface** (all `Option`, `None` refuses — D-5):
qualified tips (W-2 box 7; trade-or-business) · qualified overtime · vehicle loan interest **+ VINs**
· **MAGI add-backs 2a–2d** (D-6) · **valid-SSN predicate** per person (D-7) · Part V per-spouse split.
Part V's *age* half comes from `date_of_birth`, which btctax already collects for §63(f) — the
`is_aged` test ("born on or before January 1 of year−64") exactly matches the form's "born before
January 2, 1961".

★ **Every "see instructions" branch is transcribed or refuses (r1-8).** At minimum: line 4a's
"$176,100" branch, 4c multi-employer/occupation, 5 multi-trade-or-business, 22 ">two VINs", 36a
valid-SSN. Occupation eligibility defers to **IRS.gov/TippedOccupations, which is not a document** — a
doc comment reading "see the instructions" satisfies a conformance test and answers nothing.

### 5.3 Year seams

- ★ **`_ots_amt_disqualified` is YEAR-BLIND, and this is the seam that decides whether the pivot works
  at all** (author finding). `ots_direct.py:185` hardcodes `831_150.0` — OTS **2024**'s stale MFS
  threshold — and applies the cash-ceiling leg unconditionally. **OTS 2025 fixed both** (verified:
  charity capped at 60% of AGI, `taxsolve_US_1040_2025.c:2424`). Left as-is, TY2025's MFS kicker
  vectors are disqualified against an oracle that handles them **correctly** — the entire purpose of
  the pivot fails silently while every run prints `OK` and the census says "not witnessed".
- `ots_direct._bin` hardcodes `taxsolve_{form}_2024` (:76); `_template` looks for
  `{name}_2024_template.txt` (:84). `OTS_DIR` becomes per-year (two installs coexist).
- ★ **`gen_goldens.py` is the Schedule 1-A harness and is doubly year- and status-blind (r1-7).** It
  hardcodes `FLPDYR: 2024` / `start_year=2024`, and maps `"MARS": 2 if filing_status ==
  "Married/Joint" else 1` — **HoH, MFS and QSS all reach taxcalc as Single**, so the MFJ-doubled caps
  and the MFS bar are unreachable. Needs a real `MARS` map and the year threaded.
- ★ **The OTS driver must emit an `S1A_*` template block.** OTS's 2025 parser accepts an input file
  with no `S1A_2a` (it falls through to `A1`, `:2375-2391`), in which case `sched_1A()` is never
  called and `L13b` is silently 0 **with no error** — reconciling perfectly on every household whose
  Schedule 1-A happens to be zero.
- ★ **taxcalc needs `exact = 1` (r1-9)**, or it smooths the stepped phase-outs and diverges by up to
  $100 (Parts II/III) or $200 (Part IV) at every MAGI off a $1,000 multiple. `exact` co-governs
  `ChildDepTaxCredit` / `EducationTaxCredit` / `F2441` / `CTC_new`, so it must be set in a
  **TY2025-scoped row builder**, not a shared one — this is the one real TY2024-contamination path.
- `form6251_vectors.json` is implicitly TY2024 — add `year`; make `params()`/`bps()` and
  `verify_f6251.py` select per year; the witness census asserts the gate **per year**.

### 5.4 Form assets (author finding)

TY2025 currently ships **5** of the needed forms (`f1040`, `f8283`, `f8949`, `schedule_d`,
`schedule_se`). **Twelve are missing** — eleven that 2024 has (`f1040s1`, `s2`, `s3`, `sa`, `sb`,
`sc`, `f6251`, `f8275`, `f8959`, `f8960`, `f8995`) plus **Schedule 1-A, which exists for no year** —
each needing a PDF and an AcroForm `.map.toml`. Without them TY2025 *computes* but cannot be *filed*.
(Note `2024/f6251.pdf` is the only PDF in the tree with no map — that is Tier-2 E4, out of scope.)

### 5.5 The vectors that motivated this

TY2025 MFS vectors above the 900,350 kicker start, witnessed by OTS 2025 — the analogues of
V23/V24/V25, which have no witness at all in TY2024.

---

## 6. Test / green definition

**Green = the full validation suite passes AND 0 Critical / 0 Important.**

1. **Five gates:** `make check` · `cargo fmt --all --check` · `cargo +1.88 check --workspace --locked`
   · `cargo run -p xtask -- check-isolation` · `bash scripts/pii-scan-generic.sh` (scans HEAD).
2. **Both oracles, per year, 0 unexpected**, with **every disqualification COMPUTED from the defect's
   own mechanism and SIZED** — never a list of vector names. ★ **This rule now covers Schedule 1-A as
   well as Form 6251 (r1-5)**, so OTS's Part IV defects are a stated escape rather than an improvised
   one. Any part shipping on one oracle must say so, in the same census that says it for AMT.
3. **The witness census passes per year** — every filing status in each year's fixture has ≥1
   AMT-owing vector agreed by two oracles.
4. **Schedule 1-A reconciles part by part**, including **each phase-out's knee and its cap**, and
   **across all five filing statuses** (r1-4) — the MFS bar on II/III/V is a required case, not an
   incidental one.
5. **A TY2025 golden household above the §164(b)(6) SALT phase-down threshold** (r1-1). The corpus is
   the only harness that exercises SALT and its cells are built around "capped to $10k"; without this
   requirement nothing runs SALT at 2025 constants. **A numbered requirement, not an open question.**
6. **A test asserting the four TY2025 `std_deduction` values** (r1-2).
7. **Conformance is a test, not a review** — "is every Schedule 1-A line present?", "does each doc
   comment match the printed text?" are assertions.
8. **Mutation-verified guards.** Precedent from this session: the `phaseout_rate` split passed every
   existing test until a mutation showed the whole suite was blind to it.

---

## 7. Risks

**R-1. The transcription is bigger than the constants.** Form 6251 Part I, Form 1040, and all of
Schedule 1-A change shape. "Type in new numbers" is the failure mode, and the r1 draft made exactly
that error.

**R-2. Phase-out step arithmetic**, with opposite rounding directions per part (§5.2) — details a
paraphrase smooths over, and they change the answer at every knee.

**R-3. OTS 2025 is verified on §55(d)(3), the charitable ceiling, and Part IV.** ★ **Part IV was read
and is DEFECTIVE** (D-8) — the r1 draft listed "read `sched_1A()` before trusting it" as a risk and
then relied on it as a fact. Parts II, III and V remain **unread** and must be read before use.

**R-4. Scope creep into TY2026.** D-2 and its mutation-verified test are the control.

**R-5. A constant carried forward silently.** Every field gets a citation, or it does not ship.

**R-6. A "see instructions" branch resolved by guessing** (§5.2) — a conformance test against the
*form* passes on every one of them.

---

## 8. Open questions for review

1. **The per-year form-shape mechanism** (D-4) — per-year structs, versioned parts, or an enum? The
   plan decides; the spec forbids branching inside one struct.
2. **Does the golden corpus extend to TY2025 in this project, or after Tier-2 E5?** ★ Note §6.5 now
   *requires* a TY2025 SALT household, so "no corpus" is no longer a free answer.
3. ~~Part IV VIN storage vs. the PII scanner~~ — **answered:** `scripts/pii-scan-generic.sh:3` matches
   only SSN (`\d{3}-\d{2}-\d{4}`) and EIN (`\d{2}-\d{7}`) shapes; a 17-character VIN is not covered.
   Decide: extend `SHAPES`, or record why a VIN is out of scope.
4. ~~Does OTS 2025 apply the §170(b) cash ceiling?~~ — **answered: yes**, `taxsolve_US_1040_2025.c:2424`
   caps `charityCC` at 60% of AGI. That leg of the disqualification predicate must therefore be
   year-scoped (§5.3), not merely re-used.

---

## 9. Cross-references

- `design/amt-form6251/CONTINUITY_TY2025.md` — the recon, the pivot, the environment notes.
- `design/ty2025/reviews/` — r1 verbatim.
- `FOLLOWUPS.md` §G-6b (Tier-2 entry criteria), §G-6e (this pivot), §G-6f (the rate split, done).
- `STANDARD_WORKFLOW.md` — the review loop this must pass before a plan is written.
