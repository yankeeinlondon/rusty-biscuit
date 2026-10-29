// Spike source, archived 2026-10-24. NOT compiled from this location.
//
// It ran ONCE as an in-crate test module (several APIs it uses are
// crate-private: `functions::inspect_registration`, the runtime registry):
//   file:      darkmatter/lib/src/markdown/compose/expression/spike_deferred_operand.rs
//   registered in darkmatter/lib/src/markdown/compose/expression/mod.rs with
//              `#[cfg(test)] mod spike_deferred_operand;` after `pub mod semantics;`
//   command:   cd darkmatter && cargo nextest run -p darkmatter --lib -E 'test(spike_)' --no-capture
//   result:    5 tests run: 5 passed, 6612 skipped (macOS host)
// Both temporary edits were removed after the run.

//! DISPOSABLE spike for `2026-10-30-claudine-recursive-schemas`, Phase 1
//! "Probe Deferred Integration". Not production code.
//!
//! Every test prints its observations with `eprintln!` (visible with
//! `--no-capture`) and asserts the verdicts it reports, so a run proves them.
//! Nothing here evaluates an expression, enters a function handler, probes
//! the host, or runs an effect: only scanners, the parser, the coercion
//! engine, the call binder, and schema validation are called.

use std::cell::Cell;
use std::rc::Rc;

use serde_json::{Value, json};

use crate::markdown::Markdown;
use crate::markdown::coercion::{BindOptions, Bound, PathSegment, Shape, bind_with, render_path};
use crate::markdown::compose::expression::call_binding::{
    ArgumentProblem, CallBindingError, CompiledSignature, FunctionRegistration, SignatureProblem,
};
use crate::markdown::compose::expression::catalog::{expression_function_catalog, function_descriptors};
use crate::markdown::compose::expression::functions::inspect_registration;
use crate::markdown::compose::expression::{ExpressionFinder, parse};
use crate::markdown::schemas::{DarkmatterSchemas, EffectiveSchema, PositionMap, ValidationOptions};

// ── Template classification through the existing scanner + parser ────────

#[derive(Debug, PartialEq)]
enum Template {
    /// No `{{ … }}` span: a literal.
    Literal,
    /// At least one span, every span parses: deferred to event time.
    Deferred,
    /// A span whose expression does not parse: a diagnostic, never an exemption.
    Malformed(String),
}

/// The same scan `lifecycle::action_value_to_expr` performs
/// (`ExpressionFinder::find_all_plain` then `parse` per span); no substring test.
fn classify(text: &str) -> Template {
    let spans = ExpressionFinder::find_all_plain(text);
    if spans.is_empty() {
        return Template::Literal;
    }
    for span in &spans {
        if let Err(error) = parse(&span.expression) {
            return Template::Malformed(format!("{error}"));
        }
    }
    Template::Deferred
}

fn is_deferred(value: &Value) -> bool {
    matches!(value, Value::String(text) if classify(text) == Template::Deferred)
}

// ── Registrations ─────────────────────────────────────────────────────────

/// What an out-of-crate caller (Claudine) can build: catalog `function_type`s
/// through `builder_from_types`, named helper types resolved through the
/// public `FunctionCatalog::named_type`.
fn public_registration(name: &str) -> FunctionRegistration {
    let catalog = expression_function_catalog();
    let overloads = function_descriptors(name)
        .iter()
        .map(|descriptor| descriptor.function_type.clone())
        .collect();
    let resolver = |type_name: &str, reference: &str| {
        (reference == "this").then(|| catalog.named_type(type_name).cloned()).flatten()
    };
    FunctionRegistration::builder_from_types(name, overloads)
        .build_with(&resolver)
        .expect("catalog signature compiles")
}

/// The registration runtime dispatch binds against (crate-private accessor).
fn runtime_bind(name: &str, args: &[Value]) -> Result<(usize, bool), CallBindingError> {
    inspect_registration(name, |registration| {
        registration
            .bind_call(args)
            .map(|call| (call.overload(), call.is_exact()))
    })
    .expect("known function")
}

fn describe(error: &CallBindingError) -> String {
    match error {
        CallBindingError::Type { candidates, .. } => candidates
            .iter()
            .map(|candidate| match &candidate.problem {
                SignatureProblem::Arguments(errors) => format!(
                    "{} -> {}",
                    candidate.signature,
                    errors
                        .iter()
                        .map(|error| {
                            let detail = match &error.problem {
                                ArgumentProblem::Conversion(conversion) => conversion
                                    .failures
                                    .iter()
                                    .map(|failure| {
                                        format!("{} {:?}", render_path(&error.parameter, &failure.path), failure.reason)
                                    })
                                    .collect::<Vec<_>>()
                                    .join(", "),
                                other => format!("{} {other:?}", error.parameter),
                            };
                            format!("arg[{}] {detail}", error.index)
                        })
                        .collect::<Vec<_>>()
                        .join("; ")
                ),
                SignatureProblem::Arity { min, max, supplied } => {
                    format!("{} -> arity {min}..{max:?} got {supplied}", candidate.signature)
                }
            })
            .collect::<Vec<_>>()
            .join(" | "),
        other => format!("{other:?}"),
    }
}

// ── Spike-local deferral-aware binder ─────────────────────────────────────
//
// `bind_call` binds every argument with `coercion::bind` (no `BindOptions`),
// so it cannot defer. This reproduces its per-argument rule (source
// exclusions, then the null rule, then the engine) with `bind_with` and a
// template `defer` predicate, then applies `bind_call`'s tier selection. The
// reproduction is the proof that the integration is missing: production must
// not copy `bind_argument`.

#[derive(Debug, PartialEq)]
enum Verdict {
    /// One overload binds and no value was deferred: identical to `bind_call`.
    Bound { overload: usize, exact: bool },
    /// Some value was deferred; these overloads admit the literal parts.
    /// Selection among them happens at event time.
    Deferred { overloads: Vec<usize> },
    Ambiguous { overloads: Vec<usize>, exact: bool },
    Rejected(Vec<String>),
}

fn bind_signature_deferred(signature: &CompiledSignature, args: &[Value]) -> Result<(bool, bool), Vec<String>> {
    if !signature.accepts_count(args.len()) {
        let (min, max) = signature.arity();
        return Err(vec![format!("arity {min}..{max:?}, got {}", args.len())]);
    }
    // `coercion::Defer` is `dyn Fn(..)` in a type alias, so its object
    // lifetime defaults to `'static`: the predicate cannot borrow locals and
    // must own its state (here an `Rc`).
    let deferred_hit = Rc::new(Cell::new(false));
    let recorder = Rc::clone(&deferred_hit);
    let defer = move |_: &[PathSegment], value: &Value| {
        let hit = is_deferred(value);
        if hit {
            recorder.set(true);
        }
        hit
    };
    let options = BindOptions {
        defer: Some(&defer),
        ..BindOptions::default()
    };
    let mut exact = true;
    let mut errors = Vec::new();
    for (index, value) in args.iter().enumerate() {
        let parameter = signature.parameter_for(index).expect("arity admits every argument");
        if parameter.excluded_sources().contains(&crate::markdown::coercion::ValueKind::of(value)) {
            errors.push(format!("arg[{index}] {} SourceExcluded", parameter.name()));
        } else if value.is_null() && !parameter.admits_null() {
            errors.push(format!("arg[{index}] {} NullNotAdmitted", parameter.name()));
        } else {
            match bind_with(parameter.shape(), value, &options) {
                Ok(Bound::Kept(_)) => {}
                Ok(Bound::Converted(_)) => exact = false,
                Err(error) => errors.extend(error.failures.iter().map(|failure| {
                    format!("arg[{index}] {} {:?}", render_path(parameter.name(), &failure.path), failure.reason)
                })),
            }
        }
    }
    if errors.is_empty() { Ok((exact, deferred_hit.get())) } else { Err(errors) }
}

fn bind_deferred(registration: &FunctionRegistration, args: &[Value]) -> Verdict {
    let mut exact = Vec::new();
    let mut converted = Vec::new();
    let mut any_deferred = false;
    let mut failures = Vec::new();
    for (index, signature) in registration.overloads().iter().enumerate() {
        match bind_signature_deferred(signature, args) {
            Ok((is_exact, deferred)) => {
                any_deferred |= deferred;
                if is_exact { exact.push(index) } else { converted.push(index) }
            }
            Err(errors) => failures.push(format!("{signature} -> {}", errors.join("; "))),
        }
    }
    if any_deferred {
        let mut overloads: Vec<usize> = exact.iter().chain(&converted).copied().collect();
        overloads.sort_unstable();
        return if overloads.is_empty() { Verdict::Rejected(failures) } else { Verdict::Deferred { overloads } };
    }
    let (tier, is_exact) = if exact.is_empty() { (converted, false) } else { (exact, true) };
    match tier.as_slice() {
        [] => Verdict::Rejected(failures),
        [overload] => Verdict::Bound {
            overload: *overload,
            exact: is_exact,
        },
        _ => Verdict::Ambiguous {
            overloads: tier,
            exact: is_exact,
        },
    }
}

// ── 1. The five operand values against a numeric catalog parameter ────────

#[test]
fn spike_numeric_operand_verdicts() {
    let pr_list = public_registration("pr_list");
    for (index, signature) in pr_list.overloads().iter().enumerate() {
        eprintln!("pr_list overload {index}: {signature}");
    }

    // 3: the count overload binds exactly, at runtime and in the spike binder.
    assert_eq!(runtime_bind("pr_list", &[json!(3)]).unwrap(), (1, true));
    assert_eq!(bind_deferred(&pr_list, &[json!(3)]), Verdict::Bound { overload: 1, exact: true });
    eprintln!("3            -> runtime bind_call: overload 1 exact; spike: Bound(1, exact)");

    // "3": converted by the shared engine; exact=false.
    assert_eq!(runtime_bind("pr_list", &[json!("3")]).unwrap(), (1, false));
    assert_eq!(bind_deferred(&pr_list, &[json!("3")]), Verdict::Bound { overload: 1, exact: false });
    eprintln!("\"3\"          -> runtime bind_call: overload 1 converted; spike: Bound(1, converted)");

    // "three": every overload fails; the error names each candidate.
    let error = runtime_bind("pr_list", &[json!("three")]).unwrap_err();
    eprintln!("\"three\"      -> runtime bind_call: {:?} :: {}", error.kind(), describe(&error));
    assert!(matches!(error, CallBindingError::Type { .. }));
    assert!(matches!(bind_deferred(&pr_list, &[json!("three")]), Verdict::Rejected(_)));

    // "{{ fm.count }}": bind_call has no defer, so it REJECTS the template text.
    let template = json!("{{ fm.count }}");
    assert_eq!(classify("{{ fm.count }}"), Template::Deferred);
    let error = runtime_bind("pr_list", std::slice::from_ref(&template)).unwrap_err();
    eprintln!("\"{{{{ fm.count }}}}\" -> runtime bind_call (no defer): {:?} :: {}", error.kind(), describe(&error));
    // The spike binder defers it; both overloads admit a deferred scalar, so
    // overload selection must wait for the evaluated value.
    let verdict = bind_deferred(&pr_list, std::slice::from_ref(&template));
    eprintln!("\"{{{{ fm.count }}}}\" -> spike: {verdict:?}");
    assert_eq!(verdict, Verdict::Deferred { overloads: vec![0, 1] });

    // Single-overload numeric parameter for contrast: abs(x: number).
    let abs = public_registration("abs");
    assert_eq!(bind_deferred(&abs, std::slice::from_ref(&template)), Verdict::Deferred { overloads: vec![0] });

    // Malformed template syntax is not an exemption.
    assert!(matches!(classify("{{ 1 + }}"), Template::Malformed(_)));
    eprintln!("\"{{{{ 1 + }}}}\"    -> classify: {:?}", classify("{{ 1 + }}"));
    // An unclosed opener is no span at all: judged as a literal and rejected.
    assert_eq!(classify("{{ fm.count"), Template::Literal);
    assert!(matches!(bind_deferred(&abs, &[json!("{{ fm.count")]), Verdict::Rejected(_)));
    eprintln!("\"{{{{ fm.count\" -> classify: Literal (scanner skips an unclosed opener; no syntax diagnostic)");
}

// ── 2. Structured operands with deferred children ─────────────────────────

#[test]
fn spike_structured_operand_with_deferred_child() {
    // The positional argument list is the structured operand of `min: [a, b]`.
    let min = public_registration("min");
    let ok = bind_deferred(&min, &[json!(1), json!("{{ fm.x }}")]);
    eprintln!("min [1, \"{{{{ fm.x }}}}\"]       -> spike: {ok:?}");
    assert_eq!(ok, Verdict::Deferred { overloads: vec![0] });
    let bad = bind_deferred(&min, &[json!("three"), json!("{{ fm.x }}")]);
    eprintln!("min [\"three\", \"{{{{ fm.x }}}}\"] -> spike: {bad:?}");
    assert!(matches!(&bad, Verdict::Rejected(errors) if errors[0].contains("arg[0] a")));

    // A typed collection: only the deferred child is exempt; siblings convert
    // or fail with their own path.
    let deferred = |_: &[PathSegment], value: &Value| is_deferred(value);
    let options = BindOptions {
        defer: Some(&deferred),
        ..BindOptions::default()
    };
    let shape = Shape::array(Shape::number());
    let error = bind_with(&shape, &json!(["1", "{{ fm.x }}", "three"]), &options).unwrap_err();
    let paths: Vec<String> = error.failures.iter().map(|failure| render_path("counts", &failure.path)).collect();
    eprintln!("number[] [\"1\", \"{{{{ fm.x }}}}\", \"three\"] -> failures at {paths:?}");
    assert_eq!(paths, vec!["counts[2]".to_string()]);
    let partly_deferred = json!(["1", "{{ fm.x }}"]);
    let bound = bind_with(&shape, &partly_deferred, &options).unwrap();
    eprintln!("number[] [\"1\", \"{{{{ fm.x }}}}\"] -> {:?}", bound);
    assert_eq!(bound.value(), &json!([1, "{{ fm.x }}"]));

    // pr_list(query: object) with a deferred child: `object` is unconstrained,
    // so there is no sibling type to check.
    let pr_list = public_registration("pr_list");
    let verdict = bind_deferred(&pr_list, &[json!({"state": "open", "limit": "{{ fm.x }}"})]);
    eprintln!("pr_list {{state, limit: \"{{{{ fm.x }}}}\"}} -> spike: {verdict:?}");

    // What the lifecycle parser does with these operands today.
    for raw in [
        json!(3),
        json!("3"),
        json!("{{ fm.count }}"),
        json!("n={{ fm.count }}"),
        json!({"a": "{{ fm.x }}"}),
        json!([1, "{{ fm.x }}"]),
        Value::Null,
    ] {
        let parsed = crate::lifecycle::action_value_to_expr(&raw);
        eprintln!("action_value_to_expr({raw}) -> {}", match &parsed {
            Ok(expr) => format!("Ok({expr:?})"),
            Err(error) => format!("Err({error})"),
        });
    }
    assert!(crate::lifecycle::action_value_to_expr(&json!({"a": "{{ fm.x }}"})).is_err());
}

// ── 3. Optional-but-nonnullable parameter ─────────────────────────────────

#[test]
fn spike_optional_nonnullable_parameter() {
    let descriptor = &function_descriptors("ping")[0];
    for parameter in descriptor.parameters {
        eprintln!(
            "ping descriptor param {}: type_text={:?} optional={} rest={}",
            parameter.name, parameter.type_text, parameter.optional, parameter.rest
        );
    }
    let ping = public_registration("ping");
    let timeout = ping.overloads()[0].parameter_for(1).expect("timeout");
    eprintln!(
        "compiled timeout: optional={} admits_null={} type_text={:?}",
        timeout.is_optional(),
        timeout.admits_null(),
        timeout.type_text()
    );
    assert!(timeout.is_optional());
    assert!(!timeout.admits_null());

    let address = json!("192.168.1.1");
    assert!(runtime_bind("ping", std::slice::from_ref(&address)).is_ok());
    let error = runtime_bind("ping", &[address.clone(), Value::Null]).unwrap_err();
    eprintln!("ping(addr, null) -> runtime bind_call: {}", describe(&error));
    assert!(describe(&error).contains("NullNotAdmitted"));

    // The schema system's general rule: an optional property accepts null.
    let effective = effective_schema("$schema:\n  timeout: number\n  count: number\n  counts: \"number[]\"\n");
    let report = effective.validate(&json!({"timeout": null}));
    eprintln!("frontmatter {{timeout: null}} against `timeout: number` -> valid={}", report.valid);
    assert!(report.valid);
}

// ── 4. Frontmatter validation of a converted copy ─────────────────────────

fn effective_schema(yaml_body: &str) -> EffectiveSchema {
    let md: Markdown = format!("---\n{yaml_body}---\nbody\n").as_str().into();
    DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(std::env::temp_dir()))
        .effective_for(&md)
        .expect("effective_for")
        .expect("schema present")
}

#[test]
fn spike_frontmatter_converted_copy_and_pending() {
    let effective = effective_schema("$schema:\n  count: number\n  counts: \"number[]\"\n");
    let options = ValidationOptions::default();
    for instance in [
        json!({"count": 3}),
        json!({"count": "3"}),
        json!({"count": "three"}),
        json!({"count": "{{ fm.count }}"}),
        json!({"counts": ["1", "{{ fm.x }}", "three"]}),
    ] {
        let report = effective.validate_with_options(&instance, &PositionMap::new(), &options);
        let problems: Vec<String> = report.problems.iter().map(|problem| format!("{} {}", problem.path, problem.message)).collect();
        let pending: Vec<&str> = report.pending.iter().map(|pending| pending.key.as_str()).collect();
        eprintln!("frontmatter {instance} -> valid={} problems={problems:?} pending={pending:?}", report.valid);
    }
    let report = effective.validate_with_options(&json!({"count": "three"}), &PositionMap::new(), &options);
    assert!(!report.valid);
    let report = effective.validate_with_options(&json!({"count": "3"}), &PositionMap::new(), &options);
    assert!(report.valid);
}

// ── 5. Overload preference and AmbiguousOverload ──────────────────────────

#[test]
fn spike_overload_selection_rules() {
    let exact_over_converted = FunctionRegistration::builder(
        "pick",
        "function(parameters(x: number); returns(string)) | function(parameters(x: string); returns(string))",
    )
    .build()
    .expect("registration");
    let text_three = [json!("3")];
    let call = exact_over_converted.bind_call(&text_three).unwrap();
    eprintln!("pick(\"3\") over number|string -> overload {} exact={}", call.overload(), call.is_exact());
    assert_eq!((call.overload(), call.is_exact()), (1, true));

    let ambiguous = FunctionRegistration::builder(
        "amb",
        "function(parameters(x: number(min(0))); returns(number)) | function(parameters(x: number(max(10))); returns(number))",
    )
    .build()
    .expect("registration");
    let error = ambiguous.bind_call(&[json!(5)]).unwrap_err();
    eprintln!("amb(5) -> {error:?}");
    assert!(matches!(error, CallBindingError::AmbiguousOverload { exact: true, .. }));
    let error = ambiguous.bind_call(&[json!("5")]).unwrap_err();
    eprintln!("amb(\"5\") -> {error:?}");
    assert!(matches!(error, CallBindingError::AmbiguousOverload { exact: false, .. }));

    // A public registration omits the registry's source exclusions and
    // recoveries: `length(true)` diverges from runtime.
    let public = public_registration("length").bind_call(&[json!(true)]).map(|call| call.is_exact());
    let runtime = runtime_bind("length", &[json!(true)]);
    eprintln!("length(true) -> public registration {public:?}; runtime registry {:?}", runtime.as_ref().map_err(describe));
}
