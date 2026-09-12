# FOLD — the step-5 re-verification's one Important (the provenance hop)

**Report folded:** `design/agent-reports/REVERIFY-step5-f6251-fold.md` (`78d77754`, 0C/1I/0M/0N).
**Ledger + brief:** `…-VERIFICATION-AND-FOLD-BRIEF.md` (`27d3351a`). **Fold under repair:** `6356043e`.
One opus implementer, shared main tree, **nothing committed, no subagents, no `git stash` / `checkout` /
`revert`** — every mutation planted, measured and restored from a `cp` backup.

**Gate, at the folded state:** `make gate` → **3596 tests run: 3596 passed, 12 skipped, exit 0**
(baseline 3594/12; the delta is the two new xtask tests below). `cargo fmt --all --check` clean.
`cargo doc -p btctax-core -p btctax-forms --no-deps`: **0 warnings in any file this fold touched** (81
pre-existing elsewhere, the FR-119 noise). `cargo run -p xtask -- line-coverage`: **OK, 377 money lines
across 18 forms, f6251:43**, ratchets unmoved — 31 exceptions (31), 0 unverifiable (0), 17 not
line-bound (17). **Diff:** 11 files, **+240 / −46**, plus one new file
(`crates/xtask/src/forge_reach_check.rs`, 291 lines).

---

## 1. The mechanism chosen, and why it is not the one the brief named first

**Chosen: `Form6251Line1::Y2025` carries `SeniorDeductionSubtotal` instead of a bare
`schedule_1a_line: u32`.** The brief's option 2, which the re-verification itself recommended. Not
`#[non_exhaustive]` on the variant — and the reason is a measurement, not a preference.

`#[non_exhaustive]` binds only **outside the defining crate**, and the finding's own failure scenario is
*inside* it. The re-verification's words: *"a `return_1040.rs` edit for a later year that skips
`compute_6251` and hand-assembles the struct."* `return_1040.rs` is `btctax-core`. So the idiomatic
defence would have left the named scenario fully expressible, exactly as `#[non_exhaustive]` on the
*enum* left the variant fully constructible — the same shape one level down, for the third time in this
seam.

Measured, by planting `#[non_exhaustive]` on the variant on the pre-fold tree and running
`cargo check --workspace --all-targets` (restored from a `cp` backup afterwards). It costs a guarantee
that is live today:

```
error[E0638]: `..` required with variant marked as non-exhaustive
   --> crates/btctax-forms/src/form6251.rs:296:9
296 | /         btctax_core::tax::form6251::Form6251Line1::Y2025 {
297 | |             line1a,
298 | |             line1b,
299 | |             schedule_1a_line,
300 | |         } => (line1a, line1b, schedule_1a_line),
    | |_________^
help: add `..` at the end of the field list to ignore all other fields
```

The emitter's destructure is the one place that **must** see a field added to this variant, and
`#[non_exhaustive]` on the variant orders it to stop looking. With `..` added, the four remaining
external sites then failed as expected (`E0639`, listed in §2) — so the attribute does block external
construction; it just buys less than it costs here.

**What carrying the type buys instead.** `SeniorDeductionSubtotal`'s fields are private to the
`schedule_1a` **module**, so the forge is impossible from every other module *including inside
`btctax-core`*. Planted in `return_1040.rs` — the finding's own scenario:

```
error[E0451]: fields `amount` and `schedule_1a_line` of struct `SeniorDeductionSubtotal` are private
     --> crates/btctax-core/src/tax/return_1040.rs:11908:13
      |             ^^^^^^ private field
      |             ^^^^^^^^^^^^^^^^ private field
```

And from outside the crate (`btctax-forms/src/form6251.rs`), the same `E0451`; and the old field name is
simply gone:

```
error[E0559]: variant `Form6251Line1::Y2025` has no field named `schedule_1a_line`
   --> crates/btctax-forms/src/form6251.rs:330:9
    = note: available fields are: `senior_deduction`
```

**Two producers exist in the whole workspace**, both inside the defining module (measured, not
asserted — `grep -rn 'SeniorDeductionSubtotal {'` over `crates/`): the vouched accessor
(`schedule_1a.rs:1394`) and the deliberate forge (`schedule_1a.rs:1492`). No `Deserialize`, no
`Default`, no `From`; derives unchanged at `Debug, Clone, Copy, PartialEq, Eq`.

**The cost, stated.** The variant now carries an amount that is never printed (line 1a is the printed
box, and it is what `printed()` rounds). `printed()` passes the subtotal through unchanged with the
reason written beside it. That redundancy is the price of the guarantee and it is one-directional: a
figure cannot lose its line number, only be discarded with it.

---

## 2. What happened to each external construction site

★ **Refuted premise — there are FIVE, not four.** Part 1's table omits
`crates/btctax-forms/tests/f6251_obbba.rs:1129`, which is the single most important one: it is the
forging site, the only place in the workspace that needs a *mismatched* value. The re-verification
report names it (it lists 759, 965 **and 1129**); the ledger's table dropped it on the way. Measured:
`grep -rn 'Form6251Line1::Y2025'` returns five construction sites outside `btctax-core/src`, and the
`E0639` sweep in §1 reported four of them before compilation stopped (xtask depends on `btctax-forms`,
so its site was never reached — the fifth, `line_coverage_check.rs:1534`, is real and was updated too).

| site | what it needed | what it got |
|---|---|---|
| `crates/xtask/src/line_coverage_check.rs:1534` (M-1 kill) | **legitimate vouched** | `Schedule1A::senior_deduction_subtotal(None)` — the accessor, so this instrument can no longer be pointed at a line no revision printed. It used to name `SENIOR_DEDUCTION_SUBTOTAL_LINE`; it now cannot cite anything else at all. |
| `crates/btctax-forms/tests/f6251_fill.rs:170` (TY2025 shape refuses the TY2024 map) | **legitimate vouched** | a new local `vouched_senior_subtotal(amount)` that builds a real `Schedule1aPartV { line37: Some(amount) }` and reads the accessor — so the figure genuinely *is* what that schedule's line 37 prints |
| `crates/btctax-forms/tests/f6251_obbba.rs:759` (`part_i_only_2025`) | **legitimate vouched** | same helper, `$6,000` |
| `crates/btctax-forms/tests/f6251_obbba.rs:965` (`part_iii_routed_2025`) | **legitimate vouched** | same helper |
| `crates/btctax-forms/tests/f6251_obbba.rs:1129` (**the emitter-level kill**) | **deliberately forged** — a figure vouched for the *other* revision's line | `schedule_1a::testonly::forge_senior_deduction_subtotal_vouched_by_no_schedule(dec!(6000), other)`, where `other` is still **derived** from the revision's own printed line (37 ⇄ 43), never typed |

Two in-core sites moved with them: `line_coverage.rs::all()`'s own instance (now the accessor, so the
selected 1a sentence is chosen by a vouched number) and `return_1040.rs:12055`'s expected value, which
now reuses the very subtotal the rule carried in — so the assertion additionally pins that
`compute_6251` passes provenance through unaltered.

No legitimate caller was handed the forge. Exactly one call site in the workspace forges, and it is the
kill.

---

## 3. Kills — red, then green

### 3.1 Production forging is impossible (three directions, pasted in §1)

`E0559` on the old field name from outside the crate; `E0451` on a direct forge from outside the crate;
**`E0451` on a direct forge from inside it** (`return_1040.rs`) — the direction that separates this
mechanism from the one the brief named first. All three restored from `cp` backups.

### 3.2 The existing kills still red when the join is neutralised

Mutation: `if printed != read_by_the_computation` → `if false && printed != …` in
`f6251_revision.rs::schedule_1a_line_agreeing_with`.

```
FAIL btctax-forms f6251_revision::tests::a_revision_citing_line_43_refuses_a_figure_read_off_line_37
  a revision printing "Schedule 1-A line 43" ACCEPTED a figure read off line 37 — that is modified AGI
  in the senior deduction's slot and the AMT base is overstated by roughly the whole AGI: 43
FAIL btctax-forms::f6251_obbba the_fill_refuses_a_figure_read_off_a_schedule_1a_line_this_revision_does_not_cite
  f6251/2025: this form cites Schedule 1-A line 37 and the figure was read off line 43 — the fill
  SUCCEEDED, so the AMT base is taken from the wrong line of another revision's schedule and nothing on
  the printed page shows it
 Summary 2 tests run: 0 passed, 2 failed, 396 skipped
```

Green after restore. ★ **A brief premise refined:** the round's named central kill,
`a_revision_citing_line_43_refuses_a_figure_read_off_line_37`, does **not** construct a
`Form6251Line1` at all — it calls the join directly with bare `37`/`43` on a hand-built `ObbbaRevision`,
so this fold could not have broken it either way. The test that actually needed the mismatch, and
therefore needed the forge, is the *integration* kill
`the_fill_refuses_a_figure_read_off_a_schedule_1a_line_this_revision_does_not_cite`
(`f6251_obbba.rs:1129`) — the site missing from Part 1's table. Both are shown red above, so the
constraint is satisfied on the test the brief meant as well as the one it named.

### 3.3 The forge route cannot be mistaken for production — and that is executable

New: **`crates/xtask/src/forge_reach_check.rs`** (291 lines, `#[cfg(test)]`, so `make gate` runs it —
FR-122's lesson: a checker `make gate` does not run is another instrument nobody runs). It reads the
**production half** of every `.rs` file under `crates/` — reusing `r15_stop_list::production_source`,
which strips comments and *skips and resumes past* each `#[cfg(test)]` item rather than truncating at
the first (a blindness that module measured rather than reasoned about) — and asserts two things:

1. **no call outside a `tests/` directory.** Lawful shapes are recognised by **shape, not path**: a line
   containing `fn <symbol>` (the declaration has to live somewhere) or a file under `tests/`. A hand-
   written allow-list of filenames would have been the FR-99 shape with a laundering path attached.
2. **at least one kill still calls it.** A forge with no caller is a public hole kept alive by habit; the
   honest response is deletion, and this says so rather than letting it sit.

The symbol is **assembled from two halves** in that module, for the reason `r15_stop_list` gives for its
own `serde_json` needle: a checker whose own source contains the thing it forbids would have to excuse
itself by path.

**Watched red, direction 1** — planted a call in `compute_6251`, a production seam:

```
thread 'forge_reach_check::tests::the_provenance_forge_is_reachable_only_from_a_kill_test' panicked at
crates/xtask/src/forge_reach_check.rs:186:9:
  the Schedule 1-A provenance forge is called from shipped code. A figure paired with a line number no
  revision printed overstates the AMT base by ≈MAGI, and every other instrument stays green because
  they all take the figure as INPUT. Production obtains a `SeniorDeductionSubtotal` from
  `Schedule1A::senior_deduction_subtotal`:
  ["crates/btctax-core/src/tax/form6251.rs:491: let _planted = crate::tax::schedule_1a::testonly::forge_senior_deduction_subtotal_vouched_by_no_schedule(Usd::ZERO, 43);"]
```

**Watched red, direction 2** — replaced the kill's forge call with the lawful vouched helper, i.e. the
kill quietly stopping to forge:

```
  no kill test calls the forge any more. Either the join's kill was deleted — in which case the
  guarantee is unfalsifiable and harness B1 is violated — or a real second Schedule 1-A revision now
  supplies the mismatch, in which case DELETE the forge rather than leaving a public route to an
  unvouched line number with no caller.
```

Both restored; green. A second test (`a_production_call_to_the_forge_reds_and_its_near_misses_do_not`)
pins **four near misses** as well as two plants, because a check that reds on everything is deleted by
the next person who trips it: the declaration itself, a doc comment *naming* the forge, a call inside a
`#[cfg(test)]` item in a `src/` file, and the kill's own `tests/` call site. It also asserts the floor
counts a forge call and **not** the lawful helper — otherwise the floor would be satisfied by a test
that never forges.

**So, in ascending order of what it is worth: the name (`testonly::forge_…_vouched_by_no_schedule`),
`#[doc(hidden)]`, the module boundary, and a check watched red on a planted production call.** The
answer to *"which test reds when this guard is removed?"* is
`forge_reach_check::tests::the_provenance_forge_is_reachable_only_from_a_kill_test`.

★ **Its blind spot, stated in its own header rather than implied:** a call from a `#[cfg(test)] mod
tests` inside a `src/` file is skipped (it is test code) and is therefore *not counted* by the floor
either — so moving the kill there reds the floor and forces a deliberate widening. That is the direction
to fail in, and it is load-bearing for FR-123 below.

---

## 4. Premises checked; two refuted

**Part 1's reading of `#[non_exhaustive]` holds exactly as stated** — on the enum it forces a wildcard
arm and leaves existing variants constructible; on the variant it blocks construction (`E0639`). Both
measured, §1. The five-site grep, the private fields, the single production caller and the join's call
site all reproduced.

Two refutations, both of the ledger rather than of the report:

1. **★ The external construction sites are five, not four.** `f6251_obbba.rs:1129` is missing from
   Part 1's table — the forging site, and the one whose treatment the brief spent a whole section on.
   The re-verification report lists it; the ledger's table does not.
2. **★ The brief's named central kill is not the test that needed the forge.**
   `a_revision_citing_line_43_refuses_a_figure_read_off_line_37` calls the join with bare integers and
   never touches `Form6251Line1`. The constraint the brief was protecting is real, but it lives on the
   integration kill at the site the table dropped — i.e. the two refutations are the same omission seen
   twice.

Neither changes the finding or the fix; both are recorded because a brief that names the wrong test is
how a fold satisfies a constraint on paper and breaks it in fact.

**One thing found that neither report mentions.** Adding the forge made **two committed doc comments
false** — `Schedule1A::senior_deduction_subtotal`'s *"This is the ONLY constructor … outside this
module"* and `SeniorDeductionSubtotal`'s *"there is no public constructor, so the only way to obtain one
is …"*. Both now say **only lawful way** and name the exception and its guard. This is the class the
fold being repaired was itself about (a comment that stopped being true after a later, unrelated edit),
and it would have been introduced *by the fix for it*. Also corrected: `f6251_revision.rs`'s module doc
claimed the emitter's number *"can only be obtained from the schedule revision that printed it"* —
aspirational when written, since the variant unpacked it to a `u32`; it is true now, and says when it
became true.

---

## 5. Residue

| item | severity | owning phase |
|---|---|---|
| **FR-123** — `ObbbaRevision::schedule_1a_line_agreeing_with` still takes a bare `u32`, so the *consumer's* parameter is forgeable even though its one caller now hands it a vouched number | Minor | **the TY2026 port** (the second Schedule 1-A revision is what makes the fix free) |

Filed with its full reasoning in `FOLLOWUPS.md` (numbering checked: FR-120…FR-123 each appear exactly
once). **Why it was not folded rather than filed:** typing that parameter would force the join's pure
text-parsing unit test — which lives in `btctax-forms/src` — to call core's test-only forge from a `src/`
file, which `forge_reach_check` treats as a *stated blind spot* rather than as a counted call site. The
fix available today would weaken the guard installed in §3.3; the fix available after TY2026's schedule
is transcribed costs nothing, because the kill can then pair two real revisions and the forge goes away
entirely. It is not armed today: `fill_form_6251_obbba_with_map` is the only caller and it reads the
vouched accessor.

Nothing else was widened. **Not touched:** any `FullReturnParams`, the `2026` arm, the TY2025/TY2026
fail-closed gates, `f1040s1a/2025` staying `Unwired`, the Schedule 1-A emitter, the previous fold's six
items, the rest of the branch.

---

## 6. Files changed (11 modified, 1 added; +240 / −46 tracked)

| file | what |
|---|---|
| `crates/btctax-core/src/tax/schedule_1a.rs` | two now-false doc claims corrected; `#[doc(hidden)] pub mod testonly` with the single forge |
| `crates/btctax-core/src/tax/form6251.rs` | **`Y2025` carries `SeniorDeductionSubtotal`**; the doc records why the type and not the enum holds it; `printed()`; `compute_6251` stops unpacking; one test |
| `crates/btctax-core/src/tax/line_coverage.rs` | `cover_form6251line1` selects its 1a sentence off the vouched type; `all()`'s instance built through the accessor |
| `crates/btctax-core/src/tax/return_1040.rs` | two tests; the expected value now reuses the rule's own subtotal (so it pins pass-through too) |
| `crates/btctax-forms/src/form6251.rs` | the join is handed `senior_deduction.schedule_1a_line()`, not a field the caller could set |
| `crates/btctax-forms/src/f6251_revision.rs` | module doc: the sentence that only became true today, and when |
| `crates/btctax-forms/tests/f6251_fill.rs` | `vouched_senior_subtotal` helper; the TY2025-shape fixture |
| `crates/btctax-forms/tests/f6251_obbba.rs` | `vouched_senior_subtotal` helper; two legitimate fixtures; **the one forge call, in the kill** |
| `crates/xtask/src/line_coverage_check.rs` | the M-1 kill's instance, built through the accessor |
| `crates/xtask/src/main.rs` | `#[cfg(test)] mod forge_reach_check;` |
| `FOLLOWUPS.md` | FR-123 |
| **`crates/xtask/src/forge_reach_check.rs`** *(new, 291 lines)* | the forge-reach guard, both directions, with its planted-defect kills and its stated blind spot |

**Two tests added:** `forge_reach_check::tests::the_provenance_forge_is_reachable_only_from_a_kill_test`,
`forge_reach_check::tests::a_production_call_to_the_forge_reds_and_its_near_misses_do_not`
(3594 → 3596). No test was removed or weakened.
