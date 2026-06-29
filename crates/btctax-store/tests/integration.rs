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
    let snap = v.export_snapshot(d.path()).unwrap();
    let c = rusqlite::Connection::open(&snap).unwrap();
    assert_eq!(
        c.query_row("SELECT x FROM t", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        9
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
    assert!(bytes.starts_with(b"-----BEGIN PGP")); // armored
    assert!(sequoia_openpgp::Cert::from_bytes(&bytes).is_ok());
}
