use crate::StoreError;
use rusqlite::serialize::OwnedData;
use rusqlite::{Connection, DatabaseName};

pub fn open_in_memory() -> Result<Connection, StoreError> {
    Ok(Connection::open_in_memory()?)
}

pub fn db_to_bytes(conn: &Connection) -> Result<Vec<u8>, StoreError> {
    let data = conn.serialize(DatabaseName::Main)?; // Data: Deref<Target=[u8]>
    Ok(data.to_vec())
}

pub fn db_from_bytes(image: &[u8]) -> Result<Connection, StoreError> {
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
}
