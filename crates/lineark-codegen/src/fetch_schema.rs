//! Fetch Linear's GraphQL schema via introspection and convert to SDL.

use serde_json::Value;

const LINEAR_API_URL: &str = "https://api.linear.app/graphql";

const INTROSPECTION_QUERY: &str = r#"
query IntrospectionQuery {
  __schema {
    types {
      kind
      name
      description
      fields(includeDeprecated: true) {
        name
        description
        args {
          name
          description
          type { ...TypeRef }
          defaultValue
        }
        type { ...TypeRef }
        isDeprecated
        deprecationReason
      }
      inputFields {
        name
        description
        type { ...TypeRef }
        defaultValue
      }
      interfaces { ...TypeRef }
      enumValues(includeDeprecated: true) {
        name
        description
        isDeprecated
        deprecationReason
      }
      possibleTypes { ...TypeRef }
    }
    directives {
      name
      description
      locations
      args {
        name
        description
        type { ...TypeRef }
        defaultValue
      }
    }
  }
}

fragment TypeRef on __Type {
  kind
  name
  ofType {
    kind
    name
    ofType {
      kind
      name
      ofType {
        kind
        name
        ofType {
          kind
          name
          ofType {
            kind
            name
            ofType {
              kind
              name
            }
          }
        }
      }
    }
  }
}
"#;

/// Fetch the schema from Linear's public introspection endpoint and return SDL.
pub fn fetch_and_convert() -> Result<String, String> {
    let client = reqwest::blocking::Client::new();
    let body = serde_json::json!({ "query": INTROSPECTION_QUERY });

    eprintln!("Fetching schema from {}...", LINEAR_API_URL);
    let response = client
        .post(LINEAR_API_URL)
        .json(&body)
        .send()
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status()));
    }

    let json: Value = response
        .json()
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;

    let schema = &json["data"]["__schema"];
    if schema.is_null() {
        return Err("No __schema in response".into());
    }

    Ok(introspection_to_sdl(schema))
}

fn introspection_to_sdl(schema: &Value) -> String {
    let mut sdl = String::new();
    let types = schema["types"].as_array().unwrap_or(&Vec::new()).clone();

    // Sort by kind then name for stable output.
    let mut user_types: Vec<&Value> = types
        .iter()
        .filter(|t| {
            let name = t["name"].as_str().unwrap_or("");
            !name.starts_with("__")
        })
        .collect();
    user_types.sort_by_key(|t| {
        (
            kind_order(t["kind"].as_str().unwrap_or("")),
            t["name"].as_str().unwrap_or("").to_string(),
        )
    });

    for ty in &user_types {
        let kind = ty["kind"].as_str().unwrap_or("");
        let name = ty["name"].as_str().unwrap_or("");

        match kind {
            "SCALAR" => {
                // Skip built-in scalars.
                if matches!(name, "String" | "Int" | "Float" | "Boolean" | "ID") {
                    continue;
                }
                emit_description(&mut sdl, ty, "");
                sdl.push_str(&format!("scalar {}\n\n", name));
            }
            "ENUM" => {
                emit_description(&mut sdl, ty, "");
                sdl.push_str(&format!("enum {} {{\n", name));
                if let Some(values) = ty["enumValues"].as_array() {
                    for v in values {
                        emit_description(&mut sdl, v, "  ");
                        let vname = v["name"].as_str().unwrap_or("");
                        sdl.push_str(&format!("  {}", vname));
                        emit_deprecated(&mut sdl, v);
                        sdl.push('\n');
                    }
                }
                sdl.push_str("}\n\n");
            }
            "INPUT_OBJECT" => {
                emit_description(&mut sdl, ty, "");
                sdl.push_str(&format!("input {} {{\n", name));
                if let Some(fields) = ty["inputFields"].as_array() {
                    for f in fields {
                        emit_description(&mut sdl, f, "  ");
                        let fname = f["name"].as_str().unwrap_or("");
                        let ftype = render_type_ref(&f["type"]);
                        sdl.push_str(&format!("  {}: {}", fname, ftype));
                        emit_default(&mut sdl, f);
                        sdl.push('\n');
                    }
                }
                sdl.push_str("}\n\n");
            }
            "OBJECT" => {
                emit_description(&mut sdl, ty, "");
                sdl.push_str(&format!("type {}", name));
                emit_interfaces(&mut sdl, ty);
                sdl.push_str(" {\n");
                emit_fields(&mut sdl, ty);
                sdl.push_str("}\n\n");
            }
            "INTERFACE" => {
                emit_description(&mut sdl, ty, "");
                sdl.push_str(&format!("interface {}", name));
                emit_interfaces(&mut sdl, ty);
                sdl.push_str(" {\n");
                emit_fields(&mut sdl, ty);
                sdl.push_str("}\n\n");
            }
            "UNION" => {
                emit_description(&mut sdl, ty, "");
                let members: Vec<&str> = ty["possibleTypes"]
                    .as_array()
                    .map(|arr| arr.iter().filter_map(|t| t["name"].as_str()).collect())
                    .unwrap_or_default();
                sdl.push_str(&format!("union {} = {}\n\n", name, members.join(" | ")));
            }
            _ => {}
        }
    }

    sdl
}

fn kind_order(kind: &str) -> u8 {
    match kind {
        "SCALAR" => 0,
        "ENUM" => 1,
        "INPUT_OBJECT" => 2,
        "INTERFACE" => 3,
        "OBJECT" => 4,
        "UNION" => 5,
        _ => 6,
    }
}

fn emit_description(sdl: &mut String, val: &Value, indent: &str) {
    if let Some(desc) = val["description"].as_str() {
        if !desc.is_empty() {
            if desc.contains('\n') {
                sdl.push_str(&format!("{}\"\"\"{}\"\"\"\n", indent, desc));
            } else {
                sdl.push_str(&format!("{}\"{}\" ", indent, desc.replace('"', "\\\"")));
            }
        }
    }
}

fn emit_deprecated(sdl: &mut String, val: &Value) {
    if val["isDeprecated"].as_bool() == Some(true) {
        if let Some(reason) = val["deprecationReason"].as_str() {
            sdl.push_str(&format!(
                " @deprecated(reason: \"{}\")",
                reason.replace('"', "\\\"")
            ));
        } else {
            sdl.push_str(" @deprecated");
        }
    }
}

fn emit_default(sdl: &mut String, val: &Value) {
    if let Some(dv) = val["defaultValue"].as_str() {
        if !dv.is_empty() {
            sdl.push_str(&format!(" = {}", dv));
        }
    }
}

fn emit_interfaces(sdl: &mut String, ty: &Value) {
    if let Some(ifaces) = ty["interfaces"].as_array() {
        if !ifaces.is_empty() {
            let names: Vec<&str> = ifaces.iter().filter_map(|i| i["name"].as_str()).collect();
            if !names.is_empty() {
                sdl.push_str(&format!(" implements {}", names.join(" & ")));
            }
        }
    }
}

fn emit_fields(sdl: &mut String, ty: &Value) {
    if let Some(fields) = ty["fields"].as_array() {
        for f in fields {
            emit_description(sdl, f, "  ");
            let fname = f["name"].as_str().unwrap_or("");
            let ftype = render_type_ref(&f["type"]);
            // Args
            let args = f["args"].as_array();
            let has_args = args.is_some_and(|a| !a.is_empty());
            if has_args {
                sdl.push_str(&format!("  {}(", fname));
                let args = args.unwrap();
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        sdl.push_str(", ");
                    }
                    let aname = arg["name"].as_str().unwrap_or("");
                    let atype = render_type_ref(&arg["type"]);
                    sdl.push_str(&format!("{}: {}", aname, atype));
                    emit_default(sdl, arg);
                }
                sdl.push_str(&format!("): {}", ftype));
            } else {
                sdl.push_str(&format!("  {}: {}", fname, ftype));
            }
            emit_deprecated(sdl, f);
            sdl.push('\n');
        }
    }
}

fn render_type_ref(ty: &Value) -> String {
    match ty["kind"].as_str() {
        Some("NON_NULL") => format!("{}!", render_type_ref(&ty["ofType"])),
        Some("LIST") => format!("[{}]", render_type_ref(&ty["ofType"])),
        _ => ty["name"].as_str().unwrap_or("Unknown").to_string(),
    }
}
