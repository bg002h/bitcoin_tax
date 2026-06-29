//! Text rendering of CLI outputs (FR9 verify, FR4 report/show) + FR10 CSV export. Pure string-building
//! over engine data — the CLI displays; the engine computes (NFR4/NFR5).
use btctax_adapters::FileReport;
use btctax_core::persistence::ImportReport;
use btctax_core::{
    conservation_report, BasisSource, Blocker, BlockerKind, ConservationReport, DisposalLeg,
    DisposeKind, GiftZone, IncomeKind, LedgerEvent, LedgerState, RemovalKind, RemovalLeg, Severity,
    Term, WalletId,
};
use btctax_store::fsperms;
use csv::Writer;
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
    out
}
