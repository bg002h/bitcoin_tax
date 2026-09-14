//! ★★★ **`xtask blockers <year>` — WHAT PREVENTS A TY&lt;year&gt; RETURN, READ OFF THE TREE.**
//!
//! **The owner asked for a list. A list is the wrong artifact.** A hand-written blocker list is
//! correct on the day it is written and silently wrong after the next unrelated edit — the exact
//! shape `CLAUDE.md`'s *"derive the list, or make the compiler hold it"* rule names. Three stale
//! prose claims were corrected in this repo on 2026-09-13 alone, and one of them (*"only two text
//! cells differ"* on Form 6251) had **mispriced an owner decision**. So this is an instrument: it
//! answers *"what blocks TY2026?"* by reading the tree, and it is therefore still true next month.
//!
//! ## What it EXTENDS
//!
//! `btctax_cli::year_readiness::YearReadiness` already answers the **year-level** half — `declared`,
//! `table`, `params`, `forms_bundled`, `prices_max_date`, and `problems()`. This consumes that type
//! rather than re-deriving those five facts (see [`year_package`]); every other axis is new.
//!
//! ## The six axes, and where each row's evidence comes from
//!
//! | axis | derived from |
//! |---|---|
//! | [`year_package`] | `YearReadiness::bundled(year)` + `price_coverage_or_refuse` |
//! | [`form_rows`] | `form_delta::port_status`, the work list's OWN generator, parsed **by column name** |
//! | [`archive_rows`] | `design/forms/extract/` × every family the Rust source cites by path |
//! | [`revision_pin_rows`] | the same citations, split into per-revision PREFIXES and pinned LITERALS |
//! | [`gate_rows`] | live calls to every year-keyed gate, at every bundled year |
//! | [`owner_rows`] | `design/ROADMAP_STATUS.md`'s own pending-decision table |
//!
//! ## ★★ BLOCKED is not UNMEASURED, and the vocabulary is the work list's
//!
//! `design/TY2026_WORK_LIST.md` distinguishes `unchanged` (*every axis looked and none found
//! anything*) from `**UNWITNESSED**` (*nothing was found and at least one axis could not look*).
//! That distinction is reused here rather than re-invented: a row is [`State::Unmeasured`] when the
//! fact behind it was not measured, and printing it as though it were clean is the defect the work
//! list has been corrected for twice. Form 8949 is the live case — `the FORM = **UNWITNESSED**`.
//!
//! ## ★ What this does NOT cover — stated, because an honest boundary is reviewable
//!
//! * It does not classify all of `RefuseReason`'s variants. It enumerates the **sites that read the
//!   tax year** in `return_refuse.rs`'s non-test source and attributes each one; the variants no
//!   such site reaches are not year-scoped *by that measurement*, and are reported as a count rather
//!   than as a clean list. See [`gate_rows`].
//! * The archive axis sees only families a Rust file names by path. A document needed by a form
//!   nothing cites is invisible to it.
//! * It reads no network and asks no oracle. Whether the IRS has *published* a revision is outside
//!   it; what it knows is whether this repo has **archived** one.
//! * `when` is copied verbatim out of `forms/<year>/YEAR.toml`'s own reason text where one exists.
//!   It is the declaration's claim, not this tool's prediction.

use btctax_forms::bundled::{self, Stem};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

// ════════════════════════════════════════════════════════════════════════════════════════════════
// The row vocabulary
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// **Who clears this, and therefore where it is printed.** The owner's own four buckets.
///
/// ★ `_`-free matches on this everywhere, so a fifth bucket is a build error rather than a silently
/// unprinted group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Who {
    /// Waiting only on an IRS document — the January finals, an instructions revision.
    January,
    /// Code or a transcription we owe, with the form or module that demands it.
    Build,
    /// An owner ruling or an adjudication.
    Decide,
    /// An action only someone outside this repo can take.
    NotOurs,
}

impl Who {
    /// Every bucket, in print order. Exhaustive by construction — [`Who::heading`]'s match names
    /// every variant, and `every_bucket_has_a_heading` holds this list to it.
    pub const ALL: &'static [Who] = &[Who::January, Who::Build, Who::Decide, Who::NotOurs];

    pub fn heading(self) -> &'static str {
        match self {
            Who::January => "clears itself in January — waiting only on an IRS document",
            Who::Build => "we must build",
            Who::Decide => "we must decide",
            Who::NotOurs => "not ours",
        }
    }
}

/// **Is the row a measured obstruction, or an admission that nothing measured it?**
///
/// The two are three pixels apart on a printed page and are not the same fact — `design/HARNESS.md`
/// B1, and the work list's own `unchanged` / `**UNWITNESSED**` split.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum State {
    /// Measured: this really does stop a TY&lt;year&gt; return.
    Blocked,
    /// NOT measured. Not thereby fine.
    Unmeasured,
}

impl State {
    pub fn cell(self) -> &'static str {
        match self {
            State::Blocked => "BLOCKED",
            State::Unmeasured => "**UNMEASURED**",
        }
    }
}

/// One blocker.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Row {
    pub who: Who,
    pub state: State,
    /// The obstruction, in one sentence.
    pub what: String,
    /// When it can move — verbatim from a declaration where one exists, else `"now"`.
    pub when: String,
    /// **The file, generated column, or refusal variant a reader can check without trusting this
    /// list.** Never a prose claim.
    pub evidence: String,
}

/// The whole answer.
#[derive(Debug, Default)]
pub struct Report {
    pub rows: Vec<Row>,
    /// Facts the report states that are not themselves blockers — what was measured, what was
    /// suppressed, and why.
    pub notes: Vec<String>,
}

impl Report {
    fn push(&mut self, who: Who, state: State, what: String, when: &str, evidence: String) {
        self.rows.push(Row {
            who,
            state,
            what,
            when: when.to_string(),
            evidence,
        });
    }

    pub fn count(&self, who: Who) -> usize {
        self.rows.iter().filter(|r| r.who == who).count()
    }

    pub fn unmeasured(&self) -> usize {
        self.rows
            .iter()
            .filter(|r| r.state == State::Unmeasured)
            .count()
    }

    /// The report as the markdown the operator reads.
    pub fn render(&self, year: i32) -> String {
        let mut s = String::new();
        let _ = writeln!(
            s,
            "# TY{year} blockers — DERIVED at HEAD, {} rows ({} UNMEASURED)\n",
            self.rows.len(),
            self.unmeasured()
        );
        let _ = writeln!(
            s,
            "Generated by `cargo run -p xtask -- blockers {year}`. Every row names its evidence; \
             **UNMEASURED** means the fact behind it was not measured, which is not the same as \
             fine.\n"
        );
        for who in Who::ALL {
            let rows: Vec<&Row> = self.rows.iter().filter(|r| r.who == *who).collect();
            let _ = writeln!(s, "## {} — {} row(s)\n", who.heading(), self.count(*who));
            if rows.is_empty() {
                let _ = writeln!(s, "_none_\n");
                continue;
            }
            let _ = writeln!(s, "| state | when | what | evidence |\n|---|---|---|---|");
            for r in rows {
                let _ = writeln!(
                    s,
                    "| {} | {} | {} | {} |",
                    r.state.cell(),
                    r.when,
                    r.what,
                    r.evidence
                );
            }
            let _ = writeln!(s);
        }
        let _ = writeln!(s, "## what this measured, and what it did not\n");
        for n in &self.notes {
            let _ = writeln!(s, "* {n}");
        }
        s
    }
}

fn root() -> std::path::PathBuf {
    crate::form_geometry::repo_root()
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Axis 1 — the YEAR PACKAGE, from `YearReadiness`
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// `YearStatus` has no `Display`; this is local rather than a change to the shipped type.
fn status_word(r: &btctax_forms::year_record::YearRecord) -> &'static str {
    use btctax_forms::year_record::YearStatus;
    match r.status {
        YearStatus::Preparing => "preparing",
        YearStatus::Slice => "slice",
        YearStatus::Filable => "filable",
    }
}

fn yes_no(b: bool) -> &'static str {
    if b {
        "yes"
    } else {
        "no"
    }
}

/// The year-level half, consumed from [`btctax_cli::year_readiness::YearReadiness`] rather than
/// re-derived. Its `problems()` sentences are pushed **verbatim**: they are already written to be a
/// complete sentence a refusal can print, and re-wording them here would be a second answer to a
/// question that type already answers.
pub fn year_package(year: i32, rep: &mut Report) {
    use btctax_cli::year_readiness::{price_coverage_or_refuse, YearReadiness};
    use btctax_forms::year_record::YearStatus;

    let r = YearReadiness::bundled(year);
    let Some(d) = r.declared.clone() else {
        rep.push(
            Who::Build,
            State::Blocked,
            format!(
                "TY{year} is not a bundled year: no `forms/{year}/YEAR.toml`, so no gate, table, \
                 map or refusal in this build knows the year exists"
            ),
            "now",
            format!(
                "`YearReadiness::bundled({year}).declared` = None; this build bundles {}",
                bundled::years_sentence()
            ),
        );
        return;
    };

    // The declaration-versus-build disagreements, in the type's own words.
    for p in r.problems() {
        rep.push(
            Who::Build,
            State::Blocked,
            p,
            "now",
            "`btctax_cli::year_readiness::YearReadiness::problems()`".into(),
        );
    }

    if d.status != YearStatus::Filable {
        rep.push(
            Who::Build,
            State::Blocked,
            format!(
                "TY{year} declares `status = \"{}\"`, not `filable` — nothing about the year may be \
                 filed while it does",
                status_word(&d)
            ),
            "with the params",
            format!("`forms/{year}/YEAR.toml` `status`"),
        );
    }
    // ★★ THE FILABLE-YEAR GATE, REPORTED as well as asserted. `no_bundled_params_year_has_unreadable_forms`
    //    is the test that keeps a rehearsal branch from merging; this makes an operator running the command
    //    see the same contradiction, because an invariant with only a test for a reader is a figure with no
    //    reader. The predicate is shared, so the row and the gate can never disagree.
    {
        let arch = archived();
        // ★ ONLY years whose params are actually bundled. The predicate's contract is "given a year this
        //   build declares computable, can its forms be read?" — feeding it the reported year
        //   unconditionally states a contradiction that does not exist, and makes this row disagree with
        //   `no_bundled_params_year_has_unreadable_forms`, which filters correctly. Caught by checking the
        //   post-restore count instead of trusting the plant.
        let bundled_params: Vec<i32> = if r.params { vec![year] } else { Vec::new() };
        for msg in params_years_whose_forms_cannot_be_read(&bundled_params, &arch, &|y| {
            btctax_core::tax::state_local_refund::revision_for(y).is_some()
        }) {
            rep.push(
                Who::Build,
                State::Blocked,
                msg,
                "now",
                "`blockers::params_years_whose_forms_cannot_be_read`, asserted by \
                 `no_bundled_params_year_has_unreadable_forms`"
                    .to_string(),
            );
        }
    }

    if !r.params {
        rep.push(
            Who::Build,
            State::Blocked,
            format!(
                "no `FullReturnParams` are bundled for TY{year} — `full_return_for({year})` is \
                 None, which is THE compute gate: the full return does not compute at all"
            ),
            "with the finals",
            format!("`forms/{year}/YEAR.toml` `tables` = {:?}", d.tables.trim()),
        );
    }
    if !r.table {
        rep.push(
            Who::Build,
            State::Blocked,
            format!("no `TaxTable` is bundled for TY{year} — even the crypto slice cannot compute"),
            "with the finals",
            format!("`forms/{year}/YEAR.toml` `tables`; `BundledTaxTables::table_for({year})`"),
        );
    }
    if let Err(e) = price_coverage_or_refuse(year) {
        rep.push(
            Who::Build,
            State::Blocked,
            format!("the EXPORT-TIME price gate refuses TY{year}: {e}"),
            &format!("after {}", d.prices_through),
            format!(
                "`btctax_cli::year_readiness::price_coverage_or_refuse({year})`; \
                 `prices_through` = {}, bundled dataset ends {}",
                d.prices_through,
                r.prices_max_date
                    .map_or_else(|| "NOTHING".to_string(), |m| m.to_string())
            ),
        );
    }
    rep.notes.push(format!(
        "year package: `YearReadiness::bundled({year})` — declared {}, TaxTable {}, params {}, \
         {} forms bundled, prices through {}.",
        status_word(&d),
        yes_no(r.table),
        yes_no(r.params),
        r.forms_bundled,
        r.prices_max_date
            .map_or_else(|| "NOTHING".to_string(), |m| m.to_string())
    ));
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Axis 2 — FORM-LEVEL state, from the work list's OWN generator
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// One parsed row of `port-status`'s tables, **keyed by column NAME**.
///
/// ★ Indexed by header text rather than by position on purpose: the generator has gained six columns
/// in two days (FR-190/191/192, FR-209, FR-211), and a positional reader would have gone on printing
/// plausible wrong cells across every one of those changes. If a column this tool reads is renamed
/// or removed, [`parse_port_status`] returns `Err` — the instrument reds instead of guessing.
pub type Cells = BTreeMap<String, String>;

/// The generator's two tables, parsed, keyed by IRS stem.
#[derive(Debug)]
pub struct PortStatus {
    pub numeric: BTreeMap<String, Cells>,
    pub excused: BTreeMap<String, Cells>,
}

/// The numeric columns this tool reads. Each must appear in the printed header or the parse fails.
pub const REQUIRED_COLUMNS: &[&str] = &[
    "form",
    "the FORM",
    "field map",
    "respelled",
    "added",
    "removed",
    "lines that moved",
    "boxes UNREAD",
    "lines retired",
    "lines introduced",
    "line numbers whose MEANING changed",
    "line numbers UNREAD",
];

fn split_row(line: &str) -> Vec<String> {
    line.trim()
        .trim_start_matches('|')
        .trim_end_matches('|')
        .split('|')
        .map(|c| c.trim().trim_matches('`').to_string())
        .collect()
}

/// Parse `port_status`'s output. Both tables, by column name.
pub fn parse_port_status(printed: &str) -> Result<PortStatus, String> {
    let mut tables: Vec<(Vec<String>, Vec<Vec<String>>)> = Vec::new();
    let mut header: Option<Vec<String>> = None;
    let mut rows: Vec<Vec<String>> = Vec::new();
    for line in printed.lines() {
        let t = line.trim();
        if !t.starts_with('|') {
            if let Some(h) = header.take() {
                tables.push((h, std::mem::take(&mut rows)));
            }
            continue;
        }
        let cells = split_row(t);
        if cells
            .iter()
            .all(|c| !c.is_empty() && c.chars().all(|ch| ch == '-' || ch == ':'))
        {
            continue; // the `|---|---|` separator
        }
        match &header {
            None => header = Some(cells),
            Some(_) => rows.push(cells),
        }
    }
    if let Some(h) = header.take() {
        tables.push((h, rows));
    }
    if tables.len() != 2 {
        return Err(format!(
            "port-status printed {} markdown table(s), expected 2 (numeric + excused)",
            tables.len()
        ));
    }
    let (nh, nr) = &tables[0];
    for want in REQUIRED_COLUMNS {
        if !nh.iter().any(|c| c == want) {
            return Err(format!(
                "port-status's numeric table has no {want:?} column — its header is {nh:?}. A \
                 column this tool reads was renamed or removed; fix the reader rather than letting \
                 it print a plausible wrong cell."
            ));
        }
    }
    let to_map = |h: &[String], r: &[String]| -> Cells {
        h.iter().cloned().zip(r.iter().cloned()).collect()
    };
    let key = |m: &Cells| m.get("form").cloned().unwrap_or_default();
    let numeric = nr
        .iter()
        .map(|r| {
            let m = to_map(nh, r);
            (key(&m), m)
        })
        .collect();
    let (eh, er) = &tables[1];
    let excused = er
        .iter()
        .map(|r| {
            let m = to_map(eh, r);
            (key(&m), m)
        })
        .collect();
    Ok(PortStatus { numeric, excused })
}

/// The tags the committed work list declares it was generated with — read out of the document's own
/// regeneration command, so the two cannot disagree.
///
/// ★★ The markdown emphasis MUST be stripped, and getting this wrong cost a whole run: the document
/// writes the command as **`… port-status 2025 2026-DRAFT`** — bold *around* the code span — so a
/// bare `trim_matches('`')` leaves `2026-DRAFT\`**`, which `starts_with("2026")` happily accepts and
/// which then pairs against no archived stem at all. Every one of the fifteen numeric rows silently
/// became *"NO form axis ran"*: a plausible wrong answer from a tag nobody looked at.
/// `tags_are_stripped_of_markdown_emphasis` is the kill.
pub fn work_list_tags(doc: &str) -> Option<(String, String)> {
    let i = doc.find("port-status ")?;
    let rest = &doc[i + "port-status ".len()..];
    let strip = |s: &str| {
        s.trim_matches(|c: char| c == '`' || c == '*' || c == ',' || c == '.')
            .to_string()
    };
    let mut it = rest.split_whitespace();
    let a = strip(it.next()?);
    let b = strip(it.next()?);
    (!a.is_empty() && !b.is_empty()).then_some((a, b))
}

/// `irs_stem` for every crate stem, read out of the committed `*.map.toml` rows. Derived, because
/// the crate stem and the IRS stem differ on exactly the two schedules the IRS names `f1040sd` /
/// `f1040sse`, and a typed pair there is the `T11` shape.
pub fn irs_stems() -> BTreeMap<Stem, String> {
    let mut out = BTreeMap::new();
    for (stem, year) in bundled::BUNDLED {
        if out.contains_key(stem) {
            continue;
        }
        if let Some(text) = bundled::map_text(*stem, *year) {
            if let Some(v) = text.lines().find_map(|l| l.trim().strip_prefix("irs_stem")) {
                out.insert(
                    *stem,
                    v.trim()
                        .trim_start_matches('=')
                        .trim()
                        .trim_matches('"')
                        .to_string(),
                );
            }
        }
    }
    out
}

/// Which document tags the archive holds, per family, from `design/forms/extract/`.
pub fn archived() -> BTreeMap<String, BTreeSet<String>> {
    let dir = root().join("design/forms/extract");
    let names = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect::<Vec<_>>();
    archived_over(&names)
}

/// The pure half of [`archived`], so a kill can hand it a planted listing.
pub fn archived_over(names: &[String]) -> BTreeMap<String, BTreeSet<String>> {
    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for name in names {
        let Some(base) = name.strip_suffix(".txt") else {
            continue;
        };
        if let Some((fam, tag)) = base.split_once("--") {
            out.entry(fam.to_string())
                .or_default()
                .insert(tag.to_string());
        }
    }
    out
}

/// Does the archive hold a FINAL (non-draft) revision of `fam` for `year`?
fn has_final(arch: &BTreeMap<String, BTreeSet<String>>, fam: &str, year: i32) -> bool {
    arch.get(fam).is_some_and(|t| t.contains(&year.to_string()))
}

/// Does it hold anything at all tagged with `year` (a draft counts)?
fn has_any(arch: &BTreeMap<String, BTreeSet<String>>, fam: &str, year: i32) -> bool {
    let p = year.to_string();
    let drafty = format!("{p}-");
    arch.get(fam)
        .is_some_and(|t| t.iter().any(|x| *x == p || x.starts_with(&drafty)))
}

/// The clause of a declared reason that names WHEN it moves, for the `when` cell.
///
/// ★ The clause naming a package or a month, where the declaration has one; otherwise the FIRST
/// clause. Taking the last clause unconditionally produced *"the §223(b) figures ARE published (Rev.
/// Proc"* for Form 8889 — a citation chopped mid-word, in the column that answers *"when"*.
fn when_clause(reason: &str) -> String {
    let pick = reason
        .split(';')
        .map(str::trim)
        .find(|c| c.contains("package") || c.contains("January"))
        .or_else(|| reason.split(';').map(str::trim).next())
        .unwrap_or(reason);
    pick.chars().take(44).collect()
}

/// The generated form axes for one stem, as one cell, plus what state they leave it in.
fn axis_cell(c: &Cells) -> (String, State) {
    let get = |k: &str| c.get(k).cloned().unwrap_or_default();
    let form = get("the FORM");
    let state = if form.contains("UNWITNESSED") {
        State::Unmeasured
    } else {
        State::Blocked
    };
    (
        format!(
            "the FORM = {form}, field map = {} ({} respelled, {} added, {} removed, {} lines \
             moved, {} retired, {} introduced, {} meanings changed; {} boxes UNREAD, {} line \
             numbers UNREAD)",
            get("field map"),
            get("respelled"),
            get("added"),
            get("removed"),
            get("lines that moved"),
            get("lines retired"),
            get("lines introduced"),
            get("line numbers whose MEANING changed"),
            get("boxes UNREAD"),
            get("line numbers UNREAD"),
        ),
        state,
    )
}

/// **Every form the crate can fill that TY&lt;year&gt; cannot fill, joined to the generated form axes.**
///
/// The set is `Stem::ALL` — the compiler's own list of fillable forms, which `YEAR.toml`'s
/// `forms_expected ∪ forms_absent` is already held to partition. A stem served by
/// [`bundled::periodic_template`] is NOT a blocker and is noted rather than listed.
pub fn form_rows(year: i32, ps: &PortStatus, rep: &mut Report) {
    let Some(rec) = btctax_forms::year_record::YearRecord::for_year(year) else {
        return; // axis 1 already said the year is not bundled
    };
    let irs = irs_stems();
    let arch = archived();
    let mut served = Vec::new();
    let mut bundled_now = Vec::new();
    for stem in Stem::ALL {
        let crate_stem = stem.file_stem();
        if bundled::map_text(*stem, year).is_some() {
            bundled_now.push(crate_stem);
            continue;
        }
        if let Some((_, from)) = bundled::periodic_template(*stem, year) {
            served.push(format!("{crate_stem} (from forms/{from}/)"));
            continue;
        }
        let reason = rec
            .forms_absent
            .get(crate_stem)
            .cloned()
            .unwrap_or_else(|| "NO REASON DECLARED".to_string());
        let irs_stem = irs.get(stem).cloned().unwrap_or_else(|| crate_stem.into());
        let (axis, state) = match ps.numeric.get(&irs_stem) {
            Some(c) => axis_cell(c),
            None => {
                // No pair computed at all. The change axes did not run — that is UNMEASURED, and
                // the excused table says which side is missing.
                let cell = ps
                    .excused
                    .get(&irs_stem)
                    .and_then(|c| c.get("cell").cloned())
                    .unwrap_or_else(|| "not in either table".to_string());
                (
                    format!("NO form axis ran — the pair does not compute ({cell})"),
                    State::Unmeasured,
                )
            }
        };
        // Who clears it: if the archive already holds a FINAL revision, the document is here and
        // the port is ours. If it holds only a draft, or nothing, it waits on the IRS — a draft is
        // EVIDENCE ONLY and is never transcribed (`design/TY2026_WORK_LIST.md`).
        let (who, when) = if has_final(&arch, &irs_stem, year) {
            (Who::Build, "now".to_string())
        } else {
            (Who::January, when_clause(&reason))
        };
        rep.push(
            who,
            state,
            format!(
                "`{crate_stem}` is not bundled for TY{year} — no \
                 `forms/{year}/{crate_stem}.map.toml` and no template, so the form cannot be \
                 filled. {axis}"
            ),
            &when,
            format!(
                "`forms/{year}/YEAR.toml [forms_absent] {crate_stem}` = {reason:?}; \
                 `xtask port-status` row `{irs_stem}`; archive holds {}",
                arch.get(&irs_stem)
                    .map(|t| t.iter().cloned().collect::<Vec<_>>().join(", "))
                    .unwrap_or_else(|| "NOTHING".into())
            ),
        );
    }
    rep.notes.push(format!(
        "forms: `Stem::ALL` is {} fillable forms; {} bundled for TY{year}, {} served by a PERIODIC \
         alias and therefore NOT blockers [{}], the rest listed above.",
        Stem::ALL.len(),
        bundled_now.len(),
        served.len(),
        served.join("; ")
    ));
    rep.notes.push(format!(
        "form axes: read cell-for-cell out of `xtask port-status`, the generator \
         `design/TY2026_WORK_LIST.md` is itself pasted from, indexed BY COLUMN NAME over {} \
         required columns. {} stems have a numeric row, {} are in the excused table.",
        REQUIRED_COLUMNS.len(),
        ps.numeric.len(),
        ps.excused.len()
    ));
    // ★ The generator's own UNWITNESSED cells, whatever stem they land on — including a stem that
    //   is ALREADY bundled and therefore has no blocker row above. `unchanged` is not `read`.
    for (stem, c) in &ps.numeric {
        let get = |k: &str| c.get(k).cloned().unwrap_or_default();
        let form = get("the FORM");
        if form.contains("UNWITNESSED") {
            rep.push(
                Who::Build,
                State::Unmeasured,
                format!(
                    "`{stem}`'s form axis reads {form}: no axis found a change AND at least one \
                     could not look, so the TY{year} revision is NOT known to be unchanged"
                ),
                "now",
                format!(
                    "`xtask port-status` row `{stem}`: `the FORM` = {form}, `line numbers UNREAD` \
                     = {}, `boxes UNREAD` = {}",
                    get("line numbers UNREAD"),
                    get("boxes UNREAD")
                ),
            );
        }
    }
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Axis 3 + 4 — the ARCHIVE, and who NOTICES a new revision
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// Every `design/forms/extract/<family>--` citation in the workspace's Rust source, split by how the
/// path is FORMED.
#[derive(Debug, Default)]
pub struct Citations {
    /// `<family> -> files citing it in any form`.
    pub cited: BTreeMap<String, BTreeSet<String>>,
    /// Families read through a **prefix** const / `format!` join — the per-revision shape: the module
    /// derives its revision set from the directory, so a new revision is noticed the day it lands.
    pub per_revision: BTreeMap<String, BTreeSet<String>>,
    /// `(family, tag) -> sites` for every citation pinned to ONE literal revision, in CODE (not a
    /// doc comment). A pin does not know the year moved.
    pub pinned: BTreeMap<(String, String), BTreeSet<String>>,
}

/// Scan source text. Pure over a file list so a kill can hand it planted text.
pub fn scan_citations(files: &[(String, String)]) -> Citations {
    let mut c = Citations::default();
    const NEEDLE: &str = "design/forms/extract/";
    for (path, text) in files {
        for (n, line) in text.lines().enumerate() {
            let is_doc = line.trim_start().starts_with("//");
            let mut rest = line;
            while let Some(i) = rest.find(NEEDLE) {
                rest = &rest[i + NEEDLE.len()..];
                let fam: String = rest
                    .chars()
                    .take_while(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit())
                    .collect();
                if fam.is_empty() {
                    continue;
                }
                let after = &rest[fam.len()..];
                if !after.starts_with("--") {
                    continue;
                }
                c.cited.entry(fam.clone()).or_default().insert(path.clone());
                let tail = &after[2..];
                let site = format!("{path}:{}", n + 1);
                let digits: String = tail.chars().take_while(char::is_ascii_digit).collect();
                if digits.len() == 4 {
                    if !is_doc {
                        let mut tag = digits;
                        if tail[tag.len()..].starts_with("-DRAFT") {
                            tag.push_str("-DRAFT");
                        }
                        c.pinned.entry((fam, tag)).or_default().insert(site);
                    }
                } else if (tail.starts_with('"') || tail.starts_with('{')) && !is_doc {
                    c.per_revision.entry(fam).or_default().insert(site);
                }
            }
        }
    }
    c
}

/// Every `.rs` file under `crates/`, as `(repo-relative path, text)`.
pub fn workspace_rust() -> Vec<(String, String)> {
    fn walk(dir: &std::path::Path, base: &std::path::Path, out: &mut Vec<(String, String)>) {
        for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let p = e.path();
            let name = e.file_name().to_string_lossy().to_string();
            if p.is_dir() {
                if name == "target" || name.starts_with("target-") {
                    continue;
                }
                walk(&p, base, out);
            } else if name.ends_with(".rs") {
                if let Ok(t) = std::fs::read_to_string(&p) {
                    out.push((
                        p.strip_prefix(base).unwrap_or(&p).to_string_lossy().into(),
                        t,
                    ));
                }
            }
        }
    }
    let mut out = Vec::new();
    walk(&root().join("crates"), &root(), &mut out);
    out.sort();
    out
}

/// **FR-181 — the documents a TY&lt;year&gt; return needs against what the archive holds.**
///
/// The needed set is every family the source cites by path and for which at least one PRIOR revision
/// is archived. A family with no prior revision is a synthetic stem (`f9999`) or a one-off, and
/// claiming it as a gap would be a fabricated row.
pub fn archive_rows(
    year: i32,
    cit: &Citations,
    arch: &BTreeMap<String, BTreeSet<String>>,
    rec: Option<&btctax_forms::year_record::YearRecord>,
    rep: &mut Report,
) {
    let irs_to_crate: BTreeMap<String, Stem> =
        irs_stems().into_iter().map(|(s, irs)| (irs, s)).collect();
    let target = year.to_string();
    let mut have = Vec::new();
    let mut no_prior = Vec::new();
    for (fam, files) in &cit.cited {
        if has_any(arch, fam, year) {
            have.push(fam.clone());
            continue;
        }
        let prior: Vec<String> = arch
            .get(fam)
            .map(|t| {
                t.iter()
                    .filter(|x| x.as_str() < target.as_str())
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        if prior.is_empty() {
            no_prior.push(fam.clone());
            continue;
        }
        // A PERIODIC form has no annual revision to wait for — say so from the declaration rather
        // than inventing an exception.
        let periodic = irs_to_crate.get(fam).and_then(|s| {
            rec.and_then(|r| r.forms_absent.get(s.file_stem()))
                .filter(|why| why.contains("periodic"))
                .cloned()
        });
        let kind = if fam.starts_with('i') {
            "instructions"
        } else {
            "form"
        };
        let newest = prior.last().cloned().unwrap_or_default();
        let what = match &periodic {
            Some(why) => format!(
                "no TY{year} revision of `{fam}` ({kind}) is archived — and it is PERIODIC, so \
                 there may never be one: {why}"
            ),
            None => format!(
                "no TY{year} revision of `{fam}` ({kind}) is archived; the newest is \
                 `{fam}--{newest}`, and {} source file(s) read this family",
                files.len()
            ),
        };
        rep.push(
            Who::January,
            State::Blocked,
            what,
            "when the IRS posts it",
            format!(
                "`design/forms/extract/` holds {fam}--{{{}}}; cited by {}",
                prior.join(","),
                files.iter().take(2).cloned().collect::<Vec<_>>().join(", ")
            ),
        );
    }
    rep.notes.push(format!(
        "archive: {} document families are cited by path in the workspace's Rust source. {} already \
         have a TY{year} revision archived [{}]; {} are cited with no prior revision at all and are \
         therefore not gaps [{}].",
        cit.cited.len(),
        have.len(),
        have.join(", "),
        no_prior.len(),
        no_prior.join(", ")
    ));
}

/// **Is a per-revision family's TY&lt;year&gt; revision TRANSCRIBED, not merely archived?**
///
/// ★ One arm per family that HAS a revision table, and `None` for a family with no probe — stated,
/// rather than answered `true` by a default, because *"no probe"* and *"transcribed"* are the two
/// things this whole file exists to keep apart. At HEAD `i1040gi` is the only per-revision family
/// [`scan_citations`] finds; a second one arriving without an arm here prints as unprobed.
fn transcribed(family: &str, year: i32) -> Option<bool> {
    match family {
        "i1040gi" => Some(btctax_core::tax::state_local_refund::revision_for(year).is_some()),
        _ => None,
    }
}

/// **Who NOTICES the January document when it lands — and who does not.**
///
/// Two shapes, and the difference is the whole row:
/// * a **per-revision** family (a prefix const joined with the year) refuses a year it has no
///   revision for and picks the new one up the day it is archived. That is a blocker that clears
///   itself, and `state_local_refund` is the model.
/// * a family **pinned to one literal revision** keeps reading the old document forever. Nothing
///   reds. That is the T8 shape from `CLAUDE.md`, and it is UNMEASURED for TY&lt;year&gt;, not fine.
///
/// ★★★ **ARCHIVED is not TRANSCRIBED, and the first draft of this function said it was.** It printed
/// *"archiving `i1040gi--2026` clears this without a code change"* — and the B1 plant falsified it in
/// one command: copying the 2025 extract to 2026 cleared the archive gap and left the §111(a) gate
/// shut, because `slr::REVISIONS` is a transcription and the directory only decides what the SUITE
/// demands. The honest claim is the one printed now: archiving reds
/// `every_archived_revision_is_transcribed` and makes the transcription due. A blocker that "clears
/// itself" when it actually hands you work is the same false-completeness this whole instrument is
/// against.
pub fn revision_pin_rows(year: i32, cit: &Citations, rep: &mut Report) {
    for (fam, sites) in &cit.per_revision {
        rep.push(
            Who::January,
            State::Blocked,
            format!(
                "`{fam}` is read as a PER-REVISION family: the module derives its revision set from \
                 `design/forms/extract/`, so archiving `{fam}--{year}` REDS the suite and makes the \
                 transcription due — it does not silently clear, and it does not silently pass"
            ),
            "when the IRS posts it",
            sites.iter().cloned().collect::<Vec<_>>().join(", "),
        );
        // ★ The join the plant demanded: a revision that is archived and NOT transcribed is the work
        //   item January actually creates, and it is invisible to both the archive axis (the document
        //   is there) and the gate axis (which only asks about the target year).
        for tag in archived().get(fam).into_iter().flatten() {
            let Ok(y) = tag.parse::<i32>() else {
                continue; // a draft tag is evidence only and is never transcribed
            };
            match transcribed(fam, y) {
                Some(true) | None => {}
                Some(false) => rep.push(
                    Who::Build,
                    State::Blocked,
                    format!(
                        "`{fam}--{y}` is ARCHIVED but not TRANSCRIBED: the revision table for this \
                         family has no TY{y} entry, so every year that reads it refuses"
                    ),
                    "now",
                    format!(
                        "`design/forms/extract/{fam}--{y}.txt` exists; \
                         `state_local_refund::revision_for({y})` is None"
                    ),
                ),
            }
        }
    }
    let mut per_fam: BTreeMap<&str, Vec<(&String, &BTreeSet<String>)>> = BTreeMap::new();
    for ((fam, tag), sites) in &cit.pinned {
        per_fam.entry(fam.as_str()).or_default().push((tag, sites));
    }
    let target = year.to_string();
    for (fam, mut pins) in per_fam {
        pins.sort_by(|a, b| b.0.cmp(a.0));
        let newest = pins[0].0.clone();
        if newest.starts_with(&target) || cit.per_revision.contains_key(fam) {
            continue; // already pinned at the target year, or the family has a revision table
        }
        let shipped = pins
            .iter()
            .flat_map(|(_, s)| s.iter())
            .any(|s| s.starts_with("crates/btctax-"));
        let all: Vec<String> = pins
            .iter()
            .flat_map(|(tag, s)| s.iter().map(move |x| format!("{x} ({fam}--{tag})")))
            .collect();
        let where_ = if shipped {
            "a SHIPPED crate — it will keep computing from the old revision"
        } else {
            "an INSTRUMENT (xtask) — it will keep checking the old revision and stay green"
        };
        rep.push(
            Who::Build,
            State::Unmeasured,
            format!(
                "`{fam}` is read only through a PINNED literal (`{fam}--{newest}`) in {where_}; \
                 archiving `{fam}--{year}` reds nothing and changes nothing"
            ),
            "now",
            all.join(", "),
        );
    }
    rep.notes.push(format!(
        "revision shape: {} family(ies) are read through a per-revision prefix (they notice a new \
         document); {} (family, revision) pins are literal.",
        cit.per_revision.len(),
        cit.pinned.len()
    ));
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Axis 5 — the YEAR-KEYED GATES, probed live, and the refusals they raise
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// A gate whose answer is keyed on the tax year. Each is a REAL call, evaluated at every bundled
/// year, so *"it refuses because the year is 2026"* is a measurement rather than a reading.
pub struct Gate {
    pub name: &'static str,
    /// `Ok(())` = the gate is open for the year; `Err(why)` = it refuses, in its own words.
    pub probe: fn(i32) -> Result<(), String>,
    /// A literal from `return_refuse.rs` that sits immediately before the `RefuseReason` this gate
    /// raises, or `None` when the gate refuses outside that screen. The instrument reds if the
    /// anchor is gone.
    pub anchor: Option<&'static str>,
    /// ★★ **The symbol a refusal site NAMES when this gate governs it** — the census's attribution
    /// key, so [`year_keyed_markers`] is derived from this list instead of typed beside it. `None`
    /// says out loud that no `return_refuse.rs` site can name this gate, which is a fact about the
    /// gate rather than an omission; the compiler makes the author decide which.
    pub keyed_by: Option<&'static str>,
    /// Where the gate itself lives.
    pub source: &'static str,
}

fn ok_if(b: bool, why: impl Into<String>) -> Result<(), String> {
    if b {
        Ok(())
    } else {
        Err(why.into())
    }
}

/// The year-keyed gates. ★ This is a DECLARED set with a stated boundary (`CLAUDE.md` rule 3): it is
/// what the tree offers as `fn(year) -> Option/Result` on the return path. Each entry is ANCHORED —
/// its `anchor` must still be present in `return_refuse.rs` — so a gate that moves reds here rather
/// than silently dropping out of the list.
pub fn gates() -> Vec<Gate> {
    vec![
        Gate {
            name: "a YEAR.toml record exists (the Form 1099-DA regime is knowable)",
            probe: |y| {
                ok_if(
                    btctax_cli::year_readiness::regime_for(y).is_some(),
                    "no bundled YEAR.toml record — every command refuses the year as UNSUPPORTED",
                )
            },
            anchor: None,
            keyed_by: None,
            source: "btctax-cli/src/year_readiness.rs regime_or_refuse",
        },
        Gate {
            name: "a TaxTable is bundled (the crypto slice computes)",
            probe: |y| {
                ok_if(
                    btctax_cli::year_readiness::YearReadiness::bundled(y).table,
                    "no TaxTable is bundled",
                )
            },
            anchor: None,
            keyed_by: None,
            source: "btctax-adapters/src/tax_tables.rs BundledTaxTables::table_for",
        },
        Gate {
            name: "FullReturnParams are bundled (THE compute gate)",
            probe: |y| {
                ok_if(
                    btctax_cli::year_readiness::YearReadiness::bundled(y).params,
                    "no FullReturnParams — the full return does not compute",
                )
            },
            anchor: None,
            keyed_by: None,
            source: "btctax-adapters/src/tax_tables.rs full_return_for",
        },
        Gate {
            name: "the crypto slice can PRINT (Form 8949 + Schedule D maps for the year)",
            probe: |y| {
                ok_if(
                    btctax_cli::year_readiness::slice_can_print(y),
                    "Form8949Map::for_year / ScheduleDMap::for_year refuse — export-irs-pdf writes \
                     no byte",
                )
            },
            anchor: None,
            keyed_by: None,
            source: "btctax-cli/src/year_readiness.rs slice_can_print",
        },
        Gate {
            name: "the bundled price dataset reaches the year's prices_through",
            probe: |y| {
                btctax_cli::year_readiness::price_coverage_or_refuse(y).map_err(|e| e.to_string())
            },
            anchor: None,
            keyed_by: None,
            source: "btctax-cli/src/year_readiness.rs price_coverage_or_refuse",
        },
        Gate {
            name: "Schedule 1-A exists on the year's return",
            probe: |y| {
                ok_if(
                    btctax_core::tax::tables::schedule_1a_params(y).is_some(),
                    "no Schedule 1-A on this year's return",
                )
            },
            anchor: Some("crate::tax::tables::schedule_1a_params(ri.tax_year).is_none()"),
            keyed_by: Some("schedule_1a_params"),
            source: "btctax-core/src/tax/tables.rs schedule_1a_params",
        },
        Gate {
            name: "the §111(a) STATE AND LOCAL INCOME TAX REFUND WORKSHEET revision is TRANSCRIBED",
            probe: |y| {
                ok_if(
                    btctax_core::tax::state_local_refund::revision_for(y).is_some(),
                    "no i1040gi revision is TRANSCRIBED for the year (`slr::REVISIONS`), so lines 5 \
                     and 6 have no standard-deduction / age-blindness figures and the worksheet \
                     cannot be worked. ★ Distinct from ARCHIVED: archiving the extract reds \
                     `every_archived_revision_is_transcribed` and demands the transcription; it does \
                     not supply it",
                )
            },
            anchor: Some("Err(NotUsable::NoArchivedRevision { tax_year }) =>"),
            keyed_by: Some("NotUsable::"),
            source: "btctax-core/src/tax/state_local_refund.rs revision_for",
        },
        // ★★★ **§68 — THE ITEMIZED-DEDUCTION LIMITATION GATE (B3 I-1).** The most
        //     year-specific refusal in `return_refuse.rs`, and it was in NO list: the site census
        //     could not see it (it reads the year through a parameter, `section_68_status(year)`) and
        //     this gate set did not carry it, so the derived TY2026 blocker list omitted the fact that
        //     **every itemizing TY2026 return over the printed threshold refuses** — and will still
        //     refuse the day January's `FullReturnParams` land, because what is missing is a
        //     *worksheet transcription*, not a parameter table.
        Gate {
            name: "the §68 ITEMIZED DEDUCTIONS WORKSHEET is available for the year (the overall \
                   limitation can be computed)",
            probe: |y| {
                use btctax_core::tax::tables::Section68Status as S;
                // ★ No `_` arm: a fourth `Section68Status` is a build error here too, so a new status
                //   cannot enter the tree while this instrument keeps reporting the old three.
                match btctax_core::tax::tables::section_68_status(y) {
                    S::NotApplicable => Ok(()),
                    S::Gated(g) => Err(format!(
                        "§68 applies to TY{y} and its Schedule A line {line} prints the gate, but the \
                         {ws} it sends a Yes to lives in Form 1040 / Schedule A instructions the IRS \
                         has not published — so EVERY return that itemizes with adjusted gross \
                         income less the qualified-business-income deduction less Schedule 1-A \
                         additional deductions over ${thr} is refused. ★ January's package does not \
                         clear this by arriving: the worksheet must be TRANSCRIBED",
                        line = g.total_line,
                        ws = g.worksheet,
                        thr = g.threshold,
                    )),
                    S::ThresholdNotTranscribed => Err(format!(
                        "§68 applies to TY{y} (Pub. L. 119-21 §70111) and no Schedule A for TY{y} has \
                         been transcribed, so btctax does not know the threshold that year's form \
                         prints — it refuses EVERY itemizing return, not only a large one"
                    )),
                }
            },
            anchor: Some("crate::tax::tables::section_68_status(year) {"),
            keyed_by: Some("section_68_status"),
            source: "btctax-core/src/tax/tables.rs section_68_status",
        },
        // ★★★ B3 I-1's own assertion predicted this: "a THIRD one appearing still reds here." One did,
        //     within hours — `CharitableFloorNotComputed`, from the §170(b)(1)(I) guard that landed
        //     between the I-1 agent starting and the controller integrating it. The census correctly
        //     called it year-reading and UNCLASSIFIED, because no `Gate` probed it.
        //
        // ★★ UNCLASSIFIED was the honest answer and it is not the RIGHT one: a year-keyed gate DOES
        //    govern this refusal — `charitable_floor_status` — so the fix is to declare it, exactly as
        //    I-1 declared §68, not to widen the pinned UNCLASSIFIED set. Widening would have recorded a
        //    known attribution as unknowable.
        //
        // ★ And it matters to a reader rather than only to the census: the owner ITEMIZES and GIVES, so
        //   this is a real January blocker on their own return. As UNCLASSIFIED it printed as UNMEASURED;
        //   as a gate it prints as BLOCKED with the year it clears and what clearing it requires.
        Gate {
            name: "the §170(b)(1)(I) CHARITABLE CONTRIBUTION LIMITATION WORKSHEET is available for the \
                   year (the 0.5%-of-contribution-base floor)",
            probe: |y| {
                use btctax_core::tax::tables::CharitableFloorStatus as C;
                match btctax_core::tax::tables::charitable_floor_status(y) {
                    C::NotApplicable => Ok(()),
                    C::Gated(_) => Err(String::from(
                        "§170(b)(1)(I) floors this year's charitable deduction and btctax does not \
                         compute the Charitable Contribution Limitation Worksheet — the return refuses \
                         rather than deduct an unfloored total (which would understate tax)",
                    )),
                    C::ScheduleANotTranscribed => Err(String::from(
                        "§170(b)(1)(I) floors this year's charitable deduction and no Schedule A \
                         revision for the year is transcribed, so the refusal quotes the statute alone. \
                         ★ Archiving the form does not clear this: the WORKSHEET must be transcribed too",
                    )),
                }
            },
            anchor: Some("crate::tax::tables::charitable_floor_status(year) {"),
            keyed_by: Some("charitable_floor_status"),
            source: "btctax-core/src/tax/tables.rs charitable_floor_status",
        },
    ]
}

/// The `RefuseReason` variant constructed immediately after `anchor` in `src`.
pub fn reason_after(src: &str, anchor: &str) -> Result<String, String> {
    let i = src
        .find(anchor)
        .ok_or_else(|| format!("anchor no longer in the source: {anchor:?}"))?;
    let rest = &src[i..];
    let j = rest
        .find("RefuseReason::")
        .ok_or_else(|| format!("no RefuseReason after the anchor {anchor:?}"))?;
    let tail = &rest[j + "RefuseReason::".len()..];
    let n = tail
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(tail.len());
    Ok(tail[..n].to_string())
}

/// The non-test part of a source file — everything before the column-0 `#[cfg(test)] mod tests {`.
pub fn non_test(src: &str) -> &str {
    match src.find("\n#[cfg(test)]\nmod tests {") {
        Some(i) => &src[..i],
        None => src,
    }
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// The refusal census — the DENOMINATOR is every RAISE SITE, not the year reads a needle list finds
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// **A refusal that compares the tax year to a literal.** ★ A stated boundary (`CLAUDE.md` rule (3)):
/// every *other* attribution key is derived from [`Gate::keyed_by`], but a rule that reads the year
/// *itself* — `if ri.tax_year == 0` — is governed by no gate and never will be.
const BARE_YEAR_COMPARISON: &[&str] = &["tax_year ==", "tax_year !="];

/// Identifiers that mean *"this line reads the tax year"*. `tax_year` covers `ri.tax_year` and a
/// `tax_year` binding; a bare `year` is the **parameter** idiom — the one the pre-fix census could
/// not see, which is why all four `section_68_gate(year, …)` and `screen_broker_reporting(…, year, …)`
/// sites were invisible to it. Matched as whole identifiers, so `years` and `tax_year` never collide.
const YEAR_READ: &[&str] = &["tax_year", "year"];

/// The attribution keys the census recognises: DERIVED from [`gates`], plus [`BARE_YEAR_COMPARISON`].
///
/// ★★ This replaces a hand-typed `YEAR_KEYED` const that sat beside `gates()` knowing nothing about
/// it. A gate now carries its own attribution symbol in [`Gate::keyed_by`], so the recognised set
/// widens the day a gate lands — and a gate with nothing for a site to name must write `None`, which
/// the compiler asks for rather than letting the author forget.
#[must_use]
pub fn year_keyed_markers() -> Vec<&'static str> {
    let mut v: Vec<&'static str> = gates().into_iter().filter_map(|g| g.keyed_by).collect();
    v.extend_from_slice(BARE_YEAR_COMPARISON);
    v.sort_unstable();
    v.dedup();
    v
}

/// `src` with every comment and the **contents** of every string literal blanked, one output line per
/// input line, so a line number this census reports is the real one.
///
/// ★★ **Why the instrument cannot skip this.** A refusal's *message* names the year in nearly every
/// case — `format!("TY{year} has no disposition …")` — so a scan that reads prose calls **34 of the
/// 95** raise sites year-reading (measured) and the census degenerates into noise. Only a year read in
/// CODE decides anything.
#[must_use]
pub fn code_lines(src: &str) -> Vec<String> {
    enum Lex {
        Code,
        Block,
        Str { raw: Option<usize> },
    }
    let mut st = Lex::Code;
    let mut out = Vec::new();
    for line in src.lines() {
        let b: Vec<char> = line.chars().collect();
        let mut keep = String::with_capacity(line.len());
        let mut i = 0usize;
        while i < b.len() {
            match st {
                Lex::Code => {
                    if b[i] == '/' && b.get(i + 1) == Some(&'/') {
                        break; // a line comment ends the line
                    }
                    if b[i] == '/' && b.get(i + 1) == Some(&'*') {
                        st = Lex::Block;
                        i += 2;
                        continue;
                    }
                    // `r"…"` / `r#"…"#`. ★ The identifier guard is load-bearing: without it the `r"`
                    //   ending `"… with a preparer",` opens a raw string and swallows the code after.
                    if b[i] == 'r'
                        && i.checked_sub(1)
                            .is_none_or(|p| !(b[p].is_alphanumeric() || b[p] == '_'))
                    {
                        let mut h = 0usize;
                        while b.get(i + 1 + h) == Some(&'#') {
                            h += 1;
                        }
                        if b.get(i + 1 + h) == Some(&'"') {
                            st = Lex::Str { raw: Some(h) };
                            i += 2 + h;
                            continue;
                        }
                    }
                    if b[i] == '"' {
                        st = Lex::Str { raw: None };
                        i += 1;
                        continue;
                    }
                    keep.push(b[i]);
                    i += 1;
                }
                Lex::Block => {
                    if b[i] == '*' && b.get(i + 1) == Some(&'/') {
                        st = Lex::Code;
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                Lex::Str { raw: None } => {
                    if b[i] == '\\' {
                        i += 2;
                    } else if b[i] == '"' {
                        st = Lex::Code;
                        i += 1;
                    } else {
                        i += 1;
                    }
                }
                Lex::Str { raw: Some(h) } => {
                    if b[i] == '"' && (1..=h).all(|k| b.get(i + k) == Some(&'#')) {
                        st = Lex::Code;
                        i += 1 + h;
                    } else {
                        i += 1;
                    }
                }
            }
        }
        out.push(keep);
    }
    out
}

/// Is `needle` present in `hay` as a whole identifier? (`year` matches `year` and `(year,` — never
/// `tax_year`, `years` or `year_keyed`.)
fn word_in(hay: &str, needle: &str) -> bool {
    let ident = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let mut from = 0usize;
    while let Some(p) = hay[from..].find(needle) {
        let at = from + p;
        from = at + needle.len();
        if hay[..at].chars().next_back().is_none_or(|c| !ident(c))
            && hay[from..].chars().next().is_none_or(|c| !ident(c))
        {
            return true;
        }
    }
    false
}

/// One `RefuseReason::…` construction in the scanned body — **the census's unit, and its denominator.**
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaiseSite {
    /// 1-based line in the scanned file.
    pub line: usize,
    /// The variant raised, read off the construction itself.
    pub variant: String,
    /// The CODE this site's decision is taken in: from the end of the previous refusal statement (or
    /// the enclosing `fn`, whichever is later) through this site's own construction.
    pub decides_in: String,
    /// Why this site reads the tax year, in the instrument's own words. Empty ⇒ no year read in code.
    pub year_read: Vec<String>,
    /// The year-keyed markers the decision names — the gate(s) that could govern it.
    pub keyed: Vec<String>,
}

impl RaiseSite {
    #[must_use]
    pub fn reads_year(&self) -> bool {
        !self.year_read.is_empty() || !self.keyed.is_empty()
    }
}

/// **Every refusal this file can raise, and which of them read the tax year.**
///
/// ★★★ **The denominator WAS the defect.** (B3 I-1,
/// `design/agent-reports/REPORT-b3-2026-09-13.md` — the one citation for every `B3 I-1` below.)
/// The census this replaces scanned two
/// needles — `ri.tax_year` and `NotUsable::` — and then computed its UNCLASSIFIED set *over the sites
/// those needles had already found*. A site reading the year through a **function parameter** was
/// therefore neither counted nor reported: it could not even become an UNCLASSIFIED row, because it
/// did not exist. Four variants carry a `year` payload — they cannot be raised without reading the
/// year — and the old census attributed exactly one, while the command printed *"every other variant
/// never reads the year"* over all 134.
///
/// So the direction is inverted. **Every** `RefuseReason::…` construction is enumerated first, and
/// each is then classified. Nothing can fall between needles because there are none: a site the
/// instrument cannot attribute is a row, and a `year`-payload variant it cannot locate is a row.
#[derive(Debug, Default, Clone)]
pub struct RefusalCensus {
    /// Every variant the `RefuseReason` enum declares.
    pub variants: BTreeSet<String>,
    /// ★ Variants whose PAYLOAD declares a `year` field — read off the **type**, so they cannot be
    /// raised without reading the year, and a fifth one enters the census with no edit here.
    pub year_payload: BTreeSet<String>,
    /// Every raise site, in line order.
    pub sites: Vec<RaiseSite>,
}

impl RefusalCensus {
    pub fn reading(&self) -> Vec<&RaiseSite> {
        self.sites.iter().filter(|s| s.reads_year()).collect()
    }
    pub fn blind(&self) -> Vec<&RaiseSite> {
        self.sites.iter().filter(|s| !s.reads_year()).collect()
    }
    /// The year-reading sites, grouped by the variant they raise.
    pub fn by_variant(&self) -> BTreeMap<String, Vec<&RaiseSite>> {
        let mut m: BTreeMap<String, Vec<&RaiseSite>> = BTreeMap::new();
        for s in self.reading() {
            m.entry(s.variant.clone()).or_default().push(s);
        }
        m
    }
    pub fn raised(&self) -> BTreeSet<String> {
        self.sites.iter().map(|s| s.variant.clone()).collect()
    }
    /// Declared variants with no raise site in the scanned file. **Not** thereby year-agnostic — they
    /// are out of this census's reach, and the note reports the count instead of claiming them.
    pub fn not_raised_here(&self) -> BTreeSet<String> {
        let raised = self.raised();
        self.variants.difference(&raised).cloned().collect()
    }
    /// Year-reading variants naming no year-keyed marker at ANY of their sites: the census sees them
    /// read the year and cannot say which gate governs it. **UNCLASSIFIED, never dropped.**
    pub fn unclassified(&self) -> BTreeSet<String> {
        self.by_variant()
            .into_iter()
            .filter(|(_, ss)| ss.iter().all(|s| s.keyed.is_empty()))
            .map(|(v, _)| v)
            .collect()
    }
    /// ★★ Variants that CANNOT be raised without a year — their payload says so — for which the census
    /// found no year-reading site at all. **This is the hole §68 fell through, made loud.** Non-empty
    /// means the instrument is scanning the wrong file or the raise has moved out of it.
    pub fn unlocated_year_payload(&self) -> BTreeSet<String> {
        let seen: BTreeSet<String> = self.by_variant().keys().cloned().collect();
        self.year_payload.difference(&seen).cloned().collect()
    }
}

/// The `pub enum RefuseReason` body in `code`, as inclusive line indices.
fn enum_span(code: &[String]) -> Option<(usize, usize)> {
    let start = code
        .iter()
        .position(|l| l.contains("pub enum RefuseReason {"))?;
    let mut depth = 0usize;
    for (i, l) in code.iter().enumerate().skip(start) {
        for c in l.chars() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some((start, i));
                    }
                }
                _ => {}
            }
        }
    }
    None
}

/// Split an enum body into one string per variant, at top-level commas.
fn split_variants(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut depth = 0i32;
    for c in body.chars() {
        match c {
            '{' | '(' | '[' => depth += 1,
            '}' | ')' | ']' => depth -= 1,
            _ => {}
        }
        if c == ',' && depth == 0 {
            out.push(std::mem::take(&mut cur));
        } else {
            cur.push(c);
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur);
    }
    out
}

/// The variant name a declaration opens with, if it is one.
fn variant_name(decl: &str) -> Option<String> {
    let t = decl.trim_start();
    let n: String = t.chars().take_while(char::is_ascii_alphanumeric).collect();
    if n.is_empty() || !n.starts_with(|c: char| c.is_ascii_uppercase()) {
        return None;
    }
    let rest = t[n.len()..].trim_start();
    (rest.is_empty() || rest.starts_with([',', '{', '('])).then_some(n)
}

/// Does this line open a function?
fn opens_fn(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("fn ") || t.contains(" fn ")
}

/// Walk out from a construction at `(line, col)`; returns `(text, last line index touched)`.
///
/// `Reach::Payload` stops at the close of the variant's own `{ … }` / `( … )`. `Reach::Statement`
/// walks to the end of the whole refusal **statement** — the first `;` at depth ≤ 0, or the closer of
/// the enclosing call, whichever comes first — which is the span a following window must start after.
enum Reach {
    Payload,
    Statement,
}

fn extent(code: &[String], line: usize, col: usize, reach: &Reach) -> (String, usize) {
    let payload = matches!(reach, Reach::Payload);
    let mut text = String::new();
    let mut depth = 0i32;
    let mut opened = false;
    let mut last = line;
    for (i, l) in code.iter().enumerate().skip(line) {
        let from = if i == line { col.min(l.len()) } else { 0 };
        for c in l[from..].chars() {
            if payload {
                text.push(c);
            }
            match c {
                '{' | '(' | '[' => {
                    depth += 1;
                    opened = true;
                }
                '}' | ')' | ']' => {
                    depth -= 1;
                    if payload && opened && depth == 0 {
                        return (text, i);
                    }
                    if !payload && depth < 0 {
                        return (text, i);
                    }
                }
                ';' if !payload && depth <= 0 => return (text, i),
                _ => {}
            }
        }
        // A unit variant (`RefuseReason::Foo,`) opens no delimiter and is one line long.
        if payload && !opened {
            return (text, line);
        }
        last = i;
    }
    (text, last)
}

/// **The census.** `src` is a whole source file; the `#[cfg(test)]` half is dropped *here* rather than
/// by the caller, so a planted string can be handed straight in and classified exactly as HEAD is.
#[must_use]
pub fn refusal_census(src: &str) -> RefusalCensus {
    let code = code_lines(non_test(src));
    let mut cen = RefusalCensus::default();

    // ── the TYPE: every variant, and the ones whose payload declares a year ─────────────────────
    let span = enum_span(&code);
    if let Some((a, b)) = span {
        let joined = code[a..=b].join("\n");
        let open = joined.find('{').map_or(0, |i| i + 1);
        let close = joined.rfind('}').unwrap_or(joined.len());
        for decl in split_variants(&joined[open..close]) {
            if let Some(name) = variant_name(&decl) {
                let one = decl.split_whitespace().collect::<Vec<_>>().join(" ");
                if word_in(&one, "year") {
                    cen.year_payload.insert(name.clone());
                }
                cen.variants.insert(name);
            }
        }
    }

    // ── the SITES: every construction, in line order ────────────────────────────────────────────
    let mut raw: Vec<(usize, usize, String)> = Vec::new(); // (line index, col after the name, variant)
    for (i, l) in code.iter().enumerate() {
        if span.is_some_and(|(a, b)| i >= a && i <= b) {
            continue;
        }
        let mut from = 0usize;
        while let Some(p) = l[from..].find("RefuseReason::") {
            let at = from + p + "RefuseReason::".len();
            from = at;
            let name: String = l[at..]
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                raw.push((i, at + name.len(), name));
            }
        }
    }

    let markers = year_keyed_markers();
    let mut prev_end = 0usize; // the first line index a new window may start at
    for (li, col, var) in &raw {
        // The enclosing fn, so a window never reaches back past the function's own signature.
        let fn_at = code[..=*li].iter().rposition(|l| opens_fn(l)).unwrap_or(0);
        let lo = fn_at.max(prev_end).min(*li);
        let mut decides = code[lo..=*li].join("\n");
        // …plus this site's own construction, which may carry the year as a payload field on a LATER
        // line than the one the variant is named on.
        let (payload, payload_end) = extent(&code, *li, *col, &Reach::Payload);
        decides.push('\n');
        decides.push_str(&payload);
        // The NEXT window starts after this refusal's whole statement. ★ Measured, and the reason
        // this is delimiter-matched rather than line-counted: a refusal's own `format!` carries
        // `year = ri.tax_year,` as a message argument, and letting that bleed forward made two
        // year-blind rules (`BrokerReportingMixed`, `Schedule1aTipsFromTradeOrBusiness`) look
        // year-conditional. A message that names the year decides nothing.
        let (_, stmt_end) = extent(&code, *li, *col, &Reach::Statement);
        prev_end = stmt_end.max(payload_end).max(*li) + 1;

        let mut year_read: Vec<String> = YEAR_READ
            .iter()
            .filter(|k| word_in(&decides, k))
            .map(|k| format!("`{k}`"))
            .collect();
        if cen.year_payload.contains(var) {
            year_read.push("the variant's PAYLOAD declares `year`".into());
        }
        let keyed: Vec<String> = markers
            .iter()
            .filter(|k| decides.contains(**k))
            .map(|k| (*k).to_string())
            .collect();
        cen.sites.push(RaiseSite {
            line: li + 1,
            variant: var.clone(),
            decides_in: decides,
            year_read,
            keyed,
        });
    }
    cen
}

/// **Which refusals fire BECAUSE the year is &lt;year&gt;, separated from the ones that fire on any
/// year.**
pub fn gate_rows(year: i32, rep: &mut Report) -> Result<(), String> {
    let years: Vec<i32> = bundled::bundled_years().to_vec();
    let src = std::fs::read_to_string(root().join("crates/btctax-core/src/tax/return_refuse.rs"))
        .map_err(|e| format!("return_refuse.rs: {e}"))?;
    let body = non_test(&src);

    let mut year_specific: Vec<&'static str> = Vec::new();
    let mut everywhere: Vec<&'static str> = Vec::new();
    let mut open: Vec<&'static str> = Vec::new();
    for g in gates() {
        // The anchor is checked whether or not the gate refuses — a moved anchor must red at HEAD,
        // not only on the year that happens to trip it.
        let raises = match g.anchor {
            Some(a) => Some(reason_after(body, a)?),
            None => None,
        };
        let elsewhere: Vec<i32> = years
            .iter()
            .copied()
            .filter(|y| *y != year && (g.probe)(*y).is_ok())
            .collect();
        match (g.probe)(year) {
            Ok(()) => open.push(g.name),
            Err(why) if elsewhere.is_empty() => {
                everywhere.push(g.name);
                rep.push(
                    Who::Build,
                    State::Blocked,
                    format!(
                        "the gate *{}* is shut for TY{year} — and for EVERY bundled year, so it is \
                         NOT year-specific: {why}",
                        g.name
                    ),
                    "now",
                    format!("{} (probed at {years:?})", g.source),
                );
            }
            Err(why) => {
                year_specific.push(g.name);
                let raised = raises
                    .as_deref()
                    .map(|r| format!("; raises `RefuseReason::{r}`"))
                    .unwrap_or_default();
                rep.push(
                    Who::January,
                    State::Blocked,
                    format!(
                        "the gate *{}* is shut for TY{year} and OPEN for {elsewhere:?} — it refuses \
                         BECAUSE the year is {year}: {why}",
                        g.name
                    ),
                    "with the year's document",
                    format!("{}{raised} (probed at {years:?})", g.source),
                );
            }
        }
    }

    // ── The refusal census. Every raise site is enumerated, then classified; see [`refusal_census`].
    let cen = refusal_census(&src);
    let by_var = cen.by_variant();
    let reading = cen.reading().len();
    let blind = cen.blind().len();
    let unclassified = cen.unclassified();
    let unlocated = cen.unlocated_year_payload();
    // ★ Two independent derivations of the same set, cross-checked at run time rather than in a test
    //   only: `census_join` reads the declaration lines, this census parses the enum body.
    let joined = crate::census_join::variant_paths(&src, "RefuseReason");
    if joined.len() != cen.variants.len() {
        return Err(format!(
            "the two variant derivations disagree: census_join sees {} and refusal_census sees {} — \
             one of them is misreading `pub enum RefuseReason`",
            joined.len(),
            cen.variants.len()
        ));
    }

    for var in &unclassified {
        let at = by_var[var]
            .iter()
            .map(|s| s.line.to_string())
            .collect::<Vec<_>>()
            .join(",");
        rep.push(
            Who::Build,
            State::Unmeasured,
            format!(
                "`RefuseReason::{var}` at line(s) {at} reads the tax year and this instrument cannot \
                 say which year-keyed gate governs it — UNCLASSIFIED, which is not the same as \
                 year-agnostic"
            ),
            "now",
            "crates/btctax-core/src/tax/return_refuse.rs (blockers::refusal_census)".into(),
        );
    }
    // ★★ A variant that cannot be raised without a year, whose raise site this census could not find
    //    at all. Exactly the hole `ItemizedDeductionLimitationNotComputed` fell through — silent then,
    //    a row now.
    for var in &unlocated {
        rep.push(
            Who::Build,
            State::Unmeasured,
            format!(
                "`RefuseReason::{var}` carries a `year` payload — it CANNOT be raised without reading \
                 the tax year — and this census found no raise site for it in the file it scans, so \
                 nothing here can say whether it fires because the year is {year}"
            ),
            "now",
            "crates/btctax-core/src/tax/return_refuse.rs (blockers::refusal_census)".into(),
        );
    }

    rep.notes.push(format!(
        "refusals: `RefuseReason` declares {} variants. This census enumerates EVERY raise site in \
         `return_refuse.rs` — {} sites raising {} distinct variants — and classifies each: {} site(s) \
         read the tax year in CODE ({} distinct variants), {} do not. {} UNCLASSIFIED variant(s) \
         [{}]; {} `year`-payload variant(s) with no located raise site [{}]. Year-keyed gates: {} shut \
         for TY{year} while OPEN for another bundled year [{}]; {} shut for every bundled year [{}]; \
         {} open [{}].",
        cen.variants.len(),
        cen.sites.len(),
        cen.raised().len(),
        reading,
        by_var.len(),
        blind,
        unclassified.len(),
        unclassified.iter().cloned().collect::<Vec<_>>().join(", "),
        unlocated.len(),
        unlocated.iter().cloned().collect::<Vec<_>>().join(", "),
        year_specific.len(),
        year_specific.join(", "),
        everywhere.len(),
        everywhere.join(", "),
        open.len(),
        open.join(", "),
    ));
    // ★★★ **THE BOUNDARY, STATED (`CLAUDE.md` rule (3)).** The sentence this replaces was *"every
    //     other variant never reads the year, so it cannot fire because the year is <year>"* — a
    //     universal over all 134 variants, and false for the most year-specific refusal in the file.
    //     What this instrument can actually support is written out instead, including the two things
    //     it cannot see.
    rep.notes.push(format!(
        "refusal census — WHAT IT DOES NOT CLAIM: (a) it reads ONE file, \
         `btctax-core/src/tax/return_refuse.rs`, and {} declared variant(s) are raised nowhere in it, \
         so nothing here says those cannot fire because the year is {year}; (b) for the {blind} \
         site(s) it calls year-blind the finding is \"no year read appears in the code between this \
         refusal and the previous one\" — NOT \"this refusal never reads the year\"; a year consulted \
         further up its function is outside the window. A `year`-payload variant is exempt from (b): \
         it is read off the TYPE, so it cannot be missed. Year-reading sites, attributed: {}",
        cen.not_raised_here().len(),
        by_var
            .iter()
            .map(|(v, ss)| format!(
                "{v} @ {} [{}]",
                ss.iter()
                    .map(|s| s.line.to_string())
                    .collect::<Vec<_>>()
                    .join("/"),
                {
                    let mut k: Vec<&str> =
                        ss.iter().flat_map(|s| s.keyed.iter().map(String::as_str)).collect();
                    k.sort_unstable();
                    k.dedup();
                    if k.is_empty() {
                        "UNCLASSIFIED".to_string()
                    } else {
                        k.join("+")
                    }
                }
            ))
            .collect::<Vec<_>>()
            .join("; ")
    ));
    Ok(())
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Axis 6 — the OWNER's own pending-decision table
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The pending owner decisions, parsed out of `design/ROADMAP_STATUS.md`'s own table, so the set
/// grows when the owner's table does. A row whose text says `RULED` is no longer pending.
pub fn owner_rows(doc: &str, rep: &mut Report) {
    let Some(i) = doc.find("OWNER DECISIONS PENDING") else {
        rep.notes.push(
            "owner decisions: `design/ROADMAP_STATUS.md` has no \"OWNER DECISIONS PENDING\" \
             section — the axis did not run."
                .into(),
        );
        return;
    };
    let mut pending = 0usize;
    let mut ruled = 0usize;
    for line in doc[i..].lines() {
        let t = line.trim();
        if !t.starts_with("| **S") {
            if t.starts_with("## ") {
                break;
            }
            continue;
        }
        let cells = split_row(t);
        let id = cells[0].trim_matches('*').to_string();
        let text = cells.get(1).cloned().unwrap_or_default();
        if text.contains("RULED") {
            ruled += 1;
            continue;
        }
        pending += 1;
        let headline: String = text
            .split(" — ")
            .next()
            .unwrap_or(&text)
            .trim_matches('*')
            .chars()
            .take(150)
            .collect();
        rep.push(
            Who::Decide,
            State::Blocked,
            format!("{id} is unruled: {headline}"),
            "owner",
            format!("`design/ROADMAP_STATUS.md` §0a, row {id}"),
        );
        // An unruled decision naming work outside this repo is ALSO not ours to clear.
        if text.contains("apprais") {
            rep.push(
                Who::NotOurs,
                State::Blocked,
                format!(
                    "{id} names a §170(f)(11)(C) QUALIFIED APPRAISAL for a Bitcoin gift over $5,000 \
                     — an external appraiser's work, with a deadline that falls before filing; no \
                     code clears it"
                ),
                "before filing",
                format!(
                    "`design/ROADMAP_STATUS.md` §0a row {id}; \
                     `design/SPEC_appraisal_trigger_minimal.md`; \
                     `crates/btctax-core/src/donation.rs` (Section-B completeness needs \
                     `appraisal_date` + `appraiser_qualifications`)"
                ),
            );
        }
    }
    rep.notes.push(format!(
        "owner decisions: {pending} pending, {ruled} already RULED, parsed from \
         `design/ROADMAP_STATUS.md` §0a."
    ));
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// The command
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// Assemble the whole report.
pub fn collect(year: i32) -> Result<Report, String> {
    let mut rep = Report::default();
    year_package(year, &mut rep);

    let wl_path = root().join("design/TY2026_WORK_LIST.md");
    let wl =
        std::fs::read_to_string(&wl_path).map_err(|e| format!("{}: {e}", wl_path.display()))?;
    let (prior, new) = work_list_tags(&wl).ok_or_else(|| {
        "design/TY2026_WORK_LIST.md declares no `port-status <prior> <new>` command — the form axis \
         has no tags to run on"
            .to_string()
    })?;
    if !new.starts_with(&year.to_string()) {
        return Err(format!(
            "the committed work list is generated for `{new}`, not TY{year}: the form axis would \
             compare the wrong revision. Regenerate the work list for TY{year} first."
        ));
    }
    let printed = crate::form_delta::port_status(&prior, &new)?;
    let ps = parse_port_status(&printed)?;
    form_rows(year, &ps, &mut rep);

    let files = workspace_rust();
    let cit = scan_citations(&files);
    let arch = archived();
    let rec = btctax_forms::year_record::YearRecord::for_year(year);
    archive_rows(year, &cit, &arch, rec.as_ref(), &mut rep);
    revision_pin_rows(year, &cit, &mut rep);
    gate_rows(year, &mut rep)?;

    let rs = root().join("design/ROADMAP_STATUS.md");
    match std::fs::read_to_string(&rs) {
        Ok(doc) => owner_rows(&doc, &mut rep),
        Err(e) => rep
            .notes
            .push(format!("owner decisions: {} unreadable: {e}", rs.display())),
    }
    rep.notes.push(format!(
        "form-axis tags: `{prior}` -> `{new}`, read out of the work list's own regeneration \
         command; {} source files scanned for archive citations.",
        files.len()
    ));
    rep.rows.sort();
    Ok(rep)
}

pub fn run(year: i32) -> Result<(), String> {
    print!("{}", collect(year)?.render(year));
    Ok(())
}

/// ★★★ **THE FILABLE-YEAR GATE (stage-2 design §2).** Which bundled-params years cannot have their forms
/// READ — i.e. a year this build declares computable while the documents it was transcribed from are
/// absent, or the per-revision worksheet for it was never transcribed.
///
/// **Why a CONDITION rather than a state assertion.** A flag can be flipped, a branch can be merged and an
/// intention can be forgotten; this says *why* a year may be bundled, so it permits the bundling the day
/// the condition holds and needs no later deletion. The repo owns the idiom twice already —
/// `ty2025_full_return_must_stay_fail_closed_until_complete` and `state_local_refund`'s own
/// archive-dependent refusal.
///
/// ★ **`transcribed` is NOT `archived`, and a plant proved it** (stage 1, plant B): copying a prior
/// extract into a new year's filename cleared the archive gap and left the §111(a) gate SHUT, because
/// `state_local_refund::revision_for` reads a TRANSCRIPTION while the directory only decides what the
/// suite demands. A document arriving is not a document being read, so both conditions are required.
///
/// Split out and pure so the kill below can watch it discriminate in BOTH directions on planted input.
#[must_use]
pub fn params_years_whose_forms_cannot_be_read(
    params_years: &[i32],
    archived: &std::collections::BTreeMap<String, std::collections::BTreeSet<String>>,
    transcribed: &dyn Fn(i32) -> bool,
) -> Vec<String> {
    let mut bad = Vec::new();
    for &y in params_years {
        for fam in ["f1040", "i1040gi"] {
            if !has_any(archived, fam, y) {
                bad.push(format!(
                    "TY{y}: FullReturnParams are bundled but `{fam}--{y}` is NOT ARCHIVED — the year is \
                     declared computable while a document it must be transcribed from is absent"
                ));
            }
        }
        if !transcribed(y) {
            bad.push(format!(
                "TY{y}: FullReturnParams are bundled but the §111(a) worksheet revision for {y} is NOT \
                 TRANSCRIBED — archiving the document does not transcribe it (stage 1, plant B)"
            ));
        }
    }
    bad
}

#[cfg(test)]
mod tests {
    /// ★★★ **The filable-year gate.** No year may have `FullReturnParams` bundled unless its Form 1040 and
    /// its instructions are archived AND its §111(a) revision is transcribed.
    ///
    /// **This is what makes the stage-2 TY2026 rehearsal a throwaway by CONSTRUCTION.** The rehearsal's
    /// core is one line — `by_year.insert(2026, ty2026_full_return())` — and with `f1040--2026` and
    /// `i1040gi--2026` unarchived, that line reds THIS test. So the branch cannot merge silently, and no
    /// flag, tripwire or promise is involved. In January, when the documents land and the revision is
    /// transcribed, the gate permits the bundling and stops blocking without being edited.
    ///
    /// ★ Generic over years on purpose: a gate hardcoded to 2026 would be a typed list beside a growing
    /// set, which is the defect this repo finds most often.
    #[test]
    fn no_bundled_params_year_has_unreadable_forms() {
        let arch = archived();
        let params_years: Vec<i32> = btctax_forms::bundled::bundled_years()
            .iter()
            .copied()
            .filter(|y| btctax_cli::year_readiness::YearReadiness::bundled(*y).params)
            .collect();
        assert!(
            !params_years.is_empty(),
            "no year has params bundled at all — this gate would then be vacuous, which is itself the bug"
        );
        let bad = params_years_whose_forms_cannot_be_read(&params_years, &arch, &|y| {
            btctax_core::tax::state_local_refund::revision_for(y).is_some()
        });
        assert!(bad.is_empty(), "{}", bad.join("\n"));

        // ★ THE KILL, both directions on planted input — a year bundled with nothing archived must be
        //   named, and the archived-but-untranscribed state must be named SEPARATELY, because stage 1's
        //   plant B showed those are different facts with different remedies.
        let planted = params_years_whose_forms_cannot_be_read(&[2099], &arch, &|_| true);
        assert_eq!(
            planted.len(),
            2,
            "a wholly unarchived year must name BOTH documents: {planted:?}"
        );
        assert!(
            planted.iter().all(|m| m.contains("NOT ARCHIVED")),
            "{planted:?}"
        );

        let untranscribed =
            params_years_whose_forms_cannot_be_read(&params_years, &arch, &|_| false);
        assert_eq!(
            untranscribed.len(),
            params_years.len(),
            "every bundled year with no transcription must be named: {untranscribed:?}"
        );
        assert!(
            untranscribed.iter().all(|m| m.contains("NOT TRANSCRIBED")),
            "the untranscribed message must say TRANSCRIBED, not archived — that distinction is the \
             whole finding: {untranscribed:?}"
        );
    }

    use super::*;

    /// The extract-directory prefix, ASSEMBLED at runtime.
    ///
    /// ★ Deliberately not a literal: [`scan_citations`] walks every `.rs` file under `crates/`,
    /// including this one, so a fixture written as a literal path would make this module itself look
    /// like a module that reads `i1040sd--2025` — and it did, printing `blockers.rs` as a citing
    /// site for a document it does not read. A self-exemption would have hidden that instead of
    /// removing it.
    fn xp() -> String {
        format!("design/forms/{}/", "extract")
    }

    /// ★ Every bucket has a heading, and the print order names every variant. Adding a `Who`
    /// variant without adding it to `ALL` reds here — the group would otherwise never print.
    #[test]
    fn every_bucket_has_a_heading() {
        assert_eq!(Who::ALL.len(), 4);
        let mut seen = BTreeSet::new();
        for w in Who::ALL {
            assert!(!w.heading().is_empty());
            assert!(seen.insert(w.heading()), "two buckets share a heading");
        }
        // An exhaustive match: a new variant must be added to `heading` AND to `ALL`, and this
        // asserts the second half.
        for w in [Who::January, Who::Build, Who::Decide, Who::NotOurs] {
            assert!(Who::ALL.contains(&w), "{w:?} is not printed");
        }
    }

    const HEADER: &str = "| form | common | added | removed | respelled | unpairable | lines that moved | boxes UNREAD | lines retired | lines introduced | line numbers whose MEANING changed | line numbers UNREAD | field map | the FORM |";

    fn printed(numeric: &str) -> String {
        format!(
            "{HEADER}\n|---|---|---|---|---|---|---|---|---|---|---|---|---|---|\n{numeric}\n\n\
             | form | emitted? | prior side | TY2026 side | cell |\n|---|---|---|---|---|\n\
             | `f1040` | yes | `f1040--2025` | **NO DRAFT** | **NO DRAFT** |\n"
        )
    }

    /// ★★★ **B1 — the column reader is watched refusing a RENAMED column.**
    ///
    /// The plant is the whole point: the generator gained six columns in two days, and a positional
    /// reader would have printed a plausible wrong cell across every one of those changes without a
    /// single test going red. So the reader indexes by NAME and is proved to refuse when a name it
    /// reads is gone.
    #[test]
    fn a_renamed_generator_column_is_refused_not_guessed() {
        let good = printed(
            "| `f6251` | 62 | 0 | 0 | 0 | 0 | 0 | 2 | 0 | 0 | 8 | 0 | transfers | **CHANGED** |",
        );
        let ps = parse_port_status(&good).expect("the real header parses");
        assert_eq!(
            ps.numeric["f6251"]["line numbers whose MEANING changed"],
            "8"
        );
        assert_eq!(ps.numeric["f6251"]["the FORM"], "**CHANGED**");
        assert_eq!(ps.excused["f1040"]["cell"], "**NO DRAFT**");

        // PLANT: exactly one column renamed, the same way FR-209 renamed `shape`.
        let planted = good.replace(
            "line numbers whose MEANING changed",
            "line numbers whose meaning changed",
        );
        let err = parse_port_status(&planted).expect_err("a renamed column must be REFUSED");
        assert!(
            err.contains("line numbers whose MEANING changed"),
            "the refusal names the missing column: {err}"
        );
        // …and a dropped table is refused too (one table, not two).
        let one = format!("{HEADER}\n|---|---|\n");
        assert!(parse_port_status(&one).is_err());
    }

    /// ★★★ **B1 — an `UNWITNESSED` axis is reported as UNMEASURED, never as clean.**
    ///
    /// The plant flips one cell's verdict word. `unchanged` must NOT produce an UNMEASURED row and
    /// `**UNWITNESSED**` must, because *"no axis found a change"* and *"an axis could not look"* are
    /// the same three pixels and are not the same fact.
    #[test]
    fn an_unwitnessed_axis_is_unmeasured_and_an_unchanged_one_is_not() {
        let row = |verdict: &str| {
            format!("| `f8949` | 202 | 0 | 0 | 0 | 0 | 0 | 16 | 0 | 0 | 0 | 2 | transfers | {verdict} |")
        };
        let unwit = parse_port_status(&printed(&row("**UNWITNESSED**"))).unwrap();
        let mut rep = Report::default();
        form_rows(2026, &unwit, &mut rep);
        assert!(
            rep.rows.iter().any(|r| r.state == State::Unmeasured
                && r.what.contains("f8949")
                && r.what.contains("UNWITNESSED")),
            "an UNWITNESSED axis must produce an UNMEASURED row: {:#?}",
            rep.rows
        );
        // PLANT REMOVED: the same row saying `unchanged` must NOT raise that row.
        let clean = parse_port_status(&printed(&row("unchanged"))).unwrap();
        let mut rep2 = Report::default();
        form_rows(2026, &clean, &mut rep2);
        assert!(
            !rep2
                .rows
                .iter()
                .any(|r| r.what.contains("NOT known to be unchanged")),
            "`unchanged` must not be reported as unmeasured"
        );
    }

    /// ★★★ **B1 — hiding an archived extract GROWS the right row.**
    ///
    /// `f1098e--2026` is archived at HEAD, so it is not a gap. Hide it from the listing and the row
    /// must appear, naming the family and its prior revision. A list that cannot notice a new
    /// blocker is a list, not an instrument.
    #[test]
    fn hiding_an_archived_extract_grows_an_archive_gap_row() {
        let names: Vec<String> = ["f1098e--2025.txt", "f1098e--2026.txt", "i1040gi--2025.txt"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let mut cit = Citations::default();
        for fam in ["f1098e", "i1040gi"] {
            cit.cited
                .entry(fam.to_string())
                .or_default()
                .insert("crates/btctax-core/src/x.rs".into());
        }
        let before = {
            let mut rep = Report::default();
            archive_rows(2026, &cit, &archived_over(&names), None, &mut rep);
            rep
        };
        assert!(
            before.rows.iter().any(|r| r.what.contains("`i1040gi`")),
            "i1040gi has no 2026 revision and must already be a gap"
        );
        assert!(
            !before.rows.iter().any(|r| r.what.contains("`f1098e`")),
            "f1098e--2026 is archived, so it must NOT be a gap: {:#?}",
            before.rows
        );
        // PLANT: hide the archived extract.
        let hidden: Vec<String> = names
            .iter()
            .filter(|n| *n != "f1098e--2026.txt")
            .cloned()
            .collect();
        let after = {
            let mut rep = Report::default();
            archive_rows(2026, &cit, &archived_over(&hidden), None, &mut rep);
            rep
        };
        assert_eq!(
            after.rows.len(),
            before.rows.len() + 1,
            "the list must GROW by exactly one row"
        );
        let grown = after
            .rows
            .iter()
            .find(|r| r.what.contains("`f1098e`"))
            .expect("the f1098e gap row must appear");
        assert!(
            grown.what.contains("f1098e--2025") && grown.who == Who::January,
            "the grown row names the newest archived revision and waits on the IRS: {grown:?}"
        );
    }

    /// ★★★ **B1 — a revision PIN is told apart from a revision TABLE, and the pin is UNMEASURED.**
    ///
    /// Two files, identical but for how they name the document. The prefix form notices a new
    /// revision (`January`, BLOCKED); the literal form never will (`Build`, UNMEASURED). Plant the
    /// prefix into a pin and the verdict must move.
    #[test]
    fn a_pinned_revision_is_unmeasured_and_a_prefix_one_clears_itself() {
        let prefix = vec![(
            "crates/btctax-core/src/tax/state_local_refund.rs".to_string(),
            format!("pub const EXTRACT_STEM: &str = \"{}i1040gi--\";", xp()),
        )];
        let mut rep = Report::default();
        revision_pin_rows(2026, &scan_citations(&prefix), &mut rep);
        assert_eq!(rep.rows.len(), 1, "{:#?}", rep.rows);
        assert_eq!(rep.rows[0].who, Who::January);
        assert_eq!(rep.rows[0].state, State::Blocked);
        assert!(
            rep.rows[0].what.contains("PER-REVISION"),
            "{:?}",
            rep.rows[0]
        );

        // PLANT: the same module pinned to one literal revision instead.
        let pinned = vec![(
            "crates/btctax-core/src/tax/capital_loss_carryover.rs".to_string(),
            format!("pub const SOURCE: &str = \"{}i1040sd--2025.txt\";", xp()),
        )];
        let mut rep2 = Report::default();
        revision_pin_rows(2026, &scan_citations(&pinned), &mut rep2);
        assert_eq!(rep2.rows.len(), 1, "{:#?}", rep2.rows);
        assert_eq!(rep2.rows[0].who, Who::Build);
        assert_eq!(
            rep2.rows[0].state,
            State::Unmeasured,
            "a pin does not know the year moved — that is UNMEASURED, not fine"
        );
        assert!(rep2.rows[0].what.contains("SHIPPED"), "{:?}", rep2.rows[0]);

        // A DOC-comment citation is evidence, not code, and must pin nothing.
        let doc = vec![(
            "crates/btctax-core/src/x.rs".to_string(),
            format!("/// see {}i1040gi--2025.txt:46610", xp()),
        )];
        let c = scan_citations(&doc);
        assert!(c.pinned.is_empty() && c.cited.contains_key("i1040gi"));
    }

    /// ★★★ **B1 — every gate anchor is live, and a moved anchor is refused.**
    ///
    /// The anchor is what welds a gate to the refusal it raises. If `return_refuse.rs` moves the
    /// line, the attribution becomes a guess — so the real source is checked at HEAD, and a planted
    /// edit to the anchor text must be refused rather than silently un-attributed.
    #[test]
    fn every_gate_anchor_still_resolves_and_a_moved_one_is_refused() {
        let src =
            std::fs::read_to_string(root().join("crates/btctax-core/src/tax/return_refuse.rs"))
                .expect("return_refuse.rs");
        let body = non_test(&src);
        assert!(
            body.len() < src.len(),
            "the test module must be excluded, or a fixture would count as a raise"
        );
        let mut anchored = 0usize;
        for g in gates() {
            if let Some(a) = g.anchor {
                let r = reason_after(body, a).unwrap_or_else(|e| panic!("{}: {e}", g.name));
                assert!(!r.is_empty());
                anchored += 1;
                // PLANT: the anchor text moves.
                let planted = body.replace(a, "/* moved */");
                assert!(
                    reason_after(&planted, a).is_err(),
                    "a moved anchor must be REFUSED, not silently dropped: {a}"
                );
            }
        }
        assert!(anchored >= 2, "at least two gates are anchored");
        // The §111(a) gate is the year-specific one this whole axis exists to find.
        assert_eq!(
            reason_after(body, "Err(NotUsable::NoArchivedRevision { tax_year }) =>").unwrap(),
            "StateAndLocalRefundWorksheetNotComputed"
        );
    }

    /// ★★★ **B1 — ARCHIVED is not TRANSCRIBED.**
    ///
    /// The kill for the overclaim the `i1040gi--2026` plant falsified. Both archived revisions are
    /// transcribed at HEAD, so the join is silent; the target year is neither, and the gate says
    /// TRANSCRIBED rather than archived — which is what the plant proved it had to say.
    #[test]
    fn the_transcription_probe_is_not_the_archive() {
        let arch = archived();
        let tags = arch.get("i1040gi").expect("i1040gi is archived");
        let mut probed = 0usize;
        for tag in tags {
            let Ok(y) = tag.parse::<i32>() else { continue };
            assert_eq!(
                transcribed("i1040gi", y),
                Some(true),
                "i1040gi--{y} is archived and must be transcribed (the suite's own rule)"
            );
            probed += 1;
        }
        assert!(probed >= 2, "only {probed} archived revisions probed");
        // The target year is NEITHER archived NOR transcribed — the two facts, told apart.
        assert!(!tags.contains(&"2026".to_string()));
        assert_eq!(transcribed("i1040gi", 2026), Some(false));
        // A family with no probe says so instead of defaulting to "fine".
        assert_eq!(transcribed("i1040sd", 2026), None);
        // …and the gate's own words say TRANSCRIBED, so archiving alone cannot look like clearing it.
        let why = gates()
            .into_iter()
            .find(|g| g.name.contains("111(a)"))
            .expect("the §111(a) gate")
            .probe;
        let msg = why(2026).expect_err("shut for TY2026");
        assert!(
            msg.contains("TRANSCRIBED") && msg.contains("archiving the extract reds"),
            "the gate must not claim the extract is missing: {msg}"
        );
    }

    fn refuse_src() -> String {
        std::fs::read_to_string(root().join("crates/btctax-core/src/tax/return_refuse.rs"))
            .expect("return_refuse.rs")
    }

    fn names(v: &[&str]) -> BTreeSet<String> {
        v.iter().map(|s| (*s).to_string()).collect()
    }

    /// ★★★ **THE B1 KILL, WRITTEN OUTSIDE THE SCANNER'S OWN VOCABULARY (B3 I-1).**
    ///
    /// The plant this replaces read the year as `ri.tax_year > 0` — **the needle the old scanner
    /// already searched for** — so it could only ever confirm what that scanner already saw, and the
    /// blindness it existed to kill survived it untouched. `HARNESS.md` B1's ★★ clause is that an
    /// honest kill *"cannot be written without discovering the blindness"*. This one reads the year
    /// the way the three invisible sites do — **through a function parameter** — and asserts against
    /// literal copies of the two old needles that the old scanner could not have seen it at all.
    #[test]
    fn a_year_read_through_a_parameter_is_found_and_the_old_needles_could_not_see_it() {
        /// The scan this replaces, verbatim: `for needle in ["ri.tax_year", "NotUsable::"]`.
        const OLD_NEEDLES: [&str; 2] = ["ri.tax_year", "NotUsable::"];
        let planted = r#"
pub fn section_68_ish(year: i32, itemizes: bool) -> Option<Refusal> {
    if !itemizes {
        return None;
    }
    match crate::tax::tables::section_68_status(year) {
        Section68Status::NotApplicable => None,
        Section68Status::Gated(g) => refuse(
            RefuseReason::ItemizedDeductionLimitationNotComputed { year },
            format!("this return is for tax year {year}, it ITEMIZES, over {}", g.threshold),
        ),
    }
}
"#;
        for n in OLD_NEEDLES {
            assert!(
                !planted.contains(n),
                "a plant that speaks the checker's language is not a kill, and this one names {n}"
            );
        }
        let cen = refusal_census(planted);
        assert_eq!(cen.sites.len(), 1, "{:?}", cen.sites);
        let s = &cen.sites[0];
        assert_eq!(s.variant, "ItemizedDeductionLimitationNotComputed");
        assert!(
            s.reads_year(),
            "the parameter idiom is invisible again: {s:?}"
        );
        assert_eq!(
            s.keyed,
            vec!["section_68_status".to_string()],
            "the site names a gate, so it must be ATTRIBUTED rather than UNCLASSIFIED: {s:?}"
        );
        assert!(cen.unclassified().is_empty(), "{:?}", cen.unclassified());

        // ★★★ **And the kill must bind to the PARAMETER READ, not to the gate's name.** Measured while
        //     writing this: dropping `year` from [`YEAR_READ`] — reinstating the exact old blindness —
        //     left every assertion above GREEN, because the site was still reached through
        //     `section_68_status` and through the payload. A kill that can pass for another reason is
        //     not a kill. So the same plant is re-run with the year-keyed call made opaque: the bare
        //     `year` is then the only signal there is.
        let opaque = planted.replace(
            "crate::tax::tables::section_68_status(year)",
            "an_unknown_helper(year)",
        );
        let o = refusal_census(&opaque);
        assert_eq!(o.sites.len(), 1, "{:?}", o.sites);
        assert!(
            o.sites[0].keyed.is_empty(),
            "the opaque plant must name no gate, or it proves nothing: {:?}",
            o.sites[0]
        );
        assert!(
            o.sites[0].reads_year(),
            "a tax year read through a function PARAMETER is invisible — the defect B3 I-1 found: {:?}",
            o.sites[0]
        );

        // ★★ …and the read must be the site's OWN. A refusal whose `format!` names the year must not
        //    make the NEXT rule look year-conditional: letting a message bleed forward called 34 of
        //    the 95 real sites year-reading (measured), `BrokerReportingMixed` and
        //    `Schedule1aTipsFromTradeOrBusiness` among them — each inheriting the year from the
        //    refusal above it. A message that names the year decides nothing.
        let bleed = r#"
pub fn two_rules(ri: &ReturnInputs) -> Option<Refusal> {
    if ri.wages > 0 {
        return refuse(
            RefuseReason::First { year: ri.tax_year },
            format!(
                "this return is for tax year {year}",
                year = ri.tax_year,
            ),
        );
    }
    if ri.tips > 0 {
        return refuse(RefuseReason::Second, "nothing in this rule reads the year");
    }
    None
}
"#;
        let two = refusal_census(bleed);
        assert_eq!(two.sites.len(), 2, "{:?}", two.sites);
        assert!(two.sites[0].reads_year(), "{:?}", two.sites[0]);
        assert!(
            !two.sites[1].reads_year(),
            "the previous refusal's MESSAGE bled forward into this rule: {:?}",
            two.sites[1]
        );
    }

    /// ★★★ **A site the census cannot attribute becomes a ROW — it does not disappear.**
    ///
    /// The old census computed its UNCLASSIFIED set *over the sites its two needles had already
    /// found*, so the only sites that could be reported were the ones already counted: **the
    /// denominator was the defect.** Here both raise sites are enumerated first — one year-reading
    /// with no gate to name, one year-blind — and each lands in its own bucket.
    #[test]
    fn a_year_reading_site_naming_no_gate_is_unclassified_and_never_vanishes() {
        let planted = r#"
pub fn screen_invented(ri: &ReturnInputs, year: i32) -> Option<Refusal> {
    if year >= 2030 {
        return refuse(RefuseReason::Invented, "no year-keyed gate governs this one");
    }
    if ri.wages > 0 {
        return refuse(RefuseReason::YearBlind, "nothing in this rule reads the year");
    }
    None
}
"#;
        let cen = refusal_census(planted);
        assert_eq!(cen.sites.len(), 2, "{:?}", cen.sites);
        assert_eq!(cen.unclassified(), names(&["Invented"]));
        assert_eq!(
            cen.blind()
                .iter()
                .map(|s| s.variant.as_str())
                .collect::<Vec<_>>(),
            vec!["YearBlind"],
            "the blind site must still be COUNTED — it is the denominator"
        );
        // And the row the report prints for it says UNCLASSIFIED, not "fine".
        let mut rep = Report::default();
        for var in &cen.unclassified() {
            rep.push(
                Who::Build,
                State::Unmeasured,
                format!("`RefuseReason::{var}`"),
                "now",
                "plant".into(),
            );
        }
        assert_eq!(rep.unmeasured(), 1);
    }

    /// ★★★ **B1 — the completeness check is EXHAUSTIVE against the TYPE, in both directions.**
    ///
    /// What this replaces required only that three hand-typed variant names appear
    /// (`vars.contains(want)`), beside a `RefuseReason` set that grew by six in one day. Containment
    /// is the weaker direction: it cannot notice a missing fourth — and three of the four variants
    /// that carry a `year` payload were missing. The check now reads the payload off the enum, so a
    /// fifth one joins the census with no edit, and one with no located raise site is a ROW.
    #[test]
    fn a_year_payload_variant_with_no_raise_site_is_reported_not_assumed_year_agnostic() {
        let planted = r#"
pub enum RefuseReason {
    /// A refusal that cannot be raised without reading the tax year.
    NeverRaisedHere { year: i32 },
    /// A refusal with no year anywhere in it.
    Plain,
}

pub fn screen(ri: &ReturnInputs) -> Option<Refusal> {
    if ri.wages > 0 {
        return refuse(RefuseReason::Plain, "x");
    }
    None
}
"#;
        let cen = refusal_census(planted);
        assert_eq!(cen.variants, names(&["NeverRaisedHere", "Plain"]));
        assert_eq!(cen.year_payload, names(&["NeverRaisedHere"]));
        assert_eq!(
            cen.unlocated_year_payload(),
            names(&["NeverRaisedHere"]),
            "a variant that cannot be raised without the year, and no raise site found, must be \
             REPORTED — it is the §68 hole"
        );
        // The other direction, so the check is watched discriminating: drop the payload and the same
        // text raises no complaint. The signal is the TYPE, not the variant's name.
        let no_payload = planted.replace("NeverRaisedHere { year: i32 }", "NeverRaisedHere");
        let cen2 = refusal_census(&no_payload);
        assert_eq!(cen2.variants, names(&["NeverRaisedHere", "Plain"]));
        assert!(cen2.year_payload.is_empty());
        assert!(cen2.unlocated_year_payload().is_empty());
    }

    /// ★★ **The census at HEAD.** Every number here was MEASURED and is pinned to a literal before any
    /// derived claim is made from it (FR-230: an expectation derived from the thing it checks measures
    /// nothing).
    #[test]
    fn the_refusal_census_enumerates_every_raise_site_at_head() {
        let src = refuse_src();
        let cen = refusal_census(&src);

        // Two independent derivations of the variant set must agree: `census_join` reads declaration
        // lines, `refusal_census` parses the enum body.
        let joined: BTreeSet<String> = crate::census_join::variant_paths(&src, "RefuseReason")
            .iter()
            .map(|p| p.trim_start_matches("RefuseReason::").to_string())
            .collect();
        assert_eq!(cen.variants, joined, "the two derivations disagree");
        assert_eq!(cen.variants.len(), 135, "measured at HEAD 2026-09-14");
        assert!(
            cen.sites.len() >= 95,
            "95 raise sites measured at HEAD; found {}",
            cen.sites.len()
        );
        assert!(
            cen.reading().len() >= 11,
            "11 year-reading sites measured at HEAD; found {} — {:?}",
            cen.reading().len(),
            cen.by_variant().keys().collect::<Vec<_>>()
        );

        // ★★★ The five variants that CANNOT be raised without the year, by name, and every one
        //     located. The old census attributed ONE of them.
        //
        // ★★ `CharitableFloorNotComputed` is the fifth, and it is here as EVIDENCE rather than as
        //    maintenance: it landed between the I-1 agent starting and this fold, and BOTH halves of
        //    the new instrument moved for it without anyone going looking. The classification test
        //    reported it UNCLASSIFIED (answered by declaring its gate in `gates()`, since
        //    `charitable_floor_status` really does govern it), and this derivation picked its
        //    `year: i32` payload up on its own. The old containment-based scanner would have stayed
        //    green on both — which is the whole reason the exhaustive form replaced it.
        //
        // ★ So this literal is a PIN, not a list that decides anything: the set comes from the enum
        //   body. Widening it is the deliberate act of recording that a new year-carrying refusal
        //   exists; a variant appearing here unexpectedly is the signal, not the failure.
        assert_eq!(
            cen.year_payload,
            names(&[
                "BrokerAnswerUnread",
                "BrokerReportingUnanswered",
                "CharitableFloorNotComputed",
                "ItemizedDeductionLimitationNotComputed",
                "Schedule1aNotOnThisYearsReturn",
            ])
        );
        assert!(
            cen.unlocated_year_payload().is_empty(),
            "a `year`-payload variant has no located raise site: {:?}",
            cen.unlocated_year_payload()
        );

        let by = cen.by_variant();
        // The five the old scanner did find, still found…
        for want in [
            "ReturnInputsYearNotStated",
            "Schedule1aNotOnThisYearsReturn",
            "StateAndLocalRefundWorksheetNotComputed",
            "Pub525ItemizedDeductionRecovery",
            "StateLocalRefundFactsWithoutARefund",
        ] {
            assert!(by.contains_key(want), "regression: {want} lost");
        }
        // …and the three it could not see, the §68 one being the reason this exists.
        for want in [
            "ItemizedDeductionLimitationNotComputed",
            "BrokerReportingUnanswered",
            "BrokerAnswerUnread",
        ] {
            assert!(by.contains_key(want), "{want} is invisible again");
        }
        assert!(
            by["ItemizedDeductionLimitationNotComputed"]
                .iter()
                .any(|s| s.keyed.iter().any(|k| k == "section_68_status")),
            "§68 must be ATTRIBUTED to its gate, not merely noticed: {:?}",
            by["ItemizedDeductionLimitationNotComputed"]
        );
        // ★ And the precision half: `BrokerReportingMixed` sits between two year-payload refusals and
        //   its own message says `TY{year}`, while its decision reads no year. It must stay BLIND, or
        //   the instrument is counting prose and bleed.
        assert!(
            !by.contains_key("BrokerReportingMixed"),
            "a refusal whose decision reads no year must not be called year-reading"
        );
    }

    /// ★★ The attribution keys are DERIVED from `gates()`, and the §68 gate is a real probe.
    #[test]
    fn the_year_keyed_markers_come_from_the_gates_and_section_68_is_one_of_them() {
        let m = year_keyed_markers();
        for g in gates() {
            if let Some(k) = g.keyed_by {
                assert!(
                    m.contains(&k),
                    "{k} is declared by a gate and not recognised"
                );
            }
        }
        assert!(m.contains(&"section_68_status"), "{m:?}");
        // Pinned literals (FR-230), then the derivation: the §68 gate is SHUT for TY2026 and OPEN for
        // both earlier bundled years, so `gate_rows` files it as year-specific rather than universal.
        let g68 = gates()
            .into_iter()
            .find(|g| g.name.contains("§68"))
            .expect("the §68 gate");
        assert!((g68.probe)(2024).is_ok(), "§68 does not apply before 2026");
        assert!((g68.probe)(2025).is_ok());
        let why = (g68.probe)(2026).expect_err("§68 applies to TY2026");
        assert!(
            why.contains("384350") && why.contains("TRANSCRIBED"),
            "the row must name the threshold and say January does not clear it: {why}"
        );
        assert!(
            (g68.probe)(2027).is_err(),
            "a year with no transcribed Schedule A must fail CLOSED"
        );
        assert_eq!(
            reason_after(
                non_test(&refuse_src()),
                "crate::tax::tables::section_68_status(year) {"
            )
            .unwrap(),
            "ItemizedDeductionLimitationNotComputed"
        );
    }

    /// ★ The lexer: a year in a comment or a string decides nothing, and `"… a preparer",` does not
    /// open a raw string and swallow the code after it.
    #[test]
    fn code_lines_blanks_prose_and_survives_an_r_before_a_quote() {
        let src = "// year\nlet a = \"tax_year 2026\"; // year\nlet b = \"a preparer\", year;\n";
        let got = code_lines(src);
        assert_eq!(got[0].trim(), "");
        assert_eq!(got[1].trim(), "let a = ;");
        assert!(
            got[2].contains("year"),
            "the code after `preparer\",` was swallowed: {:?}",
            got[2]
        );
        assert!(!word_in(&got.join("\n"), "tax_year"));
    }

    /// ★★★ **B1 — the tags come out of the work list's own command, MARKDOWN AND ALL.**
    ///
    /// This is the kill for a defect this module actually shipped for one run. The committed document
    /// writes the command with bold *outside* the code span, and the first version of the reader
    /// returned `2026-DRAFT\`**` — a tag that passes every sanity check, pairs against no archived
    /// stem, and turned all fifteen numeric rows into *"NO form axis ran"* with no error. The
    /// emphasised spelling is asserted here against the REAL document, not only a synthetic one.
    #[test]
    fn tags_are_stripped_of_markdown_emphasis() {
        let want = Some(("2025".to_string(), "2026-DRAFT".to_string()));
        // plain, bold-around-code (what the document actually writes), and trailing punctuation
        assert_eq!(
            work_list_tags("run `cargo run -p xtask -- port-status 2025 2026-DRAFT` (FR-50)"),
            want
        );
        assert_eq!(
            work_list_tags(
                "**Regenerate with `cargo run -p xtask -- port-status 2025 2026-DRAFT`** (FR-50)"
            ),
            want
        );
        assert_eq!(work_list_tags("… port-status 2025 2026-DRAFT."), want);
        assert_eq!(work_list_tags("no command here"), None);
        let doc = std::fs::read_to_string(root().join("design/TY2026_WORK_LIST.md")).unwrap();
        assert_eq!(
            work_list_tags(&doc),
            want,
            "the committed work list's declared tags, read through its own emphasis"
        );
    }

    /// ★★ The `when` cell names a package or a month, and never chops a citation mid-word.
    #[test]
    fn the_when_cell_takes_the_clause_that_names_a_date() {
        assert_eq!(
            when_clause("TY2026 revision not released; draft archived; January 2027 package"),
            "January 2027 package"
        );
        // Form 8889's real reason has no package clause — the FIRST clause is the answer, not the
        // last, which used to print "the §223(b) figures ARE published (Rev. Proc".
        assert_eq!(
            when_clause(
                "TY2026 revision not released; the §223(b) figures ARE published (Rev. Proc. \
                 2025-19), but the form itself is not"
            ),
            "TY2026 revision not released"
        );
    }

    /// ★★ The owner axis takes its rows from the owner's own table, and a RULED row is not pending.
    #[test]
    fn the_owner_axis_reads_the_pending_decisions_and_skips_the_ruled_ones() {
        let doc = "### OWNER DECISIONS PENDING — x\n\n| # | decision | a | b |\n|---|---|---|---|\n\
                   | **S1** | **Un-pause a rehearsal** — detail | x | y |\n\
                   | **S2** | **Two answers** — appraisal above $5,000 is an owner action | x | y |\n\
                   | **S6** | **RULED 2026-09-06** — done | x | y |\n\n## next section\n";
        let mut rep = Report::default();
        owner_rows(doc, &mut rep);
        let decide: Vec<&Row> = rep.rows.iter().filter(|r| r.who == Who::Decide).collect();
        assert_eq!(decide.len(), 2, "S1 and S2 pending, S6 ruled: {decide:#?}");
        assert!(decide.iter().any(|r| r.what.starts_with("S1")));
        assert!(
            rep.rows.iter().any(|r| r.who == Who::NotOurs
                && r.what.contains("QUALIFIED APPRAISAL")
                && r.when == "before filing"),
            "the appraisal row is derived from S2's own text: {:#?}",
            rep.rows
        );
        // PLANT: the section heading is gone — the axis must SAY it did not run.
        let mut rep2 = Report::default();
        owner_rows("nothing here", &mut rep2);
        assert!(rep2.rows.is_empty());
        assert!(rep2.notes.iter().any(|n| n.contains("did not run")));
    }

    /// ★★★ **The whole command, at HEAD.** Not a golden — a shape assertion, so it cannot be made
    /// green by pasting output: every bucket that must be non-empty is, the TY2026 row set names the
    /// 1040 and the 1040 instructions (FR-181), and the §111(a) gate is attributed.
    #[test]
    fn the_command_answers_for_ty2026_at_head() {
        let rep = collect(2026).expect("blockers 2026");
        assert!(rep.rows.len() > 30, "{} rows", rep.rows.len());
        for who in [Who::January, Who::Build, Who::Decide, Who::NotOurs] {
            assert!(rep.count(who) > 0, "{who:?} is empty");
        }
        assert!(rep.unmeasured() > 0, "nothing is UNMEASURED — suspicious");
        let all = rep.render(2026);
        for want in [
            "`f1040` ",  // the return itself is not bundled
            "`i1040gi`", // FR-181's instructions gap
            "StateAndLocalRefundWorksheetNotComputed",
            "QUALIFIED APPRAISAL",
            "**UNMEASURED**",
            // ★★★ THE TAG KILL AT THE INTEGRATION LEVEL. Every generated form axis reaching the
            //     report at all depends on the work list's tags parsing; a mis-stripped tag pairs
            //     against nothing and prints "NO form axis ran" on every row, with no error. If this
            //     phrase is present, `port-status` really did compute pairs.
            "the FORM = **CHANGED**",
            "line numbers UNREAD",
        ] {
            assert!(all.contains(want), "the report never mentions {want:?}");
        }
        assert!(
            all.contains("15 stems have a numeric row, 6 are in the excused table"),
            "the form axis must pair the fifteen stems the generator pairs — a broken tag silently \
             sends all 21 to the excused table"
        );
        // ★ Asserted on the COUNT in the notes, not on the absence of the word: the notes line
        //   always prints the word, so `!contains("UNCLASSIFIED")` was a test that could never pass.
        //
        // ★★★ It read `0 UNCLASSIFIED site group(s)` until B3 I-1, and that zero was the symptom: the
        //     census could only classify sites its two needles had already found, so the three
        //     year-reading refusals it could not see were not UNCLASSIFIED — they were absent. With
        //     every raise site enumerated the real count is **two**, both in `screen_broker_reporting`,
        //     which reads the year through a parameter and a `regime` no `Gate` probes. Pinned to the
        //     names rather than relaxed to a floor: a THIRD one appearing still reds here.
        assert!(
            all.contains(
                "2 UNCLASSIFIED variant(s) [BrokerAnswerUnread, BrokerReportingUnanswered]"
            ),
            "the UNCLASSIFIED set moved — look at it, do not widen this assertion"
        );
        assert!(
            all.contains("0 `year`-payload variant(s) with no located raise site"),
            "a refusal that cannot be raised without the tax year has no raise site this census can \
             find — the §68 hole, reopened"
        );
        // ★★★ §68 REACHES THE TY2026 ANSWER. It appeared nowhere in the 58-row prediction frozen as
        //     stage 2's: not as a row, not in `gates()`, and the notes line asserted the opposite of
        //     it. This is the integration-level check that the year's most year-specific refusal is
        //     in the list the operator reads.
        for want in [
            "§68 ITEMIZED DEDUCTIONS WORKSHEET",
            "ItemizedDeductionLimitationNotComputed",
            "the worksheet must be TRANSCRIBED",
        ] {
            assert!(
                all.contains(want),
                "the TY2026 answer never mentions {want:?}"
            );
        }
        // A year this build does not bundle is answered, not panicked on.
        let far = collect(2031);
        assert!(
            far.is_err()
                || far
                    .unwrap()
                    .rows
                    .iter()
                    .any(|r| r.what.contains("not a bundled year"))
        );
    }
}
