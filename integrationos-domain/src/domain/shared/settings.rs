use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub parse_webhook_body: bool,
    pub show_secret: bool,
    pub allow_custom_events: bool,
    pub oauth: bool,
}
