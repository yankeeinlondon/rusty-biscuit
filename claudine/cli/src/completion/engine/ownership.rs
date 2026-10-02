//! Whether the word under the cursor is Claudine's to complete.
//!
//! A composition command reads the arguments after its file with type-aware
//! ownership ([`claudine::composition::own_arguments`]): a word an open
//! provider switch takes is the provider's, not a setter or positional. The
//! completer asks the same question of the same code, with the word under the
//! cursor as the last argument, so it never offers a setter where execution
//! would forward the word to the agent. A flag at the cursor, or the value of
//! a Claudine option, is offered only when the words before it read cleanly:
//! neither can repair an error earlier on the line.
//!
//! Completion never fails and never prompts. Anything ownership cannot decide
//! (an ambiguous word, an unreadable file or `$schema`, a line execution would
//! reject) means no suggestions. Nothing here runs templates, shell, or a
//! provider: it reads the composition file and its literal `$schema`.

use std::ffi::OsString;

use clap::Parser as _;
use claudine::composition::{
    ArgumentOwner, ArgumentsAfterFile, CallerArgument, ResolvedCompositionSource, SchemaParameters,
    authored_schema_parameters, owner_of_last_argument, setter_key,
};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeSource;

use crate::args::{Cli, Commands};
use crate::argv::{OwnedFlags, normalize_for_completion, partition_composition_tail};
use crate::commands::compose::ownership::candidates;
use crate::completion::schema_completion::resolve_prompt_path;
use crate::completion::scopes::ScopeContext;

/// Whether Claudine owns the word at `current_index` of `argv`, and the words
/// before it read without an ownership error.
///
/// `true` for a word ownership does not govern (before the composition file,
/// or in a command that is not a composition command). A Claudine option's
/// value after the file is Claudine's, so it is `true` exactly when the words
/// before that option read cleanly. `false` when the word belongs to the
/// provider or ownership cannot decide.
pub(super) fn cursor_is_claudines(argv: &[String], current_index: usize) -> bool {
    let cursor = argv.get(current_index).map(String::as_str).unwrap_or("");
    let Some((claudine_argv, arguments)) = partition(argv, current_index, Some(cursor)) else {
        return false;
    };
    match arguments.arguments().last() {
        None => true,
        // The cursor is a Claudine option's value. The option and its
        // unfinished value are left out of the check: clap would reject a
        // partial value (`--on-rate-limit a`) and refuse the line.
        Some(CallerArgument::ClaudineOption) => {
            let option_index = current_index
                .checked_sub(1)
                .filter(|&index| {
                    argv.get(index)
                        .is_some_and(|option| OwnedFlags::for_composition().consumes_next(option))
                })
                .unwrap_or(current_index);
            committed_arguments_are_owned(argv, option_index)
        }
        Some(CallerArgument::Token(_)) => {
            last_owner(&claudine_argv, &arguments) == Ok(Some(ArgumentOwner::Claudine))
        }
    }
}

/// Whether the words before `current_index` read without an ownership error,
/// so a flag may be offered at the cursor.
///
/// The partial flag under the cursor is left out: it may become a Claudine
/// option, and judging it as a forwarded switch would refuse every flag
/// after a provider argument.
pub(super) fn committed_arguments_are_owned(argv: &[String], current_index: usize) -> bool {
    partition(argv, current_index, None)
        .is_some_and(|(claudine_argv, arguments)| last_owner(&claudine_argv, &arguments).is_ok())
}

/// The line up to `current_index`, followed by `cursor` when given, split
/// into Claudine's argv and the arguments after the file. `None` when the
/// line cannot be partitioned or crosses an authored `--`.
fn partition(
    argv: &[String],
    current_index: usize,
    cursor: Option<&str>,
) -> Option<(Vec<OsString>, ArgumentsAfterFile)> {
    let mut line: Vec<OsString> = argv.iter().take(current_index).map(OsString::from).collect();
    line.extend(cursor.map(OsString::from));
    let (claudine_argv, arguments) = partition_composition_tail(normalize_for_completion(line)).ok()?;
    arguments.opaque().is_none().then_some((claudine_argv, arguments))
}

/// Ownership cannot decide the line.
#[derive(Debug, PartialEq, Eq)]
struct Undecided;

/// Who owns the last of `arguments` (`None` when there is none).
fn last_owner(
    claudine_argv: &[OsString],
    arguments: &ArgumentsAfterFile,
) -> Result<Option<ArgumentOwner>, Undecided> {
    let tokens: Vec<&str> = arguments
        .arguments()
        .iter()
        .filter_map(|argument| match argument {
            CallerArgument::Token(token) => Some(token.as_str()),
            CallerArgument::ClaudineOption => None,
        })
        .collect();
    // With no forwarded switch every word is Claudine's (or a reserved-name
    // error), whatever the file says, so the file is not read.
    if !tokens.iter().any(|token| token.starts_with('-') && *token != "-") {
        return owner_of_last_argument(arguments, &SchemaParameters::NoSchema, &[]).map_err(|_| Undecided);
    }

    let cli = Cli::try_parse_from(claudine_argv).map_err(|_| Undecided)?;
    let (shared, words) = match cli.command {
        Some(Commands::Compose(args)) => (args.shared, args.args),
        Some(Commands::InlineCompose(args)) => (args.shared, args.args),
        Some(Commands::Sequence(args)) => (args.shared, args.args),
        _ => return Err(Undecided),
    };
    let file = words.iter().find(|word| setter_key(word).is_none()).ok_or(Undecided)?;
    let source = authored_source(file, &ScopeContext::discover()).ok_or(Undecided)?;
    let schema = if tokens.iter().any(|token| setter_key(token).is_some()) {
        authored_schema_parameters(&source, None, None).map_err(|_| Undecided)?
    } else {
        SchemaParameters::NoSchema
    };
    owner_of_last_argument(arguments, &schema, &candidates(&shared, &source)).map_err(|_| Undecided)
}

/// The composition file as authored, resolved the way the setter completers
/// resolve it.
fn authored_source(file: &str, ctx: &ScopeContext) -> Option<ResolvedCompositionSource> {
    let resolved_path = resolve_prompt_path(file, ctx)?;
    let original_text = std::fs::read_to_string(&resolved_path).ok()?;
    let markdown =
        Markdown::from(original_text.clone()).with_source(ComposeSource::infer_from_path(&resolved_path));
    Some(ResolvedCompositionSource {
        original_ref: file.to_string(),
        resolved_path,
        original_text,
        markdown,
    })
}
