use chrono::{DateTime, Datelike, Utc};
use http::HeaderValue;
use integrationos_domain::{
    destination::Action, event_access::EventAccess, ownership::Ownership, Connection,
};
use segment::message::{Track, User};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

pub const TOTAL_KEY: &str = "total";
pub const DAILY_KEY: &str = "daily";
pub const MONTHLY_KEY: &str = "monthly";
pub const PLATFORMS_KEY: &str = "platforms";
pub const CREATED_AT_KEY: &str = "createdAt";

#[derive(Debug, Clone, strum::Display, Deserialize)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum MetricType {
    Passthrough(Arc<Connection>),
    Unified(Arc<Connection>),
    RateLimited(
        Arc<EventAccess>,
        #[serde(with = "http_serde_ext::header_value::option")] Option<HeaderValue>,
    ),
}

impl MetricType {
    pub fn event_name(&self) -> &'static str {
        use MetricType::*;
        match self {
            Passthrough(_) => "Called Passthrough API",
            Unified(_) => "Called Unified API",
            RateLimited(_, _) => "Reached Rate Limit",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Metric {
    pub metric_type: MetricType,
    pub date: DateTime<Utc>,
    pub action: Option<Action>,
}

impl Metric {
    pub fn passthrough(connection: Arc<Connection>) -> Self {
        Self {
            metric_type: MetricType::Passthrough(connection),
            date: Utc::now(),
            action: None,
        }
    }

    pub fn unified(connection: Arc<Connection>, action: Action) -> Self {
        Self {
            metric_type: MetricType::Unified(connection),
            date: Utc::now(),
            action: Some(action),
        }
    }

    pub fn rate_limited(event_access: Arc<EventAccess>, key: Option<HeaderValue>) -> Self {
        Self {
            metric_type: MetricType::RateLimited(event_access, key),
            date: Utc::now(),
            action: None,
        }
    }

    pub fn ownership(&self) -> &Ownership {
        use MetricType::*;
        match &self.metric_type {
            Passthrough(c) => &c.ownership,
            Unified(c) => &c.ownership,
            RateLimited(e, _) => &e.ownership,
        }
    }

    fn platform(&self) -> &str {
        use MetricType::*;
        match &self.metric_type {
            Passthrough(c) => &c.platform,
            Unified(c) => &c.platform,
            RateLimited(e, _) => &e.platform,
        }
    }

    pub fn update_doc(&self) -> bson::Document {
        let platform = self.platform();
        let metric_type = &self.metric_type;
        let day = self.date.day();
        let month = self.date.month();
        let year = self.date.year();
        let daily_key = format!("{year}-{month:02}-{day:02}");
        let monthly_key = format!("{year}-{month:02}");
        bson::doc! {
            "$inc": {
                format!("{metric_type}.{TOTAL_KEY}"): 1,
                format!("{metric_type}.{PLATFORMS_KEY}.{platform}.{TOTAL_KEY}"): 1,
                format!("{metric_type}.{DAILY_KEY}.{daily_key}"): 1,
                format!("{metric_type}.{PLATFORMS_KEY}.{platform}.{DAILY_KEY}.{daily_key}"): 1,
                format!("{metric_type}.{MONTHLY_KEY}.{monthly_key}"): 1,
                format!("{metric_type}.{PLATFORMS_KEY}.{platform}.{MONTHLY_KEY}.{monthly_key}"): 1,
            },
            "$setOnInsert": {
                CREATED_AT_KEY: self.date.timestamp_millis()
            }
        }
    }

    pub fn segment_track(&self) -> Track {
        use MetricType::*;

        match &self.metric_type {
            Unified(conn) => Track {
                user: User::UserId {
                    user_id: self
                        .ownership()
                        .clone()
                        .user_id
                        .unwrap_or(self.ownership().id.to_string()),
                },
                event: self.metric_type.event_name().to_owned(),
                properties: json!({
                    "connectionDefinitionId": conn.id.to_string(),
                    "environment": conn.environment,
                    "key": &conn.key,
                    "platform": self.platform(),
                    "platformVersion": &conn.platform_version,
                    "clientId": self.ownership().client_id,
                    "version": &conn.record_metadata.version,
                    "commonModel": self.action.as_ref().map(|a| a.name()),
                    "action": self.action.as_ref().map(|a| a.action()),
                }),
                ..Default::default()
            },
            Passthrough(conn) => Track {
                user: User::UserId {
                    user_id: self
                        .ownership()
                        .clone()
                        .user_id
                        .unwrap_or(self.ownership().id.to_string()),
                },
                event: self.metric_type.event_name().to_owned(),
                properties: json!({
                    "connectionDefinitionId": conn.id.to_string(),
                    "environment": conn.environment,
                    "key": &conn.key,
                    "platform": self.platform(),
                    "platformVersion": &conn.platform_version,
                    "clientId": self.ownership().client_id,
                    "version": &conn.record_metadata.version
                }),
                ..Default::default()
            },
            RateLimited(event_access, key) => Track {
                user: User::UserId {
                    user_id: self
                        .ownership()
                        .clone()
                        .user_id
                        .unwrap_or(self.ownership().id.to_string()),
                },
                event: self.metric_type.event_name().to_owned(),
                properties: json!({
                    "environment": event_access.environment,
                    "key": key.as_ref().map(|k| k.to_str().unwrap_or_default().to_string()),
                    "platform": self.platform(),
                    "clientId": self.ownership().client_id,
                    "version": &event_access.record_metadata.version
                }),
                ..Default::default()
            },
        }
    }
}
