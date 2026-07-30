use crate::StoreError;
use rusqlite::serialize::OwnedData;
use rusqlite::{Connection, DatabaseName};

/// ★★★ **§G-16 — every connection MUST have `secure_delete` on.**
///
/// The vault is one whole-image ciphertext: [`db_to_bytes`] serializes **every page, free ones
/// included**, and `Vault::save` encrypts that image. Without this pragma SQLite frees a deleted
/// row's pages *without overwriting them*, so the content rides into every subsequent vault
/// generation, indefinitely — while the calling code reads as a deletion.
///
/// That was live: `input_form_store::delete_draft` is a plain `DELETE`, and the draft row holds
/// SSNs, DOBs and every superseded income figure the filer typed and discarded. Its real retention
/// was **unbounded and invisible** — strictly worse than `.bak`'s bounded single generation.
///
/// ★ Set here rather than at each call site, so it cannot be forgotten by the next `DELETE`. It is
/// also the prerequisite §G-14's shred needs: a tombstone whose content survives in free pages is a
/// shred that did not happen.
fn harden(conn: Connection) -> Result<Connection, StoreError> {
    conn.pragma_update(None, "secure_delete", "ON")?;
    Ok(conn)
}

pub fn open_in_memory() -> Result<Connection, StoreError> {
    harden(Connection::open_in_memory()?)
}

pub fn db_to_bytes(conn: &Connection) -> Result<Vec<u8>, StoreError> {
    match conn.serialize(DatabaseName::Main) {
        Ok(data) => Ok(data.to_vec()),
        Err(e) => {
            // sqlite3_serialize returns NULL (rusqlite maps it to SQLITE_NOMEM / "not an error")
            // when the database has never been written to (0 pages).  Distinguish this from a
            // genuine OOM by checking page_count: 0 pages → return empty bytes so the caller can
            // round-trip an empty vault; any other NOMEM is a real allocation failure.
            if e.sqlite_error_code() == Some(rusqlite::ErrorCode::OutOfMemory) {
                let pages: i64 = conn.query_row("PRAGMA page_count", [], |r| r.get(0))?;
                if pages == 0 {
                    return Ok(Vec::new());
                }
            }
            Err(StoreError::Sqlite(e))
        }
    }
}

pub fn db_from_bytes(image: &[u8]) -> Result<Connection, StoreError> {
    // Empty image means the database was serialized before any pages were allocated
    // (see db_to_bytes).  Just return a fresh in-memory connection.
    if image.is_empty() {
        return open_in_memory();
    }
    let mut conn = Connection::open_in_memory()?;
    // SQLite owns deserialized memory; it must be allocated by sqlite3_malloc64.
    let owned = unsafe {
        let n = image.len();
        let p = rusqlite::ffi::sqlite3_malloc64(n as u64) as *mut u8;
        if p.is_null() {
            // [R0-M2] LOAD-BEARING for the vault corruption classifier: an allocation failure is
            // remapped to `StoreError::Io` (OutOfMemory), NEVER `StoreError::Sqlite`. Vault::open's
            // `is_genuine_corruption` treats `Sqlite` as genuine corruption (→ `.bak` recovery) and
            // `Io` as non-corruption (→ propagate). Do NOT change this to a `Sqlite`/`Corrupt`
            // variant: an OOM must not be misread as corruption and silently trigger a `.bak` revert.
            return Err(StoreError::Io(std::io::Error::new(
                std::io::ErrorKind::OutOfMemory,
                "sqlite3_malloc64 failed",
            )));
        }
        std::ptr::copy_nonoverlapping(image.as_ptr(), p, n);
        OwnedData::from_raw_nonnull(std::ptr::NonNull::new(p).unwrap(), n)
    };
    conn.deserialize(DatabaseName::Main, owned, false)?;
    // ★ §G-16 — AFTER the deserialize, so the pragma is not clobbered by the swapped-in database.
    // An existing vault reopened without it would keep leaking deleted rows into every save.
    harden(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_roundtrip() {
        let c = open_in_memory().unwrap();
        c.execute_batch("CREATE TABLE t(x); INSERT INTO t VALUES(42);")
            .unwrap();
        let b = db_to_bytes(&c).unwrap();
        let c2 = db_from_bytes(&b).unwrap();
        let x: i64 = c2.query_row("SELECT x FROM t", [], |r| r.get(0)).unwrap();
        assert_eq!(x, 42);
    }

    #[test]
    fn empty_db_roundtrip() {
        // An in-memory database with 0 pages must survive the serialize/deserialize cycle.
        let c = open_in_memory().unwrap();
        let bytes = db_to_bytes(&c).unwrap();
        assert!(
            bytes.is_empty(),
            "fresh empty db should serialize to empty bytes"
        );
        let c2 = db_from_bytes(&bytes).unwrap();
        // Confirm it is a functional empty database.
        c2.execute_batch("CREATE TABLE t(x);").unwrap();
    }

    /// ★★★ **§G-16 — a DELETE must not leave the deleted bytes in the serialized image.**
    ///
    /// The vault is one whole-image ciphertext: [`db_to_bytes`] serializes **every page**, free ones
    /// included, and `Vault::save` encrypts that. So without `secure_delete`, SQLite frees a deleted
    /// row's pages **without overwriting them** and the content rides into every subsequent vault
    /// generation, indefinitely — while the calling code reads as a deletion.
    ///
    /// This is not hypothetical: `input_form_store::delete_draft` is a plain `DELETE`, and the draft
    /// row holds SSNs, DOBs and every superseded income figure the filer typed and discarded.
    ///
    /// ★ The sentinel is searched for as RAW BYTES in the image, because that is exactly how it would
    /// leak — a row-level query would report the row gone and prove nothing.
    #[test]
    fn deleted_content_does_not_survive_into_the_serialized_image() {
        let secret = b"ZZ-G16-SENTINEL-987-65-4321";
        for (label, mut conn) in [
            ("fresh", open_in_memory().unwrap()),
            (
                "deserialized",
                db_from_bytes(&{
                    let c = open_in_memory().unwrap();
                    c.execute_batch("CREATE TABLE seed(x);").unwrap();
                    db_to_bytes(&c).unwrap()
                })
                .unwrap(),
            ),
        ] {
            conn.execute_batch("CREATE TABLE draft(v TEXT);").unwrap();
            // Enough rows to occupy real pages, so the free-page path is actually exercised.
            for i in 0..64 {
                conn.execute(
                    "INSERT INTO draft(v) VALUES(?1)",
                    [format!("{}-{i}", String::from_utf8_lossy(secret))],
                )
                .unwrap();
            }
            assert!(
                contains(&db_to_bytes(&conn).unwrap(), secret),
                "{label}: sentinel must be present BEFORE the delete — otherwise this test proves nothing"
            );

            conn.execute("DELETE FROM draft", []).unwrap();
            let image = db_to_bytes(&conn).unwrap();

            assert!(
                !contains(&image, secret),
                "{label}: deleted content SURVIVES in the serialized image ({} bytes). A DELETE that \
                 leaves its bytes behind is not a deletion — every subsequent vault generation would \
                 carry the discarded SSNs/DOBs. See FOLLOWUPS §G-16.",
                image.len()
            );
            let _ = &mut conn;
        }
    }

    fn contains(haystack: &[u8], needle: &[u8]) -> bool {
        haystack.windows(needle.len()).any(|w| w == needle)
    }
}
