//! §A.5(a) PER-ACCOUNT (scoped) cost-basis method election KATs (Task 2).
//!
//! Exercises the SHARED wallet-aware resolver (`resolve::resolve_election`) through BOTH callers:
//! the fold (`applicable_method`, via disposal basis) AND compliance (`disposal_compliance`, via
//! `StandingOrder`). The resolver is TWO INDEPENDENT TIERS: latest in-force election SCOPED to the
//! disposal's wallet, else latest in-force GLOBAL election, else FIFO. `LotSelection` still overrides;
//! backdating still blocks. Fixtures use two exchange ACCOUNTS with identical 3-lot pools so a method
//! flip is visible in the reported basis (FIFO→A $50, LIFO→C $40, HIFO→B $90).
use btctax_core::event::*;
use btctax_core::identity::*;
use btctax_core::price::StaticPrices;
use btctax_core::project::{project, ProjectionConfig};
use btctax_core::state::*;
use btctax_core::{disposal_compliance, ComplianceStatus, DisposalCompliance, LotMethod};
use rust_decimal_macros::dec;
use time::macros::{date, datetime, offset};

// ── Wallets ──────────────────────────────────────────────────────────────────────────────────────
fn cb() -> WalletId {
    WalletId::Exchange {
        provider: "coinbase".into(),
        account: "main".into(),
    }
}
fn gm() -> WalletId {
    WalletId::Exchange {
        provider: "gemini".into(),
        account: "main".into(),
    }
}
fn cb_b() -> WalletId {
    WalletId::Exchange {
        provider: "coinbase".into(),
        account: "B".into(),
    }
}

// ── Event builders (source is identity-only; `rf` disambiguates; wallet is the pool key) ──────────
fn imp_w(rf: &str, ts: time::OffsetDateTime, wallet: &WalletId, p: EventPayload) -> LedgerEvent {
    LedgerEvent {
        id: EventId::import(Source::Coinbase, SourceRef::new(rf)),
        utc_timestamp: ts,
        original_tz: offset!(+00:00),
        wallet: Some(wallet.clone()),
        payload: p,
    }
}
fn dec_ev(seq: u64, ts: time::OffsetDateTime, p: EventPayload) -> LedgerEvent {
    LedgerEvent {
        id: EventId::decision(seq),
        utc_timestamp: ts,
        original_tz: offset!(+00:00),
        wallet: None, // [R0-M1] the scope lives in the MethodElection PAYLOAD, not the event column
        payload: p,
    }
}
fn buy_on(
    wallet: &WalletId,
    rf: &str,
    ts: time::OffsetDateTime,
    sat: i64,
    cost: rust_decimal::Decimal,
) -> LedgerEvent {
    imp_w(
        rf,
        ts,
        wallet,
        EventPayload::Acquire(Acquire {
            sat,
            usd_cost: cost,
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    )
}
fn sell_on(
    wallet: &WalletId,
    rf: &str,
    ts: time::OffsetDateTime,
    sat: i64,
    proceeds: rust_decimal::Decimal,
) -> LedgerEvent {
    imp_w(
        rf,
        ts,
        wallet,
        EventPayload::Dispose(Dispose {
            sat,
            usd_proceeds: proceeds,
            fee_usd: dec!(0),
            kind: DisposeKind::Sell,
        }),
    )
}
/// A method election; `scope = None` ⇒ global, `Some(w)` ⇒ per-account.
fn election(
    seq: u64,
    made: time::OffsetDateTime,
    eff: time::Date,
    m: LotMethod,
    scope: Option<WalletId>,
) -> LedgerEvent {
    dec_ev(
        seq,
        made,
        EventPayload::MethodElection(MethodElection {
            effective_from: eff,
            method: m,
            wallet: scope,
        }),
    )
}
fn void_of(seq: u64, ts: time::OffsetDateTime, target: u64) -> LedgerEvent {
    dec_ev(
        seq,
        ts,
        EventPayload::VoidDecisionEvent(VoidDecisionEvent {
            target_event_id: EventId::decision(target),
        }),
    )
}
/// Three post-2025 lots on `wallet` so the three methods diverge: FIFO→A $50, LIFO→C $40, HIFO→B $90.
fn three_post2025_on(wallet: &WalletId, prefix: &str) -> Vec<LedgerEvent> {
    vec![
        buy_on(
            wallet,
            &format!("{prefix}A"),
            datetime!(2025-02-01 00:00:00 UTC),
            100_000,
            dec!(50.00),
        ),
        buy_on(
            wallet,
            &format!("{prefix}B"),
            datetime!(2025-03-01 00:00:00 UTC),
            100_000,
            dec!(90.00),
        ),
        buy_on(
            wallet,
            &format!("{prefix}C"),
            datetime!(2025-04-01 00:00:00 UTC),
            100_000,
            dec!(40.00),
        ),
    ]
}
fn has(st: &LedgerState, k: BlockerKind) -> bool {
    st.blockers.iter().any(|b| b.kind == k)
}
fn basis_of(st: &LedgerState, rf: &str) -> rust_decimal::Decimal {
    st.disposals
        .iter()
        .find(|d| d.event == EventId::import(Source::Coinbase, SourceRef::new(rf)))
        .unwrap_or_else(|| panic!("disposal {rf} not found"))
        .legs[0]
        .basis
}
fn status_of(dcs: &[DisposalCompliance], rf: &str) -> ComplianceStatus {
    dcs.iter()
        .find(|c| c.disposal == EventId::import(Source::Coinbase, SourceRef::new(rf)))
        .unwrap_or_else(|| panic!("compliance row {rf} not found"))
        .status
        .clone()
}

// ─────────────────────────────────────────────────────────────────────────────────────────────────

/// A Coinbase→HIFO scoped election governs ONLY Coinbase; a Gemini disposal with the same-shaped pool
/// still uses FIFO (no election). Different (correct) gains on the two wallets.
#[test]
fn per_wallet_method_governs_only_that_wallet() {
    let mut evs = three_post2025_on(&cb(), "cb");
    evs.extend(three_post2025_on(&gm(), "gm"));
    evs.push(election(
        1,
        datetime!(2025-01-02 00:00:00 UTC),
        date!(2025 - 01 - 02),
        LotMethod::Hifo,
        Some(cb()),
    ));
    evs.push(sell_on(
        &cb(),
        "cbD",
        datetime!(2025-07-01 00:00:00 UTC),
        100_000,
        dec!(95.00),
    ));
    evs.push(sell_on(
        &gm(),
        "gmD",
        datetime!(2025-07-01 00:00:00 UTC),
        100_000,
        dec!(95.00),
    ));
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(!has(&st, BlockerKind::MethodElectionBackdated));
    assert_eq!(
        basis_of(&st, "cbD"),
        dec!(90.00),
        "Coinbase scoped HIFO -> B"
    );
    assert_eq!(
        basis_of(&st, "gmD"),
        dec!(50.00),
        "Gemini unelected -> FIFO -> A"
    );
}

/// scoped ≻ global ≻ FIFO: a global LIFO plus a Coinbase HIFO election — Coinbase uses HIFO, Gemini
/// (no scoped election) uses the GLOBAL LIFO (NOT FIFO).
#[test]
fn scoped_beats_global_beats_fifo() {
    let mut evs = three_post2025_on(&cb(), "cb");
    evs.extend(three_post2025_on(&gm(), "gm"));
    evs.push(election(
        1,
        datetime!(2025-01-02 00:00:00 UTC),
        date!(2025 - 01 - 02),
        LotMethod::Lifo,
        None, // GLOBAL
    ));
    evs.push(election(
        2,
        datetime!(2025-01-02 00:00:00 UTC),
        date!(2025 - 01 - 02),
        LotMethod::Hifo,
        Some(cb()), // SCOPED
    ));
    evs.push(sell_on(
        &cb(),
        "cbD",
        datetime!(2025-07-01 00:00:00 UTC),
        100_000,
        dec!(95.00),
    ));
    evs.push(sell_on(
        &gm(),
        "gmD",
        datetime!(2025-07-01 00:00:00 UTC),
        100_000,
        dec!(95.00),
    ));
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert_eq!(
        basis_of(&st, "cbD"),
        dec!(90.00),
        "scoped HIFO wins on Coinbase"
    );
    assert_eq!(
        basis_of(&st, "gmD"),
        dec!(40.00),
        "global LIFO governs Gemini (not FIFO)"
    );
}

/// [R0-M2] TWO INDEPENDENT TIERS: a LATER-dated GLOBAL election does NOT override an in-force SCOPED
/// one. Coinbase HIFO (effective Jan) then a later GLOBAL LIFO (effective Mar): an April Coinbase
/// disposal still uses HIFO. A merged `max_by` would wrongly flip it to LIFO.
#[test]
fn later_global_does_not_override_in_force_scoped() {
    let mut evs = three_post2025_on(&cb(), "cb");
    evs.push(election(
        1,
        datetime!(2025-01-15 00:00:00 UTC),
        date!(2025 - 01 - 15),
        LotMethod::Hifo,
        Some(cb()), // scoped, effective Jan
    ));
    evs.push(election(
        2,
        datetime!(2025-03-15 00:00:00 UTC),
        date!(2025 - 03 - 15),
        LotMethod::Lifo,
        None, // LATER global, effective Mar
    ));
    evs.push(sell_on(
        &cb(),
        "cbD",
        datetime!(2025-04-15 00:00:00 UTC),
        100_000,
        dec!(95.00),
    ));
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert_eq!(
        basis_of(&st, "cbD"),
        dec!(90.00),
        "in-force scoped HIFO ($90/B) must NOT be overridden by the later global LIFO ($40/C)"
    );
}

/// [R0-r2-M1] A NOT-YET-EFFECTIVE scoped election must NOT suppress an in-force GLOBAL one: it fails
/// the tier-1 `effective_from <= date` filter, so tier 2 (global) governs — NOT FIFO. Global LIFO
/// (Jan) + a Coinbase HIFO effective in the FUTURE (Aug): a July Coinbase disposal uses global LIFO.
#[test]
fn not_yet_effective_scoped_falls_to_global() {
    let mut evs = three_post2025_on(&cb(), "cb");
    evs.push(election(
        1,
        datetime!(2025-01-02 00:00:00 UTC),
        date!(2025 - 01 - 02),
        LotMethod::Lifo,
        None, // in-force global
    ));
    evs.push(election(
        2,
        datetime!(2025-06-01 00:00:00 UTC),
        date!(2025 - 08 - 01), // effective_from is AFTER the disposal — not yet effective
        LotMethod::Hifo,
        Some(cb()),
    ));
    evs.push(sell_on(
        &cb(),
        "cbD",
        datetime!(2025-07-01 00:00:00 UTC),
        100_000,
        dec!(95.00),
    ));
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(!has(&st, BlockerKind::MethodElectionBackdated));
    assert_eq!(
        basis_of(&st, "cbD"),
        dec!(40.00),
        "not-yet-effective scoped HIFO falls to the in-force GLOBAL LIFO ($40/C), not FIFO ($50) nor HIFO ($90)"
    );
}

/// A per-disposal `LotSelection` still overrides the wallet election. Coinbase HIFO election would
/// pick B ($90); the selection pins C ($40) — the selection wins.
#[test]
fn lot_selection_still_overrides_scoped_election() {
    let mut evs = three_post2025_on(&cb(), "cb");
    evs.push(election(
        1,
        datetime!(2025-01-02 00:00:00 UTC),
        date!(2025 - 01 - 02),
        LotMethod::Hifo,
        Some(cb()),
    ));
    evs.push(sell_on(
        &cb(),
        "cbD",
        datetime!(2025-07-01 00:00:00 UTC),
        100_000,
        dec!(95.00),
    ));
    // Pin lot C (the cheapest, newest) — neither HIFO ($90/B) nor FIFO ($50/A) would choose it.
    evs.push(dec_ev(
        2,
        datetime!(2025-06-15 00:00:00 UTC),
        EventPayload::LotSelection(LotSelection {
            disposal_event: EventId::import(Source::Coinbase, SourceRef::new("cbD")),
            lots: vec![LotPick {
                lot: LotId {
                    origin_event_id: EventId::import(Source::Coinbase, SourceRef::new("cbC")),
                    split_sequence: 0,
                },
                sat: 100_000,
            }],
        }),
    ));
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert_eq!(
        basis_of(&st, "cbD"),
        dec!(40.00),
        "LotSelection (C, $40) overrides the scoped HIFO election (B, $90)"
    );
}

/// A scoped election whose `effective_from` precedes its made-date is BACK-DATED → the hard
/// `MethodElectionBackdated` blocker fires and the election contributes nothing (falls to FIFO).
#[test]
fn scoped_election_backdating_blocks() {
    let mut evs = three_post2025_on(&cb(), "cb");
    evs.push(election(
        1,
        datetime!(2025-05-01 00:00:00 UTC),
        date!(2025 - 02 - 10), // effective_from < made-date -> back-dated
        LotMethod::Hifo,
        Some(cb()),
    ));
    evs.push(sell_on(
        &cb(),
        "cbD",
        datetime!(2025-07-01 00:00:00 UTC),
        100_000,
        dec!(95.00),
    ));
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(has(&st, BlockerKind::MethodElectionBackdated));
    assert_eq!(
        basis_of(&st, "cbD"),
        dec!(50.00),
        "back-dated scoped election is rejected -> FIFO -> A"
    );
}

/// [R0-I1] A scoped election must NOT taint the COMPLIANCE of another wallet. A Coinbase→HIFO
/// election: the Coinbase disposal is `StandingOrder`; a Gemini disposal is `NonCompliant` (the
/// shared resolver scopes correctly in `disposal_compliance` — tier 1 empty for Gemini, tier 2 global
/// empty).
#[test]
fn scoped_election_does_not_taint_compliance_of_other_wallets() {
    let mut evs = three_post2025_on(&cb(), "cb");
    evs.extend(three_post2025_on(&gm(), "gm"));
    evs.push(election(
        1,
        datetime!(2025-06-01 00:00:00 UTC),
        date!(2025 - 06 - 01),
        LotMethod::Hifo,
        Some(cb()),
    ));
    evs.push(sell_on(
        &cb(),
        "cbD",
        datetime!(2025-07-01 00:00:00 UTC),
        100_000,
        dec!(95.00),
    ));
    evs.push(sell_on(
        &gm(),
        "gmD",
        datetime!(2025-07-01 00:00:00 UTC),
        100_000,
        dec!(95.00),
    ));
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    let dcs = disposal_compliance(&evs, &st);
    assert!(
        matches!(
            status_of(&dcs, "cbD"),
            ComplianceStatus::StandingOrder { effective_from } if effective_from == date!(2025 - 06 - 01)
        ),
        "Coinbase disposal is StandingOrder on its scoped election"
    );
    assert_eq!(
        status_of(&dcs, "gmD"),
        ComplianceStatus::NonCompliant,
        "Gemini disposal is NOT StandingOrder on account of the Coinbase-scoped election"
    );
}

/// Two ACCOUNTS at the SAME provider are independent pools/scopes: `coinbase:main` (HIFO election) vs
/// `coinbase:B` (no election → FIFO).
#[test]
fn two_accounts_same_provider_independent() {
    let mut evs = three_post2025_on(&cb(), "cbm"); // coinbase:main
    evs.extend(three_post2025_on(&cb_b(), "cbb")); // coinbase:B
    evs.push(election(
        1,
        datetime!(2025-01-02 00:00:00 UTC),
        date!(2025 - 01 - 02),
        LotMethod::Hifo,
        Some(cb()), // scoped to coinbase:main only
    ));
    evs.push(sell_on(
        &cb(),
        "mD",
        datetime!(2025-07-01 00:00:00 UTC),
        100_000,
        dec!(95.00),
    ));
    evs.push(sell_on(
        &cb_b(),
        "bD",
        datetime!(2025-07-01 00:00:00 UTC),
        100_000,
        dec!(95.00),
    ));
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert_eq!(basis_of(&st, "mD"), dec!(90.00), "coinbase:main HIFO -> B");
    assert_eq!(
        basis_of(&st, "bD"),
        dec!(50.00),
        "coinbase:B unelected -> FIFO -> A"
    );
}

/// A voided scoped election reverts the wallet to the fall-through (here: FIFO, no global present).
#[test]
fn voided_scoped_election_falls_back() {
    let mut evs = three_post2025_on(&cb(), "cb");
    evs.push(election(
        1,
        datetime!(2025-01-02 00:00:00 UTC),
        date!(2025 - 01 - 02),
        LotMethod::Hifo,
        Some(cb()),
    ));
    evs.push(void_of(2, datetime!(2025-06-01 00:00:00 UTC), 1));
    evs.push(sell_on(
        &cb(),
        "cbD",
        datetime!(2025-07-01 00:00:00 UTC),
        100_000,
        dec!(95.00),
    ));
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert_eq!(
        basis_of(&st, "cbD"),
        dec!(50.00),
        "voided scoped HIFO -> back to FIFO -> A"
    );
}

/// A scoped election coexists with pre-2025 Universal residue: `pre2025_method` still governs the
/// pre-2025 residue disposal; the scoped method governs post-2025 disposals on that wallet.
#[test]
fn pre2025_residue_plus_post2025_scoped_election() {
    // Pre-2025 Universal lots on Coinbase: P1 (older, $50) and P2 ($90).
    let mut evs = vec![
        buy_on(
            &cb(),
            "P1",
            datetime!(2024-02-01 00:00:00 UTC),
            100_000,
            dec!(50.00),
        ),
        buy_on(
            &cb(),
            "P2",
            datetime!(2024-03-01 00:00:00 UTC),
            100_000,
            dec!(90.00),
        ),
        // Pre-2025 disposal governed by the config pre2025_method (HIFO -> P2 $90).
        sell_on(
            &cb(),
            "S0",
            datetime!(2024-09-01 00:00:00 UTC),
            100_000,
            dec!(95.00),
        ),
        // Post-2025 directly-acquired lot Q on Coinbase ($40, newest).
        buy_on(
            &cb(),
            "Q",
            datetime!(2025-05-01 00:00:00 UTC),
            100_000,
            dec!(40.00),
        ),
    ];
    // Scoped LIFO on Coinbase for post-2025 disposals.
    evs.push(election(
        1,
        datetime!(2025-06-01 00:00:00 UTC),
        date!(2025 - 06 - 01),
        LotMethod::Lifo,
        Some(cb()),
    ));
    // Post-2025 disposal: pool is [P1 (seeded, 2024-02-01, $50), Q (2025-05-01, $40)]; LIFO -> Q ($40).
    evs.push(sell_on(
        &cb(),
        "S1",
        datetime!(2025-07-01 00:00:00 UTC),
        100_000,
        dec!(95.00),
    ));
    let cfg = ProjectionConfig {
        pre2025_method: LotMethod::Hifo,
        ..ProjectionConfig::default()
    };
    let st = project(&evs, &StaticPrices::default(), &cfg);
    assert_eq!(
        basis_of(&st, "S0"),
        dec!(90.00),
        "pre-2025 residue disposal governed by pre2025_method HIFO -> P2 $90"
    );
    assert_eq!(
        basis_of(&st, "S1"),
        dec!(40.00),
        "post-2025 disposal governed by the scoped LIFO -> newest Q $40 (not FIFO P1 $50)"
    );
}

/// Determinism: two scoped elections for the SAME wallet with the SAME `effective_from` are ordered by
/// `decision_seq` — the highest-seq (latest) wins — and the result is load-order independent.
#[test]
fn determinism_two_scoped_elections_latest_wins() {
    let make = |reversed: bool| -> LedgerState {
        let mut evs = three_post2025_on(&cb(), "cb");
        let e1 = election(
            1,
            datetime!(2025-01-05 00:00:00 UTC),
            date!(2025 - 06 - 01),
            LotMethod::Hifo,
            Some(cb()),
        );
        let e2 = election(
            2,
            datetime!(2025-01-10 00:00:00 UTC),
            date!(2025 - 06 - 01), // SAME effective_from -> decision_seq breaks the tie (seq 2 wins)
            LotMethod::Lifo,
            Some(cb()),
        );
        if reversed {
            evs.push(e2.clone());
            evs.push(e1.clone());
        } else {
            evs.push(e1);
            evs.push(e2);
        }
        evs.push(sell_on(
            &cb(),
            "cbD",
            datetime!(2025-07-01 00:00:00 UTC),
            100_000,
            dec!(95.00),
        ));
        project(&evs, &StaticPrices::default(), &ProjectionConfig::default())
    };
    let st = make(false);
    let st_rev = make(true);
    assert_eq!(
        basis_of(&st, "cbD"),
        dec!(40.00),
        "latest-seq scoped election (LIFO, seq 2) wins the (effective_from, decision_seq) tie -> C $40"
    );
    assert_eq!(
        basis_of(&st_rev, "cbD"),
        dec!(40.00),
        "load-order independent: reversing the two election events yields the identical result"
    );
}
