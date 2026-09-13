//! ★ Design r2 §4/§5/§7 — **the line-set revision, as a closed set, and the exhaustive
//! `line_set → schema` match.**
//!
//! Every map's ROW names the LINE-SET REVISION it transcribes (`line_set = "<stem>/<year>"` today —
//! one revision per file; collapsing a constants-only year onto its predecessor's revision is later
//! work with `forms delta` as the evidence). [`LineSet`] is that set as an enum, so an unknown string
//! in a row is a **parse refusal** ([`LineSet::parse`] → `None`, held by `tests/map_rows.rs`), and
//! [`schema`] is the ONE match from a revision to the struct that parses it — **exhaustive**, so a
//! revision added without an arm does not compile, with an explicit [`Schema::Unwired`] arm for the
//! TY2025 maps whose revision has not been VERIFIED against a struct yet (step 5 wires them: each is
//! a deletion from that arm, and forgetting one still cannot compile). **Step 5 (2026-09-05) wired
//! eight**, and `f6251/2025` followed (2026-09-11) into [`Schema::Form6251ObbbaMap`] — the new
//! transcription struct for the line-1-split revision. **ONE remains, deliberately and
//! permanently:** `f1040s1a/2025` — see [`Schema::Unwired`].
//!
//! ★ "Unwired" means unverified, not unparseable (steps-2/3 review Q3). Measured 2026-09-05 on the
//! ten: **eight** (`f1040s2`, `f1040s3`, `f1040sa`, `f1040sb`, `f1040sc`, `f8959`, `f8960`,
//! `f8995`) are key-for-key identical in shape to their 2024 maps and WOULD parse into the 2024
//! struct; `f6251/2025` would not (line 1 split into 1a/1b — a rebuild, a new struct); `f1040s1a`
//! has no struct at all. The door is closed because the label/extract join has not been run for
//! those revisions, which is step 5's criterion — `parse()` succeeding is not it.
//!
//! ★★★ **MANY-TO-ONE IS THE HAZARD THIS MODULE CARRIES, not a convenience.** Two revisions of one
//! form can share a field map *and disagree about what a box means*. The OBBBA-era Form 6251 is the
//! measured case: `xtask form-delta` reports the TY2025 → TY2026 field delta as **62 fields, 0
//! renamed, 0 moved**, and yet eight numbered lines print DIFFERENT text, two of them
//! cross-references — Schedule 1-A line **37** → **43** on line 1a, and Form 1040 line **7** →
//! **7a** on line 7. A single struct serving both would fill either year's PDF with nothing red.
//! So the year-varying cells of that layout live in [`crate::f6251_revision`], keyed per revision and
//! held by the compiler: wiring `f6251/2026` onto this schema without stating its own cells is a
//! **build error** (`E0080` from that module's `const` walk over `ALL` — it was an `_`-free match over
//! every `LineSet` until FR-141, which is why porting Form 8995-A used to red a Form 6251 module).
//!
//! Many-to-one by design: several revisions may parse into one struct (`Form1040Map` absorbs
//! 2024/2025 with `Option` lines today).
//!
//! ## ★★ FR-141 — [`LineSet`] is GENERATED; only [`schema`] is typed
//!
//! The enum, [`LineSet::parse`], [`LineSet::as_str`] and [`LineSet::ALL`] are written by `build.rs`
//! from the `line_set` row of every bundled map (`$OUT_DIR/line_set_generated.rs`, included below).
//! They are pure transcription of a string that is already on disk, and the port rehearsal measured
//! what typing them costs: **7 hand-edits in this file and `f6251_revision.rs` for one new
//! `(stem, year)`, six of which decided nothing** (`design/agent-reports/REPORT-rehearse-port-f8995a-2025.md`
//! F10 → `FOLLOWUPS.md` FR-141). It is now **one** — the [`schema`] arm.
//!
//! ★★★ **The build error survived the generation, which was the whole condition on doing it.** A new
//! map means a new variant, and [`schema`] is still an `_`-free match over every variant, so the port
//! stops at `E0004` in this file until a human says which struct transcribes the revision. Nothing
//! about a new row can compile silently.
//!
//! ★★ And two of the seven were worse than redundant. [`LineSet::ALL`] is what every gate walking the
//! revision set iterates (`tests/line_set_wiring.rs`, `tests/f6251_obbba.rs`,
//! `tests/supported_years_cross_product.rs`), and nothing held it to the glob: measured 2026-09-12, a
//! port that added the variant, the `parse` arm, the `as_str` arm and the [`schema`] arm but forgot the
//! `ALL` entry reds **NOTHING** — the whole suite passes and that revision is never measured by any of
//! those gates again. Deriving `ALL` closes a hole, not just a keystroke.
//!
//! ★ FR-146 is closed by the same change rather than by a fix: two variants had the doc comment of
//! their *neighbour* (`"f8959/2024"`'s sentence sat above `F8889_2024`, and the 2025 block repeated
//! the slip), which is only possible while the comment and the variant are typed separately. They are
//! now emitted from the same string, and `tests::every_generated_doc_comment_names_its_own_row` holds
//! that pairing on the generated text so the generator cannot drift back into it.

// ★ FR-141 — the enum, `parse`, `as_str` and `ALL`, GENERATED by `build.rs` from the `line_set` row
//   of every `forms/<year>/*.map.toml`. See the module header: the four were pure transcription of a
//   string already on disk, `ALL` was a list nothing held to the glob, and `schema` below — the one
//   arm that encodes a decision — is still hand-written and still `_`-free, so a new row is still a
//   build error until a human answers it.
include!(concat!(env!("OUT_DIR"), "/line_set_generated.rs"));

/// Which struct parses a revision — or [`Schema::Unwired`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Schema {
    /// Parses into [`crate::map::Form1040Map`].
    Form1040Map,
    /// Parses into [`crate::map::Form1040VMap`].
    Form1040VMap,
    /// Parses into [`crate::map::Form4868Map`].
    Form4868Map,
    /// Parses into [`crate::map::Form6251Map`] — the **TY2024** revision, whose Part I prints a
    /// single line 1.
    Form6251Map,
    /// Parses into [`crate::map::Form6251ObbbaMap`] — the **OBBBA-era** revision, whose Part I
    /// splits line 1 into **1a/1b** (Pub. L. 119-21). TY2025 and TY2026 share this field map
    /// exactly; their year-varying printed cells are [`crate::f6251_revision`]'s, per revision.
    Form6251ObbbaMap,
    /// Parses into [`crate::map::Form8275Map`].
    Form8275Map,
    /// Parses into [`crate::map::Form8283Map`].
    Form8283Map,
    /// Parses into [`crate::map::Form8949Map`].
    Form8949Map,
    /// Parses into [`crate::map::Form8889Map`].
    Form8889Map,
    /// Parses into [`crate::map::Form8959Map`].
    Form8959Map,
    /// Parses into [`crate::map::Form8960Map`].
    Form8960Map,
    /// Parses into [`crate::map::Form8995AMap`].
    Form8995AMap,
    /// Parses into [`crate::map::Form8995Map`].
    Form8995Map,
    /// Parses into [`crate::map::Schedule1Map`].
    Schedule1Map,
    /// Parses into [`crate::map::Schedule2Map`].
    Schedule2Map,
    /// Parses into [`crate::map::Schedule3Map`].
    Schedule3Map,
    /// Parses into [`crate::map::ScheduleAMap`].
    ScheduleAMap,
    /// Parses into [`crate::map::ScheduleBMap`].
    ScheduleBMap,
    /// Parses into [`crate::map::ScheduleCMap`].
    ScheduleCMap,
    /// Parses into [`crate::map::ScheduleDMap`].
    ScheduleDMap,
    /// Parses into [`crate::map::ScheduleSeMap`].
    ScheduleSeMap,
    /// No struct parses this revision: the map is **bundled, listed, and refused**. `for_year`
    /// returns [`crate::FormsError::UnwiredLineSet`].
    ///
    /// ★★★ **Exactly one revision carries this, and it is a settled destination rather than a queue
    /// position: `f1040s1a/2025`.** Owner ruling 2026-09-11 — *"We will not file a 2025 tax year
    /// return with this software. We only care about 2025 to the extent that it helps us with 2026
    /// and beyond."* So no TY2025 Schedule 1-A will ever be printed, and TY2026's Schedule 1-A is
    /// not a constants bump but a REBUILD (`design/TY2026_WORK_LIST.md`: 10 of 219 fields survive,
    /// 175 added), so a struct written against the TY2025 revision would have to be thrown away.
    /// The map stays bound by `build.rs`, hash-pinned, extract-archived and 100 % censused — every
    /// obligation except a filler — which is the state `Unwired` exists to express. `f6251/2025`
    /// left this arm on 2026-09-11 for the opposite reason: its TY2026 field map is IDENTICAL, so
    /// the struct written for it is TY2026's, a season early.
    Unwired,
}

/// ★ THE MATCH — the one thing a new row still costs a human, and the reason FR-141's generation did
/// not weaken anything. Exhaustive over [`LineSet`] with no `_` arm: a new revision without an arm is
/// `E0004`, in this file, naming the variant.
///
/// ★ `const` since FR-141 so [`crate::f6251_revision`] can hold its own totality at compile time
/// without an `_`-free match over every unrelated revision — see that module's header.
pub const fn schema(ls: LineSet) -> Schema {
    match ls {
        LineSet::F1040_2024 => Schema::Form1040Map,
        LineSet::F1040s1_2024 => Schema::Schedule1Map,
        LineSet::F1040s2_2024 => Schema::Schedule2Map,
        LineSet::F1040s3_2024 => Schema::Schedule3Map,
        LineSet::F1040sa_2024 => Schema::ScheduleAMap,
        LineSet::F1040sb_2024 => Schema::ScheduleBMap,
        LineSet::F1040sc_2024 => Schema::ScheduleCMap,
        LineSet::F1040v_2024 => Schema::Form1040VMap,
        LineSet::F4868_2024 => Schema::Form4868Map,
        LineSet::F6251_2024 => Schema::Form6251Map,
        LineSet::F8275_2024 => Schema::Form8275Map,
        LineSet::F8283_2024 => Schema::Form8283Map,
        LineSet::F8949_2024 => Schema::Form8949Map,
        LineSet::F8889_2024 => Schema::Form8889Map,
        LineSet::F8959_2024 => Schema::Form8959Map,
        LineSet::F8960_2024 => Schema::Form8960Map,
        LineSet::F8995_2024 => Schema::Form8995Map,
        LineSet::F8995a_2024 => Schema::Form8995AMap,
        LineSet::ScheduleD_2024 => Schema::ScheduleDMap,
        LineSet::ScheduleSe_2024 => Schema::ScheduleSeMap,
        // ── TY2025. ★ FR-141/FR-146: the notes below were doc comments on the variants until the
        //    variants became generated. They are JUDGMENT, so they belong beside the judgment — and
        //    two of them were attached to the wrong variant while they lived up there.
        //
        //    ★ Eight were wired at step 5 (2026-09-05) on one measured finding: map ⊆ PDF fields, the
        //      line→printed-label join and `[census]` all green, and each parses into the 2024 struct
        //      — a constants-only revision of the same line set. They are `f1040s2`, `f1040s3`,
        //      `f1040sa`, `f1040sb`, `f1040sc`, `f8959`, `f8960`, `f8995`.
        //    ★ `f8889/2025` is the SAME line set as 2024 — measured: the two grids' label readings
        //      are identical and the extracts differ only in the year and the §223(b) figures (T16).
        //    ★ `f6251/2025` left `Unwired` on 2026-09-11 for `Form6251ObbbaMap`, the line-1a/1b
        //      transcription struct. `f1040s1a/2025` stays `Unwired` permanently — `Schema::Unwired`
        //      carries the owner ruling.
        LineSet::F1040_2025 => Schema::Form1040Map,
        LineSet::F1040s1a_2025 => Schema::Unwired,
        LineSet::F1040s2_2025 => Schema::Schedule2Map,
        LineSet::F1040s3_2025 => Schema::Schedule3Map,
        LineSet::F1040sa_2025 => Schema::ScheduleAMap,
        LineSet::F1040sb_2025 => Schema::ScheduleBMap,
        LineSet::F1040sc_2025 => Schema::ScheduleCMap,
        LineSet::F1040v_2025 => Schema::Form1040VMap,
        LineSet::F4868_2025 => Schema::Form4868Map,
        LineSet::F6251_2025 => Schema::Form6251ObbbaMap,
        LineSet::F8283_2025 => Schema::Form8283Map,
        LineSet::F8949_2025 => Schema::Form8949Map,
        LineSet::F8889_2025 => Schema::Form8889Map,
        LineSet::F8959_2025 => Schema::Form8959Map,
        LineSet::F8960_2025 => Schema::Form8960Map,
        LineSet::F8995_2025 => Schema::Form8995Map,
        LineSet::ScheduleD_2025 => Schema::ScheduleDMap,
        LineSet::ScheduleSe_2025 => Schema::ScheduleSeMap,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The text `build.rs` generated. The three tests below hold the GENERATOR, not the enum: with the
    /// revision set derived, "is this variant right" is not a question a human answers any more, and
    /// "did the thing that reads the rows read them correctly" is.
    const GENERATED: &str = include_str!(concat!(env!("OUT_DIR"), "/line_set_generated.rs"));

    /// Each generated variant paired with the doc line `build.rs` emitted directly above it — the row
    /// string it quotes and the map file(s) it says carry that revision. Both halves are the
    /// generator's own account of what it read, which is what makes them checkable against the maps.
    fn generated_docs() -> std::collections::BTreeMap<String, (String, String)> {
        let mut out = std::collections::BTreeMap::new();
        let mut pending: Option<(String, String)> = None;
        for line in GENERATED.lines() {
            let t = line.trim();
            if let Some(doc) = t.strip_prefix("/// `\"") {
                let (row, rest) = doc.split_once('"').expect("a closing quote");
                // The tail reads "` — the revision transcribed by `forms/<year>/<stem>.map.toml`."; keep
                // the file list, and fall back to the whole tail if the generator's wording moves (the
                // `contains` join below works either way — this only keeps the message readable).
                let carried_by = rest
                    .split_once("transcribed by ")
                    .map_or(rest.trim(), |(_, tail)| tail.trim().trim_end_matches('.'));
                pending = Some((row.to_string(), carried_by.to_string()));
                continue;
            }
            if t.starts_with("//") || t.is_empty() {
                continue;
            }
            if let Some(pair) = pending.take() {
                out.insert(t.trim_end_matches(',').to_string(), pair);
            }
            if out.len() == LineSet::ALL.len() {
                break; // past the enum body; `parse` / `as_str` / `ALL` follow.
            }
        }
        out
    }

    #[test]
    fn every_variant_round_trips_through_its_string() {
        for ls in LineSet::ALL {
            assert_eq!(LineSet::parse(ls.as_str()), Some(*ls));
        }
        assert_eq!(LineSet::parse("f6251/1999"), None);
        // A revision this build no longer bundles does not parse — the five TY2017 revisions were
        // dropped by S9 (owner ruling 2026-09-06) and must not survive as a parseable string.
        assert_eq!(LineSet::parse("f1040/2017"), None);
    }

    /// ★★★ **The generator's kill — and what used to be `assert_eq!(LineSet::ALL.len(), 38)`.**
    ///
    /// That was a hand-typed number beside a set that grows, and the port rehearsal counted it as one of
    /// the seven per-`(stem, year)` edits (FR-141). The set is derived now, so the assertion that can
    /// actually fail is this one: what `build.rs` read out of the maps with a LINE SCAN must be what
    /// `MapRow::read` reads out of the same files with a real TOML parse — **per map**, not just as a
    /// set. Two independent readers of one string, joined on the file they both read.
    ///
    /// ★ Per-map matters. A set comparison alone passes if two maps' revisions are SWAPPED, and the
    /// generated variant set — which `schema`'s `_`-free match already pins — would be identical. The
    /// join is via the generator's own doc line, which names the map it read each revision from.
    ///
    /// ★★ It is the liveness check on `ALL` too: every bundled map's revision is in it. Nothing held
    /// that while the list was typed — measured 2026-09-12, a port that forgot the `ALL` entry red
    /// NOTHING, and that revision was then skipped by every gate that iterates it.
    #[test]
    fn every_bundled_map_names_the_revision_the_generator_attributed_to_it() {
        use std::collections::BTreeSet;
        let docs = generated_docs();
        let mut from_toml: BTreeSet<String> = BTreeSet::new();
        for (stem, year) in crate::bundled::BUNDLED {
            let text = crate::bundled::map_text(*stem, *year).expect("a bundled pair has a map");
            let row = crate::map::MapRow::read(text)
                .unwrap_or_else(|e| panic!("{}/{year}: the row must parse: {e}", stem.file_stem()));
            let ls = LineSet::parse(&row.line_set).unwrap_or_else(|| {
                panic!(
                    "forms/{year}/{}.map.toml names revision `{}`, which the generated `LineSet` does \
                     not know. `build.rs` read this same file: the line scan and the TOML parse \
                     disagree, and every gate that walks `LineSet::ALL` measures the scan's answer.",
                    stem.file_stem(),
                    row.line_set
                )
            });
            let (_, carried_by) = docs
                .get(&format!("{ls:?}"))
                .unwrap_or_else(|| panic!("{ls:?} has no generated doc line"));
            let expected_file = format!("forms/{year}/{}.map.toml", stem.file_stem());
            assert!(
                carried_by.contains(&expected_file),
                "`forms/{year}/{}.map.toml` names revision `{}`, but the generator recorded that \
                 revision as \"{carried_by}\" — the line scan and the TOML parse are reading \
                 different files, or the same file differently. Note the SET can still match: this \
                 is what a set comparison alone cannot see.",
                stem.file_stem(),
                row.line_set
            );
            from_toml.insert(row.line_set);
        }
        let generated: BTreeSet<String> =
            LineSet::ALL.iter().map(|ls| ls.as_str().into()).collect();
        assert_eq!(
            generated, from_toml,
            "the generated revision set is not the set the bundled maps name. A revision only the \
             TOML parse knows is a map no gate looks at; one only the scan knows is a phantom."
        );
        assert!(
            !generated.is_empty(),
            "no revisions at all — the two readers agree because neither read anything"
        );
    }

    /// ★★ **FR-146, machine-checked.** Two variants carried their NEIGHBOUR's doc comment
    /// (`"f8959/2024"`'s sentence sat above `F8889_2024`, and the 2025 block repeated the slip), which
    /// is only possible while the text and the variant are typed separately. They come off one string
    /// now — and this holds that pairing on the generated text, so the generator cannot drift back into
    /// it by emitting the doc line and the variant out of step.
    ///
    /// ★ Said plainly, because the alternative was a weak check: a misplaced doc comment in HAND-WRITTEN
    /// source is invisible to the compiler and to every test — which is exactly why FR-146 survived
    /// until a human read the file. Generation is what makes it checkable at all; the kill is a one-line
    /// mutation in `build.rs` (emit the previous revision's doc), watched red 2026-09-12.
    #[test]
    fn every_generated_doc_comment_names_its_own_row() {
        let docs = generated_docs();
        assert_eq!(
            docs.len(),
            LineSet::ALL.len(),
            "only {} of {} variants carried a readable doc line — a parse that sees nothing cannot \
             see a misattached comment either",
            docs.len(),
            LineSet::ALL.len()
        );
        for ls in LineSet::ALL {
            let (quoted, _) = &docs[&format!("{ls:?}")];
            assert_eq!(
                quoted.as_str(),
                ls.as_str(),
                "the doc comment above `{ls:?}` describes `{quoted}` — the FR-146 slip, in the \
                 generator this time"
            );
        }
    }

    /// The Unwired set is EXACTLY `f1040s1a/2025` — a shrink-only pin: wiring one edits this list
    /// down; a new unwired revision cannot appear without editing it up.
    ///
    /// 2 → 1 on 2026-09-11: `f6251/2025` was wired to [`Schema::Form6251ObbbaMap`]. The one that
    /// remains is a settled destination, not a queue position — see [`Schema::Unwired`].
    #[test]
    fn the_unwired_set_is_exactly_the_schedule_1a_revision_that_will_never_be_filed() {
        let unwired: Vec<&str> = LineSet::ALL
            .iter()
            .filter(|ls| schema(**ls) == Schema::Unwired)
            .map(|ls| ls.as_str())
            .collect();
        assert_eq!(unwired, ["f1040s1a/2025"]);
    }
}
