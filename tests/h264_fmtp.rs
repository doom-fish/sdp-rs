//! H.264 fmtp parameter round-trip and typed accessor tests (RFC 6184 §8.1).

use sdp::h264::CONSTRAINED_BASELINE_3_1;
use sdp::H264Fmtp;

#[test]
fn meeting_defaults_are_constrained_baseline() {
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
fn parse_preserves_order_and_unknown_params() {
    let value = "packetization-mode=1;profile-level-id=640c1f;max-fs=3600;max-mbps=108000";
    let fmtp = H264Fmtp::parse(value);
    assert_eq!(fmtp.profile_level_id(), Some(0x0064_0c1f));
    assert_eq!(fmtp.packetization_mode(), Some(1));
    assert_eq!(fmtp.get("max-fs"), Some("3600"));
    assert_eq!(fmtp.get("max-mbps"), Some("108000"));
    assert_eq!(fmtp.to_string(), value, "round-trip is lossless");
}

#[test]
fn profile_level_id_is_24_bit_hex() {
    let mut fmtp = H264Fmtp::new();
    fmtp.set_profile_level_id(0x0042_e01f);
    assert_eq!(fmtp.get("profile-level-id"), Some("42e01f"));

    // High byte beyond 24 bits is masked off.
    fmtp.set_profile_level_id(0xff42_e01f);
    assert_eq!(fmtp.get("profile-level-id"), Some("42e01f"));

    // Leading-zero profiles render with the full six digits.
    fmtp.set_profile_level_id(0x0000_420a);
    assert_eq!(fmtp.get("profile-level-id"), Some("00420a"));
    assert_eq!(fmtp.profile_level_id(), Some(0x0000_420a));
}

#[test]
fn packetization_modes_parse() {
    for mode in 0..=2 {
        let fmtp = H264Fmtp::parse(&format!("packetization-mode={mode}"));
        assert_eq!(fmtp.packetization_mode(), Some(mode));
    }
}

#[test]
fn whitespace_around_separators_is_tolerated() {
    let fmtp = H264Fmtp::parse("packetization-mode=1 ; profile-level-id=42e01f");
    assert_eq!(fmtp.packetization_mode(), Some(1));
    assert_eq!(fmtp.profile_level_id(), Some(CONSTRAINED_BASELINE_3_1));
}

#[test]
fn missing_params_yield_none() {
    let fmtp = H264Fmtp::parse("profile-level-id=42e01f");
    assert!(fmtp.packetization_mode().is_none());
    assert!(fmtp.level_asymmetry_allowed().is_none());
}
