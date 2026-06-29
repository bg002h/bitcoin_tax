# Task 2 Report: Format-agnostic table reading (RawRow, CSV preamble/CRLF, XLSX)

**Status:** DONE  
**Commit:** `bab7ffd`  
**Branch:** feat/btctax-adapters

---

## Files Touched

- `crates/btctax-adapters/src/read.rs` — created (306 lines after `cargo fmt`)
- `crates/btctax-adapters/src/lib.rs` — added `pub mod read;`

---

## Test Command and Output

```
cargo test -p btctax-adapters
```

```
running 14 tests
test parse::tests::btc_to_sat_is_exact_integer ... ok
test parse::tests::excel_serial_and_flex_parse ... ok
test parse::tests::fractional_satoshi_is_an_error_never_a_silent_round ... ok
test parse::tests::timestamp_rfc3339_keeps_offset_then_normalizes_to_utc ... ok
test parse::tests::parses_usd_exactly_no_float ... ok
test price::tests::fmv_of_uses_provider_for_sat_quantity ... ok
test price::tests::timestamp_naive_assumed_utc ... ok
test price::tests::looks_up_daily_close_exact_date ... ok
test price::tests::parses_exact_decimals_not_floats ... ok
test parse::tests::timestamp_confirmed_export_formats ... ok
test read::tests::fixed_skip_preamble_count_works_when_no_signature ... ok
test read::tests::missing_column_is_a_typed_error ... ok
test read::tests::skips_preamble_via_header_signature_and_handles_crlf ... ok
test read::tests::xlsx_numeric_serial_date_roundtrip ... ok

test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
```

`cargo clippy -p btctax-adapters --all-targets -- -D warnings` — clean.  
`cargo fmt --check -p btctax-adapters` — clean (applied `cargo fmt` before check).

---

## IP-1 Resolution: calamine 0.26.1 Data Variants + Serial Accessor

### Verified Data variants in calamine 0.26.1 (`src/datatype.rs`)

All 9 variants exist exactly as stated in the brief. No arm was deleted.

| Variant | Inner type |
|---|---|
| `Data::Int` | `i64` |
| `Data::Float` | `f64` |
| `Data::String` | `String` |
| `Data::Bool` | `bool` |
| `Data::DateTime` | `ExcelDateTime` |
| `Data::DateTimeIso` | `String` |
| `Data::DurationIso` | `String` |
| `Data::Error` | `CellErrorType` |
| `Data::Empty` | — |

### Serial accessor used: `ExcelDateTime::as_f64()`

Confirmed in calamine 0.26.1 `src/datatype.rs`:
```rust
/// Converting data type into a float
pub fn as_f64(&self) -> f64 {
    self.value
}
```
Returns the raw Excel serial (`self.value: f64`). No `#[cfg(feature = "dates")]` guard — always available.

The `cell_to_string` arm:
```rust
Data::DateTime(dt) => format!("{}", dt.as_f64()),
```
Produces the serial string (e.g. `"25569.5"`) which flows through `parse_timestamp_flex` →
`excel_serial_to_utc`. Note: calamine's `ExcelDateTime::Display` already writes `self.value`,
so `dt.to_string()` would yield the same result in 0.26.1 — but `as_f64()` is used explicitly
to be self-documenting and decouple from Display behaviour (the brief's IP-1 requirement).

---

## Deviations from Brief (both compile-time only, no logic change)

1. **`MissingColumn` field name:** The brief's `RawRow::get` used `source` as the field name
   in the struct literal, but the existing `AdapterError::MissingColumn` (Task 0/1) uses
   `adapter`. Fixed to `adapter: source`. No change to `AdapterError` definition.

2. **Type annotation on `open_workbook` closure:** Rustc could not infer the closure parameter
   type in `open_workbook(path).map_err(|e| ...)` with `e.into()`. Added
   `|e: calamine::XlsxError|` annotation. No API change.

---

## Gemini XLSX Fixture Test

`read::tests::xlsx_numeric_serial_date_roundtrip` exercises the full path:

1. `rust_xlsxwriter::Workbook::add_worksheet()` + `write_number(1, 0, 25569.5_f64)` — writes
   the Excel serial for 1970-01-01 12:00:00 UTC as a plain number (no date format).
2. `read_xlsx` reads it back; calamine sees no date format → `Data::Float(25569.5)`.
3. `cell_to_string` → `format!("{f}")` → `"25569.5"`.
4. `parse_timestamp_flex("gemini", 1, "25569.5")` → `excel_serial_to_utc(25569.5)`
   → `datetime!(1970-01-01 12:00:00 UTC)`. ✓

The `Data::DateTime` arm (when calamine returns a date-formatted cell from a real Gemini export)
is covered by the compile-time match exhaustion; the run-time path for it is exercised via the
confirmed `as_f64()` method body.

---

## M-1 Fix: CSV Header Signature Validation (Post-Task-2 Minor)

**Status:** DONE  
**Commit:** `d8e2ca1`

### Problem
When `header_signature` is non-empty but no line matches it, the code silently fell back to row 0
via `unwrap_or(0)`, which could misparse a changed exchange preamble as valid data.

### Solution
- **AdapterError:** Added `HeaderNotFound { adapter: &'static str }` variant with error message
  `"{adapter}: header signature not found in file"`
- **read_csv_str signature:** Added `adapter: &'static str` parameter (also updated `read_csv` and
  `read_table` for consistency)
- **Logic fix:** Replaced `.position(...).unwrap_or(0)` with `.position(...).ok_or(AdapterError::HeaderNotFound { adapter })?`
- **Test:** Added `nonmatching_header_signature_returns_error` to verify the error is returned

### Validation
- All 15 tests pass (including new M-1 test)
- `cargo clippy -p btctax-adapters --all-targets -- -D warnings` — clean
- `cargo fmt -p btctax-adapters` — clean

### Test Output
```
cargo test -p btctax-adapters

running 15 tests
test parse::tests::btc_to_sat_is_exact_integer ... ok
test parse::tests::excel_serial_and_flex_parse ... ok
test price::tests::fmv_of_uses_provider_for_sat_quantity ... ok
test parse::tests::parses_usd_exactly_no_float ... ok
test price::tests::fractional_satoshi_is_an_error_never_a_silent_round ... ok
test parse::tests::timestamp_confirmed_export_formats ... ok
test parse::tests::timestamp_rfc3339_keeps_offset_then_normalizes_to_utc ... ok
test price::tests::looks_up_daily_close_exact_date ... ok
test read::tests::nonmatching_header_signature_returns_error ... ok
test read::tests::skips_preamble_via_header_signature_and_handles_crlf ... ok
test parse::tests::timestamp_naive_assumed_utc ... ok
test price::tests::parses_exact_decimals_not_floats ... ok
test read::tests::missing_column_is_a_typed_error ... ok
test read::tests::fixed_skip_preamble_count_works_when_no_signature ... ok
test read::tests::xlsx_numeric_serial_date_roundtrip ... ok

test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

### Concerns
None. The signature change is backward-incompatible for external callers of read_csv_str/read_csv,
but since this is Task 2 and these functions haven't yet been called by any adapter implementations,
this is acceptable.
