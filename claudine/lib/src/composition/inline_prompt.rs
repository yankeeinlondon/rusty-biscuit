//! The file-aware header delivered ahead of every inline prompt.
//!
//! An inline agent works on the document itself, so it must be told which
//! file that is before it reads the author's request. The header names the
//! native absolute path in a code span and, when the document declares a
//! `$schema`, lists every top-level property with its type expression, whether
//! it is present at launch, and whether the run must leave it present.

use std::path::Path;

use darkmatter::markdown::schemas::simplified::serialize_property_atom;
use darkmatter::markdown::schemas::{
    Constraint, PropertyAtom, PropertyDef, SchemaShape, SimplifiedSchema,
};

use super::guardrails::document_path_span;
use super::types::LaunchSchema;

/// Build the header for `document_path`.
///
/// `schema` is the launch-resolved schema; a raw JSON Schema or a root union
/// contributes no property table because their per-property metadata is not
/// modelled. `instance` is the stabilized effective frontmatter the launch
/// verdict was judged against, so the launch column reports what the agent
/// will actually find in the file.
pub fn build_inline_prompt_header(
    document_path: &Path,
    schema: Option<&LaunchSchema>,
    instance: &serde_json::Value,
) -> String {
    let mut header = format!("**Document:** {}", document_path_span(document_path));
    let shape = schema.and_then(|schema| match schema.effective.simplified.as_ref() {
        Some(SimplifiedSchema::Single(shape)) => Some(shape),
        Some(SimplifiedSchema::Union(_)) | None => None,
    });
    if let Some(shape) = shape
        && !shape.properties.is_empty()
    {
        header.push_str("\n\n**Schema properties:**\n\n");
        header.push_str(&property_table(shape, instance));
    }
    header
}

fn property_table(shape: &SchemaShape, instance: &serde_json::Value) -> String {
    let mut table = String::from(
        "| Property | Type | At launch | At completion |\n|---|---|---|---|\n",
    );
    for (name, def) in &shape.properties {
        let present = instance
            .get(name)
            .is_some_and(|value| !value.is_null());
        let completion = if def_is_required_at_completion(def) {
            "required"
        } else {
            "optional"
        };
        table.push_str(&format!(
            "| `{name}` | `{}` | {} | {completion} |\n",
            type_expression(def),
            if present { "present" } else { "absent" },
        ));
    }
    table
}

fn type_expression(def: &PropertyDef) -> String {
    match def {
        PropertyDef::Single(atom) => serialize_property_atom(atom),
        PropertyDef::Union(atoms) => atoms
            .iter()
            .map(serialize_property_atom)
            .collect::<Vec<_>>()
            .join(" | "),
    }
}

/// Completion requires a property that any arm marks `required` or `eager`
/// (`eager` implies required-at-completion; see the Darkmatter phase table).
fn def_is_required_at_completion(def: &PropertyDef) -> bool {
    let atoms: Vec<&PropertyAtom> = match def {
        PropertyDef::Single(atom) => vec![atom],
        PropertyDef::Union(atoms) => atoms.iter().collect(),
    };
    atoms.iter().any(|atom| {
        atom.constraints
            .iter()
            .chain(atom.array_constraints.iter())
            .any(|constraint| matches!(constraint, Constraint::Required | Constraint::Eager))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use darkmatter::markdown::schemas::{DarkmatterSchemas, SchemaPhase};
    use darkmatter::markdown::Markdown;
    use serde_json::json;

    fn launch_schema(document: &str) -> LaunchSchema {
        let markdown: Markdown = document.to_string().into();
        let effective = DarkmatterSchemas::new()
            .effective_for(&markdown)
            .unwrap()
            .expect("document declares a schema");
        LaunchSchema {
            effective,
            phase: Some(SchemaPhase::Launch),
            report: None,
        }
    }

    #[test]
    fn header_names_the_native_path_in_a_code_span() {
        let header =
            build_inline_prompt_header(Path::new("/tmp/My Docs/voip.md"), None, &json!({}));
        assert_eq!(header, "**Document:** `/tmp/My Docs/voip.md`");
    }

    #[test]
    fn header_lists_every_property_with_launch_and_completion_status() {
        let schema = launch_schema(
            "---\n$schema:\n  prompt: string(required; eager)\n  last_updated: string(required)\n  researched_by: string(required)\n  products: object(required)\n  notes: string\nprompt: research\n---\n",
        );
        let header = build_inline_prompt_header(
            Path::new("/tmp/voip.md"),
            Some(&schema),
            &json!({"prompt": "research", "notes": null}),
        );
        assert!(header.starts_with("**Document:** `/tmp/voip.md`\n\n**Schema properties:**\n\n"));
        assert!(header.contains("| `prompt` | `string(required;eager)` | present | required |"));
        assert!(header.contains("| `last_updated` | `string(required)` | absent | required |"));
        assert!(header.contains("| `researched_by` | `string(required)` | absent | required |"));
        assert!(header.contains("| `products` | `object(required)` | absent | required |"));
        assert!(header.contains("| `notes` | `string` | absent | optional |"));
        assert_eq!(header.matches("\n| `").count(), 5, "one row per property:\n{header}");
    }

    #[test]
    fn eager_without_required_is_still_required_at_completion() {
        let schema = launch_schema("---\n$schema:\n  spec: string(eager)\nspec: x\n---\n");
        let header =
            build_inline_prompt_header(Path::new("/tmp/d.md"), Some(&schema), &json!({"spec": "x"}));
        assert!(header.contains("| `spec` | `string(eager)` | present | required |"));
    }

    #[test]
    fn raw_json_schema_contributes_no_table() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("raw.json"),
            r#"{"$schema":"https://json-schema.org/draft/2020-12/schema","type":"object","required":["a"],"properties":{"a":{"type":"string"}}}"#,
        )
        .unwrap();
        let document = dir.path().join("d.md");
        std::fs::write(&document, "---\n$schema: ./raw.json\n---\nbody\n").unwrap();
        let markdown = Markdown::try_from(document.as_path()).unwrap();
        let effective = DarkmatterSchemas::new()
            .effective_for(&markdown)
            .unwrap()
            .expect("document declares a schema");
        assert!(effective.simplified.is_none(), "fixture must be raw JSON Schema");
        let schema = LaunchSchema {
            effective,
            phase: Some(SchemaPhase::Launch),
            report: None,
        };
        let header = build_inline_prompt_header(&document, Some(&schema), &json!({}));
        assert_eq!(header, format!("**Document:** `{}`", document.display()));
    }
}
