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

/// The panel this vault produces for `year`, with the year's stored return if there is one and the
/// year's REAL Form 1099-DA regime — the same join `income answer` and the TUI make
/// (`year_readiness::regime_for`), so a fixture can never be shown a regime the product would not.
fn panel_of(vault: &Path, year: i32) -> Step0Panel {
    let s = Session::open(vault, &pp()).unwrap();
    let (state, _) = s.project().unwrap();
    let events = btctax_core::persistence::load_all(s.conn()).unwrap();
    let ri = return_inputs::get(s.conn(), year).unwrap();
    step0_panel(
        &state,
        &events,
        ri.as_ref(),
        year,
        btctax_cli::year_readiness::regime_for(year),
    )
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
///
/// ★★★ **(seam review M-2) THE BOUNDARY IS "ON OR BEFORE", NOT "BEFORE" — and part (d) says so.**
///     §4.02(2)'s own test is *"entered into the taxpayer's books and records **before** the units
///     covered by the order are sold"*, while §4.02(**1**) is the limb that says *"no later than the
///     date and time"*. `standing_order_in_force` inherits `resolve_election`'s `effective_from <=
///     date` on a DAY-granular `TaxDate`, so an order recorded the day OF the sale — hours AFTER it
///     — is treated as in force and the row is SILENT. That is measured below rather than claimed
///     either way, because the root is the pre-existing `<=` which also decides the FILED BASIS
///     through `disposal_compliance`, and that belongs to **FR-77** (the method-election track),
///     never to a panel that must mirror the engine.
#[test]
fn the_standing_order_row_fires_with_no_election_and_is_silent_with_one_effective_on_or_before_the_sale(
) {
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

    // ── (b) the SAME ledger with an election effective the day before ⇒ SILENT. (The name says
    //    "on or before" because part (d) below measures the day-OF case, which is also silent.) ───
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

    // ── (d) THE SAME-DAY CASE — the election MADE and effective on the day OF the sale, six hours
    //    AFTER it (M-2). Measured, not assumed: it falls SILENT, because the comparison is
    //    day-granular and `<=`. Nothing here is changed to make it so, and nothing here changes
    //    `resolve_election`.
    let (_d3, vault3) = vault_with(&[
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
        &vault3,
        datetime!(2026-03-10 18:00:00 UTC),
        date!(2026 - 03 - 10),
        Some(coinbase()),
    );
    assert!(
        panel_of(&vault3, YEAR).standing_orders.is_empty(),
        "★ FR-77 — an order recorded SIX HOURS AFTER the 12:00 sale is treated as in force: \
         `resolve_election` compares `effective_from <= disposed_at` on a day-granular `TaxDate`, \
         so the last day of §4.02(2)'s window fails OPEN. The panel mirrors the engine deliberately \
         (a panel that diverged would warn a filer the fold calls compliant), so the fix is the \
         engine's `<=` — which also decides the FILED BASIS — and it is FR-77's, not T6's."
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

/// ★★★ **(seam review I-1) THE VENUE ROW OBEYS THE YEAR'S FORM 1099-DA REGIME.**
///
/// The list shares `screen_broker_reporting`'s KEY derivation and used to drop its LIVENESS gate, so
/// on **TY2024 and TY2025 — the only two years this product serves** — every filer with an exchange
/// disposal was told a venue *"is not accounted for"* and handed to a Form 1099-DA block that is not
/// asked for the year and whose answer, if supplied, is REFUSED (`BrokerAnswerUnread`: *"testimony
/// it would discard is not kept"*). The committed fixture pair could not see it: both halves ran at
/// `const YEAR = 2026`, the one year the row is right for.
///
/// **THE ASSERTION IS AN ABSENCE, and it is made at both non-live years and its presence at the live
/// one** — either alone passes on a broken implementation (a panel that never names a venue passes
/// the first two; the pre-fold panel passes the third).
#[test]
fn the_venue_row_fires_only_on_a_year_whose_form_1099da_question_is_live() {
    for year in [2024, 2025, 2026] {
        let (_d, vault) = vault_with(&[
            buy(
                Source::Coinbase,
                "CB-BUY",
                datetime!(2024-01-05 12:00:00 UTC)
                    .replace_year(year)
                    .unwrap(),
                coinbase(),
                "50000.00",
            ),
            sell(
                Source::Coinbase,
                "CB-SELL",
                datetime!(2024-06-15 12:00:00 UTC)
                    .replace_year(year)
                    .unwrap(),
                coinbase(),
                "60000.00",
            ),
        ]);
        // Premise: the year HAS a key with rows, so an absent row is the gate and not a missing
        // fixture — and the regime is the one the product joins for the year, never a literal.
        let regime = btctax_cli::year_readiness::regime_for(year).expect("a bundled year record");
        {
            let s = Session::open(&vault, &pp()).unwrap();
            let (state, _) = s.project().unwrap();
            let rows = btctax_core::form_8949(&state, year);
            assert_eq!(
                btctax_core::forms::broker_key_census(&rows).len(),
                1,
                "premise: TY{year} has one (provider, cohort) key with rows"
            );
            assert_eq!(
                btctax_core::broker_question_is_live(&rows, regime),
                year == 2026,
                "premise: the Form 1099-DA question is live only from TY2026 (regime {regime:?})"
            );
        }
        let p = panel_of(&vault, year);
        let named: Vec<&str> = p
            .venues
            .iter()
            .filter(|v| !v.handoff.is_empty())
            .map(|v| v.what.as_str())
            .collect();
        if year == 2026 {
            assert_eq!(
                named.len(),
                1,
                "TY2026's question IS live and unanswered — the row must fire: {:?}",
                p.venues
            );
            assert!(named[0].contains("not accounted for"), "{named:?}");
        } else {
            assert!(
                named.is_empty(),
                "TY{year} asks no Form 1099-DA question, so no venue can be 'not accounted for' \
                 and no row may hand the filer to a block that would refuse them: {:?}",
                p.venues
            );
            // …and the panel is not SILENT about the venues either: it states the REGIME, the fact
            // that decides it, with no exit because there is no answer to give.
            assert_eq!(p.venues.len(), 1, "{:?}", p.venues);
            assert!(p.venues[0].handoff.is_empty(), "{:?}", p.venues[0]);
            for needle in ["Form 1099-DA regime", "coinbase"] {
                assert!(
                    p.venues[0].what.contains(needle),
                    "the regime row names {needle:?}: {}",
                    p.venues[0].what
                );
            }
        }
    }
}

/// ★★★ **(seam review M-1) THE STANDING-ORDER ROW CITES §4.02(2) ONLY INSIDE THE RELIEF PERIOD.**
///
/// Notice 2026-20 §3.03 defines the relief period as 2025-01-01 → 2026-12-31
/// (`legal/text/irs-guidance/Notice_2026-20.txt:299-301`), §4.01 makes §4.02's relief *"available
/// only with respect to units … sold, disposed of, or transferred during the relief period"*, and §5
/// forbids relying on it *"after the relief period ends"*. The row was filtered by tax year alone, so
/// it cited §4.02(2) at a 2024 sale (before the relief existed — and where btctax's own
/// `method_election_is_forward` refuses the election the row's exit names) and at a 2027 one.
///
/// The SUBSTANCE — no dated election ⇒ the broker's default — survives on every year, so this
/// asserts the row is still THERE and that the authority and the exit changed.
#[test]
fn the_standing_order_row_cites_the_notice_only_inside_its_relief_period() {
    for (year, cites_402, exit) in [
        (2024, false, "--set-pre2025-method"),
        (2026, true, "--set-forward-method"),
        (2027, false, "--set-forward-method"),
    ] {
        let (_d, vault) = vault_with(&[
            buy(
                Source::Coinbase,
                "BUY-1",
                datetime!(2024-01-05 12:00:00 UTC)
                    .replace_year(year)
                    .unwrap(),
                coinbase(),
                "50000.00",
            ),
            sell(
                Source::Coinbase,
                "SELL-1",
                datetime!(2024-03-10 12:00:00 UTC)
                    .replace_year(year)
                    .unwrap(),
                coinbase(),
                "60000.00",
            ),
        ]);
        let p = panel_of(&vault, year);
        assert_eq!(
            p.standing_orders.len(),
            1,
            "TY{year}: the substance survives on every year — one custodial venue with no dated \
             election: {:?}",
            p.standing_orders
        );
        let row = &p.standing_orders[0];
        let mut out = Vec::new();
        btctax_cli::step0::write_step0(&mut out, &p).unwrap();
        let printed = String::from_utf8(out).unwrap();
        // The HEADING is where the row's authority is asserted, so that is what must follow the
        // period. (Outside it the row still NAMES §4.02(2) — to say it does not reach the sale.)
        assert_eq!(
            printed.contains("STANDING ORDERS (Notice 2026-20 §4.02(2))"),
            cites_402,
            "TY{year}: §4.02(2) is asserted as the authority only inside its relief period: \
             {printed}"
        );
        // …and the TUI commit modal's heading is the SAME authority, not a second literal — it
        // asserted §4.02(2) on a TY2024 fixture whose own row said the relief had not begun.
        assert_eq!(
            p.standing_orders_authority() == "Notice 2026-20 §4.02(2)",
            cites_402,
            "TY{year}: one authority, two renderers"
        );
        assert_eq!(
            row.what.contains("relief period"),
            !cites_402,
            "TY{year}: outside the period the row says so, and inside it there is nothing to say: \
             {}",
            row.what
        );
        assert!(
            row.handoff.contains(exit),
            "TY{year}: the exit is {exit} — a pre-2025 year cannot take a forward election at all \
             (`method_election_is_forward` refuses it): {}",
            row.handoff
        );
        if year == 2024 {
            assert!(
                row.what.contains("BEGINS 2025-01-01"),
                "…and it says WHY: {}",
                row.what
            );
        }
        if year == 2027 {
            assert!(
                row.what.contains("ENDED 2026-12-31"),
                "…and it says WHY: {}",
                row.what
            );
        }
    }
}

/// ★★★ **(seam review M-4) THE STEP 0 PRINT SURFACES A CONTRADICTED `No`.**
///
/// `input_form_store::commit` runs `screen_inputs` only and holds no `LedgerState`, and
/// `interview_state(&ri)` takes no ledger — so neither the TUI commit gate nor R12's panel can see
/// that a filer's `digital_asset_activity = Some(false)` is contradicted, and they would first meet
/// the refusal at `report` / `export-irs-pdf`. The tier boundary stands; this surface DOES hold the
/// ledger, and it is where the filer is told.
///
/// Both directions, because either alone is satisfiable by a broken panel.
#[test]
fn the_step0_panel_names_a_digital_asset_answer_the_ledger_contradicts() {
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
            datetime!(2026-06-15 12:00:00 UTC),
            coinbase(),
            "60000.00",
        ),
    ]);
    let store = |answer: Option<bool>| {
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
        ri.digital_asset_activity = answer;
        return_inputs::set(s.conn(), YEAR, &ri).unwrap();
        s.save().unwrap();
    };

    // ── `No` against a ledger that witnesses a 2026 disposal ⇒ NAMED, with the event and the exit.
    store(Some(false));
    let p = panel_of(&vault, YEAR);
    assert_eq!(p.contradictions.len(), 1, "{:?}", p.contradictions);
    let row = &p.contradictions[0];
    for needle in [
        "2026-06-15",
        "exchange:coinbase:default",
        "a disposition",
        "REFUSE",
    ] {
        assert!(
            row.what.contains(needle),
            "the row must name the event the export will refuse on; missing {needle:?} in: {}",
            row.what
        );
    }
    assert!(
        row.handoff.contains("btctax income answer"),
        "…and the exit is the question: {}",
        row.handoff
    );
    // …and it is PRINTED, not merely computed.
    let mut out = Vec::new();
    btctax_cli::step0::write_step0(&mut out, &p).unwrap();
    let printed = String::from_utf8(out).unwrap();
    assert!(
        printed.contains("YOUR ANSWERS vs THE LEDGER") && printed.contains("2026-06-15"),
        "{printed}"
    );

    // ── `Yes` on the same ledger ⇒ SILENT (the asymmetry: only the `No` direction refuses).
    store(Some(true));
    assert!(
        panel_of(&vault, YEAR).contradictions.is_empty(),
        "a `Yes` this ledger witnesses is not a contradiction — a panel that always warned would \
         pass the half above"
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
                // ★★★ T7 / R6 — the per-row §152 gates join the sweep.
                Ask::DependentGate { gate, row } => {
                    !asked.contains(&format!("g{:?}{row}", gate.gate))
                }
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
                // ★★★ T7 / R6 — answered at the registry's declared claim-path polarity, because a
                //     blanket "no" on a §152 gate is not neutral: it routes the row down another
                //     branch of the flowchart, and some of those branches REFUSE.
                Ask::DependentGate { gate, row } => {
                    use btctax_core::tax::dependent_gates::GateKind;
                    asked.insert(format!("g{:?}{row}", gate.gate));
                    match gate.kind {
                        GateKind::Date => {
                            let dob = time::Date::from_calendar_date(
                                ri.tax_year - 10,
                                time::Month::June,
                                1,
                            )
                            .unwrap();
                            script.push_str(&format!("{dob}\n"));
                            ri.header.dependents[row].date_of_birth = Some(dob);
                        }
                        GateKind::YesNo => {
                            let v = gate
                                .claim_path
                                .expect("a YesNo gate declares its claim path");
                            script.push_str(if v { "y\n" } else { "n\n" });
                            (gate.set)(&mut ri.header.dependents[row], v);
                        }
                    }
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
        cmd::answer::AnswerOptions::default(),
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
///
/// ★★★ **(seam review M-3) IT RENDERS THE RENDERED PROMPTS TOO.** The first version of this test
/// walked `FORM_QUESTIONS` and `SKIPPABLE_QUESTIONS` — the STATIC `prompt` strings — and never
/// called `RENDERED_PROMPTS`, reintroducing in the same commit the blindness the T6 stop-list
/// extension was written to end: *"the words a filer is actually SHOWN were never read"*
/// (`r15_stop_list.rs`). The rendered set was clean when this was written, and *"it happened to be
/// clean"* is not the same fact as *"it is checked"* (harness B1).
#[test]
fn no_registry_question_asks_about_venue_or_account_granularity() {
    use btctax_core::tax::questions::{FORM_QUESTIONS, RENDERED_PROMPTS, SKIPPABLE_QUESTIONS};
    // A probe carrying the fixture's own year, so a renderer that interpolates one produces the
    // words a filer of THIS year is shown.
    let probe = ReturnInputs {
        tax_year: YEAR,
        filing_status: FilingStatus::Single,
        ..Default::default()
    };
    let prompts: Vec<(String, String)> = FORM_QUESTIONS
        .iter()
        .map(|q| (format!("FORM_QUESTIONS {:?}", q.id), q.prompt.to_string()))
        .chain(SKIPPABLE_QUESTIONS.iter().map(|s| {
            (
                format!("SKIPPABLE_QUESTIONS {:?}", s.id),
                s.prompt.to_string(),
            )
        }))
        .chain(
            RENDERED_PROMPTS
                .iter()
                .map(|(id, render)| (format!("RENDERED_PROMPTS {id:?}"), render(&probe))),
        )
        .map(|(label, text)| (label, text.to_ascii_lowercase()))
        .collect();
    // ★ ANTI-VACUITY, both halves: the walk must have found the registries AND it must have
    //   actually rendered every rendered prompt. Without the second, dropping the `chain` above
    //   leaves a smaller set that still clears a floor and the check goes quietly blind again.
    assert!(
        prompts.len() > 50,
        "the walk scanned only {} prompts — a check that scans nothing passes by finding nothing",
        prompts.len()
    );
    assert_eq!(
        prompts
            .iter()
            .filter(|(label, _)| label.starts_with("RENDERED_PROMPTS "))
            .count(),
        RENDERED_PROMPTS.len(),
        "every RENDERED prompt must be IN the scanned set — the words a filer is SHOWN are the \
         ones this check exists to read"
    );
    for (label, p) in &prompts {
        assert!(
            !(p.contains("how many accounts") || p.contains("which account")),
            "R9: account granularity is documented, not asked — this prompt asks it: {label}: {p}"
        );
    }
    // …and the note that replaces the question exists and says what btctax models.
    let note = btctax_cli::step0::VENUE_GRANULARITY_NOTE;
    assert!(
        note.contains("exchange:<venue>:default") && note.contains("1012(c)(1)"),
        "the note must name the shape btctax models AND the rule it matters for: {note}"
    );
}
