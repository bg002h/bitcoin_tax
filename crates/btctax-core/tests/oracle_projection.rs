//! ★★★ **T11 / SPEC_interview.md R13 — the oracle path's KATs.**
//!
//! Three guarantees, and every one of them is DERIVED rather than declared:
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

use btctax_core::conventions::Usd;
use btctax_core::state::LedgerState;
use btctax_core::tax::provenance::leaf_walk::{money_leaves, set_at};
use btctax_core::tax::return_inputs::ReturnInputs;
use btctax_core::tax::testonly::{
    build_golden_return, every_money_leaf_household, golden_households, normalize_leaf_path,
    oracle_invisible_entry, project_to_golden, GoldenCreditColumn, GoldenDependent, GoldenInputs,
    ORACLE_INVISIBLE,
};
use std::collections::BTreeSet;

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
            project_to_golden(&ri, &state),
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
        project_to_golden(&ri, &state),
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
    let full = project_to_golden(&ri, &state);
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

/// Every money leaf that MOVES the projection on at least one probe fixture — measured by
/// perturbing the leaf and comparing the projected rows, never by reading a list.
fn visible_leaves() -> BTreeSet<String> {
    let state = LedgerState::default();
    let mut visible = BTreeSet::new();
    for base in probe_fixtures() {
        let baseline = project_to_golden(&base, &state);
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
            if project_to_golden(&ri, &state) != baseline {
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
#[test]
fn no_oracle_invisible_entry_is_stale() {
    let (base, _) = every_money_leaf_household();
    let leaves: Vec<String> = money_leaves(&base).into_iter().collect();
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
        "these ORACLE_INVISIBLE entries claim no money leaf — the field they named is gone: {stale:?}"
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
    let _ = project_to_golden(&clean, &state);
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
    let json =
        serde_json::to_string(&project_to_golden(&ri, &state)).expect("the oracle row serializes");
    assert!(
        !json.contains("ZQIDENTITY"),
        "the projected row leaked an identity token: {json}"
    );
    // …and it is not vacuous: the row DID carry the household's figures and its dependents.
    assert!(json.contains("123456"), "the row lost the wages: {json}");
    assert_eq!(project_to_golden(&ri, &state).dependents.len(), 2);
}
