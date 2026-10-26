//! Typed helpers for the Opus `a=fmtp:` parameter string (RFC 7587 §6.1).
//!
//! Opus format parameters are a `;`-separated `key=value` list. This module
//! exposes the parameters a real-time meeting client cares about while
//! preserving ordering and unknown parameters for lossless round-trips.

use crate::fmtp::FmtpParams;

#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Opus;

/// Ordered key/value view over an Opus fmtp parameter string.
pub type OpusFmtp = FmtpParams<Opus>;

impl OpusFmtp {
    /// Sensible defaults for a real-time meeting client: 10 ms minimum packet
    /// time, in-band FEC enabled, DTX enabled, and stereo allowed.
    #[must_use]
    pub fn meeting_defaults() -> Self {
        let mut fmtp = Self::new();
        fmtp.set("minptime", "10");
        fmtp.set("useinbandfec", "1");
        fmtp.set("usedtx", "1");
        fmtp.set("stereo", "1");
        fmtp
    }

    /// Minimum packet duration in milliseconds (`minptime`).
    #[must_use]
    pub fn minptime(&self) -> Option<u32> {
        self.get("minptime").and_then(|v| v.parse().ok())
    }

    /// Whether in-band forward error correction is enabled (`useinbandfec`).
    #[must_use]
    pub fn use_inband_fec(&self) -> Option<bool> {
        self.get("useinbandfec").map(|v| v == "1")
    }

    /// Whether discontinuous transmission is enabled (`usedtx`).
    #[must_use]
    pub fn use_dtx(&self) -> Option<bool> {
        self.get("usedtx").map(|v| v == "1")
    }

    /// Whether stereo decoding is allowed (`stereo`).
    #[must_use]
    pub fn stereo(&self) -> Option<bool> {
        self.get("stereo").map(|v| v == "1")
    }

    /// Maximum average bitrate in bits per second (`maxaveragebitrate`).
    #[must_use]
    pub fn max_average_bitrate(&self) -> Option<u32> {
        self.get("maxaveragebitrate").and_then(|v| v.parse().ok())
    }

    /// Set `maxaveragebitrate` in bits per second.
    pub fn set_max_average_bitrate(&mut self, bps: u32) {
        self.set("maxaveragebitrate", bps.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_meeting_friendly() {
        let fmtp = OpusFmtp::meeting_defaults();
        assert_eq!(fmtp.minptime(), Some(10));
        assert_eq!(fmtp.use_inband_fec(), Some(true));
        assert_eq!(fmtp.use_dtx(), Some(true));
        assert_eq!(fmtp.stereo(), Some(true));
        assert_eq!(
            fmtp.to_string(),
            "minptime=10;useinbandfec=1;usedtx=1;stereo=1"
        );
    }

    #[test]
    fn round_trip_with_bitrate() {
        let value = "minptime=20;useinbandfec=1;maxaveragebitrate=32000";
        let fmtp = OpusFmtp::parse(value);
        assert_eq!(fmtp.max_average_bitrate(), Some(32000));
        assert_eq!(fmtp.to_string(), value);
    }
}
