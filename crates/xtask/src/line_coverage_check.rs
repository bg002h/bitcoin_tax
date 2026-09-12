//! ★★★ §G-11 — the COVERAGE CHECKER. Validates [`btctax_core::tax::line_coverage`] against the
//! extracted form text.
//!
//! **Why this is a test and not a review round.** Two independent Opus rounds measured the grammar's
//! misfit count by hand (r1: ≈25; r2: 10–29), each costing a full round to produce one number. This
//! computes it on every commit. `CLAUDE.md`: *"Conformance ⇒ test. 'Is every form line present?' 'Does
//! each doc comment match the instruction text?' are assertions, not opinions."*
//!
//! **Why it lives in xtask.** It reads `design/forms/extract/`, which is outside every published
//! crate. An `include_str!` reaching there from `btctax-core` would ship a tarball that builds in the
//! workspace and is broken for everyone else, **with exit 0** — the trap recorded in
//! `crate-publishing-state`. So the quotes travel as data and the checking travels here.

use btctax_core::tax::line_coverage::{self, Production};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("crates/xtask -> repo root")
        .to_path_buf()
}

/// ★ Collapse all runs of whitespace to one space.
///
/// **Not cosmetic — it is the fix for a structural false negative.** `pdftotext -layout` wraps a
/// clause mid-sentence, so Form 8995 line 8's instruction reads *"…If zero\n or less, enter -0-…"* in
/// the extract. A literal `contains` would miss it, and the SPEC r2 reviewer identified exactly this
/// (WRAPPING) as a class the checker must not be blind to.
/// ★★★ STANDALONE BRACE GLYPHS ARE DROPPED, and this is layout normalisation rather than an exception.
///
/// A form draws large `{` / `}` braces to group its bracketed rows, and `pdftotext` emits them as lone
/// tokens INSIDE a sentence: Form 6251 line 6's own text comes out as *"…on lines 7, 9, and } 11, and
/// go to line 10"*. Without this filter a `contains` over the extract REJECTS the faithful quote and
/// ACCEPTS a truncated one ending *"…lines 7, 9, and"* — silently dropping line 11 from the zero-out
/// set. That is the Form 6251 line-33 defect class: a citation shortened until the checker was
/// satisfied, on the very form the transcription rule exists because of.
///
/// ★★ This checker and `f6251_map.rs::norm` are two authorities on one question ("is this quote
/// verbatim?"). They now normalise identically. When they did not, the weaker one was satisfied by a
/// degraded citation and reported success — r1 Minor.
///
/// ★ A lone `{`/`}` between spaces is a glyph, never instruction text, so no per-line list is involved.
fn normalize(s: &str) -> String {
    s.split_whitespace()
        .filter(|t| *t != "{" && *t != "}")
        .collect::<Vec<_>>()
        .join(" ")
}

/// ★★★ The CLOSED SET of clamp idioms, each paired with why it denotes that polarity.
///
/// **Fail-closed by construction.** A `Clamped` row whose quote matches NO idiom here is an ERROR, not
/// a silent pass — so the checker can never bless a polarity it did not verify, and a new idiom must be
/// added in a diff, with a reason. That is the opposite of an oracle excuse list, which grows by
/// accretion and goes stale; this one cannot be satisfied by accident.
///
/// ★★ Two of these were found by the population pass, and finding them is the checker working: **two
/// readers hit the same missing idiom and resolved it OPPOSITE ways** — one filed `f8959:8` as an
/// Exception to avoid the rejection, the other declared `f1040sa:4` a floor and would have failed the
/// build. Neither route lands as written, and the defect was in this vocabulary, not in the grammar.
const FLOOR_IDIOMS: &[(&str, &str)] = &[
    (
        "zero or less",
        "the canonical form: 'If zero or less, enter -0-'",
    ),
    ("less than zero", "the same clause, negated phrasing"),
    // ★ OPERAND-RELATIVE, and it is a floor only in light of the line's own arithmetic. Schedule A
    //   line 4 is `line1 - line3`, so "line 3 is more than line 1" IS "the result is negative".
    //   Recorded here rather than left to a reader's judgment, because the direction is NOT derivable
    //   from the clause alone — which is precisely why it needs a written justification.
    (
        "is more than line",
        "operand-relative: on a subtraction A-B, 'B is more than A' means the result is negative",
    ),
    // ★ Schedule SE feeds Form 8959 line 8; a self-employment LOSS is a negative, so this clamps up.
    (
        "if you had a loss",
        "a loss is a negative amount, so this clamps up to zero",
    ),
];

/// The ceiling family. Parenthesised loss-carryforward lines (Form 8995 16/17) clamp DOWN.
const CEIL_IDIOMS: &[(&str, &str)] = &[
    (
        "greater than zero",
        "Form 8995 16/17: a loss carryforward is clamped down to -0-",
    ),
    ("more than zero", "the same clause, alternate phrasing"),
    // ★★ Form 8995-A line 40's phrasing, and it was watched RED before being added: the checker
    //    refused `Clamped(CeilAtZero)` on "If zero or greater, enter -0-" because no idiom covered it.
    //    ★ It is the INCLUSIVE twin of "greater than zero" above — 8995 L17 and 8995-A L40 are the same
    //      quantity worded differently, and `qbi_a`'s KAT proves they never differ in value. Both
    //      idioms are needed because the checker matches the QUOTE, and the two forms quote differently.
    (
        "zero or greater",
        "Form 8995-A L40: the inclusive form of 'greater than zero'; a loss carryforward clamps down",
    ),
];

/// ★★★ **There is deliberately NO exemption list here.** There was one — `NOT_PRINTED`, three entries
/// with reasons, plus a rule that audited the claim against the emitter's code because a B1 plant
/// showed anyone could widen it by writing a sentence.
///
/// It is gone because the audit turned out to be the whole answer: if the emitter's own code is the
/// witness for "this type is not printed", then it is equally the witness for "this type IS printed",
/// and the list in between is redundant. Scope is now derived at the point of use — a type is in scope
/// iff `btctax-forms` names it in real code. **A list nobody can write is a list nobody can widen.**
///
/// **The ratchet.** Exceptions are lines that fit no production, each carrying a written reason.
///
/// ★ This is the number two full review rounds were spent estimating. It only ever goes DOWN: raising
/// it requires editing this line, in a diff, with a reason — which is the whole point. Same shape as
/// the `GAPS` ratchet that went 16 → 0.
///
/// **RAISED 12 → 13 on 2026-08-02 (§G-28/B1b), and here is the reason.** Form 8995-A Parts II and III
/// entered the table. Exactly ONE of their 25 lines fits no production:
///
///   * **f8995a:24** *"Phase-in percentage. Divide line 22 by line 23"* — a PERCENTAGE, the only
///     non-money printed line on the form, and there is no `Divide` production. Whole-dollar rounding
///     would collapse it to 0 or 1.
///
/// ★★ IT WAS FIRST RAISED TO **15**, and that was wrong — worth recording, because the mistake is the
/// one this file exists to prevent. Lines **12** (*"Phased-in reduction. Enter the amount from line 26,
/// if any"*) and **14** (*"Patron reduction. Enter the amount from Schedule D (Form 8995-A), line 6"*)
/// were filed as exceptions on the reasoning that an *"if any"* conditional entry has no production.
/// It does: [`Production::Carry`] is defined three lines from where they were written as *"Enter the
/// amount from line N" — blank when the source line is blank*, which is those two verbatim. A
/// follow-up (§G-29) was even filed proposing a NEW production to cover them.
///
/// ★ The ratchet did its job anyway — it forced the raise into a diff, where a reviewer read the
/// reason and checked it against the production list. A silent counter would have absorbed both.
///
/// **RAISED 13 → 14 on 2026-08-03 (§G-6), for ONE line: `f6251:7`.** Form 6251 line 7 prints THREE
/// bullets under a single label — a Form 2555 referral, *"complete Part III on the back and enter the
/// amount from line 40 here"*, and an *"All others"* flat 26/28% computation — and core takes the
/// second or the third depending on whether Part III routed. That is a BRANCH, not a production:
/// Carry on one path, Scaled on the other.
///
/// ★★ And only the first bullet is quotable at all. The Part III bullet carries the box label printed
/// MID-SENTENCE (*"complete Part III on the · · 7 back and enter…"*), so quoting it fails rule (2b)'s
/// label-precedes test. The first draft quoted the *"All others"* bullet and the checker rejected it
/// as **verbatim-but-attached-to-the-wrong-line** — which is the Form 6251 line-33 class, caught on
/// Form 6251 itself, by the rule that exists because of it.
/// **RAISED 14 → 15 on 2026-08-03 (§G-6), for ONE line: `f1040s2:2`.** *"Alternative minimum tax.
/// **Attach Form 6251**"* is a CONDITIONAL entry, not a production: it carries Form 6251 line 11 when
/// that form is in the packet and is **BLANK when it is not** — so `Schedule2Lines::line2` is
/// `Option<Usd>` and the emitter skips the cell entirely rather than writing a `0`.
///
/// ★★ A `Carry` here would have been a lie in the direction this repo cares about: it asserts the line
/// always holds a figure, and the figure on a no-AMT return would be a sworn `0` on a form that is not
/// attached (§G-11 — *an entry is testimony*). The exception exists to say the blank is a DECISION.
/// **RAISED 15 → 24 on 2026-09-05 (B3/T2), for the NINE Schedule 1-A lines that fit no production.**
/// Measured, not estimated: `cargo run -p xtask -- line-coverage` reported *"24 exceptions, ratchet is
/// 15"* on a table carrying 50 new rows, so the delta is exactly the nine below and nothing else.
/// They are three RECURRING SHAPES, not nine judgments — Schedule 1-A prints the same phase-out block
/// three times, and each block contributes two:
///
///   * **the JUMP — `f1040s1a:10`, `:18`, `:27`, `:33`.** *"Subtract line 9 from line 8. If zero or
///     less, enter the amount from line 7 on line 13"* routes PAST the phase-out; it does not clamp.
///     `Clamped` would print `-0-` and zero the whole deduction for every filer under the threshold,
///     which is most of them. Line 33 is the same shape writing a **nonzero** constant — *"If zero or
///     less, enter $6,000 on line 35"* — where a `-0-` transcription costs the whole senior deduction.
///     Same class as `f1040:34`: a condition with no `-0-` clause is not a clamp.
///   * **the non-money QUOTIENT — `f1040s1a:11`, `:19`, `:28`.** *"Divide line 10 by $1,000 … decrease
///     the result to the next lower whole number"* is a step COUNT, not dollars, and there is no
///     `Divide` production; whole-dollar rounding would be a category error. Precedent `f8995a:24`,
///     the only other non-money printed line in this table.
///   * **`f1040s1a:4b` and `:4c`.** 4b has two branches that BOTH enter (a carry from Form 4137, or
///     the form's own `-0-` when none is filed) — `f1040sse:4a`'s shape. 4c is *"enter the **larger**
///     of line 4a or line 4b"*, and there is no larger-of production (`Bounded` is *"the smaller
///     of"*) — `f1040:12`'s shape, filed for the same reason.
///
/// **RAISED 24 → 26 on 2026-09-07 (T16 / FR-76), for Schedule 2's TWO HSA lines: `f1040s2:17c` and
/// `:17d`.** Measured, not estimated: `line-coverage` reported *"26 exceptions, ratchet is 24"* on the
/// table carrying T16's new rows, so the delta is exactly these two and nothing else.
///
/// ★★ They are `f1040s2:2`'s shape, filed for its reason and no new one. Both are CONDITIONAL entries
/// — *"Additional tax on HSA distributions. **Attach Form 8889**"* and *"Additional tax on an HSA
/// because you didn't remain an eligible individual. **Attach Form 8889**"* — so both are
/// `Option<Usd>`, blank on every return that files no Form 8889, and the emitter skips the cells
/// rather than writing a sworn `0` about a tax the filer never figured on a form the IRS never
/// receives (§G-11 — *an entry is testimony*). A `Carry` would assert the line always holds a figure.
///
/// ★ Note what did NOT need an exception: **every one of Form 8889's own 21 numbered lines** fits a
/// production, because the form states each one — *"Subtract line 4 from line 3. If zero or less,
/// enter -0-"* is `Clamped`, *"Add lines 6 and 7"* is `Combine`, *"enter the smaller of line 2 or line
/// 12"* is `Bounded`, *"Multiply line 20 by 10% (0.10)"* is `Scaled`. That is the transcription rule
/// paying off: a form written for a person to follow has a production for every line.
/// **RAISED 26 → 31 on 2026-09-07 (T16), for the FIVE lines of the *Employer Contribution
/// Worksheet*.** Measured: the run reported *"31 exceptions, ratchet is 26"*, so the delta is
/// exactly these five.
///
/// ★★ They are a different shape from every exception above, and the difference is the point: the
/// worksheet is in the INSTRUCTIONS, not on a filed form. i8889 prints it as *Keep for Your Records*
/// to reconcile a **calendar-year** Form W-2 box 12 code W against the **tax year** Form 8889 line 9
/// asks for. So its lines name `(none)` and quote nothing (rule 2c), and no production can describe
/// a line that is never printed anywhere. They are covered all the same, because a worksheet whose
/// arithmetic nothing checks is the blank-with-no-provenance this whole census exists to prevent —
/// and its OUTPUT is line 9, which is fully checked.
const MAX_EXCEPTIONS: usize = 31;
// ★ RAISED 11 → 12 for Form 8995-A **line 38** (§G-28/B1a). The DPAD line is a CONDITIONAL entry with
//   no "-0-" clause — "DPAD under section 199A(g) allocated from an agricultural or horticultural
//   cooperative. Don't enter more than line 33 minus line 37" presumes an allocation from a Schedule D
//   (Form 8995-A) that btctax does not fill. It is `Option<Usd>`, always `None`, and the emitter writes
//   NOTHING through it. No production describes "a line that is never written", which is precisely what
//   an Exception is for.

/// See the block comment at its use site in [`check`].
///
/// ★ **RAISED 12 → 17 on 2026-09-07 (T16)** for the same five Employer Contribution Worksheet rows
/// [`MAX_EXCEPTIONS`] names: a `(none)` row is by construction unlocatable in a form's text, because
/// it denies being a line of one. Measured — the run named exactly those five and no other new row.
const MAX_UNLOCATABLE: usize = 17;
// ★ RAISED 8 → 12 on 2026-09-05 (B3/T2) for Schedule 1-A line 22's four money COLUMNS —
//   `f1040s1a:22a(ii)`, `22a(iii)`, `22b(ii)`, `22b(iii)`. Measured, not estimated: the run named
//   exactly those four and no other new row. Line 22 is a heading whose entry rows print a bare `a`
//   and `b` with NO text of their own, so a column cell's only quotable text is its column HEADER —
//   and `pdftotext -layout` TRANSPOSES that three-column header block, interleaving "(ii) Deducted
//   on", "(i) Vehicle identification number (VIN)", "Schedule C,", "(iii) Schedule 1-A", "Schedule E,
//   or", "Schedule F". No adjacency to the row's label survives, which is the identical mechanism
//   already recorded for Form 8949 line 1's column headers. ★ These are COUNTED, not accused: the
//   rows are real, their quotes ARE verbatim on the form (rule 2 passes), and only rule (2b)'s
//   label-adjacency cannot reach them.

// ★ RAISED 9 → 11 when the CRYPTO-SLICE Schedule SE (`SeTaxResult`) landed — a form shape the derived
//   scope predicate found and no hand-list contained, because the full-return path uses a DIFFERENT
//   type that was already covered. Two shapes of one form; one was invisible.
//     f1040sse:4a  two-branch, BOTH branches enter — the same class as the full-return twin
//     f1040sse:—   `addl` is Additional Medicare Tax, a FORM 8959 figure that is computed here and
//                  never written to Schedule SE (schedule_se.rs:75 says so). Recorded rather than
//                  dropped: a silently-unmentioned money field is the gap this module exists to close.
// ★ RAISED 5 → 9 when Form 1040, Form 1040 income and Form 8949 landed (173 rows over 12 forms).
//   The four new ones, each argued from the form's own words:
//     f1040:12   "Standard deduction or itemized deductions (from Schedule A)" — two branches that
//                BOTH enter. Itemizing is a Carry of Schedule A line 17; not itemizing is a Constant
//                the form prints in its own margin. i1040gi adjudicates it as "the LARGER of" — and
//                there is no larger-of production (`Bounded` is "the smaller of").
//     f1040:16   "Tax (see instructions)" — the figure comes from the Tax Table / Tax Computation
//                Worksheet / QDCGT Worksheet, none of which is ever emitted. No arithmetic, no source
//                line. Same shape as f1040s1:21 and f1040s3:11.
//     f1040:34   ★★ A CONDITIONAL ENTRY, NOT A CLAMP — and the distinction is the whole program.
//                "If line 33 is more than line 24, subtract line 24 from line 33" states a condition
//                but prints NO "enter -0-", so when it fails the line is BLANK, not zero. `Combine` is
//                blank iff its operands are (both are populated on an owing return) and `Clamped`
//                requires a "-0-" the form does not give — which is why rule (3)'s "-0-" half rejects
//                it even though its clause matches the FLOOR_IDIOM "is more than line".
//     QDCGT:3    a COMPOSITION (Bounded feeding a clamp) on a worksheet that is never emitted, whose
//                clamp phrasing ("If either line 15 or line 16 is blank or a loss") is in neither
//                idiom set. Filed rather than forced — forcing it would assert a polarity the quote
//                does not state.
// ★ RAISED 0 → 5 when the table was populated (118 rows over 11 forms). The five, each a line whose
//   instruction genuinely fits no production — NOT a nearest-fit, which would print a wrong figure:
//     f1040sse:4a  two-branch, BOTH branches enter (one Scaled, one Carry)
//     f1040sse:4c  the arithmetic is a Combine, but the sentence is a form EXIT ("stop") plus an
//                  out-of-scope church-employee -0- that does not clamp the combined figure
//     f1040sse:10  a COMPOSITION — "Multiply the smaller of line 6 or line 9 by 12.4%" is Bounded
//                  feeding Scaled; neither production alone states the line or its blank rule
//     f1040s1:21   §221 student-loan: produced by a worksheet in the 1040 instructions, never emitted
//     f1040s3:11   §6413(c) excess SS/RRTA: same shape, no arithmetic and no source line on the form
//   ★★ Two review rounds estimated this number at ~25 (r1) and 10-29 (r2). Measured: 5.
//   ★ A sixth was filed by a reader (f8959:8) and dissolved when FLOOR_IDIOMS learned "if you had a
//     loss" — the misfit was in this checker's vocabulary, not in the grammar.

/// **The second ratchet, and it is a DIFFERENT number.** Rows whose form btctax emits but for which no
/// text layer is committed, so the quote cannot be verified at all.
///
/// ★ Kept separate from `MAX_EXCEPTIONS` deliberately: an Exception is *"the grammar does not cover
/// this line"*, which is a design fact. An unverifiable row is *"we cannot check what this line says"*,
/// which is a missing ASSET. Collapsing them would let a fetch failure masquerade as a grammar gap.
const MAX_UNVERIFIABLE: usize = 0;
// ★★★ BACK TO 0 (2026-08-01). It was 10 — every Schedule 1 row — because `f1040s1--2024.txt` was not
//    committed and the environment had no network, so ten quotes could not be checked at all. The
//    owner granted network access; the TY2024 revision was fetched from irs.gov, archived with its
//    sha256 in `legal/_provenance/fetch_log.tsv`, extracted, and all ten quotes now verify verbatim.
//
//    ★ The ratchet did exactly what a ratchet is for: it held a MISSING ASSET visible and counted,
//      separately from the design gap next to it, until the asset could be obtained — instead of
//      letting a fetch failure quietly masquerade as a grammar exception.

/// The label forms under which line `label`'s own text may be printed, most specific first.
///
/// ★★★ **This is the fix for the defect `CLAUDE.md` calls the standing root cause.** Rule (2) verified
/// a quote existed *somewhere in the form's file*, so a row could name line 4, quote line 9, and pass —
/// which is how Form 6251 line 33 came to read *"Subtract line 32 from line 12"* where the form says
/// line 22, and of which the rule says *"No review would have caught it."*
///
/// ★★ **The question is not "what span is line N" — it is "is N printed immediately before this
/// sentence".** A first attempt reconstructed each line's span from `pdftotext -layout` output and lost
/// to the text layer: the 1040 packs lines 2a and 2b onto one physical row, prints a section caption in
/// the left column, echoes every label again in the amount column, wraps clauses onto lines that then
/// *begin* label-shaped (`2 (Form 1040), line 4`), and puts the English article `a` in the address
/// block. Six heuristics later it still bound only 31 of 189 rows. Asking the direct question needs
/// none of them, because a form prints a line's number immediately before that line's text — which is
/// what transcription *means*.
///
/// Returns `None` for a label this cannot express (`I-1(d)`, `QDCGT Worksheet, 3`); those are COUNTED,
/// never waved through.
fn label_forms(label: &str) -> Option<Vec<String>> {
    // ★ A COLUMN suffix on a line label (`3(d)` on Schedule D) still quotes the LINE's text — the row
    //   caption — so it binds after stripping.
    let label = label.split_once('(').map_or(label, |(l, _)| l);
    // ★★ …and so does a PART prefix. `fmt_part` writes Form 8949's labels as `I-2(d)`, and stripping
    //    only at '(' leaves `I-2`, whose stem is empty — so the row returned `None` and was counted as
    //    unlocatable. Six of them are Form 8949 **line 2**, the TOTALS line, quoting its own printed
    //    sentence *"Totals. Add the amounts in columns (d), (e), (g), and (h)…"*, which is exactly the
    //    shape (2b) exists to recognise. r7 (I-4): the ratchet was carrying six units of slack under a
    //    class name — "column headers, not form lines" — that told the next author they were unbindable
    //    in principle. They are not. What genuinely cannot bind is Form 8949 line **1**, whose quotes
    //    are column HEADERS ("Proceeds", "Cost or other basis"), no line's text.
    let label = label
        .split_once('-')
        .filter(|(p, _)| !p.is_empty() && p.chars().all(|c| c == 'I' || c == 'V' || c == 'X'))
        .map_or(label, |(_, l)| l);
    let stem: String = label.chars().take_while(char::is_ascii_digit).collect();
    let suffix = &label[stem.len()..];
    if stem.is_empty() || stem.len() > 2 || suffix.len() > 1 {
        return None;
    }
    if !suffix.chars().all(|c| c.is_ascii_lowercase()) {
        return None;
    }
    let mut v = vec![label.to_string()];
    if !suffix.is_empty() {
        // A lettered sub-line prints as a bare letter (`b Taxable interest`).
        v.push(suffix.to_string());
    }
    Some(v)
}

/// Is `quote` printed as line `label`'s own text — i.e. does some form of the label sit immediately
/// before it? `text` must already be whitespace-normalized.
fn label_precedes(text: &str, label: &str, quote: &str) -> Option<bool> {
    let mut forms = label_forms(label)?;
    let bare = label.split_once('(').map_or(label, |(l, _)| l);
    let stem: String = bare.chars().take_while(char::is_ascii_digit).collect();
    let suffix = &bare[stem.len()..];
    // The stem CAPTION, admitted only when the quote carries the row's own bare letter as a token —
    // i.e. the transcription spans the caption *through* the sub-line, which is what 1040 line 25a's
    // "Federal income tax withheld from: a Form(s) W-2" does. A quote of the caption alone is line 25's
    // text, not 25d's, and is now rejected.
    if !suffix.is_empty() && quote.split(' ').any(|w| w == suffix) {
        forms.push(stem.clone());
    }
    // ★★★ THE STEM FORM IS GONE, and r7 (I-1) is why. Accepting `25 …` for a row named `25d` let a row
    //     quote its stem's CAPTION instead of its own text: Form 1040 prints *"25 Federal income tax
    //     withheld from:"* as a caption with no amount box, while 25a–25d each have their own box and
    //     line 25d's own text is *"Add lines 25a through 25c"*. It was load-bearing for exactly ONE
    //     committed row — f1040 25a, whose quote is the caption PLUS the sub-line's own bare letter —
    //     so the caption is admitted only as a PREFIX to a match on the row's real label form, never
    //     on its own.
    Some(forms.iter().any(|f| {
        let needle = format!("{f} {quote}");
        text.match_indices(&needle).any(|(i, _)| {
            // ★★★ A LEFT WORD BOUNDARY, and it is the whole guarantee. `match_indices` is a plain
            //     substring scan, so without this a row naming line N binds to any line whose printed
            //     label ENDS with N — the needle `"5 Qualified business income deduction…"` matches
            //     inside f8995's `"15 Qualified business income deduction…"`, at the `5`.
            //
            //     ★★ The rule held in ONE DIRECTION ONLY, and both committed plants sat on the side it
            //     caught, so the blind half was never observed. r7 built three real misattributions
            //     that `check()` returned `Ok` on — f8995 line 5 carrying line 15's sentence, f1040 1z
            //     carrying line 11's AGI sentence, f1040 6b carrying line 16's *"Tax (see
            //     instructions)"* — and measured the class at 71 accepted misattributions across eight
            //     forms. This is `CLAUDE.md`'s standing root cause in the direction it actually
            //     occurred: Form 6251 line 33 read as *"Subtract line 32 from line 12"* where the form
            //     says 22 — the wrong line carrying the BIGGER number.
            if i > 0 && text.as_bytes()[i - 1].is_ascii_alphanumeric() {
                return false;
            }
            // ★ The bare-letter form pins the LETTER but not the stem, so on its own it would let a row
            //   claim `5b` while quoting line `2b`. Requiring the stem to appear in the run-up closes
            //   that without reconstructing spans — the sub-line always follows its own stem's text.
            !f.chars().all(|c| c.is_ascii_lowercase()) || {
                // ★ Floored to a char boundary: the extracts carry typographic apostrophes
                //   (Schedule SE line 4c reads `you don’t owe self-employment tax`, inside the
                //   run-up of a real match), and a raw byte slice would PANIC mid-character.
                let mut lo = i.saturating_sub(700);
                while lo > 0 && !text.is_char_boundary(lo) {
                    lo -= 1;
                }
                text[lo..i].split(' ').any(|w| {
                    w.chars()
                        .take_while(char::is_ascii_digit)
                        .collect::<String>()
                        == stem
                        && w.len() <= stem.len() + 1
                })
            }
        })
    }))
}

/// ★★★ **THE INVERSE RATCHET: the table's ROW COUNT, which may only go UP.**
///
/// Every ratchet above bounds a *residue* and only ever goes down. This one bounds the **census
/// itself** and only ever goes up, and it exists because the rules above are all per-row: they check
/// what a row says, and structurally cannot notice a row that is no longer there.
///
/// ★★ Measured, not argued (2026-09-12, FR-122). Deleting one `c.line(...)` from `cover_form8995` —
/// Form 8995 line 3, the prior-year QBI-loss carryforward — took the table from 377 rows to 376 and
/// left `make gate` at **3609 passed / 12 skipped**, byte-identical to the baseline summary. Nothing
/// reds: `cover_fns_not_registered` holds whole coverage FUNCTIONS (it was written because deleting
/// `cover_form8995apartiii` from `all()` printed OK at 218 money lines instead of 228), and
/// `missing_cover_fns` demands a cover fn per money-bearing TYPE. Neither speaks about lines, so one
/// printed line of a filed form could leave the instrument with the report still saying OK — the exact
/// false-completeness shape the module header is about, one level finer than the check that caught it
/// last time.
///
/// ★ **Pinned AT the measured count, and compared with `>=`.** That asymmetry is the whole design:
/// growth is silent (378 rows clears a 377 floor, so the TY2026 port adds line-sets without touching
/// this), and any shrink reds. A *loose* floor a few rows down was tried first and measured USELESS
/// for the case that motivated it — at 370 the one-row deletion above still passed, because one row is
/// exactly the size of the defect.
///
/// ★ Its one stated residual: an addition followed later by a deletion nets back to the floor and
/// passes. Closing that needs an exact `==`, which would red on every honest addition — so the floor
/// is raised opportunistically (in a diff, with the run's own printed number) rather than enforced
/// as an equality.
///
/// ★ `#[cfg(test)]` because it belongs to the SUITE, not to [`check`]. [`check`] takes any table, and
/// every planted-defect table in `mod tests` is one row long — a floor inside it would red on all of
/// them. Same placement, and the same reason, as `forge_reach_check::FILE_FLOOR`.
#[cfg(test)]
const MIN_MONEY_LINES: usize = 377;

/// Run the check. Returns `Err` with every failure, so one run reports the whole picture rather than
/// the first problem.
pub fn run() -> Result<String, String> {
    check(&line_coverage::all())
}

/// The rules, applied to ANY table.
///
/// ★★★ **Factored out of [`run`] at review r6 (I-1), and that finding was the sharpest of the round.**
/// The test named *"B1 — the planted defects"* never called `run()`: it constructed a `LineCoverage`
/// row and asserted the row had the fields it had just set. **The entire checker could be deleted with
/// the suite green** — verified by replacing `run()`'s body with `Ok(..)`, which left 55 tests passing.
/// B1's own reviewable question is *"which test reds when this checker is removed?"*, and the answer
/// was: none. Real kills HAD been run by hand against the committed table; none of them was committed.
///
/// Taking a table as a parameter is what makes a planted defect expressible, so the kills below can
/// call the rules on a synthetic bad table instead of asserting on their own fixtures.
/// ★★★ EVERY `cover_*` FUNCTION MUST BE CALLED BY `all()`.
///
/// `missing_cover_fns` below demands that a money-bearing type HAS a coverage function. It does not —
/// and structurally cannot — notice that the function is never invoked, because a defined-but-unused
/// `pub fn` is perfectly legal. So a table can be complete, correct, and **entirely absent** from the
/// extracted rows.
///
/// ★★ This was found by mutation, not by reading: deleting `cover_form8995apartiii` from `all()` took
/// the run from 228 money lines to 218 and still printed **OK**. Ten printed lines of a filed form
/// silently left the instrument, which is exactly the false-completeness class this file exists to
/// prevent — and it is worse than the gap it guards, because the report says "OK" while it happens.
///
/// ★ REACHABILITY, not a direct-call check. `cover_schedulebrow` is invoked by
/// `cover_schedulelines_b`, never by `all()` — a nested payer row is legitimately covered one level
/// down — so "is it called BY `all()`" false-positives on the first honest case it meets. What matters
/// is whether `all()` can reach it at all.
///
/// A pure function over the source so it can be watched going red on a planted case (harness B1).
fn cover_fns_not_registered(cov_src: &str) -> Vec<String> {
    if !cov_src.contains("pub fn all() -> Coverage {") {
        return vec!["line_coverage.rs has no `pub fn all() -> Coverage`".into()];
    }
    // Every `pub fn cover_*`, and the body text between its signature and the next top-level `pub fn`.
    //
    // ★ The leading newline is prepended so a `pub fn` at OFFSET ZERO is found. Without it the first
    //   function in the file is invisible — and in a fixture that begins with `pub fn all()`, that is
    //   the ROOT, so nothing is reachable and every function is reported. The kill test caught this.
    // ★★★ COMMENTS AND STRINGS ARE STRIPPED FIRST, and the file is TRUNCATED at `mod tests`.
    //
    //     This walk is a substring scan, so without both, two silent disablings exist — and r2 found
    //     them by running the algorithm rather than reading it:
    //       · a single `// see cover_form8995apartiii(x)` inside `all()` marks that function reached;
    //       · `all()` is the LAST top-level `pub fn` in `line_coverage.rs`, so its chunk runs to EOF —
    //         a `#[cfg(test)] mod tests` appended after it grants reachability to everything its tests
    //         happen to name.
    //     Either turns the checker off while it keeps printing OK, which is the exact
    //     false-completeness class it was written to kill.
    let stripped: String = cov_src
        .lines()
        .take_while(|l| !l.starts_with("mod tests") && !l.starts_with("#[cfg(test)]"))
        .map(|l| match l.find("//") {
            Some(i) => &l[..i],
            None => l,
        })
        .collect::<Vec<_>>()
        .join("\n");
    // ★ The leading newline is prepended so a `pub fn` at OFFSET ZERO is found. Without it the first
    //   function in the file is invisible — and in a fixture that begins with `pub fn all()`, that is
    //   the ROOT, so nothing is reachable and every function is reported. The kill test caught this.
    let owned = format!("\n{stripped}");
    let cov_src: &str = &owned;
    let mut names: Vec<String> = Vec::new();
    let mut bodies: Vec<(String, String)> = Vec::new();
    let starts: Vec<usize> = cov_src
        .match_indices("\npub fn ")
        .map(|(i, _)| i + 1)
        .collect();
    for (k, &i) in starts.iter().enumerate() {
        let end = starts.get(k + 1).copied().unwrap_or(cov_src.len());
        let chunk = &cov_src[i..end];
        let Some(rest) = chunk.strip_prefix("pub fn ") else {
            continue;
        };
        let Some(fname) = rest.split(['(', '<']).next() else {
            continue;
        };
        if fname == "all" || fname.starts_with("cover_") {
            if fname.starts_with("cover_") {
                names.push(fname.to_string());
            }
            bodies.push((fname.to_string(), chunk.to_string()));
        }
    }
    // Transitive closure from `all()`.
    let body_of = |n: &str| -> String {
        bodies
            .iter()
            .find(|(f, _)| f == n)
            .map(|(_, b)| b.clone())
            .unwrap_or_default()
    };
    let mut reached: Vec<String> = Vec::new();
    let mut queue: Vec<String> = vec!["all".to_string()];
    while let Some(cur) = queue.pop() {
        let body = body_of(&cur);
        for n in &names {
            if reached.iter().any(|r| r == n) {
                continue;
            }
            // A call, not the definition itself.
            if n != &cur && body.contains(&format!("{n}(")) {
                reached.push(n.clone());
                queue.push(n.clone());
            }
        }
    }
    names
        .iter()
        .filter(|n| !reached.iter().any(|r| &r == n))
        .map(|n| {
            format!(
                "line_coverage.rs defines `{n}` but `all()` cannot reach it — its lines are absent \
                 from every check, and the run still reports OK. Register it in `all()`, or call it \
                 from a coverage function that `all()` does reach."
            )
        })
        .collect()
}

/// (4b)'s decision for ONE source file, as a pure function of its inputs.
///
/// ★★★ Extracted so B1 can be satisfied at all. r7 (I-3) neutralised this scan and the whole suite
/// stayed green: it reads the repo's real files, so no in-memory table can plant a defect in it, and
/// "which test reds when this checker is removed?" had the answer "none". Same shape as r6's keystone
/// finding one level down — the rule was real, and nothing was watching it.
fn missing_cover_fns(rel: &str, src: &str, emitter_code: &str, cov_src: &str) -> Vec<String> {
    let mut errs = Vec::new();
    for (name, body) in money_bearing_types(src) {
        if !body.contains("Usd") {
            continue;
        }
        // ★★★ IS THIS TYPE PRINTED? THE EMITTER DECIDES — not a list on either side.
        //
        // Widening the module scan from a three-file hand-list to every module under `tax/`
        // was right (it could not see a new `schedule_1a.rs`) but it swept in ~40 internal
        // compute types — `Advisory`, `CharitableResult` — that reach no page. Answering that
        // with a bigger exemption list would be the same defect twice.
        //
        // So the predicate is derived: a type is IN SCOPE iff the emitter crate names it in
        // real code. That subsumes the old NOT_PRINTED list — `Form8959`/`Form8960`/`Qbi8995`
        // drop out because btctax-forms mentions only their `*Lines` counterparts — and it
        // cannot be widened by writing a sentence.
        if !mentions_ident(emitter_code, &name) {
            continue;
        }
        let want = format!("fn cover_{}(", name.to_lowercase());
        if !cov_src.contains(&want) {
            errs.push(format!(
                "{rel}: type `{name}` declares a Usd field but has no `cover_{}()` — a \
                 money-bearing printed type that nothing in the table mentions is exactly the gap \
                 nested money was",
                name.to_lowercase()
            ));
        }
    }
    errs
}

/// ★★★ **THE FORM → INSTRUCTIONS JOIN** — which booklet is *this* form's own.
///
/// `design/forms/README.md`: *"Instructions follow the **identically-numbered** convention: form
/// `fNNNN` has instructions `iNNNN` (`f6251`→`i6251`, `f1040sa`→`i1040sca`), with `i1040gi` carrying
/// the 1040-family schedules that get no standalone booklet (Schedule 1-A, Schedules 2 and 3)."*
/// This function is that paragraph, and the two exception families are the README's own rather than
/// ours.
///
/// ★★ **It exists because `FilerRecords` used to let the AUTHOR name the booklet.** The seam review
/// re-pointed Schedule A line 5b — state and local *real estate taxes* — at the Form 6251 sentence
/// *"The AMTFTC is a credit that you can claim against the AMT."* and `line-coverage` printed OK.
/// Nothing bound the booklet to the row's form, so any sentence in any archived `i*` booklet
/// satisfied 36 of the 71 `Collected` rows. Deriving the booklet removes the choice.
///
/// ★ The anti-tautology guard (*"never the row's own form"*) is now **structural**: every value this
/// returns begins `i`, so a row cannot quote its own printed text back at itself even by accident.
fn booklet_for(form: &str) -> Option<&'static str> {
    Some(match form {
        // The 1040 itself and the schedules with no standalone booklet — the README's own exception.
        "f1040" | "f1040s1" | "f1040s2" | "f1040s3" | "f1040s1a" => "i1040gi",
        // Schedule A's booklet is `i1040sca`, not `i1040sa` — the second README exception.
        "f1040sa" => "i1040sca",
        "f1040sb" => "i1040sb",
        "f1040sc" => "i1040sc",
        "f1040sd" => "i1040sd",
        "f1040sse" => "i1040sse",
        "f1040s8" => "i1040s8",
        // The identically-numbered convention, for every standalone form.
        "f6251" => "i6251",
        "f8949" => "i8949",
        "f8959" => "i8959",
        "f8960" => "i8960",
        "f8995" => "i8995",
        "f8995a" => "i8995a",
        // ★ T16 — Form 8889's booklet follows the identically-numbered convention too.
        "f8889" => "i8889",
        _ => return None,
    })
}

/// The label region of a `Line <N>` / `Lines <A> and <B>` heading, or `None` if the line is not one.
///
/// ★★ **The blocks are enumerated from the booklet's own headings, never hand-listed** — the same
/// rule the box census follows for captions. Three heading shapes occur across the archived
/// booklets and all three are read here, because picking one would have silently emptied the others:
///
/// | shape | booklets |
/// |---|---|
/// | `Line 5b` alone on its line | `i1040sca`, `i1040gi`, `i6251`, `i8995`, `i1040sc`, `i1040sd`, `i8949` |
/// | `Line 4. <sentence>` | `i8995a`, and the worksheet blocks of `i1040sca`/`i1040sc` |
/// | `Line 7—<Title>` (em dash) | `i8960` |
///
/// A range (`Lines 5a–5d`, `Lines 1 through 3`) expands when both endpoints share a shape, so a
/// sentence under a grouped heading still binds to the row's own line.
fn line_block_labels(line: &str) -> Option<Vec<String>> {
    let t = line.trim();
    let rest = t
        .strip_prefix("Lines ")
        .or_else(|| t.strip_prefix("Line "))?;
    // The label region ends at the first character that can only be prose.
    let region: String = rest
        .chars()
        .take_while(|c| {
            c.is_ascii_alphanumeric()
                || matches!(c, ' ' | ',' | '-' | '\u{2013}' | '\u{2014}' | '&' | '/')
        })
        .collect();
    let mut labels: Vec<String> = Vec::new();
    let mut stop = false;
    for tok in region
        .split([' ', ',', '-', '\u{2013}', '\u{2014}', '&', '/'])
        .filter(|t| !t.is_empty())
    {
        if matches!(tok, "and" | "through" | "or" | "to") {
            continue;
        }
        if is_line_label(tok) {
            if stop {
                break;
            }
            labels.push(tok.to_string());
        } else {
            // The first non-label token ends the region: `Line 1 Taxable Interest` stops at
            // `Taxable`. Anything after it is the heading's prose title.
            stop = true;
        }
    }
    if labels.is_empty() {
        return None;
    }
    // A two-endpoint range covers everything between: `Lines 5a–5d`, `Lines 1 through 3`.
    if labels.len() == 2 {
        if let Some(expanded) = expand_range(&labels[0], &labels[1]) {
            return Some(expanded);
        }
    }
    Some(labels)
}

/// `1`, `5b`, `12` — digits then at most one lowercase letter.
fn is_line_label(tok: &str) -> bool {
    let digits = tok.chars().take_while(char::is_ascii_digit).count();
    if digits == 0 {
        return false;
    }
    let tail = &tok[digits..];
    tail.is_empty() || (tail.len() == 1 && tail.chars().all(|c| c.is_ascii_lowercase()))
}

/// `5a`..`5d` → `5a 5b 5c 5d`; `1`..`3` → `1 2 3`. `None` when the endpoints do not form a range.
fn expand_range(a: &str, b: &str) -> Option<Vec<String>> {
    let split = |s: &str| -> (String, Option<char>) {
        let d: String = s.chars().take_while(char::is_ascii_digit).collect();
        (d.clone(), s[d.len()..].chars().next())
    };
    let (an, al) = split(a);
    let (bn, bl) = split(b);
    match (al, bl) {
        (None, None) => {
            let (x, y) = (an.parse::<u32>().ok()?, bn.parse::<u32>().ok()?);
            (x < y && y - x < 40).then(|| (x..=y).map(|n| n.to_string()).collect())
        }
        (Some(x), Some(y)) if an == bn && x < y => {
            Some((x..=y).map(|c| format!("{an}{c}")).collect())
        }
        _ => None,
    }
}

/// **Every `Line <N>` block a booklet prints, label → the block's text** (a label can head more than
/// one block: `i1040gi` carries the 1040 *and* Schedules 1, 2, 3 and 1-A, each with its own `Line 1a`).
///
/// ★★ **A block includes its own heading line**, because in `i8995a` the heading *is* the sentence
/// (`Line 4. Enter your W-2 wages from the trade, business, or`).
///
/// ★★ **An EMPTY block extends to the next heading, and that is a measurement rather than a fudge.**
/// `i1040gi` is a three-column booklet extracted with no `pdftotext` flags, so headings and bodies
/// interleave across columns: `Line 26` is followed immediately by `Line 25a—Form(s) W-2` and only
/// then by line 26's own body. A heading with no body at all cannot be the whole instruction, so the
/// block runs on until a body appears — which pairs both headings with the shared body instead of
/// pretending line 26 has no instructions at all.
fn line_blocks(text: &str) -> BTreeMap<String, Vec<String>> {
    let lines: Vec<&str> = text.lines().collect();
    let heads: Vec<(usize, Vec<String>)> = lines
        .iter()
        .enumerate()
        .filter_map(|(i, l)| line_block_labels(l).map(|labels| (i, labels)))
        .collect();
    let mut blocks: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (n, (start, labels)) in heads.iter().enumerate() {
        let mut j = n + 1;
        let body = loop {
            let end = heads.get(j).map_or(lines.len(), |(i, _)| *i);
            let body = &lines[*start..end];
            if j >= heads.len() || body[1..].iter().any(|l| !l.trim().is_empty()) {
                break body.join("\n");
            }
            j += 1;
        };
        for l in labels {
            blocks.entry(l.clone()).or_default().push(body.clone());
        }
    }
    blocks
}

/// ★★ **Does the sentence NAME the row's own line?** — the second of the two binds, and the only one
/// available where a booklet's extraction interleaves.
///
/// A sentence that says *"also enter this amount on Schedule D, line 6"* is bound to line 6 by its own
/// words, which is a stronger statement than merely sitting in a block. It is a real bind rather than
/// an escape hatch: 31 of the 36 `FilerRecords` rows satisfy the block clause, and each of the 5 that
/// rely on this one names its line explicitly.
fn names_the_line(sentence: &str, label: &str) -> bool {
    if label.is_empty() {
        return false;
    }
    let low = format!(" {} ", normalize(sentence).to_lowercase());
    if !low.contains(" line ") && !low.contains(" lines ") {
        return false;
    }
    let needle = label.to_lowercase();
    low.match_indices(&needle).any(|(i, _)| {
        let before = low[..i].chars().next_back();
        let after = low[i + needle.len()..].chars().next();
        !before.is_some_and(|c| c.is_ascii_alphanumeric())
            && !after.is_some_and(|c| c.is_ascii_alphanumeric())
    })
}

/// The row's line reduced to the booklet's own label: `1a(d)` → `1a`, `I-1(d)` → `1`, `14b` → `14b`.
///
/// ★ Form 8949's rows are labelled `<part>-<line>(<column>)` (`I-1(d)`), so the part prefix is
/// dropped: the booklet numbers the FORM's lines, not the part's.
fn line_label_of(line: &str) -> String {
    line.rsplit('-')
        .next()
        .unwrap_or(line)
        .chars()
        .take_while(|c| c.is_ascii_digit() || c.is_ascii_lowercase())
        .collect()
}

/// ★★★ **Rule (7) as a pure function** — `SPEC_interview.md` R2 mechanism 1 / R5, T2's kill, as the
/// seam review's C1/I1 rebuilt it.
///
/// A [`Production::Collected`] row names its source, and **the row's own TAX YEAR resolves which
/// paper that source is**:
///
/// - **`DocBox`** — `box_census::revision_in_force(stem, row.year)` picks the archived edition in
///   force for the row's year (`None` reds); that edition must actually PRINT the named box; and the
///   box census's entry for it must quote that edition's own caption. So this is a join between two
///   independently-derived readings of the same paper, resolved through the IRS's own
///   revision-to-year rule rather than a year the author typed.
/// - **`FilerRecords`** — the booklet is [`booklet_for`] the ROW's form at the row's year, and the
///   sentence must fall inside that booklet's `Line <N>` block for the row's own line. Nothing about
///   either join is the author's choice; only the quotation is, and it is checked verbatim
///   (whitespace-normalised).
///
/// ★ A `Collected` row with NEITHER source cannot reach this function: [`line_coverage::CollectedFrom`]
/// has exactly two variants, so the omission is `E0061` at the call site.
fn check_collected_from(
    root: &Path,
    form: &str,
    line: &str,
    field: &str,
    year: &str,
    from: line_coverage::CollectedFrom,
) -> Vec<String> {
    use line_coverage::CollectedFrom;
    let at = format!("{form}:{line} ({field})");
    match from {
        CollectedFrom::DocBox {
            stem,
            box_label,
            also_labels,
        } => {
            let Ok(tax_year) = year.parse::<u32>() else {
                return vec![format!("{at}: the row's year {year:?} is not a tax year")];
            };
            let Some(edition) = crate::box_census::revision_in_force(stem, tax_year) else {
                return vec![format!(
                    "{at} is Collected from {stem} box {box_label}, but no such information return \
                     is archived for TY{year} — the edition a TY{year} filer HOLDS is what the box \
                     caption must be checked against, and nothing governs that year"
                )];
            };
            let Some(doc) = crate::box_census::DOCUMENTS
                .iter()
                .find(|d| d.stem == stem && d.edition == edition)
            else {
                return vec![format!(
                    "{at}: {stem}--{edition} is in force for TY{year} but is in no DOCUMENTS row"
                )];
            };
            let path = root.join(format!("design/forms/extract/{stem}--{edition}.txt"));
            let text = match std::fs::read_to_string(&path) {
                Ok(t) => t,
                Err(e) => {
                    return vec![format!("{at}: cannot read {}: {e}", path.display())];
                }
            };
            let printed = match crate::box_census::printed_boxes(&text, doc.preamble_end) {
                Ok(p) => p,
                Err(e) => return vec![format!("{at}: {stem}--{edition}: {e}")],
            };
            // ★★★ Seam review N-1 — EVERY slot the line reads, not just the one it is named for.
            //     For an ordinary box `also_labels` is empty and this is one iteration.
            let mut errs = Vec::new();
            for slot in std::iter::once(box_label).chain(also_labels.iter().copied()) {
                errs.extend(one_box_slot(doc, &printed, &at, stem, edition, year, slot));
            }
            errs
        }
        CollectedFrom::FilerRecords { instruction_line } => {
            if normalize(instruction_line).is_empty() {
                return vec![format!(
                    "{at} is Collected from the filer's records with an EMPTY instruction line — a \
                     blank quote passes every `contains` and states nothing"
                )];
            }
            let Some(booklet) = booklet_for(form) else {
                return vec![format!(
                    "{at} is Collected from the filer's records, but `booklet_for({form:?})` knows \
                     no instructions document — the booklet is DERIVED from the row's form, so an \
                     unknown form has nowhere to check its quotation"
                )];
            };
            let path = root.join(format!("design/forms/extract/{booklet}--{year}.txt"));
            let Ok(text) = std::fs::read_to_string(&path) else {
                return vec![format!(
                    "{at} needs {booklet}--{year}, this form's own instructions, which is not \
                     archived: {}",
                    path.display()
                )];
            };
            let label = line_label_of(line);
            let blocks = line_blocks(&text);
            let wanted = normalize(instruction_line);
            // (i) the sentence sits inside a `Line <N>` block for this line — its own label, or its
            //     numeric parent where the booklet heads the group rather than the column
            //     (`Line 22.` covers Schedule 1-A lines 22a and 22b).
            let numeric: String = label.chars().take_while(char::is_ascii_digit).collect();
            let bodies: Vec<&String> = blocks
                .get(&label)
                .or_else(|| blocks.get(&numeric))
                .map(Vec::as_slice)
                .unwrap_or_default()
                .iter()
                .collect();
            if bodies.iter().any(|b| normalize(b).contains(&wanted)) {
                return Vec::new();
            }
            // (ii) or the sentence NAMES this line, and is somewhere in this form's own booklet.
            if names_the_line(instruction_line, &label) && normalize(&text).contains(&wanted) {
                return Vec::new();
            }
            vec![format!(
                "{at} cites {instruction_line:?} — it is neither inside a `Line {label}` block of \
                 {booklet}--{year} ({} block(s) carry that heading) nor a sentence that NAMES line \
                 {label}. A filer's-records figure is quoted from the sentence that tells the filer \
                 where to get IT, under ITS OWN line — not from somewhere else in the booklet",
                bodies.len()
            )]
        }
    }
}

/// One slot of a [`CollectedFrom::DocBox`] row: the edition must PRINT it, the box census must
/// DECIDE it, and the census entry's caption must be the document's own text.
///
/// ★ Extracted at seam review N-1 so a line that reads several identical slots (Form W-2 box 12's
///   four) is checked on every one of them by the same rule, rather than on the first alone.
fn one_box_slot(
    doc: &crate::box_census::DocumentAuthority,
    printed: &BTreeMap<String, String>,
    at: &str,
    stem: &str,
    edition: &str,
    year: &str,
    slot: &str,
) -> Vec<String> {
    let Some(caption) = printed.get(slot) else {
        return vec![format!(
            "{at} names {stem}--{edition} box {slot}, which that form does not print in the edition \
             in force for TY{year} — the box grid enumerated from its own extract has {:?}",
            printed.keys().collect::<Vec<_>>()
        )];
    };
    let entries: Vec<&crate::box_census::BoxEntry> = crate::box_census::entries_for(doc)
        .into_iter()
        .filter(|b| b.label == slot)
        .collect();
    match entries.as_slice() {
        [] => vec![format!(
            "{at} names {stem}--{edition} box {slot}, which that edition prints but the box census \
             does not decide — a collected figure with no decided box is the 'we forgot this box' \
             defect one layer up (instructions: {})",
            doc.instructions
        )],
        [entry] if normalize(entry.caption) != normalize(caption) => vec![format!(
            "{at} is Collected from {stem}--{edition} box {slot}, whose census entry quotes {:?}, \
             but the extract prints {caption:?} — the box caption is the document's text, never \
             ours (instructions: {})",
            entry.caption, doc.instructions
        )],
        [_] => Vec::new(),
        _ => vec![format!(
            "{at}: {stem}--{edition} box {slot} has more than one census entry"
        )],
    }
}

/// ★★★ **SEAM REVIEW M-2 — THE EDITIONS ONE TRANSCRIPTION STRUCT IS CLAIMED TO SERVE.**
///
/// `cover_form8889` opens with `Coverage::quoting("2024")`, and one `Form8889` struct fills BOTH the
/// TY2024 and the TY2025 revision. The claim was measured once, by hand, when the form was
/// transcribed — *"the normalised diff of the two extracts is the line-13 dot leader and the
/// footer"* — and then nothing watched it. A TY2025 revision that reworded a line would leave the
/// build green with the wrong sentence cited on every TY2025 return.
///
/// So the claim is a row here, and the checker verifies it: every sentence the table quotes from the
/// FIRST edition must appear VERBATIM in the SECOND, with the tax year substituted. **The year is
/// the only substitution.** A form prints its own year in a dozen sentences (*"…contributed to your
/// HSA for 2024"*), and rewriting that token is what makes the two editions comparable at all;
/// rewriting anything else would be the checker deciding a sentence is close enough, which is the
/// defect this file exists to prevent. Measured on Form 8889: all 22 line-bound quotations pass with
/// the year substituted and no other relaxation.
///
/// ★ **Scope, stated rather than hidden.** This is a per-pair CLAIM, not a derivation over every
/// form: `LineSet` alone would make eleven other forms two-edition pairs (`f1040`, `f8949`, `f8959`,
/// …), whose tables are TY2024-quoted and whose TY2025 sentences nobody has checked. Extending the
/// rule to them is a real piece of work and belongs in its own change; adding a row here without
/// doing that work would report a completeness the checker does not have. A pair with no row is
/// simply unchecked, exactly as it was before.
///
/// ★ A form absent from the table entirely is NOT this rule's business — `cover_fns_not_registered`
/// is what catches a `cover_*` function dropped from `all()`, and duplicating that here would make
/// the synthetic single-row tables the kill tests build fail for the wrong reason.
const EDITIONS_SHARING_ONE_TRANSCRIPTION: &[(&str, &str, &str, &str)] = &[(
    "f8889",
    "2024",
    "2025",
    "T16 / FR-76 — one `Form8889` struct, two `LineSet` revisions (`F8889_2024`, `F8889_2025`) and \
     two maps whose `label-boxes` grids are identical",
)];

/// The rule above, as a pure function of its inputs, so a planted quotation drift can be watched
/// going red (harness B1).
fn second_edition_problems(
    rows: &[line_coverage::LineCoverage],
    form: &str,
    quoted: &str,
    other: &str,
    why: &str,
    other_text: &str,
) -> Vec<String> {
    let hay = normalize(other_text);
    let mut errs = Vec::new();
    for e in rows
        .iter()
        .filter(|e| e.form == form && e.year == quoted && e.line != "(none)")
    {
        if e.instruction.trim().is_empty() {
            continue; // rule (2)'s empty-quote error already fires on this row.
        }
        let want = normalize(&e.instruction.replace(quoted, other));
        if !hay.contains(&want) {
            errs.push(format!(
                "{form}:{} ({}) is quoted from {form}--{quoted} and this build serves \
                 {form}--{other} from the SAME struct ({why}), but the sentence is NOT in \
                 {form}--{other}.txt with the year substituted:\n      {:?}\n   Either the second \
                 edition reworded the line — in which case the struct no longer serves both and the \
                 table needs its own rows for {other} — or the quotation is wrong.",
                e.line, e.field, e.instruction
            ));
        }
    }
    errs
}

pub fn check(cov: &line_coverage::Coverage) -> Result<String, String> {
    let root = repo_root();
    let mut extracts: BTreeMap<String, String> = BTreeMap::new();
    let mut raw_extracts: BTreeMap<String, String> = BTreeMap::new();
    // Rows whose form has a text layer but whose LINE LABEL cannot be located in it — so the quote
    // cannot be bound to its line. Counted separately from a wrong quote, and ratcheted.
    let mut unlocatable: Vec<String> = Vec::new();
    let mut errs: Vec<String> = Vec::new();
    // Rows on a form btctax emits that has no committed text layer: their quotes cannot be checked.
    let mut unverifiable: Vec<String> = Vec::new();

    if cov.0.is_empty() {
        return Err("line_coverage::all() is EMPTY — the checker would vacuously pass".into());
    }

    for e in &cov.0 {
        // (1) Every Exception carries a reason; every non-Exception carries none. Both directions,
        //     because a reason on a real production is a sign the author was unsure.
        match (e.production, e.reason) {
            (Production::Exception, None) => errs.push(format!(
                "{}:{} ({}) is an Exception with NO reason — an unexplained misfit is how a residual \
                 bucket starts",
                e.form, e.line, e.field
            )),
            (p, Some(r)) if p != Production::Exception => errs.push(format!(
                "{}:{} ({}) is {:?} but carries a reason ({r:?}) — reasons are for Exceptions only",
                e.form, e.line, e.field, p
            )),
            _ => {}
        }

        // (2) The instruction must appear VERBATIM in the form's own extracted text.
        let stem = format!("{}--{}", e.form, e.year);
        let text = match extracts.get(&stem) {
            Some(t) => t,
            None => {
                let p = root.join(format!("design/forms/extract/{stem}.txt"));
                match std::fs::read_to_string(&p) {
                    Ok(t) => {
                        raw_extracts.insert(stem.clone(), t.clone());
                        extracts.entry(stem.clone()).or_insert(normalize(&t))
                    }
                    Err(_) => {
                        // ★★ A form btctax EMITS but has no text layer. Derived, never hand-listed:
                        //    a committed map TOML means we emit it, so the quote is UNVERIFIABLE and
                        //    counted; no map means the form name is a typo and is an ERROR. This is
                        //    the Schedule 1 hole the SPEC r2 reviewer found — `f1040s1.map.toml`
                        //    exists, `f1040s1--2024.txt` does not, and neither does its PDF, so it
                        //    cannot be fixed without network access.
                        let emitted = root
                            .join(format!(
                                "crates/btctax-forms/forms/{}/{}.map.toml",
                                e.year, e.form
                            ))
                            .exists();
                        if emitted {
                            unverifiable.push(format!("{}:{}", e.form, e.line));
                        } else {
                            errs.push(format!(
                                "{}:{} quotes form {:?}, which has NEITHER an extract NOR a map — the \
                                 form name is wrong",
                                e.form, e.line, e.form
                            ));
                        }
                        continue;
                    }
                }
            }
        };
        // ★★★ (2c) A ROW THAT NAMES NO LINE MUST QUOTE NO LINE. `(none)` means "this money field is
        // not a line on this form" — so carrying a verbatim sentence from the form is a claim the row
        // itself denies, and rule (2) actively REWARDED it by checking only that the sentence exists.
        // The committed table had exactly one: `SeTaxResult.addl` (Additional Medicare Tax, a Form 8959
        // figure) quoting Schedule SE line 12's *"Self-employment tax. Add lines 10 and 11."*
        // ★ Rules (2)/(2b) are what a `(none)` row escapes — and ONLY those. An early `continue` here
        //   would let it skip the polarity, `Combine`-clause and reason rules below too, opening a hole
        //   in the act of closing one: a `(none)` row could then declare `Clamped(FloorAtZero)` with no
        //   clause at all and pass.
        if e.line == "(none)" {
            if !e.instruction.trim().is_empty() {
                errs.push(format!(
                    "{}:(none) ({}) names no line yet quotes form text — a row that denies being a \
                     line cannot carry one's instruction:\n      {:?}",
                    e.form, e.field, e.instruction
                ));
            }
            // ★★★ …and it is NOT LINE-BOUND, so it is counted. r7 (I-2): the previous fold's own comment
            //     said a `(none)` row escapes "rules (2)/(2b) — and ONLY those", which was true of the
            //     RULES and false of the COUNTING. This module's standard, written a few lines below, is
            //     that a row (2b) could not reach must be "COUNTED and pinned, not left as a silent
            //     shrug". A `(none)` row is by construction such a row.
            unlocatable.push(format!("{}:(none) ({})", e.form, e.field));
        } else {
            // ★★★ AN EMPTY QUOTE IS NOT A QUOTE — and it used to pass EVERYTHING. `str::contains("")`
            //     is true for every haystack, so rule (2) was vacuous; rule (2b) then built the needle
            //     `"5 "`, which occurs on every form, and returned true. A `Collected` or `Carry` row
            //     with a blank instruction therefore faced NO rule at all and moved NO counter — a
            //     fully-passing, entirely unverified row.
            //
            //     ★★ It is the exact mirror of (2c), and the cheaper version of the evasion r6 named:
            //     `field: _` plus a zero at least leaves a `_` in the diff, whereas a blank sixth
            //     argument leaves a row that LOOKS classified — it has a form, a line and a production.
            //     Two blanks that are not the same thing, indistinguishable on the page.
            if e.instruction.trim().is_empty() {
                errs.push(format!(
                    "{}:{} ({}) has an EMPTY instruction. Every line-bound row must carry the form's \
                     own sentence verbatim; a blank quote satisfies rules (2) and (2b) vacuously and \
                     is counted by nothing.",
                    e.form, e.line, e.field
                ));
                continue;
            }
            let want = normalize(e.instruction);
            if !text.contains(&want) {
                errs.push(format!(
                    "{}:{} ({}) quotes text NOT FOUND in {stem}.txt:\n      {:?}",
                    e.form, e.line, e.field, e.instruction
                ));
            } else {
                // ★★★ (2b) THE QUOTE MUST BE THIS LINE'S OWN TEXT, not merely somewhere on the form.
                match label_precedes(text, e.line.as_str(), &want) {
                    Some(true) => {}
                    // ★★ A COLUMN cell may legitimately quote its COLUMN HEADER rather than its line's
                    //    text — Form 8949 `I-1(d)` carries "Proceeds". That cannot be bound by this
                    //    mechanism: `pdftotext -layout` TRANSPOSES the header block, so `(d)` and
                    //    `Proceeds` land on different physical lines with two other columns' text between
                    //    them, and no adjacency survives. So a column-suffixed row that does not bind is
                    //    COUNTED, not accused. ★ It is a real weakening — a Schedule D `3(d)` row quoting
                    //    the wrong line degrades from an error to a count — but the count is ratcheted and
                    //    printed, so it moves in the open rather than passing silently.
                    Some(false) if e.line.contains('(') => {
                        unlocatable.push(format!("{}:{} ({})", e.form, e.line, e.field))
                    }
                    Some(false) => errs.push(format!(
                    "{}:{} ({}) quotes text that IS on {stem} but is NOT printed as line {}'s own \
                     text:\n      {:?}\n    This is the Form 6251 line-33 class — a verbatim \
                     sentence attached to the wrong line.",
                    e.form, e.line, e.field, e.line, e.instruction
                )),
                    None => unlocatable.push(format!("{}:{} ({})", e.form, e.line, e.field)),
                }
            }
        }

        // (3) A clamp polarity must be justified by the clause actually quoted. ★ This is the δ class:
        //     f8995 16/17 say "If GREATER than zero, enter -0-", the inverse of 4/8's "if zero or
        //     less". Transcribing the wrong polarity is a wrong figure, not a wrong blank.
        if let Production::Clamped(pol) = e.production {
            let q = normalize(e.instruction).to_lowercase();
            let says_floor = FLOOR_IDIOMS.iter().any(|(i, _)| q.contains(i));
            let says_ceil = CEIL_IDIOMS.iter().any(|(i, _)| q.contains(i));
            let ok = match pol {
                line_coverage::Polarity::FloorAtZero => says_floor,
                line_coverage::Polarity::CeilAtZero => says_ceil,
            };
            if !ok {
                errs.push(format!(
                    "{}:{} ({}) declares {pol:?} but its quoted clause does not say so — polarity is \
                     TRANSCRIBED, never inferred",
                    e.form, e.line, e.field
                ));
            }
            if !q.contains("-0-") {
                errs.push(format!(
                    "{}:{} ({}) is Clamped but its quote contains no \"-0-\" clause — the trigger for \
                     Clamped is the CLAUSE, not the verb",
                    e.form, e.line, e.field
                ));
            }
        }

        // (4) A `Combine` must NOT carry a clamp clause — that is the r2 C-2 defect, where the verb
        //     decided and 12 unfloored lines were forced to print a zero.
        if e.production == Production::Combine && normalize(e.instruction).contains("-0-") {
            errs.push(format!(
                "{}:{} ({}) is Combine but its quote contains a \"-0-\" clause — it is Clamped",
                e.form, e.line, e.field
            ));
        }

        // ★★★ (7) A `Collected` line NAMES WHERE THE FIGURE COMES FROM, and the naming is checked
        //     against the archive. `SPEC_interview.md` R2 mechanism 1 / R5, T2's kill.
        //
        //     The half that is genuinely new is `DocBox`: it joins the printed line to the
        //     information return's own BOX GRID, enumerated from that document's extract by
        //     `box_census`. So a Form W-2 whose box 1 caption changes by one character reds here as
        //     well as in the box census — the line's claim about the document and the document's own
        //     text are checked against each other, not each against itself.
        if let Production::Collected(from) = e.production {
            errs.extend(check_collected_from(
                &root, e.form, &e.line, e.field, e.year, from,
            ));
        }
    }

    // ★ What the EMITTER crate actually names, comments excluded — the witness for "is this
    //   printed?". Doc comments are excluded because a compute result is routinely NAMED in the
    //   emitter's prose ("core derives the Lines from this `Form8959`") without being consumed there.
    let emitter_code = {
        let mut emitter_code = String::new();
        let dir = root.join("crates/btctax-forms/src");
        let mut stack = vec![dir];
        while let Some(d) = stack.pop() {
            if let Ok(rd) = std::fs::read_dir(&d) {
                for ent in rd.flatten() {
                    let p = ent.path();
                    if p.is_dir() {
                        stack.push(p);
                    } else if p.extension().is_some_and(|e| e == "rs") {
                        if let Ok(t) = std::fs::read_to_string(&p) {
                            for l in t.lines() {
                                let lt = l.trim_start();
                                if !lt.starts_with("//") && !lt.starts_with("//!") {
                                    emitter_code.push_str(l);
                                    emitter_code.push('\n');
                                }
                            }
                        }
                    }
                }
            }
        }
        emitter_code
    };

    // (4b) ★★★ COMPLETENESS, DERIVED FROM SOURCE — is every money-bearing printed TYPE covered at all?
    //
    // The rules above check the rows that EXIST. This one checks that no type was silently skipped,
    // which is the failure the module's own "honest limit" note recorded before nested money landed:
    // `ScheduleBRow.amount` reached paper while nothing in the table mentioned it.
    //
    // ★ Derived, never hand-listed. Every `pub struct`/`pub enum` in the covered modules that declares
    // a `Usd` is found by reading the source, and each must have a `cover_*` function. A new
    // money-bearing type therefore fails the build the moment it is written — the same lesson as the
    // productions and the missing-extract split: state the mechanism, let it decide, never enumerate
    // the outcomes you happened to see.
    {
        let cov_src =
            std::fs::read_to_string(root.join("crates/btctax-core/src/tax/line_coverage.rs"))
                .map_err(|e| format!("cannot read line_coverage.rs: {e}"))?;
        // ★★ EVERY module under tax/, not a three-file list. The hand-list could not see a new
        //    `schedule_1a.rs`, so T2 would have landed 48 printed money lines while this checker
        //    reported "OK" over zero of them — false completeness one layer down. r4 buildability I-2.
        let tax_dir = root.join("crates/btctax-core/src/tax");
        let mut modules: Vec<PathBuf> = std::fs::read_dir(&tax_dir)
            .map_err(|e| format!("cannot read {}: {e}", tax_dir.display()))?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "rs"))
            .collect();
        modules.sort();
        for path in modules {
            let rel = path
                .strip_prefix(&root)
                .unwrap_or(&path)
                .display()
                .to_string();
            let src =
                std::fs::read_to_string(&path).map_err(|e| format!("cannot read {rel}: {e}"))?;
            errs.extend(missing_cover_fns(&rel, &src, &emitter_code, &cov_src));
        }
        // …and every coverage function that DOES exist must actually be registered.
        errs.extend(cover_fns_not_registered(&cov_src));
    }

    // ★★★ (4c) SEAM REVIEW M-2 — the second edition of a form one struct serves.
    for (form, quoted, other, why) in EDITIONS_SHARING_ONE_TRANSCRIPTION {
        if !cov.0.iter().any(|e| e.form == *form && e.year == *quoted) {
            continue; // see the const's own note: an absent form is `cover_fns_not_registered`'s.
        }
        let path = root.join(format!("design/forms/extract/{form}--{other}.txt"));
        match std::fs::read_to_string(&path) {
            Ok(text) => errs.extend(second_edition_problems(
                &cov.0, form, quoted, other, why, &text,
            )),
            Err(e) => errs.push(format!(
                "{form}: this build serves {form}--{other} from the same struct as \
                 {form}--{quoted} ({why}), but {} cannot be read: {e}",
                path.display()
            )),
        }
    }

    // (5) Duplicate coverage of one LINE would let two rows disagree.
    //
    // ★ Keyed on (form, LINE, field), not (form, field) — the population pass proved why: a nested
    //   type can legitimately print on two different lines. `ScheduleBRow.amount` is Schedule B line 1
    //   (interest) AND line 5 (dividends), one Rust field, two form lines, no conflict. Keying on the
    //   field alone rejected a correct table.
    let mut seen: BTreeMap<(&str, &str, &str), usize> = BTreeMap::new();
    for e in &cov.0 {
        *seen.entry((e.form, e.line.as_str(), e.field)).or_default() += 1;
    }
    for ((form, line, field), n) in seen.iter().filter(|(_, n)| **n > 1) {
        errs.push(format!("{form}:{line} ({field}) is covered {n} times"));
    }

    // (6) The ratchet.
    let exceptions = cov
        .0
        .iter()
        .filter(|e| e.production == Production::Exception)
        .count();
    if exceptions > MAX_EXCEPTIONS {
        errs.push(format!(
            "{exceptions} exceptions, ratchet is {MAX_EXCEPTIONS} — every one is a line that fits no \
             production. Raising the ratchet is a decision, taken in a diff, with a reason."
        ));
    }

    if unverifiable.len() > MAX_UNVERIFIABLE {
        errs.push(format!(
            "{} row(s) on a form with no committed text layer, ratchet is {MAX_UNVERIFIABLE}: {}. \
             Fix by fetching the form and extracting it — NOT by deleting the rows, which would also \
             delete the compile-time guarantee for that struct.",
            unverifiable.len(),
            unverifiable.join(", ")
        ));
    }

    // ★★★ THE RATCHET ON RULE (2b)'s REACH. A row whose label cannot be located on the form is a row
    // whose quote is bound to NOTHING — rule (2) degrades back to "this sentence is somewhere on the
    // page", which is the very defect (2b) exists to remove. So the number of rows (2b) could not
    // reach is COUNTED and pinned, not left as a silent shrug. `CLAUDE.md`: an instrument that cannot
    // say which cases it did not cover is not a check.
    //
    // Only ever goes DOWN.
    // ★★★ THE RESIDUE, ENUMERATED HONESTLY — because a ratchet's only content is what it is understood
    // to hold, and this one's account of itself was wrong for 6 of 13 rows (r7, I-4):
    //
    //   6  Form 8949 line 1 columns (d)/(e)/(h), both parts — quotes are COLUMN HEADERS ("Proceeds",
    //      "Cost or other basis"), which are no line's text and which `pdftotext -layout` TRANSPOSES:
    //      `(d)` and `Proceeds` land on different physical lines with two other columns between them,
    //      so no adjacency survives to bind.
    //   1  the QDCGT worksheet line, which lives in the instructions booklet and has no form label.
    //   1  the `(none)` row — by construction not a line, and now counted rather than slipping past.
    //
    // ★★ It used to say "12 Form 8949 column cells whose quote is a column HEADER". Six of those were
    // Form 8949 **line 2** — the TOTALS line — quoting its own printed sentence, unbindable only
    // because `fmt_part` writes `I-2(d)` and the stripper stopped at '('. The class name told the next
    // author they were unbindable in principle; they were not, and the ratchet carried six units of
    // unearned slack. Only ever goes DOWN.
    if unlocatable.len() > MAX_UNLOCATABLE {
        errs.push(format!(
            "{} row(s) name a line that cannot be located in the form text, so their quote is bound \
             to nothing (ratchet {MAX_UNLOCATABLE}): {}",
            unlocatable.len(),
            unlocatable.join(", ")
        ));
    }

    if errs.is_empty() {
        let mut by_form: BTreeMap<&str, usize> = BTreeMap::new();
        for e in &cov.0 {
            *by_form.entry(e.form).or_default() += 1;
        }
        Ok(format!(
            "line-coverage OK: {} money lines across {} form(s) [{}], {exceptions} exception(s) \
             (ratchet {MAX_EXCEPTIONS}), {} unverifiable (ratchet {MAX_UNVERIFIABLE}), {} not \
             line-bound (ratchet {MAX_UNLOCATABLE})",
            cov.0.len(),
            by_form.len(),
            by_form
                .iter()
                .map(|(f, n)| format!("{f}:{n}"))
                .collect::<Vec<_>>()
                .join(" "),
            unverifiable.len(),
            unlocatable.len()
        ))
    } else {
        Err(format!(
            "line-coverage FAILED ({} problem(s)):\n  - {}",
            errs.len(),
            errs.join("\n  - ")
        ))
    }
}

/// Every `pub struct` / `pub enum` in `src` that declares at least one `Usd` field, as (name, body).
///
/// ★ Deliberately syntactic and conservative: it reads the committed source rather than trusting a
/// list. A type that stops carrying money simply drops out; one that starts carrying money appears.
fn money_bearing_types(src: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for kw in ["pub struct ", "pub enum "] {
        let mut from = 0usize;
        while let Some(i) = src[from..].find(kw) {
            let start = from + i;
            let after = start + kw.len();
            let name: String = src[after..]
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            from = after;
            let Some(open) = src[after..].find('{') else {
                continue;
            };
            // A tuple struct or a `;` before the brace is not a braced body.
            if src[after..after + open].contains(';') {
                continue;
            }
            let mut depth = 1usize;
            let mut j = after + open + 1;
            let b = src.as_bytes();
            while j < b.len() && depth > 0 {
                match b[j] {
                    b'{' => depth += 1,
                    b'}' => depth -= 1,
                    _ => {}
                }
                j += 1;
            }
            let body = &src[after + open + 1..j.saturating_sub(1)];
            // Only a real field declaration counts — not a doc-comment mention of `Usd`.
            let declares_money = body.lines().any(|l| {
                let l = l.trim();
                // ★ `Option<Usd>` is money too. Gating on ": Usd" alone made every optional money
                //   leaf invisible — and T3a REQUIRES optional money so a line can express blank.
                !l.starts_with("//")
                    && !l.starts_with("///")
                    && (l.contains(": Usd") || l.contains(": Option<Usd>"))
            });
            if declares_money && !name.is_empty() {
                out.push((name, body.to_string()));
            }
        }
    }
    out
}

/// Does `hay` mention `ident` as a whole identifier (not as a prefix of a longer one)?
fn mentions_ident(hay: &str, ident: &str) -> bool {
    let b = hay.as_bytes();
    let mut from = 0usize;
    while let Some(i) = hay[from..].find(ident) {
        let at = from + i;
        let end = at + ident.len();
        let before_ok = at == 0 || !(b[at - 1].is_ascii_alphanumeric() || b[at - 1] == b'_');
        let after_ok = end >= b.len() || !(b[end].is_ascii_alphanumeric() || b[end] == b'_');
        if before_ok && after_ok {
            return true;
        }
        from = end;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ★★★ THE KILL TEST for the box-12 SLOT SET (harness B1, seam review N-1).
    ///
    /// The census row used to name `fw2` box **12a** while
    /// `form8889::employer_contributions_from_w2s` sums every box-12 slot whose code is `W`
    /// (12a–12d are one `Vec<Box12Entry>`). The label was narrower than the code, and nothing could
    /// tell *"this line reads one slot"* from *"this line reads four and we wrote one"*.
    ///
    /// Both directions: the real row is silent, and a slot the form does not print reds NAMING that
    /// slot — which is what proves the checker walks the whole set rather than the first label.
    #[test]
    fn every_box_12_slot_form_8889_line_9_reads_is_checked_and_a_bogus_slot_reds() {
        let root = repo_root();
        let row = line_coverage::all()
            .0
            .into_iter()
            .find(|e| e.form == "f8889" && e.line == "9")
            .expect("Form 8889 line 9 is in the table");
        let line_coverage::Production::Collected(from) = row.production else {
            panic!("line 9 is Collected from the W-2: {:?}", row.production);
        };
        let line_coverage::CollectedFrom::DocBox {
            stem,
            box_label,
            also_labels,
        } = from
        else {
            panic!("line 9 is a DocBox row");
        };
        assert_eq!((stem, box_label), ("fw2", "12a"));
        assert_eq!(
            also_labels,
            ["12b", "12c", "12d"],
            "the census must name every slot the code reads, or it says something narrower than \
             the sum it describes"
        );
        assert!(
            check_collected_from(&root, "f8889", "9", "line9", row.year, from).is_empty(),
            "the real four-slot row must be silent"
        );

        // ★ A slot the Form W-2 grid does not print. It is only reachable through `also_labels`,
        //   so a checker that stopped at `box_label` would report nothing at all.
        let bogus = line_coverage::CollectedFrom::DocBox {
            stem: "fw2",
            box_label: "12a",
            also_labels: &["12b", "12c", "12e"],
        };
        let errs = check_collected_from(&root, "f8889", "9", "line9", row.year, bogus);
        assert_eq!(
            errs.len(),
            1,
            "exactly the bogus slot is reported: {errs:?}"
        );
        assert!(
            errs[0].contains("box 12e") && errs[0].contains("does not print"),
            "the message must name the slot and why: {}",
            errs[0]
        );
    }

    /// ★★★ **THE KILL TEST for the OBBBA line-1 rows (harness B1, seam review M-1).**
    ///
    /// The defect: `cover_form6251line1` was `if let Form6251Line1::Y2024 { line1 } = p`, so a TY2025
    /// chain produced an **empty** `Coverage` — an `if let` that matches nothing is silent — and the
    /// comment above it promised the 1a/1b rows *"when that year's map lands"*. That map landed in
    /// `d8d023af` and no rows were added, so `Form6251Line1::Y2025`'s transcription (the doc comment
    /// that hardcodes "line 37") was verified verbatim by NOTHING: `cite-check` also excuses this form
    /// (`AUTHORITY_NOT_YET_ARCHIVED` carries `("f6251", &[2024, 2025])`).
    ///
    /// Three directions, because a rule seen only on a clean table has not been seen discriminating:
    ///
    /// 1. the OBBBA variant yields rows AT ALL (this is what the `if let` did not do);
    /// 2. the committed rows are verbatim on `f6251--2025.txt` and bound to their own labels;
    /// 3. ★ the ROT that matters — 1a's cross-reference moved to the TY2026 draft's *"line 43"* — reds,
    ///    naming the row. That is the collision, and it is the sentence a port would paste.
    #[test]
    fn the_obbba_line_1_rows_are_verbatim_and_a_moved_cross_reference_reds() {
        use btctax_core::tax::form6251::Form6251Line1;

        // (1) The variant produces rows. Before this fold it produced none, silently.
        // ★ The cited line is VOUCHED, not typed: the accessor is the only route to a
        //   `SeniorDeductionSubtotal` outside `schedule_1a`, so this instrument cannot be pointed at a
        //   Schedule 1-A line no revision printed. `None` is a schedule-less year — a real zero on
        //   TY2025's own line number, and the LINE is what direction (3) below plants against.
        let obbba = line_coverage::cover_form6251line1(&Form6251Line1::Y2025 {
            line1a: btctax_core::conventions::Usd::ZERO,
            line1b: btctax_core::conventions::Usd::ZERO,
            senior_deduction: btctax_core::tax::schedule_1a::Schedule1A::senior_deduction_subtotal(
                None,
            ),
        });
        assert_eq!(
            obbba.0.len(),
            2,
            "the OBBBA line-1 region prints TWO boxes (1a and 1b); an empty Coverage here is the \
             `if let` that matched nothing"
        );

        // (2) …and they reach the real table, quoted from the 2025 extract, and pass every rule.
        let rows: Vec<line_coverage::LineCoverage> = line_coverage::all()
            .0
            .into_iter()
            .filter(|e| e.form == "f6251" && (e.line == "1a" || e.line == "1b"))
            .collect();
        assert_eq!(
            rows.len(),
            2,
            "both OBBBA rows must be registered in `all()` — `Form6251::default()` is the TY2024 \
             shape, so they reach the checker only through their own instance"
        );
        assert!(
            rows.iter().all(|e| e.year == "2025"),
            "the rows must be quoted from f6251--2025.txt, not from the default year: {:?}",
            rows.iter().map(|e| e.year).collect::<Vec<_>>()
        );
        let only = |rows: Vec<line_coverage::LineCoverage>| {
            let mut c = line_coverage::Coverage::default();
            c.0 = rows;
            c
        };
        check(&only(rows.clone())).expect("the committed OBBBA line-1 rows must be clean");

        // (3) The rot: line 1a's cross-reference as the TY2026 draft prints it.
        let mut planted = rows.clone();
        for e in &mut planted {
            if e.line == "1a" {
                e.instruction = "Subtract Schedule 1-A (Form 1040), line 43, from Form 1040, 1040-SR, or 1040-NR, line 14";
            }
        }
        assert!(
            planted
                .iter()
                .zip(rows.iter())
                .any(|(a, b)| a.instruction != b.instruction),
            "the plant must differ from the real rows, or the kill proves nothing"
        );
        let err = check(&only(planted)).expect_err(
            "the TY2026 cross-reference was accepted as TY2025's own text — the AMT base's source \
             line could then move by a whole revision with this instrument green",
        );
        assert!(
            err.contains("f6251:1a") && err.contains("NOT FOUND in f6251--2025.txt"),
            "the failure must name the row and the extract it is not in: {err}"
        );
    }

    /// ★★★ THE KILL TEST for `second_edition_problems` (harness B1, seam review M-2).
    ///
    /// The defect it exists to catch is *"one struct serves two revisions, and only ONE of them was
    /// ever checked"* — the shape the T16 build's own note asserted by hand and nothing watched. It
    /// is exercised in three directions, because a rule that only ever sees a clean table has not
    /// been seen discriminating:
    ///
    /// 1. the REAL table against the REAL second extract → silent;
    /// 2. a quotation that exists on the first edition and NOT the second → red, naming the row;
    /// 3. a quotation whose only difference is the YEAR → silent, which is the whole reason the
    ///    substitution exists (a form prints its own year in a dozen sentences).
    #[test]
    fn a_sentence_missing_from_the_second_edition_is_caught_and_a_year_difference_is_not() {
        let root = repo_root();
        let f8889_2025 = std::fs::read_to_string(root.join("design/forms/extract/f8889--2025.txt"))
            .expect("the TY2025 Form 8889 extract is archived");

        // (1) The committed table is clean against the second edition.
        let real = line_coverage::all();
        assert!(
            second_edition_problems(&real.0, "f8889", "2024", "2025", "why", &f8889_2025)
                .is_empty(),
            "the committed Form 8889 rows must all be present in the TY2025 extract: {:?}",
            second_edition_problems(&real.0, "f8889", "2024", "2025", "why", &f8889_2025)
        );

        // (2) ★ A REAL drift shape, not a nonsense string: the §223(b) figures, which the 2025
        //     revision moved to $4,300 / $8,550. A table that carried the 2024 sentence forward
        //     would quote a limit that edition does not print.
        let planted = vec![line_coverage::LineCoverage {
            form: "f8889",
            year: "2024",
            line: "3".to_string(),
            field: "line3",
            production: line_coverage::Production::Constant,
            instruction: "enter $4,150 ($8,300 for family coverage)",
            reason: None,
        }];
        let errs = second_edition_problems(&planted, "f8889", "2024", "2025", "why", &f8889_2025);
        assert_eq!(errs.len(), 1, "the drifted row must be reported: {errs:?}");
        assert!(
            errs[0].contains("f8889:3") && errs[0].contains("NOT in f8889--2025.txt"),
            "the message must name the row and the edition it is missing from: {}",
            errs[0]
        );

        // (3) …and a sentence that differs ONLY by the year is not a drift.
        let year_only = vec![line_coverage::LineCoverage {
            form: "f8889",
            year: "2024",
            line: "9".to_string(),
            field: "line9",
            production: line_coverage::Production::Combine,
            instruction: "Employer contributions made to your HSAs for 2024",
            reason: None,
        }];
        assert!(
            second_edition_problems(&year_only, "f8889", "2024", "2025", "why", &f8889_2025)
                .is_empty(),
            "a sentence whose only difference is the tax year must pass"
        );
    }

    /// ★★★ THE KILL TEST for `cover_fns_not_registered` (harness B1: no checker exists until it has
    /// been observed RED on a planted defect).
    ///
    /// The defect it exists to catch is REAL and was found by mutation on live source: deleting
    /// `cover_form8995apartiii` from `all()` took the run from 228 money lines to 218 and still
    /// printed **OK**. Ten printed lines of a filed form left the instrument silently — worse than an
    /// uncovered line, because the report asserts completeness while it happens.
    #[test]
    fn an_unregistered_coverage_function_is_caught_and_a_transitively_reached_one_is_not() {
        // (a) The real shape: defined, never reached.
        let orphaned = "\
pub fn all() -> Coverage {
    let mut c = Coverage::default();
    c.0.extend(cover_alpha(&x).0);
    c
}
pub fn cover_alpha(p: &A) -> Coverage { Coverage::default() }
pub fn cover_beta(p: &B) -> Coverage { Coverage::default() }
";
        let errs = cover_fns_not_registered(orphaned);
        assert_eq!(errs.len(), 1, "exactly the orphan is reported: {errs:?}");
        assert!(
            errs[0].contains("cover_beta") && errs[0].contains("reports OK"),
            "the message must name the function AND why silence is the danger: {}",
            errs[0]
        );

        // (b) ★★ TRANSITIVE reachability, not a direct-call check. `cover_schedulebrow` is invoked by
        //     `cover_schedulelines_b`, never by `all()` — a nested payer row covered one level down is
        //     legitimate, and a direct-call rule false-positives on it. This half is what kept the
        //     first draft of this checker from landing broken.
        let nested = "\
pub fn all() -> Coverage {
    let mut c = Coverage::default();
    c.0.extend(cover_alpha(&x).0);
    c
}
pub fn cover_alpha(p: &A) -> Coverage { cover_beta(&p.row) }
pub fn cover_beta(p: &B) -> Coverage { Coverage::default() }
";
        assert!(
            cover_fns_not_registered(nested).is_empty(),
            "a function `all()` reaches THROUGH another must not be reported: {:?}",
            cover_fns_not_registered(nested)
        );

        // (c) A missing `all()` at all is itself the loudest possible failure, not a silent pass.
        assert_eq!(cover_fns_not_registered("pub fn cover_x() {}").len(), 1);

        // ★★★ (d) AND (e) — THE TWO WAYS A SUBSTRING SCAN CAN BE SILENTLY TURNED OFF, both found by
        //     r2 running the algorithm rather than reading it. Note the shape: `all()` is LAST, which
        //     is how `line_coverage.rs` is actually laid out — cases (a)/(b) above put it first and so
        //     could never have exercised either.
        let orphan_but_named_in_a_comment = "\
pub fn cover_alpha(p: &A) -> Coverage { Coverage::default() }
pub fn cover_beta(p: &B) -> Coverage { Coverage::default() }
pub fn all() -> Coverage {
    let mut c = Coverage::default();
    // see cover_beta(x) for the nested rows
    c.0.extend(cover_alpha(&x).0);
    c
}
";
        assert_eq!(
            cover_fns_not_registered(orphan_but_named_in_a_comment).len(),
            1,
            "a `cover_*` named only in a COMMENT must not count as reached — one comment would \
             otherwise disable this checker for that function while it keeps printing OK"
        );

        let orphan_named_only_in_tests = "\
pub fn cover_alpha(p: &A) -> Coverage { Coverage::default() }
pub fn cover_beta(p: &B) -> Coverage { Coverage::default() }
pub fn all() -> Coverage {
    let mut c = Coverage::default();
    c.0.extend(cover_alpha(&x).0);
    c
}
#[cfg(test)]
mod tests {
    fn t() { super::cover_beta(&Default::default()); }
}
";
        assert_eq!(
            cover_fns_not_registered(orphan_named_only_in_tests).len(),
            1,
            "`all()` is the LAST top-level fn in the real file, so its chunk runs to EOF — a test \
             module appended after it must not grant reachability to everything the tests name"
        );
    }

    /// ★★★ (4b) — the completeness scan, now killable because `missing_cover_fns` is pure.
    ///
    /// It reads the repo's real files, so no in-memory `Coverage` can plant a defect in it: r7 (I-3)
    /// neutralised it and the whole suite stayed green. Feeding it synthetic sources is the only honest
    /// way to answer B1's question — *which test reds when this checker is removed?*
    #[test]
    fn the_completeness_scan_demands_a_cover_fn_for_a_printed_money_type() {
        let src = "pub struct NewMoneyThing { pub amount: Usd }";

        // Printed (the emitter names it) and uncovered -> a finding.
        let errs = missing_cover_fns("tax/new.rs", src, "let x: NewMoneyThing = todo!();", "");
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert!(errs[0].contains("cover_newmoneything()"), "{errs:?}");

        // …and each half of the predicate must matter on its own, or the test passes for a stale
        // reason. Covered -> silent; not named by the emitter -> out of scope.
        assert!(missing_cover_fns(
            "tax/new.rs",
            src,
            "let x: NewMoneyThing = todo!();",
            "fn cover_newmoneything(",
        )
        .is_empty());
        assert!(missing_cover_fns("tax/new.rs", src, "nothing mentions it", "").is_empty());
        // ★ And a type carrying no money is not in scope however loudly the emitter names it.
        assert!(missing_cover_fns(
            "tax/new.rs",
            "pub struct NoMoney { pub n: u32 }",
            "let x: NoMoney = todo!();",
            "",
        )
        .is_empty());
    }

    /// The checker passes on the committed table — **and the table is still the whole table.**
    ///
    /// ★★★ The floor is the second half, added 2026-09-12 (FR-122). `run()` is what binds the 377-row
    /// census to `design/forms/extract/`, and a planted rotted sentence reds here naming the row
    /// (measured: `f8995:2 (line2) quotes text NOT FOUND in f8995--2024.txt`). But every rule inside
    /// [`check`] is per-row, so a DELETED row is invisible to all of them — measured: dropping Form
    /// 8995 line 3 took the table 377 → 376 with `make gate` unchanged at 3609 passed / 12 skipped.
    /// See [`MIN_MONEY_LINES`]. A checker that cannot notice its own subject shrinking is the
    /// false-completeness shape this module exists against.
    #[test]
    fn the_committed_coverage_table_is_consistent_with_the_form_text() {
        let rows = line_coverage::all().0.len();
        assert!(
            rows >= MIN_MONEY_LINES,
            "the coverage table has {rows} money lines, below the {MIN_MONEY_LINES} floor. Every \
             rule in `check` is per-row and none can see a row that is GONE, so a printed line of a \
             filed form can leave the census with the report still saying OK. If the deletion is \
             deliberate, say which lines and why, and lower the floor in the same diff."
        );
        match run() {
            Ok(s) => println!("{s}"),
            Err(e) => panic!("{e}"),
        }
    }

    /// ★★★ **B1 — the planted defects, and they now CALL THE CHECKER.**
    ///
    /// The previous version of this test asserted on its own fixtures — it built a row and checked the
    /// row had the fields it had just set — so no rule was ever invoked. Review r6 (I-1) proved the
    /// consequence: replacing `run()`'s body with `Ok(..)` left the whole suite green. Twelve rules,
    /// both ratchets and the completeness scan could all vanish silently.
    ///
    /// Each case below builds a table that violates exactly one rule and asserts [`check`] REJECTS it,
    /// naming the rule. That is the difference between a kill and a decoration.
    #[test]
    fn each_rule_rejects_a_table_that_violates_it() {
        use btctax_core::tax::line_coverage::{Coverage, Polarity, Production};

        // A single row that PASSES everything, so each plant below differs in exactly one way.
        let good = |c: &mut Coverage| {
            c.line(
                btctax_core::conventions::Usd::ZERO,
                "f8995",
                "5",
                "line5",
                Production::Scaled,
                "Qualified business income component. Multiply line 4 by 20% (0.20)",
            )
        };
        let mut base = Coverage::default();
        good(&mut base);
        assert!(
            check(&base).is_ok(),
            "the control table must PASS — otherwise every plant below passes for the wrong reason"
        );

        // (2) a PARAPHRASE — the single most likely rot.
        let mut c = Coverage::default();
        c.line(
            btctax_core::conventions::Usd::ZERO,
            "f8995",
            "5",
            "line5",
            Production::Scaled,
            "Qualified business income component. Take 20 percent of line 4",
        );
        let e = check(&c).unwrap_err();
        assert!(
            e.contains("NOT FOUND"),
            "rule (2) must reject a paraphrase: {e}"
        );

        // ★★★ (2b) THE FORM 6251 LINE-33 CLASS — a VERBATIM sentence attached to the WRONG LINE.
        // This is the plant rule (2) could never fail, because the sentence really is on the form.
        // Both lines here are floor-clamped with identically-shaped clauses, so every other rule still
        // passes and only the line binding decides it.
        let mut c = Coverage::default();
        c.line(
            btctax_core::conventions::Usd::ZERO,
            "f8995",
            "4",
            "line4",
            Production::Clamped(Polarity::FloorAtZero),
            "Total qualified REIT dividends and PTP income. Combine lines 6 and 7. If zero or less, \
             enter -0-",
        );
        let e = check(&c).unwrap_err();
        assert!(
            e.contains("NOT printed as line 4's own text"),
            "rule (2b) must reject line 8's sentence filed under line 4 — this is the defect that \
             shipped Form 6251 line 33 as \"Subtract line 32 from line 12\": {e}"
        );

        // ★★ (2b) the STEM guard. A lettered sub-line prints as a bare letter, so matching `b Taxable
        // interest` alone would let any `Nb` claim it. The run-up must carry the row's own stem.
        let mut c = Coverage::default();
        c.line(
            btctax_core::conventions::Usd::ZERO,
            "f1040",
            "5b",
            "line5b",
            Production::filer_records(
                "Include on line 25c any federal income tax withheld on your Form(s) W-2G.",
            ),
            "Taxable interest",
        );
        let e = check(&c).unwrap_err();
        assert!(
            e.contains("NOT printed as line 5b's own text"),
            "rule (2b) must reject line 2b's text claimed as 5b — the bare letter is not enough: {e}"
        );

        // ★★★ (2c) a row that NAMES NO LINE may not quote one. The committed table shipped exactly
        // this: `SeTaxResult.addl` carried Schedule SE line 12's sentence on a row whose own reason
        // says it is not a Schedule SE line.
        let mut c = Coverage::default();
        c.exception(
            btctax_core::conventions::Usd::ZERO,
            "f1040sse",
            "(none)",
            "addl",
            "Self-employment tax. Add lines 10 and 11.",
            "not a Schedule SE line",
        );
        let e = check(&c).unwrap_err();
        assert!(
            e.contains("names no line yet quotes form text"),
            "rule (2c) must reject a quote on a \"(none)\" row: {e}"
        );

        // ★★ (2c) must not become an ESCAPE HATCH. A `(none)` row skips rules (2)/(2b) — it has no
        // line to bind to — but must still face every rule that reads its production. Here it declares
        // a floor clamp with no clause whatsoever; rule (3) has to reject it.
        let mut c = Coverage::default();
        c.line(
            btctax_core::conventions::Usd::ZERO,
            "f1040sse",
            "(none)",
            "addl",
            Production::Clamped(Polarity::FloorAtZero),
            "",
        );
        let e = check(&c).unwrap_err();
        assert!(
            e.contains("polarity is TRANSCRIBED"),
            "a \"(none)\" row must still face rule (3) — otherwise (2c) is a way OUT of the checker: {e}"
        );

        // ★★★ (2b) THE SUFFIX DIRECTION — the half the rule was BLIND to, and neither plant covered.
        // `match_indices` is a plain substring scan, so without a left word boundary a row naming line
        // N binds to any line whose printed label ENDS with N. Here f8995 line 5 carries line **15**'s
        // verbatim sentence, and `check()` returned Ok on it. r7 measured 71 such accepts across eight
        // forms. ★ The asymmetry is the fingerprint: line 15's text under line 5 was ACCEPTED while
        // line 5's text under line 15 was rejected — and both committed plants sat on the rejected
        // side, which is exactly how a rule ships holding in one direction only.
        let mut c = Coverage::default();
        c.line(
            btctax_core::conventions::Usd::ZERO,
            "f8995",
            "5",
            "line5",
            Production::Scaled,
            "Qualified business income deduction. Enter the smaller of line 10 or line 14. Also enter \
             this amount on the applicable line of your return (see instructions)",
        );
        let e = check(&c).unwrap_err();
        assert!(
            e.contains("NOT printed as line 5's own text"),
            "rule (2b) must reject line 15's sentence filed under line 5 — a substring match with no \
             left boundary finds `5 …` inside `15 …`: {e}"
        );

        // ★★ (2b) the stem CAPTION may not stand alone. 1040 prints `25 Federal income tax withheld
        // from:` as a caption with no amount box; line 25d's own text is `Add lines 25a through 25c`.
        // The caption is admitted only when the quote also carries the row's own bare letter.
        let mut c = Coverage::default();
        c.line(
            btctax_core::conventions::Usd::ZERO,
            "f1040",
            "25d",
            "line25d",
            Production::filer_records(
                "Include on line 25c any federal income tax withheld on your Form(s) W-2G.",
            ),
            "Federal income tax withheld from:",
        );
        let e = check(&c).unwrap_err();
        assert!(
            e.contains("NOT printed as line 25d's own text"),
            "rule (2b) must reject a stem caption quoted as a sub-line's own text: {e}"
        );

        // ★★★ AN EMPTY QUOTE IS NOT A QUOTE. `str::contains("")` is true for every haystack, so rule
        // (2) was vacuous; (2b) then built the needle `"5 "`, which occurs on every form. A `Collected`
        // row with a blank instruction faced NO rule and moved NO counter — a fully-passing, entirely
        // unverified row, and a cheaper evasion than the `field: _` one r6 named, because it leaves a
        // row that LOOKS classified.
        for blank in ["", "   "] {
            let mut c = Coverage::default();
            c.line(
                btctax_core::conventions::Usd::ZERO,
                "f8995",
                "5",
                "smuggled",
                Production::filer_records(
                    "Include on line 25c any federal income tax withheld on your Form(s) W-2G.",
                ),
                blank,
            );
            let e = check(&c).unwrap_err();
            assert!(
                e.contains("EMPTY instruction"),
                "a blank quote must be rejected outright, not pass vacuously: {e}"
            );
        }

        // ★★ (3b) A `Clamped` row needs a "-0-" clause. r7 (I-3) neutralised this and NOTHING red —
        // yet it is load-bearing for a live disposition: 1040 line 34's clause matches the FLOOR_IDIOM
        // "is more than line" and is kept OUT of `Clamped` only by this half. The escape-hatch plant
        // asserts on "polarity is TRANSCRIBED", which is (3a), a different rule.
        let mut c = Coverage::default();
        c.line(
            btctax_core::conventions::Usd::ZERO,
            "f1040",
            "34",
            "line34",
            Production::Clamped(Polarity::FloorAtZero),
            "If line 33 is more than line 24, subtract line 24 from line 33. This is the amount you \
             overpaid",
        );
        let e = check(&c).unwrap_err();
        assert!(
            !e.is_empty() && e.contains("34"),
            "(3b) must reject a Clamped row whose quote prints no -0- — this is what forces 1040 \
             line 34 into the Exception bucket rather than blessing a clamp the form never states: {e}"
        );

        // ★★★ THE RATCHETS ARE UNOBSERVABLE WHILE THE TABLE SITS AT THEM (11/11, 0/0, 8/8) — which is
        // precisely when they need a synthetic table to be watched. r7 (I-3) deleted all three and the
        // suite stayed green.
        let mut c = Coverage::default();
        for i in 0..=MAX_EXCEPTIONS {
            c.exception(
                btctax_core::conventions::Usd::ZERO,
                "f8995",
                "(none)",
                Box::leak(format!("filler{i}").into_boxed_str()),
                "",
                "a reason",
            );
        }
        let e = check(&c).unwrap_err();
        assert!(
            e.contains("exception"),
            "MAX_EXCEPTIONS must reject one more Exception than the ratchet: {e}"
        );

        let mut c = Coverage::default();
        for i in 0..=MAX_UNLOCATABLE {
            c.line(
                btctax_core::conventions::Usd::ZERO,
                "f8995",
                "QDCGT Worksheet, 3",
                Box::leak(format!("filler{i}").into_boxed_str()),
                Production::filer_records(
                    "Include on line 25c any federal income tax withheld on your Form(s) W-2G.",
                ),
                "Qualified business income component. Multiply line 4 by 20% (0.20)",
            );
        }
        let e = check(&c).unwrap_err();
        assert!(
            e.contains("cannot be located"),
            "MAX_UNLOCATABLE must reject one more unbindable row than the ratchet: {e}"
        );

        // `schedule_d` is the MAP's name; the extract is committed as `f1040sd`. So a row naming it has
        // a map and no text layer — which is exactly `unverifiable`, and the only way to reach that
        // counter without deleting a committed asset.
        let mut c = Coverage::default();
        for i in 0..=MAX_UNVERIFIABLE {
            c.line(
                btctax_core::conventions::Usd::ZERO,
                "schedule_d",
                "1",
                Box::leak(format!("filler{i}").into_boxed_str()),
                Production::filer_records(
                    "Include on line 25c any federal income tax withheld on your Form(s) W-2G.",
                ),
                "anything",
            );
        }
        let e = check(&c).unwrap_err();
        assert!(
            e.contains("unverifiable") || e.contains("no committed text"),
            "MAX_UNVERIFIABLE must reject one more unverifiable row than the ratchet: {e}"
        );

        // (3) a clamp declaring a polarity its own clause does not state.
        let mut c = Coverage::default();
        c.line(
            btctax_core::conventions::Usd::ZERO,
            "f8995",
            "16",
            "line16",
            Production::Clamped(Polarity::FloorAtZero),
            "Total qualified business (loss) carryforward. Combine lines 2 and 3. If greater than zero, enter -0-",
        );
        let e = check(&c).unwrap_err();
        assert!(
            e.contains("polarity is TRANSCRIBED"),
            "rule (3) must reject an inverted polarity — f8995 16 CEILS: {e}"
        );

        // (4) a Combine carrying a clamp clause — r2's C-2, the verb-keyed defect.
        let mut c = Coverage::default();
        c.line(
            btctax_core::conventions::Usd::ZERO,
            "f8995",
            "4",
            "line4",
            Production::Combine,
            "Total qualified business income. Combine lines 2 and 3. If zero or less, enter -0-",
        );
        let e = check(&c).unwrap_err();
        assert!(
            e.contains("it is Clamped"),
            "rule (4) must reject a Combine whose quote clamps: {e}"
        );

        // (1) an Exception with no reason — how a residual bucket starts.
        let mut c = Coverage::default();
        c.0.push(btctax_core::tax::line_coverage::LineCoverage {
            form: "f8995",
            year: btctax_core::tax::line_coverage::DEFAULT_ROW_YEAR,
            line: "5".to_string(),
            field: "line5",
            production: Production::Exception,
            instruction: "Qualified business income component. Multiply line 4 by 20% (0.20)",
            reason: None,
        });
        let e = check(&c).unwrap_err();
        assert!(
            e.contains("NO reason"),
            "rule (1) must reject an unexplained Exception: {e}"
        );

        // (1, other direction) a reason on a real production — a sign the author was unsure.
        let mut c = Coverage::default();
        c.0.push(btctax_core::tax::line_coverage::LineCoverage {
            form: "f8995",
            year: btctax_core::tax::line_coverage::DEFAULT_ROW_YEAR,
            line: "5".to_string(),
            field: "line5",
            production: Production::Scaled,
            instruction: "Qualified business income component. Multiply line 4 by 20% (0.20)",
            reason: Some("because"),
        });
        let e = check(&c).unwrap_err();
        assert!(
            e.contains("reasons are for Exceptions only"),
            "rule (1) must reject a reason on a non-Exception: {e}"
        );

        // (5) the same (form, line, field) twice — two rows that could disagree.
        let mut c = Coverage::default();
        good(&mut c);
        good(&mut c);
        let e = check(&c).unwrap_err();
        assert!(
            e.contains("is covered 2 times"),
            "rule (5) must reject duplicate coverage of one line: {e}"
        );

        // ★★★ (7) THE `Collected` SOURCE. Five plants, because the rule has five ways to be gutted
        //     and a single one would leave four of them silent. The clean row each plant is derived
        //     from is 1040 line 2b — Collected from Form 1099-INT box 1 — so nothing but the source
        //     naming differs between pass and fail.
        let clean_docbox = |c: &mut Coverage| {
            c.line(
                btctax_core::conventions::Usd::ZERO,
                "f1040",
                "2b",
                "line2b",
                Production::doc_box("f1099int", "1"),
                "Taxable interest",
            )
        };
        let mut c = Coverage::default();
        clean_docbox(&mut c);
        assert!(
            check(&c).is_ok(),
            "the DocBox control row must PASS — otherwise every plant below passes for the wrong \
             reason: {:?}",
            check(&c)
        );

        /// (what it is, the source to plant, the substring the refusal must contain).
        type SourcePlant = (
            &'static str,
            btctax_core::tax::line_coverage::CollectedFrom,
            &'static str,
        );
        use btctax_core::tax::line_coverage::CollectedFrom;
        let source_plants: &[SourcePlant] = &[
            // (7a) ★★★ THE C1 KILL — a box that EXISTS, on a row whose year it does not exist in.
            //      Form 1099-G box 10 is "Family leave benefits" on the Rev. December 2026 grid and
            //      is not printed at all on the Rev. March 2024 one, which is the edition a TY2024
            //      filer holds. Before the fold, a row pinned its own document year and this passed.
            (
                "a DocBox naming a box only a LATER edition prints",
                CollectedFrom::DocBox {
                    stem: "f1099g",
                    box_label: "10",
                    also_labels: &[],
                },
                "which that form does not print",
            ),
            // (7b) A box no edition prints — the stale entry after a revision renumbers.
            (
                "a DocBox naming a box the form does not print at all",
                CollectedFrom::DocBox {
                    stem: "f1099int",
                    box_label: "99",
                    also_labels: &[],
                },
                "which that form does not print",
            ),
            // (7c) A document that is not archived at all: nothing would ever check the caption.
            (
                "a DocBox naming an unarchived document",
                CollectedFrom::DocBox {
                    stem: "f1099nec",
                    box_label: "1",
                    also_labels: &[],
                },
                "no such information return is archived",
            ),
            // (7d) ★★★ THE I1 KILL, and it is the reviewer's own plant: Schedule A line 5b — state
            //      and local REAL ESTATE TAXES — pointed at the Form 6251 sentence about the AMT
            //      foreign tax credit. It passed green before the fold, because nothing bound the
            //      booklet to the row's form. It cannot even be SPELLED now (the variant carries no
            //      stem), so what is planted is the sentence itself: `i1040sca` does not contain it.
            (
                "a FilerRecords sentence from another form's booklet",
                CollectedFrom::FilerRecords {
                    instruction_line: "The AMTFTC is a credit that you can claim against the AMT.",
                },
                "NAMES line",
            ),
            // (7e) A sentence that IS in this form's own booklet, but under another line's heading —
            //      the second half of I1, and the harder one.
            (
                "a FilerRecords sentence from another LINE of the right booklet",
                CollectedFrom::FilerRecords {
                    instruction_line: "Enter the total of your medical and dental expenses, after \
                                       you reduce these expenses by any payments received from \
                                       insurance or other sources.",
                },
                "neither inside a `Line",
            ),
            // (7f) An empty quote: `contains("")` is true for every haystack.
            (
                "a FilerRecords with a blank instruction line",
                CollectedFrom::FilerRecords {
                    instruction_line: "   ",
                },
                "EMPTY instruction line",
            ),
        ];
        for (what, from, needle) in source_plants {
            // ★ The two `FilerRecords` plants are planted on the row the reviewer used — Schedule A
            //   line 5b — because the booklet is now DERIVED FROM THE ROW'S FORM, so which row a
            //   sentence is planted on is the whole question.
            let filer = matches!(from, CollectedFrom::FilerRecords { .. });
            let (form, line, field, instruction) = if filer {
                (
                    "f1040sa",
                    "5b",
                    "line5b",
                    "State and local real estate taxes (see instructions)",
                )
            } else {
                ("f1040", "2b", "line2b", "Taxable interest")
            };
            let mut c = Coverage::default();
            c.line(
                btctax_core::conventions::Usd::ZERO,
                form,
                line,
                field,
                Production::Collected(*from),
                instruction,
            );
            let e = check(&c)
                .err()
                .unwrap_or_else(|| panic!("planting {what} must RED, and it did not"));
            assert!(
                e.contains(needle),
                "planting {what} red for the wrong reason: expected {needle:?}, got:\n{e}"
            );
        }

        // ★★ The control for (7d)/(7e): the row's OWN sentence, under its OWN line, PASSES — so the
        //    two reds above are the binding working, not Schedule A line 5b being unquotable.
        let mut c = Coverage::default();
        c.line(
            btctax_core::conventions::Usd::ZERO,
            "f1040sa",
            "5b",
            "line5b",
            Production::filer_records(
                "Enter on line 5b the state and local taxes you paid on real estate you own that \
                 wasn't used for business",
            ),
            "State and local real estate taxes (see instructions)",
        );
        assert!(
            check(&c).is_ok(),
            "the FilerRecords control row must PASS: {:?}",
            check(&c)
        );

        // A form with neither an extract nor a map — a typo'd stem.
        let mut c = Coverage::default();
        c.line(
            btctax_core::conventions::Usd::ZERO,
            "f9999",
            "1",
            "line1",
            Production::filer_records(
                "Include on line 25c any federal income tax withheld on your Form(s) W-2G.",
            ),
            "anything",
        );
        let e = check(&c).unwrap_err();
        assert!(
            e.contains("the form name is wrong"),
            "a stem with no extract and no map must be an error, not silently unverifiable: {e}"
        );

        // An EMPTY table must not pass vacuously.
        let e = check(&Coverage::default()).unwrap_err();
        assert!(
            e.contains("EMPTY"),
            "an empty table must not vacuously pass: {e}"
        );
    }
}
