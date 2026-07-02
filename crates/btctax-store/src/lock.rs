// Cross-platform (NFR8): fs2's try_lock_exclusive maps to flock(LOCK_EX|LOCK_NB) on Unix
// and LockFileEx(LOCKFILE_EXCLUSIVE_LOCK|LOCKFILE_FAIL_IMMEDIATELY) on Windows.
use crate::{paths, StoreError};
use fs2::FileExt;
use std::fs::{File, OpenOptions};
use std::path::Path;

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
            // On contention fs2 surfaces WouldBlock: Unix EWOULDBLOCK; Windows ERROR_LOCK_VIOLATION(33)
            // mapped to WouldBlock by Rust >=1.64's decode_error_kind (PR #95306) — MSRV (1.88; ≥1.64 required) satisfies this.
            // If MSRV is ever lowered below 1.64, fall back to e.raw_os_error()==Some(33). (fs2 0.4 is dormant;
            // fd-lock is a maintained alternative that normalizes this mapping — see FOLLOWUPS.)
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Err(StoreError::Locked),
            Err(e) => Err(StoreError::Io(e)),
        }
    }
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
        assert!(matches!(VaultLock::acquire(&v), Err(StoreError::Locked)));
        drop(a);
        assert!(VaultLock::acquire(&v).is_ok());
    }
}
