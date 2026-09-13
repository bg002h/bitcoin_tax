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
        // ★ The anti-shrink pin (steps-4/5 review R6): `forms_expected` was COMPUTED from disk and
        //   `glob_problems` compares it against the same glob, so a record regenerated to match a
        //   directory that lost a file would pass both. These counts do not come from the glob.
        let expected_count = match year {
            // TY2017's `5` was removed 2026-09-06 with its form package (owner ruling S9).
            // 2026-09-06: 17 → 19 and 15 → 17 — Form 4868 and Form 1040-V (spec 4868/1040-V T1).
            // 2026-09-07: 19 → 20 and 17 → 18 — Form 8889 (T16 / FR-76).
            2024 => 20,
            2025 => 18,
            2026 => 0, // preparing: the record exists, no TY2026 revision is released (spec 1099-DA T0)
            other => panic!("TY{other}: record the expected form count here — a new year does not arrive silently"),
        };
        assert_eq!(
            r.forms_expected.len(),
            expected_count,
            "TY{year}: the expected set shrank or grew"
        );
    }
    assert_eq!(bundled_years(), &[2024, 2025, 2026]);
}

/// The declared status against what the build bundles: only a year whose declaration is `filable`
/// may say so with every form present.
///
/// ★ `YearStatus::Slice` — a crypto-slice-only package — was TY2017's, and NO bundled year carries
/// it since S9 dropped that package (owner ruling 2026-09-06). The variant stays: it is the honest
/// declaration for any future partial year, and its absence here is asserted rather than left
/// implicit, so a year that quietly declares `slice` reds.
#[test]
fn the_declared_statuses_are_the_measured_ones() {
    let s = |y| YearRecord::for_year(y).unwrap().status;
    assert_eq!(s(2024), YearStatus::Filable);
    assert_eq!(s(2025), YearStatus::Preparing);
    assert!(
        bundled_years()
            .iter()
            .all(|&y| YearRecord::for_year(y).unwrap().status != YearStatus::Slice),
        "no bundled year declares `slice` since the TY2017 package was dropped (S9)"
    );
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
    assert!(!da(2024).proceeds && !da(2024).basis);
    assert!(da(2025).proceeds && !da(2025).basis);
    assert!(da(2026).proceeds && da(2026).basis); // the year the question goes live
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
///
/// ★★ **The plant is DERIVED from the measured glob, and never names a form that happens to be
/// absent today** (FR-136, port-rehearsal F4). Until 2026-09-12 this test planted the literal
/// `"f8995a"` into TY2025's `forms_expected` — a defect only because no `forms/2025/f8995a.*`
/// existed — and pushed the literal `"f1040s1"` onto the present list for the same reason. Both
/// halves were measured to EVAPORATE when those two forms were supplied: `glob_problems` returned
/// `[]` for each, i.e. porting a form disarmed the instrument that watches ports. Worse, a year
/// whose form set is COMPLETE has no absent stem left to name at all, so at that point the plant
/// cannot be re-pointed — only deleted, which is how a B1 instrument dies quietly.
///
/// So every plant below takes its victim from `present_for(year)` — the list the build measured —
/// and plants by REMOVING it from one side of the comparison:
///
/// | plant | the edit | the message it must produce |
/// |---|---|---|
/// | **phantom** | drop the victim from the measured `present` | *expected but not bundled* |
/// | **undeclared** | drop the victim from `forms_expected` | *bundled but not expected … not declared absent either* |
/// | **contradiction** | MOVE the victim from `forms_expected` into `forms_absent` | *bundled but not expected … declared ABSENT — a contradiction* |
///
/// Every stem the year bundles is planted in all three ways, so the plant is not merely derived but
/// exhaustive over the measured set — and a year that gains a form gains three more plants rather
/// than losing one. The old test reached only the contradiction arm (`f1040s1` is declared absent
/// for TY2025); the "not declared absent either" arm was never exercised.
///
/// ★ The case no bundled year can supply — **every form expected AND present**, which is what a
/// January port sequence converges on — is exercised synthetically, because TY2024 still declares
/// `f1040s1a` absent, TY2025 cannot bundle the periodic `f8275`, and TY2026 bundles nothing.
///
/// ★ Blind spot, stated rather than left to be found: these plants mutate a **parsed** `YearRecord`
/// rather than its TOML text, so they say nothing about the text-to-record path. That path is
/// planted by `a_form_dropped_from_the_absent_list_is_the_third_state_and_is_reported` (a text edit
/// that must still parse) and refused by `a_mistyped_record_is_refused`.
#[test]
fn a_phantom_expected_form_and_an_undeclared_bundled_form_are_both_reported() {
    // (what to call it in a failure, the record, the list the build measured)
    let mut cases: Vec<(String, YearRecord, Vec<&'static str>)> = Vec::new();
    for &year in bundled_years() {
        let present = present_for(year);
        if present.is_empty() {
            continue; // a `preparing` year with only a YEAR.toml bundles no stem to plant with
        }
        cases.push((
            format!("TY{year}"),
            YearRecord::for_year(year).unwrap_or_else(|| panic!("TY{year}: no record")),
            present,
        ));
    }
    let real_years = cases.len();

    // The complete year, synthesised: `Stem::ALL` expected and `Stem::ALL` present, nothing absent.
    let mut complete = YearRecord::for_year(2024).expect("TY2024 has a committed record");
    complete.forms_expected = Stem::ALL
        .iter()
        .map(|s| s.file_stem().to_string())
        .collect();
    complete.forms_absent.clear();
    assert!(
        complete.partition_problems().is_empty(),
        "the synthetic complete year must itself be a legitimate declaration: {:#?}",
        complete.partition_problems()
    );
    cases.push((
        "a synthetic COMPLETE year (every form expected AND bundled)".to_string(),
        complete,
        Stem::ALL.iter().map(|s| s.file_stem()).collect(),
    ));

    // A plant the checker failed to report, per case and per victim — collected rather than
    // panicked on, so one run names every case that stopped discriminating instead of the first.
    let mut misses: Vec<String> = Vec::new();
    let mut plants = 0usize;
    let mut want = |label: &str, plant: &str, got: &[String], must_say: &str| {
        plants += 1;
        if got.len() != 1 || !got[0].contains(must_say) {
            misses.push(format!(
                "{label} / {plant}: glob_problems must report exactly {must_say:?}, said {got:#?}"
            ));
        }
    };

    for (label, record, present) in &cases {
        // Unplanted, the pair must be silent — otherwise a reported "problem" below proves nothing.
        let clean = record.glob_problems(present);
        assert!(
            clean.is_empty(),
            "{label}: unplanted, glob_problems must say nothing: {clean:#?}"
        );
        for victim in present {
            // 1. PHANTOM — the file is gone from the glob; the declaration still expects it.
            let shrunk: Vec<&str> = present.iter().copied().filter(|s| s != victim).collect();
            want(
                label,
                "phantom",
                &record.glob_problems(&shrunk),
                &format!("{victim} is expected but not bundled"),
            );

            // 2. UNDECLARED — the file is bundled and the declaration covers it nowhere.
            let mut undeclared = record.clone();
            undeclared.forms_expected.retain(|e| e != victim);
            want(
                label,
                "undeclared",
                &undeclared.glob_problems(present),
                &format!("{victim} is bundled but not expected (and not declared absent either)"),
            );

            // 3. CONTRADICTION — the file is bundled and the declaration says it is ABSENT.
            let mut contradiction = undeclared.clone();
            contradiction
                .forms_absent
                .insert((*victim).to_string(), "planted".to_string());
            want(
                label,
                "contradiction",
                &contradiction.glob_problems(present),
                &format!(
                    "{victim} is bundled but not expected (and declared ABSENT — a contradiction)"
                ),
            );
        }
    }
    assert!(
        misses.is_empty(),
        "{} of {plants} planted defects went unreported:\n{}",
        misses.len(),
        misses.join("\n")
    );
    // Guard the guard: a loop that planted nothing satisfies every assertion inside it. At least one
    // REAL bundled year must have been planted (the synthetic complete year alone would make this
    // test independent of the build), and three plants are owed per bundled stem.
    assert!(
        real_years >= 1,
        "no bundled year supplied a plant — `present_for` found no stems at all"
    );
    let victims: usize = cases.iter().map(|(_, _, present)| present.len()).sum();
    assert_eq!(plants, 3 * victims, "three plants per bundled stem");
}

/// A mistyped status or an unknown key is a parse REFUSAL (deny_unknown_fields), never a default.
#[test]
fn a_mistyped_record_is_refused() {
    let text = btctax_forms::bundled::year_record_text(2024).unwrap();
    assert!(YearRecord::parse(&text.replace("\"filable\"", "\"fileable\"")).is_err());
    assert!(YearRecord::parse(&text.replace("prices_through", "prices_thru")).is_err());
    assert!(YearRecord::parse(&text.replace("[oracles]", "[oracles]\nmoon = \"yes\"")).is_err());
}

// ── §7503, the District of Columbia legal-holiday calendar ───────────────────────────────────────
//
// ★★ Every expected value below is a DERIVED date: the day the shifter has to WALK TO, never the
// day it was handed. A test that fed the shifter its own answer would pass on a calendar that
// returned its argument, which is exactly the calendar this build had until 2026-09-06.

/// ★★ KILL — the six §7503 cases the DC calendar exists for, each one a date the shifter must MOVE
/// TO rather than one it was given.
///
/// - **2018-04-15 → 2018-04-17.** Sunday; the 16th is Emancipation Day (a Monday that year, so
///   observed on its own date); the 17th is the first free day. This is TY2017's real `return_due`,
///   and the next test asserts the derivation equals the committed record.
/// - **2023-04-15 → 2023-04-18.** Saturday; the 16th is a Sunday, so Emancipation Day is observed
///   Monday the 17th; Tuesday the 18th is free. **Two rules compose here** — a weekend and a holiday
///   whose own date was moved by the weekend — which a single-step shifter gets wrong.
/// - **2026-04-15 → 2026-04-15.** A Wednesday with Emancipation Day on Thursday the 16th: the
///   control. Without it a shifter that moved every April date would pass the two above.
/// - **2024-06-15 → 2024-06-17.** The weekend half still works (Saturday → Monday), and Juneteenth
///   on the 19th does not reach back.
/// - **2025-07-04 → 2025-07-07.** A holiday on a plain weekday, then straight into a weekend.
/// - **2022-12-25 → 2022-12-27.** Christmas on a Sunday, observed Monday the 26th, so the first
///   free day is Tuesday the 27th — the abutting case that forces the shift to iterate.
#[test]
fn the_section_7503_shifter_walks_past_weekends_and_dc_legal_holidays() {
    use btctax_forms::year_record::section_7503_shift;
    use time::macros::date;
    for (from, to, why) in [
        (
            date!(2018 - 04 - 15),
            date!(2018 - 04 - 17),
            "Sun → Emancipation Day Mon 16 → Tue 17",
        ),
        (
            date!(2023 - 04 - 15),
            date!(2023 - 04 - 18),
            "Sat → Sun → Emancipation Day observed Mon 17 → Tue 18",
        ),
        (
            date!(2026 - 04 - 15),
            date!(2026 - 04 - 15),
            "Wed, Emancipation Day is Thu 16 — unchanged",
        ),
        (date!(2024 - 06 - 15), date!(2024 - 06 - 17), "Sat → Mon 17"),
        (
            date!(2025 - 07 - 04),
            date!(2025 - 07 - 07),
            "Fri Independence Day → weekend → Mon 7",
        ),
        (
            date!(2022 - 12 - 25),
            date!(2022 - 12 - 27),
            "Sun Christmas, observed Mon 26 → Tue 27",
        ),
    ] {
        assert_eq!(section_7503_shift(from), to, "§7503({from}): {why}");
    }
}

/// ★★ KILL — **the derivation matches the committed record.** TY2017's `return_due` is 2018-04-17,
/// a date typed into `YEAR.toml` by a human years ago and, until now, never derivable. The shifter
/// must reproduce it from April 15 alone.
///
/// ★ The record is CONSTRUCTED from the committed TY2024 text (S9 deleted `forms/2017/YEAR.toml` on
/// 2026-09-06), which is exactly what makes this a real check and not a tautology: the expected
/// value is parsed out of a `YearRecord`, and the actual value is walked to by the calendar. Neither
/// side is a literal the other was written from.
#[test]
fn ty2017s_committed_return_due_is_derived_by_the_dc_holiday_calendar() {
    use btctax_forms::year_record::{section_7503_shift, YearRecord};
    use time::macros::date;
    let text = btctax_forms::bundled::year_record_text(2024)
        .expect("TY2024 has a committed year record")
        .replace("year           = 2024", "year           = 2017")
        .replace("return_due     = 2025-04-15", "return_due     = 2018-04-17");
    let committed = YearRecord::parse(&text)
        .expect("the TY2017-shaped record parses")
        .return_due;
    assert_eq!(
        section_7503_shift(date!(2018 - 04 - 15)),
        committed,
        "the shifter must DERIVE the committed 2018-04-17 from the statutory April 15"
    );

    // …and the same derivation on the years whose records ARE bundled leaves their April 15 alone,
    // so the shifter cannot be one that simply moves every April date to the 17th.
    for year in btctax_forms::bundled::bundled_years() {
        let r = YearRecord::for_year(*year).unwrap_or_else(|| panic!("TY{year}: no record"));
        assert_eq!(
            section_7503_shift(r.return_due),
            r.return_due,
            "TY{year}: a committed return_due is already a §7503 fixed point"
        );
    }
}

/// ★ KILL — the holiday PREDICATE itself, in both directions, on the cases a compressed calendar
/// gets wrong: the four floating Mondays and the Thursday, the §6103(b) in-lieu days on both sides,
/// the New Year's Day that is observed in the PREVIOUS year, and Inauguration Day's missing
/// Saturday rule. Each true case is paired with a false one a day away, so a predicate that
/// answered `true` everywhere fails.
#[test]
fn the_dc_legal_holiday_predicate_names_the_right_days() {
    use btctax_forms::year_record::is_dc_legal_holiday;
    use time::macros::date;
    for (d, want, why) in [
        (
            date!(2025 - 01 - 20),
            true,
            "MLK Jr., third Monday in January — AND Inauguration Day",
        ),
        (
            date!(2025 - 01 - 13),
            false,
            "the second Monday in January is not MLK",
        ),
        (
            date!(2025 - 02 - 17),
            true,
            "Washington's Birthday, third Monday in February",
        ),
        (
            date!(2025 - 02 - 24),
            false,
            "the fourth Monday in February is nothing",
        ),
        (
            date!(2025 - 05 - 26),
            true,
            "Memorial Day, LAST Monday in May",
        ),
        (
            date!(2025 - 05 - 19),
            false,
            "the third Monday in May is not Memorial Day",
        ),
        (
            date!(2025 - 09 - 01),
            true,
            "Labor Day, first Monday in September",
        ),
        (
            date!(2025 - 10 - 13),
            true,
            "Columbus Day, second Monday in October",
        ),
        (
            date!(2025 - 11 - 27),
            true,
            "Thanksgiving, FOURTH Thursday in November",
        ),
        (
            date!(2025 - 11 - 20),
            false,
            "the third Thursday in November is not Thanksgiving",
        ),
        // §6103(b), both directions
        (
            date!(2021 - 07 - 05),
            true,
            "July 4 2021 was a Sunday → observed Monday the 5th",
        ),
        (
            date!(2021 - 12 - 24),
            true,
            "Christmas 2021 was a Saturday → observed Friday the 24th",
        ),
        (
            date!(2021 - 12 - 25),
            false,
            "…and the Saturday itself is not the observed day",
        ),
        // the year-boundary case the predicate consults three statutory years for
        (
            date!(2021 - 12 - 31),
            true,
            "New Year's Day 2022 was a Saturday → observed Friday 2021-12-31",
        ),
        // §6103(c), Inauguration Day — a fourth year after 1965, and NO Saturday in-lieu day
        (
            date!(2021 - 01 - 20),
            true,
            "Inauguration Day 2021 (a Wednesday)",
        ),
        (
            date!(2024 - 01 - 20),
            false,
            "2024 is not a fourth year after 1965",
        ),
        (
            date!(2029 - 01 - 19),
            false,
            "Jan 20 2029 is a Saturday — §6103(c) grants no in-lieu Friday",
        ),
        (
            date!(2029 - 01 - 20),
            false,
            "…and the Saturday itself is a holiday for no other reason",
        ),
        (
            date!(2029 - 01 - 22),
            false,
            "…nor is the Monday after it (MLK Jr. 2029 is the 15th)",
        ),
        (
            date!(2013 - 01 - 21),
            true,
            "Jan 20 2013 was a Sunday → §6103(c)'s own alternate day, the 21st",
        ),
        // Emancipation Day, the whole point
        (
            date!(2018 - 04 - 16),
            true,
            "Emancipation Day 2018, a Monday",
        ),
        (
            date!(2023 - 04 - 17),
            true,
            "Emancipation Day 2023 observed (the 16th was a Sunday)",
        ),
        (
            date!(2026 - 04 - 15),
            false,
            "April 15 2026 is an ordinary Wednesday",
        ),
    ] {
        assert_eq!(is_dc_legal_holiday(d), want, "{d}: {why}");
    }
}

/// ★★ KILL — **the shifter terminates and its answer is the FIRST free day**, over 1960-01-01 to
/// 2099-12-31 (51,134 days). This is what makes the 30-day bound in `section_7503_shift`
/// unreachable rather than merely generous, and it is the guard against the whole class the
/// per-date table above cannot reach: a calendar that blocks a day it should not.
#[test]
fn the_shifter_lands_on_the_first_free_day_for_every_date_in_140_years() {
    use btctax_forms::year_record::{is_dc_legal_holiday, section_7503_shift};
    use time::macros::date;
    let blocked = |d: time::Date| {
        matches!(d.weekday(), time::Weekday::Saturday | time::Weekday::Sunday)
            || is_dc_legal_holiday(d)
    };
    let (mut d, end) = (date!(1960 - 01 - 01), date!(2099 - 12 - 31));
    let (mut days, mut moved) = (0u32, 0u32);
    while d <= end {
        let got = section_7503_shift(d);
        assert!(got >= d, "{d}: §7503 may only move FORWARD, got {got}");
        assert!(
            !blocked(got),
            "{d}: §7503 landed on {got}, which is itself blocked"
        );
        let mut walk = d;
        while blocked(walk) {
            walk = walk.saturating_add(time::Duration::days(1));
        }
        assert_eq!(
            got, walk,
            "{d}: §7503 must land on the FIRST free day, not {got}"
        );
        assert_eq!(
            section_7503_shift(got),
            got,
            "{d}: §7503 must be idempotent at {got}"
        );
        if got != d {
            moved += 1;
        }
        days += 1;
        d = d.saturating_add(time::Duration::days(1));
    }
    // Guard the guard: a loop that walked nothing, or a shifter that moved nothing (or everything),
    // would pass every assertion above by never discriminating.
    assert_eq!(days, 51_135, "the sweep must cover every day in the span");
    assert!(
        (14_000..24_000).contains(&moved),
        "{moved} of {days} days moved — roughly two weekend days in seven plus the holidays is the \
         only plausible figure; a shifter that moved nothing or everything is broken"
    );
}
