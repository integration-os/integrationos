use axum::{extract::Path, response::IntoResponse, Json};
use http::StatusCode;
use integrationos_domain::{prefix::IdPrefix, Id, IntegrationOSError};
use serde::Serialize;
use serde_json::json;

#[derive(Serialize)]
pub struct GenerateIdResponse {
    pub id: String,
}

pub async fn generate_id(
    Path(id_prefix): Path<String>,
) -> Result<(StatusCode, Json<GenerateIdResponse>), IntegrationOSError> {
    let id_prefix_str = id_prefix.as_str();

    let id_prefix = IdPrefix::try_from(id_prefix_str)?;

    let id = Id::now(id_prefix);

    Ok((
        StatusCode::OK,
        Json(GenerateIdResponse { id: id.to_string() }),
    ))
}

pub async fn get_version() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(json!({"version": env!("CARGO_PKG_VERSION")})),
    )
}
