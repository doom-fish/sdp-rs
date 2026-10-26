//! RTP header extension mappings (RFC 8285): the `a=extmap:` attribute.
//!
//! ```text
//! a=extmap:1 urn:ietf:params:rtp-hdrext:sdes:mid
//! a=extmap:3 http://www.ietf.org/id/draft-holmer-rmcat-transport-wide-cc-extensions-01
//! ```

use crate::error::{Result, SdpError};
use std::fmt;

/// URN for the SDES MID header extension (RFC 9143).
pub const URI_MID: &str = "urn:ietf:params:rtp-hdrext:sdes:mid";

/// URI for the Transport-Wide Congestion Control (TWCC) header extension.
pub const URI_TWCC: &str =
    "http://www.ietf.org/id/draft-holmer-rmcat-transport-wide-cc-extensions-01";

/// URN for the absolute send time header extension.
pub const URI_ABS_SEND_TIME: &str =
    "http://www.webrtc.org/experiments/rtp-hdrext/abs-send-time";

/// A parsed `a=extmap:` value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Extmap {
    /// The local identifier (1..=14 for one-byte headers, up to 255 otherwise).
    pub id: u16,
    /// Optional direction qualifier (`sendonly`, `recvonly`, ...).
    pub direction: Option<String>,
    /// The extension URI.
    pub uri: String,
    /// Optional trailing extension attributes, preserved verbatim.
    pub params: Option<String>,
}

impl Extmap {
    /// Construct an extmap mapping `id` to `uri`.
    #[must_use]
    pub fn new(id: u16, uri: impl Into<String>) -> Self {
        Self {
            id,
            direction: None,
            uri: uri.into(),
            params: None,
        }
    }

    /// The MID header extension mapped to `id`.
    #[must_use]
    pub fn mid(id: u16) -> Self {
        Self::new(id, URI_MID)
    }

    /// The TWCC header extension mapped to `id`.
    #[must_use]
    pub fn twcc(id: u16) -> Self {
        Self::new(id, URI_TWCC)
    }

    /// Parse the value portion of an `a=extmap:` line.
    ///
    /// # Errors
    /// Returns an error if the id is missing or non-numeric, or if the URI is
    /// absent.
    pub fn parse(value: &str) -> Result<Self> {
        let mut parts = value.splitn(2, ' ');
        let id_field = parts
            .next()
            .ok_or_else(|| SdpError::invalid("extmap", value))?;
        let rest = parts
            .next()
            .ok_or_else(|| SdpError::invalid("extmap", value))?;

        let (id_str, direction) = id_field.split_once('/').map_or_else(
            || (id_field, None),
            |(id, dir)| (id, Some(dir.to_owned())),
        );
        let id = id_str
            .parse()
            .map_err(|_| SdpError::number("extmap.id", id_str))?;

        let mut uri_parts = rest.splitn(2, ' ');
        let uri = uri_parts
            .next()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| SdpError::invalid("extmap", value))?
            .to_owned();
        let params = uri_parts.next().map(ToOwned::to_owned);

        Ok(Self {
            id,
            direction,
            uri,
            params,
        })
    }
}

impl fmt::Display for Extmap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.id)?;
        if let Some(dir) = &self.direction {
            write!(f, "/{dir}")?;
        }
        write!(f, " {}", self.uri)?;
        if let Some(params) = &self.params {
            write!(f, " {params}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mid_round_trip() {
        let value = "1 urn:ietf:params:rtp-hdrext:sdes:mid";
        let e = Extmap::parse(value).expect("parse");
        assert_eq!(e.id, 1);
        assert_eq!(e.uri, URI_MID);
        assert_eq!(e.to_string(), value);
    }

    #[test]
    fn direction_and_params_round_trip() {
        let value = "5/sendonly urn:3gpp:video-orientation extra";
        let e = Extmap::parse(value).expect("parse");
        assert_eq!(e.direction.as_deref(), Some("sendonly"));
        assert_eq!(e.params.as_deref(), Some("extra"));
        assert_eq!(e.to_string(), value);
    }

    #[test]
    fn rejects_bad_id() {
        assert!(Extmap::parse("x uri").is_err());
        assert!(Extmap::parse("1").is_err());
    }
}
