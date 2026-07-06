//! **Geometric, map-INDEPENDENT read-back.** The naive "re-read each value through the same map"
//! is circular — a swapped-column map would pass. Instead we re-derive the column-x / row-y bands
//! straight from the bundled PDF's widget `/Rect`s (structural grouping by `Row{n}` subform, column
//! order by x-position, row order by y-position) and assert that every value we wrote landed in the
//! band its LOGICAL cell demands. A mis-mapped cell puts the value in the wrong band → we fail
//! closed. We also assert NO field outside the authorized set carries a value. **The map is what we
//! distrust; the PDF's geometry is the oracle.**

use crate::error::FormsError;
use crate::pdf::{button_on_states, checkbox_on, text_value, Field};
use lopdf::Document;
use std::collections::{HashMap, HashSet};

const EPS: f32 = 1.0;

/// Where a written value is supposed to land.
#[derive(Debug, Clone)]
pub enum Geo {
    /// A data-grid cell: 0-based row (top→bottom) and column (a=0 … h=7).
    Data {
        /// 0-based data row (Row1 = 0, the topmost).
        row: usize,
        /// 0-based column (a=0, b=1, … h=7).
        col: usize,
    },
    /// A per-part totals-row cell in column `col`, which must sit BELOW the data grid.
    Total {
        /// 0-based column (d=3, e=4, g=6, h=7).
        col: usize,
    },
    /// A checkbox / non-grid value — excluded from the column geometry, still in the no-unmapped set.
    Check,
}

/// One authorized write: the field we set, and the logical cell it must occupy.
#[derive(Debug, Clone)]
pub struct Placement {
    /// Fully-qualified field name that was written.
    pub fqn: String,
    /// The logical cell the value must land in.
    pub geo: Geo,
}

fn page_of(fqn: &str) -> usize {
    if fqn.contains("Page2") {
        1
    } else {
        0
    }
}

/// Extract the `Row{n}` number from a data-grid field's fqn.
fn row_num(fqn: &str) -> Option<u32> {
    let i = fqn.find(".Row")? + 4;
    let rest = &fqn[i..];
    let end = rest.find('[')?;
    rest[..end].parse().ok()
}

/// Column-x and row-y bands re-derived from one page's data grid — the geometry oracle.
struct GridBands {
    /// Per-column-index (0..=7) x-interval `(min_x0, max_x1)`, ordered left→right by geometry.
    col_x: Vec<(f32, f32)>,
    /// Per-row-index (0.., top→bottom by geometry) y-interval `(y0, y1)`.
    row_y: Vec<(f32, f32)>,
    /// The lowest data-row bottom edge — the totals row must sit below this.
    min_row_y0: f32,
}

fn derive_bands(fields: &[Field], page: usize, table_token: &str) -> Result<GridBands, FormsError> {
    // Group this page's data-grid widgets by structural Row number (independent of the map).
    let mut rows: HashMap<u32, Vec<&Field>> = HashMap::new();
    for f in fields {
        if page_of(&f.fqn) == page && f.fqn.contains(table_token) && f.rect.is_some() {
            if let Some(n) = row_num(&f.fqn) {
                rows.entry(n).or_default().push(f);
            }
        }
    }
    if rows.is_empty() {
        return Err(FormsError::Structure(format!(
            "page {page}: no data-grid widgets found for band derivation"
        )));
    }
    // Order rows top→bottom by widget y-center (geometry, NOT the Row number label).
    let mut ordered: Vec<(u32, Vec<&Field>)> = rows.into_iter().collect();
    let row_cy =
        |v: &[&Field]| -> f32 { v.iter().filter_map(|f| f.cy()).sum::<f32>() / (v.len() as f32) };
    ordered.sort_by(|a, b| row_cy(&b.1).partial_cmp(&row_cy(&a.1)).unwrap());

    let ncols = ordered[0].1.len();
    let mut col_x: Vec<(f32, f32)> = vec![(f32::INFINITY, f32::NEG_INFINITY); ncols];
    let mut row_y: Vec<(f32, f32)> = Vec::with_capacity(ordered.len());
    let mut min_row_y0 = f32::INFINITY;

    for (_n, mut widgets) in ordered {
        if widgets.len() != ncols {
            return Err(FormsError::Structure(format!(
                "page {page}: inconsistent column count ({} vs {ncols})",
                widgets.len()
            )));
        }
        // Column order is defined by x-position, purely from geometry.
        widgets.sort_by(|a, b| a.rect.unwrap()[0].partial_cmp(&b.rect.unwrap()[0]).unwrap());
        let mut y0 = f32::INFINITY;
        let mut y1 = f32::NEG_INFINITY;
        for (c, w) in widgets.iter().enumerate() {
            let r = w.rect.unwrap();
            col_x[c].0 = col_x[c].0.min(r[0]);
            col_x[c].1 = col_x[c].1.max(r[2]);
            y0 = y0.min(r[1]);
            y1 = y1.max(r[3]);
        }
        min_row_y0 = min_row_y0.min(y0);
        row_y.push((y0, y1));
    }
    Ok(GridBands {
        col_x,
        row_y,
        min_row_y0,
    })
}

/// Whether a coordinate lies within a band (with float tolerance).
pub fn in_band(v: f32, band: (f32, f32)) -> bool {
    v >= band.0 - EPS && v <= band.1 + EPS
}

/// Left→right x-bands of a table's amount columns, re-derived from the PDF geometry (independent of
/// any map). Used by the Schedule D read-back to catch a mis-mapped d/e/g/h column.
pub fn column_x_bands(
    fields: &[Field],
    page: usize,
    table_token: &str,
) -> Result<Vec<(f32, f32)>, FormsError> {
    Ok(derive_bands(fields, page, table_token)?.col_x)
}

/// Verify a Form 8949 fill: (1) every written value lands in the geometrically-expected column/row
/// band, and (2) no unmapped field carries a value. Fails closed. `table_token` is the per-year
/// data-grid subform token (`Table_Line1` for 2024, `Table_Line1_Part` for 2025).
pub fn verify_8949(
    doc: &Document,
    fields: &[Field],
    placements: &[Placement],
    table_token: &str,
) -> Result<(), FormsError> {
    let index: HashMap<&str, &Field> = fields.iter().map(|f| (f.fqn.as_str(), f)).collect();
    // Independently derive the geometry oracle for every page a placement touches (0 = Part I,
    // 1 = Part II) — up front, so a mis-mapped cell cannot dodge the check.
    let mut pages: Vec<usize> = placements
        .iter()
        .filter(|p| matches!(p.geo, Geo::Data { .. } | Geo::Total { .. }))
        .map(|p| page_of(&p.fqn))
        .collect();
    pages.sort_unstable();
    pages.dedup();
    let mut bands: HashMap<usize, GridBands> = HashMap::new();
    for page in pages {
        bands.insert(page, derive_bands(fields, page, table_token)?);
    }

    for p in placements {
        let field = index
            .get(p.fqn.as_str())
            .ok_or_else(|| FormsError::MapFieldMissing(p.fqn.clone()))?;
        match &p.geo {
            Geo::Check => {} // geometry N/A; only participates in the no-unmapped scan below
            Geo::Data { row, col } => {
                let page = page_of(&p.fqn);
                let b = &bands[&page];
                let cx = field.cx().ok_or_else(|| miss_rect(&p.fqn))?;
                let cy = field.cy().ok_or_else(|| miss_rect(&p.fqn))?;
                let colb = *b.col_x.get(*col).ok_or_else(|| {
                    FormsError::Geometry(format!("column {col} out of range on page {page}"))
                })?;
                let rowb = *b.row_y.get(*row).ok_or_else(|| {
                    FormsError::Geometry(format!("row {row} out of range on page {page}"))
                })?;
                if !in_band(cx, colb) {
                    return Err(FormsError::Geometry(format!(
                        "{}: x-center {cx:.1} not in column {col} band {colb:?} (mis-mapped column)",
                        p.fqn
                    )));
                }
                if !in_band(cy, rowb) {
                    return Err(FormsError::Geometry(format!(
                        "{}: y-center {cy:.1} not in row {row} band {rowb:?} (mis-mapped row)",
                        p.fqn
                    )));
                }
            }
            Geo::Total { col } => {
                let page = page_of(&p.fqn);
                let b = &bands[&page];
                let cx = field.cx().ok_or_else(|| miss_rect(&p.fqn))?;
                let cy = field.cy().ok_or_else(|| miss_rect(&p.fqn))?;
                let colb = *b.col_x.get(*col).ok_or_else(|| {
                    FormsError::Geometry(format!("total column {col} out of range on page {page}"))
                })?;
                if !in_band(cx, colb) {
                    return Err(FormsError::Geometry(format!(
                        "{}: total x-center {cx:.1} not in column {col} band {colb:?}",
                        p.fqn
                    )));
                }
                if cy >= b.min_row_y0 {
                    return Err(FormsError::Geometry(format!(
                        "{}: total y-center {cy:.1} is not below the data grid (>= {:.1})",
                        p.fqn, b.min_row_y0
                    )));
                }
            }
        }
    }
    no_unmapped_filled(doc, fields, placements)
}

fn miss_rect(fqn: &str) -> FormsError {
    FormsError::Geometry(format!("{fqn}: field has no /Rect to verify"))
}

/// Assert that every field carrying a value is in the authorized (placement) set.
pub fn no_unmapped_filled(
    doc: &Document,
    fields: &[Field],
    placements: &[Placement],
) -> Result<(), FormsError> {
    let allowed: HashSet<&str> = placements.iter().map(|p| p.fqn.as_str()).collect();
    assert_only_filled(doc, fields, &allowed)
}

/// The form-agnostic core of the no-unmapped guard: every filled field must be in `allowed`. Shared by
/// the SP1 [`no_unmapped_filled`] and the SP2 flat-form verifier.
pub fn assert_only_filled(
    doc: &Document,
    fields: &[Field],
    allowed: &HashSet<&str>,
) -> Result<(), FormsError> {
    for f in fields {
        let filled = if f.is_button {
            checkbox_on(doc, f.id).is_some()
        } else {
            text_value(doc, f.id).is_some_and(|s| !s.is_empty())
        };
        if filled && !allowed.contains(f.fqn.as_str()) {
            return Err(FormsError::UnmappedField(f.fqn.clone()));
        }
    }
    Ok(())
}

// ── [★ R0-C3] SP2 per-form geometric oracle for the FLAT (non-grid) forms ─────────────────────────
//
// Schedule SE / Form 8283 / Form 1040 have no `Row{n}` data-grid subform, so the SP1 grid oracle does
// not fit. Instead this oracle asserts, map-INDEPENDENTLY, that each written value's widget landed:
//   (1) in its logical column's hand-pinned x-cluster (measured from the blank PDF; catches a
//       cross-column swap, e.g. SE line 12 (amount) ↔ line 13 (mid));
//   (2) in strictly-descending center-y within a logically ordered sequence (catches a same-column
//       swap, e.g. SE 10 ↔ 11) — asserted PER descent GROUP so 8283's two-table columns [R0-M1] each
//       descend within their own field set;
//   (3) for the 1040 Digital-Asset question, that the "Yes" value is the LEFT member of the top-most
//       same-y `/Btn` pair whose on-states are exactly {`/1`,`/2`} (a [`topmost_yes_no_pair`] check);
//   (4) on the expected page; 1-pt preprinted-constant spacer fields are never map targets.
// The map is what we distrust — a mis-mapped cell lands in the wrong cluster / breaks monotonicity and
// FAILS CLOSED. `assert_only_filled` still guards against any stray write.

/// One authorized SP2 write. `col` indexes a hand-pinned column-x cluster (`None` = geometry-exempt,
/// e.g. a wide free-text identity field); `descent` = `(group, ordinal)` for per-group strictly-
/// descending-y ordering (`None` = not in any ordered sequence); `check` marks a checkbox (only in the
/// no-unmapped scan + any same-y-pair predicate).
#[derive(Debug, Clone)]
pub struct FlatPlacement {
    /// Fully-qualified field name that was written.
    pub fqn: String,
    /// 0-based page the write must land on.
    pub page: usize,
    /// Logical column index into the per-form hand-pinned x-cluster table (`None` = not column-checked).
    pub col: Option<usize>,
    /// `(descent_group, ordinal)` — within a group, center-y must strictly decrease as ordinal rises.
    pub descent: Option<(u32, u32)>,
    /// `true` iff this is a checkbox (geometry-exempt; still in the no-unmapped set).
    pub check: bool,
}

impl FlatPlacement {
    /// A column-checked, descent-participating money/text cell.
    pub fn cell(fqn: impl Into<String>, page: usize, col: usize, grp: u32, ord: u32) -> Self {
        Self {
            fqn: fqn.into(),
            page,
            col: Some(col),
            descent: Some((grp, ord)),
            check: false,
        }
    }
    /// A column-checked cell that does NOT participate in any descent sequence.
    pub fn col_only(fqn: impl Into<String>, page: usize, col: usize) -> Self {
        Self {
            fqn: fqn.into(),
            page,
            col: Some(col),
            descent: None,
            check: false,
        }
    }
    /// A geometry-exempt write (wide free-text identity field): page-checked + no-unmapped only.
    pub fn free(fqn: impl Into<String>, page: usize) -> Self {
        Self {
            fqn: fqn.into(),
            page,
            col: None,
            descent: None,
            check: false,
        }
    }
    /// A checkbox: no-unmapped only (+ any same-y-pair predicate the caller runs).
    pub fn check(fqn: impl Into<String>, page: usize) -> Self {
        Self {
            fqn: fqn.into(),
            page,
            col: None,
            descent: None,
            check: true,
        }
    }
}

/// Verify a flat-form fill: page membership + hand-pinned column-x membership + per-group ordinal-y
/// descent + the no-unmapped scan. `clusters` is the per-form hand-pinned logical-column → `(min_x0,
/// max_x1)` table (measured from the blank PDF). Fails closed.
pub fn verify_flat(
    doc: &Document,
    fields: &[Field],
    placements: &[FlatPlacement],
    clusters: &[(f32, f32)],
) -> Result<(), FormsError> {
    let index: HashMap<&str, &Field> = fields.iter().map(|f| (f.fqn.as_str(), f)).collect();

    // (4)+(1) page membership + column-x membership.
    for p in placements {
        let field = index
            .get(p.fqn.as_str())
            .ok_or_else(|| FormsError::MapFieldMissing(p.fqn.clone()))?;
        if page_of(&p.fqn) != p.page {
            return Err(FormsError::Geometry(format!(
                "{}: field is on page {} but placement expected page {}",
                p.fqn,
                page_of(&p.fqn),
                p.page
            )));
        }
        if let Some(col) = p.col {
            let cx = field.cx().ok_or_else(|| miss_rect(&p.fqn))?;
            let cluster = *clusters.get(col).ok_or_else(|| {
                FormsError::Geometry(format!(
                    "column {col} out of range (clusters={})",
                    clusters.len()
                ))
            })?;
            if !in_band(cx, cluster) {
                return Err(FormsError::Geometry(format!(
                    "{}: x-center {cx:.1} not in column {col} cluster {cluster:?} (mis-mapped column)",
                    p.fqn
                )));
            }
        }
    }

    // (2) ordinal-y descent, per group.
    let mut groups: HashMap<u32, Vec<(u32, f32, &str)>> = HashMap::new();
    for p in placements {
        if let Some((grp, ord)) = p.descent {
            let cy = index[p.fqn.as_str()]
                .cy()
                .ok_or_else(|| miss_rect(&p.fqn))?;
            groups
                .entry(grp)
                .or_default()
                .push((ord, cy, p.fqn.as_str()));
        }
    }
    for seq in groups.values_mut() {
        seq.sort_by_key(|(ord, _, _)| *ord);
        for w in seq.windows(2) {
            // Earlier ordinal must sit strictly ABOVE (higher center-y) the next.
            if w[0].1 <= w[1].1 + EPS {
                return Err(FormsError::Geometry(format!(
                    "ordinal-y descent broken: {} (y {:.1}) is not strictly above {} (y {:.1}) — mis-mapped row/line",
                    w[0].2, w[0].1, w[1].2, w[1].1
                )));
            }
        }
    }

    // (3) no unmapped write.
    let allowed: HashSet<&str> = placements.iter().map(|p| p.fqn.as_str()).collect();
    assert_only_filled(doc, fields, &allowed)
}

/// Maximum horizontal gap (widget-center to widget-center, PDF points) between the two boxes of the
/// Digital-Asset Yes/No pair for them to count as **adjacent**. The real DA "Yes"/"No" boxes sit ~36pt
/// apart on both the 2024 and 2025 1040; the 2024 **filing-status** `{/1,/2}` row (Single vs MFJ) is a
/// same-y pair too but its boxes are ~266pt apart — the trap the top-most-y rule fell into. 80pt
/// brackets the real gap with margin while excluding that non-adjacent row.
const DA_PAIR_MAX_DX: f32 = 80.0;

/// Map-INDEPENDENT oracle for the 1040 Digital-Asset Yes/No question: the **top-most horizontally
/// ADJACENT** page-`page` `/Btn` pair (exactly two widgets sharing a center-y, boxes ≤ [`DA_PAIR_MAX_DX`]
/// apart) whose on-states are exactly {`/1`,`/2`}. Returns `(yes_fqn, no_fqn)` = (LEFT member, right
/// member). Derived from the blank PDF's widget geometry + appearance states, never the map.
///
/// **[R0-C2]** Selecting by adjacency (not merely top-most-y) is what keeps the 2024 fill off the
/// FILING-STATUS `{/1,/2}` row (Single @ x≈107 vs MFJ @ x≈373, ~266pt apart) that sits ABOVE the DA
/// pair; the DA "Yes"/"No" boxes are ~36pt apart. Re-verified against 2025 (its DA pair is the top-most
/// `{/1,/2}` 2-widget row AND adjacent, so no regression).
pub fn topmost_yes_no_pair(
    doc: &Document,
    fields: &[Field],
    page: usize,
) -> Result<(String, String), FormsError> {
    // Group page buttons (that carry a rect + on-states) by rounded center-y.
    let mut by_y: HashMap<i32, Vec<(&Field, Vec<String>)>> = HashMap::new();
    for f in fields {
        if !f.is_button || page_of(&f.fqn) != page {
            continue;
        }
        let Some(cy) = f.cy() else { continue };
        let states = button_on_states(doc, f.id);
        if states.is_empty() {
            continue;
        }
        by_y.entry(cy.round() as i32).or_default().push((f, states));
    }
    // A qualifying row: EXACTLY two widgets whose combined on-states are exactly {"1","2"} AND whose
    // boxes are horizontally ADJACENT (≤ DA_PAIR_MAX_DX apart).
    let mut candidates: Vec<(f32, &Field, &Field)> = Vec::new();
    for members in by_y.values() {
        if members.len() != 2 {
            continue;
        }
        let mut states: Vec<&str> = members
            .iter()
            .flat_map(|(_, s)| s.iter().map(|x| x.as_str()))
            .collect();
        states.sort_unstable();
        if states != ["1", "2"] {
            continue;
        }
        let (a, b) = (members[0].0, members[1].0);
        if (a.cx().unwrap() - b.cx().unwrap()).abs() > DA_PAIR_MAX_DX {
            continue; // non-adjacent (e.g. the 2024 filing-status row) — not the DA pair.
        }
        let cy = a.cy().unwrap();
        candidates.push((cy, a, b));
    }
    // Top-most (largest center-y) AMONG the adjacent pairs.
    candidates.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap());
    let (_, a, b) = candidates.first().ok_or_else(|| {
        FormsError::Geometry(format!(
            "no adjacent same-y {{/1,/2}} /Btn pair found on page {page}"
        ))
    })?;
    // Left member = the Yes box.
    if a.cx().unwrap() <= b.cx().unwrap() {
        Ok((a.fqn.clone(), b.fqn.clone()))
    } else {
        Ok((b.fqn.clone(), a.fqn.clone()))
    }
}
