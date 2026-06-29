# btctax-cli (CLI + Reconciliation) Implementation Plan — Foundation Plan 4 of 4 (v1)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `btctax-cli`, the **final Phase-1 crate** that ties the offline US Bitcoin-tax app together. It wires the three shipped crates — `btctax-store` (encrypted vault + session), `btctax-adapters` (ingest), `btctax-core` (event log + pure projection) — behind a command surface (§11): `init`, `import`, `verify` (FR9), `report`/`show`, the `reconcile` subcommands that emit the **decision events** core consumes (§7.2), `config` (TP8 (c)/(b), `LotMethod`), `export-snapshot` (FR10), and `backup-key`. The CLI **displays and orchestrates**; it computes no tax math — determinism (NFR4) and exact arithmetic (NFR5) are guaranteed by the engine; the CLI just appends events and renders the projection.

**Architecture:** A thin binary over a testable library.
1. **`lib` (the crate root `btctax_cli`).** Every command is a pure-ish library function over an explicit `vault_path: &Path` + an **injected `&Passphrase`** + an injected `now: OffsetDateTime` (the decision-event clock seam, §6.2). Functions open a `Session` (the `btctax-store::Vault` wrapper), mutate via `btctax_core::persistence`, `save`, and **return structured outcomes** (`ImportReport`, `LedgerState`, `VerifyReport`, `EventId`, …) — never `println!`. Rendering is separate (`render`), so tests assert on data and on rendered strings over **temp vaults + synthetic fixtures**.
2. **`Session` (`session.rs`).** Owns one open `Vault`; exposes `conn()`, `save()`, `config()`, and `project()` (= `load_all` → bundled `BundledPrices` → `project`). The passphrase is always a parameter — production resolves it (prompt/env) in `main`; tests construct `Passphrase::new(..)` directly.
3. **Decisions (`reconcile`).** Each reconcile subcommand builds exactly one `EventPayload` decision variant and calls `append_decision`, then `save`. Every decision is **idempotent at the engine level** (append-only; re-projectable; `decision_seq` monotonic) and references its target by a **canonical `EventId` string** parsed by `eventref` (round-trips `EventId::canonical()`, tolerating `|` inside `source_ref`).
4. **`main.rs`.** Pure clap-4 derive dispatch: parse args → resolve passphrase → call one lib function → render → set the exit code (non-zero on FR9 hard blockers). No logic.

**The CLI ORCHESTRATES, never re-implements.** No parsing (that is adapters), no projection/lot math (that is core), no crypto/atomic-write (that is store). It calls `ingest_files_bundled`, `append_import_batch`, `append_decision`, `load_all`, `project`, `conservation_report`, `Vault::{create,open,save,export_snapshot,backup_key}` — and renders. FR10 CSV (the projected ledger as CSV) is the **one** artifact the CLI itself writes, alongside the store's `snapshot.sqlite`.

**Tech Stack:** Rust (edition 2021, rust-version 1.74 — workspace pins). Path deps: `btctax-store`, `btctax-core`, `btctax-adapters`. `clap` 4.5 (derive; pinned) for the binary; `rpassword` 7.3 (secure interactive passphrase) in `main` only; `rust_decimal` 1.36 (parse USD args exactly — NFR5; same pin as core); `time` 0.3 (`macros`/`parsing`/`formatting`; date args + the decision clock); `csv` 1.3 (FR10 CSV export); `serde_json` 1 (the `classify-raw` typed-payload input); `rusqlite` 0.31 (the `cli_config` side-table over the vault's live `Connection`; same pin/features as store/core — `bundled` is provided transitively by store); `thiserror` 1 (typed `CliError`, matching the other crates). Dev: `tempfile` 3, `rust_decimal_macros` 1.36, `time` `macros`.

## Global Constraints
(Spec `design/SPEC_foundation.md`; every task implicitly includes these. Values are verbatim from the spec.)

- **PRIVACY (CRITICAL).** The real exchange exports + vault live under `~/Documents/BitcoinTax/ReadOnly` and the user's home — **OUTSIDE this repo**. They MUST NEVER be read by this crate, its tests, this plan, or any tool invocation, and MUST NEVER be committed. **Every test uses a temp vault (`tempfile::tempdir()`) + SYNTHETIC export fixtures** (the confirmed real §9.1 header names with invented values only, built in-test as CSV strings, exactly as `btctax-adapters/tests/integration.rs` does). No test reads a real file; no command in a test points at a real path.
- **NFR2 Encryption at rest (verbatim).** "only the PGP vault is written automatically; no plaintext DB except the explicit `export-snapshot`." → the CLI writes plaintext (`snapshot.sqlite` + FR10 CSVs) **only** in `export-snapshot`; every other mutating command ends in `Vault::save` (encrypted, atomic). `backup-key` writes only the S2K-encrypted key.
- **NFR4 Determinism (verbatim).** "identical event *set* → identical ledger, invariant to storage/load order." The CLI never sorts/dedups/caches projection inputs; it passes `load_all` straight to `project`. **Decision events carry an injected `now`** (the §6.2 creation-time / safe-harbor made-date) so tests are deterministic; `main` supplies `OffsetDateTime::now_utc()`.
- **NFR5 Exact arithmetic.** USD args parse string→`rust_decimal::Decimal` (never float); BTC amounts on the CLI are entered as integer satoshis (`i64`). The CLI performs **no** money arithmetic — it forwards values to core and renders engine output.
- **NFR6 Auditability.** All ledger state lives as events; the CLI only ever **appends** (`append_import_batch` / `append_decision`) and re-projects. The single non-event persisted item is the projection-config side-table (`cli_config`: TP8 treatment + lot method) — a projection *input parameter*, not ledger state (see Task 2 / Open question O3).
- **NFR7 Single-user safety.** Each command opens its own `Session` (acquires the store `flock`); concurrent invocations fail fast with `StoreError::Locked`. The CLI surfaces that as a clear message, never a clobber.
- **TP8 — DEFAULT (c), USER-MANDATED.** `FeeTreatment::TreatmentC` is the persisted default and `ProjectionConfig::default()`; `config set fee-treatment b` is an explicit opt-in. **Never** make (b) a default or silently flip it.
- **FR9 hard vs advisory.** `verify` exits non-zero iff a **hard** blocker is open (`BlockerKind::severity() == Severity::Hard`); advisory blockers (`safe_harbor_timebar`, `unmatched_outflows`, `pre2025_method_note`) are reported but do not fail the run.
- **Path A is the default 2025 transition (FR7).** Path A needs **no event**; Path B is `reconcile safe-harbor allocate` emitting `SafeHarborAllocation`. The CLI never auto-elects Path B.
- **Out of scope (this crate).** Forms (8949/Sch D/8283/709/SE) and the optimizer (Phases 2/3, §15/§16); non-BTC assets; GUI; networked/online pricing; multi-user. The CLI surfaces Phase-1 state only.
- **Licensing:** workspace `license = "MIT OR Unlicense"`; `edition = "2021"`; `rust-version = "1.74"`.
- **Validation gate ("green"):** `cargo test -p btctax-cli` + `cargo clippy --all-targets -p btctax-cli -- -D warnings` + `cargo fmt --check` all green; plus 0 Critical / 0 Important on review.

## File Structure
```
Cargo.toml                          # [workspace] root — ADD "crates/btctax-cli" to members
crates/btctax-cli/
  Cargo.toml                        # pinned deps (path deps + clap/rpassword/csv/serde_json/rusqlite/…)
  src/lib.rs                        # pub API + CliError + module wiring + re-exports
  src/config.rs                     # cli_config side-table: CliConfig <-> ProjectionConfig (TP8 (c)/(b), LotMethod)
  src/session.rs                    # Session: Vault wrapper (create/open/save/conn/config/project) + passphrase seam
  src/eventref.rs                   # canonical EventId <-> string; WalletId / USD / date / kind arg parsers
  src/render.rs                     # text rendering of reports + LedgerState (FR9/report/show); FR10 CSV export
  src/cmd/mod.rs                    # `pub mod` wiring for the command fns
  src/cmd/init.rs                   # `init`  (FR + §8 key lifecycle)
  src/cmd/import.rs                 # `import` (FR1/FR2)
  src/cmd/inspect.rs               # `verify` (FR9) + `report`/`show` (FR4)
  src/cmd/reconcile.rs             # reconcile decision emitters (FR6/FR7/FR8, §7.2)
  src/cmd/admin.rs                  # `config`, `export-snapshot` (FR10), `backup-key`
  src/main.rs                       # clap-4 derive dispatch + passphrase resolution + exit codes
  tests/fixtures.rs                 # shared synthetic-export builders (real §9.1 headers, invented values)
  tests/init_import.rs              # init + import over a temp vault (FR1/FR2)
  tests/verify_report.rs            # verify (FR9) + report/show (FR4)
  tests/reconcile.rs                # each decision emitter round-trips through project (FR6/FR7/FR8)
  tests/end_to_end.rs              # init→import→verify→reconcile→report→verify (the §16.5 capstone)
```

**Public interface this plan PRODUCES (the binary + a reusable library):**
- `pub enum CliError` (`thiserror`): `Store`, `Core`, `Adapter`, `Sqlite`, `Csv`, `Io`, `BadEventRef`, `Usage`.
- `pub struct Session` — `create`/`open`/`conn`/`save`/`config`/`project`/`vault`.
- `pub struct CliConfig { fee_treatment: FeeTreatment, lot_method: LotMethod }` + `init_config_table`/`read_config`/`set_fee_treatment`.
- `pub mod eventref` — `parse_event_id`/`parse_wallet_id`/`parse_usd_arg`/`parse_date_arg`/`parse_income_kind`.
- `pub mod render` — `render_file_reports`/`render_verify`/`render_report`/`write_csv_exports` (+ `VerifyReport`).
- `pub mod cmd` — `init`, `import`, `verify`, `report`, and the reconcile emitters (`link_transfer`, `classify_inbound`, `reclassify_outflow`, `set_fmv`, `void`, `classify_raw`, `accept_conflict`, `reject_conflict`, `safe_harbor_allocate`, `safe_harbor_attest`), `set_config`, `export_snapshot`, `backup_key`.

**API ground-truth (verified against the shipped crates at write time):**
- store: `Vault::create(&Path,&Passphrase)`, `Vault::open(&Path,&Passphrase)`, `vault.conn() -> &Connection` (**no `conn_mut`** — core appenders take `&Connection`), `vault.save(&mut self)`, `vault.export_snapshot(&self,&Path) -> Result<PathBuf,_>` (writes **only** `snapshot.sqlite`), `vault.backup_key(&self,&Path)`; `Passphrase::new(String)`; `StoreError::{Locked,WrongPassphrase,AlreadyExists,InvalidVaultPath,…}`.
- core persistence: `init_schema(&Connection)`, `append_import_batch(&Connection,&[LedgerEvent]) -> Result<ImportReport,_>` (`ImportReport { appended, duplicates, conflicts }`), `append_decision(&Connection, EventPayload, OffsetDateTime, UtcOffset, Option<WalletId>) -> Result<EventId,_>`, `load_all(&Connection) -> Result<Vec<LedgerEvent>,_>`.
- core projection: `project(&[LedgerEvent], &dyn PriceProvider, &ProjectionConfig) -> LedgerState`; `conservation_report(&LedgerState) -> ConservationReport`; `ProjectionConfig { self_transfer_fee, lot_method }` (`Default` = `TreatmentC`+`Fifo`); `FeeTreatment::{TreatmentC,TreatmentB}`; `LotMethod::Fifo`.
- core state: `LedgerState { lots, holdings_by_wallet, disposals, removals, income_recognized, pending_reconciliation, blockers, stats }`; `Blocker { kind, event: Option<EventId>, detail }`; `BlockerKind::severity() -> Severity::{Hard,Advisory}`.
- core events/identity: `EventPayload::{TransferLink,ReclassifyOutflow,ClassifyInbound,ManualFmv,SafeHarborAllocation,SupersedeImport,RejectImport,VoidDecisionEvent,ClassifyRaw,…}`; `TransferTarget::{InEvent,Wallet}`; `OutflowClass::{Dispose{kind},GiftOut,Donate{appraisal_required}}`; `InboundClass::{Income{kind,fmv,business},GiftReceived{donor_basis,donor_acquired_at,fmv_at_gift}}`; `AllocLot{wallet,sat,usd_basis,acquired_at}`, `AllocMethod::{ActualPosition,ProRata}`; `EventId::{import,conflict,decision,canonical}`; `WalletId::{Exchange{provider,account},SelfCustody{label}}`; `Source::{Swan,Coinbase,Gemini,River}`; `SourceRef::new`; `Fingerprint(pub String)`; conventions `Sat=i64`, `Usd=Decimal`, `TaxDate=Date`, `TRANSITION_DATE=2025-01-01`, `TY2025_RETURN_DUE=2026-04-15`.
- adapters: `ingest_files_bundled(&[PathBuf]) -> Result<IngestBatch,_>` (`IngestBatch { events, reports }`, `FileReport { source,label,parsed_rows,btc_events,dropped_no_btc,unclassified }`); `BundledPrices::load()` (`impl PriceProvider`).

---

### Task 0: Workspace member + crate scaffold + `CliError` + clap stub

**Files:** Modify root `Cargo.toml` (members). Create `crates/btctax-cli/Cargo.toml`, `src/lib.rs`, `src/main.rs`.

**Interfaces — Produces:** the pinned deps; `CliError`; an empty `cmd`/module skeleton; a compiling no-op binary.

- [ ] **Step 1: Add the workspace member.** Edit root `Cargo.toml` `members` to `["crates/btctax-store", "crates/btctax-core", "crates/btctax-adapters", "crates/btctax-cli"]` (preserve `[workspace.package]`).

- [ ] **Step 2: `crates/btctax-cli/Cargo.toml`**
```toml
[package]
name = "btctax-cli"
version = "0.1.0"
edition.workspace = true
license.workspace = true
rust-version.workspace = true   # M3: enforce the workspace MSRV (1.74) like the other 3 crates

[lib]
name = "btctax_cli"
path = "src/lib.rs"

[[bin]]
name = "btctax"
path = "src/main.rs"

[dependencies]
btctax-core = { path = "../btctax-core" }
btctax-store = { path = "../btctax-store" }
btctax-adapters = { path = "../btctax-adapters" }
rust_decimal = { version = "1.36", default-features = false, features = ["std"] }
time = { version = "0.3", features = ["macros", "parsing", "formatting"] }
rusqlite = { version = "0.31", features = ["bundled"] } # cli_config side-table over the vault Connection
clap = { version = "4.5", features = ["derive"] }       # binary arg parsing (derive)
rpassword = "7.3"                                        # secure interactive passphrase (main only)
csv = "1.3"                                              # FR10 ledger CSV export
serde_json = "1"                                         # classify-raw typed-payload input
thiserror = "1"

[dev-dependencies]
tempfile = "3"
rust_decimal_macros = "1.36"
time = { version = "0.3", features = ["macros"] }
```
*(Pin note, mirroring the store/adapters R3 discipline: after the first `cargo build`, record the exact resolved `clap`/`rpassword`/`csv` versions in FOLLOWUPS. If a cited clap derive symbol differs from the pin — `#[derive(Parser)]`, `#[command(subcommand)]`, `#[arg(long)]`, `Subcommand`, `ValueEnum` — fix to the compiler before proceeding. `rusqlite` here MUST resolve to the **same** version store/core use so the `bundled` SQLite is unified; if cargo reports two `rusqlite` versions, align the pin to store's `Cargo.toml` and re-verify.)*

- [ ] **Step 3: Stub `src/lib.rs`**
```rust
//! btctax-cli: the CLI + reconciliation library that wires the encrypted vault (btctax-store),
//! ingest (btctax-adapters), and the pure projection (btctax-core) into the Phase-1 command surface
//! (spec §11). The library is I/O-explicit and deterministic; the binary (`main.rs`) is a thin clap
//! dispatch. PRIVACY: tests use only temp vaults + synthetic fixtures; no real user file is ever read.
pub mod cmd;
pub mod config;
pub mod eventref;
pub mod render;
pub mod session;

pub use config::CliConfig;
pub use session::Session;

#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error(transparent)]
    Store(#[from] btctax_store::StoreError),
    #[error(transparent)]
    Core(#[from] btctax_core::CoreError),
    #[error(transparent)]
    Adapter(#[from] btctax_adapters::AdapterError),
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// C1: `write_csv_exports` (Task 15) uses `?` on `csv::Writer` ops (→ `csv::Error`); `csv::Error`
    /// is its own type (NOT covered by `Io(#[from] io::Error)`, whose `From` goes the other way), so it
    /// needs its own variant or Task 15 will not compile.
    #[error("csv: {0}")]
    Csv(#[from] csv::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    /// A user-supplied event reference did not parse as a canonical `EventId` (eventref.rs).
    #[error("not a valid event reference: {0:?}")]
    BadEventRef(String),
    /// A CLI argument was malformed (bad USD/date/enum/wallet spec, or a contradictory flag set).
    #[error("usage: {0}")]
    Usage(String),
}
```

- [ ] **Step 4: Stub `src/main.rs`** (compiles; real dispatch arrives in Task 16)
```rust
//! Thin clap-4 dispatch over the btctax_cli library. Real subcommands are wired in Task 16.
fn main() {
    eprintln!("btctax: command dispatch is wired in Task 16");
    std::process::exit(2);
}
```

- [ ] **Step 5: Stub the modules so `lib.rs` compiles.** Create empty `src/cmd/mod.rs` (`// command fns added per task`), and one-line placeholder files for `config.rs`, `eventref.rs`, `render.rs`, `session.rs` each containing only a module doc-comment. (They are filled in Tasks 1–5; this keeps `pub mod` declarations resolving.)

- [ ] **Step 6: Gate + commit.**
```bash
cargo build -p btctax-cli && cargo clippy --all-targets -p btctax-cli -- -D warnings && cargo fmt --check
git add Cargo.toml crates/btctax-cli/Cargo.toml crates/btctax-cli/src
git commit -m "feat(cli): scaffold btctax-cli crate + CliError + workspace member"
```

---

### Task 1: `Session` — vault wrapper + passphrase seam + projection helper

**Files:** Create `src/session.rs`; Modify `src/lib.rs` (already wires `pub mod session;`).

**Interfaces — Consumes:** `btctax_store::{Vault, Passphrase}`, `btctax_core::persistence::{init_schema, load_all}`, `btctax_core::{project, LedgerState, ProjectionConfig}`, `btctax_adapters::BundledPrices`, `crate::config`, `CliError`. **Produces:** `Session::{create, open, conn, save, config, project, vault}` — the single seam every command opens; the passphrase is always a parameter (test-injectable).

- [ ] **Step 1: Failing tests in `src/session.rs`**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use btctax_store::Passphrase;

    fn pp() -> Passphrase {
        Passphrase::new("test-pass".into())
    }

    #[test]
    fn create_then_open_round_trips_over_a_temp_vault() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        {
            let _s = Session::create(&vault, &pp()).unwrap(); // schema + config table initialized + saved
        }
        // Re-open with the same passphrase: an empty ledger projects cleanly.
        let s = Session::open(&vault, &pp()).unwrap();
        let (state, _cfg) = s.project().unwrap();
        assert!(state.lots.is_empty());
        assert!(state.blockers.is_empty());
    }

    #[test]
    fn wrong_passphrase_is_surfaced_not_a_panic() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        Session::create(&vault, &pp()).unwrap();
        let err = Session::open(&vault, &Passphrase::new("nope".into())).unwrap_err();
        assert!(matches!(
            err,
            CliError::Store(btctax_store::StoreError::WrongPassphrase)
        ));
    }
}
```

- [ ] **Step 2: Run → FAIL.** `cargo test -p btctax-cli session`

- [ ] **Step 3: Implement `src/session.rs`**
```rust
//! `Session` wraps one open `btctax_store::Vault` and is the single seam every command opens. The
//! passphrase is ALWAYS a parameter — production resolves it in `main` (prompt/env); tests inject a
//! constructed `Passphrase`. `project()` runs the pure core projection over the bundled price dataset.
use crate::config::{self, CliConfig};
use crate::CliError;
use btctax_adapters::BundledPrices;
use btctax_core::persistence::{init_schema, load_all};
use btctax_core::{project, LedgerState, ProjectionConfig};
use btctax_store::{Passphrase, Vault};
use rusqlite::Connection;
use std::path::Path;

pub struct Session {
    vault: Vault,
}

impl Session {
    /// Create a brand-new encrypted vault, then initialize the core event schema and the CLI config
    /// table, and persist. (`Vault::create` already saved once; we re-save after the DDL.)
    pub fn create(vault_path: &Path, pp: &Passphrase) -> Result<Session, CliError> {
        let mut vault = Vault::create(vault_path, pp)?;
        init_schema(vault.conn())?;
        config::init_config_table(vault.conn())?;
        vault.save()?;
        Ok(Session { vault })
    }

    /// Open an existing vault (acquires the store single-instance lock; NFR7).
    pub fn open(vault_path: &Path, pp: &Passphrase) -> Result<Session, CliError> {
        Ok(Session {
            vault: Vault::open(vault_path, pp)?,
        })
    }

    /// Borrow the live in-memory SQLite handle (core appenders use interior mutability over `&Connection`).
    pub fn conn(&self) -> &Connection {
        self.vault.conn()
    }

    /// Persist the current DB image (encrypted, atomic; NFR2/NFR3).
    pub fn save(&mut self) -> Result<(), CliError> {
        self.vault.save()?;
        Ok(())
    }

    /// Borrow the vault for store-level operations (`export_snapshot` / `backup_key`).
    pub fn vault(&self) -> &Vault {
        &self.vault
    }

    /// The persisted projection config (TP8 treatment + lot method); default = (c)+FIFO if unset.
    pub fn config(&self) -> Result<CliConfig, CliError> {
        config::read_config(self.conn())
    }

    /// Load all events and run the pure deterministic projection (NFR4) over the bundled daily-close
    /// dataset (§9.2). Returns the resolved `ProjectionConfig` too (so `verify` can display it).
    pub fn project(&self) -> Result<(LedgerState, ProjectionConfig), CliError> {
        let events = load_all(self.conn())?;
        let cfg = self.config()?.to_projection();
        let prices = BundledPrices::load()?;
        let state = project(&events, &prices, &cfg);
        Ok((state, cfg))
    }
}
```

- [ ] **Step 4: Run → PASS.** `cargo test -p btctax-cli session` *(also drives Task 2's `config.rs` — implement that first if the compile fails on `crate::config`; the two tasks compile together.)*
- [ ] **Step 5: Gate + commit.**
```bash
cargo clippy --all-targets -p btctax-cli -- -D warnings && cargo fmt --check
git commit -am "feat(cli): Session vault wrapper + passphrase seam + projection helper"
```

---

### Task 2: `cli_config` side-table — `CliConfig` ⇄ `ProjectionConfig` (TP8 (c)/(b))

**Files:** Create `src/config.rs`; Modify `src/lib.rs` (re-exports `CliConfig`).

**Interfaces — Consumes:** `btctax_core::{FeeTreatment, LotMethod, ProjectionConfig}`, `rusqlite::Connection`, `CliError`. **Produces:** `CliConfig` (`Default` = TP8 (c) + FIFO), `init_config_table`, `read_config`, `set_fee_treatment`. Persists the projection-config knob in a `cli_config(key,value)` table inside the vault DB (rides the encrypted blob; NFR6 note in Global Constraints / Open question O3).

- [ ] **Step 1: Failing tests in `src/config.rs`**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use btctax_core::FeeTreatment;

    fn mem() -> rusqlite::Connection {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        init_config_table(&c).unwrap();
        c
    }

    #[test]
    fn default_is_treatment_c_user_mandated() {
        let c = mem();
        assert_eq!(read_config(&c).unwrap().fee_treatment, FeeTreatment::TreatmentC);
    }

    #[test]
    fn set_then_read_b_opt_in_round_trips() {
        let c = mem();
        set_fee_treatment(&c, FeeTreatment::TreatmentB).unwrap();
        assert_eq!(read_config(&c).unwrap().fee_treatment, FeeTreatment::TreatmentB);
        // and back to the mandated default
        set_fee_treatment(&c, FeeTreatment::TreatmentC).unwrap();
        assert_eq!(read_config(&c).unwrap().fee_treatment, FeeTreatment::TreatmentC);
    }

    #[test]
    fn to_projection_carries_treatment_and_fifo() {
        let c = mem();
        set_fee_treatment(&c, FeeTreatment::TreatmentB).unwrap();
        let proj = read_config(&c).unwrap().to_projection();
        assert_eq!(proj.self_transfer_fee, FeeTreatment::TreatmentB);
        assert!(matches!(proj.lot_method, btctax_core::LotMethod::Fifo));
    }
}
```

- [ ] **Step 2: Run → FAIL.** `cargo test -p btctax-cli config`

- [ ] **Step 3: Implement `src/config.rs`**
```rust
//! The CLI's persisted projection-config knob (TP8 self-transfer fee treatment + lot method), stored
//! in a `cli_config(key,value)` table inside the vault DB. It is a projection *input parameter*, not
//! ledger state (NFR6): the event log remains the sole source of truth; this only selects a swappable
//! rule. TP8 default is (c), USER-MANDATED — never default to (b).
use crate::CliError;
use btctax_core::{FeeTreatment, LotMethod, ProjectionConfig};
use rusqlite::{Connection, OptionalExtension};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CliConfig {
    pub fee_treatment: FeeTreatment,
    pub lot_method: LotMethod,
}

impl Default for CliConfig {
    fn default() -> Self {
        // DO NOT change: TP8 default is (c). Spec §2/TP8 + user memory forbid flipping it to (b).
        CliConfig {
            fee_treatment: FeeTreatment::TreatmentC,
            lot_method: LotMethod::Fifo,
        }
    }
}

impl CliConfig {
    pub fn to_projection(self) -> ProjectionConfig {
        ProjectionConfig {
            self_transfer_fee: self.fee_treatment,
            lot_method: self.lot_method,
        }
    }
}

pub fn init_config_table(conn: &Connection) -> Result<(), CliError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS cli_config (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
    )?;
    Ok(())
}

fn get(conn: &Connection, key: &str) -> Result<Option<String>, CliError> {
    Ok(conn
        .query_row("SELECT value FROM cli_config WHERE key=?1", [key], |r| {
            r.get::<_, String>(0)
        })
        .optional()?)
}

/// Read the persisted config, falling back to the (c)+FIFO default for any unset key (so a freshly
/// created vault, or a future-added key, reads as the safe default). `LotMethod` has the single Phase-1
/// variant `Fifo` (TP5); it is surfaced read-only until the Phase-3 optimizer adds specific-ID.
pub fn read_config(conn: &Connection) -> Result<CliConfig, CliError> {
    let mut cfg = CliConfig::default();
    if let Some(v) = get(conn, "fee_treatment")? {
        cfg.fee_treatment = match v.as_str() {
            "b" => FeeTreatment::TreatmentB,
            _ => FeeTreatment::TreatmentC, // any other/legacy value reads as the mandated default
        };
    }
    Ok(cfg)
}

pub fn set_fee_treatment(conn: &Connection, t: FeeTreatment) -> Result<(), CliError> {
    let v = match t {
        FeeTreatment::TreatmentC => "c",
        FeeTreatment::TreatmentB => "b",
    };
    conn.execute(
        "INSERT INTO cli_config(key,value) VALUES('fee_treatment',?1)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        [v],
    )?;
    Ok(())
}
```

- [ ] **Step 4: Run → PASS.** `cargo test -p btctax-cli config`
- [ ] **Step 5: Wire + gate + commit.** Confirm `pub use config::CliConfig;` is in `lib.rs`.
```bash
cargo clippy --all-targets -p btctax-cli -- -D warnings && cargo fmt --check
git commit -am "feat(cli): cli_config side-table (TP8 (c)/(b) + LotMethod) -> ProjectionConfig"
```

---

### Task 3: `init` — create encrypted vault, init schema, force key backup (§8)

**Files:** Create `src/cmd/init.rs`; Modify `src/cmd/mod.rs`.

**Interfaces — Consumes:** `Session::create`, `btctax_store::Passphrase`, `Vault::backup_key`, `CliError`. **Produces:** `cmd::init::run(vault_path, pp, key_backup_path) -> Result<(), CliError>` — creates the vault (schema + config initialized by `Session::create`) and **forces** the §8 key-backup step (a required output path, not optional).

- [ ] **Step 1: Failing tests in `src/cmd/init.rs`**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use btctax_store::Passphrase;

    #[test]
    fn init_creates_vault_key_and_forced_backup() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let backup = dir.path().join("backup/key.asc");
        run(&vault, &Passphrase::new("pw".into()), &backup).unwrap();
        assert!(vault.exists(), "vault.pgp written");
        assert!(dir.path().join("vault.key").exists(), "sidecar key written");
        assert!(backup.exists(), "forced key backup written");
    }

    #[test]
    fn init_refuses_to_clobber_an_existing_vault() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let backup = dir.path().join("k.asc");
        run(&vault, &Passphrase::new("pw".into()), &backup).unwrap();
        let err = run(&vault, &Passphrase::new("pw".into()), &backup).unwrap_err();
        assert!(matches!(
            err,
            CliError::Store(btctax_store::StoreError::AlreadyExists)
        ));
    }
}
```

- [ ] **Step 2: Run → FAIL.** `cargo test -p btctax-cli -- cmd::init`

- [ ] **Step 3: Implement `src/cmd/init.rs`**
```rust
//! `init` — create the encrypted vault (`Vault::create`), initialize the core event schema + CLI
//! config table (via `Session::create`), and FORCE the §8 key-backup step. The key-backup path is a
//! required argument: a vault with no backed-up key is unrecoverable, so `init` never skips it.
use crate::{CliError, Session};
use btctax_store::Passphrase;
use std::path::Path;

pub fn run(vault_path: &Path, pp: &Passphrase, key_backup_path: &Path) -> Result<(), CliError> {
    let session = Session::create(vault_path, pp)?;
    // §8 key lifecycle: a forced backup of the passphrase-protected key (HIGH-security write, owner-only).
    session.vault().backup_key(key_backup_path)?;
    Ok(())
}
```

- [ ] **Step 4: Wire `cmd/mod.rs`.** Add `pub mod init;`.
- [ ] **Step 5: Run → PASS.** `cargo test -p btctax-cli -- cmd::init`
- [ ] **Step 6: Gate + commit.**
```bash
cargo clippy --all-targets -p btctax-cli -- -D warnings && cargo fmt --check
git commit -am "feat(cli): init command (create vault + schema + forced key backup, §8)"
```

---

### Task 4: `import` — ingest → append batch → save; render reports (FR1/FR2)

**Files:** Create `src/cmd/import.rs`, start `src/render.rs` (the report renderer), `tests/fixtures.rs`, `tests/init_import.rs`; Modify `src/cmd/mod.rs`, `src/lib.rs`.

**Interfaces — Consumes:** `btctax_adapters::{ingest_files_bundled, IngestBatch, FileReport}`, `btctax_core::persistence::{append_import_batch, ImportReport}`, `Session`, `render::render_file_reports`. **Produces:** `cmd::import::run(vault_path, pp, &[PathBuf]) -> Result<(IngestBatch's reports, ImportReport), CliError>` plus `render::render_file_reports`. Re-importing unchanged rows is idempotent and a changed row appends one `ImportConflict` — both are core's job; the CLI surfaces the counts.

- [ ] **Step 1: Shared synthetic fixtures `tests/fixtures.rs`** (real §9.1 Coinbase headers, invented values; no real file)
```rust
//! SYNTHETIC export fixtures for CLI integration tests — real §9.1 header names, invented values only.
//! PRIVACY: no test ever reads ~/Documents/BitcoinTax/ReadOnly; these are written into a tempdir.
use std::path::{Path, PathBuf};

/// A Coinbase CSV (3-line preamble + real 13-col header) with a Buy(Acquire), a Sell(Dispose), and a
/// Send(TransferOut→pending). `dir` is a tempdir; returns the file path.
pub fn coinbase_buy_sell_send(dir: &Path) -> PathBuf {
    let p = dir.join("coinbase.csv");
    std::fs::write(
        &p,
        "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
cb-buy,2025-03-01 12:00:00 UTC,Buy,BTC,0.10000000,USD,84000.00,8400.00,8450.00,50.00,,,\r\n\
cb-sell,2025-06-15 12:00:00 UTC,Sell,BTC,0.02000000,USD,67500.00,1350.00,1340.00,10.00,,,\r\n\
cb-send,2025-06-20 12:00:00 UTC,Send,BTC,0.03000000,USD,68000.00,,,,,,bc1qsyntheticdest\r\n",
    )
    .unwrap();
    p
}

/// A Coinbase CSV with a single Buy only (self-contained USD; no price-dataset dependency).
pub fn coinbase_single_buy(dir: &Path) -> PathBuf {
    let p = dir.join("coinbase_buy.csv");
    std::fs::write(
        &p,
        "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
cb-buy,2025-03-01 12:00:00 UTC,Buy,BTC,0.10000000,USD,84000.00,8400.00,8450.00,50.00,,,\r\n\
cb-eth,2025-03-01 08:00:00 UTC,Buy,ETH,1.00000000,USD,2000.00,2000.00,2010.00,10.00,,,\r\n",
    )
    .unwrap();
    p
}
```

- [ ] **Step 2: Failing test `tests/init_import.rs`**
```rust
mod fixtures;
use btctax_cli::{cmd, Session};
use btctax_store::Passphrase;

fn pp() -> Passphrase {
    Passphrase::new("pw".into())
}

#[test]
fn import_appends_btc_events_and_reports_fr2_counts() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    let file = fixtures::coinbase_single_buy(dir.path());
    let (reports, import) = cmd::import::run(&vault, &pp(), &[file]).unwrap();

    // One Coinbase group; the ETH row is dropped (no BTC leg, FR2); the Buy is appended.
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].dropped_no_btc, 1);
    assert_eq!(import.appended, 1);
    assert_eq!(import.conflicts, 0);

    // Idempotent re-import (FR1): same rows → zero new appends, zero conflicts.
    let file2 = fixtures::coinbase_single_buy(dir.path());
    let (_r2, import2) = cmd::import::run(&vault, &pp(), &[file2]).unwrap();
    assert_eq!(import2.appended, 0);
    assert_eq!(import2.duplicates, 1);
    assert_eq!(import2.conflicts, 0);

    // The appended Acquire is visible to the projection.
    let s = Session::open(&vault, &pp()).unwrap();
    let (state, _) = s.project().unwrap();
    assert_eq!(state.lots.len(), 1);
}
```

- [ ] **Step 3: Run → FAIL.** `cargo test -p btctax-cli --test init_import`

- [ ] **Step 4: Implement `src/cmd/import.rs`**
```rust
//! `import <files…>` (FR1/FR2) — detect+parse via adapters, append the batch atomically into the
//! vault, and save. Idempotency + `ImportConflict` detection are core's job (`append_import_batch`);
//! the CLI surfaces the per-source FR2 counts (dropped/unclassified) and the append/dup/conflict tally.
use crate::{CliError, Session};
use btctax_adapters::{ingest_files_bundled, FileReport};
use btctax_core::persistence::{append_import_batch, ImportReport};
use btctax_store::Passphrase;
use std::path::{Path, PathBuf};

pub fn run(
    vault_path: &Path,
    pp: &Passphrase,
    files: &[PathBuf],
) -> Result<(Vec<FileReport>, ImportReport), CliError> {
    let batch = ingest_files_bundled(files)?; // adapters: detect→group→parse→normalize (FR2/FR3)
    let mut session = Session::open(vault_path, pp)?;
    let import = append_import_batch(session.conn(), &batch.events)?; // ATOMIC batch (FR1)
    session.save()?; // encrypted, atomic (NFR2/NFR3)
    Ok((batch.reports, import))
}
```

- [ ] **Step 5: Start `src/render.rs` with `render_file_reports`**
```rust
//! Text rendering of CLI outputs (FR9 verify, FR4 report/show) + FR10 CSV export. Pure string-building
//! over engine data — the CLI displays; the engine computes (NFR4/NFR5).
use btctax_core::persistence::ImportReport;
use btctax_adapters::FileReport;
use std::fmt::Write as _;

/// FR1/FR2: per-source drop/unclassified counts + the append/duplicate/conflict tally.
pub fn render_file_reports(reports: &[FileReport], import: &ImportReport) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "Import:");
    for r in reports {
        let _ = writeln!(
            out,
            "  {} [{}]: parsed {} rows -> {} BTC events ({} dropped no-BTC, {} unclassified)",
            r.source.tag(),
            r.label,
            r.parsed_rows,
            r.btc_events,
            r.dropped_no_btc,
            r.unclassified
        );
    }
    let _ = writeln!(
        out,
        "  appended {} | duplicates {} | NEW import-conflicts {}",
        import.appended, import.duplicates, import.conflicts
    );
    if import.conflicts > 0 {
        let _ = writeln!(
            out,
            "  ! resolve conflicts with `reconcile accept-conflict <id>` or `reject-conflict <id>` (see `verify`)"
        );
    }
    out
}
```

- [ ] **Step 6: Wire + gate + commit.** Add `pub mod import;` to `cmd/mod.rs`; confirm `pub mod render;` in `lib.rs`. `Source::tag()` is public (verified).
```bash
cargo test -p btctax-cli --test init_import
cargo clippy --all-targets -p btctax-cli -- -D warnings && cargo fmt --check
git commit -am "feat(cli): import command (ingest -> append_import_batch -> save) + FR2 report render"
```

---

### Task 5: `report`/`show` — projected `LedgerState` with `--year` (FR4)

**Files:** Extend `src/render.rs`; Create `src/cmd/inspect.rs` (the `report` half); Modify `src/cmd/mod.rs`; Create `tests/verify_report.rs` (report half).

**Interfaces — Consumes:** `Session::project`, `btctax_core::{LedgerState, WalletId, Term, …}`. **Produces:** `cmd::inspect::report(vault_path, pp, year: Option<i32>) -> Result<LedgerState, CliError>` and `render::render_report(&LedgerState, Option<i32>) -> String` (per-lot holdings; realized disposals proceeds/basis/gain/ST-LT; income; removals — realized sections year-filtered).

- [ ] **Step 1: Failing test in `tests/verify_report.rs`**
```rust
mod fixtures;
use btctax_cli::{cmd, render};
use btctax_store::Passphrase;

fn pp() -> Passphrase {
    Passphrase::new("pw".into())
}

#[test]
fn report_shows_lots_and_year_filtered_disposals() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    let file = fixtures::coinbase_buy_sell_send(dir.path());
    cmd::import::run(&vault, &pp(), &[file]).unwrap();

    let state = cmd::inspect::report(&vault, &pp(), Some(2025)).unwrap();
    // The Buy minus the Sell leaves a remaining lot; the Sell is a 2025 disposal.
    assert!(!state.lots.is_empty());
    assert_eq!(state.disposals.len(), 1);

    let text = render::render_report(&state, Some(2025));
    assert!(text.contains("Holdings"));
    assert!(text.contains("Disposals"));

    // A year with no realized events renders the sections empty (no panic, no disposals listed).
    let none = render::render_report(&state, Some(1999));
    assert!(none.contains("Disposals (year 1999): none"));
}
```

- [ ] **Step 2: Run → FAIL.** `cargo test -p btctax-cli --test verify_report report`

- [ ] **Step 3: Implement `cmd::inspect::report` (the report half of `inspect.rs`)**
```rust
//! `verify` (FR9) + `report`/`show` (FR4) — read-only inspection of the pure projection. `verify`
//! arrives in Task 6; this file starts with `report`.
use crate::{CliError, Session};
use btctax_core::LedgerState;
use btctax_store::Passphrase;
use std::path::Path;

/// FR4: project the ledger for display. `year` filters realized sections in the renderer; holdings are
/// always the current per-lot position.
pub fn report(vault_path: &Path, pp: &Passphrase, _year: Option<i32>) -> Result<LedgerState, CliError> {
    let session = Session::open(vault_path, pp)?;
    let (state, _cfg) = session.project()?;
    Ok(state)
}
```

- [ ] **Step 4: Extend `src/render.rs` with `render_report` + helpers**
```rust
use btctax_core::{
    DisposalLeg, LedgerState, RemovalLeg, Term, WalletId,
};

/// `exchange:provider:account` | `self:label` (the same grammar `eventref::parse_wallet_id` accepts).
pub fn wallet_label(w: &WalletId) -> String {
    match w {
        WalletId::Exchange { provider, account } => format!("exchange:{provider}:{account}"),
        WalletId::SelfCustody { label } => format!("self:{label}"),
    }
}

fn term_str(t: Term) -> &'static str {
    match t {
        Term::ShortTerm => "ST",
        Term::LongTerm => "LT",
    }
}

fn disposal_year(d: &btctax_core::Disposal) -> i32 {
    d.disposed_at.year()
}

/// FR4 render: holdings (always current) + realized disposals/removals/income (year-filtered).
pub fn render_report(state: &LedgerState, year: Option<i32>) -> String {
    let mut out = String::new();
    let yr = |y: i32| year.map_or(true, |f| f == y); // year filter; None => all (1.74-compatible; not is_none_or)

    let _ = writeln!(out, "Holdings (per wallet):");
    if state.holdings_by_wallet.is_empty() {
        let _ = writeln!(out, "  none");
    }
    for (w, sat) in &state.holdings_by_wallet {
        let _ = writeln!(out, "  {}: {} sat", wallet_label(w), sat);
    }

    let _ = writeln!(out, "Lots:");
    if state.lots.is_empty() {
        let _ = writeln!(out, "  none");
    }
    for l in &state.lots {
        let _ = writeln!(
            out,
            "  {}#{} {} remaining {} sat | basis {} ({:?}){}",
            l.lot_id.origin_event_id.canonical(),
            l.lot_id.split_sequence,
            wallet_label(&l.wallet),
            l.remaining_sat,
            l.usd_basis,
            l.basis_source,
            if l.basis_pending { " [basis pending]" } else { "" }
        );
    }

    let label = match year {
        Some(y) => format!("(year {y})"),
        None => "(all years)".to_string(),
    };

    let disposals: Vec<_> = state.disposals.iter().filter(|d| yr(disposal_year(d))).collect();
    if disposals.is_empty() {
        let _ = writeln!(out, "Disposals {}: none", label);
    } else {
        let _ = writeln!(out, "Disposals {}:", label);
        for d in disposals {
            let _ = writeln!(out, "  {:?} @ {} ({:?})", d.kind, d.disposed_at, d.event.canonical());
            for leg in &d.legs {
                render_disposal_leg(&mut out, leg);
            }
        }
    }

    let removals: Vec<_> = state.removals.iter().filter(|r| yr(r.removed_at.year())).collect();
    if removals.is_empty() {
        let _ = writeln!(out, "Removals {}: none", label);
    } else {
        let _ = writeln!(out, "Removals {}:", label);
        for r in removals {
            let _ = writeln!(out, "  {:?} @ {} ({:?})", r.kind, r.removed_at, r.event.canonical());
            for leg in &r.legs {
                render_removal_leg(&mut out, leg);
            }
        }
    }

    let income: Vec<_> = state
        .income_recognized
        .iter()
        .filter(|i| yr(i.recognized_at.year()))
        .collect();
    if income.is_empty() {
        let _ = writeln!(out, "Income {}: none", label);
    } else {
        let _ = writeln!(out, "Income {}:", label);
        for i in income {
            let _ = writeln!(
                out,
                "  {:?} @ {} {} sat = {} USD{}",
                i.kind,
                i.recognized_at,
                i.sat,
                i.usd_fmv,
                if i.business { " [business]" } else { "" }
            );
        }
    }
    out
}

fn render_disposal_leg(out: &mut String, leg: &DisposalLeg) {
    let zone = leg
        .gift_zone
        .map(|z| format!(" gift-zone {z:?}"))
        .unwrap_or_default();
    let _ = writeln!(
        out,
        "    {} sat: proceeds {} basis {} gain {} {}{}",
        leg.sat,
        leg.proceeds,
        leg.basis,
        leg.gain,
        term_str(leg.term),
        zone
    );
}

fn render_removal_leg(out: &mut String, leg: &RemovalLeg) {
    let _ = writeln!(
        out,
        "    {} sat: basis {} fmv {} {} (zero gain)",
        leg.sat,
        leg.basis,
        leg.fmv_at_transfer,
        term_str(leg.term)
    );
}
```
*(Uses `map_or(true, …)`, not `is_none_or` — the latter stabilized in Rust 1.82, after the workspace's pinned rust-version 1.74.)*

- [ ] **Step 5: Wire `cmd/mod.rs`.** Add `pub mod inspect;`.
- [ ] **Step 6: Run → PASS + gate + commit.**
```bash
cargo test -p btctax-cli --test verify_report report
cargo clippy --all-targets -p btctax-cli -- -D warnings && cargo fmt --check
git commit -am "feat(cli): report/show command + LedgerState render (--year filter, FR4)"
```

---

### Task 6: `verify` (FR9) — conservation + blockers + pending + safe-harbor status; exit code

**Files:** Extend `src/cmd/inspect.rs`, `src/render.rs`; Extend `tests/verify_report.rs`.

**Interfaces — Consumes:** `Session::project`, `load_all` (to detect a `SafeHarborAllocation` for status), `btctax_core::{conservation_report, ConservationReport, Blocker, BlockerKind, Severity}`. **Produces:** `cmd::inspect::verify(vault_path, pp) -> Result<render::VerifyReport, CliError>`; `render::VerifyReport { conservation, hard, advisory, pending, unknown_basis_inbounds, safe_harbor }` with `has_hard_blockers()`; `render::render_verify`.

- [ ] **Step 1: Failing test in `tests/verify_report.rs`**
```rust
#[test]
fn verify_reports_conservation_and_advisory_pending_no_hard_blockers() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    let file = fixtures::coinbase_buy_sell_send(dir.path());
    cmd::import::run(&vault, &pp(), &[file]).unwrap();

    let report = cmd::inspect::verify(&vault, &pp()).unwrap();
    // The Send sits in pending_reconciliation (advisory unmatched_outflows); conservation still balances.
    assert!(report.conservation.balanced, "Σpending closes the FR9 identity");
    assert_eq!(report.pending, 1);
    assert!(report.hard.is_empty(), "no hard blockers -> exit 0");
    assert!(!report.has_hard_blockers());

    let text = render::render_verify(&report);
    assert!(text.contains("Conservation"));
    assert!(text.contains("Path A")); // default 2025 transition, no allocation
}
```

- [ ] **Step 2: Run → FAIL.** `cargo test -p btctax-cli --test verify_report verify`

- [ ] **Step 3: Implement `cmd::inspect::verify`**
```rust
// add to src/cmd/inspect.rs
use crate::render::{build_verify, VerifyReport};
use btctax_core::persistence::load_all;

/// FR9: project, compute the sat-conservation report, partition blockers by severity, and summarize
/// pending reconciliation + safe-harbor status. The binary maps `has_hard_blockers()` to a non-zero
/// exit (a hard blocker gates downstream tax computation, §7.1).
pub fn verify(vault_path: &Path, pp: &Passphrase) -> Result<VerifyReport, CliError> {
    let session = Session::open(vault_path, pp)?;
    let (state, _cfg) = session.project()?;
    let events = load_all(session.conn())?;
    Ok(build_verify(&state, &events))
}
```

- [ ] **Step 4: Implement `build_verify` + `render_verify` in `src/render.rs`**
```rust
use btctax_core::{
    conservation_report, Blocker, BlockerKind, ConservationReport, EventPayload, LedgerEvent,
    Severity,
};

/// Structured FR9 outcome (so tests assert on data, not stdout, and `main` keys the exit code).
#[derive(Debug, Clone)]
pub struct VerifyReport {
    pub conservation: ConservationReport,
    pub hard: Vec<Blocker>,
    pub advisory: Vec<Blocker>,
    pub pending: usize,
    pub unknown_basis_inbounds: usize,
    pub safe_harbor: String,
}

impl VerifyReport {
    /// Non-zero exit condition (§7.1): any open hard blocker. (Conservation imbalance always coincides
    /// with a hard blocker — `uncovered_disposal` — so the hard list is the single source of truth.)
    pub fn has_hard_blockers(&self) -> bool {
        !self.hard.is_empty()
    }
}

/// 2025-transition status for display: detect an allocation event + the safe-harbor blockers (§7.4).
fn safe_harbor_status(state: &LedgerState, events: &[LedgerEvent]) -> String {
    let has_alloc = events
        .iter()
        .any(|e| matches!(e.payload, EventPayload::SafeHarborAllocation(_)));
    let unconservable = state
        .blockers
        .iter()
        .any(|b| b.kind == BlockerKind::SafeHarborUnconservable);
    let timebar = state
        .blockers
        .iter()
        .any(|b| b.kind == BlockerKind::SafeHarborTimebar);
    if unconservable {
        "Path B allocation FAILS conservation/eligibility (hard, §7.4) — fix the allocation".to_string()
    } else if timebar {
        "Path B time-barred -> using Path A (advisory); `reconcile safe-harbor attest` if timely in your books".to_string()
    } else if has_alloc {
        "Path B safe-harbor allocation is effective (§7.4)".to_string()
    } else {
        "Path A (actual per-wallet reconstruction; default, no election)".to_string()
    }
}

pub fn build_verify(state: &LedgerState, events: &[LedgerEvent]) -> VerifyReport {
    let conservation = conservation_report(state);
    let mut hard = Vec::new();
    let mut advisory = Vec::new();
    for b in &state.blockers {
        match b.kind.severity() {
            Severity::Hard => hard.push(b.clone()),
            Severity::Advisory => advisory.push(b.clone()),
        }
    }
    let unknown_basis_inbounds = state
        .blockers
        .iter()
        .filter(|b| b.kind == BlockerKind::UnknownBasisInbound)
        .count();
    VerifyReport {
        conservation,
        hard,
        advisory,
        pending: state.pending_reconciliation.len(),
        unknown_basis_inbounds,
        safe_harbor: safe_harbor_status(state, events),
    }
}

pub fn render_verify(r: &VerifyReport) -> String {
    let mut out = String::new();
    let c = &r.conservation;
    let _ = writeln!(out, "Conservation (FR9): {}", if c.balanced { "BALANCED" } else { "DRIFT" });
    let _ = writeln!(
        out,
        "  in {} = disposed {} + removed {} + held {} + fee-sats {} + pending {}{}",
        c.sigma_in,
        c.sigma_disposed,
        c.sigma_removed,
        c.sigma_held,
        c.sigma_fee_sats,
        c.sigma_pending,
        if c.has_uncovered { "  [identity undefined: uncovered disposal open]" } else { "" }
    );
    let _ = writeln!(out, "2025 transition: {}", r.safe_harbor);
    let _ = writeln!(out, "Pending reconciliation: {} transfer(s); unknown-basis inbounds: {}", r.pending, r.unknown_basis_inbounds);

    let _ = writeln!(out, "Hard blockers (gate tax computation): {}", r.hard.len());
    for b in &r.hard {
        let evt = b.event.as_ref().map(|e| e.canonical()).unwrap_or_else(|| "-".to_string());
        let _ = writeln!(out, "  [{:?}] {} :: {}", b.kind, evt, b.detail);
    }
    let _ = writeln!(out, "Advisory blockers: {}", r.advisory.len());
    for b in &r.advisory {
        let evt = b.event.as_ref().map(|e| e.canonical()).unwrap_or_else(|| "-".to_string());
        let _ = writeln!(out, "  [{:?}] {} :: {}", b.kind, evt, b.detail);
    }
    out
}
```

- [ ] **Step 5: Run → PASS + gate + commit.**
```bash
cargo test -p btctax-cli --test verify_report
cargo clippy --all-targets -p btctax-cli -- -D warnings && cargo fmt --check
git commit -am "feat(cli): verify command (FR9 conservation + blockers + safe-harbor status + exit code)"
```

---

### Task 7: `eventref` — canonical `EventId` parsing + arg parsers

**Files:** Create `src/eventref.rs`; Modify `src/lib.rs` (wires `pub mod eventref;`).

**Interfaces — Consumes:** `btctax_core::{EventId, Source, SourceRef, Fingerprint, WalletId, IncomeKind, Usd, TaxDate}`, `rust_decimal::Decimal`, `time`. **Produces:** `parse_event_id` (round-trips `EventId::canonical()`, tolerating `|` inside `source_ref`), `parse_wallet_id`, `parse_usd_arg`, `parse_date_arg`, `parse_income_kind`. Needed by every reconcile subcommand (Tasks 8–13).

- [ ] **Step 1: Failing tests in `src/eventref.rs`**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use btctax_core::{EventId, Fingerprint, IncomeKind, Source, SourceRef, WalletId};
    use rust_decimal_macros::dec;
    use time::macros::date;

    #[test]
    fn import_eventref_round_trips_even_with_pipe_in_source_ref() {
        // Adapters mint direction-scoped source_refs that CONTAIN '|' (e.g. "out|cb-send").
        let id = EventId::import(Source::Coinbase, SourceRef::new("out|cb-send"));
        let s = id.canonical(); // "import|coinbase|out|cb-send"
        assert_eq!(parse_event_id(&s).unwrap(), id);
    }

    #[test]
    fn decision_and_conflict_eventrefs_round_trip() {
        let d = EventId::decision(7);
        assert_eq!(parse_event_id(&d.canonical()).unwrap(), d);

        let fp = Fingerprint::of_bytes(b"x");
        let c = EventId::conflict(Source::Gemini, SourceRef::new("in|99|credit|1#0"), &fp);
        assert_eq!(parse_event_id(&c.canonical()).unwrap(), c);
    }

    #[test]
    fn bad_eventref_is_a_typed_error() {
        assert!(matches!(parse_event_id("garbage"), Err(crate::CliError::BadEventRef(_))));
        assert!(matches!(parse_event_id("import|nosuchsource|x"), Err(crate::CliError::BadEventRef(_))));
    }

    #[test]
    fn wallet_usd_date_kind_parsers() {
        assert_eq!(
            parse_wallet_id("exchange:coinbase:main").unwrap(),
            WalletId::Exchange { provider: "coinbase".into(), account: "main".into() }
        );
        assert_eq!(parse_wallet_id("self:cold").unwrap(), WalletId::SelfCustody { label: "cold".into() });
        assert_eq!(parse_usd_arg("1234.56").unwrap(), dec!(1234.56));
        assert_eq!(parse_date_arg("2025-01-01").unwrap(), date!(2025 - 01 - 01));
        assert_eq!(parse_income_kind("interest").unwrap(), IncomeKind::Interest);
        assert!(parse_wallet_id("bogus").is_err());
    }
}
```

- [ ] **Step 2: Run → FAIL.** `cargo test -p btctax-cli eventref`

- [ ] **Step 3: Implement `src/eventref.rs`**
```rust
//! Parse CLI references back into core types. The primary case is the canonical `EventId` string the
//! engine prints (`EventId::canonical()`): `import|<src>|<source_ref…>`, `conflict|<src>|<source_ref…>|<fp>`,
//! `decision|<seq>`. `source_ref` itself may contain `|` (adapters mint direction-scoped refs like
//! `out|cb-send`), so import rejoins parts[2..] and conflict takes the LAST part as the fingerprint.
use crate::CliError;
use btctax_core::{EventId, Fingerprint, IncomeKind, Source, SourceRef, TaxDate, Usd, WalletId};
use rust_decimal::Decimal;
use std::str::FromStr;
use time::macros::format_description;
use time::Date;

fn source_of(tag: &str) -> Option<Source> {
    match tag {
        "swan" => Some(Source::Swan),
        "coinbase" => Some(Source::Coinbase),
        "gemini" => Some(Source::Gemini),
        "river" => Some(Source::River),
        _ => None,
    }
}

pub fn parse_event_id(s: &str) -> Result<EventId, CliError> {
    let bad = || CliError::BadEventRef(s.to_string());
    let parts: Vec<&str> = s.split('|').collect();
    match parts.first().copied() {
        Some("import") => {
            if parts.len() < 3 {
                return Err(bad());
            }
            let source = source_of(parts[1]).ok_or_else(bad)?;
            let source_ref = parts[2..].join("|"); // may contain '|'
            Ok(EventId::import(source, SourceRef::new(source_ref)))
        }
        Some("conflict") => {
            if parts.len() < 4 {
                return Err(bad());
            }
            let source = source_of(parts[1]).ok_or_else(bad)?;
            let fp = Fingerprint(parts[parts.len() - 1].to_string()); // fingerprint is the last segment
            let source_ref = parts[2..parts.len() - 1].join("|");
            Ok(EventId::conflict(source, SourceRef::new(source_ref), &fp))
        }
        Some("decision") => {
            if parts.len() != 2 {
                return Err(bad());
            }
            let seq = parts[1].parse::<u64>().map_err(|_| bad())?;
            Ok(EventId::decision(seq))
        }
        _ => Err(bad()),
    }
}

/// `exchange:PROVIDER:ACCOUNT` | `self:LABEL`.
pub fn parse_wallet_id(s: &str) -> Result<WalletId, CliError> {
    let parts: Vec<&str> = s.splitn(3, ':').collect();
    match parts.as_slice() {
        ["exchange", provider, account] if !provider.is_empty() && !account.is_empty() => {
            Ok(WalletId::Exchange {
                provider: (*provider).to_string(),
                account: (*account).to_string(),
            })
        }
        ["self", label] if !label.is_empty() => Ok(WalletId::SelfCustody {
            label: (*label).to_string(),
        }),
        _ => Err(CliError::Usage(format!(
            "bad wallet {s:?}; use exchange:PROVIDER:ACCOUNT or self:LABEL"
        ))),
    }
}

/// Exact USD (NFR5): string → Decimal, never float.
pub fn parse_usd_arg(s: &str) -> Result<Usd, CliError> {
    Decimal::from_str(s.trim()).map_err(|e| CliError::Usage(format!("bad USD {s:?}: {e}")))
}

pub fn parse_date_arg(s: &str) -> Result<TaxDate, CliError> {
    let fmt = format_description!("[year]-[month]-[day]");
    Date::parse(s.trim(), &fmt).map_err(|e| CliError::Usage(format!("bad date {s:?}: {e}")))
}

pub fn parse_income_kind(s: &str) -> Result<IncomeKind, CliError> {
    match s.to_ascii_lowercase().as_str() {
        "mining" => Ok(IncomeKind::Mining),
        "staking" => Ok(IncomeKind::Staking),
        "interest" => Ok(IncomeKind::Interest),
        "airdrop" => Ok(IncomeKind::Airdrop),
        "reward" => Ok(IncomeKind::Reward),
        _ => Err(CliError::Usage(format!("bad income kind {s:?}"))),
    }
}
```

- [ ] **Step 4: Run → PASS + gate + commit.**
```bash
cargo test -p btctax-cli eventref
cargo clippy --all-targets -p btctax-cli -- -D warnings && cargo fmt --check
git commit -am "feat(cli): eventref parsing (canonical EventId round-trip + arg parsers)"
```

---

### Task 8: reconcile `link-transfer` — `TransferLink` (FR6/TP7)

**Files:** Create `src/cmd/reconcile.rs`; Modify `src/cmd/mod.rs`; Create `tests/reconcile.rs`.

**Interfaces — Consumes:** `append_decision`, `eventref::{parse_event_id, parse_wallet_id}`, `EventPayload::TransferLink`, `TransferTarget`, `Session`. **Produces:** `cmd::reconcile::link_transfer(vault_path, pp, out_ref, target, now) -> Result<EventId, CliError>` where `target` is `--to-event <eventref>` or `--to-wallet <walletspec>` (resolved by the caller into a `TransferTarget`).

- [ ] **Step 1: Failing test in `tests/reconcile.rs`**
```rust
mod fixtures;
use btctax_cli::{cmd, Session};
use btctax_core::{EventPayload, TransferTarget};
use btctax_store::Passphrase;
use time::macros::datetime;

fn pp() -> Passphrase {
    Passphrase::new("pw".into())
}
fn now() -> time::OffsetDateTime {
    datetime!(2026-02-01 12:00:00 UTC) // fixed decision clock (NFR4 deterministic tests)
}

/// Import the buy/sell/send fixture and return (vault_path, the TransferOut's canonical eventref).
fn vault_with_pending(dir: &std::path::Path) -> (std::path::PathBuf, String) {
    let vault = dir.join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.join("k.asc")).unwrap();
    let file = fixtures::coinbase_buy_sell_send(dir);
    cmd::import::run(&vault, &pp(), &[file]).unwrap();
    let s = Session::open(&vault, &pp()).unwrap();
    let (state, _) = s.project().unwrap();
    let out_ref = state.pending_reconciliation[0].event.canonical();
    (vault, out_ref)
}

#[test]
fn link_transfer_clears_pending_and_relocates_lots() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, out_ref) = vault_with_pending(dir.path());

    let target = TransferTarget::Wallet(
        btctax_cli::eventref::parse_wallet_id("self:cold").unwrap(),
    );
    let id = cmd::reconcile::link_transfer(&vault, &pp(), &out_ref, target, now()).unwrap();
    assert!(matches!(id, btctax_core::EventId::Decision { seq: 1 }));

    // Re-project: the TransferOut is no longer pending (it became a self-transfer; TP7).
    let s = Session::open(&vault, &pp()).unwrap();
    let (state, _) = s.project().unwrap();
    assert!(state.pending_reconciliation.is_empty());
    // The decision is persisted as a TransferLink.
    let events = btctax_core::persistence::load_all(s.conn()).unwrap();
    assert!(events
        .iter()
        .any(|e| matches!(e.payload, EventPayload::TransferLink(_))));
}
```

- [ ] **Step 2: Run → FAIL.** `cargo test -p btctax-cli --test reconcile link_transfer`

- [ ] **Step 3: Implement `src/cmd/reconcile.rs` (start of the module)**
```rust
//! reconcile decision emitters (FR6/FR7/FR8, §7.2). Each fn builds exactly ONE `EventPayload` decision
//! variant and appends it via `append_decision` (monotonic `decision_seq`), then saves. Decisions are
//! append-only and re-projectable; the engine resolves precedence (latest-`decision_seq`, Void-first).
//! `now` is the injected decision creation-time / safe-harbor made-date (§6.2) — deterministic in tests.
use crate::{CliError, Session};
use btctax_core::persistence::append_decision;
use btctax_core::{
    EventId, EventPayload, TransferLink, TransferTarget,
};
use btctax_store::Passphrase;
use std::path::Path;
use time::{OffsetDateTime, UtcOffset};

use crate::eventref::parse_event_id;

/// Append one decision (creation tz = UTC; decisions are not wallet-scoped) and persist.
fn append_and_save(
    session: &mut Session,
    payload: EventPayload,
    now: OffsetDateTime,
) -> Result<EventId, CliError> {
    let id = append_decision(session.conn(), payload, now, UtcOffset::UTC, None)?;
    session.save()?;
    Ok(id)
}

/// FR6/TP7: confirm a self-transfer. `target` is a destination `TransferIn` event (`--to-event`) or a
/// known wallet (`--to-wallet`); the engine relocates the lots carrying basis + acquired_at.
pub fn link_transfer(
    vault_path: &Path,
    pp: &Passphrase,
    out_ref: &str,
    target: TransferTarget,
    now: OffsetDateTime,
) -> Result<EventId, CliError> {
    let out_event = parse_event_id(out_ref)?;
    let mut session = Session::open(vault_path, pp)?;
    let payload = EventPayload::TransferLink(TransferLink {
        out_event,
        in_event_or_wallet: target,
    });
    append_and_save(&mut session, payload, now)
}
```

- [ ] **Step 4: Wire `cmd/mod.rs`.** Add `pub mod reconcile;`. (And `pub mod eventref;` is already public in `lib.rs`, so the test's `btctax_cli::eventref::parse_wallet_id` resolves.)
- [ ] **Step 5: Run → PASS + gate + commit.**
```bash
cargo test -p btctax-cli --test reconcile link_transfer
cargo clippy --all-targets -p btctax-cli -- -D warnings && cargo fmt --check
git commit -am "feat(cli): reconcile link-transfer (TransferLink, FR6/TP7)"
```

---

### Task 9: reconcile `classify-inbound` — `ClassifyInbound` (FR6; re-supplies TransferIn basis)

**Files:** Extend `src/cmd/reconcile.rs`, `tests/reconcile.rs`.

**Interfaces — Produces:** `cmd::reconcile::classify_inbound(vault_path, pp, in_ref, class: InboundClass, now) -> Result<EventId, CliError>`. The caller builds the `InboundClass` (income kind/fmv/business, or gift donor_basis/donor_acquired_at/fmv_at_gift) from flags. This is the path that **re-supplies the TransferIn basis GAP** (Swan deposits drop basis at ingest; §9.1 FOUND GAP).

- [ ] **Step 1: Failing test in `tests/reconcile.rs`** (use a Coinbase `Receive` → TransferIn)
```rust
use btctax_core::{InboundClass, IncomeKind};

fn coinbase_with_receive(dir: &std::path::Path) -> std::path::PathBuf {
    let p = dir.join("cb_recv.csv");
    std::fs::write(&p, "\r\nTransactions\r\nUser,x\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
cb-recv,2025-03-01 12:00:00 UTC,Receive,BTC,0.05000000,USD,84000.00,,,,,bc1qsender,\r\n").unwrap();
    p
}

#[test]
fn classify_inbound_income_resolves_unknown_basis() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    cmd::import::run(&vault, &pp(), &[coinbase_with_receive(dir.path())]).unwrap();

    let in_ref = {
        let s = Session::open(&vault, &pp()).unwrap();
        let events = btctax_core::persistence::load_all(s.conn()).unwrap();
        events
            .iter()
            .find(|e| matches!(e.payload, EventPayload::TransferIn(_)))
            .unwrap()
            .id
            .canonical()
    };

    let class = InboundClass::Income {
        kind: IncomeKind::Reward,
        fmv: Some(btctax_cli::eventref::parse_usd_arg("4200.00").unwrap()),
        business: false,
    };
    cmd::reconcile::classify_inbound(&vault, &pp(), &in_ref, class, now()).unwrap();

    let s = Session::open(&vault, &pp()).unwrap();
    let (state, _) = s.project().unwrap();
    // The classified inbound is recognized income; no unknown-basis blocker remains.
    assert_eq!(state.income_recognized.len(), 1);
    assert!(state
        .blockers
        .iter()
        .all(|b| b.kind != btctax_core::BlockerKind::UnknownBasisInbound));
}
```

- [ ] **Step 2: Run → FAIL.** `cargo test -p btctax-cli --test reconcile classify_inbound`

- [ ] **Step 3: Extend `src/cmd/reconcile.rs`**
```rust
use btctax_core::{ClassifyInbound, InboundClass};

/// FR6: classify an externally-sourced inbound `TransferIn` as Income or a received Gift. For Income
/// this supplies the FMV basis; for Gift it supplies donor basis/date + fmv_at_gift (TP11 dual-basis).
/// This is the re-supply path for the §9.1 Swan `deposit` basis GAP.
pub fn classify_inbound(
    vault_path: &Path,
    pp: &Passphrase,
    in_ref: &str,
    class: InboundClass,
    now: OffsetDateTime,
) -> Result<EventId, CliError> {
    let transfer_in_event = parse_event_id(in_ref)?;
    let mut session = Session::open(vault_path, pp)?;
    let payload = EventPayload::ClassifyInbound(ClassifyInbound {
        transfer_in_event,
        as_: class,
    });
    append_and_save(&mut session, payload, now)
}
```

- [ ] **Step 4: Run → PASS + gate + commit.**
```bash
cargo test -p btctax-cli --test reconcile classify_inbound
cargo clippy --all-targets -p btctax-cli -- -D warnings && cargo fmt --check
git commit -am "feat(cli): reconcile classify-inbound (ClassifyInbound; re-supplies TransferIn basis)"
```

---

### Task 10: reconcile `reclassify-outflow` — `ReclassifyOutflow` (FR6; TP8 fee)

**Files:** Extend `src/cmd/reconcile.rs`, `tests/reconcile.rs`.

**Interfaces — Produces:** `cmd::reconcile::reclassify_outflow(vault_path, pp, out_ref, class: OutflowClass, principal: Usd, fee_usd: Option<Usd>, now) -> Result<EventId, CliError>` — folds a pending `TransferOut` as `Dispose{Sell|Spend}` / `GiftOut` / `Donate` with supplied proceeds/FMV (fee per TP8).

- [ ] **Step 1: Failing test in `tests/reconcile.rs`**
```rust
use btctax_core::{DisposeKind, OutflowClass};

#[test]
fn reclassify_outflow_to_sell_creates_a_disposal() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, out_ref) = vault_with_pending(dir.path());

    cmd::reconcile::reclassify_outflow(
        &vault,
        &pp(),
        &out_ref,
        OutflowClass::Dispose { kind: DisposeKind::Sell },
        btctax_cli::eventref::parse_usd_arg("2000.00").unwrap(),
        Some(btctax_cli::eventref::parse_usd_arg("3.00").unwrap()),
        now(),
    )
    .unwrap();

    let s = Session::open(&vault, &pp()).unwrap();
    let (state, _) = s.project().unwrap();
    assert!(state.pending_reconciliation.is_empty()); // outflow resolved
    assert_eq!(state.disposals.len(), 2); // the fixture Sell + the reclassified Send
}
```

- [ ] **Step 2: Run → FAIL.** `cargo test -p btctax-cli --test reconcile reclassify_outflow`

- [ ] **Step 3: Extend `src/cmd/reconcile.rs`**
```rust
use btctax_core::{OutflowClass, ReclassifyOutflow, Usd};

/// FR6: reclassify a pending `TransferOut` as a Sell/Spend disposition, a Gift out, or a Donation.
/// `principal` is the gross proceeds (Dispose) or FMV-at-transfer (Gift/Donate); `fee_usd` is the
/// optional disposition fee (TP8 / TP2). The engine applies the configured TP8 (c)/(b) fee treatment.
pub fn reclassify_outflow(
    vault_path: &Path,
    pp: &Passphrase,
    out_ref: &str,
    class: OutflowClass,
    principal: Usd,
    fee_usd: Option<Usd>,
    now: OffsetDateTime,
) -> Result<EventId, CliError> {
    let transfer_out_event = parse_event_id(out_ref)?;
    let mut session = Session::open(vault_path, pp)?;
    let payload = EventPayload::ReclassifyOutflow(ReclassifyOutflow {
        transfer_out_event,
        as_: class,
        principal_proceeds_or_fmv: principal,
        fee_usd,
    });
    append_and_save(&mut session, payload, now)
}
```

- [ ] **Step 4: Run → PASS + gate + commit.**
```bash
cargo test -p btctax-cli --test reconcile reclassify_outflow
cargo clippy --all-targets -p btctax-cli -- -D warnings && cargo fmt --check
git commit -am "feat(cli): reconcile reclassify-outflow (ReclassifyOutflow; dispose/gift/donate, TP8)"
```

---

### Task 11: reconcile `set-fmv` + `void` — `ManualFmv` / `VoidDecisionEvent` (FR3/FR8)

**Files:** Extend `src/cmd/reconcile.rs`, `tests/reconcile.rs`.

**Interfaces — Produces:** `cmd::reconcile::set_fmv(vault_path, pp, event_ref, usd_fmv, now)` (clears `fmv_missing`) and `cmd::reconcile::void(vault_path, pp, target_ref, now)` (revokes a *revocable* decision; voiding a non-revocable/effective target → `decision_conflicts`, handled by the engine — the CLI just appends).

- [ ] **Step 1: Failing test in `tests/reconcile.rs`** (void a TransferLink, then confirm it is dropped)
```rust
#[test]
fn void_drops_a_revocable_decision() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, out_ref) = vault_with_pending(dir.path());
    let link = cmd::reconcile::link_transfer(
        &vault,
        &pp(),
        &out_ref,
        TransferTarget::Wallet(btctax_cli::eventref::parse_wallet_id("self:cold").unwrap()),
        now(),
    )
    .unwrap();

    // Void the link by its decision eventref; the outflow returns to pending.
    cmd::reconcile::void(&vault, &pp(), &link.canonical(), now()).unwrap();
    let s = Session::open(&vault, &pp()).unwrap();
    let (state, _) = s.project().unwrap();
    assert_eq!(state.pending_reconciliation.len(), 1);
}

#[test]
fn set_fmv_appends_a_manual_fmv_decision() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, _out_ref) = vault_with_pending(dir.path());
    // Target the Buy event (any event id parses); the decision is appended + persisted.
    let target = {
        let s = Session::open(&vault, &pp()).unwrap();
        let events = btctax_core::persistence::load_all(s.conn()).unwrap();
        events
            .iter()
            .find(|e| matches!(e.payload, EventPayload::Acquire(_)))
            .unwrap()
            .id
            .canonical()
    };
    let id = cmd::reconcile::set_fmv(&vault, &pp(), &target, btctax_cli::eventref::parse_usd_arg("123.45").unwrap(), now()).unwrap();
    let s = Session::open(&vault, &pp()).unwrap();
    let events = btctax_core::persistence::load_all(s.conn()).unwrap();
    assert!(events.iter().any(|e| e.id == id && matches!(e.payload, EventPayload::ManualFmv(_))));
}
```

- [ ] **Step 2: Run → FAIL.** `cargo test -p btctax-cli --test reconcile -- void set_fmv`

- [ ] **Step 3: Extend `src/cmd/reconcile.rs`**
```rust
use btctax_core::{ManualFmv, VoidDecisionEvent};

/// FR3: set a manual FMV on an event (`ManualEntry`), clearing its `fmv_missing` blocker.
pub fn set_fmv(
    vault_path: &Path,
    pp: &Passphrase,
    event_ref: &str,
    usd_fmv: Usd,
    now: OffsetDateTime,
) -> Result<EventId, CliError> {
    let event = parse_event_id(event_ref)?;
    let mut session = Session::open(vault_path, pp)?;
    append_and_save(&mut session, EventPayload::ManualFmv(ManualFmv { event, usd_fmv }), now)
}

/// FR8: void a revocable decision. Voiding a non-revocable / effective-allocation target raises
/// `decision_conflicts` in the projection (no effect) — the CLI only appends; the engine adjudicates.
pub fn void(
    vault_path: &Path,
    pp: &Passphrase,
    target_ref: &str,
    now: OffsetDateTime,
) -> Result<EventId, CliError> {
    let target_event_id = parse_event_id(target_ref)?;
    let mut session = Session::open(vault_path, pp)?;
    append_and_save(
        &mut session,
        EventPayload::VoidDecisionEvent(VoidDecisionEvent { target_event_id }),
        now,
    )
}
```

- [ ] **Step 4: Run → PASS + gate + commit.**
```bash
cargo test -p btctax-cli --test reconcile
cargo clippy --all-targets -p btctax-cli -- -D warnings && cargo fmt --check
git commit -am "feat(cli): reconcile set-fmv (ManualFmv) + void (VoidDecisionEvent), FR3/FR8"
```

---

### Task 12: reconcile `classify-raw` + import-conflict `accept`/`reject` (FR1/FR8, §7.2)

**Files:** Extend `src/cmd/reconcile.rs`, `tests/reconcile.rs`.

**Interfaces — Produces:** `cmd::reconcile::classify_raw(vault_path, pp, target_ref, payload_json, now)` (resolves an `Unclassified` row to a supplied imported payload, parsed from JSON via `serde_json` since `EventPayload` is `Deserialize`); `accept_conflict`/`reject_conflict` (`SupersedeImport`/`RejectImport` over a conflict eventref).

- [ ] **Step 1: Failing test in `tests/reconcile.rs`** (Coinbase `Order` → Unclassified, then classify-raw to Acquire)
```rust
fn coinbase_with_order(dir: &std::path::Path) -> std::path::PathBuf {
    let p = dir.join("cb_order.csv");
    std::fs::write(&p, "\r\nTransactions\r\nUser,x\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
cb-ord,2025-03-01 12:00:00 UTC,Order,BTC,0.01000000,USD,84000.00,840.00,845.00,5.00,,,\r\n").unwrap();
    p
}

#[test]
fn classify_raw_resolves_an_unclassified_row() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    cmd::import::run(&vault, &pp(), &[coinbase_with_order(dir.path())]).unwrap();

    let target = {
        let s = Session::open(&vault, &pp()).unwrap();
        let events = btctax_core::persistence::load_all(s.conn()).unwrap();
        events.iter().find(|e| matches!(e.payload, EventPayload::Unclassified(_))).unwrap().id.canonical()
    };
    // Supply an Acquire payload as JSON (EventPayload is Deserialize).
    let json = r#"{"Acquire":{"sat":1000000,"usd_cost":"845.00","fee_usd":"5.00","basis_source":"ComputedFromCost"}}"#;
    cmd::reconcile::classify_raw(&vault, &pp(), &target, json, now()).unwrap();

    let s = Session::open(&vault, &pp()).unwrap();
    let (state, _) = s.project().unwrap();
    // No Unclassified blocker remains; a lot now exists.
    assert!(state.blockers.iter().all(|b| b.kind != btctax_core::BlockerKind::Unclassified));
    assert_eq!(state.lots.len(), 1);
}
```

- [ ] **Step 2: Run → FAIL.** `cargo test -p btctax-cli --test reconcile classify_raw`

- [ ] **Step 3: Extend `src/cmd/reconcile.rs`**
```rust
use btctax_core::{ClassifyRaw, RejectImport, SupersedeImport};

/// FR2/§7.3: resolve an `Unclassified` row to a real imported payload (preserving the target EventId).
/// The payload is supplied as JSON (`EventPayload` is `Deserialize`) — e.g. `{"Acquire":{…}}`.
pub fn classify_raw(
    vault_path: &Path,
    pp: &Passphrase,
    target_ref: &str,
    payload_json: &str,
    now: OffsetDateTime,
) -> Result<EventId, CliError> {
    let target = parse_event_id(target_ref)?;
    let as_: EventPayload = serde_json::from_str(payload_json)
        .map_err(|e| CliError::Usage(format!("bad --payload-json: {e}")))?;
    if !as_.is_imported() {
        return Err(CliError::Usage(
            "classify-raw payload must be an imported variant (Acquire/Income/Dispose/TransferOut/TransferIn/Unclassified)".into(),
        ));
    }
    let mut session = Session::open(vault_path, pp)?;
    append_and_save(
        &mut session,
        EventPayload::ClassifyRaw(ClassifyRaw {
            target,
            as_: Box::new(as_),
        }),
        now,
    )
}

/// FR1/FR8: accept an `ImportConflict` (apply the new payload to the target, keeping its EventId).
pub fn accept_conflict(
    vault_path: &Path,
    pp: &Passphrase,
    conflict_ref: &str,
    now: OffsetDateTime,
) -> Result<EventId, CliError> {
    let conflict_event = parse_event_id(conflict_ref)?;
    let mut session = Session::open(vault_path, pp)?;
    append_and_save(
        &mut session,
        EventPayload::SupersedeImport(SupersedeImport { conflict_event }),
        now,
    )
}

/// FR1/FR8: reject an `ImportConflict` (keep the original; clear the blocker).
pub fn reject_conflict(
    vault_path: &Path,
    pp: &Passphrase,
    conflict_ref: &str,
    now: OffsetDateTime,
) -> Result<EventId, CliError> {
    let conflict_event = parse_event_id(conflict_ref)?;
    let mut session = Session::open(vault_path, pp)?;
    append_and_save(
        &mut session,
        EventPayload::RejectImport(RejectImport { conflict_event }),
        now,
    )
}
```

- [ ] **Step 4: Run → PASS + gate + commit.** (`EventPayload::is_imported()` is public — verified.)
```bash
cargo test -p btctax-cli --test reconcile classify_raw
cargo clippy --all-targets -p btctax-cli -- -D warnings && cargo fmt --check
git commit -am "feat(cli): reconcile classify-raw + accept/reject-conflict (FR1/FR2/FR8, §7.2)"
```

---

### Task 13: reconcile `safe-harbor allocate` + `attest` — `SafeHarborAllocation` (FR7/§7.4)

**Files:** Extend `src/cmd/reconcile.rs`, `tests/reconcile.rs`.

**Interfaces — Produces:** `cmd::reconcile::safe_harbor_allocate(vault_path, pp, method, attested, now)` — builds a Path-B `SafeHarborAllocation` from a **pre-2025-only re-projection** (I-1: the 2025-01-01 Universal residue, which is exactly what the engine's allocation-independent conservation guard checks against — NOT the post-2025-disposal `state.lots`); each residue lot → one `AllocLot`, `as_of_date = TRANSITION_DATE`. And `cmd::reconcile::safe_harbor_attest(vault_path, pp, now)` — cures a *time-barred* allocation by voiding the single **live (non-voided)** prior allocation and re-appending it with `timely_allocation_attested = true`. It (Eng-I1/I-2a) **excludes voided allocations** from the single-allocation guard so the allocate→inert→void→re-allocate→attest workflow is not blocked, and (I-2b/N-2) **rejects attesting an already-effective allocation** with a clear "run `verify`" message instead of appending a void-of-effective that would become `decision_conflicts`.

- [ ] **Step 1: Failing test in `tests/reconcile.rs`**
```rust
use btctax_core::AllocMethod;

#[test]
fn safe_harbor_allocate_seeds_full_pre2025_residue_even_after_a_2025_disposal() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    // I-1 REGRESSION: a pre-2025 Buy (0.20 BTC) PLUS a 2025 Sell (0.05 BTC) that consumes part of that
    // 2024-vintage lot in FIFO. The post-2025-disposal `state.lots` would show only 0.15 BTC remaining,
    // but the engine's conservation guard compares the allocation to the *pre-2025-only* Universal residue
    // (the full 0.20 BTC at 2025-01-01). So the allocation MUST seed the full 0.20 BTC, not 0.15 — else it
    // trips the hard `SafeHarborUnconservable` blocker (the bug this fix closes).
    let p = dir.path().join("cb.csv");
    std::fs::write(&p, "\r\nTransactions\r\nUser,x\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
cb-pre,2024-01-15 12:00:00 UTC,Buy,BTC,0.20000000,USD,42500.00,8500.00,8550.00,50.00,,,\r\n\
cb-sell,2025-06-01 12:00:00 UTC,Sell,BTC,0.05000000,USD,90000.00,4500.00,4490.00,10.00,,,\r\n").unwrap();
    cmd::import::run(&vault, &pp(), &[p]).unwrap();

    let id = cmd::reconcile::safe_harbor_allocate(&vault, &pp(), AllocMethod::ActualPosition, true, now()).unwrap();
    assert!(matches!(id, btctax_core::EventId::Decision { .. }));

    let s = Session::open(&vault, &pp()).unwrap();
    let events = btctax_core::persistence::load_all(s.conn()).unwrap();
    let alloc = events.iter().find_map(|e| match &e.payload {
        EventPayload::SafeHarborAllocation(a) => Some(a.clone()),
        _ => None,
    }).expect("allocation persisted");
    assert_eq!(alloc.lots.len(), 1);
    // Seeds the FULL pre-2025 residue (0.20 BTC = 20_000_000 sat), NOT the 0.15 BTC post-Sell remainder.
    assert_eq!(alloc.lots[0].sat, 20_000_000);
    assert!(alloc.timely_allocation_attested);
    assert_eq!(alloc.as_of_date, btctax_core::conventions::TRANSITION_DATE);
    // Conservation is the engine's call; the seed equals the Universal residue → no hard safe-harbor blocker.
    let (state, _) = s.project().unwrap();
    assert!(state.blockers.iter().all(|b| b.kind != btctax_core::BlockerKind::SafeHarborUnconservable));
}

/// Build a vault with a pre-2025 lot + a 2025 disposition (so an unattested allocation is TIME-BARRED:
/// its 2026 made-date is after the first-2025-disposition prong of the §5.02(4) ActualPosition bar).
fn vault_timebarred(dir: &std::path::Path) -> std::path::PathBuf {
    let vault = dir.join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.join("k.asc")).unwrap();
    let p = dir.join("cb.csv");
    std::fs::write(&p, "\r\nTransactions\r\nUser,x\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
cb-pre,2024-01-15 12:00:00 UTC,Buy,BTC,0.20000000,USD,42500.00,8500.00,8550.00,50.00,,,\r\n\
cb-sell,2025-06-01 12:00:00 UTC,Sell,BTC,0.05000000,USD,90000.00,4500.00,4490.00,10.00,,,\r\n").unwrap();
    cmd::import::run(&vault, &pp(), &[p]).unwrap();
    vault
}

#[test]
fn safe_harbor_attest_cures_a_timebarred_allocation_excluding_voided_priors() {
    let dir = tempfile::tempdir().unwrap();
    let vault = vault_timebarred(dir.path());

    // alloc #1 (unattested) — inert: time-barred by the 2025 Sell. Then VOID it and re-allocate (alloc #2).
    // This is the legitimate allocate→inert→void→re-allocate→attest workflow (Eng-I1/I-2a). The OLD,
    // voided alloc #1 must NOT count toward attest's single-allocation guard.
    let a1 = cmd::reconcile::safe_harbor_allocate(&vault, &pp(), AllocMethod::ActualPosition, false, now()).unwrap();
    cmd::reconcile::void(&vault, &pp(), &a1.canonical(), now()).unwrap();
    let _a2 = cmd::reconcile::safe_harbor_allocate(&vault, &pp(), AllocMethod::ActualPosition, false, now()).unwrap();

    // attest is NOT blocked by the voided alloc #1; it cures the time-bar on the single LIVE allocation.
    cmd::reconcile::safe_harbor_attest(&vault, &pp(), now()).unwrap_or_else(|e| panic!("attest should succeed: {e}"));

    // Path B is now effective: the boundary seed produced SafeHarborAllocated lots; no hard blocker.
    let s = Session::open(&vault, &pp()).unwrap();
    let (state, _) = s.project().unwrap();
    assert!(state.lots.iter().any(|l| matches!(l.basis_source, btctax_core::BasisSource::SafeHarborAllocated)));
    assert!(state.blockers.iter().all(|b| b.kind != btctax_core::BlockerKind::SafeHarborUnconservable));
}

#[test]
fn safe_harbor_attest_refuses_an_already_effective_allocation() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    // A pre-2025 lot with NO 2025 disposition: an unattested allocation is ALREADY EFFECTIVE (made-date
    // precedes the only bar prong, the 2026-04-15 return-due date) → Path B with no attestation.
    let p = dir.path().join("cb_pre.csv");
    std::fs::write(&p, "\r\nTransactions\r\nUser,x\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
cb-pre,2024-01-15 12:00:00 UTC,Buy,BTC,0.20000000,USD,42500.00,8500.00,8550.00,50.00,,,\r\n").unwrap();
    cmd::import::run(&vault, &pp(), &[p]).unwrap();
    cmd::reconcile::safe_harbor_allocate(&vault, &pp(), AllocMethod::ActualPosition, false, now()).unwrap();

    // I-2(b)/N-2: attest must REFUSE (and advise `verify`) rather than append a void-of-effective.
    let err = cmd::reconcile::safe_harbor_attest(&vault, &pp(), now()).unwrap_err();
    assert!(matches!(&err, CliError::Usage(m) if m.contains("already effective") && m.contains("verify")));

    // The log was NOT mutated (no doomed Void appended): still exactly one allocation, zero voids.
    let s = Session::open(&vault, &pp()).unwrap();
    let events = btctax_core::persistence::load_all(s.conn()).unwrap();
    assert_eq!(events.iter().filter(|e| matches!(e.payload, EventPayload::SafeHarborAllocation(_))).count(), 1);
    assert_eq!(events.iter().filter(|e| matches!(e.payload, EventPayload::VoidDecisionEvent(_))).count(), 0);
}
```
*(These two tests lock in the I-2/Eng-I1 attest fixes; `CliError` must be in scope — the test file already imports `btctax_cli::{cmd, Session}`, so add `use btctax_cli::CliError;` to the test preamble.)*

- [ ] **Step 2: Run → FAIL.** `cargo test -p btctax-cli --test reconcile safe_harbor`

- [ ] **Step 3: Extend `src/cmd/reconcile.rs`**
```rust
use btctax_adapters::BundledPrices;
use btctax_core::conventions::{tax_date, TRANSITION_DATE};
use btctax_core::persistence::load_all;
use btctax_core::{project, AllocLot, AllocMethod, BlockerKind, LedgerEvent, SafeHarborAllocation};

/// FR7/§7.4: build a Path-B safe-harbor allocation that seeds from the **pre-2025 residue** (the
/// 2025-01-01 Universal-pool position), so it conserves against the engine's allocation-independent
/// conservation guard.
///
/// I-1: the engine's guard compares `Σ alloc.lots.sat`/`usd_basis` to `transition::universal_snapshot`
/// — a pre-2025-ONLY fold of the Universal pool (resolve.rs §7.4, step 3). The FULL projection's
/// `state.lots` reflects POST-2025-disposal residuals (a 2025 Sell consumes pre-2025 lots in FIFO), so
/// seeding from them would yield `alloc_sat < snap.held_sat` → hard `SafeHarborUnconservable` → Path A,
/// breaking the normal workflow. Instead we re-project a pre-2025-only event subset and read ITS lots:
///   - keep ONLY import events whose tax-date `< 2025-01-01` (drop every 2025+ acquire/dispose/transfer);
///   - keep ALL reconciliation decisions/conflicts — they SHAPE the residue (a 2026 `ClassifyInbound`
///     supplies a pre-2025 `TransferIn`'s basis; `ReclassifyOutflow`/`TransferLink` consume/relocate a
///     pre-2025 lot) — and carry a 2026 made-date, so they must NOT be tax-date-filtered;
///   - DROP any prior `SafeHarborAllocation` so the residue stays allocation-INDEPENDENT (matches
///     `universal_snapshot`, which never applies a seed) → re-allocation is idempotent.
/// This subset re-runs the IDENTICAL `fold_event` arms the engine's snapshot uses, so the totals match
/// exactly (the only difference, Path A's per-wallet relocation, preserves sat/basis 1:1; `finalize`
/// attributes Universal-pool lots by `lot.wallet`). For `ActualPosition` the per-wallet assignment falls
/// out of those residue lots' `wallet` (= the wallet holding each lot at 2025-01-01). `ProRata` still
/// seeds from these actuals; a true cross-wallet pro-rata redistribution is a manual-input refinement
/// (Open question O4). The engine's `SafeHarborUnconservable` guard remains the backstop for any residual
/// drift (e.g. a rare self-transfer straddling the 2025 boundary) — fails closed, never silent wrong tax.
pub fn safe_harbor_allocate(
    vault_path: &Path,
    pp: &Passphrase,
    method: AllocMethod,
    attested: bool,
    now: OffsetDateTime,
) -> Result<EventId, CliError> {
    let mut session = Session::open(vault_path, pp)?;

    // Pre-2025-only event subset (see the I-1 note above).
    let pre2025: Vec<LedgerEvent> = load_all(session.conn())?
        .into_iter()
        .filter(|e| match &e.id {
            EventId::Import { .. } => tax_date(e.utc_timestamp, e.original_tz) < TRANSITION_DATE,
            _ => !matches!(e.payload, EventPayload::SafeHarborAllocation(_)),
        })
        .collect();
    let cfg = session.config()?.to_projection();
    let prices = BundledPrices::load()?;
    let residue = project(&pre2025, &prices, &cfg); // == the 2025-01-01 Universal residue

    let lots: Vec<AllocLot> = residue
        .lots
        .iter()
        .filter(|l| l.remaining_sat > 0)
        .map(|l| AllocLot {
            wallet: l.wallet.clone(),
            sat: l.remaining_sat,
            usd_basis: l.usd_basis,
            acquired_at: l.acquired_at,
        })
        .collect();
    if lots.is_empty() {
        return Err(CliError::Usage(
            "no pre-2025 lots to allocate (Path A applies; safe harbor unnecessary)".into(),
        ));
    }
    let payload = EventPayload::SafeHarborAllocation(SafeHarborAllocation {
        lots,
        as_of_date: TRANSITION_DATE,
        method,
        timely_allocation_attested: attested,
    });
    append_and_save(&mut session, payload, now)
}

/// FR7: attest an existing allocation. Events are immutable, so attestation = void the single live prior
/// allocation and re-append it with `timely_allocation_attested = true`. Attestation only cures a
/// §5.02(4) TIME-BAR; it is NOT valid on an already-effective allocation (which needs nothing) nor on one
/// that fails CONSERVATION (which needs a corrected allocation, not an attestation).
pub fn safe_harbor_attest(
    vault_path: &Path,
    pp: &Passphrase,
    now: OffsetDateTime,
) -> Result<EventId, CliError> {
    let mut session = Session::open(vault_path, pp)?;
    let events = load_all(session.conn())?;

    // Eng-I1 / I-2(a): EXCLUDE voided allocations from the single-allocation guard, so the legitimate
    // allocate→inert→void→re-allocate→attest workflow (which leaves an OLD, voided allocation in the log)
    // is not blocked by "multiple allocations present." Build the voided-target set from `VoidDecisionEvent`s
    // (mirrors resolve.rs pass-1 step 1a) and keep only LIVE (non-voided) allocations.
    let voided: std::collections::BTreeSet<EventId> = events
        .iter()
        .filter_map(|e| match &e.payload {
            EventPayload::VoidDecisionEvent(v) => Some(v.target_event_id.clone()),
            _ => None,
        })
        .collect();
    let allocs: Vec<(&EventId, &SafeHarborAllocation)> = events
        .iter()
        .filter(|e| !voided.contains(&e.id))
        .filter_map(|e| match &e.payload {
            EventPayload::SafeHarborAllocation(a) => Some((&e.id, a)),
            _ => None,
        })
        .collect();
    let (prior_id, prior) = match allocs.as_slice() {
        [one] => (one.0.clone(), one.1.clone()),
        [] => return Err(CliError::Usage("no allocation to attest; run `safe-harbor allocate` first".into())),
        _ => return Err(CliError::Usage("multiple live allocations present; void the stale one before attesting".into())),
    };
    if prior.timely_allocation_attested {
        return Err(CliError::Usage("allocation is already attested".into()));
    }

    // I-2(b) / N-2: classify the prior allocation's CURRENT status via a re-projection of the live log,
    // reading the engine's own effectiveness verdict (the blockers it stamps onto `prior_id`):
    //   * `SafeHarborUnconservable` (hard) → attestation CANNOT cure it (only a corrected allocation can).
    //   * `SafeHarborTimebar` (advisory)   → inert PURELY because of the §5.02(4) bar → attestation cures it.
    //   * neither                          → ALREADY EFFECTIVE → attesting would Void an effective allocation
    //     (→ irrevocable `decision_conflicts`, §7.4) AND append a second effective allocation (→ two effective
    //     → Path A, irrecoverable). Refuse and advise `verify` (NOT "void the effective one").
    let (state, _cfg) = session.project()?;
    let blocked_with = |k: BlockerKind| {
        state
            .blockers
            .iter()
            .any(|b| b.event.as_ref() == Some(&prior_id) && b.kind == k)
    };
    let unconservable = blocked_with(BlockerKind::SafeHarborUnconservable);
    let timebarred = blocked_with(BlockerKind::SafeHarborTimebar);
    // (closure's borrow of `prior_id` ends here, so the move into the Void below is sound.)
    if unconservable {
        return Err(CliError::Usage(
            "allocation fails conservation (not a time-bar); re-run `safe-harbor allocate` to rebuild it — attestation cannot cure conservation".into(),
        ));
    }
    if !timebarred {
        return Err(CliError::Usage(
            "allocation already effective; no attestation needed — run `verify`".into(),
        ));
    }

    // Inert PURELY due to a time-bar → attestation cures it. Append Void(prior) + a re-attested copy.
    // (N2: same `now` for both; `decision_seq` orders/distinguishes them — Void first, then re-attest.)
    append_decision(
        session.conn(),
        EventPayload::VoidDecisionEvent(VoidDecisionEvent { target_event_id: prior_id }),
        now,
        UtcOffset::UTC,
        None,
    )?;
    let attested = SafeHarborAllocation {
        timely_allocation_attested: true,
        ..prior
    };
    let id = append_decision(
        session.conn(),
        EventPayload::SafeHarborAllocation(attested),
        now,
        UtcOffset::UTC,
        None,
    )?;
    session.save()?;
    Ok(id)
}
```
*(N1 (eng nit) resolved: `safe_harbor_allocate` now binds `let mut session` up front — the prior `let mut session = session;` re-bind is gone. `load_all` is imported once at the section top and shared by both fns.)*

- [ ] **Step 4: Run → PASS + gate + commit.**
```bash
cargo test -p btctax-cli --test reconcile safe_harbor
cargo clippy --all-targets -p btctax-cli -- -D warnings && cargo fmt --check
git commit -am "feat(cli): reconcile safe-harbor allocate/attest (SafeHarborAllocation, FR7/§7.4)"
```

---

### Task 14: `config` command — surface TP8 (c)/(b) + LotMethod

**Files:** Create `src/cmd/admin.rs` (config half); Modify `src/cmd/mod.rs`; Extend `tests/verify_report.rs` (or a small `tests/config.rs`).

**Interfaces — Consumes:** `config::{read_config, set_fee_treatment}`, `Session`. **Produces:** `cmd::admin::show_config(vault_path, pp) -> Result<CliConfig, CliError>` and `cmd::admin::set_config(vault_path, pp, fee_treatment: Option<FeeTreatment>) -> Result<CliConfig, CliError>` (persists then re-reads). LotMethod is read-only (single Phase-1 variant `Fifo`).

- [ ] **Step 1: Failing test (append to `tests/verify_report.rs`)**
```rust
#[test]
fn config_set_fee_treatment_b_persists_and_affects_projection_config() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    let before = cmd::admin::show_config(&vault, &pp()).unwrap();
    assert_eq!(before.fee_treatment, btctax_core::FeeTreatment::TreatmentC); // default (c)

    let after = cmd::admin::set_config(&vault, &pp(), Some(btctax_core::FeeTreatment::TreatmentB)).unwrap();
    assert_eq!(after.fee_treatment, btctax_core::FeeTreatment::TreatmentB);

    // Reopen: persisted across sessions; projection picks it up.
    let s = btctax_cli::Session::open(&vault, &pp()).unwrap();
    assert_eq!(s.config().unwrap().fee_treatment, btctax_core::FeeTreatment::TreatmentB);
}
```

- [ ] **Step 2: Run → FAIL.** `cargo test -p btctax-cli --test verify_report config_set`

- [ ] **Step 3: Implement `src/cmd/admin.rs` (config half)**
```rust
//! `config`, `export-snapshot` (FR10), `backup-key` — administrative commands. Config surfaces the TP8
//! (c)/(b) treatment + the (single-variant) lot method; export/backup arrive in Task 15.
use crate::config::{read_config, set_fee_treatment};
use crate::{CliConfig, CliError, Session};
use btctax_core::FeeTreatment;
use btctax_store::Passphrase;
use std::path::Path;

pub fn show_config(vault_path: &Path, pp: &Passphrase) -> Result<CliConfig, CliError> {
    Session::open(vault_path, pp)?.config()
}

/// Persist a new TP8 fee treatment (None = leave unchanged), then return the resulting config.
pub fn set_config(
    vault_path: &Path,
    pp: &Passphrase,
    fee_treatment: Option<FeeTreatment>,
) -> Result<CliConfig, CliError> {
    let mut session = Session::open(vault_path, pp)?;
    if let Some(t) = fee_treatment {
        set_fee_treatment(session.conn(), t)?;
        session.save()?;
    }
    session.config()
}
```

- [ ] **Step 4: Wire `cmd/mod.rs`.** Add `pub mod admin;`.
- [ ] **Step 5: Run → PASS + gate + commit.**
```bash
cargo test -p btctax-cli --test verify_report config_set
cargo clippy --all-targets -p btctax-cli -- -D warnings && cargo fmt --check
git commit -am "feat(cli): config command (TP8 (c)/(b) + LotMethod; default (c) preserved)"
```

---

### Task 15: `export-snapshot` (FR10) + `backup-key`

**Files:** Extend `src/cmd/admin.rs`, `src/render.rs` (`write_csv_exports`); Create `tests/export.rs`.

**Interfaces — Consumes:** `Vault::{export_snapshot, backup_key}`, `Session::project`, `csv::Writer`. **Produces:** `cmd::admin::export_snapshot(vault_path, pp, out_dir) -> Result<PathBuf, CliError>` (store writes `snapshot.sqlite`; the CLI additionally writes FR10 CSVs of the projected ledger) and `cmd::admin::backup_key(vault_path, pp, out_path)`. **This is the sole NFR2 plaintext exception.**

- [ ] **Step 1: Failing test in `tests/export.rs`**
```rust
mod fixtures;
use btctax_cli::cmd;
use btctax_store::Passphrase;

fn pp() -> Passphrase {
    Passphrase::new("pw".into())
}

#[test]
fn export_snapshot_writes_sqlite_and_csvs_and_backup_key() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    cmd::import::run(&vault, &pp(), &[fixtures::coinbase_buy_sell_send(dir.path())]).unwrap();

    let out = dir.path().join("export");
    let sqlite = cmd::admin::export_snapshot(&vault, &pp(), &out).unwrap();
    assert!(sqlite.exists(), "snapshot.sqlite (store)");
    assert!(out.join("lots.csv").exists());
    assert!(out.join("disposals.csv").exists());
    assert!(out.join("removals.csv").exists());
    assert!(out.join("income.csv").exists());

    let key = dir.path().join("backup.asc");
    cmd::admin::backup_key(&vault, &pp(), &key).unwrap();
    assert!(key.exists());
}
```

- [ ] **Step 2: Run → FAIL.** `cargo test -p btctax-cli --test export`

- [ ] **Step 3: Implement `write_csv_exports` in `src/render.rs`**
```rust
use btctax_core::LedgerState;
use csv::Writer; // C1: bring `csv::Writer` into scope (the `?` below yields `csv::Error`, now a CliError variant)
use std::path::Path;

/// FR10: write the projected ledger as CSV (the NFR2 plaintext exception). One row per disposal/removal
/// leg (flattened) + one per lot/income record. Exact values (Decimal/i64) as strings (NFR5).
/// C1: every `Writer::from_path`/`write_record`/`flush` returns `Result<_, csv::Error>`; the `?`
/// operator converts via `CliError::Csv(#[from] csv::Error)` (Task 0).
pub fn write_csv_exports(out_dir: &Path, state: &LedgerState) -> Result<(), crate::CliError> {
    std::fs::create_dir_all(out_dir)?;

    let mut w = Writer::from_path(out_dir.join("lots.csv"))?;
    w.write_record(["origin_event", "split", "wallet", "acquired_at", "remaining_sat", "usd_basis", "basis_source", "basis_pending"])?;
    for l in &state.lots {
        w.write_record([
            l.lot_id.origin_event_id.canonical(),
            l.lot_id.split_sequence.to_string(),
            wallet_label(&l.wallet),
            l.acquired_at.to_string(),
            l.remaining_sat.to_string(),
            l.usd_basis.to_string(),
            format!("{:?}", l.basis_source),
            l.basis_pending.to_string(),
        ])?;
    }
    w.flush()?;

    let mut w = Writer::from_path(out_dir.join("disposals.csv"))?;
    w.write_record(["event", "kind", "disposed_at", "lot", "sat", "proceeds", "basis", "gain", "term", "gift_zone"])?;
    for d in &state.disposals {
        for leg in &d.legs {
            w.write_record([
                d.event.canonical(),
                format!("{:?}", d.kind),
                d.disposed_at.to_string(),
                format!("{}#{}", leg.lot_id.origin_event_id.canonical(), leg.lot_id.split_sequence),
                leg.sat.to_string(),
                leg.proceeds.to_string(),
                leg.basis.to_string(),
                leg.gain.to_string(),
                format!("{:?}", leg.term),
                leg.gift_zone.map(|z| format!("{z:?}")).unwrap_or_default(),
            ])?;
        }
    }
    w.flush()?;

    let mut w = Writer::from_path(out_dir.join("removals.csv"))?;
    w.write_record(["event", "kind", "removed_at", "lot", "sat", "basis", "fmv_at_transfer", "term"])?;
    for r in &state.removals {
        for leg in &r.legs {
            w.write_record([
                r.event.canonical(),
                format!("{:?}", r.kind),
                r.removed_at.to_string(),
                format!("{}#{}", leg.lot_id.origin_event_id.canonical(), leg.lot_id.split_sequence),
                leg.sat.to_string(),
                leg.basis.to_string(),
                leg.fmv_at_transfer.to_string(),
                format!("{:?}", leg.term),
            ])?;
        }
    }
    w.flush()?;

    let mut w = Writer::from_path(out_dir.join("income.csv"))?;
    w.write_record(["event", "kind", "recognized_at", "sat", "usd_fmv", "business"])?;
    for i in &state.income_recognized {
        w.write_record([
            i.event.canonical(),
            format!("{:?}", i.kind),
            i.recognized_at.to_string(),
            i.sat.to_string(),
            i.usd_fmv.to_string(),
            i.business.to_string(),
        ])?;
    }
    w.flush()?;
    Ok(())
}
```

- [ ] **Step 4: Implement the export/backup half of `src/cmd/admin.rs`**
```rust
use crate::render::write_csv_exports;
use std::path::PathBuf;

/// FR10 / NFR2 exception: decrypted SQLite image (via the store) + the projected ledger as CSV.
pub fn export_snapshot(
    vault_path: &Path,
    pp: &Passphrase,
    out_dir: &Path,
) -> Result<PathBuf, CliError> {
    let session = Session::open(vault_path, pp)?;
    let sqlite = session.vault().export_snapshot(out_dir)?; // writes out_dir/snapshot.sqlite
    let (state, _cfg) = session.project()?;
    write_csv_exports(out_dir, &state)?;
    Ok(sqlite)
}

/// §8: export the passphrase-protected key (escape hatch; HIGH-security write).
pub fn backup_key(vault_path: &Path, pp: &Passphrase, out_path: &Path) -> Result<(), CliError> {
    Session::open(vault_path, pp)?.vault().backup_key(out_path)?;
    Ok(())
}
```

- [ ] **Step 5: Run → PASS + gate + commit.**
```bash
cargo test -p btctax-cli --test export
cargo clippy --all-targets -p btctax-cli -- -D warnings && cargo fmt --check
git commit -am "feat(cli): export-snapshot (SQLite + FR10 CSV) + backup-key (NFR2 exception, §8)"
```

---

### Task 16: `main.rs` clap dispatch + end-to-end integration test

**Files:** Rewrite `src/main.rs`; Create `tests/end_to_end.rs`.

**Interfaces — Consumes:** the whole `cmd` surface. **Produces:** the `btctax` binary (clap-4 derive; passphrase resolution; exit codes) and the §16.5 capstone test (init→import-synthetic→verify→reconcile→report→verify) over a temp vault.

- [ ] **Step 1: Failing end-to-end test `tests/end_to_end.rs`** (drives the **library**, deterministically)
```rust
mod fixtures;
use btctax_cli::{cmd, render, Session};
use btctax_core::{DisposeKind, OutflowClass};
use btctax_store::Passphrase;
use time::macros::datetime;

fn pp() -> Passphrase {
    Passphrase::new("pw".into())
}

#[test]
fn full_lifecycle_init_import_verify_reconcile_report() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    let now = datetime!(2026-02-01 12:00:00 UTC);

    // init
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    // import synthetic Coinbase (Buy + Sell + Send→pending)
    cmd::import::run(&vault, &pp(), &[fixtures::coinbase_buy_sell_send(dir.path())]).unwrap();

    // verify: the Send is pending (advisory), conservation balances, no hard blockers
    let v1 = cmd::inspect::verify(&vault, &pp()).unwrap();
    assert!(v1.conservation.balanced);
    assert_eq!(v1.pending, 1);
    assert!(!v1.has_hard_blockers());

    // reconcile: reclassify the pending Send as a Sell (discover its eventref from the projection)
    let out_ref = {
        let s = Session::open(&vault, &pp()).unwrap();
        let (st, _) = s.project().unwrap();
        st.pending_reconciliation[0].event.canonical()
    };
    cmd::reconcile::reclassify_outflow(
        &vault,
        &pp(),
        &out_ref,
        OutflowClass::Dispose { kind: DisposeKind::Sell },
        btctax_cli::eventref::parse_usd_arg("2050.00").unwrap(),
        Some(btctax_cli::eventref::parse_usd_arg("2.50").unwrap()),
        now,
    )
    .unwrap();

    // report: two 2025 disposals now (the original Sell + the reclassified Send)
    let state = cmd::inspect::report(&vault, &pp(), Some(2025)).unwrap();
    assert_eq!(state.disposals.len(), 2);
    let text = render::render_report(&state, Some(2025));
    assert!(text.contains("Disposals (year 2025)"));

    // verify again: nothing pending; still no hard blockers
    let v2 = cmd::inspect::verify(&vault, &pp()).unwrap();
    assert_eq!(v2.pending, 0);
    assert!(!v2.has_hard_blockers());
}
```

- [ ] **Step 2: Run → FAIL/﻿build.** `cargo test -p btctax-cli --test end_to_end`

- [ ] **Step 3: Rewrite `src/main.rs` (clap-4 derive dispatch)**
```rust
//! btctax — thin clap-4 dispatch over the btctax_cli library. Resolves the passphrase (env seam for
//! non-interactive use; otherwise a secure prompt), calls one library command, renders, and sets the
//! exit code (non-zero on FR9 hard blockers / on any CliError). NO business logic lives here.
use btctax_cli::{cmd, eventref, render, CliError};
use btctax_core::{
    AllocMethod, DisposeKind, FeeTreatment, IncomeKind, InboundClass, OutflowClass, TransferTarget,
};
use btctax_store::Passphrase;
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use std::process::ExitCode;
use time::OffsetDateTime;

#[derive(Parser)]
#[command(name = "btctax", about = "Offline US Bitcoin tax ledger (Phase 1)")]
struct Cli {
    /// Path to the encrypted vault (vault.pgp).
    #[arg(long, global = true, default_value = "vault.pgp")]
    vault: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create the encrypted vault + force a key backup.
    Init {
        #[arg(long)]
        key_backup: PathBuf,
    },
    /// Import one or more export files (auto-groups Swan).
    Import { files: Vec<PathBuf> },
    /// FR9 integrity check (non-zero exit on hard blockers).
    Verify,
    /// Show holdings + realized disposals/removals/income.
    Report {
        #[arg(long)]
        year: Option<i32>,
    },
    /// Emit a reconciliation decision event.
    #[command(subcommand)]
    Reconcile(Reconcile),
    /// Show or set projection config (TP8 fee treatment).
    Config {
        #[arg(long, value_enum)]
        set_fee_treatment: Option<FeeArg>,
    },
    /// FR10: export decrypted SQLite + CSV (the NFR2 plaintext exception).
    ExportSnapshot {
        #[arg(long)]
        out: PathBuf,
    },
    /// Export the passphrase-protected key.
    BackupKey {
        #[arg(long)]
        out: PathBuf,
    },
}

#[derive(Subcommand)]
enum Reconcile {
    /// Confirm a self-transfer (TransferLink).
    LinkTransfer {
        out: String,
        #[arg(long, conflicts_with = "to_wallet")]
        to_event: Option<String>,
        #[arg(long)]
        to_wallet: Option<String>,
    },
    /// Classify an inbound TransferIn as income.
    ClassifyInboundIncome {
        in_ref: String,
        #[arg(long)]
        kind: String,
        #[arg(long)]
        fmv: Option<String>,
        #[arg(long)]
        business: bool,
    },
    /// Classify an inbound TransferIn as a received gift.
    ClassifyInboundGift {
        in_ref: String,
        #[arg(long)]
        fmv_at_gift: String,
        #[arg(long)]
        donor_basis: Option<String>,
        #[arg(long)]
        donor_acquired: Option<String>,
    },
    /// Reclassify a pending TransferOut.
    ReclassifyOutflow {
        out: String,
        #[arg(long, value_enum)]
        as_kind: OutKindArg,
        #[arg(long)]
        amount: String,
        #[arg(long)]
        fee: Option<String>,
        #[arg(long)]
        appraisal: bool,
    },
    /// Set a manual FMV on an event.
    SetFmv { event: String, #[arg(long)] fmv: String },
    /// Void a revocable decision.
    Void { target: String },
    /// Resolve an Unclassified row from a JSON imported payload.
    ClassifyRaw { target: String, #[arg(long)] payload_json: String },
    /// Accept an import conflict.
    AcceptConflict { conflict: String },
    /// Reject an import conflict.
    RejectConflict { conflict: String },
    /// Path-B safe-harbor allocate (from the actual pre-2025 position).
    SafeHarborAllocate {
        #[arg(long, value_enum, default_value_t = MethodArg::Actual)]
        method: MethodArg,
        #[arg(long)]
        attest: bool,
    },
    /// Attest an existing allocation as timely.
    SafeHarborAttest,
}

#[derive(Copy, Clone, ValueEnum)]
enum FeeArg { C, B }
#[derive(Copy, Clone, ValueEnum)]
enum OutKindArg { Sell, Spend, Gift, Donate }
#[derive(Copy, Clone, ValueEnum)]
enum MethodArg { Actual, ProRata }

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(2)
        }
    }
}

/// Resolve the passphrase: `BTCTAX_PASSPHRASE` (non-interactive/scripted) else a secure prompt.
fn passphrase(confirm: bool) -> Result<Passphrase, CliError> {
    if let Ok(p) = std::env::var("BTCTAX_PASSPHRASE") {
        return Ok(Passphrase::new(p));
    }
    let p = rpassword::prompt_password("Vault passphrase: ")
        .map_err(CliError::Io)?;
    if confirm {
        let again = rpassword::prompt_password("Confirm passphrase: ").map_err(CliError::Io)?;
        if again != p {
            return Err(CliError::Usage("passphrases do not match".into()));
        }
    }
    Ok(Passphrase::new(p))
}

fn run() -> Result<ExitCode, CliError> {
    let cli = Cli::parse();
    let vault = cli.vault.as_path();
    let now = OffsetDateTime::now_utc();

    match cli.command {
        Command::Init { key_backup } => {
            cmd::init::run(vault, &passphrase(true)?, &key_backup)?;
            println!("Initialized vault {} (key backed up to {})", vault.display(), key_backup.display());
        }
        Command::Import { files } => {
            let (reports, import) = cmd::import::run(vault, &passphrase(false)?, &files)?;
            print!("{}", render::render_file_reports(&reports, &import));
        }
        Command::Verify => {
            let report = cmd::inspect::verify(vault, &passphrase(false)?)?;
            print!("{}", render::render_verify(&report));
            if report.has_hard_blockers() {
                return Ok(ExitCode::from(1));
            }
        }
        Command::Report { year } => {
            let state = cmd::inspect::report(vault, &passphrase(false)?, year)?;
            print!("{}", render::render_report(&state, year));
        }
        Command::Reconcile(r) => dispatch_reconcile(vault, r, now)?,
        Command::Config { set_fee_treatment } => {
            let pp = passphrase(false)?;
            let cfg = match set_fee_treatment {
                Some(FeeArg::C) => cmd::admin::set_config(vault, &pp, Some(FeeTreatment::TreatmentC))?,
                Some(FeeArg::B) => cmd::admin::set_config(vault, &pp, Some(FeeTreatment::TreatmentB))?,
                None => cmd::admin::show_config(vault, &pp)?,
            };
            println!("fee_treatment: {:?}\nlot_method: {:?}", cfg.fee_treatment, cfg.lot_method);
        }
        Command::ExportSnapshot { out } => {
            let p = cmd::admin::export_snapshot(vault, &passphrase(false)?, &out)?;
            println!("Exported {} + CSVs to {}", p.display(), out.display());
        }
        Command::BackupKey { out } => {
            cmd::admin::backup_key(vault, &passphrase(false)?, &out)?;
            println!("Key backed up to {}", out.display());
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn dispatch_reconcile(vault: &std::path::Path, r: Reconcile, now: OffsetDateTime) -> Result<(), CliError> {
    let pp = passphrase(false)?;
    let id = match r {
        Reconcile::LinkTransfer { out, to_event, to_wallet } => {
            let target = match (to_event, to_wallet) {
                (Some(ev), None) => TransferTarget::InEvent(eventref::parse_event_id(&ev)?),
                (None, Some(w)) => TransferTarget::Wallet(eventref::parse_wallet_id(&w)?),
                _ => return Err(CliError::Usage("exactly one of --to-event / --to-wallet required".into())),
            };
            cmd::reconcile::link_transfer(vault, &pp, &out, target, now)?
        }
        Reconcile::ClassifyInboundIncome { in_ref, kind, fmv, business } => {
            let fmv = fmv.as_deref().map(eventref::parse_usd_arg).transpose()?;
            let class = InboundClass::Income { kind: eventref::parse_income_kind(&kind)?, fmv, business };
            cmd::reconcile::classify_inbound(vault, &pp, &in_ref, class, now)?
        }
        Reconcile::ClassifyInboundGift { in_ref, fmv_at_gift, donor_basis, donor_acquired } => {
            let class = InboundClass::GiftReceived {
                donor_basis: donor_basis.as_deref().map(eventref::parse_usd_arg).transpose()?,
                donor_acquired_at: donor_acquired.as_deref().map(eventref::parse_date_arg).transpose()?,
                fmv_at_gift: eventref::parse_usd_arg(&fmv_at_gift)?,
            };
            cmd::reconcile::classify_inbound(vault, &pp, &in_ref, class, now)?
        }
        Reconcile::ReclassifyOutflow { out, as_kind, amount, fee, appraisal } => {
            let class = match as_kind {
                OutKindArg::Sell => OutflowClass::Dispose { kind: DisposeKind::Sell },
                OutKindArg::Spend => OutflowClass::Dispose { kind: DisposeKind::Spend },
                OutKindArg::Gift => OutflowClass::GiftOut,
                OutKindArg::Donate => OutflowClass::Donate { appraisal_required: appraisal },
            };
            let principal = eventref::parse_usd_arg(&amount)?;
            let fee = fee.as_deref().map(eventref::parse_usd_arg).transpose()?;
            cmd::reconcile::reclassify_outflow(vault, &pp, &out, class, principal, fee, now)?
        }
        Reconcile::SetFmv { event, fmv } => {
            cmd::reconcile::set_fmv(vault, &pp, &event, eventref::parse_usd_arg(&fmv)?, now)?
        }
        Reconcile::Void { target } => cmd::reconcile::void(vault, &pp, &target, now)?,
        Reconcile::ClassifyRaw { target, payload_json } => {
            cmd::reconcile::classify_raw(vault, &pp, &target, &payload_json, now)?
        }
        Reconcile::AcceptConflict { conflict } => cmd::reconcile::accept_conflict(vault, &pp, &conflict, now)?,
        Reconcile::RejectConflict { conflict } => cmd::reconcile::reject_conflict(vault, &pp, &conflict, now)?,
        Reconcile::SafeHarborAllocate { method, attest } => {
            let m = match method { MethodArg::Actual => AllocMethod::ActualPosition, MethodArg::ProRata => AllocMethod::ProRata };
            cmd::reconcile::safe_harbor_allocate(vault, &pp, m, attest, now)?
        }
        Reconcile::SafeHarborAttest => cmd::reconcile::safe_harbor_attest(vault, &pp, now)?,
    };
    println!("Recorded decision {}", id.canonical());
    Ok(())
}
```

- [ ] **Step 4: Run → PASS** (lib e2e green; the binary compiles). `cargo test -p btctax-cli --test end_to_end`
- [ ] **Step 5: Full-suite gate + commit.**
```bash
cargo test -p btctax-cli && cargo clippy --all-targets -p btctax-cli -- -D warnings && cargo fmt --check
git commit -am "feat(cli): main.rs clap dispatch + exit codes + end-to-end lifecycle test"
```

---

## Self-Review — spec coverage map (FR1–FR10 + reconciliation/config/§8/§11 → task)

**Functional requirements:**
- **FR1 Import (atomic append, idempotent, ImportConflict).** `cmd::import::run` → `append_import_batch` (atomic; idempotency + conflict detection are core's) → **Task 4**; conflict resolution emitters (`accept_conflict`/`reject_conflict`) → **Task 12**; surfaced counts → `render_file_reports` (Task 4).
- **FR2 BTC-only filter + per-file counts.** Adapters drop/unclassify; the CLI surfaces `FileReport { dropped_no_btc, unclassified }` → **Task 4** (asserted: ETH row dropped).
- **FR3 FMV resolution.** Ingest-time FMV is adapters'; the CLI's `set-fmv` (`ManualFmv`) supplies a manual FMV and clears `fmv_missing` → **Task 11**.
- **FR4 Projection display.** `cmd::inspect::report` + `render_report` (lots/holdings/disposals proceeds·basis·gain·ST-LT/removals/income; `--year`) → **Task 5**.
- **FR5 Wallets.** Wallet identity surfaced in holdings/lots (`wallet_label`) and accepted as a `TransferLink` target (`--to-wallet exchange:…/self:…`) → **Tasks 5/7/8**. (Self-custody wallets are referenced by label at reconcile time; the engine creates the pool — no separate create-wallet event in core.)
- **FR6 Reconciliation.** `link-transfer` (Task 8), `classify-inbound` (Task 9), `reclassify-outflow` (Task 10); pending items listed by `verify` (Task 6).
- **FR7 2025 transition.** Path A = default (no event); Path B = `safe-harbor allocate` (+ `attest`) emitting `SafeHarborAllocation` → **Task 13**; status shown by `verify` → **Task 6**.
- **FR8 Corrections.** `void` (`VoidDecisionEvent`) → **Task 11**; non-revocable/effective-target voids surface as `decision_conflicts` (engine), shown by `verify`.
- **FR9 Integrity (`verify`).** `conservation_report` + hard/advisory blockers + pending + unknown-basis + safe-harbor status + **non-zero exit on hard blockers** → **Task 6** (+ exit wiring Task 16).
- **FR10 Export.** `export-snapshot` (store `snapshot.sqlite` + CLI-written `write_csv_exports`) and `backup-key` → **Task 15**; the sole NFR2 plaintext exception.

**Reconciliation decision coverage (§6.4 / §7.2 — every variant the CLI must emit):** `TransferLink` (8), `ClassifyInbound` (9), `ReclassifyOutflow` (10), `ManualFmv` + `VoidDecisionEvent` (11), `ClassifyRaw` + `SupersedeImport` + `RejectImport` (12), `SafeHarborAllocation` (13). All ten reconcile-emitting decision variants are covered; each is append-only + re-projectable (idempotent at the engine level).

**Config / TP8 (USER-MANDATED):** `cli_config` table + `CliConfig`/`ProjectionConfig` with **(c) default never flipped** (Task 2); `config` command surface (Task 14). `LotMethod::Fifo` is the single Phase-1 variant (surfaced read-only). Path A/B is **not** a `ProjectionConfig` field — Path A is default; Path B is the `SafeHarborAllocation` event (Task 13).

**§8 vault session lifecycle:** `init` (create + schema + forced key backup) → **Task 3**; every mutating command opens a `Session` (flock; NFR7), appends, and `save`s (encrypted, atomic; NFR2/NFR3) → **Task 1**; `backup-key` / `export-snapshot` escape hatches → **Task 15**.

**§11 command surface:** `init`(3) · `import`(4) · `verify`(6) · `report`/`show`(5) · `reconcile`(8–13) · `config`(14) · `export-snapshot`/`backup-key`(15) · binary dispatch(16). (`wallets`/`holdings [--at]`/`lots`/`events`/`fmv`/`reconstruct-2025`/`allocate-2025` from the §11 sketch are realized here as: holdings+lots inside `report`; `fmv` as `reconcile set-fmv`; `reconstruct-2025` as the no-op Path-A default; `allocate-2025` as `reconcile safe-harbor allocate`. The standalone `wallets`/`events` listing commands are deferred — see Open question O5.)

**NFR / privacy / determinism:**
- **NFR2/3/7:** save-on-mutate, flock-per-command, export-only-plaintext — Tasks 1/4/15.
- **NFR4 determinism:** the CLI passes `load_all` straight to `project`; decision `now` is injected (deterministic tests) — Tasks 1/8–13/16.
- **NFR5 exact arithmetic:** `parse_usd_arg` (string→Decimal); satoshis as `i64`; no CLI money math — Task 7.
- **PRIVACY:** every test uses a temp vault + synthetic fixtures (real §9.1 headers, invented values); no real file read — Tasks 4/16 fixtures.

**Cross-task type/signature consistency (verified against the read crate sources):**
- `Vault::conn() -> &Connection` (there is **no `conn_mut`**); core appenders take `&Connection` and use `unchecked_transaction` — every `append_*` call site passes `session.conn()`, and `save(&mut self)` runs in a later statement (no overlapping borrow).
- `append_decision(conn, payload, OffsetDateTime, UtcOffset, Option<WalletId>)` — all reconcile fns pass `(now, UtcOffset::UTC, None)` (decisions are not wallet-scoped; §6.3).
- Decision payload field names match core verbatim: `TransferLink{out_event,in_event_or_wallet}`, `ReclassifyOutflow{transfer_out_event,as_,principal_proceeds_or_fmv,fee_usd}`, `ClassifyInbound{transfer_in_event,as_}`, `ManualFmv{event,usd_fmv}`, `SafeHarborAllocation{lots,as_of_date,method,timely_allocation_attested}` with `AllocLot{wallet,sat,usd_basis,acquired_at}`, `SupersedeImport{conflict_event}`, `RejectImport{conflict_event}`, `VoidDecisionEvent{target_event_id}`, `ClassifyRaw{target,as_}`; enums `OutflowClass`/`InboundClass`/`AllocMethod`/`TransferTarget`/`DisposeKind`/`IncomeKind` used by value match core.
- `LedgerState` fields read verbatim: `lots`, `holdings_by_wallet` (`BTreeMap<WalletId,Sat>`), `disposals`/`removals` (`legs`), `income_recognized`, `pending_reconciliation`, `blockers`, `stats`; `Blocker.kind.severity()`; `conservation_report(&LedgerState) -> ConservationReport` fields (`sigma_*`, `balanced`, `has_uncovered`).
- `EventId::canonical()` formats verified (`import|<tag>|<source_ref>` etc.); `eventref::parse_event_id` is its inverse, tolerating `|` inside `source_ref` (the adapters mint direction-scoped refs like `out|cb-send`); round-trip tested (Task 7).
- store `export_snapshot` writes **only** `snapshot.sqlite` (verified in `vault.rs`); the FR10 CSVs are therefore the CLI's own `write_csv_exports` (Task 15).
- `CliError` is the single crate error; `#[from]` for `StoreError`/`CoreError`/`AdapterError`/`rusqlite::Error`/`csv::Error`/`io::Error` are non-overlapping source types (no conflicting `From` — `csv::Error` is a distinct type from `io::Error`, so C1's `Csv(#[from] csv::Error)` does not collide with `Io(#[from] io::Error)`); style matches the other crates' `thiserror` errors.

**Out of scope (correctly absent):** form generation, the optimizer, non-BTC assets, GUI, online pricing, multi-user (§15/§16). The CLI surfaces Phase-1 state only.

## Notes for the owner / FOLLOWUPS candidates

**Open questions needing the owner's decision (CLI UX / reconciliation-workflow choices):**
- **O1 — Event-reference UX.** Reconcile targets are specified by the canonical `EventId` string (e.g. `import|coinbase|out|cb-send`), discovered from `verify`/`report` output. Alternative: a stable short index printed by a pending-items list (`reconcile list`). Canonical strings are deterministic + scriptable but verbose; confirm whether a short-index alias is wanted. *(If yes: add a `reconcile list` command + an index→EventId map; small follow-up.)*
- **O2 — FR10 CSV scope.** The CLI writes `lots/disposals/removals/income.csv` (leg-flattened) alongside the store's `snapshot.sqlite`. Confirm the desired CSV set/columns (e.g. a holdings.csv, or an 8949-shaped pre-export) — though 8949 itself is Phase 2.
- **O3 — Config persistence vs NFR6.** TP8 treatment + lot method live in a `cli_config` side-table inside the vault DB (rides the encrypted blob), **not** as a ledger event. This is a projection *input parameter*, not ledger state, so it does not violate "the log is the sole source of truth" for the ledger — but it is the one piece of persisted non-event state. Confirm acceptable, or prefer a `--fee-treatment` flag passed per projection with the table only as a remembered default.
- **O4 — `safe-harbor allocate --method pro-rata`.** Both methods currently seed `AllocLot`s from the **actual** pre-2025 remaining lots; a true ProRata cross-wallet redistribution is not auto-derived (it needs target wallet weights). Confirm whether ProRata should accept a manual `--lots-json` allocation instead of deriving from actuals.
- **O5 — Deferred §11 listing commands.** The standalone `wallets` and `events [--filter]` listing commands from the §11 sketch are folded into `report`/`verify` here (holdings+lots in `report`; pending+blockers in `verify`). Confirm that is sufficient for Phase 1, or schedule them as a thin follow-up.

**FOLLOWUPS candidates (record after first build):** pin the resolved `clap`/`rpassword`/`csv` versions and confirm a single unified `rusqlite` in the workspace; the §9.1 Swan-`deposit` basis re-supply now has a concrete home (`reconcile classify-inbound`) — note it as the operator step for externally-sourced inbounds; the `classify-raw --payload-json` UX (power-user JSON; a future typed sub-form set could replace it).

**Confirmation:** the plan reads **no real data**. Every command function takes an explicit `vault_path` + injected `&Passphrase`; every test uses `tempfile::tempdir()` vaults and synthetic in-test CSV fixtures built from the confirmed §9.1 header names with invented values. No task, test, code path, or tool invocation in this plan reads `~/Documents/BitcoinTax/ReadOnly` or any real export/vault.

## Fold record
Per STANDARD_WORKFLOW §2: both round-1 reviews were persisted verbatim under `reviews/` BEFORE folding
(`reviews/plan-foundation-04-cli-engineering-round-1.md`,
`reviews/plan-foundation-04-cli-reconciliation-round-1.md`); a re-review follows this fold (gate = 0
Critical / 0 Important from both reviewers).

### Fold record (round 1)
Maps each blocking finding (and the folded non-blocking items) to its fix in this plan.

| Finding | Sev | Source | Fix |
|---|---|---|---|
| **C1** — `CliError` has no `Csv` variant → `write_csv_exports` (Task 15) won't compile (`?` on `csv::Writer` ops yields `csv::Error`, not covered by `Io(#[from] io::Error)`). | Critical | eng | Added `#[error("csv: {0}")] Csv(#[from] csv::Error)` to the `CliError` enum (Task 0 Step 3); added `Csv` to the "Public interface this plan PRODUCES" variant list and to the Self-Review `#[from]`-source-types line (noting `csv::Error` ≠ `io::Error`, no `From` collision); switched render.rs (Task 15) to `use csv::Writer;` + `Writer::from_path` (clippy-safe vs `use csv;`/`single_component_path_imports`). |
| **I-1** — `safe-harbor allocate` seeded `AllocLot`s from the FULL projection's `state.lots` (post-2025-disposal residual) → `alloc_sat < universal_snapshot.held_sat` → hard `SafeHarborUnconservable` → Path B unusable. | Important | recon | Rewrote `safe_harbor_allocate` (Task 13 Step 3) to seed from a **pre-2025-only re-projection**: `load_all` → keep import events with `tax_date(..) < TRANSITION_DATE` + keep ALL decisions/conflicts (they shape the residue) − DROP prior `SafeHarborAllocation` (allocation-independent) → `project(pre2025, prices, cfg)` → build `AllocLot`s from THAT state's lots. This equals the engine's `universal_snapshot` (identical `fold_event` arms; Path A relocation preserves sat/basis 1:1; `finalize` attributes Universal lots by `lot.wallet`, giving ActualPosition's per-wallet assignment). The engine's conservation guard is kept as the backstop. Added an I-1 regression test (allocate after a 2025 Sell consumed part of a pre-2025 lot → seeds the FULL 0.20 BTC residue, no `SafeHarborUnconservable`). |
| **I-2(a) / Eng-I1** — `safe_harbor_attest` counted VOIDED allocations in its single-allocation guard → the allocate→inert→void→re-allocate→attest workflow tripped "multiple allocations present." | Important | recon + eng | Rewrote `safe_harbor_attest` to build the voided-target set from `VoidDecisionEvent`s (mirrors resolve.rs pass-1 step 1a) and exclude voided allocations, so only LIVE allocations count. Added a regression test (allocate→void→re-allocate→attest succeeds). |
| **I-2(b)** — `safe_harbor_attest` on an ALREADY-EFFECTIVE allocation appended a Void-of-effective (→ irrevocable `decision_conflicts`) + a second effective allocation (→ two effective → Path A), permanently breaking Path B. | Important | recon | `safe_harbor_attest` now re-projects the live log and reads the engine's verdict on the prior allocation: `SafeHarborUnconservable` → refuse (attestation can't cure conservation); neither timebar nor unconservable → **already effective** → refuse with `CliError::Usage("allocation already effective; no attestation needed — run \`verify\`")` and append NOTHING; ONLY a pure `SafeHarborTimebar` proceeds to the void + re-attest. Added a regression test (already-effective → Err advising `verify`; log unmutated). |
| **M3** — `rust-version.workspace = true` omitted from `btctax-cli/Cargo.toml`. | Minor | eng | Added `rust-version.workspace = true` to `[package]` (Task 0 Step 2). |
| **N-2** — attest error message advised voiding the effective allocation (irrecoverable). | Nit | recon | Folded into the I-2(b) fix: the already-effective path advises running `verify`, never "void the effective one." |
| **N1** (eng) — `let mut session = session;` rebinding in allocate. | Nit | eng | Resolved: `safe_harbor_allocate` now binds `let mut session` up front; the rebinding is gone. |

**Deferred to FOLLOWUPS (non-blocking; recorded, not folded into code):** recon **M-2** (`AllocLot` has no `dual_loss_basis` → pre-2025 received-gift lots lose §1015(a) dual basis under Path B — spec-faithful, Phase-2), **M-1** (`verify` double-loads events → a `Session::load_events_and_project()`), eng **M2** (render/CSV use `{:?}` Debug for enums → add `Display`/`tag()` before CSV consumers depend on the format), recon **N-1** (strengthen the `set-fmv` test to an `Income{Missing}` target asserting the blocker clears). Also recorded: the attest void+re-append leaves the original (now-voided) allocation still evaluated by resolve (allocation-voids don't suppress step-3 evaluation), so a stale advisory `safe_harbor_timebar` can persist and `safe_harbor_status` may mislabel an effective Path B as "time-barred" — display-only, advisory.

### Self-consistency pass (post-fold)
- **`CliError` variants consistent everywhere:** enum (Task 0) = Public-interface list = Self-Review `#[from]` line = `{Store, Core, Adapter, Sqlite, Csv, Io, BadEventRef, Usage}`; `Csv(#[from] csv::Error)` is the only new variant and is a distinct source type (no `From` collision).
- **`safe_harbor_allocate` compiles in principle:** all reads verified against live source — `load_all(&Connection)`, `tax_date(OffsetDateTime, UtcOffset) -> TaxDate` + `TRANSITION_DATE` (`btctax_core::conventions`), `LedgerEvent.{id,utc_timestamp,original_tz,payload}` (pub), `EventId::Import{..}` variant, `project(&[LedgerEvent], &dyn PriceProvider, &ProjectionConfig) -> LedgerState`, `BundledPrices::load() -> Result<_, AdapterError>` (impl `PriceProvider`), `CliConfig::to_projection()`, `Lot.{remaining_sat,usd_basis,wallet,acquired_at}` (pub), `AllocLot{wallet,sat,usd_basis,acquired_at}`. Imports added once at the Task-13 section top; `load_all` shared by both fns (no duplicate `use`).
- **`safe_harbor_attest` compiles in principle:** `VoidDecisionEvent{target_event_id}`, `SafeHarborAllocation{..}` struct-update `..prior`, `append_decision(conn, payload, OffsetDateTime, UtcOffset, None)`, `Session::project() -> (LedgerState, ProjectionConfig)`, `Blocker.{event: Option<EventId>, kind: BlockerKind}`, `BlockerKind::{SafeHarborUnconservable, SafeHarborTimebar}`. The two effectiveness booleans are computed before `prior_id` is moved into the Void (no borrow-after-move).
- **No new placeholder introduced;** no `todo!()`/`unimplemented!()`/`(())`. Cross-task type/signature consistency section already enumerates the field names used here and remains accurate.
