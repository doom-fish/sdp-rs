# sdp-rs

Pure-Rust [SDP](https://www.rfc-editor.org/rfc/rfc8866) (Session Description
Protocol) parser and generator with the WebRTC profile extensions a real-time
meeting client needs to interoperate with browsers and SFUs.

**Zero runtime dependencies.** The entire crate is hand-rolled `std`-only text
processing — no parser generators, no regex, no async runtime.

## What it does

- **RFC 8866 core**: every standard line (`v= o= s= i= u= e= p= c= b= t= r=
  z= k= a= m=`), parsed into typed structures and rendered back losslessly.
- **Structured errors**: malformed input is rejected with a precise
  [`SdpError`] (line number + offending text), never a panic.
- **Round-trip stable**: a conforming offer parsed and re-emitted is
  byte-for-byte stable (modulo line ordering the RFC permits to vary, and
  fingerprint hash names, which are normalised to lower case).
- **No line injection**: [`writer::write_sdp`] returns
  [`SdpError::ForbiddenCharacter`] for any value containing CR, LF or NUL, and
  the parser rejects a CR or NUL inside a line, so neither a peer nor the
  application can smuggle extra lines (such as a second `a=fingerprint`) into
  the output.

## WebRTC profile extensions

These are mandatory for modern browser interop, not optional extras:

| Feature | RFC | Surface |
| --- | --- | --- |
| BUNDLE grouping | 8843 | `a=group:BUNDLE`, per-media `a=mid:`, `a=bundle-only` |
| RTP/RTCP multiplexing | 5761 | `a=rtcp-mux` |
| ICE | 8839 | `a=ice-ufrag`, `a=ice-pwd`, `a=ice-options`, full `a=candidate:` parser |
| DTLS-SRTP fingerprint | 8122 | `a=fingerprint:sha-256 <hex:hex:…>`, digest length checked per hash function |
| DTLS setup role | 8842 | `a=setup:actpass\|active\|passive\|holdconn` |
| Direction | 8866 | `a=sendrecv\|sendonly\|recvonly\|inactive` |
| Payload mapping | 8866 | `a=rtpmap`, `a=fmtp`, `a=rtcp-fb` |
| SSRC signalling | 5576 | `a=ssrc`, `a=ssrc-group` |
| Stream/track id | — | `a=msid`, `a=mid` |
| Header extensions | 8285 | `a=extmap` incl. MID + TWCC URIs |
| H.264 fmtp helpers | 6184 | typed `profile-level-id` / `packetization-mode` / `level-asymmetry-allowed` |
| Opus fmtp helpers | 7587 | typed `minptime` / `useinbandfec` / `usedtx` / `stereo` / `maxaveragebitrate` |

## Quick start

```rust
use sdp::{Offer, LocalIdentity, Fingerprint, Attribute, RtpMap};

# fn demo(offer_text: &str) -> sdp::Result<()> {
let offer = Offer::parse(offer_text)?;

let identity = LocalIdentity::new(
    "8hhY",
    "asd88fgpdd777uzjYhag",
    Fingerprint::parse("sha-256 12:AB:34:CD:EF:00:11:22:BB:CC:DD:EE:FF:01:23:45:67:89:AB:CD:EF:10:32:54:76:98:BA:DC:FE:0F:1E:2D")?,
);

let answer = Offer::answer()
    .identity(identity)
    .accept_audio(vec![Attribute::RtpMap(RtpMap {
        payload: 111,
        encoding: "opus".into(),
        clock_rate: 48_000,
        channels: Some(2),
    })])
    .reject_data() // explicit data-channel rejection — see below
    .build(&offer)?;

let _text = sdp::writer::write_sdp(answer.sdp())?;
# Ok(())
# }
```

The answer builder:

- lists only the offered payload types you pass an `a=rtpmap` for, in offer
  order, and rejects a section with no common codec (port 0);
- tags the BUNDLE group with the first offered mid it accepts on a non-zero
  port, accepts `a=bundle-only` sections of the group only when such a tag
  exists, and groups only mids the offer grouped;
- takes the DTLS role from the media-level `a=setup`, falling back to the
  session level, and refuses offers without any `a=fingerprint`;
- mirrors `a=ice-options:trickle` once, at session level, when offered;
- uses its own session id; pass the previous answer to
  `AnswerBuilder::previous_answer` on renegotiation to keep the `o=` line and
  increment its version.

Session-level fallbacks are available on [`Sdp`] (`fingerprints_for`,
`setup_for`, `ice_ufrag_for`, `ice_pwd_for`) for offers, such as Firefox's,
that put these attributes at session level.

`Debug` output redacts credentials so documents can be logged: the ICE
password (`a=ice-pwd`, including `LocalIdentity`'s), `a=crypto` and
`a=key-mgmt` values and `k=` keys. Parse errors do not echo them either.
[`writer::write_sdp`] still emits the real values.

The low-level [`Sdp`] type is public too, for tests, debugging, or building
documents by hand; serialise it with [`writer::write_sdp`].

## Data channels (not supported)

This crate intentionally does **not** implement the SCTP/DTLS data-channel
(`m=application`) transport. A browser that offers a data channel must still
receive a syntactically valid answer, or it aborts the entire negotiation —
including the audio and video it *did* offer.

The builder therefore answers every `m=application` section with **port 0**
(RFC 8843 §7.2), the standard way to decline a media section while keeping its
`mid` in the BUNDLE bookkeeping. Call [`AnswerBuilder::reject_data`] to make
this explicit, or use the standalone [`reject_media_section`] helper for any
section you want to turn down. The protocol token and format list are
preserved so the rejection round-trips and the peer can match it back to its
offer.

## Examples

```bash
cargo run --example 01_parse_offer
cargo run --example 02_build_answer
cargo run --example 03_round_trip
```

## Validation

```bash
cargo check
cargo clippy --all-targets -- -D warnings
cargo test
cargo doc --no-deps
```

## Status

`0.2.0` (not published to crates.io; `publish = false`). The RFC 8866 grammar and the WebRTC profile attributes listed above
are implemented and tested. See [`COVERAGE.md`](COVERAGE.md) for the
section-by-section breakdown and the parts intentionally left out.

## License

Licensed under either of:

- MIT license ([`LICENSE-MIT`](LICENSE-MIT))
- Apache License 2.0 ([`LICENSE-APACHE`](LICENSE-APACHE))

at your option.
