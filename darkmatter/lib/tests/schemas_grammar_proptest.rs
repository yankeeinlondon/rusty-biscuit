//! Property-based round-trip tests for the SimplifiedSchema grammar.
//!
//! Each test generates a valid `PropertyAtom`, serialises it via
//! `serialize_property_atom`, and re-parses the resulting string. The parsed
//! atom must equal the original — proving the parser and serialiser agree on
//! every code path.

use darkmatter::markdown::schemas::simplified::{
    Constraint, PropertyAtom, SimplifiedType, TypeExpr, grammar::parse_type_expr,
    serialize_property_atom,
};
use proptest::prelude::*;

// ── Strategies ────────────────────────────────────────────────────────────

fn type_strategy() -> impl Strategy<Value = SimplifiedType> {
    prop_oneof![
        Just(SimplifiedType::String),
        Just(SimplifiedType::Date),
        Just(SimplifiedType::DateTime),
        Just(SimplifiedType::Time),
        Just(SimplifiedType::Number),
        Just(SimplifiedType::NumberLike),
        Just(SimplifiedType::Boolean),
        Just(SimplifiedType::Boolish),
        Just(SimplifiedType::Object),
        Just(SimplifiedType::File),
        Just(SimplifiedType::Url),
        Just(SimplifiedType::Email),
        Just(SimplifiedType::Literal),
        Just(SimplifiedType::Expression),
        Just(SimplifiedType::TypeDefinition),
        Just(SimplifiedType::Schema),
        Just(SimplifiedType::Any),
    ]
}

/// A typed literal value across all authorable scalar forms, including
/// quoting-sensitive strings (`a, b`) and strings that would otherwise
/// re-type on re-parse (`true`, `2` when produced by `bare_word`).
fn literal_value() -> impl Strategy<Value = serde_json::Value> {
    prop_oneof![
        bare_word().prop_map(serde_json::Value::String),
        (bare_word(), bare_word())
            .prop_map(|(a, b)| serde_json::Value::String(format!("{a}, {b}"))),
        (-100_000i64..100_000).prop_map(|n| serde_json::json!(n)),
        any::<bool>().prop_map(serde_json::Value::Bool),
    ]
}

/// A bare-word identifier safe to round-trip without quoting.
fn bare_word() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_-]{0,8}".prop_filter("non-empty", |s| !s.is_empty())
}

/// Glob-like pattern that includes `[`, `]`, `*`, `!`, etc. but no
/// whitespace, commas, semicolons, parens, or quotes.
fn glob_pattern() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9_/.*!^$+\\-\\[\\]]{1,12}".prop_filter("non-empty", |s| !s.is_empty())
}

fn item_constraints_for(ty: SimplifiedType) -> impl Strategy<Value = Vec<Constraint>> {
    let universal = prop_oneof![Just(Constraint::Required),];

    let typed: BoxedStrategy<Vec<Constraint>> = match ty {
        SimplifiedType::Number | SimplifiedType::NumberLike => (
            proptest::option::of(0i32..1_000_000),
            proptest::option::of(0i32..1_000_000),
            any::<bool>(),
        )
            .prop_map(|(min, max, integer)| {
                let mut out = vec![];
                if let Some(m) = min {
                    out.push(Constraint::Min(m as f64));
                }
                if let Some(m) = max {
                    out.push(Constraint::Max(m as f64));
                }
                if integer {
                    out.push(Constraint::Integer);
                }
                out
            })
            .boxed(),
        SimplifiedType::String => (
            proptest::option::of(0usize..200),
            proptest::option::of(0usize..200),
            any::<bool>(),
        )
            .prop_map(|(min, max, ne)| {
                let mut out = vec![];
                if let Some(m) = min {
                    out.push(Constraint::MinLen(m));
                }
                if let Some(m) = max {
                    out.push(Constraint::MaxLen(m));
                }
                if ne {
                    out.push(Constraint::NotEmpty);
                }
                out
            })
            .boxed(),
        SimplifiedType::Url => proptest::collection::vec(bare_word(), 0..3)
            .prop_map(|schemes| {
                if schemes.is_empty() {
                    vec![]
                } else {
                    vec![Constraint::Scheme(
                        schemes
                            .into_iter()
                            .map(|s| s.to_ascii_lowercase())
                            .collect(),
                    )]
                }
            })
            .boxed(),
        SimplifiedType::File => proptest::collection::vec(glob_pattern(), 1..4)
            .prop_map(|globs| vec![Constraint::Match(globs)])
            .boxed(),
        SimplifiedType::Enum => proptest::collection::vec(bare_word(), 1..5)
            .prop_map(|members| vec![Constraint::Members(members)])
            .boxed(),
        SimplifiedType::Literal => literal_value()
            .prop_map(|value| vec![Constraint::LiteralValue(value)])
            .boxed(),
        SimplifiedType::TypeDefinition => prop_oneof![
            Just(vec![]),
            Just(vec![Constraint::Generated]),
            Just(vec![Constraint::Default(serde_json::json!("string"))]),
        ]
        .boxed(),
        SimplifiedType::Schema => prop_oneof![
            Just(vec![]),
            Just(vec![Constraint::Generated]),
            Just(vec![Constraint::Default(serde_json::json!("./schema.yaml"))]),
        ]
        .boxed(),
        // Other types (including `expression`) only accept universal
        // constraints — handled via the outer `universal` strategy below.
        _ => Just(vec![]).boxed(),
    };

    (typed, proptest::option::of(universal)).prop_map(|(mut typed, req)| {
        if let Some(r) = req {
            typed.push(r);
        }
        typed
    })
}

fn array_constraints() -> impl Strategy<Value = Vec<Constraint>> {
    (
        proptest::option::of(0usize..100),
        proptest::option::of(0usize..100),
        any::<bool>(),
    )
        .prop_map(|(min, max, unique)| {
            let mut out = vec![];
            if let Some(m) = min {
                out.push(Constraint::MinItems(m));
            }
            if let Some(m) = max {
                out.push(Constraint::MaxItems(m));
            }
            if unique {
                out.push(Constraint::Unique);
            }
            out
        })
}

fn description_strategy() -> impl Strategy<Value = Option<String>> {
    proptest::option::of("[A-Za-z][A-Za-z0-9 ]{0,40}".prop_map(|s| s.trim().to_string()))
}

fn atom_strategy() -> impl Strategy<Value = PropertyAtom> {
    type_strategy().prop_flat_map(|ty| {
        let item = item_constraints_for(ty);
        let array = array_constraints();
        let is_arr = any::<bool>();
        let desc = description_strategy();
        (Just(ty), item, array, is_arr, desc).prop_map(
            |(ty, item, array, is_array, description)| PropertyAtom {
                ty: TypeExpr::Primitive(ty),
                is_array,
                constraints: item,
                array_constraints: if is_array { array } else { vec![] },
                description: description.filter(|d| !d.is_empty()),
            },
        )
    })
}

// ── Property tests ────────────────────────────────────────────────────────

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 256,
        .. ProptestConfig::default()
    })]

    #[test]
    fn round_trip_random_atoms(atom in atom_strategy()) {
        let serialized = serialize_property_atom(&atom);
        let reparsed = parse_type_expr("test", &serialized)
            .map_err(|e| TestCaseError::fail(format!(
                "re-parse of {serialized:?} failed: {e:?}"
            )))?;
        prop_assert_eq!(reparsed, atom, "round-trip failed for {:?}", serialized);
    }

    #[test]
    fn parser_does_not_panic_on_arbitrary_input(s in "[ -~]{0,40}") {
        // Calls into the parser with arbitrary printable ASCII; we only
        // assert that it never panics (Ok or Err is fine).
        let _ = parse_type_expr("fuzz", &s);
    }
}
