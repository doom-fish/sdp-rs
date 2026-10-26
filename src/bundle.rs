//! SDP grouping framework (RFC 5888) with the BUNDLE semantics (RFC 8843).
//!
//! A group line associates several media sections by their `mid` identifiers:
//!
//! ```text
//! a=group:BUNDLE 0 1 2
//! ```

use crate::error::{Result, SdpError};
use std::fmt;

/// Well-known `a=group:` semantics token.
pub const BUNDLE: &str = "BUNDLE";

/// A parsed `a=group:` attribute value: a semantics token plus the `mid`
/// identifiers it groups together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    /// Semantics token (`BUNDLE`, `LS`, `FID`, ...). Stored verbatim.
    pub semantics: String,
    /// The grouped `mid` identifiers, in order.
    pub mids: Vec<String>,
}

impl Group {
    /// Construct a BUNDLE group from a list of `mid` identifiers.
    #[must_use]
    pub fn bundle<I, S>(mids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            semantics: BUNDLE.to_owned(),
            mids: mids.into_iter().map(Into::into).collect(),
        }
    }

    /// Returns `true` if this is a BUNDLE group.
    #[must_use]
    pub fn is_bundle(&self) -> bool {
        self.semantics == BUNDLE
    }

    /// Parse the value portion of an `a=group:` line.
    ///
    /// # Errors
    /// Returns an error if the semantics token is missing.
    pub fn parse(value: &str) -> Result<Self> {
        let mut parts = value.split_whitespace();
        let semantics = parts
            .next()
            .ok_or_else(|| SdpError::invalid("group", value))?
            .to_owned();
        let mids = parts.map(ToOwned::to_owned).collect();
        Ok(Self { semantics, mids })
    }
}

impl fmt::Display for Group {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.semantics)?;
        for mid in &self.mids {
            write!(f, " {mid}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundle_round_trip() {
        let g = Group::parse("BUNDLE 0 1 2").expect("parse");
        assert!(g.is_bundle());
        assert_eq!(g.mids, vec!["0", "1", "2"]);
        assert_eq!(g.to_string(), "BUNDLE 0 1 2");
    }

    #[test]
    fn empty_bundle() {
        let g = Group::parse("BUNDLE").expect("parse");
        assert!(g.mids.is_empty());
        assert_eq!(g.to_string(), "BUNDLE");
    }
}
