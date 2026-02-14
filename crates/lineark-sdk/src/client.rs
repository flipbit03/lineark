//! Async Linear API client.
//!
//! The primary entry point for interacting with Linear's GraphQL API.
//! Construct a [`Client`] via [`Client::auto`], [`Client::from_env`],
//! [`Client::from_file`], or [`Client::from_token`], then call generated
//! query and mutation methods.

use crate::auth;
use crate::error::{GraphQLError, LinearError};
use crate::pagination::Connection;
use serde::de::DeserializeOwned;

const LINEAR_API_URL: &str = "https://api.linear.app/graphql";

/// The Linear API client.
#[derive(Debug, Clone)]
pub struct Client {
    http: reqwest::Client,
    token: String,
    base_url: String,
}

/// Raw GraphQL response shape.
#[derive(serde::Deserialize)]
struct GraphQLResponse {
    data: Option<serde_json::Value>,
    errors: Option<Vec<GraphQLError>>,
}

impl Client {
    /// Create a client with an explicit API token.
    pub fn from_token(token: impl Into<String>) -> Result<Self, LinearError> {
        let token = token.into();
        if token.is_empty() {
            return Err(LinearError::AuthConfig("Token cannot be empty".to_string()));
        }
        Ok(Self {
            http: reqwest::Client::new(),
            token,
            base_url: LINEAR_API_URL.to_string(),
        })
    }

    /// Create a client from the `LINEAR_API_TOKEN` environment variable.
    pub fn from_env() -> Result<Self, LinearError> {
        Self::from_token(auth::token_from_env()?)
    }

    /// Create a client from the `~/.linear_api_token` file.
    pub fn from_file() -> Result<Self, LinearError> {
        Self::from_token(auth::token_from_file()?)
    }

    /// Create a client by auto-detecting the token (env -> file).
    pub fn auto() -> Result<Self, LinearError> {
        Self::from_token(auth::auto_token()?)
    }

    /// Execute a GraphQL query and extract a single object from the response.
    pub async fn execute<T: DeserializeOwned>(
        &self,
        query: &str,
        variables: serde_json::Value,
        data_path: &str,
    ) -> Result<T, LinearError> {
        let body = serde_json::json!({
            "query": query,
            "variables": variables,
        });

        let response = self
            .http
            .post(&self.base_url)
            .header("Authorization", &self.token)
            .header("Content-Type", "application/json")
            .header(
                "User-Agent",
                format!("lineark-sdk/{}", env!("CARGO_PKG_VERSION")),
            )
            .json(&body)
            .send()
            .await?;

        let status = response.status();
        if status == 401 || status == 403 {
            let text = response.text().await.unwrap_or_default();
            if status == 401 {
                return Err(LinearError::Authentication(text));
            }
            return Err(LinearError::Forbidden(text));
        }
        if status == 429 {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<f64>().ok());
            let text = response.text().await.unwrap_or_default();
            return Err(LinearError::RateLimited {
                retry_after,
                message: text,
            });
        }
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(LinearError::HttpError {
                status: status.as_u16(),
                body,
            });
        }

        let gql_response: GraphQLResponse = response.json().await?;

        // Check for GraphQL-level errors.
        if let Some(errors) = gql_response.errors {
            if !errors.is_empty() {
                // Check for specific error types.
                let first_msg = errors[0].message.to_lowercase();
                if first_msg.contains("authentication") || first_msg.contains("unauthorized") {
                    return Err(LinearError::Authentication(errors[0].message.clone()));
                }
                // Extract operation name from query string (e.g. "query Viewer { ... }" → "Viewer").
                let query_name = query
                    .strip_prefix("query ")
                    .or_else(|| query.strip_prefix("mutation "))
                    .and_then(|rest| rest.split(['(', ' ', '{']).next())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string());
                return Err(LinearError::GraphQL { errors, query_name });
            }
        }

        let data = gql_response
            .data
            .ok_or_else(|| LinearError::MissingData("No data in response".to_string()))?;

        let value = data
            .get(data_path)
            .ok_or_else(|| {
                LinearError::MissingData(format!("No '{}' in response data", data_path))
            })?
            .clone();

        serde_json::from_value(value).map_err(|e| {
            LinearError::MissingData(format!("Failed to deserialize '{}': {}", data_path, e))
        })
    }

    /// Execute a GraphQL query and extract a Connection from the response.
    pub async fn execute_connection<T: DeserializeOwned>(
        &self,
        query: &str,
        variables: serde_json::Value,
        data_path: &str,
    ) -> Result<Connection<T>, LinearError> {
        self.execute::<Connection<T>>(query, variables, data_path)
            .await
    }

    /// Execute a typed query using the type's [`GraphQLFields`](crate::GraphQLFields) implementation.
    ///
    /// Builds the query from `T::selection()` — define a struct with only
    /// the fields you need for zero-overfetch queries.
    ///
    /// ```ignore
    /// #[derive(Deserialize)]
    /// struct MyViewer { name: Option<String>, email: Option<String> }
    ///
    /// impl GraphQLFields for MyViewer {
    ///     fn selection() -> String { "name email".into() }
    /// }
    ///
    /// let me: MyViewer = client.query::<MyViewer>("viewer").await?;
    /// ```
    pub async fn query<T: DeserializeOwned + crate::GraphQLFields>(
        &self,
        field: &str,
    ) -> Result<T, LinearError> {
        let selection = T::selection();
        let query = format!("query {{ {} {{ {} }} }}", field, selection);
        self.execute::<T>(&query, serde_json::json!({}), field)
            .await
    }

    /// Execute a typed connection query using the node type's
    /// [`GraphQLFields`](crate::GraphQLFields) implementation.
    ///
    /// Builds `{ field { nodes { <T::selection()> } pageInfo { ... } } }`.
    pub async fn query_connection<T: DeserializeOwned + crate::GraphQLFields>(
        &self,
        field: &str,
    ) -> Result<Connection<T>, LinearError> {
        let selection = T::selection();
        let query = format!(
            "query {{ {} {{ nodes {{ {} }} pageInfo {{ hasNextPage endCursor }} }} }}",
            field, selection
        );
        self.execute_connection::<T>(&query, serde_json::json!({}), field)
            .await
    }

    /// Execute a mutation, check `success`, and extract the entity field.
    ///
    /// Many Linear mutations return a payload shaped like
    /// `{ success: Boolean, entityField: { ... } }`. This helper:
    /// 1. Executes the query and extracts the payload at `data_path`
    /// 2. Checks the `success` field — returns an error if false
    /// 3. Extracts and deserializes `payload[entity_field]` as `T`
    pub(crate) async fn execute_mutation<T: DeserializeOwned>(
        &self,
        query: &str,
        variables: serde_json::Value,
        data_path: &str,
        entity_field: &str,
    ) -> Result<T, LinearError> {
        let payload = self
            .execute::<serde_json::Value>(query, variables, data_path)
            .await?;

        // Check success field.
        if payload.get("success").and_then(|v| v.as_bool()) != Some(true) {
            return Err(LinearError::Internal(format!(
                "Mutation '{}' failed: {}",
                data_path,
                serde_json::to_string_pretty(&payload).unwrap_or_default()
            )));
        }

        // Extract and deserialize the entity.
        let entity = payload
            .get(entity_field)
            .ok_or_else(|| {
                LinearError::MissingData(format!(
                    "No '{}' field in '{}' payload",
                    entity_field, data_path
                ))
            })?
            .clone();

        serde_json::from_value(entity).map_err(|e| {
            LinearError::MissingData(format!(
                "Failed to deserialize '{}' from '{}': {}",
                entity_field, data_path, e
            ))
        })
    }

    /// Access the underlying HTTP client.
    ///
    /// Used internally by [`helpers`](crate::helpers) for file download/upload
    /// operations that go outside the GraphQL API.
    pub(crate) fn http(&self) -> &reqwest::Client {
        &self.http
    }

    pub(crate) fn token(&self) -> &str {
        &self.token
    }

    /// Override the base URL (for testing against mock servers).
    #[cfg(test)]
    pub(crate) fn with_base_url(mut self, url: String) -> Self {
        self.base_url = url;
        self
    }

    /// Allow integration tests (in tests/ directory) to set base URL.
    #[doc(hidden)]
    pub fn set_base_url(&mut self, url: String) {
        self.base_url = url;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{header, method};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn from_token_valid() {
        let client = Client::from_token("lin_api_test123").unwrap();
        assert_eq!(client.token, "lin_api_test123");
        assert_eq!(client.base_url, LINEAR_API_URL);
    }

    #[test]
    fn from_token_empty_fails() {
        let err = Client::from_token("").unwrap_err();
        assert!(matches!(err, LinearError::AuthConfig(_)));
        assert!(err.to_string().contains("empty"));
    }

    #[tokio::test]
    async fn execute_returns_401_as_authentication_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
            .mount(&server)
            .await;

        let client = Client::from_token("bad-token")
            .unwrap()
            .with_base_url(server.uri());

        let result = client
            .execute::<serde_json::Value>(
                "query { viewer { id } }",
                serde_json::json!({}),
                "viewer",
            )
            .await;

        assert!(matches!(result, Err(LinearError::Authentication(_))));
    }

    #[tokio::test]
    async fn execute_returns_403_as_forbidden_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(403).set_body_string("Forbidden"))
            .mount(&server)
            .await;

        let client = Client::from_token("token")
            .unwrap()
            .with_base_url(server.uri());

        let result = client
            .execute::<serde_json::Value>(
                "query { viewer { id } }",
                serde_json::json!({}),
                "viewer",
            )
            .await;

        assert!(matches!(result, Err(LinearError::Forbidden(_))));
    }

    #[tokio::test]
    async fn execute_returns_429_as_rate_limited_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(429)
                    .append_header("retry-after", "30")
                    .set_body_string("Too Many Requests"),
            )
            .mount(&server)
            .await;

        let client = Client::from_token("token")
            .unwrap()
            .with_base_url(server.uri());

        let result = client
            .execute::<serde_json::Value>(
                "query { viewer { id } }",
                serde_json::json!({}),
                "viewer",
            )
            .await;

        match result {
            Err(LinearError::RateLimited {
                retry_after,
                message,
            }) => {
                assert_eq!(retry_after, Some(30.0));
                assert_eq!(message, "Too Many Requests");
            }
            other => panic!("Expected RateLimited, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn execute_returns_500_as_http_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .mount(&server)
            .await;

        let client = Client::from_token("token")
            .unwrap()
            .with_base_url(server.uri());

        let result = client
            .execute::<serde_json::Value>(
                "query { viewer { id } }",
                serde_json::json!({}),
                "viewer",
            )
            .await;

        match result {
            Err(LinearError::HttpError { status, body }) => {
                assert_eq!(status, 500);
                assert_eq!(body, "Internal Server Error");
            }
            other => panic!("Expected HttpError, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn execute_returns_graphql_errors() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": null,
                "errors": [{"message": "Field 'foo' not found"}]
            })))
            .mount(&server)
            .await;

        let client = Client::from_token("token")
            .unwrap()
            .with_base_url(server.uri());

        let result = client
            .execute::<serde_json::Value>("query { foo }", serde_json::json!({}), "foo")
            .await;

        assert!(matches!(result, Err(LinearError::GraphQL { .. })));
    }

    #[tokio::test]
    async fn execute_graphql_auth_error_detected() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": null,
                "errors": [{"message": "Authentication required"}]
            })))
            .mount(&server)
            .await;

        let client = Client::from_token("token")
            .unwrap()
            .with_base_url(server.uri());

        let result = client
            .execute::<serde_json::Value>(
                "query { viewer { id } }",
                serde_json::json!({}),
                "viewer",
            )
            .await;

        assert!(matches!(result, Err(LinearError::Authentication(_))));
    }

    #[tokio::test]
    async fn execute_missing_data_path() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {"other": {"id": "123"}}
            })))
            .mount(&server)
            .await;

        let client = Client::from_token("token")
            .unwrap()
            .with_base_url(server.uri());

        let result = client
            .execute::<serde_json::Value>(
                "query { viewer { id } }",
                serde_json::json!({}),
                "viewer",
            )
            .await;

        match result {
            Err(LinearError::MissingData(msg)) => {
                assert!(msg.contains("viewer"));
            }
            other => panic!("Expected MissingData, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn execute_no_data_in_response() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": null
            })))
            .mount(&server)
            .await;

        let client = Client::from_token("token")
            .unwrap()
            .with_base_url(server.uri());

        let result = client
            .execute::<serde_json::Value>(
                "query { viewer { id } }",
                serde_json::json!({}),
                "viewer",
            )
            .await;

        assert!(matches!(result, Err(LinearError::MissingData(_))));
    }

    #[tokio::test]
    async fn execute_success_deserializes() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {
                    "viewer": {
                        "id": "user-123",
                        "name": "Test User",
                        "email": "test@example.com",
                        "active": true
                    }
                }
            })))
            .mount(&server)
            .await;

        let client = Client::from_token("token")
            .unwrap()
            .with_base_url(server.uri());

        let result: serde_json::Value = client
            .execute("query { viewer { id } }", serde_json::json!({}), "viewer")
            .await
            .unwrap();

        assert_eq!(result["id"], "user-123");
        assert_eq!(result["name"], "Test User");
    }

    #[tokio::test]
    async fn execute_connection_deserializes() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {
                    "teams": {
                        "nodes": [
                            {"id": "team-1", "name": "Engineering", "key": "ENG"},
                            {"id": "team-2", "name": "Design", "key": "DES"}
                        ],
                        "pageInfo": {
                            "hasNextPage": false,
                            "endCursor": "cursor-abc"
                        }
                    }
                }
            })))
            .mount(&server)
            .await;

        let client = Client::from_token("token")
            .unwrap()
            .with_base_url(server.uri());

        let conn: Connection<serde_json::Value> = client
            .execute_connection(
                "query { teams { nodes { id } pageInfo { hasNextPage endCursor } } }",
                serde_json::json!({}),
                "teams",
            )
            .await
            .unwrap();

        assert_eq!(conn.nodes.len(), 2);
        assert_eq!(conn.nodes[0]["id"], "team-1");
        assert!(!conn.page_info.has_next_page);
        assert_eq!(conn.page_info.end_cursor, Some("cursor-abc".to_string()));
    }

    #[tokio::test]
    async fn execute_sends_authorization_header() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(header("Authorization", "my-secret-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {"viewer": {"id": "1"}}
            })))
            .mount(&server)
            .await;

        let client = Client::from_token("my-secret-token")
            .unwrap()
            .with_base_url(server.uri());

        let result: serde_json::Value = client
            .execute("query { viewer { id } }", serde_json::json!({}), "viewer")
            .await
            .unwrap();

        assert_eq!(result["id"], "1");
    }
}
