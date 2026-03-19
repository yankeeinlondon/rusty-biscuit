//! Wrapper formatters for transcluded content.

/// Wraps content as a markdown block quote.
pub fn wrap_quotation(content: &str, attribution: Option<&str>) -> String {
    let mut lines: Vec<String> = content
        .lines()
        .map(|line| {
            if line.is_empty() {
                ">".to_string()
            } else {
                format!("> {line}")
            }
        })
        .collect();

    if let Some(attribution) = attribution
        && !attribution.is_empty()
    {
        lines.push(">".to_string());
        lines.push(format!("> — {attribution}"));
    }

    lines.join("\n")
}

/// Wraps content in an HTML disclosure block.
pub fn wrap_disclosure(content: &str, summary: &str) -> String {
    format!(
        "<details>\n<summary>{summary}</summary>\n\n{}\n\n</details>",
        content.trim_matches('\n')
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_quotation_without_attribution() {
        let wrapped = wrap_quotation("line1\nline2", None);
        assert_eq!(wrapped, "> line1\n> line2");
    }

    #[test]
    fn wraps_quotation_with_attribution() {
        let wrapped = wrap_quotation("line", Some("Alice"));
        assert_eq!(wrapped, "> line\n>\n> — Alice");
    }

    #[test]
    fn wraps_disclosure() {
        let wrapped = wrap_disclosure("content", "More");
        assert!(wrapped.contains("<details>"));
        assert!(wrapped.contains("<summary>More</summary>"));
        assert!(wrapped.contains("content"));
        assert!(wrapped.contains("</details>"));
    }
}
