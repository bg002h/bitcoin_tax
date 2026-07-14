//! **The line-keyed inverse transcriber** — read a filled PDF back in the form's own language.
//!
//! Every other read-back in this crate goes through a fully-qualified AcroForm leaf name:
//!
//! ```ignore
//! assert_eq!(tv(&pdf, "topmostSubform[0].Page1[0].f1_3[0]").as_deref(), Some("280000")); // L1
//! ```
//!
//! The `// L1` is the only part a reader cares about and the one part nothing checks. [`extract_lines`]
//! inverts the committed map instead, handing back what the filled PDF actually SAYS keyed by the
//! logical line — so an assertion can name `line1` and mean it.
//!
//! ★ **This is a read-back, not an oracle.** It goes through the same map the fill used, so it cannot
//! catch a mis-mapped cell — a map that pointed `line1` at line 2's widget would fill and transcribe
//! consistently wrong. That is [`crate::verify`]'s job: the geometric verifier re-derives the column
//! and row bands from the blank PDF's own `/Rect`s and never consults the map. The two are
//! complementary and neither replaces the other. **Geometry says the value landed in the right box;
//! this says the right VALUE is in it.**
//!
//! It walks the map TOML generically rather than any typed `*Map` struct, because its consumer — the
//! P7 packet round-trip — transcribes every form in the packet without knowing which one it holds.

use crate::error::FormsError;
use crate::pdf::{checkbox_on, collect_fields, load, text_value};
use std::collections::BTreeMap;
use toml::Value;

/// Read a filled PDF back as `logical line name → the text actually on the paper`.
///
/// Keys are the map's own: `line1`, `line8`, and — for the nested groups every schedule carries — the
/// dotted `identity.name`, `identity.ssn`. Rows of a repeating table come back indexed:
/// `part1_rows[0].payer`.
///
/// **Only cells the fill actually WROTE appear.** A blank line on a tax form is a statement ("none"),
/// so "written as empty" and "never written" must not collapse into the same reading; an absent key
/// means the cell is blank on the paper.
///
/// A checkbox that is ON transcribes as its on-state (`"1"`); one that is off is absent, like any
/// other blank cell.
pub fn extract_lines(pdf: &[u8], map_toml: &str) -> Result<BTreeMap<String, String>, FormsError> {
    let map: Value = map_toml.parse()?;

    let doc = load(pdf)?;
    let fields = collect_fields(&doc)?;
    // A map may point two logical cells at one widget (the 2024 1040's line-7 cell is reachable as
    // both `line7a` and `line7`), so index by name rather than consuming the list.
    let by_name: BTreeMap<&str, _> = fields.iter().map(|f| (f.fqn.as_str(), f)).collect();

    let mut out = BTreeMap::new();
    walk(&map, "", &by_name, &doc, &mut out);
    Ok(out)
}

fn walk(
    node: &Value,
    prefix: &str,
    by_name: &BTreeMap<&str, &crate::pdf::Field>,
    doc: &lopdf::Document,
    out: &mut BTreeMap<String, String>,
) {
    match node {
        // A bare string is a field name — IF the PDF actually has that field. That test is what keeps
        // the map's metadata (`form = "f8959"`, `table_token = "..."`) out of the transcript without a
        // hand-maintained skip-list that would rot the first time a map grew a key.
        Value::String(fqn) => {
            if let Some(f) = by_name.get(fqn.as_str()) {
                if let Some(v) = text_value(doc, f.id) {
                    if !v.trim().is_empty() {
                        out.insert(prefix.to_string(), v);
                    }
                }
            }
        }
        // `{ field = "...", on = "1" }` — a checkbox. Present only when it is ON.
        Value::Table(t) if t.contains_key("field") => {
            let Some(Value::String(fqn)) = t.get("field") else {
                return;
            };
            if let Some(f) = by_name.get(fqn.as_str()) {
                if let Some(on) = checkbox_on(doc, f.id) {
                    out.insert(prefix.to_string(), on);
                }
            }
        }
        Value::Table(t) => {
            for (k, v) in t {
                let key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                walk(v, &key, by_name, doc, out);
            }
        }
        Value::Array(a) => {
            for (i, v) in a.iter().enumerate() {
                walk(v, &format!("{prefix}[{i}]"), by_name, doc, out);
            }
        }
        // Integers and booleans (`year = 2024`, `da_present = true`) are metadata, never cells.
        _ => {}
    }
}
