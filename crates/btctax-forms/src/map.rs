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

/// The two identity cells every IRS form carries at its top: the name line and the SSN.
///
/// **Required** on the nine full-return schedule maps (a map without it fails at DESERIALIZATION —
/// fail-closed at load, and every map is loaded by a test), and `Option` on the two maps shared with
/// the crypto slice (`ScheduleDMap`, `Form1040Map`), whose 2017/2025 editions have no verified identity
/// FQNs and no `ReturnInputs` to source an identity from. The full-return fillers refuse on `None`.
///
/// The SSN's RENDERING is not fixed here: it is chosen per-cell from the PDF's own `/MaxLen` (11 ⇒
/// hyphenated, 9 ⇒ bare digits — the schedules and the 1040 genuinely differ). See
/// [`crate::cells::push_identity`].
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityCells {
    /// "Name(s) shown on return" — or, on Schedule C, "Name of proprietor".
    pub name: String,
    /// The SSN cell.
    pub ssn: String,
}

/// One `[census]` entry: an AcroForm field this build does **not** fill, and the recorded reason.
///
/// ★ Why this is MODELLED rather than ignored. Every map struct here carries
/// `#[serde(deny_unknown_fields)]`, because without it a committed map's **renamed** line is dropped
/// in silence. Measured on `forms/2025/f6251.map.toml`, which carries the TY2025 line-1 split: its
/// `line1a` and `line1b` were discarded by `Form6251Map` without a word, and only the *absence* of
/// `line1` was loud — so a vanished line failed closed and a RENAMED line half-vanished.
/// A silently-dropped map key is a cell nothing fills and nothing reports, which is the wrong-number
/// path this crate exists to close. 27 of the 37 committed maps carry a `[census]` table, so closing
/// the key set means giving the census a field: the alternative — a blanket "unknown keys are fine" —
/// is the hole itself.
///
/// The census's own **content** gate is `tests/field_census.rs`, which text-scans the file (so it sees
/// every FQN, mapped or censused, whatever the struct models). This type closes the KEY set; that test
/// judges the decisions. Measured at the time of writing: 793 entries across 27 files, every one of
/// them exactly `{ line, rule, reason }`, all strings.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CensusDecision {
    /// The form's own label for the field ("1a", "I.3(a)", "Box A") — the printed line, not the FQN.
    pub line: String,
    /// The decision vocabulary (`unmodeled` / `artifact` / `gap` / …). Defined per-map in the
    /// `[census]` preamble and adjudicated by `tests/field_census.rs`, not by this type.
    pub rule: String,
    /// Why the field is left blank, in the words of whoever decided it.
    pub reason: String,
}

/// The 4 monetary "amount" columns of a Form 8949 / Schedule D totals row: (d) proceeds, (e) cost,
/// (g) adjustment, (h) gain. Column (f) — the code column — has no total (a spacer), so it is absent.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
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

/// How a form's PDF relates to the tax year it is bundled for (design r2 §4 `versioning`).
///
/// Most IRS forms are **annual**: `f6251--2025.pdf` is the TY2025 document. A few are **periodic** —
/// revised on their own calendar and stamped with a revision date rather than a year (`Form 8275
/// (Rev. 10-2024)`, `Form 8283 (Rev. 12-2025)`) — and a tax year may legitimately ship a PRIOR
/// revision. Recording which is which is what lets a later year alias a periodic template **by
/// hash** — `bundled::periodic_template` pairs the bytes with the year whose row says `periodic` —
/// and never by a year list.
///
/// TOML: `versioning = "annual"` or `versioning = { periodic = "Rev. 10-2024" }`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum Versioning {
    /// `versioning = "annual"` — the PDF is this tax year's own document.
    Annual(AnnualTag),
    /// `versioning = { periodic = "Rev. MM-YYYY" }` — the PDF is a revision-dated document.
    Periodic {
        /// The revision as printed on the form, e.g. `"Rev. 10-2024"`.
        periodic: String,
    },
}

/// The only string `Versioning::Annual` accepts. A separate enum so that a typo (`"anual"`) is a
/// parse refusal rather than a silently-accepted free string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AnnualTag {
    /// `"annual"`.
    Annual,
}

/// ★ **The ROW of the year-package table — design r2 §4.** Every `forms/<year>/<stem>.map.toml`
/// carries these keys at its top level, and **the row SET is the glob**: a map file IS a row, there
/// is no central list to forget one from. This struct is the schema-agnostic projection of those keys
/// (it does NOT `deny_unknown_fields`, because the same document carries the line bindings); the
/// per-form `*Map` structs carry the identical fields with `deny_unknown_fields` ON, so a map that
/// omits a required key is a **parse refusal** on the path that fills the form.
///
/// Why two readers and not one: `serde(flatten)` does not compose with `deny_unknown_fields`, and the
/// build script (§5) must read rows without knowing which struct parses the bindings. The test
/// `every_committed_map_has_a_row_that_parses` holds the two views to the same keys.
///
/// Derived by convention, **never stored** here: the form extract
/// `design/forms/extract/<irs_stem>--<year>.txt`, the instructions extract
/// `<instructions>--<year>.txt`, the geometry `design/forms/geometry/<irs_stem>--<year>.json`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct MapRow {
    /// Crate stem — the `.map.toml` / `.pdf` basename (`"f6251"`, `"schedule_d"`).
    pub form: String,
    /// Tax year — the directory the map lives in.
    pub year: i32,
    /// IRS basename. Differs from `form` only for `schedule_d` → `f1040sd` and `schedule_se` →
    /// `f1040sse`; `cite_check::STEM_ALIASES` retires into this field.
    pub irs_stem: String,
    /// Annual or periodic — see [`Versioning`].
    pub versioning: Versioning,
    /// sha256 of the bundled PDF beside this map. Joined BY CONTENT to `design/forms/MANIFEST.json`,
    /// whose entry must be an authority (`is_authority()`), never a draft.
    pub template_sha256: String,
    /// OPTIONAL. The ONLY excuse the manifest join accepts: `"not-yet-archived: <reason>"`. Six rows
    /// today (all five TY2017 templates + `forms/2024/f8283.pdf`); the count may only shrink.
    #[serde(default)]
    pub authority: Option<String>,
    /// OPTIONAL. Only while a second extract root exists (`f1040s1a/2025` reads
    /// `crates/btctax-core/src/tax/fixtures/`); see design r2 §9.
    #[serde(default)]
    pub extract_override: Option<String>,
    /// Instructions stem: `fNNNN` → `iNNNN`, with the IRS's own aliases (`f1040sa` → `i1040sca`;
    /// Form 1040 and Schedules 1/1-A/2/3 → `i1040gi`). Every bundled form has one (Form 8275's is
    /// `i8275`), and for every archived year it must be a manifest entry (`xtask` map_row_tests).
    pub instructions: String,
    /// OPTIONAL. `[first, last]` page range inside an `i1040gi`-hosted booklet, recorded once by a
    /// human (runbook step 5).
    #[serde(default)]
    pub instr_pages: Option<[u32; 2]>,
    /// The LINE-SET REVISION this map is a transcription of, `"<stem>/<year>"` by default. A
    /// constants-only year may share a revision; a renumber gets a new one. Several revisions may
    /// resolve to one struct today (design r2 §4, §7); the `line_set → struct` match is many-to-one.
    pub line_set: String,
    /// OPTIONAL. The form's printed "Attachment Sequence No." (`"32"`, `"12A"`). ABSENT on the 1040
    /// itself, which carries no sequence number.
    #[serde(default)]
    pub attachment_sequence: Option<String>,
}

impl MapRow {
    /// Read the row keys off a map file's text. Unknown keys (the line bindings) are ignored here —
    /// the per-form struct is what refuses them.
    pub fn read(toml_src: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_src)
    }
}

/// **Form 6251** (Alternative Minimum Tax—Individuals), TY2024 — §G-6.
///
/// 41 of the form's 59 numbered money boxes. Lines 2c-2t are Part I add-backs core does not model and
/// are CENSUSED as `gap` (not `unmodeled`): they are add-backs, so silence understates tax, and the
/// filer is refused through the §G-22 out-of-scope declaration rather than filed with a laundered zero.
///
/// ★ See `forms/2024/f6251.map.toml` for how the assignment was corroborated — the page-1 field names
/// are NOT a uniform offset, and the three inset widgets landing exactly on the three parenthesised
/// lines 2b/2f/2s is what pins it.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Form6251Map {
    /// `"f6251"`.
    pub form: String,
    /// Tax year.
    pub year: i32,
    // ── design r2 §4 — the ROW. The same keys as [`MapRow`]; required ones REFUSE when missing. ──
    /// IRS basename (`"f1040sd"` for `schedule_d`). See [`MapRow::irs_stem`].
    pub irs_stem: String,
    /// Annual or periodic. See [`MapRow::versioning`].
    pub versioning: Versioning,
    /// sha256 of the bundled PDF. See [`MapRow::template_sha256`].
    pub template_sha256: String,
    /// OPTIONAL — the manifest-join excuse. See [`MapRow::authority`].
    #[serde(default)]
    pub authority: Option<String>,
    /// OPTIONAL — a second extract root. See [`MapRow::extract_override`].
    #[serde(default)]
    pub extract_override: Option<String>,
    /// Instructions stem. See [`MapRow::instructions`].
    pub instructions: String,
    /// OPTIONAL — page range in an `i1040gi` booklet. See [`MapRow::instr_pages`].
    #[serde(default)]
    pub instr_pages: Option<[u32; 2]>,
    /// The line-set revision. See [`MapRow::line_set`].
    pub line_set: String,
    /// OPTIONAL — absent on the 1040. See [`MapRow::attachment_sequence`].
    #[serde(default)]
    pub attachment_sequence: Option<String>,
    /// The §G-13 **field census** — every AcroForm field on this year's PDF that this build does NOT
    /// fill, mapped to the [`CensusDecision`] that leaves it blank. Declared, not tolerated: see
    /// [`CensusDecision`] for why a modelled key is what lets `deny_unknown_fields` be switched on.
    #[serde(default)]
    pub census: std::collections::BTreeMap<String, CensusDecision>,
    pub line1: MoneyCell,
    pub line2a: MoneyCell,
    pub line2b: MoneyCell,
    pub line3: MoneyCell,
    pub line4: MoneyCell,
    pub line5: MoneyCell,
    pub line6: MoneyCell,
    pub line7: MoneyCell,
    pub line8: MoneyCell,
    pub line9: MoneyCell,
    pub line10: MoneyCell,
    pub line11: MoneyCell,
    pub line12: MoneyCell,
    pub line13: MoneyCell,
    pub line14: MoneyCell,
    pub line15: MoneyCell,
    pub line16: MoneyCell,
    pub line17: MoneyCell,
    pub line18: MoneyCell,
    pub line19: MoneyCell,
    pub line20: MoneyCell,
    pub line21: MoneyCell,
    pub line22: MoneyCell,
    pub line23: MoneyCell,
    pub line24: MoneyCell,
    pub line25: MoneyCell,
    pub line26: MoneyCell,
    pub line27: MoneyCell,
    pub line28: MoneyCell,
    pub line29: MoneyCell,
    pub line30: MoneyCell,
    pub line31: MoneyCell,
    pub line32: MoneyCell,
    pub line33: MoneyCell,
    pub line34: MoneyCell,
    pub line35: MoneyCell,
    pub line36: MoneyCell,
    pub line37: MoneyCell,
    pub line38: MoneyCell,
    pub line39: MoneyCell,
    pub line40: MoneyCell,
    /// Name + SSN. REQUIRED — a schedule that does not name its taxpayer is not filable.
    pub identity: IdentityCells,
}

impl Form6251Map {
    /// The bundled TY2024 map.
    pub fn ty2024() -> Self {
        Self::for_year(2024).expect("the bundled TY2024 map is wired and parses")
    }

    /// ★★★ Select the map by YEAR, refusing a year this build has no map for.
    ///
    /// The audit of the TY2025 map batch found `packet.rs` hardcoding `ty2024()` while `year` was
    /// already in scope two lines away. That is not a missing feature, it is a **wrong-number path
    /// waiting to be reached**: TY2025 split Form 6251 line 1 into 1a/1b and shifted the page-1
    /// field names, so filling a TY2025 PDF through the TY2024 map does not fail — it writes 2a
    /// into 1b's box and walks everything below down one, landing **line 11, the AMT itself**, in
    /// line 10's box. The figure would be wrong and the paper would look right.
    ///
    /// A hardcode cannot express "no map for this year". This can, and it fails CLOSED: an
    /// unmapped year is a refusal, never a silent substitution of a neighbouring year's geometry.
    /// ★ TY2025's map file is committed and field-verified but deliberately NOT wired here — its
    /// 1a/1b split needs `Form6251Map` and the fill logic to change together, which is a build task
    /// and not a review fold. Until then 2025 refuses, which is the honest state.
    /// The map for a tax year — design r2 §10 step 3: the file comes from the glob
    /// (`bundled::map_text`), the revision from its ROW, and the ONE exhaustive
    /// `line_set → schema` match (`line_set::schema`) decides whether THIS struct parses it.
    pub fn for_year(year: i32) -> Result<Self, crate::FormsError> {
        let text = crate::bundled::map_text(crate::bundled::Stem::F6251, year)
            .ok_or(crate::FormsError::UnsupportedYear(year))?;
        let row = MapRow::read(text).map_err(|e| {
            crate::FormsError::Structure(format!(
                "F6251 TY{year}: the map's row does not parse: {e}"
            ))
        })?;
        let ls = crate::line_set::LineSet::parse(&row.line_set).ok_or_else(|| {
            crate::FormsError::Structure(format!(
                "F6251 TY{year}: line_set {:?} is not a revision this build knows",
                row.line_set
            ))
        })?;
        match crate::line_set::schema(ls) {
            crate::line_set::Schema::Form6251Map => Self::parse(text).map_err(|e| {
                crate::FormsError::Structure(format!(
                    "F6251 TY{year}: the bundled map does not parse: {e}"
                ))
            }),
            crate::line_set::Schema::Unwired => Err(crate::FormsError::UnwiredLineSet {
                stem: "F6251",
                year,
                line_set: ls.as_str(),
            }),
            other => Err(crate::FormsError::Structure(format!(
                "F6251 TY{year}: line_set {} parses into {other:?}, not Form6251Map",
                ls.as_str()
            ))),
        }
    }
    pub fn parse(s: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(s)
    }

    /// Every modelled money cell, in the form's own printed order.
    ///
    /// ★★★ EXHAUSTIVE destructure, no `..` — a cell added to this map is *pattern does not mention
    /// field* here. That matters because the sweeps that read the FILLED page back (whole-dollar,
    /// paren-magnitude) iterate this list: a cell missing from it is a cell no read-back ever checks,
    /// which is the quietest way for a line to stop being verified while every test stays green.
    #[must_use]
    pub fn money_cells(&self) -> Vec<&MoneyCell> {
        let Self {
            form: _,
            year: _,
            // the ROW (design r2 §4) — provenance, never a money cell
            irs_stem: _,
            versioning: _,
            template_sha256: _,
            authority: _,
            extract_override: _,
            instructions: _,
            instr_pages: _,
            line_set: _,
            attachment_sequence: _,
            census: _,   // provenance for the fields we do NOT fill; never a money cell
            identity: _, // not money
            line1: _,
            line2a: _,
            line2b: _,
            line3: _,
            line4: _,
            line5: _,
            line6: _,
            line7: _,
            line8: _,
            line9: _,
            line10: _,
            line11: _,
            line12: _,
            line13: _,
            line14: _,
            line15: _,
            line16: _,
            line17: _,
            line18: _,
            line19: _,
            line20: _,
            line21: _,
            line22: _,
            line23: _,
            line24: _,
            line25: _,
            line26: _,
            line27: _,
            line28: _,
            line29: _,
            line30: _,
            line31: _,
            line32: _,
            line33: _,
            line34: _,
            line35: _,
            line36: _,
            line37: _,
            line38: _,
            line39: _,
            line40: _,
        } = self;
        vec![
            &self.line1,
            &self.line2a,
            &self.line2b,
            &self.line3,
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
            &self.line18,
            &self.line19,
            &self.line20,
            &self.line21,
            &self.line22,
            &self.line23,
            &self.line24,
            &self.line25,
            &self.line26,
            &self.line27,
            &self.line28,
            &self.line29,
            &self.line30,
            &self.line31,
            &self.line32,
            &self.line33,
            &self.line34,
            &self.line35,
            &self.line36,
            &self.line37,
            &self.line38,
            &self.line39,
            &self.line40,
        ]
    }
}

/// Schedule D lines **1a** and **8a** — columns (d), (e) and (h) only. §G-28/B4.
///
/// ★★★ THERE IS NO `adj_g`, AND ITS ABSENCE IS THE POINT. These lines are available only for
/// transactions *"for which basis was reported to the IRS and **for which you have no adjustments**"*.
/// Needing an adjustment is precisely what disqualifies a transaction from the line, so a cell for one
/// could never legitimately be written — and a type that cannot express it is a stronger guarantee
/// than a cell someone remembered not to fill. (The widget exists on the PDF and stays censused; it is
/// `_RO`, owned by the form's own JavaScript.)
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AmountColsNoAdjustment {
    /// Column (d) — proceeds.
    pub proceeds_d: String,
    /// Column (e) — cost basis.
    pub cost_e: String,
    /// Column (h) — gain/loss.
    pub gain_h: String,
}

/// One Form 8949 part (Part I short-term on page 0, Part II long-term on page 1).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartMap {
    /// `"short"` (Part I) or `"long"` (Part II).
    pub term: String,
    /// 0-based page index of this part within the bundled 2-page PDF.
    pub page: usize,
    /// The "not reported to the IRS" box checkbox field for this part's revision: the digital-asset
    /// **Box I** (ST) / **Box L** (LT) on the 2025 map, and the securities **Box C** / **Box F** on
    /// the pre-2025 (2024/2017) maps. Which one this is depends on the year the map was loaded for.
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
#[serde(deny_unknown_fields)]
pub struct Form8949Map {
    /// `"f8949"`.
    pub form: String,
    /// Tax year (e.g. 2025).
    pub year: i32,
    // ── design r2 §4 — the ROW. The same keys as [`MapRow`]; required ones REFUSE when missing. ──
    /// IRS basename (`"f1040sd"` for `schedule_d`). See [`MapRow::irs_stem`].
    pub irs_stem: String,
    /// Annual or periodic. See [`MapRow::versioning`].
    pub versioning: Versioning,
    /// sha256 of the bundled PDF. See [`MapRow::template_sha256`].
    pub template_sha256: String,
    /// OPTIONAL — the manifest-join excuse. See [`MapRow::authority`].
    #[serde(default)]
    pub authority: Option<String>,
    /// OPTIONAL — a second extract root. See [`MapRow::extract_override`].
    #[serde(default)]
    pub extract_override: Option<String>,
    /// Instructions stem. See [`MapRow::instructions`].
    pub instructions: String,
    /// OPTIONAL — page range in an `i1040gi` booklet. See [`MapRow::instr_pages`].
    #[serde(default)]
    pub instr_pages: Option<[u32; 2]>,
    /// The line-set revision. See [`MapRow::line_set`].
    pub line_set: String,
    /// OPTIONAL — absent on the 1040. See [`MapRow::attachment_sequence`].
    #[serde(default)]
    pub attachment_sequence: Option<String>,
    /// The §G-13 **field census** — every AcroForm field on this year's PDF that this build does NOT
    /// fill, mapped to the [`CensusDecision`] that leaves it blank. Declared, not tolerated: see
    /// [`CensusDecision`] for why a modelled key is what lets `deny_unknown_fields` be switched on.
    #[serde(default)]
    pub census: std::collections::BTreeMap<String, CensusDecision>,
    /// "Name(s) shown on return" + SSN — on **both pages** (the 8949 is a two-page detail attachment, and
    /// each page carries the header). `Option`: the crypto slice never writes it, and the 2017/2025 maps
    /// have no verified FQNs. The FULL-return filler refuses on `None` — an unnamed 8949 is not filable
    /// (Fable P6 r1 I3).
    #[serde(default)]
    pub identity_page1: Option<IdentityCells>,
    #[serde(default)]
    pub identity_page2: Option<IdentityCells>,
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
        Self::for_year(2025).expect("the bundled TY2025 map is wired and parses")
    }

    /// The TY2024 map.
    pub fn ty2024() -> Self {
        Self::for_year(2024).expect("the bundled TY2024 map is wired and parses")
    }

    /// The TY2017 map (pre-1099-DA: Box C/F, `/3`; field-identical grid to 2024).
    pub fn ty2017() -> Self {
        Self::for_year(2017).expect("the bundled TY2017 map is wired and parses")
    }

    /// The map for a supported tax year.
    /// The map for a tax year — design r2 §10 step 3: the file comes from the glob
    /// (`bundled::map_text`), the revision from its ROW, and the ONE exhaustive
    /// `line_set → schema` match (`line_set::schema`) decides whether THIS struct parses it.
    pub fn for_year(year: i32) -> Result<Self, FormsError> {
        let text = crate::bundled::map_text(crate::bundled::Stem::F8949, year)
            .ok_or(FormsError::UnsupportedYear(year))?;
        let row = MapRow::read(text).map_err(|e| {
            FormsError::Structure(format!("F8949 TY{year}: the map's row does not parse: {e}"))
        })?;
        let ls = crate::line_set::LineSet::parse(&row.line_set).ok_or_else(|| {
            FormsError::Structure(format!(
                "F8949 TY{year}: line_set {:?} is not a revision this build knows",
                row.line_set
            ))
        })?;
        match crate::line_set::schema(ls) {
            crate::line_set::Schema::Form8949Map => Self::parse(text).map_err(|e| {
                FormsError::Structure(format!(
                    "F8949 TY{year}: the bundled map does not parse: {e}"
                ))
            }),
            crate::line_set::Schema::Unwired => Err(FormsError::UnwiredLineSet {
                stem: "F8949",
                year,
                line_set: ls.as_str(),
            }),
            other => Err(FormsError::Structure(format!(
                "F8949 TY{year}: line_set {} parses into {other:?}, not Form8949Map",
                ls.as_str()
            ))),
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
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
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
/// The Form 1040's identity block (P6.2) — dumped and correlated against the printed form, never
/// extrapolated. The SSN cells here declare `/MaxLen 9` (comb), so they take the nine BARE digits,
/// while every schedule's SSN cell is `/MaxLen 11` and takes the hyphenated form. `push_identity`
/// reads each cell's capacity rather than assuming either.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Form1040HeaderCells {
    pub taxpayer_first: String,
    pub taxpayer_last: String,
    pub taxpayer_ssn: String,
    pub spouse_first: String,
    pub spouse_last: String,
    pub spouse_ssn: String,
    pub address_street: String,
    pub address_apt: String,
    pub address_city: String,
    pub address_state: String,
    pub address_zip: String,
    /// "If you checked the MFS box, enter the name of your spouse" — written on MFS only.
    pub mfs_spouse_name: String,
    /// The signature block's occupation cells (page 2).
    pub occupation_taxpayer: String,
    pub occupation_spouse: String,
    /// The taxpayer's Identity Protection PIN cell (page 2, a 6-character comb). A paper return that
    /// omits an ISSUED IP PIN is rejected or delayed (ARCH-P6.3a Q7 item 5).
    pub ip_pin: String,
    /// The §6096 Presidential Election Campaign boxes.
    pub presidential_taxpayer: CheckChoice,
    pub presidential_spouse: CheckChoice,
    /// "Someone can claim: You / Your spouse as a dependent" — the §63(c)(5) floor's own checkbox.
    pub claimed_dependent_taxpayer: CheckChoice,
    pub claimed_dependent_spouse: CheckChoice,
    /// "Spouse itemizes on a separate return or you were a dual-status alien" — §63(c)(6).
    pub mfs_spouse_itemizes: CheckChoice,
    /// ★ The four §63(f) aged/blind boxes. The IRS validates a nonstandard standard deduction by
    /// COUNTING these, so L12 and this checkbox count must agree or the return fails its own
    /// arithmetic cross-check (`p6-aged-blind-checkboxes-missing`).
    pub taxpayer_aged: CheckChoice,
    pub taxpayer_blind: CheckChoice,
    pub spouse_aged: CheckChoice,
    pub spouse_blind: CheckChoice,
    /// "If more than four dependents, see instructions and check here" — **CHECKED, and the
    /// continuation statement is emitted with it.**
    ///
    /// ★ This comment used to say v1 "REFUSES instead … the continuation statement is a synthetic
    /// page generator we do not have". Both halves are false and have been since §G-28/B2: the
    /// generator is `btctax_core::tax::dependents_statement`, the box is written at
    /// `form1040_full.rs`'s `check(w, p, &cells.more_than_four_dependents, !overflow.is_empty())`,
    /// and `packet::fill_full_return` emits the statement from the SAME predicate — so "box checked,
    /// no attachment" and "attachment, no box" are not expressible. Held by
    /// `full_return_forms::the_checkbox_and_the_statement_are_the_same_decision`.
    ///
    /// A stale refusal claim is not a harmless comment: it is the shape that sends a future reader
    /// looking for a refusal path that does not exist, or — worse — invites one to be re-added over
    /// working behaviour.
    pub more_than_four_dependents: CheckChoice,
    /// The four dependents rows the form physically has.
    pub dependent_rows: Vec<DependentRowCells>,
}

/// One row of the 1040's dependents table. The name is a SINGLE cell spanning the printed
/// "(1) First name / Last name" columns — the form has one widget there, not two.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DependentRowCells {
    pub name: String,
    pub ssn: String,
    pub relationship: String,
    /// The Child-Tax-Credit box. NEVER checked: v1 omits CTC/ODC entirely (1040 L19 = 0, with the
    /// `CtcOdcOmitted` advisory), and a checked credit box beside a zero credit is a form
    /// contradicting itself. Mapped so the no-unmapped oracle knows the cell exists and is DELIBERATELY
    /// left blank.
    pub ctc: CheckChoice,
    /// The Credit-for-Other-Dependents box. Never checked, same reason.
    pub odc: CheckChoice,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Form1040Map {
    /// `"f1040"`.
    pub form: String,
    /// Tax year.
    pub year: i32,
    // ── design r2 §4 — the ROW. The same keys as [`MapRow`]; required ones REFUSE when missing. ──
    /// IRS basename (`"f1040sd"` for `schedule_d`). See [`MapRow::irs_stem`].
    pub irs_stem: String,
    /// Annual or periodic. See [`MapRow::versioning`].
    pub versioning: Versioning,
    /// sha256 of the bundled PDF. See [`MapRow::template_sha256`].
    pub template_sha256: String,
    /// OPTIONAL — the manifest-join excuse. See [`MapRow::authority`].
    #[serde(default)]
    pub authority: Option<String>,
    /// OPTIONAL — a second extract root. See [`MapRow::extract_override`].
    #[serde(default)]
    pub extract_override: Option<String>,
    /// Instructions stem. See [`MapRow::instructions`].
    pub instructions: String,
    /// OPTIONAL — page range in an `i1040gi` booklet. See [`MapRow::instr_pages`].
    #[serde(default)]
    pub instr_pages: Option<[u32; 2]>,
    /// The line-set revision. See [`MapRow::line_set`].
    pub line_set: String,
    /// OPTIONAL — absent on the 1040. See [`MapRow::attachment_sequence`].
    #[serde(default)]
    pub attachment_sequence: Option<String>,
    /// The §G-13 **field census** — every AcroForm field on this year's PDF that this build does NOT
    /// fill, mapped to the [`CensusDecision`] that leaves it blank. Declared, not tolerated: see
    /// [`CensusDecision`] for why a modelled key is what lets `deny_unknown_fields` be switched on.
    #[serde(default)]
    pub census: std::collections::BTreeMap<String, CensusDecision>,
    /// The full-return identity BLOCK (P6.2). The 1040's header is not two cells like a schedule's: it
    /// is names + SSNs + address + the §63(f) aged/blind checkboxes + the dependents table. `Option`
    /// because this map is SHARED with the crypto slice, whose 2017/2025 editions have no verified
    /// header FQNs; the FULL-return filler refuses on `None` rather than emit an unnamed 1040.
    #[serde(default)]
    pub header: Option<Form1040HeaderCells>,
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

    // ── Full-return extension (P6). Absent from the 2017/2025 maps, hence optional. ───────────
    /// L1a — Σ W-2 box 1. AMOUNT column. Full-return only.
    #[serde(default)]
    pub line1a: Option<MoneyCell>,
    /// L2a — tax-exempt interest. SUBLINE column. Full-return only (absent from the 2017/2025 maps).
    #[serde(default)]
    pub line2a: Option<MoneyCell>,
    /// L1z — wages. AMOUNT column.
    #[serde(default)]
    pub line1z: Option<MoneyCell>,
    /// L2b — taxable interest. AMOUNT column.
    #[serde(default)]
    pub line2b: Option<MoneyCell>,
    /// L3a — qualified dividends. **SUBLINE column** (x ≈ [252,324]), not MID or AMOUNT.
    #[serde(default)]
    pub line3a: Option<MoneyCell>,
    /// L3b — ordinary dividends. AMOUNT column.
    #[serde(default)]
    pub line3b: Option<MoneyCell>,
    /// L8 — Schedule 1's printed L10.
    #[serde(default)]
    pub line8: Option<MoneyCell>,
    /// L9 — total income.
    #[serde(default)]
    pub line9: Option<MoneyCell>,
    /// L10 — Schedule 1's printed L26.
    #[serde(default)]
    pub line10: Option<MoneyCell>,
    /// L11 — AGI.
    #[serde(default)]
    pub line11: Option<MoneyCell>,
    /// L12 — the deduction claimed. **★ `f1_57` on the 2024 form is L12; on the 2025 form the same
    /// field name is L1z** (SPEC §7.4). Per-(form, year) maps exist for exactly this.
    #[serde(default)]
    pub line12: Option<MoneyCell>,
    /// L13 — Form 8995's printed L15 (QBI).
    #[serde(default)]
    pub line13: Option<MoneyCell>,
    /// L14 — 12 + 13.
    #[serde(default)]
    pub line14: Option<MoneyCell>,
    /// L15 — taxable income.
    #[serde(default)]
    pub line15: Option<MoneyCell>,
    /// L16 — tax.
    #[serde(default)]
    pub line16: Option<MoneyCell>,
    /// L17 — Schedule 2's printed L3 (always 0 in v1).
    #[serde(default)]
    pub line17: Option<MoneyCell>,
    /// L18 — 16 + 17.
    #[serde(default)]
    pub line18: Option<MoneyCell>,
    /// L19 — CTC/ODC (always 0 — a §3.4 conservative omission).
    #[serde(default)]
    pub line19: Option<MoneyCell>,
    /// L20 — Schedule 3's printed L8.
    #[serde(default)]
    pub line20: Option<MoneyCell>,
    /// L21 — 19 + 20.
    #[serde(default)]
    pub line21: Option<MoneyCell>,
    /// L22 — 18 − 21.
    #[serde(default)]
    pub line22: Option<MoneyCell>,
    /// L23 — Schedule 2's printed L21.
    #[serde(default)]
    pub line23: Option<MoneyCell>,
    /// L24 — TOTAL TAX.
    #[serde(default)]
    pub line24: Option<MoneyCell>,
    /// L25a — W-2 withholding. MID column.
    #[serde(default)]
    pub line25a: Option<MoneyCell>,
    /// L25b — 1099 withholding. MID column.
    #[serde(default)]
    pub line25b: Option<MoneyCell>,
    /// L25c — other withholding (Form 8959's printed L24). MID column.
    #[serde(default)]
    pub line25c: Option<MoneyCell>,
    /// L25d — 25a + 25b + 25c.
    #[serde(default)]
    pub line25d: Option<MoneyCell>,
    /// L26 — estimated tax payments.
    #[serde(default)]
    pub line26: Option<MoneyCell>,
    /// L31 — Schedule 3's printed L15. MID column.
    #[serde(default)]
    pub line31: Option<MoneyCell>,
    /// L32 — total other payments.
    #[serde(default)]
    pub line32: Option<MoneyCell>,
    /// L33 — TOTAL PAYMENTS.
    #[serde(default)]
    pub line33: Option<MoneyCell>,
    /// L34 — overpayment.
    #[serde(default)]
    pub line34: Option<MoneyCell>,
    /// L35a — refunded to you.
    #[serde(default)]
    pub line35a: Option<MoneyCell>,
    /// L37 — amount you owe.
    #[serde(default)]
    pub line37: Option<MoneyCell>,
    /// The 5-way filing-status checkbox group.
    #[serde(default)]
    pub filing_status: Option<FilingStatusBoxes>,
}

/// The 1040's **5-way filing-status checkbox group**.
///
/// **★ The leaf field names COLLIDE.** Two distinct fields are both called `c1_3[0]` and two are both
/// called `c1_3[1]`, distinguished only by their parent subform:
///
/// | status | fully-qualified name | on-state |
/// |---|---|---|
/// | Single | `…FilingStatus_ReadOrder[0].c1_3[0]` | `1` |
/// | HoH | `…Page1[0].c1_3[0]` (no wrapper!) | `2` |
/// | MFJ | `…FilingStatus_ReadOrder[0].c1_3[1]` | `3` |
/// | MFS | `…FilingStatus_ReadOrder[0].c1_3[2]` | `4` |
/// | QSS | `…Page1[0].c1_3[1]` (no wrapper!) | `5` |
///
/// A map keyed on the leaf name would silently check the WRONG FILING STATUS — which changes the
/// standard deduction, every bracket, and every threshold on the return. The on-states are distinct
/// and independently corroborate the mapping, so the filler asserts both.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FilingStatusBoxes {
    /// Single — on-state `1`.
    pub single: CheckChoice,
    /// Head of household — on-state `2`.
    pub hoh: CheckChoice,
    /// Married filing jointly — on-state `3`.
    pub mfj: CheckChoice,
    /// Married filing separately — on-state `4`.
    pub mfs: CheckChoice,
    /// Qualifying surviving spouse — on-state `5`.
    pub qss: CheckChoice,
}

impl Form1040Map {
    /// Parse the committed TOML.
    pub fn parse(toml_src: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_src)
    }

    /// The TY2025 map.
    pub fn ty2025() -> Self {
        Self::for_year(2025).expect("the bundled TY2025 map is wired and parses")
    }

    /// The TY2024 map.
    pub fn ty2024() -> Self {
        Self::for_year(2024).expect("the bundled TY2024 map is wired and parses")
    }

    /// The TY2017 map (capital gain on line 13; NO Digital-Asset question).
    pub fn ty2017() -> Self {
        Self::for_year(2017).expect("the bundled TY2017 map is wired and parses")
    }

    /// The map for a supported tax year.
    /// The map for a tax year — design r2 §10 step 3: the file comes from the glob
    /// (`bundled::map_text`), the revision from its ROW, and the ONE exhaustive
    /// `line_set → schema` match (`line_set::schema`) decides whether THIS struct parses it.
    pub fn for_year(year: i32) -> Result<Self, FormsError> {
        let text = crate::bundled::map_text(crate::bundled::Stem::F1040, year)
            .ok_or(FormsError::UnsupportedYear(year))?;
        let row = MapRow::read(text).map_err(|e| {
            FormsError::Structure(format!("F1040 TY{year}: the map's row does not parse: {e}"))
        })?;
        let ls = crate::line_set::LineSet::parse(&row.line_set).ok_or_else(|| {
            FormsError::Structure(format!(
                "F1040 TY{year}: line_set {:?} is not a revision this build knows",
                row.line_set
            ))
        })?;
        match crate::line_set::schema(ls) {
            crate::line_set::Schema::Form1040Map => Self::parse(text).map_err(|e| {
                FormsError::Structure(format!(
                    "F1040 TY{year}: the bundled map does not parse: {e}"
                ))
            }),
            crate::line_set::Schema::Unwired => Err(FormsError::UnwiredLineSet {
                stem: "F1040",
                year,
                line_set: ls.as_str(),
            }),
            other => Err(FormsError::Structure(format!(
                "F1040 TY{year}: line_set {} parses into {other:?}, not Form1040Map",
                ls.as_str()
            ))),
        }
    }
}

/// One Form 8283 **Section A** row (Donated Property of $5,000 or Less): the 8 filled columns.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
pub struct Section8283A {
    /// The 4 rows A–D.
    pub rows: Vec<Section8283ARow>,
}

/// One Form 8283 **Section B Part I** row (Over $5,000): the filled columns.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
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
    /// ★★★ **P3 — "Amount claimed as a deduction": column (i) on the 2023/2025 revisions, column (h)
    /// on the Rev. 12-2014. `None` on a revision whose instructions do not ask it of this filer, and
    /// the cell is then CENSUSED rather than mapped.**
    ///
    /// i8283 (Rev. 12-2024 and 12-2025 alike), verbatim from the extracted text layer
    /// (`design/forms/extract/i8283--2024.txt:1185-1191`): *"Column (i). Complete column (i), amount
    /// claimed as a deduction, if you are a pass-through entity or a member of a pass-through
    /// entity."* An individual donating their own bitcoin is neither, and btctax models no
    /// pass-through entity at all — the same boundary the census records for the header
    /// entity-name/TIN cells and the family-PTE box. So the 2024 and 2025 maps carry NO cell here,
    /// and the filler cannot write one: a map entry means *"we fill this"*, and a
    /// mapped-but-never-written cell is a claim nothing checks.
    ///
    /// ★ The **TY2017 (Rev. 12-2014) map keeps its cell, deliberately.** That revision predates the
    /// pass-through-entity regime, has a different Section B column layout (no qualified-conservation
    /// column at all), and **its instructions are not in this repository** — `design/forms/extract/`
    /// holds i8283 for 2024 and 2025 only. Changing a shipped behavior on a revision whose authority
    /// we do not hold would be inventing the rule rather than reading it, so TY2017 is left exactly
    /// as it was until the Rev. 12-2014 instructions are archived and read.
    #[serde(default)]
    pub deduction: Option<MoneyCell>,
}

/// Form 8283 Section B (page 1/2, over-$5,000 property + page 2 identity) — up to 3 rows (2024/2025)
/// or 4 rows (2017 Rev. 12-2014, `Line5A`–`Line5D`).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
pub struct Form8283Map {
    /// `"f8283"`.
    pub form: String,
    /// Tax year.
    pub year: i32,
    // ── design r2 §4 — the ROW. The same keys as [`MapRow`]; required ones REFUSE when missing. ──
    /// IRS basename (`"f1040sd"` for `schedule_d`). See [`MapRow::irs_stem`].
    pub irs_stem: String,
    /// Annual or periodic. See [`MapRow::versioning`].
    pub versioning: Versioning,
    /// sha256 of the bundled PDF. See [`MapRow::template_sha256`].
    pub template_sha256: String,
    /// OPTIONAL — the manifest-join excuse. See [`MapRow::authority`].
    #[serde(default)]
    pub authority: Option<String>,
    /// OPTIONAL — a second extract root. See [`MapRow::extract_override`].
    #[serde(default)]
    pub extract_override: Option<String>,
    /// Instructions stem. See [`MapRow::instructions`].
    pub instructions: String,
    /// OPTIONAL — page range in an `i1040gi` booklet. See [`MapRow::instr_pages`].
    #[serde(default)]
    pub instr_pages: Option<[u32; 2]>,
    /// The line-set revision. See [`MapRow::line_set`].
    pub line_set: String,
    /// OPTIONAL — absent on the 1040. See [`MapRow::attachment_sequence`].
    #[serde(default)]
    pub attachment_sequence: Option<String>,
    /// The §G-13 **field census** — every AcroForm field on this year's PDF that this build does NOT
    /// fill, mapped to the [`CensusDecision`] that leaves it blank. Declared, not tolerated: see
    /// [`CensusDecision`] for why a modelled key is what lets `deny_unknown_fields` be switched on.
    #[serde(default)]
    pub census: std::collections::BTreeMap<String, CensusDecision>,
    /// The FILER's identity — "Name(s) shown on your income tax return" + identifying number. `Option`
    /// because the crypto slice never writes it (its 8283 rides beside a return btctax did not produce)
    /// and the 2017/2025 maps have no verified FQNs; the FULL-return filler refuses on `None`.
    #[serde(default)]
    pub identity: Option<IdentityCells>,
    /// ★ **PAGE 2's** own "Name(s) shown on your income tax return" + "Identifying number" header.
    ///
    /// The form repeats the identity block on page 2 so a detached Section B page can still be tied to
    /// its return. btctax HELD the name and TIN and wrote them to page 1, but the map declared no
    /// page-2 cells, so a filed page 2 went out with no identifying header — §G-13's clearest "we have
    /// the datum and nothing connects it to the field" gap.
    ///
    /// ★★ **The FQNs differ per revision and were DUMPED, not inferred**: TY2024 is `f2_01`/`f2_02`,
    /// TY2025 is `f2_1`/`f2_2`, and the Rev. 12-2014 (TY2017) form uses `p2-t1`/`p2-t2` with a
    /// **/MaxLen of 12**, not 11. `Option` because only the full-return revision writes an identity at
    /// all — the crypto-slice maps carry no `[identity]` block either.
    #[serde(default)]
    pub identity_page2: Option<IdentityCells>,
    /// ★ Section B lines **5a / 5b / 5c** — the restriction questions. `Option`: only the full-return
    /// revision carries them (the crypto slice writes no Section B declarations).
    #[serde(default)]
    pub line5a: Option<YesNoPair>,
    #[serde(default)]
    pub line5b: Option<YesNoPair>,
    #[serde(default)]
    pub line5c: Option<YesNoPair>,
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
        Self::for_year(2025).expect("the bundled TY2025 map is wired and parses")
    }

    /// The TY2024 map (Form 8283 Rev. 12-2023).
    pub fn ty2024() -> Self {
        Self::for_year(2024).expect("the bundled TY2024 map is wired and parses")
    }

    /// The TY2017 map (Form 8283 Rev. 12-2014 — "j Other", no DA box, 5/4 rows, ¢-pairs).
    pub fn ty2017() -> Self {
        Self::for_year(2017).expect("the bundled TY2017 map is wired and parses")
    }

    /// The map for a supported tax year.
    /// The map for a tax year — design r2 §10 step 3: the file comes from the glob
    /// (`bundled::map_text`), the revision from its ROW, and the ONE exhaustive
    /// `line_set → schema` match (`line_set::schema`) decides whether THIS struct parses it.
    pub fn for_year(year: i32) -> Result<Self, FormsError> {
        let text = crate::bundled::map_text(crate::bundled::Stem::F8283, year)
            .ok_or(FormsError::UnsupportedYear(year))?;
        let row = MapRow::read(text).map_err(|e| {
            FormsError::Structure(format!("F8283 TY{year}: the map's row does not parse: {e}"))
        })?;
        let ls = crate::line_set::LineSet::parse(&row.line_set).ok_or_else(|| {
            FormsError::Structure(format!(
                "F8283 TY{year}: line_set {:?} is not a revision this build knows",
                row.line_set
            ))
        })?;
        match crate::line_set::schema(ls) {
            crate::line_set::Schema::Form8283Map => Self::parse(text).map_err(|e| {
                FormsError::Structure(format!(
                    "F8283 TY{year}: the bundled map does not parse: {e}"
                ))
            }),
            crate::line_set::Schema::Unwired => Err(FormsError::UnwiredLineSet {
                stem: "F8283",
                year,
                line_set: ls.as_str(),
            }),
            other => Err(FormsError::Structure(format!(
                "F8283 TY{year}: line_set {} parses into {other:?}, not Form8283Map",
                ls.as_str()
            ))),
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
            // ★ Column (i) contributes only on a year whose map carries it. On 2024/2025 it is
            //   absent, and keeping it out of the AUTHORISED set is the load-bearing half:
            //   `verify::no_unmapped_filled` fails closed if anything ever writes it again.
            if let Some(d) = &r.deduction {
                v.extend(d.fields());
            }
        }
        v
    }
}

/// One Form 8275 Part I row (Rev. 10-2024): the columns btctax actually fills, keyed to a T13
/// `Part1Item`. **FREE-TEXT, no money-grid clustering** (arch/T15): every cell here is written via
/// `push_free`/`FlatPlacement::free`, not the column-x-clustered `push_cell` form8283/Schedule-SE use.
///
/// Column (a) "Rev. Rul., Rev. Proc., etc." and column (e) "Line No." (a `/MaxLen 3` cell — far too
/// narrow for our descriptive `Part1Item.line` string, e.g. "Part I — column (e)") are **deliberately
/// absent**: there is no citation to disclose, and Form 8949 has no discrete numbered "line" (it is a
/// per-transaction, lettered-COLUMN schedule) — nothing correct could be written to either.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Form8275Row {
    /// (b) "Item or Group of Items" — the position's form-location descriptor (`Part1Item.line`).
    pub item: String,
    /// (c) "Detailed Description of Items" — the Cohan-estimate explanation (`Part1Item.description`).
    pub desc: String,
    /// (d) "Form or Schedule" — the filed form the position appears on (`Part1Item.form`, e.g. "8949").
    pub form_schedule: String,
    /// (f) "Amount".
    pub amount: String,
}

/// The Form 8275 (Disclosure Statement, Rev. 10-2024) field map. **One revision, aliased to every
/// bundled year** that has no `forms/<year>/f8275.*` of its own — `versioning = { periodic = "Rev.
/// 10-2024" }` in the row; the alias is licensed BY HASH in [`Form8275Map::alias_is_licensed_by`] and
/// served by `bundled::periodic_template` (design r2 §4).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Form8275Map {
    /// `"f8275"`.
    pub form: String,
    /// Tax year this map instance is stamped for (re-stamped by `for_year`; the field SET is identical
    /// across every supported year — see the module doc).
    pub year: i32,
    // ── design r2 §4 — the ROW. The same keys as [`MapRow`]; required ones REFUSE when missing. ──
    /// IRS basename (`"f1040sd"` for `schedule_d`). See [`MapRow::irs_stem`].
    pub irs_stem: String,
    /// Annual or periodic. See [`MapRow::versioning`].
    pub versioning: Versioning,
    /// sha256 of the bundled PDF. See [`MapRow::template_sha256`].
    pub template_sha256: String,
    /// OPTIONAL — the manifest-join excuse. See [`MapRow::authority`].
    #[serde(default)]
    pub authority: Option<String>,
    /// OPTIONAL — a second extract root. See [`MapRow::extract_override`].
    #[serde(default)]
    pub extract_override: Option<String>,
    /// Instructions stem. See [`MapRow::instructions`].
    pub instructions: String,
    /// OPTIONAL — page range in an `i1040gi` booklet. See [`MapRow::instr_pages`].
    #[serde(default)]
    pub instr_pages: Option<[u32; 2]>,
    /// The line-set revision. See [`MapRow::line_set`].
    pub line_set: String,
    /// OPTIONAL — absent on the 1040. See [`MapRow::attachment_sequence`].
    #[serde(default)]
    pub attachment_sequence: Option<String>,
    /// The §G-13 **field census** — every AcroForm field on this year's PDF that this build does NOT
    /// fill, mapped to the [`CensusDecision`] that leaves it blank. Declared, not tolerated: see
    /// [`CensusDecision`] for why a modelled key is what lets `deny_unknown_fields` be switched on.
    #[serde(default)]
    pub census: std::collections::BTreeMap<String, CensusDecision>,
    /// The FILER's identity — "Name(s) shown on return" + "Identifying number shown on return". The map
    /// always DECLARES these cells (unlike Form 8283, whose 2017 revision structurally lacks an identity
    /// block), but Task 16's crypto-slice fill (`fill_form_8275_slice`) leaves them unwritten — mirroring
    /// Form 8283's own crypto-slice fill, which writes no identity either.
    pub identity: IdentityCells,
    /// Part I rows (6 on this revision) — the per-copy capacity `fill_form_8275` refuses beyond.
    pub rows: Vec<Form8275Row>,
    /// Part II "Detailed Explanation" line 1 — the ONLY Part II line the filer's narrative
    /// (`Printed8275::part_ii`) is written to. Overflow goes to `part_iv_continuation`, NOT to
    /// `part_ii_continuation`: the bundled PDF's static page content prints the numerals "1 ".."6 "
    /// beside `p1-t80`..`p1-t85`, and those numerals correspond to Part I's rows — so writing one
    /// combined narrative across them would attribute sentence fragments to items they do not explain.
    pub part_ii_narrative: String,
    /// Part II "Detailed Explanation" lines 2–6 — `p1-t81[0]`..`p1-t85[0]` on the bundled Rev. 10-2024
    /// PDF, in printed top-to-bottom order.
    ///
    /// **Mapped but deliberately NOT written.** Retained so the map describes the form completely (and
    /// so `verify_flat` authorizes them if a future per-item Part II numbering lands — see
    /// `design/f8275-part-ii-overflow/FOLLOWUPS.md`), but the fill writes only line 1 and then spills to
    /// Part IV, for the printed-numeral reason on `part_ii_narrative`.
    pub part_ii_continuation: Vec<String>,
    /// Page-2 **Part IV** "Explanations (continued from Parts I and/or II)" — `p2-t1[0]`..`p2-t27[0]`,
    /// in printed top-to-bottom order. The narrative overflows into these once Part II **line 1** is
    /// full. Per the IRS Rev. 10-2024 Specific Instructions ("Include the corresponding part and line
    /// number from page 1"), the first line used carries a `Part II, line 1 (continued):` prefix, whose
    /// width is budgeted before wrapping.
    pub part_iv_continuation: Vec<String>,
}

impl Form8275Map {
    /// Parse the committed TOML.
    pub fn parse(toml_src: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_src)
    }

    /// The bundled Rev. 10-2024 map, as committed (`year` field reads 2024).
    pub fn ty2024() -> Self {
        Self::for_year(2024).expect("the bundled TY2024 map is wired and parses")
    }

    /// The bundled Form 8275 asset for `year` — the ONE authority for "does this build ship a Form
    /// 8275 for that year", and the thing [`Self::for_year`]'s alias is licensed by.
    ///
    /// Exposed as an associated function so a test can ask the same question the alias asks, and so
    /// nothing outside `pdf.rs` has to keep a second list of Form 8275 years.
    pub fn bundled_pdf(year: i32) -> Result<&'static [u8], FormsError> {
        crate::pdf::f8275_pdf(year)
    }

    /// ★★★ The PRECONDITION the Form 8275 alias rests on, written as a CHECK instead of a year list.
    ///
    /// Form 8275 is REVISION-versioned, not tax-year-versioned, so this one transcribed map is reused
    /// for every year that bundles the same revision. That is legitimate **only while it is true**, and
    /// the thing that makes it true is the ASSET, not the calendar: this map's FQNs were transcribed
    /// from `forms/2024/f8275.pdf` (Rev. 10-2024), so it may be stamped for `year` exactly when that
    /// year's bundled Form 8275 is that same document, byte for byte.
    ///
    /// The old spelling was `2017 | 2024 | 2025 => alias`, which asserted nothing a reader could check
    /// and made `| 2026` a one-token edit — an edit that would hand a Rev. 12-2026 form a Rev. 10-2024
    /// field map, whose free-text cells would land in whatever boxes those names now denote. Here the
    /// same edit is not available: a year is aliased because its bundled bytes ARE this revision, and a
    /// year that ever bundles a different one is refused at the point of substitution.
    ///
    /// `bundled` is a parameter rather than a lookup so the refusal can be OBSERVED: no IRS revision is
    /// needed to plant the defect this exists to catch (see `sp4.rs`,
    /// `alias_refuses_a_year_whose_bundled_8275_is_a_different_document`).
    pub fn alias_is_licensed_by(year: i32, bundled: &[u8]) -> Result<(), FormsError> {
        let rev_10_2024 = crate::bundled::template(crate::bundled::Stem::F8275, 2024)
            .expect("the Rev. 10-2024 Form 8275 is bundled");
        if bundled == rev_10_2024 {
            return Ok(());
        }
        Err(FormsError::Structure(format!(
            "TY{year} bundles a Form 8275 that is NOT the Rev. 10-2024 document \
             `forms/2024/f8275.map.toml` was transcribed from ({} bytes vs {}), so that map may not \
             be aliased to TY{year}: transcribe `forms/{year}/f8275.map.toml` against the revision \
             this build actually ships for TY{year}",
            bundled.len(),
            rev_10_2024.len(),
        )))
    }

    /// The map for a supported tax year — the ONE bundled Rev. 10-2024 map re-stamped with `year`,
    /// which is what keeps a promoted 2025 (or 2017) disposal's Form 8275 export from being refused for
    /// want of a "2025 map" that would not structurally differ from this one.
    ///
    /// ★ It holds no year literal of its own. Periodic (design r2 §4 `versioning`): a BUNDLED year
    /// with no `forms/<year>/f8275.*` of its own is served by the newest bundled revision —
    /// `bundled::periodic_template` returns the bytes TOGETHER WITH the year whose row said
    /// `periodic`, and the map read here is that year's, restamped. That pairing is the licence; a
    /// year that bundles its own 8275 gets its own file and map (steps-2/3 review Q1).
    /// [`Self::alias_is_licensed_by`] remains as the documented statement of the hash rule (and its
    /// sp4 plants) but is no longer on this path — as called it could only ever compare the bytes
    /// with themselves.
    pub fn for_year(year: i32) -> Result<Self, FormsError> {
        let (bundled, from_year) =
            crate::bundled::periodic_template(crate::bundled::Stem::F8275, year)
                .ok_or(FormsError::UnsupportedYear(year))?;
        // ★ The invariant that licenses the alias is enforced INSIDE `periodic_template`: it hands
        //   back the bytes together with the year whose row said `periodic`, and the map read below
        //   is that same year's. A hash compare against a fixed 2024 constant here was tautological
        //   as called AND wrong for a year that bundles its own newer 8275 (steps-2/3 review Q1);
        //   the assertion below states the pairing, it is not a guard against a calendar.
        debug_assert_eq!(
            Some(bundled),
            crate::bundled::template(crate::bundled::Stem::F8275, from_year),
            "periodic_template must pair the bytes with the year whose map is read"
        );
        let text = crate::bundled::map_text(crate::bundled::Stem::F8275, from_year)
            .ok_or(FormsError::UnsupportedYear(year))?;
        let mut m = Self::parse(text).map_err(|e| {
            FormsError::Structure(format!(
                "f8275 TY{from_year}: the bundled map does not parse: {e}"
            ))
        })?;
        m.year = year;
        Ok(m)
    }

    /// Every field name the map targets (for the `map_YYYY_matches_bundled_pdf_fieldset` guard).
    pub fn field_names(&self) -> Vec<&str> {
        let mut v = vec![self.identity.name.as_str(), self.identity.ssn.as_str()];
        for r in &self.rows {
            v.extend([
                r.item.as_str(),
                r.desc.as_str(),
                r.form_schedule.as_str(),
                r.amount.as_str(),
            ]);
        }
        v.push(self.part_ii_narrative.as_str());
        v.extend(self.part_ii_continuation.iter().map(String::as_str));
        v.extend(self.part_iv_continuation.iter().map(String::as_str));
        v
    }

    /// Every free-text narrative field this map declares, in printed top-to-bottom order:
    /// `part_ii_narrative` (Part II line 1), then `part_ii_continuation` (Part II lines 2–6), then
    /// `part_iv_continuation` (Part IV lines 1–27).
    ///
    /// ★ This is the map's DECLARED shape, **not** the fill's write set: the fill writes Part II line 1
    /// then spills straight to Part IV, skipping lines 2–6 (see `part_ii_continuation`). Production-dead
    /// as of the Part II overflow fix — retained for tests and for a future per-item Part II numbering.
    /// Do not use it as "the sequence the narrative wraps across".
    pub fn narrative_continuation_fields(&self) -> Vec<&str> {
        let mut v = vec![self.part_ii_narrative.as_str()];
        v.extend(self.part_ii_continuation.iter().map(String::as_str));
        v.extend(self.part_iv_continuation.iter().map(String::as_str));
        v
    }
}

/// The Schedule D field map for one tax year.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScheduleDMap {
    /// `"schedule_d"`.
    pub form: String,
    /// Tax year.
    pub year: i32,
    // ── design r2 §4 — the ROW. The same keys as [`MapRow`]; required ones REFUSE when missing. ──
    /// IRS basename (`"f1040sd"` for `schedule_d`). See [`MapRow::irs_stem`].
    pub irs_stem: String,
    /// Annual or periodic. See [`MapRow::versioning`].
    pub versioning: Versioning,
    /// sha256 of the bundled PDF. See [`MapRow::template_sha256`].
    pub template_sha256: String,
    /// OPTIONAL — the manifest-join excuse. See [`MapRow::authority`].
    #[serde(default)]
    pub authority: Option<String>,
    /// OPTIONAL — a second extract root. See [`MapRow::extract_override`].
    #[serde(default)]
    pub extract_override: Option<String>,
    /// Instructions stem. See [`MapRow::instructions`].
    pub instructions: String,
    /// OPTIONAL — page range in an `i1040gi` booklet. See [`MapRow::instr_pages`].
    #[serde(default)]
    pub instr_pages: Option<[u32; 2]>,
    /// The line-set revision. See [`MapRow::line_set`].
    pub line_set: String,
    /// OPTIONAL — absent on the 1040. See [`MapRow::attachment_sequence`].
    #[serde(default)]
    pub attachment_sequence: Option<String>,
    /// The §G-13 **field census** — every AcroForm field on this year's PDF that this build does NOT
    /// fill, mapped to the [`CensusDecision`] that leaves it blank. Declared, not tolerated: see
    /// [`CensusDecision`] for why a modelled key is what lets `deny_unknown_fields` be switched on.
    #[serde(default)]
    pub census: std::collections::BTreeMap<String, CensusDecision>,
    /// The name + SSN header cells (P6.2). `Option` because this map is SHARED with the crypto-slice
    /// path, whose 2017/2025 editions have no verified identity FQNs and no `ReturnInputs` to source an
    /// identity from. The FULL-return filler refuses on `None` — it may not emit an unnamed form.
    #[serde(default)]
    pub identity: Option<IdentityCells>,
    /// Line 3 — Part I total from Form 8949 (Box C **or Box I**): columns d,e,g,h.
    /// §G-28/B4 — line 1a, the short-term 1099-B totals that need no Form 8949.
    ///
    /// ★ `Option` because only the TY2024 map carries it so far, exactly like `line6`/`line13`/`line14`
    /// beside it. A year whose map lacks the cells and whose filer HAS 1099-B totals fails closed at
    /// fill time (`need`), rather than silently dropping a reported figure off the return.
    #[serde(default)]
    pub line1a: Option<AmountColsNoAdjustment>,
    /// §G-28/B4 — line 8a, the long-term counterpart. Same `Option` treatment as [`Self::line1a`].
    #[serde(default)]
    pub line8a: Option<AmountColsNoAdjustment>,
    pub line3: AmountCols,
    /// Line 7 — net short-term gain/loss (column h).
    pub line7_h: String,
    /// Line 10 — Part II total from Form 8949 (Box F **or Box L**): columns d,e,g,h.
    pub line10: AmountCols,
    /// Line 15 — net long-term gain/loss (column h).
    pub line15_h: String,
    /// Line 16 — total (line 7 + line 15), column h, page 2.
    pub line16_h: String,
    /// L6 — short-term capital loss carryover. **PAREN box ⇒ positive magnitude.** Full-return only
    /// (`None` on the 2017/2025 maps, which serve the crypto-slice fill).
    #[serde(default)]
    pub line6: Option<MoneyCell>,
    /// L13 — capital gain distributions (Σ 1099-DIV box 2a). Full-return only.
    #[serde(default)]
    pub line13: Option<MoneyCell>,
    /// L14 — long-term capital loss carryover. **PAREN box ⇒ positive magnitude.** Full-return only.
    #[serde(default)]
    pub line14: Option<MoneyCell>,
    /// L18 — 28%-Rate Gain Worksheet (always 0; a nonzero amount is refused upstream). Full-return only.
    #[serde(default)]
    pub line18: Option<MoneyCell>,
    /// L19 — Unrecaptured §1250 Gain Worksheet (always 0; refused upstream). Full-return only.
    #[serde(default)]
    pub line19: Option<MoneyCell>,
    /// L21 — the §1211(b) allowed loss offset. **PAREN box ⇒ positive magnitude.** Full-return only.
    #[serde(default)]
    pub line21: Option<MoneyCell>,
    /// L17 — "Are lines 15 and 16 both gains?" Full-return only.
    #[serde(default)]
    pub line17: Option<YesNoPair>,
    /// L20 — "Are lines 18 and 19 both zero or blank…?" Full-return only.
    #[serde(default)]
    pub line20: Option<YesNoPair>,
    /// L22 — "Do you have qualified dividends on Form 1040, line 3a?" Full-return only.
    #[serde(default)]
    pub line22: Option<YesNoPair>,
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
        Self::for_year(2025).expect("the bundled TY2025 map is wired and parses")
    }

    /// The TY2024 map.
    pub fn ty2024() -> Self {
        Self::for_year(2024).expect("the bundled TY2024 map is wired and parses")
    }

    /// The TY2017 map (grid token `TablePartI`; NO QOF question).
    pub fn ty2017() -> Self {
        Self::for_year(2017).expect("the bundled TY2017 map is wired and parses")
    }

    /// The map for a supported tax year.
    /// The map for a tax year — design r2 §10 step 3: the file comes from the glob
    /// (`bundled::map_text`), the revision from its ROW, and the ONE exhaustive
    /// `line_set → schema` match (`line_set::schema`) decides whether THIS struct parses it.
    pub fn for_year(year: i32) -> Result<Self, FormsError> {
        let text = crate::bundled::map_text(crate::bundled::Stem::ScheduleD, year)
            .ok_or(FormsError::UnsupportedYear(year))?;
        let row = MapRow::read(text).map_err(|e| {
            FormsError::Structure(format!(
                "ScheduleD TY{year}: the map's row does not parse: {e}"
            ))
        })?;
        let ls = crate::line_set::LineSet::parse(&row.line_set).ok_or_else(|| {
            FormsError::Structure(format!(
                "ScheduleD TY{year}: line_set {:?} is not a revision this build knows",
                row.line_set
            ))
        })?;
        match crate::line_set::schema(ls) {
            crate::line_set::Schema::ScheduleDMap => Self::parse(text).map_err(|e| {
                FormsError::Structure(format!(
                    "ScheduleD TY{year}: the bundled map does not parse: {e}"
                ))
            }),
            crate::line_set::Schema::Unwired => Err(FormsError::UnwiredLineSet {
                stem: "ScheduleD",
                year,
                line_set: ls.as_str(),
            }),
            other => Err(FormsError::Structure(format!(
                "ScheduleD TY{year}: line_set {} parses into {other:?}, not ScheduleDMap",
                ls.as_str()
            ))),
        }
    }
}

/// The Form 8959 (Additional Medicare Tax) field map for one tax year.
///
/// Only the lines we FILL are mapped. Lines 2/3 (Form 4137 / Form 8919) and all of Part III plus
/// line 23 (RRTA) are unmodeled and are deliberately absent — they stay blank on the filed form,
/// which is why line 4 = line 1, line 18 = 7 + 13, and line 24 = line 22.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Form8959Map {
    /// `"f8959"`.
    pub form: String,
    /// Tax year.
    pub year: i32,
    // ── design r2 §4 — the ROW. The same keys as [`MapRow`]; required ones REFUSE when missing. ──
    /// IRS basename (`"f1040sd"` for `schedule_d`). See [`MapRow::irs_stem`].
    pub irs_stem: String,
    /// Annual or periodic. See [`MapRow::versioning`].
    pub versioning: Versioning,
    /// sha256 of the bundled PDF. See [`MapRow::template_sha256`].
    pub template_sha256: String,
    /// OPTIONAL — the manifest-join excuse. See [`MapRow::authority`].
    #[serde(default)]
    pub authority: Option<String>,
    /// OPTIONAL — a second extract root. See [`MapRow::extract_override`].
    #[serde(default)]
    pub extract_override: Option<String>,
    /// Instructions stem. See [`MapRow::instructions`].
    pub instructions: String,
    /// OPTIONAL — page range in an `i1040gi` booklet. See [`MapRow::instr_pages`].
    #[serde(default)]
    pub instr_pages: Option<[u32; 2]>,
    /// The line-set revision. See [`MapRow::line_set`].
    pub line_set: String,
    /// OPTIONAL — absent on the 1040. See [`MapRow::attachment_sequence`].
    #[serde(default)]
    pub attachment_sequence: Option<String>,
    /// The §G-13 **field census** — every AcroForm field on this year's PDF that this build does NOT
    /// fill, mapped to the [`CensusDecision`] that leaves it blank. Declared, not tolerated: see
    /// [`CensusDecision`] for why a modelled key is what lets `deny_unknown_fields` be switched on.
    #[serde(default)]
    pub census: std::collections::BTreeMap<String, CensusDecision>,
    /// The name + SSN header cells (P6.2). REQUIRED: a full-return schedule that does not name its
    /// taxpayer is not a filable form, so a map lacking `[identity]` fails at deserialization.
    pub identity: IdentityCells,
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
        Self::for_year(2024).expect("the bundled TY2024 map is wired and parses")
    }

    /// The map for a supported tax year. Full-return v1 is **TY2024-only**: Form 8959 is reachable
    /// only from the absolute return, which itself has tables for 2024 alone.
    /// The map for a tax year — design r2 §10 step 3: the file comes from the glob
    /// (`bundled::map_text`), the revision from its ROW, and the ONE exhaustive
    /// `line_set → schema` match (`line_set::schema`) decides whether THIS struct parses it.
    pub fn for_year(year: i32) -> Result<Self, FormsError> {
        let text = crate::bundled::map_text(crate::bundled::Stem::F8959, year)
            .ok_or(FormsError::UnsupportedYear(year))?;
        let row = MapRow::read(text).map_err(|e| {
            FormsError::Structure(format!("F8959 TY{year}: the map's row does not parse: {e}"))
        })?;
        let ls = crate::line_set::LineSet::parse(&row.line_set).ok_or_else(|| {
            FormsError::Structure(format!(
                "F8959 TY{year}: line_set {:?} is not a revision this build knows",
                row.line_set
            ))
        })?;
        match crate::line_set::schema(ls) {
            crate::line_set::Schema::Form8959Map => Self::parse(text).map_err(|e| {
                FormsError::Structure(format!(
                    "F8959 TY{year}: the bundled map does not parse: {e}"
                ))
            }),
            crate::line_set::Schema::Unwired => Err(FormsError::UnwiredLineSet {
                stem: "F8959",
                year,
                line_set: ls.as_str(),
            }),
            other => Err(FormsError::Structure(format!(
                "F8959 TY{year}: line_set {} parses into {other:?}, not Form8959Map",
                ls.as_str()
            ))),
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
#[serde(deny_unknown_fields)]
pub struct Form8960Map {
    /// `"f8960"`.
    pub form: String,
    /// Tax year.
    pub year: i32,
    // ── design r2 §4 — the ROW. The same keys as [`MapRow`]; required ones REFUSE when missing. ──
    /// IRS basename (`"f1040sd"` for `schedule_d`). See [`MapRow::irs_stem`].
    pub irs_stem: String,
    /// Annual or periodic. See [`MapRow::versioning`].
    pub versioning: Versioning,
    /// sha256 of the bundled PDF. See [`MapRow::template_sha256`].
    pub template_sha256: String,
    /// OPTIONAL — the manifest-join excuse. See [`MapRow::authority`].
    #[serde(default)]
    pub authority: Option<String>,
    /// OPTIONAL — a second extract root. See [`MapRow::extract_override`].
    #[serde(default)]
    pub extract_override: Option<String>,
    /// Instructions stem. See [`MapRow::instructions`].
    pub instructions: String,
    /// OPTIONAL — page range in an `i1040gi` booklet. See [`MapRow::instr_pages`].
    #[serde(default)]
    pub instr_pages: Option<[u32; 2]>,
    /// The line-set revision. See [`MapRow::line_set`].
    pub line_set: String,
    /// OPTIONAL — absent on the 1040. See [`MapRow::attachment_sequence`].
    #[serde(default)]
    pub attachment_sequence: Option<String>,
    /// The §G-13 **field census** — every AcroForm field on this year's PDF that this build does NOT
    /// fill, mapped to the [`CensusDecision`] that leaves it blank. Declared, not tolerated: see
    /// [`CensusDecision`] for why a modelled key is what lets `deny_unknown_fields` be switched on.
    #[serde(default)]
    pub census: std::collections::BTreeMap<String, CensusDecision>,
    /// The name + SSN header cells (P6.2). REQUIRED: a full-return schedule that does not name its
    /// taxpayer is not a filable form, so a map lacking `[identity]` fails at deserialization.
    pub identity: IdentityCells,
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
    /// L9b — state/local/foreign income tax allocable to NII, MID column. Written only when the filer
    /// claimed an allocation; `push_money_opt` leaves the cell BLANK otherwise.
    pub line9b: MoneyCell,
    /// L9d — add 9a/9b/9c (= 9b; 9a and 9c unmodelled), AMOUNT column.
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
        Self::for_year(2024).expect("the bundled TY2024 map is wired and parses")
    }
    /// The map for a supported tax year. Full-return v1 is TY2024-only.
    /// The map for a tax year — design r2 §10 step 3: the file comes from the glob
    /// (`bundled::map_text`), the revision from its ROW, and the ONE exhaustive
    /// `line_set → schema` match (`line_set::schema`) decides whether THIS struct parses it.
    pub fn for_year(year: i32) -> Result<Self, FormsError> {
        let text = crate::bundled::map_text(crate::bundled::Stem::F8960, year)
            .ok_or(FormsError::UnsupportedYear(year))?;
        let row = MapRow::read(text).map_err(|e| {
            FormsError::Structure(format!("F8960 TY{year}: the map's row does not parse: {e}"))
        })?;
        let ls = crate::line_set::LineSet::parse(&row.line_set).ok_or_else(|| {
            FormsError::Structure(format!(
                "F8960 TY{year}: line_set {:?} is not a revision this build knows",
                row.line_set
            ))
        })?;
        match crate::line_set::schema(ls) {
            crate::line_set::Schema::Form8960Map => Self::parse(text).map_err(|e| {
                FormsError::Structure(format!(
                    "F8960 TY{year}: the bundled map does not parse: {e}"
                ))
            }),
            crate::line_set::Schema::Unwired => Err(FormsError::UnwiredLineSet {
                stem: "F8960",
                year,
                line_set: ls.as_str(),
            }),
            other => Err(FormsError::Structure(format!(
                "F8960 TY{year}: line_set {} parses into {other:?}, not Form8960Map",
                ls.as_str()
            ))),
        }
    }
    /// The 15 fillable cells in printed reading order (strictly descending y on page 1). ★ 9b is
    /// *fillable*, not always *filled* — the emitter skips it when the filer claimed no allocation,
    /// and `verify_flat`'s descent check compares only the placements actually written, so a skipped
    /// ordinal leaves a gap rather than breaking the sequence.
    pub fn lines(&self) -> [&MoneyCell; 15] {
        [
            &self.line1,
            &self.line2,
            &self.line5a,
            &self.line5d,
            &self.line7,
            &self.line8,
            &self.line9b,
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
/// Form 8995-A (§G-28/B1a) — **Part IV only**. See `forms/2024/f8995a.map.toml` for the scope note
/// and for how the field assignment was corroborated rather than assumed.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Form8995AMap {
    /// `"f8995a"`.
    pub form: String,
    /// Tax year.
    pub year: i32,
    // ── design r2 §4 — the ROW. The same keys as [`MapRow`]; required ones REFUSE when missing. ──
    /// IRS basename (`"f1040sd"` for `schedule_d`). See [`MapRow::irs_stem`].
    pub irs_stem: String,
    /// Annual or periodic. See [`MapRow::versioning`].
    pub versioning: Versioning,
    /// sha256 of the bundled PDF. See [`MapRow::template_sha256`].
    pub template_sha256: String,
    /// OPTIONAL — the manifest-join excuse. See [`MapRow::authority`].
    #[serde(default)]
    pub authority: Option<String>,
    /// OPTIONAL — a second extract root. See [`MapRow::extract_override`].
    #[serde(default)]
    pub extract_override: Option<String>,
    /// Instructions stem. See [`MapRow::instructions`].
    pub instructions: String,
    /// OPTIONAL — page range in an `i1040gi` booklet. See [`MapRow::instr_pages`].
    #[serde(default)]
    pub instr_pages: Option<[u32; 2]>,
    /// The line-set revision. See [`MapRow::line_set`].
    pub line_set: String,
    /// OPTIONAL — absent on the 1040. See [`MapRow::attachment_sequence`].
    #[serde(default)]
    pub attachment_sequence: Option<String>,
    /// The §G-13 **field census** — every AcroForm field on this year's PDF that this build does NOT
    /// fill, mapped to the [`CensusDecision`] that leaves it blank. Declared, not tolerated: see
    /// [`CensusDecision`] for why a modelled key is what lets `deny_unknown_fields` be switched on.
    #[serde(default)]
    pub census: std::collections::BTreeMap<String, CensusDecision>,
    /// Name + SSN. REQUIRED — a schedule that does not name its taxpayer is not filable.
    pub identity: IdentityCells,
    /// Part IV lines 27-40, in the form's own numbering. See `forms/2024/f8995a.map.toml` for how the
    /// assignment was corroborated (column partition, the inset `(loss)` pair, and monotonic y) rather
    /// than assumed from field order.
    pub line27: MoneyCell,
    pub line28: MoneyCell,
    /// Parenthesized — the box supplies the minus sign, so a POSITIVE MAGNITUDE is written.
    pub line29: MoneyCell,
    pub line30: MoneyCell,
    pub line31: MoneyCell,
    pub line32: MoneyCell,
    pub line33: MoneyCell,
    pub line34: MoneyCell,
    pub line35: MoneyCell,
    pub line36: MoneyCell,
    pub line37: MoneyCell,
    /// DPAD — mapped so the cell is authorized, but written only if a value ever exists. btctax fills
    /// no Schedule D (Form 8995-A), so today it is always blank.
    pub line38: MoneyCell,
    pub line39: MoneyCell,
    /// Parenthesized — POSITIVE MAGNITUDE.
    pub line40: MoneyCell,
    /// §G-28/B1b — Part I row A's five columns. See the map TOML for how (b)/(c)/(d)/(e) were assigned
    /// by x-position: the dump lists the checkboxes c1_3, c1_1, c1_2, so reading field order would
    /// transpose "specified service" with "patron".
    pub part1_row_a: Form8995APartIRowACells,
    /// §G-28/B1b — Part II lines 2-16, COLUMN A (the first widget of each row triple).
    pub part2_col_a: Form8995APartIiCells,
    /// §G-28/B1b — Part III lines 17-26. Lines 20-24 are the single `Ln` entry boxes, NOT column A.
    pub part3_col_a: Form8995APartIiiCells,
}

/// Form 8995-A Part I, row A — the five columns the form prints left to right.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Form8995APartIRowACells {
    /// 1(a) "Trade, business, or aggregation name".
    pub name: String,
    /// 1(d) "Taxpayer identification number" (`/MaxLen` 11 ⇒ hyphenated SSN).
    pub tin: String,
    /// 1(b) "Check if specified service".
    pub specified_service: CheckChoice,
    /// 1(c) "Check if aggregation".
    pub aggregation: CheckChoice,
    /// 1(e) "Check if patron".
    pub patron: CheckChoice,
}

/// Form 8995-A Part II, column A — lines 2-16.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Form8995APartIiCells {
    pub line2: MoneyCell,
    pub line3: MoneyCell,
    pub line4: MoneyCell,
    pub line5: MoneyCell,
    pub line6: MoneyCell,
    pub line7: MoneyCell,
    pub line8: MoneyCell,
    pub line9: MoneyCell,
    pub line10: MoneyCell,
    pub line11: MoneyCell,
    /// "Enter the amount from line 26, **if any**" — mapped so the cell is authorized, written only
    /// when Part III ran.
    pub line12: MoneyCell,
    pub line13: MoneyCell,
    /// "Enter the amount from Schedule D (Form 8995-A), line 6, **if any**" — btctax fills no
    /// Schedule D (a patron refuses), so this is always blank.
    pub line14: MoneyCell,
    pub line15: MoneyCell,
    pub line16: MoneyCell,
}

/// Form 8995-A Part III — lines 17-26.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Form8995APartIiiCells {
    pub line17: MoneyCell,
    pub line18: MoneyCell,
    pub line19: MoneyCell,
    /// Lines 20-24 are the `LnNN` single-entry boxes at x≈[266,338], NOT the `_RO` column mirrors.
    pub line20: MoneyCell,
    pub line21: MoneyCell,
    pub line22: MoneyCell,
    pub line23: MoneyCell,
    /// A PERCENTAGE, not a dollar amount — the form prints the `%` beside the box.
    pub line24: MoneyCell,
    pub line25: MoneyCell,
    pub line26: MoneyCell,
}

impl Form8995AMap {
    /// ★ Year dispatch, for the same reason as [`Form6251Map::for_year`] — `packet.rs` hardcoded
    /// `ty2024()` here too, with `year` in scope. Fails closed on an unmapped year.
    /// The map for a tax year — design r2 §10 step 3: the file comes from the glob
    /// (`bundled::map_text`), the revision from its ROW, and the ONE exhaustive
    /// `line_set → schema` match (`line_set::schema`) decides whether THIS struct parses it.
    pub fn for_year(year: i32) -> Result<Self, crate::FormsError> {
        let text = crate::bundled::map_text(crate::bundled::Stem::F8995a, year)
            .ok_or(crate::FormsError::UnsupportedYear(year))?;
        let row = MapRow::read(text).map_err(|e| {
            crate::FormsError::Structure(format!(
                "F8995a TY{year}: the map's row does not parse: {e}"
            ))
        })?;
        let ls = crate::line_set::LineSet::parse(&row.line_set).ok_or_else(|| {
            crate::FormsError::Structure(format!(
                "F8995a TY{year}: line_set {:?} is not a revision this build knows",
                row.line_set
            ))
        })?;
        match crate::line_set::schema(ls) {
            crate::line_set::Schema::Form8995AMap => Self::parse(text).map_err(|e| {
                crate::FormsError::Structure(format!(
                    "F8995a TY{year}: the bundled map does not parse: {e}"
                ))
            }),
            crate::line_set::Schema::Unwired => Err(crate::FormsError::UnwiredLineSet {
                stem: "F8995a",
                year,
                line_set: ls.as_str(),
            }),
            other => Err(crate::FormsError::Structure(format!(
                "F8995a TY{year}: line_set {} parses into {other:?}, not Form8995AMap",
                ls.as_str()
            ))),
        }
    }

    /// The bundled TY2024 map.
    pub fn ty2024() -> Self {
        Self::for_year(2024).expect("the bundled TY2024 map is wired and parses")
    }
    pub fn parse(s: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(s)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Form8995Map {
    /// `"f8995"`.
    pub form: String,
    /// Tax year.
    pub year: i32,
    // ── design r2 §4 — the ROW. The same keys as [`MapRow`]; required ones REFUSE when missing. ──
    /// IRS basename (`"f1040sd"` for `schedule_d`). See [`MapRow::irs_stem`].
    pub irs_stem: String,
    /// Annual or periodic. See [`MapRow::versioning`].
    pub versioning: Versioning,
    /// sha256 of the bundled PDF. See [`MapRow::template_sha256`].
    pub template_sha256: String,
    /// OPTIONAL — the manifest-join excuse. See [`MapRow::authority`].
    #[serde(default)]
    pub authority: Option<String>,
    /// OPTIONAL — a second extract root. See [`MapRow::extract_override`].
    #[serde(default)]
    pub extract_override: Option<String>,
    /// Instructions stem. See [`MapRow::instructions`].
    pub instructions: String,
    /// OPTIONAL — page range in an `i1040gi` booklet. See [`MapRow::instr_pages`].
    #[serde(default)]
    pub instr_pages: Option<[u32; 2]>,
    /// The line-set revision. See [`MapRow::line_set`].
    pub line_set: String,
    /// OPTIONAL — absent on the 1040. See [`MapRow::attachment_sequence`].
    #[serde(default)]
    pub attachment_sequence: Option<String>,
    /// The §G-13 **field census** — every AcroForm field on this year's PDF that this build does NOT
    /// fill, mapped to the [`CensusDecision`] that leaves it blank. Declared, not tolerated: see
    /// [`CensusDecision`] for why a modelled key is what lets `deny_unknown_fields` be switched on.
    #[serde(default)]
    pub census: std::collections::BTreeMap<String, CensusDecision>,
    /// The name + SSN header cells (P6.2). REQUIRED: a full-return schedule that does not name its
    /// taxpayer is not a filable form, so a map lacking `[identity]` fails at deserialization.
    pub identity: IdentityCells,
    /// Part I row 1i(a) — the trade or business's description.
    pub row1_business: MoneyCell,
    /// Part I row 1i(b) — its TIN (the filer's SSN; `/MaxLen` 11 ⇒ hyphenated).
    pub row1_tin: MoneyCell,
    /// Part I row 1i(c) — its QBI. With one business this IS line 2, which the form totals from it.
    pub row1_qbi: MoneyCell,
    /// L2 — total QBI: "Combine lines 1i through 1v, column (c)". MID column.
    pub line2: MoneyCell,
    /// ★ L3 — prior-year qualified business net (loss) carryforward, MID column (the paren inset,
    /// x=[414.4,478.4], same band as line 7). ★ positive magnitude (paren box).
    pub line3: MoneyCell,
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
        Self::for_year(2024).expect("the bundled TY2024 map is wired and parses")
    }
    /// The map for a supported tax year. Full-return v1 is TY2024-only.
    /// The map for a tax year — design r2 §10 step 3: the file comes from the glob
    /// (`bundled::map_text`), the revision from its ROW, and the ONE exhaustive
    /// `line_set → schema` match (`line_set::schema`) decides whether THIS struct parses it.
    pub fn for_year(year: i32) -> Result<Self, FormsError> {
        let text = crate::bundled::map_text(crate::bundled::Stem::F8995, year)
            .ok_or(FormsError::UnsupportedYear(year))?;
        let row = MapRow::read(text).map_err(|e| {
            FormsError::Structure(format!("F8995 TY{year}: the map's row does not parse: {e}"))
        })?;
        let ls = crate::line_set::LineSet::parse(&row.line_set).ok_or_else(|| {
            FormsError::Structure(format!(
                "F8995 TY{year}: line_set {:?} is not a revision this build knows",
                row.line_set
            ))
        })?;
        match crate::line_set::schema(ls) {
            crate::line_set::Schema::Form8995Map => Self::parse(text).map_err(|e| {
                FormsError::Structure(format!(
                    "F8995 TY{year}: the bundled map does not parse: {e}"
                ))
            }),
            crate::line_set::Schema::Unwired => Err(FormsError::UnwiredLineSet {
                stem: "F8995",
                year,
                line_set: ls.as_str(),
            }),
            other => Err(FormsError::Structure(format!(
                "F8995 TY{year}: line_set {} parses into {other:?}, not Form8995Map",
                ls.as_str()
            ))),
        }
    }
    /// The 15 filled cells in printed reading order (strictly descending y on page 1).
    pub fn lines(&self) -> [&MoneyCell; 16] {
        [
            &self.line2,
            &self.line3,
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
/// line 2 (AMT) is $0 by construction — line 7 ≤ line 10 ⇒ Form 6251 line 11 is $0, and a return where line 7 EXCEEDS line 10 is refused (Who Must File condition 1 — v1 computes the form but cannot file it). (The
/// pre-v0.14.0 rationale, "refused if the Form 6251 SCREEN trips", is obsolete: the screening
/// worksheet is no longer on any production path.) Only the three Part II taxes v1 computes are
/// mapped. **Line 21 is on PAGE 2.**
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Schedule2Map {
    /// `"f1040s2"`.
    pub form: String,
    /// Tax year.
    pub year: i32,
    // ── design r2 §4 — the ROW. The same keys as [`MapRow`]; required ones REFUSE when missing. ──
    /// IRS basename (`"f1040sd"` for `schedule_d`). See [`MapRow::irs_stem`].
    pub irs_stem: String,
    /// Annual or periodic. See [`MapRow::versioning`].
    pub versioning: Versioning,
    /// sha256 of the bundled PDF. See [`MapRow::template_sha256`].
    pub template_sha256: String,
    /// OPTIONAL — the manifest-join excuse. See [`MapRow::authority`].
    #[serde(default)]
    pub authority: Option<String>,
    /// OPTIONAL — a second extract root. See [`MapRow::extract_override`].
    #[serde(default)]
    pub extract_override: Option<String>,
    /// Instructions stem. See [`MapRow::instructions`].
    pub instructions: String,
    /// OPTIONAL — page range in an `i1040gi` booklet. See [`MapRow::instr_pages`].
    #[serde(default)]
    pub instr_pages: Option<[u32; 2]>,
    /// The line-set revision. See [`MapRow::line_set`].
    pub line_set: String,
    /// OPTIONAL — absent on the 1040. See [`MapRow::attachment_sequence`].
    #[serde(default)]
    pub attachment_sequence: Option<String>,
    /// The §G-13 **field census** — every AcroForm field on this year's PDF that this build does NOT
    /// fill, mapped to the [`CensusDecision`] that leaves it blank. Declared, not tolerated: see
    /// [`CensusDecision`] for why a modelled key is what lets `deny_unknown_fields` be switched on.
    #[serde(default)]
    pub census: std::collections::BTreeMap<String, CensusDecision>,
    /// The name + SSN header cells (P6.2). REQUIRED: a full-return schedule that does not name its
    /// taxpayer is not a filable form, so a map lacking `[identity]` fails at deserialization.
    pub identity: IdentityCells,
    /// L4 — self-employment tax (SS + regular Medicare only), AMOUNT column, page 1.
    /// §G-6 — L2, the AMT from Form 6251 line 11.
    pub line2: MoneyCell,
    /// §G-6 — L3, "Add lines 1z and 2", which **1040 line 17 names by number**. Blank while Part I is
    /// empty; once an AMT lands on line 2 the 1040 carries a figure that must be visible here.
    pub line3: MoneyCell,
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
        Self::for_year(2024).expect("the bundled TY2024 map is wired and parses")
    }
    /// The map for a supported tax year. Full-return v1 is TY2024-only.
    /// The map for a tax year — design r2 §10 step 3: the file comes from the glob
    /// (`bundled::map_text`), the revision from its ROW, and the ONE exhaustive
    /// `line_set → schema` match (`line_set::schema`) decides whether THIS struct parses it.
    pub fn for_year(year: i32) -> Result<Self, FormsError> {
        let text = crate::bundled::map_text(crate::bundled::Stem::F1040s2, year)
            .ok_or(FormsError::UnsupportedYear(year))?;
        let row = MapRow::read(text).map_err(|e| {
            FormsError::Structure(format!(
                "F1040s2 TY{year}: the map's row does not parse: {e}"
            ))
        })?;
        let ls = crate::line_set::LineSet::parse(&row.line_set).ok_or_else(|| {
            FormsError::Structure(format!(
                "F1040s2 TY{year}: line_set {:?} is not a revision this build knows",
                row.line_set
            ))
        })?;
        match crate::line_set::schema(ls) {
            crate::line_set::Schema::Schedule2Map => Self::parse(text).map_err(|e| {
                FormsError::Structure(format!(
                    "F1040s2 TY{year}: the bundled map does not parse: {e}"
                ))
            }),
            crate::line_set::Schema::Unwired => Err(FormsError::UnwiredLineSet {
                stem: "F1040s2",
                year,
                line_set: ls.as_str(),
            }),
            other => Err(FormsError::Structure(format!(
                "F1040s2 TY{year}: line_set {} parses into {other:?}, not Schedule2Map",
                ls.as_str()
            ))),
        }
    }
    /// The 6 filled cells in printed reading order. **Descent is grouped by PAGE** — line 21 sits on
    /// page 2, whose y-coordinates are not comparable with page 1's.
    /// ★ §G-6 — line 2 (the AMT) leads, so its descent ordinal is 0 on page 1.
    pub fn lines(&self) -> [&MoneyCell; 6] {
        [
            &self.line2,
            &self.line3,
            &self.line4,
            &self.line11,
            &self.line12,
            &self.line21,
        ]
    }
}

/// The Schedule 3 (Additional Credits and Payments) field map for one tax year.
///
/// Only the foreign tax credit (L1) and the §6413(c) excess-Social-Security credit (L11) are mapped.
/// Every other Part I credit is a §3.4 conservative omission and stays BLANK.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Schedule3Map {
    /// `"f1040s3"`.
    pub form: String,
    /// Tax year.
    pub year: i32,
    // ── design r2 §4 — the ROW. The same keys as [`MapRow`]; required ones REFUSE when missing. ──
    /// IRS basename (`"f1040sd"` for `schedule_d`). See [`MapRow::irs_stem`].
    pub irs_stem: String,
    /// Annual or periodic. See [`MapRow::versioning`].
    pub versioning: Versioning,
    /// sha256 of the bundled PDF. See [`MapRow::template_sha256`].
    pub template_sha256: String,
    /// OPTIONAL — the manifest-join excuse. See [`MapRow::authority`].
    #[serde(default)]
    pub authority: Option<String>,
    /// OPTIONAL — a second extract root. See [`MapRow::extract_override`].
    #[serde(default)]
    pub extract_override: Option<String>,
    /// Instructions stem. See [`MapRow::instructions`].
    pub instructions: String,
    /// OPTIONAL — page range in an `i1040gi` booklet. See [`MapRow::instr_pages`].
    #[serde(default)]
    pub instr_pages: Option<[u32; 2]>,
    /// The line-set revision. See [`MapRow::line_set`].
    pub line_set: String,
    /// OPTIONAL — absent on the 1040. See [`MapRow::attachment_sequence`].
    #[serde(default)]
    pub attachment_sequence: Option<String>,
    /// The §G-13 **field census** — every AcroForm field on this year's PDF that this build does NOT
    /// fill, mapped to the [`CensusDecision`] that leaves it blank. Declared, not tolerated: see
    /// [`CensusDecision`] for why a modelled key is what lets `deny_unknown_fields` be switched on.
    #[serde(default)]
    pub census: std::collections::BTreeMap<String, CensusDecision>,
    /// The name + SSN header cells (P6.2). REQUIRED: a full-return schedule that does not name its
    /// taxpayer is not a filable form, so a map lacking `[identity]` fails at deserialization.
    pub identity: IdentityCells,
    /// L1 — foreign tax credit, AMOUNT column.
    pub line1: MoneyCell,
    /// L8 — total nonrefundable credits → 1040 L20, AMOUNT column.
    pub line8: MoneyCell,
    /// L10 — "Amount paid with request for extension to file", AMOUNT column. ★ Its absence made the
    /// filed return demand a payment the filer had ALREADY made (Fable ARCH-P6.3a D1).
    pub line10: MoneyCell,
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
        Self::for_year(2024).expect("the bundled TY2024 map is wired and parses")
    }
    /// The map for a supported tax year. Full-return v1 is TY2024-only.
    /// The map for a tax year — design r2 §10 step 3: the file comes from the glob
    /// (`bundled::map_text`), the revision from its ROW, and the ONE exhaustive
    /// `line_set → schema` match (`line_set::schema`) decides whether THIS struct parses it.
    pub fn for_year(year: i32) -> Result<Self, FormsError> {
        let text = crate::bundled::map_text(crate::bundled::Stem::F1040s3, year)
            .ok_or(FormsError::UnsupportedYear(year))?;
        let row = MapRow::read(text).map_err(|e| {
            FormsError::Structure(format!(
                "F1040s3 TY{year}: the map's row does not parse: {e}"
            ))
        })?;
        let ls = crate::line_set::LineSet::parse(&row.line_set).ok_or_else(|| {
            FormsError::Structure(format!(
                "F1040s3 TY{year}: line_set {:?} is not a revision this build knows",
                row.line_set
            ))
        })?;
        match crate::line_set::schema(ls) {
            crate::line_set::Schema::Schedule3Map => Self::parse(text).map_err(|e| {
                FormsError::Structure(format!(
                    "F1040s3 TY{year}: the bundled map does not parse: {e}"
                ))
            }),
            crate::line_set::Schema::Unwired => Err(FormsError::UnwiredLineSet {
                stem: "F1040s3",
                year,
                line_set: ls.as_str(),
            }),
            other => Err(FormsError::Structure(format!(
                "F1040s3 TY{year}: line_set {} parses into {other:?}, not Schedule3Map",
                ls.as_str()
            ))),
        }
    }
    /// The 4 filled cells in printed reading order (strictly descending y on page 1).
    pub fn lines(&self) -> [&MoneyCell; 5] {
        [
            &self.line1,
            &self.line8,
            &self.line10,
            &self.line11,
            &self.line15,
        ]
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
#[serde(deny_unknown_fields)]
pub struct ScheduleAMap {
    /// `"f1040sa"`.
    pub form: String,
    /// Tax year.
    pub year: i32,
    // ── design r2 §4 — the ROW. The same keys as [`MapRow`]; required ones REFUSE when missing. ──
    /// IRS basename (`"f1040sd"` for `schedule_d`). See [`MapRow::irs_stem`].
    pub irs_stem: String,
    /// Annual or periodic. See [`MapRow::versioning`].
    pub versioning: Versioning,
    /// sha256 of the bundled PDF. See [`MapRow::template_sha256`].
    pub template_sha256: String,
    /// OPTIONAL — the manifest-join excuse. See [`MapRow::authority`].
    #[serde(default)]
    pub authority: Option<String>,
    /// OPTIONAL — a second extract root. See [`MapRow::extract_override`].
    #[serde(default)]
    pub extract_override: Option<String>,
    /// Instructions stem. See [`MapRow::instructions`].
    pub instructions: String,
    /// OPTIONAL — page range in an `i1040gi` booklet. See [`MapRow::instr_pages`].
    #[serde(default)]
    pub instr_pages: Option<[u32; 2]>,
    /// The line-set revision. See [`MapRow::line_set`].
    pub line_set: String,
    /// OPTIONAL — absent on the 1040. See [`MapRow::attachment_sequence`].
    #[serde(default)]
    pub attachment_sequence: Option<String>,
    /// The §G-13 **field census** — every AcroForm field on this year's PDF that this build does NOT
    /// fill, mapped to the [`CensusDecision`] that leaves it blank. Declared, not tolerated: see
    /// [`CensusDecision`] for why a modelled key is what lets `deny_unknown_fields` be switched on.
    #[serde(default)]
    pub census: std::collections::BTreeMap<String, CensusDecision>,
    /// L5a's §164(b)(5) sales-tax election checkbox — the election core already honours in the
    /// arithmetic, which the filed form never showed (ARCH-P6.3a Q7 item 3).
    pub check_5a_sales_tax: CheckChoice,
    /// ★ §2.7 — L8's §163(h)(3)(F) mixed-use-mortgage checkbox: "If you didn't use all of your home
    /// mortgage loan(s) to buy, build, or improve your home, check this box." Nested under
    /// `Line8_ReadOrder[0]`, like line 18's own read-order box.
    pub check_8_mixed_use: CheckChoice,
    /// L18's §63(e) "itemize even though less than the standard deduction" checkbox (Q7 item 4).
    pub check_18_elects_smaller: CheckChoice,
    /// The name + SSN header cells (P6.2). REQUIRED: a full-return schedule that does not name its
    /// taxpayer is not a filable form, so a map lacking `[identity]` fails at deserialization.
    pub identity: IdentityCells,
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
    /// L9 — investment interest (§163(d) / Form 4952), MID column.
    pub line9: MoneyCell,
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
        Self::for_year(2024).expect("the bundled TY2024 map is wired and parses")
    }
    /// The map for a supported tax year. Full-return v1 is TY2024-only.
    /// The map for a tax year — design r2 §10 step 3: the file comes from the glob
    /// (`bundled::map_text`), the revision from its ROW, and the ONE exhaustive
    /// `line_set → schema` match (`line_set::schema`) decides whether THIS struct parses it.
    pub fn for_year(year: i32) -> Result<Self, FormsError> {
        let text = crate::bundled::map_text(crate::bundled::Stem::F1040sa, year)
            .ok_or(FormsError::UnsupportedYear(year))?;
        let row = MapRow::read(text).map_err(|e| {
            FormsError::Structure(format!(
                "F1040sa TY{year}: the map's row does not parse: {e}"
            ))
        })?;
        let ls = crate::line_set::LineSet::parse(&row.line_set).ok_or_else(|| {
            FormsError::Structure(format!(
                "F1040sa TY{year}: line_set {:?} is not a revision this build knows",
                row.line_set
            ))
        })?;
        match crate::line_set::schema(ls) {
            crate::line_set::Schema::ScheduleAMap => Self::parse(text).map_err(|e| {
                FormsError::Structure(format!(
                    "F1040sa TY{year}: the bundled map does not parse: {e}"
                ))
            }),
            crate::line_set::Schema::Unwired => Err(FormsError::UnwiredLineSet {
                stem: "F1040sa",
                year,
                line_set: ls.as_str(),
            }),
            other => Err(FormsError::Structure(format!(
                "F1040sa TY{year}: line_set {} parses into {other:?}, not ScheduleAMap",
                ls.as_str()
            ))),
        }
    }
    /// The 19 filled cells in printed reading order (strictly descending y on page 1).
    pub fn lines(&self) -> [&MoneyCell; 19] {
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
            &self.line9,
            &self.line10,
            &self.line11,
            &self.line12,
            &self.line13,
            &self.line14,
            &self.line17,
        ]
    }
}

/// The Schedule 1 (Additional Income and Adjustments to Income) field map for one tax year.
///
/// Root subform is `form1[0]` (as on Schedule 2), NOT `topmostSubform[0]`. **Two pages** — Part II is
/// entirely on page 2, so descent is grouped by page.
///
/// **Line 22 is a ReadOnly "Reserved for future use" widget** that consumes a suffix number; never
/// written. Non-money fields (a date on 2b, an SSN comb on 19b, a date on 19c) sit inside the money
/// x-band — writing a dollar amount into one prints garbage.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Schedule1Map {
    /// `"f1040s1"`.
    pub form: String,
    /// Tax year.
    pub year: i32,
    // ── design r2 §4 — the ROW. The same keys as [`MapRow`]; required ones REFUSE when missing. ──
    /// IRS basename (`"f1040sd"` for `schedule_d`). See [`MapRow::irs_stem`].
    pub irs_stem: String,
    /// Annual or periodic. See [`MapRow::versioning`].
    pub versioning: Versioning,
    /// sha256 of the bundled PDF. See [`MapRow::template_sha256`].
    pub template_sha256: String,
    /// OPTIONAL — the manifest-join excuse. See [`MapRow::authority`].
    #[serde(default)]
    pub authority: Option<String>,
    /// OPTIONAL — a second extract root. See [`MapRow::extract_override`].
    #[serde(default)]
    pub extract_override: Option<String>,
    /// Instructions stem. See [`MapRow::instructions`].
    pub instructions: String,
    /// OPTIONAL — page range in an `i1040gi` booklet. See [`MapRow::instr_pages`].
    #[serde(default)]
    pub instr_pages: Option<[u32; 2]>,
    /// The line-set revision. See [`MapRow::line_set`].
    pub line_set: String,
    /// OPTIONAL — absent on the 1040. See [`MapRow::attachment_sequence`].
    #[serde(default)]
    pub attachment_sequence: Option<String>,
    /// The §G-13 **field census** — every AcroForm field on this year's PDF that this build does NOT
    /// fill, mapped to the [`CensusDecision`] that leaves it blank. Declared, not tolerated: see
    /// [`CensusDecision`] for why a modelled key is what lets `deny_unknown_fields` be switched on.
    #[serde(default)]
    pub census: std::collections::BTreeMap<String, CensusDecision>,
    /// The name + SSN header cells (P6.2). REQUIRED: a full-return schedule that does not name its
    /// taxpayer is not a filable form, so a map lacking `[identity]` fails at deserialization.
    pub identity: IdentityCells,
    /// L1 — taxable state/local refund, AMOUNT column, page 1.
    pub line1: MoneyCell,
    /// L3 — business income (crypto Schedule C net), AMOUNT column, page 1.
    pub line3: MoneyCell,
    /// L7 — unemployment compensation, AMOUNT column, page 1.
    pub line7: MoneyCell,
    /// L8v — digital assets received as ordinary income, **MID column**, page 1.
    pub line8v: MoneyCell,
    /// L9 — total other income, AMOUNT column, page 1.
    pub line9: MoneyCell,
    /// L10 — combine 1–7 and 9 → 1040 L8, AMOUNT column, page 1.
    pub line10: MoneyCell,
    /// L15 — deductible part of SE tax, AMOUNT column, **page 2**.
    pub line15: MoneyCell,
    /// L18 — early-withdrawal penalty, AMOUNT column, page 2.
    pub line18: MoneyCell,
    /// L21 — student-loan interest deduction, AMOUNT column, page 2.
    pub line21: MoneyCell,
    /// L26 — total adjustments → 1040 L10, AMOUNT column, page 2.
    pub line26: MoneyCell,
}

impl Schedule1Map {
    /// Parse the committed TOML.
    pub fn parse(toml_src: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_src)
    }
    /// The TY2024 map.
    pub fn ty2024() -> Self {
        Self::for_year(2024).expect("the bundled TY2024 map is wired and parses")
    }
    /// The map for a supported tax year. Full-return v1 is TY2024-only.
    /// The map for a tax year — design r2 §10 step 3: the file comes from the glob
    /// (`bundled::map_text`), the revision from its ROW, and the ONE exhaustive
    /// `line_set → schema` match (`line_set::schema`) decides whether THIS struct parses it.
    pub fn for_year(year: i32) -> Result<Self, FormsError> {
        let text = crate::bundled::map_text(crate::bundled::Stem::F1040s1, year)
            .ok_or(FormsError::UnsupportedYear(year))?;
        let row = MapRow::read(text).map_err(|e| {
            FormsError::Structure(format!(
                "F1040s1 TY{year}: the map's row does not parse: {e}"
            ))
        })?;
        let ls = crate::line_set::LineSet::parse(&row.line_set).ok_or_else(|| {
            FormsError::Structure(format!(
                "F1040s1 TY{year}: line_set {:?} is not a revision this build knows",
                row.line_set
            ))
        })?;
        match crate::line_set::schema(ls) {
            crate::line_set::Schema::Schedule1Map => Self::parse(text).map_err(|e| {
                FormsError::Structure(format!(
                    "F1040s1 TY{year}: the bundled map does not parse: {e}"
                ))
            }),
            crate::line_set::Schema::Unwired => Err(FormsError::UnwiredLineSet {
                stem: "F1040s1",
                year,
                line_set: ls.as_str(),
            }),
            other => Err(FormsError::Structure(format!(
                "F1040s1 TY{year}: line_set {} parses into {other:?}, not Schedule1Map",
                ls.as_str()
            ))),
        }
    }
    /// The 10 filled cells in printed reading order. **Descent is grouped by PAGE.**
    pub fn lines(&self) -> [&MoneyCell; 10] {
        [
            &self.line1,
            &self.line3,
            &self.line7,
            &self.line8v,
            &self.line9,
            &self.line10,
            &self.line15,
            &self.line18,
            &self.line21,
            &self.line26,
        ]
    }
}

/// The Schedule C (Profit or Loss From Business) field map — the crypto trade or business.
///
/// **Its money column is x ≈ [475, 576]** — not the [504, 576] of Schedules 1/2/3/A and Forms
/// 8959/8960/8995, and not Schedule B's [489.6, 576]. No amount-column constant is shared between
/// forms in this crate, and none may be.
///
/// Part II's individual expense lines (8–27b) are unmapped: v1 takes a FLAT expense total, so only
/// line 28 is printed. Line 30 (home office) and the line-32 at-risk checkboxes are unmapped too — a
/// Schedule C loss refuses upstream, so line 31 is always ≥ 0.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScheduleCMap {
    /// `"f1040sc"`.
    pub form: String,
    /// Tax year.
    pub year: i32,
    // ── design r2 §4 — the ROW. The same keys as [`MapRow`]; required ones REFUSE when missing. ──
    /// IRS basename (`"f1040sd"` for `schedule_d`). See [`MapRow::irs_stem`].
    pub irs_stem: String,
    /// Annual or periodic. See [`MapRow::versioning`].
    pub versioning: Versioning,
    /// sha256 of the bundled PDF. See [`MapRow::template_sha256`].
    pub template_sha256: String,
    /// OPTIONAL — the manifest-join excuse. See [`MapRow::authority`].
    #[serde(default)]
    pub authority: Option<String>,
    /// OPTIONAL — a second extract root. See [`MapRow::extract_override`].
    #[serde(default)]
    pub extract_override: Option<String>,
    /// Instructions stem. See [`MapRow::instructions`].
    pub instructions: String,
    /// OPTIONAL — page range in an `i1040gi` booklet. See [`MapRow::instr_pages`].
    #[serde(default)]
    pub instr_pages: Option<[u32; 2]>,
    /// The line-set revision. See [`MapRow::line_set`].
    pub line_set: String,
    /// OPTIONAL — absent on the 1040. See [`MapRow::attachment_sequence`].
    #[serde(default)]
    pub attachment_sequence: Option<String>,
    /// The §G-13 **field census** — every AcroForm field on this year's PDF that this build does NOT
    /// fill, mapped to the [`CensusDecision`] that leaves it blank. Declared, not tolerated: see
    /// [`CensusDecision`] for why a modelled key is what lets `deny_unknown_fields` be switched on.
    #[serde(default)]
    pub census: std::collections::BTreeMap<String, CensusDecision>,
    /// Line A — "Principal business or profession".
    pub line_a_business: String,
    /// Line B — the NAICS code (a 6-character comb).
    pub line_b_naics: String,
    /// Line F — the accounting-method checkboxes. `(1) Cash` and `(2) Accrual`; `(3) Other` is never
    /// checked (v1 captures only the two).
    pub method_cash: CheckChoice,
    pub method_accrual: CheckChoice,
    /// ★ Line **I** — "Did you make any payments … that would require you to file Form(s) 1099?"
    ///
    /// ★★ ON-STATES: Schedule C's Yes/No pairs are **`"Yes"`/`"No"`**, NOT the `"1"`/`"2"` that
    /// Schedule B and Schedule D use. Dumped with `xtask dump-fields`; three separate design passes
    /// asserted 1/2 by analogy and all three were wrong. `Option` because only the full-return
    /// revision carries these cells.
    #[serde(default)]
    pub line_i: Option<YesNoPair>,
    /// Line **J** — "If 'Yes,' did you or will you file required Form(s) 1099?"
    #[serde(default)]
    pub line_j: Option<YesNoPair>,
    /// The name + SSN header cells (P6.2). REQUIRED: a full-return schedule that does not name its
    /// taxpayer is not a filable form, so a map lacking `[identity]` fails at deserialization.
    pub identity: IdentityCells,
    /// L1 — gross receipts or sales.
    pub line1: MoneyCell,
    /// L3 — line 1 − line 2 (returns, blank).
    pub line3: MoneyCell,
    /// L5 — gross profit (line 3 − line 4, COGS blank).
    pub line5: MoneyCell,
    /// L7 — gross income (line 5 + line 6, other income blank).
    pub line7: MoneyCell,
    /// L28 — total expenses.
    pub line28: MoneyCell,
    /// L29 — tentative profit (line 7 − line 28).
    pub line29: MoneyCell,
    /// L31 — net profit → Schedule 1 L3 **and** Schedule SE L2.
    pub line31: MoneyCell,
}

impl ScheduleCMap {
    /// Parse the committed TOML.
    pub fn parse(toml_src: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_src)
    }
    /// The TY2024 map.
    pub fn ty2024() -> Self {
        Self::for_year(2024).expect("the bundled TY2024 map is wired and parses")
    }
    /// The map for a supported tax year. Full-return v1 is TY2024-only.
    /// The map for a tax year — design r2 §10 step 3: the file comes from the glob
    /// (`bundled::map_text`), the revision from its ROW, and the ONE exhaustive
    /// `line_set → schema` match (`line_set::schema`) decides whether THIS struct parses it.
    pub fn for_year(year: i32) -> Result<Self, FormsError> {
        let text = crate::bundled::map_text(crate::bundled::Stem::F1040sc, year)
            .ok_or(FormsError::UnsupportedYear(year))?;
        let row = MapRow::read(text).map_err(|e| {
            FormsError::Structure(format!(
                "F1040sc TY{year}: the map's row does not parse: {e}"
            ))
        })?;
        let ls = crate::line_set::LineSet::parse(&row.line_set).ok_or_else(|| {
            FormsError::Structure(format!(
                "F1040sc TY{year}: line_set {:?} is not a revision this build knows",
                row.line_set
            ))
        })?;
        match crate::line_set::schema(ls) {
            crate::line_set::Schema::ScheduleCMap => Self::parse(text).map_err(|e| {
                FormsError::Structure(format!(
                    "F1040sc TY{year}: the bundled map does not parse: {e}"
                ))
            }),
            crate::line_set::Schema::Unwired => Err(FormsError::UnwiredLineSet {
                stem: "F1040sc",
                year,
                line_set: ls.as_str(),
            }),
            other => Err(FormsError::Structure(format!(
                "F1040sc TY{year}: line_set {} parses into {other:?}, not ScheduleCMap",
                ls.as_str()
            ))),
        }
    }
    /// The 7 filled cells in printed reading order (strictly descending y on page 1).
    pub fn lines(&self) -> [&MoneyCell; 7] {
        [
            &self.line1,
            &self.line3,
            &self.line5,
            &self.line7,
            &self.line28,
            &self.line29,
            &self.line31,
        ]
    }
}

/// One listed-payer row on Schedule B: the payer-name text cell + the amount cell.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScheduleBRowMap {
    /// The payer-name field (a wide text cell in the PAYER column).
    pub payer: String,
    /// The amount field.
    pub amount: MoneyCell,
}

/// A Yes/No checkbox pair (Schedule B Part III). Both boxes share the same on-states (`"1"`/`"2"`) and
/// the same x geometry across every pair on the form, so only the field NAME distinguishes them.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct YesNoPair {
    /// The "Yes" box.
    pub yes: CheckChoice,
    /// The "No" box.
    pub no: CheckChoice,
}

/// The Schedule B (Interest and Ordinary Dividends) field map for one tax year.
///
/// **Its amount column is x ≈ [489.6, 576]** — not the [504, 576] of Schedules 1/2/3/A and Forms
/// 8959/8960/8995, nor Schedule C's [475, 576]. A shared constant would reject every cell.
///
/// **Row 1 of BOTH repeating tables has a different parent subform** (`Line1_ReadOrder` in Part I,
/// `ReadOrderControl` in Part II) while its amount sibling does not — so the rows are written out in
/// full in the TOML rather than interpolated. **Part I has 14 rows, Part II has 15**; the asymmetry
/// is real.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScheduleBMap {
    /// `"f1040sb"`.
    pub form: String,
    /// Tax year.
    pub year: i32,
    // ── design r2 §4 — the ROW. The same keys as [`MapRow`]; required ones REFUSE when missing. ──
    /// IRS basename (`"f1040sd"` for `schedule_d`). See [`MapRow::irs_stem`].
    pub irs_stem: String,
    /// Annual or periodic. See [`MapRow::versioning`].
    pub versioning: Versioning,
    /// sha256 of the bundled PDF. See [`MapRow::template_sha256`].
    pub template_sha256: String,
    /// OPTIONAL — the manifest-join excuse. See [`MapRow::authority`].
    #[serde(default)]
    pub authority: Option<String>,
    /// OPTIONAL — a second extract root. See [`MapRow::extract_override`].
    #[serde(default)]
    pub extract_override: Option<String>,
    /// Instructions stem. See [`MapRow::instructions`].
    pub instructions: String,
    /// OPTIONAL — page range in an `i1040gi` booklet. See [`MapRow::instr_pages`].
    #[serde(default)]
    pub instr_pages: Option<[u32; 2]>,
    /// The line-set revision. See [`MapRow::line_set`].
    pub line_set: String,
    /// OPTIONAL — absent on the 1040. See [`MapRow::attachment_sequence`].
    #[serde(default)]
    pub attachment_sequence: Option<String>,
    /// The §G-13 **field census** — every AcroForm field on this year's PDF that this build does NOT
    /// fill, mapped to the [`CensusDecision`] that leaves it blank. Declared, not tolerated: see
    /// [`CensusDecision`] for why a modelled key is what lets `deny_unknown_fields` be switched on.
    #[serde(default)]
    pub census: std::collections::BTreeMap<String, CensusDecision>,
    /// L7b — the foreign-country list. It IS a captured input; the claim that v1 had none was false
    /// (ARCH-P6.3a Q7 item 7).
    pub line7b_countries: String,
    /// The name + SSN header cells (P6.2). REQUIRED: a full-return schedule that does not name its
    /// taxpayer is not a filable form, so a map lacking `[identity]` fails at deserialization.
    pub identity: IdentityCells,
    /// Part I line 1 — the 14 interest-payer rows.
    pub part1_rows: Vec<ScheduleBRowMap>,
    /// L2 — add the amounts on line 1.
    pub line2: MoneyCell,
    /// L4 — line 2 − line 3 → 1040 L2b.
    pub line4: MoneyCell,
    /// Part II line 5 — the 15 dividend-payer rows.
    pub part2_rows: Vec<ScheduleBRowMap>,
    /// L6 — add the amounts on line 5 → 1040 L3b.
    pub line6: MoneyCell,
    /// L7a — the foreign-account Yes/No pair.
    pub line7a: YesNoPair,
    /// L7a's unnumbered FBAR sub-question Yes/No pair. Written ONLY when the answer is `Some` — the
    /// form asks it only under a 7a "Yes".
    pub line7a_fbar: YesNoPair,
    /// L8 — the foreign-trust Yes/No pair.
    pub line8: YesNoPair,
}

impl ScheduleBMap {
    /// Parse the committed TOML.
    pub fn parse(toml_src: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_src)
    }
    /// The TY2024 map.
    pub fn ty2024() -> Self {
        Self::for_year(2024).expect("the bundled TY2024 map is wired and parses")
    }
    /// The map for a supported tax year. Full-return v1 is TY2024-only.
    /// The map for a tax year — design r2 §10 step 3: the file comes from the glob
    /// (`bundled::map_text`), the revision from its ROW, and the ONE exhaustive
    /// `line_set → schema` match (`line_set::schema`) decides whether THIS struct parses it.
    pub fn for_year(year: i32) -> Result<Self, FormsError> {
        let text = crate::bundled::map_text(crate::bundled::Stem::F1040sb, year)
            .ok_or(FormsError::UnsupportedYear(year))?;
        let row = MapRow::read(text).map_err(|e| {
            FormsError::Structure(format!(
                "F1040sb TY{year}: the map's row does not parse: {e}"
            ))
        })?;
        let ls = crate::line_set::LineSet::parse(&row.line_set).ok_or_else(|| {
            FormsError::Structure(format!(
                "F1040sb TY{year}: line_set {:?} is not a revision this build knows",
                row.line_set
            ))
        })?;
        match crate::line_set::schema(ls) {
            crate::line_set::Schema::ScheduleBMap => Self::parse(text).map_err(|e| {
                FormsError::Structure(format!(
                    "F1040sb TY{year}: the bundled map does not parse: {e}"
                ))
            }),
            crate::line_set::Schema::Unwired => Err(FormsError::UnwiredLineSet {
                stem: "F1040sb",
                year,
                line_set: ls.as_str(),
            }),
            other => Err(FormsError::Structure(format!(
                "F1040sb TY{year}: line_set {} parses into {other:?}, not ScheduleBMap",
                ls.as_str()
            ))),
        }
    }
}

/// The Schedule SE (Form 1040) field map for one tax year — the filled §1401 line chain.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScheduleSeMap {
    /// `"schedule_se"`.
    pub form: String,
    /// Tax year.
    pub year: i32,
    // ── design r2 §4 — the ROW. The same keys as [`MapRow`]; required ones REFUSE when missing. ──
    /// IRS basename (`"f1040sd"` for `schedule_d`). See [`MapRow::irs_stem`].
    pub irs_stem: String,
    /// Annual or periodic. See [`MapRow::versioning`].
    pub versioning: Versioning,
    /// sha256 of the bundled PDF. See [`MapRow::template_sha256`].
    pub template_sha256: String,
    /// OPTIONAL — the manifest-join excuse. See [`MapRow::authority`].
    #[serde(default)]
    pub authority: Option<String>,
    /// OPTIONAL — a second extract root. See [`MapRow::extract_override`].
    #[serde(default)]
    pub extract_override: Option<String>,
    /// Instructions stem. See [`MapRow::instructions`].
    pub instructions: String,
    /// OPTIONAL — page range in an `i1040gi` booklet. See [`MapRow::instr_pages`].
    #[serde(default)]
    pub instr_pages: Option<[u32; 2]>,
    /// The line-set revision. See [`MapRow::line_set`].
    pub line_set: String,
    /// OPTIONAL — absent on the 1040. See [`MapRow::attachment_sequence`].
    #[serde(default)]
    pub attachment_sequence: Option<String>,
    /// The §G-13 **field census** — every AcroForm field on this year's PDF that this build does NOT
    /// fill, mapped to the [`CensusDecision`] that leaves it blank. Declared, not tolerated: see
    /// [`CensusDecision`] for why a modelled key is what lets `deny_unknown_fields` be switched on.
    #[serde(default)]
    pub census: std::collections::BTreeMap<String, CensusDecision>,
    /// The identity header — "Name of person **with self-employment income**" + THAT person's SSN, i.e.
    /// the PROPRIETOR, not the return's joint name line. `Option` because this map is shared with the
    /// crypto slice (whose 2017/2025 editions have no verified identity FQNs and write no identity at
    /// all); the FULL-return filler refuses on `None`.
    #[serde(default)]
    pub identity: Option<IdentityCells>,
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
        Self::for_year(2025).expect("the bundled TY2025 map is wired and parses")
    }

    /// The TY2024 map (field-name-identical to 2025; only the wage base differs).
    pub fn ty2024() -> Self {
        Self::for_year(2024).expect("the bundled TY2024 map is wired and parses")
    }

    /// The TY2017 map (OLD §B long form: dollars+cents pairs; pre-filled line 7/14 exempt).
    pub fn ty2017() -> Self {
        Self::for_year(2017).expect("the bundled TY2017 map is wired and parses")
    }

    /// The map for a supported tax year.
    /// The map for a tax year — design r2 §10 step 3: the file comes from the glob
    /// (`bundled::map_text`), the revision from its ROW, and the ONE exhaustive
    /// `line_set → schema` match (`line_set::schema`) decides whether THIS struct parses it.
    pub fn for_year(year: i32) -> Result<Self, FormsError> {
        let text = crate::bundled::map_text(crate::bundled::Stem::ScheduleSe, year)
            .ok_or(FormsError::UnsupportedYear(year))?;
        let row = MapRow::read(text).map_err(|e| {
            FormsError::Structure(format!(
                "ScheduleSe TY{year}: the map's row does not parse: {e}"
            ))
        })?;
        let ls = crate::line_set::LineSet::parse(&row.line_set).ok_or_else(|| {
            FormsError::Structure(format!(
                "ScheduleSe TY{year}: line_set {:?} is not a revision this build knows",
                row.line_set
            ))
        })?;
        match crate::line_set::schema(ls) {
            crate::line_set::Schema::ScheduleSeMap => Self::parse(text).map_err(|e| {
                FormsError::Structure(format!(
                    "ScheduleSe TY{year}: the bundled map does not parse: {e}"
                ))
            }),
            crate::line_set::Schema::Unwired => Err(FormsError::UnwiredLineSet {
                stem: "ScheduleSe",
                year,
                line_set: ls.as_str(),
            }),
            other => Err(FormsError::Structure(format!(
                "ScheduleSe TY{year}: line_set {} parses into {other:?}, not ScheduleSeMap",
                ls.as_str()
            ))),
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
