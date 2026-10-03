//! THROWAWAY SPIKE: the single value-rule core.
//!
//! One rule list, one binder. It knows nothing about JSON Schema or the
//! SimplifiedSchema grammar: both surfaces reach it through a translator that
//! produces a [`Shape`]. Formats and validate-only residual keywords are the
//! only things it delegates, through [`Checks`].

use std::fmt;
use std::sync::Arc;

use serde_json::{Map, Number, Value};

/// A compiled regex, linear or backtracking (lookaround patterns).
#[derive(Clone)]
pub enum Pattern {
    Linear(regex::Regex),
    Fancy(fancy_regex::Regex),
}

impl Pattern {
    pub fn new(src: &str) -> Option<Self> {
        if src.contains("(?!") || src.contains("(?=") || src.contains("(?<") {
            fancy_regex::Regex::new(src).ok().map(Pattern::Fancy)
        } else {
            regex::Regex::new(src).ok().map(Pattern::Linear)
        }
    }
    pub fn is_match(&self, s: &str) -> bool {
        match self {
            Pattern::Linear(r) => r.is_match(s),
            Pattern::Fancy(r) => r.is_match(s).unwrap_or(false),
        }
    }
}

impl fmt::Debug for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Pattern::Linear(r) => write!(f, "/{}/", r.as_str()),
            Pattern::Fancy(r) => write!(f, "/{}/", r.as_str()),
        }
    }
}

/// A validate-only check for keywords the translator does not model.
/// It never converts; it only judges the (possibly converted) final value.
#[derive(Clone)]
pub struct Residual {
    pub label: String,
    pub check: Arc<dyn Fn(&Value) -> bool + Send + Sync>,
}

impl fmt::Debug for Residual {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Residual({})", self.label)
    }
}

/// Delegated checks: formats (date, email, uri, darkmatter-*).
pub trait Checks {
    fn format(&self, format: &str, text: &str) -> bool;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextKind {
    Plain,
    Json,
    Yaml,
}

#[derive(Debug, Clone)]
pub struct TextShape {
    pub kind: TextKind,
    pub format: Option<String>,
    pub min_len: Option<usize>,
    pub max_len: Option<usize>,
    pub pattern: Option<Pattern>,
    /// `enum(...)` members or a string literal.
    pub one_of: Option<Vec<String>>,
    pub schemes: Option<Vec<String>>,
}

impl TextShape {
    pub fn plain() -> Self {
        Self {
            kind: TextKind::Plain,
            format: None,
            min_len: None,
            max_len: None,
            pattern: None,
            one_of: None,
            schemes: None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct NumberShape {
    pub integer: bool,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub exclusive_min: Option<f64>,
    pub exclusive_max: Option<f64>,
    /// Numeric literal / enum members.
    pub one_of: Option<Vec<Value>>,
}

#[derive(Debug, Clone)]
pub enum Additional {
    Allow,
    Deny,
    Shape(Box<Shape>),
}

#[derive(Debug, Clone)]
pub struct ObjectShape {
    pub props: Vec<(String, Shape)>,
    pub required: Vec<String>,
    pub patterns: Vec<(Pattern, Shape)>,
    pub additional: Additional,
    pub min_props: Option<usize>,
    pub max_props: Option<usize>,
}

impl ObjectShape {
    pub fn opaque() -> Self {
        Self {
            props: Vec::new(),
            required: Vec::new(),
            patterns: Vec::new(),
            additional: Additional::Allow,
            min_props: None,
            max_props: None,
        }
    }
}

/// The neutral target shape both translators produce.
#[derive(Debug, Clone)]
pub enum Shape {
    Any,
    Null,
    Boolean { literal: Option<bool> },
    Number(NumberShape),
    NumberLike,
    Boolish,
    Text(TextShape),
    List {
        items: Box<Shape>,
        min_items: Option<usize>,
        max_items: Option<usize>,
        unique: bool,
    },
    Tuple {
        prefix: Vec<Shape>,
        required: usize,
        /// `None` = closed (fixed length), `Some` = spread element.
        rest: Option<Box<Shape>>,
    },
    Object(ObjectShape),
    Union(Vec<Shape>),
    /// Untranslatable fragment: validate-only, never converts.
    Opaque(Residual),
    /// A translated shape plus sibling keywords checked after conversion.
    WithResidual(Box<Shape>, Residual),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Conv {
    TextToNumber,
    TextToBoolean,
    ToText,
    ToJson,
    ToYaml,
}

/// A successful binding. `convs` empty means the value was kept unchanged.
#[derive(Debug, Clone)]
pub struct Bound {
    pub value: Value,
    pub convs: Vec<(String, Conv)>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Reason {
    NoConversion { expected: &'static str },
    ConversionFailed { expected: &'static str },
    OutOfRange,
    Constraint(String),
    MissingRequired,
    NotAllowed,
    AmbiguousUnion { readings: Vec<Value> },
    NoUnionOption,
    Residual(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeError {
    pub path: String,
    pub reason: Reason,
}

pub type BindResult = Result<Bound, Vec<TypeError>>;

// ── scalar rules ────────────────────────────────────────────────────────

#[derive(Debug, PartialEq)]
pub enum NumText {
    NotNumber,
    OutOfRange,
}

/// Text → number under the spec grammar. Whole-number text (no `.`, no
/// exponent) becomes an exact i64/u64 or is out of range.
pub fn parse_number_text(s: &str) -> Result<Number, NumText> {
    let t = s.trim();
    let b = t.as_bytes();
    let mut i = 0;
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        i += 1;
    }
    // digits with single underscores strictly between digits
    fn digits(b: &[u8], mut i: usize) -> (usize, usize) {
        let start = i;
        let mut n = 0;
        while i < b.len() {
            if b[i].is_ascii_digit() {
                n += 1;
                i += 1;
            } else if b[i] == b'_' && i > start && b[i - 1].is_ascii_digit() && i + 1 < b.len() && b[i + 1].is_ascii_digit() {
                i += 1;
            } else {
                break;
            }
        }
        (i, n)
    }
    let (after_int, int_digits) = digits(b, i);
    i = after_int;
    let mut frac_digits = 0;
    let mut has_dot = false;
    if i < b.len() && b[i] == b'.' {
        has_dot = true;
        let (after_frac, n) = digits(b, i + 1);
        frac_digits = n;
        i = after_frac;
        if n == 0 {
            return Err(NumText::NotNumber); // trailing dot "5."
        }
    }
    if int_digits == 0 && frac_digits == 0 {
        return Err(NumText::NotNumber);
    }
    let mut has_exp = false;
    if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
        has_exp = true;
        i += 1;
        if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
            i += 1;
        }
        let s0 = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        if i == s0 {
            return Err(NumText::NotNumber);
        }
    }
    if i != b.len() {
        return Err(NumText::NotNumber);
    }
    let cleaned: String = t.chars().filter(|c| *c != '_').collect();
    let cleaned = cleaned.strip_prefix('+').unwrap_or(&cleaned).to_string();
    if !has_dot && !has_exp {
        if let Ok(v) = cleaned.parse::<i64>() {
            return Ok(Number::from(v));
        }
        if let Ok(v) = cleaned.parse::<u64>() {
            return Ok(Number::from(v));
        }
        return Err(NumText::OutOfRange);
    }
    let f: f64 = cleaned.parse().map_err(|_| NumText::NotNumber)?;
    if !f.is_finite() {
        return Err(NumText::OutOfRange);
    }
    let mantissa_nonzero = cleaned
        .split(['e', 'E'])
        .next()
        .is_some_and(|m| m.bytes().any(|c| (b'1'..=b'9').contains(&c)));
    if f == 0.0 && mantissa_nonzero {
        return Err(NumText::OutOfRange); // underflow
    }
    Number::from_f64(f).ok_or(NumText::OutOfRange)
}

/// Text → boolean: the six words, any case, trimmed.
pub fn parse_boolean_text(s: &str) -> Option<bool> {
    match s.trim().to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" | "1" => Some(true),
        "false" | "no" | "off" | "0" => Some(false),
        _ => None,
    }
}

fn canonical_text(v: &Value) -> Option<String> {
    match v {
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

fn err(path: &str, reason: Reason) -> Vec<TypeError> {
    vec![TypeError {
        path: path.to_string(),
        reason,
    }]
}

fn kept(v: &Value) -> Bound {
    Bound {
        value: v.clone(),
        convs: Vec::new(),
    }
}

fn converted(v: Value, path: &str, c: Conv) -> Bound {
    Bound {
        value: v,
        convs: vec![(path.to_string(), c)],
    }
}

fn check_number(ns: &NumberShape, n: &Number, path: &str) -> Result<(), Vec<TypeError>> {
    let f = n.as_f64().unwrap_or(f64::NAN);
    if ns.integer && !(n.is_i64() || n.is_u64() || f.fract() == 0.0) {
        return Err(err(path, Reason::Constraint("integer".into())));
    }
    if let Some(m) = ns.min
        && f < m
    {
        return Err(err(path, Reason::Constraint(format!("min({m})"))));
    }
    if let Some(m) = ns.max
        && f > m
    {
        return Err(err(path, Reason::Constraint(format!("max({m})"))));
    }
    if let Some(m) = ns.exclusive_min
        && f <= m
    {
        return Err(err(path, Reason::Constraint(format!("exclusiveMinimum({m})"))));
    }
    if let Some(m) = ns.exclusive_max
        && f >= m
    {
        return Err(err(path, Reason::Constraint(format!("exclusiveMaximum({m})"))));
    }
    if let Some(members) = &ns.one_of
        && !members.iter().any(|m| m.as_f64() == Some(f))
    {
        return Err(err(path, Reason::Constraint("literal/enum".into())));
    }
    Ok(())
}

fn check_text(ts: &TextShape, s: &str, path: &str, checks: &dyn Checks) -> Result<(), Vec<TypeError>> {
    let len = s.chars().count();
    if let Some(m) = ts.min_len
        && len < m
    {
        return Err(err(path, Reason::Constraint(format!("minLength({m})"))));
    }
    if let Some(m) = ts.max_len
        && len > m
    {
        return Err(err(path, Reason::Constraint(format!("maxLength({m})"))));
    }
    if let Some(p) = &ts.pattern
        && !p.is_match(s)
    {
        return Err(err(path, Reason::Constraint(format!("pattern {p:?}"))));
    }
    if let Some(members) = &ts.one_of
        && !members.iter().any(|m| m == s)
    {
        return Err(err(path, Reason::Constraint("enum/literal".into())));
    }
    if let Some(f) = &ts.format
        && !checks.format(f, s)
    {
        return Err(err(path, Reason::Constraint(format!("format {f}"))));
    }
    if let Some(schemes) = &ts.schemes {
        let scheme = s.split_once(':').map(|(a, _)| a.to_ascii_lowercase());
        if !scheme.is_some_and(|sc| schemes.iter().any(|x| x.eq_ignore_ascii_case(&sc))) {
            return Err(err(path, Reason::Constraint("scheme".into())));
        }
    }
    Ok(())
}

fn child(path: &str, seg: &str) -> String {
    format!("{path}/{}", seg.replace('~', "~0").replace('/', "~1"))
}

// ── the binder ───────────────────────────────────────────────────────────

/// Binds `value` to `shape`: keep, convert, or type error.
pub fn bind(shape: &Shape, value: &Value, path: &str, checks: &dyn Checks) -> BindResult {
    match shape {
        Shape::Any => Ok(kept(value)),
        Shape::Null => {
            if value.is_null() {
                Ok(kept(value))
            } else {
                Err(err(path, Reason::NoConversion { expected: "null" }))
            }
        }
        Shape::Boolean { literal } => {
            let (b, bound) = match value {
                Value::Bool(b) => (*b, kept(value)),
                Value::String(s) => match parse_boolean_text(s) {
                    Some(b) => (b, converted(Value::Bool(b), path, Conv::TextToBoolean)),
                    None => return Err(err(path, Reason::ConversionFailed { expected: "boolean" })),
                },
                _ => return Err(err(path, Reason::NoConversion { expected: "boolean" })),
            };
            if let Some(l) = literal
                && *l != b
            {
                return Err(err(path, Reason::Constraint("literal".into())));
            }
            Ok(bound)
        }
        Shape::Number(ns) => {
            let (n, bound) = match value {
                Value::Number(n) => (n.clone(), kept(value)),
                Value::String(s) => match parse_number_text(s) {
                    Ok(n) => (n.clone(), converted(Value::Number(n), path, Conv::TextToNumber)),
                    Err(NumText::OutOfRange) => return Err(err(path, Reason::OutOfRange)),
                    Err(NumText::NotNumber) => {
                        return Err(err(path, Reason::ConversionFailed { expected: "number" }));
                    }
                },
                _ => return Err(err(path, Reason::NoConversion { expected: "number" })),
            };
            check_number(ns, &n, path)?;
            Ok(bound)
        }
        Shape::NumberLike => match value {
            Value::Number(_) => Ok(kept(value)),
            Value::String(s) => match parse_number_text(s) {
                Ok(_) => Ok(kept(value)),
                Err(NumText::OutOfRange) => Err(err(path, Reason::OutOfRange)),
                Err(NumText::NotNumber) => Err(err(path, Reason::ConversionFailed { expected: "numberlike" })),
            },
            _ => Err(err(path, Reason::NoConversion { expected: "numberlike" })),
        },
        Shape::Boolish => match value {
            Value::Bool(_) => Ok(kept(value)),
            Value::String(s) if parse_boolean_text(s).is_some() => Ok(kept(value)),
            Value::String(_) => Err(err(path, Reason::ConversionFailed { expected: "boolish" })),
            _ => Err(err(path, Reason::NoConversion { expected: "boolish" })),
        },
        Shape::Text(ts) => {
            let bound = match (ts.kind, value) {
                (_, Value::String(_)) => kept(value),
                (TextKind::Plain, v) => match canonical_text(v) {
                    Some(t) => converted(Value::String(t), path, Conv::ToText),
                    None => return Err(err(path, Reason::NoConversion { expected: "text" })),
                },
                (TextKind::Json, Value::Null) | (TextKind::Yaml, Value::Null) => {
                    return Err(err(path, Reason::NoConversion { expected: "json/yaml text" }));
                }
                (TextKind::Json, v) => match serde_json::to_string(v) {
                    Ok(t) => converted(Value::String(t), path, Conv::ToJson),
                    Err(_) => return Err(err(path, Reason::ConversionFailed { expected: "json" })),
                },
                (TextKind::Yaml, v) => match serde_yaml_ng::to_string(v) {
                    Ok(t) => converted(
                        Value::String(t.trim_end_matches('\n').to_string()),
                        path,
                        Conv::ToYaml,
                    ),
                    Err(_) => return Err(err(path, Reason::ConversionFailed { expected: "yaml" })),
                },
            };
            let s = bound.value.as_str().unwrap_or_default();
            check_text(ts, s, path, checks)?;
            Ok(bound)
        }
        Shape::List {
            items,
            min_items,
            max_items,
            unique,
        } => {
            let Value::Array(elems) = value else {
                return Err(err(path, Reason::NoConversion { expected: "list" }));
            };
            let mut out = Vec::with_capacity(elems.len());
            let mut convs = Vec::new();
            let mut errors = Vec::new();
            for (i, e) in elems.iter().enumerate() {
                match bind(items, e, &child(path, &i.to_string()), checks) {
                    Ok(b) => {
                        out.push(b.value);
                        convs.extend(b.convs);
                    }
                    Err(es) => errors.extend(es),
                }
            }
            if !errors.is_empty() {
                return Err(errors);
            }
            if let Some(m) = min_items
                && out.len() < *m
            {
                return Err(err(path, Reason::Constraint(format!("minItems({m})"))));
            }
            if let Some(m) = max_items
                && out.len() > *m
            {
                return Err(err(path, Reason::Constraint(format!("maxItems({m})"))));
            }
            if *unique {
                for i in 0..out.len() {
                    for j in 0..i {
                        if out[i] == out[j] {
                            return Err(err(path, Reason::Constraint("unique".into())));
                        }
                    }
                }
            }
            Ok(Bound {
                value: Value::Array(out),
                convs,
            })
        }
        Shape::Tuple {
            prefix,
            required,
            rest,
        } => {
            let Value::Array(elems) = value else {
                return Err(err(path, Reason::NoConversion { expected: "tuple" }));
            };
            let mut errors = Vec::new();
            if elems.len() < *required {
                errors.push(TypeError {
                    path: child(path, &elems.len().to_string()),
                    reason: Reason::MissingRequired,
                });
            }
            if rest.is_none() && elems.len() > prefix.len() {
                errors.push(TypeError {
                    path: child(path, &prefix.len().to_string()),
                    reason: Reason::NotAllowed,
                });
            }
            let mut out = Vec::with_capacity(elems.len());
            let mut convs = Vec::new();
            for (i, e) in elems.iter().enumerate() {
                let slot = prefix.get(i).or(rest.as_deref());
                let Some(slot) = slot else { break };
                match bind(slot, e, &child(path, &i.to_string()), checks) {
                    Ok(b) => {
                        out.push(b.value);
                        convs.extend(b.convs);
                    }
                    Err(es) => errors.extend(es),
                }
            }
            if !errors.is_empty() {
                return Err(errors);
            }
            Ok(Bound {
                value: Value::Array(out),
                convs,
            })
        }
        Shape::Object(os) => bind_object(os, value, path, checks),
        Shape::Union(options) => bind_union(options, value, path, checks),
        Shape::Opaque(r) => {
            if (r.check)(value) {
                Ok(kept(value))
            } else {
                Err(err(path, Reason::Residual(r.label.clone())))
            }
        }
        Shape::WithResidual(inner, r) => {
            let b = bind(inner, value, path, checks)?;
            if (r.check)(&b.value) {
                Ok(b)
            } else {
                Err(err(path, Reason::Residual(r.label.clone())))
            }
        }
    }
}

fn bind_object(os: &ObjectShape, value: &Value, path: &str, checks: &dyn Checks) -> BindResult {
    let Value::Object(map) = value else {
        return Err(err(path, Reason::NoConversion { expected: "object" }));
    };
    let mut out = Map::new();
    let mut convs = Vec::new();
    let mut errors = Vec::new();
    for (k, v) in map {
        let p = child(path, k);
        let declared = os.props.iter().find(|(name, _)| name == k).map(|(_, s)| s);
        let result = if let Some(s) = declared {
            bind(s, v, &p, checks)
        } else {
            let matching: Vec<&Shape> = os
                .patterns
                .iter()
                .filter(|(re, _)| re.is_match(k))
                .map(|(_, s)| s)
                .collect();
            if let Some(first) = matching.first() {
                // JSON Schema applies every matching pattern; bind to the
                // first and require the rest to accept the result unchanged.
                bind(first, v, &p, checks).and_then(|b| {
                    for other in &matching[1..] {
                        let again = bind(other, &b.value, &p, checks)?;
                        if !again.convs.is_empty() {
                            return Err(err(&p, Reason::Constraint("overlapping patternProperties".into())));
                        }
                    }
                    Ok(b)
                })
            } else {
                match &os.additional {
                    Additional::Allow => Ok(kept(v)),
                    Additional::Deny => Err(err(&p, Reason::NotAllowed)),
                    Additional::Shape(s) => bind(s, v, &p, checks),
                }
            }
        };
        match result {
            Ok(b) => {
                out.insert(k.clone(), b.value);
                convs.extend(b.convs);
            }
            Err(es) => errors.extend(es),
        }
    }
    for r in &os.required {
        if !map.contains_key(r) {
            errors.push(TypeError {
                path: child(path, r),
                reason: Reason::MissingRequired,
            });
        }
    }
    if let Some(m) = os.min_props
        && map.len() < m
    {
        errors.push(TypeError {
            path: path.to_string(),
            reason: Reason::Constraint(format!("minProperties({m})")),
        });
    }
    if let Some(m) = os.max_props
        && map.len() > m
    {
        errors.push(TypeError {
            path: path.to_string(),
            reason: Reason::Constraint(format!("maxProperties({m})")),
        });
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(Bound {
        value: Value::Object(out),
        convs,
    })
}

/// Union rule: exact match; else the single reachable reading; else the two
/// tie-breakers (number over boolean, JSON over YAML) applied path by path;
/// any other tie is an ambiguity error. Option order never matters.
fn bind_union(options: &[Shape], value: &Value, path: &str, checks: &dyn Checks) -> BindResult {
    let mut readings: Vec<Bound> = Vec::new();
    let mut errors = Vec::new();
    for o in options {
        match bind(o, value, path, checks) {
            Ok(b) if b.convs.is_empty() => return Ok(kept(value)),
            Ok(b) => {
                if !readings.iter().any(|r| r.value == b.value) {
                    readings.push(b);
                }
            }
            Err(es) => errors.push(es),
        }
    }
    match readings.len() {
        0 => {
            // A non-null value against `T | null`: report T's own error.
            let mut informative: Vec<Vec<TypeError>> = errors
                .into_iter()
                .filter(|es| {
                    !(es.len() == 1 && es[0].reason == Reason::NoConversion { expected: "null" })
                })
                .collect();
            if informative.len() == 1 {
                Err(informative.pop().expect("one"))
            } else {
                Err(err(path, Reason::NoUnionOption))
            }
        }
        1 => Ok(readings.pop().expect("one reading")),
        _ => {
            let winner = (0..readings.len()).find(|&i| {
                (0..readings.len()).all(|j| i == j || beats(&readings[i], &readings[j], path))
            });
            match winner {
                Some(i) => Ok(readings.swap_remove(i)),
                None => Err(err(
                    path,
                    Reason::AmbiguousUnion {
                        readings: readings.into_iter().map(|r| r.value).collect(),
                    },
                )),
            }
        }
    }
}

/// True when `a` is preferred over `b` at every path where they differ.
fn beats(a: &Bound, b: &Bound, base: &str) -> bool {
    let mut diffs = Vec::new();
    diff_paths(&a.value, &b.value, base.to_string(), &mut diffs);
    !diffs.is_empty()
        && diffs.iter().all(|p| {
            matches!(
                (conv_at(a, p), conv_at(b, p)),
                (Some(Conv::TextToNumber), Some(Conv::TextToBoolean))
                    | (Some(Conv::ToJson), Some(Conv::ToYaml))
            )
        })
}

fn conv_at(b: &Bound, abs: &str) -> Option<Conv> {
    b.convs.iter().find(|(p, _)| p == abs).map(|(_, c)| *c)
}

fn diff_paths(a: &Value, b: &Value, at: String, out: &mut Vec<String>) {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            let mut keys: Vec<&String> = x.keys().chain(y.keys()).collect();
            keys.sort();
            keys.dedup();
            for k in keys {
                match (x.get(k), y.get(k)) {
                    (Some(va), Some(vb)) => diff_paths(va, vb, child(&at, k), out),
                    _ => out.push(child(&at, k)),
                }
            }
        }
        (Value::Array(x), Value::Array(y)) if x.len() == y.len() => {
            for (i, (va, vb)) in x.iter().zip(y).enumerate() {
                diff_paths(va, vb, child(&at, &i.to_string()), out);
            }
        }
        _ => {
            if a != b {
                out.push(at);
            }
        }
    }
}

/// Root binding with compose's shell-pending keys treated as `any`.
pub fn bind_root(shape: &Shape, value: &Value, pending: &[String], checks: &dyn Checks) -> BindResult {
    if pending.is_empty() {
        return bind(shape, value, "", checks);
    }
    bind(&defer_pending(shape, pending), value, "", checks)
}

fn defer_pending(shape: &Shape, pending: &[String]) -> Shape {
    match shape {
        Shape::Object(os) => {
            let mut os = os.clone();
            for (name, s) in &mut os.props {
                if pending.contains(name) {
                    *s = Shape::Any;
                }
            }
            Shape::Object(os)
        }
        Shape::Union(arms) => Shape::Union(arms.iter().map(|a| defer_pending(a, pending)).collect()),
        other => other.clone(),
    }
}
