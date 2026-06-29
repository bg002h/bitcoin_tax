use crate::{StoreError, SCHEMA_VERSION};

pub fn encode_blob(version: u32, image: &[u8]) -> Vec<u8> {
    let mut o = Vec::with_capacity(4 + image.len());
    o.extend_from_slice(&version.to_be_bytes());
    o.extend_from_slice(image);
    o
}

pub fn decode_blob(blob: &[u8]) -> Result<(u32, &[u8]), StoreError> {
    if blob.len() < 4 {
        return Err(StoreError::Corrupt("blob < 4-byte header".into()));
    }
    Ok((
        u32::from_be_bytes(blob[0..4].try_into().unwrap()),
        &blob[4..],
    ))
}

pub fn migrate(version: u32, image: Vec<u8>) -> Result<Vec<u8>, StoreError> {
    if version == SCHEMA_VERSION {
        Ok(image)
    } else {
        Err(StoreError::UnsupportedSchema(version))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let b = encode_blob(1, b"IMG");
        let (v, i) = decode_blob(&b).unwrap();
        assert_eq!(v, 1);
        assert_eq!(i, b"IMG");
    }

    #[test]
    fn rejects_short() {
        assert!(matches!(
            decode_blob(b"\x00\x00"),
            Err(StoreError::Corrupt(_))
        ));
    }

    #[test]
    fn migrate_identity() {
        assert_eq!(migrate(SCHEMA_VERSION, b"IMG".to_vec()).unwrap(), b"IMG");
    }

    #[test]
    fn migrate_future() {
        assert!(matches!(
            migrate(SCHEMA_VERSION + 1, vec![]),
            Err(StoreError::UnsupportedSchema(_))
        ));
    }
}
