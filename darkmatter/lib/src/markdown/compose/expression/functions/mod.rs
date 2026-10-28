//! Built-in expression functions: the registry that joins each function's
//! handler to its catalog signatures, and the one dispatch entry,
//! [`call_function`], that binds a call before any handler runs.
//!
//! Every call runs in two steps. The call binder
//! ([`super::call_binding`]) binds the evaluated arguments to the function's
//! catalog overloads (a binding failure is [`ExpressionError::Call`] and the
//! handler is never entered); the handler then computes with the bound values,
//! and its successful result is checked against the selected overload's
//! return type without conversion.

use chrono::{Datelike, Local, NaiveDate, NaiveDateTime, Utc};
use serde_json::Value;
use std::collections::HashSet;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};

use super::{
    ExpressionError, FileRefFailure, FileReferenceDiagnostic, json_number, make_portable_relative_in_context,
    scalar_string,
};
use super::resolve_ctx::{
    ResolutionContext, is_remote_url, is_well_formed_reference, normalize_path_arg, resolve_document_file_ref,
    resolve_document_file_ref_shape, resolve_document_file_ref_target,
};
use crate::markdown::Markdown;
use crate::markdown::coercion::ValueKind;
use crate::markdown::numeric::{self, NumberTextError};
use crate::markdown::schemas::DarkmatterSchemas;

mod agentic_cli;
pub(in crate::markdown::compose) mod agentic_cli_generated;
mod cicd;
mod collections;
mod composition;
mod dates;
pub(in crate::markdown::compose) mod escape;
mod git;
mod markdown_docs;
mod network;
mod paths;
mod predicates;
pub(in crate::markdown::compose) mod provider;
mod pull_requests;
mod repository;
mod shell;
mod skills;
mod strings;
mod terminal;
#[cfg(any(test, feature = "effects-instrumentation"))]
pub mod test_hooks;

use super::host;
use super::call_binding::{self, BoundCall, CallBindingError, FunctionRegistration, RegistrationError};
use super::catalog::{FunctionCatalog, expression_function_catalog};

/// A handler that computes from bound arguments alone.
pub(crate) type PureFn = fn(&BoundCall<'_>) -> Result<Value, String>;
/// A handler that also reads the document resolution context.
pub(crate) type ContextFn = fn(&BoundCall<'_>, &ResolutionContext) -> Result<Value, ExpressionError>;

#[derive(Clone, Copy)]
enum FunctionHandler {
    Pure(PureFn),
    Context(ContextFn),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EvaluationMode {
    Pure,
    Context,
    Lazy,
}

#[derive(Clone, Copy)]
struct FunctionBinding {
    canonical: &'static str,
    aliases: &'static [&'static str],
    evaluation: EvaluationMode,
    /// `None` only for a lazy function, which the evaluator computes.
    handler: Option<FunctionHandler>,
    /// Registered source exclusions (`reject_sources`): parameter name and the
    /// original value kinds it refuses before any conversion.
    rejects: &'static [(&'static str, &'static [ValueKind])],
    /// Registered numeric recovery: `(subject, fallback)` parameter names.
    recovery: Option<(&'static str, &'static str)>,
    /// Registered malformed-reference rule: the parameter whose malformed
    /// reference text makes the call `false`.
    malformed_reference: Option<&'static str>,
}

impl FunctionBinding {
    const fn new(
        canonical: &'static str,
        aliases: &'static [&'static str],
        evaluation: EvaluationMode,
        handler: Option<FunctionHandler>,
    ) -> Self {
        Self {
            canonical,
            aliases,
            evaluation,
            handler,
            rejects: &[],
            recovery: None,
            malformed_reference: None,
        }
    }

    const fn pure(canonical: &'static str, aliases: &'static [&'static str], handler: PureFn) -> Self {
        Self::new(canonical, aliases, EvaluationMode::Pure, Some(FunctionHandler::Pure(handler)))
    }

    const fn context(canonical: &'static str, aliases: &'static [&'static str], handler: ContextFn) -> Self {
        Self::new(canonical, aliases, EvaluationMode::Context, Some(FunctionHandler::Context(handler)))
    }

    const fn lazy(canonical: &'static str) -> Self {
        Self::new(canonical, &[], EvaluationMode::Lazy, None)
    }

    const fn rejecting(self, rejects: &'static [(&'static str, &'static [ValueKind])]) -> Self {
        Self { rejects, ..self }
    }

    const fn recovering(self, subject: &'static str, fallback: &'static str) -> Self {
        Self {
            recovery: Some((subject, fallback)),
            ..self
        }
    }

    const fn false_for_malformed_reference(self, parameter: &'static str) -> Self {
        Self {
            malformed_reference: Some(parameter),
            ..self
        }
    }
}

/// How a lazy function ends early. Which operand value stops evaluation is
/// the evaluator's domain logic (truthiness); this only names the side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShortCircuit {
    /// Stop at the first falsy operand and return `false` (`and`).
    OnFalsy,
    /// Stop at the first truthy operand and return `true` (`or`).
    OnTruthy,
}

const LAZY_BINDINGS: &[FunctionBinding] = &[FunctionBinding::lazy("and"), FunctionBinding::lazy("or")];

fn short_circuit(canonical: &str) -> ShortCircuit {
    match canonical {
        "or" => ShortCircuit::OnTruthy,
        _ => ShortCircuit::OnFalsy,
    }
}

const BINDING_GROUPS: &[&[FunctionBinding]] = &[
    predicates::BINDINGS,
    collections::BINDINGS,
    strings::BINDINGS,
    terminal::BINDINGS,
    dates::BINDINGS,
    git::BINDINGS,
    repository::BINDINGS,
    network::BINDINGS,
    pull_requests::BINDINGS,
    cicd::BINDINGS,
    paths::BINDINGS,
    shell::BINDINGS,
    agentic_cli::BINDINGS,
    skills::BINDINGS,
    markdown_docs::BINDINGS,
    composition::BINDINGS,
    LAZY_BINDINGS,
];

fn bindings() -> impl Iterator<Item = &'static FunctionBinding> {
    BINDING_GROUPS.iter().flat_map(|group| group.iter())
}

/// The canonical name an alias registers, for a lowercased `name`; `None`
/// when `name` is not an alias. Reads only the static binding tables, so the
/// catalog can resolve aliases without building the registry.
pub(crate) fn alias_canonical(name: &str) -> Option<&'static str> {
    bindings().find(|binding| binding.aliases.contains(&name)).map(|binding| binding.canonical)
}

/// The aliases registered for `canonical`; empty for an unregistered name.
pub(crate) fn registered_aliases(canonical: &str) -> &'static [&'static str] {
    bindings().find(|binding| binding.canonical == canonical).map_or(&[], |binding| binding.aliases)
}

/// A binding joined to its catalog overloads, compiled once.
struct JoinedBinding {
    binding: &'static FunctionBinding,
    registration: FunctionRegistration,
}

#[derive(Debug, PartialEq)]
enum RegistryError {
    DuplicateBinding(String),
    CatalogWithoutBinding(String),
    BindingWithoutCatalog(String),
    NameCollision(String),
    InvalidHandler(String),
    Registration(RegistrationError),
}

impl fmt::Display for RegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (kind, name) = match self {
            Self::DuplicateBinding(name) => ("duplicate runtime binding", name),
            Self::CatalogWithoutBinding(name) => ("catalog function has no runtime binding", name),
            Self::BindingWithoutCatalog(name) => ("runtime binding has no catalog function", name),
            Self::NameCollision(name) => ("expression canonical/alias name collision", name),
            Self::InvalidHandler(name) => ("runtime binding has an invalid evaluation/handler pairing", name),
            Self::Registration(error) => return write!(formatter, "function registration failed: {error}"),
        };
        write!(formatter, "{kind}: {name}")
    }
}

/// The registry joined to `catalog`.
struct Registry {
    entries: Vec<JoinedBinding>,
}

fn build_registry(catalog: &FunctionCatalog) -> Result<Registry, RegistryError> {
    validate_name_uniqueness(bindings().map(|binding| (binding.canonical, binding.aliases)))?;
    let mut seen = HashSet::new();
    for binding in bindings() {
        if !seen.insert(binding.canonical) {
            return Err(RegistryError::DuplicateBinding(binding.canonical.to_string()));
        }
        let valid_handler = matches!(
            (binding.evaluation, binding.handler),
            (EvaluationMode::Pure, Some(FunctionHandler::Pure(_))) | (
                EvaluationMode::Context,
                Some(FunctionHandler::Context(_))
            ) | (EvaluationMode::Lazy, None)
        );
        if !valid_handler {
            return Err(RegistryError::InvalidHandler(binding.canonical.to_string()));
        }
    }
    for descriptor in &catalog.descriptors {
        if !seen.contains(&*descriptor.name) {
            return Err(RegistryError::CatalogWithoutBinding(descriptor.name.to_string()));
        }
    }

    let resolver = catalog.type_resolver();
    let mut entries = Vec::new();
    for binding in bindings() {
        let overloads: Vec<_> = catalog
            .descriptors
            .iter()
            .filter(|descriptor| &*descriptor.name == binding.canonical)
            .map(|descriptor| descriptor.function_type.clone())
            .collect();
        if overloads.is_empty() {
            return Err(RegistryError::BindingWithoutCatalog(binding.canonical.to_string()));
        }
        let registration = compile_binding(binding, overloads, &resolver)?;
        entries.push(JoinedBinding { binding, registration });
    }
    Ok(Registry { entries })
}

/// Compiles one binding's overloads with its registered rules; a rule
/// naming a parameter the catalog does not declare fails the registry.
fn compile_binding(
    binding: &FunctionBinding,
    overloads: Vec<crate::markdown::schemas::simplified::FunctionType>,
    resolver: &dyn call_binding::TypeResolver,
) -> Result<FunctionRegistration, RegistryError> {
    let mut builder = FunctionRegistration::builder_from_types(binding.canonical, overloads);
    if binding.evaluation == EvaluationMode::Lazy {
        builder = builder.evaluation(call_binding::EvaluationMode::Lazy);
    }
    for (parameter, kinds) in binding.rejects {
        builder = builder.reject_sources(*parameter, kinds);
    }
    if let Some((subject, fallback)) = binding.recovery {
        builder = builder.numeric_recovery(subject, fallback);
    }
    if let Some(parameter) = binding.malformed_reference {
        builder = builder.false_for_malformed_reference(parameter);
    }
    builder.build_with(resolver).map_err(RegistryError::Registration)
}

fn validate_name_uniqueness<'a>(
    bindings: impl IntoIterator<Item = (&'a str, &'a [&'a str])>,
) -> Result<(), RegistryError> {
    let mut names = HashSet::new();
    for (canonical, aliases) in bindings {
        if !names.insert(canonical) {
            return Err(RegistryError::NameCollision(canonical.to_string()));
        }
        for alias in aliases {
            if !names.insert(*alias) {
                return Err(RegistryError::NameCollision((*alias).to_string()));
            }
        }
    }
    Ok(())
}

fn registry() -> &'static Registry {
    static REGISTRY: LazyLock<Registry> = LazyLock::new(|| {
        build_registry(expression_function_catalog())
            .unwrap_or_else(|error| panic!("invalid embedded expression-function registry: {error}"))
    });
    &REGISTRY
}

fn joined_bindings() -> &'static [JoinedBinding] {
    &registry().entries
}

/// The registry entry `name` dispatches to: lowercased, then by canonical
/// name or alias.
fn find_binding(name: &str) -> Option<&'static JoinedBinding> {
    let name = name.to_ascii_lowercase();
    joined_bindings()
        .iter()
        .find(|entry| entry.binding.canonical == name || entry.binding.aliases.contains(&name.as_str()))
}

/// The runtime registration of the built-in function `name` names, resolved
/// as dispatch resolves it: lowercased, then by canonical name or alias.
/// `None` for an unregistered name.
///
/// This is the registration calls bind against, with the registry's source
/// exclusions and numeric recoveries; a registration built from the catalog
/// signatures alone lacks them. Thread-local test functions and host
/// functions are not consulted.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::compose::expression::functions::function_registration;
/// use serde_json::json;
///
/// let length = function_registration("LENGTH").unwrap();
/// assert_eq!(length.name(), "length");
/// assert!(length.bind_call(&[json!(true)]).is_err());
/// ```
pub fn function_registration(name: &str) -> Option<&'static FunctionRegistration> {
    find_binding(name).map(|entry| &entry.registration)
}

/// Parsed components of an indexed filename stem.
///
/// The indexed grammar is `(?P<base>.+)-(?P<digits>[0-9]+)` with the additional
/// constraint that the separating hyphen is not preceded by another hyphen.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct IndexedName {
    pub base: String,
    pub index: u64,
}

/// Parses a filename stem against the indexed-file grammar.
///
/// Accepts `review-1`, `review-100`, and `review-001`. Rejects `review1`,
/// `review_1`, `review-`, and `review--1`.
#[allow(dead_code)]
pub(crate) fn parse_indexed_stem(stem: &str) -> Option<IndexedName> {
    let last_hyphen = stem.rfind('-')?;
    if last_hyphen == 0 {
        return None;
    }
    // The hyphen immediately before the index must not itself be preceded by a
    // hyphen; otherwise the base would end with `-` (e.g. `review--1`).
    if stem.as_bytes().get(last_hyphen - 1) == Some(&b'-') {
        return None;
    }
    let base = &stem[..last_hyphen];
    let digits = &stem[last_hyphen + 1..];
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let index = digits.parse::<u64>().ok()?;
    Some(IndexedName {
        base: base.to_string(),
        index,
    })
}

/// Returns the extension portion of a basename using `std::path` semantics.
///
/// The extension is everything after the final `.`; an empty string is returned
/// when the path has no extension. This mirrors `Path::extension` so the
/// indexed-stem parser and the future `ext()` function agree.
#[allow(dead_code)]
pub(crate) fn file_extension(basename: &str) -> String {
    Path::new(basename)
        .extension()
        .map(|e| e.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// Returns the stem portion of a basename using `std::path` semantics.
#[allow(dead_code)]
pub(crate) fn file_stem(basename: &str) -> String {
    Path::new(basename)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| basename.to_string())
}

/// Resolves skill roots for an executing agent with injectable directories.
///
/// User-scoped roots are derived from `home_dir`; local-scoped roots are derived
/// from `local_root` (typically the nearest git root or document base dir).
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct SkillRoots {
    home_dir: PathBuf,
    local_root: PathBuf,
}

impl SkillRoots {
    #[allow(dead_code)]
    pub(crate) fn new(home_dir: PathBuf, local_root: PathBuf) -> Self {
        Self {
            home_dir,
            local_root,
        }
    }

    /// Normalizes an agent name to its canonical form.
    ///
    /// Returns `Some("claude" | "opencode" | "codex")` for recognized names and
    /// aliases; returns `None` for unknown agents.
    #[allow(dead_code)]
    pub(crate) fn normalize_agent(name: &str) -> Option<&'static str> {
        match name.trim().to_ascii_lowercase().as_str() {
            "claude" | "claude_code" | "claude-code" => Some("claude"),
            "opencode" | "open_code" | "open-code" => Some("opencode"),
            "codex" => Some("codex"),
            _ => None,
        }
    }

    #[allow(dead_code)]
    fn user_root(&self, canonical: &str) -> Option<PathBuf> {
        match canonical {
            "claude" => Some(self.home_dir.join(".claude").join("skills")),
            "opencode" => Some(self.home_dir.join(".config").join("opencode").join("skill")),
            "codex" => Some(self.home_dir.join(".codex").join("skills")),
            _ => None,
        }
    }

    #[allow(dead_code)]
    fn local_roots(&self, canonical: Option<&str>) -> Vec<PathBuf> {
        // The local-scoped root set is shared across all recognized agents so a
        // skill placed under any agent's local directory is discoverable by any
        // other recognized agent. Unknown agents (canonical == None) are
        // restricted to the two generic roots.
        match canonical {
            Some(_) => vec![
                self.local_root.join(".claude").join("skills"),
                self.local_root.join(".opencode").join("skill"),
                self.local_root.join(".codex").join("skills"),
                self.local_root.join(".agents").join("skills"),
            ],
            None => vec![
                self.local_root.join(".agents").join("skills"),
                self.local_root.join(".codex").join("skills"),
            ],
        }
    }

    /// Returns every root that should be searched for the given agent.
    ///
    /// For recognized agents this includes the agent's user-scoped root plus all
    /// four local-scoped roots (`.claude/skills`, `.opencode/skill`,
    /// `.codex/skills`, `.agents/skills`). Unknown agents search only the
    /// generic `.agents/skills` and `.codex/skills` local roots.
    #[allow(dead_code)]
    pub(crate) fn roots_for_agent(&self, name: &str) -> Vec<PathBuf> {
        let canonical = Self::normalize_agent(name);
        let mut roots = Vec::new();
        if let Some(c) = canonical
            && let Some(user) = self.user_root(c)
        {
            roots.push(user);
        }
        roots.extend(self.local_roots(canonical));
        roots
    }

    /// Returns only the local-scoped roots for the given agent.
    #[allow(dead_code)]
    pub(crate) fn local_roots_for_agent(&self, name: &str) -> Vec<PathBuf> {
        self.local_roots(Self::normalize_agent(name))
    }
}

/// Removes strict `YYYY-MM-DD` substrings that parse as real calendar dates.
///
/// Invalid dates such as `2026-02-30` are left untouched. Datetime strings keep
/// only their date portion removed. Whitespace and punctuation around removed
/// substrings are preserved.
#[allow(dead_code)]
pub(crate) fn remove_date_substrings(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i + 10 <= s.len() {
        let candidate = &s[i..i + 10];
        let bytes = candidate.as_bytes();
        if bytes[4] == b'-'
            && bytes[7] == b'-'
            && parse_iso_date(candidate).is_some()
        {
            i += 10;
            continue;
        }
        // Push one UTF-8 character at a time so multi-byte input is handled
        // correctly even though the date candidate is byte-aligned ASCII.
        let ch = s[i..].chars().next().expect("valid UTF-8");
        out.push(ch);
        i += ch.len_utf8();
    }
    while i < s.len() {
        let ch = s[i..].chars().next().expect("valid UTF-8");
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// Tests whether the value is "empty" per the spec: `null`, empty string,
/// empty array, or empty object. Numbers, booleans, and non-empty containers
/// all return `false`.
pub fn is_empty(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(s) => s.is_empty(),
        Value::Array(arr) => arr.is_empty(),
        Value::Object(obj) => obj.is_empty(),
        Value::Number(_) | Value::Bool(_) => false,
    }
}

/// A value the binder guarantees for a pure handler (a required parameter,
/// or a declared type the accessor reads). `Err` only when a handler and its
/// catalog signature disagree, so a defect surfaces as a domain error rather
/// than a panic.
fn bound<T>(value: Option<T>, name: &str, index: usize) -> Result<T, String> {
    value.ok_or_else(|| format!("{name}() argument {} did not bind to the type its handler reads", index + 1))
}

/// The numeric value of a bound `numberlike` (delivered as sent: a number,
/// or numeric text) or of a selected `number` fallback. Text goes through
/// the one canonical numeric-text parser the engine uses, so `number()` and
/// `round()` agree with binding on every spelling.
fn numberlike_value(value: &Value, name: &str) -> Result<f64, String> {
    let admitted = match value {
        Value::Number(number) => numeric::admit_json_number(number).map_err(NumberTextError::Admission),
        Value::String(text) => numeric::admit_number_text(text),
        _ => Err(NumberTextError::NotANumber),
    };
    admitted.map_err(|reason| format!("{name}() received a value that is not numeric ({reason}): {value}"))
}


/// Parses a strict ISO date `YYYY-MM-DD`.
pub fn parse_iso_date(s: &str) -> Option<NaiveDate> {
    if s.len() != 10 {
        return None;
    }
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}

/// Parses a string as either an ISO date or an ISO datetime, returning the
/// date portion. Datetime strings without an offset are interpreted in the
/// requested timezone (`assume_utc == true` → UTC, else Local).
pub fn parse_date_or_datetime(s: &str, assume_utc: bool) -> Option<NaiveDate> {
    if let Some(date) = parse_iso_date(s) {
        return Some(date);
    }
    if let Some(date) = parse_iso_datetime_to_date(s, assume_utc) {
        return Some(date);
    }
    None
}

/// Validates an ISO datetime string.
///
/// Accepts:
/// - `YYYY-MM-DDTHH:MM:SS` (naive)
/// - `YYYY-MM-DDTHH:MM:SS.fff` (naive with fractional seconds)
/// - `YYYY-MM-DDTHH:MM:SSZ` (UTC offset shorthand)
/// - `YYYY-MM-DDTHH:MM:SS±HH:MM` (RFC3339 offset)
pub fn parse_iso_datetime(s: &str) -> bool {
    if chrono::DateTime::parse_from_rfc3339(s).is_ok() {
        return true;
    }
    NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S").is_ok()
        || NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f").is_ok()
}

fn parse_iso_datetime_to_date(s: &str, assume_utc: bool) -> Option<NaiveDate> {
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return Some(if assume_utc {
            dt.with_timezone(&Utc).date_naive()
        } else {
            dt.with_timezone(&Local).date_naive()
        });
    }
    // Naive datetimes carry no offset; the date portion is the same regardless
    // of whether we treat the wall-clock time as local or UTC.
    NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
        .ok()
        .or_else(|| NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f").ok())
        .map(|naive| naive.date())
}


/// Pure helper for relative date predicates with an injectable reference
/// date and an explicit timezone hint for naive datetime interpretation.
pub fn is_today_with(value: &Value, today: NaiveDate, assume_utc: bool) -> bool {
    let Value::String(s) = value else {
        return false;
    };
    parse_date_or_datetime(s, assume_utc)
        .map(|d| d == today)
        .unwrap_or(false)
}

pub fn is_yesterday_with(value: &Value, today: NaiveDate, assume_utc: bool) -> bool {
    let Value::String(s) = value else {
        return false;
    };
    let yesterday = today.pred_opt();
    parse_date_or_datetime(s, assume_utc)
        .and_then(|d| yesterday.map(|y| d == y))
        .unwrap_or(false)
}

pub fn is_tomorrow_with(value: &Value, today: NaiveDate, assume_utc: bool) -> bool {
    let Value::String(s) = value else {
        return false;
    };
    let tomorrow = today.succ_opt();
    parse_date_or_datetime(s, assume_utc)
        .and_then(|d| tomorrow.map(|t| d == t))
        .unwrap_or(false)
}

pub fn is_this_month_with(value: &Value, today: NaiveDate, assume_utc: bool) -> bool {
    let Value::String(s) = value else {
        return false;
    };
    parse_date_or_datetime(s, assume_utc)
        .map(|d| d.year() == today.year() && d.month() == today.month())
        .unwrap_or(false)
}

pub fn is_this_year_with(value: &Value, today: NaiveDate, assume_utc: bool) -> bool {
    let Value::String(s) = value else {
        return false;
    };
    parse_date_or_datetime(s, assume_utc)
        .map(|d| d.year() == today.year())
        .unwrap_or(false)
}


mod fs;
pub use fs::*;


/// The canonical registry name the evaluator dispatches `name` to:
/// lowercased, then by canonical name or alias. `None` for an unregistered
/// name.
pub(crate) fn canonical_function_name(name: &str) -> Option<&'static str> {
    find_binding(name).map(|entry| entry.binding.canonical)
}

/// Whether `canonical` is a registered function with an overload that takes
/// `arity` arguments.
pub(crate) fn accepts_call_arity(canonical: &str, arity: usize) -> bool {
    joined_bindings()
        .iter()
        .find(|entry| entry.binding.canonical == canonical)
        .is_some_and(|entry| entry.registration.overloads().iter().any(|signature| signature.accepts_count(arity)))
}

/// Whether the lowercased `name` is a built-in's canonical name or alias.
pub(crate) fn is_builtin_name(name: &str) -> bool {
    find_binding(name).is_some()
}

/// Runs `inspect` on the registration a call to `name` binds against, for
/// passive design-time checks, resolved as dispatch resolves it: a thread's
/// test function, then host functions ([`super::host`]: the thread's entered
/// sets, then installed ones), then the built-ins. `None` for an unknown
/// name and for a host function whose analysis metadata is incomplete.
pub(crate) fn inspect_registration<R>(name: &str, inspect: impl FnOnce(&FunctionRegistration) -> R) -> Option<R> {
    let name = name.to_ascii_lowercase();
    #[cfg(any(test, feature = "effects-instrumentation"))]
    if let Some(function) = test_hooks::lookup(&name) {
        return Some(inspect(function.registration()));
    }
    if let Some(entry) = host::lookup(&name) {
        return match entry.state() {
            host::FunctionState::Complete { registration, .. } => Some(inspect(registration)),
            host::FunctionState::Incomplete { .. } => None,
        };
    }
    find_binding(&name).map(|entry| inspect(&entry.registration))
}

/// Whether the evaluator dispatches `name`: lowercased, then a thread's test
/// function, a host function (incomplete ones included), or a built-in's
/// canonical name or alias.
pub(crate) fn is_dispatchable(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    #[cfg(any(test, feature = "effects-instrumentation"))]
    if test_hooks::lookup(&name).is_some() {
        return true;
    }
    host::lookup(&name).is_some() || find_binding(&name).is_some()
}

/// Every canonical function name the evaluator can dispatch.
pub fn dispatchable_canonical_names() -> Vec<&'static str> {
    joined_bindings().iter().map(|entry| entry.binding.canonical).collect()
}

/// Whether `name` is a built-in that reads the document resolution context
/// (its first argument is evaluated as a caller file argument). Test and
/// host functions never do.
pub(crate) fn is_context_function(name: &str) -> bool {
    #[cfg(any(test, feature = "effects-instrumentation"))]
    if test_hooks::lookup(&name.to_ascii_lowercase()).is_some() {
        return false;
    }
    if host::lookup(&name.to_ascii_lowercase()).is_some() {
        return false;
    }
    find_binding(name).is_some_and(|entry| entry.binding.evaluation == EvaluationMode::Context)
}

/// The registration and short-circuit side of the lazy function `name`
/// names; `None` for an eager or unknown name. Test and host functions are
/// always eager.
pub(crate) fn lazy_function(name: &str) -> Option<(&'static FunctionRegistration, ShortCircuit)> {
    #[cfg(any(test, feature = "effects-instrumentation"))]
    if test_hooks::lookup(&name.to_ascii_lowercase()).is_some() {
        return None;
    }
    if host::lookup(&name.to_ascii_lowercase()).is_some() {
        return None;
    }
    let entry = find_binding(name)?;
    (entry.binding.evaluation == EvaluationMode::Lazy)
        .then(|| (&entry.registration, short_circuit(entry.binding.canonical)))
}

fn call_error(error: CallBindingError) -> ExpressionError {
    ExpressionError::Call(Box::new(error))
}

/// Calls the eager function `name` with evaluated arguments.
///
/// Resolves `name` (lowercased; canonical or alias; a thread's test function
/// first, then host functions, then built-ins), binds the arguments to its
/// overloads, enters the handler, and checks a successful result against the
/// selected overload's return type. A context function without `ctx` fails
/// before anything binds, and a host function this process has no handler
/// for (an editor's analysis-only set) fails without binding.
///
/// ## Returns
///
/// `None` only for an unknown name.
///
/// ## Errors
///
/// [`ExpressionError::Call`] when the call does not bind (the handler is
/// never entered) or the handler's result breaks its declared return type;
/// a pure handler's domain failure as [`ExpressionError::Other`]; a context
/// handler's own error as returned.
pub(crate) fn call_function(
    name: &str,
    args: &[Value],
    ctx: Option<&ResolutionContext>,
) -> Option<Result<Value, ExpressionError>> {
    let name = name.to_ascii_lowercase();
    #[cfg(any(test, feature = "effects-instrumentation"))]
    if let Some(function) = test_hooks::lookup(&name) {
        return Some(test_hooks::call(&function, &name, args));
    }
    if let Some(entry) = host::lookup(&name) {
        return Some(call_host(&entry, &name, args));
    }
    let entry = find_binding(&name)?;
    Some(call_entry(entry, &name, args, ctx))
}

/// Calls a host function through the same steps as a built-in pure handler:
/// bind, count the entry, compute, check the return.
fn call_host(entry: &host::HostEntry, name: &str, args: &[Value]) -> Result<Value, ExpressionError> {
    let failure = |message: String| ExpressionError::Other {
        function: name.to_string(),
        message,
    };
    let (registration, handler) = match entry.state() {
        host::FunctionState::Complete {
            registration,
            handler: Some(handler),
        } => (registration, handler),
        host::FunctionState::Complete { handler: None, .. } => {
            return Err(failure(format!(
                "host function {}() has no implementation in this process",
                entry.canonical()
            )));
        }
        host::FunctionState::Incomplete { reason } => {
            return Err(failure(format!(
                "host function {}() has no implementation in this process, and its analysis is incomplete: {reason}",
                entry.canonical()
            )));
        }
    };
    let call = registration.bind_call(args).map_err(call_error)?;
    #[cfg(any(test, feature = "effects-instrumentation"))]
    test_hooks::record_entry(entry.canonical());
    let value = handler(&call).map_err(failure)?;
    registration.check_return(&call, &value).map_err(call_error)?;
    Ok(value)
}

fn call_entry(
    entry: &JoinedBinding,
    name: &str,
    args: &[Value],
    ctx: Option<&ResolutionContext>,
) -> Result<Value, ExpressionError> {
    let canonical = entry.binding.canonical;
    let Some(handler) = entry.binding.handler else {
        return Err(ExpressionError::Other {
            function: name.to_string(),
            message: format!("{canonical}() evaluates its arguments lazily and is called only by the evaluator"),
        });
    };
    let context = match handler {
        FunctionHandler::Context(_) => match ctx {
            Some(context) => Some(context),
            // An opt-out or test lookup with no document surface; recoverable,
            // so it does not read as an unknown symbol.
            None => {
                return Err(ExpressionError::Other {
                    function: name.to_string(),
                    message: format!(
                        "Filesystem function '{name}' requires a document resolution context, which is unavailable here"
                    ),
                });
            }
        },
        FunctionHandler::Pure(_) => None,
    };
    let registration = &entry.registration;
    let call = registration.bind_call(args).map_err(call_error)?;
    #[cfg(any(test, feature = "effects-instrumentation"))]
    test_hooks::record_entry(canonical);
    let domain = |message: String| ExpressionError::Other {
        function: name.to_string(),
        message,
    };
    let value = match (handler, context) {
        (FunctionHandler::Pure(handler), _) => handler(&call).map_err(domain)?,
        (FunctionHandler::Context(handler), Some(context)) => handler(&call, context)?,
        (FunctionHandler::Context(_), None) => {
            unreachable!("a context handler without a context returned above")
        }
    };
    registration.check_return(&call, &value).map_err(call_error)?;
    Ok(value)
}

/// Unit tests of the pure handlers, each through the one dispatch entry
/// ([`call_function`]) so every call binds against its catalog signature
/// first. An expectation that flipped in the one-pass migration cites its
/// behavior-ledger row.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::expression::call_binding::CallErrorKind;
    use serde_json::json;

    /// A pure call through the one dispatch entry, without a context.
    fn call(name: &str, args: &[Value]) -> Result<Value, ExpressionError> {
        call_function(name, args, None).unwrap_or_else(|| panic!("{name} is not registered"))
    }

    fn ok(name: &str, args: &[Value]) -> Value {
        call(name, args).unwrap_or_else(|error| panic!("{name}({args:?}) failed: {error}"))
    }

    fn text(name: &str, args: &[Value]) -> String {
        match ok(name, args) {
            Value::String(text) => text,
            other => panic!("{name}: expected text, got {other:?}"),
        }
    }

    /// Asserts the call fails to bind with `kind` (the handler never runs).
    fn binding_error(name: &str, args: &[Value], kind: CallErrorKind) {
        match call(name, args) {
            Err(ExpressionError::Call(error)) => assert_eq!(error.kind(), kind, "{name}({args:?}): {error}"),
            other => panic!("{name}({args:?}): expected a {kind:?} binding error, got {other:?}"),
        }
    }

    fn type_error(name: &str, args: &[Value]) {
        binding_error(name, args, CallErrorKind::Type);
    }

    /// Asserts a domain failure: the call bound and the handler's work failed.
    fn domain_error(name: &str, args: &[Value]) {
        match call(name, args) {
            Err(ExpressionError::Other { .. }) => {}
            other => panic!("{name}({args:?}): expected a domain error, got {other:?}"),
        }
    }

    mod fn_date_arithmetic {
        use super::*;

        fn args3(a: &str, b: &str, d: &str) -> Vec<Value> {
            vec![json!(a), json!(b), json!(d)]
        }

        #[test]
        fn date_delta_is_direction_neutral() {
            // 19 days apart, either order → ≥ 14d.
            assert_eq!(ok("date_delta", &args3("2024-06-01", "2024-06-20", "14d")), json!(true));
            assert_eq!(ok("date_delta", &args3("2024-06-20", "2024-06-01", "14d")), json!(true));
            assert_eq!(ok("date_delta", &args3("2024-06-01", "2024-06-06", "14d")), json!(false));
        }

        #[test]
        fn threshold_is_inclusive() {
            assert_eq!(ok("date_delta", &args3("2024-06-01", "2024-06-15", "14d")), json!(true));
            assert_eq!(ok("date_delta", &args3("2024-06-01", "2024-06-14", "14d")), json!(false));
        }

        #[test]
        fn older_than_and_newer_than_are_directional() {
            assert_eq!(ok("older_than", &args3("2024-06-01", "2024-06-20", "14d")), json!(true));
            assert_eq!(ok("older_than", &args3("2024-06-20", "2024-06-01", "14d")), json!(false));
            assert_eq!(ok("newer_than", &args3("2024-06-20", "2024-06-01", "14d")), json!(true));
            assert_eq!(ok("newer_than", &args3("2024-06-01", "2024-06-20", "14d")), json!(false));
        }

        #[test]
        fn months_and_years_are_calendar_aware() {
            // Jan 15 → Mar 15 is exactly 2 calendar months (spanning short Feb),
            // which a fixed 60-day threshold would misjudge.
            assert_eq!(ok("older_than", &args3("2024-01-15", "2024-03-15", "2mo")), json!(true));
            assert_eq!(ok("older_than", &args3("2024-01-15", "2024-03-14", "2mo")), json!(false));
            assert_eq!(ok("older_than", &args3("2022-06-01", "2024-06-01", "2y")), json!(true));
        }

        #[test]
        fn duration_units_and_word_forms_parse() {
            for diff in ["14d", "14 days", "2w", "336h", "2mo", "2 months"] {
                assert_eq!(ok("date_delta", &args3("2024-06-01", "2024-08-01", diff)), json!(true), "diff {diff:?}");
            }
            // Bare `m` is minutes, not months: two same-day instants are 0 apart.
            assert_eq!(ok("date_delta", &args3("2024-06-01", "2024-06-01", "5m")), json!(false));
        }

        #[test]
        fn datetime_args_reduce_to_date() {
            assert_eq!(ok("date_delta", &args3("2024-06-01T09:00:00", "2024-06-20T23:00:00", "14d")), json!(true));
            // Every naive spelling the `datetime` type admits has a date.
            assert_eq!(ok("date_delta", &args3("2024-06-01 09:00", "2024-06-20T23:00", "14d")), json!(true));
        }

        /// Ledger rows 60–62: null is a type error (comparison family), not
        /// propagated; a number converts to text and fails the date check.
        #[test]
        fn null_and_malformed_input_are_type_errors_before_the_handler() {
            type_error("date_delta", &[json!(null), json!("2024-06-20"), json!("14d")]);
            type_error("date_delta", &[json!(5), json!("2024-06-20"), json!("14d")]);
            type_error("date_delta", &args3("not-a-date", "2024-06-20", "14d"));
            type_error("date_delta", &args3("2024-06-01", "2024-06-20", "3 fortnights"));
            binding_error("date_delta", &[json!("2024-06-01"), json!("2024-06-20")], CallErrorKind::Arity);
        }

        #[test]
        fn an_overflowing_duration_is_a_domain_error() {
            domain_error("date_delta", &args3("2024-06-01", "2024-06-20", "99999999999999999d"));
        }
    }

    mod fn_list_formatting {
        use super::*;

        fn list(value: Value) -> Vec<Value> {
            vec![value]
        }

        #[test]
        fn flat_joiners_render_scalar_elements() {
            let items = list(json!(["a", "b", "c"]));
            assert_eq!(text("as_line_separated", &items), "a\nb\nc");
            assert_eq!(text("as_csv", &items), "a, b, c");
            assert_eq!(text("as_tsv", &items), "a\tb\tc");
            assert_eq!(text("as_space_separated", &items), "a b c");
        }

        #[test]
        fn tsv_uses_tabs_only_between_elements() {
            assert_eq!(text("as_tsv", &list(json!([]))), "");
            assert_eq!(text("as_tsv", &list(json!(["a"]))), "a");
            assert_eq!(text("as_tsv", &list(json!(["a", "b"]))), "a\tb");
        }

        #[test]
        fn mixed_scalar_types_render_via_scalar_string() {
            let items = list(json!(["a", 2, true, null]));
            // `null` renders as an empty element; a number drops its `.0`.
            assert_eq!(text("as_csv", &items), "a, 2, true, ");
            assert_eq!(text("as_space_separated", &items), "a 2 true ");
        }

        #[test]
        fn empty_array_renders_empty_string() {
            let empty = list(json!([]));
            for name in ["as_line_separated", "as_csv", "as_unordered_list", "as_ordered_list"] {
                assert_eq!(text(name, &empty), "", "{name}");
            }
        }

        #[test]
        fn null_argument_propagates() {
            for name in ["as_csv", "as_unordered_list", "as_json", "as_json5"] {
                assert_eq!(ok(name, &list(json!(null))), json!(null), "{name}");
            }
        }

        /// A list parameter never wraps a scalar, so a non-list is a type
        /// error at binding rather than the handler's own check.
        #[test]
        fn non_array_argument_is_a_type_error() {
            type_error("as_csv", &list(json!("scalar")));
            type_error("as_unordered_list", &list(json!(42)));
            type_error("as_json", &list(json!("scalar")));
            type_error("as_json5", &list(json!({ "a": 1 })));
            binding_error("as_json", &[], CallErrorKind::Arity);
            binding_error("as_json5", &[json!([]), json!([])], CallErrorKind::Arity);
        }

        #[test]
        fn markdown_lists_render_flat_scalars() {
            assert_eq!(text("as_unordered_list", &list(json!(["1", "2", "3"]))), "- 1\n- 2\n- 3");
            assert_eq!(text("as_ordered_list", &list(json!(["a", "b", "c"]))), "1. a\n2. b\n3. c");
        }

        #[test]
        fn nested_arrays_auto_nest_as_sublists() {
            let items = list(json!(["first", ["a", "b"], "second"]));
            assert_eq!(text("as_ordered_list", &items), "1. first\n  1. a\n  2. b\n2. second");
            assert_eq!(text("as_unordered_list", &items), "- first\n  - a\n  - b\n- second");
            let deep = list(json!(["a", ["b", ["c", "d"]]]));
            assert_eq!(text("as_unordered_list", &deep), "- a\n  - b\n    - c\n    - d");
        }

        #[test]
        fn object_array_dependency_shape_renders_as_nested_bullets() {
            let items = list(json!([
                { "package": "darkmatter", "dependencies": ["biscuit-terminal", "renderable"] },
                { "package": "renderable", "dependencies": [] },
            ]));
            assert_eq!(
                text("as_unordered_list", &items),
                "- darkmatter\n  - biscuit-terminal\n  - renderable\n- renderable"
            );
        }

        /// `as_csv` on a one-item list yields the element alone, with no
        /// separator and no brackets.
        #[test]
        fn csv_of_a_one_item_array_is_bare_text() {
            assert_eq!(text("as_csv", &list(json!(["darkmatter"]))), "darkmatter");
        }

        /// Sample arrays covering the shapes the two serializers must handle,
        /// including quote, backslash, newline, and non-ASCII payloads.
        fn json_shapes() -> Vec<Value> {
            vec![
                json!([]),
                json!(["a", "b", "c"]),
                json!(["a", 2, 3.5, true, null]),
                json!([["a", "b"], ["c"]]),
                json!([{ "package": "darkmatter", "deps": ["renderable"] }]),
                json!([r#"say "hi""#]),
                json!([r"C:\temp\new"]),
                json!(["line one\nline two"]),
                json!(["héllo", "日本語", "emoji 🍪", "tab\there"]),
            ]
        }

        #[test]
        fn as_json_matches_bare_array_rendering_for_every_shape() {
            for shape in json_shapes() {
                assert_eq!(text("as_json", &list(shape.clone())), scalar_string(&shape), "{shape}");
            }
        }

        #[test]
        fn as_json_renders_compact_json() {
            assert_eq!(text("as_json", &list(json!(["a", 2, true]))), r#"["a",2,true]"#);
            assert_eq!(
                text("as_json", &list(json!([r#"say "hi""#, "a\\b", "x\ny"]))),
                r#"["say \"hi\"","a\\b","x\ny"]"#
            );
            assert_eq!(text("as_json", &list(json!(["日本語 🍪"]))), r#"["日本語 🍪"]"#);
        }

        #[test]
        fn as_json5_renders_compact_json5() {
            assert_eq!(text("as_json5", &list(json!(["a", 2, true]))), "['a', 2, true]");
            assert_eq!(
                text("as_json5", &list(json!([r#"say "hi""#, "a\\b", "x\ny"]))),
                r#"['say "hi"', 'a\\b', 'x\ny']"#
            );
            assert_eq!(text("as_json5", &list(json!(["日本語 🍪"]))), "['日本語 🍪']");
        }

        /// Unquoted keys and single-quoted strings are the `biscuit-file`
        /// formatter's idiom; a key needing quotes keeps them. Keys keep the
        /// author's order (`serde_json`'s `preserve_order`).
        #[test]
        fn as_json5_uses_the_formatter_object_idiom() {
            let items = list(json!([{ "package": "darkmatter", "dev deps": ["renderable"] }]));
            assert_eq!(text("as_json5", &items), "[{package: 'darkmatter', 'dev deps': ['renderable']}]");
        }

        #[test]
        fn as_json5_output_parses_back_to_the_original_value() {
            for shape in json_shapes() {
                let rendered = text("as_json5", &list(shape.clone()));
                let parsed = biscuit_file::Json5::from_str(&rendered)
                    .unwrap_or_else(|e| panic!("`{rendered}` should parse as JSON5: {e}"));
                assert_eq!(parsed.value(), &shape, "round trip changed {shape}");
            }
        }

        /// The joiners collapse an empty list to the empty string; the two
        /// serializers keep the container instead, without a final newline.
        #[test]
        fn json_serializers_preserve_the_empty_container() {
            assert_eq!(text("as_json", &list(json!([]))), "[]");
            assert_eq!(text("as_json5", &list(json!([]))), "[]");
            assert!(!text("as_json", &list(json!(["a"]))).ends_with('\n'));
            assert!(!text("as_json5", &list(json!(["a"]))).ends_with('\n'));
        }

        #[test]
        fn json_serializers_dispatch_by_canonical_name_and_alias() {
            let items = list(json!(["a", 1]));
            assert_eq!(ok("as_json", &items), json!(r#"["a",1]"#));
            assert_eq!(ok("asjson", &items), json!(r#"["a",1]"#));
            assert_eq!(ok("as_json5", &items), json!("['a', 1]"));
            assert_eq!(ok("asjson5", &items), json!("['a', 1]"));
        }
    }

    mod fn_type_predicates {
        use super::*;

        fn check(name: &str, cases: &[(Value, bool)]) {
            for (value, expected) in cases {
                assert_eq!(ok(name, std::slice::from_ref(value)), json!(expected), "{name}({value})");
            }
        }

        #[test]
        fn type_predicates_inspect_the_original_value() {
            check(
                "is_string",
                &[(json!("hi"), true), (json!(""), true), (json!(0), false), (json!(null), false), (json!([]), false)],
            );
            check(
                "is_number",
                &[(json!(0), true), (json!(1.5), true), (json!("1"), false), (json!(true), false), (json!(null), false)],
            );
            check("is_array", &[(json!([]), true), (json!([1, 2]), true), (json!({}), false), (json!("a"), false)]);
            check("is_object", &[(json!({}), true), (json!({"a": 1}), true), (json!([]), false), (json!("a"), false)]);
            check("is_null", &[(json!(null), true), (json!(""), false), (json!(0), false), (json!(false), false)]);
        }

        #[test]
        fn is_empty_is_true_for_null_and_empty_containers_only() {
            check(
                "is_empty",
                &[
                    (json!(null), true),
                    (json!(""), true),
                    (json!([]), true),
                    (json!({}), true),
                    (json!(0), false),
                    (json!(0.0), false),
                    (json!(false), false),
                    (json!("a"), false),
                    (json!([0]), false),
                    (json!({"a": 1}), false),
                ],
            );
        }

        #[test]
        fn is_integer_inspects_numbers_only() {
            check(
                "is_integer",
                &[
                    (json!(1), true),
                    (json!(1.0), true),
                    (json!(0), true),
                    (json!(-42), true),
                    (json!(1.5), false),
                    (json!(0.1), false),
                    (json!(null), false),
                    (json!("123"), false),
                    (json!(true), false),
                    (json!([1]), false),
                ],
            );
        }

        #[test]
        fn sign_predicates_compare_a_bound_number() {
            check("is_positive", &[(json!(1), true), (json!(1.5), true), (json!("0.5"), true), (json!(0), false)]);
            check("is_positive", &[(json!(-1), false), (json!("-0.5"), false), (json!(" 4 "), true), (json!("1_000"), true)]);
            check("is_negative", &[(json!(-1), true), (json!("-0.5"), true), (json!(0), false), (json!(1), false)]);
        }

        /// Ledger rows 7–8: null is `false` (predicate family; was an error),
        /// and a boolean is a type error (was read as 1/0).
        #[test]
        fn sign_predicates_answer_false_for_null_and_reject_booleans() {
            check("is_positive", &[(json!(null), false)]);
            check("is_negative", &[(json!(null), false)]);
            for name in ["is_positive", "is_negative"] {
                type_error(name, &[json!(true)]);
                type_error(name, &[json!("nope")]);
                type_error(name, &[json!([1, 2])]);
            }
        }

        #[test]
        fn predicates_dispatch_by_alias_and_check_arity() {
            assert_eq!(ok("isnegative", &[json!(-1)]), json!(true));
            assert_eq!(ok("isinteger", &[json!(42)]), json!(true));
            binding_error("is_positive", &[], CallErrorKind::Arity);
            binding_error("is_positive", &[json!(1), json!(2)], CallErrorKind::Arity);
        }
    }

    mod fn_math {
        use super::*;

        #[test]
        fn min_max_abs_basic() {
            assert_eq!(ok("min", &[json!(2), json!(5)]), json!(2));
            assert_eq!(ok("max", &[json!(2), json!(5)]), json!(5));
            assert_eq!(ok("abs", &[json!(-3)]), json!(3));
            assert_eq!(ok("abs", &[json!(3.5)]), json!(3.5));
        }

        /// Ledger rows 10–12: numeric text converts at binding.
        #[test]
        fn numeric_text_converts() {
            assert_eq!(ok("min", &[json!("4"), json!(5)]), json!(4));
            assert_eq!(ok("max", &[json!(" 4 "), json!("1_000")]), json!(1000));
            assert_eq!(ok("abs", &[json!("-3.5")]), json!(3.5));
        }

        /// Ledger rows 10–12: null is a type error (arithmetic family; it
        /// propagated before), as are booleans and non-numeric text.
        #[test]
        fn null_booleans_and_non_numbers_are_type_errors() {
            type_error("min", &[json!(null), json!(5)]);
            type_error("max", &[json!(2), json!(null)]);
            type_error("abs", &[json!(null)]);
            type_error("min", &[json!(true), json!(5)]);
            type_error("max", &[json!(false), json!(5)]);
            type_error("abs", &[json!(true)]);
            type_error("min", &[json!("foo"), json!(5)]);
            type_error("max", &[json!([]), json!(5)]);
        }
    }

    mod fn_collection {
        use super::*;

        #[test]
        fn first_last_basic_empty_and_null() {
            assert_eq!(ok("first", &[json!([1, 2, 3])]), json!(1));
            assert_eq!(ok("last", &[json!([1, 2, 3])]), json!(3));
            for name in ["first", "last"] {
                assert_eq!(ok(name, &[json!([])]), json!(null), "{name}");
                assert_eq!(ok(name, &[json!(null)]), json!(null), "{name}");
            }
        }

        #[test]
        fn first_last_reject_non_lists_at_binding() {
            type_error("first", &[json!("hi")]);
            type_error("last", &[json!({"a": 1})]);
        }

        #[test]
        fn last_preserves_typed_element() {
            // `last(outputs)` over `(string | string[])[]` returns the nested
            // array with its type intact, not a stringified form.
            assert_eq!(ok("last", &[json!(["one", ["a", "b"]])]), json!(["a", "b"]));
            assert_eq!(ok("last", &[json!(["x", {"k": 1}])]), json!({"k": 1}));
        }

        /// Ledger row 16: a null object or key is `false`; a non-object `obj`
        /// is a type error (was `false`); a number key converts to text.
        #[test]
        fn has_key_follows_the_ledger() {
            assert_eq!(ok("has_key", &[json!({"a": 1}), json!("a")]), json!(true));
            assert_eq!(ok("haskey", &[json!({"a": 1}), json!("b")]), json!(false));
            assert_eq!(ok("has_key", &[json!({"1": 1}), json!(1)]), json!(true));
            assert_eq!(ok("has_key", &[json!(null), json!("a")]), json!(false));
            assert_eq!(ok("has_key", &[json!({"a": 1}), json!(null)]), json!(false));
            type_error("has_key", &[json!("x"), json!("a")]);
            type_error("has_key", &[json!([]), json!("a")]);
            type_error("has_key", &[json!({"a": 1}), json!(["a"])]);
        }

        #[test]
        fn contains_compares_rendered_text() {
            assert_eq!(ok("contains", &[json!("hello"), json!("ell")]), json!(true));
            assert_eq!(ok("contains", &[json!("Hello"), json!("hel")]), json!(false));
            assert_eq!(ok("contains", &[json!([4]), json!("4")]), json!(true));
            assert_eq!(ok("contains", &[json!(["a", [1]]), json!([1])]), json!(true));
            assert_eq!(ok("contains", &[json!({"a": "x"}), json!("a")]), json!(false));
            assert_eq!(ok("contains", &[json!({"a": "x"}), json!("x")]), json!(true));
            // A null needle renders as empty text; a null haystack is false.
            assert_eq!(ok("contains", &[json!("abc"), json!(null)]), json!(true));
            assert_eq!(ok("contains", &[json!(null), json!("a")]), json!(false));
            // A numeric haystack converts to the string option.
            assert_eq!(ok("contains", &[json!(2026), json!(20)]), json!(true));
        }

        /// Ledger row 18: booleans and null are type errors (were `0`).
        #[test]
        fn length_counts_and_rejects_booleans_and_null() {
            assert_eq!(ok("length", &[json!("Hello 🌍")]), json!(7));
            assert_eq!(ok("length", &[json!([1, 2, 3])]), json!(3));
            assert_eq!(ok("length", &[json!({"a": 1, "b": 2})]), json!(2));
            assert_eq!(ok("length", &[json!(2.5)]), json!(3));
            assert_eq!(ok("length", &[json!(12345)]), json!(5));
            assert_eq!(ok("length", &[json!("")]), json!(0));
            type_error("length", &[json!(true)]);
            type_error("length", &[json!(null)]);
        }

        #[test]
        fn keys_and_values_keep_insertion_order() {
            assert_eq!(ok("keys", &[json!({"b": 1, "a": [2]})]), json!(["b", "a"]));
            assert_eq!(ok("values", &[json!({"b": 1, "a": [2]})]), json!([1, [2]]));
            assert_eq!(ok("keys", &[json!(null)]), json!([]));
            assert_eq!(ok("values", &[json!(null)]), json!([]));
        }
    }

    mod fn_number_and_round {
        use super::*;

        /// Ledger row 19 and charter M12–M13.
        #[test]
        fn number_reads_numberlike_and_recovers_through_the_fallback() {
            assert_eq!(ok("number", &[json!("4")]), json!(4));
            assert_eq!(ok("number", &[json!(" 4 ")]), json!(4));
            assert_eq!(ok("number", &[json!("+4")]), json!(4));
            assert_eq!(ok("number", &[json!("1_000")]), json!(1000));
            assert_eq!(ok("number", &[json!("2.5")]), json!(2.5));
            assert_eq!(ok("number", &[json!(42)]), json!(42));
            for subject in [json!("pear"), json!(null), json!(true), json!([]), json!("9007199254740993")] {
                assert_eq!(ok("number", &[subject.clone(), json!(7)]), json!(7), "{subject}");
            }
            assert_eq!(ok("number", &[json!("pear"), json!("7")]), json!(7));
            type_error("number", &[json!("pear")]);
            type_error("number", &[json!("4"), json!("x")]);
            binding_error("number", &[], CallErrorKind::Arity);
        }

        /// Ledger row 13 and charter M14: half away from zero; the fallback
        /// is rounded too.
        #[test]
        fn round_rounds_half_away_from_zero() {
            assert_eq!(ok("round", &[json!(3.7)]), json!(4));
            assert_eq!(ok("round", &[json!("3.7")]), json!(4));
            assert_eq!(ok("round", &[json!(-2.5)]), json!(-3));
            assert_eq!(ok("round", &[json!(4.5)]), json!(5));
            assert_eq!(ok("round", &[json!("pear"), json!(2.7)]), json!(3));
            assert_eq!(ok("round", &[json!("pear"), json!("2.7")]), json!(3));
            assert_eq!(ok("round", &[json!(9_007_199_254_740_992_u64)]), json!(9_007_199_254_740_992_u64));
            type_error("round", &[json!("pear")]);
            type_error("round", &[json!(null)]);
            type_error("round", &[json!(true)]);
        }
    }

    mod fn_string_predicates {
        use super::*;

        #[test]
        fn starts_with_ends_with_are_case_sensitive() {
            assert_eq!(ok("starts_with", &[json!("foobar"), json!("foo")]), json!(true));
            assert_eq!(ok("starts_with", &[json!("foobar"), json!("bar")]), json!(false));
            assert_eq!(ok("ends_with", &[json!("foobar"), json!("bar")]), json!(true));
            assert_eq!(ok("ends_with", &[json!("foobar"), json!("foo")]), json!(false));
            assert_eq!(ok("startswith", &[json!("Foobar"), json!("foo")]), json!(false));
        }

        /// Ledger rows 28–29: null is `false` (was `null`); numbers convert
        /// to text (were type errors).
        #[test]
        fn null_is_false_and_numbers_convert() {
            assert_eq!(ok("starts_with", &[json!(null), json!("foo")]), json!(false));
            assert_eq!(ok("ends_with", &[json!("hi"), json!(null)]), json!(false));
            assert_eq!(ok("starts_with", &[json!(2026), json!(20)]), json!(true));
            assert_eq!(ok("ends_with", &[json!(true), json!("ue")]), json!(true));
            type_error("starts_with", &[json!(["a"]), json!("a")]);
        }
    }

    mod fn_string_mutations {
        use super::*;

        fn one(name: &str, value: &str) -> String {
            text(name, &[json!(value)])
        }

        #[test]
        fn lower_upper_capitalize() {
            assert_eq!(one("lower", "FoO"), "foo");
            assert_eq!(one("upper", "FoO"), "FOO");
            assert_eq!(one("capitalize", "hello"), "Hello");
            assert_eq!(one("capitalize", ""), "");
        }

        #[test]
        fn case_conversions() {
            assert_eq!(one("kebab_case", "Hello World!"), "hello-world");
            assert_eq!(one("snake_case", "Hello World!"), "hello_world");
            assert_eq!(one("camel_case", "hello world"), "helloWorld");
            assert_eq!(one("pascal_case", "hello world"), "HelloWorld");
            assert_eq!(one("title_case", "hello world"), "Hello World");
            assert_eq!(one("kebab_case", "foo_bar-baz"), "foo-bar-baz");
            assert_eq!(one("snake_case", "fooBarBaz"), "foo_bar_baz");
            assert_eq!(one("camel_case", "foo-bar_baz"), "fooBarBaz");
            assert_eq!(one("pascal_case", "foo-bar_baz"), "FooBarBaz");
            assert_eq!(one("snake_case", "XMLHttpRequest"), "xml_http_request");
            assert_eq!(one("kebab_case", "XMLHttpRequest"), "xml-http-request");
            for name in ["kebab_case", "snake_case", "camel_case", "pascal_case", "title_case"] {
                assert_eq!(one(name, ""), "", "{name}");
            }
        }

        const TRANSFORMS: [&str; 10] = [
            "lower",
            "upper",
            "capitalize",
            "kebab_case",
            "snake_case",
            "camel_case",
            "pascal_case",
            "title_case",
            "trim",
            "without_date",
        ];

        #[test]
        fn transforms_pass_null_through() {
            for name in TRANSFORMS {
                assert_eq!(ok(name, &[json!(null)]), json!(null), "{name}");
            }
        }

        /// Ledger rows 30–38 (widening only): numbers and booleans convert
        /// to text; lists still do not.
        #[test]
        fn transforms_convert_numbers_and_booleans_to_text() {
            assert_eq!(text("upper", &[json!(2026)]), "2026");
            assert_eq!(text("lower", &[json!(5)]), "5");
            assert_eq!(text("upper", &[json!(true)]), "TRUE");
            assert_eq!(text("without_date", &[json!(123)]), "123");
            for name in TRANSFORMS {
                type_error(name, &[json!([])]);
            }
        }

        #[test]
        fn trim_removes_surrounding_whitespace() {
            assert_eq!(one("trim", "  a b  "), "a b");
        }

        #[test]
        fn without_date_removes_valid_dates_and_preserves_invalid() {
            assert_eq!(one("without_date", "plan 2026-06-15 review"), "plan  review");
            assert_eq!(one("without_date", "invalid 2026-02-30 stays"), "invalid 2026-02-30 stays");
            assert_eq!(one("without_date", "meeting 2026-06-15T10:30:00 here"), "meeting T10:30:00 here");
            assert_eq!(one("withoutdate", "x 2026-06-15, y"), "x , y");
        }

        fn three(name: &str, x: Value, find: Value, replacement: Value) -> Value {
            ok(name, &[x, find, replacement])
        }

        #[test]
        fn replace_core_behavior() {
            assert_eq!(three("replace", json!("a.b.c"), json!("."), json!("/")), json!("a/b/c"));
            assert_eq!(three("replace_first", json!("a.b.c"), json!("."), json!("/")), json!("a/b.c"));
            assert_eq!(three("replacefirst", json!("a.b.c"), json!("."), json!("/")), json!("a/b.c"));
            assert_eq!(three("replace_last", json!("a.b.c"), json!("."), json!("/")), json!("a.b/c"));
            assert_eq!(three("replacelast", json!("a.b.c"), json!("."), json!("/")), json!("a.b/c"));
            assert_eq!(three("replace", json!("aaa"), json!("a"), json!("bb")), json!("bbbbbb"));
            assert_eq!(three("replace_first", json!("foofoo"), json!("foo"), json!("bar")), json!("barfoo"));
            assert_eq!(three("replace_last", json!("foofoo"), json!("foo"), json!("bar")), json!("foobar"));
        }

        #[test]
        fn replace_empty_find_and_no_match_are_noops() {
            for name in ["replace", "replace_first", "replace_last"] {
                assert_eq!(three(name, json!("abc"), json!(""), json!("X")), json!("abc"), "{name}");
                assert_eq!(three(name, json!("hello"), json!("z"), json!("Q")), json!("hello"), "{name}");
            }
        }

        #[test]
        fn replace_is_case_sensitive_and_literal() {
            assert_eq!(three("replace", json!("ABCabc"), json!("abc"), json!("X")), json!("ABCX"));
            assert_eq!(three("replace", json!("a.b(c)"), json!("."), json!("*")), json!("a*b(c)"));
            assert_eq!(three("replace", json!("a(b)(c)"), json!("("), json!("[")), json!("a[b)[c)"));
            assert_eq!(three("replace_last", json!("a.b.c"), json!("."), json!("[.]")), json!("a.b[.]c"));
        }

        /// Ledger rows 41–43: null passes through in every position; numbers
        /// and booleans convert to text (were type errors).
        #[test]
        fn replace_passes_null_through_and_converts_scalars() {
            for name in ["replace", "replace_first", "replace_last"] {
                assert_eq!(three(name, json!(null), json!("."), json!("/")), json!(null), "{name}");
                assert_eq!(three(name, json!("a.b"), json!(null), json!("/")), json!(null), "{name}");
                assert_eq!(three(name, json!("a.b"), json!("."), json!(null)), json!(null), "{name}");
                assert_eq!(three(name, json!(1.5), json!("."), json!(true)), json!("1true5"), "{name}");
                type_error(name, &[json!("a"), json!(["."]), json!("/")]);
                binding_error(name, &[json!("a"), json!(".")], CallErrorKind::Arity);
                binding_error(name, &[json!("a"), json!("."), json!("/"), json!("extra")], CallErrorKind::Arity);
            }
        }
    }

    mod fn_ensure_affix {
        use super::*;

        fn two(name: &str, var: Value, affix: Value) -> Value {
            ok(name, &[var, affix])
        }

        #[test]
        fn ensure_leading_examples() {
            assert_eq!(two("ensure_leading", json!("foobar"), json!("foo")), json!("foobar"));
            assert_eq!(two("ensure_leading", json!("bar"), json!("foo")), json!("foobar"));
            assert_eq!(two("ensure_leading", json!(123), json!(4)), json!(4123));
            assert_eq!(two("ensure_leading", json!(123), json!("4")), json!(4123));
            assert_eq!(two("ensure_leading", json!("123"), json!(4)), json!("4123"));
            assert_eq!(two("ensure_leading", json!(123), json!("x")), json!("x123"));
        }

        #[test]
        fn ensure_leading_preserves_number_type_when_already_prefixed() {
            assert_eq!(two("ensure_leading", json!(123), json!("12")), json!(123));
            assert_eq!(two("ensure_leading", json!(123), json!(12)), json!(123));
        }

        #[test]
        fn ensure_trailing_examples() {
            assert_eq!(two("ensure_trailing", json!("foobar"), json!("bar")), json!("foobar"));
            assert_eq!(two("ensure_trailing", json!("foo"), json!("bar")), json!("foobar"));
            assert_eq!(two("ensure_trailing", json!(123), json!(4)), json!(1234));
            assert_eq!(two("ensuretrailing", json!("123"), json!(4)), json!("1234"));
        }

        #[test]
        fn null_passes_through() {
            for name in ["ensure_leading", "ensure_trailing"] {
                assert_eq!(two(name, json!(null), json!("foo")), json!(null), "{name}");
                assert_eq!(two(name, json!("bar"), json!(null)), json!(null), "{name}");
            }
        }

        /// Ledger rows 39–40: a boolean selects the string option and becomes
        /// its text (was rejected); lists and objects stay type errors.
        #[test]
        fn booleans_become_text_and_containers_are_type_errors() {
            assert_eq!(two("ensure_leading", json!(true), json!("is-")), json!("is-true"));
            assert_eq!(two("ensure_trailing", json!("foo"), json!(false)), json!("foofalse"));
            type_error("ensure_leading", &[json!([1]), json!("foo")]);
            type_error("ensure_leading", &[json!("bar"), json!({"a": 1})]);
            type_error("ensure_trailing", &[json!([]), json!("bar")]);
            binding_error("ensure_leading", &[json!("a")], CallErrorKind::Arity);
        }
    }

    mod fn_terminal {
        use super::*;

        #[test]
        fn terminal_renders_bold_markup() {
            let out = text("terminal", &[json!("<bold>x</bold>")]);
            assert!(out.contains("\x1b[1m"), "expected bold SGR sequence in {out:?}");
            assert!(out.contains('x'), "expected literal x in {out:?}");
            assert_eq!(text("terminal", &[json!("hello world")]), "hello world");
        }

        /// Ledger row 44: null passes through; a number converts to text.
        #[test]
        fn terminal_passes_null_through_and_converts_numbers() {
            assert_eq!(ok("terminal", &[json!(null)]), json!(null));
            assert_eq!(text("terminal", &[json!(123)]), "123");
            type_error("terminal", &[json!([1, 2])]);
            binding_error("terminal", &[], CallErrorKind::Arity);
        }
    }

    mod fn_date_format {
        use super::*;

        #[test]
        fn date_formats_known_patterns() {
            let d = |iso: &str, fmt: &str| text("date", &[json!(iso), json!(fmt)]);
            assert_eq!(d("2026-07-12", "MMMM Do"), "July 12th");
            assert_eq!(d("2026-07-12", "short"), "July 12th");
            assert_eq!(d("2026-07-12", "MMMM Do YYYY"), "July 12th 2026");
            assert_eq!(d("2021-07-12", "D MMMM YYYY"), "12 July 2021");
            assert_eq!(d("2021-07-12", "long"), "Mon, July 12th, 2021");
            assert_eq!(d("2021-07-12T08:30:00Z", "long"), "Mon, July 12th, 2021");
            assert_eq!(d("2021-07-12 08:30", "D MMMM YYYY"), "12 July 2021");
            // [YYYY] optional-year extension: omit year when it equals the current year.
            let same_year = format!("{}-07-12", Local::now().format("%Y"));
            assert_eq!(d(&same_year, "MMMM Do [YYYY]"), "July 12th");
            assert_eq!(d("1999-07-12", "MMMM Do [YYYY]"), "July 12th 1999");
        }

        /// The refined `iso` and `fmt` types reject malformed input at
        /// binding, before the handler parses anything.
        #[test]
        fn invalid_iso_and_unknown_formats_are_type_errors() {
            type_error("date", &[json!("not-a-date"), json!("short")]);
            type_error("date", &[json!("2026-07-12"), json!("nope")]);
        }

        #[test]
        fn date_passes_null_through() {
            assert_eq!(ok("date", &[json!(null), json!("short")]), json!(null));
            assert_eq!(ok("date", &[json!("2026-07-12"), json!(null)]), json!(null));
        }
    }

    mod fn_date_validators {
        use super::*;

        #[test]
        fn is_date_accepts_strict_iso_dates_only() {
            for name in ["is_date", "is_date_utc", "isdate"] {
                assert_eq!(ok(name, &[json!("2024-06-15")]), json!(true), "{name}");
            }
            for value in [json!("2024/06/15"), json!("06-15-2024"), json!("2024-13-01"), json!("2024-06-15T00:00:00")] {
                assert_eq!(ok("is_date", std::slice::from_ref(&value)), json!(false), "{value}");
            }
            for value in [json!(null), json!(20240615), json!(true)] {
                assert_eq!(ok("is_date", std::slice::from_ref(&value)), json!(false), "{value}");
                assert_eq!(ok("is_date_utc", std::slice::from_ref(&value)), json!(false), "{value}");
            }
        }

        #[test]
        fn is_date_time_accepts_iso_datetimes_only() {
            for value in ["2024-06-15T12:30:00", "2024-06-15T12:30:00Z", "2024-06-15T12:30:00+02:00", "2024-06-15T12:30:00.123"] {
                assert_eq!(ok("is_date_time", &[json!(value)]), json!(true), "{value}");
            }
            assert_eq!(ok("is_date_time_utc", &[json!("2024-06-15T12:30:00Z")]), json!(true));
            for value in [json!("2024-06-15"), json!("nope"), json!(null), json!(123)] {
                assert_eq!(ok("is_datetime", std::slice::from_ref(&value)), json!(false), "{value}");
            }
        }

        #[test]
        fn relative_validators_read_the_wall_clock() {
            let today = Local::now().date_naive();
            let iso = |date: NaiveDate| json!(date.format("%Y-%m-%d").to_string());
            assert_eq!(ok("is_today", &[iso(today)]), json!(true));
            assert_eq!(ok("is_yesterday", &[iso(today.pred_opt().unwrap())]), json!(true));
            assert_eq!(ok("is_tomorrow", &[iso(today.succ_opt().unwrap())]), json!(true));
            assert_eq!(ok("is_this_month", &[iso(today)]), json!(true));
            assert_eq!(ok("is_this_year", &[iso(today)]), json!(true));
            let utc_today = Utc::now().date_naive();
            assert_eq!(ok("is_today_utc", &[iso(utc_today)]), json!(true));
            for name in ["is_today", "is_yesterday_utc", "is_tomorrow_utc", "is_this_month_utc", "is_this_year_utc"] {
                assert_eq!(ok(name, &[json!(null)]), json!(false), "{name}");
                assert_eq!(ok(name, &[json!(20240615)]), json!(false), "{name}");
            }
        }
    }

    mod fn_istoday_relative {
        use super::*;

        const TODAY: NaiveDate = match NaiveDate::from_ymd_opt(2024, 6, 15) {
            Some(d) => d,
            None => panic!(),
        };

        #[test]
        fn istoday_with_matches_today_date() {
            assert!(is_today_with(&json!("2024-06-15"), TODAY, false));
            assert!(!is_today_with(&json!("2024-06-14"), TODAY, false));
            assert!(is_today_with(&json!("2024-06-15T08:00:00"), TODAY, false));
            assert!(is_today_with(&json!("2024-06-15T08:00:00Z"), TODAY, true));
            assert!(!is_today_with(&json!(null), TODAY, false));
            assert!(!is_today_with(&json!(20240615), TODAY, false));
            assert!(!is_today_with(&json!("not-a-date"), TODAY, false));
        }

        #[test]
        fn relative_helpers_match_their_windows() {
            assert!(is_yesterday_with(&json!("2024-06-14"), TODAY, false));
            assert!(!is_yesterday_with(&json!("2024-06-15"), TODAY, false));
            assert!(is_tomorrow_with(&json!("2024-06-16"), TODAY, false));
            assert!(!is_tomorrow_with(&json!("2024-06-15"), TODAY, false));
            assert!(is_this_month_with(&json!("2024-06-01"), TODAY, false));
            assert!(!is_this_month_with(&json!("2024-07-01"), TODAY, false));
            assert!(!is_this_month_with(&json!("2023-06-15"), TODAY, false));
            assert!(is_this_year_with(&json!("2024-12-31"), TODAY, false));
            assert!(!is_this_year_with(&json!("2023-12-31"), TODAY, false));
        }

        #[test]
        fn relative_validators_accept_datetime_strings() {
            assert!(is_today_with(&json!("2024-06-15T23:59:00Z"), TODAY, true));
            // An explicit offset that lands on a different UTC date.
            assert!(is_today_with(&json!("2024-06-16T01:00:00+03:00"), TODAY, true));
        }
    }
}

#[cfg(test)]
mod fs_tests;

#[cfg(test)]
mod phase1_helpers {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parse_indexed_stem_accepts_spec_examples() {
        assert_eq!(
            parse_indexed_stem("review-1"),
            Some(IndexedName {
                base: "review".to_string(),
                index: 1,
            })
        );
        assert_eq!(
            parse_indexed_stem("review-100"),
            Some(IndexedName {
                base: "review".to_string(),
                index: 100,
            })
        );
        assert_eq!(
            parse_indexed_stem("review-001"),
            Some(IndexedName {
                base: "review".to_string(),
                index: 1,
            })
        );
    }

    #[test]
    fn parse_indexed_stem_rejects_spec_examples() {
        assert!(parse_indexed_stem("review1").is_none());
        assert!(parse_indexed_stem("review_1").is_none());
        assert!(parse_indexed_stem("review-").is_none());
        assert!(parse_indexed_stem("review--1").is_none());
        assert!(parse_indexed_stem("-1").is_none());
    }

    #[test]
    fn file_stem_and_extension_split_basenames() {
        assert_eq!(file_stem("review-1.md"), "review-1");
        assert_eq!(file_extension("review-1.md"), "md");
        assert_eq!(file_stem("review"), "review");
        assert_eq!(file_extension("review"), "");
        assert_eq!(file_stem("foo.tar.gz"), "foo.tar");
        assert_eq!(file_extension("foo.tar.gz"), "gz");
    }

    #[test]
    fn skill_roots_normalizes_agent_aliases() {
        assert_eq!(SkillRoots::normalize_agent("claude"), Some("claude"));
        assert_eq!(SkillRoots::normalize_agent("claude_code"), Some("claude"));
        assert_eq!(SkillRoots::normalize_agent("claude-code"), Some("claude"));
        assert_eq!(SkillRoots::normalize_agent("opencode"), Some("opencode"));
        assert_eq!(SkillRoots::normalize_agent("open_code"), Some("opencode"));
        assert_eq!(SkillRoots::normalize_agent("open-code"), Some("opencode"));
        assert_eq!(SkillRoots::normalize_agent("codex"), Some("codex"));
        assert_eq!(SkillRoots::normalize_agent("Codex"), Some("codex"));
        assert_eq!(SkillRoots::normalize_agent("unknown"), None);
        assert_eq!(SkillRoots::normalize_agent("  claude  "), Some("claude"));
    }

    #[test]
    fn skill_roots_selects_known_agent_roots() {
        let home = tempfile::TempDir::new().unwrap();
        let local = tempfile::TempDir::new().unwrap();
        let roots = SkillRoots::new(home.path().to_path_buf(), local.path().to_path_buf());

        let expected: Vec<PathBuf> = vec![
            home.path().join(".claude").join("skills"),
            local.path().join(".claude").join("skills"),
            local.path().join(".opencode").join("skill"),
            local.path().join(".codex").join("skills"),
            local.path().join(".agents").join("skills"),
        ];
        assert_eq!(roots.roots_for_agent("claude-code"), expected);
    }

    #[test]
    fn skill_roots_all_recognized_agents_search_all_four_local_roots() {
        let home = tempfile::TempDir::new().unwrap();
        let local = tempfile::TempDir::new().unwrap();
        let roots = SkillRoots::new(home.path().to_path_buf(), local.path().to_path_buf());

        let expected_local: Vec<PathBuf> = vec![
            local.path().join(".claude").join("skills"),
            local.path().join(".opencode").join("skill"),
            local.path().join(".codex").join("skills"),
            local.path().join(".agents").join("skills"),
        ];

        for agent in ["claude", "opencode", "codex"] {
            assert_eq!(
                roots.local_roots_for_agent(agent),
                expected_local,
                "local roots differed for agent {agent}",
            );
        }

        // User-scoped roots remain agent-specific.
        assert_eq!(
            roots.roots_for_agent("claude"),
            vec![
                home.path().join(".claude").join("skills"),
                local.path().join(".claude").join("skills"),
                local.path().join(".opencode").join("skill"),
                local.path().join(".codex").join("skills"),
                local.path().join(".agents").join("skills"),
            ]
        );
        assert_eq!(
            roots.roots_for_agent("opencode").first(),
            Some(&home.path().join(".config").join("opencode").join("skill"))
        );
        assert_eq!(
            roots.roots_for_agent("codex").first(),
            Some(&home.path().join(".codex").join("skills"))
        );
    }

    #[test]
    fn skill_roots_selects_generic_roots_for_unknown_agent() {
        let home = tempfile::TempDir::new().unwrap();
        let local = tempfile::TempDir::new().unwrap();
        let roots = SkillRoots::new(home.path().to_path_buf(), local.path().to_path_buf());

        let expected: Vec<PathBuf> = vec![
            local.path().join(".agents").join("skills"),
            local.path().join(".codex").join("skills"),
        ];
        assert_eq!(roots.roots_for_agent("somebody"), expected);
    }

    #[test]
    fn skill_roots_user_roots_are_omitted_for_unknown_agent() {
        let home = tempfile::TempDir::new().unwrap();
        let local = tempfile::TempDir::new().unwrap();
        let roots = SkillRoots::new(home.path().to_path_buf(), local.path().to_path_buf());

        for root in roots.roots_for_agent("mystery") {
            assert!(!root.starts_with(home.path()));
        }
    }

    #[test]
    fn remove_date_substrings_removes_only_valid_dates() {
        assert_eq!(remove_date_substrings("plan 2026-06-15 review"), "plan  review");
        assert_eq!(
            remove_date_substrings("dates 2024-01-01 and 2024-12-31"),
            "dates  and "
        );
        assert_eq!(
            remove_date_substrings("invalid 2026-02-30 stays"),
            "invalid 2026-02-30 stays"
        );
    }

    #[test]
    fn remove_date_substrings_only_removes_date_portion_of_datetimes() {
        assert_eq!(
            remove_date_substrings("meeting 2026-06-15T10:30:00 here"),
            "meeting T10:30:00 here"
        );
        assert_eq!(
            remove_date_substrings("created 2026-06-15T10:30:00Z"),
            "created T10:30:00Z"
        );
    }

    #[test]
    fn remove_date_substrings_preserves_surrounding_punctuation_and_whitespace() {
        assert_eq!(
            remove_date_substrings("x 2026-06-15, y"),
            "x , y"
        );
        assert_eq!(
            remove_date_substrings("start--2026-06-15--end"),
            "start----end"
        );
    }
}

#[cfg(test)]
mod registration_tests {
    use std::collections::HashSet;

    use serde_json::{Value, json};

    use super::{EvaluationMode, FunctionHandler, bindings, call_function, joined_bindings, registry};
    use crate::markdown::compose::expression::call_binding::CallErrorKind;
    use crate::markdown::compose::expression::catalog::expression_function_descriptors;
    use crate::markdown::compose::expression::{ExpressionError, ResolutionContext};

    #[test]
    fn registration_names_and_aliases_are_unique() {
        let bindings: Vec<_> = bindings().collect();
        let canonicals: HashSet<_> = bindings.iter().map(|binding| binding.canonical).collect();
        assert_eq!(canonicals.len(), bindings.len());
        let mut all_names = canonicals;
        for binding in bindings {
            for alias in binding.aliases {
                assert!(all_names.insert(*alias), "duplicate expression alias: {alias}");
            }
        }
    }

    /// The registry builds from the embedded catalog, and every catalog
    /// function has a binding and every binding a catalog function.
    #[test]
    fn catalog_and_runtime_bindings_have_bidirectional_canonical_parity() {
        let catalog: HashSet<_> = expression_function_descriptors().iter().map(|descriptor| &*descriptor.name).collect();
        let runtime: HashSet<_> = bindings().map(|binding| binding.canonical).collect();
        assert_eq!(catalog, runtime);
        assert_eq!(joined_bindings().len(), runtime.len());
    }

    /// Every registration compiles exactly the catalog's overloads for its
    /// function, in authored order.
    #[test]
    fn each_registration_compiles_its_catalog_overloads() {
        for entry in joined_bindings() {
            let descriptors: Vec<_> = expression_function_descriptors()
                .iter()
                .filter(|descriptor| &*descriptor.name == entry.binding.canonical)
                .collect();
            assert_eq!(entry.registration.overloads().len(), descriptors.len(), "{}", entry.binding.canonical);
            for (index, descriptor) in descriptors.iter().enumerate() {
                assert_eq!(descriptor.overload, index);
            }
        }
    }

    /// `length` excludes booleans and `number`/`round` recover through their
    /// fallback; a rule the catalog cannot carry fails the registry instead.
    #[test]
    fn every_registered_rule_applies() {
        assert!(!registry().entries.is_empty());
        let length = super::function_registration("length").unwrap();
        assert!(!length.overloads()[0].parameters()[0].excluded_sources().is_empty());
        for name in ["number", "round"] {
            assert!(super::function_registration(name).unwrap().numeric_recovery().is_some(), "{name}");
        }
    }

    #[test]
    fn alias_canonical_collisions_are_rejected() {
        let fixtures = [("alpha", &["beta"][..]), ("beta", &[][..])];
        assert_eq!(
            super::validate_name_uniqueness(fixtures),
            Err(super::RegistryError::NameCollision("beta".to_string()))
        );
    }

    #[test]
    fn evaluation_modes_pair_with_their_handler_kinds() {
        for binding in bindings() {
            let paired = match binding.evaluation {
                EvaluationMode::Pure => {
                    matches!(binding.handler, Some(FunctionHandler::Pure(_)))
                }
                EvaluationMode::Context => matches!(
                    binding.handler,
                    Some(FunctionHandler::Context(_))
                ),
                EvaluationMode::Lazy => binding.handler.is_none(),
            };
            assert!(paired, "{}", binding.canonical);
            assert_eq!(super::lazy_function(binding.canonical).is_some(), binding.evaluation == EvaluationMode::Lazy);
            assert_eq!(super::is_context_function(binding.canonical), binding.evaluation == EvaluationMode::Context);
        }
    }

    #[test]
    fn unknown_names_are_not_dispatched() {
        assert!(call_function("definitely_not_a_real_function", &[], None).is_none());
        let context = ResolutionContext::default();
        assert!(call_function("definitely_not_a_real_function", &[], Some(&context)).is_none());
    }

    /// A known context function without a context fails before binding, as a
    /// recoverable error rather than an unknown symbol.
    #[test]
    fn a_context_function_without_a_context_is_a_recoverable_error() {
        let error = call_function("basename", &[json!("a.md")], None).unwrap().unwrap_err();
        assert!(matches!(error, ExpressionError::Other { .. }), "{error:?}");
        assert!(error.to_string().contains("requires a document resolution context"), "{error}");
    }

    /// Arity is the binder's: a recognized function at a count no overload
    /// takes is a typed arity error, never `None`.
    #[test]
    fn a_count_no_overload_takes_is_a_typed_arity_error() {
        let context = ResolutionContext::default();
        for (name, args) in [
            ("is_positive", vec![]),
            ("is_positive", vec![json!(1), json!(2)]),
            ("basename", vec![]),
            ("link", vec![json!("a"), json!("b"), json!("c")]),
        ] {
            let error = call_function(name, &args, Some(&context)).expect("a known function").unwrap_err();
            let ExpressionError::Call(error) = &error else {
                panic!("{name}: expected a call error, got {error:?}");
            };
            assert_eq!(error.kind(), CallErrorKind::Arity, "{name}: {error}");
        }
    }

    #[test]
    fn calls_resolve_aliases_and_case() {
        assert_eq!(call_function("IsString", &[json!("x")], None).unwrap().unwrap(), Value::Bool(true));
    }

    #[test]
    fn trim_keys_and_values_compute_from_bound_arguments() {
        let registration = |signature: &str| {
            crate::markdown::compose::expression::call_binding::FunctionRegistration::builder("f", signature)
                .build()
                .unwrap()
        };
        let trim = registration("function(parameters(text: string | null); returns(string | null))");
        let args = [json!("  a b  ")];
        assert_eq!(super::strings::trim_fn(&trim.bind_call(&args).unwrap()), Ok(json!("a b")));
        let args = [Value::Null];
        assert_eq!(super::strings::trim_fn(&trim.bind_call(&args).unwrap()), Ok(Value::Null));

        let object = registration("function(parameters(obj: object | null); returns(any[]))");
        let args = [json!({"b": 1, "a": [2]})];
        let bound = object.bind_call(&args).unwrap();
        assert_eq!(super::collections::keys_fn(&bound), Ok(json!(["b", "a"])));
        assert_eq!(super::collections::values_fn(&bound), Ok(json!([1, [2]])));
        let args = [Value::Null];
        let bound = object.bind_call(&args).unwrap();
        assert_eq!(super::collections::keys_fn(&bound), Ok(json!([])));
        assert_eq!(super::collections::values_fn(&bound), Ok(json!([])));
    }
}
