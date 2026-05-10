use claudine::actions::HookAction;

pub(super) fn truncate_str(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}...", &s[..max])
    } else {
        s.to_string()
    }
}

pub(super) fn summarize_actions(actions: &[HookAction], max_width: usize) -> String {
    let summaries: Vec<String> = actions
        .iter()
        .map(|action| match action {
            HookAction::SoundEffect { effect, .. } => format!("Sound({effect})"),
            HookAction::Speak { message, .. } => {
                let preview = truncate_str(message, 20);
                format!("Speak(\"{preview}\")")
            }
            HookAction::Message { message, .. } => {
                let preview = truncate_str(message, 20);
                format!("Message(\"{preview}\")")
            }
            HookAction::Bash { command, .. } => {
                let preview = truncate_str(command, 20);
                format!("Shell(\"{preview}\")")
            }
            HookAction::Report { .. } => "Report".to_string(),
            HookAction::Call { command, .. } => format!("Call({command})"),
            _ => action.type_pascal_case().to_string(),
        })
        .collect();
    let joined = summaries.join(", ");
    truncate_str(&joined, max_width)
}

pub(super) fn format_action_detail(action: &HookAction) -> String {
    match action {
        HookAction::SoundEffect { effect, .. } => format!("Sound Effect: {effect}"),
        HookAction::Speak { message, .. } => format!("Speak: \"{}\"", truncate_str(message, 40)),
        HookAction::Message { message, .. } => {
            format!("Message: \"{}\"", truncate_str(message, 40))
        }
        HookAction::Bash { command, .. } => {
            format!("Shell Command: \"{}\"", truncate_str(command, 40))
        }
        HookAction::Report { .. } => "Report (to STDOUT)".to_string(),
        HookAction::Call { command, .. } => format!("Call: {command}"),
        _ => action.type_pascal_case().to_string(),
    }
}
