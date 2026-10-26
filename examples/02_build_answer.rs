//! Build an answer for a sample offer: accept audio + video, reject the data
//! channel, and print the resulting SDP.

use sdp::{
    Attribute, Fingerprint, H264Fmtp, LocalIdentity, Offer, OpusFmtp, RtcpFb, RtpMap,
};

const OFFER: &str = "\
v=0\r
o=- 4611731400430051336 2 IN IP4 127.0.0.1\r
s=-\r
t=0 0\r
a=group:BUNDLE 0 1 2\r
a=ice-options:trickle\r
m=audio 9 UDP/TLS/RTP/SAVPF 111\r
c=IN IP4 0.0.0.0\r
a=mid:0\r
a=sendrecv\r
a=rtcp-mux\r
a=ice-ufrag:F7gI\r
a=ice-pwd:x9cml/YzichV2+XlhiMu8g\r
a=fingerprint:sha-256 AA:BB:CC:DD:EE:FF:00:11:22:33:44:55:66:77:88:99:BB:CC:DD:EE:FF:01:23:45:67:89:AB:CD:EF:10:32:54\r
a=setup:actpass\r
a=extmap:4 urn:ietf:params:rtp-hdrext:sdes:mid\r
a=rtpmap:111 opus/48000/2\r
m=video 9 UDP/TLS/RTP/SAVPF 96\r
c=IN IP4 0.0.0.0\r
a=mid:1\r
a=sendrecv\r
a=rtcp-mux\r
a=ice-ufrag:F7gI\r
a=ice-pwd:x9cml/YzichV2+XlhiMu8g\r
a=fingerprint:sha-256 AA:BB:CC:DD:EE:FF:00:11:22:33:44:55:66:77:88:99:BB:CC:DD:EE:FF:01:23:45:67:89:AB:CD:EF:10:32:54\r
a=setup:actpass\r
a=rtpmap:96 H264/90000\r
m=application 9 UDP/DTLS/SCTP webrtc-datachannel\r
c=IN IP4 0.0.0.0\r
a=mid:2\r
a=sctp-port:5000\r
";

fn main() -> sdp::Result<()> {
    let offer = Offer::parse(OFFER)?;

    let identity = LocalIdentity::new(
        "9aBc",
        "answerPasswordMaterialHere01",
        Fingerprint::parse(
            "sha-256 99:88:77:66:55:44:33:22:11:00:FF:EE:DD:CC:BB:AA:BB:CC:DD:EE:FF:01:23:45:67:89:AB:CD:EF:10:32:54",
        )?,
    );

    let opus = OpusFmtp::meeting_defaults();
    let h264 = H264Fmtp::meeting_defaults();

    let answer = Offer::answer()
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
                params: opus.to_string(),
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
                params: h264.to_string(),
            }),
            Attribute::RtcpFb(RtcpFb {
                format: "96".into(),
                feedback: "nack pli".into(),
            }),
        ])
        .reject_data()
        .build(&offer)?;

    println!("{}", sdp::writer::write_sdp(answer.sdp())?);

    let data = answer
        .sdp()
        .media
        .iter()
        .find(|m| matches!(m.kind, sdp::MediaKind::Application))
        .expect("application section present");
    assert!(data.is_rejected(), "data channel must be rejected (port 0)");
    println!("; data channel rejected with port {}", data.port);

    Ok(())
}
