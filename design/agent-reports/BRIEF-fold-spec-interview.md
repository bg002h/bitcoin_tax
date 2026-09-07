# Brief — fold the interview spec's one review into `design/SPEC_interview.md` (r2)

Single editor, shared main tree `/scratch/code/bitcoin_tax`, branch `main` (HEAD named at dispatch).
Prose only: you edit ONE file, `design/SPEC_interview.md`, and write your report; no code, no other
design files, no commits, no subagents. Owner ruling S6 is in force: this fold is the LAST prose step
— no second review follows; the spec is then built task by task and the builds' seam reviews are the
gate. So every fold must be COMPLETE (the finding's failure scenario cannot recur under the new text)
and BUILDABLE (an implementer can act on the sentence without asking you).

## Inputs
1. `design/agent-reports/2026-09-07-spec-interview-review.md` — 28 findings (1C/14I/9M/4N), each with
   **Where / What is wrong / Evidence / Minimal change**. Fold ALL of them: C1 and I1–I14 mandatory;
   M1–M9 and N1–N4 unless a Minor conflicts with a mandatory fold (then say so).
2. `design/agent-reports/2026-09-07-spec-interview-review-VERIFICATION.md` — the controller's ledger
   (every checkable claim HOLDS; do not re-verify).
3. `design/SPEC_interview.md` — the artifact (869 lines, §1–§10). Keep its structure and section
   numbering; edit in place; the review's minimal change is the default wording — deviate only with
   a reason recorded in your report.
4. For context only: `design/BRAINSTORM_interview.md`, `design/agent-reports/2026-09-07-interview-recon.md`,
   `CLAUDE.md` (the rules every fold must keep: testimony, blank-is-normal, enumerate from the form,
   transcribe).

## Specific instructions per blocking finding
- **C1**: add the DIRECTION RULE to R2.2 — a line whose part UNDERSTATES tax when blank (income and
  additional-tax lines, derived from the form part: Form 1040 lines 1–9, Schedule 1 Part I, Schedule 2)
  may be `covered_by` only a `QuestionId` or a `RefuseReason` (never an `Advisory`, never the residual
  attestation); deductions/credits/payments lines may be covered by an `Advisory` (a forgo). Add the KAT
  the review names: the covering question's prompt text NAMES the line (or its form), so a generic
  "anything else" cannot cover a specific income line. Say where the direction table lives (derived
  from the form's part headings, cited into `design/forms/extract/`).
- **I1**: give document-less income a door — the census `No` for a document type does NOT close the
  income the instructions say to report without one (interest without a 1099-INT, state refund
  without a 1099-G, …): each such line gets a `FORM_QUESTIONS` gate phrased from the instructions.
- **I2**: per-document box censuses enumerated from the T2 extracts (like the form-line census), with
  the kill that a box the return reads is in the screen or recorded unused with a reason.
- **I3**: Form 1098 box 4 → Schedule 1 line 8z (or the refusal), never collected-and-dropped.
- **I4/I5**: the age test with a missing DOB is unevaluable → the row REFUSES (or the dependent is
  not printed with a named reason); restore the newborn exception verbatim from the instructions.
- **I6**: the Digital-Assets question's predicate exactly as the 1040 instructions phrase it (a
  buy-only year is NOT a Yes-forcing event; receipt as payment/reward/mining, sale, exchange, gift…
  are), with both refuse cells corrected.
- **I7**: HoH "considered unmarried" as the instructions' separate predicates, each a gate.
- **I8**: the mortgage declarations keyed to Schedule A's liveness (a standard-deduction filer is
  never asked them).
- **I9**: a rule for a `prompt_hash` mismatch (the answer is re-asked; the old answer is kept as
  history, never as the current answer) with its kill.
- **I10**: the diligence record keyed by a stable dependent identity (SSN or a row id), not the index.
- **I11**: the year-N+1 opener becomes a TASK in §7 (T-number, kill, once/per-year).
- **I12**: the oracle projection carries dependents, ages and blindness; the line-19 excuse compares
  real values.
- **I13**: T8's kill re-pointed at a map that has the dependents-grid cells (TY2024's) or at the
  emitter's field set on TY2025+.
- **I14**: §9 Q1 lists EVERY refusal family of §2.2, so the owner's answer decides each row.
Minors/Nits: fold each as its minimal change says (the panel's "answered, refusing" state; the
Declined skippable staying on the forgoing list; the per-row gate liveness seam; the two exit
sentences; the §163(h)(3) aggregate ceiling; T2's kill vs R5; the params-less prompt; the two
provenance holes; the two journey moments; the counts and cites).

## Report — your FINAL action
`design/agent-reports/2026-09-07-spec-interview-fold.md`: per finding the section edited and the
sentence(s) now in the spec (quote them), deviations with reasons, and a final "what the build must
now prove" list (each blocking fold → the kill an implementer will write). Return ONLY a 3-line
summary (findings folded / 28, any not folded and why, the report path).
