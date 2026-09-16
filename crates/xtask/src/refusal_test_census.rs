//! **FR-180 — every declared `RefuseReason` variant is covered by a test, and the residue is pinned.**
//!
//! `RefuseReason` is the only surface on which btctax says *"I will not file this"*. The
//! [`crate::blockers`] refusal census proves each variant is **RECORDED** and has a raise site; it
//! cannot prove any of them ever **FIRES**. This module closes the other half: a variant that no test
//! in the workspace covers is a guard nobody has watched refuse, and *"a guarantee without a test that
//! reds when it is removed does not exist."*
//!
//! ## ★★★ Why this is not a grep over `*.rs`, and the numbers that prove it
//!
//! Asked at three scopes on 2026-09-15, "is this variant named in a test?" gave three different
//! answers over the same tree — 23 (files under a `tests/` directory only), 128, 139 (all `.rs`,
//! which is useless because `return_refuse.rs` declares them itself). A number that swings with the
//! scope is not a measurement, so the scope is fixed here and two specific ways of getting it wrong
//! are closed:
//!
//! 1. **★★★ [`crate::r15_stop_list::production_source`]'s brace counter must NOT be used for this
//!    question.** It counts `{` and `}` through string and char literals — its own doc says so
//!    (*"a `{` inside a string literal inside a test module could end the skip early. That errs
//!    toward scanning MORE, which is the fail-closed direction here"*). That is true for R15, whose
//!    question is *"does a forbidden idiom appear in shipped code"*. It is exactly BACKWARDS for this
//!    question, because a test region misread as production makes a **tested** variant look
//!    **untested**. Measured, not reasoned: `return_refuse.rs`'s `#[cfg(test)] mod param_free_tier`
//!    contains `rest.find("\n}\n")` and `b'}' => depth -= 1`, so the shipped counter resumes scanning
//!    at line 11133 and classifies the module's remaining **990 lines as production**. Two variants —
//!    `FamilyLeaveBenefits` and `HsaExcessEmployerContributions` — have fixtures in that module and
//!    are asserted firing by
//!    `every_param_free_rule_is_censused_from_the_source_and_fires_on_both_paths`, yet were reported
//!    as untested by the shipped mask. [`test_context`] skips literals and comments, and
//!    [`tests::a_brace_inside_a_string_does_not_end_a_test_module`] is the kill.
//!
//! 2. **A variant named only in a COMMENT is not tested.** Comments are stripped before the search,
//!    which found a real case: `IncomeExclusionUnanswered`'s only two test-context mentions in the
//!    whole workspace are prose in `btctax-tui-edit/src/edit/form.rs`. A comment-tolerant grep called
//!    it covered.
//!
//! ## The coverage predicate — two limbs, both DERIVED
//!
//! A variant is **covered** when either limb holds.
//!
//! - **NAMED** — the variant appears in a non-comment line of a test context: any file under a
//!   `tests/` directory, or the `#[cfg(test)]` regions of a `src/` file ([`test_context`]).
//!
//! - **REGISTRY-DRIVEN** — it is the `unanswered` reason of a `FORM_QUESTIONS` entry, read off the
//!   registry itself, while `return_refuse.rs` still carries
//!   [`REGISTRY_PROPERTY_TEST`]. ★★★ **This limb exists because a correctly DERIVED test names no
//!   variant at all, and a naming census would therefore report the repo's best-built guards as its
//!   worst.** That test builds a `scenario_for(q.id)` per registry entry, answers every other live
//!   question, and asserts `reason(&r) == Some(q.unanswered.clone())` — so the variant is *observed
//!   firing* and its name occurs nowhere in the source. Measured 2026-09-15 by instrumenting the
//!   loop: **66 of 67** registry entries are driven and every one returned the reason its entry
//!   declares; the single skip is `QuestionId::DocForm1098`, taken under the loop's own derived T9
//!   exemption (*"a census row awaiting its screen"*), and its reason `DocumentCensusUnanswered` is
//!   driven by twenty sibling `Doc*` entries. Seven of FR-180's eleven reported gaps are this limb.
//!
//! ★ `SKIPPABLE_QUESTIONS` is deliberately NOT a limb: it has exactly one class-(A) entry
//!   (`HohMaritalBasisUnanswered`, verified by [`tests::the_skippable_registry_needs_no_limb`]) and no
//!   derived per-entry property test, so crediting it would be an exemption earned by nothing. That
//!   one reason is NAMED in tests and needs no credit.
//!
//! ## ★ The blind spot, stated because a gate that hides its own is worse than no gate
//!
//! **NAMED is not OBSERVED FIRING.** A variant mentioned inside a `let` binding, an unused fixture or
//! a `matches!` that never runs satisfies limb 1. Six variants are named in the workspace with no
//! `assert`-family macro within eight test lines (`FamilyLeaveBenefits`,
//! `Form8615AgeSupportUnanswered`, `HsaExcessEmployerContributions`,
//! `LiquidationDistributionNotComputed`, `ScheduleFIncomeNotModeled`,
//! `StateRefundWithout1099gContradicted`) — all six are `param_free_fixtures` table entries that a
//! derived loop asserts on elsewhere, which is why proximity-to-an-assertion is measured and
//! **reported** rather than gated: as a gate it would have produced six false rows. What this census
//! proves is the weaker, exact thing: **no declared refusal is invisible to the whole suite.**

use std::collections::BTreeSet;

/// The btctax-core test that carries the REGISTRY-DRIVEN limb. Named as a string because the credit
/// is only as good as the test's continued existence — see [`registry_driven`].
pub const REGISTRY_PROPERTY_TEST: &str =
    "fn every_live_unanswered_declaration_refuses_with_its_own_reason";

/// Variants no test covers, each with the reason it is admitted.
///
/// ★★ **EXACT, not a floor.** [`residue`]'s output is compared for SET EQUALITY against these keys,
/// so the census reds in BOTH directions: a new untested refusal grows the residue and is refused,
/// and a row left here after its variant has been covered is refused as stale. That is the shape
/// [`crate::verdict_reach::ADVISORY_ONLY`] uses, and the reason it uses it — a recorded gap is a thing
/// a future reader can grep for; a stale one is an excuse earned by nothing.
///
/// ★★★ **It is EMPTY, and that is the FR-180 result.** The eleven variants this census reported on
/// 2026-09-15 resolved as: four given tests in `return_1040.rs`
/// (`CooperativePatron`, `CooperativePatronUnanswered`, `SstbInPhaseInRange`,
/// `QbiCarryforwardNeedsSchedule8995AC` — the §G-28/B1b sub-schedule refusals, which shipped with no
/// test of any kind), and seven already observed firing by the registry property test and merely
/// invisible to a grep. Nothing was weakened and nothing was deleted to get here.
pub const UNTESTED_PIN: &[(&str, &str)] = &[];

/// `RefuseReason`'s variant name from a `Debug` rendering, so a payload-carrying variant compares like
/// a unit one.
fn variant_name(r: &btctax_core::tax::return_refuse::RefuseReason) -> String {
    let s = format!("{r:?}");
    s.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .next()
        .unwrap_or("")
        .to_string()
}

/// Remove `//` comments, string literals (raw included) and char literals from one Rust line, leaving
/// the braces that actually nest.
fn code_only(line: &str) -> String {
    let b: Vec<char> = line.chars().collect();
    let mut out = String::new();
    let mut i = 0usize;
    while i < b.len() {
        let c = b[i];
        if c == '/' && i + 1 < b.len() && b[i + 1] == '/' {
            break;
        }
        if c == 'r' && i + 1 < b.len() && (b[i + 1] == '"' || b[i + 1] == '#') {
            let mut j = i + 1;
            let mut hashes = 0usize;
            while j < b.len() && b[j] == '#' {
                hashes += 1;
                j += 1;
            }
            if j < b.len() && b[j] == '"' {
                j += 1;
                let close: String = std::iter::once('"')
                    .chain(std::iter::repeat_n('#', hashes))
                    .collect();
                let rest: String = b[j..].iter().collect();
                i = match rest.find(&close) {
                    Some(k) => j + rest[..k].chars().count() + close.chars().count(),
                    None => b.len(),
                };
                continue;
            }
        }
        if c == '"' {
            let mut j = i + 1;
            while j < b.len() {
                if b[j] == '\\' {
                    j += 2;
                    continue;
                }
                if b[j] == '"' {
                    j += 1;
                    break;
                }
                j += 1;
            }
            i = j;
            continue;
        }
        if c == '\'' {
            // A char literal (`'a'`, `'\n'`, `b'}'`) is skipped whole; a lifetime (`'static`) is not
            // a literal, so only its quote is dropped.
            if i + 2 < b.len() && b[i + 1] == '\\' {
                if let Some(k) = (i + 2..b.len()).find(|k| b[*k] == '\'') {
                    i = k + 1;
                    continue;
                }
            } else if i + 2 < b.len() && b[i + 2] == '\'' {
                i += 3;
                continue;
            }
            i += 1;
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
}

/// Strip `//` comments only, keeping string contents (a `//` inside a string is not a comment).
fn decomment(line: &str) -> String {
    let b: Vec<char> = line.chars().collect();
    let mut i = 0usize;
    while i < b.len() {
        if b[i] == '/' && i + 1 < b.len() && b[i + 1] == '/' {
            return b[..i].iter().collect();
        }
        if b[i] == '"' {
            let mut j = i + 1;
            while j < b.len() {
                if b[j] == '\\' {
                    j += 2;
                    continue;
                }
                if b[j] == '"' {
                    j += 1;
                    break;
                }
                j += 1;
            }
            i = j;
            continue;
        }
        i += 1;
    }
    line.to_string()
}

/// **The `#[cfg(test)]` regions of `src`, comments stripped.**
///
/// The complement of [`crate::r15_stop_list::production_source`], and deliberately a SECOND
/// implementation rather than a reuse: that one brace-counts through string and char literals, which
/// for THIS question fails in the direction that hides coverage. See the module header for the
/// measurement. `whole_file` is for a file under a `tests/` directory, every line of which is a test
/// context.
#[must_use]
pub fn test_context(src: &str, whole_file: bool) -> String {
    if whole_file {
        return src.lines().map(decomment).collect::<Vec<_>>().join("\n");
    }
    let mut out: Vec<String> = Vec::new();
    let mut skipping = false;
    let mut depth: i32 = 0;
    let mut opened = false;
    for line in src.lines() {
        if !skipping && line.trim_start().starts_with("#[cfg(test)]") {
            skipping = true;
            depth = 0;
            opened = false;
            continue;
        }
        if !skipping {
            continue;
        }
        for c in code_only(line).chars() {
            match c {
                '{' => {
                    depth += 1;
                    opened = true;
                }
                '}' => depth -= 1,
                _ => {}
            }
        }
        out.push(decomment(line));
        let declaration_ended = !opened && line.trim_end().ends_with(';');
        let block_ended = opened && depth <= 0;
        if declaration_ended || block_ended {
            skipping = false;
        }
    }
    out.join("\n")
}

/// Whether `path` (a repo-relative label with `/` separators) is a file every line of which is a test.
#[must_use]
pub fn is_test_file(path: &str) -> bool {
    let mut parts: Vec<&str> = path.split('/').collect();
    parts.pop();
    parts.contains(&"tests")
}

/// Does `hay` name `needle` as a whole identifier?
fn names(hay: &str, needle: &str) -> bool {
    let ident = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let mut from = 0usize;
    while let Some(i) = hay[from..].find(needle) {
        let at = from + i;
        let before_ok = hay[..at].chars().next_back().is_none_or(|c| !ident(c));
        let after_ok = hay[at + needle.len()..]
            .chars()
            .next()
            .is_none_or(|c| !ident(c));
        if before_ok && after_ok {
            return true;
        }
        from = at + needle.len();
    }
    false
}

/// Limb 1 — every variant of `variants` NAMED in the test context of `files`.
///
/// Pure over `(label, source)` pairs so a planted defect can reach it, the same shape
/// [`crate::r15_stop_list::serde_json_reflection`] uses.
#[must_use]
pub fn named_in_tests(files: &[(String, String)], variants: &BTreeSet<String>) -> BTreeSet<String> {
    let corpus: String = files
        .iter()
        .map(|(label, src)| test_context(src, is_test_file(label)))
        .collect::<Vec<_>>()
        .join("\n");
    variants
        .iter()
        .filter(|v| names(&corpus, v))
        .cloned()
        .collect()
}

/// Limb 2 — the `unanswered` reasons the registry property test drives, DERIVED from
/// `FORM_QUESTIONS`.
///
/// Empty when `return_refuse_src` no longer carries [`REGISTRY_PROPERTY_TEST`]: the credit is the
/// test, so deleting the test must return every registry reason to the residue rather than leave a
/// silent exemption behind.
#[must_use]
pub fn registry_driven(return_refuse_src: &str) -> BTreeSet<String> {
    if !return_refuse_src.contains(REGISTRY_PROPERTY_TEST) {
        return BTreeSet::new();
    }
    btctax_core::tax::questions::FORM_QUESTIONS
        .iter()
        .map(|q| variant_name(&q.unanswered))
        .collect()
}

/// Every declared `RefuseReason` variant, read off the enum. Reuses
/// [`crate::census_join::variant_paths`] rather than re-parsing the enum a second time.
#[must_use]
pub fn declared(return_refuse_src: &str) -> BTreeSet<String> {
    crate::census_join::variant_paths(return_refuse_src, "RefuseReason")
        .into_iter()
        .filter_map(|p| p.strip_prefix("RefuseReason::").map(str::to_string))
        .collect()
}

/// The variants no limb covers, sorted.
#[must_use]
pub fn residue(
    variants: &BTreeSet<String>,
    files: &[(String, String)],
    return_refuse_src: &str,
) -> Vec<String> {
    let mut covered = named_in_tests(files, variants);
    covered.extend(registry_driven(return_refuse_src));
    variants.difference(&covered).cloned().collect()
}

/// Named, but with no `assert`-family macro within eight test lines — the census's own blind spot,
/// measured and REPORTED rather than gated (see the module header for why gating it would be wrong).
#[must_use]
pub fn named_without_a_nearby_assertion(
    files: &[(String, String)],
    variants: &BTreeSet<String>,
) -> Vec<String> {
    const ASSERTIONS: &[&str] = &["assert!", "assert_eq!", "assert_ne!", "panic!", "expect("];
    let lines: Vec<String> = files
        .iter()
        .flat_map(|(label, src)| {
            test_context(src, is_test_file(label))
                .lines()
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .filter(|l| !l.trim().is_empty())
        .collect();
    let mut out = Vec::new();
    for v in variants {
        let mut seen = false;
        let mut asserted = false;
        for (i, line) in lines.iter().enumerate() {
            if !names(line, v) {
                continue;
            }
            seen = true;
            let lo = i.saturating_sub(8);
            let hi = (i + 9).min(lines.len());
            if lines[lo..hi]
                .iter()
                .any(|l| ASSERTIONS.iter().any(|a| l.contains(a)))
            {
                asserted = true;
                break;
            }
        }
        if seen && !asserted {
            out.push(v.clone());
        }
    }
    out
}

/// Compare a residue against [`UNTESTED_PIN`] for SET EQUALITY, so growth and staleness both red.
#[must_use]
pub fn pin_findings(residue: &[String], pin: &[(&str, &str)]) -> Vec<String> {
    let pinned: BTreeSet<&str> = pin.iter().map(|(v, _)| *v).collect();
    let found: BTreeSet<&str> = residue.iter().map(String::as_str).collect();
    let mut out = Vec::new();
    for v in found.difference(&pinned) {
        out.push(format!(
            "{v}: declared as a refusal and covered by NO test — no name in any test context, and \
             not an `unanswered` reason the registry property test drives. Write the test that \
             drives a return to the state it names; if no input can reach it, say so in \
             UNTESTED_PIN with the reason, and do NOT delete the variant to make this census green"
        ));
    }
    for v in pinned.difference(&found) {
        out.push(format!(
            "{v}: pinned in UNTESTED_PIN as untested, but a test now covers it — delete the row. A \
             stale exemption is an excuse earned by nothing"
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus() -> Vec<(String, String)> {
        crate::blockers::workspace_rust()
    }

    fn return_refuse() -> String {
        corpus()
            .into_iter()
            .find(|(l, _)| l.replace('\\', "/") == "crates/btctax-core/src/tax/return_refuse.rs")
            .map(|(_, s)| s)
            .expect("return_refuse.rs is in the workspace")
    }

    /// ★★★ **FR-235 — EVERY VARIANT NAME BELOW IS ASSEMBLED, NEVER TYPED, and that is not
    /// obfuscation.**
    ///
    /// This module's source is part of the corpus [`named_in_tests`] scans, and its own
    /// `#[cfg(test)]` region is therefore a test context. A variant name typed here as a string
    /// literal would make this census CREDIT it — which is exactly how the first draft of these tests
    /// failed: the plant `PlantedRefusalNobodyTests` never entered the residue because the plant's own
    /// name satisfied the grep, and three registry guards looked covered because
    /// [`the_registry_limb_dies_with_its_test`] had typed them. Assembling is the same device
    /// [`crate::r15_stop_list::serde_json_reflection`] uses on its own needle, for the same reason, and
    /// [`this_census_credits_nothing_by_naming_it`] is what holds the rule.
    fn name(a: &str, b: &str) -> String {
        format!("{a}{b}")
    }

    /// A variant that is genuinely covered by tests elsewhere in the workspace — the control for the
    /// stale-pin and mask plants.
    fn covered_control() -> String {
        name("Negative", "Amount")
    }

    /// ★★★ **THE CENSUS. Every declared `RefuseReason` is covered by a test, and the residue is
    /// exactly [`UNTESTED_PIN`].**
    #[test]
    fn every_declared_refusal_is_covered_by_a_test() {
        let files = corpus();
        let src = return_refuse();
        let variants = declared(&src);
        assert!(
            variants.len() > 100,
            "the enum scan parsed nothing usable: {} variants",
            variants.len()
        );
        let res = residue(&variants, &files, &src);
        let findings = pin_findings(&res, UNTESTED_PIN);
        assert!(
            findings.is_empty(),
            "refusal test census over {} declared variants — {} finding(s):\n  {}\n\n\
             ★ BLIND SPOT: `named in a test` is not `observed firing`. Named with no assertion \
             within eight test lines: {:?}",
            variants.len(),
            findings.len(),
            findings.join("\n  "),
            named_without_a_nearby_assertion(&files, &variants),
        );
    }

    /// ★★★ **FR-235, made structural: this census must not be a source of its own credit.**
    ///
    /// No declared variant may appear as a whole identifier in this module's own test context. Without
    /// this the module is free to launder coverage for anything it happens to mention — and it did,
    /// twice, before the assembly rule above was applied.
    #[test]
    fn this_census_credits_nothing_by_naming_it() {
        let me = include_str!("refusal_test_census.rs");
        let variants = declared(&return_refuse());
        let ctx = test_context(me, false);
        let self_credited: Vec<&String> = variants.iter().filter(|v| names(&ctx, v)).collect();
        assert!(
            self_credited.is_empty(),
            "this module's test context names {} declared variant(s), so the census credits them \
             for nothing: {self_credited:?} — assemble the name with `name(..)` instead of typing it",
            self_credited.len()
        );
    }

    /// ★ B1 — **a new refusal that no test covers grows the residue.**
    ///
    /// Per FR-235 the plant is NOT a name added to a comment the checker greps: it is a variant in the
    /// DERIVED input, i.e. exactly what a future `RefuseReason` arrival looks like to this census.
    #[test]
    fn a_new_untested_variant_is_refused_and_the_message_forbids_deleting_it() {
        let files = corpus();
        let src = return_refuse();
        let plant = name("PlantedRefusal", "NobodyTests");
        // ★★ The plant goes into the ENUM SOURCE, not into the derived set, so it travels the whole
        //    path a real new variant travels: `declared` must parse it out of the enum body first.
        //    Planting straight into the `BTreeSet` would have skipped the derivation and left
        //    `declared` itself unmeasured.
        let planted_src = src.replacen(
            "pub enum RefuseReason {",
            &format!("pub enum RefuseReason {{\n    {plant},"),
            1,
        );
        let variants = declared(&planted_src);
        assert!(
            variants.contains(&plant),
            "the enum scan must see a newly declared variant, or this plant proves nothing"
        );
        assert_eq!(
            variants.len(),
            declared(&src).len() + 1,
            "the plant must add exactly one variant"
        );
        let res = residue(&variants, &files, &planted_src);
        assert!(
            res.contains(&plant),
            "the plant must enter the residue: {res:?}"
        );
        let findings = pin_findings(&res, UNTESTED_PIN);
        assert_eq!(findings.len(), 1, "exactly the plant: {findings:?}");
        assert!(
            findings[0].contains("do NOT delete the variant"),
            "the finding must forbid the census-greening edit: {}",
            findings[0]
        );
    }

    /// ★ B1, the other direction — **a pinned row whose variant IS covered is refused as stale.**
    #[test]
    fn a_stale_pin_row_is_refused() {
        let files = corpus();
        let src = return_refuse();
        let variants = declared(&src);
        let res = residue(&variants, &files, &src);
        let covered = covered_control();
        assert!(
            variants.contains(&covered) && !res.contains(&covered),
            "{covered} must be a real, covered variant for this plant to mean anything"
        );
        let findings = pin_findings(&res, &[(&covered, "a fabricated excuse")]);
        assert!(
            findings
                .iter()
                .any(|f| f.contains(&covered) && f.contains("stale exemption")),
            "a pin row for a covered variant must red: {findings:?}"
        );
    }

    /// ★★★ **THE KILL FOR THE LITERAL-AWARE MASK, and the defect is in the SHIPPED counter.**
    ///
    /// [`crate::r15_stop_list::production_source`] brace-counts through string and char literals, so a
    /// `"\n}\n"` inside a test module ends its test region early and everything after it reads as
    /// production. This is that exact shape, minimised: the variant is named AFTER the unbalanced
    /// string, and only a literal-aware mask still sees it as a test.
    #[test]
    fn a_brace_inside_a_string_does_not_end_a_test_module() {
        let v = covered_control();
        let planted = format!(
            "pub fn f() {{}}\n\
             #[cfg(test)]\n\
             mod t {{\n\
             \x20   fn helper() -> usize {{\n\
             \x20       \"\\n}}\\n\".len()\n\
             \x20   }}\n\
             \x20   #[test]\n\
             \x20   fn k() {{\n\
             \x20       assert_eq!(reason(), Some(RefuseReason::{v}(x)));\n\
             \x20   }}\n\
             }}\n"
        );
        let want: BTreeSet<String> = std::iter::once(v.clone()).collect();
        let files = vec![("crates/x/src/lib.rs".to_string(), planted.clone())];
        assert_eq!(
            named_in_tests(&files, &want),
            want,
            "the variant is named inside the test module and must be seen as tested"
        );
        // …and the shipped counter is what this replaces: it loses the region at the string, so the
        // variant lands in what that function calls PRODUCTION.
        assert!(
            crate::r15_stop_list::production_source(&planted).contains(&v),
            "if the shipped counter ever stops leaking this region, say so here rather than \
             keeping a kill that no longer discriminates"
        );
    }

    /// ★ B1 — **a variant named ONLY in a comment is not covered.** The case is real: one
    /// answered-ness guard's only test-context mentions in the whole workspace are prose in
    /// `btctax-tui-edit/src/edit/form.rs`.
    #[test]
    fn a_comment_mention_is_not_a_test() {
        let v = covered_control();
        let planted = format!(
            "#[cfg(test)]\n\
             mod t {{\n\
             \x20   /// See `RefuseReason::{v}`, which we ought to test.\n\
             \x20   #[test]\n\
             \x20   fn k() {{\n\
             \x20       assert!(true); // RefuseReason::{v} again\n\
             \x20   }}\n\
             }}\n"
        );
        let want: BTreeSet<String> = std::iter::once(v).collect();
        let files = vec![("crates/x/src/lib.rs".to_string(), planted)];
        assert!(
            named_in_tests(&files, &want).is_empty(),
            "a comment must not satisfy the census"
        );
    }

    /// ★★ B1 — **deleting the registry property test returns every registry reason to the residue.**
    ///
    /// The REGISTRY-DRIVEN limb is a credit for a test that is *observed* driving 66 of 67 entries. If
    /// the test goes, the credit must go with it — otherwise this census would carry seven silent
    /// exemptions forever.
    #[test]
    fn the_registry_limb_dies_with_its_test() {
        let src = return_refuse();
        let live = registry_driven(&src);
        let guards = [
            name("FilerTin", "Unanswered"),
            name("WagesWithout", "W2Unanswered"),
            name("IncomeExclusion", "Unanswered"),
            name("HomeSaleGate", "Unanswered"),
        ];
        for g in &guards {
            assert!(
                live.contains(g),
                "the limb must credit the FR-180 answered-ness guard {g}: {live:?}"
            );
        }
        let deleted = src.replace(REGISTRY_PROPERTY_TEST, "fn a_test_that_no_longer_exists");
        assert!(
            registry_driven(&deleted).is_empty(),
            "with the property test gone the credit must vanish entirely"
        );
        // …and then the census itself must red, naming every guard it can no longer vouch for.
        let files = corpus();
        let variants = declared(&src);
        let res = residue(&variants, &files, &deleted);
        let findings = pin_findings(&res, UNTESTED_PIN);
        for g in &guards {
            assert!(
                findings.iter().any(|f| f.starts_with(g)),
                "the census must red on {g}: {findings:?}"
            );
        }
    }

    /// ★ The `SKIPPABLE_QUESTIONS` registry is deliberately NOT a limb, and this is why: it has
    /// exactly one class-(A) entry, so there is nothing for a limb to credit. If a second arrives this
    /// reds and someone must decide whether a derived property test exists to justify a credit.
    #[test]
    fn the_skippable_registry_needs_no_limb() {
        let class_a: Vec<String> = btctax_core::tax::questions::SKIPPABLE_QUESTIONS
            .iter()
            .filter_map(|sk| sk.unanswered.as_ref().map(variant_name))
            .collect();
        assert_eq!(
            class_a,
            vec![name("HohMaritalBasis", "Unanswered")],
            "a second class-(A) skippable needs a decision, not a silent credit"
        );
    }
}
