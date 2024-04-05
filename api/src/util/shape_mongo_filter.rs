use axum::extract::Query;
use http::HeaderMap;
use integrationos_domain::common::event_access::EventAccess;
use mongodb::bson::{doc, Document};
use std::{collections::BTreeMap, sync::Arc};

pub const DELETED_STR: &str = "deleted";
const OWNERSHIP_STR: &str = "ownership.buildableId";
const ENVIRONMENT_STR: &str = "environment";
const DUAL_ENVIRONMENT_HEADER: &str = "x-integrationos-show-all-environments";
const LIMIT_STR: &str = "limit";
const SKIP_STR: &str = "skip";

#[derive(Debug, Clone)]
pub struct MongoQuery {
    pub filter: Document,
    pub skip: u64,
    pub limit: u64,
}

pub fn shape_mongo_filter(
    query: Option<Query<BTreeMap<String, String>>>,
    event_access: Option<Arc<EventAccess>>,
    headers: Option<HeaderMap>,
) -> MongoQuery {
    let mut filter = doc! {};
    let mut skip = 0;
    let mut limit = 20;

    if let Some(q) = query {
        for (key, value) in q.0.iter() {
            if key == LIMIT_STR {
                limit = value.parse().unwrap_or(20);
            } else if key == SKIP_STR {
                skip = value.parse().unwrap_or(0);
            } else {
                match value.as_str() {
                    "true" => filter.insert(key, true),
                    "false" => filter.insert(key, false),
                    _ => filter.insert(key, value.clone()),
                };
            }
        }
    }

    filter.insert(DELETED_STR, false);

    if let Some(event_access) = event_access {
        filter.insert(OWNERSHIP_STR, event_access.ownership.id.to_string());
        filter.insert(ENVIRONMENT_STR, event_access.environment.to_string());

        if let Some(headers) = headers {
            if let Some(show_dual_envs) = headers.get(DUAL_ENVIRONMENT_HEADER) {
                if show_dual_envs == "true" {
                    filter.remove(ENVIRONMENT_STR);
                }
            }
        }
    }

    MongoQuery {
        filter,
        limit,
        skip,
    }
}

#[cfg(test)]
mod test {
    use std::{collections::BTreeMap, sync::Arc};

    use axum::extract::Query;
    use http::HeaderMap;
    use integrationos_domain::{
        common::{
            connection_definition::{ConnectionDefinitionType, Paths},
            environment::Environment,
            event_access::EventAccess,
            ownership::Ownership,
            record_metadata::RecordMetadata,
        },
        id::{prefix::IdPrefix, Id},
    };

    use crate::util::shape_mongo_filter::{
        MongoQuery, DELETED_STR, DUAL_ENVIRONMENT_HEADER, ENVIRONMENT_STR, LIMIT_STR,
        OWNERSHIP_STR, SKIP_STR,
    };

    use super::shape_mongo_filter;

    #[test]
    fn test_shape_mongo_filter() {
        let params = BTreeMap::from([
            (DELETED_STR.to_string(), "true".to_string()),
            (OWNERSHIP_STR.to_string(), "foo".to_string()),
            (ENVIRONMENT_STR.to_string(), "bar".to_string()),
            (SKIP_STR.to_string(), "10".to_string()),
            (LIMIT_STR.to_string(), "10".to_string()),
        ]);

        let MongoQuery {
            filter: mut doc,
            skip,
            limit,
        } = shape_mongo_filter(Some(Query(params.clone())), None, None);
        assert_eq!(doc.get_str(OWNERSHIP_STR).unwrap(), "foo");
        assert_eq!(doc.get_str(ENVIRONMENT_STR).unwrap(), "bar");
        assert!(!doc.get_bool(DELETED_STR).unwrap());
        assert_eq!(limit, 10);
        assert_eq!(skip, 10);

        doc.insert(DELETED_STR, true);
        assert!(doc.get_bool(DELETED_STR).unwrap());

        let event_access = Arc::new(EventAccess {
            id: Id::now(IdPrefix::EventAccess),
            name: "name".to_string(),
            key: "key".to_string(),
            namespace: "default".to_string(),
            platform: "stripe".to_string(),
            r#type: ConnectionDefinitionType::Api,
            group: "group".to_string(),
            ownership: Ownership::new("baz".to_string()),
            paths: Paths::default(),
            access_key: "access_key".to_string(),
            environment: Environment::Test,
            record_metadata: RecordMetadata::default(),
            throughput: 1000,
        });

        let MongoQuery { filter: doc, .. } =
            shape_mongo_filter(Some(Query(params)), Some(event_access), None);
        assert_eq!(doc.get_str(OWNERSHIP_STR).unwrap(), "baz");
        assert_eq!(doc.get_str(ENVIRONMENT_STR).unwrap(), "test");
    }

    #[test]
    fn requesting_dual_environments() {
        let params = BTreeMap::from([
            (DELETED_STR.to_string(), "true".to_string()),
            ("ownership.buildableId".to_string(), "foo".to_string()),
            ("environment".to_string(), "bar".to_string()),
        ]);

        let mut headers = HeaderMap::new();
        headers.insert(DUAL_ENVIRONMENT_HEADER, "true".parse().unwrap());

        let event_access = Arc::new(EventAccess {
            id: Id::now(IdPrefix::EventAccess),
            name: "name".to_string(),
            key: "key".to_string(),
            namespace: "default".to_string(),
            platform: "stripe".to_string(),
            r#type: ConnectionDefinitionType::Api,
            group: "group".to_string(),
            ownership: Ownership::new("baz".to_string()),
            paths: Paths::default(),
            access_key: "access_key".to_string(),
            environment: Environment::Test,
            record_metadata: RecordMetadata::default(),
            throughput: 1000,
        });

        let MongoQuery { filter: doc, .. } = shape_mongo_filter(
            Some(Query(params.clone())),
            Some(event_access),
            Some(headers),
        );

        assert!(!doc.contains_key(ENVIRONMENT_STR));
    }
}
