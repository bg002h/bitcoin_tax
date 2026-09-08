//! ★★★ **R15 — THE STOP LIST, AS EXECUTABLE REQUIREMENTS** (`SPEC_interview.md` R15).
//!
//! R15 is a list of things that must never happen. A stop list nobody executes is prose, and prose
//! is exactly what the shapes below crept back in past. So each sentence gets a grep, and each grep
//! gets a planted defect it has been watched going RED on (harness B1).
//!
//! **Four checks, three sentences** — *"no progress bar; no persisted 'what remains'"* is two
//! different claims about two different things, and needs a grep each:
//!
//! | R15 sentence | check |
//! |---|---|
//! | *"No representation of the form exists outside the Rust structs and the map census"* | [`serde_json_reflection`] — no `serde_json::Value` reflection in `btctax-input-form`'s PRODUCTION code |
//! | *"No progress bar; no persisted 'what remains'"* | [`progress_shaped_fields`] — no `progress` / `remaining` / `position` field on the persisted input surfaces or the panel |
//! | *"No progress bar; no persisted 'what remains'"* | [`progress_widgets`] — no `Gauge` / `LineGauge` and no formatted percentage in the files that RENDER the answer panel |
//! | *"No `reconcile` question in a return registry"* | [`ledger_words_in_registry_prompts`] — no registry prompt says *transfer*, *lot* or *FMV* |
//!
//! ★ **Why it lives in xtask.** It reads across crate boundaries (`btctax-input-form`'s sources,
//! `btctax-core`'s), which an `include_str!` from inside a published crate cannot do without
//! shipping a broken tarball — the trap `crate-publishing-state` records and
//! `package_check.rs` gates. So the reading travels here, exactly as `line_coverage_check.rs` does.

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("crates/xtask -> repo root")
        .to_path_buf()
}

/// Every `.rs` file under a directory, sorted.
fn rs_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

/// ★ The PRODUCTION half of a source file: every line outside a `#[cfg(test)]` item, with `//`
/// comments stripped.
///
/// **★★★ This SKIPS each test item and RESUMES after it — it does not truncate at the first one, and
/// the difference was measured, not reasoned about.** The first version stopped at the first
/// `#[cfg(test)]`, and a planted `serde_json::Value` appended to the END of
/// `spec/sections.rs` — which carries a `#[cfg(test)] mod tests` in its middle — was reported
/// **green**. That is the same shape `line_coverage_check.rs` records for its own scan (*"a test
/// module appended after it must not grant reachability to everything the tests name"*), and it is
/// the failure this whole file exists to prevent: an instrument that reports success over a region
/// it cannot see.
///
/// **Why the carve-out exists at all.** The coverage KAT's documented exemption — *"`serde_json::
/// Value` walking is permitted HERE ONLY — the §4 veto is on get/set/production paths, not a
/// test"* — must hold without a per-file exclusion list, which is the stale-excuse shape
/// `CLAUDE.md` names as a liability. Stripping comments is what stops a doc comment that merely
/// NAMES the forbidden shape (`sections.rs` says *"no `serde_json::Value`"* in its own header) from
/// being reported as the shape itself.
///
/// ★ Line-based and brace-counted, so a `{` inside a string literal inside a test module could end
/// the skip early. That errs toward scanning MORE, which is the fail-closed direction here.
#[must_use]
pub fn production_source(src: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
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
        if skipping {
            for c in line.chars() {
                match c {
                    '{' => {
                        depth += 1;
                        opened = true;
                    }
                    '}' => depth -= 1,
                    _ => {}
                }
            }
            // Two ways a `#[cfg(test)]` item ends: a DECLARATION (`mod tests;`) at its semicolon,
            // and a BLOCK (`mod tests { … }`) when its braces balance. Both resume the scan — the
            // truncating version reported a plant after a test module as green.
            let declaration_ended = !opened && line.trim_end().ends_with(';');
            let block_ended = opened && depth <= 0;
            if declaration_ended || block_ended {
                skipping = false;
            }
            continue;
        }
        out.push(match line.find("//") {
            Some(i) => &line[..i],
            None => line,
        });
    }
    out.join("\n")
}

/// ★★★ **R15 — no `serde_json::Value` reflection in `btctax-input-form`.**
///
/// The seam is TYPED: a `Field`'s `get`/`set` are `fn` pointers over `ReturnInputs`, and the whole
/// point of the spec's §4 veto is that no second, untyped representation of the form exists to drift
/// from the first. Reflection is how that second representation arrives.
///
/// Pure over `(file label, source)` pairs so a planted defect can reach it.
#[must_use]
pub fn serde_json_reflection(files: &[(String, String)]) -> Vec<String> {
    // ★ The needle is ASSEMBLED rather than written as one literal, and that is not obfuscation:
    //   `tax_profile.rs::m1_preserve_order_value_output_sites_are_enumerated` greps every `src/`
    //   file for the same idiom and demands a fresh audit of each hit. A checker whose own text
    //   trips a sibling checker would have to be excused by name in that one's ALLOWED list — a
    //   stale-excuse entry earned by nothing. Assembling it keeps both instruments honest.
    const JSON_CRATE: &str = "serde_json";
    const REFLECTION_IDIOMS: &[&str] = &["Value", "to_value", "from_value"];
    let mut out = Vec::new();
    for (label, src) in files {
        let prod = production_source(src);
        for (n, line) in prod.lines().enumerate() {
            if REFLECTION_IDIOMS
                .iter()
                .any(|i| line.contains(&format!("{JSON_CRATE}::{i}")))
            {
                out.push(format!("{label}:{}: {}", n + 1, line.trim()));
            }
        }
    }
    out
}

/// ★★★ **R15 — no progress bar and no persisted "what remains".**
///
/// A stored position is the interview answering *"how far through are you?"*, which is a question
/// about the FILER rather than about the return — and it is the field a renderer reaches for first.
/// The check is over field DECLARATIONS, so a local variable or a doc comment is not a finding.
#[must_use]
pub fn progress_shaped_fields(files: &[(String, String)]) -> Vec<String> {
    const BANNED: &[&str] = &["progress", "remaining", "position"];
    let mut out = Vec::new();
    for (label, src) in files {
        for (n, line) in src.lines().enumerate() {
            let t = line.trim();
            let Some(rest) = t.strip_prefix("pub ") else {
                continue;
            };
            let Some((name, _ty)) = rest.split_once(':') else {
                continue;
            };
            let name = name.trim();
            if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                continue; // not a field declaration (a `pub fn f(x: T)`, a `pub use a::b`, …)
            }
            for b in BANNED {
                if name.contains(b) {
                    out.push(format!("{label}:{}: pub {name} ({b})", n + 1));
                }
            }
        }
    }
    out
}

/// ★★★ **R15 — NO PROGRESS BAR, checked where a progress bar can actually be DRAWN (T12).**
///
/// [`progress_shaped_fields`] reads the four persisted-state modules, which is the right walk for
/// *"no persisted 'what remains'"* — and it is the wrong walk for *"no progress bar"*, because a
/// progress bar is not a stored field. It is a **widget**, and it lands in the renderer.
///
/// ★★ **Scoped by MECHANISM rather than by an exemption list.** The obvious extension — pointing the
///    field-name check at `btctax-tui-edit` — reds on `LotPickFormRow::remaining_sat`, the sats left
///    in a ledger lot, which has nothing to do with how far through an interview anyone is. Adding
///    that name to an excuse list is the shape `CLAUDE.md` refuses (*"state the mechanism, let it
///    decide, never enumerate the outcomes you happened to see"*), so the check aimed at the
///    renderer asks the renderer's own question instead: **is a progress widget instantiated, or is
///    a percentage formatted?** ratatui draws a progress bar with `Gauge` / `LineGauge` and nothing
///    else; a hand-rolled one formats a `%`.
///
/// ★ The panel is the one surface in this product whose whole temptation is a bar — it holds four
///   lists of countable items — which is exactly why the check exists on the file that draws it.
#[must_use]
pub fn progress_widgets(files: &[(String, String)]) -> Vec<String> {
    const BANNED: &[&str] = &["LineGauge", "Gauge"];
    let mut out = Vec::new();
    for (label, src) in files {
        for (n, raw) in src.lines().enumerate() {
            let line = match raw.find("//") {
                Some(i) => &raw[..i],
                None => raw,
            };
            // ★ One finding per LINE: `LineGauge` contains `Gauge`, and a check that reports the
            //   same line twice makes its own output harder to read than the defect.
            if let Some(b) = BANNED.iter().find(|b| line.contains(**b)) {
                out.push(format!("{label}:{}: {b}", n + 1));
            }
            // A hand-rolled bar: a percentage FORMATTED into a string. `{}%` / `{:.0}%` / `{pct}%`.
            if let Some(i) = line.find('%') {
                let before = &line[..i];
                if before.ends_with('}') && before.contains('{') && line.contains("format") {
                    out.push(format!("{label}:{}: a formatted percentage", n + 1));
                }
            }
        }
    }
    out
}

/// ★★★ **R15 — no `reconcile` question in a return registry.**
///
/// The ledger's questions (*which transfer is this?*, *which lot?*, *what was the FMV?*) belong to
/// `reconcile`, and the interview must never re-ask them: R9 is explicit that *"the interview never
/// re-asks a ledger question"*. A prompt using those words is the first symptom.
///
/// ★ **Word boundaries, not `contains`.** A naive substring check reds on *allot*, *plot*, *slot*
/// and *transferred*; a check that reds on everything reds on nothing, because the next person
/// deletes it.
#[must_use]
pub fn ledger_words_in_registry_prompts(prompts: &[(String, String)]) -> Vec<String> {
    const BANNED: &[&str] = &["transfer", "lot", "fmv"];
    let mut out = Vec::new();
    for (label, prompt) in prompts {
        let lower = prompt.to_ascii_lowercase();
        for w in lower
            .split(|c: char| !c.is_ascii_alphanumeric())
            .filter(|w| !w.is_empty())
        {
            if BANNED.contains(&w) {
                out.push(format!("{label}: says \"{w}\""));
            }
        }
    }
    out
}

/// ★★ Every module DECLARED `#[cfg(test)]` under `root` — i.e. every file that is test-only in its
/// entirety.
///
/// **Derived from the declarations, never a hand list.** `spec/coverage.rs` is the whole coverage
/// KAT and carries no `#[cfg(test)]` of its own; `spec/mod.rs` gates it with
/// `#[cfg(test)] mod coverage;`. An exclusion list naming `coverage.rs` would be exactly the
/// stale-excuse shape `CLAUDE.md` forbids — and would go silently wrong the day the module stopped
/// being test-only. Reading the declaration cannot.
fn test_only_modules(root: &Path) -> std::collections::BTreeSet<String> {
    let mut out = std::collections::BTreeSet::new();
    for f in rs_files(root) {
        let Ok(src) = std::fs::read_to_string(&f) else {
            continue;
        };
        let lines: Vec<&str> = src.lines().collect();
        for (i, l) in lines.iter().enumerate() {
            if l.trim() != "#[cfg(test)]" {
                continue;
            }
            let Some(next) = lines.get(i + 1) else {
                continue;
            };
            if let Some(rest) = next.trim().strip_prefix("mod ") {
                if let Some(name) = rest.strip_suffix(';') {
                    out.insert(name.trim().to_string());
                }
            }
        }
    }
    out
}

/// The `btctax-input-form` production sources, as `(label, source)` pairs.
fn input_form_sources() -> Vec<(String, String)> {
    let root = repo_root().join("crates/btctax-input-form/src");
    let test_only = test_only_modules(&root);
    rs_files(&root)
        .into_iter()
        .filter(|p| {
            !p.file_stem()
                .is_some_and(|s| test_only.contains(&s.to_string_lossy().to_string()))
        })
        .map(|p| {
            let label = p
                .strip_prefix(repo_root())
                .unwrap_or(&p)
                .to_string_lossy()
                .to_string();
            let src = std::fs::read_to_string(&p).unwrap_or_default();
            (label, src)
        })
        .collect()
}

/// The persisted-input and panel sources a progress field could land on.
fn state_bearing_sources() -> Vec<(String, String)> {
    let root = repo_root().join("crates/btctax-core/src/tax");
    [
        "return_inputs.rs",
        "provenance.rs",
        "document_census.rs",
        "interview_state.rs",
    ]
    .iter()
    .map(|f| {
        (
            format!("crates/btctax-core/src/tax/{f}"),
            std::fs::read_to_string(root.join(f)).unwrap_or_default(),
        )
    })
    .collect()
}

/// ★★★ **T12 — the RENDERER sources, for [`progress_widgets`].** The panel is drawn here; a
/// progress bar, if one were ever written, would be drawn here too.
///
/// ★★ **T12 fold, seam review N-3 — the CLI renderers are in the field of view too.** The list held
///    `draw_edit.rs` alone, and the SAME panel is rendered by `cmd/answer.rs` (`panel_lines`, what
///    `income answer` prints), printed by `render.rs` (§4.4's block) and written to paper by
///    `cmd/admin.rs` (the packet manifest) — so a hand-rolled `format!("{pct}% done")` on any of
///    them was caught by nothing. A checker whose scope no longer matches its subject is this
///    repo's dominant defect shape (harness B1: *"green because it never ran"*), and the fix is one
///    line of source list rather than a note about the limit.
fn renderer_sources() -> Vec<(String, String)> {
    let root = repo_root();
    [
        "crates/btctax-tui-edit/src/draw_edit.rs",
        "crates/btctax-tui-edit/src/edit/form.rs",
        "crates/btctax-tui-edit/src/main.rs",
        "crates/btctax-cli/src/cmd/answer.rs",
        "crates/btctax-cli/src/render.rs",
        "crates/btctax-cli/src/cmd/admin.rs",
    ]
    .iter()
    .map(|f| {
        (
            (*f).to_string(),
            std::fs::read_to_string(root.join(f)).unwrap_or_default(),
        )
    })
    .collect()
}

/// Every prompt in both return registries, labelled by its registry identity — **including the
/// RENDERED ones**.
///
/// ★★★ **T6 extended this, and the extension is the point.** The check scanned only `q.prompt`, the
/// STATIC fallback, while `FormQuestion::prompt_text` renders three questions from the return itself
/// (`RENDERED_PROMPTS`) — so the words a filer is actually SHOWN were never read. That is the exact
/// shape of the F2/F4 failure the harness records: an instrument reporting success over a region it
/// could not see. The rendered set was empty of banned words when this was written, and *"it happened
/// to be clean"* is not the same fact as *"it is checked"*.
///
/// Rendered against a `Default` return, which is enough for a WORD check: the renderers interpolate a
/// year and a filing status, and neither can introduce *transfer*, *lot* or *fmv*. The rendered text
/// is labelled distinctly so a hit names which surface said it.
fn registry_prompts() -> Vec<(String, String)> {
    use btctax_core::tax::questions::{FORM_QUESTIONS, RENDERED_PROMPTS, SKIPPABLE_QUESTIONS};
    let probe = btctax_core::tax::return_inputs::ReturnInputs::default();
    let mut out: Vec<(String, String)> = FORM_QUESTIONS
        .iter()
        .map(|q| (format!("FORM_QUESTIONS {:?}", q.id), q.prompt.to_string()))
        .collect();
    out.extend(SKIPPABLE_QUESTIONS.iter().map(|s| {
        (
            format!("SKIPPABLE_QUESTIONS {:?}", s.id),
            s.prompt.to_string(),
        )
    }));
    out.extend(
        RENDERED_PROMPTS
            .iter()
            .map(|(id, render)| (format!("RENDERED_PROMPTS {id:?}"), render(&probe))),
    );
    out
}

/// ★★★ The four checks, over the committed tree. `Ok` carries the counts it actually scanned, so a
/// walk that found nothing cannot report success quietly.
pub fn run() -> Result<String, String> {
    let form = input_form_sources();
    if form.len() < 5 {
        return Err(format!(
            "the walk found only {} files under btctax-input-form/src — a check that scans nothing \
             passes by finding nothing",
            form.len()
        ));
    }
    let state = state_bearing_sources();
    // ★ T12 — the RENDERER, for the progress-WIDGET half of R15's no-progress-bar sentence.
    let renderers = renderer_sources();
    if renderers.iter().any(|(_, src)| src.len() < 1000) {
        return Err(
            "a renderer source came back empty — a check that scans nothing passes by finding \
             nothing"
                .to_string(),
        );
    }
    let prompts = registry_prompts();
    if prompts.len() < 50 {
        return Err(format!(
            "only {} registry prompts scanned — R3's census rows alone are eighteen",
            prompts.len()
        ));
    }
    // ★★★ T6 — the RENDERED prompts must be IN the scanned set, not merely producible. Without this
    //     the extension is invisible: dropping the `RENDERED_PROMPTS` extend above would leave a
    //     smaller set that still clears the floor, and the checker would go quietly blind again on
    //     the words a filer is actually shown.
    let rendered = btctax_core::tax::questions::RENDERED_PROMPTS.len();
    let scanned_rendered = prompts
        .iter()
        .filter(|(label, _)| label.starts_with("RENDERED_PROMPTS "))
        .count();
    if scanned_rendered != rendered {
        return Err(format!(
            "{scanned_rendered} of {rendered} RENDERED prompts were scanned — the words a filer is \
             SHOWN are the ones this check exists to read, and a static-only scan reports success \
             over a region it cannot see"
        ));
    }
    let mut findings = Vec::new();
    for (rule, hits) in [
        (
            "R15: `serde_json::Value` reflection in btctax-input-form PRODUCTION code — the seam is \
             typed, and a second untyped representation of the form is what R15 forbids",
            serde_json_reflection(&form),
        ),
        (
            "R15: a progress/remaining/position field exists — the interview records WHAT WAS \
             ASKED, never how far through it the filer is",
            progress_shaped_fields(&state),
        ),
        (
            "R15: a PROGRESS WIDGET is drawn — the panel lists ITEMS, and a count of items is not a \
             measure of how far through their return a filer is",
            progress_widgets(&renderers),
        ),
        (
            "R15/R9: a return-registry prompt asks a LEDGER question — those belong to `reconcile`, \
             and the interview never re-asks one",
            ledger_words_in_registry_prompts(&prompts),
        ),
    ] {
        if !hits.is_empty() {
            findings.push(format!("{rule}:\n  {}", hits.join("\n  ")));
        }
    }
    if findings.is_empty() {
        Ok(format!(
            "R15 stop list: {} btctax-input-form sources, {} state-bearing sources, {} renderer \
             source(s) and {} registry prompts scanned; no forbidden shape",
            form.len(),
            state.len(),
            renderers.len(),
            prompts.len()
        ))
    } else {
        Err(findings.join("\n\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The four checks, run against the real tree. Each must be silent.
    #[test]
    fn the_r15_stop_list_holds_on_the_committed_tree() {
        match run() {
            Ok(s) => println!("{s}"),
            Err(e) => panic!("{e}"),
        }
    }

    /// ★★★ **B1 — each of the four greps watched going RED on the exact defect it exists to
    ///     catch, and staying green on the near-miss beside it.**
    ///
    /// The near-misses are the whole test: a checker that reds on everything is deleted by the next
    /// person who trips it, and a checker that reds on nothing was never watched discriminating.
    /// ★★★ **T12 fold, seam review N-3 — THE RENDERER SOURCE LIST IS DERIVED FROM WHO ACTUALLY
    ///     RENDERS THE PANEL, not from what someone remembered to type.**
    ///
    /// The list held `draw_edit.rs` alone while five other files render, print or write the same
    /// panel — a checker green over a region it could not see, which is this repo's dominant
    /// instrument failure (harness B1). Extending the list would have fixed today; this reds the day
    /// a SIXTH surface starts rendering it, which is the only version of the fix worth having.
    #[test]
    fn every_file_that_renders_the_panel_is_in_the_progress_widget_checks_field_of_view() {
        const RENDERS: &[&str] = &[
            "panel_lines(",
            "forgoing_lines(",
            "refusing_lines(",
            "not_computed_lines(",
        ];
        let scanned: std::collections::BTreeSet<String> =
            renderer_sources().into_iter().map(|(f, _)| f).collect();
        let mut missing: Vec<String> = Vec::new();
        for crate_src in ["crates/btctax-cli/src", "crates/btctax-tui-edit/src"] {
            for path in rs_files(&repo_root().join(crate_src)) {
                let rel = path
                    .strip_prefix(repo_root())
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                let src = std::fs::read_to_string(&path).unwrap_or_default();
                if RENDERS.iter().any(|r| src.contains(r)) && !scanned.contains(&rel) {
                    missing.push(rel);
                }
            }
        }
        assert!(
            missing.is_empty(),
            "these files render the answer panel and the progress-widget check cannot see them, so \
             a hand-rolled bar there is caught by nothing: {missing:?}"
        );
        // A broken walk must be LOUD: the set it scans is not empty and not everything.
        assert!(
            scanned.len() >= 6,
            "the renderer source list has stopped being populated: {scanned:?}"
        );
    }

    /// ★★★ **B1 — the progress-WIDGET check, watched red on a planted defect and green on its near
    ///     misses.** T12 added it because the field-name check reads the persisted-state modules,
    ///     where a progress bar was never going to be written; this reads the file that draws the
    ///     answer panel, which is the one surface in the product whose whole temptation is a bar.
    #[test]
    fn the_progress_widget_check_reds_on_a_gauge_and_not_on_its_near_misses() {
        let at =
            |src: &str| progress_widgets(&[("draw_edit.rs".to_string(), src.to_string())]).len();
        // ── The plants ──
        assert_eq!(
            at("    let g = Gauge::default().ratio(0.5);"),
            1,
            "ratatui's progress widget must be a finding"
        );
        assert_eq!(
            at("    frame.render_widget(LineGauge::default(), rect);"),
            1,
            "…and so must its thin sibling"
        );
        assert_eq!(
            at(r#"    lines.push(format!("  {done}% complete"));"#),
            1,
            "a HAND-ROLLED bar is a formatted percentage, and it must be a finding too"
        );
        // ── The near misses, which must stay green or the check reds on everything and gets
        //    deleted by the next person ──
        assert_eq!(
            at("    // a Gauge would be a progress bar, and R15 forbids one"),
            0,
            "a mention in a comment is not a widget"
        );
        assert_eq!(
            at(r#"    let pct = "7.5% of AGI";"#),
            0,
            "a literal percentage in prose — the §213 medical floor — is not a bar"
        );
        assert_eq!(
            at(r#"    let s = format!("{a} of {b} lines");"#),
            0,
            "the panel's own scroll footer counts LINES, not progress"
        );
    }

    #[test]
    fn each_r15_grep_reds_on_a_planted_line_and_not_on_its_near_miss() {
        // ── (1) serde_json reflection. ──────────────────────────────────────────────────────────
        let planted = vec![(
            "spec/sections.rs".to_string(),
            "pub fn walk(v: &serde_json::Value) {}".to_string(),
        )];
        assert_eq!(
            serde_json_reflection(&planted).len(),
            1,
            "a `serde_json::Value` in production code must be a finding"
        );
        // …a mention in a COMMENT is not (the real `sections.rs` header says exactly this).
        assert!(serde_json_reflection(&[(
            "spec/sections.rs".to_string(),
            "// no `serde_json::Value` here — the seam is typed".to_string(),
        )])
        .is_empty());
        // ★★★ THE BLINDNESS THAT WAS MEASURED, not imagined: a plant AFTER a `#[cfg(test)] mod
        //     tests { … }` block. The first version of `production_source` truncated at the first
        //     `#[cfg(test)]` and reported this GREEN — an instrument reporting success over a
        //     region it could not see, which is the exact class this file exists to prevent.
        let after_a_test_module = "pub fn a() {}\n             #[cfg(test)]\n             mod tests {\n    fn t() { let _ = 1; }\n}\n             pub type Planted = serde_json::Value;\n";
        assert_eq!(
            serde_json_reflection(&[(
                "spec/sections.rs".to_string(),
                after_a_test_module.to_string()
            )])
            .len(),
            1,
            "production code AFTER a test module must still be scanned — truncating at the first \
             `#[cfg(test)]` made this green while the plant sat in the shipped seam"
        );

        // …and neither is one below `#[cfg(test)]` (the coverage KAT's documented carve-out).
        assert!(serde_json_reflection(&[(
            "spec/coverage.rs".to_string(),
            "pub fn f() {}\n#[cfg(test)]\nmod t { use serde_json::Value; }".to_string(),
        )])
        .is_empty());
        // ★ …and the WHOLE-FILE carve-out is read off the declaration, not off a name list.
        let td = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            td.path().join("mod.rs"),
            "#[cfg(test)]\nmod coverage;\nmod sections;\n",
        )
        .unwrap();
        std::fs::write(td.path().join("coverage.rs"), "use serde_json::Value;\n").unwrap();
        std::fs::write(td.path().join("sections.rs"), "fn f() {}\n").unwrap();
        assert_eq!(
            test_only_modules(td.path()),
            ["coverage".to_string()].into_iter().collect(),
            "a `#[cfg(test)] mod X;` declaration is what makes X.rs test-only"
        );

        // ── (2) progress-shaped fields. ─────────────────────────────────────────────────────────
        assert_eq!(
            progress_shaped_fields(&[(
                "return_inputs.rs".to_string(),
                "    pub interview_progress: usize,".to_string(),
            )])
            .len(),
            1,
            "a persisted progress field must be a finding"
        );
        assert_eq!(
            progress_shaped_fields(&[(
                "provenance.rs".to_string(),
                "    pub questions_remaining: usize,".to_string(),
            )])
            .len(),
            1
        );
        // …a doc comment that merely NAMES the shape is not a finding — the real sources say
        // "Forbidden here: progress, position, …" and must stay green.
        assert!(progress_shaped_fields(&[(
            "return_inputs.rs".to_string(),
            "    /// ★★ Forbidden here: progress, position, \"what remains\".".to_string(),
        )])
        .is_empty());
        // …and neither is a function whose PARAMETER is so named: this rule is about stored state.
        assert!(progress_shaped_fields(&[(
            "x.rs".to_string(),
            "    pub fn render(progress: usize) {}".to_string(),
        )])
        .is_empty());

        // ── (3) ledger words in a registry prompt. ──────────────────────────────────────────────
        assert_eq!(
            ledger_words_in_registry_prompts(&[(
                "FORM_QUESTIONS DocW2".to_string(),
                "Which lot did you sell?".to_string(),
            )])
            .len(),
            1,
            "a registry prompt asking a LEDGER question must be a finding"
        );
        assert_eq!(
            ledger_words_in_registry_prompts(&[(
                "q".to_string(),
                "Was this a transfer between your own wallets?".to_string(),
            )])
            .len(),
            1
        );
        assert_eq!(
            ledger_words_in_registry_prompts(&[(
                "q".to_string(),
                "What was the FMV at receipt?".to_string(),
            )])
            .len(),
            1
        );
        // ★ The near-misses. A `contains` check would red on all three of these, which is how a
        //   grep like this gets deleted rather than fixed.
        assert!(ledger_words_in_registry_prompts(&[(
            "q".to_string(),
            "Did the plot of land allot you a slot in the transferable pool?".to_string(),
        )])
        .is_empty());
    }
}
