# Re-verification — fold of the T0–C build review (r2)

- **Reviewer:** independent read-only verifier (Claude Sonnet 5), no subagents, no edits outside this file
- **Date:** 2026-09-06
- **Repo / branch / HEAD verified:** `/scratch/code/bitcoin_tax`, `main`, `18d1332bbd14b7c70f8a85db6969effebae7be4f`
- **Tree state:** clean (`git status --short` empty) — the scoped run below measures `18d1332b` directly, no worktree needed.
- **Inputs read:** `design/agent-reports/2026-09-06-build-1099da-T0-C-review.md` (the build review, 0C/3I/3M/3N);
  `…-T0-C-review-VERIFICATION.md` (settled, 9/9 TRUE); `git show 18d1332b` (full diff, 725 lines); `git show
  d9863909 --stat` (T3, which answered I-1 before this fold).
- **Question answered:** does `18d1332b` resolve I-2, I-3, M-1, M-2, M-3, N-1, N-2, and does each kill it adds
  discriminate?

## Machine check run

Once, on the clean tree at `18d1332b`:

```
cargo nextest run --locked -p btctax-core -p btctax-cli \
  -E 'binary(kat_broker_reporting) | binary(broker_csv) | test(a_live_year_refuses_the_crypto_slice) | \
      test(a_lot_bought_on_the_venue) | test(pre_regime) | test(slice_broker)' --no-capture
```

`16 tests run: 16 passed, 1848 skipped` (`/tmp/claude-1000/-scratch-code-bitcoin-tax/fold-c-verify.txt`). All
seven named-in-brief kills are in the list and green: `the_slice_arm_refuses_only_on_a_live_year`,
`the_pre_regime_resolver_guesses_nothing_after_the_box_revision`, `the_8949_csv_is_gated_and_routed_on_a_live_year`,
`a_live_year_refuses_the_crypto_slice_before_any_byte`, `screen_absolute_runs_the_broker_screen_on_a_live_year`,
`the_printed_packet_routes_the_boxes_on_a_live_year`, `a_lot_bought_on_the_venue_in_2026_and_sold_out_is_covered_through_the_fold`.

## Checklist

| finding | verdict | evidence |
|---|---|---|
| **I-1** | **RESOLVED** (by T3, `d9863909`, before this fold) | `git show d9863909 --stat` shows `printed.rs` +5, `fill8949_full.rs` changed; current `crates/btctax-core/src/tax/printed.rs:53` carries `pub box_: crate::forms::Form8949Box` on `Printed8949Row`, set at `:101` from `r.box_`. `assemble_printed_forms` (`packet.rs:586-590`) now feeds routed rows into `form_8949_printed`. VERIFICATION ledger already confirms this TRUE-and-answered; not re-litigated. |
| **I-2** | **RESOLVED** | Three wiring kills, each driving the real call chain (not the predicate in isolation) — see verdict (b). All three pass at HEAD. |
| **I-3** | **RESOLVED** | `routed_8949_rows` computed before `mkdir_owner_only` on both writers; CSV kill discriminates; TUI path refuses a live year. One new Minor noted below (TUI leaves an empty directory on a refusal that fires after its own exclusive mkdir). See verdict (a). |
| **M-1** | **RESOLVED** | `a_lot_bought_on_the_venue_in_2026_and_sold_out_is_covered_through_the_fold` runs the real fold, asserts `remaining_sat == 0` on every lot, and gets `Covered`/`Noncovered` correctly by purchase year. See verdict (e). |
| **M-2** | **RESOLVED** (code), **no dedicated kill** | `admin.rs:1393-1400` appends `import_note(tax_year)` to the slice refusal message. The existing test `the_slice_arm_refuses_only_on_a_live_year` (unchanged by this fold) checks only `"income import"` and `"No forms were written"` — it does not assert the readiness-note text is present. New Minor recorded below. |
| **M-3** | **RESOLVED** (code, matches the review's own sanctioned option), **no dedicated kill, path unreachable today** | `tax.rs:685` now does `regime_or_refuse(year - 1)?` in place of the silent `is_some_and`. See verdict (f) — no test exercises the `Err` branch, and it is structurally unreachable today. New Minor recorded below. |
| **N-1** | **RESOLVED** | `bundled.rs:261`: `"four years on disk today (2026 is a record-only preparing year)"` — accurate, matches the `[2017, 2024, 2025, 2026]` assertion beside it. |
| **N-2** | **RESOLVED, weak** | `kat_forms.rs:262`: `by_lot`'s first slot is now `r.description.clone()` (String) instead of a hard-coded `0` — a real field, not a literal. But in the one test that uses it (`rows_carry_the_cohort_by_mechanism`), every leg shares `base_leg()`'s `sat: 100`, so `description` (`btc_amount_description(sat)`) is identical across all five rows — the slot still carries no discriminating information in that specific failure message. Cosmetic fix, not a functional one. Nit, not blocking. |

## Verdicts (a)–(g)

**(a) I-3 ordering + TUI consistency.**
- `write_csv_exports` (`render.rs:753-165`): `rows_8949 = match (tax_year, broker) { … routed_8949_rows(...)? … }` runs, **then** `fsperms::mkdir_owner_only(out_dir)?`. Confirmed by direct read of `render.rs`.
- `write_form_csvs` (`render.rs:944-953`): `let rows_8949 = routed_8949_rows(state, year, regime, answers)?;` runs, **then** `fsperms::mkdir_owner_only(out_dir)?`. Same order.
- The CSV kill (`broker_csv.rs::the_8949_csv_is_gated_and_routed_on_a_live_year`) asserts `!dir.exists()` after the no-answers refusal. `mkdir_owner_only` uses `DirBuilder::recursive(true)` (`fsperms.rs:73-79`), which is idempotent/creates-if-missing — so if the gate were moved *after* `mkdir_owner_only`, the directory would already exist (empty) when the refusal fires, and `!dir.exists()` would fail. **The assertion genuinely discriminates the ordering.**
- TUI (`btctax-tui/src/export.rs::do_export`): `let regime = btctax_cli::year_readiness::regime_or_refuse(year)?;` then `write_form_csvs(..., regime, None)` — `answers` is hardcoded `None`, so on any live year (`regime.basis && ≥1 exchange row`) `routed_8949_rows` returns `Err` from `slice_broker_refusal`, and `write_form_csvs` never gets far enough to write a file. This matches the commit message and is consistent with the *philosophy* S10 states ("closed … default if unanswered: closed") — but note S10's own text (`design/ROADMAP_STATUS.md:73-78`) literally scopes to "the crypto-slice `export-irs-pdf`", not the CSV surface. Applying the same closed-by-default rule to the CSV export is the conservative extension the build review's own I-3 finding recommended ("this is a gap in the spec that the build inherited … it should not survive the cycle") — a defensible reading, not a literal S10 match. The refusal names the exit (`slice_broker_refusal`'s message: "`income import` … or the TUI input form, then export the FULL return").
- **New finding (Minor, below):** `do_export`'s own `mkdir_owner_only_exclusive(&state.out_dir)` (a pre-existing, documented R0-I1 invariant) runs **before** `regime_or_refuse(year)?`. If that call errors, an **empty** `state.out_dir` is left on disk — the commit message's "a refusal writes nothing" claim holds for `write_form_csvs`'s own internal steps but not for the TUI's `do_export` as a whole. This is a pre-existing architectural pattern (not a regression introduced here), and reachability is unclear (the TUI's year picker likely only offers years with a record), so it is Minor, not Important.

**(b) I-2 wiring kills.**
1. `screen_absolute` (`return_1040.rs:2619`): `if let Some(r) = crate::tax::return_refuse::screen_broker_reporting(ri, state, year, regime) { return Some(r); }` is literally the first statement in `screen_absolute`. `screen_absolute_runs_the_broker_screen_on_a_live_year` calls `screen_absolute` directly (not `screen_broker_reporting`), so deleting that 3-line call removes the only thing the test can observe through this entry point. If deleted, `screen_absolute` falls through to its next screen; the test's `.expect("the wiring refuses")` panics if nothing else refuses, or the `matches!(r.reason, RefuseReason::BrokerReportingUnanswered { .. })` assertion fails if a *different* screen's reason surfaces instead — either way the test reds. That is the correct shape (a masking refusal from another screen is itself a discriminator, not a false pass), and it is exactly what the report's own brief anticipated.
2. `route_8949_boxes` (`packet.rs:587`, immediately after `form_8949` at `:586`, feeding `form_8949_printed` at `:590`): `the_printed_packet_routes_the_boxes_on_a_live_year` calls `assemble_printed_forms` (the real caller) and asserts `[G, K, L]`. This is now a real discriminator because T3 carried `box_` onto `Printed8949Row` — before T3 it would have asserted nothing useful (I-1's own point). Confirmed green at HEAD.
3. `slice_broker_refusal` call site in `export_irs_pdf` (`admin.rs:643-647`, immediately after `let regime = crate::year_readiness::regime_or_refuse(tax_year)?;` at `:645`, and before the `SUPPORTED_YEARS` `UnsupportedYear` check that follows a few lines later at ~`:701`): `a_live_year_refuses_the_crypto_slice_before_any_byte` drives `cmd::admin::export_irs_pdf` directly (the real CLI entry point) on a TY2026 vault with one exchange disposition and no stored `ReturnInputs`, and asserts the message contains `"Form 1099-DA answers"` and `"income import"`, and that `out_dir` does not exist. Read top-to-bottom: if the `slice_broker_refusal` block were deleted, control falls straight to `if !btctax_forms::SUPPORTED_YEARS.contains(&tax_year)`, which is `true` for `2026` (T0 made `SUPPORTED_YEARS = TEMPLATE_YEARS`, excluding 2026 — confirmed unchanged), so `UnsupportedYear` fires instead — a different `CliError` variant with a message that does **not** contain "Form 1099-DA answers", so the assertion fails. **Confirmed as described.**

**(c) Wiring kills run on TY2025-with-live-regime-as-value.** `screen_absolute_runs_the_broker_screen_on_a_live_year` uses `let st = owner_like(2025)` and calls `screen_absolute(&ri, &ar, &ty2024_params(), &st, 2025, LIVE)` — TY2025 machinery, live regime handed in as an explicit argument, not derived from any TY2026 record. The test's own comment states why: "the assembly refuses TY2026 outright today (Form 6251's 2026 Part I is untranscribed by design)." Independently confirmed: `full_return_for(2026)` stays `None` — `BundledFullReturnTables::by_year` (`btctax-adapters/src/tax_tables.rs:102`) inserts only `2024`, and `ty2026_full_return_must_stay_fail_closed` still asserts `t.full_return_for(2026).is_none()`. This is an honest test of the *wiring* (the gate fires when handed a live value through the real function), not a test that TY2026 is reachable end-to-end today — and the comment says so.

**(d) `regime_or_pre_regime`.** `year_readiness.rs:230-244`: `regime_for(year)` first; if `None` and `year < DIGITAL_ASSET_8949_FIRST_YEAR` (=`2025`, `forms.rs:147`) → `InformationReturnRegime::NONE`; else `None` → typed `UnsupportedYear`. Test `the_pre_regime_resolver_guesses_nothing_after_the_box_revision` pins `2020 → NONE`, `2025 → PROCEEDS_ONLY` (the real record), `2027 → Err(UnsupportedYear(2027))`. Grep confirms `regime_or_pre_regime` is called from exactly one production site, `admin.rs:208` (`export_snapshot`), plus its own two-year test. The pre-2025 `NONE` is a genuine historical fact — Form 1099-DA's box revision starts at `DIGITAL_ASSET_8949_FIRST_YEAR = 2025`, the same constant `forms.rs:364`'s `form_8949` already gates on — not a guessed default.

**(e) M-1's fold-driven test.** `kat_tax.rs::a_lot_bought_on_the_venue_in_2026_and_sold_out_is_covered_through_the_fold` asserts `st.lots.iter().all(|l| l.remaining_sat == 0)` immediately after running `project(...)` on a Buy-then-full-Sell pair, **before** reading `form_8949`'s cohort. Since the lot is fully consumed, `state.lots` no longer holds a live lot to query — the only place the acquisition date could come from is the value the fold wrote onto the `DisposalLeg` itself (`lot_acquired_at`). The test then asserts `Cohort::Covered` for a 2026-02 purchase and `Cohort::Noncovered` for a 2025-12-31 purchase of the same shape. This is a real, end-to-end discriminator of exactly the mutation the original M-1 finding worried about (`fold.rs:370` reading the wrong field).

**(f) M-3 — `regime_or_refuse(year - 1)?`.** `tax.rs:685`, inside the `(Some(ri_prev), Some(params), Some(table)) => { … }` arm of a `match` a few lines above (`tax.rs:667-671`, keyed on `return_inputs::get`, `full_return_for(year-1)`, `table_for(year-1)`), itself inside `report_tax_year` (`tax.rs:459`, returns `Result<TaxYearReport, CliError>`), so the `?` is valid Rust and, if it fired, would abort the **whole** `report_tax_year` call, not just the M4 advisory computation. Checked whether this is reachable: `full_return_for(year)` (`btctax-adapters/src/tax_tables.rs:108`) is `Some` **only** for `year == 2024` today (the `by_year` map has one entry). So this branch can only ever run with `year - 1 == 2024`, and `2024 ∈ bundled_years() == [2017, 2024, 2025, 2026]`, so `regime_for(2024)` — and therefore `regime_or_refuse(2024)` — cannot currently return `None`/error. **Genuinely unreachable today**, matching the review's own characterization. No new test was added to exercise the `Err` arm (none exists in the diff, and none is feasible without adding a synthetic year outside `bundled_years()` with `FullReturnParams`, which the current fixtures don't support). This is Minor, not blocking, but it is a code fix riding entirely on an invariant (every `FullReturnParams` year is also a `bundled_years()` year) that is not itself asserted by a cross-check test — new finding recorded below.

**(g) Scoped run reproduced.** `16 tests run: 16 passed, 1848 skipped`, exit 0. See "Machine check run" above; not run twice.

## New findings

### NEW-1 (Minor) — TUI `do_export` can leave an empty directory behind on a `regime_or_refuse` refusal

**Where.** `crates/btctax-tui/src/export.rs::do_export` — `fsperms::mkdir_owner_only_exclusive(&state.out_dir)` (pre-existing, R0-I1) runs before the new `let regime = btctax_cli::year_readiness::regime_or_refuse(year)?;`.

**What is wrong.** If `regime_or_refuse(year)` errors, `write_form_csvs` (whose own internal ordering — `routed_8949_rows` before its internal `mkdir_owner_only` — is correct) is never even called, but `state.out_dir` already exists (created exclusively a few lines above). The commit message's blanket "a refusal writes nothing" does not hold for this call chain as a whole, only for `write_form_csvs`'s own internal steps. This is a pre-existing TUI architectural pattern (the doc comment already documents `AlreadyExists` as an expected outcome of a prior failed attempt), not a regression this fold introduced, and reachability through the TUI's year picker is unclear (it likely only offers years with a record). No wrong data is ever written — only an empty directory can be left.

**Minimal change.** Either move `regime_or_refuse(year)?` before the exclusive mkdir (if that ordering is otherwise safe for the TUI's `AlreadyExists` contract), or soften the commit-message/doc-comment claim to scope "writes nothing" to `write_form_csvs`'s internal gate rather than the whole `do_export` chain.

### NEW-2 (Minor) — M-2's fix has no discriminating test

**Where.** `crates/btctax-cli/src/cmd/admin.rs:1393-1400` (the `import_note` interpolation); `crates/btctax-cli/tests` / `src/cmd/admin.rs`'s own `slice_broker_tests` module (unchanged by this fold).

**What is wrong.** The only test touching this message, `the_slice_arm_refuses_only_on_a_live_year`, checks for `"income import"` and `"No forms were written"` — both present before *and* after this fold's change — and does not check for the appended readiness note. A future edit that silently drops the `import_note(tax_year)` interpolation (or breaks its formatting) would not be caught by the suite.

**Minimal change.** Extend `the_slice_arm_refuses_only_on_a_live_year` (or add a small new test) asserting the TY2026 message contains `import_note(2026)`'s distinctive text (e.g., the "January-2027" / "full-return parameters" wording), so the wiring — not just `import_note`'s own standalone correctness — is watched.

### NEW-3 (Minor) — M-3's fix has no test and rests on an unasserted cross-invariant

**Where.** `crates/btctax-cli/src/cmd/tax.rs:685` (`regime_or_refuse(year - 1)?`); `crates/btctax-adapters/src/tax_tables.rs:102` (`BundledFullReturnTables::by_year` has one entry, `2024`).

**What is wrong.** As shown in verdict (f), the `Err` arm of this `?` is unreachable today only because every year present in `BundledFullReturnTables::by_year` also has a `bundled_years()` YEAR.toml record — an invariant that is true by inspection today but is not itself pinned by a test (unlike the T0-era `the_constant_and_the_regime_agree_on_every_bundled_year` pattern used elsewhere in this codebase for similar cross-checks). If a future year gets `FullReturnParams` bundled before its YEAR.toml record lands, `report_tax_year` for the *following* year would hard-fail (via `?`) instead of silently dropping the M4 advisory (the old behavior) — a bigger blast-radius change than the M-3 finding's minimal-change note fully spelled out, though `regime_or_refuse(year - 1)?` was one of the two options the review explicitly sanctioned.

**Minimal change.** Add a cross-check test asserting `BundledFullReturnTables::by_year`'s key set ⊆ `bundled_years()`, mirroring the T0 pattern already used for the regime/constant pair — this pins the invariant the M-3 fix now silently depends on.

### NEW-4 (Nit) — N-2's fix is cosmetic in the one test that exercises it

**Where.** `crates/btctax-core/tests/kat_forms.rs:262` (`by_lot`'s `r.description.clone()`); `:56` (`base_leg()`'s `sat: 100`, shared by all five legs in `rows_carry_the_cohort_by_mechanism`).

**What is wrong.** `description = btc_amount_description(sat)` (`forms.rs:333`, `:380`) — since none of the five `DisposalLeg`s built in that test override `sat`, all five rows' `description` is the identical string. The fix replaces a hard-coded `0` with a real field, which is the right shape in principle, but in this specific test it still carries no distinguishing information — the debug tuple's identifying slot is constant either way.

**Minimal change.** None required to unblock; if the message is ever actually needed for debugging a real failure, give at least one leg in the fixture a distinct `sat` (or use `lot_id`, which is already distinct per leg in this test) so the identifying slot actually identifies.

## Counts

**0 Critical / 0 Important / 3 Minor / 1 Nit** (all newly found in this re-verification; none reopen a blocking status on the original I-2/I-3/M-1/M-2/M-3/N-1/N-2 checklist, all of which are RESOLVED or RESOLVED-with-caveat as tabulated above).

**Gate: PASS.** All seven items the brief asked about are resolved; the three wiring kills genuinely discriminate their respective call sites; the CSV gate genuinely runs before any byte on both writers; the fold-driven `Covered` kill genuinely proves the date came from the leg. The four new findings are all Minor/Nit — none blocks.
