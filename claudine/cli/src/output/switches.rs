pub(crate) fn style_cli_switches(message: &str) -> String {
    let bytes = message.as_bytes();
    let mut i = 0usize;
    let mut last = 0usize;
    let mut out = String::with_capacity(message.len() + 32);

    while i < bytes.len() {
        let Some((start, end)) = next_switch_span(bytes, i) else {
            break;
        };
        out.push_str(&message[last..start]);
        out.push_str("<blue>");
        out.push_str(&message[start..end]);
        out.push_str("</blue>");
        i = end;
        last = end;
    }

    out.push_str(&message[last..]);
    out
}

fn next_switch_span(bytes: &[u8], start_at: usize) -> Option<(usize, usize)> {
    let mut i = start_at;
    while i < bytes.len() {
        if bytes[i] == b'-' && is_switch_boundary(bytes, i) {
            if i + 2 < bytes.len() && bytes[i + 1] == b'-' && is_switch_start(bytes[i + 2]) {
                let mut j = i + 3;
                while j < bytes.len() && is_switch_continue(bytes[j]) {
                    j += 1;
                }
                return Some((i, j));
            }
            if i + 1 < bytes.len() && is_switch_start(bytes[i + 1]) {
                let mut j = i + 2;
                while j < bytes.len() && is_switch_continue(bytes[j]) {
                    j += 1;
                }
                return Some((i, j));
            }
        }
        i += 1;
    }
    None
}

fn is_switch_boundary(bytes: &[u8], index: usize) -> bool {
    if index == 0 {
        return true;
    }

    !bytes[index - 1].is_ascii_alphanumeric()
}

fn is_switch_start(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
}

fn is_switch_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'-'
}

/// Truncate the passthrough args string to fit within `max_chars` visible columns.
///
/// When the args exceed the available space, we cut 4 characters before the limit
/// and append `..."` so the total fits. If there's not even room for the ellipsis,
/// we return just `..."`.
pub(crate) fn truncate_args(args: &str, max_chars: usize) -> String {
    if args.len() <= max_chars {
        return args.to_string();
    }

    const SUFFIX: &str = "...\"";
    const SUFFIX_LEN: usize = 4;

    if max_chars <= SUFFIX_LEN {
        return SUFFIX.to_string();
    }

    let cut_at = max_chars - SUFFIX_LEN;

    let truncated = if args.is_char_boundary(cut_at) {
        &args[..cut_at]
    } else {
        let mut pos = cut_at;
        while pos > 0 && !args.is_char_boundary(pos) {
            pos -= 1;
        }
        &args[..pos]
    };

    format!("{truncated}{SUFFIX}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn truncate_args_no_op_when_fits() {
        let args = "--flag 'short prompt'";
        assert_eq!(truncate_args(args, 50), args);
    }

    #[test]
    fn truncate_args_exact_fit() {
        let args = "--flag 'hello'";
        assert_eq!(truncate_args(args, args.len()), args);
    }

    #[test]
    fn truncate_args_truncates_with_suffix() {
        let args = "--dangerously-skip-permissions 'this is a very long prompt that goes on'";
        let result = truncate_args(args, 40);
        assert!(result.ends_with("...\""));
        assert_eq!(result.len(), 40);
    }

    #[test]
    fn truncate_args_tiny_budget() {
        let args = "'some long prompt'";
        let result = truncate_args(args, 4);
        assert_eq!(result, "...\"");
    }

    #[test]
    fn truncate_args_budget_smaller_than_suffix() {
        let args = "'some long prompt'";
        let result = truncate_args(args, 2);
        assert_eq!(result, "...\"");
    }

    #[test]
    fn truncate_args_respects_char_boundaries() {
        let args = "ééééééééééé";
        let result = truncate_args(args, 10);
        assert!(result.ends_with("...\""));
        assert!(result.is_char_boundary(0));
    }

    #[test]
    fn style_cli_switches_only_wraps_switch_tokens() {
        let styled = style_cli_switches("Use --plain or -v but do not touch email@example.com.");
        assert!(styled.contains("<blue>--plain</blue>"));
        assert!(styled.contains("<blue>-v</blue>"));
        assert!(styled.contains("email@example.com"));
    }

    proptest! {
        #[test]
        fn truncate_args_respects_budget_for_arbitrary_utf8(input in any::<String>(), budget in 0usize..128) {
            let truncated = truncate_args(&input, budget);

            if input.len() > budget && budget <= 4 {
                prop_assert_eq!(truncated.as_str(), "...\"");
            } else {
                prop_assert!(truncated.len() <= budget);
            }

            prop_assert!(std::str::from_utf8(truncated.as_bytes()).is_ok());
        }
    }
}
