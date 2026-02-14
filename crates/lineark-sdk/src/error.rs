//! Error types for the Linear SDK.
//!
//! [`LinearError`] covers authentication failures, HTTP transport errors,
//! GraphQL-level errors, rate limiting, and more.

use serde::{Deserialize, Serialize};
use std::fmt;

/// A single GraphQL error from the API response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLError {
    pub message: String,
    #[serde(default)]
    pub extensions: Option<serde_json::Value>,
    #[serde(default)]
    pub path: Option<Vec<serde_json::Value>>,
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
    GraphQL {
        errors: Vec<GraphQLError>,
        query_name: Option<String>,
    },
    /// The requested data path was not found in the response.
    MissingData(String),
    /// Non-2xx HTTP response not covered by a more specific variant.
    HttpError { status: u16, body: String },
    /// Auth configuration error (no token found).
    AuthConfig(String),
    /// Internal error (e.g. runtime creation failure).
    Internal(String),
}

impl fmt::Display for LinearError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Authentication(msg) => write!(f, "Authentication error: {}", msg),
            Self::RateLimited { message, .. } => write!(f, "Rate limited: {}", message),
            Self::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            Self::Forbidden(msg) => write!(f, "Forbidden: {}", msg),
            Self::Network(e) => write!(f, "Network error: {}", e),
            Self::GraphQL { errors, query_name } => {
                let msgs: Vec<String> = errors
                    .iter()
                    .map(|e| {
                        let mut parts = vec![e.message.clone()];
                        if let Some(path) = &e.path {
                            let path_str: Vec<String> =
                                path.iter().map(|p| p.to_string()).collect();
                            parts.push(format!("at {}", path_str.join(".")));
                        }
                        if let Some(ext) = &e.extensions {
                            parts.push(format!("({})", ext));
                        }
                        parts.join(" ")
                    })
                    .collect();
                if let Some(name) = query_name {
                    write!(f, "GraphQL errors in {}: {}", name, msgs.join("; "))
                } else {
                    write!(f, "GraphQL errors: {}", msgs.join("; "))
                }
            }
            Self::HttpError { status, body } => {
                write!(f, "HTTP error {}: {}", status, body)
            }
            Self::MissingData(path) => write!(f, "Missing data at path: {}", path),
            Self::AuthConfig(msg) => write!(f, "Auth configuration error: {}", msg),
            Self::Internal(msg) => write!(f, "Internal error: {}", msg),
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
        let err = LinearError::GraphQL {
            errors: vec![GraphQLError {
                message: "Field not found".to_string(),
                extensions: None,
                path: None,
            }],
            query_name: None,
        };
        assert_eq!(err.to_string(), "GraphQL errors: Field not found");
    }

    #[test]
    fn display_graphql_error_with_extensions() {
        let err = LinearError::GraphQL {
            errors: vec![GraphQLError {
                message: "Error".to_string(),
                extensions: Some(serde_json::json!({"code": "VALIDATION"})),
                path: None,
            }],
            query_name: None,
        };
        let display = err.to_string();
        assert!(display.contains("Error"));
        assert!(display.contains("VALIDATION"));
    }

    #[test]
    fn display_graphql_error_multiple() {
        let err = LinearError::GraphQL {
            errors: vec![
                GraphQLError {
                    message: "Error 1".to_string(),
                    extensions: None,
                    path: None,
                },
                GraphQLError {
                    message: "Error 2".to_string(),
                    extensions: None,
                    path: None,
                },
            ],
            query_name: None,
        };
        let display = err.to_string();
        assert!(display.contains("Error 1"));
        assert!(display.contains("Error 2"));
        assert!(display.contains("; "));
    }

    #[test]
    fn display_graphql_error_with_query_name() {
        let err = LinearError::GraphQL {
            errors: vec![GraphQLError {
                message: "Internal server error".to_string(),
                extensions: None,
                path: Some(vec![
                    serde_json::json!("viewer"),
                    serde_json::json!("drafts"),
                    serde_json::json!("nodes"),
                    serde_json::json!(0),
                    serde_json::json!("customerNeed"),
                ]),
            }],
            query_name: Some("Viewer".to_string()),
        };
        let display = err.to_string();
        assert!(display.contains("in Viewer"));
        assert!(display.contains("at \"viewer\""));
        assert!(display.contains("\"customerNeed\""));
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
            path: None,
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
    fn display_internal_error() {
        let err = LinearError::Internal("Failed to create tokio runtime: foo".to_string());
        assert_eq!(
            err.to_string(),
            "Internal error: Failed to create tokio runtime: foo"
        );
    }

    #[test]
    fn network_error_has_source() {
        // We can't easily construct a reqwest::Error directly, but we can verify
        // the source() method returns None for non-Network variants.
        let err = LinearError::Authentication("test".to_string());
        assert!(std::error::Error::source(&err).is_none());
    }
}
