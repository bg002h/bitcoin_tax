use btctax_store::{Passphrase, StoreError, Vault};
use sequoia_openpgp::parse::Parse;

#[test]
fn create_save_reopen() {
    let d = tempfile::tempdir().unwrap();
    let vp = d.path().join("vault.pgp");
    {
        let mut v = Vault::create(&vp, &Passphrase::new("pw".into())).unwrap();
        v.conn()
            .execute_batch("CREATE TABLE t(x); INSERT INTO t VALUES(7);")
            .unwrap();
        v.save().unwrap();
    }
    let v2 = Vault::open(&vp, &Passphrase::new("pw".into())).unwrap();
    assert_eq!(
        v2.conn()
            .query_row("SELECT x FROM t", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        7
    );
}

#[test]
fn wrong_passphrase() {
    let d = tempfile::tempdir().unwrap();
    let vp = d.path().join("vault.pgp");
    Vault::create(&vp, &Passphrase::new("right".into()))
        .unwrap()
        .save()
        .unwrap();
    assert!(matches!(
        Vault::open(&vp, &Passphrase::new("wrong".into())),
        Err(StoreError::WrongPassphrase)
    ));
}

#[test]
fn second_open_locked() {
    let d = tempfile::tempdir().unwrap();
    let vp = d.path().join("vault.pgp");
    Vault::create(&vp, &Passphrase::new("pw".into()))
        .unwrap()
        .save()
        .unwrap();
    let _a = Vault::open(&vp, &Passphrase::new("pw".into())).unwrap();
    assert!(matches!(
        Vault::open(&vp, &Passphrase::new("pw".into())),
        Err(StoreError::Locked)
    ));
}

#[test]
fn create_refuses_existing() {
    let d = tempfile::tempdir().unwrap();
    let vp = d.path().join("vault.pgp");
    Vault::create(&vp, &Passphrase::new("pw".into()))
        .unwrap()
        .save()
        .unwrap();
    assert!(matches!(
        Vault::create(&vp, &Passphrase::new("pw".into())),
        Err(StoreError::AlreadyExists)
    ));
}

#[test]
fn rejects_dot_key_vault_path() {
    // M1: typed error, not a panic
    let d = tempfile::tempdir().unwrap();
    let bad = d.path().join("vault.key");
    assert!(matches!(
        Vault::create(&bad, &Passphrase::new("pw".into())),
        Err(StoreError::InvalidVaultPath)
    ));
    assert!(matches!(
        Vault::open(&bad, &Passphrase::new("pw".into())),
        Err(StoreError::InvalidVaultPath)
    ));
}

#[test]
fn create_makes_missing_parent_dir() {
    // M3
    let d = tempfile::tempdir().unwrap();
    let vp = d.path().join("sub/dir/vault.pgp");
    Vault::create(&vp, &Passphrase::new("pw".into()))
        .unwrap()
        .save()
        .unwrap();
    assert!(vp.exists());
}

#[test]
fn open_recovers_from_bak_if_target_missing() {
    let d = tempfile::tempdir().unwrap();
    let vp = d.path().join("vault.pgp");
    {
        let mut v = Vault::create(&vp, &Passphrase::new("pw".into())).unwrap();
        v.conn()
            .execute_batch("CREATE TABLE t(x); INSERT INTO t VALUES(5);")
            .unwrap();
        v.save().unwrap();
        v.save().unwrap();
    }
    // simulate a crash that left only the .bak (newest committed copy is in target; older in .bak):
    std::fs::copy(&vp, btctax_store::paths::bak_of(&vp)).unwrap();
    std::fs::remove_file(&vp).unwrap();
    let v = Vault::open(&vp, &Passphrase::new("pw".into())).unwrap();
    assert_eq!(
        v.conn()
            .query_row("SELECT x FROM t", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        5
    );
}

#[test]
fn export_snapshot_is_readable_sqlite() {
    let d = tempfile::tempdir().unwrap();
    let vp = d.path().join("vault.pgp");
    let mut v = Vault::create(&vp, &Passphrase::new("pw".into())).unwrap();
    v.conn()
        .execute_batch("CREATE TABLE t(x); INSERT INTO t VALUES(9);")
        .unwrap();
    v.save().unwrap();
    let out_dir = d.path().join("snap_out");
    let snap = v.export_snapshot(&out_dir).unwrap();
    let c = rusqlite::Connection::open(&snap).unwrap();
    assert_eq!(
        c.query_row("SELECT x FROM t", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        9
    );
    // Verify owner-only permissions on Unix (dir 0o700, file 0o600)
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        let dir_mode = std::fs::metadata(&out_dir).unwrap().mode();
        assert_eq!(
            dir_mode & 0o777,
            0o700,
            "snapshot dir must be owner-only (0o700), got {:04o}",
            dir_mode & 0o777
        );
        let file_mode = std::fs::metadata(&snap).unwrap().mode();
        assert_eq!(
            file_mode & 0o777,
            0o600,
            "snapshot file must be owner-only (0o600), got {:04o}",
            file_mode & 0o777
        );
    }
}

/// Verify that all canonical secret artifacts written by `Vault::create` + `save` are
/// owner-only on Unix (mode 0o600 for files, 0o700 for the vault parent dir).
/// Gate: `#[cfg(unix)]` — Windows relies on ACL inheritance (FOLLOWUPS M-3).
#[cfg(unix)]
#[test]
fn vault_artifacts_are_owner_only() {
    use btctax_store::paths;
    use std::os::unix::fs::MetadataExt as _;

    let d = tempfile::tempdir().unwrap();
    // Put the vault in a sub-directory so we can assert its parent's mode.
    let sub = d.path().join("vaultdir");
    let vp = sub.join("vault.pgp");
    let kp = paths::suffixed_key(&vp); // vault.key

    // create() writes vault.key (via atomic_write) then vault.pgp (via save()).
    let mut v = Vault::create(&vp, &Passphrase::new("pw".into())).unwrap();

    // ── parent directory ──────────────────────────────────────────────────────
    let dir_mode = std::fs::metadata(&sub).unwrap().mode();
    assert_eq!(
        dir_mode & 0o777,
        0o700,
        "vault parent dir must be owner-only (0o700), got {:04o}",
        dir_mode & 0o777
    );

    // ── vault.key ─────────────────────────────────────────────────────────────
    let key_mode = std::fs::metadata(&kp).unwrap().mode();
    assert_eq!(
        key_mode & 0o777,
        0o600,
        "vault.key must be owner-only (0o600), got {:04o}",
        key_mode & 0o777
    );

    // ── vault.pgp ─────────────────────────────────────────────────────────────
    let pgp_mode = std::fs::metadata(&vp).unwrap().mode();
    assert_eq!(
        pgp_mode & 0o777,
        0o600,
        "vault.pgp must be owner-only (0o600), got {:04o}",
        pgp_mode & 0o777
    );

    // ── .bak files (created on second save) ───────────────────────────────────
    v.save().unwrap(); // second save → vault.pgp.bak is created
    let pgp_bak = paths::bak_of(&vp);
    let bak_mode = std::fs::metadata(&pgp_bak).unwrap().mode();
    assert_eq!(
        bak_mode & 0o777,
        0o600,
        "vault.pgp.bak must be owner-only (0o600), got {:04o}",
        bak_mode & 0o777
    );
}

#[test]
fn backup_key_is_armored_and_parseable() {
    let d = tempfile::tempdir().unwrap();
    let vp = d.path().join("vault.pgp");
    let v = Vault::create(&vp, &Passphrase::new("pw".into())).unwrap();
    let kp = d.path().join("backup.asc");
    v.backup_key(&kp).unwrap();
    let bytes = std::fs::read(&kp).unwrap();

    // M-1: full armored header (not just a prefix)
    assert!(
        bytes.starts_with(b"-----BEGIN PGP PRIVATE KEY BLOCK-----"),
        "backup must open with the PGP private-key armor header"
    );

    // I-1: parse and assert the key is STILL S2K-encrypted
    let cert = sequoia_openpgp::Cert::from_bytes(&bytes).unwrap();
    // (a) the backup must be a TSK (carries secret key material)
    assert!(
        cert.is_tsk(),
        "backed-up key must contain secret key material"
    );
    // (b) every secret-bearing key must have ENCRYPTED (passphrase-protected) secrets —
    //     has_unencrypted_secret() == false means the secret is not in plaintext
    for ka in cert.keys().secret() {
        assert!(
            !ka.key().has_unencrypted_secret(),
            "backed-up key material must be S2K-encrypted, not plaintext"
        );
    }

    // Verify owner-only file permissions on Unix (mode 0o600)
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        let mode = std::fs::metadata(&kp).unwrap().mode();
        assert_eq!(
            mode & 0o777,
            0o600,
            "backup file must be owner-only (0o600), got {:04o}",
            mode & 0o777
        );
    }
}
