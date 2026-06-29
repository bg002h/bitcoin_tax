# btctax-core (Domain + Event-Sourced Projection) Implementation Plan — Foundation Plan 2 of 4 (v1)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `btctax-core`, the domain model + the **pure, deterministic, two-pass event-sourced projection** that turns an append-only `LedgerEvent` set into a per-wallet Bitcoin lot ledger (`LedgerState`: lots, holdings, disposals, removals, recognized income, the reconciliation queue, and all blockers). The crate encodes the spec's tax positions TP1–TP11 as named, swappable fold rules; it never panics (totality); identical event *sets* yield identical ledgers regardless of storage/load order (NFR4); and it uses exact arithmetic only (NFR5). A thin, isolated persistence module is the crate's sole I/O touchpoint; the projection itself is `#![forbid]`-pure.

**Architecture:** Two layers with a hard boundary.
1. **Domain + projection (pure, no I/O).** `project(events: &[LedgerEvent], prices: &dyn PriceProvider, config: &ProjectionConfig) -> LedgerState` is a total pure function. It runs the spec's **two-pass model (§7.2)**: *Pass 1* resolves decision/correction events onto the imported timeline in `decision_seq` order (staged: non-allocation decisions → effective timeline → 2025-transition mode → allocation-void adjudication) and emits `decision_conflicts`/`import_conflicts`; *Pass 2* folds the effective imported timeline in **canonical order** (`utc_timestamp` → source priority `Swan>Coinbase>Gemini>River` → `source_ref`) into lots/disposals/removals/income/holdings. Determinism is a property of being a pure function of the *set* plus the canonical sort; we test it with permutation harnesses.
2. **Persistence glue (the only I/O).** A thin `persistence` module reads/appends the canonical event rows over a borrowed `&rusqlite::Connection` (the live in-memory handle that `btctax-store::Vault::conn()` returns — wired in by the CLI, Plan 4). **Decision (chosen): core operates on a pure `Vec<LedgerEvent>` and the projection takes `&[LedgerEvent]`; the persistence module depends only on `rusqlite` (not on `btctax-store`).** Justification: (a) keeps the projection trivially testable and deterministic with hand-built event vectors and an in-memory price stub — no crypto/vault needed; (b) avoids a heavyweight dependency on the whole vault for what is one connection borrow; (c) the projection never sees a `Connection`, so purity/NFR4 cannot regress; (d) core owns the *event-table* DDL + event (de)serialization (it owns the event schema), while `btctax-store` owns the *blob/layout* migration — clean separation. The CLI passes `vault.conn()` (a `&rusqlite::Connection`) into `core::persistence`.

**BTC-only.** Non-BTC assets are out of scope (spec §1, §15); the engine models satoshis + USD only.

**Tech Stack:** Rust (edition 2021, rust-version 1.74 — workspace pins), `rust_decimal` (exact USD money; NFR5), `time` (UTC instants + fixed-offset calendar dates; tax-date comparisons per §6.1), `serde` + `serde_json` (event payload (de)serialization), `sha2` (canonical content **fingerprint** for conflict detection, §6.2/§9), `rusqlite` 0.31 (bundled SQLite — the persistence glue only), `thiserror` (typed errors, matching `btctax-store`'s pattern). Dev: `rust_decimal_macros` (`dec!`), `time` `macros` (`date!`/`datetime!`), `proptest` (conservation/no-negative property tests, §13).

## Global Constraints
(Spec `design/SPEC_foundation.md`; every task implicitly includes these. Values are verbatim from the spec.)

- **NFR5 Exact arithmetic — no floats anywhere.** **BTC = integer satoshis = `Sat = i64`.** **USD = `Usd = rust_decimal::Decimal`.** **Rounding = `ROUND_HALF_EVEN`** (`rust_decimal::RoundingStrategy::MidpointNearestEven`), exposed as `domain::conventions::MONEY_ROUNDING`. Money is rounded to the **cent (2 dp)** only at value-producing boundaries; pro-rata splits use the **remainder-takes-the-rest** rule so sums are conserved exactly (`Σ` invariants, §13).
- **NFR4 Determinism.** Identical event *set* → identical `LedgerState`, invariant to storage/load order, with each event's `(source_ref|decision_seq, payload)` fixed. `project` is a **pure** function of `(events, prices, config)`; pass 2 sorts by the **canonical order** below; pass 1 resolves decisions in **`decision_seq`** order.
- **§6.2 Canonical fold order (pass 2):** `utc_timestamp` → **fixed source priority `Swan > Coinbase > Gemini > River`** (arbitrary-but-stable) → `source_ref`. **Decision order (pass 1):** ascending `decision_seq`. `ImportConflict` is **never folded** in pass 2 (consumed only as a blocker).
- **§6.1 Tax-date basis.** All tax-date comparisons use the **calendar date in `original_tz` at day granularity**: holding period (TP4), the 2025-01-01 pre/post boundary (§7.4), and the safe-harbor made-date-vs-first-2025-event test (§7.4). `tax_date(utc, tz) = utc.to_offset(tz).date()`.
- **§7.1 Projection contract:** `project` is **pure, deterministic, no I/O, and total (never panics)**. Any uncoverable consumption → `uncovered_disposal` blocker; never a panic, never a negative remainder.
- **Blocker severity (§7.1):** *hard* (gate downstream tax computation for the affected lots/period) = `fmv_missing`, `uncovered_disposal`, `import_conflicts`, `decision_conflicts`, `unknown_basis_inbounds`, `unclassified`, `safe_harbor_unconservable`; *advisory* (ledger still usable) = `safe_harbor_timebar`, `unmatched_outflows`, `pre2025_method_note`.
- **TP8 self-transfer network fee — DEFAULT (c)** (`fee_sat` consumed at zero proceeds, non-taxable, full basis carries). **Config (b)** = taxable mini-disposition of fee-sats. **(c) is USER-MANDATED; (b) MUST NOT be the default and a review MUST NOT flip it.** Fee-sats are the sole conservation home (FR9); config (b) adds a *recognition* record, not a second conservation entry.
- **§7.4 2025 transition dates:** `as_of_date` snapshot = **2025-01-01** (fixed); the made-date (allocation effectiveness) = the allocation event's **`utc_timestamp`** on the §6.1 calendar-date basis; the app-observable **TY2025 unextended return due date = 2026-04-15** (the extended ≈2026-10-15 is not app-observable).
- **Licensing:** workspace `license = "MIT OR Unlicense"`; `edition = "2021"`; `rust-version = "1.74"`.
- **Validation gate ("green"):** `cargo test -p btctax-core` + `cargo clippy --all-targets -p btctax-core -- -D warnings` + `cargo fmt --check` all green; plus 0 Critical / 0 Important on review.

## File Structure
```
Cargo.toml                       # [workspace] root — ADD "crates/btctax-core" to members
crates/btctax-core/
  Cargo.toml                     # pinned deps (rust_decimal, time, serde, sha2, rusqlite, thiserror)
  src/lib.rs                     # pub API + CoreError + re-exports; wires modules
  src/conventions.rs             # Sat, Usd, MONEY_ROUNDING, money math, TaxDate, tax_date, HP helpers
  src/identity.rs                # Source/priority, SourceRef, EventId, Fingerprint (sha2), WalletId, LotId
  src/event.rs                   # EventPayload enum + sub-enums + LedgerEvent (serde)
  src/state.rs                   # LedgerState, Lot, Disposal/Removal/IncomeRecord/PendingTransfer, Blocker, Term, GiftZone
  src/price.rs                   # PriceProvider trait + fmv_of helper (+ a test stub)
  src/project/mod.rs             # project(): orchestrates pass 1 + pass 2 + finalize; ProjectionConfig
  src/project/resolve.rs         # PASS 1: decision resolution → Resolution{ timeline, transition, blockers }
  src/project/fold.rs            # PASS 2: canonical-order fold of the effective timeline → LedgerState
  src/project/pools.rs           # PoolSet (UniversalPool pre-2025 / PerWalletPool 2025+), FIFO consume, splits
  src/project/transition.rs      # §7.4 2025 transition: Path A reconstruct / Path B safe-harbor effectiveness
  src/persistence.rs             # THIN GLUE: events-table DDL, append_batch (conflict detect), load_all
  tests/determinism.rs           # permutation invariance (NFR4)
  tests/kat_tax.rs               # known-answer tax tests (§13): HP, income, gift/donation, dual-basis, transition, time-bar
  tests/properties.rs            # proptest: conservation, no-negative remainders, Σbasis
  tests/persistence.rs           # round-trip + idempotent re-import + ImportConflict
```

**Public interface this plan PRODUCES (consumed by Plan 3 `btctax-adapters` and Plan 4 `btctax-cli`):**
- `pub type Sat = i64;` · `pub type Usd = rust_decimal::Decimal;` · `pub type TaxDate = time::Date;`
- `pub enum CoreError` (`thiserror`): `Sqlite`, `Serde`, `Persistence`. (The *projection* is total and returns no `Result`; only `persistence` can error.)
- Identity: `pub enum Source`, `pub struct SourceRef(String)`, `pub enum EventId`, `pub struct Fingerprint(String)`, `pub enum WalletId`, `pub struct LotId { origin_event_id: EventId, split_sequence: u32 }`.
- Events: `pub struct LedgerEvent`, `pub enum EventPayload` + sub-enums (`Acquire`, `Income`, `Dispose`, `TransferOut`, `TransferIn`, `Unclassified`, `ImportConflict`, `TransferLink`, `ReclassifyOutflow`, `ClassifyInbound`, `ManualFmv`, `SafeHarborAllocation`, `SupersedeImport`, `RejectImport`, `VoidDecisionEvent`, `ClassifyRaw`), `pub enum FmvStatus`, `pub enum BasisSource`, `pub enum IncomeKind`, `pub enum DisposeKind`.
- Projection: `pub fn project(&[LedgerEvent], &dyn PriceProvider, &ProjectionConfig) -> LedgerState`, `pub struct ProjectionConfig`, `pub enum FeeTreatment`, `pub enum LotMethod`, `pub trait PriceProvider`, `pub struct LedgerState` (incl. `pub stats: FoldStats`), `pub struct FoldStats`, `pub struct Lot`, `pub struct Disposal`/`Removal`/`IncomeRecord`/`PendingTransfer`, `pub struct Blocker`, `pub enum BlockerKind`, `pub enum Severity`, `pub enum Term`, `pub enum GiftZone`. (`project -> LedgerState` is fixed; the internal `resolve` takes `(events, prices, config)` — I-2.)
- FR9 helper: `pub fn conservation_report(&LedgerState) -> ConservationReport`.
- Persistence: `pub fn persistence::init_schema(&Connection)`, `append_import_batch`, `append_decision`, `append_outflow_class_or_inbound`(decision helpers), `load_all(&Connection) -> Vec<LedgerEvent>`, `fingerprint(&EventPayload) -> Option<Fingerprint>`.

---

### Task 0: Workspace member + crate scaffold + money/time conventions (pins `rust_decimal` + `time`)

**Files:** Modify `Cargo.toml` (workspace members). Create `crates/btctax-core/Cargo.toml`, `crates/btctax-core/src/lib.rs`, `crates/btctax-core/src/conventions.rs`.

**Interfaces — Produces:** the pinned `rust_decimal`/`time` versions; `conventions::{Sat, Usd, TaxDate, MONEY_ROUNDING, round_cents, split_pro_rata, tax_date, one_year_after, is_long_term, TRANSITION_DATE, TY2025_RETURN_DUE}`; the `CoreError` enum.

- [ ] **Step 1: Add the workspace member.** Edit root `Cargo.toml` `members` to `["crates/btctax-store", "crates/btctax-core"]` (preserve existing `[workspace.package]`).

- [ ] **Step 2: `crates/btctax-core/Cargo.toml`**
```toml
[package]
name = "btctax-core"
version = "0.1.0"
edition.workspace = true
license.workspace = true

[dependencies]
# Exact decimal money (NFR5). serde-str = (de)serialize Decimal as a string for lossless JSON round-trips.
rust_decimal = { version = "1.36", default-features = false, features = ["serde-str", "std"] }
# UTC instants + fixed-offset calendar dates (§6.1). serde-well-known = RFC3339 (de)serializers.
time = { version = "0.3", features = ["serde-well-known", "macros", "parsing", "formatting"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sha2 = "0.10"               # canonical content fingerprint for conflict detection (§6.2/§9)
rusqlite = { version = "0.31", features = ["bundled"] }   # persistence glue ONLY; same pin as btctax-store
thiserror = "1"

[dev-dependencies]
rust_decimal_macros = "1.36"
proptest = "1"
```
*(Pin note, mirroring the store's R3 discipline: record the exact resolved `rust_decimal`/`time` versions in FOLLOWUPS after the first `cargo build`; if a cited symbol — e.g. `RoundingStrategy::MidpointNearestEven`, `Decimal::round_dp_with_strategy`, `OffsetDateTime::to_offset`, `Date::from_calendar_date` — differs, fix to the compiler before proceeding.)*

- [ ] **Step 3: Stub `src/lib.rs`**
```rust
//! btctax-core: domain model + pure deterministic event-sourced projection for the bitcoin_tax ledger.
//! The projection (`project`) is total and never panics (spec §7.1); only `persistence` performs I/O.
pub mod conventions;

pub use conventions::{Sat, TaxDate, Usd};

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("event (de)serialization: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("persistence: {0}")]
    Persistence(String),
}
```

- [ ] **Step 4: Failing tests in `src/conventions.rs`**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use time::macros::{date, datetime, offset};

    #[test]
    fn rounds_half_even_to_cents() {
        assert_eq!(round_cents(dec!(1.005)), dec!(1.00)); // ties-to-even: 0 is even
        assert_eq!(round_cents(dec!(1.015)), dec!(1.02)); // ties-to-even: 2 is even
        assert_eq!(round_cents(dec!(2.675)), dec!(2.68));
    }

    #[test]
    fn pro_rata_split_conserves_exactly() {
        // split 100.00 across takes that don't divide evenly: parts must sum to the whole.
        let (part, rest) = split_pro_rata(dec!(100.00), 333, 1000);
        assert_eq!(part + rest, dec!(100.00));
        assert_eq!(part, dec!(33.30)); // 100 * 333/1000 = 33.3 -> 33.30
    }

    #[test]
    fn tax_date_uses_original_tz_calendar_date() {
        // 2025-01-01T01:30:00Z is still 2024-12-31 in UTC-05:00 (day-granularity boundary, §6.1).
        let utc = datetime!(2025-01-01 01:30:00 UTC);
        assert_eq!(tax_date(utc, offset!(-05:00)), date!(2024 - 12 - 31));
        assert_eq!(tax_date(utc, offset!(+00:00)), date!(2025 - 01 - 01));
    }

    #[test]
    fn holding_period_boundary_tp4() {
        // Pub 544 example: acquire 2020-06-19; sell 2021-06-19 = ST (exactly 1yr); 2021-06-20 = LT.
        let acq = date!(2020 - 06 - 19);
        assert!(!is_long_term(acq, date!(2021 - 06 - 19)));
        assert!(is_long_term(acq, date!(2021 - 06 - 20)));
        assert!(!is_long_term(acq, acq)); // same-day = ST
    }

    #[test]
    fn leap_day_anniversary_falls_back_to_feb_28() {
        assert_eq!(one_year_after(date!(2020 - 02 - 29)), date!(2021 - 02 - 28));
    }
}
```

- [ ] **Step 5: Run → FAIL.** `cargo test -p btctax-core conventions`

- [ ] **Step 6: Implement `src/conventions.rs`**
```rust
//! Exact money/time conventions (NFR5, §6.1). No floats anywhere.
use rust_decimal::{Decimal, RoundingStrategy};
use time::{Date, OffsetDateTime, UtcOffset};

/// Bitcoin is integer satoshis (NFR5/§6.1). Signed so shortfall/overshoot math is total; quantities are non-negative.
pub type Sat = i64;
/// USD is exact decimal (NFR5).
pub type Usd = Decimal;
/// Tax dates are calendar dates in `original_tz`, day granularity (§6.1).
pub type TaxDate = Date;

/// `ROUND_HALF_EVEN` (§6.1).
pub const MONEY_ROUNDING: RoundingStrategy = RoundingStrategy::MidpointNearestEven;
/// Satoshis per whole BTC.
pub const SATS_PER_BTC: i64 = 100_000_000;
/// The per-wallet basis snapshot date (§7.4).
pub const TRANSITION_DATE: TaxDate = time::macros::date!(2025 - 01 - 01);
/// App-observable TY2025 unextended return due date (§7.4); the extended date is not app-observable.
pub const TY2025_RETURN_DUE: TaxDate = time::macros::date!(2026 - 04 - 15);

/// Round a USD value to the cent, ties-to-even.
pub fn round_cents(v: Usd) -> Usd {
    v.round_dp_with_strategy(2, MONEY_ROUNDING)
}

/// Split `total` so the `part_sat`/`whole_sat` portion is rounded to cents (ties-to-even) and the
/// remainder is `total - part`, conserving the sum EXACTLY (Σbasis invariant, §13/§6.3).
/// `whole_sat` is assumed > 0 by callers (consumption guards remaining_sat > 0).
///
/// §7.1 totality (M6): uses **checked** Decimal ops so it can never panic on overflow. The primary
/// `total * part / whole` form holds for all in-range money (USD magnitudes within Decimal's 96-bit
/// mantissa; `Sat` ≤ 21e6·1e8 = 2.1e15); on the (practically unreachable) overflow it falls back to the
/// magnitude-safe divide-first form. Both forms round to cents and conserve via remainder-takes-the-rest.
pub fn split_pro_rata(total: Usd, part_sat: Sat, whole_sat: Sat) -> (Usd, Usd) {
    if whole_sat <= 0 || part_sat <= 0 {
        return (Usd::ZERO, total);
    }
    if part_sat >= whole_sat {
        return (total, Usd::ZERO);
    }
    let (p, w) = (Usd::from(part_sat), Usd::from(whole_sat));
    let part = total
        .checked_mul(p)
        .and_then(|x| x.checked_div(w))
        .or_else(|| total.checked_div(w).and_then(|x| x.checked_mul(p)))
        .map(round_cents)
        .unwrap_or(Usd::ZERO); // unreachable for in-range money; never panics
    (part, total - part)
}

/// Calendar date in `original_tz` (§6.1).
pub fn tax_date(utc: OffsetDateTime, tz: UtcOffset) -> TaxDate {
    utc.to_offset(tz).date()
}

/// One calendar year after `d`; a Feb-29 anniversary in a non-leap year falls back to Feb 28 (documented convention).
pub fn one_year_after(d: TaxDate) -> TaxDate {
    let y = d.year() + 1;
    Date::from_calendar_date(y, d.month(), d.day())
        .unwrap_or_else(|_| Date::from_calendar_date(y, d.month(), 28).expect("Feb 28 is always valid"))
}

/// TP4: long-term iff the disposition date is strictly more than one year after acquisition.
pub fn is_long_term(acquired: TaxDate, disposed: TaxDate) -> bool {
    disposed > one_year_after(acquired)
}
```

- [ ] **Step 7: Run → PASS.** `cargo test -p btctax-core conventions`
- [ ] **Step 8: Gate + commit.** `cargo clippy --all-targets -p btctax-core -- -D warnings && cargo fmt --check`
```bash
git add Cargo.toml crates/btctax-core/Cargo.toml crates/btctax-core/src/lib.rs crates/btctax-core/src/conventions.rs FOLLOWUPS.md
git commit -m "feat(core): scaffold + money/time conventions (exact decimal, tax-date, HP)"
```

---

### Task 1: Identity & ordering — `Source`/priority, `SourceRef`, `EventId`, `Fingerprint`, `WalletId`, `LotId`

**Files:** Create `src/identity.rs`; Modify `src/lib.rs` (`pub mod identity;` + re-exports). Test in-module.

**Interfaces — Consumes:** `conventions`. **Produces:** `Source` (`+ priority(): u8`, `+ tag(): &str`), `SourceRef`, `EventId` (`Import|Conflict|Decision` + `canonical(): String`), `Fingerprint`, `WalletId`, `LotId`, and `canonical_import_fingerprint(payload-fields) -> Fingerprint` building block (the public `fingerprint` wrapper lands in Task 3 once `EventPayload` exists).

- [ ] **Step 1: Failing tests**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_priority_is_swan_first_river_last() {
        let mut v = [Source::River, Source::Coinbase, Source::Swan, Source::Gemini];
        v.sort_by_key(|s| s.priority());
        assert_eq!(v, [Source::Swan, Source::Coinbase, Source::Gemini, Source::River]);
    }

    #[test]
    fn import_event_id_is_stable_function_of_source_and_ref() {
        let a = EventId::import(Source::Coinbase, SourceRef::new("ID-1"));
        let b = EventId::import(Source::Coinbase, SourceRef::new("ID-1"));
        assert_eq!(a, b);
        assert_eq!(a.canonical(), "import|coinbase|ID-1");
    }

    #[test]
    fn conflict_event_id_is_distinct_from_its_target() {
        let target = EventId::import(Source::Gemini, SourceRef::new("T1"));
        let fp = Fingerprint::of_bytes(b"new-content");
        let c1 = EventId::conflict(Source::Gemini, SourceRef::new("T1"), &fp);
        let c2 = EventId::conflict(Source::Gemini, SourceRef::new("T1"), &fp);
        assert_ne!(EventId::import(Source::Gemini, SourceRef::new("T1")), c1);
        assert_eq!(c1, c2); // re-importing the identical changed row reproduces the same conflict id (§6.2)
        let _ = target;
    }

    #[test]
    fn decision_event_id_is_function_of_seq() {
        assert_eq!(EventId::decision(7).canonical(), "decision|7");
        assert_ne!(EventId::decision(7), EventId::decision(8));
    }

    #[test]
    fn lot_id_is_origin_plus_split_sequence() {
        let origin = EventId::import(Source::Swan, SourceRef::new("TX9"));
        let l0 = LotId { origin_event_id: origin.clone(), split_sequence: 0 };
        let l1 = LotId { origin_event_id: origin, split_sequence: 1 };
        assert_ne!(l0, l1);
        assert!(l0 < l1); // deterministic ordering for stable output
    }
}
```

- [ ] **Step 2: Run → FAIL.** `cargo test -p btctax-core identity`

- [ ] **Step 3: Implement `src/identity.rs`**
```rust
//! Stable identity & canonical ordering (§6.2). EventId is a STRUCTURED (injective) function of its
//! components — no hashing needed for identity; only the content *fingerprint* (conflict detection) hashes.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// The four supported venues. Fixed source priority for same-instant fold ties (§6.2): Swan>Coinbase>Gemini>River.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Source {
    Swan,
    Coinbase,
    Gemini,
    River,
}
impl Source {
    /// Lower = folds first at the same `utc_timestamp` (Swan=0 … River=3).
    pub fn priority(self) -> u8 {
        match self {
            Source::Swan => 0,
            Source::Coinbase => 1,
            Source::Gemini => 2,
            Source::River => 3,
        }
    }
    pub fn tag(self) -> &'static str {
        match self {
            Source::Swan => "swan",
            Source::Coinbase => "coinbase",
            Source::Gemini => "gemini",
            Source::River => "river",
        }
    }
}

/// Stable real-world-row identity scoped by (source, direction) (§6.2). Opaque string assigned by adapters.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SourceRef(pub String);
impl SourceRef {
    pub fn new(s: impl Into<String>) -> Self {
        SourceRef(s.into())
    }
}

/// SHA-256 hex of canonical content; used ONLY for conflict detection (§6.2/§9).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Fingerprint(pub String);
impl Fingerprint {
    pub fn of_bytes(bytes: &[u8]) -> Self {
        let mut h = Sha256::new();
        h.update(bytes);
        Fingerprint(format!("{:x}", h.finalize()))
    }
}

/// Universal reference target (§6.2). Equality is what matters; we also derive Ord/Hash for map keys + stable output.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum EventId {
    /// Imported events: f(source, source_ref) — survives cosmetic re-exports/corrections.
    Import { source: Source, source_ref: SourceRef },
    /// System ImportConflict: f("conflict", source, source_ref, new_fingerprint) — distinct from its target.
    Conflict { source: Source, source_ref: SourceRef, fingerprint: Fingerprint },
    /// App-generated decisions: f("decision", decision_seq).
    Decision { seq: u64 },
}
impl EventId {
    pub fn import(source: Source, source_ref: SourceRef) -> Self {
        EventId::Import { source, source_ref }
    }
    pub fn conflict(source: Source, source_ref: SourceRef, fingerprint: &Fingerprint) -> Self {
        EventId::Conflict { source, source_ref, fingerprint: fingerprint.clone() }
    }
    pub fn decision(seq: u64) -> Self {
        EventId::Decision { seq }
    }
    /// Stable string form for the persistence `event_id` column (components are also stored separately).
    pub fn canonical(&self) -> String {
        match self {
            EventId::Import { source, source_ref } => format!("import|{}|{}", source.tag(), source_ref.0),
            EventId::Conflict { source, source_ref, fingerprint } => {
                format!("conflict|{}|{}|{}", source.tag(), source_ref.0, fingerprint.0)
            }
            EventId::Decision { seq } => format!("decision|{seq}"),
        }
    }
}

/// Basis pool identity (§6.3).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum WalletId {
    Exchange { provider: String, account: String },
    SelfCustody { label: String },
}

/// Lot identity (§6.2): origin event + a per-origin split sequence, assigned deterministically as lots split.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct LotId {
    pub origin_event_id: EventId,
    pub split_sequence: u32,
}
```

- [ ] **Step 4: Run → PASS.** `cargo test -p btctax-core identity`
- [ ] **Step 5: Wire + gate + commit.** Add `pub mod identity;` and `pub use identity::{EventId, Fingerprint, LotId, Source, SourceRef, WalletId};` to `lib.rs`.
```bash
cargo clippy --all-targets -p btctax-core -- -D warnings && cargo fmt --check
git commit -am "feat(core): identity & ordering (Source priority, EventId, Fingerprint, WalletId, LotId)"
```

---

### Task 2: Event taxonomy — `EventPayload` + sub-enums + `LedgerEvent` (serde)

**Files:** Create `src/event.rs`; Modify `src/lib.rs`. Test in-module.

**Interfaces — Consumes:** `identity`, `conventions`. **Produces:** `EventPayload` (every spec §6.4 variant), the sub-enums (`FmvStatus`, `BasisSource`, `IncomeKind`, `DisposeKind`, `OutflowClass`, `InboundClass`, `AllocMethod`, `AllocLot`, `TransferTarget`), and `LedgerEvent { id, utc_timestamp, original_tz, wallet, payload }` with serde round-trip.

- [ ] **Step 1: Failing tests**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::{EventId, Source, SourceRef};
    use rust_decimal_macros::dec;
    use time::macros::{datetime, offset};

    fn sample(payload: EventPayload) -> LedgerEvent {
        LedgerEvent {
            id: EventId::import(Source::Coinbase, SourceRef::new("X")),
            utc_timestamp: datetime!(2025-03-01 12:00:00 UTC),
            original_tz: offset!(-05:00),
            wallet: Some(crate::identity::WalletId::Exchange {
                provider: "coinbase".into(),
                account: "main".into(),
            }),
            payload,
        }
    }

    #[test]
    fn every_variant_serde_round_trips() {
        let payloads = vec![
            EventPayload::Acquire(Acquire { sat: 100_000, usd_cost: dec!(60.00), fee_usd: dec!(1.00), basis_source: BasisSource::ExchangeProvided }),
            EventPayload::Income(Income { sat: 50_000, usd_fmv: Some(dec!(30.00)), fmv_status: FmvStatus::PriceDataset, kind: IncomeKind::Interest, business: false }),
            EventPayload::Dispose(Dispose { sat: 25_000, usd_proceeds: dec!(40.00), fee_usd: dec!(0.50), kind: DisposeKind::Sell }),
            EventPayload::TransferOut(TransferOut { sat: 10_000, fee_sat: Some(150), dest_addr: Some("bc1q…".into()), txid: Some("ab12".into()) }),
            EventPayload::TransferIn(TransferIn { sat: 10_000, src_addr: None, txid: Some("ab12".into()) }),
            EventPayload::Unclassified(Unclassified { raw: "weird row".into() }),
        ];
        for p in payloads {
            let ev = sample(p);
            let json = serde_json::to_string(&ev).unwrap();
            let back: LedgerEvent = serde_json::from_str(&json).unwrap();
            assert_eq!(ev, back);
        }
    }

    #[test]
    fn decimal_is_serialized_losslessly_as_string() {
        let ev = sample(EventPayload::Acquire(Acquire {
            sat: 1, usd_cost: dec!(0.10), fee_usd: dec!(0), basis_source: BasisSource::ComputedFromCost,
        }));
        let json = serde_json::to_string(&ev).unwrap();
        assert!(json.contains("\"0.10\"")); // serde-str: exact, not a 0.1 float
    }
}
```

- [ ] **Step 2: Run → FAIL.** `cargo test -p btctax-core event`

- [ ] **Step 3: Implement `src/event.rs`** (every §6.4 variant; complete)
```rust
//! The canonical event taxonomy (§6.4). One `EventPayload` enum; imported, system, and decision variants.
use crate::conventions::{Sat, TaxDate, Usd};
use crate::identity::{EventId, Fingerprint, WalletId};
use serde::{Deserialize, Serialize};
use time::{OffsetDateTime, UtcOffset};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FmvStatus {
    ExchangeProvided,
    PriceDataset,
    ManualEntry,
    Missing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BasisSource {
    ExchangeProvided,
    ComputedFromCost,
    FmvAtIncome,
    CarriedFromTransfer,
    GiftCarryover,
    GiftFmvFallback,
    SafeHarborAllocated,
    ReconstructedPerWallet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IncomeKind {
    Mining,
    Staking,
    Interest,
    Airdrop,
    Reward,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisposeKind {
    Sell,
    Spend,
}

// ---- imported payloads ----
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Acquire {
    pub sat: Sat,
    pub usd_cost: Usd,
    pub fee_usd: Usd,
    pub basis_source: BasisSource,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Income {
    pub sat: Sat,
    pub usd_fmv: Option<Usd>,
    pub fmv_status: FmvStatus,
    pub kind: IncomeKind,
    pub business: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dispose {
    pub sat: Sat,
    pub usd_proceeds: Usd, // GROSS; fee_usd reduces proceeds (TP2)
    pub fee_usd: Usd,
    pub kind: DisposeKind,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferOut {
    pub sat: Sat,
    pub fee_sat: Option<Sat>,
    pub dest_addr: Option<String>,
    pub txid: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferIn {
    pub sat: Sat,
    pub src_addr: Option<String>,
    pub txid: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Unclassified {
    pub raw: String,
}

// ---- system payload ----
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportConflict {
    pub target: EventId,
    pub new_payload: Box<EventPayload>,
    pub new_fingerprint: Fingerprint,
}

// ---- decision payloads (§6.4) ----
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransferTarget {
    InEvent(EventId),
    Wallet(WalletId),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferLink {
    pub out_event: EventId,
    pub in_event_or_wallet: TransferTarget,
}
/// What a TransferOut is reclassified to (the proceeds/FMV ride in `principal_proceeds_or_fmv`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutflowClass {
    Dispose { kind: DisposeKind },
    GiftOut,
    Donate { appraisal_required: bool },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReclassifyOutflow {
    pub transfer_out_event: EventId,
    pub as_: OutflowClass,
    pub principal_proceeds_or_fmv: Usd,
    pub fee_usd: Option<Usd>, // TP8: fee handling for a reclassified outflow
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InboundClass {
    Income { kind: IncomeKind, fmv: Option<Usd>, business: bool },
    GiftReceived { donor_basis: Option<Usd>, donor_acquired_at: Option<TaxDate>, fmv_at_gift: Usd },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassifyInbound {
    pub transfer_in_event: EventId,
    pub as_: InboundClass,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManualFmv {
    pub event: EventId,
    pub usd_fmv: Usd,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AllocMethod {
    ActualPosition,
    ProRata,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AllocLot {
    pub wallet: WalletId,
    pub sat: Sat,
    pub usd_basis: Usd,
    pub acquired_at: TaxDate,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SafeHarborAllocation {
    pub lots: Vec<AllocLot>,
    pub as_of_date: TaxDate, // fixed 2025-01-01 snapshot
    pub method: AllocMethod,
    pub timely_allocation_attested: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupersedeImport {
    pub conflict_event: EventId,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RejectImport {
    pub conflict_event: EventId,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoidDecisionEvent {
    pub target_event_id: EventId,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassifyRaw {
    pub target: EventId,
    pub as_: Box<EventPayload>, // the supplied imported payload
}

/// The single payload sum-type carried by every `LedgerEvent` (§6.3/§6.4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventPayload {
    // imported
    Acquire(Acquire),
    Income(Income),
    Dispose(Dispose),
    TransferOut(TransferOut),
    TransferIn(TransferIn),
    Unclassified(Unclassified),
    // system
    ImportConflict(ImportConflict),
    // decisions
    TransferLink(TransferLink),
    ReclassifyOutflow(ReclassifyOutflow),
    ClassifyInbound(ClassifyInbound),
    ManualFmv(ManualFmv),
    SafeHarborAllocation(SafeHarborAllocation),
    SupersedeImport(SupersedeImport),
    RejectImport(RejectImport),
    VoidDecisionEvent(VoidDecisionEvent),
    ClassifyRaw(ClassifyRaw),
}

impl EventPayload {
    /// True for the six adapter-emitted imported payloads (the only ones folded as primary movements).
    pub fn is_imported(&self) -> bool {
        matches!(
            self,
            EventPayload::Acquire(_)
                | EventPayload::Income(_)
                | EventPayload::Dispose(_)
                | EventPayload::TransferOut(_)
                | EventPayload::TransferIn(_)
                | EventPayload::Unclassified(_)
        )
    }
}

/// An immutable ledger event (§6.3). `utc_timestamp` is the UTC instant (decisions: creation time);
/// `original_tz` drives the §6.1 tax-date. For decisions, `id` is `EventId::Decision { seq }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerEvent {
    pub id: EventId,
    #[serde(with = "time::serde::rfc3339")]
    pub utc_timestamp: OffsetDateTime,
    pub original_tz: UtcOffset,
    pub wallet: Option<WalletId>,
    pub payload: EventPayload,
}
```

- [ ] **Step 4: Run → PASS.** `cargo test -p btctax-core event`
- [ ] **Step 5: Wire + gate + commit.** Add `pub mod event;` + `pub use event::*;` to `lib.rs`.
```bash
cargo clippy --all-targets -p btctax-core -- -D warnings && cargo fmt --check
git commit -am "feat(core): full event taxonomy (§6.4) with serde round-trip"
```

---

### Task 3: Persistence glue — events table, fingerprint conflict detection, append/load (FR1)

**Files:** Create `src/persistence.rs`; Modify `src/lib.rs`. Test `tests/persistence.rs`.

**Interfaces — Consumes:** `event`, `identity`, `CoreError`; a borrowed `&rusqlite::Connection`. **Produces:**
- `pub fn fingerprint(p: &EventPayload) -> Option<Fingerprint>` — canonical content fingerprint for the **imported** payloads (None otherwise); normalizes `Decimal` scale + trims strings so cosmetic re-exports are idempotent (§13).
- `pub fn init_schema(conn) -> Result<(), CoreError>`
- `pub fn append_import_batch(conn, events: &[LedgerEvent]) -> Result<ImportReport, CoreError>` — **atomic** (single transaction, FR1); idempotent on identical rows; a changed row (same `source_ref`, different fingerprint) appends ONE `ImportConflict`; re-importing the identical changed row is a no-op.
- `pub fn append_decision(conn, payload, utc, tz, wallet) -> Result<EventId, CoreError>` — allocates the next `decision_seq`, mints `EventId::Decision`, persists.
- `pub fn load_all(conn) -> Result<Vec<LedgerEvent>, CoreError>` — loads the full set (order is irrelevant; the projection re-sorts canonically — NFR4).

- [ ] **Step 1: Failing tests in `tests/persistence.rs`**
```rust
use btctax_core::event::*;
use btctax_core::identity::*;
use btctax_core::persistence;
use rust_decimal_macros::dec;
use time::macros::{datetime, offset};

fn acq(source_ref: &str, cost: rust_decimal::Decimal) -> LedgerEvent {
    LedgerEvent {
        id: EventId::import(Source::Coinbase, SourceRef::new(source_ref)),
        utc_timestamp: datetime!(2025-03-01 12:00:00 UTC),
        original_tz: offset!(+00:00),
        wallet: Some(WalletId::Exchange { provider: "coinbase".into(), account: "main".into() }),
        payload: EventPayload::Acquire(Acquire { sat: 100_000, usd_cost: cost, fee_usd: dec!(1.00), basis_source: BasisSource::ExchangeProvided }),
    }
}

#[test]
fn round_trips_the_event_set() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    persistence::init_schema(&conn).unwrap();
    persistence::append_import_batch(&conn, &[acq("A", dec!(60.00)), acq("B", dec!(61.00))]).unwrap();
    let loaded = persistence::load_all(&conn).unwrap();
    assert_eq!(loaded.len(), 2);
}

#[test]
fn re_import_identical_is_idempotent() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    persistence::init_schema(&conn).unwrap();
    persistence::append_import_batch(&conn, &[acq("A", dec!(60.00))]).unwrap();
    // cosmetic variation: trailing-zero scale must NOT create a dup (same fingerprint).
    persistence::append_import_batch(&conn, &[acq("A", dec!(60.0))]).unwrap();
    assert_eq!(persistence::load_all(&conn).unwrap().len(), 1);
}

#[test]
fn changed_row_appends_exactly_one_conflict_idempotently() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    persistence::init_schema(&conn).unwrap();
    persistence::append_import_batch(&conn, &[acq("A", dec!(60.00))]).unwrap();
    persistence::append_import_batch(&conn, &[acq("A", dec!(99.00))]).unwrap(); // changed
    persistence::append_import_batch(&conn, &[acq("A", dec!(99.00))]).unwrap(); // same change again
    let loaded = persistence::load_all(&conn).unwrap();
    let conflicts = loaded.iter().filter(|e| matches!(e.payload, EventPayload::ImportConflict(_))).count();
    assert_eq!(conflicts, 1); // one conflict total; the original Acquire is untouched
    assert_eq!(loaded.len(), 2);
}

#[test]
fn decisions_get_monotonic_seq_and_decision_event_ids() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    persistence::init_schema(&conn).unwrap();
    let id1 = persistence::append_decision(&conn,
        EventPayload::RejectImport(RejectImport { conflict_event: EventId::decision(0) }),
        datetime!(2026-01-01 00:00:00 UTC), offset!(+00:00), None).unwrap();
    let id2 = persistence::append_decision(&conn,
        EventPayload::RejectImport(RejectImport { conflict_event: EventId::decision(0) }),
        datetime!(2026-01-02 00:00:00 UTC), offset!(+00:00), None).unwrap();
    assert_eq!(id1, EventId::decision(1));
    assert_eq!(id2, EventId::decision(2));
}
```

- [ ] **Step 2: Run → FAIL.** `cargo test -p btctax-core --test persistence`

- [ ] **Step 3: Implement `src/persistence.rs`** (complete)
```rust
//! THE ONLY I/O in btctax-core. Reads/appends canonical event rows over a borrowed rusqlite handle
//! (the live in-memory DB from btctax-store::Vault::conn(), wired by the CLI). Owns the events-table DDL,
//! event (de)serialization, decision_seq allocation, and FR1 fingerprint-based conflict detection.
use crate::event::*;
use crate::identity::{EventId, Fingerprint, Source, SourceRef, WalletId};
use crate::CoreError;
use rusqlite::Connection;
use time::{OffsetDateTime, UtcOffset};

const KIND_IMPORT: &str = "import";
const KIND_CONFLICT: &str = "conflict";
const KIND_DECISION: &str = "decision";

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ImportReport {
    pub appended: usize,
    pub duplicates: usize,
    pub conflicts: usize,
}

/// Canonical content fingerprint for the six imported payloads (None for system/decision payloads).
/// Normalizes Decimal scale (`.normalize()`) and trims string fields so whitespace/scale/CRLF
/// re-exports are idempotent (§13). Field order is FIXED (§6.2).
pub fn fingerprint(p: &EventPayload) -> Option<Fingerprint> {
    let mut b: Vec<u8> = Vec::new();
    fn d(b: &mut Vec<u8>, v: &crate::Usd) {
        b.extend_from_slice(v.normalize().to_string().as_bytes());
        b.push(0x1e);
    }
    fn od(b: &mut Vec<u8>, v: &Option<crate::Usd>) {
        match v {
            Some(x) => b.extend_from_slice(x.normalize().to_string().as_bytes()),
            None => b.extend_from_slice(b"\x00none"),
        }
        b.push(0x1e);
    }
    fn s(b: &mut Vec<u8>, v: &str) {
        b.extend_from_slice(v.trim().as_bytes());
        b.push(0x1e);
    }
    fn os(b: &mut Vec<u8>, v: &Option<String>) {
        match v {
            Some(x) => b.extend_from_slice(x.trim().as_bytes()),
            None => b.extend_from_slice(b"\x00none"),
        }
        b.push(0x1e);
    }
    fn i(b: &mut Vec<u8>, v: i64) {
        b.extend_from_slice(v.to_string().as_bytes());
        b.push(0x1e);
    }
    fn oi(b: &mut Vec<u8>, v: Option<i64>) {
        i(b, v.unwrap_or(i64::MIN));
    }
    match p {
        EventPayload::Acquire(a) => {
            b.extend_from_slice(b"acquire\x1e");
            i(&mut b, a.sat);
            d(&mut b, &a.usd_cost);
            d(&mut b, &a.fee_usd);
            b.extend_from_slice(format!("{:?}", a.basis_source).as_bytes());
        }
        EventPayload::Income(x) => {
            b.extend_from_slice(b"income\x1e");
            i(&mut b, x.sat);
            od(&mut b, &x.usd_fmv);
            b.extend_from_slice(format!("{:?}/{:?}/{}", x.fmv_status, x.kind, x.business).as_bytes());
        }
        EventPayload::Dispose(x) => {
            b.extend_from_slice(b"dispose\x1e");
            i(&mut b, x.sat);
            d(&mut b, &x.usd_proceeds);
            d(&mut b, &x.fee_usd);
            b.extend_from_slice(format!("{:?}", x.kind).as_bytes());
        }
        EventPayload::TransferOut(x) => {
            b.extend_from_slice(b"transfer_out\x1e");
            i(&mut b, x.sat);
            oi(&mut b, x.fee_sat);
            os(&mut b, &x.dest_addr);
            os(&mut b, &x.txid);
        }
        EventPayload::TransferIn(x) => {
            b.extend_from_slice(b"transfer_in\x1e");
            i(&mut b, x.sat);
            os(&mut b, &x.src_addr);
            os(&mut b, &x.txid);
        }
        EventPayload::Unclassified(x) => {
            b.extend_from_slice(b"unclassified\x1e");
            s(&mut b, &x.raw);
        }
        _ => return None,
    }
    Some(Fingerprint::of_bytes(&b))
}

pub fn init_schema(conn: &Connection) -> Result<(), CoreError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS events (
            ordinal       INTEGER PRIMARY KEY AUTOINCREMENT, -- insertion order ONLY; projection ignores it (NFR4)
            event_id      TEXT NOT NULL UNIQUE,
            kind          TEXT NOT NULL,
            source        TEXT,
            source_ref    TEXT,
            decision_seq  INTEGER,
            utc_timestamp TEXT NOT NULL,
            tz_offset_sec INTEGER NOT NULL,
            wallet_json   TEXT,
            payload_json  TEXT NOT NULL,
            fingerprint   TEXT
        );
        CREATE INDEX IF NOT EXISTS events_srcref ON events(source, source_ref);",
    )?;
    Ok(())
}

fn source_tag(s: &str) -> Option<Source> {
    match s {
        "swan" => Some(Source::Swan),
        "coinbase" => Some(Source::Coinbase),
        "gemini" => Some(Source::Gemini),
        "river" => Some(Source::River),
        _ => None,
    }
}

fn insert(conn: &Connection, ev: &LedgerEvent, kind: &str, fp: Option<&Fingerprint>) -> Result<(), CoreError> {
    let (source, source_ref, seq) = match &ev.id {
        EventId::Import { source, source_ref } => (Some(source.tag().to_string()), Some(source_ref.0.clone()), None),
        EventId::Conflict { source, source_ref, .. } => (Some(source.tag().to_string()), Some(source_ref.0.clone()), None),
        EventId::Decision { seq } => (None, None, Some(*seq as i64)),
    };
    conn.execute(
        "INSERT INTO events
          (event_id, kind, source, source_ref, decision_seq, utc_timestamp, tz_offset_sec, wallet_json, payload_json, fingerprint)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        rusqlite::params![
            ev.id.canonical(),
            kind,
            source,
            source_ref,
            seq,
            ev.utc_timestamp.format(&time::format_description::well_known::Rfc3339).map_err(|e| CoreError::Persistence(e.to_string()))?,
            ev.original_tz.whole_seconds(),
            ev.wallet.as_ref().map(serde_json::to_string).transpose()?,
            serde_json::to_string(&ev.payload)?,
            fp.map(|f| f.0.clone()),
        ],
    )?;
    Ok(())
}

pub fn append_import_batch(conn: &Connection, events: &[LedgerEvent]) -> Result<ImportReport, CoreError> {
    let tx = conn.unchecked_transaction()?; // ATOMIC batch (FR1); &Connection => unchecked_transaction
    let mut report = ImportReport::default();
    for ev in events {
        let fp = fingerprint(&ev.payload)
            .ok_or_else(|| CoreError::Persistence("append_import_batch given a non-imported payload".into()))?;
        let (source, source_ref) = match &ev.id {
            EventId::Import { source, source_ref } => (*source, source_ref.clone()),
            _ => return Err(CoreError::Persistence("imported events must carry EventId::Import".into())),
        };
        // existing event with same (source, source_ref)?
        let existing_fp: Option<String> = tx
            .query_row(
                "SELECT fingerprint FROM events WHERE kind=?1 AND source=?2 AND source_ref=?3 LIMIT 1",
                rusqlite::params![KIND_IMPORT, source.tag(), source_ref.0],
                |r| r.get(0),
            )
            .ok();
        match existing_fp {
            None => {
                insert(&tx, ev, KIND_IMPORT, Some(&fp))?;
                report.appended += 1;
            }
            Some(prev) if prev == fp.0 => {
                report.duplicates += 1; // idempotent no-op
            }
            Some(_) => {
                // changed content -> ONE ImportConflict, distinct id (idempotent on the identical change)
                let conflict_id = EventId::conflict(source, source_ref.clone(), &fp);
                let already: i64 = tx.query_row(
                    "SELECT COUNT(*) FROM events WHERE event_id=?1",
                    rusqlite::params![conflict_id.canonical()],
                    |r| r.get(0),
                )?;
                if already == 0 {
                    let conflict = LedgerEvent {
                        id: conflict_id,
                        utc_timestamp: ev.utc_timestamp,
                        original_tz: ev.original_tz,
                        wallet: ev.wallet.clone(),
                        payload: EventPayload::ImportConflict(ImportConflict {
                            target: EventId::import(source, source_ref),
                            new_payload: Box::new(ev.payload.clone()),
                            new_fingerprint: fp.clone(),
                        }),
                    };
                    insert(&tx, &conflict, KIND_CONFLICT, Some(&fp))?;
                    report.conflicts += 1;
                } else {
                    report.duplicates += 1;
                }
            }
        }
    }
    tx.commit()?;
    Ok(report)
}

pub fn append_decision(
    conn: &Connection,
    payload: EventPayload,
    utc_timestamp: OffsetDateTime,
    original_tz: UtcOffset,
    wallet: Option<WalletId>,
) -> Result<EventId, CoreError> {
    let tx = conn.unchecked_transaction()?;
    let next: i64 = tx.query_row("SELECT COALESCE(MAX(decision_seq),0)+1 FROM events WHERE kind=?1", [KIND_DECISION], |r| r.get(0))?;
    let id = EventId::decision(next as u64);
    let ev = LedgerEvent { id: id.clone(), utc_timestamp, original_tz, wallet, payload };
    insert(&tx, &ev, KIND_DECISION, None)?;
    tx.commit()?;
    Ok(id)
}

pub fn load_all(conn: &Connection) -> Result<Vec<LedgerEvent>, CoreError> {
    // SELECT the persisted identity columns and rebuild `EventId` DIRECTLY from them (no re-derivation,
    // no ambiguity — M5). Order is irrelevant; the projection re-sorts canonically (NFR4).
    let mut stmt = conn.prepare(
        "SELECT kind, source, source_ref, decision_seq, utc_timestamp, tz_offset_sec, wallet_json, payload_json FROM events",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,         // kind
            r.get::<_, Option<String>>(1)?, // source
            r.get::<_, Option<String>>(2)?, // source_ref
            r.get::<_, Option<i64>>(3)?,    // decision_seq
            r.get::<_, String>(4)?,         // utc_timestamp
            r.get::<_, i32>(5)?,            // tz_offset_sec
            r.get::<_, Option<String>>(6)?, // wallet_json
            r.get::<_, String>(7)?,         // payload_json
        ))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (kind, source, source_ref, decision_seq, ts, off, wallet_json, payload_json) = row?;
        let utc_timestamp = OffsetDateTime::parse(&ts, &time::format_description::well_known::Rfc3339)
            .map_err(|e| CoreError::Persistence(e.to_string()))?;
        let original_tz = UtcOffset::from_whole_seconds(off).map_err(|e| CoreError::Persistence(e.to_string()))?;
        let payload: EventPayload = serde_json::from_str(&payload_json)?;
        let wallet: Option<WalletId> = wallet_json.map(|w| serde_json::from_str(&w)).transpose()?;
        let bad = |m: &str| CoreError::Persistence(format!("corrupt identity row: {m}"));
        let id = match kind.as_str() {
            KIND_DECISION => EventId::decision(decision_seq.ok_or_else(|| bad("decision without seq"))? as u64),
            KIND_IMPORT => {
                let src = source_tag(&source.ok_or_else(|| bad("import without source"))?).ok_or_else(|| bad("unknown source"))?;
                EventId::import(src, SourceRef::new(source_ref.ok_or_else(|| bad("import without source_ref"))?))
            }
            KIND_CONFLICT => {
                let src = source_tag(&source.ok_or_else(|| bad("conflict without source"))?).ok_or_else(|| bad("unknown source"))?;
                let sref = SourceRef::new(source_ref.ok_or_else(|| bad("conflict without source_ref"))?);
                // The conflict's fingerprint is part of its identity; recover it from the stored payload.
                let fp = match &payload {
                    EventPayload::ImportConflict(c) => c.new_fingerprint.clone(),
                    _ => return Err(bad("conflict row without ImportConflict payload")),
                };
                EventId::conflict(src, sref, &fp)
            }
            other => return Err(bad(other)),
        };
        out.push(LedgerEvent { id, utc_timestamp, original_tz, wallet, payload });
    }
    Ok(out)
}
```
*(No `id_for` stub — `load_all` rebuilds each `EventId` from the four persisted identity columns inline, so the "implement" block above is already shippable as-is (M5).)*

- [ ] **Step 4: Run → PASS.** `cargo test -p btctax-core --test persistence`
- [ ] **Step 5: Wire + gate + commit.** Add `pub mod persistence;` to `lib.rs`.
```bash
cargo clippy --all-targets -p btctax-core -- -D warnings && cargo fmt --check
git commit -am "feat(core): persistence glue — events table, fingerprint conflict detection, append/load (FR1)"
```

---

### Task 4: Projection contract + canonical ordering + `Acquire` fold + determinism harness

**Files:** Create `src/state.rs`, `src/price.rs`, `src/project/mod.rs`, `src/project/resolve.rs`, `src/project/fold.rs`, `src/project/pools.rs`; Modify `src/lib.rs`. Test `tests/determinism.rs`.

**Interfaces — Consumes:** all prior. **Produces:** `LedgerState`, `Lot`, `Blocker`/`BlockerKind`/`Severity`, `Term`, `GiftZone`, `Disposal`/`Removal`/`IncomeRecord`/`PendingTransfer` (shapes; populated in later tasks), `PriceProvider` + `fmv_of`, `ProjectionConfig`/`FeeTreatment`/`LotMethod`, and `pub fn project(...)`. This task implements only the `Acquire` fold + the two-pass scaffolding (pass 1 = pass-through of imported events in this task; decisions handled in Task 7) + the canonical sort.

- [ ] **Step 1: `src/state.rs`** (output types — complete; later tasks populate the vectors)
```rust
use crate::conventions::{Sat, TaxDate, Usd};
use crate::event::{BasisSource, DisposeKind, IncomeKind};
use crate::identity::{EventId, LotId, WalletId};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Term {
    ShortTerm,
    LongTerm,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GiftZone {
    Gain,
    Loss,
    NoGainNoLoss,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Hard,
    Advisory,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BlockerKind {
    FmvMissing,
    UncoveredDisposal,
    ImportConflict,
    DecisionConflict,
    UnknownBasisInbound,
    Unclassified,
    SafeHarborUnconservable,
    SafeHarborTimebar,
    UnmatchedOutflows,
    Pre2025MethodNote,
}
impl BlockerKind {
    pub fn severity(self) -> Severity {
        use BlockerKind::*;
        match self {
            FmvMissing | UncoveredDisposal | ImportConflict | DecisionConflict | UnknownBasisInbound
            | Unclassified | SafeHarborUnconservable => Severity::Hard,
            SafeHarborTimebar | UnmatchedOutflows | Pre2025MethodNote => Severity::Advisory,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Blocker {
    pub kind: BlockerKind,
    pub event: Option<EventId>,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lot {
    pub lot_id: LotId,
    pub wallet: WalletId,
    pub acquired_at: TaxDate, // gift loss-zone HP start = this (gift date); see donor_acquired_at for tacking
    pub original_sat: Sat,
    pub remaining_sat: Sat,
    pub usd_basis: Usd, // gain basis
    pub basis_source: BasisSource,
    pub dual_loss_basis: Option<Usd>,    // received gifts (TP11): loss basis when FMV-at-gift < donor basis
    pub donor_acquired_at: Option<TaxDate>, // tacking (TP11/§1223(2)); gain/no-dual HP start
    pub basis_pending: bool, // FMV-missing income / unknown-basis gift: gain is gated until resolved
}
impl Lot {
    /// HP start used on the gain side / no-dual case (tacks donor period when present).
    pub fn gain_hp_start(&self) -> TaxDate {
        self.donor_acquired_at.unwrap_or(self.acquired_at)
    }
    /// HP start used on the loss side of a dual-basis gift (the gift/received date).
    pub fn loss_hp_start(&self) -> TaxDate {
        self.acquired_at
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisposalLeg {
    pub lot_id: LotId,
    pub sat: Sat,
    pub proceeds: Usd, // allocated net proceeds (gross − disposition fee, TP2)
    pub basis: Usd,    // tax-reported basis (zone-dependent for dual-basis gifts)
    pub gain: Usd,
    pub term: Term,
    pub basis_source: BasisSource,
    pub gift_zone: Option<GiftZone>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Disposal {
    pub event: EventId,
    pub kind: DisposeKind,
    pub disposed_at: TaxDate,
    pub legs: Vec<DisposalLeg>,
    /// TP8 config-(b) fee-sat mini-disposition: a RECOGNITION record only — excluded from FR9 Σdisposed
    /// (its sats live in Σ on-chain-fee-sats; no second conservation entry).
    pub fee_mini_disposition: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemovalKind {
    Gift,
    Donation,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemovalLeg {
    pub lot_id: LotId,
    pub sat: Sat,
    pub basis: Usd,
    pub fmv_at_transfer: Usd,
    pub term: Term,
    pub basis_source: BasisSource,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Removal {
    pub event: EventId,
    pub kind: RemovalKind,
    pub removed_at: TaxDate,
    pub legs: Vec<RemovalLeg>,
    pub appraisal_required: bool, // donation (>$5k FMV over-flag, FOLLOWUPS)
    pub donor_acquired_at: Option<TaxDate>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncomeRecord {
    pub event: EventId,
    pub recognized_at: TaxDate,
    pub sat: Sat,
    pub usd_fmv: Usd,
    pub kind: IncomeKind,
    pub business: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingLeg {
    pub lot_id: LotId,
    pub sat: Sat,
    pub usd_basis: Usd,
    pub acquired_at: TaxDate,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingTransfer {
    pub event: EventId,
    pub principal_sat: Sat,
    pub fee_sat: Option<Sat>,
    pub legs: Vec<PendingLeg>, // lots removed into pending (carry basis + acquired_at)
}

/// Fold accumulators that are NOT directly reconstructable from the post-fold `LedgerState` vectors
/// (FR9 `Σ in` / `Σ on-chain-fee-sats` / `Σ pending`). Carried as a FIELD on `LedgerState` (M3) —
/// `project` always returns `LedgerState` (NO `(LedgerState, FoldStats)` tuple). Populated in `finalize`;
/// a deterministic function of the events, so it is included in `PartialEq` and the determinism tests hold.
/// Zero-valued by `Default` (the early tasks leave it zero; Task 11 fills `fee_sats_consumed`, Task 13 the rest).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FoldStats {
    pub sigma_in: Sat,           // externally-sourced acquisitions (Acquire + Income + classified GiftReceived)
    pub fee_sats_consumed: Sat,  // sole FR9 conservation home for network-fee sats
    pub sigma_pending: Sat,      // principal + fee sats sitting in pending_reconciliation
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LedgerState {
    pub lots: Vec<Lot>,
    pub holdings_by_wallet: BTreeMap<WalletId, Sat>,
    pub disposals: Vec<Disposal>,
    pub removals: Vec<Removal>,
    pub income_recognized: Vec<IncomeRecord>,
    pub pending_reconciliation: Vec<PendingTransfer>,
    pub blockers: Vec<Blocker>,
    pub stats: FoldStats, // M3: fold accumulators (FR9), on-state field — never a tuple return
}
impl LedgerState {
    pub(crate) fn add_blocker(&mut self, kind: BlockerKind, event: Option<EventId>, detail: impl Into<String>) {
        self.blockers.push(Blocker { kind, event, detail: detail.into() });
    }
}
```

- [ ] **Step 2: `src/price.rs`**
```rust
use crate::conventions::{round_cents, Sat, TaxDate, Usd, SATS_PER_BTC};

/// Daily-close BTC/USD provider (§9.2). Pure & deterministic; the projection borrows `&dyn PriceProvider`
/// so identical (events, prices) → identical ledger (NFR4). The bundled dataset lives in btctax-adapters.
pub trait PriceProvider {
    /// USD per WHOLE BTC at the daily close for `date`, or None if unknown.
    fn usd_per_btc(&self, date: TaxDate) -> Option<Usd>;
}

/// FMV (USD, cents) of `sat` satoshis at `date`, if a price exists.
/// §7.1 totality (M6): **checked** Decimal ops — an overflow yields `None` (treated as missing FMV → the
/// `fmv_missing` gating path), never a panic.
pub fn fmv_of(prices: &dyn PriceProvider, date: TaxDate, sat: Sat) -> Option<Usd> {
    let px = prices.usd_per_btc(date)?;
    px.checked_mul(Usd::from(sat))
        .and_then(|x| x.checked_div(Usd::from(SATS_PER_BTC)))
        .map(round_cents)
}

/// Test/CLI stub: an explicit date→price map (deterministic).
#[derive(Debug, Default, Clone)]
pub struct StaticPrices(pub std::collections::BTreeMap<TaxDate, Usd>);
impl PriceProvider for StaticPrices {
    fn usd_per_btc(&self, date: TaxDate) -> Option<Usd> {
        self.0.get(&date).copied()
    }
}
```

- [ ] **Step 3: `src/project/pools.rs`** (FIFO pool + splits; the dual-mode pre/post-2025 keying lands in Task 12)
```rust
use crate::conventions::{split_pro_rata, Sat, TaxDate, Usd, TRANSITION_DATE};
use crate::event::BasisSource;
use crate::identity::{EventId, LotId, WalletId};
use crate::state::Lot;
use std::collections::BTreeMap;

/// Pool key: a single UniversalPool before 2025-01-01 (un-partitioned by wallet, §7.4), then per-wallet.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum PoolKey {
    Universal,
    Wallet(WalletId),
}
pub fn pool_key(date: TaxDate, wallet: &WalletId) -> PoolKey {
    if date < TRANSITION_DATE {
        PoolKey::Universal
    } else {
        PoolKey::Wallet(wallet.clone())
    }
}

#[derive(Debug, Default)]
pub struct PoolSet {
    /// Live lots per pool, kept in FIFO order (push on acquire/relocate; consume from the front).
    pub pools: BTreeMap<PoolKey, Vec<Lot>>,
    /// Per-origin split counter for deterministic split_sequence assignment (§6.2).
    next_split: BTreeMap<EventId, u32>,
}

impl PoolSet {
    /// Assign the next split_sequence for an origin (origin's first lot uses 0 via `new_origin`).
    fn bump_split(&mut self, origin: &EventId) -> u32 {
        let e = self.next_split.entry(origin.clone()).or_insert(0);
        let v = *e;
        *e += 1;
        v
    }
    /// Register a brand-new origin (Acquire/Income/seeded), claiming split_sequence 0.
    pub fn new_origin_lot(&mut self, key: PoolKey, mut lot: Lot) {
        let s = self.bump_split(&lot.lot_id.origin_event_id);
        lot.lot_id.split_sequence = s;
        self.pools.entry(key).or_default().push(lot);
    }
    /// Push a pre-built lot (already carrying a final LotId), e.g. relocated/seeded lots.
    pub fn push_lot(&mut self, key: PoolKey, lot: Lot) {
        self.pools.entry(key).or_default().push(lot);
    }

    /// FIFO-consume `need` sats from `key`. Returns the consumed (lot_id, sat, gain_basis, loss_basis, term-anchors)
    /// fragments and a shortfall (>0 if the pool could not cover `need` — caller raises uncovered_disposal).
    pub fn consume_fifo(&mut self, key: &PoolKey, need: Sat) -> (Vec<Consumed>, Sat) {
        let mut out = Vec::new();
        let mut remaining = need;
        if let Some(lots) = self.pools.get_mut(key) {
            let mut idx = 0;
            while remaining > 0 && idx < lots.len() {
                let lot = &mut lots[idx];
                if lot.remaining_sat <= 0 {
                    idx += 1;
                    continue;
                }
                let take = remaining.min(lot.remaining_sat);
                let (gain_basis, _rest) = split_pro_rata(lot.usd_basis, take, lot.remaining_sat);
                let loss_basis = lot.dual_loss_basis.map(|l| split_pro_rata(l, take, lot.remaining_sat).0);
                out.push(Consumed {
                    lot_id: lot.lot_id.clone(),
                    sat: take,
                    gain_basis,
                    loss_basis,
                    gain_hp_start: lot.gain_hp_start(),
                    loss_hp_start: lot.loss_hp_start(),
                    basis_source: lot.basis_source,
                    dual: lot.dual_loss_basis.is_some(),
                    basis_pending: lot.basis_pending,
                    wallet: lot.wallet.clone(),
                    acquired_at: lot.acquired_at,
                    donor_acquired_at: lot.donor_acquired_at,
                });
                // reduce the lot exactly (conserves Σbasis: gain_basis subtracted, remainder stays)
                lot.usd_basis -= gain_basis;
                if let (Some(dl), Some(taken)) = (lot.dual_loss_basis.as_mut(), loss_basis) {
                    *dl -= taken;
                }
                lot.remaining_sat -= take;
                remaining -= take;
                idx += 1;
            }
            lots.retain(|l| l.remaining_sat > 0);
        }
        (out, remaining)
    }
}

/// A consumed fragment (used to build Disposal/Removal/relocation legs).
#[derive(Debug, Clone)]
pub struct Consumed {
    pub lot_id: LotId,
    pub sat: Sat,
    pub gain_basis: Usd,
    pub loss_basis: Option<Usd>,
    pub gain_hp_start: TaxDate,
    pub loss_hp_start: TaxDate,
    pub basis_source: BasisSource,
    pub dual: bool,
    pub basis_pending: bool,
    pub wallet: WalletId,
    pub acquired_at: TaxDate,
    pub donor_acquired_at: Option<TaxDate>,
}
```

- [ ] **Step 4: `src/project/resolve.rs`** (PASS 1 — this task: pass-through of imported events; decisions in Task 7)
```rust
use crate::conventions::{tax_date, TaxDate};
use crate::event::*;
use crate::identity::{EventId, SourceRef};
use crate::price::PriceProvider;
use crate::project::ProjectionConfig;
use crate::state::Blocker;
use time::{OffsetDateTime, UtcOffset};

/// What an imported event behaves as in PASS 2, after decisions are applied. Variants are ADDED across tasks
/// (Task 7: decisions, Task 8: transfers, Task 9: gift/donation, Task 10: dual-basis, Task 11: fee, Task 12: seed).
#[derive(Debug, Clone)]
pub enum Op {
    Acquire(Acquire),
    // (Task 5) Dispose, (Task 6) Income, (Task 8) SelfTransfer/PendingOut/GiftReceived/IncomeInbound,
    // (Task 9) GiftOut/Donate, (Task 12) seeded — added as those tasks land.
    Unclassified,
    Skip, // e.g. a TransferIn consumed by a TransferLink; folds to nothing
}

#[derive(Debug, Clone)]
pub struct Eff {
    pub id: EventId,
    pub utc: OffsetDateTime,
    pub tz: UtcOffset,
    pub src_priority: u8,
    pub src_ref: SourceRef,
    pub wallet: Option<crate::identity::WalletId>,
    pub op: Op,
}
impl Eff {
    pub fn date(&self) -> TaxDate {
        tax_date(self.utc, self.tz)
    }
}

#[derive(Debug, Clone)]
pub enum TransitionMode {
    /// Default: pass 2 reconstructs per-wallet pools from the Universal remainder at 2025-01-01.
    PathA,
    /// An effective `SafeHarborAllocation` governs: pass 2 discards the Universal remainder and seeds
    /// these pre-built per-wallet lots (`LotId = (allocation EventId, index)`, `basis_source =
    /// SafeHarborAllocated`). Built by `resolve` in Task 12; empty/`PathA` until then. (N4: no `(())` placeholder.)
    PathB { seed: Vec<crate::state::Lot> },
}

pub struct Resolution {
    pub timeline: Vec<Eff>,
    pub transition: TransitionMode,
    pub blockers: Vec<Blocker>,
}

/// PASS 1. Task 4: copy imported events straight through (no decisions yet). Task 7 rewrites this.
/// `_prices`/`_config` are unused until Task 12 (transition effectiveness needs `config` for the TP8(b)
/// first-2025-disposition trigger and `prices` for the pre-2025 basis snapshot); they are part of the
/// signature from the START so `resolve`/`project` never change shape across tasks (I-2).
pub fn resolve(events: &[LedgerEvent], _prices: &dyn PriceProvider, _config: &ProjectionConfig) -> Resolution {
    let mut timeline = Vec::new();
    for ev in events {
        let (src_priority, src_ref) = match &ev.id {
            EventId::Import { source, source_ref } => (source.priority(), source_ref.clone()),
            _ => continue, // decisions/conflicts handled in Task 7
        };
        let op = match &ev.payload {
            EventPayload::Acquire(a) => Op::Acquire(a.clone()),
            EventPayload::Unclassified(_) => Op::Unclassified,
            _ => Op::Skip, // other imported variants land in Tasks 5/6/8
        };
        timeline.push(Eff { id: ev.id.clone(), utc: ev.utc_timestamp, tz: ev.original_tz, src_priority, src_ref, wallet: ev.wallet.clone(), op });
    }
    Resolution { timeline, transition: TransitionMode::PathA, blockers: Vec::new() }
}

/// Canonical PASS-2 order (§6.2): utc_timestamp → source priority → source_ref.
pub fn sort_canonical(timeline: &mut [Eff]) {
    timeline.sort_by(|a, b| {
        a.utc
            .cmp(&b.utc)
            .then(a.src_priority.cmp(&b.src_priority))
            .then(a.src_ref.cmp(&b.src_ref))
    });
}
```

- [ ] **Step 5: `src/project/fold.rs`** (PASS 2 — Acquire only this task)
```rust
use crate::conventions::Sat;
use crate::identity::LotId;
use crate::project::pools::{pool_key, PoolKey, PoolSet};
use crate::project::resolve::{sort_canonical, Op, Resolution};
use crate::price::PriceProvider;
use crate::state::{BlockerKind, FoldStats, LedgerState, Lot};
use crate::ProjectionConfig;
use std::collections::BTreeMap;

pub fn fold(mut res: Resolution, _prices: &dyn PriceProvider, _config: &ProjectionConfig) -> LedgerState {
    sort_canonical(&mut res.timeline);
    let mut st = LedgerState { blockers: res.blockers, ..Default::default() };
    let mut pools = PoolSet::default();
    let mut stats = FoldStats::default(); // M3/FR9: fee_sats_consumed filled in Task 11, sigma_in here

    for eff in &res.timeline {
        let date = eff.date();
        match &eff.op {
            Op::Acquire(a) => {
                let wallet = match &eff.wallet {
                    Some(w) => w.clone(),
                    None => {
                        st.add_blocker(BlockerKind::Unclassified, Some(eff.id.clone()), "acquire without wallet");
                        continue;
                    }
                };
                let lot = Lot {
                    lot_id: LotId { origin_event_id: eff.id.clone(), split_sequence: 0 },
                    wallet: wallet.clone(),
                    acquired_at: date,
                    original_sat: a.sat,
                    remaining_sat: a.sat,
                    usd_basis: a.usd_cost + a.fee_usd, // TP2: basis = cost + acquisition fee
                    basis_source: a.basis_source,
                    dual_loss_basis: None,
                    donor_acquired_at: None,
                    basis_pending: false,
                };
                pools.new_origin_lot(pool_key(date, &wallet), lot);
                stats.sigma_in += a.sat; // FR9 Σin: externally-sourced acquisition
            }
            Op::Unclassified => {
                st.add_blocker(BlockerKind::Unclassified, Some(eff.id.clone()), "unclassified BTC-side row");
            }
            Op::Skip => {}
        }
    }

    finalize(&mut st, pools, stats);
    st
}

/// Collect remaining lots + holdings; sort all output deterministically (NFR4); commit the FoldStats (M3).
pub fn finalize(st: &mut LedgerState, pools: PoolSet, mut stats: FoldStats) {
    let mut holdings: BTreeMap<crate::identity::WalletId, Sat> = BTreeMap::new();
    let mut lots: Vec<Lot> = Vec::new();
    for (_key, pool) in pools.pools {
        for lot in pool {
            if lot.remaining_sat > 0 {
                *holdings.entry(lot.wallet.clone()).or_insert(0) += lot.remaining_sat;
                lots.push(lot);
            }
        }
    }
    lots.sort_by(|a, b| {
        a.wallet
            .cmp(&b.wallet)
            .then(a.acquired_at.cmp(&b.acquired_at))
            .then(a.lot_id.cmp(&b.lot_id))
    });
    st.lots = lots;
    st.holdings_by_wallet = holdings;
    // M1: sort blockers by the DERIVED Ord of (kind, Option<EventId>, detail) — a total order, no Debug strings.
    st.blockers
        .sort_by(|a, b| a.kind.cmp(&b.kind).then_with(|| a.event.cmp(&b.event)).then_with(|| a.detail.cmp(&b.detail)));
    // Σpending is reconstructable from the queue; sigma_in/fee_sats_consumed are accumulated during the fold.
    stats.sigma_pending = st
        .pending_reconciliation
        .iter()
        .map(|p| p.principal_sat + p.fee_sat.unwrap_or(0))
        .sum();
    st.stats = stats;
}
```

- [ ] **Step 6: `src/project/mod.rs`**
```rust
pub mod fold;
pub mod pools;
pub mod resolve;

use crate::event::LedgerEvent;
use crate::price::PriceProvider;
use crate::state::LedgerState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeeTreatment {
    /// TP8 DEFAULT: fee_sat consumed at zero proceeds (non-taxable); full basis carries. USER-MANDATED default.
    TreatmentC,
    /// TP8 config: taxable mini-disposition of fee-sats (recognition record only; not a 2nd conservation entry).
    TreatmentB,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LotMethod {
    /// Default per TP5; specific-ID is a future hook (Phase 3 optimizer).
    Fifo,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectionConfig {
    pub self_transfer_fee: FeeTreatment,
    pub lot_method: LotMethod,
}
impl Default for ProjectionConfig {
    fn default() -> Self {
        // DO NOT change: TP8 default is (c); the spec/memory forbid flipping it to (b).
        ProjectionConfig { self_transfer_fee: FeeTreatment::TreatmentC, lot_method: LotMethod::Fifo }
    }
}

/// The projection contract (§7.1): pure, deterministic, no I/O, total (never panics).
pub fn project(events: &[LedgerEvent], prices: &dyn PriceProvider, config: &ProjectionConfig) -> LedgerState {
    // I-2: `resolve` takes (events, prices, config) — Task-12 transition effectiveness needs both.
    let resolution = resolve::resolve(events, prices, config);
    fold::fold(resolution, prices, config)
}
```

- [ ] **Step 7: Failing test `tests/determinism.rs`**
```rust
use btctax_core::event::*;
use btctax_core::identity::*;
use btctax_core::price::StaticPrices;
use btctax_core::project::{project, ProjectionConfig};
use rust_decimal_macros::dec;
use time::macros::{datetime, offset};

fn acq(src_ref: &str, h: u8, sat: i64, cost: rust_decimal::Decimal) -> LedgerEvent {
    LedgerEvent {
        id: EventId::import(Source::Coinbase, SourceRef::new(src_ref)),
        utc_timestamp: datetime!(2025-03-01 00:00:00 UTC).replace_hour(h).unwrap(),
        original_tz: offset!(+00:00),
        wallet: Some(WalletId::Exchange { provider: "cb".into(), account: "m".into() }),
        payload: EventPayload::Acquire(Acquire { sat, usd_cost: cost, fee_usd: dec!(0), basis_source: BasisSource::ExchangeProvided }),
    }
}

#[test]
fn identical_set_any_order_same_state() {
    let prices = StaticPrices::default();
    let cfg = ProjectionConfig::default();
    let a = acq("A", 1, 100_000, dec!(60.00));
    let b = acq("B", 2, 50_000, dec!(31.00));
    let s1 = project(&[a.clone(), b.clone()], &prices, &cfg);
    let s2 = project(&[b, a], &prices, &cfg); // reversed load order
    assert_eq!(s1, s2);
    assert_eq!(s1.holdings_by_wallet.values().sum::<i64>(), 150_000);
}
```

- [ ] **Step 8: Run → FAIL → implement → PASS.** `cargo test -p btctax-core --test determinism`
- [ ] **Step 9: Wire + gate + commit.** In `lib.rs`: `pub mod state; pub mod price; pub mod project;` + `pub use project::{project, FeeTreatment, LotMethod, ProjectionConfig}; pub use price::PriceProvider; pub use state::*;`
```bash
cargo clippy --all-targets -p btctax-core -- -D warnings && cargo fmt --check
git commit -am "feat(core): projection contract + canonical order + Acquire fold + determinism harness"
```

---

### Task 5: Lot consumption — `Dispose{Sell|Spend}` → `Disposal` (proceeds−fee, basis, gain, ST/LT) + splits + totality

**Files:** Modify `src/project/resolve.rs` (add `Op::Dispose`), `src/project/fold.rs` (Dispose arm + a shared `make_disposal_legs` helper). Test `tests/kat_tax.rs`.

**Interfaces — Consumes:** `pools::consume_fifo`, `conventions::{round_cents, split_pro_rata, is_long_term}`. **Produces:** Dispose fold; `uncovered_disposal` blocker (§7.3 totality); the reusable consumed→leg gain/term computation (dual-basis branch stubbed `None` here; filled in Task 10).

- [ ] **Step 1: Add to `Op`** in `resolve.rs`: `Dispose { sat: Sat, proceeds: Usd, fee_usd: Usd, kind: DisposeKind }` and map `EventPayload::Dispose(d)` → `Op::Dispose { sat: d.sat, proceeds: d.usd_proceeds, fee_usd: d.fee_usd, kind: d.kind }`.

- [ ] **Step 2: Failing KATs in `tests/kat_tax.rs`**
```rust
use btctax_core::event::*;
use btctax_core::identity::*;
use btctax_core::price::StaticPrices;
use btctax_core::project::{project, ProjectionConfig};
use btctax_core::state::*;
use rust_decimal_macros::dec;
use time::macros::{datetime, offset};

fn wal() -> WalletId { WalletId::Exchange { provider: "cb".into(), account: "m".into() } }
fn ev(src_ref: &str, ts: time::OffsetDateTime, p: EventPayload) -> LedgerEvent {
    LedgerEvent { id: EventId::import(Source::Coinbase, SourceRef::new(src_ref)), utc_timestamp: ts, original_tz: offset!(+00:00), wallet: Some(wal()), payload: p }
}

#[test]
fn buy_then_sell_one_year_one_day_is_long_term() {
    let buy = ev("BUY", datetime!(2025-03-01 00:00:00 UTC), EventPayload::Acquire(Acquire { sat: 100_000, usd_cost: dec!(60.00), fee_usd: dec!(0), basis_source: BasisSource::ExchangeProvided }));
    let sell = ev("SELL", datetime!(2026-03-02 00:00:00 UTC), EventPayload::Dispose(Dispose { sat: 100_000, usd_proceeds: dec!(100.50), fee_usd: dec!(0.50), kind: DisposeKind::Sell }));
    let st = project(&[buy, sell], &StaticPrices::default(), &ProjectionConfig::default());
    assert_eq!(st.disposals.len(), 1);
    let leg = &st.disposals[0].legs[0];
    assert_eq!(leg.term, Term::LongTerm);
    assert_eq!(leg.proceeds, dec!(100.00)); // 100.50 gross − 0.50 fee (TP2)
    assert_eq!(leg.basis, dec!(60.00));
    assert_eq!(leg.gain, dec!(40.00));
    assert!(st.holdings_by_wallet.is_empty());
}

#[test]
fn same_day_sell_is_short_term() {
    let buy = ev("BUY", datetime!(2025-03-01 09:00:00 UTC), EventPayload::Acquire(Acquire { sat: 100_000, usd_cost: dec!(60.00), fee_usd: dec!(0), basis_source: BasisSource::ExchangeProvided }));
    let sell = ev("SELL", datetime!(2025-03-01 17:00:00 UTC), EventPayload::Dispose(Dispose { sat: 40_000, usd_proceeds: dec!(30.00), fee_usd: dec!(0), kind: DisposeKind::Sell }));
    let st = project(&[buy, sell], &StaticPrices::default(), &ProjectionConfig::default());
    assert_eq!(st.disposals[0].legs[0].term, Term::ShortTerm);
    assert_eq!(st.holdings_by_wallet[&wal()], 60_000); // partial: 100k − 40k remains, same LotId
    assert_eq!(st.lots.len(), 1);
}

#[test]
fn oversell_raises_uncovered_disposal_and_never_panics() {
    let buy = ev("BUY", datetime!(2025-03-01 00:00:00 UTC), EventPayload::Acquire(Acquire { sat: 10_000, usd_cost: dec!(6.00), fee_usd: dec!(0), basis_source: BasisSource::ExchangeProvided }));
    let sell = ev("SELL", datetime!(2025-04-01 00:00:00 UTC), EventPayload::Dispose(Dispose { sat: 50_000, usd_proceeds: dec!(40.00), fee_usd: dec!(0), kind: DisposeKind::Sell }));
    let st = project(&[buy, sell], &StaticPrices::default(), &ProjectionConfig::default());
    assert!(st.blockers.iter().any(|b| b.kind == BlockerKind::UncoveredDisposal));
    assert!(st.lots.iter().all(|l| l.remaining_sat >= 0)); // no negative remainder
}
```

- [ ] **Step 3: Run → FAIL.** `cargo test -p btctax-core --test kat_tax`

- [ ] **Step 4: Implement** the Dispose arm in `fold.rs` + the shared helper:
```rust
use crate::conventions::{is_long_term, round_cents, split_pro_rata, TaxDate, Usd};
use crate::project::pools::Consumed;
use crate::state::{Disposal, DisposalLeg, GiftZone, Term};
use crate::event::DisposeKind;

/// TP4 term for a consumed fragment given the disposition date (gain side / no-dual uses gain_hp_start).
fn term_for(start: TaxDate, disposed: TaxDate) -> Term {
    if is_long_term(start, disposed) { Term::LongTerm } else { Term::ShortTerm }
}

/// Build disposal legs from consumed fragments and a TOTAL net proceeds amount, allocated pro-rata by sat
/// (remainder-takes-the-rest so Σproceeds is exact). Dual-basis gift logic (TP11) is added in Task 10;
/// here every leg is the simple `gift_zone = None` path.
fn make_disposal_legs(consumed: &[Consumed], total_net_proceeds: Usd, disposed: TaxDate, st: &mut crate::state::LedgerState, ev: &crate::identity::EventId) -> Vec<DisposalLeg> {
    let total_sat: i64 = consumed.iter().map(|c| c.sat).sum();
    let mut legs = Vec::new();
    let mut allocated = Usd::ZERO;
    for (i, c) in consumed.iter().enumerate() {
        let proceeds = if i + 1 == consumed.len() {
            total_net_proceeds - allocated
        } else {
            let (p, _) = split_pro_rata(total_net_proceeds, c.sat, total_sat);
            allocated += p;
            p
        };
        if c.basis_pending {
            // FMV-missing income / unknown-basis gift in this lot's history → gate the gain (§7.3).
            st.add_blocker(crate::state::BlockerKind::FmvMissing, Some(ev.clone()), "disposal consumes a basis-pending lot");
        }
        // Task 10 replaces this block with the four-zone dual-basis computation:
        let basis = c.gain_basis;
        let gain = proceeds - basis;
        let term = term_for(c.gain_hp_start, disposed);
        legs.push(DisposalLeg { lot_id: c.lot_id.clone(), sat: c.sat, proceeds, basis, gain: round_cents(gain), term, basis_source: c.basis_source, gift_zone: None::<GiftZone> });
    }
    legs
}
```
And the `Op::Dispose` arm in the fold loop:
```rust
Op::Dispose { sat, proceeds, fee_usd, kind } => {
    let wallet = match &eff.wallet { Some(w) => w.clone(), None => { st.add_blocker(BlockerKind::UncoveredDisposal, Some(eff.id.clone()), "dispose without wallet"); continue; } };
    let key = pool_key(date, &wallet);
    let (consumed, shortfall) = pools.consume_fifo(&key, *sat);
    if shortfall > 0 {
        st.add_blocker(BlockerKind::UncoveredDisposal, Some(eff.id.clone()), format!("dispose short by {shortfall} sat"));
    }
    if !consumed.is_empty() {
        let net = round_cents(*proceeds - *fee_usd); // TP2: disposition fee reduces proceeds
        let legs = make_disposal_legs(&consumed, net, date, &mut st, &eff.id);
        st.disposals.push(Disposal { event: eff.id.clone(), kind: *kind, disposed_at: date, legs, fee_mini_disposition: false });
    }
}
```

- [ ] **Step 5: Run → PASS.** `cargo test -p btctax-core --test kat_tax`
- [ ] **Step 6: Gate + commit.**
```bash
cargo clippy --all-targets -p btctax-core -- -D warnings && cargo fmt --check
git commit -am "feat(core): Dispose fold — FIFO consume, proceeds−fee/basis/gain, ST/LT, totality"
```

---

### Task 6: `Income` (TP3) — FMV-at-dominion lot, recognized income, `fmv_missing` gating

**Files:** Modify `resolve.rs` (`Op::Income`), `fold.rs` (Income arm). Extend `tests/kat_tax.rs`.

**Interfaces — Produces:** Income fold: if FMV known → new lot at FMV (`basis_source = FmvAtIncome`, HP starts that day), push `IncomeRecord`; if `Missing` → still create the **sat-bearing lot with `basis_pending = true`** (so sat-conservation holds) + `fmv_missing` blocker (§7.3). `ManualFmv` resolution is wired in Task 7 (pass 1) — this task handles the imported `Income` payload's own `usd_fmv`/`fmv_status`.

- [ ] **Step 1: Add to `Op`:** `Income { sat: Sat, fmv: Option<Usd>, kind: IncomeKind, business: bool }`; map `EventPayload::Income(x)` → `Op::Income { sat: x.sat, fmv: x.usd_fmv.filter(|_| x.fmv_status != FmvStatus::Missing), kind: x.kind, business: x.business }`.

- [ ] **Step 2: Failing KATs**
```rust
#[test]
fn income_creates_fmv_basis_lot_and_records_income() {
    let inc = ev("INC", datetime!(2025-05-01 00:00:00 UTC), EventPayload::Income(Income { sat: 100_000, usd_fmv: Some(dec!(50.00)), fmv_status: FmvStatus::PriceDataset, kind: IncomeKind::Interest, business: false }));
    let st = project(&[inc], &StaticPrices::default(), &ProjectionConfig::default());
    assert_eq!(st.income_recognized.len(), 1);
    assert_eq!(st.income_recognized[0].usd_fmv, dec!(50.00));
    assert_eq!(st.lots[0].usd_basis, dec!(50.00));
    assert_eq!(st.lots[0].basis_source, BasisSource::FmvAtIncome);
}

#[test]
fn income_missing_fmv_creates_lot_but_blocks_and_gates_downstream() {
    let inc = ev("INC", datetime!(2025-05-01 00:00:00 UTC), EventPayload::Income(Income { sat: 100_000, usd_fmv: None, fmv_status: FmvStatus::Missing, kind: IncomeKind::Mining, business: true }));
    let sell = ev("SELL", datetime!(2025-06-01 00:00:00 UTC), EventPayload::Dispose(Dispose { sat: 100_000, usd_proceeds: dec!(70.00), fee_usd: dec!(0), kind: DisposeKind::Sell }));
    let st = project(&[inc, sell], &StaticPrices::default(), &ProjectionConfig::default());
    assert!(st.blockers.iter().any(|b| b.kind == BlockerKind::FmvMissing)); // both the income AND the downstream disposal gate
    assert_eq!(st.holdings_by_wallet.get(&wal()), None); // sats existed for conservation, then disposed
    assert!(st.income_recognized.is_empty()); // no recognized income amount while FMV missing
}
```

- [ ] **Step 3: Run → FAIL → implement Income arm in `fold.rs`:**
```rust
Op::Income { sat, fmv, kind, business } => {
    let wallet = match &eff.wallet { Some(w) => w.clone(), None => { st.add_blocker(BlockerKind::FmvMissing, Some(eff.id.clone()), "income without wallet"); continue; } };
    let (basis, pending) = match fmv {
        Some(v) => {
            st.income_recognized.push(crate::state::IncomeRecord { event: eff.id.clone(), recognized_at: date, sat: *sat, usd_fmv: *v, kind: *kind, business: *business });
            (*v, false)
        }
        None => {
            st.add_blocker(BlockerKind::FmvMissing, Some(eff.id.clone()), "income FMV missing");
            (Usd::ZERO, true) // basis pending; lot still created so Σsat conservation holds (§7.3)
        }
    };
    let lot = Lot {
        lot_id: LotId { origin_event_id: eff.id.clone(), split_sequence: 0 },
        wallet: wallet.clone(), acquired_at: date, original_sat: *sat, remaining_sat: *sat,
        usd_basis: basis, basis_source: BasisSource::FmvAtIncome,
        dual_loss_basis: None, donor_acquired_at: None, basis_pending: pending,
    };
    pools.new_origin_lot(pool_key(date, &wallet), lot);
    stats.sigma_in += *sat; // FR9 Σin: income is externally-sourced (counts even while FMV is pending)
}
```
*(The disposal gating already fires via the `c.basis_pending` check in `make_disposal_legs` — Task 5. Removal gets the same check in Task 9.)*

> **Downstream-honoring invariant (tax M3):** an FMV-missing income (or unknown-basis gift) still creates a **sat-bearing lot** with `basis_pending = true` and `usd_basis = 0`, plus a **hard** `FmvMissing`/`UnknownBasisInbound` blocker. The basis-0 leg/gain is **provisional, never final**: any disposal/removal consuming a `basis_pending` lot re-raises the hard blocker (Tasks 5/9), and the Phase-2 layer MUST gate on it and never report the basis-0 number as a finished result.

- [ ] **Step 4: Run → PASS → gate + commit.**
```bash
cargo test -p btctax-core --test kat_tax && cargo clippy --all-targets -p btctax-core -- -D warnings && cargo fmt --check
git commit -am "feat(core): Income fold (TP3) — FMV basis lot, recognized income, fmv_missing gating"
```

---

### Task 7: PASS 1 decision resolution framework (§7.2 staged) — Void / Supersede / Reject / ClassifyRaw / ManualFmv + conflicts

**Files:** Rewrite `src/project/resolve.rs` (real pass 1). Test `tests/kat_tax.rs` (corrections) + add `tests/corrections.rs`.

**Interfaces — Consumes:** all decision payloads. **Produces:** the staged pass-1 of §7.2 step 1 (allocation/transition deferred to Task 12): build the applied import set (drop revocably-voided decisions; non-revocable Void → `decision_conflicts`), resolve `ImportConflict`s via the latest-`decision_seq` `SupersedeImport`/`RejectImport`, apply `ClassifyRaw`, apply `ManualFmv`, detect contradictory decisions → `decision_conflicts`, and emit `import_conflicts` for unresolved conflicts. Output: the effective imported timeline. (Transfer/inbound/outflow classification ops are added in Tasks 8–9; this task wires the *plumbing* and the import-conflict + void + ManualFmv + ClassifyRaw paths.)

- [ ] **Step 1: Failing tests `tests/corrections.rs`** (covers §13 "Determinism & corrections")
```rust
use btctax_core::event::*;
use btctax_core::identity::*;
use btctax_core::price::StaticPrices;
use btctax_core::project::{project, ProjectionConfig};
use btctax_core::state::*;
use rust_decimal_macros::dec;
use time::macros::{datetime, offset};

fn wal() -> WalletId { WalletId::Exchange { provider: "cb".into(), account: "m".into() } }
fn imp(src_ref: &str, ts: time::OffsetDateTime, p: EventPayload) -> LedgerEvent {
    LedgerEvent { id: EventId::import(Source::Coinbase, SourceRef::new(src_ref)), utc_timestamp: ts, original_tz: offset!(+00:00), wallet: Some(wal()), payload: p }
}
fn dec_ev(seq: u64, ts: time::OffsetDateTime, p: EventPayload) -> LedgerEvent {
    LedgerEvent { id: EventId::decision(seq), utc_timestamp: ts, original_tz: offset!(+00:00), wallet: None, payload: p }
}

#[test]
fn unresolved_import_conflict_blocks_and_keeps_original() {
    let buy = imp("A", datetime!(2025-03-01 00:00:00 UTC), EventPayload::Acquire(Acquire { sat: 100_000, usd_cost: dec!(60.00), fee_usd: dec!(0), basis_source: BasisSource::ExchangeProvided }));
    let new = EventPayload::Acquire(Acquire { sat: 100_000, usd_cost: dec!(99.00), fee_usd: dec!(0), basis_source: BasisSource::ExchangeProvided });
    let fp = btctax_core::persistence::fingerprint(&new).unwrap();
    let conflict = LedgerEvent { id: EventId::conflict(Source::Coinbase, SourceRef::new("A"), &fp), utc_timestamp: datetime!(2025-03-02 00:00:00 UTC), original_tz: offset!(+00:00), wallet: Some(wal()), payload: EventPayload::ImportConflict(ImportConflict { target: EventId::import(Source::Coinbase, SourceRef::new("A")), new_payload: Box::new(new), new_fingerprint: fp }) };
    let st = project(&[buy, conflict], &StaticPrices::default(), &ProjectionConfig::default());
    assert!(st.blockers.iter().any(|b| b.kind == BlockerKind::ImportConflict));
    assert_eq!(st.lots[0].usd_basis, dec!(60.00)); // original kept until resolved
}

#[test]
fn supersede_applies_new_payload_to_same_target_id() {
    let buy = imp("A", datetime!(2025-03-01 00:00:00 UTC), EventPayload::Acquire(Acquire { sat: 100_000, usd_cost: dec!(60.00), fee_usd: dec!(0), basis_source: BasisSource::ExchangeProvided }));
    let new = EventPayload::Acquire(Acquire { sat: 100_000, usd_cost: dec!(99.00), fee_usd: dec!(0), basis_source: BasisSource::ExchangeProvided });
    let fp = btctax_core::persistence::fingerprint(&new).unwrap();
    let cid = EventId::conflict(Source::Coinbase, SourceRef::new("A"), &fp);
    let conflict = LedgerEvent { id: cid.clone(), utc_timestamp: datetime!(2025-03-02 00:00:00 UTC), original_tz: offset!(+00:00), wallet: Some(wal()), payload: EventPayload::ImportConflict(ImportConflict { target: EventId::import(Source::Coinbase, SourceRef::new("A")), new_payload: Box::new(new), new_fingerprint: fp }) };
    let sup = dec_ev(1, datetime!(2026-01-01 00:00:00 UTC), EventPayload::SupersedeImport(SupersedeImport { conflict_event: cid }));
    let st = project(&[buy, conflict, sup], &StaticPrices::default(), &ProjectionConfig::default());
    assert!(st.blockers.iter().all(|b| b.kind != BlockerKind::ImportConflict)); // resolved
    assert_eq!(st.lots[0].usd_basis, dec!(99.00)); // new payload applied, same lot origin id
}

#[test]
fn void_of_supersede_is_a_decision_conflict_not_a_drop() {
    // SupersedeImport is non-revocable; voiding it must NOT silently drop it.
    let buy = imp("A", datetime!(2025-03-01 00:00:00 UTC), EventPayload::Acquire(Acquire { sat: 100_000, usd_cost: dec!(60.00), fee_usd: dec!(0), basis_source: BasisSource::ExchangeProvided }));
    let new = EventPayload::Acquire(Acquire { sat: 100_000, usd_cost: dec!(99.00), fee_usd: dec!(0), basis_source: BasisSource::ExchangeProvided });
    let fp = btctax_core::persistence::fingerprint(&new).unwrap();
    let cid = EventId::conflict(Source::Coinbase, SourceRef::new("A"), &fp);
    let conflict = LedgerEvent { id: cid.clone(), utc_timestamp: datetime!(2025-03-02 00:00:00 UTC), original_tz: offset!(+00:00), wallet: Some(wal()), payload: EventPayload::ImportConflict(ImportConflict { target: EventId::import(Source::Coinbase, SourceRef::new("A")), new_payload: Box::new(new), new_fingerprint: fp }) };
    let sup = dec_ev(1, datetime!(2026-01-01 00:00:00 UTC), EventPayload::SupersedeImport(SupersedeImport { conflict_event: cid }));
    let void = dec_ev(2, datetime!(2026-02-01 00:00:00 UTC), EventPayload::VoidDecisionEvent(VoidDecisionEvent { target_event_id: EventId::decision(1) }));
    let st = project(&[buy, conflict, sup, void], &StaticPrices::default(), &ProjectionConfig::default());
    assert!(st.blockers.iter().any(|b| b.kind == BlockerKind::DecisionConflict));
    assert_eq!(st.lots[0].usd_basis, dec!(99.00)); // supersede still in force
}

#[test]
fn late_supersede_rewrites_an_earlier_year_deterministically() {
    // A 2026 SupersedeImport rewrites a 2022 Acquire's basis; result independent of event order.
    let buy = imp("A", datetime!(2022-06-01 00:00:00 UTC), EventPayload::Acquire(Acquire { sat: 100_000, usd_cost: dec!(20.00), fee_usd: dec!(0), basis_source: BasisSource::ExchangeProvided }));
    let new = EventPayload::Acquire(Acquire { sat: 100_000, usd_cost: dec!(25.00), fee_usd: dec!(0), basis_source: BasisSource::ExchangeProvided });
    let fp = btctax_core::persistence::fingerprint(&new).unwrap();
    let cid = EventId::conflict(Source::Coinbase, SourceRef::new("A"), &fp);
    let conflict = LedgerEvent { id: cid.clone(), utc_timestamp: datetime!(2022-06-02 00:00:00 UTC), original_tz: offset!(+00:00), wallet: Some(wal()), payload: EventPayload::ImportConflict(ImportConflict { target: EventId::import(Source::Coinbase, SourceRef::new("A")), new_payload: Box::new(new), new_fingerprint: fp }) };
    let sup = dec_ev(1, datetime!(2026-01-01 00:00:00 UTC), EventPayload::SupersedeImport(SupersedeImport { conflict_event: cid }));
    let s1 = project(&[buy.clone(), conflict.clone(), sup.clone()], &StaticPrices::default(), &ProjectionConfig::default());
    let s2 = project(&[sup, buy, conflict], &StaticPrices::default(), &ProjectionConfig::default());
    assert_eq!(s1, s2);
    assert_eq!(s1.lots[0].usd_basis, dec!(25.00));
}
```

- [ ] **Step 2: Run → FAIL.** `cargo test -p btctax-core --test corrections`

- [ ] **Step 3: Rewrite `resolve.rs`** — staged pass 1 (§7.2 step 1). Structure:
```rust
// Pseudocode-complete; finalize signatures against the compiler.
// I-2: signature carries `_prices`/`_config` (still unused here; Task 12 renames + uses them).
pub fn resolve(events: &[LedgerEvent], _prices: &dyn PriceProvider, _config: &ProjectionConfig) -> Resolution {
    // index events by id
    let by_id: BTreeMap<EventId, &LedgerEvent> = events.iter().map(|e| (e.id.clone(), e)).collect();
    let mut blockers = Vec::new();

    // 1a. collect Voids; classify each target's revocability.
    //   Revocable: TransferLink, ReclassifyOutflow, ClassifyInbound, ManualFmv, ClassifyRaw,
    //              (SafeHarborAllocation handled in Task 12 — inert is voidable, effective is not).
    //   NON-revocable: SupersedeImport, RejectImport, VoidDecisionEvent.
    //   Void of a non-revocable target → decision_conflicts (target stays in force).
    let mut voided: BTreeSet<EventId> = BTreeSet::new();
    for e in events {
        if let EventPayload::VoidDecisionEvent(v) = &e.payload {
            match by_id.get(&v.target_event_id).map(|t| &t.payload) {
                Some(EventPayload::SupersedeImport(_)) | Some(EventPayload::RejectImport(_)) | Some(EventPayload::VoidDecisionEvent(_)) => {
                    blockers.push(Blocker { kind: BlockerKind::DecisionConflict, event: Some(e.id.clone()), detail: "void targets a non-revocable decision".into() });
                }
                Some(EventPayload::SafeHarborAllocation(_)) => { /* deferred to Task 12 (effective→conflict; inert→apply) */ }
                Some(_) => { voided.insert(v.target_event_id.clone()); }
                None => { blockers.push(Blocker { kind: BlockerKind::DecisionConflict, event: Some(e.id.clone()), detail: "void targets unknown event".into() }); }
            }
        }
    }

    // 1b. Resolve ImportConflicts: gather decisions in decision_seq order; per conflict, the latest
    //     SupersedeImport/RejectImport governs (§7.2). Build applied_payload: target_id -> payload.
    let mut applied: BTreeMap<EventId, EventPayload> = BTreeMap::new(); // overrides for import targets
    // collect (decision_seq, &decision) sorted ascending
    let mut decisions: Vec<(u64, &LedgerEvent)> = events.iter().filter_map(|e| match e.id { EventId::Decision { seq } => Some((seq, e)), _ => None }).collect();
    decisions.sort_by_key(|(s, _)| *s);
    // map conflict_id -> latest governing resolution
    let mut conflict_res: BTreeMap<EventId, Resolved> = BTreeMap::new(); // Resolved::Accept(payload)|Reject
    for (_seq, d) in &decisions {
        if voided.contains(&d.id) { continue; }
        match &d.payload {
            EventPayload::SupersedeImport(s) => {
                if let Some(EventPayload::ImportConflict(c)) = by_id.get(&s.conflict_event).map(|e| &e.payload) {
                    conflict_res.insert(s.conflict_event.clone(), Resolved::Accept((*c.new_payload).clone(), c.target.clone()));
                }
            }
            EventPayload::RejectImport(r) => {
                if let Some(EventPayload::ImportConflict(c)) = by_id.get(&r.conflict_event).map(|e| &e.payload) {
                    conflict_res.insert(r.conflict_event.clone(), Resolved::Reject(c.target.clone()));
                }
            }
            _ => {}
        }
    }
    // unresolved conflicts -> import_conflicts blocker; accepted -> applied override on the target id
    for e in events {
        if let EventPayload::ImportConflict(c) = &e.payload {
            match conflict_res.get(&e.id) {
                Some(Resolved::Accept(payload, target)) => { applied.insert(target.clone(), payload.clone()); }
                Some(Resolved::Reject(_)) => {}
                None => blockers.push(Blocker { kind: BlockerKind::ImportConflict, event: Some(e.id.clone()), detail: "unresolved import conflict".into() }),
            }
        }
    }

    // 1c. ClassifyRaw: replace an Unclassified target's effective payload (preserve target EventId).
    for (_seq, d) in &decisions {
        if voided.contains(&d.id) { continue; }
        if let EventPayload::ClassifyRaw(cr) = &d.payload {
            // contradictory ClassifyRaw on a target already overridden → decision_conflicts
            if applied.contains_key(&cr.target) {
                blockers.push(Blocker { kind: BlockerKind::DecisionConflict, event: Some(d.id.clone()), detail: "multiple classifications of one target".into() });
            } else {
                applied.insert(cr.target.clone(), (*cr.as_).clone());
            }
        }
    }

    // 1d. ManualFmv: collect event_id -> usd_fmv (latest decision_seq wins).
    let mut manual_fmv: BTreeMap<EventId, Usd> = BTreeMap::new();
    for (_seq, d) in &decisions {
        if voided.contains(&d.id) { continue; }
        if let EventPayload::ManualFmv(m) = &d.payload { manual_fmv.insert(m.event.clone(), m.usd_fmv); }
    }

    // 1e. Classification decisions (TransferLink/ReclassifyOutflow/ClassifyInbound): collect targets +
    //     detect contradictions (two distinct kinds on one target → decision_conflicts). Wired in Tasks 8–9.
    //     (Maps: outflow_class: out_id -> ReclassifyOutflow; inbound_class: in_id -> ClassifyInbound;
    //      links: out_id -> dest; consumed_ins: set of TransferIn ids consumed by a link.)

    // 2. Build the effective imported timeline from imported events, applying `applied` overrides + manual_fmv +
    //    classification maps. (Each imported event → Eff{ op }.) Unclassified with no ClassifyRaw → Op::Unclassified.
    let mut timeline = Vec::new();
    for e in events {
        let (src_priority, src_ref) = match &e.id { EventId::Import { source, source_ref } => (source.priority(), source_ref.clone()), _ => continue };
        let effective_payload = applied.get(&e.id).unwrap_or(&e.payload);
        let op = build_op(&e.id, effective_payload, &manual_fmv /*, classification maps */);
        timeline.push(Eff { id: e.id.clone(), utc: e.utc_timestamp, tz: e.original_tz, src_priority, src_ref, wallet: e.wallet.clone(), op });
    }

    Resolution { timeline, transition: TransitionMode::PathA, blockers }
}
```
with the `build_op` helper centralizing imported-payload→`Op` mapping (Acquire/Dispose/Income from Tasks 4–6; ManualFmv overrides an `Income`'s FMV by replacing `fmv = Some(manual)` and clearing the would-be `fmv_missing`). Add `enum Resolved { Accept(EventPayload, EventId), Reject(EventId) }`.

- [ ] **Step 4: Run → PASS** (corrections + all prior). `cargo test -p btctax-core`
- [ ] **Step 5: Gate + commit.**
```bash
cargo clippy --all-targets -p btctax-core -- -D warnings && cargo fmt --check
git commit -am "feat(core): pass-1 decision resolution (Void/Supersede/Reject/ClassifyRaw/ManualFmv + conflicts)"
```

---

### Task 8: Transfers & reconciliation — `TransferOut`→pending, `TransferIn`→unknown-basis, `TransferLink` (TP7), `ClassifyInbound`

**Files:** Modify `resolve.rs` (transfer/inbound ops + classification maps from Task 7 step 1e), `fold.rs` (new arms). Test `tests/kat_tax.rs`.

**Interfaces — Produces:** `Op::PendingOut`, `Op::SelfTransfer`, `Op::GiftReceived`, `Op::IncomeInbound`, `Op::UnknownInbound`, `Op::Skip` (a TransferIn consumed by a link). Folds:
- `TransferOut` with **no resolving decision** → consume `principal + fee_sat` FIFO from the source pool into `PendingTransfer` (sats leave holdings), raise `unmatched_outflows` (advisory).
- `TransferIn` with **no ClassifyInbound and not consumed by a link** → `unknown_basis_inbounds` (hard); **creates no lot** (sats not yet in the ledger — keeps FR9 conservation; §FR9/§7.3).
- `TransferLink` (TP7 self-transfer) → relocate `principal` sats from source pool to the dest wallet pool as new (split) lots carrying basis + `acquired_at` (+ `donor_acquired_at`), `basis_source = CarriedFromTransfer`; the destination `TransferIn` op = `Op::Skip` (a link relocates lots, it does not create sats). Fee handled in Task 11 (default TP8 (c)).
- `ClassifyInbound::Income` → income lot at FMV (`basis_source = FmvAtIncome`) + `IncomeRecord`. `ClassifyInbound::GiftReceived` → gift lot (full dual-basis logic in Task 10; this task creates the carryover/known-basis lot).

- [ ] **Step 1: Failing KATs** (these are the first `kat_tax.rs` tests that use decision events, so add a `dec_ev` helper to `kat_tax.rs`'s header — **M4: duplicated** from `corrections.rs` because each `tests/*.rs` integration file compiles as a SEPARATE crate and cannot share a private helper; the alternative is a shared `#[path = "common/mod.rs"]` helper module `include`d by both files):
```rust
// add to the kat_tax.rs header (next to `wal()`/`ev()` from Task 5) — duplicated per M4:
fn dec_ev(seq: u64, ts: time::OffsetDateTime, p: EventPayload) -> LedgerEvent {
    LedgerEvent { id: EventId::decision(seq), utc_timestamp: ts, original_tz: offset!(+00:00), wallet: None, payload: p }
}
```
```rust
#[test]
fn unclassified_transfer_out_moves_lots_to_pending() {
    let buy = ev("BUY", datetime!(2025-03-01 00:00:00 UTC), EventPayload::Acquire(Acquire { sat: 100_000, usd_cost: dec!(60.00), fee_usd: dec!(0), basis_source: BasisSource::ExchangeProvided }));
    let out = ev("OUT", datetime!(2025-04-01 00:00:00 UTC), EventPayload::TransferOut(TransferOut { sat: 100_000, fee_sat: None, dest_addr: None, txid: None }));
    let st = project(&[buy, out], &StaticPrices::default(), &ProjectionConfig::default());
    assert_eq!(st.pending_reconciliation.len(), 1);
    assert!(st.holdings_by_wallet.is_empty());
    assert!(st.blockers.iter().any(|b| b.kind == BlockerKind::UnmatchedOutflows));
}

#[test]
fn transfer_link_relocates_lots_non_taxably_carrying_basis_and_hp() {
    let cold = WalletId::SelfCustody { label: "cold".into() };
    let buy = ev("BUY", datetime!(2025-03-01 00:00:00 UTC), EventPayload::Acquire(Acquire { sat: 100_000, usd_cost: dec!(60.00), fee_usd: dec!(0), basis_source: BasisSource::ExchangeProvided }));
    let out = ev("OUT", datetime!(2025-04-01 00:00:00 UTC), EventPayload::TransferOut(TransferOut { sat: 100_000, fee_sat: None, dest_addr: None, txid: None }));
    let in_ev = LedgerEvent { id: EventId::import(Source::Swan, SourceRef::new("IN")), utc_timestamp: datetime!(2025-04-01 01:00:00 UTC), original_tz: offset!(+00:00), wallet: Some(cold.clone()), payload: EventPayload::TransferIn(TransferIn { sat: 100_000, src_addr: None, txid: None }) };
    let link = dec_ev(1, datetime!(2026-01-01 00:00:00 UTC), EventPayload::TransferLink(TransferLink { out_event: EventId::import(Source::Coinbase, SourceRef::new("OUT")), in_event_or_wallet: TransferTarget::InEvent(EventId::import(Source::Swan, SourceRef::new("IN"))) }));
    let st = project(&[buy, out, in_ev, link], &StaticPrices::default(), &ProjectionConfig::default());
    assert!(st.disposals.is_empty() && st.removals.is_empty()); // non-taxable (TP7)
    assert_eq!(st.holdings_by_wallet[&cold], 100_000);
    assert_eq!(st.lots[0].acquired_at, time::macros::date!(2025 - 03 - 01)); // HP carries
    assert_eq!(st.lots[0].usd_basis, dec!(60.00));
    assert!(st.pending_reconciliation.is_empty());
    assert!(st.blockers.iter().all(|b| b.kind != BlockerKind::UnknownBasisInbound)); // dest TransferIn consumed
}

#[test]
fn unclassified_inbound_is_blocker_without_creating_a_lot() {
    let in_ev = LedgerEvent { id: EventId::import(Source::Gemini, SourceRef::new("IN")), utc_timestamp: datetime!(2025-04-01 00:00:00 UTC), original_tz: offset!(+00:00), wallet: Some(wal()), payload: EventPayload::TransferIn(TransferIn { sat: 100_000, src_addr: None, txid: None }) };
    let st = project(&[in_ev], &StaticPrices::default(), &ProjectionConfig::default());
    assert!(st.blockers.iter().any(|b| b.kind == BlockerKind::UnknownBasisInbound));
    assert!(st.lots.is_empty() && st.holdings_by_wallet.is_empty());
}

#[test]
fn classify_inbound_as_income_creates_fmv_lot() {
    let in_ev = LedgerEvent { id: EventId::import(Source::Gemini, SourceRef::new("IN")), utc_timestamp: datetime!(2025-04-01 00:00:00 UTC), original_tz: offset!(+00:00), wallet: Some(wal()), payload: EventPayload::TransferIn(TransferIn { sat: 100_000, src_addr: None, txid: None }) };
    let cls = dec_ev(1, datetime!(2026-01-01 00:00:00 UTC), EventPayload::ClassifyInbound(ClassifyInbound { transfer_in_event: EventId::import(Source::Gemini, SourceRef::new("IN")), as_: InboundClass::Income { kind: IncomeKind::Reward, fmv: Some(dec!(45.00)), business: false } }));
    let st = project(&[in_ev, cls], &StaticPrices::default(), &ProjectionConfig::default());
    assert_eq!(st.income_recognized[0].usd_fmv, dec!(45.00));
    assert_eq!(st.lots[0].usd_basis, dec!(45.00));
}
```

- [ ] **Step 2: Run → FAIL → implement** (resolve.rs classification maps + ops; fold.rs arms). Key logic:
  - In `resolve` step 1e, build: `links: BTreeMap<out_id, TransferTarget>`, `consumed_ins: BTreeSet<in_id>` (for `TransferTarget::InEvent`), `outflow_class: BTreeMap<out_id, (OutflowClass, Usd, Option<Usd>)>`, `inbound_class: BTreeMap<in_id, InboundClass>`. A target appearing in two contradictory maps (e.g. both a link and a reclassify) → `decision_conflicts`.
  - `build_op` for `TransferOut`: if in `links` → `Op::SelfTransfer { sat, fee_sat, dest }` (resolve `InEvent`→that event's wallet; `Wallet`→given); elif in `outflow_class` → `Op::GiftOut`/`Op::Donate`/`Op::Dispose` (Task 9); else `Op::PendingOut { sat, fee_sat }`.
  - `build_op` for `TransferIn`: if id ∈ `consumed_ins` → `Op::Skip`; elif in `inbound_class` → `Op::GiftReceived{…}`/`Op::IncomeInbound{…}`; else `Op::UnknownInbound { sat }`.
  - fold `Op::PendingOut` → `consume_fifo(principal + fee)` → `PendingTransfer{ legs }`; `unmatched_outflows` advisory blocker.
  - fold `Op::SelfTransfer` → `consume_fifo(principal)` from the **source** pool; for each `Consumed` build a relocated `Lot` (new split_sequence via `pools.bump_split` exposed as a method; `basis_source = CarriedFromTransfer`; carry `acquired_at`/`donor_acquired_at`/`dual_loss_basis`/`basis_pending`) into a **`relocated: Vec<Lot>`** (do **not** push yet), then `for lot in relocated { pools.push_lot(pool_key(date, &dest), lot) }`. **Structure the arm so the fee step (Task 11) slots in BETWEEN building `relocated` and pushing it:** Task 11 consumes the `fee_sat` and, under default TP8 (c), **re-homes the fee-sats' carried basis onto `relocated.last_mut()` so the FULL basis carries to the destination (C1)** — never dropped. (This task, with no fee handling yet, builds `relocated` and pushes; the worked example's destination basis becomes correct only once Task 11's re-home lands.)
  - fold `Op::UnknownInbound` → `unknown_basis_inbounds` hard blocker; **no lot** (sats not yet in the ledger; not counted in Σin).
  - fold `Op::IncomeInbound` → identical to the Income arm (incl. `stats.sigma_in += sat`). `Op::GiftReceived` → create gift lot (Task 10 fills dual-basis; here: known donor_basis → carryover lot, `basis_source = GiftCarryover`) and `stats.sigma_in += sat` (classified `GiftReceived` is externally-sourced, FR9).

- [ ] **Step 3: Run → PASS → gate + commit.**
```bash
cargo test -p btctax-core && cargo clippy --all-targets -p btctax-core -- -D warnings && cargo fmt --check
git commit -am "feat(core): transfers & reconciliation — pending/unknown-basis/TransferLink(TP7)/ClassifyInbound"
```

---

### Task 9: Gift/donation outbound (TP10) — `ReclassifyOutflow` → `Removal` (zero gain) / `Dispose`

**Files:** Modify `resolve.rs` (`Op::GiftOut`/`Op::Donate`/reclassified `Op::Dispose`), `fold.rs` (Removal arms). Test `tests/kat_tax.rs`.

**Interfaces — Produces:** Removal fold for `GiftOut`/`Donate`: consume `principal` FIFO → per-lot `RemovalLeg` (basis + allocated FMV + ST/LT + donor metadata), **zero recognized gain** (TP10); `Donate` sets `appraisal_required`. A `ReclassifyOutflow{as: Dispose}` folds exactly like an imported `Dispose` (proceeds = `principal_proceeds_or_fmv`, fee = `fee_usd`). On-chain `fee_sat` consumed per TP8 (Task 11; default (c)).

- [ ] **Step 1: Failing KATs**
```rust
#[test]
fn gift_out_is_zero_gain_with_basis_fmv_and_term() {
    let buy = ev("BUY", datetime!(2025-03-01 00:00:00 UTC), EventPayload::Acquire(Acquire { sat: 100_000, usd_cost: dec!(60.00), fee_usd: dec!(0), basis_source: BasisSource::ExchangeProvided }));
    let out = ev("OUT", datetime!(2026-06-01 00:00:00 UTC), EventPayload::TransferOut(TransferOut { sat: 100_000, fee_sat: None, dest_addr: None, txid: None }));
    let recl = dec_ev(1, datetime!(2026-06-15 00:00:00 UTC), EventPayload::ReclassifyOutflow(ReclassifyOutflow { transfer_out_event: EventId::import(Source::Coinbase, SourceRef::new("OUT")), as_: OutflowClass::GiftOut, principal_proceeds_or_fmv: dec!(150.00), fee_usd: None }));
    let st = project(&[buy, out, recl], &StaticPrices::default(), &ProjectionConfig::default());
    assert!(st.disposals.is_empty());
    let leg = &st.removals[0].legs[0];
    assert_eq!(leg.basis, dec!(60.00));
    assert_eq!(leg.fmv_at_transfer, dec!(150.00));
    assert_eq!(leg.term, Term::LongTerm); // bought 2025-03-01, gifted 2026-06-01
    assert_eq!(st.removals[0].kind, RemovalKind::Gift);
}

#[test]
fn donation_over_5k_flags_appraisal_required() {
    let buy = ev("BUY", datetime!(2025-01-05 00:00:00 UTC), EventPayload::Acquire(Acquire { sat: 100_000_000, usd_cost: dec!(1000.00), fee_usd: dec!(0), basis_source: BasisSource::ExchangeProvided }));
    let out = ev("OUT", datetime!(2026-02-01 00:00:00 UTC), EventPayload::TransferOut(TransferOut { sat: 100_000_000, fee_sat: None, dest_addr: None, txid: None }));
    let recl = dec_ev(1, datetime!(2026-02-02 00:00:00 UTC), EventPayload::ReclassifyOutflow(ReclassifyOutflow { transfer_out_event: EventId::import(Source::Coinbase, SourceRef::new("OUT")), as_: OutflowClass::Donate { appraisal_required: true }, principal_proceeds_or_fmv: dec!(60000.00), fee_usd: None }));
    let st = project(&[buy, out, recl], &StaticPrices::default(), &ProjectionConfig::default());
    assert!(st.removals[0].appraisal_required);
    assert_eq!(st.removals[0].kind, RemovalKind::Donation);
}
```

- [ ] **Step 2: Run → FAIL → implement** the Removal builder (mirrors `make_disposal_legs`, but FMV allocated pro-rata, gain forced to zero, term from `gain_hp_start`):
```rust
fn make_removal_legs(consumed: &[Consumed], total_fmv: Usd, removed: TaxDate, st: &mut LedgerState, ev: &EventId) -> (Vec<RemovalLeg>, Option<TaxDate>) {
    let total_sat: i64 = consumed.iter().map(|c| c.sat).sum();
    let mut legs = Vec::new();
    let mut allocated = Usd::ZERO;
    let mut donor = None;
    for (i, c) in consumed.iter().enumerate() {
        if c.basis_pending { st.add_blocker(BlockerKind::UnknownBasisInbound, Some(ev.clone()), "removal consumes a basis-pending lot"); }
        let fmv = if i + 1 == consumed.len() { total_fmv - allocated } else { let (f, _) = split_pro_rata(total_fmv, c.sat, total_sat); allocated += f; f };
        donor = donor.or(c.donor_acquired_at);
        legs.push(RemovalLeg { lot_id: c.lot_id.clone(), sat: c.sat, basis: c.gain_basis, fmv_at_transfer: fmv, term: term_for(c.gain_hp_start, removed), basis_source: c.basis_source });
    }
    (legs, donor)
}
```
And the `Op::GiftOut`/`Op::Donate` arms (consume `principal`; build `legs` via `make_removal_legs`; then `Removal{ kind, legs, appraisal_required, donor_acquired_at }`; **no disposal**, zero gain). The reclassified-`Dispose` arm reuses Task 5's Dispose path. **Structure the arm so the fee step (Task 11) slots in BETWEEN building `legs` and pushing the `Removal`:** by analogy to TP8 (c), Task 11 consumes the `fee_sat` and re-homes its carried basis onto `legs.last_mut()` so the donee's carried-over basis is the **FULL** lot basis, not principal-only (C1) — the gift/donation network-fee basis is never dropped.

- [ ] **Step 3: Run → PASS → gate + commit.**
```bash
cargo test -p btctax-core && cargo clippy --all-targets -p btctax-core -- -D warnings && cargo fmt --check
git commit -am "feat(core): gift/donation outbound (TP10) — Removal with zero gain, appraisal flag; reclassify→Dispose"
```

> **Phase-2 preservation invariant (tax M2):** every `Removal` always carries `appraisal_required`, each leg's `fmv_at_transfer`, and ST/LT `term` (+ `donor_acquired_at`); these are emitted unconditionally so the Phase-2 forms layer (Form 8283 / Schedule A) has them. The engine never *derives* the precise §170(f)(11) appraisal trigger — `appraisal_required` is decision-supplied (a safe >$5k FMV over-flag, FOLLOWUPS) and must be passed through untouched.

---

### Task 10: Received-gift dual basis (TP11) — four cases + `GiftFmvFallback` + unknown-basis gating

**Files:** Modify `fold.rs` (`Op::GiftReceived` lot construction + the four-zone branch in `make_disposal_legs`), `resolve.rs` (`Op::GiftReceived` carries `donor_basis`/`donor_acquired_at`/`fmv_at_gift`). Uses `PriceProvider` for the fallback. Test `tests/kat_tax.rs`.

**Interfaces — Produces:** the §1015(a)/§1223(2) dual-basis lot + disposal logic:
- Lot construction from `ClassifyInbound::GiftReceived`:
  - `donor_basis = Some(b)`, `fmv_at_gift >= b` → single carryover: `usd_basis = b`, `dual_loss_basis = None`, `donor_acquired_at = Some(...)` (tacks), `basis_source = GiftCarryover`.
  - `donor_basis = Some(b)`, `fmv_at_gift < b` → dual: `usd_basis = b` (gain, tacks), `dual_loss_basis = Some(fmv_at_gift)` (loss, HP from gift date = `acquired_at`), `basis_source = GiftCarryover`.
  - `donor_basis = None`, `donor_acquired_at = Some(d)` → `GiftFmvFallback`: `usd_basis = fmv_of(prices, d, sat)` (if price exists), `dual_loss_basis = None`, tacks, `basis_source = GiftFmvFallback`; if price missing → `basis_pending` + `unknown_basis_inbounds`.
  - `donor_basis = None`, `donor_acquired_at = None` → create sat-bearing lot, `basis_pending = true`, `usd_basis = 0` + `unknown_basis_inbounds` (symmetric with Income-Missing; §7.3).
- Disposal four-zone branch (replaces Task 5's simple block when `c.dual`):
  - `dual = None` → single carryover (gain = proceeds − gain_basis, term from gain_hp_start). [already done]
  - else proceeds > gain_basis → **Gain** zone (basis = gain_basis, term tacks).
  - else proceeds < loss_basis → **Loss** zone (basis = loss_basis, term from loss_hp_start = gift date).
  - else → **NoGainNoLoss** (reported basis = proceeds, gain = 0, term from gain_hp_start).

- [ ] **Step 1: Failing KATs — all four TP11 cases + fallback**
```rust
fn gift_lot(donor_basis: Option<rust_decimal::Decimal>, donor_acq: Option<time::Date>, fmv_at_gift: rust_decimal::Decimal, recv: time::OffsetDateTime) -> Vec<LedgerEvent> {
    let in_ev = LedgerEvent { id: EventId::import(Source::Swan, SourceRef::new("GIN")), utc_timestamp: recv, original_tz: offset!(+00:00), wallet: Some(wal()), payload: EventPayload::TransferIn(TransferIn { sat: 100_000, src_addr: None, txid: None }) };
    let cls = dec_ev(1, datetime!(2026-12-31 00:00:00 UTC), EventPayload::ClassifyInbound(ClassifyInbound { transfer_in_event: EventId::import(Source::Swan, SourceRef::new("GIN")), as_: InboundClass::GiftReceived { donor_basis, donor_acquired_at: donor_acq, fmv_at_gift } }));
    vec![in_ev, cls]
}
fn sell(ts: time::OffsetDateTime, proceeds: rust_decimal::Decimal) -> LedgerEvent {
    ev("S", ts, EventPayload::Dispose(Dispose { sat: 100_000, usd_proceeds: proceeds, fee_usd: dec!(0), kind: DisposeKind::Sell }))
}

#[test]
fn tp11_case_no_dual_basis_fmv_ge_donor_basis_tacks() {
    let mut evs = gift_lot(Some(dec!(40.00)), Some(time::macros::date!(2024 - 01 - 01)), dec!(60.00), datetime!(2025-06-01 00:00:00 UTC));
    evs.push(sell(datetime!(2025-07-01 00:00:00 UTC), dec!(80.00)));
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    let leg = &st.disposals[0].legs[0];
    assert_eq!(leg.basis, dec!(40.00));
    assert_eq!(leg.gain, dec!(40.00));
    assert_eq!(leg.term, Term::LongTerm); // tacks from donor 2024-01-01
    assert_eq!(leg.gift_zone, None);
}

#[test]
fn tp11_case_gain_zone_with_tacking() {
    let mut evs = gift_lot(Some(dec!(100.00)), Some(time::macros::date!(2024 - 01 - 01)), dec!(60.00), datetime!(2025-06-01 00:00:00 UTC)); // dual: fmv<basis
    evs.push(sell(datetime!(2025-07-01 00:00:00 UTC), dec!(120.00))); // proceeds > gain basis
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    let leg = &st.disposals[0].legs[0];
    assert_eq!(leg.gift_zone, Some(GiftZone::Gain));
    assert_eq!(leg.basis, dec!(100.00));
    assert_eq!(leg.gain, dec!(20.00));
    assert_eq!(leg.term, Term::LongTerm); // tacks
}

#[test]
fn tp11_case_loss_zone_hp_from_gift_date() {
    let mut evs = gift_lot(Some(dec!(100.00)), Some(time::macros::date!(2024 - 01 - 01)), dec!(60.00), datetime!(2025-06-01 00:00:00 UTC)); // dual
    evs.push(sell(datetime!(2025-07-01 00:00:00 UTC), dec!(40.00))); // proceeds < loss basis (60)
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    let leg = &st.disposals[0].legs[0];
    assert_eq!(leg.gift_zone, Some(GiftZone::Loss));
    assert_eq!(leg.basis, dec!(60.00));
    assert_eq!(leg.gain, dec!(-20.00));
    assert_eq!(leg.term, Term::ShortTerm); // HP from gift date 2025-06-01
}

#[test]
fn tp11_case_middle_zone_zero_gain() {
    let mut evs = gift_lot(Some(dec!(100.00)), Some(time::macros::date!(2024 - 01 - 01)), dec!(60.00), datetime!(2025-06-01 00:00:00 UTC)); // dual: loss=60, gain=100
    evs.push(sell(datetime!(2025-07-01 00:00:00 UTC), dec!(80.00))); // 60 <= 80 <= 100
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    let leg = &st.disposals[0].legs[0];
    assert_eq!(leg.gift_zone, Some(GiftZone::NoGainNoLoss));
    assert_eq!(leg.gain, dec!(0));
}

#[test]
fn tp11_unknown_donor_basis_uses_fmv_at_donor_acquisition_date() {
    let mut prices = StaticPrices::default();
    prices.0.insert(time::macros::date!(2023 - 03 - 15), dec!(28000.00)); // BTC/USD at donor acq date
    let mut evs = gift_lot(None, Some(time::macros::date!(2023 - 03 - 15)), dec!(60.00), datetime!(2025-06-01 00:00:00 UTC));
    evs.push(sell(datetime!(2025-07-01 00:00:00 UTC), dec!(100.00)));
    let st = project(&evs, &prices, &ProjectionConfig::default());
    // 100_000 sat @ 28000/BTC = 28.00 basis (GiftFmvFallback)
    assert_eq!(st.disposals[0].legs[0].basis, dec!(28.00));
    assert_eq!(st.disposals[0].legs[0].basis_source, BasisSource::GiftFmvFallback);
}

#[test]
fn tp11_unknown_donor_basis_and_date_creates_basis_pending_lot() {
    let evs = gift_lot(None, None, dec!(60.00), datetime!(2025-06-01 00:00:00 UTC));
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(st.blockers.iter().any(|b| b.kind == BlockerKind::UnknownBasisInbound));
    assert_eq!(st.lots[0].remaining_sat, 100_000); // sat-bearing lot exists (conservation)
    assert!(st.lots[0].basis_pending);
}
```

- [ ] **Step 2: Run → FAIL → implement** the gift-lot constructor + the four-zone branch (replace the simple block in `make_disposal_legs` with the `match c.dual / c.loss_basis` logic; pass `prices` into the gift-lot builder for the fallback). Note: in the NoGainNoLoss zone the reported `basis = proceeds` (gain 0) while the lot's `usd_basis` still reduced by the pro-rata gain-basis on consume — Σbasis stays exact.

- [ ] **Step 3: Run → PASS → gate + commit.**
```bash
cargo test -p btctax-core && cargo clippy --all-targets -p btctax-core -- -D warnings && cargo fmt --check
git commit -am "feat(core): received-gift dual basis (TP11) — four zones + GiftFmvFallback + unknown-basis gating"
```

---

### Task 11: Self-transfer network fee (TP8) — default (c), config (b) mini-disposition; gift/donation fee by analogy

**Files:** Modify `fold.rs` (fee handling in `Op::SelfTransfer`, `Op::GiftOut`, `Op::Donate`); add a fee-sat ledger sum. Test `tests/kat_tax.rs`.

**Interfaces — Produces:** TP8 fee handling, gated on `config.self_transfer_fee`. The fee-sats are consumed FIFO from the **source** pool and recorded in `stats.fee_sats_consumed` (the sole FR9 conservation home). **C1 — basis must never be dropped:**
- **(c) default (full basis carries):** the fee-sats are consumed at **zero proceeds** (non-taxable, **no `Disposal`**) BUT their carried basis is **re-homed onto the surviving relocated lot(s) / removal leg(s)** — the destination ends with the **FULL** basis of the consumed (principal + fee) sats while holding only the principal sats. The fee-sats are burned; their basis fragment is **not** destroyed.
- **(b) config:** consume `fee_sat` FIFO; emit a `Disposal { fee_mini_disposition: true, kind: Spend }` with proceeds = `fmv_of(prices, date, fee_sat)`, gain = proceeds − basis (the fee-sats' basis rides the mini-disposition — **not** re-homed onto survivors); the sats still count only in `stats.fee_sats_consumed` (no second conservation entry, FR9).
- **Same rule by analogy to gift/donation `fee_sat` (§7.3):** under (c) the fee-sats' basis is re-homed onto the **last `Removal` leg** so the donee's carried-over basis is the full lot basis; under (b) a mini-disposition recognition record is emitted. `stats.fee_sats_consumed` is surfaced for FR9 in Task 13 via `conservation_report` (M3: on the `LedgerState.stats` field — no tuple return).

> **Why C1 matters:** consuming `principal` then dropping the `fee_sat` basis at zero proceeds (the pre-fix design) left the destination at **$59.88**, not the mandated **$60.00** — a silent, compounding basis leak that overstates future gain and breaks the Σbasis invariant (Task 13). Re-homing the fee basis fixes it while keeping FR9 *sat* conservation (`fee_sats_consumed` still absorbs the fee sats). This does **not** touch the TP8 default — it stays (c); the "do not flip to (b)" guards remain.

- [ ] **Step 1: Failing KATs**
```rust
fn cfg_b() -> ProjectionConfig { ProjectionConfig { self_transfer_fee: FeeTreatment::TreatmentB, ..ProjectionConfig::default() } }

#[test]
fn self_transfer_fee_default_c_is_non_taxable() {
    let cold = WalletId::SelfCustody { label: "cold".into() };
    let buy = ev("BUY", datetime!(2025-03-01 00:00:00 UTC), EventPayload::Acquire(Acquire { sat: 100_000, usd_cost: dec!(60.00), fee_usd: dec!(0), basis_source: BasisSource::ExchangeProvided }));
    let out = ev("OUT", datetime!(2025-04-01 00:00:00 UTC), EventPayload::TransferOut(TransferOut { sat: 99_800, fee_sat: Some(200), dest_addr: None, txid: None }));
    let in_ev = LedgerEvent { id: EventId::import(Source::Swan, SourceRef::new("IN")), utc_timestamp: datetime!(2025-04-01 01:00:00 UTC), original_tz: offset!(+00:00), wallet: Some(cold.clone()), payload: EventPayload::TransferIn(TransferIn { sat: 99_800, src_addr: None, txid: None }) };
    let link = dec_ev(1, datetime!(2026-01-01 00:00:00 UTC), EventPayload::TransferLink(TransferLink { out_event: EventId::import(Source::Coinbase, SourceRef::new("OUT")), in_event_or_wallet: TransferTarget::InEvent(EventId::import(Source::Swan, SourceRef::new("IN"))) }));
    let st = project(&[buy, out, in_ev, link], &StaticPrices::default(), &ProjectionConfig::default());
    assert!(st.disposals.is_empty()); // (c): no recognition
    assert_eq!(st.holdings_by_wallet[&cold], 99_800); // 100_000 − 200 fee
    // C1: the destination lot carries the FULL $60.00 basis (the 200 fee-sats' $0.12 re-homed, NOT dropped to $59.88).
    assert_eq!(st.lots.len(), 1);
    assert_eq!(st.lots[0].wallet, cold);
    assert_eq!(st.lots[0].remaining_sat, 99_800);
    assert_eq!(st.lots[0].usd_basis, dec!(60.00));
    assert_eq!(st.lots[0].basis_source, BasisSource::CarriedFromTransfer);
    assert_eq!(st.stats.fee_sats_consumed, 200); // FR9: fee-sats' sole conservation home
}

#[test]
fn gift_out_fee_default_c_carries_full_basis_onto_the_removal() {
    // Gift/donation fee BY ANALOGY (§7.3): under (c) the fee-sats' basis is re-homed onto the removal leg,
    // so the donee's carried-over basis is the FULL $60.00 (not $59.88). C1 analogue.
    let buy = ev("BUY", datetime!(2025-03-01 00:00:00 UTC), EventPayload::Acquire(Acquire { sat: 100_000, usd_cost: dec!(60.00), fee_usd: dec!(0), basis_source: BasisSource::ExchangeProvided }));
    let out = ev("OUT", datetime!(2026-06-01 00:00:00 UTC), EventPayload::TransferOut(TransferOut { sat: 99_800, fee_sat: Some(200), dest_addr: None, txid: None }));
    let recl = dec_ev(1, datetime!(2026-06-15 00:00:00 UTC), EventPayload::ReclassifyOutflow(ReclassifyOutflow { transfer_out_event: EventId::import(Source::Coinbase, SourceRef::new("OUT")), as_: OutflowClass::GiftOut, principal_proceeds_or_fmv: dec!(150.00), fee_usd: None }));
    let st = project(&[buy, out, recl], &StaticPrices::default(), &ProjectionConfig::default());
    assert!(st.disposals.is_empty()); // (c): non-recognition on the fee
    assert_eq!(st.removals.len(), 1);
    let removal_basis: rust_decimal::Decimal = st.removals[0].legs.iter().map(|l| l.basis).sum();
    let removal_sat: i64 = st.removals[0].legs.iter().map(|l| l.sat).sum();
    assert_eq!(removal_sat, 99_800);           // principal only; fee burned
    assert_eq!(removal_basis, dec!(60.00));     // FULL basis carries (200 fee-sats' $0.12 re-homed, not dropped)
    assert_eq!(st.stats.fee_sats_consumed, 200);
}

#[test]
fn self_transfer_fee_config_b_is_a_mini_disposition_recognition_record() {
    let mut prices = StaticPrices::default();
    prices.0.insert(time::macros::date!(2025 - 04 - 01), dec!(50000.00));
    let cold = WalletId::SelfCustody { label: "cold".into() };
    let buy = ev("BUY", datetime!(2025-03-01 00:00:00 UTC), EventPayload::Acquire(Acquire { sat: 100_000, usd_cost: dec!(60.00), fee_usd: dec!(0), basis_source: BasisSource::ExchangeProvided }));
    let out = ev("OUT", datetime!(2025-04-01 00:00:00 UTC), EventPayload::TransferOut(TransferOut { sat: 99_800, fee_sat: Some(200), dest_addr: None, txid: None }));
    let in_ev = LedgerEvent { id: EventId::import(Source::Swan, SourceRef::new("IN")), utc_timestamp: datetime!(2025-04-01 01:00:00 UTC), original_tz: offset!(+00:00), wallet: Some(cold.clone()), payload: EventPayload::TransferIn(TransferIn { sat: 99_800, src_addr: None, txid: None }) };
    let link = dec_ev(1, datetime!(2026-01-01 00:00:00 UTC), EventPayload::TransferLink(TransferLink { out_event: EventId::import(Source::Coinbase, SourceRef::new("OUT")), in_event_or_wallet: TransferTarget::InEvent(EventId::import(Source::Swan, SourceRef::new("IN"))) }));
    let st = project(&[buy, out, in_ev, link], &prices, &cfg_b());
    let mini: Vec<_> = st.disposals.iter().filter(|d| d.fee_mini_disposition).collect();
    assert_eq!(mini.len(), 1); // recognition record for the 200 fee-sats
    assert_eq!(st.holdings_by_wallet[&cold], 99_800);
    // (b) CONTRAST with (c): the fee basis rides the mini-disposition, so the destination lot is the
    // principal-only basis $59.88 (NOT re-homed). The mini-disposition is excluded from FR9 Σdisposed.
    assert_eq!(st.lots.iter().find(|l| l.wallet == cold).unwrap().usd_basis, dec!(59.88));
    assert_eq!(st.stats.fee_sats_consumed, 200);
}
```

- [ ] **Step 2: Run → FAIL → implement.** Thread the fold's `stats.fee_sats_consumed` accumulator (already on `FoldStats`, M3 — `fold` keeps `&mut stats`; **no tuple return**). Factor a `consume_fee` helper used by `Op::SelfTransfer`/`Op::GiftOut`/`Op::Donate`; under (c) it returns a `FeeCarry` the caller **re-homes onto the surviving lot/leg (C1)**, under (b) it emits the mini-disposition and returns an empty carry:
```rust
/// Carried basis of the burned fee-sats, to be RE-HOMED onto a surviving destination lot / removal leg
/// under TP8 (c) so the FULL basis carries (C1). Under (b) this is empty (basis rode the mini-disposition).
#[derive(Default)]
struct FeeCarry { gain_basis: Usd, loss_basis: Option<Usd> }
impl FeeCarry {
    fn rehome_onto_lot(&self, lot: &mut Lot) {
        lot.usd_basis += self.gain_basis;
        if let (Some(dl), Some(l)) = (lot.dual_loss_basis.as_mut(), self.loss_basis) { *dl += l; }
    }
    fn rehome_onto_removal_leg(&self, leg: &mut RemovalLeg) { leg.basis += self.gain_basis; }
}

/// Consume `fee_sat` FIFO from the source pool, record them in the FR9 fee-sat home, and (per config)
/// either return their carried basis for re-homing (c) or emit a mini-disposition recognition record (b).
/// §7.1 totality: a fee shortfall raises `uncovered_disposal`, never panics.
fn consume_fee(
    pools: &mut PoolSet, key: &PoolKey, fee_sat: Sat,
    config: &ProjectionConfig, prices: &dyn PriceProvider, date: TaxDate,
    stats: &mut FoldStats, st: &mut LedgerState, ev: &EventId,
) -> FeeCarry {
    if fee_sat <= 0 { return FeeCarry::default(); }
    let (consumed, shortfall) = pools.consume_fifo(key, fee_sat);
    if shortfall > 0 {
        st.add_blocker(BlockerKind::UncoveredDisposal, Some(ev.clone()), format!("self-transfer/gift fee short by {shortfall} sat"));
    }
    stats.fee_sats_consumed += consumed.iter().map(|c| c.sat).sum::<Sat>(); // sole FR9 home
    match config.self_transfer_fee {
        FeeTreatment::TreatmentC => {
            let gain_basis: Usd = consumed.iter().map(|c| c.gain_basis).sum();
            let has_loss = consumed.iter().any(|c| c.loss_basis.is_some());
            let loss_basis = has_loss.then(|| consumed.iter().filter_map(|c| c.loss_basis).sum());
            FeeCarry { gain_basis, loss_basis } // caller re-homes onto the survivor (C1: full basis carries)
        }
        FeeTreatment::TreatmentB => {
            // mini-disposition recognition record; proceeds = FMV(fee_sat); basis rides it (NOT re-homed).
            if !consumed.is_empty() {
                let net = fmv_of(prices, date, fee_sat).unwrap_or(Usd::ZERO);
                let legs = make_disposal_legs(&consumed, net, date, st, ev); // reuse Task 5 builder
                st.disposals.push(Disposal { event: ev.clone(), kind: DisposeKind::Spend, disposed_at: date, legs, fee_mini_disposition: true });
            }
            FeeCarry::default()
        }
    }
}
```
Wire it into the three arms (fee consumed from the **source** pool key, AFTER the principal is consumed so FIFO order = "principal then fee"):
```rust
// Op::SelfTransfer (Task 8 built `relocated: Vec<Lot>` but has NOT pushed it yet):
let carry = consume_fee(&mut pools, &src_key, fee_sat.unwrap_or(0), config, prices, date, &mut stats, &mut st, &eff.id);
if let Some(last) = relocated.last_mut() { carry.rehome_onto_lot(last); } // C1: full basis → destination lot
for lot in relocated { pools.push_lot(pool_key(date, &dest), lot); }

// Op::GiftOut / Op::Donate (Task 9 built `legs: Vec<RemovalLeg>` but has NOT pushed the Removal yet):
let carry = consume_fee(&mut pools, &src_key, fee_sat.unwrap_or(0), config, prices, date, &mut stats, &mut st, &eff.id);
if let Some(last) = legs.last_mut() { carry.rehome_onto_removal_leg(last); } // C1: full basis → donee
st.removals.push(Removal { event: eff.id.clone(), kind, removed_at: date, legs, appraisal_required, donor_acquired_at });
```
Edge: a degenerate pure-fee transfer (`principal == 0`, no relocated lot / removal leg) has no survivor to carry onto — not reachable for a real `TransferLink`/gift, which always move principal > 0; documented, not silently lossy. `Op::SelfTransfer`/`GiftOut`/`Donate` must therefore carry `fee_sat` through to the fold (added to `Op` in Task 8/9).

- [ ] **Step 3: Run → PASS → gate + commit.**
```bash
cargo test -p btctax-core && cargo clippy --all-targets -p btctax-core -- -D warnings && cargo fmt --check
git commit -am "feat(core): TP8 self-transfer fee — default (c) non-taxable, config (b) mini-disposition; gift/donation fee by analogy"
```

---

### Task 12: 2025 basis transition (TP6/§7.4) — UniversalPool→PerWalletPool, Path A / Path B safe-harbor

**Files:** Create `src/project/transition.rs`; Modify `resolve.rs` (allocation effectiveness in pass 1 steps 2–4 + transition mode + allocation-Void adjudication; **rename `resolve`'s `_prices`/`_config` → `prices`/`config` — now USED**), `fold.rs` (factor `fold_event`; seed per-wallet pools at the boundary), **`project/mod.rs` (declare `pub mod transition;`** — I-2: no signature change needed here, `project` already passes `prices`/`config` to `resolve`). Test `tests/kat_tax.rs` + `tests/transition.rs`.

**Interfaces — Produces:** the full §7.4 machinery:
- Pre-2025 lots live in `PoolKey::Universal` (already routed via `pool_key`), consumed FIFO; a pre-2025 disposal note `pre2025_method_note` (advisory) is emitted once if any pre-2025 disposal occurs.
- At the **first effective event with tax-date ≥ 2025-01-01**, run `seed_transition`:
  - **Path A (default):** drain `Universal` remaining lots into their holding wallet's pool (`PoolKey::Wallet`), preserving `acquired_at` + basis, `basis_source = ReconstructedPerWallet`. (A lot's wallet is its `Lot.wallet`, which already reflects reconciled relocations.) Sats still in `pending_reconciliation` at the boundary are excluded (flagged).
  - **Path B (effective `SafeHarborAllocation`):** discard the Universal remainder and seed `PoolKey::Wallet` pools from `allocation.lots` — fresh lots whose `LotId = (allocation EventId, index)` (`basis_source = SafeHarborAllocated`).
- **Effectiveness** (pass-1 steps 2–4), re-evaluated every rebuild:
  1. Determine the **first 2025 disposition** made-date reference: earliest tax-date among 2025 effective `Dispose{Sell|Spend}`, `GiftOut`, `Donate`, and §3.11 transfer-to-another-taxpayer; under TP8 (b) also a self-transfer fee-sat mini-disposition. A confirmed self-transfer (default (c)) does **not** count. A 2025 `TransferOut` still in `pending_reconciliation` does **not** count (provisional; raises `unmatched_outflows`).
  2. **Deadline guard (method-keyed, vs the allocation's made-date = its `utc_timestamp` on the §6.1 calendar-date basis):** `ActualPosition` barred at the **earlier of** (a) first-2025-disposition date and (b) `TY2025_RETURN_DUE` (2026-04-15); `ProRata` barred at the **later of** (a)/(b), and additionally requires its method description to predate 2025-01-01 (modeled as: `ProRata` always assumes the pre-2025 method description prong is attested via `timely_allocation_attested`, else timebar). If made-date is past the bar → `safe_harbor_timebar` (advisory) and the allocation is **inert → Path A**, **unless `timely_allocation_attested == true`** (bypasses both prongs).
  3. **Conservation guard:** `Σ allocation.sat == snap.held_sat` AND `Σ allocation.usd_basis == snap.basis`, where `snap = transition::universal_snapshot(&timeline, prices, config)` (I-1, below). Failure → `safe_harbor_unconservable` (hard) and inert. **Acyclic:** `universal_snapshot` folds ONLY pre-2025 events and NEVER seeds the transition, so it depends only on pre-2025 history — never on any allocation or on effectiveness — and effectiveness reads only its two totals (§7.2: not circular).
  4. **Capital-asset guard (§4.02):** assumed satisfied for a personal investor (no dealer flag in Phase 1); a future dealer flag would set it hard.
  5. **Irrevocable once effective:** a `VoidDecisionEvent` targeting an **effective** allocation → `decision_conflicts`; targeting an **inert** allocation → the Void applies (allocation dropped, stays Path A).
- Multiple allocations: if more than one is effective → `decision_conflicts`; otherwise the single effective one governs (`TransitionMode::PathB`).

- [ ] **Step 1: Failing tests `tests/transition.rs`** — concrete `#[test]`s (I-3), mirroring the Tasks 7–11 patterns. Path A vs Path B is distinguished by the surviving lots' `basis_source` (`ReconstructedPerWallet` vs `SafeHarborAllocated`); effectiveness by the presence/absence of `safe_harbor_timebar`/`safe_harbor_unconservable`/`decision_conflicts`.
```rust
use btctax_core::event::*;
use btctax_core::identity::*;
use btctax_core::price::StaticPrices;
use btctax_core::project::{project, ProjectionConfig, FeeTreatment};
use btctax_core::state::*;
use rust_decimal_macros::dec;
use time::macros::{date, datetime, offset};

fn cb() -> WalletId { WalletId::Exchange { provider: "cb".into(), account: "m".into() } }
fn cold() -> WalletId { WalletId::SelfCustody { label: "cold".into() } }
fn imp(src: Source, src_ref: &str, ts: time::OffsetDateTime, w: WalletId, p: EventPayload) -> LedgerEvent {
    LedgerEvent { id: EventId::import(src, SourceRef::new(src_ref)), utc_timestamp: ts, original_tz: offset!(+00:00), wallet: Some(w), payload: p }
}
fn dec_ev(seq: u64, ts: time::OffsetDateTime, p: EventPayload) -> LedgerEvent {
    LedgerEvent { id: EventId::decision(seq), utc_timestamp: ts, original_tz: offset!(+00:00), wallet: None, payload: p }
}
fn buy(src_ref: &str, ts: time::OffsetDateTime, sat: i64, cost: rust_decimal::Decimal) -> LedgerEvent {
    imp(Source::Coinbase, src_ref, ts, cb(), EventPayload::Acquire(Acquire { sat, usd_cost: cost, fee_usd: dec!(0), basis_source: BasisSource::ExchangeProvided }))
}
fn sell(src_ref: &str, ts: time::OffsetDateTime, sat: i64, proceeds: rust_decimal::Decimal) -> LedgerEvent {
    imp(Source::Coinbase, src_ref, ts, cb(), EventPayload::Dispose(Dispose { sat, usd_proceeds: proceeds, fee_usd: dec!(0), kind: DisposeKind::Sell }))
}
fn alloc(seq: u64, made: time::OffsetDateTime, method: AllocMethod, attested: bool, lots: Vec<AllocLot>) -> LedgerEvent {
    dec_ev(seq, made, EventPayload::SafeHarborAllocation(SafeHarborAllocation { lots, as_of_date: date!(2025 - 01 - 01), method, timely_allocation_attested: attested }))
}
fn alloc_lot(w: WalletId, sat: i64, basis: rust_decimal::Decimal, acq: time::Date) -> AllocLot {
    AllocLot { wallet: w, sat, usd_basis: basis, acquired_at: acq }
}
fn has(st: &LedgerState, k: BlockerKind) -> bool { st.blockers.iter().any(|b| b.kind == k) }

// (i) ActualPosition: a first-2025 disposition BEFORE the made-date bars at the earlier-of -> inert + timebar + Path A.
#[test]
fn actual_position_barred_by_earlier_first_disposition_is_inert_pathA() {
    let evs = vec![
        buy("B", datetime!(2024-06-01 00:00:00 UTC), 100_000, dec!(60.00)),
        sell("S", datetime!(2025-02-01 00:00:00 UTC), 40_000, dec!(30.00)),                 // first 2025 disposition
        alloc(1, datetime!(2025-03-01 00:00:00 UTC), AllocMethod::ActualPosition, false,    // made AFTER the sell
              vec![alloc_lot(cb(), 100_000, dec!(60.00), date!(2024 - 06 - 01))]),
    ];
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(has(&st, BlockerKind::SafeHarborTimebar));            // advisory
    assert!(!has(&st, BlockerKind::SafeHarborUnconservable));     // conservation passed; only the bar tripped
    assert_eq!(st.holdings_by_wallet[&cb()], 60_000);
    assert!(st.lots.iter().all(|l| l.basis_source == BasisSource::ReconstructedPerWallet)); // Path A governs
}

// (ii) ActualPosition: no 2025 disposition, made-date AFTER 2026-04-15 (return-due prong) -> inert + timebar.
#[test]
fn actual_position_barred_by_return_due_date_is_inert() {
    let evs = vec![
        buy("B", datetime!(2024-06-01 00:00:00 UTC), 100_000, dec!(60.00)),
        buy("B25", datetime!(2025-02-01 00:00:00 UTC), 50_000, dec!(40.00)),                // a 2025 ACQUIRE triggers the seed but is NOT a disposition
        alloc(1, datetime!(2026-05-01 00:00:00 UTC), AllocMethod::ActualPosition, false,    // made after 2026-04-15 (no 2025 disposition -> return-due prong governs)
              vec![alloc_lot(cb(), 100_000, dec!(60.00), date!(2024 - 06 - 01))]),
    ];
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(has(&st, BlockerKind::SafeHarborTimebar));         // barred at 2026-04-15 (return-due prong)
    // Path A governs: the pre-2025 lot is reconstructed; NO lot is safe-harbor-seeded (the 2025 buy is a normal lot).
    assert!(st.lots.iter().any(|l| l.basis_source == BasisSource::ReconstructedPerWallet));
    assert!(st.lots.iter().all(|l| l.basis_source != BasisSource::SafeHarborAllocated));
}

// (iii) Attestation bypasses BOTH prongs -> Path B governs (no timebar fires).
#[test]
fn attestation_bypasses_the_bar_path_b_governs() {
    let evs = vec![
        buy("B", datetime!(2024-06-01 00:00:00 UTC), 100_000, dec!(60.00)),
        sell("S", datetime!(2025-02-01 00:00:00 UTC), 40_000, dec!(30.00)),
        alloc(1, datetime!(2025-03-01 00:00:00 UTC), AllocMethod::ActualPosition, true,     // attested
              vec![alloc_lot(cb(), 50_000, dec!(30.00), date!(2024 - 06 - 01)),
                   alloc_lot(cb(), 50_000, dec!(30.00), date!(2024 - 07 - 01))]),
    ];
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(!has(&st, BlockerKind::SafeHarborTimebar));            // attestation suppresses it
    assert!(!has(&st, BlockerKind::SafeHarborUnconservable));
    assert!(st.lots.iter().all(|l| l.basis_source == BasisSource::SafeHarborAllocated)); // Path B
    assert_eq!(st.holdings_by_wallet[&cb()], 60_000);
}

// (iv) A confirmed self-transfer (default (c)) dated before the made-date does NOT trip prong (a).
#[test]
fn confirmed_self_transfer_does_not_trip_the_bar() {
    let evs = vec![
        buy("B", datetime!(2024-06-01 00:00:00 UTC), 100_000, dec!(60.00)),
        imp(Source::Coinbase, "OUT", datetime!(2025-01-15 00:00:00 UTC), cb(), EventPayload::TransferOut(TransferOut { sat: 100_000, fee_sat: None, dest_addr: None, txid: None })),
        imp(Source::Swan, "IN", datetime!(2025-01-15 01:00:00 UTC), cold(), EventPayload::TransferIn(TransferIn { sat: 100_000, src_addr: None, txid: None })),
        dec_ev(1, datetime!(2026-01-01 00:00:00 UTC), EventPayload::TransferLink(TransferLink { out_event: EventId::import(Source::Coinbase, SourceRef::new("OUT")), in_event_or_wallet: TransferTarget::InEvent(EventId::import(Source::Swan, SourceRef::new("IN"))) })),
        // made-date after the self-transfer but BEFORE 2026-04-15; (c) self-transfer is no disposition -> bar = return-due -> effective.
        // The coins were on `cb` at 2025-01-01 (the 2025 self-transfer moves them later), so the allocation assigns cb.
        alloc(2, datetime!(2025-06-01 00:00:00 UTC), AllocMethod::ActualPosition, false,
              vec![alloc_lot(cb(), 100_000, dec!(60.00), date!(2024 - 06 - 01))]),
    ];
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(!has(&st, BlockerKind::SafeHarborTimebar));         // effective: the (c) self-transfer did NOT trip prong (a)
    assert!(!has(&st, BlockerKind::UncoveredDisposal));
    assert!(st.disposals.is_empty());                           // the self-transfer is non-taxable (TP7)
    assert_eq!(st.holdings_by_wallet[&cold()], 100_000);        // coins relocated cb -> cold post-seed
}

// (v) Conservation mismatch -> HARD safe_harbor_unconservable, NOT timebar; falls back to Path A.
#[test]
fn conservation_mismatch_is_hard_unconservable_not_timebar() {
    let evs = vec![
        buy("B", datetime!(2024-06-01 00:00:00 UTC), 100_000, dec!(60.00)),
        sell("S", datetime!(2025-06-01 00:00:00 UTC), 30_000, dec!(24.00)),                 // made-date is BEFORE this -> bar not tripped
        alloc(1, datetime!(2025-02-01 00:00:00 UTC), AllocMethod::ActualPosition, false,
              vec![alloc_lot(cb(), 90_000, dec!(54.00), date!(2024 - 06 - 01))]),           // Σsat 90k != 100k
    ];
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(has(&st, BlockerKind::SafeHarborUnconservable));      // hard
    assert!(!has(&st, BlockerKind::SafeHarborTimebar));           // the bar was not the failure
    assert!(st.lots.iter().all(|l| l.basis_source == BasisSource::ReconstructedPerWallet)); // Path A fallback
}

// (vi) Void of an EFFECTIVE allocation -> decision_conflicts; the allocation STAYS in force (irrevocable, §7.4(2)).
#[test]
fn void_of_effective_allocation_is_a_decision_conflict() {
    let evs = vec![
        buy("B", datetime!(2024-06-01 00:00:00 UTC), 100_000, dec!(60.00)),
        sell("S", datetime!(2025-06-01 00:00:00 UTC), 30_000, dec!(24.00)),
        alloc(1, datetime!(2025-02-01 00:00:00 UTC), AllocMethod::ActualPosition, true,     // effective
              vec![alloc_lot(cb(), 50_000, dec!(30.00), date!(2024 - 06 - 01)),
                   alloc_lot(cb(), 50_000, dec!(30.00), date!(2024 - 07 - 01))]),
        dec_ev(2, datetime!(2026-01-01 00:00:00 UTC), EventPayload::VoidDecisionEvent(VoidDecisionEvent { target_event_id: EventId::decision(1) })),
    ];
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(has(&st, BlockerKind::DecisionConflict));
    assert!(st.lots.iter().all(|l| l.basis_source == BasisSource::SafeHarborAllocated)); // Path B still governs
}

// (vii) Void of an INERT allocation -> the void APPLIES (no decision_conflicts); stays Path A.
#[test]
fn void_of_inert_allocation_applies_no_conflict() {
    let evs = vec![
        buy("B", datetime!(2024-06-01 00:00:00 UTC), 100_000, dec!(60.00)),
        sell("S", datetime!(2025-02-01 00:00:00 UTC), 40_000, dec!(30.00)),                 // bars the ActualPosition alloc made later
        alloc(1, datetime!(2025-03-01 00:00:00 UTC), AllocMethod::ActualPosition, false,    // inert (timebar)
              vec![alloc_lot(cb(), 100_000, dec!(60.00), date!(2024 - 06 - 01))]),
        dec_ev(2, datetime!(2026-01-01 00:00:00 UTC), EventPayload::VoidDecisionEvent(VoidDecisionEvent { target_event_id: EventId::decision(1) })),
    ];
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(!has(&st, BlockerKind::DecisionConflict));            // void of an inert/revocable allocation is valid
    assert!(st.lots.iter().all(|l| l.basis_source == BasisSource::ReconstructedPerWallet)); // Path A
}

// (viii) Re-evaluation is a pure function of the SET: a ReclassifyOutflow that creates a first-2025 disposition
//        before the made-date flips the SAME allocation effective -> inert deterministically.
#[test]
fn reclassify_creating_a_disposition_flips_effective_to_inert() {
    let base = vec![
        buy("B", datetime!(2024-06-01 00:00:00 UTC), 100_000, dec!(60.00)),
        imp(Source::Coinbase, "OUT", datetime!(2025-02-01 00:00:00 UTC), cb(), EventPayload::TransferOut(TransferOut { sat: 40_000, fee_sat: None, dest_addr: None, txid: None })),
        sell("TRIG", datetime!(2025-09-01 00:00:00 UTC), 1_000, dec!(1.00)),                // a post-made-date seed trigger
        alloc(1, datetime!(2025-03-01 00:00:00 UTC), AllocMethod::ActualPosition, false,
              vec![alloc_lot(cb(), 100_000, dec!(60.00), date!(2024 - 06 - 01))]),
    ];
    // Variant 1: OUT left unclassified -> provisional (pending), no first-2025 disposition before made-date -> EFFECTIVE.
    let st1 = project(&base, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(st1.lots.iter().all(|l| l.basis_source == BasisSource::SafeHarborAllocated));
    assert!(!has(&st1, BlockerKind::SafeHarborTimebar));
    // Variant 2: reclassify OUT -> Dispose at 2025-02-01 (before made-date 2025-03-01) -> earlier-of bar trips -> INERT.
    let mut v2 = base.clone();
    v2.push(dec_ev(2, datetime!(2026-01-01 00:00:00 UTC), EventPayload::ReclassifyOutflow(ReclassifyOutflow {
        transfer_out_event: EventId::import(Source::Coinbase, SourceRef::new("OUT")),
        as_: OutflowClass::Dispose { kind: DisposeKind::Sell }, principal_proceeds_or_fmv: dec!(35.00), fee_usd: None })));
    let st2 = project(&v2, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(has(&st2, BlockerKind::SafeHarborTimebar));
    assert!(st2.lots.iter().all(|l| l.basis_source == BasisSource::ReconstructedPerWallet)); // Path A now
}

// (ix) Path A across mixed vintages: a pre-2025 lot reconstructed at the boundary keeps its acquired_at, so a
//      2025 disposition of it is LONG-TERM; conservation balances.
#[test]
fn path_a_mixed_vintages_post_2025_term_and_conservation() {
    let evs = vec![
        buy("OLD", datetime!(2024-06-01 00:00:00 UTC), 50_000, dec!(30.00)),
        buy("NEW", datetime!(2025-03-01 00:00:00 UTC), 50_000, dec!(40.00)),
        sell("S", datetime!(2025-08-01 00:00:00 UTC), 50_000, dec!(60.00)),                 // FIFO consumes the 2024 lot
    ];
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    let leg = &st.disposals[0].legs[0];
    assert_eq!(leg.term, Term::LongTerm);   // tacks from 2024-06-01 across the boundary
    assert_eq!(leg.basis, dec!(30.00));
    // conservation (computed inline; `conservation_report` itself lands in Task 13): in == disposed + held.
    let disposed: i64 = st.disposals.iter().flat_map(|d| &d.legs).map(|l| l.sat).sum();
    let held: i64 = st.lots.iter().map(|l| l.remaining_sat).sum();
    assert_eq!(disposed + held, 100_000);
    assert_eq!(st.holdings_by_wallet[&cb()], 50_000);
}

// (x) §6.1 calendar-date boundary: a UTC-2025 disposition whose original_tz date is 2024 is PRE-2025 (counts
//     toward neither the first-2025-disposition trigger nor the seed), so a 2025 allocation stays effective.
#[test]
fn calendar_date_boundary_keeps_a_2024_local_disposition_pre_2025() {
    let evs = vec![
        buy("B", datetime!(2024-06-01 00:00:00 UTC), 100_000, dec!(60.00)),
        // 2025-01-01 02:00 UTC is 2024-12-31 in UTC-05:00 -> a PRE-2025 disposal (Universal pool; pre2025_method_note).
        LedgerEvent { id: EventId::import(Source::Coinbase, SourceRef::new("PRE")), utc_timestamp: datetime!(2025-01-01 02:00:00 UTC), original_tz: offset!(-05:00), wallet: Some(cb()),
            payload: EventPayload::Dispose(Dispose { sat: 20_000, usd_proceeds: dec!(15.00), fee_usd: dec!(0), kind: DisposeKind::Sell }) },
        sell("S", datetime!(2025-09-01 00:00:00 UTC), 10_000, dec!(9.00)),                  // a real 2025 seed trigger, AFTER made-date
        alloc(1, datetime!(2025-03-01 00:00:00 UTC), AllocMethod::ActualPosition, false,
              vec![alloc_lot(cb(), 80_000, dec!(48.00), date!(2024 - 06 - 01))]),           // conserves to Universal-after-pre2025 sale
    ];
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(has(&st, BlockerKind::Pre2025MethodNote));            // the 2024-local sale folded pre-2025
    assert!(!has(&st, BlockerKind::SafeHarborTimebar));           // it is NOT a first-2025 disposition -> bar not tripped
    assert!(st.lots.iter().all(|l| l.basis_source == BasisSource::SafeHarborAllocated)); // Path B effective
}

// (xi) Path-A default fallback when NO allocation exists at all.
#[test]
fn path_a_is_the_default_with_no_allocation() {
    let evs = vec![
        buy("B", datetime!(2024-06-01 00:00:00 UTC), 100_000, dec!(60.00)),
        sell("S", datetime!(2025-06-01 00:00:00 UTC), 40_000, dec!(30.00)),
    ];
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(!has(&st, BlockerKind::SafeHarborTimebar) && !has(&st, BlockerKind::SafeHarborUnconservable));
    assert!(st.lots.iter().all(|l| l.basis_source == BasisSource::ReconstructedPerWallet)); // Path A
    assert_eq!(st.holdings_by_wallet[&cb()], 60_000);
}
```

- [ ] **Step 2: Run → FAIL → implement** `transition.rs`, factor `fold_event`, and wire `resolve`. Concrete skeleton (completeness of Tasks 7/11):

**(a) `fold_event` factoring + the boundary seed hook (`fold.rs`).** Lift Task 4–11's per-event `match &eff.op { … }` body into a free `fold_event(eff, prices, config, &mut pools, &mut st, &mut stats)` so BOTH pass-2 `fold` and the pass-1 `universal_snapshot` run the **identical** arms (this is what makes the conservation pre-fold provably match pass 2 — I-1). `fold` gains a one-shot boundary seed:
```rust
pub fn fold(mut res: Resolution, prices: &dyn PriceProvider, config: &ProjectionConfig) -> LedgerState {
    sort_canonical(&mut res.timeline);
    let mut st = LedgerState { blockers: std::mem::take(&mut res.blockers), ..Default::default() };
    let mut pools = PoolSet::default();
    let mut stats = FoldStats::default();
    let mut seeded = false;
    for eff in &res.timeline {
        if !seeded && eff.date() >= TRANSITION_DATE {
            transition::seed_transition(&res.transition, &mut pools, &mut st); // Path A drain / Path B seed, ONCE
            seeded = true;
        }
        fold_event(eff, prices, config, &mut pools, &mut st, &mut stats);
    }
    finalize(&mut st, pools, stats); // if no ≥2025 event ever seeds, Universal lots remain (carry their wallet)
    st
}
```
(`fold.rs` adds `use crate::conventions::TRANSITION_DATE; use crate::project::transition;`, and `fold_event` is declared `pub(crate)` so `transition::universal_snapshot` can reuse it.)

**(b) `transition.rs` — the pre-2025 snapshot + the seed.**
```rust
use crate::conventions::{Sat, Usd, TRANSITION_DATE};
use crate::event::BasisSource;
use crate::price::PriceProvider;
use crate::project::fold::fold_event;            // the SHARED per-event dispatcher (a)
use crate::project::pools::{PoolKey, PoolSet};
use crate::project::resolve::{sort_canonical, Eff, TransitionMode};
use crate::state::{FoldStats, LedgerState, Lot};
use crate::ProjectionConfig;

/// Σ held sat + Σ basis remaining in the single Universal pool at the 2025-01-01 boundary.
pub struct UniversalSnapshot { pub held_sat: Sat, pub basis: Usd }

/// I-1: a TRANSITION-FREE fold of ONLY the pre-2025 effective timeline into the Universal pool. Reuses the
/// exact pass-2 `fold_event` (so it cannot diverge) and NEVER seeds — so it depends only on pre-2025 history
/// and can be called from pass-1 effectiveness evaluation without infinite regress. Lives in `transition.rs`.
pub fn universal_snapshot(timeline: &[Eff], prices: &dyn PriceProvider, config: &ProjectionConfig) -> UniversalSnapshot {
    let mut pre: Vec<Eff> = timeline.iter().filter(|e| e.date() < TRANSITION_DATE).cloned().collect();
    sort_canonical(&mut pre); // same canonical FIFO order pass 2 uses
    let mut pools = PoolSet::default();
    let mut sink = LedgerState::default(); // discarded; we only read the pool residue
    let mut stats = FoldStats::default();
    for eff in &pre { fold_event(eff, prices, config, &mut pools, &mut sink, &mut stats); }
    let lots = pools.pools.get(&PoolKey::Universal).map(Vec::as_slice).unwrap_or(&[]);
    UniversalSnapshot {
        held_sat: lots.iter().map(|l| l.remaining_sat).sum(),
        basis: lots.iter().map(|l| l.usd_basis).sum(),
    }
}

/// Seed the per-wallet pools at the boundary, exactly once (called by `fold`).
pub fn seed_transition(mode: &TransitionMode, pools: &mut PoolSet, _st: &mut LedgerState) {
    // Take the Universal remainder out of the pool set.
    let universal: Vec<Lot> = pools.pools.remove(&PoolKey::Universal).unwrap_or_default();
    match mode {
        TransitionMode::PathA => {
            // Reconstruct: each remaining Universal lot moves to ITS holding wallet's pool, basis/acquired_at kept.
            for mut lot in universal.into_iter().filter(|l| l.remaining_sat > 0) {
                lot.basis_source = BasisSource::ReconstructedPerWallet;
                let key = PoolKey::Wallet(lot.wallet.clone());
                pools.push_lot(key, lot);
            }
        }
        TransitionMode::PathB { seed } => {
            // Discard the Universal remainder; seed fresh per-wallet lots from the effective allocation.
            for lot in seed.iter().cloned() {
                let key = PoolKey::Wallet(lot.wallet.clone());
                pools.push_lot(key, lot);
            }
        }
    }
}
```

**(c) `resolve` wiring (`resolve.rs`, steps 2–4; `_prices`/`_config` → `prices`/`config`).** After building the effective `timeline` (Task 7) and the classification maps (Task 8): compute `first_2025_disposition`, evaluate each `SafeHarborAllocation`, set `Resolution.transition`, and adjudicate allocation-targeting Voids.
```rust
// (1) earliest tax-date among the 2025 effective ops that ARE dispositions (provisional/pending OUTs and
//     confirmed (c) self-transfers excluded; under (b) the self-transfer fee mini-disposition counts).
let first_2025_disposition: Option<TaxDate> = timeline.iter()
    .filter(|e| e.date() >= TRANSITION_DATE)
    .filter(|e| is_disposition_op(&e.op, config)) // Dispose/GiftOut/Donate (+ (b) SelfTransfer fee)
    .map(|e| e.date())
    .min();

// (3-prereq) the pre-2025 Universal snapshot is allocation-INDEPENDENT — compute it ONCE (acyclic, I-1).
let snap = crate::project::transition::universal_snapshot(&timeline, prices, config);

let mut effective: Vec<(EventId, Vec<Lot>)> = Vec::new();   // (allocation id, pre-built seed lots)
for (_seq, d) in &decisions {
    if voided.contains(&d.id) { continue; }
    let EventPayload::SafeHarborAllocation(a) = &d.payload else { continue };
    let made = tax_date(d.utc_timestamp, d.original_tz);
    // (2) method-keyed deadline bar (earlier-of for ActualPosition, later-of for ProRata); attestation bypasses.
    let due = crate::conventions::TY2025_RETURN_DUE;
    let bar = match a.method {
        AllocMethod::ActualPosition => min_opt(first_2025_disposition, Some(due)), // earlier-of
        AllocMethod::ProRata        => max_opt(first_2025_disposition, Some(due)), // later-of
    };
    let timebarred = (!a.timely_allocation_attested && bar.map(|b| made > b).unwrap_or(false))
        // ProRata also needs its pre-2025 method description (modeled as the attestation); unattested => barred.
        || (a.method == AllocMethod::ProRata && !a.timely_allocation_attested);
    // (3) conservation vs the pre-2025 Universal snapshot.
    let alloc_sat: Sat = a.lots.iter().map(|l| l.sat).sum();
    let alloc_basis: Usd = a.lots.iter().map(|l| l.usd_basis).sum();
    let unconservable = alloc_sat != snap.held_sat || alloc_basis != snap.basis;
    // severity + effectiveness (conservation is HARD; timebar is ADVISORY; attestation can't bypass conservation).
    if unconservable {
        blockers.push(Blocker { kind: BlockerKind::SafeHarborUnconservable, event: Some(d.id.clone()), detail: "allocation != Universal remainder at 2025-01-01".into() });
        continue; // inert -> Path A
    }
    if timebarred {
        blockers.push(Blocker { kind: BlockerKind::SafeHarborTimebar, event: Some(d.id.clone()), detail: "allocation made past its §5.02(4) bar".into() });
        continue; // inert -> Path A
    }
    let seed = a.lots.iter().enumerate().map(|(i, l)| Lot {
        lot_id: LotId { origin_event_id: d.id.clone(), split_sequence: i as u32 },
        wallet: l.wallet.clone(), acquired_at: l.acquired_at,
        original_sat: l.sat, remaining_sat: l.sat, usd_basis: l.usd_basis,
        basis_source: BasisSource::SafeHarborAllocated,
        dual_loss_basis: None, donor_acquired_at: None, basis_pending: false,
    }).collect();
    effective.push((d.id.clone(), seed));
}
// (5) Void of an EFFECTIVE allocation -> decision_conflicts; of an inert/absent one the Void already applied (1a).
for v in allocation_voids /* collected in 1a, deferred from Task 7 */ {
    if effective.iter().any(|(id, _)| *id == v.target) {
        blockers.push(Blocker { kind: BlockerKind::DecisionConflict, event: Some(v.void_id.clone()), detail: "void targets an effective SafeHarborAllocation".into() });
    } else {
        voided.insert(v.target.clone()); // inert: the void applies (drops it; stays Path A)
    }
}
let transition = match effective.len() {
    0 => TransitionMode::PathA,
    1 => TransitionMode::PathB { seed: effective.pop().unwrap().1 },
    _ => { blockers.push(Blocker { kind: BlockerKind::DecisionConflict, event: None, detail: "multiple effective SafeHarborAllocations".into() }); TransitionMode::PathA }
};
Resolution { timeline, transition, blockers }
```
with the small helpers `min_opt`/`max_opt` (an `Option<TaxDate>` earlier-of/later-of where `None` = "no first-disposition prong"), `is_disposition_op(op, config)` (true for `Op::Dispose`/`GiftOut`/`Donate`, and the (b) `Op::SelfTransfer` fee mini-disposition; false for `PendingOut` and (c) `SelfTransfer`), and the `pre2025_method_note` advisory emitted once in `fold_event` when a `Dispose`/`Removal` consumes from `PoolKey::Universal`. ST/LT across the boundary is automatic: `acquired_at` is preserved by both seed paths, and `is_long_term` reads it. **`resolve` evaluates effectiveness BEFORE returning; effectiveness reads only `universal_snapshot` (pre-2025) + the allocation + `first_2025_disposition` — none of which depend on `transition` — so there is no regress (I-1/§7.2).**

- [ ] **Step 3: Run → PASS → gate + commit.**
```bash
cargo test -p btctax-core && cargo clippy --all-targets -p btctax-core -- -D warnings && cargo fmt --check
git commit -am "feat(core): 2025 basis transition (TP6/§7.4) — UniversalPool→PerWallet, Path A/B safe-harbor + guards"
```

---

### Task 13: Conservation invariants + property tests + FR9 `conservation_report`

**Files:** Create `src/project/conservation.rs` (or extend `state.rs`); Modify `lib.rs` (`pub fn conservation_report`). Test `tests/properties.rs`.

**Interfaces — Consumes:** `LedgerState` (incl. its `stats: FoldStats` field — `sigma_in`/`fee_sats_consumed`/`sigma_pending`, populated in `finalize`, M3). **Produces:** `pub fn conservation_report(&LedgerState) -> ConservationReport` implementing the FR9 identity, and proptest harnesses for the §13 properties.

`ConservationReport`:
```rust
pub struct ConservationReport {
    pub sigma_in: Sat,          // Acquire + Income + classified GiftReceived (externally-sourced only)
    pub sigma_disposed: Sat,    // Disposal legs where !fee_mini_disposition (Sell/Spend)
    pub sigma_removed: Sat,     // Removal legs (Gift/Donation)
    pub sigma_held: Sat,        // Σ lots remaining_sat
    pub sigma_fee_sats: Sat,    // sole home for network-fee sats (FR9)
    pub sigma_pending: Sat,     // pending_reconciliation principal + fee
    pub balanced: bool,         // sigma_in == disposed + removed + held + fee + pending (iff no uncovered_disposal)
    pub has_uncovered: bool,
}
```
Because `Σ in` (externally-sourced acquisitions) and `Σ fee_sats` are not directly reconstructable from `LedgerState` alone post-fold, they are read from the **`LedgerState.stats: FoldStats` field** that the fold already populates in `finalize` (M3: a field, NOT a `(LedgerState, FoldStats)` tuple — `project -> LedgerState` stays stable). `stats` is part of `LedgerState`'s `PartialEq` since it is a deterministic function of the events, so the determinism tests still hold. `conservation_report` is a pure function of `&LedgerState`:
```rust
pub fn conservation_report(st: &LedgerState) -> ConservationReport {
    let sigma_disposed = st.disposals.iter().filter(|d| !d.fee_mini_disposition).flat_map(|d| &d.legs).map(|l| l.sat).sum();
    let sigma_removed = st.removals.iter().flat_map(|r| &r.legs).map(|l| l.sat).sum();
    let sigma_held = st.lots.iter().map(|l| l.remaining_sat).sum();
    let has_uncovered = st.blockers.iter().any(|b| b.kind == BlockerKind::UncoveredDisposal);
    let (sigma_in, sigma_fee_sats, sigma_pending) = (st.stats.sigma_in, st.stats.fee_sats_consumed, st.stats.sigma_pending);
    let balanced = !has_uncovered
        && sigma_in == sigma_disposed + sigma_removed + sigma_held + sigma_fee_sats + sigma_pending;
    ConservationReport { sigma_in, sigma_disposed, sigma_removed, sigma_held, sigma_fee_sats, sigma_pending, balanced, has_uncovered }
}
```

- [ ] **Step 1: Failing property tests `tests/properties.rs`** — concrete generators (M2) that **synthesize `TransferLink`-confirmed fee'd self-transfers** so the Σbasis invariant actually covers the C1 path, and **real assertion bodies**.
```rust
use proptest::prelude::*;
use btctax_core::event::*;
use btctax_core::identity::*;
use btctax_core::price::StaticPrices;
use btctax_core::project::{project, ProjectionConfig};
use rust_decimal::Decimal;
use time::macros::{datetime, offset};

fn wal_a() -> WalletId { WalletId::Exchange { provider: "cb".into(), account: "m".into() } }
fn wal_b() -> WalletId { WalletId::SelfCustody { label: "cold".into() } }

/// One generated op against a single source wallet (post-2025).
#[derive(Debug, Clone)]
enum Step {
    Acquire { sat: i64, cents: i64 },
    Dispose { sat: i64, cents: i64 },
    /// A fee'd self-transfer A->B (the C1 path): principal `sat`, on-chain `fee`.
    SelfXfer { sat: i64, fee: i64 },
}

fn arb_step() -> impl Strategy<Value = Step> {
    prop_oneof![
        (1_000i64..5_000_000, 1i64..2_000_000).prop_map(|(sat, cents)| Step::Acquire { sat, cents }),
        (1_000i64..5_000_000, 1i64..2_000_000).prop_map(|(sat, cents)| Step::Dispose { sat, cents }),
        (1_000i64..5_000_000, 0i64..500).prop_map(|(sat, fee)| Step::SelfXfer { sat, fee }),
    ]
}

/// Materialize a step list into a well-formed event vector (unique source_refs / decision_seqs).
fn build(steps: &[Step]) -> Vec<LedgerEvent> {
    let mut evs = Vec::new();
    let (mut seq, ts) = (0u64, datetime!(2025-03-01 00:00:00 UTC));
    for (i, s) in steps.iter().enumerate() {
        match s {
            Step::Acquire { sat, cents } => evs.push(LedgerEvent {
                id: EventId::import(Source::Coinbase, SourceRef::new(format!("A{i}"))),
                utc_timestamp: ts, original_tz: offset!(+00:00), wallet: Some(wal_a()),
                payload: EventPayload::Acquire(Acquire { sat: *sat, usd_cost: Decimal::new(*cents, 2), fee_usd: Decimal::ZERO, basis_source: BasisSource::ExchangeProvided }),
            }),
            Step::Dispose { sat, cents } => evs.push(LedgerEvent {
                id: EventId::import(Source::Coinbase, SourceRef::new(format!("D{i}"))),
                utc_timestamp: ts, original_tz: offset!(+00:00), wallet: Some(wal_a()),
                payload: EventPayload::Dispose(Dispose { sat: *sat, usd_proceeds: Decimal::new(*cents, 2), fee_usd: Decimal::ZERO, kind: DisposeKind::Sell }),
            }),
            Step::SelfXfer { sat, fee } => {
                let (out_ref, in_ref) = (format!("O{i}"), format!("I{i}"));
                evs.push(LedgerEvent { id: EventId::import(Source::Coinbase, SourceRef::new(out_ref.clone())), utc_timestamp: ts, original_tz: offset!(+00:00), wallet: Some(wal_a()),
                    payload: EventPayload::TransferOut(TransferOut { sat: *sat, fee_sat: Some(*fee), dest_addr: None, txid: None }) });
                evs.push(LedgerEvent { id: EventId::import(Source::Swan, SourceRef::new(in_ref.clone())), utc_timestamp: ts, original_tz: offset!(+00:00), wallet: Some(wal_b()),
                    payload: EventPayload::TransferIn(TransferIn { sat: *sat, src_addr: None, txid: None }) });
                seq += 1;
                evs.push(LedgerEvent { id: EventId::decision(seq), utc_timestamp: datetime!(2026-01-01 00:00:00 UTC), original_tz: offset!(+00:00), wallet: None,
                    payload: EventPayload::TransferLink(TransferLink { out_event: EventId::import(Source::Coinbase, SourceRef::new(out_ref)), in_event_or_wallet: TransferTarget::InEvent(EventId::import(Source::Swan, SourceRef::new(in_ref))) }) });
            }
        }
    }
    evs
}

/// General mix (acquires / covered-or-not disposes / fee'd self-transfers). The `has_uncovered` guard
/// in the test handles the cases where a random Dispose exceeds holdings.
fn arb_events() -> impl Strategy<Value = Vec<LedgerEvent>> {
    prop::collection::vec(arb_step(), 1..8).prop_map(|s| build(&s))
}

/// No basis-pending paths (no income-missing / unknown-basis gifts) AND no disposals/removals — only acquires
/// and fee'd self-transfers — so the residual basis is exactly the acquired basis: the precise C1 check.
fn arb_events_no_pending_basis() -> impl Strategy<Value = Vec<LedgerEvent>> {
    let step = prop_oneof![
        (1_000i64..5_000_000, 1i64..2_000_000).prop_map(|(sat, cents)| Step::Acquire { sat, cents }),
        (1_000i64..5_000_000, 0i64..500).prop_map(|(sat, fee)| Step::SelfXfer { sat, fee }),
    ];
    prop::collection::vec(step, 1..8).prop_map(|s| build(&s))
}

proptest! {
    #[test]
    fn conservation_holds_when_no_uncovered(evs in arb_events()) {
        let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
        let r = btctax_core::conservation_report(&st);
        if !r.has_uncovered {
            prop_assert_eq!(r.sigma_in, r.sigma_disposed + r.sigma_removed + r.sigma_held + r.sigma_fee_sats + r.sigma_pending);
            prop_assert!(r.balanced);
        }
    }
    #[test]
    fn no_negative_remainders_ever(evs in arb_events()) {
        let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
        prop_assert!(st.lots.iter().all(|l| l.remaining_sat >= 0));
        prop_assert!(st.holdings_by_wallet.values().all(|&h| h >= 0));
    }
    #[test]
    fn sigma_lot_basis_conserved_through_feed_self_transfers(evs in arb_events_no_pending_basis()) {
        // C1: with only acquires + fee'd self-transfers, NO basis may be dropped — Σ remaining lot basis must
        // equal Σ acquired basis EXACTLY (the pre-fix bug leaked the fee-sats' fragment, e.g. $60.00 -> $59.88).
        let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
        let acquired: Decimal = evs.iter().filter_map(|e| match &e.payload {
            EventPayload::Acquire(a) => Some(a.usd_cost + a.fee_usd), _ => None }).sum();
        let remaining: Decimal = st.lots.iter().map(|l| l.usd_basis).sum();
        prop_assert_eq!(acquired, remaining);
    }
}
```
Plus a deterministic **golden KAT** over a hand-built mixed scenario (buy + income + fee'd self-transfer + gift + sell across the 2025 boundary) asserting pinned holdings/disposals/removals/income + `conservation_report(&st).balanced == true`.

- [ ] **Step 2: Run → FAIL → implement `conservation_report`** (reads `st.stats`, code above) — `FoldStats` is already a `LedgerState` field (M3), populated by `finalize`/the arms across Tasks 4–12.
- [ ] **Step 3: Run → PASS.** `cargo test -p btctax-core --test properties`
- [ ] **Step 4: Full gate + commit.**
```bash
cargo test -p btctax-core && cargo clippy --all-targets -p btctax-core -- -D warnings && cargo fmt --check
git commit -am "feat(core): conservation invariants + property tests + FR9 conservation_report"
```

---

## Self-Review — spec coverage map (every TP / FR / §7.x → its task)

**Tax positions (§2):**
- **TP1** (property; sale/spend realize; gift/donation non-recognition) → Tasks 5 (Dispose), 9 (Removal zero gain).
- **TP2** (basis = cost + acquisition fee; disposition fee reduces proceeds) → Task 4 (Acquire basis = cost+fee), Task 5 (`net = proceeds − fee`).
- **TP3** (income = FMV at dominion; FMV=basis; HP next day; business tag) → Task 6 (+ Task 8 ClassifyInbound::Income).
- **TP4** (HP day-after-acquisition → disposition day; >1yr LT, `original_tz` date) → Task 0 (`is_long_term`/`tax_date`), exercised Tasks 5/9/12.
- **TP5** (default FIFO; specific-ID-ready) → Task 5 (`consume_fifo`), Task 4 (`LotMethod::Fifo` hook).
- **TP6** (per-wallet basis from 2025; Path A default / Path B safe harbor) → Task 12.
- **TP7** (self-transfers non-taxable; lots carry basis + HP) → Task 8 (`Op::SelfTransfer`).
- **TP8** (self-transfer fee default (c) / config (b); gift/donation fee by analogy) → Task 11 (+ `ProjectionConfig::default()` enforces (c) in Task 4).
- **TP9** (wash-sale N/A) → no engine rule needed (out of scope; no code path treats crypto as a wash-sale security) — explicitly nothing to implement.
- **TP10** (gift out/donation = non-recognition removal; capture basis+FMV+ST/LT+appraisal) → Task 9.
- **TP11** (received-gift dual basis + conditional tacking; unknown-basis fallback/flag) → Task 10.

**Functional requirements (§3):**
- **FR1** (import; atomic batch; idempotent; `ImportConflict` on changed row) → Task 3 (`append_import_batch`, `fingerprint`).
- **FR2** (BTC-only filter; unknown → `Unclassified`) — adapter-side (Plan 3); core consumes `Unclassified` as an inert blocker → Task 4 (`Op::Unclassified`). *(Filtering itself is out of core scope by design; core models the resulting events.)*
- **FR3** (FMV resolution; `Missing` blocks income + downstream) → Task 6 (`fmv_missing` gating) + Task 10 (gift fallback). *(Adapter resolves export/dataset FMV at ingest; core handles `Missing` + `ManualFmv` (Task 7) + gift fallback.)*
- **FR4** (deterministic projection of lots/holdings/disposals/removals/income/queue/blockers) → Tasks 4–13 (`project` + `LedgerState`).
- **FR5** (wallets: exchange + self-custody as basis pools) → Task 1 (`WalletId`), Tasks 4/8/12 (per-wallet pools).
- **FR6** (reconciliation: pending + unknown-basis; `TransferLink`/`ReclassifyOutflow`/`ClassifyInbound`/conflict accept-reject) → Tasks 7 (conflicts), 8 (transfers/inbound), 9 (reclassify).
- **FR7** (2025 transition: `reconstruct-2025` Path A default / `allocate-2025` Path B; Path B iff effective) → Task 12.
- **FR8** (corrections: `VoidDecisionEvent`; irrevocable effective allocation/non-revocable → `decision_conflicts`) → Tasks 7 (general void) + 12 (allocation void adjudication).
- **FR9** (`verify` integrity: sat conservation identity, `Σ in` definition, fee-sat sole home) → Task 13 (`conservation_report`). *(CLI `verify` wiring + source-balance cross-check are Plan 4; core provides the computation.)*
- **FR10** (export-snapshot / backup-key) → `btctax-store` (Plan 1) + CLI (Plan 4); **no core code** (NFR2 exception lives in the store). Explicitly out of core scope.

**Projection (§7):**
- **§7.1** (pure/total contract; blocker severity) → Task 4 (`project` signature, `BlockerKind::severity`), totality in Task 5 (`uncovered_disposal`).
- **§7.2** (two-pass; canonical order; determinism) → Task 4 (sort + scaffold), Task 7 (staged pass 1), Task 12 (steps 2–4).
- **§7.3** (fold rules — every variant) → Tasks 4 (Acquire), 5 (Dispose/totality), 6 (Income/FMV gating), 7 (conflict/void/manual-fmv/classify-raw), 8 (transfers/inbound), 9 (gift/donate), 10 (dual-basis), 11 (TP8 fee).
- **§7.4** (2025 transition guards, time-bar advisory vs unconservable hard, provisional effectiveness, made-date vs as_of_date, pending exclusion, allocation-seeded LotId) → Task 12.

**Non-functional:** NFR4 → Task 4 (`tests/determinism.rs`) + every task's permutation discipline; NFR5 → Task 0 (`Sat=i64`, `Usd=Decimal`, no floats); NFR6 (all state in events, log = source of truth) → Task 3 (event log) + the pure projection (no hidden state). NFR1/NFR2/NFR3/NFR7/NFR8 are store/CLI concerns (Plans 1/4) — no core code.

**Cross-task type consistency check (verified at write time):** `Sat=i64`/`Usd=Decimal`/`TaxDate=time::Date` (Task 0) are used uniformly. `EventId`/`Source`/`SourceRef`/`Fingerprint`/`WalletId`/`LotId` (Task 1) are referenced identically in `event.rs` (Task 2), `persistence.rs` (Task 3), `pools.rs`/`state.rs`/`resolve.rs`/`fold.rs` (Task 4+). `EventPayload` variants (Task 2) match the `build_op`/`fold` arms across Tasks 4–12. `Consumed` (Task 4 `pools.rs`) carries exactly the fields the disposal/removal/relocation builders read (Tasks 5/8/9/10). `ProjectionConfig`/`FeeTreatment` (Task 4) drive Task 11. `LedgerState`/`Disposal`/`Removal`/`Lot`/`Blocker` shapes (Task 4 `state.rs`) are populated, never reshaped, by later tasks. `PriceProvider` (Task 4) is consumed by Tasks 10 (fallback), 11 (config-(b) fee FMV), and 12 (pre-2025 `universal_snapshot`).
- **`resolve` signature (I-2):** `resolve(events: &[LedgerEvent], prices: &dyn PriceProvider, config: &ProjectionConfig) -> Resolution` — fixed from Task 4 onward (params are `_prices`/`_config` through Task 7, **used** in Task 12). `project` (Task 4 `project/mod.rs`) calls `resolve::resolve(events, prices, config)` then `fold::fold(resolution, prices, config)`; `project`'s own signature is unchanged (`-> LedgerState`). This is the ONLY internal-signature change in the plan; there are no other `resolve` call sites.
- **`FoldStats` on `LedgerState` (M3):** `pub stats: FoldStats` is a FIELD of `LedgerState` from Task 4 `state.rs` (zero by `Default`, in `PartialEq`/`Eq`), accumulated during the fold and committed in `finalize`. `project`/`fold`/`finalize` therefore return/accept `LedgerState` (+ a `stats: FoldStats` arg to `finalize`) — **never** a `(LedgerState, FoldStats)` tuple. `fee_sats_consumed` lands in Task 11; `sigma_in` in Tasks 4/6/8; `sigma_pending` is computed in `finalize`.
- **`TransitionMode` (Task 4, finalized Task 12):** `PathA | PathB { seed: Vec<Lot> }` (not `Copy`; the `(())` placeholder is gone — N4). `resolve` builds the `seed`; `fold`/`transition::seed_transition` consume it.
- **`fold_event` (Task 12 factoring):** the per-event `match &eff.op` body (built up Tasks 4–11) is lifted to `pub(crate) fn fold_event(eff, prices, config, &mut PoolSet, &mut LedgerState, &mut FoldStats)`, reused verbatim by both pass-2 `fold` and pass-1 `transition::universal_snapshot` — so the conservation pre-fold cannot diverge from pass 2.

**Requirements that could NOT be placed in a task:** none. (FR2 filtering, FR3 ingest-time FMV resolution, FR9 CLI `verify` wiring, and FR10 export are explicitly out of `btctax-core`'s scope per spec §5 — they live in `btctax-adapters`/`btctax-cli`/`btctax-store`; core implements the parts it owns: consuming `Unclassified`, handling `Missing`/`ManualFmv`/gift-fallback, and the conservation computation. TP9 wash-sale requires no engine rule by design.)

## Notes for Plans 3–4 (and FOLLOWUPS candidates)
- **Plan 3 `btctax-adapters`:** implements the bundled `PriceProvider` dataset (§9.2 daily close) + the four parsers that mint `LedgerEvent`s (assigning `source_ref`, calling `core::persistence::fingerprint`/`append_import_batch`).
- **Plan 4 `btctax-cli`:** wires `Vault::conn()` → `core::persistence`, drives reconciliation (mints decisions via `append_decision`), runs `project` for `holdings`/`lots`/`events`, and implements `verify` over `conservation_report` + per-source running-balance cross-checks (FR9), plus `export-snapshot`/`backup-key` (store).
- **Open simplifications to record in FOLLOWUPS:** `original_tz` modeled as a fixed `UtcOffset` (no IANA/DST database) — adequate at day granularity, but a near-midnight DST-transition timestamp could land on the neighboring calendar date; the Feb-29 anniversary → Feb-28 convention; `EventId::canonical()` uses `|` delimiters (source_ref sanitization deferred; structured identity columns are the authoritative store); the ProRata "method-description predates 2025-01-01" prong is modeled via `timely_allocation_attested` (the app cannot inspect the user's books); `appraisal_required` is a safe >$5k FMV over-flag (precise §170(f)(11) trigger is Phase 2).

## Fold record (round 1)

Reviews folded (persisted verbatim before folding, per STANDARD_WORKFLOW §2):
`reviews/plan-foundation-02-core-tax-round-1.md` (1 Critical) and `reviews/plan-foundation-02-core-engineering-round-1.md` (3 Important + 6 Minor). Mapping finding → fix:

| Finding | Sev | Fix (where) |
|---|---|---|
| **C1** — TP8(c) self-transfer/gift/donation network fee **dropped** the fee-sats' basis fragment (destination $59.88, not $60.00) | Critical (tax) | **Full basis carries.** Task 11 rewritten: a `consume_fee` helper consumes the `fee_sat` FIFO from the source pool into the FR9 fee-sat home and, under default (c), returns a `FeeCarry` the caller **re-homes onto the surviving relocated lot (`Op::SelfTransfer`) / last `Removal` leg (`Op::GiftOut`/`Op::Donate`)** — destination holds principal sats with the FULL consumed basis. Task 8/9 arms restructured to build `relocated`/`legs` then apply the fee step before pushing. The (c) KAT now **asserts `st.lots[0].usd_basis == dec!(60.00)`** (+ a gift/donation analogue asserting Σ removal-leg basis == `$60.00`); a (b) contrast KAT asserts the destination stays `$59.88` (basis rode the mini-disposition). Task 13's Σbasis proptest generator now synthesizes `TransferLink`-confirmed fee'd self-transfers and asserts Σ remaining lot basis == Σ acquired basis. **TP8 default stays (c); the "do not flip" guards are untouched.** |
| **I-1** — Task 12 conservation "dry pre-fold" unspecified | Important (eng) | Concrete `transition::universal_snapshot(timeline, prices, config) -> UniversalSnapshot{ held_sat, basis }`: a transition-free fold of the **pre-2025-filtered** timeline that reuses the shared `fold_event` and **never seeds** — so it depends only on pre-2025 history and `resolve` can call it during pass-1 effectiveness with no regress (acyclicity stated). `fold_event` factoring added so pass-2 and the snapshot share identical arms. |
| **I-2** — `resolve`/`project` signature | Important (eng) | `resolve` → `resolve(events, prices, config) -> Resolution` (Tasks 4 & 7 signatures + Task 4 `project` call site updated). `project/mod.rs` added to Task 12's Files list. Recorded in the Cross-task consistency section; verified the sole call site matches. |
| **I-3** — Task 12 had no concrete failing tests | Important (eng) | The (i)–(x) comment block replaced with 11 runnable `#[test]`s in `tests/transition.rs` covering method-keyed earlier-/later-of bar, attestation bypass, provisional (pending-outflow) effectiveness, reclassify-driven effective↔inert re-evaluation, void-of-effective (conflict) vs void-of-inert (applies), hard `safe_harbor_unconservable` vs advisory `safe_harbor_timebar`, mixed-vintage ST/LT, §6.1 calendar-date boundary, and Path-A default. |
| **M1** — Debug-string blocker sort key | Minor | `finalize` sorts by derived `Ord` of `(kind, Option<EventId>, detail)` — total order, no `format!`. |
| **M2** — inert Task 13 property tests / undefined generators | Minor | Concrete `arb_events()` / `arb_events_no_pending_basis()` (with fee'd self-transfers) defined; all three proptests have real assertion bodies. |
| **M3** — `(LedgerState, FoldStats)` tuple option | Minor | Removed: `FoldStats` is a `LedgerState` field from Task 4; `project -> LedgerState` stays stable; `finalize` commits it. |
| **M4** — `dec_ev` only in `corrections.rs` | Minor | `dec_ev` duplicated into `kat_tax.rs`'s header (Task 8) with a note that integration files are separate crates. |
| **M5** — `unreachable!()` `id_for` stub in Task 3 | Minor | Task 3 Step 3 now rebuilds `EventId` inline from the persisted identity columns; `id_for` and the separate "real impl" step removed. |
| **M6** — unchecked Decimal ops vs §7.1 totality | Minor | `split_pro_rata` uses `checked_mul`/`checked_div` (divide-first fallback); `fmv_of` uses checked ops returning `None` on overflow. |
| **tax M2 / M3** (notes) | Minor | Added invariant notes: donation `appraisal_required`/FMV/`term` always preserved for Phase 2 (Task 9); FMV-missing/unknown-basis legs are provisional and gated by the hard blocker, never final (Task 6). |

**Left as documented-acceptable (no change):** tax M1 (Feb-29→Feb-28), tax M4 (ProRata attestation prong), tax M5 (conservation-over-flag), and nits N1–N4 (N4 `PathB(())` was nonetheless removed while finalizing `TransitionMode`).
