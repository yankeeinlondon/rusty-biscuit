//! THROWAWAY SPIKE: stand-in binder for the draft `function(...)` signatures.
//!
//! Parses just enough of the SimplifiedSchema function grammar to predict what
//! the spec's coercion engine (2026-09-21-schema-enhancements, "The Coercion
//! Engine") would do with one runtime argument vector. Not production code.

use serde_json::{Map, Number, Value};

#[derive(Debug, Clone, PartialEq)]
pub enum Ty {
    Any,
    Null,
    Void,
    Str { refined: Option<String>, min_len: Option<usize>, max_len: Option<usize> },
    Enum(Vec<String>),
    Num { integer: bool, min: Option<f64>, max: Option<f64> },
    Bool,
    NumberLike,
    Boolish,
    Object,
    Json,
    Yaml,
    Array(Box<Ty>),
    Union(Vec<Ty>),
    Unknown(String),
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: Ty,
    pub optional: bool,
    pub rest: bool,
}

#[derive(Debug, Clone)]
pub struct Signature {
    pub text: String,
    pub params: Vec<Param>,
    /// Grammar notes: normalizations applied (old `(optional)` form etc.).
    pub notes: Vec<String>,
}

impl Signature {
    pub fn min_arity(&self) -> usize {
        self.params.iter().filter(|p| !p.optional && !p.rest).count()
    }
    pub fn max_arity(&self) -> Option<usize> {
        if self.params.iter().any(|p| p.rest) { None } else { Some(self.params.len()) }
    }
}

// ---------- parsing ----------

/// Extracts the text inside `parameters( ... )` from a `function(...)` body.
pub fn parse_function(text: &str) -> Result<Signature, String> {
    let start = text.find("parameters(").ok_or("no parameters(...)")? + "parameters(".len();
    let inner = balanced(&text[start..]).ok_or("unbalanced parameters(...)")?;
    let mut notes = Vec::new();
    let mut params = Vec::new();
    let trimmed = inner.trim();
    if trimmed.is_empty() {
        notes.push("parameters() normalized to parameters(void)".into());
    } else if trimmed != "void" {
        for raw in split_top(trimmed, ',') {
            let raw = raw.trim();
            let (lhs, rhs) = raw.split_once(':').ok_or_else(|| format!("param without type: {raw}"))?;
            let mut name = lhs.trim().to_string();
            let mut optional = false;
            let mut rest = false;
            if let Some(n) = name.strip_prefix("...") { name = n.to_string(); rest = true; }
            if let Some(n) = name.strip_suffix('?') { name = n.to_string(); optional = true; }
            let mut ty_text = rhs.trim().to_string();
            // Old draft form `number(optional)` / `string(optional)`.
            if ty_text.contains("(optional)") {
                ty_text = ty_text.replace("(optional)", "");
                optional = true;
                notes.push(format!("{name}: `type(optional)` normalized to `{name}?:`"));
            }
            let ty = parse_type(&ty_text)?;
            params.push(Param { name, ty, optional, rest });
        }
    }
    Ok(Signature { text: text.split_whitespace().collect::<Vec<_>>().join(" "), params, notes })
}

fn balanced(s: &str) -> Option<&str> {
    let mut depth = 1i32;
    let mut in_str = false;
    for (i, c) in s.char_indices() {
        match c {
            '"' => in_str = !in_str,
            '(' | '[' | '{' if !in_str => depth += 1,
            ')' | ']' | '}' if !in_str => {
                depth -= 1;
                if depth == 0 { return Some(&s[..i]); }
            }
            _ => {}
        }
    }
    None
}

fn split_top(s: &str, sep: char) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut in_str = false;
    let mut cur = String::new();
    for c in s.chars() {
        match c {
            '"' => { in_str = !in_str; cur.push(c); }
            '(' | '[' | '{' if !in_str => { depth += 1; cur.push(c); }
            ')' | ']' | '}' if !in_str => { depth -= 1; cur.push(c); }
            c if c == sep && depth == 0 && !in_str => { out.push(std::mem::take(&mut cur)); }
            _ => cur.push(c),
        }
    }
    if !cur.trim().is_empty() { out.push(cur); }
    out
}

pub fn parse_type(text: &str) -> Result<Ty, String> {
    let text = text.trim();
    let options = split_top(text, '|');
    if options.len() > 1 {
        return Ok(Ty::Union(options.iter().map(|o| parse_type(o)).collect::<Result<_, _>>()?));
    }
    if let Some(inner) = text.strip_suffix("[]") {
        return Ok(Ty::Array(Box::new(parse_type(inner)?)));
    }
    if text.starts_with('(') && text.ends_with(')') {
        return parse_type(&text[1..text.len() - 1]);
    }
    let (head, args) = match text.find('(') {
        Some(i) if text.ends_with(')') => (&text[..i], Some(&text[i + 1..text.len() - 1])),
        _ => (text, None),
    };
    let constraints: Vec<String> = args.map(|a| split_top(a, ';').into_iter().map(|s| s.trim().to_string()).collect()).unwrap_or_default();
    let find = |name: &str| constraints.iter().find_map(|c| c.strip_prefix(&format!("{name}(")).and_then(|r| r.strip_suffix(')')).map(|s| s.trim().to_string()));
    Ok(match head.trim() {
        "any" => Ty::Any,
        "null" => Ty::Null,
        "void" => Ty::Void,
        "string" => Ty::Str { refined: None, min_len: find("min").and_then(|s| s.parse().ok()), max_len: find("max").and_then(|s| s.parse().ok()) },
        "file" | "date" | "datetime" | "url" | "email" | "time" | "ip-address" => Ty::Str { refined: Some(head.trim().to_string()), min_len: None, max_len: None },
        "number" => Ty::Num {
            integer: constraints.iter().any(|c| c == "integer"),
            min: find("min").and_then(|s| s.parse().ok()),
            max: find("max").and_then(|s| s.parse().ok()),
        },
        "boolean" => Ty::Bool,
        "numberlike" => Ty::NumberLike,
        "boolish" => Ty::Boolish,
        "object" => Ty::Object,
        "json" => Ty::Json,
        "yaml" => Ty::Yaml,
        "enum" => Ty::Enum(args.unwrap_or("").split(',').map(|s| s.trim().trim_matches('"').to_string()).collect()),
        other => Ty::Unknown(other.to_string()),
    })
}

// ---------- engine ----------

#[derive(Debug, Clone, PartialEq)]
pub enum Bind {
    Keep,
    Convert(Value, &'static str),
    Error(String),
}

/// Lenient text -> number per spec "Conversion Rules" / coercion-design
/// "Numeric and Boolean Boundaries".
pub fn text_to_number(s: &str) -> Result<Value, String> {
    let t = s.trim();
    if t.is_empty() { return Err("empty text is not a number".into()); }
    let b = t.as_bytes();
    let mut i = 0;
    if b[0] == b'+' || b[0] == b'-' { i = 1; }
    let body = &t[i..];
    // validate: digits with single underscores between digits, optional .digits, optional exponent
    let (mantissa, exponent) = match body.find(['e', 'E']) {
        Some(p) => (&body[..p], Some(&body[p + 1..])),
        None => (body, None),
    };
    let (int_part, frac_part) = match mantissa.split_once('.') {
        Some((a, b)) => (a, Some(b)),
        None => (mantissa, None),
    };
    let digits_ok = |d: &str, allow_empty: bool| -> bool {
        if d.is_empty() { return allow_empty; }
        let bytes = d.as_bytes();
        if bytes[0] == b'_' || bytes[bytes.len() - 1] == b'_' || d.contains("__") { return false; }
        d.chars().all(|c| c.is_ascii_digit() || c == '_')
    };
    if !digits_ok(int_part, frac_part.is_some()) { return Err(format!("text {s:?} is not a number")); }
    if let Some(f) = frac_part && (f.is_empty() || !digits_ok(f, false)) { return Err(format!("text {s:?} is not a number")); }
    if let Some(e) = exponent {
        let e2 = e.strip_prefix(['+', '-']).unwrap_or(e);
        if e2.is_empty() || !e2.chars().all(|c| c.is_ascii_digit()) { return Err(format!("text {s:?} is not a number")); }
    }
    let clean: String = t.chars().filter(|c| *c != '_').collect();
    let clean = clean.strip_prefix('+').unwrap_or(&clean).to_string();
    if frac_part.is_none() && exponent.is_none() {
        if let Ok(n) = clean.parse::<i64>() { return Ok(Value::Number(n.into())); }
        if let Ok(n) = clean.parse::<u64>() { return Ok(Value::Number(n.into())); }
        return Err(format!("whole-number text {s:?} is out of 64-bit range"));
    }
    let f: f64 = clean.parse().map_err(|_| format!("text {s:?} is not a number"))?;
    if !f.is_finite() { return Err(format!("text {s:?} is not finite")); }
    Number::from_f64(f).map(Value::Number).ok_or_else(|| format!("text {s:?} not representable"))
}

pub fn text_to_bool(s: &str) -> Option<bool> {
    match s.trim().to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" | "1" => Some(true),
        "false" | "no" | "off" | "0" => Some(false),
        _ => None,
    }
}

fn canonical_text(v: &Value) -> Option<String> {
    match v {
        Value::Bool(b) => Some(b.to_string()),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() { return Some(i.to_string()); }
            if let Some(u) = n.as_u64() { return Some(u.to_string()); }
            let f = n.as_f64()?;
            if f.fract() == 0.0 && f.abs() < 1e15 { Some(format!("{}", f as i64)) } else { Some(format!("{f}")) }
        }
        _ => None,
    }
}

fn type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "text",
        Value::Array(_) => "list",
        Value::Object(_) => "object",
    }
}

fn num_constraints(v: &Value, integer: bool, min: Option<f64>, max: Option<f64>) -> Result<(), String> {
    let f = v.as_f64().unwrap_or(f64::NAN);
    if integer && !(v.is_i64() || v.is_u64() || f.fract() == 0.0) { return Err(format!("{f} fails integer")); }
    if let Some(m) = min && f < m { return Err(format!("{f} fails min({m})")); }
    if let Some(m) = max && f > m { return Err(format!("{f} fails max({m})")); }
    Ok(())
}

/// Binds one value to one type. `Keep` means exact match.
pub fn bind(v: &Value, ty: &Ty) -> Bind {
    use Bind::*;
    match ty {
        Ty::Any => Keep,
        Ty::Void => Error("void parameter".into()),
        Ty::Unknown(t) => Error(format!("unknown type `{t}` in draft")),
        Ty::Null => if v.is_null() { Keep } else { Error(format!("expected null, got {}", type_name(v))) },
        Ty::Str { min_len, max_len, .. } => {
            let (s, action) = match v {
                Value::String(s) => (s.clone(), None),
                Value::Number(_) => (canonical_text(v).unwrap(), Some("number->text")),
                Value::Bool(_) => (canonical_text(v).unwrap(), Some("boolean->text")),
                other => return Error(format!("expected text, got {}; no conversion", type_name(other))),
            };
            let len = s.chars().count();
            if let Some(m) = min_len && len < *m { return Error(format!("fails min({m})")); }
            if let Some(m) = max_len && len > *m { return Error(format!("fails max({m})")); }
            match action { None => Keep, Some(a) => Convert(Value::String(s), a) }
        }
        Ty::Enum(options) => match v {
            Value::String(s) if options.contains(s) => Keep,
            Value::Number(_) | Value::Bool(_) if options.contains(&canonical_text(v).unwrap()) => Convert(Value::String(canonical_text(v).unwrap()), "scalar->text"),
            _ => Error("not an enum member".into()),
        },
        Ty::Num { integer, min, max } => match v {
            Value::Number(_) => match num_constraints(v, *integer, *min, *max) { Ok(()) => Keep, Err(e) => Error(format!("constraint: {e}")) },
            Value::String(s) => match text_to_number(s) {
                Ok(n) => match num_constraints(&n, *integer, *min, *max) {
                    Ok(()) => Convert(n, "text->number"),
                    Err(e) => Error(format!("converted, then constraint: {e}")),
                },
                Err(e) => Error(format!("conversion failed: {e}")),
            },
            Value::Bool(_) => Error("boolean->number rejected".into()),
            other => Error(format!("expected number, got {}; no conversion", type_name(other))),
        },
        Ty::Bool => match v {
            Value::Bool(_) => Keep,
            Value::String(s) => match text_to_bool(s) { Some(b) => Convert(Value::Bool(b), "text->boolean"), None => Error(format!("text {s:?} is not a boolean word")) },
            other => Error(format!("expected boolean, got {}; no conversion", type_name(other))),
        },
        Ty::NumberLike => match v {
            Value::Number(_) => Keep,
            Value::String(s) if text_to_number(s).is_ok() => Keep,
            other => Error(format!("numberlike rejects {}", type_name(other))),
        },
        Ty::Boolish => match v {
            Value::Bool(_) => Keep,
            Value::String(s) if text_to_bool(s).is_some() => Keep,
            other => Error(format!("boolish rejects {}", type_name(other))),
        },
        Ty::Object => if v.is_object() { Keep } else { Error(format!("expected object, got {}; no conversion", type_name(v))) },
        Ty::Json => match v {
            Value::String(s) => if serde_json::from_str::<Value>(s).is_ok() { Keep } else { Error("text is not valid JSON".into()) },
            Value::Null => Error("null is never serialized".into()),
            other => Convert(Value::String(other.to_string()), "native->json"),
        },
        Ty::Yaml => match v {
            Value::String(s) => if serde_yaml_ng::from_str::<serde_yaml_ng::Value>(s).is_ok() { Keep } else { Error("text is not valid YAML".into()) },
            Value::Null => Error("null is never serialized".into()),
            other => Convert(Value::String(serde_yaml_ng::to_string(other).unwrap_or_default()), "native->yaml"),
        },
        Ty::Array(inner) => match v {
            Value::Array(items) => {
                let mut out = Vec::new();
                let mut changed = false;
                for (i, item) in items.iter().enumerate() {
                    match bind(item, inner) {
                        Keep => out.push(item.clone()),
                        Convert(c, _) => { changed = true; out.push(c) }
                        Error(e) => return Error(format!("[{i}]: {e}")),
                    }
                }
                if changed { Convert(Value::Array(out), "element-wise") } else { Keep }
            }
            other => Error(format!("expected list, got {}; never wrapped", type_name(other))),
        },
        Ty::Union(options) => {
            let results: Vec<(usize, Bind)> = options.iter().enumerate().map(|(i, o)| (i, bind(v, o))).collect();
            if results.iter().any(|(_, b)| *b == Keep) { return Keep; }
            let reachable: Vec<(usize, Value, &'static str)> = results.iter().filter_map(|(i, b)| match b { Convert(c, a) => Some((*i, c.clone(), *a)), _ => None }).collect();
            match reachable.len() {
                0 => Error(format!("no union option reachable: {}", results.iter().map(|(_, b)| format!("{b:?}")).collect::<Vec<_>>().join("; "))),
                1 => Convert(reachable[0].1.clone(), reachable[0].2),
                _ => {
                    let first = &reachable[0].1;
                    if reachable.iter().all(|(_, c, _)| c == first) { return Convert(first.clone(), reachable[0].2); }
                    // tie-breaker 1: number over boolean
                    let nums: Vec<_> = reachable.iter().filter(|(_, c, _)| c.is_number()).collect();
                    let bools = reachable.iter().filter(|(_, c, _)| c.is_boolean()).count();
                    if nums.len() >= 1 && bools >= 1 && nums.len() + bools == reachable.len() {
                        return Convert(nums[0].1.clone(), "text->number (tie: number over boolean)");
                    }
                    // tie-breaker 2: json over yaml
                    let json_idx = options.iter().position(|o| *o == Ty::Json);
                    if let Some(j) = json_idx && reachable.len() == 2 && options.iter().any(|o| *o == Ty::Yaml) {
                        if let Some(r) = reachable.iter().find(|(i, _, _)| *i == j) { return Convert(r.1.clone(), "native->json (tie: json over yaml)"); }
                    }
                    Error(format!("ambiguous union: {}", reachable.iter().map(|(_, c, a)| format!("{a}={c}")).collect::<Vec<_>>().join(" | ")))
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum Prediction {
    Arity(String),
    TypeError(Vec<String>),
    /// Bound args, list of conversions applied, index of chosen overload.
    Bound { args: Vec<Value>, conversions: Vec<String>, overload: usize },
    Ambiguous(String),
}

/// Binds a full call against one or more overloads per "Overload Selection".
pub fn predict(sigs: &[Signature], args: &[Value]) -> Prediction {
    let mut exact = Vec::new();
    let mut converted = Vec::new();
    let mut failures = Vec::new();
    let mut arity_ok_any = false;
    for (oi, sig) in sigs.iter().enumerate() {
        if args.len() < sig.min_arity() || sig.max_arity().is_some_and(|m| args.len() > m) {
            failures.push(format!("overload {oi}: arity {} not in {}..{:?}", args.len(), sig.min_arity(), sig.max_arity()));
            continue;
        }
        arity_ok_any = true;
        let mut bound = Vec::new();
        let mut conversions = Vec::new();
        let mut errors = Vec::new();
        for (ai, a) in args.iter().enumerate() {
            let p = if ai < sig.params.len() { &sig.params[ai] } else { sig.params.last().unwrap() };
            match bind(a, &p.ty) {
                Bind::Keep => bound.push(a.clone()),
                Bind::Convert(c, how) => { conversions.push(format!("{}: {how}", p.name)); bound.push(c) }
                Bind::Error(e) => errors.push(format!("{} (arg {ai}): {e}", p.name)),
            }
        }
        if !errors.is_empty() {
            failures.push(format!("overload {oi}: {}", errors.join("; ")));
        } else if conversions.is_empty() {
            exact.push((oi, bound, conversions));
        } else {
            converted.push((oi, bound, conversions));
        }
    }
    if !arity_ok_any { return Prediction::Arity(failures.join(" / ")); }
    if exact.len() == 1 {
        let (oi, args, conversions) = exact.remove(0);
        return Prediction::Bound { args, conversions, overload: oi };
    }
    if exact.len() > 1 { return Prediction::Ambiguous("several overloads accept unchanged".into()); }
    match converted.len() {
        1 => { let (oi, args, conversions) = converted.remove(0); Prediction::Bound { args, conversions, overload: oi } }
        0 => Prediction::TypeError(failures),
        _ => Prediction::Ambiguous("several overloads bind after conversion".into()),
    }
}

/// Loads the draft catalog: name -> overload list (Err = unparseable).
pub fn load_draft(path: &str) -> std::collections::BTreeMap<String, Result<Vec<Signature>, String>> {
    let text = std::fs::read_to_string(path).expect("read draft catalog");
    let doc: serde_yaml_ng::Value = serde_yaml_ng::from_str(&text).expect("parse draft yaml");
    let mut out = std::collections::BTreeMap::new();
    if let Some(types) = doc.get("types").and_then(|t| t.as_mapping()) {
        for (k, v) in types {
            let name = k.as_str().unwrap_or_default().to_string();
            let bodies: Vec<String> = match v {
                serde_yaml_ng::Value::String(s) => vec![s.clone()],
                serde_yaml_ng::Value::Sequence(items) => items.iter().filter_map(|i| i.as_str().map(str::to_string)).collect(),
                _ => vec![],
            };
            let parsed: Result<Vec<Signature>, String> = bodies.iter().map(|b| parse_function(b)).collect();
            out.insert(name, parsed);
        }
    }
    out
}

#[allow(dead_code)]
pub fn empty_obj() -> Value { Value::Object(Map::new()) }

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn numbers() {
        assert_eq!(text_to_number(" 4 ").unwrap(), json!(4));
        assert_eq!(text_to_number("+4").unwrap(), json!(4));
        assert_eq!(text_to_number("1_000").unwrap(), json!(1000));
        assert_eq!(text_to_number("1e3").unwrap(), json!(1000.0));
        assert_eq!(text_to_number(".5").unwrap(), json!(0.5));
        assert_eq!(text_to_number("9007199254740993").unwrap(), json!(9007199254740993u64));
        assert!(text_to_number("99999999999999999999").is_err());
        assert!(text_to_number("5.").is_err());
        assert!(text_to_number("pear").is_err());
    }
}
