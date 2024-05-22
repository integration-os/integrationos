use serde::{Deserialize, Serialize};

use super::extractor::HttpExtractor;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase", tag = "_type")]
pub enum Middleware {
    #[serde(rename = "extractor::http")]
    HttpExtractor(HttpExtractor),
    Transformer {
        language: String,
        code: String,
    },
}
