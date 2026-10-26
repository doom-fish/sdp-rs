//! Parse a realistic browser-emitted WebRTC offer (audio + video + data
//! channel) and verify every attribute required for interop is recognised.

use sdp::attributes::Attribute;
use sdp::extmap::{URI_MID, URI_TWCC};
use sdp::writer::write_sdp;
use sdp::{MediaKind, Offer};

const BROWSER_OFFER: &str = "\
v=0\r
o=- 8675309001122334455 2 IN IP4 127.0.0.1\r
s=-\r
t=0 0\r
a=group:BUNDLE 0 1 2\r
a=extmap-allow-mixed\r
a=msid-semantic: WMS stream-id-0\r
m=audio 9 UDP/TLS/RTP/SAVPF 111 63\r
c=IN IP4 0.0.0.0\r
a=rtcp:9 IN IP4 0.0.0.0\r
a=candidate:842163049 1 udp 1677729535 198.51.100.7 51234 typ srflx raddr 0.0.0.0 rport 0 generation 0\r
a=ice-ufrag:EsAw\r
a=ice-pwd:bP+XJMM09aR8AiX1jdukzR6Y\r
a=ice-options:trickle\r
a=fingerprint:sha-256 19:E2:1C:3B:4B:9F:81:E6:B8:5C:F4:A5:A8:D8:73:04:BB:05:2F:70:9F:04:A9:0E:05:E9:26:33:E8:70:88:A2\r
a=setup:actpass\r
a=mid:0\r
a=extmap:1 urn:ietf:params:rtp-hdrext:sdes:mid\r
a=extmap:3 http://www.ietf.org/id/draft-holmer-rmcat-transport-wide-cc-extensions-01\r
a=sendrecv\r
a=msid:stream-id-0 audio-track-0\r
a=rtcp-mux\r
a=rtpmap:111 opus/48000/2\r
a=rtcp-fb:111 transport-cc\r
a=fmtp:111 minptime=10;useinbandfec=1\r
a=rtpmap:63 red/48000/2\r
a=ssrc:1111222233 cname:s9hF0xKnQ\r
a=ssrc:1111222233 msid:stream-id-0 audio-track-0\r
m=video 9 UDP/TLS/RTP/SAVPF 96 97\r
c=IN IP4 0.0.0.0\r
a=ice-ufrag:EsAw\r
a=ice-pwd:bP+XJMM09aR8AiX1jdukzR6Y\r
a=ice-options:trickle\r
a=fingerprint:sha-256 19:E2:1C:3B:4B:9F:81:E6:B8:5C:F4:A5:A8:D8:73:04:BB:05:2F:70:9F:04:A9:0E:05:E9:26:33:E8:70:88:A2\r
a=setup:actpass\r
a=mid:1\r
a=extmap:1 urn:ietf:params:rtp-hdrext:sdes:mid\r
a=extmap:3 http://www.ietf.org/id/draft-holmer-rmcat-transport-wide-cc-extensions-01\r
a=sendrecv\r
a=msid:stream-id-0 video-track-0\r
a=rtcp-mux\r
a=rtcp-rsize\r
a=rtpmap:96 H264/90000\r
a=rtcp-fb:96 nack\r
a=rtcp-fb:96 nack pli\r
a=rtcp-fb:96 transport-cc\r
a=fmtp:96 level-asymmetry-allowed=1;packetization-mode=1;profile-level-id=42e01f\r
a=rtpmap:97 rtx/90000\r
a=fmtp:97 apt=96\r
a=ssrc-group:FID 3333444455 3333444456\r
a=ssrc:3333444455 cname:s9hF0xKnQ\r
a=ssrc:3333444456 cname:s9hF0xKnQ\r
m=application 9 UDP/DTLS/SCTP webrtc-datachannel\r
c=IN IP4 0.0.0.0\r
a=ice-ufrag:EsAw\r
a=ice-pwd:bP+XJMM09aR8AiX1jdukzR6Y\r
a=ice-options:trickle\r
a=fingerprint:sha-256 19:E2:1C:3B:4B:9F:81:E6:B8:5C:F4:A5:A8:D8:73:04:BB:05:2F:70:9F:04:A9:0E:05:E9:26:33:E8:70:88:A2\r
a=setup:actpass\r
a=mid:2\r
a=sctp-port:5000\r
a=max-message-size:262144\r
";

#[test]
fn parses_and_round_trips() {
    let offer = Offer::parse(BROWSER_OFFER).expect("parse browser offer");
    assert_eq!(
        write_sdp(offer.sdp()).unwrap(),
        BROWSER_OFFER,
        "lossless round-trip"
    );
}

#[test]
fn bundle_groups_all_three_sections() {
    let offer = Offer::parse(BROWSER_OFFER).unwrap();
    let group = offer.sdp().bundle_group().expect("BUNDLE group");
    assert_eq!(group.mids, vec!["0", "1", "2"]);
    assert_eq!(offer.sdp().media.len(), 3);
}

#[test]
fn audio_section_has_required_webrtc_attrs() {
    let offer = Offer::parse(BROWSER_OFFER).unwrap();
    let audio = offer.sdp().media_by_mid("0").expect("audio mid=0");

    assert_eq!(audio.kind, MediaKind::Audio);
    assert!(audio.has_rtcp_mux(), "rtcp-mux required by browsers");
    assert!(audio.fingerprint().is_some(), "DTLS fingerprint required");
    assert!(audio.setup().is_some(), "DTLS setup role required");
    assert!(audio.direction().is_some(), "direction required");

    let has = |pred: fn(&Attribute) -> bool| audio.attributes.iter().any(pred);
    assert!(has(|a| matches!(a, Attribute::IceUfrag(_))), "ice-ufrag");
    assert!(has(|a| matches!(a, Attribute::IcePwd(_))), "ice-pwd");
    assert!(
        has(|a| matches!(a, Attribute::IceOptions(o) if o.contains("trickle"))),
        "ice-options:trickle"
    );
    assert!(has(|a| matches!(a, Attribute::Candidate(_))), "candidate");
    assert!(has(|a| matches!(a, Attribute::Msid(_))), "msid");
    assert!(has(|a| matches!(a, Attribute::Ssrc(_))), "ssrc");

    // MID and TWCC header extensions must both be present.
    let extmaps: Vec<&str> = audio
        .attributes
        .iter()
        .filter_map(|a| match a {
            Attribute::Extmap(e) => Some(e.uri.as_str()),
            _ => None,
        })
        .collect();
    assert!(extmaps.contains(&URI_MID), "MID header extension");
    assert!(extmaps.contains(&URI_TWCC), "TWCC header extension");
}

#[test]
fn candidate_is_fully_parsed() {
    let offer = Offer::parse(BROWSER_OFFER).unwrap();
    let audio = offer.sdp().media_by_mid("0").unwrap();
    let candidate = audio
        .attributes
        .iter()
        .find_map(|a| match a {
            Attribute::Candidate(c) => Some(c),
            _ => None,
        })
        .expect("candidate present");
    assert_eq!(candidate.component, 1);
    assert_eq!(candidate.transport, "udp");
    assert_eq!(candidate.kind, "srflx");
    assert_eq!(candidate.address, "198.51.100.7");
    assert_eq!(candidate.port, 51234);
    assert_eq!(candidate.rel_port, Some(0));
}

#[test]
fn video_section_has_rtx_and_simulcast_ssrc_group() {
    let offer = Offer::parse(BROWSER_OFFER).unwrap();
    let video = offer.sdp().media_by_mid("1").expect("video mid=1");

    // H.264 fmtp parses to constrained-baseline defaults.
    let fmtp = video.fmtp_for(96).expect("fmtp for pt 96");
    let h264 = sdp::H264Fmtp::parse(&fmtp.params);
    assert_eq!(h264.profile_level_id(), Some(sdp::h264::CONSTRAINED_BASELINE_3_1));
    assert_eq!(h264.packetization_mode(), Some(1));

    let has_fid = video.attributes.iter().any(|a| {
        matches!(a, Attribute::SsrcGroup(g) if g.semantics == "FID" && g.ssrcs.len() == 2)
    });
    assert!(has_fid, "FID ssrc-group for RTX");
}

#[test]
fn data_channel_section_is_present_and_application_kind() {
    let offer = Offer::parse(BROWSER_OFFER).unwrap();
    let data = offer.sdp().media_by_mid("2").expect("data mid=2");
    assert_eq!(data.kind, MediaKind::Application);
    assert_eq!(data.proto, "UDP/DTLS/SCTP");
    assert_eq!(data.formats, vec!["webrtc-datachannel"]);
    assert!(!data.is_rejected(), "offer's data channel is active (port 9)");
}
