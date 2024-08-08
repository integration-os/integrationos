use anyhow::{Context, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::prelude::*;
use core::str;
use hmac::{Hmac, Mac};
use http::Method;
use indexmap::IndexMap;
use percent_encoding::{AsciiSet, PercentEncode, NON_ALPHANUMERIC};
use rand::{thread_rng, RngCore};
use reqwest::Url;
use sha1::Sha1;
use sha2::{Sha256, Sha512};
use std::{
    borrow::Cow,
    fmt,
    time::{SystemTime, UNIX_EPOCH},
};

const NONCE_LEN: usize = 12;
const OAUTH_CALLBACK: &str = "oauth_callback";
const OAUTH_VERIFIER: &str = "oauth_verifier";
const OAUTH_CONSUMER_KEY: &str = "oauth_consumer_key";
const OAUTH_NONCE: &str = "oauth_nonce";
const OAUTH_SIGNATURE: &str = "oauth_signature";
const OAUTH_SIGNATURE_METHOD: &str = "oauth_signature_method";
const OAUTH_TIMESTAMP: &str = "oauth_timestamp";
const OAUTH_TOKEN: &str = "oauth_token";
const OAUTH_VERSION: &str = "oauth_version";
const HMAC_LENGTH_ERROR: &str = "HMAC has no key length restrictions";

const EXCLUDE: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

fn percent_encode<T: ?Sized + AsRef<[u8]>>(data: &T) -> PercentEncode<'_> {
    percent_encoding::percent_encode(data.as_ref(), EXCLUDE)
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SignatureMethod {
    HmacSha1,
    HmacSha256,
    HmacSha512,
    PlainText,
}

impl fmt::Display for SignatureMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HmacSha1 => write!(f, "HMAC-SHA1"),
            Self::HmacSha256 => write!(f, "HMAC-SHA256"),
            Self::HmacSha512 => write!(f, "HMAC-SHA512"),
            Self::PlainText => write!(f, "PLAINTEXT"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignableRequest {
    pub method: Method,
    pub uri: Url,
    pub parameters: IndexMap<String, String>,
}

impl SignableRequest {
    fn as_bytes(&self) -> Result<Cow<[u8]>> {
        let method = self.method.to_string();
        let normalized_uri = {
            let mut url = self.uri.clone();
            if let Some(host) = url.host_str() {
                url.set_host(Some(&host.to_lowercase()))
                    .context("OAuth 1.0 URI lowercasing shouldn't change host validity")?;
            }
            url.set_fragment(None);
            url.set_query(None);
            url
        };

        let encoded_url = percent_encode(normalized_uri.as_str());
        let encoded_url_params = encode_url_parameters(&self.parameters);
        let encoded_params = percent_encode(&encoded_url_params);

        let result = format!("{}&{}&{}", method, encoded_url, encoded_params);

        Ok(Cow::Owned(result.into_bytes()))
    }

    fn sorted_parameters(&self) -> SignableRequest {
        let mut params = self.parameters.clone();
        params.sort_keys();
        SignableRequest {
            method: self.method.clone(),
            uri: self.uri.clone(),
            parameters: params,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SigningKey {
    pub client_secret: String,
    pub token_secret: Option<String>,
}

impl fmt::Display for SigningKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(token_secret) = &self.token_secret {
            write!(
                f,
                "{}&{}",
                percent_encode(&self.client_secret),
                percent_encode(&token_secret)
            )
        } else {
            write!(f, "{}&", percent_encode(&self.client_secret))
        }
    }
}

impl SignatureMethod {
    pub fn sign(self, data: &SignableRequest, key: &SigningKey) -> Result<String> {
        let key = key.to_string();

        match self {
            Self::HmacSha1 => {
                let mut mac =
                    Hmac::<Sha1>::new_from_slice(key.as_bytes()).context(HMAC_LENGTH_ERROR)?;
                mac.update(&data.sorted_parameters().as_bytes()?);
                let result = mac.finalize().into_bytes();
                Ok(BASE64_STANDARD.encode(result))
            }
            Self::HmacSha256 => {
                let mut mac =
                    Hmac::<Sha256>::new_from_slice(key.as_bytes()).context(HMAC_LENGTH_ERROR)?;
                mac.update(&data.sorted_parameters().as_bytes()?);
                let result = mac.finalize().into_bytes();
                Ok(BASE64_STANDARD.encode(result))
            }
            Self::HmacSha512 => {
                let mut mac =
                    Hmac::<Sha512>::new_from_slice(key.as_bytes()).context(HMAC_LENGTH_ERROR)?;
                mac.update(&data.sorted_parameters().as_bytes()?);
                let result = mac.finalize().into_bytes();
                Ok(BASE64_STANDARD.encode(result))
            }
            Self::PlainText => Ok(key),
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Nonce(pub String);
impl Nonce {
    pub fn generate() -> Result<Nonce> {
        let mut rng = thread_rng();
        let mut rand = [0_u8; NONCE_LEN * 3 / 4];
        rng.fill_bytes(&mut rand);

        let i = rand.iter().position(|&b| b != 0).unwrap_or(rand.len());
        let rand = &rand[i..];

        let mut buf = [0u8; NONCE_LEN];
        let len = URL_SAFE_NO_PAD
            .encode_slice(rand, &mut buf)
            .context("Failed to encode nonce to Base64")?;

        let nonce_str = str::from_utf8(&buf[..len])
            .context("Failed to convert nonce bytes to UTF-8")?
            .to_string();

        Ok(Nonce(nonce_str))
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OAuthData {
    pub client_id: String,
    pub token: Option<String>,
    pub signature_method: SignatureMethod,
    pub nonce: Nonce,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AuthorizationType {
    RequestToken { callback: String },
    AccessToken { verifier: String },
    Request,
}

fn timestamp() -> Result<u64> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("Bad system time!")?
        .as_secs())
}

impl OAuthData {
    pub fn authorization(
        &self,
        mut req: SignableRequest,
        typ: AuthorizationType,
        key: &SigningKey,
        realm: Option<String>,
    ) -> Result<String> {
        req.parameters.extend(self.parameters()?);

        match typ {
            AuthorizationType::RequestToken { callback } => {
                req.parameters.insert(OAUTH_CALLBACK.into(), callback);
            }
            AuthorizationType::AccessToken { verifier } => {
                req.parameters.insert(OAUTH_VERIFIER.into(), verifier);
            }
            AuthorizationType::Request => {}
        }

        let signature = self.signature_method.sign(&req, key)?;
        req.parameters.insert(OAUTH_SIGNATURE.into(), signature);

        // Only include OAuth parameters in the Authorization header
        let oauth_params: IndexMap<_, _> = req
            .parameters
            .iter()
            .filter(|(k, _)| k.starts_with("oauth_"))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        Ok(match realm {
            Some(realm) => format!(
                "OAuth realm=\"{}\",{}",
                realm,
                encode_auth_header(&oauth_params)
            ),
            None => format!("OAuth {}", encode_auth_header(&oauth_params)),
        })
    }

    pub fn parameters(&self) -> Result<IndexMap<String, String>> {
        let mut params = IndexMap::new();

        params.insert(OAUTH_CONSUMER_KEY.into(), self.client_id.clone());
        if let Some(token) = &self.token {
            params.insert(OAUTH_TOKEN.into(), token.clone());
        }
        params.insert(
            OAUTH_SIGNATURE_METHOD.into(),
            self.signature_method.to_string(),
        );
        params.insert(OAUTH_TIMESTAMP.into(), timestamp()?.to_string());
        params.insert(OAUTH_NONCE.into(), self.nonce.0.clone());
        params.insert(OAUTH_VERSION.into(), "1.0".into());
        Ok(params)
    }
}
fn encode_auth_header(req: &IndexMap<String, String>) -> String {
    req.iter()
        .map(|(k, v)| format!(r#"{}="{}""#, percent_encode(k), percent_encode(v)))
        .collect::<Vec<String>>()
        .join(",")
}

fn encode_url_parameters(params: &IndexMap<String, String>) -> String {
    params
        .iter()
        .map(|(k, v)| format!("{}={}", percent_encode(k), percent_encode(v)))
        .collect::<Vec<String>>()
        .join("&")
}
