# Fable Recon F1 — TY2025 Final Forms + OBBBA Statutory Figures

**Agent:** Fable second pass, Agent F1. **Date:** 2026-07-11.
**Task:** resolve the two biggest opus flags (`00-SYNTHESIS.md` §7 items 1–2): (1) the final 2025
Schedule A structure; (2) exact OBBBA dollar amounts against the enacted statute + final 2025 IRS
publications. Plus re-verify the 2025 Form 1040 page-2 renumbering and the Schedule 1-A structure
on the FINAL forms (the opus pass had only drafts).

**Verdict up front:** Both opus flags **RESOLVE GREEN**. Every dollar figure the opus pass carried
is **CONFIRMED** against the enacted Public Law 119-21 and the final 2025 IRS forms/instructions —
with **ONE dollar-affecting CORRECTION**: the opus closed-form SALT formula gets the **MFS
phase-down mechanics wrong** (the IRS worksheet halves the cap *last*, producing an effective
15%-slope/$350k-floor-point for MFS, not the 30%-slope/$300k the opus "halve both constants"
footnote implies). Detail in §2.2. **GO** on bundling the TY2025 tables (§5).

Every claim below was read directly from the primary PDF (via `pdftotext`), not from memory or
secondary sources. Source manifest with revision dates in the Appendix. Note: the task brief said
"engrossed text"; I verified against the **enrolled, enacted slip law** (PLAW-119publ21, 139 Stat.
72) — strictly better (it is the law as signed 2025-07-04).

---

## 1. Final 2025 Schedule A — opus flag #1 RESOLVED

**The final TY2025 Schedule A now exists and I read it:** `irs.gov/pub/irs-prior/f1040sa--2025.pdf`,
PDF title "2025 Schedule A (Form 1040)", CreationDate **2025-12-18**, form-footer layout stamp
"Created 11/20/25", no draft banner. (The opus pass ran before this was published and could only
get the 2026-rev draft at the `irs-dft` URL — its inference is now fully confirmed.)

### 1.1 Structure: 2025 = 2024 + SALT numbers only

| Lines | 2025 final content | vs 2024 |
|---|---|---|
| 1–4 | Medical; line 3 = 7.5% × line 2 (AGI) | unchanged (line 2 now reads 1040 **line 11b**) |
| 5a–5e | SALT; **5e = smaller of 5d or $40,000 ($20,000 MFS)**, with "if line 11b is more than $500,000 ($250,000 MFS) … see instructions" | **only substantive change** (2024: $10,000/$5,000, no phase-down) |
| 6–7 | Other taxes; 7 = 5e + 6 | unchanged |
| 8a–8e, 9, 10 | Mortgage interest (8a Form-1098, 8b not-on-1098, 8c points, **8d "Reserved for future use"**, 8e sum), 9 investment interest/4952, 10 total | unchanged (PMI on 8d is the **2026** rev, not 2025) |
| 11–14 | Charitable: **11 cash/check · 12 other-than-cash (8283 if >$500) · 13 carryover · 14 total** | **unchanged — identical to 2024** |
| 15 | Casualty/theft (federally declared disaster, 4684) | unchanged |
| 16 | Other — from list in instructions (generic) | unchanged |
| 17 | Total → **Form 1040 line 12e** | destination renumbered (was L12) |
| 18 | Elect-to-itemize checkbox (§63(e)) | unchanged |

The line-5e and line-17 text on the form itself independently corroborates the 1040 **L11b** and
**L12e** renumbering (§3).

### 1.2 The three "2026 scare" items are statutorily TY2026 — CONFIRMED, not on 2025

Confirmed twice each: (a) absent from the final 2025 form + 2025 Schedule A instructions, and
(b) enacted effective dates read from Pub. L. 119-21:

| Item | OBBBA § | Codified as | Effective (verbatim) | On 2025 Sch A? |
|---|---|---|---|---|
| 0.5%-AGI charitable floor (individuals) | **§70425** | new §170(b)(1)(I) | "taxable years beginning after **December 31, 2025**" | **NO** — 2025 instructions' charitable section keeps the 2024 Pub-526-deferral structure verbatim (30%/20% triggers, 5-yr carryover) |
| Gambling-loss 90% limit | **§70114** | §165(d) rewritten ("90 percent of … losses" + to-extent-of-gains) | "taxable years beginning after **December 31, 2025**" | **NO** — 2025 line-16 instructions: losses deductible "only **to the extent of** gambling winnings reported on Schedule 1 … line 8b" (100%, unchanged rule) |
| §68-style itemized-deduction cap | **§70111** | §68 replaced: itemized reduced by **2/37** of the lesser of (itemized) or (TI + itemized − start of the 37% bracket) | "taxable years beginning after **December 31, 2025**" | **NO** — 2025 line 18 is still the elect checkbox; no limitation line/worksheet. (The "$384,350" the opus pass saw on the 2026 draft is that year's 37%-bracket start — consistent with new §68.) |

The 2025 Schedule A instructions "What's New" lists exactly **two** changes: the SALT limit and a
pointer to new Schedule 1-A. Nothing else. → **The v1 TY2024 Schedule A engine carries to TY2025
with only year-keyed SALT parameters + the new SALT worksheet** (§2.2).

---

## 2. OBBBA statutory figures — opus flag #2 RESOLVED (one correction)

### 2.1 Per-figure confirmed/corrected table

All statutory quotes from Pub. L. 119-21 (139 Stat. 72), all IRS figures from the final 2025
publications (Appendix). Every opus **value** confirmed; one opus **formula** corrected.

| Figure | Opus claim | Verdict | Primary evidence |
|---|---|---|---|
| Std deduction Single/MFS | $15,750 | **CONFIRMED** | §70102(b)(2): §63(c)(7) "$12,000" → "**$15,750**"; final 1040 margin; Pub 501 Table 6 area; 1040-instr What's New |
| Std deduction MFJ/QSS | $31,500 | **CONFIRMED** (see nuance) | Statute never prints $31,500 — it flows from §63(c)(2)(A)'s **200%-of-single** rule (2 × $15,750). IRS prints $31,500 everywhere (1040 margin, Pub 501, instructions) |
| Std deduction HoH | $23,625 | **CONFIRMED** | §70102(b)(1): "$18,000" → "**$23,625**" |
| Effective for TY2025 (retroactive) | yes | **CONFIRMED** | §70102(c): "taxable years beginning after **December 31, 2024**" |
| Future indexing | — | NEW DETAIL | §70102(b)(3)–(4): §63(c)(7)(B) inflation base year reset **2017 → 2024**; amounts index from 2026 |
| §63(f) aged/blind per box, married (MFJ/MFS/QSS) | $1,600 | **CONFIRMED** | 2025 1040-instr Std-Ded Worksheet line 4b "multiply … by **$1,600** ($2,000 if single or HoH)"; Pub 501 examples ($31,500+$1,600=$33,100; 4 boxes → $37,900). OBBBA did **not** disturb §63(f) — Rev. Proc. 2024-40 values stand |
| §63(f) aged/blind per box, unmarried (S/HoH) | $2,000 | **CONFIRMED** | same worksheet; Pub 501 filing chart single 65+ = $17,750 = 15,750+2,000 |
| Dependent std floor / earned add-on | $1,350 / +$450 | **CONFIRMED** | Pub 501 ("greater of $1,350 or earned + $450"); 1040-instr Worksheet for Dependents (line 2: "earned income more than $900? … add $450 … No → enter $1,350") |
| SALT cap | $40,000 / $20,000 MFS | **CONFIRMED** | §70120(b) new §164(b)(7)(A)(i): "**$40,000**" for TY2025; MFS = "half the applicable limitation amount" (§164(b)(6) as amended); printed on final Sch A line 5e |
| SALT out-year schedule | $40,400 (2026), +1%/yr, $10k in 2030 | **CONFIRMED** | §164(b)(7)(A)(ii)–(iv): $40,400 in 2026; 101%/yr 2027–2029; **$10,000** after 2029 |
| SALT phase-down | 30% of MAGI over $500k / $250k MFS | **CONFIRMED** | §164(b)(7)(B)(i)–(ii) |
| SALT floor | $10,000 ($5,000 MFS effective) | **CONFIRMED** | §164(b)(7)(B)(iii): reduction can't take the amount "less than $10,000"; halving then yields the $5,000 MFS floor the IRS states in the 2025 What's New |
| SALT MFS phase-down **mechanics** | "halve both constants" | **⚠ CORRECTED** — halve **last**, see §2.2 | 2025 Sch A instructions, State and Local Tax Deduction Worksheet (transcribed below) |
| SALT MAGI definition | (unverified in opus) | **RESOLVED** | §164(b)(7)(B)(iv): AGI + §911/§931/§933 exclusions; worksheet lines 3a–3d = PR excluded income, 2555 L45/L50, 4563 L15. **For btctax scope, MAGI = AGI** (fail-closed on 2555/4563/PR inputs, matching deep/02's posture) |
| Senior deduction | $6,000, §70103 | **CONFIRMED** | §70103(a)(3), codified **new §151(d)(5)(C)** (inside "Termination of deduction for personal exemptions **other than temporary senior deduction**") — NOT a §63(f) change; it's a separate deduction stacking on top, exactly as opus described behaviorally |
| Senior phase-out | 6% of MAGI over $75k / $150k MFJ | **CONFIRMED** | §151(d)(5)(C)(iii); MAGI = AGI + §911/931/933. Fully phased at $175k single / $250k MFJ (per-person amount hits 0) |
| Senior window / gates | 2025–2028 | **CONFIRMED + NEW** | TYB before 1/1/2029, effective after 12/31/2024; **SSN required** (omission = math error, §6213(g)(2)(W)); **married must file jointly** (§151(d)(5)(C)(v)); max **$12,000 MFJ** both 65+ (1040-instr What's New) |
| Tips deduction | $25k cap, $100/$1,000 over $150k/$300k | **CONFIRMED + NEW** | new **§224** (§70201): cap **$25,000 — NOT doubled for MFJ**; phase-out $100 per **full** $1,000 (no "or portion thereof" → round quotient **down**, form agrees); SSN + MFJ-required; 2025–2028 |
| Overtime deduction | $12,500 / $25,000 MFJ | **CONFIRMED** | new **§225** (§70202): same $150k/$300k phase-out, $100 per full $1,000 (round **down**); SSN + MFJ-required; 2025–2028 |
| Car-loan interest | $10k cap, $200/$1,000 over $100k/$200k | **CONFIRMED + NEW** | §70203 (§163(h) QPVLI): "$200 for each $1,000 **(or portion thereof)**" → round quotient **UP** (form line 28 agrees: "increase … to the next higher whole number"). Loans after 12/31/2024; TYB 2025–2028 |

### 2.2 ⚠ CORRECTION — the SALT MFS phase-down halves LAST (dollar-affecting)

Opus `02-computation-worksheets.md` §2b gave
`cap = max(10_000, 40_000 − 0.30 × max(0, MAGI − 500_000))` — **correct for non-MFS** — with the
footnote *"halve both constants for MFS"*, i.e. `max(5_000, 20_000 − 0.30 × (MAGI − 250_000))`.
That is **not** what the statute or the IRS worksheet does.

**The authoritative implementation** — 2025 Instructions for Schedule A, "State and Local Tax
Deduction Worksheet" (line 5e), transcribed:

```
Before you begin: if Sch A line 5d ≤ $10,000 ($5,000 MFS) → 5e = 5d, stop (never limited).
Entry condition: worksheet needed only if 1040 line 11b > $500,000 ($250,000 MFS) or
                 Form 2555 / Form 4563 / PR exclusion present; else 5e = min(5d, $40,000/$20,000 MFS).
 1.  $40,000                                   ← NOT halved for MFS
 2.  1040 line 11b (AGI)
 3a–3e. + PR exclusion, 2555 L45, 2555 L50, 4563 L15
 4.  MAGI = L2 + L3e
 5.  $500,000 ($250,000 MFS)                   ← threshold IS halved
 6.  excess = L4 − L5 (if ≤ 0: skip, L9 = L1)
 7.  0.30 × L6
 8.  L1 − L7
 9.  max(L8, $10,000)                          ← floor on the UNhalved amount
10.  5e = min( L9  [HALF of L9 if MFS] , 5d )  ← halve LAST
```

So the exact rule (matches §164(b)(6)+(7) word for word):

```
cap        = max(10_000, 40_000 − 0.30 × max(0, magi − threshold))   # threshold = 500k, 250k MFS
cap_final  = mfs ? cap / 2 : cap                                     # halve LAST
line_5e    = min(cap_final, line_5d)
```

Algebraically for MFS: `cap_MFS = max(5_000, 20_000 − 0.15 × (MAGI − 250_000))` — an **effective
15%** slope reaching the $5,000 floor at MAGI **$350,000** (not 30%/$300,000).

**Worked divergence (MFS, MAGI $300,000, 5d = $30,000):** IRS worksheet → L7 = $15,000, L8 =
$25,000, L9 = $25,000, 5e = min(½·25,000, 30,000) = **$12,500**. Opus footnote formula →
max(5,000, 20,000 − 15,000) = **$5,000**. A **$7,500** deduction difference across the whole
$250k–$350k MFS band. Non-MFS filers are unaffected (both formulas identical); the endpoints
($20,000 cap, $5,000 floor, $250,000 threshold) were all stated correctly by opus — only the
mid-band slope/ordering was wrong.

**Spec directive:** implement the worksheet order (floor on the unhalved $40k amount, halve last),
not a "halved constants" closed form. KAT the $300k-MFS example above.

---

## 3. Final 2025 Form 1040 renumbering + final Schedule 1-A — re-verified

### 3.1 Form 1040 (2025) — FINAL (`f1040--2025.pdf`, CreationDate **2026-01-02**, layout stamp
"Created 9/5/25" — i.e., the final kept the draft layout the opus pass read; no draft banner)

Every opus renumbering claim (`01-form-graph.md` §5a) **CONFIRMED verbatim on the final**:

| Concept | 2025 line (final) | Note |
|---|---|---|
| Capital gain/(loss) | **7a** | + **7b** two checkboxes: "Schedule D not required" / "Includes child's capital gain or (loss)" |
| Total income | 9 = 1z+2b+3b+4b+5b+6b+**7a**+8 | |
| AGI | **11a** (p.1) and **11b** (top of p.2, "Amount from line 11a") | Sch A lines 2/5e and Sch 1-A line 1 read **11b** |
| Someone-can-claim boxes | **12a** | moved from 2024 header |
| Spouse itemizes / dual-status | **12b** and **12c** | **REFINEMENT:** 2024's single combined checkbox is **split into two** (12b "Spouse itemizes on a separate return", 12c "You were a dual-status alien") — field-map relevant (F5); also relevant to the §63(c)(6) MFS-coupling input, which now has its own dedicated box |
| Age/blindness (4 boxes) | **12d** | cutoff "born before **January 2, 1961**" ✓ |
| Std/itemized deduction | **12e** | margin prints $15,750 / $31,500 / $23,625 |
| QBI | **13a** | |
| Schedule 1-A total | **13b** | "Additional deductions from Schedule 1-A, **line 38**" (1040-NR: line 13c) |
| Sums | 14 = 12e+13a+13b; 15 = 11b−14 | taxable income still **line 15** |
| EIC | **27a** + 27b clergy-SE checkbox + 27c EIC-opt-out checkbox | |
| Refundable adoption credit | **30** (Form 8839 line 13) | was "Reserved" in 2024 |
| Other payments total | 32 = 27a+28+29+**30**+31 | |

**Schedules 1/2/3 spine unchanged** (verbatim on the final): L8 ← Sch 1 L10 · L10 ← Sch 1 L26 ·
L17 ← Sch 2 L3 · L20 ← Sch 3 L8 · L23 ← Sch 2 L21 · L31 ← Sch 3 L15 · payments spine
24/25a–d/26/33/34/35a/36/37/38 unchanged. 2025 instructions confirm **Form 8959 line 24 withholding
→ included in line 25c** (unchanged).

**NEW-in-final header/field inventory deltas** (not in the opus draft read; hand to **F5** for the
TY2025 field maps): top-of-form checkboxes ("Filed pursuant to section 301.9100-2", "Combat zone",
"Deceased" + MM/DD/YYYY for taxpayer/spouse, "Other"); "main home in the U.S. more than half of
2025" checkbox (EIC eligibility); NRA/dual-status-spouse election checkbox + name; the Dependents
block is restructured into **numbered rows (1)–(7)** per dependent column (first name / last name /
SSN / relationship / lived-with-you-more-than-half-2025 (a) Yes (b) "And in the U.S." / full-time
student / permanently-and-totally-disabled / CTC vs ODC) — a much bigger grid than 2024; an
MFS/HOH lived-apart-6-months checkbox below Dependents; line 3c (child's dividends included on
3a/3b checkboxes); 4c (Rollover/QCD), 5c (Rollover/PSO); 6c lump-sum election, 6d MFS-lived-apart;
line 26 former-spouse SSN entry space for joint estimated payments. 2025 digital-asset question
wording = 2024 (year token aside).

### 3.2 Schedule 1-A (2025) — FINAL (`f1040s1a--2025.pdf`, CreationDate **2026-01-02**, layout
stamp "Created 11/4/25" — the final IS the draft the opus pass read, ratified)

Part/line structure **CONFIRMED exactly** as opus `01-form-graph.md` §5c; spec-grade additions:

| Part | Lines | Confirmed math + NEW details |
|---|---|---|
| I — MAGI | 1–3 | L1 = 1040 **L11b**; 2a PR excluded income, 2b Form 2555 L45, 2c 2555 L50, 2d Form 4563 L15; **L3 MAGI = L1 + 2e**. Same MAGI definition as SALT worksheet and §151(d)(5)(C) — one shared derivation; for btctax = AGI, fail-closed on 2555/4563/PR |
| II — Tips | 4–13 | 4a W-2 box 7 (caution if box 5 > **$176,100** = 2025 SS wage base — ties to engine's `ss_wage_base`); 4b Form 4137; 5 trade-or-business tips (1099-NEC box 1 / 1099-MISC box 3 / 1099-K box 1a, **capped at net profit**); 7 = min(L6, **$25,000**) — flat, not doubled MFJ; 10–12 phase-out: excess over **$150,000/$300,000 MFJ**, ÷$1,000 **rounded DOWN to whole number**, × **$100**; L13 = max(0, L7 − L12). **Occupation must be on the IRS tipped-occupation list; valid SSN; married → MFJ only** |
| III — Overtime | 14–21 | 14a W-2 box 1 portion, 14b 1099-NEC/MISC; 15 = min(14c, **$12,500 / $25,000 MFJ**); same $150k/$300k phase-out, $100 per full $1,000 (**round DOWN**); SSN; MFJ-required |
| IV — Car loan | 22–30 | 22a/b per-VIN rows with column (ii) carving out interest already deducted on Sch C/E/F; 24 = min(L23, **$10,000**); phase-out over **$100,000/$200,000 MFJ**, ÷$1,000 **rounded UP** ("increase 0.05 to 1"), × **$200**; L30. **Statutory basis for the asymmetry:** §163(h) QPVLI text says "each $1,000 **(or portion thereof)**"; §§224/225 lack that phrase → the form's floor-vs-ceil split is deliberate, not a typo. US-assembled vehicle, loan after 12/31/2024 (instructions-level detail) |
| V — Senior | 31–37 | Phase-out computed **once**: L33 = max-ish(L3 − $75,000/$150,000 MFJ), L34 = 6% × L33, **L35 = $6,000 − L34** (the per-person amount); then **36a/36b applied per qualifying spouse** (born before Jan 2, 1961 + valid SSN) → L37 ≤ **$12,000** MFJ. Married → MFJ only |
| VI — Total | 38 | **L38 = 13 + 21 + 30 + 37 → 1040 line 13b** (1040-NR L13c) ✓ |

**For F3's phase-out math spec:** the quotient-rounding asymmetry (Parts II/III floor; Part IV
ceil) and the "phase-out once, per-spouse twice" senior structure are the two traps.

---

## 4. Carry-through checks on the 2025 finals (Layer-0 / QDCGT / brackets)

- **Tax Table $100k rule unchanged for 2025** (2025 1040 instructions, line-16 methods): "If your
  taxable income is less than $100,000, you **must** use the Tax Table … $100,000 or more, use the
  Tax Computation Worksheet." → the year-independent `tax/method.rs` design (deep/01) carries to
  TY2025 as-is.
- **QDCGT worksheet (2025)** — same 25-line structure; inputs renumber only where 1040 did: L1
  reads 1040 **line 15** (unchanged), L2 reads 3a, L3 reads Sch D 15/16 else **line 7a**; the
  "Sch D not required" checkbox is now **7b**; L22/L24 keep the per-line <$100k Table/TCW split.
- **2025 LTCG breakpoints printed in the final worksheet:** 0%-max **$48,350** S/MFS, **$96,700**
  MFJ/QSS, **$64,750** HoH; 15%-max **$533,400** S, **$300,000** MFS, **$600,050** MFJ/QSS,
  **$566,700** HoH — i.e., **Rev. Proc. 2024-40 stands for 2025 brackets/breakpoints.** OBBBA's
  bracket change (§70101) only tweaks inflation indexing for TYB after 12/31/2025. → Only the
  **standard deduction** and **SALT** are OBBBA-overridden for 2025; everything else in a
  Rev. Proc. 2024-40-sourced `TaxTable` remains valid. (Confirms opus `02` §1 trap #1 precisely.)
- Other TY2025 What's-New items relevant to the program: **CTC $2,200**/child + SSN requirement
  (F3's brief — figure confirmed); refundable adoption credit up to $5,000 (1040 L30); **Form
  1099-DA arrives for 2025 digital-asset broker sales (basis reporting optional in 2025)** — flag
  for the crypto side of the house; 1099-K threshold restored to $20,000/200-transactions.

---

## 5. GO/NO-GO on bundling the TY2025 tables

**GO.** Every number needed for the TY2025 fast-follow is now verified against final, filable IRS
publications AND the enacted statute — nothing rests on drafts or secondary sources anymore.

Bundle now (per-year `TaxTable` for 2025):

| Parameter | Value(s) | Source of record |
|---|---|---|
| Std deduction basic | $15,750 S/MFS · $31,500 MFJ/QSS · $23,625 HoH | PL 119-21 §70102; final 1040 margin; Pub 501 (2025) |
| §63(f) aged/blind per box | $1,600 married · $2,000 unmarried | 2025 1040 instr. worksheet/chart; Pub 501 |
| Dependent std | min(basic, max($1,350, earned + $450)) | Pub 501 (2025); 1040 instr. |
| SALT cap / threshold / floor | $40,000 ($20,000 MFS) / $500,000 ($250,000 MFS) / $10,000 pre-halving | PL 119-21 §70120; 2025 Sch A + worksheet |
| SALT MFS mechanics | **halve last** (worksheet L10) | §2.2 — supersedes opus footnote |
| Senior deduction | $6,000/person 65+, −6% of MAGI over $75k/$150k, MFJ-required, SSN | PL 119-21 §70103 (§151(d)(5)(C)); Sch 1-A Part V |
| Sch 1-A caps | tips $25,000 flat · OT $12,500/$25,000 · car loan $10,000 | §§224/225/163(h); final Sch 1-A |
| Sch 1-A phase-outs | $100 per full $1,000 (floor) over $150k/$300k; car loan $200 per $1,000-or-part (ceil) over $100k/$200k | statute + final form, §3.2 |
| Brackets / LTCG breakpoints / everything else | Rev. Proc. 2024-40 unchanged | §4 |

Conditions attached to the GO:
1. **Use the worksheet-order SALT formula** (§2.2), not the opus closed-form; KAT the MFS
   $300k/$12,500 example.
2. Year-keyed PDF line maps are mandatory (L7a/11a/11b/12a–e/13a/13b/27a/30 etc.) — confirmed
   real on the final; F5 must also absorb the §3.1 header-block restructuring (dependents grid,
   12b/12c split).
3. Sch 1-A MAGI = SALT MAGI = senior MAGI = AGI + §911/931/933 — derive once; keep the planned
   fail-closed on any 2555/4563/PR input.
4. The 0.5% charitable floor, 90% gambling limit, and new-§68 cap are **TY2026** — do NOT bundle
   into 2025; park them as the first TY2026 delta list (§1.2 has the statutory cites ready).

---

## Appendix — source manifest (all read directly via pdftotext this pass)

| Document | URL | Revision evidence |
|---|---|---|
| Schedule A (Form 1040), 2025 FINAL | `irs.gov/pub/irs-prior/f1040sa--2025.pdf` | PDF title "2025 Schedule A"; CreationDate 2025-12-18; layout "Created 11/20/25" |
| Instructions for Schedule A, 2025 | `irs.gov/pub/irs-prior/i1040sca--2025.pdf` | What's New = SALT + Sch 1-A only; SALT worksheet p.7 |
| Form 1040, 2025 FINAL | `irs.gov/pub/irs-prior/f1040--2025.pdf` | Title "2025 Form 1040"; CreationDate 2026-01-02; layout "Created 9/5/25" |
| Schedule 1-A (Form 1040), 2025 FINAL | `irs.gov/pub/irs-prior/f1040s1a--2025.pdf` | Title "2025 Schedule 1-A"; CreationDate 2026-01-02; layout "Created 11/4/25" |
| Form 1040 Instructions, 2025 | `irs.gov/pub/irs-prior/i1040gi--2025.pdf` | Std-deduction worksheet/charts pp. 34–35; QDCGT worksheet p. 38; $100k Tax-Table rule |
| Pub. 501 (2025) | `irs.gov/pub/irs-prior/p501--2025.pdf` | Std-deduction section, filing-requirement chart, worked examples |
| **Pub. L. 119-21** (OBBBA, enacted 2025-07-04) | `govinfo.gov/content/pkg/PLAW-119publ21/pdf/PLAW-119publ21.pdf` | 139 Stat. 72; §§70101, 70102, 70103, 70104, 70111, 70114, 70120, 70201–70203, 70425 read verbatim |

Corrections ledger for the Fable synthesis: **1 correction** (SALT MFS phase-down mechanics,
§2.2); **2 refinements** (2025 1040 splits the 2024 spouse-itemizes/dual-status checkbox into
12b/12c; senior deduction codified at §151(d)(5)(C) with SSN/MFJ gates); **0 opus dollar figures
wrong**. All six `00-SYNTHESIS.md` §8 round-2 corrections were untouched by anything found here.
