//! Cross-platform owner-only permission helpers.
//!
//! All secret-bearing files (`vault.key`, `vault.pgp`, their `.tmp`/`.bak` siblings,
//! and their parent directories) must be restricted to the owning user.
//!
//! * **Unix:**     0o600 for files, 0o700 for directories.
//! * **non-Unix:** plain `std::fs` write / `create_dir_all` (ACL inherited from the
//!   user-profile directory; verified under Windows CI — FOLLOWUPS M-3).
//!
//! This module is the single authoritative definition.  Both `atomic.rs` and `vault.rs`
//! import from here; nothing is duplicated.

use crate::StoreError;
use std::path::Path;

// ── file open/create ──────────────────────────────────────────────────────────

/// Open (create-or-truncate) `path` with owner-only permissions (mode 0o600 on Unix).
/// Returns the open [`std::fs::File`] so the caller can write and/or fsync.
/// On non-Unix the file is opened with default permissions (ACL-inherited).
#[cfg(unix)]
pub fn open_owner_only(path: &Path) -> Result<std::fs::File, StoreError> {
    use std::os::unix::fs::OpenOptionsExt as _;
    Ok(std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?)
}

#[cfg(not(unix))]
pub fn open_owner_only(path: &Path) -> Result<std::fs::File, StoreError> {
    Ok(std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)?)
}

/// Write `data` to `path` with owner-only permissions (mode 0o600 on Unix).
/// Convenience wrapper around [`open_owner_only`] for callers that do not need
/// an explicit fsync (e.g. `export_snapshot`, `backup_key`).
pub fn write_owner_only(path: &Path, data: &[u8]) -> Result<(), StoreError> {
    use std::io::Write as _;
    open_owner_only(path)?.write_all(data)?;
    Ok(())
}

// ── post-copy permission fix ──────────────────────────────────────────────────

/// Restrict an existing file to owner-read/write (0o600) on Unix.
/// Used to harden `.bak` files after `fs::copy`, which carries source permissions
/// but whose result we make explicit for robustness.
/// No-op on non-Unix (ACL-inherited from parent directory).
#[cfg(unix)]
pub fn restrict_file_to_owner(path: &Path) -> Result<(), StoreError> {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[cfg(not(unix))]
pub fn restrict_file_to_owner(_path: &Path) -> Result<(), StoreError> {
    Ok(())
}

// ── directory creation ────────────────────────────────────────────────────────

/// Create `path` (and all parents) with owner-only permissions (mode 0o700 on Unix).
/// On non-Unix platforms uses `create_dir_all` (ACL-inherited).
#[cfg(unix)]
pub fn mkdir_owner_only(path: &Path) -> Result<(), StoreError> {
    use std::os::unix::fs::DirBuilderExt as _;
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(path)?;
    Ok(())
}

#[cfg(not(unix))]
pub fn mkdir_owner_only(path: &Path) -> Result<(), StoreError> {
    std::fs::create_dir_all(path)?;
    Ok(())
}

/// Create `path` with owner-only permissions (0o700 on Unix), FAILING if it already
/// exists (`ErrorKind::AlreadyExists`). NON-recursive: the parent must exist — callers
/// pass a child of an existing directory (the TUI export passes a child of the vault's
/// parent, which always exists). Guarantees the caller receives a FRESH, EMPTY,
/// caller-owned 0o700 directory — the precondition `write_form_csvs` documents.
#[cfg(unix)]
pub fn mkdir_owner_only_exclusive(path: &Path) -> Result<(), StoreError> {
    use std::os::unix::fs::DirBuilderExt as _;
    std::fs::DirBuilder::new()
        .recursive(false)
        .mode(0o700)
        .create(path)?;
    Ok(())
}

#[cfg(not(unix))]
pub fn mkdir_owner_only_exclusive(path: &Path) -> Result<(), StoreError> {
    std::fs::DirBuilder::new().recursive(false).create(path)?;
    Ok(())
}
