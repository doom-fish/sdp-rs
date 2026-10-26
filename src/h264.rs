//! Typed helpers for the H.264 `a=fmtp:` parameter string (RFC 6184 §8.1).
//!
//! The format-specific parameters are a `;`-separated list of `key=value`
//! pairs. This module provides typed access to the three parameters that
//! matter for interoperability while preserving the original ordering and any
//! unknown parameters so the line round-trips losslessly.

use crate::fmtp::FmtpParams;

/// The constrained-baseline `profile-level-id` that browsers consistently
/// accept (`profile_idc`=0x42, constraint flags=0xe0, `level_idc`=0x1f → 3.1).
pub const CONSTRAINED_BASELINE_3_1: u32 = 0x0042_e01f;

#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct H264;

/// Ordered key/value view over an H.264 fmtp parameter string.
pub type H264Fmtp = FmtpParams<H264>;

impl H264Fmtp {
    /// The defaults a meeting client should offer: constrained-baseline level
    /// 3.1, single-NAL + non-interleaved packetization, and asymmetric levels
    /// allowed. These are the values modern browsers negotiate reliably.
    #[must_use]
    pub fn meeting_defaults() -> Self {
        let mut fmtp = Self::new();
        fmtp.set("level-asymmetry-allowed", "1");
        fmtp.set("packetization-mode", "1");
        fmtp.set_profile_level_id(CONSTRAINED_BASELINE_3_1);
        fmtp
    }

    /// The `profile-level-id` as a 24-bit integer, if present and valid hex.
    #[must_use]
    pub fn profile_level_id(&self) -> Option<u32> {
        self.get("profile-level-id")
            .and_then(|v| u32::from_str_radix(v, 16).ok())
    }

    /// Set `profile-level-id` from a 24-bit integer, rendered as 6 hex digits.
    pub fn set_profile_level_id(&mut self, id: u32) {
        self.set("profile-level-id", format!("{:06x}", id & 0x00ff_ffff));
    }

    /// The `packetization-mode` (0, 1, or 2), if present and valid.
    #[must_use]
    pub fn packetization_mode(&self) -> Option<u8> {
        self.get("packetization-mode").and_then(|v| v.parse().ok())
    }

    /// The `level-asymmetry-allowed` flag, if present.
    #[must_use]
    pub fn level_asymmetry_allowed(&self) -> Option<bool> {
        self.get("level-asymmetry-allowed").map(|v| v == "1")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_browser_expectations() {
        let fmtp = H264Fmtp::meeting_defaults();
        assert_eq!(fmtp.profile_level_id(), Some(CONSTRAINED_BASELINE_3_1));
        assert_eq!(fmtp.packetization_mode(), Some(1));
        assert_eq!(fmtp.level_asymmetry_allowed(), Some(true));
        assert_eq!(
            fmtp.to_string(),
            "level-asymmetry-allowed=1;packetization-mode=1;profile-level-id=42e01f"
        );
    }

    #[test]
    fn round_trip_preserves_order_and_unknowns() {
        let value = "packetization-mode=1;profile-level-id=640c1f;max-fs=3600";
        let fmtp = H264Fmtp::parse(value);
        assert_eq!(fmtp.profile_level_id(), Some(0x0064_0c1f));
        assert_eq!(fmtp.get("max-fs"), Some("3600"));
        assert_eq!(fmtp.to_string(), value);
    }
}
