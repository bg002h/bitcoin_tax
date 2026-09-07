# Verification ledger — interview T6 seam review (`2026-09-07-build-interview-T6-review.md`, 1C/1I/4M/4N)

Controller machine-checks from the main tree at `23765606`, before any fold.

| # | claim | check run | measured | decision |
|---|---|---|---|---|
| C-1 | the R6 arm-(2) crypto slice prints the Digital Assets box from the ledger predicate, never the answer | grep `admin.rs` | `:1150 let da_yes = !rows.is_empty() …`; `digital_asset` appears only at `:440` (hand_marks on the full-return `PrintedReturn`) and in a test name; `Form1040Inputs.da_yes: bool` (`form1040.rs:98`); the slice arm sets `advisories: Vec::new()` (`:1233`) | true — the review's PDF read-back (stored `Some(false)` → Yes; `None` → Yes, no refusal, no hand-mark) follows from the code; **FOLD** |
| C-1 | spec 1099-DA R6 exempts the slice from `screen_compute_dependent` on the premise *"neither reaches a figure the slice prints"* | `grep` the spec | `SPEC_1099da_broker_reporting.md:303` | true; the premise is invalidated by T6 (the box is now an answer) — the controller amends R6's sentence; arm (2) runs the DA cross-check |
| I-1 | the Step 0 venue list consults no regime; every Step 0 fixture is TY2026 | `grep -c 'regime\|broker_question_is_live' step0.rs`; `const YEAR` | 0; `step0_panel.rs:31 const YEAR: i32 = 2026`; `broker_question_is_live` exists at `forms.rs:130` | true; **FOLD** (thread the regime; fixtures at TY2024/2025 asserting the row's absence) |
| M-1 | the standing-order row fires outside the Notice's relief period (2025-01-01 → 2026-12-31) | the Notice's §.03 at `:299-300` | the relief period is defined there | true; fold (gate on the period; pre-2025 → the `Pre2025MethodNote` exit) |
| M-2 | a same-day election after the sale silences the row; the root is `resolve_election`'s `<=` on a day-granular date, which also decides FILED BASIS | the review's plant | pre-existing engine behaviour | **FR-77** (owning: the method-election track, before the TY2026 filing); T6 folds only the same-day fixture + honest doc |
| M-3 | the granularity negative test scans only static prompts | read | true | fold (render `RENDERED_PROMPTS` too) |
| M-4 | a contradicted `No` is not caught at TUI commit (no ledger there) and the panel cannot show it | read (`input_form_store::commit` runs `screen_inputs` only) | true; a tier boundary | fold the cheap half: the Step 0 print (which holds the ledger) surfaces the contradiction row; commit stays as is, recorded as a decision |
| N-1 … N-4 | doc comment displaced; report inconsistencies (2024 vs 2026 date; 3,358 vs 3,363); off-by-a-few cites; a KAT comment | read | true | fold inline |
| (a) (b) | the pointed questions | the review's answers | (a) there is no fallback — the slice reads the ledger unconditionally (C-1); (b) a global election satisfies §4.02(2) — no finding | recorded |

## Disposition
- **C-1 — FOLD.** `Form1040Inputs.da_yes` becomes `digital_asset_answer: Option<bool>` fed from the
  working return the arm already resolved (`admin.rs:783`); `fill_form_1040_capgains` checks Yes or
  No from the answer and prints NEITHER on `None`, with the slice's `hand_marks`/note naming the
  unanswered box (the fail-closed backstop now fires on this path); the produce/skip decision gets
  its own predicate (rows or income in the year). Arm (2) runs the DA cross-check before printing: a
  contradicted `No` REFUSES the export naming the first qualifying event; an unwitnessed `Yes`
  prints with the off-ledger advisory (the slice's `advisories` no longer `Vec::new()` for this
  rule). Kills: the review's three plants read back off the PDF (`Some(false)` → No; `None` →
  neither + the mark; `Some(true)` → Yes), the contradicted `No` refusing on arm (2), and the
  full-return path unchanged.
- **I-1 — FOLD.** `step0_panel` takes the year's regime; the venues list exists only when
  `broker_question_is_live(&rows, regime)`; on a non-live year the row states the regime instead;
  the commit modal's heading stops asserting a gate that does not exist on such years; fixtures at
  TY2024 and TY2025 assert the row's ABSENCE, TY2026 its presence.
- **M-1, M-3, M-4 (the Step 0 half), N-1 … N-4 — fold inline. M-2 — FR-77 + the same-day fixture.**
- **Spec amendment (controller):** `SPEC_1099da_broker_reporting.md` R6's premise sentence — the
  slice now prints a figure a compute-dependent rule reaches (the Digital Assets box), so arm (2)
  runs exactly that rule.

Nothing is folded at the time of this ledger. Fold brief: `BRIEF-fold-interview-T6-review.md`.
