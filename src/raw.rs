//! A raw JSON-RPC path built and parsed entirely with [`reliakit_json`].
//!
//! Unlike [`crate::rpc`] (which goes through Alloy's serde-based provider), this
//! module owns the transport: it constructs the JSON-RPC envelope with
//! reliakit-json, POSTs it over HTTP, and parses the reply back into a
//! [`JsonValue`] — a showcase that reliakit-json can drive an EVM node end to
//! end, no serde involved.

use eyre::{Result, eyre};
use reliakit_json::{JsonNumber, JsonObject, JsonValue, parse_str, to_compact_string};

/// A minimal JSON-RPC client over HTTP.
pub struct RawClient {
    http: reqwest::Client,
    url: String,
}

impl RawClient {
    pub fn new(url: &str) -> Self {
        RawClient {
            http: reqwest::Client::new(),
            url: url.to_owned(),
        }
    }

    /// Call `method` and return its `result`, building the request and parsing
    /// the response with reliakit-json.
    pub async fn call(&self, method: &str, params: Vec<JsonValue>) -> Result<JsonValue> {
        let body = request_body(method, params);

        let text = self
            .http
            .post(&self.url)
            .header("content-type", "application/json")
            .body(body)
            .send()
            .await?
            .text()
            .await?;

        let response = parse_str(&text).map_err(|e| eyre!("invalid JSON-RPC response: {e:?}"))?;
        let object = response
            .as_object()
            .ok_or_else(|| eyre!("JSON-RPC response was not an object"))?;
        if let Some(error) = object.get("error") {
            return Err(eyre!("RPC error: {}", to_compact_string(error)));
        }
        object
            .get("result")
            .cloned()
            .ok_or_else(|| eyre!("JSON-RPC response had no result"))
    }
}

/// Build a JSON-RPC request envelope with reliakit-json.
fn request_body(method: &str, params: Vec<JsonValue>) -> String {
    let mut request = JsonObject::new();
    request.insert("jsonrpc".to_owned(), JsonValue::String("2.0".to_owned()));
    request.insert(
        "id".to_owned(),
        JsonValue::Number(JsonNumber::new("1").expect("1 is a valid number")),
    );
    request.insert("method".to_owned(), JsonValue::String(method.to_owned()));
    request.insert("params".to_owned(), JsonValue::Array(params));
    to_compact_string(&JsonValue::Object(request))
}

/// Decode a `0x`-prefixed hex quantity from a JSON string value.
pub fn quantity(value: &JsonValue) -> Option<u64> {
    let text = value.as_str()?;
    u64::from_str_radix(text.trim_start_matches("0x"), 16).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_body_is_well_formed_json_rpc() {
        let body = request_body("eth_chainId", vec![]);
        let parsed = parse_str(&body).unwrap();
        let object = parsed.as_object().unwrap();
        assert_eq!(
            object.get("jsonrpc").and_then(JsonValue::as_str),
            Some("2.0")
        );
        assert_eq!(
            object.get("method").and_then(JsonValue::as_str),
            Some("eth_chainId")
        );
        assert!(
            object
                .get("params")
                .and_then(JsonValue::as_array)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn quantity_decodes_hex() {
        assert_eq!(quantity(&JsonValue::String("0x1".to_owned())), Some(1));
        assert_eq!(quantity(&JsonValue::String("0xff".to_owned())), Some(255));
        assert_eq!(quantity(&JsonValue::Bool(true)), None);
    }
}
