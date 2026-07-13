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
/// The TY2024 Form 8959 (Additional Medicare Tax) map (embedded at compile time).
pub const F8959_MAP_2024: &str = include_str!("../forms/2024/f8959.map.toml");
/// The TY2024 Form 8960 (Net Investment Income Tax) map (embedded at compile time).
pub const F8960_MAP_2024: &str = include_str!("../forms/2024/f8960.map.toml");
/// The TY2024 Form 8995 (QBI deduction, simplified) map (embedded at compile time).
pub const F8995_MAP_2024: &str = include_str!("../forms/2024/f8995.map.toml");
/// The TY2024 Schedule 2 (Additional Taxes) map (embedded at compile time).
pub const SCHEDULE_2_MAP_2024: &str = include_str!("../forms/2024/f1040s2.map.toml");
/// The TY2024 Schedule 3 (Additional Credits and Payments) map (embedded at compile time).
pub const SCHEDULE_3_MAP_2024: &str = include_str!("../forms/2024/f1040s3.map.toml");
/// The TY2024 Schedule A (Itemized Deductions) map (embedded at compile time).
pub const SCHEDULE_A_MAP_2024: &str = include_str!("../forms/2024/f1040sa.map.toml");

/// The TY2017 Form 8949 map (embedded at compile time).
pub const F8949_MAP_2017: &str = include_str!("../forms/2017/f8949.map.toml");
/// The TY2017 Schedule D map (embedded at compile time).
pub const SCHEDULE_D_MAP_2017: &str = include_str!("../forms/2017/schedule_d.map.toml");
/// The TY2017 Schedule SE map (OLD short+long form; btctax fills §B long — embedded at compile time).
pub const SCHEDULE_SE_MAP_2017: &str = include_str!("../forms/2017/schedule_se.map.toml");
/// The TY2017 Form 8283 map (Rev. 12-2014, "j Other" — embedded at compile time).
pub const F8283_MAP_2017: &str = include_str!("../forms/2017/f8283.map.toml");
/// The TY2017 Form 1040 map (line 13, no DA question — embedded at compile time).
pub const F1040_MAP_2017: &str = include_str!("../forms/2017/f1040.map.toml");

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

    /// The TY2017 map (pre-1099-DA: Box C/F, `/3`; field-identical grid to 2024).
    pub fn ty2017() -> Self {
        Self::parse(F8949_MAP_2017).expect("bundled f8949 2017 map parses")
    }

    /// The map for a supported tax year.
    pub fn for_year(year: i32) -> Result<Self, FormsError> {
        match year {
            2017 => Ok(Self::ty2017()),
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

/// A dollars-field + cents-field PAIR (the 2017 Schedule SE / Form 1040 / Form 8283 split every money
/// amount into a whole-dollars field and a 2-digit cents field). The geometric oracle treats the pair
/// as ONE logical cell **at the dollars-field geometry** (the cents field rides along as an authorized
/// but geometry-exempt write). Because both fields descend from the same AcroForm root, `merge_copies`
/// (which renames only the root `/T`) rewrites BOTH names as a unit — so overflow is safe.
#[derive(Debug, Clone, Deserialize)]
pub struct MoneyPair {
    /// The whole-dollars field (the one the geometry oracle checks — column-x + row/descent).
    pub dollars_field: String,
    /// The 2-digit cents field (an authorized write; NOT independently geometry-checked).
    pub cents_field: String,
}

/// A monetary cell: a single field carrying the whole formatted amount (2024/2025), or a
/// dollars+cents [`MoneyPair`] (the 2017 forms). Deserializes untagged: a TOML **string** →
/// [`MoneyCell::Single`]; a TOML **inline table** `{ dollars_field, cents_field }` →
/// [`MoneyCell::Pair`].
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum MoneyCell {
    /// A single field holding the whole formatted amount.
    Single(String),
    /// A dollars-field + cents-field pair.
    Pair(MoneyPair),
}

impl MoneyCell {
    /// Every PDF field this cell targets (1 for a single, 2 for a pair) — for coverage guards.
    pub fn fields(&self) -> Vec<&str> {
        match self {
            MoneyCell::Single(f) => vec![f.as_str()],
            MoneyCell::Pair(p) => vec![p.dollars_field.as_str(), p.cents_field.as_str()],
        }
    }
}

/// A per-year default: the Digital-Asset question is present unless a year's map says otherwise.
fn default_da_present() -> bool {
    true
}

/// The Form 1040 capital-gains field map for one tax year: the capital-gain amount cell (line 7a in
/// 2025 / line 7 in 2024 / **line 13** in 2017) + the Digital-Asset question (absent in 2017).
#[derive(Debug, Clone, Deserialize)]
pub struct Form1040Map {
    /// `"f1040"`.
    pub form: String,
    /// Tax year.
    pub year: i32,
    /// The capital-gain amount cell (line 7a for 2025, line 7 for 2024, **line 13 for 2017**). A
    /// single field on 2024/2025; a dollars+cents [`MoneyPair`] on the 2017 form.
    pub line7a: MoneyCell,
    /// Whether this year's 1040 carries the Digital-Asset question — **per-year scaffolding**. When
    /// `true` (2024/2025) the fill answers it "Yes" and runs the map-independent adjacency guard;
    /// **2017 sets it `false`** (no DA question — the map omits `da_yes`/`da_no` and the fill produces
    /// the 1040 iff there is reportable capital activity).
    #[serde(default = "default_da_present")]
    pub da_present: bool,
    /// Digital-Asset question "Yes" (LEFT member of the adjacent pair, on-state `/1`). `None` when the
    /// year's 1040 has no DA question (2017).
    #[serde(default)]
    pub da_yes: Option<CheckChoice>,
    /// Digital-Asset question "No" (right member, on-state `/2`) — never checked by btctax. `None`
    /// when the year's 1040 has no DA question (2017).
    #[serde(default)]
    pub da_no: Option<CheckChoice>,
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

    /// The TY2017 map (capital gain on line 13; NO Digital-Asset question).
    pub fn ty2017() -> Self {
        Self::parse(F1040_MAP_2017).expect("bundled f1040 2017 map parses")
    }

    /// The map for a supported tax year.
    pub fn for_year(year: i32) -> Result<Self, FormsError> {
        match year {
            2017 => Ok(Self::ty2017()),
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
    /// (g) Donor's cost or adjusted basis (money — a [`MoneyPair`] on the 2017 Rev. 12-2014 form).
    pub cost: MoneyCell,
    /// (h) Fair market value (money — a [`MoneyPair`] on the 2017 form).
    pub fmv: MoneyCell,
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
    /// (c) Appraised fair market value (money — a [`MoneyPair`] on the 2017 Rev. 12-2014 form).
    pub fmv: MoneyCell,
    /// (d) Date acquired by donor (mo., yr.).
    pub date_acq: String,
    /// (e) How acquired by donor.
    pub how: String,
    /// (f) Donor's cost or adjusted basis (money — a [`MoneyPair`] on the 2017 form).
    pub cost: MoneyCell,
    /// (i)/(h) Amount claimed as a deduction (carrier row only; money — a [`MoneyPair`] on 2017).
    pub deduction: MoneyCell,
}

/// Form 8283 Section B (page 1/2, over-$5,000 property + page 2 identity) — up to 3 rows (2024/2025)
/// or 4 rows (2017 Rev. 12-2014, `Line5A`–`Line5D`).
#[derive(Debug, Clone, Deserialize)]
pub struct Section8283B {
    /// The property-type checkbox MUST be checked for BTC: **"k Digital assets"** (on-state `/11`) on
    /// the Rev. 12-2023/2025 forms; the Rev. 12-2014 form has no digital-asset box, so 2017 uses
    /// **"j Other"** (on-state `/9`) plus [`Self::btc_property_note`].
    pub k_digital_assets: CheckChoice,
    /// 2017 only: since "j Other" gives no category, the digital-asset nature is identified by a
    /// printed note **prepended to the first row's (a) description** (e.g. "Other property: digital
    /// asset (virtual currency)"). `None` on 2024/2025 ("k Digital assets" is self-describing).
    #[serde(default)]
    pub btc_property_note: Option<String>,
    /// Part IV/III appraiser name (page 2). `None` when the revision has no printed-name field (the
    /// Rev. 12-2014 form: the appraiser identity is the handwritten signature, left blank).
    #[serde(default)]
    pub appraiser_name: Option<String>,
    /// Appraiser business address (page 2).
    pub appraiser_address: String,
    /// Appraiser identifying number (TIN/PTIN, page 2).
    pub appraiser_tin: String,
    /// Donee organization name (page 2).
    pub donee_name: String,
    /// Donee EIN (page 2).
    pub donee_ein: String,
    /// Donee address (page 2).
    pub donee_address: String,
    /// The rows (3 on 2024/2025, 4 on 2017) — the row count also sets the per-copy overflow cap.
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

    /// The TY2017 map (Form 8283 Rev. 12-2014 — "j Other", no DA box, 5/4 rows, ¢-pairs).
    pub fn ty2017() -> Self {
        Self::parse(F8283_MAP_2017).expect("bundled f8283 2017 map parses")
    }

    /// The map for a supported tax year.
    pub fn for_year(year: i32) -> Result<Self, FormsError> {
        match year {
            2017 => Ok(Self::ty2017()),
            2024 => Ok(Self::ty2024()),
            2025 => Ok(Self::ty2025()),
            _ => Err(FormsError::UnsupportedYear(year)),
        }
    }

    /// Every field name the map targets (for the `map_YYYY_matches_bundled_pdf_fieldset` guard).
    pub fn field_names(&self) -> Vec<&str> {
        let mut v = Vec::new();
        for r in &self.section_a.rows {
            v.extend([
                r.donee.as_str(),
                r.desc.as_str(),
                r.date_contrib.as_str(),
                r.date_acq.as_str(),
                r.how.as_str(),
            ]);
            v.extend(r.cost.fields());
            v.extend(r.fmv.fields());
            v.push(r.method.as_str());
        }
        let b = &self.section_b;
        v.push(b.k_digital_assets.field.as_str());
        if let Some(n) = &b.appraiser_name {
            v.push(n.as_str());
        }
        v.extend([
            b.appraiser_address.as_str(),
            b.appraiser_tin.as_str(),
            b.donee_name.as_str(),
            b.donee_ein.as_str(),
            b.donee_address.as_str(),
        ]);
        for r in &b.rows {
            v.extend([r.desc.as_str(), r.date_acq.as_str(), r.how.as_str()]);
            v.extend(r.fmv.fields());
            v.extend(r.cost.fields());
            v.extend(r.deduction.fields());
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
    /// The Part I amount-column subform token used to re-derive the geometry bands — **per-year map
    /// config** (`Table_PartI` for 2024/2025, **`TablePartI`** (no underscore) for the 2017 form).
    #[serde(default = "default_sched_d_token")]
    pub table_token: String,
    /// QOF question "Yes" choice. `None` on years whose Schedule D has no QOF question (2017 —
    /// Qualified Opportunity Funds began in 2019).
    #[serde(default)]
    pub qof_yes: Option<CheckChoice>,
    /// QOF question "No" choice (answered No when present). `None` on 2017 (no QOF question).
    #[serde(default)]
    pub qof_no: Option<CheckChoice>,
}

/// The default Schedule D Part I grid token (2024/2025); the 2017 map overrides it to `TablePartI`.
fn default_sched_d_token() -> String {
    "Table_PartI".to_string()
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

    /// The TY2017 map (grid token `TablePartI`; NO QOF question).
    pub fn ty2017() -> Self {
        Self::parse(SCHEDULE_D_MAP_2017).expect("bundled schedule_d 2017 map parses")
    }

    /// The map for a supported tax year.
    pub fn for_year(year: i32) -> Result<Self, FormsError> {
        match year {
            2017 => Ok(Self::ty2017()),
            2024 => Ok(Self::ty2024()),
            2025 => Ok(Self::ty2025()),
            _ => Err(FormsError::UnsupportedYear(year)),
        }
    }
}

/// The Form 8959 (Additional Medicare Tax) field map for one tax year.
///
/// Only the lines we FILL are mapped. Lines 2/3 (Form 4137 / Form 8919) and all of Part III plus
/// line 23 (RRTA) are unmodeled and are deliberately absent — they stay blank on the filed form,
/// which is why line 4 = line 1, line 18 = 7 + 13, and line 24 = line 22.
#[derive(Debug, Clone, Deserialize)]
pub struct Form8959Map {
    /// `"f8959"`.
    pub form: String,
    /// Tax year.
    pub year: i32,
    /// L1 — Σ W-2 box 5 Medicare wages, MID column.
    pub line1: MoneyCell,
    /// L4 — add lines 1–3 (2/3 blank ⇒ = line 1), MID column.
    pub line4: MoneyCell,
    /// L5 — filing-status threshold, MID column.
    pub line5: MoneyCell,
    /// L6 — line 4 − line 5, floored at 0, AMOUNT column.
    pub line6: MoneyCell,
    /// L7 — 0.9% × line 6, AMOUNT column.
    pub line7: MoneyCell,
    /// L8 — Schedule SE Part I line 6 (net SE earnings), MID column.
    pub line8: MoneyCell,
    /// L9 — filing-status threshold (again), MID column.
    pub line9: MoneyCell,
    /// L10 — the amount from line 4, MID column.
    pub line10: MoneyCell,
    /// L11 — line 9 − line 10, floored at 0, MID column.
    pub line11: MoneyCell,
    /// L12 — line 8 − line 11, floored at 0, AMOUNT column.
    pub line12: MoneyCell,
    /// L13 — 0.9% × line 12, AMOUNT column.
    pub line13: MoneyCell,
    /// L18 — add 7, 13, 17 → Schedule 2 line 11, AMOUNT column.
    pub line18: MoneyCell,
    /// L19 — Σ W-2 box 6 Medicare tax withheld, MID column.
    pub line19: MoneyCell,
    /// L20 — the amount from line 1, MID column.
    pub line20: MoneyCell,
    /// L21 — 1.45% × line 20, MID column.
    pub line21: MoneyCell,
    /// L22 — line 19 − line 21, floored at 0, AMOUNT column.
    pub line22: MoneyCell,
    /// L24 — add 22 and 23 → 1040 line 25c, AMOUNT column.
    pub line24: MoneyCell,
}

impl Form8959Map {
    /// Parse the committed TOML.
    pub fn parse(toml_src: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_src)
    }

    /// The TY2024 map.
    pub fn ty2024() -> Self {
        Self::parse(F8959_MAP_2024).expect("bundled f8959 2024 map parses")
    }

    /// The map for a supported tax year. Full-return v1 is **TY2024-only**: Form 8959 is reachable
    /// only from the absolute return, which itself has tables for 2024 alone.
    pub fn for_year(year: i32) -> Result<Self, FormsError> {
        match year {
            2024 => Ok(Self::ty2024()),
            _ => Err(FormsError::UnsupportedYear(year)),
        }
    }

    /// The 17 filled cells, in **printed reading order** (strictly descending y on page 1) — the
    /// order `fill_form_8959` walks and the ordinal the geometric verifier checks the descent of.
    pub fn lines(&self) -> [&MoneyCell; 17] {
        [
            &self.line1,
            &self.line4,
            &self.line5,
            &self.line6,
            &self.line7,
            &self.line8,
            &self.line9,
            &self.line10,
            &self.line11,
            &self.line12,
            &self.line13,
            &self.line18,
            &self.line19,
            &self.line20,
            &self.line21,
            &self.line22,
            &self.line24,
        ]
    }
}

/// The Form 8960 (Net Investment Income Tax) field map for one tax year.
///
/// Only the lines v1 FILLS are mapped. Annuities (3), Schedule E (4a–4c), CFC/PFIC (6), investment
/// expenses (9a–9c, 10) and the whole estates-and-trusts branch (18a–21) are unmodeled and stay
/// BLANK. The derived totals 9d and 11 ARE filled at zero — the form's arithmetic adds them.
#[derive(Debug, Clone, Deserialize)]
pub struct Form8960Map {
    /// `"f8960"`.
    pub form: String,
    /// Tax year.
    pub year: i32,
    /// L1 — taxable interest, AMOUNT column.
    pub line1: MoneyCell,
    /// L2 — ordinary dividends, AMOUNT column.
    pub line2: MoneyCell,
    /// L5a — net gain/loss from disposition of property, MID column.
    pub line5a: MoneyCell,
    /// L5d — combine 5a–5c, AMOUNT column.
    pub line5d: MoneyCell,
    /// L7 — other modifications, AMOUNT column.
    pub line7: MoneyCell,
    /// L8 — total investment income, AMOUNT column.
    pub line8: MoneyCell,
    /// L9d — add 9a/9b/9c (zero in v1), AMOUNT column.
    pub line9d: MoneyCell,
    /// L11 — total deductions and modifications (zero in v1), AMOUNT column.
    pub line11: MoneyCell,
    /// L12 — net investment income, AMOUNT column.
    pub line12: MoneyCell,
    /// L13 — modified AGI, MID column.
    pub line13: MoneyCell,
    /// L14 — the §1411(b) threshold (fillable, NOT pre-printed), MID column.
    pub line14: MoneyCell,
    /// L15 — 13 − 14, floored, MID column.
    pub line15: MoneyCell,
    /// L16 — smaller of 12 or 15, AMOUNT column.
    pub line16: MoneyCell,
    /// L17 — 3.8% × 16 → Schedule 2 line 12, AMOUNT column.
    pub line17: MoneyCell,
}

impl Form8960Map {
    /// Parse the committed TOML.
    pub fn parse(toml_src: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_src)
    }
    /// The TY2024 map.
    pub fn ty2024() -> Self {
        Self::parse(F8960_MAP_2024).expect("bundled f8960 2024 map parses")
    }
    /// The map for a supported tax year. Full-return v1 is TY2024-only.
    pub fn for_year(year: i32) -> Result<Self, FormsError> {
        match year {
            2024 => Ok(Self::ty2024()),
            _ => Err(FormsError::UnsupportedYear(year)),
        }
    }
    /// The 14 filled cells in printed reading order (strictly descending y on page 1).
    pub fn lines(&self) -> [&MoneyCell; 14] {
        [
            &self.line1,
            &self.line2,
            &self.line5a,
            &self.line5d,
            &self.line7,
            &self.line8,
            &self.line9d,
            &self.line11,
            &self.line12,
            &self.line13,
            &self.line14,
            &self.line15,
            &self.line16,
            &self.line17,
        ]
    }
}

/// The Form 8995 (QBI deduction, simplified) field map for one tax year.
///
/// The Part I trade/business table (rows 1i–1v) and line 3 are deliberately unmapped: v1's only QBI
/// is §199A REIT dividends, so there is no business to list. Lines 2/4/5 ARE filled, at zero.
///
/// **Lines 7, 16 and 17 are PARENTHESIZED boxes — the form prints the minus sign, so the value must
/// be a POSITIVE MAGNITUDE.** `qbi::Form8995Lines` guarantees that.
#[derive(Debug, Clone, Deserialize)]
pub struct Form8995Map {
    /// `"f8995"`.
    pub form: String,
    /// Tax year.
    pub year: i32,
    /// L2 — total QBI from the (blank) table, MID column.
    pub line2: MoneyCell,
    /// L4 — combine 2 and 3, MID column.
    pub line4: MoneyCell,
    /// L5 — QBI component (20% × 4), AMOUNT column.
    pub line5: MoneyCell,
    /// L6 — qualified REIT dividends + PTP income, MID column.
    pub line6: MoneyCell,
    /// L7 — prior-year REIT/PTP loss carryforward, MID column. ★ positive magnitude (paren box).
    pub line7: MoneyCell,
    /// L8 — combine 6 and 7, MID column.
    pub line8: MoneyCell,
    /// L9 — REIT/PTP component (20% × 8), AMOUNT column.
    pub line9: MoneyCell,
    /// L10 — add 5 and 9, AMOUNT column.
    pub line10: MoneyCell,
    /// L11 — taxable income before the QBI deduction, MID column.
    pub line11: MoneyCell,
    /// L12 — net capital gain + qualified dividends, MID column.
    pub line12: MoneyCell,
    /// L13 — 11 − 12, floored, MID column.
    pub line13: MoneyCell,
    /// L14 — income limitation (20% × 13), AMOUNT column.
    pub line14: MoneyCell,
    /// L15 — the deduction: smaller of 10 or 14 → 1040 L13, AMOUNT column.
    pub line15: MoneyCell,
    /// L16 — total QB (loss) carryforward, AMOUNT column. ★ positive magnitude (paren box).
    pub line16: MoneyCell,
    /// L17 — total REIT/PTP (loss) carryforward, AMOUNT column. ★ positive magnitude (paren box).
    pub line17: MoneyCell,
}

impl Form8995Map {
    /// Parse the committed TOML.
    pub fn parse(toml_src: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_src)
    }
    /// The TY2024 map.
    pub fn ty2024() -> Self {
        Self::parse(F8995_MAP_2024).expect("bundled f8995 2024 map parses")
    }
    /// The map for a supported tax year. Full-return v1 is TY2024-only.
    pub fn for_year(year: i32) -> Result<Self, FormsError> {
        match year {
            2024 => Ok(Self::ty2024()),
            _ => Err(FormsError::UnsupportedYear(year)),
        }
    }
    /// The 15 filled cells in printed reading order (strictly descending y on page 1).
    pub fn lines(&self) -> [&MoneyCell; 15] {
        [
            &self.line2,
            &self.line4,
            &self.line5,
            &self.line6,
            &self.line7,
            &self.line8,
            &self.line9,
            &self.line10,
            &self.line11,
            &self.line12,
            &self.line13,
            &self.line14,
            &self.line15,
            &self.line16,
            &self.line17,
        ]
    }
}

/// The Schedule 2 (Additional Taxes) field map for one tax year.
///
/// Part I is entirely absent: line 1a (excess APTC) has no input and would refuse if it did, and
/// line 2 (AMT) is $0 by construction (the return is refused if the Form 6251 screen trips). Only
/// the three Part II taxes v1 computes are mapped. **Line 21 is on PAGE 2.**
#[derive(Debug, Clone, Deserialize)]
pub struct Schedule2Map {
    /// `"f1040s2"`.
    pub form: String,
    /// Tax year.
    pub year: i32,
    /// L4 — self-employment tax (SS + regular Medicare only), AMOUNT column, page 1.
    pub line4: MoneyCell,
    /// L11 — Additional Medicare Tax (Form 8959's printed L18), AMOUNT column, page 1.
    pub line11: MoneyCell,
    /// L12 — net investment income tax (Form 8960's printed L17), AMOUNT column, page 1.
    pub line12: MoneyCell,
    /// L21 — total other taxes → 1040 L23, AMOUNT column, **page 2**.
    pub line21: MoneyCell,
}

impl Schedule2Map {
    /// Parse the committed TOML.
    pub fn parse(toml_src: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_src)
    }
    /// The TY2024 map.
    pub fn ty2024() -> Self {
        Self::parse(SCHEDULE_2_MAP_2024).expect("bundled schedule 2 2024 map parses")
    }
    /// The map for a supported tax year. Full-return v1 is TY2024-only.
    pub fn for_year(year: i32) -> Result<Self, FormsError> {
        match year {
            2024 => Ok(Self::ty2024()),
            _ => Err(FormsError::UnsupportedYear(year)),
        }
    }
    /// The 4 filled cells in printed reading order. **Descent is grouped by PAGE** — line 21 sits on
    /// page 2, whose y-coordinates are not comparable with page 1's.
    pub fn lines(&self) -> [&MoneyCell; 4] {
        [&self.line4, &self.line11, &self.line12, &self.line21]
    }
}

/// The Schedule 3 (Additional Credits and Payments) field map for one tax year.
///
/// Only the foreign tax credit (L1) and the §6413(c) excess-Social-Security credit (L11) are mapped.
/// Every other Part I credit is a §3.4 conservative omission and stays BLANK.
#[derive(Debug, Clone, Deserialize)]
pub struct Schedule3Map {
    /// `"f1040s3"`.
    pub form: String,
    /// Tax year.
    pub year: i32,
    /// L1 — foreign tax credit, AMOUNT column.
    pub line1: MoneyCell,
    /// L8 — total nonrefundable credits → 1040 L20, AMOUNT column.
    pub line8: MoneyCell,
    /// L11 — excess Social Security / tier-1 RRTA withheld, AMOUNT column.
    pub line11: MoneyCell,
    /// L15 — total other payments → 1040 L31, AMOUNT column.
    pub line15: MoneyCell,
}

impl Schedule3Map {
    /// Parse the committed TOML.
    pub fn parse(toml_src: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_src)
    }
    /// The TY2024 map.
    pub fn ty2024() -> Self {
        Self::parse(SCHEDULE_3_MAP_2024).expect("bundled schedule 3 2024 map parses")
    }
    /// The map for a supported tax year. Full-return v1 is TY2024-only.
    pub fn for_year(year: i32) -> Result<Self, FormsError> {
        match year {
            2024 => Ok(Self::ty2024()),
            _ => Err(FormsError::UnsupportedYear(year)),
        }
    }
    /// The 4 filled cells in printed reading order (strictly descending y on page 1).
    pub fn lines(&self) -> [&MoneyCell; 4] {
        [&self.line1, &self.line8, &self.line11, &self.line15]
    }
}

/// The Schedule A (Itemized Deductions) field map for one tax year.
///
/// **Three x-clusters** — Schedule A is the only form here that needs a third. Line 2 (the AGI the
/// 7.5% medical floor is taken on) sits INLINE with the printed sentence at x ≈ [331,403], not in the
/// MID column, and it is the same WIDTH as MID, so nothing but its x-position distinguishes it.
///
/// Unmapped on purpose: line 6 (other taxes), 8b/8c (mortgage not on a 1098; points), 9 (investment
/// interest), 15 (casualty), 16 (other). **Line 8d is a ReadOnly "Reserved for future use" widget** —
/// live, and it consumes a suffix number. Never write it.
#[derive(Debug, Clone, Deserialize)]
pub struct ScheduleAMap {
    /// `"f1040sa"`.
    pub form: String,
    /// Tax year.
    pub year: i32,
    /// L1 — medical and dental expenses, MID column.
    pub line1: MoneyCell,
    /// L2 — AGI. ★ **AGI-INLINE column**, not MID.
    pub line2: MoneyCell,
    /// L3 — the §213(a) 7.5% floor, MID column.
    pub line3: MoneyCell,
    /// L4 — medical allowed, AMOUNT column.
    pub line4: MoneyCell,
    /// L5a — state/local income or sales taxes, MID column.
    pub line5a: MoneyCell,
    /// L5b — real-estate taxes, MID column.
    pub line5b: MoneyCell,
    /// L5c — personal-property taxes, MID column.
    pub line5c: MoneyCell,
    /// L5d — add 5a-5c, MID column.
    pub line5d: MoneyCell,
    /// L5e — the §164(b) SALT cap, MID column.
    pub line5e: MoneyCell,
    /// L7 — add 5e and 6, AMOUNT column.
    pub line7: MoneyCell,
    /// L8a — mortgage interest on Form 1098, MID column.
    pub line8a: MoneyCell,
    /// L8e — add 8a-8c, MID column.
    pub line8e: MoneyCell,
    /// L10 — add 8e and 9, AMOUNT column.
    pub line10: MoneyCell,
    /// L11 — gifts by cash or check, MID column.
    pub line11: MoneyCell,
    /// L12 — gifts other than cash (incl. crypto), MID column.
    pub line12: MoneyCell,
    /// L13 — prior-year carryover, MID column.
    pub line13: MoneyCell,
    /// L14 — add 11-13, AMOUNT column.
    pub line14: MoneyCell,
    /// L17 — total itemized deductions → 1040 L12, AMOUNT column.
    pub line17: MoneyCell,
}

impl ScheduleAMap {
    /// Parse the committed TOML.
    pub fn parse(toml_src: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_src)
    }
    /// The TY2024 map.
    pub fn ty2024() -> Self {
        Self::parse(SCHEDULE_A_MAP_2024).expect("bundled schedule A 2024 map parses")
    }
    /// The map for a supported tax year. Full-return v1 is TY2024-only.
    pub fn for_year(year: i32) -> Result<Self, FormsError> {
        match year {
            2024 => Ok(Self::ty2024()),
            _ => Err(FormsError::UnsupportedYear(year)),
        }
    }
    /// The 18 filled cells in printed reading order (strictly descending y on page 1).
    pub fn lines(&self) -> [&MoneyCell; 18] {
        [
            &self.line1,
            &self.line2,
            &self.line3,
            &self.line4,
            &self.line5a,
            &self.line5b,
            &self.line5c,
            &self.line5d,
            &self.line5e,
            &self.line7,
            &self.line8a,
            &self.line8e,
            &self.line10,
            &self.line11,
            &self.line12,
            &self.line13,
            &self.line14,
            &self.line17,
        ]
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
    pub line2: MoneyCell,
    /// Line 3 — combine 1a/1b/2 (= line 2), amount column.
    pub line3: MoneyCell,
    /// Line 4a — net SE earnings (base = net_se × 92.35%), amount column.
    pub line4a: MoneyCell,
    /// Line 4c — combine 4a/4b (= line 4a), amount column. The $400 STOP threshold.
    pub line4c: MoneyCell,
    /// Line 6 — add 4c/5b (= line 4c), amount column.
    pub line6: MoneyCell,
    /// Line 8a — Form W-2 Social Security wages, **MID column**.
    pub line8a: MoneyCell,
    /// Line 8d — add 8a/8b/8c (= line 8a), amount column.
    pub line8d: MoneyCell,
    /// Line 9 — line 7 (`ss_wage_base` constant) − line 8d, amount column.
    pub line9: MoneyCell,
    /// Line 10 — Social Security portion (`ss`), amount column.
    pub line10: MoneyCell,
    /// Line 11 — regular Medicare portion (`medicare`), amount column.
    pub line11: MoneyCell,
    /// Line 12 — SE tax = line 10 + line 11 (**SS + regular Medicare ONLY**), amount column.
    pub line12: MoneyCell,
    /// Line 13 — one-half SE-tax deduction (= line 12 × 50% = `deductible_half`), **MID column**.
    pub line13: MoneyCell,
    /// Fields the BLANK form already carries a factory `/V` for (the 2017 §B long form pre-prints
    /// line 7 = `127,200`/`00` and line 14 = `5,200`/`00`) — excluded from the `no_unmapped_filled`
    /// guard so those constants don't read as stray writes. Empty on 2024/2025.
    #[serde(default)]
    pub prefilled_exempt: Vec<String>,
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

    /// The TY2017 map (OLD §B long form: dollars+cents pairs; pre-filled line 7/14 exempt).
    pub fn ty2017() -> Self {
        Self::parse(SCHEDULE_SE_MAP_2017).expect("bundled schedule_se 2017 map parses")
    }

    /// The map for a supported tax year.
    pub fn for_year(year: i32) -> Result<Self, FormsError> {
        match year {
            2017 => Ok(Self::ty2017()),
            2024 => Ok(Self::ty2024()),
            2025 => Ok(Self::ty2025()),
            _ => Err(FormsError::UnsupportedYear(year)),
        }
    }

    /// The 12 filled line cells, in chain order.
    pub fn lines(&self) -> [&MoneyCell; 12] {
        [
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

    /// Every field name the map targets (for the `map_YYYY_matches_bundled_pdf_fieldset` guard) —
    /// both members of each dollars+cents pair on the 2017 form.
    pub fn field_names(&self) -> Vec<&str> {
        self.lines().iter().flat_map(|c| c.fields()).collect()
    }
}
