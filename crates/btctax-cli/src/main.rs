//! btctax — thin clap-4 dispatch over the btctax_cli library. Resolves the passphrase (env seam for
//! non-interactive use; otherwise a secure prompt), calls one library command, renders, and sets the
//! exit code (non-zero on FR9 hard blockers / on any CliError). NO business logic lives here.
use btctax_cli::{cmd, eventref, render, CliError};
use btctax_core::{
    AllocMethod, DisposeKind, FeeTreatment, InboundClass, OutflowClass, TransferTarget,
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
}

#[derive(Copy, Clone, ValueEnum)]
enum FeeArg {
    C,
    B,
}

#[derive(Copy, Clone, ValueEnum)]
enum OutKindArg {
    Sell,
    Spend,
    Gift,
    Donate,
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
        Command::Config { set_fee_treatment } => {
            let pp = passphrase(false)?;
            let cfg = match set_fee_treatment {
                Some(FeeArg::C) => {
                    cmd::admin::set_config(vault, &pp, Some(FeeTreatment::TreatmentC))?
                }
                Some(FeeArg::B) => {
                    cmd::admin::set_config(vault, &pp, Some(FeeTreatment::TreatmentB))?
                }
                None => cmd::admin::show_config(vault, &pp)?,
            };
            println!(
                "fee_treatment: {:?}\nlot_method: {:?}",
                cfg.fee_treatment, cfg.lot_method
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
    };
    println!("Recorded decision {}", id.canonical());
    Ok(())
}
