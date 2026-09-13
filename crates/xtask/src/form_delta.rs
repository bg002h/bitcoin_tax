//! **`xtask form-delta <old-stem> <new-stem>` — exactly what changed between two revisions of a form.**
//!
//! ★★★ **This exists to make draft→final a DIFF rather than a rebuild.** The TY2026 drafts are
//! archived now; the finals land Nov 2026 – Jan 2027. Everything built against a draft — field maps,
//! line numbering, struct shapes — has to be re-verified when the final arrives, and the difference
//! between "re-verify" and "redo" is whether the work list is computed or remembered.
//!
//! ★★ It reports the TWO axes independently, because they fail independently and the second is the
//! one that is normally missed:
//!
//! * **field-name churn** — names added, removed, or renamed. Loud, and a name-existence check
//!   already catches it.
//! * **line→label drift** — a field whose NAME is unchanged but which now sits beside a different
//!   printed line number. Measured on the real artifacts: TY2025's Form 6251 renamed **zero** page-1
//!   fields and still moved **12 of 41** mapped lines, because one added field walked everything
//!   below it down. An existence check passes on that with 0 of 61 names absent, and the AMT prints
//!   in line 10's box.
//!
//! Calibrated on pairs where BOTH sides are real: `f6251--2024` → `f6251--2025` (0 renames, 12 label
//! moves) and `f8995--2024` → `f8995--2025` (18 renames, 0 label moves). Those two are opposite
//! shapes, which is why both are pinned in the tests.
//!
//! ★★★ **AND THE VERDICT REPORTS ITS OWN EVIDENCE.** The label axis used to silently drop every
//! field it could not read — no box in the geometry, or a box no printed label claims (`?`) — and
//! then print *"no field changed the printed line it sits beside"* over whatever was left, including
//! nothing. That is the cleanest possible verdict from zero comparisons, and it is the product's
//! organising failure in miniature: the FORM fails closed, the INSTRUMENT fails open.
//!
//! Measured 2026-09-05 over the 13 archived `--2025` → `--2026-DRAFT` pairs: **604 of 664 common
//! fields were actually compared**, so 60 fields were being folded into verdicts that said nothing
//! about them. The extremes:
//!
//! | pair | common | actually compared | what it printed then | what it prints now |
//! |---|---|---|---|---|
//! | `f8949--2025` → `f8949--2026-DRAFT` | 202 | **186** | "no field changed" | 186 compared, none moved, **16 named as unwitnessed** |
//! | `f1040sc--2025` → `f1040sc--2026-DRAFT` | 59 | **39** | "8 fields moved" | 8 moved **of 39 compared**, 20 named |
//! | `f8960--2025` → `f8960--2026-DRAFT` | 38 | **33** | "no field changed" | 33 compared, none moved, **5 named** |
//! | `f6251--2024` → `f6251--2025` | 61 | **59** | "30 fields moved" | 30 moved of 59 compared, 2 named |
//!
//! So a "no change" claim now always carries the count it rests on, an unwitnessed field is named
//! with the reason it could not be read, and **zero comparisons is its own verdict**
//! ([`LabelVerdict::Unwitnessed`]) that exits non-zero — never a clean one.
//!
//! ★ The counts above are what the tool prints today; re-run it rather than trusting this table,
//! which is a dated measurement and not the authority. `f1040` is absent from it because
//! `design/forms/2026/f1040--2026-DRAFT.pdf` was withdrawn the same day (it was the TY2025 form).

use std::collections::{BTreeMap, BTreeSet};

/// Where a form's PDF lives: the bundled template if there is one, else the archived authority copy.
///
/// ★★ **This is the ARCHIVE oracle's reader, and NOT how field spellings are obtained (FR-165).**
/// It answers *"is this side of a diff in the archive at all"* for the excused rows `port-status`
/// prints, and for nothing else. The bundled templates under `crates/btctax-forms/forms/` are
/// COMMITTED, so that question has the same answer in CI as on a developer machine for every stem
/// the emitting surface asks about; the archived copies under `design/forms/` are gitignored, and a
/// stem that has only one of those answers `false` in both places. See
/// [`tests::real_archive`] for the oracle and its two kills.
fn pdf_for(stem: &str) -> Option<std::path::PathBuf> {
    let root = crate::form_geometry::repo_root();
    let (form, year) = stem.split_once("--")?;
    let year_dir = year.trim_end_matches("-DRAFT");
    let bundled = root.join(format!("crates/btctax-forms/forms/{year_dir}/{form}.pdf"));
    if bundled.is_file() {
        return Some(bundled);
    }
    let archived = root.join(format!("design/forms/{year_dir}/{stem}.pdf"));
    archived.is_file().then_some(archived)
}

/// One revision's AcroForm field spellings, read from the **committed** geometry observation.
///
/// ★★★ **This read the PDF until 2026-09-13, and that was FR-165.** `design/forms/**/*.pdf` is
/// gitignored on purpose, so six tests in this file passed on the machine that had the PDFs and could
/// not pass on any fresh checkout: CI's `test` job was red on ubuntu, macOS and Windows for eight
/// days (`209 passed; 6 failed`, last green `2bd04d458`). The dependency was the defect, not the
/// tests.
///
/// ★★ **No new fixture was needed.** `design/forms/geometry/<stem>.json` already carries every
/// AcroForm box with its name — the same committed observation the line→label axis below reads, so
/// both axes of this tool now rest on one artifact rather than two. Measured 2026-09-13 over all 70
/// committed fixtures: `boxes[].name` is IDENTICAL to the PDF's AcroForm field set on every one — 0
/// names only in the PDF, 0 only in the fixture, and `boxes.len()` equal to the PDF's whole field
/// count on all 70.
///
/// ★ **The one thing a fixture cannot carry, stated rather than hidden.** `extract-geometry` DROPS a
/// field with no widget rect — it has no position, so it cannot join to a printed label — and that
/// drop is the only way this set can differ from the PDF's. There are 0 such fields today, and
/// `xtask extract-geometry --all --check` is what keeps it that way: it regenerates every fixture
/// from the PDF, refuses any whose bytes differ, and refuses any whose box count is not the PDF's
/// full AcroForm field count. **That checker is where the PDF belongs.** A fallback to the PDF here
/// when one happened to be present would be worse than no check at all, because the committed
/// fixture would then be the thing under test only where the PDF is absent — i.e. never on a
/// developer machine.
fn field_set(stem: &str) -> Result<BTreeSet<String>, String> {
    let g = crate::form_geometry::load(&crate::form_geometry::repo_root(), stem)?;
    Ok(g.boxes.into_iter().map(|b| b.name).collect())
}

/// The last `k` dot-separated segments of an AcroForm FQN. `k` at or above the depth is the whole
/// name, which is why the ladder below can never pair fewer boxes than an exact-FQN intersection.
fn suffix(name: &str, k: usize) -> &str {
    match name
        .char_indices()
        .rev()
        .filter(|(_, c)| *c == '.')
        .nth(k - 1)
    {
        Some((i, _)) => &name[i + 1..],
        None => name,
    }
}

/// **Axis A — FR-190. Which box on the OLD side is which box on the NEW side.**
///
/// ★★★ **Keyed on the full AcroForm FQN, this axis silently EMPTIES when a container is renamed —
/// and the blindness is CORRELATED with the thing the tool exists to find.** `f1040s3--2021`'s root
/// subform is `form1[0]`; `f1040s3--2022`'s is `topmostSubform[0]`. Nothing else about the 41 boxes
/// differs. On a full-FQN key the intersection is **0**, so `form-delta` printed
/// `0 common / 40 added / 41 removed` and its loudest banner — *"LINE→LABEL DRIFT UNWITNESSED"*,
/// exit 1 — for the calmest transition in the corpus. Across the 58 consecutive pairs the committed
/// fixtures can form, a full-FQN key pairs **3,373** boxes; this ladder pairs **4,177**.
///
/// ★★ A container is renamed *because* lines were renumbered (`Lines4a-11_ReadOrder` →
/// `Line4a-11_ReadOrder` costs 11 boxes on Form 8949), so the FQN key drops boxes **preferentially
/// where the axis matters most**. That is loss concentrated on exactly the transitions being analysed,
/// not random loss.
///
/// **The ladder.** For `k` from the deepest FQN down to 1, pair the still-unpaired boxes whose last
/// `k` segments are equal **and unique on both sides among the still-unpaired**. Three properties
/// make it safe to relax this far:
///
/// * `k` at the maximum depth is the full FQN, so **every exact match is taken first** and the ladder
///   is monotone — it can never pair fewer than the old key did. Held by
///   [`tests::the_ladder_never_pairs_fewer_boxes_than_the_full_fqn_key`].
/// * uniqueness is required on **both** sides, so a leaf that two copies of a form share (a W-2 has
///   `f2_01[0]` in every copy) is left UNPAIRED and named, never merged. `fw2--2024` → `--2025` pairs
///   all 272 boxes at the top of the ladder and never reaches the leaf at all.
/// * the full FQN is kept for REPORTING: a pair whose spelling changed is listed in
///   [`Pairing::renamed`], because the bundled map stores full FQNs and every one of those is a port
///   edit.
pub struct Pairing {
    /// old FQN → new FQN, for every box the two revisions share.
    pub pairs: BTreeMap<String, String>,
    /// Of those, the ones whose FQN text changed: old → new. Each is a map edit.
    pub renamed: BTreeMap<String, String>,
    /// New-side FQNs no old-side box paired with.
    pub added: BTreeSet<String>,
    /// Old-side FQNs no new-side box paired with.
    pub removed: BTreeSet<String>,
    /// Boxes left unpaired **because a suffix was ambiguous on one side** rather than because the box
    /// is gone: `(side, fqn)`. Named rather than folded into added/removed, because "two copies of
    /// this form share this leaf" is a different fact from "this box was retired".
    pub ambiguous: BTreeSet<(&'static str, String)>,
}

pub fn pair_fields(old: &BTreeSet<String>, new: &BTreeSet<String>) -> Pairing {
    let mut ua: BTreeSet<&str> = old.iter().map(String::as_str).collect();
    let mut ub: BTreeSet<&str> = new.iter().map(String::as_str).collect();
    let depth = |s: &str| s.matches('.').count() + 1;
    let max_k = old
        .iter()
        .chain(new.iter())
        .map(|s| depth(s))
        .max()
        .unwrap_or(1);
    let mut pairs = BTreeMap::new();
    for k in (1..=max_k).rev() {
        let mut ma: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        let mut mb: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for n in &ua {
            ma.entry(suffix(n, k)).or_default().push(n);
        }
        for n in &ub {
            mb.entry(suffix(n, k)).or_default().push(n);
        }
        for (key, a) in &ma {
            let Some(b) = mb.get(key) else { continue };
            if a.len() == 1 && b.len() == 1 {
                pairs.insert(a[0].to_string(), b[0].to_string());
            }
        }
        for (o, n) in &pairs {
            ua.remove(o.as_str());
            ub.remove(n.as_str());
        }
    }
    // An unpaired box whose LEAF exists on the other side was blocked by ambiguity, not retired, so
    // it is named separately and kept OUT of added/removed — which is what makes those two counts
    // mean what they say. The accounting invariant is
    //   old.len() == pairs + removed + ambiguous(old)   and   new.len() == pairs + added + ambiguous(new),
    // held by `tests::every_box_is_paired_added_removed_or_named_ambiguous`.
    let leaves_a: BTreeSet<&str> = old.iter().map(|n| suffix(n, 1)).collect();
    let leaves_b: BTreeSet<&str> = new.iter().map(|n| suffix(n, 1)).collect();
    let mut ambiguous = BTreeSet::new();
    let (mut added, mut removed) = (BTreeSet::new(), BTreeSet::new());
    for n in &ua {
        if leaves_b.contains(suffix(n, 1)) {
            ambiguous.insert(("old", (*n).to_string()));
        } else {
            removed.insert((*n).to_string());
        }
    }
    for n in &ub {
        if leaves_a.contains(suffix(n, 1)) {
            ambiguous.insert(("new", (*n).to_string()));
        } else {
            added.insert((*n).to_string());
        }
    }
    Pairing {
        renamed: pairs
            .iter()
            .filter(|(o, n)| o != n)
            .map(|(o, n)| (o.clone(), n.clone()))
            .collect(),
        added,
        removed,
        ambiguous,
        pairs,
    }
}

/// The label `label_reader` gives a box that **no printed line label claims** — its
/// BOX-WITH-NO-LABEL marker (`label_reader::boxes_tsv`: *"A box no label claims emits `?` — that is
/// BOX-WITH-NO-LABEL, and it is a hard finding, not a blank cell."*).
///
/// It is a finding, never a value, so it can never be compared against another one. What it must
/// never do is vanish: `"?" == "?"` is not "this line did not move".
const NO_LABEL: &str = "?";

/// Why one common field contributed **no evidence** to the line→label axis.
///
/// ★★ Each of these used to be an `if let` that fell through to nothing, which made a field the tool
/// *could not read* indistinguishable from a field it read and found unmoved. A checker that cannot
/// tell "this box encodes no printed line" from "we never looked at this box" is not a check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Unwitnessed {
    /// One side's label set could not be read at all, so the axis never ran for any field.
    ///
    /// ★★ **The NAME is historical and FR-165 narrowed what reaches it.** Before 2026-09-13 the
    /// commonest way in was a side with a PDF and no geometry fixture. `field_set` now reads the
    /// fixture, so that state is a hard `Err` out of [`compute`] — naming the missing file and the
    /// command that makes it — and what reaches this variant is the *other* way `label_join` fails:
    /// a fixture that exists and whose words yield no printed label column at all.
    GeometryFixtureMissing,
    /// Both sides have geometry, and neither records a box by this name.
    BoxAbsentBothSides,
    /// The new side records a box by this name; the old side's geometry does not.
    BoxAbsentOld,
    /// The old side records a box by this name; the new side's geometry does not.
    BoxAbsentNew,
    /// The box exists on both sides and no printed line label claims it on either — the header
    /// name/SSN boxes, checkboxes, and address rows are the honest members of this class.
    UnlabelledBothSides,
    /// The new side's box carries a printed label; the old side's is BOX-WITH-NO-LABEL.
    UnlabelledOld,
    /// The old side's box carries a printed label; the new side's is BOX-WITH-NO-LABEL.
    UnlabelledNew,
}

impl Unwitnessed {
    /// A sentence a person can act on. Printed beside the field name, never in place of it.
    pub fn why(self) -> &'static str {
        match self {
            Unwitnessed::GeometryFixtureMissing => {
                "one side's label set could not be read from its geometry fixture — a missing \
                 fixture is a hard error before this point, so this means the fixture is there and \
                 yields no printed label column"
            }
            Unwitnessed::BoxAbsentBothSides => "neither side's geometry records a box by this name",
            Unwitnessed::BoxAbsentOld => "the OLD side's geometry records no box by this name",
            Unwitnessed::BoxAbsentNew => "the NEW side's geometry records no box by this name",
            Unwitnessed::UnlabelledBothSides => {
                "BOX-WITH-NO-LABEL on both sides — no printed line claims this box"
            }
            Unwitnessed::UnlabelledOld => "BOX-WITH-NO-LABEL on the OLD side",
            Unwitnessed::UnlabelledNew => "BOX-WITH-NO-LABEL on the NEW side",
        }
    }
}

/// Can this one field's two labels be compared at all? `Ok` is evidence; `Err` says why there is
/// none. There is deliberately no third answer, so a caller cannot drop a field by forgetting it.
fn witness(old: Option<&str>, new: Option<&str>) -> Result<(String, String), Unwitnessed> {
    let (o, n) = match (old, new) {
        (None, None) => return Err(Unwitnessed::BoxAbsentBothSides),
        (None, Some(_)) => return Err(Unwitnessed::BoxAbsentOld),
        (Some(_), None) => return Err(Unwitnessed::BoxAbsentNew),
        (Some(o), Some(n)) => (o, n),
    };
    match (o == NO_LABEL, n == NO_LABEL) {
        (true, true) => Err(Unwitnessed::UnlabelledBothSides),
        (true, false) => Err(Unwitnessed::UnlabelledOld),
        (false, true) => Err(Unwitnessed::UnlabelledNew),
        (false, false) => Ok((o.to_string(), n.to_string())),
    }
}

/// The line→label axis, with its own evidence attached.
///
/// The invariant that makes it a check rather than a summary: **`compared + unwitnessed.len()`
/// equals the number of common fields, always.** Every field is either evidence or a named gap.
pub struct LabelAxis {
    /// How many common fields yielded a printed label on BOTH sides. The verdict rests on exactly
    /// this many observations, and **zero is not clean**.
    pub compared: usize,
    /// Of those compared, the ones whose label changed: name → (old label, new label).
    pub moved: BTreeMap<String, (String, String)>,
    /// Every common field that yielded no comparison, by name, with the reason.
    pub unwitnessed: BTreeMap<String, Unwitnessed>,
}

/// Compare two label maps over a **pairing** (Axis A), not over a set of identical names. Pure — no
/// filesystem — so the zero-evidence case can be tested without waiting for a form to develop one.
///
/// ★ FR-190: the map is keyed by the OLD FQN and read on each side with that side's own spelling, so
/// a renamed container no longer removes a box from the axis.
pub fn label_axis(
    pairs: &BTreeMap<String, String>,
    old: &BTreeMap<String, String>,
    new: &BTreeMap<String, String>,
) -> LabelAxis {
    let mut axis = LabelAxis {
        compared: 0,
        moved: BTreeMap::new(),
        unwitnessed: BTreeMap::new(),
    };
    for (f, g) in pairs {
        match witness(
            old.get(f).map(String::as_str),
            new.get(g).map(String::as_str),
        ) {
            Ok((o, n)) => {
                axis.compared += 1;
                if o != n {
                    axis.moved.insert(f.clone(), (o, n));
                }
            }
            Err(u) => {
                axis.unwitnessed.insert(f.clone(), u);
            }
        }
    }
    axis
}

/// What the line→label axis is entitled to claim, given what it actually managed to read.
///
/// ★★★ The two evidence-free outcomes are **separate variants**, not an empty `moved` map, because
/// an empty map is what "nothing moved" and "nothing was looked at" both used to look like.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelVerdict {
    /// The axis never ran: one side has no geometry fixture.
    NoFixture { unwitnessed: usize },
    /// The axis ran and resolved a printed label on BOTH sides for **zero** common fields.
    Unwitnessed { unwitnessed: usize },
    /// `compared` bindings were actually compared and none of them moved.
    Unchanged { compared: usize, unwitnessed: usize },
    /// `compared` bindings were actually compared and `moved` of them changed line.
    Moved {
        compared: usize,
        moved: usize,
        unwitnessed: usize,
    },
}

impl LabelVerdict {
    /// How many field bindings this verdict rests on. **A "no change" claim with zero here is the
    /// absence of evidence, not evidence of absence.**
    pub fn compared(self) -> usize {
        match self {
            LabelVerdict::NoFixture { .. } | LabelVerdict::Unwitnessed { .. } => 0,
            LabelVerdict::Unchanged { compared, .. } | LabelVerdict::Moved { compared, .. } => {
                compared
            }
        }
    }

    /// How many common fields this verdict could not read, and therefore says nothing about.
    pub fn unwitnessed(self) -> usize {
        match self {
            LabelVerdict::NoFixture { unwitnessed }
            | LabelVerdict::Unwitnessed { unwitnessed }
            | LabelVerdict::Unchanged { unwitnessed, .. }
            | LabelVerdict::Moved { unwitnessed, .. } => unwitnessed,
        }
    }

    /// Whether this run is entitled to be read as a checked one. False for both evidence-free
    /// verdicts, and it is what `run` turns into a non-zero exit.
    pub fn is_witnessed(self) -> bool {
        self.compared() > 0
    }
}

// ────────────────────────── Axis B — the line SET (FR-191) ──────────────────────────

/// **Axis B — which printed line NUMBERS this revision retired and introduced.**
///
/// ★★ **`label-census` has always been able to answer this; nothing joined the two instruments.**
/// Schedule 8812 (`f1040s8`) TY2021 → TY2022 killed **34 printed lines**, and `form-delta`'s output
/// was *"54 removed"* AcroForm field names, eight of them shown, none carrying a line label. Across
/// the corpus 62 retired line numbers were reported as zero. A retired line matters because code that
/// still reads it reads a **blank**, and on the printed page a blank and a zero are the same thing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineSetAxis {
    /// Printed line labels on the old revision only — retired.
    pub retired: BTreeSet<String>,
    /// Printed line labels on the new revision only — introduced.
    pub introduced: BTreeSet<String>,
    /// Printed on both.
    pub survived: BTreeSet<String>,
}

pub fn line_set_axis(old: &BTreeSet<String>, new: &BTreeSet<String>) -> LineSetAxis {
    LineSetAxis {
        retired: old.difference(new).cloned().collect(),
        introduced: new.difference(old).cloned().collect(),
        survived: old.intersection(new).cloned().collect(),
    }
}

// ────────────────────────── Axis C — the line MEANING (FR-192) ──────────────────────────

/// Why one surviving line number contributed **no caption comparison**.
///
/// ★ Same discipline as [`Unwitnessed`]: a line the reader could not read must never be
/// indistinguishable from a line it read and found unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CaptionGap {
    /// The old revision prints this label more than once, so its caption is ambiguous.
    AmbiguousOld,
    /// The new revision prints this label more than once.
    AmbiguousNew,
    /// The reader found no describing word in this line's span on the old side.
    EmptyOld,
    /// …on the new side.
    EmptyNew,
    /// On neither side.
    EmptyBothSides,
}

impl CaptionGap {
    pub fn why(self) -> &'static str {
        match self {
            CaptionGap::AmbiguousOld => {
                "the OLD revision prints this label more than once — its caption is ambiguous and \
                 picking one would be a guess"
            }
            CaptionGap::AmbiguousNew => "the NEW revision prints this label more than once",
            CaptionGap::EmptyOld => {
                "no describing word in this line's span on the OLD side — the reader said nothing \
                 about it, which is not the same as finding it unchanged"
            }
            CaptionGap::EmptyNew => "no describing word in this line's span on the NEW side",
            CaptionGap::EmptyBothSides => "no describing word in this line's span on either side",
        }
    }
}

/// Where a changed line's OLD text turned up on the new revision.
#[derive(Debug, Clone, PartialEq)]
pub enum Travel {
    /// The same caption, token for token, under a different number.
    Verbatim { to: String },
    /// Nearly the same caption under a different number — **the line MOVED and was REWORDED**, which
    /// is FR-187's shape: the casualty line went 15 → 16 *and* widened from *"from a federally
    /// declared disaster"* to *"from a federally or state-declared disaster"*. A move table recording
    /// "15 → 16, same quantity" is right about the number and wrong about the rule.
    Reworded { to: String, similarity: f64 },
}

/// One surviving line number whose printed caption changed materially.
#[derive(Debug, Clone, PartialEq)]
pub struct Collision {
    pub old: Vec<String>,
    pub new: Vec<String>,
    /// Where the old text went, if it is still on the form under another number. `None` means the old
    /// caption has no counterpart at all — the meaning was retired, not moved.
    pub travelled: Option<Travel>,
}

/// **Axis C — FR-192.** For every line number both revisions print, whether it now describes
/// something else.
pub struct CaptionAxis {
    /// Surviving line numbers with a readable caption on BOTH sides. **Zero is not clean.**
    pub compared: usize,
    /// Of those, the ones whose caption changed materially, worst-first by how little survived.
    pub collisions: BTreeMap<String, Collision>,
    /// Every surviving line number that yielded no comparison, with the reason.
    pub gaps: BTreeMap<String, CaptionGap>,
}

/// How alike two captions are: the **Jaccard index of their token sets**.
///
/// ★ Used only to decide whether a changed line's old text *reappeared* elsewhere, never to decide
/// whether a caption changed — that question is exact (see `normalise_caption`). So a bad similarity
/// call downgrades a report from "moved and reworded" to "meaning retired"; it can never hide a
/// collision.
fn similarity(a: &[String], b: &[String]) -> f64 {
    let sa: BTreeSet<&String> = a.iter().collect();
    let sb: BTreeSet<&String> = b.iter().collect();
    if sa.is_empty() && sb.is_empty() {
        return 1.0;
    }
    sa.intersection(&sb).count() as f64 / sa.union(&sb).count() as f64
}

/// The similarity at which an old caption found under a different number counts as *the same line,
/// reworded* rather than a coincidence.
///
/// ★★ Calibrated on the two shapes that must land on opposite sides of it, both measured:
/// FR-187's casualty line (Schedule A `15` → `16`, *"federally declared"* → *"federally or
/// state-declared"*) scores **0.87**; Schedule A's genuinely different `14` → `15` pair
/// (*"Add lines 11 through 13"* vs *"Add lines 13 and 14"*) scores **0.43**.
const REWORDED_AT: f64 = 0.60;

/// Compare two caption sets. Pure — no filesystem.
///
/// ★★★ **A line that MOVED is still compared at its own number.** FR-187 is the case that makes this
/// non-negotiable: the casualty line moved 15 → 16 *and* widened who qualifies, so a reader that
/// skipped a line because it recognised a renumber would report the move and miss the eligibility
/// change. Both facts are reported: the collision at each number, and the travel of the old text.
pub fn caption_axis(
    survived: &BTreeSet<String>,
    old: &crate::label_reader::CaptionSet,
    new: &crate::label_reader::CaptionSet,
) -> CaptionAxis {
    let mut axis = CaptionAxis {
        compared: 0,
        collisions: BTreeMap::new(),
        gaps: BTreeMap::new(),
    };
    for l in survived {
        let (ao, an) = (old.ambiguous.contains(l), new.ambiguous.contains(l));
        if ao || an {
            axis.gaps.insert(
                l.clone(),
                if ao {
                    CaptionGap::AmbiguousOld
                } else {
                    CaptionGap::AmbiguousNew
                },
            );
            continue;
        }
        let (o, n) = match (old.by_label.get(l), new.by_label.get(l)) {
            (Some(o), Some(n)) => (o, n),
            // `survived` is the intersection of the two label sets, so a label missing from a
            // caption set can only be a reader that disagreed with itself. Named, never dropped.
            _ => {
                axis.gaps.insert(l.clone(), CaptionGap::EmptyBothSides);
                continue;
            }
        };
        match (o.is_empty(), n.is_empty()) {
            (true, true) => {
                axis.gaps.insert(l.clone(), CaptionGap::EmptyBothSides);
            }
            (true, false) => {
                axis.gaps.insert(l.clone(), CaptionGap::EmptyOld);
            }
            (false, true) => {
                axis.gaps.insert(l.clone(), CaptionGap::EmptyNew);
            }
            (false, false) => {
                axis.compared += 1;
                if o != n {
                    axis.collisions.insert(
                        l.clone(),
                        Collision {
                            old: o.clone(),
                            new: n.clone(),
                            travelled: travel_of(l, o, new),
                        },
                    );
                }
            }
        }
    }
    axis
}

/// Where caption `o`, printed at `from` on the old revision, now prints on the new one.
fn travel_of(from: &str, o: &[String], new: &crate::label_reader::CaptionSet) -> Option<Travel> {
    let candidates = || {
        new.by_label
            .iter()
            .filter(|(m, c)| m.as_str() != from && !new.ambiguous.contains(*m) && !c.is_empty())
    };
    if let Some((m, _)) = candidates().find(|(_, c)| c.as_slice() == o) {
        return Some(Travel::Verbatim { to: m.clone() });
    }
    candidates()
        .map(|(m, c)| (m, similarity(o, c)))
        .filter(|(_, s)| *s >= REWORDED_AT)
        .max_by(|(_, a), (_, b)| a.total_cmp(b))
        .map(|(m, s)| Travel::Reworded {
            to: m.clone(),
            similarity: s,
        })
}

/// The delta, as data. `None` for a label map means that side has no geometry fixture — reported,
/// never silently treated as "no change".
pub struct Delta {
    pub added: BTreeSet<String>,
    pub removed: BTreeSet<String>,
    pub common: BTreeSet<String>,
    /// Axis A: old FQN → new FQN for every paired box whose spelling changed. Each is a map edit.
    pub renamed: BTreeMap<String, String>,
    /// Axis A: boxes left unpaired because a suffix was ambiguous on one side, not because they went.
    pub pair_ambiguous: BTreeSet<(&'static str, String)>,
    /// Axis B: the printed line-number set difference. `None` if either side's label set is unreadable.
    pub line_set: Option<LineSetAxis>,
    /// Axis C: surviving line numbers whose printed caption changed materially.
    pub caption_compared: usize,
    pub caption_collisions: BTreeMap<String, Collision>,
    pub caption_gaps: BTreeMap<String, CaptionGap>,
    /// Why axis B/C could not run at all, if they could not.
    pub captions_unreadable: Option<String>,
    /// Fields present in BOTH whose printed line label moved: name -> (old label, new label).
    pub label_moved: BTreeMap<String, (String, String)>,
    pub labels_available: bool,
    /// How many common fields yielded a printed label on BOTH sides — the evidence
    /// [`Delta::label_moved`] being empty is or is not entitled to rest on.
    pub label_compared: usize,
    /// Every common field that yielded none, by name, with the reason it could not be read.
    pub label_unwitnessed: BTreeMap<String, Unwitnessed>,
}

impl Delta {
    /// The label axis's verdict, with its evidence count welded to it.
    pub fn label_verdict(&self) -> LabelVerdict {
        let unwitnessed = self.label_unwitnessed.len();
        if !self.labels_available {
            return LabelVerdict::NoFixture { unwitnessed };
        }
        if self.label_compared == 0 {
            return LabelVerdict::Unwitnessed { unwitnessed };
        }
        if self.label_moved.is_empty() {
            LabelVerdict::Unchanged {
                compared: self.label_compared,
                unwitnessed,
            }
        } else {
            LabelVerdict::Moved {
                compared: self.label_compared,
                moved: self.label_moved.len(),
                unwitnessed,
            }
        }
    }
}

pub fn compute(old: &str, new: &str) -> Result<Delta, String> {
    let a = field_set(old)?;
    let b = field_set(new)?;
    // ★ FR-190: the two sides are PAIRED, not intersected. `common` is the pair count.
    let p = pair_fields(&a, &b);
    let common: BTreeSet<String> = p.pairs.keys().cloned().collect();

    let (la, lb) = (
        crate::label_reader::label_join_public(old).ok(),
        crate::label_reader::label_join_public(new).ok(),
    );
    let labels_available = la.is_some() && lb.is_some();
    let axis = match (la, lb) {
        (Some(la), Some(lb)) => label_axis(&p.pairs, &la, &lb),
        // The axis did not run. Every common field is still ACCOUNTED FOR — as a named gap, so the
        // `compared + unwitnessed == common` invariant holds here too and nothing is dropped.
        _ => LabelAxis {
            compared: 0,
            moved: BTreeMap::new(),
            unwitnessed: common
                .iter()
                .map(|f| (f.clone(), Unwitnessed::GeometryFixtureMissing))
                .collect(),
        },
    };

    // Axes B and C read the printed page, not the AcroForm, so they stand or fall together on
    // whether each side yields a label column at all.
    let (line_set, caption_compared, caption_collisions, caption_gaps, captions_unreadable) = match (
        crate::label_reader::caption_join_public(old),
        crate::label_reader::caption_join_public(new),
    ) {
        (Ok(co), Ok(cn)) => {
            let ls = line_set_axis(
                &co.by_label.keys().cloned().collect(),
                &cn.by_label.keys().cloned().collect(),
            );
            let c = caption_axis(&ls.survived, &co, &cn);
            (Some(ls), c.compared, c.collisions, c.gaps, None)
        }
        (Err(e), _) | (_, Err(e)) => (None, 0, BTreeMap::new(), BTreeMap::new(), Some(e)),
    };

    Ok(Delta {
        added: p.added,
        removed: p.removed,
        renamed: p.renamed,
        pair_ambiguous: p.ambiguous,
        common,
        label_moved: axis.moved,
        labels_available,
        label_compared: axis.compared,
        label_unwitnessed: axis.unwitnessed,
        line_set,
        caption_compared,
        caption_collisions,
        caption_gaps,
        captions_unreadable,
    })
}

/// Print a capped list without hiding the cap — a truncated list that does not say it was truncated
/// is the same defect as a verdict that does not say it was unwitnessed.
fn print_capped(items: impl ExactSizeIterator<Item = String>, cap: usize, prefix: &str) {
    let n = items.len();
    for s in items.take(cap) {
        println!("    {prefix}{s}");
    }
    if n > cap {
        println!("    {prefix}… and {} more (not shown)", n - cap);
    }
}

fn print_unwitnessed(d: &Delta) {
    if d.label_unwitnessed.is_empty() {
        return;
    }
    let mut by_reason: BTreeMap<Unwitnessed, Vec<&str>> = BTreeMap::new();
    for (f, u) in &d.label_unwitnessed {
        by_reason.entry(*u).or_default().push(f.as_str());
    }
    println!(
        "  ★ {} of {} common field(s) yielded NO comparable label. The verdict above says nothing \
         about these, and they are listed rather than counted because a skipped field is not a \
         passed one:",
        d.label_unwitnessed.len(),
        d.common.len()
    );
    for (u, fs) in &by_reason {
        println!("    {} — {}:", fs.len(), u.why());
        // `GeometryFixtureMissing` is the one reason that is not about the field: it is identical for
        // every common field and the actionable item is the missing fixture, already named in the
        // banner. Every other reason is per-field and every field is printed.
        if *u == Unwitnessed::GeometryFixtureMissing {
            continue;
        }
        for f in fs {
            println!("      {f}");
        }
    }
}

pub fn run(old: &str, new: &str) -> Result<(), String> {
    let d = compute(old, new)?;
    println!("form-delta {old} -> {new}");
    println!(
        "  fields: {} paired, {} added, {} removed",
        d.common.len(),
        d.added.len(),
        d.removed.len()
    );
    print_capped(d.added.iter().cloned(), 8, "+ ");
    print_capped(d.removed.iter().cloned(), 8, "- ");
    if !d.renamed.is_empty() {
        println!(
            "  ★ {} paired field(s) were RESPELLED — the box is the same box, so it is compared, but \
             the bundled map stores the FULL FQN and every one of these is a map edit:",
            d.renamed.len()
        );
        print_capped(
            d.renamed
                .iter()
                .map(|(o, n)| format!("{o}\n        -> {n}"))
                .collect::<Vec<_>>()
                .into_iter(),
            8,
            "",
        );
    }
    if !d.pair_ambiguous.is_empty() {
        println!(
            "  ★ {} field(s) could NOT be paired because the same leaf name appears more than once \
             on one side (a form with several copies of itself does this). They are NOT reported as \
             added or removed, because \"two copies share this leaf\" is a different fact from \
             \"this box was retired\":",
            d.pair_ambiguous.len()
        );
        print_capped(
            d.pair_ambiguous
                .iter()
                .map(|(side, f)| format!("{side}: {f}"))
                .collect::<Vec<_>>()
                .into_iter(),
            8,
            "",
        );
    }

    let v = d.label_verdict();
    match v {
        LabelVerdict::NoFixture { .. } => println!(
            "  ★ LINE->LABEL DRIFT NOT CHECKED — one side's geometry fixture yielded no printed \
             label column, so the axis never ran for any field. (A fixture that is MISSING is a hard \
             error before this point, naming the file and `xtask extract-geometry <stem>`.) This is \
             the axis a name check cannot see, so an unchecked run is NOT a clean one."
        ),
        LabelVerdict::Unwitnessed { .. } => println!(
            "  ★★ LINE->LABEL DRIFT UNWITNESSED — 0 of {} common field(s) yielded a printed label \
             on BOTH sides. Nothing was compared, so \"no change\" here would be the absence of \
             evidence, not evidence of absence.",
            d.common.len()
        ),
        LabelVerdict::Unchanged { compared, .. } => println!(
            "  line->label: {compared} of {} common field(s) COMPARED; none of them changed the \
             printed line it sits beside",
            d.common.len()
        ),
        LabelVerdict::Moved {
            compared, moved, ..
        } => {
            println!(
                "  ★★ {moved} of {compared} COMPARED field(s) ({} common) KEPT THEIR NAME but now \
                 sit beside a different printed line — a map carried forward unchanged would write \
                 to the wrong line of a signed return:",
                d.common.len()
            );
            print_capped(
                d.label_moved
                    .iter()
                    .map(|(f, (x, y))| format!("{f}: line {x} -> {y}"))
                    .collect::<Vec<_>>()
                    .into_iter(),
                20,
                "",
            );
        }
    }
    print_unwitnessed(&d);
    print_line_set(&d);
    print_captions(&d);

    // ★★ The exit status is DERIVED from the verdicts rather than decided arm by arm, so no
    // evidence-free verdict can exit 0 by someone forgetting a `return` in a later branch.
    let mut blind: Vec<String> = Vec::new();
    if !v.is_witnessed() {
        blind.push(format!(
            "the line->label axis compared {} of {} paired field(s) ({} unwitnessed)",
            v.compared(),
            d.common.len(),
            v.unwitnessed()
        ));
    }
    match &d.captions_unreadable {
        Some(e) => blind.push(format!("the line-set and caption axes never ran ({e})")),
        None if d.caption_compared == 0 => blind.push(format!(
            "the caption axis compared 0 of {} surviving line number(s)",
            d.line_set.as_ref().map_or(0, |l| l.survived.len())
        )),
        None => {}
    }
    if blind.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{old} -> {new}: {} — this run is NOT evidence that nothing changed",
            blind.join("; ")
        ))
    }
}

fn print_line_set(d: &Delta) {
    let Some(ls) = &d.line_set else {
        println!(
            "  ★★ LINE SET NOT CHECKED — {}",
            d.captions_unreadable
                .as_deref()
                .unwrap_or("no reason recorded")
        );
        return;
    };
    if ls.retired.is_empty() && ls.introduced.is_empty() {
        println!(
            "  line set: {} printed line number(s), none retired, none introduced",
            ls.survived.len()
        );
        return;
    }
    println!(
        "  ★★ the PRINTED LINE SET changed: {} retired, {} introduced, {} survived. A retired line \
         number that code still reads reads a BLANK, and a blank and a zero are the same thing on \
         the page:",
        ls.retired.len(),
        ls.introduced.len(),
        ls.survived.len()
    );
    if !ls.retired.is_empty() {
        println!("    retired:    {}", join_labels(&ls.retired));
    }
    if !ls.introduced.is_empty() {
        println!("    introduced: {}", join_labels(&ls.introduced));
    }
}

fn join_labels(s: &BTreeSet<String>) -> String {
    let mut v: Vec<&str> = s.iter().map(String::as_str).collect();
    v.sort_by_key(|l| label_order(l));
    v.join(" ")
}

/// `("9", "9")` before `("10", "10")` — a form's own order, not ASCII's.
fn label_order(l: &str) -> (u32, String) {
    let digits: String = l.chars().take_while(char::is_ascii_digit).collect();
    (digits.parse().unwrap_or(0), l.to_string())
}

fn print_captions(d: &Delta) {
    if d.captions_unreadable.is_some() {
        return; // already reported by print_line_set
    }
    let survived = d.line_set.as_ref().map_or(0, |l| l.survived.len());
    if d.caption_collisions.is_empty() {
        println!(
            "  line meaning: {} of {survived} surviving line number(s) COMPARED by printed caption; \
             none of them describes something else",
            d.caption_compared
        );
    } else {
        println!(
            "  ★★★ {} of {} COMPARED surviving line number(s) now print a MATERIALLY DIFFERENT \
             caption — a line number that survived while its MEANING changed is invisible to a name \
             diff and to a label diff, and taxpayer-adverse in whichever direction the substituted \
             quantity runs:",
            d.caption_collisions.len(),
            d.caption_compared
        );
        let mut ks: Vec<&String> = d.caption_collisions.keys().collect();
        ks.sort_by_key(|l| label_order(l));
        for l in ks {
            let c = &d.caption_collisions[l];
            println!("    line {l}:");
            println!("        WAS: {}", clip(&c.old));
            println!("        NOW: {}", clip(&c.new));
            match &c.travelled {
                Some(Travel::Verbatim { to }) => println!(
                    "        ↳ line {l}'s old text now prints at line {to} (verbatim) — a pure \
                     renumber, and every cross-reference to {l} must become {to}"
                ),
                Some(Travel::Reworded { to, similarity }) => println!(
                    "        ↳ line {l}'s old text now prints at line {to} AND WAS REWORDED \
                     (similarity {similarity:.2}) — the number moved and the RULE changed; a move \
                     table recording \"{l} -> {to}, same quantity\" is right about the number and \
                     wrong about the rule"
                ),
                None => println!(
                    "        ↳ line {l}'s old text prints nowhere on the new revision — this \
                     meaning was RETIRED, not moved"
                ),
            }
        }
    }
    if d.caption_gaps.is_empty() {
        return;
    }
    let mut by_reason: BTreeMap<CaptionGap, Vec<&str>> = BTreeMap::new();
    for (l, g) in &d.caption_gaps {
        by_reason.entry(*g).or_default().push(l.as_str());
    }
    println!(
        "  ★ {} of {survived} surviving line number(s) yielded NO caption comparison. The verdict \
         above says nothing about these:",
        d.caption_gaps.len()
    );
    for (g, ls) in &by_reason {
        println!("    {} — {}: {}", ls.len(), g.why(), ls.join(" "));
    }
}

/// A caption, printed at a length a person can read, saying so when it is cut.
fn clip(toks: &[String]) -> String {
    let s = toks.join(" ");
    if s.chars().count() <= 160 {
        return s;
    }
    let head: String = s.chars().take(160).collect();
    format!("{head}… ({} tokens total)", toks.len())
}

/// Every `irs_stem` any bundled map declares — the EMITTING surface, the denominator the work list
/// must enumerate from (FR-50; the doc's own "wrong denominator" warning). Never the fixture pairs.
pub fn emitting_surface() -> std::collections::BTreeSet<String> {
    let mut stems = std::collections::BTreeSet::new();
    let forms = crate::form_geometry::repo_root().join("crates/btctax-forms/forms");
    for year in std::fs::read_dir(&forms).into_iter().flatten().flatten() {
        let Ok(maps) = std::fs::read_dir(year.path()) else {
            continue;
        };
        for m in maps.flatten() {
            let p = m.path();
            if !p.to_string_lossy().ends_with(".map.toml") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&p) else {
                continue;
            };
            if let Some(v) = text.lines().find_map(|l| l.trim().strip_prefix("irs_stem")) {
                stems.insert(
                    v.trim()
                        .trim_start_matches('=')
                        .trim()
                        .trim_matches('"')
                        .to_string(),
                );
            }
        }
    }
    stems
}

/// `xtask port-status <prior-tag> <new-tag>` — the work list's two tables, printed from the emitting
/// surface (FR-50): one row per stem, NEVER an omitted row. A pair that computes prints its counts,
/// with the moved cell a number only when labels were compared (else `**UNWITNESSED**`); a pair that
/// does not prints which side is missing (`NO PRIOR SIDE` / `NO DRAFT`), read off the archive.
/// `design/TY2026_WORK_LIST.md` is regenerated by pasting this output; the test
/// `the_committed_work_list_matches_form_delta_at_head` holds the two together.
pub fn port_status(prior_tag: &str, new_tag: &str) -> Result<String, String> {
    port_status_over(&emitting_surface(), prior_tag, new_tag)
}

/// The printer over an explicit stem set — so a test can hand it a SHRUNKEN surface and watch the
/// surface-equality check red on genuine printer output (port-status review #8).
pub fn port_status_over(
    surface: &std::collections::BTreeSet<String>,
    prior_tag: &str,
    new_tag: &str,
) -> Result<String, String> {
    // the missing new side is a DRAFT while the tag says so and a FINAL once it does not (FR-50's
    // third state; port-status review #1)
    let no_new = if new_tag.contains("DRAFT") {
        "**NO DRAFT**"
    } else {
        "**NO FINAL**"
    };
    let mut numeric = Vec::new();
    let mut excused = Vec::new();
    for form in surface {
        let (a, b) = (format!("{form}--{prior_tag}"), format!("{form}--{new_tag}"));
        match compute(&a, &b) {
            Ok(d) => {
                let moved = if d.label_compared == 0 && !d.common.is_empty() {
                    "**UNWITNESSED** — a side's label set could not be read".to_string()
                } else {
                    d.label_moved.len().to_string()
                };
                // ★★ FR-191 / FR-192 — the two new cells. The work list is what a person reads in
                //    January, so an axis that only `form-delta` prints is an axis nobody consults.
                //    Both print **UNWITNESSED** rather than 0 when the reader could not run, for the
                //    same reason the moved cell does: a clean number from no comparisons is the trap
                //    this document was corrected for.
                //    ★ and each cell carries ITS OWN witness: Form 8949 prints two line numbers, so
                //    its line SET reads perfectly while its caption axis has nothing to compare.
                let retired = match &d.line_set {
                    Some(ls) => ls.retired.len().to_string(),
                    None => "**UNWITNESSED**".to_string(),
                };
                let captions = if d.caption_compared == 0 {
                    "**UNWITNESSED**".to_string()
                } else {
                    d.caption_collisions.len().to_string()
                };
                let shape = if d.added.len() + d.removed.len() > d.common.len() {
                    "**REBUILT**"
                } else if d.added.is_empty()
                    && d.removed.is_empty()
                    && d.label_compared > 0
                    && d.label_moved.is_empty()
                    && d.caption_compared > 0
                    && d.caption_collisions.is_empty()
                    && d.line_set
                        .as_ref()
                        .is_some_and(|l| l.retired.is_empty() && l.introduced.is_empty())
                {
                    "unchanged"
                } else {
                    "port"
                };
                numeric.push(format!(
                    "| `{form}` | {} | {} | {} | {moved} | {retired} | {captions} | {shape} |",
                    d.common.len(),
                    d.added.len(),
                    d.removed.len()
                ));
            }
            Err(_) => {
                let (has_prior, has_new) = (pdf_for(&a).is_some(), pdf_for(&b).is_some());
                let prior = if has_prior {
                    format!("`{a}`")
                } else {
                    "**NO PRIOR SIDE**".to_string()
                };
                let new = if has_new {
                    format!("`{b}`")
                } else {
                    no_new.to_string()
                };
                // the cell names EVERY missing side (review #5)
                let cell = match (has_prior, has_new) {
                    (false, false) => format!("**NO PRIOR SIDE** + {no_new}"),
                    (false, true) => "**NO PRIOR SIDE**".to_string(),
                    _ => no_new.to_string(),
                };
                excused.push(format!("| `{form}` | yes | {prior} | {new} | {cell} |"));
            }
        }
    }
    let mut out = String::new();
    out.push_str(
        "| form | common | added | removed | lines that moved | lines retired | line numbers whose \
         MEANING changed | shape |\n|---|---|---|---|---|---|---|---|\n",
    );
    for r in &numeric {
        out.push_str(r);
        out.push('\n');
    }
    out.push_str(
        "\n| form | emitted? | prior side | TY2026 side | cell |\n|---|---|---|---|---|\n",
    );
    for r in &excused {
        out.push_str(r);
        out.push('\n');
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    /// ★ B1 for the label-join fold review L3: `design/TY2026_WORK_LIST.md` is this tool's OUTPUT,
    /// and it decayed silently when the reader behind the "lines that moved" column changed (four
    /// cells, including the only claim that Form 6251 moves a line). Every numeric row of the
    /// committed table is recomputed here at HEAD; every excused row ("Not listed") must be excused
    /// for the reason its own cell prints (fold review r2 N6: "no pair" alone would accept an
    /// accidentally deleted fixture); and every stem with a map in any bundled year must appear in
    /// one table or the other — the doc's own "wrong denominator" warning, made a check.
    #[test]
    fn the_committed_work_list_matches_form_delta_at_head() {
        let root = crate::form_geometry::repo_root();
        let doc = std::fs::read_to_string(root.join("design/TY2026_WORK_LIST.md")).unwrap();
        assert_eq!(
            work_list_tags(&doc),
            Some(("2025".to_string(), "2026-DRAFT".to_string())),
            "the committed work list declares the tags it was generated with"
        );
        let (compared, excused, wrong) = check_work_list(&doc);
        let stems = super::emitting_surface();
        let listed: std::collections::BTreeSet<String> =
            compared.iter().chain(&excused).cloned().collect();
        let missing: Vec<&String> = stems.iter().filter(|s| !listed.contains(*s)).collect();
        assert!(
            stems.len() >= 18,
            "expected every emitted stem's irs_stem across the bundled maps, found {stems:?}"
        );
        assert!(
            missing.is_empty(),
            "stems with a map in a bundled year but NO row in either work-list table: {missing:?}"
        );
        eprintln!(
            "work list: {} compared, {} excused, {} stems on the emitting surface",
            compared.len(),
            excused.len(),
            stems.len()
        );
        assert!(
            wrong.is_empty(),
            "work list rows that no longer match the tool:\n  {}",
            wrong.join("\n  ")
        );
    }

    /// ★ FR-50 — the printer enumerates from the emitting surface and prints the SAME rows the
    /// committed document carries: every numeric row cell-for-cell, every excused form present, and
    /// no stem missing from either. Regenerate the doc by pasting the printer; this reds if they drift.
    #[test]
    fn port_status_prints_the_committed_work_list() {
        type Cells = Option<NumericRow>;
        let printed = super::port_status("2025", "2026-DRAFT").unwrap();
        let doc = std::fs::read_to_string(
            crate::form_geometry::repo_root().join("design/TY2026_WORK_LIST.md"),
        )
        .unwrap();
        // a row's held content: its numeric cells AND its shape (numeric rows), or the two claims its
        // prior-side / new-side cells make (excused rows) — the prose around a claim is the doc's own
        let rows = |text: &str| -> std::collections::BTreeMap<String, (Cells, String)> {
            text.lines()
                .filter_map(parse_work_list_row_full)
                .map(|(form, cells, prior, new, last)| {
                    let held = if cells.is_some() {
                        last
                    } else {
                        let claim = |c: &str| {
                            if c.contains("NO PRIOR SIDE") {
                                "NO PRIOR SIDE"
                            } else if c.contains("NO DRAFT") {
                                "NO DRAFT"
                            } else if c.contains("NO FINAL") {
                                "NO FINAL"
                            } else {
                                "present"
                            }
                        };
                        format!("{} / {}", claim(&prior), claim(&new))
                    };
                    (form, (cells, held))
                })
                .collect()
        };
        let (p, d) = (rows(&printed), rows(&doc));
        let surface = super::emitting_surface();
        assert_eq!(
            p.keys().cloned().collect::<std::collections::BTreeSet<_>>(),
            surface,
            "the printer must print one row per stem on the emitting surface"
        );
        for (form, held) in &p {
            assert_eq!(
                d.get(form),
                Some(held),
                "{form}: printer says {held:?}, the committed work list says {:?}",
                d.get(form)
            );
        }
        // ★ the plant runs the PRINTER on a shrunken surface: genuine tool output missing a stem
        //   fails the surface-equality check above (port-status review #8)
        let mut shrunk = surface.clone();
        assert!(shrunk.remove("f6251"));
        let partial = rows(&super::port_status_over(&shrunk, "2025", "2026-DRAFT").unwrap());
        let partial_keys: std::collections::BTreeSet<_> = partial.keys().cloned().collect();
        assert_ne!(
            partial_keys, surface,
            "a printer that dropped a stem must fail the surface equality"
        );
        assert_eq!(
            partial_keys, shrunk,
            "…and it printed exactly the surface it was given (port-status r2 N2)"
        );
        // and a FINAL tag names the third state
        let final_tag = super::port_status_over(&shrunk, "2025", "2026").unwrap();
        assert!(final_tag.contains("**NO FINAL**") && !final_tag.contains("NO DRAFT"));
    }

    /// The plants (fold review r2 N6): the SAME checker over a synthetic document must red on a
    /// wrong cell, on a numeric row with no pair, and on an excuse whose cell names the wrong side.
    #[test]
    fn the_work_list_checker_reds_on_every_planted_row() {
        // the older plants are bare rows: check them under the committed document's own tags
        let dflt = |doc: &str| check_work_list_with(doc, "2025", "2026-DRAFT");
        // ★★★ 2026-09-13 — EVERY numeric plant below is asserted to PARSE as a numeric row first.
        //    Adding the two new axis columns silently demoted all eight of these to *excused* rows
        //    (the shape cell landed where an axis cell was expected, so `numeric` came out `None`),
        //    and the test stayed GREEN — each plant redding on "excused as having no pair, but
        //    form-delta computes one" instead of on the cell it was planting. A plant that reds for
        //    the wrong reason is an instrument nobody is watching, so the shape is now checked.
        let numeric_plant = |row: &str| {
            let parsed = parse_work_list_row_full(row)
                .unwrap_or_else(|| panic!("plant does not parse as a table row at all: {row:?}"));
            assert!(
                parsed.1.is_some(),
                "plant must parse as a NUMERIC row, or it is testing the excused arm by accident: \
                 {row:?}"
            );
            dflt(row)
        };
        let (_, _, wrong) = numeric_plant("| `f6251` | 62 | 0 | 0 | 1 | 1 | 8 | port |\n");
        assert_eq!(wrong.len(), 1, "the MOVED cell off by one: {wrong:?}");
        // ★★ FR-165 — the common/added/removed comparison had NO plant, and the assertion above
        //    calls itself "a cell off by one" while planting the MOVED cell, so nothing had ever
        //    watched the counts branch red. Measured 2026-09-13: neutralising `counts != (common,
        //    added, removed)` to `if false` left this whole test GREEN. It matters now because
        //    FR-165 replaced the SOURCE of those three numbers — they come from the committed
        //    geometry fixture's box names rather than from the PDF — so the one comparison that
        //    would notice the new source disagreeing with the document was the unwatched one.
        let (_, _, wrong) = numeric_plant("| `f6251` | 61 | 0 | 0 | 0 | 1 | 8 | port |\n");
        assert_eq!(wrong.len(), 1, "the COMMON cell off by one: {wrong:?}");
        let (_, _, wrong) = numeric_plant("| `f6251` | 62 | 1 | 0 | 0 | 1 | 8 | port |\n");
        assert_eq!(wrong.len(), 1, "the ADDED cell off by one: {wrong:?}");
        let (_, _, wrong) = numeric_plant("| `f6251` | 62 | 0 | 1 | 0 | 1 | 8 | port |\n");
        assert_eq!(wrong.len(), 1, "the REMOVED cell off by one: {wrong:?}");
        // ★★★ FR-191 / FR-192 — the two NEW axis cells, each planted in both directions. Without
        //     these the columns would print numbers nobody compares, which is the whole shape the
        //     work-list checker exists to prevent.
        let (_, _, wrong) = numeric_plant("| `f6251` | 62 | 0 | 0 | 0 | 2 | 8 | port |\n");
        assert_eq!(wrong.len(), 1, "the RETIRED cell off by one: {wrong:?}");
        let (_, _, wrong) = numeric_plant("| `f6251` | 62 | 0 | 0 | 0 | 1 | 9 | port |\n");
        assert_eq!(
            wrong.len(),
            1,
            "the MEANING-CHANGED cell off by one: {wrong:?}"
        );
        let (_, _, wrong) =
            numeric_plant("| `f6251` | 62 | 0 | 0 | 0 | **UNWITNESSED** | 8 | port |\n");
        assert_eq!(
            wrong.len(),
            1,
            "UNWITNESSED printed for a line SET the tool read: {wrong:?}"
        );
        let (_, _, wrong) =
            numeric_plant("| `f6251` | 62 | 0 | 0 | 0 | 1 | **UNWITNESSED** | port |\n");
        assert_eq!(
            wrong.len(),
            1,
            "UNWITNESSED printed for a caption axis that compared: {wrong:?}"
        );
        // ★ and the opposite direction on the ONE stem whose caption axis genuinely cannot witness:
        //   Form 8949 prints two line numbers and both are ambiguous, so its line SET reads fine
        //   (0 retired) while its caption axis has nothing to compare. A 0 there is the trap.
        let (_, _, wrong) = numeric_plant("| `f8949` | 202 | 0 | 0 | 0 | 0 | 0 | port |\n");
        assert_eq!(
            wrong.len(),
            1,
            "0 meaning-changed printed for an axis that compared nothing: {wrong:?}"
        );
        let (_, excused, wrong) =
            numeric_plant("| `f8949` | 202 | 0 | 0 | 0 | 0 | **UNWITNESSED** | port |\n");
        assert!(
            wrong.is_empty() && excused.is_empty(),
            "…and the TRUE row for that stem, with its line set still a number: {wrong:?}"
        );
        let (_, _, wrong) = numeric_plant("| `f1040` | 199 | 0 | 0 | 0 | 0 | 0 | unchanged |\n");
        assert_eq!(
            wrong.len(),
            1,
            "a numeric row whose pair does not exist: {wrong:?}"
        );
        let (_, _, wrong) = numeric_plant("| `f1040s1` | 72 | 1 | 1 | 0 | 0 | 0 | port |\n");
        assert!(
            !wrong.is_empty(),
            "0 moved printed for a pair the reader cannot witness: {wrong:?}"
        );
        let (_, _, wrong) =
            numeric_plant("| `f6251` | 62 | 0 | 0 | **UNWITNESSED** | 1 | 8 | port |\n");
        assert_eq!(
            wrong.len(),
            1,
            "UNWITNESSED printed for a pair that WAS compared: {wrong:?}"
        );
        let (_, _, wrong) =
            dflt("| `f6251` | yes | `f6251--2025` | **NO DRAFT** — planted | **NO DRAFT** |\n");
        assert_eq!(
            wrong.len(),
            1,
            "an excused row whose pair EXISTS: {wrong:?}"
        );
        // ★ isolates the `claims_no_new` conjunct (r3 R2): the pair does not compute (no prior
        //   PDF), the prior claim is true, and ONLY the draft-on-disk conjunct can red it.
        //
        // ★★ 2026-09-12 (FR-136) — the stem AND its archive are now synthetic. This plant needs a
        //   stem whose prior side is absent while its DRAFT is present, and it used to borrow that
        //   from a real form: `f8995a` under the prior tag `2017`, because TY2017 was dropped whole
        //   (S9) and no TY2026 final was archived yet. Measured 2026-09-12: dropping a TY2026 final
        //   for `f8995a` into `design/forms/2026/` — which is exactly what January does — turned the
        //   companion plant below from *excused* into
        //   `"excused as prior=… / TY2026=…, but on disk prior=false draft=true"`, and the same copy
        //   for `f6251` turned its plant into `"excused as having no pair, but form-delta computes
        //   one"`. An archive the plant DECLARES cannot be overtaken by a port, and — the case that
        //   has no repair — a COMPLETE archive leaves no real stem to re-point these to at all.
        let drafted = |s: &str| s == "zzz-drafted--2026-DRAFT";
        assert!(
            !real_archive("zzz-drafted--2025") && !real_archive("zzz-drafted--2026-DRAFT"),
            "`zzz-drafted` must be a stem no form has and no archive can hold — the declared archive \
             is the only thing the plants below may read"
        );
        assert!(
            super::compute("zzz-drafted--2025", "zzz-drafted--2026-DRAFT").is_err(),
            "…so the pair cannot compute, which is what puts the row in the excused arm"
        );
        let (_, _, wrong) = check_work_list_against(
            "| `zzz-drafted` | yes | **NO PRIOR SIDE** | **NO DRAFT** — planted | — |\n",
            "2025",
            "2026-DRAFT",
            &drafted,
        );
        assert_eq!(
            wrong.len(),
            1,
            "a NO DRAFT claim with the draft in the archive: {wrong:?}"
        );
        let (_, _, wrong) =
            dflt("| `f1040` | yes | **NO PRIOR SIDE** — planted | no draft either | — |\n");
        assert_eq!(
            wrong.len(),
            1,
            "a NO PRIOR SIDE claim with the 2025 fixture on disk: {wrong:?}"
        );
        let (_, _, wrong) = dflt("| `f1040` | yes | `f1040--2025` | nothing claimed | — |\n");
        assert_eq!(
            wrong.len(),
            1,
            "a row that claims neither side is not an excuse: {wrong:?}"
        );
        // ★ the NO FINAL half of the claim (port-status r2 N1), both directions: a true claim on a
        //   stem with no final on disk is excused; a NO FINAL claim is FALSE once a `--2026` PDF exists
        // (NO FINAL is a claim about a FINAL tag — under the document's draft tag it is the wrong word)
        let (_, excused, wrong) = check_work_list_with(
            "| `zzz-not-a-form` | yes | **NO PRIOR SIDE** | **NO FINAL** — synthetic | — |\n",
            "2025",
            "2026",
        );
        assert!(
            wrong.is_empty() && excused == ["zzz-not-a-form"],
            "a true NO FINAL excuse: {wrong:?}"
        );
        // ★ …and the same distinction on a stem whose archive holds a DRAFT and no final, which is
        //   the pair of verdicts a real form could only supply while its TY2026 final was missing
        //   (FR-136). `drafted` declares exactly one file, so "a draft is not a final" is the only
        //   thing that can decide either row.
        let (_, excused, wrong) = check_work_list_against(
            "| `zzz-drafted` | yes | **NO PRIOR SIDE** | **NO FINAL** — planted | — |\n",
            "2025",
            "2026",
            &drafted,
        );
        assert!(
            wrong.is_empty() && excused == ["zzz-drafted"],
            "NO FINAL is TRUE when only the draft is archived (a draft is not a final): {wrong:?}"
        );
        // and the false direction: a NO DRAFT claim while the draft is in the archive. The only
        // difference from the excused row just above is the new side — DRAFT against final, in both
        // the tag and the word the cell uses for it — so the opposite verdict isolates exactly the
        // draft/final distinction.
        let (_, _, wrong) = check_work_list_against(
            "| `zzz-drafted` | yes | **NO PRIOR SIDE** | **NO DRAFT** — planted | — |\n",
            "2025",
            "2026-DRAFT",
            &drafted,
        );
        assert_eq!(
            wrong.len(),
            1,
            "NO DRAFT is FALSE when the draft IS archived: {wrong:?}"
        );
        // ★ NO FINAL load-bearing (r3 N4): the prior side is PRESENT (no NO PRIOR SIDE claim), the new
        //   side claims NO FINAL under a final tag, and no final is in the archive — only the NO FINAL
        //   recognition can excuse this row; the same row under the DRAFT tag names the wrong word.
        // ★★ FR-136: this plant named `f6251` and asserted `pdf_for("f6251--2026").is_none()`, so
        //   January's finals both falsify the premise and invert the row (measured: "excused as having
        //   no pair, but form-delta computes one"). The declared archive states the prior side and
        //   withholds the final, which is the shape the plant was always describing.
        let with_prior = |s: &str| s == "zzz-with-prior--2025";
        let (_, excused, wrong) = check_work_list_against(
            "| `zzz-with-prior` | yes | `zzz-with-prior--2025` | **NO FINAL** — planted | — |\n",
            "2025",
            "2026",
            &with_prior,
        );
        assert!(
            wrong.is_empty() && excused == ["zzz-with-prior"],
            "NO FINAL alone must excuse this row: {wrong:?}"
        );
        let (_, _, wrong) = check_work_list_against(
            "| `zzz-with-prior` | yes | `zzz-with-prior--2025` | **NO FINAL** — planted | — |\n",
            "2025",
            "2026-DRAFT",
            &with_prior,
        );
        assert_eq!(
            wrong.len(),
            1,
            "NO FINAL under a draft tag names the wrong word: {wrong:?}"
        );
        // and the tags line is read from the document itself
        // ★ the tags line is LOAD-BEARING end to end (r4 N6): the same row is excused under a document
        //   declaring the final tag and wrong under one declaring the draft tag, through
        //   check_work_list's own tag parsing; a document with no tags line is wrong
        let row = "| `zzz-drafted` | yes | **NO PRIOR SIDE** | **NO FINAL** — planted | — |\n";
        let (_, excused, wrong) =
            check_work_list_tagged(&format!("<!-- tags: 2025 2026 -->\n{row}"), &drafted);
        assert!(
            wrong.is_empty() && excused == ["zzz-drafted"],
            "declared final tag: {wrong:?}"
        );
        let (_, _, wrong) =
            check_work_list_tagged(&format!("<!-- tags: 2025 2026-DRAFT -->\n{row}"), &drafted);
        assert_eq!(
            wrong.len(),
            1,
            "declared draft tag: NO FINAL is the wrong word: {wrong:?}"
        );
        let (_, _, wrong) = check_work_list_tagged(row, &drafted);
        assert_eq!(wrong.len(), 1, "no tags line: {wrong:?}");
        assert!(wrong[0].contains("tags"));
        // the control tests the PREDICATE on a stem that can never be archived (r3 R5) — the
        // inventory coupling belongs to the_committed_work_list_matches_form_delta_at_head
        let (_, excused, wrong) =
            dflt("| `zzz-not-a-form` | yes | **NO PRIOR SIDE** | **NO DRAFT** — synthetic | — |\n");
        assert!(
            wrong.is_empty() && excused == ["zzz-not-a-form"],
            "a true excuse: {wrong:?}"
        );
        // ★★ FR-136 — the plants above DECLARE their archive, so the REAL oracle owes its own kill:
        //   a `real_archive` stuck at `false` would make every excused row above vacuous and every
        //   committed-document check silent. Both directions are structural rather than accidental:
        //   every `(stem, year)` the crate BUNDLES has a committed template under
        //   `crates/btctax-forms/forms/<year>/`, which `pdf_for` resolves before the archive; and
        //   `zzz-not-a-form` is a stem no IRS form has and no archive can hold. Derived from
        //   `BUNDLED`, never a typed pair, so a year package that gains or loses a form is covered.
        assert!(
            btctax_forms::bundled::BUNDLED
                .iter()
                .all(|(stem, year)| real_archive(&format!("{}--{year}", stem.file_stem()))),
            "the real archive oracle must see every bundled template"
        );
        assert!(
            !real_archive("zzz-not-a-form--2025"),
            "…and must NOT see a stem no archive can hold — an oracle that answers the same for both \
             makes every excused row above vacuous"
        );
    }

    /// `(compared, excused, wrong)` over every table row of a work-list document.
    /// The tags a work-list document was generated with, declared on its own `<!-- tags: <prior> <new> -->`
    /// line (port-status r3 N5): the checker computes every pair with THESE, so a document regenerated
    /// after the finals (`2025 2026`) validates against the finals, not the drafts.
    fn work_list_tags(doc: &str) -> Option<(String, String)> {
        let l = doc
            .lines()
            .find(|l| l.trim_start().starts_with("<!-- tags:"))?;
        let inner = l
            .trim()
            .trim_start_matches("<!-- tags:")
            .trim_end_matches("-->")
            .trim();
        let mut it = inner.split_whitespace();
        Some((it.next()?.to_string(), it.next()?.to_string()))
    }

    /// The ARCHIVE oracle: is `<stem>.pdf` on disk?
    ///
    /// ★ The claim is about the ARCHIVE (the PDF), so the check reads the PDF (r3 R3), never the
    /// geometry fixture, a different artifact.
    ///
    /// ★★ **FR-165 opened a seam here, and it is stated rather than papered over.** `compute` now
    /// enters the excused arm on a missing GEOMETRY FIXTURE, not on a missing PDF, so the predicate
    /// that puts a row in the excused arm and the predicate that checks the row's claim are no longer
    /// the same one. Measured 2026-09-13 with and without `design/forms/**/*.pdf` present: every stem
    /// on the emitting surface answers identically either way, because every prior side in the
    /// excused table has a COMMITTED bundled template (`crates/btctax-forms/forms/<year>/`) and every
    /// absent side has neither artifact. The one state that would diverge is a stem whose PDF has
    /// been archived but whose fixture has not yet been extracted — a transient, and the fix for it
    /// is `xtask extract-geometry <stem>`, which is what the row would then be telling you to do.
    /// Filed as a follow-up rather than restructured here, because collapsing the two predicates
    /// would retarget the FR-136 plants at the same time as FR-165's fix.
    ///
    /// ★★ FR-136 — this is a PARAMETER of the checker rather than a call inside it, because a plant
    /// that borrows the real archive's accidental gaps is disarmed by the very ports it exists to
    /// watch. The committed-document checks pass this; the plants DECLARE their archive. See
    /// [`the_work_list_checker_reds_on_every_planted_row`].
    fn real_archive(stem: &str) -> bool {
        super::pdf_for(stem).is_some()
    }

    /// A document WITHOUT its tags line is wrong, never checked against a guessed pair (port-status
    /// r4 N6: a silent fallback equal to today's tags meant the parsed tags were never load-bearing).
    fn check_work_list(doc: &str) -> (Vec<String>, Vec<String>, Vec<String>) {
        check_work_list_tagged(doc, &real_archive)
    }

    /// As [`check_work_list`], against a DECLARED archive (FR-136).
    fn check_work_list_tagged(
        doc: &str,
        archived: &dyn Fn(&str) -> bool,
    ) -> (Vec<String>, Vec<String>, Vec<String>) {
        let Some((prior_tag, new_tag)) = work_list_tags(doc) else {
            return (
                Vec::new(),
                Vec::new(),
                vec!["the work list declares no `<!-- tags: <prior> <new> -->` line — nothing can be checked against it".to_string()],
            );
        };
        check_work_list_against(doc, &prior_tag, &new_tag, archived)
    }

    fn check_work_list_with(
        doc: &str,
        prior_tag: &str,
        new_tag: &str,
    ) -> (Vec<String>, Vec<String>, Vec<String>) {
        check_work_list_against(doc, prior_tag, new_tag, &real_archive)
    }

    /// As [`check_work_list_with`], against a DECLARED archive (FR-136).
    fn check_work_list_against(
        doc: &str,
        prior_tag: &str,
        new_tag: &str,
        archived: &dyn Fn(&str) -> bool,
    ) -> (Vec<String>, Vec<String>, Vec<String>) {
        let mut compared = Vec::new();
        let mut excused = Vec::new();
        let mut wrong = Vec::new();
        for l in doc.lines() {
            let Some((form, cells, prior_cell, ty2026_cell)) = parse_work_list_row(l) else {
                continue;
            };
            let pair = super::compute(
                &format!("{form}--{prior_tag}"),
                &format!("{form}--{new_tag}"),
            );
            match (cells, pair) {
                (Some(row), Ok(d)) => {
                    compared.push(form.clone());
                    let counts = (d.common.len(), d.added.len(), d.removed.len());
                    if counts != (row.common, row.added, row.removed) {
                        wrong.push(format!(
                            "{form}: table says {:?}, form-delta at HEAD says {counts:?}",
                            (row.common, row.added, row.removed)
                        ));
                    }
                    // ★ An axis cell is a number ONLY when that axis actually compared something: a
                    //   pair one side of which the reader cannot witness prints UNWITNESSED, never 0
                    //   — the "clean verdict from zero comparisons" trap this document was corrected
                    //   for. All three axis cells are checked the same way, by the same closure, so a
                    //   fourth axis cannot be added with a laxer rule by accident.
                    // ★ Each axis is checked against ITS OWN witness. Keying the line-set cell to the
                    //   caption axis's count was the first cut and this very checker refused it
                    //   (`f8949: table prints 0 retired, but form-delta compared 0`): Form 8949
                    //   prints two line numbers, so the line SET is perfectly readable while the
                    //   caption axis has nothing to compare. Two axes, two witnesses.
                    let mut axis = |name: &str,
                                    cell: Option<usize>,
                                    witnessed: bool,
                                    found: usize| {
                        match cell {
                            Some(n) if !witnessed => wrong.push(format!(
                                "{form}: table prints {n} {name}, but form-delta did not witness that axis — the cell must say UNWITNESSED"
                            )),
                            Some(n) if n != found => wrong.push(format!(
                                "{form}: table says {n} {name}, form-delta at HEAD says {found}"
                            )),
                            None if witnessed => wrong.push(format!(
                                "{form}: table says UNWITNESSED for {name}, but form-delta witnessed that axis ({found} found) — print the number"
                            )),
                            _ => {}
                        }
                    };
                    axis(
                        "moved",
                        row.moved,
                        d.label_compared > 0 || d.common.is_empty(),
                        d.label_moved.len(),
                    );
                    // FR-191: the line SET is witnessed as soon as both sides yield a label column,
                    // whether or not any caption in it could be compared.
                    axis(
                        "retired",
                        row.retired,
                        d.line_set.is_some(),
                        d.line_set.as_ref().map_or(0, |l| l.retired.len()),
                    );
                    // FR-192: the caption axis is witnessed only by an actual caption comparison.
                    axis(
                        "meaning changed",
                        row.captions,
                        d.caption_compared > 0,
                        d.caption_collisions.len(),
                    );
                }
                (Some(_), Err(e)) => {
                    wrong.push(format!("{form}: a numeric row, but no pair at HEAD ({e})"))
                }
                (None, Ok(_)) => wrong.push(format!(
                    "{form}: excused as having no pair, but form-delta computes one"
                )),
                // an excused row must be excused for the reason its own cells print: a prior-side
                // cell saying NO PRIOR SIDE means no `<form>--2025` fixture; a TY2026 cell saying
                // NO DRAFT means no `<form>--2026-DRAFT` fixture. Every claim made is checked, and
                // a row that makes neither is not an excuse.
                (None, Err(_)) => {
                    // the claim names WHICH new side is missing: NO FINAL is about `<form>--2026`,
                    // NO DRAFT about `<form>--2026-DRAFT` (port-status r2 N1 caught the draft being
                    // checked for both)

                    let claims_no_prior = prior_cell.contains("NO PRIOR SIDE");
                    let claims_no_new =
                        ty2026_cell.contains("NO DRAFT") || ty2026_cell.contains("NO FINAL");
                    // the new side is the document's own tag (r3 N5); the claim must NAME it right:
                    // NO DRAFT on a draft tag, NO FINAL on a final tag — a mismatch is a wrong row
                    let tag_is_draft = new_tag.contains("DRAFT");
                    let claim_word_matches_tag = !claims_no_new
                        || (tag_is_draft && ty2026_cell.contains("NO DRAFT"))
                        || (!tag_is_draft && ty2026_cell.contains("NO FINAL"));
                    let draft = archived(&format!("{form}--{new_tag}"));
                    let prior = archived(&format!("{form}--{prior_tag}"));
                    let ok = (claims_no_prior || claims_no_new)
                        && claim_word_matches_tag
                        && (!claims_no_prior || !prior)
                        && (!claims_no_new || !draft);
                    if ok {
                        excused.push(form.clone());
                    } else {
                        wrong.push(format!(
                            "{form}: excused as prior={prior_cell:?} / TY2026={ty2026_cell:?}, but on disk prior={prior} draft={draft}"
                        ));
                    }
                }
            }
        }
        (compared, excused, wrong)
    }

    /// A row of either work-list table: `| \`form\` | common | added | removed | moved | shape |`
    /// → `(form, Some(cells), _, _)`; a "Not listed" row (`| \`form\` | emitted? | prior | TY2026
    /// side | cell |`) → `(form, None, the prior-side cell, the TY2026-side cell)`.
    #[allow(clippy::type_complexity)]
    fn parse_work_list_row(l: &str) -> Option<(String, Option<NumericRow>, String, String)> {
        parse_work_list_row_full(l).map(|(f, c, p, t, _)| (f, c, p, t))
    }

    /// As [`parse_work_list_row`], plus the last cell (the shape column of a numeric row, or the
    /// summary cell of an excused row) so a test can hold it (port-status review #2, #3).
    #[allow(clippy::type_complexity)]
    fn parse_work_list_row_full(
        l: &str,
    ) -> Option<(String, Option<NumericRow>, String, String, String)> {
        let cells: Vec<&str> = l.split('|').map(str::trim).collect();
        if cells.len() < 6 || !cells[1].starts_with('`') {
            return None;
        }
        let form = cells[1].trim_matches('`').to_string();
        let n = |i: usize| cells.get(i).and_then(|c| c.parse::<usize>().ok());
        // an AXIS cell may be a number or the word UNWITNESSED (that axis could not run at all)
        let axis = |i: usize| match n(i) {
            Some(v) => Some(Some(v)),
            None if cells.get(i).is_some_and(|c| c.contains("UNWITNESSED")) => Some(None),
            None => None,
        };
        let numeric = match (n(2), n(3), n(4), axis(5), axis(6), axis(7)) {
            (
                Some(common),
                Some(added),
                Some(removed),
                Some(moved),
                Some(retired),
                Some(captions),
            ) => Some(NumericRow {
                common,
                added,
                removed,
                moved,
                retired,
                captions,
            }),
            _ => None,
        };
        let last = if numeric.is_some() {
            cells.get(8)
        } else {
            cells.get(5)
        };
        Some((
            form,
            numeric,
            cells.get(3).unwrap_or(&"").to_string(),
            cells.get(4).unwrap_or(&"").to_string(),
            last.unwrap_or(&"").to_string(),
        ))
    }

    /// The numeric cells of one work-list row.
    ///
    /// ★★ A struct rather than a growing tuple, so that adding an axis to the table is a **build
    /// error** at every site that reads a row rather than a silently ignored column — FR-114's shape
    /// is exactly an instrument that printed a new cell nobody checked.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct NumericRow {
        common: usize,
        added: usize,
        removed: usize,
        /// `None` = the cell says UNWITNESSED.
        moved: Option<usize>,
        retired: Option<usize>,
        captions: Option<usize>,
    }

    use super::*;

    /// ★★★ **Calibrated on the pair that motivated the whole tool.**
    ///
    /// TY2025's Form 6251 split line 1 into 1a/1b: **one field added, ZERO renamed**, and every
    /// field from 2b down then sat beside a different printed line. A name-existence check passes
    /// on this with 0 of 61 names absent, and TY2024's `line11` — the AMT itself — lands in the
    /// TY2025 form's line-10 box.
    ///
    /// If this ever reports no label movement, `form-delta`'s second axis has gone blind and
    /// draft→final stops being a diff.
    #[test]
    fn form_delta_sees_the_6251_renumber_that_renamed_nothing() {
        let d = compute("f6251--2024", "f6251--2025").expect("both revisions are archived");
        assert_eq!(d.added.len(), 1, "TY2025 added exactly one page-1 field");
        assert_eq!(d.removed.len(), 0, "and renamed NOTHING — that is the trap");
        assert!(
            d.labels_available,
            "both sides need a geometry fixture, or the label axis is silently unchecked"
        );
        assert!(
            d.label_moved.len() >= 12,
            "the added field walks every line below it down; measured 30 of 61 fields moved, got {}",
            d.label_moved.len()
        );
        // The specific cascade, spot-checked rather than merely counted.
        let f = "topmostSubform[0].Page1[0].f1_10[0]";
        assert_eq!(
            d.label_moved.get(f).map(|(a, b)| (a.as_str(), b.as_str())),
            Some(("2g", "2f")),
            "the 2b-onward cascade must be visible field by field, not just as a count"
        );
    }

    /// ★★ **The OPPOSITE shape, pinned because a tool calibrated on one failure mode is calibrated
    /// on none.** TY2025's Form 8995 renamed 18 of 33 fields while the printed form did not change
    /// at all — loud on the name axis, silent on the label axis. Together with the 6251 case these
    /// two bracket the space: rename-without-renumber, and renumber-without-rename.
    #[test]
    fn form_delta_sees_the_8995_rename_that_moved_no_lines() {
        let d = compute("f8995--2024", "f8995--2025").expect("both revisions are archived");
        // ★★ FR-190 changed these three numbers and NOT what the test is about. A full-FQN key saw
        //    18 added / 18 removed / 15 common; the suffix ladder recognises 9 of those 18 as the
        //    same boxes under renamed containers (`Ln1A_Row1[0]` → `Row1i[0]`), so 24 pair and only
        //    9 are genuinely new spellings. The *shape* is unchanged and is what is pinned: loud on
        //    the name axis, silent on every axis that reads the printed page.
        assert_eq!(
            d.common.len(),
            24,
            "24 boxes pair once containers are ignored"
        );
        assert_eq!(d.added.len(), 9, "9 spellings with no counterpart");
        assert_eq!(d.removed.len(), 9, "…and 9 retired ones");
        assert_eq!(
            d.renamed.len(),
            9,
            "9 of the paired boxes were RESPELLED — each one a map edit, and each one invisible \
             before FR-190 because the box fell out of `common` entirely"
        );
        assert!(d.labels_available, "both sides need geometry");
        assert!(
            d.label_moved.is_empty(),
            "the printed form did not change, so no surviving field may have moved a line: {:?}",
            d.label_moved
        );
    }

    /// ★ The TY2026 drafts are archived and geometry-extracted, so the tool is ready for the finals.
    /// This asserts the pipeline is wired end to end TODAY rather than discovering in January that
    /// a fixture is missing.
    #[test]
    fn the_ty2026_drafts_are_ready_to_be_diffed_against_their_finals() {
        let d = compute("f6251--2025", "f6251--2026-DRAFT")
            .expect("the TY2026 draft is archived and its geometry extracted");
        assert!(
            d.labels_available,
            "the TY2026 draft must have a geometry fixture, or the day the final lands the label \
             axis is unchecked and the port is guesswork"
        );
        assert!(
            !d.common.is_empty(),
            "the two revisions must share fields, or one side failed to load"
        );
    }

    // ---------------------------------------------------------------------------------------
    // The evidence axis. These are pure — no fixture, no PDF — so the zero-comparison case can be
    // exercised on demand instead of waiting for a real form to develop one.
    // ---------------------------------------------------------------------------------------

    fn m(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect()
    }

    fn s(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|n| (*n).to_string()).collect()
    }

    /// A `Delta` whose axes B and C are deliberately absent, for the pure label-axis tests below.
    /// The caption axis is exercised on its own inputs by its own tests; mixing the two here would
    /// mean neither was tested alone.
    fn delta_from(
        common: BTreeSet<String>,
        old: &BTreeMap<String, String>,
        new: &BTreeMap<String, String>,
    ) -> Delta {
        let pairs: BTreeMap<String, String> =
            common.iter().map(|f| (f.clone(), f.clone())).collect();
        let axis = label_axis(&pairs, old, new);
        Delta {
            added: BTreeSet::new(),
            removed: BTreeSet::new(),
            renamed: BTreeMap::new(),
            pair_ambiguous: BTreeSet::new(),
            common,
            label_moved: axis.moved,
            labels_available: true,
            label_compared: axis.compared,
            label_unwitnessed: axis.unwitnessed,
            line_set: None,
            caption_compared: 0,
            caption_collisions: BTreeMap::new(),
            caption_gaps: BTreeMap::new(),
            captions_unreadable: Some("not exercised by this test".to_string()),
        }
    }

    /// ★★★ **THE PLANTED DEFECT THIS FILE EXISTS TO KEEP DEAD: a clean verdict from zero
    /// comparisons.**
    ///
    /// Every common field here has a box on both sides and no printed label claims any of them —
    /// `?`, `label_reader`'s BOX-WITH-NO-LABEL. The old code skipped each pair silently, found
    /// `label_moved` empty, and printed *"no field changed the printed line it sits beside"*. That
    /// is the cleanest possible verdict from no evidence at all, and it is what
    /// `form-delta f8959--2025 f8959--2026-DRAFT` was reporting.
    ///
    /// Restore either half of the old behaviour — let `witness` compare two `?`s, or let
    /// `label_verdict` fall through to `Unchanged` when `label_compared == 0` — and this reds.
    #[test]
    fn zero_comparisons_is_unwitnessed_and_never_a_clean_verdict() {
        let common = s(&["f1_1[0]", "f1_2[0]", "f1_3[0]"]);
        let old = m(&[("f1_1[0]", "?"), ("f1_2[0]", "?"), ("f1_3[0]", "?")]);
        let new = m(&[("f1_1[0]", "?"), ("f1_2[0]", "?"), ("f1_3[0]", "?")]);
        let d = delta_from(common, &old, &new);

        assert_eq!(
            d.label_verdict(),
            LabelVerdict::Unwitnessed { unwitnessed: 3 },
            "0 comparisons must be its OWN verdict — an empty `label_moved` is what \"nothing \
             moved\" and \"nothing was looked at\" both look like"
        );
        assert!(
            !d.label_verdict().is_witnessed(),
            "an evidence-free run must not be readable as a checked one"
        );
        assert_eq!(
            d.label_verdict().compared(),
            0,
            "the verdict must carry the count it rests on"
        );
        assert!(
            d.label_moved.is_empty(),
            "and it is still true that nothing was OBSERVED to move — that was never the lie"
        );
    }

    /// ★★ A clean verdict must state how many bindings it rests on, and name what it could not read.
    ///
    /// Two fields are comparable and unmoved; one is BOX-WITH-NO-LABEL on the new side only. The
    /// honest report is *"2 compared, none moved, 1 unwitnessed by name"* — never *"no change"*.
    /// Plant: drop the `Err(u) => unwitnessed.insert(..)` arm in [`label_axis`] and this reds on the
    /// missing name; make `compared` a constant and it reds on the count.
    #[test]
    fn a_clean_verdict_carries_its_evidence_count_and_names_the_gaps() {
        let common = s(&["a", "b", "c"]);
        let old = m(&[("a", "1"), ("b", "2"), ("c", "3")]);
        let new = m(&[("a", "1"), ("b", "2"), ("c", "?")]);
        let d = delta_from(common, &old, &new);

        assert_eq!(
            d.label_verdict(),
            LabelVerdict::Unchanged {
                compared: 2,
                unwitnessed: 1
            },
            "the clean verdict must report 2 comparisons, not 3 and not silence"
        );
        assert_eq!(
            d.label_unwitnessed.get("c"),
            Some(&Unwitnessed::UnlabelledNew),
            "the field that could not be read must be named, with the reason — a count alone \
             cannot be acted on"
        );
        assert!(d.label_moved.is_empty());
    }

    /// ★★ A real move must stay a real move, and must be reported as a fraction of what was
    /// actually compared rather than of what happened to be present.
    #[test]
    fn a_move_among_unwitnessed_fields_is_still_a_move() {
        let common = s(&["a", "b", "c", "d"]);
        let old = m(&[("a", "?"), ("b", "5"), ("c", "6")]);
        let new = m(&[("a", "?"), ("b", "5"), ("c", "7"), ("d", "9")]);
        let d = delta_from(common, &old, &new);

        assert_eq!(
            d.label_verdict(),
            LabelVerdict::Moved {
                compared: 2,
                moved: 1,
                unwitnessed: 2
            },
            "`a` is unlabelled on both sides and `d` has no box on the old side; 2 of 4 were \
             actually compared and 1 of those moved"
        );
        assert_eq!(
            d.label_unwitnessed.get("d"),
            Some(&Unwitnessed::BoxAbsentOld),
            "a field the OLD geometry does not record must be named as such, not skipped"
        );
    }

    /// ★★ **A missing geometry fixture is unwitnessed too, and every common field still has to be
    /// accounted for.** `compute` builds this arm when either side's `label_join` fails, and `run`
    /// derives its non-zero exit from `is_witnessed()` — so the banner that already said *"an
    /// unchecked run is NOT a clean one"* now also EXITS like it means it.
    ///
    /// Reproduced end to end on a real artifact the day this was written: `f8275r--2025` had a PDF
    /// and no geometry, and `form-delta f8275r--2025 f8275r--2025` reported 102 of 102 common fields
    /// unwitnessed and exited 1 (it exited **0** before). That stem is not asserted here — extracting
    /// its geometry is legitimate work that must not red this test.
    ///
    /// ★ **FR-165 moved where that particular input lands, and NOT what this arm guarantees.** A
    /// stem with no fixture is now an `Err` out of `compute` (also non-zero, also naming the repair),
    /// because `field_set` reads the fixture; what still reaches this arm is a fixture whose words
    /// yield no printed label column. The arm is built directly here for exactly that reason — a
    /// verdict this file is responsible for must be testable without waiting for an artifact to
    /// develop the shape that produces it.
    ///
    /// Plant: make `compute`'s fixture-missing arm return an empty `unwitnessed` map, or make
    /// `is_witnessed` true for `NoFixture`, and this reds.
    #[test]
    fn a_missing_geometry_fixture_is_unwitnessed_not_clean() {
        let common = s(&["a", "b"]);
        let d = Delta {
            added: BTreeSet::new(),
            removed: BTreeSet::new(),
            renamed: BTreeMap::new(),
            pair_ambiguous: BTreeSet::new(),
            label_moved: BTreeMap::new(),
            labels_available: false,
            label_compared: 0,
            label_unwitnessed: common
                .iter()
                .map(|f| (f.clone(), Unwitnessed::GeometryFixtureMissing))
                .collect(),
            line_set: None,
            caption_compared: 0,
            caption_collisions: BTreeMap::new(),
            caption_gaps: BTreeMap::new(),
            captions_unreadable: Some("not exercised by this test".to_string()),
            common,
        };
        assert_eq!(
            d.label_verdict(),
            LabelVerdict::NoFixture { unwitnessed: 2 },
            "an axis that never ran must be its own verdict, and must still account for every field"
        );
        assert!(
            !d.label_verdict().is_witnessed(),
            "`run` turns this into a non-zero exit; if it reads as witnessed the tool goes back to \
             exiting 0 on a run that checked nothing"
        );
        assert_eq!(
            d.label_compared + d.label_unwitnessed.len(),
            d.common.len(),
            "the accounting invariant holds on this arm too — it is not exempt for being an error \
             path"
        );
    }

    /// ★★★ **The accounting invariant: every common field is either evidence or a named gap.**
    /// There is no third bucket, which is what makes the compared count meaningful — a field cannot
    /// be dropped by being forgotten.
    ///
    /// Checked on synthetic inputs covering every [`Unwitnessed`] variant *and* on the real
    /// artifacts, because an invariant that only holds on made-up data is a tautology.
    #[test]
    fn every_common_field_is_either_compared_or_named_as_unwitnessed() {
        // Synthetic: one of each outcome.
        let common = s(&[
            "ok",
            "moved",
            "noboxboth",
            "noboxold",
            "noboxnew",
            "qq",
            "q1",
            "q2",
        ]);
        let old = m(&[
            ("ok", "1"),
            ("moved", "2"),
            ("noboxnew", "3"),
            ("qq", "?"),
            ("q1", "?"),
            ("q2", "4"),
        ]);
        let new = m(&[
            ("ok", "1"),
            ("moved", "3"),
            ("noboxold", "5"),
            ("qq", "?"),
            ("q1", "6"),
            ("q2", "?"),
        ]);
        let d = delta_from(common.clone(), &old, &new);
        assert_eq!(
            d.label_compared + d.label_unwitnessed.len(),
            common.len(),
            "compared {} + unwitnessed {} must account for all {} common fields",
            d.label_compared,
            d.label_unwitnessed.len(),
            common.len()
        );
        let reasons: BTreeSet<Unwitnessed> = d.label_unwitnessed.values().copied().collect();
        assert_eq!(
            reasons,
            [
                Unwitnessed::BoxAbsentBothSides,
                Unwitnessed::BoxAbsentOld,
                Unwitnessed::BoxAbsentNew,
                Unwitnessed::UnlabelledBothSides,
                Unwitnessed::UnlabelledOld,
                Unwitnessed::UnlabelledNew,
            ]
            .into_iter()
            .collect::<BTreeSet<_>>(),
            "each way of failing to read a label must be distinguishable in the report"
        );

        // Real: the calibration pairs must obey the same accounting.
        for (a, b) in [
            ("f6251--2024", "f6251--2025"),
            ("f8995--2024", "f8995--2025"),
            ("f6251--2025", "f6251--2026-DRAFT"),
        ] {
            let d = compute(a, b).unwrap_or_else(|e| panic!("{a} -> {b}: {e}"));
            assert_eq!(
                d.label_compared + d.label_unwitnessed.len(),
                d.common.len(),
                "{a} -> {b}: compared {} + unwitnessed {} != {} common",
                d.label_compared,
                d.label_unwitnessed.len(),
                d.common.len()
            );
            assert!(
                d.label_verdict().is_witnessed(),
                "{a} -> {b} is a calibration pair; if its axis stops witnessing anything the \
                 calibration is gone: {:?}",
                d.label_verdict()
            );
        }
    }

    /// ★★ **The real-artifact census, enumerated FROM THE FILESYSTEM** — never a hand-list, which is
    /// the failure mode that produced most of the defects in this repo.
    ///
    /// Every archived `*--2026-DRAFT` geometry fixture is paired with its `--2025` counterpart and
    /// the whole set is walked. What this gates is `form_delta`'s own contract and nothing else:
    ///
    /// * every pair whose axis ran is **self-accounting** (compared + unwitnessed == common), and
    /// * **no pair anywhere yields a clean verdict backed by zero comparisons.**
    ///
    /// Pairs whose geometry fixture is missing are named in the failure text of the vacuity guard
    /// rather than gated here — fixture coverage is `label_reader`'s gate, not this one — but they
    /// can never come back as `Unchanged`, which is the thing this file is responsible for.
    #[test]
    fn no_archived_pair_reports_a_clean_verdict_from_zero_comparisons() {
        let geom = crate::form_geometry::repo_root().join("design/forms/geometry");
        let mut drafts: Vec<String> = std::fs::read_dir(&geom)
            .unwrap_or_else(|e| panic!("{}: {e}", geom.display()))
            .filter_map(|e| e.ok())
            .filter_map(|e| e.file_name().into_string().ok())
            .filter_map(|n| n.strip_suffix(".json").map(str::to_string))
            .filter(|s| s.ends_with("--2026-DRAFT"))
            .collect();
        drafts.sort();
        assert!(
            !drafts.is_empty(),
            "no TY2026 draft geometry found under {} — this test would otherwise pass vacuously",
            geom.display()
        );

        let mut witnessed = Vec::new();
        let mut blind = Vec::new();
        for draft in &drafts {
            let form = draft.split("--").next().expect("stem has a `--`");
            let prior = format!("{form}--2025");
            let d = match compute(&prior, draft) {
                Ok(d) => d,
                // No committed observation on one side: the pair cannot be diffed at all. Named,
                // not silently dropped.
                Err(e) => {
                    blind.push(format!("{prior} -> {draft}: {e}"));
                    continue;
                }
            };
            assert_eq!(
                d.label_compared + d.label_unwitnessed.len(),
                d.common.len(),
                "{prior} -> {draft}: compared {} + unwitnessed {} != {} common — a field was \
                 dropped rather than accounted for",
                d.label_compared,
                d.label_unwitnessed.len(),
                d.common.len()
            );
            match d.label_verdict() {
                LabelVerdict::Unchanged { compared, .. } | LabelVerdict::Moved { compared, .. } => {
                    assert!(
                        compared > 0,
                        "{prior} -> {draft}: a verdict about drift may not be reported with zero \
                         comparisons behind it"
                    );
                    witnessed.push(format!("{prior} -> {draft}: {compared} compared"));
                }
                v @ (LabelVerdict::NoFixture { .. } | LabelVerdict::Unwitnessed { .. }) => {
                    blind.push(format!("{prior} -> {draft}: {v:?}"));
                }
            }
        }
        assert!(
            !witnessed.is_empty(),
            "not one archived 2025 -> 2026-DRAFT pair produced a witnessed label verdict, so this \
             test proved nothing. Unwitnessed pairs were: {blind:#?}"
        );
    }

    // ══════════════════════════════════════════════════════════════════════════════════════════
    // B1 — the three axes, each watched RED on a planted defect and CLEAN on a case it must not
    // fire on. Every pair named here is a committed fixture; every count is the tool's own.
    // ══════════════════════════════════════════════════════════════════════════════════════════

    /// Every consecutive-year pair the committed geometry fixtures can form, enumerated **from the
    /// filesystem**. Never a hand-list: the corpus grows every time a revision is archived, and a
    /// typed pair list is precisely the shape this repo's own rule forbids.
    fn consecutive_pairs() -> Vec<(String, String)> {
        let dir = crate::form_geometry::repo_root().join("design/forms/geometry");
        let mut by_form: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for e in std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
            .flatten()
        {
            let Some(stem) = e
                .file_name()
                .into_string()
                .ok()
                .and_then(|n| n.strip_suffix(".json").map(str::to_string))
            else {
                continue;
            };
            let Some((form, year)) = stem.split_once("--") else {
                continue;
            };
            by_form
                .entry(form.to_string())
                .or_default()
                .push(year.to_string());
        }
        let mut out = Vec::new();
        for (form, mut years) in by_form {
            years.sort_by_key(|y| {
                y.trim_end_matches("-DRAFT")
                    .parse::<u32>()
                    .unwrap_or_else(|_| panic!("{form}--{y}: year is not a number"))
            });
            for w in years.windows(2) {
                out.push((format!("{form}--{}", w[0]), format!("{form}--{}", w[1])));
            }
        }
        assert!(
            out.len() >= 50,
            "expected the committed corpus to form at least 50 consecutive pairs, found {}",
            out.len()
        );
        out
    }

    fn boxes_of(stem: &str) -> BTreeSet<String> {
        super::field_set(stem).unwrap_or_else(|e| panic!("{stem}: {e}"))
    }

    // ───────────────────────────── Axis A — FR-190 ─────────────────────────────

    /// ★★★ **The FR-190 reproduction, and the kill.** `f1040s3--2021`'s root subform is `form1[0]`
    /// and `f1040s3--2022`'s is `topmostSubform[0]`; nothing else about the 41 boxes differs. On a
    /// full-FQN key the intersection is **0**, and `form-delta` printed
    /// `0 common / 40 added / 41 removed` plus its loudest banner — *"LINE→LABEL DRIFT UNWITNESSED"*,
    /// exit 1 — for the calmest transition in the corpus.
    ///
    /// The first assertion here IS the planted defect: it computes the old key on the real artifacts
    /// and shows it seeing nothing. Revert `compute` to `a.intersection(&b)` and everything below it
    /// reds.
    #[test]
    fn axis_a_compares_across_a_root_container_rename() {
        let (a, b) = ("f1040s3--2021", "f1040s3--2022");
        let (na, nb) = (boxes_of(a), boxes_of(b));
        assert_eq!(
            na.intersection(&nb).count(),
            0,
            "the premise: a full-FQN key sees NOTHING in common here"
        );
        let p = pair_fields(&na, &nb);
        assert_eq!(p.pairs.len(), 40, "the ladder pairs 40 of the 41 boxes");
        assert_eq!(
            p.renamed.len(),
            40,
            "and every one of them is RESPELLED — each is a map edit the old key could not name"
        );
        assert_eq!(p.added.len(), 0);
        assert_eq!(p.removed.len(), 1, "one box genuinely retired");
        let d = compute(a, b).expect("both revisions are archived");
        assert!(
            d.label_verdict().is_witnessed(),
            "the axis the rename used to empty must now compare: {:?}",
            d.label_verdict()
        );
        assert_eq!(
            d.label_compared, 38,
            "38 of the 40 pairs yield a printed label on both sides"
        );
        assert!(
            d.label_moved.is_empty(),
            "…and this really is the calm transition it always was: {:?}",
            d.label_moved
        );
    }

    /// ★★ **The planted container rename the brief asks for: an otherwise IDENTICAL pair.** Every box
    /// of a real fixture is re-rooted under a different subform and nothing else is touched, so the
    /// only thing that can change the answer is the key. A full-FQN key pairs zero; the ladder pairs
    /// all of them and names every rename.
    #[test]
    fn axis_a_still_compares_when_a_planted_rename_is_the_only_difference() {
        for stem in ["f6251--2025", "f1040sd--2025", "fw2--2025"] {
            let original = boxes_of(stem);
            let renamed: BTreeSet<String> = original
                .iter()
                .map(|n| match n.split_once('.') {
                    Some((_, rest)) => format!("PLANTED[0].{rest}"),
                    None => format!("PLANTED[0].{n}"),
                })
                .collect();
            assert_eq!(
                renamed.len(),
                original.len(),
                "{stem}: the plant must be a bijection, or it is testing something else"
            );
            assert_eq!(
                original.intersection(&renamed).count(),
                0,
                "{stem}: the premise — a full-FQN key pairs NOTHING after a root rename"
            );
            let p = pair_fields(&original, &renamed);
            assert_eq!(
                p.pairs.len(),
                original.len(),
                "{stem}: the ladder must pair every box across a pure container rename"
            );
            assert_eq!(
                p.renamed.len(),
                original.len(),
                "{stem}: …and report every one as respelled, because the map stores full FQNs"
            );
            assert!(
                p.added.is_empty() && p.removed.is_empty() && p.ambiguous.is_empty(),
                "{stem}: nothing was added, removed or ambiguous — only renamed"
            );
        }
    }

    /// ★★ **Monotone by construction, and checked on the real corpus.** `k` at the maximum depth is
    /// the whole FQN, so every exact match is taken at the top of the ladder — the new key can never
    /// pair FEWER boxes than the old one. Without this, "container-insensitive" could quietly trade
    /// one blindness for another.
    #[test]
    fn the_ladder_never_pairs_fewer_boxes_than_the_full_fqn_key() {
        let (mut full, mut ladder, mut gained) = (0usize, 0usize, 0usize);
        for (a, b) in consecutive_pairs() {
            let (na, nb) = (boxes_of(&a), boxes_of(&b));
            let exact = na.intersection(&nb).count();
            let p = pair_fields(&na, &nb);
            assert!(
                p.pairs.len() >= exact,
                "{a} -> {b}: the ladder paired {} where the full-FQN key paired {exact}",
                p.pairs.len()
            );
            // an exact match must be paired WITH ITSELF, never re-pointed at some other box
            for n in na.intersection(&nb) {
                assert_eq!(
                    p.pairs.get(n).map(String::as_str),
                    Some(n.as_str()),
                    "{a} -> {b}: {n} exists on both sides and must pair with itself"
                );
            }
            full += exact;
            ladder += p.pairs.len();
            if p.pairs.len() > exact {
                gained += 1;
            }
        }
        eprintln!("full-FQN {full} boxes, suffix ladder {ladder}, improved on {gained} pairs");
        assert!(
            ladder > full,
            "if the ladder pairs no more than the old key anywhere, FR-190 is not fixed"
        );
    }

    /// ★★ **The accounting invariant for axis A: every box is paired, added, removed, or NAMED as
    /// ambiguous.** There is no fourth bucket, which is what makes the `added`/`removed` counts mean
    /// what they say. Drop the ambiguity branch and boxes start appearing in `removed` that are
    /// plainly still on the form.
    #[test]
    fn every_box_is_paired_added_removed_or_named_ambiguous() {
        for (a, b) in consecutive_pairs() {
            let (na, nb) = (boxes_of(&a), boxes_of(&b));
            let p = pair_fields(&na, &nb);
            let amb = |side: &str| p.ambiguous.iter().filter(|(s, _)| *s == side).count();
            assert_eq!(
                p.pairs.len() + p.removed.len() + amb("old"),
                na.len(),
                "{a} -> {b}: the OLD side's boxes are not all accounted for"
            );
            assert_eq!(
                p.pairs.len() + p.added.len() + amb("new"),
                nb.len(),
                "{a} -> {b}: the NEW side's boxes are not all accounted for"
            );
        }
    }

    /// ★★★ **The ladder must NEVER merge two boxes that share a leaf.** A W-2 prints the same form
    /// several times on one page, so `f2_01[0]` exists in every copy: `fw2--2024` has **272** boxes
    /// and only **92** distinct leaves. A leaf-only key would collapse 272 boxes into 92 and then
    /// compare the wrong ones against each other — trading FR-190's blindness for a wrong answer,
    /// which is worse. Uniqueness is required on both sides, so the ladder pairs all 272 at the top
    /// and never reaches the leaf.
    #[test]
    fn the_ladder_never_merges_two_boxes_that_share_a_leaf() {
        let na = boxes_of("fw2--2024");
        let nb = boxes_of("fw2--2025");
        let leaf = |n: &String| super::suffix(n, 1).to_string();
        let leaves: BTreeSet<String> = na.iter().map(leaf).collect();
        assert_eq!(na.len(), 272, "the premise: 272 boxes");
        assert_eq!(
            leaves.len(),
            92,
            "…sharing only 92 distinct leaf names, so a leaf key would lose 180"
        );
        let p = pair_fields(&na, &nb);
        assert_eq!(p.pairs.len(), 272, "all 272 pair, on the full FQN");
        assert!(p.renamed.is_empty() && p.ambiguous.is_empty());
        // and the pairing is injective everywhere, on every pair in the corpus
        for (a, b) in consecutive_pairs() {
            let p = pair_fields(&boxes_of(&a), &boxes_of(&b));
            let targets: BTreeSet<&String> = p.pairs.values().collect();
            assert_eq!(
                targets.len(),
                p.pairs.len(),
                "{a} -> {b}: two old boxes were paired with ONE new box"
            );
        }
    }

    // ───────────────────────────── Axis B — FR-191 ─────────────────────────────

    /// ★★★ **FR-191's own measurement, made a test.** Schedule 8812 TY2021 → TY2022 killed **34
    /// printed lines** and `form-delta` said *"54 removed"* AcroForm field names, eight shown, none
    /// labelled. Now every retired line number is named. A retired line matters because code that
    /// still reads it reads a **blank**, and a blank and a zero are the same thing on the page.
    #[test]
    fn axis_b_names_every_retired_line_number() {
        let d = compute("f1040s8--2021", "f1040s8--2022").expect("both revisions are archived");
        let ls = d.line_set.as_ref().expect("both label sets are readable");
        assert_eq!(
            ls.retired.len(),
            34,
            "FR-191 measured 34 retired printed lines: {:?}",
            ls.retired
        );
        // the specific ones, spot-checked rather than merely counted: the 14a-14i / 15a-15h blocks
        for l in ["4a", "4b", "4c", "14a", "14i", "15a", "15h", "28a", "40"] {
            assert!(
                ls.retired.contains(l),
                "line {l} is printed on TY2021 and not on TY2022, and must be named"
            );
        }
        assert_eq!(
            ls.introduced.len(),
            3,
            "…against 3 introduced: {:?}",
            ls.introduced
        );
        assert_eq!(ls.survived.len(), 29);
    }

    /// ★★ **The JOIN, made checkable.** FR-191's actual complaint was that two instruments existed
    /// and nothing connected them: `label-census` could already answer *"which line numbers does this
    /// revision print"*. So axis B must be reading exactly what `label-census` prints — not a second
    /// reader that can drift from it.
    #[test]
    fn axis_b_reads_the_same_line_set_label_census_prints() {
        for stem in [
            "f1040s8--2022",
            "f1040sa--2025",
            "f6251--2025",
            "f1040s3--2020",
        ] {
            let g = crate::form_geometry::load(&crate::form_geometry::repo_root(), stem).unwrap();
            // what `label_reader::run` (label-census) prints, distinct
            let census: BTreeSet<String> = crate::label_reader::witness_text(&g)
                .unwrap()
                .into_iter()
                .map(|(l, _, _)| l)
                .collect();
            let caps = crate::label_reader::caption_join_public(stem).unwrap();
            assert_eq!(
                caps.by_label.keys().cloned().collect::<BTreeSet<String>>(),
                census,
                "{stem}: and the caption reader must cover every census label, ambiguous ones \
                 included — a label the captions do not carry is a line axis C says nothing about \
                 without saying so"
            );
        }
    }

    // ───────────────────────────── Axis C — FR-192 ─────────────────────────────

    fn collisions(a: &str, b: &str) -> (Delta, BTreeMap<String, Collision>) {
        let d = compute(a, b).unwrap_or_else(|e| panic!("{a} -> {b}: {e}"));
        let c = d.caption_collisions.clone();
        (d, c)
    }

    /// ★★★ **FR-185 — Schedule A's six-line cascade, on the form the owner's own return uses.** Each
    /// of these numbers carries a DIFFERENT quantity in TY2026, and not one of them is visible to a
    /// name diff or a label diff. The adverse one is line 13: `charitable.rs` documents the
    /// prior-year carryover as "Schedule A line 13", and in TY2026 line 13 is the worksheet-limited
    /// current-year gift total — systematically larger, so reading 13 as the carryover takes a bigger
    /// deduction and less tax. **Understatement is the worse direction under this project's rule.**
    #[test]
    fn axis_c_reds_on_the_schedule_a_cascade() {
        let (_, c) = collisions("f1040sa--2025", "f1040sa--2026-DRAFT");
        let has = |l: &str, was: &str, now: &str| {
            let x = c.get(l).unwrap_or_else(|| {
                panic!(
                    "line {l} must be reported as a collision; got {:?}",
                    c.keys()
                )
            });
            assert!(
                x.old.join(" ").contains(was),
                "line {l} WAS {was:?}, reader says {:?}",
                x.old.join(" ")
            );
            assert!(
                x.new.join(" ").contains(now),
                "line {l} NOW {now:?}, reader says {:?}",
                x.new.join(" ")
            );
        };
        has(
            "13",
            "carryover from prior year",
            "charitable contribution limitation worksheet",
        );
        has("14", "add lines 11 through 13", "carryover from prior year");
        has("15", "casualty and theft", "add lines 13 and 14");
        has(
            "16",
            "other from list in instructions",
            "casualty and theft",
        );
        has("18", "if you elect to itemize deductions", "$384 350");
        // ★ line 13's old text is still on the form, one number down: a PURE renumber, and every
        //   cross-reference to 13 must become 14.
        assert_eq!(
            c["13"].travelled,
            Some(Travel::Verbatim {
                to: "14".to_string()
            }),
            "line 13's carryover text now prints at line 14, verbatim"
        );
        // ★★ line 18 is the worst shape available: a checkbox becomes a six-figure dollar amount.
        assert_eq!(
            c["18"].travelled,
            Some(Travel::Verbatim {
                to: "19".to_string()
            }),
            "the itemize-anyway CHECKBOX moved to 19 and line 18 is now the TOTAL"
        );
    }

    /// ★★★ **FR-187 — the subtle one, and the reason a moved line is still compared at its own
    /// number.** The casualty and theft line moved 15 → 16 **and changed its text**: TY2025 reads
    /// *"from a federally declared disaster"*, TY2026 *"from a federally or state-declared
    /// disaster"*. That is a **widening of who qualifies**, carried inside a line that also moved, so
    /// a move table recording "15 → 16, same quantity" is right about the number and wrong about the
    /// rule.
    ///
    /// Plant: make `caption_axis` skip a label whose text is found elsewhere (the obvious "it is only
    /// a renumber" optimisation) and this reds on both facts at once.
    #[test]
    fn axis_c_catches_an_eligibility_change_on_a_line_that_also_moved() {
        let (_, c) = collisions("f1040sa--2025", "f1040sa--2026-DRAFT");
        let fifteen = &c["15"];
        // FACT 1 — the number moved, and it is reported as a move.
        let Some(Travel::Reworded { to, similarity }) = &fifteen.travelled else {
            panic!(
                "line 15's text must be recognised at another number: {:?}",
                fifteen.travelled
            );
        };
        assert_eq!(to, "16", "the casualty line moved 15 -> 16");
        assert!(
            *similarity > 0.8,
            "…and it is nearly the same sentence ({similarity:.2})"
        );
        // FACT 2 — and the RULE changed, which is what a move table would have lost.
        let old = fifteen.old.join(" ");
        let new = c["16"].new.join(" ");
        assert!(
            old.contains("federally declared disaster") && !old.contains("state"),
            "TY2025 reads \"federally declared\": {old:?}"
        );
        assert!(
            new.contains("federally or state declared disaster"),
            "TY2026 reads \"federally or state-declared\" — a WIDENING of who qualifies: {new:?}"
        );
    }

    /// ★★★ **FR-192's own measurement: Schedule 3 TY2020 → TY2021, thirteen surviving line numbers
    /// and NINE naming a different quantity.** Line 7 moves from *the Part I total* to an *other
    /// credits* subtotal — which **double-counts lines 1–5** if the old cross-reference is carried
    /// forward — and line 8 moves from *net premium tax credit* to *the Part I total*.
    #[test]
    fn axis_c_reds_on_schedule_3_nine_of_thirteen() {
        let (d, c) = collisions("f1040s3--2020", "f1040s3--2021");
        assert_eq!(
            d.caption_compared, 13,
            "13 line numbers survive and are comparable"
        );
        assert_eq!(
            c.len(),
            9,
            "FR-192 measured NINE of them naming a different quantity: {:?}",
            c.keys()
        );
        assert!(
            c["7"].old.join(" ").contains("add lines 1 through 6")
                && c["7"]
                    .new
                    .join(" ")
                    .contains("total other nonrefundable credits"),
            "line 7: the Part I total became an other-credits subtotal — carrying the old \
             cross-reference forward double-counts lines 1-5: {:?} -> {:?}",
            c["7"].old.join(" "),
            c["7"].new.join(" ")
        );
        assert!(
            c["8"].old.join(" ").contains("net premium tax credit")
                && c["8"].new.join(" ").contains("add lines 1 through 5 and 7"),
            "line 8: net premium tax credit became the Part I total"
        );
        assert_eq!(
            c["8"].travelled,
            Some(Travel::Verbatim {
                to: "9".to_string()
            }),
            "…and the premium-credit text is now line 9"
        );
    }

    /// ★★ **Schedule 1-A 37 → 43** — the collision found 2026-09-11, which took two review rounds to
    /// hold by a type. It is the third independent form showing the shape in one day, which is what
    /// makes this a pattern rather than an anomaly.
    #[test]
    fn axis_c_reds_on_schedule_1a_37_to_43() {
        let (_, c) = collisions("f1040s1a--2025", "f1040s1a--2026-DRAFT");
        let thirty_seven = c
            .get("37")
            .expect("line 37 must be reported as a collision");
        assert!(
            thirty_seven
                .old
                .join(" ")
                .contains("enhanced deduction for seniors"),
            "TY2025 line 37 is the seniors' enhanced deduction: {:?}",
            thirty_seven.old.join(" ")
        );
        match &thirty_seven.travelled {
            Some(Travel::Reworded { to, .. }) | Some(Travel::Verbatim { to }) => {
                assert_eq!(to, "43", "its text now prints at line 43")
            }
            None => panic!("line 37's text must be found at its new number"),
        }
    }

    /// ★★★ **THE NEGATIVE CASE: axis C must be SILENT on a rename that changed no printed line, or
    /// it is a noise generator rather than a check.**
    ///
    /// TY2025's Form 8995 respelled 9 of its 24 paired boxes and added and retired 9 more, and the
    /// printed form did not change at all. Axis C must compare all 18 of its line numbers and report
    /// **zero** collisions.
    ///
    /// ★ This pair is also the calibration that forced two of the reader's rules. Without the gutter
    /// COLUMN rule it reported lines 3 and 7 as reworded, because the gutter numeral sits 12.30pt left
    /// of its box in TY2024 and 11.30pt in TY2025 — *the box moved, not the numeral*. Without
    /// visual-line grouping, a one-point y shift re-ordered a single word against its neighbour.
    /// Restore either and this reds.
    #[test]
    fn axis_c_is_silent_on_a_rename_that_changed_no_printed_line() {
        let (d, c) = collisions("f8995--2024", "f8995--2025");
        assert_eq!(d.renamed.len(), 9, "the premise: 9 boxes were respelled");
        assert_eq!(
            d.caption_compared, 18,
            "all 18 printed line numbers compared"
        );
        assert!(
            c.is_empty(),
            "the printed form did not change, so NO line number may be reported as meaning \
             something else: {c:#?}"
        );
        assert!(
            d.caption_gaps.is_empty(),
            "…and none of them may be skipped either: {:?}",
            d.caption_gaps
        );
        let ls = d.line_set.as_ref().unwrap();
        assert!(ls.retired.is_empty() && ls.introduced.is_empty());
    }

    /// ★★★ **A caption check that reds on REFLOW is worthless, so the two pairs where reflow bit are
    /// pinned cell for cell.** Both were real false positives during calibration, both from layout
    /// alone, and each needed a different fix:
    ///
    /// * `f1040sse--2024` → `--2025` line 15 reported itself reworded because a one-point `y` shift
    ///   moved the sub-item markers `a` and `b` across a rounding boundary and swapped them against
    ///   their neighbours. Fixed by grouping words into visual LINES before ordering them.
    /// * `f1040sa--2024` → `--2025` line 15 reported itself reworded because its caption's prose `18`
    ///   (*"enter the amount from line 18 of that form"*) wraps to x2=407.46 on TY2024 and x2=404.09
    ///   on TY2025 — inside the gutter COLUMN on the second revision only, so the locator filter ate
    ///   it from one caption and not the other. Fixed by requiring a rule-3 locator to have a box on
    ///   its own row, which a prose number never does.
    ///
    /// Every collision these pairs still report is a real TY2025 change: the 1040's AGI line moving
    /// from `11` to `11b`, the SALT cap going $10,000 → $40,000, the SE wage base $168,600 →
    /// $176,100, and `line 12` → `line 12e`. Restore either layout bug and an extra line appears.
    #[test]
    fn a_reflowed_caption_is_not_a_reworded_line() {
        let (_, c) = collisions("f1040sa--2024", "f1040sa--2025");
        assert_eq!(
            c.keys().map(String::as_str).collect::<Vec<_>>(),
            ["17", "2", "5", "5e"],
            "Schedule A TY2024 -> TY2025: exactly the four real changes, and line 15 — whose \
             sentence is IDENTICAL — must not be among them"
        );
        assert!(
            c["2"].new.join(" ").contains("line 11b") && c["2"].old.join(" ").contains("line 11"),
            "line 2 now points at the 1040's line 11b: {:?}",
            c["2"].new.join(" ")
        );
        assert!(
            c["5e"].old.join(" ").contains("$10") && c["5e"].new.join(" ").contains("$40"),
            "line 5e is the SALT cap going $10,000 -> $40,000"
        );
        let (_, c) = collisions("f1040sse--2024", "f1040sse--2025");
        assert_eq!(
            c.keys().map(String::as_str).collect::<Vec<_>>(),
            ["14", "15", "7", "8a"],
            "Schedule SE TY2024 -> TY2025: the wage-base and threshold lines only"
        );
        assert!(
            c["8a"].old.join(" ").contains("$168 600")
                && c["8a"].new.join(" ").contains("$176 100"),
            "line 8a is the social-security wage base: {:?} -> {:?}",
            c["8a"].old.join(" "),
            c["8a"].new.join(" ")
        );
    }

    /// ★★★ **The `DRAFT` / `DO NOT FILE` watermark must never reach a caption, and the ceiling that
    /// keeps it out is watched RED on a planted defect.**
    ///
    /// January's whole job is diffing a draft against its final, and the final has no watermark — so
    /// one watermark token inside a caption span reds every line it touches. A draft prints
    /// `DRAFT` / `DO NOT FILE` down both margins at 16.0–53.63pt where its body text is 9.33–10.49pt,
    /// and those spans overlap real lines.
    ///
    /// **The plant is the ceiling itself.** Raised past the watermark, the token `draft` reaches a
    /// caption on the committed drafts; at the shipped ceiling it reaches none, on any of the 87
    /// fixtures. `draft` is the discriminating token because `do`, `not` and `file` are ordinary
    /// English that IRS captions genuinely use (*"A used vehicle does not qualify"*), and a test that
    /// forbade those would be measuring the language rather than the watermark.
    #[test]
    fn the_draft_watermark_never_reaches_a_caption() {
        let dir = crate::form_geometry::repo_root().join("design/forms/geometry");
        let mut planted_hits = 0;
        let mut readable = 0;
        let mut unreadable: Vec<String> = Vec::new();
        let mut drafts = 0;
        for e in std::fs::read_dir(&dir).unwrap().flatten() {
            let Some(stem) = e
                .file_name()
                .into_string()
                .ok()
                .and_then(|n| n.strip_suffix(".json").map(str::to_string))
            else {
                continue;
            };
            let is_draft = stem.ends_with("-DRAFT");
            if is_draft {
                drafts += 1;
            }
            let g = crate::form_geometry::load(&crate::form_geometry::repo_root(), &stem).unwrap();
            if is_draft {
                assert!(
                    g.words
                        .iter()
                        .any(|w| w.text == "DRAFT" && w.y2 - w.y > 12.0),
                    "{stem}: no oversized DRAFT token — the premise must be checked or this test \
                     passes vacuously"
                );
            }
            // the SHIPPED ceiling: no caption anywhere may carry the watermark
            let Ok(caps) = crate::label_reader::caption_join(&g) else {
                unreadable.push(stem);
                continue;
            };
            readable += 1;
            for (l, toks) in &caps.by_label {
                assert!(
                    !toks.iter().any(|t| t == "draft"),
                    "{stem} line {l}: the caption carries the watermark token \"draft\" — every \
                     line it touches would red against the final: {toks:?}"
                );
            }
            // THE PLANT: the same reader with the ceiling raised past the watermark
            if is_draft {
                let loose = crate::label_reader::caption_join_tuned(&g, 6.0).unwrap();
                planted_hits += loose
                    .by_label
                    .values()
                    .filter(|t| t.iter().any(|t| t == "draft"))
                    .count();
            }
        }
        assert!(
            drafts >= 15,
            "expected the 15 committed drafts, found {drafts}"
        );
        assert!(
            readable >= 80,
            "only {readable} of 87 fixtures yielded captions (unreadable: {unreadable:?})"
        );
        assert!(
            planted_hits > 0,
            "with the height ceiling raised past the watermark, NO caption picked the watermark up \
             — so the ceiling is not what is keeping it out and this test proves nothing"
        );
        eprintln!(
            "watermark: {readable} fixtures clean at the shipped ceiling; {planted_hits} captions \
             contaminated with the ceiling raised to 6.0x"
        );
    }

    /// ★★★ **The body-height ceiling is DERIVED per page, and here is the defect that proves it has
    /// to be.**
    ///
    /// The first cut was an absolute 12.0pt, measured across all 87 fixtures as sitting in a real gap
    /// (largest body word 11.95pt, smallest heading 12.43pt). `f1040s3--2020` sets its body text at
    /// **12.83pt**, so every caption on Schedule 3 for TY2020, TY2021 and TY2022 came out EMPTY — and
    /// axis C reported all three pairs as **clean**, which is the cleanest possible verdict from no
    /// evidence at all.
    ///
    /// Plant: replace `body_h * BODY_HEIGHT_FACTOR` with any absolute constant and this reds.
    #[test]
    fn the_body_height_ceiling_is_derived_per_page_not_typed() {
        // the premise: this form's body text is TALLER than any plausible typed ceiling
        let g = crate::form_geometry::load(&crate::form_geometry::repo_root(), "f1040s3--2020")
            .unwrap();
        let mut hist: BTreeMap<i64, usize> = BTreeMap::new();
        for w in &g.words {
            *hist
                .entry(((w.y2 - w.y) * 100.0).round() as i64)
                .or_default() += 1;
        }
        let (modal, n) = hist.iter().max_by_key(|(_, n)| **n).unwrap();
        assert_eq!(
            (*modal, *n),
            (1283, 341),
            "f1040s3--2020's commonest word height is 12.83pt, over 341 words"
        );
        // and every caption on it is non-empty, on every revision that shares the body size
        for stem in ["f1040s3--2020", "f1040s3--2021", "f1040s3--2022"] {
            let caps = crate::label_reader::caption_join_public(stem).unwrap();
            let empty: Vec<&String> = caps
                .by_label
                .iter()
                .filter(|(_, t)| t.is_empty())
                .map(|(l, _)| l)
                .collect();
            assert!(
                empty.is_empty(),
                "{stem}: {} of {} captions came out EMPTY — an absolute height ceiling reads this \
                 form as having no text at all, and axis C then reports it CLEAN: {empty:?}",
                empty.len(),
                caps.by_label.len()
            );
        }
    }

    // ───────────── Axis C's own accounting, on inputs built by hand ─────────────

    fn cs(pairs: &[(&str, &str)], ambiguous: &[&str]) -> crate::label_reader::CaptionSet {
        crate::label_reader::CaptionSet {
            by_label: pairs
                .iter()
                .map(|(l, c)| {
                    (
                        (*l).to_string(),
                        crate::label_reader::normalise_caption_text(c),
                    )
                })
                .collect(),
            ambiguous: ambiguous.iter().map(|l| (*l).to_string()).collect(),
        }
    }

    /// ★★★ **Every surviving line number is either a caption comparison or a NAMED gap**, and an
    /// evidence-free caption axis is never a clean one. Same discipline as the label axis, for the
    /// same reason: an empty `collisions` map is what *"nothing changed"* and *"nothing was looked
    /// at"* both look like.
    #[test]
    fn every_surviving_line_is_either_compared_or_named_as_a_caption_gap() {
        let survived: BTreeSet<String> = ["1", "2", "3", "4", "5", "6"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let old = cs(
            &[
                ("1", "Wages, salaries, tips"),
                ("2", "Tax-exempt interest . . . . ."),
                ("3", ""),
                ("4", "Ordinary dividends"),
                ("5", "Pensions and annuities"),
                ("6", "Social security benefits"),
            ],
            &["5"],
        );
        let new = cs(
            &[
                ("1", "Wages, salaries, tips"),
                ("2", "Taxable interest"),
                ("3", "Now it says something"),
                ("4", ""),
                ("5", "Pensions and annuities"),
                ("6", "Social security benefits"),
            ],
            &["6"],
        );
        let a = caption_axis(&survived, &old, &new);
        assert_eq!(
            a.compared + a.gaps.len(),
            survived.len(),
            "compared {} + gaps {} must account for all {} surviving line numbers",
            a.compared,
            a.gaps.len(),
            survived.len()
        );
        assert_eq!(a.compared, 2, "only lines 1 and 2 are comparable");
        assert_eq!(a.collisions.keys().collect::<Vec<_>>(), ["2"]);
        assert_eq!(a.gaps["3"], CaptionGap::EmptyOld);
        assert_eq!(a.gaps["4"], CaptionGap::EmptyNew);
        assert_eq!(a.gaps["5"], CaptionGap::AmbiguousOld);
        assert_eq!(a.gaps["6"], CaptionGap::AmbiguousNew);
        // every way of failing to read a caption must be distinguishable in the report
        let reasons: BTreeSet<CaptionGap> = a.gaps.values().copied().collect();
        assert_eq!(reasons.len(), 4);
    }

    /// ★★ **Whitespace, dotted leaders and typography are irrelevant; words and numbers are not.**
    /// A caption check that reds on reflow is worthless, and one that cannot see a changed
    /// cross-reference is worse than worthless.
    #[test]
    fn materially_changed_ignores_layout_and_never_ignores_a_word_or_a_number() {
        let n = crate::label_reader::normalise_caption_text;
        let same = [
            (
                "Add lines 1 and 2 . . . . . . . . . . .",
                "Add   lines 1\nand  2 ....",
            ),
            ("Other—from list", "Other - from list"),
            ("you didn\u{2019}t use", "you didn't use"),
            ("(see instructions).", "see instructions"),
            ("TAXES YOU PAID", "Taxes you paid"),
        ];
        for (a, b) in same {
            assert_eq!(n(a), n(b), "{a:?} and {b:?} are the same caption");
        }
        let differ = [
            // the Form 6251 line-33 defect: a cross-reference that changed by one character
            (
                "Subtract line 32 from line 22",
                "Subtract line 32 from line 12",
            ),
            // FR-187: an eligibility clause widening inside an otherwise identical sentence
            (
                "from a federally declared disaster",
                "from a federally or state-declared disaster",
            ),
            // an inflation-adjusted threshold
            ("Enter $191,950", "Enter $197,300"),
            // and a reordering, which the Form 6251 defect class makes load-bearing
            (
                "Subtract line 12 from line 22",
                "Subtract line 22 from line 12",
            ),
        ];
        for (a, b) in differ {
            assert_ne!(n(a), n(b), "{a:?} and {b:?} must NOT compare equal");
        }
    }

    /// ★★ **Travel is a report, never a licence to skip.** A changed line's old text is looked for at
    /// every other number, and the answer is one of three — verbatim, reworded, or gone — but the
    /// collision is reported either way.
    #[test]
    fn travel_reports_verbatim_reworded_and_gone_without_suppressing_the_collision() {
        let survived: BTreeSet<String> = ["1", "2", "3"].iter().map(|s| s.to_string()).collect();
        let old = cs(
            &[
                ("1", "Carryover from prior year"),
                (
                    "2",
                    "Casualty and theft loss from a federally declared disaster",
                ),
                ("3", "Health coverage tax credit from Form 8885"),
            ],
            &[],
        );
        let new = cs(
            &[
                ("1", "Enter the amount from line 6 of the worksheet"),
                ("2", "Add lines 13 and 14"),
                ("3", "Reserved for future use"),
                // line 1's text, verbatim, at a number outside `survived`
                ("4", "Carryover from prior year"),
                // line 2's text, WIDENED
                (
                    "5",
                    "Casualty and theft loss from a federally or state-declared disaster",
                ),
            ],
            &[],
        );
        let a = caption_axis(&survived, &old, &new);
        assert_eq!(a.compared, 3);
        assert_eq!(
            a.collisions.len(),
            3,
            "all three collide, whatever their travel"
        );
        assert_eq!(
            a.collisions["1"].travelled,
            Some(Travel::Verbatim {
                to: "4".to_string()
            })
        );
        let Some(Travel::Reworded { to, similarity }) = &a.collisions["2"].travelled else {
            panic!(
                "line 2 moved AND was reworded: {:?}",
                a.collisions["2"].travelled
            )
        };
        assert_eq!(to, "5");
        assert!(*similarity >= REWORDED_AT && *similarity < 1.0);
        assert_eq!(
            a.collisions["3"].travelled, None,
            "the health-coverage credit is gone, not moved — RETIRED, not renumbered"
        );
    }

    /// ★ The rewording threshold has to separate two measured shapes, or it is a number someone liked:
    /// FR-187's widened casualty sentence scores **0.90** on the real artifacts, while Schedule A's
    /// genuinely unrelated `14` → `15` pair scores **0.43**.
    #[test]
    fn the_rewording_threshold_separates_the_two_measured_shapes() {
        let n = crate::label_reader::normalise_caption_text;
        let widened = similarity(
            &n("Casualty and theft loss(es) from a federally declared disaster"),
            &n("Casualty and theft loss(es) from a federally or state-declared disaster"),
        );
        let unrelated = similarity(&n("Add lines 11 through 13"), &n("Add lines 13 and 14"));
        assert!(
            widened > REWORDED_AT,
            "the same sentence, widened, must read as reworded: {widened:.2}"
        );
        assert!(
            unrelated < REWORDED_AT,
            "two different sentences must not: {unrelated:.2}"
        );
        assert!(
            widened - unrelated > 0.3,
            "and the two must not be close, or the threshold is arbitrary: {widened:.2} vs {unrelated:.2}"
        );
    }

    /// ★★ **No archived pair may report a clean caption verdict from zero comparisons**, and the
    /// accounting invariant holds on the real artifacts too — an invariant that only holds on
    /// hand-built inputs is a tautology. Enumerated from the filesystem, never a hand-list.
    #[test]
    fn no_archived_pair_reports_a_clean_caption_verdict_from_zero_comparisons() {
        let mut witnessed = 0;
        let mut blind: Vec<String> = Vec::new();
        for (a, b) in consecutive_pairs() {
            let Ok(d) = compute(&a, &b) else { continue };
            let Some(ls) = &d.line_set else {
                blind.push(format!(
                    "{a} -> {b}: {}",
                    d.captions_unreadable.as_deref().unwrap_or("?")
                ));
                continue;
            };
            assert_eq!(
                d.caption_compared + d.caption_gaps.len(),
                ls.survived.len(),
                "{a} -> {b}: compared {} + gaps {} != {} surviving line numbers — a line was \
                 dropped rather than accounted for",
                d.caption_compared,
                d.caption_gaps.len(),
                ls.survived.len()
            );
            if d.caption_compared == 0 {
                assert!(
                    d.caption_collisions.is_empty(),
                    "{a} -> {b}: a collision reported with zero comparisons behind it"
                );
                blind.push(format!("{a} -> {b}: 0 of {} compared", ls.survived.len()));
            } else {
                witnessed += 1;
            }
        }
        eprintln!(
            "caption axis: {witnessed} witnessed pairs, {} blind: {blind:#?}",
            blind.len()
        );
        assert!(
            witnessed >= 40,
            "only {witnessed} pairs produced a witnessed caption verdict; the axis is not \
             exercised by the corpus"
        );
    }
}
