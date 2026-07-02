//! Compliance tab — renders the `VerifyReport` (FR9 verify output) as a read-only text display.
//!
//! Whole-ledger (year-independent): `build_verify` is not year-scoped.
//! STRICTLY READ-ONLY: no Session, no persistence, no mutations.

use crate::app::{App, Snapshot};
use btctax_cli::render::build_verify;
use btctax_core::LotMethod;
use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use std::fmt::Write as _;

use super::tags::compliance_status_tag;

/// Stable `LotMethod` display (re-implemented locally — CLI version is private).
fn lot_method_tag(m: LotMethod) -> &'static str {
    match m {
        LotMethod::Fifo => "FIFO",
        LotMethod::Lifo => "LIFO",
        LotMethod::Hifo => "HIFO",
    }
}

/// App-free renderer for the Compliance tab.
///
/// Extracted from `draw` so the editor crate can call this directly with its own
/// `Snapshot`, without holding an `App`.
///
/// Note: `year` is accepted for API consistency with other tab renderers but is not
/// used — the Compliance tab displays whole-ledger data (year-independent).
pub fn render(frame: &mut Frame, area: Rect, snap: &Snapshot, _year: i32) {
    let content = render_compliance_content(snap);
    let p = Paragraph::new(content)
        .block(
            Block::default()
                .title(" Compliance (whole-ledger) ")
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(p, area);
}

/// Render the Compliance tab into `area`.
///
/// Thin `pub(crate)` wrapper over [`render`]: handles the `snapshot == None` placeholder
/// exactly as before, then delegates to the App-free `render` fn.
/// Call sites in `draw.rs` and `tabs/tests.rs` call this wrapper — unchanged.
pub(crate) fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let Some(snap) = app.snapshot.as_ref() else {
        let p = Paragraph::new("no snapshot loaded")
            .block(Block::default().title(" Compliance ").borders(Borders::ALL));
        frame.render_widget(p, area);
        return;
    };
    render(frame, area, snap, app.selected_year);
}

/// Build the compliance report text from `snap`.
///
/// Calls `build_verify` (pure read-only builder) and formats the `VerifyReport` fields.
pub(crate) fn render_compliance_content(snap: &Snapshot) -> String {
    let verify = build_verify(&snap.state, &snap.events, &snap.cli_config);
    let mut s = String::new();

    // ── Conservation ──────────────────────────────────────────────────────────────────────────
    let conservation_label = if verify.conservation.balanced {
        "BALANCED"
    } else {
        "IMBALANCED"
    };
    let _ = writeln!(
        s,
        "  Conservation: {conservation_label}\
         (in: {}  disposed: {}  removed: {}  held: {}  fee: {}  pending: {})",
        verify.conservation.sigma_in,
        verify.conservation.sigma_disposed,
        verify.conservation.sigma_removed,
        verify.conservation.sigma_held,
        verify.conservation.sigma_fee_sats,
        verify.conservation.sigma_pending,
    );

    // ── Pre-2025 method + attestation ─────────────────────────────────────────────────────────
    let _ = writeln!(
        s,
        "  Pre-2025 method: {}  attested: {}",
        lot_method_tag(verify.declared_pre2025_method),
        verify.pre2025_method_attested
    );

    // ── Safe-harbor status ────────────────────────────────────────────────────────────────────
    let _ = writeln!(s, "  Safe-harbor: {}", verify.safe_harbor);

    // ── Pending-basis count ───────────────────────────────────────────────────────────────────
    let _ = writeln!(s, "  Pending transfers (unreconciled): {}", verify.pending);

    // ── Hard blockers ─────────────────────────────────────────────────────────────────────────
    let _ = writeln!(s);
    let _ = writeln!(s, "  Hard blockers ({}):", verify.hard.len());
    if verify.hard.is_empty() {
        let _ = writeln!(s, "    none");
    } else {
        for b in &verify.hard {
            let evt = b
                .event
                .as_ref()
                .map(|e| e.canonical())
                .unwrap_or_else(|| "-".into());
            let _ = writeln!(s, "    [{:?}] {} :: {}", b.kind, evt, b.detail);
        }
    }

    // ── Advisory blockers ─────────────────────────────────────────────────────────────────────
    let _ = writeln!(s);
    let _ = writeln!(s, "  Advisory blockers ({}):", verify.advisory.len());
    if verify.advisory.is_empty() {
        let _ = writeln!(s, "    none");
    } else {
        for b in &verify.advisory {
            let evt = b
                .event
                .as_ref()
                .map(|e| e.canonical())
                .unwrap_or_else(|| "-".into());
            let _ = writeln!(s, "    [{:?}] {} :: {}", b.kind, evt, b.detail);
        }
    }

    // ── Per-disposal compliance ───────────────────────────────────────────────────────────────
    let _ = writeln!(s);
    let _ = writeln!(
        s,
        "  Per-disposal compliance ({} post-2025 disposal(s)):",
        verify.compliance.len()
    );
    if verify.compliance.is_empty() {
        let _ = writeln!(s, "    no post-2025 disposals");
    } else {
        for dc in &verify.compliance {
            let _ = writeln!(
                s,
                "    {} @ {} :: {}",
                dc.disposal.canonical(),
                dc.date,
                compliance_status_tag(&dc.status)
            );
        }
    }

    s
}
