//! The §9 `Adapter` contract (detect → group → parse → normalize) and the ingest data types. Parsers
//! PRODUCE `LedgerEvent`s; the CLI (Plan 4) persists them via `btctax_core::persistence`.
use crate::read::RawRow;
use crate::AdapterError;
use btctax_core::{LedgerEvent, PriceProvider, Source};
use std::path::PathBuf;

/// One input file on disk (content is read lazily by the reader).
#[derive(Debug, Clone)]
pub struct SourceFile {
    pub path: PathBuf,
}
impl SourceFile {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

/// A unit of ingest: one file for most sources; Swan groups its three files into one batch.
#[derive(Debug, Clone)]
pub struct FileGroup {
    pub source: Source,
    pub label: String,
    pub files: Vec<SourceFile>,
}

/// Per-group parse result: the BTC events (incl. `Unclassified`) + FR2 counts.
#[derive(Debug, Default)]
pub struct GroupOutput {
    pub events: Vec<LedgerEvent>,
    /// FR2: rows with no BTC leg, dropped (not evented).
    pub dropped_no_btc: usize,
    /// FR2: BTC-side rows that became `Unclassified` events (NOT dropped).
    pub unclassified: usize,
    pub parsed_rows: usize,
}
impl GroupOutput {
    pub fn merge(&mut self, o: GroupOutput) {
        self.events.extend(o.events);
        self.dropped_no_btc += o.dropped_no_btc;
        self.unclassified += o.unclassified;
        self.parsed_rows += o.parsed_rows;
    }
}

/// FR2 per-group report surfaced to the CLI ("Report dropped (no-BTC) + unclassified counts per file").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileReport {
    pub source: Source,
    pub label: String,
    pub parsed_rows: usize,
    pub btc_events: usize,
    pub dropped_no_btc: usize,
    pub unclassified: usize,
}

/// The whole ingest result: every BTC event across all groups + one report per group.
#[derive(Debug, Default)]
pub struct IngestBatch {
    pub events: Vec<LedgerEvent>,
    pub reports: Vec<FileReport>,
}

/// §9 Adapter contract. Each impl's doc-comment states its `source_ref`/dedup, gross-vs-net proceeds,
/// fee placement, and unknown-type → `Unclassified` policy (asserted by a fixture test).
pub trait Adapter {
    fn source(&self) -> Source;
    /// True if this adapter recognizes `file` (header/preamble signature or extension). Signatures are
    /// matched against documented §9 tokens / synthetic-fixture headers (confirm vs real exports).
    fn detect(&self, file: &SourceFile) -> Result<bool, AdapterError>;
    /// Group recognized files into ingest units (Swan merges 3 → 1; others 1:1).
    fn group(&self, files: Vec<SourceFile>) -> Vec<FileGroup>;
    /// Parse raw rows from a group (preamble/CRLF handled by the reader; Swan tags rows by role).
    fn parse(&self, group: &FileGroup) -> Result<Vec<RawRow>, AdapterError>;
    /// Map rows → BTC `LedgerEvent`s: FR2 filter (drop no-BTC; unknown BTC → `Unclassified`) + FR3 FMV.
    fn normalize(
        &self,
        group: &FileGroup,
        rows: Vec<RawRow>,
        prices: &dyn PriceProvider,
    ) -> Result<GroupOutput, AdapterError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_output_merge_accumulates_fr2_counts() {
        let mut a = GroupOutput {
            dropped_no_btc: 1,
            unclassified: 2,
            parsed_rows: 3,
            ..Default::default()
        };
        let b = GroupOutput {
            dropped_no_btc: 4,
            unclassified: 5,
            parsed_rows: 6,
            events: Vec::new(),
        };
        a.merge(b);
        assert_eq!((a.dropped_no_btc, a.unclassified, a.parsed_rows), (5, 7, 9));
    }

    #[test]
    fn trait_is_object_safe() {
        fn _takes(_: &dyn Adapter) {}
    }
}
