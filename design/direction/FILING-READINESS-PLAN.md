# PLAN — filing readiness across the range, generalized from the $1M/$1M/$1M vector

Synthesis agent, revised by the generalization pass. Repo read and **executed** at `main @ f5ba41a1`
(clean; nothing in the repo modified except this file). Two generations of evidence:

- **[RAN]** — reproduced by the original single-vector pass (v0.17.0, scratch directory).
- **[RAN 08-21]** — reproduced by this pass: seven new low-end households driven through
  `target/debug/btctax` v0.17.0 in `/tmp` scratch vaults (L0 all-zero · L1 $40k wages/no crypto ·
  L2 $0 wages + $30k LT gain · L3 $40k + $20k LT loss · L4 $0 wages + $20k LT loss · L5 MFJ $40k +
  2 children · L7 $0 AGI + $2,000 cash gift), read back off the emitted AcroForms with qpdf, and
  cross-checked against Tax-Calculator (`.venv/bin/python`, claiming-behavior model disabled via
  `eitc_claim_prob_scale: 9e99` — its default take-up simulation zeroes the EITC stochastically and
  must be off for oracle use).

---

## REVISION — what changed versus the single-vector version, and why

The previous version of this plan was calibrated to exactly one household: MFJ, $1,000,000 wages,
$1,000,000 long-term bitcoin gain, $1,000,000 church gift, five children, jumbo-mortgage itemizer.
Seven lenses and an execution pass all examined that one rich corner. This revision re-derives the
plan across three axes independently, each including its zero — INCOME ($0 / $40k / $120k / $1M),
CAPITAL GAINS ($0–no-crypto / small / large / a LOSS), DONATIONS ($0 / <$250 / $250–$5k / >$5k /
>$500k) — by running the cells nobody had looked at. Concretely:

- **Two items are ADDED from the low end**, both invisible from the rich vector:
  **N1** — the §1212(b)(2)(B) Capital Loss Carryover Worksheet is not applied on the carryforward-**out**
  side, so a loss year with taxable income at the floor prints a carryforward understated by up to the
  whole §1211(b) allowance ($17,000 where the worksheet says $20,000 — **[RAN 08-21]**), and
  **N2** — below the §24(b)/§32 phase-outs the emitted packet forfeits EITC + CTC/ACTC worth ~22% of a
  modest family's income ($8,781 on MFJ/$40k/2 kids — **[RAN 08-21]**, oracle-verified), which
  RESCINDS the single-vector conclusion "do not build Schedule 8812" as a *general* conclusion (it
  remains true at the rich vector, where both oracles put the credit at $0).
  Two smaller additions: **N3** (1040 line 19 prints an unconditional `0` that is fabricated
  testimony exactly for the families N2 describes) and **N4** (the no-crypto packet leaves two
  mandatory hand-marks — the digital-asset question and the line-7 checkbox — with no signal to the
  filer).
- **Three items are SCOPED as rich-band artifacts** — real defects whose population lives at the top
  of an axis: **P1** (binds only above $750,000 of acquisition debt), **P5** (binds only above
  $500,000 claimed), **P8** (binds only above the $200k/$250k §1411 MAGI floor). None is deleted;
  each now states where it stops mattering.
- **Two items are STRENGTHENED by low-end evidence**: **P2** (8949 overflow is lot-count-driven, not
  dollar-driven — a modest DCA accumulator is *more* exposed than a single-lot whale) and **P6** (the
  charitable carryover-out is invisible at $0 AGI too, where 100% of the gift carries —
  **[RAN 08-21]**).
- **The ordering principle is kept**; one rank inversion follows from it (N1 enters the
  silent-wrong tier directly behind P1), and the rest of the movement is scoping, not re-tiering.
- **A COVERAGE MATRIX section is added** giving a verdict for every cell of the three axes, including
  the cells that are **fine** — the standard-deduction path, the 0% capital-gains band, the $0-income
  return, and the plainest wage-earner case all file correctly, and saying so with evidence is part
  of the answer.

---

## EXECUTIVE ANSWER

**The low end mostly files, and files correctly — the product's silent failures are concentrated at
the top of each axis, with two exceptions that only the bottom reveals.** The plainest case in the
world — a $40k wage earner with no crypto, no gains, no gifts — produces a correct one-form packet
with the standard deduction and exits 0 **[RAN 08-21]**. A filer with $0 income and a $30k long-term
gain gets the 0% band and $0 tax, byte-identical with Tax-Calculator **[RAN 08-21]**. A $20k loss
against wages is capped at −$3,000 with the right character carryforward **[RAN 08-21]**. The
all-zero return files. None of this was witnessed by the rich vector, and all of it holds.

The two genuine low-end breaks: **(1)** a loss year whose taxable income would be negative prints a
carryforward-out of `loss − $3,000` where the §1212(b)(2)(B) worksheet says the $3,000 was never
absorbed — the filer's surviving loss is understated by up to $3,000, silently, and next year's M4
consistency check will actively dispute the *correct* number if the filer computes it themselves
(N1); **(2)** a modest family's packet is honest but forfeits the refundable credits that dominate
its return — $8,781 of EITC+CTC/ACTC on $40k of income, disclosed in two advisories and then left
entirely to the filer, with 1040 line 19 swearing `0` on the signed page (N2/N3). The first is a
wrong figure failing silently; the second is a product-scope decision this plan can only surface,
with the boundary numbers, for the owner.

Everything else in the original plan survives with its severity intact *inside its population*:
the §163(h)(3)(B) understatement, the 8949 overflow, the Form 8283 fabrications, the §170(f)(8)
gap, the carryover-out that reaches no human. What the generalization changes is the *population
statement* attached to each — which is what an ordering has to weigh.

---

## COVERAGE MATRIX — three axes, each cell executed or witnessed

"Witnessed" = in the 104-household double-oracle corpus (`scripts/oracle/corpus.py` — W-2 values
span $0–$300,000 with 5 zero-wage-with-gain rows and 5 §1211(b) loss rows **[RAN 08-21]**, measured,
correcting my own first read of it). "Executed" = driven through the binary by a pass of this plan.

### INCOME (other axes at zero)

| cell | verdict | evidence |
|---|---|---|
| **$0** | **fine** — all-zero 1040 emits, standard deduction, tax $0, exit 0. Nit: no "you may have no filing requirement" note (→ N4). | **[RAN 08-21]** L0 |
| **$40k wages** | **fine** — 1-form packet, standard deduction, $2,819 vs taxcalc $2,816 (Tax-Table vs formula, known methodology split). Nits: EIC advisory over-fires (below), two hand-marks unsignalled (→ N4). | **[RAN 08-21]** L1 + taxcalc |
| **$120k** | **fine** — interpolation; corpus mid leg ($105k) is two-oracle witnessed across interest/dividend/gain shapes. | corpus |
| **$1M** | the original plan's territory (P1, P8 live here). | **[RAN]** |

### CAPITAL GAINS (independent of income)

| cell | verdict | evidence |
|---|---|---|
| **$0 — no crypto at all** | **fine** — Schedule D correctly absent (`must_file` false), line 7 prints `0`, no-Sch-D checkbox honestly blank (censused, `f1040.map.toml:293`), digital-asset question honestly blank. Both blanks are *unsignalled* (→ N4). | **[RAN 08-21]** L1 read-back |
| **small gain** | **fine** — $30k LT gain at $0 income lands in the 0% band, tax $0, both engines agree; corpus small-gain rows witness low/mid/high income. | **[RAN 08-21]** L2 + taxcalc; corpus |
| **large gain** | P2 (overflow) is the blocker and it is **lot-count-driven, not dollar-driven** — the >14-leg failure hits a $5k DCA seller and a $1M whale identically. | **[RAN]** 16 legs → exit 2 |
| **a LOSS, income positive** | **fine** — §1211(b) −$3,000 cap, leading-minus line 7, Sch D line 21 = 3000, carryforward 17,000 correct (TI positive ⇒ flat rule exact). | **[RAN 08-21]** L3 read-back |
| **a LOSS, income at the floor** | **BROKEN — N1.** Carryforward-out printed 17,000; the worksheet gives 20,000. The IN-side twin of this edge already refuses (`return_1040.rs:2342-2350`); the OUT side silently misprints. | **[RAN 08-21]** L4 |

### DONATIONS (independent of income)

| cell | verdict | evidence |
|---|---|---|
| **$0** | **fine** — nothing fires, nothing prints. | **[RAN 08-21]** L0/L1 |
| **< $250** | **fine** — no §170(f)(8) CWA required at this size; at low income the standard deduction absorbs it and no Schedule A files (correct: TY2024 has no non-itemizer charitable line). | law + L7 shape |
| **$250–$5,000** | P4 (CWA never asked) — but **only for itemizers**: a standard-deduction filer claims no deduction, so §170(f)(8) is moot. Population = itemizers, i.e., mid-and-up. | P4, scoped |
| **> $5,000 (crypto)** | P3 (Section B column (i)) starts here — a $6,000 crypto donor at $60k income hits the same fabricated cell as the $1M donor. Section-B appraiser warnings remain stderr-only. P6 becomes live whenever the gift exceeds the AGI ceiling — a *lower* bar at lower income. | **[RAN]** P3; **[RAN 08-21]** L7 |
| **> $500,000** | P5 (attach-the-appraisal) — this cell only. Rich-band by construction. | P5 |
| **gift > ceiling, any size** | P6 — at $0 AGI the ceiling is $0 and **100% of the gift carries over, invisibly**: L7's report mentions the charitable carryover nowhere. | **[RAN 08-21]** L7 |

### The five cells the brief named

- **$0 income with a gain — fine** (L2; 0% band verified by both engines).
- **Income with no crypto at all — fine**, two unsignalled hand-marks aside (L1; N4).
- **A gain that is a LOSS — fine at positive TI; N1 at the floor** (L3/L4).
- **A $0 donation — fine** (L0/L1).
- **The plainest case in the world — fine** (L1). A wage earner using a bitcoin tax tool with no
  bitcoin gets a correct 1040 and no stray crypto artifacts in the packet.

---

## WHAT I COULD NOT CONFIRM

### Corrections to the brief's own shared facts (unchanged from the single-vector pass)

- **"§G-13 records 6 remaining gap fields" is STALE.** `crates/btctax-forms/tests/field_census.rs:187`
  is `const GAPS: usize = 0;` — re-measured then; not re-litigated now. **Do not schedule gap-closing
  work.**
- ★ **GAPS = 0 must not be read as "the return is complete."** The census proves every blank is
  explained; it does not prove the explanation is right. P1, P7, P8, N1 and N3 all coexist with it.
- `Advisory` had **17** variants at the original pass; `RefuseReason` = 53.

### Claims I downgraded (unchanged)

- **AMT E4 "systematic map offset" — OVERSTATED**; three value anchors exist, a uniform shift reds.
  The real gap is 38 of 41 lines unanchored (localized swaps survive). Important, not Critical.
- **Charity item 2 (Section-B-no-appraiser emits with exit 0) — CRITICAL → Important**: warned twice
  on stderr, verbatim; the defect is stderr-only-with-exit-0 and `is_review_complete` gating nothing.
- **"btctax does *nothing* about §170(f)(11)" — imprecise**: the $5,000 threshold exists
  (`QUALIFIED_APPRAISAL_THRESHOLD`); the $500,000 *attach* rule is what is absent.

### Claims I could not settle (updated)

- **The AMT lens's branch table** (the "$270 margin", B′/C branches) — computed from the reference
  transcription, never driven through btctax. Unchanged: do not make the $270 margin load-bearing.
- **Whether §164(b)(6)'s cap limits Form 8960 line 9b** — still open; owner decision 5.
- **The §21 / Form 2441 figures** — Form 2441 still not extracted; unchanged.
- **`overflow::merge_copies` with per-copy identity writes** — settled only by its B1 test; unchanged.
- **Whether >14 disposal legs is the common case** — still judgment, but the generalization moves it:
  at *low* dollar sizes frequent small buys (DCA) are the modal acquisition pattern, so if anything
  the low end is *more* exposed. Owner decision 2 stands with a strengthened recommendation.
- **[NEW] The exact EITC/ACTC dollar for any specific household** — my $8,781 figure for L5 is
  Tax-Calculator's (EITC $4,778.18 + CTC $1,080 + ACTC $2,920), single-oracle by necessity: OTS was
  not driven on the low-end households, and the corpus *cannot* admit credit-engaged households (its
  W-2 "low" leg is deliberately floored above the childless-EIC band, r2-M5, and D-1 pins dependents
  to zero). Treat the magnitude as order-correct, not as a pinned KAT value, until an OTS run
  witnesses one such household. The *direction* and the ~20%-of-income scale are not in doubt.
- **[NEW] Whether the crypto-slice `report` path can compute the N1 worksheet** — the slice profile
  may not carry enough of the 1040 to produce taxable-income-before-floor. The full-return path can
  (it has the real TI). The N1 build below scopes to the full-return path and gates the slice.

### Conflicts between lenses, adjudicated against the form (unchanged)

1. **Form 8960 line 9b** — collect-or-blank plus an advisory; the ratio may be offered, not applied.
2. **Schedule A line 9 / Form 4952** — split: collect the line-20 boolean (refuse on `None` in
   `BothGains`), advise on the deduction, build no Form 4952.
3. **Cash or bitcoin gift** — plan for both; the shapes carry disjoint defects.
4. **AMT E4/E5/E6** — E6 not needed; E5 worth closing; E4 Important.
5. **Schedule 8812** — the single-vector adjudication ("unanimous: $0, do not build") is **RESCINDED
   as a general conclusion** and survives only as a statement about MAGI above the §24(b) window.
   See N2. The unanimity was an artifact of every lens looking at the same $2M household.

---

## THE PLAN

**Ordering principle (kept).** (1) a wrong tax figure that fails *silently* → (2) cannot produce
paper at all → (3) fabricated testimony on a §6065-signed page → (4) the figure is right and the
benefit is silently lost → (5) correct but unattested → (6) doc drift that will re-poison a
reviewer. Within a tier: cheap hazard-removal before large projects, ready before
blocked-on-a-decision. **One generalization gloss:** where two items share a tier, the one whose
population spans every income outranks the one confined to a band of one axis — this is what moves
N1 directly behind P1 and keeps P5/P8 at the bottom of their tiers.

★ This still deliberately inverts the capgain lens's "blocks filing first" ordering:
fails-closed loses to fails-silently.

| rank | # | item | class | population | size | vs original |
|---|---|---|---|---|---|---|
| 1 | P1 | §163(h)(3)(B) mortgage debt ceiling never asked | (b) wrong, silent, understates | acquisition debt > $750k — top of the income axis | S | **[SCOPED]** was #1 |
| 2 | N1 | §1212(b)(2)(B) carryover worksheet ignored on the OUT side | (b) wrong level, silent + misguides next year | loss year with TI-before-floor < 0 — bottom of the income axis | S | **[ADDED]** |
| 3 | P2a | Form 8949 overflow preflight + remedy message | (a) cannot file | any filer with >14 legs, all incomes | XS | **[STRENGTHENED]** was #2 |
| 4 | P2b | Form 8949 pagination on the full-return path | (a) cannot file | same | S–M | **[STRENGTHENED]** |
| 5 | P3 | Form 8283 Section B column (i) | (c) fabricated testimony | any Section-B filer (> $5,000 noncash), all incomes | XS | **[UNCHANGED]** |
| 6 | N3 | 1040 line 19 prints `0` for credit-eligible families | (c) fabricated testimony | dependents + MAGI below §24(b) — bottom/middle | XS–S | **[ADDED]** |
| 7 | P4 | §170(f)(8) acknowledgment + restore the line text | (b) overstates deduction, silent | itemizers with a ≥$250 gift | S | **[SCOPED]** (itemizers only) |
| 8 | P5 | §170(f)(11)(D) attach-the-appraisal over $500,000 | (a) incomplete return | claims > $500k — top of the donation axis | XS–S | **[SCOPED]** |
| 9 | P6 | carryover-out reaches no human | (b) benefit silently lost | any gift over its ceiling — *lower* bar at lower AGI | XS | **[STRENGTHENED]** |
| 10 | N2 | EITC + CTC/ACTC below the phase-outs | (b) benefit lost, loudly | families under ~$600k MAGI; EITC under ~$67k — the modal family | decision + M–L | **[ADDED]** — blocked on owner decision 11 |
| 11 | P7 | Schedule D line 20's hardcoded "Yes" | (c) testimony; (b) neighbour | every BothGains filer, all incomes | S | **[UNCHANGED]** |
| 12 | P8 | Form 8960 Part II line 9b | (b) overstates | MAGI > $250k/$200k — top of the income axis | S–M | **[SCOPED]**, still blocked |
| 13 | P9 | guards for things already correct (+ low-end additions) | (c) unattested | — | S total | **[STRENGTHENED]** |
| 14 | N4 | unsignalled hand-marks on the emitted packet | (e) correct but unserved | every no-crypto filer; every filer (signature) | XS | **[ADDED]** |
| 15 | P10 | stale comments/docs (+ LIMITATIONS:231 precision) | (f) drift | — | XS total | **[STRENGTHENED]** |

### P1 — the §163(h)(3)(B) acquisition-debt ceiling is never asked. Class (b). Size S. [SCOPED]

**[RAN]** With `mortgage_interest_1098 = "130000"` and every declaration answered truthfully, btctax
deducts 100%: ~$81,000 of overstated deduction, ~$30,000 of understated federal tax, exit 0, zero
advisories, on a notional $2,000,000 post-2017 mortgage that §163(h)(3)(B)(ii) caps at 37.5%.
Neither oracle can catch it (both consume line 8a as an input — §G-9). The cause is transcription:
i1040sca's *"Limits on home mortgage interest"* block has four limits and btctax typed in one.

**Where it stops mattering — the generalization.** The limit binds only when combined acquisition
debt exceeds $750,000 ($1M pre-2018 debt; $375k/$500k MFS). Underwriting arithmetic puts that
population above roughly $180k of household income; at $40k or $120k of income the ceiling is a
non-item not because the code improves but because the debt cannot exist. **P1 keeps rank 1 because
tier (1) is ordered by consequence and nothing else silently understates a *filed* figure** — but
any future severity review should read it as: the single worst silent number, in a band that is a
minority of filers.

Build = itemized S1 verbatim, unchanged: one `Option<bool>` on `ScheduleAInputs`, two
`RefuseReason`s (unanswered / over-limit), one `FormQuestion` on the existing
`mortgage_question_live` predicate, one `decl_tristate!`, B1 kill-tests per branch, prompt phrased so
the deductible answer is affirmative. Adverse branch = owner decision 3; the unanswered branch is
unblocked and ships first.

### N1 — the Capital Loss Carryover Worksheet is not applied to the carryforward-OUT. Class (b). Size S. [ADDED]

**[RAN 08-21]** Single filer, no wages, one long-term crypto loss of $20,000 (buy $60,000 2021, sell
$40,000 2024), every declaration answered:

```
$ btctax --vault L4.pgp report --tax-year 2024          # EXIT 0
  §1211 loss deduction (level): 3000.00   carryforward out: short 0.00 / long 17000.00
  Total income (1040 L9):   -3000.00 … Taxable income (L15): 0.00 … TOTAL TAX (L24): 0.00
```

The filed return is **correct** (line 7 = −3000 leading minus, Sch D line 21 = 3000, TI floored at
0 — all read back off the PDF). The printed **carryforward level is wrong**. The Capital Loss
Carryover Worksheet (`design/forms/extract/i1040sd--2024.txt:1445-1500`, implementing
§1212(b)(2)(B)'s adjusted-taxable-income offset) runs: line 1 = taxable income *as if negative* =
−17,600; line 2 = 3,000; line 3 = combine, floor 0 = **0**; so the amount of loss actually absorbed
is $0 and the long-term carryover to 2025 is **$20,000**, not $17,000. `net_1222`
(`crates/btctax-core/src/tax/compute.rs:126-199`) computes `carry = loss − absorbed` with
`absorbed = min(loss, $3,000)` flat (`compute.rs:178`) — no taxable-income term exists anywhere in
the carryforward path.

**Three aggravations.**
- **The IN-side twin already refuses.** `RefuseReason::TaxableIncomeNonPositiveWithCarryforward`
  (`return_1040.rs:2342-2350`) refuses a TI ≤ 0 year *with a carryforward-in*, citing this exact
  worksheet as unmodeled. The OUT side of the same worksheet, in the same file, silently misprints
  instead. This is the B3 shape a third time: the guard exists, nine lines of reasoning included,
  and was never carried to the sibling.
- **M4 will dispute the correct number.** `carryforward_consistency` (`compute.rs:444`, consumed at
  `cmd/tax.rs:553`) compares next year's declared carryforward-in against this year's *computed* out
  — so a filer who runs the worksheet themselves and correctly enters $20,000 next year is warned
  their number "does not match." The instrument built to catch transcription errors actively pushes
  toward the wrong figure.
- **Structurally unwitnessable today.** No corpus household sits in the edge region (the 5 loss rows
  all carry positive-TI incomes — measured **[RAN 08-21]**), and the sweep compares no carryforward
  line at all (`grep carryforward scripts/oracle/gen_goldens.py scripts/oracle/sweep.py` → nothing).
  A value the oracles never see is never validated by their agreement — §G-9 again.

**Where it stops mattering:** whenever taxable-income-before-floor ≥ 0 the flat rule is exact —
proved by the L3 run ($40k wages, same loss: carryforward 17,000 is *correct* there). The defect
region is precisely "the loss wiped the year out": gross income under the standard deduction plus
$3,000. Consequence per year: up to $3,000 of surviving loss, i.e., $450–$750 of next-year tax at
low-bracket rates, silently, plus the M4 misguidance.

**Build.** Transcribe the worksheet — all 13 lines, one field per line, the instruction text as doc
comments — as a `CapitalLossCarryoverWorksheet` computed in `assemble_absolute` *after* taxable
income exists (it needs TI-before-floor, the line 21 loss, and the ST/LT nets, all already
computed). The full-return carryforward-out reads off it; `net_1222`'s flat `st_carry`/`lt_carry`
remain for the crypto-slice path, which must gain the same guard the IN side has (a warning or
refusal when TI-before-floor < 0, since the slice may not hold a full 1040). Fix the
`LIMITATIONS.md:231` row, which currently implies the whole edge is handled by refusal. **B1 pair
required:** the L4 household as a KAT asserting `carryforward_out.long == 20_000` (reds today), and
the L3 household pinning 17,000 so the fix cannot overshoot. Mutation: drop the worksheet's line-3
floor and watch both directions.

### P2a — an 8949 overflow preflight. Class (a). Size XS. Ship immediately. [STRENGTHENED]

**[RAN]** 16 long-term legs, full-return path: exit 2, output directory never created, message names
no year and no remedy, while `report` on the same vault prints every figure. `admin.rs:875-910`
already contains two hand-written preflights of exactly this shape for Form 8275; adding the third
is a copy of code forty lines away.

**Generalization:** the failure is **lot-count**-driven. A filer who DCA'd $50/week and sold once has
50+ legs regardless of dollar size, so the low end of the *dollar* axis is, if anything, the more
exposed population — weekly accumulation is the modal small-holder pattern. The original plan's
framing ("the ordinary case for anyone who accumulated") holds at every income.

### P2b — paginate the full-return Form 8949. Class (a). Size S–M. [STRENGTHENED]

**[RAN]** The fix exists nine files away (`lib.rs:90-112` chunks and merges; `tests/overflow.rs`
proves it) and the comment at `fill8949_full.rs:73-75` claiming the slice also refuses is false.
Build = capgain 1 verbatim: chunk-and-merge via `fill_8949_parts_with_identity` per copy, B1 pair
(15 rows → `Ok` + row 15 on copy 2; planted direct call → `Overflow`), cross-foot Σ per-copy totals
≡ Schedule D lines 3/10, correct the false comment. Unchanged in content; population argument
strengthened as P2a.

### P3 — Form 8283 Section B column (i). Class (c). Size XS. [UNCHANGED]

**[RAN]** `f1_56[0]` prints the pre-ceiling `claimed_deduction` ($1,000,000 beside a Schedule A line
12 of $600,000) in a column i8283 reserves for pass-through entities. Fix: blank it for every btctax
filer, census the cells `unmodeled`, planted-defect test. Extract `f8283--2024.txt` first.

**Generalization note:** Section B starts at $5,000 of claimed noncash — a $6,000 crypto donation at
$60k income prints the same unrequested entry. This is a donation-axis item from $5k up, not a rich
artifact. Rank unchanged.

### N3 — 1040 line 19 prints `0` for families the credit belongs to. Class (c). Size XS–S. [ADDED]

**[RAN 08-21]** L5 (MFJ, $40k wages, two children): the emitted 1040 prints line 19 = `0` on the
signature page while Tax-Calculator puts the correct line 19 at $1,080 (and line 28 at $2,920).
`Form1040Lines::line19` is a bare `Usd` documented "**Always 0** (a §3.4 conservative omission)"
(`printed.rs:546`) and filled through the always-write plan (`form1040_full.rs:161`). Meanwhile
lines 27–30 got the opposite treatment: no field at all, censused unmodeled
(`f1040.map.toml:235-236`), printed **blank** — `line32`'s own doc says "lines 27–30 are blank: the
EIC is a §3.4 conservative omission" (`printed.rs:573-575`).

That asymmetry is the defect. Under "an entry is testimony," a blank line 27 *forgoes* the EIC —
lawful; a printed `0` on line 19 *asserts* the CTC is zero — which is **true** for the
`ctc_provably_zero` population (and there it is even the form's own instruction: Schedule 8812 line
12-No says "Enter -0- on lines 14 and 27") and **fabricated** for every family below the §24(b)
window, i.e., exactly the filers the `CtcOdcOmitted` advisory tells to "file Schedule 8812
yourself." The rich vector could never see this: at $2M MAGI the zero was provably correct.

**Build.** Make `line19` an `Option<Usd>`: `Some(0)` when `ctc_provably_zero` holds (the form
dictates the -0-), `None` otherwise (blank + the existing advisory), `push_money_opt` at the fill
site. The dependent sums (lines 21/22) treat blank as the form does. Mirrors itemized S3 (Schedule 1
line 21's eligible-vs-ineligible split) exactly. B1: a below-window family asserts no `/V` on the
line-19 cell; a provably-zero household asserts `0`. Do this **regardless of the N2 decision** — it
is testimony hygiene, not credit computation.

### P4 — §170(f)(8) is never asked; the line text was paraphrased away. Class (b). Size S. [SCOPED]

Confirmed absent (grep) and the printed line text compressed at `printed.rs:1317-1319`. Build
unchanged: one `Option<bool>` mirroring `donations_had_restrictions`, live when any single gift
≥ $250, refusal scoping = owner decision 4; the verbatim text restoration ships regardless.

**Scoping from the matrix:** §170(f)(8) conditions a *deduction*; a standard-deduction filer claims
none, so the CWA gap reaches paper only for itemizers. At $40k income the standard deduction wins
essentially always ⇒ the gap's population is itemizing filers with ≥$250 gifts — broad, but not "every
donor." The liveness predicate in the build (any single gift ≥ $250 **and the return itemizes**)
should carry that condition so a standard-deduction filer with a $300 gift is not gated on a question
whose answer cannot move their return.

### P5 — §170(f)(11)(D): attach the appraisal over $500,000. Class (a). Size XS–S. [SCOPED]

Confirmed: no $500,000 threshold exists. Build unchanged (constant + advisory + manifest line +
B1). **Population statement added:** this is the one donation-axis item confined to its top cell —
below $500,000 claimed it cannot fire, and no lower-income analogue exists. It keeps its tier
(incomplete return) but sits last within it.

### P6 — the carryover-out reaches no human. Class (b). Size XS. [STRENGTHENED]

**[RAN]** `charitable_carryover_out` reaches zero CLI/TUI sites; `--write-carryover` errors without
a year+1 row. **[RAN 08-21]** And this is not a rich-vector artifact: L7 ($0 AGI, $2,000 cash gift)
carries **100% of the gift** forward — the §170(b) ceiling is a *fraction of AGI*, so the lower the
income, the lower the bar for silent carryover creation — and the report mentions the charitable
carryover nowhere (only the capital-loss carryforward line and the *inbound* advisory print).

Fix unchanged: print it, per class and vintage, in `report --tax-year` and beside the §170 warnings
on export; gate with a test that reds when the line is removed. Note N1 is this item's sibling with
the sign flipped: P6 computes the right level and shows nobody; N1 shows everybody the wrong level.
Both close against the same doctrine — a figure with no (correct) reader.

### N2 — the refundable credits below the phase-outs. Class (b), loud. Decision + M–L. [ADDED — blocked on owner decision 11]

**[RAN 08-21]** L5 — MFJ, $40,000 wages, two qualifying children, no crypto:

```
btctax:   Tax (L16) 1083 · line 19 printed "0" · lines 27/28 blank · REFUND 417 (withholding only)
taxcalc:  EITC 4,778.18 + CTC 1,080 + ACTC 2,920  ⇒  net liability −7,698
          (claiming-model off; taxbc 1,080 vs 1,083 is the known Tax-Table methodology split)
```

**The emitted packet forfeits $8,781 — 22% of this household's income.** The posture is honest:
`CtcOdcOmitted` fires with the non-provable branch ("OVERSTATED by up to $2,000 per qualifying
child … File Schedule 8812 yourself"), `EicOmitted` fires (`advisories.rs:746-748`), lines 27/28
are blank, and LIMITATIONS discloses the omission. Nothing here is *silent*. But the generalization
pass has to state the population plainly: the children lens's own boundary arithmetic puts the CTC
window at MAGI ≤ ~$599k (MFJ, five kids) — **the §24(b) phase-out excludes almost nobody except the
rich vector this plan was calibrated on.** For any household with children in the bottom three
income cells, the credits btctax does not compute are the largest single number on their return,
and "file Schedule 8812 yourself" means hand-recomputing lines 19, 21, 22, 24, 27, 28, 32, 33 and
34 — at which point the packet is a worksheet, not a return.

**What this plan does with that.** It is a product-scope decision, not a defect fix, and it is the
single most consequential owner decision in this document (decision 11):

- **(i) Keep advisory-only** — defensible (forgone benefit never gates; the advisories are honest),
  but it should then be *stated as the product's stance*: btctax serves crypto filers' returns, and
  a family below the phase-outs should be told, in LIMITATIONS and the advisory, that the packet is
  systematically incomplete for them, with the magnitude bound.
- **(ii) Build Schedule 8812** (CTC + ACTC, Parts I/II-A/II-B): size M. Most inputs exist
  (dependents, DOBs, SSNs); the qualifying-child tests the form asks (residency months, support)
  would need collecting — which is following instructions. The earned-income base for line 18a
  exists (W-2 + SE). This serves the CTC/ACTC half ($4,003 of L5's forfeit).
- **(iii) Build EIC as well** (Schedule EIC + the Pub 596 rule set): size M–L, a genuinely larger
  collection surface (residency, relationship, tiebreaker rules, the §32(i) investment-income
  cliff — the last computable from data btctax holds). Serves the remaining $4,778.
- **Regardless of the decision, two XS moves ship now:** N3 (line 19 blank unless provably zero),
  and sharpening both advisories with *computed bounds* from data btctax already holds — the §24
  arithmetic (`ctc_provably_zero` already runs it) can print "up to $N for your M children at your
  MAGI," and the EIC advisory can stop over-firing: it currently fires on a flat
  `EIC_ADVISORY_AGI_CEILING = $70,000` (`advisories.rs:39`) for any earner, telling a childless
  single at $40k they "may qualify" when the childless AGI limit is ~$18.6k — **[RAN 08-21]**
  taxcalc: EITC $0 for L1. (The flat ceiling never *under*-fires — max EITC AGI is $66,819 — so the
  fix is per-status/child-count constants, a table of eight numbers, Minor.)

**Why ranked 10 and not 1:** the ordering principle ranks silent above loud, and this is the loudest
gap in the product. It outranks nothing above it; it dwarfs everything above it in per-household
dollars for the modal family. Both statements are true; the rank encodes the first, this paragraph
the second.

**A structural note the owner must hear:** the double-oracle sweep can never witness this gap. The
corpus floors its "low" W-2 leg above the childless-EIC band and pins dependents to zero *by
design* (r2-M5, D-1 — because admitting a credit-engaged household would diverge from btctax on
every such row). If (ii)/(iii) are built, that design inverts: the corpus gains credit households
and the oracles become witnesses instead of the thing being dodged.

### P7 — Schedule D line 20 answers a question nobody was asked. Class (c)+(b). Size S. [UNCHANGED]

Confirmed: `schedule_d_full.rs:279-284` checks Yes unconditionally on `BothGains`; the 4952 conjunct
has no source; the crypto slice adjudicated the same fact the other way. Build unchanged: collect
the boolean, add the routing variant (reds the exhaustive match), refuse on `None`/`filing-4952`,
wire Schedule A line 9 in the same edit, build no Form 4952 and no SDTW.

**Generalization:** every income cell with both-gains routing hits this line — the $0-income L2
household's Schedule D prints the same sworn "Yes" **[RAN 08-21]** (read back). Population is the
gains axis itself. Rank unchanged.

### P8 — Form 8960 Part II line 9b. Class (b), overstates. Size S–M. BLOCKED. [SCOPED]

Confirmed: `other_taxes.rs:312` hardcodes `line9d = Usd::ZERO`; both oracles structurally blind;
adjudication (owner decision 5) still required; the SPEC §3.4 advisory is unblocked and ships now.

**Where it stops mattering:** Form 8960 exists only above the §1411 MAGI floor ($250k MFJ / $200k
Single) — the bottom two income cells never see the form (L1/L2/L5 packets contain no 8960 —
**[RAN 08-21]**). Rich-band item; rank unchanged (it was already last of the number-movers).

### P9 — attestation for things that are already correct. Class (c). Size S total. [STRENGTHENED]

The original five stand: NIIT×charitable non-interaction KAT + corpus household; the two-NIIT-chain
assertion; `f6251.line20 == qdcgt.line5`; G-6d's missing AMT vector; E4's per-line read-back. The
low-end pass **adds four**, all currently guaranteed by nothing:

- **The 0% preferential band at $0 ordinary income.** L2's tax-$0 is held by no KAT naming the cell
  (the corpus has zero-wage-with-gain rows, which witnesses the sweep — but the sweep's admission is
  itself the only guard). One KAT: L2's shape, `line16 == 0`, mutation = perturb the 0% breakpoint.
- **The §1211(b)/§1212(b) pair at positive TI.** L3's shape (cap −3,000, carry 17,000, leading-minus
  line 7) — the loss-year printed surface has corpus witnesses but no in-repo KAT reading the PDF
  back. Cheap now that the fixtures exist.
- **The standard-deduction election under `itemize_election = "auto"`.** Exercised constantly,
  asserted nowhere low: a KAT pinning that a $5,000 gift + standard deduction files with **no
  Schedule A and no 8283** in the packet (the L7 shape) — the absence-of-a-form assertion the §G-28
  lesson says value-checks cannot see.
- **The all-zero return.** L0 files today; nothing pins that it keeps doing so (the "no all-none
  row" corpus exclusion means no oracle ever runs it). One KAT: all-zero inputs → exit 0, 1 form,
  every money line 0 or blank per its provenance.

Plus, from N1: the corpus should gain the L4 shape (TI-at-the-floor loss year) — admissible today
(no credits engage: EIC requires earned income) — so the *rest* of that household is two-oracle
witnessed even while the carryforward level itself stays oracle-invisible.

### N4 — the packet's unsignalled hand-marks. Class (e). Size XS. [ADDED]

**[RAN 08-21]** L1's packet (no crypto): the digital-asset question prints neither Yes nor No —
correct, btctax cannot swear "No" for a ledger it was not given — and the line-7 "If not required,
check here" box is blank with the reasoning censused (`f1040.map.toml:293`, a good census entry).
But the filer is never told either mark is theirs to make: no advisory names the digital-asset
question (grep: zero hits outside the filler), and the manifest (`L1-irs/manifest.txt`) is one line
of stapling order. A no-crypto filer signs a return with a mandatory question unanswered and no
instruction anywhere in the product output.

Fix: one advisory (or a "complete by hand" block in the manifest) enumerating the marks btctax
deliberately leaves: the digital-asset Yes/No (when no crypto activity exists), the line-7
checkbox, the signature/date/PIN block, and — L0's nit — a note when gross income is under the
filing threshold that a return may not be required at all. This is serving, not testimony: btctax
must not *make* any of these marks, only stop being silent about them. B1: a test asserting the
no-crypto export names the digital-asset question on stderr; delete the block, red.

### P10 — stale comments and docs. Class (f). Size XS total. [STRENGTHENED]

The original four stand: `LIMITATIONS.md:171` (the Schedule 8812 row contradicting
`ctc_provably_zero`'s two branches — and note the low end makes the row's "overstated" text *true*,
so the fix is the split the children lens specified, plus the kill-test that does not exist);
`map.rs:467-470` (>4 dependents "REFUSES" claim, false); `fill8949_full.rs:73-75` ("exactly as the
slice does", false); `donation.rs` Part III/IV → IV/V. The low-end pass **adds one**:

- **`LIMITATIONS.md:231`** — "Taxable income ≤ $0 with a capital-loss carryforward — the §1211/§1212
  Capital Loss Carryover Worksheet edge is unmodeled." True but incomplete in a way that misleads:
  it describes the refusing IN side and is silent about the OUT side, which does not refuse — it
  prints the wrong level (N1). Until N1 lands, the row must say so; after N1, rewrite it.

### Explicitly out of scope (revised)

Form 2441 / §21 (unchanged — but its refusal note in "three things" below gains a population:
W-2 box 10 is income-independent); Form 4952 itself; the Schedule D Tax Worksheet; Form 6251 lines
2c–2t as collected fields; §170(b)(1)(C)(iii); G-6c (MFS); anything TY2025/TY2026. **Removed from
this list:** Schedule 8812 — no longer "out of scope by unanimity" but *blocked on owner decision
11*, which is a different thing. The kiddie-tax population (a claimable dependent with crypto
gains) stays refused — that refusal is honest and correct at every income.

---

## THE THREE THINGS, SEPARATED

**(a) What stops a filer FILING AT ALL today**
- **P2** — more than 14 disposal legs, at any income. **[RAN]**
- **P5** — over $500,000 claimed with no appraisal attached (rich cell only).
- The Section-B-with-no-appraiser packet: emitted, warned on stderr, exit 0. **[RAN]**
- Honest refusals that fire and are answerable, and should not be "fixed": the mortgage pair, the
  §G-22 attestation, `NonCryptoNoncashGift`, the §G-21 gate, **KiddieTax** (a claimable dependent
  with gains — the low end's own refusal, correct), and `TaxableIncomeNonPositiveWithCarryforward`
  (the IN side of N1's worksheet — correct, and the model for N1's slice-path guard).
- `RefuseReason::DependentCareBenefit` refuses the whole return on W-2 box 10 > 0 — §129 FSAs are
  income-independent, so this hits the $40k family as readily as the $1M one.

**(b) What lets them file with a WRONG or INCOMPLETE number**
- **P1** — ~$30,000 understated, silent, jumbo-mortgage band. **[RAN]** Still the only silent
  understatement of a filed figure.
- **N1** — the carryforward level, understated up to $3,000 per floor-year loss, silent, plus M4
  misguidance next year. **[RAN 08-21]** The only wrong number the *bottom* of an axis produces.
- **P4** — the deduction stands in full with no CWA; itemizers, silent.
- **P6** — carryover-out computed and never shown; any gift over its AGI ceiling. **[RAN 08-21]**
- **N2** — $8,781 forfeited on L5, *loudly*; the modal family. **[RAN 08-21]**
- **P8** — $190–$3,800 of NIIT overstated; MAGI > $250k only.
- **P7's adjacent filer** — indeterminate direction, any income with both gains.
- Latent, outside every executed vector: Schedule A line 6 → 6251 line 2a; Schedule E → 8960
  line 4a; the private-foundation mislabel (unreachable guard on the crypto path).

**(c) What is correct but unattested**
All of P9 including its four low-end additions; E4's 38 unanchored 6251 lines; N4's hand-marks
(correct blanks nobody is told to fill). The standard-deduction path, the 0% band, the §1211 cap,
and the all-zero return are all *correct today* and held by corpus admission rather than by any
in-repo test that reds on regression.

---

## OWNER DECISIONS

### ★★★ SETTLED 2026-08-21 — the product scope, and what it decides

**Owner: *"We want to support a broad array of income and capital gains amounts."*** That is a
population statement, and it answers two decisions outright without re-ranking anything — the plan
already ranks by population within each tier, so a wider population changes what is IN, not what is
first.

- **Decision 2 — >14 disposal legs is a REAL population. P2b is LIVE, not latent.** The overflow is
  **lot-count-driven, not dollar-driven**, so the exposure runs opposite to intuition: a filer
  dollar-cost-averaging weekly holds ~52 lots and any meaningful sale draws on more than 14 of them,
  while a single-lot whale with a $1M gain emits one row. Supporting a broad array of capital-gains
  *amounts* therefore means supporting many-lot histories at the **small** end especially, and the
  failure there is total — exit 2, zero bytes, every form in the packet lost.
- **Decision 11 — EITC/ACTC is IN SCOPE. N2 unblocks.** A broad array of income includes the bottom,
  which is exactly where the refundable credits live: ~$8,781 forfeited on MFJ/$40k/2 children,
  ~22% of that household's income, oracle-verified. It is the largest dollar item in this plan and
  it lands on the filers least able to absorb it.

★ **What this does NOT change.** The ordering principle stands, and no item is re-tiered. P1 remains
first because a silent understatement that FILES outranks a loud failure that produces no paper —
a wrong number gets signed under §6065; a missing PDF does not. P1, P5 and P8 remain correctly
scoped as top-of-axis: a broad product still serves that band, it is simply not the modal filer.

★★ **What it does change about SEQUENCING.** N2 is not a plan item at this scope — it is a project.
It needs Schedule 8812 and Schedule EIC (neither has a map), a refundable-credit path that does not
exist, new collected inputs (earned income, the §32(i) investment-income limit, qualifying-child
residency), and a two-oracle witness — noting taxcalc's default take-up simulation zeroes the EITC
stochastically, so `eitc_claim_prob_scale` must be disabled or the oracle silently agrees with a
wrong answer. It belongs in its own cycle starting at brainstorm, not as a line in this table.

---

Decisions 1–10 stand as written in the single-vector plan; restated compactly with any
generalization delta, then the new ones.

1. **Cash or bitcoin gift?** Both; disjoint defect sets. Unchanged.
2. **Is >14 disposal legs a real population?** Recommendation strengthened: at low dollar sizes DCA
   makes many-lot histories the norm, so P2b should be presumed live for every income cell. Ship
   P2a today regardless.
3. **P1's adverse branch: refuse, or zero-and-advise?** Recommend refuse. Unchanged.
4. **P4 scoping: refuse on `None`, or only `Some(false)`?** Recommend refuse on `None`, **with the
   liveness now also conditioned on itemizing** (matrix: a standard-deduction filer's answer cannot
   move the return — do not gate them).
5. **Is Form 8960 line 9b capped by §164(b)(6)?** Adjudicate against the statute. Unchanged.
6. **Line 9b: compute a default allocation, or collect it?** Collect-or-blank. Unchanged.
7. **§170(f)(11)(D)'s $500,000 — contributed or claimed?** Settle against the statute. Unchanged.
8. **Form 4952: boolean only, or build?** Boolean + refuse. Unchanged.
9. **Schedule 8812 / Form 2441: build or out of scope?** — **SUPERSEDED for the 8812 half by
   decision 11.** The 2441 half stands (out of scope, with the extract-first build sequence filed).
10. **TY2024-only loudly enough?** Leave; every artifact downstream must say TY2024. Unchanged.

11. **[NEW — the structural one] Does btctax serve households below the credit phase-outs, or
    disclaim them?** Options: (i) advisory-only, stated as a stance, with computed bounds in the
    advisory text; (ii) build Schedule 8812 (M); (iii) build Schedule 8812 + EIC (M–L).
    **Recommend deciding (i) vs (ii) now and treating (iii) as its own later gate** — the CTC/ACTC
    half has most inputs already collected and no take-up ambiguity; the EIC half carries the Pub
    596 rule set and a larger collection surface. What is *not* recommended: leaving "do not build
    8812" on the books as if it were a general truth — it was a fact about one household at 3.3×
    the phase-out ceiling. Note the corpus consequence either way (r2-M5/D-1 invert if built).
12. **[NEW] N1's slice-path behavior.** The full-return path gets the transcribed worksheet; the
    crypto-slice `report` may not hold a full 1040. Options: warn on TI-unknown loss years, or
    refuse symmetric with the IN side. **Recommend warn** (the slice is a planning surface, not a
    filing one), with the wording naming the worksheet.
13. **[NEW] N4's delivery: advisory or manifest?** **Recommend the manifest** — the hand-marks are
    per-packet facts, and the manifest is the one artifact the filer is told to follow while
    assembling paper. Either way, B1 it.

---

## WHAT THIS STILL WILL NOT DO

Even with every item above landed:

- **It will not compute a refundable credit unless decision 11 says build** — and until then a
  below-phase-out family's packet is systematically a worksheet, not a return, however honest the
  advisories. This is now the product's largest per-household dollar gap, and it is a chosen one.
- **It will not serve an ISO-exercising executive** (the §G-22 refusal is correct; unchanged).
- **It will not serve rental income, a K-1, or any Schedule E** — and note the low end has these
  too: a $40k filer with one rental room is as unservable as the $1M one.
- **It will not compute an investment-interest deduction** (P7 stops the false testimony only).
- **It will not make blank expressible on most money lines.** §G-11 is architectural; N3 and S3 fix
  two instances (line 19, Schedule 1 line 21) by the per-line `Option` pattern, and the emitted `0`s
  on a no-crypto filer's lines 2b/3b/7/8 remain lawful only because the empty input lists are
  themselves the filer's testimony. The class remains open.
- **It will not give the AMT-owing branch a two-oracle end-to-end witness** (D-2; unchanged), **and
  it will never give the credits gap one** unless decision 11 builds the credits — the corpus
  excludes credit-engaged households *because* btctax omits the credits, which is the same §G-9
  shape as every other value the oracles are handed rather than derive.
- **It will not file any year but TY2024.**
- ★ **The census still answers "is every blank explained?", not "is every explanation right?"** —
  GAPS = 0 coexisted with P1, P7, P8 at the rich end and coexists with N1 and N3 at the low end.
  Two of this revision's four additions live on lines the census marks fully accounted for. Nothing
  in the repo yet audits explanations, and this plan does not change that.
