//! ★★★ **T11 / SPEC_interview.md R13 — the oracle path's KATs.**
//!
//! Four guarantees, and every one of them is DERIVED rather than declared:
//!
//! 1. **The projection inverse.** `project_to_golden(build_golden_return(g)) == g.canonical()` over
//!    every committed golden household AND over a household with two dependents — so the dependents
//!    block round-trips through ages, blindness and the CTC/ODC edges rather than being carried in
//!    name only.
//! 2. **`ORACLE_INVISIBLE` is complete.** Every `Usd` leaf of `ReturnInputs` — enumerated by TYPE
//!    with `leaf_walk::money_leaves`, never by a hand-list — either moves the projection or is named
//!    in the table with a reason. A money field added tomorrow with neither reds here.
//! 3. **The row carries no identity**, structurally: a serialized `GoldenInputs` contains no string
//!    that came from the return's PII.
//! 4. **The ROUTING partition** (T11 fold). Guarantee 2 watches `Usd` leaves, so anything that is not
//!    a `Usd` leaf is invisible to it — and two Criticals of the T11 seam review were exactly that:
//!    the itemize election and a charitable gift's §170 class both decide where money GOES without
//!    being money. So every **non-`Usd`** leaf is perturbed to every alternative its TYPE admits, and
//!    the household the row describes must go on reproducing the return the filer files.

use btctax_core::conventions::Usd;
use btctax_core::state::LedgerState;
use btctax_core::tax::provenance::leaf_walk::{money_leaves, set_at, walk};
use btctax_core::tax::return_1040::{assemble_absolute, AbsoluteReturn};
use btctax_core::tax::return_inputs::ReturnInputs;
use btctax_core::tax::testonly::{
    build_golden_return, every_money_leaf_household, golden_filing_status_token, golden_households,
    normalize_leaf_path, oracle_invisible_entry, project_to_golden, ty2024_params, ty2024_table,
    GoldenCreditColumn, GoldenDependent, GoldenInputs, GOLDEN_FILING_STATUSES, ORACLE_INVISIBLE,
};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// btctax's own assembled return for `ri` — what [`project_to_golden`] reads its filed Schedule A
/// and its 1040 line-12 decision off, so the projection re-derives neither.
fn assemble(ri: &ReturnInputs, state: &LedgerState) -> AbsoluteReturn {
    assemble_absolute(ri, state, &ty2024_params(), &ty2024_table(), 2024)
}

/// The projection, with the assembled return supplied — every call site in this file goes through it.
fn project(ri: &ReturnInputs, state: &LedgerState) -> GoldenInputs {
    project_to_golden(ri, state, &assemble(ri, state))
}

// ─────────────────────────────────────────────────────────────────────────────────────────────────
// 1. The projection inverse
// ─────────────────────────────────────────────────────────────────────────────────────────────────

#[test]
fn the_projection_inverts_build_golden_return_on_every_committed_household() {
    let households = golden_households();
    assert!(
        households.len() > 100,
        "the corpus should still be the committed 107-cell matrix, not a stub; got {}",
        households.len()
    );
    for h in &households {
        let (ri, state) = build_golden_return(&h.inputs);
        assert_eq!(
            project(&ri, &state),
            h.inputs.canonical(),
            "the projection is not the inverse of `build_golden_return` on {}",
            h.name
        );
    }
}

/// The canonicalisation is a statement about `build_golden_return`'s own many-to-one collapse, not a
/// fudge that lets the inverse pass: a row and its canonical form must build the IDENTICAL return.
#[test]
fn canonicalising_a_golden_row_changes_no_return_it_builds() {
    for h in golden_households() {
        let (a, sa) = build_golden_return(&h.inputs);
        let (b, sb) = build_golden_return(&h.inputs.canonical());
        assert_eq!(
            a, b,
            "canonicalising {} changed the return it builds",
            h.name
        );
        assert_eq!(sa.disposals.len(), sb.disposals.len());
        assert_eq!(
            h.inputs.canonical(),
            h.inputs.canonical().canonical(),
            "canonical() is not idempotent on {}",
            h.name
        );
    }
}

/// A household with TWO dependents — one on the child-tax-credit edge and one qualifying relative —
/// plus an aged, blind taxpayer and a blind spouse. Everything R13 says the block must carry.
fn two_dependent_household() -> GoldenInputs {
    GoldenInputs {
        filing_status: "Married/Joint".into(),
        w2_income: 90_000.0,
        dependents: vec![
            GoldenDependent {
                age: 9,
                credit: GoldenCreditColumn::ChildTaxCredit,
                eic_qualifying_child: true,
            },
            GoldenDependent {
                age: 30,
                credit: GoldenCreditColumn::CreditForOtherDependents,
                eic_qualifying_child: false,
            },
        ],
        age_head: Some(66),
        age_spouse: Some(64),
        blind_head: true,
        blind_spouse: true,
        ..Default::default()
    }
}

#[test]
fn the_projection_inverts_a_household_with_two_dependents() {
    let g = two_dependent_household();
    let (ri, state) = build_golden_return(&g);
    // The fixture really did reach the edges it names — otherwise the round trip below would be
    // comparing two descriptions of an empty grid (FR-88).
    assert_eq!(ri.header.dependents.len(), 2);
    assert_eq!(g.n24(), 1, "one child on the CTC edge");
    assert_eq!(g.xtot(), 4, "filer + spouse + two dependents");
    assert_eq!(g.eic_qualifying_children(), 1);
    assert_eq!(g.nu18(), 1);
    assert_eq!(
        g.n21(),
        3,
        "the 30-year-old dependent, the filer and the spouse"
    );
    assert_eq!(g.n1820(), 0);
    assert_eq!(
        project(&ri, &state),
        g.canonical(),
        "the dependents block does not round-trip"
    );
}

/// ★ **The kill for the block itself.** Deleting the dependents from the projected row must RED the
///   inverse — otherwise "the projection carries the dependents block" is a claim with no test.
#[test]
fn deleting_the_dependents_block_from_the_projection_reds_the_inverse() {
    let g = two_dependent_household();
    let (ri, state) = build_golden_return(&g);
    let full = project(&ri, &state);
    assert_eq!(full, g.canonical());

    let mut gutted = full.clone();
    gutted.dependents.clear();
    assert_ne!(gutted, g.canonical(), "dropping the rows must be visible");
    let mut no_ages = full.clone();
    no_ages.age_head = None;
    no_ages.age_spouse = None;
    assert_ne!(no_ages, g.canonical(), "dropping the ages must be visible");
    let mut sighted = full;
    sighted.blind_head = false;
    sighted.blind_spouse = false;
    assert_ne!(sighted, g.canonical(), "dropping blindness must be visible");
}

// ─────────────────────────────────────────────────────────────────────────────────────────────────
// 2. ORACLE_INVISIBLE is complete — the DERIVED partition
// ─────────────────────────────────────────────────────────────────────────────────────────────────

/// The fixtures the partition is measured over.
///
/// ★★ **Two of them, and the second one is the point.** Schedule A line 5a is an ELECTION
///    (§164(b)(5)): with `salt_use_sales_tax == Some(true)` the sales-tax amount is the figure that
///    projects and the income-tax boxes are dead, and with anything else it is the other way round.
///    A partition measured on one setting would call three live boxes invisible. Any future election
///    that ROUTES money belongs here too.
fn probe_fixtures() -> Vec<ReturnInputs> {
    let (base, _) = every_money_leaf_household();
    let mut sales_tax = base.clone();
    sales_tax
        .schedule_a
        .as_mut()
        .expect("the maximal sentinel carries a Schedule A")
        .salt_use_sales_tax = Some(true);
    vec![base, sales_tax]
}

/// Every money leaf whose FIGURE moves the projection on at least one probe fixture — measured by
/// perturbing the leaf and comparing the projected rows, never by reading a list.
///
/// ★ The comparison is over [`GoldenInputs::money_view`], i.e. with the ROUTING facts normalised
///   away. Perturbing `schedule_a.medical` to $777,777 flips 1040 line 12 to the itemized branch and
///   so moves `standard_or_itemized` — but not one dollar of medical reaches an oracle box, and
///   calling it "visible" here would make its `ORACLE_INVISIBLE` entry read as a stale exemption.
///   The routing dimension has its own partition below, which asks the right question about it.
fn visible_leaves() -> BTreeSet<String> {
    let state = LedgerState::default();
    let mut visible = BTreeSet::new();
    for base in probe_fixtures() {
        let baseline = project(&base, &state).money_view();
        let doc = serde_json::to_value(&base).expect("ReturnInputs serializes");
        for path in money_leaves(&base) {
            let mut v = doc.clone();
            // A probe distinct from every amount `every_money_leaf_household` writes
            // (`1000 + 137 * i`, so under $25,000 for the whole set).
            assert!(
                set_at(&mut v, &path, serde_json::Value::String("777777".into())),
                "money leaf {path} is walkable but not settable"
            );
            let ri: ReturnInputs =
                serde_json::from_value(v).expect("a money leaf takes a decimal string");
            if project(&ri, &state).money_view() != baseline {
                visible.insert(normalize_leaf_path(&path));
            }
        }
    }
    visible
}

#[test]
fn every_money_leaf_either_projects_or_is_named_in_oracle_invisible() {
    let visible = visible_leaves();
    assert!(
        visible.len() > 20,
        "the probe found only {} visible leaves — it has stopped measuring anything",
        visible.len()
    );
    let (base, _) = every_money_leaf_household();
    let mut unaccounted: Vec<String> = Vec::new();
    let mut wrongly_listed: Vec<String> = Vec::new();
    for path in money_leaves(&base) {
        let norm = normalize_leaf_path(&path);
        let listed = oracle_invisible_entry(&path).is_some();
        match (visible.contains(&norm), listed) {
            (false, false) => unaccounted.push(norm),
            (true, true) => wrongly_listed.push(norm),
            _ => {}
        }
    }
    unaccounted.sort();
    unaccounted.dedup();
    wrongly_listed.sort();
    wrongly_listed.dedup();
    assert!(
        unaccounted.is_empty(),
        "these `Usd` leaves neither reach the oracle row nor appear in ORACLE_INVISIBLE. Either \
         carry the figure in `project_to_golden` or add an entry saying why no oracle can take \
         it:\n  {}",
        unaccounted.join("\n  ")
    );
    assert!(
        wrongly_listed.is_empty(),
        "these leaves DO move the projection and are still listed as invisible — a stale exemption \
         hides a real figure:\n  {}",
        wrongly_listed.join("\n  ")
    );
}

/// The stale-exemption half: every entry must claim at least one real money leaf.
/// ★ T11 fold — over EVERY leaf, not only the money ones: the table now also names non-`Usd` facts
///   that route money, and a stale-entry check that walked money leaves alone would report the
///   routing entries as stale (and, worse, would never notice a routing entry that went stale).
#[test]
fn no_oracle_invisible_entry_is_stale() {
    let (base, _) = every_money_leaf_household();
    let doc = serde_json::to_value(&base).expect("ReturnInputs serializes");
    let mut leaves: Vec<String> = Vec::new();
    walk(&doc, "", &mut leaves);
    let mut stale = Vec::new();
    for e in ORACLE_INVISIBLE {
        if !leaves
            .iter()
            .any(|p| oracle_invisible_entry(p).is_some_and(|f| f.prefix == e.prefix))
        {
            stale.push(e.prefix);
        }
    }
    assert!(
        stale.is_empty(),
        "these ORACLE_INVISIBLE entries claim no leaf at all — the field they named is gone: \
         {stale:?}"
    );
}

/// `unprojected_nonzero_leaves` is what stops a truncated household reading as a btctax defect, so
/// it must actually report one.
#[test]
fn a_nonzero_unprojected_leaf_is_reported() {
    let (base, _) = every_money_leaf_household();
    let reported = btctax_core::tax::testonly::unprojected_nonzero_leaves(&base);
    let names: BTreeSet<String> = reported.iter().map(|(p, _, _)| p.clone()).collect();
    assert!(
        names.contains("schedule_a.medical"),
        "the maximal fixture carries medical expenses the oracle row cannot take, and the census \
         did not name them: {names:?}"
    );
    for (path, amount, entry) in &reported {
        assert!(
            *amount != Usd::ZERO,
            "{path} is zero and was still reported"
        );
        assert!(!entry.note.is_empty(), "{path} has an empty reason");
    }
    // ★ And on a corpus household it is EXACT, not merely non-empty. The wage-only cell carries
    //   exactly two figures the oracle row folds away — `build_golden_return` sets box 3 and box 5
    //   equal to box 1, and the row has one wage field — so the census must name those two and
    //   nothing else. An assertion of "non-empty" here would pass on a census that reported
    //   everything.
    let (clean, state) = build_golden_return(&golden_households()[0].inputs);
    let _ = project(&clean, &state);
    let clean_names: BTreeSet<String> =
        btctax_core::tax::testonly::unprojected_nonzero_leaves(&clean)
            .into_iter()
            .map(|(p, _, _)| p)
            .collect();
    assert_eq!(
        clean_names,
        ["w2s[0].box3_ss_wages", "w2s[0].box5_medicare_wages"]
            .into_iter()
            .map(String::from)
            .collect::<BTreeSet<String>>(),
        "the wage-only golden household's unprojected census is not what the fixture carries"
    );
}

/// ★★★ **The LEDGER half of the census.** `ORACLE_INVISIBLE` walks `ReturnInputs`, and the ledger is
/// not one of its leaves — so the crypto side of a btctax return, the reason the tool exists, would
/// be the one part of the projection with no completeness check at all. Two ledger figures reach the
/// printed return and cannot reach the oracle row; both must be reported.
///
/// ★ It is the FR-88 shape stated as a test: a guard whose fixture never reaches the case it exists
///   to catch is green forever. So this plants a real income record and a real donation.
#[test]
fn the_ledger_lines_the_oracle_row_cannot_carry_are_reported() {
    use btctax_core::event::IncomeKind;
    use btctax_core::identity::EventId;
    use btctax_core::state::IncomeRecord;
    use btctax_core::tax::return_1040::unprojected_ledger_lines;
    use time::macros::date;

    let mut state = LedgerState::default();
    assert!(
        unprojected_ledger_lines(&state, 2024).is_empty(),
        "an empty ledger contributes nothing the row cannot carry"
    );

    // Hobby staking rewards — Schedule 1 line 8v, and NO Tax-Calculator variable can take them.
    state.income_recognized.push(IncomeRecord {
        event: EventId::decision(9),
        recognized_at: date!(2024 - 07 - 01),
        sat: 50_000_000,
        usd_fmv: Usd::from(4_321i64),
        kind: IncomeKind::Staking,
        business: false,
        pseudo: false,
    });
    let reported = unprojected_ledger_lines(&state, 2024);
    assert_eq!(reported.len(), 1, "{reported:?}");
    assert!(reported[0].0.contains("line 8v"), "{reported:?}");
    assert_eq!(reported[0].1, Usd::from(4_321i64));
    assert!(
        !reported[0].2.is_empty(),
        "every entry states its mechanism"
    );

    // …and the same ledger in a DIFFERENT year contributes nothing to this one.
    assert!(unprojected_ledger_lines(&state, 2023).is_empty());

    // ★★★ **M-1 — the census is DERIVED, and here is the proof.** It used to be a hand list of two
    //     with nothing to red if a third ledger quantity started reaching the printed return. Every
    //     `(IncomeKind, business)` pair now goes through `ledger_income_sink`, whose match is
    //     `_`-free — so a sixth kind fails to COMPILE until it is given a sink. What this asserts is
    //     the other half: that each sink is REACHABLE and does what it claims, on a planted record,
    //     rather than being a classification nobody exercises.
    use btctax_core::tax::return_1040::{ledger_income_sink, LedgerSink};
    let kinds = [
        IncomeKind::Mining,
        IncomeKind::Staking,
        IncomeKind::Interest,
        IncomeKind::Airdrop,
        IncomeKind::Reward,
    ];
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for kind in kinds {
        for business in [false, true] {
            let mut one = LedgerState::default();
            one.income_recognized.push(IncomeRecord {
                event: EventId::decision(11),
                recognized_at: date!(2024 - 03 - 03),
                sat: 10_000_000,
                usd_fmv: Usd::from(1_111i64),
                kind,
                business,
                pseudo: false,
            });
            let reported = unprojected_ledger_lines(&one, 2024);
            match ledger_income_sink(kind, business) {
                LedgerSink::Schedule1Line8vReported => {
                    seen.insert("8v");
                    assert_eq!(reported.len(), 1, "{kind:?}/{business} must be reported");
                    assert_eq!(reported[0].1, Usd::from(1_111i64));
                }
                LedgerSink::ScheduleCProjects => {
                    seen.insert("schedule_c");
                    assert!(
                        reported.is_empty(),
                        "{kind:?}/{business} projects through Schedule C and must NOT be reported"
                    );
                }
                LedgerSink::RefusedUpstream => {
                    seen.insert("refused");
                    assert!(reported.is_empty(), "{kind:?}/{business}");
                }
            }
        }
    }
    assert_eq!(
        seen,
        BTreeSet::from(["8v", "refused", "schedule_c"]),
        "a sink nothing reaches is a classification nobody exercises (FR-88)"
    );
}

// ─────────────────────────────────────────────────────────────────────────────────────────────────
// 3. No identity
// ─────────────────────────────────────────────────────────────────────────────────────────────────

#[test]
fn the_projected_row_carries_no_identity() {
    let mut g = two_dependent_household();
    g.w2_income = 123_456.0;
    let (mut ri, state) = build_golden_return(&g);
    // Plant identity everywhere a `ReturnInputs` can hold one, with distinctive tokens.
    ri.header.taxpayer.first_name = "ZQIDENTITYFIRST".into();
    ri.header.taxpayer.last_name = "ZQIDENTITYLAST".into();
    ri.header.taxpayer.ssn = "ZQIDENTITYSSN".into();
    ri.header.address_street = "ZQIDENTITYSTREET".into();
    ri.header.address_city = "ZQIDENTITYCITY".into();
    ri.header.address_zip = "ZQIDENTITYZIP".into();
    ri.header.ip_pin = Some("ZQIDENTITYIPPIN".into());
    if let Some(s) = ri.header.spouse.as_mut() {
        s.first_name = "ZQIDENTITYSPOUSE".into();
        s.ssn = "ZQIDENTITYSPOUSESSN".into();
    }
    for d in &mut ri.header.dependents {
        d.name = "ZQIDENTITYDEP".into();
        d.ssn = "ZQIDENTITYDEPSSN".into();
    }
    for w in &mut ri.w2s {
        w.employer = "ZQIDENTITYEMPLOYER".into();
    }
    let json = serde_json::to_string(&project(&ri, &state)).expect("the oracle row serializes");
    assert!(
        !json.contains("ZQIDENTITY"),
        "the projected row leaked an identity token: {json}"
    );
    // …and it is not vacuous: the row DID carry the household's figures and its dependents.
    assert!(json.contains("123456"), "the row lost the wages: {json}");
    assert_eq!(project(&ri, &state).dependents.len(), 2);
}

// ─────────────────────────────────────────────────────────────────────────────────────────────────
// 4. The ROUTING partition — the facts that route money without being money (T11 fold)
// ─────────────────────────────────────────────────────────────────────────────────────────────────

/// The quantities btctax's return is actually asked about — every line `check_return.py` compares,
/// read off the assembled return. The routing probe compares two of these maps, never one figure.
fn compared_quantities(ri: &ReturnInputs, state: &LedgerState) -> BTreeMap<&'static str, Usd> {
    let ar = assemble(ri, state);
    let mut m = BTreeMap::new();
    m.insert("1040.line7a", ar.capital_gain);
    m.insert("1040.line11", ar.agi);
    m.insert("1040.line12", ar.deduction);
    m.insert("1040.line13", ar.qbi_deduction);
    m.insert("1040.line15", ar.taxable_income);
    m.insert("1040.line16", ar.regular_tax);
    m.insert("1040.line24", ar.total_tax);
    m.insert("schedule_2.line4", ar.se_tax_sch2_l4);
    m.insert(
        "8959.line18",
        ar.additional_medicare.additional_medicare_tax,
    );
    m.insert("8960.line17", ar.niit.tax);
    m.insert(
        "1040sa.line5e",
        ar.schedule_a.as_ref().map_or(Usd::ZERO, |a| a.salt_5e),
    );
    m
}

/// ★★★ **THE FIDELITY MEASURE — how far the household we DESCRIBE is from the return we FILE.**
///
/// `build_golden_return(project(ri))` is the household both engines are actually handed. If a fact
/// the row cannot carry moves the filed return, this vector moves with it; if the row carries the
/// fact, the two move together and the vector does not change. So the probe below never asks "did
/// the row change" (a row can change and still describe the wrong household — the §170 class made
/// `charitable_cash` drop to $0, which is a change AND a lost gift); it asks whether this vector is
/// INVARIANT under the perturbation.
fn description_gap(ri: &ReturnInputs, state: &LedgerState) -> BTreeMap<&'static str, Usd> {
    let row = project(ri, state);
    let (rebuilt, rebuilt_state) = build_golden_return(&row);
    let filed = compared_quantities(ri, state);
    let described = compared_quantities(&rebuilt, &rebuilt_state);
    filed
        .into_iter()
        .map(|(k, v)| (k, v - described[k]))
        .collect()
}

/// ★★★ **Every alternative value a non-`Usd` leaf's TYPE admits — derived from serde, never typed.**
///
/// The same move `money_leaves` makes for `Decimal`: write a value the field cannot hold and let the
/// deserializer classify it. A unit enum answers with `unknown variant \`zzqqzz\`, expected one of
/// \`cash60\`, \`cash30\`, …` — which IS the variant list, straight off the type — and a boolean
/// answers by accepting `true`. Everything else (a `String`, a `Date`, an integer, a free-form code)
/// yields no alternatives and is skipped, which is the honest limit stated in the test below.
fn type_alternatives(doc: &Value, path: &str) -> Vec<Value> {
    let mut junk = doc.clone();
    assert!(set_at(&mut junk, path, Value::String("zzqqzz".into())));
    if let Err(e) = serde_json::from_value::<ReturnInputs>(junk) {
        if let Some(rest) = e.to_string().split("expected one of ").nth(1) {
            return rest
                .split(',')
                .map(|s| s.trim().trim_matches(|c| c == '`' || c == ' ').to_string())
                .filter(|s| !s.is_empty())
                .map(Value::String)
                .collect();
        }
    }
    let mut yes = doc.clone();
    set_at(&mut yes, path, Value::Bool(true));
    if serde_json::from_value::<ReturnInputs>(yes).is_ok() {
        return vec![Value::Bool(true), Value::Bool(false)];
    }
    Vec::new()
}

/// ★★★ **A household the oracle row describes EXACTLY** — the fixtures the fidelity probe measures.
///
/// Built by `build_golden_return`, so `description_gap` is ZERO on them by the inverse KAT. That is
/// the whole point: on a fixture that already carries figures the row cannot express (the maximal
/// sentinel does, by construction), the gap is large and a perturbation moves it for reasons that
/// have nothing to do with the leaf under test — a status change alters how much the UNCARRIED
/// spouse-death fact is worth, and the measure would blame the status. Starting from a gap of zero
/// removes the interaction entirely, and the zero is ASSERTED rather than assumed.
fn fidelity_fixtures() -> Vec<(ReturnInputs, LedgerState)> {
    let rich = GoldenInputs {
        filing_status: "Married/Joint".into(),
        w2_income: 90_000.0,
        taxable_interest: 1_200.0,
        ordinary_dividends: 800.0,
        qualified_dividends: 600.0,
        short_term_capital_gains: 2_000.0,
        long_term_capital_gains: 5_000.0,
        self_employment_income: 12_000.0,
        state_income_tax: 4_000.0,
        real_estate_tax: 3_000.0,
        mortgage_interest: 26_000.0,
        charitable_cash: 1_500.0,
        hsa_deduction: 2_000.0,
        unemployment: 2_500.0,
        dependents: vec![
            GoldenDependent {
                age: 9,
                credit: GoldenCreditColumn::ChildTaxCredit,
                eic_qualifying_child: true,
            },
            GoldenDependent {
                age: 30,
                credit: GoldenCreditColumn::CreditForOtherDependents,
                eic_qualifying_child: false,
            },
        ],
        age_head: Some(66),
        age_spouse: Some(64),
        blind_head: true,
        blind_spouse: true,
        ..Default::default()
    };
    let mut standard = rich.clone();
    // The same household WITHOUT a Schedule A, so a routing fact on the standard-deduction branch is
    // probed too (an election that only ever fires on one branch is half-measured).
    standard.state_income_tax = 0.0;
    standard.real_estate_tax = 0.0;
    standard.mortgage_interest = 0.0;
    standard.charitable_cash = 0.0;
    [rich, standard]
        .into_iter()
        .map(|g| build_golden_return(&g))
        .collect()
}

/// ★★★ **THE ROOT THE T11 SEAM REVIEW ASKED FOR: a fact that ROUTES money and is not carried must
///     be NAMED** — C-2 (the itemize election) and I-1 (a gift's §170 class) were one defect, and
///     this is the partition that has an opinion about both.
///
/// Two derived questions, because neither one alone reaches both instances:
///
/// 1. **Fidelity** (on [`fidelity_fixtures`], where the description starts EXACT). For every
///    non-`Usd` leaf and every alternative its TYPE admits, the household the drivers are handed
///    must go on reproducing the return btctax files. This is what catches a fact whose loss still
///    MOVES the row — a §170 class change makes `charitable_cash` fall to $0, which is a change and
///    a lost gift, so "did the row move?" answers `yes` and means nothing.
/// 2. **Presence** (on the maximal sentinel, whose `Vec`s are realized two rows deep). A
///    perturbation that moves the filed return and does not move the row AT ALL is a fact the row
///    simply cannot express. This question is per-fixture rather than a difference of differences,
///    so it is immune to the interaction above and can safely run on the sentinel's breadth.
///
/// Either way the leaf must be named in `ORACLE_INVISIBLE` with a reason, exactly as an uncarried
/// figure must.
///
/// ★ **What it covers, and what it does not — stated because a wrong stated limit is worse than
///   none.** It covers every walked leaf that is not a money leaf and whose type is a unit enum or a
///   boolean, over both fixture families. It does NOT cover: free text, dates, integers and opaque
///   codes (no alternative is derivable from the type — `header.spouse.date_of_death` is the live
///   example, and FR-93 records it); a leaf absent from the serialized document
///   (`skip_serializing_if`); the element types of an EMPTY `Vec` — the same three blind spots
///   `leaf_walk::money_leaves` documents. And it does not re-examine MONEY leaves: their partition
///   stays *"does the figure reach an oracle box"*, so a money leaf that reaches the row **at the
///   wrong value** is invisible to both. Three such defects existed at HEAD and are fixed by making
///   the projection read the FILED Schedule A (see `project_to_golden`'s doc); the residual class is
///   recorded as FR-95.
#[test]
fn every_routing_fact_either_reaches_the_row_or_is_named_in_oracle_invisible() {
    let mut probed = 0usize;
    let mut unaccounted: BTreeSet<String> = BTreeSet::new();

    // ── 1. Fidelity, on a description that starts exact ──────────────────────────────────────────
    for (base, state) in fidelity_fixtures() {
        let zero = description_gap(&base, &state);
        assert!(
            zero.values().all(|v| *v == Usd::ZERO),
            "a fidelity fixture must be described EXACTLY before anything is perturbed, else the \
             measure below is a difference of differences and blames the wrong leaf: {zero:?}"
        );
        let doc = serde_json::to_value(&base).expect("ReturnInputs serializes");
        let money = money_leaves(&base);
        let mut leaves = Vec::new();
        walk(&doc, "", &mut leaves);
        for path in leaves.iter().filter(|p| !money.contains(*p)) {
            for alt in type_alternatives(&doc, path) {
                let mut v = doc.clone();
                assert!(set_at(&mut v, path, alt.clone()));
                let Ok(ri) = serde_json::from_value::<ReturnInputs>(v) else {
                    continue;
                };
                if ri == base {
                    continue;
                }
                probed += 1;
                let gap = description_gap(&ri, &state);
                if gap.values().all(|v| *v == Usd::ZERO) || oracle_invisible_entry(path).is_some() {
                    continue;
                }
                unaccounted.insert(format!(
                    "{} = {alt}  (the description stops reproducing the return)",
                    normalize_leaf_path(path)
                ));
            }
        }
    }

    // ── 2. Presence, on the maximal sentinel's breadth ───────────────────────────────────────────
    let state = LedgerState::default();
    for base in probe_fixtures() {
        let doc = serde_json::to_value(&base).expect("ReturnInputs serializes");
        let money = money_leaves(&base);
        let mut leaves = Vec::new();
        walk(&doc, "", &mut leaves);
        let filed = compared_quantities(&base, &state);
        let row = project(&base, &state);
        for path in leaves.iter().filter(|p| !money.contains(*p)) {
            for alt in type_alternatives(&doc, path) {
                let mut v = doc.clone();
                assert!(set_at(&mut v, path, alt.clone()));
                let Ok(ri) = serde_json::from_value::<ReturnInputs>(v) else {
                    continue;
                };
                if ri == base {
                    continue;
                }
                probed += 1;
                let moves_the_return = compared_quantities(&ri, &state) != filed;
                let moves_the_row = project(&ri, &state) != row;
                if !moves_the_return || moves_the_row || oracle_invisible_entry(path).is_some() {
                    continue;
                }
                unaccounted.insert(format!(
                    "{} = {alt}  (moves the filed return, moves nothing in the row)",
                    normalize_leaf_path(path)
                ));
            }
        }
    }

    // Anti-vacuity: a probe that stopped generating alternatives would pass on everything (FR-88).
    assert!(
        probed > 200,
        "the routing probe made only {probed} perturbations — it has stopped measuring anything"
    );
    assert!(
        unaccounted.is_empty(),
        "these NON-money facts change the return btctax files without the oracle row following — \
         the drivers would be answering about someone else. Either carry the fact in \
         `project_to_golden` or add an `ORACLE_INVISIBLE` entry saying why no oracle can take \
         it:\n  {}",
        unaccounted.into_iter().collect::<Vec<_>>().join("\n  ")
    );
}

/// The routing partition's own kill, standing beside it: the §170 class IS such a fact, and dropping
/// its entry from `ORACLE_INVISIBLE` must leave the probe with something to report. This is the FR-88
/// shape — a probe whose fixture never reaches the case it exists to catch is green forever — so the
/// fixture is asserted to carry a gift the class filter drops, and the projection to have dropped it.
#[test]
fn a_non_cash_gift_is_dropped_by_the_projection_and_reported_as_such() {
    use btctax_core::tax::return_inputs::{CharitableClass, CharitableGift};
    let state = LedgerState::default();
    let (mut ri, _) = build_golden_return(&GoldenInputs {
        filing_status: "Single".into(),
        w2_income: 120_000.0,
        mortgage_interest: 20_000.0,
        state_income_tax: 4_000.0,
        charitable_cash: 1_000.0,
        ..Default::default()
    });
    ri.schedule_a
        .as_mut()
        .expect("the fixture itemizes")
        .charitable
        .push(CharitableGift {
            class: CharitableClass::CapGainProp30,
            amount: Usd::from(5_000i64),
        });
    // The premise: btctax really does deduct the gift, so the description is short by it.
    let with = assemble(&ri, &state).deduction;
    let mut without = ri.clone();
    without
        .schedule_a
        .as_mut()
        .expect("the fixture itemizes")
        .charitable
        .pop();
    assert_eq!(
        with - assemble(&without, &state).deduction,
        Usd::from(5_000i64),
        "the fixture's appreciated-property gift must move btctax's own line 12"
    );
    // The row carries the CASH gift and nothing else…
    let row = project(&ri, &state);
    assert_eq!(row.charitable_cash, 1_000.0);
    // …and the dropped gift is REPORTED, with its mechanism, rather than vanishing.
    let reported = btctax_core::tax::testonly::unprojected_nonzero_leaves(&ri);
    let gift = reported
        .iter()
        .find(|(p, _, _)| p.starts_with("schedule_a.charitable["))
        .expect("the non-cash gift must be reported as not carried");
    assert_eq!(gift.1, Usd::from(5_000i64));
    assert!(gift.2.note.contains("§170(b)"), "{}", gift.2.note);
    // And the fidelity measure sees the loss the row cannot express.
    assert_ne!(
        description_gap(&ri, &state),
        description_gap(&without, &state),
        "dropping a whole gift from the description must move the fidelity measure"
    );
}

// ─────────────────────────────────────────────────────────────────────────────────────────────────
// 5. Every filing status projects a row its consumers can read (T11 fold, I-4)
// ─────────────────────────────────────────────────────────────────────────────────────────────────

/// ★★★ `project_to_golden` used to map `Single` and `Mfj` and fall through to the enum's `Debug`
///     name for the other three, so a HoH / MFS / QSS return produced a row `build_golden_return`
///     panicked on and `TAXCALC_MARS` could not key — with `income project` exiting 0 and printing
///     it. All five now round-trip through the drivers' own tokens.
#[test]
fn every_filing_status_round_trips_through_the_oracle_row() {
    assert_eq!(
        GOLDEN_FILING_STATUSES.len(),
        5,
        "btctax has five filing statuses; `golden_filing_status_token`'s match is `_`-free, so a \
         sixth fails to compile there before it can reach this list"
    );
    let tokens: BTreeSet<&str> = GOLDEN_FILING_STATUSES
        .into_iter()
        .map(golden_filing_status_token)
        .collect();
    assert_eq!(tokens.len(), 5, "two statuses share one driver token");
    // The exact five words `gen_goldens.TAXCALC_MARS` is keyed by and OTS's `Status` field takes.
    assert_eq!(
        tokens,
        BTreeSet::from([
            "Single",
            "Married/Joint",
            "Married/Sep",
            "Head_of_House",
            "Widow(er)"
        ])
    );
    for status in GOLDEN_FILING_STATUSES {
        let g = GoldenInputs {
            filing_status: golden_filing_status_token(status).into(),
            w2_income: 62_000.0,
            ..Default::default()
        };
        let (ri, state) = build_golden_return(&g);
        assert_eq!(ri.filing_status, status);
        assert_eq!(
            project(&ri, &state),
            g.canonical(),
            "the projection does not round-trip {status:?}"
        );
    }
}
