# BRIEF — stage 2, the throwaway TY2026 run. Tiers A and B.

Design: `design/TY2026_REHEARSAL_DESIGN.md` (read it, including the ★★★ **CORRECTED** block in §2).
Prediction, frozen: `design/agent-reports/TY2026-BLOCKERS-PREDICTION-2026-09-13.txt` (66 rows).
The filable-year gate is already landed and its kill verified.

## The deliverable is THE GAP, for both tiers

| | |
|---|---|
| predicted **and** hit | the command works; no news |
| predicted, **not** hit | it over-reports, or the blocker is already cleared |
| ★★★ **hit, NOT predicted** | **the entire point.** An unknown unknown |

FR-102 (an HSA filer who could not print a page), FR-218 (a duplicate blank page found only on paper) and
this week's five walls were all in the third row. **A run that only confirms rows already in the prediction
file has taught nothing, and must be reported as having taught nothing.** Do not pad.

## The one inviolable rule about faking

**Substitute a document we HAVE for a document we LACK. Never invent a figure.** Copying TY2025's Schedule A
in place of TY2026's is a legitimate fake — its collisions are *known* (Schedule A moves six line meanings,
Form 6251 eight), so the output is wrong in ways we already understand, and we are hunting **walls**, not
figures. Writing a threshold nobody printed is **not** legitimate, ever.

★★ And note: **the set of things you must fake is itself a blocker list**, derived by necessity rather than
by analysis. Record it precisely and compare it to the prediction — that comparison is half the value.

## This run validates NOTHING

OTS-2026 does not exist until ~2027-01-27 and substituted templates carry no authority, so **no figure this
run produces may be called correct.** Say so in the report. This is not S1; S1 has a signed return as its
answer key.

---

## Tier A — library, UNBUNDLED, may stay in the repo

**OWNS:** a new test/harness under `crates/btctax-core/tests/` (and `crates/btctax-forms/tests/` if the
emitter is reached). **Do NOT touch `crates/btctax-adapters/src/tax_tables.rs`** — Tier B holds it, and
bundling is what Tier A exists to avoid.

Drive compute → the printed chain → the emitter with `ty2026_full_return()` injected **at the call site**,
never through `by_year`. `testonly::ty2026_table()` is the existing precedent for an unbundled year
artifact. Nothing can leak because nothing enters the bundle.

Use the owner's real profile shape: **W-2 wages + Bitcoin dispositions + ITEMIZED Schedule A + Schedule B
(interest/dividends over $1,500)**; no Schedule C, no retirement. ★ Itemizing is load-bearing: `deduction`
is a `max` of standard vs itemized, so only an itemizing profile reaches Schedule A, the §164(b) SALT
worksheet and the charitable ceilings.

★ Hunt specifically the things TY2026 changes that a TY2024 run cannot reach: Schedule A's **5e** SALT cap
$40,000 → $40,400 with its phase-out $500,000 → $505,000 (FR-219), the itemized total moving **17 → 18**
behind the §68-style **$384,350** gate with two worksheets that do not exist, Form 6251 **line 5**'s
exemption phase-out drop, and Schedule 1 line 14's widened eligibility (FR-220). **Does the compute chain
notice any of them, or does it silently use TY2025 shapes?**

## Tier B — the CLI journey, BUNDLED, BRANCH-ONLY, NEVER MERGED

**OWNS:** whatever it takes, on a branch. ★★ **Nothing from Tier B is folded into `main`** — its only
deliverable is the report. The design says: *delete the branch; keep the report and the gate.*

**Expect the gate to red — that is the design working, not a problem.** Record its message verbatim as
evidence the construction held, then proceed on the branch.

`init → import → income import → export-irs-pdf`, the real binary, because that is where the **filer-facing**
walls live — the five from this week, and refusals whose exits do not work. Tier A cannot reach them.

★ What you will have to substitute, discovered rather than assumed. Known starting points:
`forms_bundled` is **0** for TY2026 (no maps at all), the price dataset ends **2026-06-03**, and no
`f1040--2026` / `i1040gi--2026` extract is archived. Find the minimum viable substitution set, **state each
item and what it was substituted from**, and stop at the first wall that cannot be substituted honestly —
**that wall is a finding, not a failure.**

---

## Rules for both

Own worktree, `CARGO_TARGET_DIR=<worktree>/target-<a|b>`, never `/tmp` (32 GB tmpfs). **FOREGROUND every
command** — FR-175: two agents have stalled by backgrounding a gate, the second with the prohibition in its
own brief, and the report is what a stall destroys. `make check` excludes `cargo fmt --all --check`; run
both **for Tier A**. Capture once, grep. **Do not commit, do not push.** No subagents.
★ Two distinct concurrency failures, do not conflate: `ld.lld: undefined hidden symbol` from a stale
`.rlib` → `cargo clean -p <crate>`; `signal: 9, SIGKILL` → memory, run nextest and clippy serially.
★★ **Eleven briefs in this arc were refuted by their implementer, eight of them mine. If you can disprove a
premise — including anything in the design doc — STOP and report it.** It has paid every single time,
including on this very design (§2's "clears itself" claim was falsified by a plant).

Deliverables, Bash heredoc (**not** `Write`): `design/agent-reports/REPORT-stage2-A.md` and `-B.md`.
