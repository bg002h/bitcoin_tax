//! ★★ Design r2 §10 step 4 — **the YEAR RECORD, held to the build.** Every `forms/<year>/YEAR.toml`
//! is a declaration; these tests hold it to the glob (`bundled::BUNDLED`), to the closed set of forms
//! (`Stem::ALL`), and — for the one date `btctax-core` still carries as a constant — to that constant.
//! Each kill was observed red on a planted copy (B1).
use btctax_forms::bundled::{bundled_years, Stem, BUNDLED};
use btctax_forms::year_record::{YearRecord, YearStatus};

fn present_for(year: i32) -> Vec<&'static str> {
    BUNDLED
        .iter()
        .filter(|(_, y)| *y == year)
        .map(|(s, _)| s.file_stem())
        .collect()
}

#[test]
fn every_bundled_year_has_a_record_that_partitions_the_closed_set_and_matches_the_glob() {
    for &year in bundled_years() {
        let r = YearRecord::for_year(year).unwrap_or_else(|| panic!("TY{year}: no record"));
        assert_eq!(
            r.year, year,
            "TY{year}: the record's `year` must be its directory"
        );
        assert!(
            r.partition_problems().is_empty(),
            "TY{year}: {:#?}",
            r.partition_problems()
        );
        let present = present_for(year);
        assert!(
            r.glob_problems(&present).is_empty(),
            "TY{year}: {:#?}",
            r.glob_problems(&present)
        );
        assert_eq!(
            r.forms_expected.len() + r.forms_absent.len(),
            Stem::ALL.len(),
            "TY{year}: expected + absent must be every form the crate can fill"
        );
    }
    assert_eq!(bundled_years(), &[2017, 2024, 2025]);
}

/// The declared status against what the build bundles: only a year whose declaration is `filable`
/// may say so with every form present; `slice` is TY2017's crypto-slice-only package.
#[test]
fn the_declared_statuses_are_the_measured_ones() {
    let s = |y| YearRecord::for_year(y).unwrap().status;
    assert_eq!(s(2017), YearStatus::Slice);
    assert_eq!(s(2024), YearStatus::Filable);
    assert_eq!(s(2025), YearStatus::Preparing);
    // A `filable` year has NO absent forms except a periodic one served by hash.
    let r = YearRecord::for_year(2024).unwrap();
    for absent in r.forms_absent.keys() {
        assert_eq!(
            absent, "f1040s1a",
            "TY2024 is the complete reference year: only the TY2025+ schedule may be absent"
        );
    }
}

/// `return_due` is declared here AND read by `btctax-core` as a constant (`resolve.rs`, one reader)
/// — two truths held equal until the reader relocates (step-4 design amendment).
#[test]
fn the_declared_ty2025_due_date_is_the_core_constant() {
    let r = YearRecord::for_year(2025).unwrap();
    assert_eq!(r.return_due, btctax_core::conventions::TY2025_RETURN_DUE);
    // Every year's due date is in the year AFTER the tax year.
    for &year in bundled_years() {
        let r = YearRecord::for_year(year).unwrap();
        assert_eq!(r.return_due.year(), year + 1, "TY{year}");
        assert_eq!(
            r.prices_through.year(),
            year,
            "TY{year}: the price dataset covers the filed year"
        );
    }
}

/// The 1099-DA regime, as declared: proceeds reporting begins TY2025, basis TY2026 (TD 10000).
#[test]
fn the_information_return_regime_is_declared_per_year() {
    let da = |y| YearRecord::for_year(y).unwrap().information_returns.f1099da;
    assert!(!da(2017).proceeds && !da(2017).basis);
    assert!(!da(2024).proceeds && !da(2024).basis);
    assert!(da(2025).proceeds && !da(2025).basis);
}

fn plant(year: i32, edit: impl Fn(String) -> String) -> YearRecord {
    let text = btctax_forms::bundled::year_record_text(year)
        .unwrap()
        .to_string();
    YearRecord::parse(&edit(text)).expect("the planted record still parses")
}

/// B1 — the partition kill observed red: a form neither expected nor absent.
#[test]
fn a_form_dropped_from_the_absent_list_is_the_third_state_and_is_reported() {
    let r = plant(2024, |t| t.replace("f1040s1a", "f1040s1a_gone"));
    let p = r.partition_problems();
    assert!(
        p.iter()
            .any(|m| m.contains("f1040s1a is neither expected nor absent")),
        "{p:?}"
    );
    assert!(
        p.iter()
            .any(|m| m.contains("f1040s1a_gone is not a form this crate can fill")),
        "{p:?}"
    );
}

/// B1 — the glob kill observed red in both directions: an expected form with no file, and a bundled
/// file no declaration covers.
#[test]
fn a_phantom_expected_form_and_an_undeclared_bundled_form_are_both_reported() {
    let r = plant(2025, |t| {
        t.replace(
            "forms_expected = [\n",
            "forms_expected = [\n    \"f8995a\",\n",
        )
    });
    let mut present = present_for(2025);
    let p = r.glob_problems(&present);
    assert!(
        p.iter()
            .any(|m| m.contains("f8995a is expected but not bundled")),
        "{p:?}"
    );
    present.push("f1040s1"); // a file the build found that the record does not expect
    let p = r.glob_problems(&present);
    assert!(
        p.iter()
            .any(|m| m.contains("f1040s1 is bundled but not expected")),
        "{p:?}"
    );
}

/// A mistyped status or an unknown key is a parse REFUSAL (deny_unknown_fields), never a default.
#[test]
fn a_mistyped_record_is_refused() {
    let text = btctax_forms::bundled::year_record_text(2024).unwrap();
    assert!(YearRecord::parse(&text.replace("\"filable\"", "\"fileable\"")).is_err());
    assert!(YearRecord::parse(&text.replace("prices_through", "prices_thru")).is_err());
    assert!(YearRecord::parse(&text.replace("[oracles]", "[oracles]\nmoon = \"yes\"")).is_err());
}
