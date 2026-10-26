//! Round-trip the canonical SDP examples from RFC 8866 §5.

use sdp::writer::write_sdp;
use sdp::{MediaKind, Sdp, SdpError};

/// The example from RFC 8866 §5 (adapted to CRLF line endings, which the RFC
/// mandates on the wire).
const RFC8866_EXAMPLE: &str = "\
v=0\r
o=jdoe 3724394400 3724394405 IN IP4 198.51.100.1\r
s=Call to John Smith\r
i=SDP Offer #1\r
u=http://www.example.com/seminars/sdp.pdf\r
e=j.doe@example.com (Jane Doe)\r
p=+1 617 555-6011\r
c=IN IP4 198.51.100.1\r
t=0 0\r
m=audio 49170 RTP/AVP 0\r
m=video 51372 RTP/AVP 99\r
a=rtpmap:99 h263-1998/90000\r
";

#[test]
fn rfc8866_example_round_trips() {
    let sdp = Sdp::parse(RFC8866_EXAMPLE).expect("parse RFC 8866 example");
    assert_eq!(sdp.version, 0);
    assert_eq!(sdp.origin.username, "jdoe");
    assert_eq!(sdp.origin.unicast_address, "198.51.100.1");
    assert_eq!(sdp.session_name, "Call to John Smith");
    assert_eq!(sdp.information.as_deref(), Some("SDP Offer #1"));
    assert_eq!(
        sdp.uri.as_deref(),
        Some("http://www.example.com/seminars/sdp.pdf")
    );
    assert_eq!(sdp.emails, vec!["j.doe@example.com (Jane Doe)"]);
    assert_eq!(sdp.phones, vec!["+1 617 555-6011"]);
    assert_eq!(sdp.media.len(), 2);
    assert_eq!(sdp.media[0].kind, MediaKind::Audio);
    assert_eq!(sdp.media[0].port, 49170);
    assert_eq!(sdp.media[1].kind, MediaKind::Video);
    assert_eq!(sdp.media[1].formats, vec!["99"]);

    assert_eq!(write_sdp(&sdp).unwrap(), RFC8866_EXAMPLE);
}

#[test]
fn minimal_session_round_trips() {
    let text = "\
v=0\r
o=- 0 0 IN IP4 127.0.0.1\r
s=-\r
t=0 0\r
";
    let sdp = Sdp::parse(text).expect("parse minimal");
    assert_eq!(write_sdp(&sdp).unwrap(), text);
}

#[test]
fn timing_with_repeats_round_trips() {
    let text = "\
v=0\r
o=- 0 0 IN IP4 127.0.0.1\r
s=-\r
t=3034423619 3042462419\r
r=604800 3600 0 90000\r
z=2882844526 -1h 2898848070 0\r
";
    let sdp = Sdp::parse(text).expect("parse timing");
    assert_eq!(sdp.timing.len(), 1);
    assert_eq!(sdp.timing[0].start, 3_034_423_619);
    assert_eq!(sdp.timing[0].repeats, vec!["604800 3600 0 90000"]);
    assert_eq!(sdp.time_zones.as_deref(), Some("2882844526 -1h 2898848070 0"));
    assert_eq!(write_sdp(&sdp).unwrap(), text);
}

#[test]
fn media_level_connection_and_bandwidth() {
    let text = "\
v=0\r
o=- 0 0 IN IP4 127.0.0.1\r
s=-\r
t=0 0\r
m=audio 49170 RTP/AVP 0\r
i=Main audio\r
c=IN IP4 233.252.0.1/127\r
b=AS:64\r
a=ptime:20\r
";
    let sdp = Sdp::parse(text).expect("parse media-level lines");
    let audio = &sdp.media[0];
    assert_eq!(audio.title.as_deref(), Some("Main audio"));
    assert_eq!(
        audio.connection.as_ref().map(|c| c.address.as_str()),
        Some("233.252.0.1/127")
    );
    assert_eq!(audio.bandwidths[0].bwtype, "AS");
    assert_eq!(audio.bandwidths[0].bandwidth, 64);
    assert_eq!(write_sdp(&sdp).unwrap(), text);
}

#[test]
fn version_must_be_zero() {
    for version in ["1", "2", "4294967295"] {
        let text = format!("v={version}\r\no=- 0 0 IN IP4 127.0.0.1\r\ns=-\r\nt=0 0\r\n");
        assert!(matches!(
            Sdp::parse(&text),
            Err(SdpError::InvalidValue { field: "v", .. })
        ));
    }

    let mut sdp = Sdp::parse("v=0\r\no=- 0 0 IN IP4 127.0.0.1\r\ns=-\r\nt=0 0\r\n").unwrap();
    sdp.version = 1;
    assert!(matches!(
        write_sdp(&sdp),
        Err(SdpError::InvalidValue { field: "v", .. })
    ));
}
