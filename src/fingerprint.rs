//! DTLS-SRTP certificate fingerprints (RFC 8122 / RFC 8841).
//!
//! A fingerprint line carries the hash algorithm and the colon-separated
//! upper-case hex digest of the DTLS certificate, for example:
//!
//! ```text
//! a=fingerprint:sha-256 12:AB:34:CD:...:EF
//! ```

use crate::error::{Result, SdpError};
use std::fmt;

/// A DTLS certificate fingerprint: a hash algorithm plus a digest.
#[derive(Debug, Clone)]
pub struct Fingerprint {
    /// Hash function token (for example `sha-256`), lower-cased by
    /// [`Fingerprint::new`] and [`Fingerprint::parse`] because hash function
    /// names are case-insensitive. Algorithms this crate does not specifically
    /// know about are kept so they still round-trip.
    pub algorithm: String,
    /// The raw digest bytes.
    pub digest: Vec<u8>,
}

impl PartialEq for Fingerprint {
    fn eq(&self, other: &Self) -> bool {
        self.algorithm.eq_ignore_ascii_case(&other.algorithm) && self.digest == other.digest
    }
}

impl Eq for Fingerprint {}

impl Fingerprint {
    /// Construct a fingerprint from an algorithm token and raw digest bytes.
    #[must_use]
    pub fn new(algorithm: impl Into<String>, digest: Vec<u8>) -> Self {
        let mut algorithm = algorithm.into();
        algorithm.make_ascii_lowercase();
        Self { algorithm, digest }
    }

    /// Parse the value portion of an `a=fingerprint:` line (the text after the
    /// colon), for example `sha-256 12:AB:...:EF`.
    ///
    /// # Errors
    /// Returns an error if the algorithm or the colon-separated hex digest is
    /// missing or malformed, or if the digest length does not match a known
    /// hash function (for example 32 bytes for `sha-256`).
    pub fn parse(value: &str) -> Result<Self> {
        let mut parts = value.splitn(2, ' ');
        let algorithm = parts
            .next()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| SdpError::invalid("fingerprint", value))?;
        let hex = parts
            .next()
            .ok_or_else(|| SdpError::invalid("fingerprint", value))?;
        let digest = parse_hex_pairs(hex)
            .ok_or_else(|| SdpError::invalid("fingerprint", value))?;
        let fingerprint = Self::new(algorithm, digest);
        if digest_len(&fingerprint.algorithm).is_some_and(|len| len != fingerprint.digest.len()) {
            return Err(SdpError::invalid("fingerprint", value));
        }
        Ok(fingerprint)
    }

    /// Render the digest as upper-case colon-separated hex.
    #[must_use]
    pub fn digest_hex(&self) -> String {
        let mut out = String::with_capacity(self.digest.len() * 3);
        for (i, b) in self.digest.iter().enumerate() {
            if i > 0 {
                out.push(':');
            }
            out.push(nibble(b >> 4));
            out.push(nibble(b & 0x0f));
        }
        out
    }
}

impl fmt::Display for Fingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.algorithm, self.digest_hex())
    }
}

fn digest_len(algorithm: &str) -> Option<usize> {
    match algorithm {
        "md2" | "md5" => Some(16),
        "sha-1" => Some(20),
        "sha-224" => Some(28),
        "sha-256" => Some(32),
        "sha-384" => Some(48),
        "sha-512" => Some(64),
        _ => None,
    }
}

const fn nibble(v: u8) -> char {
    match v {
        0..=9 => (b'0' + v) as char,
        _ => (b'A' + (v - 10)) as char,
    }
}

const fn hex_val(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

fn parse_hex_pairs(s: &str) -> Option<Vec<u8>> {
    if s.is_empty() {
        return None;
    }
    let mut out = Vec::new();
    for pair in s.split(':') {
        let bytes = pair.as_bytes();
        if bytes.len() != 2 {
            return None;
        }
        let hi = hex_val(bytes[0])?;
        let lo = hex_val(bytes[1])?;
        out.push((hi << 4) | lo);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHA256: &str = "12:AB:34:CD:EF:00:11:22:33:44:55:66:77:88:99:AA:BB:CC:DD:EE:FF:01:23:45:67:89:AB:CD:EF:10:32:54";

    #[test]
    fn round_trip() {
        let value = format!("sha-256 {SHA256}");
        let fp = Fingerprint::parse(&value).expect("parse");
        assert_eq!(fp.algorithm, "sha-256");
        assert_eq!(fp.digest.len(), 32);
        assert_eq!(&fp.digest[..6], &[0x12, 0xAB, 0x34, 0xCD, 0xEF, 0x00]);
        assert_eq!(fp.to_string(), value);
    }

    #[test]
    fn digest_length_must_match_the_algorithm() {
        assert!(Fingerprint::parse("sha-256 AB").is_err());
        assert!(Fingerprint::parse(&format!("sha-1 {SHA256}")).is_err());
        assert!(Fingerprint::parse(&format!("sha-512 {SHA256}")).is_err());
        assert!(Fingerprint::parse(
            "sha-1 00:01:02:03:04:05:06:07:08:09:0A:0B:0C:0D:0E:0F:10:11:12:13"
        )
        .is_ok());
        let unknown = Fingerprint::parse("x-future-hash AB:CD").expect("unknown algorithm kept");
        assert_eq!(unknown.digest, vec![0xAB, 0xCD]);
    }

    #[test]
    fn algorithm_is_case_insensitive() {
        let upper = Fingerprint::parse(&format!("SHA-256 {SHA256}")).expect("upper-case token");
        assert_eq!(upper.algorithm, "sha-256");
        let lower = Fingerprint::parse(&format!("sha-256 {SHA256}")).unwrap();
        assert_eq!(upper, lower);
        let literal = Fingerprint {
            algorithm: "Sha-256".into(),
            digest: lower.digest.clone(),
        };
        assert_eq!(literal, lower);
        assert_ne!(Fingerprint::new("sha-1", lower.digest.clone()), lower);
        assert!(Fingerprint::parse("SHA-256 AB").is_err());
    }

    #[test]
    fn rejects_bad_pair() {
        assert!(Fingerprint::parse("sha-256 1:2:3").is_err());
        assert!(Fingerprint::parse("sha-256").is_err());
        assert!(Fingerprint::parse("").is_err());
    }
}
