# BRIEF — FR-234: the $1 SE residual is adjudicated; fix the four INSTRUMENT defects behind it

You are implementing. One task, four defects, all in the oracle harness — **not** in the tax
computation. Work in your own worktree. Do **not** spawn subagents of your own.

## Facts already SETTLED by the controller — do not re-derive, do not re-litigate

1. **The FIGURE is correct. btctax is right.** `ca_A_single_int-none_se-over_ltcg-0_qd-0`:
   Schedule SE L10 = 13741.68, L11 = 3213.78, and btctax prints **L12 = 16956**.
   The form (`design/forms/extract/f1040sse--2024.txt:45-47`) says L10 and L11 are entry lines and
   L12 is *"Self-employment tax. **Add lines 10 and 11**."* The IRS rounding rule
   (`design/forms/extract/f4868--2024.txt:217-223`, verbatim) is *"If you do round to whole dollars,
   you must round all amounts"*, so the L10 and L11 BOXES hold 13742 and 3214 and L12 adds the boxes
   → 16956. **Do not "fix" the SE computation. `se.rs` is correct and keeps exact cents.**
2. **taxcalc's 16955 is a lawful DIFFERENT methodology**, not a defect: it keeps cents and rounds the
   total once (16955.46 → 16955). This is the Σround ≠ roundΣ residual.
3. **The OTS leg is a legitimate witness.** `se_l12_ots = sum_round(&[l10, l11])`
   (`crates/btctax-oracle-harness/src/main.rs:342-345`) reconstructs OTS's *paper* L12 from OTS's
   *paper* legs, which is the correct paper-to-paper comparison. Leave it alone.
4. `#[ignore]` on `check_mode_reconciles_every_line_of_every_admitted_golden_household`
   (`crates/btctax-oracle-harness/tests/smoke.rs:203`) promises *"run in CI"*. Controller verified:
   **zero** `run-ignored` mentions in `.github/workflows/`. The promise is false.

## The four defects to fix

### D1 (the one that matters most) — `verdict_engine` MISATTRIBUTES the witnessing engine
`main.rs:787-796` passes `Some(target)` as `verdict`'s `ots` parameter and `None` as `taxcalc`
**unconditionally**, ignoring its own `engine` argument. So a taxcalc-compared row emits
`"ots": <taxcalc's number>, "taxcalc": null, "engine": "taxcalc"`. There are **14 call sites**.

This is a defect in *what the instrument claims to have done*, which is still-blocking per
`CLAUDE.md` severity. It is not theoretical: **it misled the controller during this very
investigation** into briefly concluding OTS was the outlier. Route the value into the field named by
`engine`. ★ Consider whether `ots`/`taxcalc` as two fixed columns is even the right shape for a
single-engine verdict — if a third engine ever appears the same bug returns. State your choice.

### D2 — the corpus sweep ABORTS on the first failing household
`smoke.rs:146` is an `assert_eq!` **inside** the per-household loop, so the test reports ONE
household and stops. It cannot answer "how many households diverge, and do they share a mechanism?"
— the difference between one lawful $1 residual and a systemic wrong figure. Collect every
household's divergences, then assert once at the end with the full roster in the message.

### D3 — the taxcalc SE-L12 divergence must be expected BY MECHANISM, never by name or tolerance
`CLAUDE.md` is explicit: *"An excuse list keyed by VECTOR NAME is a liability"* and *"state the
mechanism, let it decide, never enumerate the outcomes you happened to see."*

So: do **not** add a $1 tolerance, and do **not** list household names. **Compute** the expected
residual from the mechanism — `sum_round(legs) - round_leaf(total)` — and reconcile the taxcalc leg
iff the observed difference equals that computed residual **exactly**. A divergence of any other
size, or on a line with no such leg structure, must still FAIL. The same shape applies to
`f8959_l18_ots` (`main.rs:346-349`), which has the identical two-leg structure — handle it too.
★ The residual is only defined where the legs are present; where they are not, the comparison must
stay strict rather than silently permissive.

### D4 — give the ignored test a REAL runner, or stop promising one
Either add a CI job that actually runs it (`--run-ignored`), or change the `#[ignore]` reason to say
what is true. Prefer the runner. ★ The loop spawns a subprocess per household over ~107 households
and its own doc comment calls it *"inherently serial"* — this box has **24 cores** and the standing
directive says parallelise exactly this shape. Parallelising it is likely what makes a CI runner
affordable, so treat D4 and the parallelisation as one piece of work.

## Required of every fix — B1, seen-red-once
Each of D1–D4 lands **paired with a test that plants the exact defect and is observed RED**, then
green after the fix. For D1 that means a row whose engine is taxcalc and whose value must NOT appear
in the `ots` field. For D3 it means a planted divergence of the WRONG size on a leg-bearing line,
which must still fail. ★ Per FR-235, do not write the plant in the checker's own vocabulary — a plant
that only exercises what the checker already looks at measures nothing.

State in your report, for each of D1–D4, the one sentence: **"which test reds when this fix is
reverted?"** — and paste the observed red output, not a claim about it.

## Gate — run these SERIALLY and paste real output
    cargo nextest run --workspace          # FR-223: nextest and clippy CONCURRENTLY get SIGKILLed
    cargo clippy --workspace --all-targets -- -D warnings
    cargo fmt --all --check
    cargo nextest run -p btctax-oracle-harness --run-ignored all   # the corpus test must now pass
★ **Never background a gate command and end your turn** (FR-175: two agents have stalled exactly
this way). Run it in the foreground and wait, even if it takes minutes.

## Do not touch
- `design/agent-reports/TY2026-BLOCKERS-PREDICTION-2026-09-13.txt` (frozen stage-2 baseline)
- anything in `design/forms/extract/` (archived IRS text) or any existing `design/agent-reports/REPORT-*`
- the SE computation in `crates/btctax-core/src/tax/se.rs`

## Final action — persist your report
Write `design/agent-reports/REPORT-fr234-se-rounding-and-runner.md` as your LAST action: what you
changed per defect, the four "which test reds" answers with pasted red output, the gate output, the
corpus-wide divergence roster D2 now produces, and anything you found that this brief did not predict.
Return only a short summary plus that path. Do not commit; leave the work in your worktree and say so.
