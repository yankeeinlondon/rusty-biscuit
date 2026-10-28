//! Typed shell-command templates: the authored shape of a command whose
//! `{{ … }}` placeholders stay expression nodes instead of being interpolated
//! into the command text and tokenized afterwards.
//!
//! A [`CommandTemplate`] is parsed from authored text ([`parse_authored`]),
//! typed at its site's type scope: the owning document's schema narrowed by
//! the guards that enclose the site ([`CommandTemplate::project_types`]),
//! and reduced to a [`TemplateShape`] whose holes keep only their approval
//! type. A shape is what a `template <shape>` policy rule stores, what the
//! whitelist and blacklist match, and what a template approval authorizes.
//! A [`FilledChain`] is a command after every hole has been replaced by one
//! argument ([`fill_template`]); the runtime blacklist checks it before the
//! first segment starts, and [`resolve_direct_target`] finds each program
//! without a shell.
//!
//! Design: `claudine/docs/topics/pre-flight-checks.md` and
//! `darkmatter/docs/inline/preflight-checks.md`.

mod approval;
mod canonical;
pub(crate) mod candidate;
mod fill;
mod grammar;
pub mod matching;
mod target;
mod typing;

#[cfg(test)]
mod tests;

pub use approval::{ApprovalKey, OfferedChoices, ShellPreApprovals, SuggestedRules, TemplateApprovalView};
pub use canonical::TemplateRuleError;
pub use candidate::{CommandSite, DeferredShellRoots, ModeRequirement, ShellCandidate, derived_deferred_keys, value_read_roots};
pub use fill::{
    DocumentShellValues, HoleError, HoleEvaluator, LateValueError, LateValues, LookupEvaluator, RejectReason, SiteValues,
    fill_template, resolve_late_values,
};
pub use grammar::parse_authored;
pub use matching::rule_covers;
pub use target::{TemplateTargetError, resolve_direct_target};
pub use typing::{
    ApprovalType, HoleRefinement, HoleType, HoleValidator, StringLikeType, TypeProvenance, project_hole,
};

use super::tokenize::{ShellToken, apply_redirection};
use crate::markdown::compose::expression::{GuardOrigin, TypeScope};
use super::types::{ChainOperator, RedirectionConfig};
use std::ops::Range;

/// An authored shell command with its placeholders kept as expression nodes.
#[derive(Debug, Clone, PartialEq)]
pub struct CommandTemplate {
    /// The command exactly as authored, `{{ … }}` spans included.
    pub authored: String,
    /// The chain segments, in order.
    pub segments: Vec<TemplateSegment>,
}

/// One command of a chain: a literal program word, its arguments, and its
/// curated redirections.
#[derive(Debug, Clone, PartialEq)]
pub struct TemplateSegment {
    /// The operator before this segment; [`ChainOperator::None`] for the first.
    pub operator: ChainOperator,
    /// The program word. Always literal: a placeholder here is a grammar error.
    pub program: String,
    pub args: Vec<TemplateArg>,
    /// Redirections in authored order. They are operations, not arguments.
    pub redirects: Vec<RedirectOp>,
}

/// One argument: an ordered list of literal runs and placeholders that
/// together form exactly one process argument.
///
/// Adjacent literal runs are merged and empty runs dropped, so two spellings
/// of the same argument have equal parts. An empty argument (`""`) is one
/// empty [`ArgPart::Literal`], never zero parts.
#[derive(Debug, Clone, PartialEq)]
pub struct TemplateArg {
    pub parts: Vec<ArgPart>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ArgPart {
    Literal(String),
    Hole(Placeholder),
}

/// One `{{ … }}` span inside an argument.
#[derive(Debug, Clone, PartialEq)]
pub struct Placeholder {
    /// The expression text inside `{{ }}`, trimmed.
    pub expression: String,
    /// The byte span of the whole `{{ … }}` in [`CommandTemplate::authored`].
    pub span: Range<usize>,
    /// The hole's type. [`parse_authored`] projects it without a schema;
    /// [`CommandTemplate::project_types`] replaces it with the type at the
    /// site's scope.
    pub typing: HoleType,
    /// When the hole's value exists. [`parse_authored`] assumes
    /// [`HoleTiming::ApprovalTime`]; discovery marks a read of a root the host
    /// declared late (see [`DeferredShellRoots`]) and a guard-proven value it
    /// found missing.
    pub timing: HoleTiming,
}

/// When a placeholder's value can be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HoleTiming {
    /// The value exists when the command is approved, so preflight checks it
    /// is present.
    ApprovalTime,
    /// The value is produced by the run. `roots` are the host-declared late
    /// roots the expression reads, directly or through a derived frontmatter
    /// value; nothing about the value is checked at approval.
    Deferred { roots: Vec<String> },
    /// A guard enclosing this site proves the value present, but it is
    /// missing at approval or reads a late root (`roots`, empty otherwise).
    /// It is read and validated only when the guarded command is reached:
    /// never at approval, and never by an unconditional barrier
    /// ([`resolve_late_values`]), so a branch that does not run cannot fail
    /// on it.
    Guarded { guard: GuardOrigin, roots: Vec<String> },
    /// The hole reads a local of an enclosing `::map` / `::filter` body. Its
    /// value belongs to one repeated copy, so it is read from that copy's
    /// frame when the command is reached: never at approval, and never by an
    /// unconditional barrier. Only a template can be approved for it.
    Local,
}

impl HoleTiming {
    /// Whether the value is not read at approval, so the command can be
    /// approved only as a template.
    pub fn is_late(&self) -> bool {
        !matches!(self, Self::ApprovalTime)
    }
}

/// One of the four curated redirections, in the order it was written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RedirectOp {
    /// `> /dev/null`
    StdoutNull,
    /// `2> /dev/null`
    StderrNull,
    /// `2>&1`
    StderrToStdout,
    /// `>&2`
    StdoutToStderr,
}

impl RedirectOp {
    /// The canonical spelling, as written in a policy rule.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::StdoutNull => "> /dev/null",
            Self::StderrNull => "2> /dev/null",
            Self::StderrToStdout => "2>&1",
            Self::StdoutToStderr => ">&2",
        }
    }

    fn token(self) -> ShellToken {
        match self {
            Self::StdoutNull => ShellToken::RedirectStdoutNull,
            Self::StderrNull => ShellToken::RedirectStderrNull,
            Self::StderrToStdout => ShellToken::RedirectStderrToStdout,
            Self::StdoutToStderr => ShellToken::RedirectStdoutToStderr,
        }
    }
}

/// The stream configuration `ops` produce, with the same left-to-right dup
/// semantics the resolved-byte tokenizer applies.
pub fn redirection_config(ops: &[RedirectOp]) -> RedirectionConfig {
    let mut config = RedirectionConfig::default();
    for op in ops {
        apply_redirection(&mut config, &op.token());
    }
    config
}

/// Why authored text cannot be a template. A command with one of these can
/// still be approved and run as resolved bytes where that mode is eligible.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TemplateGrammarError {
    #[error("a placeholder cannot be the program word of a command")]
    ProgramWordHole,
    #[error("a placeholder cannot be a redirection target")]
    RedirectHole,
    #[error("shell pipes (|) are not supported")]
    Pipe,
    #[error("command chaining with ; is not supported")]
    Semicolon,
    #[error("background execution (&) is not supported")]
    Background,
    #[error("input redirection (<) is not supported")]
    InputRedirect,
    #[error("only > /dev/null, 2> /dev/null, 2>&1, and >&2 redirections are supported")]
    ArbitraryRedirect,
    #[error("command substitution $( … ) is not supported")]
    CommandSubstitution,
    #[error("unterminated quote or escape")]
    Unterminated,
    #[error("a literal argument holds a control character other than newline, carriage return, or tab")]
    ControlCharacter,
    #[error("interpolation literals ({{{{{{ … }}}}}}) and literal tokens are not supported")]
    InterpolationLiteral,
    #[error("empty command, or a chain operator without a command on one side")]
    Empty,
    #[error("a conditional $( … ? … : … ) command is not supported")]
    Conditional,
}

/// A command shape: a template with each hole reduced to its approval type.
///
/// Equality is structural, so two spellings that differ only in whitespace or
/// quoting are equal, and a shape parsed back from its
/// [`canonical_text`](Self::canonical_text) equals the original.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TemplateShape {
    pub segments: Vec<ShapeSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShapeSegment {
    pub operator: ChainOperator,
    pub program: String,
    pub args: Vec<ShapeArg>,
    pub redirects: Vec<RedirectOp>,
}

/// One argument of a shape, normalized like [`TemplateArg`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShapeArg {
    pub parts: Vec<ShapePart>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ShapePart {
    Literal(String),
    Hole(ApprovalType),
}

impl CommandTemplate {
    /// Types every placeholder at the site whose types are `scope`; see
    /// [`project_hole`].
    pub fn project_types(&mut self, scope: &TypeScope<'_>) {
        for placeholder in self.placeholders_mut() {
            placeholder.typing = project_hole(&placeholder.expression, scope);
        }
    }

    /// Every placeholder, in authored order, for retyping or retiming.
    pub fn placeholders_mut(&mut self) -> impl Iterator<Item = &mut Placeholder> {
        self.segments
            .iter_mut()
            .flat_map(|segment| &mut segment.args)
            .flat_map(|arg| &mut arg.parts)
            .filter_map(|part| match part {
                ArgPart::Hole(placeholder) => Some(placeholder),
                ArgPart::Literal(_) => None,
            })
    }

    /// Every placeholder, in authored order.
    pub fn placeholders(&self) -> impl Iterator<Item = &Placeholder> {
        self.segments
            .iter()
            .flat_map(|segment| &segment.args)
            .flat_map(|arg| &arg.parts)
            .filter_map(|part| match part {
                ArgPart::Hole(placeholder) => Some(placeholder),
                ArgPart::Literal(_) => None,
            })
    }
}

impl TemplateShape {
    /// The shape of `template`, keeping each hole's approval type.
    pub fn from_template(template: &CommandTemplate) -> Self {
        let segments = template
            .segments
            .iter()
            .map(|segment| ShapeSegment {
                operator: segment.operator,
                program: segment.program.clone(),
                args: segment
                    .args
                    .iter()
                    .map(|arg| ShapeArg {
                        parts: arg
                            .parts
                            .iter()
                            .map(|part| match part {
                                ArgPart::Literal(text) => ShapePart::Literal(text.clone()),
                                ArgPart::Hole(placeholder) => ShapePart::Hole(placeholder.typing.approval.clone()),
                            })
                            .collect(),
                    })
                    .collect(),
                redirects: segment.redirects.clone(),
            })
            .collect();
        Self { segments }
    }

    /// Whether any argument holds a hole.
    pub fn has_holes(&self) -> bool {
        self.segments
            .iter()
            .flat_map(|segment| &segment.args)
            .flat_map(|arg| &arg.parts)
            .any(|part| matches!(part, ShapePart::Hole(_)))
    }
}

/// A command whose holes have all been filled: one string per argument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilledChain {
    pub segments: Vec<FilledSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilledSegment {
    pub operator: ChainOperator,
    pub program: String,
    pub args: Vec<String>,
    /// The lowered stream configuration. A resolved-byte pipeline carries only
    /// this, not the authored redirection order, so both modes share one form.
    pub redirection: RedirectionConfig,
}

impl FilledChain {
    /// The filled form of a resolved-byte pipeline, so the runtime blacklist
    /// checks a command without authored holes the same way.
    pub fn from_pipeline(pipeline: &super::types::ShellPipeline) -> Self {
        Self {
            segments: pipeline
                .actions
                .iter()
                .map(|action| FilledSegment {
                    operator: action.operator,
                    program: action.command.executable.clone(),
                    args: action.command.args.clone(),
                    redirection: action.command.redirection.clone(),
                })
                .collect(),
        }
    }
}

/// Merges adjacent literal runs and drops empty ones, keeping one empty
/// literal when nothing else remains.
fn normalize_parts<T>(parts: Vec<T>, literal: impl Fn(&T) -> Option<&str>, make: impl Fn(String) -> T) -> Vec<T> {
    let mut out: Vec<T> = Vec::with_capacity(parts.len());
    for part in parts {
        match literal(&part) {
            Some("") => {}
            Some(text) => match out.last().and_then(&literal) {
                Some(previous) => {
                    let merged = format!("{previous}{text}");
                    *out.last_mut().expect("checked above") = make(merged);
                }
                None => out.push(part),
            },
            None => out.push(part),
        }
    }
    if out.is_empty() {
        out.push(make(String::new()));
    }
    out
}

impl TemplateArg {
    pub(crate) fn normalized(parts: Vec<ArgPart>) -> Self {
        Self {
            parts: normalize_parts(
                parts,
                |part| match part {
                    ArgPart::Literal(text) => Some(text.as_str()),
                    ArgPart::Hole(_) => None,
                },
                ArgPart::Literal,
            ),
        }
    }
}

impl ShapeArg {
    pub(crate) fn normalized(parts: Vec<ShapePart>) -> Self {
        Self {
            parts: normalize_parts(
                parts,
                |part| match part {
                    ShapePart::Literal(text) => Some(text.as_str()),
                    ShapePart::Hole(_) => None,
                },
                ShapePart::Literal,
            ),
        }
    }

    /// The argument's text when it holds no hole.
    pub fn as_literal(&self) -> Option<&str> {
        match self.parts.as_slice() {
            [ShapePart::Literal(text)] => Some(text),
            _ => None,
        }
    }
}
