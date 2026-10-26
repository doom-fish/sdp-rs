//! Hand-rolled, line-by-line RFC 8866 parser.
//!
//! SDP is a strictly line-oriented `<type>=<value>` format. This parser walks
//! the input once, routing each line into the session description or the media
//! description currently being built. It is deliberately lenient about the
//! relative order of the optional session lines (real-world offers vary) while
//! still rejecting structurally malformed input with [`SdpError`].

use crate::attributes::{is_credential_attribute, Attribute, REDACTED};
use crate::error::{Result, SdpError};
use crate::media::MediaDescription;
use crate::session::{Bandwidth, Connection, Origin, Sdp, Timing};

/// Parse SDP text into a structured [`Sdp`].
///
/// # Errors
/// Returns an error if a required line (`v=`, `o=`, `s=`) is missing, the
/// version is not `0`, or any line is malformed.
pub fn parse_sdp(text: &str) -> Result<Sdp> {
    let mut version: Option<u32> = None;
    let mut origin: Option<Origin> = None;
    let mut session_name: Option<String> = None;
    let mut information = None;
    let mut uri = None;
    let mut emails = Vec::new();
    let mut phones = Vec::new();
    let mut connection = None;
    let mut bandwidths = Vec::new();
    let mut timing: Vec<Timing> = Vec::new();
    let mut time_zones = None;
    let mut key = None;
    let mut attributes = Vec::new();
    let mut media: Vec<MediaDescription> = Vec::new();

    for (idx, raw) in text.split('\n').enumerate() {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        if line.is_empty() {
            continue;
        }
        let line_no = idx + 1;
        if line.contains(['\r', '\0']) {
            return Err(malformed(line_no, line));
        }
        let bytes = line.as_bytes();
        if bytes.len() < 2 || bytes[1] != b'=' {
            return Err(malformed(line_no, line));
        }
        let kind = bytes[0] as char;
        let value = &line[2..];

        // Once a media section has started, most lines belong to it.
        if let Some(current) = media.last_mut() {
            if kind != 'm' {
                route_media_line(current, kind, value, line_no)?;
                continue;
            }
        }

        match kind {
            'v' => {
                let number: u32 = value.parse().map_err(|_| SdpError::number("v", value))?;
                if number != 0 {
                    return Err(SdpError::invalid("v", value));
                }
                version = Some(number);
            }
            'o' => origin = Some(Origin::parse(value)?),
            's' => session_name = Some(value.to_owned()),
            'i' => information = Some(value.to_owned()),
            'u' => uri = Some(value.to_owned()),
            'e' => emails.push(value.to_owned()),
            'p' => phones.push(value.to_owned()),
            'c' => connection = Some(Connection::parse(value)?),
            'b' => bandwidths.push(Bandwidth::parse(value)?),
            't' => timing.push(Timing::parse(value)?),
            'r' => {
                timing
                    .last_mut()
                    .ok_or_else(|| SdpError::invalid("r", "repeat without preceding t="))?
                    .repeats
                    .push(value.to_owned());
            }
            'z' => time_zones = Some(value.to_owned()),
            'k' => key = Some(value.to_owned()),
            'a' => attributes.push(Attribute::parse(value)?),
            'm' => media.push(MediaDescription::parse_m_line(value)?),
            _ => return Err(malformed(line_no, line)),
        }
    }

    let version = version.ok_or(SdpError::MissingField { field: "v=" })?;
    let origin = origin.ok_or(SdpError::MissingField { field: "o=" })?;
    let session_name = session_name.ok_or(SdpError::MissingField { field: "s=" })?;

    Ok(Sdp {
        version,
        origin,
        session_name,
        information,
        uri,
        emails,
        phones,
        connection,
        bandwidths,
        timing,
        time_zones,
        key,
        attributes,
        media,
    })
}

fn route_media_line(
    media: &mut MediaDescription,
    kind: char,
    value: &str,
    line_no: usize,
) -> Result<()> {
    match kind {
        'i' => media.title = Some(value.to_owned()),
        'c' => media.connection = Some(Connection::parse(value)?),
        'b' => media.bandwidths.push(Bandwidth::parse(value)?),
        'k' => media.key = Some(value.to_owned()),
        'a' => media.attributes.push(Attribute::parse(value)?),
        _ => return Err(malformed(line_no, &format!("{kind}={value}"))),
    }
    Ok(())
}

fn malformed(line: usize, text: &str) -> SdpError {
    SdpError::MalformedLine {
        line,
        text: redact_credentials(text),
    }
}

fn redact_credentials(text: &str) -> String {
    let secret_start = if text.starts_with("k=") {
        Some(2)
    } else {
        text.strip_prefix("a=").and_then(|attribute| {
            let name_len = attribute
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
                .unwrap_or(attribute.len());
            let separator = usize::from(attribute[name_len..].starts_with(':'));
            is_credential_attribute(&attribute[..name_len]).then_some(2 + name_len + separator)
        })
    };
    secret_start.map_or_else(
        || text.to_owned(),
        |start| format!("{}{REDACTED}", &text[..start]),
    )
}
