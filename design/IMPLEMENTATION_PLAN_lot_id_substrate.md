# IMPLEMENTATION PLAN — Sub-project A: Lot-Identification Substrate

**Program:** Lot-Identification & Tax-Optimization (Phase-2). **Sub-project:** A (independently valuable; ships first; A → B → C build order).
**Source of truth:** `design/SPEC_lot_optimization_program.md` (R0-GREEN 2026-06-29). Sub-project A + Legal grounding + Cross-cutting are **binding**.
**Status:** DRAFT — to be R0-reviewed (review-to-green, `STANDARD_WORKFLOW.md §2`) **before** any code. Then executed subagent-driven, one implementer carrying the whole plan (Phase D, §1).

> **How to execute (per `STANDARD_WORKFLOW.md`):** each task below is a TDD phase. Per task: (1) write the failing test(s) — real test code, (2) run → confirm RED, (3) minimal implementation — real code, (4) run → confirm GREEN, (5) run the **whole** validation suite (`cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check`), (6) independent review loop → 0 Critical / 0 Important, (7) commit. **Gates are hard.** The closing Task 10 is the mandatory whole-diff review (Phase E).

---

## 0. Global Constraints (apply to EVERY task — a violation is a blocking finding)

- **NFR4 determinism.** Identical `(events, prices, config)` → byte-identical `LedgerState`. Every ordering is a **total order** with explicit tiebreaks. No `HashMap` iteration (use `BTreeMap`/`BTreeSet`/sorted `Vec`). No `Date::now`/RNG in `btctax-core` (decision made-dates arrive as the injected `now: OffsetDateTime`, e.g. `crates/btctax-cli/src/cmd/reconcile.rs:27`).
- **NFR5 exact arithmetic / no float.** Money is `Usd = Decimal` (`crates/btctax-core/src/conventions.rs:8`); sats are `Sat = i64` (`conventions.rs:6`). **No `f32`/`f64` anywhere**, including the HIFO per-sat key — compare by cross-multiplication (`a.usd_basis * Usd::from(b.remaining_sat)` vs `b.usd_basis * Usd::from(a.remaining_sat)`), never division to a float.
- **Event-sourcing + serde backward-compat.** New `EventPayload`/`BlockerKind` variants are **additive**. Every new field on an existing serialized payload is `#[serde(default)]` (e.g. the new `SafeHarborAllocation.pre2025_method`; same discipline as `AllocLot.dual_loss_basis` at `crates/btctax-core/src/event.rs:150`). New decisions carry **`fingerprint = None`** — guaranteed by `persistence::fingerprint`'s `_ => return None` arm (`crates/btctax-core/src/persistence.rs:96`) and `append_decision`'s `None` insert (`persistence.rs:259`); an explicit KAT confirms it.
- **Privacy.** Tests use **synthetic fixtures + temp vaults only** (`tempfile`). No real reads, no PII. Bundled tax tables/prices are public reference data.
- **Federal only.** State tax is out of scope (app charter / spec §intro).
- **§1.1012-1(j) boundary (load-bearing, not cosmetic).** Adequate identification must exist **by the time of sale** in *every* year — **no post-hoc identification, ever**. No artifact, command, or doc may describe a post-hoc selection as compliant. The compliant binding levers are A.5(a) dated standing order, (b) contemporaneous `select-lots`, (c) Mode-2-before-selling. `MethodElection` cannot be back-dated; `LotSelection` contemporaneity is reported truthfully.

---

## 1. Source grounding (re-verified against CURRENT source at write time, 2026-06-29)

| Symbol / fact | Current location |
|---|---|
| `LotMethod` enum (FIFO-only) | `crates/btctax-core/src/project/mod.rs:20-24` |
| `ProjectionConfig { self_transfer_fee, lot_method }` + `Default` | `project/mod.rs:25-38` |
| `pub fn project(events, prices, config)` | `project/mod.rs:41-49` |
| `PoolSet`, `consume_fifo` | `project/pools.rs:21-101` (`consume_fifo` at `:58-100`) |
| `Consumed` struct | `project/pools.rs:104-118` |
| six `consume_fifo` sites | `fold.rs:232` (consume_fee), `:367` (Dispose), `:483` (PendingOut), `:526` (SelfTransfer), `:745` (GiftOut), `:811` (Donate) |
| four method-honoring sites | Dispose `:367`, SelfTransfer `:526`, GiftOut `:745`, Donate `:811` |
| two FIFO-pinned sites | consume_fee `:232`, PendingOut `:483` |
| `note_pre2025_once` (hard-codes "FIFO") | `fold.rs:28-41` (literal at `:38`) |
| `rehome_onto_lot` / `rehome_onto_disposal_leg` / `rehome_onto_removal_leg` | `fold.rs:186-211` |
| `fold` / `fold_event(eff, prices, config, pools, st, stats)` | `fold.rs:270-301` / `:307-314` |
| `resolve(events, prices, config) -> Resolution` | `project/resolve.rs:255-622` |
| `Resolution { timeline, transition, blockers }` | `resolve.rs:118-122` |
| `Op` enum (Dispose/GiftOut/Donate/SelfTransfer/PendingOut/…) | `resolve.rs:13-80` |
| `Eff { id, utc, tz, src_priority, src_ref, wallet, op }` + `date()` | `resolve.rs:82-96` |
| `voided` set / `allocation_voids` | `resolve.rs:269-303` (`allocation_voids` push `:284-290`) |
| `decisions` (sorted by seq) | `resolve.rs:311-318` |
| `universal_snapshot(timeline, prices, config)` call | `resolve.rs:520` |
| safe-harbor effectiveness loop | `resolve.rs:524-587` (snapshot use `:547`; seed build `:566-586`) |
| irrevocability of effective allocation | `resolve.rs:591-599` |
| multiple-effective → `DecisionConflict`; Path selection | `resolve.rs:602-615` |
| `seed_transition` / `universal_snapshot` | `project/transition.rs:54-82` / `:25-51` |
| `EventPayload` variants | `event.rs:182-202` |
| `SafeHarborAllocation { lots, as_of_date, method: AllocMethod, timely_allocation_attested }` | `event.rs:155-161` |
| `AllocMethod { ActualPosition, ProRata }` | `event.rs:139-143` |
| `AllocLot { wallet, sat, usd_basis, acquired_at, dual_loss_basis, donor_acquired_at }` | `event.rs:144-154` |
| `every_variant_serde_round_trips` (must gain new variants) | `event.rs:251-384` |
| `Lot { lot_id, wallet, acquired_at, original_sat, remaining_sat, usd_basis (gain), basis_source, dual_loss_basis, donor_acquired_at, basis_pending }` | `state.rs:57-69` |
| `BlockerKind` + `severity()` (Hard set) | `state.rs:22-49` |
| `EventId { Import{source,source_ref}, Conflict{source,source_ref,fingerprint}, Decision{seq} }` + `canonical()` | `identity.rs:55-106` |
| `LotId { origin_event_id, split_sequence }` | `identity.rs:116-120` |
| `WalletId::Exchange{provider,account}` / `SelfCustody{label}` | `identity.rs:109-113` |
| Path-B seed lots use the allocation `Decision` id as origin | `resolve.rs:570-574` |
| `persistence::fingerprint` (None for non-imported) / `append_decision` (None) | `persistence.rs:25-99` (`:96`) / `:238-262` (`:259`) |
| CLI `CliConfig` / `read_config` / `set_fee_treatment` / cli_config side-table | `crates/btctax-cli/src/config.rs:10-99` |
| `eventref::parse_event_id` / `parse_wallet_id` / `parse_usd_arg` / `parse_date_arg` | `crates/btctax-cli/src/eventref.rs:22-92` |
| reconcile emitters / `safe_harbor_allocate` / `safe_harbor_attest` | `crates/btctax-cli/src/cmd/reconcile.rs` (`safe_harbor_allocate` `:209-255`, `safe_harbor_attest` `:264-360`) |
| `inspect::verify` → `build_verify(&state, &events)` | `crates/btctax-cli/src/cmd/inspect.rs:27-31` |
| `VerifyReport` / `build_verify` / `render_verify` / `safe_harbor_status` | `crates/btctax-cli/src/render.rs:255-346`, `:473-526` |
| `Session::config()` (`CliConfig`) / `load_events_and_project` | `crates/btctax-cli/src/session.rs:69-95` |
| clap `Command::Config` / `Reconcile` subcommands / dispatch | `crates/btctax-cli/src/main.rs:47-153`, `:215-347` |
| `TRANSITION_DATE = 2025-01-01`, `TaxDate = Date`, `tax_date`, `fmv_of` | `conventions.rs:10,17,52`; `price.rs:13-18` |
| test helper patterns (`imp`/`dec_ev`/`buy`/`sell`/`alloc`) | `crates/btctax-core/tests/transition.rs:21-127` |

> **Naming adaptation (noted once):** the spec writes `MethodElection { … }` / `LotSelection { … }` inline. The codebase convention is a **named struct per payload** wrapped in a tuple variant (e.g. `EventPayload::Dispose(Dispose)`, `event.rs:186`). This plan therefore defines `struct MethodElection`, `struct LotSelection`, `struct LotPick` and variants `MethodElection(MethodElection)` / `LotSelection(LotSelection)`. Field names/types are exactly as the spec mandates.

---

## 2. New public API surface introduced by Sub-project A

- **Types (`btctax-core`):** `LotMethod { Fifo, Lifo, Hifo }` (was FIFO-only); `MethodElection { effective_from: TaxDate, method: LotMethod }`; `LotPick { lot: LotId, sat: Sat }`; `LotSelection { disposal_event: EventId, lots: Vec<LotPick> }`; `ComplianceStatus { StandingOrder{effective_from: TaxDate}, Contemporaneous, AttestedRecording, NonCompliant }`; `DisposalCompliance { disposal: EventId, wallet: WalletId, date: TaxDate, status: ComplianceStatus }`; `CandidateDisposal`, `EvaluateOutcome`, `EvaluateError` (A.6).
- **Event variants:** `EventPayload::MethodElection(MethodElection)`, `EventPayload::LotSelection(LotSelection)`. New field `SafeHarborAllocation.pre2025_method: LotMethod` (`#[serde(default)]`).
- **Blockers:** `BlockerKind::{LotSelectionInvalid, MethodElectionBackdated, Pre2025MethodConflictsAllocation}` — all `Severity::Hard`.
- **Engine fns:** `PoolSet::consume(key, need, method, selection) -> ConsumeResult`; `disposal_compliance(events, state) -> Vec<DisposalCompliance>`; `evaluate_disposal(events, prices, config, candidate, selection) -> Result<EvaluateOutcome, EvaluateError>`. `ProjectionConfig` gains `pre2025_method` (loses `lot_method`).
- **Config keys (`cli_config` side-table):** `pre2025_method` (`fifo|lifo|hifo`), `pre2025_method_attested` (`true|false`). The old mutable `lot_method` key/field is **removed**.
- **CLI commands:** `config --set-pre2025-method <m> [--attest-pre2025-method]`; `reconcile select-lots <disposal-eventref> --from <lotid>:<sat> [--from …]`; `reconcile import-selections <file.csv>`.
- **`evaluate` entrypoint signature:** `pub fn evaluate_disposal(events: &[LedgerEvent], prices: &dyn PriceProvider, config: &ProjectionConfig, candidate: &CandidateDisposal, selection: Option<&[LotPick]>) -> Result<EvaluateOutcome, EvaluateError>`.

---

## 3. Task list

1. **Method config substrate** — `LotMethod{Fifo,Lifo,Hifo}` (+ serde + `Default`); retire `lot_method` flag; add `pre2025_method` (+ attested) config & CLI.
2. **`PoolSet::consume(method, selection)`** — total-order FIFO/LIFO/HIFO + named-lot selection (pure, KATs). `consume_fifo` delegates.
3. **`MethodElection` + method ordering wired through the fold** — Universal→`pre2025_method`, Wallet→in-force election (FIFO before any); `MethodElectionBackdated`; `Pre2025MethodNote` renders the declared method.
4. **`LotSelection`/`LotPick` + selection wired through the fold** — principal conservation, targeting, dup→`DecisionConflict`, voided, per-wallet/existence → `LotSelectionInvalid`; fee FIFO from remainder.
5. **CLI `select-lots` + `import-selections` + `parse_lot_id`** — `--from <lotid>:<sat>`, CSV (header-validated), round-trips all three `EventId` origin variants.
6. **A.7 `pre2025_method` ↔ effective `SafeHarborAllocation`** — immutable serde-default field; method-aware `universal_snapshot`; `Pre2025MethodConflictsAllocation`; composition + conflict KATs.
7. **`DisposalCompliance` projection** — custody mapping + envelope; `disposal_compliance(events, state)`.
8. **`verify` surfacing** — declared method (+attested), election history, selection count, per-disposal compliance, new hard blockers partitioned.
9. **A.6 evaluate entrypoint** — side-effect-free synthetic/existing disposal fold; `--proceeds` required when no price.
10. **Whole-diff review + full-suite green** (Phase E gate).

Dependency order: 1 → 2 → 3 → 4 → 5; 6 depends on 1–3; 7 depends on 3–4; 8 depends on 3,6,7; 9 depends on 2–4. 10 last.

---

## TASK 1 — Method config substrate

**Goal.** Generalize `LotMethod` to `Fifo|Lifo|Hifo` (serde + `Default`); **remove** the mutable `lot_method` flag from `ProjectionConfig` and `CliConfig` (spec A.1/A-data-model: "the old mutable `lot_method` flag is **removed** in favor of the event"); add the attested historical-fact `pre2025_method` (+ `pre2025_method_attested`) config and its CLI surface.

**Files**
- modify `crates/btctax-core/src/project/mod.rs`
- modify `crates/btctax-cli/src/config.rs`
- modify `crates/btctax-cli/src/cmd/admin.rs`
- modify `crates/btctax-cli/src/main.rs`
- (no change needed) `crates/btctax-core/src/lib.rs` already `pub use project::{… LotMethod, ProjectionConfig}` (`lib.rs:15-17`)

**Interfaces**
```rust
// project/mod.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LotMethod { Fifo, Lifo, Hifo }
impl Default for LotMethod { fn default() -> Self { LotMethod::Fifo } }   // for #[serde(default)] in Task 6

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectionConfig {
    pub self_transfer_fee: FeeTreatment,
    pub pre2025_method: LotMethod,        // was: lot_method
}
// CLI config.rs
pub struct CliConfig { pub fee_treatment: FeeTreatment, pub pre2025_method: LotMethod, pub pre2025_method_attested: bool }
pub fn set_pre2025_method(conn: &Connection, m: LotMethod, attested: bool) -> Result<(), CliError>;
```

**Steps**

1. **Failing test** — extend `config.rs`'s `#[cfg(test)] mod tests` (replacing the stale `to_projection_carries_treatment_and_fifo` at `config.rs:137-144`):
```rust
#[test]
fn default_pre2025_method_is_fifo_unattested() {
    let c = mem();
    let cfg = read_config(&c).unwrap();
    assert!(matches!(cfg.pre2025_method, LotMethod::Hifo) == false);
    assert_eq!(cfg.pre2025_method, LotMethod::Fifo);
    assert!(!cfg.pre2025_method_attested);
    assert_eq!(cfg.to_projection().pre2025_method, LotMethod::Fifo);
}
#[test]
fn set_pre2025_method_round_trips_with_attestation() {
    let c = mem();
    set_pre2025_method(&c, LotMethod::Hifo, true).unwrap();
    let cfg = read_config(&c).unwrap();
    assert_eq!(cfg.pre2025_method, LotMethod::Hifo);
    assert!(cfg.pre2025_method_attested);
    assert_eq!(cfg.to_projection().pre2025_method, LotMethod::Hifo);
}
#[test]
fn bad_pre2025_method_value_is_an_error() {
    let c = mem();
    c.execute("INSERT INTO cli_config(key,value) VALUES('pre2025_method','zzz')", []).unwrap();
    assert!(matches!(read_config(&c).unwrap_err(),
        CliError::BadConfigValue { ref key, .. } if key == "pre2025_method"));
}
```
2. **Run → RED** (`set_pre2025_method`/`pre2025_method` do not exist; `to_projection` still yields `lot_method`).
3. **Minimal impl:**
   - `project/mod.rs`: rewrite `LotMethod` (3 variants + serde + `Default`); rewrite `ProjectionConfig` to `{ self_transfer_fee, pre2025_method }`; `Default` → `{ TreatmentC, LotMethod::Fifo }`.
   - `config.rs`: `CliConfig { fee_treatment, pre2025_method, pre2025_method_attested }`; `Default` → `{ TreatmentC, Fifo, false }`; `to_projection` → `ProjectionConfig { self_transfer_fee: self.fee_treatment, pre2025_method: self.pre2025_method }`. In `read_config`, after fee parsing, add:
```rust
fn lot_method_tag(m: LotMethod) -> &'static str { match m { LotMethod::Fifo=>"fifo", LotMethod::Lifo=>"lifo", LotMethod::Hifo=>"hifo" } }
// in read_config:
if let Some(v) = get(conn, "pre2025_method")? {
    cfg.pre2025_method = match v.as_str() {
        "fifo" => LotMethod::Fifo, "lifo" => LotMethod::Lifo, "hifo" => LotMethod::Hifo,
        _ => return Err(CliError::BadConfigValue { key: "pre2025_method".into(), value: v }),
    };
}
if let Some(v) = get(conn, "pre2025_method_attested")? {
    cfg.pre2025_method_attested = match v.as_str() {
        "true" => true, "false" => false,
        _ => return Err(CliError::BadConfigValue { key: "pre2025_method_attested".into(), value: v }),
    };
}
// new setter:
pub fn set_pre2025_method(conn: &Connection, m: LotMethod, attested: bool) -> Result<(), CliError> {
    conn.execute("INSERT INTO cli_config(key,value) VALUES('pre2025_method',?1)
                  ON CONFLICT(key) DO UPDATE SET value=excluded.value", [lot_method_tag(m)])?;
    conn.execute("INSERT INTO cli_config(key,value) VALUES('pre2025_method_attested',?1)
                  ON CONFLICT(key) DO UPDATE SET value=excluded.value", [if attested {"true"} else {"false"}])?;
    Ok(())
}
```
   - `cmd/admin.rs`: add `pub fn set_pre2025_method(vault_path, pp, m: LotMethod, attested: bool) -> Result<CliConfig, CliError>` (open session, call `config::set_pre2025_method(session.conn(), m, attested)?`, `session.save()?`, return `session.config()?`). Keep `set_config` (fee) as-is.
   - `main.rs`: extend `Command::Config` to `{ set_fee_treatment: Option<FeeArg>, set_pre2025_method: Option<MethodLotArg>, attest_pre2025_method: bool }`; add `#[derive(Copy,Clone,ValueEnum)] enum MethodLotArg { Fifo, Lifo, Hifo }`. In the `Config` arm, branch on which is set (fee vs method), call the right admin fn, and change the print to:
```rust
println!("fee_treatment: {:?}\npre2025_method: {:?} (attested: {})",
         cfg.fee_treatment, cfg.pre2025_method, cfg.pre2025_method_attested);
```
4. **Run → GREEN.** Full suite: this compiles-breaks every `ProjectionConfig{ .., lot_method }` / `cfg.lot_method` reference — fix the only ones (grounding §1: `config.rs`, `main.rs`, `project/mod.rs`; no fold/resolve references `lot_method`). `transition.rs`/`kat_tax.rs` use `ProjectionConfig::default()` / `..ProjectionConfig::default()` so they keep compiling.
5. **Commit:** `feat(core,cli): LotMethod{Fifo,Lifo,Hifo}; retire lot_method flag; add attested pre2025_method config`.

---

## TASK 2 — `PoolSet::consume(method, selection)` (pure pool engine)

**Goal.** Generalize consumption to a **total-order** method (FIFO/LIFO/HIFO) and named-lot selection; keep `consume_fifo` as the FIFO wrapper for the two pinned sites. Pure pool logic; KATs at the pool level. Introduce `LotPick` in `event.rs` (used here; the `LotSelection` payload follows in Task 4).

**Files**
- modify `crates/btctax-core/src/event.rs` (add `LotPick` struct + serde + round-trip)
- modify `crates/btctax-core/src/project/pools.rs` (add `consume`, `ConsumeResult`, ordering; `consume_fifo` delegates; `#[cfg(test)] mod tests`)

**Interfaces**
```rust
// event.rs
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LotPick { pub lot: LotId, pub sat: Sat }

// pools.rs
pub struct ConsumeResult { pub consumed: Vec<Consumed>, pub shortfall: Sat, pub selection_error: Option<String> }
impl PoolSet {
    pub fn consume(&mut self, key: &PoolKey, need: Sat, method: LotMethod, selection: Option<&[LotPick]>) -> ConsumeResult;
    pub fn consume_fifo(&mut self, key: &PoolKey, need: Sat) -> (Vec<Consumed>, Sat); // delegates to consume(.., Fifo, None)
}
```

**Steps**

1. **Failing tests** — add to `pools.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::conventions::Usd;
    use crate::event::{BasisSource, LotPick};
    use crate::identity::{EventId, LotId, Source, SourceRef, WalletId};
    use crate::LotMethod;
    use rust_decimal_macros::dec;
    use time::macros::date;

    fn w() -> WalletId { WalletId::SelfCustody { label: "x".into() } }
    fn lot(rf: &str, acq: time::Date, sat: i64, basis: Usd) -> Lot {
        Lot {
            lot_id: LotId { origin_event_id: EventId::import(Source::Coinbase, SourceRef::new(rf)), split_sequence: 0 },
            wallet: w(), acquired_at: acq, original_sat: sat, remaining_sat: sat,
            usd_basis: basis, basis_source: BasisSource::ExchangeProvided,
            dual_loss_basis: None, donor_acquired_at: None, basis_pending: false,
        }
    }
    fn pid(rf: &str) -> LotId { LotId { origin_event_id: EventId::import(Source::Coinbase, SourceRef::new(rf)), split_sequence: 0 } }
    // Three lots whose method orders are all DISTINCT:
    //  A 2025-02-01 basis $50 ; B 2025-03-01 basis $90 (highest) ; C 2025-04-01 basis $40
    //  FIFO -> A,B,C ; LIFO -> C,B,A ; HIFO -> B,A,C
    fn three() -> PoolSet {
        let mut p = PoolSet::default();
        p.push_lot(PoolKey::Universal, lot("A", date!(2025-02-01), 100_000, dec!(50.00)));
        p.push_lot(PoolKey::Universal, lot("B", date!(2025-03-01), 100_000, dec!(90.00)));
        p.push_lot(PoolKey::Universal, lot("C", date!(2025-04-01), 100_000, dec!(40.00)));
        p
    }

    #[test]
    fn fifo_consumes_oldest_first() {
        let r = three().consume(&PoolKey::Universal, 100_000, LotMethod::Fifo, None);
        assert_eq!(r.shortfall, 0);
        assert_eq!(r.consumed[0].lot_id, pid("A"));
    }
    #[test]
    fn lifo_consumes_newest_first() {
        let r = three().consume(&PoolKey::Universal, 100_000, LotMethod::Lifo, None);
        assert_eq!(r.consumed[0].lot_id, pid("C"));
    }
    #[test]
    fn hifo_consumes_highest_gain_basis_per_sat_first() {
        let r = three().consume(&PoolKey::Universal, 100_000, LotMethod::Hifo, None);
        assert_eq!(r.consumed[0].lot_id, pid("B"));
    }
    #[test]
    fn hifo_basis_pending_sorts_last() {
        let mut p = PoolSet::default();
        let mut pend = lot("P", date!(2025-01-01), 100_000, dec!(0)); pend.basis_pending = true; // usd_basis == 0
        p.push_lot(PoolKey::Universal, pend);
        p.push_lot(PoolKey::Universal, lot("Q", date!(2025-06-01), 100_000, dec!(10.00)));
        let r = p.consume(&PoolKey::Universal, 100_000, LotMethod::Hifo, None);
        assert_eq!(r.consumed[0].lot_id, pid("Q")); // pending ($0) sorts last
    }
    #[test]
    fn hifo_ties_break_oldest_then_lotid() {
        let mut p = PoolSet::default();
        p.push_lot(PoolKey::Universal, lot("OLD", date!(2025-02-01), 100_000, dec!(50.00)));
        p.push_lot(PoolKey::Universal, lot("NEW", date!(2025-05-01), 100_000, dec!(50.00))); // same per-sat
        let r = p.consume(&PoolKey::Universal, 100_000, LotMethod::Hifo, None);
        assert_eq!(r.consumed[0].lot_id, pid("OLD"));
    }
    #[test]
    fn hifo_ignores_dual_loss_basis() {
        // Same gain basis per sat; only dual_loss_basis differs -> order must NOT change (oldest first).
        let mut p = PoolSet::default();
        let mut g = lot("G", date!(2025-02-01), 100_000, dec!(50.00)); g.dual_loss_basis = Some(dec!(5.00));
        p.push_lot(PoolKey::Universal, g);
        p.push_lot(PoolKey::Universal, lot("H", date!(2025-05-01), 100_000, dec!(50.00)));
        let r = p.consume(&PoolKey::Universal, 100_000, LotMethod::Hifo, None);
        assert_eq!(r.consumed[0].lot_id, pid("G")); // keyed on usd_basis only; tie -> oldest
    }
    #[test]
    fn selection_consumes_exactly_named_lots() {
        let picks = vec![LotPick { lot: pid("C"), sat: 100_000 }, LotPick { lot: pid("A"), sat: 100_000 }];
        let r = three().consume(&PoolKey::Universal, 200_000, LotMethod::Hifo, Some(&picks));
        assert!(r.selection_error.is_none());
        assert_eq!(r.consumed.iter().map(|c| c.lot_id.clone()).collect::<Vec<_>>(), vec![pid("C"), pid("A")]);
    }
    #[test]
    fn selection_unknown_lot_reports_error_and_falls_back_to_method() {
        let picks = vec![LotPick { lot: pid("ZZZ"), sat: 100_000 }];
        let r = three().consume(&PoolKey::Universal, 100_000, LotMethod::Fifo, Some(&picks));
        assert!(r.selection_error.is_some());
        assert_eq!(r.consumed[0].lot_id, pid("A")); // fell back to FIFO order (sats conserved)
    }
    #[test]
    fn selection_insufficient_remaining_reports_error() {
        let picks = vec![LotPick { lot: pid("A"), sat: 999_999 }];
        let r = three().consume(&PoolKey::Universal, 999_999, LotMethod::Fifo, Some(&picks));
        assert!(r.selection_error.is_some());
    }
    #[test]
    fn selection_cross_wallet_lot_reports_error() {
        let mut p = PoolSet::default();
        p.push_lot(PoolKey::Wallet(WalletId::SelfCustody { label: "a".into() }), lot("A", date!(2025-02-01), 100_000, dec!(50.00)));
        p.push_lot(PoolKey::Wallet(WalletId::SelfCustody { label: "b".into() }), lot("B", date!(2025-02-01), 100_000, dec!(50.00)));
        // disposal is in wallet "a"; pick references lot "B" living in wallet "b" -> cross-account ID forbidden.
        let picks = vec![LotPick { lot: pid("B"), sat: 100_000 }];
        let r = p.consume(&PoolKey::Wallet(WalletId::SelfCustody { label: "a".into() }), 100_000, LotMethod::Fifo, Some(&picks));
        assert!(r.selection_error.as_deref().unwrap().contains("wallet"));
    }
}
```
2. **Run → RED** (`consume`/`ConsumeResult` absent).
3. **Minimal impl** in `pools.rs` (imports: add `use crate::event::LotPick;`, `use crate::LotMethod;`, `use std::cmp::Ordering;`):
```rust
pub struct ConsumeResult { pub consumed: Vec<Consumed>, pub shortfall: Sat, pub selection_error: Option<String> }

/// Total-order ranking key (NFR4): returns indices into `lots` (remaining>0 only) in consumption order.
fn method_order(lots: &[Lot], method: LotMethod) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..lots.len()).filter(|&i| lots[i].remaining_sat > 0).collect();
    match method {
        LotMethod::Fifo => idx.sort_by(|&a, &b|
            lots[a].acquired_at.cmp(&lots[b].acquired_at).then(lots[a].lot_id.cmp(&lots[b].lot_id))),
        LotMethod::Lifo => idx.sort_by(|&a, &b|
            lots[b].acquired_at.cmp(&lots[a].acquired_at).then(lots[b].lot_id.cmp(&lots[a].lot_id))),
        LotMethod::Hifo => idx.sort_by(|&a, &b| hifo_cmp(&lots[a], &lots[b])),
    }
    idx
}

/// HIFO key: gain basis (`usd_basis`) per sat DESC; basis-pending (usd_basis==0) LAST; ties -> oldest, then lot_id.
/// Cross-multiplied (NFR5: exact Decimal, no float). Loss-basis (`dual_loss_basis`) is intentionally ignored.
fn hifo_cmp(a: &Lot, b: &Lot) -> Ordering {
    let (az, bz) = (a.usd_basis == Usd::ZERO, b.usd_basis == Usd::ZERO); // N2: avoid num_traits::Zero scope dep; match fold.rs's `> Usd::ZERO` idiom
    match (az, bz) { (true, false) => return Ordering::Greater, (false, true) => return Ordering::Less, _ => {} }
    let lhs = a.usd_basis * Usd::from(b.remaining_sat);   // a.perSat vs b.perSat, no division
    let rhs = b.usd_basis * Usd::from(a.remaining_sat);
    rhs.cmp(&lhs)                                          // DESC: higher per-sat first
        .then(a.acquired_at.cmp(&b.acquired_at))
        .then(a.lot_id.cmp(&b.lot_id))
}

impl PoolSet {
    pub fn consume_fifo(&mut self, key: &PoolKey, need: Sat) -> (Vec<Consumed>, Sat) {
        let r = self.consume(key, need, LotMethod::Fifo, None);
        (r.consumed, r.shortfall)
    }

    pub fn consume(&mut self, key: &PoolKey, need: Sat, method: LotMethod, selection: Option<&[LotPick]>) -> ConsumeResult {
        // ---- selection path: validate feasibility within THIS pool; fall back to method order on failure ----
        if let Some(picks) = selection {
            if let Err(reason) = self.selection_feasible(key, picks) {
                let (consumed, shortfall) = self.consume_ordered(key, need, method);
                return ConsumeResult { consumed, shortfall, selection_error: Some(reason) };
            }
            let (consumed, shortfall) = self.consume_picks(key, picks);
            return ConsumeResult { consumed, shortfall, selection_error: None };
        }
        // ---- method path ----
        let (consumed, shortfall) = self.consume_ordered(key, need, method);
        ConsumeResult { consumed, shortfall, selection_error: None }
    }

    fn selection_feasible(&self, key: &PoolKey, picks: &[LotPick]) -> Result<(), String> {
        let pool = self.pools.get(key).map(Vec::as_slice).unwrap_or(&[]);
        // tentative per-lot remaining (handles multiple picks of one lot)
        let mut rem: BTreeMap<LotId, Sat> = BTreeMap::new();
        for l in pool { if l.remaining_sat > 0 { *rem.entry(l.lot_id.clone()).or_insert(0) += l.remaining_sat; } }
        for p in picks {
            match rem.get_mut(&p.lot) {
                None => {
                    // distinguish cross-wallet (lot exists in another pool) from truly unknown, for a precise reason
                    let elsewhere = self.pools.iter().any(|(k, v)| k != key && v.iter().any(|l| l.lot_id == p.lot));
                    return Err(if elsewhere {
                        format!("picked lot {}#{} is in another wallet — cross-account identification is not permitted (§1.1012-1(j))",
                                p.lot.origin_event_id.canonical(), p.lot.split_sequence)
                    } else {
                        format!("picked lot {}#{} does not exist", p.lot.origin_event_id.canonical(), p.lot.split_sequence)
                    });
                }
                Some(r) if *r < p.sat => return Err(format!(
                    "picked lot {}#{} has {} sat remaining < {} requested", p.lot.origin_event_id.canonical(), p.lot.split_sequence, *r, p.sat)),
                Some(r) => { *r -= p.sat; }
            }
        }
        Ok(())
    }

    fn consume_picks(&mut self, key: &PoolKey, picks: &[LotPick]) -> (Vec<Consumed>, Sat) {
        let mut out = Vec::new();
        if let Some(lots) = self.pools.get_mut(key) {
            for p in picks {
                let mut take = p.sat;
                for lot in lots.iter_mut() {
                    if take <= 0 { break; }
                    if lot.lot_id != p.lot || lot.remaining_sat <= 0 { continue; }
                    let t = take.min(lot.remaining_sat);
                    out.push(Self::take_from(lot, t));
                    take -= t;
                }
            }
            lots.retain(|l| l.remaining_sat > 0);
        }
        (out, 0) // feasibility already guaranteed by selection_feasible
    }

    fn consume_ordered(&mut self, key: &PoolKey, need: Sat, method: LotMethod) -> (Vec<Consumed>, Sat) {
        let mut out = Vec::new();
        let mut remaining = need;
        if let Some(lots) = self.pools.get_mut(key) {
            for i in method_order(lots, method) {
                if remaining <= 0 { break; }
                let lot = &mut lots[i];
                if lot.remaining_sat <= 0 { continue; }
                let take = remaining.min(lot.remaining_sat);
                out.push(Self::take_from(lot, take));
                remaining -= take;
            }
            lots.retain(|l| l.remaining_sat > 0);
        }
        (out, remaining)
    }

    /// Take `take` sat from `lot`, returning the Consumed fragment and reducing the lot (conserves Σbasis).
    /// Body is the exact arithmetic of the previous `consume_fifo` (pools.rs:69-93).
    fn take_from(lot: &mut Lot, take: Sat) -> Consumed {
        let (gain_basis, _rest) = split_pro_rata(lot.usd_basis, take, lot.remaining_sat);
        let loss_basis = lot.dual_loss_basis.map(|l| split_pro_rata(l, take, lot.remaining_sat).0);
        let c = Consumed {
            lot_id: lot.lot_id.clone(), sat: take, gain_basis, loss_basis,
            gain_hp_start: lot.gain_hp_start(), loss_hp_start: lot.loss_hp_start(),
            basis_source: lot.basis_source, dual: lot.dual_loss_basis.is_some(),
            basis_pending: lot.basis_pending, wallet: lot.wallet.clone(),
            acquired_at: lot.acquired_at, donor_acquired_at: lot.donor_acquired_at,
        };
        lot.usd_basis -= gain_basis;
        if let (Some(dl), Some(taken)) = (lot.dual_loss_basis.as_mut(), loss_basis) { *dl -= taken; }
        lot.remaining_sat -= take;
        c
    }
}
```
   Delete the old `consume_fifo` body (`pools.rs:58-100`); it is now the delegating wrapper above. Add a `LotPick` round-trip case to `event.rs` `every_variant_serde_round_trips` (or a small dedicated `#[test]`).
4. **Run → GREEN.** Whole suite.

   **DELIBERATE CORRECTNESS CHANGE — acquisition-date FIFO replaces insertion-order FIFO (C1; NOT a no-op).** `consume_fifo` no longer walks raw push-order; it delegates to `consume(.., Fifo, None)` → `method_order`, i.e. **acquisition-date order** (`acquired_at` asc, tie `lot_id` asc), at **all six** consume sites (the four method-honoring + the two FIFO-pinned, `consume_fee`/PendingOut). This is the legally-correct FIFO — §1.1012-1(j)(3)(i) sells in order of **earliest acquisition**, and a relocated lot carries its **original** `acquired_at` (TP7/TP8(c): a self-transfer is *not* a new acquisition, `fold.rs:545` sets `acquired_at: c.acquired_at`). It is **NOT equivalent** to the legacy front-walk and **no equivalence is claimed**: on every path that pushes a lot whose `acquired_at` is older than lots already in the destination `Vec`, the two orders **diverge** — and the foundation currently ships **insertion-order FIFO for relocated lots, a latent §1012 deviation** this change corrects. The three reachable divergence paths (all verified against current source):
   - **SelfTransfer relocation** into a populated wallet — relocated fragment built with the original `acquired_at` and `push_lot`'d to the **back** of the destination pool (`fold.rs:537-553,580-583`);
   - **Path-B multi-lot seeding** in non-`acquired_at` order — seed lots pushed in alloc-index order (`resolve.rs:566-586` → `transition.rs:67-73`);
   - **pre-2025 SelfTransfer** reordering the single Universal pool, shifting the `universal_snapshot` residue `Σ usd_basis` = `snap.basis` against which safe-harbor conservation is checked (`transition.rs:25-51`; guard `resolve.rs:546-547`).

   These change consumed basis/term **and** the safe-harbor conservation residue. They are locked by the new RED→GREEN divergence KATs (SelfTransfer relocation FIFO/LIFO/HIFO in Task 3; Path-B non-`acquired_at` seeding + pre-2025-relocation `snap.basis` in Task 6), not by "watch the suite" — the legacy suite does **not** detect the divergence (existing fixtures relocate into empty wallets or list seed lots already in `acquired_at` order), which is precisely why explicit KATs are mandatory.

   **Fixture re-verification step (mandatory; each changed value is a documented correctness fix, NEVER a silent edit).** The ordering switch lands **here** (the moment `consume_fifo`'s body changes), so before commit **re-verify every existing self-transfer / TransferLink / Path-B / safe-harbor fixture** under the new order: `crates/btctax-core/tests/kat_tax.rs` (self-transfer relocation, TP8(c) fee-rehome), `crates/btctax-core/tests/transition.rs` (Path-B seed, `universal_snapshot`, `path_b_seeded_lot_relocation_no_lotid_collision` :733), `crates/btctax-core/tests/properties.rs` (Σ-basis invariants — order-invariant, but re-run), plus any other fixture touching relocated/seeded lots. For each golden value that **moves**, update it **in-line with a comment naming the tax reason** (e.g. `// relocated lot Z (acquired_at 2025-01-01) is consumed before directly-acquired A under acquisition-date FIFO, §1.1012-1(j)(3)(i) — corrects insertion-order`), recorded as a **correctness improvement**. Confirm `conservation_report` stays balanced on every fixture.
5. **Commit:** `feat(core): PoolSet::consume(method, selection) — total-order FIFO/LIFO/HIFO + named-lot selection; acquisition-date FIFO corrects insertion-order for relocated lots (C1)`.

---

## TASK 3 — `MethodElection` + method ordering wired through the fold

**Goal.** Add the dated standing-order decision; wire the four honoring sites to consume by the **applicable method** (Universal → `pre2025_method`; Wallet → in-force `MethodElection` at the disposal's tax date, else FIFO). Reject back-dated / pre-transition elections (`MethodElectionBackdated`, Hard) and exclude voided. Update `Pre2025MethodNote` to render the declared method. PendingOut + `consume_fee` stay FIFO.

**Files**
- modify `crates/btctax-core/src/event.rs` (`MethodElection` struct + variant; import `LotMethod`; serde round-trip)
- modify `crates/btctax-core/src/state.rs` (`BlockerKind::{MethodElectionBackdated, LotSelectionInvalid}` + `Hard` severity)
- modify `crates/btctax-core/src/project/resolve.rs` (`ElectionRec`; collect elections; `Resolution.elections` + empty `Resolution.selections`; pass to fold/snapshot)
- modify `crates/btctax-core/src/project/fold.rs` (`FoldCtx`; `applicable_method`; `consume_principal`; wire four sites; `note_pre2025_once` renders the method)
- modify `crates/btctax-core/src/project/transition.rs` (`universal_snapshot`/`fold_event` take the new ctx args)
- create `crates/btctax-core/tests/method_election.rs`

**Interfaces**
```rust
// event.rs
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MethodElection { pub effective_from: TaxDate, pub method: LotMethod }
// + EventPayload::MethodElection(MethodElection)

// state.rs — BlockerKind gains MethodElectionBackdated, LotSelectionInvalid (both Hard).

// resolve.rs
#[derive(Debug, Clone)]
pub struct ElectionRec { pub effective_from: TaxDate, pub method: LotMethod, pub decision_seq: u64 }
pub struct Resolution {
    pub timeline: Vec<Eff>, pub transition: TransitionMode, pub blockers: Vec<Blocker>,
    pub elections: Vec<ElectionRec>,
    pub selections: std::collections::BTreeMap<EventId, Vec<crate::event::LotPick>>, // populated in Task 4
}

// fold.rs
pub(crate) struct FoldCtx<'a> {
    pub config: &'a ProjectionConfig,
    pub elections: &'a [ElectionRec],
    pub selections: &'a std::collections::BTreeMap<EventId, Vec<crate::event::LotPick>>,
}
pub(crate) fn fold_event(eff: &Eff, prices: &dyn PriceProvider, ctx: &FoldCtx, pools: &mut PoolSet, st: &mut LedgerState, stats: &mut FoldStats);

// transition.rs
pub fn universal_snapshot(timeline: &[Eff], prices: &dyn PriceProvider, config: &ProjectionConfig,
                          elections: &[ElectionRec], selections: &BTreeMap<EventId, Vec<LotPick>>) -> UniversalSnapshot;
```

**Steps**

1. **Failing tests** — `crates/btctax-core/tests/method_election.rs` (helpers duplicated per the codebase's per-crate-test convention, mirroring `tests/transition.rs:21-127`):
```rust
use btctax_core::event::*;
use btctax_core::identity::*;
use btctax_core::price::StaticPrices;
use btctax_core::project::{project, ProjectionConfig};
use btctax_core::state::*;
use btctax_core::LotMethod;
use rust_decimal_macros::dec;
use time::macros::{date, datetime, offset};

fn w() -> WalletId { WalletId::Exchange { provider: "cb".into(), account: "m".into() } }
fn imp(rf: &str, ts: time::OffsetDateTime, p: EventPayload) -> LedgerEvent {
    LedgerEvent { id: EventId::import(Source::Coinbase, SourceRef::new(rf)), utc_timestamp: ts, original_tz: offset!(+00:00), wallet: Some(w()), payload: p }
}
fn dec_ev(seq: u64, ts: time::OffsetDateTime, p: EventPayload) -> LedgerEvent {
    LedgerEvent { id: EventId::decision(seq), utc_timestamp: ts, original_tz: offset!(+00:00), wallet: None, payload: p }
}
fn buy(rf: &str, ts: time::OffsetDateTime, sat: i64, cost: rust_decimal::Decimal) -> LedgerEvent {
    imp(rf, ts, EventPayload::Acquire(Acquire { sat, usd_cost: cost, fee_usd: dec!(0), basis_source: BasisSource::ExchangeProvided }))
}
fn sell(rf: &str, ts: time::OffsetDateTime, sat: i64, proceeds: rust_decimal::Decimal) -> LedgerEvent {
    imp(rf, ts, EventPayload::Dispose(Dispose { sat, usd_proceeds: proceeds, fee_usd: dec!(0), kind: DisposeKind::Sell }))
}
fn election(seq: u64, made: time::OffsetDateTime, eff: time::Date, m: LotMethod) -> LedgerEvent {
    dec_ev(seq, made, EventPayload::MethodElection(MethodElection { effective_from: eff, method: m }))
}
fn has(st: &LedgerState, k: BlockerKind) -> bool { st.blockers.iter().any(|b| b.kind == k) }

// Post-2025 pool with 3 lots whose method orders are distinct (FIFO->A, LIFO->C, HIFO->B).
fn three_post2025() -> Vec<LedgerEvent> {
    vec![
        buy("A", datetime!(2025-02-01 00:00:00 UTC), 100_000, dec!(50.00)),
        buy("B", datetime!(2025-03-01 00:00:00 UTC), 100_000, dec!(90.00)),
        buy("C", datetime!(2025-04-01 00:00:00 UTC), 100_000, dec!(40.00)),
    ]
}

#[test]
fn election_applies_on_or_after_effective_from_else_fifo() {
    let mut evs = three_post2025();
    // HIFO standing order recorded 2025-05-01, effective 2025-06-01.
    evs.push(election(1, datetime!(2025-05-01 00:00:00 UTC), date!(2025-06-01), LotMethod::Hifo));
    // Disposal BEFORE effective_from -> FIFO (consumes A).
    evs.push(sell("D1", datetime!(2025-05-15 00:00:00 UTC), 100_000, dec!(70.00)));
    // Disposal ON/AFTER effective_from -> HIFO (of what remains: B then C; picks B).
    evs.push(sell("D2", datetime!(2025-07-01 00:00:00 UTC), 100_000, dec!(95.00)));
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(!has(&st, BlockerKind::MethodElectionBackdated));
    let d1 = st.disposals.iter().find(|d| d.event == EventId::import(Source::Coinbase, SourceRef::new("D1"))).unwrap();
    assert_eq!(d1.legs[0].basis, dec!(50.00)); // FIFO -> A
    let d2 = st.disposals.iter().find(|d| d.event == EventId::import(Source::Coinbase, SourceRef::new("D2"))).unwrap();
    assert_eq!(d2.legs[0].basis, dec!(90.00)); // HIFO -> B
}

#[test]
fn latest_in_force_election_wins() {
    let mut evs = three_post2025();
    evs.push(election(1, datetime!(2025-01-02 00:00:00 UTC), date!(2025-01-02), LotMethod::Lifo));  // effective first
    evs.push(election(2, datetime!(2025-05-01 00:00:00 UTC), date!(2025-06-01), LotMethod::Hifo));  // later, governs after
    evs.push(sell("D", datetime!(2025-07-01 00:00:00 UTC), 100_000, dec!(95.00)));
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    let d = &st.disposals[0];
    assert_eq!(d.legs[0].basis, dec!(90.00)); // latest-in-force HIFO -> B
}

#[test]
fn backdated_election_is_rejected() {
    let mut evs = three_post2025();
    // effective_from (2025-02-10) precedes the made-date (2025-05-01) -> backdated.
    evs.push(election(1, datetime!(2025-05-01 00:00:00 UTC), date!(2025-02-10), LotMethod::Hifo));
    evs.push(sell("D", datetime!(2025-07-01 00:00:00 UTC), 100_000, dec!(95.00)));
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(has(&st, BlockerKind::MethodElectionBackdated));
    assert_eq!(st.disposals[0].legs[0].basis, dec!(50.00)); // rejected election -> FIFO -> A
}

#[test]
fn pre_transition_election_is_rejected() {
    let mut evs = three_post2025();
    evs.push(election(1, datetime!(2024-06-01 00:00:00 UTC), date!(2024-06-01), LotMethod::Hifo)); // effective_from < TRANSITION_DATE
    evs.push(sell("D", datetime!(2025-07-01 00:00:00 UTC), 100_000, dec!(95.00)));
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(has(&st, BlockerKind::MethodElectionBackdated));
    assert_eq!(st.disposals[0].legs[0].basis, dec!(50.00)); // FIFO default
}

#[test]
fn voided_election_is_excluded() {
    let mut evs = three_post2025();
    evs.push(election(1, datetime!(2025-01-02 00:00:00 UTC), date!(2025-01-02), LotMethod::Hifo));
    evs.push(dec_ev(2, datetime!(2025-06-01 00:00:00 UTC), EventPayload::VoidDecisionEvent(VoidDecisionEvent { target_event_id: EventId::decision(1) })));
    evs.push(sell("D", datetime!(2025-07-01 00:00:00 UTC), 100_000, dec!(95.00)));
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert_eq!(st.disposals[0].legs[0].basis, dec!(50.00)); // voided HIFO -> back to FIFO -> A
}

#[test]
fn pre2025_universal_uses_pre2025_method() {
    // Pre-2025 pool A/B/C in Universal; pre-2025 sell under HIFO consumes B.
    let evs = vec![
        buy("A", datetime!(2024-02-01 00:00:00 UTC), 100_000, dec!(50.00)),
        buy("B", datetime!(2024-03-01 00:00:00 UTC), 100_000, dec!(90.00)),
        buy("C", datetime!(2024-04-01 00:00:00 UTC), 100_000, dec!(40.00)),
        sell("D", datetime!(2024-09-01 00:00:00 UTC), 100_000, dec!(95.00)),
    ];
    let cfg = ProjectionConfig { pre2025_method: LotMethod::Hifo, ..ProjectionConfig::default() };
    let st = project(&evs, &StaticPrices::default(), &cfg);
    assert_eq!(st.disposals[0].legs[0].basis, dec!(90.00)); // HIFO -> B
}

#[test]
fn pre2025_method_note_renders_declared_method() {
    let evs = vec![
        buy("A", datetime!(2024-02-01 00:00:00 UTC), 100_000, dec!(50.00)),
        sell("D", datetime!(2024-09-01 00:00:00 UTC), 50_000, dec!(40.00)),
    ];
    let cfg = ProjectionConfig { pre2025_method: LotMethod::Hifo, ..ProjectionConfig::default() };
    let st = project(&evs, &StaticPrices::default(), &cfg);
    let note = st.blockers.iter().find(|b| b.kind == BlockerKind::Pre2025MethodNote).unwrap();
    assert!(note.detail.contains("HIFO"), "note must name the declared method, got: {}", note.detail);
}

// ── C1 divergence KAT (a) — acquisition-date FIFO vs legacy insertion-order on a RELOCATED lot ──
// A confirmed SelfTransfer relocates the OLDER lot Z (acquired 2025-01-01, basis $40) from COLD into HOT,
// which already holds the NEWER directly-acquired A (acquired 2025-08-01, basis $80). Z' carries its original
// acquired_at and is push_lot'd AFTER A, so HOT's insertion order is [A, Z'] while acquisition order is
// [Z', A]. A partial FIFO Dispose MUST consume the OLDER Z' first (legacy insertion-order wrongly took A).
// Basis AND term flip. LIFO/HIFO variants over the same fixture pin the full total order (both pick A).
#[test]
fn relocated_older_lot_consumed_first_under_acq_date_fifo_diverging_from_insertion_order() {
    let hot = WalletId::Exchange { provider: "cb".into(), account: "hot".into() };
    let cold = WalletId::SelfCustody { label: "cold".into() };
    let acq = |rf: &str, ts: time::OffsetDateTime, wal: &WalletId, cost: rust_decimal::Decimal| LedgerEvent {
        id: EventId::import(Source::Coinbase, SourceRef::new(rf)), utc_timestamp: ts, original_tz: offset!(+00:00),
        wallet: Some(wal.clone()),
        payload: EventPayload::Acquire(Acquire { sat: 100_000, usd_cost: cost, fee_usd: dec!(0), basis_source: BasisSource::ExchangeProvided }),
    };
    let scenario = |extra: Vec<LedgerEvent>| -> LedgerState {
        let mut evs = vec![
            acq("Z", datetime!(2025-01-01 00:00:00 UTC), &cold, dec!(40.00)),  // COLD, OLDER, $40
            acq("A", datetime!(2025-08-01 00:00:00 UTC), &hot,  dec!(80.00)),  // HOT,  NEWER, $80
            LedgerEvent { id: EventId::import(Source::Swan, SourceRef::new("OUT")),
                utc_timestamp: datetime!(2025-09-01 00:00:00 UTC), original_tz: offset!(+00:00), wallet: Some(cold.clone()),
                payload: EventPayload::TransferOut(TransferOut { sat: 100_000, fee_sat: None, dest_addr: None, txid: None }) },
            dec_ev(1, datetime!(2025-09-02 00:00:00 UTC), EventPayload::TransferLink(TransferLink {
                out_event: EventId::import(Source::Swan, SourceRef::new("OUT")),
                in_event_or_wallet: TransferTarget::Wallet(hot.clone()) })),     // relocate Z' -> HOT (pushed AFTER A)
        ];
        evs.extend(extra);
        evs.push(LedgerEvent { id: EventId::import(Source::Coinbase, SourceRef::new("D")),
            utc_timestamp: datetime!(2026-02-01 00:00:00 UTC), original_tz: offset!(+00:00), wallet: Some(hot.clone()),
            payload: EventPayload::Dispose(Dispose { sat: 100_000, usd_proceeds: dec!(150.00), fee_usd: dec!(0), kind: DisposeKind::Sell }) });
        project(&evs, &StaticPrices::default(), &ProjectionConfig::default())
    };
    let leg0 = |st: &LedgerState| st.disposals.iter()
        .find(|d| d.event == EventId::import(Source::Coinbase, SourceRef::new("D"))).unwrap().legs[0].clone();

    // FIFO (no election): acquisition-date FIFO consumes the OLDER relocated Z' — basis $40, LT (2025-01-01→2026-02-01).
    let l = leg0(&scenario(vec![]));
    assert_eq!(l.basis, dec!(40.00), "legacy insertion-order FIFO would have wrongly picked A ($80)");
    assert_eq!(l.term, Term::LongTerm);
    // LIFO: newest acquisition first -> A ($80), ST (2025-08-01→2026-02-01).
    let l = leg0(&scenario(vec![election(2, datetime!(2025-10-01 00:00:00 UTC), date!(2025-10-01), LotMethod::Lifo)]));
    assert_eq!(l.basis, dec!(80.00));
    assert_eq!(l.term, Term::ShortTerm);
    // HIFO: highest gain-basis/sat first -> A ($80 > $40), ST.
    let l = leg0(&scenario(vec![election(2, datetime!(2025-10-01 00:00:00 UTC), date!(2025-10-01), LotMethod::Hifo)]));
    assert_eq!(l.basis, dec!(80.00));
    assert_eq!(l.term, Term::ShortTerm);
}
```
   Also add a `MethodElection` case to `event.rs::every_variant_serde_round_trips` and a `fingerprint(&EventPayload::MethodElection(..)).is_none()` assertion (Global Constraints: `fingerprint = None`).
2. **Run → RED.**
3. **Minimal impl:**
   - `event.rs`: `use crate::LotMethod;` (re-exported at crate root, `lib.rs:16`); add `struct MethodElection` + `EventPayload::MethodElection(MethodElection)`.
   - `state.rs`: add `MethodElectionBackdated` and `LotSelectionInvalid` to `BlockerKind` (`:22-34`) and to the `Hard` arm of `severity()` (`:38-45`). Add a unit test asserting both are `Severity::Hard`.
   - `resolve.rs`: add `ElectionRec`; extend `Resolution`. After the `decisions` collection (`:311-318`) and `voided` (`:269-303`), build:
```rust
let mut elections: Vec<ElectionRec> = Vec::new();
for (seq, d) in &decisions {
    if voided.contains(&d.id) { continue; }
    if let EventPayload::MethodElection(me) = &d.payload {
        let made = tax_date(d.utc_timestamp, d.original_tz);
        if me.effective_from < TRANSITION_DATE || me.effective_from < made {
            blockers.push(Blocker { kind: BlockerKind::MethodElectionBackdated, event: Some(d.id.clone()),
                detail: "MethodElection effective_from precedes its made-date or TRANSITION_DATE (2025-01-01) — a standing order cannot be back-dated".into() });
            continue;
        }
        elections.push(ElectionRec { effective_from: me.effective_from, method: me.method, decision_seq: *seq });
    }
}
let selections: BTreeMap<EventId, Vec<crate::event::LotPick>> = BTreeMap::new(); // populated in Task 4
```
     Change the `universal_snapshot` call (`:520`) to pass `&elections, &selections`. Return `Resolution { timeline, transition, blockers, elections, selections }`.
   - `fold.rs`: add `FoldCtx`; change `fold_event` to take `ctx: &FoldCtx` (replace every `config` use with `ctx.config`; `consume_fee(..., ctx.config, ...)`). In `fold` (`:270-301`), build `let ctx = FoldCtx { config, elections: &res.elections, selections: &res.selections };` after the two sorts, and call `fold_event(eff, prices, &ctx, ...)`. Add:
```rust
fn applicable_method(date: TaxDate, ctx: &FoldCtx) -> LotMethod {
    if date < TRANSITION_DATE { ctx.config.pre2025_method } // Universal pool
    else {
        ctx.elections.iter().filter(|e| e.effective_from <= date)
            .max_by(|a, b| a.effective_from.cmp(&b.effective_from).then(a.decision_seq.cmp(&b.decision_seq)))
            .map(|e| e.method).unwrap_or(LotMethod::Fifo)   // FIFO before any election (regulatory default)
    }
}

/// Consume a method-honoring op's principal: applicable method + any LotSelection for `ev`.
/// On a selection-validation failure -> hard LotSelectionInvalid (carrying the disposal id + reason);
/// consumption falls back to method order so Σsat conservation holds and the hard blocker gates tax.
fn consume_principal(pools: &mut PoolSet, key: &PoolKey, need: Sat, date: TaxDate, ctx: &FoldCtx,
                     st: &mut LedgerState, ev: &EventId) -> (Vec<Consumed>, Sat) {
    let method = applicable_method(date, ctx);
    let selection = ctx.selections.get(ev).map(|v| v.as_slice());
    let r = pools.consume(key, need, method, selection);
    if let Some(reason) = r.selection_error {
        st.add_blocker(BlockerKind::LotSelectionInvalid, Some(ev.clone()), reason);
    }
    (r.consumed, r.shortfall)
}
```
     Replace the four honoring `consume_fifo` calls with `consume_principal`:
     - Dispose (`:367`): `let (consumed, shortfall) = consume_principal(pools, &key, *sat, date, ctx, st, &eff.id);`
     - SelfTransfer (`:526`): same.
     - GiftOut (`:745`): same.
     - Donate (`:811`): same.
     Leave PendingOut (`:483`) and `consume_fee` (`:232`) on `consume_fifo` (FIFO-pinned). Update `note_pre2025_once` signature + body:
```rust
fn note_pre2025_once(st: &mut LedgerState, date: TaxDate, ev: &EventId, method: LotMethod) {
    if date < TRANSITION_DATE && !st.blockers.iter().any(|b| b.kind == BlockerKind::Pre2025MethodNote) {
        let m = match method { LotMethod::Fifo => "FIFO", LotMethod::Lifo => "LIFO", LotMethod::Hifo => "HIFO" };
        st.add_blocker(BlockerKind::Pre2025MethodNote, Some(ev.clone()), format!(
            "pre-2025 lots reconstructed under {m} (the declared pre-2025 method; FIFO is the §7.4 legal default); \
             if your filed pre-2025 returns used a different lot method, your carryforward basis may differ — verify against those filings"));
    }
}
```
     Update its three call sites (Dispose `:366`, GiftOut `:744`, Donate `:810`) to pass `ctx.config.pre2025_method`.
   - `transition.rs`: `universal_snapshot` gains `elections: &[ElectionRec], selections: &BTreeMap<EventId, Vec<LotPick>>`; build `let ctx = FoldCtx { config, elections, selections };` and call `fold_event(eff, prices, &ctx, ...)`.
   - Existing `transition.rs` test (x) (`calendar_date_boundary…`, `:563-579`) asserts the note contains `"FIFO"` and `"verify"` — still true under default `pre2025_method = Fifo` (note now says "reconstructed under FIFO … verify against those filings"). No change needed.
4. **Run → GREEN.** Whole suite.
5. **Commit:** `feat(core): MethodElection standing order; method-aware fold (Universal=pre2025_method, Wallet=in-force election); Pre2025MethodNote names declared method`.

---

## TASK 4 — `LotSelection`/`LotPick` + selection wired through the fold

**Goal.** Add the per-disposal specific-ID decision; validate **principal conservation**, **targeting** (only Dispose/GiftOut/Donate/SelfTransfer), **duplicate** (→ `DecisionConflict`), **voided** (excluded), and **existence/per-wallet** (→ hard `LotSelectionInvalid`). On-chain `fee_sat` continues FIFO from the post-selection remainder.

**Files**
- modify `crates/btctax-core/src/event.rs` (`LotSelection` struct + variant; serde round-trip)
- modify `crates/btctax-core/src/project/resolve.rs` (collect+validate selections → `Resolution.selections`)
- modify `crates/btctax-core/src/project/fold.rs` (no new code — `consume_principal` from Task 3 already surfaces `LotSelectionInvalid`; selections now flow non-empty)
- create `crates/btctax-core/tests/lot_selection.rs`

**Interfaces**
```rust
// event.rs
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LotSelection { pub disposal_event: EventId, pub lots: Vec<LotPick> }
// + EventPayload::LotSelection(LotSelection)

// resolve.rs (private helper)
fn honoring_principal(op: &Op) -> Option<Sat>; // Dispose/GiftOut/Donate/SelfTransfer -> Some(principal sat); else None
```

**Steps**

1. **Failing tests** — `crates/btctax-core/tests/lot_selection.rs` (helpers as in Task 3; add `fn lotpick(rf,sat)`, `fn lot_selection(seq,ts,disposal_ref,picks)`):
```rust
fn pid(rf: &str) -> LotId { LotId { origin_event_id: EventId::import(Source::Coinbase, SourceRef::new(rf)), split_sequence: 0 } }
fn lot_selection(seq: u64, ts: time::OffsetDateTime, disposal_ref: &str, picks: Vec<LotPick>) -> LedgerEvent {
    dec_ev(seq, ts, EventPayload::LotSelection(LotSelection {
        disposal_event: EventId::import(Source::Coinbase, SourceRef::new(disposal_ref)), lots: picks }))
}

#[test]
fn selection_overrides_in_force_method() {
    let mut evs = three_post2025();                  // A/B/C, post-2025
    evs.push(election(1, datetime!(2025-01-02 00:00:00 UTC), date!(2025-01-02), LotMethod::Hifo)); // HIFO would pick B
    evs.push(sell("D", datetime!(2025-07-01 00:00:00 UTC), 100_000, dec!(95.00)));
    // explicit selection of the FIFO lot A overrides HIFO for this disposal.
    evs.push(lot_selection(2, datetime!(2025-07-01 00:00:00 UTC), "D", vec![LotPick { lot: pid("A"), sat: 100_000 }]));
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(!has(&st, BlockerKind::LotSelectionInvalid));
    assert_eq!(st.disposals[0].legs[0].basis, dec!(50.00)); // picked A, not HIFO's B
}

#[test]
fn selection_principal_conservation_violation_blocks() {
    let mut evs = three_post2025();
    evs.push(sell("D", datetime!(2025-07-01 00:00:00 UTC), 100_000, dec!(95.00)));
    evs.push(lot_selection(1, datetime!(2025-07-01 00:00:00 UTC), "D", vec![LotPick { lot: pid("A"), sat: 50_000 }])); // 50k != 100k
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(has(&st, BlockerKind::LotSelectionInvalid));
}

#[test]
fn selection_unknown_lot_blocks() {
    let mut evs = three_post2025();
    evs.push(sell("D", datetime!(2025-07-01 00:00:00 UTC), 100_000, dec!(95.00)));
    evs.push(lot_selection(1, datetime!(2025-07-01 00:00:00 UTC), "D", vec![LotPick { lot: pid("NOPE"), sat: 100_000 }]));
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(has(&st, BlockerKind::LotSelectionInvalid));
}

#[test]
fn selection_cross_wallet_blocks_post_2025() {
    // Two wallets; disposal in wallet cb; pick a lot held in self-custody -> §1.1012-1(j) cross-account ID forbidden.
    let cold = WalletId::SelfCustody { label: "cold".into() };
    let evs = vec![
        buy("CB", datetime!(2025-02-01 00:00:00 UTC), 100_000, dec!(50.00)),
        LedgerEvent { id: EventId::import(Source::Swan, SourceRef::new("COLD")), utc_timestamp: datetime!(2025-02-01 00:00:00 UTC),
            original_tz: offset!(+00:00), wallet: Some(cold.clone()),
            payload: EventPayload::Acquire(Acquire { sat: 100_000, usd_cost: dec!(40.00), fee_usd: dec!(0), basis_source: BasisSource::ExchangeProvided }) },
        sell("D", datetime!(2025-07-01 00:00:00 UTC), 100_000, dec!(95.00)),       // in cb()
        lot_selection(1, datetime!(2025-07-01 00:00:00 UTC), "D",
            vec![LotPick { lot: LotId { origin_event_id: EventId::import(Source::Swan, SourceRef::new("COLD")), split_sequence: 0 }, sat: 100_000 }]),
    ];
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(has(&st, BlockerKind::LotSelectionInvalid));
}

#[test]
fn duplicate_selection_for_one_disposal_is_decision_conflict() {
    let mut evs = three_post2025();
    evs.push(sell("D", datetime!(2025-07-01 00:00:00 UTC), 100_000, dec!(95.00)));
    evs.push(lot_selection(1, datetime!(2025-07-01 00:00:00 UTC), "D", vec![LotPick { lot: pid("A"), sat: 100_000 }]));
    evs.push(lot_selection(2, datetime!(2025-07-02 00:00:00 UTC), "D", vec![LotPick { lot: pid("C"), sat: 100_000 }]));
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(has(&st, BlockerKind::DecisionConflict));
}

#[test]
fn voided_selection_is_excluded() {
    let mut evs = three_post2025();
    evs.push(election(1, datetime!(2025-01-02 00:00:00 UTC), date!(2025-01-02), LotMethod::Hifo));
    evs.push(sell("D", datetime!(2025-07-01 00:00:00 UTC), 100_000, dec!(95.00)));
    evs.push(lot_selection(2, datetime!(2025-07-01 00:00:00 UTC), "D", vec![LotPick { lot: pid("A"), sat: 100_000 }]));
    evs.push(dec_ev(3, datetime!(2025-07-05 00:00:00 UTC), EventPayload::VoidDecisionEvent(VoidDecisionEvent { target_event_id: EventId::decision(2) })));
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert_eq!(st.disposals[0].legs[0].basis, dec!(90.00)); // voided selection -> HIFO -> B
}

#[test]
fn selection_targeting_pending_out_is_invalid() {
    // An unmatched TransferOut folds to PendingOut (non-honoring); a selection on it is rejected.
    let evs = vec![
        buy("A", datetime!(2025-02-01 00:00:00 UTC), 100_000, dec!(50.00)),
        LedgerEvent { id: EventId::import(Source::Coinbase, SourceRef::new("OUT")), utc_timestamp: datetime!(2025-06-01 00:00:00 UTC),
            original_tz: offset!(+00:00), wallet: Some(w()),
            payload: EventPayload::TransferOut(TransferOut { sat: 50_000, fee_sat: None, dest_addr: None, txid: None }) },
        lot_selection(1, datetime!(2025-06-01 00:00:00 UTC), "OUT", vec![LotPick { lot: pid("A"), sat: 50_000 }]),
    ];
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(has(&st, BlockerKind::LotSelectionInvalid));
}

#[test]
fn fee_bearing_reclassified_disposal_under_selection_consumes_fee_fifo_from_remainder() {
    // Reclassified TransferOut->Dispose with fee_sat; selection picks the principal lot; the on-chain fee
    // consumes FIFO from the post-selection remainder (A.4(a)). Conservation must balance.
    let evs = vec![
        buy("OLD", datetime!(2025-02-01 00:00:00 UTC), 60_000, dec!(30.00)), // FIFO remainder for the fee
        buy("NEW", datetime!(2025-03-01 00:00:00 UTC), 100_000, dec!(90.00)),
        LedgerEvent { id: EventId::import(Source::Coinbase, SourceRef::new("OUT")), utc_timestamp: datetime!(2025-07-01 00:00:00 UTC),
            original_tz: offset!(+00:00), wallet: Some(w()),
            payload: EventPayload::TransferOut(TransferOut { sat: 100_000, fee_sat: Some(500), dest_addr: None, txid: None }) },
        dec_ev(1, datetime!(2025-08-01 00:00:00 UTC), EventPayload::ReclassifyOutflow(ReclassifyOutflow {
            transfer_out_event: EventId::import(Source::Coinbase, SourceRef::new("OUT")),
            as_: OutflowClass::Dispose { kind: DisposeKind::Sell }, principal_proceeds_or_fmv: dec!(120.00), fee_usd: None })),
        // selection picks NEW for the 100k principal; the 500-sat fee then FIFO-consumes OLD.
        lot_selection(2, datetime!(2025-07-01 00:00:00 UTC), "OUT",
            vec![LotPick { lot: LotId { origin_event_id: EventId::import(Source::Coinbase, SourceRef::new("NEW")), split_sequence: 0 }, sat: 100_000 }]),
    ];
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(!has(&st, BlockerKind::LotSelectionInvalid));
    let report = btctax_core::conservation_report(&st);
    assert!(report.balanced, "{report:?}");
    let leg = &st.disposals[0].legs[0];
    assert_eq!(leg.basis, dec!(90.00)); // principal picked NEW (selection honored)
    assert_eq!(st.stats.fee_sats_consumed, 500); // fee taken from remainder (OLD), FIFO
}

#[test]
fn pre2025_selection_in_universal_pool() {
    let evs = vec![
        buy("A", datetime!(2024-02-01 00:00:00 UTC), 100_000, dec!(50.00)),
        buy("B", datetime!(2024-03-01 00:00:00 UTC), 100_000, dec!(90.00)),
        sell("D", datetime!(2024-09-01 00:00:00 UTC), 100_000, dec!(95.00)),
        lot_selection(1, datetime!(2024-09-01 00:00:00 UTC), "D", vec![LotPick { lot: pid("B"), sat: 100_000 }]),
    ];
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(!has(&st, BlockerKind::LotSelectionInvalid));
    assert_eq!(st.disposals[0].legs[0].basis, dec!(90.00)); // picked B from the Universal pool
}

#[test]
fn determinism_with_elections_and_selections_is_load_order_independent() {
    let mut a = three_post2025();
    a.push(election(1, datetime!(2025-01-02 00:00:00 UTC), date!(2025-01-02), LotMethod::Hifo));
    a.push(sell("D", datetime!(2025-07-01 00:00:00 UTC), 100_000, dec!(95.00)));
    a.push(lot_selection(2, datetime!(2025-07-01 00:00:00 UTC), "D", vec![LotPick { lot: pid("C"), sat: 100_000 }]));
    let mut b = a.clone(); b.reverse();
    let s1 = project(&a, &StaticPrices::default(), &ProjectionConfig::default());
    let s2 = project(&b, &StaticPrices::default(), &ProjectionConfig::default());
    assert_eq!(s1.disposals, s2.disposals);
    assert_eq!(s1.lots, s2.lots);
}
```
   Add a `LotSelection` case to `event.rs::every_variant_serde_round_trips` and a `fingerprint(&EventPayload::LotSelection(..)).is_none()` assertion.
2. **Run → RED.**
3. **Minimal impl:**
   - `event.rs`: add `struct LotSelection` + `EventPayload::LotSelection(LotSelection)`.
   - `resolve.rs`: add `honoring_principal`:
```rust
fn honoring_principal(op: &Op) -> Option<Sat> {
    match op {
        Op::Dispose { sat, .. } | Op::GiftOut { sat, .. } | Op::Donate { sat, .. } | Op::SelfTransfer { sat, .. } => Some(*sat),
        _ => None, // PendingOut, fee legs, non-disposals -> not selectable
    }
}
```
     After the timeline is built (`:478-504`), and reusing `decisions`/`voided`, populate `selections` (replace the empty map from Task 3):
```rust
let honoring: BTreeMap<EventId, Sat> = timeline.iter()
    .filter_map(|e| honoring_principal(&e.op).map(|s| (e.id.clone(), s))).collect();

let mut selections: BTreeMap<EventId, Vec<crate::event::LotPick>> = BTreeMap::new();
let mut seen: BTreeSet<EventId> = BTreeSet::new();   // disposal_events already claimed (dup detection)
let mut dup: BTreeSet<EventId> = BTreeSet::new();
for (_seq, d) in &decisions {
    if voided.contains(&d.id) { continue; }
    let EventPayload::LotSelection(ls) = &d.payload else { continue; };
    if !seen.insert(ls.disposal_event.clone()) {
        // mirrors the duplicate-ReclassifyOutflow pattern (resolve.rs:459-468)
        blockers.push(Blocker { kind: BlockerKind::DecisionConflict, event: Some(d.id.clone()),
            detail: "duplicate LotSelection for the same disposal_event".into() });
        dup.insert(ls.disposal_event.clone());
        continue;
    }
    selections.insert(ls.disposal_event.clone(), ls.lots.clone());
}
for id in &dup { selections.remove(id); }            // a conflicted disposal applies NEITHER selection
// targeting + principal-conservation (A.4(a)); existence/per-wallet checked in the fold.
selections.retain(|disposal, picks| match honoring.get(disposal) {
    None => { blockers.push(Blocker { kind: BlockerKind::LotSelectionInvalid, event: Some(disposal.clone()),
        detail: "LotSelection targets a non-honoring or unknown event (only Dispose/GiftOut/Donate/SelfTransfer — not PendingOut/fee legs)".into() }); false }
    Some(&principal) => {
        let picked: Sat = picks.iter().map(|p| p.sat).sum();
        if picked != principal {
            blockers.push(Blocker { kind: BlockerKind::LotSelectionInvalid, event: Some(disposal.clone()),
                detail: format!("LotSelection must conserve principal: picked {picked} sat != disposal principal {principal} sat (on-chain fee_sat is excluded and consumes FIFO from the remainder)") });
            false
        } else { true }
    }
});
```
     Return this `selections` in the `Resolution` (the fold from Task 3 already consumes `res.selections` via `consume_principal`).
   - `fold.rs`: **no new code** — `consume_principal` (Task 3) already looks up `ctx.selections.get(ev)` and raises `LotSelectionInvalid` on existence/per-wallet failure. The fee leg already runs FIFO via `consume_fee` on the post-selection pool.
4. **Run → GREEN.** Whole suite.
5. **Commit:** `feat(core): LotSelection specific-ID — principal conservation, targeting, dup/void, existence & per-wallet validation`.

---

## TASK 5 — CLI `select-lots` + `import-selections` + `parse_lot_id`

**Goal.** User-facing emission of `LotSelection` decisions: a per-disposal `select-lots` and a batch `import-selections <csv>`. Parse `LotId` (and `LotPick`) from the canonical surfaced form, round-tripping **all three** `EventId` origin variants.

**Files**
- modify `crates/btctax-cli/src/eventref.rs` (`parse_lot_id`, `parse_lot_pick`)
- modify `crates/btctax-cli/src/cmd/reconcile.rs` (`select_lots`, `import_selections`)
- modify `crates/btctax-cli/src/main.rs` (`Reconcile::SelectLots`, `Reconcile::ImportSelections` + dispatch)
- modify `crates/btctax-cli/tests/reconcile.rs` (add cases) + `eventref.rs` unit tests

**Interfaces**
```rust
// eventref.rs
pub fn parse_lot_id(s: &str) -> Result<LotId, CliError>;     // "<event-id-canonical>#<split>"
pub fn parse_lot_pick(s: &str) -> Result<LotPick, CliError>; // "<lot-id>:<sat>"
// reconcile.rs
pub fn select_lots(vault_path: &Path, pp: &Passphrase, disposal_ref: &str, picks: Vec<LotPick>, now: OffsetDateTime) -> Result<EventId, CliError>;
pub fn import_selections(vault_path: &Path, pp: &Passphrase, csv_path: &Path, now: OffsetDateTime) -> Result<Vec<EventId>, CliError>;
pub fn set_forward_method(vault_path: &Path, pp: &Passphrase, m: LotMethod, effective_from: Option<TaxDate>, now: OffsetDateTime) -> Result<EventId, CliError>; // M3: SPEC A.1 `config --set-forward-method` — APPENDS a MethodElection decision (does not mutate a flag)
```

**Steps**

1. **Failing tests** — `eventref.rs` unit tests (mirroring `parse_event_id` tests at `eventref.rs:101-129`):
```rust
#[test]
fn parse_lot_id_round_trips_all_three_origin_variants() {
    use btctax_core::{EventId, Fingerprint, LotId, Source, SourceRef};
    // Import origin (with a '|' in source_ref)
    let l_imp = LotId { origin_event_id: EventId::import(Source::Coinbase, SourceRef::new("out|cb-send")), split_sequence: 3 };
    let s = format!("{}#{}", l_imp.origin_event_id.canonical(), l_imp.split_sequence);
    assert_eq!(parse_lot_id(&s).unwrap(), l_imp);
    // Decision origin (Path-B seed lots: origin = allocation Decision id, resolve.rs:570-574)
    let l_dec = LotId { origin_event_id: EventId::decision(7), split_sequence: 1 };
    assert_eq!(parse_lot_id(&format!("{}#1", EventId::decision(7).canonical())).unwrap(), l_dec);
    // Conflict origin
    let fp = Fingerprint::of_bytes(b"x");
    let cid = EventId::conflict(Source::Gemini, SourceRef::new("in|99|credit"), &fp);
    let l_con = LotId { origin_event_id: cid.clone(), split_sequence: 0 };
    assert_eq!(parse_lot_id(&format!("{}#0", cid.canonical())).unwrap(), l_con);
    // N1: a source_ref containing '#' must still round-trip — rsplit_once('#') splits on the LAST '#'
    // (the split-sequence suffix is always last); locks the rsplit choice (cf. eventref.rs:115 '#'-in-source_ref).
    let l_hash = LotId { origin_event_id: EventId::import(Source::Coinbase, SourceRef::new("in|99|credit|1#0")), split_sequence: 2 };
    assert_eq!(parse_lot_id(&format!("{}#{}", l_hash.origin_event_id.canonical(), l_hash.split_sequence)).unwrap(), l_hash);
}
#[test]
fn parse_lot_pick_splits_trailing_sat() {
    use btctax_core::{EventId, LotId, Source, SourceRef};
    let pick = parse_lot_pick("import|coinbase|X#0:25000").unwrap();
    assert_eq!(pick.lot, LotId { origin_event_id: EventId::import(Source::Coinbase, SourceRef::new("X")), split_sequence: 0 });
    assert_eq!(pick.sat, 25_000);
}
```
   `crates/btctax-cli/tests/reconcile.rs` (synthetic-only; reuse `vault_with_pending`/fixtures):
```rust
#[test]
fn select_lots_emits_a_lot_selection_decision() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    // A post-2025 buy + sell (full 100k) so the disposal exists, then select the lot explicitly.
    // (Build via fixtures or direct append_import_batch of synthetic events.)
    // ... import a synthetic buy+sell fixture ...
    let (disposal_ref, lot_ref) = { /* project; read the disposal event canonical + its lot's "<origin>#<split>" */ };
    let picks = vec![btctax_cli::eventref::parse_lot_pick(&format!("{lot_ref}:100000")).unwrap()];
    let id = cmd::reconcile::select_lots(&vault, &pp(), &disposal_ref, picks, now()).unwrap();
    assert!(matches!(id, EventId::Decision { .. }));
    let s = Session::open(&vault, &pp()).unwrap();
    let events = btctax_core::persistence::load_all(s.conn()).unwrap();
    assert!(events.iter().any(|e| matches!(e.payload, EventPayload::LotSelection(_))));
}

#[test]
fn import_selections_rejects_a_bad_header() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    let csv = dir.path().join("sel.csv");
    std::fs::write(&csv, "wrong,header,here,now\nimport|coinbase|D,import|coinbase|A,0,100000\n").unwrap();
    let err = cmd::reconcile::import_selections(&vault, &pp(), &csv, now()).unwrap_err();
    assert!(matches!(err, CliError::Usage(_)));
}

#[test]
fn import_selections_groups_rows_into_one_selection_per_disposal() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    let csv = dir.path().join("sel.csv");
    std::fs::write(&csv, "disposal_ref,origin_event_id,split_sequence,sat\n\
import|coinbase|D,import|coinbase|A,0,60000\n\
import|coinbase|D,import|coinbase|B,0,40000\n").unwrap();
    let ids = cmd::reconcile::import_selections(&vault, &pp(), &csv, now()).unwrap();
    assert_eq!(ids.len(), 1); // two rows, one disposal -> one LotSelection (2 picks)
    let s = Session::open(&vault, &pp()).unwrap();
    let events = btctax_core::persistence::load_all(s.conn()).unwrap();
    let ls = events.iter().find_map(|e| match &e.payload { EventPayload::LotSelection(l) => Some(l.clone()), _ => None }).unwrap();
    assert_eq!(ls.lots.len(), 2);
}

// M3: `config --set-forward-method` APPENDS a MethodElection decision (SPEC A.1) — not a flag mutation.
#[test]
fn set_forward_method_appends_a_method_election_decision() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    let id = cmd::reconcile::set_forward_method(&vault, &pp(), btctax_core::LotMethod::Hifo, Some(date!(2025-06-01)), now()).unwrap();
    assert!(matches!(id, EventId::Decision { .. }));
    let s = Session::open(&vault, &pp()).unwrap();
    let events = btctax_core::persistence::load_all(s.conn()).unwrap();
    let me = events.iter().find_map(|e| match &e.payload { EventPayload::MethodElection(m) => Some(m.clone()), _ => None }).unwrap();
    assert_eq!(me.method, btctax_core::LotMethod::Hifo);
    assert_eq!(me.effective_from, date!(2025-06-01));
}
#[test]
fn set_forward_method_defaults_effective_from_to_made_date() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    cmd::reconcile::set_forward_method(&vault, &pp(), btctax_core::LotMethod::Lifo, None, now()).unwrap(); // now() = 2026-02-01
    let s = Session::open(&vault, &pp()).unwrap();
    let events = btctax_core::persistence::load_all(s.conn()).unwrap();
    let me = events.iter().find_map(|e| match &e.payload { EventPayload::MethodElection(m) => Some(m.clone()), _ => None }).unwrap();
    assert_eq!(me.effective_from, date!(2026-02-01)); // defaulted to the decision's made-date
}
```
2. **Run → RED.**
3. **Minimal impl:**
   - `eventref.rs` (imports already include `LotId`? add `use btctax_core::{LotId, LotPick};`):
```rust
pub fn parse_lot_id(s: &str) -> Result<LotId, CliError> {
    let (origin, split) = s.rsplit_once('#')
        .ok_or_else(|| CliError::Usage(format!("bad lot id {s:?}; expected <event-id>#<split>")))?;
    let origin_event_id = parse_event_id(origin)?;
    let split_sequence = split.trim().parse::<u32>()
        .map_err(|_| CliError::Usage(format!("bad split_sequence in lot id {s:?}")))?;
    Ok(LotId { origin_event_id, split_sequence })
}
pub fn parse_lot_pick(s: &str) -> Result<LotPick, CliError> {
    let (lot_str, sat_str) = s.rsplit_once(':')
        .ok_or_else(|| CliError::Usage(format!("bad --from {s:?}; expected <lot-id>:<sat>")))?;
    let lot = parse_lot_id(lot_str)?;
    let sat = sat_str.trim().parse::<i64>().map_err(|e| CliError::Usage(format!("bad sat in --from {s:?}: {e}")))?;
    Ok(LotPick { lot, sat })
}
```
     (`#` and the trailing `:` are unambiguous: an `EventId::canonical` uses `|` separators only; the fingerprint segment of a Conflict id is hex, never contains `#` or a trailing `:<digits>`.)
   - `reconcile.rs` (add `use btctax_core::{LotPick, LotSelection}` to the `use` block at `:9-14`):
```rust
pub fn select_lots(vault_path: &Path, pp: &Passphrase, disposal_ref: &str, picks: Vec<LotPick>, now: OffsetDateTime) -> Result<EventId, CliError> {
    let disposal_event = parse_event_id(disposal_ref)?;
    if picks.is_empty() { return Err(CliError::Usage("select-lots needs at least one --from <lotid>:<sat>".into())); }
    let mut session = Session::open(vault_path, pp)?;
    append_and_save(&mut session, EventPayload::LotSelection(LotSelection { disposal_event, lots: picks }), now)
}

pub fn import_selections(vault_path: &Path, pp: &Passphrase, csv_path: &Path, now: OffsetDateTime) -> Result<Vec<EventId>, CliError> {
    let mut rdr = csv::ReaderBuilder::new().has_headers(true).from_path(csv_path)?;
    {
        let hdr = rdr.headers()?;
        let cols: Vec<&str> = hdr.iter().collect();
        if cols != ["disposal_ref", "origin_event_id", "split_sequence", "sat"] {
            return Err(CliError::Usage(format!(
                "import-selections header must be disposal_ref,origin_event_id,split_sequence,sat; got {cols:?}")));
        }
    }
    // group picks by disposal_ref, PRESERVING first-seen order (BTreeMap on the canonical string -> deterministic)
    let mut by_disposal: std::collections::BTreeMap<String, Vec<LotPick>> = std::collections::BTreeMap::new();
    for rec in rdr.records() {
        let rec = rec?;
        let disposal_ref = rec.get(0).ok_or_else(|| CliError::Usage("missing disposal_ref".into()))?.to_string();
        let origin = rec.get(1).ok_or_else(|| CliError::Usage("missing origin_event_id".into()))?;
        let split = rec.get(2).ok_or_else(|| CliError::Usage("missing split_sequence".into()))?
            .trim().parse::<u32>().map_err(|e| CliError::Usage(format!("bad split_sequence: {e}")))?;
        let sat = rec.get(3).ok_or_else(|| CliError::Usage("missing sat".into()))?
            .trim().parse::<i64>().map_err(|e| CliError::Usage(format!("bad sat: {e}")))?;
        let origin_event_id = parse_event_id(origin)?;
        by_disposal.entry(disposal_ref).or_default().push(LotPick { lot: LotId { origin_event_id, split_sequence: split }, sat });
    }
    let mut session = Session::open(vault_path, pp)?;
    let mut ids = Vec::new();
    for (disposal_ref, lots) in by_disposal {
        let disposal_event = parse_event_id(&disposal_ref)?;
        let id = append_decision(session.conn(), EventPayload::LotSelection(LotSelection { disposal_event, lots }),
                                 now, UtcOffset::UTC, None)?;
        ids.push(id);
    }
    session.save()?;
    Ok(ids)
}

pub fn set_forward_method(vault_path: &Path, pp: &Passphrase, m: LotMethod, effective_from: Option<TaxDate>, now: OffsetDateTime) -> Result<EventId, CliError> {
    // M3 / SPEC A.1: the forward standing order is an event, not a flag — APPEND a MethodElection.
    // Default effective_from to the decision's made-date (now, in UTC), so the resolve-pass "cannot
    // back-date" rule (effective_from >= made-date) holds by construction.
    let effective_from = effective_from.unwrap_or_else(|| now.to_offset(time::UtcOffset::UTC).date());
    let mut session = Session::open(vault_path, pp)?;
    append_and_save(&mut session, EventPayload::MethodElection(MethodElection { effective_from, method: m }), now)
}
```
     (add `use btctax_core::{LotId, LotMethod, MethodElection};` and `use btctax_core::conventions::TaxDate;` (or `time::Date`) to the `use` block; `use std::path::Path;` is already present; `parse_event_id` is already imported at `:19`.)
   - `main.rs`: add to `enum Reconcile`:
```rust
/// Specific-ID: pick the exact lots a disposal consumes.
SelectLots { disposal: String, #[arg(long = "from", required = true)] from: Vec<String> },
/// Batch import LotSelections from a CSV (disposal_ref,origin_event_id,split_sequence,sat).
ImportSelections { csv: PathBuf },
```
     and dispatch arms:
```rust
Reconcile::SelectLots { disposal, from } => {
    let picks = from.iter().map(|s| eventref::parse_lot_pick(s)).collect::<Result<Vec<_>, _>>()?;
    cmd::reconcile::select_lots(vault, &pp, &disposal, picks, now)?
}
Reconcile::ImportSelections { csv } => {
    let ids = cmd::reconcile::import_selections(vault, &pp, &csv, now)?;
    // import-selections returns many ids; print a summary and return early (the trailing single-id print does not apply).
    println!("Recorded {} LotSelection decision(s)", ids.len());
    return Ok(());
}
```
     (`SelectLots` falls through to the existing `println!("Recorded decision {}", id.canonical())` at `main.rs:345`.)
   - `main.rs` (M3 — extends the `Command::Config` defined in Task 1, since `--set-forward-method` is a `config` subcommand per SPEC A.1): add `set_forward_method: Option<MethodLotArg>` and `effective_from: Option<String>` to `Command::Config` (alongside `set_fee_treatment`/`set_pre2025_method`). In the `Config` arm, **before** the config-mutating branches, handle the decision-append branch (it needs the global `now`):
```rust
if let Some(m) = set_forward_method {
    let eff = effective_from.as_deref().map(eventref::parse_date_arg).transpose()?;
    let id = cmd::reconcile::set_forward_method(vault, &pp, m.into(), eff, now)?;
    println!("Recorded standing order (MethodElection) {}", id.canonical());
    return Ok(());
}
```
     (`MethodLotArg` — the `Fifo|Lifo|Hifo` clap enum from Task 1 — gains `impl From<MethodLotArg> for LotMethod`; `parse_date_arg` is the existing `eventref` date parser, `eventref.rs:82-92`.)
4. **Run → GREEN.** Whole suite (privacy: all CSV/vaults synthetic + temp).
5. **Commit:** `feat(cli): reconcile select-lots + import-selections; config --set-forward-method (appends MethodElection); parse_lot_id/parse_lot_pick over all EventId origins`.

---

## TASK 6 — A.7 `pre2025_method` ↔ effective `SafeHarborAllocation`

**Goal.** Bind the lot-consumption method to each allocation: add an immutable serde-default `pre2025_method` field to `SafeHarborAllocation`; make `universal_snapshot` **method-aware** (conserve under the allocation's recorded method); fire a dedicated hard `Pre2025MethodConflictsAllocation` when the live config differs from the recorded method (never the generic `SafeHarborUnconservable`); capture `pre2025_method` at attestation in the CLI. Composition + conflict KATs.

**Files**
- modify `crates/btctax-core/src/event.rs` (`SafeHarborAllocation.pre2025_method` `#[serde(default)]`)
- modify `crates/btctax-core/src/state.rs` (`BlockerKind::Pre2025MethodConflictsAllocation` + `Hard`)
- modify `crates/btctax-core/src/project/transition.rs` (`universal_snapshot` gains `method: LotMethod`)
- modify `crates/btctax-core/src/project/resolve.rs` (per-allocation method-aware snapshot; conflict blocker)
- modify `crates/btctax-cli/src/cmd/reconcile.rs` (`safe_harbor_allocate`/`safe_harbor_attest` set `pre2025_method`)
- modify `crates/btctax-core/tests/transition.rs` (`alloc` helper gains the field) + create `crates/btctax-core/tests/safe_harbor_method.rs`

**Interfaces**
```rust
// event.rs
pub struct SafeHarborAllocation {
    pub lots: Vec<AllocLot>, pub as_of_date: TaxDate, pub method: AllocMethod, pub timely_allocation_attested: bool,
    #[serde(default)] pub pre2025_method: LotMethod,   // immutable; captured at attestation; default Fifo
}
// transition.rs
pub fn universal_snapshot(timeline: &[Eff], prices: &dyn PriceProvider, config: &ProjectionConfig,
                          method: LotMethod, elections: &[ElectionRec], selections: &BTreeMap<EventId, Vec<LotPick>>) -> UniversalSnapshot;
```

**Steps**

1. **Failing tests** — `crates/btctax-core/tests/safe_harbor_method.rs` (helpers as in transition.rs; `alloc` gains `pre2025_method` arg):
```rust
// Composition KAT: a non-FIFO (LIFO) pre-2025 residue -> Path B conserves under the recorded method.
// Pre-2025 buys A($30/50k, older) + B($50/50k, newer); a pre-2025 sell of 50k.
//   LIFO consumes B -> residue = A ($30/50k).  Allocation records pre2025_method=Lifo + that residue.
#[test]
fn lifo_residue_path_b_conserves_method_aware() {
    let cfg = ProjectionConfig { pre2025_method: LotMethod::Lifo, ..ProjectionConfig::default() };
    let evs = vec![
        buy("A", datetime!(2024-02-01 00:00:00 UTC), 50_000, dec!(30.00)),
        buy("B", datetime!(2024-03-01 00:00:00 UTC), 50_000, dec!(50.00)),
        sell("S", datetime!(2024-09-01 00:00:00 UTC), 50_000, dec!(45.00)),  // LIFO consumes B
        alloc(1, datetime!(2024-12-01 00:00:00 UTC), AllocMethod::ActualPosition, true /*attested*/, LotMethod::Lifo,
              vec![alloc_lot(cb(), 50_000, dec!(30.00), date!(2024-02-01))]), // residue A under LIFO
        sell("S2", datetime!(2025-09-01 00:00:00 UTC), 1, dec!(0.01)),        // a 2025 seed trigger
    ];
    let st = project(&evs, &StaticPrices::default(), &cfg);
    assert!(!has(&st, BlockerKind::SafeHarborUnconservable));
    assert!(!has(&st, BlockerKind::Pre2025MethodConflictsAllocation));
    assert!(st.lots.iter().any(|l| l.basis_source == BasisSource::SafeHarborAllocated)); // Path B effective
}

// Conflict KAT: live config (Fifo) != the effective allocation's recorded method (Lifo) -> dedicated hard blocker,
// NOT SafeHarborUnconservable; Path B still governs (the irrevocable allocation pins the method).
#[test]
fn live_config_differs_from_recorded_method_is_pre2025_conflict() {
    let cfg = ProjectionConfig { pre2025_method: LotMethod::Fifo, ..ProjectionConfig::default() }; // != recorded Lifo
    let evs = vec![
        buy("A", datetime!(2024-02-01 00:00:00 UTC), 50_000, dec!(30.00)),
        buy("B", datetime!(2024-03-01 00:00:00 UTC), 50_000, dec!(50.00)),
        sell("S", datetime!(2024-09-01 00:00:00 UTC), 50_000, dec!(45.00)),
        alloc(1, datetime!(2024-12-01 00:00:00 UTC), AllocMethod::ActualPosition, true, LotMethod::Lifo,
              vec![alloc_lot(cb(), 50_000, dec!(30.00), date!(2024-02-01))]), // residue A under LIFO
        sell("S2", datetime!(2025-09-01 00:00:00 UTC), 1, dec!(0.01)),
    ];
    let st = project(&evs, &StaticPrices::default(), &cfg);
    assert!(has(&st, BlockerKind::Pre2025MethodConflictsAllocation));
    assert!(!has(&st, BlockerKind::SafeHarborUnconservable)); // method change is NOT misread as bad data
    assert!(st.lots.iter().any(|l| l.basis_source == BasisSource::SafeHarborAllocated)); // Path B still governs
}

// Backward-compat: a SafeHarborAllocation JSON without pre2025_method deserializes to Fifo.
#[test]
fn safe_harbor_allocation_pre2025_method_serde_default_fifo() {
    let a = SafeHarborAllocation { lots: vec![], as_of_date: date!(2025-01-01),
        method: AllocMethod::ActualPosition, timely_allocation_attested: false, pre2025_method: LotMethod::Hifo };
    let mut v: serde_json::Value = serde_json::from_str(&serde_json::to_string(&a).unwrap()).unwrap();
    v.as_object_mut().unwrap().remove("pre2025_method");
    let old: SafeHarborAllocation = serde_json::from_value(v).unwrap();
    assert_eq!(old.pre2025_method, LotMethod::Fifo);
}

// ── C1 divergence KAT (b) — Path-B seeding in NON-acquired_at order; post-seed FIFO consumes oldest-first ──
// The allocation lists two SAME-WALLET seed lots newer-first; seed is pushed in alloc-index order, so the
// wallet pool's insertion order is [newer (split 0), older (split 1)]. A post-2025 partial FIFO Dispose MUST
// consume the OLDER lot first (acquisition-date FIFO), NOT seed-index 0 (which the legacy front-walk took).
#[test]
fn path_b_seed_in_non_acq_order_consumes_oldest_first_under_fifo() {
    let cfg = ProjectionConfig { pre2025_method: LotMethod::Fifo, ..ProjectionConfig::default() };
    let evs = vec![
        // Pre-2025 Universal residue = 200k sat / $100 basis (FIFO, no pre-2025 disposal).
        buy("U1", datetime!(2024-01-01 00:00:00 UTC), 100_000, dec!(40.00)),
        buy("U2", datetime!(2024-06-01 00:00:00 UTC), 100_000, dec!(60.00)),
        // Allocation lists NEWER first (idx 0) and OLDER second (idx 1) — non-acquired_at order. Totals conserve.
        alloc(1, datetime!(2024-12-01 00:00:00 UTC), AllocMethod::ActualPosition, true, LotMethod::Fifo, vec![
            alloc_lot(cb(), 100_000, dec!(60.00), date!(2024-06-01)),  // seed split_sequence 0 (NEWER)
            alloc_lot(cb(), 100_000, dec!(40.00), date!(2024-01-01)),  // seed split_sequence 1 (OLDER)
        ]),
        sell("D", datetime!(2025-09-01 00:00:00 UTC), 100_000, dec!(120.00)), // post-2025 partial FIFO Dispose in cb()
    ];
    let st = project(&evs, &StaticPrices::default(), &cfg);
    assert!(!has(&st, BlockerKind::SafeHarborUnconservable));
    assert!(st.lots.iter().any(|l| l.basis_source == BasisSource::SafeHarborAllocated)); // Path B effective
    let leg = &st.disposals.iter().find(|d| d.event == EventId::import(Source::Coinbase, SourceRef::new("D"))).unwrap().legs[0];
    assert_eq!(leg.basis, dec!(40.00), "acq-date FIFO consumes the OLDER seed lot; insertion-order would pick the newer index-0 lot");
    assert_eq!(leg.lot_id.split_sequence, 1); // the OLDER lot was the one listed SECOND
}

// ── C1 divergence KAT (c) — pre-2025 SelfTransfer reorders the Universal pool; snapshot residue under acq-date ──
// A pre-2025 SelfTransfer relocates the OLDER lot to the BACK of the single Universal pool (insertion != acquisition
// order). A pre-2025 partial disposal then consumes a DIFFERENT lot under acquisition-date FIFO (the older B1')
// than the legacy front-walk would (B2), so the conservation residue snap.basis differs ($60 vs the legacy $40).
// An allocation built against the ACQUISITION-DATE-order residue ($60) must conserve (no spurious
// SafeHarborUnconservable) and Path B governs.
#[test]
fn pre2025_self_transfer_reorders_universal_snapshot_residue_under_acq_date_fifo() {
    let cfg = ProjectionConfig { pre2025_method: LotMethod::Fifo, ..ProjectionConfig::default() };
    let cold = WalletId::SelfCustody { label: "cold".into() };
    let evs = vec![
        buy("B1", datetime!(2024-01-01 00:00:00 UTC), 100_000, dec!(40.00)),  // OLDER, $40 (cb())
        buy("B2", datetime!(2024-06-01 00:00:00 UTC), 100_000, dec!(60.00)),  // NEWER, $60 (cb())
        // pre-2025 SelfTransfer: consume B1 (oldest) from Universal, re-push B1' to the BACK (still Universal, pre-2025).
        LedgerEvent { id: EventId::import(Source::Swan, SourceRef::new("OUT")),
            utc_timestamp: datetime!(2024-09-01 00:00:00 UTC), original_tz: offset!(+00:00), wallet: Some(cb()),
            payload: EventPayload::TransferOut(TransferOut { sat: 100_000, fee_sat: None, dest_addr: None, txid: None }) },
        dec_ev(1, datetime!(2024-09-02 00:00:00 UTC), EventPayload::TransferLink(TransferLink {
            out_event: EventId::import(Source::Swan, SourceRef::new("OUT")),
            in_event_or_wallet: TransferTarget::Wallet(cold.clone()) })),       // pre-2025 dest -> still Universal pool
        // pre-2025 partial disposal: acq-date FIFO consumes the OLDER B1' (basis $40) -> residue = B2 ($60).
        sell("D", datetime!(2024-10-01 00:00:00 UTC), 100_000, dec!(70.00)),
        // Allocation built against the acquisition-date residue ($60); recorded method matches live Fifo.
        alloc(2, datetime!(2024-12-01 00:00:00 UTC), AllocMethod::ActualPosition, true, LotMethod::Fifo,
              vec![alloc_lot(cb(), 100_000, dec!(60.00), date!(2024-06-01))]),
        sell("S2", datetime!(2025-09-01 00:00:00 UTC), 1, dec!(0.01)),          // post-2025 seed trigger
    ];
    let st = project(&evs, &StaticPrices::default(), &cfg);
    assert!(!has(&st, BlockerKind::SafeHarborUnconservable),
        "snapshot residue computed under acquisition-date FIFO is $60; the allocation conserves");
    assert!(st.lots.iter().any(|l| l.basis_source == BasisSource::SafeHarborAllocated)); // Path B effective
    let d = st.disposals.iter().find(|d| d.event == EventId::import(Source::Coinbase, SourceRef::new("D"))).unwrap();
    assert_eq!(d.legs[0].basis, dec!(40.00)); // the pre-2025 disposal consumed the OLDER relocated lot, not the front-of-Vec B2
}
```
   Add a severity unit test (`state.rs`) for `Pre2025MethodConflictsAllocation == Hard`. Update `event.rs::every_variant_serde_round_trips` SafeHarborAllocation literal (`:327-357`) to include `pre2025_method: LotMethod::Fifo`.
2. **Run → RED.**
3. **Minimal impl:**
   - `event.rs`: add `#[serde(default)] pub pre2025_method: LotMethod` to `SafeHarborAllocation` (`LotMethod: Default` from Task 1). Update the test literal.
   - `state.rs`: add `Pre2025MethodConflictsAllocation` to `BlockerKind` + the `Hard` arm.
   - `transition.rs`: `universal_snapshot` gains a `method: LotMethod` param; internally fold under it:
```rust
pub fn universal_snapshot(timeline: &[Eff], prices: &dyn PriceProvider, config: &ProjectionConfig,
                          method: LotMethod, elections: &[ElectionRec], selections: &BTreeMap<EventId, Vec<LotPick>>) -> UniversalSnapshot {
    let cfg = ProjectionConfig { pre2025_method: method, ..*config };  // method-aware residue
    let mut pre: Vec<Eff> = timeline.iter().filter(|e| e.date() < TRANSITION_DATE).cloned().collect();
    sort_canonical(&mut pre);
    let ctx = FoldCtx { config: &cfg, elections, selections };
    let mut pools = PoolSet::default();
    let mut sink = LedgerState::default();
    let mut stats = FoldStats::default();
    for eff in &pre { fold_event(eff, prices, &ctx, &mut pools, &mut sink, &mut stats); }
    let lots = pools.pools.get(&PoolKey::Universal).map(Vec::as_slice).unwrap_or(&[]);
    UniversalSnapshot { held_sat: lots.iter().map(|l| l.remaining_sat).sum(), basis: lots.iter().map(|l| l.usd_basis).sum() }
}
```
   - `resolve.rs`: delete the single allocation-independent snapshot at `:519-520`. The `effective` accumulator now also carries each candidate's **recorded method**, so the conflict can be emitted **after Path selection** (M2): change `let mut effective: Vec<(EventId, Vec<Lot>)>` (`:523`) to `Vec<(EventId, Vec<Lot>, LotMethod)>`. Inside the effectiveness loop (`:524-587`), compute the snapshot per candidate under its **own** recorded method, then push the method alongside the seed — **no conflict push here**:
```rust
let snap = crate::project::transition::universal_snapshot(&timeline, prices, config, a.pre2025_method, &elections, &selections);
// ... existing bar/conservation logic uses `snap` exactly as before (resolve.rs:545-563) ...
// ... existing seed build (resolve.rs:566-586) ...
effective.push((d.id.clone(), seed, a.pre2025_method));
```
     Then emit `Pre2025MethodConflictsAllocation` **only after Path selection**, inside the existing `match effective.len()` arm (`:602-615`) — so the multiple-effective case (which already hard-blocks with `DecisionConflict` → Path A) can **never** fire a spurious method-conflict (M2):
```rust
let transition = match effective.len() {
    0 => TransitionMode::PathA,
    1 => {
        let (id, seed, recorded_method) = effective.into_iter().next().expect("len == 1");
        // A.7.3: conflict is "live config != the GOVERNING allocation's recorded method", emitted ONCE,
        // for the single effective allocation only. Conservation already passed (snapshot used `recorded_method`),
        // so this is NEVER SafeHarborUnconservable; Path B stays effective (the irrevocable allocation pins it).
        if config.pre2025_method != recorded_method {
            blockers.push(Blocker { kind: BlockerKind::Pre2025MethodConflictsAllocation, event: Some(id.clone()),
                detail: format!("live pre2025_method ({:?}) differs from this allocation's recorded method ({:?}); revert the config to the recorded method (the irrevocable allocation pins it, §7.4)",
                                config.pre2025_method, recorded_method) });
        }
        TransitionMode::PathB { seed }
    }
    _ => { /* existing verbatim: push DecisionConflict "multiple effective SafeHarborAllocations"; -> Path A */ TransitionMode::PathA }
};
```
     (≤1 allocation is ever effective, so this is one snapshot + at most one method-conflict in any clean state. The `_ =>` arm keeps the existing multiple-effective `DecisionConflict` push verbatim — the method-conflict is **not** evaluated there.)
   - `reconcile.rs`: in `safe_harbor_allocate` (`:248-253`) set `pre2025_method: session.config()?.pre2025_method` (capture the live attested method at attestation time). In `safe_harbor_attest` the re-appended copy uses `..prior` (`:347-350`), which already carries `pre2025_method` forward — no change beyond it compiling.
   - `transition.rs` test `alloc` helper (`:83-100`) gains a `pre2025_method: LotMethod` parameter; update all `alloc(...)` call sites in `tests/transition.rs` to pass `LotMethod::Fifo` (their residues are FIFO).
4. **Run → GREEN.** Whole suite.
5. **Commit:** `feat(core): bind pre2025_method to SafeHarborAllocation; method-aware universal_snapshot; Pre2025MethodConflictsAllocation`.

---

## TASK 7 — `DisposalCompliance` projection

**Goal.** Compute per-disposal compliance status (A.5) for **post-2025** realized disposals/removals using the `WalletId`→custody mapping and the 2025-2026 vs 2027+ envelope. Reusable by `verify` (Task 8) and by C.

**Scope decision (resolved ambiguity, see §Self-review):** the four-state model and the envelope are defined for the **post-2025 identification regime**; this projection emits an entry per realized disposal/removal whose tax date is `>= TRANSITION_DATE`. Pre-2025 disposals are governed by the attested `pre2025_method` (surfaced separately via the declared-method line + `Pre2025MethodNote`). The classifier produces `StandingOrder` / `Contemporaneous` / `NonCompliant`; **`AttestedRecording` is conferred by C** (the narrow contemporaneous-ID attestation gate, C.2) and is a reserved enum variant here.

**Files**
- create `crates/btctax-core/src/project/compliance.rs`
- modify `crates/btctax-core/src/project/mod.rs` (`pub mod compliance;` + re-export)
- modify `crates/btctax-core/src/lib.rs` (re-export `DisposalCompliance`, `ComplianceStatus`, `disposal_compliance`)
- create `crates/btctax-core/tests/compliance.rs`

**Interfaces**
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComplianceStatus { StandingOrder { effective_from: TaxDate }, Contemporaneous, AttestedRecording, NonCompliant }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisposalCompliance { pub disposal: EventId, pub wallet: WalletId, pub date: TaxDate, pub status: ComplianceStatus }
pub fn disposal_compliance(events: &[LedgerEvent], state: &LedgerState) -> Vec<DisposalCompliance>;
```

**Steps**

1. **Failing tests** — `crates/btctax-core/tests/compliance.rs` (self-contained helpers; `cold()` = self-custody, `cb()` = exchange/broker). **M4: real fixtures, not prose stubs.**
```rust
use btctax_core::event::*;
use btctax_core::identity::*;
use btctax_core::price::StaticPrices;
use btctax_core::project::{project, ProjectionConfig};
use btctax_core::{disposal_compliance, ComplianceStatus, LotMethod};
use rust_decimal_macros::dec;
use time::macros::{date, datetime, offset};

fn cb() -> WalletId { WalletId::Exchange { provider: "cb".into(), account: "m".into() } }   // broker
fn cold() -> WalletId { WalletId::SelfCustody { label: "cold".into() } }                      // self-custody
fn ev_in(rf: &str, ts: time::OffsetDateTime, wal: &WalletId, p: EventPayload) -> LedgerEvent {
    LedgerEvent { id: EventId::import(Source::Coinbase, SourceRef::new(rf)), utc_timestamp: ts,
        original_tz: offset!(+00:00), wallet: Some(wal.clone()), payload: p }
}
fn buy_in(rf: &str, ts: time::OffsetDateTime, wal: &WalletId, sat: i64, cost: rust_decimal::Decimal) -> LedgerEvent {
    ev_in(rf, ts, wal, EventPayload::Acquire(Acquire { sat, usd_cost: cost, fee_usd: dec!(0), basis_source: BasisSource::ExchangeProvided }))
}
fn sell_in(rf: &str, ts: time::OffsetDateTime, wal: &WalletId, sat: i64, proceeds: rust_decimal::Decimal) -> LedgerEvent {
    ev_in(rf, ts, wal, EventPayload::Dispose(Dispose { sat, usd_proceeds: proceeds, fee_usd: dec!(0), kind: DisposeKind::Sell }))
}
fn dec_ev(seq: u64, ts: time::OffsetDateTime, p: EventPayload) -> LedgerEvent {
    LedgerEvent { id: EventId::decision(seq), utc_timestamp: ts, original_tz: offset!(+00:00), wallet: None, payload: p }
}
fn election(seq: u64, made: time::OffsetDateTime, eff: time::Date, m: LotMethod) -> LedgerEvent {
    dec_ev(seq, made, EventPayload::MethodElection(MethodElection { effective_from: eff, method: m }))
}
fn lot_selection(seq: u64, made: time::OffsetDateTime, disposal_ref: &str, picks: Vec<LotPick>) -> LedgerEvent {
    dec_ev(seq, made, EventPayload::LotSelection(LotSelection {
        disposal_event: EventId::import(Source::Coinbase, SourceRef::new(disposal_ref)), lots: picks }))
}
fn pid(rf: &str) -> LotId { LotId { origin_event_id: EventId::import(Source::Coinbase, SourceRef::new(rf)), split_sequence: 0 } }
fn status_of(evs: &[LedgerEvent]) -> ComplianceStatus {
    let st = project(evs, &StaticPrices::default(), &ProjectionConfig::default());
    let dc = disposal_compliance(evs, &st);
    assert_eq!(dc.len(), 1, "expected exactly one post-2025 disposal-compliance entry");
    dc[0].status.clone()
}

#[test]
fn standing_order_status_self_custody() {
    // self-custody buy 2025-02, election eff 2025-06, sell 2025-07 -> in-force own-books standing order covers it.
    let evs = vec![
        buy_in("A", datetime!(2025-02-01 00:00:00 UTC), &cold(), 100_000, dec!(50.00)),
        election(1, datetime!(2025-05-01 00:00:00 UTC), date!(2025-06-01), LotMethod::Hifo),
        sell_in("D", datetime!(2025-07-01 00:00:00 UTC), &cold(), 100_000, dec!(70.00)),
    ];
    assert!(matches!(status_of(&evs), ComplianceStatus::StandingOrder { effective_from } if effective_from == date!(2025-06-01)));
}
#[test]
fn contemporaneous_status_when_selection_made_before_sale() {
    // selection made-date (2025-07-01) <= disposal date (2025-07-01) -> Contemporaneous (canonical A.5(b)).
    let evs = vec![
        buy_in("A", datetime!(2025-02-01 00:00:00 UTC), &cold(), 100_000, dec!(50.00)),
        sell_in("D", datetime!(2025-07-01 00:00:00 UTC), &cold(), 100_000, dec!(70.00)),
        lot_selection(1, datetime!(2025-07-01 00:00:00 UTC), "D", vec![LotPick { lot: pid("A"), sat: 100_000 }]),
    ];
    assert_eq!(status_of(&evs), ComplianceStatus::Contemporaneous);
}
#[test]
fn post_hoc_selection_is_noncompliant() {
    // selection made-date (2025-09-01) AFTER the sale (2025-07-01), no election -> NonCompliant (no post-hoc, §1.1012-1(j)).
    let evs = vec![
        buy_in("A", datetime!(2025-02-01 00:00:00 UTC), &cold(), 100_000, dec!(50.00)),
        sell_in("D", datetime!(2025-07-01 00:00:00 UTC), &cold(), 100_000, dec!(70.00)),
        lot_selection(1, datetime!(2025-09-01 00:00:00 UTC), "D", vec![LotPick { lot: pid("A"), sat: 100_000 }]),
    ];
    assert_eq!(status_of(&evs), ComplianceStatus::NonCompliant);
}
#[test]
fn noncompliant_when_no_basis() {
    // self-custody sell 2025-07, no election/selection -> FIFO is the defensible position -> NonCompliant.
    let evs = vec![
        buy_in("A", datetime!(2025-02-01 00:00:00 UTC), &cold(), 100_000, dec!(50.00)),
        sell_in("D", datetime!(2025-07-01 00:00:00 UTC), &cold(), 100_000, dec!(70.00)),
    ];
    assert_eq!(status_of(&evs), ComplianceStatus::NonCompliant);
}
#[test]
fn broker_2027_plus_is_noncompliant_even_with_election() {
    // Exchange (broker) wallet, 2027 disposal, in-force election + contemporaneous selection:
    // own-books is insufficient 2027+ (broker-communication rule) -> NonCompliant.
    let evs = vec![
        buy_in("A", datetime!(2025-02-01 00:00:00 UTC), &cb(), 100_000, dec!(50.00)),
        election(1, datetime!(2025-05-01 00:00:00 UTC), date!(2025-06-01), LotMethod::Hifo),
        sell_in("D", datetime!(2027-03-01 00:00:00 UTC), &cb(), 100_000, dec!(70.00)),
        lot_selection(2, datetime!(2027-03-01 00:00:00 UTC), "D", vec![LotPick { lot: pid("A"), sat: 100_000 }]),
    ];
    assert_eq!(status_of(&evs), ComplianceStatus::NonCompliant);
}
#[test]
fn broker_2026_own_books_election_is_standing_order() {
    // Exchange wallet, 2026 disposal, in-force own-books election -> StandingOrder (relief runs through 2026).
    let evs = vec![
        buy_in("A", datetime!(2025-02-01 00:00:00 UTC), &cb(), 100_000, dec!(50.00)),
        election(1, datetime!(2025-05-01 00:00:00 UTC), date!(2025-06-01), LotMethod::Hifo),
        sell_in("D", datetime!(2026-03-01 00:00:00 UTC), &cb(), 100_000, dec!(70.00)),
    ];
    assert!(matches!(status_of(&evs), ComplianceStatus::StandingOrder { .. }));
}
```
2. **Run → RED.**
3. **Minimal impl** — `compliance.rs`:
```rust
use crate::conventions::{tax_date, TaxDate, TRANSITION_DATE};
use crate::event::{EventPayload, LedgerEvent};
use crate::identity::{EventId, WalletId};
use crate::state::LedgerState;
use crate::LotMethod;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComplianceStatus { StandingOrder { effective_from: TaxDate }, Contemporaneous, AttestedRecording, NonCompliant }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisposalCompliance { pub disposal: EventId, pub wallet: WalletId, pub date: TaxDate, pub status: ComplianceStatus }

struct Election { effective_from: TaxDate, decision_seq: u64 }

/// Re-derive the in-force-eligible elections from events (mirrors resolve.rs's collection; excludes voided +
/// back-dated/pre-transition). NOTE: kept in sync with resolve.rs by the shared spec rule; a Minor follow-up
/// may extract a single shared collector.
fn elections(events: &[LedgerEvent], voided: &BTreeSet<EventId>) -> Vec<Election> {
    let mut out = Vec::new();
    for e in events {
        let EventId::Decision { seq } = e.id else { continue; };
        if voided.contains(&e.id) { continue; }
        if let EventPayload::MethodElection(me) = &e.payload {
            let made = tax_date(e.utc_timestamp, e.original_tz);
            if me.effective_from >= TRANSITION_DATE && me.effective_from >= made {
                out.push(Election { effective_from: me.effective_from, decision_seq: seq });
            }
        }
    }
    out
}

pub fn disposal_compliance(events: &[LedgerEvent], state: &LedgerState) -> Vec<DisposalCompliance> {
    let voided: BTreeSet<EventId> = events.iter().filter_map(|e| match &e.payload {
        EventPayload::VoidDecisionEvent(v) => Some(v.target_event_id.clone()), _ => None }).collect();
    let elections = elections(events, &voided);
    let wallet_of: BTreeMap<EventId, WalletId> = events.iter()
        .filter_map(|e| e.wallet.clone().map(|w| (e.id.clone(), w))).collect();
    // non-voided LotSelection made-dates, keyed by disposal_event. NFR4 (M1): iterate decisions in
    // `decision_seq` order (NOT raw &[LedgerEvent] slice order), so when a disposal has >1 (conflicting)
    // LotSelection the surviving made-date is the HIGHEST-seq one — deterministic and load-order-independent
    // (mirrors resolve.rs's decisions sort, resolve.rs:311-318). The duplicate itself is a DecisionConflict
    // handled by resolve and gates tax independently; this map only needs a stable made-date for the status.
    let mut selections: Vec<(u64, &LedgerEvent)> = events.iter()
        .filter_map(|e| match e.id { EventId::Decision { seq } => Some((seq, e)), _ => None })
        .filter(|(_, e)| !voided.contains(&e.id) && matches!(e.payload, EventPayload::LotSelection(_)))
        .collect();
    selections.sort_by_key(|(s, _)| *s);
    let mut sel_made: BTreeMap<EventId, TaxDate> = BTreeMap::new();
    for (_seq, e) in &selections {
        if let EventPayload::LotSelection(ls) = &e.payload {
            sel_made.insert(ls.disposal_event.clone(), tax_date(e.utc_timestamp, e.original_tz));
        }
    }

    let classify = |disposal: &EventId, date: TaxDate| -> ComplianceStatus {
        let broker = matches!(wallet_of.get(disposal), Some(WalletId::Exchange { .. }));
        if broker && date.year() >= 2027 {
            // own-books is insufficient for broker-held units 2027+ (broker-side instruction needed; C's gate).
            return ComplianceStatus::NonCompliant;
        }
        if let Some(made) = sel_made.get(disposal) {
            if *made <= date { return ComplianceStatus::Contemporaneous; } // A.5(b)
        }
        if let Some(ef) = elections.iter().filter(|e| e.effective_from <= date)
            .max_by(|a, b| a.effective_from.cmp(&b.effective_from).then(a.decision_seq.cmp(&b.decision_seq)))
            .map(|e| e.effective_from) {
            return ComplianceStatus::StandingOrder { effective_from: ef };
        }
        ComplianceStatus::NonCompliant
    };

    let mut out = Vec::new();
    for d in &state.disposals {
        if d.fee_mini_disposition || d.disposed_at < TRANSITION_DATE { continue; }
        if let Some(w) = wallet_of.get(&d.event) {
            out.push(DisposalCompliance { disposal: d.event.clone(), wallet: w.clone(), date: d.disposed_at,
                status: classify(&d.event, d.disposed_at) });
        }
    }
    for r in &state.removals {
        if r.removed_at < TRANSITION_DATE { continue; }
        if let Some(w) = wallet_of.get(&r.event) {
            out.push(DisposalCompliance { disposal: r.event.clone(), wallet: w.clone(), date: r.removed_at,
                status: classify(&r.event, r.removed_at) });
        }
    }
    out.sort_by(|a, b| a.disposal.cmp(&b.disposal)); // NFR4 total order
    out
}
```
   `mod.rs`: `pub mod compliance;` + `pub use compliance::{disposal_compliance, ComplianceStatus, DisposalCompliance};`. `lib.rs`: these flow through `pub use project::{…}` — add them to the `pub use project::{ … }` list (`lib.rs:15-17`).
4. **Run → GREEN.** Whole suite.
5. **Commit:** `feat(core): DisposalCompliance projection (custody mapping + 2025-2026/2027+ envelope)`.

---

## TASK 8 — `verify` surfacing

**Goal.** `verify` reports: the declared `pre2025_method` (+ whether attested); the standing-order history (each `MethodElection`'s recorded date + `effective_from` + method); the count of `LotSelection`s; the per-disposal `DisposalCompliance`; and the new hard blockers (auto-partitioned by `severity()`).

**Files**
- modify `crates/btctax-cli/src/render.rs` (`VerifyReport` fields; `build_verify` signature; `render_verify` lines)
- modify `crates/btctax-cli/src/cmd/inspect.rs` (pass `CliConfig`)
- modify `crates/btctax-cli/tests/verify_report.rs` (add cases)

**Interfaces**
```rust
// render.rs
pub struct ElectionLine { pub recorded: TaxDate, pub effective_from: TaxDate, pub method: LotMethod, pub note: &'static str }
pub struct VerifyReport {
    pub conservation: ConservationReport, pub hard: Vec<Blocker>, pub advisory: Vec<Blocker>,
    pub pending: usize, pub unknown_basis_inbounds: usize, pub safe_harbor: String,
    pub declared_pre2025_method: LotMethod, pub pre2025_method_attested: bool,
    pub elections: Vec<ElectionLine>, pub selection_count: usize, pub compliance: Vec<DisposalCompliance>,
}
pub fn build_verify(state: &LedgerState, events: &[LedgerEvent], cli: &CliConfig) -> VerifyReport;
```

**Steps**

1. **Failing tests** — `crates/btctax-cli/tests/verify_report.rs`:
```rust
#[test]
fn verify_reports_declared_method_and_attestation() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    cmd::admin::set_pre2025_method(&vault, &pp(), btctax_core::LotMethod::Hifo, true).unwrap();
    let report = cmd::inspect::verify(&vault, &pp()).unwrap();
    assert_eq!(report.declared_pre2025_method, btctax_core::LotMethod::Hifo);
    assert!(report.pre2025_method_attested);
    let text = render::render_verify(&report);
    assert!(text.contains("HIFO") && text.contains("attested"));
}

// M4: real fixtures (not prose stubs). This file already declares `mod fixtures;` and `fn pp()`; add
// `use btctax_cli::Session;`, `use time::macros::{date, datetime};`, and
// `fn now() -> time::OffsetDateTime { datetime!(2026-02-01 12:00:00 UTC) }` to the preamble.
#[test]
fn verify_lists_election_history_and_selection_count_and_compliance() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    // Post-2025 buy + sell (+ send): exactly one 2025 disposal; cb-sell consumes one lot fully -> a single leg.
    cmd::import::run(&vault, &pp(), &[fixtures::coinbase_buy_sell_send(dir.path())]).unwrap();
    // Forward standing order (MethodElection) effective 2025-06-01 — Task 5's set_forward_method APPENDS it.
    cmd::reconcile::set_forward_method(&vault, &pp(), btctax_core::LotMethod::Hifo, Some(date!(2025-06-01)), now()).unwrap();
    // Read the 2025 disposal eventref + the lot/sat its single leg consumes; record a contemporaneous selection.
    let (disposal_ref, lot_ref, principal) = {
        let s = Session::open(&vault, &pp()).unwrap();
        let (state, _) = s.project().unwrap();
        let leg = &state.disposals[0].legs[0];
        (state.disposals[0].event.canonical(),
         format!("{}#{}", leg.lot_id.origin_event_id.canonical(), leg.lot_id.split_sequence),
         leg.sat)
    };
    let picks = vec![btctax_cli::eventref::parse_lot_pick(&format!("{lot_ref}:{principal}")).unwrap()];
    cmd::reconcile::select_lots(&vault, &pp(), &disposal_ref, picks, now()).unwrap();

    let report = cmd::inspect::verify(&vault, &pp()).unwrap();
    assert_eq!(report.elections.len(), 1);
    assert_eq!(report.selection_count, 1);
    assert!(!report.compliance.is_empty());
}

#[test]
fn verify_partitions_lot_selection_invalid_as_hard() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    cmd::import::run(&vault, &pp(), &[fixtures::coinbase_buy_sell_send(dir.path())]).unwrap();
    let (disposal_ref, lot_ref) = {
        let s = Session::open(&vault, &pp()).unwrap();
        let (state, _) = s.project().unwrap();
        let leg = &state.disposals[0].legs[0];
        (state.disposals[0].event.canonical(),
         format!("{}#{}", leg.lot_id.origin_event_id.canonical(), leg.lot_id.split_sequence))
    };
    // pick 1 sat — deliberately != the disposal principal -> conservation violation -> hard LotSelectionInvalid.
    let picks = vec![btctax_cli::eventref::parse_lot_pick(&format!("{lot_ref}:1")).unwrap()];
    cmd::reconcile::select_lots(&vault, &pp(), &disposal_ref, picks, now()).unwrap();
    let report = cmd::inspect::verify(&vault, &pp()).unwrap();
    assert!(report.hard.iter().any(|b| b.kind == btctax_core::BlockerKind::LotSelectionInvalid));
}
```
2. **Run → RED.**
3. **Minimal impl:**
   - `render.rs`: add fields to `VerifyReport`; change `build_verify` to take `cli: &CliConfig` (import `crate::config::CliConfig` and `btctax_core::{disposal_compliance, ComplianceStatus, DisposalCompliance, LotMethod, MethodElection, EventPayload, EventId}`). Build the election lines + selection count from `events` (recorded date via `tax_date(e.utc_timestamp, e.original_tz)`; mark `note` = `"backdated/ignored"` when `effective_from < TRANSITION_DATE || effective_from < recorded`, `"voided"` when in the voided set, else `"in force"`). Set `declared_pre2025_method = cli.pre2025_method`, `pre2025_method_attested = cli.pre2025_method_attested`, `compliance = disposal_compliance(events, state)`. `hard`/`advisory` partition unchanged (`render.rs:327-332`) — the new hard kinds flow through `severity()` automatically. Append render lines in `render_verify`:
```rust
let _ = writeln!(out, "Pre-2025 method (attested historical fact): {:?} (attested: {})",
                 r.declared_pre2025_method, r.pre2025_method_attested);
let _ = writeln!(out, "Standing orders (MethodElection): {}", r.elections.len());
for e in &r.elections {
    let _ = writeln!(out, "  recorded {} effective {} -> {:?} [{}]", e.recorded, e.effective_from, e.method, e.note);
}
let _ = writeln!(out, "Lot selections recorded: {}", r.selection_count);
let _ = writeln!(out, "Per-disposal compliance (post-2025): {}", r.compliance.len());
for c in &r.compliance {
    let _ = writeln!(out, "  {} @ {} :: {:?}", c.disposal.canonical(), c.date, c.status);
}
```
   - `inspect.rs`: `verify` reads the CLI config and passes it:
```rust
pub fn verify(vault_path: &Path, pp: &Passphrase) -> Result<VerifyReport, CliError> {
    let session = Session::open(vault_path, pp)?;
    let (events, state, _cfg) = session.load_events_and_project()?;
    let cli = session.config()?;
    Ok(build_verify(&state, &events, &cli))
}
```
4. **Run → GREEN.** Whole suite. (Existing `verify_report.rs` tests use `report.hard`/`report.safe_harbor`/`render_verify` — unaffected; `build_verify` is only called by `inspect::verify`.)
5. **Commit:** `feat(cli): verify surfaces declared method/attestation, election history, selection count, per-disposal compliance`.

---

## TASK 9 — A.6 evaluate entrypoint

**Goal.** One internal, side-effect-free entrypoint that folds a candidate disposal (existing-ledger **or** synthetic) plus a candidate selection through the **same** `consume`/validation/scoring path and returns resulting lots/gains/ST-LT split. Requires `--proceeds` when no dataset price exists for the candidate's date.

**Files**
- create `crates/btctax-core/src/project/evaluate.rs`
- modify `crates/btctax-core/src/project/mod.rs` (`pub mod evaluate;` + re-export) + `crates/btctax-core/src/lib.rs`
- create `crates/btctax-core/tests/evaluate.rs`

**Interfaces**
```rust
#[derive(Debug, Clone)]
pub struct CandidateDisposal {
    pub existing_event: Option<EventId>,   // Some -> score an existing disposal; None -> synthetic (Mode-2)
    pub wallet: WalletId, pub date: TaxDate, pub sat: Sat, pub kind: DisposeKind, pub proceeds: Option<Usd>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluateOutcome { pub legs: Vec<DisposalLeg>, pub st_gain: Usd, pub lt_gain: Usd, pub lots_after: Vec<Lot>, pub blockers: Vec<Blocker> }
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvaluateError { ProceedsRequired, UnknownExistingDisposal }
pub fn evaluate_disposal(events: &[LedgerEvent], prices: &dyn PriceProvider, config: &ProjectionConfig,
                         candidate: &CandidateDisposal, selection: Option<&[LotPick]>) -> Result<EvaluateOutcome, EvaluateError>;
```

**Steps**

1. **Failing tests** — `crates/btctax-core/tests/evaluate.rs`:
```rust
use btctax_core::project::{evaluate_disposal, CandidateDisposal, EvaluateError};
// helpers buy/sell/imp/w as before; StaticPrices with an entry for one date.

#[test]
fn synthetic_future_disposal_requires_proceeds_when_no_price() {
    let evs = vec![buy("A", datetime!(2025-02-01 00:00:00 UTC), 100_000, dec!(50.00))];
    let cand = CandidateDisposal { existing_event: None, wallet: w(), date: date!(2030-01-01), sat: 100_000,
        kind: DisposeKind::Sell, proceeds: None };
    let err = evaluate_disposal(&evs, &StaticPrices::default(), &ProjectionConfig::default(), &cand, None).unwrap_err();
    assert_eq!(err, EvaluateError::ProceedsRequired);
}

#[test]
fn synthetic_disposal_with_proceeds_returns_gain_and_is_side_effect_free() {
    let evs = vec![buy("A", datetime!(2025-02-01 00:00:00 UTC), 100_000, dec!(50.00))];
    let cand = CandidateDisposal { existing_event: None, wallet: w(), date: date!(2026-06-01), sat: 100_000,
        kind: DisposeKind::Sell, proceeds: Some(dec!(150.00)) };
    let out = evaluate_disposal(&evs, &StaticPrices::default(), &ProjectionConfig::default(), &cand, None).unwrap();
    assert_eq!(out.legs.len(), 1);
    assert_eq!(out.legs[0].gain, dec!(100.00)); // 150 - 50
    assert_eq!(out.lt_gain, dec!(100.00));       // acquired 2025-02, sold 2026-06 -> LT
    // side-effect-free: a plain projection of the original events still has no disposals.
    let base = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(base.disposals.is_empty());
}

#[test]
fn synthetic_disposal_uses_dataset_fmv_when_proceeds_omitted_and_price_exists() {
    let mut px = std::collections::BTreeMap::new();
    px.insert(date!(2026-06-01), dec!(100000.00)); // $/BTC -> FMV(100k sat) = $100
    let prices = StaticPrices(px);
    let evs = vec![buy("A", datetime!(2025-02-01 00:00:00 UTC), 100_000, dec!(50.00))];
    let cand = CandidateDisposal { existing_event: None, wallet: w(), date: date!(2026-06-01), sat: 100_000,
        kind: DisposeKind::Sell, proceeds: None };
    let out = evaluate_disposal(&evs, &prices, &ProjectionConfig::default(), &cand, None).unwrap();
    assert_eq!(out.legs[0].proceeds, dec!(100.00));
}

#[test]
fn existing_disposal_scored_with_an_injected_selection() {
    // ledger has a post-2025 sell; evaluate a candidate selection over it WITHOUT persisting anything.
    let evs = vec![
        buy("A", datetime!(2025-02-01 00:00:00 UTC), 100_000, dec!(50.00)),
        buy("B", datetime!(2025-03-01 00:00:00 UTC), 100_000, dec!(90.00)),
        sell("D", datetime!(2025-07-01 00:00:00 UTC), 100_000, dec!(95.00)),
    ];
    let cand = CandidateDisposal { existing_event: Some(EventId::import(Source::Coinbase, SourceRef::new("D"))),
        wallet: w(), date: date!(2025-07-01), sat: 100_000, kind: DisposeKind::Sell, proceeds: None };
    let picks = vec![LotPick { lot: LotId { origin_event_id: EventId::import(Source::Coinbase, SourceRef::new("B")), split_sequence: 0 }, sat: 100_000 }];
    let out = evaluate_disposal(&evs, &StaticPrices::default(), &ProjectionConfig::default(), &cand, Some(&picks)).unwrap();
    assert_eq!(out.legs[0].basis, dec!(90.00)); // scored against the picked lot B (default FIFO would pick A)
}
```
2. **Run → RED.**
3. **Minimal impl** — `evaluate.rs` (reuses the proven clone-append-fold-discard pattern of `universal_snapshot`):
```rust
use crate::conventions::{Sat, TaxDate, Usd};
use crate::event::{DisposeKind, LedgerEvent, LotPick};
use crate::identity::{EventId, SourceRef, WalletId};
use crate::price::{fmv_of, PriceProvider};
use crate::project::resolve::{resolve, Eff, Op};
use crate::project::fold::fold;
use crate::state::{Blocker, BlockerKind, DisposalLeg, Lot, Term};
use crate::ProjectionConfig;
use time::OffsetDateTime;

#[derive(Debug, Clone)]
pub struct CandidateDisposal {
    pub existing_event: Option<EventId>, pub wallet: WalletId, pub date: TaxDate, pub sat: Sat,
    pub kind: DisposeKind, pub proceeds: Option<Usd>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluateOutcome { pub legs: Vec<DisposalLeg>, pub st_gain: Usd, pub lt_gain: Usd, pub lots_after: Vec<Lot>, pub blockers: Vec<Blocker> }
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvaluateError { ProceedsRequired, UnknownExistingDisposal }

fn honoring(op: &Op) -> bool { matches!(op, Op::Dispose{..}|Op::GiftOut{..}|Op::Donate{..}|Op::SelfTransfer{..}) }

pub fn evaluate_disposal(events: &[LedgerEvent], prices: &dyn PriceProvider, config: &ProjectionConfig,
                         candidate: &CandidateDisposal, selection: Option<&[LotPick]>) -> Result<EvaluateOutcome, EvaluateError> {
    // proceeds: --proceeds wins; else dataset FMV; else error (Mode-2 future dates have no price).
    let proceeds = match candidate.proceeds {
        Some(p) => p,
        None => fmv_of(prices, candidate.date, candidate.sat).ok_or(EvaluateError::ProceedsRequired)?,
    };
    let mut res = resolve(events, prices, config);
    let target_id = match &candidate.existing_event {
        Some(id) => {
            if !res.timeline.iter().any(|e| &e.id == id && honoring(&e.op)) { return Err(EvaluateError::UnknownExistingDisposal); }
            id.clone()
        }
        None => {
            let id = EventId::Decision { seq: u64::MAX }; // reserved synthetic label; never a real decision_seq
            let utc: OffsetDateTime = candidate.date.midnight().assume_utc(); // tax_date(utc, UTC) == candidate.date
            res.timeline.push(Eff {
                id: id.clone(), utc, tz: time::UtcOffset::UTC, src_priority: 0, src_ref: SourceRef::new("__synthetic__"),
                wallet: Some(candidate.wallet.clone()),
                op: Op::Dispose { sat: candidate.sat, proceeds, fee_usd: Usd::ZERO, fee_sat: None, kind: candidate.kind },
            });
            id
        }
    };
    // inject the candidate selection (overrides any persisted one); mirror resolve's principal-conservation guard.
    let mut extra = Vec::new();
    if let Some(picks) = selection {
        let picked: Sat = picks.iter().map(|p| p.sat).sum();
        if picked != candidate.sat {
            extra.push(Blocker { kind: BlockerKind::LotSelectionInvalid, event: Some(target_id.clone()),
                detail: format!("candidate selection must conserve principal: {picked} != {}", candidate.sat) });
        } else {
            res.selections.insert(target_id.clone(), picks.to_vec());
        }
    }
    let state = fold(res, prices, config); // same consume/validation/scoring path; thrown away after read
    let legs: Vec<DisposalLeg> = state.disposals.iter().filter(|d| d.event == target_id).flat_map(|d| d.legs.clone()).collect();
    let st_gain: Usd = legs.iter().filter(|l| l.term == Term::ShortTerm).map(|l| l.gain).sum();
    let lt_gain: Usd = legs.iter().filter(|l| l.term == Term::LongTerm).map(|l| l.gain).sum();
    let mut blockers: Vec<Blocker> = state.blockers.iter().filter(|b| b.event.as_ref() == Some(&target_id)).cloned().collect();
    blockers.extend(extra);
    Ok(EvaluateOutcome { legs, st_gain, lt_gain, lots_after: state.lots, blockers })
}
```
   `mod.rs`: `pub mod evaluate;` + `pub use evaluate::{evaluate_disposal, CandidateDisposal, EvaluateOutcome, EvaluateError};`; `lib.rs` re-export the same (and these also remain reachable as `btctax_core::project::evaluate_disposal`). Ensure `resolve` and `fold` are `pub` enough (both already `pub fn`; `Eff`/`Op` are `pub`).
4. **Run → GREEN.** Whole suite.
5. **Commit:** `feat(core): A.6 side-effect-free evaluate_disposal (synthetic + existing; proceeds-required for priced-less dates)`.

---

## TASK 10 — Whole-diff review + full-suite green (Phase E gate)

**Goal.** The mandatory post-implementation, independent, adversarial whole-diff review (`STANDARD_WORKFLOW.md §Phase E`), run as one system — catching cross-phase inconsistencies (constant/ordering drift, a guarantee promised but not delivered).

**Steps**
1. Run the full validation surface: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check`.
2. Confirm the cross-cutting KATs exist and pass: serde round-trip of the new `EventPayload` variants (`event.rs::every_variant_serde_round_trips`); `fingerprint(&MethodElection)/(&LotSelection) == None`; serde-default `SafeHarborAllocation.pre2025_method` (Task 6); load-order-independent determinism with elections+selections (Task 4); composition + conflict KATs (Task 6).
3. Dispatch an independent reviewer over the **entire** diff; persist verbatim to `reviews/R0-lot-id-substrate-whole-diff-round-N.md` **before** folding; loop to 0 Critical / 0 Important (re-review after every fold, including the last).
4. Flip any `FOLLOWUPS.md` items this change resolves; file new ones (e.g. the shared election/selection collector noted in Task 7).
5. **Ship commit** only when green.

---

## 4. Self-review

### 4.1 Spec coverage map (every Sub-project-A deliverable → task)

| Spec item | Where covered |
|---|---|
| A.1 `LotMethod → Fifo\|Lifo\|Hifo` | Task 1 |
| A.1 `pre2025_method` attested side-table (config key + attested flag) + `config --set-pre2025-method` | Task 1 |
| A.1 `MethodElection { effective_from, method }`; ≥ TRANSITION_DATE; ≥ made-date; latest-in-force (tie decision_seq); FIFO before; voided excluded; `config --set-forward-method` | Task 3 (engine); Task 5 (CLI `set_forward_method` + 2 round-trip tests, M3) |
| A.2 `LotSelection { disposal_event, lots: Vec<LotPick{lot: LotId, sat}> }` (keyed on full `LotId`) | Task 4 |
| A.2 `select-lots` + `import-selections` (CSV `disposal_ref,origin_event_id,split_sequence,sat`, header-validated); `LotId` parse over Import/Decision/Conflict origins | Task 5 |
| A.3 `consume(method, selection)`; four honoring sites; PendingOut + consume_fee stay FIFO | Task 2 (engine), Task 3 (wiring), Task 4 (selection wiring) |
| A.3 FIFO/LIFO/HIFO total orders; HIFO gain-basis-per-sat desc, ties oldest→lot_id, basis-pending last, loss-basis ignored | Task 2 |
| A.3 Universal→`pre2025_method`; Wallet→in-force election | Task 3 |
| A.4 conservation-of-principal; existence; post-2025 per-wallet; on-chain fee FIFO from remainder; dup→`DecisionConflict`; voided excluded; `LotSelectionInvalid` | Task 4 |
| A.5 `DisposalCompliance` (StandingOrder/Contemporaneous/AttestedRecording/NonCompliant); `WalletId` custody; 2025-2026 vs 2027+ envelope; contemporaneous = made-date ≤ time of sale | Task 7 |
| A.5 `verify` surfacing (declared method+attested, election recorded/effective history, selection count, per-disposal compliance); `Pre2025MethodNote` renders declared method | Task 8 (verify), Task 3 (note) |
| A.6 evaluate entrypoint (synthetic + existing; proceeds-required for priced-less dates) | Task 9 |
| A.7 immutable serde-default `SafeHarborAllocation.pre2025_method`; method-aware `universal_snapshot`; `Pre2025MethodConflictsAllocation` (never generic `SafeHarborUnconservable`); capture at attestation | Task 6 |
| New `BlockerKind`s (3) + Hard severity + verify partitioning | Task 3 (2), Task 6 (1), Task 8 (partitioning) |
| Composition KAT (non-FIFO residue → Path B conserves) + conflict KAT | Task 6 |
| Cross-cutting: serde backward-compat; `fingerprint = None`; determinism; no-float | Tasks 2/3/4/6 + Task 10 |

**`config --set-forward-method`** (A.1's CLI surface that *appends a `MethodElection` decision*, never a flag) — **now a real task with tests (M1→M3 fold)**: the engine + decision path are built in Task 3; the CLI `set_forward_method` (`reconcile.rs`) + its `Command::Config` dispatch branch + two round-trip tests (explicit and defaulted `effective_from`) are built in Task 5. No longer a footnote-only deliverable.

### 4.2 Placeholder scan

- No `todo!()`/`unimplemented!()`/`TODO`/`FIXME`/`...` left in *delivered* code. The only `...`/prose stubs are inside **test bodies** that describe a fixture to construct (e.g. "build vault, append election") — the implementer fills the concrete synthetic events using the documented helpers; these are test scaffolding, not shipped placeholders, and each test's asserted behavior is fully specified.
- `EventId::Decision { seq: u64::MAX }` (Task 9) and `SourceRef("__synthetic__")` are **reserved sentinels** for the throwaway evaluate timeline, never persisted — documented, collision-free against real decision seqs (which start at 1).

### 4.3 Type-consistency of every signature vs cited source

- `consume(&mut self, key: &PoolKey, need: Sat, method: LotMethod, selection: Option<&[LotPick]>) -> ConsumeResult` — `Sat = i64` (`conventions.rs:6`); `LotMethod` (Task 1); `LotPick` (Task 2, `event.rs`); `PoolKey` (`pools.rs:9`); `Consumed` reused unchanged (`pools.rs:104-118`).
- `fold_event(eff: &Eff, prices: &dyn PriceProvider, ctx: &FoldCtx, pools: &mut PoolSet, st: &mut LedgerState, stats: &mut FoldStats)` — matches the current arity minus `config`, plus `ctx` (`fold.rs:307-314`); only other caller is `transition.rs:40`, updated in Task 3/6.
- `universal_snapshot(timeline: &[Eff], prices: &dyn PriceProvider, config: &ProjectionConfig, method: LotMethod, elections: &[ElectionRec], selections: &BTreeMap<EventId, Vec<LotPick>>)` — extends `transition.rs:25-29`; sole caller `resolve.rs:520`, updated in Task 6.
- `MethodElection { effective_from: TaxDate, method: LotMethod }` / `LotSelection { disposal_event: EventId, lots: Vec<LotPick> }` / `LotPick { lot: LotId, sat: Sat }` — `TaxDate = Date` (`conventions.rs:10`); `EventId`/`LotId` (`identity.rs:55-120`). All derive `Serialize, Deserialize` like every other payload (`event.rs:7-202`); `LotMethod` gains serde in Task 1.
- `SafeHarborAllocation { …, #[serde(default)] pre2025_method: LotMethod }` — additive; needs `LotMethod: Default` (Task 1) for `#[serde(default)]`, mirroring `AllocLot`'s defaulted fields (`event.rs:150-153`).
- `BlockerKind::{MethodElectionBackdated, LotSelectionInvalid, Pre2025MethodConflictsAllocation}` — added to the enum + the `Hard` arm of `severity()` (`state.rs:22-49`); B's refusal (spec B.4) keys on `severity() == Hard`, so these auto-gate.
- `disposal_compliance(events: &[LedgerEvent], state: &LedgerState) -> Vec<DisposalCompliance>` — reads `LedgerEvent.wallet` (`event.rs:227`), `Disposal.{event,disposed_at,fee_mini_disposition}` (`state.rs:93-101`), `Removal.{event,removed_at}` (`state.rs:117-124`).
- `evaluate_disposal(events: &[LedgerEvent], prices: &dyn PriceProvider, config: &ProjectionConfig, candidate: &CandidateDisposal, selection: Option<&[LotPick]>) -> Result<EvaluateOutcome, EvaluateError>` — `DisposalLeg`/`Lot`/`Blocker`/`Term` (`state.rs`); `fmv_of` (`price.rs:13`); `Op::Dispose` field set matches `resolve.rs:15-24`; `Date::midnight().assume_utc()` is `time` API, `tax_date(utc, UTC)` returns the same calendar date (`conventions.rs:52-54`).
- CLI: `select_lots`/`import_selections` mirror existing emitter shape `(vault_path: &Path, pp: &Passphrase, …, now: OffsetDateTime) -> Result<EventId, CliError>` (`reconcile.rs:35-49`); `append_decision`/`append_and_save` reused (`reconcile.rs:22-30`, `persistence.rs:238`); `csv`/`tempfile` already deps (`btctax-cli/Cargo.toml:25,30`). `parse_lot_id`/`parse_lot_pick` build on `parse_event_id` (`eventref.rs:22-52`). `build_verify(state, events, cli)` — sole caller `inspect.rs:30`.

### 4.4 Ambiguities resolved (for the R0 reviewer)

1. **`AttestedRecording` in Sub-project A.** A's binding `LotSelection` payload (A.2) carries **no** attestation field; A would introduce a spec-deviating field to *produce* `AttestedRecording`. Resolution: define the full four-variant `ComplianceStatus` enum (so C can confer `AttestedRecording` via its narrow contemporaneous-ID attestation gate, C.2) but A's classifier produces only `StandingOrder`/`Contemporaneous`/`NonCompliant`. This keeps A's event shape exactly as the spec mandates. (Documented in Task 7.)
2. **Compliance scope = post-2025.** The four-state model + envelope are inherently the post-2025 identification regime; `disposal_compliance` emits entries only for disposals/removals dated ≥ `TRANSITION_DATE`. Pre-2025 disposals are governed by the attested `pre2025_method` (surfaced via the declared-method line + `Pre2025MethodNote`). SelfTransfers honor method/selection but leave no realized record, so they carry no compliance line in A.
3. **FIFO is acquisition-date order everywhere — a deliberate correctness change (C1), NOT a no-op.** Spec A.3 defines FIFO as `acquired_at` asc, tie `lot_id` asc (§1.1012-1(j)(3)(i): earliest *acquisition*). `consume_fifo` (PendingOut/fee + the four honoring sites) now delegates to this one total order. It is **not** equivalent to the legacy insertion-order front-walk: it **reorders relocated/seeded lots** (self-transfer into a populated wallet; Path-B non-`acquired_at` seeding; a pre-2025 self-transfer shifting the `universal_snapshot` residue `snap.basis`), correcting a **latent §1012 deviation** the foundation shipped. **No equivalence is claimed.** The change is locked by the RED→GREEN divergence KATs (Tasks 3 and 6) and the Task-2 fixture-re-verification step — *not* by "the suite stays green" (the legacy suite does not detect the divergence).
4. **`LotSelectionInvalid` fold behavior.** On an existence/per-wallet failure, the fold raises the hard blocker **and** falls back to method-order consumption so Σsat conservation holds and the projection stays total; the hard blocker gates tax (B/C refuse). Conservation-of-principal (a) and targeting are caught earlier in `resolve` (the selection is then never applied).
5. **`Pre2025MethodConflictsAllocation` keeps Path B effective.** The conflict is "live config ≠ recorded method" only; conservation is checked under the *recorded* method, so the allocation remains effective (Path B seeds) while the hard blocker forces the user to revert config — never coercing a rewrite of the irrevocable allocation (A.7.3).
6. **Named-struct payloads.** `MethodElection`/`LotSelection`/`LotPick` are named structs wrapped in tuple variants, matching the codebase convention (`event.rs`), with field names/types exactly as the spec writes them.

---

## 5. Fold record (R0 round 1)

R0 = `reviews/R0-plan-lot-id-substrate-round-1.md` (2026-06-29; verdict **1 Critical, 0 Important, 4 Minor, 3 Nit**). Engine facts re-verified against current source at fold time: `consume_fifo` walks insertion/push-order (`pools.rs:58-100`); a relocated fragment carries `acquired_at: c.acquired_at` and is `push_lot`'d to the **back** (`fold.rs:545,580-583`); Path-B seed lots pushed in alloc-index order (`resolve.rs:566-586` → `transition.rs:67-73`); `universal_snapshot` residue + conservation guard (`transition.rs:25-51`, `resolve.rs:546-547`); decisions sorted by seq (`resolve.rs:311-318`); `effective: Vec<(EventId, Vec<Lot>)>` + multiple-effective `DecisionConflict` (`resolve.rs:523,602-615`); `DisposalLeg { lot_id, sat, … }` (`state.rs:80-90`); `load_events_and_project` returns `ProjectionConfig` (FOLLOWUPS burndown 39e09e0).

**Critical**
- **C1 — adopt acquisition-date FIFO DELIBERATELY (no equivalence claim).** Every "push-order ≡ total-order / distinct `acquired_at` ⇒ equivalent / behavior is identical" claim **removed** (Task 2 step 4 regression-watch; Ambiguity #3). FIFO is now stated **once** as **acquisition-date order** (`acquired_at` asc, tie `lot_id` asc), applied at all six consume sites as a **deliberate material correctness change** that replaces insertion-order FIFO for relocated/seeded lots (a latent §1012 deviation: §1.1012-1(j)(3)(i) sells earliest *acquisition*, and a self-transfer carries the original `acquired_at`). Added **RED→GREEN divergence KATs**: **(a)** self-transfer relocation of an OLDER lot into a wallet already holding a NEWER directly-acquired lot, asserting consumed basis/term under **FIFO + LIFO + HIFO** (Task 3 `relocated_older_lot_consumed_first_…`); **(b)** Path-B safe-harbor seeding with same-wallet lots in **non-`acquired_at`** order, asserting post-seed oldest-first consumption (Task 6 `path_b_seed_in_non_acq_order_…`); **(c)** a **pre-2025 self-transfer** reordering the single Universal pool, asserting the `snap.basis` residue under the new order **and** that a correct allocation still conserves (no spurious `SafeHarborUnconservable`) (Task 6 `pre2025_self_transfer_reorders_universal_snapshot_residue_…`). Added an explicit **fixture-re-verification step** (Task 2 step 4) over `kat_tax.rs` / `transition.rs` / `properties.rs` — each moved golden value documented **in-line with its tax reason**, never silently. Spec §A.3 reframed (tiebreak → deliberate adoption) + spec M2 fold-record line updated + `FOLLOWUPS.md` note (corrects a latent foundation FIFO deviation; no real users yet).

**Minors**
- **M1 — compliance made-date determinism (NFR4).** Task 7's `sel_made` map now iterates LotSelection decisions in **`decision_seq`** order (mirrors `resolve.rs:311-318`), not raw `&[LedgerEvent]` slice order — the surviving made-date is highest-seq-wins, load-order-independent.
- **M2 — conflict emitted AFTER Path selection.** Task 6 carries each candidate's recorded method in `effective: Vec<(EventId, Vec<Lot>, LotMethod)>` and emits `Pre2025MethodConflictsAllocation` **only in the single-effective `match effective.len()` arm**, so the multiple-effective (`DecisionConflict` → Path A) case never fires a spurious method-conflict.
- **M3 — `config --set-forward-method`.** Promoted from a §4.1 footnote to a real Task 5 deliverable: `reconcile::set_forward_method` (appends a `MethodElection`, default `effective_from` = made-date) + a `Command::Config` dispatch branch + two round-trip tests (explicit + defaulted `effective_from`). §4.1 coverage row + footnote updated.
- **M4 — real test bodies (RED→GREEN).** Task 7 (`compliance.rs`) and Task 8 (`verify_report.rs`) prose-stub fixtures replaced with concrete, executable test code (synthetic-event fixtures for compliance; real CLI command + `coinbase_buy_sell_send` fixture flow for verify, reading the disposal/lot refs from the projection).

**Nits**
- **N1 — folded.** `parse_lot_id` round-trip test gains a `#`-in-`source_ref` case (`"in|99|credit|1#0"`), locking the `rsplit_once('#')` choice.
- **N2 — folded.** `hifo_cmp` uses `== Usd::ZERO` (no `num_traits::Zero` scope dependency; matches `fold.rs`'s `> Usd::ZERO` idiom).
- **N3 — recorded N/A (`FOLLOWUPS.md`).** `load_events_and_project` returns `ProjectionConfig`, not `CliConfig`, so `verify`'s separate `session.config()?` read is **required**, not redundant.

**Self-consistency pass.** No residual equivalence claim remains (Task 2 step 4 and Ambiguity #3 both reframed; no "push-order ≡ total-order" / "behavior is identical on all current fixtures" text anywhere in the plan). FIFO is defined **once** as acquisition-date order. The three divergence KATs (a/b/c) + the fixture-re-verification step are present. M1–M4 are reflected in the task bodies. Spec §A.3 is reconciled with the plan (deliberate-correctness note; M2 fold-record updated).
