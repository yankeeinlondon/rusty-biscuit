//! Validates in-memory `serde_json::Value`s against the spec's `Person`
//! schema with nothing but `simplified-schema`: no Markdown host, no file.
//!
//! The schema's typed form is inspected before anything is compiled; the
//! first validation prepares it, and every later one reuses that.
//!
//! ```sh
//! cargo run -p simplified-schema --example validate_person
//! cargo run -p simplified-schema --no-default-features --example validate_person
//! ```

use serde_json::json;
use simplified_schema::{SimplifiedSchema, ValidationFailure};

const PERSON: &str = "\
kind: schema
types:
  Person:
    name: string(required)
    age: number(min(0); integer)
";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let catalog = SimplifiedSchema::try_from(PERSON)?;

    // Inspect the AST before any validation compiles anything.
    for path in ["Person.name", "Person.age"] {
        let definition = catalog.query(path)?.ok_or("the type is declared")?;
        println!("{path}: {definition} (required: {})", definition.is_required());
    }

    let person = catalog.select("Person")?;
    let values = [
        json!({ "name": "Ada", "age": 36 }),
        json!({ "name": "Grace", "age": "85" }),
        json!({ "age": 2.5 }),
    ];
    for value in &values {
        let strict = match person.validate_strict(value) {
            Ok(_) => "accepted".to_string(),
            Err(failure) => describe(&failure),
        };
        let soft = match person.validate_soft(value) {
            Ok(success) if success.is_converted() => format!("accepted as {}", success.value),
            Ok(_) => "accepted as written".to_string(),
            Err(failure) => describe(&failure),
        };
        println!("{value}\n  strict: {strict}\n  soft:   {soft}");
    }
    Ok(())
}

fn describe(failure: &ValidationFailure) -> String {
    let problems: Vec<String> = failure
        .problems()
        .iter()
        .map(|problem| format!("{} ({})", problem.message, problem.path))
        .collect();
    format!("rejected: {}", problems.join("; "))
}
