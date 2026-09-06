//! ★ Design r2 §6 / port report D7 — **`YearReadiness`: what a year DECLARES versus what this build
//! actually CARRIES, as one type.** The forms crate holds the declaration (`YearRecord`) and the
//! glob; the adapters crate holds the tables, the full-return params and the price dataset; only this
//! crate can see all of them — so this is where "is TY2026 ready?" gets one answer, rendered on every
//! number-bearing surface and used to build every refusal string, instead of five literals that drift
//! (`selected_year: 2025`, "2017, 2024 and 2025 only", …). The default year is the NEWEST bundled
//! record, which since spec 1099-DA T0 is a `preparing` TY2026 with zero forms — deliberately.
//!
//! The compute gate stays `full_return_for(year)`; this type never makes a year computable. What it
//! adds is the declared/actual COMPARISON, with kills: a year declared `filable` whose params are not
//! bundled, or whose price dataset stops before `prices_through`, reds.
use btctax_adapters::price::BundledPrices;
use btctax_core::tax::tables::{FullReturnTables, TaxTables};
use btctax_forms::bundled::{bundled_years, BUNDLED};
use btctax_forms::year_record::{YearRecord, YearStatus};

/// One year, declared versus actual.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YearReadiness {
    /// The tax year asked about.
    pub year: i32,
    /// The declaration, when this build bundles the year at all.
    pub declared: Option<YearRecord>,
    /// A `TaxTable` is bundled (the crypto slice and `report` compute).
    pub table: bool,
    /// `FullReturnParams` are bundled (the full return computes) — the compute gate.
    pub params: bool,
    /// How many forms the build binds for the year (0 = no year directory).
    pub forms_bundled: usize,
    /// The newest close in the bundled price dataset.
    pub prices_max_date: Option<btctax_core::conventions::TaxDate>,
}

impl YearReadiness {
    /// Assemble from the live registries.
    pub fn for_year(
        year: i32,
        tables: &dyn TaxTables,
        full: &dyn FullReturnTables,
        prices: &BundledPrices,
    ) -> Self {
        Self {
            year,
            declared: YearRecord::for_year(year),
            table: tables.table_for(year).is_some(),
            params: full.full_return_for(year).is_some(),
            forms_bundled: BUNDLED.iter().filter(|(_, y)| *y == year).count(),
            prices_max_date: prices.max_date(),
        }
    }

    /// The declared/actual disagreements — empty when the declaration is honest. Each string is a
    /// complete sentence a refusal can print.
    pub fn problems(&self) -> Vec<String> {
        let mut out = Vec::new();
        let Some(d) = &self.declared else {
            if self.forms_bundled > 0 || self.table || self.params {
                out.push(format!(
                    "TY{}: the build carries assets for this year but forms/{}/YEAR.toml declares nothing",
                    self.year, self.year
                ));
            }
            return out;
        };
        match d.status {
            YearStatus::Filable => {
                if !self.params {
                    out.push(format!(
                        "TY{}: declared filable but no FullReturnParams are bundled — the full return cannot compute",
                        self.year
                    ));
                }
                if !self.table {
                    out.push(format!(
                        "TY{}: declared filable but no TaxTable is bundled",
                        self.year
                    ));
                }
                match self.prices_max_date {
                    Some(max) if max >= d.prices_through => {}
                    Some(max) => out.push(format!(
                        "TY{}: declared filable but the bundled price dataset ends {max}, before prices_through {}",
                        self.year, d.prices_through
                    )),
                    None => out.push(format!("TY{}: declared filable with an empty price dataset", self.year)),
                }
            }
            YearStatus::Slice | YearStatus::Preparing => {
                if d.status == YearStatus::Slice && !self.table {
                    out.push(format!(
                        "TY{}: declared a crypto-slice year but no TaxTable is bundled — the slice cannot compute",
                        self.year
                    ));
                }
                if self.params {
                    out.push(format!(
                        "TY{}: FullReturnParams are bundled but the year declares itself {:?} — declare it filable or unbundle them",
                        self.year, d.status
                    ));
                }
            }
        }
        if d.forms_expected.len() != self.forms_bundled {
            out.push(format!(
                "TY{}: declares {} expected forms but the build binds {}",
                self.year,
                d.forms_expected.len(),
                self.forms_bundled
            ));
        }
        out
    }

    /// One line for a status surface: `"TY2025 — preparing (15 forms; TaxTable yes; full-return params no)"`.
    pub fn sentence(&self) -> String {
        match &self.declared {
            None => format!(
                "TY{} — not bundled (this build bundles {})",
                self.year,
                btctax_forms::bundled::years_sentence()
            ),
            Some(d) => format!(
                "TY{} — {} ({} forms; TaxTable {}; full-return params {}; 1099-DA {})",
                self.year,
                match d.status {
                    YearStatus::Preparing => "preparing",
                    YearStatus::Slice => "crypto slice only",
                    YearStatus::Filable => "filable",
                },
                self.forms_bundled,
                if self.table { "yes" } else { "no" },
                if self.params { "yes" } else { "no" },
                // spec 1099-DA T6 — the year's information-return regime, from its record
                match (
                    d.information_returns.f1099da.proceeds,
                    d.information_returns.f1099da.basis,
                ) {
                    (false, _) => "none",
                    (true, false) => "proceeds",
                    (true, true) => "proceeds+basis",
                },
            ),
        }
    }
}

impl YearReadiness {
    /// Assemble from the BUNDLED registries (what this binary ships). The price dataset is optional:
    /// a build whose dataset fails to load still answers the table/params questions honestly, with
    /// `prices_max_date = None`.
    pub fn bundled(year: i32) -> Self {
        use btctax_adapters::tax_tables::{BundledFullReturnTables, BundledTaxTables};
        let tables = BundledTaxTables::load();
        let full = BundledFullReturnTables::load();
        Self {
            year,
            declared: YearRecord::for_year(year),
            table: tables.table_for(year).is_some(),
            params: full.full_return_for(year).is_some(),
            forms_bundled: BUNDLED.iter().filter(|(_, y)| *y == year).count(),
            prices_max_date: BundledPrices::load().ok().and_then(|p| p.max_date()),
        }
    }
}

/// ★★★ spec 1099-DA R6 fold (I-1) — can `export-irs-pdf --tax-year {year}` print the crypto slice
/// AT ALL, template-wise?
///
/// `full_return_for(year).is_none()` is what makes the slice the *right* artifact for a year; it is
/// not what makes the slice *printable*. Printing needs the year's own form templates bundled, which
/// `slice_map_gate` and the `SUPPORTED_YEARS` refusal enforce at export time. **TY2026 is exactly
/// the year where the two diverge** — `forms/2026/` holds only `YEAR.toml`, so no full return
/// computes (the slice clause fired) while `Form8949Map::for_year(2026)` fails (the export refused).
/// The one year the whole feature exists for was being promised an artifact it could not get.
///
/// Form 8949 and Schedule D are the two forms the slice ALWAYS writes (`wants()` defaults to every
/// applicable form), so they are the right two to probe: if either map is missing, nothing prints.
pub fn slice_can_print(year: i32) -> bool {
    btctax_forms::Form8949Map::for_year(year).is_ok()
        && btctax_forms::ScheduleDMap::for_year(year).is_ok()
}

/// ★ spec 1099-DA R6 fold (I-1 + M-4) — the slice clause the two readiness sentences carry, with the
/// predicate the EXPORT actually applies: the answers are stored AND the year's templates are
/// bundled. Empty when either half fails — saying nothing beats promising an artifact that refuses.
fn slice_clause(year: i32, answers_stored: bool) -> String {
    if answers_stored && slice_can_print(year) {
        format!(
            "`export-irs-pdf --tax-year {year}` still prints the crypto slice from the stored answers. "
        )
    } else {
        String::new()
    }
}

/// ★ FR-48 / port report §2.5 — the `report --tax-year` sentence for a year that HOLDS full-return
/// inputs but cannot compute them, built from readiness instead of a year literal. It says what is
/// missing, that the inputs are KEPT (a computed carryover written onto a not-yet-ready year is a
/// legitimate row), and names `income clear` only as the way to fall back to a raw tax-profile —
/// with its cost — never as "the" remedy. The previous text said "v1 supports TY2024" and prescribed
/// `income clear` outright, which deletes the filer's W-2s.
///
/// `answers_stored`: does the year's working `ReturnInputs` carry a non-empty `broker_reporting`?
/// The slice clause is CONDITIONAL on it (M-4) and on the year's templates (I-1) — see
/// [`slice_clause`]. Both terms were missing: the sentence asserted "from the stored answers"
/// unconditionally, on years holding none and on years that cannot print.
pub fn uncomputable_sentence(year: i32, answers_stored: bool) -> String {
    let r = YearReadiness::bundled(year);
    format!(
        "tax year {year} has full-return inputs, but full-return computation is not available for it \
         in this build — {}. The inputs are KEPT and will compute when the year's package is bundled. \
         {}To fall back to a raw `tax-profile` for {year} instead, run `income clear --year {year}` \
         (this DISCARDS the stored inputs, including any computed carryover on them).",
        r.sentence(),
        slice_clause(year, answers_stored)
    )
}

/// ★ FR-48 — the note `income import` prints (stderr) when it stores inputs for a year whose full
/// return cannot compute yet. It stores rather than refuses: `report --tax-year N-1 --write-carryover`
/// legitimately writes onto year N before N's package exists, and the TUI keeps the same thing as a
/// draft. `None` when the year is ready (nothing to say).
///
/// `answers_stored`: does the `ReturnInputs` being imported carry a non-empty `broker_reporting`?
/// Same conditional slice clause as [`uncomputable_sentence`] (R6 fold I-1 + M-4).
pub fn import_note(year: i32, answers_stored: bool) -> Option<String> {
    let r = YearReadiness::bundled(year);
    if r.params {
        return None;
    }
    Some(format!(
        "note: {} — these inputs are stored now; `report --tax-year {year}` computes the crypto delta \
         on a stored `tax-profile` and reports the full return as NOT COMPUTABLE (keeping the inputs) \
         until full-return parameters for {year} are bundled. {}",
        r.sentence(),
        slice_clause(year, answers_stored).trim_end()
    ))
}

/// ★ FR-48 — the `export-snapshot --tax-year` STAMP, written into the export directory so a
/// `form8949.csv` can no longer be byte-identical across years with nothing saying which year it is.
///
/// Deliberately NOT a gate (port report D7 asked for "gate it and stamp it"): `export-snapshot` is a
/// DATA export — lots, disposals, the 8949 rows, the promoted-tranche disclosure — and it is valid
/// for any year the ledger holds events for, including a filed-tranche year like TY2020 that this
/// build bundles no table for (`declare_tranche_cli::filed_tranche_year_exports_clean`). What the
/// stamp adds is the readiness sentence beside the data: whether this build could COMPUTE the year.
pub fn export_stamp(year: i32) -> String {
    YearReadiness::bundled(year).sentence()
}

/// The year a fresh interactive surface opens on: the NEWEST bundled year — derived from the glob,
/// never a literal. (The TUIs edit any bundled year; "newest" is where a new return starts.)
/// ★ The Form 1099-DA regime for a year, JOINED from the year record (spec 1099-DA T0). `None` for a
/// year with no bundled record — the caller refuses, never assumes.
pub fn regime_for(year: i32) -> Option<btctax_core::InformationReturnRegime> {
    let r = YearRecord::for_year(year)?;
    Some(btctax_core::InformationReturnRegime {
        proceeds: r.information_returns.f1099da.proceeds,
        basis: r.information_returns.f1099da.basis,
    })
}

/// [`regime_for`], or the refusal a command gives when the year has no record — the regime is a fact
/// about the year and a command must never guess it (spec 1099-DA R1).
pub fn regime_or_refuse(
    year: i32,
) -> Result<btctax_core::InformationReturnRegime, crate::CliError> {
    // a year with no record is exactly an UNSUPPORTED year — the same typed refusal the export arms
    // raise before any byte, so the two gates cannot disagree about what "unsupported" means
    regime_for(year)
        .ok_or_else(|| crate::CliError::FormFill(btctax_forms::FormsError::UnsupportedYear(year)))
}

/// [`regime_for`], with the ONE case a missing record decides on its own: a year BEFORE the
/// digital-asset box revision (`DIGITAL_ASSET_8949_FIRST_YEAR`) had no Form 1099-DA at all, so its
/// regime is `NONE` whether or not a record exists — the same fact the constant encodes, and what
/// lets `export-snapshot` write a filed-tranche TY2020's CSVs. A later year with no record is the
/// typed `UnsupportedYear` refusal: its regime is unknown and a CSV box must not guess it.
pub fn regime_or_pre_regime(
    year: i32,
) -> Result<btctax_core::InformationReturnRegime, crate::CliError> {
    match regime_for(year) {
        Some(r) => Ok(r),
        None if year < btctax_core::DIGITAL_ASSET_8949_FIRST_YEAR => {
            Ok(btctax_core::InformationReturnRegime::NONE)
        }
        None => Err(crate::CliError::FormFill(
            btctax_forms::FormsError::UnsupportedYear(year),
        )),
    }
}

/// ★ spec 1099-DA R6 (M-5/M-7) — the EXPORT-TIME price-coverage check: does the bundled daily-close
/// dataset actually reach the end of the year being filed?
///
/// [`YearReadiness::problems`] asks the same question, but it is a STATIC readiness report and it
/// asks it only of a year declared `filable`. This is the gate: it runs in BOTH `export-irs-pdf`
/// arms before any byte, on whatever year the filer named, because a packet whose prices stop in
/// June is a return computed from a partial year — a wrong number on a signed form, not a
/// readiness nuance. The readiness surfaces (`sentence()`, the unlock screen, `year_record` tests)
/// gain NOTHING from this: no new `problems()` row, no new sentence clause.
///
/// A year with no bundled record has no `prices_through` to compare, so there is nothing to say —
/// the export's own year gates (`regime_or_refuse`, `SUPPORTED_YEARS`) answer that year.
pub fn price_coverage_or_refuse(year: i32) -> Result<(), crate::CliError> {
    let r = YearReadiness::bundled(year);
    match price_coverage_problem(
        year,
        r.declared.as_ref().map(|d| d.prices_through),
        r.prices_max_date,
    ) {
        Some(msg) => Err(crate::CliError::Usage(msg)),
        None => Ok(()),
    }
}

/// The pure half of [`price_coverage_or_refuse`] — `Some(sentence)` when the dataset stops before
/// the year's declared `prices_through`. Split out so the kill can plant a short dataset without a
/// bundled year that has one.
pub(crate) fn price_coverage_problem(
    year: i32,
    prices_through: Option<btctax_core::conventions::TaxDate>,
    prices_max_date: Option<btctax_core::conventions::TaxDate>,
) -> Option<String> {
    let through = prices_through?;
    let tail = "No forms were written. Update the bundled daily-close dataset (`scripts/` — the \
                price updater appends closes) and re-export.";
    match prices_max_date {
        Some(max) if max >= through => None,
        Some(max) => Some(format!(
            "cannot export TY{year}: the bundled price dataset ends {max}, before TY{year}'s \
             prices_through {through} — every figure on the packet would be computed from a PARTIAL \
             year. {tail}"
        )),
        None => Some(format!(
            "cannot export TY{year}: this build carries an EMPTY price dataset, so no disposition in \
             {year} can be valued. {tail}"
        )),
    }
}

pub fn default_year() -> i32 {
    *bundled_years()
        .iter()
        .max()
        .expect("the build bundles at least one year")
}

#[cfg(test)]
mod tests {
    use super::*;
    use btctax_adapters::tax_tables::{BundledFullReturnTables, BundledTaxTables};

    /// ★★★ spec 1099-DA R6 (M-5/M-7) KILL — the EXPORT-TIME price-coverage gate.
    ///
    /// Two halves, and both are the test. The pure half plants a dataset that stops mid-year, which
    /// no bundled year has; the live half calls the real gate on real years — TY2024 and TY2025 pass
    /// (the bundled dataset reaches into 2026), TY2026 does NOT, because its `prices_through` is
    /// 2026-12-31 and the dataset stops before it. Without the passing half a gate that refused
    /// every year would satisfy the refusal.
    #[test]
    fn the_export_time_price_gate_refuses_a_dataset_that_stops_short() {
        use time::macros::date;
        // planted: the dataset ends in June, the year runs to December.
        let msg = price_coverage_problem(
            2026,
            Some(date!(2026 - 12 - 31)),
            Some(date!(2026 - 06 - 03)),
        )
        .expect("a short dataset must refuse");
        assert!(
            msg.contains("ends 2026-06-03")
                && msg.contains("prices_through 2026-12-31")
                && msg.contains("PARTIAL"),
            "the refusal names both dates and the harm: {msg}"
        );
        // an EMPTY dataset is its own sentence
        assert!(
            price_coverage_problem(2026, Some(date!(2026 - 12 - 31)), None)
                .is_some_and(|m| m.contains("EMPTY price dataset"))
        );
        // covered → silent; a year with no record has no `prices_through` to compare → silent
        assert!(price_coverage_problem(
            2024,
            Some(date!(2024 - 12 - 31)),
            Some(date!(2026 - 06 - 03))
        )
        .is_none());
        assert!(price_coverage_problem(2099, None, Some(date!(2026 - 06 - 03))).is_none());

        // …and the LIVE gate, on the real record + the real bundled dataset.
        price_coverage_or_refuse(2024).expect("TY2024's prices are complete in this build");
        price_coverage_or_refuse(2025).expect("TY2025's prices are complete in this build");
        assert!(
            price_coverage_or_refuse(2026).is_err(),
            "TY2026 runs past the bundled dataset — an export of it may not print"
        );
    }

    /// ★ …and the READINESS SURFACES gain nothing from it (R6: `problems`, `sentence`, the unlock
    /// screen and the `year_record` tests are UNCHANGED). TY2025's dataset is complete, so this
    /// pins the shape rather than the value: no `problems()` row mentions the export-time gate.
    #[test]
    fn the_price_gate_adds_no_readiness_problem() {
        for year in btctax_forms::bundled::bundled_years() {
            let r = YearReadiness::bundled(*year);
            assert!(
                !r.problems().iter().any(|p| p.contains("PARTIAL year")),
                "TY{year}: the export-time gate must not leak into the readiness report"
            );
            assert!(
                !r.sentence().contains("PARTIAL"),
                "TY{year}: {}",
                r.sentence()
            );
        }
    }

    /// ★ THE KILL: every bundled year's declaration agrees with what the build carries. A year
    /// declared `filable` without params, or with a price dataset that stops short, reds here.
    #[test]
    fn every_bundled_years_declaration_agrees_with_the_build() {
        let tables = BundledTaxTables::load();
        let full = BundledFullReturnTables::load();
        let prices = BundledPrices::load().expect("the bundled price dataset loads");
        for &year in bundled_years() {
            let r = YearReadiness::for_year(year, &tables, &full, &prices);
            assert!(
                r.problems().is_empty(),
                "TY{year}: {:#?}\n{}",
                r.problems(),
                r.sentence()
            );
        }
        // and a year the build does not bundle says so, naming the years it does
        let r = YearReadiness::for_year(2031, &tables, &full, &prices);
        assert!(r.declared.is_none() && r.problems().is_empty());
        assert!(r.sentence().contains("not bundled"));
    }

    /// B1 — the declared/actual kill observed red: a `filable` declaration with no params.
    #[test]
    fn a_filable_declaration_without_params_is_reported() {
        let tables = BundledTaxTables::load();
        let full = BundledFullReturnTables::load();
        let prices = BundledPrices::load().expect("the bundled price dataset loads");
        let mut r = YearReadiness::for_year(2024, &tables, &full, &prices);
        assert!(r.problems().is_empty(), "the reference year is clean");
        r.params = false; // plant: the params vanish
        let p = r.problems();
        assert!(
            p.iter()
                .any(|m| m.contains("declared filable but no FullReturnParams")),
            "{p:?}"
        );
        // …and the price-dataset kill: a filable year whose dataset stops before prices_through.
        let mut r = YearReadiness::for_year(2024, &tables, &full, &prices);
        r.prices_max_date = Some(time::macros::date!(2024 - 06 - 01));
        assert!(r
            .problems()
            .iter()
            .any(|m| m.contains("before prices_through")));
        // …a `slice` year with no table (R11)…
        let mut r = YearReadiness::for_year(2017, &tables, &full, &prices);
        assert!(r.problems().is_empty(), "TY2017 has its table");
        r.table = false;
        assert!(r
            .problems()
            .iter()
            .any(|m| m.contains("crypto-slice year but no TaxTable")));
        // …and the reverse: params bundled for a year that says `preparing`.
        let mut r = YearReadiness::for_year(2025, &tables, &full, &prices);
        r.params = true;
        assert!(r
            .problems()
            .iter()
            .any(|m| m.contains("declare it filable or unbundle them")));
    }

    /// FR-48: the report sentence names readiness, keeps the inputs, and names `income clear` only
    /// as the fallback with its cost — no year literal.
    #[test]
    fn the_uncomputable_sentence_is_built_from_readiness_and_keeps_the_inputs() {
        let s = uncomputable_sentence(2025, true);
        assert!(s.contains("full-return"), "{s}");
        assert!(s.contains("preparing"), "{s}");
        assert!(s.contains("inputs are KEPT"), "{s}");
        assert!(s.contains("income clear --year 2025"), "{s}");
        assert!(
            !s.contains("v1 supports"),
            "the stale literal must be gone: {s}"
        );
        let s = uncomputable_sentence(2031, true);
        assert!(s.contains("not bundled"), "{s}");
    }

    /// FR-48: import warns for a not-ready year and says nothing for a ready one.
    #[test]
    fn the_import_note_fires_only_for_a_year_without_params() {
        assert!(import_note(2024, true).is_none());
        let n = import_note(2025, true).unwrap();
        assert!(
            n.contains("stored now") && n.contains("report --tax-year 2025"),
            "{n}"
        );
        assert!(import_note(2031, true).unwrap().contains("not bundled"));
    }

    /// FR-48: export-snapshot stamps the year and its readiness; it does not refuse (a data export
    /// for a filed-tranche year with no table is legitimate).
    #[test]
    fn the_export_stamp_names_the_year_and_its_readiness_for_any_year() {
        assert!(export_stamp(2024).starts_with("TY2024 — filable"));
        assert!(export_stamp(2025).starts_with("TY2025 — preparing"));
        let s = export_stamp(2020);
        assert!(s.starts_with("TY2020 — not bundled"), "{s}");
    }

    /// ★ spec 1099-DA T0 — the join, per bundled year, and the value type's two flags.
    #[test]
    fn the_regime_is_joined_from_the_year_record() {
        use btctax_core::InformationReturnRegime;
        let r = |y| regime_for(y).unwrap();
        assert_eq!(
            r(2017),
            InformationReturnRegime {
                proceeds: false,
                basis: false
            }
        );
        assert_eq!(
            r(2024),
            InformationReturnRegime {
                proceeds: false,
                basis: false
            }
        );
        assert_eq!(
            r(2025),
            InformationReturnRegime {
                proceeds: true,
                basis: false
            }
        );
        assert_eq!(
            r(2026),
            InformationReturnRegime {
                proceeds: true,
                basis: true
            }
        );
        assert!(
            regime_for(2023).is_none(),
            "a year with no record has no regime — refuse, never assume"
        );
    }

    /// A year before the box revision resolves to NONE without a record; a later one without a record
    /// is the typed refusal; a recorded year is its record.
    #[test]
    fn the_pre_regime_resolver_guesses_nothing_after_the_box_revision() {
        use btctax_core::InformationReturnRegime;
        assert_eq!(
            regime_or_pre_regime(2020).unwrap(),
            InformationReturnRegime::NONE
        );
        assert_eq!(
            regime_or_pre_regime(2025).unwrap(),
            InformationReturnRegime::PROCEEDS_ONLY
        );
        assert!(matches!(
            regime_or_pre_regime(2027),
            Err(crate::CliError::FormFill(
                btctax_forms::FormsError::UnsupportedYear(2027)
            ))
        ));
    }

    /// spec 1099-DA T6 — the readiness sentence names the year's Form 1099-DA regime, so the
    /// `report` header and the export stamp say which information-return world the year is in.
    #[test]
    fn the_sentence_names_the_1099da_regime() {
        let s24 = YearReadiness::bundled(2024).sentence();
        let s25 = YearReadiness::bundled(2025).sentence();
        let s26 = YearReadiness::bundled(2026).sentence();
        assert!(s24.contains("1099-DA none)"), "{s24}");
        assert!(s25.contains("1099-DA proceeds)"), "{s25}");
        assert!(s26.contains("1099-DA proceeds+basis)"), "{s26}");
    }

    /// ★ build review r2 NEW-3 — `report`'s prior-year regime join (`regime_or_refuse(year - 1)`)
    /// is unreachable-as-Err only because every year that HAS full-return tables also has a
    /// YEAR.toml record. Pin that, so the `?` cannot start refusing a year the tables serve.
    #[test]
    fn every_year_with_full_return_tables_is_a_bundled_year() {
        let t = BundledFullReturnTables::load();
        let served: Vec<i32> = (2000..=2100)
            .filter(|&y| t.full_return_for(y).is_some())
            .collect();
        assert!(!served.is_empty(), "the tables serve at least one year");
        for y in served {
            assert!(
                bundled_years().contains(&y),
                "TY{y} has full-return tables but no forms/{y}/YEAR.toml record"
            );
        }
    }

    /// ★ spec 1099-DA T0 — the box-revision constant and the record cannot drift: proceeds reporting
    /// begins exactly with the digital-asset box revision, for every bundled year.
    #[test]
    fn the_constant_and_the_regime_agree_on_every_bundled_year() {
        for &y in bundled_years() {
            let r = regime_for(y).expect("every bundled year has a record");
            assert_eq!(
                r.proceeds,
                y >= btctax_core::DIGITAL_ASSET_8949_FIRST_YEAR,
                "TY{y}: YEAR.toml says proceeds={} but DIGITAL_ASSET_8949_FIRST_YEAR says {}",
                r.proceeds,
                y >= btctax_core::DIGITAL_ASSET_8949_FIRST_YEAR
            );
            assert!(
                !r.basis || r.proceeds,
                "TY{y}: basis reporting implies proceeds reporting"
            );
        }
    }

    #[test]
    fn the_default_year_is_the_newest_bundled_one() {
        assert_eq!(default_year(), 2026); // TY2026's record is bundled (preparing) — the year being filed
        assert_eq!(default_year(), *bundled_years().last().unwrap());
    }
}
