# BRIEF — re-verifying the provenance fold (`99468b5d`)

**Artifact:** `99468b5d`'s own diff — 11 tracked files **+240/−46**, plus `crates/xtask/src/forge_reach_check.rs`
and its report. Not the earlier folds (re-verified clean), not the rest of the branch. **This is the round
that closes step 5 if it comes back 0C/0I**, so a false PASS here ships the gap.

You are ONE sonnet verifier in an isolated worktree. Read, in order:

1. `design/agent-reports/FOLD-step5-f6251-provenance.md` — what the fold claims.
2. `design/agent-reports/REVERIFY-step5-f6251-fold.md` — the Important it answers (0C/1I).
3. `design/agent-reports/REVERIFY-step5-f6251-fold-VERIFICATION-AND-FOLD-BRIEF.md` — the ledger and the
   brief the fold worked from, **two of whose premises the fold refuted**.
4. `git show 99468b5d --stat`, then the diff. Also `CLAUDE.md` (B1; "derive the list, or make the compiler
   hold it").

## The one question

> **Can a `SeniorDeductionSubtotal` carrying a line number no revision printed reach the join by ANY route
> — and is the new guard capable of failing?**

## Already settled — do not re-spend budget

| settled | by |
|---|---|
| `make gate` **3596 passed / 12 skipped, exit 0**, fmt clean, `xtask line-coverage` 377 / f6251:43, ratchets 31/0/17 unmoved | controller, main tree at this commit |
| in-crate forging is blocked: planting it in `return_1040.rs` gives `error[E0451]: fields amount and schedule_1a_line … are private` | **controller planted it** |
| the forge-reach guard reds on a **real production call**, quoting the offending `file:line` | **controller planted it** |
| the guard reds when **both** kill call sites are neutralised (anti-vacuity), naming both possible causes | **controller planted it** |
| a *comment* mentioning the forge does **not** red — one of four deliberately pinned near-misses | controller (a deliberately weak plant, diagnosed as weak) |
| there are **five** external construction sites, and the ledger's table listing four was wrong | the fold refuted it; controller confirmed by unpiped `git grep` |
| `a_revision_citing_line_43_refuses_a_figure_read_off_line_37` calls the join with **bare integers** and constructs no enum | same |

★ **Do not run the suite to reproduce those.** A worktree suite result is not comparable to the main tree's:
on 2026-09-09 a verifier reported 7 failures in a worktree and all seven were environmental (gitignored
`design/forms/2026/` PDF fixtures are not shared across worktrees; the mandated
`CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review` trips a hook-lookup test). Never report a suite
red without showing the mechanism is in the code.

## What to hunt

1. ★★ **Every route to a forged subtotal, derived — not the ones the fold thought of.** `SeniorDeductionSubtotal`
   is now carried *inside* `Form6251Line1::Y2025`, so ask the transitive question: does anything in the chain
   (`SeniorDeductionSubtotal`, `Form6251Line1`, `Form6251`, or a struct containing them) derive or implement
   `Deserialize`, `Default`, `From`, `Into`, `FromStr`, `Arbitrary`, or a builder that could produce one
   without going through `Schedule1A::senior_deduction_subtotal`? **Serde is the specific hole**: this crate
   round-trips through TOML, and a `Deserialize` anywhere in that chain would let a file supply the pair. If
   any route exists, the join is decoration — Critical.
2. ★ **Is the guard's scan complete, and is its stated blind spot honest?** It claims to scan "the production
   half of every `.rs` file". Derive what that means in the code: how does it split production from test —
   `#[cfg(test)] mod tests` only? What about an inline `#[test]` fn outside such a module, a `#[cfg(test)]`
   `impl`, doc-tests, `build.rs`, `benches/`, `examples/`, or a macro-generated call? A checker whose own
   partition is wrong reports success over the region that matters (that is FR-114's shape, twice folded in
   this repo). Read its header's stated blind spot and judge whether it matches what the code does.
3. **Did each of the five construction sites get the RIGHT treatment?** The brief required checking per site
   whether it needs a legitimately vouched value or a deliberately forged one. Verify no legitimate site was
   handed the forge, and that the site needing a mismatch (`f6251_obbba.rs:1129`) still produces one.
4. **The emitter's destructure.** The fold's argument against `#[non_exhaustive]` on the variant was that it
   would force `..` there, blinding the one place that must see a field added. Confirm the destructure is
   still exhaustive — no `..` — so a future field addition reds there.
5. **The three corrected doc comments** — are they now true? One was invalidated by the fix itself. False
   comments were the root cause of this whole round; a comment wrong in a new way is the same defect.
6. **FR-123's deferral reasoning** — the fold says typing the join's `u32` parameter today "would put a forge
   call in a `src/` file and weaken the guard just installed". Is that actually true, or is it deferring
   something that belongs in this fold?
7. **Scope creep** in either direction — anything beyond the one finding, or any part of it silently dropped.

## Severity

**Critical** — wrong result, data loss, an unmet guarantee, or a defect in what a tool *claims* to have done
(a gate that cannot fail, a refusal that does not refuse, a false PASS). **Important** — a real defect,
missing case, unsound assumption. **Minor/Nit** — recorded. ★ Secret-handling defects never gate. A **blank**
is the normal case; assert provenance, never non-blankness.

**Stop and report rather than build on any premise here you can disprove.** Eight briefs in this arc have
been refuted by measurement; four were the coordinator's, and the most recent — the one this fold worked
from — was refuted twice by the implementer. If a "settled" row above is wrong, that is your most valuable
finding.

★ One discipline the controller learned twice today, which applies to your own plants: **a plant that does
not red must be diagnosed before you call the instrument blind.** Two of the controller's plants failed to
red and both were too weak, not the checker's fault.

## Mechanics and output

Worktree only; `CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review` if you build. Do not commit, do
not push, edit no file but your report. **Do not spawn subagents.**

Write `design/agent-reports/REVERIFY-step5-f6251-provenance.md` as your **final action**, then return only:
the counts, the **absolute path** of the report and of your worktree root, and at most five lines of summary.
Do not return the report inline.

Structure: **Verdict** · **Findings** (severity, `file:line`, mechanism, concrete failure scenario) ·
**Refuted premises** · **Checked clean** — one line per numbered hunt naming what you actually looked at ·
**Out of scope**.
