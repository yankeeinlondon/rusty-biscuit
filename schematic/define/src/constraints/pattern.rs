//! Which `pattern` sources a generated client can enforce.
//!
//! JSON Schema patterns are ECMAScript regular expressions; the runtime
//! matches with Rust's `regex` crate. A pattern is enforced only when every
//! construct in it means the same thing in both. The admitted and refused
//! constructs are tabulated in `schematic/docs/value-constraints.md`, which a
//! test reads row by row.

use regex_syntax::ast::{
    self, Ast, AssertionKind, ClassPerl, ClassPerlKind, ClassSet, ClassSetItem, ErrorKind, GroupKind,
    HexLiteralKind, LiteralKind, SpecialLiteralKind,
};

/// The outcome of checking a pattern source for enforcement.
///
/// ## Examples
///
/// ```
/// use schematic_define::constraints::{PatternAdmission, admit_pattern};
///
/// assert_eq!(admit_pattern(r"^\d{3}$"), PatternAdmission::Enforced { translated: "^[0-9]{3}$".into() });
/// assert!(matches!(admit_pattern(r"^\s+$"), PatternAdmission::Unsupported { .. }));
/// assert!(matches!(admit_pattern("(a"), PatternAdmission::Invalid { .. }));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PatternAdmission {
    /// The pattern means the same in ECMAScript and Rust once `translated`:
    /// `\d`, `\w`, and their negations are spelled as the ASCII classes
    /// ECMAScript gives them, since Rust's are Unicode-aware. The runtime
    /// compiles `translated`, never the source.
    Enforced {
        /// The source to compile with the runtime's `Pattern`.
        translated: String,
    },
    /// A valid ECMAScript pattern using a construct that is unsupported or
    /// means something else in Rust. It can be preserved and exported but
    /// not enforced.
    Unsupported {
        /// The construct and why it is not enforced.
        reason: String,
    },
    /// Not a valid regular expression in either dialect.
    Invalid {
        /// The syntax problem.
        reason: String,
    },
}

/// Classifies `source` for enforcement. The empty pattern is enforced and
/// matches everything.
pub fn admit_pattern(source: &str) -> PatternAdmission {
    let unsupported = |reason: &str| PatternAdmission::Unsupported { reason: reason.to_owned() };
    if has_nested_bracket(source) {
        // Rust nests a class at `[` inside a class; ECMAScript reads a literal.
        return unsupported("`[` inside a character class nests a class in Rust but is a literal in ECMAScript");
    }
    let parsed = match ast::parse::Parser::new().parse(source) {
        Ok(parsed) => parsed,
        Err(error) => {
            let reason = error.kind().to_string();
            return if definitely_invalid(error.kind()) {
                PatternAdmission::Invalid { reason }
            } else {
                PatternAdmission::Unsupported { reason: format!("Rust's regex syntax does not accept it ({reason})") }
            };
        }
    };
    let mut edits = Vec::new();
    if let Err(reason) = walk(&parsed, source, &mut edits) { return unsupported(reason); }
    edits.sort_by_key(|edit| edit.0);
    let mut translated = String::with_capacity(source.len());
    let mut at = 0;
    for (start, end, text) in edits {
        translated.push_str(&source[at..start]);
        translated.push_str(text);
        at = end;
    }
    translated.push_str(&source[at..]);
    // The default size limits can still refuse a pattern the syntax accepts.
    match regex::Regex::new(&translated) {
        Ok(_) => PatternAdmission::Enforced { translated },
        Err(error) => unsupported(&format!("it does not compile within the runtime's limits ({error})")),
    }
}

/// Parse errors that ECMAScript also reports; any other error is a
/// construct Rust lacks or reads differently.
fn definitely_invalid(kind: &ErrorKind) -> bool {
    matches!(kind, ErrorKind::GroupUnclosed | ErrorKind::GroupUnopened | ErrorKind::RepetitionMissing
        | ErrorKind::RepetitionCountInvalid | ErrorKind::EscapeUnexpectedEof | ErrorKind::ClassRangeInvalid
        | ErrorKind::GroupNameDuplicate { .. })
}

/// Whether an unescaped `[` occurs inside a bracketed class, read with
/// ECMAScript's rules.
fn has_nested_bracket(source: &str) -> bool {
    let mut in_class = false;
    let mut chars = source.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => { chars.next(); }
            '[' if in_class => return true,
            '[' => in_class = true,
            ']' => in_class = false,
            _ => {}
        }
    }
    false
}

type Edit = (usize, usize, &'static str);

fn walk(node: &Ast, source: &str, edits: &mut Vec<Edit>) -> Result<(), &'static str> {
    match node {
        Ast::Empty(_) => Ok(()),
        Ast::Flags(_) => Err("inline flags have no ECMAScript equivalent"),
        Ast::Literal(literal) => check_literal(literal),
        Ast::Dot(_) => Err("`.` excludes different line terminators in ECMAScript and Rust"),
        Ast::Assertion(assertion) => match assertion.kind {
            AssertionKind::StartLine | AssertionKind::EndLine => Ok(()),
            AssertionKind::WordBoundary | AssertionKind::NotWordBoundary => {
                Err("`\\b` and `\\B` use Unicode word characters in Rust and ASCII ones in ECMAScript")
            }
            _ => Err("the assertion is Rust syntax that ECMAScript reads differently"),
        },
        Ast::ClassUnicode(_) => Err("Unicode property classes (`\\p{..}`) are not enforced"),
        Ast::ClassPerl(class) => {
            edits.push((class.span.start.offset, class.span.end.offset, perl_outside(class)?));
            Ok(())
        }
        Ast::ClassBracketed(class) => {
            let after = &source[class.span.start.offset + 1..];
            let after = if class.negated { &after[1..] } else { after };
            if after.starts_with(']') {
                return Err("a class starting with `]` is empty in ECMAScript but holds `]` in Rust");
            }
            check_class_set(&class.kind, edits)
        }
        Ast::Repetition(repetition) => {
            let op = &source[repetition.op.span.start.offset..repetition.op.span.end.offset];
            if op.starts_with("{,") || op.contains(' ') {
                return Err("this counted repetition is a literal in ECMAScript");
            }
            walk(&repetition.ast, source, edits)
        }
        Ast::Group(group) => {
            match &group.kind {
                GroupKind::CaptureIndex(_) => {}
                GroupKind::CaptureName { starts_with_p: false, .. } => {}
                GroupKind::CaptureName { starts_with_p: true, .. } => return Err("`(?P<name>..)` is not ECMAScript syntax"),
                GroupKind::NonCapturing(flags) if flags.items.is_empty() => {}
                GroupKind::NonCapturing(_) => return Err("inline flags have no ECMAScript equivalent"),
            }
            walk(&group.ast, source, edits)
        }
        Ast::Alternation(alternation) => alternation.asts.iter().try_for_each(|ast| walk(ast, source, edits)),
        Ast::Concat(concat) => concat.asts.iter().try_for_each(|ast| walk(ast, source, edits)),
    }
}

fn check_literal(literal: &ast::Literal) -> Result<(), &'static str> {
    match &literal.kind {
        LiteralKind::Verbatim | LiteralKind::Meta | LiteralKind::Superfluous => Ok(()),
        LiteralKind::HexFixed(HexLiteralKind::X | HexLiteralKind::UnicodeShort) => Ok(()),
        LiteralKind::HexFixed(HexLiteralKind::UnicodeLong) | LiteralKind::HexBrace(_) => {
            Err("`\\U` and braced hex escapes are not ECMAScript syntax without the `u` flag")
        }
        LiteralKind::Octal => Err("octal escapes are not enforced"),
        LiteralKind::Special(SpecialLiteralKind::Bell | SpecialLiteralKind::Space) => {
            Err("the escape is Rust syntax that ECMAScript reads differently")
        }
        LiteralKind::Special(_) => Ok(()),
    }
}

/// ECMAScript's ASCII meaning of a Perl class outside brackets.
fn perl_outside(class: &ClassPerl) -> Result<&'static str, &'static str> {
    Ok(match (&class.kind, class.negated) {
        (ClassPerlKind::Digit, false) => "[0-9]",
        (ClassPerlKind::Digit, true) => "[^0-9]",
        (ClassPerlKind::Word, false) => "[0-9A-Za-z_]",
        (ClassPerlKind::Word, true) => "[^0-9A-Za-z_]",
        (ClassPerlKind::Space, _) => return Err("`\\s` matches different whitespace in ECMAScript and Rust"),
    })
}

fn check_class_set(set: &ClassSet, edits: &mut Vec<Edit>) -> Result<(), &'static str> {
    match set {
        ClassSet::BinaryOp(_) => Err("class set operations (`&&`, `--`, `~~`) are literals in ECMAScript"),
        ClassSet::Item(item) => check_class_item(item, edits),
    }
}

fn check_class_item(item: &ClassSetItem, edits: &mut Vec<Edit>) -> Result<(), &'static str> {
    match item {
        ClassSetItem::Empty(_) => Ok(()),
        ClassSetItem::Literal(literal) => check_literal(literal),
        ClassSetItem::Range(range) => { check_literal(&range.start)?; check_literal(&range.end) }
        ClassSetItem::Ascii(_) => Err("POSIX classes (`[[:alpha:]]`) are not ECMAScript syntax"),
        ClassSetItem::Unicode(_) => Err("Unicode property classes (`\\p{..}`) are not enforced"),
        ClassSetItem::Perl(class) => {
            // Inside brackets a negated class nests, which Rust allows.
            let text = match (&class.kind, class.negated) {
                (ClassPerlKind::Digit, false) => "0-9",
                (ClassPerlKind::Digit, true) => "[^0-9]",
                (ClassPerlKind::Word, false) => "0-9A-Za-z_",
                (ClassPerlKind::Word, true) => "[^0-9A-Za-z_]",
                (ClassPerlKind::Space, _) => return Err("`\\s` matches different whitespace in ECMAScript and Rust"),
            };
            edits.push((class.span.start.offset, class.span.end.offset, text));
            Ok(())
        }
        ClassSetItem::Bracketed(_) => Err("`[` inside a character class nests a class in Rust but is a literal in ECMAScript"),
        ClassSetItem::Union(union) => union.items.iter().try_for_each(|item| check_class_item(item, edits)),
    }
}
