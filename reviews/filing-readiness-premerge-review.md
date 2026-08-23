# PRE-MERGE review — `main..HEAD`, pointed at the seam `99628341` (harness B3)

_Read-only, dispatched per `reviews/BRIEF-filing-readiness-premerge.md`. The one question: does
anything on this branch interact badly across the seam that no earlier review could see?_

---

VERDICT: merge

Every finding below is Minor or Nit — doc drift, no behavior, no printed figure, no refusal, no
provenance semantics. Under the repo's own severity rules none holds the gate. The seam is
behaviorally clean; the drift is worth one burndown commit, before or after merge at the owner's
discretion.

## SCOPE

Read: the brief; the six prior filing-readiness reviews (`filing-readiness-{final,widening,fold,
phase1,phase2,phase4}-review.md`) to fix what each window held; the T1–T9 and (A)/(B) commit
messages; and the **current tree** (not the diff) for everything load-bearing:
`capital_loss_carryover.rs` (module doc + LINES), `return_1040.rs` (screen_absolute doc+body
2204–2827, `cwa_unvouched_carryover` / `apply_carryover_writeback` 2829–3060, AbsoluteReturn doc
1258–1272), `questions.rs` (`carryforward_in_present` + both T1 questions), `other_taxes.rs`
(`form_8960_lines`, the line-15 predicate and line-12 floor), `cmd/tax.rs` (the dual-report seam at
495–515), `input_form_store.rs` (`coherence_clear_or_refuse`, `commit`), `render.rs` later-window
diff, `admin.rs`/`answer.rs`/`main.rs`/`seam.rs`/`scrub.rs`/`scrub_axis.rs` later-window diffs,
`LIMITATIONS.md` end to end, FOLLOWUPS FR-16..FR-21, `SPEC_full_return.md` later-window diff.

Executed (read-only): `git log/diff/blame` across both sides of `99628341` for all 34 both-sides
files (per-file shortstats both sides); `git log -L` to date the stale module doc; targeted greps
for behavioral reliance on the lifted refusal (`nonpositive`, `TI≤0`, `G22`, refusal+carryover
co-mentions) over `crates/`, `design/`, `docs/`, man pages; caller/reader greps for
`cwa_unvouched_carryover` and the provenance fields. Honored: no oracle proposals on carryovers, no
re-litigation of (A)/(B), no re-filing of FR-17..FR-21, no re-running of the already-green CI
surface.

## SEAM FINDINGS

**S-1 (Minor). The lifted refusal (A) survives in PRESENT TENSE on six surfaces, and the
instrument built to prove "no trace" is structurally blind to all of them.**

- WHAT IS ASSERTED, per surface:
  - `crates/btctax-core/src/tax/capital_loss_carryover.rs:3-5` — *"`return_1040::screen_absolute`
    refuses a return whose taxable income is ≤ 0 with a capital-loss carryforward in, naming this
    worksheet as unmodeled."* (module doc, authored by N1 `6e83c540`, early window; `git log -L`
    shows no later touch).
  - `crates/btctax-core/src/tax/return_1040.rs:2209-2211` — `screen_absolute`'s own contract doc:
    *"Rows: (a) the §199A rows …; (b) taxable income ≤ 0 WITH a capital-loss carryforward-in (the
    G22 … edge)."* The body's row is deleted (the marker at 2805 says so); the doc contradicts the
    body it documents.
  - `crates/btctax-core/src/tax/return_1040.rs:1264-1265` — AbsoluteReturn's doc still enumerates
    *"(QBI-above-threshold, Form 6251 Who Must File condition 1, TI≤0-with-carryforward) are
    screened by `screen_absolute`"* — two of the three listed rows no longer exist in that function.
  - `crates/btctax-cli/src/cmd/tax.rs:505` — *"The absolute path adds `screen_absolute`
    (QBI-over-threshold / AMT / TI≤0-with-carryforward)"* — same double staleness.
  - `crates/btctax-core/src/tax/return_refuse.rs:7` — module doc lists *"taxable income ≤ 0 with a
    carryforward"* among the compute-dependent rows screened downstream.
  - `design/SPEC_full_return.md:400` — the §4.10 refusal table still mandates *"taxable income ≤ 0
    with a capital-loss carryforward | refuse"*. T2d amended LIMITATIONS + SPEC_input_surface +
    SPEC_input_form; T4 amended this same file's R3-M6 paragraph a few sections up — the refusal
    table was missed. The governing spec now contradicts the code, `LIMITATIONS.md:283`, and itself.
- WHAT IS ACTUALLY TRUE: the population FILES (K1/K15 pin it; owner-authorized widening A).
- WHY NO EARLIER WINDOW COULD SEE IT: the final review read these sentences while they were still
  *true*; the widening review verified the deletion and its checker. Only a pass holding both sides
  sees them as false. And K19 (`the_lifted_refusal_leaves_no_trace_in_the_tree`) greps for the
  **identifier** — deliberately, per `return_1040.rs:2813-2816`, the identifier was kept out of all
  comments so the grep stays meaningful — which is exactly what guarantees prose descriptions of the
  refusal's *behavior* escape it. The instrument is green and blind to this class by design.
- CONCRETE FAILING CASE: none behavioral, and I say so plainly — that is why this is Minor and not
  Important. I hunted for surviving reliance: the E0599 deletion enumerated every code consumer, the
  one fixture premised on the refusal was repointed (`report_exit_code.rs:163`), K1/K2/K15/K14 pin
  the filing/no-line-moves/advisory/M4 behavior, and CI is green on `37004b99`. The harm is to the
  next maintainer or reviewer who reads `screen_absolute`'s contract doc or SPEC §4.10 and reasons
  "TI≤0-with-carryover cannot reach X" — precisely the mis-reasoning the repo's load-bearing-refusal
  lesson warns about, one document away from becoming a defect.
- SMALLEST FIX: one sweep commit rewording the five doc sites to past tense ("refused, until the
  owner-authorized lift — see LIMITATIONS") without the identifier (keeping K19 green), and amending
  the SPEC §4.10 row to record the lift the way SPEC §(R3-M6) already records widening (B).

## OTHER FINDINGS

**O-1 (Minor). A doc splice left `apply_carryover_writeback` undocumented and pinned its contract
to the wrong function.** `crates/btctax-core/src/tax/return_1040.rs:2829-2844` vs `:2918`. The
finding-1 fold (`1c2eda7c`) inserted `a_single_gift_reaches_the_cwa_threshold` and
`cwa_unvouched_carryover` **between** `apply_carryover_writeback`'s pre-branch doc comment and the
function; T4 (`b7e9e640`) then edited the first lines of that stranded doc (adding "§1212(b)
capital-loss"), believing it still documented the write-back. `git blame` confirms the three-way
splice (b7e9e640 / d57990da pre-branch / 1c2eda7c). Today rustdoc shows `apply_carryover_writeback`
— the function that authors the `Computed` stamp — with **no doc at all**, while
`a_single_gift_reaches_the_cwa_threshold` (a bool predicate) opens with *"Returns the updated
`next_year` to persist, or `Err(message)` …"*, which is false of it; the stranded text's *"Both
conflicts are checked BEFORE either field is written"* also under-counts what are now four `!force`
guards plus three vouch gates. Both commits sit inside the widening review's window (so this is not
strictly a seam finding — recorded because it sits on the (B) chokepoint and each reviewer read each
edit as locally consistent). Fix: move lines 2829-2835 down to `:2918`, update "Both conflicts",
and give the predicate its own one-liner.

**O-2 (Nit). "This household is, by construction, low-income" overclaims.** `LIMITATIONS.md:287`
(and the T2 commit message). A TI≤0-with-carryover year at high AGI exists — e.g. a
catastrophic-medical year (medical over the 7.5% floor is uncapped) with large charitable — and it
was equally newly admitted by (A). Harmless in effect: for that shape the EIC/ACTC caveat the
sentence introduces is vacuous rather than wrong, its printed lines are pinned by K2 and the
current-year-twin equality of K1, and the 8960-on-MAGI predicate (`other_taxes.rs`) handles the
over-threshold variant correctly. Reword "by construction" to "typically" when convenient.

Everything else I attacked came back clean, specifically: the T1 refactor of the pre-existing Form
6251 line-2k liveness is semantics-identical (`amt_carryover_question_live` now delegates to
`carryforward_in_present`, same strict-positive test); `form_8960_lines`' widened `Some` has no
earlier-window reader that assumed "8960 present ⇒ NIIT > 0"; the TUI input-form commit path
preserves a later-window `Computed` carryover it cannot author (edit operates on the loaded row;
`coherence_clear_or_refuse` already names the write-back among its four writers, so stale-draft
shadowing and parked-draft clobbering are both closed); `cwa_unvouched_carryover` is one predicate
with three readers (report, export, write-back refusal) by construction; the later-window oracle
script edits belong to the P4-review folds the widening review verified; the post-fold-review
commits (`f33d45b5..HEAD`) are docs plus a dev-tooling clippy fix, nothing tax-bearing; and no
surface across the seam started printing a figure where silence was lawful (the 8960 now printed
for a zero-tax over-MAGI filer is the form's own Who-Must-File mandate, with line 12's `-0-` being
the form's own printed instruction).

## WHAT WOULD MAKE THIS REVIEW WRONG

If some consumer outside `crates/` — or in ephemeral orchestration no grep over files can see —
still gates behavior on the lifted refusal or on the pre-(B) meaning of `Computed`, then S-1 is not
doc drift but the shadow of a live defect, and the verdict flips.
