# BRIEF — B3, the whole-branch review of the interview arc

**Written 2026-09-11 by the coordinator, at dispatch, from `PLAN-b3-whole-branch-review.md` + its
ADDENDUM.** The plan is the authority on *why*; this brief is the authority on *what you do*. Read the
plan too (`design/agent-reports/PLAN-b3-whole-branch-review.md`, 184 lines) — especially §3 (the four
seams), §6 (what makes this round a failure) and the ADDENDUM (A1–A5).

You are ONE opus reviewer in an isolated worktree. There is no fan-out. Nobody else is reviewing.

---

## 0. The instruction that outranks everything else in this brief

**Stop and report rather than build on any premise in this brief you can disprove.**

Four controller/implementer briefs in this arc have been **refuted by measurement**, and two of the four
were written by the coordinator that wrote this one:

| premise | author | refuted by |
|---|---|---|
| widening R15 would red on the IRS's `"Covered lots"` vocabulary | FR-114's author | whole-word matching — `"lots" != "lot"` |
| entering a 1099-B reopens the Form 8949 box choice | FR-111's author | `form_8949()` reads only `state.disposals`; a `[[b_1099]]` row never becomes an 8949 row |
| the export's I5 advisory may misdescribe a routed return | **the coordinator** | it is `regime`-gated; the two branches are mutually exclusive |
| *"zero labels red today"* | **the coordinator** | 209 of 279 labels are macro-generated and were invisible to the scan that produced it |

> **A hand-written scan over one syntactic form is not a measurement of a set produced by another.**

Every one of those four was written by someone being careful. What kept all four out of the product was
that the recipient **measured the premise and reported the refutation instead of proceeding**. A refuted
premise in this brief is a first-class finding — report it as one, with the measurement. That applies to
the facts in §2, the seams in §3, the "already covered" table in §4, and this section.

## 1. Scope, and the machine-checked facts you do not need to re-derive

- **Range: `121c8805..HEAD`**, `HEAD == origin/main == f9903f31`.
- Measured by the coordinator at dispatch (`git diff --stat`, `git log --oneline | wc -l`):
  **51 commits · 142 files · +32,236/−982**, of which `crates/` is **78 files / +17,657** and `design/`
  is **48 files / +12,361**.
- `RefuseReason` has **127** variants (`crates/btctax-core/src/tax/return_refuse.rs:75`, counted at
  dispatch — the plan's "~126" is one revision stale).
- At `3ae930ab` (one commit back; `f9903f31` is markdown only): `make gate` **3561 passed / 12 skipped**,
  `cargo fmt --all --check` clean, five instruments stable — `line-coverage` 375/18/31/0/17,
  `census-join` 274 across 13 maps, `r15` stop-list 8+4+6 renderer sources / 91 prompts, `prompt-check`
  90, `box-census` 268/19/9.

**You do not need to run the suite.** If you do run anything, `CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review`
is mandatory. ★ And know this before you report a red: a worktree's suite result is **not** comparable to
the main tree's. On 2026-09-09 a verifier reported 7 failures in a worktree and **all seven were
environmental** — the gitignored `design/forms/2026/` PDF fixtures are not shared across worktrees, and
the mandated `CARGO_TARGET_DIR` trips a hook-lookup test. A suite red is not a finding unless you can
show the mechanism is in the code, not in the worktree.

## 2. Where the ice is thinnest — start here

Everything before `4fa1e723` has had a per-task opus seam review **and** a sonnet re-verification. These
nine commits have had **one** sonnet re-verification scoped to two folds (`fd6adda8`, 0C/0I) and **no
independent design review at all**:

```
9961519f decision(owner): FR-110 ceiling; FR-114 label-only — FR-114's premise refuted
1e6095d7 brief(FR-114)
5a1a94be persist(FR-111 recon)      52b348c2 fold(FR-111)
b9b3810f persist(FR-114 report)     52558813 fold(FR-114)
fd6adda8 persist(re-verification)   3ae930ab ledger(FR-117)
f9903f31 plan(B3) + CONTINUITY      (markdown only)
```

Two of them change surfaces that span the workspace:

- **`52558813` added `Field.label_source`** — a field on a public struct, no `Default`, no `_` arm, so
  every `Field` literal in the workspace had to change. The re-verification confirmed they all compile
  and spot-checked 15+ classifications. It did **not** ask the seam question: **does anything else read
  `Field` in a way this widens, breaks, or silently reclassifies** — the TUI, the renderers, the census
  join, the label reader?
- **`52b348c2` edited `LIMITATIONS.md`, which is `include_str!`'d into the binary** (`main.rs:582`). That
  is **shipped filer-facing text**, not a design doc. Review it as product surface — is what it now says
  true of the code at this tip, for the years the code supports?

## 3. The one question, and the seams

> **Do the twelve tasks compose into something a person can actually file with — and where do two
> correct pieces meet and produce a wrong whole?**

Five seams. Each spans tasks; no single earlier review held both sides of any of them.

1. **The chain from answer to printed page.** keystroke → `answer_log` → `ReturnInputs` → the screens →
   `Form1040Lines` → the emitter → the read-back verifier. Every hop was reviewed; the *chain* was not.
   FR-102 (a filer with an HSA could not print a single page — a shipped Critical, found by a filer
   walking the product, not by any of twelve 0C/0I reviews) lived on the last hop. T11's root fix found
   four money defects on an earlier one.
2. **Multiple writers of one structure.** `income import`, `income answer`, the TUI seam and
   `open_next_year::seed` all write `ReturnInputs`. FR-97 was one such disagreement (fixed); FR-109 was a
   second (closed by the ask-once rule). **Enumerate every writer** — derive the list, do not type it —
   and for each ask what it records, what it skips, and whether the readers agree.
3. **Refusals as a system, not as rules.** 127 `RefuseReason` variants across twelve tasks. Can a filer
   reach a state where two refusals contradict, or where the exit named by one is blocked by another? Is
   any refusal unreachable (FR-87's shape)? Is any reachable state **unrefused**?
4. **The year boundary.** TY2024 files; TY2025/26 are `preparing`. Every task added year-conditional
   behaviour. The `stamp_year` defect was a year-scoped screen blind on the one command that creates a
   row — **that is one instance of a class, and the class was never swept.**
5. **Instruments vs. the surfaces they claim to cover.** Five instruments plus the R15 extension. FR-114
   proved an instrument can be green because it never ran over the region that mattered; FR-117 shows the
   exemption's unit (a field) differing from the thing exempted (a span of text). **For each instrument:
   what does it actually traverse, and what does it claim?**

Seams 1–4 are required. Seam 5 if budget allows.

## 4. Already covered — do not re-spend budget here

A finding a per-task review could have caught is **out of scope by definition**: if it were catchable
there, it was caught there.

| already done | evidence |
|---|---|
| Per-task correctness, T1–T12 + T16 | 14 seam reviews + 14 sonnet re-verifications, all closed 0C/0I |
| Every kill replanted after each fold | each re-verification report |
| Two end-to-end filer journeys | `2026-09-07-journey-walk-*` (walk 2: single filer, no dependents, 8949-heavy) |
| A cross-task pattern sweep | `2026-09-09-cross-task-pattern-sweep.md` (sonnet) |
| Suite, instruments, docs | `make gate` 3561 green; five instruments stable; `make docs` no diff |
| The Form 8949 box truth table, re-derived from source | `RECON-fr111-8949-box-truth.md` (`5a1a94be`) |
| Every filer-facing 1099-B / 1099-DA / box claim swept | same recon, Q3 |
| The R15 label extension, kill-tested and independently planted | `REPORT-build-fr114-label-scan.md` (`b9b3810f`) |
| Both folds re-verified | `REVERIFY-fr111-fr114.md` (`fd6adda8`) — 0C/0I, 1 Minor (FR-117) |

★ **FR-110 is a DECISION, not a gap.** "btctax cannot file a 1099-B with adjustments" is the **intended
permanent ceiling**, owner-ruled at `9961519f`. A finding to that effect is out of scope. (If you find
that the *product does not say so where a filer would look*, that is in scope — the ceiling is settled,
its disclosure is not.)

Open follow-ups are in `FOLLOWUPS.md`; FR-112, FR-113, FR-115, FR-116, FR-117 are **filed with owning
phases**. Re-reporting a filed follow-up is not a finding — cite the FR number instead.

## 5. Severity (this project's rules, not generic ones)

- **Critical** — wrong result, data loss, an unmet guarantee, a defect in what a tool *claims* to have
  done (a gate that cannot fail, a refusal that does not refuse, a test reporting a false PASS).
- **Important** — a real defect, a missing case, an unsound assumption.
- **Minor / Nit** — recorded, does not gate.
- ★ **Secret-handling defects are NEVER Critical and NEVER Important** (owner ruling 2026-08-27). Keep
  finding them, report them as follow-ups with a reproduction, and do not gate on them. Do not relabel
  one as correctness to get it back above the line.
- A **blank** line is the normal, correct case on a tax return. Never report "this line is empty" as a
  defect. The invariant is **provenance**: collected / computed from named lines / a constant the form
  prints / a refusal. "Blank because the inputs say so" is correct; "blank because nothing ever
  populated it" is the bug, and they look identical on the page.
- **An entry is sworn testimony from the filer against the filer.** A hardcoded `0` on a question nobody
  asked **fabricates testimony** — that is Critical, not cosmetic.

## 6. What makes this round a failure

From the plan, §6: *"the measure is not that a final pass happened, but that it was scoped to something
the earlier passes could not have seen."*

**A report full of per-commit correctness findings means this brief failed, not that the branch is
clean.** 0C/0I is a fine outcome **if and only if** your report's "seams traversed" section shows you
actually walked the five chains — naming the files and functions at each hop, not asserting coverage.

## 7. Output — your final action, non-negotiable

Write your report to **`design/agent-reports/2026-09-11-b3-whole-branch-review.md`** inside your
worktree, as your **final action**, then return **only**:

1. the counts — `N Critical / N Important / N Minor / N Nit`;
2. the **absolute path** of the report and of your worktree root (the coordinator copies the file out);
3. at most five lines of summary.

Do not return the report inline; it costs the coordinator's context twice. Do not commit anything, do
not push, do not edit any file other than your report. You are a reviewer, not an implementer — **propose
no patches**; state the defect, its mechanism, and the file:line where it lives.

Report structure:

- **Verdict** — counts.
- **Findings** — one block each: severity, title, `file:line`, the mechanism, the concrete failure
  scenario (inputs/state → wrong output), and why no per-task review could have seen it.
- **Refuted premises** — anything in this brief you measured and disproved, with the measurement. Empty
  is a legitimate answer; a premise you *doubt but did not measure* goes here too, labelled as such.
- **Seams traversed** — one paragraph per seam, naming the hops you actually followed and what you
  checked at each. This section is how the round is judged.
- **Out of scope / already filed** — anything you saw that belongs to §4 or an existing FR.
