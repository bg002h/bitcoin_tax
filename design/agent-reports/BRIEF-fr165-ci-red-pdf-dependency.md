# BRIEF — FR-165: CI has been red for 8 days because 6 tests read a file the repo never commits

**Tier:** opus. **Isolation:** your own worktree. **One agent.** Do not spawn subagents.
**Severity: BLOCKING** (`STANDARD_WORKFLOW.md`: a red suite is itself a blocking finding).
**This is ahead of everything else.**

## 0. The one question

> `xtask::form_delta`'s 6 tests read field spellings from IRS PDFs that `.gitignore` deliberately never
> commits. How do those tests become **hermetic** — passing on a fresh checkout — **without** going quiet?

## 1. Already machine-verified — do NOT re-derive

Measured by the controller 2026-09-13; filed as FR-165 in `FOLLOWUPS.md` (commit `7583106e4`).

| fact | value |
|---|---|
| last green CI run | **`2bd04d458`, 2026-09-05 21:15** |
| CI successes since | **0 of 60 runs** |
| failing jobs | `test` on **ubuntu AND macos AND windows** — `209 passed; 6 failed` |
| the trigger commit | `edc212b41` (2026-09-05) *"tool(form-delta): make draft→final a DIFF, not a rebuild"* |
| root cause | `crates/xtask/src/form_delta.rs:64` — `let pdf = pdf_for(stem).ok_or_else(\|\| format!("no PDF found for {stem}"))?;` |
| the 6 tests + panic sites | `every_common_field_is_either_compared_or_named_as_unwitnessed` `:1378`; `no_archived_pair_reports_a_clean_verdict_from_zero_comparisons` `:1461`; `port_status_prints_the_committed_work_list` `:630`; `the_committed_work_list_matches_form_delta_at_head` `:568`; `the_work_list_checker_reds_on_every_planted_row` `:684`; `the_ty2026_drafts_are_ready_to_be_diffed_against_their_finals` `:1127` |
| PDFs | **125 on disk, 0 tracked** (`.gitignore:71` `design/forms/**/*.pdf`) |
| committed derived fixtures that DO exist | **126** extracts (`design/forms/extract/*.txt`), **70** geometry JSONs (`design/forms/geometry/*.json`) |
| CI's test command | `cargo test --workspace --locked`, **no fetch step** (`.github/workflows/ci.yml:36`) |
| why nobody noticed | `main` is **not** branch-protected — no required checks |

★ **`.gitignore:70` asserts something that is false for these six tests.** It says the committed text layer
*"IS what the tests read."* These read the **PDF's AcroForm** for field *spellings* — which the text layer
does not carry. Fixing the code without fixing that sentence leaves the next reader misled; a stale comment
of exactly this kind already misled an agent this week (FR-160).

## 2. The direction, and the three rejected alternatives

**Direction:** commit a **derived field-list fixture** per archived form — the AcroForm field names (plus
whatever `compute()` genuinely needs) — exactly symmetric to the extract and geometry fixtures that already
exist **for this same reason**. `compute()` then reads the committed fixture as its source of truth.

**Rejected, and say so if you disagree — with an argument, not a preference:**
1. **Skip when the PDF is absent.** This repo's rule is *skipping is not passing*. It would make
   `the_ty2026_drafts_are_ready_to_be_diffed_against_their_finals` — a test whose whole purpose is to prove
   the January pipeline is wired **today** — silently vacuous. **Worse than red.**
2. **Fetch from irs.gov in CI.** A network dependency on every run, against an archive whose purpose is
   reproducibility from committed bytes.
3. ★ **A fallback to the PDF when present.** This is the tempting one and it is the trap: a fallback means
   the fixture is exercised only where the PDF is *absent*, so on every developer machine the committed
   fixture is never the thing under test. **A fallback is how a fixture silently stops being checked.**

**The verification half — reuse what exists.** A4 already built `forms extract --all --check`, which
reports *"126 text layer(s) — 126 reproduce byte-for-byte, 0 rewritten"*. The field-list fixture wants the
same treatment: a command that regenerates it from the PDF **when a PDF is present** and asserts the
committed bytes are identical. That is where the PDF belongs in this design — in a checker that a developer
can run, never in the test path.

## 3. Constraints

- **Do not weaken any of the 6 tests.** They must still red on the defects they exist to catch. Two of them
  are themselves plant-based (`the_work_list_checker_reds_on_every_planted_row`); re-run their plants after
  your change and paste the output.
- **Do not commit PDFs.** `.gitignore:71` stays.
- **Fix the `.gitignore:68-71` comment** so it states what is actually true.
- **Out of scope:** `scripts/oracle/*` and `crates/btctax-core/tests/golden_returns.rs` (FR-164 just landed
  there — do not touch); the FR-136 borrowed-absence work in `map_pdf_conformance.rs`; `LONG_RANGE_PLAN`
  §7's forbidden list.
- Do **not** "fix" CI by changing what CI runs. The tests are right to exist; the dependency is the defect.

## 4. B1 — and here the kill is unusually concrete

**Your change is verified by a fresh checkout, not by an assertion.** Prove it the way CI would:

    git clone <this repo> /path/outside/the/tree   # or: git worktree add + delete every PDF
    cd <clone> && cargo test -p xtask --locked

Paste the summary line. **A clone with no PDFs must pass.** Then, separately, in a tree that HAS the PDFs,
show the new fixture-vs-PDF checker **reds on a planted fixture edit** and greens when reverted.

★ Two-directional, both required: hermetic without the PDF, and still checked against the PDF where one
exists. Either alone is the failure mode.

## 5. Stop-and-report

**Five briefs in this arc carried a premise the implementer refuted, two written by this controller** — the
most recent an hour ago, where the brief called `verify_f6251.py` the *best* of four surfaces and it turned
out to hold two hardcoded literals. If you can disprove anything above — that all 6 failures share one
cause, that the extract genuinely lacks field spellings, that 70 geometry fixtures cover the pairs the tests
need — **stop and report it** rather than building on it.

★ Specifically worth testing: whether the **geometry** JSONs already carry the field names, in which case the
new fixture may be unnecessary and the fix is smaller than this brief assumes. **Measure before building.**

## 6. Working rules

- Your own worktree. `CARGO_TARGET_DIR=<your-worktree>/target-fr165` — covered by `.gitignore:61`'s
  `target-*/`. Never a target dir in `/tmp` (32 GB tmpfs; one filled it and killed a running test).
- `make check` (~20s) for the local gate; `cargo test -p xtask` in the PDF-less clone for the real proof.
  Capture once to a file and grep it — never run a suite twice to collect counts and failures separately.
- ★ `make check` is nextest + clippy and does **NOT** include `cargo fmt --all --check`. The pre-commit hook
  blocked a commit on exactly that an hour ago. Run `cargo fmt --all` before you finish.
- **Do not commit, do not push.** Leave the worktree dirty; name every file you changed.

## 7. Deliverable

As your **final action**, write your report with a Bash heredoc (`cat > <path> <<'MARKER'` … `MARKER`) —
**not** the `Write` tool — to:

    design/agent-reports/REPORT-build-fr165-ci-red-pdf-dependency.md

Return only a short summary plus that path. The report states: the measured answer to §5's geometry
question; what changed; the PDF-less clone's summary line, pasted; the planted-fixture kill, red and green,
pasted; the re-run plants of the two plant-based tests; and any follow-up with a proposed owning phase.
