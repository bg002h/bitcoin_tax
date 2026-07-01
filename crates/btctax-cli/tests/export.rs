mod fixtures;
use btctax_cli::{cmd, Session};
use btctax_core::OutflowClass;
use btctax_store::Passphrase;
use csv::Reader;
use std::fs::File;
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
    let sqlite = cmd::admin::export_snapshot(&vault, &pp(), &out, None).unwrap();
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
    cmd::admin::export_snapshot(&vault, &pp(), &out_none, None).unwrap();
    assert!(
        !out_none.join("form8949.csv").exists(),
        "form8949.csv must be omitted without --tax-year"
    );
    assert!(!out_none.join("schedule_d.csv").exists());

    // With Some(2025): both are written, year-scoped.
    let out = dir.path().join("export_2025");
    cmd::admin::export_snapshot(&vault, &pp(), &out, Some(2025)).unwrap();

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
        now,
    )
    .unwrap();

    let out = dir.path().join("export");
    cmd::admin::export_snapshot(&vault, &pp(), &out, None).unwrap();

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
        now,
    )
    .unwrap();

    // The donation is contributed 2026-03-01 → export for tax-year 2026.
    let out = dir.path().join("export_2026");
    cmd::admin::export_snapshot(&vault, &pp(), &out, Some(2026)).unwrap();

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
        ],
        "form8283.csv columns must be the stable snake_case contract"
    );
    let recs: Vec<_> = rdr.records().collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(recs.len(), 2, "two donation legs → two Form 8283 rows");

    // Column indices: section(0), ..., claimed_deduction(7), fmv_method(8), donee(9),
    // appraiser(10), needs_review(11).
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
        now,
    )
    .unwrap();

    let out = dir.path().join("export_2025_small");
    cmd::admin::export_snapshot(&vault, &pp(), &out, Some(2025)).unwrap();

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

    cmd::admin::export_snapshot(&vault, &pp(), &out, None).unwrap();

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
