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
    pub ty: GqlType,
    pub arguments: Vec<ArgumentDef>,
}

/// A simplified argument representation.
#[derive(Debug, Clone)]
pub struct ArgumentDef {
    pub name: String,
    pub ty: GqlType,
}

/// A simplified enum value.
#[derive(Debug, Clone)]
pub struct EnumValueDef {
    pub name: String,
}

/// A simplified enum type.
#[derive(Debug, Clone)]
pub struct EnumDef {
    pub name: String,
    pub values: Vec<EnumValueDef>,
}

/// A simplified object type.
#[derive(Debug, Clone)]
pub struct ObjectDef {
    pub name: String,
    pub fields: Vec<FieldDef>,
}

/// A simplified input type.
#[derive(Debug, Clone)]
pub struct InputDef {
    pub name: String,
    pub fields: Vec<FieldDef>,
}

/// A simplified scalar type.
#[derive(Debug, Clone)]
pub struct ScalarDef {
    pub name: String,
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
                type_kind_map.insert(name.clone(), TypeKind::Scalar);
                scalars.push(ScalarDef { name });
            }
            cst::Definition::EnumTypeDefinition(e) => {
                let name = extract_name(&e.name());
                type_kind_map.insert(name.clone(), TypeKind::Enum);
                enums.push(extract_enum(&e));
            }
            cst::Definition::ObjectTypeDefinition(o) => {
                let name = extract_name(&o.name());
                type_kind_map.insert(name.clone(), TypeKind::Object);
                let fields = extract_fields(&o.fields_definition());
                if name == "Query" {
                    query_fields = fields;
                } else if name == "Mutation" {
                    mutation_fields = fields;
                } else {
                    objects.push(ObjectDef { name, fields });
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
            let ty = extract_type(&f.ty());
            let arguments = extract_arguments(&f.arguments_definition());
            FieldDef {
                name,
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
            let ty = extract_type(&iv.ty());
            ArgumentDef { name, ty }
        })
        .collect()
}

fn extract_enum(e: &cst::EnumTypeDefinition) -> EnumDef {
    let name = extract_name(&e.name());
    let values = e
        .enum_values_definition()
        .map(|evd| {
            evd.enum_value_definitions()
                .map(|ev| {
                    let val_name = ev
                        .enum_value()
                        .map(|v| v.text().to_string())
                        .unwrap_or_default();
                    EnumValueDef { name: val_name }
                })
                .collect()
        })
        .unwrap_or_default();

    EnumDef { name, values }
}

fn extract_input(i: &cst::InputObjectTypeDefinition) -> InputDef {
    let name = extract_name(&i.name());
    let fields = i
        .input_fields_definition()
        .map(|ifd| {
            ifd.input_value_definitions()
                .map(|iv| {
                    let fname = extract_name(&iv.name());
                    let ty = extract_type(&iv.ty());
                    FieldDef {
                        name: fname,
                        ty,
                        arguments: Vec::new(),
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    InputDef { name, fields }
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
