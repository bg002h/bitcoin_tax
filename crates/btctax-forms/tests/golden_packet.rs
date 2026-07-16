//! **P7 — the golden packet round-trip.** The last link in the chain, and the only one that touches paper.
//!
//! ★ What this closes.
//!
//! `btctax-core`'s `golden_returns.rs` proves the NUMBERS are right: it diffs btctax against two
//! independent engines — OpenTaxSolver, driven directly, and the PSL Tax-Calculator — over twelve
//! households. But an engine that computes a perfect return and then prints it into the wrong box, or
//! drops a form, or leaves a cell blank, files a wrong return with a clean conscience. Every test
//! between the tax and the paper was, until now, checking btctax against btctax.
//!
//! So this fills the **actual PDFs** for the **same twelve households the oracles blessed**, reads the
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

use btctax_core::conventions::{round_dollar, Usd};
use btctax_core::tax::testonly::golden_households;
use btctax_forms::testonly::{extract_lines, F1040_MAP_2024};
use std::collections::BTreeMap;

// The `packet`/`form` builders live in the shared `tests/common/mod.rs` — ONE builder, so the P7
// round-trip and the oracle-sweep read-back (T4) fill the SAME households, never a drifting copy.
mod common;
use common::{form, packet};

fn usd(v: f64) -> Usd {
    Usd::try_from(v).expect("the oracles emit finite figures")
}

/// A dollar figure as it is PRINTED — whole dollars, no separators (SPEC §3.1).
fn printed(v: f64) -> String {
    round_dollar(usd(v)).to_string()
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

        // ★ TOTAL TAX is CROSS-FOOTED from the oracle's own component lines, not taken from its total.
        //
        // SPEC §3.1 rounds at the point of REPORTING: the filed line 24 adds the PRINTED lines (Σround),
        // while OTS keeps cents and reports round(Σexact). ★ 26 USC §6102 is genuinely AMBIGUOUS and
        // BOTH are LAWFUL — line 22 is both "an amount required to be shown" (round it) and "an item
        // taken into account" in computing line 24 (do not). btctax ELECTS Σround: line 24's own text
        // says "Add lines 22 and 23" — the LINES, which §6102(a) requires at whole dollars — and the IRS
        // cannot even receive the alternative electronically (MeF types all three as xsd:integer).
        // The full authority hierarchy — and why the IRM, the instructions and the MeF schema are all
        // EVIDENCE, never LAW — is in `design/full-return/ROUNDING_AUTHORITY.md`.
        //
        // Earlier this was a hardcoded name+cell exception with the two figures written out by hand.
        // That charged its cost twice — both constants had to be re-edited by hand the moment the
        // household's inputs changed (Fable P7 r3, Nit). Applying §3.1's own printing rule to the
        // ORACLE's components instead is not self-referential — these are OpenTaxSolver's numbers,
        // rounded the way the form rounds them — and it needs no exception at all.
        // ★ The formula encodes a PRECONDITION, so assert it rather than leave it implicit (Fable P7
        // r4, Minor). Line 24 = 22 + 23, where 22 = 18 − 21 (CREDITS) and 18 = 16 + 17 (AMT / excess
        // APTC). Dropping those terms is only valid because every golden household has zero credits and
        // zero AMT — no dependents, no foreign tax credit, AMT screened out. A thirteenth household with
        // a foreign tax credit would make this formula overstate line 24 by the credit, and it would
        // fail pointing at the printed 1040 rather than at the formula, sending the next author to
        // debug the wrong thing.
        for (cell, what) in [("line17", "AMT / excess APTC"), ("line21", "credits")] {
            // Asserted as PRESENT-and-zero, not defaulted-to-zero (Fable P7 r5, Nit). Defaulting an
            // absent cell to "0" would silently make this guard vacuous if the 1040 filler ever stopped
            // writing those lines — resting the guard's soundness on a property of a different module
            // that nothing here pins. The filler writes every mapped line, including explicit zeros; if
            // that ever changes, this should fail loudly rather than quietly stop checking.
            assert_eq!(
                got.get(cell).map(String::as_str),
                Some("0"),
                "{}: the cross-foot formula for line 24 assumes NO {what} (1040 {cell}) and that the \
                 line is PRINTED. Either this household has some — extend the formula, do not weaken \
                 the assertion — or the 1040 filler stopped writing the line, which this guard depends \
                 on.",
                h.name
            );
        }
        let oracle_line24 = round_dollar(usd(e.income_tax_before_credits))
            + round_dollar(usd(e.se_tax))
            + round_dollar(usd(e.niit))
            + round_dollar(usd(e.additional_medicare_tax));

        // (the 1040's own line label, the cell as printed, what OpenTaxSolver computed)
        let checks: [(&str, &str, Usd); 4] = [
            ("line11", "AGI", round_dollar(usd(e.adjusted_gross_income))),
            ("line15", "taxable income", round_dollar(usd(e.taxable_income))),
            ("line16", "tax", round_dollar(usd(e.income_tax_before_credits))),
            ("line24", "TOTAL TAX (cross-footed)", oracle_line24),
        ];

        for (cell, label, oracle) in checks {
            let on_paper = got.get(cell).map(String::as_str).unwrap_or("<BLANK>");
            let expected = oracle.to_string();
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
        checked, 3,
        "the matrix has exactly three self-employment households; if that changed, this test went quiet"
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


// ══════════════════════════ the packet as an ARTIFACT: what's in it, in what order, byte-for-byte ══

/// ★ **Exactly the forms each return requires — no more, no fewer.**
///
/// A DROPPED form understates the return (the P6 review found Schedule 3 missing its line 10, and a
/// filer billed twice for tax they had already paid). A SPURIOUS form makes an assertion the filer did
/// not intend: an empty Schedule SE stapled to a W-2 filer's return tells the IRS they had
/// self-employment income; an empty Schedule C tells them they ran a business.
///
/// Every set below is a claim about the LAW, not a transcript of current behaviour:
///   * Schedule B files only above the **$1,500** interest/dividend trigger — so the MFJ household
///     with $1,200 of interest gets none, and that is the discriminating case.
///   * Schedule D + 8949 file when there are capital gains — so the two self-employment households,
///     which have none, get neither.
///   * Form 8959 files above the Additional-Medicare threshold — $200k single / $250k MFJ — so the
///     Single miner at $100k gets none while the MFJ one at $300k does.
///   * Form 8995 (§199A) files when there is qualified business income — i.e. exactly with Schedule C.
#[test]
fn each_golden_packet_carries_exactly_the_forms_that_return_requires() {
    let expected: BTreeMap<&str, &[&str]> = BTreeMap::from([
        // No crypto, no schedules at all — the floor case.
        ("single_w2_only_standard", &["f1040"][..]),
        // Interest $1,200 is BELOW the $1,500 Schedule B trigger.
        ("mfj_two_w2_standard", &["f1040"][..]),
        (
            "single_w2_plus_crypto_ltcg",
            &["f1040", "schedule_d", "f8949"][..],
        ),
        (
            "single_short_term_crypto_gain",
            &["f1040", "schedule_d", "f8949"][..],
        ),
        (
            "single_capital_loss_capped",
            &["f1040", "schedule_d", "f8949"][..],
        ),
        // $2,000 interest + $10,000 dividends clears the $1,500 trigger ⇒ Schedule B.
        (
            "single_qdcgt_both_slices",
            &["f1040", "f1040sb", "schedule_d", "f8949"][..],
        ),
        (
            "mfj_itemized_over_100k",
            &["f1040", "f1040s2", "f1040sa", "f1040sb", "schedule_d", "f8949", "f8960"][..],
        ),
        (
            "mfj_high_income_niit_and_addl_medicare",
            &["f1040", "f1040s2", "f1040sb", "schedule_d", "f8949", "f8959", "f8960"][..],
        ),
        // Wages and a Schedule A, nothing else — no interest, no crypto, no business.
        ("mfj_itemized_salt_over_the_cap", &["f1040", "f1040sa"][..]),
        // Mining as a business: Schedule C ⇒ Schedule SE ⇒ Schedule 2, and §199A ⇒ 8995. No capital
        // gains, so no Schedule D. $100k total is under the $200k single 8959 threshold.
        (
            "single_crypto_business_se",
            &["f1040", "f1040s1", "f1040s2", "f1040sc", "schedule_se", "f8995"][..],
        ),
        // A miner WITH a capital gain: Schedule D + 8949 join the Schedule C/SE/8995 set. This is the
        // only household that carries both, and it is why Form 8995 line 12 is oracle-checked at all.
        (
            "single_miner_qbi_limited_by_net_capital_gain",
            // $5,000 of dividends clears the $1,500 Schedule B trigger too.
            &["f1040", "f1040s1", "f1040s2", "f1040sb", "f1040sc", "schedule_d", "f8949", "schedule_se", "f8995"][..],
        ),
        // Same, but $300k MFJ clears the $250k Additional-Medicare threshold ⇒ 8959.
        (
            "mfj_se_over_the_addl_medicare_threshold",
            &["f1040", "f1040s1", "f1040s2", "f1040sc", "schedule_se", "f8995", "f8959"][..],
        ),
    ]);

    let mut wrong = Vec::new();
    for h in &golden_households() {
        let want: BTreeMap<&str, ()> = expected
            .get(h.name.as_str())
            .unwrap_or_else(|| panic!("{}: no expected form set — a household was added and its packet went unchecked", h.name))
            .iter()
            .map(|n| (*n, ()))
            .collect();
        let got: BTreeMap<&str, ()> = packet(h).iter().map(|f| (leak(&f.name), ())).collect();

        let missing: Vec<_> = want.keys().filter(|k| !got.contains_key(*k)).collect();
        let spurious: Vec<_> = got.keys().filter(|k| !want.contains_key(*k)).collect();
        if !missing.is_empty() || !spurious.is_empty() {
            wrong.push(format!(
                "  {:<42} MISSING {missing:?}  SPURIOUS {spurious:?}",
                h.name
            ));
        }
    }
    assert!(
        wrong.is_empty(),
        "the assembled packet is not the return the law requires:\n{}",
        wrong.join("\n")
    );
}

/// The packet is stapled in IRS **Attachment Sequence** order, with the 1040 first.
///
/// The sequence numbers are printed on the forms themselves ("Attachment Sequence No. 12"), and the
/// IRS asks for them in order. This is the one property of the packet a filer can see at a glance and
/// we cannot check by reading any single form.
#[test]
fn the_packet_is_stapled_in_irs_attachment_sequence_order() {
    for h in &golden_households() {
        let pkt = packet(h);

        assert_eq!(
            pkt[0].name, "f1040",
            "{}: the 1040 sorts first — it has no attachment sequence of its own",
            h.name
        );
        assert!(
            pkt[0].attachment_sequence.is_none(),
            "{}: the 1040 carries no Attachment Sequence number",
            h.name
        );

        let seqs: Vec<&str> = pkt[1..]
            .iter()
            .map(|f| {
                f.attachment_sequence
                    .unwrap_or_else(|| panic!("{}: {} has no attachment sequence", h.name, f.name))
            })
            .collect();
        let mut sorted = seqs.clone();
        sorted.sort();
        assert_eq!(
            seqs, sorted,
            "{}: the packet is out of Attachment Sequence order — got {seqs:?}",
            h.name
        );
    }
}

/// ★ **The same return fills to the same bytes.** Twice, for every household.
///
/// Each individual filler already pins its own content hash, but nothing until now asserted the
/// property of the PACKET: that `fill_full_return` — which walks a form set, assembles an order, and
/// serializes a dozen documents — is reproducible end to end. Anything non-deterministic that leaked
/// into the output (a hash-map iteration order reaching a page tree, a timestamp, a fresh object id)
/// would show up here and nowhere else. A return you cannot reproduce is a return you cannot attest to.
#[test]
fn the_whole_packet_is_byte_reproducible() {
    for h in &golden_households() {
        let a = packet(h);
        let b = packet(h);

        assert_eq!(
            a.len(),
            b.len(),
            "{}: the packet changed SIZE between two fills",
            h.name
        );
        for (x, y) in a.iter().zip(b.iter()) {
            assert_eq!(x.name, y.name, "{}: form order is not stable", h.name);
            assert_eq!(
                x.attachment_sequence, y.attachment_sequence,
                "{}: {} attachment sequence is not stable",
                h.name, x.name
            );
            assert_eq!(
                x.bytes,
                y.bytes,
                "{}: {} does not fill to the same bytes twice ({} vs {} bytes)",
                h.name,
                x.name,
                x.bytes.len(),
                y.bytes.len()
            );
        }
    }
}

/// `&'static str` from a `String` we own for the life of the test — keeps the set comparison above
/// readable without threading lifetimes through it.
fn leak(s: &str) -> &'static str {
    Box::leak(s.to_string().into_boxed_str())
}

/// ★ **The §164(b)(5) SALT cap is applied ON THE PAPER, not just in the engine.**
///
/// Schedule A line 5e is "the smaller of line 5d or $10,000". This household's state income tax
/// ($1,068) and real estate tax ($10,509) add to **$11,577**, so the cap BINDS: line 5e must print
/// $10,000 and the filer loses $1,577 of deduction. Both independent oracles agree on the resulting
/// taxable income ($3,730), which is what makes this checkable at all.
///
/// The SALT figures are IRS ATS Test Scenario 2's — the only IRS-authored numbers in the matrix. (The
/// scenario itself is NOT a golden return: its 1040 is blank and even Schedule A's computed totals are
/// blank. It is a test-case specification, not an answer key. See FOLLOWUPS `p7-ats-scenario-2`.)
///
/// A cap that is computed but printed uncapped files a return claiming $1,577 of deduction the law
/// does not allow, and every arithmetic test in the repo would still be green.
#[test]
fn the_salt_cap_is_printed_onto_schedule_a() {
    let households = golden_households();
    let h = households
        .iter()
        .find(|h| h.name == "mfj_itemized_salt_over_the_cap")
        .expect("the SALT-cap household is in the matrix");

    let pkt = packet(h);
    let got = extract_lines(
        &form(&pkt, "f1040sa").bytes,
        btctax_forms::testonly::SCHEDULE_A_MAP_2024,
    )
    .unwrap();

    let cell = |k: &str| got.get(k).map(String::as_str).unwrap_or("<BLANK>");

    assert_eq!(cell("line5a"), "1068", "state & local income tax");
    assert_eq!(cell("line5b"), "10509", "real estate tax");
    assert_eq!(cell("line5d"), "11577", "5a + 5b — the UNCAPPED total");
    assert_eq!(
        cell("line5e"),
        "10000",
        "★ the §164(b)(5) cap: 5e is the SMALLER of 5d ($11,577) and $10,000. Printing $11,577 here \
         would claim $1,577 of deduction the law does not allow — and every arithmetic test in this \
         repo would still be green."
    );
    assert_eq!(cell("line8a"), "25000", "mortgage interest");
    assert_eq!(
        cell("line17"),
        "35000",
        "total itemized = the CAPPED $10,000 + $25,000 mortgage. It beats the $29,200 standard \
         deduction, so the cap actually changes this filer's tax."
    );
}

/// ★ **Fable P7 r1 I1.** Form 8995's Part I table must carry the business its line 2 totals.
///
/// Line 2's own text is "Total qualified business income or (loss). **Combine lines 1i through 1v,
/// column (c)**". P7 gave the crypto Schedule C a §199A deduction, which made line 2 non-zero — and
/// left the column EMPTY. The filed form totalled a column with no rows and named no business for the
/// deduction it claimed: facially incomplete, the same class as P6's unnamed Form 8949.
///
/// With one trade or business the column total IS the row, so 1i(c) must equal line 2 exactly.
#[test]
fn the_se_households_name_their_business_in_form_8995s_part_i_table() {
    let mut checked = 0;

    for h in &golden_households() {
        if h.inputs.self_employment_income <= 0.0 {
            continue;
        }
        let pkt = packet(h);
        let got = extract_lines(
            &form(&pkt, "f8995").bytes,
            btctax_forms::testonly::F8995_MAP_2024,
        )
        .unwrap();

        let cell = |k: &str| got.get(k).map(String::as_str).unwrap_or("<BLANK>");

        assert_eq!(
            cell("row1_business"),
            "Bitcoin mining",
            "{}: 8995 row 1i(a) must NAME the trade or business the deduction is claimed for",
            h.name
        );
        assert_eq!(
            cell("row1_tin"),
            "123-45-6789",
            "{}: 8995 row 1i(b) is the business's TIN — a sole proprietor's own SSN, hyphenated \
             (the cell's /MaxLen is 11)",
            h.name
        );
        assert_eq!(
            cell("row1_qbi"),
            cell("line2"),
            "{}: with ONE business, line 2 (\"combine lines 1i through 1v, column (c)\") IS row 1i(c). \
             A line 2 that does not equal the column it totals is a form that does not add up.",
            h.name
        );
        assert_ne!(
            cell("line2"),
            "<BLANK>",
            "{}: the SE households have business QBI; line 2 must be printed",
            h.name
        );
        checked += 1;
    }

    assert_eq!(
        checked, 3,
        "the matrix has exactly three self-employment households; if that changed, this test went quiet"
    );
}

/// A REIT-only Form 8995 leaves Part I blank — there IS no trade or business, and inventing one would
/// name a business the filer does not have. (No golden household has REIT dividends, so this is pinned
/// by the unit KATs in `full_return_forms.rs`; asserted here only to state the contract.)
#[test]
fn a_household_with_no_business_files_no_form_8995_row() {
    for h in &golden_households() {
        if h.inputs.self_employment_income > 0.0 {
            continue;
        }
        let pkt = packet(h);
        assert!(
            !pkt.iter().any(|f| f.name == "f8995"),
            "{}: no QBI of any kind ⇒ no Form 8995 at all",
            h.name
        );
    }
}

/// ★ **Fable P7 r3 I1.** A filed Schedule C must NAME its business on line A.
///
/// The 8995's Part I row is not the only place a business must be named — Schedule C line A
/// ("Principal business or profession") demands it too, and it is the case the 8995's own fail-closed
/// CANNOT reach: a business whose net profit is at or below the §6017 $400 SE floor produces no QBI,
/// hence no Form 8995 at all, so the filler's guard never runs. Only the core refusal stands between
/// that filer and a Schedule C with a blank line A — and the refusal shipped untested.
#[test]
fn every_filed_schedule_c_names_its_business_on_line_a() {
    let mut checked = 0;

    for h in &golden_households() {
        let pkt = packet(h);
        let Some(f) = pkt.iter().find(|f| f.name == "f1040sc") else {
            continue;
        };
        let got = extract_lines(&f.bytes, btctax_forms::testonly::SCHEDULE_C_MAP_2024).unwrap();

        assert_eq!(
            got.get("line_a_business").map(String::as_str),
            Some("Bitcoin mining"),
            "{}: Schedule C line A is the business's name. A blank line A files a business the return \
             never identifies.",
            h.name
        );
        checked += 1;
    }

    assert_eq!(
        checked, 3,
        "the matrix has exactly three Schedule C households; if that changed, this test went quiet"
    );
}
