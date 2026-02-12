use serde::{Deserialize, Serialize};

/// Relay-style page info for cursor-based pagination.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PageInfo {
    pub has_next_page: bool,
    pub end_cursor: Option<String>,
    pub has_previous_page: Option<bool>,
    pub start_cursor: Option<String>,
}

/// A paginated collection of nodes with page info.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(bound(deserialize = "T: serde::de::DeserializeOwned"))]
pub struct Connection<T> {
    #[serde(default)]
    pub nodes: Vec<T>,
    #[serde(rename = "pageInfo", default)]
    pub page_info: PageInfo,
}
