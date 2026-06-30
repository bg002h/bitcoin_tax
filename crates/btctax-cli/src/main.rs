//! btctax — thin clap-4 dispatch over the btctax_cli library. Resolves the passphrase (env seam for
//! non-interactive use; otherwise a secure prompt), calls one library command, renders, and sets the
//! exit code (non-zero on FR9 hard blockers / on any CliError). NO business logic lives here.
use btctax_cli::{cmd, eventref, render, CliError};
use btctax_core::{
    AllocMethod, Carryforward, DisposeKind, FeeTreatment, FilingStatus, InboundClass, LotMethod,
    OutflowClass, TaxProfile, TransferTarget,
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
        /// Clear an interrupted/half-created init (orphan `vault.key`, no encrypted store) and start fresh.
        #[arg(long, default_value_t = false)]
        repair: bool,
    },
    /// Import one or more export files (auto-groups Swan).
    Import { files: Vec<PathBuf> },
    /// FR9 integrity check (non-zero exit on hard blockers).
    Verify,
    /// Show holdings + realized disposals/removals/income.
    #[command(alias = "show")]
    Report {
        #[arg(long)]
        year: Option<i32>,
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
    },
    /// Export the passphrase-protected key.
    BackupKey {
        #[arg(long)]
        out: PathBuf,
    },
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
        /// Show the stored profile for `--year` instead of setting it.
        #[arg(long, default_value_t = false)]
        show: bool,
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
        Command::Report { year } => {
            let state = cmd::inspect::report(vault, &passphrase(false)?, year)?;
            print!("{}", render::render_report(&state, year));
        }
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
        Command::ExportSnapshot { out } => {
            let p = cmd::admin::export_snapshot(vault, &passphrase(false)?, &out)?;
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
            show,
        } => {
            let pp = passphrase(false)?;
            if show {
                match cmd::tax::show_profile(vault, &pp, year)? {
                    Some(p) => println!(
                        "year: {year}\n\
                         filing_status: {:?}\n\
                         ordinary_taxable_income: {}\n\
                         magi_excluding_crypto: {}\n\
                         qualified_dividends_and_other_pref_income: {}\n\
                         other_net_capital_gain: {}\n\
                         capital_loss_carryforward_in.short: {}\n\
                         capital_loss_carryforward_in.long: {}",
                        p.filing_status,
                        p.ordinary_taxable_income,
                        p.magi_excluding_crypto,
                        p.qualified_dividends_and_other_pref_income,
                        p.other_net_capital_gain,
                        p.capital_loss_carryforward_in.short,
                        p.capital_loss_carryforward_in.long,
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
        Reconcile::ReclassifyOutflow {
            out,
            as_kind,
            amount,
            fee,
            appraisal,
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
            cmd::reconcile::reclassify_outflow(vault, &pp, &out, class, principal, fee, now)?
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
    };
    println!("Recorded decision {}", id.canonical());
    Ok(())
}
