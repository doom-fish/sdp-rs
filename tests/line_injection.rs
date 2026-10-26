use sdp::writer::write_sdp;
use sdp::{
    Attribute, Candidate, Fingerprint, Fmtp, LocalIdentity, MediaDescription, MediaKind, Offer,
    RtpMap, Sdp, SdpError,
};

const BASE: &str = "\
v=0\r
o=- 1 1 IN IP4 127.0.0.1\r
s=-\r
t=0 0\r
m=audio 9 UDP/TLS/RTP/SAVPF 111\r
a=mid:0\r
a=rtpmap:111 opus/48000/2\r
";

const INJECTED: &str = "0\r\na=fingerprint:sha-256 00:11";

fn base() -> Sdp {
    Sdp::parse(BASE).expect("base document")
}

fn assert_rejected(sdp: &Sdp, line_type: char) {
    match write_sdp(sdp) {
        Err(SdpError::ForbiddenCharacter {
            line_type: got,
            character,
        }) => {
            assert_eq!(got, line_type);
            assert!(matches!(character, '\r' | '\n' | '\0'));
        }
        other => panic!("expected a forbidden-character error, got {other:?}"),
    }
}

#[test]
fn base_document_writes() {
    assert_eq!(write_sdp(&base()).unwrap(), BASE);
}

#[test]
fn session_level_values_are_checked() {
    let mut sdp = base();
    sdp.session_name = "call\r\na=fingerprint:sha-256 00".into();
    assert_rejected(&sdp, 's');

    let mut sdp = base();
    sdp.origin.username = "alice\n".into();
    assert_rejected(&sdp, 'o');

    let mut sdp = base();
    sdp.information = Some("info\0".into());
    assert_rejected(&sdp, 'i');

    let mut sdp = base();
    sdp.timing[0].repeats.push("604800\r\nk=clear:x".into());
    assert_rejected(&sdp, 'r');

    let mut sdp = base();
    sdp.attributes.push(Attribute::Other {
        name: "x-vendor".into(),
        value: Some(INJECTED.into()),
    });
    assert_rejected(&sdp, 'a');
}

#[test]
fn media_level_values_are_checked() {
    let cases: Vec<Attribute> = vec![
        Attribute::Mid(INJECTED.into()),
        Attribute::Msid(INJECTED.into()),
        Attribute::IceUfrag(INJECTED.into()),
        Attribute::IcePwd(INJECTED.into()),
        Attribute::IceOptions(INJECTED.into()),
        Attribute::Fmtp(Fmtp {
            format: "111".into(),
            params: format!("minptime=10\r\n{INJECTED}"),
        }),
        Attribute::RtpMap(RtpMap {
            payload: 111,
            encoding: "opus\r\n".into(),
            clock_rate: 48_000,
            channels: None,
        }),
        Attribute::Fingerprint(Fingerprint::new("sha-256\n", vec![0; 32])),
        Attribute::Other {
            name: "x\r\na=setup:active".into(),
            value: None,
        },
    ];
    for attr in cases {
        let mut sdp = base();
        sdp.media[0].attributes.push(attr);
        assert_rejected(&sdp, 'a');
    }

    let mut sdp = base();
    sdp.media[0].kind = MediaKind::Other("audio\r\n".into());
    assert_rejected(&sdp, 'm');

    let mut sdp = base();
    sdp.media[0].title = Some("t\r".into());
    assert_rejected(&sdp, 'i');

    let mut candidate =
        Candidate::parse("1 1 udp 2122260223 192.0.2.1 50000 typ host").expect("candidate");
    candidate.foundation = "1\r\na=candidate:evil".into();
    let mut sdp = base();
    sdp.media[0]
        .attributes
        .push(Attribute::Candidate(candidate));
    assert_rejected(&sdp, 'a');

    let mut media = MediaDescription::new(MediaKind::Video, 9, "RTP/AVP\r\nb=AS:1");
    media.formats.push("96".into());
    let mut sdp = base();
    sdp.media.push(media);
    assert_rejected(&sdp, 'm');
}

#[test]
fn parser_rejects_mid_line_cr_and_nul() {
    let cr = BASE.replace("a=mid:0\r\n", "a=mid:0\ra=fingerprint:sha-256 00:11\r\n");
    assert!(matches!(
        Sdp::parse(&cr),
        Err(SdpError::MalformedLine { line: 6, .. })
    ));

    let nul = BASE.replace("s=-\r\n", "s=a\0b\r\n");
    assert!(matches!(
        Sdp::parse(&nul),
        Err(SdpError::MalformedLine { line: 3, .. })
    ));

    let double_cr = BASE.replace("a=mid:0\r\n", "a=mid:0\r\r\n");
    assert!(Sdp::parse(&double_cr).is_err());
}

#[test]
fn injected_local_identity_cannot_reach_the_answer_text() {
    let offer = Offer::parse(
        "\
v=0\r
o=- 1 1 IN IP4 127.0.0.1\r
s=-\r
t=0 0\r
a=group:BUNDLE 0\r
a=fingerprint:sha-256 19:E2:1C:3B:4B:9F:81:E6:B8:5C:F4:A5:A8:D8:73:04:BB:05:2F:70:9F:04:A9:0E:05:E9:26:33:E8:70:88:A2\r
m=audio 9 UDP/TLS/RTP/SAVPF 111\r
a=mid:0\r
a=ice-ufrag:EsAw\r
a=ice-pwd:bP+XJMM09aR8AiX1jdukzR6Y\r
a=setup:actpass\r
a=rtpmap:111 opus/48000/2\r
",
    )
    .expect("offer");
    let identity = LocalIdentity::new(
        "ufrag",
        "pwd\r\na=fingerprint:sha-256 AA",
        Fingerprint::new("sha-256", vec![0xAB; 32]),
    );
    let answer = Offer::answer()
        .identity(identity)
        .accept_audio(vec![Attribute::RtpMap(RtpMap {
            payload: 111,
            encoding: "opus".into(),
            clock_rate: 48_000,
            channels: Some(2),
        })])
        .build(&offer)
        .expect("answer structure");
    assert!(matches!(
        write_sdp(answer.sdp()),
        Err(SdpError::ForbiddenCharacter {
            line_type: 'a',
            character: '\r'
        })
    ));
}
