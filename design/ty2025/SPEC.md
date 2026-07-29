# TY2025 — SPEC

**Status: DRAFT, not yet reviewed.** Written 2026-07-29. Per `STANDARD_WORKFLOW.md` this passes an
independent review loop to 0 Critical / 0 Important before any implementation plan is written.

Prerequisite reading: `design/amt-form6251/CONTINUITY_TY2025.md` — why TY2025 comes before Tier-2 E4,
and what the recon established. **Do not re-derive the facts recorded there.**

---

## 1. Purpose & guardrails

**Purpose.** Make btctax file a complete, correct **tax year 2025** federal return, to the same
standard as TY2024: every figure two-oracle validated, every form transcribed line by line.

Two things drive it, and both matter:

1. **Product.** btctax computes TY2024 only. It is mid-2026. TY2025 is the year people actually filed
   this spring, so a TY2024-only tool is of limited use to a real filer.
2. **Evidence.** TY2025 is the *only* way to witness the last dark region of the AMT computation.
   MFS with the exemption phased to zero is unwitnessed in TY2024 because **OTS 2024 carries stale
   2023 §55(d)(3) constants** and **taxcalc never models the Form 6251 line-4 add-back at all** —
   and for MFS those two conditions coincide by statute. **OTS 2025 implements the rule correctly**
   (installed, source-verified, smoke-tested: TI 935,000 → line 4 = 943,662.50 = 935,000 + 25% ×
   (935,000 − 900,350), exemption 0, AMT 13,571.50). Fixture vectors V23/V24/V25 currently owe AMT
   with **zero** witnesses; their TY2025 analogues will have one.

**Guardrails, inherited and non-negotiable:**

- **Transcribe, never paraphrase** (`CLAUDE.md`). One field per numbered line, named for the line, in
  the form's own numbering, carrying the official instruction text verbatim as its doc comment. This
  binds Schedule 1-A exactly as it binds Form 6251.
- **Two oracles, never one.** Every compared line reconciles against **both** OpenTaxSolver 2025 and
  Tax-Calculator, or carries a *named, computed* disqualification.
- **A disagreement is adjudicated against the FORM**, never encoded.
- **Answered-ness is structural.** A value btctax cannot see must refuse, not default to zero.
- **Conformance ⇒ test; judgment ⇒ review, kept scarce.**

---

## 2. Primary sources (archived, hashed, in-repo)

All fetched 2026-07-29 and committed under `design/amt-form6251/`. **These are the authority. An
oracle is a witness.**

| file | sha256 (first 8) | status |
|---|---|---|
| `f6251--2025.pdf` | `6995bfd2` | IRS **final** |
| `i6251--2025.pdf` | `b130e873` | IRS **final** |
| `f1040--2025.pdf` | `3d31c226` | IRS **final** |
| `f1040s1a--2025.pdf` | `64f97b38` | IRS **final** |
| `f6251--2026-DRAFT.pdf` | `a547fc9d` | **DRAFT — evidence only, never transcribe** |

Constants also trace to **Rev. Proc. 2024-40** (TY2025 inflation adjustments) and **OBBBA, Pub. L.
119-21**. Transcribe from the **text layer** (`pdftotext -layout`), never the rendered page — the
line-33 incident is why.

---

## 3. Binding decisions (defend these against a review flip)

**D-1. Schedule 1-A is implemented IN FULL — all five parts.** Owner decision, 2026-07-29. Not an
existence-question refusal, and not filer-supplied totals. Rationale: no filer is turned away, and
nothing is delegated to a filer who would have to get four phase-outs right. **This is affordable
precisely because both oracles implement it** (OTS 2025 `sched_1A()` at
`taxsolve_US_1040_2025.c:1783`; taxcalc's `TipIncomeDed_*`, `OvertimeIncomeDed_*`,
`AutoLoanInterestDed_*`, `SeniorDed_*`), so every part can be held to the two-oracle standard.

**D-2. TY2026 stays FAIL-CLOSED.** Held by `ty2026_full_return_must_stay_fail_closed`
(`btctax-adapters/src/tax_tables.rs`), mutation-verified. **Adding TY2025 deletes the `2025`
assertion beside it; 2026 must survive that.** Reasons in `CONTINUITY_TY2025.md` §5 — chiefly that
the 2026 instructions are unpublished and no oracle covers 2026.

**D-3. Form 6251 Part I is re-transcribed for 2025, not adapted.** The 2025 form is structurally
different from 2024:

| | 2024 | 2025 |
|---|---|---|
| line 1 | "…Form 1040 line **15**…" | **1a** = 1040 L14 − Sch 1-A L37 · **1b** = 1040 L11b − 1a |
| line 2a | "…otherwise, line **12**" | "…otherwise, line **12e**" |

★ **Line 1a subtracts Schedule 1-A line 37 — the *senior* deduction alone, not line 38, the total.**
So the enhanced senior deduction is **added back for AMT** while tips, overtime and car-loan interest
are **allowed**. That is a substantive tax fact taken from the form, and it must be encoded as the
form states it.

**D-4. Per-year form structure is a TYPE-level distinction, not a runtime flag.** A single
`Form6251` struct whose `line1` means different things in different years is the compression this
project keeps paying for. The mechanism is a design question for the plan (per-year structs, a
versioned Part I, an enum) — but "one struct, a year field, and branches inside" is **rejected**.

**D-5. Answered-ness for the new inputs.** Every Schedule 1-A input is `Option`; `None` **refuses**.
Zero is a filer's answer, never btctax's assumption. The §63(f) precedent applies: an unknown flag
must fail in the direction that cannot understate tax.

---

## 4. Non-goals

- **TY2026 anything** (D-2), including transcribing the draft's constants.
- **State returns.** Federal only, as today.
- **Tier-2 AMT attachment (E4/E5/E6).** TY2025 is a prerequisite for witnessing, not the attachment
  itself. The Tier-1 refusal stays in place for both years.
- **Amended/prior-year interaction.** TY2025 is a new year, not a re-filing feature.
- **Backfilling TY2017.** `TaxTable` has it; `FullReturnParams` will not.

---

## 5. Scope — what has to change

### 5.1 Constants: `FullReturnParams` for 2025
Every field, each traced to a named primary source. **`salt_cap` must be read, not assumed** — OBBBA
changed it, and carrying 10,000 forward would be silent and wrong.

`std_deduction` (×4 statuses) · `std_aged_blind_married` / `_unmarried` · `dependent_std_floor` /
`_earned_addon` · `salt_cap` · `kiddie_unearned_threshold` · `elective_deferral_limit` ·
`ftc_ceiling` · `qbi_ti_threshold_unmarried` / `_married` · `student_loan_phaseout_unmarried` /
`_married` · `amt`.

`AmtParams` for 2025, from the archived form (already read off the text layer):

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
constants reds on the day they are typed in. That test is the reason this is safe to hand-enter.

### 5.2 Schedule 1-A — a new form, transcribed line by line
Five parts, four phase-outs. Per the transcription rule this is one field per numbered line with the
printed text as its doc comment, **not** a `tips_deduction()` helper:

| part | deduction | cap | phase-out |
|---|---|---|---|
| I | MAGI (lines 1–3) | — | — |
| II | No Tax on Tips (4–13) | 25,000 | $100 per $1,000 of MAGI over 150,000 / 300,000 |
| III | No Tax on Overtime (14–21) | 12,500 / 25,000 MFJ | $100 per $1,000 over 150,000 / 300,000 |
| IV | No Tax on Car Loan Interest (22–30) | 10,000 | $200 per $1,000 over 100,000 / 200,000 |
| V | Enhanced Deduction for Seniors (31–37) | 6,000 per person | 6% of MAGI over 75,000 / 150,000 |

Line 38 (total) → 1040 line 13b. Line 37 (seniors) → Form 6251 line 1a (D-3).

**New input surface.** Qualified tips (W-2 box 7 and trade-or-business), qualified overtime, vehicle
loan interest **plus VINs** (Part IV names them), and the Part V per-spouse split. Part V is
computable from `date_of_birth`, which btctax already collects for §63(f) — but the *existence* of
tips/overtime/car-loan interest cannot be inferred and must be asked (D-5).

### 5.3 Year seams
- `ots_direct._bin` hardcodes `taxsolve_{form}_2024` (line 76) and `_template` looks for
  `{name}_2024_template.txt` (line 84). Thread the year; `OTS_DIR` becomes per-year (two installs now
  coexist).
- `form6251_vectors.json` is implicitly TY2024 — add `year`, and make `params()`/`bps()` in the Rust
  KAT and `verify_f6251.py` select per year.
- The witness census must assert the Tier-2 gate **per year**, not globally.

### 5.4 The vectors that motivated this
TY2025 MFS vectors above the 900,350 kicker start, witnessed by OTS 2025. These are the analogues of
V23/V24/V25, which have no witness at all in TY2024.

---

## 6. Test / green definition

**Green = the full validation suite passes AND 0 Critical / 0 Important**, where the suite is:

1. **Five gates:** `make check` · `cargo fmt --all --check` · `cargo +1.88 check --workspace
   --locked` · `cargo run -p xtask -- check-isolation` · `bash scripts/pii-scan-generic.sh`
   (scans HEAD — commit first).
2. **Both oracles, per year**, 0 unexpected: `verify_f6251.py` against OTS 2024 *and* OTS 2025, plus
   taxcalc, with every disqualification **computed** from the defect's own mechanism — never a list
   of vector names. That rule exists because both hand-kept lists went stale the moment E2 added
   vectors.
3. **The witness census passes per year** — every filing status in each year's fixture has ≥1
   AMT-owing vector agreed by two oracles.
4. **Schedule 1-A reconciles against both engines** part by part, including each phase-out's knee and
   its cap, in the same line-by-line style as Form 6251.
5. **Conformance is a test, not a review.** "Is every Schedule 1-A line present?" and "does each doc
   comment match the printed text?" are assertions.
6. **Mutation-verified guards.** A new guarantee ships with a mutation that kills it. Precedent from
   this session: the `phaseout_rate` split passed every existing test until a mutation showed the
   whole suite was blind to it.

---

## 7. Risks

**R-1. The transcription is bigger than the constants.** Both Form 6251 Part I and the whole of
Schedule 1-A change shape. Treating TY2025 as "type in new numbers" is the failure mode — an earlier
draft of the continuity doc made exactly that error and it is corrected there.

**R-2. Phase-out step arithmetic.** Parts II–IV reduce by a flat amount *per whole $1,000 step*, and
the form specifies the rounding direction per part (line 11 "decrease to the next whole number", line
28 "increase"). These are transcription details a paraphrase would smooth over, and they change the
answer at every knee.

**R-3. OTS 2025 is verified only on §55(d)(3) and one smoke test.** Its Schedule 1-A is unexamined.
Read `sched_1A()` before trusting it, exactly as its 2024 kicker was read.

**R-4. Scope creep into TY2026.** D-2 and its mutation-verified test are the control.

**R-5. `salt_cap` and any other OBBBA-changed constant carried forward silently.** Every field gets a
source, or it does not ship.

---

## 8. Open questions for review

1. **The per-year form-shape mechanism** (D-4) — per-year structs, a versioned Part I, or an enum?
   The plan decides; the spec only forbids branching inside one struct.
2. **Does the golden corpus extend to TY2025**, or stay TY2024 until Tier-2 E5?
3. **Part IV VIN storage** — a filed-form string btctax must persist and emit. Does it reach the PII
   scanner's surface? (`scripts/pii-scan-generic.sh` scans HEAD.)
4. **Does OTS 2025 apply the §170(b) cash ceiling** its 2024 solver omits? That defect gated V2b; if
   it is fixed, the computed disqualification narrows for TY2025.

---

## 9. Cross-references

- `design/amt-form6251/CONTINUITY_TY2025.md` — the recon, the pivot, and the environment notes.
- `FOLLOWUPS.md` §G-6b (Tier-2 entry criteria), §G-6e (this pivot), §G-6f (the rate split, done).
- `STANDARD_WORKFLOW.md` — the review loop this document must pass before a plan is written.
