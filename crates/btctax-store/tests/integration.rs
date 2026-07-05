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

// ── T2: recover from `.bak` on a GENUINELY-CORRUPT present vault (M-1) ────────────

/// Build a saved vault holding `t.x == 5`, then drop it (releasing the lock) and return the
/// vault path plus its good ciphertext bytes. The `.key` sidecar stays on disk next to it.
fn make_saved_vault(d: &std::path::Path) -> (std::path::PathBuf, Vec<u8>) {
    let vp = d.join("vault.pgp");
    {
        let mut v = Vault::create(&vp, &Passphrase::new("pw".into())).unwrap();
        v.conn()
            .execute_batch("CREATE TABLE t(x); INSERT INTO t VALUES(5);")
            .unwrap();
        v.save().unwrap();
    }
    let good = std::fs::read(&vp).unwrap();
    (vp, good)
}

/// Deterministically corrupt ciphertext: flip a byte in the SEIP body. With the CORRECT
/// passphrase this always classifies as `Crypto` (the key unlocks → `unlocked=true`), never
/// `WrongPassphrase` — i.e. GENUINE corruption. (No RNG — NFR4 determinism.)
fn corrupt(mut b: Vec<u8>) -> Vec<u8> {
    let i = b.len() / 2;
    b[i] ^= 0xFF;
    b
}

/// [T2 KAT] A genuinely-corrupt present `vault.pgp` with a good `.bak` recovers the `.bak`
/// state, restores `vault.pgp`, and leaves `.bak` STILL present (the safety net is preserved).
#[test]
fn open_recovers_from_bak_when_target_genuinely_corrupt() {
    let d = tempfile::tempdir().unwrap();
    let (vp, good) = make_saved_vault(d.path());
    let bak = btctax_store::paths::bak_of(&vp);
    // Good `.bak`, corrupt target.
    std::fs::write(&bak, &good).unwrap();
    std::fs::write(&vp, corrupt(good.clone())).unwrap();

    // Recovers (a WARNING is emitted to stderr — precedent memlock.rs, not stderr-captured here).
    let v = Vault::open(&vp, &Passphrase::new("pw".into())).unwrap();
    assert_eq!(
        v.conn()
            .query_row("SELECT x FROM t", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        5,
        "recovered vault must hold the `.bak` row"
    );
    drop(v); // release the lock before inspecting files
    assert_eq!(
        std::fs::read(&vp).unwrap(),
        good,
        "vault.pgp must be RESTORED to the good `.bak` bytes"
    );
    assert!(bak.exists(), "`.bak` must STILL be present after recovery");
    assert_eq!(
        std::fs::read(&bak).unwrap(),
        good,
        "`.bak` must be untouched (it stays the safety net)"
    );
}

/// [T2 KAT / R0-C1] A NEWER-schema present vault (decode SUCCEEDS, version > current) with an
/// OLDER good `.bak` must propagate `UnsupportedSchema` and NEVER recover — recovering the `.bak`
/// would silently DOWNGRADE and lose newer tax data. `.bak` untouched; target NOT downgraded.
#[test]
fn open_unsupported_schema_never_recovers_from_bak() {
    let d = tempfile::tempdir().unwrap();
    let (vp, good) = make_saved_vault(d.path());
    let bak = btctax_store::paths::bak_of(&vp);
    let kp = btctax_store::paths::suffixed_key(&vp);

    // Craft a ciphertext whose blob decodes to a FUTURE schema version (decode succeeds → migrate
    // rejects with UnsupportedSchema). Uses the vault's own cert so decryption succeeds.
    let cert = sequoia_openpgp::Cert::from_bytes(&std::fs::read(&kp).unwrap()).unwrap();
    let mut future_blob = (btctax_store::SCHEMA_VERSION + 1).to_be_bytes().to_vec();
    future_blob.extend_from_slice(b"img"); // image body is irrelevant — migrate fails first
    let newer_ct = btctax_store::crypto::encrypt_to(&cert, &future_blob).unwrap();

    std::fs::write(&bak, &good).unwrap(); // OLDER good copy
    std::fs::write(&vp, &newer_ct).unwrap(); // NEWER present vault

    let err = Vault::open(&vp, &Passphrase::new("pw".into())).unwrap_err();
    assert!(
        matches!(err, StoreError::UnsupportedSchema(v) if v == btctax_store::SCHEMA_VERSION + 1),
        "expected UnsupportedSchema, got: {err:?}"
    );
    assert_eq!(
        std::fs::read(&vp).unwrap(),
        newer_ct,
        "NEWER vault must NOT be downgraded/restored from `.bak`"
    );
    assert_eq!(
        std::fs::read(&bak).unwrap(),
        good,
        "`.bak` must be untouched (no recovery attempted)"
    );
}

/// [T2 KAT] A wrong passphrase must propagate `WrongPassphrase` and NEVER touch `.bak`
/// (caller error, not corruption; the `.bak` shares the key and would also fail).
#[test]
fn open_wrong_passphrase_never_touches_bak() {
    let d = tempfile::tempdir().unwrap();
    let (vp, good) = make_saved_vault(d.path());
    let bak = btctax_store::paths::bak_of(&vp);
    std::fs::write(&bak, &good).unwrap();

    let err = Vault::open(&vp, &Passphrase::new("WRONG".into())).unwrap_err();
    assert!(
        matches!(err, StoreError::WrongPassphrase),
        "expected WrongPassphrase, got: {err:?}"
    );
    assert_eq!(
        std::fs::read(&vp).unwrap(),
        good,
        "target must be untouched on WrongPassphrase"
    );
    assert_eq!(
        std::fs::read(&bak).unwrap(),
        good,
        "`.bak` must be untouched on WrongPassphrase"
    );
}

/// [T2 KAT] When BOTH the present vault and its `.bak` are corrupt, `open` propagates the
/// ORIGINAL target error (a clear, bounded single attempt — never a panic/loop) and the
/// `.bak` bytes are left intact.
#[test]
fn open_both_corrupt_propagates_and_bak_intact() {
    let d = tempfile::tempdir().unwrap();
    let (vp, good) = make_saved_vault(d.path());
    let bak = btctax_store::paths::bak_of(&vp);
    let corrupt_bak = corrupt({
        let mut g = good.clone();
        g[good.len() / 3] ^= 0xAA; // a DIFFERENT corruption than the target's
        g
    });
    std::fs::write(&vp, corrupt(good.clone())).unwrap();
    std::fs::write(&bak, &corrupt_bak).unwrap();

    let err = Vault::open(&vp, &Passphrase::new("pw".into())).unwrap_err();
    assert!(
        matches!(
            err,
            StoreError::Crypto(_) | StoreError::Corrupt(_) | StoreError::Sqlite(_)
        ),
        "both-corrupt must yield the original corruption error, got: {err:?}"
    );
    assert_eq!(
        std::fs::read(&bak).unwrap(),
        corrupt_bak,
        "`.bak` must be intact (untouched) even when unusable"
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
