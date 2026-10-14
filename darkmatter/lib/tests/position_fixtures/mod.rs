//! Complete Markdown documents whose frontmatter validation problems have
//! known source starts.
//!
//! Std-only, so the darkmatter CLI and DMLS suites can both include it with
//! `#[path]` and check the same bytes against the same expected starts. Each
//! document declares its schema inline, after the properties it validates, so
//! the authored lines above `$schema` match the specification's examples
//! exactly. Lines count the whole file (the opening `---` is line 1); columns
//! are 1-based UTF-8 byte columns.

#![allow(dead_code, reason = "each including suite uses a different subset")]

/// One validation problem and the source start it must report.
#[derive(Debug, Clone, Copy)]
pub struct ExpectedStart {
    /// The problem's JSON Pointer.
    pub path: &'static str,
    /// The CLI's JSON `kind`: `missing`, `type`, or `invalid`.
    pub kind: &'static str,
    /// The unknown key the problem names, for unknown-key problems.
    pub offending_property: Option<&'static str>,
    pub line: u32,
    pub column: u32,
}

/// A complete Markdown document and every problem whose start it pins.
#[derive(Debug, Clone, Copy)]
pub struct PositionCase {
    pub name: &'static str,
    pub document: &'static str,
    pub expected: &'static [ExpectedStart],
}

/// The specification's acceptance fixture: an unknown nested key `sya`.
pub const ACCEPTANCE: PositionCase = PositionCase {
    name: "acceptance",
    document: "---\n\
start:\n\
\x20 - then:\n\
\x20     - say: hello\n\
\x20     - sya: goodbye\n\
$schema:\n\
\x20 start: \"{ then: { say: string }[] }[]\"\n\
---\n\
Body\n",
    expected: &[ExpectedStart {
        path: "/start/0/then/1",
        kind: "invalid",
        offending_property: Some("sya"),
        line: 5,
        column: 9,
    }],
};

/// A wrong-typed block-list item (dash excluded) and a second object item
/// missing a required field (its mapping start).
pub const BLOCK_LIST: PositionCase = PositionCase {
    name: "block_list",
    document: "---\n\
tags:\n\
\x20 - 1\n\
\x20 - oops\n\
items:\n\
\x20 - name: a\n\
\x20   id: 1\n\
\x20 - name: b\n\
$schema:\n\
\x20 tags: number[]\n\
\x20 items: \"{ name: string, id: number(required) }[]\"\n\
---\n",
    expected: &[
        ExpectedStart {
            path: "/tags/1",
            kind: "type",
            offending_property: None,
            line: 4,
            column: 5,
        },
        ExpectedStart {
            path: "/items/1",
            kind: "missing",
            offending_property: None,
            line: 8,
            column: 5,
        },
    ],
};

/// Same-line errors in flow collections: an unknown key, a wrong value, and a
/// wrong sequence item, so a line-only position cannot pass.
pub const FLOW: PositionCase = PositionCase {
    name: "flow",
    document: "---\n\
m: {a: 1, b: x}\n\
n: {a: x}\n\
s: [1, two, 3]\n\
$schema:\n\
\x20 m: \"{ a: number }\"\n\
\x20 n: \"{ a: number }\"\n\
\x20 s: number[]\n\
---\n",
    expected: &[
        ExpectedStart {
            path: "/m",
            kind: "invalid",
            offending_property: Some("b"),
            line: 2,
            column: 11,
        },
        ExpectedStart {
            path: "/n/a",
            kind: "type",
            offending_property: None,
            line: 3,
            column: 8,
        },
        ExpectedStart {
            path: "/s/1",
            kind: "type",
            offending_property: None,
            line: 4,
            column: 8,
        },
    ],
};

/// A same-line prefix holding a supplementary-plane character (four UTF-8
/// bytes, two UTF-16 units) before an unknown key and a wrong value.
pub const NON_ASCII_PREFIX: PositionCase = PositionCase {
    name: "non_ascii_prefix",
    document: "---\n\
m: {a: \"\u{1F600}\", zz: 1}\n\
n: {a: \"\u{1F600}\", b: x}\n\
$schema:\n\
\x20 m: \"{ a: string }\"\n\
\x20 n: \"{ a: string, b: number }\"\n\
---\n",
    expected: &[
        ExpectedStart {
            path: "/m",
            kind: "invalid",
            offending_property: Some("zz"),
            line: 2,
            column: 16,
        },
        ExpectedStart {
            path: "/n/b",
            kind: "type",
            offending_property: None,
            line: 3,
            column: 19,
        },
    ],
};

/// A root missing field, after leading comments and a blank line, reports
/// the root mapping start.
pub const ROOT_MISSING: PositionCase = PositionCase {
    name: "root_missing",
    document: "---\n\
# comment\n\
\n\
other: x\n\
$schema:\n\
\x20 title: string(required)\n\
---\n",
    expected: &[ExpectedStart {
        path: "",
        kind: "missing",
        offending_property: None,
        line: 4,
        column: 1,
    }],
};

/// Every case, for suites that walk them all.
pub const CASES: &[PositionCase] = &[ACCEPTANCE, BLOCK_LIST, FLOW, NON_ASCII_PREFIX, ROOT_MISSING];

/// The byte offset of a 1-based line and 1-based UTF-8 byte column in
/// `text`. A line ends at `\n`, so the same call addresses LF and CRLF text.
pub fn byte_offset(text: &str, line: u32, column: u32) -> usize {
    let mut start = 0;
    for _ in 1..line {
        start += text[start..]
            .find('\n')
            .unwrap_or_else(|| panic!("text has fewer than {line} lines"))
            + 1;
    }
    start + column as usize - 1
}
