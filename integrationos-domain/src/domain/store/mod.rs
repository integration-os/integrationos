pub mod cursor;

use bson::doc;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::str::FromStr;

macro_rules! generate_stores {
    ($($name:tt, $str:expr),+) => {
        #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
        pub enum Store {
            $($name),+
        }

        impl Display for Store {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                let store = match self {
                    $(Store::$name => $str),+
                };

                write!(f, "{store}")
            }
        }

        impl FromStr for Store {
            type Err = String;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                $(
                    if s == $str {
                        return Ok(Store::$name);
                    }
                )+

                Err(format!("Invalid store name: {}", s))
            }
        }
    };
}

generate_stores!(
    Integrations,
    "integrations",
    MicroServices,
    "microservices",
    Events,
    "external-events",
    EventAccess,
    "event-access",
    IntegrationDefinitions,
    "integration-definitions",
    Pipelines,
    "pipelines",
    Jobs,
    "jobs",
    Stages,
    "stages",
    Cursors,
    "cursors",
    Messages,
    "messages",
    Metrics,
    "system-stats",
    CommonModels,
    "common-models",
    CommonEnums,
    "common-enums",
    Platforms,
    "platforms",
    PlatformPages,
    "platform-pages",
    Connections,
    "connections",
    PublicConnectionDetails,
    "public-connection-details",
    Secrets,
    "secrets",
    Settings,
    "settings",
    EmbedTokens,
    "embed-tokens",
    Sessions,
    "sessions",
    ConnectionModelDefinitions,
    "connection-model-definitions",
    ConnectionOAuthDefinitions,
    "connection-oauth-definitions",
    Store,
    "store",
    Archives,
    "archives",
    ConnectionDefinitions,
    "connection-definitions",
    ConnectionModelSchemas,
    "connection-model-schema",
    PublicConnectionModelSchemas,
    "connection-model-schema",
    Transactions,
    "event-transactions",
    Clients,
    "clients"
);
