//! ★ Design r2 §6 — **the YEAR RECORD, `forms/<year>/YEAR.toml`: what the year package INTENDS.**
//!
//! Step 1 gave every form a row; this gives every YEAR one. It is a DECLARATION, written by a human
//! and bound by `build.rs` (`bundled::year_record_text`): the intended form set, the absences with
//! their reasons, the due date, the tables' citation of record, the oracles, the price-dataset
//! requirement, and the information-return regime. `YearReadiness` (in `btctax-cli`, which can see
//! tables, params and prices too) compares this declaration against what the build actually
//! carries, so "a form present that was not expected" and "a form expected that is absent" — the
//! third state the port report named — both become visible.
//!
//! What this record does NOT carry, and why (step-4 amendment to the design's §6 example):
//! `TRANSITION_DATE` is the Rev. Proc. 2024-28 per-wallet basis snapshot date (§7.4) — a one-time
//! regulatory fact with ~90 readers in `btctax-core`'s funds-safety logic, not a per-year one; it
//! stays in `conventions`. `return_due` IS declared here; its one code reader
//! (`btctax_core::project::resolve`) cannot see this crate, so `tests/year_record.rs` holds the
//! declaration equal to `conventions::TY2025_RETURN_DUE` until that reader relocates.
use serde::Deserialize;
use std::collections::BTreeMap;

/// `status` — what the year package claims about itself. A declaration; `full_return_for(year)`
/// stays the compute gate, and `YearReadiness` holds the two to each other.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum YearStatus {
    /// Assets are being assembled; nothing about this year may be filed.
    Preparing,
    /// The crypto SLICE only (Form 8949 / Schedule D / Schedule SE / Form 8283 / 1040 cap-gains
    /// figures) — no full return. TY2017.
    Slice,
    /// The full return may be produced: every expected form present, params bundled, oracles read.
    Filable,
}

/// The information-return regime a year is under — what brokers REPORTED to the IRS about the
/// filer's dispositions, which is what routes Form 8949's boxes (Fable plan review C1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Form1099Da {
    /// Brokers report gross proceeds on Form 1099-DA (TY2025+, TD 10000).
    pub proceeds: bool,
    /// Brokers report cost BASIS on Form 1099-DA (TY2026+, TD 10000 — for covered digital assets).
    pub basis: bool,
}

/// `information_returns` — one entry per information-return regime the year is under.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InformationReturns {
    /// Form 1099-DA.
    pub f1099da: Form1099Da,
}

/// `oracles` — which independent engines can witness this year's figures.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Oracles {
    /// OpenTaxSolver release, or why none (`"none"` for TY2017).
    pub ots: String,
    /// Tax-Calculator version floor.
    pub taxcalc: String,
}

/// The record. `deny_unknown_fields`: a mistyped key is a refusal, not a silently-missing fact.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct YearRecord {
    /// The tax year — must equal the directory it lives in.
    pub year: i32,
    /// See [`YearStatus`].
    pub status: YearStatus,
    /// The unextended return due date for this tax year (a TOML date, e.g. `2026-04-15`).
    #[serde(deserialize_with = "toml_date")]
    pub return_due: time::Date,
    /// The forms this year INTENDS to bundle — runbook step 1's output, committed. Must equal the
    /// stems on disk, and together with `forms_absent` must partition `Stem::ALL`.
    pub forms_expected: Vec<String>,
    /// The forms this year deliberately does NOT bundle, each with the reason.
    pub forms_absent: BTreeMap<String, String>,
    /// The citation of record for this year's `TaxTable` and `FullReturnParams`.
    pub tables: String,
    /// See [`Oracles`].
    pub oracles: Oracles,
    /// `BundledPrices::max_date()` must reach this before the year may be `filable`.
    #[serde(deserialize_with = "toml_date")]
    pub prices_through: time::Date,
    /// See [`InformationReturns`].
    pub information_returns: InformationReturns,
}

/// A TOML local date (`2026-04-15`) into a `time::Date`. TOML's date is its own value type, so serde
/// cannot deserialise it into `time::Date` directly; a date with a time part, or a time zone, is
/// refused — a due date is a day.
fn toml_date<'de, D: serde::Deserializer<'de>>(d: D) -> Result<time::Date, D::Error> {
    use serde::de::Error;
    let dt = toml::value::Datetime::deserialize(d)?;
    if dt.time.is_some() || dt.offset.is_some() {
        return Err(D::Error::custom(format!("{dt}: a date, not a datetime")));
    }
    let date = dt
        .date
        .ok_or_else(|| D::Error::custom("a calendar date is required"))?;
    let month = time::Month::try_from(date.month).map_err(D::Error::custom)?;
    time::Date::from_calendar_date(i32::from(date.year), month, date.day).map_err(D::Error::custom)
}

impl YearRecord {
    /// Parse a record's TOML text.
    pub fn parse(toml_src: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_src)
    }

    /// The bundled record for a year, or `None` when this build bundles no such year.
    pub fn for_year(year: i32) -> Option<Self> {
        let text = crate::bundled::year_record_text(year)?;
        Some(
            Self::parse(text)
                .unwrap_or_else(|e| panic!("forms/{year}/YEAR.toml does not parse: {e}")),
        )
    }

    /// The declaration's own consistency: `forms_expected ∪ forms_absent.keys` must be EXACTLY
    /// `Stem::ALL` (every form the crate can fill is either expected or absent-with-reason — the
    /// "third state" is unrepresentable), and the two must not overlap.
    pub fn partition_problems(&self) -> Vec<String> {
        use crate::bundled::Stem;
        let mut out = Vec::new();
        let expected: std::collections::BTreeSet<&str> =
            self.forms_expected.iter().map(String::as_str).collect();
        let absent: std::collections::BTreeSet<&str> =
            self.forms_absent.keys().map(String::as_str).collect();
        for both in expected.intersection(&absent) {
            out.push(format!("{}: {both} is both expected and absent", self.year));
        }
        for s in Stem::ALL {
            let name = s.file_stem();
            if !expected.contains(name) && !absent.contains(name) {
                out.push(format!(
                    "{}: {name} is neither expected nor absent-with-reason — the third state",
                    self.year
                ));
            }
        }
        for name in expected.union(&absent) {
            if Stem::from_file_stem(name).is_none() {
                out.push(format!(
                    "{}: {name} is not a form this crate can fill",
                    self.year
                ));
            }
        }
        out
    }

    /// The declared set against the glob: expected forms with no files, and files no declaration
    /// covers. `present` is what `bundled::BUNDLED` found for this year.
    pub fn glob_problems(&self, present: &[&str]) -> Vec<String> {
        let mut out = Vec::new();
        for e in &self.forms_expected {
            if !present.contains(&e.as_str()) {
                out.push(format!("{}: {e} is expected but not bundled", self.year));
            }
        }
        for p in present {
            if !self.forms_expected.iter().any(|e| e == p) {
                out.push(format!(
                    "{}: {p} is bundled but not expected (and {})",
                    self.year,
                    if self.forms_absent.contains_key(*p) {
                        "declared ABSENT — a contradiction"
                    } else {
                        "not declared absent either"
                    }
                ));
            }
        }
        out
    }
}
