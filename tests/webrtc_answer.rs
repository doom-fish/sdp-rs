//! Build an answer that accepts audio + video and rejects the data channel,
//! then verify it is internally consistent with the offer.

use sdp::attributes::Attribute;
use sdp::extmap::{URI_MID, URI_TWCC};
use sdp::writer::write_sdp;
use sdp::{Fingerprint, H264Fmtp, LocalIdentity, MediaKind, Offer, OpusFmtp, RtcpFb, RtpMap};

const OFFER: &str = "\
v=0\r
o=- 8675309001122334455 2 IN IP4 127.0.0.1\r
s=-\r
t=0 0\r
a=group:BUNDLE 0 1 2\r
m=audio 9 UDP/TLS/RTP/SAVPF 111\r
c=IN IP4 0.0.0.0\r
a=ice-ufrag:EsAw\r
a=ice-pwd:bP+XJMM09aR8AiX1jdukzR6Y\r
a=ice-options:trickle\r
a=fingerprint:sha-256 19:E2:1C:3B:4B:9F:81:E6:B8:5C:F4:A5:A8:D8:73:04:BB:05:2F:70:9F:04:A9:0E:05:E9:26:33:E8:70:88:A2\r
a=setup:actpass\r
a=mid:0\r
a=extmap:1 urn:ietf:params:rtp-hdrext:sdes:mid\r
a=extmap:3 http://www.ietf.org/id/draft-holmer-rmcat-transport-wide-cc-extensions-01\r
a=sendrecv\r
a=rtcp-mux\r
a=rtpmap:111 opus/48000/2\r
m=video 9 UDP/TLS/RTP/SAVPF 96\r
c=IN IP4 0.0.0.0\r
a=ice-ufrag:EsAw\r
a=ice-pwd:bP+XJMM09aR8AiX1jdukzR6Y\r
a=ice-options:trickle\r
a=fingerprint:sha-256 19:E2:1C:3B:4B:9F:81:E6:B8:5C:F4:A5:A8:D8:73:04:BB:05:2F:70:9F:04:A9:0E:05:E9:26:33:E8:70:88:A2\r
a=setup:actpass\r
a=mid:1\r
a=extmap:1 urn:ietf:params:rtp-hdrext:sdes:mid\r
a=extmap:3 http://www.ietf.org/id/draft-holmer-rmcat-transport-wide-cc-extensions-01\r
a=sendonly\r
a=rtcp-mux\r
a=rtpmap:96 H264/90000\r
m=application 9 UDP/DTLS/SCTP webrtc-datachannel\r
c=IN IP4 0.0.0.0\r
a=ice-ufrag:EsAw\r
a=ice-pwd:bP+XJMM09aR8AiX1jdukzR6Y\r
a=setup:actpass\r
a=mid:2\r
a=sctp-port:5000\r
";

fn build_answer() -> sdp::Answer {
    let offer = Offer::parse(OFFER).expect("parse offer");
    let identity = LocalIdentity::new(
        "9aBc",
        "answerPasswordMaterial0123456",
        Fingerprint::parse("sha-256 99:88:77:66:55:44:33:22:11:00:FF:EE:DD:CC:BB:AA:BB:CC:DD:EE:FF:01:23:45:67:89:AB:CD:EF:10:32:54").unwrap(),
    );
    Offer::answer()
        .identity(identity)
        .accept_audio(vec![
            Attribute::RtpMap(RtpMap {
                payload: 111,
                encoding: "opus".into(),
                clock_rate: 48_000,
                channels: Some(2),
            }),
            Attribute::Fmtp(sdp::Fmtp {
                format: "111".into(),
                params: OpusFmtp::meeting_defaults().to_string(),
            }),
        ])
        .accept_video(vec![
            Attribute::RtpMap(RtpMap {
                payload: 96,
                encoding: "H264".into(),
                clock_rate: 90_000,
                channels: None,
            }),
            Attribute::Fmtp(sdp::Fmtp {
                format: "96".into(),
                params: H264Fmtp::meeting_defaults().to_string(),
            }),
            Attribute::RtcpFb(RtcpFb {
                format: "96".into(),
                feedback: "nack pli".into(),
            }),
        ])
        .reject_data()
        .build(&offer)
        .expect("build answer")
}

#[test]
fn answer_mirrors_offer_media_order() {
    let offer = Offer::parse(OFFER).unwrap();
    let answer = build_answer();
    assert_eq!(answer.sdp().media.len(), offer.sdp().media.len());

    for (o, a) in offer.sdp().media.iter().zip(&answer.sdp().media) {
        // Media kind, protocol, and mid must match the offer 1:1.
        assert_eq!(o.kind, a.kind);
        assert_eq!(o.proto, a.proto);
        assert_eq!(o.mid(), a.mid(), "mid must match for BUNDLE");
    }
}

#[test]
fn answer_round_trips_losslessly() {
    let answer = build_answer();
    let text = write_sdp(answer.sdp()).expect("write answer");
    let reparsed = sdp::Answer::parse(&text).expect("reparse answer");
    assert_eq!(write_sdp(reparsed.sdp()).unwrap(), text);
}

#[test]
fn data_channel_is_rejected_with_port_zero() {
    let answer = build_answer();
    let data = answer
        .sdp()
        .media_by_mid("2")
        .expect("data section preserved");
    assert_eq!(data.kind, MediaKind::Application);
    assert!(data.is_rejected(), "RFC 8843 §7.2: rejected => port 0");
    assert_eq!(data.port, 0);
    // Protocol + format must be preserved so the peer can match the rejection.
    assert_eq!(data.proto, "UDP/DTLS/SCTP");
    assert_eq!(data.formats, vec!["webrtc-datachannel"]);
}

#[test]
fn accepted_sections_carry_keying_material() {
    let answer = build_answer();
    for mid in ["0", "1"] {
        let m = answer.sdp().media_by_mid(mid).unwrap();
        assert!(!m.is_rejected(), "accepted section keeps a non-zero port");
        assert!(m.has_rtcp_mux(), "rtcp-mux mirrored");
        assert!(m.fingerprint().is_some(), "fingerprint present");
        assert!(
            m.attributes
                .iter()
                .any(|a| matches!(a, Attribute::IceUfrag(_))),
            "ice-ufrag present"
        );
        // Answerer picks `active` when the offerer offered `actpass`.
        assert_eq!(m.setup(), Some(sdp::SetupRole::Active));
    }
}

#[test]
fn answer_reverses_directionality() {
    let answer = build_answer();
    // Offer video was sendonly => answer must be recvonly.
    let video = answer.sdp().media_by_mid("1").unwrap();
    assert_eq!(video.direction(), Some(sdp::Direction::RecvOnly));
    // Offer audio was sendrecv => answer stays sendrecv.
    let audio = answer.sdp().media_by_mid("0").unwrap();
    assert_eq!(audio.direction(), Some(sdp::Direction::SendRecv));
}

#[test]
fn answer_preserves_negotiated_header_extensions() {
    let answer = build_answer();
    let audio = answer.sdp().media_by_mid("0").unwrap();
    let uris: Vec<&str> = audio
        .attributes
        .iter()
        .filter_map(|a| match a {
            Attribute::Extmap(e) => Some(e.uri.as_str()),
            _ => None,
        })
        .collect();
    assert!(uris.contains(&URI_MID), "MID extension kept");
    assert!(uris.contains(&URI_TWCC), "TWCC extension kept");
}

#[test]
fn answer_has_bundle_for_accepted_sections() {
    let answer = build_answer();
    let group = answer.sdp().bundle_group().expect("answer BUNDLE group");
    // Only accepted sections join the bundle.
    assert_eq!(group.mids, vec!["0", "1"]);
}
