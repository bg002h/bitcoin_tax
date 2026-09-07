//! ★★★ **R9 / T6 — THE DIGITAL ASSETS QUESTION, AS A FIVE-ROW TABLE.**
//!
//! Form 1040 page 1 asks it above line 1a, and the instructions leave no room: *"You must answer the
//! digital asset question on Form 1040 whether or not you received a Form 1099-DA"*
//! (`i1040gi--2025.txt:1398-1400`). Until T6 btctax decided the box from a LEDGER PREDICATE, and a
//! predicate can only ever say *Yes* or say nothing — so a filer who bought monthly and sold nothing
//! signed a return with a mandatory question blank.
//!
//! The answer is a class-(A) declaration now, and the ledger CROSS-CHECKS it. The cross-check is
//! **asymmetric**, and the asymmetry is the whole rule:
//!
//! | ledger | answer | outcome |
//! |---|---|---|
//! | witnesses a qualifying event | `No`  | **REFUSE**, naming the FIRST qualifying event (date, venue, kind) |
//! | witnesses a qualifying event | `Yes` | prints **Yes** |
//! | witnesses nothing            | `No`  | prints **No** — the box T6 made reachable at all |
//! | witnesses nothing            | `Yes` | prints **Yes**, WITH the off-ledger warning and **no refusal** |
//! | `Acquire` events + a linked self-transfer, only | `No` | prints **No** — a purchase is not a Yes-forcing event |
//!
//! ★★★ **Why the fourth row may never refuse.** The ledger is not complete by construction — no
//!     self-custody wallet is importable at all — so a filer paid in BTC to their own wallet has no
//!     export to import. Refusing their truthful *Yes* would leave *No* as the only way through the
//!     gate: a false answer the tool coerced into sworn testimony under §6065.
//!
//! ★★ **The fifth row is built by the REAL FOLD**, from `Acquire` events plus a `TransferLink`ed
//!    self-transfer, rather than by hand-constructing a `LedgerState`. The instruction's own
//!    carve-outs are *"[h]olding a digital asset"*, *"[t]ransferring a digital asset from one wallet
//!    or account you own or control to another"* and *"[p]urchasing digital assets using U.S. or
//!    other real currency"* (`i1040gi--2025.txt:1385-1394`) — so the claim under test is that btctax's
//!    FOLD emits no disposal, no income and no removal for those, and a hand-built empty state would
//!    assert that claim against itself.
//!
//! ★ Every row CALLS the instruments it protects — `screen_compute_dependent`, `advisories`, and the
//!   printed chain `assemble_printed_forms` — never a re-implementation of their predicates.
//!
//! All fixtures are SYNTHETIC.

use btctax_core::conventions::Sat;
use btctax_core::event::*;
use btctax_core::identity::*;
use btctax_core::price::StaticPrices;
use btctax_core::project::{project, ProjectionConfig};
use btctax_core::state::{Disposal, DisposalLeg, LedgerState, Term};
use btctax_core::tax::advisories::{advisories, Advisory};
use btctax_core::tax::return_1040::{assemble_absolute, screen_compute_dependent, AbsoluteReturn};
use btctax_core::tax::return_inputs::{Owner, ReturnInputs, W2};
use btctax_core::tax::return_refuse::RefuseReason;
use btctax_core::tax::testonly::{answer_all_live_declarations, ty2024_params, ty2024_table};
use btctax_core::tax::types::FilingStatus;
use rust_decimal_macros::dec;
use time::macros::{date, datetime, offset};

const YEAR: i32 = 2024;
const ONE_BTC: Sat = 100_000_000;

fn exchange() -> WalletId {
    WalletId::Exchange {
        provider: "coinbase".into(),
        account: "default".into(),
    }
}

/// A filer with wages and nothing else unusual, every live declaration answered at its neutral, and
/// the Digital Assets question then answered explicitly by the caller.
fn filer(answer: Option<bool>) -> ReturnInputs {
    let mut ri = ReturnInputs {
        tax_year: YEAR,
        filing_status: FilingStatus::Single,
        header: btctax_core::tax::testonly::not_a_dependent(),
        w2s: vec![W2 {
            owner: Owner::Taxpayer,
            employer: "ACME".into(),
            box1_wages: dec!(80000),
            box3_ss_wages: dec!(80000),
            box5_medicare_wages: dec!(80000),
            ..Default::default()
        }],
        ..Default::default()
    };
    // ★ FR-29 — an ADULT filer, so Form 8615's condition 3 is proved FALSE by arithmetic and this
    //   table measures the Digital Assets rule rather than the kiddie-tax gate. 1980 is mid-band for
    //   2024 ([1961, 2000]): late enough not to be under 24, early enough not to reach §63(f)'s
    //   age-65 addition.
    ri.header.taxpayer.date_of_birth = Some(date!(1980 - 05 - 05));
    answer_all_live_declarations(&mut ri);
    ri.digital_asset_activity = answer;
    ri
}

/// A ledger with ONE 2024 exchange disposal — a qualifying event under the instruction's own list.
fn ledger_with_a_disposal() -> LedgerState {
    LedgerState {
        disposals: vec![Disposal {
            event: EventId::import(Source::Coinbase, SourceRef::new("SELL-1")),
            kind: DisposeKind::Sell,
            disposed_at: date!(2024 - 06 - 15),
            legs: vec![DisposalLeg {
                lot_id: LotId {
                    origin_event_id: EventId::import(Source::Coinbase, SourceRef::new("BUY-1")),
                    split_sequence: 0,
                },
                sat: ONE_BTC,
                proceeds: dec!(30000),
                basis: dec!(10000),
                gain: dec!(20000),
                term: Term::LongTerm,
                basis_source: BasisSource::ExchangeProvided,
                gift_zone: None,
                acquired_at: date!(2020 - 01 - 01),
                lot_acquired_at: date!(2020 - 01 - 01),
                wallet: exchange(),
                pseudo: false,
            }],
            fee_mini_disposition: false,
        }],
        ..Default::default()
    }
}

/// ★★★ Row 5's ledger, BUILT BY THE FOLD: buy 1 BTC on the exchange, withdraw it, and confirm the
/// arrival at the filer's own cold wallet as ONE self-transfer (`TransferLink` → relocate).
///
/// Nothing here is a receipt "as a reward, award, or payment", and nothing is a disposition — so the
/// instruction's own carve-outs apply and the correct answer is `No`.
fn ledger_of_buys_and_a_linked_self_transfer() -> LedgerState {
    let imp = |r: &str, ts, wallet, payload| LedgerEvent {
        id: EventId::import(Source::Coinbase, SourceRef::new(r)),
        utc_timestamp: ts,
        original_tz: offset!(+00:00),
        wallet: Some(wallet),
        payload,
    };
    let events = vec![
        imp(
            "BUY-1",
            datetime!(2024-02-01 12:00:00 UTC),
            exchange(),
            EventPayload::Acquire(Acquire {
                sat: ONE_BTC,
                usd_cost: dec!(50000.00),
                fee_usd: dec!(0.00),
                basis_source: BasisSource::ExchangeProvided,
            }),
        ),
        imp(
            "BUY-2",
            datetime!(2024-03-01 12:00:00 UTC),
            exchange(),
            EventPayload::Acquire(Acquire {
                sat: ONE_BTC,
                usd_cost: dec!(60000.00),
                fee_usd: dec!(0.00),
                basis_source: BasisSource::ExchangeProvided,
            }),
        ),
        imp(
            "OUT-1",
            datetime!(2024-04-01 12:00:00 UTC),
            exchange(),
            EventPayload::TransferOut(TransferOut {
                sat: ONE_BTC,
                fee_sat: None,
                dest_addr: None,
                txid: None,
            }),
        ),
        imp(
            "IN-1",
            datetime!(2024-04-01 13:00:00 UTC),
            WalletId::SelfCustody {
                label: "cold".into(),
            },
            EventPayload::TransferIn(TransferIn {
                sat: ONE_BTC,
                src_addr: None,
                txid: None,
            }),
        ),
        LedgerEvent {
            id: EventId::decision(1),
            utc_timestamp: datetime!(2024-04-02 00:00:00 UTC),
            original_tz: offset!(+00:00),
            wallet: None,
            payload: EventPayload::TransferLink(TransferLink {
                out_event: EventId::import(Source::Coinbase, SourceRef::new("OUT-1")),
                in_event_or_wallet: TransferTarget::InEvent(EventId::import(
                    Source::Coinbase,
                    SourceRef::new("IN-1"),
                )),
            }),
        },
    ];
    let state = project(
        &events,
        &StaticPrices(Default::default()),
        &ProjectionConfig::default(),
    );
    // The premise, asserted rather than assumed: the FOLD produced no qualifying event. Without this
    // the row could pass because the fixture was malformed rather than because the rule holds.
    assert!(
        state.disposals.is_empty()
            && state.income_recognized.is_empty()
            && state.removals.is_empty(),
        "premise: buys + a linked self-transfer produce no disposal, no income and no removal — \
         got {} / {} / {}",
        state.disposals.len(),
        state.income_recognized.len(),
        state.removals.len()
    );
    assert!(
        !state.lots.is_empty(),
        "premise: the buys DID create lots, so the ledger is not merely empty"
    );
    state
}

/// The refusal `screen_compute_dependent` reports for this (return, ledger) pair — the real screen,
/// called with the real params.
fn refusal(ri: &ReturnInputs, state: &LedgerState) -> Option<(RefuseReason, String)> {
    screen_compute_dependent(ri, state, YEAR, &ty2024_params()).map(|r| (r.reason, r.detail))
}

/// What the return PRINTS in the Digital Assets box — through the whole printed chain, which is what
/// `btctax-forms` reads when it decides between the `da_yes` and `da_no` AcroForm fields.
fn printed_box(ri: &ReturnInputs, state: &LedgerState) -> Option<bool> {
    let params = ty2024_params();
    let table = ty2024_table();
    let ar: AbsoluteReturn = assemble_absolute(ri, state, &params, &table, YEAR);
    let forms = btctax_core::tax::packet::assemble_printed_forms(
        ri,
        state,
        &std::collections::BTreeMap::new(),
        &ar,
        &table,
        YEAR,
        &[],
        btctax_core::InformationReturnRegime::NONE,
    );
    forms.f1040.digital_asset_answer
}

/// The advisories this return raises — the real list, so the off-ledger warning is measured where a
/// filer would actually meet it.
fn advisory_list(ri: &ReturnInputs, state: &LedgerState) -> Vec<Advisory> {
    let params = ty2024_params();
    let table = ty2024_table();
    let ar = assemble_absolute(ri, state, &params, &table, YEAR);
    advisories(
        ri,
        state,
        ar.wages,
        ar.agi,
        ar.overpayment_refund,
        &params,
        YEAR,
        ar.deduction_is_itemized,
    )
}

/// ★★★ **THE FIVE-ROW TABLE, AS ONE TEST** (R9's kill).
#[test]
fn the_digital_asset_answer_table_holds_in_all_five_cells() {
    let active = ledger_with_a_disposal();
    let empty = LedgerState::default();
    let buys_only = ledger_of_buys_and_a_linked_self_transfer();

    // ── ROW 1 — activity ∧ `No` ⇒ REFUSE, and the message NAMES the first qualifying event. ─────
    let ri = filer(Some(false));
    let (reason, detail) =
        refusal(&ri, &active).expect("a `No` the ledger contradicts must refuse");
    assert_eq!(
        reason,
        RefuseReason::DigitalAssetAnswerContradictsLedger {
            date: "2024-06-15".into(),
            venue: "exchange:coinbase:default".into(),
            kind: "a disposition",
        },
        "the refusal carries the first qualifying event as a PAYLOAD, not only in prose"
    );
    for needle in ["2024-06-15", "exchange:coinbase:default", "a disposition"] {
        assert!(
            detail.contains(needle),
            "the message must name the event so the filer can go and look at it; missing \
             {needle:?} in: {detail}"
        );
    }

    // ── ROW 2 — activity ∧ `Yes` ⇒ no refusal, and the box prints Yes. ───────────────────────────
    let ri = filer(Some(true));
    assert_eq!(
        refusal(&ri, &active),
        None,
        "a `Yes` the ledger agrees with"
    );
    assert_eq!(printed_box(&ri, &active), Some(true));
    assert!(
        !advisory_list(&ri, &active)
            .iter()
            .any(|a| matches!(a, Advisory::DigitalAssetYesNotOnLedger { .. })),
        "the ledger witnesses the activity, so there is nothing off-ledger to warn about"
    );

    // ── ROW 3 — no activity ∧ `No` ⇒ the box prints NO. This is the cell T6 exists for: before it,
    //    the box was decided by a predicate that could only ever check *Yes*. ────────────────────
    let ri = filer(Some(false));
    assert_eq!(refusal(&ri, &empty), None);
    assert_eq!(
        printed_box(&ri, &empty),
        Some(false),
        "a filer with no receipts and no disposals must be able to PRINT \"No\""
    );

    // ── ROW 4 — no activity ∧ `Yes` ⇒ prints Yes, WITH the off-ledger warning and NO refusal. ────
    let ri = filer(Some(true));
    assert_eq!(
        refusal(&ri, &empty),
        None,
        "★★★ an unwitnessed `Yes` may NEVER refuse: the ledger is not complete by construction, and \
         refusing would leave `No` as the only way through the gate"
    );
    assert_eq!(printed_box(&ri, &empty), Some(true));
    let warned = advisory_list(&ri, &empty);
    let w = warned
        .iter()
        .find(|a| matches!(a, Advisory::DigitalAssetYesNotOnLedger { .. }))
        .expect("an unwitnessed `Yes` is accepted WITH the off-ledger warning");
    let msg = w.message();
    for needle in ["8v", "8949"] {
        assert!(
            msg.contains(needle),
            "the warning must say what the `Yes` does NOT do — off-ledger activity reaches neither \
             Schedule 1 line 8v nor Form 8949; missing {needle:?} in: {msg}"
        );
    }

    // ── ROW 5 — buys and a linked self-transfer only, answered `No` ⇒ prints NO. ─────────────────
    let ri = filer(Some(false));
    assert_eq!(
        refusal(&ri, &buys_only),
        None,
        "★ a PURCHASE is not a Yes-forcing event, and neither is moving coins between wallets you \
         own (i1040gi--2025.txt:1385-1394)"
    );
    assert_eq!(printed_box(&ri, &buys_only), Some(false));

    // ── And the mirror of row 5: answering `Yes` on that same ledger warns, and does not refuse. ─
    let ri = filer(Some(true));
    assert_eq!(refusal(&ri, &buys_only), None);
    assert!(
        advisory_list(&ri, &buys_only)
            .iter()
            .any(|a| matches!(a, Advisory::DigitalAssetYesNotOnLedger { .. })),
        "a buy-only ledger witnesses no qualifying event, so a `Yes` is off-ledger and is warned"
    );
}

/// ★★★ **The FIRST qualifying event is the EARLIEST one, and the pairing with the predicate is
/// ASSERTED rather than promised.**
///
/// Two independent definitions of *"the ledger witnesses a qualifying event"* is the drift that
/// would let the refusal fire with nothing to point at — or, worse, stay silent while the predicate
/// says the answer is contradicted. `first_digital_asset_event` is `pub(crate)`, so the pairing is
/// measured through the two PUBLIC surfaces that read it: the refusal fires exactly when the printed
/// box would have been forced, and it names the earliest event of the year.
#[test]
fn the_refusal_names_the_earliest_qualifying_event_and_fires_exactly_when_one_exists() {
    let mut state = ledger_with_a_disposal();
    // A SECOND, EARLIER qualifying event: income recognized in March, before June's disposal.
    state.income_recognized.push(btctax_core::IncomeRecord {
        event: EventId::import(Source::Swan, SourceRef::new("MINE-1")),
        recognized_at: date!(2024 - 03 - 02),
        sat: 1_000_000,
        usd_fmv: dec!(500),
        kind: IncomeKind::Mining,
        business: false,
        pseudo: false,
    });
    let ri = filer(Some(false));
    let (reason, _) = refusal(&ri, &state).expect("still contradicted");
    assert_eq!(
        reason,
        RefuseReason::DigitalAssetAnswerContradictsLedger {
            date: "2024-03-02".into(),
            venue: "swan".into(),
            kind: "digital assets received as income",
        },
        "the FIRST qualifying event is the earliest one — a filer told to check a June sale would \
         look at the wrong row"
    );

    // …and with the year moved off both events, the refusal is silent and the box prints `No`.
    let mut ri2015 = filer(Some(false));
    ri2015.tax_year = YEAR;
    assert_eq!(
        screen_compute_dependent(&ri2015, &LedgerState::default(), YEAR, &ty2024_params())
            .map(|r| r.reason),
        None,
        "no qualifying event ⇒ no refusal, whatever the answer says"
    );
}
