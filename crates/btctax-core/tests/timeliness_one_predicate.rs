//! **I-1 — §1.1012-1(j)(2) timeliness is ONE predicate, and these tests hold it to that.**
//!
//! The reg requires a specific identification "no later than the date **and time** of the sale,
//! disposition, or transfer". That rule was implemented at THREE sites, and FR-35 narrowed only one
//! of them from calendar-day to instant granularity:
//!
//! | site | what it decides |
//! |---|---|
//! | `resolve.rs` §A.4 timeliness `retain` | whether the fold APPLIES the selection (the filed number) |
//! | `compliance::identification_made` | the §A.5 verdict `verify` PRINTS, and the fold's FR-34 advisory |
//! | `optimize::persistability` / `::proposed_compliance_status` | what `optimize` PRINTS and what `accept` will write |
//!
//! For seven hours of every sale day the three disagreed: a 10:00 sale with an unattested 17:00
//! same-day selection was DROPPED by the fold as `LotSelectionPostHoc` (basis falling back to method
//! order) while `verify` and `optimize` reported the very same disposal `Contemporaneous`. The filed
//! number was conservative on every input, so this was never an understatement path — it was the tool
//! contradicting itself about one disposal, and a violation of
//! `design/SPEC_lot_optimization_program.md:218` ("no artifact, command, or doc may describe post-hoc
//! selection as compliant").
//!
//! ★★ **B1 — what these tests are for is not "all three are instants today".** It is that they
//! cannot silently diverge again. Every site is pinned at the SAME same-day-late window (10:00 sale,
//! 17:00 selection), so re-narrowing ANY ONE of them back to `TaxDate` reds a test naming that site.
//! The `..._agree_...` test additionally asserts the three against EACH OTHER, so a change that
//! narrows all of them at once — which no per-site test can see — reds too.
use btctax_core::event::*;
use btctax_core::identity::*;
use btctax_core::optimize::{persistability, proposed_compliance_status, Persistability};
use btctax_core::price::StaticPrices;
use btctax_core::project::{project, DispositionMoment, ProjectionConfig};
use btctax_core::state::*;
use btctax_core::{disposal_compliance, ComplianceStatus, LotMethod};
use rust_decimal_macros::dec;
use time::macros::{date, datetime, offset};
use time::OffsetDateTime;

// ── Fixture ──────────────────────────────────────────────────────────────────────────────────────
//
// Self-custody on purpose: the 2027+ broker envelope is a separate, DATE-granular gate (§6045 broker
// reporting is a year test) that would short-circuit ahead of the timeliness lever and mask it.

fn cold() -> WalletId {
    WalletId::SelfCustody {
        label: "cold".into(),
    }
}

/// The sale: 100k sat on 2025-07-01 at **10:00 UTC**.
const SALE_AT: OffsetDateTime = datetime!(2025-07-01 10:00:00 UTC);

fn imp(rf: &str, ts: OffsetDateTime, p: EventPayload) -> LedgerEvent {
    LedgerEvent {
        id: EventId::import(Source::Coinbase, SourceRef::new(rf)),
        utc_timestamp: ts,
        original_tz: offset!(+00:00),
        wallet: Some(cold()),
        payload: p,
    }
}

fn dec_ev(seq: u64, ts: OffsetDateTime, p: EventPayload) -> LedgerEvent {
    LedgerEvent {
        id: EventId::decision(seq),
        utc_timestamp: ts,
        original_tz: offset!(+00:00),
        wallet: None,
        payload: p,
    }
}

fn pid(rf: &str) -> LotId {
    LotId {
        origin_event_id: EventId::import(Source::Coinbase, SourceRef::new(rf)),
        split_sequence: 0,
    }
}

/// Three post-2025 lots whose method orders are distinct: FIFO→A ($50), LIFO→C ($40), HIFO→B ($90).
/// The late selection cherry-picks **B ($90)** — the highest basis, i.e. the lowest reported gain,
/// which is exactly the direction §1.1012-1(j)(2) exists to stop.
fn three_lots() -> Vec<LedgerEvent> {
    vec![
        imp(
            "A",
            datetime!(2025-02-01 00:00:00 UTC),
            EventPayload::Acquire(Acquire {
                sat: 100_000,
                usd_cost: dec!(50.00),
                fee_usd: dec!(0),
                basis_source: BasisSource::ExchangeProvided,
            }),
        ),
        imp(
            "B",
            datetime!(2025-03-01 00:00:00 UTC),
            EventPayload::Acquire(Acquire {
                sat: 100_000,
                usd_cost: dec!(90.00),
                fee_usd: dec!(0),
                basis_source: BasisSource::ExchangeProvided,
            }),
        ),
        imp(
            "C",
            datetime!(2025-04-01 00:00:00 UTC),
            EventPayload::Acquire(Acquire {
                sat: 100_000,
                usd_cost: dec!(40.00),
                fee_usd: dec!(0),
                basis_source: BasisSource::ExchangeProvided,
            }),
        ),
    ]
}

/// The whole ledger: three lots, a FIFO standing order, the 10:00 sale, and one UNATTESTED
/// `LotSelection` for lot B recorded at `made`.
///
/// `with_election` picks which of the reviewer's two defect rows this is:
/// - `true`  — a FIFO election is in force, so the drop falls back to FIFO ($50) and the ONLY
///   advisory is `LotSelectionPostHoc`.
/// - `false` — no election, so the drop additionally raises `IdentificationDefaulted` (FR-34) and the
///   report would say "identified contemporaneously" and "identified by NOTHING" in one breath.
fn ledger(made: OffsetDateTime, with_election: bool) -> Vec<LedgerEvent> {
    let mut evs = three_lots();
    if with_election {
        evs.push(dec_ev(
            1,
            datetime!(2025-01-02 00:00:00 UTC),
            EventPayload::MethodElection(MethodElection {
                effective_from: date!(2025 - 01 - 02),
                method: LotMethod::Fifo,
                wallet: None,
            }),
        ));
    }
    evs.push(imp(
        "D",
        SALE_AT,
        EventPayload::Dispose(Dispose {
            sat: 100_000,
            usd_proceeds: dec!(95.00),
            fee_usd: dec!(0),
            kind: DisposeKind::Sell,
        }),
    ));
    evs.push(dec_ev(
        2,
        made,
        EventPayload::LotSelection(LotSelection {
            disposal_event: EventId::import(Source::Coinbase, SourceRef::new("D")),
            lots: vec![LotPick {
                lot: pid("B"),
                sat: 100_000,
            }],
            attested: false,
        }),
    ));
    evs
}

/// What the FOLD did (the filed number) and what the REPORT says, for one made-instant.
struct Verdicts {
    /// `true` when the §A.4 timeliness pass dropped the selection (`LotSelectionPostHoc` raised).
    fold_dropped: bool,
    /// The basis the disposal actually reports — $90.00 if the selection governed, $50.00 (FIFO) if
    /// it was dropped.
    basis: rust_decimal::Decimal,
    /// The §A.5 status `verify` prints for that disposal.
    reported: ComplianceStatus,
}

fn verdicts(made: OffsetDateTime, with_election: bool) -> Verdicts {
    let evs = ledger(made, with_election);
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    let dc = disposal_compliance(&evs, &st);
    assert_eq!(dc.len(), 1, "fixture must produce exactly one §A.5 row");
    Verdicts {
        fold_dropped: st
            .blockers
            .iter()
            .any(|b| b.kind == BlockerKind::LotSelectionPostHoc),
        basis: st.disposals[0].legs[0].basis,
        reported: dc[0].status.clone(),
    }
}

// ── The I-1 reproduction ─────────────────────────────────────────────────────────────────────────

/// ★★★ **I-1, row 1 of the reviewer's probe.** A 10:00 sale, an unattested 17:00 same-day selection,
/// a FIFO election in force. The fold rejects the selection and files the $50 FIFO basis; before the
/// fix `disposal_compliance` called that same disposal `Contemporaneous`.
///
/// Pins the COMPLIANCE site. Re-narrowing `identification_made` (or the made-instant
/// `disposal_compliance` hands it) back to `TaxDate` makes 17:00 compare equal to 10:00 and reds here.
#[test]
fn a_same_day_late_selection_the_fold_dropped_is_never_reported_contemporaneous() {
    let v = verdicts(datetime!(2025-07-01 17:00:00 UTC), true);
    assert!(
        v.fold_dropped,
        "precondition: the §A.4 pass must have dropped the 17:00 selection against the 10:00 sale"
    );
    assert_eq!(
        v.basis,
        dec!(50.00),
        "precondition: the filed basis is the FIFO order in force, not the cherry-picked $90 lot"
    );
    assert_eq!(
        v.reported,
        ComplianceStatus::NonCompliant,
        "the §A.5 verdict must agree with the fold that rejected the selection — SPEC \
         §Cross-cutting forbids ANY artifact, command or doc describing post-hoc selection as \
         compliant, and `verify` printed `contemporaneous` for this disposal"
    );
}

/// ★★★ **I-1, row 2 — the sharpest contradiction.** Same window, no election. The fold drops the
/// selection AND raises `IdentificationDefaulted`, so before the fix one `verify` run said both
/// "identified contemporaneously" and "identified by NOTHING — default applied".
#[test]
fn with_no_election_the_double_advisory_and_the_verdict_cannot_contradict_each_other() {
    let evs = ledger(datetime!(2025-07-01 17:00:00 UTC), false);
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    let kinds: Vec<BlockerKind> = st.blockers.iter().map(|b| b.kind).collect();
    assert!(
        kinds.contains(&BlockerKind::LotSelectionPostHoc),
        "precondition: the late selection is dropped"
    );
    assert!(
        kinds.contains(&BlockerKind::IdentificationDefaulted),
        "precondition: with the selection gone and no election, FR-34 discloses the default"
    );
    let dc = disposal_compliance(&evs, &st);
    assert_eq!(
        dc[0].status,
        ComplianceStatus::NonCompliant,
        "the report cannot say `contemporaneous` about a disposal it is simultaneously telling the \
         filer was identified by nothing at all"
    );
}

/// Control — the case the reg's own Example 1 describes (§1.1012-1(j)(5)(i)(A)): a notation made on
/// the day of the sale and PRIOR to it. Timely at both granularities; the selection governs and the
/// verdict is `Contemporaneous`. Guards the fix against over-correction: the answer here must not
/// change.
#[test]
fn a_same_day_early_selection_still_governs_and_still_reports_contemporaneous() {
    let v = verdicts(datetime!(2025-07-01 09:00:00 UTC), true);
    assert!(!v.fold_dropped);
    assert_eq!(v.basis, dec!(90.00));
    assert_eq!(v.reported, ComplianceStatus::Contemporaneous);
}

/// Control — a day later is late under BOTH granularities, so the three sites agreed here even
/// before the fix. Keeps the fix from being read as "the drop got weaker".
#[test]
fn a_next_day_selection_is_late_under_either_granularity() {
    let v = verdicts(datetime!(2025-07-02 00:00:00 UTC), true);
    assert!(v.fold_dropped);
    assert_eq!(v.basis, dec!(50.00));
    assert_eq!(v.reported, ComplianceStatus::NonCompliant);
}

/// ★★ **The guarantee itself, stated as one assertion over the whole same-day window.** Not "each
/// site is instant-granular" — that is what the per-site tests pin — but "the number the fold FILED
/// and the verdict the tool REPORTS are one computation", which is the sentence FR-34 put in
/// `identification_made`'s doc comment and which this range made false.
///
/// The equal-instant row (10:00:00 exactly) is deliberate: §1.1012-1(j)(2) says "no later than", so
/// the boundary is inclusive, and a fix that reached for `<` instead of `<=` reds here.
#[test]
fn the_fold_and_the_reported_verdict_agree_at_every_instant_of_the_sale_day() {
    let cases: [(OffsetDateTime, bool); 5] = [
        (datetime!(2025-07-01 09:00:00 UTC), true), // before  → timely
        (datetime!(2025-07-01 10:00:00 UTC), true), // exactly → timely ("no later than")
        (datetime!(2025-07-01 10:00:01 UTC), false), // one second after → late
        (datetime!(2025-07-01 17:00:00 UTC), false), // seven hours after → late
        (datetime!(2025-07-02 00:00:00 UTC), false), // next day → late
    ];
    for (made, expect_timely) in cases {
        let v = verdicts(made, true);
        assert_eq!(
            !v.fold_dropped, expect_timely,
            "the FOLD disagreed about a selection made {made} against a {SALE_AT} sale"
        );
        let reported_timely = v.reported == ComplianceStatus::Contemporaneous;
        assert_eq!(
            reported_timely, expect_timely,
            "the REPORT disagreed about a selection made {made} against a {SALE_AT} sale"
        );
        assert_eq!(
            v.fold_dropped, !reported_timely,
            "★ the fold applied the selection and the report rejected it (or vice versa) for a \
             selection made {made} — the filed number and the printed verdict must be ONE \
             computation (FR-34), and they are one predicate (I-1)"
        );
        assert_eq!(
            v.basis,
            if expect_timely {
                dec!(90.00)
            } else {
                dec!(50.00)
            },
            "and the FILED basis must follow the same verdict, for a selection made {made}"
        );
    }
}

// ── The third site: `optimize` ───────────────────────────────────────────────────────────────────
//
// The same 10:00-sale / 17:00-selection window, pinned at the two `optimize` entry points. These are
// pure — no fold, no tax tables — and that is exactly what makes them per-site kills: they red when
// `optimize.rs` narrows, and only then.

/// The disposition the two gates below judge against: the same 10:00 sale, self-custody, 2025 — well
/// inside the own-books envelope, so the 2027+ broker branch cannot short-circuit ahead of the
/// timeliness lever and answer for it.
fn sale_moment() -> DispositionMoment {
    DispositionMoment {
        date: date!(2025 - 07 - 01),
        at: SALE_AT,
    }
}

/// ★★★ **I-1 at `optimize::persistability` — and this gate is a WRITE, not advice.**
///
/// `optimize accept` reads `ContemporaneousNow` and persists an UNATTESTED `LotSelection`. While this
/// compared `TaxDate`s, a filer who sold at 10:00 and ran `optimize` that afternoon was told the pick
/// was "persistable now (made ≤ sale → Contemporaneous)" — and the very next projection dropped what
/// `accept` had just written, as `LotSelectionPostHoc`. The tool's claim and its own next action
/// disagreed about the same selection.
///
/// Re-narrowing `persistability` reds HERE.
#[test]
fn persistability_of_a_same_day_late_pick_needs_attestation() {
    assert_eq!(
        persistability(&cold(), sale_moment(), datetime!(2025-07-01 17:00:00 UTC)),
        Persistability::NeedsAttestation,
        "a pick made seven hours after the sale is post-hoc: `accept` must demand the §C.2 \
         attestation, not write it unattested for the next projection to drop"
    );
    // Control, one second before the sale: still freely persistable. A "fix" that simply demanded an
    // attestation everywhere would red here.
    assert_eq!(
        persistability(&cold(), sale_moment(), datetime!(2025-07-01 09:59:59 UTC)),
        Persistability::ContemporaneousNow
    );
    // The inclusive boundary — §1.1012-1(j)(2) says "no later than", so the exact instant is timely.
    assert_eq!(
        persistability(&cold(), sale_moment(), SALE_AT),
        Persistability::ContemporaneousNow
    );
}

/// ★★★ **I-1 at `optimize::proposed_compliance_status` — what `optimize` PRINTS.**
///
/// The SPEC's load-bearing cross-cutting rule ("no artifact, command, or doc may describe post-hoc
/// selection as compliant", `design/SPEC_lot_optimization_program.md:218`) binds a printed status
/// exactly as it binds a filed figure. Comparing dates printed `Contemporaneous` for a proposal made
/// hours after the sale it names.
///
/// Re-narrowing `proposed_compliance_status` reds HERE.
#[test]
fn proposed_status_of_a_same_day_late_divergent_pick_is_noncompliant() {
    let proposed = [LotPick {
        lot: pid("B"),
        sat: 100_000,
    }];
    let current = [LotPick {
        lot: pid("A"),
        sat: 100_000,
    }];
    assert_eq!(
        proposed_compliance_status(
            &cold(),
            sale_moment(),
            datetime!(2025-07-01 17:00:00 UTC),
            &proposed,
            &current,
            // A standing order as the baseline, so this re-pins R0-C2 at the same time: it may NEVER
            // rescue a divergent post-hoc pick.
            &ComplianceStatus::StandingOrder {
                effective_from: date!(2025 - 01 - 02)
            },
        ),
        ComplianceStatus::NonCompliant,
        "a divergent pick proposed seven hours after the sale is a post-hoc cherry-pick; \
         `optimize` may not print `contemporaneous` for it"
    );
    // Control: proposed before the sale on the same day → genuinely contemporaneous.
    assert_eq!(
        proposed_compliance_status(
            &cold(),
            sale_moment(),
            datetime!(2025-07-01 09:00:00 UTC),
            &proposed,
            &current,
            &ComplianceStatus::NonCompliant,
        ),
        ComplianceStatus::Contemporaneous
    );
}

/// ★★ **The cross-site statement, as one assertion.** Not "each site is instant-granular" — the tests
/// above pin that individually — but that `resolve`'s drop, the §A.5 verdict, the status `optimize`
/// prints and the gate `accept` writes through answer the SAME question the same way at every instant
/// of the sale day, against the shared predicate itself. That is the FR-34 sentence ("the reported
/// verdict and the filed number can never diverge") restated as something executable.
///
/// ★ **What it does and does NOT catch, measured, not assumed.** In the B1 sweep it reds on a
/// narrowing of ANY ONE site (M1–M4), because that site then disagrees with the other three. It goes
/// GREEN on a narrowing of `identification_is_timely` ITSELF (M5) — every site moves together, so they
/// still agree — and the per-site tests above are what red there. The two kinds are complementary and
/// neither is redundant: this one measures AGREEMENT, they measure CORRECTNESS. A reader who assumes
/// this test subsumes them will delete the wrong ones.
#[test]
fn all_four_sites_answer_the_timeliness_question_identically() {
    let proposed = [LotPick {
        lot: pid("B"),
        sat: 100_000,
    }];
    let current = [LotPick {
        lot: pid("A"),
        sat: 100_000,
    }];
    for made in [
        datetime!(2025-07-01 00:00:00 UTC),
        datetime!(2025-07-01 09:59:59 UTC),
        SALE_AT,
        datetime!(2025-07-01 10:00:01 UTC),
        datetime!(2025-07-01 17:00:00 UTC),
        datetime!(2025-07-01 23:59:59 UTC),
        datetime!(2025-07-02 00:00:00 UTC),
    ] {
        let timely = btctax_core::project::identification_is_timely(made, SALE_AT);
        let v = verdicts(made, true);
        assert_eq!(!v.fold_dropped, timely, "resolve's §A.4 drop, at {made}");
        assert_eq!(
            v.reported == ComplianceStatus::Contemporaneous,
            timely,
            "the §A.5 verdict `verify` prints, at {made}"
        );
        assert_eq!(
            persistability(&cold(), sale_moment(), made) == Persistability::ContemporaneousNow,
            timely,
            "the `optimize accept` write gate, at {made}"
        );
        assert_eq!(
            proposed_compliance_status(
                &cold(),
                sale_moment(),
                made,
                &proposed,
                &current,
                &ComplianceStatus::NonCompliant,
            ) == ComplianceStatus::Contemporaneous,
            timely,
            "the status `optimize` prints, at {made}"
        );
    }
}
