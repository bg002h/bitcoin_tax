//! ★ Design r2 §10 step 5 — **the first measurement: which `Unwired` revisions PARSE today.**
//!
//! `line_set::Schema::Unwired` means "not yet verified against a struct"; the steps-2/3 review
//! measured (by key tree) that eight of the ten would parse into their 2024 structs. This test
//! measures it by actually parsing, and pins the answer as a ratchet: wiring a revision at step 5
//! (deleting it from the `Unwired` arm) requires that its parse be `Ok` here; leaving a revision
//! `Unwired` that parses needs the stated reason (the label/extract join has not been run for it).
//! A revision moving from `Ok` to `Err` or back without this table changing is a red.
use btctax_forms::bundled::{map_text, Stem};
use btctax_forms::line_set::{schema, LineSet, Schema};
use btctax_forms::testonly::*;

/// Try the would-be 2024 struct on a TY2025 map. `None` = no struct exists for the stem at all.
fn parses_into_2024_struct(stem: Stem) -> Option<Result<(), String>> {
    let text = map_text(stem, 2025)?;
    let first = |e: toml::de::Error| e.to_string().lines().next().unwrap_or("").to_string();
    Some(match stem {
        Stem::F1040s2 => Schedule2Map::parse(text).map(|_| ()).map_err(first),
        Stem::F1040s3 => Schedule3Map::parse(text).map(|_| ()).map_err(first),
        Stem::F1040sa => ScheduleAMap::parse(text).map(|_| ()).map_err(first),
        Stem::F1040sb => ScheduleBMap::parse(text).map(|_| ()).map_err(first),
        Stem::F1040sc => ScheduleCMap::parse(text).map(|_| ()).map_err(first),
        Stem::F6251 => Form6251Map::parse(text).map(|_| ()).map_err(first),
        Stem::F8959 => Form8959Map::parse(text).map(|_| ()).map_err(first),
        Stem::F8960 => Form8960Map::parse(text).map(|_| ()).map_err(first),
        Stem::F8995 => Form8995Map::parse(text).map(|_| ()).map_err(first),
        Stem::F1040s1a => return None, // no struct exists (Schedule1AMap: count 0)
        _ => return None,
    })
}

#[test]
fn the_unwired_revisions_parse_status_is_exactly_as_recorded() {
    let mut measured: Vec<(String, &'static str)> = Vec::new();
    // The ten TY2025 revisions step 5 started from — measured whether wired or not, so this table
    // is the full record (after step 5, eight are wired and two remain `Unwired`).
    let ten = [
        "f1040s1a/2025",
        "f1040s2/2025",
        "f1040s3/2025",
        "f1040sa/2025",
        "f1040sb/2025",
        "f1040sc/2025",
        "f6251/2025",
        "f8959/2025",
        "f8960/2025",
        "f8995/2025",
    ];
    for name in ten {
        let ls = LineSet::parse(name).unwrap();
        let (stem_s, year) = ls.as_str().split_once('/').unwrap();
        assert_eq!(year, "2025", "every Unwired revision today is TY2025");
        let stem = Stem::from_file_stem(stem_s).unwrap();
        let verdict = match parses_into_2024_struct(stem) {
            None => "NO STRUCT",
            Some(Ok(())) => "PARSES",
            Some(Err(e)) => {
                eprintln!("{stem_s}/2025 does not parse: {e}");
                "DOES NOT PARSE"
            }
        };
        measured.push((ls.as_str().to_string(), verdict));
        // Wired ⇔ parses: a wired revision must parse; one that parses must be wired.
        assert_eq!(
            schema(ls) != Schema::Unwired,
            verdict == "PARSES",
            "{name}: wired={} but verdict={verdict}",
            schema(ls) != Schema::Unwired
        );
    }
    let expected: Vec<(String, &str)> = [
        ("f1040s1a/2025", "NO STRUCT"),
        ("f1040s2/2025", "PARSES"),
        ("f1040s3/2025", "PARSES"),
        ("f1040sa/2025", "PARSES"),
        ("f1040sb/2025", "PARSES"),
        ("f1040sc/2025", "PARSES"),
        ("f6251/2025", "DOES NOT PARSE"),
        ("f8959/2025", "PARSES"),
        ("f8960/2025", "PARSES"),
        ("f8995/2025", "PARSES"),
    ]
    .iter()
    .map(|(s, v)| (s.to_string(), *v))
    .collect();
    assert_eq!(measured, expected, "the parse status of the ten TY2025 revisions moved — update the table AND the Unwired arm together");
}
