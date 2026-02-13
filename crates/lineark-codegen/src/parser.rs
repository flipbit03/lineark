use apollo_parser::cst;
use apollo_parser::Parser;
use std::collections::HashMap;

/// What kind of GraphQL type a name refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeKind {
    Scalar,
    Enum,
    Object,
    InputObject,
    Interface,
    Union,
}

/// A simplified field representation extracted from the CST.
#[derive(Debug, Clone)]
pub struct FieldDef {
    pub name: String,
    pub description: Option<String>,
    pub ty: GqlType,
    pub arguments: Vec<ArgumentDef>,
}

/// A simplified argument representation.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ArgumentDef {
    pub name: String,
    pub description: Option<String>,
    pub ty: GqlType,
}

/// A simplified enum value.
#[derive(Debug, Clone)]
pub struct EnumValueDef {
    pub name: String,
    pub description: Option<String>,
}

/// A simplified enum type.
#[derive(Debug, Clone)]
pub struct EnumDef {
    pub name: String,
    pub description: Option<String>,
    pub values: Vec<EnumValueDef>,
}

/// A simplified object type.
#[derive(Debug, Clone)]
pub struct ObjectDef {
    pub name: String,
    pub description: Option<String>,
    pub fields: Vec<FieldDef>,
}

/// A simplified input type.
#[derive(Debug, Clone)]
pub struct InputDef {
    pub name: String,
    pub description: Option<String>,
    pub fields: Vec<FieldDef>,
}

/// A simplified scalar type.
#[derive(Debug, Clone)]
pub struct ScalarDef {
    pub name: String,
    pub description: Option<String>,
}

/// Represents a GraphQL type reference (NamedType, List, NonNull wrapping).
#[derive(Debug, Clone)]
pub enum GqlType {
    Named(String),
    List(Box<GqlType>),
    NonNull(Box<GqlType>),
}

impl GqlType {
    /// Get the base (innermost) named type.
    pub fn base_name(&self) -> &str {
        match self {
            GqlType::Named(name) => name,
            GqlType::List(inner) => inner.base_name(),
            GqlType::NonNull(inner) => inner.base_name(),
        }
    }
}

/// Parsed and categorized schema data.
pub struct ParsedSchema {
    pub scalars: Vec<ScalarDef>,
    pub enums: Vec<EnumDef>,
    pub objects: Vec<ObjectDef>,
    pub inputs: Vec<InputDef>,
    pub query_fields: Vec<FieldDef>,
    pub mutation_fields: Vec<FieldDef>,
    pub type_kind_map: HashMap<String, TypeKind>,
}

/// Built-in GraphQL scalar names.
const BUILTIN_SCALARS: &[&str] = &["String", "Int", "Float", "Boolean", "ID"];

pub fn parse(schema_text: &str) -> ParsedSchema {
    let parser = Parser::new(schema_text);
    let tree = parser.parse();

    // Report parse errors but continue (apollo-parser is error-resilient).
    for err in tree.errors() {
        eprintln!("Schema parse warning: {}", err.message());
    }

    let doc = tree.document();

    let mut scalars = Vec::new();
    let mut enums = Vec::new();
    let mut objects = Vec::new();
    let mut inputs = Vec::new();
    let mut query_fields = Vec::new();
    let mut mutation_fields = Vec::new();
    let mut type_kind_map: HashMap<String, TypeKind> = HashMap::new();

    // Register built-in scalars.
    for s in BUILTIN_SCALARS {
        type_kind_map.insert(s.to_string(), TypeKind::Scalar);
    }

    for def in doc.definitions() {
        match def {
            cst::Definition::ScalarTypeDefinition(s) => {
                let name = extract_name(&s.name());
                let description = extract_description(&s.description());
                type_kind_map.insert(name.clone(), TypeKind::Scalar);
                scalars.push(ScalarDef { name, description });
            }
            cst::Definition::EnumTypeDefinition(e) => {
                let name = extract_name(&e.name());
                type_kind_map.insert(name.clone(), TypeKind::Enum);
                enums.push(extract_enum(&e));
            }
            cst::Definition::ObjectTypeDefinition(o) => {
                let name = extract_name(&o.name());
                let description = extract_description(&o.description());
                type_kind_map.insert(name.clone(), TypeKind::Object);
                let fields = extract_fields(&o.fields_definition());
                if name == "Query" {
                    query_fields = fields;
                } else if name == "Mutation" {
                    mutation_fields = fields;
                } else {
                    objects.push(ObjectDef {
                        name,
                        description,
                        fields,
                    });
                }
            }
            cst::Definition::InputObjectTypeDefinition(i) => {
                let name = extract_name(&i.name());
                type_kind_map.insert(name.clone(), TypeKind::InputObject);
                inputs.push(extract_input(&i));
            }
            cst::Definition::InterfaceTypeDefinition(i) => {
                let name = extract_name(&i.name());
                type_kind_map.insert(name.clone(), TypeKind::Interface);
            }
            cst::Definition::UnionTypeDefinition(u) => {
                let name = extract_name(&u.name());
                type_kind_map.insert(name.clone(), TypeKind::Union);
            }
            _ => {}
        }
    }

    ParsedSchema {
        scalars,
        enums,
        objects,
        inputs,
        query_fields,
        mutation_fields,
        type_kind_map,
    }
}

fn extract_name(name: &Option<cst::Name>) -> String {
    name.as_ref()
        .map(|n| n.text().to_string())
        .unwrap_or_default()
}

fn extract_description(desc: &Option<cst::Description>) -> Option<String> {
    desc.as_ref()
        .and_then(|d| d.string_value())
        .map(String::from)
        .filter(|s| !s.is_empty())
}

fn extract_type(ty: &Option<cst::Type>) -> GqlType {
    match ty {
        None => GqlType::Named("String".to_string()),
        Some(t) => match t {
            cst::Type::NamedType(nt) => {
                let name = extract_name(&nt.name());
                GqlType::Named(name)
            }
            cst::Type::ListType(lt) => {
                let inner = extract_type(&lt.ty());
                GqlType::List(Box::new(inner))
            }
            cst::Type::NonNullType(nnt) => {
                if let Some(named) = nnt.named_type() {
                    let name = extract_name(&named.name());
                    GqlType::NonNull(Box::new(GqlType::Named(name)))
                } else if let Some(list) = nnt.list_type() {
                    let inner = extract_type(&list.ty());
                    GqlType::NonNull(Box::new(GqlType::List(Box::new(inner))))
                } else {
                    GqlType::NonNull(Box::new(GqlType::Named("String".to_string())))
                }
            }
        },
    }
}

fn extract_fields(fields_def: &Option<cst::FieldsDefinition>) -> Vec<FieldDef> {
    let Some(fd) = fields_def else {
        return Vec::new();
    };
    fd.field_definitions()
        .map(|f| {
            let name = extract_name(&f.name());
            let description = extract_description(&f.description());
            let ty = extract_type(&f.ty());
            let arguments = extract_arguments(&f.arguments_definition());
            FieldDef {
                name,
                description,
                ty,
                arguments,
            }
        })
        .collect()
}

fn extract_arguments(args_def: &Option<cst::ArgumentsDefinition>) -> Vec<ArgumentDef> {
    let Some(ad) = args_def else {
        return Vec::new();
    };
    ad.input_value_definitions()
        .map(|iv| {
            let name = extract_name(&iv.name());
            let description = extract_description(&iv.description());
            let ty = extract_type(&iv.ty());
            ArgumentDef {
                name,
                description,
                ty,
            }
        })
        .collect()
}

fn extract_enum(e: &cst::EnumTypeDefinition) -> EnumDef {
    let name = extract_name(&e.name());
    let description = extract_description(&e.description());
    let values = e
        .enum_values_definition()
        .map(|evd| {
            evd.enum_value_definitions()
                .map(|ev| {
                    let val_name = ev
                        .enum_value()
                        .map(|v| v.text().to_string())
                        .unwrap_or_default();
                    let val_desc = extract_description(&ev.description());
                    EnumValueDef {
                        name: val_name,
                        description: val_desc,
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    EnumDef {
        name,
        description,
        values,
    }
}

fn extract_input(i: &cst::InputObjectTypeDefinition) -> InputDef {
    let name = extract_name(&i.name());
    let description = extract_description(&i.description());
    let fields = i
        .input_fields_definition()
        .map(|ifd| {
            ifd.input_value_definitions()
                .map(|iv| {
                    let fname = extract_name(&iv.name());
                    let fdesc = extract_description(&iv.description());
                    let ty = extract_type(&iv.ty());
                    FieldDef {
                        name: fname,
                        description: fdesc,
                        ty,
                        arguments: Vec::new(),
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    InputDef {
        name,
        description,
        fields,
    }
}

/// Emit `/// ...` doc comment tokens from an optional description string.
/// Multi-line descriptions produce multiple `/// ` lines.
/// Sanitizes the description to avoid rustdoc warnings (unresolved links,
/// bare URLs).
pub fn doc_comment_tokens(description: &Option<String>) -> proc_macro2::TokenStream {
    let Some(desc) = description else {
        return proc_macro2::TokenStream::new();
    };
    let sanitized = sanitize_doc(desc);
    let lines: Vec<proc_macro2::TokenStream> = sanitized
        .lines()
        .map(|line| {
            let text = format!(" {}", line);
            quote::quote! { #[doc = #text] }
        })
        .collect();
    quote::quote! { #(#lines)* }
}

/// Sanitize a GraphQL description for use as a Rust doc comment.
///
/// - Escapes `[Foo]` bracket tags (e.g. `[DEPRECATED]`) that rustdoc would
///   interpret as intra-doc links, by replacing them with backtick-quoted text.
/// - Wraps bare `https://` URLs in angle brackets so rustdoc renders them as
///   clickable links instead of warning.
fn sanitize_doc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        if chars[i] == '[' {
            // Find matching `]`
            if let Some(close) = chars[i + 1..].iter().position(|&c| c == ']') {
                let close = i + 1 + close;
                let inner: String = chars[i + 1..close].iter().collect();
                // Only escape if it's NOT a markdown link (no `(` follows the `]`)
                let is_md_link = close + 1 < len && chars[close + 1] == '(';
                if !is_md_link && !inner.is_empty() {
                    out.push('`');
                    out.push_str(&inner);
                    out.push('`');
                    i = close + 1;
                    continue;
                }
            }
            out.push('[');
            i += 1;
        } else if i + 8 <= len && chars[i..i + 8].iter().collect::<String>() == "https://" {
            // Check if already inside angle brackets
            let already_bracketed = i > 0 && chars[i - 1] == '<';
            // Find end of URL (whitespace, closing paren, comma, or end of string)
            let url_end = chars[i..]
                .iter()
                .position(|&c| c.is_whitespace() || c == ')' || c == ',' || c == '>' || c == '\'')
                .map(|p| i + p)
                .unwrap_or(len);
            let url: String = chars[i..url_end].iter().collect();
            if already_bracketed {
                out.push_str(&url);
            } else {
                out.push('<');
                out.push_str(&url);
                out.push('>');
            }
            i = url_end;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }

    out
}

/// Rust keywords that need r# prefix when used as identifiers.
const RUST_KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn", "for",
    "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
    "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where",
    "while", "async", "await", "dyn", "abstract", "become", "box", "do", "final", "macro",
    "override", "priv", "typeof", "unsized", "virtual", "yield", "try",
];

/// Make a name safe for use as a Rust identifier.
pub fn safe_ident(name: &str) -> String {
    if RUST_KEYWORDS.contains(&name) {
        format!("r#{}", name)
    } else {
        name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINI_SCHEMA: &str = r#"
        "Represents a date and time." scalar DateTime
        scalar JSON

        "The status of an issue." enum IssueStatus {
            BACKLOG
            TODO
            "Work in progress." IN_PROGRESS
            DONE
        }

        "A user account." type User {
            "The unique identifier." id: ID!
            "The user's display name." name: String!
            email: String
            active: Boolean
            createdAt: DateTime
        }

        type Team {
            id: ID!
            key: String!
            name: String!
            description: String
        }

        "Filter for users." input UserFilter {
            "Filter by name." name: String
            active: Boolean
        }

        type Query {
            viewer: User!
            users(first: Int, after: String): UserConnection!
            team(id: String!): Team!
        }

        type UserConnection {
            nodes: [User!]!
            pageInfo: PageInfo!
        }

        type PageInfo {
            hasNextPage: Boolean!
            endCursor: String
        }
    "#;

    #[test]
    fn parse_scalars() {
        let schema = parse(MINI_SCHEMA);
        let names: Vec<&str> = schema.scalars.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"DateTime"));
        assert!(names.contains(&"JSON"));
    }

    #[test]
    fn parse_enums() {
        let schema = parse(MINI_SCHEMA);
        assert_eq!(schema.enums.len(), 1);
        let e = &schema.enums[0];
        assert_eq!(e.name, "IssueStatus");
        let values: Vec<&str> = e.values.iter().map(|v| v.name.as_str()).collect();
        assert_eq!(values, vec!["BACKLOG", "TODO", "IN_PROGRESS", "DONE"]);
    }

    #[test]
    fn parse_objects() {
        let schema = parse(MINI_SCHEMA);
        let obj_names: Vec<&str> = schema.objects.iter().map(|o| o.name.as_str()).collect();
        assert!(obj_names.contains(&"User"));
        assert!(obj_names.contains(&"Team"));
        assert!(obj_names.contains(&"UserConnection"));
        assert!(obj_names.contains(&"PageInfo"));
        // Query should NOT appear as a regular object
        assert!(!obj_names.contains(&"Query"));
    }

    #[test]
    fn parse_query_fields() {
        let schema = parse(MINI_SCHEMA);
        let query_names: Vec<&str> = schema
            .query_fields
            .iter()
            .map(|f| f.name.as_str())
            .collect();
        assert!(query_names.contains(&"viewer"));
        assert!(query_names.contains(&"users"));
        assert!(query_names.contains(&"team"));
    }

    #[test]
    fn parse_inputs() {
        let schema = parse(MINI_SCHEMA);
        assert_eq!(schema.inputs.len(), 1);
        let input = &schema.inputs[0];
        assert_eq!(input.name, "UserFilter");
        assert_eq!(input.fields.len(), 2);
    }

    #[test]
    fn parse_type_kind_map() {
        let schema = parse(MINI_SCHEMA);
        assert_eq!(
            schema.type_kind_map.get("DateTime"),
            Some(&TypeKind::Scalar)
        );
        assert_eq!(
            schema.type_kind_map.get("IssueStatus"),
            Some(&TypeKind::Enum)
        );
        assert_eq!(schema.type_kind_map.get("User"), Some(&TypeKind::Object));
        assert_eq!(
            schema.type_kind_map.get("UserFilter"),
            Some(&TypeKind::InputObject)
        );
        // Built-in scalars
        assert_eq!(schema.type_kind_map.get("String"), Some(&TypeKind::Scalar));
        assert_eq!(schema.type_kind_map.get("Int"), Some(&TypeKind::Scalar));
        assert_eq!(schema.type_kind_map.get("Boolean"), Some(&TypeKind::Scalar));
    }

    #[test]
    fn parse_field_types() {
        let schema = parse(MINI_SCHEMA);
        let user = schema.objects.iter().find(|o| o.name == "User").unwrap();

        // id: ID! -> NonNull(Named("ID"))
        let id_field = user.fields.iter().find(|f| f.name == "id").unwrap();
        assert!(matches!(
            &id_field.ty,
            GqlType::NonNull(inner) if matches!(inner.as_ref(), GqlType::Named(n) if n == "ID")
        ));

        // email: String -> Named("String")
        let email_field = user.fields.iter().find(|f| f.name == "email").unwrap();
        assert!(matches!(&email_field.ty, GqlType::Named(n) if n == "String"));

        // createdAt: DateTime -> Named("DateTime")
        let created_field = user.fields.iter().find(|f| f.name == "createdAt").unwrap();
        assert_eq!(created_field.ty.base_name(), "DateTime");
    }

    #[test]
    fn parse_query_arguments() {
        let schema = parse(MINI_SCHEMA);
        let users_query = schema
            .query_fields
            .iter()
            .find(|f| f.name == "users")
            .unwrap();
        assert_eq!(users_query.arguments.len(), 2);

        let first_arg = users_query
            .arguments
            .iter()
            .find(|a| a.name == "first")
            .unwrap();
        assert_eq!(first_arg.ty.base_name(), "Int");

        let after_arg = users_query
            .arguments
            .iter()
            .find(|a| a.name == "after")
            .unwrap();
        assert_eq!(after_arg.ty.base_name(), "String");
    }

    #[test]
    fn safe_ident_keywords() {
        assert_eq!(safe_ident("type"), "r#type");
        assert_eq!(safe_ident("match"), "r#match");
        assert_eq!(safe_ident("async"), "r#async");
        assert_eq!(safe_ident("self"), "r#self");
    }

    #[test]
    fn safe_ident_non_keywords() {
        assert_eq!(safe_ident("name"), "name");
        assert_eq!(safe_ident("id"), "id");
        assert_eq!(safe_ident("user_name"), "user_name");
    }

    #[test]
    fn gql_type_base_name() {
        let named = GqlType::Named("User".to_string());
        assert_eq!(named.base_name(), "User");

        let non_null = GqlType::NonNull(Box::new(GqlType::Named("String".to_string())));
        assert_eq!(non_null.base_name(), "String");

        let list = GqlType::List(Box::new(GqlType::NonNull(Box::new(GqlType::Named(
            "Int".to_string(),
        )))));
        assert_eq!(list.base_name(), "Int");
    }

    #[test]
    fn parse_descriptions() {
        let schema = parse(MINI_SCHEMA);

        // Scalar description
        let dt = schema
            .scalars
            .iter()
            .find(|s| s.name == "DateTime")
            .unwrap();
        assert_eq!(
            dt.description.as_deref(),
            Some("Represents a date and time.")
        );

        let json = schema.scalars.iter().find(|s| s.name == "JSON").unwrap();
        assert!(json.description.is_none());

        // Enum description
        let e = &schema.enums[0];
        assert_eq!(e.description.as_deref(), Some("The status of an issue."));

        // Enum value description
        let in_progress = e.values.iter().find(|v| v.name == "IN_PROGRESS").unwrap();
        assert_eq!(
            in_progress.description.as_deref(),
            Some("Work in progress.")
        );
        let backlog = e.values.iter().find(|v| v.name == "BACKLOG").unwrap();
        assert!(backlog.description.is_none());

        // Object description
        let user = schema.objects.iter().find(|o| o.name == "User").unwrap();
        assert_eq!(user.description.as_deref(), Some("A user account."));

        // Field description
        let id_field = user.fields.iter().find(|f| f.name == "id").unwrap();
        assert_eq!(
            id_field.description.as_deref(),
            Some("The unique identifier.")
        );
        let email_field = user.fields.iter().find(|f| f.name == "email").unwrap();
        assert!(email_field.description.is_none());

        // Input description
        let input = &schema.inputs[0];
        assert_eq!(input.description.as_deref(), Some("Filter for users."));

        // Input field description
        let name_field = input.fields.iter().find(|f| f.name == "name").unwrap();
        assert_eq!(name_field.description.as_deref(), Some("Filter by name."));
    }

    #[test]
    fn parse_empty_schema() {
        let schema = parse("");
        assert!(schema.scalars.is_empty());
        assert!(schema.enums.is_empty());
        assert!(schema.objects.is_empty());
        assert!(schema.inputs.is_empty());
        assert!(schema.query_fields.is_empty());
        assert!(schema.mutation_fields.is_empty());
        // Built-in scalars should still be in the map
        assert_eq!(schema.type_kind_map.len(), 5);
    }

    #[test]
    fn parse_interface_and_union() {
        let schema_text = r#"
            interface Node {
                id: ID!
            }
            union SearchResult = User | Team
            type User {
                id: ID!
            }
            type Team {
                id: ID!
            }
        "#;
        let schema = parse(schema_text);
        assert_eq!(schema.type_kind_map.get("Node"), Some(&TypeKind::Interface));
        assert_eq!(
            schema.type_kind_map.get("SearchResult"),
            Some(&TypeKind::Union)
        );
    }

    #[test]
    fn parse_mutation_fields() {
        let schema_text = r#"
            type User {
                id: ID!
                name: String
            }
            type Mutation {
                createUser(name: String!): User!
                deleteUser(id: ID!): Boolean!
            }
        "#;
        let schema = parse(schema_text);
        assert_eq!(schema.mutation_fields.len(), 2);
        let names: Vec<&str> = schema
            .mutation_fields
            .iter()
            .map(|f| f.name.as_str())
            .collect();
        assert!(names.contains(&"createUser"));
        assert!(names.contains(&"deleteUser"));
    }
}
