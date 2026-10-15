// `CallError` is non-exhaustive, so a match outside the runtime that names
// only today's variants does not compile; a later variant cannot break it.
use schematic_runtime::{CallError, NoDeclaredError};

fn describe(error: &CallError<NoDeclaredError>) -> &'static str {
    match error {
        CallError::Transport(_) => "transport",
        CallError::Api { .. } => "api",
    }
}

fn main() {
    let _ = describe;
}
