//! Text rendering of CLI outputs (FR9 verify, FR4 report/show) + FR10 CSV export. Pure string-building
//! over engine data — the CLI displays; the engine computes (NFR4/NFR5).
use crate::config::CliConfig;
use btctax_adapters::FileReport;
use btctax_core::conventions::{tax_date, Sat, Usd, TRANSITION_DATE};
use btctax_core::persistence::ImportReport;
use btctax_core::DonationDetails;
use btctax_core::{
    conservation_report, disposal_compliance, form_8283, form_8949, schedule_d,
    year_donation_deduction, BasisSource, Blocker, BlockerKind, ComplianceStatus,
    ConservationReport, DisposalCompliance, DisposalLeg, DisposeKind, EventId, EventPayload,
    Form8283HowAcquired, Form8283Section, Form8949Box, Form8949Part, GiftZone, HarvestReport,
    HarvestStatus, HarvestTarget, InboundClass, IncomeKind, LedgerEvent, LedgerState, LotMethod,
    LtcgBracket, OutflowClass, RemovalKind, RemovalLeg, ScheduleDTotals, SeTaxResult, SellReport,
    SellStatus, Severity, TaxDate, Term, WalletId,
};
use btctax_store::fsperms;
use csv::Writer;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::Path;

// ── Money formatting helper ──────────────────────────────────────────────────────────────────────

/// Format any `Decimal` money value as exactly 2 decimal places (e.g. "0.00", "1747.50").
///
/// Load-bearing figures (`ltcg_tax`, `niit`, `total_federal_tax_attributable`) are always
/// `round_cents`-scaled (scale 2) so they already print with cents. Descriptive level fields
/// (`st_net`, `lt_net`, `carryforward`, `loss_deduction`, etc.) inherit the source `Decimal`
/// scale and may print as "7000" or "0" without explicit 2dp formatting. This helper ensures
/// every dollar figure in the tax report renders consistently with 2 decimal places.
///
/// **Equality is unaffected** — this is display only. The underlying `Decimal` value is
/// unchanged; only the `Display` string gains the forced 2dp format.
fn fmt_money(d: btctax_core::conventions::Usd) -> String {
    format!("{d:.2}")
}

// ── Stable CSV/display tags for core enums ──────────────────────────────────────────────────────
// These are free functions (not inherent methods) because the CLI crate cannot add methods to
// core types. Values are human-readable and STABLE — changing them breaks the CSV contract.

fn basis_source_tag(bs: BasisSource) -> &'static str {
    match bs {
        BasisSource::ExchangeProvided => "exchange",
        BasisSource::ComputedFromCost => "cost",
        BasisSource::FmvAtIncome => "income_fmv",
        BasisSource::CarriedFromTransfer => "transferred",
        BasisSource::GiftCarryover => "gift_carryover",
        BasisSource::GiftFmvFallback => "gift_fmv_fallback",
        BasisSource::SafeHarborAllocated => "safe_harbor",
        BasisSource::ReconstructedPerWallet => "reconstructed",
        BasisSource::SelfTransferInbound => "self_transfer_in",
        BasisSource::EstimatedConservative => "estimated_conservative",
    }
}

/// Pseudo-reconcile (sub-project 2, [R0-I4]): the ON-SCREEN-ONLY `[PSEUDO]` marker for a row whose
/// existence or basis traces to a synthetic (non-persisted) default. Driven by the DEDICATED `pseudo`
/// bool on `Lot`/`DisposalLeg`/`RemovalLeg` — NEVER a `BasisSource` variant (which the CSV writers emit
/// via `basis_source_tag`, and would LEAK "PSEUDO" into lots.csv). The CSV/form writers OMIT this marker
/// entirely (they never call this helper), so it can never reach any export file (the ★ headline guard).
fn pseudo_tag(pseudo: bool) -> &'static str {
    if pseudo {
        " [PSEUDO]"
    } else {
        ""
    }
}

fn dispose_kind_tag(dk: DisposeKind) -> &'static str {
    match dk {
        DisposeKind::Sell => "sell",
        DisposeKind::Spend => "spend",
    }
}

fn income_kind_tag(ik: IncomeKind) -> &'static str {
    match ik {
        IncomeKind::Mining => "mining",
        IncomeKind::Staking => "staking",
        IncomeKind::Interest => "interest",
        IncomeKind::Airdrop => "airdrop",
        IncomeKind::Reward => "reward",
    }
}

/// UX-P4-7: SCREEN-ONLY human summary of an `InboundClass` decision payload (income / received gift /
/// self-transfer). Replaces the raw `{:?}` Debug dump (`SelfTransferMine { basis: Some(19000.00),
/// acquired_at: Some(2026-01-01) }`) that the CLI bulk-void preview + TUI void list truncated
/// mid-field. Like `pseudo_tag` [R0-I4], this is for the terminal ONLY — the CSV/form writers MUST
/// NOT call it (they emit stable machine tags via `*_tag`), so no human phrasing can leak into an
/// export file.
pub fn describe_inbound_class(c: &InboundClass) -> String {
    match c {
        InboundClass::Income {
            kind,
            fmv,
            business,
        } => {
            let mut s = format!("income {}", income_kind_tag(*kind));
            if let Some(v) = fmv {
                let _ = write!(s, ", fmv ${}", fmt_money(*v));
            }
            if *business {
                s.push_str(", business");
            }
            s
        }
        InboundClass::GiftReceived {
            donor_basis,
            donor_acquired_at,
            fmv_at_gift,
        } => {
            let mut s = format!("gift received, fmv ${}", fmt_money(*fmv_at_gift));
            if let Some(b) = donor_basis {
                let _ = write!(s, ", donor-basis ${}", fmt_money(*b));
            }
            if let Some(d) = donor_acquired_at {
                let _ = write!(s, ", donor-acquired {d}");
            }
            s
        }
        InboundClass::SelfTransferMine { basis, acquired_at } => {
            // `None` is a defaulted field (zero basis / 1yr+1day-before-receipt date), NOT the Debug
            // `None` — name it "default" so the void preview is legible.
            let basis_s = match basis {
                Some(v) => format!("${}", fmt_money(*v)),
                None => "default".to_string(),
            };
            let acq_s = match acquired_at {
                Some(d) => d.to_string(),
                None => "default".to_string(),
            };
            format!("self-transfer (mine), basis {basis_s}, acquired {acq_s}")
        }
    }
}

/// UX-P4-7: SCREEN-ONLY human summary of an `OutflowClass` decision payload. Same screen-only rule as
/// [`describe_inbound_class`] — never called by a CSV/form writer.
pub fn describe_outflow_class(c: &OutflowClass) -> String {
    match c {
        OutflowClass::Dispose { kind } => dispose_kind_tag(*kind).to_string(),
        OutflowClass::GiftOut => "gift".to_string(),
        OutflowClass::Donate { appraisal_required } => {
            if *appraisal_required {
                "donate (appraisal required)".to_string()
            } else {
                "donate".to_string()
            }
        }
    }
}

fn gift_zone_tag(gz: GiftZone) -> &'static str {
    match gz {
        GiftZone::Gain => "gain",
        GiftZone::Loss => "loss",
        GiftZone::NoGainNoLoss => "no_gain_no_loss",
    }
}

fn removal_kind_tag(rk: RemovalKind) -> &'static str {
    match rk {
        RemovalKind::Gift => "gift",
        RemovalKind::Donation => "donation",
    }
}

/// Stable term tag: "long" or "short" (not the Debug "LongTerm"/"ShortTerm").
fn term_tag(t: Term) -> &'static str {
    match t {
        Term::ShortTerm => "short",
        Term::LongTerm => "long",
    }
}

/// Stable Form 8949 part tag: "ST" (Part I) / "LT" (Part II). STABLE — part of the form8949.csv contract.
fn form8949_part_tag(p: Form8949Part) -> &'static str {
    match p {
        Form8949Part::ShortTerm => "ST",
        Form8949Part::LongTerm => "LT",
    }
}

/// Stable Form 8949 box tag: pre-TY2025 securities boxes "C" (ST) / "F" (LT), and from TY2025 the
/// digital-asset boxes "I" (ST) / "L" (LT) — the conservative "not reported on a 1099-B / 1099-DA"
/// default (D4). We never emit the 1099-reported boxes (A/B/D/E; G/H/J/K): the model carries no
/// 1099 basis-reported signal.
fn form8949_box_tag(b: Form8949Box) -> &'static str {
    match b {
        Form8949Box::C => "C",
        Form8949Box::F => "F",
        Form8949Box::I => "I",
        Form8949Box::L => "L",
    }
}

/// Stable Form 8283 section tag: "A" (deduction ≤ $5,000) / "B" (> $5,000). STABLE — part of the
/// form8283.csv contract.
fn form8283_section_tag(s: Form8283Section) -> &'static str {
    match s {
        Form8283Section::A => "A",
        Form8283Section::B => "B",
    }
}

/// Stable Form 8283 "how acquired" tag: literal Form 8283 categories (NOT the internal BasisSource
/// tags). STABLE — part of the form8283.csv contract.
fn form8283_how_acquired_tag(h: Form8283HowAcquired) -> &'static str {
    match h {
        Form8283HowAcquired::Purchased => "Purchased",
        Form8283HowAcquired::Gift => "Gift",
        Form8283HowAcquired::Other => "Other",
        Form8283HowAcquired::Review => "Review",
    }
}

/// Stable compliance-status display string, used in `render_verify` and optimizer output
/// in place of `{:?}` (which would expose unstable Rust Debug formatting).
///
/// Values:
/// - `standing_order:<date>` — in-force standing order effective from `<date>` (YYYY-MM-DD).
/// - `contemporaneous`       — `LotSelection` recorded on or before the day of sale.
/// - `attested_recording`    — Mode-1-persisted selection backed by contemporaneous-ID attestation (§C.2).
/// - `non_compliant`         — no adequate identification; FIFO is the defensible filing position.
fn compliance_status_tag(cs: &ComplianceStatus) -> String {
    match cs {
        ComplianceStatus::StandingOrder { effective_from } => {
            format!("standing_order:{effective_from}")
        }
        ComplianceStatus::Contemporaneous => "contemporaneous".into(),
        ComplianceStatus::AttestedRecording => "attested_recording".into(),
        ComplianceStatus::NonCompliant => "non_compliant".into(),
    }
}

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

/// `exchange:provider:account` | `self:label` (the same grammar `eventref::parse_wallet_id` accepts).
pub fn wallet_label(w: &WalletId) -> String {
    match w {
        WalletId::Exchange { provider, account } => format!("exchange:{provider}:{account}"),
        WalletId::SelfCustody { label } => format!("self:{label}"),
    }
}

/// UX-P4-9: the shared "insufficient balance" message for a `what-if sell`/`harvest` whose wallet
/// pool cannot cover the sale. Names the AVAILABLE balance, the wallet, and the as-of date so the
/// refusal is legible; `available == 0` is the honest "no BTC" case (an empty wallet), distinct from
/// mere insufficiency (lots exist but fall short). Used by BOTH the CLI (`cmd::whatif::map_whatif_err`)
/// and the interactive TUI panel (`btctax_tui::whatif_panel::refusal_message`) so the two surfaces
/// read identically. SCREEN-ONLY.
pub fn no_lots_message(wallet: &WalletId, at: TaxDate, available: Sat, requested: Sat) -> String {
    if available == 0 {
        format!("no BTC available in {} as of {}", wallet_label(wallet), at)
    } else {
        format!(
            "only {} BTC available in {} as of {} (requested {} BTC)",
            fmt_btc(available),
            wallet_label(wallet),
            at,
            fmt_btc(requested),
        )
    }
}

fn disposal_year(d: &btctax_core::Disposal) -> i32 {
    d.disposed_at.year()
}

/// FR4 render: holdings (always current) + realized disposals/removals/income (year-filtered).
pub fn render_report(state: &LedgerState, year: Option<i32>) -> String {
    let mut out = String::new();
    let yr = |y: i32| year.is_none_or(|f| f == y); // year filter; None => all (is_none_or stable since 1.82)

    let _ = writeln!(out, "Holdings (per wallet):");
    if state.holdings_by_wallet.is_empty() {
        let _ = writeln!(out, "  none");
    }
    for (w, sat) in &state.holdings_by_wallet {
        let _ = writeln!(out, "  {}: {} sat", wallet_label(w), sat);
    }
    // UX-P4-6: surface unreconciled transfer sats in the holdings view (BTC unit) so `report` no
    // longer hides what `verify` alone reported. Shown only when sats are actually pending.
    if state.stats.sigma_pending > 0 {
        let n = state.pending_reconciliation.len();
        let plural = if n == 1 { "transfer" } else { "transfers" };
        let _ = writeln!(
            out,
            "  Pending: {} BTC ({n} unreconciled {plural} — see `btctax verify`)",
            fmt_btc(state.stats.sigma_pending),
        );
    }

    let _ = writeln!(out, "Lots:");
    if state.lots.is_empty() {
        let _ = writeln!(out, "  none");
    }
    for l in &state.lots {
        let _ = writeln!(
            out,
            "  {}#{} {} remaining {} sat | basis {} ({}){}{}",
            l.lot_id.origin_event_id.canonical(),
            l.lot_id.split_sequence,
            wallet_label(&l.wallet),
            l.remaining_sat,
            l.usd_basis,
            basis_source_tag(l.basis_source),
            if l.basis_pending {
                " [basis pending]"
            } else {
                ""
            },
            pseudo_tag(l.pseudo),
        );
    }

    let label = match year {
        Some(y) => format!("(year {y})"),
        None => "(all years)".to_string(),
    };

    let disposals: Vec<_> = state
        .disposals
        .iter()
        .filter(|d| yr(disposal_year(d)))
        .collect();
    if disposals.is_empty() {
        let _ = writeln!(out, "Disposals {}: none", label);
    } else {
        let _ = writeln!(out, "Disposals {}:", label);
        for d in disposals {
            let _ = writeln!(
                out,
                "  {} @ {} ({})",
                dispose_kind_tag(d.kind),
                d.disposed_at,
                d.event.canonical()
            );
            for leg in &d.legs {
                render_disposal_leg(&mut out, leg);
            }
        }
    }

    let removals: Vec<_> = state
        .removals
        .iter()
        .filter(|r| yr(r.removed_at.year()))
        .collect();
    if removals.is_empty() {
        let _ = writeln!(out, "Removals {}: none", label);
    } else {
        let _ = writeln!(out, "Removals {}:", label);
        for r in removals {
            let deduction_tag = match r.claimed_deduction {
                Some(d) => format!(" [claimed deduction {}]", fmt_money(d)),
                None => String::new(),
            };
            let _ = writeln!(
                out,
                "  {} @ {} ({}){}",
                removal_kind_tag(r.kind),
                r.removed_at,
                r.event.canonical(),
                deduction_tag
            );
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
                "  {} @ {} {} sat = {} USD{}{}",
                income_kind_tag(i.kind),
                i.recognized_at,
                i.sat,
                i.usd_fmv,
                if i.business { " [business]" } else { "" },
                pseudo_tag(i.pseudo), // [R0-I2] a pseudo daily-close income FMV is flagged on screen
            );
        }
    }

    // Per-year charitable-deduction total (Schedule A itemized; pre-§170(b) AGI limits).
    // Σ claimed_deduction over Donation removals in the year-filtered window.
    let charitable_total: btctax_core::conventions::Usd = state
        .removals
        .iter()
        .filter(|r| yr(r.removed_at.year()) && r.kind == RemovalKind::Donation)
        .filter_map(|r| r.claimed_deduction)
        .sum();
    let _ = writeln!(
        out,
        "Charitable deduction {} (Schedule A itemized) — BEFORE §170(b) AGI limits / carryover: {}",
        label,
        fmt_money(charitable_total)
    );

    out
}

fn render_disposal_leg(out: &mut String, leg: &DisposalLeg) {
    let zone = leg
        .gift_zone
        .map(|z| format!(" gift-zone {}", gift_zone_tag(z)))
        .unwrap_or_default();
    let _ = writeln!(
        out,
        "    {} sat: proceeds {} basis {} gain {} {}{}{}",
        leg.sat,
        leg.proceeds,
        leg.basis,
        leg.gain,
        term_tag(leg.term),
        zone,
        pseudo_tag(leg.pseudo),
    );
}

fn render_removal_leg(out: &mut String, leg: &RemovalLeg) {
    let _ = writeln!(
        out,
        "    {} sat: basis {} fmv {} {} (zero gain){}",
        leg.sat,
        leg.basis,
        leg.fmv_at_transfer,
        term_tag(leg.term),
        pseudo_tag(leg.pseudo),
    );
}

// ── FR9 verify ──────────────────────────────────────────────────────────────────────────────────

/// Stable display tag for `FilingStatus` (lowercase, matches the CLI value-enum strings).
///
/// Values: "single" | "mfj" | "mfs" | "hoh" | "qss". These mirror the `FilingStatusArg`
/// `ValueEnum` strings accepted by `--filing-status`, so the `tax-profile --show` output
/// is round-trip-parseable via the same flag.
pub fn filing_status_tag(fs: btctax_core::FilingStatus) -> &'static str {
    use btctax_core::FilingStatus::*;
    match fs {
        Single => "single",
        Mfj => "mfj",
        Mfs => "mfs",
        HoH => "hoh",
        Qss => "qss",
    }
}

/// Stable display tag for `LotMethod` (FIFO/LIFO/HIFO — uppercase, human-readable).
/// UX-P4-12(e): a human label for the TP8 self-transfer fee treatment, instead of the raw Debug
/// variant name (`TreatmentC`/`TreatmentB`) leaking on screen.
pub fn fee_treatment_display(t: btctax_core::FeeTreatment) -> &'static str {
    match t {
        btctax_core::FeeTreatment::TreatmentC => "non-taxable, basis carries (TP8 c)",
        btctax_core::FeeTreatment::TreatmentB => "taxable mini-disposition (TP8 b)",
    }
}

pub fn lot_method_display(m: LotMethod) -> &'static str {
    match m {
        LotMethod::Fifo => "FIFO",
        LotMethod::Lifo => "LIFO",
        LotMethod::Hifo => "HIFO",
    }
}

/// One entry in the `MethodElection` standing-order history reported by `verify`.
#[derive(Debug, Clone)]
pub struct ElectionLine {
    pub recorded: TaxDate,
    pub effective_from: TaxDate,
    pub method: LotMethod,
    /// `None` = a VAULT-WIDE (global) election; `Some(w)` = scoped to that exchange account only.
    pub wallet: Option<WalletId>,
    /// "in force" | "voided" | "backdated/ignored"
    pub note: &'static str,
}

/// The set of decision targets that a `VoidDecisionEvent` has voided (for election notes + selection
/// counting). Shared by `verify` and `config` (UX-P4-12(c)).
pub fn voided_targets(events: &[LedgerEvent]) -> BTreeSet<EventId> {
    events
        .iter()
        .filter_map(|e| match &e.payload {
            EventPayload::VoidDecisionEvent(v) => Some(v.target_event_id.clone()),
            _ => None,
        })
        .collect()
}

/// The `MethodElection` standing-order history, sorted by `decision_seq` for a stable total order;
/// each `note` marks in-force / voided / backdated. Shared by `verify`'s Standing-orders block and
/// `config`'s forward-method read-back (UX-P4-12(c)).
pub fn method_election_lines(
    events: &[LedgerEvent],
    voided: &BTreeSet<EventId>,
) -> Vec<ElectionLine> {
    let mut election_events: Vec<(u64, &LedgerEvent)> = events
        .iter()
        .filter_map(|e| {
            if let EventId::Decision { seq } = e.id {
                if matches!(e.payload, EventPayload::MethodElection(_)) {
                    return Some((seq, e));
                }
            }
            None
        })
        .collect();
    election_events.sort_by_key(|(s, _)| *s);

    election_events
        .iter()
        .map(|(_, e)| {
            let EventPayload::MethodElection(me) = &e.payload else {
                unreachable!("filtered to MethodElection above")
            };
            let recorded = tax_date(e.utc_timestamp, e.original_tz);
            let note = if voided.contains(&e.id) {
                "voided"
            } else if me.effective_from < TRANSITION_DATE || me.effective_from < recorded {
                "backdated/ignored"
            } else {
                "in force"
            };
            ElectionLine {
                recorded,
                effective_from: me.effective_from,
                method: me.method,
                wallet: me.wallet.clone(),
                note,
            }
        })
        .collect()
}

/// Structured FR9 outcome (so tests assert on data, not stdout, and `main` keys the exit code).
#[derive(Debug, Clone)]
pub struct VerifyReport {
    pub conservation: ConservationReport,
    pub hard: Vec<Blocker>,
    pub advisory: Vec<Blocker>,
    pub pending: usize,
    pub unknown_basis_inbounds: usize,
    pub safe_harbor: String,
    /// Task 8: declared `pre2025_method` from the CLI config (attested or not).
    pub declared_pre2025_method: LotMethod,
    pub pre2025_method_attested: bool,
    /// Task 8: standing-order history (all `MethodElection` decisions, sorted by decision_seq).
    pub elections: Vec<ElectionLine>,
    /// Task 8: count of non-voided `LotSelection` decisions.
    pub selection_count: usize,
    /// Task 8: per-disposal compliance (post-2025 only).
    pub compliance: Vec<DisposalCompliance>,
    /// Task 11 (BG-D3): per-live-promote verify-drift advisory — the stored filed floor recomputed
    /// against CURRENT price data (overstated → conditional void+re-promote; understated → surfaced).
    /// Empty when no live promote drifts. Informational; never gates (the fold still uses the stored
    /// number).
    pub drift: Vec<String>,
}

impl VerifyReport {
    /// Non-zero exit condition (§7.1): any open hard blocker. (Conservation imbalance always coincides
    /// with a hard blocker — `uncovered_disposal` — so the hard list is the single source of truth.)
    pub fn has_hard_blockers(&self) -> bool {
        !self.hard.is_empty()
    }
}

/// 2025-transition status for display: detect effective Path B via lot basis-source, then
/// advisory blockers (§7.4). Prefer the effective-state signal (SafeHarborAllocated lots) over
/// the advisory blocker so the attest happy-path (void-prior → re-attest) is not
/// misreported as time-barred when a stale SafeHarborTimebar advisory remains in state.blockers
/// from the now-voided inert allocation.
///
/// Fix: also OR in disposal/removal legs for SafeHarborAllocated basis-source. When ALL
/// Path-B allocated lots are fully consumed (remaining_sat==0 → filtered out by `finalize`),
/// `state.lots` has no SafeHarborAllocated entries, but the disposed/removed legs still carry
/// the correct basis_source and prove Path B was effective at fold time.
fn safe_harbor_status(state: &LedgerState, _events: &[LedgerEvent]) -> String {
    // Effective Path B: seeded SafeHarborAllocated lots at the 2025-01-01 boundary.
    // Check remaining lots, disposal legs, and removal legs (all three carry basis_source).
    let effective_path_b = state
        .lots
        .iter()
        .any(|l| l.basis_source == BasisSource::SafeHarborAllocated)
        || state.disposals.iter().any(|d| {
            d.legs
                .iter()
                .any(|leg| leg.basis_source == BasisSource::SafeHarborAllocated)
        })
        || state.removals.iter().any(|r| {
            r.legs
                .iter()
                .any(|leg| leg.basis_source == BasisSource::SafeHarborAllocated)
        });
    let unconservable = state
        .blockers
        .iter()
        .any(|b| b.kind == BlockerKind::SafeHarborUnconservable);
    let timebar = state
        .blockers
        .iter()
        .any(|b| b.kind == BlockerKind::SafeHarborTimebar);
    // SafeHarborUnconservable is a hard blocker; resolve never seeds effective lots when it fires,
    // so unconservable wins unconditionally. effective_path_b next: if the engine is on Path B
    // (lots are present), report it regardless of any stale timebar advisory.
    if unconservable {
        "Path B allocation FAILS conservation/eligibility (hard, §7.4) — fix the allocation"
            .to_string()
    } else if effective_path_b {
        "Path B safe-harbor allocation is effective (§7.4)".to_string()
    } else if timebar {
        "Path B time-barred -> using Path A (advisory); `reconcile safe-harbor attest` if timely in your books".to_string()
    } else {
        "Path A (actual per-wallet reconstruction; default, no election)".to_string()
    }
}

pub fn build_verify(
    state: &LedgerState,
    events: &[LedgerEvent],
    prices: &dyn btctax_core::price::PriceProvider,
    cli: &CliConfig,
) -> VerifyReport {
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

    // Build the voided set (for election notes and selection counting).
    let voided = voided_targets(events);

    // Build election history (NFR4: sorted by decision_seq for a stable total order).
    let elections = method_election_lines(events, &voided);

    // Count non-voided LotSelection decisions.
    // Note: a `Decision`-id guard is intentionally omitted — `LotSelection` payloads are
    // exclusively carried by `EventId::Decision` events (appended via `append_decision` in the
    // CLI); filtering by payload alone is equivalent and sufficient.
    let selection_count = events
        .iter()
        .filter(|e| matches!(e.payload, EventPayload::LotSelection(_)) && !voided.contains(&e.id))
        .count();

    // Per-disposal compliance (§A.5): side-effect-free projection.
    let compliance = disposal_compliance(events, state);

    // Task 11 (BG-D3): the per-live-promote verify-drift advisory — recompute each stored floor against
    // CURRENT prices (overstated → conditional void+re-promote; understated → surfaced). Empty otherwise.
    let drift = btctax_core::conservative_promote::promote_drift_advisory(events, prices);

    VerifyReport {
        conservation,
        hard,
        advisory,
        pending: state.pending_reconciliation.len(),
        unknown_basis_inbounds,
        safe_harbor: safe_harbor_status(state, events),
        declared_pre2025_method: cli.pre2025_method,
        pre2025_method_attested: cli.pre2025_method_attested,
        elections,
        selection_count,
        compliance,
        drift,
    }
}

/// FR10: write the projected ledger as CSV (the NFR2 plaintext exception). One row per disposal/removal
/// leg (flattened) + one per lot/income record. Exact values (Decimal/i64) as strings (NFR5).
/// Each CSV is opened via `fsperms::open_owner_only` (0o600 on Unix) so decrypted PII matches the
/// hardened permissions already applied to `snapshot.sqlite` by the store crate. The out-dir is
/// created owner-only (0o700) if absent; when the dir PRE-EXISTS, open_owner_only still forces 0o600
/// on each new CSV file (the hole that `Writer::from_path` + umask would leave).
///
/// The `lots`/`disposals`/`removals`/`income` CSVs are all-years (a full projection dump). The
/// **Form 8949 + Schedule D** filing artifacts are inherently per-tax-year (P2-B): when `tax_year`
/// is `Some(y)`, `form8949.csv` + `schedule_d.csv` are additionally written, year-scoped to `y`;
/// when `None`, they are omitted.
pub fn write_csv_exports(
    out_dir: &Path,
    state: &LedgerState,
    tax_year: Option<i32>,
    se_result: Option<&SeTaxResult>,
    donation_details: &BTreeMap<EventId, DonationDetails>,
) -> Result<(), crate::CliError> {
    fsperms::mkdir_owner_only(out_dir)?;

    let mut w = Writer::from_writer(fsperms::open_owner_only(&out_dir.join("lots.csv"))?);
    w.write_record([
        "origin_event",
        "split",
        "wallet",
        "acquired_at",
        "remaining_sat",
        "usd_basis",
        "basis_source",
        "basis_pending",
    ])?;
    for l in &state.lots {
        w.write_record([
            l.lot_id.origin_event_id.canonical(),
            l.lot_id.split_sequence.to_string(),
            wallet_label(&l.wallet),
            l.acquired_at.to_string(),
            l.remaining_sat.to_string(),
            l.usd_basis.to_string(),
            basis_source_tag(l.basis_source).to_string(),
            l.basis_pending.to_string(),
        ])?;
    }
    w.flush()?;

    let mut w = Writer::from_writer(fsperms::open_owner_only(&out_dir.join("disposals.csv"))?);
    w.write_record([
        "event",
        "kind",
        "disposed_at",
        "lot",
        "sat",
        "proceeds",
        "basis",
        "gain",
        "term",
        "gift_zone",
        "acquired_at",
        "wallet",
    ])?;
    for d in &state.disposals {
        for leg in &d.legs {
            w.write_record([
                d.event.canonical(),
                dispose_kind_tag(d.kind).to_string(),
                d.disposed_at.to_string(),
                format!(
                    "{}#{}",
                    leg.lot_id.origin_event_id.canonical(),
                    leg.lot_id.split_sequence
                ),
                leg.sat.to_string(),
                leg.proceeds.to_string(),
                leg.basis.to_string(),
                leg.gain.to_string(),
                term_tag(leg.term).to_string(),
                leg.gift_zone
                    .map(|z| gift_zone_tag(z).to_string())
                    .unwrap_or_default(),
                leg.acquired_at.to_string(),
                wallet_label(&leg.wallet),
            ])?;
        }
    }
    w.flush()?;

    let mut w = Writer::from_writer(fsperms::open_owner_only(&out_dir.join("removals.csv"))?);
    // claimed_deduction: §170(e)(1)(A) deduction amount (pre-§170(b) AGI limits) for Donation
    // rows; empty for Gift rows. See Removal.claimed_deduction in btctax-core/src/state.rs.
    // Multi-leg donations: the value is a per-REMOVAL total shown on the FIRST leg row only;
    // subsequent leg rows carry an empty cell so a naive SUM() equals the correct per-donation
    // total (no double-counting). Do not divide across legs — that would create rounding artifacts.
    // donee: free-form donee label (Chunk 2); empty when not set.
    w.write_record([
        "event",
        "kind",
        "removed_at",
        "lot",
        "sat",
        "basis",
        "fmv_at_transfer",
        "term",
        "acquired_at",
        "claimed_deduction",
        "donee",
    ])?;
    for r in &state.removals {
        let deduction_first = r
            .claimed_deduction
            .map(|d| d.to_string())
            .unwrap_or_default();
        let donee_cell = r.donee.clone().unwrap_or_default();
        for (leg_idx, leg) in r.legs.iter().enumerate() {
            // Emit claimed_deduction only on leg 0; leave empty on subsequent legs so SUM()
            // over the column equals the true per-donation total (not N-legs × deduction).
            let deduction_cell: &str = if leg_idx == 0 { &deduction_first } else { "" };
            w.write_record([
                r.event.canonical(),
                removal_kind_tag(r.kind).to_string(),
                r.removed_at.to_string(),
                format!(
                    "{}#{}",
                    leg.lot_id.origin_event_id.canonical(),
                    leg.lot_id.split_sequence
                ),
                leg.sat.to_string(),
                leg.basis.to_string(),
                leg.fmv_at_transfer.to_string(),
                term_tag(leg.term).to_string(),
                leg.acquired_at.to_string(),
                deduction_cell.to_string(),
                donee_cell.clone(),
            ])?;
        }
    }
    w.flush()?;

    let mut w = Writer::from_writer(fsperms::open_owner_only(&out_dir.join("income.csv"))?);
    w.write_record([
        "event",
        "kind",
        "recognized_at",
        "sat",
        "usd_fmv",
        "business",
    ])?;
    for i in &state.income_recognized {
        w.write_record([
            i.event.canonical(),
            income_kind_tag(i.kind).to_string(),
            i.recognized_at.to_string(),
            i.sat.to_string(),
            i.usd_fmv.to_string(),
            i.business.to_string(),
        ])?;
    }
    w.flush()?;

    // P2-B: per-tax-year Form 8949 + Schedule D filing artifacts (year-scoped; omitted when None).
    // P2-C: per-tax-year Form 8283 donation artifact rides the same year-scoped block.
    if let Some(year) = tax_year {
        write_form8949_csv(out_dir, state, year)?;
        write_schedule_d_csv(out_dir, state, year)?;
        write_form8283_csv(out_dir, state, year, donation_details)?;
        write_basis_methodology_txt(out_dir, state, year)?; // P7 / D-4 (mandatory when a tranche is filed)
                                                            // P2-D / Chunk B: standalone Schedule SE §1401 figure — written only when there IS SE tax
                                                            // (a computed SeTaxResult); omitted when there is no business SE income OR when the year
                                                            // is fully expensed (expenses ≥ gross → net_se == 0 → compute_se_tax returns None — [N4]).
                                                            // The "fully expensed" render advisory (render_schedule_se) surfaces the liability status
                                                            // ("no §1401 SE tax"); the CSV writer sees None in both the no-income and fully-expensed
                                                            // cases — same omission, different reason.
        if let Some(se) = se_result {
            write_schedule_se_csv(out_dir, se)?;
        }
    }
    Ok(())
}

/// Write the year-scoped form artifacts for `year` into `out_dir`.
///
/// Creates `out_dir` owner-only (0o700) if absent (tolerant `mkdir_owner_only`, mkdir-p);
/// each file is written via `fsperms::open_owner_only` (0o600).  Writes `form8949.csv`,
/// `schedule_d.csv`, `form8283.csv`; `schedule_se.csv` when `se_result` is `Some`; and the
/// mandatory `basis_methodology.txt` (P7 / D-4) when a conservative-filing tranche is in the
/// year's filed set. The all-years dump CSVs (`lots.csv`, `disposals.csv`, `removals.csv`,
/// `income.csv`) are NOT written; `export_snapshot` / `snapshot.sqlite` are NEVER called
/// or written.
///
/// Path containment is the CALLER's job (matching `write_csv_exports` / `export_snapshot`
/// / `backup_key`): callers must pass a freshly-created or trusted directory — this
/// function truncates the four fixed filenames in `out_dir` (`open_owner_only` is
/// create-or-truncate and follows symlinks). The TUI's `export.rs` satisfies this via
/// `mkdir_owner_only_exclusive` (D2).
pub fn write_form_csvs(
    out_dir: &Path,
    state: &LedgerState,
    year: i32,
    se_result: Option<&SeTaxResult>,
    donation_details: &BTreeMap<EventId, DonationDetails>,
) -> Result<(), crate::CliError> {
    fsperms::mkdir_owner_only(out_dir)?;
    write_form8949_csv(out_dir, state, year)?;
    write_schedule_d_csv(out_dir, state, year)?;
    write_form8283_csv(out_dir, state, year, donation_details)?;
    write_basis_methodology_txt(out_dir, state, year)?; // P7 / D-4 (mandatory when a tranche is filed)
    if let Some(se) = se_result {
        write_schedule_se_csv(out_dir, se)?;
    }
    Ok(())
}

/// BG-D8 (Task 14): write the Form 8275 disclosure (`form_8275.txt`, 0o600) — by its OWN name — alongside
/// the year's form artifacts whenever a promoted DISPOSAL leg files in `year`. Writes NOTHING for a year
/// with no promoted disposal leg (`disclosure_8275` → `None`).
///
/// ★ Distinct from [`write_basis_methodology_txt`] in TWO ways the review loop pinned:
/// - **Its own name.** The gate + the success KAT key on `form_8275.txt`, never a `form_8275.txt ||
///   basis_methodology.txt` disjunction — `basis_methodology.txt` is written unconditionally for a
///   promoted year, so the disjunction would be a vacuous assertion (tax r1 I-8).
/// - **The gate ran first.** The completeness gate ([`crate::cmd::admin::promote_export_gate`]) refuses
///   BEFORE any bytes when a promoted leg's Part II is empty/incomplete, so a promoted leg reaching HERE
///   is guaranteed to carry a complete Part II — this always emits a filing-ready disclosure.
///
/// `pub(crate)` so the `export-snapshot` CSV / `export-irs-pdf` / full-return packet writers
/// (`cmd/admin.rs`) emit it at their `write_basis_methodology_txt` call sites.
pub(crate) fn write_form_8275_txt(
    out_dir: &Path,
    state: &LedgerState,
    events: &[LedgerEvent],
    year: i32,
) -> Result<(), crate::CliError> {
    use std::io::Write as _;
    if let Some(disc) = btctax_core::tax::form8275::disclosure_8275(events, state, year) {
        let mut file = fsperms::open_owner_only(&out_dir.join("form_8275.txt"))?;
        // `render()` already terminates with a newline — write, don't writeln (no trailing blank line).
        write!(file, "{}", disc.render())?;
    }
    Ok(())
}

/// P7 (D-4): write the MANDATORY conservative-filing methodology disclosure (`basis_methodology.txt`,
/// 0o600) alongside the year's form artifacts whenever a tranche is in the year's filed set. A no-tranche
/// year writes NOTHING — the i8949 basis explanation is required only when actual cost is not used.
/// `pub(crate)` so the `export-irs-pdf` / full-return packet writers (`cmd/admin.rs`) emit it too (I-3).
pub(crate) fn write_basis_methodology_txt(
    out_dir: &Path,
    state: &LedgerState,
    year: i32,
) -> Result<(), crate::CliError> {
    use std::io::Write as _;
    if let Some(text) = btctax_core::conservative::basis_methodology(state, year) {
        let mut file = fsperms::open_owner_only(&out_dir.join("basis_methodology.txt"))?;
        writeln!(file, "{text}")?;
    }
    Ok(())
}

/// P2-D Task 2: write `schedule_se.csv` — the standalone §1401 SE-tax components for the tax year.
/// One data row. Stable snake_case columns; exact `Decimal` string values (NFR5). 0o600 via
/// `open_owner_only`. Written only when a `SeTaxResult` exists (business SE income present + table).
fn write_schedule_se_csv(out_dir: &Path, se: &SeTaxResult) -> Result<(), crate::CliError> {
    let mut w = Writer::from_writer(fsperms::open_owner_only(&out_dir.join("schedule_se.csv"))?);
    w.write_record([
        "net_se_earnings",
        "se_base_9235",
        "ss_component",
        "medicare_component",
        "additional_medicare_component",
        "total_se_tax",
        "deductible_half",
    ])?;
    w.write_record([
        se.net_se.to_string(),
        se.base.to_string(),
        se.ss.to_string(),
        se.medicare.to_string(),
        se.addl.to_string(),
        se.total.to_string(),
        se.deductible_half.to_string(),
    ])?;
    w.flush()?;
    Ok(())
}

/// The standing Section A/B aggregation note — emitted as the first (comment) line of form8283.csv
/// and reused by any text/advisory path. Reflects the §170(f)(11)(F) year-aggregate implementation:
/// all BTC is "similar property"; the YEAR-total BTC donation deduction determines Section A/B
/// uniformly for all rows. CCA 202302012 confirms the readily-valued exception does not apply to
/// crypto, so a year-aggregate > $5,000 requires a qualified appraisal.
pub const FORM_8283_AGGREGATION_CAVEAT: &str =
    "Section A/B reflects the \u{00a7}170(f)(11)(F) year-aggregate for similar property: all BTC \
     donations in the year are summed (all BTC is 'similar property'); if the year-total claimed \
     deduction exceeds $5,000 every row is Section B (qualified appraisal required), otherwise \
     Section A. CCA 202302012: the readily-valued exception does not apply to crypto.";

/// §170(f)(11)(F) year-aggregate appraisal advisory (D2) — render-time only.
///
/// Emits a standalone advisory when the year-aggregate claimed deduction exceeds
/// `QUALIFIED_APPRAISAL_THRESHOLD` ($5,000, strict `>`): even if no single BTC donation exceeds
/// $5,000, the year-aggregate may require a qualified appraisal (CCA 202302012: the readily-valued
/// exception does not apply to crypto; all BTC is "similar property").
///
/// **Render-time only — does NOT enter `state.advisory` / the blocker set** (consistent with the
/// standalone-forms pattern; the per-donation `BlockerKind::QualifiedAppraisalNote` in fold.rs
/// is left as-is — this advisory adds the year-aggregate signal without touching the fold).
///
/// Delegates to `btctax_core::year_donation_deduction` — the **shared helper** that `form_8283`
/// uses for the Section A/B decision and that `write_form8283_csv` uses for the [R0-M1] $500 floor
/// note. This is the single source of truth: the form, the floor note, and this advisory all call
/// the same function, making it structurally impossible for them to diverge.
///
/// Returns `None` when the year has no donations or the aggregate ≤ $5,000.
pub fn render_donation_appraisal_advisory(state: &LedgerState, year: i32) -> Option<String> {
    use btctax_core::QUALIFIED_APPRAISAL_THRESHOLD;
    let agg = year_donation_deduction(state, year);
    if agg <= QUALIFIED_APPRAISAL_THRESHOLD {
        return None;
    }
    // N1: format both the aggregate and the threshold with the same money formatter (fmt_money →
    // 2dp, no thousands separator) so the two dollar figures in the advisory are styled uniformly.
    let threshold = fmt_money(QUALIFIED_APPRAISAL_THRESHOLD);
    Some(format!(
        "\u{00a7}170(f)(11)(F): your {year} BTC donations aggregate ${} of claimed deduction \
         (> ${threshold}) \u{2014} a qualified appraisal is required for the donated BTC even if \
         no single donation exceeds ${threshold} (all BTC is 'similar property'; CCA 202302012 \
         \u{2014} no readily-valued exception for crypto).",
        fmt_money(agg)
    ))
}

/// P2-C Task 2: write `form8283.csv` — one row per `Donation` `RemovalLeg` contributed in `year`.
/// Stable snake_case columns; exact `Decimal`/`i64` string values (NFR5). 0o600 via `open_owner_only`.
///
/// The file leads with `#`-prefixed comment lines: the standing aggregation note, and — when the
/// year's total noncash charitable deduction is ≤ $500 — the [R0-M1] filing-floor note that Form
/// 8283 is not required at that level (the rows are still emitted, informationally).
fn write_form8283_csv(
    out_dir: &Path,
    state: &LedgerState,
    year: i32,
    details: &BTreeMap<EventId, DonationDetails>,
) -> Result<(), crate::CliError> {
    use std::io::Write as _;
    let mut file = fsperms::open_owner_only(&out_dir.join("form8283.csv"))?;

    // STANDING aggregation note — CSV header comment line (read with comment=b'#').
    writeln!(file, "# {FORM_8283_AGGREGATION_CAVEAT}")?;

    // [R0-M1] $500 form-filing floor: Form 8283 is required only when total noncash contributions
    // for the year exceed $500. Rows are emitted regardless; add a note when the year's total
    // donation deduction is ≤ $500 that Form 8283 is not required at that level.
    // Uses btctax_core::year_donation_deduction — the shared helper (single source of truth) that
    // form_8283 and render_donation_appraisal_advisory also call.
    let total_deduction = year_donation_deduction(state, year);
    if total_deduction <= rust_decimal::Decimal::from(500) {
        writeln!(
            file,
            "# [R0-M1] The year's total noncash charitable deduction ({}) is <= $500; Form 8283 is \
             NOT required at that level (rows below are informational only).",
            fmt_money(total_deduction)
        )?;
    }

    let mut w = Writer::from_writer(file);
    w.write_record([
        "section",
        "description",
        "how_acquired",
        "date_acquired",
        "date_contributed",
        "cost_basis",
        "fmv",
        "claimed_deduction",
        "fmv_method",
        "donee",
        "appraiser",
        "needs_review",
        // NEW Part III/IV detail columns:
        "donee_ein",
        "donee_address",
        "appraiser_tin",
        "appraiser_ptin",
        "appraiser_qualifications",
        "appraisal_date",
    ])?;
    for row in form_8283(state, year, details) {
        let d = row.details.as_ref();
        w.write_record([
            row.section
                .map(form8283_section_tag)
                .unwrap_or("")
                .to_string(),
            row.description,
            form8283_how_acquired_tag(row.how_acquired).to_string(),
            row.date_acquired.to_string(),
            row.date_contributed.to_string(),
            row.cost_basis.to_string(),
            row.fmv.to_string(),
            row.claimed_deduction
                .map(|d| d.to_string())
                .unwrap_or_default(),
            row.fmv_method,
            row.donee,
            row.appraiser,
            row.needs_review.to_string(),
            // NEW:
            d.and_then(|d| d.donee_ein.clone()).unwrap_or_default(),
            d.and_then(|d| d.donee_address.clone()).unwrap_or_default(),
            d.and_then(|d| d.appraiser_tin.clone()).unwrap_or_default(),
            d.and_then(|d| d.appraiser_ptin.clone()).unwrap_or_default(),
            d.and_then(|d| d.appraiser_qualifications.clone())
                .unwrap_or_default(),
            d.and_then(|d| d.appraisal_date.map(|dt| dt.to_string()))
                .unwrap_or_default(),
        ])?;
    }
    w.flush()?;
    Ok(())
}

/// P2-B Task 2: write `form8949.csv` — one row per `DisposalLeg` disposed in `year`. Stable
/// snake_case columns; exact `Decimal`/`i64` string values (NFR5). 0o600 via `open_owner_only`.
fn write_form8949_csv(
    out_dir: &Path,
    state: &LedgerState,
    year: i32,
) -> Result<(), crate::CliError> {
    let mut w = Writer::from_writer(fsperms::open_owner_only(&out_dir.join("form8949.csv"))?);
    w.write_record([
        "part",
        "box",
        "box_needs_review",
        "description",
        "date_acquired",
        "date_sold",
        "proceeds",
        "cost_basis",
        "adjustment_code",
        "adjustment_amount",
        "gain",
        "wallet",
        "disposition_kind",
    ])?;
    for r in form_8949(state, year) {
        w.write_record([
            form8949_part_tag(r.part).to_string(),
            form8949_box_tag(r.box_).to_string(),
            r.box_needs_review.to_string(),
            r.description,
            r.date_acquired.to_string(),
            r.date_sold.to_string(),
            r.proceeds.to_string(),
            r.cost_basis.to_string(),
            r.adjustment_code,
            r.adjustment_amount.to_string(),
            r.gain.to_string(),
            wallet_label(&r.wallet),
            dispose_kind_tag(r.disposition_kind).to_string(),
        ])?;
    }
    w.flush()?;
    Ok(())
}

/// P2-B Task 3: write `schedule_d.csv` — the two RAW pre-netting part totals (Part I ST, Part II LT)
/// for `year`. §1222/§1211/§1212 netting + carryforward is applied by engine B, not here (D3).
fn write_schedule_d_csv(
    out_dir: &Path,
    state: &LedgerState,
    year: i32,
) -> Result<(), crate::CliError> {
    let mut w = Writer::from_writer(fsperms::open_owner_only(&out_dir.join("schedule_d.csv"))?);
    w.write_record(["part", "proceeds", "cost_basis", "gain"])?;
    let totals = schedule_d(state, year);
    for (part, p) in [("ST", &totals.st), ("LT", &totals.lt)] {
        w.write_record([
            part.to_string(),
            p.proceeds.to_string(),
            p.cost_basis.to_string(),
            p.gain.to_string(),
        ])?;
    }
    w.flush()?;
    Ok(())
}

/// UX-P4-1: the pseudo-disclosure channel for a tax-year figure — which, if any, deliberately-synthetic
/// input the number rides on. Carries the FULL §3.1 predicate (`pseudo_active() OR PseudoPlaceholder`) so a
/// caller cannot thread a single disjunct and silently drop the other (SPEC r2-N3). The two active channels
/// are mutually exclusive by PRECEDENCE — `Synthetic` (a pseudo synthetic lot/FMV; `pseudo_active()`, i.e.
/// `pseudo_synthetic_count > 0`) is chosen ahead of `Placeholder` (computed on the all-$0 pseudo placeholder
/// profile; mode on, nothing stored, `count == 0`) — even though the underlying states can co-occur.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PseudoDisclosure {
    /// Not pseudo-contributed — no banner, no suffix.
    None,
    /// A pseudo synthetic lot/FMV feeds the figure (`pseudo_active()`).
    Synthetic,
    /// Computed on the all-$0 pseudo placeholder profile (mode on, nothing stored, `count == 0`).
    Placeholder,
}

impl PseudoDisclosure {
    /// True iff the figure is pseudo-contributed (either active channel).
    pub fn contributed(self) -> bool {
        self != PseudoDisclosure::None
    }
    /// The ` [PSEUDO]` suffix for a headline total line (leading space kept so a last-field scraper reads
    /// `[PSEUDO]` and fails loud), or `""` when not contributed.
    pub fn suffix(self) -> &'static str {
        if self.contributed() {
            " [PSEUDO]"
        } else {
            ""
        }
    }
    /// The channel-aware top banner (with a trailing newline), or `""` when not contributed. Each clause is
    /// true for its channel; the remedy pointers are live only for the channel that fires them (SPEC §3.1).
    pub fn banner(self) -> &'static str {
        match self {
            PseudoDisclosure::None => "",
            PseudoDisclosure::Synthetic => {
                "⚠ [PSEUDO] This vault has pseudo-reconciled (deliberately-synthetic) entries; figures shown \
                 are an ESTIMATE, not filing-ready. See '[PSEUDO]' rows in 'btctax report' and the \
                 [PseudoReconcileActive] advisory in 'btctax verify'; resolve them before filing.\n"
            }
            PseudoDisclosure::Placeholder => {
                "⚠ [PSEUDO] These figures are estimated on a synthetic $0 placeholder profile — no tax \
                 profile or full-return inputs are stored for this year. This is an ESTIMATE, not \
                 filing-ready. Set a tax profile ('btctax tax-profile --year <Y> …' — setting is the \
                 default; '--show' inverts), import inputs ('btctax income import'), or turn pseudo mode off \
                 ('btctax reconcile pseudo off').\n"
            }
        }
    }
}

/// Task 9 (B.5) + Task 10 (M4): render the `TaxOutcome` for `report --tax-year <y>`. Exact Decimal
/// Display; no float (NFR5). B-M2 fold: surfaces the ordinary-rate attributable delta so the three
/// printed attributable components visibly reconcile to `total_federal_tax_attributable`.
///
/// `advisory` is the optional M4 carryforward-consistency warning string (Task 10). When `Some`,
/// it is printed as a non-gating advisory line that does not affect the exit code.
///
/// `pseudo` (UX-P4-1): the disclosure channel. When contributed, an unconditional top banner is emitted and
/// the TOTAL line is ` [PSEUDO]`-suffixed — so neither a human nor a single-line scraper reads the
/// authoritative number without the flag.
pub fn render_tax_outcome(
    year: i32,
    out: &btctax_core::TaxOutcome,
    advisory: Option<&str>,
    pseudo: PseudoDisclosure,
) -> String {
    use btctax_core::TaxOutcome::*;
    let mut s = String::new();
    s.push_str(pseudo.banner());
    let _ = writeln!(s, "Federal tax attributable to crypto — tax year {year}");
    match out {
        NotComputable(b) => {
            let _ = writeln!(s, "  NOT COMPUTABLE [{:?}]: {}", b.kind, b.detail);
        }
        Computed(r) => {
            let _ = writeln!(
                s,
                "  net short-term: {}   net long-term: {}",
                fmt_money(r.st_net),
                fmt_money(r.lt_net)
            );
            let _ = writeln!(
                s,
                "  crypto ordinary income (level): {}",
                fmt_money(r.ordinary_from_crypto)
            );
            // B-M2: surface the ordinary-rate attributable DELTA so the three attributable components
            // visibly reconcile to TOTAL. By the pinned identity this equals (ord_with − ord_without) exactly.
            let ordinary_rate_attributable = r.total_federal_tax_attributable - r.ltcg_tax - r.niit;
            let _ = writeln!(
                s,
                "  ordinary-rate tax (attributable): {}",
                fmt_money(ordinary_rate_attributable)
            );
            let _ = writeln!(
                s,
                "  LTCG tax (attributable): {}   NIIT (attributable): {}",
                fmt_money(r.ltcg_tax),
                fmt_money(r.niit)
            );
            let _ = writeln!(
                s,
                "  TOTAL federal tax attributable to crypto (delta): {}{}   \
                (= ordinary-rate + LTCG + NIIT attributable)",
                fmt_money(r.total_federal_tax_attributable),
                pseudo.suffix()
            );
            let _ = writeln!(
                s,
                "  §1211 loss deduction (level): {}   carryforward out: short {} / long {}",
                fmt_money(r.loss_deduction),
                fmt_money(r.carryforward_out.short),
                fmt_money(r.carryforward_out.long)
            );
            let _ = writeln!(
                s,
                "  marginal rates: ordinary {} / LTCG {} / NIIT increased by crypto: {}",
                r.marginal_rates.ordinary,
                r.marginal_rates.ltcg,
                if r.marginal_rates.niit_applies {
                    "yes"
                } else {
                    "no"
                }
            );
            let _ = writeln!(
                s,
                "  (incremental ceteris-paribus delta on the minimal profile; \
                excludes AGI-driven SS/IRMAA/AMT/QBI/phaseout effects — I5. §1411 NIIT reduces NII by the \
                §1211(b)-allowed net capital loss (≤ $3,000 / $1,500 MFS — Form 8960 line 5a / §1.1411-4(d)) \
                and is floored at $0; crypto ordinary income (mining/staking/airdrops/rewards) is correctly \
                excluded from NII; crypto-lending interest income (§1411(c)(1)(A)(i)) is INCLUDED in NII; \
                mining/staking/airdrops/rewards remain excluded (SE income per §1411(c)(6) or non-NII other income).)"
            );
        }
    }
    // M4 (Task 10): non-gating advisory — render after the main block so it is visible
    // regardless of whether the outcome is Computed or NotComputable.
    if let Some(msg) = advisory {
        let _ = writeln!(s, "  ADVISORY (M4): {msg}");
    }
    s
}

/// The house wrap width for advisory text (the widest line the tool prints anywhere).
pub(crate) const ADVISORY_WRAP_COLS: usize = 92;

/// Wrap `text` under a `  • ` bullet, with continuation lines hanging under the bullet's TEXT (a
/// 4-space indent) rather than under the bullet glyph. Breaks on whitespace only; a word longer than
/// the line (a URL, say) is left to overflow rather than being cut mid-token — a broken citation is
/// worse than a long line.
pub(crate) fn wrap_bulleted(text: &str) -> String {
    const BULLET: &str = "  \u{2022} ";
    const HANG: &str = "    ";
    let mut out = String::new();
    let mut line = String::from(BULLET);
    let mut have_word = false;

    for word in text.split_whitespace() {
        let prospective = line.chars().count() + usize::from(have_word) + word.chars().count();
        if have_word && prospective > ADVISORY_WRAP_COLS {
            out.push_str(line.trim_end());
            out.push('\n');
            line = String::from(HANG);
            have_word = false;
        }
        if have_word {
            line.push(' ');
        }
        line.push_str(word);
        have_word = true;
    }
    out.push_str(line.trim_end());
    out
}

/// Render the Phase-5 full-return **advisories** (SPEC §3.4 / §9.2) — the loud, non-gating notes that a
/// favorable credit was omitted conservatively (your tax is OVERSTATED), or that a disclosure is yours to
/// make. Never changes a number and never changes the exit code.
pub fn render_advisories(advisories: &[btctax_core::tax::advisories::Advisory]) -> String {
    let mut s = String::new();
    if advisories.is_empty() {
        return s;
    }
    let _ = writeln!(s, "\n  ── ADVISORIES ({}) ──", advisories.len());
    for a in advisories {
        // Wrap each message under its bullet (`p5-n5`): an advisory is a 300–400-character sentence, and
        // an unwrapped one is unreadable in an 80-column terminal. The message text itself is
        // single-sourced in core — this only decides where the line breaks are.
        let _ = writeln!(s, "{}", wrap_bulleted(&a.message()));
    }
    let _ = writeln!(
        s,
        "  (Advisories never change a number and never fail the command. See `btctax limitations`.)"
    );
    s
}

/// The §4.12 provenance label for the resolved profile — printed on the full-return output so a
/// reviewer can audit which source produced the figures (`p2-provenance-printing`).
pub fn provenance_label(p: crate::resolve::Provenance) -> &'static str {
    use crate::resolve::Provenance::*;
    match p {
        ReturnInputs => "ReturnInputs (derived from line items)",
        StoredProfile => "stored TaxProfile (raw override)",
        PseudoPlaceholder => "pseudo-reconcile placeholder ($0)",
        Missing => "none (TaxProfileMissing)",
    }
}

/// Render the **§6 dual report**: the absolute filed-return liability (Form 1040, WITH crypto) and the
/// crypto-attribution DELTA are **different questions** — shown together, labeled, and NEVER reconciled to
/// the dollar (SPEC §6). Only produced for a `ReturnInputs`-provenance year (a full 1040 exists). `delta`
/// is the same `TaxOutcome` the crypto-delta block already showed above. Provenance is printed here (§4.12).
/// ★ The absolute block renders the **PRINTED** figures — the whole-dollar, cross-footing lines the filed
/// PDF carries — not the exact-cents computation behind them (ARCH-P6 Q3).
///
/// The clinching case is line 37. "Amount you owe" is not an analytical figure; it is an instruction to
/// write a check. A tool that says $12,345.67 in the terminal and prints $12,347 on the filed form has
/// produced TWO authoritative answers to "what do I pay", and no LIMITATIONS paragraph repairs that. The
/// moment a report line is labelled with a form-line citation, it has promised the FORM's figure.
///
/// The crypto-DELTA block below stays in exact cents: it is not a filed figure — it answers a different
/// question (§6), and the frozen engine computes it in cents.
pub fn render_dual_report(
    year: i32,
    ar: &btctax_core::AbsoluteReturn,
    printed: &btctax_core::tax::packet::PrintedForms,
    delta: &btctax_core::TaxOutcome,
    provenance: crate::resolve::Provenance,
    pseudo: PseudoDisclosure,
) -> String {
    let f = &printed.f1040;
    let mut s = String::new();
    let _ = writeln!(
        s,
        "\n═══ Absolute filed return (Form 1040) — tax year {year} ═══"
    );
    let _ = writeln!(s, "  Profile source: {}", provenance_label(provenance));
    let _ = writeln!(s, "  Total income (1040 L9):   {}", fmt_money(f.line9));
    let _ = writeln!(s, "  Adjustments (L10):        {}", fmt_money(f.line10));
    let _ = writeln!(s, "  AGI (L11):                {}", fmt_money(f.line11));
    let ded_kind = if ar.deduction_is_itemized {
        "itemized"
    } else {
        "standard"
    };
    let _ = writeln!(s, "  Deduction (L12, {ded_kind}): {}", fmt_money(f.line12));
    if ar.qbi_deduction > Usd::ZERO {
        let _ = writeln!(s, "  QBI deduction (L13):      {}", fmt_money(f.line13));
    }
    let _ = writeln!(s, "  Taxable income (L15):     {}", fmt_money(f.line15));
    let _ = writeln!(s, "  Tax (L16):                {}", fmt_money(f.line16));
    if ar.foreign_tax_credit > Usd::ZERO {
        let _ = writeln!(
            s,
            "  Foreign tax credit (Sch 3 L1): {}",
            fmt_money(printed.sch_3.map_or(Usd::ZERO, |s3| s3.line1))
        );
    }
    if ar.se_tax_sch2_l4 > Usd::ZERO {
        let _ = writeln!(
            s,
            "  Self-employment tax (Sch 2 L4): {}",
            fmt_money(printed.sch_2.map_or(Usd::ZERO, |s2| s2.line4))
        );
    }
    if ar.additional_medicare.additional_medicare_tax > Usd::ZERO {
        let _ = writeln!(
            s,
            "  Additional Medicare (Form 8959 → Sch 2 L11): {}",
            fmt_money(printed.f8959.line18)
        );
    }
    if ar.niit.tax > Usd::ZERO {
        let _ = writeln!(
            s,
            "  Net Investment Income Tax (Form 8960 → Sch 2 L12): {}",
            fmt_money(printed.f8960.map_or(Usd::ZERO, |f| f.line17))
        );
    }
    let _ = writeln!(
        s,
        "  TOTAL TAX (L24):          {}{}",
        fmt_money(f.line24),
        pseudo.suffix()
    );
    let _ = writeln!(s, "  Total payments (L33):     {}", fmt_money(f.line33));
    if f.line34 > Usd::ZERO {
        let _ = writeln!(s, "  → REFUND (L35a):          {}", fmt_money(f.line34));
    } else {
        let _ = writeln!(s, "  → AMOUNT OWED (L37):      {}", fmt_money(f.line37));
    }
    // §6: the two figures answer different questions and are NEVER reconciled.
    let delta_str = match delta {
        btctax_core::TaxOutcome::Computed(r) => fmt_money(r.total_federal_tax_attributable),
        btctax_core::TaxOutcome::NotComputable(_) => "not computable".to_string(),
    };
    let _ = writeln!(
        s,
        "\n  ── Two DIFFERENT questions — NOT reconciled (SPEC §6) ──"
    );
    let _ = writeln!(
        s,
        "  • Absolute TOTAL TAX (this filed return, WITH crypto): {}{}",
        fmt_money(f.line24),
        pseudo.suffix()
    );
    let _ = writeln!(
        s,
        "  • Crypto-attributable tax (DELTA, shown above):        {delta_str}"
    );
    let _ = writeln!(
        s,
        "  The delta's implied deduction is fixed at derivation time (non-crypto AGI), so it is \
         APPROXIMATE where a\n  deduction is AGI-sensitive (e.g. the 7.5% medical floor); the two do NOT \
         reconcile to the dollar."
    );
    s
}

/// P2-B Task 3: render the RAW pre-netting Schedule D part totals (Part I ST, Part II LT) for
/// `year`, mirroring `render_tax_outcome`. These are the Form 8949/Schedule D part totals BEFORE
/// §1222/§1211/§1212 netting + carryforward — that netting is applied in the tax computation
/// (`report --tax-year`), and the netted figures are shown by `render_tax_outcome` above.
///
/// When `outcome` is `Computed`, the standard netting note is shown. When `outcome` is
/// `NotComputable`, a caveat is printed instead: the raw totals are valid disposal sums but are
/// informational — no netting or carryforward is applied because the tax is not computable.
/// The raw totals are ALWAYS shown (never suppressed); only the trailing note differs.
pub fn render_schedule_d(
    year: i32,
    totals: &ScheduleDTotals,
    outcome: &btctax_core::TaxOutcome,
) -> String {
    let mut s = String::new();
    let _ = writeln!(
        s,
        "Schedule D (raw pre-netting part totals) — tax year {year}"
    );
    let _ = writeln!(
        s,
        "  Part I  (short-term): proceeds {}   cost basis {}   gain {}",
        fmt_money(totals.st.proceeds),
        fmt_money(totals.st.cost_basis),
        fmt_money(totals.st.gain)
    );
    let _ = writeln!(
        s,
        "  Part II (long-term):  proceeds {}   cost basis {}   gain {}",
        fmt_money(totals.lt.proceeds),
        fmt_money(totals.lt.cost_basis),
        fmt_money(totals.lt.gain)
    );
    match outcome {
        btctax_core::TaxOutcome::NotComputable(_) => {
            let _ = writeln!(
                s,
                "  (raw disposition totals shown above; the year's tax is NOT COMPUTABLE until \
                 the blocker is resolved — these Form 8949/Schedule D part totals are \
                 informational and are not netted/carried until the tax computes)."
            );
        }
        btctax_core::TaxOutcome::Computed(_) => {
            let _ = writeln!(
                s,
                "  Note: §1222/§1211/§1212 netting + carryforward are applied in the tax \
                 computation (report --tax-year); these are the raw pre-netting Form \
                 8949/Schedule D part totals."
            );
        }
    }
    s
}

/// P2-D Task 2 / Chunk B (Schedule SE): render the standalone §1401 SE-tax block for `year` as an
/// informational block that does NOT feed engine B (`TaxResult::total_federal_tax_attributable` is
/// UNCHANGED by SE tax).
///
/// Three-way `None` split [R0-I1] (no silent drop — mirrors P2-C's m6):
/// - `gross_se == 0` → `None` (no business SE income → no Schedule SE section at all).
/// - `gross_se > 0 && !table_present` → a "SS wage base unavailable for {year}" note (business SE
///   income exists but the year has no bundled table → the wage base is unknown; the §1401 tax is
///   NOT computed rather than silently dropped).
/// - `gross_se > 0 && table_present && result == None` → a "fully expensed" line (expenses ≥ gross
///   → net_se == 0 → no §1401 SE tax owed; distinct from the "wage base unavailable" case).
/// - `result = Some(r)` → the full Schedule SE section (breakout or $0 note, components, total,
///   §164(f) advisory, W-2 coordination, the Chunk-B expense advisory, and the [D5] standalone note).
///
/// # Parameters
/// - `gross_se`: `se_net_income(state, year)` — the GROSS SE income before expenses (caller computes).
/// - `table_present`: `tables.table_for(year).is_some()` (caller has this from the `and_then` chain).
/// - `schedule_c_expenses`: from `TaxProfile.schedule_c_expenses` (≥ 0). When > 0 triggers the
///   breakout line and the Chunk-B ordinary-income advisory.
/// - `w2_ss_wages` / `w2_medicare_wages`: from `TaxProfile` (both ≥ 0). When either is > $0 the
///   W-2 coordinated disclosure is rendered; when both are $0 the short $0-assumed note is shown.
pub fn render_schedule_se(
    year: i32,
    result: Option<&SeTaxResult>,
    gross_se: Usd,
    table_present: bool,
    schedule_c_expenses: Usd,
    w2_ss_wages: Usd,
    w2_medicare_wages: Usd,
) -> Option<String> {
    match result {
        Some(r) => {
            let mut s = String::new();
            let _ = writeln!(
                s,
                "Schedule SE (§1401 self-employment tax on business crypto income) — tax year {year}"
            );
            // [Chunk B] Breakout line or $0 note depending on whether expenses were supplied.
            if schedule_c_expenses > Usd::ZERO {
                // The gross for display = net_se + expenses (since net_se = max(0, gross − expenses)
                // and net_se > 0 here, gross = net_se + expenses exactly).
                let gross_display = r.net_se + schedule_c_expenses;
                let _ = writeln!(
                    s,
                    "  gross business income {} \u{2212} Schedule C expenses {} = net SE earnings {}",
                    fmt_money(gross_display),
                    fmt_money(schedule_c_expenses),
                    fmt_money(r.net_se)
                );
                // [Chunk B / I3-mechanism] Ordinary-income advisory — correct mechanism; NO OTI-edit prescription.
                let _ = writeln!(
                    s,
                    "  (Schedule C advisory) Schedule C expenses also reduce your ORDINARY taxable \
                     income, but the income-tax total above uses GROSS crypto income \u{2014} to first \
                     order it OVERSTATES your tax by your marginal ordinary rate applied to {}. The tax \
                     profile cannot express this (an `ordinary_taxable_income` edit would shift both \
                     legs of the crypto-attributable delta); the engine-side coordination is deferred \
                     \u{2014} coordinate it on your actual return.",
                    fmt_money(schedule_c_expenses)
                );
            } else {
                let _ = writeln!(
                    s,
                    "  net self-employment income (business crypto, Interest excluded): {}",
                    fmt_money(r.net_se)
                );
                let _ = writeln!(
                    s,
                    "  (Schedule C) no Schedule C expenses supplied (--schedule-c-expenses)"
                );
            }
            let _ = writeln!(
                s,
                "  \u{00d7} 92.35% net-earnings factor (\u{00a7}1402(a)) = net SE earnings: {}",
                fmt_money(r.base)
            );
            let _ = writeln!(
                s,
                "  Social Security component (12.4%, §1401(a); capped at the SS wage base): {}",
                fmt_money(r.ss)
            );
            let _ = writeln!(
                s,
                "  Medicare component (2.9%, §1401(b); uncapped): {}",
                fmt_money(r.medicare)
            );
            let _ = writeln!(
                s,
                "  Additional Medicare component (0.9%, §1401(b)(2)): {}",
                fmt_money(r.addl)
            );
            let _ = writeln!(
                s,
                "  TOTAL self-employment tax (§1401): {}",
                fmt_money(r.total)
            );
            let _ = writeln!(
                s,
                "  §164(f) one-half-SE-tax deduction (above-the-line; EXCLUDES Additional Medicare per \
                 §164(f)(1)): {}",
                fmt_money(r.deductible_half)
            );
            // [Chunk A / R0-I3] §164(f) advisory — quantified first-order overstatement; NO prescription
            // to edit ordinary_taxable_income (wrong mechanism — see spec D3/R0-I3 rationale).
            let _ = writeln!(
                s,
                "  (§164(f) advisory) The §164(f) deduction ({}) is NOT auto-coordinated into the \
                 income-tax total above — to first order, that total overstates your combined tax by \
                 your marginal ordinary rate applied to {}. The tax profile cannot express this deduction \
                 directly (reducing `ordinary_taxable_income` would shift BOTH legs of the \
                 crypto-attributable delta and only correct the bracket differential, not the level) — \
                 coordinate it on your actual return.",
                fmt_money(r.deductible_half),
                fmt_money(r.deductible_half)
            );
            // [Chunk A / D3] W-2 coordination disclosure — accurate when W-2 values are set;
            // short $0-assumed note otherwise. REMOVES the old OVERSTATED/UNDERSTATED hedging.
            if w2_ss_wages > Usd::ZERO || w2_medicare_wages > Usd::ZERO {
                let _ = writeln!(
                    s,
                    "  (W-2 coordination applied) SS cap = max(0, wage base \u{2212} {}) (Box 3+7); \
                     Additional-Medicare threshold reduced (not below 0) by {} (Box 5, \
                     §1401(b)(2)(B)/Form 8959 Part II).",
                    fmt_money(w2_ss_wages),
                    fmt_money(w2_medicare_wages)
                );
            } else {
                let _ = writeln!(
                    s,
                    "  (W-2) assumes $0 W-2 wages (set --w2-ss-wages/--w2-medicare-wages on the tax \
                     profile if you had a wage job)."
                );
            }
            // [burndown-3 D2] §6017 $400 filing floor — the test is on the ×0.9235 base (§1402(a),
            // which includes the §1402(a)(12) 7.65% reduction), NOT the pre-factor net_se.
            if r.base < rust_decimal::Decimal::from(400) {
                let _ = writeln!(
                    s,
                    "  (§6017 filing floor) Net earnings from self-employment ({base}) are below $400: \
                     a Schedule SE filing is required on account of this income only when net earnings \
                     from self-employment (the ×92.35% base, §1402(a)) are $400 or more (§6017), and \
                     below that floor no §1401 SE tax is imposed (§1402(b)(2); church employee income \
                     excepted — §1402(j)(2), not modeled) — the figures above are shown for \
                     transparency (other self-employment activities, if any, combine on your actual \
                     Schedule SE).",
                    base = fmt_money(r.base)
                );
            }
            // [D5] standalone note — SE tax is a SEPARATE liability, not in the income-tax + NIIT total.
            let _ = writeln!(
                s,
                "  (standalone) This §1401 SE tax is a SEPARATE federal liability, NOT included in the \
                 income-tax + NIIT total above; the §164(f) one-half-SE-tax deduction is not \
                 auto-coordinated into that total."
            );
            Some(s)
        }
        None => {
            if gross_se.is_zero() {
                None // no business SE income → no Schedule SE section
            } else if !table_present {
                // Business SE income present but no bundled table → wage base unknown; do NOT drop.
                let mut s = String::new();
                let _ = writeln!(
                    s,
                    "Schedule SE (§1401 self-employment tax) — tax year {year}"
                );
                let _ = writeln!(
                    s,
                    "  SS wage base unavailable for {year}: business self-employment income is present \
                     but no bundled tax table (ss_wage_base) exists for {year}; the §1401 SE tax was \
                     NOT computed (no silent drop)."
                );
                Some(s)
            } else {
                // Business SE income present + table available + net_se == 0: fully expensed.
                // [R0-I1] The liability status is "no tax owed", NOT "couldn't compute".
                let mut s = String::new();
                let _ = writeln!(
                    s,
                    "Schedule SE (§1401 self-employment tax on business crypto income) — tax year {year}"
                );
                let _ = writeln!(
                    s,
                    "  fully expensed: gross {} \u{2212} Schedule C expenses {} \u{2264} $0 \
                     \u{2192} no \u{00a7}1401 SE tax for {year}.",
                    fmt_money(gross_se),
                    fmt_money(schedule_c_expenses)
                );
                Some(s)
            }
        }
    }
}

/// P2-C Task 3 (D3): Form 709 gift **per-donee advisory** (§2503(b) annual exclusion applied
/// independently per donee, not in aggregate).
///
/// Groups `Removal{Gift}` legs by their `donee` label for `year`:
/// - **`None`** when there are NO Gift removals in the year (even with a table present). [R0-I2]
/// - **`Some(note)` [R0-m6]** when gifts ARE present but the year has NO bundled table (exclusion
///   unavailable): Form 709 exposure is NOT evaluated — do NOT silently return `None`.
/// - **`Some(advisory)`** for all other cases: per-labeled-donee §2503(b) breakdown (each donee's
///   total vs the per-donee exclusion; filing trigger fires when ANY labeled donee exceeds the
///   exclusion), plus an unlabeled-bucket caveat when any `None`-donee gifts exist.
///
/// **Why per-donee matters (§2503(b)):** the exclusion applies to each recipient independently.
/// Two donees at $15k each with a $19k exclusion → $0 taxable (the old aggregate was WRONG: it
/// would flag the $30k combined total, even though neither donee exceeded their exclusion).
///
/// **Unlabeled bucket:** `None`-donee gifts cannot have per-donee exclusion applied; a conservative
/// aggregate-vs-one-exclusion signal is emitted with an explicit caveat. Nothing is silently dropped.
///
/// **Donations excluded:** `Removal{Donation}` (§170) must NOT appear here — this advisory is for
/// §2503(b) Gifts only. `kind == Gift` filter enforces this.
///
/// Standalone informational artifact — does NOT feed `compute_tax_year` / engine B.
pub fn render_gift_advisory(
    state: &LedgerState,
    year: i32,
    prior_taxable_gifts: btctax_core::conventions::Usd,
    tables: &impl btctax_core::TaxTables,
) -> Option<String> {
    use std::collections::BTreeMap;

    // [R0-I2] Preserve safety: any_gift guard — Donation removals do NOT count here.
    let any_gift = state
        .removals
        .iter()
        .any(|r| r.kind == RemovalKind::Gift && r.removed_at.year() == year);
    if !any_gift {
        return None; // (a) no Gift removals in the year
    }

    // [R0-m6] gifts present but no bundled table → emit the note, never None.
    let t = match tables.table_for(year) {
        None => {
            let total: btctax_core::conventions::Usd = state
                .removals
                .iter()
                .filter(|r| r.kind == RemovalKind::Gift && r.removed_at.year() == year)
                .flat_map(|r| r.legs.iter())
                .map(|leg| leg.fmv_at_transfer)
                .sum();
            return Some(format!(
                "Form 709 gift advisory ({year}): gift annual-exclusion table unavailable for \
                 {year}; Form 709 exposure not evaluated — ${} in gifts recorded.",
                fmt_money(total)
            ));
        }
        Some(t) => t,
    };
    let excl = t.gift_annual_exclusion;

    // Group Gift removals by donee label (BTreeMap → deterministic order; None → unlabeled bucket).
    let mut labeled: BTreeMap<String, btctax_core::conventions::Usd> = BTreeMap::new();
    let mut unlabeled_count: usize = 0;
    let mut unlabeled_total: btctax_core::conventions::Usd = Default::default();

    for r in state
        .removals
        .iter()
        .filter(|r| r.kind == RemovalKind::Gift && r.removed_at.year() == year)
    {
        let fmv: btctax_core::conventions::Usd = r.legs.iter().map(|l| l.fmv_at_transfer).sum();
        match &r.donee {
            Some(label) => {
                *labeled.entry(label.clone()).or_default() += fmv;
            }
            None => {
                unlabeled_count += 1;
                unlabeled_total += fmv;
            }
        }
    }

    // Per-donee §2503(b) analysis.
    let mut filing_required_donees: Vec<String> = Vec::new();
    let mut total_taxable: btctax_core::conventions::Usd = Default::default();
    let mut s = format!("Form 709 gift advisory ({year}):");

    if !labeled.is_empty() {
        s.push_str(&format!(
            "\n§2503(b) per-donee annual exclusion analysis (TY{year}, exclusion ${}):",
            fmt_money(excl)
        ));
        for (donee, &total) in &labeled {
            let applied = if total < excl { total } else { excl };
            let taxable: btctax_core::conventions::Usd = if total > excl {
                total - excl
            } else {
                Default::default()
            };
            s.push_str(&format!(
                "\n  {donee}: total ${}, exclusion applied ${}, taxable ${}",
                fmt_money(total),
                fmt_money(applied),
                fmt_money(taxable)
            ));
            if total > excl {
                filing_required_donees.push(donee.clone());
                total_taxable += taxable;
            }
        }
        if !filing_required_donees.is_empty() {
            s.push_str(&format!(
                "\nForm 709 filing required (donee(s): {}). Total taxable gifts: ${}.",
                filing_required_donees.join(", "),
                fmt_money(total_taxable)
            ));
        } else {
            s.push_str(&format!(
                "\nNo Form 709 filing required based on per-donee totals \
                 (each ≤ ${} exclusion). Total taxable gifts: $0.00.",
                fmt_money(excl)
            ));
        }
    }

    if unlabeled_count > 0 {
        s.push_str(&format!(
            "\nNOTE: {unlabeled_count} gift(s) totalling ${} have no donee label — the §2503(b) \
             annual exclusion is PER DONEE and cannot be applied without one; label them via \
             `reconcile reclassify-outflow --donee`. Shown as a single conservative aggregate.",
            fmt_money(unlabeled_total)
        ));
        // Conservative aggregate signal (old per-bucket logic): keep the signal so nothing is
        // silently dropped; mark it explicitly as a conservative estimate (may span multiple donees).
        if unlabeled_total > excl {
            s.push_str(&format!(
                "\n  Conservative aggregate ${} > ${} (one exclusion); \
                 verify per-donee totals after labelling.",
                fmt_money(unlabeled_total),
                fmt_money(excl)
            ));
        } else {
            s.push_str(&format!(
                "\n  Conservative aggregate ${} \u{2264} ${} (one exclusion); \
                 per-donee exposure unverifiable without labels.",
                fmt_money(unlabeled_total),
                fmt_money(excl)
            ));
        }
    }

    // [D3 / R0-I1] §2505 lifetime (basic) exclusion consumption block.
    // current_year_taxable = total_taxable (Σ LABELED-donee taxable, per Chunk-2 design).
    // unlabeled gifts are excluded from this figure (their per-donee taxable is unknown).
    let lifetime_excl = t.gift_lifetime_exclusion;
    let cumulative_taxable = prior_taxable_gifts + total_taxable;

    // Emit block only when cumulative > 0 (covers prior-only [M4] and over-annual cases).
    if cumulative_taxable > btctax_core::conventions::Usd::ZERO {
        let remaining = if cumulative_taxable >= lifetime_excl {
            btctax_core::conventions::Usd::ZERO
        } else {
            lifetime_excl - cumulative_taxable
        };
        s.push_str(&format!(
            "\n§2505 lifetime (basic) exclusion: you have used ${} of your ${} ({year}) lifetime \
             exclusion (${} remaining). No gift tax is DUE until cumulative taxable gifts exceed \
             the lifetime exclusion.",
            fmt_money(cumulative_taxable),
            fmt_money(lifetime_excl),
            fmt_money(remaining),
        ));
        // Strict `>`: at exactly the exclusion → remaining $0, NOT exceeded.
        if cumulative_taxable > lifetime_excl {
            let excess = cumulative_taxable - lifetime_excl;
            s.push_str(&format!(
                "\nlifetime exclusion EXCEEDED — gift tax may be due on ${} \
                 (the excess base past the unified credit, not a computed tax); \
                 consult a professional.",
                fmt_money(excess),
            ));
        }
        // [R0-I2] Unlabeled-omission disclosure: when unlabeled gifts exist, §2505 consumption
        // is understated (unlabeled gifts could have taxable amounts not reflected here).
        if unlabeled_count > 0 {
            s.push_str(&format!(
                "\n§2505 consumption reflects LABELED-donee taxable gifts only; \
                 {unlabeled_count} unlabeled gift(s) totalling ${} are NOT included — \
                 label them via `--donee` for a complete figure; consumption may be \
                 understated / remaining overstated.",
                fmt_money(unlabeled_total),
            ));
        }
    }

    // [R0-I1] Updated caveats: stale "§2505 … later chunk (Chunk 3)" removed; §2513 and
    // future-interest caveats preserved; §2505-specific caveats added.
    s.push_str(
        "\nCaveats: §2513 gift-splitting (MFJ) not modeled (single-filer advisory only); \
         future-interest gifts (which require Form 709 filing regardless of amount) not \
         detectable; §2505 figures are advisory only — no portability/DSUE (§2010(c)(4)) \
         applied; prior cumulative taxable gifts are user-supplied (default $0 if \
         --prior-taxable-gifts not given).",
    );

    Some(s)
}

// ── Sub-project C: optimize run ─────────────────────────────────────────────────────────────────

/// Format a lot-pick slice as comma-separated `"<event>#<split>:<sat>"` entries for proposal display.
/// Mirrors the grammar `eventref::parse_lot_pick` accepts, so picks are both human-readable and
/// round-trip-parseable. An empty pick list renders as `"(none)"`.
fn picks_str(picks: &[btctax_core::LotPick]) -> String {
    if picks.is_empty() {
        return "(none)".to_string();
    }
    picks
        .iter()
        .map(|p| {
            format!(
                "{}#{}:{}",
                p.lot.origin_event_id.canonical(),
                p.lot.split_sequence,
                p.sat
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Render a `OptimizeProposal` (Mode-1 what-if) for the `optimize run` command. Returns a String
/// containing the proposal header, any approximate banner, the aggregate tax delta, per-disposal
/// rows (with proposed selection + compliance status + persistability), and the R0-M2 caveat footer.
///
/// **Approximate banner (R0-C1/C3/R2-C1):** when `p.approximate == true`, a ⚠ APPROXIMATE banner
/// and the specific `approx_reason` are printed. When `false`, no banner is printed (proven global
/// minimum — do NOT add a banner for this case).
///
/// **R2-M1 no-change rows:** a disposal whose `proposed_selection == current_selection` has nothing
/// to attest or persist (the optimizer is NOT asking to change it). The persistability line is
/// suppressed and a "no change — already optimal" note is shown instead, preventing a misleading
/// "needs --attest" prompt on a row the user does not need to act on.
pub fn render_optimize_proposal(p: &btctax_core::OptimizeProposal) -> String {
    use btctax_core::{ApproxReason, Persistability};
    let mut s = String::new();
    let _ = writeln!(
        s,
        "Optimize (what-if) — tax year {} — NOTHING is filed or bound by running this.",
        p.year
    );
    // R0-C1/C3: a non-fully-enumerated result is NEVER presented as "the optimum" without this banner.
    if p.approximate {
        let why = match p.approx_reason {
            Some(ApproxReason::ComboCapExceeded { combos, cap }) => format!(
                "input exceeded the exhaustive bound ({combos} combos > {cap}); \
                 a coordinate-descent fallback ran"
            ),
            Some(ApproxReason::ContentionUnenumerated { contended, .. }) => format!(
                "{contended} contended same-wallet disposal(s) could not be fully joint-enumerated"
            ),
            Some(ApproxReason::PoolHeuristic { lots, bound }) => format!(
                "a pool of {lots} lots exceeds the {bound}-lot exhaustive-enumeration bound; \
                 only a deterministic heuristic SUBSET of that pool's identifications was searched"
            ),
            None => "approximate".to_string(),
        };
        let _ = writeln!(
            s,
            "  \u{26a0} APPROXIMATE \u{2014} NOT a guaranteed global minimum: {why}."
        );
        let _ = writeln!(
            s,
            "    The true least-tax assignment may be lower; this is a disclosed improvement over your"
        );
        let _ = writeln!(
            s,
            "    current filing position (delta \u{2264} 0), NOT \u{2018}the least tax.\u{2019}"
        );
    }
    let _ = writeln!(
        s,
        "  current federal tax (attributable): {}",
        p.baseline_tax
    );
    let _ = writeln!(
        s,
        "  optimized federal tax (attributable): {}",
        p.optimized_tax
    );
    let _ = writeln!(
        s,
        "  delta (optimized \u{2212} current): {}  (negative = saving; always \u{2264} 0)",
        p.delta
    );
    for d in &p.per_disposal {
        let _ = writeln!(
            s,
            "  {} @ {} [{}] :: {}",
            d.disposal.canonical(),
            d.date,
            wallet_label(&d.wallet),
            compliance_status_tag(&d.status)
        );
        // R2-M1: a NO-CHANGE row (proposed == current) has nothing to attest/persist — `accept` SKIPS it
        // ("already optimal under current identification"). Do NOT print a persistability line here: a
        // `NeedsAttestation` "needs --attest" line on a disposal the optimizer is NOT asking to change is
        // misleading and invites a pointless/contradictory attestation. Show a no-change note instead.
        if d.proposed_selection == d.current_selection {
            let _ = writeln!(
                s,
                "      proposed: {}  [no change \u{2014} already optimal under current identification]",
                picks_str(&d.proposed_selection)
            );
            continue;
        }
        let persist = match d.persistable {
            Persistability::ContemporaneousNow => {
                "persistable now (made \u{2264} sale \u{2192} Contemporaneous)"
            }
            Persistability::NeedsAttestation => {
                "already executed \u{2014} needs `optimize accept --disposal <ref> \
                 --attest \"\u{2026}\"` (genuine contemporaneous ID only)"
            }
            Persistability::ForbiddenBroker2027 => {
                "2027+ broker-held \u{2014} CANNOT be persisted (own-books insufficient); \
                 FIFO is the defensible position"
            }
        };
        let _ = writeln!(
            s,
            "      proposed: {}  [{}]",
            picks_str(&d.proposed_selection),
            persist
        );
    }
    // R0-M2: surface the vertex-granularity limitation in OUTPUT, not only in docs.
    let _ = writeln!(
        s,
        "  (vertex-granularity identification: a multi-partial split landing exactly on a \
         tax-bracket kink is out of scope.)"
    );
    let _ = writeln!(
        s,
        "  (this is the tax IF you had identified thus; adequate ID must exist by the time \
         of sale \u{2014} \u{a7}1.1012-1(j))"
    );
    // C-M3: document the optimizer scope boundary (mirrors R0-M2 vertex-granularity caveat).
    let _ = writeln!(
        s,
        "  (scope: global over taxable-disposal lot selections; self-transfer lot routing is \
         held at its baseline position and is not re-optimized.)"
    );
    s
}

/// Render an `AcceptOutcome` (Task 10 `optimize accept`): one line per persisted `LotSelection`
/// (with the appended decision id to pass to `reconcile void` for revocation, and the §A.5 basis
/// label) and one line per skipped disposal (with the gate reason). A persisted attestation is noted
/// inline on the `AttestedRecording` rows.
pub fn render_accept_outcome(o: &crate::cmd::optimize::AcceptOutcome) -> String {
    let mut s = String::new();
    let _ = writeln!(
        s,
        "Optimize accept \u{2014} {} persisted, {} skipped.",
        o.persisted.len(),
        o.skipped.len()
    );
    for (disposal, decision, basis) in &o.persisted {
        let _ = writeln!(
            s,
            "  PERSISTED {} \u{2192} LotSelection {} [{}]{}",
            disposal.canonical(),
            decision.canonical(),
            basis,
            if *basis == "AttestedRecording" {
                " (+ attestation recorded; revoke with `reconcile void`)"
            } else {
                " (revoke with `reconcile void`)"
            }
        );
    }
    for (disposal, reason) in &o.skipped {
        let _ = writeln!(s, "  skipped {}: {}", disposal.canonical(), reason);
    }
    if o.persisted.is_empty() && o.skipped.is_empty() {
        let _ = writeln!(s, "  (no disposals matched)");
    }
    s
}

/// Render a `ConsultReport` (Task 11 / §C.3 Mode-2 read-only pre-trade what-if) for the
/// `optimize consult` command. Returns a String with:
///   - The hypothetical sale header (sat amount, wallet, date).
///   - The proposed lot selection (the tax-minimizing picks).
///   - The ST/LT gain split and the federal tax attributable to this contemplated sale.
///   - When `timing.is_some()`: the ST→LT crossover line (crossover date + saving), OMITTED when None.
///   - A footer: tax decision-support only, not investment advice.
///
/// **READ-ONLY:** this function only renders; it never writes any event or side-table row.
pub fn render_consult(r: &btctax_core::ConsultReport) -> String {
    let mut s = String::new();
    let _ = writeln!(
        s,
        "Consult (read-only what-if): sell {} sat from {} on {}",
        r.req.sell_sat,
        wallet_label(&r.req.wallet),
        r.req.at
    );
    // C-M2: for large pools (>12 lots) the candidate set is a heuristic subset — disclose it.
    if r.approximate {
        let _ = writeln!(
            s,
            "  \u{26a0} heuristic \u{2014} searched a subset of a large (>12-lot) pool; \
             the proposed selection may not be the exact minimum."
        );
    }
    let _ = writeln!(
        s,
        "  proposed selection: {}",
        picks_str(&r.proposed_selection)
    );
    let _ = writeln!(
        s,
        "  short-term gain: {}   long-term gain: {}",
        r.st_gain, r.lt_gain
    );
    // [consult fix] Headline the sale's OWN marginal effect (withhyp − baseline); keep the whole-year
    // figure clearly relabeled below it (on a year with real disposals the two DIFFER).
    let _ = writeln!(s, "  marginal federal tax (this sale): {}", r.marginal_tax);
    let _ = writeln!(
        s,
        "  whole-year federal tax attributable (with this sale): {}",
        r.total_federal_tax_attributable
    );
    if let Some(t) = &r.timing {
        let _ = writeln!(
            s,
            "  timing: {} sat of the best selection is short-term until {}; \
             selling on/after then would be taxed long-term, a \u{2248} {} difference.",
            t.st_sat_in_selection, t.latest_crossover, t.saving_if_waited
        );
    }
    let _ = writeln!(
        s,
        "Tax decision-support only \u{2014} consequences of a contemplated sale; \
         not investment advice (no buy/sell/hold recommendation)."
    );
    s
}

/// The §1(h) 0/15/20 rate zone label for the sale's preferential dollars.
fn ltcg_bracket_label(b: LtcgBracket) -> &'static str {
    match b {
        LtcgBracket::Zero => "0%",
        LtcgBracket::Fifteen => "15%",
        LtcgBracket::Twenty => "20%",
    }
}

/// Render a `what-if sell` `SellReport` (task #43). Headlines the MARGINAL federal tax (the sale's OWN
/// effect — `withhyp − baseline`), then the §1(h) bracket + room, the effective rate (or n/a for a loss),
/// the §1212 carryforward disclosure (delta-based — the this-year ordinary offset AND the amount carried,
/// NEVER a hard-coded $3,000), and the §1411 NIIT delta with its sign. `magi_caveat` prints the
/// ad-hoc-profile "MAGI assumed = ordinary income" note. Read-only; the vault is never touched.
pub fn render_whatif_sell(r: &SellReport, magi_caveat: bool) -> String {
    let mut s = String::new();
    let _ = writeln!(
        s,
        "What-if (read-only): sell {} sat from {} on {}",
        r.req.sell_sat,
        wallet_label(&r.req.wallet),
        r.req.at
    );
    if magi_caveat {
        let _ = writeln!(
            s,
            "  \u{26a0} MAGI assumed = ordinary income; NIIT may be understated if you have other MAGI."
        );
    }
    let _ = writeln!(s, "  proceeds: {}", r.proceeds);
    let _ = writeln!(s, "  lots consumed:");
    for leg in &r.lots {
        let term = match leg.term {
            Term::ShortTerm => "ST",
            Term::LongTerm => "LT",
        };
        let _ = writeln!(
            s,
            "    {}#{}  {} sat  basis {}  {} \u{2192} {}  {}  gain {}",
            leg.lot_id.origin_event_id.canonical(),
            leg.lot_id.split_sequence,
            leg.sat,
            leg.basis,
            leg.acquired_at,
            leg.sold_at,
            term,
            leg.gain
        );
    }
    let _ = writeln!(
        s,
        "  short-term gain: {}   long-term gain: {}",
        r.st_gain, r.lt_gain
    );
    // §1(h) bracket + headroom to the next breakpoint.
    match r.bracket_room {
        Some(room) => {
            let _ = writeln!(
                s,
                "  \u{00a7}1(h) LTCG bracket: {} (room {} before the next breakpoint)",
                ltcg_bracket_label(r.bracket),
                room
            );
        }
        None => {
            let _ = writeln!(
                s,
                "  \u{00a7}1(h) LTCG bracket: {} (top bracket \u{2014} no headroom)",
                ltcg_bracket_label(r.bracket)
            );
        }
    }
    // The headline: the sale's OWN marginal federal tax.
    let _ = writeln!(s, "  marginal federal tax (this sale): {}", r.marginal_tax);
    match r.effective_rate {
        Some(rate) => {
            let _ = writeln!(s, "  effective rate: {rate}");
        }
        None => {
            let _ = writeln!(
                s,
                "  effective rate: n/a (a loss/zero-gain sale \u{2014} its value is the carryforward, \
                 not this-year tax)"
            );
        }
    }
    // §1212 disclosure — delta-based, NEVER a hard-coded $3,000. Shown whenever a loss is carried OR the
    // sale unlocks a this-year ordinary offset.
    let carried = r.carryforward_delta.short + r.carryforward_delta.long;
    if carried != Usd::ZERO || r.ordinary_offset_delta != Usd::ZERO {
        let _ = writeln!(
            s,
            "  \u{00a7}1212: {} offsets ordinary income this year, {} carried to next year \
             (short {} / long {})",
            r.ordinary_offset_delta, carried, r.carryforward_delta.short, r.carryforward_delta.long
        );
    }
    // §1411 NIIT delta (with its sign) — only when the sale actually moved NIIT.
    if r.niit_applies {
        let dir = if r.niit_incremental < Usd::ZERO {
            "decrease"
        } else {
            "increase"
        };
        let _ = writeln!(
            s,
            "  \u{00a7}1411 NIIT: {} ({dir}) attributable to this sale",
            r.niit_incremental
        );
    }
    let status = match r.status {
        SellStatus::Gain => "net gain",
        SellStatus::Loss => "net loss (the carryforward is the value \u{2014} not this-year tax)",
    };
    let _ = writeln!(s, "  status: {status}");
    let _ = writeln!(
        s,
        "Tax decision-support only \u{2014} consequences of a contemplated sale; \
         not investment advice (no buy/sell/hold recommendation)."
    );
    s
}

/// A human label for a harvest target.
fn harvest_target_label(t: &HarvestTarget) -> String {
    match t {
        HarvestTarget::ZeroLtcg => {
            "zero-ltcg (all gain in the \u{00a7}1(h) 0% bracket)".to_string()
        }
        HarvestTarget::FifteenLtcg => "fifteen-ltcg (stay at/under 15%)".to_string(),
        HarvestTarget::Gain(x) => format!("gain \u{2264} {x}"),
        HarvestTarget::Tax(x) => format!("marginal tax \u{2264} {x}"),
    }
}

/// Render a `what-if harvest` `HarvestReport` (task #43). Headlines the MAX BTC to sell (N*), the binding
/// constraint, the realized ST/LT split at N*, which §1(h) bracket the surviving preferential dollars
/// land in, the exact marginal federal tax, and the MANDATORY disclosures — the §1212(b) carryforward
/// delta/burn, the §1411 NIIT kink (a 0%/15% answer can still cost +3.8%), and the plateau note. The
/// answer is engine-verified. `magi_caveat` prints the ad-hoc "MAGI assumed = ordinary income" note.
pub fn render_whatif_harvest(r: &HarvestReport, magi_caveat: bool) -> String {
    let mut s = String::new();
    let _ = writeln!(
        s,
        "What-if HARVEST (read-only): {} from {} on {}",
        harvest_target_label(&r.req.target),
        wallet_label(&r.req.wallet),
        r.req.at
    );
    if magi_caveat {
        let _ = writeln!(
            s,
            "  \u{26a0} MAGI assumed = ordinary income; NIIT may be understated if you have other MAGI."
        );
    }
    let status = match &r.status {
        HarvestStatus::Found => "FOUND (the target binds)",
        HarvestStatus::NotBinding => "NOT BINDING (the whole position fits)",
        HarvestStatus::AlreadyBreached => "ALREADY BREACHED at N=0 (nothing can be harvested)",
        HarvestStatus::NoLots => "NO LOTS (nothing to harvest from that wallet)",
        HarvestStatus::ProceedsRequired => "PROCEEDS REQUIRED",
        HarvestStatus::PreTransitionYear => "PRE-2025 (a closed year, not a plan)",
        HarvestStatus::YearNotComputable(_) => "YEAR NOT COMPUTABLE",
    };
    let _ = writeln!(s, "  status: {status}");
    let _ = writeln!(s, "  \u{2192} sell up to {} BTC ({} sat)", r.n_btc, r.n_sat);
    let _ = writeln!(s, "  bound by: {}", r.binding_constraint);
    let _ = writeln!(
        s,
        "  realized gain at N*: short-term {}   long-term {}",
        r.st_gain, r.lt_gain
    );
    // §1(h) bracket of the surviving preferential dollars at N*.
    let ps = &r.with_result.pref_split;
    let bracket = if ps.at_20 > Usd::ZERO {
        "20%"
    } else if ps.at_15 > Usd::ZERO {
        "15%"
    } else {
        "0%"
    };
    let _ = writeln!(
        s,
        "  \u{00a7}1(h) preferential dollars at N*: {} in 0% / {} in 15% / {} in 20% (top bracket: {})",
        ps.at_0, ps.at_15, ps.at_20, bracket
    );
    let _ = writeln!(s, "  marginal federal tax at N*: {}", r.marginal_tax);
    // §1212 carryforward delta (burn = a gain absorbing a carried loss).
    let carried = r.carryforward_delta.short + r.carryforward_delta.long;
    if carried != Usd::ZERO {
        let dir = if carried < Usd::ZERO {
            "burned (spent)"
        } else {
            "carried to next year"
        };
        let _ = writeln!(
            s,
            "  \u{00a7}1212 carryforward {}: {} (short {} / long {})",
            dir, carried, r.carryforward_delta.short, r.carryforward_delta.long
        );
    }
    // §1411 NIIT kink — surfaced on bracket targets too (a 0%/15% answer can still cost +3.8%).
    if r.niit_applies {
        let dir = if r.niit_incremental < Usd::ZERO {
            "decrease"
        } else {
            "increase"
        };
        let _ = writeln!(
            s,
            "  \u{00a7}1411 NIIT: {} ({dir}) at N* \u{2014} the +3.8% kink applies even inside a 0%/15% bracket answer",
            r.niit_incremental
        );
    }
    if let Some(note) = &r.plateau_note {
        let _ = writeln!(s, "  \u{2139} {note}");
    }
    let _ = writeln!(
        s,
        "Tax decision-support only \u{2014} the engine-verified consequences of a contemplated harvest; \
         not investment advice (no buy/sell/hold recommendation)."
    );
    s
}

pub fn render_verify(r: &VerifyReport) -> String {
    let mut out = String::new();
    let c = &r.conservation;
    let _ = writeln!(
        out,
        "Conservation (FR9): {}",
        if c.balanced { "BALANCED" } else { "DRIFT" }
    );
    let _ = writeln!(
        out,
        "  in {} = disposed {} + removed {} + held {} + fee-sats {} + pending {}{}",
        c.sigma_in,
        c.sigma_disposed,
        c.sigma_removed,
        c.sigma_held,
        c.sigma_fee_sats,
        c.sigma_pending,
        if c.has_uncovered {
            "  [identity undefined: uncovered disposal open]"
        } else {
            ""
        }
    );
    let _ = writeln!(out, "2025 transition: {}", r.safe_harbor);
    let _ = writeln!(
        out,
        "Pending reconciliation: {} transfer(s); unknown-basis inbounds: {}",
        r.pending, r.unknown_basis_inbounds
    );

    let _ = writeln!(
        out,
        "Hard blockers (gate tax computation): {}",
        r.hard.len()
    );
    for b in &r.hard {
        let evt = b
            .event
            .as_ref()
            .map(|e| e.canonical())
            .unwrap_or_else(|| "-".to_string());
        let _ = writeln!(out, "  [{:?}] {} :: {}", b.kind, evt, b.detail);
        // #41 Part C: an FMV gap can be a missing daily close — point at the separate updater (a STRING
        // only; the tax binaries never fetch). Pseudo mode can also fill it from the cache (Part B).
        if b.kind == BlockerKind::FmvMissing {
            let _ = writeln!(out, "         ↳ {}", crate::price_cache::UPDATE_PRICES_HINT);
        }
    }
    let _ = writeln!(out, "Advisory blockers: {}", r.advisory.len());
    for b in &r.advisory {
        let evt = b
            .event
            .as_ref()
            .map(|e| e.canonical())
            .unwrap_or_else(|| "-".to_string());
        let _ = writeln!(out, "  [{:?}] {} :: {}", b.kind, evt, b.detail);
    }
    let _ = writeln!(
        out,
        "Pre-2025 method (attested historical fact): {} (attested: {})",
        lot_method_display(r.declared_pre2025_method),
        r.pre2025_method_attested
    );
    let _ = writeln!(
        out,
        "Standing orders (MethodElection): {}",
        r.elections.len()
    );
    for e in &r.elections {
        let _ = writeln!(
            out,
            "  recorded {} effective {} -> {} [{}]",
            e.recorded,
            e.effective_from,
            lot_method_display(e.method),
            e.note
        );
    }
    let _ = writeln!(out, "Lot selections recorded: {}", r.selection_count);
    let _ = writeln!(
        out,
        "Per-disposal compliance (post-2025): {}",
        r.compliance.len()
    );
    for c in &r.compliance {
        let _ = writeln!(
            out,
            "  {} @ {} :: {}",
            c.disposal.canonical(),
            c.date,
            compliance_status_tag(&c.status)
        );
    }
    // Task 11 (BG-D3): the per-live-promote verify-drift advisory (a stored floor that recomputes away
    // from current price data). Informational — never gates.
    let _ = writeln!(out, "Promote-basis drift advisories: {}", r.drift.len());
    for d in &r.drift {
        let _ = writeln!(out, "  {d}");
    }
    out
}

#[cfg(test)]
mod gift_advisory_tests {
    //! P2-C Task 3 KATs — `render_gift_advisory` (per-donee §2503(b) refactor, Chunk 2).
    //!
    //! Direct-state `Removal{Gift}` fixtures + a `BTreeMap<i32, TaxTable>` table double so the
    //! exclusion + no-table cases are under exact control. Exclusion = $19,000 (TY2025) throughout.
    //! PRIVACY: synthetic values only.
    use super::*;
    use btctax_core::conventions::Usd;
    use btctax_core::{EventId, LotId, Removal, RemovalLeg, TaxTable};
    use rust_decimal_macros::dec;
    use std::collections::BTreeMap;
    use time::macros::date;

    /// Build an unlabeled (`donee: None`) Gift removal with a single leg of the given FMV.
    fn gift_removal(seq: u64, removed_at: TaxDate, fmv: Usd) -> Removal {
        Removal {
            event: EventId::decision(seq),
            kind: RemovalKind::Gift,
            removed_at,
            legs: vec![RemovalLeg {
                lot_id: LotId {
                    origin_event_id: EventId::decision(seq),
                    split_sequence: 0,
                },
                sat: 100,
                basis: dec!(0),
                fmv_at_transfer: fmv,
                term: Term::LongTerm,
                basis_source: BasisSource::ComputedFromCost,
                acquired_at: date!(2024 - 01 - 01),
                pseudo: false,
            }],
            appraisal_required: false,
            donor_acquired_at: None,
            claimed_deduction: None,
            donee: None,
        }
    }
    /// Build a labeled Gift removal (donee = `Some(label)`) using the same single-leg structure.
    fn gift_removal_labeled(seq: u64, removed_at: TaxDate, fmv: Usd, label: &str) -> Removal {
        Removal {
            donee: Some(label.to_string()),
            ..gift_removal(seq, removed_at, fmv)
        }
    }
    fn state_with(removals: Vec<Removal>) -> LedgerState {
        LedgerState {
            removals,
            ..Default::default()
        }
    }
    /// A table double carrying only the gift_annual_exclusion (ordinary/ltcg empty — unread here).
    /// Uses TY2025 lifetime exclusion ($13,990,000) as default; tests that need a different
    /// lifetime exclusion can use `tables_with_lifetime`.
    fn tables_with(year: i32, excl: Usd) -> BTreeMap<i32, TaxTable> {
        tables_with_lifetime(year, excl, dec!(13_990_000))
    }

    /// Like `tables_with` but with an explicit `lifetime_excl` for §2505 boundary tests.
    fn tables_with_lifetime(year: i32, excl: Usd, lifetime_excl: Usd) -> BTreeMap<i32, TaxTable> {
        let mut m = BTreeMap::new();
        m.insert(
            year,
            TaxTable {
                year,
                source: "TEST",
                ordinary: BTreeMap::new(),
                ltcg: BTreeMap::new(),
                gift_annual_exclusion: excl,
                ss_wage_base: dec!(176100),
                gift_lifetime_exclusion: lifetime_excl,
            },
        );
        m
    }

    // ── Preserved safety branches ────────────────────────────────────────────────────────────────

    /// No gifts in the year → None (even with a table present). [R0-I2] safety preserved.
    #[test]
    fn no_gifts_is_none() {
        let st = state_with(vec![]);
        let tables = tables_with(2025, dec!(19000));
        assert!(render_gift_advisory(&st, 2025, dec!(0), &tables).is_none());
    }

    /// [R0-m6] gifts present but NO bundled table → Some(note), NOT None (no silent skip).
    /// The no-table note records the total gifts so nothing is silently dropped.
    #[test]
    fn gifts_present_but_no_table_emits_note_not_none() {
        // Unlabeled gift — the no-table branch fires before per-donee grouping.
        let st = state_with(vec![gift_removal(1, date!(2026 - 06 - 01), dec!(50000))]);
        // Table double has 2025 only → table_for(2026) == None.
        let tables = tables_with(2025, dec!(19000));
        let msg =
            render_gift_advisory(&st, 2026, dec!(0), &tables).expect("note expected, not None");
        assert!(msg.contains("unavailable"), "{msg}");
        assert!(msg.contains("Form 709 exposure not evaluated"), "{msg}");
        assert!(
            msg.contains("50000.00"),
            "must record the gift total: {msg}"
        );
    }

    // ── Labeled-donee over-exclusion ─────────────────────────────────────────────────────────────

    /// A labeled donee over the exclusion → filing required advisory with the per-donee breakdown.
    /// (Replaces the stale `over_exclusion_emits_advisory_with_total_and_caveat` which asserted the
    /// now-removed "donee identity is not modeled" / "total-exposure signal" phrases.)
    #[test]
    fn labeled_donee_over_exclusion_emits_advisory() {
        let st = state_with(vec![gift_removal_labeled(
            1,
            date!(2025 - 06 - 01),
            dec!(20000),
            "Alice",
        )]);
        let tables = tables_with(2025, dec!(19000));
        let msg = render_gift_advisory(&st, 2025, dec!(0), &tables).expect("advisory expected");
        assert!(msg.contains("20000.00"), "must show Alice's total: {msg}");
        assert!(msg.contains("19000.00"), "must show the exclusion: {msg}");
        assert!(
            msg.contains("Form 709 filing required"),
            "must flag filing required: {msg}"
        );
        assert!(msg.contains("Alice"), "must name Alice: {msg}");
        // taxable = 20000 − 19000 = 1000.
        assert!(msg.contains("1000.00"), "taxable must be $1000.00: {msg}");
        // The stale "donee identity is not modeled" caveat must be gone.
        assert!(
            !msg.contains("donee identity is not modeled"),
            "stale aggregate caveat must not appear: {msg}"
        );
    }

    /// A labeled donee under the exclusion → advisory with "no filing required" (not None).
    /// (Replaces the stale `under_exclusion_is_none` which tested None for an unlabeled gift.)
    #[test]
    fn labeled_donee_under_exclusion_no_filing_required() {
        let st = state_with(vec![gift_removal_labeled(
            1,
            date!(2025 - 06 - 01),
            dec!(10000),
            "Alice",
        )]);
        let tables = tables_with(2025, dec!(19000));
        // Gifts present + labeled donee → always Some (per-donee breakdown shown).
        let msg =
            render_gift_advisory(&st, 2025, dec!(0), &tables).expect("advisory expected, not None");
        assert!(
            msg.contains("No Form 709 filing required"),
            "must say no filing required: {msg}"
        );
        assert!(msg.contains("Alice"), "must mention Alice: {msg}");
        assert!(msg.contains("10000.00"), "must show Alice's total: {msg}");
    }

    // ── KATs (hand-verified; TY2025 gift_annual_exclusion $19,000) ──────────────────────────────

    /// KEY LOCK — per-donee under exclusion: Alice $15,000 + Bob $15,000 (aggregate $30,000 > $19k,
    /// but each < $19k) → NO filing required, $0 taxable. The OLD aggregate rule wrongly flagged
    /// this — this test proves per-donee §2503(b) is correctly applied.
    #[test]
    fn per_donee_under_exclusion_two_donees_no_filing_required() {
        let st = state_with(vec![
            gift_removal_labeled(1, date!(2025 - 03 - 01), dec!(15000), "Alice"),
            gift_removal_labeled(2, date!(2025 - 06 - 01), dec!(15000), "Bob"),
        ]);
        let tables = tables_with(2025, dec!(19000));
        let msg = render_gift_advisory(&st, 2025, dec!(0), &tables).expect("advisory expected");
        // No filing required — neither Alice nor Bob exceeds $19,000.
        assert!(
            msg.contains("No Form 709 filing required"),
            "neither donee exceeds exclusion → no filing required: {msg}"
        );
        // Both donees appear in the per-donee breakdown.
        assert!(msg.contains("Alice"), "Alice must appear: {msg}");
        assert!(msg.contains("Bob"), "Bob must appear: {msg}");
        // Both totals shown ($15,000 each).
        assert!(msg.contains("15000.00"), "donee total must appear: {msg}");
        // No labeled donee triggered the filing trigger.
        assert!(
            !msg.contains("Form 709 filing required (donee(s):"),
            "filing trigger must NOT fire: {msg}"
        );
        // Total taxable = $0 for both donees.
        assert!(
            msg.contains("Total taxable gifts: $0.00"),
            "total taxable must be $0.00: {msg}"
        );
    }

    /// One labeled donee over exclusion: Alice $25,000 → filing required, taxable $6,000
    /// (= $25,000 − $19,000). Exact figures are hand-verified KAT values.
    #[test]
    fn one_donee_over_exclusion_filing_required() {
        let st = state_with(vec![gift_removal_labeled(
            1,
            date!(2025 - 06 - 01),
            dec!(25000),
            "Alice",
        )]);
        let tables = tables_with(2025, dec!(19000));
        let msg = render_gift_advisory(&st, 2025, dec!(0), &tables).expect("advisory expected");
        assert!(
            msg.contains("Form 709 filing required (donee(s): Alice)"),
            "must trigger filing required for Alice: {msg}"
        );
        assert!(msg.contains("25000.00"), "Alice total must appear: {msg}");
        assert!(
            msg.contains("19000.00"),
            "exclusion applied must appear: {msg}"
        );
        // taxable = 25000 − 19000 = 6000.
        assert!(
            msg.contains("6000.00"),
            "taxable $6,000.00 must appear: {msg}"
        );
    }

    /// Unlabeled bucket: a None-donee gift $30,000 → the unlabeled caveat + conservative aggregate
    /// signal (per-donee cannot be applied without a label). $30,000 > $19,000 → conservative signal.
    #[test]
    fn unlabeled_bucket_caveat_with_conservative_aggregate() {
        let st = state_with(vec![gift_removal(1, date!(2025 - 06 - 01), dec!(30000))]);
        let tables = tables_with(2025, dec!(19000));
        let msg = render_gift_advisory(&st, 2025, dec!(0), &tables).expect("advisory expected");
        // Unlabeled caveat must appear.
        assert!(
            msg.contains("no donee label"),
            "unlabeled caveat must appear: {msg}"
        );
        assert!(
            msg.contains("30000.00"),
            "unlabeled total must appear: {msg}"
        );
        // Conservative aggregate signal: $30,000 > $19,000 (one exclusion).
        assert!(
            msg.contains("Conservative aggregate"),
            "conservative aggregate signal must appear: {msg}"
        );
        assert!(
            msg.contains("19000.00"),
            "one-exclusion comparison must appear: {msg}"
        );
        // No labeled-donee filing trigger must have fired.
        assert!(
            !msg.contains("Form 709 filing required (donee(s):"),
            "labeled filing trigger must NOT fire for unlabeled gifts: {msg}"
        );
    }

    /// Mixed: Alice $25,000 (over exclusion) + unlabeled $5,000 → filing required for Alice +
    /// the unlabeled caveat for the $5,000 (which cannot have per-donee exclusion applied).
    #[test]
    fn mixed_labeled_over_and_unlabeled_shows_both() {
        let st = state_with(vec![
            gift_removal_labeled(1, date!(2025 - 03 - 01), dec!(25000), "Alice"),
            gift_removal(2, date!(2025 - 06 - 01), dec!(5000)), // unlabeled
        ]);
        let tables = tables_with(2025, dec!(19000));
        let msg = render_gift_advisory(&st, 2025, dec!(0), &tables).expect("advisory expected");
        // Alice triggers the filing required signal.
        assert!(
            msg.contains("Form 709 filing required"),
            "filing required for Alice: {msg}"
        );
        assert!(msg.contains("Alice"), "Alice must appear: {msg}");
        // Unlabeled caveat must also appear for the $5,000 gift.
        assert!(
            msg.contains("no donee label"),
            "unlabeled caveat must appear: {msg}"
        );
        assert!(
            msg.contains("5000.00"),
            "unlabeled total must appear: {msg}"
        );
    }

    /// Donations excluded: a `Removal{Donation}` does NOT count as a Gift → advisory returns None
    /// (no Gift events in the year). Form 709 is §2503(b) — Gifts only; §170 Donations are separate.
    #[test]
    fn donations_excluded_from_form709_advisory() {
        let donation_removal = Removal {
            event: EventId::decision(1),
            kind: RemovalKind::Donation,
            removed_at: date!(2025 - 06 - 01),
            legs: vec![RemovalLeg {
                lot_id: LotId {
                    origin_event_id: EventId::decision(1),
                    split_sequence: 0,
                },
                sat: 100,
                basis: dec!(0),
                fmv_at_transfer: dec!(50000), // large FMV — must NOT trigger the advisory
                term: Term::LongTerm,
                basis_source: BasisSource::ComputedFromCost,
                acquired_at: date!(2024 - 01 - 01),
                pseudo: false,
            }],
            appraisal_required: false,
            donor_acquired_at: None,
            claimed_deduction: Some(dec!(50000)),
            donee: Some("Charity X".to_string()),
        };
        let st = state_with(vec![donation_removal]);
        let tables = tables_with(2025, dec!(19000));
        // A Donation is NOT a Gift → any_gift == false → advisory returns None.
        assert!(
            render_gift_advisory(&st, 2025, dec!(0), &tables).is_none(),
            "Donation must be excluded from the Form 709 advisory"
        );
    }

    // ── Chunk-3a §2505 KATs (hand-verified; TY2025: annual $19,000, lifetime $13,990,000) ────────

    /// [KAT-U] Under lifetime — Alice $100,000 gift, prior $0.
    /// current-year taxable = $81,000 (100k − 19k); used $81,000; remaining $13,909,000.
    /// No "EXCEEDED" line.
    #[test]
    fn section_2505_under_lifetime_shows_used_and_remaining() {
        let st = state_with(vec![gift_removal_labeled(
            1,
            date!(2025 - 06 - 01),
            dec!(100000),
            "Alice",
        )]);
        let tables = tables_with(2025, dec!(19000)); // lifetime = $13,990,000 via tables_with
        let msg = render_gift_advisory(&st, 2025, dec!(0), &tables).expect("advisory expected");
        // current-year taxable = 100000 − 19000 = 81000
        assert!(
            msg.contains("81000.00"),
            "taxable $81,000 must appear: {msg}"
        );
        // §2505 block: used $81,000 of $13,990,000
        assert!(
            msg.contains("§2505 lifetime (basic) exclusion"),
            "§2505 block must appear: {msg}"
        );
        assert!(
            msg.contains("13990000.00"),
            "lifetime exclusion $13,990,000 must appear: {msg}"
        );
        // remaining = 13,990,000 − 81,000 = 13,909,000
        assert!(
            msg.contains("13909000.00"),
            "remaining $13,909,000 must appear: {msg}"
        );
        // No "EXCEEDED" — still under lifetime
        assert!(
            !msg.contains("EXCEEDED"),
            "must NOT say EXCEEDED when under limit: {msg}"
        );
        // [I1] stale Chunk-3 caveat is gone
        assert!(
            !msg.contains("later chunk (Chunk 3)"),
            "stale Chunk-3 caveat must be absent: {msg}"
        );
    }

    /// [KAT-P] Prior gifts accumulate — Alice $100,000, prior $13,900,000.
    /// cumulative = 13,900,000 + 81,000 = 13,981,000; remaining = $9,000; no tax.
    #[test]
    fn section_2505_prior_gifts_accumulate() {
        let st = state_with(vec![gift_removal_labeled(
            1,
            date!(2025 - 06 - 01),
            dec!(100000),
            "Alice",
        )]);
        let tables = tables_with(2025, dec!(19000));
        let msg =
            render_gift_advisory(&st, 2025, dec!(13_900_000), &tables).expect("advisory expected");
        // cumulative = 13,900,000 + 81,000 = 13,981,000
        assert!(
            msg.contains("13981000.00"),
            "cumulative $13,981,000 must appear: {msg}"
        );
        // remaining = 13,990,000 − 13,981,000 = 9,000
        assert!(
            msg.contains("9000.00"),
            "remaining $9,000 must appear: {msg}"
        );
        assert!(
            !msg.contains("EXCEEDED"),
            "must NOT say EXCEEDED when under limit: {msg}"
        );
    }

    /// [KAT-E] Exceeds lifetime — Alice $100,000, prior $13,950,000.
    /// cumulative = 13,950,000 + 81,000 = 14,031,000 > 13,990,000.
    /// excess = 14,031,000 − 13,990,000 = 41,000.
    #[test]
    fn section_2505_exceeds_lifetime_shows_exceeded_and_excess() {
        let st = state_with(vec![gift_removal_labeled(
            1,
            date!(2025 - 06 - 01),
            dec!(100000),
            "Alice",
        )]);
        let tables = tables_with(2025, dec!(19000));
        let msg =
            render_gift_advisory(&st, 2025, dec!(13_950_000), &tables).expect("advisory expected");
        assert!(
            msg.contains("14031000.00"),
            "cumulative $14,031,000 must appear: {msg}"
        );
        assert!(
            msg.contains("EXCEEDED"),
            "must say EXCEEDED when over lifetime limit: {msg}"
        );
        // excess = 41,000
        assert!(
            msg.contains("41000.00"),
            "excess $41,000 must appear: {msg}"
        );
    }

    /// [KAT-B / R0-M2] Exact boundary — cumulative EXACTLY $13,990,000.
    /// Alice $100,000, prior = 13,990,000 − 81,000 = 13,909,000.
    /// remaining = $0; NOT "EXCEEDED" (strict `>`, not `>=`).
    #[test]
    fn section_2505_exact_boundary_remaining_zero_not_exceeded() {
        let st = state_with(vec![gift_removal_labeled(
            1,
            date!(2025 - 06 - 01),
            dec!(100000),
            "Alice",
        )]);
        let tables = tables_with(2025, dec!(19000));
        // prior = 13,990,000 − 81,000 = 13,909,000 → cumulative = 13,990,000 exactly
        let msg =
            render_gift_advisory(&st, 2025, dec!(13_909_000), &tables).expect("advisory expected");
        assert!(
            msg.contains("13990000.00"),
            "cumulative $13,990,000 must appear: {msg}"
        );
        // remaining = 0 — assert the exact phrasing so "13990000.00" cannot satisfy this
        assert!(
            msg.contains("($0.00 remaining)"),
            "remaining $0.00 in exact phrasing '($0.00 remaining)' must appear: {msg}"
        );
        // strict >: at exactly the limit, NOT exceeded
        assert!(
            !msg.contains("EXCEEDED"),
            "must NOT say EXCEEDED at exactly the limit: {msg}"
        );
    }

    /// [KAT-P4 / R0-M4] Prior-only edge — prior $5,000,000, all current donees under annual.
    /// Alice $10,000 gift (under $19k annual) → current taxable $0.
    /// cumulative = 5,000,000 + 0 = 5,000,000 > 0 → §2505 block SHOWS.
    #[test]
    fn section_2505_prior_only_block_shows_even_when_current_taxable_zero() {
        let st = state_with(vec![gift_removal_labeled(
            1,
            date!(2025 - 06 - 01),
            dec!(10000), // under $19k annual exclusion → current taxable = 0
            "Alice",
        )]);
        let tables = tables_with(2025, dec!(19000));
        let msg =
            render_gift_advisory(&st, 2025, dec!(5_000_000), &tables).expect("advisory expected");
        // cumulative = 5,000,000 (from prior; current taxable = 0)
        assert!(
            msg.contains("5000000.00"),
            "cumulative $5,000,000 must appear: {msg}"
        );
        assert!(
            msg.contains("§2505 lifetime (basic) exclusion"),
            "§2505 block must appear for prior-only case: {msg}"
        );
        assert!(!msg.contains("EXCEEDED"), "must NOT say EXCEEDED: {msg}");
    }

    /// [KAT-N] No taxable gifts → no §2505 block.
    /// Alice $10,000 (under annual), prior $0 → cumulative = $0 → no §2505 line.
    #[test]
    fn section_2505_no_block_when_cumulative_zero() {
        let st = state_with(vec![gift_removal_labeled(
            1,
            date!(2025 - 06 - 01),
            dec!(10000), // under $19k annual exclusion
            "Alice",
        )]);
        let tables = tables_with(2025, dec!(19000));
        let msg = render_gift_advisory(&st, 2025, dec!(0), &tables).expect("advisory expected");
        assert!(
            !msg.contains("§2505 lifetime"),
            "§2505 block must NOT appear when cumulative = 0: {msg}"
        );
    }

    /// [KAT-D0] Default $0 prior — no flag → prior $0 + the new caveats present (no stale Chunk-3).
    #[test]
    fn section_2505_default_zero_prior_shows_caveats() {
        let st = state_with(vec![gift_removal_labeled(
            1,
            date!(2025 - 06 - 01),
            dec!(100000), // taxable $81k
            "Alice",
        )]);
        let tables = tables_with(2025, dec!(19000));
        let msg = render_gift_advisory(&st, 2025, dec!(0), &tables).expect("advisory expected");
        // [I1] stale Chunk-3 caveat is ABSENT
        assert!(
            !msg.contains("later chunk (Chunk 3)"),
            "stale 'later chunk (Chunk 3)' must be absent: {msg}"
        );
        // New caveats present
        assert!(
            msg.contains("§2513 gift-splitting"),
            "§2513 caveat must be present: {msg}"
        );
        assert!(
            msg.contains("portability/DSUE"),
            "portability/DSUE caveat must be present: {msg}"
        );
        assert!(
            msg.contains("prior cumulative taxable gifts are user-supplied"),
            "prior-cumulative disclosure caveat must be present: {msg}"
        );
    }

    /// [KAT-I2] Mixed/unlabeled — Alice $100,000 (taxable $81k) + unlabeled $50,000.
    /// §2505 block shows used $81k AND the unlabeled-omission disclosure line.
    #[test]
    fn section_2505_mixed_shows_omission_disclosure_for_unlabeled() {
        let st = state_with(vec![
            gift_removal_labeled(1, date!(2025 - 03 - 01), dec!(100000), "Alice"),
            gift_removal(2, date!(2025 - 06 - 01), dec!(50000)), // unlabeled
        ]);
        let tables = tables_with(2025, dec!(19000));
        let msg = render_gift_advisory(&st, 2025, dec!(0), &tables).expect("advisory expected");
        // §2505 block shows used $81,000 (LABELED only)
        assert!(
            msg.contains("§2505 lifetime (basic) exclusion"),
            "§2505 block must appear: {msg}"
        );
        assert!(
            msg.contains("81000.00"),
            "used $81,000 (labeled only) must appear: {msg}"
        );
        // [I2] Unlabeled-omission disclosure in §2505 block
        assert!(
            msg.contains("§2505 consumption reflects LABELED-donee taxable gifts only"),
            "omission disclosure must appear: {msg}"
        );
        assert!(
            msg.contains("50000.00"),
            "unlabeled total $50,000 must appear in omission disclosure: {msg}"
        );
        assert!(
            msg.contains("consumption may be understated"),
            "under-stated warning must appear: {msg}"
        );
    }

    /// [KAT-I1] Absence — the stale "§2505 … later chunk (Chunk 3)" string is GONE from output.
    #[test]
    fn section_2505_stale_chunk3_caveat_is_absent() {
        let st = state_with(vec![gift_removal_labeled(
            1,
            date!(2025 - 06 - 01),
            dec!(20000),
            "Alice",
        )]);
        let tables = tables_with(2025, dec!(19000));
        let msg = render_gift_advisory(&st, 2025, dec!(0), &tables).expect("advisory expected");
        assert!(
            !msg.contains("later chunk (Chunk 3)"),
            "stale Chunk-3 caveat must be absent: {msg}"
        );
        assert!(
            !msg.contains("§2505 lifetime exemption is a later chunk"),
            "stale §2505 future-chunk phrase must be absent: {msg}"
        );
    }
}

#[cfg(test)]
mod schedule_se_tests {
    //! P2-D Task 2 / Chunk A + Chunk B KATs — `render_schedule_se` + `schedule_se.csv`.
    //! The rendered figures reuse hand-verified SeTaxResult fixtures (see btctax-core se.rs KATs).
    //! PRIVACY: synthetic values only.
    use super::*;
    use rust_decimal_macros::dec;

    /// Golden 1 SeTaxResult (Single, $100,000 business mining, no W-2, no expenses).
    fn golden1() -> SeTaxResult {
        SeTaxResult {
            net_se: dec!(100000),
            base: dec!(92350.00),
            ss: dec!(11451.40),
            medicare: dec!(2678.15),
            addl: dec!(0.00),
            total: dec!(14129.55),
            deductible_half: dec!(7064.78),
        }
    }

    /// [Chunk A] W-2 SeTaxResult: Single, mining $100k, w2_ss $150k, w2_medicare $150k.
    fn w2_headline() -> SeTaxResult {
        SeTaxResult {
            net_se: dec!(100000),
            base: dec!(92350.00),
            ss: dec!(3236.40),
            medicare: dec!(2678.15),
            addl: dec!(381.15),
            total: dec!(6295.70),
            deductible_half: dec!(2957.28),
        }
    }

    /// [Chunk A] Asymmetric SeTaxResult: w2_ss $150k, w2_medicare $0.
    fn w2_asymmetric() -> SeTaxResult {
        SeTaxResult {
            net_se: dec!(100000),
            base: dec!(92350.00),
            ss: dec!(3236.40),
            medicare: dec!(2678.15),
            addl: dec!(0.00),
            total: dec!(5914.55),
            deductible_half: dec!(2957.28),
        }
    }

    /// [Chunk B] Headline expenses SeTaxResult: Single, mining $100k, expenses $20k, no W-2.
    /// net_se = 80,000; base = 80,000 × 0.9235 = 73,880.00; ss = 12.4% × 73,880 = 9,161.12;
    /// medicare = 2.9% × 73,880 = 2,142.52; addl = 0; total = 11,303.64;
    /// deductible_half = (9,161.12 + 2,142.52)/2 = 5,651.82.
    fn expenses_headline() -> SeTaxResult {
        SeTaxResult {
            net_se: dec!(80000),
            base: dec!(73880.00),
            ss: dec!(9161.12),
            medicare: dec!(2142.52),
            addl: dec!(0.00),
            total: dec!(11303.64),
            deductible_half: dec!(5651.82),
        }
    }

    /// [Chunk B] W-2 + expenses SeTaxResult: Single, mining $100k, expenses $20k, w2_ss $150k,
    /// w2_medicare $150k.
    /// net_se = 80,000; base = 73,880.00; ss_cap = max(0, 176,100 − 150,000) = 26,100 →
    /// ss = 12.4% × min(73,880, 26,100) = 12.4% × 26,100 = 3,236.40;
    /// medicare = 2.9% × 73,880 = 2,142.52;
    /// addl_threshold = max(0, 200,000 − 150,000) = 50,000; over = 73,880 − 50,000 = 23,880 →
    /// addl = 0.9% × 23,880 = 214.92;
    /// total = 3,236.40 + 2,142.52 + 214.92 = 5,593.84;
    /// deductible_half = (3,236.40 + 2,142.52)/2 = 2,689.46.
    fn expenses_w2_combined() -> SeTaxResult {
        SeTaxResult {
            net_se: dec!(80000),
            base: dec!(73880.00),
            ss: dec!(3236.40),
            medicare: dec!(2142.52),
            addl: dec!(214.92),
            total: dec!(5593.84),
            deductible_half: dec!(2689.46),
        }
    }

    /// Business-mining year → full Schedule SE section: components + total + deductible half +
    /// [Chunk A] the $0-W-2 short note + the §164(f) advisory + the [D5] standalone note.
    /// [Chunk B] expenses $0 → "no Schedule C expenses supplied" note (old "not modeled" GONE).
    #[test]
    fn business_mining_year_renders_full_section() {
        let r = golden1();
        let s = render_schedule_se(
            2025,
            Some(&r),
            dec!(100000),
            true,
            Usd::ZERO,
            Usd::ZERO,
            Usd::ZERO,
        )
        .expect("SE section expected");
        // Components + total + §164(f) half.
        assert!(s.contains("92350.00"), "net SE earnings base: {s}");
        assert!(s.contains("11451.40"), "SS component: {s}");
        assert!(s.contains("2678.15"), "Medicare component: {s}");
        assert!(s.contains("14129.55"), "total SE tax: {s}");
        assert!(s.contains("7064.78"), "§164(f) deductible half: {s}");
        assert!(
            s.contains("Additional Medicare"),
            "addl component labeled: {s}"
        );
        // [Chunk A / R0-I2] NEW $0-W-2 short note present; old OVERSTATED/UNDERSTATED GONE.
        assert!(
            s.contains("$0 W-2 wages"),
            "short $0-W-2 note must appear (both W-2 = 0): {s}"
        );
        assert!(
            s.contains("--w2-ss-wages"),
            "$0 note must mention --w2-ss-wages flag: {s}"
        );
        assert!(
            !s.contains("OVERSTATED"),
            "old OVERSTATED text must be absent (Chunk A regression): {s}"
        );
        assert!(
            !s.contains("UNDERSTATED"),
            "old UNDERSTATED text must be absent (Chunk A regression): {s}"
        );
        // [Chunk A / R0-I3] §164(f) advisory present.
        assert!(
            s.contains("NOT auto-coordinated"),
            "§164(f) advisory must appear: {s}"
        );
        assert!(
            s.contains("coordinate it on your actual return"),
            "§164(f) advisory must include coordination instruction: {s}"
        );
        // [D5] standalone note.
        assert!(
            s.contains("SEPARATE federal liability"),
            "standalone note: {s}"
        );
        assert!(
            s.contains("not") && s.contains("§164(f)"),
            "notes §164(f) not auto-coordinated: {s}"
        );
        // [Chunk B] $0-expenses note replaces the old "not modeled" caveat.
        assert!(
            s.contains("no Schedule C expenses supplied"),
            "Chunk B $0-expenses note must appear: {s}"
        );
        assert!(
            s.contains("--schedule-c-expenses"),
            "$0 note must mention --schedule-c-expenses flag: {s}"
        );
        assert!(
            !s.contains("not modeled"),
            "old 'not modeled' caveat must be absent (replaced by Chunk B): {s}"
        );
    }

    /// [Chunk A / D3] When W-2 values are set, the coordinated disclosure appears with §1401(b)(2)(B).
    #[test]
    fn w2_set_renders_coordinated_disclosure() {
        let r = w2_headline();
        let s = render_schedule_se(
            2025,
            Some(&r),
            dec!(100000),
            true,
            Usd::ZERO,
            dec!(150000),
            dec!(150000),
        )
        .expect("SE section expected");
        // [D3] Coordinated text present.
        assert!(
            s.contains("W-2 coordination applied"),
            "coordinated disclosure must appear: {s}"
        );
        assert!(
            s.contains("§1401(b)(2)(B)"),
            "must cite §1401(b)(2)(B): {s}"
        );
        assert!(
            s.contains("Form 8959 Part II"),
            "must cite Form 8959 Part II: {s}"
        );
        // The W-2 amounts appear in the disclosure text.
        assert!(s.contains("150000"), "w2_ss_wages amount must appear: {s}");
        // Old OVERSTATED/UNDERSTATED text ABSENT even in W-2 mode (expenses = 0).
        assert!(!s.contains("OVERSTATED"), "OVERSTATED must be absent: {s}");
        assert!(
            !s.contains("UNDERSTATED"),
            "UNDERSTATED must be absent: {s}"
        );
        // Figures correct.
        assert!(s.contains("3236.40"), "reduced SS component: {s}");
        assert!(s.contains("381.15"), "non-zero addl: {s}");
        assert!(s.contains("6295.70"), "reduced total: {s}");
        assert!(s.contains("2957.28"), "deductible_half: {s}");
    }

    /// [Chunk A / I4] Asymmetric-W-2 transposition guard (render level): w2_ss $150k, w2_medicare $0 →
    /// ss == $3,236.40 AND addl == $0.00 in the rendered text.
    /// A swapped (w2_medicare, w2_ss) argument order at the call site would flip both values.
    #[test]
    fn w2_asymmetric_render_transposition_guard() {
        let r = w2_asymmetric();
        let s = render_schedule_se(
            2025,
            Some(&r),
            dec!(100000),
            true,
            Usd::ZERO,
            dec!(150000),
            Usd::ZERO,
        )
        .expect("SE section expected");
        // W-2 coordination text must appear (w2_ss > 0).
        assert!(
            s.contains("W-2 coordination applied"),
            "coordinated disclosure must appear: {s}"
        );
        // ss is reduced, addl is 0 — not transposed values.
        assert!(s.contains("3236.40"), "ss must be 3236.40 (reduced): {s}");
        assert!(
            s.contains("0.00"),
            "addl must be 0.00 (threshold un-reduced): {s}"
        );
        // The old OVERSTATED/UNDERSTATED is absent.
        assert!(!s.contains("OVERSTATED"), "{s}");
        assert!(!s.contains("UNDERSTATED"), "{s}");
    }

    /// No business SE income → no Schedule SE section (None). [gross_se == 0 path]
    #[test]
    fn no_business_income_no_section() {
        assert!(
            render_schedule_se(2025, None, Usd::ZERO, true, Usd::ZERO, Usd::ZERO, Usd::ZERO)
                .is_none()
        );
    }

    /// Business SE income present but no bundled table → the "SS wage base unavailable" note (no
    /// silent drop). [gross_se > 0 && !table_present path]
    #[test]
    fn business_income_but_no_table_emits_note() {
        let s = render_schedule_se(
            2099,
            None,
            dec!(100000),
            false,
            Usd::ZERO,
            Usd::ZERO,
            Usd::ZERO,
        )
        .expect("wage-base-unavailable note expected");
        assert!(s.contains("SS wage base unavailable"), "{s}");
        assert!(s.contains("2099"), "names the year: {s}");
        assert!(s.contains("no silent drop"), "{s}");
    }

    // ── Chunk B golden KATs ────────────────────────────────────────────────────────────────────

    /// [Chunk B] Headline: expenses $20k, no W-2 → breakout line + Schedule C advisory.
    /// Verifies: gross = net_se + expenses shown, advisory text present, NO old "not modeled" caveat.
    #[test]
    fn expenses_20k_no_w2_renders_breakout_and_advisory() {
        let r = expenses_headline(); // net_se = 80,000; expenses = 20,000 → gross = 100,000
        let s = render_schedule_se(
            2025,
            Some(&r),
            dec!(100000), // gross_se
            true,
            dec!(20000), // schedule_c_expenses
            Usd::ZERO,
            Usd::ZERO,
        )
        .expect("SE section expected");
        // Breakout line: gross − expenses = net SE
        assert!(
            s.contains("gross business income"),
            "breakout line must appear: {s}"
        );
        assert!(
            s.contains("100000.00"),
            "gross ($100k) must appear in breakout: {s}"
        );
        assert!(
            s.contains("20000.00"),
            "expenses ($20k) must appear in breakout: {s}"
        );
        assert!(
            s.contains("80000.00"),
            "net_se ($80k) must appear in breakout: {s}"
        );
        // Schedule C advisory: OVERSTATES text present; NO OTI-edit prescription.
        assert!(
            s.contains("OVERSTATES"),
            "Schedule C advisory OVERSTATES text: {s}"
        );
        assert!(
            s.contains("ORDINARY taxable income"),
            "advisory must mention ORDINARY taxable income: {s}"
        );
        assert!(
            s.contains("engine-side coordination is deferred"),
            "advisory must mention deferred coordination: {s}"
        );
        // NO OTI-edit prescription: must NOT say "reduce your ordinary_taxable_income" (spec D3).
        assert!(
            !s.contains("reduce your ordinary_taxable_income"),
            "NO OTI-edit prescription allowed (spec D3): {s}"
        );
        assert!(
            !s.contains("set --ordinary-taxable-income"),
            "NO OTI-edit prescription allowed (spec D3): {s}"
        );
        // Golden figures: base, ss, medicare, total, deductible_half.
        assert!(s.contains("73880.00"), "base $73,880: {s}");
        assert!(s.contains("9161.12"), "ss $9,161.12: {s}");
        assert!(s.contains("2142.52"), "medicare $2,142.52: {s}");
        assert!(s.contains("11303.64"), "total $11,303.64: {s}");
        assert!(s.contains("5651.82"), "deductible_half $5,651.82: {s}");
        // Old "not modeled" caveat is ABSENT.
        assert!(
            !s.contains("not modeled"),
            "old 'not modeled' caveat must be absent: {s}"
        );
    }

    /// [Chunk B / R0-I1] Fully expensed (gross > 0, table present, net_se == 0) → the NEW
    /// "fully expensed" line; the "SS wage base unavailable" note ABSENT.
    #[test]
    fn fully_expensed_shows_new_line_not_wage_base_note() {
        // mining $10,000, expenses $15,000 → net_se = 0 → compute_se_tax returns None.
        // Render with gross_se = 10,000 and table_present = true.
        let s = render_schedule_se(
            2025,
            None,
            dec!(10000), // gross_se
            true,        // table_present = true
            dec!(15000), // schedule_c_expenses
            Usd::ZERO,
            Usd::ZERO,
        )
        .expect("fully-expensed section expected (not None)");
        // The new "fully expensed" line is present.
        assert!(
            s.contains("fully expensed"),
            "fully-expensed line must appear: {s}"
        );
        assert!(
            s.contains("10000.00"),
            "gross $10k must appear in fully-expensed line: {s}"
        );
        assert!(
            s.contains("15000.00"),
            "expenses $15k must appear in fully-expensed line: {s}"
        );
        assert!(
            s.contains("no §1401 SE tax"),
            "must state no SE tax owed: {s}"
        );
        assert!(s.contains("2025"), "must name the year: {s}");
        // The "SS wage base unavailable" note is ABSENT (negative assertion per [R0-I1]).
        assert!(
            !s.contains("SS wage base unavailable"),
            "wage-base-unavailable note must be ABSENT for fully-expensed case: {s}"
        );
    }

    /// [Chunk B] W-2 + expenses combined render: breakout and W-2 coordination both appear.
    #[test]
    fn expenses_w2_combined_renders_both() {
        let r = expenses_w2_combined(); // net_se = 80,000; gross = 100,000; expenses = 20,000
        let s = render_schedule_se(
            2025,
            Some(&r),
            dec!(100000),
            true,
            dec!(20000),  // schedule_c_expenses
            dec!(150000), // w2_ss_wages
            dec!(150000), // w2_medicare_wages
        )
        .expect("SE section expected");
        // Breakout line.
        assert!(s.contains("gross business income"), "breakout line: {s}");
        assert!(s.contains("80000.00"), "net_se in breakout: {s}");
        // Schedule C advisory.
        assert!(s.contains("OVERSTATES"), "Schedule C advisory: {s}");
        // W-2 coordination also present.
        assert!(
            s.contains("W-2 coordination applied"),
            "W-2 coordination: {s}"
        );
        // Figures correct.
        assert!(s.contains("73880.00"), "base: {s}");
        assert!(s.contains("3236.40"), "ss (reduced by W-2 cap): {s}");
        assert!(s.contains("2142.52"), "medicare: {s}");
        assert!(s.contains("214.92"), "addl: {s}");
        assert!(s.contains("5593.84"), "total: {s}");
        assert!(s.contains("2689.46"), "deductible_half: {s}");
    }

    /// `schedule_se.csv` columns + values (year-scoped; written when a SeTaxResult exists).
    #[test]
    fn schedule_se_csv_columns_and_values() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("export");
        let st = LedgerState::default();
        let r = golden1();
        write_csv_exports(&out, &st, Some(2025), Some(&r), &BTreeMap::new()).unwrap();

        let path = out.join("schedule_se.csv");
        assert!(path.exists(), "schedule_se.csv must be written");
        let mut rdr = csv::Reader::from_reader(std::fs::File::open(&path).unwrap());
        let headers: Vec<String> = rdr.headers().unwrap().iter().map(String::from).collect();
        assert_eq!(
            headers,
            vec![
                "net_se_earnings",
                "se_base_9235",
                "ss_component",
                "medicare_component",
                "additional_medicare_component",
                "total_se_tax",
                "deductible_half",
            ]
        );
        let rec = rdr
            .records()
            .next()
            .expect("one data row")
            .expect("readable");
        assert_eq!(&rec[0], "100000"); // net_se_earnings
        assert_eq!(&rec[1], "92350.00"); // se_base_9235
        assert_eq!(&rec[2], "11451.40"); // ss_component
        assert_eq!(&rec[3], "2678.15"); // medicare_component
        assert_eq!(&rec[4], "0.00"); // additional_medicare_component
        assert_eq!(&rec[5], "14129.55"); // total_se_tax
        assert_eq!(&rec[6], "7064.78"); // deductible_half
    }

    /// No SeTaxResult → schedule_se.csv is NOT written (nothing to file; also covers fully-expensed).
    #[test]
    fn schedule_se_csv_omitted_when_no_se_tax() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("export");
        let st = LedgerState::default();
        write_csv_exports(&out, &st, Some(2025), None, &BTreeMap::new()).unwrap();
        assert!(!out.join("schedule_se.csv").exists());
    }
}

#[cfg(test)]
mod form8283_csv_tests {
    //! P2-C / Chunk-3b Task 2 unit KATs — `write_form8283_csv` Part III/IV detail columns.
    //! Direct-state fixtures; pure unit (no vault). PRIVACY: synthetic values only.
    use super::*;

    /// form8283.csv — new Part III/IV detail columns populated when details are present.
    #[test]
    fn form8283_csv_detail_columns_present_and_empty() {
        use btctax_core::{
            BasisSource, DonationDetails, EventId, LedgerState, Removal, RemovalKind, RemovalLeg,
            Term,
        };
        use time::macros::date;

        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("export");

        // Build a minimal state with one Section-B donation.
        let event = EventId::decision(99);
        let leg = RemovalLeg {
            lot_id: btctax_core::LotId {
                origin_event_id: event.clone(),
                split_sequence: 0,
            },
            sat: 100_000_000,
            basis: rust_decimal::Decimal::ZERO,
            fmv_at_transfer: rust_decimal::Decimal::from(52000),
            term: Term::LongTerm,
            basis_source: BasisSource::ComputedFromCost,
            acquired_at: date!(2025 - 01 - 01),
            pseudo: false,
        };
        let removal = Removal {
            event: event.clone(),
            kind: RemovalKind::Donation,
            removed_at: date!(2025 - 03 - 01),
            legs: vec![leg],
            appraisal_required: false,
            donor_acquired_at: None,
            claimed_deduction: Some(rust_decimal::Decimal::from(52000)),
            donee: Some("Test Charity Two".into()),
        };
        // N1: second removal with NO details in dmap — locks the empty-half of the 6 new columns.
        let event2 = EventId::decision(100);
        let leg2 = RemovalLeg {
            lot_id: btctax_core::LotId {
                origin_event_id: event2.clone(),
                split_sequence: 0,
            },
            sat: 10_000_000,
            basis: rust_decimal::Decimal::ZERO,
            fmv_at_transfer: rust_decimal::Decimal::from(8000),
            term: Term::LongTerm,
            basis_source: BasisSource::ComputedFromCost,
            acquired_at: date!(2025 - 01 - 15),
            pseudo: false,
        };
        let removal2 = Removal {
            event: event2.clone(),
            kind: RemovalKind::Donation,
            removed_at: date!(2025 - 05 - 01),
            legs: vec![leg2],
            appraisal_required: false,
            donor_acquired_at: None,
            claimed_deduction: Some(rust_decimal::Decimal::from(8000)),
            donee: Some("No Details Org".into()),
        };

        let st = LedgerState {
            removals: vec![removal, removal2],
            ..Default::default()
        };

        let mut dmap: BTreeMap<EventId, DonationDetails> = BTreeMap::new();
        dmap.insert(
            event,
            DonationDetails {
                donee_name: "Test Charity".into(),
                donee_ein: Some("12-3456789".into()),
                donee_address: Some("123 Main".into()),
                appraiser_name: "Test Appraiser".into(),
                appraiser_tin: Some("987654321".into()),
                appraiser_ptin: Some("P01234567".into()),
                appraiser_qualifications: Some("Certified".into()),
                appraisal_date: Some(date!(2025 - 06 - 01)),
                appraiser_address: None,
                fmv_method_override: None,
            },
        );
        // event2 intentionally NOT inserted — exercises the empty-column path.

        write_csv_exports(&out, &st, Some(2025), None, &dmap).unwrap();

        let path = out.join("form8283.csv");
        assert!(path.exists(), "form8283.csv must exist");

        let mut rdr = csv::ReaderBuilder::new()
            .comment(Some(b'#'))
            .from_path(&path)
            .unwrap();
        let headers: Vec<String> = rdr.headers().unwrap().iter().map(String::from).collect();
        let idx = |name: &str| {
            headers
                .iter()
                .position(|h| h == name)
                .unwrap_or_else(|| panic!("header {name} not found"))
        };
        // Collect both rows. form_8283 sorts by (removed_at, event, lot_id):
        //   records[0] = removal (removed_at 2025-03-01, event decision(99)) — WITH details.
        //   records[1] = removal2 (removed_at 2025-05-01, event decision(100)) — NO details.
        let all_recs: Vec<csv::StringRecord> = rdr.records().map(|r| r.unwrap()).collect();
        assert_eq!(
            all_recs.len(),
            2,
            "must have exactly two data rows (one per removal)"
        );
        let rec = &all_recs[0];
        let no_details_rec = &all_recs[1];

        // WITH-details half: all 6 new columns populated.
        assert_eq!(&rec[idx("donee")], "Test Charity");
        assert_eq!(&rec[idx("appraiser")], "Test Appraiser");
        assert_eq!(&rec[idx("donee_ein")], "12-3456789");
        assert_eq!(&rec[idx("donee_address")], "123 Main");
        assert_eq!(&rec[idx("appraiser_tin")], "987654321");
        assert_eq!(&rec[idx("appraiser_ptin")], "P01234567");
        assert_eq!(&rec[idx("appraiser_qualifications")], "Certified");
        assert_eq!(&rec[idx("appraisal_date")], "2025-06-01");
        assert_eq!(&rec[idx("needs_review")], "false");

        // N1: EMPTY half — no-details removal has all 6 new columns blank.
        assert_eq!(
            &no_details_rec[idx("donee_ein")],
            "",
            "no-details row: donee_ein must be empty"
        );
        assert_eq!(
            &no_details_rec[idx("donee_address")],
            "",
            "no-details row: donee_address must be empty"
        );
        assert_eq!(
            &no_details_rec[idx("appraiser_tin")],
            "",
            "no-details row: appraiser_tin must be empty"
        );
        assert_eq!(
            &no_details_rec[idx("appraiser_ptin")],
            "",
            "no-details row: appraiser_ptin must be empty"
        );
        assert_eq!(
            &no_details_rec[idx("appraiser_qualifications")],
            "",
            "no-details row: appraiser_qualifications must be empty"
        );
        assert_eq!(
            &no_details_rec[idx("appraisal_date")],
            "",
            "no-details row: appraisal_date must be empty"
        );
        assert_eq!(
            &no_details_rec[idx("needs_review")],
            "true",
            "no-details carrier row: needs_review must be true"
        );
    }
}

/// UX-P4-11: one row of `events list` — a decidable event and its decision status. The `reff` is the
/// canonical event reference (`EventId::canonical()`) a `reconcile` verb accepts verbatim.
pub struct EventRow {
    /// The canonical event ref (pasteable into a reconcile verb). Named `reff` — `ref` is reserved.
    pub reff: String,
    /// A stable human kind tag: transfer-in | transfer-out | unclassified | import-conflict | income.
    pub kind: &'static str,
    /// The event's tax-timezone calendar date.
    pub date: TaxDate,
    /// Principal sats, when the payload carries a structured amount (None for unclassified/conflict).
    pub sat: Option<btctax_core::Sat>,
    /// USD value at the event-date close (stored FMV for income; else priced), when resolvable.
    pub usd: Option<Usd>,
    /// `Some("decision|N")` when a live (non-voided) decision targets this event; `None` = still
    /// decidable (a pseudo-defaulted event is decidable — its default is never persisted).
    pub decision_ref: Option<String>,
}

/// Format sats as a BTC amount with 8 decimals (integer math — no float).
fn fmt_btc(sat: btctax_core::Sat) -> String {
    let whole = sat / 100_000_000;
    let frac = (sat % 100_000_000).unsigned_abs();
    format!("{whole}.{frac:08}")
}

/// UX-P4-11: render the `events list` table. Ref-first per row (so it is trivially copyable), then
/// kind @ date, amount, and the bracketed decision status. Read-only display.
pub fn render_events_list(rows: &[EventRow]) -> String {
    let mut out = String::new();
    if rows.is_empty() {
        let _ = writeln!(out, "No decidable events.");
        return out;
    }
    let decided = rows.iter().filter(|r| r.decision_ref.is_some()).count();
    let _ = writeln!(
        out,
        "Decidable events — {} ({} decided, {} open):",
        rows.len(),
        decided,
        rows.len() - decided
    );
    for r in rows {
        let amount = match (r.sat, r.usd) {
            (Some(s), Some(u)) => format!("{} BTC (~${})", fmt_btc(s), fmt_money(u)),
            (Some(s), None) => format!("{} BTC", fmt_btc(s)),
            (None, _) => "—".to_string(),
        };
        let status = match &r.decision_ref {
            Some(d) => format!("[decided: {d}]"),
            None => "[decidable]".to_string(),
        };
        let _ = writeln!(
            out,
            "  {}  {} @ {}  {}  {}",
            r.reff, r.kind, r.date, amount, status
        );
    }
    out
}

#[cfg(test)]
mod advisory_wrap_tests {
    use super::*;

    /// `p5-n5-advisory-line-wrapping`: an advisory is a 300–400-character sentence, and the house style
    /// wraps everywhere else. An unwrapped one is unreadable in an 80-column terminal — and it is the ONE
    /// place the tool explains a conservative omission, so it is the text most worth reading.
    #[test]
    fn advisories_wrap_to_the_house_width_with_a_hanging_indent() {
        use btctax_core::tax::advisories::Advisory;
        let out = render_advisories(&[Advisory::CtcOdcOmitted { dependents: 2 }]);

        for line in out.lines() {
            assert!(
                line.chars().count() <= ADVISORY_WRAP_COLS,
                "line is {} cols, over the {}-col house width: {line:?}",
                line.chars().count(),
                ADVISORY_WRAP_COLS
            );
        }
        // Continuation lines hang under the bullet's TEXT, not under the bullet.
        assert!(
            out.lines()
                .any(|l| l.starts_with("    ") && !l.trim().is_empty()),
            "a 300-char advisory must wrap onto continuation lines, got:\n{out}"
        );
    }
}

#[cfg(test)]
mod events_list_render_tests {
    use super::*;
    use time::macros::date;

    fn row(reff: &str, kind: &'static str, decision_ref: Option<&str>) -> EventRow {
        EventRow {
            reff: reff.to_owned(),
            kind,
            date: date!(2025 - 03 - 01),
            sat: Some(5_000_000),
            usd: Some(rust_decimal_macros::dec!(4271.78)),
            decision_ref: decision_ref.map(str::to_owned),
        }
    }

    /// Empty → an explicit "none" line (never a blank rendering).
    #[test]
    fn empty_renders_a_none_line() {
        assert_eq!(render_events_list(&[]), "No decidable events.\n");
    }

    /// Each row is ref-FIRST (trivially copyable), carries kind/date/BTC(+USD), and a bracketed status:
    /// `[decidable]` when open, `[decided: decision|N]` when a decision targets it.
    #[test]
    fn rows_are_ref_first_with_bracketed_status() {
        let out = render_events_list(&[
            row("import|coinbase|in|cb-recv", "transfer-in", None),
            row(
                "import|coinbase|out|cb-send",
                "transfer-out",
                Some("decision|1"),
            ),
        ]);
        let lines: Vec<&str> = out.lines().collect();
        assert!(
            lines[0].contains("2 (1 decided, 1 open)"),
            "header: {}",
            lines[0]
        );
        // ref is the first whitespace token on each row (the paste contract).
        assert_eq!(
            lines[1].split_whitespace().next(),
            Some("import|coinbase|in|cb-recv")
        );
        assert!(lines[1].contains("[decidable]"), "open row: {}", lines[1]);
        assert!(
            lines[1].contains("0.05000000 BTC") && lines[1].contains("4271.78"),
            "amount: {}",
            lines[1]
        );
        assert!(
            lines[2].contains("[decided: decision|1]"),
            "decided row: {}",
            lines[2]
        );
    }
}

#[cfg(test)]
mod holdings_pending_tests {
    //! UX-P4-6 — the holdings view shows a BTC-unit pending line when sats sit unreconciled, and
    //! hides it on a reconciled ledger. `report` otherwise never mentioned pending (only `verify` did).
    use super::*;
    use btctax_core::state::PendingTransfer;
    use btctax_core::EventId;

    #[test]
    fn holdings_pending_line_shows_in_btc_and_hides_when_reconciled() {
        let mut pending = LedgerState::default();
        pending.stats.sigma_pending = 3_000_000; // 0.03 BTC unreconciled
        pending.pending_reconciliation = vec![PendingTransfer {
            event: EventId::decision(1),
            principal_sat: 3_000_000,
            fee_sat: None,
            legs: vec![],
        }];
        let shown = render_report(&pending, None);
        assert!(shown.contains("Pending:"), "pending line present: {shown}");
        assert!(shown.contains("0.03000000 BTC"), "BTC unit: {shown}");
        assert!(
            shown.contains("1 unreconciled transfer"),
            "names the count (singular): {shown}"
        );
        assert!(shown.contains("verify"), "points at `verify`: {shown}");

        // Reconciled ledger: no pending sats → no pending line at all.
        let reconciled = LedgerState::default();
        let hidden = render_report(&reconciled, None);
        assert!(
            !hidden.contains("Pending:"),
            "no pending line when reconciled: {hidden}"
        );
    }

    #[test]
    fn holdings_pending_line_pluralizes_multiple_transfers() {
        let mut pending = LedgerState::default();
        pending.stats.sigma_pending = 150_000_000; // 1.5 BTC
        pending.pending_reconciliation = vec![
            PendingTransfer {
                event: EventId::decision(1),
                principal_sat: 100_000_000,
                fee_sat: None,
                legs: vec![],
            },
            PendingTransfer {
                event: EventId::decision(2),
                principal_sat: 50_000_000,
                fee_sat: None,
                legs: vec![],
            },
        ];
        let shown = render_report(&pending, None);
        assert!(
            shown.contains("2 unreconciled transfers"),
            "plural: {shown}"
        );
        assert!(shown.contains("1.50000000 BTC"), "{shown}");
    }
}

#[cfg(test)]
mod decision_class_tests {
    //! UX-P4-7 — the shared SCREEN-ONLY human formatter for decision-payload class fields, replacing
    //! the raw `{:?}` Debug dumps (`SelfTransferMine { basis: Some(19000.00), acquired_at:
    //! Some(2026-01-01) }`) that the CLI bulk-void preview + TUI void list truncated mid-field.
    use super::*;
    use btctax_core::{DisposeKind, InboundClass, IncomeKind, OutflowClass};
    use rust_decimal_macros::dec;
    use time::macros::date;

    #[test]
    fn inbound_self_transfer_mine_is_human_no_debug_struct() {
        let s = describe_inbound_class(&InboundClass::SelfTransferMine {
            basis: Some(dec!(19000)),
            acquired_at: Some(date!(2026 - 01 - 01)),
        });
        assert!(s.contains("self-transfer"), "{s}");
        assert!(s.contains("$19000.00"), "names the basis in $: {s}");
        assert!(s.contains("2026-01-01"), "names the acquired date: {s}");
        assert!(!s.contains('{'), "no Debug struct braces: {s}");
        assert!(!s.contains("Some("), "no Debug Option wrapper: {s}");
    }

    #[test]
    fn inbound_self_transfer_mine_defaults_are_named_not_none() {
        let s = describe_inbound_class(&InboundClass::SelfTransferMine {
            basis: None,
            acquired_at: None,
        });
        assert!(s.contains("self-transfer"), "{s}");
        assert!(
            s.matches("default").count() >= 2,
            "None basis AND date read as 'default': {s}"
        );
        assert!(!s.contains("None"), "no Debug None: {s}");
    }

    #[test]
    fn inbound_income_names_kind_fmv_business() {
        let s = describe_inbound_class(&InboundClass::Income {
            kind: IncomeKind::Mining,
            fmv: Some(dec!(500)),
            business: true,
        });
        assert!(s.contains("income"), "{s}");
        assert!(s.contains("mining"), "names the income kind: {s}");
        assert!(s.contains("$500.00"), "names the fmv: {s}");
        assert!(s.contains("business"), "flags business: {s}");
        assert!(!s.contains('{'), "{s}");
    }

    #[test]
    fn inbound_gift_received_names_fmv() {
        let s = describe_inbound_class(&InboundClass::GiftReceived {
            donor_basis: Some(dec!(1000)),
            donor_acquired_at: Some(date!(2024 - 05 - 05)),
            fmv_at_gift: dec!(30000),
        });
        assert!(s.contains("gift"), "{s}");
        assert!(s.contains("$30000.00"), "names the FMV at gift: {s}");
        assert!(!s.contains('{'), "{s}");
    }

    #[test]
    fn outflow_classes_are_human() {
        assert_eq!(
            describe_outflow_class(&OutflowClass::Dispose {
                kind: DisposeKind::Sell
            }),
            "sell"
        );
        assert_eq!(
            describe_outflow_class(&OutflowClass::Dispose {
                kind: DisposeKind::Spend
            }),
            "spend"
        );
        assert_eq!(describe_outflow_class(&OutflowClass::GiftOut), "gift");
        let donate = describe_outflow_class(&OutflowClass::Donate {
            appraisal_required: true,
        });
        assert!(
            donate.contains("donate") && donate.contains("appraisal"),
            "{donate}"
        );
        assert_eq!(
            describe_outflow_class(&OutflowClass::Donate {
                appraisal_required: false
            }),
            "donate"
        );
    }
}
