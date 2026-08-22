# AMT / Form 6251 — readiness for the $1M/$1M/$1M MFJ household

## VERDICT — can this slice FILE today?

**Files clean — and the brief's premise about this vector is wrong.** Form 6251 is transcribed line by
line (`crates/btctax-core/src/tax/form6251.rs`, 41 numbered fields, official instruction text as doc
comments), the AcroForm map covers all 41 modelled lines plus a census of the 18 it does not
(`crates/btctax-forms/forms/2024/f6251.map.toml`), the emitter honours the form's own Part III gate,
and the packet staples the form exactly when i6251's *Who Must File* condition 1 holds
(`crates/btctax-core/src/tax/packet.rs:653`). Nothing in this slice refuses, understates, or files a
laundered figure for the stated household.

**But at $1M ordinary + $1M LTCG + a $1M gift, the §55(d) exemption is NOT phased out and Part III does
not decide anything — because the $1M charitable deduction is allowed for the AMT too.** AMTI is
taxable income *plus the $10,000 SALT add-back*, so the gift drags AMTI down in lockstep with taxable
income. Machine-checked by running the repo's own line-by-line reference
(`design/amt-form6251/f6251_reference.py`, the file that generated the 30 committed fixture vectors,
pinned equal to the Rust by `form6251.rs::every_vector_reproduces_the_form_line_by_line`), TY2024 MFJ
constants (TY2024 is the only year `full_return_for` bundles —
`crates/btctax-adapters/src/tax_tables.rs:101`):

| branch | itemized | L1 (=TI) | L2a | L4 (AMTI) | L5 exemption | L7 | L10 | attach? | **AMT (L11)** |
|---|---|---:|---:|---:|---:|---:|---:|---|---:|
| **A** — $1M **cash** gift (60%-of-AGI limit = $1.2M, full), $30k mortgage | 1,040,000 | 960,000 | 10,000 | 970,000 | **133,300 (intact)** | 124,045 | 148,705 | **no** | **0** |
| **B** — $1M **appreciated bitcoin** gift (30%-of-AGI limit ⇒ $600k allowed, $400k carryover), $30k mortgage | 640,000 | 1,360,000 | 10,000 | 1,370,000 | 95,475 (partial) | 261,027.50 | 261,297.50 | **no, by $270** | **0** |
| **B′** — same, **no** mortgage interest | 610,000 | 1,390,000 | 10,000 | 1,400,000 | 87,975 | 273,027.50 | 270,485.50 | **yes** | **2,542** |
| **C** — gift allowed only $200k (sensitivity) | 210,000 | 1,790,000 | 10,000 | 1,800,000 | **0 (fully phased out)** | 419,348 | 418,425.50 | yes | 922.50 |

Read the table as the finding: **AMT is owed only in a middle band of allowed charitable deduction
(~$150k–$620k), and the household as stated sits on the edge of it.** In branch B the attach test misses
by $270, and the crossover in mortgage interest is **≈ $27,500** (scanned at $1,000 steps: attach at
$27,000, no attach at $28,000). Whichever way it lands, btctax computes it correctly and files the
right thing — Schedule 2 line 2 is `Option<Usd>` and stays **blank** when no Form 6251 is attached
(`crates/btctax-core/src/tax/line_coverage.rs:1546-1553`), so a no-AMT return swears nothing about an
AMT it did not figure.

The gates this household must still pass are collected, not missing: it is a homeowner with 1098
interest, so `QuestionId::MortgageAllUsedToBuyBuildImprove` and `QuestionId::AmtQualifiedDwelling` are
both live (`crates/btctax-core/src/tax/questions.rs:374-427`, liveness `mortgage_question_live` at
:244) and refuse if unanswered; the always-live §G-22 scope attestation (:513-570) refuses on a `yes`
(`crates/btctax-core/src/tax/return_refuse.rs:775-786`). All three are answerable with
`btctax income answer`.

## WHAT IS MISSING

**1. The AMT answer for this household is decided by a figure the AMT slice does not own — and the
margin is $270.**
Form 6251 line 1 is 1040 line 15 (`form6251.rs:458-465`), so the §170(b) limit branch — 60% of AGI for
cash vs 30% for capital-gain property — moves AMTI dollar for dollar. Consequence: an error of more
than ~$1,000 in the charitable lens's *allowed* deduction, or a mortgage-interest figure ~$500 off,
**flips whether a Form 6251 is stapled to the return at all**. Filing without one when line 7 > line 10
is an incomplete return under i6251 p.1 condition 1; stapling one when it is not required prints 41
lines of unnecessary testimony. This is not a Form 6251 defect — it is a hard dependency the plan must
sequence: the AMT slice cannot be signed off before the charitable-limit slice is.

**2. Schedule A line 6 ("Other taxes") is not modelled, so Form 6251 line 2a is line 5e only.**
The form says line 2a = "the taxes from **Schedule A, line 7**", and line 7 = 5e + 6.
`return_1040.rs:2056` passes `p.salt_5e`; `ScheduleAParts` (`return_1040.rs:325-372`) has no line-6
field, and `amt.rs`'s helper doc states the substitution explicitly. Consequence: a filer with line-6
taxes (e.g. foreign income taxes taken as a deduction) gets an **understated AMT add-back ⇒ understated
AMT**. Not live for this household — there is no input to carry such a tax — but it is the ceiling on
line 2a's correctness and it is invisible because the omission is at the *input* surface, not the form.

**3. Lines 2c–2t have no fields; two of the eighteen are plausible for THIS household.**
`f6251.map.toml:147-164` censuses all 18 as `unmodeled`, and `form6251.rs:485` computes
`line4 = line1 + line2a + line2b + line3`, treating every one as $0. For a $1,000,000-wage MFJ filer
the live candidates are **2c "Investment interest expense (difference between regular tax and AMT)"**
(Form 4952 is absent from btctax entirely) and **2i "Exercise of incentive stock options"** — the
dominant post-TCJA AMT trigger, in exactly this income band. Consequence today is **refusal, not
understatement**: the §G-22 attestation names an ISO exercise (limb b) and "another AMT item" (limb c),
and a `yes` refuses (`return_refuse.rs:775`). The residual risk is a filer who answers `no` while
holding one — then the add-back prints as nothing, and because `must_attach()` is `line7 > line10`, the
missing add-back also suppresses its own detection (FOLLOWUPS §G-6). That is E6, and it is a *serving*
gap, not a correctness gap.

**4. G-6d — the exact limb this household drives has no two-oracle witness.**
FOLLOWUPS §G-6d, verbatim: *"no fixture vector has Schedule A line 7 > 0. Every itemizing vector
deducts a cash gift only, so the fixture drives line 2a's itemizer limb at zero throughout."* This
household's **entire AMT preference is the $10,000 SALT add-back at line 2a** — the one number in its
Form 6251 that is not just taxable income re-stated. It is covered by a unit KAT
(`amt::tests::itemizer_addback_is_schedule_a_line7_not_the_itemized_total`) and by nothing else.
Consequence: if this vector lands in the AMT-owing branch, the figure filed on a §6065-signed form
rests on our own transcription with **no independent engine ever having scored a household of this
shape**. G-6d's owning phase is Tier-2 · E4, and this vector *is* the missing household.

**5. E5 — the double-oracle corpus structurally cannot contain this household in its AMT branch.**
`scripts/oracle/gen_goldens.py:314` and `scripts/oracle/sweep.py:370` reject any candidate taxcalc sees
AMT on (D-2). Consequence: in branch B′/C the end-to-end packet (6251 → Sch 2 L2 → Sch 2 L3 → 1040 L17
→ L24) is validated by **zero** oracles; only `scripts/oracle/verify_f6251.py`'s 30 hand-built vectors
validate the form in isolation, and none of them has Schedule A line 7 > 0 (see 4).

**6. E4 — no test compares the emitted f6251 PDF's values, line by line, against the computed struct.**
`crates/btctax-forms/tests/f6251_fill.rs:192-233` reads all 41 cells back from the serialized PDF but
asserts only that each is a whole dollar; the per-line value assertions
(`part_iii_prints_in_full_when_line_7_routes_there`, :123) cover three lines on a synthetic struct.
Placement is well corroborated — `verify_flat` enforces monotone y-descent per page, and
`tests/f6251_map.rs` checks the inset trio, the per-page counts and every quoted instruction verbatim —
but the assignment has never been checked against the form's own **labels** (the instrument for that,
`design/forms/LABEL_READER.md`, exists). Consequence: a systematic map offset of the kind the map's own
header warns about (`f1_N = line N−2`) would file a Form 6251 with every figure one line out and
nothing would red. Also unclosed: the Σround/roundΣ residual rule on the 6251 → Sch 2 → 1040 L17 chain
that E4 names.

**7. Minor, §G-11 instances specific to this form.** When the form *is* attached, `line8` prints `0`
whenever the filer has no FTC — i6251 p.10 says *"Leave line 8 blank"* in the adjacent case, so the
form itself distinguishes blank from `-0-`. Likewise, in the fully-phased-out branch (C) the Exemption
Worksheet's Note says *"Don't complete this worksheet; instead, enter the amount from Form 6251, line
4, on line 6"* — btctax prints `line5 = 0`. Neither moves a dollar; both are laundered zeros on a
signed page, and both are downstream of the known `fmt_money(Usd) -> String` limit.

**8. Doctrine note, no consequence found.** The §55(d)(3) exemption phase-out is a **closed form**
(`form6251.rs:495`), not a transcription of the six-line Exemption Worksheet. I checked it against the
worksheet in `design/forms/extract/i6251--2024.txt:1168-1198` line for line — `max(0, exemption − 25% ×
max(0, L4 − threshold))` is exactly worksheet lines 1–6, and the `max(0, ·)` reproduces the Note's
zero-exemption cliff for all four statuses. There is a KAT holding the rate split
(`the_exemption_phaseout_and_the_mfs_kicker_use_their_own_rates`). Recording it because doctrine says
worksheets get transcribed, not because I found a branch where it breaks.

## THE SMALLEST THING THAT CLOSES IT

Sequenced; nothing here changes `form6251.rs`'s arithmetic.

1. **Settle the charitable branch first** (§170(b): cash 60% vs capital-gain property 30%, and whether
   the gift is bitcoin). Until that number is fixed, the AMT slice has two different answers, $270
   apart. No AMT work should be scheduled before it. *(Dependency, not a task for this lens.)*
2. **Add G-6d's missing vector — the household in this plan.** One entry in
   `design/amt-form6251/gen_e2_vectors.py` → `crates/btctax-core/src/tax/fixtures/form6251_vectors.json`,
   MFJ, itemizing, **Schedule A line 7 = $10,000**, large charitable deduction, LTCG > taxable income
   (so QDCGT worksheet line 5 = 0 and Part III's lines 20/27 are zero — a routing no current vector
   exercises either). Wire the SALT into both oracles in `scripts/oracle/verify_f6251.py`: `A5a`/`A5b`
   for OTS, `e18400`/`e18500` for taxcalc. This is the smallest thing that gives line 2a's itemizer
   limb an independent witness, and it closes G-6d. **Attaches to:** existing `Form6251Inputs.schedule_a_line7`
   — no new field.
3. **E4, in two assertions.** In `crates/btctax-forms/tests/f6251_fill.rs`: (a) drive
   `fill_form_6251_with_map` from a **real assembled household** (branch B′ above) and assert every one
   of the 41 cells equals the corresponding `Form6251::printed()` field — the existing `tv()` helper
   already reads values back, so this is a loop, not new machinery; (b) assert the identity
   `f6251.printed().line11 == Sch2.line2 == Sch2.line3 == 1040.line17` on that same household, which is
   the Σround/roundΣ half E4 names. Plant the defect first (swap two adjacent map cells) and watch it
   red — B1.
4. **E5, one predicate.** Lift D-2 for **itemizing** AMT households only in
   `scripts/oracle/gen_goldens.py:314` / `scripts/oracle/sweep.py:370`, keeping taxcalc's
   standard-deduction AMT defect (#3108) as a `KnownDefect` class rather than an exclusion — FOLLOWUPS
   §G-6b already specifies this shape, and the itemizing slice is precisely where taxcalc is a valid
   second oracle. This household is an itemizer, so it becomes corpus-admissible.
5. **Collect nothing new for the AMT itself.** The three declarations this household needs are already
   in `FORM_QUESTIONS` and answerable via `btctax income answer`. The one genuinely *missing* collection
   in this slice is Form 4952 investment interest — it is both a Schedule A line 9 deduction the filer
   forgoes and Form 6251 line 2c — but the household as stated has none, so it is out of scope here.

**Which of E4/E5/E6 this vector actually needs:**

- **E4 — NEEDED**, in the branch where the form attaches (B′/C). A perfect computation still files a
  wrong number through a mis-assigned AcroForm field, and today's read-back checks whole-dollarness and
  geometry, not per-line values.
- **E5 — NEEDED**, same branch, and this household is the archetype E5 names ("lift D-2 for *itemizing*
  AMT households"). In branch A/B it is moot: no Form 6251 is filed, and the return's AMT content is
  a blank Schedule 2 line 2.
- **E6 — NOT NEEDED to file this vector as stated.** The household has no ISO, no §1202, no depletion,
  no passive activity, no depreciation (no Schedule C), no capital-loss carryover (so line 2k's
  question is not even live) and no §170(e) basis difference (bitcoin's AMT basis ≡ its regular basis).
  Every 2c–2t line is genuinely zero here, and the §G-22 attestation is the standing gate for the case
  where it is not. E6 becomes required only if the plan intends to **serve** an equity-comp filer rather
  than refuse one.
- **G-6d — NEEDED, and this vector is its missing household** (see step 2). It is filed under E4's
  owning phase.
- **G-6c** (upstream taxcalc MFS report) — **not needed**: MFS-only, and this household is MFJ.
- **G-6e** (TY2025) — **not needed**: `full_return_for` bundles TY2024 only, so this is a TY2024 return
  and the TY2024 constants are the ones on the form.

## WHAT I AM NOT SURE OF

- **Whether the gift is cash or appreciated bitcoin.** It decides the whole slice (branch A vs B) and
  the brief does not say. I computed both rather than pick.
- **The mortgage-interest amount.** I assumed $30,000; the attach crossover in branch B is ≈ $27,500,
  so this single unstated number decides whether a Form 6251 exists. Whoever owns the itemized slice
  should hand the AMT slice a fixed figure.
- **Whether `charitable.rs` actually applies the 30%-of-AGI capital-gain-property limit and rolls the
  $400k carryover.** I did not read it — another lens owns it — but branch B's entire arithmetic
  assumes it does.
- **I ran the reference transcription, not the Rust.** `design/amt-form6251/f6251_reference.py` and
  `compute_6251` are pinned equal on 30 fixture vectors by
  `every_vector_reproduces_the_form_line_by_line`, and both take `sch_a_line7` on the itemizer limb, so
  the substitution is sound — but I did not construct a `ReturnInputs` and drive the real engine on
  this household. If the plan wants the table above to be load-bearing, that is a 20-line integration
  test.
- **Whether Schedule A line 6 could be nonzero for this household** (it would change line 2a). No input
  exists to carry it, so I could not test it; I do not know whether the interview should start asking.
- **Whether `verify_flat`'s monotone-descent check is strong enough to exclude a whole-form map
  offset.** I believe it is not (a uniform shift preserves descent), but I did not plant the defect and
  watch it, so treat that as a hypothesis, not a finding.
- **Whether the §55(d)(3) closed form has any live divergent branch** beyond the four filing statuses I
  checked against the worksheet text. I found none; i6251's Form 1040-NR chart uses the same worksheet,
  and the kiddie-tax population refuses upstream (`RefuseReason::KiddieTax`).
