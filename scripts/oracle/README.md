# `scripts/oracle/` — the two-engine validation path

btctax's figures are checked against **two engines that share no lineage**, never one:

| oracle | engine | driver | licence |
|---|---|---|---|
| 1 | **OpenTaxSolver**, its own binaries driven directly | `ots_direct.py` | GPL-2.0, observe-only |
| 2 | **PSL Tax-Calculator** | `gen_goldens.py` | CC0 |

A single oracle disagreeing is ambiguous; two splitting is diagnostic. **A disagreement is
adjudicated against the FORM, never encoded** — the IRS PDF is the authority and an engine is a
witness (precedent: tenforty #278/#279, where OTS was never wrong and the wrapper was).

## Setup

```sh
export OTS_DIR=/path/to/OpenTaxSolver2024_22.07_linux64   # unset by default
cargo build -p btctax-oracle-harness                       # the §9 harness every driver runs
```

The Python stack is the repo's `.venv` (`taxcalc`, `pandas`) — bare `python3` has neither.

## The scripts

| script | what it does |
|---|---|
| `corpus.py` | the covering-array household corpus (12 anchors + generated cells) |
| `gen_goldens.py` | regenerates `crates/btctax-core/tests/goldens/full_return_goldens.json` from both engines |
| `ots_direct.py` | drives OpenTaxSolver form by form; also `python3 ots_direct.py` for its offline self-check |
| `sweep.py` | the live, seeded, threshold-biased **divergence hunt** over generated scenarios |
| `verify_f6251.py` | the Form 6251 (AMT) line-by-line diff, both engines, 30 vectors |
| `verify_schedule_1a.py` | the TY2025 Schedule 1-A diff |
| **`check_return.py`** | **runs both engines on YOUR OWN return** — see below |
| `check_determinism.py` | pins the goldens' regeneration determinism |

## `check_return.py` — the oracle path for a real return (SPEC_interview.md R13)

Everything above validates the built-in **corpus**. `check_return.py` validates the return you are
about to file. `btctax income project` prints your return as the household description both engines
take — a `GoldenInputs` row, which carries **no identity by type**: it has no name, SSN, address,
employer or payer field to put one in.

```sh
export OTS_DIR=/path/to/OpenTaxSolver2024_22.07_linux64
cargo build -p btctax-oracle-harness
btctax income project --year 2024 > /tmp/row.json
.venv/bin/python scripts/oracle/check_return.py --file /tmp/row.json
```

Exit 0 = every compared line reconciles (or diverges by exactly its computed excuse); 1 = a
divergence; 2 = the run could not be made — no OTS, a stale harness, or **a return btctax refuses to
compute**. `--selftest` runs the offline B1 kill (no OTS, no taxcalc, no vault) and watches the
credit-line verdict tell a blank from a sworn figure, the refusal block survive parsing, and the
`--year` guard hold.

`--year` **defaults to the projection's own `tax_year` and refuses to contradict it**: driving a 2024
projection with `--year 2025` put Tax-Calculator on 2025 law while btctax's harness and
OpenTaxSolver 2024 stayed on 2024, and fabricated a divergence on 1040 line 15.

**What is checked is the DESCRIPTION, not the packet.** Every column — including btctax's own — is
computed from the projected row by `build_golden_return`, so a figure the row cannot carry is
missing from all three columns and produces silence rather than a disagreement. `build_golden_return`
is a fixture builder: it answers every live declaration and both Schedule B Part III questions. That
is right for the corpus and wrong for a filer, so the REFUSAL is taken where the filer's own inputs
are in hand — `income project` runs the same screens `btctax report` runs and emits a `"refused"`
block, and this script exits 2 on it rather than reporting success for a return that does not file.

Three things it prints that are easy to skip past and are the whole point:

* **The excuses are COMPUTED, not named — and a BLANK is not a `0`.** 1040 line 19 is blank on a
  btctax return (there is no Schedule 8812), so the expected gap is *exactly* Tax-Calculator's own
  `c07220 + odc` for this household; line 27 likewise, against `eitc`. A gap of any other size
  FAILS, including an off-by-one. Never enumerate the outcomes you happened to see. **But
  `advisories.rs::ctc_odc_line19` really does print a cell — `Some(Usd::ZERO)` — whenever
  `ctc_provably_zero` fires**, and a printed cell is testimony under §6065, not a forgone line: it
  must EQUAL the oracle's figure or it diverges. FR-85 is why this matters and is not hypothetical —
  the §24(h)(2) per-child ceiling is a year-blind $2,000 against TY2025's $2,200, so when that
  package lands the proof will conclude *"provably zero"* for a household that still has credit and
  swear a `0`. This check is the one instrument that can catch it, and the `--selftest` case named
  for FR-85 runs that exact plant offline forever.
* **The witness census.** OpenTaxSolver reads `L19`, `L27` and `L28` with `GetLine(...)` — they are
  *inputs* — and parses `Dependents` into `NumDependents` without ever using it, so it computes no
  credit at all. Its "agreement" with btctax on line 19 is `0 == 0` between two engines neither of
  which computed anything: `FOLLOWUPS.md` §G-9, *a value the oracles take as INPUT is never
  validated by their agreement*. The census counts engines per line and names every single-witness
  line with the mechanism — and an **unnamed** single-witness line is now an ERROR. The default
  string it used to fall back to (*"only one engine models this line"*) was false for four lines
  whose second engine's figure was already in the same process; a census that guesses at why cover
  is missing destroys the one thing it exists to report.
* **`not_carried` and `not_carried_from_the_ledger`.** The oracle row is the corpus's household model and is smaller than the 1040. A
  filer with medical expenses, student-loan interest or a prior-year capital-loss carryforward is
  described to both engines *without* them, so a divergence on such a line is the description's, not
  btctax's. Every non-projecting `Usd` leaf is named with its reason in
  `btctax_core::tax::testonly::ORACLE_INVISIBLE`, whose completeness is asserted by
  `crates/btctax-core/tests/oracle_projection.rs` in **two dimensions**. The AMOUNT partition
  perturbs each money leaf and watches the projection, so a money field added tomorrow with neither
  a projection nor a reason reds it. The **ROUTING** partition perturbs every *non*-money leaf to
  every alternative its TYPE admits (serde's own `unknown variant` list; `true`/`false` for a
  boolean) and asks whether the household the row describes still reproduces the return btctax
  files — because a fact that decides *where money goes* without being money is invisible to a probe
  that only watches figures. A charitable gift's §170 class and the itemize election were both
  exactly that. The **ledger** is censused separately
  (`return_1040::unprojected_ledger_lines`) because it is not a `ReturnInputs` leaf and would
  otherwise be the one part of the crypto side with no completeness check at all: non-business
  crypto ordinary income (Schedule 1 line 8v) and crypto donations (Schedule A line 12) reach the
  printed return and cannot reach the row.

**A known non-excuse.** If the §G-9 death question is unanswered, btctax FORGOES the §63(f) age-65
addition (class (B): silence forgoes) while both engines grant it, and 1040 line 12 comes up short by
a multiple of the year's addition. That is a real, actionable finding — answer the question and the
deduction appears — so it is left to fail rather than excused.
