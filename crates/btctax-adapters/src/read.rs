//! Format-agnostic table reading. A `RawRow` is a header→cell string map; CSV and XLSX both reduce
//! to it, so every parser is format-independent. Money/amount cells stay strings here (NFR5: the
//! exact decimal parse happens in `parse`).
use crate::AdapterError;
use calamine::{open_workbook, Data, Range, Reader, Xlsx};
use std::collections::BTreeMap;
use std::path::Path;

/// Which logical table a row came from. Swan ships three files in one batch; everyone else is `Single`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableRole {
    Single,
    SwanTrades,
    SwanTransfers,
    SwanWithdrawals,
}

/// One parsed data row: the originating table role, a 1-based data-row number (error context), and
/// the header→cell map (cells are trimmed).
#[derive(Debug, Clone)]
pub struct RawRow {
    pub role: TableRole,
    pub line: usize,
    pub cells: BTreeMap<String, String>,
}
impl RawRow {
    /// Required column; `MissingColumn` if absent.
    pub fn get(&self, source: &'static str, col: &str) -> Result<&str, AdapterError> {
        self.cells
            .get(col)
            .map(|s| s.as_str())
            .ok_or_else(|| AdapterError::MissingColumn {
                adapter: source,
                line: self.line,
                column: col.to_string(),
            })
    }
    /// Optional column: `None` if absent OR blank.
    pub fn opt(&self, col: &str) -> Option<&str> {
        self.cells
            .get(col)
            .map(|s| s.as_str())
            .filter(|s| !s.trim().is_empty())
    }
}

/// CSV preamble handling: either scan for the first line containing all `header_signature` tokens
/// (robust to preamble-length drift — preferred), or skip a fixed `skip_preamble_lines` count.
#[derive(Debug, Clone, Default)]
pub struct ReadOpts {
    pub skip_preamble_lines: usize,
    pub header_signature: &'static [&'static str],
}

/// Read a CSV file's data rows. Reads the whole file as text first (so any error is a clean parse
/// error with path context); the `csv` crate handles CRLF transparently.
pub fn read_csv(
    path: &Path,
    role: TableRole,
    opts: &ReadOpts,
) -> Result<Vec<RawRow>, AdapterError> {
    let text = std::fs::read_to_string(path).map_err(|e| AdapterError::Io {
        path: path.display().to_string(),
        source: e,
    })?;
    read_csv_str(&text, role, opts).map_err(|e| with_path(e, path))
}

/// CSV-from-string (used by `read_csv` and tests).
pub fn read_csv_str(
    text: &str,
    role: TableRole,
    opts: &ReadOpts,
) -> Result<Vec<RawRow>, AdapterError> {
    let start = if !opts.header_signature.is_empty() {
        text.lines()
            .position(|l| opts.header_signature.iter().all(|t| l.contains(t)))
            .unwrap_or(0)
    } else {
        opts.skip_preamble_lines
    };
    let body: String = text.split_inclusive('\n').skip(start).collect();
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(body.as_bytes());
    let headers: Vec<String> = rdr
        .headers()
        .map_err(csv_err)?
        .iter()
        .map(|h| h.trim().to_string())
        .collect();
    let mut out = Vec::new();
    for (i, rec) in rdr.records().enumerate() {
        let rec = rec.map_err(csv_err)?;
        let mut cells = BTreeMap::new();
        for (h, v) in headers.iter().zip(rec.iter()) {
            if !h.is_empty() {
                cells.insert(h.clone(), v.trim().to_string());
            }
        }
        out.push(RawRow {
            role,
            line: i + 1,
            cells,
        });
    }
    Ok(out)
}

/// Read the first worksheet of an XLSX file's data rows (header = first row).
pub fn read_xlsx(path: &Path, role: TableRole) -> Result<Vec<RawRow>, AdapterError> {
    let mut wb: Xlsx<_> =
        open_workbook(path).map_err(|e: calamine::XlsxError| AdapterError::Xlsx {
            path: path.display().to_string(),
            source: e.into(),
        })?;
    let range = wb
        .worksheet_range_at(0)
        // M-2: use EmptyXlsx (not PriceDataset — wrong category) for the "no worksheet" case.
        // calamine returns None when the workbook has no sheet, which is a reader-layer concern.
        .ok_or_else(|| AdapterError::EmptyXlsx {
            path: path.display().to_string(),
        })?
        .map_err(|e| AdapterError::Xlsx {
            path: path.display().to_string(),
            source: e.into(),
        })?;
    Ok(rows_from_range(&range, role))
}

/// Dispatch on file extension: `.xlsx`/`.xls` → calamine; everything else → CSV.
pub fn read_table(
    path: &Path,
    role: TableRole,
    opts: &ReadOpts,
) -> Result<Vec<RawRow>, AdapterError> {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("xlsx") | Some("xls") => read_xlsx(path, role),
        _ => read_csv(path, role, opts),
    }
}

/// Peek the first `max_bytes` of a file as lossy UTF-8 (for source detection). For XLSX, returns the
/// path's bytes (binary) — XLSX detection keys on the extension, not the snippet.
pub fn peek_text(path: &Path, max_bytes: usize) -> Result<String, AdapterError> {
    use std::io::Read;
    let mut f = std::fs::File::open(path).map_err(|e| AdapterError::Io {
        path: path.display().to_string(),
        source: e,
    })?;
    let mut buf = vec![0u8; max_bytes];
    let n = f.read(&mut buf).map_err(|e| AdapterError::Io {
        path: path.display().to_string(),
        source: e,
    })?;
    buf.truncate(n);
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

/// XLSX numeric cells are IEEE-754 doubles in the file format. Rust's shortest-round-trip `{}` for f64
/// reproduces the intended ≤8-dp exchange decimal exactly (e.g. 0.12345678 → "0.12345678"); that string
/// is then parsed by the exact decimal parser (NFR5 — documented bound, FOLLOWUPS).
fn cell_to_string(d: &Data) -> String {
    match d {
        Data::Empty => String::new(),
        Data::String(s) => s.trim().to_string(),
        Data::Float(f) => format!("{f}"),
        Data::Int(i) => i.to_string(),
        Data::Bool(b) => b.to_string(),
        // Excel-serial datetimes: extract the underlying serial number and format it as a string
        // so it flows through the same Data::Float → parse_timestamp_flex(serial) path used for
        // Gemini's numeric Date/Time columns. dt.to_string() must NOT be used here — it formats
        // as the serial already (ExcelDateTime Display writes self.value), but we use as_f64()
        // explicitly to be self-documenting and to decouple from calamine's Display impl.
        // Verified: calamine 0.26.1 ExcelDateTime::as_f64() returns self.value (the Excel serial).
        Data::DateTime(dt) => format!("{}", dt.as_f64()),
        Data::DateTimeIso(s) | Data::DurationIso(s) => s.clone(),
        Data::Error(e) => format!("#ERR:{e:?}"),
    }
}

fn rows_from_range(range: &Range<Data>, role: TableRole) -> Vec<RawRow> {
    let mut iter = range.rows();
    let header: Vec<String> = match iter.next() {
        Some(h) => h.iter().map(cell_to_string).collect(),
        None => return Vec::new(),
    };
    let mut out = Vec::new();
    for (i, r) in iter.enumerate() {
        let mut cells = BTreeMap::new();
        for (h, c) in header.iter().zip(r.iter()) {
            if !h.is_empty() {
                cells.insert(h.clone(), cell_to_string(c));
            }
        }
        if cells.values().all(|v| v.is_empty()) {
            continue;
        }
        out.push(RawRow {
            role,
            line: i + 1,
            cells,
        });
    }
    out
}

fn csv_err(e: csv::Error) -> AdapterError {
    AdapterError::Csv {
        path: "<csv>".to_string(),
        source: e,
    }
}

fn with_path(e: AdapterError, path: &Path) -> AdapterError {
    match e {
        AdapterError::Csv { source, .. } => AdapterError::Csv {
            path: path.display().to_string(),
            source,
        },
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_preamble_via_header_signature_and_handles_crlf() {
        // 3 preamble lines then the header; CRLF line endings (River-style).
        let text = "Transactions\r\nUser,acct\r\n\r\nID,Amount,Total\r\nX1,0.5,100.00\r\nX2,0.25,50.00\r\n";
        let opts = ReadOpts {
            header_signature: &["ID", "Total"],
            ..Default::default()
        };
        let rows = read_csv_str(text, TableRole::Single, &opts).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].get("t", "ID").unwrap(), "X1");
        assert_eq!(rows[0].get("t", "Total").unwrap(), "100.00");
        assert_eq!(rows[1].line, 2);
    }

    #[test]
    fn missing_column_is_a_typed_error() {
        let rows = read_csv_str("A,B\n1,2\n", TableRole::Single, &ReadOpts::default()).unwrap();
        let e = rows[0].get("t", "C").unwrap_err();
        assert!(matches!(e, crate::AdapterError::MissingColumn { .. }));
        assert_eq!(rows[0].opt("B"), Some("2"));
        assert_eq!(rows[0].opt("missing"), None);
    }

    #[test]
    fn fixed_skip_preamble_count_works_when_no_signature() {
        let text = "junk1\njunk2\nA,B\n1,2\n";
        let opts = ReadOpts {
            skip_preamble_lines: 2,
            ..Default::default()
        };
        let rows = read_csv_str(text, TableRole::Single, &opts).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].get("t", "A").unwrap(), "1");
    }

    /// Gemini exports Date/Time as numeric Excel serials in .xlsx files.
    /// This test writes a plain number (no date format) via rust_xlsxwriter so calamine reads it
    /// as Data::Float; cell_to_string formats it with "{}" (shortest-round-trip f64); and
    /// parse_timestamp_flex converts the serial string to UTC. Full numeric→serial→UTC path.
    #[test]
    fn xlsx_numeric_serial_date_roundtrip() {
        use crate::parse::parse_timestamp_flex;
        use rust_xlsxwriter::Workbook;
        use time::macros::datetime;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("gemini_fixture.xlsx");

        let mut wb = Workbook::new();
        let ws = wb.add_worksheet();
        // Header
        ws.write_string(0, 0, "Date").unwrap();
        ws.write_string(0, 1, "Amount").unwrap();
        // Data row: write the Excel serial for 1970-01-01 12:00:00 UTC as a plain number.
        // Serial 25569 = 1970-01-01 00:00 UTC; 0.5 days = 12 hours → serial 25569.5.
        let serial = 25569.5_f64;
        ws.write_number(1, 0, serial).unwrap();
        ws.write_string(1, 1, "0.5").unwrap();
        wb.save(&path).unwrap();

        let rows = read_xlsx(&path, TableRole::Single).unwrap();
        assert_eq!(rows.len(), 1);
        let date_str = rows[0].get("t", "Date").unwrap();
        // The cell was stored as a float; cell_to_string must yield the serial as a string.
        assert_eq!(date_str, "25569.5");
        // parse_timestamp_flex must then convert the serial string to the correct UTC instant.
        let (utc, _) = parse_timestamp_flex("gemini", 1, date_str).unwrap();
        assert_eq!(utc, datetime!(1970-01-01 12:00:00 UTC));
    }
}
