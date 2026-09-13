//! Form 8949 fill: format the tax data into grid cells, authorize each write against the per-year
//! map, fill the bundled PDF, and READ BACK the serialized bytes through the geometric verifier.
//!
//! **Digital-asset box:** on the 2025 1099-DA revision Bitcoin is filed under **Box I** (short-term)
//! / **Box L** (long-term) — NEVER Box C/F ("other than digital asset transactions"); the pre-2025
//! revisions use Box C/F. The box is chosen by the part's on-state in the per-year map, so the core
//! `Form8949Box` taxonomy (`{C,F,I,L}`, itself year-aware) is not reused here.

use crate::error::FormsError;
use crate::map::{Form8949Map, PartMap};
use crate::verify::{verify_8949, Geo, Placement};
use crate::{fmt_date, pdf};
use btctax_core::forms::Form8949Row;
use btctax_core::Form8949Part;
use rust_decimal::Decimal;

/// One part's formatted rows + totals, ready to place on the form.
pub struct PartData {
    /// ★ spec 1099-DA T3 — the ONE Form 8949 box every row on this page-set carries (`G`…`L`, or
    /// `C`/`F`/`I`/`L` for a not-reported set); `None` for an empty part. A page-set never mixes
    /// boxes: the filler groups rows by (part, box) before paginating.
    pub box_letter: Option<String>,
    /// Each row's 8 column strings (a..h); an empty string means "leave that cell blank".
    pub rows: Vec<[String; 8]>,
    /// Totals row: (d) proceeds, (e) cost, (g) adjustment, (h) gain — as display strings.
    pub totals: [String; 4],
    /// Whether the (g) adjustment total is non-zero (else that cell stays blank).
    pub adj_nonzero: bool,
}

/// Column indices on Form 8949 (a=0 … h=7).
const COL_D: usize = 3;
const COL_E: usize = 4;
const COL_G: usize = 6;
const COL_H: usize = 7;

/// Format one core row into its 8 column strings. Money is the exact `Decimal` Display (identical to
/// the `form8949.csv`); dates render MM/DD/YYYY (the form's native date format); the (f) code and a
/// zero (g) adjustment stay blank per IRS convention.
fn row_cells(r: &Form8949Row) -> Result<[String; 8], FormsError> {
    Ok([
        r.description.clone(),
        fmt_date(r.date_acquired)?,
        fmt_date(r.date_sold)?,
        r.proceeds.to_string(),
        r.cost_basis.to_string(),
        r.adjustment_code.clone(),
        if r.adjustment_amount.is_zero() {
            String::new()
        } else {
            r.adjustment_amount.to_string()
        },
        r.gain.to_string(),
    ])
}

/// Build one part's `PartData` from its rows.
pub fn part_data(rows: &[&Form8949Row]) -> Result<PartData, FormsError> {
    let box_letter = rows.first().map(|r| format!("{:?}", r.box_));
    if let Some(b) = &box_letter {
        if let Some(other) = rows.iter().find(|r| format!("{:?}", r.box_) != *b) {
            return Err(FormsError::Structure(format!(
                "a Form 8949 page-set mixes boxes {b} and {:?} — group by (part, box) before paginating (spec 1099-DA T3)",
                other.box_
            )));
        }
    }
    let mut out = Vec::with_capacity(rows.len());
    let (mut sp, mut sc, mut sg, mut sh) =
        (Decimal::ZERO, Decimal::ZERO, Decimal::ZERO, Decimal::ZERO);
    for r in rows {
        out.push(row_cells(r)?);
        sp += r.proceeds;
        sc += r.cost_basis;
        sg += r.adjustment_amount;
        sh += r.gain;
    }
    Ok(PartData {
        box_letter,
        rows: out,
        totals: [
            sp.to_string(),
            sc.to_string(),
            sg.to_string(),
            sh.to_string(),
        ],
        adj_nonzero: !sg.is_zero(),
    })
}

/// Accumulate the writes + placements for one part onto its map page.
fn place_part(
    part: &PartMap,
    data: &PartData,
    writes: &mut Vec<(String, pdf::FieldValue)>,
    placements: &mut Vec<Placement>,
) -> Result<(), FormsError> {
    if data.rows.is_empty() {
        return Ok(()); // nothing to report on this part — leave the box + totals blank
    }
    // ★ The box this page-set carries, resolved by LETTER against the map (spec 1099-DA T3): a
    //   letter the map does not name refuses — never the default checkbox in its place.
    let letter = data.box_letter.as_deref().unwrap_or("");
    let (field, on) = part.box_cell(letter).ok_or_else(|| {
        FormsError::UnmappedField(format!(
            "Form 8949 box {letter} on the {} part: this revision's map names no checkbox for it (the \
             scalar pair is `{}`; a 2025+ map carries the G/H/I and J/K/L table) — no page written",
            part.term, part.box_field
        ))
    })?;
    writes.push((
        field.to_string(),
        pdf::FieldValue::Check { on: on.to_string() },
    ));
    placements.push(Placement {
        fqn: field.to_string(),
        geo: Geo::Check,
    });
    // Data rows.
    for (ri, cells) in data.rows.iter().enumerate() {
        for (ci, value) in cells.iter().enumerate() {
            if value.is_empty() {
                continue; // blank cell — do not write, do not authorize
            }
            let fqn = part.rows[ri][ci].clone();
            writes.push((fqn.clone(), pdf::FieldValue::Text(value.clone())));
            placements.push(Placement {
                fqn,
                geo: Geo::Data { row: ri, col: ci },
            });
        }
    }
    // Per-part totals (the line-2 text says "Enter each total here").
    let totals = [
        (COL_D, &part.totals.proceeds_d, true),
        (COL_E, &part.totals.cost_e, true),
        (COL_G, &part.totals.adj_g, data.adj_nonzero),
        (COL_H, &part.totals.gain_h, true),
    ];
    let vals = [
        &data.totals[0],
        &data.totals[1],
        &data.totals[2],
        &data.totals[3],
    ];
    for (i, (col, fqn, include)) in totals.into_iter().enumerate() {
        if !include {
            continue;
        }
        writes.push((fqn.clone(), pdf::FieldValue::Text(vals[i].clone())));
        placements.push(Placement {
            fqn: fqn.clone(),
            geo: Geo::Total { col },
        });
    }
    Ok(())
}

/// Fill Form 8949 (Part I + Part II, ≤ 11 rows each) into the bundled TY2025 PDF and return the
/// serialized bytes. The output is read back through the geometric verifier — a mis-mapped cell or a
/// stray write FAILS CLOSED (no bytes returned). Pagination for > 11 rows is handled by
/// [`crate::fill_form_8949`], which chunks before calling this.
pub fn fill_8949_parts(
    short: &PartData,
    long: &PartData,
    map: &Form8949Map,
) -> Result<Vec<u8>, FormsError> {
    fill_8949_parts_inner(short, long, map, None)
}

/// As [`fill_8949_parts`], with the FILER's identity written on **every page it files** — the
/// full-return path. An unnamed 8949 is not filable, and each page carries its own "Name(s) shown on
/// return" + SSN header (Fable P6 r1 I3).
///
/// ★ FR-218 — "every page it files" is not "both pages": a part with no rows is not filed at all, so
/// it gets no header either. See [`pages_to_file`].
pub fn fill_8949_parts_with_identity(
    short: &PartData,
    long: &PartData,
    map: &Form8949Map,
    header: &btctax_core::tax::packet::ReturnHeader,
) -> Result<Vec<u8>, FormsError> {
    fill_8949_parts_inner(short, long, map, Some(header))
}

/// Rows were routed to a part this revision's map cannot place, so they have nowhere to print.
fn unplaceable_part(year: i32, term: &str, rows: usize) -> FormsError {
    FormsError::Structure(format!(
        "the {year} Form 8949 map declares no {term}-term part, but {rows} row(s) are routed to it \
         — they would vanish from the filed form"
    ))
}

/// ★★★ **FR-218 — which of the template's pages this copy actually files**, as a strictly increasing
/// set of 0-based page indices.
///
/// `i8949` (`design/forms/extract/i8949--2024.txt:422-424`): *"You don't need to complete and file an
/// entire copy of Form 8949 (Parts I and II) if you can check a single box to describe all your
/// transactions. In that case, complete and file **either Part I or II** and check the box that
/// describes the transactions."* A part with no rows is therefore not a blank page to be filed — it is
/// no page at all. Before FR-218 every copy filed both pages, so a long-term-only filer needing two
/// Part II pages received two blank Part I pages, each carrying their name and SSN, and the owner found
/// it by printing the packet.
///
/// ★ The kept set is **derived from the map's own part→page mapping**, never a literal `{0, 1}`: a
/// revision that moves a part to another page keeps deciding correctly. Two things fail CLOSED here
/// rather than silently dropping a page — a part whose `term` is neither `"short"` nor `"long"` (there
/// would be no rows to consult), and a template page that no part claims (nothing would decide whether
/// to file it).
fn pages_to_file(
    map: &Form8949Map,
    short: &PartData,
    long: &PartData,
    template_pages: usize,
) -> Result<Vec<usize>, FormsError> {
    let mut mapped: Vec<usize> = Vec::new();
    let mut keep: Vec<usize> = Vec::new();
    for p in &map.parts {
        let live = match p.term.as_str() {
            "short" => !short.rows.is_empty(),
            "long" => !long.rows.is_empty(),
            other => {
                return Err(FormsError::Structure(format!(
                    "the {} Form 8949 map declares a part {other:?} that is neither \"short\" nor \
                     \"long\" — which rows decide whether its page is filed is then undefined",
                    map.year
                )))
            }
        };
        mapped.push(p.page);
        if live {
            keep.push(p.page);
        }
    }
    mapped.sort_unstable();
    mapped.dedup();
    if mapped != (0..template_pages).collect::<Vec<usize>>() {
        return Err(FormsError::Structure(format!(
            "the {} Form 8949 map's parts claim pages {mapped:?}, but the bundled template has \
             {template_pages} page(s) — a page no part claims has nothing to decide whether it is \
             filed",
            map.year
        )));
    }
    keep.sort_unstable();
    keep.dedup();
    if keep.is_empty() {
        // Neither part has a row. That is not a filed Form 8949 at all (`form_8949_printed` returns
        // `None` for a year with no disposals), and FR-218 is about a blank page filed BESIDE a
        // populated one — so the row-less fill keeps the template's shape unchanged.
        return Ok(mapped);
    }
    Ok(keep)
}

fn fill_8949_parts_inner(
    short: &PartData,
    long: &PartData,
    map: &Form8949Map,
    filer: Option<&btctax_core::tax::packet::ReturnHeader>,
) -> Result<Vec<u8>, FormsError> {
    if short.rows.len() > map.rows_per_page {
        return Err(FormsError::Overflow {
            part: "Part I",
            rows: short.rows.len(),
            capacity: map.rows_per_page,
        });
    }
    if long.rows.len() > map.rows_per_page {
        return Err(FormsError::Overflow {
            part: "Part II",
            rows: long.rows.len(),
            capacity: map.rows_per_page,
        });
    }

    let mut writes: Vec<(String, pdf::FieldValue)> = Vec::new();
    let mut placements: Vec<Placement> = Vec::new();
    for (term, data) in [("short", short), ("long", long)] {
        match map.part(term) {
            Some(p) => place_part(p, data, &mut writes, &mut placements)?,
            // A map that declares no such part cannot print it. That is only survivable when the part
            // is empty; rows routed to a part the map cannot place would vanish off the filed form,
            // which is an understatement, so it refuses.
            None if data.rows.is_empty() => {}
            None => return Err(unplaceable_part(map.year, term, data.rows.len())),
        }
    }

    let mut doc = pdf::load(pdf::f8949_pdf(map.year)?)?;
    let blank_fields = pdf::collect_fields(&doc)?;
    let keep = pages_to_file(map, short, long, doc.get_pages().len())?;

    // The filer's identity, on every page this copy FILES. `Geo::Check` = authorized + in the
    // no-unmapped set, but excluded from the column-geometry oracle (an identity cell is in no data
    // column). ★ FR-218 — a page this copy does not file gets no name and no SSN, because it is not a
    // page of the return. The loop is driven by `keep` and INDEXES the map's identity blocks by page
    // (`identity_page1` is template page 0, `identity_page2` page 1), so a filed page with no identity
    // block refuses rather than going out unnamed — including on a future revision with a third page.
    if let Some(header) = filer {
        let blocks = [&map.identity_page1, &map.identity_page2];
        for &page in &keep {
            let cells = blocks.get(page).copied().and_then(Option::as_ref).ok_or_else(|| {
                FormsError::Geometry(format!(
                    "the {} Form 8949 map has no [identity] block for filed page {} — a full return \
                     cannot file an unnamed Form 8949",
                    map.year,
                    page + 1
                ))
            })?;
            let ssn = crate::cells::render_ssn(
                &header.taxpayer.ssn,
                blank_fields
                    .iter()
                    .find(|f| f.fqn == cells.ssn)
                    .and_then(|f| f.max_len),
            )?;
            writes.push((
                cells.name.clone(),
                pdf::FieldValue::Text(header.name_line.clone()),
            ));
            placements.push(Placement {
                fqn: cells.name.clone(),
                geo: crate::verify::Geo::Check,
            });
            writes.push((cells.ssn.clone(), pdf::FieldValue::Text(ssn)));
            placements.push(Placement {
                fqn: cells.ssn.clone(),
                geo: crate::verify::Geo::Check,
            });
        }
    }

    let index = pdf::index(&blank_fields);
    pdf::drop_xfa_and_set_needappearances(&mut doc)?;
    pdf::apply_writes(&mut doc, &index, &writes)?;
    // ★★ FR-218 — reduce the copy to the pages it files, AFTER the writes (the index above is built
    //    from the unreduced template) and BEFORE the read-back verify, so the geometry oracle and the
    //    no-unmapped scan run on the document that is actually emitted.
    crate::overflow::retain_pages(&mut doc, &keep)?;
    pdf::strip_nondeterminism(&mut doc);
    let bytes = pdf::save(&mut doc)?;

    // True read-back: re-parse the SERIALIZED output and verify geometry against the PDF's own rects.
    let check = pdf::load(&bytes)?;
    let fields = pdf::collect_fields(&check)?;
    verify_8949(&check, &fields, &placements, &map.table_token)?;
    // XFA must be gone on the actual output.
    if pdf_has_xfa(&check)? {
        return Err(FormsError::Structure(
            "output still carries /XFA after fill".into(),
        ));
    }
    Ok(bytes)
}

/// Whether the document's AcroForm still carries an `/XFA` key.
pub fn pdf_has_xfa(doc: &lopdf::Document) -> Result<bool, FormsError> {
    let acro = match doc.catalog()?.get(b"AcroForm") {
        Ok(lopdf::Object::Reference(id)) => doc.get_dictionary(*id)?,
        Ok(lopdf::Object::Dictionary(d)) => d,
        _ => return Ok(false),
    };
    Ok(acro.has(b"XFA"))
}

/// Split the year's rows into Part I (short-term) and Part II (long-term), preserving the core's
/// deterministic order.
pub fn split_parts(rows: &[Form8949Row]) -> (Vec<&Form8949Row>, Vec<&Form8949Row>) {
    let mut st = Vec::new();
    let mut lt = Vec::new();
    for r in rows {
        match r.part {
            Form8949Part::ShortTerm => st.push(r),
            Form8949Part::LongTerm => lt.push(r),
        }
    }
    (st, lt)
}
