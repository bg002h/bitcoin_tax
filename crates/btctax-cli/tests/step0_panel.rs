//! ★★★ **R9 / T6 — STEP 0: the exchange seam's status panel, and the Digital Assets answer's life
//! outside `btctax-core`.**
//!
//! The five-row DA table lives in `btctax-core/tests/kat_digital_asset_question.rs`, where the screen
//! and the printed chain are. What lives HERE is everything that needs a real vault:
//!
//! - the **standing-order fixture pair** — a 2026 Coinbase disposal with no scoped election FIRES the
//!   Notice 2026-20 §4.02(2) row, and one effective the day before is SILENT;
//! - the **unnamed-venue fixture** — a venue in the year's Form 8949 rows and absent from the stored
//!   Form 1099-DA answers is NAMED in the panel and in the commit modal's listing;
//! - **`digital_asset_activity = None` blocks commit** and is listed by `interview_state`;
//! - **the answer survives an `income import` round-trip**;
//! - **T4b's opener seeds it `None`** — a `PerYear` gate is re-asked blank, never carried.
//!
//! All fixtures are SYNTHETIC; no real user data is read.

use btctax_cli::step0::{step0_panel, Step0Panel};
use btctax_cli::{cmd, return_inputs, Session};
use btctax_core::conventions::Sat;
use btctax_core::event::*;
use btctax_core::identity::*;
use btctax_core::tax::return_inputs::{Person, ReturnInputs};
use btctax_core::FilingStatus;
use btctax_store::Passphrase;
use rust_decimal_macros::dec;
use std::path::{Path, PathBuf};
use time::macros::{date, datetime, offset};

const ONE_BTC: Sat = 100_000_000;
/// The year the standing-order rows are about: Notice 2026-20's relief period.
const YEAR: i32 = 2026;

fn pp() -> Passphrase {
    Passphrase::new("pw".into())
}

fn coinbase() -> WalletId {
    WalletId::Exchange {
        provider: "coinbase".into(),
        account: "default".into(),
    }
}

fn river() -> WalletId {
    WalletId::Exchange {
        provider: "river".into(),
        account: "default".into(),
    }
}

fn imp(
    source: Source,
    r: &str,
    ts: time::OffsetDateTime,
    w: WalletId,
    p: EventPayload,
) -> LedgerEvent {
    LedgerEvent {
        id: EventId::import(source, SourceRef::new(r)),
        utc_timestamp: ts,
        original_tz: offset!(+00:00),
        wallet: Some(w),
        payload: p,
    }
}

fn buy(source: Source, r: &str, ts: time::OffsetDateTime, w: WalletId, cost: &str) -> LedgerEvent {
    imp(
        source,
        r,
        ts,
        w,
        EventPayload::Acquire(Acquire {
            sat: ONE_BTC,
            usd_cost: cost.parse().unwrap(),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    )
}

fn sell(
    source: Source,
    r: &str,
    ts: time::OffsetDateTime,
    w: WalletId,
    proceeds: &str,
) -> LedgerEvent {
    imp(
        source,
        r,
        ts,
        w,
        EventPayload::Dispose(Dispose {
            sat: ONE_BTC,
            kind: DisposeKind::Sell,
            usd_proceeds: proceeds.parse().unwrap(),
            fee_usd: dec!(0),
        }),
    )
}

/// A dated `MethodElection` — a STANDING ORDER under Notice 2026-20 §4.02(2) — scoped to `wallet`,
/// **MADE** at `made` and effective from `effective`.
///
/// ★ It is appended as a DECISION (`append_decision`), which is the only way a `MethodElection` can
///   reach the ledger: `append_import_batch` refuses a non-imported payload. The made-INSTANT is
///   load-bearing — `method_election_is_forward` refuses a back-dated election, and §4.02(2)'s whole
///   test is *"entered into the books BEFORE the units are sold"*.
fn record_election(
    vault: &Path,
    made: time::OffsetDateTime,
    effective: btctax_core::TaxDate,
    wallet: Option<WalletId>,
) {
    let mut s = Session::open(vault, &pp()).unwrap();
    btctax_core::persistence::append_decision(
        s.conn(),
        EventPayload::MethodElection(MethodElection {
            effective_from: effective,
            method: btctax_core::LotMethod::Hifo,
            wallet,
        }),
        made,
        offset!(+00:00),
        None,
    )
    .unwrap();
    s.save().unwrap();
}

fn vault_with(events: &[LedgerEvent]) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    let mut s = Session::open(&vault, &pp()).unwrap();
    btctax_core::persistence::append_import_batch(s.conn(), events).unwrap();
    s.save().unwrap();
    (dir, vault)
}

/// The panel this vault produces for `YEAR`, with the year's stored return if there is one.
fn panel_of(vault: &Path, year: i32) -> Step0Panel {
    let s = Session::open(vault, &pp()).unwrap();
    let (state, _) = s.project().unwrap();
    let events = btctax_core::persistence::load_all(s.conn()).unwrap();
    let ri = return_inputs::get(s.conn(), year).unwrap();
    step0_panel(&state, &events, ri.as_ref(), year)
}

/// ★★★ **THE STANDING-ORDER FIXTURE PAIR** (R9's kill; J-9, J-10).
///
/// Notice 2026-20 §4.02(2) makes a standing order an adequate identification only when it is
/// *"entered into the taxpayer's books and records **before** the units covered by the order are
/// sold"*. So the panel's row is about ONE moment: the filer's first custodial disposition of the
/// year.
///
/// **Both halves in one test, because either alone is satisfiable by a broken implementation:** a
/// panel that always warned would pass the first, and one that never warned would pass the second.
#[test]
fn the_standing_order_row_fires_with_no_election_and_is_silent_with_one_effective_the_day_before() {
    let sale = datetime!(2026-03-10 12:00:00 UTC);

    // ── (a) NO scoped election ⇒ the row FIRES, names the venue and the date, and states the
    //    consequence and the exit. ────────────────────────────────────────────────────────────────
    let (_d, vault) = vault_with(&[
        buy(
            Source::Coinbase,
            "BUY-1",
            datetime!(2026-01-05 12:00:00 UTC),
            coinbase(),
            "50000.00",
        ),
        sell(Source::Coinbase, "SELL-1", sale, coinbase(), "60000.00"),
    ]);
    let p = panel_of(&vault, YEAR);
    assert_eq!(
        p.standing_orders.len(),
        1,
        "one custodial venue with no standing order: {:?}",
        p.standing_orders
    );
    let row = &p.standing_orders[0];
    for needle in [
        "exchange:coinbase:default",
        "2026-03-10",
        "box 1g",
        "basis_differs",
        "select-lots",
    ] {
        assert!(
            row.what.contains(needle),
            "the row must name the venue, the date, and state the consequence with its exit \
             (§4.02(2), J-9/J-10); missing {needle:?} in: {}",
            row.what
        );
    }
    assert!(
        row.handoff.contains("--set-forward-method") && row.handoff.contains("forward only"),
        "the exit is a FORWARD election, and it must say so — an election can never be back-dated: \
         {}",
        row.handoff
    );

    // ── (b) the SAME ledger with an election effective the day before ⇒ SILENT. ──────────────────
    let (_d2, vault2) = vault_with(&[
        buy(
            Source::Coinbase,
            "BUY-1",
            datetime!(2026-01-05 12:00:00 UTC),
            coinbase(),
            "50000.00",
        ),
        sell(Source::Coinbase, "SELL-1", sale, coinbase(), "60000.00"),
    ]);
    record_election(
        &vault2,
        datetime!(2026-03-09 09:00:00 UTC),
        date!(2026 - 03 - 09),
        Some(coinbase()),
    );
    let p2 = panel_of(&vault2, YEAR);
    assert!(
        p2.standing_orders.is_empty(),
        "a standing order recorded the day BEFORE the sale IS the §4.02(2) identification — \
         warning here would contradict the engine's own `StandingOrder` verdict: {:?}",
        p2.standing_orders
    );

    // ── (c) …and the panel's verdict is the ENGINE's. `disposal_compliance` runs the same shared
    //    resolver, so the two may never disagree about whether an order governs. ─────────────────
    let s = Session::open(&vault2, &pp()).unwrap();
    let (state, _) = s.project().unwrap();
    let events = btctax_core::persistence::load_all(s.conn()).unwrap();
    assert!(
        btctax_core::disposal_compliance(&events, &state)
            .iter()
            .any(|c| matches!(
                c.status,
                btctax_core::ComplianceStatus::StandingOrder { .. }
            )),
        "premise: the engine itself calls this disposal `StandingOrder`, which is why the panel is \
         silent about it"
    );
}

/// ★★★ **THE UNNAMED-VENUE FIXTURE** (R9's kill; J-4, J-7).
///
/// A venue with a disposition on the year's Form 8949 and no Form 1099-DA answer on file is the
/// filer who forgot River. The panel NAMES it, and the same rows ride the commit modal's listing.
#[test]
fn a_venue_with_rows_and_no_broker_answer_is_named_and_an_answered_one_is_not() {
    let (_d, vault) = vault_with(&[
        buy(
            Source::Coinbase,
            "CB-BUY",
            datetime!(2026-01-05 12:00:00 UTC),
            coinbase(),
            "50000.00",
        ),
        sell(
            Source::Coinbase,
            "CB-SELL",
            datetime!(2026-03-10 12:00:00 UTC),
            coinbase(),
            "60000.00",
        ),
        buy(
            Source::River,
            "RV-BUY",
            datetime!(2026-01-06 12:00:00 UTC),
            river(),
            "40000.00",
        ),
        sell(
            Source::River,
            "RV-SELL",
            datetime!(2026-04-11 12:00:00 UTC),
            river(),
            "45000.00",
        ),
    ]);

    // Premise: BOTH venues have rows this year, so neither is missing for a mechanical reason.
    {
        let s = Session::open(&vault, &pp()).unwrap();
        let (state, _) = s.project().unwrap();
        let rows = btctax_core::form_8949(&state, YEAR);
        let census = btctax_core::forms::broker_key_census(&rows);
        assert_eq!(
            census.len(),
            2,
            "premise: two (provider, cohort) keys with rows: {census:?}"
        );
    }

    // ── Nothing answered yet ⇒ BOTH venues are named. ───────────────────────────────────────────
    let p = panel_of(&vault, YEAR);
    assert_eq!(p.venues.len(), 2, "{:?}", p.venues);
    for venue in ["coinbase", "river"] {
        assert!(
            p.venues
                .iter()
                .any(|v| v.what.contains(venue) && !v.handoff.is_empty()),
            "{venue} has rows and no answer, so it must be NAMED with an exit: {:?}",
            p.venues
        );
    }

    // ── Answer COINBASE only. River must still be named; Coinbase must not. ─────────────────────
    {
        let mut s = Session::open(&vault, &pp()).unwrap();
        let mut ri = ReturnInputs {
            tax_year: YEAR,
            filing_status: FilingStatus::Single,
            ..Default::default()
        };
        ri.header.taxpayer = Person {
            first_name: "Pat".into(),
            last_name: "Roe".into(),
            ssn: "222-33-4444".into(),
            date_of_birth: Some(date!(1980 - 05 - 05)),
            ..Default::default()
        };
        btctax_core::tax::testonly::answer_all_live_declarations(&mut ri);
        ri.digital_asset_activity = Some(true);
        ri.broker_reporting.0.insert(
            "coinbase".into(),
            btctax_core::forms::CohortAnswers {
                covered: Some(btctax_core::forms::BrokerReported::BasisMatches),
                noncovered: Some(btctax_core::forms::BrokerReported::BasisMatches),
            },
        );
        return_inputs::set(s.conn(), YEAR, &ri).unwrap();
        s.save().unwrap();
    }
    let p = panel_of(&vault, YEAR);
    let named: Vec<&str> = p
        .venues
        .iter()
        .filter(|v| !v.handoff.is_empty())
        .map(|v| v.what.as_str())
        .collect();
    assert_eq!(
        named.len(),
        1,
        "exactly one venue is still unaccounted for: {named:?}"
    );
    assert!(
        named[0].starts_with("river"),
        "★ the filer who forgot RIVER is the whole of J-7: {named:?}"
    );
    assert!(
        p.venues
            .iter()
            .any(|v| v.what.starts_with("coinbase") && v.handoff.is_empty()),
        "…and the answered venue is listed as answered rather than dropped: {:?}",
        p.venues
    );
}

/// ★★★ **The blocker rows hand off to `reconcile`, and the exit is real** (R9: *"every row hands off
/// to the `reconcile` command that answers it"*).
#[test]
fn every_blocker_row_carries_a_reconcile_exit() {
    // An UNCLASSIFIED inbound: the ledger's own question, which only `reconcile` can answer.
    let (_d, vault) = vault_with(&[imp(
        Source::Coinbase,
        "IN-1",
        datetime!(2026-05-01 12:00:00 UTC),
        coinbase(),
        EventPayload::TransferIn(TransferIn {
            sat: ONE_BTC,
            src_addr: None,
            txid: None,
        }),
    )]);
    let p = panel_of(&vault, YEAR);
    assert!(
        !p.blockers.is_empty(),
        "premise: an unclassified inbound blocks"
    );
    for row in &p.blockers {
        assert!(
            !row.handoff.is_empty(),
            "a blocker row with no exit is a brick with better prose: {row:?}"
        );
    }
    assert!(
        p.blockers
            .iter()
            .any(|r| r.handoff.contains("btctax reconcile")),
        "the ledger's questions belong to `reconcile`: {:?}",
        p.blockers
    );
}

/// ★★★ **`digital_asset_activity = None` BLOCKS COMMIT, and the panel says so before the filer gets
/// there** (R12's list is where a class-(A) blank shows up while authoring).
#[test]
fn an_unanswered_digital_asset_question_blocks_and_is_listed_by_interview_state() {
    use btctax_core::tax::interview_state::interview_state;
    use btctax_core::tax::provenance::AnswerKey;
    use btctax_core::tax::questions::QuestionId;

    let mut ri = ReturnInputs {
        tax_year: 2024,
        filing_status: FilingStatus::Single,
        ..Default::default()
    };
    btctax_core::tax::testonly::answer_all_live_declarations(&mut ri);
    ri.digital_asset_activity = None;

    let st = interview_state(&ri);
    assert!(
        st.blocking
            .iter()
            .any(|b| b.item == AnswerKey::Question(QuestionId::DigitalAssetActivity)),
        "an unanswered ALWAYS-LIVE declaration is a blocking panel item: {:?}",
        st.blocking
            .iter()
            .map(|b| b.item.clone())
            .collect::<Vec<_>>()
    );
    assert!(!st.is_committable(), "…and the return is not committable");

    // …and the screen refuses it with its own reason, naming the exit.
    let refusal = btctax_core::tax::return_refuse::screen_inputs(
        &ri,
        &btctax_core::tax::testonly::ty2024_table(),
        &btctax_core::tax::testonly::ty2024_params(),
    )
    .expect("the unanswered question refuses");
    assert_eq!(
        refusal.reason,
        btctax_core::tax::return_refuse::RefuseReason::DigitalAssetActivityUnanswered
    );
    assert!(
        refusal.detail.contains("btctax income answer"),
        "the refusal names its exit: {}",
        refusal.detail
    );

    // Answering it — through the REGISTRY's own setter, the way a filer would — clears both.
    let q = btctax_core::tax::questions::FORM_QUESTIONS
        .iter()
        .find(|q| q.id == QuestionId::DigitalAssetActivity)
        .expect("the registry entry exists");
    (q.set)(&mut ri, false);
    let st = interview_state(&ri);
    assert!(
        !st.blocking
            .iter()
            .any(|b| b.item == AnswerKey::Question(QuestionId::DigitalAssetActivity)),
        "answering it through its own setter empties the item (the no-brick property)"
    );
}

/// ★★★ **The answer survives an `income import` round-trip** — it is `#[serde(default)]`, so an older
/// TOML loads with it UNANSWERED rather than failing or being assumed, and a TOML that states it
/// stores exactly what it said.
#[test]
fn the_digital_asset_answer_round_trips_through_income_import() {
    let (dir, vault) = vault_with(&[]);
    // ★ The tables come LAST: `digital_asset_activity` is a TOP-LEVEL key, and a line written after
    //   `[sch1]` would land inside it (which is exactly what the import's unknown-key check caught
    //   the first time this fixture was written — `sch1.digital_asset_activity`).
    let tables = "[header]\ncan_be_claimed_as_dependent_taxpayer = false\n\
                  taxpayer_died_during_year = false\n\
                  [sch1]\nhsa_activity = false\n";
    let base = "filing_status = \"Single\"\n\
                foreign_accounts = false\nforeign_trust = false\ndual_status_alien = false\n\
                has_income_exclusion = false\nother_out_of_scope_income = false\n\
                filing_form_4952 = false\n\
                w2_wages_without_w2 = false\ninterest_or_dividends_without_1099 = false\n\
                state_refund_without_1099g = false\n";

    // (1) STATED — the value is stored verbatim, both ways.
    for stated in [true, false] {
        let toml = dir.path().join(format!("ri-{stated}.toml"));
        std::fs::write(
            &toml,
            format!("{base}digital_asset_activity = {stated}\n{tables}"),
        )
        .unwrap();
        cmd::tax::import_return_inputs(&vault, &pp(), 2024, &toml, false, false).unwrap();
        let s = Session::open(&vault, &pp()).unwrap();
        let stored = return_inputs::get(s.conn(), 2024).unwrap().unwrap();
        assert_eq!(
            stored.digital_asset_activity,
            Some(stated),
            "the answer must round-trip through the store byte-for-byte"
        );
    }

    // (2) OMITTED — `#[serde(default)]`, so the import parses and the question stays UNANSWERED. A
    //     defaulted `Some(false)` here would be the exact laundering the class-(A) rule exists to
    //     end: a "no" nobody swore to, indistinguishable on the printed page from one they did.
    let toml = dir.path().join("ri-omitted.toml");
    std::fs::write(&toml, format!("{base}{tables}")).unwrap();
    cmd::tax::import_return_inputs(&vault, &pp(), 2024, &toml, false, false).unwrap();
    let s = Session::open(&vault, &pp()).unwrap();
    let stored = return_inputs::get(s.conn(), 2024).unwrap().unwrap();
    assert_eq!(
        stored.digital_asset_activity, None,
        "an omitted answer is UNANSWERED, never a defaulted \"no\""
    );
}

/// ★★★ **T4b's opener seeds it `None`.** `Durability::PerYear`: *"a prior-year answer must NEVER
/// silently satisfy this year's provenance"*, and the Digital Assets question asks *"at any time
/// during <year>"* — so last year's `Yes` is not testimony for this year.
#[test]
fn the_year_opener_seeds_the_digital_asset_answer_blank() {
    use btctax_cli::open_next_year::open_next_year;

    let (dir, vault) = vault_with(&[]);
    let _ = dir;
    {
        let mut ri = ReturnInputs {
            tax_year: 2024,
            filing_status: FilingStatus::Single,
            ..Default::default()
        };
        ri.header.taxpayer = Person {
            first_name: "Alex".into(),
            last_name: "Filer".into(),
            ssn: "123456789".into(),
            date_of_birth: Some(date!(1980 - 05 - 05)),
            ..Default::default()
        };
        btctax_core::tax::testonly::answer_all_live_declarations(&mut ri);
        // Year N answered it YES — the state that would be carried if anything carried.
        ri.digital_asset_activity = Some(true);
        let mut s = Session::open(&vault, &pp()).unwrap();
        return_inputs::set(s.conn(), 2024, &ri).unwrap();
        s.save().unwrap();
    }
    {
        let mut s = Session::open(&vault, &pp()).unwrap();
        open_next_year(&mut s, 2024, false).expect("the opener runs");
        s.save().unwrap();
    }
    let s = Session::open(&vault, &pp()).unwrap();
    let (loaded, _) = btctax_cli::input_form_store::load(s.conn(), 2025).unwrap();
    let btctax_cli::input_form_store::Loaded::Draft { ri: seeded, .. } = loaded else {
        panic!("the opener writes a DRAFT for year N+1");
    };
    assert_eq!(
        seeded.digital_asset_activity, None,
        "★★★ a `PerYear` gate is re-asked BLANK. Year N's `Yes` is an answer about year N, and \
         carrying it would be software answering a §6065 declaration for the filer"
    );
}

/// The keystroke script that answers every live question `no`, swept the way `income answer` itself
/// sweeps (a `No` on a census row makes a NEW question live mid-session, R3/T5).
fn answer_everything_no(ri: &ReturnInputs) -> Vec<u8> {
    use btctax_cli::cmd::answer::{live_questions, Ask};
    let mut ri = ri.clone();
    let mut asked: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut script = String::new();
    for _ in 0..8 {
        let round: Vec<Ask> = live_questions(&ri)
            .into_iter()
            .filter(|a| match a {
                Ask::Declaration(q) => !asked.contains(&format!("d{:?}", q.id)),
                Ask::Skippable(sk) => !asked.contains(&format!("s{:?}", sk.id)),
            })
            .collect();
        if round.is_empty() {
            break;
        }
        for a in round {
            match a {
                Ask::Declaration(q) => {
                    asked.insert(format!("d{:?}", q.id));
                    script.push_str("n\n");
                    (q.set)(&mut ri, false);
                }
                Ask::Skippable(sk) => {
                    asked.insert(format!("s{:?}", sk.id));
                    script.push('\n');
                }
            }
        }
    }
    script.into_bytes()
}

/// ★★★ **`income answer` PRINTS STEP 0, BEFORE THE CENSUS** (R9: *"printed by `income answer` before
/// the census — document-first order stays, because the panel is STATUS, not a question"*).
///
/// The ORDER is the assertion, not merely the presence: the ledger's status is what a filer needs in
/// hand while they answer, and the first thing they are ASKED is still the shoebox.
#[test]
fn income_answer_prints_step_0_before_the_first_census_question() {
    let (_d, vault) = vault_with(&[
        buy(
            Source::River,
            "RV-BUY",
            datetime!(2026-01-06 12:00:00 UTC),
            river(),
            "40000.00",
        ),
        sell(
            Source::River,
            "RV-SELL",
            datetime!(2026-04-11 12:00:00 UTC),
            river(),
            "45000.00",
        ),
    ]);
    // A committed return for the year, so `income answer` has something to answer.
    let ri = {
        let mut ri = ReturnInputs {
            tax_year: YEAR,
            filing_status: FilingStatus::Single,
            ..Default::default()
        };
        ri.header.taxpayer = Person {
            first_name: "Pat".into(),
            last_name: "Roe".into(),
            ssn: "222-33-4444".into(),
            date_of_birth: Some(date!(1980 - 05 - 05)),
            ..Default::default()
        };
        let mut s = Session::open(&vault, &pp()).unwrap();
        return_inputs::set(s.conn(), YEAR, &ri.clone()).unwrap();
        s.save().unwrap();
        ri.tax_year = YEAR;
        ri
    };

    let script = answer_everything_no(&ri);
    let mut keystrokes: &[u8] = &script;
    let mut screen: Vec<u8> = Vec::new();
    cmd::answer::answer_return_inputs(
        &vault,
        &pp(),
        YEAR,
        date!(2026 - 09 - 01),
        &mut keystrokes,
        &mut screen,
        false,
    )
    .unwrap();
    let out = String::from_utf8(screen).unwrap();

    let step0_at = out
        .find("── Step 0: your ledger")
        .expect("`income answer` prints the Step 0 ledger panel");
    // The first thing ASKED is still the shoebox — the census's own first row.
    let first_question_at = out
        .find("Form W-2")
        .expect("the document census is still asked first");
    assert!(
        step0_at < first_question_at,
        "Step 0 is STATUS and comes before the questions; document-first order is untouched:\n{out}"
    );
    assert!(
        out.contains("river") && out.contains("NO Form 1099-DA answer"),
        "…and it names the venue with no answer: {out}"
    );
    assert!(
        out.contains("Notice 2026-20") || out.contains("STANDING ORDERS"),
        "…and the §4.02(2) standing-order section: {out}"
    );
    // ★★★ R9 — VENUE/ACCOUNT GRANULARITY IS DOCUMENTED, NOT ASKED. The sentence is printed beside
    //     the standing-order list, and NO question was added for it.
    assert!(
        out.contains("ONE account per venue") && out.contains("1012(c)(1)"),
        "the granularity note rides the standing-order list: {out}"
    );
}

/// ★★★ **R9 — venue/account granularity is DOCUMENTED, NOT ASKED**, and this is the half a printed
/// sentence cannot prove: **no registry question asks about accounts**.
///
/// The account segment is hardcoded `default` (`btctax-adapters/src/normalize.rs:63-69`), so a
/// question like *"how many accounts do you hold at Coinbase?"* would collect an answer nothing
/// reads — a stored value with no reader, which this codebase treats as a defect. The note states
/// the §1012(c)(1) consequence instead.
#[test]
fn no_registry_question_asks_about_venue_or_account_granularity() {
    use btctax_core::tax::questions::{FORM_QUESTIONS, SKIPPABLE_QUESTIONS};
    let prompts: Vec<String> = FORM_QUESTIONS
        .iter()
        .map(|q| q.prompt.to_ascii_lowercase())
        .chain(
            SKIPPABLE_QUESTIONS
                .iter()
                .map(|s| s.prompt.to_ascii_lowercase()),
        )
        .collect();
    for p in &prompts {
        assert!(
            !(p.contains("how many accounts") || p.contains("which account")),
            "R9: account granularity is documented, not asked — this prompt asks it: {p}"
        );
    }
    // …and the note that replaces the question exists and says what btctax models.
    let note = btctax_cli::step0::VENUE_GRANULARITY_NOTE;
    assert!(
        note.contains("exchange:<venue>:default") && note.contains("1012(c)(1)"),
        "the note must name the shape btctax models AND the rule it matters for: {note}"
    );
}
