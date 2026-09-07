# Brief — BRAINSTORM: the interview that fills the return (Fable, lens 1 of 3)

You are a Fable-class design agent writing `design/BRAINSTORM_interview.md` for the Rust project
btctax at `/scratch/code/bitcoin_tax` (branch `main`; the controller names HEAD at dispatch).
Read-only with respect to the repo; do NOT commit; do NOT spawn subagents. Your deliverable is ONE
document, written as your final action.

## The owner's ask (verbatim)
*"Can we design an interview process to elicit income data, deduction related information, real
estate, dependence, etc., as well as exports from Bitcoin exchanges to allow the interview process
to fill out most of the tax return, if not all of it?"*

## Settled facts (read the recon first — it measured them: `design/agent-reports/2026-09-07-interview-recon.md`)
- btctax already has an input-form ENGINE (`crates/btctax-input-form`: a UI-agnostic `FormSpec` of
  sections/fields over `ReturnInputs`, a coverage KAT that polices every in-scope leaf, two question
  registries — `FORM_QUESTIONS`/`SKIPPABLE_QUESTIONS` — with answered-ness classes), a TUI that
  renders it (`btctax-tui-edit` "tax inputs"), a CLI twin (`income answer`), and a TOML wire
  (`income import`). Several information-return vectors (1099-INT/DIV/G/B rows) are import-only today.
- The exchange side is a LEDGER: four adapters (Coinbase, Gemini, River, Swan) import exports into an
  event ledger; the projection produces Form 8949 / Schedule D; the filer answers the Form 1099-DA
  question per (provider, cohort) (spec 1099-DA, rule R6 built 2026-09-06). The ledger asks its own
  questions (self-transfers, income classification) through `reconcile`.
- Standing rules the interview MUST obey (quote them from `CLAUDE.md` and the memory notes in the
  recon): a form line is TESTIMONY — never default an answer, a blank by omission is a defect while a
  blank by the inputs is the normal case; enumerate line sets FROM the form text, never a hand-list;
  transcribe instructions verbatim (the form's own "if X, skip to Y" is the branching logic); the
  FORMS derive the INTERVIEW, not the reverse; two oracles validate every figure; the year package is a
  data change per year (TY2026 is the first filed year; forms arrive Nov 2026 – Jan 2027).
- Owner decisions in force: S2 (the owner's real 2026 return defines "done" — its income types are
  NOT yet known to the project; the interview must be able to DISCOVER them), S6 (one review round per
  document, then build), one opus agent at a time for builds, the extension-by-default calendar.
- Out of scope unless you argue otherwise: e-file (closed by IRS rule; paper is the channel), state
  returns, non-BTC assets beyond what the 1099-B import already carries.

## The lenses (answer each; a lens with nothing to add says so in one line)
1. **The interview as a DERIVED artifact.** If every question comes from a form line and its
   instruction text, what is the generator? Sketch the pipeline: form line set (extracted text) →
   question graph (with the instructions' skip logic as edges) → the existing `FormSpec` sections and
   registries. Where does that break — lines whose instructions reference worksheets, other forms, or
   the filer's documents (W-2 box 12 codes, 1099 boxes)? What is the unit: a LINE, a DOCUMENT the
   filer holds, or a LIFE EVENT ("I sold my house")? Argue for one.
2. **Document-first vs question-first.** A real filer holds a pile of documents (W-2, 1099-INT/DIV/
   B/DA/R/G/NEC, 1098, 1095-A, K-1, property-tax bill, closing statement). Compare: (a) "type each
   document's boxes" (transcription, no judgment, matches the IRS's own design) vs (b) "answer
   questions" vs (c) a hybrid where the interview's first question is "which documents do you have?"
   and each document type is a transcription screen whose boxes map to lines. Which one obeys "an
   entry is testimony" best? Which one the two oracles can validate?
3. **The gap map** (from the recon §B/§G): which of the missing income/deduction/credit families does a
   TYPICAL W-2-plus-crypto filer need, ranked by 1040 lines unlocked; which are one struct + one
   section + one map line, and which need a new form (Schedule E, Form 8949 for securities, Form
   8812, education credits, 8889…). Give a cost/value ordering and say where "most of the return"
   stops for a v1.
4. **Dependents and real estate specifically** (the owner named them): what the form ACTUALLY asks
   (1040 dependents grid + the qualifying-child / qualifying-relative tests in the 1040 instructions
   and Pub. 501; Schedule A lines 5b/8a, Form 1098, the §121 home-sale exclusion and its Form 8949
   code H, Schedule E for rentals). What can be a question, what must be a document, what must be a
   refusal ("btctax does not do rentals — here is the exit").
5. **The exchange exports as interview input.** The exports already produce the 8949; what does the
   interview still have to ASK about them (1099-DA answers; venue naming; self-custody; gifts/
   donations; income classification; the standing order T7)? Can the ledger's `reconcile` questions and
   the return's questions become ONE interview, or should they stay two (argue from the answered-ness
   classes and from who owns the truth — the ledger vs the filer)?
6. **Provenance and re-interview.** Every answer carries who/when/from-which-document; a second year
   re-asks only what changed ("still the same employer?"). What does the year-N+1 interview look like,
   and what does the data model need for that (the owner's north star is EVERY year, not one year)?
7. **Validation.** How does the interview's output reach the two oracles (Tax-Calculator, OpenTaxSolver)
   — is the interview's record itself the oracle input? Where can a filer's answer be checked against a
   document (W-2 box 1 vs the wage total; 1099-DA box 1g vs column (e))?
8. **Journey walk (solo).** Walk the owner in February 2027: a shoebox of documents and four exchange
   exports. Step by step: what they have in hand, what the tool does, what ELSE they might do — and
   classify each divergence as refusal / warning / default / not our concern / documentation only.
   Every divergence that is worse than silence becomes a spec requirement.
9. **What to STOP / not build.** Name the seductive things that would not survive the answered-ness
   rule or the calendar (an AI that guesses, a "smart default", a schema-driven generic form builder
   that hides the form's own numbering).

## Output — your FINAL action
`design/BRAINSTORM_interview.md`: the lenses above as sections, each ending in a one-line
"so the spec should say…"; a ranked candidate scope for v1 (with the S2 caveat that the owner's
real return decides); the open questions for the owner (at most six, each with the answer that
changes the design); and a "what the recon measured" table you relied on (cite the recon). Match the
length to the substance — no filler, no boilerplate. Return ONLY a 3-line summary (the recommended
shape in one sentence, the v1 scope in one sentence, the file path).
