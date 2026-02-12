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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_info_deserializes_camel_case() {
        let json = r#"{
            "hasNextPage": true,
            "endCursor": "abc123",
            "hasPreviousPage": false,
            "startCursor": "xyz"
        }"#;
        let pi: PageInfo = serde_json::from_str(json).unwrap();
        assert!(pi.has_next_page);
        assert_eq!(pi.end_cursor, Some("abc123".to_string()));
        assert_eq!(pi.has_previous_page, Some(false));
        assert_eq!(pi.start_cursor, Some("xyz".to_string()));
    }

    #[test]
    fn page_info_defaults() {
        let json = r#"{}"#;
        let pi: PageInfo = serde_json::from_str(json).unwrap();
        assert!(!pi.has_next_page);
        assert!(pi.end_cursor.is_none());
        assert!(pi.has_previous_page.is_none());
        assert!(pi.start_cursor.is_none());
    }

    #[test]
    fn connection_deserializes_with_nodes() {
        let json = r#"{
            "nodes": [{"value": 1}, {"value": 2}],
            "pageInfo": {"hasNextPage": true, "endCursor": "cur"}
        }"#;
        let conn: Connection<serde_json::Value> = serde_json::from_str(json).unwrap();
        assert_eq!(conn.nodes.len(), 2);
        assert!(conn.page_info.has_next_page);
        assert_eq!(conn.page_info.end_cursor, Some("cur".to_string()));
    }

    #[test]
    fn connection_deserializes_empty_nodes() {
        let json = r#"{
            "nodes": [],
            "pageInfo": {"hasNextPage": false}
        }"#;
        let conn: Connection<serde_json::Value> = serde_json::from_str(json).unwrap();
        assert!(conn.nodes.is_empty());
        assert!(!conn.page_info.has_next_page);
    }

    #[test]
    fn connection_defaults_when_missing() {
        let json = r#"{}"#;
        let conn: Connection<serde_json::Value> = serde_json::from_str(json).unwrap();
        assert!(conn.nodes.is_empty());
        assert!(!conn.page_info.has_next_page);
    }

    #[test]
    fn page_info_serializes_camel_case() {
        let pi = PageInfo {
            has_next_page: true,
            end_cursor: Some("abc".to_string()),
            has_previous_page: Some(false),
            start_cursor: None,
        };
        let json = serde_json::to_value(&pi).unwrap();
        assert_eq!(json["hasNextPage"], true);
        assert_eq!(json["endCursor"], "abc");
    }
}
