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

/// ★★★ spec 1099-DA R2 (r3 I-1) — the Box column the Forms tab PRINTS, decided from the year's
/// regime and the filer's stored answers. Pure: no `Snapshot`, no vault, no `Session`.
///
/// **Why this exists.** `form_8949` returns rows carrying the *pre-route* box — I short-term, L
/// long-term from TY2025. On a live year (basis regime + ≥1 exchange row) the box the return
/// actually carries is CHOSEN from the filer's Form 1099-DA answers by `route_8949_boxes`, and the
/// tab called neither. A TY2026 vault with `[broker_reporting.coinbase] covered = "basis_matches"`
/// showed **I** for a row the packet files under **G**, two lines above a footnote reading *"boxes
/// G/H/J/K follow your Form 1099-DA answers … compare column (e) of every G/J row with box 1g"* —
/// the filer was told to find G/J rows on a table that showed none. T6-a even widened
/// [`form8949_box_tag`] with the G/H/J/K arms, which no path in either TUI could reach.
///
/// **The three outcomes.**
/// - not live (no year record, or a regime without basis) → the rows exactly as built, no caption;
/// - live and the answers settle every key → the ROUTED letters;
/// - live and they do not (a key unanswered, `mixed`, or `basis_differs`) → **`—`** on every KEYED
///   row plus a caption naming the exit. Not a guessed letter, and not a partial route: routing
///   mutates in place and stops at the first unsettled key, so the attempt runs on a CLONE and the
///   originals are what gets displayed.
///
/// Returns each row paired with the Box cell's text, and the caption to print under the table.
pub fn routed_box_tags(
    rows: Vec<btctax_core::Form8949Row>,
    regime: Option<InformationReturnRegime>,
    answers: Option<&btctax_core::BrokerReporting>,
) -> (
    Vec<(btctax_core::Form8949Row, &'static str)>,
    Option<String>,
) {
    let Some(regime) = regime else {
        return (tagged(rows), None);
    };
    if !btctax_core::broker_question_is_live(&rows, regime) {
        return (tagged(rows), None);
    }
    let stored = answers.cloned().unwrap_or_default();
    let mut attempt = rows.clone();
    match btctax_core::route_8949_boxes(&mut attempt, regime, &stored) {
        Ok(()) => (tagged(attempt), None),
        Err(e) => {
            let shown = rows
                .into_iter()
                .map(|r| {
                    // a keyed row's box is UNDECIDED; a self-custody row is I/L by mechanism
                    let tag = if btctax_core::forms::broker_key(&r).is_some() {
                        "—"
                    } else {
                        form8949_box_tag(r.box_)
                    };
                    (r, tag)
                })
                .collect();
            (
                shown,
                Some(format!(
                    "NOTE: Box is — where the Form 1099-DA answers do not settle the row ({e}) — \
                     answer the Form 1099-DA keys (`report` lists them) to see the boxes."
                )),
            )
        }
    }
}

fn tagged(rows: Vec<btctax_core::Form8949Row>) -> Vec<(btctax_core::Form8949Row, &'static str)> {
    rows.into_iter()
        .map(|r| {
            let tag = form8949_box_tag(r.box_);
            (r, tag)
        })
        .collect()
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
    // ★ spec 1099-DA R2 (r3 I-1) — the Box column is ROUTED on a live year, from the year's regime
    //   and the answers the snapshot projected at unlock; an unsettled key shows `—` + a caption.
    let regime = btctax_cli::year_readiness::regime_for(year);
    let (rows_8949, route_caption) = routed_box_tags(
        form_8949(&snap.state, year),
        regime,
        snap.broker_answers.get(&year),
    );

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
            .map(|(r, box_tag)| {
                Row::new(vec![
                    Cell::from(form8949_part_tag(r.part)),
                    Cell::from(*box_tag),
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
    let _ = writeln!(bottom, "{}", broker_box_note(year, regime));
    // ★ r3 I-1 — and, when the answers do not settle the keys, the caption for the `—` boxes above.
    if let Some(caption) = &route_caption {
        let _ = writeln!(bottom, "{caption}");
    }

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

#[cfg(test)]
mod broker_route_tests {
    use super::routed_box_tags;
    use btctax_core::forms::{BrokerReported, BrokerReporting, Cohort, CohortAnswers};
    use btctax_core::{
        DisposeKind, Form8949Box, Form8949Part, Form8949Row, InformationReturnRegime as R, Usd,
        WalletId,
    };
    use time::macros::date;

    fn row(part: Form8949Part, box_: Form8949Box, wallet: WalletId) -> Form8949Row {
        Form8949Row {
            part,
            box_,
            box_needs_review: matches!(wallet, WalletId::Exchange { .. }),
            cohort: Cohort::Covered,
            description: "0.50000000 BTC".to_string(),
            date_acquired: date!(2026 - 02 - 01),
            date_sold: date!(2026 - 06 - 01),
            proceeds: Usd::from(50000),
            cost_basis: Usd::from(30000),
            adjustment_code: String::new(),
            adjustment_amount: Usd::ZERO,
            gain: Usd::from(20000),
            wallet,
            disposition_kind: DisposeKind::Sell,
        }
    }

    fn exchange(provider: &str) -> WalletId {
        WalletId::Exchange {
            provider: provider.to_string(),
            account: "main".to_string(),
        }
    }

    fn answers(provider: &str, cohort: Cohort, a: BrokerReported) -> BrokerReporting {
        let mut m = BrokerReporting::default();
        let e =
            m.0.entry(provider.to_string())
                .or_insert_with(CohortAnswers::default);
        match cohort {
            Cohort::Covered => e.covered = Some(a),
            Cohort::Noncovered => e.noncovered = Some(a),
        }
        m
    }

    fn tags(v: &[(Form8949Row, &'static str)]) -> Vec<&'static str> {
        v.iter().map(|(_, t)| *t).collect()
    }

    /// ★★★ spec 1099-DA R2 (r3 I-1) KILL — on a LIVE year the Box column shows the ROUTED letter.
    ///
    /// **This is the mutation.** `form_8949` builds the coinbase row as **I** (the TY2025+
    /// not-reported default). With `covered = "basis_matches"` stored, the return files it under
    /// **G**. Before the fold the Forms tab printed `r.box_` straight from `form_8949`, so the
    /// screen said **I** while the packet said **G** — under a footnote telling the filer to
    /// *"compare column (e) of every G/J row with box 1g"*. Delete the `route_8949_boxes` call in
    /// [`routed_box_tags`] and this test reds with `I` where `G` is asserted: the exact defect,
    /// re-planted.
    ///
    /// The self-custody row stays **L** either way — no broker, no form, I/L by mechanism.
    #[test]
    fn a_live_year_shows_the_routed_box_not_the_pre_route_default() {
        let rows = vec![
            row(
                Form8949Part::ShortTerm,
                Form8949Box::I,
                exchange("coinbase"),
            ),
            row(
                Form8949Part::LongTerm,
                Form8949Box::L,
                WalletId::SelfCustody {
                    label: "cold".to_string(),
                },
            ),
        ];
        // the pre-route state the tab used to print, pinned so the kill is unambiguous
        assert_eq!(
            rows.iter()
                .map(|r| super::form8949_box_tag(r.box_))
                .collect::<Vec<_>>(),
            ["I", "L"],
            "the fixture starts on the not-reported default — G is produced by ROUTING, not by the ledger"
        );

        let a = answers("coinbase", Cohort::Covered, BrokerReported::BasisMatches);
        let (shown, caption) = routed_box_tags(rows.clone(), Some(R::PROCEEDS_AND_BASIS), Some(&a));
        assert_eq!(tags(&shown), ["G", "L"], "basis_matches ⇒ box G short-term");
        assert!(caption.is_none(), "{caption:?}");

        // proceeds_only ⇒ H; not_reported ⇒ I (the same letter, now by ANSWER rather than default)
        for (answer, want) in [
            (BrokerReported::ProceedsOnly, "H"),
            (BrokerReported::NotReported, "I"),
        ] {
            let a = answers("coinbase", Cohort::Covered, answer);
            let (shown, _) = routed_box_tags(rows.clone(), Some(R::PROCEEDS_AND_BASIS), Some(&a));
            assert_eq!(tags(&shown)[0], want, "{answer:?}");
        }
    }

    /// ★★ r3 I-1 — an UNSETTLED key shows `—` and a caption, never a guessed letter.
    ///
    /// Unanswered, `mixed` and `basis_differs` are all states in which no box is true of the key's
    /// rows. The keyed row's Box becomes `—`; the self-custody row keeps its mechanical L; the
    /// caption names the exit (`report` lists the keys). ★ The route attempt runs on a CLONE —
    /// `route_8949_boxes` mutates in place and stops at the first unsettled key, so a two-key ledger
    /// must not display the half that happened to route before the failure.
    #[test]
    fn an_unsettled_key_shows_an_em_dash_and_a_caption() {
        let rows = vec![
            row(Form8949Part::ShortTerm, Form8949Box::I, exchange("aaa")),
            row(Form8949Part::ShortTerm, Form8949Box::I, exchange("zzz")),
            row(
                Form8949Part::LongTerm,
                Form8949Box::L,
                WalletId::SelfCustody {
                    label: "cold".to_string(),
                },
            ),
        ];
        // `aaa` routes, `zzz` does not — the partial-mutation trap.
        let a = answers("aaa", Cohort::Covered, BrokerReported::BasisMatches);
        let (shown, caption) = routed_box_tags(rows.clone(), Some(R::PROCEEDS_AND_BASIS), Some(&a));
        assert_eq!(
            tags(&shown),
            ["—", "—", "L"],
            "no key is settled until EVERY key is: a half-routed table is a claim about a return \
             that will not be filed"
        );
        let c = caption.expect("an unsettled key must carry its caption");
        assert!(c.contains("zzz") && c.contains("report"), "{c}");

        for answer in [BrokerReported::Mixed, BrokerReported::BasisDiffers] {
            let mut a = answers("aaa", Cohort::Covered, BrokerReported::BasisMatches);
            let e = a.0.entry("zzz".to_string()).or_default();
            e.covered = Some(answer);
            let (shown, caption) =
                routed_box_tags(rows.clone(), Some(R::PROCEEDS_AND_BASIS), Some(&a));
            assert_eq!(tags(&shown), ["—", "—", "L"], "{answer:?}");
            assert!(caption.is_some(), "{answer:?}");
        }

        // and with NO stored answers at all
        let (shown, caption) = routed_box_tags(rows, Some(R::PROCEEDS_AND_BASIS), None);
        assert_eq!(tags(&shown), ["—", "—", "L"]);
        assert!(caption.is_some());
    }

    /// ★ A NOT-LIVE year is untouched — the rows exactly as `form_8949` built them, no caption. This
    /// is what holds the two pre-live walkthrough goldens (j2 TY2025, j6 TY2024) byte-identical.
    #[test]
    fn a_not_live_year_prints_the_rows_as_built() {
        let rows = vec![
            row(
                Form8949Part::ShortTerm,
                Form8949Box::I,
                exchange("coinbase"),
            ),
            row(Form8949Part::LongTerm, Form8949Box::C, exchange("coinbase")),
        ];
        let a = answers("coinbase", Cohort::Covered, BrokerReported::BasisMatches);
        for regime in [None, Some(R::NONE), Some(R::PROCEEDS_ONLY)] {
            let (shown, caption) = routed_box_tags(rows.clone(), regime, Some(&a));
            assert_eq!(tags(&shown), ["I", "C"], "{regime:?}");
            assert!(caption.is_none(), "{regime:?}");
        }
        // …and a live year with no keyed row at all is equally untouched
        let self_only = vec![row(
            Form8949Part::LongTerm,
            Form8949Box::L,
            WalletId::SelfCustody {
                label: "cold".to_string(),
            },
        )];
        let (shown, caption) = routed_box_tags(self_only, Some(R::PROCEEDS_AND_BASIS), None);
        assert_eq!(tags(&shown), ["L"]);
        assert!(caption.is_none());
    }
}
