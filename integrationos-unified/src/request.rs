use bson::doc;
use http::HeaderMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestCrudBorrowed<'a> {
    pub query_params: &'a HashMap<String, String>,
    #[serde(with = "http_serde_ext_ios::header_map", default)]
    pub headers: &'a HeaderMap,
    pub path_params: Option<PathParams<'a>>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PathParams<'a> {
    pub id: &'a str,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestCrud {
    pub query_params: Option<HashMap<String, String>>,
    #[serde(with = "http_serde_ext_ios::header_map", default)]
    pub headers: HeaderMap,
    pub path_params: Option<HashMap<String, String>>,
    pub body: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseCrudToMap<'a> {
    #[serde(with = "http_serde_ext_ios::header_map")]
    pub headers: &'a HeaderMap,
    pub pagination: Option<Value>,
    pub request: ResponseCrudToMapRequest<'a>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseCrudToMapRequest<'a> {
    pub query_params: &'a HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseCrud {
    pub pagination: Option<Value>,
}
