pub mod region;

use bson::oid::ObjectId;
use bson::DateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MicroService {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub name: String,
    pub class: String,
    pub r#type: String,
    pub tags: Vec<String>,
    pub host: String,
    pub region: String,
    // TODO ui
    #[serde(rename = "__microservice__")]
    pub microservice: MicroServiceMicroService,
    #[serde(rename = "__buildableTtl__")]
    pub buildable_ttl: DateTime,
    pub buildable_id: String,
    pub author: MicroServiceAuthor,
    #[serde(rename = "__global")]
    pub global: bool,
    pub frontend: MicroServiceFrontend,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MicroServiceMicroService {
    #[serde(rename = "__global")]
    pub global: bool,
    pub url: String,
    pub r#type: String,
    pub version: String,
    // TODO history
    pub buildable_package: MicroServiceMicroServiceBuildablePackage,
    // TODO settings
    pub instance: MicroServiceMicroServiceInstance,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MicroServiceMicroServiceBuildablePackage {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MicroServiceMicroServiceInstance {
    pub instance_id: String,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MicroServiceAuthor {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub first_name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MicroServiceFrontend {
    pub show_details: bool,
}
