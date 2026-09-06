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

/// ★ FR-48 / port report §2.5 — the `report --tax-year` sentence for a year that HOLDS full-return
/// inputs but cannot compute them, built from readiness instead of a year literal. It says what is
/// missing, that the inputs are KEPT (a computed carryover written onto a not-yet-ready year is a
/// legitimate row), and names `income clear` only as the way to fall back to a raw tax-profile —
/// with its cost — never as "the" remedy. The previous text said "v1 supports TY2024" and prescribed
/// `income clear` outright, which deletes the filer's W-2s.
pub fn uncomputable_sentence(year: i32) -> String {
    let r = YearReadiness::bundled(year);
    format!(
        "tax year {year} has full-return inputs, but full-return computation is not available for it \
         in this build — {}. The inputs are KEPT and will compute when the year's package is bundled. \
         To fall back to a raw `tax-profile` for {year} instead, run `income clear --year {year}` \
         (this DISCARDS the stored inputs, including any computed carryover on them).",
        r.sentence()
    )
}

/// ★ FR-48 — the note `income import` prints (stderr) when it stores inputs for a year whose full
/// return cannot compute yet. It stores rather than refuses: `report --tax-year N-1 --write-carryover`
/// legitimately writes onto year N before N's package exists, and the TUI keeps the same thing as a
/// draft. `None` when the year is ready (nothing to say).
pub fn import_note(year: i32) -> Option<String> {
    let r = YearReadiness::bundled(year);
    if r.params {
        return None;
    }
    Some(format!(
        "note: {} — these inputs are stored now; `report --tax-year {year}` will refuse (keeping them) \
         until full-return parameters for {year} are bundled.",
        r.sentence()
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
        let s = uncomputable_sentence(2025);
        assert!(s.contains("full-return"), "{s}");
        assert!(s.contains("preparing"), "{s}");
        assert!(s.contains("inputs are KEPT"), "{s}");
        assert!(s.contains("income clear --year 2025"), "{s}");
        assert!(
            !s.contains("v1 supports"),
            "the stale literal must be gone: {s}"
        );
        let s = uncomputable_sentence(2031);
        assert!(s.contains("not bundled"), "{s}");
    }

    /// FR-48: import warns for a not-ready year and says nothing for a ready one.
    #[test]
    fn the_import_note_fires_only_for_a_year_without_params() {
        assert!(import_note(2024).is_none());
        let n = import_note(2025).unwrap();
        assert!(
            n.contains("stored now") && n.contains("report --tax-year 2025"),
            "{n}"
        );
        assert!(import_note(2031).unwrap().contains("not bundled"));
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
