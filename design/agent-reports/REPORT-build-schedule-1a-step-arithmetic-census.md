# REPORT — the Schedule 1-A APPLIED-RESULT census: the step arithmetic, two oracles, and the kills

**Built 2026-09-11 by one opus agent in the shared main tree at `d4114a02`. Nothing committed.**
Brief: `design/agent-reports/BRIEF-build-schedule-1a-step-arithmetic-census.md`.

> ★★ **PROVENANCE OF THIS FILE — read before citing it as an agent-written report.** The building agent's
> harness **refused its report write** (`Write` returned *"Subagents should return findings as text, not
> write report files"*), so it returned the report inline and asked the coordinator to persist it. This copy
> is therefore a **controller transcription**, not an agent-written artifact, and the
> responder-must-not-be-the-scribe guarantee in `/scratch/code/CLAUDE.md` is **not** satisfied for it. The
> byte-exact original is the agent's task output. Two deliberate transcription edits: HTML entities in the
> notification channel (`&lt;` / `&gt;`) were restored to `<` / `>` in `taxsolve_US_1040_<year>` and
> `Option<Usd>`. Filed as FR-129.

## 0. Outcome in one paragraph

The census now compares **computed deductions**, not only constants. 138 boundary vectors — derived from the
parameters, not typed — run against **both** engines across every year in `SCHEDULE_1A_YEARS`, and the run is
green with **0 unexpected divergences**. It found three real OpenTaxSolver 2025 Part IV defects and
re-derived Tax-Calculator's QSS threshold defect from the parameter itself, and those two engines' blind
spots **overlap**: 4 of 6 Part IV/QSS vectors have no witness at all. Thirteen planted defects were each
watched RED and restored — the three `StepRounding` flips, a Part V fake stair on both sides, a
mis-transcribed cap, the real-world inert-`exact` mistake, and both directions of the new SALT checker. Two
premises in the brief were refuted by measurement.

## 1. What was built, and where

| file | change |
|---|---|
| `scripts/oracle/verify_schedule_1a.py` | +887/−11 lines. The existing per-part CONSTANTS census is untouched in behaviour (now `_parameter_census`); a second **applied-result census** was added beside it. |
| `crates/btctax-core/tests/schedule_1a_compute.rs` | +277 lines. Two new tests: `the_step_arithmetic_matches_the_oracle_census` (Parts II/III/IV) and `part_v_is_smooth_and_a_planted_stair_would_show_here`. |
| `scripts/oracle/corpus.py` | +68/−4 lines. `salt_axis_reachability()` + `SALT_YEARS_NOT_REACHABLE`, wired into `selftest_salt_axis()`. |
| `FOLLOWUPS.md` | FR-124 … FR-128, each with an owning phase. |

Run it with `OTS_DIR=~/OpenTaxSolver2025_23.06_linux64 OTS_YEAR=2025 .venv/bin/python scripts/oracle/verify_schedule_1a.py`. Without `OTS_DIR` it still runs and says loudly that every vector rests on one oracle (verified).

### The one architectural decision worth stating

There is **one** implementation of the form's arithmetic, `_deduction()`, called three ways: with the form's
printed figures, with Tax-Calculator's policy parameters (and its unrounded line 34), and with
OpenTaxSolver's floored line 28 / $300 line 29 / unrounded line 34. Each engine is then held to **two
separate questions, neither excusing the other**:

1. did it do what its own code and parameters predict? If not → **UNEXPECTED**, a finding.
2. does that prediction differ from the FORM? If so the vector is **disqualified**, and a mechanism must say
   why. A prediction that differs from the form with **no** mechanism to explain it fails the run — because
   that means our list of mechanisms is incomplete, which is the dangerous state.

★ **That shape was forced by writing the kill, not chosen.** The first draft computed the form's expectation
and each engine's prediction in two separate functions. A planted fake stair in Part V then left every
engine prediction correct, the run stayed **green**, and only the witness census went to nonsense. Two
implementations of one arithmetic are two chances to be wrong, and the plant only mutated one of them. B1
behaving exactly as advertised: *an honest kill-test for a blind checker cannot be written without
discovering the blindness.*

## 2. The vector set, and how it was derived

138 vectors = `parts × FilingStatus × offsets`, with **no hand-typed MAGI anywhere**. The run prints its own
shape:

```
THE APPLIED-RESULT CENSUS — 138 boundary vectors × TY2025-2028
  60 of 138 vectors sit at a FRACTIONAL step, so they tell floor from ceil apart.
  vectors per part: II=30, III=30, IV=30, V=48
```

- **MAGI** = that status's own printed threshold (`threshold_for`) + a derived offset.
- **stepped offsets** (Parts II/III/IV), from `(cap, step, per_step, direction)`: `0` (the form's JUMP) ·
  `+$1` · `+step/20` (**the form's own example**, *"decrease 0.05 to 0"* vs *"increase 0.05 to 1"*) ·
  `+1.5 steps` (**its other example**) · `exhaustion − $1` · `exhaustion`.
- **exhaustion is per-direction**, which is why it is computed and not typed: a flooring part exhausts at
  `(cap/per_step)×step`; a ceiling part exhausts one dollar past the last full step. Part IV's last live
  vector is therefore an excess of **$49,000 carrying $200**, and $49,001 is the first zero — matching
  `StairStepPhaseOut::exhaustion_excess`'s own doc comment.
- **Part V offsets** (smooth) come from the rate: exhaustion is `per_person / rate = $100,000`, and the
  fake-stair probes (`+step/2`, `+1.5×step`) take the **$1,000 from the stepped parts** rather than a typed
  literal, because that is the step a planted stair would plausibly use. `+$25` is there because
  `0.06 × 25 = $1.50` is the only place line 34's whole-dollar rounding is visible.
- **statuses**: all five, every part, no exceptions typed — MFS included, so the form's asymmetry (Parts
  II/III/V barred, **Part IV allowed**) is measured rather than asserted. The Rust side derives its status
  set from `FilingStatus::ALL`, itself held by an `_`-free match.
- **years**: every year in the window, plus the first year after it:

```
  TY2026: taxcalc reproduces all 138 vectors — identical to TY2025
  TY2027: taxcalc reproduces all 138 vectors — identical to TY2025
  TY2028: taxcalc reproduces all 138 vectors — identical to TY2025
  TY2029: every vector deducts 0.00 — OK. §§224(f)/225(f)/163(h)(4)(F)/151(d)(5)(D) expire after TY2028,
          so taxcalc independently witnesses btctax's `None`.
```

### TY2026, stated as the brief asked

`SCHEDULE_1A_YEARS` is `2025..=2028` (`tables.rs:1215`) and no `FullReturnParams` is involved, so btctax
computes Schedule 1-A for **TY2026 today** and this is the arithmetic TY2026 will file with. Measured
addition: the **TY2026 DRAFT form renumbers the lines** (the $25,000 cap moves from line 7 to line 9, the
divide from 11 to 13, the car-loan ceil from 28 to 34) while **every constant and both rounding directions
are unchanged** — `design/forms/extract/f1040s1a--2026-DRAFT.txt`. The arithmetic transfers; only the
numbering moves, which is why the census is year-spanning and a transcription struct stays per line-set
revision.

## 3. Per-part witness counts, and the disqualifications (all computed)

```
── witness census, per part × filing status ──
    Part II  hoh/mfj/mfs/qss/single   6 vectors each: 6 with TWO independent witnesses, 0 with NONE
    Part III hoh/mfj/mfs/qss/single   6 vectors each: 6 with TWO independent witnesses, 0 with NONE
    Part IV  hoh       6 vectors: 2 with TWO independent witnesses, 0 with NONE
    Part IV  mfj       6 vectors: 2 with TWO independent witnesses, 0 with NONE
    Part IV  mfs       6 vectors: 1 with TWO independent witnesses, 0 with NONE
  ★ Part IV  qss       6 vectors: 1 with TWO independent witnesses, 4 with NONE
    Part IV  single    6 vectors: 2 with TWO independent witnesses, 0 with NONE
  ★ Part V   hoh       8 vectors: 5 with TWO independent witnesses, 3 with NONE
  ★ Part V   mfj      16 vectors: 10 with TWO independent witnesses, 6 with NONE
    Part V   mfs       8 vectors: 8 with TWO independent witnesses, 0 with NONE
  ★ Part V   qss       8 vectors: 5 with TWO independent witnesses, 3 with NONE
  ★ Part V   single    8 vectors: 5 with TWO independent witnesses, 3 with NONE

  59 of 72 phase-out-exercising vectors have at least one witness; every part has one, so no part is
  reported on unmeasured.
  unwitnessed, by part: IV=3, V=10
```

**Every disqualification is computed from a mechanism, none keyed by vector name.**

| engine | mechanism | how it is derived | consequence |
|---|---|---|---|
| taxcalc | any policy parameter differing from the printed figure | read from `policy_current_law.json` per status, then the arithmetic is re-run with **taxcalc's own** constants | reaches any future parameter defect, not only the QSS one it was written against |
| taxcalc | line 34 carries the **unrounded** `excess × 0.06` | `calcfunctions.py` read; the predicted value is exact, so a divergence of the wrong shape is still a finding | Part V half-dollar vectors |
| taxcalc | smooth fallback instead of the stepped branch | compared against `_smooth_fallback()` on every fractional-step vector | see kill 10 |
| OTS | line 28 floored (`int j`, `:1888`) | **re-verified from OTS's own printed `S1A_28`** on every vector | Part IV stepped |
| OTS | line 29 multiplies by **$300** (`:1895`) | reproduces OTS's printed line 30 exactly, or the run reds | Part IV stepped |
| OTS | Parts II–V all inside `if (status != MFS)` (`:1823`) | the part's printed **threshold line** is absent though the cap is positive — `showline_wlabelnz` suppresses zeros, so the threshold is what tells "computed 0" from "never computed" | Part IV/MFS |
| OTS | line 34 unrounded (`:1909`) | same sizing as taxcalc's | Part V half-dollar |
| OTS | **no solver binary for the year** | `OTS_DIR/bin/taxsolve_US_1040_<year>` existence — a year-scoped disqualification computed from the install, printed for TY2026/27/28 | no OTS witness outside TY2025 |

★ **The two engines' blind spots overlap, and that is the finding the census exists for.** On Part IV for a
qualifying surviving spouse, taxcalc uses the MFJ $200,000 threshold *and* OTS both floors a ceiling line and
multiplies it by $300. Sample rows (form / taxcalc / OTS):

```
    qss         100,001   0   9,800.00  10,000.00  10,000.00  +$1 over   taxcalc DISQUALIFIED … | OTS DISQUALIFIED …
    qss         149,000   0     200.00  10,000.00       0.00  $1 before exhaustion   (both disqualified)
    single      101,500   0   9,600.00   9,600.00   9,700.00  +1.5 steps ($1,500)   OTS DISQUALIFIED …
    mfs         100,001   0   9,800.00   9,800.00       0.00  +$1 over   OTS DISQUALIFIED — never runs Part IV for MFS
```

That $9,700 is OTS's two Part IV defects partially cancelling: floor(1.5)=1 step at $300 instead of
ceil(1.5)=2 steps at $200. Both are new findings, both upstream-reportable on **two** independent
authorities (the form's text layer and taxcalc), both filed as FR-125.

## 4. The kills — thirteen planted defects, each watched RED and restored

Every mutation reverted with a `cp` backup; `diff -q` confirmed byte-identical restoration.

### Rust — the three `StepRounding` flips (the point of the task)

| # | plant | result |
|---|---|---|
| 1 | `tips_phase_out.rounding` **Floor → Ceil** | RED |
| 2 | `overtime_phase_out.rounding` **Floor → Ceil** | RED |
| 3 | `qpvli_phase_out.rounding` **Ceil → Floor** | RED |

```
1) assertion `left == right` failed: Part II line 13/Single @ MAGI 150001 (excess 1, threshold 150000)
     left: Some(24900)   right: Some(25000)
2) assertion `left == right` failed: Part III line 21/Single @ MAGI 150001 (excess 1, threshold 150000)
     left: Some(12400)   right: Some(12500)
3) assertion `left == right` failed: Part IV line 30/Single @ MAGI 100001 (excess 1, threshold 100000)
     left: Some(10000)   right: Some(9800)
```

GREEN after each restore: `test result: ok. 15 passed; 0 failed`.

### Rust — Part V's planted stair, and the KAT's own discriminating power

| # | plant | result |
|---|---|---|
| 4 | `Schedule1aPartV::compute`'s line 34 replaced by a $60-per-$1,000 **stair** (same average slope) | RED |
| 5 | the test's `opposite()` made the identity, so nothing distinguishes the directions | RED |

```
4) assertion `left == right` failed: Part V line 35/Single @ MAGI 75025 (excess 25)
     left: Some(6000)   right: Some(5998)
5) Part II line 13: not one of these offsets distinguishes Floor from Floor, so nothing here holds the
   rounding direction
```

Kill 5 is the KAT asserting its **own** ability to fail: for each part it constructs the flipped phase-out
and requires at least one offset where the two directions disagree.

### Python — the same flips, from the oracles' side

| # | plant | result |
|---|---|---|
| 6 | `FORM_1A["II"]["direction"]` floor → ceil | RED, exit 1, **48** unexpected |
| 7 | `FORM_1A["III"]["direction"]` floor → ceil | RED, exit 1, **48** unexpected |
| 8 | `FORM_1A["IV"]["direction"]` ceil → floor | RED, exit 1, **16** unexpected |
| 9 | Part V fake stair inside `_deduction` | RED, exit 1, **60** unexpected |

```
6) single 150,001  form 24,900.00  taxcalc 25,000.00  OTS 25,000.00  +$1 over
     ★ UNEXPECTED taxcalc: predicted 24900.00, got 25000.0 | ★ OTS printed S1A_11 = 0; the mechanism
     above predicts 1 | ★ UNEXPECTED OTS …
8) single 100,001  form 10,000.00  taxcalc 9,800.00  OTS 10,000.00  +$1 over
     ★ UNEXPECTED taxcalc: predicted 10000.00, got 9800.0
9) single  75,025  form 6,000.00  taxcalc 5,998.50  OTS 5,998.50  +$25 (0.06 × excess on a half-dollar)
     ★ UNEXPECTED taxcalc: predicted 6000, got 5998.5 | ★ UNEXPECTED OTS: predicted 6000, got 5998.5
```

Note kill 8: only taxcalc catches a flipped Part IV direction, because OTS is already disqualified there.
That is precisely why the witness count is printed per part **and** per status.

### Python — the instrument's own guards

| # | plant | result |
|---|---|---|
| 10 | `exact` passed as a Records DataFrame column instead of through the Calculator — **the real mistake `gen_goldens.py` makes** | RED, exit 1, **48** `SMOOTH value` notes, 96 total |
| 11 | Part IV cap mis-transcribed as $11,000 in both transcriptions | RED, exit 1, the phase-out-witness **gate fires** |

```
10) single 150,001  form 25,000.00  taxcalc 24,999.90  OTS 25,000.00  +$1 over
      ★ UNEXPECTED taxcalc: predicted 25000.00, got 24999.9
      ★ taxcalc returned the SMOOTH value — the stepped branch never ran
11) FAIL: Part(s) IV have NO witnessed vector that exercises the phase-out — this census would be
      reporting on arithmetic no engine confirmed.
```

★ **Kill 11 took three goes and improved the gate twice, which is the whole value of doing it.** The gate's
first form asked only for "some witnessed vector in this part" and the plant could not make it fire, because
`max(0, cap − reduction)` collapses to zero at exhaustion no matter how wrong the cap is — so an exhausted
vector stayed "witnessed" while every figure that depends on the arithmetic had stopped being witnessed.
Narrowing it to *vectors that exercise the phase-out* was not enough either: at exhaustion the excess **is**
fractional, so "discriminating" was still true. It fires only once the gate also requires a **live
(non-zero) figure**. (An earlier attempt was caught first by `_transcription_tie()` — a legitimate but
earlier red — which is why the final plant mutates both transcriptions.)

### corpus.py — the SALT reachability checker, both directions

| # | plant | result |
|---|---|---|
| 12 | delete TY2025's `SALT_YEARS_NOT_REACHABLE` entry | RED, exit status 1 |
| 13 | declare TY2024 (which *is* reachable) dormant | RED, exit status 1 |

```
12) AssertionError: TY2025's SALT axis cell(s) ['over'] reach NO household in `households()`, and
    TY2025 is not listed in SALT_YEARS_NOT_REACHABLE. Either wire the builder to `salt_for(2025)` or
    state the boundary there with its measurement — an axis that runs nowhere still reports coverage.
13) AssertionError: TY2024's SALT axis is now fully reachable, so its SALT_YEARS_NOT_REACHABLE entry
    (PLANTED) is stale — delete it. A dormancy note kept past its dormancy is an excuse nobody
    re-measures.
```

GREEN after restore: exit status **0**, `candidates: 107`.

## 5. The SALT answer, with its measurement

**No, the TY2025 SALT axis cannot execute — and the brief's stated reason is not the proximate one.** Three
measurements, in the order that matters:

1. **The builder never sees it.** `_build` reads the year-blind module constant `SALT` (the TY2024 axis), not
   `salt_for(year)`, and `households()` takes **no year parameter at all**. So `SALT_BY_YEAR[2025]` is
   consumed by `salt_for`/`selftest_salt_axis` and by nothing else. Measured: of the **107** households
   `households()` assembles, **0** carry the TY2025 `over` cell (25,000 + 20,000). The distinct
   (state_income_tax, real_estate_tax) pairs actually built are **(1,068 · 10,509), (3,000 · 4,000),
   (8,000 · 9,000)** — the last being TY2024's `over`. The TY2025 `under` cell *is* built, but only because
   it is byte-identical to TY2024's, which is coincidence rather than reachability. (Hence the new checker
   requires **every** cell of an axis.)
2. **The driver is TY2024 too.** `gen_goldens`'s `taxcalc_run` / `taxcalc_credits` / `_taxcalc_amt_credits`
   all default to `year=2024` with no call site passing anything else; the emitted golden stamps
   `"tax_year": 2024`; `sweep.py` is pinned to TY2024 constants (`OASDI_BASE = 168_600`,
   `STD_DEDUCTION_2024`).
3. **And btctax could not take it.** `full_return_for(2025)` is a tested `None` —
   `ty2025_full_return_must_stay_fail_closed_until_complete` in `crates/btctax-adapters/src/tax_tables.rs`.

So the TY2025 entry is **correct and dormant**, not wrong: it is the cap figure the day a TY2025 corpus
exists. What was missing is the boundary being *stated*, because a dormant axis reads exactly like a live
one. The fix is in source and it is **measured, not declared**: `salt_axis_reachability()` walks
`households()` and reports which cells reach nothing, and `selftest_salt_axis()` reds in **both** directions.
`SALT_YEARS_NOT_REACHABLE[2025]` carries the reason. **No `FullReturnParams` was bundled**; that gate is the
owner's.

## 6. Refuted premises

1. **`schedule_1a_table_matches_the_oracle_census` does not exist and never has.** The brief asked for a KAT
   "mirroring the existing" test of that name, quoting `verify_schedule_1a.py`'s own header comment.
   Measured: `grep -rn 'schedule_1a_table_matches_the_oracle_census' --include='*.rs' .` → **zero matches**;
   the only hits in the repo are the brief and that comment. The constants are actually pinned by
   `tables.rs::schedule_1a_tests::every_constant_matches_its_printed_line`. The header comment is now
   corrected to name the real tests, with the measurement recorded in it — a citation nobody can follow is
   the class `xtask cite-check` exists for.
2. **The SALT axis's blocker is the corpus builder, not the full-return gate.** `full_return_for(2025)` being
   `None` is true and is the *third* reason; the axis is already unreachable two layers earlier (§5).
   Reporting only the full-return gate would have suggested that bundling params makes the axis live, which
   it would not.
3. Confirmed, not refuted: the brief's citations `tables.rs:1059-1084` (`StepRounding`), `tables.rs:1215`
   (`SCHEDULE_1A_YEARS`) and the baseline `make gate` **3596 passed / 12 skipped** all check out.

## 7. Residue, with owning phases

All filed in `FOLLOWUPS.md` under *"From the Schedule 1-A step-arithmetic census (2026-09-11)"*:

- **FR-124** — `gen_goldens.py`'s `exact: 1` Records column is silently dropped, so
  `TAXCALC_EXACT_YEARS = {2025}` is inert and every stepped phase-out would take taxcalc's smooth fallback.
  Minor, no live effect today (the corpus is TY2024-only). *Owning phase: before any TY2025+ household is
  driven through the sweep — the TY2026 port.*
- **FR-125** — OTS 2025's three Schedule 1-A Part IV defects + the `S1A_20`-printed-twice reporting bug.
  Minor for us. *Owning phase: ownerless residue — batch with the next upstream report.*
- **FR-126** — taxcalc's `AutoLoanInterestDed_ps[widow]` = 200,000, and the resulting **zero-witness**
  Part IV/QSS region. Minor for us. *Owning phase: ownerless residue — batch with the next upstream report.*
- **FR-127** — neither engine rounds line 34, so btctax's printed figure has no witness at the half-dollar
  (10 of the 13 unwitnessed phase-out vectors). Minor, recorded blind spot; nothing to fix. *Owning phase:
  ownerless residue — record only.*
- **FR-128** — the TY2025 SALT axis dormancy, with the measurement. Minor, boundary now stated and held by a
  checker. *Owning phase: whenever a year-parameterised corpus is built — the TY2026 port.*

Nothing blocking is open. No scope was widened: no params bundled, no fail-closed gate touched, no draft
transcribed, and the `Option<Usd>` blank-versus-zero distinction is left where it lives (the census compares
the amount reaching line 38; the Rust suite holds the blanks).

## 8. The gate, as numbers

| check | result |
|---|---|
| `make gate` (final state) | **3598 tests run: 3598 passed, 12 skipped**, exit 0 — baseline was 3596/12, +2 for the two new tests |
| `cargo fmt --all -- --check` | clean, exit 0 (it failed once on two closures; `cargo fmt --all` applied, gate re-run after) |
| `cargo test -p btctax-core --test schedule_1a_compute` | 15 passed; 0 failed |
| census with both oracles | exit 0 — `OK: 0 unexpected divergence(s) across both censuses` |
| census with no `OTS_DIR` | exit 0, and it says every vector rests on one oracle |
| `.venv/bin/python scripts/oracle/corpus.py` | exit 0, `candidates: 107` |
| `git status --porcelain` | 4 files modified, all intended; no commits, no stash, no checkout |
