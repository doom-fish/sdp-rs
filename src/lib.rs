#![doc = include_str!("../README.md")]
//!
//! ---
//!
//! # API documentation
#![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod attributes;
pub mod bundle;
pub mod error;
pub mod extmap;
pub mod fingerprint;
mod fmtp;
pub mod h264;
pub mod ice;
pub mod media;
pub mod opus;
pub mod parser;
pub mod session;
pub mod writer;

pub use attributes::{
    Attribute, Direction, Fmtp, RtcpFb, RtpMap, SetupRole, Ssrc, SsrcGroup,
};
pub use bundle::Group;
pub use error::{Result, SdpError};
pub use extmap::Extmap;
pub use fingerprint::Fingerprint;
pub use fmtp::FmtpParams;
pub use h264::H264Fmtp;
pub use ice::Candidate;
pub use media::{MediaDescription, MediaKind};
pub use opus::OpusFmtp;
pub use session::{
    reject_media_section, Answer, AnswerBuilder, Bandwidth, Connection, LocalIdentity, Offer,
    Origin, Sdp, Timing,
};
