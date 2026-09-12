# REVERIFY — the step-5 f6251 fold (`6356043e`)

**Verifier:** one sonnet re-verifier, isolated worktree `cb5c0269`
(`/scratch/code/bitcoin_tax/.claude/worktrees/agent-a6239c78921392e0b`). Nothing built beyond reading
source; no file edited but this one; no subagents.

**Counts: 0 Critical / 1 Important / 0 Minor / 0 Nit.**

---

## Verdict

The fold does close the collision the review found, and every kill I traced actually discriminates —
the two corrected comments are true against the extracts, the M-1 coverage kill reds on a *single*
wrong row (not just "0 vs 2"), the I-2 read-back genuinely checks values (not just presence) and its
`42` is measured against `map.money_cells().len()`, and N-1's `after_the_only` fails closed on any
number of repeated occurrences, not only the tested double. `SeniorDeductionSubtotal` — the type the
brief names — is exactly as unforgeable as claimed: no `Deserialize`, no `Default`, no `From`, one
construction site in the whole workspace, private fields, and the single production call chain
(`return_1040.rs:2647` → `compute_6251`, the only production caller) derives it correctly.

But the brief's Hunt 1 asked the wrong question by one hop. The type that is *actually unforgeable*
(`SeniorDeductionSubtotal`) is not the value the emitter's join compares. By the time the figure
reaches `fill_form_6251_obbba_with_map`'s `revision.schedule_1a_line_agreeing_with(schedule_1a_line)?`
call, `schedule_1a_line` has been unpacked into a **bare, public `u32` field** on the *output* enum
`Form6251Line1::Y2025` (`crates/btctax-core/src/tax/form6251.rs:67-71`). That field carries none of
`SeniorDeductionSubtotal`'s protection: it is directly settable by any code that can write a
`Form6251Line1::Y2025 { .. }` literal, from any crate, to any value — proven, not inferred, because the
fold's own tests already do exactly this (`crates/btctax-forms/tests/f6251_obbba.rs:759,965,1129`,
compiling from `btctax-forms`, a different crate than the one that defines the enum). So the "unforgeable
type" claim in the fold's report is true as stated, and also narrower than the guarantee the report's
prose implies for "the join" as a whole — the join's actual input has already left the protected type
by the time it is checked. See Finding 1.

This is **Important, not Critical**: the sole production constructor of `Form6251Line1::Y2025`
(`compute_6251`, `form6251.rs:494-502`, called from exactly one place) always derives the field
correctly via `senior_deduction.schedule_1a_line()`, so no live or near-live wrong-result path exists
today, and the one realistic porter mistake the review's I-1 was about — reusing the `Y2025` accessor
call against the wrong schedule — is fully caught (measured, Layers 1-3 in the fold's own report).
Defeating the join as I describe requires a second, as-yet-nonexistent constructor that deliberately
hand-types a `schedule_1a_line` disagreeing with where the amount actually came from — a materially
harder and less natural mistake than the one this fold was built to prevent, and Hunt 2 confirms no such
second constructor exists yet.

---

## Findings

### Important — F1: the emitter's join operates on a bare, forgeable `u32`, one hop downstream of the type the fold protects

**Where:** `crates/btctax-core/src/tax/form6251.rs:67-71` (`Form6251Line1::Y2025.schedule_1a_line: u32`,
no visibility modifier possible on an enum-variant field — it is exactly as visible as the `pub enum`
itself, line 47); consumed by the join at `crates/btctax-forms/src/form6251.rs:296-321`
(`revision.schedule_1a_line_agreeing_with(schedule_1a_line)?`).

**Mechanism.** `SeniorDeductionSubtotal` (`schedule_1a.rs:1418-1437`) is genuinely unforgeable — private
fields, no `Deserialize`/`Default`/`From`, and the *only* construction site in the workspace is
`Schedule1A::senior_deduction_subtotal` (`schedule_1a.rs:1388-1394`; confirmed by grep, one hit). But
that protection ends the moment `compute_6251` calls `.schedule_1a_line()` to populate the *output*
enum's plain `u32` field (`form6251.rs:494-502`). From there to the emitter, the number is an ordinary
public integer on an ordinary public struct (`Form6251`, whose `line1: Form6251Line1` field is `pub`,
`form6251.rs:103`), and `#[non_exhaustive]` on the enum (line 46) does **not** block struct-literal
construction of an existing variant from another crate — it only requires a wildcard arm when
*matching*. I confirmed this is not a theoretical reading of the attribute: `btctax-forms`, a different
crate, already constructs `Form6251Line1::Y2025 { .. }` literals directly in three places in its own
test file (`f6251_obbba.rs:759,965,1129`), and they compile.

**Concrete failure scenario.** A future second way of building a `Form6251` (a slice export, a golden
generator, or a `return_1040.rs` edit for a later year that skips `compute_6251` and hand-assembles the
struct) sets `schedule_1a_line: 43` to match whatever the target map states, while `line1a` is actually
computed from a schedule struct whose real subtotal line is 37 (e.g., because the porter has not yet
transcribed the new year's own `Schedule1A`-equivalent and reused the old one for the *amount* while
correctly guessing the *map's* stated line for the *label*). `revision.schedule_1a_line_agreeing_with(43)`
against a map stating 43 returns `Ok(43)` — the join agrees, because it only checks "does the claimed
line match what this revision prints," never "does the claimed line match where the amount was actually
read from." The wrong figure fills with every instrument green, exactly the shape of harm the fold exists
to prevent, recreated one layer downstream of where it closed the first occurrence.

**Why this is Important and not Critical.** No such second constructor exists today (Hunt 2, below) —
`fill_form_6251_obbba_with_map` is the only fill path, and it is called only from
`crates/btctax-forms/tests/f6251_obbba.rs`; `compute_6251` is the only production builder of
`Form6251Line1::Y2025` and it is called from exactly one production site
(`return_1040.rs:2647`, inside `assemble_absolute`), which always calls the protected accessor. So this
is armed only in the sense that the *type system* does not prevent it — there is no live or near-live
path, and the fold's report does not claim otherwise (it claims `SeniorDeductionSubtotal` is unforgeable,
which is true; it does not claim the same for `Form6251Line1::Y2025.schedule_1a_line`).

**Recommendation (not actioned — reporting only):** carry `SeniorDeductionSubtotal` itself through to
`Form6251Line1::Y2025` (or a same-shaped private-field wrapper around the bare `u32`) instead of
unpacking to a plain integer, so the emitter's join input is protected by the same mechanism as its
source, all the way to the point of use. This is a natural companion to FR-122 (line-coverage not gated
in CI) as a residue item for the TY2026 port — filing it is outside this re-verification's scope (edit
no file but the report).

---

## Refuted premises

None. Every "already settled" row in the brief held under my own re-derivation (I independently
confirmed the private fields, the single construction site, the E0559/Layer-2/Layer-3 mechanics by
reading the relevant source rather than by trusting the report's paraphrase). No factual claim in the
FOLD report or the brief was found to be false.

---

## Checked clean — one line per hunt

1. **Is `SeniorDeductionSubtotal` genuinely unforgeable?** Yes, as literally asked. Read the full
   `schedule_1a.rs` module: `#[derive(Debug, Clone, Copy, PartialEq, Eq)]` only (no `Deserialize`, no
   `Default`, no `From`); grepped `"SeniorDeductionSubtotal {"` — one construction site in the whole
   workspace (`schedule_1a.rs:1389`); grepped for any `pub(crate)`/`#[cfg(test)]` helper near it — none.
   Confirmed no `Deserialize` anywhere touches `Schedule1A` either, so the TOML round-trip the brief
   flagged cannot inject a line number (only a schedule *value*, under the type's own hardcoded
   constant). See Finding 1 for the adjacent gap this hunt's literal framing did not reach.
2. **Does every path that fills an OBBBA Form 6251 go through the join?** Yes. Grepped every reference
   to `fill_form_6251_obbba_with_map` and `Form6251ObbbaMap` across the workspace: the only fill function
   is re-exported from `pub mod testonly` (`lib.rs:552`, with the module's own comment stating
   `packet.rs` does not reach it), and its only caller is `crates/btctax-forms/tests/f6251_obbba.rs`,
   which never writes a field directly (grepped `apply_writes`/`FieldValue::` in that file — zero hits).
   The join (`form6251.rs:296-321`) runs unconditionally near the top of the one function, before any
   PDF load or write. No second fill path, slice export, snapshot, or golden generator exists in
   `crates/` or `scripts/`.
3. **Are the two corrected comments now true?** Yes, checked against the primary source, not the report.
   `f1040s1a--2025.txt:108` reads "Enhanced deduction for seniors. Add lines 36a and 36b" for line 37;
   `f1040s1a--2026-DRAFT.txt:213` reads "Enter the amount from line 3" for line 37 and `:222` "Enhanced
   deduction for seniors. Add lines 42a and 42b" for line 43; line 3 (`:59`) is "Add lines 1 and 2e"
   under the Part I header "Modified Adjusted Gross Income (MAGI) Amount" (`:52`), and line 1 (`:53`) is
   1040 line 11b. Both `f6251_revision.rs:19-33` and `return_1040.rs:2971-2985`'s corrected prose match
   this exactly.
4. **Does the M-1 coverage kill discriminate per row?** Yes. Read
   `the_obbba_line_1_rows_are_verbatim_and_a_moved_cross_reference_reds`
   (`crates/xtask/src/line_coverage_check.rs:1524-1583`) in full: direction 3 mutates only the `1a` row's
   instruction text to the TY2026 draft's "line 43" sentence, leaves `1b` untouched, and asserts the
   error names `f6251:1a` specifically and `NOT FOUND in f6251--2025.txt` — a single wrong sentence with
   the row count still 2 is caught, not just an empty-vs-nonempty count.
5. **Does I-2's read-back compare values, and is `checked == 42` derived?** Compares values, not just
   presence, for the shape that matters. The sweep
   (`f6251_obbba.rs:1030-1094`) checks `!v.contains('.')` (a real format assertion, over the *serialized
   PDF*) for all 42 cells, and separately asserts exact values for line 33 (`"0"`), line 36
   (`"163750"`, a different box, a different figure — the exact pair the review said no test had ever
   reached), line 40, line 2b, line 1a and line 37. `checked == 42` is pinned alongside a *separately
   measured* `map.money_cells().len() == 42`, per the test's own stated reason (so the number's meaning
   is unambiguous). Compared this shape against the pre-existing TY2024 twin
   (`f6251_fill.rs:194-235`) and found it identical — whole-dollar sweep + count + a handful of exact
   spot-checks — so the new test is a faithful port at the same fidelity as its sibling, not a weaker
   copy; the "does it verify all 29 cells individually" question the hunt raises is answered "no, same
   as TY2024's twin," which is a pre-existing pattern limitation, not a defect this fold introduced.
6. **Does N-1 fail closed on more than two occurrences?** Yes, by the algorithm's own structure, not
   only the tested case. `after_the_only` (`f6251_revision.rs:232-237`) uses `sentence.split(anchor)`
   and requires exactly two segments (`it.next()` after consuming two must be `None`); any occurrence
   count ≥2 produces ≥3 segments, so the `is_none()` check fails for 2, 3, or any higher count alike —
   traced the logic by hand for N=0,1,2,3. The committed test
   (`f6251_revision.rs:415-437`) plants exactly two occurrences in all three accessors and confirms each
   returns `None`, and separately confirms `schedule_1a_line_agreeing_with` also refuses on both
   candidate readings (37 and 43) rather than accepting either.
7. **Scope creep?** None found. Diffed every file in `6356043e`: `f6251_fill.rs`'s +3 lines are the
   mechanical new-field addition required by the struct change; `FOLLOWUPS.md`'s +41 lines are exactly
   FR-120/121/122, matching the fold report's own filed list, no duplicate numbering
   (`grep -c` confirms exactly one header per FR); `line_coverage_check.rs`'s diff is 82 insertions vs 1
   deletion (pure addition); `map.rs`'s M-3 diff removes the unchecked `$900,350` quote and points at the
   checked copy, nothing else touched. No `2026` arm, no `FullReturnParams`, no fail-closed gate edited
   anywhere in the diff.

---

## Out of scope

The build under repair (`d8d023af`), the rest of the branch, `FullReturnParams`, any TY2026 wiring, the
Schedule 1-A emitter, and re-running `make gate` (per the brief: the controller's run stands, and no
finding here rests on a suite result — every claim above is a source read or a grep, both quoted inline
above with exact locations). FR-120/FR-121/FR-122 themselves were not re-adjudicated beyond confirming
they are filed and non-duplicated; their content was already covered by the review this fold answers.
