// Cross-platform (NFR8): fs2's try_lock_exclusive maps to flock(LOCK_EX|LOCK_NB) on Unix
// and LockFileEx(LOCKFILE_EXCLUSIVE_LOCK|LOCKFILE_FAIL_IMMEDIATELY) on Windows.
use crate::{paths, StoreError};
use fs2::FileExt;
use std::fs::{File, OpenOptions};
use std::path::Path;

#[derive(Debug)]
pub struct VaultLock(File);

impl VaultLock {
    pub fn acquire(vault: &Path) -> Result<VaultLock, StoreError> {
        let f = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(paths::lock_of(vault))?;
        match f.try_lock_exclusive() {
            Ok(()) => Ok(VaultLock(f)),
            Err(e) if is_contention(&e) => Err(StoreError::Locked),
            Err(e) => Err(StoreError::Io(e)),
        }
    }
}

/// Is this `try_lock_exclusive` error lock contention (the file is already locked), as opposed to a
/// real I/O failure?
///
/// Platforms surface contention differently:
/// - **Unix:** `flock`/`EWOULDBLOCK` → `ErrorKind::WouldBlock`.
/// - **Windows:** `LockFileEx(LOCKFILE_FAIL_IMMEDIATELY)` fails with `ERROR_LOCK_VIOLATION` (33) — and,
///   for a sharing conflict, `ERROR_SHARING_VIOLATION` (32). Current stable std does **NOT** normalize
///   33 to `WouldBlock` (verified empirically on `windows-latest` CI — an earlier assumption that it did
///   was wrong), so match the raw OS codes directly. The codes are Windows-specific, hence `cfg(windows)`
///   (32/33 mean unrelated errors — `EPIPE`/`EDOM` — on Unix).
fn is_contention(e: &std::io::Error) -> bool {
    if e.kind() == std::io::ErrorKind::WouldBlock {
        return true;
    }
    #[cfg(windows)]
    if matches!(e.raw_os_error(), Some(32) | Some(33)) {
        return true;
    }
    false
}

impl Drop for VaultLock {
    fn drop(&mut self) {
        let _ = fs2::FileExt::unlock(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_acquire_refused() {
        let d = tempfile::tempdir().unwrap();
        let v = d.path().join("vault.pgp");
        let a = VaultLock::acquire(&v).unwrap();
        let second = VaultLock::acquire(&v);
        assert!(
            matches!(second, Err(StoreError::Locked)),
            "second acquire must be refused as Locked (contention); got {second:?}"
        );
        drop(a);
        assert!(VaultLock::acquire(&v).is_ok());
    }
}
