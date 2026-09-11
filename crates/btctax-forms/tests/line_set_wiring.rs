//! ★ Design r2 §10 step 5 — **every bundled revision parses into the struct its OWN row names.**
//!
//! `line_set::Schema::Unwired` means "no struct parses this revision"; every other `Schema` names one,
//! and [`btctax_forms::testonly::parses_into_its_schema`] is the exhaustive-over-`Schema` dispatch that
//! actually tries it. This test measures the answer for the whole revision set and pins it as a
//! ratchet: wiring a revision (deleting it from the `Unwired` arm) requires that its parse be `Ok`
//! here, and leaving one `Unwired` requires the stated reason in [`Schema::Unwired`]'s doc comment. A
//! revision moving from `Ok` to `Err` or back without this table changing is a red.
//!
//! ## ★★★ What this file used to get wrong, and why it is worth recording
//!
//! Until 2026-09-11 the helper was `parses_into_2024_struct`: a `match` on nine `Stem`s that hardcoded
//! *"the 2024 struct"* as the thing a TY2025 map ought to parse into. That was correct on the day it
//! was written — eight of the ten TY2025 maps ARE constants-only revisions of their 2024 line sets —
//! and it became wrong the moment a revision got a struct of its **own**. `f6251/2025` splits Part I
//! line 1 into 1a/1b and parses into `Form6251ObbbaMap`; the old helper would have demanded that the
//! 1a/1b map parse into the single-line-1 TY2024 struct, i.e. it would have asserted the exact defect
//! `deny_unknown_fields` exists to prevent. `CLAUDE.md`: *"derive the list, or make the compiler hold
//! it — never type one beside a set that grows."* The list is now derived from each revision's own row,
//! and the dispatch is `_`-free over `Schema`, so a new struct is a build error rather than a silent
//! gap. The iteration is `LineSet::ALL`, so it is no longer a hand-list of ten either.
use btctax_forms::line_set::{schema, LineSet, Schema};
use btctax_forms::testonly::parses_into_its_schema;

/// The revisions this build knows of that **no struct parses**, each with the reason it stays that
/// way. Shrink-only: wiring one edits this list down, and a new unwired revision cannot appear
/// without editing it up.
///
/// 2 → 1 on 2026-09-11. `f6251/2025` was wired to `Form6251ObbbaMap` (design r2 §10 step 5, completed).
/// The one that remains is a **settled destination, not a queue position**: owner ruling 2026-09-11
/// says TY2025 is never filed with this software, and TY2026's Schedule 1-A is a rebuild (10 of 219
/// fields survive), so a struct written against the TY2025 revision would be thrown away.
const NO_STRUCT: &[(&str, &str)] = &[(
    "f1040s1a/2025",
    "no struct exists: Schedule 1-A is P2's missing 17th form, and under the owner's 2026-09-11 \
     ruling no TY2025 Schedule 1-A will ever be printed",
)];

#[test]
fn every_revision_parses_into_the_struct_its_own_row_names() {
    let mut wired_and_parses = 0usize;
    let mut no_struct: Vec<&str> = Vec::new();
    let mut failures: Vec<String> = Vec::new();

    for ls in LineSet::ALL {
        let unwired = schema(*ls) == Schema::Unwired;
        match parses_into_its_schema(*ls) {
            // No struct named, or no bundled map — `Unwired` is the first and the only one today.
            None => {
                assert!(
                    unwired,
                    "{}: no parse was attempted although its schema names a struct ({:?}) — either \
                     the map is not bundled or the dispatch has gone blind, and a blind dispatch \
                     would make this whole test vacuously green",
                    ls.as_str(),
                    schema(*ls)
                );
                no_struct.push(ls.as_str());
            }
            Some(Ok(())) => {
                assert!(
                    !unwired,
                    "{}: it PARSES into a struct and yet is recorded `Unwired` — wire it or record \
                     why not",
                    ls.as_str()
                );
                wired_and_parses += 1;
            }
            Some(Err(e)) => failures.push(format!("{}: {e}", ls.as_str())),
        }
    }

    assert!(
        failures.is_empty(),
        "these revisions name a struct that REFUSES their own bundled map — a map and the struct \
         that is supposed to transcribe it have diverged:\n  {}",
        failures.join("\n  ")
    );
    assert_eq!(
        no_struct,
        NO_STRUCT.iter().map(|(n, _)| *n).collect::<Vec<_>>(),
        "the set of revisions no struct parses moved — update NO_STRUCT (with the reason) and the \
         `Unwired` arm together"
    );
    // ★ Liveness: a dispatch that returned `None` for everything would satisfy every assertion above
    //   while measuring nothing. 38 revisions today, 37 of them wired.
    assert!(
        wired_and_parses >= 37,
        "only {wired_and_parses} revisions were actually parsed — the reader has gone blind"
    );
    assert_eq!(
        wired_and_parses + no_struct.len(),
        LineSet::ALL.len(),
        "every revision must be measured exactly once"
    );
}

/// ★★ The two Form 6251 revisions do NOT accept each other's map, and that is the guarantee.
///
/// They share a form, an attachment sequence and (for 2025/2026) a byte-identical field name set. The
/// only thing separating the TY2024 single-line-1 revision from the 1a/1b one is which struct parses
/// it — so "each refuses the other's map" is the property that keeps a TY2025 chain off a TY2024 PDF
/// and vice versa. Filling the wrong way round writes 2a into 1b's box and walks everything below down
/// one, landing **line 11, the AMT itself**, in line 10's box with nothing red.
#[test]
fn neither_form_6251_revision_accepts_the_others_map() {
    use btctax_forms::bundled::{map_text, Stem};
    use btctax_forms::testonly::{Form6251Map, Form6251ObbbaMap};

    let ty2024 = map_text(Stem::F6251, 2024).expect("bundled");
    let ty2025 = map_text(Stem::F6251, 2025).expect("bundled");
    assert!(Form6251Map::parse(ty2024).is_ok());
    assert!(Form6251ObbbaMap::parse(ty2025).is_ok());
    assert!(
        Form6251Map::parse(ty2025).is_err(),
        "the TY2024 struct accepted the 1a/1b map — line 1a would be discarded in silence"
    );
    assert!(
        Form6251ObbbaMap::parse(ty2024).is_err(),
        "the OBBBA struct accepted the single-line-1 map — line 1a and 1b would be blank on a filed \
         form"
    );
}
