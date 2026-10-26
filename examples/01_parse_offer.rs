//! Parse a sample WebRTC offer and print a structured summary.

use sdp::{Attribute, Offer};

const OFFER: &str = "\
v=0\r
o=- 4611731400430051336 2 IN IP4 127.0.0.1\r
s=-\r
t=0 0\r
a=group:BUNDLE 0 1\r
a=ice-options:trickle\r
m=audio 9 UDP/TLS/RTP/SAVPF 111\r
c=IN IP4 0.0.0.0\r
a=mid:0\r
a=sendrecv\r
a=rtcp-mux\r
a=ice-ufrag:F7gI\r
a=ice-pwd:x9cml/YzichV2+XlhiMu8g\r
a=fingerprint:sha-256 12:AB:34:CD:EF:00:11:22:33:44:55:66:77:88:99:AA:BB:CC:DD:EE:FF:01:23:45:67:89:AB:CD:EF:10:32:54\r
a=setup:actpass\r
a=rtpmap:111 opus/48000/2\r
a=fmtp:111 minptime=10;useinbandfec=1\r
m=video 9 UDP/TLS/RTP/SAVPF 96\r
c=IN IP4 0.0.0.0\r
a=mid:1\r
a=sendrecv\r
a=rtcp-mux\r
a=ice-ufrag:F7gI\r
a=ice-pwd:x9cml/YzichV2+XlhiMu8g\r
a=fingerprint:sha-256 12:AB:34:CD:EF:00:11:22:33:44:55:66:77:88:99:AA:BB:CC:DD:EE:FF:01:23:45:67:89:AB:CD:EF:10:32:54\r
a=setup:actpass\r
a=rtpmap:96 VP8/90000\r
a=rtcp-fb:96 nack pli\r
";

fn main() -> sdp::Result<()> {
    let offer = Offer::parse(OFFER)?;
    let sdp = offer.sdp();

    println!("session: {:?}", sdp.session_name);
    if let Some(group) = sdp.bundle_group() {
        println!("BUNDLE mids: {:?}", group.mids);
    }

    for m in &sdp.media {
        println!(
            "\nm={} port={} proto={} formats={:?}",
            m.kind, m.port, m.proto, m.formats
        );
        println!("  mid={:?} direction={:?}", m.mid(), m.direction());
        println!("  rtcp-mux={} setup={:?}", m.has_rtcp_mux(), m.setup());
        if let Some(fp) = m.fingerprint() {
            println!("  fingerprint alg={} ({} bytes)", fp.algorithm, fp.digest.len());
        }
        for map in m.rtpmaps() {
            println!(
                "  codec pt={} {}/{}{}",
                map.payload,
                map.encoding,
                map.clock_rate,
                map.channels.map(|c| format!("/{c}")).unwrap_or_default()
            );
        }
        for attr in &m.attributes {
            if let Attribute::RtcpFb(fb) = attr {
                println!("  rtcp-fb pt={} {}", fb.format, fb.feedback);
            }
        }
    }

    Ok(())
}
