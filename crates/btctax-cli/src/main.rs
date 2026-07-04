//! btctax — thin clap-4 dispatch over the btctax_cli library. Resolves the passphrase (env seam for
//! non-interactive use; otherwise a secure prompt), calls one library command, renders, and sets the
//! exit code (non-zero on FR9 hard blockers / on any CliError). NO business logic lives here.
use btctax_cli::{cmd, eventref, render, CliError};
use btctax_core::{
    AllocMethod, Carryforward, DisposeKind, DonationDetails, FeeTreatment, FilingStatus,
    InboundClass, LotMethod, OutflowClass, TaxProfile, TransferTarget,
};
use btctax_store::Passphrase;
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use std::process::ExitCode;
use time::{OffsetDateTime, UtcOffset};

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
        /// Clear an interrupted/half-created init (orphan `vault.key`, no encrypted store) and start fresh.
        #[arg(long, default_value_t = false)]
        repair: bool,
    },
    /// Import one or more export files (auto-groups Swan).
    Import { files: Vec<PathBuf> },
    /// FR9 integrity check (non-zero exit on hard blockers).
    Verify,
    /// Show holdings + realized disposals/removals/income. With --tax-year: standalone TaxResult.
    #[command(alias = "show")]
    Report {
        /// Filter realized disposals/removals/income to a specific calendar year (display path).
        #[arg(long)]
        year: Option<i32>,
        /// Compute the crypto-attributable federal tax for the given tax year (B.5 / Task 9).
        /// Requires a stored tax profile (`tax-profile --year Y ...`) and the bundled TY table.
        /// Independent of --year; the two flags are not aliased.
        #[arg(long)]
        tax_year: Option<i32>,
        /// Cumulative prior-year TAXABLE gifts (post-annual-exclusion Form 709 amounts), not
        /// gross gifts. Used for the §2505 lifetime-exclusion consumption advisory. Defaults to
        /// $0 when omitted (the advisory discloses this assumption). Must not be negative.
        #[arg(long)]
        prior_taxable_gifts: Option<String>,
    },
    /// Emit a reconciliation decision event.
    #[command(subcommand)]
    Reconcile(Reconcile),
    /// Show or set projection config (TP8 fee treatment / pre-2025 lot method / forward method).
    Config {
        #[arg(long, value_enum)]
        set_fee_treatment: Option<FeeArg>,
        #[arg(long, value_enum)]
        set_pre2025_method: Option<MethodLotArg>,
        #[arg(long, default_value_t = false)]
        attest_pre2025_method: bool,
        /// §A.5(a): append a MethodElection decision (the forward standing order). Not a flag
        /// mutation — this is an event in the ledger. Use --effective-from to set the date
        /// (default: today / the decision's made-date).
        #[arg(long, value_enum)]
        set_forward_method: Option<MethodLotArg>,
        /// Effective-from date for --set-forward-method (YYYY-MM-DD). Defaults to made-date.
        #[arg(long)]
        effective_from: Option<String>,
    },
    /// FR10: export decrypted SQLite + CSV (the NFR2 plaintext exception).
    ExportSnapshot {
        #[arg(long)]
        out: PathBuf,
        /// Also emit the per-tax-year Form 8949 + Schedule D CSVs (form8949.csv / schedule_d.csv),
        /// scoped to this calendar year. Omit to write only the all-years projection CSVs.
        #[arg(long)]
        tax_year: Option<i32>,
    },
    /// Export the passphrase-protected key.
    BackupKey {
        #[arg(long)]
        out: PathBuf,
    },
    /// Lot-specific-identification optimizer (§C — read-only proposal or gated persistence).
    #[command(subcommand)]
    Optimize(Optimize),
    /// Set or show the per-tax-year tax profile (filing status, income, MAGI, etc.).
    TaxProfile {
        /// The tax year (e.g. 2025).
        #[arg(long)]
        year: i32,
        /// IRS filing status.
        #[arg(long, value_enum)]
        filing_status: Option<FilingStatusArg>,
        /// Ordinary taxable income EXCLUDING all app-computed crypto items (net ST gains,
        /// mining/staking ordinary income). The engine adds the crypto items on top (B.1 / I5).
        #[arg(long)]
        ordinary_taxable_income: Option<String>,
        /// Modified AGI excluding crypto items, for the §1411 NIIT threshold comparison.
        ///
        /// IMPORTANT (§1411 contract): this value MUST already include the taxpayer's qualified
        /// dividends and non-crypto net capital gains (and any other MAGI add-backs from
        /// §1411(d)). The engine adds ONLY the crypto AGI delta on top (ambiguity #5 in the
        /// design). Omitting QD or non-crypto cap gains from this figure understates NIIT.
        #[arg(
            long,
            long_help = "Modified AGI excluding crypto items, for the §1411 NIIT \
            threshold comparison.\n\nIMPORTANT (§1411 contract): this value MUST already \
            include the taxpayer's qualified dividends and non-crypto net capital gains (and \
            any other MAGI add-backs from §1411(d)). The engine adds ONLY the crypto AGI \
            delta on top (ambiguity #5 in the design). Omitting QD or non-crypto cap gains \
            from this figure understates NIIT."
        )]
        magi_excluding_crypto: Option<String>,
        /// Qualified dividends + other preferential-rate income sharing the §1(h) 0/15/20 LTCG
        /// rate stack. Required when setting a profile.
        #[arg(long)]
        qualified_dividends: Option<String>,
        /// Non-crypto net LT-character capital gain already in the profile (optional; defaults
        /// to 0 when omitted).
        #[arg(long)]
        other_net_capital_gain: Option<String>,
        /// §1212(b) short-term capital loss carryforward into this year (optional; defaults to 0).
        #[arg(long)]
        carryforward_short: Option<String>,
        /// §1212(b) long-term capital loss carryforward into this year (optional; defaults to 0).
        #[arg(long)]
        carryforward_long: Option<String>,
        /// Form W-2 Social Security wages (Box 3 + Box 7 tips; Schedule SE line 8a).
        /// Reduces the §1401(a) SS cap: ss_cap = max(0, wage_base − w2_ss_wages). Optional;
        /// defaults to $0. Must not be negative.
        #[arg(long)]
        w2_ss_wages: Option<String>,
        /// Medicare wages (Box 5; Form 8959 line 1).
        /// Reduces the Additional-Medicare threshold: addl_threshold = max(0, threshold − w2_medicare_wages)
        /// (§1401(b)(2)(B)/Form 8959 Part II). Optional; defaults to $0. Must not be negative.
        #[arg(long)]
        w2_medicare_wages: Option<String>,
        /// Schedule C deductible business expenses for the year — reduces net SE earnings;
        /// the income-tax stack above is NOT adjusted (see the advisory).
        /// Optional; defaults to $0. Must not be negative.
        #[arg(long)]
        schedule_c_expenses: Option<String>,
        /// Show the stored profile for `--year` instead of setting it.
        #[arg(long, default_value_t = false)]
        show: bool,
    },
}

/// `optimize` subcommand tree.  Task 9 adds `Run`; Task 10 adds `Accept`; Task 11 adds `Consult`.
#[derive(Subcommand)]
enum Optimize {
    /// Mode-1 what-if: print the tax-saving lot-selection proposal. NOTHING is filed or bound.
    Run {
        /// The tax year to optimize (must be 2025 or later).
        #[arg(long)]
        tax_year: i32,
    },
    /// Mode-1 gated persistence: recompute the optimum and persist the proposed LotSelection(s),
    /// gated per disposal (§1.1012-1(j)). A genuinely-contemporaneous pick (made ≤ sale) persists
    /// freely; an already-executed disposal persists ONLY with a narrow per-disposal `--attest`
    /// scoped to one `--disposal`; a 2027+ broker-held pick is refused. Revoke via `reconcile void`.
    Accept {
        /// The tax year to accept (must be 2025 or later).
        #[arg(long)]
        tax_year: i32,
        /// Restrict to ONE disposal (required to carry `--attest`).
        #[arg(long)]
        disposal: Option<String>,
        /// Narrow contemporaneous-ID attestation for an already-executed disposal. Requires
        /// `--disposal` (no blanket attestation across all disposals).
        #[arg(long)]
        attest: Option<String>,
    },
    /// Mode-2 read-only pre-trade what-if (§C.3): tax-min lots + ST/LT split + federal tax + ST→LT
    /// timing. NOTHING is written — no event, no side-table row. Tax decision-support only;
    /// not buy/sell/hold advice.
    Consult {
        /// Hypothetical sale amount in satoshis (required).
        #[arg(long)]
        sell: String,
        /// Wallet to sell from, e.g. `self:cold` or `exchange:coinbase:default` (required; per-wallet
        /// pool is mandatory post-2025).
        #[arg(long)]
        wallet: Option<String>,
        /// Sale date for the what-if (YYYY-MM-DD; defaults to today UTC if omitted).
        #[arg(long)]
        at: Option<String>,
        /// Explicit USD proceeds for the hypothetical sale. Required when `--at` is a future date
        /// with no bundled dataset price and `--fmv` is not used. Mutually exclusive with `--fmv`.
        #[arg(long, conflicts_with = "fmv")]
        proceeds: Option<String>,
        /// Use the bundled daily-close FMV for `--at` instead of an explicit proceeds amount.
        /// A future date with no dataset price will return a ProceedsRequired error. Mutually
        /// exclusive with `--proceeds`.
        #[arg(long, conflicts_with = "proceeds")]
        fmv: bool,
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
    /// Classify an inbound TransferIn as an inbound self-transfer ("my own coins" returning) —
    /// non-taxable, creates a fresh lot. `--basis` defaults to $0 (conservative; fires the honest
    /// zero-basis advisory when omitted); `--acquired` defaults to the receipt date (short-term).
    ClassifyInboundSelfTransfer {
        in_ref: String,
        #[arg(long)]
        basis: Option<String>,
        #[arg(long)]
        acquired: Option<String>,
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
        /// Free-form donee identifier (e.g. "Alice", "Charity X"). Carried through to
        /// removals.csv and Form 8283; does not affect tax math.
        #[arg(long)]
        donee: Option<String>,
    },
    /// Set a manual FMV on an event.
    SetFmv {
        event: String,
        #[arg(long)]
        fmv: String,
    },
    /// Void a revocable decision.
    Void { target: String },
    /// Resolve an Unclassified row from a JSON imported payload.
    ClassifyRaw {
        target: String,
        #[arg(long)]
        payload_json: String,
    },
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
    /// §A.4 Specific-ID: pick the exact lots a disposal consumes.
    SelectLots {
        disposal: String,
        #[arg(long = "from", required = true)]
        from: Vec<String>,
    },
    /// §A.4 Batch import LotSelections from a CSV (disposal_ref,origin_event_id,split_sequence,sat).
    ImportSelections { csv: PathBuf },
    /// SE-completion Chunk C: flip `business` (and optionally `kind`) on an already-imported Income event.
    ///
    /// Corrects the `business: false` hard-code that River (and other adapters) emit at ingest time,
    /// enabling SE-tax treatment for professional miners / stakers. The engine validates that the target
    /// event exists and is an Income event — a missing or non-Income target fires a Hard DecisionConflict
    /// blocker (decision excluded). For TransferIn rows use `classify-inbound-income` instead.
    ///
    /// DecisionConflict is Hard — to re-decide, `void` the prior decision first, then re-issue.
    ReclassifyIncome {
        /// The Income event reference (from `report` or `income_recognized.csv` 'event' column).
        income_event: String,
        /// Whether this income is from a trade or business (true → SE-tax eligible).
        /// Must be supplied explicitly: `--business true` or `--business false`.
        #[arg(long, required = true, action = clap::ArgAction::Set)]
        business: bool,
        /// Optional income kind correction: mining|staking|interest|airdrop|reward.
        /// Omit to keep the original kind (only flip `business`).
        #[arg(long)]
        kind: Option<String>,
    },
    /// Store Form 8283 Section-B donation + appraiser details for a donation event.
    /// The event ref is the TransferOut EventId from the removals.csv 'event' column.
    SetDonationDetails {
        /// TransferOut event reference for the donation (from removals.csv 'event' column).
        out_event_ref: String,
        /// Donee organization name (Part IV; required).
        #[arg(long, required = true)]
        donee_name: String,
        /// Donee mailing address (Part IV; optional).
        #[arg(long)]
        donee_address: Option<String>,
        /// Donee EIN (Part IV; required for Section-B completeness).
        #[arg(long)]
        donee_ein: Option<String>,
        /// Qualified appraiser name (Part III; required).
        #[arg(long, required = true)]
        appraiser_name: String,
        /// Appraiser mailing address (Part III; optional).
        #[arg(long)]
        appraiser_address: Option<String>,
        /// Appraiser TIN/SSN/EIN (Part III §6695A; satisfies the TIN-or-PTIN requirement).
        #[arg(long)]
        appraiser_tin: Option<String>,
        /// Appraiser PTIN (Part III §6695A; satisfies the TIN-or-PTIN requirement).
        #[arg(long)]
        appraiser_ptin: Option<String>,
        /// Appraiser qualifications declaration (§170(f)(11)(E)).
        #[arg(long)]
        appraiser_qualifications: Option<String>,
        /// Date the qualified appraisal was made (YYYY-MM-DD).
        #[arg(long)]
        appraisal_date: Option<String>,
        /// FMV determination method override (overrides the section-derived default on the
        /// Form 8283 carrier row; resolves the Section-A fmv_method deferral when supplied).
        #[arg(long)]
        fmv_method: Option<String>,
    },
    /// Show stored Form 8283 donation details for a donation event.
    ShowDonationDetails {
        /// TransferOut event reference for the donation (from removals.csv 'event' column).
        out_event_ref: String,
    },
    /// Bulk-confirm self-transfers: link every PENDING outbound transfer in a time frame to one
    /// destination wallet (non-taxable). Shows a preview + requires --yes (or interactive y/N).
    BulkLinkTransfer {
        /// Destination wallet every selected outflow links to.
        #[arg(long)]
        to_wallet: String,
        /// Restrict to a single tax year (mutually exclusive with --from/--to).
        #[arg(long, conflicts_with_all = ["from", "to"])]
        year: Option<i32>,
        /// Range start (YYYY-MM-DD; requires --to).
        #[arg(long, requires = "to")]
        from: Option<String>,
        /// Range end (YYYY-MM-DD, inclusive; requires --from).
        #[arg(long, requires = "from")]
        to: Option<String>,
        /// Only outflows FROM this source wallet.
        #[arg(long)]
        from_wallet: Option<String>,
        /// Print the preview and exit without writing.
        #[arg(long)]
        dry_run: bool,
        /// Skip the interactive confirmation (non-interactive apply).
        #[arg(long)]
        yes: bool,
    },
    /// Bulk-classify unknown-basis inbound deposits as self-transfer-ins ("my own coins"): apply
    /// Cycle A's `SelfTransferMine` ($0 conservative basis, non-taxable) to MANY pending inbounds in a
    /// time frame at once. Shows a preview surfacing the total USD given $0 basis (the over-tax
    /// exposure) + requires --yes (or interactive y/N). Each is a voidable decision; for a deposit
    /// whose real cost you can substantiate, classify it single-item with `classify-inbound-self-transfer --basis`.
    BulkClassifyInboundSelfTransfer {
        /// Restrict to a single tax year (mutually exclusive with --from/--to).
        #[arg(long, conflicts_with_all = ["from", "to"])]
        year: Option<i32>,
        /// Range start (YYYY-MM-DD; requires --to).
        #[arg(long, requires = "to")]
        from: Option<String>,
        /// Range end (YYYY-MM-DD, inclusive; requires --from).
        #[arg(long, requires = "from")]
        to: Option<String>,
        /// Only inbounds received INTO this wallet.
        #[arg(long)]
        wallet: Option<String>,
        /// Print the preview and exit without writing.
        #[arg(long)]
        dry_run: bool,
        /// Skip the interactive confirmation (non-interactive apply).
        #[arg(long)]
        yes: bool,
    },
    /// Bulk-resolve import conflicts: ACCEPT (adopt each new payload) or REJECT (keep each current
    /// payload) MANY flagged `ImportConflict`s in one confirmed batch. Shows a `current → new` preview,
    /// then requires --yes (or interactive y/N). Exactly one of --accept / --reject is required. Each
    /// resolution is NON-REVOCABLE (`SupersedeImport`/`RejectImport` cannot be voided); to resolve a
    /// conflict differently, exclude it and use single-item `accept-conflict`/`reject-conflict`.
    #[command(group(clap::ArgGroup::new("resolve_action").required(true).args(["accept", "reject"])))]
    BulkResolveConflict {
        /// Accept every listed conflict (adopt each new payload onto its target).
        #[arg(long)]
        accept: bool,
        /// Reject every listed conflict (keep each target's current payload).
        #[arg(long)]
        reject: bool,
        /// Print the preview and exit without writing.
        #[arg(long)]
        dry_run: bool,
        /// Skip the interactive confirmation (non-interactive apply).
        #[arg(long)]
        yes: bool,
    },
    /// Bulk-void MANY revocable reconcile decisions in one confirmed batch (bulk-void). Shows a preview
    /// of every voidable decision (the SHARED `voidable_decisions` predicate — effective safe-harbor
    /// allocations are OMITTED, #7), then requires --yes (or interactive y/N). Each void is
    /// NON-REVOCABLE (a `VoidDecisionEvent` cannot itself be voided — re-apply the original decision to
    /// restore). Voiding a `LotSelection` also re-exposes its disposal to the default method and clears
    /// its optimizer attestation.
    BulkVoid {
        /// Print the preview and exit without writing.
        #[arg(long)]
        dry_run: bool,
        /// Skip the interactive confirmation (non-interactive apply).
        #[arg(long)]
        yes: bool,
    },
    /// Match unreconciled inbound + outbound legs as self-transfers (self-transfer-passthrough C3).
    /// With no --in/--out: PREVIEW the proposed pairs (read-only). With --in and --out: confirm ONE
    /// pair (DROP for a same-wallet passthrough, RELOCATE for a cross-wallet transfer). NEVER automatic.
    MatchSelfTransfers {
        /// Confirm this in-leg (TransferIn eventref); requires --out.
        #[arg(long = "in", requires = "out_ref")]
        in_ref: Option<String>,
        /// Confirm this out-leg (TransferOut eventref); requires --in.
        #[arg(long = "out", requires = "in_ref")]
        out_ref: Option<String>,
        /// Override the suggested action (else the proposal's topology-derived action is used).
        #[arg(long, value_enum)]
        action: Option<SelfTransferActionArg>,
        /// Print the preview and exit without writing (conflicts with --in/--out).
        #[arg(long, conflicts_with_all = ["in_ref", "out_ref"])]
        dry_run: bool,
    },
}

#[derive(Copy, Clone, ValueEnum)]
enum SelfTransferActionArg {
    /// Same-wallet passthrough → SelfTransferPassthrough (both legs skipped, non-taxable).
    Drop,
    /// Cross-wallet transfer → TransferLink (relocate the lots to the destination wallet).
    Relocate,
}

#[derive(Copy, Clone, ValueEnum)]
enum FilingStatusArg {
    Single,
    Mfj,
    Mfs,
    Hoh,
    Qss,
}

impl From<FilingStatusArg> for FilingStatus {
    fn from(a: FilingStatusArg) -> Self {
        match a {
            FilingStatusArg::Single => FilingStatus::Single,
            FilingStatusArg::Mfj => FilingStatus::Mfj,
            FilingStatusArg::Mfs => FilingStatus::Mfs,
            FilingStatusArg::Hoh => FilingStatus::HoH,
            FilingStatusArg::Qss => FilingStatus::Qss,
        }
    }
}

#[derive(Copy, Clone, ValueEnum)]
enum FeeArg {
    C,
    B,
}

#[derive(Copy, Clone, ValueEnum)]
enum MethodLotArg {
    Fifo,
    Lifo,
    Hifo,
}

#[derive(Copy, Clone, ValueEnum)]
enum OutKindArg {
    Sell,
    Spend,
    Gift,
    Donate,
}

impl From<MethodLotArg> for LotMethod {
    fn from(a: MethodLotArg) -> Self {
        match a {
            MethodLotArg::Fifo => LotMethod::Fifo,
            MethodLotArg::Lifo => LotMethod::Lifo,
            MethodLotArg::Hifo => LotMethod::Hifo,
        }
    }
}

#[derive(Copy, Clone, ValueEnum)]
enum MethodArg {
    Actual,
    ProRata,
}

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
    let p = rpassword::prompt_password("Vault passphrase: ").map_err(CliError::Io)?;
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
        Command::Init { key_backup, repair } => {
            cmd::init::run_with_repair(vault, &passphrase(true)?, &key_backup, repair)?;
            println!(
                "{} vault {} (key backed up to {})",
                if repair {
                    "Repaired + initialized"
                } else {
                    "Initialized"
                },
                vault.display(),
                key_backup.display()
            );
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
        Command::Report {
            year,
            tax_year,
            prior_taxable_gifts,
        } => {
            // [R0-M3] Parse --prior-taxable-gifts as exact Decimal (no float); reject negative
            // REGARDLESS of whether --tax-year is present. Validated once, before the branch.
            // Uses Decimal::default() == 0; is_sign_negative() for the non-negative guard
            // (rejects negative; zero is accepted).
            let ptg_raw = prior_taxable_gifts
                .as_deref()
                .map(eventref::parse_usd_arg)
                .transpose()?
                .unwrap_or_default();
            if ptg_raw.is_sign_negative() {
                return Err(CliError::Usage(
                    "--prior-taxable-gifts must not be negative".into(),
                ));
            }
            if let Some(y) = tax_year {
                let (outcome, advisory, sched_d, gift_advisory, schedule_se, donation_appraisal) =
                    cmd::tax::report_tax_year(vault, &passphrase(false)?, y, ptg_raw)?;
                print!(
                    "{}",
                    render::render_tax_outcome(y, &outcome, advisory.as_deref())
                );
                print!("{}", render::render_schedule_d(y, &sched_d, &outcome));
                // P2-D Task 2: standalone Schedule SE §1401 SE-tax section (non-gating; STANDALONE —
                // does NOT feed engine B's total_federal_tax_attributable).
                if let Some(se) = schedule_se {
                    print!("{se}");
                }
                // P2-C Task 3 + Chunk-3a: standalone Form 709 gift advisory + §2505 lifetime-
                // exclusion consumption (non-gating; does not feed engine B).
                if let Some(msg) = gift_advisory {
                    println!("{msg}");
                }
                // Chunk-1 D2: §170(f)(11)(F) year-aggregate donation appraisal advisory (non-gating;
                // render-time only — does not feed engine B or the blocker set).
                if let Some(msg) = donation_appraisal {
                    println!("{msg}");
                }
            } else {
                let state = cmd::inspect::report(vault, &passphrase(false)?, year)?;
                print!("{}", render::render_report(&state, year));
            }
        }
        Command::Optimize(opt) => match opt {
            Optimize::Run { tax_year } => {
                let p = cmd::optimize::run(vault, &passphrase(false)?, tax_year, now)?;
                print!("{}", render::render_optimize_proposal(&p));
            }
            Optimize::Accept {
                tax_year,
                disposal,
                attest,
            } => {
                let outcome = cmd::optimize::accept(
                    vault,
                    &passphrase(false)?,
                    tax_year,
                    disposal.as_deref(),
                    attest.as_deref(),
                    now,
                )?;
                print!("{}", render::render_accept_outcome(&outcome));
            }
            Optimize::Consult {
                sell,
                wallet,
                at,
                proceeds,
                fmv: _,
                // `--fmv` simply leaves proceeds = None (forces dataset FMV); clap's conflicts_with
                // enforces that --fmv and --proceeds are never both passed.
            } => {
                let pp = passphrase(false)?;
                // Parse sell amount (satoshis, i64).
                let sell_sat = sell.trim().parse::<i64>().map_err(|e| {
                    CliError::Usage(format!(
                        "bad --sell {sell:?}: expected an integer sat amount: {e}"
                    ))
                })?;
                // --wallet is semantically required: the per-wallet pool is mandatory post-2025.
                let wallet_id = wallet
                    .as_deref()
                    .ok_or_else(|| {
                        CliError::Usage(
                            "--wallet is required for `optimize consult` \
                             (per-wallet pool is mandatory post-2025; use e.g. self:cold or \
                             exchange:coinbase:default)"
                                .into(),
                        )
                    })
                    .and_then(eventref::parse_wallet_id)?;
                // --at defaults to today UTC (the CLI clock seam; core stays clock-free).
                let at_date = at
                    .as_deref()
                    .map(eventref::parse_date_arg)
                    .transpose()?
                    .unwrap_or_else(|| btctax_core::conventions::tax_date(now, UtcOffset::UTC));
                // --proceeds: explicit USD; None when --fmv or neither flag (forces dataset FMV).
                let proceeds_usd = proceeds
                    .as_deref()
                    .map(eventref::parse_usd_arg)
                    .transpose()?;
                let report = cmd::optimize::consult(
                    vault,
                    &pp,
                    sell_sat,
                    wallet_id,
                    at_date,
                    proceeds_usd,
                    DisposeKind::Sell,
                )?;
                print!("{}", render::render_consult(&report));
            }
        },
        Command::Reconcile(r) => dispatch_reconcile(vault, r, now)?,
        Command::Config {
            set_fee_treatment,
            set_pre2025_method,
            attest_pre2025_method,
            set_forward_method,
            effective_from,
        } => {
            let pp = passphrase(false)?;

            // Task-1 review Minor: --attest-pre2025-method without --set-pre2025-method would
            // silently no-op under the old if/else dispatch. Reject with a clear error instead.
            // Checked first so no event/mutation is recorded for an invalid flag combination.
            if attest_pre2025_method && set_pre2025_method.is_none() {
                return Err(CliError::Usage(
                    "--attest-pre2025-method requires --set-pre2025-method".into(),
                ));
            }

            // M3 (apply-all, no silent drop): --set-forward-method APPENDS a MethodElection
            // decision (SPEC A.1 standing order) — it is an event, not a flag mutation. The old
            // dispatch returned early here, silently dropping any co-passed --set-fee-treatment /
            // --set-pre2025-method (the same anti-pattern Task 1/5 fixed for the config-flag pair).
            // Now every provided flag is applied independently; no early return.
            if let Some(m) = set_forward_method {
                let eff = effective_from
                    .as_deref()
                    .map(eventref::parse_date_arg)
                    .transpose()?;
                let id = cmd::reconcile::set_forward_method(vault, &pp, m.into(), eff, now)?;
                println!(
                    "Recorded standing order (MethodElection) {}",
                    id.canonical()
                );
            }

            // Task-1 review Minor (apply-all): apply each provided flag independently — no
            // silent drops. The old if/else dispatch ignored --set-fee-treatment when
            // --set-pre2025-method was also provided.
            if let Some(m) = set_pre2025_method {
                cmd::admin::set_pre2025_method(vault, &pp, m.into(), attest_pre2025_method)?;
            }
            if let Some(f) = set_fee_treatment {
                let t = match f {
                    FeeArg::C => FeeTreatment::TreatmentC,
                    FeeArg::B => FeeTreatment::TreatmentB,
                };
                cmd::admin::set_config(vault, &pp, Some(t))?;
            }
            let cfg = cmd::admin::show_config(vault, &pp)?;
            println!(
                "fee_treatment: {:?}\npre2025_method: {:?} (attested: {})",
                cfg.fee_treatment, cfg.pre2025_method, cfg.pre2025_method_attested
            );
        }
        Command::ExportSnapshot { out, tax_year } => {
            let p = cmd::admin::export_snapshot(vault, &passphrase(false)?, &out, tax_year)?;
            println!("Exported {} + CSVs to {}", p.display(), out.display());
        }
        Command::BackupKey { out } => {
            cmd::admin::backup_key(vault, &passphrase(false)?, &out)?;
            println!("Key backed up to {}", out.display());
        }
        Command::TaxProfile {
            year,
            filing_status,
            ordinary_taxable_income,
            magi_excluding_crypto,
            qualified_dividends,
            other_net_capital_gain,
            carryforward_short,
            carryforward_long,
            w2_ss_wages,
            w2_medicare_wages,
            schedule_c_expenses,
            show,
        } => {
            let pp = passphrase(false)?;
            if show {
                match cmd::tax::show_profile(vault, &pp, year)? {
                    Some(p) => println!(
                        "year: {year}\n\
                         filing_status: {}\n\
                         ordinary_taxable_income: {}\n\
                         magi_excluding_crypto: {}\n\
                         qualified_dividends_and_other_pref_income: {}\n\
                         other_net_capital_gain: {}\n\
                         capital_loss_carryforward_in.short: {}\n\
                         capital_loss_carryforward_in.long: {}\n\
                         w2_ss_wages: {}\n\
                         w2_medicare_wages: {}\n\
                         schedule_c_expenses: {}",
                        render::filing_status_tag(p.filing_status),
                        p.ordinary_taxable_income,
                        p.magi_excluding_crypto,
                        p.qualified_dividends_and_other_pref_income,
                        p.other_net_capital_gain,
                        p.capital_loss_carryforward_in.short,
                        p.capital_loss_carryforward_in.long,
                        p.w2_ss_wages,
                        p.w2_medicare_wages,
                        p.schedule_c_expenses,
                    ),
                    None => println!("none"),
                }
            } else {
                // Require all mandatory fields.
                let fs = filing_status.ok_or_else(|| {
                    CliError::Usage("--filing-status is required when setting a profile".into())
                })?;
                let oti = ordinary_taxable_income
                    .as_deref()
                    .ok_or_else(|| {
                        CliError::Usage(
                            "--ordinary-taxable-income is required when setting a profile".into(),
                        )
                    })
                    .and_then(eventref::parse_usd_arg)?;
                let magi = magi_excluding_crypto
                    .as_deref()
                    .ok_or_else(|| {
                        CliError::Usage(
                            "--magi-excluding-crypto is required when setting a profile".into(),
                        )
                    })
                    .and_then(eventref::parse_usd_arg)?;
                let qd = qualified_dividends
                    .as_deref()
                    .ok_or_else(|| {
                        CliError::Usage(
                            "--qualified-dividends is required when setting a profile".into(),
                        )
                    })
                    .and_then(eventref::parse_usd_arg)?;
                // Optional fields default to 0.
                let oncg = other_net_capital_gain
                    .as_deref()
                    .map(eventref::parse_usd_arg)
                    .transpose()?
                    .unwrap_or_default();
                let cf_short = carryforward_short
                    .as_deref()
                    .map(eventref::parse_usd_arg)
                    .transpose()?
                    .unwrap_or_default();
                let cf_long = carryforward_long
                    .as_deref()
                    .map(eventref::parse_usd_arg)
                    .transpose()?
                    .unwrap_or_default();
                let w2_ss = w2_ss_wages
                    .as_deref()
                    .map(eventref::parse_usd_arg)
                    .transpose()?
                    .unwrap_or_default();
                if w2_ss.is_sign_negative() {
                    return Err(CliError::Usage("--w2-ss-wages must not be negative".into()));
                }
                let w2_medicare = w2_medicare_wages
                    .as_deref()
                    .map(eventref::parse_usd_arg)
                    .transpose()?
                    .unwrap_or_default();
                if w2_medicare.is_sign_negative() {
                    return Err(CliError::Usage(
                        "--w2-medicare-wages must not be negative".into(),
                    ));
                }
                let sce = schedule_c_expenses
                    .as_deref()
                    .map(eventref::parse_usd_arg)
                    .transpose()?
                    .unwrap_or_default();
                if sce.is_sign_negative() {
                    return Err(CliError::Usage(
                        "--schedule-c-expenses must not be negative".into(),
                    ));
                }

                let profile = TaxProfile {
                    filing_status: FilingStatus::from(fs),
                    ordinary_taxable_income: oti,
                    magi_excluding_crypto: magi,
                    qualified_dividends_and_other_pref_income: qd,
                    other_net_capital_gain: oncg,
                    capital_loss_carryforward_in: Carryforward {
                        short: cf_short,
                        long: cf_long,
                    },
                    w2_ss_wages: w2_ss,
                    w2_medicare_wages: w2_medicare,
                    schedule_c_expenses: sce,
                };
                cmd::tax::set_profile(vault, &pp, year, profile)?;
                println!("Tax profile for {year} saved.");
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn dispatch_reconcile(
    vault: &std::path::Path,
    r: Reconcile,
    now: OffsetDateTime,
) -> Result<(), CliError> {
    let pp = passphrase(false)?;
    let id = match r {
        Reconcile::LinkTransfer {
            out,
            to_event,
            to_wallet,
        } => {
            let target = match (to_event, to_wallet) {
                (Some(ev), None) => TransferTarget::InEvent(eventref::parse_event_id(&ev)?),
                (None, Some(w)) => TransferTarget::Wallet(eventref::parse_wallet_id(&w)?),
                _ => {
                    return Err(CliError::Usage(
                        "exactly one of --to-event / --to-wallet required".into(),
                    ))
                }
            };
            cmd::reconcile::link_transfer(vault, &pp, &out, target, now)?
        }
        Reconcile::ClassifyInboundIncome {
            in_ref,
            kind,
            fmv,
            business,
        } => {
            let fmv = fmv.as_deref().map(eventref::parse_usd_arg).transpose()?;
            let class = InboundClass::Income {
                kind: eventref::parse_income_kind(&kind)?,
                fmv,
                business,
            };
            cmd::reconcile::classify_inbound(vault, &pp, &in_ref, class, now)?
        }
        Reconcile::ClassifyInboundGift {
            in_ref,
            fmv_at_gift,
            donor_basis,
            donor_acquired,
        } => {
            let class = InboundClass::GiftReceived {
                donor_basis: donor_basis
                    .as_deref()
                    .map(eventref::parse_usd_arg)
                    .transpose()?,
                donor_acquired_at: donor_acquired
                    .as_deref()
                    .map(eventref::parse_date_arg)
                    .transpose()?,
                fmv_at_gift: eventref::parse_usd_arg(&fmv_at_gift)?,
            };
            cmd::reconcile::classify_inbound(vault, &pp, &in_ref, class, now)?
        }
        Reconcile::ClassifyInboundSelfTransfer {
            in_ref,
            basis,
            acquired,
        } => {
            let class = InboundClass::SelfTransferMine {
                basis: basis.as_deref().map(eventref::parse_usd_arg).transpose()?,
                acquired_at: acquired
                    .as_deref()
                    .map(eventref::parse_date_arg)
                    .transpose()?,
            };
            cmd::reconcile::classify_inbound(vault, &pp, &in_ref, class, now)?
        }
        Reconcile::ReclassifyOutflow {
            out,
            as_kind,
            amount,
            fee,
            appraisal,
            donee,
        } => {
            let class = match as_kind {
                OutKindArg::Sell => OutflowClass::Dispose {
                    kind: DisposeKind::Sell,
                },
                OutKindArg::Spend => OutflowClass::Dispose {
                    kind: DisposeKind::Spend,
                },
                OutKindArg::Gift => OutflowClass::GiftOut,
                OutKindArg::Donate => OutflowClass::Donate {
                    appraisal_required: appraisal,
                },
            };
            let principal = eventref::parse_usd_arg(&amount)?;
            let fee = fee.as_deref().map(eventref::parse_usd_arg).transpose()?;
            cmd::reconcile::reclassify_outflow(vault, &pp, &out, class, principal, fee, donee, now)?
        }
        Reconcile::SetFmv { event, fmv } => {
            cmd::reconcile::set_fmv(vault, &pp, &event, eventref::parse_usd_arg(&fmv)?, now)?
        }
        Reconcile::Void { target } => cmd::reconcile::void(vault, &pp, &target, now)?,
        Reconcile::ClassifyRaw {
            target,
            payload_json,
        } => cmd::reconcile::classify_raw(vault, &pp, &target, &payload_json, now)?,
        Reconcile::AcceptConflict { conflict } => {
            cmd::reconcile::accept_conflict(vault, &pp, &conflict, now)?
        }
        Reconcile::RejectConflict { conflict } => {
            cmd::reconcile::reject_conflict(vault, &pp, &conflict, now)?
        }
        Reconcile::SafeHarborAllocate { method, attest } => {
            let m = match method {
                MethodArg::Actual => AllocMethod::ActualPosition,
                MethodArg::ProRata => AllocMethod::ProRata,
            };
            cmd::reconcile::safe_harbor_allocate(vault, &pp, m, attest, now)?
        }
        Reconcile::SafeHarborAttest => cmd::reconcile::safe_harbor_attest(vault, &pp, now)?,
        Reconcile::SelectLots { disposal, from } => {
            let picks = from
                .iter()
                .map(|s| eventref::parse_lot_pick(s))
                .collect::<Result<Vec<_>, _>>()?;
            cmd::reconcile::select_lots(vault, &pp, &disposal, picks, now)?
        }
        Reconcile::ImportSelections { csv } => {
            let ids = cmd::reconcile::import_selections(vault, &pp, &csv, now)?;
            // import-selections emits N decisions (one per disposal); print a summary and return
            // early (the trailing single-id println does not apply here).
            println!("Recorded {} LotSelection decision(s)", ids.len());
            return Ok(());
        }
        Reconcile::ReclassifyIncome {
            income_event,
            business,
            kind,
        } => {
            let kind = kind
                .as_deref()
                .map(eventref::parse_income_kind)
                .transpose()?;
            cmd::reconcile::reclassify_income(vault, &pp, &income_event, business, kind, now)?
        }
        Reconcile::SetDonationDetails {
            out_event_ref,
            donee_name,
            donee_address,
            donee_ein,
            appraiser_name,
            appraiser_address,
            appraiser_tin,
            appraiser_ptin,
            appraiser_qualifications,
            appraisal_date,
            fmv_method,
        } => {
            // Parse the optional appraisal date (YYYY-MM-DD) before building details.
            let appraisal_date = appraisal_date
                .as_deref()
                .map(eventref::parse_date_arg)
                .transpose()?;
            let details = DonationDetails {
                donee_name,
                donee_address,
                donee_ein,
                appraiser_name,
                appraiser_address,
                appraiser_tin,
                appraiser_ptin,
                appraiser_qualifications,
                appraisal_date,
                fmv_method_override: fmv_method,
            };
            cmd::reconcile::set_donation_details(vault, &pp, &out_event_ref, details)?;
            println!("Donation details saved for {out_event_ref}.");
            return Ok(());
        }
        Reconcile::ShowDonationDetails { out_event_ref } => {
            match cmd::reconcile::show_donation_details(vault, &pp, &out_event_ref)? {
                None => println!("none"),
                Some(d) => {
                    fn opt(v: Option<&str>) -> &str {
                        v.unwrap_or("none")
                    }
                    let appraisal_date_str = d
                        .appraisal_date
                        .map(|dt| dt.to_string())
                        .unwrap_or_else(|| "none".into());
                    println!(
                        "donee_name: {}\n\
                         donee_address: {}\n\
                         donee_ein: {}\n\
                         appraiser_name: {}\n\
                         appraiser_address: {}\n\
                         appraiser_tin: {}\n\
                         appraiser_ptin: {}\n\
                         appraiser_qualifications: {}\n\
                         appraisal_date: {}\n\
                         fmv_method_override: {}",
                        d.donee_name,
                        opt(d.donee_address.as_deref()),
                        opt(d.donee_ein.as_deref()),
                        d.appraiser_name,
                        opt(d.appraiser_address.as_deref()),
                        opt(d.appraiser_tin.as_deref()),
                        opt(d.appraiser_ptin.as_deref()),
                        opt(d.appraiser_qualifications.as_deref()),
                        appraisal_date_str,
                        opt(d.fmv_method_override.as_deref()),
                    );
                }
            }
            return Ok(());
        }
        Reconcile::BulkLinkTransfer {
            to_wallet,
            year,
            from,
            to,
            from_wallet,
            dry_run,
            yes,
        } => {
            let dest = eventref::parse_wallet_id(&to_wallet)?;
            let from_wallet = from_wallet
                .as_deref()
                .map(eventref::parse_wallet_id)
                .transpose()?;
            // clap enforces: none / --year alone / --from + --to. The catch-all is defensive.
            let frame = match (year, from, to) {
                (Some(y), None, None) => btctax_cli::Frame::Year(y),
                (None, Some(f), Some(t)) => btctax_cli::Frame::Range {
                    from: eventref::parse_date_arg(&f)?,
                    to: eventref::parse_date_arg(&t)?,
                },
                (None, None, None) => btctax_cli::Frame::All,
                _ => {
                    return Err(CliError::Usage(
                        "bulk-link-transfer: use --year, or --from with --to, or neither".into(),
                    ))
                }
            };
            let filter = btctax_cli::BulkFilter { frame, from_wallet };
            let plan = cmd::reconcile::bulk_link_plan(vault, &pp, filter, dest.clone())?;

            render_bulk_link_preview(&plan);

            if plan.included.is_empty() {
                println!("no pending outbound transfers match");
                return Ok(());
            }
            if dry_run {
                return Ok(());
            }
            let confirmed = if yes {
                true
            } else {
                print!(
                    "Link {} outflow(s) to {} as self-transfers? [y/N] ",
                    plan.included.len(),
                    render::wallet_label(&dest)
                );
                use std::io::Write;
                std::io::stdout().flush().ok();
                let mut line = String::new();
                std::io::stdin().read_line(&mut line)?;
                matches!(line.trim(), "y" | "Y" | "yes" | "YES")
            };
            if !confirmed {
                println!("aborted; nothing written");
                return Ok(());
            }
            let out_events: Vec<_> = plan.included.iter().map(|r| r.out_event.clone()).collect();
            let n = cmd::reconcile::apply_bulk_link_transfer(
                vault,
                &pp,
                out_events,
                dest.clone(),
                now,
            )?;
            println!(
                "linked {n} outflows to {}; {} skipped (same wallet)",
                render::wallet_label(&dest),
                plan.skipped_same_wallet.len()
            );
            return Ok(());
        }
        Reconcile::BulkClassifyInboundSelfTransfer {
            year,
            from,
            to,
            wallet,
            dry_run,
            yes,
        } => {
            let wallet = wallet
                .as_deref()
                .map(eventref::parse_wallet_id)
                .transpose()?;
            // clap enforces: none / --year alone / --from + --to. The catch-all is defensive.
            let frame = match (year, from, to) {
                (Some(y), None, None) => btctax_cli::Frame::Year(y),
                (None, Some(f), Some(t)) => btctax_cli::Frame::Range {
                    from: eventref::parse_date_arg(&f)?,
                    to: eventref::parse_date_arg(&t)?,
                },
                (None, None, None) => btctax_cli::Frame::All,
                _ => {
                    return Err(CliError::Usage(
                        "bulk-classify-inbound-self-transfer: use --year, or --from with --to, or neither"
                            .into(),
                    ))
                }
            };
            let filter = btctax_cli::BulkStiFilter { frame, wallet };
            let plan = cmd::reconcile::bulk_self_transfer_in_plan(vault, &pp, filter)?;

            render_bulk_sti_preview(&plan);

            if plan.included.is_empty() {
                println!("no unclassified inbound deposits match");
                return Ok(());
            }
            if dry_run {
                return Ok(());
            }
            let confirmed = if yes {
                true
            } else {
                print!(
                    "Classify {} inbound deposit(s) as self-transfer-in ($0 basis)? [y/N] ",
                    plan.included.len()
                );
                use std::io::Write;
                std::io::stdout().flush().ok();
                let mut line = String::new();
                std::io::stdin().read_line(&mut line)?;
                matches!(line.trim(), "y" | "Y" | "yes" | "YES")
            };
            if !confirmed {
                println!("aborted; nothing written");
                return Ok(());
            }
            let in_events: Vec<_> = plan.included.iter().map(|r| r.in_event.clone()).collect();
            let n = cmd::reconcile::apply_bulk_self_transfer_in(vault, &pp, in_events, now)?;
            println!("classified {n} inbound deposits as self-transfer-in ($0 basis)");
            return Ok(());
        }
        Reconcile::BulkResolveConflict {
            accept,
            reject: _, // clap's ArgGroup guarantees EXACTLY one of --accept/--reject; dispatch on `accept`.
            dry_run,
            yes,
        } => {
            // clap's ArgGroup guarantees EXACTLY one of --accept / --reject; the else is defensive.
            let plan = cmd::reconcile::bulk_resolve_conflict_plan(vault, &pp)?;
            render_bulk_resolve_preview(&plan, accept);

            if plan.rows.is_empty() {
                println!("no unresolved import conflicts");
                return Ok(());
            }
            if dry_run {
                return Ok(());
            }
            let action = if accept { "Accept" } else { "Reject" };
            let confirmed = if yes {
                true
            } else {
                print!("{action} {} import conflict(s)? [y/N] ", plan.rows.len());
                use std::io::Write;
                std::io::stdout().flush().ok();
                let mut line = String::new();
                std::io::stdin().read_line(&mut line)?;
                matches!(line.trim(), "y" | "Y" | "yes" | "YES")
            };
            if !confirmed {
                println!("aborted; nothing written");
                return Ok(());
            }
            let conflict_events: Vec<_> =
                plan.rows.iter().map(|r| r.conflict_event.clone()).collect();
            if accept {
                let n =
                    cmd::reconcile::apply_bulk_accept_conflicts(vault, &pp, conflict_events, now)?;
                println!("accepted {n} import conflicts");
            } else {
                let n =
                    cmd::reconcile::apply_bulk_reject_conflicts(vault, &pp, conflict_events, now)?;
                println!("rejected {n} import conflicts");
            }
            return Ok(());
        }
        Reconcile::BulkVoid { dry_run, yes } => {
            let plan = cmd::reconcile::bulk_void_plan(vault, &pp)?;
            render_bulk_void_preview(&plan);

            if plan.rows.is_empty() {
                println!("no revocable decisions to void");
                return Ok(());
            }
            if dry_run {
                return Ok(());
            }
            let confirmed = if yes {
                true
            } else {
                print!(
                    "Void {} decision(s)? THESE VOIDS CANNOT THEMSELVES BE UNDONE [y/N] ",
                    plan.rows.len()
                );
                use std::io::Write;
                std::io::stdout().flush().ok();
                let mut line = String::new();
                std::io::stdin().read_line(&mut line)?;
                matches!(line.trim(), "y" | "Y" | "yes" | "YES")
            };
            if !confirmed {
                println!("aborted; nothing written");
                return Ok(());
            }
            // [R0-M3] targets are derived from the predicate-filtered plan rows — NEVER raw ids — so an
            // effective allocation (omitted by the plan) can never reach `apply_bulk_void`.
            let targets: Vec<_> = plan
                .rows
                .iter()
                .map(|r| (r.target_event_id.clone(), r.disposal_to_clear.clone()))
                .collect();
            let n = cmd::reconcile::apply_bulk_void(vault, &pp, targets, now)?;
            println!("voided {n} decisions");
            return Ok(());
        }
        Reconcile::MatchSelfTransfers {
            in_ref,
            out_ref,
            action,
            dry_run,
        } => {
            let proposals = cmd::reconcile::self_transfer_match_plan(vault, &pp)?;
            render_self_transfer_matches(&proposals);

            // Phase 1 only: preview (no refs) or explicit --dry-run.
            let (Some(in_ref), Some(out_ref)) = (in_ref, out_ref) else {
                if proposals.is_empty() {
                    println!("no self-transfer matches proposed");
                }
                return Ok(());
            };
            if dry_run {
                return Ok(());
            }
            // Phase 2: confirm ONE pair. The action is the explicit override, else the proposal's
            // topology-derived suggestion; an unproposed pair requires an explicit --action.
            let in_id = eventref::parse_event_id(&in_ref)?;
            let out_id = eventref::parse_event_id(&out_ref)?;
            let suggested = proposals
                .iter()
                .find(|p| p.in_event == in_id && p.out_event == out_id)
                .map(|p| p.action);
            let resolved =
                match (action, suggested) {
                    (Some(SelfTransferActionArg::Drop), _) => btctax_cli::MatchAction::Drop,
                    (Some(SelfTransferActionArg::Relocate), _) => btctax_cli::MatchAction::Relocate,
                    (None, Some(a)) => a,
                    (None, None) => return Err(CliError::Usage(
                        "that in/out pair is not a proposed match; pass --action drop|relocate \
                         to confirm it explicitly"
                            .into(),
                    )),
                };
            match resolved {
                btctax_cli::MatchAction::Drop => {
                    // DROP: append one SelfTransferPassthrough (both legs → Op::Skip).
                    let id = cmd::reconcile::apply_self_transfer_passthrough(
                        vault, &pp, &in_ref, &out_ref, now,
                    )?;
                    println!(
                        "dropped self-transfer passthrough (in {} + out {}); decision {}",
                        in_id.canonical(),
                        out_id.canonical(),
                        id.canonical()
                    );
                }
                btctax_cli::MatchAction::Relocate => {
                    // RELOCATE routes to the EXISTING link_transfer out→in (G-RELOCATE-REUSE).
                    let id = cmd::reconcile::link_transfer(
                        vault,
                        &pp,
                        &out_ref,
                        TransferTarget::InEvent(in_id.clone()),
                        now,
                    )?;
                    println!(
                        "relocated self-transfer (out {} → in {}); decision {}",
                        out_id.canonical(),
                        in_id.canonical(),
                        id.canonical()
                    );
                }
            }
            return Ok(());
        }
    };
    println!("Recorded decision {}", id.canonical());
    Ok(())
}

/// Render the self-transfer match proposals (self-transfer-passthrough C3, Phase 1). Read-only preview:
/// each pair's two `EventId`s, the two dates + wallets + sats, the advisory USD value, the suggested
/// action (DROP vs RELOCATE), and the `ambiguous` flag (surfaced, NEVER auto-picked).
fn render_self_transfer_matches(proposals: &[btctax_cli::MatchProposal]) {
    if proposals.is_empty() {
        return;
    }
    println!("Self-transfer match proposals (confirm one with --in <in> --out <out>):");
    for p in proposals {
        let in_w = p
            .in_wallet
            .as_ref()
            .map(render::wallet_label)
            .unwrap_or_else(|| "(no wallet)".to_string());
        let out_w = p
            .out_wallet
            .as_ref()
            .map(render::wallet_label)
            .unwrap_or_else(|| "(no wallet)".to_string());
        let usd = match p.usd_value {
            Some(v) => format!("${v}"),
            None => "\u{2014}".to_string(),
        };
        let action = match p.action {
            btctax_cli::MatchAction::Drop => "DROP",
            btctax_cli::MatchAction::Relocate => "RELOCATE",
        };
        let flags = {
            let mut s = String::new();
            if p.ambiguous {
                s.push_str(" [AMBIGUOUS]");
            }
            if p.txid_match {
                s.push_str(" [txid-match]");
            }
            s
        };
        println!(
            "  {action}{flags}  in {} ({}, {}, {} sat)  out {} ({}, {}, {} sat)  {usd}",
            p.in_event.canonical(),
            p.in_date,
            in_w,
            p.in_sat,
            p.out_event.canonical(),
            p.out_date,
            out_w,
            p.out_principal_sat,
        );
    }
}

/// Render the bulk link-transfer preview table + totals footer (bulk-link-transfer D2). The USD
/// total is the HONEST FLOOR [R0-I2]: exact `$X` when every included row has a price, else
/// `≥ $X (N unavailable)`.
fn render_bulk_link_preview(plan: &btctax_cli::BulkLinkPlan) {
    println!(
        "Bulk self-transfer preview → {}",
        render::wallet_label(&plan.dest)
    );
    println!(
        "{:<12}  {:<28}  {:>14}  {:>16}",
        "date", "source wallet", "sat", "USD value"
    );
    for r in &plan.included {
        let wallet = r
            .source_wallet
            .as_ref()
            .map(render::wallet_label)
            .unwrap_or_else(|| "(no wallet)".to_string());
        let usd = match r.usd_value {
            Some(v) => format!("${v}"),
            None => "—".to_string(),
        };
        println!(
            "{:<12}  {:<28}  {:>14}  {:>16}",
            r.date, wallet, r.principal_sat, usd
        );
    }
    let total = if plan.missing_price_count == 0 {
        format!("${}", plan.total_usd_value_floor)
    } else {
        format!(
            "\u{2265} ${} ({} unavailable)",
            plan.total_usd_value_floor, plan.missing_price_count
        )
    };
    println!(
        "included {} | {} sat | total USD reclassified non-taxable {} | skipped (same wallet) {}",
        plan.included.len(),
        plan.total_sat,
        total,
        plan.skipped_same_wallet.len()
    );
}

/// Render the bulk classify-inbound-self-transfer preview table + totals footer
/// (bulk-classify-inbound-self-transfer D2). The USD total is the HONEST FLOOR: exact `$X` when every
/// included row has a price, else `≥ $X (N unavailable)` — the market value being GIVEN $0 basis (the
/// deliberate over-tax exposure the user is accepting).
fn render_bulk_sti_preview(plan: &btctax_cli::BulkStiPlan) {
    println!("Bulk classify-inbound-self-transfer preview ($0 conservative basis, non-taxable)");
    println!(
        "{:<12}  {:<28}  {:>14}  {:>16}",
        "date", "receiving wallet", "sat", "USD FMV"
    );
    for r in &plan.included {
        let wallet = r
            .wallet
            .as_ref()
            .map(render::wallet_label)
            .unwrap_or_else(|| "(no wallet)".to_string());
        let usd = match r.usd_fmv {
            Some(v) => format!("${v}"),
            None => "—".to_string(),
        };
        println!("{:<12}  {:<28}  {:>14}  {:>16}", r.date, wallet, r.sat, usd);
    }
    let total = if plan.missing_price_count == 0 {
        format!("${}", plan.total_usd_fmv_floor)
    } else {
        format!(
            "\u{2265} ${} ({} unavailable)",
            plan.total_usd_fmv_floor, plan.missing_price_count
        )
    };
    println!(
        "included {} | {} sat | total USD reclassified to $0 basis (you'll be conservatively over-taxed on this later) {}",
        plan.included.len(),
        plan.total_sat,
        total
    );
}

/// One-line human summary of an imported payload for the bulk resolve-conflict preview. The CLI
/// front-end's own summary formatter [R0-M1] — `import_payload_summary` is a private tui-edit binary
/// fn unreachable from btctax-cli, so the row carries the STRUCTURED `EventPayload` and each front-end
/// renders it. Covers the common imported variants; anything else falls back to a compact debug form.
fn bulk_resolve_payload_summary(p: &btctax_core::EventPayload) -> String {
    use btctax_core::EventPayload;
    match p {
        EventPayload::Acquire(a) => format!("Acquire {} sat, cost {}", a.sat, a.usd_cost),
        EventPayload::Income(i) => {
            let fmv = i
                .usd_fmv
                .map(|v| v.to_string())
                .unwrap_or_else(|| "(no fmv)".to_string());
            format!("Income {} sat @ {}", i.sat, fmv)
        }
        EventPayload::Dispose(d) => format!("Dispose {} sat, proceeds {}", d.sat, d.usd_proceeds),
        EventPayload::TransferIn(t) => format!("TransferIn {} sat", t.sat),
        EventPayload::TransferOut(t) => format!("TransferOut {} sat", t.sat),
        EventPayload::Unclassified(u) => {
            format!(
                "Unclassified: {}",
                u.raw.chars().take(40).collect::<String>()
            )
        }
        other => format!("{other:?}"),
    }
}

/// Render the bulk resolve-conflict preview: a `current → new` table (bulk-resolve-conflict D2). `accept`
/// selects the action shown in the banner (ACCEPT adopts new / REJECT keeps current). No $ number — a
/// conflict resolution recognizes no gain.
fn render_bulk_resolve_preview(plan: &btctax_cli::BulkResolvePlan, accept: bool) {
    let action = if accept {
        "ACCEPT (adopt new)"
    } else {
        "REJECT (keep current)"
    };
    println!("Bulk resolve-conflict preview — action: {action} (NON-REVOCABLE)");
    println!(
        "{:<12}  {:<26}  {:<10}  {:<34}  {:<34}",
        "date", "target", "new-fp", "current", "→ new"
    );
    for r in &plan.rows {
        println!(
            "{:<12}  {:<26}  {:<10}  {:<34}  {:<34}",
            r.date,
            r.target.canonical(),
            r.new_fingerprint,
            bulk_resolve_payload_summary(&r.current_payload),
            bulk_resolve_payload_summary(&r.new_payload),
        );
    }
    println!("conflicts {}", plan.rows.len());
}

/// One-line human summary of a voidable decision for the bulk-void preview (the CLI front-end's own
/// summary formatter — `summarize_void_payload` is a private tui-edit binary fn). Shows the payload tag
/// + the inner target the void undoes.
fn bulk_void_payload_summary(p: &btctax_core::EventPayload) -> String {
    use btctax_core::EventPayload;
    match p {
        EventPayload::TransferLink(tl) => {
            format!("TransferLink out {}", tl.out_event.canonical())
        }
        EventPayload::ReclassifyOutflow(ro) => format!(
            "ReclassifyOutflow out {} as {:?}",
            ro.transfer_out_event.canonical(),
            ro.as_
        ),
        EventPayload::ClassifyInbound(ci) => format!(
            "ClassifyInbound in {} as {:?}",
            ci.transfer_in_event.canonical(),
            ci.as_
        ),
        EventPayload::ManualFmv(m) => {
            format!("ManualFmv {} for {}", m.usd_fmv, m.event.canonical())
        }
        EventPayload::ClassifyRaw(cr) => format!("ClassifyRaw {}", cr.target.canonical()),
        EventPayload::MethodElection(me) => {
            format!("MethodElection {:?} from {}", me.method, me.effective_from)
        }
        EventPayload::LotSelection(ls) => {
            format!("LotSelection lots for {}", ls.disposal_event.canonical())
        }
        EventPayload::ReclassifyIncome(ri) => format!(
            "ReclassifyIncome {} biz={}",
            ri.income_event.canonical(),
            ri.business
        ),
        EventPayload::SelfTransferPassthrough(stp) => format!(
            "SelfTransferPassthrough in {} out {}",
            stp.in_event.canonical(),
            stp.out_event.canonical()
        ),
        EventPayload::SafeHarborAllocation(a) => {
            format!(
                "SafeHarborAllocation {} lots as_of {}",
                a.lots.len(),
                a.as_of_date
            )
        }
        other => format!("{other:?}"),
    }
}

/// Render the bulk-void preview: a `seq · date · decision` table (bulk-void D2). Flags the count of
/// `LotSelection` voids that re-expose disposals + clear attestations (the blast radius). No $ number.
fn render_bulk_void_preview(plan: &btctax_cli::BulkVoidPlan) {
    println!("Bulk-void preview — these voids CANNOT themselves be undone (NON-REVOCABLE)");
    println!(
        "{:<10}  {:<12}  {:<60}",
        "seq", "date", "decision (what the void undoes)"
    );
    for r in &plan.rows {
        println!(
            "{:<10}  {:<12}  {:<60}",
            r.seq,
            r.date,
            bulk_void_payload_summary(&r.payload),
        );
    }
    let lot_selections = plan
        .rows
        .iter()
        .filter(|r| r.disposal_to_clear.is_some())
        .count();
    println!(
        "voidable {} ({} LotSelection void(s) re-expose disposals + clear attestations)",
        plan.rows.len(),
        lot_selections
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parse a `reconcile bulk-resolve-conflict …` invocation via the real clap derivation.
    fn parse_brc(args: &[&str]) -> Result<Cli, clap::Error> {
        let mut full = vec!["btctax", "reconcile", "bulk-resolve-conflict"];
        full.extend_from_slice(args);
        Cli::try_parse_from(full)
    }

    /// The clap ArgGroup makes `--accept | --reject` REQUIRED and MUTUALLY EXCLUSIVE [R0-r2]:
    /// neither fails, both fail, exactly one parses. (No `ResolveKind` in the CLI — a batch-wide bool.)
    #[test]
    fn bulk_resolve_cli_requires_accept_xor_reject() {
        assert!(
            parse_brc(&[]).is_err(),
            "neither --accept nor --reject must fail (group required)"
        );
        assert!(
            parse_brc(&["--accept", "--reject"]).is_err(),
            "both must fail (group mutually exclusive)"
        );
        assert!(parse_brc(&["--accept"]).is_ok(), "--accept alone parses");
        assert!(parse_brc(&["--reject"]).is_ok(), "--reject alone parses");
        assert!(
            parse_brc(&["--reject", "--dry-run"]).is_ok(),
            "--reject --dry-run parses"
        );
    }

    /// Parse a `reconcile bulk-void …` invocation via the real clap derivation.
    fn parse_bv(args: &[&str]) -> Result<Cli, clap::Error> {
        let mut full = vec!["btctax", "reconcile", "bulk-void"];
        full.extend_from_slice(args);
        Cli::try_parse_from(full)
    }

    /// bulk-void takes no --accept/--reject (void is single-valued); the two-phase --dry-run / --yes
    /// flags parse, and there is no required arg group.
    #[test]
    fn bulk_void_cli_flags_parse() {
        assert!(parse_bv(&[]).is_ok(), "bare bulk-void parses (interactive)");
        assert!(parse_bv(&["--dry-run"]).is_ok(), "--dry-run parses");
        assert!(parse_bv(&["--yes"]).is_ok(), "--yes parses");
        assert!(
            parse_bv(&["--accept"]).is_err(),
            "bulk-void has no --accept flag"
        );
    }
}
