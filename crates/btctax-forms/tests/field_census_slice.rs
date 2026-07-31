//! §G-13/§G-17 — **the thin vertical slice**: one REAL return, its REAL form set, every emitted
//! AcroForm field accounted for or not.
//!
//! ★★ **Why this exists rather than another design round.** The census design has been reviewed
//! twice and the gaps that mattered were all found in *code*, not in the document. This measures the
//! three §G-17 questions instead of arguing them:
//!
//! 1. does **overflow renaming** (`overflow::merge_copies` uniquifies FQNs on copies 1..) actually
//!    bite on a real return, or is it theoretical?
//! 2. how many fields belong to forms the return **does not emit** — the "~800 flood" estimate?
//! 3. what is the **real** unaccounted count on a return, versus the static 496?
//!
//! ★ It asserts only what is already settled, and PRINTS the rest. A measurement that reds on a
//! number nobody has agreed to would be a gate masquerading as a probe — run with
//! `cargo test -p btctax-forms --test field_census_slice -- --nocapture` to read it.

use btctax_core::conventions::Usd;
use btctax_core::event::DisposeKind;
use btctax_core::forms::{Form8949Box, Form8949Part, Form8949Row};
use btctax_core::identity::WalletId;
use btctax_core::tax::packet::assemble_printed_return;
use btctax_core::tax::return_1040::assemble_absolute;
use btctax_core::tax::testonly::{kitchen_sink_household, ty2024_params, ty2024_table};
use btctax_forms::fill_full_return;
use btctax_forms::testonly::{collect_fields, load};
use rust_decimal_macros::dec;
use std::collections::{BTreeMap, BTreeSet};

/// Every FQN the committed map for `stem` names. `None` when the form has no map file (which is
/// itself a finding — an emitted form with no map is 100% unaccounted).
fn mapped_fqns(stem: &str, year: i32) -> Option<BTreeSet<String>> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("forms")
        .join(year.to_string())
        .join(format!("{stem}.map.toml"));
    let text = std::fs::read_to_string(path).ok()?;
    let mut out = BTreeSet::new();
    // The maps are `key = "FQN"` TOML; an FQN is the only quoted string containing `[0]`.
    for line in text.lines() {
        for chunk in line.split('"').skip(1).step_by(2) {
            if chunk.contains("[0]") && chunk.contains('.') {
                out.insert(chunk.to_string());
            }
        }
    }
    Some(out)
}

/// ★ Strip a `merge_copies` per-copy prefix so an emitted FQN can be compared with the TEMPLATE's.
///
/// `overflow.rs` renames the root `/T` on copies 1.., uniquifying every descendant name. The census
/// keys on the template — copy 2's cell is the same *logical* decision a second time, not a new one —
/// so the emitted name must be normalised back before it is called unaccounted.
fn normalise_copy(fqn: &str) -> String {
    // Root segment only; everything after the first `.` is the template-stable path.
    match fqn.split_once('.') {
        Some((_root, rest)) => rest.to_string(),
        None => fqn.to_string(),
    }
}

#[test]
fn one_real_return_field_census() {
    let (ri, state) = kitchen_sink_household();
    let table = ty2024_table();
    let ar = assemble_absolute(&ri, &state, &ty2024_params(), &table, 2024);
    let details: BTreeMap<_, _> = BTreeMap::new();
    let pr = assemble_printed_return(&ri, &state, &details, &ar, &table, 2024, &[])
        .expect("kitchen_sink assembles");
    let forms = fill_full_return(&pr, 2024).expect("the packet fills");

    // The 15 forms the census tracks — the STATIC decision surface, per census.rs's settled rule
    // that a household's packet is never the authority.
    const CENSUS_KEYS: [&str; 15] = [
        "f1040",
        "f1040s1",
        "f1040s2",
        "f1040s3",
        "f1040sa",
        "f1040sb",
        "f1040sc",
        "schedule_d",
        "f8949",
        "schedule_se",
        "f8959",
        "f8960",
        "f8995",
        "f8283",
        "f8275",
    ];

    let emitted: BTreeSet<&str> = forms.iter().map(|f| f.name.as_str()).collect();
    let absent: Vec<&str> = CENSUS_KEYS
        .iter()
        .copied()
        .filter(|k| !emitted.contains(k))
        .collect();

    println!("\n  ┌─ ONE REAL RETURN (kitchen_sink, TY2024) ─────────────────────────────");
    println!(
        "  │ forms emitted: {} of {}",
        emitted.len(),
        CENSUS_KEYS.len()
    );
    println!("  │ NOT emitted:   {absent:?}");
    println!("  ├──────────────────────────────────────────────────────────────────────");
    println!(
        "  │ {:<12} {:>7} {:>7} {:>8} {:>9}",
        "form", "fields", "mapped", "unmapped", "renamed"
    );

    let (mut tot_fields, mut tot_mapped, mut tot_renamed) = (0usize, 0usize, 0usize);
    for f in &forms {
        let doc = load(&f.bytes).expect("emitted form parses");
        let fields = collect_fields(&doc).expect("fields");
        let map = mapped_fqns(&f.name, 2024);

        // A field is "renamed" when its own FQN is absent from the map but its NORMALISED form is
        // present — i.e. it is an overflow copy of a mapped cell, not an unaccounted field.
        let (mut mapped, mut renamed) = (0usize, 0usize);
        for fld in &fields {
            match &map {
                Some(m) if m.contains(&fld.fqn) => mapped += 1,
                Some(m) => {
                    let n = normalise_copy(&fld.fqn);
                    if m.iter().any(|k| normalise_copy(k) == n) {
                        renamed += 1;
                    }
                }
                None => {}
            }
        }
        let unmapped = fields.len() - mapped - renamed;
        println!(
            "  │ {:<12} {:>7} {:>7} {:>8} {:>9}{}",
            f.name,
            fields.len(),
            mapped,
            unmapped,
            renamed,
            if map.is_none() {
                "  ← NO MAP FILE"
            } else {
                ""
            }
        );
        tot_fields += fields.len();
        tot_mapped += mapped;
        tot_renamed += renamed;
    }

    println!("  ├──────────────────────────────────────────────────────────────────────");
    println!(
        "  │ {:<12} {:>7} {:>7} {:>8} {:>9}",
        "TOTAL",
        tot_fields,
        tot_mapped,
        tot_fields - tot_mapped - tot_renamed,
        tot_renamed
    );
    println!("  └──────────────────────────────────────────────────────────────────────\n");

    // ── What this asserts (only the settled parts) ──────────────────────────────────────────
    assert!(
        !forms.is_empty(),
        "a real household must emit forms — an empty packet would make every number here vacuous"
    );
    assert!(
        !absent.is_empty(),
        "PREMISE of §G-17 gap 3: a real household emits FEWER than all 15 forms. If this ever fires, \
         the 'absent form floods the resolution' concern is moot and the note should say so."
    );
    assert!(
        tot_fields > 0,
        "the emitted forms must expose AcroForm fields, or the census has nothing to account for"
    );
}

/// ★★★ §G-17 gap 1 — **does `merge_copies` renaming actually bite?** Measured, not argued.
///
/// The one-return census above showed **zero** renamed fields, because that household's Form 8949
/// fits a single copy. This forces the overflow path (the 2024 revision grids 14 rows per part) and
/// asks the only question that matters: **do the emitted FQNs still match the map?**
///
/// If they do not, then any census that walks an EMITTED document must normalise the copy prefix, or
/// every paginating return reds with a screenful of phantom unaccounted fields — and a checker that
/// cries wolf on a normal return is one that gets muted.
#[test]
fn overflow_copies_rename_fields_so_an_emitted_census_must_normalise() {
    let row = |part| Form8949Row {
        part,
        box_: if part == Form8949Part::ShortTerm {
            Form8949Box::C
        } else {
            Form8949Box::F
        },
        box_needs_review: false,
        description: "1.00000000 BTC".into(),
        date_acquired: time::macros::date!(2020 - 01 - 02),
        date_sold: time::macros::date!(2024 - 05 - 01),
        proceeds: dec!(30000),
        cost_basis: dec!(10000),
        adjustment_code: String::new(),
        adjustment_amount: Usd::ZERO,
        gain: dec!(20000),
        wallet: WalletId::SelfCustody { label: "w".into() },
        disposition_kind: DisposeKind::Sell,
    };

    // One copy's worth, then well past it — the 2024 revision grids 14 rows per part.
    for (label, n) in [("single copy", 10usize), ("overflowed", 40usize)] {
        let rows: Vec<Form8949Row> = (0..n).map(|_| row(Form8949Part::ShortTerm)).collect();
        let pdf = btctax_forms::fill_form_8949(&rows, 2024).expect("8949 fills");
        let doc = load(&pdf).expect("parses");
        let fields = collect_fields(&doc).expect("fields");
        let map = mapped_fqns("f8949", 2024).expect("f8949 has a map");

        let exact = fields.iter().filter(|f| map.contains(&f.fqn)).count();
        let renamed = fields
            .iter()
            .filter(|f| !map.contains(&f.fqn))
            .filter(|f| {
                let n = normalise_copy(&f.fqn);
                map.iter().any(|k| normalise_copy(k) == n)
            })
            .count();

        println!(
            "  8949 {label:<12} rows={n:<3} fields={:<4} exact-match={exact:<4} renamed-copy={renamed}",
            fields.len()
        );

        if n > 14 {
            assert!(
                renamed > 0,
                "§G-17 gap 1 PREMISE: an overflowing 8949 must produce fields whose FQN differs from \
                 the map (merge_copies uniquifies the root /T). Got {} fields, {exact} exact, 0 \
                 renamed — if this fires, copy renaming does NOT happen and the normalisation \
                 requirement should be struck from §G-17.",
                fields.len()
            );
        }
    }
}

/// ★★★ §G-17 gap 2 — **does a return with no capital transactions omit Schedule D?**
///
/// If it does, Form 1040 line 7 is blank AND its "if not required, check here" box (`c1_23`, which
/// the map does not name) is blank — and on the printed page those two blanks are indistinguishable
/// from "we forgot Schedule D". The IRS supplied the provenance marker; this measures whether the
/// case that needs it actually arises.
#[test]
fn a_w2_only_return_shows_whether_the_schedule_d_case_arises() {
    let (ri, state) = btctax_core::tax::testonly::w2_only_household();
    let table = ty2024_table();
    let ar = assemble_absolute(&ri, &state, &ty2024_params(), &table, 2024);
    let details: BTreeMap<_, _> = BTreeMap::new();
    let pr = assemble_printed_return(&ri, &state, &details, &ar, &table, 2024, &[])
        .expect("w2_only assembles");
    // ★★ `sch_d` is NOT an Option — Schedule D is ALWAYS in the packet. `f8949` IS optional.
    println!(
        "  w2_only: schedule_d always present (not Option); f8949 emitted = {}",
        pr.forms.f8949.is_some()
    );
    let forms = fill_full_return(&pr, 2024).expect("fills");
    println!(
        "  w2_only emits {} forms: {:?}",
        forms.len(),
        forms.iter().map(|f| f.name.as_str()).collect::<Vec<_>>()
    );

    // ★★★ THE GAP-2 INSTANCE. No Schedule D is attached, so Form 1040 line 7 is blank — and the
    // form's OWN provenance marker for that case ("If not required, check here", `c1_23`) is
    // unmapped, so it is blank too. Two blanks that mean different things, rendered identically.
    let f1040 = forms
        .iter()
        .find(|f| f.name == "f1040")
        .expect("the 1040 is always emitted");
    let doc = load(&f1040.bytes).expect("parses");
    let fields = collect_fields(&doc).expect("fields");
    let line7 = fields.iter().find(|f| f.fqn.ends_with("f1_52[0]"));
    let notreq = fields.iter().find(|f| f.fqn.ends_with("c1_23[0]"));
    println!(
        "  line 7 amount value = {:?}",
        line7.and_then(|f| btctax_forms::testonly::text_value(&doc, f.id))
    );
    println!(
        "  line 7 'not required' checkbox present in PDF = {}, CHECKED = {:?}",
        notreq.is_some(),
        notreq.and_then(|f| btctax_forms::testonly::checkbox_on(&doc, f.id))
    );
}

/// ★ A DEMONSTRATION of Schedule 1-A Part IV's VIN comb — not an emitter. Schedule 1-A is not
/// emitted yet (B3 T2–T7 unstarted, no `.map.toml`), so this fills the AcroForm directly to answer
/// one measurable question: **does a 17-character VIN land one character per cell, and what does
/// `maxlen=17` do at the boundary?**
///
/// Run: `cargo test -p btctax-forms --test field_census_slice -- --ignored vin_demo --nocapture`
#[test]
#[ignore = "writes a demo PDF; run explicitly"]
fn vin_demo() {
    use btctax_forms::testonly as bf;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root");
    let pdf = root.join("design/forms/2025/f1040s1a--2025.pdf");
    let bytes = std::fs::read(&pdf).expect("the TY2025 Schedule 1-A (gitignored; re-fetchable)");
    let mut doc = bf::load(&bytes).expect("parses");
    bf::drop_xfa_and_set_needappearances(&mut doc).expect("appearances");
    let fields = bf::collect_fields(&doc).expect("fields");
    let idx = bf::index(&fields);

    // ★ Obviously-synthetic VINs: 17 chars, and VINs never contain I, O or Q.
    let vin_a = "ZZ9TESTVEHZZ00001"; // exactly 17
    let vin_b = "ZZ9TESTVEHZZ00002";
    assert_eq!(
        vin_a.len(),
        17,
        "a VIN is 17 characters — the comb's /MaxLen"
    );

    let w = |fqn: &str, v: &str| (fqn.to_string(), bf::FieldValue::Text(v.to_string()));
    let writes = vec![
        w(
            "form1[0].Page2[0].Table_Line22[0].Line22a[0].VIN-1_Comb[0].f2_01[0]",
            vin_a,
        ),
        w("form1[0].Page2[0].Table_Line22[0].Line22a[0].f2_02[0]", "0"),
        w(
            "form1[0].Page2[0].Table_Line22[0].Line22a[0].f2_03[0]",
            "2,400",
        ),
        w(
            "form1[0].Page2[0].Table_Line22[0].Line22b[0].VIN-2_Comb[0].f2_04[0]",
            vin_b,
        ),
        w("form1[0].Page2[0].Table_Line22[0].Line22b[0].f2_05[0]", "0"),
        w(
            "form1[0].Page2[0].Table_Line22[0].Line22b[0].f2_06[0]",
            "1,100",
        ),
        w("form1[0].Page2[0].f2_07[0]", "3,500"), // line 23 — "Add lines 22a and 22b, column (iii)"
    ];
    for (fqn, _) in &writes {
        assert!(idx.contains_key(fqn), "field missing from the PDF: {fqn}");
    }
    bf::apply_writes(&mut doc, &idx, &writes).expect("writes");
    bf::strip_nondeterminism(&mut doc);
    let out = bf::save(&mut doc).expect("save");
    let dest = std::env::var("VIN_DEMO_OUT").unwrap_or_else(|_| "/tmp/sch1a-vin-demo.pdf".into());
    std::fs::write(&dest, &out).expect("write");
    println!("  wrote {dest} — VIN a = {vin_a} ({} chars)", vin_a.len());
}
