//! Validated CLI inputs.
//!
//! RPC URLs frequently embed an API key, so the endpoint is parsed through
//! [`reliakit_primitives::HttpUrl`], held as a redacted [`reliakit_secret::Secret`],
//! and only exposed at the moment a request is made.

use eyre::{Result, eyre};
use reliakit_primitives::{HexString, HttpUrl};
use reliakit_secret::Secret;
use reliakit_validate::Validate;

/// A syntactically valid, redacted RPC endpoint.
pub struct RpcEndpoint {
    url: Secret<String>,
    https: bool,
}

impl RpcEndpoint {
    /// Validate `raw` as an HTTP(S) URL and wrap it as a secret.
    pub fn parse(raw: &str) -> Result<Self> {
        let url = HttpUrl::new(raw.to_owned()).map_err(|e| eyre!("invalid RPC URL: {e:?}"))?;
        let endpoint = RpcEndpoint {
            https: url.is_https(),
            url: Secret::from_string(url.into_inner()),
        };
        endpoint.validate().map_err(|e| eyre!("{e}"))?;
        Ok(endpoint)
    }

    /// The underlying URL, for passing to a provider. Avoid printing it.
    pub fn expose(&self) -> &str {
        self.url.expose_str()
    }

    /// Whether the endpoint uses TLS.
    pub fn is_https(&self) -> bool {
        self.https
    }

    /// Replace any occurrence of the (secret) URL in `text` with a placeholder,
    /// so transport errors that echo the endpoint do not leak an embedded key.
    pub fn scrub(&self, text: String) -> String {
        text.replace(self.expose(), "<redacted-rpc-url>")
    }
}

impl Validate for RpcEndpoint {
    type Error = String;

    fn validate(&self) -> std::result::Result<(), Self::Error> {
        if self.url.expose_str().is_empty() {
            return Err("RPC URL must not be empty".to_owned());
        }
        Ok(())
    }
}

/// Parse hex calldata (with or without a `0x` prefix) into bytes.
pub fn parse_calldata(raw: &str) -> Result<Vec<u8>> {
    let hex = HexString::new(raw.to_owned()).map_err(|e| eyre!("invalid hex calldata: {e:?}"))?;
    alloy::hex::decode(hex.hex_digits()).map_err(|e| eyre!("calldata is not valid hex: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_rejects_non_url() {
        assert!(RpcEndpoint::parse("not-a-url").is_err());
        assert!(RpcEndpoint::parse("").is_err());
    }

    #[test]
    fn parse_accepts_http_and_reports_tls() {
        assert!(!RpcEndpoint::parse("http://localhost:8545").unwrap().is_https());
        assert!(RpcEndpoint::parse("https://example.com").unwrap().is_https());
    }

    #[test]
    fn scrub_removes_the_url() {
        let endpoint = RpcEndpoint::parse("https://node.example/key=abc123").unwrap();
        let leaked = format!("connect failed for {}", endpoint.expose());
        let scrubbed = endpoint.scrub(leaked);
        assert!(!scrubbed.contains("abc123"));
        assert!(scrubbed.contains("<redacted-rpc-url>"));
    }

    #[test]
    fn parse_calldata_accepts_prefixed_and_bare_hex() {
        assert_eq!(parse_calldata("0x01ff").unwrap(), vec![0x01, 0xff]);
        assert_eq!(parse_calldata("01ff").unwrap(), vec![0x01, 0xff]);
    }

    #[test]
    fn parse_calldata_rejects_odd_length() {
        assert!(parse_calldata("0xabc").is_err());
    }
}
