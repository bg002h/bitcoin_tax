# REPORT — I-1 fold: the refusal census's denominator (B3 interaction review, 2026-09-13)

Implementer: independent agent, own worktree `agent-a8b0d2ac5ab0fba96`,
`CARGO_TARGET_DIR=<worktree>/target-i1` (clippy in `target-i1-clippy`). Every command foregrounded.
No subagents. **Nothing committed, nothing pushed.** Files changed: `crates/xtask/src/blockers.rs`
and `design/agent-reports/TY2026-BLOCKERS-PREDICTION-2026-09-13.txt` — and nothing else.
`return_refuse.rs` and `charitable.rs` were read, never written.

## Verdict

**I-1 is closed. The brief's premise held — I could not refute it.** The fix is structural on both
halves the report named, the universal claim is gone, five separate mutations were each watched RED,
and §68 now reaches the TY2026 answer as a substantive row.

Two things the brief did **not** name are reported below: a second boundary this census still has
(§ *What this still does not claim*), and a defect in **my own first attempt at the B1 plant** that
only the mutation run exposed (§ *The plant that passed for the wrong reason*).

## The premise, re-measured independently — CONFIRMED, not refuted

Four `RefuseReason` variants carry a `year` payload. Derived by parsing the enum body and splitting
at top-level commas (not by grepping declaration lines, which finds only one — three of the four
declare the field on a *continuation* line):

| variant | payload |
|---|---|
| `BrokerAnswerUnread` | `{ provider: String, cohort: crate::forms::Cohort, year: i32 }` |
| `BrokerReportingUnanswered` | `{ provider: String, cohort: crate::forms::Cohort, year: i32 }` |
| `ItemizedDeductionLimitationNotComputed` | `{ year: i32 }` |
| `Schedule1aNotOnThisYearsReturn` | `{ year: i32 }` |

134 variants declared; the old census attributed **one** of the four. Every other measured claim in
I-1 also reproduced: two needles at the old `blockers.rs:1166`, `UNCLASSIFIED` computed only over
sites those needles found, the §68 site at `return_refuse.rs:2819/2828`, and the frozen 58-row
prediction containing no §68 row. The report's line citations resolve exactly against `HEAD`.

## What changed — the two structural causes, plus the claim

**1. The denominator is inverted.** `year_sites` is gone. `refusal_census(src)` now enumerates
**every** `RefuseReason::…` construction in the scanned body first — 95 sites raising 81 distinct
variants — and classifies each afterwards. A site cannot fall out between needles because there are
none. Classification has three inputs, two of them derived from something that cannot go stale:

* **the TYPE** — `year_payload` is read off the enum, so a variant that cannot be raised without a
  year is in the census whether or not any scanner recognises its guard idiom, and a *fifth* one
  joins with no edit;
* **the site's decision region** — the code between the previous refusal *statement* and this
  construction, plus the construction's own payload, with comments and string contents blanked by a
  small lexer (`code_lines`). Only a year read in CODE decides anything: reading prose calls **34 of
  the 95** sites year-reading, measured;
* **`year_keyed_markers()`** — now DERIVED from `gates()` via a new `Gate::keyed_by` field, so the
  attribution set widens the day a gate lands, and a gate with nothing to name must write `None`
  because the compiler asks. The hand-typed `YEAR_KEYED` const is gone. The two bare year
  comparisons that no gate owns are a named const with the boundary stated in its doc.

Result at HEAD: **11 year-reading sites, 8 distinct variants** (was 11 sites / 5 variants), all four
`year`-payload variants located, **2 UNCLASSIFIED** — both in `screen_broker_reporting`, which reads
the year through a parameter and a `regime` that no `Gate` probes. They are rows, not silence.

**2. The completeness assertion is exhaustive against the type.** What it replaced required three
hand-typed names to be *present* (`vars.contains(want)`), beside a variant set that grew by six in a
day; containment cannot notice a missing fourth, and three of the four were missing. Now
`unlocated_year_payload()` is `year_payload − located`, both sides derived, and any non-empty result
is **a printed row** as well as a red test. The four names are still pinned as literals first
(FR-230) with the derived check as the enforcement.

**3. The universal claim is gone.** The old sentence — *"every other variant never reads the year, so
it cannot fire because the year is 2026"* — is replaced by what the instrument can support, plus a
second note headed **WHAT IT DOES NOT CLAIM** that states both boundaries in the output itself.

**4. §68 is a real gate.** A `Gate` entry with a probe on `section_68_status(y)`, an `_`-free match
(a fourth `Section68Status` is a build error in xtask too), and the anchor
`crate::tax::tables::section_68_status(year) {` — so the existing
`every_gate_anchor_still_resolves_and_a_moved_one_is_refused` covers it, and a moved anchor reds.

## B1 — five mutations, each watched RED

★ The old plant read `ri.tax_year > 0`, *the needle the old scanner already searched for*. The new
one reads the year the way the invisible sites do — through a function parameter — and asserts
against literal copies of both old needles that the old scanner could not have seen it.

**M1 — reinstate the exact old blindness** (`YEAR_READ` drops the bare `year`):

```
FAIL a_year_read_through_a_parameter_is_found_and_the_old_needles_could_not_see_it
  a tax year read through a function PARAMETER is invisible — the defect B3 I-1 found:
  RaiseSite { line: 9, variant: "ItemizedDeductionLimitationNotComputed", … year_read: [], keyed: [] }
FAIL a_year_reading_site_naming_no_gate_is_unclassified_and_never_vanishes
Summary 18 tests run: 16 passed, 2 failed
```

**M2 — restore the needle-driven denominator** (enumerate only sites whose line names an old needle):

```
FAIL a_year_read_through_a_parameter_is_found_and_the_old_needles_could_not_see_it   assertion failed: []
FAIL a_year_reading_site_naming_no_gate_is_unclassified_and_never_vanishes           assertion failed: []
FAIL the_refusal_census_enumerates_every_raise_site_at_head
FAIL the_command_answers_for_ty2026_at_head
  the UNCLASSIFIED set moved — look at it, do not widen this assertion
Summary 18 tests run: 14 passed, 4 failed
```

**M3 — make an unlocated `year`-payload variant silent again** (`unlocated_year_payload` → empty):

```
FAIL a_year_payload_variant_with_no_raise_site_is_reported_not_assumed_year_agnostic
  a variant that cannot be raised without the year, and no raise site found, must be REPORTED
  — it is the §68 hole
Summary 18 tests run: 17 passed, 1 failed
```

**M4 — the §68 gate stops being an attribution key** (`keyed_by: None`):

```
FAIL a_year_read_through_a_parameter_is_found_and_the_old_needles_could_not_see_it
FAIL the_year_keyed_markers_come_from_the_gates_and_section_68_is_one_of_them
FAIL the_refusal_census_enumerates_every_raise_site_at_head
  §68 must be ATTRIBUTED to its gate, not merely noticed
FAIL the_command_answers_for_ty2026_at_head
Summary 18 tests run: 14 passed, 4 failed
```

**M5 — remove both message-bleed guards** (`prev_end = *li + 1`):

```
FAIL a_year_read_through_a_parameter_is_found_and_the_old_needles_could_not_see_it
  the previous refusal's MESSAGE bled forward into this rule:
  RaiseSite { line: 13, variant: "Second", … year_read: ["`tax_year`", "`year`"] }
FAIL the_refusal_census_enumerates_every_raise_site_at_head
  a refusal whose decision reads no year must not be called year-reading
FAIL the_command_answers_for_ty2026_at_head
Summary 18 tests run: 15 passed, 3 failed
```

★ M5 was run twice on purpose. Dropping only the *statement* extent reds two tests but **not** the
HEAD precision assertion, because the *payload* extent independently still blocks the bleed. Both
guards had to go for `BrokerReportingMixed` to be mislabelled. Recorded because the first run would
otherwise read as a partial plant framing a good kill as blind.

Green again after each restore, from a byte-exact backup copy (never `git checkout --`).

## The plant that passed for the wrong reason — my own defect, found by running the mutation

The first version of the headline B1 test asserted the §68 parameter plant was found year-reading.
**M1 left it GREEN.** The site was still reached two other ways — the `section_68_status` marker in
`keyed`, and the `{ year }` payload text — so the assertion held while the exact blindness it existed
to kill was reinstated underneath it. A kill that can pass for another reason is not a kill.

Fixed by re-running the same plant with the year-keyed call replaced by `an_unknown_helper(year)`, so
the bare parameter is the only signal there is, and asserting `keyed.is_empty()` first so the plant
cannot quietly stop being opaque. M1 then reds, as pasted above. The comment in the test records the
measurement rather than the conclusion.

★ This is the brief's own warning arriving in practice, and **no amount of re-reading the test would
have found it** — only the mutation did.

## What this still does not claim — a second boundary the brief did not name

The census reads **one file**. Of the 134 declared variants, **53 are raised nowhere in
`return_refuse.rs`**, and raise sites exist elsewhere in non-test code — measured:
`questions.rs` 68, `return_1040.rs` 22, `xtask/box_census.rs` 21, `interview_state.rs` 5,
`btctax-cli/src/step0.rs` 1. So even the corrected census cannot support a universal over 134
variants, and it no longer pretends to: the output names the count and says nothing here claims those
cannot fire because the year is 2026. That is the brief's rule (3) — an honest boundary — rather than
a silent one.

**Follow-up candidate for the controller** (not filed; no FR number claimed): extend the census to
every raise site in the workspace's non-test sources, at which point the universal becomes provable.
It needs a construction-vs-match-pattern discriminator (`btctax-tui-edit/src/edit/tax_inputs.rs:864`
matches four broker variants in a pattern, and `box_census.rs` raises through `reason: ||` closures),
which is why it is a separate piece of work and not part of this fold.

Also unchanged on purpose: for the 84 sites called year-blind, the finding is *"no year read appears
in the code between this refusal and the previous one"* — not *"this refusal never reads the year."* A
year consulted further up a 1600-line `screen_inputs_tiered` is outside the window. The output says
so, and `year`-payload variants are exempt because they are read off the type.

## §68 reaches the answer — and the prediction delta

`design/agent-reports/TY2026-BLOCKERS-PREDICTION-2026-09-13.txt` regenerated in place:
**58 rows → 61 rows (15 UNMEASURED → 17).** Three rows gained, all three new facts:

1. **`clears itself in January` 41 → 42 rows.** `BLOCKED | with the year's document | the gate *the
   §68 ITEMIZED DEDUCTIONS WORKSHEET is available for the year* is shut for TY2026 and OPEN for
   [2024, 2025] — it refuses BECAUSE the year is 2026: §68 applies to TY2026 and its Schedule A line
   18 prints the gate, but the Itemized Deductions Worksheet it sends a Yes to lives in Form 1040 /
   Schedule A instructions the IRS has not published — so EVERY return that itemizes with AGI less
   the QBI deduction less Schedule 1-A additional deductions over $384350 is refused. ★ January's
   package does not clear this by arriving: the worksheet must be TRANSCRIBED.` Evidence cell:
   `tables.rs section_68_status; raises RefuseReason::ItemizedDeductionLimitationNotComputed (probed
   at [2024, 2025, 2026])`.
2. **`we must build` 13 → 15 rows.** `RefuseReason::BrokerAnswerUnread` at line 2923 — UNCLASSIFIED.
3. …and `RefuseReason::BrokerReportingUnanswered` at line 2943 — UNCLASSIFIED. Neither could have
   become a row before: `UNCLASSIFIED` was computed over sites the needles had already found.

Both notes lines were rewritten (the census note, and the new `WHAT IT DOES NOT CLAIM` note). Two
lines changed for reasons unrelated to this fold: the frozen file was generated three commits back, so
it read `133 variants` / `361 source files` where HEAD reads `134` / `364`.

★ **One judgment call flagged for the controller.** The brief said to regenerate that path, so it was
regenerated in place; the pre-fix 58-row version is preserved in history at `3199a13de`. But
`design/agent-reports/BRIEF-stage2-run.md` and `REPORT-stage2-A.md` both cite that filename as
stage 2's *frozen baseline*, so what they point at has changed meaning. If the stage-2 comparison
should keep the original as its baseline, restore the file and write the corrected list to a new
dated path instead — a one-command change either way.

## A false citation I introduced and caught in my own work

The first draft of the fix cited **`FR-231`** in seven places as the follow-up for this defect.
`FR-231` already exists in `FOLLOWUPS.md:8950` and means something else entirely (*"recorded, NOT a
defect: the §68 screen is deliberately over-inclusive"*). All seven now cite **B3 I-1** with one
authoritative path reference on `RefusalCensus`. No FR number is claimed by this fold; if the
controller wants one filed, the next free number is FR-234.

## Gates, at the final state

| gate | result |
|---|---|
| `cargo nextest run --workspace --no-fail-fast` | **3811 passed, 0 failed, 12 skipped** (was 3806/12 at HEAD; +6 tests, −1 removed) |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | **0 warnings, exit 0** |
| `cargo fmt --all --check` | **exit 0** |
| `cargo run -p xtask -- blockers 2026` | exit 0, 61 rows |
| B1 mutations M1-M5 | **each watched RED, restored green** |

Tests added: `a_year_read_through_a_parameter_is_found_and_the_old_needles_could_not_see_it`,
`a_year_reading_site_naming_no_gate_is_unclassified_and_never_vanishes`,
`a_year_payload_variant_with_no_raise_site_is_reported_not_assumed_year_agnostic`,
`the_refusal_census_enumerates_every_raise_site_at_head`,
`the_year_keyed_markers_come_from_the_gates_and_section_68_is_one_of_them`,
`code_lines_blanks_prose_and_survives_an_r_before_a_quote`. Removed:
`the_year_site_census_attributes_the_real_sites` (its subject, forward attribution from a year read to
a variant, cannot recur — the census reads the variant off the construction it is standing on).

One existing assertion was deliberately changed rather than deleted:
`the_command_answers_for_ty2026_at_head` required `0 UNCLASSIFIED site group(s)`. That zero **was the
symptom**. It now pins the real set by name — `2 UNCLASSIFIED variant(s) [BrokerAnswerUnread,
BrokerReportingUnanswered]` — so a third one still reds, plus `0 year-payload variant(s) with no
located raise site` and three §68 phrases that must appear in the TY2026 answer.
