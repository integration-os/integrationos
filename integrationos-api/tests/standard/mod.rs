use crate::checker::{CheckType, JsonChecker, JsonCheckerImpl};
use http::Method;
use integrationos_domain::{
    api_model_config::{ApiModelConfig, AuthMethod, ContentType, SamplesInput, SchemasInput},
    connection_model_definition::{
        ConnectionModelDefinition, CrudAction, CrudMapping, PlatformInfo, TestConnection,
    },
    connection_model_schema::{ConnectionModelSchema, Mappings, SchemaPaths},
    json_schema::JsonSchema,
    prefix::IdPrefix,
    record_metadata::RecordMetadata,
    Id,
};
use serde_json::Value;

#[test]
fn test_json_connection_model_definition() {
    let checker = JsonCheckerImpl::default();

    let connection_model_definition: ConnectionModelDefinition = ConnectionModelDefinition {
        id: Id::test(IdPrefix::ConnectionModelDefinition),
        connection_platform: "connection-platform".to_string(),
        connection_definition_id: Id::test(IdPrefix::ConnectionDefinition),
        platform_version: "platform-version".to_string(),
        key: "key".to_string(),
        title: "title".to_string(),
        name: "name".to_string(),
        model_name: "model-name".to_string(),
        action: Method::GET,
        action_name: CrudAction::GetOne,
        platform_info: PlatformInfo::Api(ApiModelConfig {
            base_url: "base-url".to_string(),
            path: "path".to_string(),
            auth_method: AuthMethod::OAuth,
            headers: None,
            query_params: None,
            content: Some(ContentType::Json),
            schemas: SchemasInput {
                headers: None,
                query_params: None,
                path_params: None,
                body: None,
            },
            samples: SamplesInput {
                headers: None,
                query_params: None,
                path_params: None,
                body: None,
            },
            responses: vec![],
            paths: None,
        }),
        extractor_config: None,
        test_connection_status: TestConnection::default(),
        test_connection_payload: None,
        is_default_crud_mapping: Some(true),
        mapping: Some(CrudMapping {
            action: CrudAction::GetOne,
            common_model_name: "common-model-name".to_string(),
            from_common_model: Some("from-common-model".to_string()),
            to_common_model: Some("to-common-model".to_string()),
        }),
        record_metadata: RecordMetadata::test(),
        supported: false,
    };

    assert!(
        checker.check::<ConnectionModelDefinition>(&connection_model_definition, CheckType::Json)
    );
    assert!(
        checker.check::<ConnectionModelDefinition>(&connection_model_definition, CheckType::Bson)
    );
}

#[test]
fn test_json_connection_model_schema() {
    let checker = JsonCheckerImpl::default();

    let connection_model_schema: ConnectionModelSchema = ConnectionModelSchema {
        id: Id::test(IdPrefix::ConnectionModelSchema),
        platform_id: Id::test(IdPrefix::Platform),
        platform_page_id: Id::test(IdPrefix::PlatformPage),
        connection_platform: "connection-platform".to_string(),
        connection_definition_id: Id::test(IdPrefix::ConnectionDefinition),
        platform_version: "platform-version".to_string(),
        key: "key".to_string(),
        model_name: "model-name".to_string(),
        sample: Value::Null,
        schema: JsonSchema::default(),
        paths: Some(SchemaPaths {
            id: Some("id".to_string()),
            created_at: Some("createdAt".to_string()),
            updated_at: Some("updatedAt".to_string()),
        }),
        mapping: Some(Mappings {
            from_common_model: "from-common-model".to_string(),
            to_common_model: "to-common-model".to_string(),
            common_model_name: "common-model-name".to_string(),
            common_model_id: Id::test(IdPrefix::CommonModel),
            unmapped_fields: JsonSchema::default(),
        }),
        record_metadata: RecordMetadata::test(),
    };

    assert!(
        checker.check::<ConnectionModelSchema>(&connection_model_schema, CheckType::Json)
    );
    assert!(
        checker.check::<ConnectionModelSchema>(&connection_model_schema, CheckType::Bson)
    );
}
