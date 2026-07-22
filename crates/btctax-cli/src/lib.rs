//! btctax-cli: the CLI + reconciliation library that wires the encrypted vault (btctax-store),
//! ingest (btctax-adapters), and the pure projection (btctax-core) into the Phase-1 command surface
//! (spec §11). The library is I/O-explicit and deterministic; the binary (`main.rs`) is a thin clap
//! dispatch. PRIVACY: tests use only temp vaults + synthetic fixtures; no real user file is ever read.
pub mod bulk_estimated;
pub mod chokepoint;
pub mod cli;
pub mod cmd;
pub mod config;
pub mod donation_details;
pub mod eventref;
pub mod input_form_store;
pub mod optimize_attest;
pub mod price_cache;
pub mod render;
pub mod resolve;
pub mod return_inputs;
pub mod session;
pub mod tax_profile;
pub mod testonly;

pub use cli::Cli;
// Re-exported at the crate root so the TUI editor (`btctax-tui-edit`) can call it WITHOUT the `cmd::`
// token its KAT-G1 source gate forbids in non-test code. That gate exists to keep session-lifecycle /
// lock-holding `cmd::` fns out of the held-session editor; `guard_allocation_vs_tranche` is a PURE
// `&[LedgerEvent] -> Result` predicate — no `Session`, no lock, no I/O — so the gate's intent is honored,
// not evaded. Any FUTURE addition here must be equally pure (do NOT re-export a session-opening fn).
pub use cmd::tranche::guard_allocation_vs_tranche;
// Re-exported at the crate root mirroring `ATTEST_PHRASE` (below): a plain, distinct consent-phrase
// constant, not a `cmd::`-scoped session/lock fn, so it belongs beside the other top-level phrase gates.
pub use cmd::promote::PROMOTE_ACK_PHRASE;
// Re-exported at the crate root so the TUI export path (`btctax-tui::export::do_export`) can call the
// BG-D8 completeness gate WITHOUT the `cmd::` token its KAT-E10 source gate forbids in non-test code
// (Approach-B Task 17). Like `guard_allocation_vs_tranche` above, this is a PURE
// `(&LedgerState, &[LedgerEvent], Option<i32>) -> Result` predicate — no `Session`, no lock, no I/O — so
// the gate's intent (keep session-lifecycle `cmd::` fns out of the held-session viewer) is honored, not
// evaded. Any FUTURE addition here must be equally pure (do NOT re-export a session-opening fn).
pub use cmd::admin::promote_export_gate;
// Re-exported at the crate root (Defensive Filing Wizard Task 3, ★ arch-n-1) so a future TUI export
// surface (`btctax-tui-edit`'s `persist.rs`, Task 10) can name `IrsPdfReport` WITHOUT the `cmd::` token
// its KAT-G1 source gate forbids in non-test code (mirrors `promote_export_gate` above). `IrsPdfReport`
// is a plain data struct (no `Session`, no lock, no I/O) — the gate's intent is honored, not evaded.
pub use cmd::admin::IrsPdfReport;
pub use config::CliConfig;
pub use session::{
    BulkFilter, BulkIncomeFilter, BulkIncomePlan, BulkIncomeRow, BulkLinkPlan, BulkLinkRow,
    BulkReclassifyOutflowPlan, BulkReclassifyOutflowRow, BulkResolvePlan, BulkResolveRow,
    BulkStiFilter, BulkStiPlan, BulkStiRow, BulkVoidPlan, BulkVoidRow, Frame, MatchAction,
    MatchProposal, Session,
};

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
    /// `export-irs-pdf`: an official IRS PDF fill failed — most importantly the geometric read-back
    /// FAILING CLOSED on a mis-mapped cell, so no wrong tax form is ever written.
    #[error("IRS form fill: {0}")]
    FormFill(#[from] btctax_forms::FormsError),
    /// A user-supplied event reference did not parse as a canonical `EventId` (eventref.rs).
    #[error("not a valid event reference: {0:?}")]
    BadEventRef(String),
    /// A CLI argument was malformed (bad USD/date/enum/wallet spec, or a contradictory flag set).
    #[error("usage: {0}")]
    Usage(String),
    /// M1: a `cli_config` row held an unrecognized value (corrupt DB, future-written value, or manual
    /// edit gone wrong). Returning an error is safer than silently misreading the stored intent.
    #[error("unrecognized stored config value: key={key:?} value={value:?}")]
    BadConfigValue { key: String, value: String },
    /// P9 §2.6: a stored `return_inputs` row predates the form-question registry (or was written by a newer
    /// build) and this build does not migrate it. There is no user data yet, so the policy is refuse-and-
    /// reimport rather than a per-key migration (a version check cannot forget a key). The remedy names all
    /// THREE commands, in order — `clear` DISCARDS any computed carryover this row's prior reports wrote onto
    /// it, so the rebuild step is not optional. (Retire this the moment real data exists — FOLLOWUPS, release
    /// gate.)
    #[error(
        "the stored inputs for {year} predate the form-question registry (schema v{found}; this build reads \
         v{expected}). Run `btctax income clear {year}` — which DISCARDS any carryover this row's prior \
         reports computed onto it — then `btctax income import` for {year}; then, if this row carried a \
         computed carryover, `btctax report --tax-year {prior} --write-carryover` to rebuild it.",
        prior = year - 1
    )]
    StaleReturnInputs {
        year: i32,
        found: i64,
        expected: i64,
    },
    /// §6.3 / C-1: a PARKED input-form draft is at a schema version this build does not read, and this
    /// build does not migrate it. Unlike a stale WIP draft (regenerable → discarded), a parked draft may
    /// hold irreplaceable carryover that exists ONLY in the draft — there is no committed row to re-import
    /// from — so we REFUSE (fail closed) rather than discard. The remedy therefore is NOT `income import`
    /// (that recovers a WIP row from committed state, which a parked draft has none of): the message must
    /// tell the filer the data lives in the draft, must not be discarded, and to re-run on / export from the
    /// app version that wrote it. (Retire alongside `StaleReturnInputs` the moment migrations exist.)
    #[error(
        "year {year}'s parked full return is schema v{found} but this build expects v{expected}; \
         an upgrade changed the input format. Its data lives only in the draft — do not discard it. \
         Re-run on the app version that wrote it, or export it there first."
    )]
    StaleParkedDraft {
        year: i32,
        found: i64,
        expected: i64,
    },
    /// §6.2 draft-coherence: an authoritative committed-row write (`income import` / `income answer` /
    /// carryover write-back / `income clear`) was attempted for a year whose input-form draft is PARKED.
    /// A parked draft is the SOLE copy of a screened return (C-1) — clobbering it via the committed row
    /// would silently destroy irreplaceable data — so the write is REFUSED (fail closed). The message
    /// names BOTH in-form exits (M-d): re-commit it (`use full return`) or drop it (`discard parked
    /// draft`, a confirmed delete); a WIP draft, by contrast, is regenerable and is cleared silently.
    #[error(
        "year {year} holds a parked full return — in the form, 'use full return' to re-commit it, or \
         'discard parked draft' (a confirmed delete) to drop it; then re-run this command."
    )]
    ParkedDraftBlocksWrite { year: i32 },
    /// Sub-project 3 attestation gate: an export was attempted while the ledger is pseudo-reconciled
    /// (a synthetic, non-persisted default contributes to the projection) and NO attestation phrase was
    /// supplied. Producing a form/data file from a fictional draft requires typing the exact phrase.
    /// (Supersedes sub-2's interim [I3] blanket refusal.)
    #[error(
        "export refused: the ledger is pseudo-reconciled (a synthetic default contributes to the \
         projection). To export this draft ON PURPOSE, attest the exact phrase {:?} (pass --attest, or \
         type it at the prompt). Otherwise run `reconcile pseudo off` (or approve + attest the defaults).",
        ATTEST_PHRASE
    )]
    AttestationRequired,
    /// Sub-project 3 attestation gate: an export was attempted while the ledger is pseudo-reconciled and
    /// the supplied attestation phrase did NOT match (trimmed, case-sensitive, exact). A wrong phrase is
    /// FAILED regardless of environment [R0-I1] — no fictional form leaves the machine.
    #[error(
        "export refused: the attestation phrase did not match. The ledger is pseudo-reconciled; type the \
         phrase EXACTLY (trimmed, case-sensitive): {:?}.",
        ATTEST_PHRASE
    )]
    AttestationFailed,
    /// UX-P4-8: an I/O failure at a user-named path — a `--vault` that is missing/unreadable, or an
    /// `--out` that collides / cannot be created. Carries the offending PATH and a one-clause remedy
    /// hint that the bare `io::Error` (surfaced pathlessly through `Store::Io` / `Io`) lacks. Mirrors
    /// the adapters' `AdapterError::Io { path, source }` so every path-bearing io error reads alike.
    #[error("io {path}: {source} ({hint})")]
    PathIo {
        path: String,
        hint: String,
        #[source]
        source: std::io::Error,
    },
}

/// UX-P4-8 hint: shown when a `--vault` cannot be opened (missing/unreadable path).
pub const VAULT_OPEN_HINT: &str =
    "check the --vault path, or run `btctax init` to create a new vault";

/// UX-P4-8 hint: shown when an export `--out` directory cannot be created (a colliding file, a
/// missing parent, or a permission problem).
pub const EXPORT_OUT_HINT: &str =
    "choose an --out path that does not already exist as a file and whose parent is writable";

/// Re-wrap a `StoreError` I/O failure with the offending PATH + a one-clause hint (UX-P4-8). ONLY the
/// pathless `StoreError::Io` is enriched; every other variant (`WrongPassphrase`, `Locked`,
/// `HalfCreatedVault`, …) passes through unchanged — each already carries its own precise meaning and
/// must NOT be masked behind a generic path/hint.
pub fn store_io_with_path(
    e: btctax_store::StoreError,
    path: &std::path::Path,
    hint: &str,
) -> CliError {
    match e {
        btctax_store::StoreError::Io(source) => CliError::PathIo {
            path: path.display().to_string(),
            hint: hint.to_string(),
            source,
        },
        other => CliError::Store(other),
    }
}

/// Re-wrap a pathless I/O failure with the offending PATH + a one-clause hint (UX-P4-8). Enriches
/// BOTH shapes an export write can produce: a raw `CliError::Io` (a `write`/`flush` mid-write) AND a
/// `CliError::Store(StoreError::Io)` (a `mkdir_owner_only`/`open_owner_only` under `out_dir` — e.g. a
/// SUBPATH collision like `out_dir/lots.csv` already existing as a directory, which `?`-converts
/// through `From<StoreError>`). A `CliError::Csv` (a serialization error, not a path problem) and
/// every other variant pass through unchanged.
pub fn cli_io_with_path(e: CliError, path: &std::path::Path, hint: &str) -> CliError {
    match e {
        CliError::Io(source) | CliError::Store(btctax_store::StoreError::Io(source)) => {
            CliError::PathIo {
                path: path.display().to_string(),
                hint: hint.to_string(),
                source,
            }
        }
        other => other,
    }
}

/// The exact phrase a user must affirm to export a form/data file while the ledger is pseudo-reconciled
/// (sub-project 3). Compared TRIMMED, case-SENSITIVE, exact. The prompt + both error strings are BUILT
/// from this constant [R0-M1] so there is no drift (a KAT asserts they contain it). `pub` so btctax-tui
/// shares it [R0-r2-N2].
pub const ATTEST_PHRASE: &str = "I attest this is true";

/// PURE exact-compare attestation gate — NO I/O, NO TTY read [R0-I2]. The interactive prompt lives in
/// the caller (the `export-snapshot` main.rs arm / the btctax-tui export modal); this helper only
/// compares, keeping the library I/O-explicit and the KATs deterministic (no env-dependent branch).
///
/// - `attest.map(str::trim) == Some(ATTEST_PHRASE)` → `Ok(())`.
/// - `Some(_)` non-matching → `Err(AttestationFailed)` (a wrong phrase FAILS regardless of env) [R0-I1].
/// - `None` → `Err(AttestationRequired)`.
///
/// `pub` so btctax-tui shares the exact-compare [R0-r2-N2].
pub fn require_attestation(attest: Option<&str>) -> Result<(), CliError> {
    match attest.map(str::trim) {
        Some(p) if p == ATTEST_PHRASE => Ok(()),
        Some(_) => Err(CliError::AttestationFailed),
        None => Err(CliError::AttestationRequired),
    }
}
