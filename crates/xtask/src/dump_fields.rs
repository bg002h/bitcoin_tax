//! `cargo run -p xtask -- dump-fields <pdf>` — list every AcroForm field in a PDF.
//!
//! Authoring a `<form>.map.toml` means writing `logical_line = "<fully-qualified AcroForm field
//! name>"`, and those FQNs are not guessable — they are IRS-internal
//! (`topmostSubform[0].Page2[0].f2_01[0]`). This dumps them in reading order so a human can walk
//! the printed form top-to-bottom and pair each visible line with its field.
//!
//! Output columns: `page  x0,y0-x1,y1  kind  FQN  [on-states]`.
//!
//! Geometry is what disambiguates the many identically-shaped names: on a two-column form the
//! left/right cells of one line differ only in `x0`, and the bare name `f2_01[0]` tells you
//! nothing about which row it sits on.

use btctax_forms::testonly::{button_on_states, collect_fields, load, Field};

/// The page a field sits on, parsed out of the IRS FQN (`…Page2[0]…` ⇒ 2).
///
/// `Field` carries no page number, and the widget's `/P` back-reference isn't exposed. The IRS
/// templates always nest widgets under a `PageN[0]` subform, so the name is the page. Anything
/// that doesn't match sorts last rather than being dropped — a missing field in this dump would
/// be an invisible hole in the map.
fn page_of(f: &Field) -> u32 {
    f.fqn
        .split(".")
        .find_map(|seg| seg.strip_prefix("Page")?.split('[').next()?.parse().ok())
        .unwrap_or(u32::MAX)
}

pub fn run(path: &str) -> Result<(), String> {
    let bytes = std::fs::read(path).map_err(|e| format!("read {path}: {e}"))?;
    let doc = load(&bytes).map_err(|e| format!("parse {path}: {e}"))?;
    let mut fields = collect_fields(&doc).map_err(|e| format!("collect fields {path}: {e}"))?;

    // Reading order: page, then top-to-bottom (PDF y grows upward, so descending y), then
    // left-to-right. Fields with no widget rect sort to the end of their page.
    fields.sort_by(|a, b| {
        let (ra, rb) = (a.rect.unwrap_or([0.0; 4]), b.rect.unwrap_or([0.0; 4]));
        page_of(a)
            .cmp(&page_of(b))
            .then(a.rect.is_none().cmp(&b.rect.is_none()))
            .then(rb[3].total_cmp(&ra[3]))
            .then(ra[0].total_cmp(&rb[0]))
    });

    println!("# {} — {} AcroForm fields", path, fields.len());
    for f in &fields {
        let geo = match f.rect {
            Some(r) => format!("{:>6.1},{:>6.1}-{:>6.1},{:>6.1}", r[0], r[1], r[2], r[3]),
            None => format!("{:>27}", "(no rect)"),
        };
        let kind = if f.is_button { "btn " } else { "text" };
        let on = match f.is_button {
            true => format!("  on={:?}", button_on_states(&doc, f.id)),
            false => String::new(),
        };
        println!("p{:<3} {geo}  {kind}  {}{on}", page_of(f), f.fqn);
    }
    Ok(())
}
