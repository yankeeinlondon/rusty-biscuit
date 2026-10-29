use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use serde_json::Value;

use super::FunctionBinding;
use crate::markdown::compose::expression::call_binding::BoundCall;

pub(super) const BINDINGS: &[FunctionBinding] = &[FunctionBinding::pure("terminal", &[], terminal)];

/// `terminal(content: string | null)`: renders Prose markup to a terminal
/// string with deterministic, non-interactive terminal settings; null passes
/// through.
fn terminal(call: &BoundCall<'_>) -> Result<Value, String> {
    Ok(call
        .args()
        .text(0)
        .map_or(Value::Null, |content| Value::String(Prose::new(content).render_optimistic(None))))
}
