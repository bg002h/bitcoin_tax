# TY2025 — SPEC

**Status: DRAFT r3** (r1 folded: 5C/4I/1M. r2 folded: **4C/3I/1M/1N** surviving adversarial
verification, 5 refuted; plus 6 author findings across both rounds). Per `STANDARD_WORKFLOW.md` this passes an independent review loop to
0 Critical / 0 Important before any implementation plan is written. **Not yet re-reviewed.**

Prerequisite reading: `design/amt-form6251/CONTINUITY_TY2025.md` — why TY2025 comes before Tier-2 E4,
and what the recon established. **Do not re-derive the facts recorded there.**
Review history: `design/ty2025/reviews/`.

> **★ What r2 changed, in one line.** r2 attacked the r1 fold and found four more Criticals in it:
> **SALT's MAGI and Schedule 1-A's MAGI are the same statutory quantity** and nothing connected them;
> **taxcalc is wrong for QSS in Part IV**, so blanket-disqualifying OTS there left *zero* witnesses;
> the constants test covered **one field of ~20**; and the SALT design was **taxcalc's parameterization
> rather than the form's worksheet**. Schedule 1-A also has **six** parts, not five.
>
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
| `f1040sa--2025.pdf` | `c14acf34` | IRS **final** — Schedule A |
| `i1040sca--2025.pdf` | `b0999b12` | IRS **final** — Schedule A instructions, carries the SALT worksheet |
| `i1040gi--2025.pdf` | `482e9c48` | IRS **final** — the 1040 instructions, **which carry the Schedule 1-A instructions** |
| `crates/btctax-forms/forms/2024/f6251.pdf` | `7fea4e42` | the TY2024 form, already in-repo |

★ **r1-8 IS CLOSED, AND IT WAS PARTLY WRONG AS STATED.** It claimed the form "defers to instructions
in at least five places that change the number" and that occupation eligibility "defers to
IRS.gov/TippedOccupations, which is not a document at all". The Schedule 1-A instructions live inside
`i1040gi--2025.pdf`, now archived, and they answer every one of those branches outright:

| branch | what we are told to do |
|---|---|
| line 22, >2 VINs | *"attach a statement to your return showing the information required on line 22"* |
| line 36a, valid SSN | stated as a plain eligibility condition, per person, incl. the MFJ case |
| tips cap | *"can't deduct more than $25,000 of qualified tips, regardless of your filing status"* |
| occupation eligibility | **the list IS in the instructions**, each occupation carrying a numeric **Treasury Tipped Occupation Code** |

★ **And reading them caught a rule the spec had MISSED entirely** — a *Net income limitation* on Part
II line 5: trade-or-business tips "can't be more than the gross income from the trade or business …
minus the total of all deductions allocable to that trade or business, including the deductible part
of self-employment tax; … SEP, SIMPLE, and qualified plans; and the self-employed health insurance
deduction, but not including the deduction for qualified tips." That is a second cap on line 5 that no
amount of reviewing the *form* would have surfaced.

**Still to fetch — but nothing is blocked on them.** Rev. Proc. 2024-40 and the OBBBA text would give
some §5.1 fields a *second* citation; every field already has one from an archived IRS document. ★ The
lesson worth keeping: three of §5.1's four "still a lookup" fields, plus the QSS threshold, plus the
Schedule 1-A occupation list, were all printed in `i1040gi--2025.pdf` — **a document already sitting in
this directory while the spec listed them as outstanding.** Grep the archive before deferring.

## 2a. ★★ SOURCING OF RECORD — READ THIS BEFORE DERIVING ANYTHING

**`design/full-return/recon/fable/` (2026-07-11) already contains most of what this spec defers**, and
the r3 sanity check found it only because it went looking in the repo. It was written against the
enacted Pub. L. 119-21 and the TY2025 finals:

| file | what it already holds |
|---|---|
| `01-ty2025-finals-obbba.md` | the **full 10-line SALT worksheet** with the halve-last rule, §63(f) and dependent-standard amounts, every Schedule 1-A cap/threshold/rounding direction with statutory cites |
| `03-followon-math-sch1a-qbi-ctc.md` | per-part worked examples; TY2025 QBI thresholds **$197,300 / $394,600** |
| `05-ty2025-field-maps.md` | **462 lines of verbatim AcroForm field names** extracted from the six final TY2025 PDFs — the §5.4 work item |

★ **This session re-derived the SALT rule from scratch and got the MFS shape wrong twice** (r2 adopted
taxcalc's quadruple; the r2 fold half-corrected it), when `01`'s line 13 already names that exact
error — *"not the 30%-slope/$350k … the opus 'halve both constants'"* — and carries the MFS
$300,000 → **$12,500** worked example as a directive. **Re-derivation regressed a correct result that
was already recorded.** Consult these three files first; re-verify against the archived finals at write
time, since they are our own notes and not primary sources.

★ **Per-field source rule (r1-2).** For every constant, **the later of Rev. Proc. 2024-40 and OBBBA
controls.** The Rev. Proc. is *not* the blanket source for TY2025. Each field in §5.1 carries its own
citation, and a field without one does not ship.

---

## 3. Binding decisions

**D-1. Schedule 1-A is implemented IN FULL — all SIX parts.** (r2-9: the form has Parts I–VI; Part VI
is line 38, "Total Additional Deductions". The r2 draft said five.) Owner decision. Not existence-question
refusal, not filer-supplied totals.

★ **Justification corrected by r1.** The r1 draft said "both oracles implement it", which is true only
in outline. Both do — OTS 2025 `sched_1A()` (`taxsolve_US_1040_2025.c:1783`); taxcalc's
`TipIncomeDed_*`, `OvertimeIncomeDed_*`, `AutoLoanInterestDed_*`, `SeniorDed_*` in a live
`@iterate_jit` `MiscDed()`. **But OTS 2025's Part IV is defective three ways (D-8), taxcalc is wrong
for QSS in Part IV (D-8), and neither oracle uses MAGI where the law requires it (D-6).** So Schedule
1-A is *provisionally two-oracle-backed in Parts II, III and V — **provisional because R-3 records
OTS's Parts II/III/V as UNREAD, and an unread oracle is not a claimed one*** — two-oracle in Part IV
below the phase-out threshold (where OTS is exact, D-8), one-oracle inside the band for non-QSS, and
**ZERO-oracle for QSS Part IV inside it**. That is survivable only because it is stated and
censused — asserting it were cleaner is what r1 and r2 each caught.

**D-2. TY2026 stays FAIL-CLOSED.** Held by `ty2026_full_return_must_stay_fail_closed`
(`btctax-adapters/src/tax_tables.rs`), mutation-verified. **Adding TY2025 deletes the `2025` assertion
beside it; 2026 must survive that.** Reasons: `CONTINUITY_TY2025.md` §5.

**D-3. Form 6251 Part I is re-transcribed for 2025.**

| | 2024 | 2025 |
|---|---|---|
| line 1 | "…Form 1040 line **15**…" | **1a** = 1040 L14 − Sch 1-A L37 · **1b** = 1040 L11b − 1a |
| line 2a | "…otherwise, line **12**" | "…otherwise, line **12e**" |
| line 4 | "Combine lines **1** through 3" | "Combine lines **1b** through 3" |

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
`qbi_over_threshold` **too EARLY — a false refusal** (r2-8: the r2 draft said "too late", backwards;
omitting a deduction *overstates* taxable income, so the threshold is crossed sooner). **The QBI ordering for 2025 is AGI − 12e − 13b.** ★ Cite it as the form does, not as we prefer:
Form 8995 line 11 prints only "Taxable income before qualified business income deduction (see
instructions)", so the 1040-line reference lives in i8995 and the doc comment quotes i8995 verbatim.
There is no ambiguity in the *value* — the 2025 1040 defines line 11a as "Subtract line 10 from line 9.
This is your adjusted gross income" and line 11b as "**Amount from line 11a** (adjusted gross income)",
a page-2 carry-forward, so 11a and 11b are the same number.
"One struct, a year field, branches inside" is **rejected**; the mechanism is the plan's to choose.

**D-9. COLLECT EVERY NUMBERED LINE — including ones that will almost always be zero.** Owner decision,
2026-07-29. The input surface **is the form**. Excluded Puerto Rico income, Form 2555 lines 45 and 50,
and Form 4563 line 15 get collected like any other line, even though they are zero for nearly every
filer.

★ **This is not thoroughness for its own sake — it retires a category of defect.** "btctax cannot see
it" was the premise of D-6's MAGI hole, of r1-3, and of half the answered-ness arguments in this
document. If every line the form asks for is collected, that premise is gone: there is no value btctax
can *silently answer for the filer*, because there is no value it fails to ask about. It is a direct
step against this codebase's one standing architectural defect — answered-ness held by convention
rather than construction.

**D-10. WHEN THE ORACLES CANNOT AGREE, CITE THE IRS INSTRUCTIONS.** Owner decision, 2026-07-29. An
absent or split oracle is not a blocker and not a reason to refuse. **We are literally told what to
do, on the form or in its instruction document** — so the number is anchored there, pinned by a KAT,
and the suspected engine defect goes on an upstream register.

Three tiers of citation, because not all of them are load-bearing:

1. **A worked example in the instructions** → a KAT with exact numbers. Strongest available evidence,
   stronger than two oracles agreeing. (Precedent: i6251's MFS kicker example, already a KAT.)
2. **An explicit rule with constants** → transcribed verbatim; conformance test compares the doc
   comment to the printed text.
3. ~~*"See instructions" is not citable*~~ — **struck.** "See instructions" is a pointer to a document
   that exists; go read it. The r2 draft treated it as a dead end and was wrong (see §2's r1-8 note),
   which is exactly the inversion this decision corrects: the instructions are the authority, and the
   oracles exist to catch **our** transcription slips, not the reverse.

★ **The census still prints "no oracle here" every run** — so a cell resting on transcription alone
stays visible, and if an engine later fixes its defect we notice and reclaim the witness.

★ **The upstream register carries a filing bar** (`FOLLOWUPS.md`): before filing, grep their source,
confirm against the second oracle **or** the form's worked example, and check the latest release.
#3108 was filed on one oracle against our own rule and contained a claim that was wrong.

**D-5. Answered-ness.** Every Schedule 1-A input is `Option`; `None` **refuses**. Zero is a filer's
answer, never btctax's assumption. This explicitly covers the **MAGI add-backs (D-6)** and the
**valid-SSN predicate (D-7)**, which the r1 draft omitted.

**D-6. MAGI is ONE collected surface driving FIVE phase-outs — Schedule 1-A's four AND SALT (r1-3,
r2-2, r2-3).** Schedule 1-A Part I is lines 1, **2a–2e**, 3, where line 3 = "Add lines 1 and 2e" and
2a–2d are excluded Puerto Rico income, Form 2555 lines 45 and 50, and Form 4563 line 15.

★ **The §164(b) SALT worksheet uses the IDENTICAL add-backs** — its lines 3a–3d are the same four,
and §164(b)(7)(B)(iv) defines MAGI as "adjusted gross income increased by any amount excluded from
gross income under section 911, 931, or 933". So MAGI is a single quantity, **collected whenever
Schedule A is in play and not only when Schedule 1-A is**, and `None` refuses at the SALT site too.

★ **BOTH oracles are blind on this leg** — taxcalc's `MiscDed` and its SALT phase-down both use
`c00100`/`posagi` directly as the MAGI proxy; OTS hardcodes its worksheet add-back line to `0.0` under
a printed disclaimer. **So the add-back leg is ZERO-oracle and must be held by a KAT against the
worksheet, never by reconciliation.** A filer with $130,000 of §911-excluded income and $30,000 of
SALT deducts $30,000 instead of $16,000, and both engines agree with the wrong answer.

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
$4,000.

★ **The disqualification is narrowed to those three CELLS, not all of Part IV (r2-1).** All three live
**downstream of line 27**, so whenever MAGI ≤ the threshold, line 27 = 0 ⇒ line 29 = 0 and OTS's line
30 is **exact** — except for MFS, where D-7's blanket bar applies instead. A blanket bar is not "computed from the defect's own mechanism" as §6.2 requires — and
a blanket bar would also discard OTS where it is exact:

★★ **AND TAXCALC IS WRONG FOR QSS IN PART IV.** `AutoLoanInterestDed_ps` = `[100000, 200000, 100000,
100000, 200000]` — it doubles the threshold for QSS, where Schedule 1-A line 26 says "$100,000
(**$200,000 if married filing jointly**)". It is an outlier inside taxcalc's own family: `TipIncomeDed_ps`,
`OvertimeIncomeDed_ps` and `SeniorDed_ps` all leave QSS at the unmarried figure. OTS gets it right
(`if (status == MARRIED_FILING_JOINTLY) 200000 else 100000`, `:1885-1888`).

★ **The instructions settle it beyond the form's parenthetical**: `i1040gi--2025.pdf` states the
threshold as "• Married filing jointly—**$200,000**. • **All other filing statuses—$100,000**." QSS is
an "other filing status", so taxcalc is definitively wrong rather than arguably so.

**So: QSS Part IV inside the phase-out region has ZERO valid oracles** — taxcalc has the wrong
threshold, OTS has the wrong rate and rounding. At QSS / MAGI $110,000 / $10,000 interest the form
gives $8,000, OTS $7,000, taxcalc $10,000. **That cell ships zero-oracle and the §6.2 census must say
so**, exactly as it does for the AMT's MFS region.

---

## 4. Non-goals

- **TY2026 anything** (D-2), including transcribing the draft's constants.
- **State returns.**
- **Tier-2 AMT attachment (E4/E5/E6).** The Tier-1 refusal stays for both years.
- **Amended/prior-year interaction.** **Backfilling TY2017** full-return params.

---

## 5. Scope

### 5.0 ★★ ORDERING IS A SAFETY PROPERTY — TY2025 MUST FAIL CLOSED UNTIL IT IS WHOLE

`full_return_for(year) → Some` is the **only** year gate on the full-return path
(`btctax-cli/src/cmd/tax.rs:499`, `session.rs:517`/`560`, `resolve.rs:264`), and the one consistency
guard (`input_form_store.rs:312`, `if table.year != year || params.year != year`) starts **passing**
the moment a `FullReturnParams { year: 2025, .. }` is bundled.

So bundling §5.1 alone does not produce a refusal — it produces **plausible wrong numbers**: TY2025
computed with Form 6251 line 1 in 2024 numbering, `AbsoluteReturn.line14` as "L12 + L13", a scalar
`salt_cap`, and no Schedule 1-A. D-2 gives TY2026 a mutation-verified fail-closed test; nothing gives
*partially built* TY2025 one.

★ **Requirement: a `ty2025_full_return_must_stay_fail_closed_until_complete` test, mutation-verified,
deleted only in the final work item.** That single addition is also what makes this shippable
incrementally instead of whole-or-nothing.

★ **§5 IS A DEPENDENCY ORDER, and the shared MAGI surface comes FIRST.** §5.1's SALT worksheet reads
its line 4 from lines 3a–3d, which are the same inputs as Schedule 1-A Part I lines 2a–2d (D-6). There
is no computational cycle — Schedule 1-A itself reads only AGI — but the MAGI *input surface* must
exist before either consumer. Build order: MAGI surface → SALT worksheet → Schedule 1-A parts.

### 5.1 Constants: `FullReturnParams` for 2025

Each field carries its own citation under the §2 per-field rule. **Two are OBBBA, not the Rev. Proc.:**

★ **`std_deduction` — OBBBA, verified against the 2025 Form 1040's own margin: 15,750 single /
31,500 MFJ / 23,625 HoH / 15,750 MFS.** *Not* Rev. Proc. 2024-40's 15,000 / 30,000 / 22,500. Both
oracles agree (OTS `S_STD_DEDUC = 15750.0`; taxcalc `STD` 2025 = `[15750, 31500, 15750, 23625,
31500]`). Typing the Rev. Proc. figures overstates **every** filer's taxable income by $750–$1,500,
and the existing `ty2025_..._match_rev_proc_2024_40` test covers **brackets only** and would not
notice. **Requires its own test against the four values.**

★ **`salt_cap` is NOT a scalar — and it is NOT a 4-field struct either (r1-1, r2-7).**
`salt_cap: Usd` (`tables.rs:326`, consumed at `return_1040.rs:322-327` and `printed.rs:1145` as a bare
`min`) cannot express TY2025 §164(b). But the r2 draft's fix — a per-status
cap/rate/threshold/floor sub-structure — was **taxcalc's parameterization, not the form's**, and the
standing transcription rule *explicitly covers worksheets*.

**The instrument is the Schedule A instructions' "State and Local Tax Deduction Worksheet", now
archived, and it is transcribed line by line in its own numbering:**

| line | printed text (verbatim, abridged only where marked …) |
|---|---|
| 1 | "Is the amount on Schedule A, line 5d more than $10,000 ($5,000 if married filing separately)?" — **No ⇒ STOP**, deduction isn't limited. **Yes ⇒ Enter $40,000** |
| 2 | "Enter the amount from Form 1040 or 1040-SR, line 11b" |
| 3a–3d | excluded Puerto Rico income · Form 2555 line 45 · Form 2555 line 50 · Form 4563 line 15 |
| 3e | "Add lines 3a through 3d" |
| 4 | "Add lines 2 and 3e" ← **this is the MAGI of D-6, identically** |
| 5 | "Enter $500,000 ($250,000 if married filing separately)" |
| 6 | "Is the amount on line 4 more than the amount on line 5?" — No ⇒ skip 7–8, use line 1 |
| 7 | "**Multiply line 6 by 30% (0.30)**" |
| 8 | "Subtract line 7 from line 1" |
| 9 | "Enter the **larger** of the amount on line 8 or **$10,000**" |
| 10 | "State and local tax deduction. Enter the **smaller** of the amount on line 9 (**half** the amount on line 9 if married filing separately) or the amount from Schedule A, line 5d here and on Schedule A, line 5e" |

★★ **THE WORKSHEET DECIDES MFS, AND IT IS A FINAL HALVING — NOT PER-STATUS CONSTANTS.** The r2 fold
adopted the worksheet as the instrument but transcribed it only through line 7, leaving the
taxcalc-shaped quadruple standing as the only MFS rule two sentences after forbidding it. Lines 8–10
settle it: **line 1's $40,000 cap and line 9's $10,000 floor are NOT halved** — only line 10 halves,
and it then takes the smaller of that and the SALT actually paid.

The two shapes give different answers, so this is not cosmetic. **MFS at MAGI $300,000: the worksheet
gives $12,500** (line 6 = 50,000 → line 7 = 15,000 → line 8 = 25,000 → line 9 = 25,000 → halved =
12,500); **the quadruple gives $5,000** — and $300,000 is the point of *maximum* divergence between
them. **The MFS floor therefore begins at MAGI $350,000**, not $300,000.

★ **Every parameter is transcribed to a worksheet LINE**, not to prose: cap $40,000 (line 1),
threshold $500,000 / $250,000 MFS (line 5, the one genuinely per-status value), rate 30% (line 7),
floor $10,000 (line 9), MFS halving (line 10). taxcalc's `ID_AllTaxes_c` = `[40000, 40000, 20000,
40000, 40000]` folds the halving into the cap; it is a witness, not the source, and it does not agree
with the worksheet for MFS.

★ **Note the short-circuit at line 1**: a filer with ≤ $10,000 of SALT is never limited at all, so the
phase-down and floor are unreachable for them. A derived `max(cap − rate × excess, floor)` reproduces
that by accident; the worksheet states it.

**Remaining fields — 7 of 11 now have a value or a definitive answer** (from §2a's recon, re-verify at
write time):

| field | TY2025 | source |
|---|---|---|
| `std_aged_blind_married` | **1,600** | recon 01, §63(f) |
| `std_aged_blind_unmarried` | **2,000** | recon 01, §63(f) |
| `dependent_std_floor` | **1,350** | recon 01, §63(c)(5) |
| `dependent_std_earned_addon` | **450** | recon 01, §63(c)(5) |
| `qbi_ti_threshold_unmarried` | **197,300** | recon 03, §199A(e)(2) / Rev. Proc. 2024-40 |
| `qbi_ti_threshold_married` | **394,600** | recon 03, §199A(e)(2) / Rev. Proc. 2024-40 |
| `ftc_ceiling` | **300 / 600 MFJ, unchanged** | §904(j) is statutory and **not** indexed |

★ **AND THE LAST FOUR ARE CLOSED TOO — they were never fetches.** They are all printed in
`i1040gi--2025.pdf`, which §2 already archives. Read, not deferred:

| field | TY2025 | quoted from `i1040gi--2025.pdf` |
|---|---|---|
| `kiddie_unearned_threshold` | **2,700** | "You had more than **$2,700** of unearned income" |
| `elective_deferral_limit` | **23,500** | "…deferred for under all plans was more than **$23,500** (excluding catch-up contributions)" — §402(g)(1) |
| `student_loan_phaseout_unmarried` | **(85,000, 100,000)** | start "…surviving spouse—**$85,000**"; "Divide line 6 by **$15,000**" |
| `student_loan_phaseout_married` | **(170,000, 200,000)** | start "Married filing jointly—**$170,000**"; "…(**$30,000** if married filing jointly)" |

**Every `FullReturnParams` field now has a value and a citation. Nothing in §5.1 is outstanding.**

`AmtParams` for 2025 — **all cells verified against `f6251--2025.pdf` in review r1**:

| | single/hoh | mfj/qss | mfs |
|---|---|---|---|
| exemption | 88,100 | 137,000 | 68,500 |
| phase-out start | 626,350 | 1,252,700 | 626,350 |
| 26/28% breakpoint | 239,100 | 239,100 | 119,550 |
| 28% subtrahend | 4,782 | 4,782 | 2,391 |
| line 19 † | 48,350 (hoh 64,750) | 96,700 | 48,350 |
| line 25 † | 533,400 (hoh 566,700) | 600,050 | 300,000 |

† **Not `AmtParams` cells** — these are `LtcgBreakpoints.max_zero`/`max_fifteen`, read from `bp` at
`form6251.rs:372`/`378`, and **already bundled for TY2025** with exactly these values in `ty2025()` and
already asserted by `ty2025_ltcg_breakpoints_all_statuses`. §6.6 must not have an implementer adding
duplicate fields to `AmtParams`, which deliberately does not carry them.

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
| VI | 38 | Total Additional Deductions | — | — | — |

★ **Part II line 5's printed net-profit cap is DEFINED by the instructions** — the form prints "Do not
enter more than the net profit from the trade or business", and the instructions' *Net income
limitation* says what that net profit means. **One cap, elaborated — not two** (an earlier draft said
the form did not state it, which was wrong): trade-or-business tips cannot exceed that business's gross income minus all deductions
allocable to it (including the deductible part of SE tax, SEP/SIMPLE/qualified-plan contributions and
self-employed health insurance, but **not** the tips deduction itself). Found by reading the
instructions; no amount of reviewing the *form* would have surfaced it.

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
valid-SSN. ★ **Occupation eligibility is IN the instructions** — a list keyed by numeric **Treasury Tipped
Occupation Code** (see §2's r1-8 note, which retracted the earlier "not a document at all" claim).

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
- ★ **`scripts/oracle/corpus.py` is a year seam too (r2-5).** Its SALT axis tops out at
  `$8,000 + $9,000 = $17,000` — built for a $10,000 cap, and therefore a **no-op against TY2025's
  $40,000 cap**. The axis that exists to test the cap stops testing anything.
- ★ **The fixture's LINE SET is per-year, not just a `year` tag.** The Rust KAT is
  `let lines: [(&str, Usd); 41]` — closed at both ends and **indexed**, so a renamed key panics by
  design — and `verify_f6251.py` sums `("line1","line2a","line2b","line3")` and holds a hardcoded
  `STANDARD_DEDUCTION_2024`. TY2025 replaces `line1` with `line1a`/`line1b`, so the array forks to
  **42** entries and the gap sizing becomes per-year. That 41-entry array is the thing that forks.
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

★ **Plus at least one TY2025 MFS AMT-owing vector BELOW the kicker start.** `_taxcalc_expected_gaps`
disqualifies taxcalc on every vector whose line 4 exceeds lines 1+2a+2b+3 — i.e. on exactly the
above-kicker vectors — so a TY2025 fixture containing only those makes MFS a one-oracle status and
**reds §6.3's own census**. TY2024's V22 is precisely this vector; TY2025 needs its analogue.

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
   AMT-owing vector agreed by two oracles, **or a D-10 citation**: an IRS worked example pinned as a
   KAT satisfies this in place of a second oracle, since D-10 ranks it the stronger evidence. Without
   that clause §6.3 and D-10 disagree about what counts.
4. **Schedule 1-A reconciles part by part**, including **each phase-out's knee and its cap**, and
   **across all five filing statuses** (r1-4) — the MFS bar on II/III/V is a required case, not an
   incidental one.
5. **The SALT worksheet reconciles PER FILING STATUS, with a household in each region** (r1-1, r2-5,
   r2-6): unlimited (≤ $10,000 of SALT), capped-not-phased, phasing, and **at the floor**
   (MAGI ≥ $600,000; **≥ $350,000 MFS** — the floor is reached where worksheet line 8 falls to $10,000,
   i.e. MAGI ≥ 250,000 + 30,000/0.30; the earlier "$300,000 MFS" was computed from the rejected
   quadruple and is the point of *maximum* divergence from the worksheet, so it is worth its own
   household too). One household above the threshold witnesses one point of a five-line × five-status
   rule. ★ **Pin each household's filing status explicitly** — OTS 2025's
   SALT worksheet is itself MFS-defective, so an unpinned "some household" can land on a cell with no
   sound witness.
6. **A TY2025 analogue of `ty2024_full_return_params_bundled` asserting EVERY `FullReturnParams` and
   `AmtParams` cell against its §5.1 citation** (r2-4). The `std_deduction` test is **one field of
   about twenty**; the §55(d)(3) identities constrain only the five MFS constants; §5.5 scopes the
   TY2025 vectors to MFS; and the corpus has 0/104 AMT-owing households. Nothing else would notice a
   mistyped non-MFS exemption or breakpoint.
7. ★ **EVERY ZERO-ORACLE CELL HAS A KAT AGAINST THE INSTRUCTIONS** (D-6, D-10). §6 previously
   contained no KAT requirement at all, while D-6 declares the MAGI add-back leg zero-oracle and
   "held by a KAT against the worksheet, never by reconciliation", and D-10 makes cite-then-KAT the
   standing answer to a split oracle. The known set today: the MAGI add-back leg, QSS Part IV inside
   the phase-out band, and the SALT MFS halving. ★ **§6.2's escape rule extends to Schedule A / SALT,
   not only Schedule 1-A and Form 6251**, and its census line reads *zero*-oracle as well as
   one-oracle — D-8 and D-10 now ship cells with no witness at all.
8. **Conformance is a test, not a review** — "is every Schedule 1-A line present?", "does each doc
   comment match the printed text?" are assertions.
9. **The TY2025 form assets exist and read back** (§5.4, §8a B4) — 12 new PDFs with AcroForm maps plus
   the two partial 2025 maps rebuilt. §4 does not exclude them, so §6 asserts them.
10. **Mutation-verified guards.** Precedent from this session: the `phaseout_rate` split passed every
   existing test until a mutation showed the whole suite was blind to it.

---

## 7. Risks

**R-1. The transcription is bigger than the constants.** Form 6251 Part I, Form 1040, and all of
Schedule 1-A change shape. "Type in new numbers" is the failure mode, and the r1 draft made exactly
that error.

**R-2. Phase-out step arithmetic**, with opposite rounding directions per part (§5.2) — details a
paraphrase smooths over, and they change the answer at every knee.

**R-3. Every oracle claim is read from source, or it is not made.** OTS 2025 is verified on §55(d)(3),
the §170(b) charitable ceiling, and Schedule 1-A Part IV — **which was read and is DEFECTIVE** (D-8).
The r1 draft listed "read `sched_1A()` before trusting it" as a risk and then relied on it as a fact;
r2 caught the same pattern again, in the opposite direction, where the r2 draft asserted the SALT
parameters were "corroborated by taxcalc only" without reading OTS's SALT worksheet — **OTS implements
it too, and is MFS-defective** (r2-6). ★ **Still UNREAD and therefore not yet claimable: OTS's
Schedule 1-A Parts II, III and V, and its SALT worksheet's non-MFS legs.** Read before use.

**R-4. Scope creep into TY2026.** D-2 and its mutation-verified test are the control.

**R-5. A constant carried forward silently.** Every field gets a citation, or it does not ship.

**R-6. A "see instructions" branch resolved by guessing** (§5.2) — a conformance test against the
*form* passes on every one of them.

---

## 8. Open questions for review

1. **The per-year form-shape mechanism** (D-4) — per-year structs, versioned parts, or an enum? The
   plan decides; the spec forbids branching inside one struct.
2. ~~Does the golden corpus extend to TY2025?~~ — **CLOSED: yes, it must.** §6.5 requires TY2025 SALT
   households across four regions × five statuses, which only the corpus can supply, and `corpus.py`'s
   SALT axis needs raising regardless (§5.3). Scheduled to **§8a B4**.
5. ~~How is a zero-oracle cell shipped?~~ — **ANSWERED by D-9 and D-10.** Cite the IRS instructions,
   KAT it, census it, and register the suspected oracle defect upstream. Nobody is refused for an
   oracle's shortcoming, and nobody is refused for a line we can simply collect.
3. ~~Part IV VIN storage vs. the PII scanner~~ — **CLOSED: out of scope for the generic scanner, and
   here is why.** `pii-scan-generic.sh` matches only SSN and EIN shapes and scans **HEAD** — its job is
   catching PII accidentally committed to *source*. A VIN reaches btctax through runtime input, never a
   source file, and a 17-character alphanumeric pattern would false-positive on hashes and short SHAs.
   **Instead:** the Schedule 1-A emitter's tests assert no VIN-shaped literal appears in any committed
   fixture. Recorded rather than left as a decision, since D-9 makes VIN collection mandatory.
4. ~~Does OTS 2025 apply the §170(b) cash ceiling?~~ — **answered: yes**, `taxsolve_US_1040_2025.c:2424`
   caps `charityCC` at 60% of AGI. That leg of the disqualification predicate must therefore be
   year-scoped (§5.3), not merely re-used.

---

## 8a. ★ SCALE — four branches, not one

The r3 buildability check sized this against the codebase. **Cut points, in dependency order:**

| | branch | why it cuts here |
|---|---|---|
| **B1** | harness year seams (§5.3) | changes no signed output; prerequisite for validating everything after it |
| **B2** | TY2025 numbers + form shapes (§5.1, D-3, D-4, the SALT worksheet) | the `salt_cap` type change alone `E0063`s six literals — `tax_tables.rs:128`, `qbi.rs:318`, `return_refuse.rs:913`, `advisories.rs:422`, `testonly.rs:65`, `return_1040.rs:1647` |
| **B3** | Schedule 1-A end to end (§5.2, D-9) | 38 lines, 6 parts, ~25 collected inputs across `return_inputs.rs`, `classifier.rs`, `questions.rs`, the input-form spec, the TUI, CLI, docs — **its own spec-sized feature** |
| **B4** | filing assets + corpus (§5.4, §6.5) | 12 new PDFs/maps **plus rebuilding two existing ones** — `2025/f1040.map.toml` is 10 lines vs 2024's 168, and `2025/schedule_d.map.toml` 11 vs 55; both are pseudo-slice maps, not full-return ones |

★ **§5.0's fail-closed test is what makes this incremental rather than whole-or-nothing.** Without it,
any partial landing opens TY2025 with wrong numbers. With it, B1–B4 can ship in sequence and the gate
comes out in B4.

★ **B3 should get its own SPEC.** D-1 ("all SIX parts") and D-9 ("every numbered line") make Schedule
1-A larger than the rest of TY2025 combined, and this document does not size its input-registration
surface — the r3 check found that `classifier.rs` **permits** `_` on money leaves by its own stated
rule, so "`None` refuses" for ~25 `Option<Usd>` fields would be held by convention, which is the exact
defect class D-5 exists to prevent.

---

## 9. Cross-references

- ★ `design/full-return/recon/fable/` — **sourcing of record, read before deriving** (§2a).
- `design/amt-form6251/CONTINUITY_TY2025.md` — the recon, the pivot, the environment notes.
- `design/ty2025/reviews/` — r1, r2 and the r3 sanity check, verbatim.
- `FOLLOWUPS.md` §G-6b (Tier-2 entry criteria), §G-6e (this pivot), §G-6f (the rate split, done).
- `STANDARD_WORKFLOW.md` — the review loop this must pass before a plan is written.
