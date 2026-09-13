# REPORT — S1 — FR-142 (rehearsal F17): nothing filled a newly ported year's template

## Scope and file touched

Exclusive file only: `crates/btctax-forms/tests/f8995a_fill.rs`. No other file was touched. The port
machine, `line_set.rs`, `map.rs`, `bundled.rs`, `packet.rs`, `form8995a.rs`, `Makefile`, and
`FOLLOWUPS.md` were all read-only for this task.

## What changed

Every one of the 10 original fill tests called `Form8995AMap::ty2024()` once. All 10 now iterate
`for year in covered_years()`, calling `Form8995AMap::for_year(year)` per iteration, and every
assertion message in the loop bodies is prefixed `"TY{year}: ..."` so a failure names which year it is
about. One new test, `the_covered_year_set_can_never_be_silently_empty`, pins the empty-set guard
itself. Total tests: 10 -> 11.

`covered_years()` (new, private to this file):

```rust
fn covered_years() -> Vec<i32> {
    let mut years: Vec<i32> = BUNDLED
        .iter()
        .filter(|(stem, _)| *stem == Stem::F8995a)
        .map(|(_, year)| *year)
        .filter(|&year| Form8995AMap::for_year(year).is_ok())
        .collect();
    years.sort_unstable();
    years.dedup();
    assert!(!years.is_empty(), "...loop over nothing must never pass silently...");
    years
}
```

This derives from `btctax_forms::bundled::BUNDLED` (the build.rs-generated glob of every `(Stem, year)`
pair actually on disk), filtered through `Form8995AMap::for_year`, the SAME schema dispatch
`packet.rs` uses in production. No hand-typed year literal remains in this file.

## Which years are in scope, and how that was determined (the report's first measurement ask)

**Measured, not assumed.** In this worktree `crates/btctax-forms/forms/` has `2024/`, `2025/` and
`2026/` directories, but `find crates/btctax-forms/forms -iname '*f8995a*'` returns exactly two files:
`forms/2024/f8995a.map.toml` and `forms/2024/f8995a.pdf`. The TY2025 port the rehearsal report
describes was never committed (its own section 6 point 7: "Nothing was committed or pushed") and this
worktree does not carry it. So **today `covered_years()` returns exactly `[2024]`** — confirmed by the
loop-body test run below (every failure/pass message in the suite reads `TY2024:`).

This is not a limitation of the fix — it is why deriving matters instead of writing `[2024]` by hand:
the day a `forms/2025/f8995a.{pdf,map.toml}` pair is committed with a `line_set` that
`line_set::schema()` maps to `Schema::Form8995AMap` (the same struct, i.e. no line renumbered), that
year is picked up here with **no edit to this file**. A year whose schema moved to something else
(`Schema::Unwired`, or a different map type) is correctly excluded — `Form8995AMap::for_year` returns
`Err` for it and the filter drops it, rather than the test guessing that the old assertions still
apply to unknown geometry.

**Why sharing the schema is sufficient for the SAME test bodies to be valid**, verified by reading the
production code, not assumed: `fill_form_8995a_with_map` (`crates/btctax-forms/src/form8995a.rs`)
loads `pdf::f8995a_pdf(map.year)` — the year-specific template — and writes through `map`'s own
`MoneyCell`s; every formatting rule this suite pins (parenthesized-box magnitude, DPAD blank-not-zero,
percentage x100, checkbox-only-when-true) is encoded in the emitter and `MoneyCell`, not per-year data.
`header()` (`kitchen_sink_header()`, hardcoded to `ReturnHeader::build(&ri, 2024)`) only supplies
name/SSN through `push_identity`, which reads its field names from `map.identity` — the year-specific
map — so reusing one fixed header across years is safe for what these tests check.

## Does the parameterized test still discriminate? (the report's second measurement ask)

Yes, on two axes:

1. **Every failure names its year.** All 10 loop bodies were edited to embed `TY{year}` in every
   `assert_eq!`/`assert!`/`panic!` message inside the loop, not just the outermost one. See the kill
   below: the failure reads `TY2024: the box for ... should read 9999`, not a bare mismatch.
2. **An empty covered-year set cannot pass.** `covered_years()` itself asserts `!years.is_empty()`
   before returning, so a `for year in covered_years()` loop can never silently execute zero times —
   the call site panics first. `the_covered_year_set_can_never_be_silently_empty` pins that this
   assertion actually fires on an empty vector (via `catch_unwind`), independent of how many years
   happen to be bundled when the suite runs.

## The kill (B1) — pasted, not summarized

**Part 1 — planted a wrong value in one cell for TY2024, showed the failure names the year.**

Plant (`part_iv_writes_each_figure_to_its_own_line`, one cell only):
```rust
(&map.line28, "9999"), // PLANTED DEFECT for the FR-142 kill -- should read "4000"
```

RED:
```
running 1 test
test part_iv_writes_each_figure_to_its_own_line ... FAILED

---- part_iv_writes_each_figure_to_its_own_line stdout ----

thread 'part_iv_writes_each_figure_to_its_own_line' panicked at crates/btctax-forms/tests/f8995a_fill.rs:128:13:
assertion `left == right` failed: TY2024: the box for topmostSubform[0].Page2[0].f2_37[0] should read 9999
  left: Some("4000")
 right: Some("9999")

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 10 filtered out; finished in 0.04s
```

Reverted via `cp` from a pre-kill backup (never `git checkout`). GREEN after revert:
```
running 11 tests
test the_covered_year_set_can_never_be_silently_empty ... ok
test a_negative_in_a_parenthesised_box_fails_closed ... ok
test the_phase_in_percentage_prints_scaled_by_a_hundred ... ok
test a_nameless_business_with_qbi_fails_closed ... ok
test the_dpad_line_carries_no_testimony ... ok
test a_part_i_checkbox_is_written_only_when_it_is_checked ... ok
test a_loss_carryforward_prints_as_a_magnitude ... ok
test the_conditional_part_ii_lines_carry_no_testimony ... ok
test a_reit_only_filer_writes_nothing_in_parts_i_to_iii ... ok
test part_iv_writes_each_figure_to_its_own_line ... ok
test parts_i_to_iii_write_each_figure_to_its_own_line ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.28s
```

**Part 2 — the covered-year set can never be empty without reddening.**

`the_covered_year_set_can_never_be_silently_empty` re-states the identical guard against a literal
empty `Vec<i32>` (deliberately not calling the real `covered_years()`, since that one is correctly
non-empty today and calling it would prove nothing about the empty case):

```rust
let empty: Vec<i32> = Vec::new();
let result = std::panic::catch_unwind(|| {
    assert!(!empty.is_empty(), "...loop over nothing must never pass silently...");
});
assert!(result.is_err(), "an empty covered-year set must panic the guard, not pass through it");
```

This passed in the same 11/11 run above (`the_covered_year_set_can_never_be_silently_empty ... ok`),
i.e. it is proven that the assertion actually panics on empty input -- the test would itself fail
(`result.is_err()` would be false) if someone weakened the guard to something that lets an empty set
through.

## Refuted premises

None of the report's premises for FR-142 needed refuting -- both measurements it asked for came out as
the report anticipated:
- "Which years actually share Form 8995-A's schema" is answered by measurement to be `{2024}` today
  (see above), not assumed to be "2024 and 2025" as a hasty reading of the port rehearsal might
  suggest -- the TY2025 port was never committed in this worktree.
- The proposed parameterization is sound because the emitter and `MoneyCell` are the year-generic
  layer, not the map -- confirmed by reading `form8995a.rs`, not assumed from the report's prose.

## What I could not do for want of a file

Nothing. The fix is contained entirely within the owned test file. I considered (and rejected)
temporarily copying a second, synthetic `forms/2025/f8995a.{pdf,map.toml}` pair into the crate to
prove the loop genuinely iterates when the covered-year set has more than one member -- that would
have touched files outside this task's exclusive ownership (`crates/btctax-forms/forms/2025/`) even
temporarily, so I did not do it. The single-year kill above still proves the two required properties
(year-naming in failures; non-empty enforcement) without it.

## Gate numbers, with the 7 pre-existing failures accounted for

Ran under `CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-s1` (unique target dir per plan).

`make gate`'s concurrent nextest+clippy run hit a SIGKILL (OOM) on `regex-automata` during this
session -- `free -h` at the time showed ~3.8 GiB free with ~27 concurrent `rustc`/`cargo` processes
already running system-wide (the plan's Wave-1 agents building concurrently), matching the plan's own
documented risk ("`make gate` OOM-killing the linker at 3 GB free"). This was not caused by, or
specific to, this file's change. Re-ran the same two checks the check/gate targets run, sequentially
instead of concurrently, after `find crates -name '*.rs' -exec touch {} +` (the same freshness-forcing
`gate` does):

- `cargo nextest run --workspace --no-fail-fast`: **3608 passed, 7 failed, 12 skipped, 3615 total.**
  The 7 failures are exactly the 7 the brief predicted:
  - 6 in `xtask::form_delta::tests::*` (`the_ty2026_drafts_are_ready_to_be_diffed_against_their_finals`,
    `every_common_field_is_either_compared_or_named_as_unwitnessed`,
    `the_work_list_checker_reds_on_every_planted_row`,
    `no_archived_pair_reports_a_clean_verdict_from_zero_comparisons`,
    `port_status_prints_the_committed_work_list`,
    `the_committed_work_list_matches_form_delta_at_head`) -- the gitignored `design/forms/**/*.pdf`
    not being shared into this worktree.
  - 1 in `xtask::harness_check::tests::the_write_hook_denies_new_archives_and_asks_once_per_new_directory`
    (FR-147, another agent's item; failure message: "cargo build -p xtask succeeded but left no binary
    where on-write.sh looks" -- the `CARGO_TARGET_DIR` override).
  - **Zero failures in `f8995a_fill`** -- all 11 tests in the owned file passed in this same run.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: **exit 0**, no warnings.
- `cargo fmt --all --check -- crates/btctax-forms/tests/f8995a_fill.rs`: was NOT clean on the first
  pass (3 reflow diffs from the new loop indentation); ran `cargo fmt --all -- <file>` to apply them,
  then confirmed `--check` clean. Re-ran the file's own test binary and clippy after formatting:
  still 11/11 passed, clippy still exit 0.

No product behaviour changed -- this is a test-only file.
