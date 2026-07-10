pub(super) fn unescape_brackets(output: &mut String) {
    // Only process if there are escaped brackets
    if !output.contains("\\[") {
        return;
    }

    let mut result = String::with_capacity(output.len());
    let mut chars = output.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.peek() {
                Some('[') => {
                    // Look ahead to see if this could be a link
                    // Pattern: \[...\] or \[...](...)
                    // We want to unescape standalone \[...\] that aren't links
                    chars.next(); // consume '['

                    // Collect until we find \] or ]( or end
                    let mut bracket_content = String::new();
                    let mut found_close = false;

                    while let Some(&next) = chars.peek() {
                        if next == '\\' {
                            chars.next();
                            if chars.peek() == Some(&']') {
                                chars.next();
                                found_close = true;
                                break;
                            } else {
                                bracket_content.push('\\');
                            }
                        } else if next == ']' {
                            chars.next();
                            found_close = true;
                            // Check if followed by ( - would make this a link
                            if chars.peek() == Some(&'(') {
                                // This is actually a link, restore and keep escape
                                result.push_str("\\[");
                                result.push_str(&bracket_content);
                                result.push(']');
                                break;
                            }
                            break;
                        } else if next == '\n' {
                            // Line break - not a link, but stop searching
                            break;
                        } else {
                            bracket_content.push(chars.next().unwrap());
                        }
                    }

                    if found_close {
                        // Unescape: output [content] instead of \[content\]
                        result.push('[');
                        result.push_str(&bracket_content);
                        result.push(']');
                    } else {
                        // Didn't find proper close, restore original
                        result.push_str("\\[");
                        result.push_str(&bracket_content);
                        // Restore chars iterator - actually we can't easily do this
                        // Just continue from where we are
                    }
                }
                Some(']') => {
                    // Standalone escaped ] - keep as is (shouldn't happen often)
                    result.push('\\');
                    result.push(chars.next().unwrap());
                }
                _ => {
                    result.push(c);
                }
            }
        } else {
            result.push(c);
        }
    }

    *output = result;
}
