/// One overload of a generated lifecycle verb (a Darkmatter side effect or
/// expression function), read from its typed catalog entry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Overload {
    descriptor: &'static ExpressionFunctionDescriptor,
}

impl Overload {
    #[allow(missing_docs)]
    pub fn parameters(&self) -> &'static [FunctionParameterDescriptor] {
        &self.descriptor.parameters
    }

    /// The parameter names, in positional order.
    pub fn names(&self) -> impl Iterator<Item = &'static str> {
        self.descriptor.parameters.iter().map(|parameter| &*parameter.name)
    }

    /// The catalog display key, e.g. `round(x, [fallback])` or `and(...)`.
    pub fn display(&self) -> &'static str {
        &self.descriptor.signature
    }

    /// Whether a rest parameter takes any number of trailing arguments.
    pub fn is_variadic(&self) -> bool {
        self.descriptor.parameters.iter().any(|parameter| parameter.rest)
    }

    /// The number of parameters that are neither optional nor rest.
    pub fn min_count(&self) -> usize {
        self.descriptor
            .parameters
            .iter()
            .filter(|parameter| !parameter.optional && !parameter.rest)
            .count()
    }

    /// `None` when a rest parameter takes any number of arguments.
    pub fn max_count(&self) -> Option<usize> {
        (!self.is_variadic()).then_some(self.descriptor.parameters.len())
    }

    /// Whether `supplied` positional arguments fit this overload; the same
    /// count rule the runtime binder applies before any argument binds.
    pub fn accepts_count(&self, supplied: usize) -> bool {
        supplied >= self.min_count() && self.max_count().is_none_or(|max| supplied <= max)
    }

    /// The required parameters absent from `keys`.
    fn missing(&self, keys: &[&str]) -> Vec<&'static str> {
        self.descriptor
            .parameters
            .iter()
            .filter(|parameter| !parameter.optional && !parameter.rest && !keys.contains(&&*parameter.name))
            .map(|parameter| &*parameter.name)
            .collect()
    }
}

/// Every overload of one generated lifecycle verb, in catalog order.
///
/// Arguments are checked per overload, never against a merge of them: a
/// positional call must fit one overload's count, and a long form must name
/// exactly one overload's parameters ([`VerbSignature::select_named`]). This
/// is the parse-time half of the rule the event-time binder applies, so a
/// call that binds no overload is rejected before the event fires.
#[derive(Debug, Clone, PartialEq)]
pub struct VerbSignature {
    verb: &'static str,
    overloads: Vec<Overload>,
}

/// Why a long form's keys select no overload ([`VerbSignature::select_named`]).
#[derive(Debug, Clone, PartialEq)]
pub enum LongFormMismatch {
    /// Every key names a parameter of `overload`, the closest overload that
    /// takes them all, which still needs `missing`.
    Missing {
        overload: Overload,
        missing: Vec<&'static str>,
    },
    /// Keys that no overload names, sorted.
    Unknown(Vec<String>),
    /// Every key is some overload's parameter, but no one overload takes
    /// them all together (`pr_list: { query, count }`).
    Unrelated,
}

impl VerbSignature {
    #[allow(missing_docs)]
    pub fn verb(&self) -> &'static str {
        self.verb
    }

    #[allow(missing_docs)]
    pub fn overloads(&self) -> &[Overload] {
        &self.overloads
    }

    /// Whether any overload takes a rest parameter. Such a verb is
    /// positional-only: a long form cannot name a variable argument list.
    pub fn is_variadic(&self) -> bool {
        self.overloads.iter().any(Overload::is_variadic)
    }

    /// The fewest arguments any overload takes.
    pub fn min_count(&self) -> usize {
        self.overloads.iter().map(Overload::min_count).min().unwrap_or(0)
    }

    /// Whether `supplied` positional arguments fit at least one overload.
    pub fn accepts_count(&self, supplied: usize) -> bool {
        self.overloads.iter().any(|overload| overload.accepts_count(supplied))
    }

    /// The overloads' display keys joined with ` | `.
    pub fn display_overloads(&self) -> String {
        self.overloads.iter().map(Overload::display).collect::<Vec<_>>().join(" | ")
    }

    /// The overload a long form's `keys` select: the first non-variadic
    /// overload, in catalog order, whose parameters include every key and
    /// whose required parameters are all among the keys. Its parameter order
    /// is the positional order the action dispatches in.
    ///
    /// ## Errors
    ///
    /// The [`LongFormMismatch`] naming the closest problem: missing required
    /// parameters of the overload that takes every key with the fewest
    /// missing (the first on a tie), else the keys no overload names, else
    /// [`LongFormMismatch::Unrelated`].
    pub fn select_named(&self, keys: &[&str]) -> Result<Overload, LongFormMismatch> {
        let covering: Vec<&Overload> = self
            .overloads
            .iter()
            .filter(|overload| !overload.is_variadic())
            .filter(|overload| keys.iter().all(|key| overload.names().any(|name| name == *key)))
            .collect();
        if let Some(selected) = covering.iter().find(|overload| overload.missing(keys).is_empty()) {
            return Ok(**selected);
        }
        if let Some(closest) = covering.iter().min_by_key(|overload| overload.missing(keys).len()) {
            return Err(LongFormMismatch::Missing {
                overload: **closest,
                missing: closest.missing(keys),
            });
        }
        let mut unknown: Vec<String> = keys
            .iter()
            .filter(|key| !self.overloads.iter().any(|overload| overload.names().any(|name| name == **key)))
            .map(|key| (*key).to_string())
            .collect();
        if unknown.is_empty() {
            return Err(LongFormMismatch::Unrelated);
        }
        unknown.sort_unstable();
        Err(LongFormMismatch::Unknown(unknown))
    }

    /// The catalog overloads named `verb`, in catalog order.
    fn from_catalog(verb: &'static str, descriptors: &'static [ExpressionFunctionDescriptor]) -> Self {
        Self {
            verb,
            overloads: descriptors
                .iter()
                .filter(|descriptor| &*descriptor.name == verb)
                .map(|descriptor| Overload { descriptor })
                .collect(),
        }
    }
}

/// The overloads of a known Darkmatter side-effect verb.
///
/// [`EFFECT_DESCRIPTORS`] decides which verbs exist; the overloads come from
/// the typed side-effect catalog ([`effect_function_descriptors`]), which a
/// coverage test holds in step with it.
pub fn side_effect_signature(verb: &str) -> Option<&'static VerbSignature> {
    static SIGNATURES: LazyLock<HashMap<&'static str, VerbSignature>> = LazyLock::new(|| {
        EFFECT_DESCRIPTORS
            .iter()
            .filter_map(|descriptor| parsed_verb_of(descriptor.signature))
            .map(|verb| (verb, VerbSignature::from_catalog(verb, effect_function_descriptors())))
            .collect()
    });
    SIGNATURES.get(verb)
}

/// Returns `true` when `verb` names a known Darkmatter side-effect.
///
/// Positional actions whose verb is not a communication, shell, or
/// lifecycle-control keyword are first parsed as
/// [`ExpressionFunctionAction`]. At execution time the stack executor uses
/// this predicate to route a known side-effect verb (e.g.
/// `ensure_file('@x')`) to the side-effect engine rather than the read-only
/// expression engine.
pub fn is_known_side_effect(verb: &str) -> bool {
    side_effect_signature(verb).is_some()
}

/// Returns `true` when `verb` names any known lifecycle action verb.
///
/// This is the parse-time validator required by decision #6 in the
/// positional-and-key-value plan. It unions communication channels, the
/// `shell` verb, lifecycle control verbs, Darkmatter side-effect verbs, and
/// Darkmatter expression-function verbs.
pub fn is_known_lifecycle_verb(verb: &str) -> bool {
    if CommunicationChannel::from_verb(verb).is_some() || verb == "shell" {
        return true;
    }
    if is_lifecycle_control_verb(verb) {
        return true;
    }
    if is_known_side_effect(verb) {
        return true;
    }
    is_known_expression_function_verb(verb)
}

/// The flow-control directive verbs, one per [`LifecycleControlAction`]
/// variant. The parser, the bundle ranking, and the source map all read this
/// one list.
const LIFECYCLE_CONTROL_VERBS: &[&str] = &[
    "stop", "skip", "error", "proxy", "retry", "resume", "defer", "break", "prep",
];

/// Returns `true` when `verb` names a lifecycle control action.
pub fn is_lifecycle_control_verb(verb: &str) -> bool {
    LIFECYCLE_CONTROL_VERBS.contains(&verb)
}

/// Returns `true` when `verb` names a known Darkmatter expression function.
fn is_known_expression_function_verb(verb: &str) -> bool {
    expression_function_descriptors()
        .iter()
        .any(|d| &*d.name == verb && d.accepts_value_arguments())
}

/// Extract the verb name from a raw descriptor signature string.
fn parsed_verb_of(signature: &str) -> Option<&str> {
    let signature = signature.trim();
    let open = signature.find('(')?;
    if open == 0 {
        return None;
    }
    Some(&signature[..open])
}

/// The overloads of a known Darkmatter expression-function verb (exact
/// canonical name; aliases are not lifecycle verbs).
pub fn expression_function_signature(verb: &str) -> Option<&'static VerbSignature> {
    static SIGNATURES: LazyLock<HashMap<&'static str, VerbSignature>> = LazyLock::new(|| {
        let descriptors = expression_function_descriptors();
        descriptors
            .iter()
            .filter(|descriptor| descriptor.accepts_value_arguments())
            .map(|descriptor| (&*descriptor.name, VerbSignature::from_catalog(&descriptor.name, descriptors)))
            .collect()
    });
    SIGNATURES.get(verb)
}

/// Returns every lifecycle action verb known to the parser.
///
/// Used for did-you-mean suggestions when a positional or key/value verb is
/// not recognized.
pub fn all_lifecycle_verbs() -> Vec<&'static str> {
    let mut verbs: Vec<&'static str> = Vec::new();
    for channel in [
        CommunicationChannel::Say,
        CommunicationChannel::Speak,
        CommunicationChannel::Effect,
        CommunicationChannel::Message,
        CommunicationChannel::Notify,
        CommunicationChannel::Stderr,
        CommunicationChannel::Info,
        CommunicationChannel::Warn,
        CommunicationChannel::Success,
        CommunicationChannel::Stdout,
    ] {
        verbs.push(channel.verb());
    }
    verbs.push("shell");
    verbs.extend(LIFECYCLE_CONTROL_VERBS);
    for desc in EFFECT_DESCRIPTORS {
        if let Some(verb) = parsed_verb_of(desc.signature) {
            verbs.push(verb);
        }
    }
    verbs.extend(expression_function_descriptors().iter().filter(|desc| desc.accepts_value_arguments()).map(|desc| &*desc.name));
    verbs.sort_unstable();
    verbs.dedup();
    verbs
}
use std::collections::HashMap;
use std::sync::LazyLock;

use crate::effects::{EFFECT_DESCRIPTORS, effect_function_descriptors};
use crate::markdown::compose::expression::{
    ExpressionFunctionDescriptor, FunctionParameterDescriptor, expression_function_descriptors,
};

use super::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::expression::catalog::load_catalog;

    /// A rest parameter after a fixed prefix (Phase 1 finding 9): no catalog
    /// function has one today, so the shape is built from a fixture catalog.
    #[test]
    fn a_rest_parameter_after_a_fixed_prefix_counts_the_prefix_as_required() {
        let source = "kind: schema\ncategories:\n    - Text -> Text.\ntypes:\n    joined: |-\n        function(\n            parameters(separator: string, ...parts: string);\n            returns(string);\n            category(Text);\n        ) -> Joins parts.\n";
        let catalog: &'static _ = Box::leak(Box::new(load_catalog(source, "fixture.yaml", &[]).expect("fixture loads")));
        let joined = VerbSignature::from_catalog("joined", &catalog.descriptors);

        assert!(joined.is_variadic());
        assert_eq!(joined.min_count(), 1);
        assert!(!joined.accepts_count(0));
        assert!(joined.accepts_count(1) && joined.accepts_count(4));
        assert!(joined.select_named(&["separator"]).is_err(), "a variadic overload has no long form");
    }

    /// The built-in catalogs live in `LazyLock` owners, so their accessors
    /// and the lifecycle views over them lend permanent borrows. The typed
    /// bindings are the check: narrowing any of these lifetimes stops this
    /// test compiling.
    #[test]
    fn static_catalog_accessors_lend_permanent_borrows() {
        use crate::catalog::{Described, Example, ExampleVerification};
        use crate::markdown::compose::expression::function_descriptors;

        let all_functions: fn() -> &'static [ExpressionFunctionDescriptor] = expression_function_descriptors;
        let overloads_of: fn(&str) -> &'static [ExpressionFunctionDescriptor] = function_descriptors;
        let all_effects: fn() -> &'static [ExpressionFunctionDescriptor] = effect_function_descriptors;

        let round: &'static [ExpressionFunctionDescriptor] = overloads_of("round");
        assert!(all_functions().iter().any(|descriptor| &*descriptor.signature == "round(x, [fallback])"));
        assert!(all_effects().iter().any(|descriptor| &*descriptor.signature == "set(key, value)"));
        let example: Option<Example<'static>> = round[0].example();
        assert_eq!(
            example,
            Some(Example {
                invocation: "round(3.7)",
                result: "4",
                verification: ExampleVerification::Executable,
            })
        );

        let verbs: Vec<&'static str> = all_lifecycle_verbs();
        assert!(verbs.contains(&"round") && verbs.contains(&"set"), "{verbs:?}");

        let function: &'static VerbSignature = expression_function_signature("round").expect("`round` is a verb");
        let verb: &'static str = function.verb();
        let overload: Overload = function.overloads()[0];
        let display: &'static str = overload.display();
        let names: Vec<&'static str> = overload.names().collect();
        let parameters: &'static [FunctionParameterDescriptor] = overload.parameters();
        let parameter: &'static str = &parameters[1].name;
        assert_eq!((verb, display, parameter), ("round", "round(x, [fallback])", "fallback"));
        assert_eq!(names, ["x", "fallback"]);

        let effect: &'static VerbSignature = side_effect_signature("set").expect("`set` is a side effect");
        let effect_verb: &'static str = effect.verb();
        let effect_names: Vec<&'static str> = effect.overloads()[0].names().collect();
        assert_eq!((effect_verb, effect_names), ("set", vec!["key", "value"]));
    }
}
