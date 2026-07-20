//! Shared synthetic corpora + per-journey fixtures for the worked-example journeys (J1–J9).
//!
//! Hoisted here (from xtask, TUI-walkthrough spec §4.1) so ALL consumers share ONE source of truth: the
//! `xtask` examples generator, and the `btctax-tui` / `btctax-tui-edit` screen-walkthrough emit tests. A
//! plain `pub` module (like `btctax_forms::testonly`) — the corpora are tiny synthetic CSVs, and the
//! generator's `generate()` is a non-test fn, so they cannot be `#[cfg(test)]`.
//!
//! The CSV corpora carry explicit CRLF: committed `.csv` files are force-LF'd by `.gitattributes` and would
//! break the Coinbase parser, so they live as string consts (a driver writes them to a tempdir at runtime).
//! These bytes are byte-identical to their previous home in `xtask/src/examples.rs` — the
//! `examples_golden_matches_committed` gate proves the move changed nothing.

// ── Synthetic corpora (explicit CRLF) ──────────────────────────────────────────────────────────────

/// J1 corpus: a single-buyer happy path — one Buy + one partial Sell in 2025.
pub const J1_CSV: &str = "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
cb-buy,2025-03-01 12:00:00 UTC,Buy,BTC,0.10000000,USD,84000.00,8400.00,8450.00,50.00,,,\r\n\
cb-sell,2025-06-15 12:00:00 UTC,Sell,BTC,0.02000000,USD,67500.00,1350.00,1340.00,10.00,,,\r\n";

/// J2 corpus: an LT lot (2023) + an ST lot (2025) + a 2025 Send of 2 BTC donated to charity.
pub const J2_CSV: &str = "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
cb-buy-lt,2023-06-01 12:00:00 UTC,Buy,BTC,1.00000000,USD,5000.00,5000.00,5000.00,0.00,,,\r\n\
cb-buy-st,2025-03-01 12:00:00 UTC,Buy,BTC,1.00000000,USD,2000.00,2000.00,2000.00,0.00,,,\r\n\
cb-donate,2025-09-01 12:00:00 UTC,Send,BTC,2.00000000,USD,108996.17,,,,,,bc1qcharity\r\n";

/// J3 corpus: a Buy + a Receive (an inbound transfer with unknown basis → a hard blocker until classified).
pub const J3_CSV: &str = "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
cb-buy,2025-02-01 12:00:00 UTC,Buy,BTC,0.50000000,USD,95000.00,47500.00,47550.00,50.00,,,\r\n\
cb-recv,2025-08-01 12:00:00 UTC,Receive,BTC,0.20000000,USD,110000.00,,,,,,\r\n";

/// J4 corpus: two River staking-income deposits in 2025 (FMV resolved from the bundled dataset).
pub const J4_CSV: &str =
    "Date,Sent Amount,Sent Currency,Received Amount,Received Currency,Fee Amount,Tag\r\n\
2025-04-15 12:00:00 UTC,,,0.05000000,BTC,,income\r\n\
2025-05-20 12:00:00 UTC,,,0.03000000,BTC,,income\r\n";

/// J5 corpus: an LT lot + a higher-basis ST lot + a 2025 sell — a genuine changed-selection scenario
/// (HIFO ≠ FIFO) so the optimizer has a tax-saving pick to propose.
pub const J5_CSV: &str = "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
opt-buy-lt,2023-01-01 12:00:00 UTC,Buy,BTC,1.00000000,USD,30000.00,30000.00,30000.00,0.00,,,\r\n\
opt-buy-st,2025-01-02 12:00:00 UTC,Buy,BTC,1.00000000,USD,80000.00,80000.00,80000.00,0.00,,,\r\n\
opt-sell,2025-06-01 12:00:00 UTC,Sell,BTC,1.00000000,USD,50000.00,50000.00,50000.00,0.00,,,\r\n";

/// J6 River corpus: one small 2024 business mining-income deposit (FMV from the bundled dataset).
/// Kept modest deliberately: the kitchen-sink household clears the 2024 Form-6251 AMT-screen worksheet by
/// only a thin margin — a corpus editor who enlarges the sale, income, or donation must keep the household
/// on the computable side of that screen.
pub const J6_RIVER_CSV: &str =
    "Date,Sent Amount,Sent Currency,Received Amount,Received Currency,Fee Amount,Tag\r\n\
2024-03-15 12:00:00 UTC,,,0.05000000,BTC,,income\r\n";

/// J6 Coinbase corpus: a cheap 2020 long-term lot, a small 2024 long-term sale (Schedule D Part II / Form
/// 8949), and a 2024 charitable Send of 0.1 BTC (§170(e) donation ⇒ Form 8283). Amounts kept small so the
/// return stays under the AMT screen.
pub const J6_COINBASE_CSV: &str = "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
cb-buy,2020-01-01 12:00:00 UTC,Buy,BTC,0.30000000,USD,30000.00,9000.00,9000.00,0.00,,,\r\n\
cb-sell,2024-05-01 12:00:00 UTC,Sell,BTC,0.05000000,USD,63000.00,3150.00,3130.00,20.00,,,\r\n\
cb-donate,2024-09-01 12:00:00 UTC,Send,BTC,0.10000000,USD,60000.00,,,,,,bc1qcharity\r\n";

/// The committed full-return ReturnInputs (the `kitchen_sink_household()` oracle, TOML-serialized). J6
/// imports it via `income import`. The fixture lives in this crate (self-contained), so this is a
/// SAME-crate `include_str!` — retiring the cross-crate include the xtask copy carried (M-5 exception).
pub const J6_FULLRETURN_TOML: &str =
    include_str!("../tests/fixtures/examples/fullreturn_inputs.toml");

/// J7 corpus (UX-P1-7): a single 2024 Coinbase Receive of staking rewards — an unknown-basis inbound the
/// single-event `classify-inbound-income` command values only from a hand-supplied `--fmv`.
pub const J7_CSV: &str = "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
cb-recv,2024-06-15 12:00:00 UTC,Receive,BTC,0.05000000,USD,,,,,,,\r\n";

/// J8 River corpus (UX-P1-8): a buy (to give the coins a basis) then a Withdrawal OUT of 0.10 BTC — the
/// out-leg of a cross-exchange self-transfer whose in-leg lands on Coinbase below.
pub const J8_RIVER_CSV: &str =
    "Date,Sent Amount,Sent Currency,Received Amount,Received Currency,Fee Amount,Tag\r\n\
2025-01-05 12:00:00 UTC,4000.00,USD,0.10000000,BTC,,buy\r\n\
2025-03-10 12:00:00 UTC,0.10000000,BTC,,,,withdrawal\r\n";

/// J8 Coinbase corpus (UX-P1-8): the matching inbound Receive of 0.10 BTC — the SAME coins landing at a
/// second exchange, so the pair is a cross-wallet RELOCATE (not a same-wallet DROP).
pub const J8_COINBASE_CSV: &str = "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
cb-recv,2025-03-10 12:00:00 UTC,Receive,BTC,0.10000000,USD,,,,,,,\r\n";

/// J9 corpus (UX-P1-10): a cheap 2023 long-term lot (0.60) + a pricier 2024 lot (0.40), then a 2025 sale of
/// only 0.50 — smaller than either combined holding, so which lots cover it is a GENUINE choice.
pub const J9_CSV: &str = "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
lot-a,2023-01-01 12:00:00 UTC,Buy,BTC,0.60000000,USD,25000.00,15000.00,15000.00,0.00,,,\r\n\
lot-b,2024-01-02 12:00:00 UTC,Buy,BTC,0.40000000,USD,60000.00,24000.00,24000.00,0.00,,,\r\n\
sale,2025-06-01 12:00:00 UTC,Sell,BTC,0.50000000,USD,47500.00,47500.00,47500.00,0.00,,,\r\n";

// ── Per-journey fixtures ───────────────────────────────────────────────────────────────────────────

/// One journey's synthetic input: the named corpus files a driver writes to a tempdir before `import`.
/// The SAME fixture seeds both walkthrough capture halves (the editor emit test drives its flows; the
/// viewer emit test re-seeds this corpus and replays the equivalent decisions via `btctax-cli`), so the
/// two halves of a journey converge by construction on one source of truth (walkthrough spec §4.2).
pub struct JourneyFixture {
    /// Short journey id, e.g. `"j8"`.
    pub name: &'static str,
    /// `(filename, CRLF content)` pairs written to the tempdir before ingest.
    pub corpus: &'static [(&'static str, &'static str)],
}

/// The nine journey fixtures, keyed by the same corpus filenames the examples journeys use.
pub fn j1() -> JourneyFixture {
    JourneyFixture {
        name: "j1",
        corpus: &[("coinbase.csv", J1_CSV)],
    }
}
pub fn j2() -> JourneyFixture {
    JourneyFixture {
        name: "j2",
        corpus: &[("coinbase.csv", J2_CSV)],
    }
}
pub fn j3() -> JourneyFixture {
    JourneyFixture {
        name: "j3",
        corpus: &[("coinbase.csv", J3_CSV)],
    }
}
pub fn j4() -> JourneyFixture {
    JourneyFixture {
        name: "j4",
        corpus: &[("river.csv", J4_CSV)],
    }
}
pub fn j5() -> JourneyFixture {
    JourneyFixture {
        name: "j5",
        corpus: &[("coinbase.csv", J5_CSV)],
    }
}
pub fn j6() -> JourneyFixture {
    JourneyFixture {
        name: "j6",
        corpus: &[
            ("river.csv", J6_RIVER_CSV),
            ("coinbase.csv", J6_COINBASE_CSV),
        ],
    }
}
pub fn j7() -> JourneyFixture {
    JourneyFixture {
        name: "j7",
        corpus: &[("coinbase.csv", J7_CSV)],
    }
}
/// J8 — the PoC journey: a cross-exchange self-transfer (River Withdrawal out → Coinbase Receive in).
pub fn j8() -> JourneyFixture {
    JourneyFixture {
        name: "j8",
        corpus: &[
            ("river.csv", J8_RIVER_CSV),
            ("coinbase.csv", J8_COINBASE_CSV),
        ],
    }
}
pub fn j9() -> JourneyFixture {
    JourneyFixture {
        name: "j9",
        corpus: &[("coinbase.csv", J9_CSV)],
    }
}

// ── Seeders (shared so a journey's editor + viewer walkthrough halves converge on ONE vault state) ──
// These live HERE (btctax-cli) rather than in the TUI crates because the read-only viewer (btctax-tui)
// forbids the write-path `cmd::`/`save(` tokens even in its tests (its e10 source gate) — so the viewer
// half calls a testonly seeder to reach the post-decision state, then only Session::open + build_snapshot
// (read-only) + render.

/// Init a vault at `dir/vault.pgp` and import the journey `fx`'s corpus via the REAL adapter ingest.
pub fn seed_journey(
    dir: &std::path::Path,
    pp: &btctax_store::Passphrase,
    fx: &JourneyFixture,
) -> std::path::PathBuf {
    let vault = dir.join("vault.pgp");
    let key = dir.join("key.asc");
    crate::cmd::init::run(&vault, pp, &key).unwrap();
    let mut files: Vec<std::path::PathBuf> = Vec::new();
    for (name, content) in fx.corpus {
        let p = dir.join(name);
        std::fs::write(&p, content).unwrap();
        files.push(p);
    }
    crate::cmd::import::run(&vault, pp, &files).unwrap();
    vault
}

/// Seed J8 and apply its RELOCATE self-transfer (river out → coinbase in) — the post-match state the
/// walkthrough's VIEWER half renders (BALANCED). Refs are J8's deterministic event refs; RELOCATE routes
/// to `link_transfer` (out → in), the same decision the editor's confirm modal makes.
pub fn seed_j8_relocated(
    dir: &std::path::Path,
    pp: &btctax_store::Passphrase,
    now: time::OffsetDateTime,
) -> std::path::PathBuf {
    use btctax_core::event::TransferTarget;
    let vault = seed_journey(dir, pp, &j8());
    let in_id = crate::eventref::parse_event_id("import|coinbase|in|cb-recv").unwrap();
    crate::cmd::reconcile::link_transfer(
        &vault,
        pp,
        "import|river|out|1741608000000|withdrawal|10000000#0",
        TransferTarget::InEvent(in_id),
        now,
    )
    .unwrap();
    vault
}

/// Seed J1 (single buyer: one Buy + one partial Sell, no transfers → no reconciliation) and set the 2025
/// tax profile its walkthrough VIEWER needs for the Tax tab to compute — single filer, $100k ordinary
/// income, mirroring the J1 worked example's `tax-profile` step. The viewer half then only opens + reads.
pub fn seed_j1_with_profile(
    dir: &std::path::Path,
    pp: &btctax_store::Passphrase,
) -> std::path::PathBuf {
    use btctax_core::{Carryforward, FilingStatus, TaxProfile};
    use rust_decimal::Decimal;
    let vault = seed_journey(dir, pp, &j1());
    let z = Decimal::ZERO;
    let profile = TaxProfile {
        filing_status: FilingStatus::Single,
        ordinary_taxable_income: Decimal::from(100_000),
        magi_excluding_crypto: Decimal::from(100_000),
        qualified_dividends_and_other_pref_income: z,
        other_net_capital_gain: z,
        capital_loss_carryforward_in: Carryforward { short: z, long: z },
        w2_ss_wages: z,
        w2_medicare_wages: z,
        schedule_c_expenses: z,
    };
    crate::cmd::tax::set_profile(&vault, pp, 2025, profile, false).unwrap();
    vault
}

/// Seed J4 (two River staking receipts) with a 2025 profile AND both receipts reclassified as a trade or
/// business (`--business true --kind staking`) — the post-reclassify state the walkthrough's VIEWER half
/// renders (Income now on Schedule C/SE; Tax shows the self-employment tax). The editor half drives the
/// reclassify itself, so it seeds only the raw import (`seed_journey(&j4())`). Made-date pinned for
/// determinism. Refs embed the ms-timestamp of each receipt (not wall-clock).
pub fn seed_j4_reclassified(
    dir: &std::path::Path,
    pp: &btctax_store::Passphrase,
    now: time::OffsetDateTime,
) -> std::path::PathBuf {
    use btctax_core::{Carryforward, FilingStatus, IncomeKind, TaxProfile};
    use rust_decimal::Decimal;
    let vault = seed_journey(dir, pp, &j4());
    let z = Decimal::ZERO;
    let profile = TaxProfile {
        filing_status: FilingStatus::Single,
        ordinary_taxable_income: Decimal::from(100_000),
        magi_excluding_crypto: Decimal::from(100_000),
        qualified_dividends_and_other_pref_income: z,
        other_net_capital_gain: z,
        capital_loss_carryforward_in: Carryforward { short: z, long: z },
        w2_ss_wages: z,
        w2_medicare_wages: z,
        schedule_c_expenses: z,
    };
    crate::cmd::tax::set_profile(&vault, pp, 2025, profile, false).unwrap();
    // Made-date threaded from the caller so it matches the depicted editor session's pinned clock (J4
    // review Minor 2 — the J8 `seed_j8_relocated(…, now)` pattern), not a divergent hardcoded literal.
    for r in [
        "import|river|in|1744718400000|income|5000000#0",
        "import|river|in|1747742400000|income|3000000#0",
    ] {
        crate::cmd::reconcile::reclassify_income(
            &vault,
            pp,
            r,
            true,
            Some(IncomeKind::Staking),
            now,
        )
        .unwrap();
    }
    vault
}

/// Seed J7 (a single off-exchange Receive with no market price → a hard unknown-basis blocker) classified
/// as staking INCOME with a hand-supplied FMV ($3,300), plus a 2024 profile — the post-classify state the
/// walkthrough's VIEWER renders (Income + Tax). The editor half seeds only the raw import and drives the
/// classify. Made-date threaded from the caller (matches the depicted editor session's clock).
pub fn seed_j7_income(
    dir: &std::path::Path,
    pp: &btctax_store::Passphrase,
    now: time::OffsetDateTime,
) -> std::path::PathBuf {
    use btctax_core::{Carryforward, FilingStatus, InboundClass, IncomeKind, TaxProfile};
    use rust_decimal::Decimal;
    let vault = seed_journey(dir, pp, &j7());
    let z = Decimal::ZERO;
    let profile = TaxProfile {
        filing_status: FilingStatus::Single,
        ordinary_taxable_income: Decimal::from(100_000),
        magi_excluding_crypto: Decimal::from(100_000),
        qualified_dividends_and_other_pref_income: z,
        other_net_capital_gain: z,
        capital_loss_carryforward_in: Carryforward { short: z, long: z },
        w2_ss_wages: z,
        w2_medicare_wages: z,
        schedule_c_expenses: z,
    };
    crate::cmd::tax::set_profile(&vault, pp, 2024, profile, false).unwrap();
    let fmv: Decimal = "3300.00".parse().unwrap();
    crate::cmd::reconcile::classify_inbound(
        &vault,
        pp,
        "import|coinbase|in|cb-recv",
        InboundClass::Income {
            kind: IncomeKind::Staking,
            fmv: Some(fmv),
            business: false,
        },
        now,
    )
    .unwrap();
    vault
}

/// Seed J3 (an unknown-basis inbound Receive → a hard blocker) classified as a SELF-TRANSFER of the
/// filer's own coins returning — non-taxable, carrying a supplied basis ($19,000) and acquisition date
/// (2024-11-01, so the holding period runs from then). The post-classify state the VIEWER renders (Holdings).
/// The editor half seeds only the raw import and drives the classify. Made-date threaded from the caller.
pub fn seed_j3_self_transfer(
    dir: &std::path::Path,
    pp: &btctax_store::Passphrase,
    now: time::OffsetDateTime,
) -> std::path::PathBuf {
    use btctax_core::InboundClass;
    use rust_decimal::Decimal;
    let vault = seed_journey(dir, pp, &j3());
    let basis: Decimal = "19000.00".parse().unwrap();
    crate::cmd::reconcile::classify_inbound(
        &vault,
        pp,
        "import|coinbase|in|cb-recv",
        InboundClass::SelfTransferMine {
            basis: Some(basis),
            acquired_at: Some(time::macros::date!(2024 - 11 - 01)),
        },
        now,
    )
    .unwrap();
    vault
}

/// Seed J9 (a cheap 2023 LT lot 0.60 + a pricier 2024 lot 0.40, then a 2025 sale of 0.50 — less than
/// either holding, so which lots cover it is a genuine choice) with the whole 0.50 IDENTIFIED against the
/// cheap long-term `lot-a` (a deliberate per-disposal specific identification). The post-selection state
/// the walkthrough's VIEWER renders (Disposals drawing from lot-a; Compliance now satisfied). The editor
/// half seeds only the raw import and drives the select-lots flow. Made-date threaded from the caller
/// (the identification is contemporaneous — before/at the sale).
pub fn seed_j9_selected(
    dir: &std::path::Path,
    pp: &btctax_store::Passphrase,
    now: time::OffsetDateTime,
) -> std::path::PathBuf {
    let vault = seed_journey(dir, pp, &j9());
    let pick = crate::eventref::parse_lot_pick("import|coinbase|trade|lot-a#0:50000000").unwrap();
    crate::cmd::reconcile::select_lots(&vault, pp, "import|coinbase|trade|sale", vec![pick], now)
        .unwrap();
    vault
}

/// Seed J2 with the outbound Send reclassified as a §170(e) charitable DONATION (FMV $217,992.34 = 2 BTC
/// × the contribution-date close, donee "Habitat for Humanity") — but WITHOUT the Form 8283 appraiser/donee
/// details yet. This is the state the walkthrough's `d` (set-donation-details) EDITOR frame drives from
/// (the details form still empty). Made-date threaded from the caller.
pub fn seed_j2_reclassified(
    dir: &std::path::Path,
    pp: &btctax_store::Passphrase,
    now: time::OffsetDateTime,
) -> std::path::PathBuf {
    use btctax_core::OutflowClass;
    let vault = seed_journey(dir, pp, &j2());
    let principal: rust_decimal::Decimal = "217992.34".parse().unwrap();
    crate::cmd::reconcile::reclassify_outflow(
        &vault,
        pp,
        "import|coinbase|out|cb-donate",
        OutflowClass::Donate {
            appraisal_required: false,
        },
        principal,
        None,
        Some("Habitat for Humanity".to_string()),
        now,
    )
    .unwrap();
    vault
}

/// Seed J2's fully post-decision state — the outbound reclassified as a donation, the Form 8283
/// appraiser/donee details recorded (Section B complete), and a 2025 profile so the viewer's Tax tab
/// computes the charitable-deduction line. The walkthrough's VIEWER renders Forms (the two 8283 rows) +
/// Tax. Made-date threaded from the caller.
pub fn seed_j2_donated(
    dir: &std::path::Path,
    pp: &btctax_store::Passphrase,
    now: time::OffsetDateTime,
) -> std::path::PathBuf {
    use btctax_core::{Carryforward, DonationDetails, FilingStatus, TaxProfile};
    use rust_decimal::Decimal;
    let vault = seed_j2_reclassified(dir, pp, now);
    let z = Decimal::ZERO;
    let profile = TaxProfile {
        filing_status: FilingStatus::Single,
        ordinary_taxable_income: Decimal::from(100_000),
        magi_excluding_crypto: Decimal::from(100_000),
        qualified_dividends_and_other_pref_income: z,
        other_net_capital_gain: z,
        capital_loss_carryforward_in: Carryforward { short: z, long: z },
        w2_ss_wages: z,
        w2_medicare_wages: z,
        schedule_c_expenses: z,
    };
    crate::cmd::tax::set_profile(&vault, pp, 2025, profile, false).unwrap();
    crate::cmd::reconcile::set_donation_details(
        &vault,
        pp,
        "import|coinbase|out|cb-donate",
        DonationDetails {
            donee_name: "Habitat for Humanity".into(),
            donee_address: None,
            donee_ein: Some("98-7654321".into()),
            appraiser_name: "Jane Appraiser".into(),
            appraiser_address: None,
            appraiser_tin: Some("12-3456789".into()),
            appraiser_ptin: None,
            appraiser_qualifications: Some("ASA-accredited digital-asset appraiser, 8 yrs".into()),
            appraisal_date: Some(time::macros::date!(2025 - 09 - 15)),
            fmv_method_override: None,
        },
    )
    .unwrap();
    vault
}

/// Seed J5 (an LT lot $30k + a higher-basis ST lot $80k + a 2025 sale $50k) with a 2025 profile AND a
/// standing FIFO election — but NOT the optimizer's accept. This is the state the walkthrough's `z`
/// (optimize-accept) EDITOR frames drive from: the optimizer proposes swapping to the ST lot to realize a
/// loss. The election made-date is hardcoded 2025-01-01 (the engine blocks effective_from before the
/// made-date, so a later threaded `now` would trip MethodElectionBackdated).
pub fn seed_j5_elected(dir: &std::path::Path, pp: &btctax_store::Passphrase) -> std::path::PathBuf {
    use btctax_core::{Carryforward, FilingStatus, LotMethod, TaxProfile};
    use rust_decimal::Decimal;
    let vault = seed_journey(dir, pp, &j5());
    let z = Decimal::ZERO;
    let profile = TaxProfile {
        filing_status: FilingStatus::Single,
        ordinary_taxable_income: Decimal::from(100_000),
        magi_excluding_crypto: Decimal::from(100_000),
        qualified_dividends_and_other_pref_income: z,
        other_net_capital_gain: z,
        capital_loss_carryforward_in: Carryforward { short: z, long: z },
        w2_ss_wages: z,
        w2_medicare_wages: z,
        schedule_c_expenses: z,
    };
    crate::cmd::tax::set_profile(&vault, pp, 2025, profile, false).unwrap();
    crate::cmd::reconcile::set_forward_method(
        &vault,
        pp,
        LotMethod::Fifo,
        None,
        Some(time::macros::date!(2025 - 01 - 01)),
        time::macros::datetime!(2025 - 01 - 01 00:00:00 UTC),
    )
    .unwrap();
    vault
}

/// Seed J5's post-accept state — the FIFO election PLUS the optimizer's proposed lot pick accepted (the
/// whole sale moved onto the short-term `opt-buy-st` lot to realize a loss). The walkthrough's VIEWER
/// renders the resulting Disposals (ST loss) + Tax (§1211 deduction + carryforward). `now` is the accept
/// made-date — MUST be ≤ the 2025-06-01 sale so the selection is Contemporaneous; the caller passes the
/// same instant the editor frames pin.
pub fn seed_j5_optimized(
    dir: &std::path::Path,
    pp: &btctax_store::Passphrase,
    now: time::OffsetDateTime,
) -> std::path::PathBuf {
    let vault = seed_j5_elected(dir, pp);
    crate::cmd::optimize::accept(&vault, pp, 2025, None, None, now).unwrap();
    vault
}

/// Seed J6 — a COMPLETE Form 1040 (the kitchen-sink MFJ household). Combines the crypto reconciliation
/// (mining income → a trade or business; the 0.10 BTC Send → a §170(e) donation with Form 8283 details)
/// with the non-crypto household imported from the committed `kitchen_sink_household()` TOML. This is the
/// state BOTH walkthrough halves render: the editor half opens the tax-inputs authoring form (`T`) over
/// these committed inputs; the viewer shows Forms (the crypto forms) + Tax (the merged MFJ return). No raw
/// tax-profile — the full-return inputs take precedence and `build_snapshot` derives the MFJ profile from
/// them. Made-date threaded from the caller.
pub fn seed_j6_full(
    dir: &std::path::Path,
    pp: &btctax_store::Passphrase,
    now: time::OffsetDateTime,
) -> std::path::PathBuf {
    use btctax_core::{DonationDetails, IncomeKind, OutflowClass};
    let vault = seed_journey(dir, pp, &j6());
    // 1. mining income → a trade or business (Schedule C/SE).
    crate::cmd::reconcile::reclassify_income(
        &vault,
        pp,
        "import|river|in|1710504000000|income|5000000#0",
        true,
        Some(IncomeKind::Mining),
        now,
    )
    .unwrap();
    // 2. the 0.10 BTC Send → a §170(e) charitable donation, FMV $6,000.
    let fmv: rust_decimal::Decimal = "6000.00".parse().unwrap();
    crate::cmd::reconcile::reclassify_outflow(
        &vault,
        pp,
        "import|coinbase|out|cb-donate",
        OutflowClass::Donate {
            appraisal_required: false,
        },
        fmv,
        None,
        Some("Habitat for Humanity".to_string()),
        now,
    )
    .unwrap();
    // 3. the Form 8283 Section-B appraiser/donee details.
    crate::cmd::reconcile::set_donation_details(
        &vault,
        pp,
        "import|coinbase|out|cb-donate",
        DonationDetails {
            donee_name: "Habitat for Humanity".into(),
            donee_address: None,
            donee_ein: Some("98-7654321".into()),
            appraiser_name: "Jane Appraiser".into(),
            appraiser_address: None,
            appraiser_tin: Some("12-3456789".into()),
            appraiser_ptin: None,
            appraiser_qualifications: Some("ASA-accredited digital-asset appraiser, 8 yrs".into()),
            appraisal_date: Some(time::macros::date!(2024 - 09 - 15)),
            fmv_method_override: None,
        },
    )
    .unwrap();
    // 4. the non-crypto household — a programmatic `income import` (reads a file, so materialize the TOML).
    let toml = dir.join("fullreturn.toml");
    std::fs::write(&toml, J6_FULLRETURN_TOML).unwrap();
    crate::cmd::tax::import_return_inputs(&vault, pp, 2024, &toml).unwrap();
    vault
}
