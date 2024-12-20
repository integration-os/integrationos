use anyhow::{anyhow, Result};
use http::HeaderMap;
use serde_json::json;
use std::collections::HashMap;

pub fn get_value_from_path(
    path: &mut String,
    headers: &HeaderMap,
    body: &[u8],
    query: &Option<HashMap<String, String>>,
) -> Result<String> {
    if path.len() < 2 || &path[0..2] != "_." {
        return Ok(path.to_owned());
    }

    let body = serde_json::from_slice::<serde_json::Value>(body)?;
    let headers =
        http_serde_ext_ios::header_map::serialize(headers, serde_json::value::Serializer)?;
    let mut obj = json!({
        "headers": headers,
        "body": body,
        "query": query,
    });

    for key in path.split('.').skip(1) {
        let temp = match obj.get(key) {
            Some(t) => t,
            None => {
                return Err(anyhow!("No value found for path: {}", path));
            }
        };
        obj = temp.clone();
    }

    Ok(obj
        .as_str()
        .map(|s| s.to_owned())
        .unwrap_or_else(|| obj.to_string()))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use http::HeaderMap;

    use super::*;

    #[test]
    fn test_stripe_signature() {
        let mut path = r#"_.headers.stripe-signature"#.to_owned();
        let headers = r#"{"content-type":"application/json; charset=utf-8","cache-control":"no-cache","user-agent":"Stripe/1.0 (+https://stripe.com/docs/webhooks)","accept":"*/*; q=0.5, application/xml","stripe-signature":"t=1689703968,v1=035b09d5fd7ddad1ba0a05798e7fa914ad704e50e39845eb2d03c2234d1fbb2a,v0=a78258146fc18af95b4bca66051fe7dae809a398ba524d10c0a972b26106d33e","host":"development-stream.event.dev","content-length":"1117","x-cloud-trace-context":"283401a42e9257773bfe4320acce8e17/319072271516621528","via":"1.1 google","x-forwarded-for":"35.154.171.200, 34.117.226.41","x-forwarded-proto":"https","connection":"Keep-Alive"}"#;
        let headers = http_serde_ext_ios::header_map::deserialize(
            &mut serde_json::Deserializer::from_str(headers),
        )
        .unwrap();
        let body = r#"{
            "id": "evt_1NVIOBSGVSOWoR3QvDwZ4VjP",
            "object": "event",
            "api_version": "2020-08-27",
            "created": 1689703967,
            "data": {
              "object": {
                "id": "cus_OHs8z1ZNSlvJ3r",
                "object": "customer",
                "address": null,
                "balance": 0,
                "created": 1689703967,
                "currency": null,
                "default_currency": null,
                "default_source": null,
                "delinquent": false,
                "description": null,
                "discount": null,
                "email": null,
                "invoice_prefix": "957E59A7",
                "invoice_settings": {
                  "custom_fields": null,
                  "default_payment_method": null,
                  "footer": null,
                  "rendering_options": null
                },
                "livemode": false,
                "metadata": {
                },
                "name": "Demo",
                "next_invoice_sequence": 1,
                "phone": null,
                "preferred_locales": [

                ],
                "shipping": null,
                "tax_exempt": "none",
                "test_clock": null
              }
            },
            "livemode": false,
            "pending_webhooks": 15,
            "request": {
              "id": "req_4JjE4wAaOiFbkq",
              "idempotency_key": "ef53b6c6-2bf1-45b6-9e7c-0c5c65c9d579"
            },
            "type": "customer.created"
          }"#;

        let res = get_value_from_path(&mut path, &headers, body.to_owned().as_bytes(), &None);
        assert_eq!(res.unwrap(), "t=1689703968,v1=035b09d5fd7ddad1ba0a05798e7fa914ad704e50e39845eb2d03c2234d1fbb2a,v0=a78258146fc18af95b4bca66051fe7dae809a398ba524d10c0a972b26106d33e");
    }

    #[test]
    fn test_get_value_from_path() {
        let mut path = "foo".to_owned();
        let mut headers = HeaderMap::new();
        headers.insert("quux", "quuz".parse().unwrap());
        let body = b"{}";
        let query = None;
        let name = get_value_from_path(&mut path, &headers, body, &query).unwrap();
        assert_eq!(name, "foo");

        let mut path = "_.foo".to_owned();
        assert!(get_value_from_path(&mut path, &headers, body, &query).is_err());
        let mut path = "_...".to_owned();
        assert!(get_value_from_path(&mut path, &headers, body, &query).is_err());

        let body = b"{\"foo\": \"bar\"}";

        let mut path = "_.body.foo".to_owned();
        let name = get_value_from_path(&mut path, &headers, body, &query).unwrap();
        assert_eq!(name, "bar");

        let mut path = "_.body.bar".to_owned();
        assert!(get_value_from_path(&mut path, &headers, body, &query).is_err());

        let mut query = HashMap::new();
        query.insert("baz".to_owned(), "qux".to_owned());
        let query = Some(query);
        let mut path = "_.query.baz".to_owned();
        let name = get_value_from_path(&mut path, &headers, body, &query).unwrap();
        assert_eq!(name, "qux");

        let mut path = "_.query.foo".to_owned();
        assert!(get_value_from_path(&mut path, &headers, body, &query).is_err());

        let mut path = "_.headers.quux".to_owned();
        let name = get_value_from_path(&mut path, &headers, body, &query).unwrap();
        assert_eq!(name, "quuz");

        let mut path = "_.headers.foo".to_owned();
        assert!(get_value_from_path(&mut path, &headers, body, &query).is_err());
    }
}
