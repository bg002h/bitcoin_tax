//! ★ Design r2 §6 / port report D7 — **`YearReadiness`: what a year DECLARES versus what this build
//! actually CARRIES, as one type.** The forms crate holds the declaration (`YearRecord`) and the
//! glob; the adapters crate holds the tables, the full-return params and the price dataset; only this
//! crate can see all of them — so this is where "is TY2026 ready?" gets one answer, rendered on every
//! number-bearing surface and used to build every refusal string, instead of five literals that drift
//! (`selected_year: 2025`, "2017, 2024 and 2025 only", …).
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
                "TY{} — {} ({} forms; TaxTable {}; full-return params {})",
                self.year,
                match d.status {
                    YearStatus::Preparing => "preparing",
                    YearStatus::Slice => "crypto slice only",
                    YearStatus::Filable => "filable",
                },
                self.forms_bundled,
                if self.table { "yes" } else { "no" },
                if self.params { "yes" } else { "no" },
            ),
        }
    }
}

/// The year a fresh interactive surface opens on: the NEWEST bundled year — derived from the glob,
/// never a literal. (The TUIs edit any bundled year; "newest" is where a new return starts.)
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
        // …and the reverse: params bundled for a year that says `preparing`.
        let mut r = YearReadiness::for_year(2025, &tables, &full, &prices);
        r.params = true;
        assert!(r
            .problems()
            .iter()
            .any(|m| m.contains("declare it filable or unbundle them")));
    }

    #[test]
    fn the_default_year_is_the_newest_bundled_one() {
        assert_eq!(default_year(), 2025);
        assert_eq!(default_year(), *bundled_years().last().unwrap());
    }
}
