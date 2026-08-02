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
fn normalize(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
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
const MAX_EXCEPTIONS: usize = 11;
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
    //   caption — so it binds after stripping. Form 8949's `I-1(d)` does not: its quote is the column
    //   HEADER ("Proceeds"), which is not any line's text, so it stays unlocatable by construction
    //   rather than by being forced onto line 1.
    let label = label.split_once('(').map_or(label, |(l, _)| l);
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
        // A lettered sub-line prints as a bare letter (`b Taxable interest`), and its stem may carry
        // the lead-in that the transcription rightly quotes (`25 Federal income tax withheld from:`).
        v.push(suffix.to_string());
        v.push(stem);
    }
    Some(v)
}

/// Is `quote` printed as line `label`'s own text — i.e. does some form of the label sit immediately
/// before it? `text` must already be whitespace-normalized.
fn label_precedes(text: &str, label: &str, quote: &str) -> Option<bool> {
    let forms = label_forms(label)?;
    let stem: String = label.chars().take_while(char::is_ascii_digit).collect();
    Some(forms.iter().any(|f| {
        let needle = format!("{f} {quote}");
        text.match_indices(&needle).any(|(i, _)| {
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
        } else {
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
            for (name, body) in money_bearing_types(&src) {
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
                if !mentions_ident(&emitter_code, &name) {
                    continue;
                }
                let want = format!("fn cover_{}(", name.to_lowercase());
                if !cov_src.contains(&want) {
                    errs.push(format!(
                        "{rel}: type `{name}` declares a Usd field but has no `cover_{}()` — a \
                         money-bearing printed type that nothing in the table mentions is exactly the \
                         gap nested money was",
                        name.to_lowercase()
                    ));
                }
            }
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
    // The residue, by class: 12 Form 8949 column cells whose quote is a column HEADER rather than any
    // line's text, and the Qualified Dividends & Capital Gain Tax Worksheet line 3, which lives in the
    // instructions booklet and has no form line label. Both are stated, neither is a form line.
    const MAX_UNLOCATABLE: usize = 13;
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

    /// The checker passes on the committed table.
    #[test]
    fn the_committed_coverage_table_is_consistent_with_the_form_text() {
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
            Production::Collected,
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

        // A form with neither an extract nor a map — a typo'd stem.
        let mut c = Coverage::default();
        c.line(
            btctax_core::conventions::Usd::ZERO,
            "f9999",
            "1",
            "line1",
            Production::Collected,
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
