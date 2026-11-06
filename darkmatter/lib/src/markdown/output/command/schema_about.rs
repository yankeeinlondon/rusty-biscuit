//! The data value of `md schema about`: the whole compiled reference, whatever
//! the verbosity. It only reads compiled-in catalogs.

use serde::Serialize;

use crate::markdown::compose::context::context_variable_descriptors;
use crate::markdown::compose::expression::expression_function_catalog;
use crate::markdown::schemas::{
    coercion_rule_descriptors, inline_object_rule_descriptors, match_safe_constraint_descriptors, schema_constraint_descriptors,
    schema_shape_descriptors, schema_type_descriptors, trigger_grammar_descriptors, validation_behavior_descriptors,
};

/// A named string fact about an entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AboutDetail {
    /// What the fact is.
    pub name: String,
    /// Its value.
    pub value: String,
}

/// One item of the reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AboutEntry {
    /// The item's name or keyword.
    pub name: String,
    /// What it means.
    pub description: String,
    /// How it is written, when the catalog has an example.
    pub example: Option<String>,
    /// The constraint keywords it accepts.
    pub constraints: Vec<String>,
    /// Anything else the catalog records.
    pub details: Vec<AboutDetail>,
}

impl AboutEntry {
    fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            example: None,
            constraints: Vec::new(),
            details: Vec::new(),
        }
    }

    fn with_example(mut self, example: impl Into<String>) -> Self {
        self.example = Some(example.into());
        self
    }

    fn with_detail(mut self, name: &str, value: impl Into<String>) -> Self {
        self.details.push(AboutDetail { name: name.to_string(), value: value.into() });
        self
    }
}

/// One section of the reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AboutSection {
    /// A stable identifier.
    pub id: &'static str,
    /// The heading.
    pub title: &'static str,
    /// What the section covers.
    pub description: &'static str,
    /// Its items, in catalog order.
    pub entries: Vec<AboutEntry>,
}

/// `md schema about`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AboutData {
    /// The sections, in reading order.
    pub sections: Vec<AboutSection>,
}

impl AboutData {
    /// The compiled catalogs as one value.
    #[must_use]
    pub fn catalog() -> Self {
        Self {
            sections: vec![
                AboutSection {
                    id: "shapes",
                    title: "Schema Shapes",
                    description: "The forms a `$schema` value can use.",
                    entries: schema_shape_descriptors()
                        .iter()
                        .map(|shape| AboutEntry::new(shape.name, shape.description).with_example(shape.example).with_detail("form", shape.form))
                        .collect(),
                },
                AboutSection {
                    id: "types",
                    title: "Type System",
                    description: "The type keywords, and the constraints each accepts.",
                    entries: schema_type_descriptors()
                        .iter()
                        .map(|schema_type| {
                            let mut entry = AboutEntry::new(schema_type.keyword, schema_type.description);
                            entry.constraints = schema_type.accepted_constraints.iter().map(ToString::to_string).collect();
                            for alias in schema_type.aliases {
                                entry = entry.with_detail("alias", *alias);
                            }
                            for option in schema_type.type_options {
                                entry = entry.with_detail(&format!("option {}", option.keyword()), option.effect());
                            }
                            entry
                        })
                        .collect(),
                },
                AboutSection {
                    id: "constraints",
                    title: "Constraint Vocabulary",
                    description: "What can go inside `(...)` after a type.",
                    entries: schema_constraint_descriptors()
                        .iter()
                        .map(|constraint| {
                            AboutEntry::new(constraint.name, constraint.description)
                                .with_example(constraint.form)
                                .with_detail("keyword", constraint.keyword)
                                .with_detail("applies to", constraint.target_types)
                                .with_detail("arguments", constraint.argument_arity)
                        })
                        .collect(),
                },
                AboutSection {
                    id: "trigger_grammar",
                    title: "Trigger Schemas",
                    description: "How a trigger schema activates a payload. Its match language is pure and side-effect-free.",
                    entries: trigger_grammar_descriptors()
                        .iter()
                        .map(|item| AboutEntry::new(item.name, item.description).with_example(item.form))
                        .collect(),
                },
                AboutSection {
                    id: "match_safe_constraints",
                    title: "Match-Safe Constraints",
                    description: "The only constraints a trigger's match may use.",
                    entries: match_safe_constraint_descriptors()
                        .iter()
                        .map(|constraint| AboutEntry::new(constraint.keyword, constraint.description))
                        .collect(),
                },
                AboutSection {
                    id: "inline_objects",
                    title: "Nested Objects",
                    description: "Inline object literals validate nested frontmatter values.",
                    entries: inline_object_rule_descriptors()
                        .iter()
                        .map(|rule| AboutEntry::new(rule.name, rule.description).with_detail("rule", rule.rule))
                        .collect(),
                },
                AboutSection {
                    id: "coercion",
                    title: "Compose-time Coercion",
                    description: "How compose turns common string forms into real values.",
                    entries: coercion_rule_descriptors()
                        .iter()
                        .map(|rule| AboutEntry::new(rule.name, rule.description).with_detail("rule", rule.rule))
                        .collect(),
                },
                AboutSection {
                    id: "validation",
                    title: "Validation Notes",
                    description: "How Darkmatter applies schemas in real documents.",
                    entries: validation_behavior_descriptors()
                        .iter()
                        .map(|behavior| AboutEntry::new(behavior.name, behavior.description).with_detail("rule", behavior.rule))
                        .collect(),
                },
                AboutSection {
                    id: "context_variables",
                    title: "Context Variables",
                    description: "`ctx.*` variables available in `{{ }}` interpolation and expressions.",
                    entries: context_variable_descriptors()
                        .iter()
                        .map(|descriptor| {
                            AboutEntry::new(format!("ctx.{}", descriptor.name), descriptor.description)
                                .with_detail("type", descriptor.display_type.to_string())
                                .with_detail("category", descriptor.category)
                                .with_detail("required", descriptor.required.to_string())
                        })
                        .collect(),
                },
                AboutSection {
                    id: "expression_functions",
                    title: "Expression Functions",
                    description: "Read-side functions callable in `{{ }}` interpolation and `$( )` frontmatter values.",
                    entries: expression_function_catalog()
                        .descriptors
                        .iter()
                        .map(|descriptor| {
                            AboutEntry::new(&*descriptor.name, &*descriptor.description)
                                .with_example(descriptor.typed_signature())
                                .with_detail("category", &*descriptor.category)
                        })
                        .collect(),
                },
            ],
        }
    }
}
