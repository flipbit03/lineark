#![recursion_limit = "256"]

pub mod auth;
#[cfg(feature = "blocking")]
pub mod blocking_client;
pub mod client;
pub mod error;
pub mod field_selection;
pub mod generated;
pub mod helpers;
pub mod pagination;

// Re-export key types at crate root for convenience.
pub use client::Client;
pub use error::LinearError;
pub use field_selection::FieldCompatible;
pub use field_selection::GraphQLFields;
pub use lineark_derive::GraphQLFields;
pub use pagination::{Connection, PageInfo};
