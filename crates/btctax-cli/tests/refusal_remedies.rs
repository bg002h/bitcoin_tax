//! ★★★ **FR-225 — A REMEDY A REFUSAL NAMES MUST EXIST.**
//!
//! FR-225's remainder was not a wrong figure and not a missing refusal. `screen_absolute`'s
//! §170(f)(8)(A) refusal is **correct** — the statute denies the deduction outright without a
//! contemporaneous written acknowledgment — and it fired exactly when it should. What was broken is
//! the sentence *after* it: the refusal named two cures, and the filer could perform neither.
//!
//! ★★ **The class is invisible to every other instrument in this repo, which is why it needs its
//! own.** Both oracles agree with the figures; the golden packets are byte-stable; the refusal
//! refuses. A dead-end cure is a defect in the **English**, measured against the **command tree** —
//! nothing else here holds both halves at once. It took a journey walk to find, and a journey walk
//! does not run on every commit.
//!
//! ★★★ **So the checker is DERIVED ON BOTH ENDS**, per `CLAUDE.md`'s *"derive the list, or make the
//! compiler hold it — never type one beside a set that grows"*:
//!
//!   - the **cited** set is scanned from every `.rs` file under `crates/`, so a remedy named in a
//!     refusal written next year is covered without anyone editing this file;
//!   - the **valid** set is walked from `Cli::command()` — clap's own tree — so renaming or deleting
//!     a verb reds this test instead of silently falsifying a printed instruction.
//!
//! **What it does NOT cover, stated plainly** (`design/HARNESS.md` B1: a gate that hides its own
//! blind spot is worse than no gate):
//!
//!   1. **Existence, not efficacy.** It proves a named command *exists*; it cannot prove that running
//!      it changes the filer's state. The behavioural half for this refusal is
//!      `tests/year_gate_t4.rs::fr225_income_answer_states_the_returns_own_verdict_and_reaches_the_refusing_question`,
//!      which drives `income answer` over a bricked return and asserts the stored answer moves.
//!      Per FR-235 the two are deliberately different instruments: a grep over the refusal string
//!      would be satisfied by editing the refusal string.
//!   2. **Backticked citations in non-comment lines only** — i.e. string literals, the text the
//!      product prints. A remedy named in prose, or without backticks, is not seen. Doc comments are
//!      excluded on purpose: they discuss the design, and a design note may legitimately name a flag
//!      that has since been removed.
//!   3. **Bare tokens past the subcommand path are treated as VALUES** and checked no further, so
//!      `btctax config --set-forward-method hifo` resolves `config` and `--set-forward-method` and
//!      accepts `hifo` unexamined. A bare token where clap *does* expect a subcommand is still an
//!      error — that is the hole a leaf-only rule would have left open.
//!   4. A `` `--flag` `` span with no command attached is resolved against the union of long options
//!      over the whole tree, so it catches *"this flag exists nowhere"* but not *"this flag is on a
//!      different command"*.

use btctax_cli::cli::Cli;
use clap::CommandFactory;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

// ════════════════════════════════════════════════════════════════════════════════════════════════
// The scanner: what the product's strings CITE.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// One citation found in a printed string.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Citation {
    /// A `` `btctax …` `` span, as the tokens after `btctax`.
    Command(Vec<String>),
    /// A bare `` `--flag` `` span, without the leading dashes.
    Flag(String),
}

/// Every `.rs` file under `crates/`. **Walked, never listed** — a new crate, or a new module in an
/// old one, is covered the day it lands.
fn rust_sources() -> Vec<(PathBuf, String)> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for e in std::fs::read_dir(dir).expect("read_dir").flatten() {
            let p = e.path();
            if p.is_dir() {
                // Generated artefacts are not this workspace's text.
                if p.file_name().is_some_and(|n| n == "target") {
                    continue;
                }
                walk(&p, out);
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push(p);
            }
        }
    }
    let crates = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/ is the parent of this manifest dir");
    let mut files = Vec::new();
    walk(crates, &mut files);
    files.sort();
    assert!(
        files.len() > 50,
        "the walk found only {} .rs files — it is not reading the workspace",
        files.len()
    );
    files
        .into_iter()
        .map(|p| {
            let t = std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {p:?}: {e}"));
            (p, t)
        })
        .collect()
}

/// `src` with whole-line comments dropped and Rust's `\`-newline string continuations re-joined.
///
/// ★ The join is not cosmetic. Refusal details are long `format!` strings broken with `\` at end of
///   line, and Rust's continuation escape eats the newline **and the following indentation** — so a
///   citation may be split across two source lines. Without the join the scanner would silently miss
///   exactly the longest, most instruction-dense strings in the product.
fn printed_text(src: &str) -> String {
    let mut kept = String::with_capacity(src.len());
    for line in src.lines() {
        if line.trim_start().starts_with("//") {
            continue;
        }
        kept.push_str(line);
        kept.push('\n');
    }
    let mut out = String::with_capacity(kept.len());
    let mut it = kept.chars().peekable();
    while let Some(c) = it.next() {
        if c == '\\' && it.peek() == Some(&'\n') {
            it.next();
            while it.peek().is_some_and(|n| n.is_whitespace() && *n != '\n') {
                it.next();
            }
            continue;
        }
        out.push(c);
    }
    out
}

/// A token that stands in for a value rather than naming one: a `{}`/`{year}` format placeholder or
/// a `<YEAR>`-style usage metavariable. Dropped per TOKEN rather than per span, so
/// `` `btctax income answer --year {year}` `` still checks `income`, `answer` and `--year`.
fn is_placeholder(tok: &str) -> bool {
    tok.contains('{')
        || tok.contains('}')
        || tok.contains('<')
        || tok.contains('…')
        || tok.contains("...")
}

/// The citations in `text`. Backtick spans are taken per LINE, so one unbalanced backtick cannot
/// shift the parity of a whole file.
fn citations(text: &str) -> BTreeSet<Citation> {
    let mut out = BTreeSet::new();
    for line in text.lines() {
        for span in line.split('`').skip(1).step_by(2) {
            let span = span.trim();
            if let Some(rest) = span.strip_prefix("btctax") {
                if !rest.is_empty() && !rest.starts_with(' ') {
                    continue; // `btctax-core`, `btctax_cli`, …
                }
                out.insert(Citation::Command(
                    rest.split_whitespace()
                        .filter(|t| !is_placeholder(t))
                        .map(str::to_string)
                        .collect(),
                ));
            } else if let Some(flag) = span.strip_prefix("--") {
                if !flag.is_empty() && !flag.contains(char::is_whitespace) && !is_placeholder(flag)
                {
                    out.insert(Citation::Flag(flag.to_string()));
                }
            }
        }
    }
    out
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// The resolver: what the CLI actually HAS. Walked from clap, never transcribed.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The long options declared on `cmd` itself; `globals_only` keeps just those clap propagates down.
fn long_flags_of(cmd: &clap::Command, globals_only: bool) -> BTreeSet<String> {
    cmd.get_arguments()
        .filter(|a| !globals_only || a.is_global_set())
        .filter_map(|a| a.get_long().map(str::to_string))
        .collect()
}

/// Every long option anywhere in the tree.
fn all_long_flags(cmd: &clap::Command, out: &mut BTreeSet<String>) {
    out.extend(long_flags_of(cmd, false));
    for s in cmd.get_subcommands() {
        all_long_flags(s, out);
    }
}

/// The set a bare `` `--flag` `` citation is resolved against: every long option in the tree, plus
/// the two clap synthesises on every command and therefore never lists.
fn known_flags() -> BTreeSet<String> {
    let mut f = BTreeSet::new();
    all_long_flags(&Cli::command(), &mut f);
    f.insert("help".to_string());
    f.insert("version".to_string());
    f
}

/// `Ok` when every token of a `btctax …` citation names something the CLI has.
fn resolve_command(root: &clap::Command, tokens: &[String]) -> Result<(), String> {
    let mut cur = root;
    let mut globals = long_flags_of(root, true);
    let mut path = vec![root.get_name().to_string()];
    // Past the subcommand path, bare tokens are argument VALUES (module doc, boundary 3).
    let mut in_values = false;
    for t in tokens {
        if let Some(flag) = t.strip_prefix("--") {
            // clap gives every command both, and neither appears in `get_arguments()` unbuilt.
            if flag == "help" || flag == "version" {
                continue;
            }
            if !long_flags_of(cur, false).contains(flag) && !globals.contains(flag) {
                return Err(format!(
                    "`--{flag}` is not an option of `{}`",
                    path.join(" ")
                ));
            }
            continue;
        }
        if in_values {
            continue;
        }
        match cur.find_subcommand(t.as_str()) {
            Some(sub) => {
                globals.extend(long_flags_of(cur, true));
                cur = sub;
                path.push(t.clone());
            }
            // ★ THE HOLE A LEAF-ONLY RULE WOULD LEAVE: `btctax income bogus` must not pass as
            //   "`income` plus a value". A command that has subcommands takes one.
            None if cur.has_subcommands() => {
                return Err(format!(
                    "`{t}` is not a subcommand of `{}` (it has: {})",
                    path.join(" "),
                    cur.get_subcommands()
                        .map(clap::Command::get_name)
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            None => in_values = true,
        }
    }
    Ok(())
}

/// ★★★ **THE ONE PLACE A DOC COMMENT *IS* PRINTED OUTPUT.** `cli.rs`'s `///` comments become clap's
///     `--help` text, so the module doc's boundary 2 — *"doc comments are excluded, they discuss the
///     design"* — is FALSE of that file, and excluding it would have left the largest filer-facing
///     text surface in the product unchecked. This walks clap's own rendered strings instead of the
///     source, so it reads exactly what `--help` prints and needs no per-file exception.
///
/// ★ It found a live one on its first run: `income answer`'s long help said SSNs *"belong to
///   `set-pii`"*, a command that has never existed.
fn help_strings(cmd: &clap::Command, out: &mut Vec<(String, String)>) {
    let name = cmd.get_name().to_string();
    let mut push = |s: Option<&clap::builder::StyledStr>| {
        if let Some(s) = s {
            out.push((name.clone(), s.to_string()));
        }
    };
    push(cmd.get_about());
    push(cmd.get_long_about());
    push(cmd.get_before_help());
    push(cmd.get_after_help());
    for a in cmd.get_arguments() {
        if let Some(h) = a.get_help() {
            out.push((format!("{name} --{}", a.get_id()), h.to_string()));
        }
        if let Some(h) = a.get_long_help() {
            out.push((format!("{name} --{}", a.get_id()), h.to_string()));
        }
    }
    for s in cmd.get_subcommands() {
        help_strings(s, out);
    }
}

fn rel(p: &Path) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("repo root");
    p.strip_prefix(root).unwrap_or(p).display().to_string()
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// KILL 1 — the whole workspace: no printed string names a command or flag that does not exist.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// ★★★ **The general form of FR-225, made a command rather than a discipline.**
///
/// **Plants watched going red on this tree** (both, separately):
///   - `screen_absolute`'s §170(f)(8) refusal changed to say ``Run `btctax income acknowledge` ``;
///   - the deferred-carryover cure changed to name `` `--write-carryovers` ``.
#[test]
fn every_command_and_flag_a_printed_string_names_exists_in_the_cli() {
    let root = Cli::command();
    let flags = known_flags();

    let mut problems: Vec<String> = Vec::new();
    let mut commands_seen: BTreeSet<Vec<String>> = BTreeSet::new();
    let mut flags_seen: BTreeSet<String> = BTreeSet::new();

    for (path, text) in rust_sources() {
        let in_xtask = path.components().any(|c| c.as_os_str() == "xtask");
        for c in citations(&printed_text(&text)) {
            match c {
                Citation::Command(tokens) => {
                    commands_seen.insert(tokens.clone());
                    if let Err(e) = resolve_command(&root, &tokens) {
                        problems.push(format!(
                            "{}: the product prints `btctax {}` — {e}",
                            rel(&path),
                            tokens.join(" ")
                        ));
                    }
                }
                // ★★ **A BARE FLAG IS ONLY CHECKED WHERE THE AMBIENT BINARY IS `btctax`**, and
                //    `xtask` is the one crate here where it is not: it is the `cargo xtask` dev tool
                //    with its own clap tree, so its `--all` and `--restore` are its own options and
                //    resolving them against btctax would red on a true statement. Its `` `btctax …` ``
                //    spans ARE still checked above — a span that names the binary is unambiguous
                //    about which tree it means, which is exactly what a bare flag is not.
                Citation::Flag(f) if !in_xtask => {
                    flags_seen.insert(f.clone());
                    if !flags.contains(&f) {
                        problems.push(format!(
                            "{}: the product prints `--{f}`, which is not an option of any btctax \
                             command",
                            rel(&path)
                        ));
                    }
                }
                Citation::Flag(_) => {}
            }
        }
    }

    // ── …and clap's own `--help` text, which is `cli.rs`'s doc comments RENDERED. See
    //    `help_strings`: excluding doc comments is right everywhere except there.
    let mut helps = Vec::new();
    help_strings(&root, &mut helps);
    assert!(
        helps.len() > 100,
        "only {} help strings — the walk is not reading the command tree",
        helps.len()
    );
    for (where_, text) in &helps {
        for c in citations(text) {
            match c {
                Citation::Command(tokens) => {
                    commands_seen.insert(tokens.clone());
                    if let Err(e) = resolve_command(&root, &tokens) {
                        problems.push(format!(
                            "`btctax {where_}` --help prints `btctax {}` — {e}",
                            tokens.join(" ")
                        ));
                    }
                }
                Citation::Flag(f) => {
                    flags_seen.insert(f.clone());
                    if !flags.contains(&f) {
                        problems.push(format!(
                            "`btctax {where_}` --help prints `--{f}`, which is not an option of any \
                             btctax command"
                        ));
                    }
                }
            }
        }
    }

    // ★★★ B1a — THE SCANNER IS ASSERTED TO HAVE MEASURED SOMETHING. A checker that silently matched
    //     nothing would report this whole class clean forever, which is F2/F4's exact shape.
    //
    // ★ The anchor is the citation FR-225 was opened on, and it is a LIVENESS PROBE rather than a
    //   list: `btctax income answer` is the remedy the §170(f)(8)(A) refusal names, so a scanner that
    //   stops seeing it has stopped measuring the thing it was built for — and a fold that renames
    //   that remedy has to come here and say so.
    assert!(
        commands_seen.contains(&vec!["income".to_string(), "answer".to_string()]),
        "the scanner did not find `btctax income answer` — the §170(f)(8)(A) refusal's own remedy. \
         Either the remedy was renamed, or the scanner has stopped reading printed strings. It \
         saw: {commands_seen:?}"
    );
    assert!(
        commands_seen.len() >= 10 && flags_seen.len() >= 5,
        "the scanner found {} command citations and {} flag citations — too few to be reading the \
         workspace's printed strings",
        commands_seen.len(),
        flags_seen.len()
    );

    assert!(
        problems.is_empty(),
        "a printed string names {} command(s)/flag(s) that do not exist. A filer who follows the \
         instruction reaches a dead end, and no oracle, golden or refusal test can see it:\n  {}",
        problems.len(),
        problems.join("\n  ")
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// KILL 2 — the §170(f)(8) refusal's OWN runtime text, on every arm that fires.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// A TY2024 return that ITEMIZES because of one $40,000 cash gift, with the §170(f)(8)(A)
/// acknowledgment question answered `answered` — FR-225's reproduction.
///
/// ★ $50,000 of W-2 wages so the §170(b) 60%-of-AGI ceiling admits $30,000 of the gift, which clears
///   TY2024's $14,600 standard deduction. `ar.deduction_is_itemized` is the gate's first conjunct, so
///   the fixture has to EARN the refusal rather than assert it.
///
/// ★★ The gift is a `schedule_a.charitable` row and the ledger is EMPTY, which is deliberate: this
///    fixture exercises the source whose cure btctax can actually offer. The ledger-donation arm —
///    the one with no cure — is driven by [`crypto_donor_with_an_unresolved_cwa`].
fn itemizer_with_an_unresolved_cwa(
    answered: Option<bool>,
) -> btctax_core::tax::return_inputs::ReturnInputs {
    use btctax_core::tax::return_inputs::{
        CharitableClass, CharitableGift, Owner, ReturnInputs, ScheduleAInputs, W2,
    };
    let mut h = btctax_core::tax::testonly::not_a_dependent();
    h.taxpayer.date_of_birth = Some(time::macros::date!(1980 - 05 - 05));
    let mut ri = ReturnInputs {
        tax_year: 2024,
        filing_status: btctax_core::FilingStatus::Single,
        header: h,
        w2s: vec![W2 {
            owner: Owner::Taxpayer,
            box1_wages: rust_decimal_macros::dec!(50000),
            box3_ss_wages: rust_decimal_macros::dec!(50000),
            box5_medicare_wages: rust_decimal_macros::dec!(50000),
            ..Default::default()
        }],
        schedule_a: Some(ScheduleAInputs {
            charitable: vec![CharitableGift {
                class: CharitableClass::Cash60,
                amount: rust_decimal_macros::dec!(40000),
            }],
            ..Default::default()
        }),
        ..Default::default()
    };
    btctax_core::tax::testonly::answer_all_live_declarations(&mut ri);
    if let Some(v) = answered {
        ri.charitable_cwa_obtained = Some(v);
    }
    ri
}

/// A `LedgerState` holding one long-term crypto `Donation` of `fmv` in TY2024 — the source whose gift
/// btctax **cannot** un-claim, because `project::fold` computes `claimed_deduction` from the legs and
/// no CLI verb writes it.
///
/// ★ Built with the public `state` types rather than through a projection: what this arm turns on is
///   `max_single_donation_contribution`, which reads exactly these fields.
fn one_crypto_donation(fmv: rust_decimal::Decimal) -> btctax_core::LedgerState {
    use btctax_core::event::BasisSource;
    use btctax_core::identity::{EventId, LotId, Source, SourceRef};
    use btctax_core::state::{LedgerState, Removal, RemovalKind, RemovalLeg, Term};
    let event = EventId::import(Source::Coinbase, SourceRef::new("DONATION-1"));
    LedgerState {
        removals: vec![Removal {
            event: event.clone(),
            kind: RemovalKind::Donation,
            removed_at: time::macros::date!(2024 - 09 - 09),
            legs: vec![RemovalLeg {
                lot_id: LotId {
                    origin_event_id: event,
                    split_sequence: 0,
                },
                sat: 10_000_000,
                basis: fmv / rust_decimal_macros::dec!(5),
                fmv_at_transfer: fmv,
                term: Term::LongTerm,
                basis_source: BasisSource::ExchangeProvided,
                acquired_at: time::macros::date!(2019 - 01 - 01),
                pseudo: false,
            }],
            appraisal_required: false,
            donor_acquired_at: None,
            // §170(e): a long-term appreciated capital asset deducts FMV. What `fold` would write.
            claimed_deduction: Some(fmv),
            donee: Some("A Public Charity".to_string()),
        }],
        ..Default::default()
    }
}

/// The refusal `btctax report` would give for `ri` over `state`, computed through the same three
/// screens `income answer`'s own verdict uses.
fn refusal_for(
    ri: &btctax_core::tax::return_inputs::ReturnInputs,
    state: &btctax_core::LedgerState,
) -> Option<btctax_core::tax::return_refuse::Refusal> {
    use btctax_core::tax::tables::{FullReturnTables as _, TaxTables as _};
    let year = 2024;
    let ft = btctax_adapters::BundledFullReturnTables::load();
    let params = ft.full_return_for(year).expect("TY2024 params are bundled");
    let tt = btctax_adapters::BundledTaxTables::load();
    let table = tt.table_for(year).expect("the TY2024 table is bundled");
    let regime = btctax_cli::year_readiness::regime_for(year).expect("TY2024 has a year record");
    let ar = btctax_core::assemble_absolute(ri, state, params, table, year);
    btctax_core::screen_absolute(ri, &ar, params, state, year, regime)
}

/// Check every citation in one runtime refusal detail. Returns the problems found.
fn dead_ends_in(detail: &str) -> Vec<String> {
    let root = Cli::command();
    let flags = known_flags();
    let mut bad = Vec::new();
    for c in citations(detail) {
        match c {
            Citation::Command(tokens) => {
                if let Err(e) = resolve_command(&root, &tokens) {
                    bad.push(format!("`btctax {}` — {e}", tokens.join(" ")));
                }
            }
            Citation::Flag(f) => {
                if !flags.contains(&f) {
                    bad.push(format!("`--{f}` is not an option of any btctax command"));
                }
            }
        }
    }
    bad
}

/// ★★★ **THE ONE THAT MATTERS: the refusal the filer actually READS names only cures that exist.**
///
/// KILL 1 reads source text; this reads the **runtime string**, on **both** arms of the gate — the
/// `None` arm and the `Some(false)` arm, which are different sentences with different cures, and
/// only one of which any test had ever read.
///
/// It also refuses a refusal that names **no** cure at all: an omitted remedy is the same dead end
/// arrived at from the other side, and that is the mutation a pure existence check survives.
///
/// **Plant to watch it red:** make the `Some(false)` cure say *"remove that gift with `btctax
/// charitable remove`"*, or delete every backticked command from either arm.
#[test]
fn the_170f8_refusal_names_only_cures_that_exist_on_every_arm_that_fires() {
    for answered in [None, Some(false)] {
        let ri = itemizer_with_an_unresolved_cwa(answered);
        let state = btctax_core::LedgerState::default();
        let r = refusal_for(&ri, &state).unwrap_or_else(|| {
            panic!("the fixture must EARN the refusal; with {answered:?} it computed cleanly")
        });
        assert_eq!(
            r.reason,
            btctax_core::tax::return_refuse::RefuseReason::CharitableCwaUnresolved,
            "the fixture must reach the §170(f)(8) gate, not some earlier screen: {}",
            r.detail
        );

        // ★ A refusal that names no command at all is a dead end by OMISSION.
        assert!(
            citations(&r.detail)
                .iter()
                .any(|c| matches!(c, Citation::Command(_))),
            "the {answered:?} arm names no command a filer could run:\n{}",
            r.detail
        );
        let bad = dead_ends_in(&r.detail);
        assert!(
            bad.is_empty(),
            "the {answered:?} arm of the §170(f)(8) refusal sends the filer to {} thing(s) that do \
             not exist: {}\n\n{}",
            bad.len(),
            bad.join("; "),
            r.detail
        );
    }
}

/// ★★★ **KILL 3 — THE CRYPTO ARM, WHICH IS THE ONE WITH NO CURE, AND MUST NOT PRETEND OTHERWISE.**
///
/// A `[[schedule_a.charitable]]` row can be deleted and re-imported; a ledger donation cannot be
/// un-claimed by any command, because `project::fold` computes `Removal.claimed_deduction` from the
/// donation's legs unconditionally. The shipped refusal printed *"remove that gift from the
/// deduction"* over **both**, so a crypto donor was handed an instruction the product cannot honour.
///
/// ★★ **This is the assertion that a text fix cannot fake**, and the reason it is not just KILL 2 with
///    another fixture: it pins the refusal to the **absence** of the un-claim offer on this source, and
///    to the presence of the acknowledgment-only sentence. A fold that "helpfully" restored the
///    forgo wording for crypto — or that offered to void the donation event, which would assert the
///    gift did not happen — reds here.
///
/// **Plant to watch it red:** make the `src.schedule_a_entry` clause unconditional in the
/// `cwa_claimed > 0` cure (i.e. drop the `if`), which is the shipped behaviour restored.
#[test]
fn the_crypto_arm_offers_no_forgo_because_btctax_cannot_un_claim_a_ledger_donation() {
    // A $40,000 LT crypto gift on $50,000 of wages: the §170(b)(1)(C) 30% ceiling admits $15,000,
    // which clears TY2024's $14,600 standard deduction, so the return ITEMIZES and the gate's first
    // conjunct holds. No `schedule_a.charitable` row at all — the ledger is the only source.
    let mut ri = itemizer_with_an_unresolved_cwa(Some(false));
    ri.schedule_a
        .as_mut()
        .expect("the fixture carries a Schedule A")
        .charitable
        .clear();
    // ★ A >$5,000 crypto gift files Form 8283 Section B, whose lines 5a-5c refuse the return until
    //   the restriction question is answered. Answered NO so the §170(f)(8) gate is the screen under
    //   test rather than the one before it — otherwise this fixture measures §G-21.
    ri.donations_had_restrictions = Some(false);
    let state = one_crypto_donation(rust_decimal_macros::dec!(40000));

    // B1a — the fixture must present the case, not assert it: exactly ONE source is armed.
    let src = btctax_core::tax::return_1040::cwa_gift_sources(&ri, &state, 2024);
    assert_eq!(
        (src.schedule_a_entry, src.ledger_donation),
        (false, true),
        "the fixture must arm the LEDGER source alone; it armed {src:?}"
    );

    let r =
        refusal_for(&ri, &state).expect("an itemizing crypto donor who holds no CWA is refused");
    assert_eq!(
        r.reason,
        btctax_core::tax::return_refuse::RefuseReason::CharitableCwaUnresolved,
        "the fixture must reach the §170(f)(8) gate: {}",
        r.detail
    );

    // ★ The shipped dead end, pinned out: the words a filer would act on and cannot.
    assert!(
        !r.detail.contains("remove that gift from the deduction"),
        "the crypto arm must not tell the filer to remove a gift btctax computes and no verb \
         un-claims:\n{}",
        r.detail
    );
    assert!(
        !r.detail.contains("[[schedule_a.charitable]]"),
        "…and must not send them to a TOML table their gift is not in:\n{}",
        r.detail
    );
    // ★ …and it must say what IS true, so the refusal is a boundary rather than a silence.
    assert!(
        r.detail.contains("acknowledgment is the only cure"),
        "a refusal with no alternative must SAY there is none — a silent boundary is the defect:\n{}",
        r.detail
    );
    // ★ btctax must never propose rewriting the ledger to escape a substantiation rule — and it must
    //   say so, because a filer with no other exit WILL reach for `reconcile void` otherwise.
    assert!(
        r.detail
            .contains("will not rewrite your ledger to get past a substantiation rule"),
        "…and must say why voiding or re-classifying the event is not the answer:\n{}",
        r.detail
    );
    // Every remedy it DOES name still has to exist — the same resolver as KILL 1 and KILL 2.
    let bad = dead_ends_in(&r.detail);
    assert!(
        bad.is_empty(),
        "the crypto arm names {} thing(s) that do not exist: {}\n\n{}",
        bad.len(),
        bad.join("; "),
        r.detail
    );

    // ── AND THE PAIRED HALF, so none of the above is a gate that always fires: the SCHEDULE-A
    //    source, on the same code path, DOES get the forgo offer. Without this an implementation that
    //    deleted the forgo wording entirely would pass every assertion above.
    let sched = itemizer_with_an_unresolved_cwa(Some(false));
    let empty = btctax_core::LedgerState::default();
    let s2 = btctax_core::tax::return_1040::cwa_gift_sources(&sched, &empty, 2024);
    assert_eq!(
        (s2.schedule_a_entry, s2.ledger_donation),
        (true, false),
        "premise: the paired fixture arms the SCHEDULE-A source alone; it armed {s2:?}"
    );
    let r2 = refusal_for(&sched, &empty).expect("the same gate fires on the Schedule A source");
    assert!(
        r2.detail.contains("[[schedule_a.charitable]]")
            && r2.detail.contains("FORGO the deduction for that gift"),
        "the source btctax CAN un-claim must be offered the forgo, in the words of the act it \
         is — not a retraction of testimony:\n{}",
        r2.detail
    );
    assert!(
        !r2.detail.contains("acknowledgment is the only cure"),
        "…and must not claim there is no alternative when there is one:\n{}",
        r2.detail
    );
}
