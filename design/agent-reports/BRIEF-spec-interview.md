# Brief — SPEC: the interview that fills the return (Fable, lens 2 of 3)

You are a Fable-class design agent writing `design/SPEC_interview.md` for btctax at
`/scratch/code/bitcoin_tax` (branch `main`; HEAD named at dispatch). Read-only with respect to the
repo; do NOT commit; do NOT spawn subagents. ONE document, written as your final action. This spec
gets ONE independent review round (owner ruling S6) and is then BUILT by one opus agent at a time
under briefs written from it — so every requirement must be buildable as written and every guarantee
must name its kill.

## Inputs (read in this order)
1. `design/agent-reports/2026-09-07-interview-recon.md` — the measured map of what exists (surfaces,
   line-level coverage, dependents, real estate, the exchange side, the rules, the gap ranking).
2. `design/BRAINSTORM_interview.md` — the lenses, the recommended shape, the v1 scope, the owner
   questions. Where the brainstorm hedges, DECIDE and say why; where it recommends, adopt unless you
   can show a defect.
3. The existing specs the interview extends, not replaces: `design/SPEC_input_surface.md` (the
   answered-ness classes, the three refusal classes) and `design/SPEC_input_form.md` (the `FormSpec`
   seam, sections, coverage KAT, the draft table, the TUI model). Name every seam you reuse by its
   real name (`crates/btctax-input-form/src/seam.rs`, `spec/sections.rs`, `spec/registries.rs`,
   `spec/coverage.rs`; `crates/btctax-core/src/tax/return_inputs.rs`, `questions.rs`, `classifier.rs`,
   `return_refuse.rs`; `crates/btctax-cli/src/cmd/answer.rs`, `tax.rs::import_return_inputs`;
   `crates/btctax-tui-edit/src/edit/*`).
4. `CLAUDE.md` (the transcription rule with its scope amendment; blank-is-normal; the answered-ness
   invariant), `STANDARD_WORKFLOW.md` §2 (S6), `design/ROADMAP_STATUS.md` §0a (S2, the calendar).

## The spec must contain (in this shape — the repo's specs use it)
- **Why this exists** (the owner's words; the gap counts from the recon).
- **Scope / non-scope**: the v1 the brainstorm ranked, with each excluded family named and given its
  REFUSAL sentence (a filer who has it must be told, never silently under-filed). Say plainly where
  "most of the return" ends and what the exit is.
- **The rule(s)** R1…Rn, each one paragraph with the mechanism and the kill: the unit of the interview
  (per the brainstorm's decision), how questions derive from the form text (the generator or the
  discipline that stands in for it, with the kill that catches a line nobody asked about), the
  document screens (which documents, which boxes, which lines each box reaches — transcribed from the
  form/instruction text with cites into `design/forms/extract/*.txt`), the dependent tests (the
  qualifying-child / qualifying-relative questions as the instructions phrase them, and which 1040
  lines and credits they gate), real estate (Schedule A 1098/property tax; the §121 question and its
  Form 8949 code-H consequence, or the refusal), the exchange side (what stays in `reconcile`, what
  the interview asks about the ledger: the 1099-DA answers, venue naming, T7), provenance per answer
  (who/when/which document) and the year-N+1 re-interview, the oracle path (how the interview's
  record becomes the two-oracle input; which answers are document-checkable), and the answered-ness
  guarantee (no default, structural — say which classifier class each new field joins).
- **Surfaces**: the TUI flow (screen order derived from the document list), the CLI twin (`income
  interview`? or `answer` extended — decide), the TOML wire (every new struct on `ReturnInputs` with
  `#[serde(default)]` discipline stated per field), `report`'s rendering of the new lines.
- **Data model**: the new `ReturnInputs` structs/fields, one per document or life event, with the
  form line each reaches and the provenance fields; what is a `Vec` (repeating documents) and what is
  a singleton; what the coverage KAT's fixture must carry for each.
- **The journey**: the brainstorm's walk restated as requirements, each divergence with its class.
- **Build plan**: tasks T1…Tn, each with its kill, ordered so the owner's own return (S2) is fillable
  earliest; each task sized for one opus agent; say which are per-year (data) and which are once.
- **Kills**: a consolidated list — every guarantee with the test that reds when it is removed; the
  coverage KAT extended so a new struct without a section bites.
- **Open questions for the owner**: at most six, each with the answer that changes the design.
- **What this spec does NOT change**: the crypto engine, the 1099-DA rules, the packet, the oracles.

Cite every claim about the tree with a file:line that exists at HEAD (the controller's ledger will
resolve each). Match length to substance. Return ONLY a 3-line summary (the unit and the v1 scope,
the task count, the file path).
