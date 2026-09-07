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
    /// figures) — no full return. TY2017 was the one year that declared this, and S9 dropped its
    /// package (2026-09-06), so NO bundled year carries it today — asserted by
    /// `tests/year_record.rs::the_declared_statuses_are_the_measured_ones`. The variant stays: it is
    /// the honest declaration for a future partial year, and `btctax-cli`'s `YearReadiness` still
    /// checks it (a `slice` year with no `TaxTable` is a reported problem).
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
    /// OpenTaxSolver release, or why none (the literal `"none"`).
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

/// **26 U.S.C. §7503** — *"When the last day prescribed under authority of the internal revenue laws
/// for performing any act falls on Saturday, Sunday, or a legal holiday, the performance of such act
/// shall be considered timely if it is performed on the next succeeding day which is not a Saturday,
/// Sunday, or a legal holiday."*
///
/// **Treas. Reg. §301.7503-1(b)** settles WHOSE holidays count, and the answer is the reason this
/// calendar is District of Columbia law rather than the filer's: *"the term 'legal holiday' includes
/// only a legal holiday in the District of Columbia"* for a return filed with the Service in
/// Washington — and §7503's own definition (§7503, last sentence) carries the same rule for every
/// filer, with a State holiday counting only additionally where the return goes to a local office.
/// So DC Emancipation Day moves April 15 for a filer in Anchorage exactly as it does for one in
/// Georgetown.
///
/// **Both halves are modelled here** — the weekend half and the legal-holiday half. Until
/// 2026-09-06 only the weekend half existed, and the omission was recorded rather than hidden: the
/// one caller shifted **June 15**, a date no DC legal holiday can fall on, so the gap could not
/// produce a wrong date. It can now, because the same function derives April dates.
///
/// The holidays, transcribed from **5 U.S.C. §6103** rather than compressed into a closed form:
///
/// - **§6103(a)**, the eleven federal legal public holidays — New Year's Day (January 1); Birthday
///   of Martin Luther King, Jr. (third Monday in January); Washington's Birthday (third Monday in
///   February); Memorial Day (last Monday in May); Juneteenth National Independence Day (June 19);
///   Independence Day (July 4); Labor Day (first Monday in September); Columbus Day (second Monday
///   in October); Veterans Day (November 11); Thanksgiving Day (fourth Thursday in November);
///   Christmas Day (December 25).
/// - **§6103(b)**, the observance rule for the fixed-date ones: a holiday falling on a **Saturday**
///   is observed the preceding **Friday**, one falling on a **Sunday** the following **Monday**.
///   (The floating ones are Mondays or a Thursday by construction and never move.)
/// - **§6103(c)**, **Inauguration Day** — January 20 of each fourth year after 1965, a legal public
///   holiday in the District of Columbia and its neighbouring counties. It gets **no Saturday
///   in-lieu day** (the statute grants one only for Sunday, when the observance moves to January
///   21), so it is modelled exactly that way and not through the §6103(b) rule.
/// - **D.C. Code §1-612.02(a)(10)**, **Emancipation Day** — April 16, the District's own holiday,
///   observed the preceding Friday when it falls on a Saturday and the following Monday when it
///   falls on a Sunday. **This is the holiday that moves April 15**, and the reason TY2017's return
///   was due 2018-04-17.
///
/// The result is the FIRST day on or after `d` that is neither a Saturday, nor a Sunday, nor a DC
/// legal holiday — §7503's "next succeeding day" read literally, applied repeatedly because a
/// holiday can abut a weekend (Christmas 2022 fell on a Sunday, was observed Monday the 26th, and
/// the next available day was Tuesday the 27th).
///
/// ★ Idempotent by construction: its output is a day the predicate does not block, so shifting an
/// already-shifted date returns it unchanged. That is what makes it safe to apply to a date whose
/// provenance is unknown.
pub fn section_7503_shift(d: time::Date) -> time::Date {
    let mut out = d;
    // Bounded because a real calendar can never block a long run (the longest is a Christmas or New
    // Year's Day landing beside a weekend — four days). A run this long means the calendar below is
    // broken, and a broken calendar silently returning a still-blocked due date is the worse
    // outcome: a filer would be told a deadline the IRS does not recognise.
    for _ in 0..30 {
        if !is_weekend(out) && !is_dc_legal_holiday(out) {
            return out;
        }
        out = out.saturating_add(time::Duration::days(1));
    }
    panic!(
        "§7503: no non-holiday weekday within 30 days of {d} — the DC holiday calendar is broken"
    );
}

fn is_weekend(d: time::Date) -> bool {
    matches!(d.weekday(), time::Weekday::Saturday | time::Weekday::Sunday)
}

/// The `n`th `weekday` of `month` in `year` (n = 1 is the first).
fn nth_weekday_of(year: i32, month: time::Month, weekday: time::Weekday, n: i64) -> time::Date {
    let first =
        time::Date::from_calendar_date(year, month, 1).expect("day 1 exists in every month");
    let offset = (i64::from(weekday.number_days_from_sunday())
        - i64::from(first.weekday().number_days_from_sunday()))
    .rem_euclid(7);
    first.saturating_add(time::Duration::days(offset + 7 * (n - 1)))
}

/// The LAST `weekday` of `month` in `year` — walked forward rather than computed from the month
/// length, so no month-length table can be wrong.
fn last_weekday_of(year: i32, month: time::Month, weekday: time::Weekday) -> time::Date {
    let mut d = nth_weekday_of(year, month, weekday, 1);
    loop {
        let next = d.saturating_add(time::Duration::days(7));
        if next.month() != month {
            return d;
        }
        d = next;
    }
}

/// **5 U.S.C. §6103(b)** — a legal public holiday falling on a Saturday is observed the preceding
/// Friday; one falling on a Sunday, the following Monday.
fn observed(d: time::Date) -> time::Date {
    match d.weekday() {
        time::Weekday::Saturday => d.saturating_sub(time::Duration::days(1)),
        time::Weekday::Sunday => d.saturating_add(time::Duration::days(1)),
        _ => d,
    }
}

/// Every day OBSERVED as a District of Columbia legal holiday because of a holiday whose statutory
/// date falls in `year`.
///
/// ★ Returned as observed dates rather than statutory ones because the observance rule can carry a
/// holiday across a year boundary: New Year's Day 2022 fell on a Saturday and was observed **Friday
/// 2021-12-31**. A predicate that only ever consulted its own argument's year would have missed it.
fn dc_legal_holidays_observed_from(year: i32) -> Vec<time::Date> {
    use time::Month::*;
    use time::Weekday::*;
    let fixed = |m: time::Month, day: u8| {
        observed(time::Date::from_calendar_date(year, m, day).expect("a real calendar date"))
    };
    let mut out = vec![
        // ── 5 U.S.C. §6103(a), in the statute's own order ───────────────────────────────────────
        fixed(January, 1),                           // New Year's Day
        nth_weekday_of(year, January, Monday, 3),    // Birthday of Martin Luther King, Jr.
        nth_weekday_of(year, February, Monday, 3),   // Washington's Birthday
        last_weekday_of(year, May, Monday),          // Memorial Day
        fixed(June, 19),                             // Juneteenth National Independence Day
        fixed(July, 4),                              // Independence Day
        nth_weekday_of(year, September, Monday, 1),  // Labor Day
        nth_weekday_of(year, October, Monday, 2),    // Columbus Day
        fixed(November, 11),                         // Veterans Day
        nth_weekday_of(year, November, Thursday, 4), // Thanksgiving Day
        fixed(December, 25),                         // Christmas Day
        // ── D.C. Code §1-612.02(a)(10) — Emancipation Day, the one that moves April 15 ──────────
        fixed(April, 16),
    ];
    // ── 5 U.S.C. §6103(c) — Inauguration Day, "January 20 of each fourth year after 1965" ───────
    // No Saturday in-lieu day: the statute grants an alternate day only for Sunday, when the
    // observance moves to the 21st. So this deliberately does NOT go through `observed`.
    if year > 1965 && (year - 1965) % 4 == 0 {
        let jan20 = time::Date::from_calendar_date(year, January, 20).expect("January 20 exists");
        match jan20.weekday() {
            Saturday => {}
            Sunday => out.push(jan20.saturating_add(time::Duration::days(1))),
            _ => out.push(jan20),
        }
    }
    out
}

/// Is `d` a District of Columbia legal holiday, as **26 U.S.C. §7503** and **Treas. Reg.
/// §301.7503-1(b)** mean it?
///
/// Three statutory years are consulted, not one, because §6103(b) observance moves a holiday off
/// its own year: `d`'s year for the ordinary case, `d.year() - 1` for a New Year's Day observed on
/// the preceding December 31, and `d.year() + 1` for the symmetric case a future calendar could
/// produce.
pub fn is_dc_legal_holiday(d: time::Date) -> bool {
    let y = d.year();
    (y - 1..=y + 1).any(|y| dc_legal_holidays_observed_from(y).contains(&d))
}
