# T11 — the oracle path (R13): implementation report

Single implementer, shared main tree `/scratch/code/bitcoin_tax`, branch `main`, dispatch HEAD
`2a9e9478`. Nothing committed, nothing pushed; no `git checkout --` / `restore` / `stash` was used
(every plant was reverted from a `cp` backup and `touch`ed). No subagents.

**Closing gate — `make check`: `Summary [ 20.648s] 3506 tests run: 3506 passed, 12 skipped`.**
`cargo fmt --all` clean; `CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets
--all-features -- -D warnings` clean (no output). Every measurement below was taken after
`find crates -name '*.rs' -exec touch {} +` (FR-90).

---

## 1. Citation drift found in the brief

The brief's "Settled facts" block was measured at `1f3cc137`. Re-measured at the dispatch HEAD
`2a9e9478` **before** any edit of mine:

| brief's citation | actual at `2a9e9478` | drift |
|---|---|---|
| `GoldenInputs` at `testonly.rs:650-693` | `:1032` | **+382 lines** |
| `build_golden_return` at `testonly.rs:731` | `:1127` | **+396 lines** |
| `_taxcalc_row` at `gen_goldens.py:199-232` | `:199` | none |
| the OTS template fill at `ots_direct.py:135` | `:135` | none |
| `sweep.py:18` (`cargo build -p btctax-oracle-harness`) | `:18` | none |

The substance was correct in every case; only the two Rust anchors moved. After my edits the same
symbols are at `testonly.rs:1113` (`GoldenInputs`), `:1359` (`build_golden_return`), `:1782`
(`project_to_golden`), `:1936` (`ORACLE_INVISIBLE`); `gen_goldens.py:203` (`dependent_block`), `:251`
(`_taxcalc_row`), `:296` (`taxcalc_credits`); `ots_direct.py:158` (`_fill`), `:447`
(`_ots_aged_blind`).

## 2. Were both oracles actually run, and on what?

**Yes, both, live.**

* **OpenTaxSolver** — `OTS_DIR=~/OpenTaxSolver2024_22.07_linux64`, reported by the driver as
  `OpenTaxSolver 2024 (OpenTaxSolver2024_22.07_linux64)`. Driven on: six committed corpus households
  (baseline capture), a two-dependent aged/blind MFJ household, a one-CTC-child low-income Single
  household, a real vault return (below), and 25 live `sweep.py` scenarios.
* **PSL Tax-Calculator 6.8.2** (`.venv/bin/python`, pandas 3.0.3). Driven on: **all 107 committed
  corpus households** (baseline capture, twice), plus every household above.

Neither engine's answers moved on the existing corpus — see §5.

## 3. What landed, per numbered deliverable

### (1) `project_to_golden` + the dependents block on `GoldenInputs`

`crates/btctax-core/src/tax/testonly.rs:1782` —
`project_to_golden(ri: &ReturnInputs, state: &LedgerState) -> GoldenInputs`.

**Deviation from R13's signature, recorded:** it takes the ledger as well as the return. R13's own
sentence requires it (*"`p22250/p23250` from the ledger's Schedule D"*), `build_golden_return` returns
the pair, and the inverse of a function returning a pair takes a pair. `income project` has both in
hand.

**No arithmetic of its own.** Every figure is read from the helper the printed return reads:
`sum_wages`, `sum_taxable_interest`, `sum_ordinary_dividends`, `sum_qualified_dividends`,
`sum_cap_gain_distr`, `sum_unemployment`, `form_1099b_gains`, `forms::schedule_d`, and a new
`schedule_c_net_profit`. The last is a **hoist, not a copy**: Schedule C lines 1 and 31 were formed
inline in `assemble_absolute` (`return_1040.rs:2162-2163`); they are now
`schedule_c_gross_receipts` / `schedule_c_net_profit` and `assemble_absolute` calls them, with a
`debug_assert_eq!` pinning the old expression. That is the two-chain rule: a second summation in the
projection is the shape that once left `total_tax` short by the whole AMT with every test green.

The projection table (leaf → oracle box) is committed as a doc-comment table on `project_to_golden`:

| oracle row | Tax-Calculator | OpenTaxSolver | btctax |
|---|---|---|---|
| `w2_income` | `e00200` | `L1a` | Σ `w2s[].box1_wages` |
| `taxable_interest` | `e00300` | `L2b` | Σ `int_1099[].(box1+box3+box10)` + Schedule B filer records |
| `ordinary_dividends` | `e00600` | `L3b` | Σ `div_1099[].box1a` + Schedule B filer records |
| `qualified_dividends` | `e00650` | `L3a` | Σ `div_1099[].box1b` |
| `short_term_capital_gains` | `p22250` | 8949 rows | Schedule D Part I gain + Form 1099-B ST net |
| `long_term_capital_gains` | `p23250` | 8949 rows | Schedule D Part II gain + 1099-B LT net + Σ box 2a |
| `self_employment_income` | `e00900` | `S1_3` | Schedule C line 31 |
| `unemployment` | `e02300` | `S1_7` | Σ `g_1099[].box1` |
| `state_income_tax` | `e18400` | `A5a` | Schedule A line 5a, following the §164(b)(5) election |
| `real_estate_tax` | `e18500` | `A5b` | `schedule_a.salt_real_estate` |
| `mortgage_interest` | `e19200` | `A8a` | Schedule A 8a + 8b + 8c |
| `charitable_cash` | `e19800` | `A11` | Σ `schedule_a.charitable[]` of class `Cash60` |
| `hsa_deduction` | `e03290` | `S1_13` | Form 8889 line 2 |
| the dependents block | `n24`/`XTOT`/`EIC`/ages/blindness | `Dependents`/`You_65+Over?`/… | the T7/T8 answers |

**Deviations on the block's SHAPE, recorded.** R13 names `n24`, `nu18`, `n1820`, `n21`,
`age_head`/`age_spouse`, `blind_head`/`blind_spouse` and the EIC count as fields. They are delivered
as a **`dependents: Vec<GoldenDependent>` plus the two adult ages and the two blindness flags, with
`n24` / `nu18` / `n1820` / `n21` / `XTOT` / `EIC` DERIVED** by accessors on `GoldenInputs`. Reasons:

* A stored count beside a stored row set is two encodings of one fact, and this repo's recorded
  failure mode is exactly that going stale (`CLAUDE.md`, twice in this harness). Deriving also makes
  it impossible for a corpus cell to state a set of counts no set of rows can realise.
* `XTOT` is **not in R13's list at all and had to be added**, as a derived accessor. Schedule 8812's
  other-dependent leg is `ODC_c × max(0, XTOT − childnum − num)`
  (`taxcalc/calcfunctions.py:3362`), so without it every *credit for other dependents* row would have
  projected to a taxcalc household computing $0 — a line-19 excuse that cannot fail on the ODC arm.
  That is the FR-88 shape precisely, and it is why it is derived from the filing status and the row
  set rather than summed from the age bands (a member whose age is unknown must still be counted).
* `nu18`/`n1820`/`n21` are **inert under baseline law** and are documented as such: they reach only
  `UBI` (a reform parameter, all rates $0) and `AGI`'s `pre_c04600 = max(0, XTOT − nu18) × II_em`,
  whose `II_em` is $0 for 2018-2025. They are carried because they are part of the row's input space.

`GoldenInputs` also gained `unemployment` (`e02300` / `S1_7`), which R13's projection list names and
which had no field: without it every Form 1099-G box 1 would have had to be laundered into
`ORACLE_INVISIBLE`.

`build_golden_return` consumes the block: `age_head` materialises the taxpayer's date of birth,
`blind_head`/`blind_spouse` the §63(f) flags, and each `GoldenDependent` a full `Dependent` row with
all fifteen live §152 gates answered so the flowchart reaches the stated credit column — **and the
function re-walks each row and panics on a mismatch**, so a corpus cell cannot name a column its
answers do not produce.

Two changes inside `build_golden_return` that move nothing on the committed corpus and are load-bearing
for the block:

* `ri.tax_year = 2024` (was the `0` "not stated" sentinel). The dependent age tests read `ri.tax_year`;
  at `0` a ten-year-old is 2,014 years old.
* `taxpayer_died_during_year = Some(false)` and, with a spouse, `spouse_died_during_year = Some(false)`
  — what `not_a_dependent()` already does, and what §G-9 requires before a §63(f) age-65 addition can
  count. **Measured**: without them a 66-year-old MFJ filer's 1040 line 12 stayed at the flat $29,200
  while both engines added $1,550. With them, one aged box gives $30,750 and all four give $35,400.
  Invisible on all 107 committed cells (whose taxpayer is 44).

`GoldenInputs::canonical()` was added and the inverse KAT compares canonical forms, because
`build_golden_return` is **not injective** in exactly two places, neither of them a hole in the
projection: the corpus's catch-all `itemized_deductions` lump is materialised by ADDING it to the
Form 1098 row, and a spouse's age/blindness are dropped when the status has no spouse. A separate KAT
asserts `build(g)` and `build(g.canonical())` produce the identical return and that `canonical()` is
idempotent — which is what makes it a statement about the fixture rather than a fudge.

`DependentWalk` gained `is_qualifying_child()` (`dependent_gates.rs`), recorded by the walk rather
than recomputed, because §32(c)(3)(A) defines an EIC qualifying child as a §152(c) qualifying child
and a second copy of Step 1's five-limb conjunction would be a second thing to keep true.

### (2) `ORACLE_INVISIBLE` — 67 entries, and the partition is DERIVED

`testonly.rs:1936`. **67 entries**, each a normalised serde path prefix with a reason and a
one-sentence mechanism: `NotInTheOracleRow` ×55, `PaymentOrWithholding` ×9, `PriorYearCarryIn` ×3.

`crates/btctax-core/tests/oracle_projection.rs` does **not** read that list to decide what is
visible. It enumerates the money leaves of the maximal sentinel with
`leaf_walk::money_leaves` (by TYPE, through `Decimal`'s own deserializer), perturbs each one to a
distinct probe, and asks whether `project_to_golden` moved. The table only has to account for the
complement, in both directions:

* a leaf that moved the projection and is still listed ⇒ **red** ("a stale exemption hides a real
  figure");
* a leaf that did not move it and is not listed ⇒ **red** ("either carry the figure or add an entry
  saying why no oracle can take it");
* an entry claiming no money leaf ⇒ **red** (stale).

**The probe runs over TWO fixtures, and the second one is the point.** Schedule A line 5a is an
election (§164(b)(5)): with `salt_use_sales_tax == Some(true)` the sales-tax amount is the live box
and the income-tax boxes are dead, and otherwise the reverse. A partition measured on one setting
called three live boxes invisible.

**The derivation found a real defect in my own first draft.** It classified
`b_1099[].short_term_proceeds` / `..._basis` / the long-term pair as INVISIBLE, and they are not:
`capital_net` joins the broker totals to the crypto nets at the §1222 netting (§G-28/B4), so the
projection was understating a brokered filer's `p22250`/`p23250` by the whole securities gain. A
hand-written list would have recorded my belief instead. That is the single strongest argument for
the shape the prompt asked for.

`unprojected_nonzero_leaves(ri)` returns the non-zero money leaves that do not project, each with its
reason — printed by `income project` and echoed by `check_return.py`, so a household described to an
engine WITHOUT a figure never reads as a btctax defect.

**Beyond the brief, and deliberately:** the leaf census walks `ReturnInputs`, and **the ledger is not
one of its leaves** — so the crypto side of a btctax return, the reason this tool exists, would have
been the one part of the projection with no completeness check at all.
`return_1040::unprojected_ledger_lines(state, year)` censuses it: non-business crypto ordinary income
(Schedule 1 line 8v) and crypto donations (Schedule A line 12). Both are reported rather than
carried, because neither can be put to two engines — see FR-92.

### (3) `btctax income project --year N`

`crates/btctax-cli/src/cli.rs` (`IncomeCmd::Project`), `src/main.rs` (dispatch),
`src/cmd/tax.rs::project_return_inputs`. It reads through `input_form_store::load_for_read`, the same
seam `income scrub` uses, so a version-current DRAFT shadows the committed row — the return the filer
is looking at is the return that gets checked. Output is
`{ "tax_year", "row", "not_carried", "not_carried_from_the_ledger" }`.

**No identity, structurally**: `GoldenInputs` has no name, SSN, address, employer or payer field, so
there is nothing to strip. Asserted twice — in core, by planting `ZQIDENTITY*` tokens in every
identity field of a `ReturnInputs` and asserting none survives serialization (and that the row is not
thereby vacuous: the wages and both dependents are still there); and at the vault level in
`btctax-cli/tests/tax_profile.rs`, over a real `income import` of a TOML carrying a cleartext SSN.

`docs/man/btctax-income-project.1` is generated (`cargo run -p xtask -- docs`);
`docs/man/btctax-income.1` gained the subcommand line.

### (4) `scripts/oracle/check_return.py`

332 lines. Reads `income project`'s wrapper (or a bare row) and runs, in order:

1. **A cross-language check.** `gen_goldens.dependent_block(row)` versus the harness's new
   `--row-counts` mode, which prints the counts off `GoldenInputs`'s own Rust accessors. It exists
   because a Python `XTOT` that drifted from the Rust one would move the $500-per-dependent credit
   the whole line-19 excuse is measured against, silently. **It fired on its first live use** — the
   Rust side emitted `age_spouse: null` where Python emitted `0` for the same absent spouse — and the
   harness was changed to emit the taxcalc-ready form. That is the instrument seen discriminating
   without a plant.
2. The harness in DEFAULT mode → btctax's line map off the printed page.
3. Both oracles on the same row.
4. The harness in `--check` mode → **every per-line verdict, computed in Rust**. No btctax arithmetic
   is re-implemented in Python: `round_dollar`, the Tax Table and the QDCGT worksheet all stay in
   Rust behind the harness (I4), including the Tax-Table-vs-rate-schedule methodology class.
5. The two credit lines the harness does not compare, with excuses computed from mechanism.
6. A **witness census**, per line rather than per row.

`--selftest` runs the offline B1 kill with no OTS, no taxcalc and no vault.

### (5) Docs

`scripts/oracle/README.md` (new, 82 lines) — the index the directory did not have: both oracles and
their drivers, the setup, every script, and the `check_return.py` step end to end, including the three
things in its output that are easy to skip past (computed excuses, the witness census, `not_carried`).
`docs/man/btctax-income-project.1` (generated). **`CLAUDE.md` was deliberately NOT edited** — my
operating rules forbid it. Its "Two oracles, and the `.venv`" section would benefit from one sentence
naming `check_return.py` as the real-return path; that is the controller's call.

## 4. The excuses, with their mechanisms and their measured sizes

| line | btctax | OTS | Tax-Calculator | excuse |
|---|---|---|---|---|
| 1040 line 19 (CTC/ODC) | **blank** — no Schedule 8812, and the packet emits no cell at all | **not a witness** | `c07220 + odc` | the whole of taxcalc's figure |
| 1040 line 27 (EIC) | **blank** — no Schedule EIC | **not a witness** | `eitc` | the whole of taxcalc's figure |
| 1040 line 24 | line 16 + Schedule 2 | cross-footed by the harness | **not compared** | — |

**Why OpenTaxSolver is not a witness on 19 or 27, read out of its source.**
`taxsolve_US_1040_2024.c:1978,1985,1986` read `L19`, `L27` and `L28` with `GetLine(...)` — they are
INPUTS — and line 1928's `GetLine1("Dependents", &NumDependents)` is the ONLY use of that variable in
the file: it is parsed and never read again. OTS therefore computes no CTC, no ODC and no EIC, and
its agreement with btctax on line 19 is `0 == 0` between two engines neither of which computed
anything. That is §G-9 exactly, and `check_return.py` prints it as a **one-witness** line with the
mechanism rather than as two OKs. What OTS *does* read is the §63(f) chart (`:2012`, *"Std. Deduction
chart for People who were Born Before January 2, 1960, or Were Blind"*), so the four aged/blind
parameters are a genuine second witness on 1040 line 12 — that is the two-oracle content of the
dependents block. Watched discriminating on a Single/$62,000 household: $14,600 → $16,550 (aged) →
$16,550 (blind) → $18,500 (both), i.e. §63(f)'s $1,950 per box, exactly.

**Why line 24 is not compared against taxcalc:** `c09200` is the liability AFTER nonrefundable
credits (so a household with dependents makes it smaller than btctax's line 24 by `c07100`) **and**
it inherits the line-16 methodology dissent. Two mechanisms in one number is not a check; the harness
already declares taxcalc unwitnessed on line 24 and cross-foots it against OTS. Measured on the
two-dependent household: btctax 6,277, taxcalc 3,774, `c07100` = 2,500, line-16 dissent = 3.

**Measured excuse sizes, all exact:**

| fixture | line 19 | line 27 |
|---|---|---|
| MFJ, $90,000, one CTC child + one ODC dependent, aged + blind ×2 | gap 2500, expected 2500 | gap 0, expected 0 |
| Single, $22,000, one CTC child aged 6 | gap 740, expected 740 | **gap 4213, expected 4213** |
| a real vault return, Single $41,000, one CTC child aged 6 | **gap 2000, expected 2000** | gap 0, expected 0 |

The middle row is the one that makes line 27 a real comparison rather than `0 == 0`.

## 5. Every pinned number, old → new, with cause

| instrument | at dispatch | now | cause |
|---|---|---|---|
| `make check` | 3496 passed / 12 skipped | **3506 passed / 12 skipped** | +9 in the new `oracle_projection.rs`, +1 (`income_project_prints_the_oracle_row_and_no_identity`) in `btctax-cli::tax_profile`. Nothing else moved. |
| `line-coverage` | 375 / 18 forms / 31 exceptions (ratchet 31) / 0 unverifiable / 17 not-line-bound | **identical** | — |
| `census-join` | 274 across 13 maps | **identical** | — |
| `stop-list` | 8 + 4 sources, 91 prompts | **identical** | — |
| `prompt-check` | 88 assertions | **identical** | — |
| `box-census` | 268 / 19 editions / 9 returns | **identical** | — |

**Both oracles' baselines on the existing corpus are unchanged**, measured rather than argued:

* Tax-Calculator over **all 107 committed households**, before and after adding `e02300` and the
  whole dependents block to `_taxcalc_row`: byte-identical JSON.
* And again with `age_head` forced to 44 on every household, to justify aligning the Python
  absent-key default with `GoldenInputs`'s serde default (`GOLDEN_ADULT_AGE`): byte-identical, so no
  baked golden moved.
* OpenTaxSolver over six committed households, before and after adding `S1_7`, the four aged/blind
  parameters, `Dependents`, and the `_fill` key-regex change: byte-identical JSON.
* `sweep.py --seed 1 --count 25`: `25 admitted, 0 skipped … 0 undeclared divergences`.
* `gen_goldens.assert_baked_provenance_is_current()`: current. `python3 ots_direct.py`: OK.

**The `_fill` regex change, named because it is the one silent-failure risk in the Python diff.** The
1040's aged/blind parameters are literally called `You_65+Over?` and `Spouse_Blind?`; the key class
`[A-Za-z0-9_#/]` matched `You_65`, failed its own lookahead on the `+`, and left the line untouched —
so passing the key raised *"keys not found in template"* and the §63(f) additions could not be driven
at all. `+` and `?` were added to the class. No existing key contains either character, and the OTS
baseline above is the measurement that says so.

`GOLDEN_ADULT_AGE` is **read out of `testonly.rs`** by `ots_direct._rust_const`, not retyped in
Python, and raises if the constant cannot be found.

## 6. Every kill, with its planted defect and the observed red

All plants were reverted from a `cp` backup and `touch`ed; the suite was re-run green after each.

| # | guarantee | plant | observed red |
|---|---|---|---|
| K1 | `ORACLE_INVISIBLE` is complete | added `schedule_a.planted_money_leaf: Usd` (which first `E0027`'d the classifier and `return_refuse`'s exhaustive destructures — the intended blast radius — then, once satisfied) | `every_money_leaf_either_projects_or_is_named_in_oracle_invisible` — *"these `Usd` leaves neither reach the oracle row nor appear in ORACLE_INVISIBLE … `schedule_a.planted_money_leaf`"* |
| K2a | no stale exemption | added a `prefix: "schedule_a.a_field_that_no_longer_exists"` entry | `no_oracle_invisible_entry_is_stale` — *"these ORACLE_INVISIBLE entries claim no money leaf — the field they named is gone: [\"schedule_a.a_field_that_no_longer_exists\"]"* |
| K2b | no exemption over a live figure | added a `prefix: "w2s[].box1_wages"` entry | `every_money_leaf_either_projects_or_is_named_in_oracle_invisible` — *"these leaves DO move the projection and are still listed as invisible — a stale exemption hides a real figure"* |
| K3 | the projection carries the dependents block | `dependents: Vec::new()` in `project_to_golden` | 3 red: `the_projection_inverts_a_household_with_two_dependents` — *"assertion `left == right` failed: the dependents block does not round-trip"*; plus `deleting_the_dependents_block_…` and `the_projected_row_carries_no_identity` |
| K4 | the ledger census reports line 8v | `if false && crypto.nonbusiness_ordinary > Usd::ZERO` | `the_ledger_lines_the_oracle_row_cannot_carry_are_reported` — *"assertion `left == right` failed: []"* |
| K5 | the line-19 excuse refuses any other size (offline) | `actual_gap = … + 1` in `credit_line_verdict` | `check_return.py --selftest` → `AssertionError: (False, 2501, 2500)` |
| K6 | the line-19 excuse refuses any other size (**live, both oracles**) | injected `lines["1040.line19"] = "1"` after the harness read-back | full run exit **1**: *"CTC/ODC (L19): btctax 1, taxcalc 740 — the gap is 739 and the mechanism predicts exactly 740"* |
| K7 | the projected row carries no identity | (assertion-side, not a plant) `ZQIDENTITY*` tokens in every identity field; the test also asserts the row is not vacuous | green, and K3 shows it reds when the row loses content |

The B1 pairing for `check_return.py` is `--selftest`, which exercises `credit_line_verdict` on four
cases: the exact gap (accepted), an off-by-one (refused), btctax printing the FULL credit (refused —
the excuse is *"btctax forgoes it"*, not *"line 19 may be anything"*, so the day Schedule 8812 lands
this check reds and demands a real comparison), and a no-credit household (accepted).

Also run: `canonicalising_a_golden_row_changes_no_return_it_builds` over all 107 cells, and
`the_projection_inverts_build_golden_return_on_every_committed_household` (with a floor assertion
that the corpus is still >100 cells, so a stubbed corpus cannot make it vacuous).

## 7. The end-to-end journey, run for real (spec J-18 / J-28)

A scratch vault, `btctax init` → `income import` of a TOML with a cleartext SSN and one
CTC-edge child → `income project --year 2024` → `check_return.py`:

* The row carries `w2_income 41000`, `dependents [{age 6, credit child_tax_credit,
  eic_qualifying_child true}]`, `age_head 35`, and **none** of `Jordan`, `Reyes`, `123-45-6789`,
  `RIVER LOGISTICS`.
* `not_carried` names `w2s[0].box2_fed_withheld` ($3,100, `PaymentOrWithholding`) and box 3 / box 5
  ($41,000 each, `NotInTheOracleRow`).
* `check_return.py`: **10 compared lines, all OK**; 7 of 10 with two independent engines; the three
  single-witness lines named with their mechanisms; `CTC/ODC (L19) [taxcalc] blank 2000 OK (excused)
  gap=2000 expected=2000`. Exit 0.

## 8. Deviations from the brief, collected

1. `project_to_golden` takes `(ri, state)`, not `(ri)` — R13's own `p22250/p23250` sentence requires
   the ledger. §3(1).
2. The dependents block is `Vec<GoldenDependent>` + two ages + two blindness flags, with
   `n24`/`nu18`/`n1820`/`n21`/`XTOT`/`EIC` **derived**, not stored. §3(1).
3. `XTOT` was added — it is not in R13's list, and without it the ODC arm of the line-19 excuse
   cannot fail. §3(1).
4. `unemployment` was added to `GoldenInputs` — R13's projection list names `e02300` and there was no
   field for it. §3(1).
5. `GoldenInputs::canonical()` exists and the inverse KAT compares canonical forms, with a second KAT
   proving the canonicalisation changes no return. §3(1).
6. `build_golden_return` now sets `tax_year` and the two §G-9 death gates. Both measured as
   no-ops on the committed corpus. §3(1).
7. A ledger-side census (`unprojected_ledger_lines`) beyond `ORACLE_INVISIBLE`'s `ReturnInputs`
   scope. §3(2) — added because the crypto side would otherwise be the one uncensused part.
8. A `--row-counts` mode on the harness, so no dependent count is derived twice across the language
   boundary. §3(4).
9. `scripts/oracle/README.md` is new (the directory had no index); **`CLAUDE.md` was not edited** —
   see §3(5) for the one sentence it wants.
10. `check_return.py` does not compare 1040 line 24 against Tax-Calculator; the mechanism is in §4.

## 9. Follow-ups filed (`FOLLOWUPS.md`)

* **FR-91** — the oracle row carries ONE wage figure, so W-2 box 3 and box 5 are modelled as equal to
  box 1. Reported, never silent. Closing it means re-baking the corpus, so it wants its own cycle.
* **FR-92** — non-business crypto ordinary income (Schedule 1 line 8v) has no Tax-Calculator
  variable at all, so btctax's most characteristic income line is one-witness at best; crypto
  donations (Schedule A line 12) are the same shape. Both are reported rather than carried, because
  carrying either would be calling a figure validated on one oracle.
* **FR-93** — a filer who never answered the §G-9 death question sees 1040 line 12 diverge from BOTH
  engines by the §63(f) aged addition. Deliberately NOT excused (it is an actionable finding); filed
  because `check_return.py` does not yet name the cause on the diff line.

## 10. Files touched

Modified: `crates/btctax-core/src/tax/{testonly.rs,return_1040.rs,dependent_gates.rs}`,
`crates/btctax-cli/src/{cli.rs,main.rs,cmd/tax.rs}`, `crates/btctax-cli/tests/tax_profile.rs`,
`crates/btctax-forms/tests/attestation.rs`, `crates/btctax-oracle-harness/src/main.rs`,
`scripts/oracle/{gen_goldens.py,ots_direct.py}`, `docs/man/btctax-income.1`, `FOLLOWUPS.md`.
New: `crates/btctax-core/tests/oracle_projection.rs` (372 lines, 9 tests),
`scripts/oracle/check_return.py` (332), `scripts/oracle/README.md` (82),
`docs/man/btctax-income-project.1` (generated).
`git diff --stat`: **1589 insertions, 17 deletions across 13 tracked files**, plus the four new ones.

## 11. Suite lines, per crate (scoped runs; the closing gate is `make check`)

* `btctax-core::oracle_projection` — `9 tests run: 9 passed, 0 skipped`
* `btctax-core` (`--test oracle_projection --test golden_returns --test kat_tax`) —
  `95 tests run: 95 passed, 0 skipped`
* `btctax-cli::tax_profile` — `12 tests run: 12 passed, 0 skipped`
* whole workspace, `make check` — **`3506 tests run: 3506 passed, 12 skipped`** in 20.6 s
