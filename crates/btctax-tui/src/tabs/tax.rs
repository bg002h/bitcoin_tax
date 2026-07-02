//! Tax tab — renders the `compute_tax_year` result as a text report.
//!
//! STRICTLY READ-ONLY: no Session, no persistence, no mutations.
//! No float — all money values are exact `Decimal` formatted with `{:.2}`.

use crate::app::{App, Snapshot};
use btctax_core::{
    compute_se_tax, compute_tax_year, FilingStatus, RemovalKind, Severity, TaxOutcome, TaxTables,
};
use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use std::fmt::Write as _;

/// Render the Tax tab into `area`.
pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let Some(snap) = app.snapshot.as_ref() else {
        let p = Paragraph::new("no snapshot loaded")
            .block(Block::default().title(" Tax ").borders(Borders::ALL));
        frame.render_widget(p, area);
        return;
    };

    let year = app.selected_year;
    let content = render_tax_content(snap, year);

    let p = Paragraph::new(content)
        .block(
            Block::default()
                .title(format!(" Tax — {year} "))
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(p, area);
}

/// Build the full Tax tab text for `year` from `snap`.
///
/// Calls `compute_tax_year` (pure, read-only) and formats the result.
/// Returns a multi-line `String` ready for a `Paragraph` widget.
pub(crate) fn render_tax_content(snap: &Snapshot, year: i32) -> String {
    let profile = snap.profiles.get(&year);
    let outcome = compute_tax_year(&snap.events, &snap.state, year, profile, &snap.tables);

    let mut s = String::new();

    match &outcome {
        TaxOutcome::NotComputable(b) => {
            let _ = writeln!(s, "  NOT COMPUTABLE [{:?}]: {}", b.kind, b.detail);
        }
        TaxOutcome::Computed(r) => {
            // B-M2: ordinary-rate attributable = total − ltcg_tax − niit (the reconciliation identity).
            let ord_attr = r.total_federal_tax_attributable - r.ltcg_tax - r.niit;

            let _ = writeln!(s, "  ST net: {:.2}   LT net: {:.2}", r.st_net, r.lt_net);
            let _ = writeln!(
                s,
                "  Ordinary income from crypto: {:.2}",
                r.ordinary_from_crypto
            );
            let _ = writeln!(
                s,
                "  Ordinary-rate tax (attributable delta): {:.2}",
                ord_attr
            );
            let _ = writeln!(
                s,
                "  LTCG tax (attributable delta): {:.2}   NIIT (attributable delta): {:.2}",
                r.ltcg_tax, r.niit
            );
            let _ = writeln!(
                s,
                "  TOTAL federal tax attributable (delta): {:.2}  [= ord + LTCG + NIIT]",
                r.total_federal_tax_attributable
            );
            let _ = writeln!(
                s,
                "  §1211 loss deduction: {:.2}   Carryforward out: short {:.2} / long {:.2}",
                r.loss_deduction, r.carryforward_out.short, r.carryforward_out.long
            );
            let _ = writeln!(
                s,
                "  Marginal rates: ordinary {}  LTCG {}  NIIT applies: {}",
                r.marginal_rates.ordinary, r.marginal_rates.ltcg, r.marginal_rates.niit_applies
            );

            // ── SE-tax block (standalone §1401; not in income-tax+NIIT total) ───────────────────
            let filing_status = profile
                .map(|p| p.filing_status)
                .unwrap_or(FilingStatus::Single);
            let w2_ss = profile.map(|p| p.w2_ss_wages).unwrap_or_default();
            let w2_medicare = profile.map(|p| p.w2_medicare_wages).unwrap_or_default();
            if let Some(t) = snap.tables.table_for(year) {
                if let Some(se) =
                    compute_se_tax(&snap.state, year, filing_status, t, w2_ss, w2_medicare)
                {
                    let _ = writeln!(s);
                    let _ = writeln!(s, "  --- Schedule SE (§1401 self-employment tax) ---");
                    let _ = writeln!(s, "  Net SE income: {:.2}", se.net_se);
                    let _ = writeln!(s, "  × 92.35% base: {:.2}", se.base);
                    let _ = writeln!(
                        s,
                        "  SS (12.4%): {:.2}   Medicare (2.9%): {:.2}   Addl Medicare (0.9%): {:.2}",
                        se.ss, se.medicare, se.addl
                    );
                    let _ = writeln!(
                        s,
                        "  TOTAL SE tax: {:.2}   §164(f) deductible half: {:.2}",
                        se.total, se.deductible_half
                    );
                    let _ = writeln!(
                        s,
                        "  (standalone) This §1401 SE tax is a SEPARATE federal liability, NOT \
                         included in the income-tax+NIIT total; §164(f) half not auto-coordinated."
                    );
                }
            }

            // ── Charitable deduction total for the year ────────────────────────────────────────
            let charitable_total: btctax_core::Usd = snap
                .state
                .removals
                .iter()
                .filter(|r| r.removed_at.year() == year && r.kind == RemovalKind::Donation)
                .filter_map(|r| r.claimed_deduction)
                .sum();
            let _ = writeln!(s);
            let _ = writeln!(
                s,
                "  Charitable deduction {year} (before §170(b) AGI limits): {:.2}",
                charitable_total
            );
        }
    }

    // Advisory blockers — shown for BOTH Computed and NotComputable outcomes.
    let advisories: Vec<_> = snap
        .state
        .blockers
        .iter()
        .filter(|b| b.kind.severity() == Severity::Advisory)
        .collect();
    if !advisories.is_empty() {
        let _ = writeln!(s);
        let _ = writeln!(s, "  Advisory blockers ({}):", advisories.len());
        for b in &advisories {
            let _ = writeln!(s, "    [{:?}] {}", b.kind, b.detail);
        }
    }

    s
}
