# Oracle-sweep — CONTINUITY (resume point for execution)

*Written 2026-07-16, end of the design+plan session. **Safe to clear context here** — the spec and the plan
are both GREEN and committed, all reviews persisted, memory updated. Nothing is in flight.*

## Where things stand

- **SPEC:** `design/SPEC_oracle_sweep.md` — **GREEN 0C/0I** (commit `ab41b36`). Fable review loop
  r1 2C/6I → r2 0C/2I → r3 0C/2I → r4 0C/1I → r5 0C/0I, all persisted verbatim in
  `design/oracle-sweep/reviews/SPEC-oracle-sweep-fable-r1..r5.md`.
- **PLAN:** `design/IMPLEMENTATION_PLAN_oracle_sweep.md` — **GREEN 0C/0I** (commit `263e29f`), 13 tasks.
  Fable loop r1 1C/4I → r2 0C/4I → r3 0C/0I, persisted in `.../reviews/PLAN-oracle-sweep-fable-r1..r3.md`.
- **NOT YET EXECUTED.** No code written yet; the 12-household baseline is untouched. Execution is the next
  and only remaining phase.

## What the project is

Extend btctax's existing paper-vs-oracle test (`crates/btctax-forms/tests/golden_packet.rs`, already holding
the filled 1040 against OpenTaxSolver for 12 households) into a scaled, deeper, **two-oracle** differential
harness whose btctax side is read **off the filled IRS PDF**, held against OTS **and** PSL Tax-Calculator,
with a variable-strength generated corpus (~80–120 + 12 anchors + 2 pinned cells) and a non-CI live sweep.

## How to resume — execute the plan

Use **superpowers:subagent-driven-development** to execute `design/IMPLEMENTATION_PLAN_oracle_sweep.md`
task-by-task (a fresh implementer per task, a Fable review after each, `make check` green at every
boundary). The plan is self-contained; read it + this doc + the auto-memory `[[oracle-sweep-project]]`.

**Three phases (build order preserves `make check` green):**
- **Phase A — T1–T7 (pure Rust, HERMETIC, no oracle toolchain needed):** the `oracle_diff` reproduction
  helpers (the per-line reproduction table is in the plan), the divergence-class machinery (per-oracle
  provenance + taxcalc methodology + stacking + liveness + `KnownDefect`), the on-paper sign/blank read-back
  (`tests/common/`), the evolved `golden_returns.rs` (compute level) + `golden_packet.rs` (paper level,
  sharded, derived form-sets), and the test-only `oracle_harness` bin. Proves the whole comparison machinery
  against the existing 12 anchors, with deeper/provenance assertions inert (serde-default `Option`).
- **Phase B — T8–T11 (OFFLINE, needs the oracle toolchain):** extend `ots_direct.py`/`gen_goldens.py` (deeper
  lines + the C1 component legs + the §1211 8960 fix), build the covering-array generator + the two pinned
  liveness cells + D-2 admission, then **regenerate the baked corpus** and switch the deeper assertions on.
- **Phase C — T12–T13:** the live `sweep.py` + the §12 validation KATs (fault-injection, deeper-line teeth,
  determinism).

**Toolchain prerequisite for Phase B (T8+):** the OTS 2024 binaries + a `taxcalc` venv. As of this session
an OTS install was at `…/scratchpad/OpenTaxSolver2024_22.07_linux64` (scratchpad is session-specific — may
be gone) and a venv at `/scratch/code/.venv`. **At T8, first locate/confirm both:** `taxcalc` must import
(`/scratch/code/.venv/bin/python -c "import taxcalc"`) and `OTS_DIR` must point at the OTS install; the
regeneration recipe is in `scripts/oracle/gen_goldens.py`'s header. If missing, ask the user to provide
them. Phase A needs none of this, so start there regardless.

## Standing constraints (do not violate)

- **FROZEN — never edit:** `crates/btctax-core/src/tax/{types,compute,se}.rs`. No change to the compute
  engine, the fillers, or the map TOMLs. The harness READS/REPRODUCES btctax's printing; it never alters it.
- **`make check` (~6s warm) is the gate; keep it green at EVERY task boundary.** Hermetic — never let
  `make check` touch the OTS binary or the venv (oracle answers stay baked in the JSON).
- **★ Caught bugs FILE FOLLOWUPS, NOT inline fixes (user-mandated).** If T11's re-bake or the T12 sweep
  surfaces a genuine btctax fill/compute bug: file a `FOLLOWUPS.md` entry (severity + owning phase) and pin
  the scenario as a **declared known-defect divergence** (btctax's current value, `KNOWN DEFECT → <FU-id>`,
  oracle figures beside it) to keep `make check` green with the bug tracked. Never weaken/skip a test; never
  fix compute/fill in this plan (spec §10). An **oracle-driver** error (e.g. a mis-parsed line) is a DIFFERENT
  cause → fix the driver, never a false btctax pin (plan T11 step 3's four causes).
- **MFS deferred; AMT / dependents / credits OUT; TY2024 only; domain {Single, MFJ}, refusal-free.**
- **Reviews use Fable** (standing user directive). Per task: TDD, mutation-check each new guard.
- **Progress ledger:** subagent-driven-development writes `.superpowers/sdd/progress.md` once building starts
  — that is the durable per-task record; trust it + `git log` over recollection after any compaction.

## The kick-off prompt (paste into a fresh session in this repo)

> Execute the GREEN oracle-sweep implementation plan (`design/IMPLEMENTATION_PLAN_oracle_sweep.md`)
> subagent-driven (superpowers:subagent-driven-development). Read `design/oracle-sweep/CONTINUITY.md` first.
> Start with Phase A (T1–T7, hermetic Rust); before Phase B (T8) confirm the OTS binary + `/scratch/code/.venv`
> taxcalc toolchain. Caught btctax bugs → FOLLOWUPS + known-defect pin, not inline fixes. Fable for reviews.
