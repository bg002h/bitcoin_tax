//! Text rendering of CLI outputs (FR9 verify, FR4 report/show) + FR10 CSV export. Pure string-building
//! over engine data — the CLI displays; the engine computes (NFR4/NFR5).
use crate::config::CliConfig;
use btctax_adapters::FileReport;
use btctax_core::conventions::{tax_date, TRANSITION_DATE};
use btctax_core::persistence::ImportReport;
use btctax_core::{
    conservation_report, disposal_compliance, BasisSource, Blocker, BlockerKind,
    ConservationReport, DisposalCompliance, DisposalLeg, DisposeKind, EventId, EventPayload,
    GiftZone, IncomeKind, LedgerEvent, LedgerState, LotMethod, RemovalKind, RemovalLeg, Severity,
    TaxDate, Term, WalletId,
};
use btctax_store::fsperms;
use csv::Writer;
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::Path;

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
            "  {}#{} {} remaining {} sat | basis {} ({}){}",
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
            }
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
            let _ = writeln!(
                out,
                "  {} @ {} ({})",
                removal_kind_tag(r.kind),
                r.removed_at,
                r.event.canonical()
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
                "  {} @ {} {} sat = {} USD{}",
                income_kind_tag(i.kind),
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
        .map(|z| format!(" gift-zone {}", gift_zone_tag(z)))
        .unwrap_or_default();
    let _ = writeln!(
        out,
        "    {} sat: proceeds {} basis {} gain {} {}{}",
        leg.sat,
        leg.proceeds,
        leg.basis,
        leg.gain,
        term_tag(leg.term),
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
        term_tag(leg.term)
    );
}

// ── FR9 verify ──────────────────────────────────────────────────────────────────────────────────

/// Stable display tag for `LotMethod` (FIFO/LIFO/HIFO — uppercase, human-readable).
fn lot_method_display(m: LotMethod) -> &'static str {
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
    /// "in force" | "voided" | "backdated/ignored"
    pub note: &'static str,
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

pub fn build_verify(state: &LedgerState, events: &[LedgerEvent], cli: &CliConfig) -> VerifyReport {
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
    let voided: BTreeSet<EventId> = events
        .iter()
        .filter_map(|e| match &e.payload {
            EventPayload::VoidDecisionEvent(v) => Some(v.target_event_id.clone()),
            _ => None,
        })
        .collect();

    // Build election history (NFR4: sorted by decision_seq for a stable total order).
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

    let elections: Vec<ElectionLine> = election_events
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
                note,
            }
        })
        .collect();

    // Count non-voided LotSelection decisions.
    let selection_count = events
        .iter()
        .filter(|e| matches!(e.payload, EventPayload::LotSelection(_)) && !voided.contains(&e.id))
        .count();

    // Per-disposal compliance (§A.5): side-effect-free projection.
    let compliance = disposal_compliance(events, state);

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
    }
}

/// FR10: write the projected ledger as CSV (the NFR2 plaintext exception). One row per disposal/removal
/// leg (flattened) + one per lot/income record. Exact values (Decimal/i64) as strings (NFR5).
/// Each CSV is opened via `fsperms::open_owner_only` (0o600 on Unix) so decrypted PII matches the
/// hardened permissions already applied to `snapshot.sqlite` by the store crate. The out-dir is
/// created owner-only (0o700) if absent; when the dir PRE-EXISTS, open_owner_only still forces 0o600
/// on each new CSV file (the hole that `Writer::from_path` + umask would leave).
pub fn write_csv_exports(out_dir: &Path, state: &LedgerState) -> Result<(), crate::CliError> {
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
            ])?;
        }
    }
    w.flush()?;

    let mut w = Writer::from_writer(fsperms::open_owner_only(&out_dir.join("removals.csv"))?);
    w.write_record([
        "event",
        "kind",
        "removed_at",
        "lot",
        "sat",
        "basis",
        "fmv_at_transfer",
        "term",
    ])?;
    for r in &state.removals {
        for leg in &r.legs {
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
    Ok(())
}

/// Task 9 (B.5) + Task 10 (M4): render the `TaxOutcome` for `report --tax-year <y>`. Exact Decimal
/// Display; no float (NFR5). B-M2 fold: surfaces the ordinary-rate attributable delta so the three
/// printed attributable components visibly reconcile to `total_federal_tax_attributable`.
///
/// `advisory` is the optional M4 carryforward-consistency warning string (Task 10). When `Some`,
/// it is printed as a non-gating advisory line that does not affect the exit code.
pub fn render_tax_outcome(
    year: i32,
    out: &btctax_core::TaxOutcome,
    advisory: Option<&str>,
) -> String {
    use btctax_core::TaxOutcome::*;
    let mut s = String::new();
    let _ = writeln!(s, "Federal tax attributable to crypto — tax year {year}");
    match out {
        NotComputable(b) => {
            let _ = writeln!(s, "  NOT COMPUTABLE [{:?}]: {}", b.kind, b.detail);
        }
        Computed(r) => {
            let _ = writeln!(
                s,
                "  net short-term: {}   net long-term: {}",
                r.st_net, r.lt_net
            );
            let _ = writeln!(
                s,
                "  crypto ordinary income (level): {}",
                r.ordinary_from_crypto
            );
            // B-M2: surface the ordinary-rate attributable DELTA so the three attributable components
            // visibly reconcile to TOTAL. By the pinned identity this equals (ord_with − ord_without) exactly.
            let ordinary_rate_attributable = r.total_federal_tax_attributable - r.ltcg_tax - r.niit;
            let _ = writeln!(
                s,
                "  ordinary-rate tax (attributable): {}",
                ordinary_rate_attributable
            );
            let _ = writeln!(
                s,
                "  LTCG tax (attributable): {}   NIIT (attributable): {}",
                r.ltcg_tax, r.niit
            );
            let _ = writeln!(
                s,
                "  TOTAL federal tax attributable to crypto (delta): {}   \
                (= ordinary-rate + LTCG + NIIT attributable)",
                r.total_federal_tax_attributable
            );
            let _ = writeln!(
                s,
                "  §1211 loss deduction (level): {}   carryforward out: short {} / long {}",
                r.loss_deduction, r.carryforward_out.short, r.carryforward_out.long
            );
            let _ = writeln!(
                s,
                "  marginal rates: ordinary {} / LTCG {} / NIIT {}",
                r.marginal_rates.ordinary, r.marginal_rates.ltcg, r.marginal_rates.niit_applies
            );
            let _ = writeln!(
                s,
                "  (incremental ceteris-paribus delta on the minimal profile; \
                excludes AGI-driven SS/IRMAA/AMT/QBI/phaseout effects — I5. NIIT uses a minimal NII model \
                — excludes crypto ordinary income from NII and does not reduce NII by the allowed §1211 \
                loss — so it MAY UNDERSTATE NIIT; see §5 Phase-2 refinement.)"
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
            "  {} @ {} :: {:?}",
            c.disposal.canonical(),
            c.date,
            c.status
        );
    }
    out
}
