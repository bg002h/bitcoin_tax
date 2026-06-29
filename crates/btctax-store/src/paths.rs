use std::path::{Path, PathBuf};

fn suffixed(p: &Path, suffix: &str) -> PathBuf {
    let mut name = p.file_name().unwrap_or_default().to_os_string();
    name.push(suffix);
    p.with_file_name(name)
}

pub fn tmp_of(p: &Path) -> PathBuf {
    suffixed(p, ".tmp")
}

pub fn bak_of(p: &Path) -> PathBuf {
    suffixed(p, ".bak")
}

pub fn lock_of(p: &Path) -> PathBuf {
    suffixed(p, ".lock")
}

/// Sidecar key path: `vault.pgp` -> `vault.key`. Replacing the extension is safe here
/// (`.key` is distinct from the appended `.tmp`/`.bak`/`.lock` families). Guards against a
/// vault literally named `*.key` (which would collide with its own key file).
pub fn suffixed_key(p: &Path) -> PathBuf {
    let k = p.with_extension("key");
    debug_assert_ne!(
        k, p,
        "call sites pre-check this; vault path must not end in .key"
    ); // M1: create/open return InvalidVaultPath
    k
}

#[cfg(test)]
mod tests {

    #[test]
    fn names_append_not_replace() {
        let p = std::path::Path::new("/x/vault.key");
        assert_eq!(
            crate::paths::tmp_of(p).file_name().unwrap(),
            "vault.key.tmp"
        );
        assert_eq!(
            crate::paths::bak_of(p).file_name().unwrap(),
            "vault.key.bak"
        );
    }

    #[test]
    fn suffixed_key_maps_pgp_to_key() {
        assert_eq!(
            crate::paths::suffixed_key(std::path::Path::new("/x/vault.pgp"))
                .file_name()
                .unwrap(),
            "vault.key"
        );
    }
}
