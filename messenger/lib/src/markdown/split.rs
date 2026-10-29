//! Splitting rendered provider text into posts that fit a length limit.

/// Split rendered Markdown into consecutive chunks of at most `max_chars`
/// Unicode scalar values each.
///
/// Text that already fits is returned unchanged as a single chunk. Longer text
/// is broken at the last paragraph boundary (a blank line outside a code
/// fence) that fits, else at the last line boundary that fits, else hard-split
/// at a `char` boundary. A chunk that ends inside a fenced code block is closed
/// with the fence marker and the next chunk reopens the fence with the original
/// opening line (info string included), so every chunk is valid Markdown on
/// its own. Trailing whitespace at each break is dropped, and whitespace-only
/// chunks are never produced.
///
/// ## Examples
///
/// ```
/// use messenger::markdown::split::split_to_char_limit;
///
/// let chunks = split_to_char_limit("first paragraph\n\nsecond paragraph", 20);
/// assert_eq!(chunks, ["first paragraph", "second paragraph"]);
/// ```
///
/// ## Panics
///
/// Panics if `max_chars` is zero.
pub fn split_to_char_limit(text: &str, max_chars: usize) -> Vec<String> {
    assert!(max_chars > 0, "max_chars must be positive");
    if text.chars().count() <= max_chars {
        return vec![text.to_owned()];
    }

    // A line is `(text, is_fragment)`; fragments are the remainders of a
    // hard-split line and never open or close a fence.
    let mut lines: Vec<(&str, bool)> = text.split_inclusive('\n').map(|l| (l, false)).collect();
    let mut chunks: Vec<String> = Vec::new();
    let mut fence_at_start: Option<Fence> = None;
    let mut start = 0;

    while start < lines.len() {
        let mut chunk = String::new();
        if let Some(fence) = &fence_at_start {
            chunk.push_str(&fence.open_line);
            chunk.push('\n');
        }
        let mut chunk_chars = chunk.chars().count();
        let mut fence = fence_at_start.clone();
        // (line index to resume from, byte length of `chunk` at that point)
        let mut last_paragraph_break: Option<(usize, usize)> = None;
        let mut next = start;
        let mut has_body = false;

        while next < lines.len() {
            let (line, is_fragment) = lines[next];
            let fence_after = if is_fragment {
                fence.clone()
            } else {
                Fence::advance(fence.as_ref(), line)
            };
            let line_chars = line.chars().count();
            if chunk_chars + line_chars + Fence::closing_cost(fence_after.as_ref()) > max_chars {
                break;
            }
            let toggles_fence = fence.is_some() != fence_after.is_some();
            has_body |= !toggles_fence && !line.trim().is_empty();
            chunk.push_str(line);
            chunk_chars += line_chars;
            fence = fence_after;
            next += 1;
            if fence.is_none() && line.trim().is_empty() {
                last_paragraph_break = Some((next, chunk.len()));
            }
        }

        if next < lines.len() && !has_body {
            // The chunk holds nothing but blank lines or fence markers, so
            // breaking here would emit an empty post: hard-split the line that
            // did not fit at a char boundary instead.
            let mut available =
                max_chars.saturating_sub(chunk_chars + Fence::closing_cost(fence.as_ref()));
            if available == 0 {
                // A fence opening line too long to leave room for content:
                // drop the fence wrapping for this chunk rather than loop.
                chunk.clear();
                fence = None;
                next = start;
                available = max_chars;
            }
            let (line, _) = lines[next];
            let cut = line
                .char_indices()
                .nth(available)
                .map_or(line.len(), |(index, _)| index);
            chunk.push_str(&line[..cut]);
            if cut < line.len() {
                lines[next] = (&line[cut..], true);
            } else {
                next += 1;
            }
        } else if next < lines.len()
            && let Some((resume, byte_len)) = last_paragraph_break
            && resume > start
        {
            // Paragraph breaks are only recorded outside a fence.
            chunk.truncate(byte_len);
            next = resume;
            fence = None;
        }

        if let Some(open) = &fence {
            if !chunk.ends_with('\n') {
                chunk.push('\n');
            }
            chunk.push_str(&open.marker);
        }
        let trimmed = chunk.trim_end();
        if !trimmed.trim_start().is_empty() {
            chunks.push(trimmed.to_owned());
        }
        fence_at_start = fence;
        if next > start {
            start = next;
        }
    }

    chunks
}

/// An open fenced code block.
#[derive(Debug, Clone)]
struct Fence {
    /// The opening line without its line ending, e.g. "```rust".
    open_line: String,
    /// The run of backticks or tildes that opened the block.
    marker: String,
}

impl Fence {
    /// Fence state after `line`, given the state before it.
    fn advance(open: Option<&Fence>, line: &str) -> Option<Fence> {
        let trimmed = line.trim();
        match open {
            Some(fence) => {
                let fence_char = fence.marker.chars().next().unwrap_or('`');
                let closes =
                    trimmed.len() >= fence.marker.len() && trimmed.chars().all(|c| c == fence_char);
                if closes { None } else { Some(fence.clone()) }
            }
            None => {
                let fence_char = trimmed.chars().next()?;
                if fence_char != '`' && fence_char != '~' {
                    return None;
                }
                let marker: String = trimmed.chars().take_while(|&c| c == fence_char).collect();
                (marker.len() >= 3).then(|| Fence {
                    open_line: line.trim_end_matches(['\n', '\r']).to_owned(),
                    marker,
                })
            }
        }
    }

    /// Characters needed to close `open` at the end of a chunk: a newline
    /// plus the marker.
    fn closing_cost(open: Option<&Fence>) -> usize {
        open.map_or(0, |fence| fence.marker.len() + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn char_len(chunk: &str) -> usize {
        chunk.chars().count()
    }

    #[test]
    fn text_at_limit_is_one_chunk() {
        let text = "a".repeat(2000);
        assert_eq!(split_to_char_limit(&text, 2000), vec![text]);
    }

    #[test]
    fn text_one_over_limit_is_two_chunks() {
        let text = "a".repeat(2001);
        let chunks = split_to_char_limit(&text, 2000);
        assert_eq!(chunks.len(), 2);
        assert_eq!(char_len(&chunks[0]), 2000);
        assert_eq!(chunks[1], "a");
    }

    #[test]
    fn prefers_paragraph_boundary_over_line_boundary() {
        let text = "para one line one\npara one line two\n\npara two line one\npara two line two";
        let chunks = split_to_char_limit(text, 50);
        assert_eq!(
            chunks,
            [
                "para one line one\npara one line two",
                "para two line one\npara two line two"
            ]
        );
    }

    #[test]
    fn falls_back_to_line_boundary_inside_a_long_paragraph() {
        let text = "line one\nline two\nline three";
        let chunks = split_to_char_limit(text, 18);
        assert_eq!(chunks, ["line one\nline two", "line three"]);
    }

    #[test]
    fn hard_split_never_cuts_a_multibyte_character() {
        // 'é' is two bytes and '😀' four; a byte-based cut would land inside them.
        let text = format!("{}é😀{}", "a".repeat(1999), "b".repeat(5));
        let chunks = split_to_char_limit(&text, 2000);
        assert_eq!(chunks.len(), 2);
        assert_eq!(char_len(&chunks[0]), 2000);
        assert!(chunks[0].ends_with('é'));
        assert_eq!(chunks[1], "😀bbbbb");
        assert_eq!(chunks.concat(), text);
    }

    #[test]
    fn counts_characters_not_bytes() {
        let text = "é".repeat(2000);
        assert_eq!(split_to_char_limit(&text, 2000), vec![text]);
    }

    #[test]
    fn long_code_block_is_closed_and_reopened_in_every_chunk() {
        let body: String = (0..300)
            .map(|i| format!("let value_{i} = {i};\n"))
            .collect();
        let text = format!("Intro paragraph.\n\n```rust\n{body}```\n\nOutro paragraph.");
        let chunks = split_to_char_limit(&text, 2000);
        assert!(
            chunks.len() > 2,
            "expected several chunks, got {}",
            chunks.len()
        );

        for chunk in &chunks {
            assert!(
                char_len(chunk) <= 2000,
                "chunk over limit: {}",
                char_len(chunk)
            );
            let fence_lines = chunk.lines().filter(|l| l.starts_with("```")).count();
            assert_eq!(fence_lines % 2, 0, "unbalanced fence in chunk:\n{chunk}");
        }
        // The intro breaks off at its paragraph boundary rather than
        // dragging the fence opening into its chunk.
        assert_eq!(chunks[0], "Intro paragraph.");
        assert!(
            chunks
                .last()
                .is_some_and(|c| c.ends_with("```\n\nOutro paragraph."))
        );
        for chunk in &chunks[1..] {
            assert!(
                chunk.starts_with("```rust\n"),
                "chunk does not open fence:\n{chunk}"
            );
        }
        for chunk in &chunks[1..chunks.len() - 1] {
            assert!(
                chunk.ends_with("\n```"),
                "chunk does not close fence:\n{chunk}"
            );
        }
    }

    #[test]
    fn preserves_order_of_every_line() {
        let text: String = (0..500).map(|i| format!("item {i}\n")).collect();
        let chunks = split_to_char_limit(&text, 2000);
        assert!(chunks.len() > 1);
        let rejoined: Vec<&str> = chunks.iter().flat_map(|c| c.lines()).collect();
        let original: Vec<&str> = text.lines().collect();
        assert_eq!(rejoined, original);
    }

    #[test]
    fn hard_splits_an_overlong_line_inside_a_fence() {
        let text = format!("```\n{}\n```", "x".repeat(30));
        let chunks = split_to_char_limit(&text, 20);
        for chunk in &chunks {
            assert!(char_len(chunk) <= 20);
            assert!(
                chunk.starts_with("```\n") && chunk.ends_with("\n```"),
                "{chunk:?}"
            );
        }
        let payload: String = chunks
            .iter()
            .map(|c| c.trim_start_matches("```\n").trim_end_matches("\n```"))
            .collect();
        assert_eq!(payload, "x".repeat(30));
    }
}
