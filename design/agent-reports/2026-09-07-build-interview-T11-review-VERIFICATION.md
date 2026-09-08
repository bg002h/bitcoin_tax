# Ledger — controller machine-check of the T11 seam review

Every measurable claim the controller re-ran from `2026-09-07-build-interview-T11-review.md`
(persisted verbatim at `f6dc308e`). Report counts: `C=2 I=4 M=3 N=1` — the heaviest of the interview arc.

Baseline: `make check` at `46d4b2d6` — **3506 passed, 12 skipped, exit 0**, taken after
`find crates -name '*.rs' -exec touch {} +` per FR-90, so it is a real recompile. The reviewer's
worktree carried no change but the report; all its plants were reverted **and touched**.

| # | Claim | Check | Result |
|---|---|---|---|
| **C-1** | `check_return.py` reconciles a return `btctax report` REFUSES | source, 3 sites | **CONFIRMED** |
| **C-2** | No projected row can drive OTS's Schedule A | source, 4 sites | **CONFIRMED** |
| **I-2** | The credit excuse cannot tell a blank from an asserted `0` | source, 3 sites | **CONFIRMED** |
| **I-4** | HoH / MFS / QSS project a row that panics downstream | source, 3 sites | **CONFIRMED** |
| I-1, I-3, M-1…M-3, N-1 | — | accepted, not re-run | see below |

---

## C-1 — CONFIRMED

Three sites, each read at HEAD:

- `crates/btctax-core/src/tax/testonly.rs:1557-1559`, **inside `build_golden_return`**:
  `ri.foreign_accounts = Some(false); ri.foreign_trust = Some(false); answer_all_live_declarations(&mut ri);`
- `scripts/oracle/check_return.py:196` — `default = _harness([], row)`, i.e. the harness is handed the
  **projected row**, and rebuilds a household from it.
- `crates/btctax-cli/src/cli.rs:512` — the help says *"Run it before you export: it is how a REAL
  return reaches an oracle."*

So the tool documented as the real-return path checks a **round-trip rebuild** in which the filer's
unanswered FBAR and declaration gates have been answered on their behalf. The docstring's own
`Exit 2 = … a refused return` contract cannot fire. The reviewer's two-command evidence shows
`btctax report` refusing the same vault the harness reconciles at exit 0.

**Severity sustained at Critical.** This repo's one named architectural defect is answered-ness —
anything that can silently answer for the filer — and here the *verification instrument* does it, then
reports success. It also means any figure the row cannot carry is missing from **all three** columns, so
the failure mode is silence rather than disagreement.

## C-2 — CONFIRMED

- `scripts/oracle/ots_direct.py:573` and `:687` — `if h.get("standard_or_itemized") == "Itemized":`
  gates every Schedule A line (A5a/A5b/A8a/A11/A16/A18) into the OTS input.
- `scripts/oracle/corpus.py:164` says it in as many words:
  `inp["standard_or_itemized"] = "Itemized"  # read by the Python oracles (not a GoldenInputs field)`.
- `grep -c standard_or_itemized crates/btctax-core/src/tax/testonly.rs` → **0**. `GoldenInputs` has no
  such field, so `project_to_golden` cannot emit one.
- `crates/btctax-core/src/tax/printed.rs:796` — btctax already **has** the quantity, so the fix is
  available rather than new work.

An itemizing filer is therefore handed to OpenTaxSolver with no Schedule A at all: line 12 DIVERGES on
a correct return, the run exits 1 telling the filer to adjudicate against the form, and Schedule A line
5e loses its OTS witness while the census reports *"only one engine models this line"* — which is false.
The reviewer's before/after, whose **only** difference is the inserted key, is conclusive.

**Root cause worth naming for the fold:** the completeness partition is derived over `Usd` **leaves**.
The itemize election is a *non-`Usd` fact that routes money*, so it falls outside the derivation
entirely — the same gap as I-1.

## I-2 — CONFIRMED, and it composes with a defect we already carry

- `check_return.py:279-280` — `btctax_value = int(printed) if printed is not None else 0`, under a
  comment that states the opposite of what the code does (*"that is the blank, not a zero it
  asserted"*).
- `credit_line_verdict` (`:140-151`) returns `ok ⟺ round(oracle) - btctax == round(oracle)`, i.e.
  `ok ⟺ btctax_value == 0`. A printed `0` and a blank are the same verdict.
- `advisories.rs:937-939` — `ctc_odc_line19` returns `Some(Usd::ZERO)` whenever `ctc_provably_zero`
  fires, so btctax **does** print a cell.

★ **This is the second half of a hole neither task owns.** T8's I-2 (FR-85) established that the §24(h)(2)
per-child ceiling is a year-blind $2,000 and that when TY2025's package lands `ctc_provably_zero` will
swear a `0` on line 19 for a household that still has credit. T11's excuse is the one instrument that
could catch that — and it cannot, because it reads the sworn `0` as the forgone blank. The reviewer
planted the understated ceiling and showed the btctax column move from `blank` to `0` against an oracle
computing $1,000, with the verdict unchanged at `OK (excused)`, exit 0. Two independently defensible
designs, one hole.

## I-4 — CONFIRMED

- `testonly.rs:1826-1829` — `project_to_golden` maps `Single` and `Mfj` and falls through
  `other => format!("{other:?}")`, yielding `"HoH"` / `"Mfs"` / `"Qss"`.
- `testonly.rs:1363` — `other => panic!("unmapped filing status {other:?}")`.
- `gen_goldens.py:119-125` — `TAXCALC_MARS` keys are `"Married/Sep"`, `"Head_of_House"`, `"Widow(er)"`,
  matching none of the Debug names.

So for **three of five** filing statuses the whole T11 gate is unavailable, `income project` exits 0 and
prints a full row anyway, and the failure surfaces three steps later as a Rust panic. This is a
hand-typed match beside a derived enum — the standing *"no decision keys on a list you typed beside
derived data"* rule — written against a status set that T8 had just widened.

## Accepted without independent re-run

**I-1** (a non-`Cash60` charitable gift silently dropped by the projection and both censuses; deleting
the class filter leaves all 9 T11 KATs plus `golden_returns`/`kat_tax` green) — same root as C-2, and the
fold must close both. **I-3** (the witness census prints a false mechanism for four lines taxcalc does
compute; measured `additional_medicare_tax = 1530.0` on a line reported single-witness). **M-1** (the
ledger census is a hand list of two with no derivation and no completeness kill), **M-2**
(`--year` silently overrides the wrapper's `tax_year`), **M-3** (seam 6's §G-9 statement unwritten),
**N-1**. None gates except as noted.

## Verified clean by the reviewer, and not re-run here

The inverse round trip (a wrong-box plant reds two KATs); the derived `Usd` partition (the controller
separately planted a deleted `ORACLE_INVISIBLE` entry and saw it red); `XTOT` genuinely lighting the ODC
arm ($500 live); the §G-9 OTS claim against `taxsolve_US_1040_2024.c` (`NumDependents` occurs exactly
twice — declared and parsed, never used); no excuse keyed by vector name; the live sweep; and **no
identity** in the projected row on a household carrying `ZQ…` tokens in 22 identity and free-text
fields. That last one is the contract question from the dispatch, answered affirmatively on a hostile
household rather than the builder's fixture.

## Disposition

**2C / 4I blocking.** C-1, C-2, I-1, I-2, I-3, I-4 all fold, plus M-1…M-3 and N-1. Two of them
(C-2, I-1) share one root — the completeness partition covers `Usd` leaves only and is blind to facts
that *route* money — and the fold should close the root, not the two instances. Fold brief:
`BRIEF-fold-interview-T11-review.md`.
