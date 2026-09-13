//! xtask — developer tooling for the btctax workspace.
//!
//! `cargo run -p xtask -- docs` regenerates the committed man pages under `docs/man/`.
//! `cargo run -p xtask -- docs --pdf` additionally renders `docs/pdf/*.pdf` (requires `groff`).
//! `cargo run -p xtask -- check-isolation` asserts no HTTP client in the tax crates (#41 Part C).
//! `cargo run -p xtask -- dump-fields <pdf>` lists a PDF's AcroForm field names (map authoring).
//! `cargo run -p xtask -- cite-check` verifies every quotation in the Schedule 1-A design docs against
//! the committed IRS text-layer extracts. `… -- extract-schedule-1a` regenerates those extracts.

mod archive_check;
mod authority_conflicts;
mod authority_manifest;
mod authority_refresh;
mod box_census;
/// N1 — asserts `btctax_core::tax::capital_loss_carryover` is verbatim and complete against the 2025
/// Schedule D instructions' text layer.
///
/// `#[cfg(test)]` because it has no operator-facing mode: unlike `line_coverage_check`, whose `run()`
/// prints a coverage REPORT worth reading, this one answers a yes/no question and the answer belongs
/// in the suite, where `make check` asks it on every commit rather than when someone remembers to.
#[cfg(test)]
mod capital_loss_carryover_check;
mod census_join;
mod check_isolation;
mod cite_check;
mod dependents_grid;
mod docs;
mod dump_fields;
mod examples;
/// The Schedule 1-A **provenance forge** — the deliberate route to a `SeniorDeductionSubtotal` whose
/// line number no revision printed — reaches no shipped seam, and is still exercised by a kill.
///
/// `#[cfg(test)]` because it has no operator-facing mode, like `capital_loss_carryover_check`: it
/// answers a yes/no question, and the answer belongs in the suite where `make check` asks it on every
/// commit rather than when someone remembers to.
#[cfg(test)]
mod forge_reach_check;
mod form_delta;
mod form_geometry;
mod forms_extract;
mod forms_fetch;
mod harness_check;
mod label_reader;
mod line_coverage_check;
mod package_check;
mod prompt_check;
mod r15_stop_list;
/// Half 1a of the Schedule 1-A conformance KAT — the 48 entry labels, adjudicated by `label_reader`'s
/// two witnesses over the committed geometry and compared to `Schedule1A`'s own leaves.
///
/// `#[cfg(test)]` because it has no operator-facing mode, exactly like `capital_loss_carryover_check`:
/// it answers a yes/no question, and the answer belongs in the suite.
#[cfg(test)]
mod schedule_1a_membership;
/// ★★★ Phase 4 / decision D-H — **no IRS service-center postal address in shipped text.** The plan
/// decided the mailing-address help is a LINK plus the two facts, never a bundled table: a stale
/// address fails with no error message, and the IRS itself warns it is reducing the number of paper
/// processing sites.
///
/// `#[cfg(test)]` because it has no operator-facing mode, like `forge_reach_check`: it answers a
/// yes/no question and the answer belongs in the suite, where `make check` asks it on every commit.
#[cfg(test)]
mod service_center_check;
mod verdict_reach;
/// FR-108 — a filer-facing sentence with the wrap indentation still inside its quotes.
mod wrapped_literal_check;

/// ★★★ **FR-143 — every subcommand, the flags it accepts, and its positional spelling, in ONE place.**
///
/// **The defect this exists to stop, measured.** `design/TY2026_PORT_REPORT.md`'s runbook step 4 prints
/// `xtask authority-manifest --regenerate`. The code accepts only `--regen`, and each arm tested its own
/// flags with `args.iter().any(|a| a == "--regen")` — so an unrecognised flag was simply *not seen*, and
/// `--regenerate` fell through to the read-only checker. In the port rehearsal's isolated worktree that
/// printed **`authority-manifest: OK — every entry resolves and every source is listed` and exited 0**:
/// an operator following the documented command got a success message and no regeneration at all.
/// A wrong flag must never be indistinguishable from the right one.
///
/// ★ Column 3 is not decoration: [`tests::the_flag_list_and_the_argument_spelling_agree`] asserts every
/// flag in column 2 appears in column 3 and every `--token` in column 3 appears in column 2, so the
/// usage text cannot drift away from what the parser accepts — the exact drift F7 was.
///
/// ★ And the table is not a hand-list beside a growing set:
/// [`tests::every_dispatched_subcommand_declares_its_flags`] reads this file's own `Some("…")` dispatch
/// arms and reds if one is missing a row. Adding an arm without declaring its flags fails the suite.
const SUBCOMMANDS: &[(&str, &[&str], &str)] = &[
    ("archive-check", &[], ""),
    ("authority-conflicts", &[], ""),
    ("authority-manifest", &["--regen"], "[--regen]"),
    (
        "authority-refresh",
        &["--check", "--from-dir"],
        "--check [--from-dir <dir>]",
    ),
    ("box-census", &[], ""),
    ("census-join", &[], ""),
    ("check-isolation", &[], ""),
    ("cite-check", &[], ""),
    ("classify-path", &[], "<path>"),
    ("dependents-grid", &[], "<stem>"),
    ("docs", &["--pdf"], "[--pdf]"),
    ("dump-fields", &[], "<pdf>"),
    ("examples", &[], ""),
    ("extract-geometry", &[], "<stem>"),
    ("extract-schedule-1a", &[], ""),
    ("form-delta", &[], "<old-stem> <new-stem>"),
    // ★ FR-139 / FR-140 — the port machine's namespace (`design/TY2026_PORT_REPORT.md` §4).
    ("forms", &[], "<fetch|extract> …"),
    (
        "forms extract",
        &["--all", "--check", "--adopt"],
        "<stem> | --all [--check] | --adopt <stem>",
    ),
    (
        "forms fetch",
        &["--restore", "--from-dir", "--stem"],
        "--restore [--from-dir <dir>] [--stem <stem>]",
    ),
    ("harness-check", &[], ""),
    ("label-boxes", &[], "<stem>"),
    ("label-census", &[], "<stem>"),
    ("label-proof", &[], "<stem> [out.pdf]"),
    ("line-coverage", &[], ""),
    ("port-status", &[], "<prior-tag> <new-tag>"),
    ("prompt-check", &[], ""),
    ("stop-list", &[], ""),
    ("subcommand-coverage", &[], ""),
    ("verdict-reach", &[], ""),
    ("wrapped-literals", &[], ""),
];

/// The table key for what the operator actually typed. `forms` is a namespace, so its key is two words
/// — otherwise `forms fetch --all` would be accepted because `forms extract` allows `--all`.
fn flag_key(args: &[String]) -> String {
    let Some(first) = args.first() else {
        return String::new();
    };
    if first == "forms" {
        if let Some(second) = args.get(1) {
            if !second.starts_with('-') {
                return format!("forms {second}");
            }
        }
    }
    first.clone()
}

/// ★★★ **Refuse an argument the subcommand does not accept, instead of not noticing it.**
///
/// Fails closed twice over: a subcommand with no table row accepts **no** flags, and any `-`-leading
/// argument not listed is refused by name with the accepted set printed beside it.
fn reject_unknown_flags(args: &[String]) -> Result<(), String> {
    let key = flag_key(args);
    let row = SUBCOMMANDS.iter().find(|(name, _, _)| *name == key);
    let allowed: &[&str] = row.map(|(_, f, _)| *f).unwrap_or(&[]);
    // The words that belong to the key itself are not arguments to it.
    let skip = key.split_whitespace().count();
    for arg in args.iter().skip(skip) {
        if arg.starts_with('-') && !allowed.contains(&arg.as_str()) {
            let accepts = if allowed.is_empty() {
                "it accepts no flags".to_string()
            } else {
                format!("it accepts only {}", allowed.join(" "))
            };
            return Err(format!(
                "xtask {key}: unrecognised argument {arg:?} — {accepts}.\n\
                 ★ REFUSING rather than ignoring it: an unread flag makes a typo look like a \
                 success. `authority-manifest --regenerate` used to print OK and regenerate \
                 nothing (FR-143).\n\
                 usage: cargo run -p xtask -- {key} {}",
                row.map(|(_, _, spec)| *spec).unwrap_or_default()
            ));
        }
    }
    Ok(())
}

/// ★★★ **FR-148 — where `label-proof` writes when the operator names no path.**
///
/// It was a hardcoded `/tmp/<stem>-label-proof.pdf`. On this development box `/tmp` is a **32 GB tmpfs
/// shared with every build**, and a scratch `target/` filling it has already killed a running test once
/// — so the constellation directive says to check what `/tmp` is before writing there. A proof PDF is
/// only ~240 KB, but which filesystem takes it is the operator's call, not this tool's.
///
/// `std::env::temp_dir()` reads `TMPDIR` and falls back to `/tmp`, so nobody who has not set it sees
/// any change. A function rather than an inline expression **so the kill can call the real thing**
/// instead of grepping the source for a literal.
fn default_proof_path(stem: &str) -> String {
    std::env::temp_dir()
        .join(format!("{stem}-label-proof.pdf"))
        .to_string_lossy()
        .into_owned()
}

/// The usage text, GENERATED from [`SUBCOMMANDS`] so it cannot omit a subcommand.
///
/// ★ The hand-written string this replaced was missing `verdict-reach` and `wrapped-literals`.
fn usage() -> String {
    let mut s = String::from(
        "usage: cargo run -p xtask -- <subcommand> [args]\n\nsubcommands (and the ONLY flags each accepts):\n",
    );
    for (name, _, spec) in SUBCOMMANDS {
        if spec.is_empty() {
            s.push_str(&format!("  {name}\n"));
        } else {
            s.push_str(&format!("  {name} {spec}\n"));
        }
    }
    s
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Err(e) = reject_unknown_flags(&args) {
        eprintln!("{e}");
        std::process::exit(2);
    }
    match args.first().map(String::as_str) {
        Some("docs") => {
            if let Err(e) = docs::write_man_pages() {
                eprintln!("xtask docs: {e}");
                std::process::exit(1);
            }
            if args.iter().any(|a| a == "--pdf") {
                if let Err(e) = docs::write_pdfs() {
                    eprintln!("xtask docs --pdf: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some("examples") => {
            examples::run();
        }
        Some("subcommand-coverage") => {
            examples::run_coverage();
        }
        Some("authority-conflicts") => {
            if let Err(e) = authority_conflicts::run() {
                eprintln!("xtask authority-conflicts: {e}");
                std::process::exit(1);
            }
        }
        // ★★★ FR-29 / SPEC §9 G7 — the Form 8615 prompts and help strings against the extracts they
        //     are transcribed from, both directions. See `prompt_check`'s module doc.
        Some("prompt-check") => {
            if let Err(e) = prompt_check::run() {
                eprintln!("xtask prompt-check: a prompt no longer matches the form:\n{e}");
                std::process::exit(1);
            }
        }
        Some("cite-check") => {
            if let Err(e) = cite_check::run() {
                eprintln!("xtask cite-check: {e}");
                std::process::exit(1);
            }
        }
        Some("extract-schedule-1a") => {
            if let Err(e) = cite_check::extract() {
                eprintln!("xtask extract-schedule-1a: {e}");
                std::process::exit(1);
            }
        }
        Some("archive-check") => {
            if let Err(e) = archive_check::run() {
                eprintln!("xtask archive-check: {e}");
                std::process::exit(1);
            }
        }
        Some("verdict-reach") => {
            if let Err(e) = verdict_reach::run() {
                eprintln!("xtask verdict-reach: {e}");
                std::process::exit(1);
            }
        }
        Some("classify-path") => {
            let Some(path) = args.get(1) else {
                eprintln!("usage: cargo run -p xtask -- classify-path <path>");
                std::process::exit(2);
            };
            if let Err(e) = archive_check::classify_path(path) {
                eprintln!("xtask classify-path: {e}");
                std::process::exit(1);
            }
        }
        Some("authority-manifest") => {
            if args.iter().any(|a| a == "--regen") {
                match authority_manifest::regen(
                    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                        .parent()
                        .and_then(std::path::Path::parent)
                        .expect("workspace root"),
                ) {
                    Ok(n) => println!("authority-manifest: regenerated {n} entries"),
                    Err(e) => {
                        eprintln!("xtask authority-manifest --regen: {e}");
                        std::process::exit(1);
                    }
                }
            } else if let Err(e) = authority_manifest::run() {
                eprintln!("xtask authority-manifest: {e}");
                std::process::exit(1);
            }
        }
        Some("form-delta") => {
            let (Some(a), Some(b)) = (args.get(1), args.get(2)) else {
                eprintln!(
                    "usage: cargo run -p xtask -- form-delta <old-stem> <new-stem>\n\
                     e.g. form-delta f6251--2025 f6251--2026-DRAFT"
                );
                std::process::exit(2);
            };
            if let Err(e) = form_delta::run(a, b) {
                eprintln!("xtask form-delta: {e}");
                std::process::exit(1);
            }
        }
        Some("port-status") => {
            let (Some(a), Some(b)) = (args.get(1), args.get(2)) else {
                eprintln!(
                    "usage: cargo run -p xtask -- port-status <prior-tag> <new-tag>\n\
                     e.g. port-status 2025 2026-DRAFT   (prints the work list's two tables from the emitting surface)"
                );
                std::process::exit(2);
            };
            match form_delta::port_status(a, b) {
                Ok(s) => print!("{s}"),
                Err(e) => {
                    eprintln!("xtask port-status: {e}");
                    std::process::exit(1);
                }
            }
        }
        // ★ ON DEMAND ONLY — it needs the network, and `make check` must stay offline. See the
        //   module doc: the suite tests the pure comparison, never the fetch.
        Some("authority-refresh") => {
            if !args.iter().any(|a| a == "--check") {
                eprintln!(
                    "usage: cargo run -p xtask -- authority-refresh --check [--from-dir <dir>]"
                );
                std::process::exit(2);
            }
            let from_dir = args
                .iter()
                .position(|a| a == "--from-dir")
                .and_then(|i| args.get(i + 1))
                .map(std::path::PathBuf::from);
            if let Err(e) = authority_refresh::run(from_dir) {
                eprintln!("xtask authority-refresh: {e}");
                std::process::exit(1);
            }
        }
        Some("extract-geometry") => {
            let Some(stem) = args.get(1) else {
                eprintln!(
                    "usage: cargo run -p xtask -- extract-geometry <stem>   e.g. f1040s1a--2025"
                );
                std::process::exit(2);
            };
            if let Err(e) = form_geometry::extract(stem) {
                eprintln!("xtask extract-geometry: {e}");
                std::process::exit(1);
            }
        }
        Some("label-census") => {
            let Some(stem) = args.get(1) else {
                eprintln!("usage: cargo run -p xtask -- label-census <stem>   e.g. f1040s1a--2025");
                std::process::exit(2);
            };
            if let Err(e) = label_reader::run(stem) {
                eprintln!("xtask label-census: {e}");
                std::process::exit(1);
            }
        }
        Some("dependents-grid") => {
            // ★★★ T8 / R6: MEASURE the TY2025+ Dependents grid off the form and print the map
            //     section, so `forms/<year>/f1040.map.toml` is generated rather than typed.
            let Some(stem) = args.get(1) else {
                eprintln!("usage: cargo run -p xtask -- dependents-grid <stem>   e.g. f1040--2025");
                std::process::exit(2);
            };
            if let Err(e) = dependents_grid::run(stem) {
                eprintln!("xtask dependents-grid: {e}");
                std::process::exit(1);
            }
        }
        Some("label-boxes") => {
            let Some(stem) = args.get(1) else {
                eprintln!("usage: cargo run -p xtask -- label-boxes <stem>");
                std::process::exit(2);
            };
            if let Err(e) = label_reader::boxes_tsv(stem) {
                eprintln!("xtask label-boxes: {e}");
                std::process::exit(1);
            }
        }
        Some("label-proof") => {
            let Some(stem) = args.get(1) else {
                eprintln!("usage: cargo run -p xtask -- label-proof <stem> [out.pdf]");
                std::process::exit(2);
            };
            let out = args
                .get(2)
                .cloned()
                .unwrap_or_else(|| default_proof_path(stem));
            if let Err(e) = label_reader::proof(stem, &out) {
                eprintln!("xtask label-proof: {e}");
                std::process::exit(1);
            }
        }
        Some("harness-check") => {
            if let Err(e) = harness_check::run() {
                eprintln!("xtask harness-check: {e}");
                std::process::exit(1);
            }
        }
        Some("line-coverage") => {
            // ★ §G-11: every printed money field declares which IRS instruction production it
            //   transcribes, checked verbatim against design/forms/extract/.
            match line_coverage_check::run() {
                Ok(msg) => println!("{msg}"),
                Err(e) => {
                    eprintln!("xtask line-coverage: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some("census-join") => {
            // ★★★ SPEC_interview R2.2 / T3: every `unmodeled` census entry on the seven forms the
            //   interview reaches is ANNOUNCED or REFUSED, never silent — with the DIRECTION rule
            //   (no Advisory may cover a line whose omission understates tax) and the REACH check
            //   (a covering question's prompt must NAME the line).
            match census_join::run() {
                Ok(msg) => println!("{msg}"),
                Err(e) => {
                    eprintln!("xtask census-join: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some("stop-list") => {
            // ★★★ SPEC_interview R15: the stop list as executable requirements — no `serde_json`
            //   reflection in the typed seam, no progress/remaining field, no ledger question in a
            //   return registry.
            match r15_stop_list::run() {
                Ok(msg) => println!("{msg}"),
                Err(e) => {
                    eprintln!("xtask stop-list: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some("wrapped-literals") => {
            // ★★★ FR-108: a string literal left on one source line with the next line's
            //   indentation typed in as real spaces prints a gap in the middle of a sentence.
            //   See `wrapped_literal_check`'s module doc for what it covers and what it does not.
            match wrapped_literal_check::run() {
                Ok(msg) => println!("{msg}"),
                Err(e) => {
                    eprintln!("xtask wrapped-literals: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some("box-census") => {
            // ★ SPEC_interview R4/I2: every box an archived information return PRINTS carries exactly
            //   one recorded decision, the caption checked verbatim against design/forms/extract/.
            if let Err(e) = box_census::run() {
                eprintln!("xtask box-census: {e}");
                std::process::exit(1);
            }
        }
        Some("check-isolation") => {
            if let Err(e) = check_isolation::run() {
                eprintln!("xtask check-isolation: {e}");
                std::process::exit(1);
            }
        }
        Some("dump-fields") => {
            let Some(path) = args.get(1) else {
                eprintln!("usage: cargo run -p xtask -- dump-fields <pdf>");
                std::process::exit(2);
            };
            if let Err(e) = dump_fields::run(path) {
                eprintln!("xtask dump-fields: {e}");
                std::process::exit(1);
            }
        }
        // ★★★ The PORT MACHINE's namespace — `design/TY2026_PORT_REPORT.md` §4. It is `forms` and not
        //     two more top-level verbs because **58 committed extracts already print
        //     `cargo run -p xtask -- forms extract` as their regeneration command** (FR-140).
        Some("forms") => match args.get(1).map(String::as_str) {
            Some("fetch") => {
                if let Err(e) = forms_fetch::run(&args[2..]) {
                    eprintln!("xtask forms fetch: {e}");
                    std::process::exit(1);
                }
            }
            Some("extract") => {
                if let Err(e) = forms_extract::run(&args[2..]) {
                    eprintln!("xtask forms extract: {e}");
                    std::process::exit(1);
                }
            }
            other => {
                eprintln!(
                    "xtask forms: unknown sub-subcommand {other:?}\n\
                     usage: cargo run -p xtask -- forms fetch --restore [--from-dir <dir>] [--stem <stem>]\n\
                     \x20      cargo run -p xtask -- forms extract <stem> | --all [--check] | --adopt <stem>"
                );
                std::process::exit(2);
            }
        },
        _ => {
            eprintln!("{}", usage());
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ★★★ **B1 for FR-143 — the documented-but-wrong flag is REFUSED, and the real one accepted.**
    ///
    /// The plant is the exact string the runbook printed. Before this, `--regenerate` matched no arm's
    /// `any(|a| a == "--regen")`, fell through to the read-only checker, and the rehearsal measured it
    /// printing `OK` and exiting **0** in a worktree where nothing had been regenerated.
    #[test]
    fn an_unknown_flag_is_refused_and_the_documented_one_is_accepted() {
        let typo: Vec<String> = ["authority-manifest", "--regenerate"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let err = reject_unknown_flags(&typo).expect_err("--regenerate must be REFUSED");
        assert!(err.contains("--regenerate"), "{err}");
        assert!(
            err.contains("it accepts only --regen"),
            "the refusal must name what IS accepted: {err}"
        );

        let right: Vec<String> = ["authority-manifest", "--regen"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(
            reject_unknown_flags(&right).is_ok(),
            "--regen must be taken"
        );

        // ★ A subcommand with no flags at all accepts none — fail closed, not open.
        let flagless: Vec<String> = ["cite-check", "--fix"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let err = reject_unknown_flags(&flagless).expect_err("cite-check takes no flags");
        assert!(err.contains("it accepts no flags"), "{err}");

        // ★ And a namespace does not lend its flags to its sibling: `forms extract` allows `--all`,
        //   so a key of one word would have let `forms fetch --all` through.
        let crossed: Vec<String> = ["forms", "fetch", "--all"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let err = reject_unknown_flags(&crossed).expect_err("--all is not a `forms fetch` flag");
        assert!(err.contains("forms fetch"), "{err}");
        let ok: Vec<String> = ["forms", "extract", "--all"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(
            reject_unknown_flags(&ok).is_ok(),
            "`forms extract --all` is real"
        );

        // ★ Positional arguments are untouched — this guard is about flags only.
        let positional: Vec<String> = ["label-proof", "f8995a--2025", "out.pdf"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(reject_unknown_flags(&positional).is_ok());
    }

    /// ★★★ **The table is DERIVED-CHECKED against the dispatcher, not trusted.**
    ///
    /// Reads this file's own source for `Some("…") =>` dispatch arms and requires a [`SUBCOMMANDS`] row
    /// for each. A new subcommand with no row therefore cannot land silently accepting any flag anyone
    /// types at it. The nested `forms` arms are keyed two-word, exactly as [`flag_key`] keys them.
    #[test]
    fn every_dispatched_subcommand_declares_its_flags() {
        // ★ Only `fn main`'s dispatcher: the scan stops at `#[cfg(test)]`, because a `Some("…")` in a
        //   test assertion is not a subcommand. It read one as one until this line existed.
        let src = include_str!("main.rs");
        let dispatcher = &src[..src
            .find("#[cfg(test)]\nmod tests")
            .expect("the test module")];
        let mut dispatched: Vec<String> = Vec::new();
        let mut in_forms = false;
        for line in dispatcher.lines() {
            let t = line.trim_start();
            if t.starts_with("Some(\"forms\") =>") {
                in_forms = true;
            }
            // A dispatch arm, not any string literal: `Some("name") =>`.
            let Some(rest) = t.strip_prefix("Some(\"") else {
                continue;
            };
            let Some((name, tail)) = rest.split_once('"') else {
                continue;
            };
            if !tail.trim_start().starts_with(") =>") {
                continue;
            }
            // The nested arms are indented deeper than the outer `match`'s arms.
            let nested = line.len() - t.len() > 8;
            if in_forms && nested && name != "forms" {
                dispatched.push(format!("forms {name}"));
            } else {
                dispatched.push(name.to_string());
            }
        }
        dispatched.sort();
        dispatched.dedup();
        assert!(
            dispatched.len() >= 28,
            "the source scan found only {} arms — it has stopped reading the dispatcher: {dispatched:?}",
            dispatched.len()
        );
        let declared: Vec<&str> = SUBCOMMANDS.iter().map(|(n, _, _)| *n).collect();
        let undeclared: Vec<&String> = dispatched
            .iter()
            .filter(|d| !declared.contains(&d.as_str()))
            .collect();
        assert!(
            undeclared.is_empty(),
            "dispatched with no SUBCOMMANDS row, so ANY flag typed at it would be accepted: \
             {undeclared:?}"
        );
        let undispatched: Vec<&&str> = declared
            .iter()
            .filter(|d| !dispatched.contains(&d.to_string()))
            .collect();
        assert!(
            undispatched.is_empty(),
            "declared in SUBCOMMANDS but dispatched nowhere — the usage text would advertise a \
             subcommand that does not exist: {undispatched:?}"
        );
    }

    /// ★ Column 2 (what the parser accepts) and column 3 (what the usage text promises) must agree —
    /// the *exact* drift F7 found between the runbook's `--regenerate` and the code's `--regen`.
    #[test]
    fn the_flag_list_and_the_argument_spelling_agree() {
        for (name, flags, spec) in SUBCOMMANDS {
            for f in *flags {
                assert!(
                    spec.contains(f),
                    "{name} accepts {f} and the usage text does not mention it: {spec:?}"
                );
            }
            for tok in spec.split_whitespace() {
                let tok = tok.trim_start_matches('[').trim_end_matches(']');
                if tok.starts_with("--") {
                    assert!(
                        flags.contains(&tok),
                        "{name}'s usage text advertises {tok} which the parser does not accept"
                    );
                }
            }
        }
        // ★ The generated usage text names every subcommand. The hand-written string this replaced
        //   was missing `verdict-reach` and `wrapped-literals`.
        let u = usage();
        for (name, _, _) in SUBCOMMANDS {
            assert!(u.contains(name), "usage omits {name}");
        }
    }

    /// ★★★ **B1 for FR-148 — the default proof path follows `TMPDIR`.**
    ///
    /// `label-proof` wrote to a hardcoded `/tmp/<stem>-label-proof.pdf`. Measured before the fix: with
    /// `TMPDIR` pointed elsewhere the 239,238-byte PDF still landed in `/tmp`, a 32 GB tmpfs shared
    /// with every build on this box. This pins the *mechanism* — `std::env::temp_dir()` — so a future
    /// edit cannot quietly go back to a literal.
    #[test]
    fn the_label_proof_default_path_honours_tmpdir() {
        // SAFETY: single-threaded within this test, and the variable is restored before returning.
        let before = std::env::var_os("TMPDIR");
        let dir = tempfile::tempdir().expect("tempdir");
        unsafe { std::env::set_var("TMPDIR", dir.path()) };
        // ★ The REAL function the `label-proof` arm calls — not a re-implementation of it, and not a
        //   grep of the source, either of which can pass while the arm keeps its literal.
        let got = std::path::PathBuf::from(default_proof_path("f8995a--2025"));
        assert!(
            got.starts_with(dir.path()),
            "the default proof path ignored TMPDIR: {}",
            got.display()
        );
        assert_eq!(
            got.file_name().and_then(|f| f.to_str()),
            Some("f8995a--2025-label-proof.pdf"),
            "the filename convention must not change: {}",
            got.display()
        );

        // ★★ And with TMPDIR unset the behaviour is exactly what it always was.
        unsafe { std::env::remove_var("TMPDIR") };
        assert_eq!(
            default_proof_path("f8995a--2025"),
            "/tmp/f8995a--2025-label-proof.pdf",
            "an operator who has not set TMPDIR must see no change"
        );

        unsafe {
            match before {
                Some(v) => std::env::set_var("TMPDIR", v),
                None => std::env::remove_var("TMPDIR"),
            }
        }
    }
}
