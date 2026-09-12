# BRIEF — FR-122 and FR-124: two armed traps, both aimed at January

**Owner approved 2026-09-12.** Both are filed **Minor** and both are **armed rather than live** — which is
exactly why they are being done now instead of when they fire. Each one's entry already names its fix, so
the work here is small and **the value is in the kills**, not the edits.

You are ONE opus agent in the **shared main tree**. Nothing is committed while you work. Two items. Do not
widen scope.

---

## 0. Stop-and-report

**Stop and report rather than build on any premise here you can disprove.** Ten briefs in this arc have been
refuted by measurement, five of them the coordinator's. A refuted premise is a first-class finding.

## 1. FR-124 — the inert `exact` flag, and the duplication it now creates

Read the entry in `FOLLOWUPS.md`. Summary of its measurement: `gen_goldens.py:251-257` adds
`**({"exact": 1} if year in TAXCALC_EXACT_YEARS else {})` to a Records **DataFrame column**, but `exact` is
a *calculated* variable in taxcalc's `records_variables.json`, not a read variable — so
`tc.Records(data=DataFrame(...))` **silently drops it**, the array comes back all zeros, and every stepped
phase-out takes taxcalc's **smooth marginal-rate fallback** instead of the stepped branch a tax form
performs. `TAXCALC_EXACT_YEARS = frozenset({2025})` is therefore inert.

The fix is named in the entry: write it **through the Calculator** after `advance_to_year` —
`calc.array("exact", np.ones(n, dtype=np.int32))` — which does stick.

★★ **But the more important part is that the correct mechanism ALREADY EXISTS in this repo**, in
`scripts/oracle/verify_schedule_1a.py`, which sets it that way *and asserts it stuck*. So a naive fix leaves
**two implementations of "turn `exact` on", one right and one that was wrong for a year.** Make **one** of
them authoritative — extract the helper and have both call it — so they cannot diverge again. That is
`CLAUDE.md`'s *"derive the list, or make the compiler hold it"* applied to a procedure rather than a list,
and it is the difference between fixing this once and fixing it twice.

**Kills.** Not "the flag is set" — that is what the broken code also looked like:

1. **It stuck**: assert `calc.array("exact")` is all ones *after* the write. Plant the old DataFrame-column
   form → red.
2. ★ **The stepped branch actually ran.** The symptom was a *smooth* value, so the kill must detect the
   branch, not the flag: on a fractional-step vector the stepped result and the smooth result differ, and the
   test must red when the smooth one comes back. `verify_schedule_1a.py`'s `_smooth_fallback()` already
   expresses that comparison — reuse it rather than restating it.
3. The `TAXCALC_EXACT_YEARS` set must stop being able to be inert: if a year is in the set, the assertion
   must run for that year. Plant a year in the set that is never asserted → red.

## 2. FR-122 — the whole-table check that nothing runs

`line_coverage_check::run()` — the only caller of `check(&line_coverage::all())` — is reachable **only** from
`cargo run -p xtask -- line-coverage`, and `grep -rn 'line-coverage' .github Makefile` returns nothing. Its
`mod tests` exercises the *rules* on synthetic tables; **the 377-row table's verbatim check against
`design/forms/extract/` is unheld.** The module's own recorded question answers itself badly here: *"which
test reds when this checker is removed?"* — none, for the table as a whole.

Fix: a test that calls `run()` (the `forge_reach_check.rs` precedent — `#[cfg(test)]` in `xtask`, so
`make gate` runs it). The table is green today: measured `line-coverage OK: 377 money lines`.

**Kills.**

1. Plant a **rotted sentence** in one `line_coverage` row — a doc comment that no longer matches its
   extract — and show the new test reds naming the row. Today nothing reds.
2. ★ **Anti-vacuity**: the test must fail if the table ever becomes *empty* or the extract set unreadable,
   rather than passing by checking nothing. Plant an empty table → red. This is the floor `forge_reach_check`
   pins, for the same reason.
3. Say in the report **how long the added check takes**, measured. If it materially slows `make check`,
   say so with the number and propose where it belongs instead — do not silently make the fast gate slow.

## 3. Scope and mechanics

Two items. Anything else goes to `FOLLOWUPS.md` with an owning phase. Do not bundle params, do not touch the
fail-closed gates, do not touch the corpus's year (that is FR-128, owned elsewhere), and do not start FR-132.

**Main tree. Do NOT commit, push, `git stash`, `git checkout` or revert anything** — a builder in this arc ran
`git stash` against its brief. Revert a mutation with a **cp backup**. **No subagents.** No `--no-verify`.
Never hand-count what a tool can count, and never quote a number or list from a `head`/`tail` view.

`.venv/bin/python` for anything taxcalc (6.8.2, version floor enforced); bare `python3` has neither taxcalc
nor pandas. Baseline: `make gate` **3609 passed / 12 skipped**, fmt clean. Report numbers as numbers.

## 4. Your report — final action

Write `design/agent-reports/REPORT-build-fr122-fr124.md`: per item, what changed and where (`file:line`) and
**why that mechanism**; how you made the `exact` procedure single-authoritative; kills with pasted
red-then-green for all six; **the measured cost of the new whole-table check**; refuted premises; residue with
owning phases; the literal gate numbers. Return only a short summary plus that path.

★ If your harness refuses the report write — it happened to an agent on 2026-09-11 — say so first and return
the text rather than silently skipping it.
