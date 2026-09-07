//! ★★★ **R2.2 — THE CENSUS JOIN: *a line btctax cannot take is either ANNOUNCED or REFUSED, never
//! silent*** (`SPEC_interview.md` R2 part 2, task T3).
//!
//! Every `rule = "unmodeled"` entry on the seven forms the interview reaches carries
//! `covered_by = "Advisory::X" | "RefuseReason::Y" | "QuestionId::Z"`, and this module joins each
//! against the enum's own SOURCE. That much was already the plan; the part that makes it a guarantee
//! rather than a formality is the **direction rule**.
//!
//! **The direction rule, and why "announced or refused" is not one guarantee but two.**
//! A deduction or credit left blank forgoes money lawfully, and a sentence saying btctax did not try
//! is an honest cover for it. An income or additional-tax line left blank is an omission that
//! UNDERSTATES the tax, and no sentence covers that. So:
//!
//! - an **`Understates`** entry may be covered only by a `QuestionId` or a `RefuseReason` — an
//!   `Advisory` on one REDS, while the same advisory on an `Overstates` entry stays green;
//! - a **`QuestionId`** cover is checked for **REACH**, not existence: the covering question's
//!   prompt must contain the entry's own line-caption keyword. *A variant that exists is not a
//!   cover; a question the filer can answer "No" to without ever reading the line's name is not a
//!   cover.* In particular the residual scope attestation covers exactly the lines its prompt
//!   enumerates, and its trailing *"or anything else it never asked about"* covers **nothing**;
//! - a **`RefuseReason`** cover is existence-checked, because a refusal stops the return and
//!   nothing is filed silently.
//!
//! **And the direction is DERIVED, never typed — the VALUE as well as the heading.** Each map
//! carries a `[direction]` table of the form's own part headings; every caption is asserted VERBATIM
//! against `design/forms/extract/<stem>--<year>.txt` at a pinned line before the table is used, and
//! an entry the table cannot place **reds** — it is never defaulted to `Overstates`.
//!
//! ★★★ A block states **no direction of its own**. The T3 seam review found `direction` to be the
//! one field of a block that was never read off the form, and demonstrated the consequence: two TOML
//! keys moved Form 1040 line 1b — a numbered INCOME line — under an advisory cover with every
//! instrument green. So the value left the maps entirely ([`DirectionBlock`] carries
//! `deny_unknown_fields`, which makes a `direction = "…"` key a parse error), and the sign is read
//! from the caption in ONE place, [`DIRECTION_OF_CAPTION`], each row citing the form's own total
//! sentence. Two further rules close the gap around it: a block that heads a LINE RANGE may not be
//! graded `NoDollar` (a range is the claim that the block carries dollars), and the caption ⇄
//! reading join is asserted in **both** directions, so a dead reading reds rather than waiting to be
//! inherited.
//!
//! ★ **Why it lives in xtask.** It reads `design/forms/extract/` (outside every published crate) AND
//! `btctax-core`'s enum sources AND `btctax-forms`'s maps. An `include_str!` reaching out of a
//! published crate ships a tarball that builds in the workspace and is broken for everyone else,
//! with exit 0 — the trap `crate-publishing-state` records. So the reading travels here, exactly as
//! `line_coverage_check.rs` does.

use btctax_forms::{CensusDecision, DirectionBlock, SubtractSentence};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Which way a wrongly-blank line moves the tax (R2.2's DIRECTION RULE).
///
/// ★★★ **It lives HERE, in the checker, and deliberately not in `btctax-forms`.** A map has no
/// vocabulary for it, cannot name it, and cannot parse it: the direction of a block is a READING of
/// the caption the form prints, taken once in [`DIRECTION_OF_CAPTION`] below.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Income, or an addition to tax. A blank here is a FALSE STATEMENT, so an `Advisory` may never
    /// cover it — only a `QuestionId` the filer actually reads or a `RefuseReason` that stops the
    /// return.
    Understates,
    /// A deduction, credit or payment. A blank forgoes money lawfully, and a sentence saying btctax
    /// did not try is an honest cover.
    Overstates,
    /// The cell carries no dollar at all — a date, a name, a checkbox, an election btctax does not
    /// offer. It can move the tax in neither direction.
    NoDollar,
}

/// One row of [`DIRECTION_OF_CAPTION`]: the block caption the form prints, the direction a blank
/// under it moves the tax, and **the form's own sentence that justifies the sign**.
pub type Reading = (&'static str, Direction, &'static str);

/// ★★★ **THE ONE PLACE A DIRECTION IS READ.**
///
/// The seam review of T3 found that `direction` was the single field of a `[[direction]]` block that
/// was never read off the form: the caption was asserted verbatim at a pinned extract line, the
/// range ends and the overlap were asserted, D11's flip was parsed out of the form's own sentence —
/// and the sign itself was a bare judgement typed beside them, checked by nothing. Two TOML keys
/// then moved Form 1040 line **1b** (*Household employee wages not reported on Form(s) W-2*, a
/// numbered income line) under an `Advisory` cover with `census-join` printing its success line.
///
/// The value therefore left the maps. A block records only what the form PRINTS; the sign is read
/// from its caption here, once, with **evidence**: the total sentence the block's lines are combined
/// into, and the line of the Form 1040 that total is carried to. *Combine … This is your additional
/// income … line 8* is income, so a blank UNDERSTATES; *Add lines 1 through 4, 5a, 5b, and 7 … line
/// 20* is a nonrefundable-credit total, so a blank OVERSTATES. A caption whose cells carry no amount
/// at all cites the sentence that shows it (dates, an address, a signature block).
///
/// **Both directions of the join are asserted** ([`tests::every_committed_caption_has_exactly_one_reading_and_every_reading_is_used`]):
/// every committed block's caption has exactly one reading here, and every reading names a caption
/// at least one map carries — so a dead reading reds instead of waiting to be inherited by the next
/// revision. Every evidence sentence is itself asserted to be printed in one of the archived
/// extracts, so the justification cannot rot into a comment either.
pub const DIRECTION_OF_CAPTION: &[Reading] = &[
    // ── Form 1040. ───────────────────────────────────────────────────────────────────────────────
    (
        "For the year Jan. 1",
        Direction::NoDollar,
        "or other tax year beginning",
    ),
    (
        "Foreign country name",
        Direction::NoDollar,
        "Foreign country name Foreign province/state/county Foreign postal code",
    ),
    (
        "Filing Status",
        Direction::NoDollar,
        "If treating a nonresident alien or dual-status alien spouse as a U.S. resident for the \
         entire tax year, check the box and enter their name",
    ),
    (
        "Income",
        Direction::Understates,
        "Add lines 1z, 2b, 3b, 4b, 5b, 6b, 7, and 8. This is your total income",
    ),
    (
        "Tax and",
        Direction::Understates,
        "Add lines 22 and 23. This is your total tax",
    ),
    (
        "Payments",
        Direction::Overstates,
        "Add lines 25d, 26, and 32. These are your total payments",
    ),
    (
        "Refund",
        Direction::Overstates,
        "If line 33 is more than line 24, subtract line 24 from line 33. This is the amount you \
         overpaid",
    ),
    (
        "Amount",
        Direction::Understates,
        "Subtract line 33 from line 24. This is the amount you owe.",
    ),
    (
        "Third Party",
        Direction::NoDollar,
        "Do you want to allow another person to discuss this return with the IRS?",
    ),
    (
        "Sign",
        Direction::NoDollar,
        "Under penalties of perjury, I declare that I have examined this return",
    ),
    // ── Schedule 1. ──────────────────────────────────────────────────────────────────────────────
    // ★ The unnumbered reconciliation cell above Part I. The amount it asks for is one that does NOT
    //   belong in income, so leaving it blank cannot omit income — it forgoes a reduction.
    (
        "For 2024, enter the amount reported to you on Form(s) 1099-K",
        Direction::Overstates,
        "enter the amount reported to you on Form(s) 1099-K that was included in error or for \
         personal items sold at a loss",
    ),
    (
        "Part I       Additional Income",
        Direction::Understates,
        "Combine lines 1 through 7 and 9. This is your additional income. Enter here and on Form \
         1040, 1040-SR, or 1040-NR, line 8",
    ),
    (
        "Part II     Adjustments to Income",
        Direction::Overstates,
        "Add lines 11 through 23 and 25. These are your adjustments to income. Enter here and on \
         Form 1040, 1040-SR, or 1040-NR, line 10",
    ),
    // ── Schedule 2. Both parts are ADDITIONS TO TAX: line 3 goes to Form 1040 line 17 and line 21
    //    to line 23, both of which the 1040 ADDS into total tax. ───────────────────────────────────
    (
        "Part I       Tax",
        Direction::Understates,
        "Add lines 1z and 2. Enter here and on Form 1040, 1040-SR, or 1040-NR, line 17",
    ),
    (
        "Part II      Other Taxes",
        Direction::Understates,
        "Add lines 4, 7 through 16, 18, and 19. These are your total other taxes. Enter here and on \
         Form 1040",
    ),
    // ── Schedule 3. Credits and payments: line 8 goes to Form 1040 line 20 (nonrefundable credits,
    //    SUBTRACTED at line 21) and line 15 to line 31 (payments). ────────────────────────────────
    (
        "Part I       Nonrefundable Credits",
        Direction::Overstates,
        "Add lines 1 through 4, 5a, 5b, and 7. Enter here and on Form 1040, 1040-SR, or 1040-NR, \
         line 20",
    ),
    (
        "Part II      Other Payments and Refundable Credits",
        Direction::Overstates,
        "Add lines 9 through 12 and 14. Enter here and on Form 1040, 1040-SR, or 1040-NR, line 31",
    ),
    // ── Schedule A — the whole-form key. Its total is a DEDUCTION carried to Form 1040 line 12. ──
    (
        "Itemized Deductions",
        Direction::Overstates,
        "Add the amounts in the far right column for lines 4 through 16",
    ),
    // ── Schedule B — the whole-form key. Its total is INTEREST carried to Form 1040 line 2b. (The
    //    one line the form itself subtracts, line 3, is flipped by the `[[subtracts]]` rule.) ─────
    (
        "Interest and Ordinary Dividends",
        Direction::Understates,
        "Subtract line 3 from line 2. Enter the result here and on Form 1040 or 1040-SR, line 2b",
    ),
    // ── Schedule D — all three parts are capital GAIN, carried to Form 1040 line 7. ──────────────
    (
        "Part I        Short-Term Capital Gains and Losses",
        Direction::Understates,
        "Net short-term capital gain or (loss). Combine lines 1a through 6 in column (h)",
    ),
    (
        "Part II       Long-Term Capital Gains and Losses",
        Direction::Understates,
        "Net long-term capital gain or (loss). Combine lines 8a through 14 in column (h)",
    ),
    (
        "Part III      Summary",
        Direction::Understates,
        "Combine lines 7 and 15 and enter the result",
    ),
];

/// A form's `[direction]` table, paired with the readings that grade its captions.
///
/// ★ One parameter rather than two so the rule and its vocabulary travel together — and so a caller
/// cannot pass a table of blocks with no way to grade them.
#[derive(Clone, Copy)]
pub struct DirectionTable<'a> {
    /// The blocks the map commits, verbatim.
    pub blocks: &'a [DirectionBlock],
    /// The readings that grade their captions — [`DIRECTION_OF_CAPTION`] in production.
    pub readings: &'a [Reading],
}

/// The seven forms the interview reaches (R2.2). Map stem → the `irs_stem` its extract is filed
/// under (they differ for Schedule D).
const INTERVIEW_FORMS: &[(&str, &str)] = &[
    ("f1040", "f1040"),
    ("f1040s1", "f1040s1"),
    ("f1040s2", "f1040s2"),
    ("f1040s3", "f1040s3"),
    ("f1040sa", "f1040sa"),
    ("f1040sb", "f1040sb"),
    ("schedule_d", "f1040sd"),
];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("crates/xtask -> repo root")
        .to_path_buf()
}

fn norm(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// ★ A FORM LINE LABEL at the start of a string: one or two digits, an optional letter, and then
/// something that is not alphanumeric.
///
/// **The bound is what makes it a label rather than a number.** `"1099-K"` starts with four digits
/// and is the unnumbered reconciliation line above Schedule 1 Part I — it must NOT parse as line 10
/// or line 99. `"16 box 1"`, `"35a Form 8888"` and `"8z amount"` must. Every entry this returns
/// `None` for is required to name its part explicitly, which is the only way a cell reaches a block
/// other than by its own printed number.
#[must_use]
pub fn line_label(line: &str) -> Option<u32> {
    let t = line.trim();
    let digits: String = t.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() || digits.len() > 2 {
        return None;
    }
    let rest = &t[digits.len()..];
    let rest = rest
        .strip_prefix(|c: char| c.is_ascii_lowercase())
        .unwrap_or(rest);
    if rest
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphanumeric())
    {
        return None;
    }
    digits.parse().ok()
}

/// ★★★ **The line a *"Subtract line X from line Y"* sentence SUBTRACTS**, parsed out of the sentence
/// itself so it can never be a second key that disagrees with the form's words.
///
/// `None` when the string is not that shape — which is a finding, not a silent skip: a
/// `[[subtracts]]` entry whose sentence is not a subtraction is recording something the rule cannot
/// act on.
#[must_use]
pub fn subtracted_line(sentence: &str) -> Option<String> {
    let s = norm(sentence).to_ascii_lowercase();
    let after = s.split("subtract line ").nth(1)?;
    let token = after.split(" from line ").next()?.trim();
    (!token.is_empty()
        && token.len() <= 3
        && token.starts_with(|c: char| c.is_ascii_digit())
        && token.chars().all(|c| c.is_ascii_alphanumeric()))
    .then(|| token.to_string())
}

/// The opposite direction. `NoDollar` has no opposite — a cell carrying no dollar is not made into
/// one by being subtracted.
fn flipped(d: Direction) -> Direction {
    match d {
        Direction::Understates => Direction::Overstates,
        Direction::Overstates => Direction::Understates,
        Direction::NoDollar => Direction::NoDollar,
    }
}

/// The span of extract lines a block heads: from its own caption to the next block's, or EOF.
fn spans(blocks: &[DirectionBlock], total: usize) -> Vec<(usize, usize)> {
    let mut starts: Vec<usize> = blocks.iter().map(|b| b.extract_line).collect();
    starts.sort_unstable();
    blocks
        .iter()
        .map(|b| {
            let end = starts
                .iter()
                .copied()
                .find(|s| *s > b.extract_line)
                .unwrap_or(total + 1);
            (b.extract_line, end)
        })
        .collect()
}

/// ★★★ **The whole verdict for ONE form, as a pure function** — so a planted defect can reach every
/// rule without mutating a committed map (harness B1, the `field_census.rs::verdict` shape).
///
/// `extract` is the archived text layer's lines. `variants` is the set of enum variant paths that
/// exist (`"Advisory::EicOmitted"`, …). `prompts` maps `"QuestionId::X"` to that question's prompt.
/// Returns every finding; `Ok` is an empty vector.
#[must_use]
pub fn verdict(
    label: &str,
    table: DirectionTable<'_>,
    subtracts: &[SubtractSentence],
    census: &BTreeMap<String, CensusDecision>,
    extract: &[String],
    variants: &BTreeSet<String>,
    prompts: &BTreeMap<String, String>,
) -> Vec<String> {
    let blocks = table.blocks;
    let mut out = Vec::new();
    let unmodeled: Vec<(&String, &CensusDecision)> = census
        .iter()
        .filter(|(_, e)| e.rule == "unmodeled")
        .collect();

    // ── 0. A form with unmodeled entries has a table, and a table has entries to place. ──────────
    if unmodeled.is_empty() != blocks.is_empty() {
        out.push(format!(
            "{label}: {} unmodeled entries and {} [[direction]] blocks — a form with unmodeled \
             entries must carry a direction table, and a direction table must have entries to place",
            unmodeled.len(),
            blocks.len()
        ));
        return out;
    }
    if blocks.is_empty() {
        return out;
    }

    // ── 1. Every caption VERBATIM, at its pinned extract line. ───────────────────────────────────
    for b in blocks {
        match extract.get(b.extract_line - 1) {
            None => out.push(format!(
                "{label}: [[direction]] {:?} points at line {} of an extract with {} lines",
                b.caption,
                b.extract_line,
                extract.len()
            )),
            Some(l) => {
                let (actual, want) = (norm(l), norm(&b.caption));
                // A part heading or a left-margin caption LEADS its line; a form title (the
                // whole-form key R2.2 allows where the parts are not separably captioned) is
                // printed inside the masthead row. Both are checked against the same pinned line.
                if !actual.starts_with(&want) && !actual.contains(&want) {
                    out.push(format!(
                        "{label}:{}: the extract does not carry {:?} — it reads {:?}. The direction \
                         table is a READING of the form: a re-parted revision must RED here, never \
                         drift.",
                        b.extract_line, b.caption, actual
                    ));
                }
            }
        }
    }

    // ── 2. Each block's line range is printed inside the block's own span. ───────────────────────
    let sp = spans(blocks, extract.len());
    for (b, (from, to)) in blocks.iter().zip(&sp) {
        let (Some(first), Some(last)) = (&b.first_line, &b.last_line) else {
            if b.first_line.is_some() || b.last_line.is_some() {
                out.push(format!(
                    "{label}: [[direction]] {:?} declares only one end of its range",
                    b.caption
                ));
            }
            continue;
        };
        for want in [first, last] {
            let printed = extract[from - 1..(*to - 1).min(extract.len())]
                .iter()
                .any(|l| l.split_whitespace().any(|t| t == want));
            if !printed {
                out.push(format!(
                    "{label}: [[direction]] {:?} claims line {want}, which is not printed anywhere \
                     between extract lines {from} and {to} — the range must be a reading of the \
                     form, not a hand-typed guess",
                    b.caption
                ));
            }
        }
    }

    // ── 2a. ★★★ EVERY CAPTION IS GRADED BY EXACTLY ONE READING, and the reading is the ONLY source
    //        of a direction. A caption the reading table does not know grades NOTHING — it is not
    //        defaulted, and the entries it would have placed become unplaceable below, which is the
    //        same fail-loud shape a deleted heading already has. ───────────────────────────────────
    let mut graded: Vec<(&DirectionBlock, Direction)> = Vec::new();
    for b in blocks {
        let want = norm(&b.caption);
        let mut hits = table.readings.iter().filter(|(c, _, _)| norm(c) == want);
        match (hits.next(), hits.next()) {
            (Some((_, d, _)), None) => graded.push((b, *d)),
            (None, _) => out.push(format!(
                "{label}: [[direction]] {:?} has no reading in DIRECTION_OF_CAPTION — a block \
                 states no direction of its own, so a caption with no reading grades nothing. Add \
                 the reading with the form's own total sentence as evidence; it is never defaulted.",
                b.caption
            )),
            (Some(_), Some(_)) => out.push(format!(
                "{label}: [[direction]] {:?} has more than one reading in DIRECTION_OF_CAPTION — \
                 two readings of one caption are two directions for one block",
                b.caption
            )),
        }
    }

    // ── 2b. ★★★ A RANGE AND `NoDollar` ARE CONTRADICTORY CLAIMS. `NoDollar` says *this cell carries
    //        no dollar*; a range of the form's own numbered lines is precisely the claim that it
    //        does. So the one downgrade that switches off both the `Understates` rule and the REACH
    //        check for a whole block of NUMBERED lines at once is not expressible: an income block
    //        cannot be graded `NoDollar` while still heading lines 1a–9. (A rangeless block may be
    //        any direction — the header/trailer cells and Schedule 1's unnumbered 1099-K line.) ────
    for (b, d) in &graded {
        if *d == Direction::NoDollar && (b.first_line.is_some() || b.last_line.is_some()) {
            out.push(format!(
                "{label}: [[direction]] {:?} is graded `NoDollar` and yet heads the form's numbered \
                 lines {}–{} — a block that carries a LINE RANGE carries dollars by construction, \
                 so `NoDollar` there would switch the `Understates` rule and the REACH check off \
                 for every line it heads.",
                b.caption,
                b.first_line.as_deref().unwrap_or("(none)"),
                b.last_line.as_deref().unwrap_or("(none)")
            ));
        }
    }

    // ── 3. Ranges are disjoint. ──────────────────────────────────────────────────────────────────
    let ranged: Vec<(&DirectionBlock, Direction, u32, u32)> = graded
        .iter()
        .filter_map(|(b, d)| {
            Some((
                *b,
                *d,
                line_label(b.first_line.as_deref()?)?,
                line_label(b.last_line.as_deref()?)?,
            ))
        })
        .collect();
    for (i, (a, _, af, al)) in ranged.iter().enumerate() {
        if af > al {
            out.push(format!(
                "{label}: [[direction]] {:?} has an inverted range {af}..{al}",
                a.caption
            ));
        }
        for (b, _, bf, bl) in ranged.iter().skip(i + 1) {
            if af <= bl && bf <= al {
                out.push(format!(
                    "{label}: [[direction]] {:?} and {:?} both claim a line — a line cannot move in \
                     two directions",
                    a.caption, b.caption
                ));
            }
        }
    }

    // ── 3b. ★★★ THE DERIVED FLIP. Every recorded *"Subtract line X from line Y"* is asserted
    //        VERBATIM at its extract line, and X is parsed OUT of the sentence — so the set of
    //        flipped lines is a reading of the form, exactly like the block captions above. ────────
    let mut flips: BTreeSet<u32> = BTreeSet::new();
    for sub in subtracts {
        match extract.get(sub.extract_line - 1) {
            None => out.push(format!(
                "{label}: [[subtracts]] {:?} points at line {} of an extract with {} lines",
                sub.sentence,
                sub.extract_line,
                extract.len()
            )),
            Some(l) => {
                if !norm(l).contains(&norm(&sub.sentence)) {
                    out.push(format!(
                        "{label}:{}: the extract does not carry {:?} — it reads {:?}. A direction \
                         FLIP must be the form subtracting the line in its own words, never a \
                         judgement typed into the map.",
                        sub.extract_line,
                        sub.sentence,
                        norm(l)
                    ));
                    continue;
                }
            }
        }
        match subtracted_line(&sub.sentence)
            .as_deref()
            .and_then(line_label)
        {
            Some(n) => {
                flips.insert(n);
            }
            None => out.push(format!(
                "{label}: [[subtracts]] {:?} is not a \"Subtract line X from line Y\" sentence, so \
                 there is no line it could flip",
                sub.sentence
            )),
        }
    }

    // ── 4/5. Place every entry, and join it. ─────────────────────────────────────────────────────
    let mut placed: BTreeMap<&str, usize> = graded
        .iter()
        .map(|(b, _)| (b.caption.as_str(), 0))
        .collect();
    for (fqn, e) in &unmodeled {
        let n = line_label(&e.line);
        let block = match (n, &e.part) {
            // ★ A numbered line is placed by its NUMBER and may not name a part. Otherwise an income
            //   line could be moved into a no-dollar bucket by one TOML key, which is precisely the
            //   laundering the direction rule exists to stop.
            (Some(_), Some(p)) => {
                out.push(format!(
                    "{label} {fqn} (line {:?}): carries `part = {p:?}` although it has a printed \
                     line number — a numbered line is placed by its number alone",
                    e.line
                ));
                continue;
            }
            (Some(n), None) => ranged
                .iter()
                .find(|(_, _, f, l)| *f <= n && n <= *l)
                .map(|(b, d, _, _)| (*b, *d)),
            (None, Some(p)) => graded
                .iter()
                .find(|(b, _)| &b.caption == p)
                .map(|(b, d)| (*b, *d)),
            (None, None) => {
                out.push(format!(
                    "{label} {fqn} (line {:?}): has no printed line number and names no part — an \
                     entry the table cannot place REDS; it is never defaulted to Overstates",
                    e.line
                ));
                continue;
            }
        };
        let Some((block, block_direction)) = block else {
            out.push(format!(
                "{label} {fqn} (line {:?}): no [[direction]] block places it — a heading may have \
                 been deleted or a revision re-parted. It is never defaulted to Overstates.",
                e.line
            ));
            continue;
        };
        *placed.get_mut(block.caption.as_str()).unwrap() += 1;

        // ── THE JOIN. ────────────────────────────────────────────────────────────────────────────
        let Some(cover) = e.covered_by.as_deref() else {
            out.push(format!(
                "{label} {fqn} (line {:?}): `rule = \"unmodeled\"` with no `covered_by` — the line \
                 is SILENT: neither announced nor refused",
                e.line
            ));
            continue;
        };
        if !variants.contains(cover) {
            out.push(format!(
                "{label} {fqn} (line {:?}): `covered_by = {cover:?}` names no variant that exists",
                e.line
            ));
            continue;
        }
        // ★★ The block grades the line, UNLESS the form subtracts it — then the line is a
        //    reduction of whatever the block measures, and its direction inverts.
        let direction = if n.is_some_and(|n| flips.contains(&n)) {
            flipped(block_direction)
        } else {
            block_direction
        };
        let is_advisory = cover.starts_with("Advisory::");
        if direction == Direction::Understates && is_advisory {
            out.push(format!(
                "{label} {fqn} (line {:?}): an ADVISORY covers an `Understates` line ({:?}). A blank \
                 there is a FALSE STATEMENT, not a forgone benefit — only a question the filer reads \
                 or a refusal that stops the return may cover it.",
                e.line, block.caption
            ));
            continue;
        }
        if let Some(qid) = cover.strip_prefix("QuestionId::") {
            // ★ REACH, not existence. The keyword is `names` when the entry declares one, else the
            //   leading phrase of `reason` — and either way it must appear in the REASON, so it is a
            //   keyword OF THE LINE rather than a token picked to satisfy this check.
            let keyword = match &e.names {
                Some(n) => n.clone(),
                None => e
                    .reason
                    .split('—')
                    .next()
                    .unwrap_or(&e.reason)
                    .trim()
                    .trim_matches('"')
                    .trim_end_matches('.')
                    .to_string(),
            };
            if keyword.chars().count() < 4 {
                out.push(format!(
                    "{label} {fqn} (line {:?}): the cover keyword {keyword:?} is too short to name \
                     anything — a three-character token would be satisfied by accident",
                    e.line
                ));
                continue;
            }
            if !e.reason.to_lowercase().contains(&keyword.to_lowercase()) {
                out.push(format!(
                    "{label} {fqn} (line {:?}): `names = {keyword:?}` does not appear in this \
                     entry's own `reason` — a keyword chosen to match the prompt rather than the \
                     LINE is not a cover",
                    e.line
                ));
                continue;
            }
            let Some(prompt) = prompts.get(cover) else {
                out.push(format!(
                    "{label} {fqn} (line {:?}): no prompt is known for {qid}",
                    e.line
                ));
                continue;
            };
            if !prompt.to_lowercase().contains(&keyword.to_lowercase()) {
                out.push(format!(
                    "{label} {fqn} (line {:?}): {qid}'s prompt never says {keyword:?}. A variant \
                     that exists is not a cover — a question the filer can answer \"No\" to without \
                     ever reading the line's name covers NOTHING.",
                    e.line
                ));
            }
        }
    }

    // ★ NO "dead block" RULE, and the omission is deliberate. A block that places nothing today is
    //   not stale — Schedule D's Part III heads lines 16–22, every one of which btctax DOES fill, so
    //   it grades nothing and is still the correct reading of the form. Deleting it to satisfy a
    //   liveness rule would be deleting the very heading that must red when a Part III line later
    //   goes unmodelled. Drift is caught where drift actually shows: rules 1 and 2, which hold every
    //   caption and every range end against the archived extract.
    let _ = &placed;
    out
}

/// ★★★ **THE OTHER DIRECTION OF THE JOIN.** Every reading must name a caption at least one
/// committed map carries — a reading nothing uses is a direction nobody can watch being applied, and
/// the next revision inherits it silently. Duplicated captions are reported here too, so a second
/// reading of a caption reds even on a form that carries no block for it yet.
///
/// `captions` is the whitespace-normalised caption of every `[[direction]]` block across every
/// checked map.
#[must_use]
pub fn reading_table_findings(readings: &[Reading], captions: &BTreeSet<String>) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for (caption, direction, _) in readings {
        if !seen.insert(norm(caption)) {
            out.push(format!(
                "DIRECTION_OF_CAPTION reads {caption:?} twice — one caption, two directions"
            ));
        }
        if !captions.contains(&norm(caption)) {
            out.push(format!(
                "DIRECTION_OF_CAPTION reads {caption:?} as `{direction:?}`, but no committed \
                 [[direction]] block carries that caption — a reading nothing uses is a judgement \
                 nobody can watch being applied. Delete it, or add the block it was written for."
            ));
        }
    }
    out
}

/// ★★★ **The evidence is itself a reading of the form.** Every sentence quoted in
/// [`DIRECTION_OF_CAPTION`] must be printed in one of the archived extracts, so the justification
/// for a sign cannot decay into a comment nobody re-checked.
///
/// `corpus` is every in-scope extract, whitespace-normalised and concatenated — normalised whole
/// rather than per line because the forms wrap their own total sentences across two printed lines.
#[must_use]
pub fn evidence_findings(readings: &[Reading], corpus: &str) -> Vec<String> {
    readings
        .iter()
        .filter(|(_, _, evidence)| !corpus.contains(&norm(evidence)))
        .map(|(caption, _, evidence)| {
            format!(
                "DIRECTION_OF_CAPTION grades {caption:?} on the evidence {evidence:?}, which no \
                 in-scope extract prints — the sign of a block is a reading of the form's own total \
                 sentence, so the sentence must be one the form actually carries"
            )
        })
        .collect()
}

/// Every variant path declared by the three enums, read from their SOURCE.
///
/// ★ Source-scanned rather than enumerated at runtime because two of the three have no `ALL` list —
/// a Rust enum cannot be iterated — and because the source is the thing a reviewer would check.
#[must_use]
pub fn variant_paths(src: &str, enum_name: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let Some(start) = src.find(&format!("pub enum {enum_name} {{")) else {
        return out;
    };
    let open = src[start..].find('{').expect("an enum has a brace") + start;
    let mut depth = 0usize;
    let mut end = open;
    for (i, c) in src[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = open + i;
                    break;
                }
            }
            _ => {}
        }
    }
    for line in src[open + 1..end].lines() {
        let t = line.trim();
        let name: String = t
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric())
            .collect();
        if name.is_empty() || !name.starts_with(|c: char| c.is_ascii_uppercase()) {
            continue;
        }
        let after = t[name.len()..].trim_start();
        if after.starts_with(',') || after.starts_with('{') || after.starts_with('(') {
            out.insert(format!("{enum_name}::{name}"));
        }
    }
    out
}

fn all_variants() -> BTreeSet<String> {
    let core = repo_root().join("crates/btctax-core/src/tax");
    let read = |f: &str| std::fs::read_to_string(core.join(f)).unwrap_or_default();
    let mut v = variant_paths(&read("advisories.rs"), "Advisory");
    v.extend(variant_paths(&read("return_refuse.rs"), "RefuseReason"));
    v.extend(variant_paths(&read("questions.rs"), "QuestionId"));
    v
}

fn all_prompts() -> BTreeMap<String, String> {
    btctax_core::tax::questions::FORM_QUESTIONS
        .iter()
        .map(|q| (format!("QuestionId::{:?}", q.id), q.prompt.to_string()))
        .collect()
}

/// Run the join over every committed `(year, stem)` of the seven interview forms.
pub fn run() -> Result<String, String> {
    let variants = all_variants();
    let prompts = all_prompts();
    let forms = repo_root().join("crates/btctax-forms/forms");
    let mut findings = Vec::new();
    let mut checked = 0usize;
    let mut entries = 0usize;
    // Both directions of the caption ⇄ reading join need the whole walk: the captions every map
    // carries, and the text of every extract the readings cite.
    let mut captions_seen: BTreeSet<String> = BTreeSet::new();
    let mut corpus = String::new();

    let mut years: Vec<i32> = std::fs::read_dir(&forms)
        .map_err(|e| format!("forms/: {e}"))?
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().to_string_lossy().parse::<i32>().ok())
        .collect();
    years.sort_unstable();

    for year in years {
        for (stem, irs_stem) in INTERVIEW_FORMS {
            let map = forms
                .join(year.to_string())
                .join(format!("{stem}.map.toml"));
            let Ok(text) = std::fs::read_to_string(&map) else {
                continue;
            };
            let parsed: MapShape =
                toml::from_str(&text).map_err(|e| format!("{year}/{stem}: {e}"))?;
            let extract_path = repo_root()
                .join("design/forms/extract")
                .join(format!("{irs_stem}--{year}.txt"));
            let raw = std::fs::read_to_string(&extract_path)
                .map_err(|e| format!("{}: {e}", extract_path.display()))?;
            let extract: Vec<String> = raw.lines().map(str::to_string).collect();
            corpus.push_str(&norm(&raw));
            corpus.push(' ');
            captions_seen.extend(parsed.direction.iter().map(|b| norm(&b.caption)));
            entries += parsed
                .census
                .values()
                .filter(|e| e.rule == "unmodeled")
                .count();
            findings.extend(verdict(
                &format!("{year}/{stem}"),
                DirectionTable {
                    blocks: &parsed.direction,
                    readings: DIRECTION_OF_CAPTION,
                },
                &parsed.subtracts,
                &parsed.census,
                &extract,
                &variants,
                &prompts,
            ));
            checked += 1;
        }
    }
    findings.extend(reading_table_findings(DIRECTION_OF_CAPTION, &captions_seen));
    findings.extend(evidence_findings(DIRECTION_OF_CAPTION, &corpus));
    if checked < 11 {
        return Err(format!(
            "the walk checked only {checked} maps — a join that walks nothing passes by finding \
             nothing"
        ));
    }
    if findings.is_empty() {
        Ok(format!(
            "census join: {entries} unmodeled entries across {checked} maps, every one placed by a \
             direction block asserted against the form's extract, graded by one of \
             DIRECTION_OF_CAPTION's {} readings (each cited to a sentence the form prints) and \
             covered by an existing variant",
            DIRECTION_OF_CAPTION.len()
        ))
    } else {
        Err(format!(
            "the census join found {} finding(s):\n\n{}",
            findings.len(),
            findings.join("\n\n")
        ))
    }
}

/// The slice of a map this module reads. Deliberately a LOOSE shape (not `btctax-forms`'s typed
/// per-form struct): the join is about the census and the direction table, and it must not have to
/// grow a variant every time a form gains a line.
#[derive(serde::Deserialize)]
struct MapShape {
    #[serde(default)]
    census: BTreeMap<String, CensusDecision>,
    #[serde(default)]
    direction: Vec<DirectionBlock>,
    #[serde(default)]
    subtracts: Vec<SubtractSentence>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(cap: &str, at: usize, r: Option<(&str, &str)>) -> DirectionBlock {
        DirectionBlock {
            caption: cap.to_string(),
            extract_line: at,
            first_line: r.map(|(f, _)| f.to_string()),
            last_line: r.map(|(_, l)| l.to_string()),
        }
    }
    /// The synthetic reading table these fixtures are graded by. ★ A test that leaned on the REAL
    /// `DIRECTION_OF_CAPTION` could not plant a missing or a dead reading without editing the
    /// production const, so the readings are a parameter exactly as the blocks are.
    fn table<'a>(blocks: &'a [DirectionBlock], readings: &'a [Reading]) -> DirectionTable<'a> {
        DirectionTable { blocks, readings }
    }
    fn entry(line: &str, reason: &str, cover: &str, names: Option<&str>) -> CensusDecision {
        CensusDecision {
            line: line.to_string(),
            rule: "unmodeled".to_string(),
            reason: reason.to_string(),
            covered_by: Some(cover.to_string()),
            names: names.map(str::to_string),
            part: None,
        }
    }

    /// The committed tree passes the join.
    #[test]
    fn the_committed_maps_are_covered_and_placed() {
        match run() {
            Ok(s) => println!("{s}"),
            Err(e) => panic!("{e}"),
        }
    }

    /// The source scan finds the three enums' variants, including the payload-carrying ones.
    #[test]
    fn the_variant_scan_reads_the_real_enums() {
        let v = all_variants();
        for want in [
            "Advisory::EicOmitted",
            "Advisory::CtcOdcOmitted",
            "RefuseReason::ScheduleCLoss",
            "RefuseReason::DocumentTypeUnsupported", // payload-carrying
            "QuestionId::OtherOutOfScopeIncome",
        ] {
            assert!(
                v.contains(want),
                "the scan missed {want}: {} found",
                v.len()
            );
        }
        assert!(
            !v.contains("Advisory::NoSuchThing"),
            "the scan must not invent variants"
        );
    }

    /// ★★★ **B1 — the join watched going RED on every defect class it claims to catch, and GREEN on
    ///     the near-miss beside it.** The near-misses are the point: a check that reds on everything
    ///     is deleted by the next person who trips it.
    #[test]
    fn the_join_reds_on_every_planted_defect() {
        // A two-part form: Part I is income (Understates), Part II is adjustments (Overstates).
        let extract: Vec<String> = vec![
            "  Part I       Additional Income".into(),        // 1
            "  1   Taxable refunds".into(),                   // 2
            "  8   Other income:".into(),                     // 3
            "    h Jury duty pay . . .            8h".into(), // 4
            "  10  Combine lines 1 through 9".into(),         // 5
            "  Part II     Adjustments to Income".into(),     // 6
            "  11  Educator expenses".into(),                 // 7
            "  26  Add lines 11 through 25".into(),           // 8
        ];
        let blocks = vec![
            block("Part I       Additional Income", 1, Some(("1", "10"))),
            block("Part II     Adjustments to Income", 6, Some(("11", "26"))),
        ];
        // ★ The readings are the FIXTURE's, not the production const — see `table`.
        let readings: &[Reading] = &[
            (
                "Part I       Additional Income",
                Direction::Understates,
                "Combine lines 1 through 9",
            ),
            (
                "Part II     Adjustments to Income",
                Direction::Overstates,
                "Add lines 11 through 25",
            ),
        ];
        let variants: BTreeSet<String> = [
            "Advisory::EicOmitted",
            "RefuseReason::ScheduleCLoss",
            "QuestionId::OtherOutOfScopeIncome",
        ]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
        let prompts: BTreeMap<String, String> = [(
            "QuestionId::OtherOutOfScopeIncome".to_string(),
            "…rent or royalties, Jury duty pay, or a farm…".to_string(),
        )]
        .into_iter()
        .collect();

        let census = |v: Vec<(&str, CensusDecision)>| -> BTreeMap<String, CensusDecision> {
            v.into_iter().map(|(k, e)| (k.to_string(), e)).collect()
        };
        let run = |c: &BTreeMap<String, CensusDecision>, b: &[DirectionBlock]| {
            verdict(
                "T",
                table(b, readings),
                &[],
                c,
                &extract,
                &variants,
                &prompts,
            )
        };

        // ── The clean baseline. Both parts placed, both covered legally. ─────────────────────────
        let good = census(vec![
            (
                "f.8h",
                entry(
                    "8h",
                    "Jury duty pay — not modelled.",
                    "QuestionId::OtherOutOfScopeIncome",
                    None,
                ),
            ),
            (
                "f.11",
                entry(
                    "11",
                    "Educator expenses — not modelled.",
                    "Advisory::EicOmitted",
                    None,
                ),
            ),
        ]);
        assert!(
            run(&good, &blocks).is_empty(),
            "the baseline must PASS, or every red below is meaningless: {:?}",
            run(&good, &blocks)
        );

        // ── (1) ★★★ THE DIRECTION RULE, watched DISCRIMINATING. An `Advisory` on the Part I
        //        (Understates) entry reds, while the SAME advisory on the Part II entry stays
        //        green — which the baseline above already proved. ────────────────────────────────
        let mut c = good.clone();
        c.get_mut("f.8h").unwrap().covered_by = Some("Advisory::EicOmitted".into());
        let f = run(&c, &blocks);
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(
            f[0].contains("ADVISORY covers an `Understates` line"),
            "{}",
            f[0]
        );

        // ── (2) A cover naming a variant that does not exist. ────────────────────────────────────
        let mut c = good.clone();
        c.get_mut("f.11").unwrap().covered_by = Some("Advisory::NoSuchAdvisory".into());
        assert!(run(&c, &blocks)[0].contains("names no variant that exists"));

        // ── (3) No cover at all — the line is silent. ────────────────────────────────────────────
        let mut c = good.clone();
        c.get_mut("f.11").unwrap().covered_by = None;
        assert!(run(&c, &blocks)[0].contains("SILENT"));

        // ── (4) ★★★ THE REACH CHECK. A `QuestionId` cover whose prompt never says the line's name.
        let mut c = good.clone();
        c.get_mut("f.8h").unwrap().names = Some("Prizes and awards".into());
        c.get_mut("f.8h").unwrap().reason = "Prizes and awards — not modelled.".into();
        let f = run(&c, &blocks);
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(f[0].contains("never says"), "{}", f[0]);
        // …and adding the words to the prompt makes it green — the R2.2 kill, both halves.
        let mut prompts2 = prompts.clone();
        prompts2.insert(
            "QuestionId::OtherOutOfScopeIncome".to_string(),
            "…Jury duty pay, Prizes and awards, or a farm…".to_string(),
        );
        assert!(verdict(
            "T",
            table(&blocks, readings),
            &[],
            &c,
            &extract,
            &variants,
            &prompts2
        )
        .is_empty());

        // ── (5) A keyword that is not in the entry's OWN reason — chosen to match the prompt. ────
        let mut c = good.clone();
        c.get_mut("f.8h").unwrap().names = Some("royalties".into());
        assert!(run(&c, &blocks)[0].contains("does not appear in this entry's own `reason`"));

        // ── (6) ★★★ A DELETED PART HEADING. Every entry of that part becomes UNPLACEABLE — it does
        //        NOT fall back to `Overstates`. ────────────────────────────────────────────────────
        let only_part_i = vec![blocks[0].clone()];
        let f = run(&good, &only_part_i);
        assert!(
            f.iter()
                .any(|x| x.contains("no [[direction]] block places it")),
            "deleting Part II must make its entries unplaceable: {f:?}"
        );
        assert!(
            !f.iter().any(|x| x.contains("ADVISORY covers")),
            "★ and it must NOT have been silently regraded as Overstates and passed"
        );

        // ── (7) A caption the extract does not carry (a one-character drift). ────────────────────
        let mut b = blocks.clone();
        b[0].caption = "Part I       Additional Incom".into();
        // (a prefix still matches — so drift the OTHER way, which is what a re-parting looks like)
        b[0].caption = "Part I       Additional Incomes".into();
        assert!(run(&good, &b)[0].contains("does not carry"));

        // ── (8) A range end that is not printed inside the block's span. ─────────────────────────
        let mut b = blocks.clone();
        // "7" is printed nowhere between extract lines 1 and 5; "9" IS (in "…lines 1 through 9"),
        // which is why the plant uses 7 — a plant the rule accepts is not a plant.
        b[0].last_line = Some("7".into());
        let f = run(&good, &b);
        assert!(
            f.iter()
                .any(|x| x.contains("which is not printed anywhere")),
            "{f:?}"
        );

        // ── (9) Two blocks claiming one line. ────────────────────────────────────────────────────
        let mut b = blocks.clone();
        b[1].first_line = Some("8".into());
        assert!(run(&good, &b)
            .iter()
            .any(|x| x.contains("both claim a line")));

        // ── (11) ★★ A NUMBERED line trying to name a part — the laundering path. ────────────────
        let mut c = good.clone();
        c.get_mut("f.8h").unwrap().part = Some("Part II     Adjustments to Income".into());
        assert!(run(&c, &blocks)[0].contains("placed by its number alone"));

        // ── (12) An UNNUMBERED entry naming no part. ─────────────────────────────────────────────
        let mut c = good.clone();
        c.insert(
            "f.hdr".to_string(),
            entry(
                "designee Yes",
                "A designee — not collected.",
                "Advisory::EicOmitted",
                None,
            ),
        );
        assert!(run(&c, &blocks).iter().any(|x| x.contains("names no part")));

        // ── (13) A form with unmodeled entries and NO table at all. ──────────────────────────────
        assert!(run(&good, &[])[0].contains("must carry a direction table"));

        // ── (14) ★★★ A CAPTION WITH NO READING. The direction VALUE left the maps in the T3 seam
        //        fold, so a block states only what the form prints; a caption `DIRECTION_OF_CAPTION`
        //        does not know grades NOTHING, and its entries become unplaceable rather than
        //        defaulting to anything. ───────────────────────────────────────────────────────────
        let f = verdict(
            "T",
            table(&blocks, &readings[..1]),
            &[],
            &good,
            &extract,
            &variants,
            &prompts,
        );
        assert!(
            f.iter()
                .any(|x| x.contains("has no reading in DIRECTION_OF_CAPTION")),
            "{f:?}"
        );
        assert!(
            f.iter()
                .any(|x| x.contains("no [[direction]] block places it")),
            "★ and an ungraded block must place nothing — never grade by default: {f:?}"
        );

        // ── (15) TWO readings of one caption are two directions for one block. ──────────────────
        let doubled: Vec<Reading> = readings
            .iter()
            .copied()
            .chain(std::iter::once((
                "Part I       Additional Income",
                Direction::NoDollar,
                "Combine lines 1 through 9",
            )))
            .collect();
        let f = verdict(
            "T",
            table(&blocks, &doubled),
            &[],
            &good,
            &extract,
            &variants,
            &prompts,
        );
        assert!(
            f.iter().any(|x| x.contains("more than one reading")),
            "{f:?}"
        );

        // ── (16) ★★★ `NoDollar` OVER A LINE RANGE — the reviewer's plant, made structural. It is
        //        the single edit that switches the `Understates` rule and the REACH check off for a
        //        whole block of NUMBERED lines at once, so it is refused on its face. ─────────────
        let downgraded: &[Reading] = &[
            (
                "Part I       Additional Income",
                Direction::NoDollar,
                "Combine lines 1 through 9",
            ),
            readings[1],
        ];
        let f = verdict(
            "T",
            table(&blocks, downgraded),
            &[],
            &good,
            &extract,
            &variants,
            &prompts,
        );
        assert!(
            f.iter()
                .any(|x| x
                    .contains("is graded `NoDollar` and yet heads the form's numbered lines 1–10")),
            "{f:?}"
        );
    }

    /// ★★★ **B1 — the reading table's own two directions, each watched RED.**
    ///
    /// A reading nothing uses is a direction nobody can watch being applied; an evidence sentence no
    /// form prints is a justification that has decayed into a comment. Both are the shape this whole
    /// fold exists to remove, so both red.
    #[test]
    fn the_reading_table_reds_on_a_dead_reading_a_duplicate_and_an_unprintable_evidence() {
        let readings: &[Reading] = &[
            (
                "Income",
                Direction::Understates,
                "This is your total income",
            ),
            (
                "Payments",
                Direction::Overstates,
                "These are your total payments",
            ),
        ];
        let used: BTreeSet<String> = ["Income", "Payments"].iter().map(|c| norm(c)).collect();
        assert!(
            reading_table_findings(readings, &used).is_empty(),
            "the baseline must pass, or every red below is meaningless"
        );

        // A reading no committed map carries.
        let only_income: BTreeSet<String> = ["Income"].iter().map(|c| norm(c)).collect();
        let f = reading_table_findings(readings, &only_income);
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(f[0].contains("no committed"), "{}", f[0]);
        assert!(
            f[0].contains("\"Payments\""),
            "the finding names the orphan: {}",
            f[0]
        );

        // One caption, two readings.
        let dup: Vec<Reading> = readings
            .iter()
            .copied()
            .chain(std::iter::once((
                "Income",
                Direction::NoDollar,
                "This is your total income",
            )))
            .collect();
        let f = reading_table_findings(&dup, &used);
        assert!(f.iter().any(|x| x.contains("twice")), "{f:?}");

        // ── EVIDENCE. The sign is a reading of the form's own total sentence, so the sentence must
        //    be one the form prints — normalised whole, because the forms wrap them. ─────────────
        let corpus =
            "9 Add lines 1z, 2b, 3b, 4b, 5b, 6b, 7, and 8. This is your total income . . 9 \
                      33 Add lines 25d, 26, and 32. These are your total payments . . 33";
        let corpus = norm(corpus);
        assert!(evidence_findings(readings, &corpus).is_empty(), "baseline");
        let invented: &[Reading] = &[(
            "Income",
            Direction::Understates,
            "This is your total gross receipts",
        )];
        let f = evidence_findings(invented, &corpus);
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(
            f[0].contains("which no in-scope extract prints"),
            "{}",
            f[0]
        );
    }

    /// ★★★ **SCHEDULE C LINE 6 — the one understating blank outside the join's scope.**
    ///
    /// R2.2 scopes `census-join` to the seven forms the interview reaches, so Schedule C's 88
    /// `unmodeled` entries per year are not walked (widening that scope is **FR-66**, owning task
    /// T5). One of those entries is an INCOME line — *"Other income, including federal and state
    /// gasoline or fuel tax credit or refund"* — whose blank understates, while every other
    /// consequential Schedule C blank runs the safe way. Its credit side, Schedule 3 line 12
    /// (Form 4136), was already covered; the income side was covered by nothing at all.
    ///
    /// So the cover is asserted HERE, directly and narrowly, under the same two rules the join
    /// applies to a `QuestionId`: the keyword must appear in the entry's own `reason` (a keyword OF
    /// THE LINE, not one picked to satisfy a checker) and in the covering question's PROMPT (REACH:
    /// a variant that exists is not a cover). Without this the cover could rot silently, since no
    /// walk reaches the map.
    #[test]
    fn schedule_c_line_6_is_covered_and_the_attestation_reaches_it() {
        let forms = repo_root().join("crates/btctax-forms/forms");
        let prompts = all_prompts();
        let variants = all_variants();
        let mut checked = 0usize;
        for year in ["2024", "2025"] {
            let text = std::fs::read_to_string(forms.join(year).join("f1040sc.map.toml"))
                .expect("both years ship a Schedule C map");
            let parsed: MapShape = toml::from_str(&text).expect("it parses");
            let (fqn, e) = parsed
                .census
                .iter()
                .find(|(_, e)| e.line == "6")
                .unwrap_or_else(|| {
                    panic!("{year} Schedule C must carry a census entry for line 6")
                });
            assert_eq!(e.rule, "unmodeled", "{year} {fqn}");
            let cover = e.covered_by.as_deref().unwrap_or_else(|| {
                panic!(
                    "{year} Schedule C line 6 is an INCOME line btctax leaves blank: it must be \
                     announced or refused, never silent"
                )
            });
            assert_eq!(cover, "QuestionId::OtherOutOfScopeIncome", "{year} {fqn}");
            assert!(
                variants.contains(cover),
                "{cover} names no variant that exists"
            );
            let keyword = e.names.as_deref().expect("the entry names its keyword");
            assert!(
                e.reason.to_lowercase().contains(&keyword.to_lowercase()),
                "{year}: `names = {keyword:?}` must be a keyword OF THE LINE — it is not in the \
                 entry's own reason: {}",
                e.reason
            );
            let prompt = prompts.get(cover).expect("the question has a prompt");
            assert!(
                prompt.to_lowercase().contains(&keyword.to_lowercase()),
                "{year}: REACH — the attestation never says {keyword:?}, so a filer can answer \
                 \"No\" to it without ever reading the line's name"
            );
            assert!(
                prompt.contains("Form 4136"),
                "…and it must name the form the line points at, in the form's own words"
            );
            checked += 1;
        }
        assert_eq!(checked, 2, "both years, or the assertion walked nothing");
    }

    /// ★★★ **THE JOIN, BOTH WAYS, OVER THE COMMITTED TREE.** Every `[[direction]]` block's caption
    /// has exactly one reading, and every reading names a caption at least one map carries.
    ///
    /// ★ The caption set is read from the maps themselves, never hand-listed: the failure this fold
    /// closes was a value nobody re-derived, and a hand-list here would reintroduce it one layer up.
    #[test]
    fn every_committed_caption_has_exactly_one_reading_and_every_reading_is_used() {
        let forms = repo_root().join("crates/btctax-forms/forms");
        let mut captions: BTreeSet<String> = BTreeSet::new();
        let mut blocks = 0usize;
        for year in std::fs::read_dir(&forms)
            .expect("forms/")
            .filter_map(Result::ok)
        {
            for (stem, _) in INTERVIEW_FORMS {
                let map = year.path().join(format!("{stem}.map.toml"));
                let Ok(text) = std::fs::read_to_string(&map) else {
                    continue;
                };
                let parsed: MapShape = toml::from_str(&text).expect("a committed map parses");
                for b in &parsed.direction {
                    blocks += 1;
                    let want = norm(&b.caption);
                    let n = DIRECTION_OF_CAPTION
                        .iter()
                        .filter(|(c, _, _)| norm(c) == want)
                        .count();
                    assert_eq!(
                        n,
                        1,
                        "{}/{stem}: {:?} has {n} readings in DIRECTION_OF_CAPTION — exactly one \
                         reading grades a caption",
                        year.file_name().to_string_lossy(),
                        b.caption
                    );
                    captions.insert(want);
                }
            }
        }
        assert_eq!(
            blocks, 28,
            "the committed tree carries 28 [[direction]] blocks"
        );
        assert_eq!(captions.len(), 22, "…over 22 distinct captions");
        let orphans = reading_table_findings(DIRECTION_OF_CAPTION, &captions);
        assert!(orphans.is_empty(), "{orphans:?}");
        assert_eq!(
            DIRECTION_OF_CAPTION.len(),
            22,
            "one reading per committed caption, and no reading nothing uses"
        );
    }

    /// ★★★ **THE DERIVED FLIP (D11), and both ways it must fail.**
    ///
    /// A line the form itself subtracts is a REDUCTION of whatever its block measures, so its
    /// direction inverts — which is what stops Schedule B line 3 (the Form 8815 exclusion) being
    /// covered by a question that REFUSES a filer for holding a benefit. The flip is only ever a
    /// reading of the form: the sentence must be printed where the map says it is, and the
    /// subtracted line number is parsed OUT of the sentence rather than typed beside it.
    #[test]
    fn a_subtracted_line_flips_its_blocks_direction_and_only_the_forms_own_words_may_flip_it() {
        // A one-part income form whose line 4 subtracts line 3 — Schedule B's shape.
        let extract: Vec<String> = vec![
            "SCHEDULE B                     Interest and Ordinary Dividends".into(), // 1
            "  1  List name of payer".into(),                                        // 2
            "  2  Add the amounts on line 1".into(),                                 // 3
            "  3  Excludable interest on series EE and I U.S. savings bonds".into(), // 4
            "  4  Subtract line 3 from line 2. Enter the result here".into(),        // 5
        ];
        let blocks = vec![block(
            "Interest and Ordinary Dividends",
            1,
            Some(("1", "4")),
        )];
        let readings: &[Reading] = &[(
            "Interest and Ordinary Dividends",
            Direction::Understates,
            "Subtract line 3 from line 2",
        )];
        let variants: BTreeSet<String> = ["Advisory::UnmodeledDeductionsOmitted"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let prompts: BTreeMap<String, String> = BTreeMap::new();
        let census: BTreeMap<String, CensusDecision> = [(
            "f.3".to_string(),
            entry(
                "3",
                "Excludable interest on series EE and I U.S. savings bonds (attach Form 8815).",
                "Advisory::UnmodeledDeductionsOmitted",
                None,
            ),
        )]
        .into_iter()
        .collect();
        let sub = |sentence: &str, at: usize| SubtractSentence {
            sentence: sentence.to_string(),
            extract_line: at,
        };

        // ── WITH the flip: line 3 is `Overstates`, so an Advisory is an honest cover. ────────────
        let subs = vec![sub("Subtract line 3 from line 2", 5)];
        let t = table(&blocks, readings);
        assert!(
            verdict("T", t, &subs, &census, &extract, &variants, &prompts).is_empty(),
            "{:?}",
            verdict("T", t, &subs, &census, &extract, &variants, &prompts)
        );

        // ── (1) ★★★ REMOVE THE FLIP: the same Advisory now reds under the Understates rule. ─────
        let f = verdict("T", t, &[], &census, &extract, &variants, &prompts);
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(
            f[0].contains("ADVISORY covers an `Understates` line"),
            "without the flip the block grades line 3, and the cover must red: {}",
            f[0]
        );

        // ── (2) ★★★ A SENTENCE THE EXTRACT DOES NOT CARRY. The flip must be the FORM subtracting
        //        the line, never a judgement typed into the map to make a cover legal. ───────────
        let planted = vec![sub("Subtract line 3 from line 1", 5)];
        let f = verdict("T", t, &planted, &census, &extract, &variants, &prompts);
        assert!(
            f.iter().any(|x| x.contains("the extract does not carry")),
            "a sentence the form does not print must red: {f:?}"
        );

        // ── (3) …and one pointed at the wrong LINE of the extract reds too. ─────────────────────
        let f = verdict(
            "T",
            t,
            &[sub("Subtract line 3 from line 2", 3)],
            &census,
            &extract,
            &variants,
            &prompts,
        );
        assert!(
            f.iter().any(|x| x.contains("the extract does not carry")),
            "{f:?}"
        );

        // ── (4) A recorded sentence that is not a subtraction at all. ───────────────────────────
        let f = verdict(
            "T",
            t,
            &[sub("Interest and Ordinary Dividends", 1)],
            &census,
            &extract,
            &variants,
            &prompts,
        );
        assert!(
            f.iter()
                .any(|x| x.contains("is not a \"Subtract line X from line Y\" sentence")),
            "{f:?}"
        );

        // ── The parser: the subtracted line comes OUT of the sentence, never beside it. ─────────
        assert_eq!(
            subtracted_line("Subtract line 3 from line 2").as_deref(),
            Some("3")
        );
        assert_eq!(
            subtracted_line("  4   Subtract line 14 from line 11. If zero or less, enter -0-")
                .as_deref(),
            Some("14")
        );
        assert_eq!(subtracted_line("Add lines 1 through 7"), None);
        assert_eq!(subtracted_line("Subtract the amount from line 2"), None);
    }

    /// The line-label parser: the bound is what tells a form line from a form NUMBER.
    #[test]
    fn a_line_label_is_one_or_two_digits_and_an_optional_letter() {
        assert_eq!(line_label("8h"), Some(8));
        assert_eq!(line_label("16 box 1"), Some(16));
        assert_eq!(line_label("35a Form 8888"), Some(35));
        assert_eq!(line_label("8z amount"), Some(8));
        assert_eq!(line_label("1a(g)"), Some(1));
        // ★ NOT a line label — the unnumbered 1099-K reconciliation line, which must be placed by
        //   name or red, never mistaken for line 10.
        assert_eq!(line_label("1099-K"), None);
        assert_eq!(line_label("designee Yes"), None);
        assert_eq!(line_label("FY begin"), None);
        assert_eq!(line_label("spouse IP PIN"), None);
    }
}
