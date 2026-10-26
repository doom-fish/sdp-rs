//! Opus fmtp parameter round-trip and typed accessor tests (RFC 7587 §6.1).

use sdp::OpusFmtp;

#[test]
fn meeting_defaults_are_sensible() {
    let fmtp = OpusFmtp::meeting_defaults();
    assert_eq!(fmtp.minptime(), Some(10));
    assert_eq!(fmtp.use_inband_fec(), Some(true));
    assert_eq!(fmtp.use_dtx(), Some(true));
    assert_eq!(fmtp.stereo(), Some(true));
    assert_eq!(fmtp.to_string(), "minptime=10;useinbandfec=1;usedtx=1;stereo=1");
}

#[test]
fn parse_round_trips_with_bitrate() {
    let value = "minptime=20;useinbandfec=1;usedtx=0;stereo=0;maxaveragebitrate=24000";
    let fmtp = OpusFmtp::parse(value);
    assert_eq!(fmtp.minptime(), Some(20));
    assert_eq!(fmtp.use_inband_fec(), Some(true));
    assert_eq!(fmtp.use_dtx(), Some(false));
    assert_eq!(fmtp.stereo(), Some(false));
    assert_eq!(fmtp.max_average_bitrate(), Some(24_000));
    assert_eq!(fmtp.to_string(), value, "round-trip is lossless");
}

#[test]
fn set_max_average_bitrate_appends() {
    let mut fmtp = OpusFmtp::meeting_defaults();
    fmtp.set_max_average_bitrate(32_000);
    assert_eq!(fmtp.max_average_bitrate(), Some(32_000));
    assert_eq!(
        fmtp.to_string(),
        "minptime=10;useinbandfec=1;usedtx=1;stereo=1;maxaveragebitrate=32000"
    );
}

#[test]
fn replacing_a_param_preserves_position() {
    let mut fmtp = OpusFmtp::parse("minptime=10;useinbandfec=1;stereo=0");
    fmtp.set("stereo", "1");
    assert_eq!(fmtp.stereo(), Some(true));
    assert_eq!(fmtp.to_string(), "minptime=10;useinbandfec=1;stereo=1");
}

#[test]
fn unknown_params_are_preserved() {
    let value = "minptime=10;sprop-maxcapturerate=16000;cbr=1";
    let fmtp = OpusFmtp::parse(value);
    assert_eq!(fmtp.get("sprop-maxcapturerate"), Some("16000"));
    assert_eq!(fmtp.get("cbr"), Some("1"));
    assert_eq!(fmtp.to_string(), value);
}

#[test]
fn missing_params_yield_none() {
    let fmtp = OpusFmtp::parse("useinbandfec=1");
    assert!(fmtp.minptime().is_none());
    assert!(fmtp.max_average_bitrate().is_none());
}
