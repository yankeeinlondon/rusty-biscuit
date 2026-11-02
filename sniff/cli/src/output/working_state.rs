//! `sniff repo state` output.

use serde_json::{Value, json};
use sniff::filesystem::git::{PathContent, WorkingState};

/// `{ id, head, changed: [{ path, content }] }`; `content` is the token that
/// entered the id (`file:<hash>`, `symlink:<hash>`, `directory`, `absent`).
pub(crate) fn working_state_json(state: &WorkingState) -> Value {
    json!({
        "id": state.id,
        "head": state.head,
        "changed": state.changed.iter().map(|entry| json!({
            "path": entry.path.to_string_lossy(),
            "content": entry.content.token(),
        })).collect::<Vec<_>>(),
    })
}

/// The id first, alone on its line, so `sniff repo state | head -1` is the id.
pub(crate) fn render_working_state(state: &WorkingState) -> String {
    let mut out = format!(
        "{}\nhead: {}\nchanged: {}\n",
        state.id,
        state.head.as_deref().unwrap_or("unborn"),
        state.changed.len()
    );
    for entry in &state.changed {
        let content = match entry.content {
            PathContent::Absent => "deleted".to_owned(),
            other => other.token(),
        };
        out.push_str(&format!("  {}  {content}\n", entry.path.display()));
    }
    out
}
