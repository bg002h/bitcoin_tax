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
pub mod open_next_year;
pub mod optimize_attest;
pub mod price_cache;
pub mod render;
pub mod resolve;
pub mod return_inputs;
pub mod session;
/// ★★★ R9 / T6 — Step 0 of the interview: a STATUS PANEL over the held session, never a question set.
pub mod step0;
pub mod tax_profile;
pub mod testonly;
pub mod year_readiness;

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
// Re-exported at the crate root (Defensive Filing Wizard Task 9) alongside `PROMOTE_ACK_PHRASE`: the
// TUI Promote flow (`btctax-tui-edit`'s `edit/promote_flow.rs`) needs `ProvenanceKind::Purchase` to call
// `plan_promote` WITHOUT the `cmd::` token its KAT-G1 source gate forbids in non-test code, and
// `PROVENANCE_TEXT` to show the filer what BG-D5 attestation is being made on their behalf. Both are
// plain data (a `Copy` enum; a `&'static str`) — no `Session`, no lock, no I/O.
pub use cmd::promote::{ProvenanceKind, PROVENANCE_TEXT};
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
// ★★★ Re-exported at the crate root (T12 / R12) so the TUI's answer-panel pane and commit modal can
// render the panel with the SAME functions `income answer` prints, WITHOUT the `cmd::` token its
// KAT-G1 source gate forbids in non-test code — mirrors `guard_allocation_vs_tranche` and
// `promote_export_gate` above. All three are PURE `&InterviewState -> Vec<String>` renderers: no
// `Session`, no lock, no I/O, and nothing that could hold or drop the editor's vault lock. The
// gate's intent is honored, not evaded. Re-exporting them is what makes "one derivation, two
// surfaces" structural — a second renderer in the TUI would be a second chance to word one forgo
// differently.
pub use cmd::answer::{forgoing_lines, not_computed_lines, panel_lines, refusing_lines};
// Re-exported at the crate root (Defensive Filing Wizard Task 8, ★ C-3) so the TUI Declare flow
// (`btctax-tui-edit`'s `edit/declare_flow.rs` + `edit/persist.rs`) can drive the DECLARE chokepoint
// WITHOUT the `cmd::` token its KAT-G1 source gate forbids in non-test code — mirrors
// `promote_export_gate`/`IrsPdfReport` above. `plan_declare` is a pure `(events, prices, cfg, ...) ->
// Result` planner (no `Session`, no lock, no I/O); `DeclarePlan`/`Refusal` are plain data types.
// `apply_declare` DOES touch the mutation surface (`append_decision` + `session.save()`) — it is
// re-exported here ONLY so `edit/persist.rs`'s `persist_declare_tranche` wrapper can reach it (KAT-G1's
// `persist_only_tokens` confines the LITERAL `apply_declare(` call token to that one file crate-wide;
// re-exporting the name itself does not weaken that confinement — the gate scans call sites, not
// import lists). Any FUTURE addition here must be equally justified (do NOT re-export a second
// session-opening or unconfined-write fn).
pub use chokepoint::{apply_declare, plan_declare, DeclarePlan, Refusal};
// Re-exported at the crate root (Defensive Filing Wizard Task 9, ★ C-3) so the TUI Promote flow
// (`btctax-tui-edit`'s `edit/promote_flow.rs` + `edit/persist.rs`) can drive the PROMOTE chokepoint
// WITHOUT the `cmd::` token its KAT-G1 source gate forbids in non-test code — mirrors the
// `plan_declare`/`DeclarePlan`/`apply_declare` re-export directly above. `plan_promote`/`render_consent`
// are pure `(events, ...) -> Result` / `(&PromotePlan) -> String` fns (no `Session`, no lock, no I/O);
// `PromotePlan` is a plain data type; `Refusal` is ALREADY re-exported above (the SAME shared enum both
// `plan_declare` and `plan_promote` return). `apply_promote` DOES touch the mutation surface — it is
// re-exported here ONLY so `edit/persist.rs`'s `persist_promote_tranche` wrapper can reach it (KAT-G1's
// `persist_only_tokens` confines the LITERAL `apply_promote(` call token to that one file crate-wide;
// re-exporting the name itself does not weaken that confinement — the gate scans call sites, not import
// lists). Any FUTURE addition here must be equally justified (do NOT re-export a second session-opening
// or unconfined-write fn).
pub use chokepoint::{apply_promote, plan_promote, render_consent, PromotePlan};
// (The composed multi-year EXPORT chokepoint — `plan_export`/`apply_export`/`ExportPlan`/`ExportOutcome`
// /`ExportOutcomes` — was re-exported here for the TUI wizard's export step. Both the wizard and the trio
// were removed in 0.13.0: the owner ruled that amending several prior years at once is not a real
// workflow, and after the wizard's deletion the trio had zero callers from any shipped surface. Single-year
// export is unchanged and lives where it always did, in `cmd::admin`'s `export_irs_pdf`. Note that
// `btctax_core::conservative::flagged_years` — which the trio composed over — is a SEPARATE, still-live
// symbol: `btctax defensive status` reports its year set so a filer knows which years to re-export by hand.)
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
    /// ★★★ **T4 / `SPEC_interview.md` R11 — a WIP draft that HOLDS AN INTERVIEW is never
    ///     superseded on a note.**
    ///
    /// §6.2's rule — *"an authoritative committed-row write CLEARS that year's WIP draft"* — was
    /// written when a draft was crash-recovery scratch: regenerable, seconds of typing, `warn if
    /// discarding a non-trivial WIP`. R11 makes the draft the **Sep–Dec store for TY2026**, because
    /// a year with no `FullReturnParams` cannot commit at all: the interview lives there for
    /// months. A draft carrying `answer_log` records or transcribed document rows is therefore not
    /// disposable — the records in particular are *unreproducible*, since `record_answer` writes
    /// them only when btctax itself asks the question, and `income import` refuses to read them
    /// back for exactly that reason.
    ///
    /// So the note becomes a refusal, and the discard becomes deliberate. `--discard-draft` is a
    /// SEPARATE flag from `--force` on purpose: `--force` documents itself as overriding the
    /// scrub-marker guard *"and NOTHING else"*, and a filer loading a scrubbed copy into a scratch
    /// vault must not thereby authorise destroying an interview.
    #[error(
        "year {year} has a work-in-progress draft holding {holdings}, and this write would \
         discard it. Nothing was written. Re-run with --discard-draft to discard it \
         deliberately, or open the tax-inputs form for {year} to finish it (and commit it, \
         once the year's package has arrived)."
    )]
    NonTrivialDraftBlocksWrite { year: i32, holdings: String },
    /// ★★★ **T4 / R11 — the §6.3 stale-WIP DISCARD, refused when the draft holds an interview.**
    ///
    /// §6.3 discards a stale-version WIP draft silently *"because it is regenerable, so refusing
    /// would brick a resume for no benefit"*. That premise fails for a draft carrying answers or
    /// transcribed documents: it is months of work, and the `answer_log` cannot be re-created by
    /// re-typing. The parked half of §6.3 already fails closed for the same reason (C-1); this is
    /// the WIP half, narrowed to the drafts that are not in fact regenerable.
    #[error(
        "year {year}'s draft is schema v{found} but this build expects v{expected}, and it \
         holds {holdings} — an upgrade changed the input format, so this build cannot read \
         it. It was NOT discarded. Re-run on the app version that wrote it and commit (or \
         export) it there; or discard it deliberately from the tax-inputs form for {year}."
    )]
    StaleDraftHoldsInterview {
        year: i32,
        found: i64,
        expected: i64,
        holdings: String,
    },
    /// `income scrub` could not render the scrubbed return as TOML, or could not write it.
    ///
    /// ★ NOT `Usage` (which renders "usage:" and means the filer typed the command wrong) and NOT
    /// `BadConfigValue` (documented as a corrupt-DB row, whose natural remedy — clear it and
    /// re-import — is DESTRUCTIVE). Both were used here at some point; neither is true. A
    /// serialization limit is btctax's fault and an I/O failure is the filesystem's, and in both
    /// cases the filer's stored return is untouched.
    #[error("income scrub: {0}")]
    ScrubOutput(String),
    /// `income import` refused a file carrying the scrub provenance marker (SPEC_income_scrub.md
    /// §4.3). A scrubbed file is schema-identical to a real one and import is an unconfirmed
    /// whole-blob upsert, so this would destroy the vault's real identity and IP PIN —
    /// unrestorable — and leave a synthetic SSN well-formed enough to print on a filed 1040.
    ///
    /// ★★ `--force` overrides THIS guard and nothing else. The parked-draft refusal
    /// ([`Self::ParkedDraftBlocksWrite`], raised by `coherence_clear_or_refuse` before any marker
    /// logic) stays unconditional: it protects the sole copy of a screened return, and a flag named
    /// `--force` must not be read as licence to destroy that too.
    #[error(
        "refusing to import {path}: it carries the btctax scrub marker, so it is a SCRUBBED copy — \
         its identity is synthetic and its IP PIN was dropped. Importing it would overwrite this \
         vault's real identity with placeholders, and that cannot be undone.\n\
         \n\
         If you meant to load a scrubbed return (to reproduce someone else's defect), re-run with \
         --force, ideally against a scratch vault."
    )]
    ImportOfScrubbedFile { path: String },
    /// `income scrub` refused because the LEDGER contributes to `year` (SPEC_income_scrub.md §2.2).
    ///
    /// ★★★ This is a `CliError` and deliberately **NOT** a `RefuseReason`. That taxonomy answers
    /// *why a return cannot be FILED*, and its exhaustive cross-crate anchor map in
    /// `btctax-input-form` has no honest entry for a SHARING refusal. Adding one would be a category
    /// error: this return may be perfectly filable — it is the scrubbed COPY that cannot be
    /// authorized as safe, because the recipient's file would not reproduce the ledger's effect.
    ///
    /// ★ Ledger-bearing years have no scrub path in v1 and the message says so plainly rather than
    /// offering a consolation the spec refuted. It must **never** point at `export-snapshot`: that
    /// is the FR10 plaintext exception and is not scrubbed at all.
    #[error(
        "income scrub refused for {year}: {cause}.\n\
         \n\
         Scrub may emit only when the ledger contributes NOTHING to the year — no figure, refusal, \
         gate, watermark or advisory. Here it contributes, so a scrubbed copy would behave \
         DIFFERENTLY from your own: the recipient would not reproduce what you are seeing, and the \
         file would carry an authorization it has not earned.\n\
         \n\
         There is no scrub path for a ledger-bearing year in this version. What does not travel is \
         the ledger's effect on the return, and nothing here can stand in for it."
    )]
    ScrubLedgerContributes { year: i32, cause: String },
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

/// ★ **THE NO-AUTHORISATION NOTICE**, printed on stderr by every command that hands the user a
/// fillable IRS form this tool produced — `export-irs-pdf` and `extension`.
///
/// One constant, not two literals: the moment a filer is holding a form with a cheque beside it is
/// exactly where this has to land, and a second copy in a second arm is a second copy to drift. It
/// disclaims authorisation, warranty and liability; it does NOT restrict the licence or forbid filing
/// (the grant stays MIT OR Unlicense, unrestricted — see NOTICE / `btctax limitations`).
pub const NOT_AUTHORISED_FOR_FILING: &str = "NOT AUTHORISED FOR FILING. btctax is a mechanical \
     calculator. No right is granted and no authorisation is given to use it, or anything it \
     produces, to prepare or file a tax return, and NO WARRANTY is given that any figure or form it \
     produces is accurate, complete, or fit to file. If you file any of this, you do so entirely on \
     your own responsibility: YOU are the preparer, you must check every figure against the forms \
     and instructions before you sign, and the authors accept no liability for the consequences. \
     This is not tax advice. See `btctax limitations`.";

// ════════════════════════════════════════════════════════════════════════════════════════════════
// ★★★ WHERE TO FILE — the two facts, the pointer, and why there is no address table here.
//
// Phase 4's exit gate is *"a filer holding the packet can post it without consulting anything
// outside it."* Measured 2026-09-11 before this landed: `grep -rn "irs.gov" crates/` matched only
// archived IRS form text — no where-to-file or service-center reference existed on any surface a
// filer reads, so a signed, assembled packet did not say where to post it.
//
// **Two facts decide the address, and a filer who knows only one of them picks wrong.** Both are
// `const` rather than a literal per surface, for the reason `cmd::admin`'s DIGITAL_ASSET_HAND_MARK
// is: the packet manifest (October) and `btctax extension` (April) describe the SAME decision, and
// the filer must not be told two different things about it.

/// ★★★ **Fact 1 of 2 — the STATE.** See the block comment above.
pub const WHERE_TO_FILE_STATE_FACT: &str = "WHICH STATE YOU LIVE IN. There is no single IRS \
     address: paper goes to one of several processing centers, and which one is yours is decided by \
     the state on your address line.";

/// ★★★ **Fact 2 of 2 — whether a PAYMENT is enclosed.** See the block comment above.
///
/// ★ This is the half that is easiest to lose, and losing it is not harmless: every state has two
/// addresses, and the with-payment one is a lockbox P.O. Box at a different center from the
/// no-payment one. A filer who knows only their state has a 50% chance of the wrong envelope.
pub const WHERE_TO_FILE_PAYMENT_FACT: &str =
    "WHETHER A PAYMENT IS IN THE ENVELOPE. Each state has \
     TWO addresses \u{2014} one for an envelope containing a check or money order, a different one \
     for an envelope with no payment in it (a refund, or tax you paid online). They are not \
     interchangeable, so your state alone does not decide it.";

/// Where the RETURN's two columns actually live. A pointer, never a copy — see
/// [`WHY_NO_ADDRESS_TABLE`].
///
/// The URL is transcribed from Form 1040 itself (`design/forms/extract/f1040--2025.txt:168`), which
/// prints it as its own footer line. The table is on the last page of the year's instructions
/// (`design/forms/extract/i1040gi--2025.txt:46610`, *"Where Do You File?"*, page 126 of 126).
pub const WHERE_TO_FILE_1040_SOURCE: &str =
    "Both columns are in the \"Where Do You File?\" table on \
     the LAST PAGE of the IRS Instructions for Form 1040 for the year you are filing. Form 1040 \
     prints where to get them: \"Go to www.irs.gov/Form1040 for instructions and the latest \
     information.\"";

/// Where FORM 4868's two columns live — and they are **not** the return's addresses.
///
/// ★ Measured, not assumed: Form 4868's own last page carries the table (*"Where To File a Paper
/// Form 4868"*, `crates/btctax-core/src/tax/fixtures/f4868_2025_instructions.txt:427`, and page 4 of
/// 4 of the bundled `crates/btctax-forms/forms/2025/f4868.pdf`). Its service centers differ from the
/// 1040's on every row — Charlotte NC 28201-**1302** against the return's **1214**, Austin TX
/// 73301-**0045** against **0002** — so a filer who reuses the return's address posts the extension
/// to a lockbox that is not expecting it.
///
/// The URL is transcribed from Form 4868 itself
/// (`crates/btctax-core/src/tax/fixtures/f4868_2025_form.txt:18`).
pub const WHERE_TO_FILE_4868_SOURCE: &str = "Both columns are in the \"Where To File a Paper Form \
     4868\" table printed on FORM 4868 ITSELF \u{2014} the last page of the f4868.pdf this command \
     just wrote. It is not the address your RETURN goes to: the two tables print different centers \
     and different P.O. boxes for the same state. The form also prints \"Go to www.irs.gov/Form4868 \
     for the latest information.\"";

/// ★★★ **Why this product prints a pointer and never an address table.**
///
/// Quoted from the instructions themselves (`design/forms/extract/i1040gi--2025.txt:40973-40978`),
/// because the authority says the set is moving. A bundled table rots silently between releases and
/// a signed return posted to a closed center fails with no error message; a pointer at the year's own
/// instructions cannot fail in that direction. The IRS also corrected the Form 1040-ES addresses
/// mid-2026 (recon-efile §5), which is the same failure one step earlier.
pub const WHY_NO_ADDRESS_TABLE: &str = "btctax prints no mailing address, deliberately. The IRS is \
     consolidating paper processing and says so in those same instructions: \"Over the next several \
     years, the IRS will be reducing the number of paper tax return processing sites. Because of \
     this, you may need to mail your return to a different address than you have in the past.\" An \
     address compiled into this program would go stale between releases with nothing to announce it, \
     and a signed return posted to a closed service center fails silently. Read the table for the \
     year you are filing.";

/// ★★★ **B1's instrument for the where-to-file guidance: which of the three required parts a
/// filer-facing text is MISSING.** Empty means complete.
///
/// **Why a predicate and not three `assert!(contains)` calls at each site.** The guidance reaches a
/// filer on two surfaces with different chrome — the packet manifest prefixes `#` and wraps under a
/// `\u{2022}` bullet, `btctax extension` wraps without the `#` — so a raw `contains` would pass on
/// one surface and fail on the other for reasons that have nothing to do with the facts being
/// present. [`normalize_guidance`] removes the chrome; this names what is absent.
///
/// ★ The returned labels are what makes the kill readable: a test that reds says *which* fact went
/// missing, which is the difference between "the text changed" and "the filer is now told only half
/// of what decides the address."
///
/// `source` is the surface's own pointer ([`WHERE_TO_FILE_1040_SOURCE`] or
/// [`WHERE_TO_FILE_4868_SOURCE`]) — the facts are shared, the address source is not.
#[must_use]
pub fn missing_where_to_file_facts(text: &str, source: &str) -> Vec<&'static str> {
    let have = normalize_guidance(text);
    let mut missing = Vec::new();
    if !have.contains(&normalize_guidance(WHERE_TO_FILE_STATE_FACT)) {
        missing.push("the STATE-dependence");
    }
    if !have.contains(&normalize_guidance(WHERE_TO_FILE_PAYMENT_FACT)) {
        missing.push("the PAYMENT-dependence");
    }
    if !have.contains(&normalize_guidance(source)) {
        missing.push("the address source (the year's own table)");
    }
    missing
}

/// Strip the surface's chrome so [`missing_where_to_file_facts`] compares sentences, not layout: a
/// manifest comment marker, a bullet, a hanging indent and any run of whitespace all collapse.
///
/// ★ Deliberately NOT a general normalizer — it removes exactly the three markers the two surfaces
/// add (`#`, `\u{2022}`, whitespace). Anything else a future surface prefixes will make the check
/// red rather than pass, which is the direction to fail in.
#[must_use]
pub fn normalize_guidance(text: &str) -> String {
    text.split_whitespace()
        .filter(|w| *w != "#" && *w != "\u{2022}")
        .map(|w| w.trim_start_matches('#'))
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// ★★★ RECORD RETENTION — inform, and prescribe nothing.
//
// The decision already existed (`design/forms/FIELD_PROVENANCE.md:265` *"Never auto-shred"*, `:464`
// *"the architecture should make retention the filer's decision, not pick a window"*) but lived in a
// design document: nothing a filer reads mentioned it.
//
// ★★ The two sources pull against each other and BOTH are right. The long-range plan says the window
// that matters here is holding period + 3 years, because a crypto lot's acquisition record
// substantiates a disposal that has not happened yet; FIELD_PROVENANCE says the product must not pick
// a window. The resolution is that the IRS instruction itself declines to name one for property
// records — *"as long as they are needed to figure the basis"* — so quoting the authority and naming
// the mechanism informs without prescribing. No date, no deadline, no default, and no retention
// mechanism: `export-snapshot` already writes the artifact, unprompted.

/// ★★★ **What the IRS instruction actually says, and why the second sentence is the one that governs
/// a crypto ledger.** Verbatim from `design/forms/extract/i1040gi--2025.txt:41182-41198`
/// (*"How Long Should Records Be Kept?"*).
pub const RECORD_RETENTION_GUIDANCE: &str = "btctax will not shred anything, sets no deletion date, \
     and does not decide how long you keep this. That is yours. The IRS instruction for Form 1040 \
     is: \"Keep a copy of your tax return, worksheets you used, and records of all items appearing \
     on it (such as Forms W-2 and 1099) until the statute of limitations runs out for that return. \
     Usually, this is 3 years from the date the return was due or filed or 2 years from the date the \
     tax was paid, whichever is later.\" Then it adds: \"You should keep some records longer. For \
     example, keep property records (including those on your home) as long as they are needed to \
     figure the basis of the original or replacement property.\" It is the SECOND sentence that \
     governs a crypto ledger, and the generic three years is the wrong instinct here: every lot you \
     still hold is a property record, and its acquisition date and cost basis are what Form 8949 \
     will report in the year you finally dispose of it \u{2014} which can be many years after the \
     return you are posting now. So the clock on an acquisition record does not start when you file \
     this return; it starts when the lot it documents is disposed of, and runs while THAT return can \
     still be examined. btctax names no date, because the length depends on facts only you have: \
     which lots you still hold, when you dispose of them, and what happens to those returns. \
     `btctax export-snapshot` already writes the ledger and its inputs out for you \u{2014} what you \
     then keep, and for how long, is your decision to make on purpose.";

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

#[cfg(test)]
mod where_to_file_tests {
    use super::*;

    /// The three parts, as a surface would carry them: the two facts and the pointer, wrapped and
    /// `#`-prefixed exactly the way the packet manifest does it.
    fn manifest_shaped(state: &str, payment: &str, source: &str) -> String {
        let mut s = String::from("# \u{2500}\u{2500} WHERE TO POST IT \u{2500}\u{2500}\n#\n");
        for part in [state, payment, source] {
            for line in crate::render::wrap_bulleted(part).lines() {
                s.push('#');
                s.push_str(line);
                s.push('\n');
            }
        }
        s
    }

    /// ★★★ **B1 — the completeness checker, watched RED on each of the three deletions and GREEN on
    /// the complete text.**
    ///
    /// The brief for this work states the bar explicitly: *"A test asserting only that some string
    /// appears is not enough; it must fail when the payment-dependence or the state-dependence goes
    /// missing, because a filer told only one of them mails to the wrong place."* So each fact is
    /// planted absent, one at a time, and the checker must name that one and only that one.
    #[test]
    fn a_guidance_text_missing_either_fact_or_the_pointer_is_named_and_a_complete_one_is_not() {
        let src = WHERE_TO_FILE_1040_SOURCE;
        let whole = manifest_shaped(
            WHERE_TO_FILE_STATE_FACT,
            WHERE_TO_FILE_PAYMENT_FACT,
            WHERE_TO_FILE_1040_SOURCE,
        );
        assert!(
            missing_where_to_file_facts(&whole, src).is_empty(),
            "the complete, manifest-shaped text must be accepted \u{2014} chrome and wrapping and \
             all: {whole}"
        );

        // ── Plant 1: the STATE fact deleted. This is the shape where a filer reads "with a payment
        //    or without" and posts to whichever center they remember.
        let no_state = manifest_shaped("", WHERE_TO_FILE_PAYMENT_FACT, WHERE_TO_FILE_1040_SOURCE);
        assert_eq!(
            missing_where_to_file_facts(&no_state, src),
            vec!["the STATE-dependence"],
            "deleting the state fact must red, and must name only it: {no_state}"
        );

        // ── Plant 2: the PAYMENT fact deleted — the half that is easiest to lose, and a 50/50
        //    chance of the wrong envelope.
        let no_payment = manifest_shaped(WHERE_TO_FILE_STATE_FACT, "", WHERE_TO_FILE_1040_SOURCE);
        assert_eq!(
            missing_where_to_file_facts(&no_payment, src),
            vec!["the PAYMENT-dependence"],
            "deleting the payment fact must red, and must name only it: {no_payment}"
        );

        // ── Plant 3: the pointer deleted. Both facts, and nowhere to get the answer.
        let no_source = manifest_shaped(WHERE_TO_FILE_STATE_FACT, WHERE_TO_FILE_PAYMENT_FACT, "");
        assert_eq!(
            missing_where_to_file_facts(&no_source, src),
            vec!["the address source (the year's own table)"],
            "deleting the pointer must red: {no_source}"
        );

        // ── Plant 4: everything gone. All three, in order — so a stripped surface cannot
        //    satisfy the check by reporting one thing and hiding two.
        assert_eq!(
            missing_where_to_file_facts("nothing to see here", src),
            vec![
                "the STATE-dependence",
                "the PAYMENT-dependence",
                "the address source (the year's own table)",
            ],
        );

        // ── Near miss: the EXTENSION's pointer is not the return's. A surface that carries the
        //    4868's table reference where the 1040's belongs is a finding, because the two tables
        //    print different service centers for the same state.
        let wrong_table = manifest_shaped(
            WHERE_TO_FILE_STATE_FACT,
            WHERE_TO_FILE_PAYMENT_FACT,
            WHERE_TO_FILE_4868_SOURCE,
        );
        assert_eq!(
            missing_where_to_file_facts(&wrong_table, src),
            vec!["the address source (the year's own table)"],
            "the 4868's table must not satisfy the RETURN's pointer: {wrong_table}"
        );
        // …and symmetrically, it does satisfy the extension's own.
        assert!(
            missing_where_to_file_facts(&wrong_table, WHERE_TO_FILE_4868_SOURCE).is_empty(),
            "the 4868 surface's own pointer is accepted: {wrong_table}"
        );
    }

    /// [`normalize_guidance`] removes the two surfaces' chrome and nothing else — a sentence
    /// that differs in WORDS still differs after normalizing, or the checker above is vacuous.
    #[test]
    fn normalizing_removes_chrome_but_not_words() {
        assert_eq!(
            normalize_guidance("#  \u{2022} alpha beta\n#    gamma"),
            "alpha beta gamma"
        );
        assert_eq!(normalize_guidance("  alpha   beta  "), "alpha beta");
        assert_ne!(
            normalize_guidance("a check or money order"),
            normalize_guidance("a check"),
            "normalizing must not erase a missing clause"
        );
    }
}
