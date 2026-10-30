//! Frontmatter conversion, from the library ([`simplified_schema::coerce`]),
//! plus Darkmatter's pending classification of the templates validation
//! sets aside ([`nested_templates`]).

use serde_json::Value;

use simplified_schema::coerce::{ConversionPlan, NestedTemplates};
use simplified_schema::coercion::{BindOptions, PathSegment, bind_with_marked};
use simplified_schema::pending::holds_pending_syntax;

use crate::markdown::compose::expression::is_deferred_template;

/// The `{{ … }}` templates nested below the top-level keys of `instance`
/// that sit where the schema marks a value `deferred`: the core binds
/// `instance` once, consulting the template test (the
/// [`is_deferred_template`] classification) only at the shape's deferrable
/// nodes, and every path where it held is listed.
///
/// Does no work when the schema marks nothing `deferred` or the instance
/// holds no `{{`. Pass the authored instance (before literal tokens are
/// decoded): a whole-leaf literal token is stored data and is never listed.
/// `$schema` is a control key and is never listed either.
pub(crate) fn nested_templates(plan: &ConversionPlan, json_schema: &Value, instance: &Value) -> NestedTemplates {
    if !plan.marks_deferral(json_schema) || !holds_template_syntax(instance) {
        return NestedTemplates::default();
    }
    let found = std::cell::RefCell::new(Vec::<Vec<PathSegment>>::new());
    let marked = |path: &[PathSegment], value: &Value| {
        let template = matches!(value, Value::String(text) if holds_pending_syntax(text) && is_deferred_template(text));
        let nested = path.len() >= 2 && !matches!(path.first(), Some(PathSegment::Key(key)) if key == "$schema");
        if template && nested && !found.borrow().iter().any(|known| known == path) {
            found.borrow_mut().push(path.to_vec());
        }
        template && nested
    };
    // Only the paths matter; the verdict is the validator's.
    let _ = bind_with_marked(plan.shape(json_schema), instance, &BindOptions::default(), Some(&marked));
    NestedTemplates::from_template_paths(found.into_inner())
}

/// Whether some string in `value` could hold a template; the cheap
/// necessary condition [`nested_templates`] checks before binding.
fn holds_template_syntax(value: &Value) -> bool {
    match value {
        Value::String(text) => holds_pending_syntax(text),
        Value::Array(items) => items.iter().any(holds_template_syntax),
        Value::Object(map) => map.values().any(holds_template_syntax),
        _ => false,
    }
}
