# REPORT — wave 1 A: FR-206, FR-205, FR-207

**Agent:** opus, own worktree (`.claude/worktrees/agent-aab543586c2888f7e`), CARGO_TARGET_DIR=<worktree>/target-a.
**Brief:** `design/agent-reports/BRIEF-wave1-A-box12-consequences.md` (commit 323cbeba4), read in full.
**Files touched — exactly the four owned, nothing else:**

```
 crates/btctax-core/src/tax/advisories.rs        | 171 +++++++
 crates/btctax-core/src/tax/return_refuse.rs     | 604 +++++++++++++++++++++---
 crates/btctax-forms/forms/2024/f1040s3.map.toml |   2 +-
 crates/btctax-forms/forms/2025/f1040s3.map.toml |   2 +-
 4 files changed, 715 insertions(+), 64 deletions(-)
```

**Gate (foreground, captured once):**

```
make check -> exit 0
     Summary [  23.277s] 3678 tests run: 3678 passed, 12 skipped
clippy: 0 warnings / 0 errors
cargo fmt --all -- --check -> exit 0
```

**Not committed, not pushed. No subagents.** All three items DONE. Three premises refined (§4) and one
required edit lies outside my ownership (§5).

---

## 1. FR-206 — the understatement. DONE.

### 1.1 The adjudication, and the authority I pinned the rule to

**The cap rule is the Form 1040's own line-1h paragraph, not §402(g)(3).** I went to §402(g)(3) first and it
does **not** support the rule as implemented: it enumerates only §401(k) (A), §408(k)(6) SEP (B), §403(b) (C)
and §408(p) SIMPLE (D) — it names neither §457(b) (code `G`, which the screen has always summed) nor
§501(c)(18) (code `H`). The instruction that actually governs the line btctax cannot compute does cover them,
by its own words, and it is what I cited in the source:

> • **Excess elective deferrals.** The amount deferred should be shown in box 12 of your Form W-2, and the
> "Retirement plan" box in box 13 should be checked. If the total amount you (or your spouse if filing
> jointly) deferred for 2025 **under all plans** was more than $23,500 (excluding catch-up contributions, as
> explained later), include the excess on line 1h. … **Although designated Roth contributions are subject to
> this limit, don't include the excess attributable to such contributions on line 1h. They are already
> included as income in box 1 of your Form W-2.**

`design/forms/extract/i1040gi--2025.txt:2346-2363`. The TY2024 revision says the same at
`i1040gi--2024.txt:2308-2325` with its own dollar figures. The W-2 instructions' worked example agrees and is
the shorter quotation:

> Even though the 2026 limit for elective deferrals and designated Roth contributions is $24,500, Alex's
> total elective deferral amount of $26,500 is reported in box 12 with code D

`iw2w3--2026.txt:2504-2507`.

**So the limit covers designated Roth contributions — the brief's premise is correct — and the set to sum is
`D E F G S` PLUS `AA BB EE`.** What the same paragraph also says, and what changes the *reason* rather than
the fix, is in §4.1 below.

### 1.2 The set is now DERIVED from the 33-row table

The five-code const `ELECTIVE_DEFERRAL_CODES` is **deleted**. Each `BOX12_CODES` row now carries a
`deferral_limit: DeferralLimit` field, where `DeferralLimit` is
`{ Outside, PretaxDeferral, DesignatedRoth }`.

Adding a row to the table is now an `E0063` until someone decides this question — form (2) of *"derive the
list, or make the compiler hold it"*. The screen reads `row.deferral_limit` off the row `box12_row` already
looked up, so there is no second set to fall out of step. `DeferralTotals::add` matches on `DeferralLimit`
without a wildcard arm, so a fourth variant is a compile error rather than an amount that silently stops
counting.

Measured off the source (33 rows, one script, no hand-count):

```
AlreadyInBox1               6  C V AA BB EE GG
DrivesAnUncomputedLine     15  A B K L M N P Q R T Z FF II TP TT
ForgoneDeductionAdvised     1  H
NoLineReadsIt               4  J Y DD HH
NotAdjudicated              1  TA
ReadByBtctax                6  D E F G S W
```

The two sides split PretaxDeferral = {D,E,F,G,S}, DesignatedRoth = {AA,BB,EE}, everything else Outside —
asserted in both directions by `the_capped_set_is_both_sides_of_the_limit`.

### 1.3 The refusal is strictly WIDER, never narrower

The comparison is `DeferralTotals::total()` (both sides) against `p.elective_deferral_limit`, per owner. The
sum can only grow, so the change cannot introduce an understatement anywhere.

The sides are kept apart because they carry a **reader**: the 1040 adds back the pre-tax-attributable excess
and excludes the Roth-attributable one, so what belongs on line 1h is determinate at one end, determinate at
the other, and unknowable in between. Three message branches, all three exercised by the test.

### 1.4 B1 — the plant, and the control. Both pasted.

**The plant.** $20,000 of code `D` plus $5,000 of code `AA` against a $23,000 limit. Pre-tax alone is
$20,000, **under** the limit — exactly the return the five-code list admitted in silence.

Revert `AA`'s `deferral_limit` to `Outside` and run the whole btctax-core suite:

```
FAIL ( 605/1385) btctax-core tax::return_refuse::tests::an_excess_reached_only_by_the_designated_roth_codes_still_refuses
FAIL ( 643/1385) btctax-core tax::return_refuse::tests::the_capped_set_is_both_sides_of_the_limit
 Summary [   0.589s] 1385 tests run: 1383 passed, 2 failed, 0 skipped

panicked at crates/btctax-core/src/tax/return_refuse.rs:7751:34:
★ THE KILL: $20,000 code D + $5,000 code AA is $25,000 against a $23,000 limit — the Form 1040 applies
the limit to designated Roth contributions too, so this return must not file

panicked at crates/btctax-core/src/tax/return_refuse.rs:7544:9:
assertion `left == right` failed: ★ THE FR-206 KILL: the designated Roth codes are subject to the SAME
limit — dropping them is what let an excess deferral through unscreened
  left: {"BB", "EE"}
 right: {"AA", "BB", "EE"}
```

★ **Note what did NOT red: the other 1383 tests.** The pre-FR-206 suite was entirely green with `AA` outside
the limit. That is the measurement of the blindness, not an aside.

Restored, and the real behaviour — the refusal a filer actually reads:

```
=== MIXED (D 20000 + AA 5000, limit 23000) -> REFUSED ===
one person's Form W-2 box 12 elective deferrals and designated Roth contributions total $25000, more than
the $23000 the Form 1040 instructions allow "under all plans" before the excess goes back into income on
line 1h. $5000 of that is designated Roth, which is already inside box 1, and the Form 1040 instructions
say "Although designated Roth contributions are subject to this limit, don't include the excess
attributable to such contributions on line 1h" — so how much of the $2000 excess is taxable depends on
which contributions the plan returns to you, which is designated rather than computed and is not on your
W-2. btctax does not compute line 1h. The plan also has to return the excess to you, and that distribution
arrives on a Form 1099-R. File this return with a preparer

=== THE CONTROL: UNDER (D 20000 + AA 2000 = $22000) -> FILES ===
None

=== PURE PRE-TAX (D 25000) -> REFUSED, and the excess is DETERMINATE ===
… all of that is pre-tax, so the whole $2000 excess belongs on Form 1040 line 1h. …

=== PURE ROTH (AA 25000) -> REFUSED, and NOTHING goes on line 1h ===
… all of that is designated Roth, which is already inside box 1 — and the Form 1040 instructions say
"don't include the excess attributable to such contributions on line 1h", so NOTHING of the $2000 excess
goes back into income on this return. …
```

### 1.5 Three pre-existing over-refusals, now STATED in the source rather than implied

All three make the screen refuse a return the law permits. All three are the safe direction, and none is
new; none was written down before.

1. **Catch-up contributions are not separable from the box 12 figure.** *"report the elective deferrals and
   the elective deferral 'catch-up' contributions as a single sum in box 12 using the appropriate code"*
   (`iw2w3--2026.txt:2449-2456`), while the 1040's test is *"excluding catch-up contributions"*. A lawful
   50-or-over filer can trip this screen.
2. **Code `G` carries employer money** — *"when using code G for section 457(b) plans, include both elective
   and nonelective deferrals"* (`iw2w3--2026.txt:2447-2448`) — and a nonelective contribution is not an
   elective deferral.
3. **`F` and `S` are mixed codes**: their own IRS labels include *"elective deferrals made to a Roth SEP
   IRA"* and *"salary reduction contributions made to a Roth SIMPLE IRA"*, so one box 12 figure can hold both
   sides of the limit. Recorded as PretaxDeferral; the only consequence is that the message may call an
   excess determinate when part of it is not. The refusal itself is unaffected.

---

## 2. FR-205 — code H FILES with an advisory. DONE.

### 2.1 A fourth ADMITTED verdict, not a widened existing one

None of the three admitting verdicts was true of code H: its amount **is** in box 1 (so not
`NoLineReadsIt`), and reading box 12 would **not** change nothing (so `AlreadyInBox1`'s stated proof is false
of it). So a fourth arrives:

    /// **ADMITTED, WITH AN ADVISORY (FR-205).**
    ForgoneDeductionAdvised { line: &'static str, advisory: &'static str },

The `Box12Verdict` doc now says *"Four ways to be admitted, two to be refused"* and states the boundary
explicitly: **this variant is available only where forgoing the amount moves the tax UP.** It is not
available to an uncollected tax (`A`,`B`,`M`,`N`), an excise tax (`K`,`Z`) or omitted income (`T`). The
wildcard-free match in `refusal()` is updated (a *seventh* verdict is now the compile error).

### 2.2 The advisory, built like its two siblings

`Advisory::Section501c18DeductionForgone { ceiling: Usd }` — fires only on a box 12 code-`H` entry with money
on it, summed across W-2s, read **trimmed and upper-cased exactly as the screen reads it** so a lower-case
`h` cannot be admitted by the screen and skipped by the advisory. Message:

```
§501(c)(18)(D) DEDUCTION NOT COMPUTED — your Form W-2 reports $2,400 in box 12 with code H. Your employer
had to "include this amount in box 1 as wages", and box 1 is what this return files on Form 1040 line 1a —
but the instruction for that code goes on: "The employee will deduct the amount on their Form 1040 or
1040-SR." That deduction is Schedule 1 line 24f ("Enter contributions to section 501(c)(18)(D) pension
plans"), which v1 does not compute, so the wages went on and the deduction did not come off: your tax is
OVERSTATED by up to $2,400. The figure is a CEILING, not the deduction — line 24f sends you to Pub. 525 for
the limit. Enter the deductible amount on Schedule 1 line 24f by hand, or file with a preparer.
```

*"up to"*, and the sentence *"The figure is a CEILING, not the deduction"* — the
`MixedUseMortgageNotAllocated` discipline. The ceiling language is not decoration: line 24f's own instruction
is *"Enter contributions to section 501(c)(18)(D) pension plans (see Pub. 525)"*
(`i1040gi--2025.txt:43215-43217`), and Pub. 525 carries a limit btctax does not model, so the deductible
amount can be smaller than what the employer printed.

### 2.3 The pairing is machine-checked, and the kills were watched RED

The `advisory` payload on `ForgoneDeductionAdvised` is held to the real type by
`a_verdict_that_promises_an_advisory_names_one_that_exists`: it asserts the promised list exactly, constructs
the named variant (so a missing variant will not compile), compares the payload string against the derived
`Debug` name (so a rename reds), and asserts the advisory's own message names the promised line. A
covered_by-shaped string nothing checks is how a census entry goes stale.

Two plants, run against the whole btctax-core suite:

```
PLANT A — code H's verdict reverted to DrivesAnUncomputedLine:
  FAIL ( 619/1387) return_refuse::tests::an_inert_box12_code_files_and_a_consequential_one_refuses_by_name
  FAIL ( 623/1387) return_refuse::tests::a_verdict_that_promises_an_advisory_names_one_that_exists
   Summary 1387 tests run: 1385 passed, 2 failed

PLANT B — the advisory's emission gated off:
  FAIL ( 121/1387) advisories::tests::the_501c18_advisory_fires_on_a_code_h_entry_and_names_it_as_a_ceiling
   Summary 1387 tests run: 1386 passed, 1 failed
```

Both restored; suite back to 1387 passed, 0 skipped.

### 2.4 ★ The class survey the brief asked for — NOTHING ELSE CONVERTED

**Sixteen codes still refuse: `A B K L M N P Q R T Z FF II TP TT TA`** (derived from the source, not typed).
Classifying each by the brief's own test — *a forgone taxpayer-favourable amount, or a figure we would get
WRONG?*

**Same class as `H` — these over-refuse. I have NOT converted them.**

| code | why it is the same class |
|---|---|
| `L` | The amount is the *nontaxable substantiated* part, outside box 1; the excess is already in box 1 as wages. Nothing on the return would be wrong. What is forgone is the Form 2106 to Schedule 1 line 12 deduction for the four narrow classes still allowed it post-TCJA, whose expenses btctax never collects. ★ A reimbursement is not an expense, so the code L figure could not even name a ceiling. |
| `P` | Excluded from box 1. What is forgone is the Form 3903 to Schedule 1 line 14 moving-expense deduction, whose expenses btctax never collects. Same shape as `L`. |
| `Q` | Not in box 1; it is an **election** into earned income for the EIC. btctax computes no EIC — and `Advisory::EicOmitted` (*"Your tax may be OVERSTATED. Check Pub. 596"*) already exists for precisely that omission. |
| `TP`, `TT` | Refuse because the Schedule 1-A Part II/III deduction would go unclaimed — a forgone taxpayer-favourable deduction, by the refusal's own words. ★★ **And the inconsistency is sharp:** box **14b** is the *same* evidence about the *same* deduction on the *same* W-2, and it **advises** (`Advisory::TipsDeductionForgoneWithTtoc`) rather than refusing. One fact, two boxes, two opposite verdicts. |
| `II` | Both halves point the same way. The EIC-election half is forgone-favourable like `Q`. The Schedule 1 line **8s** half is the `H` shape exactly: a *subtraction* btctax does not compute, so omitting it can only overstate. ★ A ceiling here needs care — the 1040's own Caution says box 1 *"is blank or has zeros"* in the common case, so how much (if any) is in box 1 is not knowable from the code. |

**NOT the same class — these refuse correctly, because admitting one would make a figure WRONG.**

- `A`, `B`, `M`, `N` — uncollected social security / Medicare tax to Schedule 2 line 13. An additional **tax**: admitting UNDERSTATES.
- `K` — 20% golden-parachute excise tax to Schedule 2 line 17k. Understates.
- `Z` — §409A failure: in box 1 **and** carrying a 20% additional tax plus interest, Schedule 2 line 17h. Understates.
- `T` — adoption benefits: *"Report all amounts including those in excess of the … exclusion"*, and Form 8839 line 31 carries the taxable remainder to Form 1040 line 1f. Omitted **income**: understates.
- `R` — an employer Archer MSA contribution counts against the filer's own limit, so an excess is taxable plus a 6% excise. Understates. (btctax also refuses Archer MSA activity outright by a separate rule.)
- `TA` — NotAdjudicated, and honestly so: a §128 Trump account contribution whose exclusion limit, over-limit treatment and Form 4547 interaction nobody has adjudicated. This is the fail-closed default and should stay until someone does the work.

**`FF` is a third thing, and worth a separate line.** Its stated reason is that a QSEHRA disqualifies the
self-employed health-insurance deduction and bears on the premium tax credit. But Schedule 1 line 17 is
censused `unmodeled` (`crates/btctax-forms/forms/2024/f1040s1.map.toml:157`) and btctax computes no PTC — so
admitting `FF` would change no figure btctax prints. It is not the forgone-benefit class; on today's model it
is **inert**. I did not lift it, because that judgement rests on a fact about what btctax models rather than
about the code, and the refusal is the safe resting place. Worth a follow-up to state the boundary.

---

## 3. FR-207 — the invalidated justification. DONE (reason restated, verdict unchanged).

`f1040s3.map.toml:96` (2024) and `:156` (2025) said Form 8880 was unmodeled because *"no retirement
contributions collected"*. **That is false and the tree says so:** the Schedule 3 line 4 instruction's
qualifying limb (b) is

> elective deferrals to a 401(k) or 403(b) plan (including designated Roth contributions) or to a
> governmental section 457(b) plan, SIMPLE IRA, or a SEP

(`i1040gi--2024.txt:43158-43170`, `i1040gi--2025.txt:45308-45320`) — which is exactly box 12 codes
`D/E/F/G/S` and `AA/BB/EE`, every one collected and admitted since FR-197; and limb (d), *"contributions to a
501(c)(18)(D) plan"*, is code `H`, admitted since FR-205.

**The verdict stands and the reason is now the eligibility test.** The instruction's disqualifier 2 is

> The person(s) who made the qualified contribution or elective deferral (a) was born after January 1, 2007,
> (b) is claimed as a dependent on someone else's 2024 tax return, or (c) was a student (defined next).

(`i1040gi--2024.txt:43177-43180`; 2025 at `:45327-45330` with 2008.) btctax collects (b) for both spouses and
can compute disqualifier 1 from AGI — but **(a) rests on a date of birth that is optional** (the
`AgedBoxForfeitedNoDob` advisory exists precisely because it is often absent) and **(c) is never collected at
all for the filer or the spouse**: `full_time_student` is a field of `Dependent`, not of `Person`
(`return_inputs.rs:781`; `Person` is lines 706-750, `Dependent` 751-875). A credit computed past an untested
disqualifier would be a figure the return swears to and cannot support; a forgone credit can only OVERSTATE.
So the line stays blank with `Advisory::OtherCreditsOmitted`, now for a reason the tree does not contradict.

Both years carry the restated reason, each citing its own revision. The stale cross-reference in
`return_refuse.rs`'s `AA`/`BB`/`EE` comment — which pointed at the old "no retirement contributions
collected" wording — is updated in the same pass.

---

## 4. Premises refined (none of the three items is refuted; one reason is)

### 4.1 ★ FR-206: the excess is NOT simply "income that should have been added back"

The brief says the undetected excess *"is income which should have been added back and was not."* The same
line-1h paragraph that establishes the cap **explicitly excludes the Roth-attributable excess from the
add-back**: *"Although designated Roth contributions are subject to this limit, don't include the excess
attributable to such contributions on line 1h. They are already included as income in box 1 of your Form
W-2."*

Consequences, and they matter for the message rather than the fix:

- A household with D $0 / AA $25,000 against a $23,000 limit breaches the limit and puts **nothing** on
  line 1h. That return was *not* understated. It is still refused (fail-closed, and the plan still owes a
  corrective distribution), and the message now says so instead of claiming the amount is unknowable.
- A household with pre-tax **and** Roth is where the understatement lives — and the size is **not
  computable**, because which contributions the plan returns is *designated* rather than derived (§402(g)(2)
  / Reg §1.402(g)-1(e)). That is the real justification for refusing rather than adding back, and it is what
  the source and the message now say.

**The fix the brief asked for is unchanged and correct**: the cap must count both sides. Only the stated
reason moves from *"the excess is income"* to *"part of the excess may be income and the split is the
filer's/plan's designation."*

### 4.2 §402(g)(3) does not support the rule as implemented

§402(g)(3) enumerates §401(k), §408(k)(6), §403(b) and §408(p) — not §457(b) (code `G`) and not §501(c)(18)
(code `H`). Code `G`'s long-standing membership in the sum is therefore justified by the 1040's *"under all
plans"* line-1h test, not by §402(g)(3), and that is the authority the source now cites. `H` is recorded
Outside the sum on the same reading (the paragraph never names §501(c)(18), and the deduction for it has a
limit of its own on line 24f); this is asserted by `the_capped_set_is_both_sides_of_the_limit`.

### 4.3 Nothing else in the brief was refuted

FR-205's premise (code `H`: box 1 wages, forgone Schedule 1 line 24f deduction, currently refusing) and
FR-207's premise (the Form 8880 reason is falsified by admitted code `D`) both checked out verbatim against
the extracts.

---

## 5. ★ ONE REQUIRED EDIT IS OUTSIDE MY OWNERSHIP — reporting, not editing

The brief asks that the new advisory get **a census entry**. The only census row for Schedule 1 line 24f is

```
crates/btctax-forms/forms/2024/f1040s1.map.toml:173
  { line = "24f", rule = "unmodeled",
    reason = "Contributions to §501(c)(18)(D) pension plans — not modelled.",
    covered_by = "Advisory::UnmodeledDeductionsOmitted" }
```

That file is **not in my ownership list**, so I did not touch it. There is no 2025 `f1040s1.map.toml` — 2024
is the only bundled map carrying this line (verified by a recursive search for 501(c)(18) across
`crates/btctax-forms/forms/`).

**Nothing is broken by leaving it:** `Advisory::UnmodeledDeductionsOmitted` exists, line 24f grades
Overstates, and the census join accepts an advisory cover there — the gate is green. The **improvement** is to
repoint it at `Advisory::Section501c18DeductionForgone`, which names the amount and the code instead of
covering the line with a blanket note. Recommended follow-up for whoever owns that file.

---

## 6. Process notes

- **FR-175 hazard hit once, and handled in-turn.** My first gate run exceeded the Bash tool's default 120s
  timeout and was moved to the background. I did **not** end the turn on it: I stayed in-session, re-ran the
  gate in the foreground with an explicit 600s timeout, and every gate figure quoted above comes from a
  foreground run. ★ The backgrounded run also reported exit code 0 while its log said FAILED — because the
  exit status belonged to the `tee` at the end of the pipe. A pipeline is not a gate.
- **My own defect, caught by the gate.** My first FR-207 write put raw double quotes inside a TOML basic
  string, producing invalid TOML: **33 tests red across btctax-forms, xtask, cite_check, census_join,
  line_coverage_check and the examples golden — all one root cause.** Fixed, and I added a `tomllib` parse
  check over every bundled `*.map.toml` so the next such slip is one command rather than a 200-second suite.
- **Every citation machine-checked, not reviewed.** `cite_check` scans only `design/ty*/` markdown — it reads
  neither `.rs` doc comments nor map `reason` fields, so none of the 15 new quotations was covered by any
  committed gate. I re-implemented `cite_check::normalise`'s exact policy (typographic folding,
  de-hyphenation across a break, then non-alphanumerics to spaces, digits and dollar amounts preserved) and
  checked all 15 against their cited extract **line spans**: FAILURES: 0. ★ Two of my line citations were
  wrong on the first pass (`i1040gi--2024.txt:43107-43124` should be `:2308-2325`; the Alex quote spans
  `:2504-2507`, not `:2506`), and the checker is what found them. ★★ A trap for the next person: the extracts
  contain form feeds, so Python `splitlines()` disagrees with sed/grep line numbers — split on newline only.
- **Counts and sets are derived.** The 33-row verdict census, the 16-code refused list and the two sides of
  the deferral limit in this report all come from scripts reading the source, never from a hand-count.
