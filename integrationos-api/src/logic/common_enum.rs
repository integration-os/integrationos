use super::{PublicExt, RequestExt};
use crate::server::AppStores;
use integrationos_domain::common_model::CommonEnum;
use integrationos_domain::MongoStore;

#[derive(Debug, Clone, PartialEq)]
pub struct GetRequest;

impl PublicExt<CommonEnum> for GetRequest {}
impl RequestExt for GetRequest {
    type Output = CommonEnum;

    fn get_store(stores: AppStores) -> MongoStore<Self::Output> {
        stores.common_enum
    }
}
