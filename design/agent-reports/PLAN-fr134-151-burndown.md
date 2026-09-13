# PLAN — burning down FR-134…FR-151, the port rehearsal's 18 findings

**Owner instruction 2026-09-12:** *"Work on all the follow-ups you just identified. Use up to 5 opus agents
and 5 sonnet agents."* This plan is the partition that makes parallelism safe, and it is the authority every
dispatched agent is pointed at.

## Why a partition and not just ten agents

The standing rule is one builder in the shared main tree, because the pre-commit gate runs `make check` over
the working tree and a second builder's uncommitted edits get swept (`git checkout` has eaten work here
before). The owner's instruction widens the agent count, not that rule. So:

1. **Every agent runs in its own `isolation: worktree`.** Nothing is built in the shared tree.
2. **Exclusive file ownership.** Each agent owns a disjoint set of files and touches nothing else. If a fix
   needs a file it does not own, it **stops and reports** rather than editing across the boundary — that is
   a finding about the partition, not a failure.
3. **Nobody touches `FOLLOWUPS.md`.** Eighteen agents marking their own entries closed is a guaranteed
   conflict, and on 2026-09-12 a batch shipped with its ledger entries unmarked because the brief never
   asked. **The coordinator marks every closure at integration**, with evidence.
4. **Two waves, not ten at once.** Peak concurrency is 5. The rehearsal recorded `make gate` OOM-killing the
   linker at 3 GB free; five concurrent full builds is the risk ceiling, ten is not worth finding out.

## Ownership table

| agent | tier | findings | OWNS (exclusive) |
|---|---|---|---|
| **A1** | opus | **FR-134**, FR-135 | `crates/xtask/src/line_coverage_check.rs`, `crates/btctax-core/src/tax/line_coverage.rs` |
| **A2** | opus | **FR-141**, FR-146 | `crates/btctax-forms/build.rs`, `crates/btctax-forms/src/line_set.rs`, `crates/btctax-forms/src/f6251_revision.rs` |
| **A3** | opus | FR-138, FR-150 | `crates/xtask/src/cite_check.rs`, `crates/xtask/src/archive_check.rs` |
| **A4** | opus | FR-139, FR-140, FR-143, FR-148 | `crates/xtask/src/main.rs`, new `crates/xtask/src/forms_*.rs`, `crates/xtask/src/form_geometry.rs`, `crates/xtask/src/label_reader.rs`, `design/forms/extract/*` headers |
| **A5** | opus | **FR-136** | `crates/btctax-forms/tests/year_record.rs`, `crates/xtask/src/form_delta.rs` |
| **S1** | sonnet | FR-142 | `crates/btctax-forms/tests/f8995a_fill.rs` |
| **S2** | sonnet | FR-144 | `scripts/archive_drafts.py` |
| **S3** | sonnet | FR-147 | `crates/xtask/src/harness_check.rs` |
| **S4** | sonnet | FR-137, FR-145, FR-149, FR-151 | `design/TY2026_PORT_REPORT.md`, `design/FORM_AUTHORITY_TABLE_DESIGN.md` |
| **S5** | sonnet | — | the final re-verification over the **integrated** tree |

**Bold** = the report's own three-before-January (§7). A1, A2 and A5 carry them.

Wave 1 = A1–A5. Wave 2 = S1–S4. Wave 3 = S5 over integrated `main`.

## Rules binding every agent

- **Own worktree.** `CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-<agent>` — a **unique** path, never
  `/tmp` (a 32 GB tmpfs shared with builds; a scratch `target/` filled it and killed a running test once).
- **Do not commit, push, `git stash`, `git checkout` or revert.** Revert a mutation with a **cp backup**.
- **No subagents.**
- **Every finding gets a B1 kill seen RED on a planted defect**, with the red pasted into the report. A
  finding whose fix has no kill is not closed — it is a fix with a promise attached.
- **Never hand-count what a tool can count**, and never quote a number or a list from a `head`/`tail` view.
  Nine coordinator errors in this session were that shape; the ledger records them.
- ★ A worktree suite result is **not** comparable to the main tree's — gitignored `design/forms/2026/` PDFs
  are not shared across worktrees and the mandated `CARGO_TARGET_DIR` trips a hook-lookup test (FR-147 is
  exactly that). Never report a suite red without showing the mechanism is in the code.
- **Stop and report rather than build on a premise you can disprove.** Twelve briefs in this arc have been
  refuted by measurement, six of them the coordinator's — and the rehearsal that produced these findings
  refuted five runbook claims. Each entry below is a *proposal*, not a fact.
- **Report via a Bash heredoc** (`cat > <path> <<'MARKER'` … `MARKER`), **not** the `Write` tool, which
  subagent harnesses refuse for report files (FR-129). Path: `design/agent-reports/REPORT-<agent>-<frs>.md`.

## Integration, by the coordinator

One worktree at a time: copy the diff in, run `make gate` + `cargo fmt --all --check`, re-plant that agent's
headline kill independently, mark its `FOLLOWUPS.md` entries closed with evidence, commit, push. **The first
gate failure names the conflicting pair** — which is why ownership is exclusive and integration is serial.

Scope for the round: **no product behaviour changes.** Every one of the 18 is machinery, an instrument, a
script, or a document. If a fix would change what a filer sees or what a form prints, **stop and report** —
that is a different round with a different gate.
