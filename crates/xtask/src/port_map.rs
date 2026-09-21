//! `cargo run -p xtask -- port-map <stem> <prior-tag> <new-tag>` — a CANDIDATE field map for a new
//! revision, derived from the prior year's map and refusing every cell it cannot prove.
//!
//! ## Why this exists
//!
//! In January a year's forms are ported at once, under time pressure, for the year that is actually
//! filed. Porting **one** form by hand (f1040, TY2024 → TY2025, 2026-09-20/21) hit four traps, and not
//! one of them is visible to a carry-forward of last year's map:
//!
//! | trap | what it looked like |
//! |---|---|
//! | an FQN exists but is a DIFFERENT cell | TY2024's `taxpayer_ssn` is a `/MaxLen 2` box on TY2025 |
//! | a checkbox's on-state ORDER changed | MFS is `3` on TY2025 and `4` on TY2024 |
//! | the page break MOVED | the descent oracle compared a page-1 `y` with a page-2 `y` |
//! | printed labels RENUMBERED | AGI at `11a`, the deduction at `12e`, QBI at `13a` |
//!
//! The first would have printed a truncated SSN; the second would have filed the return under a
//! different filing status, taking every bracket and phase-out with it. Both were caught by a guard
//! firing at fill time, which is to say: by luck of having a fill to run.
//!
//! ## What it does, and what it refuses to do
//!
//! Every cell of the prior map is re-pointed through [`crate::form_delta::pair_fields`] — the suffix
//! ladder already used by `form-delta` — and then **verified five ways** before it is emitted:
//!
//! 1. **the printed LABEL** beside the new widget equals the cell's own key, because this repo requires
//!    a key to BE the label (`label_reader::every_mapped_line_lands_on_its_own_printed_label`);
//! 2. **the COLUMN band** matches the prior cell's, because a line's sub-line and amount widgets share
//!    one printed label and the label check cannot tell them apart;
//! 3. **`/MaxLen` compatibility** — a cell that held nine digits must still hold nine;
//! 4. **the on-state is one the new widget DECLARES**, for every checkbox;
//! 5. **the PAGE**, reported rather than enforced: the emitter derives descent groups from the page now,
//!    so a moved break is information, not an error.
//!
//! A cell failing any check is **not emitted**. It is listed with the measurement that killed it, for a
//! human to resolve against the form. This tool proposes; it never decides.
//!
//! ★★ **It is not a golden regenerator.** Its output is a CANDIDATE to review, and
//! `tests::the_f1040_ty2025_port_reproduces_the_hand_written_map` holds it to the one port that was done
//! by hand — a known answer, not its own previous output.

use std::collections::{BTreeMap, BTreeSet};

/// The three amount columns of the 1040 family, as centre-x bands (`form1040_full`'s own).
const BANDS: [(&str, f64, f64); 3] = [
    ("SUBLINE", 252.0, 324.0),
    ("MID", 410.0, 482.0),
    ("AMOUNT", 504.0, 576.0),
];

fn band_of(x: f64) -> String {
    BANDS
        .iter()
        .find(|(_, lo, hi)| x >= lo - 1.0 && x <= hi + 1.0)
        .map_or_else(|| format!("other({x:.0})"), |(n, _, _)| (*n).to_string())
}

/// How far an unnamed column may drift between revisions and still be the same column.
///
/// ★★★ **CALIBRATED, not chosen.** Over the 183 census entries that 2024 and 2025 maps resolve to the
/// same line on both revisions, 179 sit at |Δx| = **0.0pt** and the largest true column shift is
/// **7.2pt** (Schedule A lines 6 and 8d, 417.6 → 410.4). Over the 200 same-row widget pairs in the 21
/// committed templates, the CLOSEST two distinct columns are **35.2pt** apart, and **none** is within
/// 12pt. So 12pt clears every real shift with 4.8pt to spare and is 23pt short of merging two columns.
/// If a future layout crosses that, the KATs over the committed maps are what will say so.
const OTHER_COLUMN_TOLERANCE_PT: f64 = 12.0;

/// Are these the same column on two revisions?
///
/// ★★ A NAMED band is compared by identity, exactly as before — the three money bands are 72pt wide and
/// already absorb any drift inside them, so widening them would be a change with no defect behind it.
/// The tolerance applies only where [`band_of`] had to fall back to a bare x, which is where it was
/// **exact-x equality** and therefore could never match: Schedule 1's line-19c date box moved 334.8 →
/// 338.4, so `other(335)` and `other(338)` compared unequal and five entries refused that should carry.
/// Every one of those five is a narrow write-in or date box — precisely the boxes a money band misses.
#[must_use]
fn same_column(was: &Shape, now: &Shape) -> bool {
    let named = |b: &str| !b.starts_with("other(");
    if named(&was.band) || named(&now.band) {
        return was.band == now.band;
    }
    (was.x - now.x).abs() <= OTHER_COLUMN_TOLERANCE_PT
}

/// One cell of a map: its key, the FQN it names, and the on-state when it is a checkbox.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    pub key: String,
    pub fqn: String,
    pub on: Option<String>,
}

/// Every `key = "fqn"` and `key = { field = "fqn", on = "N" }` in a committed map, in file order.
///
/// ★ Deliberately a TEXT scan rather than a `toml` parse: the keys of a map are per-revision (TY2025
/// spells AGI `line11a`), so a typed parse would silently normalise the very thing being ported. The
/// first quoted span is the FQN — a trailing `# comment` on the same line is common in these files and
/// broke an earlier reader that used `trim_matches('"')`.
#[must_use]
pub fn cells_of(map_text: &str) -> Vec<Cell> {
    let mut out = Vec::new();
    // ★★★ **The enclosing TABLE is part of a cell's identity.** A grid writes the same key once per row —
    //     `payer` appears fourteen times in Schedule B — so a flat scan produces fourteen colliding
    //     `payer` cells and reports every one as unresolvable. Keys are therefore qualified
    //     `part1_rows[3].payer`, which is what lets [`grids_of`] carry a grid by ROW INDEX.
    let mut table: Option<String> = None;
    let mut row: BTreeMap<String, usize> = BTreeMap::new();
    for line in map_text.lines() {
        let t = line.trim_start();
        if t.starts_with('#') {
            continue;
        }
        if let Some(rest) = t.strip_prefix("[[") {
            let name = rest.split(']').next().unwrap_or("").to_string();
            let n = row.entry(name.clone()).or_insert(0);
            table = Some(format!("{name}[{}]", *n));
            *n += 1;
            continue;
        }
        if t.starts_with('[') {
            table = t
                .trim_start_matches('[')
                .split(']')
                .next()
                .map(str::to_string);
            continue;
        }
        let Some((k, v)) = t.split_once('=') else {
            continue;
        };
        let key = k.trim();
        if key.is_empty() || !key.chars().next().is_some_and(char::is_alphabetic) {
            continue;
        }
        let v = v.trim();
        let Some(fqn) = v.split('"').nth(1) else {
            continue;
        };
        // ★ An FQN root is NOT always `topmostSubform`: the TY2025 Schedule A template uses `form1[0]`,
        //   and filtering on that one root silently dropped every Schedule A cell — which made the
        //   committed map look EMPTY and produced 20 phantom "committed ABSENT" disagreements. The test
        //   that caught it was comparing the tool against nothing. Shape, not prefix.
        if !fqn.contains("[0]") || !fqn.contains('.') {
            continue;
        }
        let on = v.split("on = \"").nth(1).and_then(|r| r.split('"').next());
        out.push(Cell {
            key: match &table {
                Some(t) => format!("{t}.{key}"),
                None => key.to_string(),
            },
            fqn: fqn.to_string(),
            on: on.map(str::to_string),
        });
    }
    out
}

/// One `[census]` entry: the widget it accounts for, the line it claims, and the decision verbatim.
///
/// ★★★ **A census entry states its own printed label, which a line cell only implies.** `line1 = "…"`
/// says "line 1" through its KEY; `"…f1_25[0]" = { line = "8n", … }` says "8n" in a FIELD. That makes
/// the census the stronger carry of the two: the claim is checked against the printed label on BOTH
/// revisions, so a mis-carry needs both forms to print one label twice in one column — which is the
/// [`Why::AmbiguousLabel`] refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CensusEntry {
    /// The prior revision's FQN — the entry's key.
    pub fqn: String,
    /// The `line` field verbatim, qualifier included: `"8n"`, `"13z amount"`, `"16 box 1"`.
    pub line: String,
    /// The whole inline table, byte-exact, so `rule` and `reason` are re-emitted and never reworded.
    pub rest: String,
}

impl CensusEntry {
    /// The printed LABEL this entry claims — the first whitespace-delimited token of `line`.
    ///
    /// ★★ `line` is a human location, not always a bare label: `"13z amount"` and `"13z type"` are two
    /// widgets of line 13z distinguished by COLUMN, `"34 Yes"` is a checkbox of line 34, and `"1 stat
    /// emp"` is a box beside line 1. The label is the part a form prints in its margin; the qualifier
    /// says which widget of that line, and the column band is what actually resolves it.
    #[must_use]
    pub fn label(&self) -> &str {
        self.line.split_whitespace().next().unwrap_or("")
    }
}

/// Every `"<fqn>" = { … }` line of a map's `[census]` table, in file order.
///
/// ★ Scoped to `[census]` deliberately: it is the only table in any of the 22 committed maps whose keys
/// are quoted FQNs (measured — 763 of 763 quoted-key lines sit under it), and a future table of that
/// shape must be opted in here rather than swept up silently.
#[must_use]
pub fn census_of(map_text: &str) -> Vec<CensusEntry> {
    let mut out = Vec::new();
    let mut in_census = false;
    for line in map_text.lines() {
        let t = line.trim_start();
        if t.starts_with('#') {
            continue;
        }
        if t.starts_with('[') {
            in_census = t.starts_with("[census]");
            continue;
        }
        if !in_census || !t.starts_with('"') {
            continue;
        }
        let mut q = t.split('"');
        let (Some(_), Some(fqn)) = (q.next(), q.next()) else {
            continue;
        };
        let Some((_, rest)) = t.split_once('=') else {
            continue;
        };
        let rest = rest.trim();
        let Some(claim) = rest
            .split("line = \"")
            .nth(1)
            .and_then(|r| r.split('"').next())
        else {
            continue;
        };
        out.push(CensusEntry {
            fqn: fqn.to_string(),
            line: claim.to_string(),
            rest: rest.to_string(),
        });
    }
    out
}

/// What a widget looks like on one revision: where it sits, how much it holds, what it declares.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Shape {
    pub page: u32,
    pub band: String,
    /// The widget's left edge in points — what resolves a column [`band_of`] could not name.
    pub x: f64,
    pub max_len: Option<usize>,
    pub on_states: BTreeSet<String>,
}

/// Read every widget's shape from a bundled template PDF.
pub fn shapes(pdf: &std::path::Path) -> Result<BTreeMap<String, Shape>, String> {
    let bytes = std::fs::read(pdf).map_err(|e| format!("{}: {e}", pdf.display()))?;
    let doc = btctax_forms::testonly::load(&bytes).map_err(|e| format!("load: {e}"))?;
    let fields =
        btctax_forms::testonly::collect_fields(&doc).map_err(|e| format!("fields: {e}"))?;
    let mut out = BTreeMap::new();
    for f in &fields {
        let (page, band, x) = f.rect.map_or((0, String::new(), 0.0), |r| {
            (
                u32::from(f.fqn.contains("Page2")) + 1,
                band_of(f64::from(r[0])),
                f64::from(r[0]),
            )
        });
        out.insert(
            f.fqn.clone(),
            Shape {
                page,
                band,
                x,
                max_len: f.max_len,
                on_states: if f.is_button {
                    btctax_forms::testonly::button_on_states(&doc, f.id)
                        .into_iter()
                        .collect()
                } else {
                    BTreeSet::new()
                },
            },
        );
    }
    Ok(out)
}

/// Why a cell was refused, as DATA.
///
/// ★★★ **Not a substring of `why`.** The first forecast classified refusals by sniffing their prose in a
/// fixed needle order — and the grid refusal's own explanation mentions `/MaxLen`, so breaking the grid
/// needle silently reclassified 110 cells as "capacity changed" instead of leaving them unclassified. The
/// test meant to notice passed. A class the report counts must be a field, not a phrase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Why {
    /// No printed line label to carry — a grid row or an identity cell. Declined by design.
    NotANumberedLine,
    /// Nothing on the new revision carries this label in this column: renumbered, or gone.
    LabelRenumberedOrGone,
    /// Two widgets share one label in one column.
    AmbiguousLabel,
    /// `/MaxLen` changed — nine digits meeting a shorter box.
    CapacityChanged,
    /// The on-state the map writes is not one the new widget declares.
    OnStateNotDeclared,
    /// The prior map's key disagrees with the prior form's printed label.
    PriorMapDisagreesWithPriorForm,
    /// The prior FQN is not a widget of the prior template.
    PriorFqnNotAWidget,
    /// Two prior entries resolve to ONE new widget — the revision merged them.
    TwoPriorCellsOneNewWidget,
}

impl Why {
    /// A human label for the forecast table. ★ `_`-free, so a new variant is a build error here.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotANumberedLine => "grid or identity cell — declined by design",
            Self::LabelRenumberedOrGone => "label renumbered or gone — a human must read the form",
            Self::AmbiguousLabel => "two widgets share one label in one column",
            Self::CapacityChanged => "capacity changed",
            Self::OnStateNotDeclared => "on-state not declared by the new widget",
            Self::PriorMapDisagreesWithPriorForm => "the prior map disagrees with the prior form",
            Self::PriorFqnNotAWidget => "prior FQN is not a widget of the prior template",
            Self::TwoPriorCellsOneNewWidget => "two prior entries resolve to one new widget",
        }
    }
}

/// A cell the tool would not emit, and the measurement that killed it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refused {
    pub key: String,
    pub candidate: String,
    pub class: Why,
    pub why: String,
}

/// The result of a port: cells that passed every check, cells refused, and the page moves.
#[derive(Debug, Default)]
pub struct Port {
    pub emitted: Vec<Cell>,
    /// Carried `[census]` entries: `(the NEW revision's fqn, the prior entry verbatim)`.
    pub census: Vec<(String, CensusEntry)>,
    pub refused: Vec<Refused>,
    /// `key: prior page → new page`, reported because the emitter derives descent groups from the page.
    pub page_moves: Vec<String>,
}

/// Derive a candidate map from `prior`'s cells.
///
/// ★★★ **THE PRINTED LABEL IS THE IDENTITY; THE FQN IS AN IMPLEMENTATION DETAIL.** The first version of
/// this paired cells through [`crate::form_delta::pair_fields`], the suffix ladder `form-delta` uses.
/// That pairs by FQN NAME, and it produced **0 of 102** usable cells: TY2025 inserted widgets above the
/// income block, so `f1_47` names line 4b on TY2024 and line 1a on TY2025. The ladder is right for
/// COUNTING what changed — its actual job — and wrong for deciding what a cell means.
///
/// So a line cell is carried by `(printed label, column band)`: the new widget is the one this form
/// prints the SAME label beside, in the SAME column. That transfers every line the revision did not
/// renumber, and refuses the ones it did — `11 → 11a` cannot be inferred, it has to be read, which is
/// exactly the judgement a tool must not make.
///
/// ★★ Cells with no printed line label — the identity block, the filing-status radios, the dependents
/// grid — are refused by design, and that refusal is now MEASURED rather than argued.
///
/// ★★★ **The obvious shortcut was tried and is catastrophically wrong.** Carrying an identity cell by
/// `(page, column x, rank within the column)` — which is roughly how the f1040 header was resolved by
/// hand — reproduces **5** of the 29 hand-mapped TY2025 header cells. **12 are WRONG and 12 are
/// unpredictable.** Ranking within a column mixes checkboxes and text fields, and TY2025 inserted widgets
/// that shift every rank, so the failures are not near-misses:
///
/// | cell | positional guess | the truth |
/// |---|---|---|
/// | `taxpayer_first` | `c1_1[0]` — a CHECKBOX | `f1_14[0]` |
/// | `presidential_taxpayer` (a checkbox) | `f1_47[0]` — a MONEY field | `c1_6[0]` |
/// | `spouse_first` | `c1_4[0]` — a checkbox | `f1_17[0]` |
/// | `taxpayer_blind` | `c1_44[0]` — wrong PAGE | `c2_6[0]` |
///
/// A tool that emitted those would propose writing a filer's name into a checkbox and their blindness
/// into a money box. So identity cells stay refused, they are named in the report so the remaining work
/// is counted honestly, and this table is here to stop the shortcut being tried again.
pub fn port(
    prior: &[Cell],
    census: &[CensusEntry],
    old_shapes: &BTreeMap<String, Shape>,
    new_shapes: &BTreeMap<String, Shape>,
    old_labels: &BTreeMap<String, String>,
    new_labels: &BTreeMap<String, String>,
) -> Port {
    // The new revision indexed the way a port needs it: (label, band) -> the widgets that match.
    let mut by_label_band: BTreeMap<(String, String), Vec<&String>> = BTreeMap::new();
    for (fqn, label) in new_labels {
        if let Some(sh) = new_shapes.get(fqn) {
            by_label_band
                .entry((label.clone(), sh.band.clone()))
                .or_default()
                .push(fqn);
        }
    }

    let mut p = Port::default();
    for c in prior {
        // ★★★ The label lives in the key's `lineN` COMPONENT, not in the whole key. A map may nest a
        //     line's cells in a table — Schedule B's `[line7a]` holds the `yes`/`no` pair — so the
        //     qualified key is `line7a.yes` and the label is still `7a`. Reading the whole key gave
        //     `7a.yes` and reported ten cells as "the prior map disagrees with the prior form".
        let want = c.key.split('.').find_map(|seg| {
            seg.strip_prefix("line")
                .filter(|r| r.chars().next().is_some_and(|ch| ch.is_ascii_digit()))
        });
        // Only LINE cells have a printed label to carry. Everything else is the identity block's.
        let Some(want) = want else {
            p.refused.push(Refused {
                key: c.key.clone(),
                candidate: String::new(),
                class: Why::NotANumberedLine,
                why: "not a numbered line, so it has no printed label to carry: the identity block, \
                      the filing-status radios and the dependents grid are resolved from column x, \
                      /MaxLen, row order and the caption ABOVE the widget. No single fact checks them, \
                      and a wrong one prints an SSN in a date box. Read the form."
                    .into(),
            });
            continue;
        };
        match carry(
            &c.key,
            &c.fqn,
            c.on.as_deref(),
            want,
            old_shapes,
            new_shapes,
            old_labels,
            &by_label_band,
        ) {
            Ok(got) => {
                if let Some(m) = got.page_move {
                    p.page_moves.push(m);
                }
                p.emitted.push(Cell {
                    key: c.key.clone(),
                    fqn: got.fqn,
                    on: c.on.clone(),
                });
            }
            Err(r) => p.refused.push(r),
        }
    }
    // ★★★ **The census runs the IDENTICAL five checks, through a second door.** The two artifacts state
    //     the label differently — a cell in its key, an entry in its `line` field — so only the
    //     extraction differs; everything that decides is [`carry`], once. A second matcher here would be
    //     a second list to keep in step with the first, which is the failure this repo has measured most.
    //
    // ★★ The label is NOT taken by splitting a synthesised dotted key. One committed entry claims line
    //    `"1.1411-10(g)"` — a regulation cite — and a dotted key would have split it to `1` and carried a
    //    §1.1411-10(g) census entry onto line 1 of the new form. Hence [`CensusEntry::label`].
    for e in census {
        match carry(
            &format!("census.{}", e.line),
            &e.fqn,
            None,
            e.label(),
            old_shapes,
            new_shapes,
            old_labels,
            &by_label_band,
        ) {
            Ok(got) => {
                if let Some(m) = got.page_move {
                    p.page_moves.push(m);
                }
                p.census.push((got.fqn, e.clone()));
            }
            Err(r) => p.refused.push(r),
        }
    }
    refuse_collisions(&mut p);
    p
}

/// Withdraw every carry that shares its target widget with another, and refuse them all.
///
/// ★★★ **Found by widening [`same_column`], and it is a real form change rather than a tool artefact.**
/// TY2024 Schedule A gave line 16 three write-in description boxes — `f1_30` at x=331.8, `f1_31` and
/// `f1_32` both at x=115.2. TY2025 MERGED them into one 24pt-tall box, `f1_28` at x=122.4. Two prior
/// entries then resolved to that one widget and the renderer emitted the same TOML key twice, which TOML
/// either rejects or silently resolves last-wins.
///
/// ★★ **Both sides are withdrawn, not one.** Which prior entry now accounts for a merged box is a
/// reading of the new form — the three reasons were "write-in description", "second description line",
/// "third description line", and the merged box is none of those verbatim. Picking the first, the
/// narrowest, or the nearest would each be a guess dressed as a rule; the tool exists not to make it.
///
/// ★ It sweeps cells and census entries TOGETHER. A widget that is mapped *and* censused is the same
/// defect wearing two hats: the census would claim it encodes no decision while a line writes to it.
fn refuse_collisions(p: &mut Port) {
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    for fqn in p
        .emitted
        .iter()
        .map(|c| &c.fqn)
        .chain(p.census.iter().map(|(f, _)| f))
    {
        *seen.entry(fqn.clone()).or_default() += 1;
    }
    let dup: BTreeSet<String> = seen
        .into_iter()
        .filter(|(_, n)| *n > 1)
        .map(|(f, _)| f)
        .collect();
    if dup.is_empty() {
        return;
    }
    let why = |fqn: &str| {
        format!(
            "another prior entry resolves to {fqn} as well. The revision MERGED two boxes into one, \
             and which prior entry accounts for the survivor is a reading of the new form, not a rule."
        )
    };
    let mut withdrawn: Vec<Refused> = Vec::new();
    p.emitted.retain(|c| {
        if dup.contains(&c.fqn) {
            withdrawn.push(Refused {
                key: c.key.clone(),
                candidate: c.fqn.clone(),
                class: Why::TwoPriorCellsOneNewWidget,
                why: why(&c.fqn),
            });
            return false;
        }
        true
    });
    p.census.retain(|(fqn, e)| {
        if dup.contains(fqn) {
            withdrawn.push(Refused {
                key: format!("census.{}", e.line),
                candidate: fqn.clone(),
                class: Why::TwoPriorCellsOneNewWidget,
                why: why(fqn),
            });
            return false;
        }
        true
    });
    p.refused.append(&mut withdrawn);
}

/// A carried cell: where it lands on the new revision, and the page move if it moved.
pub struct Carried {
    pub fqn: String,
    pub page_move: Option<String>,
}

/// The five checks, applied to one `(key, prior fqn, on-state, printed label)` — the only place that
/// decides anything. Both a line cell and a census entry come through here.
#[allow(clippy::too_many_arguments)]
fn carry(
    key: &str,
    fqn: &str,
    on: Option<&str>,
    want_label: &str,
    old_shapes: &BTreeMap<String, Shape>,
    new_shapes: &BTreeMap<String, Shape>,
    old_labels: &BTreeMap<String, String>,
    by_label_band: &BTreeMap<(String, String), Vec<&String>>,
) -> Result<Carried, Refused> {
    let refuse = |class: Why, candidate: String, why: String| {
        Err(Refused {
            key: key.to_string(),
            candidate,
            class,
            why,
        })
    };
    let Some(was) = old_shapes.get(fqn) else {
        return refuse(
            Why::PriorFqnNotAWidget,
            String::new(),
            format!("{fqn} is not a widget of the prior template"),
        );
    };
    // ★ Sanity: the prior cell's own key must match the prior form's label, or the premise is gone.
    if let Some(printed_before) = old_labels.get(fqn) {
        if printed_before != "?" && !crate::label_reader::label_matches(want_label, printed_before)
        {
            return refuse(
                Why::PriorMapDisagreesWithPriorForm,
                String::new(),
                format!(
                    "the PRIOR form prints {printed_before:?} beside {fqn}, not {want_label:?}. The \
                     prior map disagrees with the prior form, so nothing can be carried from it."
                ),
            );
        }
    }
    // ★ The same one-way tolerance on the NEW side: a key may carry a sub-letter the form omits.
    let hits: Vec<&String> = by_label_band
        .iter()
        .filter(|((label, _), _)| crate::label_reader::label_matches(want_label, label))
        .flat_map(|(_, v)| v.iter().copied())
        .filter(|f| new_shapes.get(*f).is_some_and(|now| same_column(was, now)))
        .collect();
    // ★★ A `yes`/`no` pair shares one printed label in one column, and the ON-STATE is what tells
    //    them apart — the same fact the emitter writes. Narrowing by it before declaring ambiguity is
    //    not a guess: a candidate that does not declare the state this cell writes cannot be it.
    let hits: Vec<&String> = match on {
        Some(on) if hits.len() > 1 => {
            let narrowed: Vec<&String> = hits
                .iter()
                .copied()
                .filter(|f| new_shapes[*f].on_states.contains(on))
                .collect();
            if narrowed.is_empty() {
                hits
            } else {
                narrowed
            }
        }
        _ => hits,
    };
    match hits.as_slice() {
        [one] => {
            let now = &new_shapes[*one];
            if was.max_len.is_some() && was.max_len != now.max_len {
                return refuse(
                    Why::CapacityChanged,
                    (*one).clone(),
                    format!(
                        "/MaxLen {:?} → {:?}. The label and column agree and the CAPACITY does \
                         not; this is how a nine-digit SSN meets a two-character box.",
                        was.max_len, now.max_len
                    ),
                );
            }
            if let Some(on) = on {
                if !now.on_states.contains(on) {
                    return refuse(
                        Why::OnStateNotDeclared,
                        (*one).clone(),
                        format!(
                            "on-state {on:?} is not declared by that widget, which offers {:?}. An \
                             undeclared on-state renders as UNCHECKED while the write reports \
                             success.",
                            now.on_states
                        ),
                    );
                }
            }
            Ok(Carried {
                fqn: (*one).clone(),
                page_move: (was.page != now.page)
                    .then(|| format!("{key}: page {} → {}", was.page, now.page)),
            })
        }
        many if !many.is_empty() => refuse(
            Why::AmbiguousLabel,
            String::new(),
            format!(
                "{} widgets carry label {want_label:?} in the {} column: {many:?}. Ambiguous is \
                 not resolvable by rule.",
                many.len(),
                was.band
            ),
        ),
        _ => refuse(
            Why::LabelRenumberedOrGone,
            String::new(),
            format!(
                "the new revision prints no {want_label:?} in the {} column. Either the line was \
                 RENUMBERED — this revision spells AGI `11a` where the prior spells it `11`, and \
                 that cannot be inferred, only read — or it is gone.",
                was.band
            ),
        ),
    }
}

/// Render a port as TOML plus a refusal report.
#[must_use]
pub fn render(stem: &str, prior_tag: &str, p: &Port) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        "# CANDIDATE map for {stem}, derived from {prior_tag} by `xtask port-map`.\n\
         # ★ REVIEW EVERY LINE against the form. This tool proposes; it never decides.\n\
         # {} cell(s) and {} census entr(ies) passed all five checks; {} refused; {} page move(s).\n",
        p.emitted.len(),
        p.census.len(),
        p.refused.len(),
        p.page_moves.len()
    ));
    for m in &p.page_moves {
        s.push_str(&format!("# page move: {m}\n"));
    }
    for c in &p.emitted {
        match &c.on {
            Some(on) => s.push_str(&format!(
                "{} = {{ field = \"{}\", on = \"{on}\" }}\n",
                c.key, c.fqn
            )),
            None => s.push_str(&format!("{} = \"{}\"\n", c.key, c.fqn)),
        }
    }
    if !p.census.is_empty() {
        // ★ The inline table is re-emitted BYTE-EXACT. A census entry's `reason` is a decision someone
        //   wrote and a reviewer approved — the tool re-homes it onto a new widget and must not reword
        //   one character of it, because a reworded reason reads as a fresh reading of the new form.
        s.push_str("\n[census]\n");
        for (fqn, e) in &p.census {
            s.push_str(&format!("\"{fqn}\" = {}\n", e.rest));
        }
    }
    if !p.refused.is_empty() {
        s.push_str("\n# ── REFUSED — resolve each against the form, then add it by hand. ──\n");
        // ★ The CLASS is printed beside each refusal, because the count a reader takes away depends on
        //   it: 110 of these are the tool declining a grid by design and 28 are real reading. A report
        //   that prints only prose invites the two to be added together — which is exactly what I did on
        //   first reading this output.
        for r in &p.refused {
            s.push_str(&format!(
                "# {} -> {}   [{}]\n#   {}\n",
                r.key,
                r.candidate,
                r.class.as_str(),
                r.why
            ));
        }
        let mut by: BTreeMap<&str, usize> = BTreeMap::new();
        for r in &p.refused {
            *by.entry(r.class.as_str()).or_default() += 1;
        }
        s.push_str("#\n# refusals by class:\n");
        for (c, n) in &by {
            s.push_str(&format!("#   {n:4}  {c}\n"));
        }
    }
    s
}

/// `port-map <stem> <prior-tag> <new-tag>`.
pub fn run(stem: &str, prior_tag: &str, new_tag: &str) -> Result<(), String> {
    let root = crate::form_geometry::repo_root();
    let year_of = |tag: &str| tag.split('-').next().unwrap_or(tag).to_string();
    let map_path = root.join(format!(
        "crates/btctax-forms/forms/{}/{stem}.map.toml",
        year_of(prior_tag)
    ));
    let prior_text =
        std::fs::read_to_string(&map_path).map_err(|e| format!("{}: {e}", map_path.display()))?;
    // ★★★ Two homes, and the tag says which. A BUNDLED year has a template the crate ships; an ARCHIVE
    //     tag — `2026-DRAFT` — has only `design/forms/<year>/<stem>--<tag>.pdf`, because a draft is
    //     EVIDENCE and is never bundled. Resolving both is what lets this tool forecast January's work
    //     from the drafts months before a final exists.
    let pdf = |tag: &str| -> Result<std::path::PathBuf, String> {
        let bundled = root.join(format!(
            "crates/btctax-forms/forms/{}/{stem}.pdf",
            year_of(tag)
        ));
        if bundled.exists() {
            return Ok(bundled);
        }
        let archived = root.join(format!("design/forms/{}/{stem}--{tag}.pdf", year_of(tag)));
        if archived.exists() {
            return Ok(archived);
        }
        Err(format!(
            "no template for {stem}--{tag}: neither {} nor {}",
            bundled.display(),
            archived.display()
        ))
    };
    let (old_sh, new_sh) = (shapes(&pdf(prior_tag)?)?, shapes(&pdf(new_tag)?)?);
    let old_labels = crate::label_reader::label_join(&format!("{stem}--{prior_tag}"))?;
    let new_labels = crate::label_reader::label_join(&format!("{stem}--{new_tag}"))?;
    let p = port(
        &cells_of(&prior_text),
        &census_of(&prior_text),
        &old_sh,
        &new_sh,
        &old_labels,
        &new_labels,
    );
    print!("{}", render(stem, prior_tag, &p));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ★★★ **THE KNOWN ANSWER: the tool must agree with the one port done by hand.**
    ///
    /// f1040 TY2024 → TY2025 was ported cell by cell against the form on 2026-09-20/21. That makes it a
    /// known answer rather than a golden the tool could regenerate to match itself, which is the whole
    /// reason this test is possible and the reason it was worth doing the hand port first.
    ///
    /// The bar is **zero disagreements on the cells it emits** — not full coverage. A tool that emitted
    /// everything would have to guess at the four cells below, and guessing is what it exists not to do.
    #[test]
    fn the_f1040_ty2025_port_reproduces_the_hand_written_map() {
        assert_disagreements_are_zero("f1040", 35);
        let p = ported("f1040");
        // ★ And it must NOT have invented the four a human had to read: three renumbered lines
        //   (11 → 11a, 12 → 12e, 13 → 13a) and one line TY2025 added outright (6d).
        for key in ["line11a", "line12e", "line13a", "line6d"] {
            assert!(
                !p.emitted.iter().any(|c| c.key == key),
                "{key} must be REFUSED — a renumbered or new line cannot be inferred from last year's \
                 map, and emitting it would be the tool deciding"
            );
        }
        // ★ The page move that broke the descent oracle is REPORTED, not swallowed.
        assert!(
            p.page_moves.iter().any(|m| m.starts_with("line14:")),
            "the page move for line14 must be reported: {:?}",
            p.page_moves
        );
    }

    /// Run the tool for one stem, TY2024 → TY2025.
    fn ported(stem: &str) -> Port {
        let root = crate::form_geometry::repo_root();
        let map = |y: i32| {
            std::fs::read_to_string(
                root.join(format!("crates/btctax-forms/forms/{y}/{stem}.map.toml")),
            )
            .unwrap_or_else(|e| panic!("{stem} {y} map: {e}"))
        };
        let pdf = |y: i32| root.join(format!("crates/btctax-forms/forms/{y}/{stem}.pdf"));
        let (o, n) = (shapes(&pdf(2024)).unwrap(), shapes(&pdf(2025)).unwrap());
        let ol = crate::label_reader::label_join(&format!("{stem}--2024")).unwrap_or_default();
        let nl = crate::label_reader::label_join(&format!("{stem}--2025")).unwrap_or_default();
        port(
            &cells_of(&map(2024)),
            &census_of(&map(2024)),
            &o,
            &n,
            &ol,
            &nl,
        )
    }

    /// Every cell the tool emits for `stem` must match the committed TY2025 map, with a floor so a tool
    /// that stopped resolving cannot pass by emitting nothing.
    /// Returns `(cells that agree, census entries that agree)`.
    fn assert_disagreements_are_zero(stem: &str, floor: usize) -> (usize, usize) {
        let root = crate::form_geometry::repo_root();
        let held = std::fs::read_to_string(
            root.join(format!("crates/btctax-forms/forms/2025/{stem}.map.toml")),
        )
        .expect("committed TY2025 map");
        let hand: BTreeMap<String, String> = cells_of(&held)
            .into_iter()
            .map(|c| (c.key, c.fqn))
            .collect();
        let p = ported(stem);
        let census = assert_census_agrees(stem, &held, &hand, &p);
        assert!(
            p.emitted.len() >= floor,
            "{stem}: only {} cell(s) emitted against a floor of {floor} — the tool has stopped \
             resolving and this check is measuring almost nothing",
            p.emitted.len()
        );
        let wrong: Vec<String> = p
            .emitted
            .iter()
            .filter_map(|c| match hand.get(&c.key) {
                Some(h) if *h == c.fqn => None,
                Some(h) => Some(format!(
                    "{}/{}: tool {} vs committed {h}",
                    stem, c.key, c.fqn
                )),
                None => Some(format!(
                    "{}/{}: tool {} vs committed ABSENT",
                    stem, c.key, c.fqn
                )),
            })
            .collect();
        assert!(
            wrong.is_empty(),
            "{} disagreement(s) between the tool and a committed map. One of the two is wrong, and a \
             filed return depends on which:\n  {}",
            wrong.len(),
            wrong.join("\n  ")
        );
        (p.emitted.len(), census)
    }

    /// Every census entry the tool carries must land where the committed TY2025 map put that decision.
    ///
    /// ★★★ **"The committed map does not census it" is NOT automatically a disagreement.** A line that
    /// became MODELLED between revisions moves from the census to a real cell — Schedule A's TY2025 map
    /// censuses 6 widgets where TY2024 censused 9, and three of those became mapped. So the bar is that
    /// the widget is *accounted for* by the committed map either way; a carry onto a widget the committed
    /// map never mentions at all is the real defect, because then nothing accounts for it.
    ///
    /// ★★ This is the [`CensusEntry`] carry's known answer, and it is a larger population than the cell
    /// carry's: measured over the 11 forms whose maps census on both sides, **173** entries carry with
    /// **zero** disagreements, against 197 cells.
    /// TY2025 maps where the tool can carry census entries and the committed map records NONE.
    ///
    /// ★★★ **A pinned GAP, not an exemption.** f1040's TY2025 map was ported by mapping its cells and
    /// never carrying its census: TY2024 accounts for 39 widgets with a reason each, TY2025 accounts for
    /// none, and the field register's `(2025, "f1040", 96)` row absorbs the residue. The register is
    /// honest about the COUNT and silent about the CAUSE — that decisions already made were available and
    /// not brought forward. This names the cause.
    ///
    /// ★★ **"No census" is not by itself a defect, which is why the condition is narrow.** f8889 maps
    /// every one of its 24 widgets and needs no census at all; three more — f8949, `schedule_d`, `schedule_se` — have TY2024
    /// censuses (6, 6, 13 entries) that this tool carries **0** of, so their gap is register-recorded but
    /// not yet demonstrable here. The trigger is therefore *the tool
    /// carried entries AND the committed map has none*, which is the only case where the comparison had
    /// something to say and found nothing to compare against.
    const CENSUS_NOT_PORTED: [&str; 2] = ["f1040", "f8283"];

    fn assert_census_agrees(
        stem: &str,
        held: &str,
        mapped: &BTreeMap<String, String>,
        p: &Port,
    ) -> usize {
        if census_of(held).is_empty() {
            assert!(
                p.census.is_empty() || CENSUS_NOT_PORTED.contains(&stem),
                "{stem}: the tool carries {} census entr(ies) and the committed TY2025 map records \
                 none, so that many widgets are accounted for on TY2024 and unaccounted on TY2025 — \
                 with the field register absorbing the count and naming no cause. Port the census, or \
                 add {stem} to CENSUS_NOT_PORTED with the reason.",
                p.census.len()
            );
            return 0;
        }
        assert!(
            !CENSUS_NOT_PORTED.contains(&stem),
            "{stem} now HAS a committed census, so it must leave CENSUS_NOT_PORTED — and the field \
             register's (2025, {stem:?}, n) row should shrink by what the census now accounts for"
        );
        let held_census: BTreeMap<String, String> = census_of(held)
            .into_iter()
            .map(|e| (e.fqn, e.line))
            .collect();
        let mapped_fqns: BTreeSet<&String> = mapped.values().collect();
        let wrong: Vec<String> = p
            .census
            .iter()
            .filter_map(|(fqn, e)| match held_census.get(fqn) {
                Some(line) if *line == e.line => None,
                Some(line) => Some(format!(
                    "{stem}/{fqn}: tool claims line {:?}, committed census says {line:?}",
                    e.line
                )),
                None if mapped_fqns.contains(fqn) => None,
                None => Some(format!(
                    "{stem}/{fqn}: tool censuses it as line {:?}; the committed map neither censuses \
                     NOR maps it, so nothing accounts for that widget",
                    e.line
                )),
            })
            .collect();
        assert!(
            wrong.is_empty(),
            "{} census disagreement(s) between the tool and a committed map:\n  {}",
            wrong.len(),
            wrong.join("\n  ")
        );
        p.census.len()
    }

    /// ★★★ **SEVENTEEN known answers, not one.** Every form bundled for BOTH years was ported by
    /// somebody against its form and is held by the map gates, so each is an independent check on the
    /// tool. Validating against one form proves the tool agrees with me; validating against seventeen
    /// proves it agrees with the repo.
    ///
    /// ★ Stems are DERIVED from the intersection of the two years' bundled maps, so a form bundled later
    /// joins this check with no edit — and a form that stops being bundled cannot silently leave it.
    #[test]
    fn the_tool_agrees_with_every_committed_ty2025_map() {
        let root = crate::form_geometry::repo_root();
        let stems = |y: i32| -> BTreeSet<String> {
            std::fs::read_dir(root.join(format!("crates/btctax-forms/forms/{y}")))
                .expect("forms dir")
                .flatten()
                .filter_map(|e| {
                    e.file_name()
                        .to_str()?
                        .strip_suffix(".map.toml")
                        .map(str::to_string)
                })
                .collect()
        };
        let both: Vec<String> = stems(2024).intersection(&stems(2025)).cloned().collect();
        assert!(
            both.len() >= 15,
            "only {} form(s) are bundled for both years — this check has lost its population",
            both.len()
        );
        let mut total = 0usize;
        let mut entries = 0usize;
        let mut covered = 0usize;
        for stem in &both {
            // A stem with no bundled template on one side cannot be shaped; skipped and counted.
            let pdf = |y: i32| root.join(format!("crates/btctax-forms/forms/{y}/{stem}.pdf"));
            if !pdf(2024).exists() || !pdf(2025).exists() {
                continue;
            }
            // ★ ONE port per stem. An earlier cut called `ported` twice per form — once for the cells,
            //   once for the census — which doubled 17 PDF parses to measure the same thing.
            let (cells, census) = assert_disagreements_are_zero(stem, 0);
            total += cells;
            entries += census;
            covered += 1;
        }
        // ★ A FLOOR on the census population too, for the same reason the cell floor exists: the carry
        //   could regress to emitting nothing and every `filter_map` above would be vacuously satisfied.
        assert!(
            entries >= 150,
            "only {entries} census entr(ies) carried — measured at 173, so the census carry has \
             regressed and its zero-disagreement result above is near-vacuous"
        );
        assert!(
            covered >= 12 && total >= 150,
            "only {covered} form(s) and {total} cell(s) were actually compared — a population this \
             small means the intersection or the templates moved, and the check is near-vacuous"
        );
        eprintln!(
            "  port-map: {covered} form(s), {total} cell(s) and {entries} census entr(ies) agree \
             with the committed maps"
        );
    }

    /// ★★★ **EVERY f1040 IDENTITY CELL IS REFUSED, and the measurement behind that is in `port`'s docs.**
    ///
    /// The positional shortcut — carry by `(page, column x, rank)` — reproduces 5 of these 29 and gets 12
    /// WRONG, several by proposing a text value for a checkbox or a checkbox for a money field. This pins
    /// the refusal so a future reader who finds 110 grid/identity refusals "obviously automatable" has to
    /// delete an assertion that says why it is not.
    #[test]
    fn no_identity_cell_is_ever_carried() {
        let p = ported("f1040");
        let carried: Vec<&str> = p
            .emitted
            .iter()
            .filter(|c| !c.key.starts_with("line"))
            .map(|c| c.key.as_str())
            .collect();
        assert!(
            carried.is_empty(),
            "the tool carried {} identity cell(s): {carried:?}. Identity cells have no printed label, \
             and the positional alternative gets 12 of 29 wrong — including a filer's NAME into a \
             checkbox. Refusing them is the design, not a gap.",
            carried.len()
        );
        // …and they are REFUSED rather than silently dropped, so the remaining work stays counted.
        let named = p
            .refused
            .iter()
            .filter(|r| r.class == Why::NotANumberedLine)
            .count();
        assert!(
            named >= 25,
            "only {named} identity cell(s) were reported as refused — a dropped cell is work nobody \
             can see"
        );
    }

    /// ★★★ **EVERY CHECKBOX CELL IN EVERY COMMITTED MAP WRITES AN ON-STATE ITS WIDGET DECLARES.**
    ///
    /// An undeclared on-state does not fail: `apply_writes` reports success and the box renders as
    /// **unchecked**. So a wrong on-state is an answer the filer gave and the paper does not carry —
    /// invisible in the emitted PDF, invisible to both oracles, and invisible to any test that checks a
    /// figure.
    ///
    /// ★★★ **Nothing checked this across the maps until 2026-09-21.** `button_on_states` had exactly three
    /// callers: one form's dependents grid and two TY2025 f1040 cells. Measured: setting Form 8283 line
    /// 5a's `no` to on-state `1` — which that widget does not declare, it declares `2` — red NOTHING
    /// across the whole suite, and a filer answering "no" to a restriction question would have filed a
    /// blank.
    ///
    /// ★★ Also asserted: the two halves of a yes/no pair must write DIFFERENT on-states. Both writing the
    /// same one is declared-but-wrong, which the check above cannot see.
    #[test]
    fn every_checkbox_cell_writes_an_on_state_its_widget_declares() {
        let root = crate::form_geometry::repo_root();
        let mut checked = 0usize;
        let mut maps = 0usize;
        let mut bad: Vec<String> = Vec::new();
        let mut pairs: BTreeMap<(String, String), Vec<(String, String)>> = BTreeMap::new();
        for year in std::fs::read_dir(root.join("crates/btctax-forms/forms"))
            .expect("forms")
            .flatten()
        {
            let y = year.file_name().to_string_lossy().to_string();
            if y.len() != 4 || !y.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }
            for e in std::fs::read_dir(year.path()).expect("year dir").flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                let Some(stem) = name.strip_suffix(".map.toml") else {
                    continue;
                };
                let pdf = year.path().join(format!("{stem}.pdf"));
                if !pdf.exists() {
                    continue; // a map whose template is not bundled cannot be shaped; counted by absence
                }
                let Ok(sh) = shapes(&pdf) else { continue };
                maps += 1;
                for c in cells_of(&std::fs::read_to_string(e.path()).expect("map")) {
                    let Some(on) = &c.on else { continue };
                    checked += 1;
                    match sh.get(&c.fqn) {
                        None => {
                            bad.push(format!("{y}/{stem} {}: {} is not a widget", c.key, c.fqn))
                        }
                        Some(shape) if !shape.on_states.contains(on) => bad.push(format!(
                            "{y}/{stem} {}: writes on-state {on:?}, widget declares {:?} — the box \
                             would render UNCHECKED and the answer would be lost",
                            c.key, shape.on_states
                        )),
                        Some(_) => {}
                    }
                    // Group yes/no siblings by their table so the pair can be compared.
                    if let Some((table, role)) = c.key.rsplit_once('.') {
                        if role == "yes" || role == "no" {
                            pairs
                                .entry((format!("{y}/{stem}"), table.to_string()))
                                .or_default()
                                .push((role.to_string(), on.clone()));
                        }
                    }
                }
            }
        }
        for ((map, table), v) in &pairs {
            if v.len() == 2 && v[0].1 == v[1].1 {
                bad.push(format!(
                    "{map} [{table}]: yes and no both write on-state {:?}. One of them is \
                     declared-but-wrong, and the box it belongs to would carry the other answer.",
                    v[0].1
                ));
            }
        }
        assert!(
            maps >= 25 && checked >= 60,
            "only {maps} map(s) and {checked} checkbox cell(s) were examined — the population moved and \
             this gate is near-vacuous"
        );
        assert!(
            bad.is_empty(),
            "{} checkbox cell(s) would render UNCHECKED or carry the wrong answer:\n  {}",
            bad.len(),
            bad.join("\n  ")
        );
        eprintln!("  on-states: {checked} checkbox cell(s) across {maps} map(s) all declared");
    }

    /// ★★★ **JANUARY'S QUEUE, FORECAST FROM THE DRAFTS — and every refusal in a NAMED class.**
    ///
    /// The tool's real job is TY2025 → TY2026, in January, for the year that is filed. Eleven forms have
    /// both a TY2025 map and an archived TY2026 **draft**, so the size of that job is knowable now. As
    /// measured 2026-09-21:
    ///
    /// | | cells |
    /// |---|---|
    /// | carry automatically | **155** |
    /// | grid or identity, declined by design | 110 |
    /// | **label renumbered — a human must read the form** | **28** |
    /// | change PAGE (reported, still carried) | 18 |
    ///
    /// ★★ **A draft is EVIDENCE ONLY and nothing here is transcribed as authority.** This forecasts
    /// effort; it does not produce a map. The counts will move when finals land, which is the point —
    /// so this test asserts the SHAPE (it runs, the classes are known, the floors hold) and not the
    /// numbers, which would make a January final into a failing test.
    ///
    /// ★★★ **The load-bearing assertion is that NO refusal is unclassified.** A tool that grows a new
    /// failure mode and reports it in prose nobody counted is how "138 need reading" gets repeated when
    /// 110 of them were the tool declining a grid — which is exactly the mistake this decomposition
    /// caught in its own first reading.
    #[test]
    fn the_january_queue_is_forecastable_and_every_refusal_is_classified() {
        let root = crate::form_geometry::repo_root();
        let stems_2025: BTreeSet<String> =
            std::fs::read_dir(root.join("crates/btctax-forms/forms/2025"))
                .expect("forms/2025")
                .flatten()
                .filter_map(|e| {
                    e.file_name()
                        .to_str()?
                        .strip_suffix(".map.toml")
                        .map(str::to_string)
                })
                .collect();
        let drafts: BTreeSet<String> = std::fs::read_dir(root.join("design/forms/2026"))
            .expect("design/forms/2026")
            .flatten()
            .filter_map(|e| {
                let n = e.file_name().to_str()?.to_string();
                n.strip_suffix("--2026-DRAFT.pdf")
                    .map(str::to_string)
                    .filter(|_| n.ends_with(".pdf"))
            })
            .collect();
        let both: Vec<&String> = stems_2025.intersection(&drafts).collect();
        assert!(
            both.len() >= 10,
            "only {} form(s) have both a TY2025 map and a TY2026 draft — the forecast has lost its \
             population",
            both.len()
        );

        let (mut carried, mut pages) = (0usize, 0usize);
        let mut by_class: BTreeMap<&'static str, usize> = BTreeMap::new();
        let mut unclassified: Vec<String> = Vec::new();
        for stem in &both {
            let map_2025 = std::fs::read_to_string(
                root.join(format!("crates/btctax-forms/forms/2025/{stem}.map.toml")),
            )
            .expect("TY2025 map");
            let old_pdf = root.join(format!("crates/btctax-forms/forms/2025/{stem}.pdf"));
            let new_pdf = root.join(format!("design/forms/2026/{stem}--2026-DRAFT.pdf"));
            if !old_pdf.exists() || !new_pdf.exists() {
                continue;
            }
            let (o, n) = (shapes(&old_pdf).unwrap(), shapes(&new_pdf).unwrap());
            let ol = crate::label_reader::label_join(&format!("{stem}--2025")).unwrap_or_default();
            let nl =
                crate::label_reader::label_join(&format!("{stem}--2026-DRAFT")).unwrap_or_default();
            let p = port(
                &cells_of(&map_2025),
                &census_of(&map_2025),
                &o,
                &n,
                &ol,
                &nl,
            );
            carried += p.emitted.len();
            pages += p.page_moves.len();
            for r in &p.refused {
                // ★ The class is a FIELD, so this cannot mis-sort on a phrase — and `Why` is `_`-free
                //   at `as_str`, so a new reason is a build error until someone names its bucket.
                *by_class.entry(r.class.as_str()).or_default() += 1;
                let _ = &mut unclassified;
            }
        }
        // ★★★ Every `Why` variant that OCCURS is counted by construction. What can still go wrong is a
        //     variant nobody ever produces — a class that looks covered and is dead — so the buckets that
        //     must be non-empty are named, and the grid/human split is asserted rather than described.
        assert!(unclassified.is_empty(), "unreachable: the class is a field");
        let of = |v: Why| by_class.get(v.as_str()).copied().unwrap_or(0);
        assert!(
            of(Why::NotANumberedLine) >= 50,
            "the grid/identity bucket holds {} — Schedule B alone has ~66 payer rows, so a small \
             number here means the reader or the maps moved: {by_class:?}",
            of(Why::NotANumberedLine)
        );
        assert!(
            carried >= 120,
            "only {carried} cell(s) carry across the whole queue — the tool has stopped resolving"
        );
        let human = by_class
            .iter()
            .filter(|(c, _)| c.contains("human must read"))
            .map(|(_, n)| *n)
            .sum::<usize>();
        assert!(
            human >= 1,
            "no form needs a human to read it, which cannot be true of a year that renumbered lines: \
             {by_class:?}"
        );
        eprintln!("  port-map January forecast: {carried} carry, {pages} change page");
        for (c, n) in &by_class {
            eprintln!("    {n:4}  {c}");
        }
    }

    /// A planted shape. ★ `x` is DERIVED from the band rather than taken as a parameter, so a fixture
    /// cannot plant a band and an x that contradict each other — `same_column` reads both.
    fn shape(page: u32, band: &str, max_len: Option<usize>, on: &[&str]) -> Shape {
        let x = band
            .strip_prefix("other(")
            .and_then(|r| r.trim_end_matches(')').parse().ok())
            .or_else(|| {
                BANDS
                    .iter()
                    .find(|(n, ..)| *n == band)
                    .map(|(_, lo, _)| *lo)
            })
            .unwrap_or(0.0);
        Shape {
            page,
            band: band.to_string(),
            x,
            max_len,
            on_states: on.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    /// One prior cell, one candidate widget, and whatever shapes/labels the caller wants to plant.
    fn one(
        key: &str,
        on: Option<&str>,
        was: Shape,
        now: Shape,
        printed_before: &str,
        printed_after: &str,
    ) -> Port {
        let prior = vec![Cell {
            key: key.to_string(),
            fqn: "topmostSubform[0].Page1[0].old[0]".into(),
            on: on.map(str::to_string),
        }];
        let mut olds = BTreeMap::new();
        olds.insert("topmostSubform[0].Page1[0].old[0]".to_string(), was);
        let mut news = BTreeMap::new();
        news.insert("topmostSubform[0].Page1[0].new[0]".to_string(), now);
        let mut ol = BTreeMap::new();
        ol.insert(
            "topmostSubform[0].Page1[0].old[0]".to_string(),
            printed_before.to_string(),
        );
        let mut nl = BTreeMap::new();
        nl.insert(
            "topmostSubform[0].Page1[0].new[0]".to_string(),
            printed_after.to_string(),
        );
        port(&prior, &[], &olds, &news, &ol, &nl)
    }

    /// Two prior census entries, one candidate widget, and the labels/shapes to make both match.
    fn two_onto_one(prior_x: (f64, f64), new_x: f64, line: &str) -> Port {
        let census = vec![
            CensusEntry {
                fqn: "form1[0].Page1[0].a[0]".into(),
                line: line.into(),
                rest: r#"{ line = "16", rule = "unmodeled", reason = "first description line" }"#
                    .into(),
            },
            CensusEntry {
                fqn: "form1[0].Page1[0].b[0]".into(),
                line: line.into(),
                rest: r#"{ line = "16", rule = "unmodeled", reason = "second description line" }"#
                    .into(),
            },
        ];
        let sh = |x: f64| shape(1, &format!("other({x:.0})"), None, &[]);
        let olds: BTreeMap<String, Shape> = [
            ("form1[0].Page1[0].a[0]".to_string(), sh(prior_x.0)),
            ("form1[0].Page1[0].b[0]".to_string(), sh(prior_x.1)),
        ]
        .into();
        let news: BTreeMap<String, Shape> =
            [("form1[0].Page1[0].merged[0]".to_string(), sh(new_x))].into();
        let ol: BTreeMap<String, String> = [
            ("form1[0].Page1[0].a[0]".to_string(), line.to_string()),
            ("form1[0].Page1[0].b[0]".to_string(), line.to_string()),
        ]
        .into();
        let nl: BTreeMap<String, String> =
            [("form1[0].Page1[0].merged[0]".to_string(), line.to_string())].into();
        port(&[], &census, &olds, &news, &ol, &nl)
    }

    /// ★★★ **B1 — a MERGED box must refuse both claimants, and the plant is the real Schedule A change.**
    ///
    /// TY2024 line 16 had three write-in description boxes (`f1_30` x=331.8, `f1_31` and `f1_32` both
    /// x=115.2); TY2025 merged them into one 24pt-tall `f1_28` at x=122.4. The two x=115.2 entries both
    /// land on it within the 12pt tolerance, and the renderer would emit the same TOML key twice — which
    /// TOML either rejects or resolves last-wins, silently keeping one reason and dropping the other.
    ///
    /// ★★ The assertion is a REFUSAL of BOTH, not a note and not a survivor. A checker that kept the
    /// first claimant would be choosing which of two approved reasons accounts for a box on a form nobody
    /// has read, and it would pass a test that only asserted "no duplicate keys".
    #[test]
    fn a_merged_box_refuses_every_claimant_rather_than_picking_one() {
        let p = two_onto_one((115.2, 115.2), 122.4, "16");
        assert!(
            p.census.is_empty() && p.emitted.is_empty(),
            "a merged box was carried anyway: {:?}",
            p.census
        );
        assert_eq!(
            p.refused.len(),
            2,
            "both claimants must be refused: {:?}",
            p.refused
        );
        for r in &p.refused {
            assert_eq!(
                r.class,
                Why::TwoPriorCellsOneNewWidget,
                "a merged box must be refused AS a merge, not as something else: {r:?}"
            );
        }
        // ★ And the control: the same two entries in genuinely DIFFERENT columns still both carry, so the
        //   check above is not passing by refusing everything.
        let far = two_onto_one((115.2, 331.8), 122.4, "16");
        assert_eq!(
            far.census.len(),
            1,
            "only the near entry should resolve; the far one is 209pt away: {:?}",
            far.census
        );
    }

    /// ★★★ **B1 — the column tolerance must have a CEILING, or it is not a tolerance.**
    ///
    /// A window that admits everything is the same instrument as no window at all, and it reads as
    /// deliberate in the source. This plants a widget just outside 12pt and asserts a refusal, so raising
    /// `OTHER_COLUMN_TOLERANCE_PT` to swallow a hard case reds here.
    #[test]
    fn a_column_that_moved_further_than_the_window_is_refused() {
        let near = one(
            "line19c",
            None,
            shape(1, "other(335)", None, &[]),
            shape(1, "other(338)", None, &[]),
            "19c",
            "19c",
        );
        assert_eq!(
            near.emitted.len(),
            1,
            "a 3.6pt drift is inside the measured 7.2pt maximum and must carry: {:?}",
            near.refused
        );
        let far = one(
            "line19c",
            None,
            shape(1, "other(335)", None, &[]),
            shape(1, "other(360)", None, &[]),
            "19c",
            "19c",
        );
        assert!(
            far.emitted.is_empty(),
            "a 25pt jump is nearly the 35.2pt gap between two DISTINCT columns and must not carry"
        );
        assert_eq!(far.refused[0].class, Why::LabelRenumberedOrGone);
    }

    /// ★★★ **B1 — a census entry claiming a REGULATION must never be read as a line number.**
    ///
    /// Schedule 1's committed census carries `line = "1.1411-10(g)"`. The first design synthesised a
    /// dotted key (`census.1.1411-10(g)`) and reused the cell path's `key.split('.')` label extraction —
    /// which yields `1`, and would have carried a §1.1411-10(g) census entry onto **line 1 of the new
    /// form**, attaching an approved reason to the wrong box. [`CensusEntry::label`] takes the first
    /// whitespace token instead, so the claim stays `1.1411-10(g)` and refuses.
    #[test]
    fn a_regulation_cite_is_never_carried_as_line_one() {
        assert_eq!(
            CensusEntry {
                fqn: String::new(),
                line: "1.1411-10(g)".into(),
                rest: String::new(),
            }
            .label(),
            "1.1411-10(g)",
            "the label is the whole first token — splitting on '.' yields \"1\""
        );
        let census = vec![CensusEntry {
            fqn: "form1[0].Page1[0].reg[0]".into(),
            line: "1.1411-10(g)".into(),
            rest: r#"{ line = "1.1411-10(g)", rule = "unmodeled", reason = "planted" }"#.into(),
        }];
        let olds: BTreeMap<String, Shape> = [(
            "form1[0].Page1[0].reg[0]".to_string(),
            shape(1, "AMOUNT", None, &[]),
        )]
        .into();
        // The new revision prints a line "1" in that column — the box the bug would have chosen.
        let news: BTreeMap<String, Shape> = [(
            "form1[0].Page1[0].line1[0]".to_string(),
            shape(1, "AMOUNT", None, &[]),
        )]
        .into();
        let ol: BTreeMap<String, String> = [(
            "form1[0].Page1[0].reg[0]".to_string(),
            "1.1411-10(g)".to_string(),
        )]
        .into();
        let nl: BTreeMap<String, String> =
            [("form1[0].Page1[0].line1[0]".to_string(), "1".to_string())].into();
        let p = port(&[], &census, &olds, &news, &ol, &nl);
        assert!(
            p.census.is_empty(),
            "a regulation cite was carried onto {:?}",
            p.census
        );
        assert_eq!(p.refused[0].class, Why::LabelRenumberedOrGone);
    }

    /// ★★★ **B1 — `census_of` reads the `[census]` table and nothing else.**
    ///
    /// The parser is scoped by table because `[census]` is the only one of the 22 committed maps' tables
    /// whose keys are quoted FQNs. This plants an FQN-keyed line under a DIFFERENT table and asserts it is
    /// not swept up — the failure mode being a future table of that shape silently becoming census
    /// decisions, which are the entries that say "this widget encodes nothing".
    #[test]
    fn census_of_reads_only_the_census_table() {
        let text = r#"
line1 = "topmostSubform[0].Page1[0].f1_01[0]"

[somewhere_else]
"topmostSubform[0].Page1[0].decoy[0]" = { line = "99", rule = "unmodeled", reason = "planted" }

[census]
"topmostSubform[0].Page1[0].real[0]" = { line = "8n", rule = "unmodeled", reason = "kept" }

[after]
"topmostSubform[0].Page1[0].after[0]" = { line = "98", rule = "unmodeled", reason = "planted" }
"#;
        let got = census_of(text);
        assert_eq!(
            got.iter().map(|e| e.line.as_str()).collect::<Vec<_>>(),
            ["8n"],
            "only the [census] table's entries are census decisions: {got:?}"
        );
        assert_eq!(
            got[0].rest,
            r#"{ line = "8n", rule = "unmodeled", reason = "kept" }"#
        );
    }

    /// ★★★ **B1 — each of the four traps the hand port actually hit, planted.**
    ///
    /// Every one is a real measurement from f1040 TY2024 → TY2025, not an invented case: the `/MaxLen`
    /// shrink is TY2024's `taxpayer_ssn` FQN meeting TY2025's two-character date box; the undeclared
    /// on-state is the filing-status radio renumbered `mfs 4 → 3`; the column move is a sub-line cell
    /// pointed at its own pair's amount widget; the renumbering is `11 → 11a`.
    #[test]
    fn every_trap_the_hand_port_hit_is_refused() {
        // (1) capacity: nine digits into a two-character box.
        let p = one(
            "line1a",
            None,
            shape(1, "AMOUNT", Some(9), &[]),
            shape(1, "AMOUNT", Some(2), &[]),
            "1a",
            "1a",
        );
        assert!(p.emitted.is_empty(), "a /MaxLen shrink must be refused");
        assert!(
            p.refused[0].why.contains("/MaxLen"),
            "and named as capacity: {}",
            p.refused[0].why
        );

        // (2) an on-state the new widget does not declare — the filing-status renumbering.
        let p = one(
            "line1a",
            Some("4"),
            shape(1, "AMOUNT", None, &["4"]),
            shape(1, "AMOUNT", None, &["3"]),
            "1a",
            "1a",
        );
        assert!(
            p.emitted.is_empty(),
            "an undeclared on-state must be refused"
        );
        assert!(
            p.refused[0].why.contains("on-state"),
            "and named as such: {}",
            p.refused[0].why
        );

        // (3) the column moved — the trap the label check structurally cannot see.
        let p = one(
            "line1a",
            None,
            shape(1, "AMOUNT", None, &[]),
            shape(1, "MID", None, &[]),
            "1a",
            "1a",
        );
        assert!(p.emitted.is_empty(), "a column move must be refused");

        // (4) the line was RENUMBERED: nothing carries label `11` on the new revision.
        let p = one(
            "line11",
            None,
            shape(1, "AMOUNT", None, &[]),
            shape(1, "AMOUNT", None, &[]),
            "11",
            "11a",
        );
        assert!(p.emitted.is_empty(), "a renumbered line must be refused");
        assert!(
            p.refused[0].why.contains("RENUMBERED"),
            "and say so, because that is the one case a human must read: {}",
            p.refused[0].why
        );

        // …and the control: same label, same column, same capacity ⇒ EMITTED. Without this the four
        // above are satisfied by a tool that refuses everything.
        let p = one(
            "line1a",
            None,
            shape(1, "AMOUNT", Some(9), &[]),
            shape(1, "AMOUNT", Some(9), &[]),
            "1a",
            "1a",
        );
        assert_eq!(
            p.emitted.len(),
            1,
            "an unchanged cell must carry: {:?}",
            p.refused
        );
        assert!(p.page_moves.is_empty());

        // A PAGE move carries, and is reported — the emitter derives descent groups from the page.
        let p = one(
            "line1a",
            None,
            shape(1, "AMOUNT", None, &[]),
            shape(2, "AMOUNT", None, &[]),
            "1a",
            "1a",
        );
        assert_eq!(p.emitted.len(), 1, "a page move is not a refusal");
        assert_eq!(p.page_moves.len(), 1, "but it must be reported");
    }

    /// ★★★ **The one-way label tolerance is the SHARED predicate's, not a second opinion.**
    ///
    /// The first version of this tool compared labels strictly and refused TY2024's `line7a` against a
    /// printed `7` — a false positive against a convention `label_reader::label_matches` already
    /// encodes. This pins both directions so the two can never drift apart.
    #[test]
    fn the_label_tolerance_is_one_way_and_shared_with_the_gate() {
        // A key MAY carry a sub-letter the form omits: there is only one line 7 to mean.
        let p = one(
            "line7a",
            None,
            shape(1, "AMOUNT", None, &[]),
            shape(1, "AMOUNT", None, &[]),
            "7",
            "7a",
        );
        assert_eq!(
            p.emitted.len(),
            1,
            "line7a over a printed 7: {:?}",
            p.refused
        );
        // ★★ And the tolerance must apply on the NEW side too — the direction a strict comparison
        //    would break. A key `line7a` against a new form that prints plain `7` is the TY2025 → a
        //    hypothetical TY2026 case, and it is also what TY2024's own map needs today. Planting
        //    strictness had to red SOMETHING, and until this case existed it red nothing.
        let p = one(
            "line7a",
            None,
            shape(1, "AMOUNT", None, &[]),
            shape(1, "AMOUNT", None, &[]),
            "7a",
            "7",
        );
        assert_eq!(
            p.emitted.len(),
            1,
            "line7a must carry onto a form printing plain `7`: {:?}",
            p.refused
        );
        // It may NOT lack a letter the form HAS: 11a and 11b both exist, so `line11` says neither.
        assert!(crate::label_reader::label_matches("7a", "7"));
        assert!(!crate::label_reader::label_matches("11", "11a"));
    }

    /// ★★★ **AN AMBIGUOUS LABEL IS REFUSED, NEVER PICKED.**
    ///
    /// Two widgets carrying one label in one column is the 2a/2b shape — the form prints `2a` once over a
    /// pair — and picking the first would put a sub-line figure in the amount column of a signed return.
    /// Planting `[one] => …` as `[one, ..] => …` (take the first of many) red NOTHING until this test
    /// existed, which is the whole reason it does.
    #[test]
    fn two_widgets_with_one_label_in_one_column_are_refused() {
        let prior = vec![Cell {
            key: "line2a".into(),
            fqn: "topmostSubform[0].Page1[0].old[0]".into(),
            on: None,
        }];
        let mut olds = BTreeMap::new();
        olds.insert(
            "topmostSubform[0].Page1[0].old[0]".to_string(),
            shape(1, "AMOUNT", None, &[]),
        );
        let mut news = BTreeMap::new();
        let mut nl = BTreeMap::new();
        for n in ["a", "b"] {
            let fqn = format!("topmostSubform[0].Page1[0].new_{n}[0]");
            news.insert(fqn.clone(), shape(1, "AMOUNT", None, &[]));
            nl.insert(fqn, "2a".to_string());
        }
        let mut ol = BTreeMap::new();
        ol.insert(
            "topmostSubform[0].Page1[0].old[0]".to_string(),
            "2a".to_string(),
        );
        let p = port(&prior, &[], &olds, &news, &ol, &nl);
        assert!(
            p.emitted.is_empty(),
            "two candidates must be refused, not picked from: {:?}",
            p.emitted
        );
        assert!(
            p.refused[0].why.contains("Ambiguous"),
            "and named as ambiguity: {}",
            p.refused[0].why
        );
    }
}
