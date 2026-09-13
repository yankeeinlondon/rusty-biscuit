//! Byte-preserving Rust source sanitizer shared by this crate's structural
//! gate (`spawn_site_guard.rs`).
//!
//! The gate answers "does this file *do* X?" by searching source text, and it
//! must not fire on an X that appears in a doc comment or a string literal —
//! the guard's own explanatory prose names the very APIs it forbids.
//! [`sanitize`] blanks comments and literals while keeping every byte offset
//! and newline in place, so a hit in the sanitized buffer indexes straight
//! back into the original source (line numbers, and the literal text a call
//! site passed).

/// Whether `byte` may appear inside a Rust identifier.
pub fn is_ident(byte: u8) -> bool {
    byte == b'_' || byte.is_ascii_alphanumeric()
}

/// Blanks comments and literals while preserving bytes and newline offsets.
pub fn sanitize(source: &str) -> Vec<u8> {
    let bytes = source.as_bytes();
    let mut output = bytes.to_vec();
    let blank = |output: &mut [u8], from: usize, to: usize| {
        for byte in output.iter_mut().take(to).skip(from) {
            if !matches!(*byte, b'\n' | b'\r') {
                *byte = b' ';
            }
        }
    };

    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'/') {
            let start = index;
            while index < bytes.len() && !matches!(bytes[index], b'\n' | b'\r') {
                index += 1;
            }
            blank(&mut output, start, index);
        } else if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'*') {
            let start = index;
            let mut depth = 1usize;
            index += 2;
            while index < bytes.len() && depth > 0 {
                if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'*') {
                    depth += 1;
                    index += 2;
                } else if bytes[index] == b'*' && bytes.get(index + 1) == Some(&b'/') {
                    depth -= 1;
                    index += 2;
                } else {
                    index += 1;
                }
            }
            blank(&mut output, start, index);
        } else if let Some((content_start, hashes)) = raw_string_open(bytes, index) {
            let start = index;
            index = content_start;
            while index < bytes.len() {
                if bytes[index] == b'"'
                    && bytes[index + 1..].len() >= hashes
                    && bytes[index + 1..]
                        .iter()
                        .take(hashes)
                        .all(|&byte| byte == b'#')
                {
                    index += 1 + hashes;
                    break;
                }
                index += 1;
            }
            blank(&mut output, start, index);
        } else if bytes[index] == b'"' {
            let start = index;
            index += 1;
            while index < bytes.len() {
                if bytes[index] == b'\\' {
                    index = (index + 2).min(bytes.len());
                } else if bytes[index] == b'"' {
                    index += 1;
                    break;
                } else {
                    index += 1;
                }
            }
            blank(&mut output, start, index);
        } else if bytes[index] == b'\'' {
            if let Some(end) = char_literal_end(bytes, index) {
                blank(&mut output, index, end);
                index = end;
            } else {
                index += 1;
            }
        } else {
            index += 1;
        }
    }
    output
}

fn raw_string_open(bytes: &[u8], index: usize) -> Option<(usize, usize)> {
    if index > 0 && is_ident(bytes[index - 1]) {
        return None;
    }
    let mut cursor = index;
    if bytes.get(cursor) == Some(&b'b') {
        cursor += 1;
    }
    if bytes.get(cursor) != Some(&b'r') {
        return None;
    }
    cursor += 1;
    let mut hashes = 0;
    while bytes.get(cursor) == Some(&b'#') {
        hashes += 1;
        cursor += 1;
    }
    (bytes.get(cursor) == Some(&b'"')).then_some((cursor + 1, hashes))
}

fn char_literal_end(bytes: &[u8], index: usize) -> Option<usize> {
    let mut cursor = index + 1;
    if bytes.get(cursor) == Some(&b'\\') {
        cursor += 1;
        if bytes.get(cursor) == Some(&b'u') && bytes.get(cursor + 1) == Some(&b'{') {
            cursor += 2;
            while cursor < bytes.len() && bytes[cursor] != b'}' {
                cursor += 1;
            }
        }
        cursor += 1;
    } else {
        let width = std::str::from_utf8(&bytes[cursor..])
            .ok()?
            .chars()
            .next()?
            .len_utf8();
        cursor += width;
    }
    (bytes.get(cursor) == Some(&b'\'')).then_some(cursor + 1)
}

/// 1-indexed line holding `offset`.
pub fn line_at(source: &str, offset: usize) -> usize {
    source.as_bytes()[..offset.min(source.len())]
        .iter()
        .filter(|&&byte| byte == b'\n')
        .count()
        + 1
}
