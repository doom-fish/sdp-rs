//! SDP generator: renders an [`Sdp`] back to RFC 8866 text.
//!
//! Lines are emitted in the canonical RFC 8866 order with CRLF terminators.
//! Recognised attributes reconstruct exactly from their typed form, so a
//! conforming document parsed and re-emitted is byte-for-byte stable (modulo
//! line ordering the RFC permits to vary).

use crate::error::{Result, SdpError};
use crate::media::MediaDescription;
use crate::session::Sdp;
use std::fmt::Write as _;

const CRLF: &str = "\r\n";

/// Render an [`Sdp`] as RFC 8866 text.
///
/// # Errors
/// Returns [`SdpError::ForbiddenCharacter`] if any emitted value contains a
/// CR, LF or NUL character, which would otherwise inject extra lines, and
/// [`SdpError::InvalidValue`] if the version is not `0`.
pub fn write_sdp(sdp: &Sdp) -> Result<String> {
    if sdp.version != 0 {
        return Err(SdpError::invalid("v", sdp.version.to_string()));
    }
    let mut out = String::new();

    line(&mut out, 'v', &sdp.version.to_string())?;
    line(&mut out, 'o', &sdp.origin.to_string())?;
    line(&mut out, 's', &sdp.session_name)?;
    if let Some(i) = &sdp.information {
        line(&mut out, 'i', i)?;
    }
    if let Some(u) = &sdp.uri {
        line(&mut out, 'u', u)?;
    }
    for e in &sdp.emails {
        line(&mut out, 'e', e)?;
    }
    for p in &sdp.phones {
        line(&mut out, 'p', p)?;
    }
    if let Some(c) = &sdp.connection {
        line(&mut out, 'c', &c.to_string())?;
    }
    for b in &sdp.bandwidths {
        line(&mut out, 'b', &b.to_string())?;
    }
    if sdp.timing.is_empty() {
        line(&mut out, 't', "0 0")?;
    }
    for t in &sdp.timing {
        line(&mut out, 't', &format!("{} {}", t.start, t.stop))?;
        for r in &t.repeats {
            line(&mut out, 'r', r)?;
        }
    }
    if let Some(z) = &sdp.time_zones {
        line(&mut out, 'z', z)?;
    }
    if let Some(k) = &sdp.key {
        line(&mut out, 'k', k)?;
    }
    for a in &sdp.attributes {
        line(&mut out, 'a', &a.to_string())?;
    }

    for m in &sdp.media {
        write_media(&mut out, m)?;
    }

    Ok(out)
}

fn write_media(out: &mut String, media: &MediaDescription) -> Result<()> {
    line(out, 'm', &media.m_line())?;
    if let Some(i) = &media.title {
        line(out, 'i', i)?;
    }
    if let Some(c) = &media.connection {
        line(out, 'c', &c.to_string())?;
    }
    for b in &media.bandwidths {
        line(out, 'b', &b.to_string())?;
    }
    if let Some(k) = &media.key {
        line(out, 'k', k)?;
    }
    for a in &media.attributes {
        line(out, 'a', &a.to_string())?;
    }
    Ok(())
}

fn line(out: &mut String, kind: char, value: &str) -> Result<()> {
    if let Some(character) = value.chars().find(|c| matches!(c, '\r' | '\n' | '\0')) {
        return Err(SdpError::ForbiddenCharacter {
            line_type: kind,
            character,
        });
    }
    // `write!` to a String is infallible.
    let _ = write!(out, "{kind}={value}{CRLF}");
    Ok(())
}
