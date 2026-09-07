# Brief — the ONE review of `design/SPEC_interview.md` (Fable, lens 3 of 3; owner ruling S6)

You are an independent, adversarial Fable-class reviewer in your own worktree at the commit the
controller names. Read-only: no source edits, no commits, no subagents. This is the ONLY prose
review round the spec gets (owner ruling S6, `STANDARD_WORKFLOW.md` §2): the controller folds your
Criticals and Importants and then BUILDS from the text, one opus agent per task. So every finding
must carry the minimal spec wording or build task that resolves it, and you should spend your budget
where an executing build review cannot reach — the design's shape, the answered-ness invariant, the
journey, and whether a filer can be under-filed silently — not on compile-level detail.

## The one question
If `design/SPEC_interview.md` were built exactly as written, would a filer with a shoebox of
documents and four exchange exports be led, without a single defaulted answer, to a return that is
either CORRECT for what it covers or REFUSED with a named exit for what it does not — and would the
same interview a year later re-ask only what changed?

## Read, in this order
1. `CLAUDE.md` — the transcription rule, blank-is-normal, the answered-ness invariant, "an entry is
   testimony" (the standing rules; every finding cites the rule it enforces).
2. `design/agent-reports/2026-09-07-interview-recon.md` — the measured map (do not re-measure; cite).
3. `design/BRAINSTORM_interview.md` — the lenses and the recommended shape; where the spec departs
   from it, decide whether the departure is justified.
4. `design/SPEC_interview.md` — the artifact under review, in full.
5. The seams the spec names: `crates/btctax-input-form/src/seam.rs`, `spec/sections.rs`,
   `spec/registries.rs`, `spec/coverage.rs`; `crates/btctax-core/src/tax/{return_inputs,questions,
   classifier,return_refuse}.rs`; `crates/btctax-cli/src/cmd/{answer,tax}.rs`. Resolve every
   file:line the spec cites (a stale cite is a Nit; a cite that points at something that does not do
   what the spec says is an Important).

## Lenses (answer each)
- **Testimony.** Every new field: can it be `None`/unanswered structurally? Does any default, any
  "smart" inference, any document-derived value get written as the filer's answer without the filer
  affirming it? Which classifier class does each field join, and does the coverage KAT police it?
- **Derivation.** Is the question set derived from the form text (line set enumerated from the
  extract) or from a hand-list? What catches a line nobody asked about? Are the instructions' own
  skip rules the branching logic, quoted verbatim, with cites into `design/forms/extract/`?
- **Documents.** For each document screen (W-2, 1099-INT/DIV/B/R/G/NEC/DA, 1098, SSA-1099, K-1 if
  any): are the boxes transcribed from the form, does each box name the 1040/schedule line it
  reaches, and is a box the return does not use recorded as such rather than dropped?
- **Dependents.** Are the §152 qualifying-child / qualifying-relative tests asked as the 1040
  instructions phrase them, and do they gate line 19 (CTC/ODC), the filing-status HoH question and
  the dependent-care/EIC lines honestly (or refuse)? Can a dependent be listed without the tests?
- **Real estate.** Schedule A 5b/8a via Form 1098 and the property-tax bill; the §121 home-sale
  question and its Form 8949 consequence; rentals — a question, a document, or a refusal? Is any of
  it a silent omission?
- **The exchange side.** The 1099-DA answers, venue naming, T7, self-custody — does the spec keep the
  ledger's truth in `reconcile` and the filer's testimony in the interview, or does it blur them?
- **Year N+1.** Provenance per answer and the re-interview: is it a data-model requirement with a
  kill, or a sentence?
- **Oracles.** Does the interview's record reach the two oracles unchanged? Which answers are
  document-checkable and are those checks specified?
- **Journey.** Walk the owner in February 2027 through the spec's screens with the recon's gap list
  in hand; at each step, what ELSE might they do, and is the outcome worse than silence? Every such
  case is a finding with a class (refusal / warning / default / not our concern / doc only).
- **Buildability at the task level.** Is each T-task sized for one opus agent, ordered so the
  owner's own return (S2) is fillable earliest, with a kill that reds when its guarantee is removed?
  Compile-level detail is NOT your job — the build catches it.

## Severity (STANDARD_WORKFLOW.md)
Critical = a defaulted or inferred answer written as testimony; a silent under-filing; an unmet
guarantee; a gate that cannot fail. Important = a missing case, an unsound assumption, a task that
cannot be built as written. Minor / Nit non-blocking.

## Output — write the report as your FINAL action
`design/agent-reports/2026-09-07-spec-interview-review.md` in your worktree: header; per lens what
you checked and the verdict; findings as `### <ID> (<Severity>) — <claim>` with **Where** (spec
section / file:line), **What is wrong**, **Evidence**, **Minimal change** (spec wording or a task);
`Counts: C=<n> I=<n> M=<n> N=<n>`; "What I did not examine". Return ONLY a 3-line summary (counts,
the single most important finding, the report path).
