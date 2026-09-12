//! ★★★ **THE PROVENANCE FORGE MAY NOT REACH PRODUCTION.**
//!
//! `btctax_core::tax::schedule_1a::SeniorDeductionSubtotal` carries the Schedule 1-A line its figure
//! was read off, and its fields are private so that a figure cannot be paired with a line number no
//! revision printed. The two revisions of that schedule print a ≤$6,000 deduction and a six-figure
//! modified AGI under colliding line numbers (37 ⇄ 43), so the wrong pairing overstates the AMT base
//! by ≈MAGI — taxpayer-adverse, with the field map, the geometry, the read-back and both oracles all
//! green, because every one of them takes the figure as an input.
//!
//! ★★ **And yet a forge has to exist.** The kill test for the emitter's join
//! (`btctax-forms/tests/f6251_obbba.rs::the_fill_refuses_a_figure_read_off_a_schedule_1a_line_this_revision_does_not_cite`)
//! must construct exactly that mismatch — it is the defect the join exists to refuse. Sealing the
//! type so hard the mismatch is inexpressible would leave the guarantee **unfalsifiable**, which is
//! the failure mode harness B1 was written against. So the route exists, in a `testonly` module, with
//! a name that reads as a defect on sight.
//!
//! ★★★ **This module is what makes "on sight" mean something.** A forge guarded only by its name is
//! guarded by whoever reads it next; the whole class `CLAUDE.md` calls *"derive the list, or make the
//! compiler hold it"* is about lists and boundaries that were right on the day they were written. Two
//! checks, both derived:
//!
//! | check | claim |
//! |---|---|
//! | [`forge_reaches_production`] | no call outside a `tests/` directory, over the PRODUCTION half of every `.rs` file in the workspace |
//! | [`forge_call_sites`] | …and at least one kill still calls it, so the forge is never kept alive by nothing |
//!
//! The second is not symmetry for its own sake: a forge with no caller is a public hole earning its
//! keep by habit, and the honest fix then is to delete it. When TY2026's Schedule 1-A is transcribed
//! the kill gains a lawful second revision to pair (that struct's own
//! `SENIOR_DEDUCTION_SUBTOTAL_LINE = 43`), the forge's last caller goes away, and this check says so
//! rather than letting it sit.
//!
//! ★ **Where it is blind, stated rather than implied.** Lawful call sites are recognised by living
//! under a `tests/` directory. A call from a `#[cfg(test)] mod tests` inside a `src/` file is
//! *skipped* by [`crate::r15_stop_list::production_source`] and so is not a finding — but it is not
//! counted by [`forge_call_sites`] either, so moving the kill there reds the floor and forces a
//! deliberate widening. That is the direction to fail in.

use std::path::{Path, PathBuf};

/// The forge's symbol, **assembled rather than written as one literal.**
///
/// ★ Not obfuscation: this module greps the workspace for that symbol, and a checker whose own source
/// contains the thing it forbids would have to excuse itself by path — the stale-excuse shape
/// `CLAUDE.md` names as a liability, earned by nothing. `r15_stop_list` splits its `serde_json` needle
/// for the identical reason.
const FORGE_HEAD: &str = "forge_senior_deduction_subtotal";
/// The second half of [`FORGE_HEAD`]'s symbol. See its note.
const FORGE_TAIL: &str = "_vouched_by_no_schedule";

/// The full symbol this module hunts.
#[must_use]
pub fn forge_symbol() -> String {
    format!("{FORGE_HEAD}{FORGE_TAIL}")
}

/// The walk must see at least this many files. **A check that scans nothing passes by finding
/// nothing** — `crates/**/*.rs` measured **355** on 2026-09-11
/// (`find crates -name "*.rs" -not -path "*/target*" | wc -l`). The floor is deliberately loose, not a
/// pinned count: it exists so a scan that reads nothing cannot pass by finding nothing.
///
/// ★ A SECOND blind spot, narrower than the one below and provably inert (re-verification Nit 2): a forge
/// call typed inside a ```rust fence in a doc comment is invisible here, because [`production_source`]
/// strips every `//`-leading line before the `#[cfg(test)]` skip runs. It cannot smuggle an executing
/// production call — doc-test code never compiles into the shipped library — and a doc-test cannot satisfy
/// the anti-vacuity floor either, which still reds. Named so the boundary is stated rather than discovered.
const FILE_FLOOR: usize = 300;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("crates/xtask -> repo root")
        .to_path_buf()
}

/// Every `.rs` file under `crates/`, sorted, as `(repo-relative label, source)` pairs.
fn workspace_sources() -> Vec<(String, String)> {
    let root = repo_root();
    let mut files = Vec::new();
    let mut stack = vec![root.join("crates")];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                // `target/` holds generated and vendored sources; it is not the workspace's text.
                if p.file_name().is_some_and(|n| n == "target") {
                    continue;
                }
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "rs") {
                files.push(p);
            }
        }
    }
    files.sort();
    files
        .into_iter()
        .map(|p| {
            let label = p
                .strip_prefix(&root)
                .unwrap_or(&p)
                .to_string_lossy()
                .replace('\\', "/");
            let src = std::fs::read_to_string(&p).unwrap_or_default();
            (label, src)
        })
        .collect()
}

/// True when `label` names a file under a `tests/` directory — i.e. an integration test target, which
/// is not shipped code.
fn is_integration_test(label: &str) -> bool {
    label.contains("/tests/")
}

/// ★★★ **Every occurrence of the forge in shipped code.** Pure over `(label, source)` pairs so a
/// planted defect can reach it (B1).
///
/// An occurrence is lawful in exactly two shapes, and both are recognised by **shape, not by path**:
///
/// 1. the **declaration** — a line containing `fn <symbol>`. The function has to live somewhere, and
///    naming its file here would be a hand-written path standing beside a thing that can move.
/// 2. a call from a file under a `tests/` directory.
///
/// Everything else is a finding, including a call from a `src/` file's shipped half, from a binary,
/// from a build script or from another crate entirely. Comments and doc comments are stripped by
/// [`crate::r15_stop_list::production_source`], so a file that merely *names* the forge in prose —
/// this module's own header does — is not reported as calling it.
#[must_use]
pub fn forge_reaches_production(files: &[(String, String)]) -> Vec<String> {
    let needle = forge_symbol();
    let mut out = Vec::new();
    for (label, src) in files {
        if is_integration_test(label) {
            continue;
        }
        for (i, line) in crate::r15_stop_list::production_source(src)
            .lines()
            .enumerate()
        {
            if !line.contains(&needle) {
                continue;
            }
            // The declaration itself, wherever it lives.
            if line.contains(&format!("fn {needle}")) {
                continue;
            }
            out.push(format!("{label}:{}: {}", i + 1, line.trim()));
        }
    }
    out
}

/// ★ **The other direction: the kills that still use the forge.** A forge nothing calls is a public
/// hole kept alive by habit, and the honest response is deletion — see the module header.
#[must_use]
pub fn forge_call_sites(files: &[(String, String)]) -> Vec<String> {
    let needle = forge_symbol();
    let mut out = Vec::new();
    for (label, src) in files {
        if !is_integration_test(label) {
            continue;
        }
        for (i, line) in crate::r15_stop_list::production_source(src)
            .lines()
            .enumerate()
        {
            if line.contains(&needle) && !line.contains(&format!("fn {needle}")) {
                out.push(format!("{label}:{}", i + 1));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The committed tree: the forge reaches no shipped seam, and a kill still exercises it.
    #[test]
    fn the_provenance_forge_is_reachable_only_from_a_kill_test() {
        let files = workspace_sources();
        assert!(
            files.len() >= FILE_FLOOR,
            "the walk saw {} files (floor {FILE_FLOOR}) — a scan that reads nothing reports clean",
            files.len()
        );
        let reached = forge_reaches_production(&files);
        assert!(
            reached.is_empty(),
            "the Schedule 1-A provenance forge is called from shipped code. A figure paired with a \
             line number no revision printed overstates the AMT base by ≈MAGI, and every other \
             instrument stays green because they all take the figure as INPUT. Production obtains a \
             `SeniorDeductionSubtotal` from `Schedule1A::senior_deduction_subtotal`: {reached:?}"
        );
        let callers = forge_call_sites(&files);
        assert!(
            !callers.is_empty(),
            "no kill test calls the forge any more. Either the join's kill was deleted — in which \
             case the guarantee is unfalsifiable and harness B1 is violated — or a real second \
             Schedule 1-A revision now supplies the mismatch, in which case DELETE the forge rather \
             than leaving a public route to an unvouched line number with no caller."
        );
    }

    /// ★★★ **B1 — both directions watched on planted defects, and on the near misses beside them.**
    ///
    /// The near misses are the point. A check that reds on everything is deleted by the next person
    /// who trips it; a check seen red on nothing was never seen discriminating at all.
    #[test]
    fn a_production_call_to_the_forge_reds_and_its_near_misses_do_not() {
        let sym = forge_symbol();
        let at = |label: &str, src: String| forge_reaches_production(&[(label.into(), src)]).len();

        // ── The plant: the porting mistake this whole seam exists to refuse. A later year's
        //    `return_1040` hand-assembling Part I and typing the line number it wants.
        assert_eq!(
            at(
                "crates/btctax-core/src/tax/return_1040.rs",
                format!("            senior_deduction: schedule_1a::testonly::{sym}(l43, 43),"),
            ),
            1,
            "a production seam calling the forge must be a finding — this is the defect"
        );
        // …and from any other shipped surface, not only core.
        assert_eq!(
            at(
                "crates/btctax-cli/src/report.rs",
                format!("    let s = {sym}(amount, 43);"),
            ),
            1,
            "the check must not be scoped to one crate"
        );

        // ── Near miss 1: the declaration. It has to live somewhere.
        assert_eq!(
            at(
                "crates/btctax-core/src/tax/schedule_1a.rs",
                format!("    pub fn {sym}(amount: Usd, schedule_1a_line: u32) -> SeniorDeductionSubtotal {{"),
            ),
            0,
            "the function's own declaration is not a call"
        );
        // ── Near miss 2: prose. A doc comment naming the forge — this module's header does it four
        //    times — must not be reported as calling it.
        assert_eq!(
            at(
                "crates/btctax-forms/tests/../src/f6251_revision.rs",
                format!("/// never call {sym} from here; see the module header.\npub fn ok() {{}}"),
            ),
            0,
            "a comment that NAMES the forge is not a call to it"
        );
        // ── Near miss 3: a call inside a `#[cfg(test)]` item in a `src/` file. Skipped, per the
        //    module header's stated blind spot — it is test code, and the direction to fail in.
        assert_eq!(
            at(
                "crates/btctax-forms/src/f6251_revision.rs",
                format!("#[cfg(test)]\nmod tests {{\n    fn k() {{ let _ = {sym}(a, 43); }}\n}}\n"),
            ),
            0,
            "a `#[cfg(test)]` item is not shipped code"
        );
        // ── Near miss 4: an integration test, the lawful home of the kill.
        assert_eq!(
            at(
                "crates/btctax-forms/tests/f6251_obbba.rs",
                format!("            senior_deduction: {sym}(dec!(6000), other),"),
            ),
            0,
            "the kill's own call site is lawful"
        );

        // ── The call-site floor, in both directions.
        assert_eq!(
            forge_call_sites(&[(
                "crates/btctax-forms/tests/f6251_obbba.rs".into(),
                format!("    let s = {sym}(dec!(6000), 43);"),
            )])
            .len(),
            1,
            "a kill's call must be counted, or the floor can never notice the kill's deletion"
        );
        assert!(
            forge_call_sites(&[(
                "crates/btctax-forms/tests/f6251_obbba.rs".into(),
                "    let s = vouched_senior_subtotal(dec!(6000));".to_string(),
            )])
            .is_empty(),
            "the LAWFUL route must not be counted as a forge call — otherwise the floor is \
             satisfied by a test that never forges anything and the kill can vanish unnoticed"
        );
    }
}
