//! §G-28/B1a+B1b — the Form 8995-A map, held against the form itself.
//!
//! ★★★ Every FQN must exist on the blank PDF, sit in the column the form prints it in, descend the
//! page in line order, and carry the form's own sentence verbatim. `CLAUDE.md`: *"Conformance ⇒
//! test."* B1a pinned Part IV this way; B1b extends the same four checks over Parts I–III.

use btctax_forms::testonly::{collect_fields, f8995a_pdf, load};

const MAP: &str = include_str!("../forms/2024/f8995a.map.toml");
const FORM_TEXT: &str = include_str!("../../../design/forms/extract/f8995a--2024.txt");

/// Every `lineNN = "…"` in the map, as (line label, FQN) — across ALL parts.
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

/// The lines belonging to one part, by number.
fn part(lines: &[u32]) -> Vec<(String, String)> {
    let all = mapped();
    lines
        .iter()
        .map(|n| {
            all.iter()
                .find(|(l, _)| l.parse::<u32>() == Ok(*n))
                .unwrap_or_else(|| panic!("line {n} is not in the map"))
                .clone()
        })
        .collect()
}

const PART_II: &[u32] = &[2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
const PART_III: &[u32] = &[17, 18, 19, 20, 21, 22, 23, 24, 25, 26];
const PART_IV: &[u32] = &[27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40];

/// Part I row A's five column FQNs, in the order the form prints them: (a) (b) (c) (d) (e).
/// ★ Read out of the map by KEY, not by position — the whole point of the assignment is that the
///   PDF's field order and the form's column order DISAGREE.
fn part_i_row_a() -> Vec<(&'static str, String)> {
    let by_key = |key: &str| -> String {
        let line = MAP
            .lines()
            .find(|l| l.trim_start().starts_with(&format!("{key} =")))
            .unwrap_or_else(|| panic!("{key} is not in [part1_row_a]"));
        // Either `key = "fqn"` or `key = { field = "fqn", on = "1" }`.
        let after = line.split_once('=').unwrap().1;
        let inner = after.split_once("field =").map_or(after, |(_, r)| r);
        inner
            .trim()
            .trim_start_matches('{')
            .trim()
            .split('"')
            .nth(1)
            .unwrap()
            .to_string()
    };
    vec![
        ("a", by_key("name")),
        ("b", by_key("specified_service")),
        ("c", by_key("aggregation")),
        ("d", by_key("tin")),
        ("e", by_key("patron")),
    ]
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
        39,
        "Parts II (2-16), III (17-26) and IV (27-40) are 39 numbered lines: {m:?}"
    );
    for (col, fqn) in part_i_row_a() {
        assert!(
            present.contains(fqn.as_str()),
            "Part I column ({col}) maps to {fqn}, which is NOT a field on the blank form"
        );
    }
    for (line, fqn) in &m {
        assert!(
            present.contains(fqn.as_str()),
            "line {line} maps to {fqn}, which is NOT a field on the blank form — a fill would write \
             nothing and the line would vanish from the return"
        );
    }
}

/// ★★★ THE COLUMN PARTITION — the corroboration that makes each assignment more than an in-order zip.
///
/// Every part is a grid whose fields could be zipped to lines in order, unfalsifiably. The form itself
/// says which column each figure prints in, and this reds the moment a line lands in the wrong one.
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
    let band = |label: String, fqn: &str, lo: f32, hi: f32, what: &str| {
        let cx = x_of(fqn);
        assert!(
            (lo..=hi).contains(&cx),
            "{label} ({fqn}) has centre x {cx}, outside the {what} cluster [{lo},{hi}]"
        );
    };

    // ── Part IV — the form's own text: seven lines in MID, seven in AMOUNT. ────────────────────────
    const MID: &[&str] = &["27", "28", "29", "30", "31", "33", "34"];
    const AMOUNT: &[&str] = &["32", "35", "36", "37", "38", "39", "40"];
    for (line, fqn) in part(PART_IV) {
        if MID.contains(&line.as_str()) {
            band(format!("line {line}"), &fqn, 410.0, 482.0, "MID");
        } else if AMOUNT.contains(&line.as_str()) {
            band(format!("line {line}"), &fqn, 504.0, 576.0, "AMOUNT");
        } else {
            panic!("line {line} is in neither column list — the lists must cover Part IV exactly");
        }
    }
    assert_eq!(
        MID.len() + AMOUNT.len(),
        14,
        "the two lists partition Part IV"
    );

    // ── Part II — every mapped line is COLUMN A, the first widget of each row triple. ──────────────
    for (line, fqn) in part(PART_II) {
        band(
            format!("Part II line {line}"),
            &fqn,
            360.0,
            432.0,
            "column A",
        );
    }

    // ── Part III — lines 17, 18, 19, 25, 26 are column A; 20-24 are the single `Ln` entry box, which
    //    sits WELL LEFT of column A. That separation is the check: if a line 20-24 mapping ever drifted
    //    onto an `_RO` mirror it would land in the column-A band and this reds.
    for (line, fqn) in part(PART_III) {
        let n: u32 = line.parse().unwrap();
        if (20..=24).contains(&n) {
            band(
                format!("Part III line {line}"),
                &fqn,
                266.0,
                338.0,
                "Ln entry",
            );
            assert!(
                !fqn.contains("_RO"),
                "Part III line {line} maps to a READ-ONLY mirror ({fqn}); the form's JavaScript \
                 fills those and btctax never runs it"
            );
        } else {
            band(
                format!("Part III line {line}"),
                &fqn,
                360.0,
                432.0,
                "column A",
            );
        }
    }

    // ── Part I row A — the five columns, strictly left to right in the form's own order. ───────────
    let xs: Vec<(&str, f32)> = part_i_row_a().iter().map(|(c, f)| (*c, x_of(f))).collect();
    for w in xs.windows(2) {
        assert!(
            w[0].1 < w[1].1,
            "Part I column ({}) sits at or right of column ({}) — x {} vs {}. The form prints them \
             (a) name, (b) specified service, (c) aggregation, (d) TIN, (e) patron, and the PDF's \
             field ORDER is different, which is exactly the transposition this catches",
            w[0].0,
            w[1].0,
            w[0].1,
            w[1].1
        );
    }
}

/// ★★ y must DESCEND as the line number ascends, WITHIN each part. The corroboration that catches a
/// transposition the column partition cannot see (two lines swapped inside one column).
///
/// ★ Per part, because Part II ends on page 1 and Part III begins on page 2, where y resets to the top.
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
    for lines in [PART_II, PART_III, PART_IV] {
        let mut m = part(lines);
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
    // ★ Part I's row A sits ABOVE every Part II line — the two grids must not have been swapped.
    let row_a_y = y_of(&part_i_row_a()[0].1);
    let line2_y = y_of(&part(&[2])[0].1);
    assert!(
        row_a_y > line2_y,
        "Part I row A ({row_a_y}) must print above Part II line 2 ({line2_y})"
    );
}

/// ★★★ Every instruction the map quotes must be VERBATIM from the form. This is `CLAUDE.md`'s standing
/// root cause — Form 6251 line 33 shipped as *"Subtract line 32 from line 12"* where the form says 22 —
/// and a comment is exactly where that rot starts, because nothing executes a comment.
#[test]
fn every_quoted_instruction_is_verbatim_on_the_form() {
    let norm = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
    // ★★★ THE TWO-ROW COLUMN HEADER, REJOINED BY COLUMN.
    //
    //     Part I's column captions are printed as a table header wrapped over two rows: the extract
    //     has `(b) Check if   (c) Check if   (d) Taxpayer   (e) Check if` on one line and
    //     `specified service   aggregation   identification number   patron` two lines below. So
    //     `"Check if specified service"` — the form's actual caption — is NOWHERE contiguous in the
    //     text layer, and a naive `contains` would reject a FAITHFUL quote.
    //
    //     ★★ Rejoined MECHANICALLY: split both rows on runs of 2+ spaces (the column separator this
    //     layout uses) and zip them by position. That is the same thing the eye does reading the
    //     printed page, and it is derived from the form's own layout rather than from a hand-list of
    //     the four captions we happen to expect — which would be the "enumerate the outcomes you
    //     happened to see" failure this repo keeps paying for.
    let cells = |l: &str| -> Vec<String> {
        l.split("  ")
            .map(str::trim)
            .filter(|c| !c.is_empty())
            .map(str::to_string)
            .collect()
    };
    let lines: Vec<&str> = FORM_TEXT.lines().collect();
    let mut rejoined = String::new();
    for (i, l) in lines.iter().enumerate() {
        if !l.contains("(b) Check if") {
            continue;
        }
        // ★ The continuation is the FIRST of the next three lines with the same cell count — and the
        //   `break` is the point: without it every matching line within three is zipped in, so a second
        //   equal-arity row would silently append a wrong reconstruction. Today exactly one matches.
        for cont in lines.iter().skip(i + 1).take(3) {
            let (top, bot) = (cells(l), cells(cont));
            if top.len() == bot.len() && top.len() > 1 {
                for (t, b) in top.iter().zip(bot.iter()) {
                    rejoined.push_str(&format!("{t} {b}\n"));
                }
                break;
            }
        }
    }
    let form = format!("{} {}", norm(FORM_TEXT), norm(&rejoined));
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
        // ★★ A label is either a line number (`27`) or a Part I COLUMN (`1(a)`). The original parser
        //    accepted only the first, so all five Part I quotes were silently skipped — and the
        //    `checked` count was set to exactly the number of numbered lines, so their absence could
        //    not show up there either. A checker with a blind region and a count that fits the
        //    sighted region is the shape of `cite-check` before its own kill test.
        let label_ok = !num.is_empty()
            && num.len() <= 5
            && num.chars().next().is_some_and(|c| c.is_ascii_digit())
            && num
                .chars()
                .all(|c| c.is_ascii_digit() || c == '(' || c == ')' || c.is_ascii_lowercase());
        if !label_ok || !tail.starts_with('"') {
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
        checked, 44,
        "all FORTY-FOUR quoted captions must be checked — Parts II, III and IV's 39 numbered lines \
         PLUS Part I's five columns. The count is the guard against a quote that stops being parsed \
         and therefore silently stops being verified, which is exactly what happened to Part I"
    );
}
