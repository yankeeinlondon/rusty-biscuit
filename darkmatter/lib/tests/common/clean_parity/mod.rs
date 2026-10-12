//! Shared `clean_document` parity cases: one raw input, the text every
//! consumer must emit for it, and the outcome.
//!
//! The library suite (`lib/tests/l1/clean_parity.rs`) checks each case
//! against `clean_document`; the `md clean` suite and the DMLS
//! `textDocument/formatting` suite include this file by `#[path]` and check
//! the same cases through their normal invocation paths, so the three agree
//! byte for byte. Inputs are literals, not files, so line endings and BOMs
//! survive any checkout's end-of-line normalization.

/// What a consumer does with a case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParityOutcome {
    /// Emit `expected`, which differs from the input.
    Changed,
    /// Emit nothing new; `expected` is the input.
    Unchanged,
    /// Refuse: emit the input, apply no repair, report the fallback.
    Declined,
    /// Darkmatter cannot load the frontmatter that would be emitted.
    Invalid,
}

#[derive(Debug, Clone, Copy)]
pub struct ParityCase {
    pub name: &'static str,
    pub input: &'static str,
    /// The emitted text; the input for every outcome except `Changed`.
    pub expected: &'static str,
    pub outcome: ParityOutcome,
}

pub const CASES: &[ParityCase] = &[
    ParityCase {
        name: "repairable-frontmatter",
        input: "---\ntitle: @daily-report\n---\n\n```\ncode\n```\n",
        expected: "---\ntitle: \"@daily-report\"\n---\n\n```text\ncode\n```\n",
        outcome: ParityOutcome::Changed,
    },
    ParityCase {
        name: "repair-disabled",
        input: "---\ntitle: Doc   \nstyle:\n  markdown:\n    frontmatter: false\n---\n\n```\ncode\n```\n",
        expected: "---\ntitle: Doc   \nstyle:\n  markdown:\n    frontmatter: false\n---\n\n```text\ncode\n```\n",
        outcome: ParityOutcome::Changed,
    },
    ParityCase {
        name: "repair-disabled-invalid-yaml",
        input: "---\ntitle: @daily-report\nstyle:\n  markdown:\n    frontmatter: false\n---\n\n```\ncode\n```\n",
        expected: "---\ntitle: @daily-report\nstyle:\n  markdown:\n    frontmatter: false\n---\n\n```\ncode\n```\n",
        outcome: ParityOutcome::Invalid,
    },
    ParityCase {
        name: "unrepairable-frontmatter",
        input: "---\ntitle: a\ntitle: b\n---\n\n```\ncode\n```\n",
        expected: "---\ntitle: a\ntitle: b\n---\n\n```\ncode\n```\n",
        outcome: ParityOutcome::Invalid,
    },
    ParityCase {
        // Strict YAML rejects the bare `{{ … }}` value; Darkmatter loads it.
        name: "bare-interpolation-frontmatter",
        input: "---\nreview: {{ ctx.area }}/reviews/{{ ctx.today }}/review.md\n---\n\n```\ncode\n```\n",
        expected: "---\nreview: {{ ctx.area }}/reviews/{{ ctx.today }}/review.md\n---\n\n```text\ncode\n```\n",
        outcome: ParityOutcome::Changed,
    },
    ParityCase {
        name: "empty-frontmatter",
        input: "---\n---\n\n```\ncode\n```\n",
        expected: "---\n---\n\n```text\ncode\n```\n",
        outcome: ParityOutcome::Changed,
    },
    ParityCase {
        name: "bom-without-frontmatter",
        input: "\u{feff}# Title\n\n```\ncode\n```\n",
        expected: "\u{feff}# Title\n\n```text\ncode\n```\n",
        outcome: ParityOutcome::Changed,
    },
    ParityCase {
        name: "bom-before-frontmatter",
        input: "\u{feff}---\ntitle: Doc\n---\n\nBody.\n",
        expected: "---\ntitle: Doc\n---\n\nBody.\n",
        outcome: ParityOutcome::Changed,
    },
    ParityCase {
        name: "crlf",
        input: "# Title\r\n\r\n- a\r\n- b\r\n\r\n```\r\ncode\r\n```\r\n",
        expected: "# Title\r\n\r\n- a\r\n- b\r\n\r\n```text\r\ncode\r\n```\r\n",
        outcome: ParityOutcome::Changed,
    },
    // The YAML between the delimiters gets biscuit-file's reported
    // `yaml.line-ending` repair (CRLF to LF); the delimiter lines and the
    // body keep their CRLF terminators.
    ParityCase {
        name: "crlf-frontmatter",
        input: "---\r\ntitle: Doc\r\n---\r\n\r\n# Title\r\n",
        expected: "---\r\ntitle: Doc\n---\r\n\r\n# Title\r\n",
        outcome: ParityOutcome::Changed,
    },
    ParityCase {
        name: "mixed-line-endings",
        input: "one\r\ntwo\n\n- a\r\n- b\n\n```\ncode\r\n```\r\n",
        expected: "one\r\ntwo\n\n- a\r\n- b\n\n```text\ncode\r\n```\r\n",
        outcome: ParityOutcome::Changed,
    },
    ParityCase {
        name: "missing-final-newline",
        input: "# Title\n\n```\ncode\n```",
        expected: "# Title\n\n```text\ncode\n```",
        outcome: ParityOutcome::Changed,
    },
    ParityCase {
        name: "reference-definitions",
        input: "See [a][ref].\n\n[ref]: /one \"One\"\n[unused]: /two\n",
        expected: "See [a][ref].\n\n[ref]: /one \"One\"\n[unused]: /two\n",
        outcome: ParityOutcome::Unchanged,
    },
    ParityCase {
        name: "unpaired-directive-after-repairable-frontmatter",
        input: "---\ntitle: @daily-report\n---\n\n::block when=\"a\"\n\n```\ncode\n```\n",
        expected: "---\ntitle: @daily-report\n---\n\n::block when=\"a\"\n\n```\ncode\n```\n",
        outcome: ParityOutcome::Declined,
    },
];
