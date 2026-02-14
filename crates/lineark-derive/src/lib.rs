//! Derive macro for the `GraphQLFields` trait.
//!
//! Automatically generates `selection()` from a struct's field definitions.
//! No manual GraphQL strings needed — the struct shape IS the query shape.
//!
//! # Usage
//!
//! ```ignore
//! use lineark_sdk::GraphQLFields;
//! use lineark_sdk::generated::types::Issue;
//!
//! #[derive(GraphQLFields, Deserialize)]
//! #[graphql(full_type = Issue)]
//! #[serde(rename_all = "camelCase")]
//! struct MyIssue {
//!     id: Option<String>,
//!     title: Option<String>,
//!     #[graphql(nested)]
//!     state: Option<StateRef>,
//! }
//! ```
//!
//! Generates: `"id title state { <StateRef::selection()> }"`
//!
//! With `full_type`, the macro also generates compile-time validation that
//! each field exists on the full type with a compatible type.

use heck::ToLowerCamelCase;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, Type};

/// Derive `GraphQLFields` for a struct.
///
/// Each field becomes a GraphQL selection entry:
/// - Plain fields → `camelCaseName`
/// - `#[graphql(nested)]` fields → `camelCaseName { <InnerType::selection()> }`
///
/// # Struct-level attributes
///
/// - `#[graphql(full_type = Path)]` — enables compile-time validation against
///   the specified generated type. Without this, `FullType = Self` and no
///   validation is performed (used by codegen for full types).
#[proc_macro_derive(GraphQLFields, attributes(graphql))]
pub fn derive_graphql_fields(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // Parse optional #[graphql(full_type = Path)] from struct-level attributes
    let full_type_path = parse_full_type(&input.attrs);

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(named) => &named.named,
            _ => {
                return syn::Error::new_spanned(
                    &input,
                    "GraphQLFields can only be derived on structs with named fields",
                )
                .to_compile_error()
                .into();
            }
        },
        _ => {
            return syn::Error::new_spanned(&input, "GraphQLFields can only be derived on structs")
                .to_compile_error()
                .into();
        }
    };

    let mut selection_parts = Vec::new();
    let mut validation_checks = Vec::new();

    for field in fields {
        let field_name = field.ident.as_ref().expect("named field should have ident");

        // Convert Rust snake_case field name to GraphQL camelCase.
        // Handle raw identifiers (r#type → type).
        let rust_name = field_name.to_string();
        let clean_name = rust_name.strip_prefix("r#").unwrap_or(&rust_name);
        let gql_name = clean_name.to_lower_camel_case();

        let is_nested = field.attrs.iter().any(|attr| {
            if !attr.path().is_ident("graphql") {
                return false;
            }
            let mut found = false;
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("nested") {
                    found = true;
                }
                Ok(())
            });
            found
        });

        if is_nested {
            // Extract the inner type (unwrap Option<T>, Vec<T>, Box<T>).
            let inner_ty = unwrap_type(&field.ty);
            selection_parts.push(quote! {
                {
                    let nested = <#inner_ty as GraphQLFields>::selection();
                    format!("{} {{ {} }}", #gql_name, nested)
                }
            });
            // For nested fields, only validate field existence (not type compatibility,
            // since the parent can't know the nested type's FullType).
            if full_type_path.is_some() {
                validation_checks.push(quote! {
                    { let _ = &__v.#field_name; }
                });
            }
        } else {
            selection_parts.push(quote! {
                #gql_name.to_string()
            });
            // For scalar fields, validate both field existence AND type compatibility.
            if full_type_path.is_some() {
                let field_ty = &field.ty;
                validation_checks.push(quote! {
                    {
                        fn __check<__F: ::lineark_sdk::FieldCompatible<__C>, __C>(_: &__F) {}
                        __check::<_, #field_ty>(&__v.#field_name);
                    }
                });
            }
        }
    }

    let full_type_assoc = if let Some(ref path) = full_type_path {
        quote! { type FullType = #path; }
    } else {
        quote! { type FullType = Self; }
    };

    let validation_block = if !validation_checks.is_empty() {
        let full_type = full_type_path.as_ref().unwrap();
        quote! {
            const _: () = {
                #[allow(unused)]
                fn __graphql_validate(__v: &#full_type) {
                    #(#validation_checks)*
                }
            };
        }
    } else {
        quote! {}
    };

    let expanded = quote! {
        impl GraphQLFields for #name {
            #full_type_assoc

            fn selection() -> String {
                let parts: Vec<String> = vec![
                    #(#selection_parts),*
                ];
                parts.join(" ")
            }
        }

        #validation_block
    };

    expanded.into()
}

/// Parse `#[graphql(full_type = Path)]` from struct-level attributes.
fn parse_full_type(attrs: &[syn::Attribute]) -> Option<syn::Path> {
    for attr in attrs {
        if !attr.path().is_ident("graphql") {
            continue;
        }
        let mut full_type = None;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("full_type") {
                let value = meta.value()?;
                full_type = Some(value.parse::<syn::Path>()?);
            }
            Ok(())
        });
        if full_type.is_some() {
            return full_type;
        }
    }
    None
}

/// Unwrap wrapper types to get the "leaf" type for nested selections.
/// `Option<Box<Foo>>` → `Foo`, `Vec<Bar>` → `Bar`, `Option<Vec<Baz>>` → `Baz`, etc.
fn unwrap_type(ty: &Type) -> &Type {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            let ident = segment.ident.to_string();
            if ident == "Option" || ident == "Vec" || ident == "Box" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        return unwrap_type(inner);
                    }
                }
            }
        }
    }
    ty
}
