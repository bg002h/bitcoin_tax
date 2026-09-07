# Brief — interview build T11: the oracle path (R13) — a box-named projection from the return to the oracle row, run locally before export

Single implementer, shared main tree `/scratch/code/bitcoin_tax`, branch `main` (HEAD named at
dispatch). No subagents; no commit/push; never `git checkout --`/`git restore`/`git stash` (revert a
plant via a `cp` backup). Tests via `cargo nextest run --locked -p <crate> -E '<filter>'` (never
`cargo test`, never `--release`, never the whole workspace); `cargo fmt --all` and a clean
`CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings`
before finishing. The Python stack is `.venv/bin/python` (taxcalc 6.8.2, pandas); OpenTaxSolver
needs `OTS_DIR` (read `scripts/oracle/README*` / `ots_direct.py`'s header for the setup and say in
the report whether it was available). Every guarantee lands with a kill seen red once; quote the
red. Every pinned number moved: old → new with cause. `/tmp` is a 32 GB tmpfs. Process in force
(owner S6): ONE seam review and ONE re-verification after you.

★ **Standing rules:** two oracles, never one — a figure is validated only when BOTH reconcile, and a
disagreement is adjudicated against the FORM, never encoded; an excuse list is COMPUTED from the
defect's mechanism, never keyed by vector name; a value the oracles take as INPUT is never validated
by their agreement (§G-9); no decision keys on a list you typed beside derived data.

## The contract
`design/SPEC_interview.md` r2 **R13** (`project_to_golden(ri) -> GoldenInputs` in core, box-named
both sides; `GoldenInputs` gains the dependents block — `n24`, `nu18`, `n1820`, `n21` computed from
each row's DOB and `tax_year`, `age_head`/`age_spouse` from the DOB skippables when `Given` and
absent when `Declined`, `blind_head`/`blind_spouse`, the EIC-qualifying-child count — and
`gen_goldens.py`'s row and the OTS template carry them likewise; `btctax income project --year N`
prints the projection with no identity; `scripts/oracle/check_return.py` runs the harness and both
oracles on it and diffs the compared lines; excuses computed from mechanism — line 19 differs by
exactly the oracle's computed CTC while Schedule 8812 is unbuilt, any other size fails;
`ORACLE_INVISIBLE` — every `Usd` leaf either projects or is listed), **§7 row T11** (its kills),
`CLAUDE.md` "Two oracles, and the `.venv`", `design/SPEC_interview.md` §G-9 as cited. T7/T8's answers
are your inputs (the flowchart verdict enum, the credit column, `date_of_birth`, the blindness
skippables). Build AS WRITTEN; the tree's real names win; deviations recorded.

## Settled facts (controller-measured at `1f3cc137`; re-measure the T7/T8-dependent ones at dispatch)
- `GoldenInputs` (`crates/btctax-core/src/tax/testonly.rs:650-693`) carries filing status, wages,
  interest, dividends, gains, SE income, four Schedule A components and a cash gift — no dependents,
  ages or blindness; `build_golden_return` (`:731`) builds `ReturnInputs` from it;
  `scripts/oracle/gen_goldens.py:199-232` builds the Tax-Calculator row from the same dict;
  `scripts/oracle/ots_direct.py:135` is the OTS template fill; `scripts/oracle/sweep.py:18` runs the
  harness (`cargo build -p btctax-oracle-harness`). The golden households live in
  `crates/btctax-core/tests/goldens/`. The sweep asserts every admitted household reconciles on
  every compared line against BOTH oracles; `verify_f6251.py` has a taxcalc version FLOOR.
- The interview's new leaves since R13 was written: the 1099 sections (T5: `e00300 = Σ int_1099.
  (box1+box3)`, `e00600/e00650 = Σ div_1099.box1a/1b`, `e02300 = Σ g_1099.box1`, the 1098-E sum),
  Form 1098 rows and 8b/8c (T9: `e19200 = 8a + 8b + 8c`), the dependents (T7/T8), `opened_from`,
  the census and door questions (`Option<bool>` — invisible by type). Enumerate the leaves with
  T1's `LEAF_SOURCE` / `leaf_walk`, never by hand.

## What T11 delivers
1. **`project_to_golden(ri: &ReturnInputs) -> GoldenInputs`** in core, box-named both sides, plus
   the dependents block on `GoldenInputs` (the fields R13 names); `build_golden_return` consumes the
   block (a golden with two dependents builds a return with two `Dependent` rows on the CTC/ODC edges
   with the ages implied); `gen_goldens.py` and the OTS template carry the block (I12).
2. **`ORACLE_INVISIBLE`** — the list of leaves that do not project (declarations, 1099-DA answers,
   every `Option<bool>`, the census), asserted COMPLETE inside the KAT by walking `LEAF_SOURCE`: every
   `Usd` leaf either projects or is listed; a leaf added tomorrow without a decision reds.
3. **`btctax income project --year N`** — prints the projection (no identity by type: `GoldenInputs`
   carries no name/SSN/address; assert it structurally).
4. **`scripts/oracle/check_return.py`** — runs the harness and both oracles on the projection and
   diffs the compared lines, with excuses computed from mechanism: line 19 = the oracle's computed
   CTC exactly (Schedule 8812 unbuilt); line 27 (EIC) likewise if the oracles compute it and btctax
   forgoes it — state each excuse's mechanism and size; anything else fails.
5. **Docs**: `scripts/oracle/README` (or the existing doc) gains the check-return step; the man page
   for `income project`.

## Kills (each seen red once)
`project_to_golden(build_golden_return(g).0) == g` over every golden household (the projection
inverse) AND over a new golden with two dependents (ages, blindness and the CTC edge round-trip);
`ORACLE_INVISIBLE` complete (plant a new `Usd` leaf with neither a projection nor a listing → red);
on a fixture with one CTC-edge child `oracle_line19 > 0` and the excuse equals it exactly — a diff of
any other size fails (plant an off-by-one → red); deleting the dependents block from the projection
reds the inverse; `income project` output contains no identity leaf (structural assertion);
`check_return.py` on the TY2024 example fixture reconciles against both oracles (or names the exact
excused line and its mechanism); the existing sweep still reconciles.

## Constraints
- Never file an upstream defect or call a figure validated on one oracle; if the two split, record
  the line, the form's answer and the mechanism in the report — do not encode an excuse by name.
- No fixture carries real identity; the projection carries none by type.
- If context runs short: leave the tree compiling, fmt-clean and green, and say what is unfinished.

## Report — your FINAL action
`design/agent-reports/2026-09-07-build-interview-T11-implementation.md`: per numbered item what
landed, the projection table (leaf → oracle box), `ORACLE_INVISIBLE` as committed, the excuse
mechanisms with sizes on the fixtures, whether OTS was available, every deviation, every kill with
its red text, every pinned number moved, suite lines per crate. Return only a 4-line summary plus
the path.
