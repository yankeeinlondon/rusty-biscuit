//! The scan behind Claudine's one-implementation guards
//! (`2026-10-27-darkmatter-lifecycle-ownership`, acceptance 4), shared by
//! `claudine/lib/tests/l1/lifecycle_one_implementation_guard.rs` (library
//! source) and `claudine/cli/tests/l1/lifecycle_one_implementation_guard.rs`
//! (CLI source). Each guard scans only its own package, so CI runs the guard
//! whenever that package changes, and keeps the allowlist for the files it
//! scans.
//!
//! The scan reads production source (comments, literals, and test-only code
//! blanked by `source_scan`, which the including module declares beside this
//! one) and reports when a file:
//!
//! - defines an item named like one Darkmatter owns ([`OWNED_NAMES`]);
//! - compares `MAX_PROXY_HOPS` instead of asking
//!   `policy::proxy_budget_exhausted`;
//! - constructs a recovery-refusal error ([`DECISION_ERRORS`]) instead of
//!   mapping the kernel's refusal through `from_lifecycle_refusal`;
//! - constructs a `CompositionError` twin of a `LifecycleError` variant
//!   outside its `From` conversion;
//! - implements a Darkmatter lifecycle seam trait, or a `*Ext` trait on a
//!   Darkmatter lifecycle type;
//! - builds an event registry or a preflight plan.
//!
//! A re-export (`pub use darkmatter::lifecycle::…`) is not a definition. Each
//! legitimate adapter has an [`Allowed`] entry naming its file, item, and
//! reason; an entry that matches nothing is reported, so a list cannot go
//! stale.

use std::collections::{BTreeMap, BTreeSet};

use super::source_scan::{ident_offsets, is_ident, line_at, matching_brace};

/// Items Darkmatter's lifecycle owns; Claudine defines none of them.
pub const OWNED_NAMES: &[&str] = &[
    // Accounting, routing, catch protocol, and control admission.
    "RunAccounting",
    "record_event_emission",
    "decide_lifecycle_transition",
    "decide_control",
    "control_budget_for",
    "LifecycleCatchProtocol",
    "LifecycleRunGuard",
    "ContainerRunner",
    "MAX_PROXY_HOPS",
    "proxy_handoff_allowed",
    // The policy kernel, and the decisions it replaced.
    "recovery_verdict",
    "refusal_route",
    "failure_event",
    "proxy_budget_exhausted",
    "refused_recovery",
    "admits_recovery",
    "setup_phase_deferred",
    // The parser and its validation.
    "parse_lifecycle_stacks",
    "parse_lifecycle_config_with",
    "parse_loop_concerns_with",
    "parse_task_action_stack_with",
    "parse_single_action_with",
    "with_mapping_shape",
    "validate_no_nested_spans_in_literals",
    // The executor, the registry, and preflight.
    "StackExecutor",
    "EventRegistry",
    "PreflightPlan",
    "PreflightRun",
    "LifecycleShellSites",
    "stamp",
    "resolve_shell_text",
    "guarded_template",
    "sequence_lifecycle_template",
    "lifecycle_shell_candidates",
    "collect_lifecycle_shell_audit",
    "settle_lifecycle_shell_modes",
    "lifecycle_document_roots",
];

/// Errors that report a refused recovery; only the kernel's refusal decides
/// them.
const DECISION_ERRORS: &[&str] =
    &["LifecycleSetupPhaseRecoveryUnsupported", "LifecycleResumeWithoutSession", "LifecycleDeferNotImplemented"];

/// The `From<LifecycleError>` conversion whose variants name the twins,
/// relative to `claudine/`.
pub const TWIN_SOURCE: &str = "lib/src/composition/error/lifecycle_authoring.rs";

/// Darkmatter constructors whose calls register a host's events or checks.
const REGISTRATIONS: &[&str] = &["EventRegistry", "PreflightPlan"];

const DEFINITION_KEYWORDS: &[&str] = &["fn", "struct", "enum", "trait", "type", "const", "static", "union"];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rule {
    /// A definition of an [`OWNED_NAMES`] item; `item` is its name.
    Definition,
    /// A [`DECISION_ERRORS`] construction; `item` is `function: Variant`.
    DecisionError,
    /// A parser-twin construction; `item` is `function: Variant`.
    ParserTwin,
    /// A seam implementation; `item` is `Trait for Type`.
    Seam,
    /// A registration; `item` is `function: Constructor`.
    Registration,
}

/// One reviewed adapter. `path` is relative to `claudine/`.
pub struct Allowed {
    rule: Rule,
    path: &'static str,
    item: &'static str,
    reason: &'static str,
}

pub const fn allowed(rule: Rule, path: &'static str, item: &'static str, reason: &'static str) -> Allowed {
    Allowed { rule, path, item, reason }
}

/// One `fn` and its body's byte range in sanitized text.
struct Function<'a> {
    name: &'a str,
    body: std::ops::Range<usize>,
}

/// Every `fn` with a body in sanitized `text`.
fn functions(text: &str) -> Vec<Function<'_>> {
    let bytes = text.as_bytes();
    ident_offsets(text, "fn")
        .into_iter()
        .filter_map(|at| {
            let name_start = skip_space(text, at + 2);
            let name_len = text[name_start..].bytes().take_while(|&byte| is_ident(byte)).count();
            if name_len == 0 {
                return None; // a `fn(..)` pointer type
            }
            let after = name_start + name_len;
            let open = after + text[after..].find(['{', ';'])?;
            (bytes[open] == b'{').then(|| Function { name: &text[name_start..after], body: open..matching_brace(bytes, open) })
        })
        .collect()
}

/// The innermost function whose body holds `offset`.
fn enclosing<'a>(functions: &'a [Function<'a>], offset: usize) -> &'a str {
    functions
        .iter()
        .filter(|function| function.body.contains(&offset))
        .max_by_key(|function| function.body.start)
        .map_or("<item>", |function| function.name)
}

fn skip_space(text: &str, from: usize) -> usize {
    from + (text[from..].len() - text[from..].trim_start().len())
}

/// The identifier starting at `from` after whitespace.
fn ident_at(text: &str, from: usize) -> &str {
    let start = skip_space(text, from);
    let len = text[start..].bytes().take_while(|&byte| is_ident(byte)).count();
    &text[start..start + len]
}

/// Offset of the delimiter closing the one at `open` (`(`, `[`, `{`, or `<`).
fn closing(bytes: &[u8], open: usize) -> usize {
    let (up, down) = match bytes[open] {
        b'(' => (b'(', b')'),
        b'[' => (b'[', b']'),
        b'<' => (b'<', b'>'),
        _ => (b'{', b'}'),
    };
    let mut depth = 0usize;
    for (offset, &byte) in bytes.iter().enumerate().skip(open) {
        if byte == up {
            depth += 1;
        } else if byte == down && !(down == b'>' && bytes[offset - 1] == b'-') {
            depth -= 1;
            if depth == 0 {
                return offset;
            }
        }
    }
    bytes.len() - 1
}

/// Names `text` defines with one of `keywords` (`fn name`, `struct Name`, …).
fn definitions<'a>(text: &'a str, keywords: &[&str]) -> Vec<(usize, &'a str)> {
    keywords
        .iter()
        .flat_map(|keyword| {
            ident_offsets(text, keyword).into_iter().filter_map(move |at| {
                if text[..at].ends_with('\'') {
                    return None; // the `'static` lifetime
                }
                let name = ident_at(text, at + keyword.len());
                (!name.is_empty()).then_some((at, name))
            })
        })
        .collect()
}

/// Whether the variant reference whose name ends at `end` is a construction
/// rather than a pattern: a pattern ends in `..` or `_`, or is followed by
/// `=>`, `|`, `=`, or a match guard.
fn is_construction(text: &str, end: usize) -> bool {
    let bytes = text.as_bytes();
    let open = skip_space(text, end);
    if !matches!(bytes.get(open), Some(b'{' | b'(')) {
        return false;
    }
    let close = closing(bytes, open);
    let inner = text[open + 1..close].trim();
    if inner.ends_with("..") || inner == "_" {
        return false;
    }
    let rest = text[close + 1..].trim_start();
    let followed_by = |token: &str| rest.starts_with(token);
    !(followed_by("=>")
        || (followed_by("|") && !followed_by("||"))
        || (followed_by("=") && !followed_by("=="))
        || ident_at(rest, 0) == "if")
}

/// Offsets of `variant` constructed as `CompositionError::variant` (or as
/// `Self::variant` inside the error module).
fn constructions(path: &str, text: &str, variant: &str) -> Vec<usize> {
    let in_error_module = path.starts_with("lib/src/composition/error/");
    ident_offsets(text, variant)
        .into_iter()
        .filter(|&at| {
            let before = text[..at].trim_end();
            let qualified = before.strip_suffix("::").map(str::trim_end).is_some_and(|owner| {
                owner.ends_with("CompositionError") || (in_error_module && owner.ends_with("Self"))
            });
            qualified && is_construction(text, at + variant.len())
        })
        .collect()
}

/// Whether `MAX_PROXY_HOPS` at `at` is an operand of a comparison.
fn is_compared(text: &str, at: usize, name: &str) -> bool {
    let mut before = text[..at].trim_end();
    // Step back over a path prefix (`super::control::`).
    while let Some(prefix) = before.strip_suffix("::") {
        before = prefix.trim_end().trim_end_matches(|character: char| character == '_' || character.is_ascii_alphanumeric()).trim_end();
    }
    let after = text[at + name.len()..].trim_start();
    let compares_before = [">=", "<=", "==", "!="].iter().any(|op| before.ends_with(op))
        || (before.ends_with('<') && !before.ends_with("<<"))
        || (before.ends_with('>') && !before.ends_with("->") && !before.ends_with("=>") && !before.ends_with(">>"));
    let compares_after = [">=", "<=", "==", "!="].iter().any(|op| after.starts_with(op))
        || (after.starts_with('<') && !after.starts_with("<<") && !after.starts_with("<="))
        || (after.starts_with('>') && !after.starts_with(">>") && !after.starts_with(">="));
    compares_before || compares_after
}

/// The last path segment of a type or trait, without generics.
fn last_segment(path: &str) -> String {
    let mut depth = 0usize;
    let plain: String = path
        .chars()
        .filter(|&character| {
            match character {
                '<' => depth += 1,
                '>' => depth = depth.saturating_sub(1),
                _ => return depth == 0,
            }
            false
        })
        .collect();
    plain.rsplit("::").next().unwrap_or("").trim().trim_start_matches('&').trim().to_string()
}

/// `(offset, trait, type)` of every `impl Trait for Type` in `text`.
fn trait_impls(text: &str) -> Vec<(usize, String, String)> {
    let bytes = text.as_bytes();
    ident_offsets(text, "impl")
        .into_iter()
        .filter_map(|at| {
            let mut cursor = skip_space(text, at + 4);
            if bytes.get(cursor) == Some(&b'<') {
                cursor = skip_space(text, closing(bytes, cursor) + 1);
            }
            let header_end = cursor + text[cursor..].find(['{', ';'])?;
            let header = &text[cursor..header_end];
            let header = header.split(" where ").next().unwrap_or(header);
            let split = ident_offsets(header, "for").into_iter().next()?;
            let trait_name = last_segment(&header[..split]);
            let type_name = last_segment(&header[split + 3..]);
            Some((at, trait_name, type_name))
        })
        .collect()
}

/// What Darkmatter's lifecycle module declares, read from its source, and
/// the `CompositionError` twins of its `LifecycleError` variants.
pub struct Owned {
    /// Public traits a host implements.
    pub seams: BTreeSet<String>,
    /// Public types an `*Ext` trait may extend.
    pub types: BTreeSet<String>,
    /// `LifecycleError` variants [`TWIN_SOURCE`] converts; Claudine builds
    /// each twin only inside that conversion.
    pub twins: BTreeSet<String>,
}

impl Owned {
    /// Reads Darkmatter's lifecycle `sources` and the sanitized text of
    /// [`TWIN_SOURCE`].
    pub fn read(sources: &BTreeMap<String, String>, twin_source: &str) -> Self {
        let mut seams = BTreeSet::new();
        let mut types = BTreeSet::new();
        for text in sources.values() {
            for (at, name) in definitions(text, &["trait", "struct", "enum", "type"]) {
                if !text[..at].trim_end().ends_with("pub") {
                    continue;
                }
                if text[at..].starts_with("trait") {
                    seams.insert(name.to_string());
                } else {
                    types.insert(name.to_string());
                }
            }
        }
        let twins = ident_offsets(twin_source, "LifecycleError")
            .into_iter()
            .filter_map(|at| twin_source[at + "LifecycleError".len()..].strip_prefix("::"))
            .map(|rest| ident_at(rest, 0).to_string())
            .filter(|name| !name.is_empty())
            .collect();
        Self { seams, types, twins }
    }
}

/// Every violation in Claudine's `sources` (keyed relative to `claudine/`),
/// and every allowlist entry that matched nothing.
pub fn violations(sources: &BTreeMap<String, String>, owned: &Owned, allowed: &[Allowed]) -> Vec<String> {

    // Every finding, keyed by the allowlist key that would cover it.
    let mut found: BTreeMap<(Rule, String, String), Vec<usize>> = BTreeMap::new();
    let mut found_at = |rule: Rule, path: &str, item: String, line: usize| {
        found.entry((rule, path.to_string(), item)).or_default().push(line);
    };
    let mut violations = Vec::new();

    for (path, text) in sources {
        let functions = functions(text);
        for (at, name) in definitions(text, DEFINITION_KEYWORDS) {
            if OWNED_NAMES.contains(&name) {
                found_at(Rule::Definition, path, name.to_string(), line_at(text, at));
            }
        }
        for at in ident_offsets(text, "MAX_PROXY_HOPS") {
            if is_compared(text, at, "MAX_PROXY_HOPS") {
                violations.push(format!(
                    "{path}:{}: `MAX_PROXY_HOPS` compared directly; ask `policy::proxy_budget_exhausted`",
                    line_at(text, at)
                ));
            }
        }
        for variant in DECISION_ERRORS {
            for at in constructions(path, text, variant) {
                found_at(Rule::DecisionError, path, format!("{}: {variant}", enclosing(&functions, at)), line_at(text, at));
            }
        }
        for variant in &owned.twins {
            for at in constructions(path, text, variant) {
                let function = enclosing(&functions, at);
                let item = if path == TWIN_SOURCE && function == "from" { "from: *".to_string() } else { format!("{function}: {variant}") };
                found_at(Rule::ParserTwin, path, item, line_at(text, at));
            }
        }
        for (at, trait_name, type_name) in trait_impls(text) {
            let extends = trait_name.ends_with("Ext") && owned.types.contains(&type_name);
            if owned.seams.contains(&trait_name) || extends {
                found_at(Rule::Seam, path, format!("{trait_name} for {type_name}"), line_at(text, at));
            }
        }
        for constructor in REGISTRATIONS {
            for at in ident_offsets(text, constructor) {
                let called = text[at + constructor.len()..].trim_start().strip_prefix("::").is_some_and(|rest| ident_at(rest, 0) == "new");
                if called {
                    found_at(Rule::Registration, path, format!("{}: {constructor}", enclosing(&functions, at)), line_at(text, at));
                }
            }
        }
    }

    for entry in allowed {
        if entry.reason.trim().is_empty() {
            violations.push(format!("{:?} entry {}::{} has no reason", entry.rule, entry.path, entry.item));
        }
        if found.remove(&(entry.rule, entry.path.to_string(), entry.item.to_string())).is_none() {
            violations.push(format!("stale {:?} entry: nothing matches {}::{} (remove it)", entry.rule, entry.path, entry.item));
        }
    }
    for ((rule, path, item), lines) in found {
        let advice = match rule {
            Rule::Definition => "Darkmatter owns this item; re-export or call it",
            Rule::DecisionError => "map the kernel's refusal with `CompositionError::from_lifecycle_refusal`",
            Rule::ParserTwin => "convert the `LifecycleError` through `From`",
            Rule::Seam | Rule::Registration => "add a reviewed adapter entry with its reason",
        };
        violations.push(format!("{rule:?} `{item}` in {path} (lines {lines:?}): {advice}"));
    }
    violations
}
