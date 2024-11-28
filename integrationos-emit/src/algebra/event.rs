use crate::{domain::event::Event, server::AppState};
use async_trait::async_trait;
use http::header::AUTHORIZATION;
use integrationos_domain::{ApplicationError, Claims, Id, IntegrationOSError, InternalError, Unit};

#[async_trait]
pub trait EventExt {
    async fn side_effect(&self, ctx: &AppState, entity_id: Id) -> Result<Unit, IntegrationOSError>;
}

#[async_trait]
impl EventExt for Event {
    async fn side_effect(&self, ctx: &AppState, entity_id: Id) -> Result<Unit, IntegrationOSError> {
        match self {
            Event::DatabaseConnectionLost { connection_id, .. } => {
                let base_path = &ctx.config.event_callback_url;
                let path = format!("{base_path}/database-connection-lost/{connection_id}");

                let authorization = Claims::from_secret(ctx.config.jwt_secret.as_str())?;

                ctx.http_client
                    .post(path)
                    .header(AUTHORIZATION, format!("Bearer {authorization}"))
                    .send()
                    .await
                    .inspect(|res| {
                        tracing::info!("Response: {:?}", res);
                    })
                    .map_err(|e| {
                        tracing::error!("Failed to build request for entity id {entity_id}: {e}");
                        InternalError::io_err(
                            &format!("Failed to build request for entity id {entity_id}"),
                            None,
                        )
                    })?
                    .error_for_status()
                    .map_err(|e| {
                        tracing::error!("Failed to execute request for entity id {entity_id}: {e}");
                        ApplicationError::bad_request(
                            &format!("Failed to execute request for entity id {entity_id}"),
                            None,
                        )
                    })
                    .map(|res| tracing::info!("Response: {:?}", res))
            }
        }
    }
}
