//! Validates in-memory `serde_json::Value`s against the spec's `Person`
//! schema with nothing but `simplified-schema`: no Markdown host, no file.
//!
//! The schema's typed form is inspected before anything is compiled; the
//! first validation prepares it, and every later one reuses that. `role` is
//! required but has a default, so a person without one passes: strict
//! validation reports the default it relied on, and soft validation inserts
//! it.
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
    role: string(required; default(member))
";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let catalog = SimplifiedSchema::try_from(PERSON)?;

    // Inspect the AST before any validation compiles anything.
    for path in ["Person.name", "Person.age", "Person.role"] {
        let definition = catalog.query(path)?.ok_or("the type is declared")?;
        println!("{path}: {definition} (required: {})", definition.is_required());
    }

    let person = catalog.select("Person")?;
    let values = [
        json!({ "name": "Ada", "age": 36, "role": "admin" }),
        json!({ "name": "Grace", "age": "85", "role": "admin" }),
        json!({ "name": "Linus", "age": 27 }),
        json!({ "age": 2.5 }),
    ];
    for value in &values {
        let strict = match person.validate_strict(value) {
            Ok(success) if success.defaults.is_empty() => "accepted".to_string(),
            Ok(success) => {
                let relied_on: Vec<String> = success
                    .defaults
                    .iter()
                    .map(|applied| format!("{} = {}", applied.path, applied.value))
                    .collect();
                format!("accepted with defaults {}", relied_on.join(", "))
            }
            Err(failure) => describe(&failure),
        };
        let soft = match person.validate_soft(value) {
            Ok(success) if success.is_modified() => format!("accepted as {}", success.value),
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
