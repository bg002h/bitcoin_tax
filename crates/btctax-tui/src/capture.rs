//! Style-aware `TestBackend` capture (SPEC §8) — serialize a ratatui [`Buffer`] to a DETERMINISTIC,
//! diff-legible text golden. Shared by both TUI crates' golden tests (`btctax-tui` tabs and
//! `btctax-tui-edit` reconcile flows); lives in the lib (not a test module) so both integration-test
//! crates can call it, like the CLI docs generator is a `pub fn`.
//!
//! Format — two views of the same frame:
//! 1. a **glyph grid** (`NNN│<row text>`, trailing spaces trimmed) — the human-readable screenshot;
//! 2. a compact **style overlay**: per row, the maximal runs of cells sharing a non-default style,
//!    `NNN│ start..end <sig>` where `<sig>` names only the non-default `fg`/`bg`/`modifier` (a run of the
//!    terminal default — `fg=Reset bg=Reset`, no modifier — is omitted). A color change flips ONE run
//!    line, so diffs stay localized, and the overlay carries the full color info the PDF render needs.
//!
//! **§14 gap 7 decision (M-1):** the signature is `(symbol, fg, bg, modifier)` and deliberately DROPS
//! `Cell.underline_color` and `Cell.skip`. Rationale: neither varies in the current TUI — the only
//! underline is `sort.rs`'s column cursor via `Modifier::UNDERLINED` (which IS captured), no code sets a
//! distinct `underline_color`, and `skip` is uniform in a `TestBackend` buffer. Re-open (add them to the
//! signature) the moment any screen sets a per-cell `underline_color` or `skip`, or their regressions
//! become invisible to the goldens.
//!
//! **Stability (N-3):** `color_str` relies on ratatui `Color`'s derived `Debug`. Under the locked ratatui
//! 0.29 this is fixed; a future format change would red EVERY golden at regen (loud, never silent), so it
//! is a diagnosable regen-time failure, not a correctness hazard.

use ratatui::buffer::{Buffer, Cell};
use ratatui::style::{Color, Modifier};
use std::fmt::Write as _;

/// Serialize a rendered `Buffer` to its golden text (glyph grid + style overlay).
pub fn to_golden(buf: &Buffer) -> String {
    let area = buf.area();
    let mut out = String::new();
    out.push_str("── glyphs ──\n");
    for y in 0..area.height {
        let mut row = String::new();
        for x in 0..area.width {
            row.push_str(buf.cell((x, y)).map_or(" ", Cell::symbol));
        }
        let _ = writeln!(out, "{y:>3}│{}", row.trim_end());
    }
    out.push_str("── styles (runs; terminal-default fg=Reset bg=Reset no-modifier omitted) ──\n");
    for y in 0..area.height {
        // Fold consecutive cells with the same style signature into runs.
        let mut x = 0u16;
        while x < area.width {
            let sig = buf.cell((x, y)).and_then(cell_sig);
            let start = x;
            x += 1;
            while x < area.width && buf.cell((x, y)).and_then(cell_sig) == sig {
                x += 1;
            }
            if let Some(sig) = sig {
                let _ = writeln!(out, "{y:>3}│ {start}..{x} {sig}");
            }
        }
    }
    out
}

/// The non-default style signature of a cell, or `None` when the cell is the terminal default
/// (`fg=Reset`, `bg=Reset`, no modifier) — those runs are omitted from the overlay.
fn cell_sig(c: &Cell) -> Option<String> {
    if c.fg == Color::Reset && c.bg == Color::Reset && c.modifier.is_empty() {
        return None;
    }
    let mut parts = Vec::new();
    if c.fg != Color::Reset {
        parts.push(format!("fg={}", color_str(c.fg)));
    }
    if c.bg != Color::Reset {
        parts.push(format!("bg={}", color_str(c.bg)));
    }
    if !c.modifier.is_empty() {
        parts.push(format!("mod={}", modifier_str(c.modifier)));
    }
    Some(parts.join(" "))
}

/// A compact, stable rendering of a [`Color`] (no interior spaces, so `Rgb(1,2,3)` stays one token).
fn color_str(c: Color) -> String {
    format!("{c:?}").replace(", ", ",")
}

/// A compact, stable rendering of a [`Modifier`] bitset (`BOLD|REVERSED`), iterating the flags in a fixed
/// order so the string is deterministic regardless of how the modifier was assembled.
fn modifier_str(m: Modifier) -> String {
    const FLAGS: &[(Modifier, &str)] = &[
        (Modifier::BOLD, "BOLD"),
        (Modifier::DIM, "DIM"),
        (Modifier::ITALIC, "ITALIC"),
        (Modifier::UNDERLINED, "UNDERLINED"),
        (Modifier::SLOW_BLINK, "SLOW_BLINK"),
        (Modifier::RAPID_BLINK, "RAPID_BLINK"),
        (Modifier::REVERSED, "REVERSED"),
        (Modifier::HIDDEN, "HIDDEN"),
        (Modifier::CROSSED_OUT, "CROSSED_OUT"),
    ];
    FLAGS
        .iter()
        .filter(|(f, _)| m.contains(*f))
        .map(|(_, name)| *name)
        .collect::<Vec<_>>()
        .join("|")
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::style::Style;

    #[test]
    fn glyphs_and_style_runs_are_deterministic_and_style_aware() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 6, 2));
        buf.set_string(
            0,
            0,
            "Hi",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );
        buf.set_string(0, 1, "xy", Style::default().bg(Color::Cyan));
        let a = to_golden(&buf);
        let b = to_golden(&buf);
        assert_eq!(a, b, "capture must be byte-deterministic");
        assert!(
            a.contains("  0│Hi"),
            "glyph grid shows the row text (trailing spaces trimmed):\n{a}"
        );
        assert!(
            a.contains("  0│ 0..2 fg=Yellow mod=BOLD"),
            "styled run for the Hi cells:\n{a}"
        );
        assert!(
            a.contains("  1│ 0..2 bg=Cyan"),
            "styled run for the xy cells:\n{a}"
        );
    }

    #[test]
    fn default_cells_produce_no_style_runs() {
        let buf = Buffer::empty(Rect::new(0, 0, 4, 1)); // all default (Reset/Reset/no-mod)
        let g = to_golden(&buf);
        // The glyphs section has the (blank) row; the styles section has zero run lines after its header.
        let styles = g.split("── styles").nth(1).expect("has a styles section");
        assert!(
            !styles.contains("│ "),
            "an all-default frame must emit no style runs; got:\n{styles}"
        );
    }
}
