use crate::StoreError;
use rusqlite::serialize::OwnedData;
use rusqlite::{Connection, DatabaseName};

pub fn open_in_memory() -> Result<Connection, StoreError> {
    Ok(Connection::open_in_memory()?)
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
            return Err(StoreError::Io(std::io::Error::new(
                std::io::ErrorKind::OutOfMemory,
                "sqlite3_malloc64 failed",
            )));
        }
        std::ptr::copy_nonoverlapping(image.as_ptr(), p, n);
        OwnedData::from_raw_nonnull(std::ptr::NonNull::new(p).unwrap(), n)
    };
    conn.deserialize(DatabaseName::Main, owned, false)?;
    Ok(conn)
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
}
