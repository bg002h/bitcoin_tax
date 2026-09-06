//! §G-13 — **the FIELD CENSUS gate**: every AcroForm field is accounted for exactly once,
//! **in every bundled year**.
//!
//! ★★ **The missing direction.** `verify.rs::no_unmapped_filled` already asserts every field
//! carrying a VALUE is authorised — it stops us writing where we should not. Nothing asserted the
//! other way: that every field is either mapped (we fill it) or deliberately named as one we do not.
//! One direction stops stray writes; this one stops **silent omissions**, and omission is the
//! direction that costs a filer money.
//!
//! ★★ **This is not new knowledge — it is knowledge made enforceable.** `Form8959Map`'s doc comment
//! already said, in prose: *"Lines 2/3 (Form 4137 / Form 8919) and all of Part III plus line 23
//! (RRTA) are unmodeled and are deliberately absent."* `f1040.map.toml` says the same of the spouse's
//! IP PIN: *"ReturnInputs does not capture one, so it is left BLANK, never guessed."* Both are exactly
//! the provenance record this census wants, written where nothing could check them.
//!
//! **The rule:** `(map FQNs) ∪ (census FQNs) == (the PDF's AcroForm FQNs)`, exactly. A field in
//! neither is the "we forgot this line" defect; a field in both is a contradiction.
//!
//! ★★★ **THE YEAR PIN, and why it was the worse half of the same defect (2026-09-05).** This file's
//! own gate ran `let year = 2024;` while the build bundles **three** years (`SUPPORTED_YEARS =
//! &[2017, 2024, 2025]`) and a fourth is being ported. It therefore re-checked the one year that was
//! already exact, found it exact, and **reported success** — the shape `TY2026_PORT_REPORT.md` §2
//! names *"the product fails CLOSED, the instruments fail OPEN"* (row 12, R15).
//!
//! The sibling `map_pdf_conformance.rs` already held the other half of this invariant and was already
//! year-general, walking `crates/btctax-forms/forms/` on the filesystem. Two halves of one rule sat in
//! one directory, one derived and one pinned. **The year set is now derived the same way** — a new
//! `forms/<year>/` directory is under the gate the moment it is committed, with nobody remembering to
//! add it.
//!
//! ★★★ **What de-pinning revealed, and why the answer is a REGISTER and not an exclusion.** Made
//! year-general the gate goes RED, because two of the three bundled years were never censused at all:
//!
//! | year | maps | censused | **fields with NO recorded decision** |
//! |---|---|---|---|
//! | 2017 | 5 | **0** | **403** |
//! | 2024 | 17 | 17 | 0 |
//! | 2025 | 15 | 10 | **330** |
//!
//! That is the honest state and it is the finding, so it is **recorded**, not excluded:
//! [`UNCENSUSED`] names every `(year, stem)` whose map carries no `[census]` section together with
//! its exact unaccounted count, and the gate pins each number. A `(year, stem)` that is **not** on the
//! register must account for **100%** of its fields. So the register is the only allowance, it is
//! shrink-only, and — this is the point of the whole exercise — **a newly committed `forms/2026/`
//! reds immediately**, because nothing on disk grants it an allowance.
//!
//! ★ Per `TY2026_PORT_REPORT.md` §7 D6 the alternative was to de-pin only onto *wired* years, so the
//! gate would not red on paused TY2025 work. A register does strictly better: it fails closed on a new
//! year exactly the same way, and it also carries TY2017's 403 and TY2025's 330 as **numbers in the
//! suite** rather than as prose in a report nobody executes.

use btctax_forms::testonly::{collect_fields, load};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
mod common;

/// ★★★ **The ONE allowance in the gate, and it may only shrink.**
///
/// Every `(year, stem, unaccounted)` whose committed map carries **no `[census]` section**: nobody has
/// classified that form's fields for that year yet. Being here is not "exempt" — it is a *recorded*
/// gap rather than a silent one, and the number is what burns down.
///
/// Measured 2026-09-05 by walking `forms/` (see the module doc's table). Every count below is the
/// output of the gate itself, not a hand tally.
///
/// **Closing an entry** means writing that map's `[census]` section and DELETING the line — at which
/// point the gate demands 100% accounting for it, permanently. **Lowering** a number without removing
/// the line is the partial-progress case and is equally legal. Raising one, or adding a line, is the
/// defect this register exists to make loud.
const UNCENSUSED: &[(i32, &str, usize)] = &[
    // TY2017 — a fully wired, shipped year (`SUPPORTED_YEARS`) with ZERO census coverage. Its maps
    // are deliberate crypto-slice partials (2017 f1040 maps the line-13 dollars+cents pair and
    // nothing else), but "deliberately partial" and "nobody recorded a decision" are indistinguishable
    // on the printed page — which is the entire premise of this file.
    (2017, "f1040", 254),
    (2017, "f8283", 62),
    (2017, "f8949", 8),
    (2017, "schedule_d", 40),
    (2017, "schedule_se", 39),
    // TY2025 — ten of fifteen maps DO carry a census and account for 100%. These five do not.
    (2025, "f1040", 196),
    (2025, "f8283", 63),
    (2025, "f8949", 16),
    (2025, "schedule_d", 40),
    (2025, "schedule_se", 15),
];

/// The register's own totals, pinned so that a single edited line is visible as a changed number.
/// 5 + 5 entries; 403 + 330 fields.
const UNCENSUSED_ENTRIES: usize = 10;
const UNCENSUSED_FIELDS: usize = 733;

// ★ Design r2 §10 step 4: the per-year ABSENT list is no longer a hand-list here — it is each year's
//   `forms/<year>/YEAR.toml` `[forms_absent]` (with the reason beside each), read through
//   `btctax_forms::year_record::YearRecord`. This gate reads the declaration; `tests/year_record.rs`
//   holds the declaration to the glob and to `Stem::ALL`.

fn forms_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("forms")
}

/// Every year directory under `forms/`. **Derived from the filesystem**, exactly as
/// `map_pdf_conformance.rs::every_committed_map` does — never a hand-list, never a range.
fn every_bundled_year() -> Vec<i32> {
    let mut years: Vec<i32> = std::fs::read_dir(forms_root())
        .expect("forms/ must exist")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().to_string_lossy().parse::<i32>().ok())
        .collect();
    years.sort_unstable();
    assert!(
        years.len() >= 3,
        "the walk found only {} year directories under {} — a gate that walks nothing passes by \
         finding nothing",
        years.len(),
        forms_root().display()
    );
    years
}

/// Every `.map.toml` stem committed for one year. Derived, sorted, never hand-listed.
fn stems_for(year: i32) -> Vec<String> {
    let mut stems: Vec<String> = std::fs::read_dir(forms_root().join(year.to_string()))
        .unwrap_or_else(|e| panic!("forms/{year}/ must be readable: {e}"))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.to_string_lossy().ends_with(".map.toml"))
        .map(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .trim_end_matches(".map.toml")
                .to_string()
        })
        .collect();
    stems.sort();
    assert!(
        !stems.is_empty(),
        "forms/{year}/ contains no .map.toml — a year directory the gate cannot see into must not \
         be silently counted as clean"
    );
    stems
}

fn map_path(stem: &str, year: i32) -> PathBuf {
    forms_root()
        .join(year.to_string())
        .join(format!("{stem}.map.toml"))
}

/// Every FQN-shaped string in a map file, split into the mapped set and the `[census]` set, plus
/// whether the file declares a `[census]` section **at all** (an empty one is legitimate — TY2025's
/// `f1040s1a` maps all 54 of its fields — and is NOT the same as having none).
///
/// ★ Text-scanned rather than deserialized on purpose: the census must see **every** FQN the file
/// names, including any a future typed struct forgets to model. A parser that only sees what its
/// struct declares would go blind exactly where this test needs sight.
fn map_and_census(stem: &str, year: i32) -> Option<(BTreeSet<String>, BTreeSet<String>, bool)> {
    let text = std::fs::read_to_string(map_path(stem, year)).ok()?;
    let (mut mapped, mut census) = (BTreeSet::new(), BTreeSet::new());
    let (mut in_census, mut has_census) = (false, false);
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with('[') && !t.starts_with("[[") {
            in_census = t == "[census]";
            has_census |= in_census;
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
    Some((mapped, census, has_census))
}

fn acroform_fqns(stem: &str, year: i32) -> Option<BTreeSet<String>> {
    let pdf = forms_root()
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

/// ★★★ **THE GATE, as a pure function.**
///
/// Kept separate from the filesystem walk for one reason: **B1 — no checker exists until it has been
/// observed RED on a planted defect.** A plant that has to mutate a committed `*.map.toml` cannot be
/// left in the suite, so the verdict is a function over sets and
/// [`the_gate_reds_on_every_planted_defect`] watches it reject each defect class by construction. The
/// walk below is then only plumbing.
///
/// `Ok(())` iff the form is exactly accounted for under its register allowance.
fn verdict(
    has_census: bool,
    allowance: Option<usize>,
    mapped: &BTreeSet<String>,
    census: &BTreeSet<String>,
    actual: &BTreeSet<String>,
) -> Result<(), String> {
    let accounted: BTreeSet<&String> = mapped.union(census).collect();

    let both: Vec<&&String> = accounted
        .iter()
        .filter(|f| mapped.contains(**f) && census.contains(**f))
        .collect();
    if !both.is_empty() {
        return Err(format!(
            "{both:?} are BOTH mapped and censused — a field cannot be one we fill and one we \
             deliberately leave blank"
        ));
    }

    let phantom: Vec<&&String> = accounted.iter().filter(|f| !actual.contains(**f)).collect();
    if !phantom.is_empty() {
        return Err(format!(
            "{phantom:?} are named by the map or census but do not exist in the PDF — a stale \
             entry, which is how a closed gap silently reopens"
        ));
    }

    let unaccounted: Vec<&String> = actual.iter().filter(|f| !accounted.contains(f)).collect();

    match (has_census, allowance) {
        // A written census means 100% accounting, with no allowance of any kind.
        (true, None) => {
            if !unaccounted.is_empty() {
                return Err(format!(
                    "{} field(s) are in NEITHER the map nor the [census] — this is the \"we forgot \
                     this line\" defect, invisible on the printed page and to both oracles: \
                     {unaccounted:#?}",
                    unaccounted.len()
                ));
            }
            Ok(())
        }
        (true, Some(n)) => Err(format!(
            "this map HAS a [census] section but is still on the UNCENSUSED register (allowance \
             {n}) — a stale excuse is how a closed gap reopens; delete the register line"
        )),
        // No census section at all: the count must match the register EXACTLY.
        (false, None) => Err(format!(
            "this map has NO [census] section and NO entry on the UNCENSUSED register, so {} \
             field(s) carry no recorded decision and nothing says so. Either write the [census], or \
             record the year on the register. A new forms/<year>/ lands here, which is the point.",
            unaccounted.len()
        )),
        (false, Some(n)) if unaccounted.len() != n => Err(format!(
            "recorded {n} unaccounted field(s), measured {}. The register is SHRINK-ONLY: it goes \
             down by writing [census] entries (or mapping the field), never by editing the number \
             up. If the count genuinely fell, lower the register line in the same commit.",
            unaccounted.len()
        )),
        (false, Some(_)) => Ok(()),
    }
}

/// ★★★ **THE GATE.** Every committed `(year, stem)` on disk — not one pinned year — must account for
/// its PDF's field set exactly, or carry an exact [`UNCENSUSED`] allowance.
#[test]
fn census_accounts_for_every_field() {
    let years = every_bundled_year();
    let mut failures: Vec<String> = Vec::new();
    let mut checked = 0usize;

    for year in &years {
        let stems = stems_for(*year);
        for stem in &stems {
            let (mapped, census, has_census) =
                map_and_census(stem, *year).expect("map file exists");
            let actual = acroform_fqns(stem, *year).unwrap_or_else(|| {
                panic!("forms/{year}/{stem}.pdf must exist and parse — a map with no readable form")
            });
            let allowance = UNCENSUSED
                .iter()
                .find(|(y, s, _)| y == year && s == stem)
                .map(|(_, _, n)| *n);

            if let Err(e) = verdict(has_census, allowance, &mapped, &census, &actual) {
                failures.push(format!("{year}/{stem}: {e}"));
            }
            checked += 1;
        }
    }

    assert!(
        failures.is_empty(),
        "the field census gate failed for {} of {checked} committed maps:\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
    assert!(
        checked >= 20,
        "the gate walked only {checked} maps across {years:?} — it must actually check the forms, \
         or it passes by finding nothing"
    );
    eprintln!("field census: {checked} committed maps checked across years {years:?}");
}

/// ★★ **B1 — the gate watched going RED on every defect class it claims to catch.**
///
/// Each plant is the minimal mutation of an exactly-accounted form. If any of these starts returning
/// `Ok`, the corresponding arm of [`verdict`] has been gutted and
/// [`census_accounts_for_every_field`] is reporting green over a blind check.
#[test]
fn the_gate_reds_on_every_planted_defect() {
    let set = |v: &[&str]| -> BTreeSet<String> { v.iter().map(|s| s.to_string()).collect() };
    let actual = set(&[
        "Page1[0].f1_01[0]",
        "Page1[0].f1_02[0]",
        "Page1[0].f1_03[0]",
    ]);

    // The clean baseline: mapped ∪ census == actual, disjoint, census written, no allowance.
    let mapped = set(&["Page1[0].f1_01[0]", "Page1[0].f1_02[0]"]);
    let census = set(&["Page1[0].f1_03[0]"]);
    assert!(
        verdict(true, None, &mapped, &census, &actual).is_ok(),
        "the baseline must PASS, or every red below is meaningless"
    );

    /// One planted defect: (what it is, census-written, allowance, mapped set, census set, the
    /// substring the refusal must contain). Named so `-D clippy::type-complexity` is satisfied by
    /// making the shape readable rather than by an `allow`.
    type Plant = (
        &'static str,
        bool,
        Option<usize>,
        BTreeSet<String>,
        BTreeSet<String>,
        &'static str,
    );

    let plants: &[Plant] = &[
        // (1) the "we forgot this line" defect — a field in neither set.
        (
            "a field in NEITHER the map nor the census",
            true,
            None,
            set(&["Page1[0].f1_01[0]", "Page1[0].f1_02[0]"]),
            BTreeSet::new(),
            "we forgot",
        ),
        // (2) the contradiction — one field both filled and deliberately left blank.
        (
            "a field in BOTH the map and the census",
            true,
            None,
            set(&[
                "Page1[0].f1_01[0]",
                "Page1[0].f1_02[0]",
                "Page1[0].f1_03[0]",
            ]),
            set(&["Page1[0].f1_03[0]"]),
            "BOTH mapped and censused",
        ),
        // (3) the stale entry — a name that no longer exists in the PDF (the year-port defect: a
        //     renamed field leaves the old name behind and the gap silently reopens).
        (
            "a name that does not exist in the PDF",
            true,
            None,
            set(&[
                "Page1[0].f1_01[0]",
                "Page1[0].f1_02[0]",
                "Page1[0].f1_99[0]",
            ]),
            set(&["Page1[0].f1_03[0]"]),
            "do not exist in the PDF",
        ),
        // (4) ★ THE YEAR-PORT DEFECT: a new forms/<year>/ arrives with no census and no register
        //     entry. This is the arm that made the old `let year = 2024;` gate report success.
        (
            "an uncensused form with no register entry (a fresh year)",
            false,
            None,
            set(&["Page1[0].f1_01[0]"]),
            BTreeSet::new(),
            "NO entry on the UNCENSUSED register",
        ),
        // (5) the register grew — more fields lost their decision than are recorded.
        (
            "an uncensused form whose unaccounted count ROSE above the register",
            false,
            Some(1),
            set(&["Page1[0].f1_01[0]"]),
            BTreeSet::new(),
            "SHRINK-ONLY",
        ),
        // (6) the register is stale low — progress made and not recorded, so the number stops
        //     meaning anything.
        (
            "an uncensused form whose unaccounted count FELL below the register",
            false,
            Some(9),
            set(&["Page1[0].f1_01[0]"]),
            BTreeSet::new(),
            "SHRINK-ONLY",
        ),
        // (7) the stale excuse — a census was written but the allowance was left behind.
        (
            "a censused form still carrying a register allowance",
            true,
            Some(3),
            set(&["Page1[0].f1_01[0]", "Page1[0].f1_02[0]"]),
            set(&["Page1[0].f1_03[0]"]),
            "stale excuse",
        ),
    ];

    for (name, has_census, allowance, m, c, needle) in plants {
        let got = verdict(*has_census, *allowance, m, c, &actual);
        let msg = match &got {
            Ok(()) => panic!(
                "PLANTED DEFECT NOT CAUGHT — {name}: the gate returned Ok. This checker does not \
                 exist (B1)."
            ),
            Err(e) => e.clone(),
        };
        assert!(
            msg.contains(needle),
            "{name}: the gate red for the WRONG reason — expected a message naming {needle:?}, got \
             {msg:?}"
        );
    }
}

/// ★★★ `rule = "gap"` is a countable DEFECT, not an exemption — and it may only shrink.
///
/// A `gap` is a field btctax **cannot honestly account for**: a required declaration it never asks.
/// Censusing `schedule_se` found the first one — line A, the Form 4361 minister declaration. A
/// minister with $400+ of other self-employment earnings would file an incomplete Schedule SE and
/// nothing would say so.
///
/// ★ Without this test, `gap` would be exactly the escape hatch the census exists to remove: a way to
/// mark a field "accounted for" while accounting for nothing. Pinning the count means a new gap
/// cannot be added quietly, and closing one (by adding a `QuestionId`) must lower the number.
///
/// ★ 2026-09-05: this scan is now over **every** bundled year, not `forms/2024/` alone. A TY2026 port
/// that carries a TY2025 census reason forward as a `gap` is a new gap, and it now lands here.
#[test]
fn recorded_gaps_may_only_shrink() {
    /// Every `rule = "gap"` in the committed maps. Closing one means deleting its census entry and
    /// mapping the field to a real question — at which point this comes down.
    // ★ Counted per FIELD, not per question: the FBAR pair is TWO fields (the Yes and No halves of
    // one radio) for ONE logical question. Fields is the mechanical unit — a question-level count
    // would need someone to decide what "one question" means, which is exactly the judgement this
    // ratchet is meant to avoid depending on.
    // 2026-07-30: 13 → 12. Form 8995 line 3 (`f1_19`) was a gap — the prior-year QBI loss
    // carryforward, SUBTRACTED at line 4, that btctax neither modelled nor asked for, so a filer who
    // had one got an INFLATED deduction and UNDERSTATED tax. Closed the way the message below
    // demands: the field is now MAPPED and the input collected, and its census entry deleted because
    // it is no longer unaccounted-for — not because the record was inconvenient.
    // 2026-07-31: 12 → 10. Form 8283's PAGE-2 identity header (`f2_01`/`f2_02`) was a gap of the
    // "we have the datum and nothing connects it to the field" species — btctax held the name and TIN
    // and wrote them to page 1, while a filed page 2 went out unidentified. Closed by MAPPING the
    // cells (`[identity_page2]`), not by re-describing the omission.
    // 2026-07-31: 10 → 6. Schedule C lines I and J (4 fields) — the Form-1099 compliance pair — are
    // now ASKED as class-(B) skippables and PRINTED from the filer's own answer, with a §6721/§6722
    // advisory on the skip. Closed by collecting the input, which is the only way the message below
    // permits.
    // 2026-07-31: unchanged at 6 (all six are Form 8283 lines 5a/5b/5c). ★ Form 1040 line 7's
    // "if not required, check here" box was NOT a gap — it was recorded `unmodeled` on the reasoning
    // that "Schedule D is always required", which is FALSE (`ScheduleDLines::must_file` exists
    // precisely because it is not). §G-18: the census REASON was wrong, not the count. The field is
    // now mapped and written; see the map's own note.
    // ★★★ 2026-07-31: 6 → 0. THE CENSUS'S GAP SURFACE IS CLOSED. The last six were Form 8283
    // Section B lines 5a/5b/5c, asked as ONE return-level universal ("did any donation have strings
    // attached?") and printed as three Nos from the filer's own answer; a YES refuses the year.
    // 2026-09-05: still 0, now measured over ALL bundled years (2017/2024/2025 = 37 maps), not
    // TY2024's 17. The widened scan added nothing, which is itself a measurement: the uncensused
    // years have no `gap` records because they have no census records at all.
    //
    // ★ Zero is not the end of the census — it is the ratchet at rest. `<` is still the only legal
    // direction, and every field must still be accounted for, so a NEW gap can be added (that is what
    // the register is for) but not silently.
    const GAPS: usize = 0;

    let mut found = Vec::new();
    let mut scanned = 0usize;
    for year in every_bundled_year() {
        for stem in stems_for(year) {
            let text = std::fs::read_to_string(map_path(&stem, year)).expect("map file");
            scanned += 1;
            for line in text.lines() {
                // ★ Skip comments — the section's own legend explains `rule = "gap"`, and counting
                // that line made this test report 2 gaps where the form has 1. A census that
                // miscounts its own defects is worse than none.
                if line.trim_start().starts_with('#') {
                    continue;
                }
                if line.contains(r#"rule = "gap""#) {
                    found.push(format!(
                        "{year}/{stem}: {}",
                        line.split('"').nth(1).unwrap_or("?")
                    ));
                }
            }
        }
    }
    assert!(
        scanned >= 20,
        "the gap scan opened only {scanned} maps — it must read the committed forms, not nothing"
    );
    assert_eq!(
        found.len(),
        GAPS,
        "recorded gaps changed (pinned {GAPS}, scanned {scanned} maps). A gap is a field btctax \
         CANNOT honestly account for — a required declaration it never asks — so the count must only \
         go DOWN, and it goes down by adding a QuestionId, never by deleting the record. Found: \
         {found:#?}"
    );
}

/// ★★ **The uncensused register may only shrink, and may not name a form that is not there.**
///
/// The per-form counts are pinned by the gate itself; this pins the register's *shape*, so that
/// adding a line — the way a new year would try to buy itself a pass — changes a number here too.
#[test]
fn the_uncensused_register_may_only_shrink() {
    assert_eq!(
        UNCENSUSED.len(),
        UNCENSUSED_ENTRIES,
        "the UNCENSUSED register gained or lost entries (pinned {UNCENSUSED_ENTRIES}). An entry is \
         a whole form whose fields carry no recorded decision; it is removed by WRITING the \
         [census], never by deleting the line"
    );
    let total: usize = UNCENSUSED.iter().map(|(_, _, n)| n).sum();
    assert_eq!(
        total, UNCENSUSED_FIELDS,
        "the register totals {total} unaccounted fields, pinned {UNCENSUSED_FIELDS}. This number is \
         the census's outstanding debt across every bundled year — TY2017's 403 and TY2025's 330 — \
         and DOWN is the only legal direction"
    );

    // ★ A register line naming a `(year, stem)` that is not on disk is an excuse for nothing: it
    // would sit there granting an allowance no gate ever consumes, and the next year's port would
    // inherit it. Derived from the filesystem, so it cannot go stale.
    let on_disk: BTreeSet<(i32, String)> = every_bundled_year()
        .into_iter()
        .flat_map(|y| stems_for(y).into_iter().map(move |s| (y, s)))
        .collect();
    let orphans: Vec<String> = UNCENSUSED
        .iter()
        .filter(|(y, s, _)| !on_disk.contains(&(*y, s.to_string())))
        .map(|(y, s, n)| format!("{y}/{s} (allowance {n})"))
        .collect();
    assert!(
        orphans.is_empty(),
        "the UNCENSUSED register names {} (year, stem) pair(s) with no committed map: {orphans:?}",
        orphans.len()
    );
}

/// ★★ **No emittable form escapes the gate — and a form that is ABSENT from a year is NAMED, never
/// silently counted as clean.**
///
/// `Stem::ALL` (the closed set of forms this crate can fill) is the one authority for the set of forms `fill_full_return` can emit. The
/// gate walks the filesystem, so a form with no committed map for a year is simply not walked —
/// exactly the "skipping is not passing" shape. each year's `YEAR.toml` `[forms_absent]` records those absences per
/// year, derived-against and pinned, so deleting a committed map reds here instead of quietly
/// shrinking the gate's field of view.
#[test]
fn every_emittable_form_is_reached_by_the_gate_or_named_absent() {
    use btctax_forms::bundled::Stem;
    use btctax_forms::year_record::YearRecord;
    let emittable: Vec<&str> = Stem::ALL.iter().map(|s| s.file_stem()).collect();
    let years = every_bundled_year();

    // The register must cover every year on disk — a new forms/<year>/ has no row, and reds.
    let registered: BTreeSet<i32> = years
        .iter()
        .copied()
        .filter(|y| YearRecord::for_year(*y).is_some())
        .collect();
    let unregistered: Vec<&i32> = years.iter().filter(|y| !registered.contains(y)).collect();
    assert!(
        unregistered.is_empty(),
        "years {unregistered:?} are on disk but have no forms/<year>/YEAR.toml — the gate \
         would walk whatever maps happen to exist and say nothing about the forms that do not"
    );

    let mut wrong: Vec<String> = Vec::new();
    let mut complete_years: Vec<i32> = Vec::new();
    for year in &years {
        let present: BTreeSet<String> = stems_for(*year).into_iter().collect();
        let measured: BTreeSet<&str> = emittable
            .iter()
            .copied()
            .filter(|k| !present.contains(*k))
            .collect();
        let record = YearRecord::for_year(*year).expect("registered above");
        let recorded: BTreeSet<&str> = record.forms_absent.keys().map(String::as_str).collect();
        if measured != recorded {
            wrong.push(format!(
                "{year}: recorded absent {recorded:?}, measured absent {measured:?}"
            ));
        }
        // "Complete" is the year's DECLARATION (`status = "filable"`), held above to what is on disk —
        // not "every one of the 18 forms present": Schedule 1-A exists for TY2025+ only, so no year
        // can bundle all of `Stem::ALL`, and each absence carries its reason in the record.
        if record.status == btctax_forms::year_record::YearStatus::Filable {
            complete_years.push(*year);
        }
    }
    assert!(
        wrong.is_empty(),
        "the per-year form coverage moved:\n  {}\nA form appearing is progress and lowers the row; \
         a form DISAPPEARING is a deleted map and must not pass as a shrunken gate.",
        wrong.join("\n  ")
    );
    assert!(
        !complete_years.is_empty(),
        "no bundled year carries a map for every form fill_full_return can emit — there is no year \
         the packet can actually be filled for"
    );
    eprintln!("complete years (every emittable form mapped): {complete_years:?}");
}
