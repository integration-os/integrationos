use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserClient {
    #[serde(rename = "_id")]
    pub id: String,
    #[serde(rename = "buildableId")]
    pub buildable_id: String,
    pub name: String,
    pub author: Author,
    pub containers: Vec<Container>,
    pub billing: Option<Billing>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Author {
    #[serde(rename = "_id")]
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Container {
    #[serde(rename = "_id")]
    pub id: String,
    pub subscription: Subscription,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub tier: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Billing {
    #[serde(default = "default_throughput")]
    pub throughput: u64,
    pub provider: Option<String>,
    #[serde(rename = "customerId")]
    pub customer_id: String,
    pub subscription: BillingSubscription,
}

fn default_throughput() -> u64 {
    500
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingSubscription {
    pub id: String,
    #[serde(rename = "endDate")]
    pub end_date: i64,
    pub valid: bool,
    pub key: String,
    pub reason: Option<String>,
}
