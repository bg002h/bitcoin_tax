//! **T7 smoke test** — the oracle-sweep harness assembles + fills + reads a scenario BACK OFF THE PAPER.
//!
//! The floor case (`single_w2_only_standard`) is piped through the harness as a bare `GoldenInputs`
//! JSON on stdin; the harness assembles the SAME return the golden matrix fills, reads it back with
//! `extract_lines`, and prints the flattened `form.line → string` map. The assertion is that the AGI in
//! the box on the 1040 (`1040.line11`) is the AGI two independent oracles baked into the matrix — read
//! from the baked value, never hard-coded.
//!
//! A second case drives `--check` (the I4 reproduction/classification mode) over a whole golden
//! household and asserts every line reconciles, since the golden matrix is green by construction.

use std::io::Write;
use std::process::{Command, Stdio};

use btctax_core::tax::testonly::{golden_households, GOLDEN_RETURNS_JSON};

const HARNESS: &str = env!("CARGO_BIN_EXE_btctax-oracle-harness");

/// Run the harness with `args`, feeding `stdin_json`, and return its stdout as a parsed JSON value.
fn run(args: &[&str], stdin_json: &str) -> serde_json::Value {
    let mut child = Command::new(HARNESS)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the harness binary spawns");
    child
        .stdin
        .take()
        .expect("stdin is piped")
        .write_all(stdin_json.as_bytes())
        .expect("write the scenario to the harness");
    let out = child
        .wait_with_output()
        .expect("the harness runs to completion");
    assert!(
        out.status.success(),
        "the harness exited non-zero (args {args:?}):\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "the harness stdout is not JSON ({e}):\n{}",
            String::from_utf8_lossy(&out.stdout)
        )
    })
}

/// The raw JSON object for the named household, straight from the baked matrix (so the harness is fed
/// exactly the committed shape, no round-trip through a partial re-serialization).
fn raw_household(name: &str) -> serde_json::Value {
    let matrix: serde_json::Value =
        serde_json::from_str(GOLDEN_RETURNS_JSON).expect("the golden matrix parses");
    matrix["households"]
        .as_array()
        .expect("households is an array")
        .iter()
        .find(|h| h["name"] == serde_json::json!(name))
        .unwrap_or_else(|| panic!("{name} is in the golden matrix"))
        .clone()
}

#[test]
fn floor_case_reads_back_the_baked_ots_agi_off_the_paper() {
    let name = "single_w2_only_standard";

    // The baked AGI two independent oracles agreed on — read from the matrix, never guessed.
    let baked_agi = golden_households()
        .iter()
        .find(|h| h.name == name)
        .expect("the floor case is in the matrix")
        .expected_ots
        .adjusted_gross_income;
    let expected = format!("{}", baked_agi as i64); // whole dollars on the paper (SPEC §3.1)

    // Feed the bare GoldenInputs (the default-mode contract).
    let inputs = raw_household(name)["inputs"].clone();
    let out = run(&[], &serde_json::to_string(&inputs).unwrap());

    assert_eq!(
        out["refused"],
        serde_json::json!(false),
        "the floor case is refusal-free"
    );
    assert_eq!(
        out["lines"]["1040.line11"],
        serde_json::json!(expected),
        "the AGI in the box on the 1040 must be the baked oracle AGI ({expected})"
    );
}

/// ★ **EMPTY BY DESIGN, and its emptiness is the proof.** This used to hold
/// `mfj_high_income_niit_and_addl_medicare`: btctax's Form 6251 *screening worksheet* flagged that
/// high-income anchor as "may owe AMT" and refused the whole return, even though BOTH oracles computed
/// zero actual AMT — so the harness reported it out-of-domain and the sweep could never reach it.
///
/// btctax now **computes Form 6251** rather than refusing on the screen. The worksheet only ever said
/// "fill in Form 6251"; filling it in shows line 7 ≤ line 10, so no attachment is required
/// (i6251, Who Must File, condition 1) and the return is produced. The anchor is admitted and swept.
///
/// A name reappearing here means a household now genuinely owes AMT (or must attach the form) — that is
/// a deliberate change to adjudicate, not something to paper over.
const EXPECTED_REFUSED: &[&str] = &[];

/// The inverse of the test this replaces: the former AMT-screen anchor now **proceeds**.
///
/// This is the end-to-end proof that the screen-tripping, zero-AMT population was un-refused — worth
/// strictly more than the refusal assertion it replaces, because it exercises the whole path rather
/// than the fact that it stopped early.
#[test]
fn the_former_amt_screen_anchor_now_proceeds_in_default_mode() {
    let name = "mfj_high_income_niit_and_addl_medicare";
    let inputs = raw_household(name)["inputs"].clone();
    let out = run(&[], &serde_json::to_string(&inputs).unwrap());
    assert_eq!(
        out["refused"],
        serde_json::json!(false),
        "{name} trips the cheap AMT SCREEN, but Form 6251 computes line 7 <= line 10, so the return \
         must now be produced rather than refused"
    );
    assert!(
        out.get("lines").is_some(),
        "an admitted scenario carries its filled lines"
    );
}

/// One household's `--check` outcome: the name it is reported under, and the harness's whole JSON.
struct Outcome {
    name: String,
    check: serde_json::Value,
}

/// Drive `--check` over one household.
fn check_one(household: &serde_json::Value) -> Outcome {
    Outcome {
        name: household["name"]
            .as_str()
            .unwrap_or("<unnamed>")
            .to_string(),
        check: run(&["--check"], &serde_json::to_string(household).unwrap()),
    }
}

/// Drive `--check` over every household, **in parallel**, preserving corpus order in the result.
///
/// ★★★ **FR-234 D4 — this was never "inherently serial", and saying so kept it out of CI.** One
/// subprocess *per household* is not one subprocess *at a time*: the work is process-bound and
/// embarrassingly parallel, and the standing directive names exactly this shape ("consider parallel
/// execution for ALL tests, cache generation and long calculations" — 24 cores on this box). The
/// whole-corpus twin below is what the parallelism is for: it is what makes a real CI runner for all
/// 107 households affordable instead of a promise in an `#[ignore]` reason.
fn check_all(households: &[serde_json::Value]) -> Vec<Outcome> {
    let workers = std::thread::available_parallelism().map_or(4, |n| n.get());
    let chunk = households.len().div_ceil(workers).max(1);
    let mut outcomes: Vec<Outcome> = Vec::with_capacity(households.len());
    std::thread::scope(|scope| {
        let handles: Vec<_> = households
            .chunks(chunk)
            .map(|part| scope.spawn(move || part.iter().map(check_one).collect::<Vec<Outcome>>()))
            .collect();
        // Joined in chunk order, so the roster a failure prints is deterministic.
        for handle in handles {
            outcomes.extend(handle.join().expect("a sweep worker thread finishes"));
        }
    });
    outcomes
}

/// ★★★ **FR-234 D2 — the WHOLE roster, collected, instead of the first failure.**
///
/// The sweep used to `assert_eq!` on `all_reconciled` **inside** its per-household loop, so it
/// reported one household and stopped. That cannot answer the only question that matters when the
/// corpus goes red — *how many diverge, and do they share a mechanism?* — which is the difference
/// between one lawful rounding residual and a systemic wrong figure. Measured: the pre-fix run died
/// 5s in, on household 1 of 107, while 21 of them were red for one shared reason.
///
/// So every household is driven, every divergence is kept, and the assertion happens ONCE with the
/// full roster in the message.
#[derive(Default)]
struct SweepReport {
    /// Households the harness declared out of domain.
    refused: Vec<String>,
    /// Households that were swept.
    admitted: usize,
    /// `(household, one line per diverging verdict)` — every divergence, not the first.
    divergences: Vec<(String, Vec<String>)>,
    /// Admitted households whose T7-m1 structural reproduction witness failed.
    reproduction_failures: Vec<String>,
    /// ★ The absorbed-row census: how many verdicts reconciled through a computed
    /// rounding-order residual, and which residual SIZES appeared. An instrument that reports what
    /// it absorbed — a mass-absorption regression is visible here rather than as silent green.
    absorbed: Vec<String>,
}

impl SweepReport {
    fn of(outcomes: &[Outcome]) -> SweepReport {
        let mut r = SweepReport::default();
        for Outcome { name, check } in outcomes {
            if check["refused"] == serde_json::json!(true) {
                r.refused.push(name.clone());
                continue;
            }
            r.admitted += 1;
            if check["reproduction_ok"] != serde_json::json!(true) {
                r.reproduction_failures.push(name.clone());
            }
            let mut lines: Vec<String> = Vec::new();
            for v in check["verdicts"].as_array().expect("verdicts is an array") {
                // ★ The row is described from its OWN `oracles` map, so a row compared against a
                //   third engine would be reported correctly with no edit here (the D1 lesson,
                //   applied to the reporter as well as the emitter).
                let who = serde_json::to_string(&v["oracles"]).unwrap_or_default();
                if v["reconciled"] != serde_json::json!(true) {
                    lines.push(format!(
                        "{} ({}): on_paper={} internal={} oracles={} class={}",
                        v["line"].as_str().unwrap_or("?"),
                        v["label"].as_str().unwrap_or("?"),
                        v["on_paper"].as_str().unwrap_or("<absent>"),
                        v["internal"].as_str().unwrap_or("?"),
                        who,
                        v["class"].as_str().unwrap_or("?"),
                    ));
                } else if let Some(res) = v["rounding_order_residual"].as_str() {
                    r.absorbed.push(format!(
                        "{name}: {} residual={res} oracles={who}",
                        v["line"].as_str().unwrap_or("?")
                    ));
                }
            }
            if !lines.is_empty() {
                r.divergences.push((name.clone(), lines));
            }
        }
        r
    }

    /// Every problem this sweep found, in one message — or `None` when the sweep is green.
    fn failure_message(&self) -> Option<String> {
        let mut msg = String::new();
        if !self.divergences.is_empty() {
            msg.push_str(&format!(
                "{} of {} admitted households DIVERGE (every one listed — the sweep no longer stops \
                 at the first):\n",
                self.divergences.len(),
                self.admitted
            ));
            for (name, lines) in &self.divergences {
                msg.push_str(&format!("  {name}\n"));
                for l in lines {
                    msg.push_str(&format!("      {l}\n"));
                }
            }
            msg.push_str(
                "  ★ Read the roster for a SHARED MECHANISM before adjudicating: one lawful rounding \
                 residual on many households looks nothing like a wrong figure on a few.\n",
            );
        }
        if !self.reproduction_failures.is_empty() {
            msg.push_str(&format!(
                "table_l16(btctax operands) failed to reproduce btctax's own regular tax on: {:?}\n",
                self.reproduction_failures
            ));
        }
        // ★ T5 — a NUMERIC FLOOR, not a vibe. T3 un-refused the screen-tripping zero-AMT population,
        // so the previously-excluded anchor joins the sweep: the floor is 11. If a future change
        // re-refuses a household this reds, instead of the count quietly sagging.
        if self.admitted < 11 {
            msg.push_str(&format!(
                "T3 admitted the former AMT-screen anchor, so at least 11 households sweep; got {}\n",
                self.admitted
            ));
        }
        if self.refused != EXPECTED_REFUSED {
            msg.push_str(&format!(
                "NOTHING should be refused: btctax now computes Form 6251 instead of refusing on the \
                 screen. A name here means that household genuinely owes AMT or must attach the form — \
                 adjudicate it deliberately, don't paper over it. Got {:?}\n",
                self.refused
            ));
        }
        (!msg.is_empty()).then_some(msg)
    }
}

/// Drive `--check` over `households` and assert every ADMITTED one reconciles on every compared line
/// (the golden matrix is green by construction). Since T3 the former AMT-screen anchor is ADMITTED, so
/// the assertion is on the reconciliation of every admitted household, not on a refusal roster.
fn sweep_check_reconciliation(households: &[serde_json::Value]) {
    let report = SweepReport::of(&check_all(households));
    // The absorbed-row census is printed even on a green run: a computed excuse that silently grew to
    // cover half the corpus is a finding, and it is invisible if only failures are reported.
    if !report.absorbed.is_empty() {
        println!(
            "[sweep] {} verdict rows reconciled through a computed rounding-order residual:\n  {}",
            report.absorbed.len(),
            report.absorbed.join("\n  ")
        );
    }
    if let Some(msg) = report.failure_message() {
        panic!("{msg}");
    }
}

/// ★ The make-check sweep: the 12 hand-audited ANCHORS + the 2 §5.1 PINNED cells (the non-`ca_` names).
/// Together they exercise EVERY reconciliation category `--check` implements — the L16 methodology class
/// (Table anchors), BOTH per-oracle provenance classes (the pinned cells), the C1 cross-foots (L24, SE
/// L12, 8959 L18), NIIT, the SALT cap and the deeper lines. The generated covering array is swept
/// differentially in `make check` by the sharded `golden_packet` (and by the `#[ignore]` twin below); a
/// serial subprocess-per-household loop over all ~104 here would blow the `make check` budget (§8).
#[test]
fn check_mode_reconciles_every_line_of_the_anchors_and_pinned_cells() {
    let matrix: serde_json::Value =
        serde_json::from_str(GOLDEN_RETURNS_JSON).expect("the golden matrix parses");
    let households: Vec<serde_json::Value> = matrix["households"]
        .as_array()
        .expect("households array")
        .iter()
        .filter(|h| !h["name"].as_str().unwrap_or("").starts_with("ca_"))
        .cloned()
        .collect();
    assert!(
        households.len() >= 12,
        "the matrix carries the twelve anchors + the two pinned cells"
    );
    sweep_check_reconciliation(&households);
}

/// The whole-corpus twin (§8) — `--check` reconciles every line of ALL 107 admitted households.
/// `#[ignore]`d out of `make check` because it drives one harness subprocess per household; the
/// `corpus-sweep` job in `.github/workflows/ci.yml` runs it with `-- --ignored` on every push.
///
/// ★★★ **FR-234 D4 — the `#[ignore]` reason used to promise *"run in CI"* and `ci.yml` contained zero
/// `--ignored` invocations, so this test had never executed anywhere.** It was the purest
/// green-and-blind instrument found in this repo: the *attribute itself* carried the false claim, and
/// the gate-level twin above filters out every `ca_` household, so nothing anywhere ran it. When it
/// was finally run it failed on 21 of 107. The reason string now names the job that runs it, and
/// `every_ignored_test_in_this_file_is_actually_run_by_a_ci_job` holds the two together.
#[test]
#[ignore = "full corpus (107), one harness subprocess per household — make-check sweeps the anchors + pinned cells; the corpus-sweep job in ci.yml runs this with --ignored"]
fn check_mode_reconciles_every_line_of_every_admitted_golden_household() {
    let matrix: serde_json::Value =
        serde_json::from_str(GOLDEN_RETURNS_JSON).expect("the golden matrix parses");
    let households = matrix["households"].as_array().expect("households array");
    assert!(households.len() >= 100, "the whole T11 corpus");
    sweep_check_reconciliation(households);
}

/// ★ T7-m2: the `--known-defect` pass-through has TEETH. A pinned §10 known-defect is authoritative for
/// its line — a `--check` run reconciles L16 iff btctax still prints the pinned wrong value — and a STALE
/// pin FAILS, forcing the entry's removal. To exercise a divergence on the green corpus we INJECT a wrong
/// oracle L16 into the household (both oracles perturbed off btctax's on-paper figure), so without a pin
/// the line diverges; the pin at btctax's ACTUAL printed value then suppresses it, and a pin at any other
/// value stays red.
#[test]
fn known_defect_pin_suppresses_an_l16_divergence_and_a_stale_pin_stays_red() {
    let mut household = raw_household("single_w2_only_standard");

    // btctax's ACTUAL on-paper L16 (whole dollars) — read it back once, never guessed.
    let baseline = run(&["--check"], &serde_json::to_string(&household).unwrap());
    let l16 = baseline["verdicts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["line"] == serde_json::json!("1040.line16"))
        .expect("an L16 verdict");
    let on_paper: i64 = l16["on_paper"].as_str().unwrap().parse().unwrap();
    assert_eq!(l16["reconciled"], serde_json::json!(true), "green baseline");

    // Inject a wrong oracle L16 on BOTH oracles (off btctax's figure) so the line genuinely diverges.
    let wrong = (on_paper + 500) as f64;
    household["expected_ots"]["income_tax_before_credits"] = serde_json::json!(wrong);
    household["expected_taxcalc"]["income_tax_before_credits"] = serde_json::json!(wrong);
    let hj = serde_json::to_string(&household).unwrap();

    let l16_of = |out: &serde_json::Value| -> serde_json::Value {
        out["verdicts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["line"] == serde_json::json!("1040.line16"))
            .cloned()
            .expect("an L16 verdict")
    };

    // No pin ⇒ the injected divergence surfaces (the anti-world guard, no class absorbs it above ceiling
    // — here below ceiling the methodology class would absorb a taxcalc-only dissent, but OTS is ALSO
    // wrong, so OTS's provenance conjunct-1 fails and the line stays red).
    let bare = run(&["--check"], &hj);
    assert_eq!(
        bare["all_reconciled"],
        serde_json::json!(false),
        "injected divergence must surface"
    );
    assert_eq!(l16_of(&bare)["reconciled"], serde_json::json!(false));

    // Pin at btctax's ACTUAL value ⇒ the known defect is suppressed (reconciled, labelled `known-defect`).
    let pinned = run(
        &[
            "--check",
            "--known-defect",
            &format!("1040.line16={on_paper}@FU-SMOKE"),
        ],
        &hj,
    );
    assert_eq!(
        l16_of(&pinned)["reconciled"],
        serde_json::json!(true),
        "the pin holds"
    );
    assert_eq!(l16_of(&pinned)["class"], serde_json::json!("known-defect"));

    // STALE pin (btctax's value is not what was pinned) ⇒ stays red, forcing the entry's removal.
    let stale = run(
        &[
            "--check",
            "--known-defect",
            &format!("1040.line16={}@FU-SMOKE", on_paper + 1),
        ],
        &hj,
    );
    assert_eq!(
        l16_of(&stale)["reconciled"],
        serde_json::json!(false),
        "a stale pin must fail"
    );
}

/// T7-m2 (untested-guard): `--known-defect` rejects a non-L16 line and every malformed spec with a
/// non-zero exit (before any stdin is consumed), so a typo can never silently disable a pin. Only L16
/// is a class/`stacking_ok` line; a non-L16 pin belongs in the golden test at promotion.
#[test]
fn known_defect_rejects_non_l16_and_malformed_specs() {
    for bad in [
        "1040.line24=5@FU-X",          // not the class/stacking line
        "1040.line16=notanumber@FU-X", // value is not a whole-dollar integer
        "1040.line16=5",               // missing @<fu-id>
        "1040.line16=5@",              // empty fu-id
        "garbage",                     // no `=`
    ] {
        let out = Command::new(HARNESS)
            .args(["--check", "--known-defect", bad])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the harness spawns")
            .wait_with_output()
            .expect("the harness runs");
        assert_eq!(
            out.status.code(),
            Some(2),
            "malformed --known-defect {bad:?} must exit 2"
        );
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(
            err.contains("known-defect"),
            "stderr must name the flag for {bad:?}: {err}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────────────────────────
// FR-234 — B1 "seen-red-once" for the four instrument defects. Each of these plants the exact defect
// its fix exists to catch; each was observed RED before the fix and green after. `design/HARNESS.md`
// B1: no checker exists until it has been watched discriminating.
// ─────────────────────────────────────────────────────────────────────────────────────────────────

/// Every verdict row the harness emits for one household, driven once.
fn verdicts_of(household: &serde_json::Value) -> Vec<serde_json::Value> {
    let out = run(&["--check"], &serde_json::to_string(household).unwrap());
    assert_eq!(
        out["refused"],
        serde_json::json!(false),
        "the fixture household must be admitted, or the test measures nothing"
    );
    out["verdicts"]
        .as_array()
        .expect("verdicts is an array")
        .clone()
}

/// The one verdict row for `line` whose `engine` is `engine`.
fn row(verdicts: &[serde_json::Value], line: &str, engine: &str) -> serde_json::Value {
    let mut hits = verdicts.iter().filter(|v| {
        v["line"] == serde_json::json!(line) && v["engine"] == serde_json::json!(engine)
    });
    let hit = hits
        .next()
        .unwrap_or_else(|| panic!("no {line} row compared against {engine}"))
        .clone();
    assert!(
        hits.next().is_none(),
        "{line}/{engine} must identify exactly one row"
    );
    hit
}

/// ★★★ **FR-234 D1 — B1: every single-engine verdict publishes its figure under the engine that
/// actually spoke.**
///
/// `verdict_engine` took the engine as an argument and then routed the figure into the `ots` column
/// **unconditionally**, so a taxcalc-compared row emitted `"ots": <taxcalc's number>, "taxcalc":
/// null, "engine": "taxcalc"` — across all 14 single-engine call sites. It is a defect in what the
/// instrument *claims to have done*, and it is not theoretical: it misled this cycle's controller
/// into concluding OTS was the outlier on Schedule SE L12 when OTS agreed with btctax to the dollar.
///
/// ★ **The assertion is DERIVED over every row**, not written against the one that bit: the engine
///   column set comes from the `oracles` maps the run actually produced, and for each row every
///   engine present must match its column while every engine absent must leave a `null`. A new
///   compared line is covered the day it is added.
///
/// ★ **The fixture is half the checker (B1a).** A corpus where the two engines always agree would
///   pass this vacuously even fully broken, so the household is one where they DISAGREE on a
///   single-engine line, and that disagreement is asserted before anything else is.
#[test]
fn every_single_engine_verdict_publishes_its_figure_under_the_engine_that_spoke() {
    let verdicts = verdicts_of(&raw_household("ca_A_single_int-none_se-over_ltcg-0_qd-0"));

    // Non-vacuity: OTS and taxcalc must genuinely differ on a single-engine line, so a mis-routed
    // figure is detectable by VALUE and not only by nullness.
    let se_ots = row(&verdicts, "schedule_se.line12", "OTS");
    let se_tc = row(&verdicts, "schedule_se.line12", "taxcalc");
    assert_ne!(
        se_ots["oracles"]["OTS"], se_tc["oracles"]["taxcalc"],
        "this fixture exists because the two engines disagree on Sch SE L12; if they now agree the \
         test can no longer see a mis-routed figure and needs a new household"
    );

    // The pre-fix output, verbatim, was:
    //   "label": "Sch SE L12 (SE tax) [taxcalc]", "ots": "16955", "taxcalc": null, "engine": "taxcalc"
    // i.e. taxcalc's 16955 published as OTS's opinion, with OTS's actual 16956 nowhere on the row.
    assert_eq!(
        se_tc["ots"],
        serde_json::Value::Null,
        "a taxcalc-compared row must leave the OTS column EMPTY — publishing taxcalc's figure there \
         is what made OTS look like the dissenter: {se_tc}"
    );
    assert_eq!(
        se_tc["taxcalc"], se_tc["oracles"]["taxcalc"],
        "a taxcalc-compared row's figure belongs in the taxcalc column: {se_tc}"
    );

    // ── The derived invariant, over EVERY row. ───────────────────────────────────────────────────
    // The engine set comes from the run's own `oracles` maps; the legacy column name is the engine
    // name lowercased (`OTS` -> `ots`), so no engine list is typed here either.
    let engines: std::collections::BTreeSet<String> = verdicts
        .iter()
        .flat_map(|v| {
            v["oracles"]
                .as_object()
                .expect("every verdict carries an oracles map")
                .keys()
                .cloned()
                .collect::<Vec<String>>()
        })
        .collect();
    assert!(
        engines.len() >= 2,
        "the run must exercise more than one engine; got {engines:?}"
    );
    for v in &verdicts {
        let oracles = v["oracles"].as_object().expect("an oracles map");
        for engine in &engines {
            let column = &v[engine.to_lowercase()];
            match oracles.get(engine) {
                Some(figure) => assert_eq!(
                    column,
                    figure,
                    "{}: {engine} spoke, so the {} column must carry ITS figure, not another \
                     engine's: {v}",
                    v["line"],
                    engine.to_lowercase()
                ),
                None => assert_eq!(
                    column,
                    &serde_json::Value::Null,
                    "{}: {engine} did not speak about this line, so its column must be null — a \
                     figure there is another engine's opinion wearing {engine}'s name: {v}",
                    v["line"]
                ),
            }
        }
        // A single-engine row says which engine in `engine`, and that must be the only witness.
        if let Some(engine) = v["engine"].as_str() {
            assert_eq!(
                oracles.keys().collect::<Vec<_>>(),
                vec![engine],
                "{}: `engine` claims {engine}, so exactly that engine must appear in `oracles`: {v}",
                v["line"]
            );
        }
    }
}

/// ★★★ **FR-234 D3 — B1: the rounding-order residual absorbs ITS OWN EXACT SIZE and nothing else.**
///
/// The lawful mechanism: Schedule SE L12 is *"Add lines 10 and 11"* over the printed BOXES (the IRS
/// whole-dollar rule — *"if you do round to whole dollars, you must round all amounts"*), so btctax
/// files `Σ round(leg)`; taxcalc publishes only the exact total `setax`, so its figure is
/// `round(Σ leg)`. On this household those are 16956 and 16955 and **both are right**.
///
/// `CLAUDE.md` forbids the two shortcuts that would also turn the corpus green — a $1 tolerance, and
/// a list of household names — so what is asserted here is that the excuse is *computed*: exactly
/// one dollar is absorbed on a leg-bearing line, and every other shape still fails.
///
/// ★ **Per FR-235 the plants are NOT phrased in the checker's own vocabulary.** The checker tests
///   `paper == Σround(legs) && target == roundΣ(legs)`; the plants perturb an *oracle's baked exact
///   total* (a) by a large amount, (b) by a small amount of the wrong sign, and (c) by exactly the
///   absorbable size but on a line with **no leg structure at all** — which is the case a $1
///   tolerance would wave through and the mechanism must not.
#[test]
fn the_rounding_order_residual_absorbs_only_its_own_exact_size() {
    let name = "ca_A_single_int-none_se-over_ltcg-0_qd-0";
    let base = raw_household(name);

    // ── (a) The real case reconciles, SAYS WHY, and names the size. ──────────────────────────────
    //
    // ★ These three figures are PINNED LITERALS, and deliberately so. They were adjudicated against
    //   the form, not read off this harness: `f1040sse--2024.txt:45-47` makes L10 and L11 entry lines
    //   and L12 *"Self-employment tax. Add lines 10 and 11"*, and the IRS whole-dollar rule
    //   (`f4868--2024.txt:217-223`) puts whole dollars in the L10/L11 boxes — so the boxes hold
    //   13742 and 3214 and the filed L12 is 16956, while taxcalc's `round(16955.46)` is 16955.
    //   Deriving the expectation from the harness instead would be FR-230's tautology: a kill whose
    //   expectation comes from the thing it mutates measures nothing.
    const PAPER: i64 = 16956; // btctax's filed L12 = Σ round(leg)
    const TAXCALC: i64 = 16955; // taxcalc's round(Σ leg)
    const RESIDUAL: i64 = PAPER - TAXCALC; // the only gap this mechanism may absorb: $1

    let se = row(&verdicts_of(&base), "schedule_se.line12", "taxcalc");
    assert_eq!(
        se["on_paper"],
        serde_json::json!(PAPER.to_string()),
        "the adjudicated filed L12 for {name} is {PAPER}; if the paper moved, re-adjudicate against \
         the form before touching this test: {se}"
    );
    assert_eq!(
        se["oracles"]["taxcalc"],
        serde_json::json!(TAXCALC.to_string()),
        "the adjudicated taxcalc figure for {name} is {TAXCALC}: {se}"
    );
    assert_eq!(
        se["reconciled"],
        serde_json::json!(true),
        "the sum-round/round-sum residual is lawful and must reconcile: {se}"
    );
    assert_eq!(
        se["class"],
        serde_json::json!("methodology-rounding-order"),
        "reconciling is not enough — the row must name the MECHANISM that absorbed it: {se}"
    );
    assert_eq!(
        se["rounding_order_residual"],
        serde_json::json!(RESIDUAL.to_string()),
        "and the residual's exact SIZE, so a divergence of the wrong shape is unexpected even on a \
         line expected to diverge: {se}"
    );

    // ── (b) Every OTHER size still fails, and the boundary is exact. ──────────────────────────────
    //
    // Perturbing taxcalc's baked exact total by a whole-dollar `d` moves its figure to
    // `TAXCALC + d`, so the mechanism says the row reconciles iff that lands on `TAXCALC` (d = 0,
    // unperturbed) or on `PAPER` (d = RESIDUAL, exact agreement — which must be labelled
    // `agree-taxcalc`, NOT absorbed). The expectation below is computed from that statement and the
    // adjudicated constants above, never from the harness's own answer.
    for d in [-500_i64, -2, -1, 1, 2, 500] {
        let mut planted = base.clone();
        let se_tax = planted["expected_taxcalc"]["se_tax"].as_f64().unwrap();
        planted["expected_taxcalc"]["se_tax"] = serde_json::json!(se_tax + d as f64);
        let got = row(&verdicts_of(&planted), "schedule_se.line12", "taxcalc");
        assert_eq!(
            got["oracles"]["taxcalc"],
            serde_json::json!((TAXCALC + d).to_string()),
            "a whole-dollar perturbation must move taxcalc's figure by exactly that much: {got}"
        );
        let expected_class = if d == RESIDUAL {
            "agree-taxcalc" // lands ON the paper: plain agreement, nothing absorbed
        } else {
            "diverge" // any other size is a real divergence, residual or not
        };
        assert_eq!(
            got["class"],
            serde_json::json!(expected_class),
            "taxcalc at {} against a filed {PAPER} must be {expected_class}: a {d:+} gap is not \
             this line's ${RESIDUAL} rounding-order residual, and absorbing it would turn the \
             mechanism into a tolerance: {got}",
            TAXCALC + d
        );
        assert_eq!(
            got["reconciled"],
            serde_json::json!(d == RESIDUAL),
            "reconciled must follow the mechanism, not the size of the gap: {got}"
        );
    }

    // (c) ★ The same $1 gap on a line with NO leg structure must still fail. Form 8960 L17 is
    //     `round_dollar` of one figure, not a sum of printed legs, so no residual is defined there
    //     and the comparison stays strict. This is the case that separates a computed mechanism
    //     from a $1 tolerance.
    let mut niit = raw_household("mfj_high_income_niit_and_addl_medicare");
    let baseline_niit = row(&verdicts_of(&niit), "8960.line17", "taxcalc");
    assert_eq!(
        baseline_niit["reconciled"],
        serde_json::json!(true),
        "green baseline on the NIIT line"
    );
    let n = niit["expected_taxcalc"]["niit"].as_f64().unwrap();
    niit["expected_taxcalc"]["niit"] = serde_json::json!(n + 1.0);
    let planted_niit = row(&verdicts_of(&niit), "8960.line17", "taxcalc");
    assert_eq!(
        planted_niit["reconciled"],
        serde_json::json!(false),
        "Form 8960 L17 sums no printed legs, so a $1 gap there is a real divergence and must fail: \
         {planted_niit}"
    );
}

/// ★★★ **FR-234 D2 — B1: the sweep names EVERY divergent household, not just the first.**
///
/// The corpus sweep's `assert_eq!` lived inside its per-household loop, so it reported one household
/// and stopped. Measured: the pre-fix run died 5.05s in, on household 1 of 107, while 21 were red —
/// and the roster is the whole diagnosis, because 21 households sharing one $1 mechanism and 21
/// households with a wrong figure look identical when you can only see the first one.
///
/// ★ The plant is TWO genuine divergences on DIFFERENT lines and different engines (an L16 injected
///   on both oracles; a taxcalc-only Schedule SE total), so the roster is proven to collect across
///   households *and* across line kinds — not to repeat one shape twice. Neither perturbation is
///   phrased in the aggregator's own vocabulary (FR-235): they are oracle-figure injections upstream
///   of the harness, and the aggregator never looks at oracle figures at all.
#[test]
fn the_sweep_roster_names_every_divergent_household_not_just_the_first() {
    // Plant 1 — a real L16 divergence, both oracles perturbed off btctax's figure (the same
    // injection `known_defect_pin_suppresses_an_l16_divergence_and_a_stale_pin_stays_red` uses).
    let mut first = raw_household("single_w2_only_standard");
    let bumped = first["expected_ots"]["income_tax_before_credits"]
        .as_f64()
        .unwrap()
        + 500.0;
    first["expected_ots"]["income_tax_before_credits"] = serde_json::json!(bumped);
    first["expected_taxcalc"]["income_tax_before_credits"] = serde_json::json!(bumped);

    // Plant 2 — a DIFFERENT line, a DIFFERENT engine: taxcalc's Schedule SE total, off by a size no
    // rounding-order residual can explain.
    let mut second = raw_household("ca_A_single_int-none_se-over_ltcg-0_qd-0");
    let se_tax = second["expected_taxcalc"]["se_tax"].as_f64().unwrap();
    second["expected_taxcalc"]["se_tax"] = serde_json::json!(se_tax + 777.0);

    // A clean household between them, so "collects all" is not "collects everything blindly".
    let clean = raw_household("mfj_two_w2_standard");

    let outcomes = check_all(&[first, clean, second]);
    let report = SweepReport::of(&outcomes);

    // ★ The roster assertion comes FIRST deliberately: a short-circuiting sweep also truncates the
    //   admitted COUNT, and "3 != 1" is a far less useful red than the named-households one.
    let named: Vec<&str> = report
        .divergences
        .iter()
        .map(|(name, _)| name.as_str())
        .collect();
    assert_eq!(
        named,
        vec![
            "single_w2_only_standard",
            "ca_A_single_int-none_se-over_ltcg-0_qd-0"
        ],
        "BOTH divergent households must be in the roster, and the clean one must not. Reporting \
         only the first is the D2 defect: {named:?}"
    );
    assert_eq!(
        report.admitted, 3,
        "all three plants must be admitted, and all three must be COUNTED — a sweep that stops \
         early undercounts as well as under-reports"
    );
    // The roster must carry the actual diverging LINES, not merely the household names — otherwise it
    // still cannot show a shared mechanism.
    let (_, first_lines) = &report.divergences[0];
    let (_, second_lines) = &report.divergences[1];
    assert!(
        first_lines.iter().any(|l| l.contains("1040.line16")),
        "the first household's roster must name its diverging line: {first_lines:?}"
    );
    assert!(
        second_lines
            .iter()
            .any(|l| l.contains("schedule_se.line12")),
        "the second household's roster must name ITS line, which is a different one: {second_lines:?}"
    );
    let msg = report
        .failure_message()
        .expect("a sweep with two divergences must fail");
    for name in &named {
        assert!(
            msg.contains(name),
            "the panic message is the only thing an operator sees, so it must name {name}: {msg}"
        );
    }
}

/// ★★★ **FR-234 D4 — B1: every ignored test in this file is actually RUN by a CI job.**
///
/// The whole-corpus reconciliation carried an ignore reason promising *"run in CI / on demand"* while
/// `ci.yml` contained **zero** `--ignored` invocations, so it had never executed anywhere — and the
/// moment it was run it failed on 21 of 107 households. The attribute *itself* was the false claim,
/// which is why a prose fix ("say what is true") is not enough on its own: nothing would stop the
/// next reason string from drifting.
///
/// ★ **The ignored set is DERIVED from this file's own source**, never a hand-written list of test
///   names, so a new ignored test with no runner reds here (`CLAUDE.md` — "derive the list, or make
///   the compiler hold it"). The join key is the CI job's name, required in both places.
///
/// ★ **What this does NOT cover, stated rather than left silent:** it proves the workflow *invokes*
///   the ignored tests of this target, not that GitHub ran the job or that the job is a required
///   check (promotion is a repository-settings action). It cannot: a test cannot observe CI.
#[test]
fn every_ignored_test_in_this_file_is_actually_run_by_a_ci_job() {
    const JOB: &str = "corpus-sweep";
    // Built in two pieces so this checker's own source never contains the byte sequence it scans for.
    let needle = concat!("#[", "ignore");

    let src = include_str!("smoke.rs");
    let mut ignored: Vec<(String, String)> = Vec::new(); // (test fn name, ignore reason)
    let lines: Vec<&str> = src.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if !line.trim_start().starts_with(needle) {
            continue;
        }
        let reason = line.to_string();
        let name = lines[i + 1..]
            .iter()
            .find_map(|l| l.trim_start().strip_prefix("fn "))
            .and_then(|rest| rest.split('(').next())
            .unwrap_or_else(|| panic!("an ignore attribute with no following fn: {line}"))
            .to_string();
        ignored.push((name, reason));
    }
    assert!(
        !ignored.is_empty(),
        "this checker must have something to check — if the last ignored test was deleted, delete \
         this test too rather than leaving it green over an empty set"
    );

    let ci = include_str!("../../../.github/workflows/ci.yml");
    let invocation = "cargo test -p btctax-oracle-harness --test smoke --locked -- --ignored";
    assert!(
        ci.contains(&format!("{JOB}:")),
        "ci.yml must define the {JOB} job that runs this target's ignored tests"
    );
    assert!(
        ci.contains(invocation),
        "ci.yml must actually invoke the ignored tests of this target ({invocation:?}) — an ignore \
         reason promising a runner, with no runner, is how 21 red households went unnoticed"
    );
    for (name, reason) in &ignored {
        assert!(
            reason.contains(JOB),
            "{name}'s ignore reason must name the CI job that runs it ({JOB}), so the promise and \
             the runner cannot drift apart: {reason}"
        );
    }
}
