//! Structured error type for SDP parsing and construction.
//!
//! The crate has zero external dependencies, so this is a hand-rolled error
//! enum implementing [`std::fmt::Display`] and [`std::error::Error`] rather
//! than relying on a derive macro.

use std::fmt;

/// Errors produced while parsing, validating, or building an SDP document.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SdpError {
    /// A required line (for example `v=`, `o=`, or `s=`) was absent.
    MissingField {
        /// Human-readable name of the missing field.
        field: &'static str,
    },
    /// A line did not match the expected `<type>=<value>` shape.
    MalformedLine {
        /// 1-based line number within the source text.
        line: usize,
        /// The offending raw line.
        text: String,
    },
    /// A line had the right shape but an internally invalid value.
    InvalidValue {
        /// Name of the field whose value failed to parse.
        field: &'static str,
        /// The value that could not be interpreted.
        value: String,
    },
    /// A numeric field could not be parsed as an integer.
    InvalidNumber {
        /// Name of the numeric field.
        field: &'static str,
        /// The raw token that was not a valid number.
        value: String,
    },
    /// An attribute that must appear inside a media section was found at the
    /// session level, or vice versa.
    MisplacedAttribute {
        /// The attribute name.
        name: String,
    },
    /// The document was structurally valid SDP but violated a profile rule
    /// required for the requested operation (for example building an answer).
    Profile {
        /// Description of the violated rule.
        reason: String,
    },
    #[allow(missing_docs)]
    ForbiddenCharacter { line_type: char, character: char },
}

impl SdpError {
    pub(crate) fn invalid(field: &'static str, value: impl Into<String>) -> Self {
        Self::InvalidValue {
            field,
            value: value.into(),
        }
    }

    pub(crate) fn number(field: &'static str, value: impl Into<String>) -> Self {
        Self::InvalidNumber {
            field,
            value: value.into(),
        }
    }

    pub(crate) fn profile(reason: impl Into<String>) -> Self {
        Self::Profile {
            reason: reason.into(),
        }
    }
}

impl fmt::Display for SdpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingField { field } => {
                write!(f, "missing required SDP field: {field}")
            }
            Self::MalformedLine { line, text } => {
                write!(f, "malformed SDP line {line}: {text:?}")
            }
            Self::InvalidValue { field, value } => {
                write!(f, "invalid value for {field}: {value:?}")
            }
            Self::InvalidNumber { field, value } => {
                write!(f, "invalid number for {field}: {value:?}")
            }
            Self::MisplacedAttribute { name } => {
                write!(f, "attribute {name:?} is not valid in this context")
            }
            Self::Profile { reason } => write!(f, "SDP profile violation: {reason}"),
            Self::ForbiddenCharacter {
                line_type,
                character,
            } => write!(
                f,
                "SDP {line_type}= value contains forbidden character {character:?}"
            ),
        }
    }
}

impl std::error::Error for SdpError {}

/// Convenience alias for results returned by this crate.
pub type Result<T> = std::result::Result<T, SdpError>;
