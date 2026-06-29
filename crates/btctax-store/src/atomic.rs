use crate::{paths, StoreError};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;

pub fn atomic_write(target: &Path, bytes: &[u8]) -> Result<(), StoreError> {
    let tmp = paths::tmp_of(target);
    {
        let mut f = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
    }
    if target.exists() {
        // keep a fsync'd backup BEFORE we touch the live file
        let bak = paths::bak_of(target);
        fs::copy(target, &bak)?;
        File::open(&bak)?.sync_all()?;
    }
    fs::rename(&tmp, target)?; // atomic replace; target never absent. (NFR8: std::fs::rename replaces an
                               // existing file on Windows via MoveFileExW(MOVEFILE_REPLACE_EXISTING); its
                               // atomicity is best-effort there, but the fsync'd .bak copied above is the safety net.)
    if let Some(dir) = target.parent() {
        let _ = File::open(dir).and_then(|d| d.sync_all());
    }
    Ok(())
}

/// If the target is missing but a backup exists (e.g., a crash on the very first
/// create before any rename completed), restore it.
pub fn recover_target(target: &Path) -> Result<(), StoreError> {
    if !target.exists() {
        let bak = paths::bak_of(target);
        if bak.exists() {
            fs::copy(&bak, target)?;
        }
    }
    Ok(())
}

/// Remove a stray temp file — only safe once the target is present.
pub fn reap_tmp(target: &Path) -> Result<(), StoreError> {
    let tmp = paths::tmp_of(target);
    if tmp.exists() && target.exists() {
        fs::remove_file(&tmp)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn write_keeps_prev_in_bak_and_target_never_absent() {
        let d = tempfile::tempdir().unwrap();
        let t = d.path().join("vault.pgp");
        atomic_write(&t, b"v1").unwrap();
        assert_eq!(fs::read(&t).unwrap(), b"v1");
        atomic_write(&t, b"v2").unwrap();
        assert_eq!(fs::read(&t).unwrap(), b"v2");
        assert_eq!(fs::read(crate::paths::bak_of(&t)).unwrap(), b"v1");
    }

    #[test]
    fn recover_from_bak_when_target_missing() {
        let d = tempfile::tempdir().unwrap();
        let t = d.path().join("vault.pgp");
        fs::write(crate::paths::bak_of(&t), b"good").unwrap(); // only the bak survived
        recover_target(&t).unwrap();
        assert_eq!(fs::read(&t).unwrap(), b"good");
    }

    #[test]
    fn reap_tmp_only_when_target_present() {
        let d = tempfile::tempdir().unwrap();
        let t = d.path().join("vault.pgp");
        atomic_write(&t, b"good").unwrap();
        fs::write(crate::paths::tmp_of(&t), b"partial").unwrap();
        reap_tmp(&t).unwrap();
        assert!(!crate::paths::tmp_of(&t).exists());
        assert_eq!(fs::read(&t).unwrap(), b"good");
    }
}
