//! Built-in helper functions for the Darkmatter expression evaluator.
//!
//! This module groups the type predicates, math helpers, collection helpers,
//! string predicates and mutations, and date validators added in the
//! expression-syntax expansion. Functions here all take fully-evaluated
//! `serde_json::Value` arguments and follow the spec's null-propagation /
//! type-mismatch contract: a `Value::Null` argument propagates through the
//! call as `Value::Null`, while a value of the wrong domain returns an
//! evaluator error.

use chrono::{Datelike, Local, NaiveDate, NaiveDateTime, Utc};
use serde_json::Value;
use std::path::{Path, PathBuf};

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;

use super::{json_number, scalar_string, to_number, to_number_coerce};
use super::resolve_ctx::{ResolutionContext, is_remote_url, normalize_path_arg};
use crate::markdown::Markdown;
use crate::markdown::schemas::DarkmatterSchemas;

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

/// Renders a path with `/` separators for stable Markdown output.
///
/// Platform path semantics are used for parsing; the result is normalized to
/// forward slashes so composed Markdown is portable.
#[allow(dead_code)]
pub(crate) fn display_path_with_forward_slashes(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
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

fn require_args(name: &str, args: &[Value], expected: usize) -> Result<(), String> {
    if args.len() != expected {
        Err(format!(
            "{name}() requires {expected} argument{}",
            if expected == 1 { "" } else { "s" }
        ))
    } else {
        Ok(())
    }
}

fn any_null(args: &[Value]) -> bool {
    args.iter().any(Value::is_null)
}

fn require_number(name: &str, value: &Value) -> Result<f64, String> {
    match value {
        Value::Number(n) => n
            .as_f64()
            .ok_or_else(|| format!("{name}() received an unrepresentable number")),
        _ => Err(format!("{name}() requires numeric arguments")),
    }
}

fn require_string<'a>(name: &str, value: &'a Value) -> Result<&'a str, String> {
    match value {
        Value::String(s) => Ok(s.as_str()),
        _ => Err(format!("{name}() requires string arguments")),
    }
}

fn require_array<'a>(name: &str, value: &'a Value) -> Result<&'a Vec<Value>, String> {
    match value {
        Value::Array(items) => Ok(items),
        _ => Err(format!("{name}() requires an array argument")),
    }
}

/// Type predicates from the spec.
pub fn is_string(args: &[Value]) -> Result<Value, String> {
    require_args("is_string", args, 1)?;
    Ok(Value::Bool(matches!(args[0], Value::String(_))))
}

pub fn is_number(args: &[Value]) -> Result<Value, String> {
    require_args("is_number", args, 1)?;
    Ok(Value::Bool(matches!(args[0], Value::Number(_))))
}

pub fn is_array(args: &[Value]) -> Result<Value, String> {
    require_args("is_array", args, 1)?;
    Ok(Value::Bool(matches!(args[0], Value::Array(_))))
}

pub fn is_null(args: &[Value]) -> Result<Value, String> {
    require_args("is_null", args, 1)?;
    Ok(Value::Bool(matches!(args[0], Value::Null)))
}

pub fn is_object(args: &[Value]) -> Result<Value, String> {
    require_args("is_object", args, 1)?;
    Ok(Value::Bool(matches!(args[0], Value::Object(_))))
}

pub fn is_empty_fn(args: &[Value]) -> Result<Value, String> {
    require_args("is_empty", args, 1)?;
    Ok(Value::Bool(is_empty(&args[0])))
}

/// `is_positive(val)` — `true` only when `to_number(val) > 0`.
pub fn is_positive(args: &[Value]) -> Result<Value, String> {
    require_args("is_positive", args, 1)?;
    match to_number(&args[0]) {
        Some(n) => Ok(Value::Bool(n > 0.0)),
        None => Err("is_positive() cannot coerce argument to a number".to_string()),
    }
}

/// `is_negative(val)` — `true` only when `to_number(val) < 0`.
pub fn is_negative(args: &[Value]) -> Result<Value, String> {
    require_args("is_negative", args, 1)?;
    match to_number(&args[0]) {
        Some(n) => Ok(Value::Bool(n < 0.0)),
        None => Err("is_negative() cannot coerce argument to a number".to_string()),
    }
}

/// `is_integer(val)` — inspecting predicate; `true` only for JSON numbers with
/// no fractional component. Never errors and does not null-propagate.
pub fn is_integer(args: &[Value]) -> Result<Value, String> {
    require_args("is_integer", args, 1)?;
    let ok = match &args[0] {
        Value::Number(n) => n
            .as_f64()
            .map(|f| f.is_finite() && f.fract() == 0.0)
            .unwrap_or(false),
        _ => false,
    };
    Ok(Value::Bool(ok))
}

/// `min(a, b)`.
pub fn min_fn(args: &[Value]) -> Result<Value, String> {
    require_args("min", args, 2)?;
    if any_null(args) {
        return Ok(Value::Null);
    }
    let a = require_number("min", &args[0])?;
    let b = require_number("min", &args[1])?;
    json_number(a.min(b))
}

/// `max(a, b)`.
pub fn max_fn(args: &[Value]) -> Result<Value, String> {
    require_args("max", args, 2)?;
    if any_null(args) {
        return Ok(Value::Null);
    }
    let a = require_number("max", &args[0])?;
    let b = require_number("max", &args[1])?;
    json_number(a.max(b))
}

/// `abs(x)`.
pub fn abs_fn(args: &[Value]) -> Result<Value, String> {
    require_args("abs", args, 1)?;
    if any_null(args) {
        return Ok(Value::Null);
    }
    let x = require_number("abs", &args[0])?;
    json_number(x.abs())
}

/// `first(x)` returns the first element of an array (or null when empty).
pub fn first_fn(args: &[Value]) -> Result<Value, String> {
    require_args("first", args, 1)?;
    if any_null(args) {
        return Ok(Value::Null);
    }
    let items = require_array("first", &args[0])?;
    Ok(items.first().cloned().unwrap_or(Value::Null))
}

/// `last(x)` returns the last element of an array (or null when empty).
pub fn last_fn(args: &[Value]) -> Result<Value, String> {
    require_args("last", args, 1)?;
    if any_null(args) {
        return Ok(Value::Null);
    }
    let items = require_array("last", &args[0])?;
    Ok(items.last().cloned().unwrap_or(Value::Null))
}

/// `starts_with(x, find)` — case-sensitive prefix test.
pub fn starts_with(args: &[Value]) -> Result<Value, String> {
    require_args("starts_with", args, 2)?;
    if any_null(args) {
        return Ok(Value::Null);
    }
    let haystack = require_string("starts_with", &args[0])?;
    let needle = require_string("starts_with", &args[1])?;
    Ok(Value::Bool(haystack.starts_with(needle)))
}

/// `ends_with(x, find)` — case-sensitive suffix test.
pub fn ends_with(args: &[Value]) -> Result<Value, String> {
    require_args("ends_with", args, 2)?;
    if any_null(args) {
        return Ok(Value::Null);
    }
    let haystack = require_string("ends_with", &args[0])?;
    let needle = require_string("ends_with", &args[1])?;
    Ok(Value::Bool(haystack.ends_with(needle)))
}

fn string_mutation<F>(name: &str, args: &[Value], f: F) -> Result<Value, String>
where
    F: Fn(&str) -> String,
{
    require_args(name, args, 1)?;
    if any_null(args) {
        return Ok(Value::Null);
    }
    let s = require_string(name, &args[0])?;
    Ok(Value::String(f(s)))
}

pub fn lower(args: &[Value]) -> Result<Value, String> {
    string_mutation("lower", args, |s| s.to_lowercase())
}

pub fn upper(args: &[Value]) -> Result<Value, String> {
    string_mutation("upper", args, |s| s.to_uppercase())
}

pub fn capitalize(args: &[Value]) -> Result<Value, String> {
    string_mutation("capitalize", args, |s| {
        let mut chars = s.chars();
        match chars.next() {
            Some(first) => first.to_uppercase().chain(chars).collect(),
            None => String::new(),
        }
    })
}

/// Splits a string into "words" suitable for case-conversion. Words are
/// runs of letters or digits, with boundaries at any non-alphanumeric
/// character or transitions like `aB` (camel boundary) and `IDFoo`
/// (acronym boundary). The output is lowercase tokens.
fn split_words(input: &str) -> Vec<String> {
    let mut words: Vec<String> = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = input.chars().collect();

    let flush = |current: &mut String, words: &mut Vec<String>| {
        if !current.is_empty() {
            words.push(current.to_lowercase());
            current.clear();
        }
    };

    for i in 0..chars.len() {
        let c = chars[i];
        if !c.is_alphanumeric() {
            flush(&mut current, &mut words);
            continue;
        }

        if !current.is_empty() {
            let prev = chars[i - 1];
            // Word boundaries:
            // - lower→Upper transition (e.g., "fooBar" → "foo|Bar")
            // - digit↔letter transition (e.g., "v2foo" → "v|2|foo")
            // - acronym boundary: "FOOBar" → "FOO|Bar"
            let lower_to_upper = prev.is_lowercase() && c.is_uppercase();
            let alpha_digit_flip = prev.is_alphabetic() != c.is_alphabetic();
            let acronym_break = prev.is_uppercase()
                && c.is_uppercase()
                && i + 1 < chars.len()
                && chars[i + 1].is_lowercase();
            if lower_to_upper || alpha_digit_flip || acronym_break {
                flush(&mut current, &mut words);
            }
        }
        current.push(c);
    }
    flush(&mut current, &mut words);
    words
}

pub fn kebab_case(args: &[Value]) -> Result<Value, String> {
    string_mutation("kebab_case", args, |s| split_words(s).join("-"))
}

pub fn snake_case(args: &[Value]) -> Result<Value, String> {
    string_mutation("snake_case", args, |s| split_words(s).join("_"))
}

pub fn camel_case(args: &[Value]) -> Result<Value, String> {
    string_mutation("camel_case", args, |s| {
        let words = split_words(s);
        let mut iter = words.into_iter();
        let mut out = String::new();
        if let Some(first) = iter.next() {
            out.push_str(&first);
        }
        for word in iter {
            let mut chars = word.chars();
            if let Some(c) = chars.next() {
                out.extend(c.to_uppercase());
                out.extend(chars);
            }
        }
        out
    })
}

pub fn pascal_case(args: &[Value]) -> Result<Value, String> {
    string_mutation("pascal_case", args, |s| {
        let mut out = String::new();
        for word in split_words(s) {
            let mut chars = word.chars();
            if let Some(c) = chars.next() {
                out.extend(c.to_uppercase());
                out.extend(chars);
            }
        }
        out
    })
}

pub fn title_case(args: &[Value]) -> Result<Value, String> {
    string_mutation("title_case", args, |s| {
        let mut parts: Vec<String> = Vec::new();
        for word in split_words(s) {
            let mut chars = word.chars();
            if let Some(c) = chars.next() {
                let mut buf: String = c.to_uppercase().collect();
                buf.extend(chars);
                parts.push(buf);
            }
        }
        parts.join(" ")
    })
}

/// `without_date(string)` — removes strict `YYYY-MM-DD` substrings that parse
/// as real calendar dates. Null-propagates; non-string arguments error.
pub fn without_date(args: &[Value]) -> Result<Value, String> {
    require_args("without_date", args, 1)?;
    if args[0].is_null() {
        return Ok(Value::Null);
    }
    let s = require_string("without_date", &args[0])?;
    Ok(Value::String(remove_date_substrings(s)))
}

/// Whether a scalar JSON value is usable as a string/number operand for
/// `ensure_leading` / `ensure_trailing`. Arrays, objects, and booleans are
/// rejected; strings and numbers are accepted.
fn ensure_operand(value: &Value) -> Result<String, String> {
    match value {
        Value::String(s) => Ok(s.clone()),
        Value::Number(_) => Ok(scalar_string(value)),
        Value::Bool(_) => Err("ensure_leading() does not accept boolean arguments".to_string()),
        Value::Array(_) | Value::Object(_) => {
            Err("ensure_leading() does not accept array or object arguments".to_string())
        }
        Value::Null => Ok(String::new()),
    }
}

/// Returns `true` when `s` parses as a finite number.
fn is_numberlike_string(s: &str) -> bool {
    s.parse::<f64>().map(|f| f.is_finite()).unwrap_or(false)
}

/// Builds a numeric result from concatenated decimal string forms, falling
/// back to a string when the value is not representable as JSON number.
fn ensure_numeric_concat(concatenated: &str) -> Result<Value, String> {
    if let Some(n) = concatenated.parse::<f64>().ok().filter(|f| f.is_finite())
        && let Ok(v) = json_number(n)
    {
        return Ok(v);
    }
    Ok(Value::String(concatenated.to_string()))
}

/// `ensure_leading(var, prefix)` — ensures the string form of `var` starts
/// with the string form of `prefix`. Preserves the original JSON type when
/// already prefixed; returns a JSON number when `var` is a number and the
/// result is representable.
pub fn ensure_leading(args: &[Value]) -> Result<Value, String> {
    require_args("ensure_leading", args, 2)?;
    if any_null(args) {
        return Ok(Value::Null);
    }
    let var = &args[0];
    let prefix = &args[1];
    if matches!(var, Value::Bool(_) | Value::Array(_) | Value::Object(_))
        || matches!(prefix, Value::Bool(_) | Value::Array(_) | Value::Object(_))
    {
        return Err("ensure_leading() arguments must be strings or numbers".to_string());
    }
    let var_str = ensure_operand(var)?;
    let prefix_str = ensure_operand(prefix)?;
    if var_str.starts_with(&prefix_str) {
        return Ok(var.clone());
    }
    let combined = format!("{prefix_str}{var_str}");
    if matches!(var, Value::Number(_))
        && (matches!(prefix, Value::Number(_)) || is_numberlike_string(&prefix_str))
    {
        ensure_numeric_concat(&combined)
    } else {
        Ok(Value::String(combined))
    }
}

/// `ensure_trailing(var, postfix)` — ensures the string form of `var` ends
/// with the string form of `postfix`. Same type-preservation rules as
/// [`ensure_leading`].
pub fn ensure_trailing(args: &[Value]) -> Result<Value, String> {
    require_args("ensure_trailing", args, 2)?;
    if any_null(args) {
        return Ok(Value::Null);
    }
    let var = &args[0];
    let postfix = &args[1];
    if matches!(var, Value::Bool(_) | Value::Array(_) | Value::Object(_))
        || matches!(postfix, Value::Bool(_) | Value::Array(_) | Value::Object(_))
    {
        return Err("ensure_trailing() arguments must be strings or numbers".to_string());
    }
    let var_str = ensure_operand(var)?;
    let postfix_str = ensure_operand(postfix)?;
    if var_str.ends_with(&postfix_str) {
        return Ok(var.clone());
    }
    let combined = format!("{var_str}{postfix_str}");
    if matches!(var, Value::Number(_))
        && (matches!(postfix, Value::Number(_)) || is_numberlike_string(&postfix_str))
    {
        ensure_numeric_concat(&combined)
    } else {
        Ok(Value::String(combined))
    }
}

/// `terminal(string)` — renders Prose markup to a terminal string with
/// deterministic, non-interactive terminal settings.
pub fn terminal(args: &[Value]) -> Result<Value, String> {
    require_args("terminal", args, 1)?;
    if args[0].is_null() {
        return Ok(Value::Null);
    }
    let s = require_string("terminal", &args[0])?;
    Ok(Value::String(Prose::new(s).render_optimistic(None)))
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

/// English ordinal suffix for a day-of-month (1..=31). 11/12/13 are "th".
fn ordinal_suffix(day: u32) -> &'static str {
    match (day % 100, day % 10) {
        (11..=13, _) => "th",
        (_, 1) => "st",
        (_, 2) => "nd",
        (_, 3) => "rd",
        _ => "th",
    }
}

/// Joins non-empty parts with a single space (used for the optional year).
fn join_nonempty(parts: &[String]) -> String {
    parts
        .iter()
        .filter(|p| !p.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Reformats an ISO date/datetime string into a named human format.
///
/// Supported formats (canonical name plus aliases): `"MMMM Do"`/`short`,
/// `"MMMM Do [YYYY]"`/`short-optional`, `"MMMM Do YYYY"`, `"D MMMM [YYYY]"`,
/// `"D MMMM YYYY"`, and `"ddd, MMMM Do, YYYY"`/`long`. The `[YYYY]` token
/// includes the year only when it differs from the current year.
pub fn date_fn(args: &[Value]) -> Result<Value, String> {
    require_args("date", args, 2)?;
    if any_null(args) {
        return Ok(Value::Null);
    }
    let iso = require_string("date", &args[0])?;
    let fmt = require_string("date", &args[1])?;
    let parsed = parse_date_or_datetime(iso, false)
        .ok_or_else(|| format!("date() invalid ISO date or datetime: {iso:?}"))?;

    let month = parsed.format("%B").to_string(); // "July"
    let dow = parsed.format("%a").to_string(); // "Mon"
    let day = parsed.day();
    let day_ord = format!("{day}{}", ordinal_suffix(day));
    let year = parsed.year();
    let current_year = Local::now().year();
    // Year token honoring the `[YYYY]` optional-year extension.
    let opt_year = if year == current_year {
        String::new()
    } else {
        year.to_string()
    };

    let out = match fmt {
        "MMMM Do" | "short" => format!("{month} {day_ord}"),
        "MMMM Do [YYYY]" | "short-optional" => {
            join_nonempty(&[format!("{month} {day_ord}"), opt_year])
        }
        "MMMM Do YYYY" => format!("{month} {day_ord} {year}"),
        "D MMMM [YYYY]" => join_nonempty(&[format!("{day} {month}"), opt_year]),
        "D MMMM YYYY" => format!("{day} {month} {year}"),
        "ddd, MMMM Do, YYYY" | "long" => format!("{dow}, {month} {day_ord}, {year}"),
        other => return Err(format!("date() unknown format: {other:?}")),
    };
    Ok(Value::String(out))
}

/// `is_date(x)` — strict `YYYY-MM-DD` validator. Strings only.
pub fn is_date(args: &[Value]) -> Result<Value, String> {
    require_args("is_date", args, 1)?;
    let ok = matches!(&args[0], Value::String(s) if parse_iso_date(s).is_some());
    Ok(Value::Bool(ok))
}

/// `is_date_utc(x)` — same as `is_date` (the format itself is timezone-agnostic).
pub fn is_date_utc(args: &[Value]) -> Result<Value, String> {
    require_args("is_date_utc", args, 1)?;
    let ok = matches!(&args[0], Value::String(s) if parse_iso_date(s).is_some());
    Ok(Value::Bool(ok))
}

/// `is_date_time(x)` — strict ISO datetime validator. Strings only.
pub fn is_datetime(args: &[Value]) -> Result<Value, String> {
    require_args("is_date_time", args, 1)?;
    let ok = matches!(&args[0], Value::String(s) if parse_iso_datetime(s));
    Ok(Value::Bool(ok))
}

/// `is_date_time_utc(x)` — same parse contract as `is_date_time`.
pub fn is_datetime_utc(args: &[Value]) -> Result<Value, String> {
    require_args("is_date_time_utc", args, 1)?;
    let ok = matches!(&args[0], Value::String(s) if parse_iso_datetime(s));
    Ok(Value::Bool(ok))
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

fn today_local() -> NaiveDate {
    Local::now().date_naive()
}

fn today_utc() -> NaiveDate {
    Utc::now().date_naive()
}

pub fn is_today(args: &[Value]) -> Result<Value, String> {
    require_args("is_today", args, 1)?;
    Ok(Value::Bool(is_today_with(&args[0], today_local(), false)))
}

pub fn is_today_utc(args: &[Value]) -> Result<Value, String> {
    require_args("is_today_utc", args, 1)?;
    Ok(Value::Bool(is_today_with(&args[0], today_utc(), true)))
}

pub fn is_yesterday(args: &[Value]) -> Result<Value, String> {
    require_args("is_yesterday", args, 1)?;
    Ok(Value::Bool(is_yesterday_with(
        &args[0],
        today_local(),
        false,
    )))
}

pub fn is_yesterday_utc(args: &[Value]) -> Result<Value, String> {
    require_args("is_yesterday_utc", args, 1)?;
    Ok(Value::Bool(is_yesterday_with(&args[0], today_utc(), true)))
}

pub fn is_tomorrow(args: &[Value]) -> Result<Value, String> {
    require_args("is_tomorrow", args, 1)?;
    Ok(Value::Bool(is_tomorrow_with(
        &args[0],
        today_local(),
        false,
    )))
}

pub fn is_tomorrow_utc(args: &[Value]) -> Result<Value, String> {
    require_args("is_tomorrow_utc", args, 1)?;
    Ok(Value::Bool(is_tomorrow_with(&args[0], today_utc(), true)))
}

pub fn is_this_month(args: &[Value]) -> Result<Value, String> {
    require_args("is_this_month", args, 1)?;
    Ok(Value::Bool(is_this_month_with(
        &args[0],
        today_local(),
        false,
    )))
}

pub fn is_this_month_utc(args: &[Value]) -> Result<Value, String> {
    require_args("is_this_month_utc", args, 1)?;
    Ok(Value::Bool(is_this_month_with(&args[0], today_utc(), true)))
}

pub fn is_this_year(args: &[Value]) -> Result<Value, String> {
    require_args("is_this_year", args, 1)?;
    Ok(Value::Bool(is_this_year_with(
        &args[0],
        today_local(),
        false,
    )))
}

pub fn is_this_year_utc(args: &[Value]) -> Result<Value, String> {
    require_args("is_this_year_utc", args, 1)?;
    Ok(Value::Bool(is_this_year_with(&args[0], today_utc(), true)))
}

/// Resolves a filepath argument to an absolute path using FileReference rules.
///
/// Resolution is attempted document-relative first (against `ctx.base_dir`, so
/// references written inside a composed document resolve next to that document),
/// then falls back to ambient process-CWD resolution. The fallback keeps these
/// functions in agreement with the `file`-typed `$schema` validator, which uses
/// `FileReference::resolve()` (process CWD): a caller-supplied path — e.g. a CLI
/// `-y` variable typed relative to the user's working directory — that the
/// schema layer accepts must not read as missing here. The two anchors only
/// differ when the process CWD is not the document's directory.
///
/// ## Returns
///
/// - `Ok(Some(path))` when the reference resolves to a path.
/// - `Ok(None)` when the reference is well-formed but resolves to nothing.
/// - `Err` when the reference string itself is invalid.
fn resolve_arg(raw: &str, ctx: &ResolutionContext) -> Result<Option<PathBuf>, String> {
    let normalized = normalize_path_arg(raw);
    let mut file_ref = biscuit_file::FileReference::new(&normalized)
        .map_err(|e| format!("invalid file path {raw:?}: {e}"))?;
    for (path, position) in &ctx.magic_paths {
        file_ref = file_ref.add_magic_path(path, *position);
    }
    match file_ref
        .resolve_from(&ctx.base_dir)
        .map_err(|e| format!("invalid file path {raw:?}: {e}"))?
    {
        Some(path) => Ok(Some(path)),
        None => file_ref
            .resolve()
            .map_err(|e| format!("invalid file path {raw:?}: {e}")),
    }
}

/// `absolute(file) -> file | Error::InvalidFilePath`
pub fn absolute_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, String> {
    require_args("absolute", args, 1)?;
    if any_null(args) {
        return Ok(Value::Null);
    }
    let raw = require_string("absolute", &args[0])?;
    match resolve_arg(raw, ctx)? {
        Some(p) => Ok(Value::String(p.to_string_lossy().to_string())),
        None => Err(format!("absolute() invalid file path: {raw:?}")),
    }
}

/// `file_exists(file) -> bool` — invalid local paths return `false`, never
/// error. A remote URL argument errors when the resolution context is
/// local-only (no remote runtime); see the URL branch below.
pub fn file_exists_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, String> {
    require_args("file_exists", args, 1)?;
    if any_null(args) {
        return Ok(Value::Bool(false));
    }
    let raw = match require_string("file_exists", &args[0]) {
        Ok(s) => s,
        Err(_) => return Ok(Value::Bool(false)),
    };
    // A remote URL "exists" when it was fetched successfully. With a remote
    // runtime attached (body/post-shell), a denied or failed fetch reads as
    // non-existent (never errors). With **no** runtime — the local-only
    // frontmatter context — a URL is unreadable here, so fail loudly rather
    // than silently reporting it as absent (Decision B).
    if is_remote_url(raw) {
        return match ctx.fetch_remote_text(raw) {
            Ok(Some(_)) => Ok(Value::Bool(true)),
            Ok(None) => Err(format!(
                "file_exists() cannot read remote URL {raw:?}: this resolution \
                 context is local-only (no remote runtime)"
            )),
            Err(_) => Ok(Value::Bool(false)),
        };
    }
    let exists = match resolve_arg(raw, ctx) {
        Ok(Some(p)) => p.exists(),
        _ => false,
    };
    Ok(Value::Bool(exists))
}

/// Best-effort relative rendering: repo-root relative when inside the repo,
/// else base_dir-relative, else `~`-aliased home path, else the absolute path.
fn make_relative(abs: &Path, base_dir: &Path) -> String {
    if let Some(repo) = crate::markdown::compose::find_git_root_from(base_dir)
        && let Ok(stripped) = abs.strip_prefix(&repo)
    {
        return stripped.to_string_lossy().to_string();
    }
    if let Ok(stripped) = abs.strip_prefix(base_dir) {
        return stripped.to_string_lossy().to_string();
    }
    if let Some(home) = dirs::home_dir()
        && let Ok(stripped) = abs.strip_prefix(&home)
    {
        return format!("~/{}", stripped.to_string_lossy());
    }
    abs.to_string_lossy().to_string()
}

/// `relative(file) -> file | Error::InvalidFilePath`
pub fn relative_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, String> {
    require_args("relative", args, 1)?;
    if any_null(args) {
        return Ok(Value::Null);
    }
    let raw = require_string("relative", &args[0])?;
    let abs = match resolve_arg(raw, ctx)? {
        Some(p) => p,
        None => return Err(format!("relative() invalid file path: {raw:?}")),
    };
    Ok(Value::String(make_relative(&abs, &ctx.base_dir)))
}

/// Resolves a filepath argument through the shared FS-path rules.
///
/// Rejects HTTP(S) URLs. First tries the standard `FileReference` resolution
/// (which requires the file to exist so magic paths and git-root fallbacks
/// work); if that yields no match, falls back to a path shape computed from
/// the reference kind and the base directory. This lets path-component
/// functions operate on missing files and directories without checking
/// `Path::exists()`.
fn resolve_path_arg(name: &str, value: &Value, ctx: &ResolutionContext) -> Result<PathBuf, String> {
    let raw = require_string(name, value)?;
    resolve_path_shape(name, raw, ctx)
}

/// Resolves a raw path string to an absolute path shape.
///
/// See [`resolve_path_arg`]. Exposed separately so `join()` can validate its
/// computed result without constructing a temporary [`Value`].
fn resolve_path_shape(name: &str, raw: &str, ctx: &ResolutionContext) -> Result<PathBuf, String> {
    if is_remote_url(raw) {
        return Err(format!("{name}() does not accept HTTP(S) URLs"));
    }
    // Existing `FileReference` resolution handles existing files, magic paths,
    // package paths, git-root fallbacks, and absolute references.
    if let Ok(Some(p)) = resolve_arg(raw, ctx) {
        return Ok(p);
    }
    // No existing match: build a deterministic path shape without touching
    // `Path::exists()`.
    let normalized = normalize_path_arg(raw);
    let path = PathBuf::from(&normalized);
    if path.is_absolute() {
        return Ok(path);
    }
    if normalized.starts_with("./")
        || normalized.starts_with("../")
        || normalized == "."
        || normalized == ".."
    {
        return Ok(ctx.base_dir.join(path));
    }
    if let Some(rest) = normalized.strip_prefix('@') {
        for (magic, _position) in &ctx.magic_paths {
            let candidate = magic.join(rest);
            if candidate.exists() {
                return Ok(candidate);
            }
        }
        return Ok(ctx.base_dir.join(rest));
    }
    if let Some(rest) = normalized.strip_prefix('!') {
        return Ok(ctx.base_dir.join(rest));
    }
    if normalized.starts_with("vault:") {
        return Err(format!("{name}() vault references require an existing file"));
    }
    Ok(ctx.base_dir.join(path))
}

/// Parses a stem against the indexed grammar, returning the base name, parsed
/// index, and the original zero-padding width of the index.
fn indexed_stem_info(stem: &str) -> Option<(String, u64, usize)> {
    let last_hyphen = stem.rfind('-')?;
    if last_hyphen == 0 {
        return None;
    }
    if stem.as_bytes().get(last_hyphen - 1) == Some(&b'-') {
        return None;
    }
    let base = &stem[..last_hyphen];
    let digits = &stem[last_hyphen + 1..];
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let width = digits.len();
    let index = digits.parse::<u64>().ok()?;
    Some((base.to_string(), index, width))
}

/// Formats an indexed stem, preserving the original zero-padding width.
fn format_indexed_stem(base: &str, index: u64, width: usize) -> String {
    format!("{base}-{index:0>width$}")
}

/// Splits a resolved path into its display directory components and basename.
///
/// The display shape follows the same `relative(file)` policy used elsewhere:
/// repo-root relative, base-dir relative, `~`-aliased, or absolute. Components
/// are returned with `/` separators in mind.
fn path_display_components(path: &Path, base_dir: &Path) -> (Vec<String>, String) {
    let rel = make_relative(path, base_dir).replace('\\', "/");
    let trimmed = rel.strip_prefix('/').unwrap_or(&rel).to_string();
    match trimmed.rfind('/') {
        Some(pos) => {
            let dir_part = &trimmed[..pos];
            let dirs = if dir_part.is_empty() {
                Vec::new()
            } else {
                dir_part.split('/').map(|s| s.to_string()).collect()
            };
            (dirs, trimmed[pos + 1..].to_string())
        }
        None => (Vec::new(), trimmed),
    }
}

/// `is_indexed_file(file) -> bool` — true when the filename stem matches the
/// indexed grammar (`base-NNN`).
pub fn is_indexed_file_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, String> {
    require_args("is_indexed_file", args, 1)?;
    if any_null(args) {
        return Ok(Value::Null);
    }
    let path = resolve_path_arg("is_indexed_file", &args[0], ctx)?;
    let (_, base) = path_display_components(&path, &ctx.base_dir);
    let stem = file_stem(&base);
    Ok(Value::Bool(parse_indexed_stem(&stem).is_some()))
}

/// `file_index(file) -> number` — the parsed index, or `-1` when non-indexed.
pub fn file_index_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, String> {
    require_args("file_index", args, 1)?;
    if any_null(args) {
        return Ok(Value::Null);
    }
    let path = resolve_path_arg("file_index", &args[0], ctx)?;
    let (_, base) = path_display_components(&path, &ctx.base_dir);
    let stem = file_stem(&base);
    let index = parse_indexed_stem(&stem)
        .map(|i| i.index as i64)
        .unwrap_or(-1);
    Ok(Value::Number(index.into()))
}

/// `increment_file_index(file) -> string` — bumps the numeric suffix, starting
/// at `2` for non-indexed files. Preserves zero-padding width for indexed stems.
pub fn increment_file_index_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, String> {
    require_args("increment_file_index", args, 1)?;
    if any_null(args) {
        return Ok(Value::Null);
    }
    let path = resolve_path_arg("increment_file_index", &args[0], ctx)?;
    let (_, base) = path_display_components(&path, &ctx.base_dir);
    let ext = file_extension(&base);
    let stem = file_stem(&base);
    let new_stem = if let Some((base_name, index, width)) = indexed_stem_info(&stem) {
        let next = index.saturating_add(1);
        format_indexed_stem(&base_name, next, width)
    } else {
        format!("{stem}-2")
    };
    let new_base = if ext.is_empty() {
        new_stem
    } else {
        format!("{new_stem}.{ext}")
    };
    let out = path
        .parent()
        .map(|p| p.join(&new_base))
        .unwrap_or_else(|| PathBuf::from(&new_base));
    Ok(Value::String(make_relative(&out, &ctx.base_dir).replace('\\', "/")))
}

/// `decrement_file_index(file) -> string` — decrements the numeric suffix,
/// clamped at `0`. Non-indexed files start at `0` and preserve no padding.
pub fn decrement_file_index_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, String> {
    require_args("decrement_file_index", args, 1)?;
    if any_null(args) {
        return Ok(Value::Null);
    }
    let path = resolve_path_arg("decrement_file_index", &args[0], ctx)?;
    let (_, base) = path_display_components(&path, &ctx.base_dir);
    let ext = file_extension(&base);
    let stem = file_stem(&base);
    let new_stem = if let Some((base_name, index, width)) = indexed_stem_info(&stem) {
        let next = index.saturating_sub(1);
        format_indexed_stem(&base_name, next, width)
    } else {
        format!("{stem}-0")
    };
    let new_base = if ext.is_empty() {
        new_stem
    } else {
        format!("{new_stem}.{ext}")
    };
    let out = path
        .parent()
        .map(|p| p.join(&new_base))
        .unwrap_or_else(|| PathBuf::from(&new_base));
    Ok(Value::String(make_relative(&out, &ctx.base_dir).replace('\\', "/")))
}

/// `basename(file) -> string` — the final path component including extension.
pub fn basename_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, String> {
    require_args("basename", args, 1)?;
    if any_null(args) {
        return Ok(Value::Null);
    }
    let path = resolve_path_arg("basename", &args[0], ctx)?;
    let (_, base) = path_display_components(&path, &ctx.base_dir);
    Ok(Value::String(base))
}

/// `basename_without_index(file) -> string` — removes an indexed suffix from
/// the stem. Non-indexed basenames pass through unchanged.
pub fn basename_without_index_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, String> {
    require_args("basename_without_index", args, 1)?;
    if any_null(args) {
        return Ok(Value::Null);
    }
    let path = resolve_path_arg("basename_without_index", &args[0], ctx)?;
    let (_, base) = path_display_components(&path, &ctx.base_dir);
    let stem = file_stem(&base);
    let ext = file_extension(&base);
    let unindexed = match indexed_stem_info(&stem) {
        Some((base_name, _, _)) => base_name,
        None => stem,
    };
    let out = if ext.is_empty() {
        unindexed
    } else {
        format!("{unindexed}.{ext}")
    };
    Ok(Value::String(out))
}

/// `dir(file) -> string` — the directory portion of the display path.
pub fn dir_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, String> {
    require_args("dir", args, 1)?;
    if any_null(args) {
        return Ok(Value::Null);
    }
    let path = resolve_path_arg("dir", &args[0], ctx)?;
    let (dirs, _) = path_display_components(&path, &ctx.base_dir);
    Ok(Value::String(if dirs.is_empty() {
        String::new()
    } else {
        dirs.join("/")
    }))
}

/// `ext(file) -> string` — the final extension without the leading dot, or an
/// empty string when there is none.
pub fn ext_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, String> {
    require_args("ext", args, 1)?;
    if any_null(args) {
        return Ok(Value::Null);
    }
    let path = resolve_path_arg("ext", &args[0], ctx)?;
    let (_, base) = path_display_components(&path, &ctx.base_dir);
    Ok(Value::String(file_extension(&base)))
}

/// `parent_dir(file) -> string` — the directory segment immediately above the
/// basename, or an empty string when there is none.
pub fn parent_dir_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, String> {
    require_args("parent_dir", args, 1)?;
    if any_null(args) {
        return Ok(Value::Null);
    }
    let path = resolve_path_arg("parent_dir", &args[0], ctx)?;
    let (dirs, _) = path_display_components(&path, &ctx.base_dir);
    Ok(Value::String(dirs.last().cloned().unwrap_or_default()))
}

/// `file_trailing(file) -> string` — the last directory segment plus the
/// basename, or just the basename when there is no directory.
pub fn file_trailing_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, String> {
    require_args("file_trailing", args, 1)?;
    if any_null(args) {
        return Ok(Value::Null);
    }
    let path = resolve_path_arg("file_trailing", &args[0], ctx)?;
    let (dirs, base) = path_display_components(&path, &ctx.base_dir);
    Ok(Value::String(match dirs.last() {
        Some(d) => format!("{d}/{base}"),
        None => base,
    }))
}

/// `dir_leading(file) -> string` — the directory path before the last segment,
/// or an empty string when there is no leading directory.
pub fn dir_leading_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, String> {
    require_args("dir_leading", args, 1)?;
    if any_null(args) {
        return Ok(Value::Null);
    }
    let path = resolve_path_arg("dir_leading", &args[0], ctx)?;
    let (dirs, _) = path_display_components(&path, &ctx.base_dir);
    Ok(Value::String(if dirs.len() <= 1 {
        String::new()
    } else {
        dirs[..dirs.len() - 1].join("/")
    }))
}

/// `join(left, right) -> string` — joins two path strings, normalizing leading
/// and duplicate separators. Validates the result through the shared FS-path
/// rules and rejects HTTP(S) arguments.
pub fn join_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, String> {
    require_args("join", args, 2)?;
    if any_null(args) {
        return Ok(Value::Null);
    }
    let left_raw = require_string("join", &args[0])?;
    let right_raw = require_string("join", &args[1])?;
    if is_remote_url(left_raw) || is_remote_url(right_raw) {
        return Err("join() does not accept HTTP(S) URLs".to_string());
    }
    let left = resolve_path_arg("join", &args[0], ctx)?;
    let right = right_raw.trim_start_matches(['/', '\\']);
    let joined = left.join(right);
    let joined_str = joined.to_string_lossy().to_string();
    let validated = resolve_path_shape("join", &joined_str, ctx)?;
    Ok(Value::String(
        make_relative(&validated, &ctx.base_dir).replace('\\', "/"),
    ))
}

/// Escapes `[` and `]` in Markdown link text with backslashes.
fn escape_link_text(text: &str) -> String {
    text.replace('[', "\\[").replace(']', "\\]")
}

/// Returns `true` when a link destination needs angle-bracket wrapping to stay
/// CommonMark-safe.
fn destination_needs_wrapping(dest: &str) -> bool {
    dest.chars().any(|c| {
        c.is_ascii_control()
            || c == ' '
            || c == '\t'
            || c == '('
            || c == ')'
            || c == '<'
            || c == '>'
    })
}

/// Formats a Markdown inline link `[text](destination)` with text and
/// destination escaping.
fn format_markdown_link(text: &str, destination: &str) -> String {
    let escaped_text = escape_link_text(text);
    let safe_dest = if destination_needs_wrapping(destination) {
        let inner = destination.replace('<', "\\<").replace('>', "\\>");
        format!("<{inner}>")
    } else {
        destination.to_string()
    };
    format!("[{escaped_text}]({safe_dest})")
}

/// `link(file)` / `link(target, desc)` — emits a Markdown inline link.
///
/// - One argument: resolves a local file and uses `relative(file)` as the link
///   text and the resolved absolute path as the destination. HTTP(S) URLs are
///   rejected because a description is required.
/// - Two arguments: `target` may be a local file reference or an HTTP(S) URL;
///   `desc` must be a string and is used as the link text.
pub fn link_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, String> {
    match args.len() {
        1 => {
            if args[0].is_null() {
                return Ok(Value::Null);
            }
            let raw = require_string("link", &args[0])?;
            if is_remote_url(raw) {
                return Err(
                    "link() one-argument form does not accept HTTP(S) URLs; use link(target, desc)"
                        .to_string(),
                );
            }
            let path = resolve_path_arg("link", &args[0], ctx)?;
            let desc = make_relative(&path, &ctx.base_dir).replace('\\', "/");
            let dest = path.to_string_lossy().replace('\\', "/");
            Ok(Value::String(format_markdown_link(&desc, &dest)))
        }
        2 => {
            if any_null(args) {
                return Ok(Value::Null);
            }
            let target_raw = require_string("link", &args[0])?;
            let desc = require_string("link", &args[1])?;
            let dest = if is_remote_url(target_raw) {
                url::Url::parse(target_raw)
                    .map_err(|e| format!("link() invalid URL {target_raw:?}: {e}"))?;
                target_raw.to_string()
            } else {
                let path = resolve_path_arg("link", &args[0], ctx)?;
                path.to_string_lossy().replace('\\', "/")
            };
            Ok(Value::String(format_markdown_link(desc, &dest)))
        }
        _ => Err("link() requires 1 or 2 arguments".to_string()),
    }
}

/// Validates that a skill name is a single basename component.
fn validate_skill_name(name: &str) -> bool {
    let path = Path::new(name);
    let mut components = path.components();
    matches!(components.next(), Some(std::path::Component::Normal(_)))
        && components.next().is_none()
}

/// Checks whether a direct child directory named `name` exists under any root.
fn skill_exists_in_roots(roots: &[PathBuf], name: &str) -> bool {
    roots.iter().any(|root| root.join(name).is_dir())
}

/// `has_skill(name)` — `true` when a direct child directory named `name` exists
/// in any user-scoped or local-scoped skill root for the executing agent.
pub fn has_skill_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, String> {
    require_args("has_skill", args, 1)?;
    if args[0].is_null() {
        return Ok(Value::Null);
    }
    let name = require_string("has_skill", &args[0])?;
    if !validate_skill_name(name) {
        return Err(
            "has_skill() skill name must be a basename without path separators".to_string(),
        );
    }
    let agent = ctx.agent();
    let home_dir = ctx.home_dir().unwrap_or_else(|| PathBuf::from("."));
    let local_root = crate::markdown::compose::find_git_root_from(&ctx.base_dir)
        .unwrap_or_else(|| ctx.base_dir.clone());
    let roots = SkillRoots::new(home_dir, local_root).roots_for_agent(&agent);
    Ok(Value::Bool(skill_exists_in_roots(&roots, name)))
}

/// `has_local_skill(name)` — `true` when a direct child directory named `name`
/// exists in any local-scoped skill root for the executing agent.
pub fn has_local_skill_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, String> {
    require_args("has_local_skill", args, 1)?;
    if args[0].is_null() {
        return Ok(Value::Null);
    }
    let name = require_string("has_local_skill", &args[0])?;
    if !validate_skill_name(name) {
        return Err(
            "has_local_skill() skill name must be a basename without path separators"
                .to_string(),
        );
    }
    let agent = ctx.agent();
    let local_root = crate::markdown::compose::find_git_root_from(&ctx.base_dir)
        .unwrap_or_else(|| ctx.base_dir.clone());
    let roots = SkillRoots::new(PathBuf::from("."), local_root).local_roots_for_agent(&agent);
    Ok(Value::Bool(skill_exists_in_roots(&roots, name)))
}

/// Loads a Markdown file via the resolution context. `Err` if the path is
/// invalid or unreadable.
fn load_markdown(raw: &str, ctx: &ResolutionContext, fname: &str) -> Result<Markdown, String> {
    // HTTP(S) arguments read from the run's remote-fetch cache instead of disk,
    // mirroring how `::file`/`::code` directives resolve remote targets.
    if is_remote_url(raw) {
        return match ctx.fetch_remote_text(raw)? {
            Some(body) => Markdown::try_from_content(body)
                .map_err(|e| format!("{fname}() failed to parse {raw:?}: {e}")),
            None => Err(format!("{fname}() remote reads are not enabled for {raw:?}")),
        };
    }
    let path = resolve_arg(raw, ctx)?.ok_or_else(|| format!("{fname}() invalid file path: {raw:?}"))?;
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("{fname}() invalid file path {raw:?}: {e}"))?;
    Markdown::try_from_content(content).map_err(|e| format!("{fname}() failed to parse {raw:?}: {e}"))
}

/// `frontmatter(file)` → object; `frontmatter(file, prop)` → value | null.
pub fn frontmatter_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, String> {
    if args.is_empty() || args.len() > 2 {
        return Err("frontmatter() requires 1 or 2 arguments".to_string());
    }
    if matches!(args.first(), Some(Value::Null)) {
        return Ok(Value::Null);
    }
    let raw = require_string("frontmatter", &args[0])?;
    let md = load_markdown(raw, ctx, "frontmatter")?;
    let map = md.frontmatter().as_map();
    if args.len() == 1 {
        let obj: serde_json::Map<String, Value> =
            map.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        return Ok(Value::Object(obj));
    }
    let prop = require_string("frontmatter", &args[1])?;
    Ok(map.get(prop).cloned().unwrap_or(Value::Null))
}

/// `markdown_body_empty(file) -> bool | Error` — body has only whitespace.
pub fn markdown_body_empty_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, String> {
    require_args("markdown_body_empty", args, 1)?;
    if any_null(args) {
        return Ok(Value::Null);
    }
    let raw = require_string("markdown_body_empty", &args[0])?;
    let md = load_markdown(raw, ctx, "markdown_body_empty")?;
    Ok(Value::Bool(md.content().trim().is_empty()))
}

/// `markdown_title(file) -> string | null | Error` — frontmatter `title`,
/// else first H1. Multiple H1s: first wins, warning to STDERR.
pub fn markdown_title_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, String> {
    require_args("markdown_title", args, 1)?;
    if any_null(args) {
        return Ok(Value::Null);
    }
    let raw = require_string("markdown_title", &args[0])?;
    let md = load_markdown(raw, ctx, "markdown_title")?;
    if let Some(t) = md.frontmatter().as_map().get("title").and_then(Value::as_str) {
        return Ok(Value::String(t.to_string()));
    }
    let h1s: Vec<String> = md
        .content()
        .lines()
        .filter_map(|l| l.strip_prefix("# ").map(|t| t.trim().to_string()))
        .collect();
    match h1s.as_slice() {
        [] => Ok(Value::Null),
        [single] => Ok(Value::String(single.clone())),
        [first, ..] => {
            eprintln!("markdown_title(): multiple H1 headings in {raw:?}; using the first");
            Ok(Value::String(first.clone()))
        }
    }
}

/// `validate_schema(file)` / `validate_schema(file, obj)` -> bool | Error.
/// Returns `true` when the document declares no `$schema`.
///
/// ## Notes
///
/// The two-argument form is accepted but currently validates the referenced
/// document itself rather than the supplied object; validating an arbitrary
/// object against the document's schema requires a dedicated
/// [`DarkmatterSchemas`] entry point that does not yet exist.
pub fn validate_schema_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, String> {
    if args.is_empty() || args.len() > 2 {
        return Err("validate_schema() requires 1 or 2 arguments".to_string());
    }
    if matches!(args.first(), Some(Value::Null)) {
        return Ok(Value::Null);
    }
    let raw = require_string("validate_schema", &args[0])?;
    let md = load_markdown(raw, ctx, "validate_schema")?;
    // No `$schema` → always valid (per spec).
    if !md.frontmatter().as_map().contains_key("$schema") {
        return Ok(Value::Bool(true));
    }
    let schemas = DarkmatterSchemas::new();
    let report = schemas
        .validate(&md)
        .map_err(|e| format!("validate_schema() error for {raw:?}: {e}"))?;
    Ok(Value::Bool(report.valid))
}

/// `has_key(obj, key)` → bool. Eagerly-evaluated form of the core operator.
pub fn has_key_fn(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("has_key() requires 2 arguments".to_string());
    }
    let key = scalar_string(&args[1]);
    let has = args[0]
        .as_object()
        .map(|obj| obj.contains_key(&key))
        .unwrap_or(false);
    Ok(Value::Bool(has))
}

/// `contains(haystack, needle)` → bool over arrays, objects, or strings.
pub fn contains_fn(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("contains() requires 2 arguments".to_string());
    }
    let needle = &args[1];
    let found = match &args[0] {
        Value::Array(values) => values
            .iter()
            .any(|value| scalar_string(value) == scalar_string(needle)),
        Value::Object(values) => values
            .values()
            .any(|value| scalar_string(value) == scalar_string(needle)),
        Value::String(value) => value.contains(&scalar_string(needle)),
        value => scalar_string(value).contains(&scalar_string(needle)),
    };
    Ok(Value::Bool(found))
}

/// `length(x)` → count of chars / array items / object keys.
pub fn length_fn(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("length() requires 1 argument".to_string());
    }
    let len = match &args[0] {
        Value::String(s) => s.chars().count(),
        Value::Array(arr) => arr.len(),
        Value::Object(obj) => obj.len(),
        Value::Number(n) => n.to_string().chars().count(),
        Value::Bool(_) | Value::Null => 0,
    };
    Ok(Value::Number(serde_json::Number::from(len)))
}

/// `number(x, [default])` → numeric conversion with optional fallback.
pub fn number_fn(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("number() requires at least 1 argument".to_string());
    }
    let default = if args.len() > 1 {
        to_number_coerce(&args[1])
    } else {
        0.0
    };
    let number = to_number(&args[0]).unwrap_or(default);
    let json = if number.fract() == 0.0 {
        serde_json::Number::from(number as i64)
    } else {
        serde_json::Number::from_f64(number)
            .ok_or_else(|| "Unable to represent number".to_string())?
    };
    Ok(Value::Number(json))
}

/// `round(x, [default])` → nearest integer with optional fallback.
pub fn round_fn(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("round() requires at least 1 argument".to_string());
    }
    let default = if args.len() > 1 {
        to_number_coerce(&args[1])
    } else {
        0.0
    };
    let number = to_number(&args[0]).unwrap_or(default).round() as i64;
    Ok(Value::Number(serde_json::Number::from(number)))
}

/// Signature of a pure expression function (fully-evaluated `Value` args).
pub type PureFn = fn(&[Value]) -> Result<Value, String>;
/// Signature of a context-aware (filesystem/document) expression function.
pub type FsFn = fn(&[Value], &ResolutionContext) -> Result<Value, String>;

/// One pure (context-free) expression function registration.
pub struct PureFunction {
    /// Canonical snake_case name (matches the descriptor signature).
    pub canonical: &'static str,
    /// Accepted lowercased aliases (e.g. the underscore-free spelling).
    pub aliases: &'static [&'static str],
    /// Every canonical signature this function answers to, including each
    /// overload and optional/variadic arity (e.g. `number(x, [default])`). This
    /// is the authoritative set of callable signatures the descriptor catalog
    /// is checked against, so adding or removing an overload here (or in a
    /// descriptor) is detected by `descriptor_signature_set_equals_dispatchable_signature_set`.
    pub signatures: &'static [&'static str],
    /// Handler invoked with fully-evaluated arguments.
    pub handler: PureFn,
}

/// One context-aware expression function registration.
pub struct FsFunction {
    /// Canonical snake_case name (matches the descriptor signature).
    pub canonical: &'static str,
    /// Accepted lowercased aliases (e.g. the underscore-free spelling).
    pub aliases: &'static [&'static str],
    /// Every canonical signature this function answers to, including overloads
    /// (e.g. `frontmatter(file)` and `frontmatter(file, prop)`). See
    /// [`PureFunction::signatures`].
    pub signatures: &'static [&'static str],
    /// Handler invoked with fully-evaluated arguments and the resolution context.
    pub handler: FsFn,
}

/// The authoritative table of pure expression functions.
///
/// [`dispatch`] resolves names against this slice, so it is the single source
/// of truth for which pure functions the evaluator recognizes — not a list
/// maintained beside the dispatcher. The descriptor catalog must match it
/// (enforced by `descriptor_name_set_equals_dispatchable_runtime_name_set`).
pub const PURE_FUNCTIONS: &[PureFunction] = &[
    // Type predicates
    PureFunction { canonical: "is_string", aliases: &["isstring"], signatures: &["is_string(x)"], handler: is_string },
    PureFunction { canonical: "is_number", aliases: &["isnumber"], signatures: &["is_number(x)"], handler: is_number },
    PureFunction { canonical: "is_array", aliases: &["isarray"], signatures: &["is_array(x)"], handler: is_array },
    PureFunction { canonical: "is_null", aliases: &["isnull"], signatures: &["is_null(x)"], handler: is_null },
    PureFunction { canonical: "is_object", aliases: &["isobject"], signatures: &["is_object(x)"], handler: is_object },
    PureFunction { canonical: "is_empty", aliases: &["isempty"], signatures: &["is_empty(x)"], handler: is_empty_fn },
    PureFunction { canonical: "is_positive", aliases: &["ispositive"], signatures: &["is_positive(val)"], handler: is_positive },
    PureFunction { canonical: "is_negative", aliases: &["isnegative"], signatures: &["is_negative(val)"], handler: is_negative },
    PureFunction { canonical: "is_integer", aliases: &["isinteger"], signatures: &["is_integer(val)"], handler: is_integer },
    // Math helpers
    PureFunction { canonical: "min", aliases: &[], signatures: &["min(a, b)"], handler: min_fn },
    PureFunction { canonical: "max", aliases: &[], signatures: &["max(a, b)"], handler: max_fn },
    PureFunction { canonical: "abs", aliases: &[], signatures: &["abs(x)"], handler: abs_fn },
    PureFunction { canonical: "round", aliases: &[], signatures: &["round(x, [default])"], handler: round_fn },
    // Collection helpers
    PureFunction { canonical: "first", aliases: &[], signatures: &["first(x)"], handler: first_fn },
    PureFunction { canonical: "last", aliases: &[], signatures: &["last(x)"], handler: last_fn },
    PureFunction { canonical: "has_key", aliases: &["haskey"], signatures: &["has_key(obj, key)"], handler: has_key_fn },
    PureFunction { canonical: "contains", aliases: &[], signatures: &["contains(haystack, needle)"], handler: contains_fn },
    PureFunction { canonical: "length", aliases: &[], signatures: &["length(x)"], handler: length_fn },
    // Type conversion
    PureFunction { canonical: "number", aliases: &[], signatures: &["number(x, [default])"], handler: number_fn },
    // String predicates
    PureFunction { canonical: "starts_with", aliases: &["startswith"], signatures: &["starts_with(x, find)"], handler: starts_with },
    PureFunction { canonical: "ends_with", aliases: &["endswith"], signatures: &["ends_with(x, find)"], handler: ends_with },
    // String mutations
    PureFunction { canonical: "lower", aliases: &[], signatures: &["lower(x)"], handler: lower },
    PureFunction { canonical: "upper", aliases: &[], signatures: &["upper(x)"], handler: upper },
    PureFunction { canonical: "capitalize", aliases: &[], signatures: &["capitalize(x)"], handler: capitalize },
    PureFunction { canonical: "kebab_case", aliases: &["kebabcase"], signatures: &["kebab_case(x)"], handler: kebab_case },
    PureFunction { canonical: "snake_case", aliases: &["snakecase"], signatures: &["snake_case(x)"], handler: snake_case },
    PureFunction { canonical: "camel_case", aliases: &["camelcase"], signatures: &["camel_case(x)"], handler: camel_case },
    PureFunction { canonical: "pascal_case", aliases: &["pascalcase"], signatures: &["pascal_case(x)"], handler: pascal_case },
    PureFunction { canonical: "title_case", aliases: &["titlecase"], signatures: &["title_case(x)"], handler: title_case },
    PureFunction { canonical: "without_date", aliases: &["withoutdate"], signatures: &["without_date(string)"], handler: without_date },
    PureFunction { canonical: "ensure_leading", aliases: &["ensureleading"], signatures: &["ensure_leading(var, prefix)"], handler: ensure_leading },
    PureFunction { canonical: "ensure_trailing", aliases: &["ensuretrailing"], signatures: &["ensure_trailing(var, postfix)"], handler: ensure_trailing },
    // Rendering
    PureFunction { canonical: "terminal", aliases: &["terminal"], signatures: &["terminal(string)"], handler: terminal },
    // Date formatting
    PureFunction { canonical: "date", aliases: &[], signatures: &["date(iso, fmt)"], handler: date_fn },
    // Strict date validators
    PureFunction { canonical: "is_date", aliases: &["isdate"], signatures: &["is_date(x)"], handler: is_date },
    PureFunction { canonical: "is_date_utc", aliases: &["isdateutc"], signatures: &["is_date_utc(x)"], handler: is_date_utc },
    PureFunction {
        canonical: "is_date_time",
        aliases: &["isdatetime", "is_datetime"],
        signatures: &["is_date_time(x)"],
        handler: is_datetime,
    },
    PureFunction {
        canonical: "is_date_time_utc",
        aliases: &["isdatetimeutc", "is_datetime_utc"],
        signatures: &["is_date_time_utc(x)"],
        handler: is_datetime_utc,
    },
    // Relative date validators
    PureFunction { canonical: "is_today", aliases: &["istoday"], signatures: &["is_today(x)"], handler: is_today },
    PureFunction { canonical: "is_today_utc", aliases: &["istodayutc"], signatures: &["is_today_utc(x)"], handler: is_today_utc },
    PureFunction { canonical: "is_yesterday", aliases: &["isyesterday"], signatures: &["is_yesterday(x)"], handler: is_yesterday },
    PureFunction {
        canonical: "is_yesterday_utc",
        aliases: &["isyesterdayutc"],
        signatures: &["is_yesterday_utc(x)"],
        handler: is_yesterday_utc,
    },
    PureFunction { canonical: "is_tomorrow", aliases: &["istomorrow"], signatures: &["is_tomorrow(x)"], handler: is_tomorrow },
    PureFunction {
        canonical: "is_tomorrow_utc",
        aliases: &["istomorrowutc"],
        signatures: &["is_tomorrow_utc(x)"],
        handler: is_tomorrow_utc,
    },
    PureFunction { canonical: "is_this_month", aliases: &["isthismonth"], signatures: &["is_this_month(x)"], handler: is_this_month },
    PureFunction {
        canonical: "is_this_month_utc",
        aliases: &["isthismonthutc"],
        signatures: &["is_this_month_utc(x)"],
        handler: is_this_month_utc,
    },
    PureFunction { canonical: "is_this_year", aliases: &["isthisyear"], signatures: &["is_this_year(x)"], handler: is_this_year },
    PureFunction {
        canonical: "is_this_year_utc",
        aliases: &["isthisyearutc"],
        signatures: &["is_this_year_utc(x)"],
        handler: is_this_year_utc,
    },
];

/// The authoritative table of context-aware (filesystem/document) functions.
///
/// [`dispatch_fs`] resolves names against this slice, making it the single
/// source of truth for the fs surface.
pub const FS_FUNCTIONS: &[FsFunction] = &[
    FsFunction { canonical: "absolute", aliases: &[], signatures: &["absolute(file)"], handler: absolute_fn },
    FsFunction { canonical: "relative", aliases: &[], signatures: &["relative(file)"], handler: relative_fn },
    FsFunction { canonical: "file_exists", aliases: &["fileexists"], signatures: &["file_exists(file)"], handler: file_exists_fn },
    FsFunction { canonical: "is_indexed_file", aliases: &["isindexedfile"], signatures: &["is_indexed_file(file)"], handler: is_indexed_file_fn },
    FsFunction { canonical: "file_index", aliases: &["fileindex"], signatures: &["file_index(file)"], handler: file_index_fn },
    FsFunction { canonical: "increment_file_index", aliases: &["incrementfileindex"], signatures: &["increment_file_index(file)"], handler: increment_file_index_fn },
    FsFunction { canonical: "decrement_file_index", aliases: &["decrementfileindex"], signatures: &["decrement_file_index(file)"], handler: decrement_file_index_fn },
    FsFunction { canonical: "basename", aliases: &[], signatures: &["basename(file)"], handler: basename_fn },
    FsFunction { canonical: "basename_without_index", aliases: &["basenamewithoutindex"], signatures: &["basename_without_index(file)"], handler: basename_without_index_fn },
    FsFunction { canonical: "dir", aliases: &[], signatures: &["dir(file)"], handler: dir_fn },
    FsFunction { canonical: "ext", aliases: &[], signatures: &["ext(file)"], handler: ext_fn },
    FsFunction { canonical: "parent_dir", aliases: &["parentdir"], signatures: &["parent_dir(file)"], handler: parent_dir_fn },
    FsFunction { canonical: "file_trailing", aliases: &["filetrailing"], signatures: &["file_trailing(file)"], handler: file_trailing_fn },
    FsFunction { canonical: "dir_leading", aliases: &["dirleading"], signatures: &["dir_leading(file)"], handler: dir_leading_fn },
    FsFunction { canonical: "join", aliases: &[], signatures: &["join(left, right)"], handler: join_fn },
    FsFunction { canonical: "link", aliases: &[], signatures: &["link(file)", "link(target, desc)"], handler: link_fn },
    FsFunction { canonical: "has_skill", aliases: &["hasskill"], signatures: &["has_skill(name)"], handler: has_skill_fn },
    FsFunction { canonical: "has_local_skill", aliases: &["haslocalskill"], signatures: &["has_local_skill(name)"], handler: has_local_skill_fn },
    FsFunction {
        canonical: "frontmatter",
        aliases: &[],
        signatures: &["frontmatter(file)", "frontmatter(file, prop)"],
        handler: frontmatter_fn,
    },
    FsFunction {
        canonical: "markdown_body_empty",
        aliases: &["markdownbodyempty"],
        signatures: &["markdown_body_empty(file)"],
        handler: markdown_body_empty_fn,
    },
    FsFunction {
        canonical: "markdown_title",
        aliases: &["markdowntitle"],
        signatures: &["markdown_title(file)"],
        handler: markdown_title_fn,
    },
    FsFunction {
        canonical: "validate_schema",
        aliases: &["validateschema"],
        signatures: &["validate_schema(file)", "validate_schema(file, obj)"],
        handler: validate_schema_fn,
    },
];

/// Logical short-circuit operators handled directly in `evaluate_function`
/// (they evaluate their arguments lazily and so cannot live in the pure
/// `&[Value]` table). Listed here so the dispatchable-name enumeration stays
/// complete; `lazy_operators_are_dispatchable` proves each one resolves.
pub const LAZY_OPERATOR_NAMES: &[&str] = &["and", "or"];

/// Canonical signatures of the lazy logical operators, kept in lock-step with
/// [`LAZY_OPERATOR_NAMES`] so they appear in [`dispatchable_signatures`].
pub const LAZY_OPERATOR_SIGNATURES: &[&str] = &["and(...)", "or(...)"];

/// Returns every canonical function name the evaluator can dispatch.
///
/// This is the authoritative runtime surface: the lazy operators plus the two
/// dispatch tables that [`dispatch`]/[`dispatch_fs`] actually consult.
pub fn dispatchable_canonical_names() -> Vec<&'static str> {
    let mut names: Vec<&'static str> = LAZY_OPERATOR_NAMES.to_vec();
    names.extend(PURE_FUNCTIONS.iter().map(|f| f.canonical));
    names.extend(FS_FUNCTIONS.iter().map(|f| f.canonical));
    names
}

/// Returns every canonical callable signature the evaluator dispatches,
/// including each overload and optional/variadic arity.
///
/// This is the authoritative runtime signature surface: the lazy operators plus
/// the per-registration [`PureFunction::signatures`]/[`FsFunction::signatures`].
/// The expression descriptor catalog is checked for exact set equality against
/// it, so adding or removing a callable *overload* (not merely a name) without
/// a matching descriptor — or vice versa — fails a test.
pub fn dispatchable_signatures() -> Vec<&'static str> {
    let mut sigs: Vec<&'static str> = LAZY_OPERATOR_SIGNATURES.to_vec();
    sigs.extend(PURE_FUNCTIONS.iter().flat_map(|f| f.signatures.iter().copied()));
    sigs.extend(FS_FUNCTIONS.iter().flat_map(|f| f.signatures.iter().copied()));
    sigs
}

/// Context-aware dispatch for filesystem/document functions.
///
/// ## Returns
///
/// `Some(result)` for names registered in [`FS_FUNCTIONS`], or `None` for any
/// other name (the caller then falls through to [`dispatch`]).
pub fn dispatch_fs(
    name: &str,
    args: &[Value],
    ctx: &ResolutionContext,
) -> Option<Result<Value, String>> {
    for f in FS_FUNCTIONS {
        if f.canonical == name || f.aliases.contains(&name) {
            return Some((f.handler)(args, ctx));
        }
    }
    None
}

/// Returns `true` when `name` matches a canonical name or alias in
/// [`FS_FUNCTIONS`].
///
/// Lets the evaluator distinguish a known filesystem function that fell
/// through dispatch only because no document resolution context was available
/// from a genuinely unrecognized symbol.
pub fn is_fs_function(name: &str) -> bool {
    FS_FUNCTIONS
        .iter()
        .any(|f| f.canonical == name || f.aliases.contains(&name))
}

/// Dispatches an evaluated function call against the pure helper library.
///
/// Returns `Some(result)` when the name matches an entry in [`PURE_FUNCTIONS`],
/// or `None` to let the outer dispatcher report `Unknown function: ...`.
pub fn dispatch(name: &str, args: &[Value]) -> Option<Result<Value, String>> {
    for f in PURE_FUNCTIONS {
        if f.canonical == name || f.aliases.contains(&name) {
            return Some((f.handler)(args));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn v(value: Value) -> Vec<Value> {
        vec![value]
    }

    fn vv(a: Value, b: Value) -> Vec<Value> {
        vec![a, b]
    }

    mod fn_type_predicates {
        use super::*;

        #[test]
        fn is_string_true_for_strings_only() {
            assert_eq!(is_string(&v(json!("hi"))).unwrap(), json!(true));
            assert_eq!(is_string(&v(json!(""))).unwrap(), json!(true));
            assert_eq!(is_string(&v(json!(0))).unwrap(), json!(false));
            assert_eq!(is_string(&v(json!(null))).unwrap(), json!(false));
            assert_eq!(is_string(&v(json!([]))).unwrap(), json!(false));
            assert_eq!(is_string(&v(json!({}))).unwrap(), json!(false));
            assert_eq!(is_string(&v(json!(true))).unwrap(), json!(false));
        }

        #[test]
        fn is_number_true_for_numbers_only() {
            assert_eq!(is_number(&v(json!(0))).unwrap(), json!(true));
            assert_eq!(is_number(&v(json!(1.5))).unwrap(), json!(true));
            assert_eq!(is_number(&v(json!("1"))).unwrap(), json!(false));
            assert_eq!(is_number(&v(json!(true))).unwrap(), json!(false));
            assert_eq!(is_number(&v(json!(null))).unwrap(), json!(false));
        }

        #[test]
        fn is_array_true_for_arrays_only() {
            assert_eq!(is_array(&v(json!([]))).unwrap(), json!(true));
            assert_eq!(is_array(&v(json!([1, 2]))).unwrap(), json!(true));
            assert_eq!(is_array(&v(json!({}))).unwrap(), json!(false));
            assert_eq!(is_array(&v(json!("a"))).unwrap(), json!(false));
        }

        #[test]
        fn is_null_true_for_null_only() {
            assert_eq!(is_null(&v(json!(null))).unwrap(), json!(true));
            assert_eq!(is_null(&v(json!(""))).unwrap(), json!(false));
            assert_eq!(is_null(&v(json!(0))).unwrap(), json!(false));
            assert_eq!(is_null(&v(json!(false))).unwrap(), json!(false));
        }

        #[test]
        fn is_object_true_for_objects_only() {
            assert_eq!(is_object(&v(json!({}))).unwrap(), json!(true));
            assert_eq!(is_object(&v(json!({"a": 1}))).unwrap(), json!(true));
            assert_eq!(is_object(&v(json!([]))).unwrap(), json!(false));
            assert_eq!(is_object(&v(json!("a"))).unwrap(), json!(false));
        }

        #[test]
        fn is_empty_true_for_null_empty_string_empty_array_empty_object() {
            assert_eq!(is_empty_fn(&v(json!(null))).unwrap(), json!(true));
            assert_eq!(is_empty_fn(&v(json!(""))).unwrap(), json!(true));
            assert_eq!(is_empty_fn(&v(json!([]))).unwrap(), json!(true));
            assert_eq!(is_empty_fn(&v(json!({}))).unwrap(), json!(true));
        }

        #[test]
        fn is_empty_false_for_numbers_booleans_and_non_empty_containers() {
            assert_eq!(is_empty_fn(&v(json!(0))).unwrap(), json!(false));
            assert_eq!(is_empty_fn(&v(json!(0.0))).unwrap(), json!(false));
            assert_eq!(is_empty_fn(&v(json!(false))).unwrap(), json!(false));
            assert_eq!(is_empty_fn(&v(json!(true))).unwrap(), json!(false));
            assert_eq!(is_empty_fn(&v(json!("a"))).unwrap(), json!(false));
            assert_eq!(is_empty_fn(&v(json!([0]))).unwrap(), json!(false));
            assert_eq!(is_empty_fn(&v(json!({"a": 1}))).unwrap(), json!(false));
        }
    }

    mod fn_math {
        use super::*;

        #[test]
        fn min_max_abs_basic() {
            assert_eq!(min_fn(&vv(json!(2), json!(5))).unwrap(), json!(2));
            assert_eq!(max_fn(&vv(json!(2), json!(5))).unwrap(), json!(5));
            assert_eq!(abs_fn(&v(json!(-3))).unwrap(), json!(3));
            assert_eq!(abs_fn(&v(json!(3.5))).unwrap(), json!(3.5));
        }

        #[test]
        fn math_null_propagates() {
            assert_eq!(min_fn(&vv(json!(null), json!(5))).unwrap(), json!(null));
            assert_eq!(max_fn(&vv(json!(2), json!(null))).unwrap(), json!(null));
            assert_eq!(abs_fn(&v(json!(null))).unwrap(), json!(null));
        }

        #[test]
        fn math_type_mismatch_errors() {
            let err = min_fn(&vv(json!("foo"), json!(5))).unwrap_err();
            assert!(err.contains("min"));
            let err = max_fn(&vv(json!([]), json!(5))).unwrap_err();
            assert!(err.contains("max"));
            let err = abs_fn(&v(json!("nope"))).unwrap_err();
            assert!(err.contains("abs"));
        }

        #[test]
        fn math_with_boolean_returns_error() {
            let err = min_fn(&vv(json!(true), json!(5))).unwrap_err();
            assert!(err.contains("min"), "got: {err}");
            let err = max_fn(&vv(json!(false), json!(5))).unwrap_err();
            assert!(err.contains("max"), "got: {err}");
            let err = abs_fn(&v(json!(true))).unwrap_err();
            assert!(err.contains("abs"), "got: {err}");
        }

        #[test]
        fn math_with_null_propagates() {
            assert_eq!(min_fn(&vv(json!(null), json!(5))).unwrap(), json!(null));
            assert_eq!(max_fn(&vv(json!(2), json!(null))).unwrap(), json!(null));
            assert_eq!(abs_fn(&v(json!(null))).unwrap(), json!(null));
        }
    }

    mod fn_collection {
        use super::*;

        #[test]
        fn first_last_basic() {
            assert_eq!(first_fn(&v(json!([1, 2, 3]))).unwrap(), json!(1));
            assert_eq!(last_fn(&v(json!([1, 2, 3]))).unwrap(), json!(3));
        }

        #[test]
        fn first_last_empty_returns_null() {
            assert_eq!(first_fn(&v(json!([]))).unwrap(), json!(null));
            assert_eq!(last_fn(&v(json!([]))).unwrap(), json!(null));
        }

        #[test]
        fn first_last_null_propagates() {
            assert_eq!(first_fn(&v(json!(null))).unwrap(), json!(null));
            assert_eq!(last_fn(&v(json!(null))).unwrap(), json!(null));
        }

        #[test]
        fn first_last_type_mismatch_errors() {
            assert!(first_fn(&v(json!("hi"))).is_err());
            assert!(last_fn(&v(json!({"a": 1}))).is_err());
        }
    }

    mod fn_string_predicates {
        use super::*;

        #[test]
        fn startswith_endswith_basic() {
            assert_eq!(
                starts_with(&vv(json!("foobar"), json!("foo"))).unwrap(),
                json!(true)
            );
            assert_eq!(
                starts_with(&vv(json!("foobar"), json!("bar"))).unwrap(),
                json!(false)
            );
            assert_eq!(
                ends_with(&vv(json!("foobar"), json!("bar"))).unwrap(),
                json!(true)
            );
            assert_eq!(
                ends_with(&vv(json!("foobar"), json!("foo"))).unwrap(),
                json!(false)
            );
        }

        #[test]
        fn startswith_case_sensitive() {
            assert_eq!(
                starts_with(&vv(json!("Foobar"), json!("foo"))).unwrap(),
                json!(false)
            );
        }

        #[test]
        fn startswith_endswith_null_propagates() {
            assert_eq!(
                starts_with(&vv(json!(null), json!("foo"))).unwrap(),
                json!(null)
            );
            assert_eq!(
                ends_with(&vv(json!("hi"), json!(null))).unwrap(),
                json!(null)
            );
        }

        #[test]
        fn startswith_endswith_type_mismatch_errors() {
            assert!(starts_with(&vv(json!(5), json!("foo"))).is_err());
            assert!(ends_with(&vv(json!("hi"), json!(5))).is_err());
        }
    }

    mod fn_string_mutations {
        use super::*;

        #[test]
        fn lower_upper_capitalize() {
            assert_eq!(lower(&v(json!("FoO"))).unwrap(), json!("foo"));
            assert_eq!(upper(&v(json!("FoO"))).unwrap(), json!("FOO"));
            assert_eq!(capitalize(&v(json!("hello"))).unwrap(), json!("Hello"));
            assert_eq!(capitalize(&v(json!(""))).unwrap(), json!(""));
        }

        #[test]
        fn case_conversions_handle_whitespace_and_punctuation() {
            assert_eq!(
                kebab_case(&v(json!("Hello World!"))).unwrap(),
                json!("hello-world")
            );
            assert_eq!(
                snake_case(&v(json!("Hello World!"))).unwrap(),
                json!("hello_world")
            );
            assert_eq!(
                camel_case(&v(json!("hello world"))).unwrap(),
                json!("helloWorld")
            );
            assert_eq!(
                pascal_case(&v(json!("hello world"))).unwrap(),
                json!("HelloWorld")
            );
            assert_eq!(
                title_case(&v(json!("hello world"))).unwrap(),
                json!("Hello World")
            );
        }

        #[test]
        fn case_conversions_handle_existing_separators() {
            assert_eq!(
                kebab_case(&v(json!("foo_bar-baz"))).unwrap(),
                json!("foo-bar-baz")
            );
            assert_eq!(
                snake_case(&v(json!("fooBarBaz"))).unwrap(),
                json!("foo_bar_baz")
            );
            assert_eq!(
                camel_case(&v(json!("foo-bar_baz"))).unwrap(),
                json!("fooBarBaz")
            );
            assert_eq!(
                pascal_case(&v(json!("foo-bar_baz"))).unwrap(),
                json!("FooBarBaz")
            );
        }

        #[test]
        fn case_conversions_handle_acronyms() {
            assert_eq!(
                snake_case(&v(json!("XMLHttpRequest"))).unwrap(),
                json!("xml_http_request")
            );
            assert_eq!(
                kebab_case(&v(json!("XMLHttpRequest"))).unwrap(),
                json!("xml-http-request")
            );
        }

        #[test]
        fn case_conversions_empty_string() {
            assert_eq!(kebab_case(&v(json!(""))).unwrap(), json!(""));
            assert_eq!(snake_case(&v(json!(""))).unwrap(), json!(""));
            assert_eq!(camel_case(&v(json!(""))).unwrap(), json!(""));
            assert_eq!(pascal_case(&v(json!(""))).unwrap(), json!(""));
            assert_eq!(title_case(&v(json!(""))).unwrap(), json!(""));
        }

        #[test]
        fn mutations_null_propagates() {
            assert_eq!(lower(&v(json!(null))).unwrap(), json!(null));
            assert_eq!(upper(&v(json!(null))).unwrap(), json!(null));
            assert_eq!(capitalize(&v(json!(null))).unwrap(), json!(null));
            assert_eq!(kebab_case(&v(json!(null))).unwrap(), json!(null));
            assert_eq!(snake_case(&v(json!(null))).unwrap(), json!(null));
            assert_eq!(camel_case(&v(json!(null))).unwrap(), json!(null));
            assert_eq!(pascal_case(&v(json!(null))).unwrap(), json!(null));
            assert_eq!(title_case(&v(json!(null))).unwrap(), json!(null));
        }

        #[test]
        fn mutations_type_mismatch_errors() {
            assert!(lower(&v(json!(5))).is_err());
            assert!(upper(&v(json!(true))).is_err());
            assert!(kebab_case(&v(json!([]))).is_err());
        }
    }

    mod fn_date_format {
        use super::*;

        #[test]
        fn ordinal_suffix_covers_teens_and_units() {
            assert_eq!(ordinal_suffix(1), "st");
            assert_eq!(ordinal_suffix(2), "nd");
            assert_eq!(ordinal_suffix(3), "rd");
            assert_eq!(ordinal_suffix(4), "th");
            assert_eq!(ordinal_suffix(11), "th");
            assert_eq!(ordinal_suffix(12), "th");
            assert_eq!(ordinal_suffix(13), "th");
            assert_eq!(ordinal_suffix(21), "st");
            assert_eq!(ordinal_suffix(22), "nd");
            assert_eq!(ordinal_suffix(23), "rd");
        }

        #[test]
        fn date_formats_known_patterns() {
            let d = |iso: &str, fmt: &str| date_fn(&[json!(iso), json!(fmt)]).unwrap();
            assert_eq!(d("2026-07-12", "MMMM Do"), json!("July 12th"));
            assert_eq!(d("2026-07-12", "short"), json!("July 12th"));
            assert_eq!(d("2026-07-12", "MMMM Do YYYY"), json!("July 12th 2026"));
            assert_eq!(d("2021-07-12", "D MMMM YYYY"), json!("12 July 2021"));
            assert_eq!(d("2021-07-12", "long"), json!("Mon, July 12th, 2021"));
            // [YYYY] optional-year extension: omit year when it equals the current year.
            let current_year = Local::now().format("%Y").to_string();
            let same_year = format!("{current_year}-07-12");
            assert_eq!(
                date_fn(&[json!(same_year), json!("MMMM Do [YYYY]")]).unwrap(),
                json!("July 12th")
            );
            assert_eq!(
                date_fn(&[json!("1999-07-12"), json!("MMMM Do [YYYY]")]).unwrap(),
                json!("July 12th 1999")
            );
        }

        #[test]
        fn date_errors_on_invalid_iso() {
            let err = date_fn(&[json!("not-a-date"), json!("short")]);
            assert!(err.is_err(), "expected error for invalid ISO input");
        }

        #[test]
        fn date_errors_on_unknown_format() {
            let err = date_fn(&[json!("2026-07-12"), json!("nope")]);
            assert!(err.is_err(), "expected error for unknown format token");
        }

        #[test]
        fn date_null_propagates() {
            assert_eq!(date_fn(&[json!(null), json!("short")]).unwrap(), json!(null));
        }

        #[test]
        fn date_dispatches_by_name() {
            let result = dispatch("date", &[json!("2021-07-12"), json!("long")]);
            assert_eq!(result.unwrap().unwrap(), json!("Mon, July 12th, 2021"));
        }
    }

    mod fn_filesystem {
        use super::*;

        #[test]
        fn dispatch_fs_returns_none_for_non_fs_names() {
            let ctx = ResolutionContext::new(std::path::PathBuf::from("."));
            assert!(dispatch_fs("lower", &[json!("x")], &ctx).is_none());
        }

        #[test]
        fn absolute_and_file_exists_resolve_relative_to_base_dir() {
            let dir = tempfile::TempDir::new().unwrap();
            std::fs::write(dir.path().join("a.md"), "# A\n").unwrap();
            let ctx = ResolutionContext::new(dir.path().to_path_buf());

            let abs = absolute_fn(&[json!("a.md")], &ctx).unwrap();
            assert_eq!(
                abs,
                json!(dir.path().join("a.md").to_string_lossy().to_string())
            );

            assert_eq!(file_exists_fn(&[json!("a.md")], &ctx).unwrap(), json!(true));
            assert_eq!(
                file_exists_fn(&[json!("missing.md")], &ctx).unwrap(),
                json!(false)
            );
            // Invalid path string → file_exists is false (never errors).
            assert_eq!(
                file_exists_fn(&[json!("\0bad")], &ctx).unwrap(),
                json!(false)
            );
        }

        #[test]
        #[serial_test::serial]
        fn file_exists_falls_back_to_process_cwd() {
            // A path that does not exist relative to base_dir but DOES exist
            // relative to the process CWD must still resolve — matching the
            // `file` schema validator, which resolves from the ambient CWD.
            // This is the case a CLI `-y` variable typed relative to the user's
            // working directory hits when the document lives elsewhere.
            let cwd_dir = tempfile::TempDir::new().unwrap();
            std::fs::write(cwd_dir.path().join("plan.md"), "# Plan\n").unwrap();
            // base_dir deliberately lacks plan.md, so document-relative
            // resolution misses and only the CWD fallback can succeed.
            let base_dir = tempfile::TempDir::new().unwrap();
            let ctx = ResolutionContext::new(base_dir.path().to_path_buf());

            let original = std::env::current_dir().unwrap();
            std::env::set_current_dir(cwd_dir.path()).unwrap();
            let found = file_exists_fn(&[json!("plan.md")], &ctx);
            std::env::set_current_dir(&original).unwrap();

            assert_eq!(found.unwrap(), json!(true));
        }

        #[test]
        fn relative_returns_repo_or_cwd_relative_path() {
            let dir = tempfile::TempDir::new().unwrap();
            std::fs::create_dir_all(dir.path().join("sub")).unwrap();
            std::fs::write(dir.path().join("sub/a.md"), "# A\n").unwrap();
            let ctx = ResolutionContext::new(dir.path().to_path_buf());
            let rel = relative_fn(&[json!("sub/a.md")], &ctx).unwrap();
            assert_eq!(rel, json!("sub/a.md"));
        }

        #[test]
        fn frontmatter_reads_whole_map_and_single_prop() {
            let dir = tempfile::TempDir::new().unwrap();
            std::fs::write(
                dir.path().join("d.md"),
                "---\ntitle: Hi\nstatus: draft\n---\nBody\n",
            )
            .unwrap();
            let ctx = ResolutionContext::new(dir.path().to_path_buf());

            let whole = frontmatter_fn(&[json!("d.md")], &ctx).unwrap();
            assert_eq!(whole["title"], json!("Hi"));

            let one = frontmatter_fn(&[json!("d.md"), json!("status")], &ctx).unwrap();
            assert_eq!(one, json!("draft"));

            // Missing prop → null (not error).
            let missing = frontmatter_fn(&[json!("d.md"), json!("nope")], &ctx).unwrap();
            assert_eq!(missing, Value::Null);

            // Invalid filepath → error.
            assert!(frontmatter_fn(&[json!("does-not-exist.md")], &ctx).is_err());
        }

        #[test]
        fn markdown_body_empty_and_title() {
            let dir = tempfile::TempDir::new().unwrap();
            std::fs::write(dir.path().join("empty.md"), "---\ntitle: T\n---\n\n   \n").unwrap();
            std::fs::write(dir.path().join("full.md"), "---\n---\n# Heading\n\nWords\n").unwrap();
            std::fs::write(dir.path().join("fm_title.md"), "---\ntitle: FM\n---\n# H1\n").unwrap();
            let ctx = ResolutionContext::new(dir.path().to_path_buf());

            assert_eq!(
                markdown_body_empty_fn(&[json!("empty.md")], &ctx).unwrap(),
                json!(true)
            );
            assert_eq!(
                markdown_body_empty_fn(&[json!("full.md")], &ctx).unwrap(),
                json!(false)
            );
            // Frontmatter title wins over H1.
            assert_eq!(
                markdown_title_fn(&[json!("fm_title.md")], &ctx).unwrap(),
                json!("FM")
            );
            // No frontmatter title → first H1.
            assert_eq!(
                markdown_title_fn(&[json!("full.md")], &ctx).unwrap(),
                json!("Heading")
            );
        }

        #[test]
        fn validate_schema_true_when_no_schema_property() {
            let dir = tempfile::TempDir::new().unwrap();
            std::fs::write(dir.path().join("plain.md"), "---\ntitle: T\n---\nBody\n").unwrap();
            let ctx = ResolutionContext::new(dir.path().to_path_buf());
            assert_eq!(
                validate_schema_fn(&[json!("plain.md")], &ctx).unwrap(),
                json!(true)
            );
        }
    }

    mod fn_isdate_strict {
        use super::*;

        #[test]
        fn isdate_accepts_iso_dates() {
            assert_eq!(is_date(&v(json!("2024-06-15"))).unwrap(), json!(true));
            assert_eq!(is_date_utc(&v(json!("2024-06-15"))).unwrap(), json!(true));
        }

        #[test]
        fn isdate_rejects_bad_formats() {
            assert_eq!(is_date(&v(json!("2024/06/15"))).unwrap(), json!(false));
            assert_eq!(is_date(&v(json!("06-15-2024"))).unwrap(), json!(false));
            assert_eq!(is_date(&v(json!("2024-13-01"))).unwrap(), json!(false));
            assert_eq!(
                is_date(&v(json!("2024-06-15T00:00:00"))).unwrap(),
                json!(false)
            );
        }

        #[test]
        fn isdate_rejects_non_strings_including_null() {
            assert_eq!(is_date(&v(json!(null))).unwrap(), json!(false));
            assert_eq!(is_date(&v(json!(20240615))).unwrap(), json!(false));
            assert_eq!(is_date(&v(json!(true))).unwrap(), json!(false));
            assert_eq!(is_date_utc(&v(json!(null))).unwrap(), json!(false));
        }

        #[test]
        fn isdatetime_accepts_iso_datetimes() {
            assert_eq!(
                is_datetime(&v(json!("2024-06-15T12:30:00"))).unwrap(),
                json!(true)
            );
            assert_eq!(
                is_datetime(&v(json!("2024-06-15T12:30:00Z"))).unwrap(),
                json!(true)
            );
            assert_eq!(
                is_datetime(&v(json!("2024-06-15T12:30:00+02:00"))).unwrap(),
                json!(true)
            );
            assert_eq!(
                is_datetime(&v(json!("2024-06-15T12:30:00.123"))).unwrap(),
                json!(true)
            );
            assert_eq!(
                is_datetime_utc(&v(json!("2024-06-15T12:30:00Z"))).unwrap(),
                json!(true)
            );
        }

        #[test]
        fn isdatetime_rejects_invalid_inputs() {
            assert_eq!(is_datetime(&v(json!("2024-06-15"))).unwrap(), json!(false));
            assert_eq!(is_datetime(&v(json!("nope"))).unwrap(), json!(false));
            assert_eq!(is_datetime(&v(json!(null))).unwrap(), json!(false));
            assert_eq!(is_datetime(&v(json!(123))).unwrap(), json!(false));
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
        }

        #[test]
        fn istoday_with_accepts_datetime_strings() {
            assert!(is_today_with(&json!("2024-06-15T08:00:00"), TODAY, false));
            assert!(is_today_with(&json!("2024-06-15T08:00:00Z"), TODAY, true));
        }

        #[test]
        fn istoday_with_rejects_non_strings_and_invalid() {
            assert!(!is_today_with(&json!(null), TODAY, false));
            assert!(!is_today_with(&json!(20240615), TODAY, false));
            assert!(!is_today_with(&json!("not-a-date"), TODAY, false));
        }

        #[test]
        fn isyesterday_with_matches_prev_day() {
            assert!(is_yesterday_with(&json!("2024-06-14"), TODAY, false));
            assert!(!is_yesterday_with(&json!("2024-06-15"), TODAY, false));
            assert!(!is_yesterday_with(&json!(null), TODAY, false));
        }

        #[test]
        fn istomorrow_with_matches_next_day() {
            assert!(is_tomorrow_with(&json!("2024-06-16"), TODAY, false));
            assert!(!is_tomorrow_with(&json!("2024-06-15"), TODAY, false));
        }

        #[test]
        fn isthismonth_with_matches_same_month_year() {
            assert!(is_this_month_with(&json!("2024-06-01"), TODAY, false));
            assert!(is_this_month_with(&json!("2024-06-30"), TODAY, false));
            assert!(!is_this_month_with(&json!("2024-07-01"), TODAY, false));
            assert!(!is_this_month_with(&json!("2023-06-15"), TODAY, false));
        }

        #[test]
        fn isthisyear_with_matches_same_year() {
            assert!(is_this_year_with(&json!("2024-01-01"), TODAY, false));
            assert!(is_this_year_with(&json!("2024-12-31"), TODAY, false));
            assert!(!is_this_year_with(&json!("2023-12-31"), TODAY, false));
        }

        #[test]
        fn relative_validators_accept_datetime_strings() {
            // "today" datetime in UTC.
            assert!(is_today_with(&json!("2024-06-15T23:59:00Z"), TODAY, true));
            // Datetime carrying an explicit offset that lands on a
            // different UTC date when assume_utc=true.
            assert!(is_today_with(
                &json!("2024-06-16T01:00:00+03:00"),
                TODAY,
                true
            ));
        }
    }

    mod fn_phase3 {
        use super::*;

        #[test]
        fn is_positive_true_for_positive_numbers_and_strings() {
            assert_eq!(is_positive(&v(json!(1))).unwrap(), json!(true));
            assert_eq!(is_positive(&v(json!(1.5))).unwrap(), json!(true));
            assert_eq!(is_positive(&v(json!("0.5"))).unwrap(), json!(true));
            assert_eq!(is_positive(&v(json!(true))).unwrap(), json!(true));
        }

        #[test]
        fn is_positive_false_for_zero_and_negative() {
            assert_eq!(is_positive(&v(json!(0))).unwrap(), json!(false));
            assert_eq!(is_positive(&v(json!(0.0))).unwrap(), json!(false));
            assert_eq!(is_positive(&v(json!(-1))).unwrap(), json!(false));
            assert_eq!(is_positive(&v(json!("-0.5"))).unwrap(), json!(false));
            assert_eq!(is_positive(&v(json!(false))).unwrap(), json!(false));
        }

        #[test]
        fn is_positive_errors_for_null_and_non_numeric() {
            assert!(is_positive(&v(json!(null))).is_err());
            assert!(is_positive(&v(json!("nope"))).is_err());
            assert!(is_positive(&v(json!([1, 2]))).is_err());
        }

        #[test]
        fn is_negative_true_for_negative_numbers_and_strings() {
            assert_eq!(is_negative(&v(json!(-1))).unwrap(), json!(true));
            assert_eq!(is_negative(&v(json!(-1.5))).unwrap(), json!(true));
            assert_eq!(is_negative(&v(json!("-0.5"))).unwrap(), json!(true));
        }

        #[test]
        fn is_negative_false_for_zero_and_positive() {
            assert_eq!(is_negative(&v(json!(0))).unwrap(), json!(false));
            assert_eq!(is_negative(&v(json!(0.0))).unwrap(), json!(false));
            assert_eq!(is_negative(&v(json!(1))).unwrap(), json!(false));
            assert_eq!(is_negative(&v(json!(true))).unwrap(), json!(false));
        }

        #[test]
        fn is_negative_errors_for_null_and_non_numeric() {
            assert!(is_negative(&v(json!(null))).is_err());
            assert!(is_negative(&v(json!("nope"))).is_err());
            assert!(is_negative(&v(json!({"a": 1}))).is_err());
        }

        #[test]
        fn is_integer_never_errors_or_null_propagates() {
            assert_eq!(is_integer(&v(json!(null))).unwrap(), json!(false));
            assert_eq!(is_integer(&v(json!("123"))).unwrap(), json!(false));
            assert_eq!(is_integer(&v(json!(true))).unwrap(), json!(false));
            assert_eq!(is_integer(&v(json!([1]))).unwrap(), json!(false));
            assert_eq!(is_integer(&v(json!({"a": 1}))).unwrap(), json!(false));
        }

        #[test]
        fn is_integer_true_for_whole_numbers() {
            assert_eq!(is_integer(&v(json!(1))).unwrap(), json!(true));
            assert_eq!(is_integer(&v(json!(1.0))).unwrap(), json!(true));
            assert_eq!(is_integer(&v(json!(0))).unwrap(), json!(true));
            assert_eq!(is_integer(&v(json!(-42))).unwrap(), json!(true));
        }

        #[test]
        fn is_integer_false_for_fractional_numbers() {
            assert_eq!(is_integer(&v(json!(1.5))).unwrap(), json!(false));
            assert_eq!(is_integer(&v(json!(0.1))).unwrap(), json!(false));
        }

        #[test]
        fn without_date_removes_valid_dates_and_preserves_invalid() {
            assert_eq!(
                without_date(&v(json!("plan 2026-06-15 review"))).unwrap(),
                json!("plan  review")
            );
            assert_eq!(
                without_date(&v(json!("invalid 2026-02-30 stays"))).unwrap(),
                json!("invalid 2026-02-30 stays")
            );
            assert_eq!(
                without_date(&v(json!("meeting 2026-06-15T10:30:00 here"))).unwrap(),
                json!("meeting T10:30:00 here")
            );
            assert_eq!(
                without_date(&v(json!("x 2026-06-15, y"))).unwrap(),
                json!("x , y")
            );
        }

        #[test]
        fn without_date_null_propagates_and_type_mismatch_errors() {
            assert_eq!(without_date(&v(json!(null))).unwrap(), json!(null));
            assert!(without_date(&v(json!(123))).is_err());
            assert!(without_date(&v(json!([1, 2]))).is_err());
        }

        #[test]
        fn ensure_leading_examples() {
            assert_eq!(
                ensure_leading(&vv(json!("foobar"), json!("foo"))).unwrap(),
                json!("foobar")
            );
            assert_eq!(
                ensure_leading(&vv(json!("bar"), json!("foo"))).unwrap(),
                json!("foobar")
            );
            assert_eq!(
                ensure_leading(&vv(json!(123), json!(4))).unwrap(),
                json!(4123)
            );
            assert_eq!(
                ensure_leading(&vv(json!("123"), json!(4))).unwrap(),
                json!("4123")
            );
        }

        #[test]
        fn ensure_leading_preserves_number_type_when_already_prefixed() {
            assert_eq!(
                ensure_leading(&vv(json!(123), json!("12"))).unwrap(),
                json!(123)
            );
            assert_eq!(
                ensure_leading(&vv(json!(123), json!(12))).unwrap(),
                json!(123)
            );
        }

        #[test]
        fn ensure_leading_null_propagates() {
            assert_eq!(
                ensure_leading(&vv(json!(null), json!("foo"))).unwrap(),
                json!(null)
            );
            assert_eq!(
                ensure_leading(&vv(json!("bar"), json!(null))).unwrap(),
                json!(null)
            );
        }

        #[test]
        fn ensure_leading_rejects_booleans_arrays_objects() {
            assert!(ensure_leading(&vv(json!(true), json!("foo"))).is_err());
            assert!(ensure_leading(&vv(json!("bar"), json!(true))).is_err());
            assert!(ensure_leading(&vv(json!([1]), json!("foo"))).is_err());
            assert!(ensure_leading(&vv(json!("bar"), json!({"a": 1}))).is_err());
        }

        #[test]
        fn ensure_trailing_examples() {
            assert_eq!(
                ensure_trailing(&vv(json!("foobar"), json!("bar"))).unwrap(),
                json!("foobar")
            );
            assert_eq!(
                ensure_trailing(&vv(json!("foo"), json!("bar"))).unwrap(),
                json!("foobar")
            );
            assert_eq!(
                ensure_trailing(&vv(json!(123), json!(4))).unwrap(),
                json!(1234)
            );
            assert_eq!(
                ensure_trailing(&vv(json!("123"), json!(4))).unwrap(),
                json!("1234")
            );
        }

        #[test]
        fn ensure_trailing_null_propagates_and_type_mismatch() {
            assert_eq!(
                ensure_trailing(&vv(json!(null), json!("bar"))).unwrap(),
                json!(null)
            );
            assert!(ensure_trailing(&vv(json!("foo"), json!(false))).is_err());
            assert!(ensure_trailing(&vv(json!([]), json!("bar"))).is_err());
        }

        #[test]
        fn terminal_renders_bold_markup() {
            let out = terminal(&v(json!("<bold>x</bold>"))).unwrap();
            let s = out.as_str().unwrap();
            assert!(s.contains("\x1b[1m"), "expected bold SGR sequence in {s:?}");
            assert!(s.contains('x'), "expected literal x in {s:?}");
        }

        #[test]
        fn terminal_literal_text_passes_through() {
            let out = terminal(&v(json!("hello world"))).unwrap();
            assert_eq!(out, json!("hello world"));
        }

        #[test]
        fn terminal_null_propagates_and_type_mismatch_errors() {
            assert_eq!(terminal(&v(json!(null))).unwrap(), json!(null));
            assert!(terminal(&v(json!(123))).is_err());
            assert!(terminal(&v(json!([1, 2]))).is_err());
        }

        #[test]
        fn phase3_functions_dispatch_by_name() {
            assert_eq!(dispatch("is_positive", &[json!(5)]).unwrap().unwrap(), json!(true));
            assert_eq!(dispatch("isnegative", &[json!(-1)]).unwrap().unwrap(), json!(true));
            assert_eq!(dispatch("isinteger", &[json!(42)]).unwrap().unwrap(), json!(true));
            assert_eq!(
                dispatch("without_date", &[json!("2026-06-15x")]).unwrap().unwrap(),
                json!("x")
            );
            assert_eq!(
                dispatch("ensure_leading", &[json!("bar"), json!("foo")])
                    .unwrap()
                    .unwrap(),
                json!("foobar")
            );
            assert_eq!(
                dispatch("ensuretrailing", &[json!("foo"), json!("bar")])
                    .unwrap()
                    .unwrap(),
                json!("foobar")
            );
            assert!(dispatch("terminal", &[json!("<bold>x</bold>")]).unwrap().is_ok());
        }

        #[test]
        fn phase3_functions_require_correct_arity() {
            assert!(is_positive(&[]).is_err());
            assert!(is_positive(&[json!(1), json!(2)]).is_err());
            assert!(without_date(&[]).is_err());
            assert!(without_date(&[json!("a"), json!("b")]).is_err());
            assert!(ensure_leading(&[json!("a")]).is_err());
            assert!(ensure_leading(&[json!("a"), json!("b"), json!("c")]).is_err());
            assert!(terminal(&[]).is_err());
        }
    }

    mod fn_phase4 {
        use super::*;

        fn ctx_with_temp_dir() -> (tempfile::TempDir, ResolutionContext) {
            let dir = tempfile::TempDir::new().unwrap();
            let ctx = ResolutionContext::new(dir.path().to_path_buf());
            (dir, ctx)
        }

        #[test]
        fn is_indexed_file_detects_indexed_stems() {
            let (dir, ctx) = ctx_with_temp_dir();
            std::fs::write(dir.path().join("review-1.md"), "x").unwrap();
            std::fs::write(dir.path().join("review.md"), "x").unwrap();
            std::fs::write(dir.path().join("review_1.md"), "x").unwrap();

            assert_eq!(is_indexed_file_fn(&[json!("review-1.md")], &ctx).unwrap(), json!(true));
            assert_eq!(is_indexed_file_fn(&[json!("review-100.md")], &ctx).unwrap(), json!(true));
            assert_eq!(is_indexed_file_fn(&[json!("review-001.md")], &ctx).unwrap(), json!(true));
            assert_eq!(is_indexed_file_fn(&[json!("review.md")], &ctx).unwrap(), json!(false));
            assert_eq!(is_indexed_file_fn(&[json!("review_1.md")], &ctx).unwrap(), json!(false));
            assert_eq!(is_indexed_file_fn(&[json!("review1.md")], &ctx).unwrap(), json!(false));
        }

        #[test]
        fn file_index_parses_or_negative_one() {
            let (dir, ctx) = ctx_with_temp_dir();
            std::fs::write(dir.path().join("review-42.md"), "x").unwrap();
            std::fs::write(dir.path().join("review.md"), "x").unwrap();

            assert_eq!(file_index_fn(&[json!("review-42.md")], &ctx).unwrap(), json!(42));
            assert_eq!(file_index_fn(&[json!("review.md")], &ctx).unwrap(), json!(-1));
        }

        #[test]
        fn increment_file_index_preserves_zero_padding() {
            let (dir, ctx) = ctx_with_temp_dir();
            std::fs::write(dir.path().join("review-1.md"), "x").unwrap();
            std::fs::write(dir.path().join("review-001.md"), "x").unwrap();
            std::fs::write(dir.path().join("review.md"), "x").unwrap();

            assert_eq!(
                increment_file_index_fn(&[json!("review-1.md")], &ctx).unwrap(),
                json!("review-2.md")
            );
            assert_eq!(
                increment_file_index_fn(&[json!("review-001.md")], &ctx).unwrap(),
                json!("review-002.md")
            );
            assert_eq!(
                increment_file_index_fn(&[json!("review.md")], &ctx).unwrap(),
                json!("review-2.md")
            );
        }

        #[test]
        fn decrement_file_index_clamps_at_zero() {
            let (dir, ctx) = ctx_with_temp_dir();
            std::fs::write(dir.path().join("review-001.md"), "x").unwrap();
            std::fs::write(dir.path().join("review-0.md"), "x").unwrap();
            std::fs::write(dir.path().join("review.md"), "x").unwrap();

            assert_eq!(
                decrement_file_index_fn(&[json!("review-001.md")], &ctx).unwrap(),
                json!("review-000.md")
            );
            assert_eq!(
                decrement_file_index_fn(&[json!("review-0.md")], &ctx).unwrap(),
                json!("review-0.md")
            );
            assert_eq!(
                decrement_file_index_fn(&[json!("review.md")], &ctx).unwrap(),
                json!("review-0.md")
            );
        }

        #[test]
        fn indexed_functions_null_propagate_and_reject_remote() {
            let ctx = ResolutionContext::new(std::env::temp_dir());

            assert_eq!(is_indexed_file_fn(&[json!(null)], &ctx).unwrap(), json!(null));
            assert_eq!(file_index_fn(&[json!(null)], &ctx).unwrap(), json!(null));
            assert_eq!(
                increment_file_index_fn(&[json!(null)], &ctx).unwrap(),
                json!(null)
            );

            let err = is_indexed_file_fn(&[json!("https://example.com/doc.md")], &ctx)
                .unwrap_err();
            assert!(err.contains("HTTP(S)"), "got: {err}");
        }

        #[test]
        fn indexed_functions_require_arity() {
            let ctx = ResolutionContext::new(std::env::temp_dir());
            assert!(is_indexed_file_fn(&[], &ctx).is_err());
            assert!(is_indexed_file_fn(&[json!("a"), json!("b")], &ctx).is_err());
            assert!(file_index_fn(&[], &ctx).is_err());
            assert!(increment_file_index_fn(&[json!("a"), json!("b")], &ctx).is_err());
        }

        #[test]
        fn path_components_split_display_path() {
            let (dir, ctx) = ctx_with_temp_dir();
            std::fs::create_dir_all(dir.path().join("foo/bar/baz")).unwrap();
            std::fs::write(dir.path().join("foo/bar/baz/test.md"), "x").unwrap();
            std::fs::write(dir.path().join("foo/review-1.md"), "x").unwrap();
            std::fs::write(dir.path().join("no-ext"), "x").unwrap();

            assert_eq!(basename_fn(&[json!("foo/bar/baz/test.md")], &ctx).unwrap(), json!("test.md"));
            assert_eq!(
                basename_without_index_fn(&[json!("foo/review-1.md")], &ctx).unwrap(),
                json!("review.md")
            );
            assert_eq!(dir_fn(&[json!("foo/bar/baz/test.md")], &ctx).unwrap(), json!("foo/bar/baz"));
            assert_eq!(ext_fn(&[json!("foo/bar/baz/test.md")], &ctx).unwrap(), json!("md"));
            assert_eq!(ext_fn(&[json!("no-ext")], &ctx).unwrap(), json!(""));
            assert_eq!(
                parent_dir_fn(&[json!("foo/bar/baz/test.md")], &ctx).unwrap(),
                json!("baz")
            );
            assert_eq!(
                file_trailing_fn(&[json!("foo/bar/baz/test.md")], &ctx).unwrap(),
                json!("baz/test.md")
            );
            assert_eq!(
                dir_leading_fn(&[json!("foo/bar/baz/test.md")], &ctx).unwrap(),
                json!("foo/bar")
            );
        }

        #[test]
        fn path_components_handle_bare_basename() {
            let (dir, ctx) = ctx_with_temp_dir();
            std::fs::write(dir.path().join("test.md"), "x").unwrap();

            assert_eq!(basename_fn(&[json!("test.md")], &ctx).unwrap(), json!("test.md"));
            assert_eq!(dir_fn(&[json!("test.md")], &ctx).unwrap(), json!(""));
            assert_eq!(parent_dir_fn(&[json!("test.md")], &ctx).unwrap(), json!(""));
            assert_eq!(file_trailing_fn(&[json!("test.md")], &ctx).unwrap(), json!("test.md"));
            assert_eq!(dir_leading_fn(&[json!("test.md")], &ctx).unwrap(), json!(""));
        }

        #[test]
        fn join_normalizes_separators_and_resolves() {
            let (dir, ctx) = ctx_with_temp_dir();
            std::fs::create_dir_all(dir.path().join("foo/bar")).unwrap();
            std::fs::write(dir.path().join("foo/bar/baz.md"), "x").unwrap();

            assert_eq!(
                join_fn(&[json!("foo/bar/"), json!("/baz.md")], &ctx).unwrap(),
                json!("foo/bar/baz.md")
            );
            assert_eq!(
                join_fn(&[json!("foo/bar"), json!("baz/bax.md")], &ctx).unwrap(),
                json!("foo/bar/baz/bax.md")
            );
            assert_eq!(
                join_fn(&[json!("foo//bar//"), json!("//baz.md")], &ctx).unwrap(),
                json!("foo/bar/baz.md")
            );
        }

        #[test]
        fn join_rejects_http_and_null_propagates() {
            let ctx = ResolutionContext::new(std::env::temp_dir());

            assert_eq!(join_fn(&[json!(null), json!("b")], &ctx).unwrap(), json!(null));
            let err = join_fn(&[json!("https://example.com"), json!("b")], &ctx).unwrap_err();
            assert!(err.contains("HTTP(S)"), "got: {err}");
            let err = join_fn(&[json!("a"), json!("https://example.com/b")], &ctx).unwrap_err();
            assert!(err.contains("HTTP(S)"), "got: {err}");
        }

        #[test]
        fn join_requires_strings_and_correct_arity() {
            let ctx = ResolutionContext::new(std::env::temp_dir());
            assert!(join_fn(&[], &ctx).is_err());
            assert!(join_fn(&[json!("a")], &ctx).is_err());
            assert!(join_fn(&[json!(123), json!("b")], &ctx).is_err());
            assert!(join_fn(&[json!("a"), json!([])], &ctx).is_err());
        }

        #[test]
        fn path_functions_do_not_require_existence() {
            let ctx = ResolutionContext::new(std::env::temp_dir());

            assert_eq!(basename_fn(&[json!("foo/bar/missing.md")], &ctx).unwrap(), json!("missing.md"));
            assert_eq!(dir_fn(&[json!("foo/bar/missing.md")], &ctx).unwrap(), json!("foo/bar"));
            assert_eq!(
                increment_file_index_fn(&[json!("foo/bar/missing.md")], &ctx).unwrap(),
                json!("foo/bar/missing-2.md")
            );
        }

        #[test]
        fn path_functions_dispatch_by_name() {
            let ctx = ResolutionContext::new(std::env::temp_dir());
            assert_eq!(
                dispatch_fs("is_indexed_file", &[json!("foo-1.md")], &ctx)
                    .unwrap()
                    .unwrap(),
                json!(true)
            );
            assert_eq!(
                dispatch_fs("file_index", &[json!("foo-7.md")], &ctx).unwrap().unwrap(),
                json!(7)
            );
            assert_eq!(
                dispatch_fs("basename", &[json!("foo/bar.md")], &ctx).unwrap().unwrap(),
                json!("bar.md")
            );
            assert_eq!(
                dispatch_fs("join", &[json!("foo"), json!("bar.md")], &ctx)
                    .unwrap()
                    .unwrap(),
                json!("foo/bar.md")
            );
        }
    }

    mod fn_phase5 {
        use super::*;

        fn ctx_with_temp_dir() -> (tempfile::TempDir, ResolutionContext) {
            let dir = tempfile::TempDir::new().unwrap();
            let ctx = ResolutionContext::new(dir.path().to_path_buf());
            (dir, ctx)
        }

        #[test]
        fn link_one_arg_uses_relative_text_and_absolute_destination() {
            let (dir, ctx) = ctx_with_temp_dir();
            std::fs::create_dir_all(dir.path().join("foo/bar")).unwrap();
            std::fs::write(dir.path().join("foo/bar/baz.md"), "x").unwrap();

            let result = link_fn(&[json!("foo/bar/baz.md")], &ctx)
                .unwrap()
                .as_str()
                .unwrap()
                .to_string();
            assert!(result.starts_with("[foo/bar/baz.md]("));
            assert!(result.contains("/foo/bar/baz.md)"));
        }

        #[test]
        fn link_two_arg_file_uses_provided_description() {
            let (dir, ctx) = ctx_with_temp_dir();
            std::fs::write(dir.path().join("doc.md"), "x").unwrap();

            let result = link_fn(&[json!("doc.md"), json!("My Doc")], &ctx)
                .unwrap()
                .as_str()
                .unwrap()
                .to_string();
            assert_eq!(result, "[My Doc](".to_string() + &dir.path().join("doc.md").to_string_lossy() + ")");
        }

        #[test]
        fn link_two_arg_https_emits_url_destination() {
            let ctx = ResolutionContext::new(std::env::temp_dir());
            let result = link_fn(
                &[
                    json!("https://example.com/page"),
                    json!("Example"),
                ],
                &ctx,
            )
            .unwrap();
            assert_eq!(result, json!("[Example](https://example.com/page)"));
        }

        #[test]
        fn link_escapes_brackets_in_text() {
            let ctx = ResolutionContext::new(std::env::temp_dir());
            let result = link_fn(
                &[
                    json!("https://example.com"),
                    json!("click [here]"),
                ],
                &ctx,
            )
            .unwrap();
            assert_eq!(
                result,
                json!("[click \\[here\\]](https://example.com)")
            );
        }

        #[test]
        fn link_wraps_destination_with_spaces_and_parens() {
            let (dir, ctx) = ctx_with_temp_dir();
            std::fs::write(dir.path().join("my doc (v1).md"), "x").unwrap();

            let result = link_fn(
                &[
                    json!("my doc (v1).md"),
                    json!("spaces"),
                ],
                &ctx,
            )
            .unwrap()
            .as_str()
            .unwrap()
            .to_string();
            assert!(
                result.starts_with("[spaces](<"),
                "expected angle-bracket wrapping for destination with spaces, got {result:?}"
            );
            assert!(result.ends_with(">)"));
        }

        #[test]
        fn link_one_arg_rejects_remote_url() {
            let ctx = ResolutionContext::new(std::env::temp_dir());
            let err = link_fn(
                &[json!("https://example.com/doc.md")],
                &ctx,
            )
            .unwrap_err();
            assert!(err.contains("HTTP(S)"), "got: {err}");
        }

        #[test]
        fn basename_rejects_uppercase_scheme_urls() {
            let ctx = ResolutionContext::new(std::env::temp_dir());

            let err = basename_fn(&[json!("HTTPS://example.com/doc.md")], &ctx)
                .unwrap_err();
            assert!(err.contains("HTTP(S)"), "got: {err}");
            let err = basename_fn(&[json!("hTtPs://example.com/doc.md")], &ctx)
                .unwrap_err();
            assert!(err.contains("HTTP(S)"), "got: {err}");
        }

        #[test]
        fn join_rejects_uppercase_scheme_urls() {
            let ctx = ResolutionContext::new(std::env::temp_dir());

            let err = join_fn(&[json!("HTTPS://example.com"), json!("b")], &ctx)
                .unwrap_err();
            assert!(err.contains("HTTP(S)"), "got: {err}");
            let err = join_fn(&[json!("a"), json!("HTTP://example.com/b")], &ctx)
                .unwrap_err();
            assert!(err.contains("HTTP(S)"), "got: {err}");
        }

        #[test]
        fn link_one_arg_rejects_uppercase_scheme_urls() {
            let ctx = ResolutionContext::new(std::env::temp_dir());
            let err = link_fn(&[json!("HTTPS://example.com/doc.md")], &ctx).unwrap_err();
            assert!(err.contains("HTTP(S)"), "got: {err}");
        }

        #[test]
        fn link_two_arg_accepts_uppercase_scheme_urls() {
            let ctx = ResolutionContext::new(std::env::temp_dir());

            let result = link_fn(
                &[json!("HTTPS://example.com/page"), json!("Example")],
                &ctx,
            )
            .unwrap();
            assert_eq!(result, json!("[Example](HTTPS://example.com/page)"));
            let result = link_fn(
                &[json!("hTtP://example.com/page"), json!("Example")],
                &ctx,
            )
            .unwrap();
            assert_eq!(result, json!("[Example](hTtP://example.com/page)"));
        }

        #[test]
        fn link_null_propagates_and_arity_errors() {
            let ctx = ResolutionContext::new(std::env::temp_dir());
            assert_eq!(
                link_fn(&[json!(null), json!("desc")], &ctx).unwrap(),
                json!(null)
            );
            assert_eq!(
                link_fn(&[json!("https://example.com"), json!(null)], &ctx).unwrap(),
                json!(null)
            );
            assert!(link_fn(&[], &ctx).is_err());
            assert!(link_fn(&[json!("a"), json!("b"), json!("c")], &ctx).is_err());
        }

        #[test]
        fn link_type_mismatch_errors() {
            let ctx = ResolutionContext::new(std::env::temp_dir());
            assert!(link_fn(&[json!(123)], &ctx).is_err());
            assert!(link_fn(&[json!("a"), json!(123)], &ctx).is_err());
        }

        #[test]
        fn has_skill_finds_user_and_local_roots() {
            let home = tempfile::TempDir::new().unwrap();
            let local = tempfile::TempDir::new().unwrap();
            let ctx = ResolutionContext::new(local.path().to_path_buf())
                .with_home_dir(home.path().to_path_buf())
                .with_ctx_value("agent", json!("claude"));

            std::fs::create_dir_all(home.path().join(".claude/skills/user-skill")).unwrap();
            std::fs::create_dir_all(local.path().join(".claude/skills/local-skill")).unwrap();

            assert_eq!(
                has_skill_fn(&[json!("user-skill")], &ctx).unwrap(),
                json!(true)
            );
            assert_eq!(
                has_skill_fn(&[json!("local-skill")], &ctx).unwrap(),
                json!(true)
            );
        }

        #[test]
        fn has_skill_finds_cross_agent_local_roots() {
            // Local-scoped roots are shared across all recognized agents: a skill
            // placed in any agent's local directory is discoverable by any other
            // recognized agent via both `has_skill` and `has_local_skill`.
            let home = tempfile::TempDir::new().unwrap();
            let local = tempfile::TempDir::new().unwrap();

            std::fs::create_dir_all(local.path().join(".opencode/skill/cross-skill")).unwrap();
            std::fs::create_dir_all(local.path().join(".claude/skills/cross-skill")).unwrap();

            let claude_ctx = ResolutionContext::new(local.path().to_path_buf())
                .with_home_dir(home.path().to_path_buf())
                .with_ctx_value("agent", json!("claude"));
            assert_eq!(
                has_skill_fn(&[json!("cross-skill")], &claude_ctx).unwrap(),
                json!(true)
            );
            assert_eq!(
                has_local_skill_fn(&[json!("cross-skill")], &claude_ctx).unwrap(),
                json!(true)
            );

            let opencode_ctx = ResolutionContext::new(local.path().to_path_buf())
                .with_home_dir(home.path().to_path_buf())
                .with_ctx_value("agent", json!("opencode"));
            assert_eq!(
                has_skill_fn(&[json!("cross-skill")], &opencode_ctx).unwrap(),
                json!(true)
            );
            assert_eq!(
                has_local_skill_fn(&[json!("cross-skill")], &opencode_ctx).unwrap(),
                json!(true)
            );
        }

        #[test]
        fn has_local_skill_excludes_user_roots() {
            let home = tempfile::TempDir::new().unwrap();
            let local = tempfile::TempDir::new().unwrap();
            let ctx = ResolutionContext::new(local.path().to_path_buf())
                .with_home_dir(home.path().to_path_buf())
                .with_ctx_value("agent", json!("claude"));

            std::fs::create_dir_all(home.path().join(".claude/skills/user-only")).unwrap();
            std::fs::create_dir_all(local.path().join(".claude/skills/local-only")).unwrap();

            assert_eq!(
                has_local_skill_fn(&[json!("user-only")], &ctx).unwrap(),
                json!(false)
            );
            assert_eq!(
                has_local_skill_fn(&[json!("local-only")], &ctx).unwrap(),
                json!(true)
            );
        }

        #[test]
        fn has_skill_uses_env_agent_when_ctx_not_set() {
            let local = tempfile::TempDir::new().unwrap();
            let ctx = ResolutionContext::new(local.path().to_path_buf())
                .with_home_dir(local.path().to_path_buf());

            // Unknown agent: only .agents/skills and .codex/skills are searched.
            std::fs::create_dir_all(local.path().join(".agents/skills/env-skill")).unwrap();

            assert_eq!(
                has_skill_fn(&[json!("env-skill")], &ctx).unwrap(),
                json!(true)
            );
        }

        #[test]
        fn has_skill_rejects_path_separators_and_dotdot() {
            let ctx = ResolutionContext::new(std::env::temp_dir());
            assert!(has_skill_fn(&[json!("foo/bar")], &ctx).is_err());
            assert!(has_skill_fn(&[json!("..")], &ctx).is_err());
            assert!(has_skill_fn(&[json!(".")], &ctx).is_err());
        }

        #[test]
        fn has_skill_nested_directory_does_not_count() {
            let local = tempfile::TempDir::new().unwrap();
            let ctx = ResolutionContext::new(local.path().to_path_buf())
                .with_home_dir(local.path().to_path_buf())
                .with_ctx_value("agent", json!("claude"));

            std::fs::create_dir_all(local.path().join(".claude/skills/parent/nested")).unwrap();

            assert_eq!(
                has_skill_fn(&[json!("parent")], &ctx).unwrap(),
                json!(true)
            );
            assert_eq!(
                has_skill_fn(&[json!("nested")], &ctx).unwrap(),
                json!(false)
            );
        }

        #[test]
        fn has_skill_missing_root_returns_false() {
            let local = tempfile::TempDir::new().unwrap();
            let ctx = ResolutionContext::new(local.path().to_path_buf())
                .with_home_dir(local.path().to_path_buf())
                .with_ctx_value("agent", json!("claude"));

            assert_eq!(
                has_skill_fn(&[json!("missing")], &ctx).unwrap(),
                json!(false)
            );
        }

        #[test]
        fn has_skill_null_propagates_and_arity_errors() {
            let ctx = ResolutionContext::new(std::env::temp_dir());
            assert_eq!(
                has_skill_fn(&[json!(null)], &ctx).unwrap(),
                json!(null)
            );
            assert_eq!(
                has_local_skill_fn(&[json!(null)], &ctx).unwrap(),
                json!(null)
            );
            assert!(has_skill_fn(&[], &ctx).is_err());
            assert!(has_skill_fn(&[json!("a"), json!("b")], &ctx).is_err());
        }

        #[test]
        fn phase5_functions_dispatch_by_name() {
            let ctx = ResolutionContext::new(std::env::temp_dir());
            assert_eq!(
                dispatch_fs("link", &[json!("https://example.com"), json!("x")], &ctx)
                    .unwrap()
                    .unwrap(),
                json!("[x](https://example.com)")
            );
            assert_eq!(
                dispatch_fs("hasskill", &[json!("missing")], &ctx)
                    .unwrap()
                    .unwrap(),
                json!(false)
            );
            assert_eq!(
                dispatch_fs("haslocalskill", &[json!("missing")], &ctx)
                    .unwrap()
                    .unwrap(),
                json!(false)
            );
        }
    }
}

#[cfg(test)]
mod fn_remote_tests {
    use super::*;
    use crate::markdown::compose::remote_fetch::RemoteFetchRuntime;
    use biscuit_file::file_reference::fetch::FetchPolicy;
    use serde_json::json;
    use std::time::Duration;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// Builds a context whose remote-fetch runtime has already fetched `url`.
    async fn ready_ctx(url: &str, allow: bool) -> ResolutionContext {
        let policy = if allow {
            FetchPolicy::deny_all().allow_host("127.0.0.1")
        } else {
            FetchPolicy::deny_all()
        };
        let rt = RemoteFetchRuntime::with_policy(policy);
        rt.register_and_fetch(url::Url::parse(url).unwrap());
        // Allow the eager fetch task to settle before reading point-of-use.
        tokio::time::sleep(Duration::from_millis(200)).await;
        ResolutionContext {
            remote_fetch: Some(rt),
            ..ResolutionContext::new(std::path::PathBuf::from("."))
        }
    }

    #[tokio::test]
    async fn frontmatter_title_and_exists_read_remote_url() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/doc.md"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                "---\ntitle: Remote Title\nstatus: draft\n---\n# H1\n\nBody\n",
            ))
            .mount(&server)
            .await;
        let url = format!("{}/doc.md", server.uri());
        let ctx = ready_ctx(&url, true).await;

        assert_eq!(
            frontmatter_fn(&[json!(url.clone()), json!("status")], &ctx).unwrap(),
            json!("draft")
        );
        assert_eq!(
            markdown_title_fn(&[json!(url.clone())], &ctx).unwrap(),
            json!("Remote Title")
        );
        assert_eq!(
            markdown_body_empty_fn(&[json!(url.clone())], &ctx).unwrap(),
            json!(false)
        );
        assert_eq!(file_exists_fn(&[json!(url)], &ctx).unwrap(), json!(true));
    }

    #[tokio::test]
    async fn file_exists_false_for_policy_denied_remote() {
        let server = MockServer::start().await;
        // No mock mounted and a deny-all policy: the fetch never reaches the
        // network, so the URL is treated as non-existent.
        let url = format!("{}/blocked.md", server.uri());
        let ctx = ready_ctx(&url, false).await;
        assert_eq!(file_exists_fn(&[json!(url)], &ctx).unwrap(), json!(false));
    }

    #[test]
    fn file_exists_remote_url_fails_loudly_in_local_only_context() {
        // Frontmatter's resolution context carries no remote runtime, so a
        // remote URL argument is unreadable and must error rather than
        // silently reporting the URL as absent (Decision B).
        let ctx = ResolutionContext::new(std::path::PathBuf::from("."));
        let err = file_exists_fn(&[json!("https://example.com/doc.md")], &ctx)
            .expect_err("local-only remote URL must fail loudly");
        assert!(err.contains("local-only"), "unexpected message: {err}");
    }

    #[test]
    fn load_markdown_remote_url_fails_loudly_in_local_only_context() {
        // The document-reading functions share the same local-only contract.
        let ctx = ResolutionContext::new(std::path::PathBuf::from("."));
        let err = markdown_title_fn(&[json!("https://example.com/doc.md")], &ctx)
            .expect_err("local-only remote URL must fail loudly");
        assert!(
            err.contains("remote reads are not enabled"),
            "unexpected message: {err}"
        );
    }
}

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
    fn display_path_with_forward_slashes_normalizes_separators() {
        assert_eq!(
            display_path_with_forward_slashes(Path::new("foo/bar/baz.md")),
            "foo/bar/baz.md"
        );
        assert_eq!(
            display_path_with_forward_slashes(Path::new("/tmp/foo/bar.md")),
            "/tmp/foo/bar.md"
        );
        // Backslashes are normalized even if the running platform does not use
        // them as separators, so composed Markdown stays portable.
        assert_eq!(
            display_path_with_forward_slashes(Path::new("foo\\bar\\baz.md")),
            "foo/bar/baz.md"
        );
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
