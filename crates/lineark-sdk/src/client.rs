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
            .post(LINEAR_API_URL)
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
                return Err(LinearError::GraphQL(errors));
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
}
