# Schedule 1-A (TY2025) — SPEC

**Status: DRAFT r1**, written 2026-07-29. Spun out of `design/ty2025/SPEC.md` §8a **B3**, which sized it
as "its own spec-sized feature, not a section of one": 38 lines, six parts, four phase-outs, a filed
VIN, and ~25 collected inputs across five crates.

Passes an independent review loop to 0 Critical / 0 Important before an implementation plan.
Parent decisions (D-1 … D-11) in `design/ty2025/SPEC.md` bind here unless restated.

---

## 1. Sourcing of record — READ THIS FIRST

★ **`design/full-return/recon/fable/` already transcribes Schedule 1-A in full**, written 2026-07-11
against the enacted Pub. L. 119-21 and the TY2025 finals. Use it; do not re-derive. The parent spec
learned that lesson expensively (§2a): the SALT rule was re-derived from scratch and got the MFS shape
wrong **twice** when `01`'s line 13 already named that exact error.

| file | what it holds for Schedule 1-A |
|---|---|
| `03-followon-math-sch1a-qbi-ctc.md` §1.0–1.7 | per-part line formulas, statutory cites, input-side definitions, and **four worked examples** |
| `01-ty2025-finals-obbba.md` §190–195 | the six-part table with caps, thresholds and rounding directions |

**Authority** (archived, hashed, in `design/amt-form6251/`): `f1040s1a--2025.pdf` (`64f97b38`) and
`i1040gi--2025.pdf` (`482e9c48`), which carries the Schedule 1-A instructions. The recon is our own
notes — **re-verify each figure against those finals at write time**.

---

## 2. Binding decisions

**S-1. THE PHASE-OUT ROUNDING DIRECTION IS A PARAMETER, NEVER A SHARED CONSTANT.** The single most
dangerous fact in this form, and it is statutory rather than an IRS quirk:

| part | rule | direction | statute |
|---|---|---|---|
| II — tips | $100 per $1,000 over MAGI | **floor** (line 11: "decrease the result to the next **lower** whole number") | §224(b)(2) — "for each $1,000" |
| III — overtime | $100 per $1,000 | **floor** (line 19, same wording) | §225(b)(2) — "for each $1,000" |
| IV — car loan | $200 per $1,000 | **CEIL** (line 28: "**increase** the result to the next **higher** whole number") | §163(h)(4)(B)(iii) — "for each $1,000 **or portion thereof**" |
| V — seniors | 6% of the excess | **smooth**, no stair-step at all | §151(d)(5)(C) |

★ Schedule 8812 line 10 also **ceils**, for the same "or fraction thereof" reason (§24). So a
`phase_out(excess, per_step, step)` helper with a baked-in direction is **silently wrong on one side**.
The direction is an explicit argument, and **each part carries its own test at its own knee**. The
recon's worked examples (b) and (c) are exactly the $100 and $200 errors a shared helper produces.

**S-2. ONE MAGI, SHARED.** Part I line 3 is `1040 L11b + §933 PR + Form 2555 L45 + L50 + Form 4563 L15`
— the statutory definition is *identical* in §224(d), §225(d), §151(d)(5)(C) and §163(h)(4)(C), and the
**same** quantity drives §164(b)'s SALT phase-down (already built) and **Schedule 8812**. It is
computed once. `ReturnInputs::modified_agi` and the four `has_income_exclusion`-gated amounts already
exist (parent D-9); this spec consumes them and adds no new MAGI surface.

**S-3. THE TIPS AND OVERTIME CAPS ARE PER-RETURN, NOT PER-SPOUSE.** $25,000 tips "regardless of your
filing status" (§224(b)(1), and the form's line 7 prints no MFJ figure); $12,500 overtime, $25,000 MFJ
(§225(b)(1)). A combined per-return cap, so an MFJ couple with two tipped earners shares one $25,000 —
the natural per-person reading is wrong and would overstate the deduction.

**S-4. PART V's REDUCTION IS COMPUTED ONCE, THEN MULTIPLIED.** `L35 = max(0, 6,000 − 6% × L33)` is a
**per-person** amount reduced once; `L37 = L35 × (qualifying individuals)`. So an MFJ couple with two
seniors loses **12¢ per $1** of MAGI in the band. Everyone reaches $0 at `threshold + 100,000`.
★ §151(d)(5) stacks **on top of** the §63(f) aged-65 addition and, unlike §63(f), **survives itemizing**
— it is codified in §151, not §63(f).

**S-5. FILING-STATUS AND ELIGIBILITY BARS ARE PART OF THE TRANSCRIPTION** (parent D-7). Parts II, III
and V each print "If married, you must file jointly to claim this deduction" ⇒ **zero for MFS**. Part
IV carries no such caution ⇒ **allowed for MFS**, adjudicated against the form against OTS, which bars
all four. Parts II/III/V additionally require a **valid SSN** per person.

**S-6. EVERY NUMBERED LINE IS COLLECTED** (parent D-9), and each "see instructions" branch is
transcribed or refuses (parent §2's r1-8 note). The branches, all answered in `i1040gi--2025.pdf`:
line 4a's W-2-box-5-over-$176,100 case, 4c's multi-employer worksheet, line 5's multi-business
net-income limitation, line 22's ">two VINs ⇒ attach a statement", and 36a's valid-SSN condition.

**S-7. THE PER-YEAR TABLE COVERS 2025–2028 AND NOTHING IS INDEXED.** All four provisions expire after
TY2028. Caps and thresholds are fixed dollar amounts in the statute, so a "next year's Rev. Proc."
lookup is not merely unnecessary — it is wrong. **TY2029+ must fail closed**, like TY2026 does today.

---

## 3. Non-goals

- **Not** an optimizer change. The phase-out bands add hidden marginal-rate adders (each $1,000 of MAGI
  claws back $100 + $100 + $200 staired and 6%/12% smooth), so per-$1 what-ifs will show $0 then a
  cliff. ★ **Document that; do not "fix" it** — the step function is the law.
- **Not** Schedule 8812. It shares S-2's MAGI and S-1's ceil, and is its own work.
- **Not** determining whether an occupation is on the Treasury tipped-occupation list. The filer
  answers; we record the code.

---

## 4. Scope

### 4.1 Inputs (~25 leaves)
Part II: `L4a` W-2 box 7, `L4b` Form 4137, `L4c` multi-employer resolution, `L5` per-business
trade-or-business tips **plus** each business's net-profit ceiling. Part III: `L14a` W-2-side FLSA
**premium** portion, `L14b` 1099-NEC/MISC side. Part IV: per-vehicle **VIN**, total QPVLI, and
column (ii)'s portion already deducted on Schedule C/E/F. Part V: derived from `date_of_birth`
(already collected for §63(f)) **plus** the valid-SSN predicate per person.

★ **Two definitional traps in the input surface**, both from the recon: *qualified overtime* is only
the **premium half** of FLSA time-and-a-half (not double-time's second half, not holiday premiums);
and tips from an **SSTB** employer or SSTB self-employment are **not qualified** (§224(d)(3)). Both are
prompt wording, not arithmetic, and neither is visible on any W-2.

### 4.2 Computation
Six parts, transcribed line by line in the form's own numbering with the printed text as doc comments.
Order: `AGI → Part I MAGI → Parts II–V → L38 → then Form 8995 line 11` (which subtracts L13b) →
taxable income. Schedule 1-A never reads a deduction, so the DAG stays acyclic.

### 4.3 Emission
`L38 → 1040 L13b`; `L37 → Form 6251 line 1a` (parent D-3 — the *senior* subtotal, not the total).
File Schedule 1-A only when `L38 > 0`. Needs a new PDF and AcroForm map (parent §5.4), including the
VIN's per-character comb boxes — `recon/fable/05-ty2025-field-maps.md` has the extracted field names.

---

## 5. Test / green definition

1. **The five gates**, as the parent spec defines them.
2. **Each part's phase-out tested AT ITS OWN KNEE, in its own direction** (S-1). At minimum the recon's
   worked examples: (b) single, MAGI $157,350, tips $3,000 ⇒ **$2,300** (a ceil gives $2,200); (c)
   single, MAGI $104,050, QPVLI $6,000 ⇒ **$5,000** (a floor gives $5,200). ★ A test that passes under
   both rounding modes has not tested the rounding.
3. **Both oracles per part**, with disqualifications **computed and sized** — and OTS 2025's Part IV is
   already known defective three ways, while taxcalc has the wrong QSS threshold (parent D-8). So
   **QSS Part IV in the phase-out band ships zero-oracle** and the census must say so.
4. **All five filing statuses**, because S-3's per-return caps and S-5's MFS bar are status-dependent.
5. **A two-senior MFJ case** proving the 12¢-per-$1 aggregate slope (recon example (d): MAGI $200,000
   ⇒ L37 = $6,000, $3,000 each).
6. **`L38 > 0` gates filing**, and `L37` (not `L38`) reaches Form 6251 line 1a.
7. **TY2029 fails closed**, mutation-verified, like `ty2026_full_return_must_stay_fail_closed`.
8. **Mutation-verified guards**, and — the parent's hardest-won lesson — **a test whose mutation
   survives is not a test**. Two of this session's guards were vacuous until a mutation said so.

---

## 6. Risks

**R-1. The rounding asymmetry** (S-1). A shared helper is the natural implementation and it is wrong.

**R-2. Input definitions with no source document.** Qualified overtime's premium-half rule and the
SSTB exclusion are invisible on a W-2; the filer must be asked precisely. Wrong prompt wording is a
wrong return that every test passes.

**R-3. Part IV has the weakest oracle coverage of anything in TY2025** — three OTS defects plus
taxcalc's QSS threshold. Expect to lean on the form's own arithmetic and say so.

**R-4. The VIN is a new class of filed data** — a per-character comb-box string. Parent §8's OQ-3
recorded that the generic PII scanner does not cover VINs and why; the emitter's tests assert no
VIN-shaped literal in committed fixtures.

**R-5. Expiry.** Four provisions die after TY2028 (S-7). A table that quietly extends them files a
deduction that does not exist.

---

## 7. Open questions

1. **Does Part V's death rule reach us?** The 2026-02-27 errata: a person who died in 2025 *before*
   reaching 65 is not qualified even if born before 1/2/1961. btctax collects `date_of_birth`; does it
   collect a date of death?
2. **Per-business or aggregate for line 5's net-income limitation?** The recon reads it per trade or
   business, with a net-loss business contributing $0. Confirm against the instructions before coding.
3. **Notice 2025-69 transition relief** for 2025 tips reporting — does it change what we may accept as
   `L4a`, or only what employers must report?

---

## 8. Cross-references

- `design/ty2025/SPEC.md` — parent; D-1 … D-11, and §8a's branch plan.
- `design/full-return/recon/fable/03-…md` §1.0–1.7 — **the transcription of record**.
- `design/ty2025/reviews/` — the parent's r1/r2/r3.
