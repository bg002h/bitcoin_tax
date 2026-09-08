# Fold — the T11 seam review (2C / 4I / 3M / 1N)

Single implementer, shared main tree `/scratch/code/bitcoin_tax`, branch `main`, dispatch HEAD
`3e82e852`. **Nothing committed, nothing pushed**; no `git stash` / `restore` / `checkout --` was
used — every plant was reverted from a `cp` backup and `touch`ed (FR-90). No subagents.

**Closing gate — `make check`: `Summary [ 20.888s] 3510 tests run: 3510 passed, 12 skipped`, exit 0.**
`cargo fmt --all` clean; `CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets
--all-features -- -D warnings` clean (no output, exit 0). Every measurement below was taken after
`find crates -name '*.rs' -exec touch {} +`.

**Both oracles ran live**: OpenTaxSolver 2024 (`OTS_DIR=~/OpenTaxSolver2024_22.07_linux64`) and PSL
Tax-Calculator 6.8.2 (`.venv/bin/python`). Neither baseline moved — §"Oracles", below.

`3506 → 3510 tests`: **+4**, all new KATs (§"Kills").

---

## ★ The ROOT — the completeness partition was `Usd`-typed, so a fact that ROUTES money was invisible

C-2 and I-1 were one defect, and the fold closes the root rather than the two instances.

**What changed.** `oracle_projection.rs` now derives **two** partitions over `ReturnInputs`, and
`ORACLE_INVISIBLE` accounts for both:

| partition | question | how the alternatives are derived |
|---|---|---|
| AMOUNT (HEAD) | does this `Usd` leaf's **figure** reach an oracle box? | `leaf_walk::money_leaves` — by TYPE, through `Decimal`'s deserializer |
| **ROUTING (new)** | does this **non-`Usd` fact** reach the drivers? | serde's own `unknown variant … expected one of …` list for a unit enum; `true`/`false` for a boolean |

The routing partition asks its question two ways, because neither reaches both instances alone:

1. **Fidelity**, on fixtures where the description starts EXACT. `description_gap(ri)` is the vector
   of compared quantities of `ri` minus those of
   `build_golden_return(project_to_golden(ri, …))` — the household the engines are actually handed.
   The fixtures are built BY `build_golden_return`, so the gap is zero by the inverse KAT, and the
   test **asserts** that zero before perturbing anything. Any perturbation that makes the gap
   non-zero is a fact the drivers are not told, attributable to that perturbation alone.
2. **Presence**, on the maximal sentinel (whose `Vec`s are realized two rows deep). A perturbation
   that moves the filed return and moves **nothing** in the row is a fact the row cannot express.
   Per-fixture rather than a difference of differences, so it is immune to the interaction below and
   can safely use the sentinel's breadth.

**Why "did the row move?" is not the question, measured.** Changing a gift's class from `Cash60` to
`CapGainProp30` makes `charitable_cash` fall from $6,000 to $0 — the row moves, and the whole gift is
gone. And on the maximal sentinel (which carries figures the row cannot express) a plain
gap-invariance test blames the wrong leaf: perturbing `filing_status` there changed the gap because
it changed how much the UNCARRIED spouse-death fact was worth. Both were observed while building the
probe; the two-question shape is the answer to both.

`probed = 717` perturbations — measured by planting the anti-vacuity floor at 100 000 and reading the
failure (*"the routing probe made only 717 perturbations"*), never counted by hand. The floor the
committed test carries is `probed > 200`.

### What the extended derivation NOW COVERS

* Every walked leaf of `ReturnInputs` that is **not** a money leaf and whose type is a **unit enum**
  or a **boolean**, over both fixture families — 717 perturbations. Enum variants come from serde's
  own error text, so a variant added tomorrow is probed the day it exists.
* Both directions of accounting: a routing fact that breaks the description must appear in
  `ORACLE_INVISIBLE`, and `no_oracle_invisible_entry_is_stale` now walks **all** leaves (it walked
  only money leaves before, which would have reported the routing entries as stale and could never
  have noticed a routing entry going stale).

### What it does NOT cover — stated, because a wrong stated limit is worse than none

* **Free text, dates, integers and opaque codes.** No alternative is derivable from the type. The
  live example is `header.spouse.date_of_death` / a filer's date of death — a §G-9 routing fact whose
  *boolean* half (`taxpayer_died_during_year`, `spouse_died_during_year`) IS probed and IS now named
  in the table, but whose date half is not. FR-93 records the consequence.
* **A leaf absent from the serialized document** (`skip_serializing_if`), and **the element types of
  an empty `Vec`** — the two blind spots `leaf_walk::money_leaves` already documents. Mitigated the
  same way: the sentinel realizes two rows of every `Vec`.
* **MONEY leaves are not re-examined by the fidelity measure.** Their partition stays *"does the
  figure reach an oracle box"*, so a money leaf that reaches the row **at the wrong value** is
  invisible to both partitions. Three such defects were live at HEAD (below); all three are fixed,
  but nothing structurally prevents the next one. Running fidelity over money leaves reports **56**
  candidates on the maximal sentinel, almost all of them `build_golden_return`'s deliberate
  many-to-one collapses (one wage figure, one interest figure — FR-91), so it needs those modelled
  first. Measured, not estimated. Filed as **FR-95**.
* The **ledger** is censused separately and only its income half is derived — **FR-96**.

### Three latent projection defects the new measure found, all understating the description

Fixed by making `project_to_golden` read the **filed** `ScheduleAParts` instead of re-deriving it —
which is the two-chain rule, and is why the projection now takes the assembled `AbsoluteReturn`:

| leaf | the projection said | the filed Schedule A says |
|---|---|---|
| `w2s[].box17_state_tax_withheld` + `box19_local_tax` | not in line 5a (listed as `PaymentOrWithholding`, *"btctax is federal-only"*) | `income_tax_salt` puts both **on line 5a** |
| `form_1098[].box6_points` | not in line 8a (*"line 8a takes them inside box 1's figure"*) | line 8a is **Σ(box 1 + box 6)** (`i1040sca--2025.txt:1060-1061`) |
| `schedule_a.mortgage_all_used_to_buy_build_improve` | ignored | a mixed-use mortgage **zeroes 8a** (§163(h)(3)(F)) and checks the line-8 box |

All three entries were removed / the flag now carried; the KAT reds if any is listed again. Live
proof of the first: the scratch vault's row prints `state_income_tax: 7000.0` = $1,000 estimated
payments + $6,000 W-2 box 17.

A fourth, found the same way and fixed: `hsa_deduction` was projected unconditionally, so a filer who
answered *"no HSA activity"* while carrying a stale Form 8889 line-2 figure was described to both
engines WITH a §223 deduction btctax does not take. Now gated on `sch1.hsa_activity == Some(true)`.

`ORACLE_INVISIBLE` is **67 entries**, the same count as HEAD: +3 (`schedule_a.charitable[].class`,
`header.taxpayer_died_during_year`, `header.spouse_died_during_year`), −3 (the two W-2 state/local
boxes and the 1098 points box, all now visible).

**The kill, and its observed red.** Two plants, both reverted from a `cp` backup and `touch`ed.

*Plant A — delete the `schedule_a.charitable[].class` entry from `ORACLE_INVISIBLE`:*

```
thread 'every_routing_fact_either_reaches_the_row_or_is_named_in_oracle_invisible' panicked:
these NON-money facts change the return btctax files without the oracle row following — the drivers
would be answering about someone else. …
  schedule_a.charitable[].class = "cap_gain_prop30"  (the description stops reproducing the return)
  schedule_a.charitable[].class = "ordinary_prop50"  (the description stops reproducing the return)
```

(Exactly the two classes a filed btctax return can carry; the other three refuse at import under
`RefuseReason::NonPublicCharityContribution`.) Two further tests red on the same plant.

*Plant B — the reviewer's own: delete the class filter entirely, so every gift class rides
`e19800`/`A11`.* At HEAD this left all 9 T11 KATs plus `golden_returns`/`kat_tax` green — that was
the review's headline evidence that the partition, not the tests, had failed. Now:

```
$ cargo nextest run --locked -p btctax-core --test oracle_projection --test golden_returns --test kat_tax
FAIL btctax-core::oracle_projection a_non_cash_gift_is_dropped_by_the_projection_and_reported_as_such
  assertion `left == right` failed
    left: 6000.0
   right: 1000.0
     Summary 99 tests run: 98 passed, 1 failed, 0 skipped
```

Reverted; `git diff --stat crates/btctax-core/src/tax/testonly.rs` back to the fold's own diff; 12/12
green again.

---

## C-1 (Critical) — the harness reconciled a return `btctax report` refuses

**What changed.** `income project` now runs the filer's OWN refuse chain — `screen_inputs` →
`screen_compute_dependent` → `assemble_absolute` → `screen_absolute`, the same chain every computing
consumer composes — on the loaded `ReturnInputs`, and emits a `"refused"` block naming the screen and
carrying its own sentence. The row still prints, so a filer can see what the engines would be told.
`check_return.py` exits **2** on it.

**Where.** `crates/btctax-cli/src/cmd/tax.rs::project_return_inputs`;
`scripts/oracle/check_return.py::main`; the docstring, `cli.rs`'s `Project` help and
`scripts/oracle/README.md`.

The refusal is taken *there* and not in the harness deliberately: the harness rebuilds a household
from the ROW with `build_golden_return`, a fixture builder that sets the tax year and answers every
live declaration plus both Schedule B Part III questions. The corpus depends on that, so it was not
touched — the **claim** was fixed instead, at the point the filer's own return is in hand.

**Exit 2 did not exist.** Every *"could not be made"* path used `sys.exit(<str>)`, which exits **1** —
the divergence code — so a missing OTS install and a real disagreement were indistinguishable, and the
docstring's `Exit 2 = … a refused return` contract could not have been honoured even once the refusal
was detectable. All ten such paths now go through `cannot_run()`.

**The claim is now bounded, in three places.** The docstring, the `cli.rs` help and the README each
say plainly that **what is checked is the DESCRIPTION, not the packet**: every column including
btctax's own is computed from the projected row, so a figure the row cannot carry is missing from all
three columns and produces silence rather than a disagreement.

**The kill and its observed red.** A scratch vault whose Schedule B Part III (FBAR) gate is
unanswered — the reviewer's own case:

```
$ btctax --vault ./v.pgp report --tax-year 2024
error: usage: tax year 2024 cannot be computed from its full-return inputs: Schedule B Part III
line 7a (a foreign financial account) must be answered on every return …

$ btctax --vault ./v.pgp income project --year 2024 > row.json      # exit 0, a full row
  "refused": { "screen": "ScheduleBPart3Unanswered", "detail": "Schedule B Part III line 7a …" }

$ .venv/bin/python scripts/oracle/check_return.py --file row.json
btctax REFUSES this return — the ScheduleBPart3Unanswered screen fired, so there is nothing to compare:
  Schedule B Part III line 7a (a foreign financial account) must be answered on every return …
EXIT=2
```

*Plant — the `"refused"` block removed from the payload:*

```
$ .venv/bin/python scripts/oracle/check_return.py --file row_planted.json
…
Every compared line reconciles.
PLANTED EXIT=0
```

— the HEAD behaviour, reproduced on demand.

**And a repo test now holds it**, because nothing did: planting `"refused": None::<Refusal>` in
`project_return_inputs` left all 12 `tax_profile` tests green.
`income_project_reports_the_screen_that_refuses_the_filers_own_return` (new, `tax_profile.rs`) asserts
the block names `ScheduleBPart3Unanswered` and carries the words *"foreign financial account"*, that
the row still prints, and — so the block is not a constant — that a FULLY answered MFJ return (every
live declaration, the whole document census, all fifteen §152 dependent gates, committed as the
`ANSWERED_MFJ_RETURN` literal) carries **no** block and really does itemize and really does claim a
child. Planted:

```
thread 'income_project_reports_the_screen_that_refuses_the_filers_own_return' panicked at
crates/btctax-cli/tests/tax_profile.rs:324:9:
this fixture must be a return btctax refuses, else the test proves nothing
```

---

## C-2 (Critical) — no projected row could drive OpenTaxSolver's Schedule A

**What changed.** `GoldenInputs` gained `standard_or_itemized: GoldenDeduction` (`Standard` |
`Itemized`, `#[serde(default)]` ⇒ `Standard`), set by `project_to_golden` from btctax's own
`ar.deduction_is_itemized`. `build_golden_return` deliberately IGNORES it — btctax decides
§63(e)/(c)(6) from the amounts, exactly as the corpus intends — which is what makes the inverse KAT a
real check: a row claiming `Itemized` whose amounts lose to the standard deduction reds.

**Where.** `testonly.rs` (`GoldenDeduction`, the field, `project_to_golden`); no change to
`ots_direct.py` was needed — it has read the key since the SALT axis landed, and 27 of the 107 corpus
cells write it. Those 27 now deserialize it instead of dropping it on the floor, and the corpus
inverse passes on all 107, i.e. btctax's line-12 decision agrees with every cell's declaration.

**The kill and its observed red.** An itemizing MFJ household (W-2 $150,000; SALT $1,000 + $6,000 box
17 + $5,000 real estate; mortgage $30,000; $2,000 cash gift), both engines live:

```
deduction (L12) [OTS]                       42000      42000  OK  agree-ots
deduction (L12) [taxcalc]                   42000      42000  OK  agree-taxcalc
SALT (Sch A L5e) [OTS]                      10000      10000  OK  agree-ots
SALT (Sch A L5e) [taxcalc]                  10000      10000  OK  agree-taxcalc
── witness census ── 8 of 11 compared lines have TWO independent engines behind them.
```

*Plant — the field deleted from the row:*

```
taxable income (L15)                       108000 120800/108000  DIVERGES  diverge
deduction (L12) [OTS]                       42000      29200  DIVERGES  diverge
deduction (L12) [taxcalc]                   42000      42000  OK  agree-taxcalc
SALT (Sch A L5e) [taxcalc]                  10000      10000  OK  agree-taxcalc      <- no [OTS] row
── witness census ── 7 of 11 …
     1040sa.line5e          witness: taxcalc  — *** NO MECHANISM ON FILE — see below ***
EXIT=1
```

The reviewer's failure reproduced exactly: a false DIVERGES and exit 1 on a correct return, and
Schedule A line 5e silently losing its OTS witness. **And the I-3 fix is seen discriminating in the
same run** — the census refuses to paper over the lost witness with the old default string.

*Rust-side plant — `project_to_golden` stops reading the decision:*

```
thread 'the_projection_inverts_build_golden_return_on_every_committed_household' panicked:
the projection is not the inverse of `build_golden_return` on mfj_itemized_over_100k
  left:  … standard_or_itemized: Standard …
 right:  … standard_or_itemized: Itemized …
```

**Recorded honestly as FR-94:** OTS's `A18` is *"Elect to itemize, even when less than standard
deduction"*, so handing it this token makes the line-12 **branch** an oracle input (§G-9) — the
AMOUNT stays independently computed, and Tax-Calculator still chooses the branch for itself, so line
12 keeps a genuine second opinion on the choice. Closing it properly means giving OTS the components
unconditionally and setting `A18` from the filer's own §63(e) `itemize_election`; that changes how
all 27 itemizing corpus cells are driven and needs a re-bake, so it is its own cycle.

---

## I-1 (Important) — a non-`Cash60` charitable gift was dropped by the projection and both censuses

Folded as a consequence of the root, above. Two halves:

**The projection.** `charitable_gift_projects(class)` is the single `_`-free predicate; only Schedule
A **line 11**'s cash class rides `e19800`/`A11`.

**The census.** `unprojected_nonzero_leaves` gained a derived pass over the gifts that same predicate
drops, reporting each amount under the new `schedule_a.charitable[].class` entry. One predicate, two
readers — a hand-list would have been a second thing to keep true.

**Decided differently from one arm of the review's *"either/or"*, and why.** The review offered
*"either carry the non-cash class (taxcalc `e20100`, OTS `A12` — both model it) or report it … with
its §170(b) ceiling mechanism"*. **It is reported, not carried.** A line-12 gift carries its own
§170(b) 30%/50%-of-AGI ceiling and **OTS 2024 applies no §170(b) ceiling at all**, so putting it into
`A12` would hand OpenTaxSolver a deduction btctax caps and OTS does not — a FALSE divergence on a
correct return, which is the V2b shape `charitable_cash`'s own doc comment already records. It is
also the disposition `unprojected_ledger_lines` already gives a crypto donation, which lands on the
identical line.

**The kill**: `a_non_cash_gift_is_dropped_by_the_projection_and_reported_as_such` (new). It first
measures its own premise — btctax's line 12 really does move by the $5,000 `CapGainProp30` gift —
then asserts the row carries only the $1,000 cash gift, that the census names the dropped one with a
`§170(b)` mechanism, and that the fidelity measure sees the loss. Reds on both root plants above.

---

## I-2 (Important) — the credit excuse could not tell a blank from an asserted `0`

**What changed.** `credit_line_verdict(printed: str | None, oracle_value)` takes the whole
`Optional`. `None` ⇒ no cell, no testimony, the forgo excuse (the gap must be the WHOLE oracle
credit). `Some(v)` ⇒ btctax **swore** a figure under §6065 and it must EQUAL `round(oracle_value)`.

**Where.** `scripts/oracle/check_return.py` — `credit_line_verdict`, its call site, `selftest`, the
module docstring, and `scripts/oracle/README.md`.

**The FR-85 connection is written into the source and into `--selftest`,** as the brief requires. The
function's doc comment names `advisories.rs::ctc_odc_line19` (which returns `Some(Usd::ZERO)` whenever
`ctc_provably_zero` fires) and FR-85 (the §24(h)(2) per-child ceiling is a year-blind $2,000 against
TY2025's $2,200 under Pub. L. 119-21 §70104(a)(2)), and states that this check is the one instrument
that can catch the sworn `0` when the TY2025 package lands. `selftest` case 4 is that exact plant and
runs offline forever. The review's third case is also corrected: a btctax that printed the CORRECT
full credit now **reconciles** (it used to be asserted as a divergence, which would have failed on a
correct return the day Schedule 8812 lands).

**The kill and its observed red.** MFJ / W-2 $420,000 / one CTC child aged 9, both engines live.
Plant `CTC_PER_CHILD_SS24H2 = dec!(1000)` in `advisories.rs:857`; `touch`; rebuild the harness.

| | btctax | taxcalc | verdict | exit |
|---|---|---|---|---|
| clean HEAD-of-fold | `blank` | 550 | `OK (excused)  gap=550 expected=550` | 0 |
| **planted** | **`0`** | 550 | **`DIVERGES  gap=550 expected=550`** | **1** |

```
DIVERGENCES:
  · CTC/ODC (L19): btctax SWORE 0, taxcalc computed 550 — a printed cell is testimony, not a forgone
    line, so it must EQUAL the oracle's figure; the gap is 550 …
```

Restored from the `cp` backup and `touch`ed (`git diff --stat advisories.rs` empty), harness rebuilt,
the row prints `blank … 550 … OK (excused)` at exit 0 again. Under the HEAD verdict the planted run
was `OK (excused)`, exit 0 — the reviewer's measurement.

Offline: `check_return.py --selftest` → `check_return: blank-vs-sworn, refusal, year and witness
checks discriminate (B1 kill OK)`, exit 0.

---

## I-3 (Important) — the witness census printed a false mechanism

**What changed, both halves.**

* **The figures are now asked for.** `schedule_se.line12`, `8959.line18` and `8960.line17` each gained
  a `[taxcalc]` leg in the harness beside the existing `[OTS]` one (`t.se_tax`,
  `t.additional_medicare_tax`, `t.niit` — already baked, and already compared cent-exact against
  btctax on all 107 corpus cells by `golden_returns.rs`'s Level-1 block, which is why this was safe).
  `1040sa.line5e` got its OTS witness back with C-2.
* **The default string is gone.** An unnamed single-witness line is now an **error**, not a shrug.

**Where.** `crates/btctax-oracle-harness/src/main.rs`; `check_return.py`'s `SINGLE_WITNESS_REASON`
and census.

**Live, on MFJ / $300,000 W-2 / $35,000 Schedule C / $9,000 interest / one CTC child:**

```
Sch SE L12 (SE tax) [OTS]                     937        937  OK  agree-ots
Sch SE L12 (SE tax) [taxcalc]                 937        937  OK  agree-taxcalc
8959 L18 (Add'l Medicare) [OTS]               741        741  OK  agree-ots
8959 L18 (Add'l Medicare) [taxcalc]           741        741  OK  agree-taxcalc
8960 L17 (NIIT) [OTS]                         342        342  OK  agree-ots
8960 L17 (NIIT) [taxcalc]                     342        342  OK  agree-taxcalc
── witness census ── 11 of 15 compared lines have TWO independent engines behind them.
   single-witness lines: 1040.line19, 1040.line24, 1040.line27, 8995.line12   (all four NAMED)
```

**The kill and its observed red** — remove `1040.line24`'s mechanism from `SINGLE_WITNESS_REASON`:

```
     1040.line24            witness: OTS      — *** NO MECHANISM ON FILE — see below ***
DIVERGENCES:
  · the witness census has no mechanism on file for 1040.line24 (witness: OTS). Either the line lost
    a witness it used to have (find out why — that is a real finding), or the other engine does model
    it and the harness is not asking. Do not add a default string: …
EXIT=1
```

`--selftest` case 9 pins the key set offline, so the same plant reds with no OTS and no taxcalc. And
the C-2 plant above is the *other* half of this kill, live: a line that quietly LOSES a witness is
reported as unnamed rather than passing as "modelled by one engine".

---

## I-4 (Important) — HoH / MFS / QSS projected a row that panicked three steps later

**What changed.** `golden_filing_status_token(FilingStatus) -> &'static str` is an **exhaustive,
`_`-free** match onto the five tokens BOTH drivers already agree on — OTS's `Status` field and
`gen_goldens.TAXCALC_MARS`'s keys: `Single`, `Married/Joint`, `Married/Sep`, `Head_of_House`,
`Widow(er)`. `project_to_golden` emits through it; `golden_filing_status` is its inverse, DERIVED by
emitting each status's own word rather than retyped, and `build_golden_return` maps all five through
it. A sixth `FilingStatus` variant fails to COMPILE until a human gives it a token.

**Where.** `testonly.rs` — `golden_filing_status_token`, `GOLDEN_FILING_STATUSES`,
`golden_filing_status`, and the two call sites that used to hand-list `Single`/`Mfj`.

`income project` no longer needs a refusal path for a status: all five project and all five
round-trip. (It refuses on a year with no full-return package instead — see Deviations.)

**The kill and its observed red** — `every_filing_status_round_trips_through_the_oracle_row` (new)
round-trips all five and pins the token set; plant one status back to its `Debug` name:

```
thread 'every_filing_status_round_trips_through_the_oracle_row' panicked:
assertion `left == right` failed
  left:  {"HoH", "Married/Joint", "Married/Sep", "Single", "Widow(er)"}
 right:  {"Head_of_House", "Married/Joint", "Married/Sep", "Single", "Widow(er)"}
```

The routing probe reproduced the original defect independently while it was being built: at HEAD it
died on `unmapped filing status "Mfs"` inside `build_golden_return`.

---

## M-1 — the ledger census was a hand list of two

**What changed.** `LedgerSink` + `ledger_income_sink(kind, business)` — an `_`-free match over
`(IncomeKind, business)` into `ScheduleCProjects` / `Schedule1Line8vReported` / `RefusedUpstream`
(`RefuseReason::BusinessInterestIncome`). `unprojected_ledger_lines` classifies every income record
through it, so a sixth `IncomeKind` cannot be silently omitted — it fails to compile until it is given
a sink. (A stray broken line-continuation in the crypto-donation note was fixed while there.)

**What it does NOT derive, stated in the doc comment and as FR-96.** The other two ledger channels
are named rather than walked: `state.disposals` (→ Schedule D → the row's capital-gain fields, so they
project) and `state.removals`' `claimed_deduction` (→ Schedule A line 12, reported). `LedgerState`
has no type-driven leaf walk the way `ReturnInputs` does, so a NEW top-level ledger channel would not
red.

**The kill and its observed red.** The KAT now plants one record for every `(kind, business)` pair,
asserts each sink does what it claims, and asserts all three sinks are REACHED (a classification
nobody exercises is green forever — FR-88). Plant: route hobby staking to `ScheduleCProjects`:

```
thread 'the_ledger_lines_the_oracle_row_cannot_carry_are_reported' panicked at :353:
  left: 0
 right: 1
```

## M-2 — `--year` silently overrode the wrapper's `tax_year`

`--year` now has **no default**: it takes the projection's own `tax_year`, and an explicit
disagreement REFUSES (exit 2) rather than driving Tax-Calculator on one year's law while btctax's
harness and OpenTaxSolver 2024 stay on another. Observed:

```
$ check_return.py --file big2_row.json --year 2025
--year 2025 contradicts the projection's own tax_year 2024. The engines would be driven on different
law from btctax and each other … and the divergence would be fabricated. …
EXIT=2
```

`--selftest` case 8 pins `read_row`'s year handling offline (a bare row carries `None`, so `--year`
cannot contradict a year that was never stated).

## M-3 — seam 6's §G-9 statement is written down

`ORACLE_INVISIBLE`'s doc comment now carries it, generalised to the gates: *"NOTHING BELOW IS
VALIDATED BY ORACLE AGREEMENT, AND NEITHER ARE THE GATES."* Every fact the table names is either
absent from the description (so no engine has an opinion) or handed to the drivers AS AN INPUT — the
filing status, the dependents block, the aged/blind boxes, the itemize election. What validates the
gates is R2's transcription checks; a reader must take no comfort from a green run about any answer
btctax put INTO the row. C-1 is why it is worth writing: the round trip answers gates for the filer.

## N-1 — the `[taxcalc]` rows printed `class: agree-ots`

`verdict_ots` became `verdict_engine(line, label, engine, …)`: the class is `agree-taxcalc` for a
taxcalc leg, and the verdict carries an explicit `"engine"` field. `check_return.py`'s census now
reads that field instead of looking for the substring `"[taxcalc]"` in a human label — a census that
reads a display string is one relabelling away from miscounting the thing it exists to count.
Confirmed live: `deduction (L12) [taxcalc] … OK agree-taxcalc`.

---

## Oracles — both re-run, neither baseline moved

No Python driver logic changed (`ots_direct.py` and `gen_goldens.py` are untouched by this fold), and
the measurements confirm it rather than arguing it:

```
$ .venv/bin/python scripts/oracle/check_determinism.py
PASS: fresh-regen and committed are byte-identical over 107 households
      (excluding `_provenance.generated`) — §12 determinism holds.
```

That regenerates the whole corpus from **both** engines and compares byte for byte, which is the
strongest available statement. Also, separately:

```
gen_goldens.assert_baked_provenance_is_current()  -> baked provenance: current
taxcalc over 107 committed households: moved fields = 0
OTS over 24 committed households (12 anchors + 12 itemized): 383 numeric fields compared, moved = 0
$ .venv/bin/python scripts/oracle/ots_direct.py     -> ots_direct: defect-year scoping OK
$ .venv/bin/python scripts/oracle/sweep.py --seed 1 --count 8
[sweep] seed=1 count=8: 8 admitted, 0 skipped (out of domain). 0 suppressed known-defect(s).
0 undeclared divergences
```

The sweep drives the harness's `--check`, so it exercises the three NEW `[taxcalc]` legs on live
generated scenarios; `btctax-oracle-harness::smoke
check_mode_reconciles_every_line_of_the_anchors_and_pinned_cells` does the same over the 12 anchors
and the §5.1 pinned cells and is green in `make check`.

No figure in this report rests on one engine.

---

## Deviations

1. **I-1 is REPORTED, not carried.** The review offered either; the §170(b) ceiling that OTS 2024
   does not apply makes carrying it a source of false divergences. Reasoned above.
2. **`project_to_golden` gained a third parameter** — `&AbsoluteReturn`. R13's signature was already
   deviated to `(ri, state)` at T11; this adds the assembled return so the projection can read
   btctax's own line-12 decision (C-2) and the FILED Schedule A rather than re-deriving it. It is not
   extra work at the call site: C-1 makes `income project` compute the return anyway, for
   `screen_absolute`.
3. **The AMOUNT probe compares `GoldenInputs::money_view()`** — the row with the routing facts
   normalised away. Without it, perturbing `schedule_a.medical` to $777,777 flips 1040 line 12 to the
   itemized branch, moves `standard_or_itemized`, and would make medical read as "visible" though not
   one dollar of it reaches an oracle box — turning its correct `ORACLE_INVISIBLE` entry into a false
   "stale exemption". The routing partition asks the right question about that leaf instead.
4. **The fidelity half of the routing probe runs on purpose-built fixtures, not the maximal
   sentinel.** A difference-of-differences measure on a fixture that already carries uncarried figures
   blames the wrong leaf (observed: `filing_status` flagged because it changed what the uncarried
   spouse-death fact was worth). The presence half keeps the sentinel's breadth.
5. **`check_return.py` now exits 2 on every "could not be made" path**, not only the refusal. The
   docstring has promised that since T11 and every such path exited 1. Beyond the brief's letter;
   without it the C-1 fix would have been indistinguishable from a divergence.
6. **`income project` on a year with stored inputs but NO full-return package now refuses** with
   `year_readiness`'s own sentence, where it used to print a row. Forced by (2) — `assemble_absolute`
   needs the year's params — and correct on its own terms: OTS 2024 and the harness's `YEAR = 2024`
   could not have been driven with such a row anyway. A year with no stored inputs still prints
   *"No full-return inputs set for tax year N."* as before (verified on TY2025 both ways).
7. **`form_1098[].box6_points`, `w2s[].box17_state_tax_withheld` and `w2s[].box19_local_tax` were
   REMOVED from `ORACLE_INVISIBLE`,** and `header.taxpayer_died_during_year` /
   `header.spouse_died_during_year` added. Not asked for; forced by the routing probe and by reading
   the filed Schedule A. Each removal's note asserted something the code contradicted.
8. **A fourth `InvisibleBecause` variant, `RoutingFactNotCarried`.** The enum's own doc says the cases
   are not equally benign and a flat list would let the worst hide among the harmless; a routing fact
   is a fourth kind whose failure mode looks like none of the other three.
9. **FR-94, FR-95, FR-96 filed** (`FOLLOWUPS.md`), each with an owning phase and a measured cost.

---

## Instruments — all five, unchanged from HEAD

```
$ cargo run -q -p xtask -- line-coverage
line-coverage OK: 375 money lines across 18 form(s) [f1040:45 f1040s1:12 f1040s1a:50 f1040s2:9
f1040s3:5 f1040sa:21 f1040sb:5 f1040sc:7 f1040sd:31 f1040sse:22 f6251:41 f8889:27 f8949:12 f8959:17
f8960:15 f8995:16 f8995a:39 i1040gi:1], 31 exception(s) (ratchet 31), 0 unverifiable (ratchet 0),
17 not line-bound (ratchet 17)

$ cargo run -q -p xtask -- census-join
census join: 274 unmodeled entries across 13 maps, every one placed by a direction block asserted
against the form's extract, graded by one of DIRECTION_OF_CAPTION's 22 readings (each cited to a
sentence the form prints) and covered by an existing variant

$ cargo run -q -p xtask -- stop-list
R15 stop list: 8 btctax-input-form sources, 4 state-bearing sources and 91 registry prompts scanned;
no forbidden shape

$ cargo run -q -p xtask -- prompt-check
xtask prompt-check: OK — 88 assertions, all verbatim

$ cargo run -q -p xtask -- box-census
box-census OK: 268 printed boxes across 19 archived editions of 9 information returns, every one
decided (268 entries)
```

## Closing gate

```
$ cargo fmt --all                                            # clean
$ CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings
                                                             # no output, exit 0
$ find crates -name '*.rs' -exec touch {} +
$ make check
     Summary [  20.888s] 3510 tests run: 3510 passed, 12 skipped        (exit 0)
```

`git diff --stat`: **13 files changed, 1491 insertions(+), 194 deletions(-)**. Nothing committed,
nothing pushed.

## Files touched

`crates/btctax-core/src/tax/{testonly.rs,return_1040.rs}`,
`crates/btctax-core/tests/oracle_projection.rs`,
`crates/btctax-cli/src/{cli.rs,cmd/tax.rs}`, `crates/btctax-cli/tests/tax_profile.rs`,
`crates/btctax-forms/tests/attestation.rs`, `crates/btctax-oracle-harness/src/main.rs`,
`scripts/oracle/{check_return.py,README.md}`, `docs/man/btctax-income-project.1` (regenerated),
`docs/man/btctax-income.1` (regenerated), `FOLLOWUPS.md`.
