use sdp::attributes::Attribute;
use sdp::writer::write_sdp;
use sdp::{Fingerprint, LocalIdentity, Offer, RtpMap, Sdp, SdpError};

const PWD: &str = "s3cretIcePasswordMaterial";

fn identity(pwd: &str) -> LocalIdentity {
    LocalIdentity::new("ufrag", pwd, Fingerprint::new("sha-256", vec![0x5A; 32]))
}

#[test]
fn local_identity_debug_redacts_the_ice_password() {
    let text = format!("{:?}", identity(PWD));
    assert!(!text.contains(PWD), "Debug leaked the ICE password: {text}");
    assert!(text.contains("ufrag"));
    assert!(text.contains("<redacted>"));

    let builder = Offer::answer()
        .identity(identity(PWD))
        .accept_audio(vec![Attribute::RtpMap(RtpMap {
            payload: 111,
            encoding: "opus".into(),
            clock_rate: 48_000,
            channels: Some(2),
        })]);
    let text = format!("{builder:?}");
    assert!(
        !text.contains(PWD),
        "AnswerBuilder Debug leaked the ICE password: {text}"
    );
}

#[test]
fn local_identity_equality_still_compares_every_field() {
    assert_eq!(identity(PWD), identity(PWD));
    assert_ne!(identity(PWD), identity("s3cretIcePasswordMateriaL"));
    assert_ne!(identity(PWD), identity("short"));
    let mut other = identity(PWD);
    other.ice_ufrag = "other".into();
    assert_ne!(identity(PWD), other);
    let mut other = identity(PWD);
    other.fingerprint = Fingerprint::new("sha-256", vec![0xA5; 32]);
    assert_ne!(identity(PWD), other);
}

const SESSION_KEY: &str = "s3cretSessionKeyMaterial";
const SESSION_PWD: &str = "s3cretSessionIcePassword";
const MEDIA_KEY: &str = "czNjcmV0TWVkaWFLZXk=";
const MEDIA_PWD: &str = "s3cretMediaIcePassword";
const CRYPTO_KEY: &str = "PS1uQCVeeCFCanVmcjkpPywjNWhcYD0mXXtxaVBR";
const MIKEY: &str = "AQAFgM0XflABAAAAAAAAAAAAAAYAyOr5T2yH";

fn offer_text() -> String {
    let digest = vec!["5A"; 32].join(":");
    format!(
        "v=0\r\n\
         o=- 1 1 IN IP4 127.0.0.1\r\n\
         s=-\r\n\
         t=0 0\r\n\
         k=clear:{SESSION_KEY}\r\n\
         a=ice-pwd:{SESSION_PWD}\r\n\
         m=audio 9 UDP/TLS/RTP/SAVPF 111\r\n\
         k=base64:{MEDIA_KEY}\r\n\
         a=mid:0\r\n\
         a=ice-ufrag:visibleUfrag\r\n\
         a=ice-pwd:{MEDIA_PWD}\r\n\
         a=crypto:1 AES_CM_128_HMAC_SHA1_80 inline:{CRYPTO_KEY}|2^20|1:32\r\n\
         a=key-mgmt:mikey {MIKEY}\r\n\
         a=fingerprint:sha-256 {digest}\r\n\
         a=rtpmap:111 opus/48000/2\r\n"
    )
}

fn assert_redacted(text: &str) {
    for secret in [
        SESSION_KEY,
        SESSION_PWD,
        MEDIA_KEY,
        MEDIA_PWD,
        CRYPTO_KEY,
        MIKEY,
        PWD,
    ] {
        assert!(!text.contains(secret), "Debug leaked {secret}: {text}");
    }
}

#[test]
fn attribute_debug_redacts_credentials_and_display_keeps_them() {
    let pwd = Attribute::IcePwd(PWD.into());
    assert_eq!(format!("{pwd:?}"), r#"IcePwd("<redacted>")"#);
    assert_eq!(pwd.to_string(), format!("ice-pwd:{PWD}"));

    let crypto_text = format!("crypto:1 AES_CM_128_HMAC_SHA1_80 inline:{CRYPTO_KEY}|2^20|1:32");
    let crypto = Attribute::parse(&crypto_text).unwrap();
    assert_eq!(
        format!("{crypto:?}"),
        r#"Other { name: "crypto", value: Some("<redacted>") }"#
    );
    assert_eq!(crypto.to_string(), crypto_text);

    let mikey_text = format!("key-mgmt:mikey {MIKEY}");
    let mikey = Attribute::parse(&mikey_text).unwrap();
    assert_redacted(&format!("{mikey:?} {mikey:#?}"));
    assert_eq!(mikey.to_string(), mikey_text);

    let shouted = Attribute::Other {
        name: "ICE-PWD".into(),
        value: Some(PWD.into()),
    };
    assert_eq!(
        format!("{shouted:?}"),
        r#"Other { name: "ICE-PWD", value: Some("<redacted>") }"#
    );
    assert_eq!(shouted.to_string(), format!("ICE-PWD:{PWD}"));

    let bare = Attribute::Other {
        name: "crypto".into(),
        value: None,
    };
    assert_eq!(format!("{bare:?}"), r#"Other { name: "crypto", value: None }"#);
}

#[test]
fn attribute_debug_is_unchanged_for_other_attributes() {
    assert_eq!(format!("{:?}", Attribute::RtcpMux), "RtcpMux");
    assert_eq!(format!("{:?}", Attribute::Mid("0".into())), r#"Mid("0")"#);
    assert_eq!(
        format!("{:?}", Attribute::IceUfrag("visibleUfrag".into())),
        r#"IceUfrag("visibleUfrag")"#
    );
    assert_eq!(
        format!(
            "{:?}",
            Attribute::Other {
                name: "x-vendor".into(),
                value: Some("visible".into()),
            }
        ),
        r#"Other { name: "x-vendor", value: Some("visible") }"#
    );
    assert_eq!(
        format!("{:?}", Attribute::parse("rtpmap:111 opus/48000/2").unwrap()),
        r#"RtpMap(RtpMap { payload: 111, encoding: "opus", clock_rate: 48000, channels: Some(2) })"#
    );
}

#[test]
fn parsed_documents_redact_credentials_in_debug_and_write_them_unchanged() {
    let text = offer_text();
    let offer = Offer::parse(&text).unwrap();
    let debug = format!("{offer:?}\n{offer:#?}\n{:?}", offer.sdp().media[0]);
    assert_redacted(&debug);
    assert!(debug.contains("visibleUfrag"));
    assert!(debug.contains(r#"key: Some("<redacted>")"#));

    assert_eq!(write_sdp(offer.sdp()).unwrap(), text);
    let media = &offer.sdp().media[0];
    assert_eq!(media.ice_pwd(), Some(MEDIA_PWD));
    assert_eq!(offer.sdp().key.as_deref(), Some(&*format!("clear:{SESSION_KEY}")));
    assert_eq!(media.key.as_deref(), Some(&*format!("base64:{MEDIA_KEY}")));
}

#[test]
fn answers_redact_the_local_ice_password_only_in_debug() {
    let offer = Offer::parse(&offer_text()).unwrap();
    let answer = Offer::answer()
        .identity(identity(PWD))
        .accept_audio(vec![Attribute::RtpMap(RtpMap {
            payload: 111,
            encoding: "opus".into(),
            clock_rate: 48_000,
            channels: Some(2),
        })])
        .build(&offer)
        .unwrap();
    assert_redacted(&format!("{answer:?} {answer:#?}"));
    assert!(write_sdp(answer.sdp())
        .unwrap()
        .contains(&format!("\r\na=ice-pwd:{PWD}\r\n")));
}

#[test]
fn malformed_credential_lines_are_redacted_in_errors() {
    let header = "v=0\r\no=- 1 1 IN IP4 127.0.0.1\r\ns=-\r\nt=0 0\r\n";
    let media = "m=audio 9 UDP/TLS/RTP/SAVPF 111\r\n";
    let cases = [
        (format!("{header}a=ice-pwd:{PWD}\0tail\r\n"), 5, "a=ice-pwd:<redacted>"),
        (format!("{header}a=ICE-PWD:{PWD}\rtail\r\n"), 5, "a=ICE-PWD:<redacted>"),
        (format!("{header}k=clear:{SESSION_KEY}\0\r\n"), 5, "k=<redacted>"),
        (
            format!("{header}{media}k=base64:{MEDIA_KEY}\rtail\r\n"),
            6,
            "k=<redacted>",
        ),
        (
            format!("{header}{media}a=crypto:1 AES_CM_128_HMAC_SHA1_80 inline:{CRYPTO_KEY}\0\r\n"),
            6,
            "a=crypto:<redacted>",
        ),
        (
            format!("{header}{media}a=key-mgmt\0mikey {MIKEY}\r\n"),
            6,
            "a=key-mgmt<redacted>",
        ),
    ];
    for (text, line, redacted) in cases {
        let err = Sdp::parse(&text).unwrap_err();
        assert_eq!(
            err,
            SdpError::MalformedLine {
                line,
                text: redacted.into(),
            }
        );
        assert_redacted(&format!("{err} {err:?}"));
    }

    let err = Sdp::parse(&format!("{header}a=mid:0\0tail\r\n")).unwrap_err();
    assert_eq!(
        err,
        SdpError::MalformedLine {
            line: 5,
            text: "a=mid:0\0tail".into(),
        }
    );
}
