# BRIEF — FR-164: make the tax year a REQUIRED argument through the oracle harness

**Tier:** opus. **Isolation:** your own worktree. **One agent.** Do not spawn subagents.
**Owning phase:** NOW (Sep–Dec 2026), before the January TY2026 census. Off the critical path.

## 0. The one question

> Can the oracle harness and the golden corpus still answer *"which tax year did this validate?"*
> when a second year enters them — or does the answer come from an omitted argument?

Today it comes from an omitted argument in four places. Your job is to **delete the defaults**, not to
build an instrument and not to refactor. If a change starts looking like a new instrument, stop.

## 1. Already machine-verified — do NOT re-derive any of this

Measured by the controller 2026-09-13 and committed in the ledger at `557be34de`. Spend no budget
re-measuring; spend it on the design questions in §3.

| fact | value |
|---|---|
| `gen_goldens.py` `year: int = 2024` defaults | four, at `:262`, `:308`, `:337`, `:423` |
| `gen_goldens.py` hardcoded literal | `"tax_year": 2024` at `:618` |
| `ots_direct.py` | `:79` `OTS_YEAR = int(os.environ.get("OTS_YEAR", "2024"))` |
| `verify_f6251.py` | `:54` `DEFAULT_FIXTURE_YEAR = 2024`; `:59` `_year_of(v)` |
| **f6251 vectors carrying an explicit `year`** | **0 of 31** — every one rides the default |
| vectors file | `crates/btctax-core/src/tax/fixtures/form6251_vectors.json`, a dict, vector keys `id/inputs/derived/form6251/why/attach_required_who_must_file_cond1` |
| **the Rust reader** | `form6251.rs:692` `include_str!`, parsed as **untyped `serde_json::Value`** — ★ so adding a `"year"` key CANNOT break it; there is no `deny_unknown_fields` struct in this path |
| `sweep.py` module-level | `:89` `OASDI_BASE = 168_600`, `:92` `STD_DEDUCTION = corpus.STD_DEDUCTION_2024` |
| `golden_returns.rs` | `ty2024_params`/`ty2024_table` at `:39`, `:87`, `:522`, `:596`; `_provenance.tax_year` **never read** |
| goldens corpus | ONE file, `crates/btctax-core/tests/goldens/full_return_goldens.json`, 227,699 bytes |
| `ty2024_params\|ty2024_table` references | **354** across `crates/` + `scripts/`, vs **4** for the TY2026 pair |
| the model to copy | `corpus.py::salt_for(year)` — refuses an unknown year |

★ **`verify_f6251.py` is the BEST of the four, not the worst.** It already threads `_year_of(v)` to both
oracles (`:400`, `:411`), prints the fixture years present (`:297`), and year-keys each oracle's defect set
(`OTS_YEARS_WITH_STALE_MFS_KICKER`, `:265`). Its `:53` comment states the boundary honestly. So its default
is the one that matters *least*; `gen_goldens.py` and `sweep.py` are where the year is genuinely unstated.
Weight your effort accordingly.

## 2. Scope — exactly this, nothing adjacent

**IN:**
1. `scripts/oracle/gen_goldens.py` — the four defaults become required parameters; `"tax_year": 2024`
   becomes the year actually passed.
2. `scripts/oracle/ots_direct.py` — the year reaches every call explicitly. Make `version()` report the
   tree it **actually ran** (this is R21; SPEC §11 gates regeneration on that string), so a mismatched
   `OTS_DIR`/`OTS_YEAR` is visible rather than inferred.
3. `scripts/oracle/verify_f6251.py` — give all 31 vectors an explicit `"year": 2024` so
   `DEFAULT_FIXTURE_YEAR` becomes unreachable, then delete it. (Cheap and safe: the Rust reader is untyped.)
4. `scripts/oracle/sweep.py` — the module-level 2024 constants become year-derived or explicitly
   year-parameterised.
5. `crates/btctax-core/tests/golden_returns.rs` — **read the `_provenance.tax_year` the corpus already
   carries** and refuse when it disagrees with the params it is being fed.

**OUT — do not touch, and say so if tempted:**
- Any form, map, template, `line_set`, or anything under `crates/btctax-forms/`.
- The per-year Rust param fns in `tax_tables.rs` / `testonly.rs`. Do **not** bundle TY2026 params. Do
  **not** add a `ty2026_params()` to `testonly.rs` — the 354-reference single-year surface is what
  `ty2024_params` became, and new years enter through `bundle.full_return_for(y)` / `covered_years()`.
- `frozen_guard`'s three files (`types.rs`, `compute.rs`, `se.rs`). Named here so nobody tidies them.
- `Schedule1aParams`'s `2025..=2028` identity guard. Correct as-is for ports 2 and 3.
- `LONG_RANGE_PLAN_filing.md` §7 stands: no e-file, no state, no new credits.
- Do not regenerate the goldens corpus. If a change would require regenerating it, **stop and report** —
  a golden cannot validate its own regeneration.

## 3. The design questions worth your tier

1. **Required argument vs. explicit-and-checked.** For `ots_direct.py`, `OTS_YEAR` is an *environment*
   input naming an external install. Making it required at every call may be right; making `version()`
   honest may be sufficient. Argue it, don't assume it.
2. **What should happen on a disagreement** between `_provenance.tax_year` and the params fed to
   `golden_returns.rs` — refuse, or refuse-with-a-named-year? Refusing is the point; the message is where
   the value is.
3. Does removing a default leave any caller that now has **nothing to pass**? That caller is the real
   finding — it is a path where the year was never known, only assumed.

## 4. B1 — the kill is not optional

Every change here must land with a test **observed RED on a planted defect**:
- The named one: feed the TY2024 corpus to TY2026 params ⇒ the new check reds. Plant it, watch it red,
  unplant it, watch it green, and **paste both outputs** into your report.
- For a Python change, the equivalent is a vector or invocation with the wrong year that the script now
  refuses and previously accepted.
- ★ A kill that reds for the wrong reason (import error, typo, unrelated assertion) is not a kill. Say
  which line of output proves the checker discriminated.

## 5. Stop-and-report — this has paid FOUR times in this arc

If you can **disprove any premise in this brief**, stop and report it rather than building on it. Four
briefs in this arc carried a refuted premise, two written by the controller. The generalisation:
*a hand-written scan over one syntactic form is not a measurement of a set produced by another.*
Specifically worth testing: that all four defaults are genuinely reachable; that `verify_f6251.py`'s
default is truly load-bearing given `_year_of`; and that the goldens file's `_provenance.tax_year` says
what I claim it says.

## 6. Working rules

- Work in **your own worktree**. Set `CARGO_TARGET_DIR` to `<your-worktree>/target-fr164` — the repo's
  `.gitignore:61` glob `target-*/` already covers it. **Never** put a target dir in `/tmp` (32 GB tmpfs;
  one filled it and killed a running test) and never outside the ignore glob (109 GB of build artifacts
  reached a commit this way and had to be filter-branched out).
- Run `make check` (nextest + clippy, ~20s) — **not** `cargo test --workspace`. Capture once to a file and
  grep it twice; never run a suite twice to collect counts and failures separately.
- The Python stack is `.venv/bin/python` (taxcalc 6.8.2 + pandas). Bare `python3` has neither.
- **Do not commit and do not push.** The controller persists, ledgers and folds. Leave your worktree dirty
  and say exactly which files you changed.
- Do not run the oracles against a live OTS install unless `OTS_DIR` is already set; if it is not, say so
  and verify by reading, not by inventing a fixture.

## 7. Deliverable

As your **final action**, write your report with a Bash heredoc (`cat > <path> <<'MARKER'` … `MARKER`) —
**not** the `Write` tool — to:

    design/agent-reports/REPORT-build-fr164-oracle-year-required.md

Return only a short summary plus that path. The report states: what changed per file; the kill output
(red and green, pasted); every premise you tested and whether it survived; what you did NOT do and why;
and any new follow-up worth filing, with a proposed owning phase.
