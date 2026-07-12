//! Tax tab — renders the `compute_tax_year` result as a text report.
//!
//! never writes the vault or any decrypted image of it; writes only the four form CSVs
//! via `export.rs` on explicit user confirmation. This module performs no writes.
//! No float — all money values are exact `Decimal` formatted with `{:.2}`.

use crate::app::{App, Snapshot};
use btctax_core::{
    compute_se_tax, compute_tax_year, se_net_income, RemovalKind, Severity, TaxOutcome, TaxTables,
};
use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use std::fmt::Write as _;

/// App-free renderer for the Tax tab.
///
/// Extracted from `draw` so the editor crate can call this directly with its own
/// `Snapshot` and `year`, without holding an `App`.
pub fn render(frame: &mut Frame, area: Rect, snap: &Snapshot, year: i32) {
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

/// Render the Tax tab into `area`.
///
/// Thin `pub(crate)` wrapper over [`render`]: handles the `snapshot == None` placeholder
/// exactly as before, then delegates to the App-free `render` fn.
/// Call sites in `draw.rs` and `tabs/tests.rs` call this wrapper — unchanged.
pub(crate) fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let Some(snap) = app.snapshot.as_ref() else {
        let p = Paragraph::new("no snapshot loaded")
            .block(Block::default().title(" Tax ").borders(Borders::ALL));
        frame.render_widget(p, area);
        return;
    };

    render(frame, area, snap, app.selected_year);
}

/// Build the full Tax tab text for `year` from `snap`.
///
/// Calls `compute_tax_year` (pure, read-only) and formats the result.
/// Returns a multi-line `String` ready for a `Paragraph` widget.
pub(crate) fn render_tax_content(snap: &Snapshot, year: i32) -> String {
    // [P2-C1] A full-return year that resolves refused/uncomputable renders its REASON, never a number —
    // the same fail-closed answer `report --tax-year` gives. `snap.profiles` already holds the DERIVED
    // profile for a ReturnInputs year (resolved in `build_snapshot`), so the compute below matches the CLI.
    if let Some(reason) = snap.refused.get(&year) {
        return format!("  NOT COMPUTABLE (full-return inputs): {reason}\n");
    }
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

    // ── SE section — OUTSIDE the outcome match (mirrors cmd/tax.rs:79–106) ──────────────────
    // PROFILE-GATED: no profile ⇒ no SE section (matches the CLI report AND the export).
    // Outcome-independent: NotComputable years with a profile + business income still show SE.
    let se_text = match snap.profiles.get(&year) {
        Some(p) => {
            let gross_se = se_net_income(&snap.state, year);
            let table_opt = snap.tables.table_for(year);
            let table_prsnt = table_opt.is_some();
            let se_result = table_opt.and_then(|t| {
                compute_se_tax(
                    &snap.state,
                    year,
                    p.filing_status,
                    t,
                    p.w2_ss_wages,
                    p.w2_medicare_wages,
                    p.schedule_c_expenses,
                )
            });
            btctax_cli::render::render_schedule_se(
                year,
                se_result.as_ref(),
                gross_se,
                table_prsnt,
                p.schedule_c_expenses,
                p.w2_ss_wages,
                p.w2_medicare_wages,
            )
        }
        None => None, // PROFILE-GATED: no profile ⇒ no SE section (mirrors cmd/tax.rs:79–106)
    };
    if let Some(text) = se_text {
        let _ = write!(s, "{text}");
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
