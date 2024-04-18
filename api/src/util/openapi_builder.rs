use convert_case::{Case, Casing};
use indexmap::IndexMap;
use integrationos_domain::{common_model::CommonModel, connection_model_definition::CrudAction};
use openapiv3::*;
use strum::IntoEnumIterator;
use tracing::debug;

#[derive(Debug, Clone, PartialEq)]
struct PathItemAction {
    item: PathItem,
    path: String,
}

// OPENAPI METADATA
const URI: &str = "https://api.integrationos.com/v1/unified";
const OPENAPI_VERSION: &str = "3.0.3";
const SPEC_VERSION: &str = "1.0.0";
const TITLE: &str = "Common Models";
const X_SPEAKEASY_NAME_OVERRIDE: &str = "x-speakeasy-name-override";
const X_SPEAKEASY_IGNORE: &str = "x-speakeasy-ignore";
const X_INTEGRATIONOS_SECRET: &str = "X-INTEGRATIONOS-SECRET";
const X_INTEGRATIONOS_CONNECTION_KEY: &str = "X-INTEGRATIONOS-CONNECTION-KEY";
const X_INTEGRATIONOS_ENABLE_PASSTHROUGH: &str = "X-INTEGRATIONOS-ENABLE-PASSTHROUGH";
const X_INTEGRATIONOS_PASSTHROUGH_FORWARD: &str = "X-INTEGRATIONOS-PASSTHROUGH-FORWARD";

pub fn generate_path_item(common_model: &CommonModel) -> IndexMap<String, ReferenceOr<PathItem>> {
    IndexMap::from_iter(
        items(common_model)
            .iter()
            .map(|item| (item.path.to_string(), ReferenceOr::Item(item.item.clone())))
            .collect::<Vec<(String, ReferenceOr<PathItem>)>>(),
    )
}

pub fn generate_openapi_schema(
    path: Vec<IndexMap<String, ReferenceOr<PathItem>>>,
    schemas: IndexMap<String, ReferenceOr<Schema>>,
) -> Box<OpenAPI> {
    debug!(
        "Generating OpenAPI schema for common models: {}",
        path.len()
    );

    let paths = path
        .iter()
        .fold(Paths::default(), |mut paths_acc, common_model| {
            paths_acc.paths.extend(common_model.clone());

            paths_acc.paths.sort_keys();

            paths_acc
        });

    debug!("All common models processed");

    Box::new(OpenAPI {
        openapi: OPENAPI_VERSION.to_string(),
        info: Info {
            title: TITLE.to_string(),
            version: SPEC_VERSION.to_string(),
            ..Default::default()
        },
        servers: vec![Server {
            url: URI.to_string(),
            ..Default::default()
        }],
        paths,
        components: Some(Components {
            schemas,
            ..Default::default()
        }),
        security: Some(vec![IndexMap::from_iter(vec![(
            "secret".to_string(),
            vec![],
        )])]),
        ..Default::default()
    })
}

fn items(common_model: &CommonModel) -> [PathItemAction; 3] {
    [
        PathItemAction {
            path: format!("/{}/{{id}}", common_model.name.to_case(Case::Kebab)),
            item: PathItem {
                get: Some(operation(&CrudAction::GetOne, common_model)),
                delete: Some(operation(&CrudAction::Delete, common_model)),
                patch: Some(operation(&CrudAction::Update, common_model)),
                parameters: header(),
                ..Default::default()
            },
        },
        PathItemAction {
            path: format!("/{}", common_model.name.to_case(Case::Kebab)),
            item: PathItem {
                description: Some(CrudAction::GetMany.description().into()),
                get: Some(operation(&CrudAction::GetMany, common_model)),
                post: Some(operation(&CrudAction::Create, common_model)),
                parameters: header(),
                ..Default::default()
            },
        },
        PathItemAction {
            path: format!("/{}/count", common_model.name.to_case(Case::Kebab)),
            item: PathItem {
                description: Some(CrudAction::GetCount.description().into()),
                get: Some(operation(&CrudAction::GetCount, common_model)),
                parameters: header(),
                ..Default::default()
            },
        },
    ]
}

fn operation(action: &CrudAction, common_model: &CommonModel) -> Operation {
    let summary = match action {
        CrudAction::GetOne => format!("Get {}", common_model.name.to_case(Case::Pascal)),
        CrudAction::GetMany => format!("List {}", common_model.name.to_case(Case::Pascal)),
        CrudAction::GetCount => format!("Get {} count", common_model.name.to_case(Case::Pascal)),
        CrudAction::Create => format!("Create {}", common_model.name.to_case(Case::Pascal)),
        CrudAction::Update => format!("Update {}", common_model.name.to_case(Case::Pascal)),
        CrudAction::Delete => format!("Delete {}", common_model.name.to_case(Case::Pascal)),
        _ => unimplemented!("Not implemented yet"),
    };

    let description = match action {
        CrudAction::GetOne => format!(
            "Get a single {} record",
            common_model.name.to_case(Case::Pascal)
        ),
        CrudAction::GetMany => format!(
            "Get all {} records",
            common_model.name.to_case(Case::Pascal)
        ),
        CrudAction::GetCount => format!(
            "Get the count of {} records",
            common_model.name.to_case(Case::Pascal)
        ),
        CrudAction::Create => format!(
            "Create a single {} record",
            common_model.name.to_case(Case::Pascal)
        ),
        CrudAction::Update => format!(
            "Update a single {} record",
            common_model.name.to_case(Case::Pascal)
        ),
        CrudAction::Delete => format!(
            "Delete a single {} record",
            common_model.name.to_case(Case::Pascal)
        ),
        _ => unimplemented!("Not implemented yet"),
    };

    let response = IndexMap::from_iter(vec![(
        StatusCode::Code(200),
        ReferenceOr::Item(Response {
            description: "Successful response".to_string(),
            content: content(action, common_model),
            ..Default::default()
        }),
    )]);

    let extensions = IndexMap::from_iter(vec![(
        X_SPEAKEASY_NAME_OVERRIDE.to_string(),
        serde_json::Value::String(
            {
                match action {
                    CrudAction::GetOne => "get",
                    CrudAction::GetMany => "list",
                    CrudAction::GetCount => "count",
                    CrudAction::Create => "create",
                    CrudAction::Update => "update",
                    CrudAction::Delete => "delete",
                    _ => unimplemented!("Not implemented yet"),
                }
            }
            .to_string(),
        ),
    )]);

    Operation {
        tags: vec![common_model.name.to_owned()],
        summary: Some(summary),
        description: Some(description),
        parameters: parameter(action),
        request_body: body(action, common_model),
        responses: Responses {
            responses: response,
            ..Default::default()
        },
        extensions,
        ..Default::default()
    }
}

fn parameter(action: &CrudAction) -> Vec<ReferenceOr<Parameter>> {
    let passthrough_query_param = ReferenceOr::Item(Parameter::Query {
        parameter_data: ParameterData {
            name: "passthroughForward".to_string(),
            description: Some("A string of all query parameters to forward in the request to the 3rd-party platform".to_string()),
            required: false,
            deprecated: Some(false),
            format: ParameterSchemaOrContent::Schema(ReferenceOr::Item(Schema {
                schema_data: Default::default(),
                schema_kind: SchemaKind::Type(Type::String(StringType {
                    format: VariantOrUnknownOrEmpty::Unknown("string".to_string()),
                    ..Default::default()
                })),
            })),
            example: Some(serde_json::Value::String("customParam=sample&customParam2=123".to_string())),
            examples: Default::default(),
            explode: Default::default(),
            extensions: Default::default(),
        },
        style: QueryStyle::Form,
        allow_reserved: false,
        allow_empty_value: None,
    });

    let path = vec![
        ReferenceOr::Item(Parameter::Path {
            parameter_data: ParameterData {
                name: "id".to_string(),
                description: Some("The id of the model".to_string()),
                required: true,
                deprecated: Some(false),
                format: ParameterSchemaOrContent::Schema(ReferenceOr::Item(Schema {
                    schema_data: Default::default(),
                    schema_kind: SchemaKind::Type(Type::String(StringType {
                        format: VariantOrUnknownOrEmpty::Unknown("string".to_string()),
                        ..Default::default()
                    })),
                })),
                example: serde_json::to_value("cm::F5YOwU3hwyA::vTW3YaBvT3CHilxcppJOrQ").ok(),
                examples: Default::default(),
                explode: Default::default(),
                extensions: Default::default(),
            },
            style: PathStyle::Simple,
        }),
        passthrough_query_param.clone(),
    ];
    match action {
        CrudAction::Create => vec![passthrough_query_param],
        CrudAction::GetCount => vec![passthrough_query_param],
        CrudAction::GetOne => path,
        CrudAction::Delete => path
            .into_iter()
            .chain(vec![ReferenceOr::Item(Parameter::Query {
                parameter_data: ParameterData {
                    name: MODIFY_TOKEN.to_string(),
                    description: Some("The modified token of the model".to_string()),
                    required: false,
                    deprecated: Some(false),
                    format: ParameterSchemaOrContent::Schema(ReferenceOr::Item(Schema {
                        schema_data: Default::default(),
                        schema_kind: SchemaKind::Type(Type::String(StringType {
                            format: VariantOrUnknownOrEmpty::Unknown("string".to_string()),
                            ..Default::default()
                        })),
                    })),
                    example: Default::default(),
                    examples: Default::default(),
                    explode: Default::default(),
                    extensions: Default::default(),
                },
                style: QueryStyle::Form,
                allow_reserved: false,
                allow_empty_value: None,
            })])
            .collect(),
        CrudAction::Update => path
            .into_iter()
            .chain(vec![ReferenceOr::Item(Parameter::Query {
                parameter_data: ParameterData {
                    name: MODIFY_TOKEN.to_string(),
                    description: Some("The modified token of the model".to_string()),
                    required: false,
                    deprecated: Some(false),
                    format: ParameterSchemaOrContent::Schema(ReferenceOr::Item(Schema {
                        schema_data: Default::default(),
                        schema_kind: SchemaKind::Type(Type::String(StringType {
                            format: VariantOrUnknownOrEmpty::Unknown("string".to_string()),
                            ..Default::default()
                        })),
                    })),
                    example: Default::default(),
                    examples: Default::default(),
                    explode: Default::default(),
                    extensions: Default::default(),
                },
                style: QueryStyle::Form,
                allow_reserved: false,
                allow_empty_value: None,
            })])
            .collect(),
        CrudAction::GetMany => [
            LIMIT,
            CURSOR,
            CREATED_AFTER,
            CREATED_BEFORE,
            UPDATED_AFTER,
            UPDATED_BEFORE,
        ]
        .iter()
        .map(|name| {
            ReferenceOr::Item(Parameter::Query {
                parameter_data: ParameterData {
                    name: name.to_string(),
                    description: None,
                    required: false,
                    deprecated: Some(false),
                    format: ParameterSchemaOrContent::Schema(ReferenceOr::Item(Schema {
                        schema_data: Default::default(),
                        schema_kind: SchemaKind::Type(Type::String(StringType {
                            format: VariantOrUnknownOrEmpty::Unknown("string".to_string()),
                            ..Default::default()
                        })),
                    })),
                    example: Default::default(),
                    examples: Default::default(),
                    explode: Default::default(),
                    extensions: Default::default(),
                },
                style: QueryStyle::Form,
                allow_reserved: false,
                allow_empty_value: None,
            })
        })
        .chain(vec![passthrough_query_param])
        .collect(),
        _ => vec![],
    }
}

fn header() -> Vec<ReferenceOr<Parameter>> {
    vec![
        ReferenceOr::Item(Parameter::Header {
            parameter_data: ParameterData {
                name: X_INTEGRATIONOS_SECRET.to_string(),
                description: Some("IntegrationOS API key".to_string()),
                required: true,
                deprecated: Some(false),
                format: ParameterSchemaOrContent::Schema(ReferenceOr::Item(Schema {
                    schema_data: Default::default(),
                    schema_kind: SchemaKind::Type(Type::String(StringType {
                        format: VariantOrUnknownOrEmpty::Unknown("string".to_string()),
                        pattern: None,
                        enumeration: vec![],
                        min_length: None,
                        max_length: None,
                    })),
                })),
                example: None,
                examples: Default::default(),
                explode: Default::default(),
                extensions: IndexMap::from_iter(vec![(
                    X_SPEAKEASY_IGNORE.to_string(),
                    serde_json::Value::Bool(true),
                )]),
            },
            style: HeaderStyle::Simple,
        }),
        ReferenceOr::Item(Parameter::Header {
            parameter_data: ParameterData {
                name: X_INTEGRATIONOS_CONNECTION_KEY.to_string(),
                description: Some("The unique identifier of a Connected Account".to_string()),
                required: true,
                deprecated: Some(false),
                format: ParameterSchemaOrContent::Schema(ReferenceOr::Item(Schema {
                    schema_data: Default::default(),
                    schema_kind: SchemaKind::Type(Type::String(StringType {
                        format: VariantOrUnknownOrEmpty::Unknown("string".to_string()),
                        ..Default::default()
                    })),
                })),
                example: Some(serde_json::Value::String("customHeader=sample;customHeader2=123".to_string())),
                examples: Default::default(),
                explode: Default::default(),
                extensions: Default::default(),
            },
            style: HeaderStyle::Simple,
        }),
        ReferenceOr::Item(Parameter::Header {
            parameter_data: ParameterData {
                name: X_INTEGRATIONOS_ENABLE_PASSTHROUGH.to_string(),
                description: Some("Set to true to receive the exact API response from the connection platform in the passthrough object".to_string()),
                required: false,
                deprecated: Some(false),
                format: ParameterSchemaOrContent::Schema(ReferenceOr::Item(Schema {
                    schema_data: Default::default(),
                    schema_kind: SchemaKind::Type(Type::String(StringType {
                        format: VariantOrUnknownOrEmpty::Unknown("string".to_string()),
                        ..Default::default()
                    })),
                })),
                example: Some(serde_json::Value::String("true".to_string())),
                examples: Default::default(),
                explode: Default::default(),
                extensions: Default::default(),
            },
            style: HeaderStyle::Simple,
        }),
        ReferenceOr::Item(Parameter::Header {
            parameter_data: ParameterData {
                name: X_INTEGRATIONOS_PASSTHROUGH_FORWARD.to_string(),
                description: Some("A string of all headers to forward in the request to the 3rd-party platform".to_string()),
                required: false,
                deprecated: Some(false),
                format: ParameterSchemaOrContent::Schema(ReferenceOr::Item(Schema {
                    schema_data: Default::default(),
                    schema_kind: SchemaKind::Type(Type::String(StringType {
                        format: VariantOrUnknownOrEmpty::Unknown("string".to_string()),
                        ..Default::default()
                    })),
                })),
                example: Some(serde_json::Value::String("customHeader=sample;customHeader2=123".to_string())),
                examples: Default::default(),
                explode: Default::default(),
                extensions: Default::default(),
            },
            style: HeaderStyle::Simple,
        }),
    ]
}

fn content(action: &CrudAction, common_model: &CommonModel) -> IndexMap<String, MediaType> {
    let mut content = IndexMap::new();
    content.insert(
        "application/json".to_string(),
        MediaType {
            schema: Some(ReferenceOr::Item(Schema {
                schema_data: Default::default(),
                schema_kind: SchemaKind::Type(Type::Object(ObjectType {
                    properties: properties(action, common_model),
                    ..Default::default()
                })),
            })),
            example: Some(action.example(common_model)),
            ..Default::default()
        },
    );
    content
}

fn body(action: &CrudAction, common_model: &CommonModel) -> Option<ReferenceOr<RequestBody>> {
    match action {
        CrudAction::Create | CrudAction::Update => Some(ReferenceOr::Item(
            common_model.request_body(CrudAction::Create == *action),
        )),
        _ => None,
    }
}

fn string_schema(format: &str) -> ReferenceOr<Box<Schema>> {
    ReferenceOr::Item(Box::new(Schema {
        schema_data: Default::default(),
        schema_kind: SchemaKind::Type(Type::String(StringType {
            format: VariantOrUnknownOrEmpty::Unknown(format.to_string()),
            ..Default::default()
        })),
    }))
}

fn integer_schema(format: IntegerFormat) -> ReferenceOr<Box<Schema>> {
    ReferenceOr::Item(Box::new(Schema {
        schema_data: Default::default(),
        schema_kind: SchemaKind::Type(Type::Integer(IntegerType {
            format: VariantOrUnknownOrEmpty::Item(format),
            ..Default::default()
        })),
    }))
}

fn boolean_schema() -> ReferenceOr<Box<Schema>> {
    ReferenceOr::Item(Box::new(Schema {
        schema_data: Default::default(),
        schema_kind: SchemaKind::Type(Type::Boolean(BooleanType {
            enumeration: vec![],
        })),
    }))
}

fn array_schema(items: ReferenceOr<Box<Schema>>) -> ReferenceOr<Box<Schema>> {
    ReferenceOr::Item(Box::new(Schema {
        schema_data: Default::default(),
        schema_kind: SchemaKind::Type(Type::Array(ArrayType {
            items: Some(items),
            max_items: None,
            min_items: None,
            unique_items: false,
        })),
    }))
}

fn object_schema(
    properties: IndexMap<String, ReferenceOr<Box<Schema>>>,
    additional_properties: Option<AdditionalProperties>,
) -> ReferenceOr<Box<Schema>> {
    ReferenceOr::Item(Box::new(Schema {
        schema_data: Default::default(),
        schema_kind: SchemaKind::Type(Type::Object(ObjectType {
            properties,
            additional_properties,
            ..Default::default()
        })),
    }))
}

fn reference_schema(reference: &str) -> ReferenceOr<Box<Schema>> {
    ReferenceOr::Reference {
        reference: "#/components/schemas/".to_owned() + reference,
    }
}

// OPENAPI PROPERTY KEYS
const META: &str = "meta";
const STATUS: &str = "status";
const STATUS_CODE: &str = "statusCode";
const UNIFIED: &str = "unified";
const PASSTHROUGH: &str = "passthrough";
const CONNECTION_DEFINITION_KEY: &str = "connectionDefinitionKey";
const CONNECTION_KEY: &str = "connectionKey";
const TRANSACTION_KEY: &str = "transactionKey";
const TXN: &str = "txn";
const PLATFORM: &str = "platform";
const PLATFORM_VERSION: &str = "platformVersion";
const ACTION: &str = "action";
const COMMON_MODEL: &str = "commonModel";
const COMMON_MODEL_VERSION: &str = "commonModelVersion";
const HASH: &str = "hash";
const MODIFY_TOKEN: &str = "modifyToken";
const HEARTBEATS: &str = "heartbeats";
const TOTAL_TRANSACTIONS: &str = "totalTransactions";
const CACHE: &str = "cache";
const HIT: &str = "hit";
const TTL: &str = "ttl";
const KEY: &str = "key";
const RATE_LIMIT_REMAINING: &str = "rateLimitRemaining";
const PLATFORM_RATE_LIMIT_REMAINING: &str = "platformRateLimitRemaining";
const LATENCY: &str = "latency";
const TIMESTAMP: &str = "timestamp";
const COUNT: &str = "count";
const PAGINATION: &str = "pagination";
const CURSOR: &str = "cursor";
const NEXT_CURSOR: &str = "nextCursor";
const PREV_CURSOR: &str = "previousCursor";
const LIMIT: &str = "limit";
const CREATED_AFTER: &str = "createdAfter";
const CREATED_BEFORE: &str = "createdBefore";
const UPDATED_AFTER: &str = "updatedAfter";
const UPDATED_BEFORE: &str = "updatedBefore";

fn properties(
    action: &CrudAction,
    common_model: &CommonModel,
) -> IndexMap<String, ReferenceOr<Box<Schema>>> {
    let mut properties = IndexMap::new();

    properties.insert(STATUS.to_owned(), string_schema("success | failure"));
    properties.insert(STATUS_CODE.to_owned(), integer_schema(IntegerFormat::Int32));

    match action {
        CrudAction::GetOne | CrudAction::Create => {
            properties.insert(
                UNIFIED.to_owned(),
                reference_schema(common_model.name.as_str()),
            );
        }
        CrudAction::GetMany => {
            properties.insert(
                UNIFIED.to_owned(),
                array_schema(reference_schema(common_model.name.as_str())),
            );
            properties.insert(
                PAGINATION.to_owned(),
                object_schema(
                    IndexMap::from_iter(vec![
                        (NEXT_CURSOR.to_owned(), string_schema("string")),
                        (PREV_CURSOR.to_owned(), string_schema("string")),
                        (LIMIT.to_owned(), integer_schema(IntegerFormat::Int32)),
                    ]),
                    None,
                ),
            );
        }
        CrudAction::GetCount => {
            properties.insert(
                UNIFIED.to_owned(),
                object_schema(
                    IndexMap::from_iter(vec![(
                        COUNT.to_owned(),
                        integer_schema(IntegerFormat::Int32),
                    )]),
                    None,
                ),
            );
        }
        CrudAction::Update => {
            properties.insert(UNIFIED.to_owned(), object_schema(IndexMap::new(), None));
        }
        CrudAction::Delete => {
            properties.insert(
                UNIFIED.to_owned(),
                reference_schema(common_model.name.as_str()),
            );
        }
        CrudAction::Custom => unimplemented!("Not implemented yet"),
    }

    properties.insert(
        PASSTHROUGH.to_owned(),
        object_schema(IndexMap::new(), Some(AdditionalProperties::Any(true))),
    );

    properties.insert(
        META.to_owned(),
        object_schema(
            IndexMap::from_iter(vec![
                (TIMESTAMP.to_owned(), integer_schema(IntegerFormat::Int64)),
                (LATENCY.to_owned(), integer_schema(IntegerFormat::Int32)),
                (
                    PLATFORM_RATE_LIMIT_REMAINING.to_owned(),
                    integer_schema(IntegerFormat::Int32),
                ),
                (
                    RATE_LIMIT_REMAINING.to_owned(),
                    integer_schema(IntegerFormat::Int32),
                ),
                (
                    TOTAL_TRANSACTIONS.to_owned(),
                    integer_schema(IntegerFormat::Int32),
                ),
                (HASH.to_owned(), string_schema("string")),
                (TRANSACTION_KEY.to_owned(), string_schema("string")),
                (TXN.to_owned(), string_schema("string")),
                (COMMON_MODEL.to_owned(), string_schema("string")),
                (CONNECTION_KEY.to_owned(), string_schema("string")),
                (PLATFORM.to_owned(), string_schema("string")),
                (PLATFORM_VERSION.to_owned(), string_schema("string")),
                (
                    CONNECTION_DEFINITION_KEY.to_owned(),
                    string_schema("string"),
                ),
                (
                    ACTION.to_owned(),
                    string_schema(
                        &CrudAction::iter()
                            .filter(|action| action != &CrudAction::Custom)
                            .map(|action| action.to_string())
                            .collect::<Vec<String>>()
                            .join(" | "),
                    ),
                ),
                (COMMON_MODEL_VERSION.to_owned(), string_schema("string")),
                (KEY.to_owned(), string_schema("string")),
                (HEARTBEATS.to_owned(), array_schema(string_schema("string"))),
                (
                    CACHE.to_owned(),
                    object_schema(
                        IndexMap::from_iter(vec![
                            (HIT.to_owned(), boolean_schema()),
                            (TTL.to_owned(), integer_schema(IntegerFormat::Int32)),
                        ]),
                        None,
                    ),
                ),
            ]),
            None,
        ),
    );

    properties
}
