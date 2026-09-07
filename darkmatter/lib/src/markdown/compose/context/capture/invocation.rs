use super::*;
use super::snapshot::ContextCapture;

pub(super) const KEYS: &[&str] = &["cwd"];

pub(super) fn populate_invocation(cap: &ContextCapture, values: &mut Map<String, Value>) {
    values.insert(
        "cwd".into(),
        cap.invocation_cwd.as_ref().map_or(Value::Null, |path| {
            Value::String(biscuit_file::to_portable_string(path))
        }),
    );
}
