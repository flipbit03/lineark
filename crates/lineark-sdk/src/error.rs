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
