//! §G-28/B1a — the Form 8995-A map, held against the form itself.
//!
//! ★★★ The map is groundwork: the emitter that writes through it is B1a's next commit. Landing an
//! unvalidated map would be dead weight that rots, so this pins it to the PDF and to the extracted
//! text NOW — every FQN must exist, sit in the column the form prints it in, and carry the form's own
//! sentence. `CLAUDE.md`: *"Conformance ⇒ test."*

use btctax_forms::testonly::{collect_fields, f8995a_pdf, load};

const MAP: &str = include_str!("../forms/2024/f8995a.map.toml");
const FORM_TEXT: &str = include_str!("../../../design/forms/extract/f8995a--2024.txt");

/// Every `lineNN = "…"` in the map, as (line label, FQN).
fn mapped() -> Vec<(String, String)> {
    MAP.lines()
        .filter_map(|l| {
            let (k, v) = l.split_once('=')?;
            let k = k.trim();
            let n = k.strip_prefix("line")?;
            let fqn = v.trim().trim_matches('"');
            (!n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
                .then(|| (n.to_string(), fqn.to_string()))
        })
        .collect()
}

/// ★★★ Every mapped FQN must EXIST in the blank PDF. A typo'd field name is invisible to a fill —
/// `lopdf` simply writes nothing — so a whole line would silently vanish from a filed return.
#[test]
fn every_mapped_field_exists_in_the_blank_form() {
    let doc = load(f8995a_pdf(2024).expect("the crate ships f8995a.pdf")).unwrap();
    let fields = collect_fields(&doc).unwrap();
    let present: std::collections::BTreeSet<&str> = fields.iter().map(|f| f.fqn.as_str()).collect();

    let m = mapped();
    assert_eq!(
        m.len(),
        14,
        "Part IV is lines 27-40 — fourteen lines: {m:?}"
    );
    for (line, fqn) in &m {
        assert!(
            present.contains(fqn.as_str()),
            "line {line} maps to {fqn}, which is NOT a field on the blank form — a fill would write \
             nothing and the line would vanish from the return"
        );
    }
}

/// ★★★ THE COLUMN PARTITION — the corroboration that makes the assignment more than an in-order zip.
///
/// Page 2 has exactly 14 non-table fields and Part IV has exactly 14 lines, so zipping them is
/// tempting and unfalsifiable. The form itself prints seven of those lines in the MID column and seven
/// in the AMOUNT column; if the map ever drifts, a line lands in the wrong cluster and this reds.
#[test]
fn each_line_sits_in_the_column_the_form_prints_it_in() {
    let doc = load(f8995a_pdf(2024).unwrap()).unwrap();
    let fields = collect_fields(&doc).unwrap();
    let x_of = |fqn: &str| -> f32 {
        fields
            .iter()
            .find(|f| f.fqn == fqn)
            .and_then(|f| f.rect.map(|r| (r[0] + r[2]) / 2.0))
            .unwrap_or_else(|| panic!("no rect for {fqn}"))
    };

    // Read off the form's own text: which Part IV lines carry a MID-column entry, and which the
    // far-right AMOUNT column.
    const MID: &[&str] = &["27", "28", "29", "30", "31", "33", "34"];
    const AMOUNT: &[&str] = &["32", "35", "36", "37", "38", "39", "40"];

    for (line, fqn) in mapped() {
        let cx = x_of(&fqn);
        if MID.contains(&line.as_str()) {
            assert!(
                (410.0..=482.0).contains(&cx),
                "line {line} ({fqn}) has centre x {cx}, outside the MID cluster [410,482]"
            );
        } else if AMOUNT.contains(&line.as_str()) {
            assert!(
                (504.0..=576.0).contains(&cx),
                "line {line} ({fqn}) has centre x {cx}, outside the AMOUNT cluster [504,576]"
            );
        } else {
            panic!("line {line} is in neither column list — the lists must cover Part IV exactly");
        }
    }
    assert_eq!(
        MID.len() + AMOUNT.len(),
        14,
        "the two lists partition Part IV"
    );
}

/// ★★ y must DESCEND as the line number ascends. The third corroboration, and the one that would catch
/// a transposition the column partition cannot see (two lines swapped inside one column).
#[test]
fn the_lines_descend_the_page_in_order() {
    let doc = load(f8995a_pdf(2024).unwrap()).unwrap();
    let fields = collect_fields(&doc).unwrap();
    let y_of = |fqn: &str| -> f32 {
        fields
            .iter()
            .find(|f| f.fqn == fqn)
            .and_then(|f| f.rect.map(|r| (r[1] + r[3]) / 2.0))
            .unwrap()
    };
    let mut m = mapped();
    m.sort_by_key(|(l, _)| l.parse::<u32>().unwrap());
    for w in m.windows(2) {
        let (a, b) = (&w[0], &w[1]);
        assert!(
            y_of(&a.1) > y_of(&b.1),
            "line {} sits at or below line {} on the page ({} vs {}) — the map is transposed",
            a.0,
            b.0,
            y_of(&a.1),
            y_of(&b.1)
        );
    }
}

/// ★★★ Every instruction the map quotes must be VERBATIM from the form. This is `CLAUDE.md`'s standing
/// root cause — Form 6251 line 33 shipped as *"Subtract line 32 from line 12"* where the form says 22 —
/// and a comment is exactly where that rot starts, because nothing executes a comment.
#[test]
fn every_quoted_instruction_is_verbatim_on_the_form() {
    let norm = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
    let form = norm(FORM_TEXT);
    // The map quotes each line's sentence in the `#` comment directly above it. Pull them back out.
    let mut checked = 0;
    let lines: Vec<&str> = MAP.lines().collect();
    for (i, l) in lines.iter().enumerate() {
        if !l.trim_start().starts_with("# ") {
            continue;
        }
        let Some(rest) = l.trim_start().strip_prefix("# ") else {
            continue;
        };
        // A quoted instruction starts `NN "` and may continue on following `#     "…"` lines.
        let Some((num, tail)) = rest.split_once(' ') else {
            continue;
        };
        if !(num.len() <= 2 && num.chars().all(|c| c.is_ascii_digit())) || !tail.starts_with('"') {
            continue;
        }
        let mut quote = tail.to_string();
        for cont in lines.iter().skip(i + 1) {
            let t = cont.trim_start();
            if !t.starts_with('#') || quote.matches('"').count() >= 2 {
                break;
            }
            quote.push(' ');
            quote.push_str(t.trim_start_matches('#').trim());
        }
        // ★ Between the FIRST and SECOND quote marks — not `trim_matches`, which would swallow a
        //   trailing annotation like `— PARENTHESIZED` that sits after the closing quote.
        let body = quote
            .split_once('"')
            .and_then(|(_, r)| r.split_once('"').map(|(inner, _)| inner))
            .unwrap_or("");
        let q = norm(body);
        if q.is_empty() {
            continue;
        }
        assert!(
            form.contains(&q),
            "line {num}'s quoted instruction is NOT verbatim on the form:\n      {q:?}"
        );
        checked += 1;
    }
    assert_eq!(
        checked, 14,
        "all fourteen Part IV instructions must be checked, not merely present"
    );
}
