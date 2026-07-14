//! **P7 — the golden packet round-trip.** The last link in the chain, and the only one that touches paper.
//!
//! ★ What this closes.
//!
//! `btctax-core`'s `golden_returns.rs` proves the NUMBERS are right: it diffs btctax against two
//! independent engines — OpenTaxSolver, driven directly, and the PSL Tax-Calculator — over ten
//! households. But an engine that computes a perfect return and then prints it into the wrong box, or
//! drops a form, or leaves a cell blank, files a wrong return with a clean conscience. Every test
//! between the tax and the paper was, until now, checking btctax against btctax.
//!
//! So this fills the **actual PDFs** for the **same ten households the oracles blessed**, reads the
//! bytes back with the line-keyed inverse transcriber, and asserts that the figures on the paper are
//! the figures the independent engines computed. Not btctax's figures — the ORACLE's. The assertion
//! is literally: *the number OpenTaxSolver arrived at is the number in the box on the 1040.*
//!
//! ★ Why it can assert against a single oracle with no divergence machinery.
//!
//! `golden_returns.rs` declares six divergences, and every one of them is btctax **and OTS** on one
//! side with Tax-Calculator on the other (the Tax Table's $50 bins, which the 1040 instructions make
//! mandatory below $100,000 of taxable income, versus taxcalc's exact rate schedule). So OTS agrees
//! with btctax on **every line of every household**, and the round-trip can hold the paper against it
//! directly.
//!
//! ★ The households come from `btctax_core::tax::testonly`, not from a copy.
//!
//! A second builder in this crate could drift, and a drifted round-trip would be filling forms for a
//! different taxpayer than the one the oracle validated — while still passing. One fixture, one packet.

use btctax_core::conventions::round_dollar;
use btctax_core::tax::packet::assemble_printed_return;
use btctax_core::tax::return_1040::assemble_absolute;
use btctax_core::tax::testonly::{
    build_golden_household, golden_households, ty2024_params, ty2024_table, GoldenHousehold,
};
use btctax_forms::testonly::{extract_lines, F1040_MAP_2024};
use btctax_forms::{fill_full_return, NamedForm};
use std::collections::BTreeMap;

/// Fill the whole packet for one golden household.
fn packet(h: &GoldenHousehold) -> Vec<NamedForm> {
    let (ri, state) = build_golden_household(h);
    let params = ty2024_params();
    let table = ty2024_table();
    let ar = assemble_absolute(&ri, &state, &params, &table, 2024);
    // No golden household makes a charitable donation, so there are no §170(e) details to carry.
    let pr = assemble_printed_return(&ri, &state, &BTreeMap::new(), &ar, &table, 2024)
        .expect("the golden households carry well-formed SSNs");
    fill_full_return(&pr, 2024).unwrap_or_else(|e| panic!("{}: the packet must fill — {e}", h.name))
}

fn form<'a>(pkt: &'a [NamedForm], name: &str) -> &'a NamedForm {
    pkt.iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("the packet is missing {name}"))
}

/// A dollar figure as it is PRINTED — whole dollars, no separators (SPEC §3.1).
fn printed(v: f64) -> String {
    round_dollar(btctax_core::conventions::Usd::try_from(v).expect("finite")).to_string()
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// ★ **The figures an independent engine computed are the figures in the boxes on the 1040.**
#[test]
fn every_golden_household_prints_the_oracles_figures_onto_the_1040() {
    let mut wrong: Vec<String> = Vec::new();

    for h in &golden_households() {
        let pkt = packet(h);
        let f1040 = form(&pkt, "f1040");
        let got = extract_lines(&f1040.bytes, F1040_MAP_2024)
            .unwrap_or_else(|e| panic!("{}: the filled 1040 must transcribe — {e}", h.name));

        let e = &h.expected_ots;
        // (the 1040's own line label, the cell as printed, what OpenTaxSolver computed)
        let checks: [(&str, &str, f64); 4] = [
            ("line11", "AGI", e.adjusted_gross_income),
            ("line15", "taxable income", e.taxable_income),
            ("line16", "tax", e.income_tax_before_credits),
            ("line24", "TOTAL TAX", e.total_tax),
        ];

        for (cell, label, oracle) in checks {
            let on_paper = got.get(cell).map(String::as_str).unwrap_or("<BLANK>");
            let expected = printed(oracle);
            if on_paper != expected {
                wrong.push(format!(
                    "  {:<42} 1040 {cell:<8} ({label:<14}) paper {on_paper:>10}   OpenTaxSolver {expected:>10}",
                    h.name
                ));
            }
        }
    }

    assert!(
        wrong.is_empty(),
        "the filed 1040 disagrees with an INDEPENDENT tax engine on {} cell(s).\n\
         The return computes correctly and prints something else — which is the one class of bug every \
         other test in this repo is blind to. Do not weaken this test to make it pass.\n\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

/// The SE households must carry their Schedule SE, and its line 12 must be the oracle's SE tax.
///
/// Schedule 2 line 4 CITES Schedule SE line 12, so a return whose 1040 is right while its Schedule SE
/// says something else is internally contradictory on its face — the kind of thing an examiner sees
/// immediately and no self-referential test can see at all.
#[test]
fn the_se_households_print_the_oracles_se_tax_onto_schedule_se() {
    let mut checked = 0;

    for h in &golden_households() {
        if h.expected_ots.se_tax == 0.0 {
            continue;
        }
        let pkt = packet(h);
        let se = form(&pkt, "schedule_se");
        let got = extract_lines(&se.bytes, btctax_forms::testonly::SCHEDULE_SE_MAP_2024).unwrap();

        assert_eq!(
            got.get("line12").map(String::as_str),
            Some(printed(h.expected_ots.se_tax).as_str()),
            "{}: Schedule SE line 12 must be the SE tax OpenTaxSolver computed",
            h.name
        );
        checked += 1;
    }

    assert_eq!(
        checked, 2,
        "the matrix has exactly two self-employment households; if that changed, this test went quiet"
    );
}

/// A household with no self-employment must not be handed a Schedule SE at all.
///
/// The packet is assembled from what the return NEEDS. Stapling an empty Schedule SE to a W-2 filer's
/// return is not a cosmetic defect: it asserts to the IRS that they had self-employment income.
#[test]
fn a_w2_only_household_gets_no_schedule_se_and_no_schedule_c() {
    let households = golden_households();
    let h = households
        .iter()
        .find(|h| h.name == "single_w2_only_standard")
        .expect("the floor case is in the matrix");

    let pkt = packet(h);
    let names: Vec<&str> = pkt.iter().map(|f| f.name.as_str()).collect();

    assert!(names.contains(&"f1040"), "every return has a 1040");
    assert!(
        !names.contains(&"schedule_se"),
        "a W-2-only filer has no self-employment tax; the packet must not include Schedule SE. Got: {names:?}"
    );
    assert!(
        !names.contains(&"f1040sc"),
        "a W-2-only filer runs no business; the packet must not include Schedule C. Got: {names:?}"
    );
}

/// Every form in every golden packet carries the filer's name and SSN.
///
/// A schedule that arrives at the IRS without an SSN on it is a loose page. This iterates the WHOLE
/// packet for EVERY household rather than pinning one form — the P6 review found an unnamed Form 8949
/// precisely because a test that checked one form had promised to check all of them.
#[test]
fn every_form_of_every_golden_packet_carries_the_filers_identity() {
    // The map key under which each form carries its identity block, and the map to read it with.
    let maps: BTreeMap<&str, &str> = BTreeMap::from([
        ("f1040", F1040_MAP_2024),
        ("f1040s1", btctax_forms::testonly::SCHEDULE_1_MAP_2024),
        ("f1040s2", btctax_forms::testonly::SCHEDULE_2_MAP_2024),
        ("f1040s3", btctax_forms::testonly::SCHEDULE_3_MAP_2024),
        ("f1040sa", btctax_forms::testonly::SCHEDULE_A_MAP_2024),
        ("f1040sb", btctax_forms::testonly::SCHEDULE_B_MAP_2024),
        ("f1040sc", btctax_forms::testonly::SCHEDULE_C_MAP_2024),
        ("schedule_d", btctax_forms::testonly::SCHEDULE_D_MAP_2024),
        ("schedule_se", btctax_forms::testonly::SCHEDULE_SE_MAP_2024),
        ("f8959", btctax_forms::testonly::F8959_MAP_2024),
        ("f8960", btctax_forms::testonly::F8960_MAP_2024),
        ("f8995", btctax_forms::testonly::F8995_MAP_2024),
        ("f8949", btctax_forms::testonly::F8949_MAP_2024),
    ]);

    let mut naked: Vec<String> = Vec::new();
    let mut seen = 0;

    for h in &golden_households() {
        for f in &packet(h) {
            let Some(map) = maps.get(f.name.as_str()) else {
                panic!(
                    "{}: the packet contains {} but this test has no map for it — a new form was added \
                     and the identity check would have silently skipped it",
                    h.name, f.name
                );
            };
            let got = extract_lines(&f.bytes, map).unwrap();
            seen += 1;

            // Forms spell the identity block differently — the 1040 has `header.taxpayer_ssn`, the
            // schedules `identity.ssn`, the 8949 one per page (`identity_page1.ssn`). Match on the
            // SHAPE of the key rather than enumerating them, so a form that invents a fourth spelling
            // still gets checked instead of quietly passing. `extract_lines` only ever returns cells
            // that carry text, so a key being present IS the value being non-empty.
            let key_ends = |suffix: &str| got.keys().any(|k| k.ends_with(suffix));
            let has_ssn = key_ends("ssn");
            let has_name = key_ends("name") || (key_ends("_first") && key_ends("_last"));
            if !has_name || !has_ssn {
                naked.push(format!(
                    "  {:<42} {:<12} name={has_name} ssn={has_ssn}   keys: {:?}",
                    h.name,
                    f.name,
                    got.keys().take(6).collect::<Vec<_>>()
                ));
            }
        }
    }

    assert!(seen > 30, "the sweep must actually see forms, saw {seen}");
    assert!(
        naked.is_empty(),
        "{} form(s) would reach the IRS without a name or an SSN on them:\n{}",
        naked.len(),
        naked.join("\n")
    );
}
