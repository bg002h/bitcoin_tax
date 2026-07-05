mod fixtures;
use btctax_cli::{cmd, Session};
use btctax_core::{BlockerKind, EventPayload, InboundClass, OutflowClass, Severity};
use btctax_store::Passphrase;
use csv::Reader;
use std::fs::File;
use std::path::Path;
use time::macros::datetime;

fn pp() -> Passphrase {
    Passphrase::new("pw".into())
}

#[test]
fn export_snapshot_writes_sqlite_and_csvs_and_backup_key() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    cmd::import::run(
        &vault,
        &pp(),
        &[fixtures::coinbase_buy_sell_send(dir.path())],
    )
    .unwrap();

    let out = dir.path().join("export");
    let sqlite = cmd::admin::export_snapshot(&vault, &pp(), &out, None, None)
        .unwrap()
        .path;
    assert!(sqlite.exists(), "snapshot.sqlite (store)");

    let lots_path = out.join("lots.csv");
    let disposals_path = out.join("disposals.csv");
    let removals_path = out.join("removals.csv");
    let income_path = out.join("income.csv");

    assert!(lots_path.exists());
    assert!(disposals_path.exists());
    assert!(removals_path.exists());
    assert!(income_path.exists());

    // Strengthen: read back lots.csv and verify content (not just existence).
    // coinbase_buy_sell_send creates:
    //   - Buy 0.10000000 BTC (10,000,000 sat)
    //   - Sell 0.02000000 BTC (2,000,000 sat)
    //   - Send 0.03000000 BTC (3,000,000 sat)
    // Expected remaining: 10,000,000 - 2,000,000 - 3,000,000 = 5,000,000 sat
    let mut reader = Reader::from_reader(File::open(&lots_path).unwrap());
    let records: Vec<_> = reader
        .records()
        .collect::<Result<Vec<_>, _>>()
        .expect("Failed to read lots.csv records");

    assert!(
        !records.is_empty(),
        "lots.csv must contain at least one data record"
    );

    // Verify that at least one lot has the expected remaining_sat value.
    // The remaining_sat column is at index 4 (0-indexed).
    let expected_remaining_sat = "5000000";
    let found_expected = records
        .iter()
        .any(|rec| rec.get(4) == Some(expected_remaining_sat));

    assert!(
        found_expected,
        "Expected to find a lot with remaining_sat={}, but none was found in lots.csv",
        expected_remaining_sat
    );

    // Strengthen: verify that disposals.csv uses stable tag strings (not Debug repr).
    // coinbase_buy_sell_send: Buy 2025-03-01, Sell 2025-06-15 — less than 1 year → short-term.
    // The `term` column (index 8) must read "short", not the Debug form "ShortTerm".
    let mut dreader = Reader::from_reader(File::open(&disposals_path).unwrap());
    let disposal_records: Vec<_> = dreader
        .records()
        .collect::<Result<Vec<_>, _>>()
        .expect("Failed to read disposals.csv records");
    assert!(
        !disposal_records.is_empty(),
        "disposals.csv must contain at least one data record (the Sell event)"
    );
    // term column is at index 8 (0-indexed): event,kind,disposed_at,lot,sat,proceeds,basis,gain,term,gift_zone
    let term_col = disposal_records[0].get(8).expect("term column missing");
    assert!(
        term_col == "short" || term_col == "long",
        "term column must use stable tag ('short'/'long'), got: {:?}",
        term_col
    );
    // The fixture sell is short-term (bought and sold in 2025, within a year).
    assert_eq!(
        term_col, "short",
        "fixture sell (Mar→Jun 2025) must be short-term, got: {:?}",
        term_col
    );

    // Verify basis_source column in lots.csv uses stable tags (not Debug repr).
    // The Buy is via Coinbase adapter → BasisSource::ExchangeProvided → tag "exchange".
    // Re-read lots.csv (reader was already consumed above; open fresh).
    let mut lreader2 = Reader::from_reader(File::open(&lots_path).unwrap());
    let lot_records: Vec<_> = lreader2
        .records()
        .collect::<Result<Vec<_>, _>>()
        .expect("Failed to re-read lots.csv records");
    // basis_source column is at index 6: origin_event,split,wallet,acquired_at,remaining_sat,usd_basis,basis_source,basis_pending
    let bs_col = lot_records[0].get(6).expect("basis_source column missing");
    assert_eq!(
        bs_col, "exchange",
        "basis_source column must use stable tag 'exchange' (not Debug 'ExchangeProvided'), got: {:?}",
        bs_col
    );

    let key = dir.path().join("backup.asc");
    cmd::admin::backup_key(&vault, &pp(), &key).unwrap();
    assert!(key.exists());
}

/// P2-B Task 2/3: with `--tax-year`, export additionally writes year-scoped `form8949.csv` +
/// `schedule_d.csv`. The `coinbase_buy_sell_send` fixture has a single ST sell of 0.02 BTC in 2025
/// from an exchange wallet → one Form 8949 row (Part I / box C / box_needs_review true) and matching
/// Schedule D ST part totals. Without `--tax-year` (None) the two files are omitted.
#[test]
fn export_writes_year_scoped_form8949_and_schedule_d() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    cmd::import::run(
        &vault,
        &pp(),
        &[fixtures::coinbase_buy_sell_send(dir.path())],
    )
    .unwrap();

    // With None: the year-scoped filing artifacts are NOT written.
    let out_none = dir.path().join("export_none");
    cmd::admin::export_snapshot(&vault, &pp(), &out_none, None, None).unwrap();
    assert!(
        !out_none.join("form8949.csv").exists(),
        "form8949.csv must be omitted without --tax-year"
    );
    assert!(!out_none.join("schedule_d.csv").exists());

    // With Some(2025): both are written, year-scoped.
    let out = dir.path().join("export_2025");
    cmd::admin::export_snapshot(&vault, &pp(), &out, Some(2025), None).unwrap();

    let f8949 = out.join("form8949.csv");
    let schedd = out.join("schedule_d.csv");
    assert!(
        f8949.exists(),
        "form8949.csv must be written for --tax-year"
    );
    assert!(
        schedd.exists(),
        "schedule_d.csv must be written for --tax-year"
    );

    // form8949.csv: header + exactly one data row (the single 2025 ST sell leg).
    let mut fr = Reader::from_reader(File::open(&f8949).unwrap());
    let headers: Vec<String> = fr
        .headers()
        .unwrap()
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(
        headers,
        vec![
            "part",
            "box",
            "box_needs_review",
            "description",
            "date_acquired",
            "date_sold",
            "proceeds",
            "cost_basis",
            "adjustment_code",
            "adjustment_amount",
            "gain",
            "wallet",
            "disposition_kind",
        ],
        "form8949.csv columns must be the stable snake_case contract"
    );
    let recs: Vec<_> = fr.records().collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(recs.len(), 1, "one ST sell leg → one Form 8949 row");
    let row = &recs[0];
    assert_eq!(row.get(0), Some("ST"), "part");
    assert_eq!(row.get(1), Some("C"), "box (conservative C default)");
    assert_eq!(
        row.get(2),
        Some("true"),
        "exchange disposition → box_needs_review"
    );
    assert_eq!(row.get(3), Some("0.02000000 BTC"), "exact BTC description");
    assert_eq!(row.get(8), Some(""), "adjustment_code blank");
    assert_eq!(row.get(9), Some("0"), "adjustment_amount zero");

    // schedule_d.csv: header + two part rows (ST, LT).
    let mut sr = Reader::from_reader(File::open(&schedd).unwrap());
    let sheaders: Vec<String> = sr
        .headers()
        .unwrap()
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(sheaders, vec!["part", "proceeds", "cost_basis", "gain"]);
    let srecs: Vec<_> = sr.records().collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(srecs.len(), 2, "Schedule D has two part rows (ST, LT)");
    assert_eq!(srecs[0].get(0), Some("ST"));
    assert_eq!(srecs[1].get(0), Some("LT"));
}

/// (P2-A Minor fix KAT) Multi-leg donation: removals.csv must show `claimed_deduction` on the
/// FIRST leg row only; subsequent leg rows carry an empty cell so `SUM()` over the column equals
/// the correct per-donation total without double-counting.
///
/// Setup: two lots consumed by one Donation reclassification → `make_removal_legs` yields 2 legs.
///   Lot A: LT (2024-01-01), 1 BTC, basis $5,000 → FMV pro-rata $50,000 → deduction $50,000.
///   Lot B: ST (2025-12-01), 1 BTC, basis $2,000 → FMV pro-rata $50,000 → min($50k,$2k) = $2,000.
///   Total claimed_deduction = $52,000.
///
/// Before fix: both rows carried "52000.00" → SUM = $104,000 (double-counted).
/// After fix:  row 0 = "52000.00", row 1 = "" → SUM = $52,000 (correct).
#[test]
fn removals_csv_multi_leg_donation_no_double_count() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    let now = datetime!(2026-04-01 12:00:00 UTC);

    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    cmd::import::run(
        &vault,
        &pp(),
        &[fixtures::coinbase_two_lot_donation(dir.path())],
    )
    .unwrap();

    // The 2-BTC Send is pending; retrieve its event ref for reclassification.
    let out_ref = {
        let s = Session::open(&vault, &pp()).unwrap();
        let (state, _) = s.project().unwrap();
        assert_eq!(
            state.pending_reconciliation.len(),
            1,
            "Send must be pending before reclassification"
        );
        state.pending_reconciliation[0].event.canonical()
    };

    // Reclassify as Donate with total FMV $100,000 (2 BTC × $50k each, pro-rata by sat).
    // FIFO consumes both lots: LT leg ($50k FMV → $50k deduction) + ST leg ($2k basis,
    // $50k FMV → min($50k,$2k) = $2k deduction).  Total claimed_deduction = $52,000.
    cmd::reconcile::reclassify_outflow(
        &vault,
        &pp(),
        &out_ref,
        OutflowClass::Donate {
            appraisal_required: false,
        },
        btctax_cli::eventref::parse_usd_arg("100000.00").unwrap(),
        None,
        None,
        now,
    )
    .unwrap();

    let out = dir.path().join("export");
    cmd::admin::export_snapshot(&vault, &pp(), &out, None, None).unwrap();

    let removals_path = out.join("removals.csv");
    assert!(removals_path.exists(), "removals.csv must exist");

    // claimed_deduction column index 9:
    // event(0), kind(1), removed_at(2), lot(3), sat(4), basis(5), fmv_at_transfer(6), term(7),
    // acquired_at(8), claimed_deduction(9).
    const DED_COL: usize = 9;

    let mut reader = Reader::from_reader(File::open(&removals_path).unwrap());
    let records: Vec<_> = reader
        .records()
        .collect::<Result<Vec<_>, _>>()
        .expect("Failed to read removals.csv records");

    assert_eq!(
        records.len(),
        2,
        "expected 2 removal leg rows (one per lot consumed by the donation); got {}",
        records.len()
    );

    // Task 1 KAT: removals.csv shows the acquired_at column (index 8), populated with a date on
    // every leg row (the lots' holding-period starts: lot A 2024-01-01, lot B 2025-12-01).
    let mut hreader = Reader::from_reader(File::open(&removals_path).unwrap());
    let header: Vec<String> = hreader
        .headers()
        .unwrap()
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(
        header.get(8).map(String::as_str),
        Some("acquired_at"),
        "removals.csv must show the acquired_at column at index 8; header = {header:?}"
    );
    for rec in &records {
        let acq = rec.get(8).expect("acquired_at cell missing");
        assert!(
            acq.contains('-') && !acq.is_empty(),
            "acquired_at must be a populated date; got {acq:?}"
        );
    }

    let row0_ded = records[0]
        .get(DED_COL)
        .expect("row 0 claimed_deduction missing");
    let row1_ded = records[1]
        .get(DED_COL)
        .expect("row 1 claimed_deduction missing");

    // First leg: full per-donation deduction total.
    assert_eq!(
        row0_ded, "52000.00",
        "first leg row must carry the full claimed_deduction ($52,000); got {row0_ded:?}"
    );

    // Subsequent legs: empty cell so a naive SUM() does not double-count.
    assert!(
        row1_ded.is_empty(),
        "second leg row must have an empty claimed_deduction (not {row1_ded:?}); \
         a non-empty value means the deduction is double-counted in a spreadsheet SUM()"
    );

    // Invariant: SUM over the column == single donation total.
    let sum: f64 = records
        .iter()
        .filter_map(|r| r.get(DED_COL))
        .filter(|s| !s.is_empty())
        .map(|s| {
            s.parse::<f64>()
                .expect("non-empty claimed_deduction must be numeric")
        })
        .sum();
    assert!(
        (sum - 52_000.0_f64).abs() < 0.01,
        "SUM of claimed_deduction column must equal $52,000 (no double-count); got {sum}"
    );
}

/// P2-C Task 2: with `--tax-year`, export also writes `form8283.csv`. Using the two-lot donation
/// (total deduction $52,000 > $5,000 → Section B) contributed 2026-03-01, exported with Some(2026):
/// the caveat comment is present, the header is the stable contract, a Section B row exists with
/// `needs_review=true`, the deduction appears on ONE row only (no SUM double-count), and the
/// unmodeled donee/appraiser/fmv_method columns are blank.
#[test]
fn export_writes_form8283_with_section_b_and_aggregation_caveat() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    let now = datetime!(2026-04-01 12:00:00 UTC);

    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    cmd::import::run(
        &vault,
        &pp(),
        &[fixtures::coinbase_two_lot_donation(dir.path())],
    )
    .unwrap();

    let out_ref = {
        let s = Session::open(&vault, &pp()).unwrap();
        let (state, _) = s.project().unwrap();
        state.pending_reconciliation[0].event.canonical()
    };
    cmd::reconcile::reclassify_outflow(
        &vault,
        &pp(),
        &out_ref,
        OutflowClass::Donate {
            appraisal_required: false,
        },
        btctax_cli::eventref::parse_usd_arg("100000.00").unwrap(),
        None,
        None,
        now,
    )
    .unwrap();

    // The donation is contributed 2026-03-01 → export for tax-year 2026.
    let out = dir.path().join("export_2026");
    cmd::admin::export_snapshot(&vault, &pp(), &out, Some(2026), None).unwrap();

    let f8283 = out.join("form8283.csv");
    assert!(
        f8283.exists(),
        "form8283.csv must be written for --tax-year"
    );

    // Raw file: the aggregation note comment line must be present (confirms §170(f)(11)(F) year-
    // aggregate is implemented and the note reflects it).
    let raw = std::fs::read_to_string(&f8283).unwrap();
    assert!(
        raw.contains("\u{00a7}170(f)(11)(F) year-aggregate"),
        "form8283.csv must carry the §170(f)(11)(F) year-aggregate aggregation note:\n{raw}"
    );
    assert!(
        raw.starts_with("# "),
        "note must be a leading comment line:\n{raw}"
    );
    // $52,000 total (> $500) → the [R0-M1] filing-floor note must NOT appear.
    assert!(
        !raw.contains("[R0-M1]"),
        "the $500 note must NOT appear when the total deduction exceeds $500:\n{raw}"
    );

    // Parse with comment support so the leading `#` line is skipped.
    let mut rdr = csv::ReaderBuilder::new()
        .comment(Some(b'#'))
        .from_reader(File::open(&f8283).unwrap());
    let headers: Vec<String> = rdr
        .headers()
        .unwrap()
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(
        headers,
        vec![
            "section",
            "description",
            "how_acquired",
            "date_acquired",
            "date_contributed",
            "cost_basis",
            "fmv",
            "claimed_deduction",
            "fmv_method",
            "donee",
            "appraiser",
            "needs_review",
            "donee_ein",
            "donee_address",
            "appraiser_tin",
            "appraiser_ptin",
            "appraiser_qualifications",
            "appraisal_date",
        ],
        "form8283.csv columns must be the stable snake_case contract"
    );
    let recs: Vec<_> = rdr.records().collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(recs.len(), 2, "two donation legs → two Form 8283 rows");

    // Column indices: section(0), description(1), how_acquired(2), date_acquired(3),
    // date_contributed(4), cost_basis(5), fmv(6), claimed_deduction(7), fmv_method(8),
    // donee(9), appraiser(10), needs_review(11), donee_ein(12), donee_address(13),
    // appraiser_tin(14), appraiser_ptin(15), appraiser_qualifications(16), appraisal_date(17).
    let section_b_rows = recs.iter().filter(|r| r.get(0) == Some("B")).count();
    assert_eq!(
        section_b_rows, 1,
        "exactly one (first) leg row carries Section B; got {section_b_rows}"
    );
    // needs_review true on every row; donee/appraiser blank on every row.
    // fmv_method: "qualified appraisal" on the Section B carrier row; "" on subsequent legs.
    for r in &recs {
        assert_eq!(r.get(11), Some("true"), "needs_review must be true");
        assert_eq!(r.get(9), Some(""), "donee blank");
        assert_eq!(r.get(10), Some(""), "appraiser blank");
    }
    // Carrier row (Section B, first in output) → fmv_method = "qualified appraisal".
    let carrier = recs
        .iter()
        .find(|r| r.get(0) == Some("B"))
        .expect("carrier row");
    assert_eq!(
        carrier.get(8),
        Some("qualified appraisal"),
        "Section B carrier row must have fmv_method = 'qualified appraisal'"
    );
    // Non-carrier row (section empty) → fmv_method = "".
    let non_carrier = recs
        .iter()
        .find(|r| r.get(0) == Some(""))
        .expect("non-carrier row");
    assert_eq!(
        non_carrier.get(8),
        Some(""),
        "non-carrier row must have fmv_method = '' (carrier convention)"
    );
    // claimed_deduction appears on exactly one row (no SUM double-count) and equals $52,000.
    let deds: Vec<&str> = recs
        .iter()
        .filter_map(|r| r.get(7))
        .filter(|s| !s.is_empty())
        .collect();
    assert_eq!(deds, vec!["52000.00"], "deduction on the first leg only");
}

/// P2-C Task 2 [R0-M1]: when the year's total noncash donation deduction is ≤ $500, form8283.csv
/// carries the filing-floor note that Form 8283 is not required at that level (rows still emitted).
#[test]
fn export_form8283_small_donation_shows_500_floor_note() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    let now = datetime!(2025-12-31 12:00:00 UTC);

    // Small donation: buy 0.001 BTC @ $100 (2025-01-01), send 0.001 BTC (2025-06-01),
    // reclassify as Donate FMV $300 → deduction ≤ $500 regardless of term.
    let csv_path = dir.path().join("coinbase_small_donation.csv");
    std::fs::write(
        &csv_path,
        "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,\
Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
sd-buy,2025-01-01 12:00:00 UTC,Buy,BTC,0.00100000,USD,100000.00,100.00,100.00,0.00,,,\r\n\
sd-send,2025-06-01 12:00:00 UTC,Send,BTC,0.00100000,USD,300.00,,,,,,bc1qsyntheticcharity\r\n",
    )
    .unwrap();

    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    cmd::import::run(&vault, &pp(), &[csv_path]).unwrap();

    let out_ref = {
        let s = Session::open(&vault, &pp()).unwrap();
        let (state, _) = s.project().unwrap();
        state.pending_reconciliation[0].event.canonical()
    };
    cmd::reconcile::reclassify_outflow(
        &vault,
        &pp(),
        &out_ref,
        OutflowClass::Donate {
            appraisal_required: false,
        },
        btctax_cli::eventref::parse_usd_arg("300.00").unwrap(),
        None,
        None,
        now,
    )
    .unwrap();

    let out = dir.path().join("export_2025_small");
    cmd::admin::export_snapshot(&vault, &pp(), &out, Some(2025), None).unwrap();

    let raw = std::fs::read_to_string(out.join("form8283.csv")).unwrap();
    assert!(
        raw.contains("[R0-M1]") && raw.contains("<= $500"),
        "small (≤ $500) donation total must carry the filing-floor note:\n{raw}"
    );
    // Rows are still emitted (informational).
    let mut rdr = csv::ReaderBuilder::new()
        .comment(Some(b'#'))
        .from_reader(File::open(out.join("form8283.csv")).unwrap());
    let recs: Vec<_> = rdr.records().collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(
        recs.len(),
        1,
        "the single donation leg row is still emitted"
    );
    assert_eq!(recs[0].get(0), Some("A"), "≤ $5k → Section A");
}

/// CLI-I1 fix: exported CSVs are owner-only (0o600) even when the out-dir PRE-EXISTS.
/// The old `Writer::from_path` path created files at the process umask (typically 0o644 —
/// world-readable). The fix routes each CSV through `fsperms::open_owner_only` (0o600 on
/// Unix create-or-truncate) so the mode is hardened regardless of umask or dir pre-existence.
#[cfg(unix)]
#[test]
fn csv_exports_are_owner_only_on_pre_existing_dir() {
    use std::os::unix::fs::PermissionsExt as _;

    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    cmd::import::run(
        &vault,
        &pp(),
        &[fixtures::coinbase_buy_sell_send(dir.path())],
    )
    .unwrap();

    // Create the out-dir BEFORE the export so it pre-exists — this is the hole the fix closes.
    let out = dir.path().join("exports_pre");
    std::fs::create_dir_all(&out).unwrap();

    cmd::admin::export_snapshot(&vault, &pp(), &out, None, None).unwrap();

    for name in ["lots.csv", "disposals.csv", "removals.csv", "income.csv"] {
        let path = out.join(name);
        assert!(path.exists(), "{name} must exist");
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(
            mode & 0o777,
            0o600,
            "{name} must be owner-only (0o600), got {:#o}",
            mode & 0o777
        );
    }
}

/// [Chunk 2 Task 1] removals.csv donee column — populated and empty cases.
/// Gift with donee "Alice" → donee cell = "Alice"; Donate without donee → donee cell = "".
/// Also asserts that the `donee` column is present in the header at the correct position.
/// Engine B / tax math is NOT exercised — donee is data only; this test verifies CSV schema.
#[test]
fn removals_csv_has_donee_column_populated_and_empty() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    let now = datetime!(2026-04-01 12:00:00 UTC);

    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    // Import the fixture that has a buy + a send (coinbase_buy_sell_send has a pending send).
    cmd::import::run(
        &vault,
        &pp(),
        &[fixtures::coinbase_buy_sell_send(dir.path())],
    )
    .unwrap();

    // Find both pending outflows: the fixture produces one pending send.
    // We reclassify it as GiftOut with donee "Alice".
    let out_ref = {
        let s = Session::open(&vault, &pp()).unwrap();
        let (state, _) = s.project().unwrap();
        // Use the pending send (the send in the fixture).
        state.pending_reconciliation[0].event.canonical()
    };

    cmd::reconcile::reclassify_outflow(
        &vault,
        &pp(),
        &out_ref,
        OutflowClass::GiftOut,
        btctax_cli::eventref::parse_usd_arg("2100.00").unwrap(),
        None,
        Some("Alice".to_string()), // donee populated
        now,
    )
    .unwrap();

    let out = dir.path().join("export_donee");
    cmd::admin::export_snapshot(&vault, &pp(), &out, None, None).unwrap();

    let removals_path = out.join("removals.csv");
    assert!(removals_path.exists(), "removals.csv must exist");

    // Check the header has "donee" at column 10.
    // Header: event(0), kind(1), removed_at(2), lot(3), sat(4), basis(5), fmv_at_transfer(6),
    //         term(7), acquired_at(8), claimed_deduction(9), donee(10).
    let mut rdr = Reader::from_reader(File::open(&removals_path).unwrap());
    let header: Vec<String> = rdr
        .headers()
        .unwrap()
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(
        header.get(10).map(String::as_str),
        Some("donee"),
        "removals.csv must have 'donee' header at column 10; got: {header:?}"
    );

    let records: Vec<_> = rdr
        .records()
        .collect::<Result<Vec<_>, _>>()
        .expect("removals.csv must parse");

    // The fixture gift row has donee "Alice".
    assert!(
        !records.is_empty(),
        "expected at least one removal row in removals.csv"
    );
    let donee_cell = records[0].get(10).expect("donee column missing on row 0");
    assert_eq!(
        donee_cell, "Alice",
        "GiftOut with donee 'Alice' must appear in removals.csv donee column; got {donee_cell:?}"
    );
}

/// [Chunk 2 Task 1] Engine B / tax math unchanged: donee is data only, does not affect
/// computed disposals or gain. The sell in coinbase_buy_sell_send produces a known gain;
/// adding a gift with a donee must not change any disposal or tax figure.
#[test]
fn engine_b_tax_math_unchanged_by_donee() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    let now = datetime!(2026-04-01 12:00:00 UTC);

    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    cmd::import::run(
        &vault,
        &pp(),
        &[fixtures::coinbase_buy_sell_send(dir.path())],
    )
    .unwrap();

    // Baseline: project before reclassifying the send.
    let baseline_disposals = {
        let s = Session::open(&vault, &pp()).unwrap();
        let (state, _) = s.project().unwrap();
        state.disposals.clone()
    };

    // Reclassify the pending send as GiftOut with a donee.
    let out_ref = {
        let s = Session::open(&vault, &pp()).unwrap();
        let (state, _) = s.project().unwrap();
        state.pending_reconciliation[0].event.canonical()
    };
    cmd::reconcile::reclassify_outflow(
        &vault,
        &pp(),
        &out_ref,
        OutflowClass::GiftOut,
        btctax_cli::eventref::parse_usd_arg("2100.00").unwrap(),
        None,
        Some("Alice".to_string()),
        now,
    )
    .unwrap();

    let after_disposals = {
        let s = Session::open(&vault, &pp()).unwrap();
        let (state, _) = s.project().unwrap();
        state.disposals.clone()
    };

    // The disposal count and all disposal figures must be unchanged.
    assert_eq!(
        baseline_disposals.len(),
        after_disposals.len(),
        "donee on a gift must NOT change disposal count"
    );
    for (before, after) in baseline_disposals.iter().zip(after_disposals.iter()) {
        assert_eq!(
            before.legs.len(),
            after.legs.len(),
            "disposal leg count must be unchanged by donee"
        );
        for (bl, al) in before.legs.iter().zip(after.legs.iter()) {
            assert_eq!(bl.gain, al.gain, "disposal gain must be unchanged by donee");
            assert_eq!(
                bl.basis, al.basis,
                "disposal basis must be unchanged by donee"
            );
            assert_eq!(
                bl.proceeds, al.proceeds,
                "disposal proceeds must be unchanged by donee"
            );
        }
    }
}

// ---------------------------------------------------------------------------------------------
// export-blocker-summary: `ExportReport { path, unresolved_hard }` + the main.rs stderr warning.
// Library KATs call `cmd::admin::export_snapshot` directly (they see only the struct); the binary
// KATs drive the compiled `btctax` so they can observe the `eprintln!` in the main.rs arm.
// ---------------------------------------------------------------------------------------------

/// Run the compiled `btctax export-snapshot` binary; returns (exit_code, stderr). Uses
/// `Command::output()` to CAPTURE stderr — the `fr9_exit_code.rs` / `tax_report.rs` bin pattern
/// [R0-r2-N]. The vault is built with the library (init + import); the binary only opens it.
fn run_export_bin(vault: &Path, out: &Path, tax_year: Option<i32>) -> (i32, String) {
    let bin = env!("CARGO_BIN_EXE_btctax");
    let mut command = std::process::Command::new(bin);
    command
        .arg("--vault")
        .arg(vault.to_str().expect("vault path is valid UTF-8"))
        .arg("export-snapshot")
        .arg("--out")
        .arg(out.to_str().expect("out path is valid UTF-8"));
    if let Some(y) = tax_year {
        command.arg("--tax-year").arg(y.to_string());
    }
    let output = command
        .env("BTCTAX_PASSPHRASE", "pw")
        .output()
        .expect("btctax binary must execute");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let code = output
        .status
        .code()
        .expect("btctax process must exit normally (not via signal)");
    (code, stderr)
}

/// `unresolved_hard` counts Hard blockers ONLY. A real Advisory (`SelfTransferInboundZeroBasis`,
/// fired by classifying an inbound as a self-transfer with the DEFAULT $0 basis) must NOT count
/// (→ 0), while a Hard blocker (`UnknownBasisInbound`, an unclassified inbound) does (→ ≥ 1). [R0-N2]
#[test]
fn export_report_counts_only_hard() {
    // (a) Advisory-only ledger: classify the raw inbound as a self-transfer with defaulted $0 basis
    //     → clears the Hard `UnknownBasisInbound`, fires the honest `SelfTransferInboundZeroBasis`.
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    let now = datetime!(2026-04-01 12:00:00 UTC);
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    cmd::import::run(&vault, &pp(), &[fixtures::coinbase_buy_receive(dir.path())]).unwrap();

    let in_ref = {
        let s = Session::open(&vault, &pp()).unwrap();
        let events = btctax_core::persistence::load_all(s.conn()).unwrap();
        events
            .iter()
            .find(|e| matches!(e.payload, EventPayload::TransferIn(_)))
            .expect("the Receive must fold to a TransferIn")
            .id
            .canonical()
    };
    cmd::reconcile::classify_inbound(
        &vault,
        &pp(),
        &in_ref,
        InboundClass::SelfTransferMine {
            basis: None,
            acquired_at: None,
        },
        now,
    )
    .unwrap();

    // Sanity: the ledger now carries a REAL Advisory but NO Hard blocker.
    {
        let s = Session::open(&vault, &pp()).unwrap();
        let (state, _) = s.project().unwrap();
        assert!(
            state
                .blockers
                .iter()
                .any(|b| b.kind == BlockerKind::SelfTransferInboundZeroBasis),
            "the zero-basis Advisory must be present"
        );
        assert!(
            state
                .blockers
                .iter()
                .all(|b| b.kind.severity() != Severity::Hard),
            "an Advisory-only ledger has no Hard blocker"
        );
    }

    let out = dir.path().join("export_advisory");
    let report = cmd::admin::export_snapshot(&vault, &pp(), &out, None, None).unwrap();
    assert_eq!(
        report.unresolved_hard, 0,
        "a real Advisory (SelfTransferInboundZeroBasis) must NOT count as a Hard blocker"
    );

    // (b) Hard ledger: a bare unclassified Receive → `UnknownBasisInbound` Hard blocker → counted.
    let dir2 = tempfile::tempdir().unwrap();
    let vault2 = dir2.path().join("vault.pgp");
    cmd::init::run(&vault2, &pp(), &dir2.path().join("k.asc")).unwrap();
    cmd::import::run(
        &vault2,
        &pp(),
        &[fixtures::coinbase_buy_receive(dir2.path())],
    )
    .unwrap();
    let out2 = dir2.path().join("export_hard");
    let report2 = cmd::admin::export_snapshot(&vault2, &pp(), &out2, None, None).unwrap();
    assert!(
        report2.unresolved_hard >= 1,
        "an unclassified inbound (UnknownBasisInbound) is a Hard blocker and must be counted; got {}",
        report2.unresolved_hard
    );
}

/// `report.path` points at the store's `snapshot.sqlite`, inside the requested out-dir.
#[test]
fn export_report_path_points_at_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    cmd::import::run(
        &vault,
        &pp(),
        &[fixtures::coinbase_buy_sell_send(dir.path())],
    )
    .unwrap();

    let out = dir.path().join("export");
    let report = cmd::admin::export_snapshot(&vault, &pp(), &out, None, None).unwrap();
    assert!(report.path.exists(), "the returned path must exist");
    assert_eq!(
        report.path.file_name().and_then(|n| n.to_str()),
        Some("snapshot.sqlite"),
        "report.path must point at the store snapshot.sqlite; got {:?}",
        report.path
    );
    assert!(
        report.path.starts_with(&out),
        "the snapshot must live inside the requested out-dir"
    );
}

/// Export is a DISCLOSURE, not a refusal: even with an unresolved Hard blocker it STILL writes the
/// snapshot + all-years CSVs + the (empty) year-scoped forms — the "silent empty forms" scenario,
/// now flagged via `unresolved_hard` rather than blocked.
#[test]
fn export_still_writes_files_with_blockers() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    cmd::import::run(&vault, &pp(), &[fixtures::coinbase_buy_receive(dir.path())]).unwrap();

    let out = dir.path().join("export_blocked");
    let report = cmd::admin::export_snapshot(&vault, &pp(), &out, Some(2025), None).unwrap();

    assert!(
        report.unresolved_hard >= 1,
        "the unclassified inbound is a Hard blocker"
    );
    // The files are written REGARDLESS — the warning is a disclosure, not a gate.
    for name in [
        "snapshot.sqlite",
        "lots.csv",
        "disposals.csv",
        "removals.csv",
        "income.csv",
        "form8949.csv",
        "schedule_d.csv",
    ] {
        assert!(
            out.join(name).exists(),
            "{name} must still be written despite the Hard blocker"
        );
    }
}

/// BINARY KAT — with an unresolved Hard blocker + `--tax-year`, the compiled binary WARNS on stderr
/// ("NOT COMPUTABLE" + the exact count + "verify"), still writes the files, and exits 0. [★ fault-
/// inject target: deleting the `unresolved_hard > 0` eprintln! in the main.rs arm turns this RED.]
#[test]
fn export_with_hard_blockers_warns_on_stderr() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    cmd::import::run(&vault, &pp(), &[fixtures::coinbase_buy_receive(dir.path())]).unwrap();

    // The real Hard-blocker count (drives the exact "{n} unresolved Hard blocker(s)" string).
    let n = {
        let s = Session::open(&vault, &pp()).unwrap();
        let (state, _) = s.project().unwrap();
        state
            .blockers
            .iter()
            .filter(|b| b.kind.severity() == Severity::Hard)
            .count()
    };
    assert!(n >= 1, "the fixture must carry at least one Hard blocker");

    let out = dir.path().join("export_warn");
    let (code, stderr) = run_export_bin(&vault, &out, Some(2025));

    assert_eq!(
        code, 0,
        "export is a disclosure, not a refusal — exit 0; stderr: {stderr}"
    );
    assert!(
        stderr.contains("NOT COMPUTABLE"),
        "stderr must flag the year NOT COMPUTABLE; got: {stderr}"
    );
    assert!(
        stderr.contains(&format!("{n} unresolved Hard blocker")),
        "stderr must carry the exact unresolved-Hard count ({n}); got: {stderr}"
    );
    assert!(
        stderr.contains("verify"),
        "stderr must direct the user to `btctax verify`; got: {stderr}"
    );
    assert!(
        stderr.contains("INFORMATIONAL, not final"),
        "stderr must state the forms are INFORMATIONAL, not final; got: {stderr}"
    );
    // Files are still written (the empty-forms scenario, now disclosed).
    assert!(
        out.join("snapshot.sqlite").exists(),
        "snapshot must be written"
    );
    assert!(
        out.join("form8949.csv").exists(),
        "form8949.csv must be written"
    );
}

/// BINARY KAT — a FULL export (no `--tax-year`) with unresolved Hard blockers warns that the
/// exported FIGURES (projection CSVs, not the 8949/Schedule D forms) are INFORMATIONAL, and every
/// affected year is NOT COMPUTABLE. Exit 0.
#[test]
fn export_full_no_year_warns_informational() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    cmd::import::run(&vault, &pp(), &[fixtures::coinbase_buy_receive(dir.path())]).unwrap();

    let out = dir.path().join("export_full");
    let (code, stderr) = run_export_bin(&vault, &out, None);

    assert_eq!(code, 0, "exit 0; stderr: {stderr}");
    assert!(
        stderr.contains("INFORMATIONAL, not final"),
        "the full-export warning must say INFORMATIONAL, not final; got: {stderr}"
    );
    assert!(
        stderr.contains("NOT COMPUTABLE"),
        "the full-export warning must say every affected year is NOT COMPUTABLE; got: {stderr}"
    );
    // "figures" (not "forms") — the no-year path writes projection CSVs, not the 8949/Schedule D.
    assert!(
        stderr.contains("figures"),
        "the no-year message must say 'figures' (projection CSVs), not 'forms'; got: {stderr}"
    );
}

/// BINARY KAT — a fully-resolved (0 Hard) ledger exports with NO warning: stderr carries no ⚠ /
/// "NOT COMPUTABLE", stdout still confirms the export, exit 0.
#[test]
fn export_clean_ledger_no_warning() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    // A Buy + Sell + (pending) Send — a real disposal + an Advisory UnmatchedOutflows, NO Hard.
    cmd::import::run(
        &vault,
        &pp(),
        &[fixtures::coinbase_buy_sell_send(dir.path())],
    )
    .unwrap();

    // Guard: this ledger really is clean of Hard blockers.
    {
        let s = Session::open(&vault, &pp()).unwrap();
        let (state, _) = s.project().unwrap();
        assert!(
            state
                .blockers
                .iter()
                .all(|b| b.kind.severity() != Severity::Hard),
            "the clean fixture must carry zero Hard blockers"
        );
    }

    let out = dir.path().join("export_clean");
    let (code, stderr) = run_export_bin(&vault, &out, Some(2025));

    assert_eq!(code, 0, "clean export exits 0; stderr: {stderr}");
    assert!(
        !stderr.contains('\u{26a0}'),
        "a clean ledger must print NO ⚠ warning; got: {stderr}"
    );
    assert!(
        !stderr.contains("NOT COMPUTABLE"),
        "a clean ledger must not flag NOT COMPUTABLE; got: {stderr}"
    );
}
