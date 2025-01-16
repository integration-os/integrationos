pub mod algebra;
pub mod client;
pub mod domain;
pub mod unified;
pub mod utility;

pub const ID_KEY: &str = "id";
pub const BODY_KEY: &str = "body";
pub const MODIFY_TOKEN_KEY: &str = "modifyToken";
pub const PASSTHROUGH_PARAMS: &str = "passthroughForward";
pub const PASSTHROUGH_HEADERS: &str = "x-pica-passthrough-forward";
pub const UNIFIED_KEY: &str = "unified";
pub const COUNT_KEY: &str = "count";
pub const PASSTHROUGH_KEY: &str = "passthrough";
pub const LIMIT_KEY: &str = "limit";
pub const PAGE_SIZE_KEY: &str = "pageSize";
pub const PAGINATION_KEY: &str = "pagination";
pub const STATUS_HEADER_KEY: &str = "response-status";
pub const META_KEY: &str = "meta";
