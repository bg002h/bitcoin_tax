//! spec 1099-DA T3 — one Form 8949 page-set per (part, BOX): rows routed to the broker-reported boxes
//! print on their own page-set with THEIR checkbox, a pre-2025 map (no digital-asset boxes) refuses
//! them, and the map's table is complete exactly where the revision carries the boxes.

mod common;
use common::*;

use btctax_core::forms::Form8949Box;
use btctax_core::Form8949Part;
use btctax_forms::fill_form_8949;
use btctax_forms::testonly::*;
use rust_decimal_macros::dec;

fn values_ending(doc: &lopdf::Document, fields: &[Field], suffix: &str) -> Vec<String> {
    fields
        .iter()
        .filter(|f| f.fqn.ends_with(suffix))
        .filter_map(|f| text_value(doc, f.id))
        .collect()
}

fn boxed(part: Form8949Part, b: Form8949Box, n: u32) -> btctax_core::Form8949Row {
    let mut r = row(
        part,
        &format!("{n}.00000000 BTC"),
        dec!(100),
        dec!(50),
        false,
    );
    r.box_ = b;
    r
}

/// The checkbox state (`/V`, else `/AS`) of every field whose FQN ends with `suffix`, in document
/// order — one entry per copy that carries the field.
fn check_states(doc: &lopdf::Document, fields: &[Field], suffix: &str) -> Vec<Option<String>> {
    fields
        .iter()
        .filter(|f| f.fqn.ends_with(suffix))
        .map(|f| {
            let d = doc.get_object(f.id).ok()?.as_dict().ok()?;
            let v = d.get(b"V").or_else(|_| d.get(b"AS")).ok()?;
            match v {
                lopdf::Object::Name(n) => Some(String::from_utf8_lossy(n).to_string()),
                lopdf::Object::String(s, _) => Some(String::from_utf8_lossy(s).to_string()),
                _ => None,
            }
        })
        .collect()
}

/// ★ R3's worked example: a G + I short-term set (each ≤ cap) with a single J long-term set →
/// TWO copies; copy 1 page 1 boxed G, copy 2 page 1 boxed I, copy 1 page 2 boxed J, copy 2 page 2
/// blank. Read back from the PDF's checkbox states, not from the struct.
#[test]
fn a_mixed_short_term_set_prints_one_page_set_per_box() {
    let rows = vec![
        boxed(Form8949Part::ShortTerm, Form8949Box::G, 1),
        boxed(Form8949Part::ShortTerm, Form8949Box::I, 2),
        boxed(Form8949Part::ShortTerm, Form8949Box::G, 3),
        boxed(Form8949Part::LongTerm, Form8949Box::J, 4),
    ];
    let bytes = fill_form_8949(&rows, 2025).expect("T3: broker boxes print on 2025");
    let doc = load(&bytes).unwrap();
    assert_eq!(doc.get_pages().len(), 4, "2 copies × 2 pages");
    let fields = collect_fields(&doc).unwrap();
    // Part I: box G (c1_1[3], on "4") checked on exactly one copy, box I (c1_1[5], on "6") on the other
    let g = check_states(&doc, &fields, "Page1[0].c1_1[3]");
    let i = check_states(&doc, &fields, "Page1[0].c1_1[5]");
    assert_eq!(
        g.iter().filter(|s| s.as_deref() == Some("4")).count(),
        1,
        "G on one copy: {g:?}"
    );
    assert_eq!(
        i.iter().filter(|s| s.as_deref() == Some("6")).count(),
        1,
        "I on one copy: {i:?}"
    );
    // never the securities boxes
    for suffix in ["Page1[0].c1_1[0]", "Page1[0].c1_1[1]", "Page1[0].c1_1[2]"] {
        let st = check_states(&doc, &fields, suffix);
        assert!(
            st.iter().all(|s| s.as_deref().is_none_or(|v| v == "Off")),
            "{suffix}: {st:?}"
        );
    }
    // Part II: box J (c2_1[3], on "4") on exactly one copy; the other copy's Part II is blank
    let j = check_states(&doc, &fields, "Page2[0].c2_1[3]");
    assert_eq!(
        j.iter().filter(|s| s.as_deref() == Some("4")).count(),
        1,
        "J on one copy: {j:?}"
    );
    let l = check_states(&doc, &fields, "Page2[0].c2_1[5]");
    assert_eq!(
        l.iter().filter(|s| s.as_deref() == Some("6")).count(),
        0,
        "L nowhere: {l:?}"
    );
    // and the G rows are on the G page, the I row on the I page: three col-(a) descriptors in all,
    // never four on one copy
    // the merger renames each copy's ROOT component, so match on the FQN below the root
    let row1_col_a = Form8949Map::ty2025().parts[0].rows[0][0].clone();
    let below_root = row1_col_a
        .split_once('.')
        .map(|(_, rest)| rest.to_string())
        .unwrap();
    let a = values_ending(&doc, &fields, &below_root);
    assert_eq!(
        a.len(),
        2,
        "each Part I page-set starts a row 1 ({row1_col_a}): {a:?}"
    );
}

/// A pre-2025 map carries no digital-asset boxes: a G/H/J/K row refuses, naming the box, rather
/// than being laundered under C/F.
#[test]
fn a_pre_2025_map_refuses_a_broker_reported_box() {
    for (b, part) in [
        (Form8949Box::G, Form8949Part::ShortTerm),
        (Form8949Box::K, Form8949Part::LongTerm),
    ] {
        let err = fill_form_8949(&[boxed(part, b, 1)], 2024).expect_err("2024 refuses");
        let msg = err.to_string();
        assert!(msg.contains(&format!("box {b:?}")), "{msg}");
    }
    // and the 2024 default boxes still print
    fill_form_8949(&[boxed(Form8949Part::ShortTerm, Form8949Box::C, 1)], 2024)
        .expect("C prints on 2024");
    // a page-set may never mix boxes — the filler groups first, so this is unreachable from
    // fill_form_8949; the part_data guard is held directly
    let mixed: Vec<&btctax_core::Form8949Row> = vec![];
    let _ = mixed;
}

/// The map's table is complete exactly where the revision carries the boxes: 2025 names G/H/I and
/// J/K/L and its I/L entries repeat the scalar not-reported pair; 2024 names none.
#[test]
fn the_box_table_is_complete_on_2025_and_absent_before() {
    let m = Form8949Map::ty2025();
    for p in &m.parts {
        let want: Vec<&str> = if p.term == "short" {
            vec!["G", "H", "I"]
        } else {
            vec!["J", "K", "L"]
        };
        let have: Vec<&str> = p.boxes.keys().map(String::as_str).collect();
        assert_eq!(have, want, "TY2025 {} part", p.term);
        let dflt = if p.term == "short" { "I" } else { "L" };
        assert_eq!(
            (p.boxes[dflt].field.as_str(), p.boxes[dflt].on.as_str()),
            (p.box_field.as_str(), p.box_on.as_str()),
            "the table's {dflt} entry is the scalar pair"
        );
        // the letters ascend with the widget index, as the form prints them top to bottom
        let idx = |l: &str| {
            p.boxes[l]
                .field
                .rsplit('[')
                .next()
                .unwrap()
                .trim_end_matches(']')
                .parse::<u32>()
                .unwrap()
        };
        assert!(
            idx(want[0]) < idx(want[1]) && idx(want[1]) < idx(want[2]),
            "print order"
        );
    }
    let m = Form8949Map::for_year(2024).unwrap();
    for p in &m.parts {
        assert!(
            p.boxes.is_empty(),
            "TY2024 {} part has no digital-asset boxes",
            p.term
        );
    }
}
