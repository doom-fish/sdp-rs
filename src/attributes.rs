//! Typed SDP attribute model (`a=` lines).
//!
//! Recognised WebRTC-profile attributes are parsed into dedicated variants so
//! callers can inspect them ergonomically; everything else is preserved
//! verbatim in [`Attribute::Other`] so that any conforming document
//! round-trips losslessly.

use crate::bundle::Group;
use crate::error::{Result, SdpError};
use crate::extmap::Extmap;
use crate::fingerprint::Fingerprint;
use crate::ice::Candidate;
use std::fmt;

/// Media direction (RFC 8866 §6.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// `a=sendrecv`
    SendRecv,
    /// `a=sendonly`
    SendOnly,
    /// `a=recvonly`
    RecvOnly,
    /// `a=inactive`
    Inactive,
}

impl Direction {
    /// The reciprocal direction to offer back in an answer.
    #[must_use]
    pub const fn reverse(self) -> Self {
        match self {
            Self::SendRecv => Self::SendRecv,
            Self::SendOnly => Self::RecvOnly,
            Self::RecvOnly => Self::SendOnly,
            Self::Inactive => Self::Inactive,
        }
    }

    const fn token(self) -> &'static str {
        match self {
            Self::SendRecv => "sendrecv",
            Self::SendOnly => "sendonly",
            Self::RecvOnly => "recvonly",
            Self::Inactive => "inactive",
        }
    }
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.token())
    }
}

/// DTLS setup role (RFC 8842 / RFC 4145).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupRole {
    /// `a=setup:actpass` — willing to be client or server (offer default).
    ActPass,
    /// `a=setup:active` — DTLS client (sends `ClientHello`).
    Active,
    /// `a=setup:passive` — DTLS server.
    Passive,
    /// `a=setup:holdconn` — hold the connection.
    HoldConn,
}

impl SetupRole {
    /// The role an answerer should use given the offerer's role. An offerer
    /// that says `actpass` lets the answerer pick; we pick `active`.
    #[must_use]
    pub const fn answer_for(offer: Self) -> Self {
        match offer {
            Self::ActPass | Self::Passive => Self::Active,
            Self::Active => Self::Passive,
            Self::HoldConn => Self::HoldConn,
        }
    }

    const fn token(self) -> &'static str {
        match self {
            Self::ActPass => "actpass",
            Self::Active => "active",
            Self::Passive => "passive",
            Self::HoldConn => "holdconn",
        }
    }

    fn parse(value: &str) -> Result<Self> {
        match value {
            "actpass" => Ok(Self::ActPass),
            "active" => Ok(Self::Active),
            "passive" => Ok(Self::Passive),
            "holdconn" => Ok(Self::HoldConn),
            other => Err(SdpError::invalid("setup", other)),
        }
    }
}

impl fmt::Display for SetupRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.token())
    }
}

/// `a=rtpmap:<pt> <encoding>/<clock>[/<channels>]`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtpMap {
    /// RTP payload type.
    pub payload: u8,
    /// Encoding name (for example `opus`, `VP8`, `H264`).
    pub encoding: String,
    /// RTP clock rate in Hz.
    pub clock_rate: u32,
    /// Optional channel count (for example `2` for stereo Opus).
    pub channels: Option<u32>,
}

impl RtpMap {
    fn parse(value: &str) -> Result<Self> {
        let (pt, rest) = value
            .split_once(' ')
            .ok_or_else(|| SdpError::invalid("rtpmap", value))?;
        let payload = pt
            .parse()
            .map_err(|_| SdpError::number("rtpmap.payload", pt))?;
        let mut fields = rest.split('/');
        let encoding = fields
            .next()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| SdpError::invalid("rtpmap", value))?
            .to_owned();
        let clock = fields
            .next()
            .ok_or_else(|| SdpError::invalid("rtpmap", value))?;
        let clock_rate = clock
            .parse()
            .map_err(|_| SdpError::number("rtpmap.clock", clock))?;
        let channels = match fields.next() {
            Some(c) => Some(
                c.parse()
                    .map_err(|_| SdpError::number("rtpmap.channels", c))?,
            ),
            None => None,
        };
        Ok(Self {
            payload,
            encoding,
            clock_rate,
            channels,
        })
    }
}

impl fmt::Display for RtpMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}/{}", self.payload, self.encoding, self.clock_rate)?;
        if let Some(ch) = self.channels {
            write!(f, "/{ch}")?;
        }
        Ok(())
    }
}

/// `a=fmtp:<pt> <params>` — the parameter string is preserved verbatim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fmtp {
    /// Format (payload type, or `*`) the parameters apply to.
    pub format: String,
    /// Raw parameter string.
    pub params: String,
}

impl Fmtp {
    fn parse(value: &str) -> Result<Self> {
        let (pt, params) = value
            .split_once(' ')
            .ok_or_else(|| SdpError::invalid("fmtp", value))?;
        Ok(Self {
            format: pt.to_owned(),
            params: params.to_owned(),
        })
    }
}

impl fmt::Display for Fmtp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.format, self.params)
    }
}

/// `a=rtcp-fb:<pt> <type> [param]`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtcpFb {
    /// Format (payload type, or `*`) the feedback applies to.
    pub format: String,
    /// Feedback type and any parameters, preserved verbatim.
    pub feedback: String,
}

impl RtcpFb {
    fn parse(value: &str) -> Result<Self> {
        let (pt, feedback) = value
            .split_once(' ')
            .ok_or_else(|| SdpError::invalid("rtcp-fb", value))?;
        Ok(Self {
            format: pt.to_owned(),
            feedback: feedback.to_owned(),
        })
    }
}

impl fmt::Display for RtcpFb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.format, self.feedback)
    }
}

/// `a=ssrc:<id> <attribute>[:<value>]` (RFC 5576).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ssrc {
    /// Synchronization source identifier.
    pub id: u32,
    /// Attribute name (for example `cname`, `msid`).
    pub attribute: String,
    /// Optional attribute value.
    pub value: Option<String>,
}

impl Ssrc {
    fn parse(value: &str) -> Result<Self> {
        let (id_str, rest) = value
            .split_once(' ')
            .ok_or_else(|| SdpError::invalid("ssrc", value))?;
        let id = id_str
            .parse()
            .map_err(|_| SdpError::number("ssrc.id", id_str))?;
        let (attribute, attr_value) = rest
            .split_once(':')
            .map_or((rest, None), |(a, v)| (a, Some(v.to_owned())));
        Ok(Self {
            id,
            attribute: attribute.to_owned(),
            value: attr_value,
        })
    }
}

impl fmt::Display for Ssrc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.id, self.attribute)?;
        if let Some(v) = &self.value {
            write!(f, ":{v}")?;
        }
        Ok(())
    }
}

/// `a=ssrc-group:<semantics> <ssrc>+` (RFC 5576).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SsrcGroup {
    /// Grouping semantics (for example `FID`, `SIM`).
    pub semantics: String,
    /// The grouped SSRC identifiers.
    pub ssrcs: Vec<u32>,
}

impl SsrcGroup {
    fn parse(value: &str) -> Result<Self> {
        let mut parts = value.split_whitespace();
        let semantics = parts
            .next()
            .ok_or_else(|| SdpError::invalid("ssrc-group", value))?
            .to_owned();
        let mut ssrcs = Vec::new();
        for tok in parts {
            ssrcs.push(
                tok.parse()
                    .map_err(|_| SdpError::number("ssrc-group.ssrc", tok))?,
            );
        }
        Ok(Self { semantics, ssrcs })
    }
}

impl fmt::Display for SsrcGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.semantics)?;
        for s in &self.ssrcs {
            write!(f, " {s}")?;
        }
        Ok(())
    }
}

/// A typed SDP attribute. Unrecognised attributes are kept verbatim in
/// [`Attribute::Other`].
#[derive(Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Attribute {
    /// `a=rtcp-mux` (RFC 5761).
    RtcpMux,
    /// `a=rtcp-mux-only` (RFC 8858).
    RtcpMuxOnly,
    /// `a=rtcp-rsize` (reduced-size RTCP, RFC 5506).
    RtcpRsize,
    /// `a=end-of-candidates` (RFC 8838).
    EndOfCandidates,
    #[allow(missing_docs)]
    BundleOnly,
    /// `a=group:<semantics> <mid>+`.
    Group(Group),
    /// `a=mid:<id>`.
    Mid(String),
    /// `a=msid:<value>`.
    Msid(String),
    /// `a=ice-ufrag:<value>`.
    IceUfrag(String),
    /// `a=ice-pwd:<value>`.
    IcePwd(String),
    /// `a=ice-options:<value>` (for example `trickle`).
    IceOptions(String),
    /// `a=candidate:<...>`.
    Candidate(Candidate),
    /// `a=fingerprint:<algorithm> <digest>`.
    Fingerprint(Fingerprint),
    /// `a=setup:<role>`.
    Setup(SetupRole),
    /// One of the four direction attributes.
    Direction(Direction),
    /// `a=rtpmap:<...>`.
    RtpMap(RtpMap),
    /// `a=fmtp:<...>`.
    Fmtp(Fmtp),
    /// `a=rtcp-fb:<...>`.
    RtcpFb(RtcpFb),
    /// `a=ssrc:<...>`.
    Ssrc(Ssrc),
    /// `a=ssrc-group:<...>`.
    SsrcGroup(SsrcGroup),
    /// `a=extmap:<...>`.
    Extmap(Extmap),
    /// Any attribute this crate does not model specifically.
    Other {
        /// Attribute name (text before the first `:`).
        name: String,
        /// Attribute value (text after the first `:`), if any.
        value: Option<String>,
    },
}

impl Attribute {
    /// Parse the text following `a=` into a typed attribute.
    ///
    /// # Errors
    /// Returns an error if a recognised attribute is structurally invalid.
    /// Unrecognised attributes never error — they become [`Attribute::Other`].
    pub fn parse(text: &str) -> Result<Self> {
        let (name, value) = text
            .split_once(':')
            .map_or((text, None), |(n, v)| (n, Some(v)));

        let need = |field: &'static str| {
            value.ok_or(SdpError::MissingField { field })
        };

        Ok(match name {
            "rtcp-mux" => Self::RtcpMux,
            "rtcp-mux-only" => Self::RtcpMuxOnly,
            "rtcp-rsize" => Self::RtcpRsize,
            "end-of-candidates" => Self::EndOfCandidates,
            "bundle-only" => Self::BundleOnly,
            "sendrecv" => Self::Direction(Direction::SendRecv),
            "sendonly" => Self::Direction(Direction::SendOnly),
            "recvonly" => Self::Direction(Direction::RecvOnly),
            "inactive" => Self::Direction(Direction::Inactive),
            "group" => Self::Group(Group::parse(need("group")?)?),
            "mid" => Self::Mid(need("mid")?.to_owned()),
            "msid" => Self::Msid(need("msid")?.to_owned()),
            "ice-ufrag" => Self::IceUfrag(need("ice-ufrag")?.to_owned()),
            "ice-pwd" => Self::IcePwd(need("ice-pwd")?.to_owned()),
            "ice-options" => Self::IceOptions(need("ice-options")?.to_owned()),
            "candidate" => Self::Candidate(Candidate::parse(need("candidate")?)?),
            "fingerprint" => Self::Fingerprint(Fingerprint::parse(need("fingerprint")?)?),
            "setup" => Self::Setup(SetupRole::parse(need("setup")?)?),
            "rtpmap" => Self::RtpMap(RtpMap::parse(need("rtpmap")?)?),
            "fmtp" => Self::Fmtp(Fmtp::parse(need("fmtp")?)?),
            "rtcp-fb" => Self::RtcpFb(RtcpFb::parse(need("rtcp-fb")?)?),
            "ssrc" => Self::Ssrc(Ssrc::parse(need("ssrc")?)?),
            "ssrc-group" => Self::SsrcGroup(SsrcGroup::parse(need("ssrc-group")?)?),
            "extmap" => Self::Extmap(Extmap::parse(need("extmap")?)?),
            _ => Self::Other {
                name: name.to_owned(),
                value: value.map(ToOwned::to_owned),
            },
        })
    }

    /// The attribute name (the token before any `:`).
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::RtcpMux => "rtcp-mux",
            Self::RtcpMuxOnly => "rtcp-mux-only",
            Self::RtcpRsize => "rtcp-rsize",
            Self::EndOfCandidates => "end-of-candidates",
            Self::BundleOnly => "bundle-only",
            Self::Group(_) => "group",
            Self::Mid(_) => "mid",
            Self::Msid(_) => "msid",
            Self::IceUfrag(_) => "ice-ufrag",
            Self::IcePwd(_) => "ice-pwd",
            Self::IceOptions(_) => "ice-options",
            Self::Candidate(_) => "candidate",
            Self::Fingerprint(_) => "fingerprint",
            Self::Setup(_) => "setup",
            Self::Direction(d) => d.token(),
            Self::RtpMap(_) => "rtpmap",
            Self::Fmtp(_) => "fmtp",
            Self::RtcpFb(_) => "rtcp-fb",
            Self::Ssrc(_) => "ssrc",
            Self::SsrcGroup(_) => "ssrc-group",
            Self::Extmap(_) => "extmap",
            Self::Other { name, .. } => name,
        }
    }
}

impl fmt::Display for Attribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RtcpMux
            | Self::RtcpMuxOnly
            | Self::RtcpRsize
            | Self::EndOfCandidates
            | Self::BundleOnly
            | Self::Direction(_) => f.write_str(self.name()),
            Self::Group(g) => write!(f, "group:{g}"),
            Self::Mid(v) => write!(f, "mid:{v}"),
            Self::Msid(v) => write!(f, "msid:{v}"),
            Self::IceUfrag(v) => write!(f, "ice-ufrag:{v}"),
            Self::IcePwd(v) => write!(f, "ice-pwd:{v}"),
            Self::IceOptions(v) => write!(f, "ice-options:{v}"),
            Self::Candidate(c) => write!(f, "candidate:{c}"),
            Self::Fingerprint(fp) => write!(f, "fingerprint:{fp}"),
            Self::Setup(s) => write!(f, "setup:{s}"),
            Self::RtpMap(m) => write!(f, "rtpmap:{m}"),
            Self::Fmtp(m) => write!(f, "fmtp:{m}"),
            Self::RtcpFb(m) => write!(f, "rtcp-fb:{m}"),
            Self::Ssrc(s) => write!(f, "ssrc:{s}"),
            Self::SsrcGroup(s) => write!(f, "ssrc-group:{s}"),
            Self::Extmap(e) => write!(f, "extmap:{e}"),
            Self::Other { name, value } => match value {
                Some(v) => write!(f, "{name}:{v}"),
                None => f.write_str(name),
            },
        }
    }
}

pub(crate) const REDACTED: &str = "<redacted>";

pub(crate) fn is_credential_attribute(name: &str) -> bool {
    ["ice-pwd", "crypto", "key-mgmt"]
        .iter()
        .any(|credential| name.eq_ignore_ascii_case(credential))
}

impl fmt::Debug for Attribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RtcpMux => f.write_str("RtcpMux"),
            Self::RtcpMuxOnly => f.write_str("RtcpMuxOnly"),
            Self::RtcpRsize => f.write_str("RtcpRsize"),
            Self::EndOfCandidates => f.write_str("EndOfCandidates"),
            Self::BundleOnly => f.write_str("BundleOnly"),
            Self::Group(g) => f.debug_tuple("Group").field(g).finish(),
            Self::Mid(v) => f.debug_tuple("Mid").field(v).finish(),
            Self::Msid(v) => f.debug_tuple("Msid").field(v).finish(),
            Self::IceUfrag(v) => f.debug_tuple("IceUfrag").field(v).finish(),
            Self::IcePwd(_) => f.debug_tuple("IcePwd").field(&REDACTED).finish(),
            Self::IceOptions(v) => f.debug_tuple("IceOptions").field(v).finish(),
            Self::Candidate(c) => f.debug_tuple("Candidate").field(c).finish(),
            Self::Fingerprint(fp) => f.debug_tuple("Fingerprint").field(fp).finish(),
            Self::Setup(s) => f.debug_tuple("Setup").field(s).finish(),
            Self::Direction(d) => f.debug_tuple("Direction").field(d).finish(),
            Self::RtpMap(m) => f.debug_tuple("RtpMap").field(m).finish(),
            Self::Fmtp(m) => f.debug_tuple("Fmtp").field(m).finish(),
            Self::RtcpFb(m) => f.debug_tuple("RtcpFb").field(m).finish(),
            Self::Ssrc(s) => f.debug_tuple("Ssrc").field(s).finish(),
            Self::SsrcGroup(s) => f.debug_tuple("SsrcGroup").field(s).finish(),
            Self::Extmap(e) => f.debug_tuple("Extmap").field(e).finish(),
            Self::Other { name, value } => {
                let redacted = value.as_ref().map(|_| REDACTED);
                let value: &dyn fmt::Debug = if is_credential_attribute(name) {
                    &redacted
                } else {
                    value
                };
                f.debug_struct("Other")
                    .field("name", name)
                    .field("value", value)
                    .finish()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(text: &str) {
        let attr = Attribute::parse(text).expect("parse");
        assert_eq!(attr.to_string(), text, "round-trip mismatch for {text:?}");
    }

    #[test]
    fn flags_and_directions() {
        round_trip("rtcp-mux");
        round_trip("rtcp-rsize");
        round_trip("end-of-candidates");
        round_trip("bundle-only");
        round_trip("sendrecv");
        round_trip("inactive");
    }

    #[test]
    fn typed_attributes_round_trip() {
        round_trip("mid:0");
        round_trip("group:BUNDLE 0 1");
        round_trip("ice-ufrag:8hhY");
        round_trip("setup:actpass");
        round_trip("rtpmap:111 opus/48000/2");
        round_trip("rtpmap:96 VP8/90000");
        round_trip("fmtp:111 minptime=10;useinbandfec=1");
        round_trip("rtcp-fb:96 nack pli");
        round_trip("ssrc:1234 cname:abc");
        round_trip("ssrc:1234 mslabel");
        round_trip("ssrc-group:FID 1111 2222");
        round_trip("extmap:1 urn:ietf:params:rtp-hdrext:sdes:mid");
    }

    #[test]
    fn unknown_is_preserved() {
        round_trip("x-vendor:custom-value");
        round_trip("recvonly-ish");
        match Attribute::parse("x-flag").unwrap() {
            Attribute::Other { name, value } => {
                assert_eq!(name, "x-flag");
                assert!(value.is_none());
            }
            other => panic!("expected Other, got {other:?}"),
        }
    }

    #[test]
    fn missing_value_errors() {
        assert!(Attribute::parse("mid").is_err());
        assert!(Attribute::parse("rtpmap").is_err());
    }
}
