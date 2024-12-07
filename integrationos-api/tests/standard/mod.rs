use crate::checker::{CheckType, JsonChecker, JsonCheckerImpl};
use http::{HeaderMap, HeaderName, HeaderValue, Method};
use integrationos_domain::{
    api_model_config::{ApiModelConfig, AuthMethod, ContentType, SamplesInput, SchemasInput},
    common_model::{CommonEnum, CommonModel, DataType, Field},
    connection_definition::{
        AuthSecret, ConnectionDefinition, ConnectionDefinitionType, ConnectionForm,
        ConnectionStatus, FormDataItem, Paths,
    },
    connection_model_definition::{
        ConnectionModelDefinition, CrudAction, CrudMapping, PlatformInfo, TestConnection,
    },
    connection_model_schema::{ConnectionModelSchema, Mappings, SchemaPaths},
    environment::Environment,
    json_schema::JsonSchema,
    ownership::Ownership,
    prefix::IdPrefix,
    record_metadata::RecordMetadata,
    settings::Settings,
    Connection, ConnectionIdentityType, ConnectionType, Id, OAuth, Throughput,
};
use serde_json::{json, Value};

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
            headers: Some(HeaderMap::from_iter(vec![(
                HeaderName::from_static("authorization"),
                HeaderValue::from_static("Bearer secret"),
            )])),
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

    assert!(checker.check::<ConnectionModelSchema>(&connection_model_schema, CheckType::Json));
    assert!(checker.check::<ConnectionModelSchema>(&connection_model_schema, CheckType::Bson));
}

#[test]
fn test_json_common_enum() {
    let checker = JsonCheckerImpl::default();

    let common_enum = CommonEnum {
        id: Id::test(IdPrefix::CommonEnum),
        name: "common-enum-name".to_string(),
        options: vec!["option1".to_string(), "option2".to_string()],
        record_metadata: RecordMetadata::test(),
    };

    assert!(checker.check::<CommonEnum>(&common_enum, CheckType::Json));
    assert!(checker.check::<CommonEnum>(&common_enum, CheckType::Bson));
}

#[test]
fn test_json_common_model() {
    let checker = JsonCheckerImpl::default();

    let common_model = CommonModel {
        id: Id::test(IdPrefix::CommonModel),
        name: "common-model-name".to_string(),
        fields: vec![
            Field {
                name: "field-name".to_string(),
                datatype: DataType::String,
                description: Some("field-description".to_string()),
                required: true,
            },
            Field {
                name: "field-name-2".to_string(),
                datatype: DataType::Number,
                description: Some("field-description-2".to_string()),
                required: false,
            },
        ],
        sample: json!({
            "field-name": "field-value",
            "field-name-2": 123,
        }),
        primary: true,
        category: "category".to_string(),
        interface: Default::default(),
        record_metadata: RecordMetadata::test(),
    };

    assert!(checker.check::<CommonModel>(&common_model, CheckType::Json));
    assert!(checker.check::<CommonModel>(&common_model, CheckType::Bson));
}

#[test]
fn test_json_connection() {
    let checker = JsonCheckerImpl::default();

    let connection = Connection {
        id: Id::test(IdPrefix::Connection),
        platform_version: "platform-version".to_string(),
        connection_definition_id: Id::test(IdPrefix::ConnectionDefinition),
        r#type: ConnectionType::Api {},
        key: "key".to_string().into(),
        group: "group".to_string(),
        name: Some("name".to_string()),
        environment: Environment::Live,
        platform: "platform".to_string().into(),
        secrets_service_id: "secrets-service-id".to_string(),
        event_access_id: Id::test(IdPrefix::EventAccess),
        access_key: "access-key".to_string(),
        identity: Some("identity".to_string()),
        identity_type: Some(ConnectionIdentityType::User),
        settings: Settings {
            parse_webhook_body: true,
            show_secret: false,
            allow_custom_events: false,
            oauth: false,
        },
        throughput: Throughput {
            key: "throughput-key".to_string(),
            limit: 100,
        },
        has_error: false,
        error: None,
        ownership: Ownership {
            id: "owner-id".to_string().into(),
            client_id: "client-id".to_string(),
            organization_id: Some("organization-id".to_string()),
            project_id: Some("project-id".to_string()),
            user_id: Some("user-id".to_string()),
        },
        oauth: Some(OAuth::Enabled {
            connection_oauth_definition_id: Id::test(IdPrefix::ConnectionOAuthDefinition),
            expires_in: Some(100),
            expires_at: Some(100),
        }),
        record_metadata: RecordMetadata::test(),
    };

    assert!(checker.check::<Connection>(&connection, CheckType::Json));
    assert!(checker.check::<Connection>(&connection, CheckType::Bson));
}

#[test]
fn test_json_connection_definition() {
    let checker = JsonCheckerImpl::default();

    let connection_definition = ConnectionDefinition {
        id: Id::test(IdPrefix::ConnectionDefinition),
        platform_version: "platform-version".to_string(),
        platform: "platform".to_string(),
        status: ConnectionStatus::Alpha,
        key: "key".to_string(),
        r#type: ConnectionDefinitionType::Api,
        name: "name".to_string(),
        auth_secrets: vec![AuthSecret {
            name: "name".to_string(),
        }],
        auth_method: Some(AuthMethod::BasicAuth {
            username: "username".to_string(),
            password: "password".to_string(),
        }),
        multi_env: true,
        frontend: integrationos_domain::connection_definition::Frontend {
            spec: integrationos_domain::connection_definition::Spec {
                title: "title".to_string(),
                description: "description".to_string(),
                platform: "platform".to_string(),
                category: "category".to_string(),
                image: "image".to_string(),
                tags: vec!["tag1".to_string(), "tag2".to_string()],
                helper_link: Some("helper-link".to_string()),
                markdown: Some("markdown".to_string()),
            },
            connection_form: ConnectionForm {
                name: "name".to_string(),
                description: "description".to_string(),
                form_data: vec![FormDataItem {
                    name: "name".to_string(),
                    r#type: "type".to_string(),
                    label: "label".to_string(),
                    placeholder: "placeholder".to_string(),
                }],
            },
        },
        paths: Paths {
            id: Some("id".to_string()),
            event: Some("event".to_string()),
            payload: Some("payload".to_string()),
            timestamp: Some("timestamp".to_string()),
            secret: Some("secret".to_string()),
            signature: Some("signature".to_string()),
            cursor: Some("cursor".to_string()),
        },
        settings: Settings {
            parse_webhook_body: true,
            show_secret: false,
            allow_custom_events: false,
            oauth: false,
        },
        hidden: true,
        test_connection: Some(Id::test(IdPrefix::Connection)),
        record_metadata: RecordMetadata::test(),
    };

    // assert!(checker.check::<ConnectionDefinition>(&connection_definition, CheckType::Json));
    assert!(checker.check::<ConnectionDefinition>(&connection_definition, CheckType::Bson));
}

//TODO: Add tests for event_access, events, platform, platform_page, transactions, and secrets
