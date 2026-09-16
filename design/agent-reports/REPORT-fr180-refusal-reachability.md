# REPORT — FR-180: can a filer reach each refusal, and does each one fire on the state it names?

**Agent:** FR-180 refusal reachability. **Date:** 2026-09-15.
**Tree:** worktree `.claude/worktrees/agent-a562eeada646a6ea8`, base `cc29bb30e` (47 commits behind
`main`). **NOT COMMITTED** — everything below is uncommitted work in that worktree.

**Headline: nothing is unreachable, nothing fires on the wrong state — and the brief's eleven is wrong
in three places, because the instrument that produced it is broken.** Four refusals genuinely had no
test anywhere (all four §G-28/B1b §199A sub-schedule refusals) and now have one each, mutation-verified.
Seven were *already observed firing* by a derived property test that names no variant at all, which is
exactly what a naming census cannot see. Two of the brief's eleven (`FamilyLeaveBenefits`,
`HsaExcessEmployerContributions`) were never gaps: they have fixtures asserted firing, and were reported
as untested by a brace counter that reads string literals. Two variants the brief's list *omits*
(`CooperativePatron`, `IncomeExclusionUnanswered`) were real residue.

---

## 1. The count, re-measured — and why the brief's differs

The controller's facts were to be taken as given, but the brief's own framing invited a re-measurement,
and the sibling-task lesson (*reproduce before fixing*) applies. Measured at my base and at `main`, with
a test/production splitter that skips string and char literals and strips comments:

| tree | declared | named in test ctx | residue |
|---|---|---|---|
| my base `cc29bb30e` | **135** | 124 | **11** |
| `main` | **139** | 127 | **12** |

The brief reported 139 / 128 / 11. The composition differences have two mechanical causes.

### (a) `production_mask` / `production_source` brace-counts through string and char literals

`crates/xtask/src/r15_stop_list.rs`'s counter increments on every `{` and decrements on every `}`,
including those inside strings and `b'…'` char literals. Its own doc admits it:

> *"★ Line-based and brace-counted, so a `{` inside a string literal inside a test module could end the
> skip early. That errs toward scanning MORE, which is the fail-closed direction here."*

That is true for R15, whose question is *"does a forbidden idiom appear in shipped code"*. It is
**exactly backwards** for FR-180's question, because a test region misread as production makes a
**tested** variant look **untested**.

Measured, not reasoned. `return_refuse.rs`'s `#[cfg(test)] mod param_free_tier` begins at line 11105 and
contains `rest.find("\n}\n")` and `b'}' => depth -= 1`. Running the shipped algorithm over the file:

```
first #[cfg(test)] at line 305 ; total lines 12757
production runs after the first cfg(test):
  lines 11133..12122   (990 lines)
```

The whole of `param_free_tier` after line 11132 — **990 lines** — is classified as *production*. That
module holds `add("FamilyLeaveBenefits", …)` and `add("HsaExcessEmployerContributions", …)`, two entries
of the `param_free_fixtures()` table whose members are driven through **both** entry points and asserted
firing by `every_param_free_rule_is_censused_from_the_source_and_fires_on_both_paths`. Both variants are
therefore *observed firing* and were reported as untested. **Two of the eleven are artifacts of the
measuring instrument, not gaps in the suite.**

Same measurement with a literal-aware mask: 127 → **129** named at `main`; residue 12 → **10**.

### (b) a comment-only mention satisfies a grep — and one variant lives entirely in prose

`IncomeExclusionUnanswered` has exactly two test-context occurrences in the whole workspace, both
comments, in `crates/btctax-tui-edit/src/edit/form.rs:4745` and `:4760`. The nearby test
(`the_entry_screens_completeness_line_counts_a_year_scoped_question`) does drive
`ri.has_income_exclusion = None` — but it asserts on the *rendered completeness line*, never on the
refusal. Strip comments and it joins the residue. It is **not** in the brief's eleven.

### (c) `CooperativePatron` is residue and is not in the brief's eleven

The `Some(true)` sibling of `CooperativePatronUnanswered` — "a patron must reduce the QBI component on
Schedule D (Form 8995-A)" — is named in no test either. It is the same §G-28/B1b block, it shipped on
the same day, and omitting it would have left the more dangerous of the pair (it guards an **overstated
deduction**) untested while its sibling got a test.

### The corrected residue at my base — 11 variants, a different 11

```
CooperativePatron                        QbiCarryforwardNeedsSchedule8995AC
CooperativePatronUnanswered              SstbInPhaseInRange
FilerTinUnanswered                       StateRefundWithout1099gUnanswered
FilingStatusUnconfirmed                  WagesWithoutW2Unanswered
HomeSaleGateUnanswered
IncomeExclusionUnanswered
InterestOrDividendsWithout1099Unanswered
```

Set difference from the brief: **−`FamilyLeaveBenefits`, −`HsaExcessEmployerContributions`,
+`CooperativePatron`, +`IncomeExclusionUnanswered`**. Eight are answered-ness guards, not seven.

---

## 2. ★★★ The central FR-180 finding: the premise is inverted for seven of them

The ledger asks *"the census verifies a refusal is RECORDED, never that it FIRES."* For seven of the
eleven it is the other way round. **The refusal fires, is watched firing, and the census is what is
blind.**

`crates/btctax-core/src/tax/return_refuse.rs::every_live_unanswered_declaration_refuses_with_its_own_reason`
(line 7703) walks `FORM_QUESTIONS`, builds `scenario_for(q.id)` — a per-question liveness fixture —
answers every *other* live question, and asserts:

```rust
assert_eq!(reason(&r), Some(q.unanswered.clone()), "blank {:?} must refuse with its own unanswered reason", q.id);
```

I instrumented that exact loop and printed its per-entry outcome. **66 of 67 registry entries are driven
and every single one returned the reason its entry declares**, including all seven residue guards:

```
FR180-PROBE driven=66 skipped=1
FR180-PROBE SKIPPED DocForm1098 -> DocumentCensusUnanswered
FR180-PROBE DRIVEN FilingStatusConfirmed -> want FilingStatusUnconfirmed got Some("FilingStatusUnconfirmed")
FR180-PROBE DRIVEN WagesWithoutW2Question -> want WagesWithoutW2Unanswered got Some("WagesWithoutW2Unanswered")
FR180-PROBE DRIVEN InterestOrDividendsWithout1099 -> want InterestOrDividendsWithout1099Unanswered got Some("InterestOrDividendsWithout1099Unanswered")
FR180-PROBE DRIVEN StateRefundWithout1099g -> want StateRefundWithout1099gUnanswered got Some("StateRefundWithout1099gUnanswered")
FR180-PROBE DRIVEN FilerTinIssuedByDueDate -> want FilerTinUnanswered got Some("FilerTinUnanswered")
FR180-PROBE DRIVEN HasIncomeExclusion -> want IncomeExclusionUnanswered got Some("IncomeExclusionUnanswered")
FR180-PROBE DRIVEN SoldMainHome -> want HomeSaleGateUnanswered got Some("HomeSaleGateUnanswered")
FR180-PROBE DRIVEN HomeSaleTest1OwnedAndLived -> want HomeSaleGateUnanswered got Some("HomeSaleGateUnanswered")
FR180-PROBE DRIVEN HomeSaleTest2NoRecentExclusion -> want HomeSaleGateUnanswered got Some("HomeSaleGateUnanswered")
FR180-PROBE DRIVEN HomeSaleCanExcludeAllGain -> want HomeSaleGateUnanswered got Some("HomeSaleGateUnanswered")
```

The one skip is taken under the loop's own *derived* T9 exemption — a census row whose screen does not
exist yet — and its reason `DocumentCensusUnanswered` is driven by twenty sibling `Doc*` entries. The
probe was reverted after measuring; `return_refuse.rs` is unmodified in the diff.

**The structural point, and it is the durable half of FR-180.** The assertion is
`Some(q.unanswered.clone())`. No variant name appears in the source, because the test is *derived* —
precisely what `CLAUDE.md`'s highest-yield rule demands. **A census keyed on naming will therefore always
report the repo's best-built guards as its worst.** A naive FR-180 census would have shipped with a
permanent seven-row false residue, each row an exemption earned by nothing. That is the finding the
instrument had to be built around, not a detail of its implementation.

---

## 3. The eleven verdicts (plus the two the brief listed that are not gaps)

All thirteen are **FIRES**. **0 UNREACHABLE. 0 FIRES-ON-THE-WRONG-STATE.** No refusal was weakened and
no variant was deleted.

| variant | verdict | which test reds when this guard is reverted? |
|---|---|---|
| `CooperativePatronUnanswered` | **FIRES** (new) | `return_1040.rs::an_unanswered_cooperative_patron_question_refuses_below_the_threshold` — killed by making the `None` arm fall through |
| `CooperativePatron` | **FIRES** (new) | `return_1040.rs::a_cooperative_patron_refuses_on_the_unfilled_schedule_d` — killed by making the `Some(true)` arm fall through |
| `SstbInPhaseInRange` | **FIRES** (new) | `return_1040.rs::an_sstb_inside_the_phase_in_range_refuses_and_above_the_range_files` — killed by widening the guard to `regime != AtOrBelowThreshold` |
| `QbiCarryforwardNeedsSchedule8995AC` | **FIRES** (new) | `return_1040.rs::a_qbi_loss_carryforward_above_the_threshold_refuses_on_schedule_c_of_form_8995a` — killed by deleting the `if` block |
| `FilerTinUnanswered` | **FIRES** (already) | `return_refuse.rs::every_live_unanswered_declaration_refuses_with_its_own_reason`, entry `FilerTinIssuedByDueDate` |
| `FilingStatusUnconfirmed` | **FIRES** (already) | same test, entry `FilingStatusConfirmed` |
| `HomeSaleGateUnanswered` | **FIRES** (already) | same test, all **four** payload questions driven |
| `IncomeExclusionUnanswered` | **FIRES** (already) | same test, entry `HasIncomeExclusion` |
| `InterestOrDividendsWithout1099Unanswered` | **FIRES** (already) | same test, entry `InterestOrDividendsWithout1099` |
| `StateRefundWithout1099gUnanswered` | **FIRES** (already) | same test, entry `StateRefundWithout1099g` |
| `WagesWithoutW2Unanswered` | **FIRES** (already) | same test, entry `WagesWithoutW2Question` |
| `FamilyLeaveBenefits` | **FIRES — NOT A GAP** | `return_refuse.rs::every_param_free_rule_is_censused_from_the_source_and_fires_on_both_paths`, fixture `add("FamilyLeaveBenefits", …)` at line 11711 |
| `HsaExcessEmployerContributions` | **FIRES — NOT A GAP** | same test, fixture at line 11828 |

### Reachability notes worth keeping

- `CooperativePatronUnanswered` / `CooperativePatron` are reachable at **any** income — the fixtures are
  deliberately *below* the §199A(e)(2) threshold, because "at any income" is the claim the refusal's own
  comment makes and an above-threshold fixture would not test it.
- `SstbInPhaseInRange` needs taxable-income-before-QBI strictly inside the phase-in range. Measured:
  mining $250,000 Single gives `ti_before_qbi = 221,134`, inside (191,950, 241,950].
- `QbiCarryforwardNeedsSchedule8995AC` is reachable with **no Schedule C at all** — the guard sits
  outside the `if let Some(c) = ri.schedule_c` block and `has_qbi` is true on the carryforward alone. The
  test uses a $250,000 wage filer with `qbi.qbi_carryforward_in = 4,000` and asserts `business_qbi == 0`,
  so the carryforward is provably the only thing requiring the form.
- **The exit is real, and that was checked rather than assumed.** `CooperativePatronUnanswered` prints
  *"run `btctax income answer`"*. The brief cites two recorded cases of a refusal naming a cure that
  dead-ends (`btctax set-pii`, a verb that never existed; the `QbiAboveThreshold` anchor falsehood). I
  measured this one through the registry itself: `SkippableId::ScheduleCIsCooperativePatron` is live on
  exactly the refusing return (`live: |ri| ri.schedule_c.is_some()`), and driving
  `(sk.set_bool)(…, false)` clears the refusal. That is asserted inside the test, not read off
  `attribute.rs`'s anchor table.

---

## 4. The instrument — `crates/xtask/src/refusal_test_census.rs` (new, 661 lines)

**What it asserts.** Every variant `census_join::variant_paths(src, "RefuseReason")` derives from the
enum is covered, and the residue equals `UNTESTED_PIN` by **set equality** (so it reds on growth *and*
on a stale row). `UNTESTED_PIN` is **empty** — the FR-180 result, reached with no exemptions.

**Reused, not reimplemented** (the brief asked which): `crate::census_join::variant_paths` for the enum
derivation and `crate::blockers::workspace_rust()` for the file walk. `blockers::refusal_census` does not
exist at my base (it arrived in the 47 newer commits); `variant_paths` is the same derivation it wraps.

**Two coverage limbs, both derived.**

1. **NAMED** — the variant appears in a non-comment line of a test context: any file under a `tests/`
   directory, or the `#[cfg(test)]` regions of a `src/` file, split by a **literal-aware** mask
   (`test_context`) rather than by `r15_stop_list::production_source`, for the measured reason in §1(a).
2. **REGISTRY-DRIVEN** — it is the `unanswered` reason of a `FORM_QUESTIONS` entry, read off the registry
   at runtime, **while `return_refuse.rs` still carries the property test by name**. Delete the test and
   the credit vanishes entirely, so it can never become a silent exemption.

`SKIPPABLE_QUESTIONS` is deliberately **not** a limb: it has exactly one class-(A) entry
(`HohMaritalBasisUnanswered`, asserted), and there is no derived per-entry property test for it, so a
credit would be an exemption earned by nothing. A second class-(A) skippable reds the test and forces a
decision.

**The blind spot is in the output, not only the doc.** `named in a test` is not `observed firing`. The
census measures and **prints** the variants named with no `assert`-family macro within eight test lines,
and deliberately does **not** gate on it — as a gate it would have produced six false rows, all six
being `param_free_fixtures` table entries that a derived loop asserts on elsewhere. Gating it would have
repeated §1(a)'s mistake in a new place.

### B1 — the kills, all of them watched red

| plant | test | what it proves |
|---|---|---|
| a new variant inserted into the **enum source**, no test anywhere | `a_new_untested_variant_is_refused_and_the_message_forbids_deleting_it` | growth reds, and the finding text forbids the census-greening edit |
| a pin row for a variant that **is** covered | `a_stale_pin_row_is_refused` | shrink-without-updating-the-pin reds |
| `"\n}\n"` inside a `#[cfg(test)]` module, variant named after it | `a_brace_inside_a_string_does_not_end_a_test_module` | the literal-aware mask still sees the test — and the shipped counter is asserted to still leak, so the kill discriminates |
| the variant named only in `///` and `//` comments | `a_comment_mention_is_not_a_test` | prose does not satisfy the census |
| the registry property test renamed away | `the_registry_limb_dies_with_its_test` | all four named guards re-enter the residue and the census reds |
| any declared variant typed in this module's own test context | `this_census_credits_nothing_by_naming_it` | FR-235 made structural — see below |

**Per FR-235 the plants are not in the checker's own vocabulary**, and enforcing that found two real
defects in my own first draft:

- `a_new_untested_variant…` did **not** red: the plant name `PlantedRefusalNobodyTests` was typed as a
  string literal in this module's test region, so the census credited its own plant.
- `the_registry_limb_dies_with_its_test` passed on three of four guards for the same reason — the names
  it asserted on were the names it had typed.

Both are fixed by assembling every variant name at runtime (`name("FilerTin", "Unanswered")`), the same
device `r15_stop_list::serde_json_reflection` uses on its own needle.
`this_census_credits_nothing_by_naming_it` is the standing guard, and it immediately caught a third:
`name("WagesWithoutW2", "Unanswered")` types `WagesWithoutW2`, which **is itself a declared variant** —
split to `name("WagesWithout", "W2Unanswered")`.

### Pasted red output — growth

```
thread 'refusal_test_census::tests::a_new_untested_variant_is_refused_and_the_message_forbids_deleting_it'
panicked at crates/xtask/src/refusal_test_census.rs:516:9:
assertion `left == right` failed: TEMP capture: ["PlantedRefusalNobodyTests: declared as a refusal and
covered by NO test — no name in any test context, and not an `unanswered` reason the registry property
test drives. Write the test that drives a return to the state it names; if no input can reach it, say so
in UNTESTED_PIN with the reason, and do NOT delete the variant to make this census green"]
  left: 1
 right: 0
```

### Pasted red output — shrink / stale pin, with the blind spot in the message

```
thread 'refusal_test_census::tests::every_declared_refusal_is_covered_by_a_test'
panicked at crates/xtask/src/refusal_test_census.rs:451:9:
refusal test census over 135 declared variants — 1 finding(s):
  DependentStatusUnanswered: pinned in UNTESTED_PIN as untested, but a test now covers it — delete the
  row. A stale exemption is an excuse earned by nothing

★ BLIND SPOT: `named in a test` is not `observed firing`. Named with no assertion within eight test
lines: ["FamilyLeaveBenefits", "Form8615AgeSupportUnanswered", "HsaExcessEmployerContributions",
"LiquidationDistributionNotComputed", "ScheduleFIncomeNotModeled", "StateRefundWithout1099gContradicted"]
```

Both captures were taken with a temporary edit that has been reverted; the tests as they stand assert
these outcomes rather than print them.

---

## 5. ★ A false PASS I planted and then killed in my own new test

Worth recording, because it is the B1 argument in miniature and it was **only** found by mutating.

The first draft of `an_sstb_inside_the_phase_in_range_refuses_and_above_the_range_files` proved half (b)
— *above* the range the same answer files — with a mining-only $500,000 return. The widening mutation
(`regime == InPhaseInRange` → `regime != AtOrBelowThreshold`) left **all four** new tests green.

Measured cause: above the range §199A(d)(3) zeroes an SSTB's QBI, so `business_qbi = 0`, so `has_qbi` is
**false**, so `screen_absolute` never enters the §199A block at all.

```
FR180-P2 fmv=250000 ti_before_qbi=221134 regime=InPhaseInRange       business_qbi=235734.11 files_a_199a_form=true  refusal=Some("SstbInPhaseInRange")
FR180-P2 fmv=500000 ti_before_qbi=467786 regime=AboveThePhaseInRange business_qbi=0         files_a_199a_form=false refusal=None
```

The fix adds a $5,000 §199A REIT dividend, which keeps `has_qbi` true with `business_qbi == 0` so the
`is_sstb` match is reached, plus an explicit assertion of that reachability premise — the line that would
have caught the vacuous draft. The mutation then kills:

```
thread 'tax::return_1040::tests::an_sstb_inside_the_phase_in_range_refuses_and_above_the_range_files'
panicked at crates/btctax-core/src/tax/return_1040.rs:8311:9:
assertion `left != right` failed: above the range no Schedule A (Form 8995-A) is needed — this refusal
must NOT fire
  left: Some(SstbInPhaseInRange)
 right: Some(SstbInPhaseInRange)
```

---

## 6. Gate — run serially, in the foreground

```
$ cargo nextest run --workspace
     Summary [  22.398s] 3833 tests run: 3833 passed, 12 skipped

$ cargo clippy --workspace --all-targets -- -D warnings
    Finished `dev` profile [optimized + debuginfo] target(s) in 3.48s      (no warnings)

$ cargo fmt --all --check
FMT CLEAN
```

Mutation runs, each planted and reverted one at a time (`cargo test -p btctax-core --lib`):

| mutation | result |
|---|---|
| patron `None` arm falls through | `an_unanswered_cooperative_patron_question_refuses_below_the_threshold` FAILED |
| patron `Some(true)` arm falls through | `a_cooperative_patron_refuses_on_the_unfilled_schedule_d` FAILED |
| in-range guard widened to above-threshold | `an_sstb_inside_the_phase_in_range_refuses_and_above_the_range_files` FAILED |
| carryforward `if` block deleted | `a_qbi_loss_carryforward_above_the_threshold_refuses_on_schedule_c_of_form_8995a` FAILED |

---

## 7. What I changed, and what the controller must know

```
 crates/btctax-core/src/tax/return_1040.rs | 367 +++++++++++++++++++++++  (tests only)
 crates/xtask/src/main.rs                  |   7 +
 crates/xtask/src/refusal_test_census.rs   | new, 661 lines
```

**xtask files touched, named plainly as the brief required:**

- `crates/xtask/src/refusal_test_census.rs` — **new**, my census module.
- `crates/xtask/src/main.rs` — **7 lines**: `#[cfg(test)] mod refusal_test_census;` plus its doc comment,
  following the `capital_loss_carryover_check` precedent (a checker with no operator-facing report
  belongs in the suite, where `make check` asks it on every commit). **No subcommand arm**, to keep the
  conflict surface with the 47 newer commits to one insertion point.
- **Nothing else.** `blockers.rs` untouched; `ledger_check.rs` does not exist at my base.

**No `RefuseReason` variant was added**, so `xtask blockers`' pinned census count and `xtask ledger-check`
are untouched — there is nothing for the controller to move. The B1 growth plant travels through the enum
*source text* in memory instead, which exercises the same derivation without repo churn.

**Not touched, per the brief:** `btctax-oracle-harness/**`, `design/forms/extract/**`, `legal/**`,
`tax/packet.rs`, the golden packets, existing `design/agent-reports/REPORT-*`, `FOLLOWUPS.md`.
`return_refuse.rs` is **unmodified** — the probe that produced §2's measurement was inserted, run, and
reverted from the tree.

### Two follow-ups I did not act on (not mine to file)

1. **`r15_stop_list::production_source` / `production_mask` mis-splits `return_refuse.rs`.** 990 lines of
   a test module read as production. For R15 that is fail-closed noise; for **any** consumer asking
   *"is this in a test?"* it is fail-open in the direction that hides gaps. `blockers::abort_census`
   cites `file:line` off the mask, and this is the file with the largest test modules in the repo.
   `refusal_test_census::test_context` is the literal-aware replacement, and
   `a_brace_inside_a_string_does_not_end_a_test_module` asserts the shipped one still leaks — so if that
   assertion ever reds, the shipped counter was fixed and the kill should be retired deliberately.
2. **`blockers::non_test`** (`crates/xtask/src/blockers.rs:1113`) truncates at the *first*
   `\n#[cfg(test)]\nmod tests {` — the exact truncating shape `production_source`'s own header records as
   a measured defect ("a planted `serde_json::Value` appended to the END … was reported green").
   `return_refuse.rs` has five `#[cfg(test)]` items and only one of them is `mod tests`.

---

## 8. One judgement call, stated because it is the load-bearing one

The brief asked for *"a census asserting every declared variant is named in a test context"*. I built
that, and then added the second limb, because a pure naming census is **provably wrong here**: seven of
the eleven guards are watched firing by a test that, being correctly derived, names nothing. Pinning
those seven as residue would have written seven false rows into the repo, each one indistinguishable from
a real gap to every future reader — the "false completeness" failure this codebase names as its worst
shape. The second limb is derived from the registry, dies with its test, and is measured (66/67) rather
than asserted. If the controller prefers the pure form, the honest version of it carries seven pin rows
whose reason is *"observed firing by a derived test that names no variant"* — which is the same claim,
recorded as an exemption instead of as structure.
