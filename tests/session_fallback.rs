use sdp::{Offer, SetupRole};

const FINGERPRINT: &str = "19:E2:1C:3B:4B:9F:81:E6:B8:5C:F4:A5:A8:D8:73:04:BB:05:2F:70:9F:04:A9:0E:05:E9:26:33:E8:70:88:A2";

fn firefox_style_offer() -> String {
    format!(
        "\
v=0\r
o=mozilla...THIS_IS_SDPARTA-99.0 7157000000000000000 0 IN IP4 0.0.0.0\r
s=-\r
t=0 0\r
a=fingerprint:sha-256 {FINGERPRINT}\r
a=setup:active\r
a=ice-ufrag:sessUfrag\r
a=ice-pwd:sessionLevelPasswordValue\r
a=group:BUNDLE 0 1\r
a=ice-options:trickle\r
m=audio 9 UDP/TLS/RTP/SAVPF 109 9 0 8 101\r
c=IN IP4 0.0.0.0\r
a=sendrecv\r
a=ice-pwd:mediaLevelPasswordValue0\r
a=ice-ufrag:m0uf\r
a=mid:0\r
a=rtcp-mux\r
a=rtpmap:109 opus/48000/2\r
a=fmtp:109 maxplaybackrate=48000;stereo=1;useinbandfec=1\r
a=rtpmap:9 G722/8000/1\r
a=rtpmap:0 PCMU/8000\r
a=rtpmap:8 PCMA/8000\r
a=rtpmap:101 telephone-event/8000\r
a=fmtp:101 0-15\r
a=candidate:0 1 UDP 2122252543 192.0.2.4 61665 typ host\r
a=candidate:1 1 UDP 1686052863 203.0.113.9 61665 typ srflx raddr 192.0.2.4 rport 61665\r
m=video 0 UDP/TLS/RTP/SAVPF 120 121\r
c=IN IP4 0.0.0.0\r
a=bundle-only\r
a=recvonly\r
a=mid:1\r
a=rtcp-mux\r
a=rtpmap:120 VP8/90000\r
a=rtpmap:121 VP9/90000\r
"
    )
}

#[test]
fn session_level_fingerprint_and_setup_are_used_as_fallback() {
    let offer = Offer::parse(&firefox_style_offer()).expect("parse");
    let sdp = offer.sdp();
    let audio = sdp.media_by_mid("0").unwrap();

    assert!(audio.fingerprint().is_none());
    let fps = sdp.fingerprints_for(audio);
    assert_eq!(fps.len(), 1);
    assert_eq!(fps[0].algorithm, "sha-256");
    assert_eq!(fps[0].digest_hex(), FINGERPRINT);

    assert_eq!(audio.setup(), None);
    assert_eq!(sdp.setup_for(audio), Some(SetupRole::Active));
}

#[test]
fn ice_credentials_prefer_media_level_values() {
    let offer = Offer::parse(&firefox_style_offer()).expect("parse");
    let sdp = offer.sdp();
    let audio = sdp.media_by_mid("0").unwrap();
    let video = sdp.media_by_mid("1").unwrap();

    assert_eq!(audio.ice_ufrag(), Some("m0uf"));
    assert_eq!(audio.ice_pwd(), Some("mediaLevelPasswordValue0"));
    assert_eq!(sdp.ice_ufrag_for(audio), Some("m0uf"));
    assert_eq!(sdp.ice_pwd_for(audio), Some("mediaLevelPasswordValue0"));

    assert_eq!(video.ice_ufrag(), None);
    assert_eq!(sdp.ice_ufrag_for(video), Some("sessUfrag"));
    assert_eq!(sdp.ice_pwd_for(video), Some("sessionLevelPasswordValue"));
}

#[test]
fn candidates_are_listed_in_order() {
    let offer = Offer::parse(&firefox_style_offer()).expect("parse");
    let audio = offer.sdp().media_by_mid("0").unwrap();
    let candidates = audio.candidates();
    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].kind, "host");
    assert_eq!(candidates[1].kind, "srflx");
    assert_eq!(candidates[1].rel_port, Some(61665));
    assert!(offer
        .sdp()
        .media_by_mid("1")
        .unwrap()
        .candidates()
        .is_empty());
}

#[test]
fn bundle_only_section_is_not_rejected() {
    let offer = Offer::parse(&firefox_style_offer()).expect("parse");
    let video = offer.sdp().media_by_mid("1").unwrap();
    assert_eq!(video.port, 0);
    assert!(video.is_bundle_only());
    assert!(!video.is_rejected());
}
