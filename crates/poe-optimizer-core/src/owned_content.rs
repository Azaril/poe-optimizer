//! Bounded, domain-separated identities of canonical owned snapshots.
//!
//! A deserialized digest is a claim, not provenance or private-plan authority.
//! Callers must validate and canonicalize their semantic value before hashing it.
//! Exact snapshot bindings include revision/watermark; future numerical-plan keys
//! may deliberately exclude unrelated stock and presentation state.
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use sha2::{Digest, Sha256};
use std::{fmt, io, str::FromStr};

pub const MAX_OWNED_CONTENT_BYTES: usize = 64 * 1024 * 1024;
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct OwnedContentDigest([u8; 32]);
impl OwnedContentDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}
impl fmt::Display for OwnedContentDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}
impl FromStr for OwnedContentDigest {
    type Err = ContentDigestError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.len() != 64 {
            return Err(ContentDigestError::InvalidEncoding);
        }
        fn digit(byte: u8) -> Result<u8, ContentDigestError> {
            match byte {
                b'0'..=b'9' => Ok(byte - b'0'),
                b'a'..=b'f' => Ok(byte - b'a' + 10),
                _ => Err(ContentDigestError::InvalidEncoding),
            }
        }
        let mut bytes = [0; 32];
        for (target, pair) in bytes.iter_mut().zip(value.as_bytes().as_chunks::<2>().0) {
            *target = digit(pair[0])? << 4 | digit(pair[1])?;
        }
        Ok(Self(bytes))
    }
}
impl Serialize for OwnedContentDigest {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}
impl<'de> Deserialize<'de> for OwnedContentDigest {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl de::Visitor<'_> for Visitor {
            type Value = OwnedContentDigest;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("exactly 64 lowercase hexadecimal digits")
            }
            fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                value.parse().map_err(E::custom)
            }
        }
        deserializer.deserialize_str(Visitor)
    }
}

#[derive(Debug)]
pub enum ContentDigestError {
    InvalidEncoding,
    InvalidDomain,
    InvalidLimit,
    TooLarge { maximum: usize },
    Json(serde_json::Error),
}
impl fmt::Display for ContentDigestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidEncoding => {
                f.write_str("content digest requires exactly 64 lowercase hexadecimal digits")
            }
            Self::InvalidDomain => f.write_str("content domain must be a bounded owned symbol"),
            Self::InvalidLimit => {
                f.write_str("content byte limit must be positive and within the hard ceiling")
            }
            Self::TooLarge { maximum } => write!(f, "canonical content exceeds {maximum} bytes"),
            Self::Json(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for ContentDigestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        if let Self::Json(e) = self {
            Some(e)
        } else {
            None
        }
    }
}

/// Hash a canonical serializable value without buffering or cloning its contents.
/// The bounded domain is length-delimited and separated from JSON by a fixed prefix.
pub fn digest_owned<T: Serialize>(
    domain: &'static str,
    value: &T,
    max_bytes: usize,
) -> Result<OwnedContentDigest, ContentDigestError> {
    crate::owned_definitions::OwnedDefinitionKey::new(domain)
        .map_err(|_| ContentDigestError::InvalidDomain)?;
    if max_bytes == 0 || max_bytes > MAX_OWNED_CONTENT_BYTES {
        return Err(ContentDigestError::InvalidLimit);
    }
    let mut hash = Sha256::new();
    hash.update(b"poe-optimizer-owned-content-v1\0");
    hash.update((domain.len() as u64).to_le_bytes());
    hash.update(domain.as_bytes());
    let mut writer = DigestWriter {
        hash,
        written: 0,
        maximum: max_bytes,
        exceeded: false,
    };
    let result = serde_json::to_writer(&mut writer, value);
    if writer.exceeded {
        return Err(ContentDigestError::TooLarge { maximum: max_bytes });
    }
    result.map_err(ContentDigestError::Json)?;
    Ok(OwnedContentDigest(writer.hash.finalize().into()))
}
struct DigestWriter {
    hash: Sha256,
    written: usize,
    maximum: usize,
    exceeded: bool,
}
impl io::Write for DigestWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.maximum - self.written {
            self.exceeded = true;
            return Err(io::Error::other("canonical content byte limit exceeded"));
        }
        self.hash.update(bytes);
        self.written += bytes.len();
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
