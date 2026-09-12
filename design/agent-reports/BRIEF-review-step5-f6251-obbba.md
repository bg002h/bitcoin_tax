# BRIEF — seam review of the `f6251/2025` build (`d8d023af`)

**Artifact:** commit `d8d023af`'s own diff — 11 files, +919/−110. **Not** the rest of the branch, not the
B3 work, not the year-package table's earlier steps. Scope is this diff because a build is the text
nobody has independently read.

**Inputs:** `design/agent-reports/REPORT-build-step5-f6251-obbba.md` (the builder's own account),
`BRIEF-build-step5-f6251-obbba-transcription.md` (`0b6bb46d`, the brief it was given — **and refuted**),
and `design/FORM_AUTHORITY_TABLE_DESIGN.md` §10.

You are ONE opus reviewer in an isolated worktree.

---

## 0. Stop-and-report, and why it is not a formality here

Six briefs in this arc have now been refuted by measurement. **The brief this build worked from was one of
them** — it claimed the 2025→2026 text delta was "exactly two cells"; it is eight numbered lines, and the
builder found a second cross-reference (line 7: 1040 line `7` → `7a`) that the coordinator had missed.
Two of the six refuted briefs were the coordinator's, both about *this form*.

So: **stop and report rather than build on any premise below you can disprove.** If a "settled" row in §2
is wrong, that is your most valuable finding.

## 2. Settled — machine-verified by the controller; do not re-spend budget

| settled | by whom |
|---|---|
| `make gate` **3589 passed / 12 skipped, exit 0** (+18 from 3571), `cargo fmt --all --check` clean | controller, on the main tree at this commit |
| line 7's cross-reference really differs (`line 7` → `line 7a`) | controller, verbatim from `f6251--2025.txt:65` and `f6251--2026-DRAFT.txt:103`; `grep -c "line 7a"` = 0 / 1 |
| the TY2025 Form 1040 prints only **7a**, no bare line 7 | controller, `f1040--2025.txt:81` — so the TY2025 6251 cites a line its own 1040 lacks; transcribed as printed, by the rule that the form is the authority |
| the `E0004` leg of the trap kill | **controller re-planted it**: a stub `LineSet::F6251_2026` pointed at `Schema::Form6251ObbbaMap` → `error[E0004]: non-exhaustive patterns: LineSet::F6251_2026 not covered` |
| the evasion leg | **controller re-planted it**: writing that arm `=> None` compiles and reds `every_obbba_revision_has_cells` with the message naming the inherited Schedule 1-A line number |
| the tree is free of plant residue | `diff <(git diff) <pre-plant patch>` empty before the commit |
| the Unwired pin is down to `["f1040s1a/2025"]` | controller, `line_set.rs:391` |

**Do not run the suite to reproduce those.** ★ A worktree suite result is not comparable to the main
tree's: on 2026-09-09 a verifier reported 7 failures in a worktree and all seven were environmental
(gitignored `design/forms/2026/` PDF fixtures are not shared across worktrees; the mandated
`CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review` trips a hook-lookup test). Never report a suite
red without showing the mechanism is in the code.

## 3. The seams — in priority order. Seam 1 is where the money is.

### ★★★ Seam 1 — the COMPUTATION-side twin, which the forms-side table cannot reach

The builder closed the trap on the **forms** side: a new revision cannot inherit another's
cross-references without a compile error. It then filed, as its own first follow-up, that
**`btctax_core`'s `Form6251Line1::Y2025` hardcodes "line 37" in its doc and its `1a/1b` shape serves
TY2026 too.** The forms crate's `_`-free match cannot see into core.

**So ask the question the build could not:** when TY2026 is computed, does the AMT chain read the *right*
line of Schedule 1-A, and the right 1040 line for Part III's capital-gain branch? Trace it:
`Form6251Line1::Y2025` → whoever constructs it → `return_1040.rs`'s AMT path → `Schedule1A`'s output.
Core's `Schedule1A` ends at `line38`, i.e. it is the **TY2025** revision, while TY2026's Schedule 1-A runs
to at least 43 — so a TY2026 computation that reads "line 37" is reading a line that means something else
on that year's schedule.

Two things to establish, separately:
- **Is there a live wrong-result path today?** TY2026 params are not bundled and `full_return_for(2026)`
  is `None`, so the honest answer may be "unreachable today, armed for the day it is bundled". Say which,
  with the evidence. An armed trap behind a closed gate is Important, not Critical — but only if you can
  show the gate actually blocks it.
- **Is the naming itself a defect?** A type named `Y2025` that serves TY2026 is the compression this
  project's transcription rule exists to forbid. Is it, or is it honestly a "the OBBBA-era shape" type
  with a misleading name?

### Seam 2 — are the THREE cells the right three, and are the other five really parameter-driven?

The report classifies the eight differing lines as three cells (1a, 4, 7) plus five sets of *printed
constants* (5, 18, 19, 25, 39) that it says live in `AmtParams`. **Check that claim per line, in the
code**, not in the table: for each of those five, is the figure actually read from params on the OBBBA
path, or is a 2025 figure transcribed into the new struct or its doc comments in a way TY2026 would
inherit? One literal here is a silent wrong figure on a signed return.

### Seam 3 — does anything READ this?

42 money cells and a filler landed. Is the OBBBA revision reachable from a surface that fills a form, and
can that be exercised — or is this a figure with no reader? The build states that `packet.rs` is
deliberately left unwired **and refuses**. Verify the refusal is real (a refusal that does not refuse is
Critical by this project's severity rules) and that the boundary is stated in source, not implied.

### Seam 4 — the instruments the build touched

It found `supported_years_cross_product.rs::map_resolves` green while recording a false `dispatch` gap,
and replaced it with a derived cross-check exhaustive over `Schema`. It also rewrote
`line_set_wiring.rs`'s `parses_into_2024_struct`. **For each: does it discriminate, and does it now
measure what its name says?** And the conformance partition — 60 labels = 42 mapped + 18 censused — is any
of those 18 a real line laundered as "censused with a reason"? That is the `blank-is-the-normal-case`
distinction: a censused line must encode no decision, not merely be unimplemented.

### Seam 5 — transcription faithfulness

Spot-check the 42 money cells against `design/forms/extract/f6251--2025.txt`: is any doc comment a
paraphrase rather than the official text, and is every *"enter the amount from line N"* cross-reference
right? The original AMT defect here was a single mis-transcribed cross-reference that inflated tentative
minimum tax by $200,000, and it was found by running vectors, not by reading.

## 4. Severity (this project's rules)

**Critical** — wrong result, data loss, an unmet guarantee, or a defect in what a tool *claims* to have
done (a gate that cannot fail, a refusal that does not refuse, a test reporting a false PASS).
**Important** — a real defect, a missing case, an unsound assumption. **Minor/Nit** — recorded only.
★ **Secret-handling defects never gate** — file them. A **blank** is the normal case; assert provenance,
never non-blankness. A hardcoded `0` on an unasked line fabricates sworn testimony.

## 5. Out of scope

The owner's TY2025 ruling itself; bundling any params; the TY2025/TY2026 fail-closed gates (both
deliberately untouched); the Schedule 1-A emitter (deliberately not built); `f1040s1a/2025` staying
`Unwired`; the B3 work; the rest of the branch.

## 6. Mechanics and output

Worktree only; `CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review` if you build. Do not commit, do
not push, edit no file but your report. **Do not spawn subagents.**

Write `design/agent-reports/2026-09-11-review-step5-f6251-obbba.md` as your **final action**, then return
only: the counts (`N Critical / N Important / N Minor / N Nit`), the **absolute path** of the report and of
your worktree root, and at most five lines of summary. Do not return the report inline.

Structure: **Verdict** · **Findings** (severity, `file:line`, mechanism, concrete failure scenario —
inputs/state → wrong output) · **Refuted premises** · **Seams traversed** (one paragraph each, naming what
you actually followed — this is how the round is judged) · **Out of scope / already filed**.
