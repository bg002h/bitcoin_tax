# REPORT — wave 2, parcel F: FR-216, FR-217, FR-215

**Agent:** parcel F. **Worktree:** `agent-ade799f0d017c0b4a`, `CARGO_TARGET_DIR=<worktree>/target-f`.
**Files touched — all three inside the parcel's ownership, nothing else:**

```
 crates/btctax-core/src/tax/advisories.rs        | 448 ++++++++++++++-
 crates/btctax-core/src/tax/return_refuse.rs     | 730 ++++++++++++++++++++----
 crates/btctax-forms/forms/2024/f1040s1.map.toml |   2 +-
 3 files changed, 1042 insertions(+), 138 deletions(-)
```

`f1040s3.map.toml` is in the parcel's ownership and was **not** touched — FR-207 already restated both of its
Form 8880 rows and nothing in FR-215/216/217 reaches them. Only `forms/2024/` carries an `f1040s1.map.toml`,
so FR-215 is a one-line edit and not a per-year sweep.

**Gate (`make gate`, which touches every `.rs` first — FR-90):**

```
GATE_EXIT=0
     Summary [  27.646s] 3696 tests run: 3696 passed, 12 skipped
$ cargo fmt --all --check
FMT_EXIT=0
```

`make check` = nextest + clippy `-D warnings` concurrently; both clean. `cargo fmt --all --check` run
separately, as the brief requires.

---

## 0. ★★ TWO PREMISES DISPROVED, one of them load-bearing

The brief's standing instruction is to stop and report a premise I can disprove. Two here, and the first one
**changed a verdict I would otherwise have got wrong**.

### 0.1 ★★★ `II` is NOT "the `H` shape exactly". The code is defined as the amount that is NOT in box 1.

`REPORT-wave1-A.md:238` classified code `II` as *"the `H` shape exactly: a subtraction btctax does not
compute, so omitting it can only overstate"*, and FR-216 carried that reading forward. The W-2 instruction for
the code says the opposite in its own first sentence:

> **Code II—Medicaid waiver payments excluded from gross income under Notice 2014-7.** Report the amount of
> Medicaid waiver payments **not reported in box 1**.
> — `design/forms/extract/iw2w3--2026.txt:2793-2796`

And Schedule 1 line 8s subtracts only

> Nontaxable amount of Medicaid waiver payments **included on Form 1040, line 1a or 1d**.
> — `design/forms/extract/i1040gi--2025.txt:42322-42323`

So the code `II` amount is by construction **outside** what line 8s can back out. The 1040's actual mechanism
is an election: *"If you did not receive a Form W-2 for nontaxable payments, or you received nontaxable
payments that you didn't report on line 1a, **and choose to include nontaxable amounts in earned income for
purposes of claiming a credit or other tax benefit**, report the amount on … line 1d. Then, on line 8s, enter
the total amount … (as a negative number)"* (`i1040gi--2025.txt:42331-42346`). **Line 1d plus line 8s is
exactly income-neutral by construction** — +X then −X — and exists only to make the money count as *earned
income* for the EIC and the ACTC.

Consequences:
1. Code `II` is **inert for every income line**, not an overstatement. It is therefore
   `Box12Verdict::NoLineReadsIt`, not `ForgoneDeductionAdvised` — the amount could never have named a ceiling
   on a deduction, because there is no deduction.
2. **The refusal I removed was justified by a false sentence.** It said code `II` *"is a Medicaid waiver
   payment the 1040 instructions send to Schedule 1 line 8s"*. They do not; they send an amount the employer
   put in **box 1** there, which this code is defined not to be.

### 0.2 The tips advisory asserted something its own evidence can contradict (`000`)

Pre-existing, found while adjudicating the pair. `Advisory::TipsDeductionForgoneWithTtoc` told every filer
that their box 14b code *"is your employer's statement that your tips were earned in a listed occupation."*
Box 14b's instruction:

> If any tips were received in a **nonqualifying** occupation, then **"000" must be input** as one of the
> occupation code(s). — `iw2w3--2026.txt:2935-2945`

So on a W-2 whose only code is `000` the advisory asserted the exact opposite of what the paper says, in the
direction of encouraging a §224 deduction the form denies. Fixed (§1.4); it still prints — a notice saying
*"your paper says this may not qualify"* forgoes nothing — it just no longer claims the employer vouched.

---

## 1. FR-216 — the self-contradiction first, then the class

### 1.1 The contradiction is worse than filed, in three ways

**Filed:** box 14b advises while box 12 code `TP` refuses over the same fact on the same W-2. All true. Three
things the entry did not have:

**(a) The advisory was UNREACHABLE, not merely inconsistent.** The two boxes are printed together **by rule** —
each instruction requires the other:

> **Code TP** … Report the total amount of cash tips reported to the employer. … **You must also list an
> occupation code in box 14b**—Treasury Tipped Occupation Code(s). — `iw2w3--2026.txt:2810-2819`
>
> **Box 14b—Treasury Tipped Occupation Code(s).** Use this box to report the Treasury Tipped Occupation
> Code(s) **if cash tips are reported in box 12 with code TP**. — `iw2w3--2026.txt:2935-2937`

So on a **conforming** 2026 W-2 both boxes are present, `screen_inputs` runs before any advisory is collected,
and the box 12 refusal always won. `Advisory::TipsDeductionForgoneWithTtoc` — built by FR-65, tested, shipped —
**could never print on the document it was built for.** It could fire only on a W-2 transcribed with box 14b
and *without* code TP.

**(b) The refusal's own exit did not work.** It said *"Complete Schedule 1-A Part II if those tips qualify, and
file with a preparer"* — and completing Part II changed nothing, because the rule fired on the code's presence
and never looked at Part II. The table's own doctrine is *"a refusal with no exit is just a brick with better
prose"*; this one had the prose. Same for `TT` and Part III.

**(c) The stated reason does not distinguish this year from any other.** The refusal's ground was that btctax
*"does not reconcile the two figures."* Neither does it for TY2025, where a tipped filer's Part II is their own
unreconciled statement and **files today**. Admitting `TP` adds no unchecked figure that was not already
unchecked; it adds a notice where there was silence.

### 1.2 The adjudication: the pair resolves in favour of the ADVISORY, and the class test splits the rest in two

The brief's test — *does forgoing the amount move the tax **up**, or produce a figure we'd get **wrong***? —
admits all six candidates. But `Box12Verdict::ForgoneDeductionAdvised`'s own documented contract is narrower,
and it is that contract which decides *which* admit-verdict each code gets:

> The amount is inside box 1 … and a line btctax does not compute would take it back out.

| code | in box 1? | what would read it | verdict now |
|---|---|---|---|
| `TP` | **yes** (tips are wages) | Schedule 1-A Part II §224 — a Part btctax **models and computes** | `ForgoneDeductionAdvised` → `Advisory::TipsDeductionForgoneWithTtoc` |
| `TT` | **yes** (overtime is wages) | Schedule 1-A Part III §225 — likewise modelled | `ForgoneDeductionAdvised` → `Advisory::OvertimeDeductionForgone` (**new**) |
| `L` | no — *"Report in box 12 only the amount treated as substantiated"* | Sch 1 line 12 / Form 2106 — **unmodelled**, and its *expenses* are never collected | `NoLineReadsIt` |
| `P` | no — an **excludable** reimbursement | Sch 1 line 14 / Form 3903 — unmodelled, expenses never collected | `NoLineReadsIt` |
| `Q` | no — §112 excluded (the taxable excess is already in box 1) | the EIC/ACTC earned-income election, 1040 line 1i — **no credit computed** | `NoLineReadsIt` |
| `II` | **no**, by the code's own definition (§0.1) | the same election, 1040 line 1d + Sch 1 line 8s, income-neutral | `NoLineReadsIt` |

**The line between the two families is the one the file already draws for Form 8880**: *"That boundary is the
missing FORM, not this code."* For `TP`/`TT` the forgone deduction is one btctax **can** claim, and only the
filer's statement of the qualified subset is missing — code `H`'s shape, one schedule over. For
`L`/`P`/`Q`/`II` the amount is outside box 1, reaches **no line btctax computes**, and *could not name a
ceiling on anything*: a reimbursement is not an expense, and an electable earned-income amount is not a
deduction. Their forgone benefit belongs to the missing form or credit, each already censused and advised
(`f1040s1.map.toml` lines 12 and 14 → `Advisory::UnmodeledDeductionsOmitted`, **unconditional**; 1040 line 27 →
`Advisory::EicOmitted`).

★ **And refusing them closed nothing.** Every one of those four benefits is reached from its own 1040 /
Schedule 1 line without passing through box 12 at all — a reservist's Form 2106 expenses, an Armed Forces move,
combat pay on line 1i, waiver payments on line 1d. A filer with the same facts and no box 12 code was admitted,
with the same benefit forgone, before FR-216.

★★ **Verified: admitting these six computes no new figure.** The only readers of `w2.box12` in the tree are
`form8889.rs` (code `W`), the deferral cap in `return_refuse.rs`, `transcription_warnings.rs` (codes `A`/`B`
only), and `advisories.rs` — measured by `grep -rn '\.box12' crates/*/src`. Box 1 goes to Form 1040 line 1a
either way, so what changed is the advisory list, not a dollar.

### 1.3 ★ The codes that are NOT this class, said plainly

Ten codes still refuse. **Measured from the source, not hand-counted** (parser over `BOX12_CODES`):

```
rows parsed: 33
ADMIT  23: C D E F G H J L P Q S V W Y AA BB DD EE GG HH II TP TT
REFUSE 10: A B K M N R T Z FF TA
  AlreadyInBox1            6  C V AA BB EE GG
  ReadByBtctax             6  D E F G S W
  NoLineReadsIt            8  J L P Q Y DD HH II
  ForgoneDeductionAdvised  3  H TP TT
  DrivesAnUncomputedLine   9  A B K M N R T Z FF
  NotAdjudicated           1  TA
```

(wave1-A measured 16 refusing: `A B K L M N P Q R T Z FF II TP TT TA`. Six converted, as above.)

- **`A`, `B`, `M`, `N`** — uncollected social security / Medicare tax → Schedule 2 line 13. An additional
  **tax**. Admitting UNDERSTATES.
- **`K`** — 20% golden-parachute excise tax → Schedule 2 line 17k. Understates.
- **`Z`** — §409A failure: in box 1 **and** carrying a 20% additional tax plus interest → Schedule 2 line 17h.
  Understates.
- **`T`** — adoption benefits: *"Report all amounts including those in excess of the … exclusion"*, and Form
  8839 line 31 carries the taxable remainder to Form 1040 line 1f. Omitted **income**. Understates.
- **`R`** — an employer Archer MSA contribution counts against the filer's own §220 limit, so an excess is
  taxable plus a 6% excise. Understates. (btctax also refuses Archer MSA activity by a separate rule.)
- **`TA`** — `NotAdjudicated`, and honestly so: a §128 Trump account contribution whose exclusion limit,
  over-limit treatment and Form 4547 interaction nobody has worked. The fail-closed default stays until someone
  does.
- **`FF`** — a third thing, and I did **not** lift it, endorsing wave1-A's reasoning. Its stated ground (a
  QSEHRA disqualifies the self-employed health-insurance deduction and bears on the PTC) rests on
  `f1040s1.map.toml:157` line 17 being unmodelled and btctax computing no PTC — i.e. on facts about **what
  btctax models**, not about the code. On today's model it is inert; the refusal is the safe resting place, and
  the boundary deserves a follow-up rather than a verdict flip in this pass.

### 1.4 What was built

`advisories.rs`
- `TipsDeductionForgoneWithTtoc` gains `cash_tips_reported: Usd`; the trigger becomes
  `schedule_1a_exists && !claims_tips && (!ttoc_codes.is_empty() || box12_code_tp > 0)`. The message has three
  evidence branches (both boxes / code TP alone / box 14b alone) plus the `000` branch, and it names the code-TP
  figure as a **CEILING** — the 2026 Schedule 1-A wants only the *qualified* part of it, then caps at $25,000
  and phases out on MAGI.
  ★ The variant **keeps its name** even though the trigger outgrew it, stated in its own doc as the honest
  boundary: renaming needs `return_inputs.rs:102` (an intra-doc link — parcel **E**'s file) and
  `xtask/src/box_census.rs:721` (a census note — parcel **G**'s). Rename filed as a follow-up.
- **New** `Advisory::OvertimeDeductionForgone { ceiling }` — the §225 mirror, one Part down.
- ★★ **Both Schedule 1-A advisories are now YEAR-GATED** on `tables::schedule_1a_params(ri.tax_year)`, derived
  from `SCHEDULE_1A_YEARS` rather than a second copy of `2025..=2028`. Without it, a hand-typed box 14b code or
  code `TP` on a TY2024 return made the note name a Part of a schedule that does not exist —
  `Schedule1aNotOnThisYearsReturn` cannot catch that, because it needs Part II to *carry data* and these
  advisories fire only when it is empty. This was a **pre-existing** hole on the box-14b path.
- ★★ **`Advisory::EicOmitted`'s trigger now counts codes `Q` and `II` as electable earned income.** Required,
  not optional: `earned_income` is wages + net SE earnings, both amounts are outside box 1 and outside AGI, so a
  caregiver whose entire wage is code `II` has `earned_income == 0` and — once FR-216 admits the return — would
  have filed with a forgone **refundable** credit and **no advisory at all**. §3.4 permits a conservative
  omission *only if the filer is told*; this is what makes the telling structural rather than incidental.
- One `box12_total(code)` closure replaces the same `flat_map` typed once per code.

`return_refuse.rs`
- The six verdicts above, each carrying the primary-source quotation that decides it.
- Table note 4 rewritten. It predicted *"The first one admitted gives it [a year axis]"* — and FR-216 admits two
  of the three 2026 codes, so **the year axis has arrived and is not implemented**. Stated rather than closed,
  because closing it well means deriving each code's first revision from the extracts
  `the_box12_table_is_the_irs_table` already parses; a typed year beside a derived set is the shape `CLAUDE.md`
  forbids. No figure and no sentence is wrong in the interim, because the advisories are year-gated.
- ★★★ **An FR-99-shape defect the change exposed, now derived.** The param-free census fixture for
  `RefuseReason::UnsupportedBox12Code` was `code: "Q"` — a refusing code on the day it was written and an
  admitted one as of FR-216 — so three census gates went red and the census row silently stopped being
  reachable. The fixture now asks `BOX12_CODES` for a code whose verdict refuses, and `expect`s loudly if none
  does.

---

## 2. FR-217 — adjudicated against the statute. **Behaviour stays fail-closed, and here is the reason.**

### 2.1 The limbs

| limb | codes | its own limit |
|---|---|---|
| **§402(g)** elective deferrals | `D` §401(k), `E` §403(b), `F` §408(k)(6) SEP, `S` §408(p) SIMPLE; Roth side `AA` §401(k), `BB` §403(b) | §402(g)(1)(B) |
| **§457(b)** deferred compensation | `G` §457(b); Roth side `EE` — *"Designated Roth contributions under a **governmental section 457(b)** plan"* | §457(b)(2) / §457(e)(15) |

★ **A finding the brief did not have: `EE` is §457(b) money too.** FR-217 asks about code `G`. Code `EE`'s own
label names a governmental §457(b) plan, so the identical question applies to it, and it was likewise being
summed into a §402(g)-shaped cap. Both are now recorded in the §457 limb.

The brief's premise holds: §402(g)(3) enumerates §401(k), §408(k)(6), §403(b) and §408(p) and names neither
§457(b) nor §501(c)(18); §457(b)(2) sets its own ceiling by reference to §457(e)(15). On that reading a filer
with a 401(k) **and** a governmental 457(b) may defer the full amount to **each**, and one summed comparison
refuses a return that is inside both limits.

### 2.2 It still sums them — deliberately, and the source now says so in a paragraph rather than by omission

Three reasons, and **none of them is that the statute is unclear**:

1. **No primary statutory text is archived in this repo.** `design/forms/extract/` holds forms and instructions;
   there is no 26 U.S.C. §402 or §457 anywhere in the tree (checked). `CLAUDE.md`'s authority hierarchy cuts
   **both** ways here: it forbids pinning the *limit* to the 1040's *"under all plans"* paragraph, and it equally
   forbids pinning it to an unarchived recollection of the Code. The instruction that *is* archived, read
   literally, supports the single sum — and it mentions §457(b) only to say *"A higher limit may also apply to
   participants in section 457(b) deferred compensation plans for the 3 years before retirement age"*
   (`i1040gi--2025.txt:2369-2374`), which reads as a variant of *this* limit rather than a separate one.
2. **The two limbs carry the same adjusted dollar figure** in every year btctax bundles, so splitting the
   comparison changes the outcome only for the filer over the combined figure and under both single ones.
3. **The error directions are not symmetric.** Summing over-refuses a lawful return, with a working exit
   (*"File this return with a preparer"*). Splitting risks the opposite — a genuine excess admitted with Form
   1040 line 1h blank, an **understatement**, the one direction §3.4 never permits silently. And the population a
   split would rescue is one btctax already cannot serve: §457(b)'s last-three-years catch-up is not separable
   from the box 12 figure, exactly like the age-50 catch-up boundary already recorded above the table.

**So: the answer is not "unclear", it is "clear on the statute and unprovable from what this repo archives",
and the resolution is conservative on purpose.** Said in the source, not implied.

### 2.3 ★ The limb is recorded structurally and **has a reader**

- `DeferralLimit` is now `{ Outside, Section402gPretax, Section402gDesignatedRoth, Section457bPretax,
  Section457bDesignatedRoth }`; `DeferralTotals` keeps four buckets, and `add` is `_`-free over all five, so a
  sixth is a compile error rather than an amount that silently stops being counted.
- The conservative fold now happens in **one visible place** (`DeferralTotals::total`) instead of being invisible
  in the labelling.
- **The refusal message names the limb when there is §457(b) money in it, and only then:** *"★ $40,000 of that
  total is section 457(b) money (box 12 code G or EE). A section 457(b) plan has a SEPARATE annual limit of its
  own (§457(b)(2)/§457(e)(15)) rather than sharing §402(g)'s, and a higher one still 'may also apply … for the 3
  years before retirement age' — so you may well be inside both of your plans' limits with nothing at all to add
  back."* Before FR-217 that filer was told only that they were over "the" limit, with a preparer as the exit and
  nothing for the preparer to look at first.

---

## 3. FR-215 — done

`crates/btctax-forms/forms/2024/f1040s1.map.toml:173` now reads
`covered_by = "Advisory::Section501c18DeductionForgone"`, with the reason stating why the specific reader
replaces the generic blanket (which still fires unconditionally beside it).

**Its kill, and its limit, measured rather than asserted.** `xtask census-join` watches the row:

```
# planted: covered_by = "Advisory::Section501c18DeductionForgoneXX"
CENSUS_EXIT=1
xtask census-join: the census join found 1 finding(s):
2024/f1040s1 form1[0].Page2[0].f2_21[0] (line "24f"): `covered_by = "…ForgoneXX"` names no variant that exists
# restored:
CENSUS_EXIT=0
census join: 274 unmodeled entries across 13 maps, … covered by an existing variant
```

★ **Honest boundary:** that check proves the named variant *exists*, not that it is the *right* one — reverting to
`Advisory::UnmodeledDeductionsOmitted` would also pass. A join asserting "the advisory's own message names this
line" would have to live in `btctax-forms`' tests or `xtask` — `btctax-core` cannot read the maps, because
`btctax-forms` depends on `btctax-core` and not the reverse — and neither is in this parcel.

---

## 4. B1 — twelve plants, twelve reds. Every changed verdict, planted and measured.

`plant → run → restore` from a byte-identical `cp` backup (never a VCS revert). Restore verified by `md5sum`:
both files match the backup exactly, and `grep -c PLANTED` is `0` in each.

| # | plant (the pre-change state) | tests run | result |
|---|---|---|---|
| P1 | `TP` → `DrivesAnUncomputedLine` | 3 | **RED** |
| P2 | `TT` → `DrivesAnUncomputedLine` | 3 | **RED** |
| P3 | `L` → a refusal | 1 | **RED** |
| P4 | `P` → a refusal | 1 | **RED** |
| P5 | `Q` → a refusal | 1 | **RED** |
| P6 | `II` → a refusal | 1 | **RED** |
| P7 | `G` re-labelled `Section402gPretax` | 2 | **RED** |
| P8 | `EE` re-labelled `Section402gDesignatedRoth` | 2 | **RED** |
| P9 | tips trigger back to box 14b **alone** | 1 | **RED** |
| P10 | the `000` branch removed | 1 | **RED** |
| P11 | the Schedule 1-A year gate removed | 1 | **RED** |
| P12 | EIC trigger back to `earned_income` alone | 1 | **RED** |

Verbatim, the ones that matter most:

```
PLANT P1  TP: ForgoneDeductionAdvised -> the pre-FR-216 refusal
  exit=100  RED (the kill works)
  FAIL a_verdict_that_promises_an_advisory_names_one_that_exists
    left:  [("H", "Schedule 1 line 24f", …), ("TT", "Schedule 1-A Part III", …)]
    right: [("H", …), ("TP", "Schedule 1-A Part II", "Advisory::TipsDeductionForgoneWithTtoc"), ("TT", …)]
  FAIL the_box14b_and_box12_tp_tips_surfaces_agree_about_one_w2
    assertion `left == right` failed: ★ THE KILL: a W-2 with box 12 code TP is a truthful W-2 whose tips are
      already inside box 1 — the return must FILE, and forgoing the §224 deduction can only OVERSTATE the tax
    left: Some(UnsupportedBox12Code("TP"))   right: None
  FAIL an_inert_box12_code_files_and_a_consequential_one_refuses_by_name
    ★ box 12 code TP makes no figure on this return WRONG and must file
```

```
PLANT P2  TT: ForgoneDeductionAdvised -> the pre-FR-216 refusal
  FAIL box12_code_tt_files_and_advises_and_goes_quiet_once_part_iii_is_claimed
    ★ THE KILL: qualified overtime is wages, already inside box 1 — the return must FILE
    left: Some(UnsupportedBox12Code("TT"))   right: None
  FAIL a_verdict_that_promises_an_advisory_names_one_that_exists  (TT row missing)
  FAIL an_inert_box12_code_files_…  (★ box 12 code TT … must file)

PLANT P3/P4/P5/P6  L / P / Q / II: NoLineReadsIt -> the pre-FR-216 refusal
  FAIL an_inert_box12_code_files_and_a_consequential_one_refuses_by_name, each time:
    ★ box 12 code L  makes no figure on this return WRONG and must file  left: Some(UnsupportedBox12Code("L"))
    ★ box 12 code P  …                                                   left: Some(UnsupportedBox12Code("P"))
    ★ box 12 code Q  …                                                   left: Some(UnsupportedBox12Code("Q"))
    ★ box 12 code II …                                                   left: Some(UnsupportedBox12Code("II"))
```

```
PLANT P7  FR-217: code G re-labelled Section402gPretax
  exit=100  RED (the kill works)
  FAIL the_capped_set_is_both_sides_of_the_limit
    assertion `left == right` failed: the §402(g) limb, pre-tax
    left: {"D", "E", "F", "G", "S"}   right: {"D", "E", "F", "S"}
  FAIL the_excess_deferral_refusal_names_the_457b_limb_only_when_there_is_457b_money
    ★ THE FR-217 KILL: a $40,000 code G breach must tell the filer their §457(b) plan has a limit of its own
      — it must name "457(b)". Got: … more than the $23000 the Form 1040 instructions allow "under all plans"
      … all of that is pre-tax, so the whole $17000 excess belongs on Form 1040 line 1h. …

PLANT P8  FR-217: code EE re-labelled Section402gDesignatedRoth
  FAIL the_capped_set_is_both_sides_of_the_limit — the §402(g) limb, designated Roth
    left: {"AA", "BB", "EE"}   right: {"AA", "BB"}
  FAIL the_excess_deferral_refusal_names_the_457b_limb_… (same kill, code EE)
```

```
PLANT P9  tips trigger reverted to box 14b ALONE
  FAIL the_tips_advisory_reads_box_12_code_tp_and_never_claims_000_is_a_listed_occupation
    ★ THE KILL: box 12 code TP is evidence of tips all by itself and must be advised

PLANT P10 the `000` branch removed
  FAIL … ★ THE KILL: a lone 000 is the employer stating a NONQUALIFYING occupation: "… Code(s) 000, which is
    your employer's statement of the occupation the tips were earned in. …"

PLANT P11 the Schedule 1-A year gate removed
  FAIL … ★ THE KILL: TY2024 has no Schedule 1-A at all, so a note naming Part II or Part III would be a
    false sentence about this return.   left: 2   right: 0

PLANT P12 EIC trigger reverted to `earned_income` alone
  FAIL … ★ THE KILL: box 12 code Q is earned income the filer may elect in, so a return carrying it and
    nothing else must still be told the EIC was not computed
```

★ **One measurement artefact, recorded because it is FR-90 exactly.** The driver's closing "tree is clean again"
re-run reported a **phantom RED** on the code-`II` plant. Cause: `shutil.copy2` preserves the backup's *older*
mtime, so cargo judged the file unchanged and reused the **planted** rlib. The files were byte-identical to the
backup (md5 verified) and `make gate` — which touches every `.rs` first — is green. This is the exact failure
`make gate` exists for, observed again.

### The two surfaces, agreeing — the test the brief asked for

`the_box14b_and_box12_tp_tips_surfaces_agree_about_one_w2` lives in `return_refuse.rs`'s tests, the one place
where **both** surfaces can be called (`btctax-core`'s `advisories()` is `pub`, and `ri()`/`refusal()` give a
fully-answered fixture that the screen accepts). On **one conforming 2026 W-2** — $9,000 in box 12 code TP plus
box 14b `102` — it asserts:

1. the **screen admits** it (the half that used to refuse);
2. the **advisory surface says exactly ONE thing** about it, naming `102`, `code TP`, `$9,000`, `CEILING`,
   `Schedule 1-A Part II` and `OVERSTATED`;
3. with Part II **claimed**, both surfaces go quiet — no refusal, and no nagging.

★ Both halves have to be in one test, because either alone passes on the contradiction: a screen test alone
passes while the advisory is dead, and an advisory test alone passes while the screen refuses the very return it
would have advised. What this asserts is the **agreement**. P1 reds it from the screen side; P9 reds it from the
advisory side.

**Tests added (6):** `the_box14b_and_box12_tp_tips_surfaces_agree_about_one_w2`,
`box12_code_tt_files_and_advises_and_goes_quiet_once_part_iii_is_claimed`,
`the_excess_deferral_refusal_names_the_457b_limb_only_when_there_is_457b_money`,
`the_tips_advisory_reads_box_12_code_tp_and_never_claims_000_is_a_listed_occupation`,
`the_schedule_1a_advisories_are_silent_in_a_year_that_has_no_schedule_1a`,
`the_eic_advisory_counts_combat_pay_and_medicaid_waiver_payments_as_electable_earned_income`.

**Tests extended (4):** `the_capped_set_is_both_sides_of_the_limit` (limb partitions asserted alongside the two
sides), `a_verdict_that_promises_an_advisory_names_one_that_exists` (three rows, each advisory's message held to
the line the table promised), `an_inert_box12_code_files_and_a_consequential_one_refuses_by_name` (six codes move
to the admit list; `TP` in the named-line refusal list is replaced by `T` → Form 1040 line 1f),
`every_param_free_rule_is_censused_from_the_source_and_fires_on_both_paths` (fixture derived).

---

## 5. Report-don't-edit — findings outside this parcel's files

1. **★★★ The TY2026 Schedule 1-A DRAFT restructures Part II, and btctax models the TY2025 shape.** The 2026 draft
   replaces lines 4a/4b/4c with a **per-employer table** whose column (iii) is literally *"Qualified tips included
   in Form W-2, box 12, code 'TP'"*, column (iv) is *"Form 4137, line 1, column (c)"*, and column (v) is *"the
   larger of column (iii) or column (iv)"* (`design/forms/extract/f1040s1a--2026-DRAFT.txt`, Part II; Part III's
   line 16 column (iii) is the same shape for code `TT`). `Schedule1aTips` carries a single
   `qualified_tips_reported` figure. So for the **first filed year** the form wants a per-employer breakdown keyed
   to box 12. ★ And note column (iv): qualified tips can legitimately **exceed** the code TP figure via Form 4137,
   which is why I did **not** build a "Part II ≤ code TP" consistency refusal — the inequality is not sound. Owner:
   the TY2026 port, not this parcel.
2. **`Advisory::TipsDeductionForgoneWithTtoc` should be renamed** to drop `WithTtoc`, since the trigger is now
   wider than the name. Blocked here: needs `crates/btctax-core/src/tax/return_inputs.rs:102` (an intra-doc link —
   parcel **E**) and `crates/xtask/src/box_census.rs:721` (a census note — parcel **G**). The boundary is stated in
   the variant's own doc in the meantime.
3. **`design/SPEC_input_form.md:357` is now stale twice over:** *"(non-inert codes refuse `UnsupportedBox12Code`;
   D/E/F/G/S over §402(g) refuse `ExcessElectiveDeferral`)"* — the refusing set is now `A B K M N R T Z FF TA`, and
   the capped set is `D E F G S AA BB EE` across **two** statutory limbs.
4. **The box 12 year axis** (§1.4). Derive each code's first archived revision from the extracts
   `the_box12_table_is_the_irs_table` already parses; do **not** type a year beside that derived set.
5. **Archive 26 U.S.C. §402(g) and §457** (or an IRS text that states both limits). That is the one artefact that
   would let FR-217 be *settled* rather than resolved conservatively, and it would also retire the FR-206 comment's
   dependence on an instruction paragraph for a statutory question.
6. **`f1040s1.map.toml:144` (line 8s) and `f1040.map.toml:323` (line 1i)** are
   `covered_by = "QuestionId::OtherOutOfScopeIncome"` — the scope attestation, whose prompt names *"Medicaid waiver
   payments"* and *"a Nontaxable combat pay election"* and which **refuses on a truthful YES** at
   `return_refuse.rs:3797`, **before** the box 12 loop. So a code-`II`/`Q` filer who answers that question
   truthfully is still refused, by a better-aimed rule; this change moves the needle only for one who answers NO.
   ★ Worth deciding deliberately whether the attestation should cover a W-2-reported waiver payment at all: the
   1040's own line 1d is titled *"Medicaid Waiver Payments **Not Reported on Form(s) W-2, Box 1**"*, so a code-`II`
   filer could reasonably read the attestation as not applying to them. Also note `f1040s1.map.toml:144`'s reason
   already says *"a negative line, so forgoing it overstates"* — which §0.1 shows is true only of box-1-included
   payments, not of the code `II` amount.
7. **`FF`'s boundary** (§1.3) — restate the reason or change the verdict, as wave1-A recommended.
8. **FR-215's join is variant-existence only** (§3). A "the advisory's message names this line" join belongs to
   `btctax-forms`' tests or `xtask`.
9. **Stale `INERT_BOX12_CODES` references** in `f1040.map.toml:320`, `f1040s2.map.toml:83/168/183` and
   `forms/2025/f1040s2.map.toml:158/290` — that const was deleted by FR-197. The conclusions still hold (`T`,
   `A`/`B`/`M`/`N` and `K` all still refuse); the named mechanism no longer exists. Pre-existing and cosmetic, but
   it is the fourth stale-mechanism citation in this area.
10. **Suggested FOLLOWUPS entries** (controller's to file): the box 12 year axis (Minor, owning phase the TY2026
    port); the advisory rename (Nit, opportunistic — needs E's and G's files); `SPEC_input_form.md:357` (Nit); the
    statute archive (Minor, owning phase "before FR-217 is ever re-opened"); the Schedule 1-A Part II/III TY2026
    restructure (**Important**, owning phase the TY2026 port); the `OtherOutOfScopeIncome` overlap for codes
    `II`/`Q` (Minor); `FF`'s reason (Minor); FR-215's message-level join (Nit).
