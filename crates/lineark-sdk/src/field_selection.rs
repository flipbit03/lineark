//! Type-driven GraphQL field selection.
//!
//! Each type that implements [`GraphQLFields`] knows how to describe its
//! own GraphQL selection string. Generated types return all their scalar
//! fields. Consumers can define custom lean types with only the fields
//! they need — the struct shape *is* the query shape.
//!
//! The `FullType` associated type provides compile-time validation:
//! - Generated types implement `GraphQLFields` with `FullType = Self`
//! - Custom types use `#[graphql(full_type = X)]` to validate fields at compile time

/// Trait implemented by types that know their GraphQL field selection.
///
/// The `FullType` associated type ties this implementation to a specific
/// GraphQL schema type, enabling compile-time validation:
/// - **Generated types** set `FullType = Self` — they validate against themselves.
/// - **Custom lean types** set `FullType` to the corresponding generated type,
///   enabling compile-time field existence and type checks via `#[graphql(full_type = X)]`.
///
/// # Example
///
/// ```ignore
/// use lineark_sdk::generated::types::Team;
///
/// #[derive(Deserialize, GraphQLFields)]
/// #[graphql(full_type = Team)]
/// struct TeamRow {
///     id: String,
///     key: String,
///     name: String,
/// }
///
/// // Compile-time validated: TeamRow fields exist on Team with compatible types
/// let teams = client.teams::<TeamRow>().first(10).send().await?;
/// ```
pub trait GraphQLFields {
    /// The full generated type this implementation validates against.
    type FullType;

    /// Return the GraphQL field selection string for this type.
    ///
    /// For flat types, this is just space-separated field names.
    /// For types with nested objects, include sub-selections:
    /// `"id title team { id name }"`.
    fn selection() -> String;
}

/// Marker trait for compile-time field type compatibility.
///
/// Validates that a full type's field type `Self` is compatible with a custom
/// type's field type `Custom`. Covers common wrapping patterns used in
/// generated types (`Option`, `Box`, `Vec`).
pub trait FieldCompatible<Custom> {}

// Exact match
impl<T> FieldCompatible<T> for T {}

// Unwrap Option: full type has Option<T>, custom type has T
impl<T> FieldCompatible<T> for Option<T> {}

// Unwrap Option<Box<T>>: full type has Option<Box<T>>, custom type has T
impl<T> FieldCompatible<T> for Option<Box<T>> {}

// Unbox, keep Option: full type has Option<Box<T>>, custom type has Option<T>
impl<T> FieldCompatible<Option<T>> for Option<Box<T>> {}

// Cross-type: DateTime serializes as ISO 8601 string in JSON
impl FieldCompatible<String> for chrono::DateTime<chrono::Utc> {}
impl FieldCompatible<Option<String>> for Option<chrono::DateTime<chrono::Utc>> {}
impl FieldCompatible<String> for Option<chrono::DateTime<chrono::Utc>> {}
