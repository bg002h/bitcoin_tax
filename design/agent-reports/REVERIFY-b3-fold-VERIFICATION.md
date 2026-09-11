# Controller ledger — machine-checking the B3 fold's re-verification

**Report under check:** `design/agent-reports/REVERIFY-b3-fold.md` (persisted verbatim at `5c86ce48`,
138 lines, **0C/0I/0M/0N**). A zero-finding report is the one that most needs checking: it is
indistinguishable, from the outside, from a pass that never looked.

The controller re-derived the report's **load-bearing** claim — hunt #1, the completeness of the I-1 fix —
from the tree rather than accepting it, because a second surface drawing the static label while `apply`
hashes the rendered one would be I-1 surviving with the round closed on top of it.

## Hunt #1 re-derived independently — the claim HOLDS

A `#[cfg(test)]`-stripped scan of every `.rs` in `crates/` finds **41** production occurrences of `.label`.
All 41 accounted for:

| group | count | verdict |
|---|---|---|
| `draw_edit.rs:2946-2955` — the fix, via `field_label` | 1 | the only production site drawing a `Field`'s registry label |
| `registries.rs:686-733` — `field_label`'s own body and doc comment | 6 | the resolver itself |
| `xtask/{box_census,label_reader,line_coverage_check,dependents_grid}.rs` | 24 | `b.label` = a **box** label from a form's extracted text, not a `Field` |
| `draw_edit.rs:2063` — `filing_status_field().label` | 1 | cleared, see below |
| `draw_edit.rs:2392` (`PendingRemove`), `:6434` (`modal.action.label()`), `main.rs` (`FileReport`) | 3 | unrelated structs |
| wallet / provenance labels (`render.rs`, `step0.rs`, `chokepoint`, `ingest.rs`, `resolve.rs`, `return_1040.rs`) | 6 | unrelated |

**The one that mattered, checked rather than reasoned about.** `draw_edit.rs:2063` draws
`filing_status_field().label` directly, and `QuestionId::FilingStatusConfirmed` **is** one of the five
rendered prompts — so if that field were a declaration leaf, the round would be closing over a live second
instance of I-1. It is not:

- `filing_status_field()` (`edit/form.rs:843-849`) finds `FieldId::FilingStatus`, whose label is the
  literal **`"Filing status"`** (`spec/sections.rs:168-170`) — a selector caption, not a question.
- `QuestionId::FilingStatusConfirmed` maps to a **different** field:
  `FieldId::DeclFilingStatusConfirmed => QuestionId::FilingStatusConfirmed` (`registries.rs:781`), declared
  by `decl_tristate!(35, FieldId::DeclFilingStatusConfirmed, …)` (`registries.rs:289`).
- `field_to_question` is an explicit `Some(match …)` over `Decl*`/`Sa*`/census leaves; `FieldId::FilingStatus`
  appears nowhere in it, so `field_label` would return the identical static for it anyway.

The report's clearing is correct, and for the reason it gave.

## Also spot-checked

- **`field_label` has exactly one production caller** (`draw_edit.rs:2955`); the other three hits are a doc
  comment and two test names. Confirmed by grep across `crates/`.
- **`ExcessSsSingleEmployerNotCreditable`** — zero occurrences workspace-wide, so N-1 is fully closed.
- **The fold's deletions** — all 51 removed lines were read individually by the controller before the fold
  was committed: the `LIMITATIONS.md` sentence, the CLI `stamp_year` body (moved to core), the fixture's
  hardcoded `tax_year: TO`, the old `apply` signature and call site, the FR-97 hand-stamp, `f.label`, and
  the `NotInForm` count-pin expression (replaced, with the new anchor justified). **No expected tax figure
  and no golden changed** — which is what rules out the "regenerate the golden to match the breakage"
  failure this repo has recorded.
- **Not re-derived**, and recorded as such: the report's hunts #2–#7 (kill non-vacuity beyond the two the
  controller re-planted itself, the nine restated fixtures, the new refusal's reachability, `form.year`'s
  provenance, the four corrected doc comments, the FOLLOWUPS entries). The controller independently
  re-planted the two load-bearing kills before the fold was committed (see `db056c57`'s message) and read
  all nine fixture diffs as part of the deletion sweep.

## Verdict

**The re-verification stands at 0C/0I/0M/0N, and the B3 round closes.** Its "checked clean" section names
what it looked at per hunt rather than asserting coverage, it ran the ten new tests itself rather than
quoting the fold's transcript, and its one substantive piece of work — clearing the second label site —
reproduces under independent derivation.
