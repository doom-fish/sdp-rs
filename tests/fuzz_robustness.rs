//! Adversarial / malformed-input robustness tests.
//!
//! These are hand-written (no external fuzzer): every input below must make
//! `Sdp::parse` return an `Err` — never panic, never hang, never `unwrap` an
//! internal `None`. The point is to prove the hand-rolled parser degrades
//! gracefully on hostile input.

use sdp::writer::write_sdp;
use sdp::Sdp;

/// Each of these is structurally invalid and must be rejected.
const MALFORMED: &[&str] = &[
    // Empty input.
    "",
    // Whitespace only.
    "   \r\n\r\n",
    // Missing the mandatory v= line.
    "o=- 1 1 IN IP4 127.0.0.1\r\ns=-\r\nt=0 0\r\n",
    // Missing the mandatory o= line.
    "v=0\r\ns=-\r\nt=0 0\r\n",
    // Missing the mandatory s= line.
    "v=0\r\no=- 1 1 IN IP4 127.0.0.1\r\nt=0 0\r\n",
    // A line without the '=' at position 1.
    "v=0\r\nthis-is-not-sdp\r\ns=-\r\n",
    // Type field longer than one character.
    "vv=0\r\no=- 1 1 IN IP4 127.0.0.1\r\ns=-\r\n",
    // Non-numeric version.
    "v=abc\r\no=- 1 1 IN IP4 127.0.0.1\r\ns=-\r\nt=0 0\r\n",
    // Origin with too few fields.
    "v=0\r\no=- 1 1 IN IP4\r\ns=-\r\nt=0 0\r\n",
    // Origin session-id is not a number.
    "v=0\r\no=- notanumber 1 IN IP4 127.0.0.1\r\ns=-\r\nt=0 0\r\n",
    // Timing with non-numeric start.
    "v=0\r\no=- 1 1 IN IP4 127.0.0.1\r\ns=-\r\nt=x 0\r\n",
    // m= line with a non-numeric port.
    "v=0\r\no=- 1 1 IN IP4 127.0.0.1\r\ns=-\r\nt=0 0\r\nm=audio NaN RTP/AVP 0\r\n",
    // m= line with too few fields.
    "v=0\r\no=- 1 1 IN IP4 127.0.0.1\r\ns=-\r\nt=0 0\r\nm=audio\r\n",
    // Truncated m= line (no formats).
    "v=0\r\no=- 1 1 IN IP4 127.0.0.1\r\ns=-\r\nt=0 0\r\nm=audio 9 RTP/AVP\r\n",
    // Bare '=' with no type.
    "=0\r\n",
];

#[test]
fn malformed_inputs_error_without_panicking() {
    for (i, input) in MALFORMED.iter().enumerate() {
        let result = Sdp::parse(input);
        assert!(
            result.is_err(),
            "input #{i} should have been rejected but parsed: {input:?}",
        );
    }
}

#[test]
fn parser_does_not_panic_on_truncated_prefixes() {
    // Feed every byte-prefix of a valid offer; none may panic.
    let valid = "\
v=0\r\n\
o=- 1 1 IN IP4 127.0.0.1\r\n\
s=-\r\n\
t=0 0\r\n\
a=group:BUNDLE 0\r\n\
m=audio 9 UDP/TLS/RTP/SAVPF 111\r\n\
a=mid:0\r\n\
a=rtcp-mux\r\n\
a=rtpmap:111 opus/48000/2\r\n";

    for end in 0..valid.len() {
        // Must return without panicking; Ok or Err are both acceptable here.
        let _ = Sdp::parse(&valid[..end]);
    }
    assert!(Sdp::parse(valid).is_ok());
}

#[test]
fn parser_handles_lone_lf_and_mixed_newlines() {
    // Bare LF (no CR) is tolerated by RFC 8866 receivers.
    let lf_only = "v=0\no=- 1 1 IN IP4 127.0.0.1\ns=-\nt=0 0\n";
    assert!(Sdp::parse(lf_only).is_ok(), "lone LF should be accepted");

    // A stray CR in the middle of a value must not crash the parser.
    let weird = "v=0\r\no=- 1 1 IN IP4 127.0.0.1\r\ns=-\r\nt=0 0\r\ni=line\rwith\rcr\r\n";
    assert!(Sdp::parse(weird).is_err());
}

#[test]
fn enormous_repeated_attributes_do_not_blow_up() {
    use std::fmt::Write as _;
    let mut text = String::from(
        "v=0\r\no=- 1 1 IN IP4 127.0.0.1\r\ns=-\r\nt=0 0\r\nm=audio 9 RTP/AVP 0\r\n",
    );
    for n in 0..5_000 {
        let _ = write!(text, "a=x-custom-{n}:value-{n}\r\n");
    }
    let parsed = Sdp::parse(&text).expect("large but valid input should parse");
    // Round-trips back to a byte-identical document.
    let text = write_sdp(&parsed).unwrap();
    assert_eq!(write_sdp(&Sdp::parse(&text).unwrap()).unwrap(), text);
}
