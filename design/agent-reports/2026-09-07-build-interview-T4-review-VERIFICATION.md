# Verification ledger — interview T4 seam review (`2026-09-07-build-interview-T4-review.md`, 1C/0I/3M/1N)

Controller machine-checks from the main tree at `62d7828f`, before any fold.

| # | claim | check run | measured | decision |
|---|---|---|---|---|
| C-1 | `draft_holdings` is four hand-listed categories; `broker_reporting`, `schedule_c` and the rest are absent | `awk` over the fn | `answers`, `documents` (derived from `DocumentRow::ALL`), `dependents`, `schedule_a` — nothing else | true |
| C-1 | the file already carries the wider non-trivial test for the NOTE only | `grep 'ReturnInputs::default()'` | `input_form_store.rs:443` `if d.ri != ReturnInputs::default()` — decides whether to print a note before deleting | true; the refusal's predicate is strictly narrower than the warning's |
| C-1 | `broker_reporting` is a `ReturnInputs` field | grep | `return_inputs.rs:1029` | true — the params-less year's primary authoring content is invisible to the guard |
| C-1 | the reviewer's plant (a TY2026 draft with two providers' answers + a Schedule C is destroyed by an unconfirmed `income import`, `income clear`, and `load`'s stale discard) | the review quotes the plant's output; the mechanism read above is decisive | `is_empty=true`, `draft survived = false` on all three | **FOLD**, structural: the disposable predicate compares the draft against the fresh seed, never a category list |
| M-1 | the discard-only screen's chrome is hard-coded "parked" | grep `draw_edit.rs` | `:2230` *"Stale parked draft for"*, `:2252` *"discard the parked draft"*, `:2256` *" — stale parked draft "* | fold inline |
| M-2 | `SalesTaxElectionWithoutAmount`'s detail names `income answer` as an exit that a refused import cannot reach | grep | `return_refuse.rs:1592` | fold inline (drop the clause) |
| M-3 | `working_return` / `broker_answers`' new `Err` propagates with `?` into read-only commands | grep | `cmd/tax.rs:575,646`, `cmd/admin.rs:212,755` | fold: the read-only projections map `StaleDraftHoldsInterview` to `(None, note)`; write paths keep the hard refusal |
| N-1 | two `CliError` literals carry collapsed runs of spaces | grep | `lib.rs:187,198`; 10 lines with ≥10-space runs in the file | fold inline |

## Disposition
- **C-1 — FOLD.** `draft_is_disposable(ri)` = the draft equals the year's fresh seed (`ReturnInputs
  { tax_year, filing_status: ri.filing_status, ..Default::default() }` — the filing status alone is
  not work, per the existing `a_disposable_draft_is_still_superseded_without_a_flag` fixture) —
  structural, no category list; `DraftHoldings::describe()` stays for the message with a fallback
  clause (*"and work not otherwise itemised"*) when the difference lies outside the counted fields.
  Kill: the reviewer's three plants (broker answers + Schedule C through `income import`, `income
  clear`, `load`'s stale discard) each refuse and the draft survives byte-identical; a field added to
  `ReturnInputs` is protected the day it is added (plant: a draft differing only in a new/uncounted
  field → refuses).
- **M-1, M-2, N-1 — fold inline. M-3 — fold** (read-only surfaces continue with a note).

Nothing is folded at the time of this ledger. Fold brief: `BRIEF-fold-interview-T4-review.md`.
