//! Type-driven GraphQL field selection.
//!
//! Each type that implements [`GraphQLFields`] knows how to describe its
//! own GraphQL selection string. Generated types return all their scalar
//! fields. Consumers can define custom lean types with only the fields
//! they need — the struct shape *is* the query shape.

/// Trait implemented by types that know their GraphQL field selection.
///
/// Enables zero-overfetch queries: define a Rust struct with only the
/// fields you need, implement this trait, and pass it as `<T>` to
/// [`Client::query`](crate::Client::query). The SDK builds a query
/// that fetches exactly those fields.
///
/// # Example
///
/// ```ignore
/// #[derive(Deserialize)]
/// struct MyViewer {
///     name: Option<String>,
///     email: Option<String>,
/// }
///
/// impl GraphQLFields for MyViewer {
///     fn selection() -> String {
///         "name email".into()
///     }
/// }
///
/// let me: MyViewer = client.query::<MyViewer>("viewer").await?;
/// // Sends: query { viewer { name email } }
/// ```
pub trait GraphQLFields {
    /// Return the GraphQL field selection string for this type.
    ///
    /// For flat types, this is just space-separated field names.
    /// For types with nested objects, include sub-selections:
    /// `"id title team { id name }"`.
    fn selection() -> String;
}

/// Blanket impl for `serde_json::Value` — selects only `id`.
///
/// Use this when you want a quick untyped result. For proper field
/// selection, prefer a concrete struct that derives `GraphQLFields`.
impl GraphQLFields for serde_json::Value {
    fn selection() -> String {
        "id".into()
    }
}
