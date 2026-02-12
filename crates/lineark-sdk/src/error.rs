use serde::{Deserialize, Serialize};
use std::fmt;

/// A single GraphQL error from the API response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLError {
    pub message: String,
    #[serde(default)]
    pub extensions: Option<serde_json::Value>,
}

/// Errors that can occur when interacting with the Linear API.
#[derive(Debug)]
pub enum LinearError {
    /// Authentication failed (invalid or expired token).
    Authentication(String),
    /// Request was rate-limited.
    RateLimited {
        retry_after: Option<f64>,
        message: String,
    },
    /// Invalid input (bad arguments to a mutation).
    InvalidInput(String),
    /// Forbidden (insufficient permissions).
    Forbidden(String),
    /// Network or HTTP transport error.
    Network(reqwest::Error),
    /// GraphQL errors returned by the API.
    GraphQL(Vec<GraphQLError>),
    /// The requested data path was not found in the response.
    MissingData(String),
    /// Non-2xx HTTP response not covered by a more specific variant.
    HttpError { status: u16, body: String },
    /// Auth configuration error (no token found).
    AuthConfig(String),
}

impl fmt::Display for LinearError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Authentication(msg) => write!(f, "Authentication error: {}", msg),
            Self::RateLimited { message, .. } => write!(f, "Rate limited: {}", message),
            Self::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            Self::Forbidden(msg) => write!(f, "Forbidden: {}", msg),
            Self::Network(e) => write!(f, "Network error: {}", e),
            Self::GraphQL(errors) => {
                let msgs: Vec<String> = errors
                    .iter()
                    .map(|e| {
                        if let Some(ext) = &e.extensions {
                            format!("{} ({})", e.message, ext)
                        } else {
                            e.message.clone()
                        }
                    })
                    .collect();
                write!(f, "GraphQL errors: {}", msgs.join("; "))
            }
            Self::HttpError { status, body } => {
                write!(f, "HTTP error {}: {}", status, body)
            }
            Self::MissingData(path) => write!(f, "Missing data at path: {}", path),
            Self::AuthConfig(msg) => write!(f, "Auth configuration error: {}", msg),
        }
    }
}

impl std::error::Error for LinearError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Network(e) => Some(e),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for LinearError {
    fn from(e: reqwest::Error) -> Self {
        Self::Network(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_authentication_error() {
        let err = LinearError::Authentication("Invalid token".to_string());
        assert_eq!(err.to_string(), "Authentication error: Invalid token");
    }

    #[test]
    fn display_rate_limited_error() {
        let err = LinearError::RateLimited {
            retry_after: Some(30.0),
            message: "Too many requests".to_string(),
        };
        assert_eq!(err.to_string(), "Rate limited: Too many requests");
    }

    #[test]
    fn display_invalid_input_error() {
        let err = LinearError::InvalidInput("bad field".to_string());
        assert_eq!(err.to_string(), "Invalid input: bad field");
    }

    #[test]
    fn display_forbidden_error() {
        let err = LinearError::Forbidden("not allowed".to_string());
        assert_eq!(err.to_string(), "Forbidden: not allowed");
    }

    #[test]
    fn display_graphql_error_single() {
        let err = LinearError::GraphQL(vec![GraphQLError {
            message: "Field not found".to_string(),
            extensions: None,
        }]);
        assert_eq!(err.to_string(), "GraphQL errors: Field not found");
    }

    #[test]
    fn display_graphql_error_with_extensions() {
        let err = LinearError::GraphQL(vec![GraphQLError {
            message: "Error".to_string(),
            extensions: Some(serde_json::json!({"code": "VALIDATION"})),
        }]);
        let display = err.to_string();
        assert!(display.contains("Error"));
        assert!(display.contains("VALIDATION"));
    }

    #[test]
    fn display_graphql_error_multiple() {
        let err = LinearError::GraphQL(vec![
            GraphQLError {
                message: "Error 1".to_string(),
                extensions: None,
            },
            GraphQLError {
                message: "Error 2".to_string(),
                extensions: None,
            },
        ]);
        let display = err.to_string();
        assert!(display.contains("Error 1"));
        assert!(display.contains("Error 2"));
        assert!(display.contains("; "));
    }

    #[test]
    fn display_http_error() {
        let err = LinearError::HttpError {
            status: 500,
            body: "Internal Server Error".to_string(),
        };
        assert_eq!(err.to_string(), "HTTP error 500: Internal Server Error");
    }

    #[test]
    fn display_missing_data_error() {
        let err = LinearError::MissingData("No 'viewer' in response data".to_string());
        assert_eq!(
            err.to_string(),
            "Missing data at path: No 'viewer' in response data"
        );
    }

    #[test]
    fn display_auth_config_error() {
        let err = LinearError::AuthConfig("Token file not found".to_string());
        assert_eq!(
            err.to_string(),
            "Auth configuration error: Token file not found"
        );
    }

    #[test]
    fn graphql_error_deserializes() {
        let json = r#"{"message": "Something failed", "extensions": {"code": "BAD_INPUT"}}"#;
        let err: GraphQLError = serde_json::from_str(json).unwrap();
        assert_eq!(err.message, "Something failed");
        assert!(err.extensions.is_some());
    }

    #[test]
    fn graphql_error_deserializes_without_extensions() {
        let json = r#"{"message": "Something failed"}"#;
        let err: GraphQLError = serde_json::from_str(json).unwrap();
        assert_eq!(err.message, "Something failed");
        assert!(err.extensions.is_none());
    }

    #[test]
    fn graphql_error_serializes() {
        let err = GraphQLError {
            message: "test".to_string(),
            extensions: None,
        };
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["message"], "test");
    }

    #[test]
    fn linear_error_is_std_error() {
        let err = LinearError::Authentication("test".to_string());
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn network_error_has_source() {
        // We can't easily construct a reqwest::Error directly, but we can verify
        // the source() method returns None for non-Network variants.
        let err = LinearError::Authentication("test".to_string());
        assert!(std::error::Error::source(&err).is_none());
    }
}
