//! Committed, per-(form, year) field maps: **logical cell → fully-qualified PDF field name**.
//!
//! The maps are DATA (TOML committed next to the bundled PDFs), not code — "adding a year" is a
//! `forms/<year>/` directory (PDF + maps), never a code change. Keys are the fully-qualified,
//! bracketed AcroForm names (`topmostSubform[0].Page1[0].Table_Line1_Part1[0].Row1[0].f1_03[0]`).
//!
//! Nothing here is trusted blindly: the geometric read-back ([`crate::verify`]) re-derives the
//! column/row bands from the bundled PDF's own widget `/Rect`s and would flag any mis-labeled cell,
//! and `map_2025_matches_bundled_pdf_fieldset` asserts every name here exists in the PDF.

use crate::error::FormsError;
use serde::Deserialize;

/// The TY2025 Form 8949 map (embedded at compile time).
pub const F8949_MAP_2025: &str = include_str!("../forms/2025/f8949.map.toml");
/// The TY2025 Schedule D map (embedded at compile time).
pub const SCHEDULE_D_MAP_2025: &str = include_str!("../forms/2025/schedule_d.map.toml");
/// The TY2025 Schedule SE map (embedded at compile time).
pub const SCHEDULE_SE_MAP_2025: &str = include_str!("../forms/2025/schedule_se.map.toml");
/// The TY2025 Form 8283 map (embedded at compile time).
pub const F8283_MAP_2025: &str = include_str!("../forms/2025/f8283.map.toml");
/// The TY2025 Form 1040 map (embedded at compile time).
pub const F1040_MAP_2025: &str = include_str!("../forms/2025/f1040.map.toml");

/// The TY2024 Form 8949 map (embedded at compile time).
pub const F8949_MAP_2024: &str = include_str!("../forms/2024/f8949.map.toml");
/// The TY2024 Schedule D map (embedded at compile time).
pub const SCHEDULE_D_MAP_2024: &str = include_str!("../forms/2024/schedule_d.map.toml");
/// The TY2024 Schedule SE map (embedded at compile time).
pub const SCHEDULE_SE_MAP_2024: &str = include_str!("../forms/2024/schedule_se.map.toml");
/// The TY2024 Form 8283 map (Rev. 12-2023, embedded at compile time).
pub const F8283_MAP_2024: &str = include_str!("../forms/2024/f8283.map.toml");
/// The TY2024 Form 1040 map (embedded at compile time).
pub const F1040_MAP_2024: &str = include_str!("../forms/2024/f1040.map.toml");

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
    /// The data-grid subform token used to re-derive the geometry bands — **per-year map config**,
    /// not a const (2024 = `Table_Line1`, 2025 = `Table_Line1_Part`; the row fqns differ by year).
    pub table_token: String,
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

    /// The TY2024 map.
    pub fn ty2024() -> Self {
        Self::parse(F8949_MAP_2024).expect("bundled f8949 2024 map parses")
    }

    /// The map for a supported tax year.
    pub fn for_year(year: i32) -> Result<Self, FormsError> {
        match year {
            2024 => Ok(Self::ty2024()),
            2025 => Ok(Self::ty2025()),
            _ => Err(FormsError::UnsupportedYear(year)),
        }
    }

    /// The part with the given term, if present.
    pub fn part(&self, term: &str) -> Option<&PartMap> {
        self.parts.iter().find(|p| p.term == term)
    }
}

/// A checkbox choice (field + on-state) — used for the Schedule D QOF Yes/No answer and the Form 1040
/// Digital-Asset Yes/No question.
#[derive(Debug, Clone, Deserialize)]
pub struct CheckChoice {
    /// The checkbox field name.
    pub field: String,
    /// On-state PDF name (without leading `/`).
    pub on: String,
}

/// A per-year default: the Digital-Asset question is present unless a year's map says otherwise.
fn default_da_present() -> bool {
    true
}

/// The Form 1040 capital-gains field map for one tax year: the capital-gain amount cell (line 7a in
/// 2025 / line 7 in 2024) + the Digital-Asset question.
#[derive(Debug, Clone, Deserialize)]
pub struct Form1040Map {
    /// `"f1040"`.
    pub form: String,
    /// Tax year.
    pub year: i32,
    /// The capital-gain amount cell (line 7a for 2025, line 7 for 2024), amount column.
    pub line7a: String,
    /// Whether this year's 1040 carries the Digital-Asset question — **per-year scaffolding**. When
    /// `true` (2024/2025) the fill answers it "Yes" and runs the map-independent adjacency guard; a
    /// future no-DA year (2017) sets it `false` (SP3b then makes `da_yes`/`da_no` optional).
    #[serde(default = "default_da_present")]
    pub da_present: bool,
    /// Digital-Asset question "Yes" (LEFT member of the adjacent pair, on-state `/1`).
    pub da_yes: CheckChoice,
    /// Digital-Asset question "No" (right member, on-state `/2`) — never checked by btctax.
    pub da_no: CheckChoice,
}

impl Form1040Map {
    /// Parse the committed TOML.
    pub fn parse(toml_src: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_src)
    }

    /// The TY2025 map.
    pub fn ty2025() -> Self {
        Self::parse(F1040_MAP_2025).expect("bundled f1040 2025 map parses")
    }

    /// The TY2024 map.
    pub fn ty2024() -> Self {
        Self::parse(F1040_MAP_2024).expect("bundled f1040 2024 map parses")
    }

    /// The map for a supported tax year.
    pub fn for_year(year: i32) -> Result<Self, FormsError> {
        match year {
            2024 => Ok(Self::ty2024()),
            2025 => Ok(Self::ty2025()),
            _ => Err(FormsError::UnsupportedYear(year)),
        }
    }
}

/// One Form 8283 **Section A** row (Donated Property of $5,000 or Less): the 8 filled columns.
#[derive(Debug, Clone, Deserialize)]
pub struct Section8283ARow {
    /// (a) Name and address of the donee organization.
    pub donee: String,
    /// (c) Description and condition of donated property.
    pub desc: String,
    /// (d) Date of the contribution (full date).
    pub date_contrib: String,
    /// (e) Date acquired by donor (mo., yr.).
    pub date_acq: String,
    /// (f) How acquired by donor.
    pub how: String,
    /// (g) Donor's cost or adjusted basis.
    pub cost: String,
    /// (h) Fair market value.
    pub fmv: String,
    /// (i) Method used to determine the FMV.
    pub method: String,
}

/// Form 8283 Section A (page 1, Line 1) — up to 4 rows A–D.
#[derive(Debug, Clone, Deserialize)]
pub struct Section8283A {
    /// The 4 rows A–D.
    pub rows: Vec<Section8283ARow>,
}

/// One Form 8283 **Section B Part I** row (Over $5,000): the filled columns.
#[derive(Debug, Clone, Deserialize)]
pub struct Section8283BRow {
    /// (a) Description of donated property.
    pub desc: String,
    /// (c) Appraised fair market value.
    pub fmv: String,
    /// (d) Date acquired by donor (mo., yr.).
    pub date_acq: String,
    /// (e) How acquired by donor.
    pub how: String,
    /// (f) Donor's cost or adjusted basis.
    pub cost: String,
    /// (i) Amount claimed as a deduction (carrier row only).
    pub deduction: String,
}

/// Form 8283 Section B (page 1, Line 3 + page 2 identity) — up to 3 rows A–C.
#[derive(Debug, Clone, Deserialize)]
pub struct Section8283B {
    /// Line 2 "k Digital assets" property-type checkbox (MUST be checked for BTC; on-state `/11`).
    pub k_digital_assets: CheckChoice,
    /// Part IV appraiser name (page 2).
    pub appraiser_name: String,
    /// Part IV appraiser business address (page 2).
    pub appraiser_address: String,
    /// Part IV appraiser identifying number (TIN/PTIN, page 2).
    pub appraiser_tin: String,
    /// Part V donee organization name (page 2).
    pub donee_name: String,
    /// Part V donee EIN (page 2).
    pub donee_ein: String,
    /// Part V donee address (page 2).
    pub donee_address: String,
    /// The 3 rows A–C.
    pub rows: Vec<Section8283BRow>,
}

/// The Form 8283 (Rev. 12-2025) field map for one tax year.
#[derive(Debug, Clone, Deserialize)]
pub struct Form8283Map {
    /// `"f8283"`.
    pub form: String,
    /// Tax year.
    pub year: i32,
    /// Section A (≤ $5,000).
    pub section_a: Section8283A,
    /// Section B (> $5,000).
    pub section_b: Section8283B,
}

impl Form8283Map {
    /// Parse the committed TOML.
    pub fn parse(toml_src: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_src)
    }

    /// The TY2025 map.
    pub fn ty2025() -> Self {
        Self::parse(F8283_MAP_2025).expect("bundled f8283 2025 map parses")
    }

    /// The TY2024 map (Form 8283 Rev. 12-2023).
    pub fn ty2024() -> Self {
        Self::parse(F8283_MAP_2024).expect("bundled f8283 2024 map parses")
    }

    /// The map for a supported tax year.
    pub fn for_year(year: i32) -> Result<Self, FormsError> {
        match year {
            2024 => Ok(Self::ty2024()),
            2025 => Ok(Self::ty2025()),
            _ => Err(FormsError::UnsupportedYear(year)),
        }
    }

    /// Every field name the map targets (for the `map_2025_matches_bundled_pdf_fieldset` guard).
    pub fn field_names(&self) -> Vec<&str> {
        let mut v = Vec::new();
        for r in &self.section_a.rows {
            v.extend([
                r.donee.as_str(),
                r.desc.as_str(),
                r.date_contrib.as_str(),
                r.date_acq.as_str(),
                r.how.as_str(),
                r.cost.as_str(),
                r.fmv.as_str(),
                r.method.as_str(),
            ]);
        }
        let b = &self.section_b;
        v.extend([
            b.k_digital_assets.field.as_str(),
            b.appraiser_name.as_str(),
            b.appraiser_address.as_str(),
            b.appraiser_tin.as_str(),
            b.donee_name.as_str(),
            b.donee_ein.as_str(),
            b.donee_address.as_str(),
        ]);
        for r in &b.rows {
            v.extend([
                r.desc.as_str(),
                r.fmv.as_str(),
                r.date_acq.as_str(),
                r.how.as_str(),
                r.cost.as_str(),
                r.deduction.as_str(),
            ]);
        }
        v
    }
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

    /// The TY2024 map.
    pub fn ty2024() -> Self {
        Self::parse(SCHEDULE_D_MAP_2024).expect("bundled schedule_d 2024 map parses")
    }

    /// The map for a supported tax year.
    pub fn for_year(year: i32) -> Result<Self, FormsError> {
        match year {
            2024 => Ok(Self::ty2024()),
            2025 => Ok(Self::ty2025()),
            _ => Err(FormsError::UnsupportedYear(year)),
        }
    }
}

/// The Schedule SE (Form 1040) field map for one tax year — the filled §1401 line chain.
#[derive(Debug, Clone, Deserialize)]
pub struct ScheduleSeMap {
    /// `"schedule_se"`.
    pub form: String,
    /// Tax year.
    pub year: i32,
    /// Line 2 — net profit (net_se), amount column.
    pub line2: String,
    /// Line 3 — combine 1a/1b/2 (= line 2), amount column.
    pub line3: String,
    /// Line 4a — net SE earnings (base = net_se × 92.35%), amount column.
    pub line4a: String,
    /// Line 4c — combine 4a/4b (= line 4a), amount column. The $400 STOP threshold.
    pub line4c: String,
    /// Line 6 — add 4c/5b (= line 4c), amount column.
    pub line6: String,
    /// Line 8a — Form W-2 Social Security wages, **MID column**.
    pub line8a: String,
    /// Line 8d — add 8a/8b/8c (= line 8a), amount column.
    pub line8d: String,
    /// Line 9 — line 7 (`ss_wage_base` constant) − line 8d, amount column.
    pub line9: String,
    /// Line 10 — Social Security portion (`ss`), amount column.
    pub line10: String,
    /// Line 11 — regular Medicare portion (`medicare`), amount column.
    pub line11: String,
    /// Line 12 — SE tax = line 10 + line 11 (**SS + regular Medicare ONLY**), amount column.
    pub line12: String,
    /// Line 13 — one-half SE-tax deduction (= line 12 × 50% = `deductible_half`), **MID column**.
    pub line13: String,
}

impl ScheduleSeMap {
    /// Parse the committed TOML.
    pub fn parse(toml_src: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_src)
    }

    /// The TY2025 map.
    pub fn ty2025() -> Self {
        Self::parse(SCHEDULE_SE_MAP_2025).expect("bundled schedule_se 2025 map parses")
    }

    /// The TY2024 map (field-name-identical to 2025; only the wage base differs).
    pub fn ty2024() -> Self {
        Self::parse(SCHEDULE_SE_MAP_2024).expect("bundled schedule_se 2024 map parses")
    }

    /// The map for a supported tax year.
    pub fn for_year(year: i32) -> Result<Self, FormsError> {
        match year {
            2024 => Ok(Self::ty2024()),
            2025 => Ok(Self::ty2025()),
            _ => Err(FormsError::UnsupportedYear(year)),
        }
    }

    /// Every field name the map targets (for the `map_2025_matches_bundled_pdf_fieldset` guard).
    pub fn field_names(&self) -> Vec<&str> {
        vec![
            &self.line2,
            &self.line3,
            &self.line4a,
            &self.line4c,
            &self.line6,
            &self.line8a,
            &self.line8d,
            &self.line9,
            &self.line10,
            &self.line11,
            &self.line12,
            &self.line13,
        ]
    }
}
