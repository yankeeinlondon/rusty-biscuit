use std::collections::HashMap;

use darkmatter::markdown::compose::expression::{
    EvaluationLookup, Expr, ExpressionFinder, ParseMode, Parser, evaluate, scalar_string,
};
use serde_json::Value;

use crate::harness::model::{ValidationKind, ValidationRule};

pub(crate) fn default_message(kind: &ValidationKind, vars: &HashMap<&str, String>) -> String {
    let template = match kind {
        ValidationKind::FileExists { .. } => "the file {{file}} exists",
        ValidationKind::DirExists { .. } => "the directory {{dir}} exists",
        ValidationKind::JsonFileExists { .. } => "{{file}} is a valid JSON file",
        ValidationKind::YamlFileExists { .. } => "{{file}} is a valid YAML file",
        ValidationKind::TomlFileExists { .. } => "{{file}} is a valid TOML file",
        ValidationKind::HasWritePermission { .. } => "write permission for {{file}}",
        ValidationKind::ShellCommand { .. } => "shell command: {{command}}",
        ValidationKind::NoDirtySourceCode { .. } => "no dirty source code in {{dir}}",
        ValidationKind::HasDirtySourceCode { .. } => "dirty source code found in {{dir}}",
        ValidationKind::FileChanged { .. } => "the file {{file}} was modified",
        ValidationKind::FileUnchanged { .. } => "the file {{file}} was not modified",
        ValidationKind::FrontmatterPropChanged { .. } => {
            "frontmatter property \"{{prop}}\" was modified"
        }
        ValidationKind::FrontmatterPropUnchanged { .. } => {
            "frontmatter property \"{{prop}}\" was not modified"
        }
        ValidationKind::FrontmatterPropEquals { .. } => {
            "frontmatter properties match expected values"
        }
        ValidationKind::ResponseLengthAtLeast { .. } => {
            "response is at least {{length}} characters (actual: {{response_length}})"
        }
        ValidationKind::ResponseLengthAtMost { .. } => {
            "response is at most {{length}} characters (actual: {{response_length}})"
        }
        ValidationKind::ResponseIncludes { .. } => "response includes \"{{expected}}\"",
        ValidationKind::ResponseMissing { .. } => "response does not include \"{{expected}}\"",
    };
    render_template(template, vars)
}

pub(crate) fn build_vars(kind: &ValidationKind) -> HashMap<&str, String> {
    use crate::harness::report::prose_escape;

    let mut vars: HashMap<&str, String> = HashMap::new();

    match kind {
        ValidationKind::FileExists { file }
        | ValidationKind::JsonFileExists { file, .. }
        | ValidationKind::YamlFileExists { file, .. }
        | ValidationKind::TomlFileExists { file }
        | ValidationKind::HasWritePermission { file }
        | ValidationKind::FileChanged { file }
        | ValidationKind::FileUnchanged { file } => {
            vars.insert("file", prose_escape(&file.display().to_string()));
        }
        ValidationKind::DirExists { dir }
        | ValidationKind::NoDirtySourceCode { root: dir }
        | ValidationKind::HasDirtySourceCode { root: dir } => {
            vars.insert("dir", prose_escape(&dir.display().to_string()));
        }
        ValidationKind::ShellCommand { command, .. } => {
            vars.insert("command", prose_escape(&command.raw));
        }
        ValidationKind::FrontmatterPropChanged { prop }
        | ValidationKind::FrontmatterPropUnchanged { prop } => {
            vars.insert("prop", prose_escape(prop));
        }
        ValidationKind::FrontmatterPropEquals { .. } => {}
        ValidationKind::ResponseLengthAtLeast { length }
        | ValidationKind::ResponseLengthAtMost { length } => {
            vars.insert("length", length.to_string());
        }
        ValidationKind::ResponseIncludes { needle }
        | ValidationKind::ResponseMissing { needle } => {
            vars.insert("expected", prose_escape(needle));
        }
    }

    vars
}

/// Build the prose-ready markup string for a check result.
///
/// Returns a markup body suitable for `Status::from_prose(...)`.
/// Does not render to a terminal. The `Status` component provides the
/// pass/fail indicator via `StatusState`, so no inline glyph is emitted.
pub(crate) fn build_check_markup(rule: &ValidationRule) -> String {
    let vars = build_vars(&rule.kind);

    if let Some(ref tmpl) = rule.message_template {
        render_template(tmpl, &vars)
    } else {
        default_message(&rule.kind, &vars)
    }
}

/// Render `{{...}}` placeholders in a validation message template.
///
/// Each `{{...}}` token is parsed as a Darkmatter interpolation expression
/// against the validation variable map, so simple `{{key}}` lookups still
/// work while richer authors can reach for fallbacks (`{{file || "n/a"}}`),
/// ternaries, comparisons, and helper functions like `length(...)`.
///
/// Bare-variable references that are not in `vars` and tokens that fail
/// to parse are preserved unchanged so validation messages remain
/// best-effort and cannot mask the underlying validation result.
pub(crate) fn render_template(template: &str, vars: &HashMap<&str, String>) -> String {
    let locations = ExpressionFinder::find_all_plain(template);
    if locations.is_empty() {
        return template.to_string();
    }

    let lookup = ValidationVarsLookup { vars };
    let mut output = String::with_capacity(template.len());
    let mut cursor = 0;
    for location in &locations {
        output.push_str(&template[cursor..location.start]);
        let original = &template[location.start..location.end];
        match render_validation_expression(&location.expression, &lookup, vars) {
            Some(rendered) => output.push_str(&rendered),
            None => output.push_str(original),
        }
        cursor = location.end;
    }
    output.push_str(&template[cursor..]);
    output
}

/// Lookup adapter exposing the validation variable map to Darkmatter's
/// expression evaluator.
struct ValidationVarsLookup<'a> {
    vars: &'a HashMap<&'a str, String>,
}

impl<'a> EvaluationLookup for ValidationVarsLookup<'a> {
    fn get(&self, path: &str) -> Option<Value> {
        self.vars
            .get(path)
            .map(|value| Value::String(value.clone()))
    }
}

/// Evaluate a single `{{...}}` body, or return `None` to preserve the
/// original token.
fn render_validation_expression(
    expression: &str,
    lookup: &ValidationVarsLookup<'_>,
    vars: &HashMap<&str, String>,
) -> Option<String> {
    let trimmed = expression.trim();
    if trimmed.is_empty() {
        return None;
    }

    let parsed = Parser::with_mode(trimmed, ParseMode::Interpolation)
        .ok()?
        .parse()
        .ok()?;

    // Preserve bare-variable references not present in the validation map
    // (for example `{{response_length}}`, which is intentionally omitted
    // because it depends on outcome not available at template build time).
    if let Expr::Variable(path) = &parsed
        && !vars.contains_key(path.as_str())
    {
        return None;
    }

    evaluate(&parsed, lookup)
        .ok()
        .map(|value| scalar_string(&value))
}
