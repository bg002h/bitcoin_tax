# IMPLEMENTATION PLAN — Sub-project B: Rate / NIIT / Loss-Limit Engine

**Program:** Lot-Identification & Tax-Optimization (Phase-2). **Sub-project:** B (rate engine; built after A, before C; A → B → C).
**Source of truth:** `design/SPEC_lot_optimization_program.md` (R0-GREEN 2026-06-29). Sub-project B (§B.1–B.5) + the "Rate authorities (B)" bullet of **Legal grounding** + **Cross-cutting** are **binding**.
**Predecessor:** Sub-project A (lot-identification substrate) is **SHIPPED on main** — `LotMethod{Fifo,Lifo,Hifo}`, `MethodElection`/`LotSelection`, the three new `BlockerKind`s, `disposal_compliance`, and `evaluate_disposal` are present in current source (verified §1 below).
**Status:** DRAFT — to be **R0-reviewed** (review-to-green, `STANDARD_WORKFLOW.md §2`) **before** any code. Then executed subagent-driven, **one implementer carrying the whole plan** (Phase D, §1).

> **How to execute (per `STANDARD_WORKFLOW.md`):** each numbered task below is a TDD phase. Per task: (1) write the failing test(s) — real test code; (2) run → confirm **RED**; (3) minimal implementation — real code; (4) run → confirm **GREEN**; (5) run the **whole** validation surface (`cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check`); (6) independent review loop → 0 Critical / 0 Important; (7) commit. **Gates are hard** (§0 of the workflow). The closing **Task 11** is the mandatory whole-diff review (Phase E).

---

## 0. Global Constraints (apply to EVERY task — a violation is a blocking finding)

- **NFR4 determinism.** Identical `(events, prices, config, profile, tables, year)` → byte-identical `TaxOutcome`. Every iteration over a map is over a `BTreeMap`/`BTreeSet` or a pre-sorted `Vec` (no `HashMap` iteration). No `Date::now`/RNG in `btctax-core`. The bundled tax tables are compiled-in, immutable reference data (no I/O on the compute path).
- **NFR5 exact arithmetic / no float — ALL rate math in `Decimal`.** Money is `Usd = Decimal` (`crates/btctax-core/src/conventions.rs:8`); sats are `Sat = i64` (`conventions.rs:6`). **No `f32`/`f64` anywhere**, including bracket rates, the 0/15/20% LT rates, and the 3.8% NIIT rate — all are `Decimal` literals (`rust_decimal_macros::dec!`). Final tax dollars are rounded with the project's canonical `round_cents` (ROUND_HALF_EVEN, `conventions.rs:13,22-24`). B uses the **exact marginal-bracket formula method at cent precision** — NOT the IRS binned Tax Tables and NOT whole-dollar rounding (deliberate exactness/determinism choice; documented in Task 3).
- **Federal only.** State tax is out of scope (app charter / spec intro). B computes federal tax only.
- **Incremental delta is the objective (I5).** `total_federal_tax_attributable := tax(profile WITH app-computed crypto items) − tax(profile WITHOUT them)`, ceteris-paribus on the minimal profile. It is **not** a full-1040 liability and does **not** model SS taxability, IRMAA, AMT, QBI, or AGI-driven phaseouts (labeled on output). Crypto ordinary income (mining/staking/etc.) is **excluded** from `ordinary_taxable_income` in the profile (B.1) and added back on the ordinary stack **exactly once** (B.3) — a double-count guard KAT pins this (Task 5/7).
- **Statutory-vs-indexed discipline (I4).** §1411 NIIT thresholds ($250k MFJ/QSS, $200k Single/HoH, $125k MFS) and the §1211 loss limit ($3,000 / $1,500 MFS) are **STATUTORY, NOT inflation-indexed** — hard-coded as year-independent constants/functions **with their statute cite**, and **never** placed in the per-year indexed table. Every indexed value (LT 0/15/20% breakpoints; ordinary brackets) lives in the per-year table and **cites its Rev. Proc.** A KAT asserts the statutory values are **constant across years** while the indexed ones move.
- **Refuse on unsound data (B.4 / I6).** B emits **no `TaxResult`** for **any** year while **any** unresolved blocker of **`Severity::Hard`** is present **anywhere** in the projection (B-I1) — the gate keys on `BlockerKind::severity() == Severity::Hard` (`state.rs:47-64`), **not** an enumerated kind/year subset, so future hard kinds auto-gate **and** cross-year basis contamination (e.g. an out-of-year `ImportConflict`/`DecisionConflict` on a lot a later disposal consumes) cannot slip a wrong number through. Instead it returns a `TaxYearNotComputable` outcome. Missing profile / missing table are their own hard outcomes (`TaxProfileMissing` / `TaxTableMissing`). A wrong number must never be presented as authoritative. **Deliberate conservatism:** any open Hard blocker anywhere blocks all year computations until it is resolved.
- **Privacy.** Tests use **synthetic fixtures + temp vaults only** (`tempfile`). No real reads, no PII. Bundled tax tables are **public reference data** (statutes + Rev. Procs.).
- **Event-sourcing boundary.** B introduces **no new ledger events**. The `tax_profile` is a **per-year side-table** (a projection input, like `cli_config` — `crates/btctax-cli/src/config.rs:1-4`); the tax tables are **bundled reference data** (like the price dataset — `crates/btctax-adapters/src/price.rs:1-2`). Neither is ledger state. B reads the projected `LedgerState` produced by A's `project` and computes on top of it.
- **Citations.** Re-verify every `file:line` in §1 against current source **at task write time** (`STANDARD_WORKFLOW.md §4`); line numbers decay. Re-verify every bracket/breakpoint value against its **primary Rev. Proc. PDF** and every statutory value against the **U.S. Code section** at Task 6 write time.

---

## 1. Source grounding (re-verified against CURRENT source at write time, 2026-06-29)

| Symbol / fact | Current location |
|---|---|
| `Usd = Decimal`, `Sat = i64`, `TaxDate = Date` | `crates/btctax-core/src/conventions.rs:6,8,10` |
| `MONEY_ROUNDING` (ROUND_HALF_EVEN), `round_cents` | `conventions.rs:13`, `conventions.rs:22-24` |
| `TRANSITION_DATE = 2025-01-01`, `tax_date(utc,tz)` | `conventions.rs:17`, `conventions.rs:52-54` |
| `LedgerState { lots, holdings_by_wallet, disposals, removals, income_recognized, pending_reconciliation, blockers, stats }` | `crates/btctax-core/src/state.rs:176-186` |
| `Disposal { event, kind, disposed_at, legs, fee_mini_disposition }` | `state.rs:107-116` |
| `DisposalLeg { lot_id, sat, proceeds, basis, gain, term, basis_source, gift_zone }` | `state.rs:96-106` |
| `Term { ShortTerm, LongTerm }` | `state.rs:6-10` |
| `IncomeRecord { event, recognized_at, sat, usd_fmv, kind, business }` | `state.rs:140-148` |
| `Severity { Hard, Advisory }` | `state.rs:17-21` |
| `BlockerKind` enum (ends at `Pre2025MethodConflictsAllocation`) | `state.rs:22-46` |
| `BlockerKind::severity()` (Hard set) | `state.rs:47-64` |
| `Blocker { kind, event: Option<EventId>, detail: String }` | `state.rs:65-70` |
| `LedgerState::add_blocker` | `state.rs:187-200` |
| `new_blockers_are_hard` test (extend with the 3 new kinds) | `state.rs:206-217` |
| `LedgerEvent { id, utc_timestamp, original_tz, wallet, payload }` | `crates/btctax-core/src/event.rs:263-271` |
| `IncomeKind { Mining, Staking, Interest, Airdrop, Reward }` | `event.rs:28-35` |
| `project(events, prices, config) -> LedgerState` | `crates/btctax-core/src/project/mod.rs:47-56` |
| `LotMethod { Fifo, Lifo, Hifo }` (+ `Default`, serde) | `project/mod.rs:24-30` |
| `ProjectionConfig { self_transfer_fee, pre2025_method }` | `project/mod.rs:31-45` |
| core `PriceProvider` trait / `fmv_of` / `StaticPrices` (test double) | `crates/btctax-core/src/price.rs:5`, `:13`, `:22-23` |
| core `lib.rs` re-exports (`pub use project::{…}`, `pub use state::*`) | `crates/btctax-core/src/lib.rs:11-20` |
| **Bundled-dataset pattern**: `BundledPrices { by_date: BTreeMap }`, `include_str!`, `load()`, `from_csv_str`, `impl PriceProvider` | `crates/btctax-adapters/src/price.rs:10,14,19-21,24-44,46-50` |
| adapters `lib.rs` re-export of `BundledPrices` | `crates/btctax-adapters/src/lib.rs` (mirror for `BundledTaxTables`) |
| **Side-table pattern**: `CliConfig`, `Default`, `init_config_table`, `get`, `read_config`, `set_pre2025_method`, `BadConfigValue` | `crates/btctax-cli/src/config.rs:10-15,17-26,42-47,49-55,75-119,122-134` |
| `Session::{create,open,config,project,load_events_and_project}`, `from_fresh_vault` inits config table | `crates/btctax-cli/src/session.rs:26-28,46-50,69-71,75-81,87-95,38-43` |
| `cmd::inspect::report(vault, pp, year) -> LedgerState`; `verify` | `crates/btctax-cli/src/cmd/inspect.rs:11-19,29-34` |
| `render_report(state, year)`; `render_verify` | `crates/btctax-cli/src/render.rs:119-226,566-655` |
| `cmd::admin::{show_config,set_config,set_pre2025_method}` | `crates/btctax-cli/src/cmd/admin.rs:10-12,15-26,29-39` |
| `eventref::{parse_usd_arg, parse_date_arg, parse_wallet_id, parse_income_kind}` | `crates/btctax-cli/src/eventref.rs:76-78,80-83,57-73,122-131` |
| clap `Command::{Report{year}, Config{…}}` + dispatch | `crates/btctax-cli/src/main.rs:25-74` (Report `:40-43`), dispatch `:247-305` |
| `CliError::BadConfigValue { key, value }` pattern | `crates/btctax-cli/src/config.rs:86-90` (defined in `lib.rs`/`CliError`) |
| test helpers `ev`/`dec_ev`, `StaticPrices`, KAT style | `crates/btctax-core/tests/kat_tax.rs:16-34` |

> **Naming adaptation (noted once).** The spec writes the profile fields and the `TaxResult` shape inline. The codebase convention is a **named struct per concept** with `snake_case` fields exactly matching the spec wording. This plan therefore defines `struct TaxProfile`, `struct TaxResult`, `struct Carryforward`, `struct MarginalRates`, `enum FilingStatus`, and the table structs. Field names/types are exactly as the spec mandates (B.1/B.3).

---

## 2. New public API surface introduced by Sub-project B

- **Core types (`btctax-core`, new module `tax`):**
  - `FilingStatus { Single, Mfj, Mfs, HoH, Qss }` (serde; `Qss` aliases `Mfj` for every rate lookup — §1(h)/§1/§1411 treat a qualifying surviving spouse as MFJ).
  - `TaxProfile { filing_status: FilingStatus, ordinary_taxable_income: Usd, magi_excluding_crypto: Usd, qualified_dividends_and_other_pref_income: Usd, other_net_capital_gain: Usd, capital_loss_carryforward_in: Carryforward }` (serde — persisted by the CLI side-table as JSON).
  - `Carryforward { short: Usd, long: Usd }` (serde; used both for `carryforward_in` and `carryforward_out`).
  - `MarginalRates { ordinary: Usd, ltcg: Usd, niit_applies: bool }`.
  - `TaxResult { st_net: Usd, lt_net: Usd, ordinary_from_crypto: Usd, ltcg_tax: Usd, niit: Usd, loss_deduction: Usd, carryforward_out: Carryforward, total_federal_tax_attributable: Usd, marginal_rates: MarginalRates }`.
  - `TaxOutcome { Computed(TaxResult), NotComputable(Blocker) }`.
- **Core table types + trait (`tax::tables`):**
  - `OrdinaryBracket { lower: Usd, rate: Usd }`; `OrdinarySchedule { brackets: Vec<OrdinaryBracket> }` (ascending; last open-ended).
  - `LtcgBreakpoints { max_zero: Usd, max_fifteen: Usd }` (§1(h)).
  - `TaxTable { year: i32, source: &'static str, ordinary: BTreeMap<FilingStatus, OrdinarySchedule>, ltcg: BTreeMap<FilingStatus, LtcgBreakpoints> }` (**indexed values only** — never NIIT/loss-limit) + accessors `ordinary_for(status)` / `ltcg_for(status)` (map `Qss → Mfj`).
  - `trait TaxTables { fn table_for(&self, year: i32) -> Option<&TaxTable>; }`.
  - **Statutory (NOT indexed), in `tax::tables` as year-independent fns/consts with cites:** `fn niit_threshold(s: FilingStatus) -> Usd` (§1411(b)); `const NIIT_RATE: Usd = dec!(0.038)` (§1411(a)); `fn loss_limit(s: FilingStatus) -> Usd` (§1211(b)).
- **Core compute fns (`tax::compute`):**
  - `pub fn ordinary_tax_on(schedule: &OrdinarySchedule, taxable: Usd) -> Usd`.
  - `pub fn preferential_tax(bp: &LtcgBreakpoints, bottom: Usd, pref: Usd) -> PrefSplit` where `PrefSplit { at_0: Usd, at_15: Usd, at_20: Usd, tax: Usd }`.
  - `pub fn net_1222(crypto_st: Usd, crypto_lt: Usd, other_lt: Usd, cf_short: Usd, cf_long: Usd, loss_limit: Usd) -> CapNet` where `CapNet { st_net, lt_net, ordinary_gain, preferential_gain, loss_deduction, st_carry, lt_carry }`.
  - `pub fn compute_tax_year(events: &[LedgerEvent], state: &LedgerState, year: i32, profile: Option<&TaxProfile>, tables: &dyn TaxTables) -> TaxOutcome`.
  - `pub fn carryforward_consistency(prior_year_out: Option<&Carryforward>, this_year_in: &Carryforward) -> Option<String>` (M4 advisory).
- **New `BlockerKind`s (`state.rs`), all `Severity::Hard`:** `TaxProfileMissing`, `TaxTableMissing`, `TaxYearNotComputable`.
- **Adapters (`btctax-adapters`):** `BundledTaxTables { by_year: BTreeMap<i32, TaxTable> }` with `load()`; `impl TaxTables`. TY2025 numbers encoded from Rev. Proc. 2024-40 (TY2026 from Rev. Proc. 2025-32 **if verified**, else omitted → `TaxTableMissing` is the safety).
- **CLI (`btctax-cli`):**
  - `tax_profile` side-table (`tax_profile(year INTEGER PRIMARY KEY, profile_json TEXT)`); `init_tax_profile_table`, `get_tax_profile(conn, year)`, `set_tax_profile(conn, year, &TaxProfile)`, `all_tax_profiles(conn)`.
  - `Command::TaxProfile { year, filing_status, ordinary_taxable_income, magi_excluding_crypto, qualified_dividends, other_net_capital_gain, carryforward_short, carryforward_long, show }`.
  - `Command::Report` gains `--tax-year <y>` (distinct from the existing display `--year` filter) → computes + renders the `TaxResult` / `TaxYearNotComputable` reason.
  - `cmd::tax::{set_profile, show_profile, report_tax_year}`; `render::render_tax_result` / `render_tax_outcome`.

---

## 3. Task list

1. **Core tax types + 3 hard blockers** — `tax` module skeleton: `FilingStatus`, `TaxProfile`, `Carryforward`, `MarginalRates`, `TaxResult`, `TaxOutcome` (serde where needed); `BlockerKind::{TaxProfileMissing, TaxTableMissing, TaxYearNotComputable}` (Hard).
2. **Tax-table types + `TaxTables` trait + statutory constants** — `OrdinaryBracket`/`OrdinarySchedule`/`LtcgBreakpoints`/`TaxTable`/`TaxTables`; statutory `niit_threshold`/`NIIT_RATE`/`loss_limit` (year-independent, cited); statutory-vs-indexed KAT; a `BTreeMap`-backed test-double `TaxTables`.
3. **Rate-application primitives** — `ordinary_tax_on` (exact marginal bracket math) + `preferential_tax` (§1(h) 0/15/20 stacking with `PrefSplit`); boundary KATs.
4. **§1222 netting + §1211/§1212 loss limit** — `net_1222`/`CapNet`: within-character net (incl. `carryforward_in`), cross-net, $3k/$1.5k limit, §1212(b) **ST-first** character-preserving carryforward; netting-order + ST-first + multi-year KATs.
5. **`compute_tax_year` (assembly + delta + refusal)** — pull year's disposals/income, two-scenario (with/without crypto) ordinary + §1(h) + §1411 NIIT, incremental delta, `TaxResult`; hard-blocker refusal → `TaxYearNotComputable`; `TaxProfileMissing`/`TaxTableMissing`. Mechanics KATs on **synthetic** tables (NIIT crossing, ST stacking, double-count guard, refusal).
6. **Bundled per-year tax tables (adapters)** — `BundledTaxTables` with **TY2025 (Rev. Proc. 2024-40)** numbers (+ TY2026 if verified); pin-the-data KATs; statutory-constant-across-years KAT.
7. **Worked-example golden KATs against the bundled TY2025 table (adapters tests)** — per filing status, hand-verified from the bundled numbers: LT 0→15→20; QD pushes 15→20; NIIT threshold crossing; ST stacking; $3k limit + multi-year §1212(b) ST-first carryforward; §1222 netting; incremental-delta double-count guard; `total = ordinary_delta + ltcg_tax + niit` identity.
8. **`tax_profile` side-table + `tax-profile` CLI command** — storage (modeled on `cli_config`); `tax-profile` set/show; `TaxProfileMissing` surfaced; round-trip + bad-JSON KATs.
9. **`report --tax-year` surfacing** — wire `BundledTaxTables` + profile + projection into `compute_tax_year`; render `TaxResult` / `TaxYearNotComputable` / missing-profile / missing-table; exact Decimal formatting; deterministic; CLI integration KATs.
10. **`carryforward_in ↔ prior-year carryforward_out` consistency (M4)** — `carryforward_consistency` advisory; surfaced (non-gating) in `report --tax-year` when both years' profiles exist.
11. **Whole-diff review + full-suite green (Phase E gate).**

**Dependency order:** 1 → 2 → 3 → 4 → 5; 6 depends on 2; 7 depends on 5,6; 8 depends on 1; 9 depends on 5,6,8; 10 depends on 5,8. **11 last.**

---

## TASK 1 — Core tax types + 3 hard blockers

**Goal.** Stand up the `tax` module with the input/output types (no logic yet) and the three new hard `BlockerKind`s, so every later task has concrete types to compute over. `TaxProfile`/`FilingStatus`/`Carryforward` derive serde (the CLI side-table persists `TaxProfile` as JSON in Task 8).

**Files**
- new `crates/btctax-core/src/tax/mod.rs` (module root + re-exports)
- new `crates/btctax-core/src/tax/types.rs`
- modify `crates/btctax-core/src/state.rs` (3 `BlockerKind` variants + `severity()` arm + extend `new_blockers_are_hard`)
- modify `crates/btctax-core/src/lib.rs` (`pub mod tax;` + re-exports)

**Interfaces**
```rust
// tax/types.rs
use crate::conventions::Usd;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum FilingStatus { Single, Mfj, Mfs, HoH, Qss }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Carryforward { pub short: Usd, pub long: Usd }   // §1212(b): always ≥ 0 magnitudes

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaxProfile {
    pub filing_status: FilingStatus,
    pub ordinary_taxable_income: Usd,                  // EXCLUDES all app-computed crypto items (B.1)
    pub magi_excluding_crypto: Usd,                    // for the §1411 threshold (B.1)
    pub qualified_dividends_and_other_pref_income: Usd, // shares the §1(h) 0/15/20 stack (B.1/I9)
    #[serde(default)] pub other_net_capital_gain: Usd,  // non-crypto net LT-character gain (optional)
    #[serde(default)] pub capital_loss_carryforward_in: Carryforward, // prior-year, by character (optional)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarginalRates { pub ordinary: Usd, pub ltcg: Usd, pub niit_applies: bool }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaxResult {
    pub st_net: Usd,                  // §1222 within-character net short-term WITH crypto (signed)
    pub lt_net: Usd,                  // §1222 within-character net long-term WITH crypto (signed)
    pub ordinary_from_crypto: Usd,    // Σ crypto ordinary income for the year (added to the stack once)
    pub ltcg_tax: Usd,                // crypto-attributable preferential-rate tax (DELTA)
    pub niit: Usd,                    // crypto-attributable §1411 NIIT (DELTA)
    pub loss_deduction: Usd,          // §1211 ordinary offset actually used this year WITH crypto (level)
    pub carryforward_out: Carryforward, // §1212(b) carryforward WITH crypto (level; feeds next year)
    pub total_federal_tax_attributable: Usd, // THE objective (DELTA: with − without)
    pub marginal_rates: MarginalRates,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaxOutcome {
    Computed(TaxResult),
    NotComputable(crate::state::Blocker), // kind ∈ {TaxYearNotComputable, TaxTableMissing, TaxProfileMissing}
}
```
```rust
// tax/mod.rs
pub mod types;
pub use types::{Carryforward, FilingStatus, MarginalRates, TaxOutcome, TaxProfile, TaxResult};
// (tables / compute modules + re-exports added in Tasks 2–5)
```

**Steps**

1. **Failing test** — extend `state.rs`'s `new_blockers_are_hard` (`state.rs:206-217`) and add a types smoke test in `tax/types.rs`:
```rust
// in state.rs tests::new_blockers_are_hard, append:
assert_eq!(BlockerKind::TaxProfileMissing.severity(), Severity::Hard);
assert_eq!(BlockerKind::TaxTableMissing.severity(), Severity::Hard);
assert_eq!(BlockerKind::TaxYearNotComputable.severity(), Severity::Hard);
```
```rust
// tax/types.rs  #[cfg(test)] mod tests
use super::*;
use rust_decimal_macros::dec;

#[test]
fn tax_profile_serde_round_trips() {
    let p = TaxProfile {
        filing_status: FilingStatus::Mfj,
        ordinary_taxable_income: dec!(120000.00),
        magi_excluding_crypto: dec!(130000.00),
        qualified_dividends_and_other_pref_income: dec!(0.00),
        other_net_capital_gain: dec!(0.00),
        capital_loss_carryforward_in: Carryforward { short: dec!(0.00), long: dec!(0.00) },
    };
    let json = serde_json::to_string(&p).unwrap();
    let back: TaxProfile = serde_json::from_str(&json).unwrap();
    assert_eq!(p, back);
}

#[test]
fn optional_profile_fields_default_to_zero() {
    // Older/minimal stored profiles omit the optional fields → serde-default to ZERO.
    let json = r#"{"filing_status":"Single","ordinary_taxable_income":"50000",
                   "magi_excluding_crypto":"50000","qualified_dividends_and_other_pref_income":"0"}"#;
    let p: TaxProfile = serde_json::from_str(json).unwrap();
    assert_eq!(p.other_net_capital_gain, Usd::ZERO);
    assert_eq!(p.capital_loss_carryforward_in, Carryforward::default());
}
```
2. **Run → RED** (the 3 `BlockerKind`s, the `tax` module, and the types do not exist).
3. **Minimal impl:**
   - `state.rs`: add the three variants to `BlockerKind` (after `Pre2025MethodConflictsAllocation`, `state.rs:45`) with doc-cites, and add them to the `Hard` arm of `severity()` (`state.rs:50-60`):
```rust
/// §B.4: the projection carries an unresolved Hard blocker (severity()==Hard) anywhere, so B refuses to
/// present a number for the year (projection-wide gate, B-I1). Carries the offending kind + EventId. Hard.
TaxYearNotComputable,
/// §B.1: no `tax_profile` is set for the year being computed. Hard — B does not guess the surrounding
/// tax context.
TaxProfileMissing,
/// §B.2: no bundled tax table is available for the year being computed. Hard.
TaxTableMissing,
```
   …and in `severity()` add `| TaxYearNotComputable | TaxProfileMissing | TaxTableMissing` to the `=> Severity::Hard` arm.
   - `tax/types.rs`: the structs/enums above. `Carryforward` derives `Default` (the `Usd::ZERO` default holds because `Decimal::default() == 0`). `TaxProfile`'s two optional fields use `#[serde(default)]`.
   - `tax/mod.rs`: `pub mod types;` + the re-export line above.
   - `lib.rs`: add `pub mod tax;` (after `pub mod state;`, `lib.rs:9`) and `pub use tax::{Carryforward, FilingStatus, MarginalRates, TaxOutcome, TaxProfile, TaxResult};` (after the `project::{…}` block, `lib.rs:15-19`).
4. **Run → GREEN.** Whole suite (additive; nothing else references the new variants yet).
5. **Commit:** `feat(core): tax module skeleton (TaxProfile/TaxResult/TaxOutcome) + 3 hard blockers`.

---

## TASK 2 — Tax-table types + `TaxTables` trait + statutory constants

**Goal.** Define the **indexed** per-year table types and the `TaxTables` lookup trait, plus the **statutory (non-indexed)** NIIT thresholds, NIIT rate, and loss limit as **year-independent** functions/consts with statute cites. Lock the statutory-vs-indexed separation with a KAT and provide a `BTreeMap`-backed test-double `TaxTables` for later tasks. **No bundled real numbers here** (Task 6).

**Files**
- new `crates/btctax-core/src/tax/tables.rs`
- modify `crates/btctax-core/src/tax/mod.rs` (`pub mod tables;` + re-exports)

**Interfaces**
```rust
// tax/tables.rs
use crate::conventions::Usd;
use crate::tax::types::FilingStatus;
use rust_decimal_macros::dec;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrdinaryBracket { pub lower: Usd, pub rate: Usd } // rate applies to income in [lower, next.lower)

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrdinarySchedule { pub brackets: Vec<OrdinaryBracket> } // ascending by `lower`; last is open-ended

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LtcgBreakpoints { pub max_zero: Usd, pub max_fifteen: Usd } // §1(h): top of 0% / top of 15%

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaxTable {
    pub year: i32,
    pub source: &'static str,                                 // e.g. "Rev. Proc. 2024-40 §2.01/§2.03 (TY2025)"
    pub ordinary: BTreeMap<FilingStatus, OrdinarySchedule>,   // INDEXED only
    pub ltcg: BTreeMap<FilingStatus, LtcgBreakpoints>,        // INDEXED only — NEVER NIIT/loss-limit
}
impl TaxTable {
    /// §1(h)/§1: a Qualifying Surviving Spouse uses the MFJ schedule/breakpoints.
    fn key(status: FilingStatus) -> FilingStatus {
        match status { FilingStatus::Qss => FilingStatus::Mfj, s => s }
    }
    pub fn ordinary_for(&self, status: FilingStatus) -> &OrdinarySchedule {
        &self.ordinary[&Self::key(status)]
    }
    pub fn ltcg_for(&self, status: FilingStatus) -> &LtcgBreakpoints {
        &self.ltcg[&Self::key(status)]
    }
}

pub trait TaxTables { fn table_for(&self, year: i32) -> Option<&TaxTable>; }

// ── STATUTORY, NOT inflation-indexed — year-independent (I4 / Global Constraints) ───────────────
/// §1411(a): the NIIT rate. STATUTORY (26 U.S.C. §1411(a)). Never indexed.
pub const NIIT_RATE: Usd = dec!(0.038);
/// §1411(b): the MAGI threshold. STATUTORY (26 U.S.C. §1411(b)); NOT inflation-indexed — these dollar
/// amounts are fixed in the Code and do not move year-over-year. Never placed in a `TaxTable`.
pub fn niit_threshold(status: FilingStatus) -> Usd {
    match status {
        FilingStatus::Mfj | FilingStatus::Qss => dec!(250000),
        FilingStatus::Single | FilingStatus::HoH => dec!(200000),
        FilingStatus::Mfs => dec!(125000),
    }
}
/// §1211(b): the capital-loss ordinary-offset limit. STATUTORY (26 U.S.C. §1211(b)); NOT indexed.
pub fn loss_limit(status: FilingStatus) -> Usd {
    match status { FilingStatus::Mfs => dec!(1500), _ => dec!(3000) }
}
```

**Steps**

1. **Failing tests** — `tax/tables.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// A tiny test-double table builder reused by Tasks 3–5 (synthetic, hand-chosen numbers).
    pub(crate) fn synthetic_table(year: i32) -> TaxTable {
        let mut ordinary = BTreeMap::new();
        ordinary.insert(FilingStatus::Single, OrdinarySchedule { brackets: vec![
            OrdinaryBracket { lower: dec!(0),      rate: dec!(0.10) },
            OrdinaryBracket { lower: dec!(10000),  rate: dec!(0.22) },
            OrdinaryBracket { lower: dec!(100000), rate: dec!(0.32) },
        ]});
        let mut ltcg = BTreeMap::new();
        ltcg.insert(FilingStatus::Single, LtcgBreakpoints { max_zero: dec!(40000), max_fifteen: dec!(400000) });
        TaxTable { year, source: "SYNTHETIC", ordinary, ltcg }
    }

    /// STATUTORY values are constant across years while indexed values move (I4 KAT).
    #[test]
    fn statutory_values_are_constant_across_years() {
        for status in [FilingStatus::Single, FilingStatus::Mfj, FilingStatus::Mfs,
                       FilingStatus::HoH, FilingStatus::Qss] {
            assert_eq!(niit_threshold(status), niit_threshold(status)); // year-independent by construction
        }
        assert_eq!(niit_threshold(FilingStatus::Mfj), dec!(250000));
        assert_eq!(niit_threshold(FilingStatus::Qss), dec!(250000));
        assert_eq!(niit_threshold(FilingStatus::Single), dec!(200000));
        assert_eq!(niit_threshold(FilingStatus::HoH), dec!(200000));
        assert_eq!(niit_threshold(FilingStatus::Mfs), dec!(125000));
        assert_eq!(NIIT_RATE, dec!(0.038));
        assert_eq!(loss_limit(FilingStatus::Mfs), dec!(1500));
        assert_eq!(loss_limit(FilingStatus::Single), dec!(3000));
    }

    /// QSS aliases MFJ for the indexed lookups too.
    #[test]
    fn qss_uses_mfj_schedule() {
        let mut t = synthetic_table(2025);
        // give MFJ a distinct schedule; QSS must resolve to it
        t.ordinary.insert(FilingStatus::Mfj, OrdinarySchedule { brackets: vec![
            OrdinaryBracket { lower: dec!(0), rate: dec!(0.10) },
            OrdinaryBracket { lower: dec!(50000), rate: dec!(0.22) },
        ]});
        t.ltcg.insert(FilingStatus::Mfj, LtcgBreakpoints { max_zero: dec!(80000), max_fifteen: dec!(500000) });
        assert_eq!(t.ordinary_for(FilingStatus::Qss).brackets, t.ordinary_for(FilingStatus::Mfj).brackets);
        assert_eq!(*t.ltcg_for(FilingStatus::Qss), *t.ltcg_for(FilingStatus::Mfj));
    }
}
```
2. **Run → RED** (types/trait/statutory fns absent).
3. **Minimal impl:** the structs/trait/consts/fns above. `tax/mod.rs`: `pub mod tables;` + `pub use tables::{niit_threshold, loss_limit, LtcgBreakpoints, OrdinaryBracket, OrdinarySchedule, TaxTable, TaxTables, NIIT_RATE};`. Make `synthetic_table` reachable to sibling test modules via `pub(crate)` + a `#[cfg(test)] pub(crate) mod test_support { pub use super::tests::synthetic_table; }` (or move `synthetic_table` to a `#[cfg(test)] pub(crate) fn` at module scope) so Tasks 3–5 reuse it without duplication.
4. **Run → GREEN.** Whole suite.
5. **Commit:** `feat(core): tax-table types + TaxTables trait + STATUTORY NIIT/loss-limit constants (cited, non-indexed)`.

---

## TASK 3 — Rate-application primitives

**Goal.** Two pure, exact-`Decimal` functions: ordinary marginal-bracket tax, and §1(h) preferential (0/15/20) stacking. These are the arithmetic core; KATs hand-verify each at bracket boundaries.

**Files**
- new `crates/btctax-core/src/tax/compute.rs`
- modify `crates/btctax-core/src/tax/mod.rs` (`pub mod compute;` + re-exports)

**Interfaces**
```rust
// tax/compute.rs
use crate::conventions::{round_cents, Usd};
use crate::tax::tables::{LtcgBreakpoints, OrdinarySchedule};
use rust_decimal::Decimal;

/// Exact marginal-bracket tax on `taxable` (≥ 0). Sums (min(taxable, next_lower) − lower) × rate over each
/// bracket the income reaches; the open-ended top bracket has no upper bound. ROUND_HALF_EVEN to cents at
/// the END only (NFR5). NOT the IRS binned Tax Tables and NOT whole-dollar rounding — the exact formula
/// method at cent precision (deliberate determinism/exactness choice).
pub fn ordinary_tax_on(schedule: &OrdinarySchedule, taxable: Usd) -> Usd {
    if taxable <= Usd::ZERO { return Usd::ZERO; }
    let b = &schedule.brackets;
    let mut tax = Usd::ZERO;
    for (i, br) in b.iter().enumerate() {
        if taxable <= br.lower { break; }
        let upper = b.get(i + 1).map(|n| n.lower).unwrap_or(taxable); // open-ended top
        let span_top = if taxable < upper { taxable } else { upper };
        tax += (span_top - br.lower) * br.rate;
    }
    round_cents(tax)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrefSplit { pub at_0: Usd, pub at_15: Usd, pub at_20: Usd, pub tax: Usd }

/// §1(h) stacking: preferential income `pref` (= QD + net LT gain) sits ON TOP of `bottom` (ordinary
/// taxable income incl. net ST gain). Breakpoints are compared against TOTAL taxable income (bottom+pref);
/// ordinary income fills the bottom of the stack first. Exact Decimal; ROUND_HALF_EVEN at the end.
pub fn preferential_tax(bp: &LtcgBreakpoints, bottom: Usd, pref: Usd) -> PrefSplit {
    let z = Usd::ZERO;
    if pref <= z {
        return PrefSplit { at_0: z, at_15: z, at_20: z, tax: z };
    }
    let bottom = if bottom < z { z } else { bottom };
    let top = bottom + pref;
    let clamp = |v: Usd| if v < z { z } else { v };
    // 0% zone: pref dollars below max_zero
    let at_0 = {
        let room = clamp(bp.max_zero - bottom);
        if room < pref { room } else { pref }
    };
    // 15% zone: (max_zero, max_fifteen]
    let lower15 = if bottom > bp.max_zero { bottom } else { bp.max_zero };
    let upper15 = if top < bp.max_fifteen { top } else { bp.max_fifteen };
    let at_15 = clamp(upper15 - lower15);
    let at_20 = pref - at_0 - at_15; // remainder above max_fifteen
    let tax = round_cents(at_15 * dec_15() + at_20 * dec_20());
    PrefSplit { at_0, at_15, at_20, tax }
}
fn dec_15() -> Usd { rust_decimal_macros::dec!(0.15) }
fn dec_20() -> Usd { rust_decimal_macros::dec!(0.20) }
```

**Steps**

1. **Failing tests** — `tax/compute.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tax::tables::{LtcgBreakpoints, OrdinaryBracket, OrdinarySchedule};
    use rust_decimal_macros::dec;

    fn sched() -> OrdinarySchedule {
        OrdinarySchedule { brackets: vec![
            OrdinaryBracket { lower: dec!(0),     rate: dec!(0.10) },
            OrdinaryBracket { lower: dec!(10000), rate: dec!(0.20) },
            OrdinaryBracket { lower: dec!(40000), rate: dec!(0.30) },
        ]}
    }

    #[test]
    fn ordinary_tax_sums_marginal_brackets_exactly() {
        // 0 → 0
        assert_eq!(ordinary_tax_on(&sched(), dec!(0)), dec!(0.00));
        // exactly at a boundary: $10,000 all at 10% = $1,000.00
        assert_eq!(ordinary_tax_on(&sched(), dec!(10000)), dec!(1000.00));
        // $25,000 = 10%·10,000 + 20%·15,000 = 1,000 + 3,000 = 4,000.00
        assert_eq!(ordinary_tax_on(&sched(), dec!(25000)), dec!(4000.00));
        // into the open-ended top: $50,000 = 1,000 + 20%·30,000(=6,000) + 30%·10,000(=3,000) = 10,000.00
        assert_eq!(ordinary_tax_on(&sched(), dec!(50000)), dec!(10000.00));
    }

    fn bp() -> LtcgBreakpoints { LtcgBreakpoints { max_zero: dec!(48350), max_fifteen: dec!(533400) } }

    #[test]
    fn preferential_zero_then_fifteen() {
        // bottom 40,000 ordinary, pref 20,000 LT → 8,350 @ 0%, 11,650 @ 15% = 1,747.50
        let s = preferential_tax(&bp(), dec!(40000), dec!(20000));
        assert_eq!(s.at_0, dec!(8350));
        assert_eq!(s.at_15, dec!(11650));
        assert_eq!(s.at_20, dec!(0));
        assert_eq!(s.tax, dec!(1747.50));
    }

    #[test]
    fn preferential_fifteen_then_twenty() {
        // bottom 500,000 ordinary, pref 100,000 → 33,400 @ 15% + 66,600 @ 20% = 5,010 + 13,320 = 18,330.00
        let s = preferential_tax(&bp(), dec!(500000), dec!(100000));
        assert_eq!(s.at_0, dec!(0));
        assert_eq!(s.at_15, dec!(33400));
        assert_eq!(s.at_20, dec!(66600));
        assert_eq!(s.tax, dec!(18330.00));
    }

    #[test]
    fn preferential_all_zero_when_under_max_zero() {
        // bottom 10,000, pref 20,000, top 30,000 < 48,350 → all 0%
        let s = preferential_tax(&bp(), dec!(10000), dec!(20000));
        assert_eq!(s.at_0, dec!(20000));
        assert_eq!(s.tax, dec!(0.00));
    }

    #[test]
    fn preferential_zero_pref_is_zero_tax() {
        assert_eq!(preferential_tax(&bp(), dec!(100000), dec!(0)).tax, dec!(0.00));
    }
}
```
2. **Run → RED** (`ordinary_tax_on`/`preferential_tax`/`PrefSplit` absent).
3. **Minimal impl:** the two fns + `PrefSplit` above. `tax/mod.rs`: `pub mod compute;` + `pub use compute::{ordinary_tax_on, preferential_tax, PrefSplit};`. (`net_1222`/`compute_tax_year` appended in Tasks 4–5.)
4. **Run → GREEN.** Whole suite.
5. **Commit:** `feat(core): exact-Decimal ordinary marginal-bracket + §1(h) 0/15/20 preferential stacking`.

---

## TASK 4 — §1222 netting + §1211/§1212 loss limit (`net_1222` / `CapNet`)

**Goal.** One pure function that takes the year's crypto ST/LT sums + the profile's non-crypto LT + carryforward-in (by character) and produces: the §1222 within-character nets (`st_net`, `lt_net`), the cross-netted **taxable** gains (`ordinary_gain` = surviving net ST gain at ordinary rates; `preferential_gain` = surviving net capital gain for §1(h)), and — in a net-loss year — the §1211 `loss_deduction` plus the §1212(b) **ST-first**, character-preserving `st_carry`/`lt_carry`. This is run **twice** by Task 5 (with crypto / without crypto) to form the delta.

**Files**
- modify `crates/btctax-core/src/tax/compute.rs` (add `CapNet` + `net_1222` + tests)
- modify `crates/btctax-core/src/tax/mod.rs` (re-export `CapNet`, `net_1222`)

**Interfaces**
```rust
// tax/compute.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapNet {
    pub st_net: Usd,           // §1222(5)/(6): within-character net short-term (signed; after cf_short)
    pub lt_net: Usd,           // §1222(7)/(8): within-character net long-term (signed; after other_lt & cf_long)
    pub ordinary_gain: Usd,    // net short-term gain surviving cross-net (≥0) → ordinary rates
    pub preferential_gain: Usd,// §1222(11) net capital gain surviving cross-net (≥0) → §1(h)
    pub loss_deduction: Usd,   // §1211(b) ordinary offset used this year (≥0)
    pub st_carry: Usd,         // §1212(b) short-term carryforward out (≥0)
    pub lt_carry: Usd,         // §1212(b) long-term carryforward out (≥0)
}

/// Inputs are signed: gains positive, losses negative for `crypto_st`/`crypto_lt`/`other_lt`.
/// `cf_short`/`cf_long` are prior-year carryforward LOSS magnitudes (≥0) — they REDUCE the matching
/// character. `loss_limit` is the statutory §1211(b) cap ($3,000 / $1,500 MFS).
pub fn net_1222(crypto_st: Usd, crypto_lt: Usd, other_lt: Usd,
                cf_short: Usd, cf_long: Usd, loss_limit: Usd) -> CapNet {
    let z = Usd::ZERO;
    // §1222(5)/(6): within-character net short-term (carryforward-in is a short-term loss → subtract).
    let st_net = crypto_st - cf_short;
    // §1222(7)/(8): within-character net long-term (other_net_capital_gain is LT-character; cf_long subtracts).
    let lt_net = crypto_lt + other_lt - cf_long;

    // Cross-net a gain in one character against a loss in the other (§1222 / Schedule D line 16).
    let (st2, lt2) = match (st_net >= z, lt_net >= z) {
        (true, true) | (false, false) => (st_net, lt_net), // both gains, or both losses: no cross-net
        (true, false) => {                                  // ST gain, LT loss
            if -lt_net <= st_net { (st_net + lt_net, z) } else { (z, st_net + lt_net) }
        }
        (false, true) => {                                  // ST loss, LT gain
            if -st_net <= lt_net { (z, lt_net + st_net) } else { (st_net + lt_net, z) }
        }
    };
    let ordinary_gain = if st2 > z { st2 } else { z };
    let preferential_gain = if lt2 > z { lt2 } else { z };
    let net_st_loss = if st2 < z { -st2 } else { z };
    let net_lt_loss = if lt2 < z { -lt2 } else { z };
    let net_loss = net_st_loss + net_lt_loss;

    // §1211(b) limit + §1212(b) ST-first absorption, character-preserving carryforward (M3).
    let loss_deduction = if net_loss < loss_limit { net_loss } else { loss_limit };
    let absorbed_st = if net_st_loss < loss_deduction { net_st_loss } else { loss_deduction };
    let absorbed_lt = loss_deduction - absorbed_st;
    CapNet {
        st_net, lt_net, ordinary_gain, preferential_gain, loss_deduction,
        st_carry: net_st_loss - absorbed_st,
        lt_carry: net_lt_loss - absorbed_lt,
    }
}
```

**Steps**

1. **Failing tests** — `tax/compute.rs`:
```rust
#[cfg(test)]
mod net_tests {
    use super::*;
    use rust_decimal_macros::dec;
    fn lim() -> Usd { dec!(3000) }

    #[test]
    fn both_gains_no_crossnet() {
        let n = net_1222(dec!(5000), dec!(8000), dec!(0), dec!(0), dec!(0), lim());
        assert_eq!(n.ordinary_gain, dec!(5000));
        assert_eq!(n.preferential_gain, dec!(8000));
        assert_eq!(n.loss_deduction, dec!(0));
    }

    #[test]
    fn within_character_then_crossnet_order() {
        // ST gain 10,000; LT loss 4,000 → LT loss offsets ST gain → net ST gain 6,000, no preferential.
        let n = net_1222(dec!(10000), dec!(-4000), dec!(0), dec!(0), dec!(0), lim());
        assert_eq!(n.st_net, dec!(10000));
        assert_eq!(n.lt_net, dec!(-4000));
        assert_eq!(n.ordinary_gain, dec!(6000));
        assert_eq!(n.preferential_gain, dec!(0));
        assert_eq!(n.loss_deduction, dec!(0));
    }

    #[test]
    fn st_loss_offsets_lt_gain_to_preferential() {
        // ST loss 3,000; LT gain 9,000 → net capital gain 6,000 (preferential), no ordinary.
        let n = net_1222(dec!(-3000), dec!(9000), dec!(0), dec!(0), dec!(0), lim());
        assert_eq!(n.ordinary_gain, dec!(0));
        assert_eq!(n.preferential_gain, dec!(6000));
    }

    #[test]
    fn loss_year_3k_limit_st_first_carryforward() {
        // ST loss 5,000; LT loss 2,000 → total loss 7,000; deduct 3,000 (ST-first); carry 2,000 ST + 2,000 LT.
        let n = net_1222(dec!(-5000), dec!(-2000), dec!(0), dec!(0), dec!(0), lim());
        assert_eq!(n.loss_deduction, dec!(3000));
        assert_eq!(n.st_carry, dec!(2000)); // §1212(b): the $3k came out of ST loss first
        assert_eq!(n.lt_carry, dec!(2000));
    }

    #[test]
    fn loss_limit_is_mfs_1500() {
        let n = net_1222(dec!(-5000), dec!(0), dec!(0), dec!(0), dec!(0), dec!(1500));
        assert_eq!(n.loss_deduction, dec!(1500));
        assert_eq!(n.st_carry, dec!(3500));
        assert_eq!(n.lt_carry, dec!(0));
    }

    #[test]
    fn multi_year_carryforward_preserves_character() {
        // Year 1: ST loss 5,000 + LT loss 2,000 → carry {short:2000, long:2000} (from prior test).
        let y1 = net_1222(dec!(-5000), dec!(-2000), dec!(0), dec!(0), dec!(0), lim());
        // Year 2: LT gain 10,000, no crypto ST; carry-in {short:2000, long:2000}.
        // st_net = 0 - 2000 = -2000; lt_net = 10000 - 2000 = 8000; cross-net: ST loss offsets LT gain →
        // preferential 6,000, no loss.
        let y2 = net_1222(dec!(0), dec!(10000), dec!(0), y1.st_carry, y1.lt_carry, lim());
        assert_eq!(y2.preferential_gain, dec!(6000));
        assert_eq!(y2.ordinary_gain, dec!(0));
        assert_eq!(y2.loss_deduction, dec!(0));
    }

    #[test]
    fn st_loss_only_3k_all_st_character() {
        let n = net_1222(dec!(-10000), dec!(0), dec!(0), dec!(0), dec!(0), lim());
        assert_eq!(n.loss_deduction, dec!(3000));
        assert_eq!(n.st_carry, dec!(7000));
        assert_eq!(n.lt_carry, dec!(0));
    }
}
```
2. **Run → RED** (`CapNet`/`net_1222` absent).
3. **Minimal impl:** the `CapNet` + `net_1222` above. `tax/mod.rs`: extend the `compute` re-export to add `CapNet, net_1222`.
4. **Run → GREEN.** Whole suite.
5. **Commit:** `feat(core): §1222 ST/LT netting + §1211/§1212(b) ST-first character-preserving carryforward`.

---

## TASK 5 — `compute_tax_year` (assembly + incremental delta + refusal)

**Goal.** Assemble Tasks 2–4 into the year computation: gather the year's crypto disposals (ST/LT) and crypto ordinary income; run `net_1222` **with** and **without** crypto; compute ordinary + §1(h) + §1411 NIIT in each scenario; produce the crypto-attributable deltas + the `TaxResult`. Gate it: any Hard blocker touching the year → `TaxYearNotComputable`; no table → `TaxTableMissing`; no profile → `TaxProfileMissing`. **Mechanics KATs use synthetic tables** (real-number goldens are Task 7).

**Files**
- modify `crates/btctax-core/src/tax/compute.rs` (add `compute_tax_year` + helpers + tests)
- modify `crates/btctax-core/src/tax/mod.rs` (re-export `compute_tax_year`)
- (no change) `lib.rs` already re-exports the `tax::*` items added in Task 1; add `compute_tax_year` to that list.

**Interfaces & algorithm**
```rust
// tax/compute.rs
use crate::event::LedgerEvent;
use crate::state::{Blocker, BlockerKind, LedgerState, Severity, Term};
use crate::tax::tables::{loss_limit, niit_threshold, TaxTables, NIIT_RATE};
use crate::tax::types::{Carryforward, FilingStatus, MarginalRates, TaxOutcome, TaxProfile, TaxResult};
// (B-I1) the projection-wide gate drops the per-event scoping, so `tax_date`/`EventId`/`BTreeMap` are no
// longer imported here; `EventId::canonical()` is still reached as a method on the carried `event`.

/// §B.3/§B.4. Returns `Computed(TaxResult)` or `NotComputable(Blocker)`.
/// Precedence of refusal (deterministic): any-Hard-blocker-in-projection → table-missing → profile-missing.
pub fn compute_tax_year(
    events: &[LedgerEvent],
    state: &LedgerState,
    year: i32,
    profile: Option<&TaxProfile>,
    tables: &dyn TaxTables,
) -> TaxOutcome {
    // `events` is retained in the signature for the §0 determinism-tuple / spec parity and a future
    // per-year lot-lineage refinement (Option 2); the projection-wide gate below consults only `state`.
    let _ = events;
    // (1) §B.4 refusal (B-I1): ANY unresolved severity()==Hard blocker ANYWHERE in the projection gates
    // EVERY year's computation — deliberately conservative. The earlier per-event/per-year enumeration
    // UNDER-gated cross-year basis contamination: an out-of-year unresolved `ImportConflict` leaves the
    // disputed-basis lot in the pool (`resolve.rs:362-377`) and a basis-affecting `DecisionConflict`
    // (`resolve.rs:388-395`) can postdate the disposal it taints; neither lot is `basis_pending`, so neither
    // re-triggers an in-year `FmvMissing` (`fold.rs:124-131`) — a later disposal would then consume the
    // contaminated lot and B would emit an authoritative-but-wrong number. Any open Hard blocker ⇒ the
    // basis foundation is unsound ⇒ refuse. (Re-narrow to per-year only via Option-2 lot-lineage + KATs.)
    if let Some(b) = first_hard_blocker(state) {
        let evt = b.event.as_ref().map(|e| e.canonical()).unwrap_or_else(|| "-".into());
        return TaxOutcome::NotComputable(Blocker {
            kind: BlockerKind::TaxYearNotComputable,
            event: b.event.clone(), // B-N1: carry the structured offending EventId for downstream C
            detail: format!("year {year} not computable: unresolved Hard blocker [{:?}] {} :: {}",
                b.kind, evt, b.detail),
        });
    }
    // (2) §B.2 missing table.
    let Some(table) = tables.table_for(year) else {
        return TaxOutcome::NotComputable(Blocker {
            kind: BlockerKind::TaxTableMissing, event: None,
            detail: format!("no bundled tax table for {year}"),
        });
    };
    // (3) §B.1 missing profile.
    let Some(profile) = profile else {
        return TaxOutcome::NotComputable(Blocker {
            kind: BlockerKind::TaxProfileMissing, event: None,
            detail: format!("no tax_profile set for {year}"),
        });
    };

    let status = profile.filing_status;
    let limit = loss_limit(status);
    let sched = table.ordinary_for(status);
    let bp = *table.ltcg_for(status);
    let thr = niit_threshold(status);

    // ── crypto inputs for the year ──────────────────────────────────────────────────────────────
    let mut crypto_st = Usd::ZERO;
    let mut crypto_lt = Usd::ZERO;
    for d in state.disposals.iter().filter(|d| d.disposed_at.year() == year) {
        for leg in &d.legs {
            match leg.term { Term::ShortTerm => crypto_st += leg.gain, Term::LongTerm => crypto_lt += leg.gain }
        }
    }
    // Crypto ordinary income (mining/staking/interest/airdrop/reward): all IncomeKind are ordinary at FMV.
    let crypto_ord: Usd = state.income_recognized.iter()
        .filter(|i| i.recognized_at.year() == year).map(|i| i.usd_fmv).sum();

    let cf = profile.capital_loss_carryforward_in;
    // ── two scenarios ───────────────────────────────────────────────────────────────────────────
    let with = net_1222(crypto_st, crypto_lt, profile.other_net_capital_gain, cf.short, cf.long, limit);
    let without = net_1222(Usd::ZERO, Usd::ZERO, profile.other_net_capital_gain, cf.short, cf.long, limit);

    let qd = profile.qualified_dividends_and_other_pref_income;
    let scen = |cap: &CapNet, add_ord: Usd| -> (Usd, Usd, Usd) {
        // returns (ordinary_tax, preferential_tax, niit) for a scenario.
        let bottom = profile.ordinary_taxable_income + add_ord + cap.ordinary_gain - cap.loss_deduction;
        let ord_tax = ordinary_tax_on(sched, bottom);
        let pref_tax = preferential_tax(&bp, bottom, qd + cap.preferential_gain).tax;
        (ord_tax, pref_tax, bottom, /* niit computed by caller using bottom */)
        // NOTE: real impl returns a small struct; pseudo-tuple here for readability.
    };
    // (real impl) compute bottoms + NII + MAGI explicitly:
    let bottom_with = profile.ordinary_taxable_income + crypto_ord + with.ordinary_gain - with.loss_deduction;
    let bottom_without = profile.ordinary_taxable_income + without.ordinary_gain - without.loss_deduction;
    let ord_with = ordinary_tax_on(sched, bottom_with);
    let ord_without = ordinary_tax_on(sched, bottom_without);
    let pref_with = preferential_tax(&bp, bottom_with, qd + with.preferential_gain).tax;
    let pref_without = preferential_tax(&bp, bottom_without, qd + without.preferential_gain).tax;

    // §1411 NIIT. NII = QD + surviving net capital gains (ST+LT). Crypto ordinary income is NOT NII
    // (mining/staking is trade/business or otherwise outside the minimal NII model). B-M1: this minimal NII
    // model can only ever **understate** NIIT — investor-level staking/rewards/airdrops could be NII, and NII
    // is not reduced by the allowed §1211 loss in a net-loss year. Recorded as a Phase-2 refinement (§5).
    let nii_with = qd + with.ordinary_gain + with.preferential_gain;
    let nii_without = qd + without.ordinary_gain + without.preferential_gain;
    // MAGI_excluding_crypto already includes QD + non-crypto cap gain; add ONLY the crypto AGI contribution.
    let crypto_agi = (with.ordinary_gain + with.preferential_gain - with.loss_deduction)
        - (without.ordinary_gain + without.preferential_gain - without.loss_deduction)
        + crypto_ord;
    let magi_without = profile.magi_excluding_crypto;
    let magi_with = magi_without + crypto_agi;
    let niit = |nii: Usd, magi: Usd| -> Usd {
        let over = if magi > thr { magi - thr } else { Usd::ZERO };
        let base = if nii < over { nii } else { over };
        round_cents(base * NIIT_RATE)
    };
    let niit_with = niit(nii_with, magi_with);
    let niit_without = niit(nii_without, magi_without);

    let total = (ord_with + pref_with + niit_with) - (ord_without + pref_without + niit_without);

    let top = bottom_with + qd + with.preferential_gain;
    let marginal_rates = MarginalRates {
        ordinary: marginal_ordinary_rate(sched, bottom_with),
        ltcg: if top <= bp.max_zero { Usd::ZERO }
              else if top <= bp.max_fifteen { dec_15() } else { dec_20() },
        niit_applies: niit_with > niit_without,
    };

    TaxOutcome::Computed(TaxResult {
        st_net: with.st_net,
        lt_net: with.lt_net,
        ordinary_from_crypto: crypto_ord,
        ltcg_tax: pref_with - pref_without,    // crypto-attributable preferential tax (DELTA)
        niit: niit_with - niit_without,        // crypto-attributable NIIT (DELTA)
        loss_deduction: with.loss_deduction,   // WITH-scenario level (drives carryforward_out)
        carryforward_out: Carryforward { short: with.st_carry, long: with.lt_carry },
        total_federal_tax_attributable: total, // = (ord_with-ord_without) + ltcg_tax + niit
        marginal_rates,
    })
}

/// Highest ordinary bracket rate the income reaches (the rate on its last dollar).
fn marginal_ordinary_rate(sched: &OrdinarySchedule, taxable: Usd) -> Usd {
    let mut r = Usd::ZERO;
    for br in &sched.brackets { if taxable > br.lower { r = br.rate; } else { break; } }
    r
}

/// §B.4 (B-I1): the projection-wide Hard-blocker gate. Returns the FIRST unresolved blocker whose
/// `severity() == Severity::Hard`, anywhere in `state.blockers`. Deliberately conservative — any open Hard
/// blocker means the basis foundation is unsound, so EVERY year's computation refuses until it is resolved.
/// This closes the cross-year basis-contamination hole the earlier per-event/3-kind enumeration left open
/// (an out-of-year unresolved `ImportConflict` — `resolve.rs:362-377` — leaves the disputed-basis lot in the
/// pool; a basis-affecting `DecisionConflict` — `resolve.rs:388-395` — can postdate the disposal it taints;
/// neither is `basis_pending`, so neither re-triggers an in-year `FmvMissing`, cf. `fold.rs:124-131`).
/// `state.blockers` iteration is the deterministic projection order (NFR4) and `.find` returns the first,
/// so the chosen blocker — hence the `TaxYearNotComputable` detail/event — is deterministic.
fn first_hard_blocker(state: &LedgerState) -> Option<&Blocker> {
    state.blockers.iter().find(|b| b.kind.severity() == Severity::Hard)
}
```

> **Resolved-ambiguity note (carried into §4.4).** `ltcg_tax`, `niit`, and `total_federal_tax_attributable` are **crypto-attributable deltas**; `st_net`, `lt_net`, `ordinary_from_crypto`, `loss_deduction`, `carryforward_out`, and `marginal_rates` describe the **WITH-crypto filing position** (`carryforward_out` MUST be a level — it feeds next year's `carryforward_in`). A "level" `ltcg_tax` for crypto LT is **ill-defined** when QD shares the §1(h) stack (which dollars are "the QD" vs "the crypto LT"?); the delta `pref_tax(with) − pref_tax(without)` is unambiguous and is exactly "the additional preferential tax the crypto LT caused." The ordinary-rate piece of `total` (= `ord_with − ord_without`) is intentionally **not** a named field (the spec's `TaxResult` does not name it); a KAT pins the identity `total == (ord_with − ord_without) + ltcg_tax + niit`. **B-M2:** the report (Task 9) nonetheless **surfaces** that ordinary-rate delta as a labeled line, derived as `total − ltcg_tax − niit` (exactly `ord_with − ord_without` by the pinned identity, with no extra rounding), so the printed attributable components visibly reconcile to `total`. **B-M4 (display-only):** `marginal_rates.ordinary` uses `taxable > br.lower`, so exactly at a bracket boundary it reports the **lower** bracket's rate; `marginal_rates.ltcg` reflects the top-of-stack LTCG rate of the WITH-crypto position even when the crypto has no preferential income — both are display conventions with **no effect on any tax figure**.

**Steps**

1. **Failing tests** — new `crates/btctax-core/tests/tax_compute.rs` (integration test crate; mirrors `kat_tax.rs` helpers `ev`/`StaticPrices`):
```rust
use btctax_core::price::StaticPrices;
use btctax_core::project::{project, ProjectionConfig};
use btctax_core::tax::compute::compute_tax_year;
use btctax_core::tax::tables::{LtcgBreakpoints, OrdinaryBracket, OrdinarySchedule, TaxTable, TaxTables};
use btctax_core::tax::types::{Carryforward, FilingStatus, TaxOutcome, TaxProfile};
use btctax_core::{BlockerKind, /* … */};
use rust_decimal_macros::dec;
use std::collections::BTreeMap;
use time::macros::{datetime, offset};

// flat-22% ordinary; LT 0% to 40k, 15% to 400k — synthetic, hand-pickable.
struct OneTable(TaxTable);
impl TaxTables for OneTable {
    fn table_for(&self, year: i32) -> Option<&TaxTable> { (year == self.0.year).then_some(&self.0) }
}
fn synth(year: i32) -> OneTable {
    let mut ordinary = BTreeMap::new();
    ordinary.insert(FilingStatus::Single, OrdinarySchedule { brackets: vec![
        OrdinaryBracket { lower: dec!(0),      rate: dec!(0.10) },
        OrdinaryBracket { lower: dec!(50000),  rate: dec!(0.22) },
        OrdinaryBracket { lower: dec!(250000), rate: dec!(0.32) },
    ]});
    let mut ltcg = BTreeMap::new();
    ltcg.insert(FilingStatus::Single, LtcgBreakpoints { max_zero: dec!(40000), max_fifteen: dec!(400000) });
    OneTable(TaxTable { year, source: "SYNTHETIC", ordinary, ltcg })
}
fn profile(ord: Usd, magi: Usd, qd: Usd) -> TaxProfile {
    TaxProfile {
        filing_status: FilingStatus::Single,
        ordinary_taxable_income: ord, magi_excluding_crypto: magi,
        qualified_dividends_and_other_pref_income: qd,
        other_net_capital_gain: dec!(0),
        capital_loss_carryforward_in: Carryforward { short: dec!(0), long: dec!(0) },
    }
}
// helpers `buy`/`sell`/`mine` build synthetic events with StaticPrices (see kat_tax.rs:16-34 pattern).

#[test]
fn double_count_guard_crypto_ordinary_income_added_exactly_once() {
    // $10,000 mining income, OTI 60,000 (already at the 22% margin in synth). No disposals.
    // ordinary_delta = tax(70,000) - tax(60,000) = 0.22*10,000 = 2,200.00 ; no LT, no NIIT.
    let events = vec![/* mine 10k FMV in `year`, StaticPrices priced */];
    let st = project(&events, &priced_for(&events), &ProjectionConfig::default());
    let out = compute_tax_year(&events, &st, 2025, Some(&profile(dec!(60000), dec!(60000), dec!(0))), &synth(2025));
    let TaxOutcome::Computed(r) = out else { panic!("computable") };
    assert_eq!(r.ordinary_from_crypto, dec!(10000.00));
    assert_eq!(r.total_federal_tax_attributable, dec!(2200.00)); // counted ONCE (not 4,400)
    assert_eq!(r.ltcg_tax, dec!(0.00));
    assert_eq!(r.niit, dec!(0.00));
}

#[test]
fn st_gain_stacks_on_ordinary() {
    // OTI 40,000 (10% band top is 50,000); crypto ST gain 20,000 → bottom 60,000 crosses 10%→22%.
    // ord_with = 10%*50,000 + 22%*10,000 = 5,000 + 2,200 = 7,200 ; ord_without = 10%*40,000 = 4,000.
    // delta = 3,200.00 ; no LT, no NIIT (magi 60k < 200k).
    let out = /* one ST sell with gain 20,000 in 2025 */;
    let TaxOutcome::Computed(r) = out else { panic!() };
    assert_eq!(r.st_net, dec!(20000)); assert_eq!(r.lt_net, dec!(0));
    assert_eq!(r.total_federal_tax_attributable, dec!(3200.00));
    assert_eq!(r.marginal_rates.ordinary, dec!(0.22));
}

#[test]
fn niit_threshold_crossing() {
    // OTI/magi 190,000; crypto LT gain 30,000 → magi_with 220,000, crosses 200,000 by 20,000.
    // NII_with = 30,000 ; niit = 3.8% * min(30,000, 20,000)=20,000 → 760.00. niit_without = 0.
    // (synth LT: bottom 190,000 > max_zero 40,000, top 220,000 < max_fifteen 400,000 → all 15%.)
    let out = /* one LT sell gain 30,000 in 2025, profile(190000,190000,0) */;
    let TaxOutcome::Computed(r) = out else { panic!() };
    assert_eq!(r.lt_net, dec!(30000));
    assert_eq!(r.ltcg_tax, dec!(4500.00));   // 15% * 30,000
    assert_eq!(r.niit, dec!(760.00));
    assert!(r.marginal_rates.niit_applies);
    // identity: total == ordinary_delta(0) + ltcg_tax + niit
    assert_eq!(r.total_federal_tax_attributable, r.ltcg_tax + r.niit);
}

#[test]
fn refuses_year_with_hard_blocker() {
    // A disposal in 2025 missing FMV → FmvMissing (Hard) on that event → TaxYearNotComputable.
    let out = compute_tax_year(&events, &st_with_fmv_missing, 2025, Some(&profile(dec!(50000), dec!(50000), dec!(0))), &synth(2025));
    assert!(matches!(out, TaxOutcome::NotComputable(b) if b.kind == BlockerKind::TaxYearNotComputable));
}

#[test]
fn refuses_year_with_out_of_year_import_conflict_on_consumed_lot() {
    // B-I1: an UNRESOLVED 2024 ImportConflict leaves the disputed-basis lot in the pool
    // (`resolve.rs:362-377`); a 2025 disposal consumes it. The lot is NOT basis_pending (no in-year
    // FmvMissing re-trigger, `fold.rs:124-131`) and the ImportConflict's event-year is 2024 ≠ 2025 — so
    // under the OLD per-event/3-kind enumeration 2025 would have computed an AUTHORITATIVE-BUT-WRONG number
    // off the disputed basis. The projection-wide gate must refuse 2025 instead (not a wrong number).
    // Fixture: Acquire(2024) → ImportConflict(2024, unresolved) targeting it → Dispose(2025) consuming it.
    let out = compute_tax_year(&events, &st_unresolved_import_conflict_2024, 2025,
        Some(&profile(dec!(50000), dec!(50000), dec!(0))), &synth(2025));
    assert!(matches!(out, TaxOutcome::NotComputable(b)
        if b.kind == BlockerKind::TaxYearNotComputable));
}

#[test]
fn missing_table_then_profile_blockers() {
    let st = /* clean projection, no disposals */;
    // wrong year → no table
    assert!(matches!(compute_tax_year(&[], &st, 2099, Some(&profile(dec!(1),dec!(1),dec!(0))), &synth(2025)),
        TaxOutcome::NotComputable(b) if b.kind == BlockerKind::TaxTableMissing));
    // right year, no profile
    assert!(matches!(compute_tax_year(&[], &st, 2025, None, &synth(2025)),
        TaxOutcome::NotComputable(b) if b.kind == BlockerKind::TaxProfileMissing));
}

#[test]
fn determinism_same_inputs_same_outcome() {
    let a = compute_tax_year(&events, &st, 2025, Some(&p), &synth(2025));
    let b = compute_tax_year(&events, &st, 2025, Some(&p), &synth(2025));
    assert_eq!(a, b);
}
```
   *(The `/* … */` placeholders are fixture-construction scaffolding: the implementer builds the concrete synthetic `Acquire`/`Dispose`/`Income` events with `StaticPrices` per the `kat_tax.rs:16-34` helper pattern. Each test's asserted behavior + hand-computed golden is fully specified above.)*
2. **Run → RED** (`compute_tax_year` absent / module path).
3. **Minimal impl:** `compute_tax_year` + `marginal_ordinary_rate` + `first_hard_blocker` as above (drop the readability pseudo-`scen` closure; keep the explicit bottoms). `tax/mod.rs`: add `compute_tax_year` (and `CapNet` from Task 4) to the `compute` re-export; `lib.rs`: add `compute_tax_year` to the `tax::{…}` re-export. Note `state.disposals`/`income_recognized` (`state.rs:180,182`), `DisposalLeg.{gain,term}` (`state.rs:101,102`), `IncomeRecord.{recognized_at,usd_fmv}` (`state.rs:143,145`), `Blocker`/`BlockerKind`/`Severity` (`state.rs:17-70`) + `BlockerKind::severity()` (`state.rs:47-64`), `EventId::canonical()` reused. The projection-wide gate (B-I1) drops the per-event `tax_date`/`EventId`/`BTreeMap` scoping, so those imports are removed.
4. **Run → GREEN.** Whole suite.
5. **Commit:** `feat(core): compute_tax_year — incremental crypto-attributable federal-tax delta + hard-blocker refusal`.

---

## TASK 6 — Bundled per-year tax tables (adapters)

**Goal.** Provide `BundledTaxTables` (mirroring `BundledPrices`) with the **TY2025** indexed numbers encoded from **Rev. Proc. 2024-40** (and TY2026 from **Rev. Proc. 2025-32** *only if* verified at write time). Unlike the flat daily-close CSV, bracket schedules are nested per-status, so each year is a typed Rust constructor (still compiled-in, pure, deterministic — the load-invariant of the price pattern), each with a `source` cite. Pin-the-data KATs assert the exact bundled numbers; a KAT re-asserts the statutory values are constant across years.

> **Verification gate (binding, do at write time).** Re-verify EVERY value below against the **primary PDF** — Rev. Proc. 2024-40 **§2.01** (tax rate tables under §1(j)(2)) and **§2.03** (Maximum Capital Gains Rate under §1(h)) — `https://www.irs.gov/pub/irs-drop/rp-24-40.pdf`. **OBBBA note (I4 "structural changes sourced to enacted law"):** the One Big Beautiful Bill Act (Pub. L. 119-21, 2025) made the TCJA rate structure permanent and raised the **2025 standard deduction**, but did **not** change the 2025 **bracket thresholds** or the §1(h) **breakpoints** (the extra inflation bump to the 10%/12% brackets begins **2026**). B takes `ordinary_taxable_income` (already post-deduction) as input and **does not use the standard deduction**, so the TY2025 indexed values are exactly Rev. Proc. 2024-40. Record this confirmation in the `source` strings and a doc comment. The values asserted by the KATs below were verified 2026-06-29 against Rev. Proc. 2024-40 (cross-checked vs Tax Foundation & IRS IR-2024-273); confirm again before coding.

**TY2025 indexed values to encode (Rev. Proc. 2024-40):**

*Ordinary brackets — `(lower, rate)`, ascending:*
- **Single** & **MFS (lower bands)**: 0→10%, 11,925→12%, 48,475→22%, 103,350→24%, 197,300→32%, 250,525→35%; **Single** 626,350→37%; **MFS** 375,800→37%.
- **MFJ / QSS**: 0→10%, 23,850→12%, 96,950→22%, 206,700→24%, 394,600→32%, 501,050→35%, 751,600→37%.
- **HoH**: 0→10%, 17,000→12%, 64,850→22%, 103,350→24%, 197,300→32%, 250,500→35%, 626,350→37%.

*§1(h) LT breakpoints — `(max_zero, max_fifteen)`:*
- **Single**: (48,350; 533,400). **MFJ/QSS**: (96,700; 600,050). **HoH**: (64,750; 566,700). **MFS**: (48,350; 300,000).

(QSS reuses the MFJ entry via `TaxTable::key`; encode MFJ once and let QSS alias it, OR insert an explicit `Qss` clone — pick the alias to avoid drift.)

**Files**
- new `crates/btctax-adapters/src/tax_tables.rs`
- modify `crates/btctax-adapters/src/lib.rs` (`pub mod tax_tables;` + `pub use tax_tables::BundledTaxTables;`)

**Interfaces**
```rust
// tax_tables.rs
use btctax_core::tax::tables::{
    LtcgBreakpoints, OrdinaryBracket, OrdinarySchedule, TaxTable, TaxTables,
};
use btctax_core::FilingStatus;
use rust_decimal_macros::dec;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct BundledTaxTables { by_year: BTreeMap<i32, TaxTable> }

impl BundledTaxTables {
    /// Build the compiled-in tables (TY2025 mandatory; later years added as their Rev. Procs. are verified).
    pub fn load() -> Self {
        let mut by_year = BTreeMap::new();
        by_year.insert(2025, ty2025());
        // by_year.insert(2026, ty2026()); // add ONLY when verified vs Rev. Proc. 2025-32 + OBBBA structural law
        Self { by_year }
    }
}
impl TaxTables for BundledTaxTables {
    fn table_for(&self, year: i32) -> Option<&TaxTable> { self.by_year.get(&year) }
}

fn br(lower: btctax_core::Usd, rate: btctax_core::Usd) -> OrdinaryBracket { OrdinaryBracket { lower, rate } }

/// TY2025 — Rev. Proc. 2024-40 §2.01 (rate tables) + §2.03 (Maximum Capital Gains Rate). Verified <DATE>.
fn ty2025() -> TaxTable {
    let mut ordinary = BTreeMap::new();
    ordinary.insert(FilingStatus::Single, OrdinarySchedule { brackets: vec![
        br(dec!(0), dec!(0.10)), br(dec!(11925), dec!(0.12)), br(dec!(48475), dec!(0.22)),
        br(dec!(103350), dec!(0.24)), br(dec!(197300), dec!(0.32)), br(dec!(250525), dec!(0.35)),
        br(dec!(626350), dec!(0.37)),
    ]});
    ordinary.insert(FilingStatus::Mfj, OrdinarySchedule { brackets: vec![
        br(dec!(0), dec!(0.10)), br(dec!(23850), dec!(0.12)), br(dec!(96950), dec!(0.22)),
        br(dec!(206700), dec!(0.24)), br(dec!(394600), dec!(0.32)), br(dec!(501050), dec!(0.35)),
        br(dec!(751600), dec!(0.37)),
    ]});
    ordinary.insert(FilingStatus::HoH, OrdinarySchedule { brackets: vec![
        br(dec!(0), dec!(0.10)), br(dec!(17000), dec!(0.12)), br(dec!(64850), dec!(0.22)),
        br(dec!(103350), dec!(0.24)), br(dec!(197300), dec!(0.32)), br(dec!(250500), dec!(0.35)),
        br(dec!(626350), dec!(0.37)),
    ]});
    ordinary.insert(FilingStatus::Mfs, OrdinarySchedule { brackets: vec![
        br(dec!(0), dec!(0.10)), br(dec!(11925), dec!(0.12)), br(dec!(48475), dec!(0.22)),
        br(dec!(103350), dec!(0.24)), br(dec!(197300), dec!(0.32)), br(dec!(250525), dec!(0.35)),
        br(dec!(375800), dec!(0.37)),
    ]});
    let mut ltcg = BTreeMap::new();
    ltcg.insert(FilingStatus::Single, LtcgBreakpoints { max_zero: dec!(48350), max_fifteen: dec!(533400) });
    ltcg.insert(FilingStatus::Mfj,    LtcgBreakpoints { max_zero: dec!(96700), max_fifteen: dec!(600050) });
    ltcg.insert(FilingStatus::HoH,    LtcgBreakpoints { max_zero: dec!(64750), max_fifteen: dec!(566700) });
    ltcg.insert(FilingStatus::Mfs,    LtcgBreakpoints { max_zero: dec!(48350), max_fifteen: dec!(300000) });
    TaxTable { year: 2025, source: "Rev. Proc. 2024-40 §2.01/§2.03 (TY2025); OBBBA Pub. L. 119-21 \
               left 2025 brackets/breakpoints unchanged", ordinary, ltcg }
}
```

**Steps**

1. **Failing tests** — `tax_tables.rs` `#[cfg(test)] mod tests`:
```rust
use super::*;
use btctax_core::tax::tables::{niit_threshold, loss_limit, NIIT_RATE};
use rust_decimal_macros::dec;

#[test]
fn ty2025_single_ordinary_brackets_match_rev_proc_2024_40() {
    let t = BundledTaxTables::load();
    let s = t.table_for(2025).unwrap().ordinary_for(FilingStatus::Single);
    assert_eq!(s.brackets[1].lower, dec!(11925)); // 12% start
    assert_eq!(s.brackets[2].lower, dec!(48475)); // 22% start
    assert_eq!(s.brackets[6].lower, dec!(626350)); // 37% start
    assert_eq!(s.brackets[6].rate, dec!(0.37));
}
#[test]
fn ty2025_ltcg_breakpoints_all_statuses() {
    let t = BundledTaxTables::load(); let tt = t.table_for(2025).unwrap();
    assert_eq!(*tt.ltcg_for(FilingStatus::Single), LtcgBreakpoints { max_zero: dec!(48350), max_fifteen: dec!(533400) });
    assert_eq!(*tt.ltcg_for(FilingStatus::Mfj),    LtcgBreakpoints { max_zero: dec!(96700), max_fifteen: dec!(600050) });
    assert_eq!(*tt.ltcg_for(FilingStatus::Qss),    LtcgBreakpoints { max_zero: dec!(96700), max_fifteen: dec!(600050) }); // QSS≡MFJ
    assert_eq!(*tt.ltcg_for(FilingStatus::HoH),    LtcgBreakpoints { max_zero: dec!(64750), max_fifteen: dec!(566700) });
    assert_eq!(*tt.ltcg_for(FilingStatus::Mfs),    LtcgBreakpoints { max_zero: dec!(48350), max_fifteen: dec!(300000) });
}
#[test]
fn mfs_37_pct_starts_at_375800_and_mfj_at_751600() {
    let t = BundledTaxTables::load(); let tt = t.table_for(2025).unwrap();
    assert_eq!(tt.ordinary_for(FilingStatus::Mfs).brackets.last().unwrap().lower, dec!(375800));
    assert_eq!(tt.ordinary_for(FilingStatus::Mfj).brackets.last().unwrap().lower, dec!(751600));
}
#[test]
fn missing_year_returns_none() { assert!(BundledTaxTables::load().table_for(2099).is_none()); }
#[test]
fn statutory_values_are_NOT_in_the_table_and_constant_across_years() {
    // STATUTORY (I4): no TaxTable field carries NIIT/loss-limit; the cited fns are year-independent.
    assert_eq!(niit_threshold(FilingStatus::Mfj), dec!(250000));
    assert_eq!(loss_limit(FilingStatus::Mfs), dec!(1500));
    assert_eq!(NIIT_RATE, dec!(0.038));
    // (If TY2026 is bundled, assert its indexed breakpoints DIFFER from TY2025 while the statutory values
    //  above are identical — the indexed-moves / statutory-fixed contrast.)
}
```
2. **Run → RED** (`BundledTaxTables` absent).
3. **Minimal impl:** the module above (Single/MFJ/HoH/MFS schedules + 4 LTCG entries; QSS aliases MFJ via `TaxTable::key`). Replace `<DATE>` with the verification date. `lib.rs` re-export.
4. **Run → GREEN.** Whole suite.
5. **Commit:** `feat(adapters): BundledTaxTables — TY2025 indexed brackets/breakpoints (Rev. Proc. 2024-40)`.

---

## TASK 7 — Worked-example golden KATs against the bundled TY2025 table

**Goal.** The spec-mandated **hand-verified goldens** (B "Tests"), each **reproducible by hand from the bundled TY2025 numbers** (so they live where `BundledTaxTables` is available — adapters tests — and exercise the real data end-to-end through `btctax_core::compute_tax_year`). These complement Task 5's synthetic-table mechanics tests (no overlap: Task 5 = math units on simple tables; Task 7 = end-to-end goldens on real tables).

**Files**
- new `crates/btctax-adapters/tests/kat_rate_engine.rs`

**Interfaces consumed:** `btctax_core::tax::compute::compute_tax_year`, `btctax_core::project::project`, `btctax_adapters::BundledTaxTables`, `btctax_core::price::StaticPrices`, `TaxProfile`/`FilingStatus`/`TaxOutcome`/`Carryforward`.

**Steps**

1. **Failing tests** — `tests/kat_rate_engine.rs` (each golden is hand-derived in its doc comment from the bundled numbers):
```rust
// Shared fixture helpers: build Acquire/Dispose/Income events priced by StaticPrices so projection yields
// disposals with the intended ST/LT gains (acquire ≥1yr+1day before sell ⇒ LT; same-year ⇒ ST), then call
// compute_tax_year(&events, &project(...), 2025, Some(&profile), &BundledTaxTables::load()).

fn single(ord: Usd, magi: Usd, qd: Usd) -> TaxProfile { /* FilingStatus::Single, zeros for optionals */ }

#[test]
fn single_lt_crosses_0_to_15() {
    // Single TY2025: max_zero 48,350. OTI 40,000 ordinary; crypto LT gain 20,000 (top 60,000).
    // 8,350 @ 0% + 11,650 @ 15% = 1,747.50. No ordinary delta, no NIIT (magi 60k<200k).
    let r = computed(/* LT 20,000 in 2025 */, single(dec!(40000), dec!(60000), dec!(0)));
    assert_eq!(r.ltcg_tax, dec!(1747.50));
    assert_eq!(r.total_federal_tax_attributable, dec!(1747.50));
    assert_eq!(r.marginal_rates.ltcg, dec!(0.15));
}

#[test]
fn single_lt_crosses_15_to_20() {
    // Single TY2025: max_fifteen 533,400. OTI 500,000; crypto LT gain 100,000 (top 600,000).
    // 33,400 @ 15% + 66,600 @ 20% = 5,010 + 13,320 = 18,330.00.
    // NIIT: nii 100,000; magi_with = 600,000 (magi_excl 500,000 + 100,000) over 200,000 by 400,000 →
    //   3.8% * min(100,000, 400,000)=100,000 → 3,800.00. magi_without 500,000 already >200,000, nii_without 0 → 0.
    let r = computed(/* LT 100,000 in 2025 */, single(dec!(500000), dec!(500000), dec!(0)));
    assert_eq!(r.ltcg_tax, dec!(18330.00));
    assert_eq!(r.niit, dec!(3800.00));
    assert_eq!(r.total_federal_tax_attributable, dec!(18330.00) + dec!(3800.00));
    assert_eq!(r.marginal_rates.ltcg, dec!(0.20));
}

#[test]
fn single_qd_pushes_crypto_lt_from_15_to_20() {
    // I9: QD shares the §1(h) stack. OTI 450,000; QD 80,000; crypto LT 20,000.
    //   without crypto: bottom 450,000, pref=QD 80,000, top 530,000<533,400 → 15% on 80,000 = 12,000.
    //   with crypto:    pref 100,000, top 550,000 → 15% on 83,400 + 20% on 16,600 = 12,510 + 3,320 = 15,830.
    //   ltcg_tax (delta) = 3,830.00  (QD pushed 16,600 of the crypto LT into 20%; without QD it'd be 3,000).
    // B-M3: magi_excluding_crypto = 530,000 = OTI 450,000 + QD 80,000 — INTERNALLY CONSISTENT (it must
    // already include the QD per the B.1 / ambiguity-#5 contract). This is a fixture input, not a tax figure;
    // ltcg_tax is independent of MAGI, so the asserted 3,830.00 is unchanged.
    let r = computed(/* LT 20,000 in 2025 */, single(dec!(450000), dec!(530000), dec!(80000)));
    assert_eq!(r.ltcg_tax, dec!(3830.00));
}

#[test]
fn mfj_st_gain_stacks_on_ordinary() {
    // MFJ TY2025: 12% band top 96,950. OTI 90,000; crypto ST gain 20,000 → bottom 110,000.
    //   tax(110,000) = 10%*23,850 + 12%*(96,950-23,850) + 22%*(110,000-96,950)
    //               = 2,385 + 8,772 + 2,871 = 14,028.
    //   tax(90,000)  = 2,385 + 12%*(90,000-23,850)=7,938 = 10,323.  delta = 3,705.00.
    let r = computed_mfj(/* ST 20,000 in 2025 */, dec!(90000), dec!(90000));
    assert_eq!(r.st_net, dec!(20000));
    assert_eq!(r.total_federal_tax_attributable, dec!(3705.00));
    assert_eq!(r.marginal_rates.ordinary, dec!(0.22));
}

#[test]
fn single_3k_loss_limit_and_multiyear_st_first_carryforward() {
    // Year 2025: crypto ST loss 5,000 + LT loss 2,000 → loss_deduction 3,000 (ST-first),
    //   carryforward_out {short:2,000, long:2,000}. ordinary_delta = tax(OTI-3,000)-tax(OTI) (a benefit).
    let r25 = computed(/* ST -5,000 & LT -2,000 in 2025 */, single(dec!(60000), dec!(60000), dec!(0)));
    assert_eq!(r25.loss_deduction, dec!(3000));
    assert_eq!(r25.carryforward_out, Carryforward { short: dec!(2000), long: dec!(2000) });
    // Feed carryforward_out → next year's carryforward_in (M4 chains in Task 10): LT gain 10,000 nets to 6,000.
    // (Modeled here by setting profile.capital_loss_carryforward_in = r25.carryforward_out for a 2025 re-run.)
}

#[test]
fn refusal_and_missing_table_end_to_end() {
    let tables = BundledTaxTables::load();
    let st = /* clean projection, no disposals */;
    assert!(matches!(compute_tax_year(&[], &st, 2099, Some(&single(dec!(1),dec!(1),dec!(0))), &tables),
        TaxOutcome::NotComputable(b) if b.kind == btctax_core::BlockerKind::TaxTableMissing));
}
```
2. **Run → RED** (assertions fail until the bundled table + compute are wired; if Task 6 done, RED only on missing fixtures).
3. **Minimal impl:** none in `src` (Tasks 5/6 supply the behavior); implement the fixture helpers (`computed`/`computed_mfj`/`single`) that build priced synthetic events and call `compute_tax_year`. Every golden must equal its doc-comment hand computation.
4. **Run → GREEN.** Whole suite.
5. **Commit:** `test(adapters): hand-verified rate-engine goldens vs bundled TY2025 (0→15→20, QD-share, NIIT, ST stack, $3k §1212)`.

---

## TASK 8 — `tax_profile` side-table + `tax-profile` CLI command

**Goal.** Persist the per-year `TaxProfile` in a `tax_profile(year, profile_json)` side-table (modeled on `cli_config`, `config.rs:42-134`) — a projection input, **not** ledger state. Add a `tax-profile` command to set/show it. A computed year with no profile surfaces `TaxProfileMissing` (already produced by `compute_tax_year`, Task 5).

**Files**
- new `crates/btctax-cli/src/tax_profile.rs`
- modify `crates/btctax-cli/src/lib.rs` (`pub mod tax_profile;`)
- modify `crates/btctax-cli/src/session.rs` (`from_fresh_vault` also calls `tax_profile::init_table`; add `Session::tax_profile(year)` + `all_tax_profiles()`)
- modify `crates/btctax-cli/src/cmd/mod.rs` + new `crates/btctax-cli/src/cmd/tax.rs`
- modify `crates/btctax-cli/src/main.rs` (clap `Command::TaxProfile` + dispatch; `FilingStatusArg` value-enum)

**Interfaces**
```rust
// tax_profile.rs  (mirror of config.rs’s side-table discipline)
use crate::CliError;
use btctax_core::TaxProfile;
use rusqlite::{Connection, OptionalExtension};
use std::collections::BTreeMap;

pub fn init_table(conn: &Connection) -> Result<(), CliError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS tax_profile (year INTEGER PRIMARY KEY, profile_json TEXT NOT NULL);")?;
    Ok(())
}
pub fn get(conn: &Connection, year: i32) -> Result<Option<TaxProfile>, CliError> {
    init_table(conn)?; // robust to older vaults (same guard as read_config, config.rs:77)
    let json: Option<String> = conn
        .query_row("SELECT profile_json FROM tax_profile WHERE year=?1", [year], |r| r.get(0))
        .optional()?;
    match json {
        None => Ok(None),
        Some(j) => Ok(Some(serde_json::from_str(&j).map_err(|e| CliError::BadConfigValue {
            key: format!("tax_profile[{year}]"), value: format!("invalid JSON: {e}"),
        })?)),
    }
}
pub fn set(conn: &Connection, year: i32, p: &TaxProfile) -> Result<(), CliError> {
    init_table(conn)?;
    let j = serde_json::to_string(p).map_err(|e| CliError::BadConfigValue {
        key: format!("tax_profile[{year}]"), value: e.to_string() })?;
    conn.execute(
        "INSERT INTO tax_profile(year,profile_json) VALUES(?1,?2)
         ON CONFLICT(year) DO UPDATE SET profile_json=excluded.profile_json", rusqlite::params![year, j])?;
    Ok(())
}
pub fn all(conn: &Connection) -> Result<BTreeMap<i32, TaxProfile>, CliError> {
    init_table(conn)?;
    let mut stmt = conn.prepare("SELECT year, profile_json FROM tax_profile ORDER BY year")?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, i32>(0)?, r.get::<_, String>(1)?)))?;
    let mut out = BTreeMap::new();
    for row in rows { let (y, j) = row?; out.insert(y, serde_json::from_str(&j)
        .map_err(|e| CliError::BadConfigValue { key: format!("tax_profile[{y}]"), value: e.to_string() })?); }
    Ok(out)
}
```
```rust
// cmd/tax.rs  (set/show; report_tax_year added in Task 9)
use crate::{tax_profile, CliError, Session};
use btctax_core::TaxProfile;
use btctax_store::Passphrase;
use std::path::Path;

pub fn set_profile(vault: &Path, pp: &Passphrase, year: i32, p: TaxProfile) -> Result<(), CliError> {
    let mut s = Session::open(vault, pp)?;
    tax_profile::set(s.conn(), year, &p)?;
    s.save()
}
pub fn show_profile(vault: &Path, pp: &Passphrase, year: i32) -> Result<Option<TaxProfile>, CliError> {
    tax_profile::get(Session::open(vault, pp)?.conn(), year)
}
```
clap (main.rs): a `Command::TaxProfile { year: i32, filing_status: Option<FilingStatusArg>, ordinary_taxable_income: Option<String>, magi_excluding_crypto: Option<String>, qualified_dividends: Option<String>, other_net_capital_gain: Option<String>, carryforward_short: Option<String>, carryforward_long: Option<String>, show: bool }` and `#[derive(Copy,Clone,ValueEnum)] enum FilingStatusArg { Single, Mfj, Mfs, Hoh, Qss }` (`impl From<FilingStatusArg> for FilingStatus`). USD args parse via `eventref::parse_usd_arg` (`eventref.rs:76`). On `--show`, print the stored profile (or "none"); otherwise require all mandatory fields (`filing_status`, `ordinary_taxable_income`, `magi_excluding_crypto`, `qualified_dividends`), default the optionals to `0`, build a `TaxProfile`, persist via `cmd::tax::set_profile`. **B-M3:** the `--magi-excluding-crypto` arg carries a clap `help`/`long_help` stating the §1411 contract explicitly — it **must already include** the taxpayer's qualified dividends + non-crypto net capital gains (and exclude only the app-computed crypto items); the engine adds only the crypto AGI delta on top (ambiguity #5), so omitting them **understates** NIIT. A matching doc comment sits on `TaxProfile::magi_excluding_crypto` (Task 1).

**Steps**

1. **Failing tests** — `tax_profile.rs` `#[cfg(test)] mod tests` (in-memory conn, mirrors `config.rs:151-242`):
```rust
use super::*;
use btctax_core::{Carryforward, FilingStatus, TaxProfile};
use rust_decimal_macros::dec;
fn mem() -> rusqlite::Connection { let c = rusqlite::Connection::open_in_memory().unwrap(); init_table(&c).unwrap(); c }
fn prof() -> TaxProfile { TaxProfile {
    filing_status: FilingStatus::Mfj, ordinary_taxable_income: dec!(120000), magi_excluding_crypto: dec!(130000),
    qualified_dividends_and_other_pref_income: dec!(0), other_net_capital_gain: dec!(0),
    capital_loss_carryforward_in: Carryforward { short: dec!(0), long: dec!(0) } } }

#[test] fn set_then_get_round_trips() {
    let c = mem(); set(&c, 2025, &prof()).unwrap();
    assert_eq!(get(&c, 2025).unwrap().unwrap(), prof());
    assert_eq!(get(&c, 2024).unwrap(), None);
}
#[test] fn get_on_tableless_vault_is_ok_none() {
    let c = rusqlite::Connection::open_in_memory().unwrap(); // no init_table
    assert_eq!(get(&c, 2025).unwrap(), None);
}
#[test] fn bad_json_is_a_typed_error_not_a_panic() {
    let c = mem();
    c.execute("INSERT INTO tax_profile(year,profile_json) VALUES(2025,'not json')", []).unwrap();
    assert!(matches!(get(&c, 2025).unwrap_err(), CliError::BadConfigValue { .. }));
}
#[test] fn all_returns_sorted_by_year() {
    let c = mem(); set(&c, 2026, &prof()).unwrap(); set(&c, 2025, &prof()).unwrap();
    assert_eq!(all(&c).unwrap().keys().copied().collect::<Vec<_>>(), vec![2025, 2026]);
}
```
   Plus a CLI integration test in `crates/btctax-cli/tests/` (temp vault): `tax-profile --year 2025 --filing-status mfj --ordinary-taxable-income 120000 --magi-excluding-crypto 130000 --qualified-dividends 0`, then `tax-profile --year 2025 --show` prints the stored values.
2. **Run → RED.**
3. **Minimal impl:** the module + command + dispatch + `Session` helpers + `from_fresh_vault` init (`session.rs:38-43`).
4. **Run → GREEN.** Whole suite.
5. **Commit:** `feat(cli): tax_profile per-year side-table + tax-profile set/show command`.

---

## TASK 9 — `report --tax-year` surfacing (B.5)

**Goal.** Wire the projection + the stored profile + `BundledTaxTables` into `compute_tax_year` and render the `TaxResult` (or the `TaxYearNotComputable`/missing reason) under `report --tax-year <y>`. Standalone "tax owed / what-if" calculator; exact Decimal formatting; deterministic.

**Files**
- modify `crates/btctax-cli/src/cmd/tax.rs` (`report_tax_year`)
- modify `crates/btctax-cli/src/render.rs` (`render_tax_outcome`)
- modify `crates/btctax-cli/src/main.rs` (`Command::Report` gains `--tax-year`; dispatch)
- new/extend `crates/btctax-cli/tests/tax_report.rs`

**Interfaces**
```rust
// cmd/tax.rs
use btctax_adapters::BundledTaxTables;
use btctax_core::{tax::compute::compute_tax_year, TaxOutcome};
pub fn report_tax_year(vault: &Path, pp: &Passphrase, year: i32) -> Result<TaxOutcome, CliError> {
    let s = Session::open(vault, pp)?;
    let (events, state, _cfg) = s.load_events_and_project()?;   // session.rs:87-95
    let profile = tax_profile::get(s.conn(), year)?;
    let tables = BundledTaxTables::load();
    Ok(compute_tax_year(&events, &state, year, profile.as_ref(), &tables))
}
```
```rust
// render.rs  (exact Decimal Display, like render_report leg fields, render.rs:228-243)
pub fn render_tax_outcome(year: i32, out: &btctax_core::TaxOutcome) -> String {
    use btctax_core::TaxOutcome::*;
    let mut s = String::new();
    let _ = writeln!(s, "Federal tax attributable to crypto — tax year {year}");
    match out {
        NotComputable(b) => { let _ = writeln!(s, "  NOT COMPUTABLE [{:?}]: {}", b.kind, b.detail); }
        Computed(r) => {
            let _ = writeln!(s, "  net short-term: {}   net long-term: {}", r.st_net, r.lt_net);
            let _ = writeln!(s, "  crypto ordinary income (level): {}", r.ordinary_from_crypto);
            // B-M2: surface the ordinary-rate attributable DELTA so the three attributable components
            // visibly reconcile to TOTAL. By the pinned identity this equals (ord_with − ord_without) exactly.
            let ordinary_rate_attributable = r.total_federal_tax_attributable - r.ltcg_tax - r.niit;
            let _ = writeln!(s, "  ordinary-rate tax (attributable): {}", ordinary_rate_attributable);
            let _ = writeln!(s, "  LTCG tax (attributable): {}   NIIT (attributable): {}", r.ltcg_tax, r.niit);
            let _ = writeln!(s, "  TOTAL federal tax attributable to crypto (delta): {}   \
                (= ordinary-rate + LTCG + NIIT attributable)", r.total_federal_tax_attributable);
            let _ = writeln!(s, "  §1211 loss deduction (level): {}   carryforward out: short {} / long {}",
                r.loss_deduction, r.carryforward_out.short, r.carryforward_out.long);
            let _ = writeln!(s, "  marginal rates: ordinary {} / LTCG {} / NIIT {}",
                r.marginal_rates.ordinary, r.marginal_rates.ltcg, r.marginal_rates.niit_applies);
            let _ = writeln!(s, "  (incremental ceteris-paribus delta on the minimal profile; \
                excludes AGI-driven SS/IRMAA/AMT/QBI/phaseout effects — I5. NIIT uses a minimal NII model \
                — excludes crypto ordinary income from NII and does not reduce NII by the allowed §1211 \
                loss — so it MAY UNDERSTATE NIIT; see §5 Phase-2 refinement.)");
        }
    }
    s
}
```
clap: `Command::Report { #[arg(long)] year: Option<i32>, #[arg(long)] tax_year: Option<i32> }`. Dispatch (`main.rs:247-250`): if `tax_year` is `Some(y)`, call `cmd::tax::report_tax_year` + print `render::render_tax_outcome`; else the existing display path (`render::render_report`). The two flags are independent (do not alias).

**Steps**

1. **Failing tests** — `crates/btctax-cli/tests/tax_report.rs` (temp vault, synthetic CSV import or direct event append + a set profile):
```rust
// build a vault with one LT sell (gain 20,000) dated 2025; set a Single profile (OTI 40,000, magi 60,000);
// run `report --tax-year 2025`; assert stdout contains "TOTAL federal tax attributable to crypto (delta): 1747.50".
#[test] fn report_tax_year_renders_golden() { /* assert_cmd over the temp vault */ }

#[test] fn report_tax_year_components_reconcile_to_total() {
    // B-M2: the printed attributable components sum to TOTAL. For the Single LT 0→15 golden:
    // ordinary-rate (attributable) 0.00 + LTCG 1747.50 + NIIT 0.00 = TOTAL 1747.50.
    // Assert stdout shows "ordinary-rate tax (attributable): 0.00", "LTCG tax (attributable): 1747.50",
    // "NIIT (attributable): 0.00", and "...crypto (delta): 1747.50" — and that the three add to the total.
}

#[test] fn report_tax_year_without_profile_says_not_computable() {
    // same vault, no profile set → "NOT COMPUTABLE [TaxProfileMissing]".
}
#[test] fn report_tax_year_with_hard_blocker_says_not_computable() {
    // a disposal missing FMV → "NOT COMPUTABLE [TaxYearNotComputable]".
}
#[test] fn report_display_year_still_works_unchanged() {
    // `report --year 2025` keeps the existing holdings/disposals rendering (no regression).
}
```
2. **Run → RED.**
3. **Minimal impl:** the three functions + dispatch above.
4. **Run → GREEN.** Whole suite.
5. **Commit:** `feat(cli): report --tax-year — standalone TaxResult / what-if calculator surfacing`.

---

## TASK 10 — `carryforward_in ↔ prior-year carryforward_out` consistency (M4)

**Goal.** A **non-gating advisory** (M4): when both year `Y` and year `Y−1` profiles exist, cross-check `profile[Y].capital_loss_carryforward_in` against the `carryforward_out` that `compute_tax_year(Y−1)` produces; warn on mismatch in `report --tax-year`. Pure, deterministic; never a hard blocker (the user may legitimately not have computed Y−1 here).

**Files**
- modify `crates/btctax-core/src/tax/compute.rs` (`carryforward_consistency`)
- modify `crates/btctax-core/src/tax/mod.rs` + `lib.rs` (re-export)
- modify `crates/btctax-cli/src/cmd/tax.rs` (compute Y−1 when its profile exists; pass the warning out)
- modify `crates/btctax-cli/src/render.rs` (render the optional warning line)

**Interfaces**
```rust
// tax/compute.rs
/// M4: compare the declared carryforward-in for a year against the prior year's computed carryforward-out.
/// Returns a human warning when they differ; `None` when they match or the prior year is unavailable.
pub fn carryforward_consistency(prior_out: Option<&Carryforward>, this_in: &Carryforward) -> Option<String> {
    match prior_out {
        Some(p) if p != this_in => Some(format!(
            "carryforward_in (short {} / long {}) does not match prior-year carryforward_out \
             (short {} / long {}) — verify your prior return", this_in.short, this_in.long, p.short, p.long)),
        _ => None,
    }
}
```
CLI (`report_tax_year`): if `tax_profile::get(conn, year-1)` is `Some` and `compute_tax_year(.., year-1, ..)` is `Computed(prev)`, call `carryforward_consistency(Some(&prev.carryforward_out), &profile.capital_loss_carryforward_in)`; thread the `Option<String>` into the rendered outcome (extra advisory line; does **not** change exit code or block computation).

**Steps**

1. **Failing tests** — `tax/compute.rs`:
```rust
#[test] fn carryforward_match_is_silent() {
    let c = Carryforward { short: dec!(2000), long: dec!(2000) };
    assert_eq!(carryforward_consistency(Some(&c), &c), None);
}
#[test] fn carryforward_mismatch_warns() {
    let prior = Carryforward { short: dec!(2000), long: dec!(2000) };
    let declared = Carryforward { short: dec!(0), long: dec!(2000) };
    assert!(carryforward_consistency(Some(&prior), &declared).unwrap().contains("does not match"));
}
#[test] fn no_prior_is_silent() {
    assert_eq!(carryforward_consistency(None, &Carryforward::default()), None);
}
```
   Plus a CLI test: chain 2025→2026 profiles where 2026's declared `carryforward_in` ≠ 2025's `carryforward_out`; assert `report --tax-year 2026` prints the advisory line (and still computes/exits 0).
2. **Run → RED.**
3. **Minimal impl:** the fn + CLI threading + render line.
4. **Run → GREEN.** Whole suite.
5. **Commit:** `feat(core,cli): M4 carryforward_in↔prior carryforward_out consistency advisory`.

---

## TASK 11 — Whole-diff review + full-suite green (Phase E gate)

**Goal.** The mandatory post-implementation, independent, adversarial whole-diff review (`STANDARD_WORKFLOW.md` Phase E), run as one system — catching cross-phase drift (a bracket constant disagreeing across modules, a level/delta contract that slipped, a guarantee promised but not delivered).

**Steps**
1. Run the full validation surface: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check`.
2. Confirm the cross-cutting guarantees hold by test: statutory NIIT/loss-limit constant across years AND absent from `TaxTable` (Tasks 2/6); `total == (ord_with − ord_without) + ltcg_tax + niit` identity (Task 7); double-count guard (Tasks 5/7); the printed attributable components reconcile to TOTAL (B-M2, Task 9); refusal on **any** `severity()==Hard` blocker in the projection — keyed generally, not by a per-year/kind subset (Task 5; B-I1), `TaxYearNotComputable` carrying the offending `EventId` (B-N1); determinism (Task 5); no `f32`/`f64` anywhere in `tax`/`tax_tables` (grep the diff); every TY2025 number re-verified vs Rev. Proc. 2024-40 PDF (Task 6 verification gate).
3. Dispatch an independent reviewer over the **entire** diff; persist verbatim to `reviews/R0-plan-rate-engine-whole-diff-round-N.md` **before** folding; loop to 0 Critical / 0 Important (re-review after every fold, including the last).
4. Flip any `FOLLOWUPS.md` items this change resolves (the "Rate/limit mechanics (Phase 2/3): 0/15/20% §1(h), 3.8% NIIT, $3,000 loss limit + carryforward" item, `FOLLOWUPS.md:217`); file new ones (e.g. TY2026 table bundling once Rev. Proc. 2025-32 + OBBBA 2026 structural law are verified; crypto-ordinary-income-in-NII modeling; non-LT `other_net_capital_gain` ST split).
5. **Ship commit** only when green.

---

## 4. Self-review

### 4.1 Spec coverage map (every Sub-project-B deliverable → task)

| Spec item | Where covered |
|---|---|
| B.1 `TaxProfile` (filing_status; ordinary_taxable_income **excl. all crypto items**; magi_excluding_crypto; qualified_dividends_and_other_pref_income; optional other_net_capital_gain + carryforward_in {short,long}) | Task 1 (type) |
| B.1 `tax_profile` per-year side-table + `tax-profile` command; missing → `TaxProfileMissing` | Task 8 (storage+CLI), Task 5 (blocker), Task 1 (kind) |
| B.2 bundled per-year tables (LT 0/15/20 breakpoints §1(h) + ordinary brackets, **inflation-indexed, cited to Rev. Proc.**) | Task 6 (data, Rev. Proc. 2024-40), Task 2 (types/trait) |
| B.2 NIIT thresholds + $3k/$1.5k limit **STATUTORY, not indexed** (hard-coded, statute cite, NOT in the table) | Task 2 (`niit_threshold`/`NIIT_RATE`/`loss_limit`); KAT Tasks 2/6 |
| B.2 year with no table → `TaxTableMissing`; KAT statutory constant across years while indexed move | Task 5 (blocker), Task 6 (KAT) |
| B.3 §1222 ST/LT netting incl. `carryforward_in` | Task 4 (`net_1222`) |
| B.3 ST net gain at ordinary marginal rates stacked on `ordinary_taxable_income` | Task 3 (`ordinary_tax_on`), Task 5 (stacking via `bottom`) |
| B.3 LT net gain 0/15/20 via §1(h) stacking (LT+QD share the preferential space) | Task 3 (`preferential_tax`), Task 5 (`qd + preferential_gain`) |
| B.3 §1411 NIIT 3.8% × min(NII, MAGI − threshold), threshold statutory | Task 5 (NIIT block), Task 2 (`niit_threshold`/`NIIT_RATE`) |
| B.3 §1211/§1212(b) $3k offset + **ST-first** character-preserving carryforward → carryforward_out | Task 4 (`net_1222`) |
| B.3 crypto **ordinary income** (mining/staking) on the ordinary stack, added exactly once | Task 5 (`crypto_ord` add to `bottom_with` only); double-count KAT Tasks 5/7 |
| B.3 objective = incremental delta (tax-with − tax-without) | Task 5 (two-scenario `total`) |
| B.3 `TaxResult { st_net, lt_net, ordinary_from_crypto, ltcg_tax, niit, loss_deduction, carryforward_out, total_federal_tax_attributable, marginal_rates }` | Task 1 (type), Task 5 (population) |
| B.4 refuse: no `TaxResult` when **any** unresolved **`severity()==Hard`** blocker is present anywhere in the projection → `TaxYearNotComputable` (projection-wide, B-I1) | Task 5 (`first_hard_blocker`), Task 1 (kind) |
| B.5 `report --tax-year <y>` shows the `TaxResult`/reason; exact Decimal; deterministic | Task 9 (CLI+render), Task 5 (determinism KAT) |
| Tests: per-filing-status goldens (0→15→20; QD pushes 15→20; NIIT crossing; ST stacking; $3k + multi-year §1212 ST-first; §1222 netting), hand-verified, reproducible from bundled tables | Task 7 (real-table goldens) + Task 4/5 (mechanics) |
| Tests: NIIT threshold / $3k limit constant across years | Tasks 2/6 |
| Tests: double-count guard; incremental-delta correctness; `TaxYearNotComputable`; missing-profile/table; determinism | Task 5 |
| Tests (M4): `carryforward_in ↔ prior carryforward_out` consistency check/warn | Task 10 |
| Cross-cutting: no new events; side-table + bundled-data inputs; no float; BTreeMap-only iteration; federal-only; privacy synthetic | Tasks 1–10 + Task 11 |

### 4.2 Placeholder scan

- No `todo!()`/`unimplemented!()`/`TODO`/`FIXME` in *delivered* code. The only `/* … */` markers are inside **test bodies** (fixture construction — building priced synthetic `Acquire`/`Dispose`/`Income` events via the documented `kat_tax.rs:16-34` + `StaticPrices` helpers) and the one readability `scen` pseudo-closure in Task 5's interface block, explicitly flagged "(real impl) compute bottoms … explicitly" with the concrete code immediately following. Each test's asserted golden is fully hand-derived in its doc comment.
- `<DATE>` in Task 6's `ty2025()` source string is a fill-at-write-time verification stamp, not shipped ambiguity.
- TY2026 is intentionally **not** bundled (commented out) — `TaxTableMissing` is the deliberate safety until Rev. Proc. 2025-32 + OBBBA 2026 structural law are verified (filed as a follow-up in Task 11).

### 4.3 Type-consistency of every signature vs cited source

- `compute_tax_year(events: &[LedgerEvent], state: &LedgerState, year: i32, profile: Option<&TaxProfile>, tables: &dyn TaxTables) -> TaxOutcome` — `LedgerEvent` (`event.rs:263-271`), `LedgerState` (`state.rs:176-186`); reads `state.disposals` (`:180`) → `Disposal.{disposed_at,legs}` (`:111,112`) → `DisposalLeg.{gain,term}` (`:101,102`); `state.income_recognized` (`:182`) → `IncomeRecord.{recognized_at,usd_fmv}` (`:143,145`); `state.blockers` (`:184`) → `Blocker.{kind,event,detail}` (`:65-70`) + `BlockerKind::severity()` (`:47-64`) + `Severity::Hard` (`:17-21`); `tax_date(utc,tz)` (`conventions.rs:52`); `EventId::canonical()` (identity.rs, used by render/eventref today). All money is `Usd = Decimal` (`conventions.rs:8`); `round_cents` (`conventions.rs:22`).
- `net_1222(...) -> CapNet`, `ordinary_tax_on(&OrdinarySchedule, Usd) -> Usd`, `preferential_tax(&LtcgBreakpoints, Usd, Usd) -> PrefSplit` — all `Decimal`, no float; comparisons/`min`/`max` done with `Decimal` `<`/`>` (no `f64` casts), consistent with `conventions.rs` / `fold.rs` Decimal idioms.
- `TaxTable`/`OrdinarySchedule`/`LtcgBreakpoints` keyed `BTreeMap<FilingStatus, …>` (NFR4 ordered iteration); `FilingStatus` derives `Ord` for the map key. `TaxTables::table_for(year) -> Option<&TaxTable>` mirrors `PriceProvider::usd_per_btc(date) -> Option<Usd>` (`price.rs:5,46-50`).
- `BundledTaxTables` mirrors `BundledPrices` (`price.rs:13-50`): `BTreeMap`-backed, `load()` constructor, `impl TaxTables`. Lives in `btctax-adapters` (depends on core), re-exported from its `lib.rs` like `BundledPrices`.
- `tax_profile` side-table mirrors `config.rs` (`init_table`/`get`/`set`/`all` + `init` from `Session::from_fresh_vault`, `session.rs:38-43`); errors reuse `CliError::BadConfigValue { key, value }` (`config.rs:86-90`). USD CLI args via `eventref::parse_usd_arg` (`eventref.rs:76`).
- `TaxProfile`/`FilingStatus`/`Carryforward` derive `Serialize,Deserialize` (persisted as JSON); the two optional `TaxProfile` fields use `#[serde(default)]` (same discipline as `AllocLot.dual_loss_basis`, `event.rs:151-154`).
- New `BlockerKind`s added to the enum (`state.rs:22-46`) + the `Hard` arm of `severity()` (`state.rs:50-60`) + `new_blockers_are_hard` (`state.rs:206-217`). `BlockerKind` derives `Ord` (`state.rs:22`); appending variants is additive.
- CLI commands mirror existing shape: `Command::TaxProfile`/`Report{..,tax_year}` (clap `:25-74`), dispatched (`:222-305`); handlers return through `Session::{open,conn,save,load_events_and_project,config}` (`session.rs:46-95`).

### 4.4 Statutory-vs-indexed confirmation (every value classified + sourced)

- **STATUTORY, NOT indexed (hard-coded with cite, never in `TaxTable`):** NIIT rate `0.038` (§1411(a)); NIIT thresholds $250k MFJ/QSS, $200k Single/HoH, $125k MFS (§1411(b)); capital-loss ordinary-offset limit $3,000 / $1,500 MFS (§1211(b)). Implemented as `NIIT_RATE`/`niit_threshold()`/`loss_limit()` in `tax::tables` — **year-independent**; the KAT (Tasks 2/6) asserts they are constant and that `TaxTable` has **no** field carrying them.
- **INDEXED, in the per-year `TaxTable`, each citing its Rev. Proc.:** §1(h) LT 0/15/20% breakpoints and the ordinary brackets — TY2025 sourced to **Rev. Proc. 2024-40 §2.03 / §2.01** (encoded in `ty2025()`, `source` string cites it). **Structural (I4):** the OBBBA (Pub. L. 119-21) note records that 2025 brackets/breakpoints are unchanged by OBBBA and that B does not use the OBBBA-bumped standard deduction (it consumes post-deduction taxable income). TY2026 would be sourced to **Rev. Proc. 2025-32** + OBBBA 2026 structural changes — bundled only once verified.
- Every bundled TY2025 number (brackets for all five statuses; breakpoints for all five) is asserted by a pin-the-data KAT (Task 6) and re-derived by hand in the Task 7 goldens — so "reproducible by hand from the bundled tables" holds, and a value drifting from Rev. Proc. 2024-40 fails a test.

### 4.5 Ambiguities resolved (for the R0 reviewer)

1. **`TaxResult` components — levels vs deltas.** The spec marks only `total_federal_tax_attributable` "(delta)". Resolved: `ltcg_tax`, `niit`, `total` are **crypto-attributable deltas** (`with − without`); `st_net`, `lt_net`, `ordinary_from_crypto`, `loss_deduction`, `carryforward_out`, `marginal_rates` describe the **WITH-crypto filing position**. Decisive reason: a *level* `ltcg_tax` for crypto LT is **ill-defined** when QD shares the §1(h) stack (no non-arbitrary split of stacked dollars between QD and crypto LT), whereas the delta `pref_tax(with) − pref_tax(without)` is exact and is precisely "the extra preferential tax the crypto LT caused" — which the I9 QD-share KAT needs. `carryforward_out` MUST be a level (it feeds next year's `carryforward_in`). The ordinary-rate piece of `total` is intentionally unnamed (the spec's field list omits it); a KAT pins `total == (ord_with − ord_without) + ltcg_tax + niit`. **B-M2 fold:** the report nonetheless **prints** that ordinary-rate delta (derived as `total − ltcg_tax − niit`, exact by the identity) as a labeled line so the displayed attributable components reconcile to `total`.
2. **`other_net_capital_gain` character.** Modeled as additional **net long-term (preferential)** capital gain — the §1222(11) "net capital gain" sense — entering the LT bucket of `net_1222`. The minimal profile does not separately carry non-crypto **short-term** gains (out of the minimal-model scope, Q#1); filed as a follow-up if needed.
3. **`capital_loss_carryforward_in` semantics + scenario placement.** Treated as prior-year **loss magnitudes by character** (≥0) that reduce the matching character in `net_1222`. It is part of the **fixed baseline** (present in BOTH the with- and without-crypto scenarios) so the delta isolates **this year's** crypto disposals only.
4. **Crypto ordinary income in NII.** **Excluded** from net investment income (mining/staking is typically a trade/business or otherwise outside the minimal NII model). The spec's NIIT bullet names only "these gains"; crypto ordinary income is added to the ordinary/MAGI stack (it raises the §1411 threshold-crossing via MAGI) but is **not** itself NII. Documented as a model limitation alongside the I5 SS/IRMAA/AMT/QBI exclusions; filed as a follow-up. **B-M1 direction:** excluding crypto ordinary income from NII (and not reducing NII by the allowed §1211 loss) can only ever **understate** NIIT — the labeled-limitation render line and the §5 follow-up say "may understate," recorded as a Phase-2 refinement.
5. **MAGI construction (no double count).** `magi_excluding_crypto` already includes the user's QD + non-crypto capital gain, so the WITHOUT scenario uses it as-is and the WITH scenario adds **only** the crypto AGI contribution (`crypto net taxable capital amount delta + crypto_ord`) — never re-adding the non-crypto baseline. NII, by contrast, is reconstructed from `QD + surviving net gains` (computable from components, no double-count risk).
6. **Hard-blocker gate (B.4) — projection-wide (B-I1 fold).** A year is `NotComputable` when **any** unresolved `severity()==Hard` blocker exists **anywhere** in the projection (`first_hard_blocker(state)`), not a per-event/per-year subset. Decisive reason: the earlier per-event scoping (event-`None`, event-tax-year`==year`, or a 3-kind basis-foundation set) **under-gated cross-year basis contamination** — an out-of-year unresolved `ImportConflict` (leaves the disputed-basis lot in the pool, `resolve.rs:362-377`) or a basis-affecting `DecisionConflict` (made-date can postdate the disposal, `resolve.rs:388-395`) is **not** `basis_pending`, so it does not re-trigger an in-year `FmvMissing` (`fold.rs:124-131`); a later year's disposal could consume the contaminated lot and B would emit an authoritative-but-wrong number. Any open Hard blocker ⇒ the basis foundation is unsound ⇒ EVERY year's computation refuses until it is resolved (deliberate conservatism; trades per-year granularity for a one-line, auditable guarantee). Still keys on the general `severity()` classifier, so future hard kinds auto-gate; the `TaxYearNotComputable` blocker carries the offending `EventId` (B-N1). Per-year granularity can be recovered later only via lot-lineage gating (Option 2) with its own KATs.
7. **Tax computed by the exact formula method at cent precision** (not the IRS binned Tax Tables, not whole-dollar rounding) — a deliberate NFR5 exactness/determinism choice that makes every golden exactly hand-reproducible; documented in Task 3 and Global Constraints.
8. **Crate placement.** Computation + types + trait + statutory constants live in **core** (`tax`); the bundled real numbers live in **adapters** (`BundledTaxTables`, mirroring `BundledPrices`); the per-year profile side-table + commands live in **CLI** (mirroring `cli_config`). Real-number goldens therefore live in **adapters tests** (so they are literally reproducible from the bundled tables), and core unit tests use small synthetic tables — a clean layering with no number duplication across crates.

---

## 5. Open follow-ups this plan will file (Task 11)

- **TY2026 table** — bundle once verified vs **Rev. Proc. 2025-32** + OBBBA 2026 structural changes; until then `TaxTableMissing` is the safety.
- **NIIT minimal-NII model (Phase-2 refinement, B-M1)** — the model can only **understate** NIIT: (a) crypto ordinary income (investor-level staking/rewards/airdrops) is excluded from NII; (b) NII is not reduced by the allowed §1211 loss in a net-loss year. Revisit both; the "may understate" caveat is surfaced on output (done), and the NII basis is refined in Phase 2.
- **Non-LT `other_net_capital_gain`** — add a short-term split to the profile if a real case needs it.
- Resolves `FOLLOWUPS.md:217` ("Rate/limit mechanics (Phase 2/3): 0/15/20% §1(h), 3.8% NIIT, $3,000 loss limit + carryforward").

---

## Fold record (R0 round 1)

Source review: `reviews/R0-plan-rate-engine-round-1.md` (2026-06-29; **0 Critical, 1 Important, 4 Minor, 2 Nit**). All findings folded below.

**No bracket / threshold / tax figure was changed.** Every TY2025 value was independently verified correct by the reviewer (ordinary brackets all 5 statuses; §1(h) breakpoints all 5 incl. MFS=$300,000; HoH-35%=$250,500 vs Single/MFS=$250,525; statutory $250k/$200k/$125k NIIT, 3.8%, $3,000/$1,500) and is untouched. The only numeric edit is a **fixture input** (`magi_excluding_crypto` in one Task-7 KAT); the asserted tax output is unchanged.

**Cites re-verified against current source at fold time (B-N2):** `BlockerKind::severity()` Hard set `state.rs:47-64` ✓; unresolved-`ImportConflict` branch ("original import stands unchanged", lot stays in pool) `resolve.rs:362-377` ✓; basis-affecting `DecisionConflict` branch `resolve.rs:388-395` ✓; `basis_pending` → in-year `FmvMissing` re-trigger `fold.rs:124-131` ✓.

- **B-I1 (Important) — RESOLVED** via the simpler recommended fix. The refusal gate is now **projection-wide**: `compute_tax_year` refuses the year (`TaxYearNotComputable`) when `first_hard_blocker(state)` finds **any** unresolved `severity()==Hard` blocker anywhere — the per-event / per-year / 3-kind enumeration (`hard_blocker_for_year`) is **deleted**. This closes the cross-year basis-contamination hole (an out-of-year unresolved `ImportConflict`, or a basis-affecting `DecisionConflict`, on a non-`basis_pending` lot consumed by a later disposal). New KAT `refuses_year_with_out_of_year_import_conflict_on_consumed_lot` (Task 5): a 2024 unresolved `ImportConflict` on a lot a 2025 disposal consumes ⇒ `TaxYearNotComputable`, **not** a wrong number. Folded into Global Constraints, Task 1 blocker doc, Task 5 (gate + helper + imports + reused-symbols note), §4.1 coverage map, §4.5 #6, Task 11. Deliberate conservatism documented (any open Hard blocker anywhere blocks all year computations until resolved); per-year granularity is recoverable later only via Option-2 lot-lineage gating with its own KATs.
- **B-M1 (Minor) — folded.** The NIIT minimal-NII limitation now states the **direction**: excluding crypto ordinary income from NII and not reducing NII by the allowed §1211 loss can only **understate** NIIT. The render note (Task 9) says "may understate"; recorded as a Phase-2 refinement in §5 (Task 11 files it to `FOLLOWUPS.md`); §4.5 #4 + the Task-5 NIIT comment updated.
- **B-M2 (Minor) — folded.** The report (Task 9) now prints the **ordinary-rate attributable delta** (`total − ltcg_tax − niit`, exactly `ord_with − ord_without` by the pinned identity, no extra rounding) as a labeled line, so the three attributable components visibly reconcile to TOTAL. Left **unnamed on `TaxResult`** (spec-faithful; ambiguity #1). Reconciliation KAT `report_tax_year_components_reconcile_to_total` added to Task 9.
- **B-M3 (Minor) — folded.** The `--magi-excluding-crypto` clap help + a `TaxProfile::magi_excluding_crypto` doc comment now state the §1411 contract (must already include QD + non-crypto net capital gains; the engine adds only the crypto AGI delta). The self-inconsistent Task-7 QD fixture is fixed to `magi_excluding_crypto = 530,000 = OTI 450,000 + QD 80,000` — a fixture input; `ltcg_tax` is MAGI-independent, so the asserted **3,830.00 is unchanged**.
- **B-M4 / B-N1 / B-N2 — folded.** B-M4: `marginal_rates` boundary/empty conventions pinned (display-only — `taxable > br.lower` reports the lower bracket at a boundary; `ltcg` reflects the WITH-crypto top-of-stack even with no crypto preferential income), in the level-vs-delta note + §4.5. B-N1: `TaxYearNotComputable` now carries the offending `EventId` (`event: b.event.clone()`) for downstream C. B-N2: cites re-verified (above).

**Self-consistency pass.** The gate is now "any Hard blocker in the projection"; no per-event/per-year enumeration remains anywhere (Global Constraints, Task 5 code + helper, §4.1, §4.5 #6, Task 11 all aligned; `events` retained in the signature for determinism-tuple parity, discarded in-body). The delta-vs-level rendering reconciles to the total (B-M2). The new KAT is reproducible. **No bracket/threshold/tax figure was altered.** Re-review after this fold per `STANDARD_WORKFLOW.md §2` (including the last).
