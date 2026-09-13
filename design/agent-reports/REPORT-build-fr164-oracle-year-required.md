# REPORT — FR-164: the tax year is now a required, explicit argument through the oracle harness

**Agent:** opus, own worktree `.claude/worktrees/agent-a0785d4ed9e3589f7`, branch
`worktree-agent-a0785d4ed9e3589f7`, base `557be34de` (the brief commit `422ab1fe2` is its child and
touches only `design/agent-reports/BRIEF-*`; the brief was read out of the object store).
**NOT committed, NOT pushed.** Worktree left dirty.
**Target dir:** `<worktree>/target-fr164` (covered by `.gitignore:61` `target-*/`). Nothing in `/tmp`.

---

## 0. Answer to the one question

> *Can the oracle harness and the golden corpus still answer "which tax year did this validate?" when a
> second year enters them — or does the answer come from an omitted argument?*

It came from an omitted argument in **nine** places, not six. All nine are deleted. The year is now
**derived** where a set already decides it (the corpus's year, the §164(b) cap), **stated** where only a
human can know it (`--year`, `OTS_YEAR`, a vector's `"year"`), and **checked** at every seam where two
statements of it could disagree.

---

## 1. Premises tested — four survived, two are REFUTED, one is materially incomplete

Per §5. Everything below was measured, not read off a comment.

| # | premise (brief §1/§5) | verdict |
|---|---|---|
| P1 | `gen_goldens.py` four `year: int = 2024` at `:262 :308 :337 :423`, literal `"tax_year": 2024` at `:618` | **HOLDS**, all five located and deleted |
| P2 | `ots_direct.py:79` `OTS_YEAR` default `"2024"` | **HOLDS** |
| P3 | **0 of 31** f6251 vectors carry an explicit `year` | **HOLDS** — measured: 31 vectors, 0 with `year` |
| P4 | the Rust reader is untyped `serde_json::Value`, so adding a `"year"` key cannot break it | **HOLDS** — `form6251.rs:692` `include_str!`, parsed at `:821 :940 :1208` as `serde_json::Value`; after adding the key, `tax::form6251::tests::every_vector_reproduces_the_form_line_by_line` **PASSES** |
| P5 | goldens corpus: one file, 227,699 bytes, `_provenance.tax_year` present and never read | **HOLDS** — measured 227,699 B / 107 households; `_provenance.tax_year == 2024`; the only reference repo-wide was the *write* at `gen_goldens.py:618` |
| P6 | `FullReturnParams` / `TaxTable` are the right things to compare against | **HOLDS, and better than the brief assumed** — both structs already carry `year: i32` (`tables.rs:543`, `tables.rs:55`), so the new gate needs **no typed year at all** |
| **P7** | ★ *"`verify_f6251.py` already threads `_year_of(v)` to **both** oracles (`:400`, `:411`)"* | **REFUTED.** `:400`/`:411` are the **OTS** leg only. The **taxcalc** leg carried two hardcoded literals — `:226 "FLPDYR": 2024` and `:246 build_calculator(rows, 2024)` — *not even defaults*, so no caller could override them. A TY2026 vector would have been scored by oracle 2 under TY2024 policy while oracle 1 scored it correctly, and the two-oracle design reads that split as **btctax** being wrong. A **third** undeclared default sat in the same file: `:372 o_year_label()` returned `os.environ.get("OTS_YEAR", "2024")` — the banner the operator reads, able to print "2024" whatever tree actually ran. |
| **P8** | the vectors file is *"a dict, vector keys id/inputs/…"* | **partially refuted (harmless).** The top level is a dict of `_source/_generated_by/_note/vectors`; `vectors` is a **JSON array**. The per-vector keys are as stated. No consequence beyond the shape of the edit script. |
| **P9** | §1's table is the complete inventory of year defaults | **INCOMPLETE.** It names six; there are **nine** live ones in the four in-scope files, plus two more out of scope (§6). `sweep.py` has **four** year-dependent module constants, not the two listed — `SALT_CAP :88` and `QBI_8995_CEILING :101` as well as `OASDI_BASE :89` / `STD_DEDUCTION :92` — and `SALT_CAP` is the one OBBBA actually moves ($10,000 → $40,000 at TY2025, per `corpus.SALT_CAP_BY_YEAR`), i.e. the most consequential single omission from the list. |

**Nothing was built on a refuted premise.** P7 *raised* the priority of `verify_f6251.py` rather than
lowering it. The brief's weighting instruction was still followed — `gen_goldens.py` and `sweep.py` are
the largest hunks — but the taxcalc leg was fixed too, because a hardcoded literal is strictly worse than
a default.

**Not disproved, and worth recording:** `OTS_DIR` is **unset** in this environment (the OTS env vars are
empty), and no `~/OpenTaxSolver*` tree was consulted. No oracle was run against a live OTS install; every
OTS-path claim below comes from reading the source and from pure selftests, exactly as §6 instructs.

---

## 2. What changed, per file

### `scripts/oracle/gen_goldens.py`

- The four defaults are now **required parameters**: `_taxcalc_row(n, i, year: int)`,
  `taxcalc_credits(households, year: int)`, `taxcalc_run(households, year: int)`,
  `_taxcalc_amt_credits(inputs_list, year: int)`. `admit(candidates, year)` takes and forwards it.
- `main()` states the year **once** and passes it to `admit`, `ots_direct.evaluate(..., year=year)`,
  `taxcalc_run(inputs, year)`, `ots_direct.version(year)` and the emitted `"tax_year": year`.
- **§3.3's real finding lives here.** `main()` had nothing to pass: it took no year, computed none, and
  the emitted label was a literal. So the year is **derived from the set that decides it**:

  ```python
  def corpus_tax_year() -> int:
      reachable = sorted(set(corpus.SALT_BY_YEAR) - set(corpus.SALT_YEARS_NOT_REACHABLE))
      if len(reachable) != 1: raise RuntimeError(...)
      return reachable[0]
  ```

  Not a typed `CORPUS_TAX_YEAR = 2024`. `corpus.py` already holds both sets, and
  `corpus.selftest_salt_axis()` already enforces their relationship **in both directions** (a bundled
  year whose axis reaches no household must be declared dormant; a dormancy note that has gone stale is
  an error). So widening the builder to a second year makes this **raise** instead of silently
  mislabelling the bake. Measured: `{2024, 2025} − {2025} = {2024}`.
- New `--selftest` (pure: no OTS, no regeneration, no network). `main()`'s stdout contract is unchanged,
  so the recipe `gen_goldens.py > full_return_goldens.json` still works verbatim and the baked
  `_provenance.regeneration` string stays accurate — **no regeneration is implied by this change.**

### `scripts/oracle/ots_direct.py`

**§3.1, argued rather than assumed.** `OTS_YEAR` names *which tree is installed on this machine*; a
caller's `year=` names *which tax year it is asking about*. Those are two facts — the file's own comment
at `:75-78` already makes that argument one level down, about `OTS_DIR` vs `OTS_YEAR`. An argument cannot
know what is on disk, so `OTS_YEAR` **stays an environment setting**; what it loses is its default.

- `OTS_YEAR: int | None` — unset now means **unstated**, not 2024.
- Two resolvers, deliberately different:
  - `_require_year(year)` — caller, else declared install, else **refuse**. Used by the *pure*,
    year-scoped defect predicate `_ots_amt_disqualified`, which reasons *about* solver years and must be
    askable about a year this machine has no install for.
  - `_require_installed_year(year)` — the same, **plus** the year asked for must equal `OTS_YEAR`. Used
    only on the path that actually **runs** a solver: `_bin`, `_template`, `run_form`, `evaluate`,
    `version`. A mismatch is now a refusal naming both years instead of a `FileNotFoundError` several
    frames down.
- `run_form` resolves the year **once** at the top and uses it thereafter; it previously re-defaulted
  `OTS_YEAR if year is None else year` in three separate places, including the Schedule-1-A sentinel
  guard at `:425`.
- `evaluate(h, *, year)` — keyword-only, resolved, threaded into **all 7** `run_form` calls inside it
  (counted by the patch script, not by hand) and into the `_ots_amt_disqualified` call at its end.
- **R21 closed.** `version()` returned `"OpenTaxSolver 2024 …"` unconditionally, and `gen_goldens.py`
  writes that string into `_provenance.oracle_1_version`, which **SPEC §11 gates golden regeneration
  on** — a version gate blind to the exact transition it guards. It is now `version(year)`, built by a
  pure `_version_string(year, release, tree)`, and it **proves** the tree can run that year by calling
  `_bin("US_1040", year)` first, so the string can no longer name a year the install cannot run. It also
  carries the tree's directory name, because the README release number ("22.07") does not say which tax
  year the solvers are for.

### `scripts/oracle/verify_f6251.py`

- `DEFAULT_FIXTURE_YEAR` **deleted**. `_year_of(v)` requires `v["year"]` and raises a message naming the
  fixture and its generator.
- All **31** committed vectors given `"year": 2024` (inserted after `id`). Purely additive: the diffstat
  for the fixture is **32 insertions, 1 deletion** — 31 keys plus the `_note` sentence.
- New `_fixture_year(vectors)`: the one year the vectors share, **refusing a mixed-year fixture** with
  the reason — `taxcalc_exact.build_calculator(rows, year)` builds one `Records` at one `start_year`, so
  a mixed fixture cannot be scored in a single pass. That boundary is *stated in the source* rather than
  papered over with a majority year (option 3 of the derive-or-hold rule).
- The two refuted hardcodes replaced: `"FLPDYR": fixture_year` and `build_calculator(rows, fixture_year)`.
- `o_year_label(vectors)` now derives the banner from the vectors and calls out a contradicting
  `OTS_YEAR` instead of printing its own default.
- New `--selftest`.
- **End-to-end regression check (taxcalc leg, real run, no OTS):** `0 unexpected divergence(s); 3 known
  and adjudicated against the form` — identical verdicts to before, and the census now prints
  `fixture years present: TY2024` from the vectors themselves rather than from a literal.

### `scripts/oracle/sweep.py`

- `--year` is **required**. It reaches the threshold table, oracle 1 (`evaluate(..., year=year)`),
  oracle 2 (`_taxcalc_amt_credits(all_inputs, year)`, `taxcalc_run(all_inputs, year)`), the `[sweep]`
  summary line, and the `reproduce:` line of every divergence report — a pasted report was previously
  **not reproducible**, because it named no year.
- The four year-dependent module constants became `_YEAR_THRESHOLDS` + `thresholds_for(year)`, modelled
  on `corpus.salt_for(year)`: **a year with no entry raises**, naming what it must supply. The §164(b)
  cap is **read from `corpus.SALT_CAP_BY_YEAR[year]`**, not retyped — the corpus already holds it per
  year with a straddle guard, and a second copy here is exactly the list that goes stale beneath a set
  that grew. Threaded through `_itemize`, `_gen_scenario`, `generate`.
- The two genuinely **unindexed statutory** amounts stay module constants and **say so**
  (`SCH_B_TRIGGER` §6012; `ADDL_MEDICARE_NIIT` §3101(b)(2)/§1411 name their dollar figures outright).
  Keying them by year would invent a variation the Code does not have.
- New `verify_year_matches_corpus(year)`, run **first** in `run_sweep` — before `OTS_DIR` is required,
  because it is pure and it is the thing most likely to be wrong. `--year` must equal the baked corpus's
  `_provenance.tax_year`, because btctax's side of the sweep comes from the compiled harness whose year
  is fixed at `crates/btctax-oracle-harness/src/main.rs:68 const YEAR: i32 = 2024`. Unchecked,
  `--year 2026` would bias every draw at TY2026 edges and score them against a TY2024 btctax and a
  TY2026 taxcalc — **manufactured divergences, in a tool whose entire output is divergences.**
- `_parser()` factored out of `main()` so the B1 kill probes the real CLI, not a look-alike.
- New `--selftest` (handled before the parser, so it needs none of the required flags).

### `crates/btctax-core/tests/golden_returns.rs`

- `baked_corpus_tax_year()` reads `_provenance.tax_year` out of `GOLDEN_RETURNS_JSON` and **panics** if
  it is absent (an unlabelled corpus is the state FR-164 ends). Read here rather than added to
  `testonly::Goldens`, which deliberately exposes only `households`.
- `years_agree(corpus_year, params_year, table_year) -> Result<(), String>` — pure, so the kill can watch
  it discriminate on real values.
- `corpus_params_and_table()` is now the **only** door to the params in this file, so a test cannot skip
  the gate by reaching for `ty2024_params()` directly. All three previous call sites (`:87`, `:522`,
  `:596`) go through it.
- **Answer to §3.2 — refuse, and name every year.** The message names the corpus year, the params year
  *and* the table year, says the corpus figures were computed by both engines under the corpus's law, and
  gives the two lawful exits: use that year's params, or regenerate deliberately per SPEC §11. Without
  the names, the symptom is a wall of tax diffs whose cause sits unread in the file.
- **No year is typed in the gate.** Expected year = the corpus's own label; actual years =
  `FullReturnParams::year` and `TaxTable::year`. A TY2026 port that swaps params in cannot leave a stale
  literal behind.

### `design/amt-form6251/gen_e2_vectors.py` — a 6th file, named and justified

Not on the brief's IN list, and touched because the change is otherwise **incoherent**: this generator
appends vectors to the same fixture, and would have emitted yearless ones the moment it next ran —
producing a fixture `verify_f6251.py` now refuses. Three additions: `FIXTURE_TAX_YEAR = 2024` (stated
once, with why `f6251_reference.py`'s constants make it 2024), `"year": FIXTURE_TAX_YEAR` in the emitted
vector, and a guard in the regression loop refusing to "reproduce" a vector from another year against
these TY2024 constants. Verified: the dry run still reports
`regression guard OK — all 31 committed vectors reproduce` and `0 vectors would be appended`.

### `scripts/oracle/check_return.py` — forced, minimal

`ots_direct.evaluate(row, year=year)` and `ots_direct.version(year)`. It already had a correct year in
hand (`--year`, defaulting to the projection's own `tax_year` and refusing to contradict it). **Its own
remaining 2024 fallback was left alone** — see §6, follow-up F1.

---

## 3. B1 — eight plants, each observed RED then GREEN

Every plant was applied to the **unplanted, working** code, watched, then reverted and re-watched. Rust
runs `touch` the test file first — the `make gate` rationale: a reverted plant can stay compiled in.

### Kill 1 — the brief's named kill: feed the TY2024 corpus another year's parameters

Plant: in `corpus_params_and_table()`, `ty2024_table()` → `ty2026_table()`.

```
        FAIL [   0.020s] (3/5) btctax-core::golden_returns deeper_lines_have_teeth_at_the_compute_level
        FAIL [   0.022s] (4/5) btctax-core::golden_returns every_golden_household_matches_the_independent_oracles
        FAIL [   0.023s] (5/5) btctax-core::golden_returns the_main_loop_reports_a_both_oracle_l16_divergence
     Summary [   0.025s] 5 tests run: 2 passed, 3 failed, 0 skipped
```

The line that proves it discriminated — not an import error, not an unrelated assertion:

```
thread 'every_golden_household_matches_the_independent_oracles' panicked at
crates/btctax-core/tests/golden_returns.rs:105:9:
the baked oracle corpus validated TY2024 (its own `_provenance.tax_year`), but this test is feeding it
FullReturnParams for TY2024 and a TaxTable for TY2026. …
```

All **three** real gates red, at the seam, with the year named. Unplanted:

```
        PASS [   0.003s] (1/5) btctax-core::golden_returns stacking_ok_guards_golden_returns_against_btctax_alone
        PASS [   0.005s] (2/5) btctax-core::golden_returns every_golden_household_matches_the_independent_oracles
        PASS [   0.005s] (3/5) btctax-core::golden_returns the_baked_corpus_refuses_another_years_parameters
        PASS [   0.005s] (4/5) btctax-core::golden_returns the_main_loop_reports_a_both_oracle_l16_divergence
        PASS [   0.006s] (5/5) btctax-core::golden_returns deeper_lines_have_teeth_at_the_compute_level
     Summary [   0.006s] 5 tests run: 5 passed, 0 skipped
```

### Kill 2 — make the gate unable to fail

Plant: `years_agree` → `return Ok(())`.

```
thread 'the_baked_corpus_refuses_another_years_parameters' panicked at
crates/btctax-core/tests/golden_returns.rs:738:10:
a TY{corpus_year} corpus fed a TY2026 TaxTable must be REFUSED: ()
        FAIL [   0.004s] (2/5) btctax-core::golden_returns the_baked_corpus_refuses_another_years_parameters
     Summary [   0.007s] 5 tests run: 4 passed, 1 failed, 0 skipped
```

The kill test reds; the three real gates stay green, because they consume a *matched* pair. ★ This plant
also exposed a cosmetic defect **in my own code** — `expect_err("…TY{corpus_year}…")` is a plain `&str`,
not a format string, so the brace text printed literally. Both occurrences rewritten; green after:
`5 tests run: 5 passed, 0 skipped`.

### Kill 3 — restore `DEFAULT_FIXTURE_YEAR`

Plant: `_year_of` → `return int(v.get("year", 2024))`.

```
  FAIL: a yearless vector was accepted — a default is back (FR-164)
  …
verify_f6251: FR-164 year plumbing FAILED
exit=1
```

### Kill 4 — a committed vector loses its year (the brief's "a vector with the wrong year")

Plant: delete `"year"` from V6 in the fixture. **Both** the selftest and the real run red:

```
  FAIL: 1 committed vector(s) carry no `year`: ['V6']
verify_f6251: FR-164 year plumbing FAILED
```

```
  File ".../scripts/oracle/verify_f6251.py", line 70, in _year_of
KeyError: 'vector \'V6\' carries no `year`. There is no default — a TY2026 vector scored under TY2024
law is exactly the silence FR-164 deletes. Add "year": <yyyy> to it in form6251_vectors.json; new
vectors get it from design/amt-form6251/gen_e2_vectors.py.'
```

Unplanted: `all 31 committed vectors state a year; the taxcalc pass will use TY2024: OK` … `exit=0`.

### Kill 5 — restore all four `gen_goldens` defaults **and** a literal corpus year

Plant: the four `year: int = 2024` signatures back, plus `corpus_tax_year()` → `return 2024`.

```
  corpus_tax_year() = 2024 (bundled [2024, 2025], dormant [2025])
  FAIL: two reachable years returned 2024 instead of refusing
  committed _provenance.tax_year == corpus_tax_year() == 2024: OK
  FAIL: _taxcalc_row ran with NO year — a default is back (FR-164)
  FAIL: taxcalc_credits ran with NO year — a default is back (FR-164)
  FAIL: taxcalc_run ran with NO year — a default is back (FR-164)
  FAIL: _taxcalc_amt_credits ran with NO year — a default is back (FR-164)
gen_goldens: FR-164 year plumbing FAILED
exit=1
```

Note claim 3 deliberately still passes: a literal `2024` happens to agree with today's corpus, which is
**why** claim 2 (the refusal) is the load-bearing one. Unplanted: all 7 lines OK.

### Kill 6 — restore R21 (the version string's hardcoded year)

Plant: `_version_string` → `f"OpenTaxSolver 2024 {suffix} …"`.

```
  File ".../scripts/oracle/ots_direct.py", line 893, in selftest_year_is_never_assumed
    assert "2025" in v25 and "2024" not in v25, v25
AssertionError: OpenTaxSolver 2024 v23.06 [tree OpenTaxSolver2025_23.06_linux64]
exit=1
```

The assertion output **is** the R21 defect string recorded verbatim in `TY2026_PORT_REPORT.md`.
Unplanted: `ots_direct: the year is never assumed (FR-164) + R21 version string OK`.

### Kill 6b — restore the `OTS_YEAR` module default. ★★ THE INSTRUMENT WAS GREEN ON IT FIRST.

Plant: `OTS_YEAR = … else 2024`. **First run: `exit=0`.** The selftest sets `OTS_YEAR` itself in order to
exercise `_require_year`, so it was **structurally blind to an import-time default** — green on the exact
defect it existed to catch (`HARNESS.md` class β). Closed by adding **claim 0**, which checks the module
binding against the environment, with its boundary stated in the source: it can only witness the *unset*
case, which is the only case a default ever applied to. Re-planted:

```
  File ".../scripts/oracle/ots_direct.py", line 875, in selftest_year_is_never_assumed
    assert saved is None, (
AssertionError: OTS_YEAR is UNSET in this environment, but the module bound 2024. A module-level default
is back: every call that passes no `year=` would silently mean that year. (FR-164 — this is the defect,
not a convenience.)
exit=1
```

Unplanted, green with `OTS_YEAR` unset **and** with `OTS_YEAR=2025` exported.

### Kill 7 — a full TY2024 fallback in `thresholds_for`

Plant: return `_YEAR_THRESHOLDS[2024]` for any year.

```
  FAIL: thresholds_for(1999) returned a table — a fallback is back (FR-164)
  …
sweep: FR-164 year plumbing FAILED
exit=1
```

Unplanted: all 6 lines OK.

### Kill 8 — disable the run-path `OTS_YEAR`-vs-argument check

Plant: `if OTS_YEAR is not None and year != OTS_YEAR:` → `if False:`.

```
  File ".../scripts/oracle/ots_direct.py", line 907, in selftest_year_is_never_assumed
    raise AssertionError(
AssertionError: RUNNING TY2026 against an OTS_YEAR=2024 tree must REFUSE, not silently run 2024
exit=1
```

### ★★ A defect the plants found in MY OWN change, before any reviewer saw it

Kill 6's first attempt did not red the way it should. Instead the **pre-existing**
`selftest_defect_years()` blew up — and `OTS_YEAR=2024` reproduced it on the **unplanted** code:

```
RuntimeError: asked for TY2025 but OTS_YEAR declares the installed tree is 2024. …
```

My first design put the env-vs-argument cross-check in one resolver used by everything, including the
**pure** year-scoped defect predicate — which `selftest_defect_years` deliberately asks about TY2025 and
TY2024 in one process, and which `verify_f6251._ots_pass` runs **before any solver**. With a perfectly
normal `OTS_YEAR=2024` exported, that pure selftest would have crashed. Fixed by splitting
`_require_year` (pure) from `_require_installed_year` (run path). Verified green under **three**
environments: `OTS_YEAR` unset, `=2024`, `=2025`.

---

## 4. Validation gate

`CARGO_TARGET_DIR=<worktree>/target-fr164 make gate` — forced rebuild, then nextest + clippy:

```
     Summary [ 120.901s] 3647 tests run: 3641 passed (6 slow), 6 failed, 12 skipped
```

Clippy `--workspace --all-targets --all-features -- -D warnings`: **passed**
(`Finished dev profile … in 20.93s`, no warnings emitted).

**The 6 failures are a worktree artifact, not caused by this change.** All six are
`xtask::bin/xtask form_delta::tests::*`, and all fail on a missing **gitignored** PDF:

```
panicked at crates/xtask/src/form_delta.rs:1127:14:
the TY2026 draft is archived and its geometry extracted: "no PDF found for f6251--2026-DRAFT"
```

`git check-ignore -v design/forms/2026/f6251--2026-DRAFT.pdf` answers
`.gitignore:71:design/forms/**/*.pdf`. The `.pdf` files exist in the main checkout and cannot exist in a
fresh worktree. My diff touches **zero** paths under `crates/xtask/` or `design/forms/`, and the failure
is a filesystem fact, so the change cannot reach it.

**Recommendation to the controller: re-run `make gate` in the main tree before folding.** That is the one
measurement this worktree structurally cannot make.

Python, all pure (no OTS, no network):

| command | result |
|---|---|
| `ots_direct.py` (unset / `OTS_YEAR=2024` / `=2025`) | OK ×3 |
| `gen_goldens.py --selftest` | 7/7 OK |
| `verify_f6251.py --selftest` | 5/5 OK |
| `verify_f6251.py` (real run, taxcalc leg) | `0 unexpected divergence(s); 3 known` — unchanged from before |
| `sweep.py --selftest` | 6/6 OK |
| `sweep.py --year 2025 --seed 1 --count 3` | refused, naming the corpus's TY2024 |
| `sweep.py --seed 1 --count 3` | `error: the following arguments are required: --year` |
| `gen_e2_vectors.py` (dry run) | `regression guard OK — all 31 committed vectors reproduce` |
| `py_compile` on all 6 touched scripts | OK |

---

## 5. What I did NOT do, and why

- **Did not regenerate the goldens corpus** (§2). Nothing required it: `corpus_tax_year()` returns the
  same 2024 the file already carries, `main()`'s stdout contract is unchanged, and the baked
  `_provenance.regeneration` recipe still works verbatim, so no generator-vs-artifact drift was
  introduced. The 227,699-byte file is untouched.
- **Did not add `ty2026_params()`** to `testonly.rs`, did not bundle TY2026 params, did not touch
  `tax_tables.rs`, `frozen_guard`'s three files, the `Schedule1aParams` `2025..=2028` guard, any form,
  map, template or `line_set`, or anything under `crates/btctax-forms/`. The kill uses the **existing**
  `testonly::ty2026_table()` — a real committed other-year artifact — plus a mutated
  `FullReturnParams::year`, which is exactly how a TY2026 params struct would differ.
- **Did not change `corpus.py`.** Not on the IN list, and not needed: `SALT_BY_YEAR`,
  `SALT_YEARS_NOT_REACHABLE`, `SALT_CAP_BY_YEAR`, `STD_DEDUCTION_2024` and `selftest_salt_axis` were all
  *read* as the authority they already are.
- **Did not fix `check_return.py:307`'s own 2024 fallback**, nor `verify_schedule_1a.py:85`'s
  `year=2025` — same class, out of scope. Filed below.
- **Did not add the `FLPDYR`-vs-`start_year` guard to `taxcalc_exact.build_calculator`** — the single
  highest-leverage remaining check, and a 7th script. Filed below.
- **Did not run the oracles live.** `OTS_DIR` is unset (verified); every OTS claim is from source and
  pure selftests.
- **Did not commit, did not push, did not touch the main checkout.**

---

## 6. Follow-ups worth filing

| id | finding | severity | proposed owning phase |
|---|---|---|---|
| **F1** | `scripts/oracle/check_return.py:307` — `year = wrapper_year if wrapper_year is not None else 2024`. A projection carrying no `tax_year`, run without `--year`, is silently scored as TY2024 by all three engines. Exactly the class FR-164 deletes; the file's own `--year` help text already argues why it matters ("fabricated a divergence on 1040 line 15"). Fix: refuse, naming `btctax income project --year`. | Important | NOW (Sep–Dec 2026), with F3 |
| **F2** | `scripts/oracle/verify_schedule_1a.py:85` — `def _rows(pol, name, year=2025)`. A TY2025 default in a second script, same shape as `gen_goldens`'s four. Whether any caller relies on it is unmeasured. | Minor | NOW, with F1 |
| **F3** | `taxcalc_exact.build_calculator(rows, year)` does **not** compare each row's `FLPDYR` to `year` — read at `:112-149`, it checks `rows` non-empty and refuses an `exact` column, nothing more. It is *"the ONE authority on building a Tax-Calculator run"*, so one loud check there makes a year mismatch unwritable across **all four** callers at once, which beats four separate disciplines. Needs a kill in its existing `selftest()`. | Important | NOW — highest leverage of the three |
| **F4** | `gen_goldens.assert_baked_provenance_is_current()` has **no caller anywhere** — grep over `scripts/`, `crates/` and the `Makefile` finds the definition at `:201` and one prose mention at `:144`, nothing else. Its docstring calls it *"the cheap half that can run any time"* and it carries its own `PLANT TO RE-RUN THE KILL` note: an instrument that has never run. B1's own subject matter. | Important | NOW — it is one line to wire into `--selftest`, and I deliberately did not, to keep this scope honest |
| **F5** | `crates/btctax-oracle-harness/src/main.rs:68 const YEAR: i32 = 2024` is the last unstated year in the sweep path: btctax's side of a live sweep is fixed at 2024 with no way to say so from outside. `sweep.py` now *checks* it indirectly, via the corpus label, but cannot *read* it. A `--year` on the harness, refusing a year it has no params for, closes the loop. | Minor (gating for a TY2026 sweep) | the TY2026 port phase |
| **F6** | `verify_f6251.py`'s taxcalc pass is one `Records` at one `start_year`, so a mixed-year fixture is now **refused** rather than supported. When the first TY2026 vector lands, that pass must be grouped by year. The refusal message says exactly this. | Minor | the TY2026 port phase |

---

## 7. Files changed (worktree left dirty — nothing committed)

```
 crates/btctax-core/src/tax/fixtures/form6251_vectors.json |  33 ++-
 crates/btctax-core/tests/golden_returns.rs                | 140 +++++++++-
 design/amt-form6251/gen_e2_vectors.py                     |  21 ++
 scripts/oracle/check_return.py                            |   4 +-
 scripts/oracle/gen_goldens.py                             | 129 ++++++++-
 scripts/oracle/ots_direct.py                              | 218 +++++++++++++--
 scripts/oracle/sweep.py                                   | 309 ++++++++++++++++++---
 scripts/oracle/verify_f6251.py                            | 149 +++++++++-
 8 files changed, 903 insertions(+), 100 deletions(-)
```

Five are the brief's IN list. `design/amt-form6251/gen_e2_vectors.py` and
`scripts/oracle/check_return.py` are the two forced by the signature changes, both named and justified in
§2; neither appears on the brief's OUT list.

**Reproduce the kills:** `.venv/bin/python scripts/oracle/ots_direct.py`,
`.venv/bin/python scripts/oracle/gen_goldens.py --selftest`,
`.venv/bin/python scripts/oracle/verify_f6251.py --selftest`,
`.venv/bin/python scripts/oracle/sweep.py --selftest`, and
`CARGO_TARGET_DIR=<dir> cargo nextest run -p btctax-core --test golden_returns`.
