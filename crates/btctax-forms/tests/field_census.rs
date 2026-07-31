//! §G-13 — **the FIELD CENSUS gate**: every AcroForm field is accounted for exactly once.
//!
//! ★★ **The missing direction.** `verify.rs::no_unmapped_filled` already asserts every field
//! carrying a VALUE is authorised — it stops us writing where we should not. Nothing asserted the
//! other way: that every field is either mapped (we fill it) or deliberately named as one we do not.
//! One direction stops stray writes; this one stops **silent omissions**, and omission is the
//! direction that costs a filer money.
//!
//! ★★★ **This is not new knowledge — it is knowledge made enforceable.** `Form8959Map`'s doc comment
//! already said, in prose: *"Lines 2/3 (Form 4137 / Form 8919) and all of Part III plus line 23
//! (RRTA) are unmodeled and are deliberately absent."* `f1040.map.toml` says the same of the spouse's
//! IP PIN: *"ReturnInputs does not capture one, so it is left BLANK, never guessed."* Both are exactly
//! the provenance record this census wants, written where nothing could check them.
//!
//! **The rule:** `(map FQNs) ∪ (census FQNs) == (the PDF's AcroForm FQNs)`, exactly. A field in
//! neither is the "we forgot this line" defect; a field in both is a contradiction.

use btctax_forms::testonly::{collect_fields, load};
use std::collections::BTreeSet;

/// Forms whose `[census]` section is not yet written. **Shrink-only** — the same ratchet shape as
/// `cite_check::AUTHORITY_NOT_YET_ARCHIVED`.
///
/// ★ Being on this list is not "exempt": it is *"nobody has classified this form's fields yet"*,
/// which is a recorded gap rather than a silent one. Measured 2026-07-30: 496 of 1158 TY2024 fields
/// have no recorded decision, and this list is how that number is burned down one form at a time.
/// Forms whose `[census]` section IS written. This list and [`CENSUS_NOT_YET_WRITTEN`] must together
/// partition the 15 census keys — so a form can never be dropped from both and silently escape.
const CENSUSED: &[&str] = &[
    "f8959",
    "schedule_se",
    "f1040sb",
    "f8949",
    "f8995",
    "f1040sa",
    "f8960",
    "schedule_d",
    "f1040s3",
    "f8275",
    "f1040s2",
    "f8283",
    "f1040",
    "f1040s1",
    "f1040sc",
];

const CENSUS_NOT_YET_WRITTEN: &[&str] = &[];

/// Every FQN-shaped string in a map file, split into the mapped set and the `[census]` set.
///
/// ★ Text-scanned rather than deserialized on purpose: the census must see **every** FQN the file
/// names, including any a future typed struct forgets to model. A parser that only sees what its
/// struct declares would go blind exactly where this test needs sight.
fn map_and_census(stem: &str, year: i32) -> Option<(BTreeSet<String>, BTreeSet<String>)> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("forms")
        .join(year.to_string())
        .join(format!("{stem}.map.toml"));
    let text = std::fs::read_to_string(path).ok()?;
    let (mut mapped, mut census) = (BTreeSet::new(), BTreeSet::new());
    let mut in_census = false;
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with('[') && !t.starts_with("[[") {
            in_census = t == "[census]";
        }
        for chunk in line.split('"').skip(1).step_by(2) {
            if chunk.contains("[0]") && chunk.contains('.') {
                if in_census {
                    census.insert(chunk.to_string());
                } else {
                    mapped.insert(chunk.to_string());
                }
            }
        }
    }
    Some((mapped, census))
}

fn acroform_fqns(stem: &str, year: i32) -> Option<BTreeSet<String>> {
    let pdf = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("forms")
        .join(year.to_string())
        .join(format!("{stem}.pdf"));
    let bytes = std::fs::read(pdf).ok()?;
    let doc = load(&bytes).ok()?;
    Some(
        collect_fields(&doc)
            .ok()?
            .into_iter()
            .map(|f| f.fqn)
            .collect(),
    )
}

/// ★★★ **THE GATE.** For every form whose census is written, the map and the census together must
/// account for the PDF's field set exactly.
#[test]
fn census_accounts_for_every_field() {
    let year = 2024;
    let mut checked = 0;

    for stem in CENSUSED {
        let (mapped, census) = map_and_census(stem, year).expect("map file exists");
        let actual = acroform_fqns(stem, year).expect("pdf exists");

        let accounted: BTreeSet<String> = mapped.union(&census).cloned().collect();

        let unaccounted: Vec<&String> = actual.difference(&accounted).collect();
        assert!(
            unaccounted.is_empty(),
            "{stem}: {} field(s) are in NEITHER the map nor the [census] — this is the \"we forgot \
             this line\" defect, invisible on the printed page and to both oracles: {unaccounted:#?}",
            unaccounted.len()
        );

        let both: Vec<&String> = mapped.intersection(&census).collect();
        assert!(
            both.is_empty(),
            "{stem}: {both:?} are BOTH mapped and censused — a field cannot be one we fill and one \
             we deliberately leave blank"
        );

        let phantom: Vec<&String> = accounted.difference(&actual).collect();
        assert!(
            phantom.is_empty(),
            "{stem}: {phantom:?} are named by the map or census but do not exist in the PDF — a \
             stale entry, which is how a closed gap silently reopens"
        );
        checked += 1;
    }

    assert!(
        checked > 0,
        "the gate must actually check a form, or it passes by finding nothing"
    );
}

/// ★★★ **`rule = "gap"` is a countable DEFECT, not an exemption — and it may only shrink.**
///
/// A `gap` is a field btctax **cannot honestly account for**: a required declaration it never asks.
/// Censusing `schedule_se` found the first one — line A, the Form 4361 minister declaration. A
/// minister with $400+ of other self-employment earnings would file an incomplete Schedule SE and
/// nothing would say so.
///
/// ★ Without this test, `gap` would be exactly the escape hatch the census exists to remove: a way to
/// mark a field "accounted for" while accounting for nothing. Pinning the count means a new gap
/// cannot be added quietly, and closing one (by adding a `QuestionId`) must lower the number.
#[test]
fn recorded_gaps_may_only_shrink() {
    /// Every `rule = "gap"` in the censused forms. Closing one means deleting its census entry and
    /// mapping the field to a real question — at which point this comes down.
    // ★ Counted per FIELD, not per question: the FBAR pair is TWO fields (the Yes and No halves of
    // one radio) for ONE logical question. Fields is the mechanical unit — a question-level count
    // would need someone to decide what "one question" means, which is exactly the judgement this
    // ratchet is meant to avoid depending on.
    const GAPS: usize = 16;

    let mut found = Vec::new();
    for stem in CENSUSED {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("forms")
            .join("2024")
            .join(format!("{stem}.map.toml"));
        let text = std::fs::read_to_string(&path).expect("map file");
        for line in text.lines() {
            // ★ Skip comments — the section's own legend explains `rule = "gap"`, and counting that
            // line made this test report 2 gaps where the form has 1. A census that miscounts its
            // own defects is worse than none.
            if line.trim_start().starts_with('#') {
                continue;
            }
            if line.contains(r#"rule = "gap""#) {
                found.push(format!("{stem}: {}", line.split('"').nth(1).unwrap_or("?")));
            }
        }
    }
    assert_eq!(
        found.len(),
        GAPS,
        "recorded gaps changed (pinned {GAPS}). A gap is a field btctax CANNOT honestly account for \
         — a required declaration it never asks — so the count must only go DOWN, and it goes down \
         by adding a QuestionId, never by deleting the record. Found: {found:#?}"
    );
}

/// ★★ **The ratchet may only shrink.** A form leaves `CENSUS_NOT_YET_WRITTEN` by gaining a `[census]`
/// section — never by being quietly dropped from the list.
#[test]
fn the_two_lists_partition_every_form() {
    // ★★ The 15 forms `fill_full_return` can emit (btctax-forms/tests/census.rs is the authority for
    // this set). Every one must be on exactly ONE of the two lists: censused, or recorded as not yet
    // censused. A form on neither would escape the gate entirely — the census's own version of the
    // defect it exists to catch.
    // ★ The authority for the 15 forms `fill_full_return` can emit — btctax-forms/tests/census.rs
    // owns that set. This list must NEVER shrink to match a shorter working list; the compiler
    // enforced that when a careless edit dropped two entries.
    const ALL: [&str; 15] = [
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
    let censused: BTreeSet<&str> = CENSUSED.iter().copied().collect();
    let pending: BTreeSet<&str> = CENSUS_NOT_YET_WRITTEN.iter().copied().collect();

    let both: Vec<&&str> = censused.intersection(&pending).collect();
    assert!(both.is_empty(), "{both:?} are on BOTH lists");

    let covered: BTreeSet<&str> = censused.union(&pending).copied().collect();
    let missing: Vec<&&str> = ALL.iter().filter(|f| !covered.contains(**f)).collect();
    assert!(
        missing.is_empty(),
        "{missing:?} are on NEITHER list — a form that is neither censused nor recorded as pending          escapes the gate silently"
    );
}

/// ★★ **The ratchet may only shrink.**
#[test]
fn the_uncensused_list_may_only_shrink() {
    // ★★ THE RATCHET IS CLOSED (2026-07-30). It ran 15 → 0: every form `fill_full_return` can emit
    // now has a `[census]` section, so the bound is EMPTINESS, not a slack allowance. A 16th form
    // cannot slip in uncensused — `the_two_lists_partition_every_form` forces it onto one of the two
    // lists, this assertion rejects the uncensused one, and `census_accounts_for_every_field` rejects
    // the other unless its fields are actually accounted for. Both routes fail closed.
    assert!(
        CENSUS_NOT_YET_WRITTEN.is_empty(),
        "the uncensused list is no longer empty ({:?}) — every form must arrive WITH its census, or \
         the count of unaccounted fields silently rises again",
        CENSUS_NOT_YET_WRITTEN
    );

    // ★ Nothing may be both censused and listed as uncensused — a stale excuse is how a closed gap
    // reopens for the next form.
    for stem in CENSUS_NOT_YET_WRITTEN {
        if let Some((_, census)) = map_and_census(stem, 2024) {
            assert!(
                census.is_empty(),
                "{stem} HAS a [census] section but is still listed as not-yet-written — remove it \
                 from CENSUS_NOT_YET_WRITTEN so the ratchet actually tightens"
            );
        }
    }
}
