//! Committed, per-(form, year) field maps: **logical cell → fully-qualified PDF field name**.
//!
//! The maps are DATA (TOML committed next to the bundled PDFs), not code — "adding a year" is a
//! `forms/<year>/` directory (PDF + maps), never a code change. Keys are the fully-qualified,
//! bracketed AcroForm names (`topmostSubform[0].Page1[0].Table_Line1_Part1[0].Row1[0].f1_03[0]`).
//!
//! Nothing here is trusted blindly: the geometric read-back ([`crate::verify`]) re-derives the
//! column/row bands from the bundled PDF's own widget `/Rect`s and would flag any mis-labeled cell,
//! and `map_2025_matches_bundled_pdf_fieldset` asserts every name here exists in the PDF.

use serde::Deserialize;

/// The TY2025 Form 8949 map (embedded at compile time).
pub const F8949_MAP_2025: &str = include_str!("../forms/2025/f8949.map.toml");
/// The TY2025 Schedule D map (embedded at compile time).
pub const SCHEDULE_D_MAP_2025: &str = include_str!("../forms/2025/schedule_d.map.toml");

/// The 4 monetary "amount" columns of a Form 8949 / Schedule D totals row: (d) proceeds, (e) cost,
/// (g) adjustment, (h) gain. Column (f) — the code column — has no total (a spacer), so it is absent.
#[derive(Debug, Clone, Deserialize)]
pub struct AmountCols {
    /// Column (d) — proceeds.
    pub proceeds_d: String,
    /// Column (e) — cost basis.
    pub cost_e: String,
    /// Column (g) — adjustment amount.
    pub adj_g: String,
    /// Column (h) — gain/loss.
    pub gain_h: String,
}

/// One Form 8949 part (Part I short-term on page 0, Part II long-term on page 1).
#[derive(Debug, Clone, Deserialize)]
pub struct PartMap {
    /// `"short"` (Part I) or `"long"` (Part II).
    pub term: String,
    /// 0-based page index of this part within the bundled 2-page PDF.
    pub page: usize,
    /// The digital-asset box checkbox field — **Box I** (ST) / **Box L** (LT), NOT C/F.
    pub box_field: String,
    /// The checkbox on-state (a PDF name without the leading `/`), e.g. `"6"`.
    pub box_on: String,
    /// The line-2 per-part totals row (d,e,g,h).
    pub totals: AmountCols,
    /// The 11 data rows; each row is the 8 column field names in order a,b,c,d,e,f,g,h.
    pub rows: Vec<Vec<String>>,
}

/// The full Form 8949 field map for one tax year.
#[derive(Debug, Clone, Deserialize)]
pub struct Form8949Map {
    /// `"f8949"`.
    pub form: String,
    /// Tax year (e.g. 2025).
    pub year: i32,
    /// Rows per part per page — **map data**, not a hard-coded constant (a new form revision that
    /// changes the grid is a data-only edit).
    pub rows_per_page: usize,
    /// Part I then Part II.
    pub parts: Vec<PartMap>,
}

impl Form8949Map {
    /// Parse the committed TOML.
    pub fn parse(toml_src: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_src)
    }

    /// The TY2025 map.
    pub fn ty2025() -> Self {
        Self::parse(F8949_MAP_2025).expect("bundled f8949 2025 map parses")
    }

    /// The part with the given term, if present.
    pub fn part(&self, term: &str) -> Option<&PartMap> {
        self.parts.iter().find(|p| p.term == term)
    }
}

/// A checkbox choice (field + on-state) — used for the Schedule D QOF Yes/No answer.
#[derive(Debug, Clone, Deserialize)]
pub struct CheckChoice {
    /// The checkbox field name.
    pub field: String,
    /// On-state PDF name (without leading `/`).
    pub on: String,
}

/// The Schedule D field map for one tax year.
#[derive(Debug, Clone, Deserialize)]
pub struct ScheduleDMap {
    /// `"schedule_d"`.
    pub form: String,
    /// Tax year.
    pub year: i32,
    /// Line 3 — Part I total from Form 8949 (Box C **or Box I**): columns d,e,g,h.
    pub line3: AmountCols,
    /// Line 7 — net short-term gain/loss (column h).
    pub line7_h: String,
    /// Line 10 — Part II total from Form 8949 (Box F **or Box L**): columns d,e,g,h.
    pub line10: AmountCols,
    /// Line 15 — net long-term gain/loss (column h).
    pub line15_h: String,
    /// Line 16 — total (line 7 + line 15), column h, page 2.
    pub line16_h: String,
    /// QOF question "Yes" choice.
    pub qof_yes: CheckChoice,
    /// QOF question "No" choice (SP1 answers No).
    pub qof_no: CheckChoice,
}

impl ScheduleDMap {
    /// Parse the committed TOML.
    pub fn parse(toml_src: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_src)
    }

    /// The TY2025 map.
    pub fn ty2025() -> Self {
        Self::parse(SCHEDULE_D_MAP_2025).expect("bundled schedule_d 2025 map parses")
    }
}
