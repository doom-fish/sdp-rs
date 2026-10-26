use std::fmt;
use std::marker::PhantomData;

#[allow(missing_docs)]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FmtpParams<C> {
    params: Vec<(String, String)>,
    codec: PhantomData<C>,
}

impl<C> FmtpParams<C> {
    /// An empty parameter set.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            params: Vec::new(),
            codec: PhantomData,
        }
    }

    /// Parse a `;`-separated fmtp parameter string.
    ///
    /// Whitespace around the separator is tolerated. Parameters without a
    /// value (rare, but legal) are preserved with an empty value.
    #[must_use]
    pub fn parse(value: &str) -> Self {
        let mut params = Vec::new();
        for raw in value.split(';') {
            let token = raw.trim();
            if token.is_empty() {
                continue;
            }
            if let Some((k, v)) = token.split_once('=') {
                params.push((k.trim().to_owned(), v.trim().to_owned()));
            } else {
                params.push((token.to_owned(), String::new()));
            }
        }
        Self {
            params,
            codec: PhantomData,
        }
    }

    /// Look up a raw parameter value by key.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        self.params
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    /// Insert or replace a raw parameter, preserving position on replace.
    pub fn set(&mut self, key: &str, value: impl Into<String>) {
        let value = value.into();
        if let Some(slot) = self.params.iter_mut().find(|(k, _)| k == key) {
            slot.1 = value;
        } else {
            self.params.push((key.to_owned(), value));
        }
    }
}

impl<C> fmt::Display for FmtpParams<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, (k, v)) in self.params.iter().enumerate() {
            if i > 0 {
                f.write_str(";")?;
            }
            if v.is_empty() {
                f.write_str(k)?;
            } else {
                write!(f, "{k}={v}")?;
            }
        }
        Ok(())
    }
}
