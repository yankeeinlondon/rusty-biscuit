/// Parsed form of a canonical descriptor signature string such as
/// `set_frontmatter(file, prop, value)` or `and(...)`.
///
/// `optional_tail` counts trailing parameters that are absent from every
/// overload of the verb (e.g. `ensure_file` has one optional tail parameter,
/// `content`). `variadic` is set for descriptors that explicitly use `...`.
#[derive(Debug, Clone, PartialEq)]
pub struct Signature {
    /// The verb name.
    pub verb: String,
    /// Ordered positional parameter names.
    pub params: Vec<String>,
    /// Number of trailing parameters that are optional across all overloads.
    pub optional_tail: usize,
    /// Whether the descriptor accepts a variable number of arguments.
    pub variadic: bool,
}

impl Signature {
    /// Minimum number of arguments required by this signature.
    pub fn required_count(&self) -> usize {
        self.params.len().saturating_sub(self.optional_tail)
    }

    /// Maximum number of arguments accepted by this signature.
    ///
    /// Returns `None` for variadic signatures.
    pub fn max_count(&self) -> Option<usize> {
        if self.variadic {
            None
        } else {
            Some(self.params.len())
        }
    }
}

/// Parse a canonical descriptor signature string into a typed [`Signature`].
///
/// Supported forms:
/// - `verb(p1, p2, ...)` — fixed positional parameters
/// - `verb(p1, p2?, ...)` — trailing optional parameters (explicit `?`)
/// - `verb(p1, [p2], ...)` — trailing optional parameters (bracket notation;
///   the Darkmatter catalog form, e.g. `number(x, [default])`)
/// - `verb(...)` — variadic
///
/// Returns `None` for syntactically malformed signatures.
pub fn parse_signature(signature: &str) -> Option<Signature> {
    let signature = signature.trim();
    let open = signature.find('(')?;
    let close = signature.rfind(')')?;
    if close != signature.len() - 1 || open == 0 {
        return None;
    }

    let verb = signature[..open].trim().to_string();
    if verb.is_empty() {
        return None;
    }

    let inner = signature[open + 1..close].trim();
    if inner == "..." {
        return Some(Signature {
            verb,
            params: Vec::new(),
            optional_tail: 0,
            variadic: true,
        });
    }

    if inner.is_empty() {
        return Some(Signature {
            verb,
            params: Vec::new(),
            optional_tail: 0,
            variadic: false,
        });
    }

    let mut params = Vec::new();
    let mut optional_tail = 0;
    for raw in split_signature_params(inner) {
        let raw = raw.trim();
        if raw.is_empty() {
            return None;
        }
        // Two optional-parameter spellings are accepted: a trailing `?`
        // (`content?`) and Darkmatter's catalog bracket form (`[default]`).
        let optional_name = raw
            .strip_suffix('?')
            .or_else(|| raw.strip_prefix('[').and_then(|s| s.strip_suffix(']')));
        if let Some(stripped) = optional_name {
            optional_tail += 1;
            let name = stripped.trim();
            if name.is_empty() {
                return None;
            }
            params.push(name.to_string());
        } else {
            if optional_tail > 0 {
                // A required parameter cannot follow an optional one.
                return None;
            }
            params.push(raw.to_string());
        }
    }

    Some(Signature {
        verb,
        params,
        optional_tail,
        variadic: false,
    })
}

/// Split a signature parameter list on commas, respecting nested parentheses,
/// brackets, and quotes so that type-like expressions or default values are
/// kept intact.
fn split_signature_params(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut depth = 0i32;
    let mut quote: Option<char> = None;
    for (i, ch) in s.char_indices() {
        if let Some(qc) = quote {
            if ch == qc {
                quote = None;
            }
            continue;
        }
        match ch {
            '\'' | '"' => quote = Some(ch),
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            ',' if depth == 0 => {
                out.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    if start <= s.len() {
        out.push(&s[start..]);
    }
    out
}

/// The positional parameter signature for a known Darkmatter side-effect verb.
///
/// Derived from [`EFFECT_DESCRIPTORS`]. Overloaded verbs (e.g. `ensure_file`)
/// are merged so that parameters present only in longer overloads are marked
/// as optional tail parameters.
///
/// Long-form side-effect actions carry their arguments as named sibling keys
/// (`file:`, `prop:`, `value:`). The parser uses this signature to reorder
/// those named parameters into the verb's positional call order so the
/// executor can dispatch positionally.
pub fn side_effect_signature(verb: &str) -> Option<Signature> {
    SIDE_EFFECT_SIGNATURES.get(verb).cloned()
}

/// Lazily-built map from side-effect verb to merged positional signature.
///
/// Build cost is paid once and the result is immutable; no runtime I/O or
/// host probing is performed.
static SIDE_EFFECT_SIGNATURES: std::sync::LazyLock<std::collections::HashMap<String, Signature>> =
    std::sync::LazyLock::new(build_side_effect_signatures);

fn build_side_effect_signatures() -> std::collections::HashMap<String, Signature> {
    let mut by_verb: std::collections::HashMap<String, Vec<Signature>> =
        std::collections::HashMap::new();
    for desc in EFFECT_DESCRIPTORS {
        if let Some(sig) = parse_signature(desc.signature) {
            by_verb.entry(sig.verb.clone()).or_default().push(sig);
        }
    }

    let mut out = std::collections::HashMap::new();
    for (verb, sigs) in by_verb {
        out.insert(verb, merge_signatures(&sigs));
    }
    out
}

/// Merge overloaded signatures for one verb into a single signature with the
/// longest parameter list and an `optional_tail` covering parameters that do
/// not appear in the shortest overload.
///
/// Assumes overloads only add trailing parameters, which is true for the
/// current Darkmatter catalog (`ensure_file(file)` / `ensure_file(file, content)`,
/// `frontmatter(file)` / `frontmatter(file, prop)`).
///
/// The merged `optional_tail` is the larger of two sources: the parameters the
/// shortest overload omits, and the longest overload's own intrinsic optional
/// tail (`[default]` / `name?` markers). The latter matters for a single
/// non-overloaded signature such as `number(x, [default])`, where `base` and
/// `longest` are the same signature and the omitted-parameter count is zero.
fn merge_signatures(sigs: &[Signature]) -> Signature {
    debug_assert!(!sigs.is_empty());
    let mut sorted = sigs.to_vec();
    sorted.sort_by_key(|s| s.params.len());
    let base = sorted.first().expect("non-empty signature list").clone();
    let longest = sorted.last().expect("non-empty signature list").clone();
    let from_overloads = longest.params.len().saturating_sub(base.params.len());
    let optional_tail = from_overloads.max(longest.optional_tail);
    Signature {
        verb: longest.verb,
        params: longest.params,
        optional_tail,
        variadic: longest.variadic,
    }
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

/// Returns `true` when `verb` names a lifecycle control action.
fn is_lifecycle_control_verb(verb: &str) -> bool {
    matches!(
        verb,
        "stop" | "skip" | "error" | "proxy" | "retry" | "resume" | "defer"
    )
}

/// Returns `true` when `verb` names a known Darkmatter expression function.
fn is_known_expression_function_verb(verb: &str) -> bool {
    expression_function_descriptors()
        .iter()
        .any(|d| parsed_verb_of(d.signature) == Some(verb))
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

/// Positional signature for a known Darkmatter expression-function verb.
///
/// Derived from [`expression_function_descriptors`]. Overloaded functions are
/// merged so that parameters present only in longer overloads are marked as
/// optional tail parameters.
pub fn expression_function_signature(verb: &str) -> Option<Signature> {
    let sigs: Vec<Signature> = expression_function_descriptors()
        .iter()
        .filter_map(|d| {
            let sig = parse_signature(d.signature)?;
            if sig.verb == verb { Some(sig) } else { None }
        })
        .collect();
    if sigs.is_empty() {
        return None;
    }
    Some(merge_signatures(&sigs))
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
    verbs.extend([
        "stop", "skip", "error", "proxy", "retry", "resume", "defer",
    ]);
    for desc in EFFECT_DESCRIPTORS {
        if let Some(verb) = parsed_verb_of(desc.signature) {
            verbs.push(verb);
        }
    }
    for desc in expression_function_descriptors() {
        if let Some(verb) = parsed_verb_of(desc.signature) {
            verbs.push(verb);
        }
    }
    verbs.sort_unstable();
    verbs.dedup();
    verbs
}
use darkmatter::effects::EFFECT_DESCRIPTORS;
use darkmatter::markdown::compose::expression::expression_function_descriptors;

use super::*;
