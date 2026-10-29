//! THROWAWAY SPIKE harness: case corpus, classified diff, micro-benchmark,
//! and the numberlike/boolish format check.
//!
//! Run: `cargo nextest run -p darkmatter --lib spike_engine --no-capture`
//! Set `SPIKE_OUT=<dir>` to write `frontmatter-diff.json` there.

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Instant;

use serde_json::{Value, json};

use super::FormatChecks;
use super::core::{self, Shape};
use super::from_grammar;
use super::from_json_schema::JsonTranslator;
use super::numberlike_format as nf;
use crate::markdown::Markdown;
use crate::markdown::compose::ComposeSource;
use crate::markdown::schemas::DarkmatterSchemas;
use crate::markdown::schemas::coerce::{coerce_frontmatter, coerce_frontmatter_with_pending};
use crate::markdown::schemas::simplified::{parse_yaml_schema, to_json_schema};
use crate::markdown::schemas::validate::{build_validator, error_top_level_key};

// ── case model ────────────────────────────────────────────────────────────

#[derive(Clone)]
enum S {
    /// A property fragment; wrapped as `{ properties: { v: frag } }`.
    Frag(Value),
    /// A whole compiled / raw root JSON Schema.
    Json(Value),
    /// A SimplifiedSchema `$schema` body (YAML).
    Grammar(&'static str),
}

#[derive(Clone, Debug, PartialEq)]
enum Out {
    Val(Value),
    Err,
}

impl Out {
    fn to_json(&self) -> Value {
        match self {
            Out::Val(v) => json!({ "outcome": "ok", "value": v }),
            Out::Err => json!({ "outcome": "type-error" }),
        }
    }
}

struct Case {
    id: String,
    source: &'static str,
    schema: S,
    instance: Value,
    pending: Vec<String>,
    /// Spec-mandated outcome when the spec changes today's behavior.
    spec: Option<Out>,
    spec_ref: &'static str,
}

fn c(id: &str, source: &'static str, schema: S, instance: Value) -> Case {
    Case {
        id: id.to_string(),
        source,
        schema,
        instance,
        pending: Vec::new(),
        spec: None,
        spec_ref: "",
    }
}

impl Case {
    fn spec(mut self, out: Out, r: &'static str) -> Self {
        self.spec = Some(out);
        self.spec_ref = r;
        self
    }
    fn pending(mut self, keys: &[&str]) -> Self {
        self.pending = keys.iter().map(|k| k.to_string()).collect();
        self
    }
}

fn wrap(frag: &Value) -> Value {
    json!({ "type": "object", "additionalProperties": true, "properties": { "v": frag } })
}

fn v(value: Value) -> Out {
    Out::Val(json!({ "v": value }))
}

// ── corpus ────────────────────────────────────────────────────────────────

const BOOLISH: &str = r#"{"anyOf":[{"type":"boolean"},{"enum":["true","false","True","False","TRUE","FALSE"]}]}"#;
const NUMBERLIKE: &str = r#"{"anyOf":[{"type":"number"},{"type":"string","pattern":"^-?\\d+(\\.\\d+)?$"}]}"#;

const R_NUM: &str = "Changes table: number spellings";
const R_BOOL: &str = "Changes table: boolean words / mixed case";
const R_RANGE: &str = "Changes table: out-of-range whole-number text";
const R_NL: &str = "Changes table: numberlike/boolish kept exactly as written";
const R_NLTEXT: &str = "Changes table: text accepted by numberlike/boolish";
const R_ROOT: &str = "Changes table: root-level unions order-independent";
const R_PROP: &str = "Changes table: property-level unions";
const R_NEST: &str = "Changes table: nested objects at every depth";
const R_TUPLE: &str = "Containers table (tuple is a new type)";

fn frag(s: &str) -> S {
    S::Frag(serde_json::from_str(s).expect("frag json"))
}

#[allow(clippy::too_many_lines)]
fn corpus() -> Vec<Case> {
    let src = "coerce.rs unit tests";
    let mut cases = Vec::new();
    let f = |id: &str, fr: S, val: Value| {
        let instance = json!({ "v": val });
        c(id, src, fr, instance)
    };

    // recognizers turned into value cases
    cases.push(f("boolean/true", frag(r#"{"type":"boolean"}"#), json!("true")));
    cases.push(f("boolish/true", frag(BOOLISH), json!("true")).spec(v(json!("true")), R_NL));
    cases.push(f("boolish/True", frag(BOOLISH), json!("True")).spec(v(json!("True")), R_NL));
    cases.push(f("boolish/yes", frag(BOOLISH), json!("yes")).spec(v(json!("yes")), R_NLTEXT));
    cases.push(f("number/42", frag(r#"{"type":"number"}"#), json!("42")));
    cases.push(f("integer/42", frag(r#"{"type":"integer"}"#), json!("42")));
    cases.push(f("numberlike/42", frag(NUMBERLIKE), json!("42")).spec(v(json!("42")), R_NL));
    cases.push(f("numberlike/+4", frag(NUMBERLIKE), json!("+4")).spec(v(json!("+4")), R_NLTEXT));
    cases.push(
        f("numberlike/huge", frag(NUMBERLIKE), json!("99999999999999999999")).spec(Out::Err, R_RANGE),
    );
    cases.push(f("string/7", frag(r#"{"type":"string"}"#), json!(7)));
    cases.push(f("string-email/7", frag(r#"{"type":"string","format":"email"}"#), json!(7)));
    cases.push(f("string-pattern/42", frag(r#"{"type":"string","pattern":"x","minLength":1}"#), json!(42)));
    cases.push(f("bool-array/strings", frag(r#"{"type":"array","items":{"type":"boolean"}}"#), json!(["true", "false"])));
    cases.push(f("object-array/opaque", frag(r#"{"type":"array","items":{"type":"object"}}"#), json!([{"a": 1}])));
    cases.push(f("opaque-object", frag(r#"{"type":"object"}"#), json!({"k": "v"})));
    cases.push(f("empty-schema/any", frag("{}"), json!("x")));
    cases.push(f("bare-enum/a", frag(r#"{"enum":["a","b"]}"#), json!("a")));
    cases.push(f("bare-enum/number-member", frag(r#"{"enum":["1","2"]}"#), json!(2)));
    cases.push(f("type-array/string-null/7", frag(r#"{"type":["string","null"]}"#), json!(7)));
    cases.push(f("union/number|string/'42'", frag(r#"{"anyOf":[{"type":"number"},{"type":"string"}]}"#), json!("42")));
    cases.push(f("union/number|string/true", frag(r#"{"anyOf":[{"type":"number"},{"type":"string"}]}"#), json!(true)));
    cases.push(f("union/boolean|enum-auto/'true'", frag(r#"{"anyOf":[{"type":"boolean"},{"enum":["auto"]}]}"#), json!("true")));
    cases.push(f("union/boolean|enum-true/'false'", frag(r#"{"anyOf":[{"type":"boolean"},{"enum":["true"]}]}"#), json!("false")));
    cases.push(f("union/boolean|enum-true/'true'", frag(r#"{"anyOf":[{"type":"boolean"},{"enum":["true"]}]}"#), json!("true")));
    cases.push(f(
        "union/number|string-upper/'42'",
        frag(r#"{"anyOf":[{"type":"number"},{"type":"string","pattern":"^[A-Z]+$"}]}"#),
        json!("42"),
    ));
    // literal const
    cases.push(f("const-2/'2'", frag(r#"{"const":2}"#), json!("2")));
    cases.push(f("const-2/'3'", frag(r#"{"const":2}"#), json!("3")));
    cases.push(f("const-2/' 2 '", frag(r#"{"const":2}"#), json!(" 2 ")).spec(v(json!(2)), R_NUM));
    cases.push(f("const-false/'false'", frag(r#"{"const":false}"#), json!("false")));
    cases.push(f("const-false/'true'", frag(r#"{"const":false}"#), json!("true")));
    cases.push(f("const-false/'off'", frag(r#"{"const":false}"#), json!("off")).spec(v(json!(false)), R_BOOL));
    cases.push(f("const-'spec'/'spec'", frag(r#"{"const":"spec"}"#), json!("spec")));
    cases.push(f("const-'2'/2", frag(r#"{"const":"2"}"#), json!(2)));
    cases.push(c(
        "literal-const-object-and-array",
        src,
        S::Json(json!({"type":"object","properties":{
            "version":{"anyOf":[{"type":"null"},{"const":2}]},
            "modes":{"anyOf":[{"type":"null"},{"type":"array","items":{"const":3}}]}}})),
        json!({"version":"2","modes":["3","4"]}),
    ));
    // boolean spellings
    for s in ["true", "True", "TRUE", "false", "False", "FALSE"] {
        cases.push(f(&format!("boolean/'{s}'"), frag(r#"{"type":"boolean"}"#), json!(s)));
    }
    cases.push(f("boolean/'tRuE'", frag(r#"{"type":"boolean"}"#), json!("tRuE")).spec(v(json!(true)), R_BOOL));
    for (s, b) in [("yes", true), ("no", false), ("on", true), ("off", false), ("1", true), ("0", false)] {
        cases.push(f(&format!("boolean/'{s}'"), frag(r#"{"type":"boolean"}"#), json!(s)).spec(v(json!(b)), R_BOOL));
    }
    cases.push(f("boolean/' true '", frag(r#"{"type":"boolean"}"#), json!(" true ")).spec(v(json!(true)), R_BOOL));
    for (id, val) in [("1", json!(1)), ("true", json!(true)), ("null", Value::Null), ("[1]", json!([1]))] {
        cases.push(f(&format!("boolean/native-{id}"), frag(r#"{"type":"boolean"}"#), val));
    }
    // numbers
    for s in ["42", "-7", "3.14", "-0.5"] {
        cases.push(f(&format!("number/'{s}'"), frag(r#"{"type":"number"}"#), json!(s)));
    }
    cases.push(f("integer/'3.14'", frag(r#"{"type":"integer"}"#), json!("3.14")));
    for s in ["abc", "1.2.3", "", "-", "5."] {
        cases.push(f(&format!("number/'{s}'"), frag(r#"{"type":"number"}"#), json!(s)));
    }
    for (s, n) in [(".5", json!(0.5)), ("1e3", json!(1000.0)), (" 1", json!(1)), ("1 ", json!(1)), ("+4", json!(4)), ("1_000", json!(1000))] {
        cases.push(f(&format!("number/'{s}'"), frag(r#"{"type":"number"}"#), json!(s)).spec(v(n), R_NUM));
    }
    cases.push(f("number/'1,000'", frag(r#"{"type":"number"}"#), json!("1,000")));
    cases.push(f("number/'NaN'", frag(r#"{"type":"number"}"#), json!("NaN")));
    cases.push(f("number/'0x10'", frag(r#"{"type":"number"}"#), json!("0x10")));
    for (id, val) in [("42", json!(42)), ("null", Value::Null), ("[1]", json!([1])), ("true", json!(true))] {
        cases.push(f(&format!("number/native-{id}"), frag(r#"{"type":"number"}"#), val));
    }
    cases.push(f("number/i64-max", frag(r#"{"type":"number"}"#), json!("9223372036854775807")));
    cases.push(f("number/i64-max+1", frag(r#"{"type":"number"}"#), json!("9223372036854775808")));
    cases.push(f("number/2^53+1", frag(r#"{"type":"number"}"#), json!("9007199254740993")));
    cases.push(f("number/1e20-text", frag(r#"{"type":"number"}"#), json!("99999999999999999999")).spec(Out::Err, R_RANGE));
    cases.push(f("integer/'4.0'", frag(r#"{"type":"integer"}"#), json!("4.0")));
    // to string
    for (id, val) in [("42", json!(42)), ("3.14", json!(3.14)), ("true", json!(true)), ("false", json!(false))] {
        cases.push(f(&format!("string/native-{id}"), frag(r#"{"type":"string"}"#), val));
    }
    for (id, val) in [("null", Value::Null), ("[1]", json!([1])), ("obj", json!({"a": 1}))] {
        cases.push(f(&format!("string/native-{id}"), frag(r#"{"type":"string"}"#), val));
    }
    // arrays
    cases.push(f("bool-array/mixed", frag(r#"{"type":"array","items":{"type":"boolean"}}"#), json!(["true", "nope"])));
    cases.push(f("bool-array/none", frag(r#"{"type":"array","items":{"type":"boolean"}}"#), json!(["nope", "maybe"])));
    cases.push(f("bool-array/scalar", frag(r#"{"type":"array","items":{"type":"boolean"}}"#), json!("not-an-array")));
    // object pass
    cases.push(c(
        "object-pass/declared-props",
        src,
        S::Json(json!({"type":"object","properties":{"flag":{"type":"boolean"},"count":{"type":"number"},"label":{"type":"string"},"opaque":{"type":"object"}}})),
        json!({"flag":"true","count":"42","label":7,"opaque":{"k":"v"},"undeclared":"left-alone"}),
    ));
    cases.push(c(
        "object-pass/unchanged",
        src,
        S::Json(json!({"type":"object","properties":{"flag":{"type":"boolean"}}})),
        json!({"flag": true, "other": "x"}),
    ));
    cases.push(
        c(
            "object-pass/boolish-numberlike",
            src,
            S::Json(json!({"type":"object","properties":{
                "flag": serde_json::from_str::<Value>(BOOLISH).unwrap(),
                "n": serde_json::from_str::<Value>(NUMBERLIKE).unwrap()}})),
            json!({"flag":"true","n":"42"}),
        )
        .spec(Out::Val(json!({"flag":"true","n":"42"})), R_NL),
    );
    // root unions
    let implement = json!({"anyOf":[
        {"type":"object","additionalProperties":true,"properties":{"kind":{"type":"string"},"has_spec":{"type":"boolean"},"has_plan":{"type":"boolean"},"has_review":{"type":"boolean"}},"required":["kind"]},
        {"type":"object","additionalProperties":true,"properties":{"other":{"type":"string"}},"required":["other"]},
        {"type":"object","additionalProperties":true,"properties":{"third":{"type":"string"}},"required":["third"]}]});
    cases.push(c(
        "root-union/implement-like",
        src,
        S::Json(implement.clone()),
        json!({"kind":"implement","has_spec":"true","has_plan":"false","has_review":"false"}),
    ));
    cases.push(c("root-union/no-arm", src, S::Json(implement), json!({"has_spec":"true"})));
    let pending_union = json!({"anyOf":[
        {"type":"object","additionalProperties":true,"properties":{"kind":{"type":"string"},"n":{"type":"number"},"flag":{"type":"boolean"}},"required":["kind"]},
        {"type":"object","additionalProperties":true,"properties":{"other":{"type":"string"}},"required":["other"]}]});
    cases.push(
        c("root-union/pending-key-blocks", src, S::Json(pending_union.clone()), json!({"kind":"implement","n":"$(echo 1)","flag":"false"}))
            .pending(&["n"]),
    );
    cases.push(c(
        "root-union/no-pending-full-validation",
        src,
        S::Json(json!({"anyOf":[pending_union["anyOf"][0].clone()]})),
        json!({"kind":"implement","n":"$(echo 1)","flag":"false"}),
    ));
    cases.push(
        c(
            "root-union/pending-does-not-mask",
            src,
            S::Json(json!({"anyOf":[pending_union["anyOf"][0].clone()]})),
            json!({"kind":"implement","n":"$(echo 1)","flag":[1,2]}),
        )
        .pending(&["n"]),
    );
    cases.push(c(
        "object-pass/null-array-object-vs-scalars",
        src,
        S::Json(json!({"type":"object","properties":{"a":{"type":"boolean"},"b":{"type":"number"},"c":{"type":"string"}}})),
        json!({"a":null,"b":[1,2],"c":{"k":"v"}}),
    ));
    // inline objects
    let inline = |props: Value| json!({"type":"object","properties":props,"additionalProperties":false});
    cases.push(c(
        "inline-object/declared-plus-undeclared",
        src,
        S::Json(json!({"type":"object","properties":{"config": inline(json!({"enabled":{"type":"boolean"},"retries":{"type":"number"}}))}})),
        json!({"config":{"enabled":"true","retries":"3","untouched":"x"}}),
    ));
    cases.push(c(
        "inline-object/declared-only",
        src,
        S::Json(json!({"type":"object","properties":{"config": inline(json!({"enabled":{"type":"boolean"},"retries":{"type":"number"}}))}})),
        json!({"config":{"enabled":"true","retries":"3"}}),
    ));
    cases.push(c(
        "inline-object/opaque-sibling",
        src,
        S::Json(json!({"type":"object","properties":{"config": inline(json!({"enabled":{"type":"boolean"},"metadata":{"type":"object"}}))}})),
        json!({"config":{"enabled":"true","metadata":{"source":"user"}}}),
    ));
    cases.push(c(
        "inline-object/not-an-object",
        src,
        S::Json(json!({"type":"object","properties":{"config": inline(json!({"enabled":{"type":"boolean"}}))}})),
        json!({"config":"not-an-object"}),
    ));
    cases.push(c(
        "inline-object-array/per-item",
        src,
        S::Json(json!({"type":"object","properties":{"authors":{"type":"array","items": inline(json!({"active":{"type":"boolean"},"score":{"type":"number"}}))}}})),
        json!({"authors":[{"active":"true","score":"4.5"},{"active":"false","score":"-1"}]}),
    ));
    let meta_union = json!({"type":"object","properties":{"metadata":{"anyOf":[inline(json!({"key":{"type":"string"},"count":{"type":"number"}})),{"type":"string"}]}}});
    cases.push(c("prop-union/inline-object-arm", src, S::Json(meta_union.clone()), json!({"metadata":{"key":"visits","count":"42"}})));
    cases.push(c("prop-union/zero-match", src, S::Json(meta_union), json!({"metadata":["a","b"]})));
    cases.push(c(
        "prop-union/const-kind-arm",
        src,
        S::Json(json!({"type":"object","properties":{"metadata":{"anyOf":[
            {"type":"object","properties":{"kind":{"const":"config"},"enabled":{"type":"boolean"},"details":{"type":"object"}},"required":["kind","enabled","details"],"additionalProperties":false},
            {"type":"string"}]}}})),
        json!({"metadata":{"kind":"config","enabled":"true","details":{"source":"user"}}}),
    ));
    // nullable wrappers
    let file3 = r#"{"anyOf":[{"type":"null"},{"const":""},{"type":"string","format":"darkmatter-file-reference"}]}"#;
    cases.push(f("optional-file/null", frag(file3), Value::Null));
    cases.push(f("optional-file/empty", frag(file3), json!("")));
    cases.push(f("optional-file/42", frag(file3), json!(42)));
    cases.push(f("optional-string/7", frag(r#"{"anyOf":[{"type":"null"},{"type":"string"}]}"#), json!(7)));
    cases.push(f("optional-number/'42'", frag(r#"{"anyOf":[{"type":"null"},{"type":"number"}]}"#), json!("42")));
    cases.push(f("optional-boolean/'true'", frag(r#"{"anyOf":[{"type":"null"},{"type":"boolean"}]}"#), json!("true")));
    cases.push(
        f("optional-boolish/'false'", S::Frag(json!({"anyOf":[{"type":"null"}, serde_json::from_str::<Value>(BOOLISH).unwrap()]})), json!("false"))
            .spec(v(json!("false")), R_NL),
    );
    cases.push(
        f("optional-numberlike/'99'", S::Frag(json!({"anyOf":[{"type":"null"}, serde_json::from_str::<Value>(NUMBERLIKE).unwrap()]})), json!("99"))
            .spec(v(json!("99")), R_NL),
    );
    cases.push(f("optional-bool-array", frag(r#"{"anyOf":[{"type":"null"},{"type":"array","items":{"type":"boolean"}}]}"#), json!(["true", "false"])));
    cases.push(f("optional-union/'42'", frag(r#"{"anyOf":[{"type":"null"},{"anyOf":[{"type":"number"},{"type":"string"}]}]}"#), json!("42")));
    cases.push(f("optional-union/true", frag(r#"{"anyOf":[{"type":"null"},{"anyOf":[{"type":"number"},{"type":"string"}]}]}"#), json!(true)));
    cases.push(f("optional-union/null", frag(r#"{"anyOf":[{"type":"null"},{"anyOf":[{"type":"number"},{"type":"string"}]}]}"#), Value::Null));
    // content formats
    let yaml = r#"{"type":"string","format":"darkmatter-yaml"}"#;
    let jsn = r#"{"type":"string","format":"darkmatter-json"}"#;
    cases.push(f("yaml/mapping", frag(yaml), json!({"title":"Foo","tags":["a","b"]})));
    cases.push(f("json/mapping", frag(jsn), json!({"title":"Foo","tags":["a","b"]})));
    cases.push(f("json/sequence", frag(jsn), json!([1, 2, 3])));
    cases.push(f("yaml/sequence", frag(yaml), json!([1, 2, 3])));
    cases.push(f("json/42", frag(jsn), json!(42)));
    cases.push(f("json/true", frag(jsn), json!(true)));
    cases.push(f("yaml/42", frag(yaml), json!(42)));
    cases.push(f("yaml/text", frag(yaml), json!("title: Foo")));
    cases.push(f("json/text-{}", frag(jsn), json!("{}")));
    cases.push(f("json/null", frag(jsn), Value::Null));
    cases.push(f("json/invalid-text", frag(jsn), json!("title: Foo")));
    cases.push(f(
        "semantic-keyword/array",
        S::Frag(json!({"type":"array","items":{"type":["string","object","array"],"x-darkmatter-type-definition":true}})),
        json!(["string", {"title":"string(required)"}]),
    ));

    // ── grammar-authored cases: L1 tests + spec acceptance criteria ──────
    let l1 = "L1: schemas_literal_expression.rs / compose_schema.rs";
    let g = |id: &str, source: &'static str, schema: &'static str, inst: Value| c(id, source, S::Grammar(schema), inst);
    cases.push(g("literal(2)/'2'", l1, "version: literal(2)", json!({"version":"2"})));
    cases.push(g("literal(spec)/spec", l1, "kind: literal(spec)", json!({"kind":"spec"})));
    for lit in ["9007199254740993", "9223372036854775807", "9223372036854775808", "18446744073709551615"] {
        let schema: &'static str = Box::leak(format!("version: literal({lit})").into_boxed_str());
        cases.push(g(&format!("literal({lit})/text"), l1, schema, json!({"version": lit})));
    }
    cases.push(g("literal-union/auto", l1, "width:\n  - literal(auto)\n  - number", json!({"width":"auto"})));
    cases.push(g("literal-union/5", l1, "width:\n  - literal(auto)\n  - number", json!({"width":5})));
    cases.push(g("literal-union/'5'", l1, "width:\n  - literal(auto)\n  - number", json!({"width":"5"})));
    cases.push(g("expression/true", l1, "when: expression", json!({"when": true})));
    cases.push(g("expression/3", l1, "retries: expression", json!({"retries": 3})));
    cases.push(g("expression/mapping", l1, "when: expression", json!({"when": {"a": 1}})));
    cases.push(g("cli-boolish/'True'", l1, "flag: boolish", json!({"flag":"True"})).spec(Out::Val(json!({"flag":"True"})), R_NL));
    cases.push(g("cli-numberlike/'42'", l1, "n: numberlike", json!({"n":"42"})).spec(Out::Val(json!({"n":"42"})), R_NL));

    let ac = "spec acceptance criteria 28-34";
    cases.push(g("AC28/version-1.2", ac, "version: string", json!({"version": 1.2})));
    cases.push(g("AC29/draft-yes", ac, "draft: boolean", json!({"draft":"yes"})).spec(Out::Val(json!({"draft":true})), R_BOOL));
    cases.push(g("AC29/draft-On", ac, "draft: boolean", json!({"draft":"On"})).spec(Out::Val(json!({"draft":true})), R_BOOL));
    cases.push(g("AC30/+4", ac, "n: number", json!({"n":"+4"})).spec(Out::Val(json!({"n":4})), R_NUM));
    cases.push(g("AC30/1_000", ac, "n: number", json!({"n":"1_000"})).spec(Out::Val(json!({"n":1000})), R_NUM));
    cases.push(g("AC30/huge", ac, "n: number", json!({"n":"99999999999999999999"})).spec(Out::Err, R_RANGE));
    cases.push(g("AC31/number-first", ac, "- x: number\n- x: boolean", json!({"x":"1"})));
    cases.push(g("AC31/boolean-first", ac, "- x: boolean\n- x: number", json!({"x":"1"})));
    cases.push(g("root-union/number-first/'4'", ac, "- x: number\n- x: string", json!({"x":"4"})).spec(Out::Val(json!({"x":"4"})), R_ROOT));
    cases.push(g("root-union/string-first/'4'", ac, "- x: string\n- x: number", json!({"x":"4"})));
    cases.push(g("root-union/yes-number-boolean", ac, "- x: number\n- x: boolean", json!({"x":"yes"})).spec(Out::Val(json!({"x":true})), R_BOOL));
    cases.push(g("AC32/nested-inline", ac, "meta: \"{ inner: { count: number } }\"", json!({"meta":{"inner":{"count":"3"}}})));
    cases.push(g("AC32/nested-mapping", ac, "meta:\n  inner:\n    count: number", json!({"meta":{"inner":{"count":"3"}}})));
    cases.push(
        g("nested/union-inside-object", ac, "meta:\n  id:\n    - number\n    - boolean", json!({"meta":{"id":"7"}}))
            .spec(Out::Val(json!({"meta":{"id":7}})), R_NEST),
    );
    cases.push(
        g("nested/numberlike-inside-object", ac, "meta: \"{ n: numberlike }\"", json!({"meta":{"n":"7"}}))
            .spec(Out::Val(json!({"meta":{"n":"7"}})), R_NL),
    );
    cases.push(g("nested/array-in-object-array", ac, "items: \"{ tags: boolean[] }[]\"", json!({"items":[{"tags":["true"]}]})));
    cases.push(g("AC33/json-native", ac, "config: json", json!({"config":{"port":8080}})));
    cases.push(
        g("AC33/json|yaml-native", ac, "config:\n  - json\n  - yaml", json!({"config":{"port":8080,"debug":true}}))
            .spec(Out::Val(json!({"config":"{\"port\":8080,\"debug\":true}"})), R_PROP),
    );
    cases.push(g("AC33/json-null-required", ac, "config: json(required)", json!({"config":null})));
    cases.push(g("json|yaml/text", ac, "config:\n  - json\n  - yaml", json!({"config":"port: 8080"})));
    cases.push(g("AC34/zip", ac, "zip: numberlike", json!({"zip":"02134"})).spec(Out::Val(json!({"zip":"02134"})), R_NL));
    cases.push(g("AC34/flag-On", ac, "flag: boolish", json!({"flag":"On"})).spec(Out::Val(json!({"flag":"On"})), R_NLTEXT));
    cases.push(g("AC34/zip-pear", ac, "zip: numberlike", json!({"zip":"pear"})));

    let ut = "spec Unions table";
    cases.push(g("U/string|number/'4'", ut, "x:\n  - string\n  - number", json!({"x":"4"})));
    cases.push(g("U/string|string[]/2026", ut, "x:\n  - string\n  - string[]", json!({"x":2026})));
    cases.push(
        g("U/number(min5)|boolean/'1'", ut, "x:\n  - number(min(5))\n  - boolean", json!({"x":"1"}))
            .spec(Out::Val(json!({"x":true})), R_PROP),
    );
    cases.push(g("U/number|boolean/'1'", ut, "x:\n  - number\n  - boolean", json!({"x":"1"})));
    cases.push(g("U/boolean|number/'0'", ut, "x:\n  - boolean\n  - number", json!({"x":"0"})));
    cases.push(g("U/{id:number}|{id:boolean}/'1'", ut, "x:\n  - \"{ id: number }\"\n  - \"{ id: boolean }\"", json!({"x":{"id":"1"}})));
    cases.push(g(
        "U/{a:n,b:b}|{a:b,b:n}/'1','1'",
        ut,
        "x:\n  - \"{ a: number, b: boolean }\"\n  - \"{ a: boolean, b: number }\"",
        json!({"x":{"a":"1","b":"1"}}),
    ));
    cases.push(
        g("U/{a:n,b:b}|{a:b,b:n}/'1','yes'", ut, "x:\n  - \"{ a: number, b: boolean }\"\n  - \"{ a: boolean, b: number }\"", json!({"x":{"a":"1","b":"yes"}}))
            .spec(Out::Val(json!({"x":{"a":1,"b":true}})), R_BOOL),
    );
    cases.push(g("U/boolean|json/'true'", ut, "x:\n  - boolean\n  - json", json!({"x":"true"})));
    cases.push(
        g("U/number|number(integer)/'4'", ut, "x:\n  - number\n  - number(integer)", json!({"x":"4"}))
            .spec(Out::Val(json!({"x":4})), R_PROP),
    );
    cases.push(g("refined/date/2026", ut, "d: date", json!({"d":2026})));
    cases.push(g("refined/file/42", ut, "f: file", json!({"f":42})));
    cases.push(g("refined/url/42", ut, "u: url", json!({"u":42})));
    cases.push(g("refined/enum/2", ut, "s: enum(1,2,3)", json!({"s":2})));
    cases.push(g("refined/enum/'2'", ut, "s: enum(1,2,3)", json!({"s":"2"})));
    cases.push(g("refined/literal-string/2", ut, "s: literal('2')", json!({"s":2})));
    cases.push(g("constraint/string(min3;max3)/'ABCD'", ut, "s: string(min(3); max(3))", json!({"s":"ABCD"})));
    cases.push(g("constraint/integer/'2.5'", ut, "n: number(integer)", json!({"n":"2.5"})));
    cases.push(g("constraint/integer/'4.0'", ut, "n: number(integer)", json!({"n":"4.0"})));

    // tuples (compiled form per spec: prefixItems/minItems/items)
    let tuple = json!({"type":"object","properties":{"p":{"type":"array","prefixItems":[{"type":"string"},{"type":"number"},{"type":"boolean"}],"minItems":2,"items":false}}});
    cases.push(g("pattern-key/<string>:number/'3'", ut, "\"<string>\": number", json!({"a":"3"})));
    cases.push(g("pattern-key/<starting::n_>:boolean/'true'", ut, "\"<starting::n_>\": boolean", json!({"n_a":"true"})));
    cases.push(g("pattern-key/inline-dict", ut, "m: \"{ <string>: number }\"", json!({"m":{"a":"3"}})));
    let ct = "spec Containers table";
    cases.push(c("T/tuple/Ada-36-yes", ct, S::Json(tuple.clone()), json!({"p":["Ada","36","yes"]})).spec(Out::Val(json!({"p":["Ada",36,true]})), R_TUPLE));
    cases.push(c("T/tuple/Ada-36", ct, S::Json(tuple.clone()), json!({"p":["Ada","36"]})).spec(Out::Val(json!({"p":["Ada",36]})), R_TUPLE));
    cases.push(c("T/tuple/Ada", ct, S::Json(tuple), json!({"p":["Ada"]})));
    cases.push(g("T/number[]/'1','2','x'", ct, "values: number[]", json!({"values":["1","2","x"]})));
    cases.push(g("T/number[]/scalar", ct, "values: number[]", json!({"values":"1"})));
    cases.push(g("T/number[]/'1','2'", ct, "values: number[]", json!({"values":["1","2"]})));

    // raw JSON Schema keywords
    let raw = "raw JSON Schema keyword probe";
    cases.push(c("raw/$ref-integer", raw, S::Json(json!({"$defs":{"n":{"type":"integer"}},"type":"object","properties":{"id":{"$ref":"#/$defs/n"}}})), json!({"id":"42"})));
    cases.push(c("raw/allOf-number", raw, S::Json(json!({"type":"object","allOf":[{"properties":{"n":{"type":"number"}}}]})), json!({"n":"4"})));
    cases.push(c("raw/allOf-sibling-required", raw, S::Json(json!({"type":"object","properties":{"n":{"type":"number"}},"allOf":[{"required":["n"]}]})), json!({"n":"4"})));
    cases.push(c("raw/oneOf-number|boolean/'1'", raw, S::Json(json!({"type":"object","properties":{"x":{"oneOf":[{"type":"number"},{"type":"boolean"}]}}})), json!({"x":"1"})));
    cases.push(c("raw/if-then", raw, S::Json(json!({"type":"object","properties":{"n":{"type":"number"}},"if":{"properties":{"n":{"const":1}}},"then":{"required":["m"]}})), json!({"n":"1","m":true})));
    cases.push(c("raw/type-array-integer-null", raw, S::Json(json!({"type":"object","properties":{"n":{"type":["integer","null"]}}})), json!({"n":"5"})));
    cases.push(c("raw/enum-mixed/'1'", raw, S::Json(json!({"type":"object","properties":{"e":{"enum":[1,"a"]}}})), json!({"e":"1"})));
    cases.push(c("raw/exclusiveMinimum/'0'", raw, S::Json(json!({"type":"object","properties":{"n":{"type":"number","exclusiveMinimum":0}}})), json!({"n":"0"})));
    cases.push(c("raw/patternProperties/'3'", raw, S::Json(json!({"type":"object","patternProperties":{"^n_":{"type":"number"}},"additionalProperties":false})), json!({"n_a":"3"})));
    cases.push(c("raw/additionalProperties-schema", raw, S::Json(json!({"type":"object","additionalProperties":{"type":"boolean"}})), json!({"any":"true"})));
    cases.push(c("raw/multipleOf", raw, S::Json(json!({"type":"object","properties":{"n":{"type":"number","multipleOf":2}}})), json!({"n":"4"})));
    cases.push(c("raw/url-scheme", raw, S::Json(json!({"type":"object","properties":{"u":{"type":"string","format":"uri","x-darkmatter-url-scheme":["https"]}}})), json!({"u":"http://x.y"})));
    cases
}

// ── runners ───────────────────────────────────────────────────────────────

struct Row {
    json: Value,
    class: &'static str,
}

fn today(schema: &Value, instance: &Value, pending: &[String]) -> Out {
    let set: HashSet<String> = pending.iter().cloned().collect();
    let coerced = coerce_frontmatter_with_pending(schema, instance, &set).value;
    let validator = build_validator(schema, None, None).expect("schema builds");
    let pending_only = |val: &jsonschema::Validator| {
        val.iter_errors(&coerced)
            .all(|e| error_top_level_key(&e).is_some_and(|k| set.contains(&k)))
    };
    let ok = if set.is_empty() {
        validator.is_valid(&coerced)
    } else if let Some(arms) = schema.get("anyOf").and_then(Value::as_array) {
        // mirror coerce.rs `arm_accepts`: some arm fails only on pending keys
        arms.iter().any(|arm| {
            let wrapped = crate::markdown::schemas::validate::wrap_arm_as_root_schema(arm);
            build_validator(&wrapped, None, None).is_ok_and(|val| pending_only(&val))
        })
    } else {
        pending_only(&validator)
    };
    if ok { Out::Val(coerced) } else { Out::Err }
}

fn core_run(shape: &Shape, instance: &Value, pending: &[String], checks: &FormatChecks) -> (Out, Value) {
    match core::bind_root(shape, instance, pending, checks) {
        Ok(b) => (Out::Val(b.value), Value::Null),
        Err(es) => (
            Out::Err,
            Value::Array(
                es.iter()
                    .map(|e| json!({ "path": e.path, "reason": format!("{:?}", e.reason) }))
                    .collect(),
            ),
        ),
    }
}

fn classify(today: &Out, core: &Out, spec: &Option<Out>) -> &'static str {
    match spec {
        Some(expect) if core == expect && core != today => "intentionally changed",
        Some(expect) if core == expect => "unchanged (spec row already true today)",
        Some(_) => "UNEXPECTEDLY changed (core disagrees with spec expectation)",
        None if core == today => "unchanged",
        None => "UNEXPECTEDLY changed",
    }
}

fn run_case(case: &Case) -> Row {
    let (compiled, grammar_shape, schema_repr) = match &case.schema {
        S::Frag(fr) => (wrap(fr), None, fr.clone()),
        S::Json(j) => (j.clone(), None, j.clone()),
        S::Grammar(y) => {
            let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(y).expect("yaml");
            let simplified = parse_yaml_schema(&yaml).unwrap_or_else(|e| panic!("[{}] grammar: {e}", case.id));
            let compiled = to_json_schema(&simplified).expect("convert");
            (compiled, from_grammar::translate_schema(&simplified), Value::String((*y).to_string()))
        }
    };
    let today_out = today(&compiled, &case.instance, &case.pending);
    let checks = FormatChecks::new(None);
    let translator = JsonTranslator::new(&compiled, None);
    let shape = translator.translate_root();
    let (core_out, core_errors) = core_run(&shape, &case.instance, &case.pending, &checks);
    let grammar_agrees = grammar_shape.as_ref().map(|gs| core_run(gs, &case.instance, &case.pending, &checks).0 == core_out);
    let jsonschema_agrees = match (&core_out, case.pending.is_empty()) {
        (Out::Val(val), true) => Some(build_validator(&compiled, None, None).expect("builds").is_valid(val)),
        _ => None,
    };
    let class = classify(&today_out, &core_out, &case.spec);
    Row {
        class,
        json: json!({
            "id": case.id,
            "source": case.source,
            "schema": schema_repr,
            "instance": case.instance,
            "pending": case.pending,
            "today": today_out.to_json(),
            "core": core_out.to_json(),
            "core_errors": core_errors,
            "spec_expectation": case.spec.as_ref().map(Out::to_json),
            "spec_ref": case.spec_ref,
            "classification": class,
            "translators_agree": grammar_agrees,
            "core_result_passes_compiled_jsonschema": jsonschema_agrees,
            "residual_keywords": translator.notes.borrow().clone(),
        }),
    }
}

fn fixture_rows() -> Vec<Row> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/validate");
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(&root)
        .expect("fixtures")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    let api = DarkmatterSchemas::new();
    let mut rows = Vec::new();
    for dir in dirs {
        let doc = dir.join("doc.md");
        let text = std::fs::read_to_string(&doc).expect("doc");
        let md: Markdown = text.as_str().into();
        let md = md.with_source(ComposeSource::File(doc.clone()));
        let Ok(Some(effective)) = api.effective_for(&md) else { continue };
        let instance = crate::markdown::schemas::frontmatter_as_json(&md);
        let coerced = coerce_frontmatter(&effective.json_schema, &instance).value;
        let today_out = if effective.validate(&instance).valid { Out::Val(coerced) } else { Out::Err };
        let checks = FormatChecks::new(effective.base_dir.clone());
        let translator = JsonTranslator::new(&effective.json_schema, effective.base_dir.clone());
        let shape = translator.translate_root();
        let (core_out, core_errors) = core_run(&shape, &instance, &[], &checks);
        let grammar_agrees = effective
            .simplified
            .as_ref()
            .and_then(from_grammar::translate_schema)
            .map(|gs| core_run(&gs, &instance, &[], &checks).0 == core_out);
        let jsonschema_agrees = match &core_out {
            Out::Val(val) => Some(effective.validator().is_valid(val)),
            Out::Err => None,
        };
        let name = dir.file_name().unwrap().to_string_lossy().into_owned();
        // spec-mandated changes among fixtures
        let spec = match name.as_str() {
            "boolish_valid" => Some(Out::Val(json!({"enabled":"true"}))),
            "numberlike_valid" => Some(Out::Val(json!({"count":"42"}))),
            _ => None,
        };
        let class = classify(&today_out, &core_out, &spec);
        rows.push(Row {
            class,
            json: json!({
                "id": format!("fixture/{name}"),
                "source": "L1: tests/fixtures/validate (schemas_validate_table.rs)",
                "schema": effective.json_schema.as_ref().clone(),
                "instance": instance,
                "pending": [],
                "today": today_out.to_json(),
                "core": core_out.to_json(),
                "core_errors": core_errors,
                "spec_expectation": spec.as_ref().map(Out::to_json),
                "spec_ref": if spec.is_some() { R_NL } else { "" },
                "classification": class,
                "translators_agree": grammar_agrees,
                "core_result_passes_compiled_jsonschema": jsonschema_agrees,
                "residual_keywords": translator.notes.borrow().clone(),
            }),
        });
    }
    rows
}

#[test]
fn spike_frontmatter_diff() {
    let mut rows: Vec<Row> = corpus().iter().map(run_case).collect();
    rows.extend(fixture_rows());
    let mut counts = std::collections::BTreeMap::new();
    for r in &rows {
        *counts.entry(r.class).or_insert(0) += 1;
    }
    println!("cases: {}  {counts:?}", rows.len());
    for r in &rows {
        if r.class.starts_with("UNEXPECTED") {
            println!("UNEXPECTED {}: today={} core={}", r.json["id"], r.json["today"], r.json["core"]);
        }
        if r.json["translators_agree"] == json!(false) {
            println!("TRANSLATOR MISMATCH {}", r.json["id"]);
        }
        if r.json["core_result_passes_compiled_jsonschema"] == json!(false) {
            println!("CORE-OK-BUT-JSONSCHEMA-REJECTS {}: core={}", r.json["id"], r.json["core"]);
        }
    }
    if let Ok(dir) = std::env::var("SPIKE_OUT") {
        let doc = json!({
            "generated_by": "darkmatter/lib/src/markdown/schemas/spike_engine/tests.rs::spike_frontmatter_diff",
            "summary": counts,
            "cases": rows.iter().map(|r| r.json.clone()).collect::<Vec<_>>(),
        });
        std::fs::write(
            PathBuf::from(dir).join("frontmatter-diff.json"),
            serde_json::to_string_pretty(&doc).expect("json"),
        )
        .expect("write diff");
    }
}

#[test]
fn spike_core_is_idempotent() {
    let checks = FormatChecks::new(None);
    for case in corpus() {
        let compiled = match &case.schema {
            S::Frag(fr) => wrap(fr),
            S::Json(j) => j.clone(),
            S::Grammar(y) => to_json_schema(&parse_yaml_schema(&serde_yaml_ng::from_str(y).unwrap()).unwrap()).unwrap(),
        };
        let shape = JsonTranslator::new(&compiled, None).translate_root();
        if let Ok(first) = core::bind_root(&shape, &case.instance, &case.pending, &checks) {
            let second = core::bind_root(&shape, &first.value, &case.pending, &checks).expect("rebinding succeeds");
            assert!(second.convs.is_empty(), "[{}] second pass converted again", case.id);
        }
    }
}

// ── question 3: union timing ──────────────────────────────────────────────

fn time<F: FnMut()>(iters: u32, mut f: F) -> f64 {
    for _ in 0..(iters / 10).max(1) {
        f();
    }
    let start = Instant::now();
    for _ in 0..iters {
        f();
    }
    start.elapsed().as_secs_f64() * 1e6 / f64::from(iters)
}

#[test]
fn spike_union_timing() {
    let inline = |props: Value| json!({"type":"object","properties":props,"additionalProperties":false});
    let scenarios: Vec<(&str, Value, Value)> = vec![
        (
            "property union number|boolean, '1'",
            json!({"type":"object","properties":{"x":{"anyOf":[{"type":"null"},{"type":"number"},{"type":"boolean"}]}}}),
            json!({"x":"1"}),
        ),
        (
            "property union {key,count}|string, object value",
            json!({"type":"object","properties":{"metadata":{"anyOf":[inline(json!({"key":{"type":"string"},"count":{"type":"number"}})),{"type":"string"}]}}}),
            json!({"metadata":{"key":"visits","count":"42"}}),
        ),
        (
            "property union json|yaml, native mapping",
            json!({"type":"object","properties":{"c":{"anyOf":[{"type":"string","format":"darkmatter-json"},{"type":"string","format":"darkmatter-yaml"}]}}}),
            json!({"c":{"port":8080,"debug":true}}),
        ),
        (
            "root union, 3 object arms (implement-like)",
            json!({"anyOf":[
                {"type":"object","additionalProperties":true,"properties":{"kind":{"type":"string"},"has_spec":{"type":"boolean"},"has_plan":{"type":"boolean"},"has_review":{"type":"boolean"}},"required":["kind"]},
                {"type":"object","additionalProperties":true,"properties":{"other":{"type":"string"}},"required":["other"]},
                {"type":"object","additionalProperties":true,"properties":{"third":{"type":"string"}},"required":["third"]}]}),
            json!({"kind":"implement","has_spec":"true","has_plan":"false","has_review":"false"}),
        ),
        (
            "no union: 8 scalar props",
            json!({"type":"object","properties":{
                "a":{"type":"string"},"b":{"type":"number"},"c":{"type":"boolean"},"d":{"type":"string","format":"date"},
                "e":{"type":"number","minimum":0},"f":{"type":"array","items":{"type":"string"}},"g":{"type":"string","minLength":1},"h":{"type":"integer"}}}),
            json!({"a":1,"b":"2","c":"true","d":"2026-01-01","e":"3","f":["x",1],"g":"y","h":"7"}),
        ),
    ];
    let iters = 2000;
    println!("| scenario | today coerce_frontmatter (µs/call, warm) | core translate once (µs) | core bind (µs/call) | full jsonschema validate (µs/call) |");
    println!("|---|---|---|---|---|");
    for (name, schema, instance) in scenarios {
        // today's path warm-up also fills the per-arm validator cache
        let today_us = time(iters, || {
            std::hint::black_box(coerce_frontmatter(&schema, &instance));
        });
        let t0 = Instant::now();
        let shape = JsonTranslator::new(&schema, None).translate_root();
        let translate_us = t0.elapsed().as_secs_f64() * 1e6;
        let checks = FormatChecks::new(None);
        let core_us = time(iters, || {
            std::hint::black_box(core::bind_root(&shape, &instance, &[], &checks).ok());
        });
        let validator = build_validator(&schema, None, None).expect("builds");
        let validate_us = time(iters, || {
            std::hint::black_box(validator.is_valid(&instance));
        });
        println!("| {name} | {today_us:.2} | {translate_us:.1} | {core_us:.2} | {validate_us:.2} |");
    }
    // cold: first-ever per-arm validator build (what a fresh process pays)
    let cold_schema = json!({"anyOf":[{"type":"number","minimum":12345.5},{"type":"boolean"}]});
    let t0 = Instant::now();
    let _ = build_validator(&cold_schema, None, None);
    println!("cold: one per-arm jsonschema validator build = {:.1} µs", t0.elapsed().as_secs_f64() * 1e6);
}

// ── question 4: numberlike / boolish as custom formats ────────────────────

#[test]
fn spike_numberlike_boolish_formats() {
    let nl_fmt = nf::build(&nf::numberlike_schema_format());
    let nl_pat = nf::build(&nf::numberlike_schema_pattern());
    let bo_fmt = nf::build(&nf::boolish_schema_format());
    let bo_pat = nf::build(&nf::boolish_schema_pattern());
    let checks = FormatChecks::new(None);
    let mut disagreements = Vec::new();
    for s in [
        "4", " 4 ", "+4", "-3.5", "1e3", ".5", "1_000", "04", "4.0", "2.5E-2", "9007199254740993",
        "99999999999999999999", "5.", "1__000", "_1", "1,000", "NaN", "inf", "0x10", "", "  ", "pear",
        "1e-400",
    ] {
        let value = json!(s);
        let core_ok = core::bind(&Shape::NumberLike, &value, "", &checks).is_ok();
        let (f, p) = (nl_fmt.is_valid(&value), nl_pat.is_valid(&value));
        if f != core_ok || p != core_ok {
            disagreements.push(format!("numberlike {s:?}: core={core_ok} format={f} pattern={p}"));
        }
    }
    for s in ["true", "TRUE", "tRuE", "yes", "On", "off", "1", "0", " yes ", "2", "y", "", "null"] {
        let value = json!(s);
        let core_ok = core::bind(&Shape::Boolish, &value, "", &checks).is_ok();
        let (f, p) = (bo_fmt.is_valid(&value), bo_pat.is_valid(&value));
        if f != core_ok || p != core_ok {
            disagreements.push(format!("boolish {s:?}: core={core_ok} format={f} pattern={p}"));
        }
    }
    for native in [json!(5), json!(true), Value::Null, json!([1])] {
        assert_eq!(nl_fmt.is_valid(&native), core::bind(&Shape::NumberLike, &native, "", &checks).is_ok(), "{native}");
        assert_eq!(bo_fmt.is_valid(&native), core::bind(&Shape::Boolish, &native, "", &checks).is_ok(), "{native}");
    }
    println!("numberlike/boolish disagreements with core:");
    for d in &disagreements {
        println!("  {d}");
    }
    let format_disagrees = disagreements.iter().any(|d| {
        let core_ok = d.contains("core=true");
        d.contains(&format!("format={}", !core_ok))
    });
    assert!(!format_disagrees, "format-based numberlike/boolish must match the core exactly");
    // translator recognizes the format-based fragments
    let wrapped = json!({"type":"object","properties":{"n": nf::numberlike_schema_format(), "b": nf::boolish_schema_format()}});
    let shape = JsonTranslator::new(&wrapped, None).translate_root();
    let got = core::bind(&shape, &json!({"n":"02134","b":"On"}), "", &checks).expect("binds");
    assert_eq!(got.value, json!({"n":"02134","b":"On"}));
    assert!(got.convs.is_empty());
}
