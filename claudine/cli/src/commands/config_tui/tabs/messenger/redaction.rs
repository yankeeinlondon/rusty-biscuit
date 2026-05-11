/// Masked rendering of a webhook detail string in the configuration list.
///
/// Inline webhook URLs are never rendered raw in the TUI. Instead the
/// configuration list shows this constant as the secondary detail so secrets
/// never reach the visible buffer.
pub const MASKED_WEBHOOK_DETAIL: &str = "webhook: ********";
