use std::collections::BTreeSet;

use serde_json::Value;

pub const OUTPUT_SCHEMA_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/schemas/tracelens-output.schema.json"
));

const COMMAND_SECTIONS: &[(SchemaCommand, &str)] = &[
    (SchemaCommand::Validate, "validateOutput"),
    (SchemaCommand::Summary, "summaryOutput"),
    (SchemaCommand::ListTraces, "listTracesOutput"),
    (SchemaCommand::Tree, "treeOutput"),
    (SchemaCommand::Services, "servicesOutput"),
    (SchemaCommand::CriticalPath, "criticalPathOutput"),
    (SchemaCommand::Timeline, "timelineOutput"),
    (SchemaCommand::Detect, "detectOutput"),
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SchemaCommand {
    All,
    Validate,
    Summary,
    ListTraces,
    Tree,
    Services,
    CriticalPath,
    Timeline,
    Detect,
}

impl SchemaCommand {
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Validate => "validate",
            Self::Summary => "summary",
            Self::ListTraces => "list-traces",
            Self::Tree => "tree",
            Self::Services => "services",
            Self::CriticalPath => "critical-path",
            Self::Timeline => "timeline",
            Self::Detect => "detect",
        }
    }

    fn def_name(self) -> Option<&'static str> {
        COMMAND_SECTIONS
            .iter()
            .find_map(|(command, def_name)| (*command == self).then_some(*def_name))
    }
}

pub fn format_schema_json() -> String {
    let mut output = OUTPUT_SCHEMA_JSON.to_string();
    if !output.ends_with('\n') {
        output.push('\n');
    }
    output
}

pub fn format_schema_text(command: SchemaCommand) -> serde_json::Result<String> {
    let schema: Value = serde_json::from_str(OUTPUT_SCHEMA_JSON)?;
    let mut output = String::new();

    output.push_str("tracelens JSON Output Reference\n");
    output.push_str("schema_version: 0.1\n");
    output.push_str(&format!("command filter: {}\n\n", command.label()));
    output.push_str("How to use:\n");
    output.push_str(
        "- Run `tracelens schema --output json` for the full machine-readable JSON Schema.\n",
    );
    output.push_str("- Run `tracelens schema --command <name> --output text` for one command's field reference.\n");
    output.push_str("- JSON output always prints the full schema in this version; use `$defs.<command>Output` for a command branch.\n\n");

    write_common_section(&mut output);

    match command {
        SchemaCommand::All => {
            for (section_command, def_name) in COMMAND_SECTIONS {
                write_section(&mut output, &schema, section_command.label(), def_name);
            }
        }
        selected => {
            if let Some(def_name) = selected.def_name() {
                write_section(&mut output, &schema, selected.label(), def_name);
            }
        }
    }

    Ok(output)
}

fn write_common_section(output: &mut String) {
    output.push_str("[common]\n");
    output.push_str("- schema_version: Output contract version. Current value is \"0.1\" and may change before a stable 1.0 contract.\n");
    output.push_str("- command: CLI command that produced the JSON object; use it to select the matching schema branch.\n");
    output.push_str("- diagnostics: Input or trace quality diagnostics. Treat these as analysis caveats, not incidental logs.\n");
    output.push_str("- notes: Analysis notes and caveats. Notes are explanatory hints, not proof of root cause.\n\n");
}

fn write_section(output: &mut String, schema: &Value, label: &str, def_name: &str) {
    let Some(definition) = schema.pointer(&format!("/$defs/{def_name}")) else {
        return;
    };

    output.push_str(&format!("[{label}]\n"));
    if let Some(description) = definition.get("description").and_then(Value::as_str) {
        output.push_str(description);
        output.push('\n');
    }

    let mut fields = Vec::new();
    let mut seen_refs = BTreeSet::new();
    collect_field_descriptions(schema, definition, "", &mut fields, &mut seen_refs, 0);

    if fields.is_empty() {
        output.push_str("- No field descriptions are available for this schema section yet.\n");
    } else {
        for field in fields {
            output.push_str(&format!("- {}: {}\n", field.path, field.description));
        }
    }

    output.push('\n');
}

#[derive(Debug)]
struct FieldDescription {
    path: String,
    description: String,
}

fn collect_field_descriptions(
    root: &Value,
    node: &Value,
    path: &str,
    fields: &mut Vec<FieldDescription>,
    seen_refs: &mut BTreeSet<String>,
    depth: usize,
) {
    if depth > 24 {
        return;
    }

    if let Some(reference) = node.get("$ref").and_then(Value::as_str)
        && seen_refs.insert(reference.to_string())
    {
        if let Some(resolved) = resolve_ref(root, reference) {
            if let Some(description) = resolved.get("description").and_then(Value::as_str)
                && !path.is_empty()
                && !has_description_for_path(fields, path)
            {
                fields.push(FieldDescription {
                    path: path.to_string(),
                    description: description.to_string(),
                });
            }
            collect_field_descriptions(root, resolved, path, fields, seen_refs, depth + 1);
        }
        seen_refs.remove(reference);
    }

    if let Some(properties) = node.get("properties").and_then(Value::as_object) {
        for (name, value) in properties {
            let field_path = if path.is_empty() {
                name.to_string()
            } else {
                format!("{path}.{name}")
            };

            if let Some(description) = value.get("description").and_then(Value::as_str) {
                fields.push(FieldDescription {
                    path: field_path.clone(),
                    description: description.to_string(),
                });
            }

            collect_field_descriptions(root, value, &field_path, fields, seen_refs, depth + 1);
        }
    }

    if let Some(items) = node.get("items") {
        let item_path = if path.is_empty() {
            "[]".to_string()
        } else {
            format!("{path}[]")
        };
        collect_field_descriptions(root, items, &item_path, fields, seen_refs, depth + 1);
    }

    for keyword in ["oneOf", "allOf", "anyOf"] {
        if let Some(values) = node.get(keyword).and_then(Value::as_array) {
            for value in values {
                collect_field_descriptions(root, value, path, fields, seen_refs, depth + 1);
            }
        }
    }
}

fn resolve_ref<'a>(root: &'a Value, reference: &str) -> Option<&'a Value> {
    let path = reference.strip_prefix("#/")?;
    let mut current = root;

    for part in path.split('/') {
        let part = part.replace("~1", "/").replace("~0", "~");
        current = current.get(part)?;
    }

    Some(current)
}

fn has_description_for_path(fields: &[FieldDescription], path: &str) -> bool {
    fields.iter().any(|field| field.path == path)
}
