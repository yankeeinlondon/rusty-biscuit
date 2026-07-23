//! Value/formatting helpers shared by the context reports.

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use darkmatter::effects::EffectSafety;
use darkmatter::markdown::compose::context::ContextValueType;
use darkmatter::markdown::schemas::SimplifiedType;

use crate::commands::context_render::inline_code_text;

pub(super) fn display_property(name: &str) -> String {
    format!("ctx.{name}")
}

pub(super) fn format_context_value_type(ty: &ContextValueType, term: &Terminal) -> String {
    Prose::new(context_value_type_markup(ty)).render(term)
}

/// Builds the Prose markup for a type label, coloring each type by its base
/// SimplifiedSchema type family. The label is the schema keyword form
/// (`string`, `string[]`, `number(integer)`, …) from the type's `Display`.
fn context_value_type_markup(ty: &ContextValueType) -> String {
    let color = match ty.base {
        SimplifiedType::Number | SimplifiedType::NumberLike => "green",
        SimplifiedType::Boolean | SimplifiedType::Boolish => "orange",
        SimplifiedType::Date | SimplifiedType::DateTime | SimplifiedType::Time => "violet",
        SimplifiedType::Object
        | SimplifiedType::Any
        | SimplifiedType::TypeDefinition
        | SimplifiedType::Schema => "cyan",
        SimplifiedType::String
        | SimplifiedType::Enum
        | SimplifiedType::File
        | SimplifiedType::Url
        | SimplifiedType::Email
        | SimplifiedType::Yaml
        | SimplifiedType::Json
        | SimplifiedType::Expression
        | SimplifiedType::Literal => "blue",
    };
    format!("<{color}>{ty}</{color}>")
}

pub(super) fn format_value(value: &serde_json::Value, term: &Terminal) -> String {
    match value {
        serde_json::Value::Null => Prose::new("<dim>null</dim>").render(term),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(|v| format_array_element(v, term)).collect();
            items.join(", ")
        }
        serde_json::Value::Object(_) => value.to_string(),
    }
}

/// Formats a single array element for the values report.
///
/// Scalar elements use the same plain rules as top-level values (e.g. a string
/// renders as `alpha`, not the JSON-quoted `"alpha"`). Nested arrays and objects
/// retain compact JSON serialization, since there is no flat plain form for them.
fn format_array_element(value: &serde_json::Value, term: &Terminal) -> String {
    match value {
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => value.to_string(),
        scalar => format_value(scalar, term),
    }
}

pub(super) fn format_safety(safety: EffectSafety, term: &Terminal) -> String {
    let text = match safety {
        EffectSafety::FilesystemWrite => "FilesystemWrite",
        EffectSafety::Network => "Network",
        EffectSafety::MarkdownMutation => "MarkdownMutation",
        EffectSafety::InMemoryState => "InMemoryState",
    };
    let colored = match safety {
        EffectSafety::FilesystemWrite => format!("<orange>{text}</orange>"),
        EffectSafety::Network => format!("<red>{text}</red>"),
        EffectSafety::MarkdownMutation => format!("<blue>{text}</blue>"),
        EffectSafety::InMemoryState => format!("<green>{text}</green>"),
    };
    Prose::new(colored).render(term)
}

/// Renders a descriptor's optional example for table cells.
pub(super) fn format_example(example: Option<&darkmatter::catalog::Example>, term: &Terminal) -> String {
    match example {
        Some(ex) => {
            let example = format!("{} → {}", ex.invocation, ex.result).replace('\t', r"\t");
            inline_code_text(&example, term)
        }
        None => String::new(),
    }
}

/// Compact example formatter for side-effect capabilities.
///
/// The signature is already shown in the `Capability` column, so the Example
/// column only displays the result arrow and value, keeping the four-column
/// report within the minimum supported terminal width.
pub(super) fn format_effect_example(
    example: Option<&darkmatter::catalog::Example>,
    term: &Terminal,
) -> String {
    match example {
        Some(ex) => inline_code_text(&format!("→ {}", ex.result), term),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_schema_types_render_as_structural_types() {
        let definition = ContextValueType {
            base: SimplifiedType::TypeDefinition,
            is_array: false,
            integer: false,
        };
        let schema = ContextValueType {
            base: SimplifiedType::Schema,
            is_array: true,
            integer: false,
        };

        assert_eq!(context_value_type_markup(&definition), "<cyan>type-definition</cyan>");
        assert_eq!(context_value_type_markup(&schema), "<cyan>schema[]</cyan>");
    }
}
