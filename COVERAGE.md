# Coverage

This document records which parts of the SDP specification surface
`sdp-rs` implements, and which are intentionally deferred. The goal of
0.2.0 is a faithful, round-trip-safe core plus the WebRTC profile
attributes needed to negotiate audio/video and cleanly decline data
channels — not exhaustive coverage of every historical SDP extension.

## RFC 8866 — SDP base grammar

| Line | Field | Status | Notes |
|------|-------|--------|-------|
| `v=` | Protocol version | Implemented | Must be `0`; any other value is rejected by the parser and the writer. |
| `o=` | Origin | Implemented | `sess-id` / `sess-version` validated numeric, kept as text to avoid `u64` precision loss. |
| `s=` | Session name | Implemented | Preserved verbatim. |
| `i=` | Session / media title | Implemented | Preserved; routed to the active media section after `m=`. |
| `u=` | URI | Preserved | Carried through as an unrecognised session line. |
| `e=` / `p=` | Email / phone | Preserved | Carried through verbatim. |
| `c=` | Connection data | Implemented | Typed (`Connection`). |
| `b=` | Bandwidth | Implemented | Typed (`Bandwidth`); `b=<type>:<value>`. |
| `t=` | Timing | Implemented | Typed (`Timing`). |
| `r=` | Repeat times | Preserved | Appended to the last `t=`; not interpreted. |
| `z=` | Time-zone adjustments | Preserved | Not interpreted. |
| `k=` | Encryption key | Preserved | Deprecated by RFC 8866; carried through but never emitted by the builder. |
| `m=` | Media description | Implemented | Requires ≥1 format token; port-count (`/n`) supported. |
| `a=` | Attributes | Implemented | Typed where recognised, otherwise `Attribute::Other`. |

Unrecognised lines and attributes are preserved byte-for-byte through a
parse → generate cycle, so the writer never silently drops information.

The writer refuses any value containing CR, LF or NUL
(`SdpError::ForbiddenCharacter`), and the parser rejects a CR or NUL inside a
line (RFC 8866 `byte-string`), so no value can inject extra lines.

`Debug` output redacts credentials: `a=ice-pwd`, `a=crypto` (RFC 4568 SDES
keys) and `a=key-mgmt` (RFC 4567) values, and `k=` keys. A malformed line
carrying one of them is reported without its value. The writer still emits
the real text.

## WebRTC profile attributes

| Attribute | Status | Reference |
|-----------|--------|-----------|
| `a=group:BUNDLE` | Implemented | RFC 8843 |
| `a=bundle-only` | Implemented | RFC 8843; a port-0 section with it is not treated as rejected |
| `a=mid` | Implemented | RFC 5888 |
| `a=msid` | Implemented | RFC 8830 |
| `a=rtcp-mux` / `a=rtcp-mux-only` / `a=rtcp-rsize` | Implemented | RFC 5761 / RFC 8858 |
| `a=ice-ufrag` / `a=ice-pwd` / `a=ice-options` | Implemented | RFC 8839; media accessors plus session-level fallback via `Sdp::ice_ufrag_for` / `ice_pwd_for` |
| `a=candidate` / `a=end-of-candidates` | Implemented | RFC 8839; an extension name without a value is rejected |
| `a=fingerprint` | Implemented | RFC 8122; digest length checked for md2/md5/sha-1/sha-224/sha-256/sha-384/sha-512, algorithm lower-cased and compared case-insensitively; session-level fallback via `Sdp::fingerprints_for` |
| `a=setup` | Implemented | RFC 4145 / RFC 8842; session-level fallback via `Sdp::setup_for` |
| `a=sendrecv` / `sendonly` / `recvonly` / `inactive` | Implemented | RFC 8866 |
| `a=rtpmap` | Implemented | RFC 8866 |
| `a=fmtp` | Implemented | RFC 8866; typed H.264 / Opus helpers |
| `a=rtcp-fb` | Implemented | RFC 4585 |
| `a=ssrc` / `a=ssrc-group` | Implemented | RFC 5576 |
| `a=extmap` | Implemented | RFC 8285; MID / TWCC / abs-send-time URIs |

## Offer/answer

`AnswerBuilder` mirrors the offer's media order and, for each accepted
audio/video section, answers only the offered payload types the application
supplied an `a=rtpmap` for (rejecting the section if none match), tags the
BUNDLE group with the first offered mid it accepts on a non-zero port and
accepts `a=bundle-only` sections of the group only when such a tag exists
(RFC 8843 §7.3), groups only mids the offer grouped, derives the DTLS role
from the media- or session-level `a=setup`, requires an `a=fingerprint`
somewhere in the offer, and mirrors `a=ice-options:trickle` once at session
level. Answers get their own session id; `previous_answer` keeps the previous
`o=` line and increments the version (RFC 3264 §8). Data channels are always
declined with port 0.

## Codec fmtp helpers

- **H.264** (RFC 6184): typed `profile-level-id`, `packetization-mode`,
  and `level-asymmetry-allowed`, with constrained-baseline 3.1 defaults.
  Unknown parameters are preserved in order.
- **Opus** (RFC 7587): typed `minptime`, `useinbandfec`, `usedtx`,
  `stereo`, and `maxaveragebitrate`, with meeting defaults. Unknown
  parameters are preserved in order.

## Intentionally deferred

These are out of scope for 0.2.0 and are carried through verbatim rather
than interpreted:

- The general SDP grouping framework (RFC 5888) beyond `BUNDLE`
  semantics (e.g. `LS`, `FID`, `SRF`). `Group` stores arbitrary
  semantics tokens but only `BUNDLE` has a constructor helper.
- `r=` repeat and `z=` time-zone semantics — preserved but not expanded
  into concrete schedules.
- `k=` encryption keys — deprecated; preserved on parse, never generated.
- SDP capability negotiation (RFC 5939), simulcast (RFC 8853), and
  redundancy/FEC grouping beyond `ssrc-group` pass-through.
- Full ICE candidate trickle state-machine logic; this crate models the
  candidate lines, not the agent.

## Validation

- `cargo test` — unit tests plus RFC 8866, WebRTC offer/answer, answer
  negotiation, session-level fallback, H.264, Opus, line-injection, secret
  redaction and adversarial-robustness integration suites.
- `cargo clippy --all-targets --all-features -- -D warnings` — clean under
  pedantic, nursery, and cargo lint groups.
- `cargo doc --no-deps` — documents with zero warnings.
- Zero runtime dependencies; `publish = false`.
