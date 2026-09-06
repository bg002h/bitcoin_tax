//! Forms tab — renders Form 8949 rows (selectable table), Schedule D totals,
//! and Form 8283 rows for the selected tax year.
//!
//! STRICTLY READ-ONLY: no Session, no persistence, no mutations.
//! No float (NFR5 / [R0-M5]): all amounts are exact `Decimal`.

use crate::app::{App, Snapshot};
use btctax_core::{
    form_8283, form_8949, schedule_d, Form8283Section, Form8949Box, Form8949Part,
    InformationReturnRegime,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState, Wrap},
    Frame,
};
use std::fmt::Write as _;

// ── Local tag helpers (re-implemented — CLI versions are private) ──────────────────────────────

/// Stable Form 8949 part tag. Values: "ST" (Part I short-term) / "LT" (Part II long-term).
fn form8949_part_tag(p: Form8949Part) -> &'static str {
    match p {
        Form8949Part::ShortTerm => "ST",
        Form8949Part::LongTerm => "LT",
    }
}

/// Stable Form 8949 box tag. Values: the pre-TY2025 securities boxes "C" (ST) / "F" (LT), and the
/// TY2025+ digital-asset boxes "I" (ST) / "L" (LT). Which one a row carries is decided year-aware by
/// `btctax_core::form_8949`; this fn only renders whatever box the row already holds.
pub(super) fn form8949_box_tag(b: Form8949Box) -> &'static str {
    match b {
        Form8949Box::C => "C",
        Form8949Box::F => "F",
        Form8949Box::I => "I",
        Form8949Box::L => "L",
        // spec 1099-DA R2 — the broker-reported boxes, chosen from the filer's answers on a live year
        Form8949Box::G => "G",
        Form8949Box::H => "H",
        Form8949Box::J => "J",
        Form8949Box::K => "K",
    }
}

/// Stable Form 8283 section tag. Values: "A" (≤ $5,000) / "B" (> $5,000).
fn form8283_section_tag(s: Form8283Section) -> &'static str {
    match s {
        Form8283Section::A => "A",
        Form8283Section::B => "B",
    }
}

/// App-free renderer for the Forms tab.
///
/// Extracted from `draw` so the editor crate can call this directly with its own
/// `Snapshot`, `year`, and `TableState`, without holding an `App`.
///
/// Layout: upper portion = Form 8949 scrollable table; lower portion = Schedule D totals +
/// Form 8283 rows + standing footnotes.
/// ★ spec 1099-DA T6 — the box-review footnote follows the year's Form 1099-DA REGIME, joined from
/// its record, never the box-revision constant: no proceeds reporting → the securities pairing
/// (C/F ↔ A/B/D/E, 1099-B); proceeds only → review I/L against G/H/J/K; proceeds AND basis → the
/// boxes were CHOSEN from the filer's answers, so the note says what to compare them with (R4);
/// a year with no record says so rather than guessing a pairing.
pub fn broker_box_note(year: i32, regime: Option<InformationReturnRegime>) -> String {
    match regime {
        None => format!(
            "NOTE: TY{year} has no year record in this build — review the broker-reported boxes by hand."
        ),
        Some(r) if r.basis => "NOTE: boxes G/H/J/K follow your Form 1099-DA answers (`report` lists the keys) — \
                               compare column (e) of every G/J row with box 1g, and (d) with box 1f."
            .to_string(),
        Some(r) if r.proceeds => {
            "NOTE: Review box I/L — exchange disposals may require G/H/J/K (1099-DA).".to_string()
        }
        Some(_) => "NOTE: Review box C/F — exchange disposals may require A/B/D/E (1099-B).".to_string(),
    }
}

pub fn render(
    frame: &mut Frame,
    area: Rect,
    snap: &Snapshot,
    year: i32,
    table_state: &mut TableState,
) {
    // Split: top = 8949 table, bottom = Schedule D + 8283 + footnotes.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(10)])
        .split(area);

    // ── Form 8949 table ────────────────────────────────────────────────────────────────────────
    let rows_8949 = form_8949(&snap.state, year);

    if rows_8949.is_empty() {
        let p = Paragraph::new(format!("no Form 8949 rows for {year}")).block(
            Block::default()
                .title(format!(" Forms — {year} "))
                .borders(Borders::ALL),
        );
        frame.render_widget(p, chunks[0]);
    } else {
        let header = Row::new(vec![
            Cell::from("Part"),
            Cell::from("Box"),
            Cell::from("Description"),
            Cell::from("Acquired"),
            Cell::from("Sold"),
            Cell::from("Proceeds"),
            Cell::from("Basis"),
            Cell::from("Gain"),
        ]);

        let table_rows: Vec<Row> = rows_8949
            .iter()
            .map(|r| {
                Row::new(vec![
                    Cell::from(form8949_part_tag(r.part)),
                    Cell::from(form8949_box_tag(r.box_)),
                    Cell::from(r.description.clone()),
                    Cell::from(r.date_acquired.to_string()),
                    Cell::from(r.date_sold.to_string()),
                    Cell::from(format!("{:.2}", r.proceeds)),
                    Cell::from(format!("{:.2}", r.cost_basis)),
                    Cell::from(format!("{:.2}", r.gain)),
                ])
            })
            .collect();

        let widths = vec![
            Constraint::Length(4),
            Constraint::Length(3),
            Constraint::Percentage(18),
            Constraint::Percentage(12),
            Constraint::Percentage(12),
            Constraint::Percentage(12),
            Constraint::Percentage(12),
            Constraint::Percentage(12),
        ];

        let table = Table::new(table_rows, widths)
            .header(header)
            .block(
                Block::default()
                    .title(format!(" Form 8949 — {year} "))
                    .borders(Borders::ALL),
            )
            .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

        frame.render_stateful_widget(table, chunks[0], table_state);
    }

    // ── Schedule D + Form 8283 + footnotes ────────────────────────────────────────────────────
    let sd = schedule_d(&snap.state, year);
    let rows_8283 = form_8283(&snap.state, year, &snap.donation_details);

    let mut bottom = String::new();
    let _ = writeln!(
        bottom,
        "Schedule D Part I (ST): proceeds {:.2}  basis {:.2}  gain {:.2}",
        sd.st.proceeds, sd.st.cost_basis, sd.st.gain
    );
    let _ = writeln!(
        bottom,
        "Schedule D Part II (LT): proceeds {:.2}  basis {:.2}  gain {:.2}",
        sd.lt.proceeds, sd.lt.cost_basis, sd.lt.gain
    );

    if !rows_8283.is_empty() {
        let _ = writeln!(bottom, "Form 8283 ({} row(s)):", rows_8283.len());
        for r in &rows_8283 {
            let sec = r.section.map(form8283_section_tag).unwrap_or("");
            let deduction = r
                .claimed_deduction
                .map(|d| format!(" deduction {:.2}", d))
                .unwrap_or_default();
            let _ = writeln!(
                bottom,
                "  [§{}] {}{}{}",
                sec,
                r.description,
                deduction,
                if r.needs_review { " [review]" } else { "" }
            );
        }
    }
    // Standing caveats (footnotes)
    let _ = writeln!(
        bottom,
        "NOTE: the Section (A/B) is set by the §170(f)(11)(F) year-aggregate of similar donated items, not per donation."
    );
    // spec 1099-DA T6 — the box-review caveat follows the year's REGIME (its record), not the constant
    let _ = writeln!(
        bottom,
        "{}",
        broker_box_note(year, btctax_cli::year_readiness::regime_for(year))
    );

    let p = Paragraph::new(bottom)
        .block(
            Block::default()
                .title(" Schedule D / Form 8283 ")
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(p, chunks[1]);
}

/// Render the Forms tab into `area`.
///
/// Thin `pub(crate)` wrapper over [`render`]: handles the `snapshot == None` placeholder
/// exactly as before, then delegates to the App-free `render` fn.
/// Call sites in `draw.rs` and `tabs/tests.rs` call this wrapper — unchanged.
pub(crate) fn draw(frame: &mut Frame, area: Rect, app: &mut App) {
    let Some(snap) = app.snapshot.as_ref() else {
        let p = Paragraph::new("no snapshot loaded")
            .block(Block::default().title(" Forms ").borders(Borders::ALL));
        frame.render_widget(p, area);
        return;
    };

    let year = app.selected_year;
    render(frame, area, snap, year, &mut app.forms_state);
}

#[cfg(test)]
mod broker_note_tests {
    use super::broker_box_note;
    use btctax_core::InformationReturnRegime as R;

    /// spec 1099-DA T6 — one note per regime, the two pre-live wordings byte-identical to the
    /// walkthrough goldens (j6 TY2024, j2 TY2025), the live one naming 1g/1f, and no record → say so.
    #[test]
    fn the_note_follows_the_regime() {
        assert_eq!(
            broker_box_note(2024, Some(R::NONE)),
            "NOTE: Review box C/F — exchange disposals may require A/B/D/E (1099-B)."
        );
        assert_eq!(
            broker_box_note(2025, Some(R::PROCEEDS_ONLY)),
            "NOTE: Review box I/L — exchange disposals may require G/H/J/K (1099-DA)."
        );
        let live = broker_box_note(2026, Some(R::PROCEEDS_AND_BASIS));
        assert!(live.contains("box 1g") && live.contains("box 1f"), "{live}");
        assert!(!live.contains("Review box I/L"), "{live}");
        let none = broker_box_note(2031, None);
        assert!(
            none.contains("no year record") && none.contains("2031"),
            "{none}"
        );
    }
}
