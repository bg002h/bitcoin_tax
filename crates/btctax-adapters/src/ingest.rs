//! Ingest orchestration: detect each file's source, group (Swan merges its files), dispatch to the
//! parser, and aggregate FR2 counts into one `FileReport` per group. Produces events only — the CLI
//! (Plan 4) persists them via `btctax_core::persistence::append_import_batch`.
use crate::adapter::{Adapter, FileReport, IngestBatch, SourceFile};
use crate::sources::{coinbase::Coinbase, gemini::Gemini, river::River, swan::Swan};
use crate::AdapterError;
use btctax_core::{PriceProvider, Source};
use std::collections::HashMap;
use std::path::PathBuf;

fn adapters() -> Vec<Box<dyn Adapter>> {
    // Detection order: highest-specificity detectors first. Swan/Coinbase/River detect by CSV
    // header tokens (high specificity — content-based). Gemini detects by .xlsx extension alone
    // (very broad: any .xlsx file matches). By running Gemini last, the content-based detectors
    // claim their files first; Gemini only picks up .xlsx files that the others declined.
    // Coinbase and River also explicitly return false for .xlsx extensions, so no cross-source
    // confusion is possible regardless of order — but Gemini last is the correct idiom.
    vec![
        Box::new(Swan),
        Box::new(Coinbase),
        Box::new(River),
        Box::new(Gemini),
    ]
}

/// FR2 capstone. Detect → bucket → group → parse → normalize → report (dropped/unclassified per group).
pub fn ingest_files(
    paths: &[PathBuf],
    prices: &dyn PriceProvider,
) -> Result<IngestBatch, AdapterError> {
    let adapters = adapters();
    let mut buckets: HashMap<Source, Vec<SourceFile>> = HashMap::new();
    for p in paths {
        let f = SourceFile::new(p.clone());
        let mut matched = None;
        for a in &adapters {
            if a.detect(&f)? {
                matched = Some(a.source());
                break;
            }
        }
        match matched {
            Some(s) => buckets.entry(s).or_default().push(f),
            None => {
                return Err(AdapterError::UnknownSource {
                    path: p.display().to_string(),
                })
            }
        }
    }

    let mut batch = IngestBatch::default();
    for a in &adapters {
        let Some(files) = buckets.remove(&a.source()) else {
            continue;
        };
        for group in a.group(files) {
            let rows = a.parse(&group)?;
            let out = a.normalize(&group, rows, prices)?;
            batch.reports.push(FileReport {
                source: group.source,
                label: group.label.clone(),
                parsed_rows: out.parsed_rows,
                btc_events: out.events.len(),
                dropped_no_btc: out.dropped_no_btc,
                unclassified: out.unclassified,
            });
            batch.events.extend(out.events);
        }
    }
    Ok(batch)
}
