//! Parse an offer and re-emit it, verifying byte-for-byte stability.

use sdp::Sdp;

const SDP: &str = "\
v=0\r
o=alice 2890844526 2890842807 IN IP4 192.0.2.10\r
s=Example Session\r
i=A short session description\r
c=IN IP4 192.0.2.10\r
b=AS:256\r
t=0 0\r
a=group:BUNDLE 0 1\r
a=ice-options:trickle\r
m=audio 49170 UDP/TLS/RTP/SAVPF 111\r
a=mid:0\r
a=rtcp-mux\r
a=ice-ufrag:F7gI\r
a=ice-pwd:x9cml/YzichV2+XlhiMu8g\r
a=fingerprint:sha-256 12:AB:34:CD:EF:00:11:22:33:44:55:66:77:88:99:AA:BB:CC:DD:EE:FF:01:23:45:67:89:AB:CD:EF:10:32:54\r
a=setup:actpass\r
a=sendrecv\r
a=rtpmap:111 opus/48000/2\r
a=fmtp:111 minptime=10;useinbandfec=1\r
m=video 49174 UDP/TLS/RTP/SAVPF 96 97\r
a=mid:1\r
a=rtcp-mux\r
a=rtpmap:96 H264/90000\r
a=fmtp:96 level-asymmetry-allowed=1;packetization-mode=1;profile-level-id=42e01f\r
a=rtcp-fb:96 nack pli\r
a=rtpmap:97 rtx/90000\r
a=fmtp:97 apt=96\r
a=ssrc-group:FID 1111 2222\r
a=ssrc:1111 cname:stream-cname\r
";

fn main() -> sdp::Result<()> {
    let parsed = Sdp::parse(SDP)?;
    let reemitted = sdp::writer::write_sdp(&parsed)?;

    if reemitted == SDP {
        println!("round-trip stable: {} bytes", reemitted.len());
    } else {
        eprintln!("--- expected ---\n{SDP}\n--- got ---\n{reemitted}");
        return Err(sdp::SdpError::Profile {
            reason: "round-trip mismatch".into(),
        });
    }

    // Re-parsing the emitted text yields an equal structure.
    let twice = Sdp::parse(&reemitted)?;
    assert_eq!(parsed, twice, "structural idempotence");
    println!("structural idempotence verified");

    Ok(())
}
