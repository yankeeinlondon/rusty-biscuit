//! Whether the word under the cursor is Claudine's to complete.
//!
//! A composition command reads the arguments after its file with type-aware
//! ownership ([`claudine::composition::own_arguments`]): a word an open
//! provider switch takes is the provider's, not a setter or positional. The
//! completer asks the same question of the same code, with the word under the
//! cursor as the last argument, so it never offers a setter where execution
//! would forward the word to the agent.
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
use crate::argv::{normalize_for_completion, partition_composition_tail};
use crate::commands::compose::ownership::candidates;
use crate::completion::schema_completion::resolve_prompt_path;
use crate::completion::scopes::ScopeContext;

/// Whether Claudine owns the word at `current_index` of `argv`.
///
/// `true` for anything ownership does not govern: a word before the
/// composition file, a Claudine option or its value, or a command that is not
/// a composition command. `false` when the word belongs to the provider or
/// ownership cannot decide.
pub(super) fn cursor_is_claudines(argv: &[String], current_index: usize) -> bool {
    let mut line: Vec<OsString> = argv.iter().take(current_index).map(OsString::from).collect();
    line.push(OsString::from(argv.get(current_index).map(String::as_str).unwrap_or("")));
    let Ok((claudine_argv, arguments)) = partition_composition_tail(normalize_for_completion(line)) else {
        return false;
    };
    if arguments.opaque().is_some() {
        return false;
    }
    match arguments.arguments().last() {
        None | Some(CallerArgument::ClaudineOption) => true,
        Some(CallerArgument::Token(_)) => {
            owner(&claudine_argv, &arguments) == Some(ArgumentOwner::Claudine)
        }
    }
}

/// Who owns the last of `arguments`; `None` when ownership cannot decide.
fn owner(claudine_argv: &[OsString], arguments: &ArgumentsAfterFile) -> Option<ArgumentOwner> {
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
        return owner_of_last_argument(arguments, &SchemaParameters::NoSchema, &[]).ok()?;
    }

    let cli = Cli::try_parse_from(claudine_argv).ok()?;
    let (shared, words) = match cli.command? {
        Commands::Compose(args) => (args.shared, args.args),
        Commands::InlineCompose(args) => (args.shared, args.args),
        Commands::Sequence(args) => (args.shared, args.args),
        _ => return None,
    };
    let file = words.iter().find(|word| setter_key(word).is_none())?;
    let source = authored_source(file, &ScopeContext::discover())?;
    let schema = if tokens.iter().any(|token| setter_key(token).is_some()) {
        authored_schema_parameters(&source, None, None).ok()?
    } else {
        SchemaParameters::NoSchema
    };
    owner_of_last_argument(arguments, &schema, &candidates(&shared, &source)).ok()?
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
