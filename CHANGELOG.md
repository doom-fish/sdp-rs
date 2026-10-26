# Changelog

All notable changes to `sdp-rs` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - Unreleased

### Security

- `writer::write_sdp` rejects CR, LF and NUL in every emitted value with
  `SdpError::ForbiddenCharacter`, and the parser rejects a CR or NUL inside a
  line. No value can inject extra lines (such as a second `a=fingerprint`),
  and a mid-line CR in an offered `mid` can no longer reach the answer.
- `Fingerprint::parse` checks the digest length against the hash function
  (md2, md5, sha-1, sha-224, sha-256, sha-384, sha-512).
- `LocalIdentity`'s `Debug` output, and therefore `AnswerBuilder`'s, redacts
  `ice_pwd`; equality compares the password in constant time.
- `Debug` output redacts every credential-bearing value, so `Offer`, `Answer`,
  `Sdp` and `MediaDescription` can be logged. This covers `Attribute::IcePwd`,
  an `Attribute::Other` named `ice-pwd`, `crypto` (RFC 4568 SDES keys) or
  `key-mgmt` (RFC 4567) in any case, and the `k=` key of `Sdp` and
  `MediaDescription`. `SdpError::MalformedLine` no longer echoes such a value.
  `Display` and `writer::write_sdp` still emit the real wire text.

### Fixed

- The answer lists only the offered payload types the application supplied an
  `a=rtpmap` for, in offer order; other offered codecs (including static
  PT 0/8/9) are no longer echoed, plan attributes for other payload types are
  dropped, and a section with no common codec is rejected with port 0.
- Port-0 `a=bundle-only` sections in the offer's BUNDLE group are accepted
  instead of rejected, provided the group's tagged section (the first mid
  accepted on a non-zero port) is accepted; the tag is listed first, and only
  mids the offer grouped are grouped in the answer.
- The answer's DTLS role falls back to the session-level `a=setup`.
- Accepting media from an offer without any `a=fingerprint` fails, as the
  `AnswerBuilder::build` documentation already stated.
- `a=ice-options:trickle` is emitted once, at session level, and only when the
  offer advertised trickle (matched as a token).
- `v=` other than `0` is rejected by the parser and refused by the writer.
- `Candidate::parse` rejects an extension name without a value instead of
  dropping it.
- Fingerprint hash names are compared case-insensitively (`SHA-256` equals
  `sha-256`).

### Changed

- **Breaking:** `writer::write_sdp` returns `Result<String>`.
- **Breaking:** `Sdp`, `Offer` and `Answer` no longer implement `Display`
  (it could only fail by panicking in `to_string`); serialise with
  `writer::write_sdp(offer.sdp())`.
- **Breaking:** `Fingerprint::new` and `Fingerprint::parse` lower-case the
  algorithm, and `Fingerprint` equality ignores its case.
- **Breaking:** `a=bundle-only` parses to `Attribute::BundleOnly` instead of
  `Attribute::Other`, and `MediaDescription::is_rejected` (no longer `const`)
  returns `false` for bundle-only sections.
- **Breaking:** `H264Fmtp` and `OpusFmtp` are type aliases of
  `FmtpParams<h264::H264>` and `FmtpParams<opus::Opus>`; their methods are
  unchanged.
- **Breaking:** `AnswerBuilder::accept_audio`/`accept_video` select payload
  types from the supplied `a=rtpmap` attributes; a plan without a matching
  rtpmap rejects the section.
- The answer's `o=` line uses its own random session id instead of the
  offerer's.
- `rust-version` is 1.82.

### Added

- `SdpError::ForbiddenCharacter { line_type, character }`.
- `Sdp::fingerprints_for`, `Sdp::setup_for`, `Sdp::ice_ufrag_for` and
  `Sdp::ice_pwd_for` (media level with session-level fallback).
- `MediaDescription::fingerprints`, `ice_ufrag`, `ice_pwd`, `candidates` and
  `is_bundle_only`.
- `Attribute::BundleOnly`.
- `AnswerBuilder::previous_answer` to keep the `o=` line and increment the
  session version on renegotiation.
- `FmtpParams`, `h264::H264` and `opus::Opus`.
- Regression tests for line injection, secret redaction, answer negotiation
  and session-level fallback.

## [0.1.0] - 2026-06-04

### Added

- Initial release: a pure-Rust, zero-dependency SDP (RFC 8866)
  parser and generator.
- `Sdp::parse` / `Display`: a lenient, line-oriented parser and a
  canonical CRLF writer that round-trips recognised lines losslessly and
  preserves unknown attributes verbatim.
- Session-level types: `Origin`, `Connection`, `Bandwidth`, `Timing`,
  and `Sdp`.
- `MediaDescription` with typed accessors (`mid`, `direction`, `setup`,
  `fingerprint`, `has_rtcp_mux`, `rtpmaps`, `fmtp_for`, `is_rejected`).
- Typed `Attribute` enum covering the WebRTC attribute set: `rtcp-mux`,
  `rtcp-rsize`, `mid`, `msid`, `group`, `ice-ufrag`, `ice-pwd`,
  `ice-options`, `candidate`, `end-of-candidates`, `fingerprint`,
  `setup`, direction flags, `rtpmap`, `fmtp`, `rtcp-fb`, `ssrc`,
  `ssrc-group`, and `extmap`.
- WebRTC profile helpers:
  - `bundle::Group` and the `BUNDLE` semantics constant.
  - `ice::Candidate` full attribute parser.
  - `fingerprint::Fingerprint` for DTLS-SRTP.
  - `extmap::Extmap` with MID / TWCC / abs-send-time URI constants.
  - `h264::H264Fmtp` and `opus::OpusFmtp` typed fmtp helpers with
    meeting-friendly defaults.
- High-level offer/answer workflow: `Offer`, `Answer`, `LocalIdentity`,
  and `AnswerBuilder` (`accept_audio`, `accept_video`, `reject_data`),
  which mirrors media order, reverses directionality, picks the DTLS
  role, rebuilds the BUNDLE group, and preserves negotiated header
  extensions.
- Explicit data-channel rejection via `reject_media_section` /
  `AnswerBuilder::reject_data` (RFC 8843 §7.2, port 0).
- `error::SdpError` structured error type with line context.
- Examples: `01_parse_offer`, `02_build_answer`, `03_round_trip`.
- Test suites covering RFC 8866 basics, a realistic WebRTC offer, the
  answer workflow, H.264 / Opus fmtp round-trips, and adversarial
  malformed input robustness.

[0.2.0]: https://github.com/doom-fish/sdp-rs/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/doom-fish/sdp-rs/releases/tag/v0.1.0
