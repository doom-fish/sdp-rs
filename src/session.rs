//! Session-level SDP types: [`Sdp`], the [`Offer`]/[`Answer`] wrappers, and
//! the [`AnswerBuilder`].

use crate::attributes::{Attribute, Direction, SetupRole, REDACTED};
use crate::bundle::Group;
use crate::error::{Result, SdpError};
use crate::extmap::Extmap;
use crate::fingerprint::Fingerprint;
use crate::media::{MediaDescription, MediaKind};
use crate::parser;
use std::collections::hash_map::RandomState;
use std::fmt;
use std::hash::BuildHasher;

/// The `o=` origin line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Origin {
    /// Originator user name (`-` if not relevant).
    pub username: String,
    /// Session identifier (numeric, but kept as text to avoid precision loss).
    pub session_id: String,
    /// Session version (incremented on each modified offer/answer).
    pub session_version: String,
    /// Network type (`IN`).
    pub net_type: String,
    /// Address type (`IP4` / `IP6`).
    pub addr_type: String,
    /// Unicast address of the originator.
    pub unicast_address: String,
}

impl Origin {
    pub(crate) fn parse(value: &str) -> Result<Self> {
        let parts: Vec<&str> = value.split(' ').collect();
        if parts.len() != 6 {
            return Err(SdpError::invalid("o", value));
        }
        // RFC 8866 §5.2: sess-id and sess-version are numeric strings. They are
        // kept as text to avoid u64 precision loss, but must still be digits.
        for field in [parts[1], parts[2]] {
            if field.is_empty() || !field.bytes().all(|b| b.is_ascii_digit()) {
                return Err(SdpError::invalid("o", value));
            }
        }
        Ok(Self {
            username: parts[0].to_owned(),
            session_id: parts[1].to_owned(),
            session_version: parts[2].to_owned(),
            net_type: parts[3].to_owned(),
            addr_type: parts[4].to_owned(),
            unicast_address: parts[5].to_owned(),
        })
    }
}

impl fmt::Display for Origin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} {} {} {} {}",
            self.username,
            self.session_id,
            self.session_version,
            self.net_type,
            self.addr_type,
            self.unicast_address
        )
    }
}

/// A `c=` connection line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Connection {
    /// Network type (`IN`).
    pub net_type: String,
    /// Address type (`IP4` / `IP6`).
    pub addr_type: String,
    /// Connection address (may include TTL / multicast count).
    pub address: String,
}

impl Connection {
    pub(crate) fn parse(value: &str) -> Result<Self> {
        let parts: Vec<&str> = value.split(' ').collect();
        if parts.len() != 3 {
            return Err(SdpError::invalid("c", value));
        }
        Ok(Self {
            net_type: parts[0].to_owned(),
            addr_type: parts[1].to_owned(),
            address: parts[2].to_owned(),
        })
    }
}

impl fmt::Display for Connection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} {}", self.net_type, self.addr_type, self.address)
    }
}

/// A `b=` bandwidth line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bandwidth {
    /// Bandwidth type (`AS`, `CT`, `TIAS`, ...).
    pub bwtype: String,
    /// Bandwidth value (units depend on `bwtype`).
    pub bandwidth: u64,
}

impl Bandwidth {
    pub(crate) fn parse(value: &str) -> Result<Self> {
        let (bwtype, num) = value
            .split_once(':')
            .ok_or_else(|| SdpError::invalid("b", value))?;
        let bandwidth = num
            .parse()
            .map_err(|_| SdpError::number("b", num))?;
        Ok(Self {
            bwtype: bwtype.to_owned(),
            bandwidth,
        })
    }
}

impl fmt::Display for Bandwidth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.bwtype, self.bandwidth)
    }
}

/// A `t=` timing line together with any following `r=` repeat lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Timing {
    /// Start time (NTP seconds, or `0`).
    pub start: u64,
    /// Stop time (NTP seconds, or `0` for unbounded).
    pub stop: u64,
    /// Raw `r=` repeat values, preserved verbatim.
    pub repeats: Vec<String>,
}

impl Timing {
    /// A permanent session (`t=0 0`).
    #[must_use]
    pub const fn permanent() -> Self {
        Self {
            start: 0,
            stop: 0,
            repeats: Vec::new(),
        }
    }

    pub(crate) fn parse(value: &str) -> Result<Self> {
        let (start, stop) = value
            .split_once(' ')
            .ok_or_else(|| SdpError::invalid("t", value))?;
        Ok(Self {
            start: start
                .parse()
                .map_err(|_| SdpError::number("t.start", start))?,
            stop: stop.parse().map_err(|_| SdpError::number("t.stop", stop))?,
            repeats: Vec::new(),
        })
    }
}

/// A complete parsed SDP document.
#[derive(Clone, PartialEq, Eq)]
pub struct Sdp {
    /// `v=` protocol version (always `0`).
    pub version: u32,
    /// `o=` origin.
    pub origin: Origin,
    /// `s=` session name.
    pub session_name: String,
    /// `i=` session information.
    pub information: Option<String>,
    /// `u=` URI.
    pub uri: Option<String>,
    /// `e=` email addresses.
    pub emails: Vec<String>,
    /// `p=` phone numbers.
    pub phones: Vec<String>,
    /// `c=` session-level connection.
    pub connection: Option<Connection>,
    /// `b=` session-level bandwidth lines.
    pub bandwidths: Vec<Bandwidth>,
    /// `t=`/`r=` time descriptions.
    pub timing: Vec<Timing>,
    /// `z=` time-zone adjustments (raw).
    pub time_zones: Option<String>,
    /// `k=` session-level encryption key (deprecated, preserved).
    pub key: Option<String>,
    /// `a=` session-level attributes.
    pub attributes: Vec<Attribute>,
    /// `m=` media descriptions.
    pub media: Vec<MediaDescription>,
}

impl fmt::Debug for Sdp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Sdp")
            .field("version", &self.version)
            .field("origin", &self.origin)
            .field("session_name", &self.session_name)
            .field("information", &self.information)
            .field("uri", &self.uri)
            .field("emails", &self.emails)
            .field("phones", &self.phones)
            .field("connection", &self.connection)
            .field("bandwidths", &self.bandwidths)
            .field("timing", &self.timing)
            .field("time_zones", &self.time_zones)
            .field("key", &self.key.as_ref().map(|_| REDACTED))
            .field("attributes", &self.attributes)
            .field("media", &self.media)
            .finish()
    }
}

impl Sdp {
    /// Parse SDP text into a structured [`Sdp`].
    ///
    /// # Errors
    /// Returns an error if a required line is missing or any line is malformed.
    pub fn parse(text: &str) -> Result<Self> {
        parser::parse_sdp(text)
    }

    /// The `a=group:BUNDLE` group at the session level, if present.
    #[must_use]
    pub fn bundle_group(&self) -> Option<&Group> {
        self.attributes.iter().find_map(|a| match a {
            Attribute::Group(g) if g.is_bundle() => Some(g),
            _ => None,
        })
    }

    /// Find a media section by its `mid`.
    #[must_use]
    pub fn media_by_mid(&self, mid: &str) -> Option<&MediaDescription> {
        self.media.iter().find(|m| m.mid() == Some(mid))
    }

    #[allow(missing_docs)]
    #[must_use]
    pub fn fingerprints_for<'a>(&'a self, media: &'a MediaDescription) -> Vec<&'a Fingerprint> {
        let media_level = media.fingerprints();
        if !media_level.is_empty() {
            return media_level;
        }
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
    pub fn setup_for(&self, media: &MediaDescription) -> Option<SetupRole> {
        media.setup().or_else(|| {
            self.attributes.iter().find_map(|a| match a {
                Attribute::Setup(s) => Some(*s),
                _ => None,
            })
        })
    }

    #[allow(missing_docs)]
    #[must_use]
    pub fn ice_ufrag_for<'a>(&'a self, media: &'a MediaDescription) -> Option<&'a str> {
        media.ice_ufrag().or_else(|| {
            self.attributes.iter().find_map(|a| match a {
                Attribute::IceUfrag(u) => Some(u.as_str()),
                _ => None,
            })
        })
    }

    #[allow(missing_docs)]
    #[must_use]
    pub fn ice_pwd_for<'a>(&'a self, media: &'a MediaDescription) -> Option<&'a str> {
        media.ice_pwd().or_else(|| {
            self.attributes.iter().find_map(|a| match a {
                Attribute::IcePwd(p) => Some(p.as_str()),
                _ => None,
            })
        })
    }
}

/// A received SDP offer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offer(pub Sdp);

impl Offer {
    /// Parse offer text.
    ///
    /// # Errors
    /// Returns an error if the document is not valid SDP.
    pub fn parse(text: &str) -> Result<Self> {
        Ok(Self(Sdp::parse(text)?))
    }

    /// Access the underlying [`Sdp`].
    #[must_use]
    pub const fn sdp(&self) -> &Sdp {
        &self.0
    }

    /// Start building an [`Answer`] for this offer.
    #[must_use]
    pub fn answer() -> AnswerBuilder {
        AnswerBuilder::default()
    }
}

/// A generated SDP answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer(pub Sdp);

impl Answer {
    /// Start building an answer.
    #[must_use]
    pub fn builder() -> AnswerBuilder {
        AnswerBuilder::default()
    }

    /// Parse answer text.
    ///
    /// # Errors
    /// Returns an error if the document is not valid SDP.
    pub fn parse(text: &str) -> Result<Self> {
        Ok(Self(Sdp::parse(text)?))
    }

    /// Access the underlying [`Sdp`].
    #[must_use]
    pub const fn sdp(&self) -> &Sdp {
        &self.0
    }
}

/// The keying / identity material a local peer contributes to an answer.
#[derive(Clone)]
pub struct LocalIdentity {
    /// ICE username fragment.
    pub ice_ufrag: String,
    /// ICE password.
    pub ice_pwd: String,
    /// DTLS certificate fingerprint.
    pub fingerprint: Fingerprint,
}

impl LocalIdentity {
    /// Construct local identity material.
    #[must_use]
    pub fn new(
        ice_ufrag: impl Into<String>,
        ice_pwd: impl Into<String>,
        fingerprint: Fingerprint,
    ) -> Self {
        Self {
            ice_ufrag: ice_ufrag.into(),
            ice_pwd: ice_pwd.into(),
            fingerprint,
        }
    }
}

impl fmt::Debug for LocalIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LocalIdentity")
            .field("ice_ufrag", &self.ice_ufrag)
            .field("ice_pwd", &REDACTED)
            .field("fingerprint", &self.fingerprint)
            .finish()
    }
}

impl PartialEq for LocalIdentity {
    fn eq(&self, other: &Self) -> bool {
        let pwd_eq = constant_time_eq(self.ice_pwd.as_bytes(), other.ice_pwd.as_bytes());
        pwd_eq && self.ice_ufrag == other.ice_ufrag && self.fingerprint == other.fingerprint
    }
}

impl Eq for LocalIdentity {}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let diff = a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y));
    std::hint::black_box(diff) == 0
}

#[derive(Debug, Clone, Default)]
struct MediaPlan {
    accept: bool,
    attributes: Vec<Attribute>,
}

/// Builder for [`Answer`] documents.
///
/// The builder records which media kinds to accept (and with which payload
/// attributes) and which to reject, then [`AnswerBuilder::build`] mirrors the
/// offer's media order, answering each section accordingly.
#[derive(Debug, Clone, Default)]
pub struct AnswerBuilder {
    identity: Option<LocalIdentity>,
    audio: Option<MediaPlan>,
    video: Option<MediaPlan>,
    reject_application: bool,
    previous_origin: Option<Origin>,
}

impl AnswerBuilder {
    /// Provide the local ICE credentials and DTLS fingerprint used on every
    /// accepted media section.
    #[must_use]
    pub fn identity(mut self, identity: LocalIdentity) -> Self {
        self.identity = Some(identity);
        self
    }

    /// Accept the audio section, adding the supplied media-level attributes
    /// (typically `rtpmap`/`fmtp`/`rtcp-fb` for the codecs to keep).
    ///
    /// The answer's `m=` line lists only the offered payload types that have
    /// an `a=rtpmap` among these attributes, and attributes for other payload
    /// types are dropped. If no offered payload type matches, the section is
    /// rejected with port 0.
    #[must_use]
    pub fn accept_audio(mut self, attributes: Vec<Attribute>) -> Self {
        self.audio = Some(MediaPlan {
            accept: true,
            attributes,
        });
        self
    }

    /// Accept the video section with the supplied media-level attributes,
    /// selecting payload types as [`accept_audio`](Self::accept_audio) does.
    #[must_use]
    pub fn accept_video(mut self, attributes: Vec<Attribute>) -> Self {
        self.video = Some(MediaPlan {
            accept: true,
            attributes,
        });
        self
    }

    /// Explicitly reject any data-channel (`m=application`) section by
    /// answering it with port 0 (RFC 8843 §7.2). This is required for browsers
    /// to complete negotiation when SCTP is not supported.
    #[must_use]
    pub const fn reject_data(mut self) -> Self {
        self.reject_application = true;
        self
    }

    #[allow(missing_docs)]
    #[must_use]
    pub fn previous_answer(mut self, previous: &Answer) -> Self {
        self.previous_origin = Some(previous.0.origin.clone());
        self
    }

    /// Build the answer for `offer`.
    ///
    /// # Errors
    /// Returns an error if the offer is missing required keying material, or
    /// if the builder was asked to accept media the offer does not contain
    /// while lacking local identity material, or if the previous answer's
    /// session version cannot be incremented.
    pub fn build(&self, offer: &Offer) -> Result<Answer> {
        let offer = &offer.0;
        let identity = self.identity.as_ref();
        let group: &[String] = offer.bundle_group().map_or(&[], |g| g.mids.as_slice());
        let in_group =
            |m: &MediaDescription| m.mid().is_some_and(|mid| group.iter().any(|g| g == mid));
        let selected: Vec<Vec<String>> = offer
            .media
            .iter()
            .map(|m| self.supported_formats(m))
            .collect();
        let tag = group.iter().find(|mid| {
            offer.media.iter().zip(&selected).any(|(m, formats)| {
                m.mid() == Some(mid.as_str()) && m.port != 0 && !formats.is_empty()
            })
        });

        let mut media = Vec::with_capacity(offer.media.len());
        let mut bundle_mids = Vec::new();

        for (offered, formats) in offer.media.iter().zip(selected) {
            let active = offered.port != 0
                || (offered.is_bundle_only() && tag.is_some() && in_group(offered));
            if !active || formats.is_empty() {
                media.push(reject_media_section(offered));
                continue;
            }
            let answered = self.answer_section(offer, offered, identity, formats)?;
            if in_group(offered) {
                if let Some(mid) = answered.mid() {
                    bundle_mids.push(mid.to_owned());
                }
            }
            media.push(answered);
        }
        if let Some(position) = tag.and_then(|tag| bundle_mids.iter().position(|m| m == tag)) {
            let tagged = bundle_mids.remove(position);
            bundle_mids.insert(0, tagged);
        }

        let mut attributes = Vec::new();
        if !bundle_mids.is_empty() {
            attributes.push(Attribute::Group(Group::bundle(bundle_mids)));
        }
        // Mirror trickle ICE support if the offer advertised it.
        if offer
            .attributes
            .iter()
            .chain(offer.media.iter().flat_map(|m| m.attributes.iter()))
            .any(|a| {
                matches!(a, Attribute::IceOptions(o) if o.split_whitespace().any(|t| t == "trickle"))
            })
        {
            attributes.push(Attribute::IceOptions("trickle".to_owned()));
        }

        let sdp = Sdp {
            version: 0,
            origin: self.origin()?,
            session_name: "-".to_owned(),
            information: None,
            uri: None,
            emails: Vec::new(),
            phones: Vec::new(),
            connection: None,
            bandwidths: Vec::new(),
            timing: vec![Timing::permanent()],
            time_zones: None,
            key: None,
            attributes,
            media,
        };

        Ok(Answer(sdp))
    }

    fn origin(&self) -> Result<Origin> {
        let Some(previous) = &self.previous_origin else {
            return Ok(Origin {
                username: "-".to_owned(),
                session_id: (RandomState::new().hash_one(0_u8) >> 1).to_string(),
                session_version: "1".to_owned(),
                net_type: "IN".to_owned(),
                addr_type: "IP4".to_owned(),
                unicast_address: "0.0.0.0".to_owned(),
            });
        };
        let next = previous
            .session_version
            .parse::<i64>()
            .ok()
            .and_then(|v| v.checked_add(1))
            .ok_or_else(|| SdpError::invalid("o.sess-version", previous.session_version.clone()))?;
        Ok(Origin {
            session_version: next.to_string(),
            ..previous.clone()
        })
    }

    fn plan_for(&self, kind: &MediaKind) -> Option<&MediaPlan> {
        match kind {
            MediaKind::Audio => self.audio.as_ref(),
            MediaKind::Video => self.video.as_ref(),
            _ => None,
        }
        .filter(|p| p.accept)
    }

    fn supported_formats(&self, offered: &MediaDescription) -> Vec<String> {
        self.plan_for(&offered.kind).map_or_else(Vec::new, |p| {
            offered
                .formats
                .iter()
                .filter(|f| {
                    p.attributes
                        .iter()
                        .any(|a| matches!(a, Attribute::RtpMap(m) if m.payload.to_string() == **f))
                })
                .cloned()
                .collect()
        })
    }

    fn answer_section(
        &self,
        offer: &Sdp,
        offered: &MediaDescription,
        identity: Option<&LocalIdentity>,
        formats: Vec<String>,
    ) -> Result<MediaDescription> {
        let plan = self.plan_for(&offered.kind);

        let identity = identity.ok_or_else(|| {
            SdpError::profile(
                "accepting media requires local identity (ICE + fingerprint); \
                 call AnswerBuilder::identity",
            )
        })?;

        let offer_has_fingerprint = offer
            .attributes
            .iter()
            .chain(offer.media.iter().flat_map(|m| m.attributes.iter()))
            .any(|a| matches!(a, Attribute::Fingerprint(_)));
        if !offer_has_fingerprint {
            return Err(SdpError::profile(
                "the offer carries no a=fingerprint, so DTLS-SRTP cannot be keyed",
            ));
        }

        let mid = offered
            .mid()
            .ok_or_else(|| SdpError::profile("offered media section is missing a=mid"))?
            .to_owned();

        let plan_attributes: Vec<Attribute> = plan
            .map(|p| {
                p.attributes
                    .iter()
                    .filter(|a| applies_to_formats(a, &formats))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();

        let mut answer = MediaDescription::new(offered.kind.clone(), 9, offered.proto.clone());
        answer.formats = formats;
        answer.connection = Some(Connection {
            net_type: "IN".to_owned(),
            addr_type: "IP4".to_owned(),
            address: "0.0.0.0".to_owned(),
        });

        answer.push_attribute(Attribute::Mid(mid));

        // Direction is the reciprocal of the offer (default sendrecv).
        let direction = offered.direction().unwrap_or(Direction::SendRecv).reverse();
        answer.push_attribute(Attribute::Direction(direction));

        if offered.has_rtcp_mux() {
            answer.push_attribute(Attribute::RtcpMux);
        }

        answer.push_attribute(Attribute::IceUfrag(identity.ice_ufrag.clone()));
        answer.push_attribute(Attribute::IcePwd(identity.ice_pwd.clone()));
        answer.push_attribute(Attribute::Fingerprint(identity.fingerprint.clone()));

        let setup = SetupRole::answer_for(offer.setup_for(offered).unwrap_or(SetupRole::ActPass));
        answer.push_attribute(Attribute::Setup(setup));

        // Preserve MID/TWCC header-extension negotiation so congestion control
        // keeps working with browsers and SFUs.
        for attr in &offered.attributes {
            if let Attribute::Extmap(ext) = attr {
                if ext.uri == crate::extmap::URI_MID || ext.uri == crate::extmap::URI_TWCC {
                    answer.push_attribute(Attribute::Extmap(Extmap::new(ext.id, ext.uri.clone())));
                }
            }
        }

        for attr in plan_attributes {
            answer.push_attribute(attr);
        }

        Ok(answer)
    }
}

fn applies_to_formats(attr: &Attribute, formats: &[String]) -> bool {
    let format = match attr {
        Attribute::RtpMap(m) => return formats.contains(&m.payload.to_string()),
        Attribute::Fmtp(f) => &f.format,
        Attribute::RtcpFb(fb) => &fb.format,
        _ => return true,
    };
    format == "*" || formats.iter().any(|f| f == format)
}

/// Build a rejected mirror of `offered`.
///
/// Same media kind and protocol, port 0, formats preserved, and a single
/// `a=mid` so BUNDLE bookkeeping stays valid (RFC 8843 §7.2). This is how a
/// data channel is cleanly declined.
#[must_use]
pub fn reject_media_section(offered: &MediaDescription) -> MediaDescription {
    let mut rejected = MediaDescription::new(offered.kind.clone(), 0, offered.proto.clone());
    rejected.formats.clone_from(&offered.formats);
    if let Some(mid) = offered.mid() {
        rejected.push_attribute(Attribute::Mid(mid.to_owned()));
    }
    rejected
}
