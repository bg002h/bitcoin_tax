use crate::StoreError;
use openpgp::cert::CertBuilder;
use openpgp::parse::{
    stream::{DecryptionHelper, DecryptorBuilder, MessageStructure, VerificationHelper},
    Parse,
};
use openpgp::policy::StandardPolicy;
use openpgp::serialize::stream::{Encryptor2, LiteralWriter, Message};
use sequoia_openpgp as openpgp;
use std::io::Write;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use zeroize::Zeroize;

pub struct Passphrase(String);
impl Passphrase {
    pub fn new(s: String) -> Self {
        Self(s)
    }
    fn pw(&self) -> openpgp::crypto::Password {
        self.0.as_str().into()
    }
}
impl Drop for Passphrase {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

pub fn generate_cert(pp: &Passphrase) -> Result<openpgp::Cert, StoreError> {
    // S2K: Sequoia 1.21 exposes no Argon2 and no public S2K-work-factor setter on
    // set_password/encrypt_secret; set_password uses the library default, which the Task-0
    // spike asserts is an S2K::Iterated (iterated-salted SHA-256) — the strongest available
    // via the supported API (spec §8 "else high-work-factor iterated-salted" fallback / R3).
    let (cert, _rev) = CertBuilder::new()
        .add_userid("vault@btctax.local")
        .add_storage_encryption_subkey()
        .set_password(Some(pp.pw()))
        .generate()
        .map_err(StoreError::Crypto)?;
    Ok(cert)
}

pub fn encrypt_to(cert: &openpgp::Cert, plaintext: &[u8]) -> Result<Vec<u8>, StoreError> {
    let p = StandardPolicy::new();
    let recips = cert
        .keys()
        .with_policy(&p, None)
        .supported()
        .for_storage_encryption()
        .map(|ka| ka.key())
        .collect::<Vec<_>>();
    if recips.is_empty() {
        return Err(StoreError::Corrupt("no encryption subkey".into()));
    }
    let mut ct = Vec::new();
    let m = Message::new(&mut ct);
    let m = Encryptor2::for_recipients(m, recips)
        .build()
        .map_err(StoreError::Crypto)?;
    let mut w = LiteralWriter::new(m).build().map_err(StoreError::Crypto)?;
    w.write_all(plaintext)?;
    w.finalize().map_err(StoreError::Crypto)?;
    Ok(ct)
}

pub fn decrypt_with(
    cert: &openpgp::Cert,
    pp: &Passphrase,
    ct: &[u8],
) -> Result<Vec<u8>, StoreError> {
    struct H {
        cert: openpgp::Cert,
        pw: openpgp::crypto::Password,
        unlocked: Arc<AtomicBool>,
    }
    impl VerificationHelper for H {
        fn get_certs(&mut self, _: &[openpgp::KeyHandle]) -> openpgp::Result<Vec<openpgp::Cert>> {
            Ok(vec![])
        }
        fn check(&mut self, _: MessageStructure) -> openpgp::Result<()> {
            Ok(())
        }
    }
    impl DecryptionHelper for H {
        fn decrypt<D>(
            &mut self,
            pkesks: &[openpgp::packet::PKESK],
            _: &[openpgp::packet::SKESK],
            sym: Option<openpgp::types::SymmetricAlgorithm>,
            mut decrypt: D,
        ) -> openpgp::Result<Option<openpgp::Fingerprint>>
        where
            D: FnMut(openpgp::types::SymmetricAlgorithm, &openpgp::crypto::SessionKey) -> bool,
        {
            let p = StandardPolicy::new();
            for ka in self
                .cert
                .keys()
                .with_policy(&p, None)
                .secret()
                .for_storage_encryption()
            {
                let Ok(key) = ka.key().clone().decrypt_secret(&self.pw) else {
                    continue;
                };
                self.unlocked.store(true, Ordering::SeqCst);
                let mut pair = key.into_keypair()?;
                for pk in pkesks {
                    if pk
                        .decrypt(&mut pair, sym)
                        .map(|(a, sk)| decrypt(a, &sk))
                        .unwrap_or(false)
                    {
                        return Ok(Some(ka.key().fingerprint()));
                    }
                }
            }
            Ok(None)
        }
    }
    let p = StandardPolicy::new();
    let unlocked = Arc::new(AtomicBool::new(false));
    let h = H {
        cert: cert.clone(),
        pw: pp.pw(),
        unlocked: unlocked.clone(),
    };
    let res = DecryptorBuilder::from_bytes(ct)
        .map_err(StoreError::Crypto)?
        .with_policy(&p, None, h);
    let mut dec = match res {
        Ok(d) => d,
        Err(e) => {
            return Err(if unlocked.load(Ordering::SeqCst) {
                StoreError::Crypto(e)
            } else {
                StoreError::WrongPassphrase
            })
        }
    };
    let mut pt = Vec::new();
    std::io::copy(&mut dec, &mut pt)?;
    Ok(pt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let pp = Passphrase::new("correct horse".into());
        let c = generate_cert(&pp).unwrap();
        let ct = encrypt_to(&c, b"img").unwrap();
        assert_ne!(ct, b"img");
        assert_eq!(decrypt_with(&c, &pp, &ct).unwrap(), b"img");
    }

    #[test]
    fn wrong_pass() {
        let c = generate_cert(&Passphrase::new("right".into())).unwrap();
        let ct = encrypt_to(&c, b"x").unwrap();
        assert!(matches!(
            decrypt_with(&c, &Passphrase::new("wrong".into()), &ct),
            Err(StoreError::WrongPassphrase)
        ));
    }
}
