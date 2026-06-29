//! THE ONLY I/O in btctax-core. Reads/appends canonical event rows over a borrowed rusqlite handle
//! (the live in-memory DB from btctax-store::Vault::conn(), wired by the CLI). Owns the events-table DDL,
//! event (de)serialization, decision_seq allocation, and FR1 fingerprint-based conflict detection.
use crate::event::*;
use crate::identity::{EventId, Fingerprint, Source, SourceRef, WalletId};
use crate::CoreError;
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use time::{OffsetDateTime, UtcOffset};

const KIND_IMPORT: &str = "import";
const KIND_CONFLICT: &str = "conflict";
const KIND_DECISION: &str = "decision";

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ImportReport {
    pub appended: usize,
    pub duplicates: usize,
    pub conflicts: usize,
}

/// Canonical content fingerprint for the six imported payloads (None for system/decision payloads).
/// Normalizes Decimal scale (`.normalize()`) and trims string fields so whitespace/scale/CRLF
/// re-exports are idempotent (§13). Field order is FIXED (§6.2).
pub fn fingerprint(p: &EventPayload) -> Option<Fingerprint> {
    let mut b: Vec<u8> = Vec::new();
    fn d(b: &mut Vec<u8>, v: &crate::Usd) {
        b.extend_from_slice(v.normalize().to_string().as_bytes());
        b.push(0x1e);
    }
    fn od(b: &mut Vec<u8>, v: &Option<crate::Usd>) {
        match v {
            Some(x) => b.extend_from_slice(x.normalize().to_string().as_bytes()),
            None => b.extend_from_slice(b"\x00none"),
        }
        b.push(0x1e);
    }
    fn s(b: &mut Vec<u8>, v: &str) {
        b.extend_from_slice(v.trim().as_bytes());
        b.push(0x1e);
    }
    fn os(b: &mut Vec<u8>, v: &Option<String>) {
        match v {
            Some(x) => b.extend_from_slice(x.trim().as_bytes()),
            None => b.extend_from_slice(b"\x00none"),
        }
        b.push(0x1e);
    }
    fn i(b: &mut Vec<u8>, v: i64) {
        b.extend_from_slice(v.to_string().as_bytes());
        b.push(0x1e);
    }
    fn oi(b: &mut Vec<u8>, v: Option<i64>) {
        i(b, v.unwrap_or(i64::MIN));
    }
    match p {
        EventPayload::Acquire(a) => {
            b.extend_from_slice(b"acquire\x1e");
            i(&mut b, a.sat);
            d(&mut b, &a.usd_cost);
            d(&mut b, &a.fee_usd);
            b.extend_from_slice(format!("{:?}", a.basis_source).as_bytes());
        }
        EventPayload::Income(x) => {
            b.extend_from_slice(b"income\x1e");
            i(&mut b, x.sat);
            od(&mut b, &x.usd_fmv);
            b.extend_from_slice(
                format!("{:?}/{:?}/{}", x.fmv_status, x.kind, x.business).as_bytes(),
            );
        }
        EventPayload::Dispose(x) => {
            b.extend_from_slice(b"dispose\x1e");
            i(&mut b, x.sat);
            d(&mut b, &x.usd_proceeds);
            d(&mut b, &x.fee_usd);
            b.extend_from_slice(format!("{:?}", x.kind).as_bytes());
        }
        EventPayload::TransferOut(x) => {
            b.extend_from_slice(b"transfer_out\x1e");
            i(&mut b, x.sat);
            oi(&mut b, x.fee_sat);
            os(&mut b, &x.dest_addr);
            os(&mut b, &x.txid);
        }
        EventPayload::TransferIn(x) => {
            b.extend_from_slice(b"transfer_in\x1e");
            i(&mut b, x.sat);
            os(&mut b, &x.src_addr);
            os(&mut b, &x.txid);
        }
        EventPayload::Unclassified(x) => {
            b.extend_from_slice(b"unclassified\x1e");
            s(&mut b, &x.raw);
        }
        _ => return None,
    }
    Some(Fingerprint::of_bytes(&b))
}

pub fn init_schema(conn: &Connection) -> Result<(), CoreError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS events (
            ordinal       INTEGER PRIMARY KEY AUTOINCREMENT, -- insertion order ONLY; projection ignores it (NFR4)
            event_id      TEXT NOT NULL UNIQUE,
            kind          TEXT NOT NULL,
            source        TEXT,
            source_ref    TEXT,
            decision_seq  INTEGER,
            utc_timestamp TEXT NOT NULL,
            tz_offset_sec INTEGER NOT NULL,
            wallet_json   TEXT,
            payload_json  TEXT NOT NULL,
            fingerprint   TEXT
        );
        CREATE INDEX IF NOT EXISTS events_srcref ON events(source, source_ref);",
    )?;
    Ok(())
}

fn source_tag(s: &str) -> Option<Source> {
    match s {
        "swan" => Some(Source::Swan),
        "coinbase" => Some(Source::Coinbase),
        "gemini" => Some(Source::Gemini),
        "river" => Some(Source::River),
        _ => None,
    }
}

fn insert(
    conn: &Connection,
    ev: &LedgerEvent,
    kind: &str,
    fp: Option<&Fingerprint>,
) -> Result<(), CoreError> {
    let (source, source_ref, seq) = match &ev.id {
        EventId::Import { source, source_ref } => (
            Some(source.tag().to_string()),
            Some(source_ref.0.clone()),
            None,
        ),
        EventId::Conflict {
            source, source_ref, ..
        } => (
            Some(source.tag().to_string()),
            Some(source_ref.0.clone()),
            None,
        ),
        EventId::Decision { seq } => (None, None, Some(*seq as i64)),
    };
    conn.execute(
        "INSERT INTO events
          (event_id, kind, source, source_ref, decision_seq, utc_timestamp, tz_offset_sec, wallet_json, payload_json, fingerprint)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        rusqlite::params![
            ev.id.canonical(),
            kind,
            source,
            source_ref,
            seq,
            ev.utc_timestamp.format(&time::format_description::well_known::Rfc3339).map_err(|e| CoreError::Persistence(e.to_string()))?,
            ev.original_tz.whole_seconds(),
            ev.wallet.as_ref().map(serde_json::to_string).transpose()?,
            serde_json::to_string(&ev.payload)?,
            fp.map(|f| f.0.clone()),
        ],
    )?;
    Ok(())
}

pub fn append_import_batch(
    conn: &Connection,
    events: &[LedgerEvent],
) -> Result<ImportReport, CoreError> {
    let tx = conn.unchecked_transaction()?; // ATOMIC batch (FR1); &Connection => unchecked_transaction
    let mut report = ImportReport::default();
    for ev in events {
        let fp = fingerprint(&ev.payload).ok_or_else(|| {
            CoreError::Persistence("append_import_batch given a non-imported payload".into())
        })?;
        let (source, source_ref) = match &ev.id {
            EventId::Import { source, source_ref } => (*source, source_ref.clone()),
            _ => {
                return Err(CoreError::Persistence(
                    "imported events must carry EventId::Import".into(),
                ))
            }
        };
        // existing event with same (source, source_ref)?
        let existing_fp: Option<String> = tx
            .query_row(
                "SELECT fingerprint FROM events WHERE kind=?1 AND source=?2 AND source_ref=?3 LIMIT 1",
                rusqlite::params![KIND_IMPORT, source.tag(), source_ref.0],
                |r| r.get(0),
            )
            .optional()?;
        match existing_fp {
            None => {
                insert(&tx, ev, KIND_IMPORT, Some(&fp))?;
                report.appended += 1;
            }
            Some(prev) if prev == fp.0 => {
                report.duplicates += 1; // idempotent no-op
            }
            Some(_) => {
                // changed content -> ONE ImportConflict, distinct id (idempotent on the identical change)
                let conflict_id = EventId::conflict(source, source_ref.clone(), &fp);
                let already: i64 = tx.query_row(
                    "SELECT COUNT(*) FROM events WHERE event_id=?1",
                    rusqlite::params![conflict_id.canonical()],
                    |r| r.get(0),
                )?;
                if already == 0 {
                    let conflict = LedgerEvent {
                        id: conflict_id,
                        utc_timestamp: ev.utc_timestamp,
                        original_tz: ev.original_tz,
                        wallet: ev.wallet.clone(),
                        payload: EventPayload::ImportConflict(ImportConflict {
                            target: EventId::import(source, source_ref),
                            new_payload: Box::new(ev.payload.clone()),
                            new_fingerprint: fp.clone(),
                        }),
                    };
                    insert(&tx, &conflict, KIND_CONFLICT, Some(&fp))?;
                    report.conflicts += 1;
                } else {
                    report.duplicates += 1;
                }
            }
        }
    }
    tx.commit()?;
    Ok(report)
}

pub fn append_decision(
    conn: &Connection,
    payload: EventPayload,
    utc_timestamp: OffsetDateTime,
    original_tz: UtcOffset,
    wallet: Option<WalletId>,
) -> Result<EventId, CoreError> {
    let tx = conn.unchecked_transaction()?;
    let next: i64 = tx.query_row(
        "SELECT COALESCE(MAX(decision_seq),0)+1 FROM events WHERE kind=?1",
        [KIND_DECISION],
        |r| r.get(0),
    )?;
    let id = EventId::decision(next as u64);
    let ev = LedgerEvent {
        id: id.clone(),
        utc_timestamp,
        original_tz,
        wallet,
        payload,
    };
    insert(&tx, &ev, KIND_DECISION, None)?;
    tx.commit()?;
    Ok(id)
}

pub fn load_all(conn: &Connection) -> Result<Vec<LedgerEvent>, CoreError> {
    // SELECT the persisted identity columns and rebuild `EventId` DIRECTLY from them (no re-derivation,
    // no ambiguity — M5). Order is irrelevant; the projection re-sorts canonically (NFR4).
    let mut stmt = conn.prepare(
        "SELECT kind, source, source_ref, decision_seq, utc_timestamp, tz_offset_sec, wallet_json, payload_json FROM events",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,         // kind
            r.get::<_, Option<String>>(1)?, // source
            r.get::<_, Option<String>>(2)?, // source_ref
            r.get::<_, Option<i64>>(3)?,    // decision_seq
            r.get::<_, String>(4)?,         // utc_timestamp
            r.get::<_, i32>(5)?,            // tz_offset_sec
            r.get::<_, Option<String>>(6)?, // wallet_json
            r.get::<_, String>(7)?,         // payload_json
        ))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (kind, source, source_ref, decision_seq, ts, off, wallet_json, payload_json) = row?;
        let utc_timestamp =
            OffsetDateTime::parse(&ts, &time::format_description::well_known::Rfc3339)
                .map_err(|e| CoreError::Persistence(e.to_string()))?;
        let original_tz = UtcOffset::from_whole_seconds(off)
            .map_err(|e| CoreError::Persistence(e.to_string()))?;
        let payload: EventPayload = serde_json::from_str(&payload_json)?;
        let wallet: Option<WalletId> = wallet_json.map(|w| serde_json::from_str(&w)).transpose()?;
        let bad = |m: &str| CoreError::Persistence(format!("corrupt identity row: {m}"));
        let id = match kind.as_str() {
            KIND_DECISION => {
                EventId::decision(decision_seq.ok_or_else(|| bad("decision without seq"))? as u64)
            }
            KIND_IMPORT => {
                let src = source_tag(&source.ok_or_else(|| bad("import without source"))?)
                    .ok_or_else(|| bad("unknown source"))?;
                EventId::import(
                    src,
                    SourceRef::new(source_ref.ok_or_else(|| bad("import without source_ref"))?),
                )
            }
            KIND_CONFLICT => {
                let src = source_tag(&source.ok_or_else(|| bad("conflict without source"))?)
                    .ok_or_else(|| bad("unknown source"))?;
                let sref =
                    SourceRef::new(source_ref.ok_or_else(|| bad("conflict without source_ref"))?);
                // The conflict's fingerprint is part of its identity; recover it from the stored payload.
                let fp = match &payload {
                    EventPayload::ImportConflict(c) => c.new_fingerprint.clone(),
                    _ => return Err(bad("conflict row without ImportConflict payload")),
                };
                EventId::conflict(src, sref, &fp)
            }
            other => return Err(bad(other)),
        };
        out.push(LedgerEvent {
            id,
            utc_timestamp,
            original_tz,
            wallet,
            payload,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_tag_round_trip_all_variants() {
        // Ensure that every Source variant can round-trip through tag() -> source_tag().
        // Adding a new Source variant without updating source_tag() will fail this test.
        let variants = [
            Source::Swan,
            Source::Coinbase,
            Source::Gemini,
            Source::River,
        ];

        for variant in &variants {
            let tag_str = variant.tag();
            let recovered = source_tag(tag_str).unwrap_or_else(|| {
                panic!(
                    "source_tag('{}') should round-trip from variant {:?}",
                    tag_str, variant
                )
            });
            assert_eq!(
                recovered, *variant,
                "source_tag round-trip failed for {:?}",
                variant
            );
        }
    }
}
