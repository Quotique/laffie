use eyre::{Result, WrapErr};
use serde::{Serialize, de::DeserializeOwned};

const ZSTD_LEVEL: i32 = 3;

/// Encode a value as JSON, then compress with zstd. Used as the on-disk
/// representation for every value stored in the database.
pub fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    let json = serde_json::to_vec(value).wrap_err("json encode")?;
    zstd::encode_all(&json[..], ZSTD_LEVEL).wrap_err("zstd encode")
}

/// Inverse of [`encode`].
pub fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    let json = zstd::decode_all(bytes).wrap_err("zstd decode")?;
    serde_json::from_slice(&json).wrap_err("json decode")
}

#[cfg(test)]
mod tests {
    use serde_derive::{Deserialize, Serialize};

    use super::*;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Sample {
        a: u32,
        b: String,
        c: Vec<i64>,
    }

    #[test]
    fn roundtrip() {
        let v = Sample {
            a: 42,
            b: "hello".into(),
            c: vec![-1, 0, 1],
        };
        let bytes = encode(&v).unwrap();
        let back: Sample = decode(&bytes).unwrap();
        assert_eq!(v, back);
    }

    #[test]
    fn corrupted_zstd_fails_cleanly() {
        let err = decode::<Sample>(&[0xFF, 0xFF, 0xFF, 0xFF]).unwrap_err();
        assert!(format!("{err:?}").contains("zstd decode"));
    }

    #[test]
    fn corrupted_json_fails_cleanly() {
        let bytes = zstd::encode_all(&b"not valid json"[..], ZSTD_LEVEL).unwrap();
        let err = decode::<Sample>(&bytes).unwrap_err();
        assert!(format!("{err:?}").contains("json decode"));
    }
}
