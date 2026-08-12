use serde_json::Value;

pub(super) fn require_args(name: &str, args: &[Value], expected: usize) -> Result<(), String> {
    if args.len() != expected {
        Err(format!(
            "{name}() requires {expected} argument{}",
            if expected == 1 { "" } else { "s" }
        ))
    } else {
        Ok(())
    }
}
