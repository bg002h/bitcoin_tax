//! `Session` wraps one open `btctax_store::Vault` and is the single seam every command opens. The
//! passphrase is ALWAYS a parameter — production resolves it in `main` (prompt/env); tests inject a
//! constructed `Passphrase`. `project()` runs the pure core projection over the bundled price dataset.
use crate::bulk_estimated;
use crate::config::{self, CliConfig};
use crate::donation_details;
use crate::optimize_attest;
use crate::CliError;
use crate::{return_inputs, tax_profile};
use btctax_adapters::{BundledFullReturnTables, BundledTaxTables};
use btctax_core::conventions::{round_cents, tax_date, TRANSITION_DATE};
use btctax_core::persistence::{init_schema, load_all};
use btctax_core::tax::tables::FullReturnTables;
use btctax_core::{project, LedgerEvent, LedgerState, PriceProvider, ProjectionConfig, TaxTables};
use btctax_core::{
    AllocLot, BlockerKind, DonationDetails, EventId, EventPayload, LotMethod, PendingTransfer, Sat,
    TaxDate, TaxProfile, Usd, WalletId,
};
use btctax_store::{Passphrase, Vault};
use rusqlite::Connection;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

// ── Bulk link-transfer plan (bulk-link-transfer D1) ──────────────────────────
//
// The shared, READ-ONLY plan both the CLI (`cmd::reconcile::bulk_link_plan`) and the TUI priced
// preview compute from the HELD session. Modeled on `optimize_proposal`/`safe_harbor_residue`: a
// `&self` read helper that appends and persists NOTHING.

/// Time-frame selector for a bulk link-transfer plan. `Range` bounds are INCLUSIVE.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Frame {
    All,
    Year(i32),
    Range { from: TaxDate, to: TaxDate },
}

/// Filter narrowing which pending outbound transfers a bulk plan selects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkFilter {
    pub frame: Frame,
    pub from_wallet: Option<WalletId>,
}

/// One enriched pending outbound transfer in a bulk link-transfer plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkLinkRow {
    pub out_event: EventId,
    pub date: TaxDate,
    /// [R0-N2] ALWAYS `Some` for a pending out (a wallet-less TransferOut never reaches
    /// `pending_reconciliation`); `Option` kept defensively.
    pub source_wallet: Option<WalletId>,
    pub principal_sat: Sat,
    /// `fmv_of(prices, date, principal_sat)` [R0-M1]; advisory, `None` on missing price / overflow.
    pub usd_value: Option<Usd>,
    /// Σ leg `usd_basis` carried (over the principal+fee sats the legs cover); non-taxable → carries.
    pub basis_usd: Usd,
}

/// The read-only plan a bulk link-transfer would execute: the eligible/in-frame `included` rows, the
/// `skipped_same_wallet` rows (source == dest — a meaningless self-link), and the preview totals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkLinkPlan {
    pub dest: WalletId,
    /// Eligible + in-frame + passes `from_wallet` + source != dest. Sorted by `date`.
    pub included: Vec<BulkLinkRow>,
    /// Source wallet == dest → cannot self-link to itself.
    pub skipped_same_wallet: Vec<BulkLinkRow>,
    pub total_sat: Sat,
    /// [R0-I2] Σ of the priced `usd_value`s — a FLOOR, always a real number.
    pub total_usd_value_floor: Usd,
    /// [R0-I2] rows priced `None` → render "≥ $X (N unavailable)" vs exact "$X".
    pub missing_price_count: usize,
    /// Σ `basis_usd` over `included`.
    pub total_basis_usd: Usd,
}

// ── Bulk classify-inbound-self-transfer plan (bulk-classify-inbound-self-transfer D1) ─
//
// The shared, READ-ONLY plan both the CLI (`cmd::reconcile::bulk_self_transfer_in_plan`) and the TUI
// priced preview compute from the HELD session. A close MIRROR of `bulk_link_transfer_plan` applied to
// Cycle A's `InboundClass::SelfTransferMine` ($0 conservative basis, non-taxable). Appends and
// persists NOTHING.

/// Filter narrowing which pending unknown-basis inbound deposits a bulk STI plan selects. `wallet`
/// filters the RECEIVING wallet (the inbound has no "destination" — it IS the receiving leg).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkStiFilter {
    pub frame: Frame,
    pub wallet: Option<WalletId>,
}

/// One enriched pending unknown-basis inbound deposit in a bulk STI plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkStiRow {
    pub in_event: EventId,
    pub date: TaxDate,
    /// [R0-M2] ALWAYS `Some` (wallet-less inbounds are excluded — they create no lot); `Option` kept
    /// defensively / for display symmetry with `BulkLinkRow`.
    pub wallet: Option<WalletId>,
    pub sat: Sat,
    /// `fmv_of(prices, date, sat)` — the market value being given $0 basis; `None` on missing price.
    pub usd_fmv: Option<Usd>,
}

/// The read-only plan a bulk STI would execute: the eligible/in-frame `included` rows + preview totals.
/// No `skipped_*` bucket (an inbound has no self-destination to skip).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkStiPlan {
    /// Eligible + in-frame + passes `wallet` filter. Sorted by `date`.
    pub included: Vec<BulkStiRow>,
    pub total_sat: Sat,
    /// Σ of the priced `usd_fmv`s — the HONEST FLOOR (the over-tax exposure), always a real number.
    pub total_usd_fmv_floor: Usd,
    /// Rows priced `None` → render "≥ $X (N unavailable)" vs exact "$X".
    pub missing_price_count: usize,
}

// ── Bulk classify-inbound-income plan (bulk-classify-inbound-income, Cycle 4) ─
//
// The shared, READ-ONLY plan both the CLI (`cmd::reconcile::bulk_classify_income_plan`) and the TUI
// `I` flow compute from the HELD session. A NEAR-CLONE of `bulk_self_transfer_in_plan` with ONE
// load-bearing difference [#a tax-safety]: a candidate whose `fmv_of(date, sat)` is `None` (no bundled
// price / overflow) is EXCLUDED from `included` (counted in `excluded_missing_price`), NOT included —
// because a persisted `InboundClass::Income { fmv: None }` projects to a Hard `FmvMissing` year-gate
// (and on the inbound path that is NOT clearable by `ManualFmv` — the sole escape is void + reclassify).
// So `included` carries a RESOLVED `fmv: Usd` (non-Option), making `Income{fmv:None}` structurally
// unrepresentable from the bulk path. Appends and persists NOTHING.

/// Filter narrowing which pending unknown-basis inbound deposits a bulk classify-income plan selects.
/// Mirrors `BulkStiFilter`; `wallet` filters the RECEIVING wallet (the inbound IS the receiving leg).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkIncomeFilter {
    pub frame: Frame,
    pub wallet: Option<WalletId>,
}

/// One enriched pending unknown-basis inbound in a bulk classify-income plan, carrying a RESOLVED FMV.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkIncomeRow {
    pub in_event: EventId,
    pub date: TaxDate,
    pub sat: Sat,
    /// [#a] The RESOLVED auto-FMV `fmv_of(prices, date, sat)` — ALWAYS a real number (the `None`
    /// rows are EXCLUDED upstream). This is the income recognized AND the lot basis.
    pub fmv: Usd,
}

/// The read-only plan a bulk classify-income would execute: the eligible/in-frame `included` rows (each
/// with a resolved `fmv`) + the count of candidates dropped for a MISSING price + preview totals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkIncomePlan {
    /// Eligible + in-frame + passes `wallet` filter + HAS a price. Sorted by `date`.
    pub included: Vec<BulkIncomeRow>,
    /// [#a] Candidates dropped because `fmv_of == None` (surfaced, NOT silently dropped — the user
    /// learns N inbounds could not be auto-valued as income; they stay pending).
    pub excluded_missing_price: usize,
    pub total_sat: Sat,
    /// Σ `fmv` over `included` — the total income being recognized (always a real number).
    pub total_income_usd: Usd,
}

// ── Bulk reclassify-outflow plan (bulk-reclassify-outflow, Cycle 5 — the LAST) ─
//
// The shared, READ-ONLY plan both the CLI (`cmd::reconcile::bulk_reclassify_outflow_plan`) and the TUI
// `O` flow compute from the HELD session. The bulk analog of the single `o` reclassify-outflow: it
// sweeps MANY `pending_reconciliation` outflows to a `Dispose{Sell|Spend}` with the auto-FMV as
// ESTIMATED proceeds. Enriches over `pending_reconciliation` exactly as `bulk_link_transfer_plan`
// (session.rs) does. Appends/persists NOTHING.
//
// Load-bearing tax-safety [#a]: a candidate whose `fmv_of(date, principal_sat)` is `None` is EXCLUDED
// (counted in `excluded_missing_price`), NOT included — `ReclassifyOutflow.principal_proceeds_or_fmv`
// is `Usd` (NOT Option), so a missing-price row cannot be constructed WITHOUT fabricating a number, and
// (unlike bulk-income's LOUD `FmvMissing`) a fabricated/`0` proceeds would be SILENT (gates nothing,
// misreports gain/loss). So `included` carries a RESOLVED `fmv: Usd` (non-Option) — the load-bearing
// structural defense, mirroring `BulkIncomeRow.fmv: Usd`.
//
// Estimated gain [Q3]: `basis_usd = Σ pt.legs.usd_basis` (already computed by the fold's SINGLE
// chronological pass with all N candidate PendingOuts pending, so `Σ` over multiple rows' legs is NEVER
// double-counted — an earlier-dated PendingOut has already drawn the pool down before a later one
// folds). `estimated_gain = round_cents(fmv − basis_usd)` per row. Precedent: `bulk_link_transfer_plan`
// already sums `pt.legs.usd_basis`.

/// One enriched pending outbound transfer in a bulk reclassify-outflow plan, carrying a RESOLVED FMV.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkReclassifyOutflowRow {
    pub out_event: EventId,
    pub date: TaxDate,
    /// [R0-N1] ALWAYS `Some` for a pending out (a wallet-less TransferOut never reaches
    /// `pending_reconciliation`); `Option` kept defensively (mirror `BulkLinkRow.source_wallet`).
    pub wallet: Option<WalletId>,
    pub principal_sat: Sat,
    /// [#a] The RESOLVED auto-FMV `fmv_of(prices, date, principal_sat)` — ALWAYS a real number (the
    /// `None` rows are EXCLUDED upstream). This is the ESTIMATED proceeds a `Dispose` recognizes.
    pub fmv: Usd,
    /// Σ leg `usd_basis` carried by the fold's PendingOut consumption (the disposal's basis).
    pub basis_usd: Usd,
    /// `round_cents(fmv − basis_usd)` — the per-row ESTIMATED gain (never double-counted; see above).
    pub estimated_gain: Usd,
}

/// The read-only plan a bulk reclassify-outflow would execute: the eligible/in-frame `included` rows
/// (each with a resolved `fmv`), the count of candidates dropped for a MISSING price, and preview
/// totals. Mirrors `BulkIncomePlan`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkReclassifyOutflowPlan {
    /// Eligible + in-frame + passes `from_wallet` filter + HAS a price. Sorted by `date`.
    pub included: Vec<BulkReclassifyOutflowRow>,
    /// [#a] Candidates dropped because `fmv_of == None` (surfaced, NOT silently dropped — a Sell with
    /// fabricated proceeds would be a SILENT misreport; they stay pending).
    pub excluded_missing_price: usize,
    pub total_sat: Sat,
    /// Σ `fmv` over `included` — the total ESTIMATED proceeds (always a real number).
    pub total_proceeds_usd: Usd,
    /// Σ `basis_usd` over `included`.
    pub total_basis_usd: Usd,
    /// Σ `estimated_gain` over `included` — the total ESTIMATED gain shown in the preview.
    pub total_estimated_gain: Usd,
}

// ── Bulk resolve-conflict plan (bulk-resolve-conflict D1) ────────────────────
//
// The shared, READ-ONLY plan both the CLI (`cmd::reconcile::bulk_resolve_conflict_plan`) and the TUI
// `C` flow compute from the HELD session. Candidate set = live `ImportConflict` blockers only (engine
// post-filtered — an accepted/rejected conflict is no longer flagged → structural idempotence). Each
// row carries the STRUCTURED current/new payloads (NOT pre-rendered strings) so each front-end renders
// its own summary (CLI table formatter; TUI reuses `import_payload_summary`). Appends/persists NOTHING.

/// One flagged import conflict in a bulk resolve-conflict plan. STRUCTURED (front-ends render summaries).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkResolveRow {
    /// The `ImportConflict` event id — the resolution target (`SupersedeImport`/`RejectImport` carry
    /// this as `conflict_event`).
    pub conflict_event: EventId,
    /// Calendar date (tax tz) of the conflict event.
    pub date: TaxDate,
    /// The TARGET import event id whose payload the conflict proposes to supersede (≠ conflict_event).
    pub target: EventId,
    /// Payload currently at the target (front-end renders "current"; KEPT on reject).
    pub current_payload: EventPayload,
    /// `ImportConflict.new_payload` (front-end renders "→ new"; ADOPTED on accept).
    pub new_payload: EventPayload,
    /// The 8-char `new_fingerprint` disambiguator (front-end shows it).
    pub new_fingerprint: String,
}

/// The read-only plan a bulk resolve-conflict would execute: the live `ImportConflict` rows (sorted by
/// date). No $ number (a conflict resolution recognizes no gain); no time/wallet filter (per-row
/// exclude is the precision tool at the front-end).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkResolvePlan {
    pub rows: Vec<BulkResolveRow>,
}

// ── Bulk void plan (bulk-void D1) ────────────────────────────────────────────
//
// The shared, READ-ONLY plan both the CLI (`cmd::reconcile::bulk_void_plan`) and the TUI `V` sweep
// compute from the HELD session. Candidate set = the SINGLE shared predicate `voidable_decisions`
// (btctax-core) over the projected events + blockers — the ONLY defense against voiding an EFFECTIVE
// `SafeHarborAllocation` (#7 → Hard `DecisionConflict`). Each row carries the STRUCTURED decision
// `payload` (front-ends render `summarize_void_payload`) + the precomputed `disposal_to_clear` (a
// `LotSelection` target → `ls.disposal_event`) so the persist path never re-loads the log per row.
// Appends/persists NOTHING; mirrors `bulk_resolve_conflict_plan`.

/// One voidable reconcile decision in a bulk-void plan. STRUCTURED (front-ends render summaries).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkVoidRow {
    /// The Decision event id to void (`VoidDecisionEvent.target_event_id`).
    pub target_event_id: EventId,
    /// `decision|seq` sequence number (display + deterministic sort).
    pub seq: u64,
    /// Calendar date (tax tz) of the decision event.
    pub date: TaxDate,
    /// The decision's payload — front-ends render `summarize_void_payload` (tag + what the void undoes).
    pub payload: EventPayload,
    /// Precomputed side-effect target: a `LotSelection` target → `Some(ls.disposal_event)` (whose
    /// optimizer attestation the void clears); every other decision → `None`.
    pub disposal_to_clear: Option<EventId>,
}

/// The read-only plan a bulk-void would execute: the voidable decisions (shared predicate), sorted by
/// `seq`. No $ number (a void recognizes no gain); no time/wallet filter (per-row exclude is the
/// front-end precision tool).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkVoidPlan {
    pub rows: Vec<BulkVoidRow>,
}

// ── Self-transfer matcher (self-transfer-passthrough C2) ─────────────────────
//
// A READ-ONLY proposal helper (mirrors `bulk_link_transfer_plan`/`safe_harbor_residue`): it appends and
// persists NOTHING. It pairs ONLY UNRECONCILED legs — candidate ins are `TransferIn`s still flagged
// `UnknownBasisInbound` (an already-classified income / self-transfer-in is no longer flagged, so it is
// structurally excluded), candidate outs are `pending_reconciliation` entries (an already-reclassified
// sale is no longer pending). The user CONFIRMS every match; nothing is written by this helper.

/// The confirm action a matched self-transfer pair resolves to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchAction {
    /// Same tracked wallet on both legs (both counterparties external): DROP → `SelfTransferPassthrough`.
    Drop,
    /// Different tracked wallets (out from X, in to Y, X≠Y): RELOCATE → the EXISTING `TransferLink` out→in.
    Relocate,
}

/// One proposed self-transfer match (C2). A PROPOSAL — never applied until the user confirms it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchProposal {
    pub in_event: EventId,
    pub out_event: EventId,
    pub in_date: TaxDate,
    pub out_date: TaxDate,
    /// The in-leg's wallet (the RELOCATE destination). `None` legs are never proposed (can't relocate /
    /// can't be same-wallet), so this is always `Some` for a real proposal; `Option` kept defensively.
    pub in_wallet: Option<WalletId>,
    /// The out-leg's (source) wallet. Always `Some` for a pending out; `Option` kept defensively.
    pub out_wallet: Option<WalletId>,
    pub in_sat: Sat,
    pub out_principal_sat: Sat,
    /// `fmv_of(prices, out_date, out_principal_sat)` — advisory, `None` on missing price / overflow.
    pub usd_value: Option<Usd>,
    /// Suggested action from wallet topology (same-wallet ⇒ Drop, cross-tracked-wallet ⇒ Relocate).
    pub action: MatchAction,
    /// True when this in OR this out matches >1 counterpart — surfaced FLAGGED, NEVER auto-picked (G-FALSE-MATCH).
    pub ambiguous: bool,
    /// True when `in.txid == out.txid` (both `Some`) — decisive cross-wallet corroboration (relaxes the
    /// amount check, but NOT the ambiguity guard).
    pub txid_match: bool,
}

pub struct Session {
    vault: Vault,
    /// The active price provider (§9.2 bundled daily-close dataset, cache-layered in Part C). The
    /// projection and every priced plan read prices through THIS instance seam [R0-C1/r2 I-A] rather
    /// than a hard-wired `BundledPrices::load()`, so tests inject a CONTROLLED synthetic dataset
    /// (`set_prices`) — decoupling their asserted FMVs from the shipped data. Pure/deterministic; the
    /// default is the layered bundled+cache provider (resolved at open time).
    prices: Box<dyn PriceProvider>,
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session").finish_non_exhaustive()
    }
}

/// Build the DEFAULT price provider a freshly-opened `Session` carries: the bundled daily-close dataset
/// (§9.2) with the local price cache layered OVER it (Part C — cache-over-bundled; cache absent ⇒
/// byte-identical to bundled-only). Pure/deterministic; NO network — the cache is a documented LOCAL
/// INPUT populated only by the separate `btctax-update-prices` binary. The cache PATH is resolved HERE
/// (btctax-cli, via `dirs`), NOT in btctax-adapters.
fn default_prices() -> Result<Box<dyn PriceProvider>, CliError> {
    let cache_path = crate::price_cache::default_cache_path();
    Ok(Box::new(btctax_adapters::LayeredPrices::load_with_cache(
        cache_path.as_deref(),
    )?))
}

impl Session {
    /// Create a brand-new encrypted vault, then initialize the core event schema and the CLI config
    /// table, and persist. (`Vault::create` already saved once; we re-save after the DDL.)
    pub fn create(vault_path: &Path, pp: &Passphrase) -> Result<Session, CliError> {
        Self::from_fresh_vault(Vault::create(vault_path, pp)?)
    }

    /// Like `create`, but first clears a half-created vault (orphan key, no pgp/bak) under
    /// explicit `--repair` consent. Delegates to `Vault::repair` which refuses if a real or
    /// recoverable vault is present (see `Vault::repair` safety invariant).
    pub fn repair(vault_path: &Path, pp: &Passphrase) -> Result<Session, CliError> {
        Self::from_fresh_vault(Vault::repair(vault_path, pp)?)
    }

    /// Initialize the core schema + CLI config + tax profile table on a freshly-created vault,
    /// then persist.
    fn from_fresh_vault(mut vault: Vault) -> Result<Session, CliError> {
        init_schema(vault.conn())?;
        config::init_config_table(vault.conn())?;
        tax_profile::init_table(vault.conn())?;
        optimize_attest::init_table(vault.conn())?;
        donation_details::init_table(vault.conn())?;
        bulk_estimated::init_table(vault.conn())?;
        vault.save()?;
        Ok(Session {
            vault,
            prices: default_prices()?,
        })
    }

    /// Open an existing vault (acquires the store single-instance lock; NFR7). A pathless I/O failure
    /// (missing/unreadable `--vault`) is enriched with the path + a one-clause hint (UX-P4-8).
    pub fn open(vault_path: &Path, pp: &Passphrase) -> Result<Session, CliError> {
        let vault = Vault::open(vault_path, pp)
            .map_err(|e| crate::store_io_with_path(e, vault_path, crate::VAULT_OPEN_HINT))?;
        Ok(Session {
            vault,
            prices: default_prices()?,
        })
    }

    /// Borrow the active price provider (§9.2). The projection and every priced plan read prices
    /// through this instance seam [R0-C1].
    pub fn prices(&self) -> &dyn PriceProvider {
        self.prices.as_ref()
    }

    /// Test / advanced seam [R0-C1/r2 I-A]: replace the price provider on an OPEN session. A KAT injects
    /// a CONTROLLED synthetic `BundledPrices::from_csv_str(...)` so its asserted FMVs are independent of
    /// the shipped dataset; a caller may also inject a pre-resolved layered/cache provider. Pure; no I/O.
    pub fn set_prices(&mut self, prices: Box<dyn PriceProvider>) {
        self.prices = prices;
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

    /// Snapshot the in-memory DB image (no disk I/O) for a possible `restore()` after a failed save.
    pub fn snapshot(&self) -> Result<Vec<u8>, CliError> {
        Ok(self.vault.snapshot()?)
    }

    /// Restore the in-memory DB from a prior `snapshot()` (no disk I/O). On `Err`, the in-memory DB
    /// is UNCHANGED and unsaved residue may still be live — the caller MUST latch, never swallow.
    pub fn restore(&mut self, image: &[u8]) -> Result<(), CliError> {
        self.vault.restore(image)?;
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

    /// The stored per-year `TaxProfile` for `year`, or `None` if none has been set.
    /// Robust to older vaults (calls `tax_profile::init_table` as a defensive guard).
    pub fn tax_profile(&self, year: i32) -> Result<Option<TaxProfile>, CliError> {
        tax_profile::get(self.conn(), year)
    }

    /// Resolve + FULLY screen `year`'s profile through the single resolver (SPEC §4.12 / §4.10 / G4) — the
    /// shared entry point every computing consumer (report / optimize / what-if / export) should use so
    /// the app never shows two liabilities, or a wrong number, for one year. `state`/`tables` come from
    /// the caller's projection (`tables` is injectable so `accept` can pass a test table for a later year).
    pub fn resolve_screened(
        &self,
        state: &LedgerState,
        year: i32,
        tables: &dyn TaxTables,
    ) -> Result<crate::resolve::ProfileOutcome, CliError> {
        let pseudo = self.config()?.to_projection().pseudo_reconcile;
        let fr = BundledFullReturnTables::load();
        crate::resolve::resolve_and_screen(
            self.conn(),
            state,
            year,
            pseudo,
            fr.full_return_for(year),
            tables.table_for(year),
        )
    }

    /// [`resolve_screened`] flattened to just the profile: an uncomputable outcome becomes a `Usage` error.
    /// The drop-in replacement for `tax_profile(year)?` at a computing consumer that needs one figure.
    pub fn resolve_screened_profile(
        &self,
        state: &LedgerState,
        year: i32,
        tables: &dyn TaxTables,
    ) -> Result<Option<TaxProfile>, CliError> {
        match self.resolve_screened(state, year, tables)? {
            crate::resolve::ProfileOutcome::Uncomputable { detail } => Err(CliError::Usage(detail)),
            crate::resolve::ProfileOutcome::Ready { profile, .. } => Ok(profile),
        }
    }

    /// Resolve + screen EVERY year that has a stored `TaxProfile` or full-return `ReturnInputs`, for the
    /// read-only viewer (which holds a `Snapshot`, not a live `Session`, so it cannot resolve on demand).
    /// Returns per-year [`crate::resolve::ProfileOutcome`] so the TUI can render a derived number OR a
    /// refusal — never a stale/absent profile (SPEC §4.12: the TUI is a consumer; review P2-C1).
    pub fn resolve_all_screened(
        &self,
        state: &LedgerState,
        tables: &dyn TaxTables,
    ) -> Result<BTreeMap<i32, crate::resolve::ProfileOutcome>, CliError> {
        // Enumerate keys WITHOUT deserializing every blob (N3: one corrupt row must not break enumeration),
        // and hoist the config/full-return-table loads OUT of the per-year loop.
        let pseudo = self.config()?.to_projection().pseudo_reconcile;
        let fr = BundledFullReturnTables::load();
        let mut years: BTreeSet<i32> = tax_profile::years(self.conn())?.into_iter().collect();
        years.extend(return_inputs::years(self.conn())?);
        let mut out = BTreeMap::new();
        for year in years {
            // A corrupt side-table blob for ONE year must surface as a per-year refusal, NOT a failure that
            // bricks the whole read-only viewer (fail-closed availability — review N3).
            let outcome = match crate::resolve::resolve_and_screen(
                self.conn(),
                state,
                year,
                pseudo,
                fr.full_return_for(year),
                tables.table_for(year),
            ) {
                Ok(o) => o,
                Err(e) => crate::resolve::ProfileOutcome::Uncomputable {
                    detail: format!("could not read the stored inputs for {year}: {e}"),
                },
            };
            out.insert(year, outcome);
        }
        Ok(out)
    }

    /// All stored `TaxProfile`s, sorted by year ascending.
    pub fn all_tax_profiles(
        &self,
    ) -> Result<std::collections::BTreeMap<i32, TaxProfile>, CliError> {
        tax_profile::all(self.conn())
    }

    /// All attested disposal `EventId`s (NFR4-stable `BTreeSet`; feeds `compliance_overlay`).
    /// Robust to older vaults (defensive `init_table` guard inside `attested_set`).
    pub fn optimize_attested_set(
        &self,
    ) -> Result<std::collections::BTreeSet<btctax_core::EventId>, CliError> {
        optimize_attest::attested_set(self.conn())
    }

    /// All stored `DonationDetails`, keyed by donation `EventId` (NFR4-stable `BTreeMap`).
    /// Robust to older vaults (defensive `init_table` guard inside `donation_details::all`).
    pub fn donation_details(
        &self,
    ) -> Result<std::collections::BTreeMap<EventId, DonationDetails>, CliError> {
        donation_details::all(self.conn())
    }

    /// All disposals flagged as estimated-FMV proceeds by the bulk-reclassify-outflow path, keyed by
    /// the `transfer_out_event` (== `Disposal.event`); value = the `date_marked` provenance stamp
    /// (NFR4-stable `BTreeMap`). Robust to older vaults (defensive `init_table` guard inside
    /// `bulk_estimated::all`). `build_snapshot` loads the `[est]` marker set via THIS accessor,
    /// NEVER `conn()` directly [R0-M1].
    pub fn bulk_estimated(&self) -> Result<std::collections::BTreeMap<EventId, String>, CliError> {
        bulk_estimated::all(self.conn())
    }

    /// Load all events and run the pure deterministic projection (NFR4) over the bundled daily-close
    /// dataset (§9.2). Returns the resolved `ProjectionConfig` too (so `verify` can display it).
    pub fn project(&self) -> Result<(LedgerState, ProjectionConfig), CliError> {
        let events = load_all(self.conn())?;
        let cfg = self.config()?.to_projection();
        let prices = self.prices();
        let state = project(&events, prices, &cfg);
        Ok((state, cfg))
    }

    /// Single-load variant: loads events ONCE and returns them alongside the projection. Callers
    /// that need both the raw event log and the projected state (e.g. `verify`, `safe_harbor_attest`)
    /// use this to avoid the double `load_all` call that the `project()` + separate `load_all()`
    /// pattern incurs.
    pub fn load_events_and_project(
        &self,
    ) -> Result<(Vec<LedgerEvent>, LedgerState, ProjectionConfig), CliError> {
        let events = load_all(self.conn())?;
        let cfg = self.config()?.to_projection();
        let prices = self.prices();
        let state = project(&events, prices, &cfg);
        Ok((events, state, cfg))
    }

    /// §A.5(a): the distinct Exchange accounts in the vault, each with its currently-in-force
    /// cost-basis method and whether that method is an explicit per-account election (`true`) vs
    /// inherited from a global election / FIFO default (`false`), as of `date`. Feeds the
    /// btctax-tui-edit method-election flow's account list. Uses the SHARED resolver via
    /// `btctax_core::in_force_methods` (the sole precedence path). Sorted by `WalletId: Ord`.
    pub fn exchange_method_election_rows(
        &self,
        date: TaxDate,
    ) -> Result<Vec<(WalletId, LotMethod, bool)>, CliError> {
        let events = load_all(self.conn())?;
        let cfg = self.config()?.to_projection();
        let prices = self.prices();
        // Distinct Exchange wallets (BTreeSet dedups AND sorts — NFR4).
        let wallets: Vec<WalletId> = events
            .iter()
            .filter_map(|e| e.wallet.clone())
            .filter(|w| matches!(w, WalletId::Exchange { .. }))
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        let methods = btctax_core::in_force_methods(&events, prices, &cfg, date, &wallets);
        Ok(wallets
            .into_iter()
            .zip(methods)
            .map(|(w, m)| (w, m.method, m.scoped))
            .collect())
    }

    /// Recompute the Mode-1 optimizer proposal for `year` on the HELD session. READ-ONLY: appends and
    /// persists NOTHING (a clone-fold-discard recompute).
    ///
    /// The TUI editor's optimize-accept opener calls this to obtain a FRESH proposal (NFR4 — never
    /// trusts a stale one) WITHOUT opening a second `Session` (a second open would deadlock on the
    /// held VaultLock, and `cmd::optimize::accept` is forbidden to the editor for the same reason).
    /// Assembles `optimize_year`'s inputs exactly as `cmd::optimize::run`/`accept` do — events + config
    /// from this conn, bundled prices + tables, a FRESH `tax_profile(year)` read (not the cached snap),
    /// the attested set, and `proposal_made = tax_date(now, UtcOffset::UTC)` — and maps `OptimizeError`
    /// through the crate-internal `map_opt_err` (which is `pub(crate)` and not TUI-reachable). `now`
    /// is injected by the caller for determinism.
    pub fn optimize_proposal(
        &self,
        year: i32,
        now: time::OffsetDateTime,
    ) -> Result<btctax_core::OptimizeProposal, CliError> {
        let (events, state, cfg) = self.load_events_and_project()?;
        let prices = self.prices();
        let tables = BundledTaxTables::load();
        let profile = self.resolve_screened_profile(&state, year, &tables)?;
        let attested = self.optimize_attested_set()?;
        let proposal_made = tax_date(now, time::UtcOffset::UTC);
        btctax_core::optimize_year(
            &events,
            prices,
            &cfg,
            year,
            profile.as_ref(),
            &tables,
            &attested,
            proposal_made,
        )
        .map_err(crate::cmd::optimize::map_opt_err)
    }

    /// READ-ONLY: the 2025-01-01 pre-2025 Universal residue as `AllocLot`s, plus the `pre2025_method`
    /// (`LotMethod`) it was computed under. Appends/persists NOTHING. The single source of the pre-2025
    /// subset, shared by `cmd::reconcile::safe_harbor_allocate` and the TUI allocate opener.
    ///
    /// Reads the config ONCE: `cfg.pre2025_method` is the recorded method returned to the caller, and
    /// `cfg.to_projection()` is the projection the residue is computed under — the two are STRUCTURALLY
    /// the same config read, so the returned method can never diverge from the residue's [R0-M1]. The
    /// pre-2025 subset keeps only imports whose tax-date `< 2025-01-01` plus ALL reconciliation decisions
    /// (which shape the residue), and DROPs any prior `SafeHarborAllocation` so the residue stays
    /// allocation-INDEPENDENT (matches `transition::universal_snapshot`).
    pub fn safe_harbor_residue(&self) -> Result<(Vec<AllocLot>, LotMethod), CliError> {
        let cfg = self.config()?;
        let pre2025_method = cfg.pre2025_method; // recorded field == the one used below
        let proj = cfg.to_projection();
        let pre2025: Vec<LedgerEvent> = load_all(self.conn())?
            .into_iter()
            .filter(|e| match &e.id {
                EventId::Import { .. } => {
                    tax_date(e.utc_timestamp, e.original_tz) < TRANSITION_DATE
                }
                _ => !matches!(e.payload, EventPayload::SafeHarborAllocation(_)),
            })
            .collect();
        let prices = self.prices();
        let residue = project(&pre2025, prices, &proj);
        let lots = residue
            .lots
            .iter()
            .filter(|l| l.remaining_sat > 0)
            .map(|l| AllocLot {
                wallet: l.wallet.clone(),
                sat: l.remaining_sat,
                usd_basis: l.usd_basis,
                acquired_at: l.acquired_at,
                dual_loss_basis: l.dual_loss_basis,
                donor_acquired_at: l.donor_acquired_at,
            })
            .collect();
        Ok((lots, pre2025_method))
    }

    /// READ-ONLY: compute the bulk link-transfer plan (bulk-link-transfer D1). Selects over the
    /// PROJECTED `pending_reconciliation` (which already excludes already-decided / already-linked
    /// outs), enriches each with date / source wallet / principal / advisory USD value / carried
    /// basis, applies the frame + `from_wallet` filters, and routes `source == dest` rows to
    /// `skipped_same_wallet`. Appends and persists NOTHING; mirrors `safe_harbor_residue`.
    ///
    /// The USD value is `btctax_core::price::fmv_of(prices, date, principal_sat)` [R0-M1] — the
    /// vetted checked helper (round_cents + overflow→`None`), NOT a hand-rolled `principal × price`.
    /// The total is the HONEST FLOOR [R0-I2]: `total_usd_value_floor` is Σ of the PRICED rows only,
    /// and `missing_price_count` records how many rows lacked a price, so the caller renders exact
    /// `$X` (when 0) or `≥ $X (N unavailable)`.
    pub fn bulk_link_transfer_plan(
        &self,
        filter: BulkFilter,
        dest: WalletId,
    ) -> Result<BulkLinkPlan, CliError> {
        let (events, state, _cfg) = self.load_events_and_project()?;
        let prices = self.prices();
        let index: std::collections::HashMap<EventId, &LedgerEvent> =
            events.iter().map(|e| (e.id.clone(), e)).collect();

        let enrich = |pt: &PendingTransfer| -> BulkLinkRow {
            let ev = index.get(&pt.event).copied();
            let date = ev
                .map(|e| tax_date(e.utc_timestamp, e.original_tz))
                .unwrap_or_else(|| {
                    // Defensive: a pending out always has an indexed source event; fall back to the
                    // epoch date rather than panic (mirrors the single link-transfer opener).
                    tax_date(
                        time::OffsetDateTime::from_unix_timestamp(0).unwrap(),
                        time::UtcOffset::UTC,
                    )
                });
            let source_wallet = ev.and_then(|e| e.wallet.clone());
            let usd_value = btctax_core::price::fmv_of(prices, date, pt.principal_sat);
            let basis_usd: Usd = pt.legs.iter().map(|l| l.usd_basis).sum();
            BulkLinkRow {
                out_event: pt.event.clone(),
                date,
                source_wallet,
                principal_sat: pt.principal_sat,
                usd_value,
                basis_usd,
            }
        };

        let in_frame = |date: TaxDate| match &filter.frame {
            Frame::All => true,
            Frame::Year(y) => date.year() == *y,
            Frame::Range { from, to } => *from <= date && date <= *to,
        };

        let mut included: Vec<BulkLinkRow> = Vec::new();
        let mut skipped_same_wallet: Vec<BulkLinkRow> = Vec::new();
        for pt in &state.pending_reconciliation {
            let row = enrich(pt);
            if !in_frame(row.date) {
                continue;
            }
            if let Some(w) = &filter.from_wallet {
                if row.source_wallet.as_ref() != Some(w) {
                    continue;
                }
            }
            // Same-wallet guard: a self-link to the SAME wallet is meaningless — report, never link.
            if row.source_wallet.as_ref() == Some(&dest) {
                skipped_same_wallet.push(row);
            } else {
                included.push(row);
            }
        }
        included.sort_by_key(|r| r.date);

        let total_sat: Sat = included.iter().map(|r| r.principal_sat).sum();
        let total_usd_value_floor: Usd = included.iter().filter_map(|r| r.usd_value).sum();
        let missing_price_count = included.iter().filter(|r| r.usd_value.is_none()).count();
        let total_basis_usd: Usd = included.iter().map(|r| r.basis_usd).sum();

        Ok(BulkLinkPlan {
            dest,
            included,
            skipped_same_wallet,
            total_sat,
            total_usd_value_floor,
            missing_price_count,
            total_basis_usd,
        })
    }

    /// READ-ONLY: compute the bulk classify-inbound-self-transfer plan
    /// (bulk-classify-inbound-self-transfer D1). A close MIRROR of `bulk_link_transfer_plan` applied to
    /// Cycle A's inbound `SelfTransferMine` ($0 conservative basis, non-taxable). Appends/persists
    /// NOTHING (clone-fold-discard recompute); KAT-G1-clean at the TUI call site.
    ///
    /// **Selection (structural false-classify safety) [R0-I1]:** candidates are `TransferIn` events
    /// still flagged `UnknownBasisInbound` (blocker set joined to the raw event via the index, as
    /// `self_transfer_match_plan` does) **MINUS** any already targeted by a NON-VOIDED `ClassifyInbound`
    /// (mirror `open_classify_inbound_flow`'s filter 3 — appending a second fires a return-blocking Hard
    /// `DecisionConflict`; `UnknownBasisInbound` is RE-EMITTED for gift-basis-unknown states, so
    /// "flagged" ≠ "unclassified") **MINUS** wallet-less inbounds [R0-M2] (create no lot). The USD is
    /// `fmv_of` [G4]; the total is the HONEST FLOOR (`total_usd_fmv_floor` + `missing_price_count`).
    pub fn bulk_self_transfer_in_plan(
        &self,
        filter: BulkStiFilter,
    ) -> Result<BulkStiPlan, CliError> {
        let (events, state, _cfg) = self.load_events_and_project()?;
        let prices = self.prices();
        let index: std::collections::HashMap<EventId, &LedgerEvent> =
            events.iter().map(|e| (e.id.clone(), e)).collect();

        // [R0-I1] filter-3, mirroring `open_classify_inbound_flow`: the set of TransferIn event ids
        // already targeted by a NON-VOIDED `ClassifyInbound`. Build the voided-decision-id set first,
        // then keep only ClassifyInbounds whose OWN id is not voided [R0-M-r2-1: decision-id space and
        // TransferIn-id space are disjoint; we intersect a decision's own id against `voided`, and map
        // the survivor to its `transfer_in_event`].
        let voided: std::collections::BTreeSet<EventId> = events
            .iter()
            .filter_map(|e| match &e.payload {
                EventPayload::VoidDecisionEvent(v) => Some(v.target_event_id.clone()),
                _ => None,
            })
            .collect();
        let already_classified: std::collections::BTreeSet<EventId> = events
            .iter()
            .filter(|e| !voided.contains(&e.id))
            .filter_map(|e| match &e.payload {
                EventPayload::ClassifyInbound(ci) => Some(ci.transfer_in_event.clone()),
                _ => None,
            })
            .collect();

        let in_frame = |date: TaxDate| match &filter.frame {
            Frame::All => true,
            Frame::Year(y) => date.year() == *y,
            Frame::Range { from, to } => *from <= date && date <= *to,
        };

        let mut included: Vec<BulkStiRow> = Vec::new();
        for b in &state.blockers {
            if b.kind != BlockerKind::UnknownBasisInbound {
                continue;
            }
            let Some(id) = &b.event else { continue };
            // [R0-I1] EXCLUDE any inbound that already carries a live ClassifyInbound (else the bulk
            // append duplicates it → Hard DecisionConflict blocks compute_tax_year).
            if already_classified.contains(id) {
                continue;
            }
            let Some(ev) = index.get(id) else { continue };
            let EventPayload::TransferIn(ti) = &ev.payload else {
                continue;
            };
            // [R0-M2] EXCLUDE wallet-less inbounds — a self-transfer-in creates no lot and re-fires
            // the blocker (matcher skips them too).
            let Some(wallet) = ev.wallet.clone() else {
                continue;
            };
            let date = tax_date(ev.utc_timestamp, ev.original_tz);
            if !in_frame(date) {
                continue;
            }
            if let Some(w) = &filter.wallet {
                if &wallet != w {
                    continue;
                }
            }
            let sat = ti.sat;
            let usd_fmv = btctax_core::price::fmv_of(prices, date, sat);
            included.push(BulkStiRow {
                in_event: id.clone(),
                date,
                wallet: Some(wallet),
                sat,
                usd_fmv,
            });
        }
        included.sort_by_key(|r| r.date);

        let total_sat: Sat = included.iter().map(|r| r.sat).sum();
        let total_usd_fmv_floor: Usd = included.iter().filter_map(|r| r.usd_fmv).sum();
        let missing_price_count = included.iter().filter(|r| r.usd_fmv.is_none()).count();

        Ok(BulkStiPlan {
            included,
            total_sat,
            total_usd_fmv_floor,
            missing_price_count,
        })
    }

    /// READ-ONLY: compute the bulk classify-inbound-income plan (bulk-classify-inbound-income, Cycle 4).
    /// A NEAR-CLONE of `bulk_self_transfer_in_plan`: candidates are `TransferIn`s still flagged
    /// `UnknownBasisInbound` MINUS any already targeted by a NON-VOIDED `ClassifyInbound` (filter-3;
    /// a second `ClassifyInbound` fires a Hard `DecisionConflict`) MINUS wallet-less inbounds (create no
    /// lot; also a Hard-`FmvMissing` vector). Then the **Cycle-4 tax-safety difference [#a]**: any
    /// candidate whose `fmv_of(date, sat)` is `None` (missing daily-close price OR overflow) is EXCLUDED
    /// from `included` and counted in `excluded_missing_price` — a persisted `Income{fmv:None}` projects
    /// to a Hard `FmvMissing` year-gate (NOT clearable by `ManualFmv` on the inbound path). `included`
    /// therefore carries a RESOLVED `fmv: Usd` (non-Option). Appends/persists NOTHING.
    pub fn bulk_classify_income_plan(
        &self,
        filter: BulkIncomeFilter,
    ) -> Result<BulkIncomePlan, CliError> {
        let (events, state, _cfg) = self.load_events_and_project()?;
        let prices = self.prices();
        let index: std::collections::HashMap<EventId, &LedgerEvent> =
            events.iter().map(|e| (e.id.clone(), e)).collect();

        // filter-3, mirroring `bulk_self_transfer_in_plan`: the set of TransferIn ids already targeted
        // by a NON-VOIDED `ClassifyInbound` (build the voided-decision-id set first, keep only
        // ClassifyInbounds whose OWN id is not voided, map the survivor to its `transfer_in_event`).
        let voided: std::collections::BTreeSet<EventId> = events
            .iter()
            .filter_map(|e| match &e.payload {
                EventPayload::VoidDecisionEvent(v) => Some(v.target_event_id.clone()),
                _ => None,
            })
            .collect();
        let already_classified: std::collections::BTreeSet<EventId> = events
            .iter()
            .filter(|e| !voided.contains(&e.id))
            .filter_map(|e| match &e.payload {
                EventPayload::ClassifyInbound(ci) => Some(ci.transfer_in_event.clone()),
                _ => None,
            })
            .collect();

        let in_frame = |date: TaxDate| match &filter.frame {
            Frame::All => true,
            Frame::Year(y) => date.year() == *y,
            Frame::Range { from, to } => *from <= date && date <= *to,
        };

        let mut included: Vec<BulkIncomeRow> = Vec::new();
        let mut excluded_missing_price = 0usize;
        for b in &state.blockers {
            if b.kind != BlockerKind::UnknownBasisInbound {
                continue;
            }
            let Some(id) = &b.event else { continue };
            // filter-3: EXCLUDE any inbound already carrying a live ClassifyInbound (a second one →
            // Hard DecisionConflict blocks compute_tax_year).
            if already_classified.contains(id) {
                continue;
            }
            let Some(ev) = index.get(id) else { continue };
            let EventPayload::TransferIn(ti) = &ev.payload else {
                continue;
            };
            // EXCLUDE wallet-less inbounds — an income inbound without a wallet creates no lot and
            // itself raises a Hard FmvMissing (fold.rs); the matcher skips them too.
            let Some(wallet) = ev.wallet.clone() else {
                continue;
            };
            let date = tax_date(ev.utc_timestamp, ev.original_tz);
            if !in_frame(date) {
                continue;
            }
            if let Some(w) = &filter.wallet {
                if &wallet != w {
                    continue;
                }
            }
            let sat = ti.sat;
            // [#a tax-safety — the whole cycle] a candidate with no price is EXCLUDED (counted), NEVER
            // classified as income: an Income{fmv:None} would trade UnknownBasisInbound for a Hard
            // FmvMissing year-gate. bulk-sti INCLUDES these rows ($0-basis needs no FMV); bulk-income
            // must NOT. `included` carries the resolved non-Option fmv.
            match btctax_core::price::fmv_of(prices, date, sat) {
                Some(fmv) => included.push(BulkIncomeRow {
                    in_event: id.clone(),
                    date,
                    sat,
                    fmv,
                }),
                None => excluded_missing_price += 1,
            }
        }
        included.sort_by_key(|r| r.date);

        let total_sat: Sat = included.iter().map(|r| r.sat).sum();
        let total_income_usd: Usd = included.iter().map(|r| r.fmv).sum();

        Ok(BulkIncomePlan {
            included,
            excluded_missing_price,
            total_sat,
            total_income_usd,
        })
    }

    /// READ-ONLY: compute the bulk reclassify-outflow plan (bulk-reclassify-outflow, Cycle 5). Selects
    /// over the PROJECTED `pending_reconciliation` (which already excludes already-reclassified / linked
    /// outs, and never contains wallet-less outflows), enriches each with date / source wallet /
    /// principal / RESOLVED auto-FMV / carried basis / estimated gain, and applies the frame +
    /// `from_wallet` filters. Appends/persists NOTHING (clone-fold-discard recompute); mirrors
    /// `bulk_link_transfer_plan` + the `bulk_classify_income_plan` `#a` missing-price exclusion.
    ///
    /// **[#a tax-safety — the whole cycle]** a candidate with no price is EXCLUDED (counted in
    /// `excluded_missing_price`), NEVER reclassified: `ReclassifyOutflow.principal_proceeds_or_fmv` is
    /// `Usd` (NOT Option), so the row could only be built by FABRICATING a `0`/bogus proceeds — a SILENT
    /// misreport (unlike bulk-income's LOUD `FmvMissing`). `included` therefore carries a RESOLVED
    /// `fmv: Usd` (non-Option), making a fabricated-proceeds Sell structurally unrepresentable here.
    ///
    /// **Estimated gain [Q3]:** `basis_usd = Σ pt.legs.usd_basis` — computed by the fold's SINGLE
    /// chronological pass with ALL candidate PendingOuts pending, so `Σ` over multiple rows is NEVER
    /// double-counted (an earlier-dated out drew the pool down before a later one folded). `estimated_gain
    /// = round_cents(fmv − basis_usd)` per row. The persisted Form-8949 numbers always run the ordinary
    /// fold (exact); only this preview carries the FIFO-vs-method / fee-treatment residual (label it).
    pub fn bulk_reclassify_outflow_plan(
        &self,
        filter: BulkFilter,
    ) -> Result<BulkReclassifyOutflowPlan, CliError> {
        let (events, state, _cfg) = self.load_events_and_project()?;
        let prices = self.prices();
        let index: std::collections::HashMap<EventId, &LedgerEvent> =
            events.iter().map(|e| (e.id.clone(), e)).collect();

        let in_frame = |date: TaxDate| match &filter.frame {
            Frame::All => true,
            Frame::Year(y) => date.year() == *y,
            Frame::Range { from, to } => *from <= date && date <= *to,
        };

        let mut included: Vec<BulkReclassifyOutflowRow> = Vec::new();
        let mut excluded_missing_price = 0usize;
        for pt in &state.pending_reconciliation {
            let ev = index.get(&pt.event).copied();
            let date = ev
                .map(|e| tax_date(e.utc_timestamp, e.original_tz))
                .unwrap_or_else(|| {
                    // Defensive: a pending out always has an indexed source event; fall back to the
                    // epoch date rather than panic (mirrors `bulk_link_transfer_plan`).
                    tax_date(
                        time::OffsetDateTime::from_unix_timestamp(0).unwrap(),
                        time::UtcOffset::UTC,
                    )
                });
            if !in_frame(date) {
                continue;
            }
            let wallet = ev.and_then(|e| e.wallet.clone());
            if let Some(w) = &filter.from_wallet {
                if wallet.as_ref() != Some(w) {
                    continue;
                }
            }
            // [#a] EXCLUDE (count) a missing-price row — a Sell with fabricated proceeds is a SILENT
            // misreport. `included` carries the resolved non-Option fmv.
            let fmv = match btctax_core::price::fmv_of(prices, date, pt.principal_sat) {
                Some(v) => v,
                None => {
                    excluded_missing_price += 1;
                    continue;
                }
            };
            let basis_usd: Usd = pt.legs.iter().map(|l| l.usd_basis).sum();
            let estimated_gain = round_cents(fmv - basis_usd);
            included.push(BulkReclassifyOutflowRow {
                out_event: pt.event.clone(),
                date,
                wallet,
                principal_sat: pt.principal_sat,
                fmv,
                basis_usd,
                estimated_gain,
            });
        }
        included.sort_by_key(|r| r.date);

        let total_sat: Sat = included.iter().map(|r| r.principal_sat).sum();
        let total_proceeds_usd: Usd = included.iter().map(|r| r.fmv).sum();
        let total_basis_usd: Usd = included.iter().map(|r| r.basis_usd).sum();
        let total_estimated_gain: Usd = included.iter().map(|r| r.estimated_gain).sum();

        Ok(BulkReclassifyOutflowPlan {
            included,
            excluded_missing_price,
            total_sat,
            total_proceeds_usd,
            total_basis_usd,
            total_estimated_gain,
        })
    }

    /// READ-ONLY: compute the bulk resolve-conflict plan (bulk-resolve-conflict D1). Candidate set =
    /// live `ImportConflict` blockers only (engine post-filtered — an accepted/rejected conflict is no
    /// longer flagged, so re-running never double-resolves; structural idempotence). Joins each blocker
    /// (whose `.event` is the `ImportConflict` event id) to the event index to build a STRUCTURED row:
    /// the conflict's `target` + `new_payload` + `new_fingerprint`, plus the `current_payload` read from
    /// the TARGET event (a SEPARATE event — accept adopts `new_payload`, reject keeps `current_payload`).
    /// Appends/persists NOTHING; mirrors `bulk_self_transfer_in_plan`.
    pub fn bulk_resolve_conflict_plan(&self) -> Result<BulkResolvePlan, CliError> {
        let (events, state, _cfg) = self.load_events_and_project()?;
        let index: std::collections::HashMap<EventId, &LedgerEvent> =
            events.iter().map(|e| (e.id.clone(), e)).collect();

        let mut rows: Vec<BulkResolveRow> = Vec::new();
        for b in &state.blockers {
            if b.kind != BlockerKind::ImportConflict {
                continue;
            }
            let Some(conflict_id) = &b.event else {
                continue;
            };
            let Some(conflict_ev) = index.get(conflict_id) else {
                continue;
            };
            let EventPayload::ImportConflict(c) = &conflict_ev.payload else {
                continue;
            };
            // CURRENT payload lives at the TARGET id (a separate event, conflict_event != target).
            let Some(target_ev) = index.get(&c.target) else {
                continue;
            };
            let date = tax_date(conflict_ev.utc_timestamp, conflict_ev.original_tz);
            rows.push(BulkResolveRow {
                conflict_event: conflict_id.clone(),
                date,
                target: c.target.clone(),
                current_payload: target_ev.payload.clone(),
                new_payload: (*c.new_payload).clone(),
                new_fingerprint: c.new_fingerprint.0.chars().take(8).collect::<String>(),
            });
        }
        rows.sort_by_key(|r| r.date);
        Ok(BulkResolvePlan { rows })
    }

    /// READ-ONLY: compute the bulk-void plan (bulk-void D1). Candidate set = the SINGLE shared
    /// predicate `btctax_core::voidable_decisions` over the projected events + blockers (Decision-id ∧
    /// not-voided ∧ `is_revocable_payload` ∧ #7 `!effective_alloc`) — the ONLY defense against sweeping
    /// an EFFECTIVE `SafeHarborAllocation` into a Hard `DecisionConflict`. Each row precomputes its
    /// `disposal_to_clear` ONCE from the same event set (a `LotSelection` target → `ls.disposal_event`)
    /// so `apply_bulk_void`/`persist_bulk_void` never re-load the log per row. Appends/persists NOTHING;
    /// mirrors `bulk_resolve_conflict_plan`. Sorted by `seq` for deterministic display.
    pub fn bulk_void_plan(&self) -> Result<BulkVoidPlan, CliError> {
        let (events, state, _cfg) = self.load_events_and_project()?;
        let mut rows: Vec<BulkVoidRow> = btctax_core::voidable_decisions(&events, &state.blockers)
            .into_iter()
            .map(|e| {
                let seq = match &e.id {
                    EventId::Decision { seq } => *seq,
                    _ => 0,
                };
                let disposal_to_clear = match &e.payload {
                    EventPayload::LotSelection(ls) => Some(ls.disposal_event.clone()),
                    _ => None,
                };
                BulkVoidRow {
                    target_event_id: e.id.clone(),
                    seq,
                    date: tax_date(e.utc_timestamp, e.original_tz),
                    payload: e.payload.clone(),
                    disposal_to_clear,
                }
            })
            .collect();
        rows.sort_by_key(|r| r.seq);
        Ok(BulkVoidPlan { rows })
    }

    /// READ-ONLY: propose self-transfer matches (self-transfer-passthrough C2). Appends/persists NOTHING
    /// (a clone-fold-discard recompute, like `bulk_link_transfer_plan`). Pairs ONLY unreconciled legs —
    /// candidate ins are `TransferIn`s flagged `UnknownBasisInbound` [R0-M2] (enumerated from the blocker
    /// set + joined to the raw event via the event index), candidate outs are `pending_reconciliation`.
    ///
    /// A pair is proposed iff ALL criteria pass: amount within `tol = max(out.fee_sat, ceil(0.005 ×
    /// out.principal))` (a `txid` EXACT match relaxes the amount check), a ±2-day window consistent with
    /// the direction (passthrough: in on/before out; relocate: in on/after out), and BOTH wallets present.
    /// The suggested `action` is wallet topology (same-wallet ⇒ Drop, cross-tracked-wallet ⇒ Relocate).
    /// A leg matching >1 counterpart is flagged `ambiguous` (surfaced, NEVER auto-picked — G-FALSE-MATCH).
    pub fn self_transfer_match_plan(&self) -> Result<Vec<MatchProposal>, CliError> {
        let (events, state, _cfg) = self.load_events_and_project()?;
        let prices = self.prices();
        let index: std::collections::HashMap<EventId, &LedgerEvent> =
            events.iter().map(|e| (e.id.clone(), e)).collect();

        // Candidate ins: TransferIn events still flagged UnknownBasisInbound (unreconciled), joined to the
        // raw event via the index (the exact pattern `bulk_link_transfer_plan` uses for pending outs).
        struct CandIn {
            id: EventId,
            sat: Sat,
            txid: Option<String>,
            date: TaxDate,
            wallet: Option<WalletId>,
        }
        let mut ins: Vec<CandIn> = Vec::new();
        for b in &state.blockers {
            if b.kind != BlockerKind::UnknownBasisInbound {
                continue;
            }
            let Some(id) = &b.event else { continue };
            let Some(ev) = index.get(id) else { continue };
            let EventPayload::TransferIn(ti) = &ev.payload else {
                continue;
            };
            ins.push(CandIn {
                id: id.clone(),
                sat: ti.sat,
                txid: ti.txid.clone(),
                date: tax_date(ev.utc_timestamp, ev.original_tz),
                wallet: ev.wallet.clone(),
            });
        }

        // Candidate outs: pending_reconciliation entries (already Op::PendingOut — unreconciled).
        struct CandOut {
            id: EventId,
            principal: Sat,
            fee: Option<Sat>,
            txid: Option<String>,
            date: TaxDate,
            wallet: Option<WalletId>,
        }
        let mut outs: Vec<CandOut> = Vec::new();
        for pt in &state.pending_reconciliation {
            let ev = index.get(&pt.event).copied();
            let date = ev
                .map(|e| tax_date(e.utc_timestamp, e.original_tz))
                .unwrap_or_else(|| {
                    tax_date(
                        time::OffsetDateTime::from_unix_timestamp(0).unwrap(),
                        time::UtcOffset::UTC,
                    )
                });
            let txid = ev.and_then(|e| match &e.payload {
                EventPayload::TransferOut(t) => t.txid.clone(),
                _ => None,
            });
            outs.push(CandOut {
                id: pt.event.clone(),
                principal: pt.principal_sat,
                fee: pt.fee_sat,
                txid,
                date,
                wallet: ev.and_then(|e| e.wallet.clone()),
            });
        }

        // Passing (in_idx, out_idx, action, txid_match) tuples.
        let mut passing: Vec<(usize, usize, MatchAction, bool)> = Vec::new();
        for (i, ci) in ins.iter().enumerate() {
            // A wallet-less in can be neither a same-wallet DROP nor a RELOCATE destination.
            let Some(in_wallet) = ci.wallet.as_ref() else {
                continue;
            };
            for (j, co) in outs.iter().enumerate() {
                let Some(out_wallet) = co.wallet.as_ref() else {
                    continue;
                };
                let action = if in_wallet == out_wallet {
                    MatchAction::Drop
                } else {
                    MatchAction::Relocate
                };
                // Amount: tol = max(fee, ceil(0.005 × principal)). ceil(p/200) = (p + 199) / 200 (p ≥ 0).
                let slack = if co.principal > 0 {
                    (co.principal + 199) / 200
                } else {
                    0
                };
                let tol = co.fee.unwrap_or(0).max(slack);
                let txid_match = ci.txid.is_some() && ci.txid == co.txid;
                let amount_ok = txid_match || (ci.sat - co.principal).abs() <= tol;
                if !amount_ok {
                    continue;
                }
                // ±2-day window, direction keyed to the topology (exchange timestamp drift tolerated).
                let window_ok = match action {
                    // Passthrough: the deposit precedes the withdrawal (in on/before out).
                    MatchAction::Drop => {
                        let d = (co.date - ci.date).whole_days();
                        (0..=2).contains(&d)
                    }
                    // Relocate: the withdrawal precedes the arrival (in on/after out).
                    MatchAction::Relocate => {
                        let d = (ci.date - co.date).whole_days();
                        (0..=2).contains(&d)
                    }
                };
                if !window_ok {
                    continue;
                }
                passing.push((i, j, action, txid_match));
            }
        }

        // Ambiguity: a leg (in OR out) appearing in >1 passing pair is flagged, never silently picked.
        let mut in_count: std::collections::HashMap<usize, usize> =
            std::collections::HashMap::new();
        let mut out_count: std::collections::HashMap<usize, usize> =
            std::collections::HashMap::new();
        for (i, j, _, _) in &passing {
            *in_count.entry(*i).or_insert(0) += 1;
            *out_count.entry(*j).or_insert(0) += 1;
        }

        let mut proposals: Vec<MatchProposal> = passing
            .iter()
            .map(|(i, j, action, txid_match)| {
                let ci = &ins[*i];
                let co = &outs[*j];
                let ambiguous = in_count[i] > 1 || out_count[j] > 1;
                MatchProposal {
                    in_event: ci.id.clone(),
                    out_event: co.id.clone(),
                    in_date: ci.date,
                    out_date: co.date,
                    in_wallet: ci.wallet.clone(),
                    out_wallet: co.wallet.clone(),
                    in_sat: ci.sat,
                    out_principal_sat: co.principal,
                    usd_value: btctax_core::price::fmv_of(prices, co.date, co.principal),
                    action: *action,
                    ambiguous,
                    txid_match: *txid_match,
                }
            })
            .collect();
        // Deterministic order (NFR4): by out date, then the two ids.
        proposals.sort_by(|a, b| {
            a.out_date
                .cmp(&b.out_date)
                .then(a.out_event.cmp(&b.out_event))
                .then(a.in_event.cmp(&b.in_event))
        });
        Ok(proposals)
    }
}

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

    /// `Session::snapshot`/`restore` delegate to the vault and revert an in-memory mutation
    /// without touching disk (the wrapper the persist layer uses for save-rollback).
    #[test]
    fn session_snapshot_restore_reverts_in_memory_mutation() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let mut s = Session::create(&vault, &pp()).unwrap();
        s.conn().execute("CREATE TABLE t (x INTEGER)", []).unwrap();
        s.save().unwrap();

        let snap = s.snapshot().unwrap();
        s.conn().execute("INSERT INTO t VALUES (7)", []).unwrap();
        let n: i64 = s
            .conn()
            .query_row("SELECT COUNT(*) FROM t", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1, "pre-restore: the inserted row is present in memory");

        let before = std::fs::read(&vault).unwrap();
        s.restore(&snap).unwrap();
        let after = std::fs::read(&vault).unwrap();
        assert_eq!(before, after, "restore must not write the vault file");

        let n: i64 = s
            .conn()
            .query_row("SELECT COUNT(*) FROM t", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 0, "restore must revert the in-memory insert");
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

    /// `load_events_and_project` must return the same (events, state, config) triple as calling
    /// `load_all` + `project` separately. Verifies the single-load contract (no double DB round-trip).
    #[test]
    fn load_events_and_project_matches_separate_calls() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        Session::create(&vault, &pp()).unwrap();
        let s = Session::open(&vault, &pp()).unwrap();

        let (events, state, cfg) = s.load_events_and_project().unwrap();

        // Reference path: separate load_all + project calls.
        let events2 = btctax_core::persistence::load_all(s.conn()).unwrap();
        let (state2, cfg2) = s.project().unwrap();

        assert_eq!(events.len(), events2.len(), "event count must match");
        assert_eq!(state.lots.len(), state2.lots.len(), "lots count must match");
        assert_eq!(
            state.blockers.len(),
            state2.blockers.len(),
            "blocker count must match"
        );
        assert_eq!(cfg, cfg2, "ProjectionConfig must match");
    }

    /// `Session::optimize_proposal` recomputes the Mode-1 proposal on the HELD session (READ-ONLY,
    /// no second open). For a 2025 year with two same-wallet lots and a 500k sale, the FIFO baseline
    /// consumes the cheaper lot A (higher gain); the optimizer prefers the dearer lot B → a
    /// per-disposal row whose proposed_selection differs from current_selection, with `delta ≤ 0`.
    #[test]
    fn optimize_proposal_recomputes_a_persistable_proposal_on_held_session() {
        use btctax_core::event::{
            Acquire, BasisSource, DisposeKind, EventPayload, LedgerEvent, MethodElection,
            OutflowClass, ReclassifyOutflow, TransferOut,
        };
        use btctax_core::identity::{Source, SourceRef};
        use btctax_core::persistence::{append_decision, append_import_batch};
        use btctax_core::{Carryforward, EventId, FilingStatus, LotMethod, TaxProfile, WalletId};
        use rust_decimal_macros::dec;
        use time::{OffsetDateTime, UtcOffset};

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        Session::create(&vault, &pp()).unwrap();
        let mut s = Session::open(&vault, &pp()).unwrap();

        let wallet = Some(WalletId::Exchange {
            provider: "River".to_string(),
            account: "main".to_string(),
        });
        let lot_a = EventId::import(Source::River, SourceRef::new("op-lot-a"));
        let lot_b = EventId::import(Source::River, SourceRef::new("op-lot-b"));
        let to_id = EventId::import(Source::River, SourceRef::new("op-sell"));
        let ta = OffsetDateTime::from_unix_timestamp(1_739_000_000).unwrap();
        let tb = OffsetDateTime::from_unix_timestamp(1_741_000_000).unwrap();
        let tc = OffsetDateTime::from_unix_timestamp(1_748_000_000).unwrap();
        let td = OffsetDateTime::from_unix_timestamp(1_748_100_000).unwrap();
        let batch = vec![
            LedgerEvent {
                id: lot_a.clone(),
                utc_timestamp: ta,
                original_tz: UtcOffset::UTC,
                wallet: wallet.clone(),
                payload: EventPayload::Acquire(Acquire {
                    sat: 1_000_000,
                    usd_cost: dec!(30000),
                    fee_usd: dec!(0),
                    basis_source: BasisSource::ExchangeProvided,
                }),
            },
            LedgerEvent {
                id: lot_b.clone(),
                utc_timestamp: tb,
                original_tz: UtcOffset::UTC,
                wallet: wallet.clone(),
                payload: EventPayload::Acquire(Acquire {
                    sat: 1_000_000,
                    usd_cost: dec!(50000),
                    fee_usd: dec!(0),
                    basis_source: BasisSource::ExchangeProvided,
                }),
            },
            LedgerEvent {
                id: to_id.clone(),
                utc_timestamp: tc,
                original_tz: UtcOffset::UTC,
                wallet: wallet.clone(),
                payload: EventPayload::TransferOut(TransferOut {
                    sat: 500_000,
                    fee_sat: None,
                    dest_addr: None,
                    txid: None,
                }),
            },
        ];
        append_import_batch(s.conn(), &batch).unwrap();
        let ro = EventPayload::ReclassifyOutflow(ReclassifyOutflow {
            transfer_out_event: to_id.clone(),
            as_: OutflowClass::Dispose {
                kind: DisposeKind::Sell,
            },
            principal_proceeds_or_fmv: dec!(30000),
            fee_usd: None,
            donee: None,
        });
        append_decision(s.conn(), ro, td, UtcOffset::UTC, None).unwrap();
        // [reconcile-defaults] pin a global FIFO standing order so the BASELINE picks the older lot_a
        // (the app default is now HIFO, which would already pick the dearer lot_b → nothing to propose).
        // Made-date 2024-12-24 ≤ effective 2025-01-01 → not backdated.
        append_decision(
            s.conn(),
            EventPayload::MethodElection(MethodElection {
                effective_from: time::macros::date!(2025 - 01 - 01),
                method: LotMethod::Fifo,
                wallet: None,
            }),
            OffsetDateTime::from_unix_timestamp(1_735_000_000).unwrap(),
            UtcOffset::UTC,
            None,
        )
        .unwrap();
        let profile = TaxProfile {
            filing_status: FilingStatus::Single,
            ordinary_taxable_income: dec!(100000),
            magi_excluding_crypto: dec!(100000),
            qualified_dividends_and_other_pref_income: dec!(0),
            other_net_capital_gain: dec!(0),
            capital_loss_carryforward_in: Carryforward::default(),
            w2_ss_wages: dec!(0),
            w2_medicare_wages: dec!(0),
            schedule_c_expenses: dec!(0),
        };
        crate::tax_profile::set(s.conn(), 2025, &profile).unwrap();
        s.save().unwrap();

        let now = OffsetDateTime::from_unix_timestamp(1_752_000_000).unwrap();
        let proposal = s.optimize_proposal(2025, now).unwrap();
        assert!(
            proposal.delta <= dec!(0),
            "delta must be ≤ 0 (baseline-seeded)"
        );
        let row = proposal
            .per_disposal
            .iter()
            .find(|d| d.disposal == to_id)
            .expect("the 2025 sale must be in the proposal");
        assert_ne!(
            row.proposed_selection, row.current_selection,
            "the optimizer must propose the dearer lot (a change from FIFO)"
        );
    }
}
