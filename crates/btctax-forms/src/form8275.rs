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
//!
//! **★ Part II is WRAPPED, never truncated (T-f8275-part-ii-overflow).** The filer's Part II narrative
//! is the §1.6662-4(f) adequate-disclosure text — the whole of Approach B's estimated-basis defense
//! rests on it. It used to be written whole to the ONE `part_ii_narrative` field, which is a
//! single-line, `DoNotScroll`, no-`/MaxLen` widget: the write always "succeeded" (nothing at the
//! PDF-data level stops an over-long `/V`), but a viewer honoring the widget's own box could only
//! DISPLAY the first ~137 characters at 8pt — a fifth of a disclosure is not a disclosure, and neither
//! `/MaxLen` (not declared on this field) nor `verify_flat`'s geometry leg (skipped for free
//! placements) could see it.
//!
//! **★ Only Part II's OWN line 1 (`part_ii_narrative`, `p1-t80[0]`) is ever written — round 2.** The
//! bundled PDF's XFA template draws printed numerals `"1 "`..`"6 "` beside `p1-t80[0]`..`p1-t85[0]`
//! (`Line1PartII`..`Line6PartII`, confirmed by decompressing the asset's own `template` XFA packet),
//! numbered to match Part I's 6 rows — Part II line *n* is meant to explain Part I row *n*. Spreading
//! one COMBINED narrative (multiple promoted tranches' accounts, joined by
//! `btctax-core/src/tax/form8275.rs`) across those numbered lines would attribute sentence fragments to
//! Part I items they do not explain — an early build of this fix did exactly that and its own golden
//! showed line 2's text sitting beside Part I's SECOND row while actually continuing line 1's sentence.
//! Per-item numbering (one explanation per numbered line, matched to its own Part I row) is the more
//! faithful long-term shape, but it is a bigger change (needs `Part1Item`-to-narrative correlation this
//! crate does not have today) — filed as a follow-up, not built here
//! (`design/f8275-part-ii-overflow/FOLLOWUPS.md`). For now: everything past Part II's line 1 spills to
//! Part IV instead, which has no per-line numbering to misclaim.
//!
//! **★ Part IV carries the IRS-required cross-reference.** Rev. 10-2024's Specific Instructions for
//! Part IV: *"Use Part IV on page 2 if you need more space for Parts I and/or II. Include the
//! corresponding part and line number from page 1."* Page 2 is captioned "Explanations (continued
//! from Parts I **and/or** II)" — an examiner reading it cold cannot tell which without the label. The
//! FIRST Part IV line used therefore always starts with
//! [`crate::wrap::PART_IV_CROSS_REFERENCE_PREFIX`] ("Part II, line 1 (continued): "), budgeted into the
//! wrap so it never itself causes an overflow.
//!
//! **★ The wrap budgets the renderer's text inset, not the raw widget box.** AcroForm variable text is
//! inset from its `/Rect` edges by (at minimum) border width + 1 (PDF 32000-1 §12.7.4.3); measured
//! against this asset, poppler starts ~2.16pt in from the left edge and pdf.js is more conservative
//! still (≈4pt total) — budgeting the RAW rect width let 3 lines of the fix's own >1500-char KAT
//! fixture measure over their box once inset was accounted for (see
//! `design/f8275-part-ii-overflow/BUILD-REPORT.md`). [`usable_width`] subtracts
//! `2 * TEXT_INSET_PTS` before `crate::wrap` ever sees a width.
//!
//! `crate::wrap` measures at the SAME Helvetica-Bold 8pt the PDF's own `/DA` declares (using the
//! bundled asset's OWN embedded font `/Widths`, not a generic AFM table — see `wrap.rs`'s doc comment)
//! and wraps the narrative, respecting paragraph breaks as hard breaks (`wrap::split_paragraphs`) so
//! two promoted tranches' independent accounts never blend into one filed sentence. If it still will
//! not fit across Part II's line 1 + all of Part IV, the fill fails closed with [`FormsError::Overflow`]
//! (mirroring the Part I >6-row refusal below) rather than clip. [`part_ii_capacity_check`] runs the
//! SAME wrap without filling anything, so `btctax-cli`'s export paths can refuse before writing any
//! packet file at all.

use crate::error::FormsError;
use crate::map::Form8275Map;
use crate::verify::{verify_flat, FlatPlacement};
use crate::wrap::{wrap_part_ii, PartIiOverflow};
use crate::{fmt_money, pdf};
use btctax_core::tax::packet::ReturnHeader;
use btctax_core::tax::printed::Printed8275;

/// The font size Form 8275's Part II ("Detailed Explanation") and Part IV ("Explanations (continued
/// from Parts I and/or II)") continuation lines declare in their own `/DA`
/// (`/HelveticaLTStd-Bold 8.00 Tf`, verified against the bundled Rev. 10-2024 asset) — the size
/// `crate::wrap` measures the narrative at.
const PART_II_FONT_SIZE_PT: f32 = 8.0;

/// Minimum text inset (PDF points) a `/Rect`'s raw width is reduced by, PER SIDE, before it is treated
/// as usable — see the module doc's "the wrap budgets the renderer's text inset" note. `2.0` matches
/// pdf.js's observed ≈4pt total inset on this asset (the more conservative of the renderers measured),
/// while still comfortably covering poppler's smaller ~2.16pt.
const TEXT_INSET_PTS: f32 = 2.0;

/// The usable text width of field `fqn` (its `/Rect` width minus `2 * TEXT_INSET_PTS`) — the SAME
/// figure both the real fill and [`part_ii_capacity_check`] budget the wrap against, so the two can
/// never disagree about what fits.
fn usable_width(fields: &[pdf::Field], fqn: &str) -> Result<f32, FormsError> {
    let field = fields
        .iter()
        .find(|f| f.fqn == fqn)
        .ok_or_else(|| FormsError::MapFieldMissing(fqn.to_string()))?;
    let rect = field
        .rect
        .ok_or_else(|| FormsError::Structure(format!("{fqn}: field has no /Rect")))?;
    Ok((rect[2] - rect[0]) - 2.0 * TEXT_INSET_PTS)
}

/// Part II's own line-1 usable width + a Part IV line's usable width, for `map`'s bundled asset —
/// shared by the real fill and [`part_ii_capacity_check`].
fn part_ii_iv_widths(
    blank_fields: &[pdf::Field],
    map: &Form8275Map,
) -> Result<(f32, f32), FormsError> {
    let part_ii_width = usable_width(blank_fields, &map.part_ii_narrative)?;
    let part_iv_width = match map.part_iv_continuation.first() {
        Some(fqn) => usable_width(blank_fields, fqn)?,
        // No Part IV lines mapped at all — degrade to Part II's own (proven-safe) width; unreachable on
        // the bundled asset (27 lines mapped), defensive only.
        None => part_ii_width,
    };
    Ok((part_ii_width, part_iv_width))
}

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

/// Like [`push_free`], but the placement also joins a per-group strictly-descending-y ordinal sequence
/// (`FlatPlacement::free_ordered`) — used for Part IV's continuation lines, which (unlike every other
/// free-text write in this form) really are a physically ORDERED sequence the fill assumes is
/// top-to-bottom (round 2 finding 6).
fn push_free_ordered(
    w: &mut Vec<(String, pdf::FieldValue)>,
    p: &mut Vec<FlatPlacement>,
    fqn: &str,
    value: &str,
    group: u32,
    ordinal: u32,
) {
    if value.is_empty() {
        return;
    }
    w.push((fqn.to_string(), pdf::FieldValue::Text(value.to_string())));
    p.push(FlatPlacement::free_ordered(
        fqn.to_string(),
        crate::cells::page_of(fqn),
        group,
        ordinal,
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

    // The BLANK PDF's own field set — needed for (a) each Part II/IV continuation line's `/Rect` width
    // (the wrap measurement below) and (b) the identity SSN cell's `/MaxLen` when `filer` is `Some`.
    // One load covers both; loaded unconditionally because the wrap needs it even on the crypto-slice
    // path (Task 16), which has no identity to write.
    let blank_fields = pdf::collect_fields(&pdf::load(pdf::f8275_pdf(map.year)?)?)?;

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
    // Part II — the filer's combined narrative, WRAPPED (never truncated) onto Part II's own line 1
    // (`part_ii_narrative`) then, if it does not fit there alone, Part IV's continuation lines
    // (`part_iv_continuation`, with the IRS-required cross-reference on the first one used) — see the
    // module doc for why Part II's numbered lines 2-6 are deliberately left unclaimed. Fails closed
    // ([`FormsError::Overflow`], mirroring the Part I row refusal above) rather than silently clipping
    // past a field boundary — the shipped defect this fix exists to end.
    let (part_ii_width, part_iv_width) = part_ii_iv_widths(&blank_fields, map)?;
    let wrapped = wrap_part_ii(
        &printed.part_ii,
        part_ii_width,
        part_iv_width,
        map.part_iv_continuation.len(),
        PART_II_FONT_SIZE_PT,
    )
    .map_err(|overflow: PartIiOverflow| FormsError::Overflow {
        part: "Part II",
        rows: overflow.rows_needed,
        capacity: overflow.capacity,
    })?;
    const PART_II_GROUP: u32 = 0;
    const PART_IV_GROUP: u32 = 1;
    push_free_ordered(
        &mut w,
        &mut p,
        &map.part_ii_narrative,
        &wrapped.part_ii_line1,
        PART_II_GROUP,
        0,
    );
    for (i, (fqn, line)) in map
        .part_iv_continuation
        .iter()
        .zip(wrapped.part_iv_lines.iter())
        .enumerate()
    {
        push_free_ordered(&mut w, &mut p, fqn, line, PART_IV_GROUP, i as u32);
    }

    // The FILER's identity — "Name(s) shown on return" + identifying number. `None` on the crypto-slice
    // path (Task 16): the disclosure still rides the export even with no `ReturnInputs` on file.
    if let Some(header) = filer {
        crate::cells::push_identity(
            &mut w,
            &mut p,
            &map.identity,
            &header.name_line,
            &header.taxpayer.ssn,
            &blank_fields,
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

/// Whether `narrative` fits, WITHOUT filling anything — the SAME wrap the real fill runs (via
/// [`part_ii_iv_widths`]/[`wrap_part_ii`], so the two can never disagree). `Ok(PartIiCapacity::Fits)`
/// covers an empty narrative too. Overflow is `Ok(PartIiCapacity::Overflow(_))`, not `Err` — it is an
/// expected, real outcome the caller builds ITS OWN refusal message from (naming the year, a remedy,
/// `--part-ii-file`); `Err` is reserved for an actual engine failure (an unsupported year, a bundled
/// map/PDF that fails to parse) the caller must still surface as such.
///
/// Exists so `btctax-cli`'s export paths (`crates/btctax-cli/src/cmd/admin.rs`,
/// `export_irs_pdf_from_session` + `export_full_return`) can refuse BEFORE `mkdir_out`/writing any
/// packet file — round 2 finding 2: without this, the crypto-slice path left an estimated-basis 8949
/// on disk with no 8275 PDF behind it when the narrative overflowed, because the overflow was only
/// discovered mid-write, inside `fill_form_8275_slice`, well after `basis_methodology.txt` and
/// `form_8275.txt` were already written.
pub fn part_ii_capacity_check(narrative: &str, year: i32) -> Result<PartIiCapacity, FormsError> {
    let map = Form8275Map::for_year(year)?;
    let blank_fields = pdf::collect_fields(&pdf::load(pdf::f8275_pdf(map.year)?)?)?;
    let (part_ii_width, part_iv_width) = part_ii_iv_widths(&blank_fields, &map)?;
    match wrap_part_ii(
        narrative,
        part_ii_width,
        part_iv_width,
        map.part_iv_continuation.len(),
        PART_II_FONT_SIZE_PT,
    ) {
        Ok(_) => Ok(PartIiCapacity::Fits),
        Err(overflow) => Ok(PartIiCapacity::Overflow(overflow)),
    }
}

/// The verdict [`part_ii_capacity_check`] returns.
#[derive(Debug, Clone, Copy)]
pub enum PartIiCapacity {
    /// The narrative fits (including an empty one).
    Fits,
    /// The narrative does not fit — detail for building a refusal message.
    Overflow(PartIiOverflow),
}
