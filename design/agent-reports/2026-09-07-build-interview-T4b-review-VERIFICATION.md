# Verification ledger — interview T4b seam review (`2026-09-07-build-interview-T4b-review.md`, 1C/3I/4M/2N)

Controller machine-checks from the main tree at `8ce1f470`, before any fold.

| # | claim | check run | measured | decision |
|---|---|---|---|---|
| C-1 | `skippable_state` decides `Given`/`Declined` by reading the VALUE | `sed -n 139,157p answer.rs` | `SkippableKind::Date => (sk.get_date)(ri).is_some()` → `Given`, else `Declined`; the doc says so in words | true |
| C-1 | the seed pre-fills the `Durable` date of birth | grep `open_next_year.rs` | `:254 date_of_birth: p.date_of_birth` | true — with the reader above, a bare Enter records a this-year `Given` (the review's probe: `answered_on: 2026-02-03, state: Given`) |
| C-1 | `Durability::Durable`'s contract forbids exactly this | `sed -n 31,34p questions.rs` | *"The prior MAY be displayed, but it still requires the same explicit keystroke as a fresh ask: never Enter-to-accept, never pre-filled."* | true; **FOLD** (a): shown, not pre-filled |
| I-1 | `filing_status` and the header cross unnamed; the classifier's `SerdeRequired` ground is bypassed by `seed()` | grep | `:259 filing_status: prior.filing_status`; `classifier.rs:148 Class::SerdeRequired` for the field | true; **FOLD** (name what crossed; a confirmation question live on an opened year; the exemption text corrected) |
| I-2 | dependents are seeded with no answer surface | grep | `:276 .map(\|d\| Dependent {`; `identities_of` dependent arm `census_row: None` (`:367`) | true; **FOLD** (a): prompt only, no seeded row, until T7's gates give the row an answer (FR-70) |
| I-3 | the seeded venue key flips the stored-answers predicate | grep `resolve.rs` | `:184-186 answers_stored = … !r.broker_reporting.0.is_empty()` — presence in the key set IS answered-ness | true; **FOLD** (drop the arm) |
| M-1 | `--discard-draft` after the opener prints a discard that did not happen | the review's probe (draft byte-identical after both calls) | mechanism: `coherence_clear` notes before the write saves | fold inline |
| M-2 | `payer_of` closes with `_ => Default::default()` | grep | `:421` | fold inline (exhaustive) |
| M-3 / M-4 | the retention kill covers the committed row only; the delegation kill catches the first offender only | read | true | fold inline |
| N-1 / N-2 | `render`'s `{from}` replace; `let to = from + 1` before any check | grep | `:154`, `:104-105` | fold inline |

## Disposition — the shape
The opener may carry an IDENTITY only where this year has a SURFACE to answer it. One new
provenance leaf makes that structural: `ReturnInputs.opened_from: Option<i32>` (`#[serde(default)]`;
a provenance class in the classifier and `LEAF_SOURCE`, never money, never testimony). The seed sets
it to N. Then:
- **C-1:** the seed leaves `date_of_birth: None` (taxpayer and spouse). The prior date is DISPLAYED
  at prompt time — `income answer` and the TUI, when `opened_from` is `Some(N)`, read year N's
  committed row for the hint and show it in the prompt; the filer types it to confirm (a fresh
  `SetField` → a fresh record); a bare Enter records `Declined` and `is_aged` forgoes. Kill: the
  review's probe.
- **I-1:** `filing_status` stays carried (the field has no `None`; not carrying asserts Single) and
  gains its confirmation surface: a class-(A) `FormQuestion` `FilingStatusConfirmed` — *"Your TY(N)
  return filed as <status>. Is <status> your filing status for TY(N+1)?"* — live iff `opened_from`
  is `Some`; `No` refuses naming where to change it (the TUI header / the TOML) so the question is
  re-asked on the new status. The classifier's `SerdeRequired` text for `filing_status` is corrected
  to cite this question as the ground on an opened year. `Opened` gains `carried_identity` and
  `render`, `--help`, the man page and the TUI offer name every carried field.
- **I-2:** dependents are NOT seeded; `identities_of` names them in the prompt from `prior` only;
  FR-70 (T7) seeds them once `DEPENDENT_GATES` exist to answer for each row.
- **I-3:** the `broker_reporting` arm leaves `seed`; the venue prompt already reads `prior`.
- **M-1 → M-4, N-1, N-2:** inline as the review's minimal changes.

Nothing is folded at the time of this ledger. Fold brief: `BRIEF-fold-interview-T4b-review.md`.
