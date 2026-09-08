# Seam review — interview build T16 (Form 8889, transcribed)

Independent adversarial build review. Worktree `agent-ae665e45084ddff34`, checkout `4b241dd7`
(`git status --short` clean at start and at end; every plant reverted from a `cp` backup under the
scratchpad, never `git checkout --`). Brief: `design/agent-reports/BRIEF-review-interview-T16.md`.
Builder's account: `design/agent-reports/2026-09-07-build-interview-T16-implementation.md`, read
whole including §0.

## Commands

```
export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review

cargo run -q -p xtask -- line-coverage        OK: 373 money lines / 18 forms, f8889:27
cargo run -q -p xtask -- box-census           OK: 268 boxes / 19 editions / 9 information returns
cargo run -q -p xtask -- census-join          290 unmodeled entries across 13 maps, every one placed
cargo run -q -p xtask -- authority-manifest   OK — every entry resolves and every source is listed
cargo run -q -p xtask -- cite-check           OK — 51 quotations; 7/38 pairs extracted, 0 unaccounted

cargo nextest run --locked -p btctax-forms                359 passed, 4 skipped
cargo nextest run --locked -p btctax-core -p xtask        1401 passed, 6 failed
    — the 6 are `xtask::form_delta::*`, exactly the PDF-less-worktree set the brief names as
      environment. btctax-core itself: 0 failed.
cargo nextest run --locked -p btctax-core -E 'test(golden)'   7/7

OTS_DIR=/home/bcg/OpenTaxSolver2024_22.07_linux64 .venv/bin/python  (both engines live, on the
    T16 corpus cell `single_w2_with_an_hsa_deduction`)
      OTS   AGI 90850.0  TI 76250.0  total 11834.0
      TAXC  AGI 90850.0  TI 76250.0  total 11828.0
    — byte-for-byte the baked `expected_ots` / `expected_taxcalc`. The $6 gap is the pre-existing
      Tax-Table-vs-formula methodology difference the corpus already carries.
```

Plants made and reverted (each reproduced independently of the builder's list):

| plant | file | observed |
|---|---|---|
| line 12's line reference: `line8 - line11` → `line8 - line10` | `form8889.rs:330` | RED — `line_9_reads_w2_box_12_code_w_through_the_worksheet` *"line 12 subtracts line 11: left 4150, right 3150"* (+ a second red in `a_contribution_over_the_years_limit…`) |
| quote drift on line 14c: *"Subtract line 14b from line 14a"* → *"…14a from line 14b"* | `line_coverage.rs` | RED — `line-coverage FAILED (1 problem(s)): f8889:14c (line14c) quotes text NOT FOUND in f8889--2024.txt` |
| wrong-section FieldId: 1099-SA box 1 → `Sa5498Box5Fmv` | `box_census.rs:933` | RED — two problems: *"Sa5498Box5Fmv is in [Sa5498s], not in this document's own section (Sa1099s)"* and *"the box prints \"gross distribution\", and no field that collects it … says so"* |
| probe (not a defect plant): print `AbsoluteReturn` vs `assemble_printed_forms` on the build's own reach fixture | `form8889.rs` tests | GREEN — and that is finding C-1/I-1 |

---

## Summary

Form 8889 itself is transcribed correctly. All 21 printed lines plus the instructions' Employer
Contribution Worksheet are present in the form's own numbering with the printed text verbatim; every
computed line matches the sentence the form prints (verified line-by-line against
`design/forms/extract/f8889--2024.txt` and the arithmetic in `compute`); the §223(b) figures match
Rev. Proc. 2023-23/2024-25/2025-19 §2.01(1) read out of `legal/text/`; the 2025 edition is the same
line set (measured: the normalised diff of the two extracts is the line-13 period and the footer,
nothing else); the map, the emitter read-back, the two box censuses, the refusals and the packet
placement are all sound, and the three plants above each red.

**The defect is not on Form 8889 — it is on the wire out of it.** Two of the four figures the form
produces reach the *printed* chain and are simply never added to the *computed* chain. Schedule 1
line 8f (Form 8889 line 16 + line 20) is absent from `AbsoluteReturn::schedule_1_income` /
`total_income` / `agi` / `taxable_income`, and Schedule 2 lines 17c/17d are absent from
`schedule_2_other_taxes` / `total_tax`. The deduction leg (line 13) *was* wired into `adjustments`;
the income leg and the additional-tax leg were not. On the build's **own** reach fixture the two
chains disagree by $1,800 of AGI and $576 of total tax, and — because the exact-lane AGI is what the
§221 phase-out, the NIIT MAGI, the AMT, the §170(b) base and the CTC/ODC screen all read — it
reaches a printed, signed line: a demonstrated $1,500 overstatement of the Schedule 1 line 21
student-loan deduction.

The oracle cell cannot see it (its own `why` says so: *"With no distribution there is no Part II, so
Schedule 1 line 8f and Schedule 2 lines 17c/17d stay blank and the cell tests the DEDUCTION leg
alone"*), and the one instrument built for exactly this class —
`packet.rs::the_absolute_total_tax_equals_the_printed_1040_line_24` — runs on two fixtures, neither
of which affirms `hsa_activity`.

---

## Findings

### C-1 (Critical) — Form 8889 line 16 + line 20 never enter AGI, and a filed §221 deduction is overstated by $1,500

**Where.** `crates/btctax-core/src/tax/return_1040.rs:2107-2113` (`schedule_1_income`,
`total_income`) and `:2134-2145` (`form_8889`, `hsa_income_8f`, `adjustments`, `agi`).

**What is wrong.** `hsa_income_8f` is computed at :2136 and placed into `Schedule1Parts` at :2164 —
where only `printed::schedule_1_lines` reads it. It is *not* added to `schedule_1_income`, which is
the sole path into `total_income`, which is the sole path into `agi`. Every other Schedule 1 Part I
income (state refund, unemployment, Schedule C net, crypto 8v) is in that sum; line 8f is the one
omission. The comment at :2142 — *"Line 8f is Schedule 1 PART I income, so it belongs in
`total_income` through line 10 — see `schedule_1_income` below"* — points at a binding that does not
exist (and `schedule_1_income` is 27 lines *above*, not below).

The exact-lane `agi` is not an unread number. It is the argument to `student_loan_deduction`
(`agi_before_student_loan`, :2123), `form_8960(..., agi, ...)` (:2403, the §1411 MAGI),
`assemble_amt(... agi, taxable_income ...)` (:2440), the §170(b) contribution base and the §213(a)
floor in `schedule_a_parts`, `ctc_odc_line19`, `ti_before_qbi`, the itemize-vs-standard election,
and `regular_tax`. Four of those produce **printed** figures.

**Evidence.** Probe added to the build's own reach test
(`an_unqualified_distribution_reaches_schedule_1_line_8f_and_schedule_2_line_17c`, $3,000 distributed,
$1,200 of qualified expenses), printing `AbsoluteReturn` beside `assemble_printed_forms`:

```
PROBE   total_income=60000 schedule_1_income=0 agi=58000 taxable_income=43400 total_tax=4979
PRINTED l9=61800        l11=59800       l15=45200       l16=5195  l23=360  l24=5555
```

Computed AGI is short by exactly the $1,800 taxable distribution; computed taxable income by the
same; computed total tax by $576.

And it reaches a filed cell. Second probe — the same household at $85,000 wages, $2,500 of Form
1098-E student-loan interest, and a $9,000 HSA distribution fully meeting an exception (so the 20%
tax is out of the picture and only the AGI term moves):

```
PROBE2 total_income=85000 agi=81333 s1_line8f=9000 s1_line21_FILED=1667
PROBE2 true_magi=94000 correct_line21=167
```

The **printed Schedule 1 line 21** carries $1,667 where §221(b)(2) on the MAGI the return actually
has ($94,000, inside the TY2024 $80,000–$95,000 unmarried range) gives $167. A $1,500 overstated
above-the-line deduction on a signed page — an understatement of tax, produced by a Form 8889 the
build fills correctly.

**Minimal change.** Hoist the `form_8889` / `hsa_income_8f` derivation above `schedule_1_income`
(it needs only `ri` and `params.hsa`, and `screen_inputs` already calls `form8889::compute` with
nothing more), then add `+ hsa_income_8f` to `schedule_1_income`. Hold it with a test that asserts
`round_dollar(ar.agi) == printed f1040.line11` on an HSA-distribution household — the assertion the
existing reach test stops one line short of.

---

### I-1 (Important) — `AbsoluteReturn::total_tax` omits the §223(f)(4) 20% and §223(b)(8) 10% taxes, and the guarantee test written for exactly this shape is blind to it

**Where.** `crates/btctax-core/src/tax/return_1040.rs:2473-2475`; the instrument at
`crates/btctax-core/src/tax/packet.rs:1455`.

**What is wrong.** `schedule_2_other_taxes = se_tax_sch2_l4 + additional_medicare.additional_medicare_tax
+ niit.tax`. Form 8889 line 17b → Schedule 2 line 17c and line 21 → 17d are not in the sum, so
`total_tax` (and `amount_owed` / `overpayment_refund`, which `advisories_for` surfaces to both
`report` and `export`) is short by the whole HSA additional tax. The printed chain is correct
(`printed.rs:452-456` sums 17c/17d into line 18, line 21, 1040 line 23), so the two chains disagree.

This is the same defect, in the same function, that the doc comment 40 lines above it records as
already having happened once: *"`l18` was hardcoded to `regular_tax` … `AbsoluteReturn::total_tax`
short by exactly the AMT — an UNDERSTATEMENT. It was invisible…"*. The comparison written in
response — `the_absolute_total_tax_equals_the_printed_1040_line_24`, whose own doc says *"A figure
with no reader is not thereby correct — it is a wrong number waiting for its first caller"* — loops
over exactly two fixtures, `kitchen_sink_household` and `amt_owing_household`. Neither sets
`hsa_activity` (`grep -n hsa crates/btctax-core/src/tax/packet.rs` → one hit, and it is a *different*
test's `ri.sch1.hsa_activity = None`). T16 added a new term to 1040 line 24 and did not extend the
one test that exists to catch a new term going missing: it passes, and the guarantee it states is
now false for every HSA return with a Part II or Part III figure.

**Evidence.** Same probe as C-1: `ar.total_tax = 4979`, `pr.f1040.line24 = 5555`. That is precisely
what `assert_eq!(round_dollar(ar.total_tax), pr.forms.f1040.line24)` would report — on a household
the test's fixture list does not contain.

**Minimal change.** Add the Form 8889 legs to `schedule_2_other_taxes` (the printed lane already
shows the shape), and add an HSA household to the `the_absolute_total_tax_equals_the_printed_1040_line_24`
loop with the same *"fixture must exercise X"* anti-vacuity guard the AMT row carries.

Graded Important rather than Critical because no printed cell moves *today* from this term alone —
but note the repo's own §G-6 precedent treated the identical shape as Critical, and a folder may
reasonably raise it.

---

### I-2 (Important) — the import tier refuses a code-W W-2 before the HSA question has been asked, with a message that asserts something false

**Where.** `crates/btctax-core/src/tax/return_refuse.rs:2374`, inside the unconditional W-2 loop —
i.e. outside the `tier.unanswered_refuses` gate.

**What is wrong.** The contradiction rule reads `ri.sch1.hsa_activity != Some(true)`, which is true
for `None` as well as `Some(false)`. `screen_param_free` — the gate `income import` runs, whose whole
premise (`:1807-1809`) is *"the import is the only row-creating path and `income answer` the only
answering one, so demanding the answers here would make answering unreachable"* — passes
`unanswered_refuses: false` precisely so an unanswered declaration is lawful there. So an ordinary
HSA filer whose Form W-2 carries box 12 code W cannot import their W-2 until the TOML also carries
`sch1.hsa_activity = true`, and the refusal tells them something untrue about their own file.

**Evidence.** Probe calling `screen_param_free` on the build's own `hsa_household()` with
`hsa_activity = None`:

```
PROBE3 import-tier refusal = Some(HsaEmployerContributionWithoutActivity)
PROBE3 detail = a Form W-2 reports $1000 in box 12 code W — "Employer contributions to your Health
  Savings Account" — but this return says no health savings account activity happened. …
  Answer the HSA question Yes and complete Form 8889, or correct the box 12 entry
```

The return says nothing of the kind — nobody has been asked. `cmd/tax.rs:272` turns this into
`CliError::Usage("the {year} inputs were NOT stored …")`, and the remedy the message offers second
("correct the box 12 entry") invites the filer to delete a true entry off an information return.

**Minimal change.** `ri.sch1.hsa_activity == Some(false)`. `None` is already covered, on every tier
that answers, by the `FORM_QUESTIONS` registry loop (`HsaActivityUnanswered`, `live: |_| true`), so
the rule loses nothing and the contradiction it names stays exactly as sharp.

---

### I-3 (Important) — line 1 and line 3 are asked about the filer's own plan; the instructions ask about "you or your spouse"

**Where.** `crates/btctax-core/src/tax/questions.rs:1874-1892` (the `HsaFamilyCoverage` prompt);
`crates/btctax-core/src/tax/form8889.rs:285-289` and `:220-238`.

**What is wrong.** `compute` derives both the line-1 checkbox and the line-3 base from one boolean
about the filer's own coverage. The instructions do not:

- i8889 **Line 1** (`i8889--2024.txt:460-472`): *"If you and your spouse are considered covered by a
  family HDHP, you are considered covered by a family HDHP **regardless of whether you file jointly
  or separately**."*
- i8889 **Line 3 rule 1** (`:496-499`): *"Use the family coverage amount **if you or your spouse**
  had an HDHP with family coverage. Disregard any plan with self-only coverage."*
- i8889 **Line 7** (`:722-724`): *"**You or your spouse** had family coverage under an HDHP…"*

The prompt quotes the *other* two sentences of Line 1 (both-at-different-times, both-at-the-same-time)
and omits the spouse sentence, and no question asks about the spouse's plan at all. A married filer
with self-only HDHP coverage whose spouse has family coverage answers "No" truthfully about their own
plan and gets the **wrong line-1 box** and **$4,150 on line 3 where the instructions say $8,300**.

The direction is conservative for the deduction, but the consequence is not benign: if that filer
contributed, say, $6,000, `line13 = min(line2, line12)` makes line 2 > line 13 and the return is
refused `HsaExcessContributionsNeedForm5329` — telling a fully compliant filer that they have excess
contributions and may owe the §4973 excise tax. (Verified by reading; no code change needed to see
it — `screen_form_8889`'s excess rule at `return_refuse.rs:2687` compares the same two lines.)

CLAUDE.md's own corollary applies verbatim: *"If the form asks something our input surface cannot
answer, collect it. That is following instructions, not scope creep."*

**Minimal change.** Either widen `HsaFamilyCoverage`'s prompt to the instruction's own words (*"…or
your spouse…"* plus the regardless-of-filing-status sentence), or add the spouse's-plan declaration
and OR it into `coverage`. Hold it with a fixture: MFJ, taxpayer self-only, spouse family, `line3 ==
family_limit`.

---

### M-1 (Minor) — no channel for an HSA distribution with no Form 1099-SA

`form8889::total_hsa_distributions` (`form8889.rs:246-254`) is the only source of line 14a, and it
sums transcribed `sa_1099` rows. `hsa_activity` is a four-way disjunction, so a Yes does not say
*which* trigger fired; a filer whose only trigger is *"(b) you took money out of one"* and who
answers the `sa_1099` census row "no" produces line 14a = $0 with no refusal and no advisory. That is
the shape R3 already handles for wages/interest/state refunds with its three document-less income
questions (`w2_wages_without_w2` and friends). Risk is lower here — a trustee must issue a 1099-SA for
every distribution — which is why this is Minor rather than blocking, but the asymmetry with R3 is
worth a follow-up rather than silence.

### M-2 (Minor) — `line-coverage` verifies the TY2025 revision against the TY2024 extract only

`cover_form8889` opens with `Coverage::quoting("2024")` and one struct serves both editions, so the
2025 line-13 text (which differs by a trailing period — measured) and any future TY2025 drift are
unchecked. Harmless today (`full_return_for(2025)` is `None`), but the guard the build relies on to
say *"one struct serves both years"* does not itself watch the second year.

### N-1 (Nit) — the line-9 census box label is narrower than the code

`Production::doc_box("fw2", "12a")` names slot 12a, while `employer_contributions_from_w2s` correctly
sums every box-12 slot whose code is `W` (12a–12d). The label reads as if only the first slot were
consulted.

---

## Seams checked clean

1. **Transcription fidelity.** All 21 printed lines present, named for the line, doc comments
   verbatim against `f8889--2024.txt` (walked line by line). Every arithmetic line matches the
   printed sentence: `line5`/`line12`/`line16` floor at zero exactly where the form says *"If zero
   or less, enter -0-"* and nowhere else; `line8 = line6 + line7`; `line11 = line9 + line10`;
   `line14c = line14a - line14b` (no floor — the form prints none); `line13 = line2.min(line12)`,
   which is i8889's *"Generally, enter the smaller of line 2 or line 12"* with its *"However…"*
   refusing upstream; `line17b = 0.20 × (line16 − excepted)`, which is i8889's *"only 20% … of any
   amount included on line 16 that does not meet any of the exceptions"*; `line21 = 0.10 × line20`.
   The §223(b)(3)(B) line-3/line-7 split matches i8889 item (6) and its Note, and the flat $1,000 (not
   the Additional Contribution Amount Worksheet's proration) is right because the only path to line 7
   requires eligibility every month and no Medicare — the instruction's own TIP. Line 6's allocation
   sentence applies only to spouses with separate HSAs, which refuses. 2025 edition: normalised diff
   of the two extracts is two lines (a period, the footer); `label-boxes` grids identical (per the map
   header, and the `form_8889_ty2025_fills_the_identical_cells` test).
2. **Reach — the printed lane.** Schedule 1 line 8f = line 16 + line 20 (both routed by the form's
   own text, and the sum is pinned on a directly-constructed struct because `compute` cannot reach a
   non-zero Part III); Schedule 1 line 13 = line 13; Schedule 2 17c = line 17b, 17d = line 21, line 18
   sums them, line 21 carries them to 1040 line 23. Map cells verified against `f1040s1--2024`/
   `f1040s2--2024` captions; `census-join` fell to 290 with the eight retired entries. *(The exact
   lane is C-1/I-1.)*
3. **The documents and the census.** 1099-SA (Rev. 11-2019 and Rev. 4-2025) and 5498-SA (2024, 2025)
   censused complete, 268/19/9 with every box decided; the annual 5498-SA's boxes 2 and 3 split per
   edition on their year-bearing captions, which is the property the per-edition census exists for.
   Box 5 / box 6 `None` refuses `SaAccountTypeNotTranscribed` rather than defaulting to HSA; a
   non-HSA type refuses `ArcherOrMaMsaNeedsForm8853` naming the form; the contradiction refuses both
   directions. **Code W traced end to end** (question b): W-2 box 12 code W → `employer_contributions_from_w2s`
   → worksheet W1 → W5 → line 9 → line 11 → line 12. It cannot double-count with line 2 (a separate
   filer field whose instruction excludes employer, cafeteria-plan and rollover amounts) or with the
   5498-SA (`grep` confirms no money line reads `ri.sa_5498`; `requires_transcription(Sa5498)` is
   `false` for the reason the census note states), and box-12 amounts are outside W-2 box 1 wages.
4. **The year's figures.** `self_only_limit`/`family_limit` = 4150/8300 (TY2024) and 4400/8750
   (TY2026) read out of `legal/text/irs-guidance/RevProc_2023-23.txt:17-22` and
   `RevProc_2025-19.txt:17-22`; TY2025's 4300/8550 archived (`RevProc_2024-25.txt:17-22`) with no
   params row, consistent with `full_return_for(2025) == None`. `additional_contribution_55` = $1,000,
   §223(b)(3)(B), correctly not indexed. `$4,150` is also what `f8889--2024.txt:22` prints. Shipped vs
   independently-transcribed compared by `shipped_tables_are_the_validated_tables.rs:709`. **No silent
   clamp**: over-limit refuses naming Form 5329 (and the excess-employer arm reduces line 8 by line 10
   first, exactly as i8889's *Excess Employer Contributions* says).
5. **The oracles** (question e). Both engines run live and reproduce the baked cell exactly (numbers
   above). `e03290` and `S1_13` are the right variables. `build_golden_return` feeds btctax the
   *contribution* and lets line 13 produce the deduction, so the cell is discriminating.
   `every_golden_household_matches_the_independent_oracles` 7/7. The cell is honest about its own
   limit — it witnesses the deduction leg only, which is why it cannot see C-1.
6. **Refusals and the packet.** `Form8889Unanswered` covers all seven questions, each `live` iff
   `hsa_activity == Some(true)`, each `None` refused by the single `FORM_QUESTIONS` loop — so
   (question a) `family_coverage: None` is blocking wherever it is live, and the `else` arm in
   `compute` picks `SelfOnly`, the lower limit, the direction that cannot overstate a deduction.
   (Question c) the Part III refusal quotes the form's own rule — *"the LINE 3 LIMITATION CHART AND
   WORKSHEET FOR THE YEAR THE CONTRIBUTION WAS MADE"* — and lines 18–21's reaches are pinned by
   `part_iii_reaches_schedule_1_line_8f_and_schedule_2_line_17d` on a directly-built struct, exactly
   as the report says. (Question d) the only two paths to `HsaLine3WorksheetRequired` are
   `eligible_every_month_same_coverage == Some(false)` and `enrolled_in_medicare_any_month ==
   Some(true)`; a full-year-eligible, non-Medicare filer meets neither, and the prompt correctly tells
   the last-month-rule filer that *"considered"* counts (i8889 line 3 rule 2). Form 8853, Form 5329,
   the second spouse's Form 8889 and the prior year's worksheet each refuse by name. Attachment
   sequence **52** read off `f8889--2024.txt:8` and held by `packet_sequences_agree_with_every_map_row`;
   `YEAR.toml` 2024/2025 list `f8889`, 2026 carries a `forms_absent` reason. **A return without an HSA
   is untouched**: `schedule23.rs:57,77` skips the whole 17c/17d/18 block when `line17c.is_none()`,
   the golden-packet diff shards and `the_whole_packet_is_byte_reproducible` pass, and the
   `full_return_goldens.json` diff is purely additive (one household, none changed).
7. **The interruption (§0).** Nothing half-landed: the tree at `4b241dd7` is clean, `grep -rn "PLANT:"
   crates/` finds only the pre-existing `cite_check.rs` hit, and the eight deviations and three
   follow-ups the report lists were each checked against the source — all eight are as described,
   with deviation 5 (question a) sound and deviation 6 (question b) sound *except* for the `None`
   arm, which is I-2.

Counts: C=1 I=3 M=2 N=1
