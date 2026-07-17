# Fable independent re-review — IMPLEMENTATION_PLAN r2 (persisted verbatim)

*Persisted 2026-07-16 verbatim, per STANDARD_WORKFLOW §2. Reviewer: Fable (independent). Verdict: GREEN —
0 Critical / 0 Important. Ready to execute. One non-gating Minor (M8) folded inline into Task 1.2.*

---

# Fable independent re-review — IMPLEMENTATION_PLAN r2

## VERDICT: GREEN — 0 Critical / 0 Important. Ready to execute.

The r1→r2 diff (474af90..ac626ea) is confined to exactly the two folds plus Status; both folds verified against source; no regression, no stale citation.

### I9 — RESOLVED

- **Genuine changed-row scenario.** The new `coinbase_two_lot_tax_saving(dir)` spec matches `write_tax_saving_csv` (`crates/btctax-cli/tests/optimize_run.rs:85-98`) exactly: LT 1 BTC @ $30k 2023-01-01 + ST 1 BTC @ $80k 2025-01-02 + 1 BTC Sell @ $50k 2025-06-01. The fixture's own golden comment (optimize_run.rs:76-84) documents FIFO→Lot A ($20k LT gain) vs HIFO→Lot B (−$30k ST loss), so `proposed_selection != current_selection` and the "already optimal" skip (`cmd/optimize.rs:222-228`) does not fire — the persistability match (:231-270) is reached. Born-PASSING: backdated stdout is `"…1 persisted, 0 skipped." + "PERSISTED … [Contemporaneous]…"`, postdated is `"…0 persisted, 1 skipped." + "skipped …: already executed — re-run …"` (`render.rs:1871-1900`, printed at `main.rs:185`) — `assert_ne!` holds, and the inverted mutation goes RED for the right reason.
- **Pins.** `persistability(wallet, sale_date, selection_made)` verified at `crates/btctax-core/src/optimize.rs:469-484`: broker ∧ year≥2027 first, else made≤sale ⇒ ContemporaneousNow, else NeedsAttestation. `made = tax_date(now)` from the seam (`cmd/optimize.rs:183`). 2025-01-01 ≤ 2025-06-01 ⇒ ContemporaneousNow; 2026-06-01 > sale ⇒ NeedsAttestation; sale year 2025 keeps ForbiddenBroker2027 dead despite Coinbase = `WalletId::Exchange` = broker (`is_broker`, optimize.rs:451-453). The comment now correctly claims the **pre-2027-sale arm** (plan ~:229-233); the "non-broker" error is gone.
- **Compiles.** All three `accept_under` calls match `run_in(&Path, &[(&str,&str)], &[&str]) -> (i32,String,String)`; fixtures builders return `PathBuf` (`coinbase_buy_sell_send(dir: &Path) -> PathBuf` convention) so `csv.to_str().unwrap()` typechecks; `#[path="fixtures.rs"] mod fixtures;` from a tests/-root file is equivalent to end_to_end.rs's `mod fixtures;`. Bonus check: `append_decision` (`btctax-core/src/persistence.rs:238-262`) has **no timestamp-monotonicity guard**, so the backdated decision appends cleanly after wall-clock import rows. `Optimize::Accept{tax_year, disposal: Option, attest: Option}` confirmed at cli.rs:315-326.

### M7 — RESOLVED

`income import --year --file` (cli.rs:359-366) and `income show --year` (cli.rs:368-372) exist; dispatch prints stored inputs as pretty JSON (main.rs:236-246). `parse_return_inputs_toml` (cmd/tax.rs:114-132) is plain serde (+`serde_ignored`), so the committed TOML maps 1:1 onto `ReturnInputs`, which derives `Serialize, Deserialize, PartialEq, Eq` (return_inputs.rs:370-371) — the plan's **primary** comparison path (direct serde equality vs `kitchen_sink_household().0`) is confirmed available, and btctax-cli integration tests reach both btctax-core and the binary. Coherent; the "else" fallback never has to run.

### NEW Minor (record; non-gating)

**M8 — the `income show` read-back is PII-masked; the comparison must never be show-JSON-vs-raw-vector.** `show_return_inputs` applies `mask_pii` (cmd/tax.rs:149-162: SSNs → `***-**-NNNN`, ip_pin → `***`) and `kitchen_sink_household` sets real SSN strings (testonly.rs:152/:177/:524/:533). So at execution the "captured ReturnInputs equals the vector" assertion must use committed-TOML-parse-vs-vector (the primary path, unaffected) or show-vs-show; parsing the show JSON against the raw vector would be loudly born-RED. (Also: if the show-vs-show fallback is ever taken, serialize the vector via `toml::Value::try_from` — direct `toml::to_string` of the nested model can hit ValueAfterTable, per the code's own note at cmd/tax.rs:177.) The plan's "finalize the comparison surface at execution" hedge plus the loud failure mode keeps this non-gating.

### Regression scan — clean

New fixture name used consistently across Task 0.2 Files ↔ Interfaces ↔ test body ↔ Step 3 hedge ↔ the Task 1.2 C-multilot DRY note ↔ Status; the only surviving `coinbase_buy_sell_send` mention is historical narration in Status. All citations introduced by the fold re-verified: optimize_run.rs:85-98 (exact), optimize.rs:451-453/476-478, cmd/optimize.rs:198-208 (call spans :199-209 — accurate), cli.rs:315-326 (exact), cmd/tax.rs:114 (exact). No other task text changed; no r1-resolved item regressed.

**GREEN 0C/0I — the plan is ready to execute.** Record M8 in FOLLOWUPS (owning phase P1, Task 1.2 execution) or fold the one-line caution inline; either way it does not hold the gate.
