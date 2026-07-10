pub(super) fn fix_blockquote_formatting(output: &mut String) {
    // Process line by line for clarity
    let mut result = String::with_capacity(output.len());
    let mut lines = output.lines().peekable();
    let mut in_code_block = false;
    let mut prev_was_blockquote = false;

    while let Some(line) = lines.next() {
        // Track code blocks to avoid modifying content inside them
        if line.trim_start().starts_with("```") {
            in_code_block = !in_code_block;
            prev_was_blockquote = false;
            result.push_str(line);
            if lines.peek().is_some() {
                result.push('\n');
            }
            continue;
        }

        // Don't process lines inside code blocks
        if in_code_block {
            result.push_str(line);
            if lines.peek().is_some() {
                result.push('\n');
            }
            continue;
        }

        // Only fix blockquote lines (those starting with optional space + ">")
        let is_blockquote_line =
            line.starts_with('>') || (line.starts_with(' ') && line.trim_start().starts_with('>'));

        let fixed_line = if is_blockquote_line {
            fix_blockquote_line(line)
        } else {
            line.to_string()
        };

        // Check if this is an empty blockquote line (just "> " or nested like "> > ")
        let trimmed = fixed_line.trim_end();
        let is_empty_blockquote = trimmed.chars().all(|c| c == '>' || c == ' ')
            && trimmed.contains('>')
            && !trimmed.is_empty();

        // Only strip empty blockquote lines at the START of a blockquote (pulldown-cmark
        // artifact). Preserve them mid-blockquote where they represent intentional
        // paragraph breaks (e.g., blank line before an attribution).
        if is_empty_blockquote
            && !prev_was_blockquote
            && let Some(next_line) = lines.peek()
            && next_line.trim_start().starts_with('>')
        {
            // Skip this empty blockquote line at the start of the blockquote
            continue;
        }

        prev_was_blockquote = is_blockquote_line;

        result.push_str(&fixed_line);
        // Add newline unless this is the last line
        if lines.peek().is_some() {
            result.push('\n');
        }
    }

    // Preserve trailing newline if original had one
    if output.ends_with('\n') && !result.ends_with('\n') {
        result.push('\n');
    }

    *output = result;
}

/// Fixes a single blockquote line's prefix formatting.
///
/// Handles:
/// - Leading space: " > text" -> "> text"
/// - Multiple spaces after >: ">  > text" -> "> > text"
fn fix_blockquote_line(line: &str) -> String {
    let mut result = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    let mut in_prefix = true;

    // Skip leading space if followed by >
    if chars.peek() == Some(&' ') {
        let mut lookahead = chars.clone();
        lookahead.next(); // consume space
        if lookahead.peek() == Some(&'>') {
            chars.next(); // skip the leading space
        }
    }

    while let Some(c) = chars.next() {
        if in_prefix {
            if c == '>' {
                result.push(c);
                // After >, we expect exactly one space before content or next >
                // Skip any extra spaces, but keep one
                let mut space_count = 0;
                while chars.peek() == Some(&' ') {
                    chars.next();
                    space_count += 1;
                }
                // Add exactly one space after >
                if space_count > 0 || chars.peek().is_some() {
                    result.push(' ');
                }
                // Check if next char is another > (nested blockquote)
                if chars.peek() != Some(&'>') {
                    in_prefix = false;
                }
            } else if c == ' ' {
                // Skip spaces in prefix area (between > markers)
                continue;
            } else {
                in_prefix = false;
                result.push(c);
            }
        } else {
            result.push(c);
        }
    }

    result
}
