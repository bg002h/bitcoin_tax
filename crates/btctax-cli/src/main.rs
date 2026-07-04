//! btctax — thin clap-4 dispatch over the btctax_cli library. Resolves the passphrase (env seam for
//! non-interactive use; otherwise a secure prompt), calls one library command, renders, and sets the
//! exit code (non-zero on FR9 hard blockers / on any CliError). NO business logic lives here.
use btctax_cli::cli::{
    Cli, Command, FeeArg, MethodArg, Optimize, OutKindArg, Reconcile, SelfTransferActionArg,
};
use btctax_cli::{cmd, eventref, render, CliError};
use btctax_core::{
    AllocMethod, Carryforward, DisposeKind, DonationDetails, FeeTreatment, FilingStatus,
    InboundClass, IncomeKind, OutflowClass, TaxProfile, TransferTarget,
};
use btctax_store::Passphrase;
use clap::Parser;
use std::process::ExitCode;
use time::{OffsetDateTime, UtcOffset};

fn main() -> ExitCode {
    // Windows' default main-thread stack is 1 MiB, and some ledger-fold code paths have large stack
    // frames (especially in debug builds) that exceed it → a hard STATUS_STACK_OVERFLOW crash (empty
    // stderr, exit 0xC00000FD), observed on windows-latest CI in the classify-inbound-self-transfer
    // flow. Linux/macOS default to 8 MiB and are unaffected. Run the CLI on a worker thread with an
    // explicit, generous stack so behavior is identical on every platform — the same approach
    // rustc/cargo take (RUST_MIN_STACK). The 64 MiB reservation is virtual (not committed until
    // touched), so it is effectively free where it is not needed.
    let worker = std::thread::Builder::new()
        .name("btctax-main".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(run_to_exit)
        .expect("spawn btctax worker thread");
    // The worker's own default panic hook prints any panic to stderr before unwinding; a join Err
    // means it panicked, so surface the generic error exit code (2), matching the Err(e) arm below.
    worker.join().unwrap_or(ExitCode::from(2))
}

/// Run the CLI and map its result to a process exit code. Extracted from `main` so it executes on the
/// large-stack worker thread `main` spawns (see the stack-size rationale there).
fn run_to_exit() -> ExitCode {
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
        Reconcile::BulkClassifyInboundIncome {
            kind,
            business,
            year,
            from,
            to,
            wallet,
            dry_run,
            yes,
        } => {
            let kind = eventref::parse_income_kind(&kind)?;
            let wallet = wallet
                .as_deref()
                .map(eventref::parse_wallet_id)
                .transpose()?;
            // clap enforces: none / --year alone / --from + --to. The catch-all is defensive.
            let frame =
                match (year, from, to) {
                    (Some(y), None, None) => btctax_cli::Frame::Year(y),
                    (None, Some(f), Some(t)) => btctax_cli::Frame::Range {
                        from: eventref::parse_date_arg(&f)?,
                        to: eventref::parse_date_arg(&t)?,
                    },
                    (None, None, None) => btctax_cli::Frame::All,
                    _ => return Err(CliError::Usage(
                        "bulk-classify-inbound-income: use --year, or --from with --to, or neither"
                            .into(),
                    )),
                };
            let filter = btctax_cli::BulkIncomeFilter { frame, wallet };
            let plan = cmd::reconcile::bulk_classify_income_plan(vault, &pp, filter)?;

            render_bulk_income_preview(&plan, kind, business);

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
                    "Recognize {} inbound deposit(s) as {} income (total ${})? [y/N] ",
                    plan.included.len(),
                    kind_label(kind),
                    plan.total_income_usd
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
            let n = cmd::reconcile::apply_bulk_classify_inbound_income(
                vault, &pp, in_events, kind, business, now,
            )?;
            println!(
                "recognized {n} inbound deposit(s) as {} income (total ${})",
                kind_label(kind),
                plan.total_income_usd
            );
            return Ok(());
        }
        Reconcile::BulkReclassifyOutflow {
            kind,
            year,
            from,
            to,
            wallet,
            dry_run,
            yes,
        } => {
            // Scope-lock: sell|spend only; gift/donate rejected structurally [Q2].
            let kind = eventref::parse_dispose_kind(&kind)?;
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
                        "bulk-reclassify-outflow: use --year, or --from with --to, or neither"
                            .into(),
                    ))
                }
            };
            let filter = btctax_cli::BulkFilter {
                frame,
                from_wallet: wallet,
            };
            let plan = cmd::reconcile::bulk_reclassify_outflow_plan(vault, &pp, filter)?;

            render_bulk_reclassify_outflow_preview(&plan, kind);

            if plan.included.is_empty() {
                println!("no pending outflows match");
                return Ok(());
            }
            if dry_run {
                return Ok(());
            }
            let confirmed = if yes {
                true
            } else {
                print!(
                    "Reclassify {} pending outflow(s) as {} with ESTIMATED proceeds ${} \
                     (ESTIMATED gain ${})? [y/N] ",
                    plan.included.len(),
                    dispose_kind_label(kind),
                    plan.total_proceeds_usd,
                    plan.total_estimated_gain
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
            // Dispatch derives out_events from `plan.included` (never raw --ref) — the #a exclusion + the
            // estimate must not be bypassable.
            let out_events: Vec<_> = plan.included.iter().map(|r| r.out_event.clone()).collect();
            let n =
                cmd::reconcile::apply_bulk_reclassify_outflow(vault, &pp, out_events, kind, now)?;
            println!(
                "reclassified {n} pending outflow(s) as {} with ESTIMATED proceeds ${} \
                 (ESTIMATED gain ${})",
                dispose_kind_label(kind),
                plan.total_proceeds_usd,
                plan.total_estimated_gain
            );
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

/// Lower-case CLI label for an `IncomeKind` (matches `parse_income_kind`'s accepted spellings).
fn kind_label(kind: IncomeKind) -> &'static str {
    match kind {
        IncomeKind::Mining => "mining",
        IncomeKind::Staking => "staking",
        IncomeKind::Interest => "interest",
        IncomeKind::Airdrop => "airdrop",
        IncomeKind::Reward => "reward",
    }
}

fn dispose_kind_label(kind: DisposeKind) -> &'static str {
    match kind {
        DisposeKind::Sell => "sell",
        DisposeKind::Spend => "spend",
    }
}

/// Preview for `bulk-reclassify-outflow`: the included rows (each auto-valued at its outflow-date FMV =
/// the ESTIMATED proceeds), the total ESTIMATED proceeds AND the total ESTIMATED gain (Σ fmv − Σ basis),
/// and the count of outflows EXCLUDED because their date has no bundled price (those stay pending — a
/// Sell with fabricated proceeds would SILENTLY misreport gain/loss). The word "ESTIMATED" is printed
/// adjacent to BOTH the proceeds and the gain totals.
fn render_bulk_reclassify_outflow_preview(
    plan: &btctax_cli::BulkReclassifyOutflowPlan,
    kind: DisposeKind,
) {
    println!(
        "Bulk reclassify-outflow preview ({}, auto-FMV at outflow date = ESTIMATED proceeds)",
        dispose_kind_label(kind)
    );
    println!(
        "{:<12}  {:>14}  {:>18}  {:>16}  {:>16}",
        "date", "sat", "est. proceeds USD", "basis USD", "est. gain USD"
    );
    for r in &plan.included {
        println!(
            "{:<12}  {:>14}  {:>18}  {:>16}  {:>16}",
            r.date,
            r.principal_sat,
            format!("${}", r.fmv),
            format!("${}", r.basis_usd),
            format!("${}", r.estimated_gain),
        );
    }
    println!(
        "included {} | {} sat | total ESTIMATED proceeds ${} | total basis ${} | total ESTIMATED gain ${}",
        plan.included.len(),
        plan.total_sat,
        plan.total_proceeds_usd,
        plan.total_basis_usd,
        plan.total_estimated_gain,
    );
    if plan.excluded_missing_price > 0 {
        println!(
            "excluded {} outflow(s) with no available price (still pending — a disposition with no \
             FMV cannot be auto-valued; set it single-item with `reclassify-outflow` instead)",
            plan.excluded_missing_price
        );
    }
}

/// Preview for `bulk-classify-inbound-income`: the included rows (each auto-valued at its receipt-date
/// FMV = the income recognized), the total income being recognized, and the count of inbounds EXCLUDED
/// because their date has no bundled price (those stay pending — an income row with no FMV would raise a
/// Hard `FmvMissing` year-gate, so they are surfaced here, NOT silently dropped).
fn render_bulk_income_preview(plan: &btctax_cli::BulkIncomePlan, kind: IncomeKind, business: bool) {
    println!(
        "Bulk classify-inbound-income preview ({} income{}, auto-FMV at receipt)",
        kind_label(kind),
        if business { ", business" } else { "" }
    );
    println!("{:<12}  {:>14}  {:>16}", "date", "sat", "income USD");
    for r in &plan.included {
        println!(
            "{:<12}  {:>14}  {:>16}",
            r.date,
            r.sat,
            format!("${}", r.fmv)
        );
    }
    println!(
        "included {} | {} sat | total income recognized ${}",
        plan.included.len(),
        plan.total_sat,
        plan.total_income_usd
    );
    if plan.excluded_missing_price > 0 {
        println!(
            "excluded {} inbound(s) with no available price (still pending — set FMV manually or \
             classify as self-transfer instead)",
            plan.excluded_missing_price
        );
    }
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
