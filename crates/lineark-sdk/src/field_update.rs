//! Three-state wrapper for nullable input fields.
//!
//! GraphQL distinguishes between an omitted field ("don't change this") and an
//! explicit `null` ("clear this field"). `Option<T>` + `skip_serializing_if`
//! can only express the first. [`MaybeUndefined<T>`] carries both, so
//! generated input types can drive the Linear API faithfully without hand-rolled
//! JSON patches on the consumer side.
//!
//! Codegen emits nullable input fields as:
//!
//! ```rust,ignore
//! #[serde(default, skip_serializing_if = "MaybeUndefined::is_undefined")]
//! pub lead_id: MaybeUndefined<String>,
//! ```
//!
//! Consumers choose one of:
//!
//! | Intent                    | Value                          | Wire form       |
//! |---------------------------|--------------------------------|-----------------|
//! | Leave unchanged           | `MaybeUndefined::Undefined`    | field omitted   |
//! | Clear on the server       | `MaybeUndefined::Null`         | `"field": null` |
//! | Set to a value            | `MaybeUndefined::Value(v)`     | `"field": v`    |

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A three-state field value: undefined (omitted), null (explicit clear), or a concrete value.
///
/// See the [module documentation](self) for the rationale and wire-format mapping.
///
/// # Struct-context contract
///
/// The [`Undefined`](MaybeUndefined::Undefined) / [`Null`](MaybeUndefined::Null)
/// distinction is preserved on the wire **only** when this value sits in a
/// struct field carrying
/// `#[serde(default, skip_serializing_if = "MaybeUndefined::is_undefined")]`
/// — which is what codegen emits for every nullable input field. In any other
/// context (`serde_json::to_value(MaybeUndefined::<T>::Undefined)`, a bare
/// value inside a `Vec<MaybeUndefined<T>>`, etc.) `Undefined` cannot be
/// "omitted" — there's no containing struct to omit it from — and serializes
/// as JSON `null`, collapsing the distinction. Use this type *only* as a
/// struct field paired with the skip predicate above.
///
/// # Derive bounds
///
/// The [`Eq`] and [`Hash`] impls are conditional on `T: Eq + Hash`. Types
/// containing non-`Eq` scalars (notably `f64`) therefore can't derive those
/// traits transitively — this is expected and matches `Option<T>`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MaybeUndefined<T> {
    /// Field is absent from the serialized output.
    Undefined,
    /// Field is serialized as JSON `null` (clears the value on the server).
    Null,
    /// Field is serialized as the wrapped value.
    Value(T),
}

impl<T> MaybeUndefined<T> {
    /// Returns `true` if the value is [`MaybeUndefined::Undefined`].
    ///
    /// Codegen uses this as the `skip_serializing_if` predicate so `Undefined`
    /// fields are omitted from the serialized output entirely.
    pub fn is_undefined(&self) -> bool {
        matches!(self, Self::Undefined)
    }
}

// Manual impl (not `#[derive(Default)]`) so the `Default` bound on `T` is
// avoided — consumers need `MaybeUndefined::<T>::default()` to work for any T,
// not just those that themselves implement `Default`.
#[allow(clippy::derivable_impls)]
impl<T> Default for MaybeUndefined<T> {
    fn default() -> Self {
        Self::Undefined
    }
}

impl<T> From<T> for MaybeUndefined<T> {
    fn from(v: T) -> Self {
        Self::Value(v)
    }
}

/// Lifts an `Option<T>` into the three-state world, collapsing `None` to
/// [`Undefined`](MaybeUndefined::Undefined).
///
/// This is the right default for **constructing** input values: a CLI flag
/// that wasn't passed (`Option::None`) means "leave the field unchanged", so
/// it maps to `Undefined`. If you instead want `None` to clear the field on
/// the server, use [`MaybeUndefined::Null`] explicitly.
///
/// Note the intentional asymmetry with [`Deserialize`]: when round-tripping
/// through JSON, an absent field deserializes to `Undefined` (via the
/// `#[serde(default)]` on the field), while an explicit JSON `null`
/// deserializes to `Null`. That matches GraphQL's wire semantics. `Option<T>`
/// doesn't carry the "absent" vs "null" distinction, so `From<Option<T>>`
/// can't preserve it either.
impl<T> From<Option<T>> for MaybeUndefined<T> {
    fn from(o: Option<T>) -> Self {
        match o {
            Some(v) => Self::Value(v),
            None => Self::Undefined,
        }
    }
}

impl<T: Serialize> Serialize for MaybeUndefined<T> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            // Unreachable in normal use: codegen emits
            // `skip_serializing_if = "MaybeUndefined::is_undefined"`, so serde
            // never asks us to serialize the Undefined variant on struct fields.
            Self::Undefined => s.serialize_none(),
            Self::Null => s.serialize_none(),
            Self::Value(v) => v.serialize(s),
        }
    }
}

/// Maps the three JSON inputs a struct field can present as follows:
///
/// | Input                          | Result      |
/// |--------------------------------|-------------|
/// | field absent from JSON         | `Undefined` (via `#[serde(default)]`) |
/// | field present with `null`      | `Null`      |
/// | field present with a value `v` | `Value(v)`  |
///
/// Absent-field handling is driven by `#[serde(default)]` on the struct
/// field, not by this impl — serde only calls `deserialize` when the key is
/// present. Codegen emits that attribute on every nullable input field, which
/// is what preserves the three-state distinction on round-trip.
///
/// Note the intentional asymmetry with [`From<Option<T>>`]: `None → Undefined`
/// during construction (a missing CLI flag shouldn't touch the server),
/// but JSON `null → Null` during deserialization (the server *did* send
/// `null`).
impl<'de, T: Deserialize<'de>> Deserialize<'de> for MaybeUndefined<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Option::<T>::deserialize(d).map(|o| match o {
            Some(v) => Self::Value(v),
            None => Self::Null,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
    struct Host {
        #[serde(default, skip_serializing_if = "MaybeUndefined::is_undefined")]
        field: MaybeUndefined<String>,
    }

    #[test]
    fn default_is_undefined() {
        let m: MaybeUndefined<String> = MaybeUndefined::default();
        assert!(matches!(m, MaybeUndefined::Undefined));
        assert!(m.is_undefined());
    }

    #[test]
    fn from_value_is_value() {
        let m: MaybeUndefined<String> = MaybeUndefined::from("hi".to_string());
        assert_eq!(m, MaybeUndefined::Value("hi".to_string()));
    }

    #[test]
    fn from_option_maps_correctly() {
        let some: MaybeUndefined<i32> = MaybeUndefined::from(Some(5));
        let none: MaybeUndefined<i32> = MaybeUndefined::from(Option::<i32>::None);
        assert_eq!(some, MaybeUndefined::Value(5));
        assert_eq!(none, MaybeUndefined::Undefined);
    }

    #[test]
    fn serialize_value_emits_value() {
        let host = Host {
            field: MaybeUndefined::Value("hello".to_string()),
        };
        assert_eq!(
            serde_json::to_string(&host).unwrap(),
            r#"{"field":"hello"}"#
        );
    }

    #[test]
    fn serialize_null_emits_null() {
        let host = Host {
            field: MaybeUndefined::Null,
        };
        assert_eq!(serde_json::to_string(&host).unwrap(), r#"{"field":null}"#);
    }

    #[test]
    fn serialize_undefined_is_skipped() {
        let host = Host {
            field: MaybeUndefined::Undefined,
        };
        assert_eq!(serde_json::to_string(&host).unwrap(), r#"{}"#);
    }

    #[test]
    fn deserialize_value_is_value() {
        let host: Host = serde_json::from_str(r#"{"field":"hello"}"#).unwrap();
        assert_eq!(host.field, MaybeUndefined::Value("hello".to_string()));
    }

    #[test]
    fn deserialize_null_is_null() {
        let host: Host = serde_json::from_str(r#"{"field":null}"#).unwrap();
        assert_eq!(host.field, MaybeUndefined::Null);
    }

    #[test]
    fn deserialize_absent_is_undefined() {
        let host: Host = serde_json::from_str(r#"{}"#).unwrap();
        assert_eq!(host.field, MaybeUndefined::Undefined);
    }
}
