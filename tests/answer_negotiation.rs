use sdp::writer::write_sdp;
use sdp::{
    Attribute, Direction, Fingerprint, Fmtp, LocalIdentity, MediaKind, Offer, RtcpFb, RtpMap,
    SdpError, SetupRole,
};

const FINGERPRINT: &str = "19:E2:1C:3B:4B:9F:81:E6:B8:5C:F4:A5:A8:D8:73:04:BB:05:2F:70:9F:04:A9:0E:05:E9:26:33:E8:70:88:A2";

fn offer_text(session_extra: &str, audio_extra: &str) -> String {
    format!(
        "\
v=0\r
o=mozilla...THIS_IS_SDPARTA-99.0 7157000000000000000 0 IN IP4 0.0.0.0\r
s=-\r
t=0 0\r
{session_extra}a=group:BUNDLE 0 1\r
m=audio 9 UDP/TLS/RTP/SAVPF 109 9 0 8 101\r
c=IN IP4 0.0.0.0\r
a=sendrecv\r
a=ice-pwd:mediaLevelPasswordValue0\r
a=ice-ufrag:m0uf\r
a=mid:0\r
a=rtcp-mux\r
{audio_extra}a=rtpmap:109 opus/48000/2\r
a=fmtp:109 maxplaybackrate=48000;stereo=1;useinbandfec=1\r
a=rtpmap:9 G722/8000/1\r
a=rtpmap:0 PCMU/8000\r
a=rtpmap:8 PCMA/8000\r
a=rtpmap:101 telephone-event/8000\r
a=fmtp:101 0-15\r
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

fn firefox_offer() -> Offer {
    Offer::parse(&offer_text(
        &format!(
            "a=fingerprint:sha-256 {FINGERPRINT}\r\na=setup:actpass\r\na=ice-options:trickle\r\n"
        ),
        "",
    ))
    .expect("offer")
}

fn identity() -> LocalIdentity {
    LocalIdentity::new(
        "9aBc",
        "answerPasswordMaterial0123456",
        Fingerprint::new("sha-256", vec![0x99; 32]),
    )
}

fn rtpmap(payload: u8, encoding: &str, clock_rate: u32, channels: Option<u32>) -> Attribute {
    Attribute::RtpMap(RtpMap {
        payload,
        encoding: encoding.into(),
        clock_rate,
        channels,
    })
}

fn opus_only() -> Vec<Attribute> {
    vec![
        rtpmap(109, "opus", 48_000, Some(2)),
        Attribute::Fmtp(Fmtp {
            format: "109".into(),
            params: "minptime=10;useinbandfec=1".into(),
        }),
    ]
}

fn vp8_only() -> Vec<Attribute> {
    vec![rtpmap(120, "VP8", 90_000, None)]
}

#[test]
fn answer_lists_only_supported_payload_types() {
    let answer = Offer::answer()
        .identity(identity())
        .accept_audio(opus_only())
        .build(&firefox_offer())
        .expect("answer");
    let audio = answer.sdp().media_by_mid("0").unwrap();
    assert_eq!(audio.formats, vec!["109"]);
    let payloads: Vec<u8> = audio.rtpmaps().iter().map(|m| m.payload).collect();
    assert_eq!(payloads, vec![109]);
    assert!(audio.fmtp_for(101).is_none());
}

#[test]
fn supported_static_payload_types_keep_offer_order() {
    let mut attrs = vec![rtpmap(0, "PCMU", 8_000, None)];
    attrs.extend(opus_only());
    attrs.push(Attribute::Fmtp(Fmtp {
        format: "110".into(),
        params: "unused=1".into(),
    }));
    attrs.push(Attribute::RtcpFb(RtcpFb {
        format: "110".into(),
        feedback: "nack".into(),
    }));
    attrs.push(Attribute::RtcpFb(RtcpFb {
        format: "*".into(),
        feedback: "transport-cc".into(),
    }));
    let answer = Offer::answer()
        .identity(identity())
        .accept_audio(attrs)
        .build(&firefox_offer())
        .expect("answer");
    let audio = answer.sdp().media_by_mid("0").unwrap();
    assert_eq!(audio.formats, vec!["109", "0"]);
    assert!(audio
        .attributes
        .iter()
        .all(|a| !matches!(a, Attribute::Fmtp(f) if f.format == "110")));
    assert!(audio
        .attributes
        .iter()
        .all(|a| !matches!(a, Attribute::RtcpFb(fb) if fb.format == "110")));
    assert!(audio
        .attributes
        .iter()
        .any(|a| matches!(a, Attribute::RtcpFb(fb) if fb.format == "*")));
}

#[test]
fn section_without_a_common_codec_is_rejected() {
    let answer = Offer::answer()
        .identity(identity())
        .accept_audio(vec![rtpmap(111, "opus", 48_000, Some(2))])
        .build(&firefox_offer())
        .expect("answer");
    let audio = answer.sdp().media_by_mid("0").unwrap();
    assert!(audio.is_rejected());
    assert_eq!(audio.formats, vec!["109", "9", "0", "8", "101"]);
    assert!(answer.sdp().bundle_group().is_none());
}

#[test]
fn bundle_only_sections_are_accepted_into_the_bundle() {
    let answer = Offer::answer()
        .identity(identity())
        .accept_audio(opus_only())
        .accept_video(vp8_only())
        .build(&firefox_offer())
        .expect("answer");
    let video = answer.sdp().media_by_mid("1").unwrap();
    assert_eq!(video.kind, MediaKind::Video);
    assert!(!video.is_rejected());
    assert_eq!(video.port, 9);
    assert_eq!(video.formats, vec!["120"]);
    assert_eq!(video.direction(), Some(Direction::SendOnly));
    assert_eq!(
        answer.sdp().bundle_group().expect("bundle").mids,
        vec!["0", "1"]
    );
}

#[test]
fn bundle_only_section_outside_the_bundle_is_rejected() {
    let text = offer_text(&format!("a=fingerprint:sha-256 {FINGERPRINT}\r\n"), "")
        .replace("a=group:BUNDLE 0 1\r\n", "a=group:BUNDLE 0\r\n");
    let offer = Offer::parse(&text).expect("offer");
    let answer = Offer::answer()
        .identity(identity())
        .accept_audio(opus_only())
        .accept_video(vp8_only())
        .build(&offer)
        .expect("answer");
    assert!(answer.sdp().media_by_mid("1").unwrap().is_rejected());
}

#[test]
fn session_level_setup_drives_the_answer_role() {
    for (offered, expected) in [
        ("active", SetupRole::Passive),
        ("passive", SetupRole::Active),
        ("actpass", SetupRole::Active),
    ] {
        let offer = Offer::parse(&offer_text(
            &format!("a=fingerprint:sha-256 {FINGERPRINT}\r\na=setup:{offered}\r\n"),
            "",
        ))
        .expect("offer");
        let answer = Offer::answer()
            .identity(identity())
            .accept_audio(opus_only())
            .build(&offer)
            .expect("answer");
        assert_eq!(
            answer.sdp().media_by_mid("0").unwrap().setup(),
            Some(expected),
            "offer setup {offered}"
        );
    }

    let offer = Offer::parse(&offer_text(
        &format!("a=fingerprint:sha-256 {FINGERPRINT}\r\na=setup:active\r\n"),
        "a=setup:passive\r\n",
    ))
    .expect("offer");
    let answer = Offer::answer()
        .identity(identity())
        .accept_audio(opus_only())
        .build(&offer)
        .expect("answer");
    assert_eq!(
        answer.sdp().media_by_mid("0").unwrap().setup(),
        Some(SetupRole::Active),
        "media-level setup wins"
    );
}

#[test]
fn offer_without_any_fingerprint_is_refused() {
    let offer = Offer::parse(&offer_text("", "")).expect("offer");
    let result = Offer::answer()
        .identity(identity())
        .accept_audio(opus_only())
        .build(&offer);
    assert!(matches!(result, Err(SdpError::Profile { .. })));

    let rejected_only = Offer::answer().identity(identity()).build(&offer);
    assert!(
        rejected_only.is_ok(),
        "rejecting every section needs no keys"
    );
}

#[test]
fn ice_options_are_emitted_once_at_session_level_when_offered() {
    let answer = Offer::answer()
        .identity(identity())
        .accept_audio(opus_only())
        .build(&firefox_offer())
        .expect("answer");
    let session_trickle = answer
        .sdp()
        .attributes
        .iter()
        .filter(|a| matches!(a, Attribute::IceOptions(o) if o == "trickle"))
        .count();
    assert_eq!(session_trickle, 1);
    assert!(answer
        .sdp()
        .media
        .iter()
        .flat_map(|m| m.attributes.iter())
        .all(|a| !matches!(a, Attribute::IceOptions(_))));

    let no_trickle = Offer::parse(&offer_text(
        &format!("a=fingerprint:sha-256 {FINGERPRINT}\r\na=ice-options:ice2\r\n"),
        "",
    ))
    .expect("offer");
    let answer = Offer::answer()
        .identity(identity())
        .accept_audio(opus_only())
        .build(&no_trickle)
        .expect("answer");
    let text = write_sdp(answer.sdp()).unwrap();
    assert!(!text.contains("ice-options"), "{text}");
}

#[test]
fn session_version_increments_on_renegotiation() {
    let offer = firefox_offer();
    let builder = Offer::answer()
        .identity(identity())
        .accept_audio(opus_only());
    let first = builder.build(&offer).expect("first answer");
    let origin = &first.sdp().origin;
    assert_ne!(origin.session_id, offer.sdp().origin.session_id);
    assert!(origin.session_id.parse::<i64>().is_ok());
    assert_eq!(origin.session_version, "1");

    let second = builder
        .clone()
        .previous_answer(&first)
        .build(&offer)
        .expect("second answer");
    assert_eq!(second.sdp().origin.session_id, origin.session_id);
    assert_eq!(second.sdp().origin.session_version, "2");

    let third = builder
        .clone()
        .previous_answer(&second)
        .build(&offer)
        .expect("third answer");
    assert_eq!(third.sdp().origin.session_version, "3");

    let mut exhausted = first;
    exhausted.0.origin.session_version = i64::MAX.to_string();
    assert!(builder.previous_answer(&exhausted).build(&offer).is_err());
}

#[test]
fn bundle_only_section_needs_an_accepted_tagged_section() {
    let answer = Offer::answer()
        .identity(identity())
        .accept_video(vp8_only())
        .build(&firefox_offer())
        .expect("answer");
    assert!(answer.sdp().media_by_mid("0").unwrap().is_rejected());
    assert!(answer.sdp().media_by_mid("1").unwrap().is_rejected());
    assert!(answer.sdp().bundle_group().is_none());

    let answer = Offer::answer()
        .identity(identity())
        .accept_audio(vec![rtpmap(111, "opus", 48_000, Some(2))])
        .accept_video(vp8_only())
        .build(&firefox_offer())
        .expect("answer");
    assert!(answer.sdp().media_by_mid("1").unwrap().is_rejected());
    assert!(answer.sdp().bundle_group().is_none());
}

#[test]
fn answerer_tagged_section_comes_first_in_the_group() {
    let text = format!(
        "\
v=0\r
o=- 1 1 IN IP4 0.0.0.0\r
s=-\r
t=0 0\r
a=fingerprint:sha-256 {FINGERPRINT}\r
a=group:BUNDLE 0 1 2\r
m=audio 9 UDP/TLS/RTP/SAVPF 109\r
a=mid:0\r
a=rtpmap:109 opus/48000/2\r
m=video 0 UDP/TLS/RTP/SAVPF 120\r
a=bundle-only\r
a=mid:1\r
a=rtpmap:120 VP8/90000\r
m=video 9 UDP/TLS/RTP/SAVPF 120\r
a=mid:2\r
a=rtpmap:120 VP8/90000\r
"
    );
    let offer = Offer::parse(&text).expect("offer");
    let answer = Offer::answer()
        .identity(identity())
        .accept_video(vp8_only())
        .build(&offer)
        .expect("answer");
    assert!(answer.sdp().media_by_mid("0").unwrap().is_rejected());
    assert!(!answer.sdp().media_by_mid("1").unwrap().is_rejected());
    assert!(!answer.sdp().media_by_mid("2").unwrap().is_rejected());
    assert_eq!(
        answer.sdp().bundle_group().expect("bundle").mids,
        vec!["2", "1"]
    );
}

#[test]
fn no_bundle_group_is_answered_without_an_offered_group() {
    let text = offer_text(&format!("a=fingerprint:sha-256 {FINGERPRINT}\r\n"), "")
        .replace("a=group:BUNDLE 0 1\r\n", "");
    let offer = Offer::parse(&text).expect("offer");
    let answer = Offer::answer()
        .identity(identity())
        .accept_audio(opus_only())
        .accept_video(vp8_only())
        .build(&offer)
        .expect("answer");
    assert!(!answer.sdp().media_by_mid("0").unwrap().is_rejected());
    assert!(
        answer.sdp().media_by_mid("1").unwrap().is_rejected(),
        "bundle-only without a BUNDLE group"
    );
    assert!(answer.sdp().bundle_group().is_none());
}
