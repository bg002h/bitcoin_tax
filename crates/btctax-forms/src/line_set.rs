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
//! eight**; two remain: `f6251/2025` (line 1 split into 1a/1b — a rebuild, a new struct, done once
//! for TY2026 which shares the layout; TY2025 is paused) and `f1040s1a/2025` (no filler exists yet —
//! the Schedule 1-A emitter is the missing 17th form).
//!
//! ★ "Unwired" means unverified, not unparseable (steps-2/3 review Q3). Measured 2026-09-05 on the
//! ten: **eight** (`f1040s2`, `f1040s3`, `f1040sa`, `f1040sb`, `f1040sc`, `f8959`, `f8960`,
//! `f8995`) are key-for-key identical in shape to their 2024 maps and WOULD parse into the 2024
//! struct; `f6251/2025` would not (line 1 split into 1a/1b — a rebuild, a new struct); `f1040s1a`
//! has no struct at all. The door is closed because the label/extract join has not been run for
//! those revisions, which is step 5's criterion — `parse()` succeeding is not it.
//!
//! Many-to-one by design: several revisions may parse into one struct (`Form1040Map` absorbs
//! 2017/2024/2025 with `Option` lines today).

/// The closed set of line-set revisions this build knows. Generated 2026-09-05 from the 37 rows; the four `f4868`/`f1040v` revisions added 2026-09-06 (spec 4868/1040-V R1) make 41.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LineSet {
    /// `"f1040/2017"`.
    F1040_2017,
    /// `"f8283/2017"`.
    F8283_2017,
    /// `"f8949/2017"`.
    F8949_2017,
    /// `"schedule_d/2017"`.
    ScheduleD_2017,
    /// `"schedule_se/2017"`.
    ScheduleSe_2017,
    /// `"f1040/2024"`.
    F1040_2024,
    /// `"f1040s1/2024"`.
    F1040s1_2024,
    /// `"f1040s2/2024"`.
    F1040s2_2024,
    /// `"f1040s3/2024"`.
    F1040s3_2024,
    /// `"f1040sa/2024"`.
    F1040sa_2024,
    /// `"f1040sb/2024"`.
    F1040sb_2024,
    /// `"f1040sc/2024"`.
    F1040sc_2024,
    /// `"f1040v/2024"`.
    F1040v_2024,
    /// `"f4868/2024"`.
    F4868_2024,
    /// `"f6251/2024"`.
    F6251_2024,
    /// `"f8275/2024"`.
    F8275_2024,
    /// `"f8283/2024"`.
    F8283_2024,
    /// `"f8949/2024"`.
    F8949_2024,
    /// `"f8959/2024"`.
    F8959_2024,
    /// `"f8960/2024"`.
    F8960_2024,
    /// `"f8995/2024"`.
    F8995_2024,
    /// `"f8995a/2024"`.
    F8995a_2024,
    /// `"schedule_d/2024"`.
    ScheduleD_2024,
    /// `"schedule_se/2024"`.
    ScheduleSe_2024,
    /// `"f1040/2025"`.
    F1040_2025,
    /// `"f1040s1a/2025"` — UNWIRED: not yet verified against a struct (design r2 §10 step 5).
    F1040s1a_2025,
    /// `"f1040s2/2025"` — wired at step 5 (2026-09-05): map ⊆ PDF fields, the label join and `[census]` all green; parses into the 2024 struct (a constants-only revision of the same line set).
    F1040s2_2025,
    /// `"f1040s3/2025"` — wired at step 5 (2026-09-05): map ⊆ PDF fields, the label join and `[census]` all green; parses into the 2024 struct (a constants-only revision of the same line set).
    F1040s3_2025,
    /// `"f1040sa/2025"` — wired at step 5 (2026-09-05): map ⊆ PDF fields, the label join and `[census]` all green; parses into the 2024 struct (a constants-only revision of the same line set).
    F1040sa_2025,
    /// `"f1040sb/2025"` — wired at step 5 (2026-09-05): map ⊆ PDF fields, the label join and `[census]` all green; parses into the 2024 struct (a constants-only revision of the same line set).
    F1040sb_2025,
    /// `"f1040sc/2025"` — wired at step 5 (2026-09-05): map ⊆ PDF fields, the label join and `[census]` all green; parses into the 2024 struct (a constants-only revision of the same line set).
    F1040sc_2025,
    /// `"f1040v/2025"`.
    F1040v_2025,
    /// `"f4868/2025"`.
    F4868_2025,
    /// `"f6251/2025"` — UNWIRED: not yet verified against a struct (design r2 §10 step 5).
    F6251_2025,
    /// `"f8283/2025"`.
    F8283_2025,
    /// `"f8949/2025"`.
    F8949_2025,
    /// `"f8959/2025"` — wired at step 5 (2026-09-05): map ⊆ PDF fields, the label join and `[census]` all green; parses into the 2024 struct (a constants-only revision of the same line set).
    F8959_2025,
    /// `"f8960/2025"` — wired at step 5 (2026-09-05): map ⊆ PDF fields, the label join and `[census]` all green; parses into the 2024 struct (a constants-only revision of the same line set).
    F8960_2025,
    /// `"f8995/2025"` — wired at step 5 (2026-09-05): map ⊆ PDF fields, the label join and `[census]` all green; parses into the 2024 struct (a constants-only revision of the same line set).
    F8995_2025,
    /// `"schedule_d/2025"`.
    ScheduleD_2025,
    /// `"schedule_se/2025"`.
    ScheduleSe_2025,
}

impl LineSet {
    /// Parse a row's `line_set` string. `None` is a revision this build does not know — a refusal.
    pub fn parse(s: &str) -> Option<LineSet> {
        match s {
            "f1040/2017" => Some(LineSet::F1040_2017),
            "f8283/2017" => Some(LineSet::F8283_2017),
            "f8949/2017" => Some(LineSet::F8949_2017),
            "schedule_d/2017" => Some(LineSet::ScheduleD_2017),
            "schedule_se/2017" => Some(LineSet::ScheduleSe_2017),
            "f1040/2024" => Some(LineSet::F1040_2024),
            "f1040s1/2024" => Some(LineSet::F1040s1_2024),
            "f1040s2/2024" => Some(LineSet::F1040s2_2024),
            "f1040s3/2024" => Some(LineSet::F1040s3_2024),
            "f1040sa/2024" => Some(LineSet::F1040sa_2024),
            "f1040sb/2024" => Some(LineSet::F1040sb_2024),
            "f1040sc/2024" => Some(LineSet::F1040sc_2024),
            "f1040v/2024" => Some(LineSet::F1040v_2024),
            "f4868/2024" => Some(LineSet::F4868_2024),
            "f6251/2024" => Some(LineSet::F6251_2024),
            "f8275/2024" => Some(LineSet::F8275_2024),
            "f8283/2024" => Some(LineSet::F8283_2024),
            "f8949/2024" => Some(LineSet::F8949_2024),
            "f8959/2024" => Some(LineSet::F8959_2024),
            "f8960/2024" => Some(LineSet::F8960_2024),
            "f8995/2024" => Some(LineSet::F8995_2024),
            "f8995a/2024" => Some(LineSet::F8995a_2024),
            "schedule_d/2024" => Some(LineSet::ScheduleD_2024),
            "schedule_se/2024" => Some(LineSet::ScheduleSe_2024),
            "f1040/2025" => Some(LineSet::F1040_2025),
            "f1040s1a/2025" => Some(LineSet::F1040s1a_2025),
            "f1040s2/2025" => Some(LineSet::F1040s2_2025),
            "f1040s3/2025" => Some(LineSet::F1040s3_2025),
            "f1040sa/2025" => Some(LineSet::F1040sa_2025),
            "f1040sb/2025" => Some(LineSet::F1040sb_2025),
            "f1040sc/2025" => Some(LineSet::F1040sc_2025),
            "f1040v/2025" => Some(LineSet::F1040v_2025),
            "f4868/2025" => Some(LineSet::F4868_2025),
            "f6251/2025" => Some(LineSet::F6251_2025),
            "f8283/2025" => Some(LineSet::F8283_2025),
            "f8949/2025" => Some(LineSet::F8949_2025),
            "f8959/2025" => Some(LineSet::F8959_2025),
            "f8960/2025" => Some(LineSet::F8960_2025),
            "f8995/2025" => Some(LineSet::F8995_2025),
            "schedule_d/2025" => Some(LineSet::ScheduleD_2025),
            "schedule_se/2025" => Some(LineSet::ScheduleSe_2025),
            _ => None,
        }
    }

    /// The row string this variant names.
    pub fn as_str(self) -> &'static str {
        match self {
            LineSet::F1040_2017 => "f1040/2017",
            LineSet::F8283_2017 => "f8283/2017",
            LineSet::F8949_2017 => "f8949/2017",
            LineSet::ScheduleD_2017 => "schedule_d/2017",
            LineSet::ScheduleSe_2017 => "schedule_se/2017",
            LineSet::F1040_2024 => "f1040/2024",
            LineSet::F1040s1_2024 => "f1040s1/2024",
            LineSet::F1040s2_2024 => "f1040s2/2024",
            LineSet::F1040s3_2024 => "f1040s3/2024",
            LineSet::F1040sa_2024 => "f1040sa/2024",
            LineSet::F1040sb_2024 => "f1040sb/2024",
            LineSet::F1040sc_2024 => "f1040sc/2024",
            LineSet::F1040v_2024 => "f1040v/2024",
            LineSet::F4868_2024 => "f4868/2024",
            LineSet::F6251_2024 => "f6251/2024",
            LineSet::F8275_2024 => "f8275/2024",
            LineSet::F8283_2024 => "f8283/2024",
            LineSet::F8949_2024 => "f8949/2024",
            LineSet::F8959_2024 => "f8959/2024",
            LineSet::F8960_2024 => "f8960/2024",
            LineSet::F8995_2024 => "f8995/2024",
            LineSet::F8995a_2024 => "f8995a/2024",
            LineSet::ScheduleD_2024 => "schedule_d/2024",
            LineSet::ScheduleSe_2024 => "schedule_se/2024",
            LineSet::F1040_2025 => "f1040/2025",
            LineSet::F1040s1a_2025 => "f1040s1a/2025",
            LineSet::F1040s2_2025 => "f1040s2/2025",
            LineSet::F1040s3_2025 => "f1040s3/2025",
            LineSet::F1040sa_2025 => "f1040sa/2025",
            LineSet::F1040sb_2025 => "f1040sb/2025",
            LineSet::F1040sc_2025 => "f1040sc/2025",
            LineSet::F1040v_2025 => "f1040v/2025",
            LineSet::F4868_2025 => "f4868/2025",
            LineSet::F6251_2025 => "f6251/2025",
            LineSet::F8283_2025 => "f8283/2025",
            LineSet::F8949_2025 => "f8949/2025",
            LineSet::F8959_2025 => "f8959/2025",
            LineSet::F8960_2025 => "f8960/2025",
            LineSet::F8995_2025 => "f8995/2025",
            LineSet::ScheduleD_2025 => "schedule_d/2025",
            LineSet::ScheduleSe_2025 => "schedule_se/2025",
        }
    }

    /// Every variant, for the tests that hold this set to the rows on disk.
    pub const ALL: &'static [LineSet] = &[
        LineSet::F1040_2017,
        LineSet::F8283_2017,
        LineSet::F8949_2017,
        LineSet::ScheduleD_2017,
        LineSet::ScheduleSe_2017,
        LineSet::F1040_2024,
        LineSet::F1040s1_2024,
        LineSet::F1040s2_2024,
        LineSet::F1040s3_2024,
        LineSet::F1040sa_2024,
        LineSet::F1040sb_2024,
        LineSet::F1040sc_2024,
        LineSet::F1040v_2024,
        LineSet::F4868_2024,
        LineSet::F6251_2024,
        LineSet::F8275_2024,
        LineSet::F8283_2024,
        LineSet::F8949_2024,
        LineSet::F8959_2024,
        LineSet::F8960_2024,
        LineSet::F8995_2024,
        LineSet::F8995a_2024,
        LineSet::ScheduleD_2024,
        LineSet::ScheduleSe_2024,
        LineSet::F1040_2025,
        LineSet::F1040s1a_2025,
        LineSet::F1040s2_2025,
        LineSet::F1040s3_2025,
        LineSet::F1040sa_2025,
        LineSet::F1040sb_2025,
        LineSet::F1040sc_2025,
        LineSet::F1040v_2025,
        LineSet::F4868_2025,
        LineSet::F6251_2025,
        LineSet::F8283_2025,
        LineSet::F8949_2025,
        LineSet::F8959_2025,
        LineSet::F8960_2025,
        LineSet::F8995_2025,
        LineSet::ScheduleD_2025,
        LineSet::ScheduleSe_2025,
    ];
}

/// Which struct parses a revision — or [`Schema::Unwired`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Schema {
    /// Parses into [`crate::map::Form1040Map`].
    Form1040Map,
    /// Parses into [`crate::map::Form1040VMap`].
    Form1040VMap,
    /// Parses into [`crate::map::Form4868Map`].
    Form4868Map,
    /// Parses into [`crate::map::Form6251Map`].
    Form6251Map,
    /// Parses into [`crate::map::Form8275Map`].
    Form8275Map,
    /// Parses into [`crate::map::Form8283Map`].
    Form8283Map,
    /// Parses into [`crate::map::Form8949Map`].
    Form8949Map,
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
    /// This revision has not been verified against a struct yet (design r2 §10 step 3 → step 5) —
    /// eight of the ten would parse today; see the module doc. `for_year` returns
    /// [`crate::FormsError::UnwiredLineSet`] for it — the map is bundled, listed, and refused.
    Unwired,
}

/// ★ THE MATCH. Exhaustive over [`LineSet`]: a new revision without an arm is a compile error.
pub fn schema(ls: LineSet) -> Schema {
    match ls {
        LineSet::F1040_2017 => Schema::Form1040Map,
        LineSet::F8283_2017 => Schema::Form8283Map,
        LineSet::F8949_2017 => Schema::Form8949Map,
        LineSet::ScheduleD_2017 => Schema::ScheduleDMap,
        LineSet::ScheduleSe_2017 => Schema::ScheduleSeMap,
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
        LineSet::F8959_2024 => Schema::Form8959Map,
        LineSet::F8960_2024 => Schema::Form8960Map,
        LineSet::F8995_2024 => Schema::Form8995Map,
        LineSet::F8995a_2024 => Schema::Form8995AMap,
        LineSet::ScheduleD_2024 => Schema::ScheduleDMap,
        LineSet::ScheduleSe_2024 => Schema::ScheduleSeMap,
        LineSet::F1040_2025 => Schema::Form1040Map,
        LineSet::F1040s1a_2025 => Schema::Unwired,
        LineSet::F1040s2_2025 => Schema::Schedule2Map,
        LineSet::F1040s3_2025 => Schema::Schedule3Map,
        LineSet::F1040sa_2025 => Schema::ScheduleAMap,
        LineSet::F1040sb_2025 => Schema::ScheduleBMap,
        LineSet::F1040sc_2025 => Schema::ScheduleCMap,
        LineSet::F1040v_2025 => Schema::Form1040VMap,
        LineSet::F4868_2025 => Schema::Form4868Map,
        LineSet::F6251_2025 => Schema::Unwired,
        LineSet::F8283_2025 => Schema::Form8283Map,
        LineSet::F8949_2025 => Schema::Form8949Map,
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

    #[test]
    fn every_variant_round_trips_through_its_string() {
        for ls in LineSet::ALL {
            assert_eq!(LineSet::parse(ls.as_str()), Some(*ls));
        }
        assert_eq!(LineSet::parse("f6251/1999"), None);
        // 37 → 41 on 2026-09-06: the four Form 4868 / Form 1040-V revisions (spec 4868/1040-V R1).
        assert_eq!(LineSet::ALL.len(), 41);
    }

    /// The Unwired set is EXACTLY the two that step 5 could not wire — a shrink-only pin: wiring
    /// one edits this list down; a new unwired revision cannot appear without editing it up.
    #[test]
    fn the_unwired_set_is_exactly_the_two_ty2025_maps_step_5_could_not_wire() {
        let unwired: Vec<&str> = LineSet::ALL
            .iter()
            .filter(|ls| schema(**ls) == Schema::Unwired)
            .map(|ls| ls.as_str())
            .collect();
        assert_eq!(unwired, ["f1040s1a/2025", "f6251/2025"]);
    }
}
