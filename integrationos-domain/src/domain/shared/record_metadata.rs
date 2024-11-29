use chrono::prelude::*;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase", default)]
pub struct RecordMetadata {
    pub created_at: i64,
    pub updated_at: i64,
    pub updated: bool,
    #[cfg_attr(feature = "dummy", dummy(expr = "Version::new(1,0,0)"))]
    pub version: Version,
    pub last_modified_by: String,
    pub deleted: bool,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub change_log: BTreeMap<String, i64>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    pub active: bool,
    pub deprecated: bool,
}

impl Default for RecordMetadata {
    fn default() -> Self {
        let now = Utc::now().timestamp_millis();
        RecordMetadata {
            created_at: now,
            updated_at: now,
            updated: false,
            version: Version::new(1, 0, 0),
            last_modified_by: String::from("system"),
            deleted: false,
            change_log: BTreeMap::new(),
            tags: Vec::new(),
            active: true,
            deprecated: false,
        }
    }
}

impl RecordMetadata {
    // Mark record as updated
    pub fn mark_updated(&mut self, modifier: &str) {
        let now = Utc::now().timestamp_millis();
        self.updated = true;
        self.updated_at = now;
        self.version = Version::new(
            self.version.major,
            self.version.minor,
            self.version.patch + 1,
        );
        self.last_modified_by = modifier.to_string();
        let log_entry = format!("Updated by {}", modifier);
        self.change_log.insert(log_entry, now);
    }

    // Mark record as soft deleted
    pub fn mark_deleted(&mut self, modifier: &str) {
        let now = Utc::now().timestamp_millis();
        self.deleted = true;
        let log_entry = format!("Marked as deleted by {}", modifier);
        self.change_log.insert(log_entry, now);
    }

    // Mark record as soft undeleted
    pub fn mark_undeleted(&mut self, modifier: &str) {
        let now = Utc::now().timestamp_millis();
        self.deleted = false;
        let log_entry = format!("Marked as undeleted by {}", modifier);
        self.change_log.insert(log_entry, now);
    }

    // Mark record as inactive
    pub fn mark_inactive(&mut self, modifier: &str) {
        let now = Utc::now().timestamp_millis();
        self.active = false;
        let log_entry = format!("Marked as inactive by {}", modifier);
        self.change_log.insert(log_entry, now);
    }

    // Mark record as deprecated
    pub fn mark_deprecated(&mut self, modifier: &str) {
        let now = Utc::now().timestamp_millis();
        self.deprecated = true;
        let log_entry = format!("Marked as deprecated by {}", modifier);
        self.change_log.insert(log_entry, now);
    }

    // Add tag to record
    pub fn add_tag(&mut self, tag: &str) {
        self.tags.push(tag.to_string());
    }

    pub fn test() -> Self {
        let epoch = Utc
            .timestamp_opt(0, 0)
            .single()
            .expect("Failed to get UTC time");
        Self {
            created_at: epoch.timestamp_millis(),
            updated_at: epoch.timestamp_millis(),
            updated: false,
            version: Version::new(1, 0, 0),
            last_modified_by: "system".to_string(),
            deleted: false,
            ..Default::default()
        }
    }
}
