//! Darkmatter's numeric model, owned by `simplified_schema::numeric` and
//! re-exported here at its original path (one implementation, same types).
//! The user-facing rules are in `darkmatter/docs/topics/numbers.md`.

// migration: removed in Phase 7
pub use simplified_schema::numeric::{
    NumberTextError, NumericAdmissionError, admit_f64, admit_integer, admit_json_number, admit_number_spelling,
    admit_number_text, format_number, json_number, parse_boolean_word, reject_non_finite_json5,
};
// migration: removed in Phase 7
pub(crate) use simplified_schema::numeric::{admit_json_text_numbers, is_yaml_admission_error, load_admitted_yaml};
