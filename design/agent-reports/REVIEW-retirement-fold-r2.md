# Review r2 — the FOLD of retirement-spec review r1

**Artifact:** `design/ty2025/SPEC_retirement_income.md`
**Range reviewed:** `git diff 3a4118011..108017c28 -- design/ty2025/SPEC_retirement_income.md`
(commits `3807e48d6`, `12956b419`, `108017c28`)
**Brief:** `design/agent-reports/BRIEF-retirement-fold-review-r2.md`
**Date:** 2026-09-20

**Counts: 1C / 8I / 5M / 1N.**

---

## What I checked and found SOUND — so budget is not spent re-litigating it

- **★★★ THE BRIEF'S SHARPEST ISSUE: the I-3 fold's reading of line 4a on the Q/T sub-branch is CORRECT.**
  Your reading ("the *But if…* sentence replaces only the 4b / Form 8606 half") is the one the text
  supports, on three independent grounds:
  1. **Syntax.** The lead sentence carries two coordinate instructions — *"enter the total distribution
     on line 4a **and** see Form 8606 and its instructions to figure the amount to enter on line 4b"*
     (`i1040gi--2025.txt:2698-2700`). The sub-branch replaces exactly the second: *"enter -0- on line
     4b; you don't have to see Form 8606 or its instructions"* (`:2708-2709`). It is silent on 4a, and
     an instruction it does not mention is not an instruction it revokes.
  2. **The IRS's own parallel structure for a non-taxable IRA distribution is 4a = total, 4b = -0-, in
     both of the neighbouring Exceptions.** Exception 1: *"Enter the total distribution on line 4a if
     you rolled over part or all of the distribution … If the total distribution was rolled over, enter
     -0- on line 4b"* (`:2678-2686`). Exception 3: *"enter the total distribution on line 4a. If the
     total amount distributed is a QCD, enter -0- on line 4b"* (`:2727-2729`). The competing reading
     would make Exception 2's Roth sub-branch the sole member of this family that prints nothing at all
     for a distribution the payer has already reported to the Service.
  3. **Pub 590-B cannot settle it here and does not need to** — it is not in the tree
     (`legal/text/irs-publications/` holds 525, 526, 544, 550, 551, 561, 575, 915, 936 and no 590-B).
     The 1040 instructions decide it on their own.
- **I-5's S-1 premise holds for BOTH questions — verified against Pub 915, not assumed.** The lump-sum
  election is elective and comparative: *"You can elect this method if it lowers"*
  (`Pub915…txt:734`), and *"If the taxable benefits on Worksheet 4 are lower than the taxable benefits
  on Worksheet 1, you can elect to report the lower amount on your return"* (`:766-768`). The PSO
  exclusion is likewise *"you can **elect** to exclude"* (`i1040gi--2025.txt:2912-2913`). Neither can
  raise tax, so `unanswered: None` is right and neither may refuse. The `SKIPPABLE_QUESTIONS` home is
  real and the `BlindTaxpayer` precedent is accurate (`questions.rs:2919-2936`).
- **I-8's arithmetic and its correction of r1 are right.** `grep -cE '^line[0-9]'` gives 35 / 1; every
  one of the 35 is a money line (`line1a`…`line37`); TY2025's single row `line7a` is inside TY2024's
  set, so the gap is **34**; neither map has any `line4*`/`line5*`/`line6*`; `34 + 6 + 6 = 46` ✓. And
  r1's "33" really did subtract `line_set` from a count that never matched `^line[0-9]` — your
  correction is correct. `line7a` = `f1_70[0]` (2025) vs `…Line4a-11_ReadOrder[0].f1_52[0]` (2024) ✓.
- **The T-code year pair and its lesson are right.** `:2713` = 2020 (−5), `i1040gi--2024.txt:2647` =
  2018 (−6). A two-year jump across a one-year revision; `tax_year - 5` really is wrong for TY2024.
- **Three of the four Pub 915 claims are verbatim and correctly read.** The box-5 offset sentence
  (`:973-976`), the Ryan/Jordan MFJ example ($3,000 / ($500) / $2,500, `:987-992`), and the §1341 pair
  of routes with *"Your tax for 2025 is the smaller of the two amounts"* (`:970-976`) — Schedule A line
  16 vs Schedule 3 line 13z with *"I.R.C. 1341"*. The fourth is **C-nothing/I-7 below.**
- **The plural-IRA quote is verbatim** at `:2789-2795`, and TY2024 `:2726-2732` is word-identical ✓.
- **Deriving box 5 is the right design** and is transcription, not a forbidden closed form — the
  document's own step *is* the subtraction. The reasoning survives I-7 intact; only the citation fails.
- **The Schedule A box-14 authority is real:** *"Forms W-2G, 1099-G, 1099-R, 1099-MISC, and 1099-NEC
  may also show state and local income taxes withheld"* (`i1040sca--2024.txt:323-325`), and
  `W2.box17_state_tax_withheld` / `box19_local_tax` → `ScheduleALines.line5a` exist at HEAD, so the
  understatement it names is live. See I-8 for the disposition, not the diagnosis.

---

## C-1. The I-3 and I-2 folds landed in the same commit and their composition understates line 4b: a return with one code-Q Roth 1099-R and one taxable traditional IRA 1099-R gets `4b = -0-` under the rule table as written.

**Sections:** §8 R-2 + the I-3 rule table (`:889-896`); §3 S-2's I-2 adjudication table (`:219-224`); §4.1.

The I-3 rule table's rows are keyed on a **per-document** condition and its columns are **per-return
1040 lines**:

> | box 7 contains | line 4a | line 4b | asked |
> | `Q` | Σ box 1 | `-0-` | nothing |

And R-2's new clause is per-document: *"`exception_applies == Some(true)` on an IRA document **AND the
document is not on the Roth qualified sub-branch below**."*

Now take the return the I-2 fold, in the same commit, brought explicitly into scope — *"two IRA
documents are squarely in v1 scope"*:

| document | box 7 | box 1 | `exception_applies` | what fires |
|---|---|---|---|---|
| Roth IRA, qualified | `Q` | 10,000 | `Some(true)` (a Roth distribution **is** Exception 2) | exempted from R-2 by the new clause |
| traditional IRA, fully taxable | `7` | 20,000 | `Some(false)` | computes; nothing refuses |

Nothing refuses. The I-3 table says box 7 contains `Q` ⇒ **line 4b = `-0-`**. The correct figures come
from the paragraph the I-2 fold itself quotes — *"figure the taxable amount of each distribution and
enter the total of the taxable amounts on line 4b. Enter the total amount of those distributions on
line 4a"* (`i1040gi--2025.txt:2789-2795`) — i.e. **4a = 30,000, 4b = 20,000**. The spec as folded
produces **4b = -0-**, understating total income by the whole traditional distribution.

§4.1 cannot rescue it: its 4b rule is *"Σ Form 1099-R box 1 over IRA-flagged documents, **when the
filer declares no Exception applies**"*, and on this return one document declares that one does. Two
rules, one line, and the one that is stated as a table of 1040 line values is the understating one.

The instructions themselves compose per part and say so twice — *"enter the part that is not a QCD on
line 4b **unless Exception 2 applies to that part**"* (`:2731`) — so the composition rule is not an
exotic case the form declines to answer. It is answered, and the fold did not transcribe it.

**Why Critical.** Wrong result on a money line, in the understating direction, reachable with two
ordinary documents, produced by applying the fold's own table literally — and the branch has no
refusal standing behind it. It is also the exact seam class the brief asked for: two folds in one
commit, each correct alone.

**What I would change.** Make the sub-branch per-document and state the aggregation once, in the
instructions' own words: for each IRA document compute its taxable amount (`-0-` on the Q / T-yes
sub-branch, Σ box 1 on the fully-taxable branch, refuse otherwise); **line 4b = Σ of those taxable
amounts; line 4a = Σ box 1 over all IRA documents whenever more than one exists or any is on the
sub-branch.** Then relabel the I-3 table's columns as *this document's taxable amount*, not *line 4b*,
and add a KAT for the mixed return with `4b = 20,000` pinned.

---

## I-1. The I-3 fold adds a SEVENTH question with no field, no live condition, no `None` rule, no refusal id and no mutation row — in the same commit whose I-5 fold announces the count "corrected everywhere: SIX."

**Sections:** §7's two question tables; the I-3 rule table (`:895`); §10 M-12/M-13; §12 R-E.

The I-3 rule table's third column introduces a new filer question:

> | `T` and the Roth-contribution-year answer is **yes** | Σ box 1 | `-0-` | **one question** |
> | `T` and the answer is **no** or `None` | — | — | **refuse** |

`grep -i "roth_\|contribution year\|roth contribution"` over the whole spec returns **exactly one
line** — that table cell (`:895`). The question has:

- no field on `Form1099R` or `ReturnInputs` (§7's struct is unchanged but for box 7's doc comment);
- no row in §7's class-(A) table and no row in the I-5 fold's new silent table;
- no `live` condition (it must be `box7` contains `T` on an IRA document — narrower than any existing
  `live` in the spec);
- no named refusal for the `no` / `None` case (the cell says only "refuse"; R-2 is implied but its
  wording — *"a Roth to Form 8606"* — is never tied to it);
- no mutation row, while A-1 and A-2 each got one (M-12, M-13);
- and it is **absent from the count**: the I-5 fold declares *"The count, corrected everywhere it
  appears (§7 here, §10's journey walk, §12 R-E): **SIX** questions, of which **FOUR** refuse when
  unanswered and TWO are silent"*, and R-E and the journey walk were edited to match. With the T
  question the figures are **seven and five** — and the seventh is the only one of the seven whose
  `None` decides a money line.

This is r1's I-5 reproduced verbatim in structure — *"the two new questions have no field, no live
condition, no `None` behaviour, and no mutation row anywhere in the document"* — by a sibling fold, one
commit after the fold that exists to punish it. The I-5 fold's own framing applies to it: *"this is the
edited §X, forgot §Y class this project's own doctrine flags."*

**Why Important.** A question that exists only in a rule table has undefined semantics at the moment a
build reads §7, and the ambiguity has a direction: if a builder reads R-2's *"not on the Roth qualified
sub-branch"* as satisfied by a `T` in box 7 (the sub-branch is, after all, defined by box 7), then a
`T` document with an unanswered year takes the `-0-` branch and understates. The intended reading is
recoverable from the table's third row, but it is recoverable rather than stated.

**What I would change.** Give it a field (`roth_contribution_before_5yr_window: Option<bool>` on
`Form1099R`, or a name keyed to the year the revision prints), a row in §7's **class-(A)** table with
`live` = *this document's box 7 is exactly `T`* and `None ⇒ R-2`, one mutation row (leave it `None` on
a `T` document; R-2 must fire, and the `-0-` branch must not be taken), and correct the count to seven
/ five in §7, §10 and R-E.

---

## I-2. §4.1 and §6 still state the retracted claim as the v1 behaviour, and they are the two tables a build implements from.

**Sections:** §4.1 (`:449,451`); §6 (`:566,570`); against S-2's I-2 adjudication (`:219-224`).

The I-2 fold retracts *"line 4a is structurally never populated"* and *"line 5a is populated only when
the 1099-R shows a smaller box 2a"* — inside S-2. Both survive unedited elsewhere:

| section | text at HEAD |
|---|---|
| §4.1 in-scope table | `| **4a** | `Option<Usd>`, **always `None` in v1** — the form instructs a blank on the only branch that computes (S-2) |` |
| §4.1 in-scope table | `| **5a** | `Option<Usd>` — `Some(Σ box 1)` when box 2a < box 1; `None` when the pension is fully taxable |` |
| §6 transcription table | `| 4a | "IRA distributions" | `:74` | `line4a: Option<Usd>` | `Collected` — **always `None` in v1** (S-2) |` |

§4.1 is the section titled *"In scope — v1 behaviour"* and §6 is the transcription/census table whose
rows become `LineCoverage` entries. A build that implements §4.1 emits `line4a = None` on every return,
which is precisely the reading S-2 now calls wrong, and which C-1 shows is the operand column the
Service document-matches. §6's row additionally cites *"(S-2)"* — a citation to the section that
retracted it.

**Why Important.** The retraction is real but it is not where a builder looks, and the spec now asserts
both readings with equal force in three places. This is the same failure the fold identifies in I-5,
two sections away from the fold that identifies it.

**What I would change.** Rewrite both §4.1 rows and §6's 4a row to the adjudicated conditions, exactly
as §7a item 2 does for line 25b (*"Line **25b** in §4.1's in-scope table and §6's transcription
table"*) — name the sections the change lands in, and make the change. A ⚠️ retraction marker in §3
is not a substitute for the table a builder reads.

---

## I-3. The I-2 fold's 5a adjudication is built on the Simplified Method Worksheet — which this spec REFUSES — and leaves the in-scope partially-taxable case in neither row of its own table. The governing paragraph sits 90 lines earlier and is never cited.

**Sections:** S-2's adjudication table row 3 (`:222`) and its ★★★ item 4 (`:210-214`); §1 (`:89`);
against §4.2 (`:466`) and §5 (`:503`).

The fold's strongest new claim is:

> **★★★ Line 5a is populated unconditionally whenever the Simplified Method runs**, by that worksheet's
> own line 1 … **That is the worksheet this spec builds** (`:2981-2982`).

The quote is verbatim (`i1040gi--2025.txt:2981-2982` ✓). **The clause after it is false, and the spec
says so twice in its own words:**

- §4.2: `| Simplified Method / General Rule (`2973-3036`, Pub. 939) | needs cost at the annuity
  starting date … | R-3 / R-4 |` — i.e. non-scope, refused.
- §5 (`:503`): *"**The Simplified Method Worksheet is the opposite call and is REFUSED** (R-3/R-5)."*

The worksheet this spec builds is the **Social Security Benefits Worksheet** (S-4). The same false
claim appears a second time in the I-10 fold in this range, at `:89`: *"box 5 and box 9b (the
Simplified Method's cost in the plan — **the very worksheet this spec builds**)"*.

The consequence is not only rhetorical. The adjudication table's 5a rows are:

| 5a | populated when | authority |
|---|---|---|
| 5a | the Simplified Method runs **or** more than one *partially taxable* pension | `:2981-2982`; `:2978-2980` |
| 5a | left blank on a **single** fully-taxable pension | `:2876-2880` |

In v1 the first row's first limb never fires (refused), and its second limb — *"More than one pension
or annuity. If you had more than one **partially taxable** pension or annuity…"* — is at `:2978-2980`,
which is the **"Before you begin" header of the Simplified Method Worksheet itself** (the worksheet
title is at `:2973`), not a general 5a instruction. So under the table as written, **a single partially
taxable pension — box 2a shown, box 2a < box 1, the one partially-taxable shape v1 does compute per
§4.1 — falls in neither row.**

And the paragraph that actually governs it was never cited anywhere in the spec:

> *"**Partially Taxable Pensions and Annuities.** Enter the total pension or annuity payments (from
> Form 1099-R, box 1) on line 5a."* — `i1040gi--2025.txt:2888-2891`

That is unconditional, outside any worksheet, and it is the authority §4.1's 5a rule needed all along.
It also disposes of the "asymmetry is the tell" argument in the fold's item 2: the *"partially
taxable"* limb at `:2978` is scoped because **the worksheet it heads only applies to partially taxable
pensions**, not because the IRS drew a deliberate contrast with the IRA rule 190 lines earlier. The
two are not, as the fold says, *"Two adjacent rules."* The conclusion about 4a stands on item 1 alone;
item 2 does not carry weight it is presented as carrying.

**Why Important.** A money cell's population rule now rests on an out-of-scope worksheet, the in-scope
case it must cover is unaddressed, and the governing sentence is uncited — exactly the "true but
unverifiable as written" class the brief names. It also puts §6's `5a` row and M-4 (*"line 5a is …
present when box 2a < box 1"*) at odds with S-2.

**What I would change.** Replace the first 5a row with `:2888-2891` as the authority — *5a = Σ box 1
whenever any pension is partially taxable (box 2a < box 1), by the Partially Taxable Pensions
instruction* — and demote the Simplified Method limb to a note reading *"and the Simplified Method
Worksheet writes 5a as its line 1 if that worksheet ever comes into scope; it is refused in v1 (§4.2)."*
Delete *"the very worksheet this spec builds"* at both `:89` and `:210`.

---

## I-4. M-1b is unbuildable as specified. `LineCoverage.field` is the RUST field name; `label_join` is keyed by the ACROFORM field name. The bridge between them is the map row that I-8 proves does not exist for any of these six lines.

**Sections:** the I-4 fold's ★★ paragraph (`:376-379`); §10 M-1b.

The fold asserts:

> ★★ Coverage rows are **per field** … so the row already carries the key `label_join` is indexed by.
> **Binding `row.line` to `label_join[row.field]` is therefore a lookup, not a heuristic.**

Both halves of the key-space claim are wrong, and the struct says so:

```
crates/btctax-core/src/tax/line_coverage.rs:251-252
    /// The Rust field, e.g. `"line16"`.
    pub field: &'static str,
```

and

```
crates/xtask/src/label_reader.rs:1343-1350
/// The line→box join: for every AcroForm box, the printed line label that governs it.
pub fn label_join(stem: &str) -> Result<std::collections::BTreeMap<String, String>, String>
```

Measured, the two keys for the one line both maps already carry:

| source | key |
|---|---|
| `LineCoverage.field` | `"line7a"` |
| `label_join("f1040--2024")` | `"topmostSubform[0].Page1[0].Line4a-11_ReadOrder[0].f1_52[0]"` |

`label_join[row.field]` is a lookup of `"line4b"` in a map keyed by `…f1_47[0]`. It misses on every
row. The quoted in-repo phrase is accurate but means the other thing: `line_coverage_check.rs:1629`
reads *"Keyed on DISTINCT LINES, not rows. A coverage row is per FIELD, so one line legitimately…"* —
*field* there is the Rust field, which is why one Rust field can serve two form lines.

**The bridge exists and it is the map** — `crates/btctax-forms/forms/<year>/f1040.map.toml`, which maps
`line7a = "topmostSubform[0]…f1_52[0]"`. And `grep -E '^line(4|5|6)'` returns **nothing** in either
year's file, which is the I-8 fold's own headline finding. So M-1b's kill is **circular**: it proposes
to verify the attribution of the 4a–6b coverage rows through a map row that does not exist until those
same rows are built, and whose correctness is the thing under verification.

**Why Important.** The brief asked exactly this and the answer is the unfavourable one: M-1b prescribes
an implementation that cannot be written as stated, and it is the row that carries the guarantee the
I-4 fold hands the build in exchange for dropping S-9's prerequisite framing. A build reaching M-1b
discovers the gap a round late.

**What I would change.** State the three-way join explicitly: `row.(form, year, field)` → the map's
AcroForm name → `label_join(stem)[acroform_name]` → the printed label, compared against `row.line`.
Say that it therefore requires the map rows for 4a–6b **and must be added in the same task that adds
them** (so the map row and the label agree by construction rather than by inspection), and keep the
I-4 fold's fallback sentence — if it is deferred, S-9 must say the mutual attribution is not
machine-held and name the KAT standing in.

---

## I-5. The box-7 "parse to a code SET, never `contains`" rule is justified by a citation that says the opposite for `Q` and `T`, and the test it prescribes is LOOSER than the authority. The right test is equality, not membership.

**Sections:** §7's `box7_distribution_codes` doc comment (`:604-609`); the I-3 fold's ★ paragraph
(`:906-910`).

The fold's authority:

> ★ Parse to a code SET; never `contains`, **because box 7 is composed**
> (`i1099r--2025.txt`: *"If any other code, such as 8 or P, applies, use Code J"*).

That Note is printed twice and both times it sits **inside the `Q` and `T` entries of Table 1**
(`i1099r--2025.txt:2414` under *"Q—Qualified distribution from a Roth IRA"*, `:2447` under
*"T—Roth IRA distribution, exception applies"*). It says: for these two codes, if another code
applies, **use Code J instead**. It is the authority for `Q` and `T` being used **alone**, not for
their being composed. Table 1's *used with* column for `Q` reads **`None`** (`:2416`, between Q's entry
and R's — R's own `None` follows at `:2428`; T's cell is dropped by the layout at `:2448` and must not
be asserted from this extract).

Box 7 **is** composable in general, and the authority for that is a different sentence the fold never
cites: *"Enter a maximum of two alphanumeric codes in box 7. See Table 1 for allowable combinations"*
(`:1930-1931`), with *"If two or more distribution codes are not valid combinations, you must file more
than one Form 1099-R"* (`:1927-1929`).

Two consequences, and they run the wrong way for the money:

1. **`set.contains("Q")` is looser than the instruction, not tighter.** Because the payer is told to
   use `J` whenever anything else applies, a box 7 whose parsed set *contains* `Q` alongside another
   code is a form the payer was forbidden to issue — and admitting it to the `-0-` branch is the
   understating direction. The rule the authority supports is **box 7 is exactly `Q`** (or exactly
   `T`), with anything else falling through to R-2.
2. **The hypothesised failure mode cannot occur.** The fold justifies the SET with *"a future
   alphanumeric code containing the letter would silently take the `-0-` branch"*. Every code in
   Table 1 is a **single** character (1–9, A–N, P, Q, R, S, T, U, W) and box 7 holds at most two of
   them, so no code can contain another. The real hazard — an adjacent second code, e.g. a typed
   `"Q8"` — is the one equality closes and membership opens.

The separator is also unstated, which the brief anticipated. Box 7 has two printed positions
(`f1099r--2025.txt:48-52`, *"7 Distribution code(s)"* beside the IRA/SEP/SIMPLE box) and codes are
adjacent characters on the paper, so *"parse to a code SET"* over a `String` has no stated
tokenization; an implementer splitting on `,` gets one token `"Q8"`, and an implementer splitting on
characters gets `{Q, 8}` — with opposite outcomes under a `contains` test.

**Why Important.** A money branch's trigger is specified by a citation that supports the opposite rule,
and the prescribed implementation admits a case the authority excludes. Same class as r1's I-7.

**What I would change.** State the rule as equality and cite the two sentences that support it: *the
sub-branch applies only when box 7, after trimming, is exactly `Q` or exactly `T` — Table 1 gives
`Q`'s "used with" as `None` (`i1099r--2025.txt:2416`) and routes any combination to Code J (`:2414`,
`:2447`); box 7 may otherwise carry up to two codes (`:1930-1931`), and any such box 7 falls through
to R-2.* Then the KAT is *a two-code box 7 containing `Q` must NOT take the `-0-` branch* — which is a
kill, whereas the fold's *"the KAT set must include a multi-code box 7"* does not say what the answer
should be.

---

## I-6. §9's A-1 and A-2 still fire on the RETURN's condition, which the I-5 ruling and the owner's "zero when nothing is wrong" ruling replace — so M-12's and M-13's own plants would red against §9 as written.

**Sections:** §9 (`:1022-1030`) against §7's I-5 ruling table (`:748-750`) and §10 M-12/M-13.

The I-5 fold rules that A-1 and A-2 fire on the new fields — *"advisory only (§9)"* on `Some(true)`,
silent on `None`, and *"the §9 advisory on `Some(true)`"*. §9's advisory definitions are unchanged:

> **A-1 `PsoPremiumExclusionNotTaken { pensions: usize }`** — **fires when line 5b > 0.**
> **A-2 `SocialSecurityLumpSumElectionNotTaken`** — **fires when line 6b > 0.**

Those are the return-conditions §9's own ruling paragraph rejects (*"An advisory must be keyed on
whether it applies to THIS filer"*), and they are what r1's I-6 red-flagged for A-4 (*"it fires on
**100%** of in-scope pension returns"*). The new mutation rows then contradict §9 directly:

- **M-12 plant (b)**: *"`Some(false)` ⇒ it must not [appear]."* Under §9 as written it appears anyway,
  because 5b > 0. The plant reds on a spec-conformant implementation.
- **M-13 plant (a)**: *"`Some(true)` ⇒ the §9 advisory must appear"*, with `live` = `Σ box 5 > 0`. A
  filer whose benefits are not taxable has `Σ box 5 > 0` and `6b = 0`, so §9 suppresses A-2 and the
  plant reds. (§9's own quoted authority agrees with §9 here — *"If any of your benefits are taxable
  for 2025 and they include a lump-sum benefit payment…"*, `:3470-3471` — so the mutation row, not the
  quote, is the thing to fix.)

**Why Important.** The owner ruling *"Zero is the goal when nothing is wrong"* is a stated guarantee,
and §9 as it stands violates it for A-1 on every in-scope pension return. Two of the four new mutation
plants cannot pass against the section they test. The I-5 fold enumerated the places the count appeared
(§7, §10, §12) and did not visit the section whose rewrite caused the finding in the first place.

**What I would change.** Rewrite A-1's and A-2's firing conditions in §9 to
`retired_public_safety_officer == Some(true)` and `benefits_included_an_earlier_year == Some(true)`
respectively, and add *"and line 6b > 0"* to A-2 so M-13's plant (a) matches its own authority. While
there, note that A-4's `5b > 0` condition is now the one r1's I-6 already ruled on, so it does not
quietly become the thing A-1 used to be.

---

## I-7. The Pub 915 caption quoted three times as the SOLE authority for deriving box 5 does not appear in the archived publication. The archive is the 2025 edition and prints "Net Benefits for 2025."

**Sections:** §7's `box5_net_benefits` doc comment (`:627`); the I-11 fold (`:661-662`, `:671`); §1's
authority table row for `Pub915…pdf` (`:64`).

The fold quotes, three times, in the form cite-check would grade:

> *"Box 5. Net Benefits for **2024** (Box 3 minus Box 4)"*

Measured against the archived text:

```
$ grep -c "Net Benefits for 2024" legal/text/irs-publications/Pub915_Social_Security_and_RRB_Benefits.txt
0
$ grep -n "Net Benefits for" …Pub915….txt
1246:  Box 3. Benefits Paid in 2025   Box 4. Benefits Repaid to SSA in 2025   Box 5. Net Benefits for 2025 (Box 3 minus Box 4)
1349:amount of any lump-sum benefit payment received in    Box 5—Net Benefits for 2025 (Box 3 Minus
1425:  Box 3. Benefits Paid in 2025   Box 4. Benefits Repaid to SSA in 2025   Box 5. Net Benefits for 2025 (Box 3 minus Box 4)
```

and the publication identifies itself as *"Publication 915 (2025)"* (`:61`, `:1634`), *"For use in
preparing 2025 Returns"* (`:29-31`).

This is the one citation that carries the whole I-11 design decision. The fold's argument is *"the
document's own step **is** the subtraction, printed in the box's caption, so applying it is following
instructions"* — a derivation permitted **only** because the caption says so. The caption as quoted is
not in the authority, and the year in it is not the year of the archived document.

Compounding it, §1's authority table row is `| Pub915…pdf | 44c4053d | … |` — the only rows in that
table with **no revision year**, in a repo whose own `LineCoverage.year` doc comment says *"Per-row,
not a module constant: a form is year-shaped"*, and in a spec that covers TY2024 **and** TY2025. The
spec therefore quotes a 2024-shaped caption from a 2025 publication for a two-year feature.

**Why Important.** The brief's rule is explicit — a claim whose citation does not say it is at least
Important — and here the citation is the single support for a design that deliberately stops
collecting a filer-visible box. It is also invisible to tooling: `cite-check` does not cover this spec
(FR-257), and `legal/text/irs-publications/` is not in `design/forms/extract/`, so no checker would
ever reach it.

**What I would change.** Quote `:1246` verbatim — *"Box 5. Net Benefits for 2025 (Box 3 minus Box 4)"* —
in all three places, or quote the year-neutral `:1349` heading. Year-stamp the `Pub915…pdf` and
`Pub575…pdf` rows in §1 (the archive is the 2025 edition; a TY2024 edition is not in the tree), and
say in §7 that the caption is quoted per revision so the TY2024 build re-reads it.

---

## I-8. The fold discovers a money defect — 1099-R box 14 → Schedule A line 5a — and files it nowhere: no §7 field, no owning phase, no FOLLOWUPS entry. The two non-money findings from the same fold both got FR numbers.

**Sections:** §1's I-10 fold (`:76-90`) against §7's `Form1099R` (`:590-613`) and `FOLLOWUPS.md`.

The fold's own words:

> **Form 1099-R box 14, *State tax withheld*, feeds Schedule A line 5a, and §7 omits it.** … An
> itemizing retiree understates line 5a by the whole of their state pension withholding. ★ This is the
> SAME defect interview T11 found and fixed for **W-2 boxes 17 and 19** ([[FR-91]]).

The diagnosis is correct — verified above — and the disposition is one prose sentence: *"★ §7 must
therefore be rewritten against the extract, not patched."* Measured at HEAD:

| where a builder would look | what is there |
|---|---|
| §7 `Form1099R` | five boxes (1, 2a, 2b, 4, 7) — no `box14`, no boxes 15–19 |
| §4.1 / §4.2 / §6 | no mention |
| §8 refusals | none for state withholding |
| §10 mutations | none |
| `FOLLOWUPS.md` | `grep -n "box 14"` returns only W-2 box 14b rows (FR-91 family). **No 1099-R entry.** |
| this commit's FOLLOWUPS delta | FR-257 (cite-check scope) and FR-256 (negative parse) — both non-money |

So the one finding in this fold with a dollar consequence for the filer is the one with no owning
phase, while the two without one each got an entry with an owning phase named. Contrast §7a, which
handled the structurally identical C-2 (SSA box 6 → line 25b) correctly: *"What T14 must carry: 1.
`box6_fed_withheld: Usd` on `FormSsa1099` … 4. Mutation rows in §10, one per new family."*

**Why Important.** The project's follow-up rule is that an item is burned down on its owning phase and
that *"Record the owning phase in each follow-up entry so reconciliation is a grep"*. An unrecorded
money defect is not deferred, it is lost — and the repo's own ledger lesson is that a finding held only
in prose stops being actionable. The understatement is live: `ScheduleALines.line5a` is computed and
`W2.box17_state_tax_withheld` already feeds it, so the 1099-R is the only withholding source with no
route.

**What I would change.** Either (a) add `box14_state_tax_withheld: Usd` (and the boxes 15–19 that ride
with it, or an explicit "carries no decision, with a reason" note per the provenance gate) to §7 with a
§10 mutation row in the shape of §7a item 4 — *drop the 1099-R box-14 term from Schedule A 5a; an
itemizing retiree with pension withholding must stop reconciling* — or (b) file it as a FOLLOWUPS entry
with **owning phase = the §7 rewrite (retirement build)**, the `i1040sca--2024.txt:323-325` quote, and
the FR-91 precedent. Do not leave it as a ★ in §1.

---

## M-1. The I-11 fold duplicates a paragraph and orphans a truncated quote that begins mid-clause with a stray delimiter.

**Section:** §7, `:659-672`.

Lines 659-664 and 669-672 are the same paragraph twice (*"I-11's second half is the one that decides
the design"* / *"I-11's second half is the deeper one"*), evidently a rewrite inserted above the
original without deleting it. Between them:

```
666: Pub 915 states the negative case in its own words, on **both** forms:
667: > figure in box 3. This is a negative figure and means you repaid more money than you received."*
```

The block quote starts mid-clause, has no opening delimiter and a stray closing `"*`. The sentence in
the authority is *"If parentheses are around the figure in box 5, it means that the figure in box 4 is
larger than the figure in box 3. This is a negative figure and means you repaid more money than you
received in 2025"* (`Pub915…txt:1358-1361`, and the RRB analogue at `:1808-1812` — which is about
**boxes 10/11/12**, not box 5, so *"on both forms"* needs the RRB box numbers if it stays).

**Why Minor.** No rule changes; it is the *"never quote a truncated read"* shape rather than a wrong
figure. But it is in the doc comment region a build copies into source.

**What I would change.** Delete `:669-672`, restore the quote from `:1358-1361` in full, and give the
RRB sentence its own box numbers.

---

## M-2. The Exception-2 citation ranges do not contain the sentences they are cited for.

**Sections:** the I-3 fold (`:874`); S-2's item 3 (`:206-208`) and adjudication table (`:222`).

- *"the same sentence at TY2024 `:2640-2649`"* — the load-bearing sentence (*"But if either (a) or (b)
  below applies, enter -0- on line 4b…"*) is at TY2024 `:2638-2639`; `:2640` is blank and `:2641` is
  the page footer *"Need more information or forms? Visit IRS.gov."* The cited range holds only
  sub-branches (a) and (b).
- The 4a row's authority `:2707-2715` does not contain *"enter the total distribution on line 4a"* —
  that is the Exception-2 lead at `:2698`. Item 3's *"**Exception 2 sub-branch (b)** instructs *'enter
  the total distribution on line 4a'*"* attributes to the sub-branch a phrase the lead sentence
  carries; that attribution is the very thing C-1's competing reading turns on, so it should be exact.

**What I would change.** Cite TY2024 `:2628-2649` and `:2698-2715` / `:2698,2707-2715`, and reword item
3 to *"Exception 2's lead sentence (`:2698`) instructs 4a, and sub-branch (b) (`:2714-2715`) replaces
only the 4b half."*

---

## M-3. §7's box-4 doc comment still keys R-6 per form, three lines above the fold's ruling that it must be a Σ across all forms and both spouses.

**Sections:** §7 (`:622`) against the I-11 fold (`:689-695`) and §8 R-6 (`:946`).

`/// Box 4 — "the amount of any benefits you repaid in 2025" (:3237-3239). **Box 4 > box 3 ⇒ R-6.**`
sits in the code block the fold edited. The fold then rules the opposite, correctly and with the
authority: *"R-6's Σ is right, and must stay a Σ"* and *"On MFJ it must sum across BOTH spouses"* —
which Pub 915 confirms (*"If the **total** amount shown in box 5 of **all** of your Forms SSA-1099 and
RRB-1099 is a negative figure…"*, `:941-945`; the Ryan/Jordan example at `:987-992`). §8's firing
condition is already the right one, `Σ box 4 > Σ box 3`.

**Why Minor.** §8 is authoritative and correct, so nothing is wrong-by-construction — but the fold
asserted the rule adjacent to a line stating its negation and left the negation in a doc comment a
build will copy. A per-form R-6 is the too-wide refusal the fold itself says costs the filer the return.

**What I would change.** Change the box-4 doc comment to *"Σ box 4 > Σ box 3 over ALL SSA-1099 /
RRB-1099 documents, both spouses ⇒ R-6 (Pub 915 `:941-945`, `:987-992`)."*

---

## M-4. The I-5 fold says "the build has no decision left to make here" but never names the classifier bucket, which the compiler will demand.

**Section:** §7's I-5 ruling (`:753-767`).

`ReturnInputs`' `Option<bool>` leaves are partitioned by an exhaustive destructure in
`classifier.rs`, so the two new fields force a decision at compile time. The precedent the fold
already picked for everything else answers it too:

```
crates/btctax-core/src/tax/classifier.rs:644-652
        blind,
    …
    c.exempt(
        blind,
        Class::BenefitClaim,
        "§63(f) blindness — New Colonial Ice: the burden to CLAIM is the filer's, so `false`/absent is
         lawful; the forgone benefit fires `BlindBoxForfeitedNotDeclared` (§2.2)",
    );
```

The fold cites `c.exempt(…)`'s existence to refute the false-dilemma premise but does not say that
A-1/A-2 must be registered with it, nor with which `Class`.

**Why Minor.** The compiler holds it and the precedent is one line away, so nothing ships wrong — but
the fold's claim that no decision remains is not accurate, and naming `c.exempt(…, Class::BenefitClaim,
…)` costs one clause.

---

## M-5. §6's closing note still gates the three new quotes on "S-9's checker fix", which the I-4 fold retracted as a gate — and r1's I-4 named that note explicitly.

**Sections:** §6 (`:581-582`) against the I-4 fold's revised position (`:388-392`).

§6 ends: *"★ Every one of these quotes must land **after** S-9's checker fix, or the three `"Taxable
amount"` rows are unverified by construction."* The I-4 fold's conclusion is the opposite: *"this is
**no longer framed as a prerequisite that gates the rows** — it is a build task."* r1's I-4 listed its
sections as *"S-9, **§6's closing note**, M-1, R-A"* and asked for the gating framing to be replaced in
all of them; S-9 and M-1 were rewritten and §6's note was not.

**Why Minor.** Framing only — nothing is built wrong either way, and R-A is legitimately left standing
as an accepted risk. But it is a third instance of the pattern in one commit, in a spot the review
named.

**What I would change.** Replace the note with what landed: *"The three `"Taxable amount"` rows coexist
because the form prints the words three times (the FR-184 pigeonhole, M-1). Their mutual attribution is
held by M-1b, not by the quote."*

---

## N-1. The pasted `SkippableQuestion` block's `help:` string shows a collapsed line-continuation as a run of spaces.

**Section:** §7 (`:772-786`).

`help: "…Skipping leaves it unclaimed — lawful, since the burden to claim is yours — and the
forgone-benefit advisory fires."` is rendered with ~12 spaces before *"forgone-benefit"*, from the
source's `\` continuation and indentation (`questions.rs:2926-2927`). The block is marked elided with
`…` so nothing is misrepresented; it just reads as a typo in a code quote a builder may copy.

---

## Method note

The fold diff was read in full. Every citation in the range was opened in the archived extract or the
publication text: `i1040gi--2025.txt` (`:2655-2730`, `:2789-2796`, `:2870-2915`, `:2965-2995`),
`i1040gi--2024.txt` (`:2630-2660`, `:2720-2740`), `i1099r--2025.txt` (`:1899-1960`, `:2400-2460`),
`f1099r--2025.txt` (`:25-61`), `i1040sca--2024.txt` (`:320-328`),
`Pub915_Social_Security_and_RRB_Benefits.txt` (`:29-62`, `:706-790`, `:940-996`, `:1246`, `:1349-1362`,
`:1425`, `:1629-1634`, `:1807-1812`). Code read at HEAD: `line_coverage.rs:238-259`,
`line_coverage_check.rs:1629`, `label_reader.rs:1343-1350`, `questions.rs:2807-2945`,
`classifier.rs:640-652`, `printed.rs:1636-1772`, `return_inputs.rs:72-74`, both `f1040.map.toml`.
The brief's pre-verified table was taken as given and not re-derived, except where a finding above
re-uses one of its facts (the 35/1/34 counts, the `f1_46`…`f1_51` bindings, the T-code year pair) and
the re-use is stated. No file other than this report was modified.
