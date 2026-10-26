//! ICE attributes (RFC 8839): `ice-ufrag`, `ice-pwd`, `ice-options`, and the
//! full `candidate` line grammar.

use crate::error::{Result, SdpError};
use std::fmt;

/// A parsed `a=candidate:` line (the text after the `candidate:` prefix).
///
/// Grammar (RFC 8839 §5.1):
/// ```text
/// <foundation> <component-id> <transport> <priority> <addr> <port> typ <type>
///     [raddr <addr> rport <port>] *(<name> <value>)
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    /// Foundation string identifying related candidates.
    pub foundation: String,
    /// RTP/RTCP component id (1 = RTP, 2 = RTCP).
    pub component: u32,
    /// Transport protocol token (`udp`, `tcp`, ...). Stored verbatim.
    pub transport: String,
    /// Candidate priority.
    pub priority: u32,
    /// Connection address (IPv4, IPv6, or FQDN).
    pub address: String,
    /// Transport port.
    pub port: u16,
    /// Candidate type token (`host`, `srflx`, `prflx`, `relay`).
    pub kind: String,
    /// Related address (`raddr`), present for reflexive/relay candidates.
    pub rel_addr: Option<String>,
    /// Related port (`rport`).
    pub rel_port: Option<u16>,
    /// Remaining `name value` extension pairs, preserved in order.
    pub extensions: Vec<(String, String)>,
}

impl Candidate {
    /// Parse the value portion of an `a=candidate:` line.
    ///
    /// # Errors
    /// Returns an error if any mandatory token is missing, if `typ` is absent,
    /// if an extension name has no value, or if a numeric field cannot be
    /// parsed.
    #[allow(clippy::missing_panics_doc)]
    pub fn parse(value: &str) -> Result<Self> {
        let tokens: Vec<&str> = value.split_whitespace().collect();
        if tokens.len() < 8 {
            return Err(SdpError::invalid("candidate", value));
        }
        let foundation = tokens[0].to_owned();
        let component = tokens[1]
            .parse()
            .map_err(|_| SdpError::number("candidate.component", tokens[1]))?;
        let transport = tokens[2].to_owned();
        let priority = tokens[3]
            .parse()
            .map_err(|_| SdpError::number("candidate.priority", tokens[3]))?;
        let address = tokens[4].to_owned();
        let port = tokens[5]
            .parse()
            .map_err(|_| SdpError::number("candidate.port", tokens[5]))?;
        if tokens[6] != "typ" {
            return Err(SdpError::invalid("candidate", value));
        }
        let kind = tokens[7].to_owned();

        let mut rel_addr = None;
        let mut rel_port = None;
        let mut extensions = Vec::new();
        let mut i = 8;
        while i < tokens.len() {
            let name = tokens[i];
            let val = *tokens
                .get(i + 1)
                .ok_or_else(|| SdpError::invalid("candidate", value))?;
            match name {
                "raddr" => rel_addr = Some(val.to_owned()),
                "rport" => {
                    rel_port = Some(
                        val.parse()
                            .map_err(|_| SdpError::number("candidate.rport", val))?,
                    );
                }
                _ => extensions.push((name.to_owned(), val.to_owned())),
            }
            i += 2;
        }

        Ok(Self {
            foundation,
            component,
            transport,
            priority,
            address,
            port,
            kind,
            rel_addr,
            rel_port,
            extensions,
        })
    }
}

impl fmt::Display for Candidate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} {} {} {} {} typ {}",
            self.foundation,
            self.component,
            self.transport,
            self.priority,
            self.address,
            self.port,
            self.kind
        )?;
        if let Some(addr) = &self.rel_addr {
            write!(f, " raddr {addr}")?;
        }
        if let Some(port) = self.rel_port {
            write!(f, " rport {port}")?;
        }
        for (name, value) in &self.extensions {
            write!(f, " {name} {value}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_candidate_round_trip() {
        let value = "1 1 udp 2122260223 192.0.2.10 50000 typ host generation 0";
        let c = Candidate::parse(value).expect("parse");
        assert_eq!(c.component, 1);
        assert_eq!(c.port, 50000);
        assert_eq!(c.kind, "host");
        assert_eq!(c.extensions, vec![("generation".into(), "0".into())]);
        assert_eq!(c.to_string(), value);
    }

    #[test]
    fn srflx_candidate_round_trip() {
        let value =
            "2 1 udp 1686052607 203.0.113.5 60000 typ srflx raddr 192.0.2.10 rport 50000";
        let c = Candidate::parse(value).expect("parse");
        assert_eq!(c.kind, "srflx");
        assert_eq!(c.rel_addr.as_deref(), Some("192.0.2.10"));
        assert_eq!(c.rel_port, Some(50000));
        assert_eq!(c.to_string(), value);
    }

    #[test]
    fn rejects_dangling_extension_name() {
        assert!(Candidate::parse("1 1 udp 1 192.0.2.1 5 typ host generation").is_err());
        assert!(Candidate::parse("1 1 udp 1 192.0.2.1 5 typ srflx raddr 192.0.2.2 rport").is_err());
        assert!(
            Candidate::parse("1 1 udp 1 192.0.2.1 5 typ host generation 0 network-id").is_err()
        );
        let ok = Candidate::parse("1 1 udp 1 192.0.2.1 5 typ host generation 0 network-id 1")
            .expect("paired extensions");
        assert_eq!(
            ok.extensions,
            vec![
                ("generation".to_owned(), "0".to_owned()),
                ("network-id".to_owned(), "1".to_owned())
            ]
        );
    }

    #[test]
    fn rejects_missing_typ() {
        assert!(Candidate::parse("1 1 udp 1 1.2.3.4 5 host").is_err());
        assert!(Candidate::parse("too short").is_err());
    }
}
