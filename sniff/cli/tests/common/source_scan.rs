//! Byte-preserving removal of comments and literals for structural test gates.

pub fn sanitize(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut output = bytes.to_vec();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index..].starts_with(b"//") {
            index = blank_line(bytes, &mut output, index);
        } else if bytes[index..].starts_with(b"/*") {
            index = blank_block_comment(bytes, &mut output, index);
        } else if bytes[index] == b'"' {
            index = blank_quoted(bytes, &mut output, index, b'"');
        } else if bytes[index] == b'\'' && looks_like_char_literal(bytes, index) {
            index = blank_quoted(bytes, &mut output, index, b'\'');
        } else if let Some((hashes, quote)) = raw_string_start(bytes, index) {
            index = blank_raw(bytes, &mut output, index, hashes, quote);
        } else {
            index += 1;
        }
    }
    String::from_utf8(output).expect("blanking preserves UTF-8")
}

pub fn is_ident(byte: u8) -> bool {
    byte == b'_' || byte.is_ascii_alphanumeric()
}

pub fn line_at(source: &str, offset: usize) -> usize {
    source.as_bytes()[..offset]
        .iter()
        .filter(|byte| **byte == b'\n')
        .count()
        + 1
}

fn blank(output: &mut [u8], bytes: &[u8], index: usize) {
    if bytes[index] != b'\n' && bytes[index] != b'\r' {
        output[index] = b' ';
    }
}

fn blank_line(bytes: &[u8], output: &mut [u8], mut index: usize) -> usize {
    while index < bytes.len() && bytes[index] != b'\n' {
        blank(output, bytes, index);
        index += 1;
    }
    index
}

fn blank_block_comment(bytes: &[u8], output: &mut [u8], mut index: usize) -> usize {
    let mut depth = 0usize;
    while index < bytes.len() {
        if bytes[index..].starts_with(b"/*") {
            depth += 1;
            blank(output, bytes, index);
            blank(output, bytes, index + 1);
            index += 2;
        } else if bytes[index..].starts_with(b"*/") {
            blank(output, bytes, index);
            blank(output, bytes, index + 1);
            index += 2;
            depth -= 1;
            if depth == 0 {
                return index;
            }
        } else {
            blank(output, bytes, index);
            index += 1;
        }
    }
    index
}

fn blank_quoted(bytes: &[u8], output: &mut [u8], mut index: usize, delimiter: u8) -> usize {
    blank(output, bytes, index);
    index += 1;
    let mut escaped = false;
    while index < bytes.len() {
        let byte = bytes[index];
        blank(output, bytes, index);
        index += 1;
        if escaped {
            escaped = false;
        } else if byte == b'\\' {
            escaped = true;
        } else if byte == delimiter {
            break;
        }
    }
    index
}

fn looks_like_char_literal(bytes: &[u8], index: usize) -> bool {
    let mut cursor = index + 1;
    if cursor >= bytes.len() || bytes[cursor] == b'\n' {
        return false;
    }
    if bytes[cursor] == b'\\' {
        cursor += 2;
    } else {
        cursor += 1;
    }
    cursor < bytes.len() && bytes[cursor] == b'\''
}

fn raw_string_start(bytes: &[u8], index: usize) -> Option<(usize, usize)> {
    if bytes.get(index) != Some(&b'r') {
        return None;
    }
    let mut cursor = index + 1;
    while bytes.get(cursor) == Some(&b'#') {
        cursor += 1;
    }
    (bytes.get(cursor) == Some(&b'"')).then_some((cursor - index - 1, cursor))
}

fn blank_raw(bytes: &[u8], output: &mut [u8], start: usize, hashes: usize, quote: usize) -> usize {
    for position in start..=quote {
        blank(output, bytes, position);
    }
    let mut index = quote + 1;
    while index < bytes.len() {
        if bytes[index] == b'"'
            && bytes.get(index + 1..index + 1 + hashes) == Some(&vec![b'#'; hashes][..])
        {
            blank(output, bytes, index);
            index += 1;
            for _ in 0..hashes {
                blank(output, bytes, index);
                index += 1;
            }
            return index;
        }
        blank(output, bytes, index);
        index += 1;
    }
    index
}
