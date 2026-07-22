//! Form 8275 (Disclosure Statement, Rev. 10-2024) fill: Part I disclosed positions (one row per T13
//! `Part1Item` — a promoted Form 8949 disposal leg's estimated basis) + Part II's filer-authored
//! narrative + the FILER's identity, read back through the shared SP2 flat oracle (`verify_flat`).
//!
//! **FREE-TEXT, not a money grid ([R0-C3] scope, T15).** Unlike `form8283.rs`'s property-table rows
//! (column-x-clustered `push_cell`), every Form 8275 write here is `push_free`
//! (`FlatPlacement::free`) — geometry-exempt but still page-checked, `/MaxLen`-checked, and inside the
//! no-unmapped set. Column (a) "Rev. Rul., Rev. Proc., etc." and column (e) "Line No." (a genuine
//! 3-character comb cell) are never written — see `Form8275Row`'s doc comment (`map.rs`) for why.
//!
//! **★ Year coverage is MANDATORY, not conditional (arch r1 I-6 / tax r1 M-7).** Form 8275 is
//! REVISION-versioned, not tax-year-versioned: `Form8275Map::for_year` and `pdf::f8275_pdf` both alias
//! the single bundled Rev. 10-2024 asset to EVERY `SUPPORTED_YEAR` (2017/2024/2025), so a promoted
//! disposal filed in any supported year gets a real fillable disclosure — this is what keeps T16's
//! re-pointed BG-D8 gate from PERMANENTLY refusing a promoted 2025 (or 2017) export, the dominant
//! current-year flow.

use crate::error::FormsError;
use crate::map::Form8275Map;
use crate::verify::{verify_flat, FlatPlacement};
use crate::{fmt_money, pdf};
use btctax_core::tax::packet::ReturnHeader;
use btctax_core::tax::printed::Printed8275;

/// A free-text cell (geometry-exempt, page-derived): written + authorized only when non-empty. Mirrors
/// `form8283.rs::push_free` exactly — Form 8275 has no money-grid columns, so every cell here uses it.
fn push_free(
    w: &mut Vec<(String, pdf::FieldValue)>,
    p: &mut Vec<FlatPlacement>,
    fqn: &str,
    value: &str,
) {
    if value.is_empty() {
        return;
    }
    w.push((fqn.to_string(), pdf::FieldValue::Text(value.to_string())));
    p.push(FlatPlacement::free(
        fqn.to_string(),
        crate::cells::page_of(fqn),
    ));
}

/// Fill Form 8275 for `year` from the T13 printed disclosure + the FULL-RETURN filer's identity.
///
/// `Ok(None)` when `printed.part_i` is empty — there is no position to disclose (T13's
/// `disclosure_8275` already returns `None` upstream for a year with no promoted disposal leg, but a
/// defensive re-check here means this fill never emits a content-less "blank" 8275).
///
/// Refuses ([`FormsError::Overflow`]) when there are more Part I items than this revision's 6 rows —
/// v1 does not paginate Form 8275 (unlike Form 8283's `overflow::merge_copies`); a promoted year with
/// more than 6 disposal legs is a real but rare case, tracked as a follow-up rather than built here.
pub fn fill_form_8275(
    printed: &Printed8275,
    header: &ReturnHeader,
    year: i32,
) -> Result<Option<Vec<u8>>, FormsError> {
    let map = Form8275Map::for_year(year)?;
    fill_form_8275_inner(printed, Some(header), &map)
}

/// Fill Form 8275 for `year` for the **crypto-slice** `export-irs-pdf` path (Task 16) — NO filer
/// identity: mirrors `form8283::fill_form_8283`, which writes its property rows the same way (the
/// crypto slice never wrote an identity block; its 8275 rides beside a return btctax did not produce).
/// `Ok(None)` when `printed.part_i` is empty (no promoted disposal leg filed in `year`).
pub fn fill_form_8275_slice(
    printed: &Printed8275,
    year: i32,
) -> Result<Option<Vec<u8>>, FormsError> {
    let map = Form8275Map::for_year(year)?;
    fill_form_8275_inner(printed, None, &map)
}

/// The map-parametrized fill (exposed via `testonly` for fault-injection KATs — mirrors
/// `fill_schedule_se_with_map` / `fill_1040_with_map`). Kept identity-REQUIRED (unlike
/// `fill_form_8283`'s `Option`) at this call surface — every existing caller (the full-return packet,
/// `sp4.rs`'s KATs) has one; the crypto-slice caller goes through [`fill_form_8275_slice`] instead.
pub fn fill_form_8275_with_map(
    printed: &Printed8275,
    header: &ReturnHeader,
    map: &Form8275Map,
) -> Result<Option<Vec<u8>>, FormsError> {
    fill_form_8275_inner(printed, Some(header), map)
}

/// The shared fill: `filer: None` (crypto-slice) skips the identity cells entirely, exactly as
/// `form8283::fill_form_8283_inner` does for its `filer: None` case.
fn fill_form_8275_inner(
    printed: &Printed8275,
    filer: Option<&ReturnHeader>,
    map: &Form8275Map,
) -> Result<Option<Vec<u8>>, FormsError> {
    if printed.part_i.is_empty() {
        return Ok(None);
    }
    if printed.part_i.len() > map.rows.len() {
        return Err(FormsError::Overflow {
            part: "Part I",
            rows: printed.part_i.len(),
            capacity: map.rows.len(),
        });
    }

    let mut w: Vec<(String, pdf::FieldValue)> = Vec::new();
    let mut p: Vec<FlatPlacement> = Vec::new();

    for (row_map, item) in map.rows.iter().zip(printed.part_i.iter()) {
        // (b) Item or Group of Items ← the position's form-location descriptor.
        push_free(&mut w, &mut p, &row_map.item, &item.line);
        // (c) Detailed Description of Items ← the Cohan-estimate explanation.
        push_free(&mut w, &mut p, &row_map.desc, &item.description);
        // (d) Form or Schedule ← the filed form (e.g. "8949").
        push_free(
            &mut w,
            &mut p,
            &row_map.form_schedule,
            &format!("Form {}", item.form),
        );
        // (f) Amount.
        push_free(&mut w, &mut p, &row_map.amount, &fmt_money(item.amount));
    }
    // Part II — the filer's combined narrative, written whole to the one free-text field (no per-line
    // splitting; mirrors form8283's whole-address identity writes).
    push_free(&mut w, &mut p, &map.part_ii_narrative, &printed.part_ii);

    // The FILER's identity — "Name(s) shown on return" + identifying number. `None` on the crypto-slice
    // path (Task 16): the disclosure still rides the export even with no `ReturnInputs` on file.
    if let Some(header) = filer {
        // The blank's fields are needed for the identity cells' /MaxLen.
        let blank_for_identity = pdf::collect_fields(&pdf::load(pdf::f8275_pdf(map.year)?)?)?;
        crate::cells::push_identity(
            &mut w,
            &mut p,
            &map.identity,
            &header.name_line,
            &header.taxpayer.ssn,
            &blank_for_identity,
        )?;
    }

    let writes = w;
    let placements = p;

    let mut doc = pdf::load(pdf::f8275_pdf(map.year)?)?;
    let index = pdf::index(&pdf::collect_fields(&doc)?);
    pdf::drop_xfa_and_set_needappearances(&mut doc)?;
    pdf::apply_writes(&mut doc, &index, &writes)?;
    pdf::strip_nondeterminism(&mut doc);
    let bytes = pdf::save(&mut doc)?;

    // Read back the SERIALIZED output. No column clusters (free-text form; see module doc).
    let check = pdf::load(&bytes)?;
    let fields = pdf::collect_fields(&check)?;
    verify_flat(&check, &fields, &placements, &[])?;
    Ok(Some(bytes))
}
