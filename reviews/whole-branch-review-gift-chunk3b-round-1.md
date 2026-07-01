# Whole-branch review — Charitable/gift Chunk 3b (Form 8283 Section-B appraiser/donee)

**Scope:** FINAL whole-branch net over Task 1 + Task 2 + the fix sweep.
**Range:** `114d6e0..c81b6f3` (4 commits: spec `11896f0`, task1 `276f346`, task2 `bef4ad1`, fix-sweep `c81b6f3`).
**Branch:** `feat/gift-chunk3b`, working tree clean.
**Method:** cross-cutting only (per-task reviews were clean; hunt for what they MISSED). Gate is pre-verified GREEN (644 tests, clippy `-D warnings`, fmt) — not re-run.

## Verdict

**READY TO MERGE — 0 Critical / 0 Important.** (2 Minor, 4 Nit — all non-blocking, recommended follow-ups.)

Explicit confirmations requested by the brief:
- **(a) Skeletal-Section-B honest-gap lock: INTACT.** The predicate `needs_review = d.is_none_or(|d| !d.is_review_complete(section))` on the carrier row is correct end-to-end; `is_review_complete(Section::B)` requires `appraiser_name` (non-empty) + (`tin` OR `ptin`) + `appraisal_date` + `appraiser_qualifications` + `donee_ein`. Skeletal `--donee-name X --appraiser-name Y` on a Section-B donation → `is_review_complete` = false → `needs_review` = **true**. KAT `form8283_skeletal_section_b_keeps_needs_review_true` genuinely locks this — it asserts `needs_review == true` WITH names populated, so it fails if the flip were made unconditional (`d.is_some()`). Non-carrier legs are hard-wired `needs_review = true`. Per-field donation.rs unit tests lock every Section-B requirement individually.
- **(b) TUI read-only guarantee: INTACT (compile-enforced).** `Snapshot.donation_details` is loaded in `build_snapshot(session: &Session)` via `session.donation_details()`, which is `&self` (`session.rs`). The `Session` binding in `attempt_open` is immutable (`let session`, not `let mut`), so `save()` (which takes `&mut self`) is a compile error. Grep of `crates/btctax-tui/src/` for `save(` / `append_` / `.conn(` / `mut session` → **zero production hits**; the only `cmd::` hits are doc-comment text and test-only `cmd::init::run` (all inside `#[cfg(test)]`). The `vault_file_bytes_unchanged_after_open_build_snapshot_drop` byte-identity test is present and passing.

## Cross-cutting verification (brief items 1–7)

1. **needs_review honest-gaps mechanism (highest priority) — PASS.** Predicate + KATs genuine end-to-end: side-table (`donation_details::all`) → `Session::donation_details()` → `form_8283(state, year, &map)` → CSV (`write_form8283_csv`, `needs_review.to_string()`) and TUI (`tabs/forms.rs` `[review]` marker). All four KAT cases present and correct: full-B → false; skeletal-B → true; A-with-details → false; none → true.

2. **Old-vault back-compat — PASS.** `donation_details` DDL is `CREATE TABLE IF NOT EXISTS`; `init_table` is called in `from_fresh_vault` AND defensively at the top of every `get`/`set`/`all`. `Session::open` does NOT require the table (only the defensive guard runs on first access). Tableless-vault tests are genuine (`get_on_tableless_vault_returns_none`, `all_on_tableless_vault_returns_empty_map`, `defensive_guard_in_set_creates_table` — each opens a bare in-memory conn with NO `init_table`). No path where an existing vault fails to open or read.

3. **btctax-tui read-only (Critical-class) — PASS.** See confirmation (b). Loaded via the immutable Session / `&self` accessor; zero production mutation hits; byte-identity test present.

4. **Standalone — PASS.** `git diff --stat 114d6e0..c81b6f3` for `state.rs`, `project.rs`, `tax/`, `project/` is **empty** — all untouched. `DonationDetails` never enters `LedgerState`/the fold (it lives in its own `donation.rs`, is passed as a function parameter to `form_8283` only, and is embedded on `Form8283Row` for CSV flattening). No golden/tax-identity file appears in the changed-file list.

5. **Fix sweep (c81b6f3) — PASS (with Minor-1 on the PTG test).** §2505 KAT now pins `"($0.00 remaining)"` — genuinely un-satisfiable by the lifetime-exclusion figure `"13990000.00"` (correct tightening). The `--prior-taxable-gifts` negative guard is moved OUT of the `if let Some(y) = tax_year` block in `main.rs`, so it is now ALWAYS-ON (parse + `is_sign_negative` run before the branch; absent flag → `unwrap_or_default()` = 0, not rejected; only a negative VALUE errors). Relocated `form8283_csv_tests` module + updated `tests/export.rs` column comment (through `appraisal_date(17)`) are correct.

6. **CSV integrity — PASS.** `form8283.csv` header adds the 6 new columns (`donee_ein`, `donee_address`, `appraiser_tin`, `appraiser_ptin`, `appraiser_qualifications`, `appraisal_date`) in stable order after `needs_review`; populated on the carrier row from `row.details`, empty on non-carrier legs and when no details. `write_form8283_csv` still opens via `fsperms::open_owner_only` (0o600). The `set`/`show` CLI validate against the PROJECTED `state.removals` (Donation-only): `set_donation_details` errors on missing ref and on a non-Donation removal; the Gift-arm error is tested (`set_donation_details_gift_removal_is_usage_error`).

7. **PII — PASS.** All test data synthetic: `987-65-4321` (SSA-reserved never-issued SSN), `12-3456789` (sequential synthetic EIN), `987654321`/`P01234567` (synthetic TIN/PTIN), `"Test Charity"`/`"Test Appraiser"`. `202302012` is the CCA legal citation in the SPEC, not PII. Exact `Decimal` throughout (no float); determinism via `BTreeMap<EventId, DonationDetails>`.

## Findings

### Minor

- **M1 — `report_negative_prior_taxable_gifts_rejected_without_tax_year` (tests/tax_report.rs) is tautological.** The test re-implements the guard inline (`let result = if parsed.is_sign_negative() { Err(...) } else { Ok(()) }`) and asserts its OWN re-implementation. It never drives `main::run()` / the `Command::Report` path, so it cannot catch a regression that moves the guard back inside the `if let Some(y) = tax_year` block — the exact regression it claims to lock. The production code in `main.rs` IS correct (verified by reading the moved guard). This is ironic given the same sweep tightened KAT-B for being weak. Not blocking (no invariant broken, code correct), but it gives false assurance. Recommend an `assert_cmd`-style test invoking the binary with `--prior-taxable-gifts -5` and no `--tax-year`, or extract a small testable guard helper the test can call directly.

- **M2 — No end-to-end test through the `all()` reparse seam.** No single test wires `set-donation-details` → `session.donation_details()` (the `donation_details::all()` path that reparses each stored `canonical()` key via `parse_event_id`) → `form_8283`'s `details.get(&r.event)` lookup → exported CSV/TUI needs_review flip. Each link is individually tested (reconcile set/show round-trip via `get`; CSV columns via a directly-built map; form_8283 flip via a literal map), and the EventId canonical↔reparse round-trip is an established invariant — `optimize_attest::attested_set` uses the identical `parse_event_id(&stored_canonical)` pattern to feed `compliance_overlay`'s `contains(&disposal.event)`. So this is a thoroughness gap, not a latent defect. Recommend one integration test that stores details on a real projected donation and asserts the exported row's `needs_review == "false"`.

### Nit

- **N1 — "empty otherwise" for the 6 new CSV columns is not directly asserted.** The unit test is named `..._present_and_empty` but only exercises the POPULATED case; the integration export test asserts emptiness for columns 9–11 (donee/appraiser/needs_review) but not 12–17. Behaviorally correct (`d.and_then(..).unwrap_or_default()` → `""` when `None`); the "empty" half of the name is just unfulfilled.
- **N2 — `"Habitat For Humanity"` / `"Habitat"` used as test donee labels** (kat_forms.rs, render.rs unit test). A real public charity name; no EIN attached, so not a PII leak, but inconsistent with the fully-fictional `"Test Charity"` used elsewhere. Prefer a fictional name for uniformity.
- **N3 — Stale citation in the Task-2 report.** Minor finding #2 cited `cmd/export.rs (~line 473-474)`; no such file exists (export lives in `cmd/admin.rs::export_snapshot`). The actual stale column comment was in `tests/export.rs` and WAS correctly fixed by sweep item 4. Report-only; no code impact.
- **N4 — Behavior change on the display path (intended).** `--prior-taxable-gifts -5` with no `--tax-year` now errors where it was previously a silent no-op. Correct per the fix-sweep intent (don't silently skip validation); a non-negative value without `--tax-year` remains silently ignored (unchanged). Noted for completeness — not a compat concern.

## Severity roll-up

| Severity  | Count |
|-----------|-------|
| Critical  | 0     |
| Important | 0     |
| Minor     | 2     |
| Nit       | 4     |

**Chunk 3b is READY TO MERGE (0 Critical / 0 Important).** The charitable/gift completion cluster (Chunk 1/2/3a/3b) is complete; M1/M2 are recommended test-quality follow-ups for `FOLLOWUPS.md`, not merge blockers.
