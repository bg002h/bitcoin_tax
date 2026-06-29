use sequoia_openpgp as openpgp;
use openpgp::cert::CertBuilder;
use openpgp::serialize::stream::{Encryptor2, LiteralWriter, Message};
use openpgp::parse::{Parse, stream::{DecryptorBuilder, DecryptionHelper, VerificationHelper, MessageStructure}};
use openpgp::policy::StandardPolicy;
use std::io::Write;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

#[test]
fn sequoia_roundtrip_with_shared_unlock_flag_and_strong_s2k() {
    let p = StandardPolicy::new();
    let (cert, _rev) = CertBuilder::new()
        .add_userid("vault@btctax.local")
        .add_storage_encryption_subkey()
        .set_password(Some("hunter2".into()))
        .generate().unwrap();

    // R3: EXTRACT and ASSERT the S2K actually applied to the secret key (record in FOLLOWUPS).
    // Sequoia 1.21 has no Argon2 S2K variant; the strongest available is iterated-salted SHA-256
    // (the spec §8 "else high-work-factor iterated-salted" fallback). Confirm it is Iterated, not
    // a weaker simple/salted-only S2K. (Confirm the exact accessor `Encrypted::s2k()` in the pinned ver.)
    use openpgp::packet::key::SecretKeyMaterial;
    use openpgp::crypto::S2K;
    let mut saw_iterated = false;
    for ka in cert.keys().secret() {
        if let SecretKeyMaterial::Encrypted(e) = ka.key().secret() {
            match e.s2k() {
                S2K::Iterated { hash, hash_bytes, .. } => {
                    eprintln!("secret-key S2K = Iterated{{hash={:?}, hash_bytes={}}}", hash, hash_bytes);
                    saw_iterated = true;
                }
                other => panic!("weak S2K {:?}; pin a stronger one before proceeding", other),
            }
        }
    }
    assert!(saw_iterated, "expected an encrypted secret key protected by an Iterated S2K");

    // encrypt (Encryptor2)
    let recips = cert.keys().with_policy(&p, None).supported()
        .for_storage_encryption().map(|ka| ka.key()).collect::<Vec<_>>();
    let mut ct = Vec::new();
    {
        let m = Message::new(&mut ct);
        let m = Encryptor2::for_recipients(m, recips).build().unwrap();
        let mut w = LiteralWriter::new(m).build().unwrap();
        w.write_all(b"hello").unwrap();
        w.finalize().unwrap();
    }

    // decrypt with a SHARED unlocked-flag (observable on Ok or Err) — the wrong-passphrase mechanism
    struct H { cert: openpgp::Cert, pw: openpgp::crypto::Password, unlocked: Arc<AtomicBool> }
    impl VerificationHelper for H {
        fn get_certs(&mut self, _: &[openpgp::KeyHandle]) -> openpgp::Result<Vec<openpgp::Cert>> { Ok(vec![]) }
        fn check(&mut self, _: MessageStructure) -> openpgp::Result<()> { Ok(()) }
    }
    impl DecryptionHelper for H {
        fn decrypt<D>(&mut self, pkesks: &[openpgp::packet::PKESK], _: &[openpgp::packet::SKESK],
            sym: Option<openpgp::types::SymmetricAlgorithm>, mut decrypt: D) -> openpgp::Result<Option<openpgp::Fingerprint>>
        where D: FnMut(openpgp::types::SymmetricAlgorithm, &openpgp::crypto::SessionKey) -> bool {
            let p = StandardPolicy::new();
            for ka in self.cert.keys().with_policy(&p, None).secret().for_storage_encryption() {
                let Ok(key) = ka.key().clone().decrypt_secret(&self.pw) else { continue };
                self.unlocked.store(true, Ordering::SeqCst);
                let mut pair = key.into_keypair()?;
                for pk in pkesks {
                    if pk.decrypt(&mut pair, sym).map(|(a, sk)| decrypt(a, &sk)).unwrap_or(false) {
                        return Ok(Some(ka.key().fingerprint()));
                    }
                }
            }
            Ok(None)
        }
    }
    let unlocked = Arc::new(AtomicBool::new(false));
    let h = H { cert: cert.clone(), pw: "hunter2".into(), unlocked: unlocked.clone() };
    let mut d = DecryptorBuilder::from_bytes(&ct).unwrap().with_policy(&p, None, h).unwrap();
    let mut pt = Vec::new();
    std::io::copy(&mut d, &mut pt).unwrap();
    assert_eq!(pt, b"hello");
    assert!(unlocked.load(Ordering::SeqCst));
}

#[test]
fn rusqlite_serialize_deserialize_roundtrip_via_owneddata() {
    use rusqlite::{Connection, DatabaseName};
    let c = Connection::open_in_memory().unwrap();
    c.execute_batch("CREATE TABLE t(x); INSERT INTO t VALUES(42);").unwrap();
    let data = c.serialize(DatabaseName::Main).unwrap();      // rusqlite::serialize::Data (Deref<[u8]>)
    let image: Vec<u8> = data.to_vec();                        // copy out of Shared/Owned
    // rebuild via OwnedData allocated by sqlite3_malloc64
    let owned = unsafe {
        let n = image.len();
        let p = rusqlite::ffi::sqlite3_malloc64(n as u64) as *mut u8;
        assert!(!p.is_null());
        std::ptr::copy_nonoverlapping(image.as_ptr(), p, n);
        rusqlite::serialize::OwnedData::from_raw_nonnull(std::ptr::NonNull::new(p).unwrap(), n)
    };
    let mut c2 = Connection::open_in_memory().unwrap();
    c2.deserialize(DatabaseName::Main, owned, false).unwrap();
    let x: i64 = c2.query_row("SELECT x FROM t", [], |r| r.get(0)).unwrap();
    assert_eq!(x, 42);
}
