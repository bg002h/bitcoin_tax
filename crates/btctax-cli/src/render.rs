//! Text rendering of CLI outputs (FR9 verify, FR4 report/show) + FR10 CSV export. Pure string-building
//! over engine data — the CLI displays; the engine computes (NFR4/NFR5).
use crate::config::CliConfig;
use btctax_adapters::FileReport;
use btctax_core::conventions::{tax_date, TRANSITION_DATE};
use btctax_core::persistence::ImportReport;
use btctax_core::{
    conservation_report, disposal_compliance, form_8283, form_8949, schedule_d, BasisSource,
    Blocker, BlockerKind, ComplianceStatus, ConservationReport, DisposalCompliance, DisposalLeg,
    DisposeKind, EventId, EventPayload, Form8283HowAcquired, Form8283Section, Form8949Box,
    Form8949Part, GiftZone, IncomeKind, LedgerEvent, LedgerState, LotMethod, RemovalKind,
    RemovalLeg, ScheduleDTotals, SeTaxResult, Severity, TaxDate, Term, WalletId,
};
use btctax_store::fsperms;
use csv::Writer;
use std::collections::BTreeSet;
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

/// Stable Form 8949 part tag: "ST" (Part I) / "LT" (Part II). STABLE — part of the form8949.csv contract.
fn form8949_part_tag(p: Form8949Part) -> &'static str {
    match p {
        Form8949Part::ShortTerm => "ST",
        Form8949Part::LongTerm => "LT",
    }
}

/// Stable Form 8949 box tag: "C" (ST) / "F" (LT) — the conservative "not reported on a 1099-B"
/// default (D4). We never emit A/B/D/E (the model carries no 1099-B / basis-reported signal).
fn form8949_box_tag(b: Form8949Box) -> &'static str {
    match b {
        Form8949Box::C => "C",
        Form8949Box::F => "F",
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
                "  {} @ {} {} sat = {} USD{}",
                income_kind_tag(i.kind),
                i.recognized_at,
                i.sat,
                i.usd_fmv,
                if i.business { " [business]" } else { "" }
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
    // Note: a `Decision`-id guard is intentionally omitted — `LotSelection` payloads are
    // exclusively carried by `EventId::Decision` events (appended via `append_decision` in the
    // CLI); filtering by payload alone is equivalent and sufficient.
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
    ])?;
    for r in &state.removals {
        let deduction_first = r
            .claimed_deduction
            .map(|d| d.to_string())
            .unwrap_or_default();
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
        write_form8283_csv(out_dir, state, year)?;
        // P2-D: standalone Schedule SE §1401 figure — written only when there IS SE tax (a computed
        // SeTaxResult); omitted when the year has no business SE income (nothing to file).
        if let Some(se) = se_result {
            write_schedule_se_csv(out_dir, se)?;
        }
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

/// The standing [R0-I1] Section A/B aggregation caveat — emitted as the first (comment) line of
/// form8283.csv and reused by any text/advisory path. §170(f)(11)(F) aggregates similar items across
/// the year; this app's split is per-donation, so a set of small Section-A donations may in AGGREGATE
/// require Section B + a qualified appraisal (CCA 202302012 confirms this applies to crypto).
pub const FORM_8283_AGGREGATION_CAVEAT: &str = "Section A/B is per-donation; the $5,000 appraisal \
    threshold AGGREGATES similar crypto items donated across the year — rows shown as Section A may \
    require Section B + a qualified appraisal; verify AGGREGATE similar-item totals.";

/// P2-C Task 2: write `form8283.csv` — one row per `Donation` `RemovalLeg` contributed in `year`.
/// Stable snake_case columns; exact `Decimal`/`i64` string values (NFR5). 0o600 via `open_owner_only`.
///
/// The file leads with `#`-prefixed comment lines: the standing [R0-I1] aggregation caveat, and —
/// when the year's total noncash charitable deduction is ≤ $500 — the [R0-M1] filing-floor note that
/// Form 8283 is not required at that level (the rows are still emitted, informationally).
fn write_form8283_csv(
    out_dir: &Path,
    state: &LedgerState,
    year: i32,
) -> Result<(), crate::CliError> {
    use std::io::Write as _;
    let mut file = fsperms::open_owner_only(&out_dir.join("form8283.csv"))?;

    // [R0-I1] STANDING aggregation caveat — CSV header comment line (read with comment=b'#').
    writeln!(file, "# [R0-I1] {FORM_8283_AGGREGATION_CAVEAT}")?;

    // [R0-M1] $500 form-filing floor: Form 8283 is required only when total noncash contributions
    // for the year exceed $500. Rows are emitted regardless; add a note when the year's total
    // donation deduction is ≤ $500 that Form 8283 is not required at that level.
    let total_deduction: btctax_core::conventions::Usd = state
        .removals
        .iter()
        .filter(|r| r.kind == RemovalKind::Donation && r.removed_at.year() == year)
        .filter_map(|r| r.claimed_deduction)
        .sum();
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
    ])?;
    for row in form_8283(state, year) {
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
                "  TOTAL federal tax attributable to crypto (delta): {}   \
                (= ordinary-rate + LTCG + NIIT attributable)",
                fmt_money(r.total_federal_tax_attributable)
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
                "  marginal rates: ordinary {} / LTCG {} / NIIT {}",
                r.marginal_rates.ordinary, r.marginal_rates.ltcg, r.marginal_rates.niit_applies
            );
            let _ = writeln!(
                s,
                "  (incremental ceteris-paribus delta on the minimal profile; \
                excludes AGI-driven SS/IRMAA/AMT/QBI/phaseout effects — I5. §1411 NIIT reduces NII by the \
                §1211(b)-allowed net capital loss (≤ $3,000 / $1,500 MFS — Form 8960 line 5a / §1.1411-4(d)) \
                and is floored at $0; crypto ordinary income (mining/staking/airdrops/rewards) is correctly \
                excluded from NII. The only residual understatement is crypto-lending interest (NII under \
                §1411(c)(1)(A)(i)), which the minimal model cannot yet isolate — a Phase-2 refinement.)"
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

/// P2-D Task 2 (D3): render the standalone Schedule SE **§1401 self-employment tax** figure for
/// `year`. Mirrors `render_schedule_d` / `render_gift_advisory` — a standalone informational block
/// that does NOT feed engine B (`TaxResult::total_federal_tax_attributable` is UNCHANGED by SE tax).
///
/// Three cases (no silent drop — mirrors P2-C's m6):
/// - `result = Some(r)` → the full Schedule SE section: net SE income, the 92.35% base, the
///   SS/Medicare/Additional-Medicare components, total §1401 SE tax, the §164(f) deductible half,
///   the [D4/I2] dual-direction W-2 disclosure, and the [D5] standalone note.
/// - `result = None` AND `business_income_present` → a "SS wage base unavailable for {year}" note
///   (business SE income exists but the year has no bundled table → the wage base is unknown; the
///   §1401 tax is NOT computed rather than silently dropped).
/// - `result = None` AND NOT `business_income_present` → `None` (no Schedule SE section at all).
///
/// `business_income_present` is `!se_net_income(state, year).is_zero()` (computed by the caller —
/// the single §1402(a) SE-eligibility predicate lives in core).
pub fn render_schedule_se(
    year: i32,
    result: Option<&SeTaxResult>,
    business_income_present: bool,
) -> Option<String> {
    match result {
        Some(r) => {
            let mut s = String::new();
            let _ = writeln!(
                s,
                "Schedule SE (§1401 self-employment tax on business crypto income) — tax year {year}"
            );
            let _ = writeln!(
                s,
                "  net self-employment income (Schedule C net; business crypto, Interest excluded): {}",
                fmt_money(r.net_se)
            );
            let _ = writeln!(
                s,
                "  × 92.35% net-earnings factor (§1402(a)) = net SE earnings: {}",
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
            // [D4/I2] W-2 disclosure — the $0-W-2 assumption moves TWO components in OPPOSITE
            // directions; state each with the correct direction.
            let _ = writeln!(
                s,
                "  (W-2 assumption) Assumes $0 W-2 wages. If you had a wage job: (1) the 12.4% Social \
                 Security component may be OVERSTATED — its cap is the wage base LESS your W-2 \
                 Social-Security wages (a lower cap → less SS); AND (2) the 0.9% Additional Medicare \
                 component may be UNDERSTATED — the §1401(b)(2)(B)/Form 8959 threshold is REDUCED by \
                 your W-2 Medicare wages (a lower threshold → MORE income taxed at 0.9%). Adjust each \
                 accordingly."
            );
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
            if business_income_present {
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
                None // no business SE income → no Schedule SE section
            }
        }
    }
}

/// P2-C Task 3 (D3): Form 709 gift over-annual-exclusion **advisory** (thin, total-exposure only).
///
/// Sums `Σ fmv_at_transfer` over `Removal{Gift}` legs contributed in `year`, then:
/// - **`None`** when (a) there are NO gifts in the year, or (b) gifts are present but the total is
///   ≤ the year's §2503(b) annual exclusion.
/// - **`Some(advisory)`** when gifts exceed the exclusion — the total + the donee-not-modeled caveat.
/// - **`Some(note)` [R0-m6]** when gifts ARE present but the year has NO bundled table (the exclusion
///   is unavailable): Form 709 exposure is NOT evaluated — do NOT silently return `None`.
///
/// Standalone informational artifact — does NOT feed `compute_tax_year` / engine B. Because no donee
/// identifier is modeled, this is a TOTAL-exposure signal, not a per-donee determination.
pub fn render_gift_advisory(
    state: &LedgerState,
    year: i32,
    tables: &impl btctax_core::TaxTables,
) -> Option<String> {
    // Identify "gifts present" independently of the total (a zero-FMV gift is still a gift).
    let any_gift = state
        .removals
        .iter()
        .any(|r| r.kind == RemovalKind::Gift && r.removed_at.year() == year);
    if !any_gift {
        return None; // (a) no gifts in the year
    }
    let total: btctax_core::conventions::Usd = state
        .removals
        .iter()
        .filter(|r| r.kind == RemovalKind::Gift && r.removed_at.year() == year)
        .flat_map(|r| r.legs.iter())
        .map(|leg| leg.fmv_at_transfer)
        .sum();
    match tables.table_for(year) {
        // [R0-m6] gifts present but no bundled table → emit the note, never None (M5).
        None => Some(format!(
            "Form 709 gift advisory ({year}): gift annual-exclusion table unavailable for {year}; \
             Form 709 exposure not evaluated — ${} in gifts recorded.",
            fmt_money(total)
        )),
        Some(t) => {
            let excl = t.gift_annual_exclusion;
            if total > excl {
                Some(format!(
                    "Form 709 gift advisory ({year}): total gifts in {year}: ${}; the §2503(b) \
                     annual exclusion is ${} per donee (TY{year}). If any single donee received more \
                     than ${}, Form 709 may be required. NOTE: donee identity is not modeled — verify \
                     per-donee totals; this is a total-exposure signal, not a per-donee determination.",
                    fmt_money(total),
                    fmt_money(excl),
                    fmt_money(excl)
                ))
            } else {
                None // (b) gifts present but total ≤ exclusion
            }
        }
    }
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
    let _ = writeln!(
        s,
        "  federal tax attributable (estimated): {}",
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
            "  {} @ {} :: {}",
            c.disposal.canonical(),
            c.date,
            compliance_status_tag(&c.status)
        );
    }
    out
}

#[cfg(test)]
mod gift_advisory_tests {
    //! P2-C Task 3 KATs — `render_gift_advisory`. Direct-state `Removal{Gift}` fixtures + a
    //! `BTreeMap<i32, TaxTable>` table double so the exclusion + no-table cases are under exact
    //! control. PRIVACY: synthetic values only.
    use super::*;
    use btctax_core::conventions::Usd;
    use btctax_core::{EventId, LotId, Removal, RemovalLeg, TaxTable};
    use rust_decimal_macros::dec;
    use std::collections::BTreeMap;
    use time::macros::date;

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
            }],
            appraisal_required: false,
            donor_acquired_at: None,
            claimed_deduction: None,
        }
    }
    fn state_with(removals: Vec<Removal>) -> LedgerState {
        LedgerState {
            removals,
            ..Default::default()
        }
    }
    /// A table double carrying only the gift_annual_exclusion (ordinary/ltcg empty — unread here).
    fn tables_with(year: i32, excl: Usd) -> BTreeMap<i32, TaxTable> {
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
            },
        );
        m
    }

    /// Gifts over the exclusion → advisory with the total + the donee-not-modeled caveat.
    #[test]
    fn over_exclusion_emits_advisory_with_total_and_caveat() {
        let st = state_with(vec![gift_removal(1, date!(2025 - 06 - 01), dec!(20000))]);
        let tables = tables_with(2025, dec!(19000));
        let msg = render_gift_advisory(&st, 2025, &tables).expect("advisory expected");
        assert!(msg.contains("20000.00"), "must show the total: {msg}");
        assert!(msg.contains("19000.00"), "must show the exclusion: {msg}");
        assert!(
            msg.contains("donee identity is not modeled"),
            "must carry the donee caveat: {msg}"
        );
        assert!(msg.contains("total-exposure signal"), "{msg}");
    }

    /// Gifts under the exclusion → None.
    #[test]
    fn under_exclusion_is_none() {
        let st = state_with(vec![gift_removal(1, date!(2025 - 06 - 01), dec!(10000))]);
        let tables = tables_with(2025, dec!(19000));
        assert!(render_gift_advisory(&st, 2025, &tables).is_none());
    }

    /// No gifts in the year → None (even with a table present).
    #[test]
    fn no_gifts_is_none() {
        let st = state_with(vec![]);
        let tables = tables_with(2025, dec!(19000));
        assert!(render_gift_advisory(&st, 2025, &tables).is_none());
    }

    /// [R0-m6] gifts present but NO bundled table → Some(note), NOT None (no silent skip).
    #[test]
    fn gifts_present_but_no_table_emits_note_not_none() {
        let st = state_with(vec![gift_removal(1, date!(2026 - 06 - 01), dec!(50000))]);
        // Table double has 2025 only → table_for(2026) == None.
        let tables = tables_with(2025, dec!(19000));
        let msg = render_gift_advisory(&st, 2026, &tables).expect("note expected, not None");
        assert!(msg.contains("unavailable"), "{msg}");
        assert!(msg.contains("Form 709 exposure not evaluated"), "{msg}");
        assert!(
            msg.contains("50000.00"),
            "must record the gift total: {msg}"
        );
    }
}

#[cfg(test)]
mod schedule_se_tests {
    //! P2-D Task 2 KATs — `render_schedule_se` + `schedule_se.csv`. The rendered figures reuse the
    //! hand-verified Golden 1 `SeTaxResult` (Single $100,000 business mining). PRIVACY: synthetic.
    use super::*;
    use rust_decimal_macros::dec;

    /// Golden 1 SeTaxResult (Single, $100,000 business mining) — see btctax-core se.rs KATs.
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

    /// Business-mining year → full Schedule SE section: components + total + deductible half + the
    /// [I2] dual-direction W-2 disclosure + the [D5] standalone note.
    #[test]
    fn business_mining_year_renders_full_section() {
        let r = golden1();
        let s = render_schedule_se(2025, Some(&r), true).expect("SE section expected");
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
        // [I2] W-2 disclosure — CORRECT opposite directions.
        assert!(
            s.contains("OVERSTATED"),
            "SS component may be OVERSTATED: {s}"
        );
        assert!(
            s.contains("UNDERSTATED"),
            "Additional-Medicare component may be UNDERSTATED: {s}"
        );
        assert!(
            s.contains("§1401(b)(2)(B)"),
            "cites the Form 8959 threshold: {s}"
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
    }

    /// No business SE income → no Schedule SE section (None).
    #[test]
    fn no_business_income_no_section() {
        assert!(render_schedule_se(2025, None, false).is_none());
    }

    /// Business SE income present but no bundled table → the "SS wage base unavailable" note (no
    /// silent drop).
    #[test]
    fn business_income_but_no_table_emits_note() {
        let s = render_schedule_se(2099, None, true).expect("wage-base-unavailable note expected");
        assert!(s.contains("SS wage base unavailable"), "{s}");
        assert!(s.contains("2099"), "names the year: {s}");
        assert!(s.contains("no silent drop"), "{s}");
    }

    /// `schedule_se.csv` columns + values (year-scoped; written when a SeTaxResult exists).
    #[test]
    fn schedule_se_csv_columns_and_values() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("export");
        let st = LedgerState::default();
        let r = golden1();
        write_csv_exports(&out, &st, Some(2025), Some(&r)).unwrap();

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

    /// No SeTaxResult → schedule_se.csv is NOT written (nothing to file).
    #[test]
    fn schedule_se_csv_omitted_when_no_se_tax() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("export");
        let st = LedgerState::default();
        write_csv_exports(&out, &st, Some(2025), None).unwrap();
        assert!(!out.join("schedule_se.csv").exists());
    }
}
