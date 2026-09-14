# REPORT — FR-234: the four instrument defects behind the $1 SE residual

> **PROVENANCE NOTE (added by the controller, not the author).** This report was written by the
> implementing agent but could NOT be persisted by it: its tooling refused the write with *"Subagents
> should return findings as text, not write report files"*, overriding the brief's persist
> instruction. It therefore returned inline and the controller wrote it to disk verbatim. The only
> controller edits are (a) this note and (b) restoring `<`, `>` and `&` where the notification
> channel had HTML-escaped them. Filed as FR-240.

**All four fixed, all four mutation-verified RED then green, all four gates green.** Work is UNCOMMITTED in `.claude/worktrees/agent-a3cd225375c7d63e9`.

| file | change |
|---|---|
| `crates/btctax-oracle-harness/src/main.rs` | D1 (`Engine`, `Oracles`, `legacy_columns`), D3 (`Crossfoot`), 14 call sites |
| `crates/btctax-oracle-harness/tests/smoke.rs` | D2 (`SweepReport`), D4 parallelism + ignore reason, 4 B1 tests |
| `.github/workflows/ci.yml` | D4 — the `corpus-sweep` job that actually runs the ignored test |
| `scripts/oracle/check_return.py` | witness census now reads the authoritative `oracles` map |

Untouched: `se.rs`, `design/forms/extract/`, existing `REPORT-*`, the frozen TY2026 baseline, `crates/xtask/**`, `crates/btctax-core/**`. No subagents spawned. No commit. No gate backgrounded.

## The corpus-wide roster D2 now produces

Captured with D3 reverted (the full pre-fix numeric state), all 107 households driven:

```
21 of 107 admitted households DIVERGE (every one listed — the sweep no longer stops at the first):
  ca_A_single_int-none_se-over_ltcg-0_qd-0
      schedule_se.line12 (Sch SE L12 (SE tax) [taxcalc]): on_paper=16956 internal=16956 oracles={"taxcalc":"16955"} class=diverge
  ca_A_single_int-none_se-over_ltcg-0_qd-1 / -1_qd-0 / -1_qd-1      … identical line
  ca_A_single_int-over_se-over_ltcg-0_qd-0 / -0_qd-1 / -1_qd-0 / -1_qd-1   … identical line
  ca_A_mfj_int-none_se-over_ltcg-0_qd-0 / -0_qd-1 / -1_qd-0 / -1_qd-1     … identical line
  ca_A_mfj_int-over_se-over_ltcg-0_qd-0 / -0_qd-1 / -1_qd-0 / -1_qd-1     … identical line
  ca_P_03 / ca_P_05 / ca_P_10 / ca_P_14 / ca_P_16                        … identical line
  ★ Read the roster for a SHARED MECHANISM before adjudicating: one lawful rounding residual on many
    households looks nothing like a wrong figure on a few.
```

All 21 rows are byte-identical — same line, same engine, same `16956 / 16955`. **That uniformity is the diagnosis**, and the abort-on-first sweep could never show it. It confirms the adjudication exactly. Post-fix, an independent scan of all 107 households reports **21 rows absorbed and 0 rows still red** — the mechanism resolves precisely the red set and did not silently widen.

## D1 — `verdict_engine` misattributed the witnessing engine

`enum Engine { Ots, Taxcalc }` with `_`-free `name()`/`agree_class()`. The authoritative verdict field is now an **`oracles` map** — one entry per engine that spoke; "did not speak" is an absent key, never a fabricated `$0`. The `ots`/`taxcalc` columns survive as `legacy_columns()`, a projection of that map through an `_`-free match, so they cannot disagree with it and a third `Engine` variant is a build error at the projection.

**Answering the ★: no, two fixed columns is not the right shape, and I changed it.** It is exactly the "derive the list, or make the compiler hold it" shape — the engine set grows, the columns do not. My choice: **keyed map as authoritative, columns as a compiler-held projection**, rather than deleting the columns, because `gen_goldens.py:557` reads `l16["ots"]`/`["taxcalc"]` for the pinned-cell class flips and `sweep.py:416-417` prints both. The map is what the witness census reads — the surface where a third engine matters. The projection's blind spot (a third engine gets no column) is stated in the source: rule option 3, an honest boundary.

**Which test reds when reverted?** `every_single_engine_verdict_publishes_its_figure_under_the_engine_that_spoke`. Mutation: return the lone figure in the `ots` slot whenever one engine spoke.

```
FAIL [0.450s] (5/10) every_single_engine_verdict_publishes_its_figure_under_the_engine_that_spoke
panicked at smoke.rs:533:5: assertion `left == right` failed: a taxcalc-compared row must leave the
OTS column EMPTY — publishing taxcalc's figure there is what made OTS look like the dissenter:
{"line":"schedule_se.line12","label":"Sch SE L12 (SE tax) [taxcalc]","on_paper":"16956",
 "internal":"16956","oracles":{"taxcalc":"16955"},"ots":"16955","taxcalc":null,
 "reconciled":true,"class":"methodology-rounding-order","engine":"taxcalc",
 "rounding_order_residual":"1"}
```

That mutated row is **byte-for-byte the shape that misled the controller**. The pre-fix corpus run printed exactly it: a reader scanning `ots` reads taxcalc's 16955 as OTS's opinion, while OTS's actual 16956 appears nowhere on the row. Strongest evidence D1 was blocking, not cosmetic — the instrument stated something other than what it did. The test asserts the invariant **derived over every row** (engine set from the run's own `oracles` maps, column = engine name lowercased), and per B1a refuses to pass vacuously: it first asserts the two engines genuinely *disagree*, so a mis-route is detectable by value, not only nullness.

## D2 — the sweep aborted on the first failing household

`sweep_check_reconciliation` no longer asserts inside its loop: it drives every household into `Vec<Outcome>`, folds them into `SweepReport { refused, admitted, divergences, reproduction_failures, absorbed }`, and asserts **once** via `failure_message()`, reporting every problem in one message (full roster with each diverging line, reproduction failures, the admitted floor, the refused roster). Not in the brief: `absorbed` **prints the absorbed-row census even on a green run** — an excuse that silently grew to cover half the corpus is a finding, invisible if only failures are reported.

**Which test reds when reverted?** `the_sweep_roster_names_every_divergent_household_not_just_the_first`. Mutation: `return r;` after the first divergent household is pushed.

```
FAIL [0.472s] (6/10) the_sweep_roster_names_every_divergent_household_not_just_the_first
panicked at smoke.rs:762:5: assertion `left == right` failed: BOTH divergent households must be in the
roster, and the clean one must not. Reporting only the first is the D2 defect: ["single_w2_only_standard"]
  left: ["single_w2_only_standard"]
 right: ["single_w2_only_standard", "ca_A_single_int-none_se-over_ltcg-0_qd-0"]
```

The plant is two **genuine** divergences on **different lines and engines** (L16 perturbed on both oracles; a taxcalc-only SE total off by $777) with a clean household between them, so "collects all" is not "flags everything". Per FR-235 neither is in the aggregator's vocabulary — they are oracle-figure injections upstream, and the aggregator never looks at oracle figures.

★ I got this wrong first: I asserted `admitted == 3` before the roster, so the mutation red on `3 != 1` — a far less useful message. The roster assertion now comes first, deliberately, with the reason in the source.

## D3 — the divergence is expected BY MECHANISM

No tolerance, no names. A `Crossfoot` type carrying the mechanism:

```
sigma_round = Σ round_dollar(exact leg)   // what the form prints — "Add lines 10 and 11" over the boxes
round_sigma = round_dollar(Σ exact leg)   // what an engine publishing only the exact total produces
residual    = sigma_round − round_sigma
```

`absorbs(paper, target)` — three load-bearing conjuncts: (1) `residual != 0`, so a non-cross-footed line absorbs nothing and `Crossfoot::NONE` is inert; (2) `paper == sigma_round`, so a **filler bug** that dropped a dollar between the chain and the PDF is never absorbed; (3) `target == round_sigma`, an exact landing with **no window** — which is what makes this an equality on a lawful methodology rather than a tolerance. Given (2), `paper − target == residual` ⟺ (3).

Two structural properties: **`Crossfoot` is a REQUIRED parameter of `verdict_engine`**, so all 14 call sites must state their leg structure and a newly compared line cannot inherit either behaviour by silence — adding one without a decision does not compile. And **a line with no leg structure needs no special case**: its residual is 0, so conjunct 1 makes it strict automatically — the brief's ★ discharged structurally, not by a guard someone must remember.

Legs come from btctax's own exact compute: `[se.ss, se.medicare]` (`printed.rs:327-329`) and `[0.9% × printed L6, 0.9% × printed L12]` (`other_taxes.rs:170,174`), the latter from the same public `SE_RATE_ADDL_MEDICARE` constant the form uses. **Form 8959 L18 is handled as required**, though no household hits its residual today.

**The OTS leg was left alone** and its strictness made explicit: where OTS publishes printed legs the comparison is paper-to-paper (both Σround) and gets `Crossfoot::NONE`; only the `round_leaf(total)` fallback gets the residual. This closes a hole the naive per-line form would have opened — OTS legs genuinely differing by a dollar would otherwise have been absorbable.

**The reproduction guard** (a new instrument, so it gets its own kill): `Crossfoot::reproducing(legs, printed_total)` panics unless `Σ round(leg)` equals what btctax printed. Planting a drifted reproduction (dropping the 0.9% rate):

```
the harness exited non-zero (args ["--check"]): assertion `left == right` failed:
oracle_harness --check: the cross-foot legs [50000, 0] sum-round to 50000 but btctax printed 450 —
the leg reproduction has drifted from the printed chain, so its rounding-order residual cannot be trusted
FAIL (8/10) check_mode_reconciles_every_line_of_the_anchors_and_pinned_cells
FAIL (9/10) check_mode_reconciles_every_line_of_every_admitted_golden_household
```

**Which test reds when reverted?** `the_rounding_order_residual_absorbs_only_its_own_exact_size` — in **both** directions.

(i) Mechanism removed (`absorbs` → `false`):
```
FAIL [0.579s] (6/10) the_rounding_order_residual_absorbs_only_its_own_exact_size
panicked at smoke.rs:642:5: assertion failed: the sum-round/round-sum residual is lawful and must
reconcile: {"line":"schedule_se.line12", … "oracles":{"taxcalc":"16955"},"ots":null,
 "taxcalc":"16955","reconciled":false,"class":"diverge","engine":"taxcalc"}
FAIL [2.106s] (10/10) check_mode_reconciles_every_line_of_every_admitted_golden_household
  → 21 of 107 admitted households DIVERGE  (roster quoted above)
```

(ii) Mechanism widened to the forbidden $1 tolerance (`(paper - target).abs() <= 1`):
```
FAIL [1.877s] (9/10) the_rounding_order_residual_absorbs_only_its_own_exact_size
panicked at smoke.rs:681:9: assertion failed: taxcalc at 16957 against a filed 16956 must be diverge:
a +2 gap is not this line's $1 rounding-order residual, and absorbing it would turn the mechanism into
a tolerance: {"line":"schedule_se.line12", … "oracles":{"taxcalc":"16957"},"reconciled":true,
 "class":"methodology-rounding-order","rounding_order_residual":"1"}
```

(iii) The same tolerance on a line with **no** leg structure, isolated by temporarily skipping the (b) loop so plant (c) is shown to discriminate alone:
```
FAIL (1/1) the_rounding_order_residual_absorbs_only_its_own_exact_size
panicked at smoke.rs:710:5: assertion failed: Form 8960 L17 sums no printed legs, so a $1 gap there is
a real divergence and must fail: {"line":"8960.line17","on_paper":"2926",
 "oracles":{"taxcalc":"2927"},"reconciled":true,"class":"methodology-rounding-order",
 "rounding_order_residual":"0"}
```
Note `"rounding_order_residual":"0"` there — absorbing a gap while reporting a zero residual is itself the tell that a tolerance is not a mechanism.

**On FR-235/FR-230.** The checker tests `paper == Σround(legs) && target == roundΣ(legs)`; no plant is phrased that way — they perturb an oracle's baked exact total, and (c) does it on a line whose leg structure the checker was never written to find. Expectations are **not** derived from the mutated thing: `PAPER = 16956`, `TAXCALC = 16955`, `RESIDUAL = 1` are pinned literals traceable to `f1040sse--2024.txt:45-47` and `f4868--2024.txt:217-223`, and each per-delta expectation is computed from those constants. The `d == RESIDUAL` case is asserted to be `agree-taxcalc` (plain agreement), proving the checker distinguishes "the engine agrees" from "the engine differs lawfully".

## D4 — a real runner, and it is parallel

1. **A `corpus-sweep` job in `ci.yml`** running `cargo test -p btctax-oracle-harness --test smoke --locked -- --ignored`. It runs *the ignored tests of the target*, not a named test, so a new ignored test is picked up with no workflow edit — derived, not a typed list.
2. **The ignore reason names that job** instead of promising "run in CI".
3. **The sweep is parallel** — `check_all` chunks over `available_parallelism()`, joining in chunk order so the roster stays deterministic. The old "inherently serial" comment confused *one subprocess per household* with *one at a time*.

**Measured: the 107-household corpus test runs in 2.17–2.98 s** (1.89 s under the literal CI command), versus 5.05 s to abort on household 1 of 107 pre-fix. D4 and the parallelisation really were one piece of work.

**Which test reds when reverted?** `every_ignored_test_in_this_file_is_actually_run_by_a_ci_job`, both halves:

(i) Job stops running the ignored tests:
```
FAIL [0.003s] (1/1) every_ignored_test_in_this_file_is_actually_run_by_a_ci_job
panicked at smoke.rs:849:5: ci.yml must actually invoke the ignored tests of this target ("cargo test
-p btctax-oracle-harness --test smoke --locked -- --ignored") — an ignore reason promising a runner,
with no runner, is how 21 red households went unnoticed
```
(ii) Ignore reason reverts to its original vague promise:
```
FAIL (1/1) every_ignored_test_in_this_file_is_actually_run_by_a_ci_job
panicked at smoke.rs:855:9: check_mode_reconciles_every_line_of_every_admitted_golden_household's
ignore reason must name the CI job that runs it (corpus-sweep), so the promise and the runner cannot
drift apart: #[ignore = "full corpus (107), one harness subprocess per household — make-check sweeps
the anchors + pinned cells; run in CI / on demand"]
```

The ignored set is **parsed from the file's own source**, never a hand list, so a new ignored test with no runner reds here. The needle is `concat!("#[", "ignore")` so the checker's source never contains the sequence it scans for. Its blind spot is stated in the doc comment: it proves the workflow *invokes* the ignored tests, not that GitHub ran the job or that it is required — a test cannot observe CI, and pretending otherwise would be the same false claim one level up.

## Gate output (run SERIALLY, real output)

```
$ cargo nextest run --workspace
     Summary [  93.328s] 3825 tests run: 3825 passed (3 slow), 12 skipped

$ cargo clippy --workspace --all-targets -- -D warnings
    Finished `dev` profile [optimized + debuginfo] target(s) in 34.49s     (no warnings)

$ cargo fmt --all --check
    (no output, exit 0)

$ cargo nextest run -p btctax-oracle-harness --run-ignored all
     Summary [   2.984s] 10 tests run: 10 passed, 0 skipped
       incl. PASS check_mode_reconciles_every_line_of_every_admitted_golden_household
             PASS every_single_engine_verdict_publishes_its_figure_under_the_engine_that_spoke
             PASS the_rounding_order_residual_absorbs_only_its_own_exact_size
             PASS the_sweep_roster_names_every_divergent_household_not_just_the_first
             PASS every_ignored_test_in_this_file_is_actually_run_by_a_ci_job

$ cargo test -p btctax-oracle-harness --test smoke --locked -- --ignored   # the literal CI command
test check_mode_reconciles_every_line_of_every_admitted_golden_household ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 9 filtered out; finished in 1.89s
```

## Things this brief did not predict

1. **★★ D1 had a live downstream consumer the fix would have silently broken — the witness census itself.** `check_return.py:426-437` branched on `if v.get("ots") is not None: witness(v["line"], v.get("engine","OTS"))`. With D1 fixed, a taxcalc single-engine row has `ots: null`, so that branch stops firing and the row falls to `oracle = "none"` — **the census would have lost the taxcalc witness on Sch SE L12, Form 8959 L18, Form 8960 L17 and the four deeper lines**, telling the filer "only one engine models this line" for lines both engines model. That is the exact false statement the T11 I-3 fold existed to remove. Fixed in the same pass: the census iterates `oracles`, with a staleness `sys.exit` (matching `sweep.py`'s `reproduction_ok` precedent) if a pre-FR-234 binary is on disk. **A fix to one instrument's output format can break a second instrument's input, and the second fails green.**

2. **The residual's correct carrier is the TARGET, not the line.** My first design attached the crossfoot per *line*, which would have relaxed the OTS leg too — OTS legs genuinely differing by a dollar could then have been absorbed as "rounding order". Now attached per *target*: only where the engine published no legs.

3. **`8959.line18`'s residual is latent, not absent.** No household exercises it, so the instruction to handle it anyway is what made the reproduction guard necessary — there is no test vector proving those legs are right, only the guard asserting they reproduce the printed line. Hence its own planted-defect kill.

4. **One of my own plants was wrong, instructively.** I first planted `+$1` on taxcalc's exact SE total as a "wrong-sized divergence" — but `+1` moves it *onto* the filed 16956, so it reconciles as plain `agree-taxcalc`. The plant meant to stay red was measuring agreement. Fixed by deriving each delta's expected class from the pinned constants, which turned the error into the extra assertion that the checker distinguishes agreement from lawful divergence.

5. **The pre-fix corpus test wasted its own evidence** — it printed the entire ~48-row verdict list of the one household it reached while suppressing the other 106 entirely. Maximum volume, minimum information. The new report inverts that.

## Recommendations for the controller (not done here)

- Close `FOLLOWUPS.md` §FR-234 and update its "21 of 107 are RED" framing: the figure was never wrong, the four instruments were. I deliberately did not edit `FOLLOWUPS.md` — it is your ledger and a second agent is active in the tree.
- `design/HARNESS.md` may want D1 beside B1/B1a/FR-230/FR-235 as a distinct shape: **an instrument whose output MISATTRIBUTES its own source.** Not a blind checker (it measured correctly), not a bad plant — it reported the right number under the wrong name, and it cost a real wrong conclusion during this investigation.
- `corpus-sweep` is a new CI job and is not a required check; promotion is a repository-settings action.
