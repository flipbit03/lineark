pub mod auth;
#[cfg(feature = "blocking")]
pub mod blocking;
pub mod client;
pub mod error;
pub mod generated;
pub mod helpers;
pub mod pagination;

// Re-export key types at crate root for convenience.
pub use client::Client;
pub use error::LinearError;
pub use pagination::{Connection, PageInfo};
