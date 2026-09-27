//! THROWAWAY SPIKE (2026-09-21-schema-enhancements): records today's behavior of
//! every registered expression function over a shared input set, predicts the
//! spec's coercion-engine behavior from the draft `function(...)` signatures,
//! and diffs the two. Not production code; never commit.
//!
//! Run: cargo run -p darkmatter --example spike_coercion_baseline -- <draft functions.yaml> <out dir>

mod binder;

use std::collections::{BTreeMap, HashMap};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;

use darkmatter::markdown::compose::expression::{
    DataType, EvaluationLookup, Expr, ExpressionError, ResolutionContext, evaluate,
    expression_function_descriptors, functions::dispatchable_canonical_names, parse,
};
use serde_json::{Value, json};

use binder::{Prediction, Signature};

struct Lookup {
    vars: HashMap<String, Value>,
    ctx: ResolutionContext,
}

impl EvaluationLookup for Lookup {
    fn get(&self, path: &str) -> Option<Value> {
        self.vars.get(path).cloned()
    }
    fn resolution_context(&self) -> Option<ResolutionContext> {
        Some(self.ctx.clone())
    }
    fn resolution_context_ref(&self) -> Option<&ResolutionContext> {
        Some(&self.ctx)
    }
}

/// Functions that may touch a provider, git remote, ICMP, or the network.
/// They run in a non-repository temp dir with default (deny-all) authorities,
/// so they fail before any request once argument checks pass.
const NETWORKISH: &[&str] = &[
    "pr", "pr_list", "cicd", "cicd_list", "branch_exists_on_remote", "remote_vendor",
    "predict_conflicts", "recent_commits", "ping", "ping_under", "ipv4", "ipv6",
];

fn samples() -> Vec<(&'static str, Value)> {
    vec![
        ("null", Value::Null),
        ("true", json!(true)),
        ("false", json!(false)),
        ("0", json!(0)),
        ("-1", json!(-1)),
        ("2.5", json!(2.5)),
        ("9007199254740993", json!(9007199254740993u64)),
        ("\"9007199254740993\"", json!("9007199254740993")),
        ("\"4\"", json!("4")),
        ("\" 4 \"", json!(" 4 ")),
        ("\"+4\"", json!("+4")),
        ("\"1e3\"", json!("1e3")),
        ("\"1_000\"", json!("1_000")),
        ("\".5\"", json!(".5")),
        ("\"1\"", json!("1")),
        ("\"pear\"", json!("pear")),
        ("\"yes\"", json!("yes")),
        ("\"On\"", json!("On")),
        ("\"TRUE\"", json!("TRUE")),
        ("\"\"", json!("")),
        ("\"2026-09-25\"", json!("2026-09-25")),
        ("[]", json!([])),
        ("[1,\"2\"]", json!([1, "2"])),
        ("{}", json!({})),
        ("{\"a\":1}", json!({"a": 1})),
    ]
}

/// Per-function valid argument overrides (full positional vector, including
/// optional parameters), used where the catalog example is not all-literal.
fn overrides() -> HashMap<&'static str, Vec<Value>> {
    HashMap::from([
        ("is_array", vec![json!([1])]),
        ("first", vec![json!([1, 2])]),
        ("last", vec![json!([1, 2])]),
        ("has_key", vec![json!({"a": 1}), json!("a")]),
        ("contains", vec![json!("hello"), json!("ell")]),
        ("number", vec![json!("4"), json!(7)]),
        ("round", vec![json!(4.4), json!(7)]),
        ("date", vec![json!("2026-09-25"), json!("long")]),
        ("absolute", vec![json!("doc.md")]),
        ("relative", vec![json!("doc.md")]),
        ("file_exists", vec![json!("doc.md")]),
        ("markdown_body_empty", vec![json!("doc.md")]),
        ("markdown_title", vec![json!("doc.md")]),
        ("date_delta", vec![json!("2024-06-01"), json!("2024-06-20"), json!("14d")]),
        ("older_than", vec![json!("2024-06-01"), json!("2024-06-20"), json!("14d")]),
        ("newer_than", vec![json!("2024-06-01"), json!("2024-06-20"), json!("14d")]),
        ("frontmatter", vec![json!("doc.md"), json!("title")]),
        ("validate_schema", vec![json!("doc.md"), json!({"a": 1})]),
        ("link", vec![json!("doc.md"), json!("Docs")]),
        ("join", vec![json!("dir"), json!("doc.md")]),
        ("ensure_leading", vec![json!("path"), json!("/")]),
        ("ensure_trailing", vec![json!("path"), json!("/")]),
        ("replace", vec![json!("hello"), json!("l"), json!("L")]),
        ("replace_first", vec![json!("hello"), json!("l"), json!("L")]),
        ("replace_last", vec![json!("hello"), json!("l"), json!("L")]),
        ("starts_with", vec![json!("hello"), json!("he")]),
        ("ends_with", vec![json!("hello"), json!("lo")]),
        ("and", vec![json!(true), json!(true)]),
        ("or", vec![json!(false), json!(true)]),
        ("pr", vec![json!(4)]),
        ("cicd", vec![json!(4)]),
        ("pr_list", vec![json!(3)]),
        ("cicd_list", vec![json!(3)]),
        ("recent_commits", vec![json!(3)]),
        ("ping", vec![json!("127.0.0.1"), json!(250)]),
        ("ping_under", vec![json!("127.0.0.1"), json!(50), json!(5)]),
        ("branch_exists_on_remote", vec![json!("main"), json!("origin")]),
        ("remote_vendor", vec![json!("origin")]),
        ("predict_conflicts", vec![json!("main")]),
        ("has_command", vec![json!("ls")]),
        ("has_binary", vec![json!("ls")]),
        ("can_execute", vec![json!("ls")]),
        ("has_alias", vec![json!("ll")]),
        ("has_builtin_function", vec![json!("cd")]),
        ("has_user_function", vec![json!("myfn")]),
        ("has_skill", vec![json!("rust")]),
        ("has_local_skill", vec![json!("rust")]),
        ("has_agentic_cli", vec![json!("claude")]),
        ("as_markdown", vec![json!("doc.md")]),
        ("terminal", vec![json!("**bold**")]),
        ("without_date", vec![json!("notes-2026-09-25")]),
    ])
}

fn default_for(ty: DataType, array: bool) -> Value {
    let scalar = match ty {
        DataType::String | DataType::Any => json!("hello"),
        DataType::Number => json!(3),
        DataType::Integer => json!(2),
        DataType::Boolean => json!(true),
        DataType::Date => json!("2026-09-25"),
        DataType::DateTime => json!("2026-09-25T10:00:00Z"),
        DataType::Time => json!("10:00:00"),
        DataType::Object => json!({"a": 1}),
        DataType::File => json!("doc.md"),
        DataType::Url => json!("https://example.com"),
        DataType::Email => json!("a@example.com"),
        DataType::Yaml => json!("a: 1"),
        DataType::Json => json!("{\"a\":1}"),
    };
    if array { json!(["a", "b"]) } else { scalar }
}

fn literal_value(expr: &Expr) -> Option<Value> {
    match expr {
        Expr::Variable(_) | Expr::FunctionCall { .. } => None,
        Expr::ArrayLiteral(items) => items.iter().map(literal_value).collect::<Option<Vec<_>>>().map(Value::Array),
        other => {
            let lookup = Lookup { vars: HashMap::new(), ctx: ResolutionContext::new(PathBuf::from(".")) };
            evaluate(other, &lookup).ok()
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
struct Outcome {
    /// ok | null-passthrough | arg-error | arity | domain | io-skipped | panic
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

fn classify_error(error: &ExpressionError) -> &'static str {
    let msg = error.to_string();
    let lower = msg.to_ascii_lowercase();
    match error {
        ExpressionError::Arity { .. } => return "arity",
        ExpressionError::ArgType { .. } => return "arg-error",
        ExpressionError::Provider { .. } => return "io-skipped",
        ExpressionError::FileReference(_) => return "domain",
        _ => {}
    }
    const ARITY: &[&str] = &["requires 1 argument", "requires 2 arguments", "requires 3 arguments", "requires 0 arguments", "requires at least", "to 2 arguments", "to 3 arguments", "requires one identifier, got", "requires one query or count, got", "zero, one, or two", "argument(s), got", "arguments, got"];
    const IO: &[&str] = &["repository", "remote", "not a git", "resolution context", "nested", "unavailable", "icmp", "ping is not", "denied", "not granted", "permission", "no such file", "does not exist", "failed to read", "could not read", "not captured", "compose request", "git", "capture group", "only available while a document"];
    const ARG: &[&str] = &["expected", "must be", "requires a", "requires an", "not a string", "not a number", "not an array", "not an object", "invalid", "argument", "unknown field", "positive", "cannot be", "unsupported", "only accepts", "must not"];
    // Content validation of a value whose JSON type the handler accepted.
    const VALUE: &[&str] = &["invalid icmp budget", "must be between", "remote must not be empty", "is not an ip address literal", "unknown format", "invalid iso date", "invalid duration", "unknown agentic cli", "unknown field", "invalid query", "identifier must be"];
    if ARITY.iter().any(|p| lower.contains(p)) { return "arity"; }
    if VALUE.iter().any(|p| lower.contains(p)) { return "value-invalid"; }
    if IO.iter().any(|p| lower.contains(p)) { return "io-skipped"; }
    if ARG.iter().any(|p| lower.contains(p)) { return "arg-error"; }
    "domain"
}

fn call(name: &str, args: &[Value], ctx: &ResolutionContext) -> Outcome {
    let vars: HashMap<String, Value> = args.iter().enumerate().map(|(i, v)| (format!("a{i}"), v.clone())).collect();
    let expr = Expr::FunctionCall { name: name.to_string(), args: (0..args.len()).map(|i| Expr::Variable(format!("a{i}"))).collect() };
    let lookup = Lookup { vars, ctx: ctx.clone() };
    match catch_unwind(AssertUnwindSafe(|| evaluate(&expr, &lookup))) {
        Ok(Ok(value)) => Outcome { kind: "ok".into(), value: Some(value), message: None },
        Ok(Err(error)) => Outcome { kind: classify_error(&error).into(), value: None, message: Some(error.to_string()) },
        Err(_) => Outcome { kind: "panic".into(), value: None, message: None },
    }
}

#[derive(Debug, serde::Serialize)]
struct Record {
    function: String,
    position: usize,
    param: String,
    sample: String,
    args: Vec<Value>,
    today: Outcome,
    predicted: Value,
    signature_source: String,
    diff: String,
}

fn old_param_names(signature: &str) -> Vec<String> {
    let inner = signature.split_once('(').map(|(_, r)| r.trim_end_matches(')')).unwrap_or("");
    inner.split(',').map(|s| s.trim().trim_matches(['[', ']']).to_string()).filter(|s| !s.is_empty()).collect()
}

fn old_catalog_signature(name: &str) -> Vec<Signature> {
    expression_function_descriptors().iter().filter(|d| d.signature.split('(').next() == Some(name)).map(|d| {
        let names = old_param_names(d.signature);
        let params = d.parameters.iter().enumerate().map(|(i, p)| {
            let base = match p.ty {
                DataType::String => "string", DataType::Number => "number", DataType::Integer => "number(integer)",
                DataType::Boolean => "boolean", DataType::Date => "date", DataType::DateTime => "datetime", DataType::Time => "time",
                DataType::Object => "object", DataType::File => "file", DataType::Url => "url", DataType::Email => "email",
                DataType::Yaml => "yaml", DataType::Json => "json", DataType::Any => "any",
            };
            let ty_text = if p.array { format!("{base}[]") } else { base.to_string() };
            binder::Param { name: names.get(i).cloned().unwrap_or(format!("p{i}")), ty: binder::parse_type(&ty_text).unwrap(), optional: p.optional, rest: p.variadic }
        }).collect();
        Signature { text: d.signature.to_string(), params, notes: vec!["provisional: translated from old catalog".into()] }
    }).collect()
}

fn predicted_json(pred: &Prediction, compute: Option<&Outcome>) -> Value {
    match pred {
        Prediction::Arity(m) => json!({"kind": "arity", "message": m}),
        Prediction::TypeError(m) => json!({"kind": "type-error", "reasons": m}),
        Prediction::Ambiguous(m) => json!({"kind": "ambiguous", "message": m}),
        Prediction::Bound { args, conversions, overload } => {
            let c = compute.unwrap();
            let kind = match c.kind.as_str() {
                "arg-error" | "arity" => "bound-but-handler-rejects".to_string(),
                "value-invalid" => "bound-but-content-invalid".to_string(),
                other => other.to_string(),
            };
            json!({"kind": kind, "bound_args": args, "conversions": conversions, "overload": overload, "compute": c})
        }
    }
}

fn diff_class(sample: &Value, target_is_number: bool, today: &Outcome, pred: &Value) -> String {
    let pk = pred["kind"].as_str().unwrap_or("");
    let tk = today.kind.as_str();
    let conv = pred["conversions"].as_array().map(|a| a.iter().filter_map(|c| c.as_str()).map(|c| c.split(": ").nth(1).unwrap_or(c).to_string()).collect::<Vec<_>>().join(",")).unwrap_or_default();
    let today_rejects = matches!(tk, "arg-error" | "arity" | "value-invalid");
    let pred_rejects = matches!(pk, "type-error" | "arity" | "ambiguous");
    let today_passes = !today_rejects && tk != "panic";
    let pred_passes = !pred_rejects && !pk.starts_with("bound-but");
    if pk == "bound-but-handler-rejects" {
        return if today_rejects { "draft signature wider than code (engine binds, handler still rejects)".into() } else { format!("changed: handler rejects converted value ({conv})") };
    }
    if pk == "bound-but-content-invalid" {
        return match tk {
            "value-invalid" => "preserved: content validation stays in handler (needs refined type or `fallible`)".into(),
            "arg-error" | "arity" => format!("type error today -> content error after conversion ({conv})"),
            _ => format!("changed: content invalid after conversion ({conv})"),
        };
    }
    if today_rejects && pred_rejects {
        if tk == "arity" && pk != "arity" { return "preserved-rejection (arity error today, type error predicted)".into(); }
        if tk != "arity" && pk == "arity" { return "preserved-rejection (type error today, arity error predicted)".into(); }
        return "preserved-rejection".into();
    }
    if sample.is_null() && today_passes && pred_rejects {
        return match (tk, &today.value) {
            ("ok", Some(Value::Null)) => "null: propagates today -> type error".into(),
            ("ok", _) => "null: consumed as a value today -> type error".into(),
            _ => "null: reaches domain/io today -> type error".into(),
        };
    }
    if today_passes && pred_rejects {
        if pk == "arity" { return "newly rejected: arity (draft signature has fewer parameters)".into(); }
        if sample.is_boolean() && target_is_number { return "newly rejected: boolean->number".into(); }
        if sample.is_string() && target_is_number { return "newly rejected: text the handler parsed itself fails the spec number grammar".into(); }
        let reasons: Vec<&str> = pred["reasons"].as_array().map(|r| r.iter().filter_map(|x| x.as_str()).collect()).unwrap_or_default();
        let reason = reasons.iter().find(|r| !r.contains("arity")).or(reasons.first()).copied().unwrap_or_default().to_string();
        let reason = reason.split(": ").skip(2).collect::<Vec<_>>().join(": ");
        let reason = if reason.contains("expected text") { "container/null to text (handler stringified it)".to_string() } else { reason };
        return format!("newly rejected: {}", reason.chars().take(70).collect::<String>());
    }
    if today_rejects && pred_passes {
        if sample.is_null() { return "null: rejected today -> accepted".into(); }
        if conv.is_empty() { return format!("newly accepted: exact match under draft type ({tk} today)"); }
        return format!("newly accepted via conversion: {conv}");
    }
    if today_passes && pred_passes {
        if tk == "ok" && pk == "ok" {
            if today.value.as_ref() == Some(&pred["compute"]["value"]) {
                return if conv.is_empty() { "preserved".into() } else { format!("preserved (via conversion {conv})") };
            }
            return format!("changed result (conversion {conv})");
        }
        if tk == pk { return "preserved (same non-ok outcome)".into(); }
        return format!("changed outcome class: {tk} -> {pk}");
    }
    format!("unclassified: {tk} -> {pk}")
}

fn main() {
    let mut argv = std::env::args().skip(1);
    let draft_path = argv.next().expect("draft functions.yaml path");
    let out_dir = PathBuf::from(argv.next().expect("out dir"));
    std::fs::create_dir_all(&out_dir).unwrap();

    // Fixture: a non-repository temp dir with a few Markdown files.
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().to_path_buf();
    std::fs::write(root.join("doc.md"), "---\ntitle: Hello\ncount: 3\n---\n# Title\n\nBody\n").unwrap();
    std::fs::write(root.join("notes-01.md"), "# one\n").unwrap();
    std::fs::write(root.join("notes-02.md"), "# two\n").unwrap();
    std::fs::create_dir_all(root.join("dir")).unwrap();
    let ctx = ResolutionContext::new(root.clone());

    let draft = binder::load_draft(&draft_path);
    let overrides = overrides();
    let samples = samples();
    let mut records = Vec::new();
    let mut base_calls = BTreeMap::new();
    let mut signature_info = BTreeMap::new();

    for name in dispatchable_canonical_names() {
        let descriptors: Vec<_> = expression_function_descriptors().iter().filter(|d| d.signature.split('(').next() == Some(name)).collect();
        // Widest overload drives which positions we probe.
        let widest = descriptors.iter().max_by_key(|d| d.parameters.len()).unwrap();
        let example_args: Option<Vec<Value>> = widest.example.as_ref().and_then(|e| match parse(e.invocation) {
            Ok(Expr::FunctionCall { args, .. }) => args.iter().map(literal_value).collect(),
            _ => None,
        });
        let mut base: Vec<Value> = widest.parameters.iter().map(|p| default_for(p.ty, p.array)).collect();
        if let Some(ex) = &example_args { for (i, v) in ex.iter().enumerate() { if i < base.len() { base[i] = v.clone(); } } }
        if let Some(ov) = overrides.get(name) { base = ov.clone(); }
        let variadic = widest.parameters.iter().any(|p| p.variadic);
        let min_required = widest.parameters.iter().filter(|p| !p.optional && !p.variadic).count();
        let names = old_param_names(widest.signature);

        let (sigs, source) = match draft.get(name) {
            Some(Ok(s)) => (s.clone(), "draft".to_string()),
            Some(Err(e)) => (old_catalog_signature(name), format!("draft-unparseable ({e}); provisional old-catalog")),
            None => (old_catalog_signature(name), "missing-from-draft; provisional old-catalog".to_string()),
        };
        signature_info.insert(name.to_string(), json!({
            "source": source,
            "draft": sigs.iter().map(|s| s.text.clone()).collect::<Vec<_>>(),
            "notes": sigs.iter().flat_map(|s| s.notes.clone()).collect::<Vec<_>>(),
            "old": descriptors.iter().map(|d| d.typed_signature()).collect::<Vec<_>>(),
        }));

        let probe_net = NETWORKISH.contains(&name);
        let base_outcome = call(name, &base, &ctx);
        base_calls.insert(name.to_string(), json!({"args": base, "today": base_outcome, "networkish": probe_net}));

        let positions = if base.is_empty() { 0 } else if variadic { base.len().max(2) } else { base.len() };
        // Zero-parameter functions: record the no-arg call plus a one-arg arity probe.
        if positions == 0 {
            for (label, sample) in &samples {
                let args = vec![sample.clone()];
                let today = call(name, &args, &ctx);
                let pred = binder::predict(&sigs, &args);
                let compute = if let Prediction::Bound { args: b, .. } = &pred { Some(call(name, b, &ctx)) } else { None };
                let pj = predicted_json(&pred, compute.as_ref());
                let diff = diff_class(sample, false, &today, &pj);
                records.push(Record { function: name.into(), position: 0, param: "(extra)".into(), sample: label.to_string(), args, today, predicted: pj, signature_source: source.clone(), diff });
            }
            continue;
        }
        for position in 0..positions {
            for (label, sample) in &samples {
                // Positions beyond the required ones include only the optionals up to `position`.
                let len = if variadic { base.len().max(position + 1) } else { (position + 1).max(min_required) };
                let mut args: Vec<Value> = (0..len).map(|i| base.get(i).cloned().unwrap_or(json!(true))).collect();
                args[position] = sample.clone();
                let today = call(name, &args, &ctx);
                let pred = binder::predict(&sigs, &args);
                let compute = if let Prediction::Bound { args: b, .. } = &pred { Some(call(name, b, &ctx)) } else { None };
                let pj = predicted_json(&pred, compute.as_ref());
                let target_is_number = sigs.iter().any(|s| s.params.get(position).or(s.params.last()).is_some_and(|p| matches!(p.ty, binder::Ty::Num { .. })));
                let diff = if source.starts_with("draft") && !source.contains("unparseable") { diff_class(sample, target_is_number, &today, &pj) } else { format!("missing-signature | {}", diff_class(sample, target_is_number, &today, &pj)) };
                let param = names.get(position).cloned().unwrap_or_else(|| names.last().cloned().unwrap_or_default());
                records.push(Record { function: name.into(), position, param, sample: label.to_string(), args, today, predicted: pj, signature_source: source.clone(), diff });
            }
        }
    }

    let draft_only: Vec<_> = draft.keys().filter(|k| !dispatchable_canonical_names().contains(&k.as_str())).cloned().collect();
    let out = json!({
        "generated": "spike 2026-09-26",
        "samples": samples.iter().map(|(l, _)| l).collect::<Vec<_>>(),
        "function_count": dispatchable_canonical_names().len(),
        "draft_only_names": draft_only,
        "signatures": signature_info,
        "base_calls": base_calls,
        "records": records,
    });
    std::fs::write(out_dir.join("baseline.json"), serde_json::to_string_pretty(&out).unwrap()).unwrap();
    eprintln!("wrote {} records", out["records"].as_array().unwrap().len());
}
