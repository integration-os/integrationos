use super::{delete, read, update, ApiResult, HookExt, PublicExt, RequestExt};
use crate::{
    bad_request, internal_server_error,
    server::{AppState, AppStores},
};
use axum::{
    extract::State,
    routing::{patch, post},
    Extension, Json, Router,
};
use convert_case::{Case, Casing};
use integrationos_domain::{
    algebra::{MongoStore, StoreExt},
    event_access::EventAccess,
    hashed_secret::HashedSecret,
    id::{prefix::IdPrefix, Id},
    ownership::Owners,
    page::PlatformPage,
    r#type::PageType,
};
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tracing::error;

pub fn get_router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/",
            post(create_platform_page).get(read::<CreateRequest, PlatformPage>),
        )
        .route(
            "/:id",
            patch(update::<CreateRequest, PlatformPage>)
                .delete(delete::<CreateRequest, PlatformPage>),
        )
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct CreateRequest {
    pub platform_id: Id,
    pub connection_definition_id: Id,
    pub platform_name: String,
    #[serde(flatten)]
    pub r#type: PageType,
    pub url: String,
    pub model_name: String,
    pub content: String,
    pub ownership: Owners,
    pub analyzed: bool,
}

impl HookExt<PlatformPage> for CreateRequest {}
impl PublicExt<PlatformPage> for CreateRequest {}

pub async fn create_platform_page(
    event_access: Option<Extension<Arc<EventAccess>>>,
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateRequest>,
) -> ApiResult<PlatformPage> {
    let output = if let Some(Extension(event_access)) = event_access {
        req.clone().access(event_access)
    } else {
        req.clone().from()
    };

    let mut output = match output {
        Some(output) => output,
        None => return Err(bad_request!("Invalid request")),
    };

    output.model_name = output.model_name.to_case(Case::Pascal);

    let res = match CreateRequest::get_store(state.app_stores.clone())
        .create_one(&output)
        .await
    {
        Ok(_) => Ok(Json(output)),
        Err(e) => {
            error!("Error creating object: {e}");
            Err(internal_server_error!())
        }
    }?;

    Ok(res)
}

impl RequestExt for CreateRequest {
    type Output = PlatformPage;

    fn from(&self) -> Option<Self::Output> {
        let hash_value = json!({
            "platform_id": self.platform_id,
            "platform_name": self.platform_name,
            "model_name": self.model_name,
            "page_type": self.r#type,
            "content": self.content
        });

        let hashed = HashedSecret::try_from(hash_value).ok()?;

        Some(Self::Output {
            id: Id::now(IdPrefix::PlatformPage),
            platform_id: self.platform_id,
            platform_name: self.platform_name.clone(),
            connection_definition_id: self.connection_definition_id,
            r#type: self.r#type.clone(),
            url: self.url.clone(),
            model_name: self.model_name.clone(),
            content: self.content.clone(),
            hashed_content: hashed.into_inner(),
            ownership: self.ownership.clone(),
            analyzed: self.analyzed,
            job_started: false,
            record_metadata: Default::default(),
        })
    }

    fn update(&self, mut record: Self::Output) -> Self::Output {
        record.platform_id = self.platform_id;
        record.connection_definition_id = self.connection_definition_id;
        record.r#type = self.r#type.clone();
        record.url = self.url.clone();
        record.model_name = self.model_name.clone();
        record.content = self.content.clone();
        record.ownership = self.ownership.clone();
        record.analyzed = self.analyzed;

        record
    }

    fn get_store(stores: AppStores) -> MongoStore<Self::Output> {
        stores.platform_page.clone()
    }
}
