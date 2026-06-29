//! Text rendering of CLI outputs (FR9 verify, FR4 report/show) + FR10 CSV export. Pure string-building
//! over engine data — the CLI displays; the engine computes (NFR4/NFR5).
use btctax_adapters::FileReport;
use btctax_core::persistence::ImportReport;
use btctax_core::{DisposalLeg, LedgerState, RemovalLeg, Term, WalletId};
use std::fmt::Write as _;

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

fn term_str(t: Term) -> &'static str {
    match t {
        Term::ShortTerm => "ST",
        Term::LongTerm => "LT",
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
            "  {}#{} {} remaining {} sat | basis {} ({:?}){}",
            l.lot_id.origin_event_id.canonical(),
            l.lot_id.split_sequence,
            wallet_label(&l.wallet),
            l.remaining_sat,
            l.usd_basis,
            l.basis_source,
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
                "  {:?} @ {} ({:?})",
                d.kind,
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
                "  {:?} @ {} ({:?})",
                r.kind,
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
                "  {:?} @ {} {} sat = {} USD{}",
                i.kind,
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
        .map(|z| format!(" gift-zone {z:?}"))
        .unwrap_or_default();
    let _ = writeln!(
        out,
        "    {} sat: proceeds {} basis {} gain {} {}{}",
        leg.sat,
        leg.proceeds,
        leg.basis,
        leg.gain,
        term_str(leg.term),
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
        term_str(leg.term)
    );
}
