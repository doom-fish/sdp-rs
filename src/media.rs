//! Media descriptions (`m=` sections) and their owned lines.

use crate::attributes::{Attribute, Direction, Fmtp, RtpMap, SetupRole, REDACTED};
use crate::error::{Result, SdpError};
use crate::fingerprint::Fingerprint;
use crate::ice::Candidate;
use crate::session::{Bandwidth, Connection};
use std::fmt;

/// The media type of an `m=` section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaKind {
    /// `audio`
    Audio,
    /// `video`
    Video,
    /// `application` (used by WebRTC data channels / SCTP).
    Application,
    /// `text`
    Text,
    /// `message`
    Message,
    /// Any other media token.
    Other(String),
}

impl MediaKind {
    fn from_token(token: &str) -> Self {
        match token {
            "audio" => Self::Audio,
            "video" => Self::Video,
            "application" => Self::Application,
            "text" => Self::Text,
            "message" => Self::Message,
            other => Self::Other(other.to_owned()),
        }
    }

    /// The raw media token.
    #[must_use]
    pub fn token(&self) -> &str {
        match self {
            Self::Audio => "audio",
            Self::Video => "video",
            Self::Application => "application",
            Self::Text => "text",
            Self::Message => "message",
            Self::Other(s) => s,
        }
    }
}

impl fmt::Display for MediaKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.token())
    }
}

/// A single media description (`m=` line plus its owned sub-lines).
#[derive(Clone, PartialEq, Eq)]
pub struct MediaDescription {
    /// Media type.
    pub kind: MediaKind,
    /// Transport port.
    pub port: u16,
    /// Optional port count (`m=audio 9/2 ...`).
    pub port_count: Option<u16>,
    /// Transport protocol (for example `UDP/TLS/RTP/SAVPF`).
    pub proto: String,
    /// Format descriptors (payload types, or `webrtc-datachannel`).
    pub formats: Vec<String>,
    /// Optional media title (`i=`).
    pub title: Option<String>,
    /// Optional media-level connection (`c=`).
    pub connection: Option<Connection>,
    /// Media-level bandwidth lines (`b=`).
    pub bandwidths: Vec<Bandwidth>,
    /// Media-level encryption key (`k=`). Deprecated by RFC 8866 but preserved.
    pub key: Option<String>,
    /// Media-level attributes (`a=`).
    pub attributes: Vec<Attribute>,
}

impl fmt::Debug for MediaDescription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MediaDescription")
            .field("kind", &self.kind)
            .field("port", &self.port)
            .field("port_count", &self.port_count)
            .field("proto", &self.proto)
            .field("formats", &self.formats)
            .field("title", &self.title)
            .field("connection", &self.connection)
            .field("bandwidths", &self.bandwidths)
            .field("key", &self.key.as_ref().map(|_| REDACTED))
            .field("attributes", &self.attributes)
            .finish()
    }
}

impl MediaDescription {
    /// Construct a media description with no optional lines.
    #[must_use]
    pub fn new(kind: MediaKind, port: u16, proto: impl Into<String>) -> Self {
        Self {
            kind,
            port,
            port_count: None,
            proto: proto.into(),
            formats: Vec::new(),
            title: None,
            connection: None,
            bandwidths: Vec::new(),
            key: None,
            attributes: Vec::new(),
        }
    }

    /// Parse an `m=` line value (the text after `m=`).
    ///
    /// # Errors
    /// Returns an error if the line has fewer than three space-separated
    /// fields or the port is not numeric.
    pub fn parse_m_line(value: &str) -> Result<Self> {
        let mut parts = value.split(' ');
        let media = parts
            .next()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| SdpError::invalid("m", value))?;
        let port_field = parts
            .next()
            .ok_or_else(|| SdpError::invalid("m", value))?;
        let proto = parts
            .next()
            .ok_or_else(|| SdpError::invalid("m", value))?
            .to_owned();

        let (port_str, port_count) = port_field.split_once('/').map_or_else(
            || (port_field, None),
            |(p, c)| (p, Some(c)),
        );
        let port = port_str
            .parse()
            .map_err(|_| SdpError::number("m.port", port_str))?;
        let port_count = match port_count {
            Some(c) => Some(
                c.parse()
                    .map_err(|_| SdpError::number("m.port-count", c))?,
            ),
            None => None,
        };
        let formats: Vec<String> = parts.map(ToOwned::to_owned).collect();
        // RFC 8866 §5.14: the m= line requires at least one format token.
        if formats.is_empty() {
            return Err(SdpError::invalid("m", value));
        }

        Ok(Self {
            kind: MediaKind::from_token(media),
            port,
            port_count,
            proto,
            formats,
            title: None,
            connection: None,
            bandwidths: Vec::new(),
            key: None,
            attributes: Vec::new(),
        })
    }

    /// Render just the `m=` header line value (without the trailing CRLF or
    /// the `m=` prefix).
    #[must_use]
    pub fn m_line(&self) -> String {
        let mut out = format!("{} {}", self.kind, self.port);
        if let Some(count) = self.port_count {
            out.push('/');
            out.push_str(&count.to_string());
        }
        out.push(' ');
        out.push_str(&self.proto);
        for fmt_tok in &self.formats {
            out.push(' ');
            out.push_str(fmt_tok);
        }
        out
    }

    /// Returns `true` if this section is rejected: port 0 without
    /// `a=bundle-only` (RFC 8843 §6).
    #[must_use]
    pub fn is_rejected(&self) -> bool {
        self.port == 0 && !self.is_bundle_only()
    }

    #[allow(missing_docs)]
    #[must_use]
    pub fn is_bundle_only(&self) -> bool {
        self.attributes
            .iter()
            .any(|a| matches!(a, Attribute::BundleOnly))
    }

    /// The `mid` of this section, if present.
    #[must_use]
    pub fn mid(&self) -> Option<&str> {
        self.attributes.iter().find_map(|a| match a {
            Attribute::Mid(m) => Some(m.as_str()),
            _ => None,
        })
    }

    /// The declared media direction, if any.
    #[must_use]
    pub fn direction(&self) -> Option<Direction> {
        self.attributes.iter().find_map(|a| match a {
            Attribute::Direction(d) => Some(*d),
            _ => None,
        })
    }

    /// The DTLS setup role, if present.
    #[must_use]
    pub fn setup(&self) -> Option<SetupRole> {
        self.attributes.iter().find_map(|a| match a {
            Attribute::Setup(s) => Some(*s),
            _ => None,
        })
    }

    /// The DTLS fingerprint declared at media level, if present.
    #[must_use]
    pub fn fingerprint(&self) -> Option<&Fingerprint> {
        self.attributes.iter().find_map(|a| match a {
            Attribute::Fingerprint(fp) => Some(fp),
            _ => None,
        })
    }

    #[allow(missing_docs)]
    #[must_use]
    pub fn fingerprints(&self) -> Vec<&Fingerprint> {
        self.attributes
            .iter()
            .filter_map(|a| match a {
                Attribute::Fingerprint(fp) => Some(fp),
                _ => None,
            })
            .collect()
    }

    #[allow(missing_docs)]
    #[must_use]
    pub fn ice_ufrag(&self) -> Option<&str> {
        self.attributes.iter().find_map(|a| match a {
            Attribute::IceUfrag(u) => Some(u.as_str()),
            _ => None,
        })
    }

    #[allow(missing_docs)]
    #[must_use]
    pub fn ice_pwd(&self) -> Option<&str> {
        self.attributes.iter().find_map(|a| match a {
            Attribute::IcePwd(p) => Some(p.as_str()),
            _ => None,
        })
    }

    #[allow(missing_docs)]
    #[must_use]
    pub fn candidates(&self) -> Vec<&Candidate> {
        self.attributes
            .iter()
            .filter_map(|a| match a {
                Attribute::Candidate(c) => Some(c),
                _ => None,
            })
            .collect()
    }

    /// Returns `true` if `a=rtcp-mux` is present.
    #[must_use]
    pub fn has_rtcp_mux(&self) -> bool {
        self.attributes
            .iter()
            .any(|a| matches!(a, Attribute::RtcpMux))
    }

    /// All `a=rtpmap:` entries in order.
    #[must_use]
    pub fn rtpmaps(&self) -> Vec<&RtpMap> {
        self.attributes
            .iter()
            .filter_map(|a| match a {
                Attribute::RtpMap(m) => Some(m),
                _ => None,
            })
            .collect()
    }

    /// The `a=fmtp:` entry for a given payload type, if present.
    #[must_use]
    pub fn fmtp_for(&self, payload: u8) -> Option<&Fmtp> {
        let pt = payload.to_string();
        self.attributes.iter().find_map(|a| match a {
            Attribute::Fmtp(f) if f.format == pt => Some(f),
            _ => None,
        })
    }

    /// Push an attribute onto this section.
    pub fn push_attribute(&mut self, attr: Attribute) {
        self.attributes.push(attr);
    }
}
