use super::{
    create, delete, update, ApiResult, CachedRequest, CrudHook, CrudRequest, ReadResponse, Unit,
};
use crate::{
    internal_server_error, not_found,
    server::{AppState, AppStores},
};
use axum::{
    extract::{Path, State},
    routing::{patch, post},
    Json, Router,
};
use integrationos_domain::{
    algebra::{MongoStore, StoreExt},
    api_model_config::AuthMethod,
    connection_definition::ConnectionStatus,
    connection_definition::{
        AuthSecret, ConnectionDefinition, ConnectionDefinitionType, ConnectionForm, FormDataItem,
        Frontend, Paths, Spec,
    },
    connection_model_definition::{ConnectionModelDefinition, CrudAction},
    id::{prefix::IdPrefix, Id},
    record_metadata::RecordMetadata,
    settings::Settings,
};
use moka::future::Cache;
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc};
use tracing::error;

pub fn get_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(create::<CreateRequest, ConnectionDefinition>))
        .route(
            "/:id",
            patch(update::<CreateRequest, ConnectionDefinition>)
                .delete(delete::<CreateRequest, ConnectionDefinition>),
        )
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct CreateRequest {
    pub platform: String,
    pub platform_version: String,
    #[serde(default)]
    pub status: ConnectionStatus,
    pub r#type: ConnectionDefinitionType,
    pub name: String,
    pub description: String,
    pub category: String,
    pub image: String,
    pub tags: Vec<String>,
    pub helper_link: Option<String>,
    pub authentication: Vec<AuthenticationItem>,
    pub auth_method: Option<AuthMethod>,
    pub settings: Settings,
    pub paths: Paths,
    pub test_connection: Option<Id>,
    pub active: bool,
}

impl CrudHook<ConnectionDefinition> for CreateRequest {}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
pub struct AuthenticationItem {
    pub name: String,
    pub label: String,
    pub r#type: String,
    pub placeholder: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpdateFields {
    pub active: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicGetConnectionDetailsResponse {
    pub platform: String,
    pub status: ConnectionStatus,
    pub supported_actions: Vec<CrudAction>,
    pub oauth: PublicConnectionDataOauth,
    pub pagination: bool,
    pub filtration: bool,
    pub sorting: bool,
    pub caveats: Vec<PublicConnectionDataCaveat>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PublicConnectionDataCaveat {
    pub name: String,
    pub action: Option<CrudAction>,
    pub comments: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PublicConnectionDataOauth {
    pub enabled: bool,
    pub scopes: String,
}

pub async fn public_get_connection_details(
    Path((common_model, platform_name)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> ApiResult<PublicGetConnectionDetailsResponse> {
    let Some(connection_definition) = state
        .app_stores
        .connection_config
        .get_one(doc! {
            "platform": &platform_name,
        })
        .await
        .map_err(|e| {
            error!("Error reading from connection definitions: {e}");
            internal_server_error!()
        })?
    else {
        return Err(not_found!("Connection definition"));
    };

    let connection_model_definitions = state
        .app_stores
        .model_config
        .get_many(
            Some(doc! {
                "connectionPlatform": {
                    "$regex": format!("^{}$", &platform_name),
                    "$options": "i"
                },
                "mapping.commonModelName": {
                    "$regex": format!("^{}$", &common_model),
                    "$options": "i"
                },
                "actionName": {
                    "$in": [
                        "create",
                        "update",
                        "getMany",
                        "getOne",
                        "getCount",
                        "delete"
                    ]
                }
            }),
            None,
            None,
            None,
            None,
        )
        .await
        .map_err(|e| {
            error!("Error reading from connection model definitions: {e}");
            internal_server_error!()
        })?;

    let supported_actions = connection_model_definitions
        .clone()
        .into_iter()
        .map(|definition| definition.action_name)
        .collect::<Vec<CrudAction>>();

    let oauth_enabled = matches!(connection_definition.auth_method, Some(AuthMethod::OAuth));

    let scopes = if oauth_enabled {
        let connection_oauth_definition = state
            .app_stores
            .oauth_config
            .get_one(doc! {
                "connectionPlatform": &platform_name,
            })
            .await
            .map_err(|e| {
                error!("Error reading from connection definitions: {e}");
                internal_server_error!()
            })?
            .ok_or_else(|| not_found!("Connection OAuth definition"))?;

        connection_oauth_definition.frontend.scopes
    } else {
        String::new()
    };

    let public_connection_details_record = state
        .app_stores
        .public_connection_details
        .get_one(doc! {
            "platform": &platform_name,
        })
        .await
        .map_err(|e| {
            error!("Error reading from public connection details: {e}");
            internal_server_error!()
        })?
        .ok_or_else(|| not_found!("Public Connection Details"))?;

    let model_features = public_connection_details_record
        .models
        .iter()
        .find(|model| model.name.to_lowercase() == common_model.to_lowercase())
        .ok_or_else(|| not_found!("Model Features"))?;

    let caveats =
        public_connection_details_record
            .caveats
            .into_iter()
            .fold(vec![], |mut v, caveat| {
                match caveat.connection_model_definition_id {
                    Some(cmd_id) => {
                        let connection_model_definition = connection_model_definitions.iter().find(
                            |definition: &&ConnectionModelDefinition| {
                                definition.id.to_string() == cmd_id
                            },
                        );

                        if let Some(definition) = connection_model_definition {
                            v.push(PublicConnectionDataCaveat {
                                name: definition.title.clone(),
                                action: Some(definition.action_name.clone()),
                                comments: caveat.comments,
                            });
                        }
                    }
                    None => {
                        v.push(PublicConnectionDataCaveat {
                            name: "General".to_string(),
                            action: None,
                            comments: caveat.comments,
                        });
                    }
                }
                v
            });

    Ok(Json(PublicGetConnectionDetailsResponse {
        platform: connection_definition.platform,
        status: connection_definition.status,
        oauth: PublicConnectionDataOauth {
            enabled: oauth_enabled,
            scopes,
        },
        supported_actions,
        pagination: model_features.pagination,
        filtration: model_features.filtration,
        sorting: model_features.sorting,
        caveats,
    }))
}

impl CrudRequest for CreateRequest {
    type Output = ConnectionDefinition;

    fn output(&self) -> Option<Self::Output> {
        let auth_secrets: Vec<AuthSecret> = self
            .authentication
            .iter()
            .map(|item| AuthSecret {
                name: item.name.to_string(),
            })
            .collect();

        let connection_form_items: Vec<FormDataItem> = self
            .authentication
            .iter()
            .map(|item| FormDataItem {
                name: item.name.clone(),
                r#type: item.r#type.clone(),
                label: item.label.clone(),
                placeholder: item.placeholder.clone(),
            })
            .collect();

        let connection_form = ConnectionForm {
            name: "Connect".to_string(),
            description: "Securely connect your account".to_string(),
            form_data: connection_form_items,
        };

        let key = format!("api::{}::{}", self.platform, self.platform_version);

        let mut record = Self::Output {
            id: Id::now(IdPrefix::ConnectionDefinition),
            platform_version: self.platform_version.clone(),
            platform: self.platform.clone(),
            status: self.status.clone(),
            r#type: self.r#type.clone(),
            name: self.name.clone(),
            key,
            frontend: Frontend {
                spec: Spec {
                    title: self.name.clone(),
                    description: self.description.clone(),
                    platform: self.platform.clone(),
                    category: self.category.clone(),
                    image: self.image.clone(),
                    tags: self.tags.clone(),
                    helper_link: self.helper_link.clone(),
                },
                connection_form,
            },
            test_connection: self.test_connection,
            auth_secrets,
            auth_method: self.auth_method.clone(),
            paths: self.paths.clone(),
            settings: self.settings.clone(),
            hidden: false,
            record_metadata: RecordMetadata::default(),
        };

        record.record_metadata.active = self.active;
        Some(record)
    }

    fn update(&self, record: &mut Self::Output) -> Unit {
        record.name = self.name.clone();
        record.frontend.spec.description = self.description.clone();
        record.frontend.spec.category = self.category.clone();
        record.frontend.spec.image = self.image.clone();
        record.frontend.spec.tags = self.tags.clone();
        record.test_connection = self.test_connection;
        record.platform = self.platform.clone();
        record.record_metadata.active = self.active;
    }

    fn get_store(stores: AppStores) -> MongoStore<Self::Output> {
        stores.connection_config
    }
}

impl CachedRequest for CreateRequest {
    fn get_cache(
        state: Arc<AppState>,
    ) -> Arc<Cache<Option<BTreeMap<String, String>>, Arc<ReadResponse<Self::Output>>>> {
        state.connection_definitions_cache.clone()
    }
}
