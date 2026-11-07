//! Tab completion for property paths and their values, on `md compose`, `get`,
//! `set`, and `rm`.
//!
//! A value completer receives only the current word, so this reads the rest of the
//! line through [`recover_words`], opens the input document with the same opener
//! the command would use, and asks
//! [`path_candidates`](darkmatter::markdown::schemas::completion::path_candidates).
//!
//! Completion is advisory and passive. Every failure means fewer candidates and
//! no output: a document that cannot be opened or parsed, or a schema that cannot
//! be resolved, leaves the document's keys and the compiled baseline. It never
//! reads stdin, evaluates nothing, and makes no network request, since a remote
//! `$schema` fails to resolve without one.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use clap::CommandFactory;
use clap_complete::engine::CompletionCandidate;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::setters::{CompletionOverlay, RawSetter, TokenClass, classify_token, completion_overlay};
use darkmatter::markdown::schemas::completion::{
    PathCandidate, PathCommand, PathPosition, PathQuery, path_candidates, path_value_takes_files,
};
use darkmatter::markdown::schemas::{EffectiveSchema, frontmatter_as_json};
use serde_json::Value;
use simplified_schema::PropertyPath;

use crate::args::{Cli, complete_any_files, complete_markdown_files};
use crate::commands::schema::validate::document_schemas;
use crate::completion_shells::recover_words;
use crate::io::{OpenedArgument, open_argument};
use crate::request::{MdRequest, completion_request};

/// Completes a positional word of `md compose`: a setter's key or value, or the
/// input document.
pub fn complete_compose_word(current: &OsStr) -> Vec<CompletionCandidate> {
    let Some(typed) = current.to_str() else {
        return Vec::new();
    };
    let line = CommandLine::read(recover_words().before_cursor());
    if !typed.contains('=') {
        let site = DocumentSite::for_compose(&line);
        let mut candidates = shell_candidates(&site, PathCommand::Compose, PathPosition::Path(typed));
        candidates.extend(complete_markdown_files(current));
        return candidates;
    }
    match classify_token(typed) {
        TokenClass::Setter(setter) => {
            let site = DocumentSite::for_compose(&line);
            let mut candidates = shell_candidates(&site, PathCommand::Compose, PathPosition::Path(typed));
            if site.takes_files(PathCommand::Compose, PathPosition::Path(typed)) {
                let lead = &typed[..typed.len() - setter.raw.len()];
                candidates.extend(complete_any_files(OsStr::new(&setter.raw)).into_iter().map(|file| file.add_prefix(lead)));
            }
            candidates
        }
        // `./foo=ba` is a document, whatever its name holds.
        TokenClass::Path => complete_markdown_files(current),
        TokenClass::EmptyKey => Vec::new(),
    }
}

/// Completes a path argument of `md get`.
pub fn complete_get_path(current: &OsStr) -> Vec<CompletionCandidate> {
    complete_path_word(PathCommand::Get, current)
}

/// Completes a path argument of `md rm`.
pub fn complete_rm_path(current: &OsStr) -> Vec<CompletionCandidate> {
    complete_path_word(PathCommand::Rm, current)
}

/// Completes the path argument of `md set`.
pub fn complete_set_path(current: &OsStr) -> Vec<CompletionCandidate> {
    complete_path_word(PathCommand::Set, current)
}

/// Completes the value argument of `md set`.
pub fn complete_set_value(current: &OsStr) -> Vec<CompletionCandidate> {
    let Some(typed) = current.to_str() else {
        return Vec::new();
    };
    let line = CommandLine::read(recover_words().before_cursor());
    let Some(path) = line.positionals.get(1).and_then(|path| PropertyPath::parse(path).ok()) else {
        return Vec::new();
    };
    let site = DocumentSite::for_path_command(&line);
    let position = PathPosition::Value { path: &path, typed };
    let mut candidates = shell_candidates(&site, PathCommand::Set, position);
    if site.takes_files(PathCommand::Set, position) {
        candidates.extend(complete_any_files(current));
    }
    candidates
}

fn complete_path_word(command: PathCommand, current: &OsStr) -> Vec<CompletionCandidate> {
    let Some(typed) = current.to_str() else {
        return Vec::new();
    };
    let line = CommandLine::read(recover_words().before_cursor());
    let site = DocumentSite::for_path_command(&line);
    shell_candidates(&site, command, PathPosition::Path(typed))
}

fn shell_candidates(site: &DocumentSite, command: PathCommand, position: PathPosition<'_>) -> Vec<CompletionCandidate> {
    path_candidates(&site.query(command, position)).into_iter().map(shell_candidate).collect()
}

fn shell_candidate(candidate: PathCandidate) -> CompletionCandidate {
    CompletionCandidate::new(candidate.insert).help(candidate.help.map(Into::into))
}

/// The arguments before the cursor that decide what a path can name.
#[derive(Debug, Default, PartialEq, Eq)]
struct CommandLine {
    magic_roots: Vec<PathBuf>,
    positionals: Vec<String>,
    /// `--state`, as typed.
    defaults: Option<String>,
    /// `--set`, as typed.
    overrides: Option<String>,
}

impl CommandLine {
    /// Reads `words`, the program name first, through the same declarations `md`
    /// parses with, so a flag's value is never mistaken for the input document.
    fn read(words: &[String]) -> Self {
        let mut root = Cli::command();
        root.build();
        let mut subcommand: Option<&clap::Command> = None;
        let mut line = Self::default();
        let mut escaped = false;
        let mut words = words.iter().skip(1);
        while let Some(word) = words.next() {
            if escaped {
                line.positionals.push(word.clone());
            } else if word == "--" {
                escaped = true;
            } else if let Some(long) = word.strip_prefix("--") {
                let (name, inline) = long.split_once('=').map_or((long, None), |(name, value)| (name, Some(value.to_string())));
                let takes_value = subcommand
                    .unwrap_or(&root)
                    .get_arguments()
                    .find(|arg| arg.get_long() == Some(name))
                    .is_some_and(|arg| arg.get_action().takes_values());
                let value = if takes_value && inline.is_none() { words.next().cloned() } else { inline };
                line.record_flag(name, value, subcommand.is_some());
            } else if word.len() > 1 && word.starts_with('-') {
                let takes_value = word.chars().nth(1).is_some_and(|short| {
                    subcommand
                        .unwrap_or(&root)
                        .get_arguments()
                        .find(|arg| arg.get_short() == Some(short))
                        .is_some_and(|arg| arg.get_action().takes_values())
                });
                if takes_value && word.len() == 2 {
                    words.next();
                }
            } else if let Some(found) = subcommand.is_none().then(|| root.find_subcommand(word)).flatten() {
                subcommand = Some(found);
            } else {
                line.positionals.push(word.clone());
            }
        }
        line
    }

    fn record_flag(&mut self, name: &str, value: Option<String>, in_subcommand: bool) {
        let Some(value) = value else {
            return;
        };
        match (name, in_subcommand) {
            ("magic-root", false) => self.magic_roots.push(PathBuf::from(value)),
            ("state", true) => self.defaults = Some(value),
            ("set", true) => self.overrides = Some(value),
            _ => {}
        }
    }

    fn compose_input_and_setters(&self) -> (Option<&str>, Vec<RawSetter>) {
        let mut input = None;
        let mut setters = Vec::new();
        for word in &self.positionals {
            match classify_token(word) {
                TokenClass::Setter(setter) => setters.push(setter),
                TokenClass::Path if input.is_none() => input = Some(word.as_str()),
                TokenClass::Path | TokenClass::EmptyKey => {}
            }
        }
        (input, setters)
    }
}

/// `--state` or `--set` as an object. Anything else is ignored, as an incomplete
/// word on the line is.
fn object_argument(text: Option<&str>) -> Option<Value> {
    let parsed = biscuit_file::Json5::from_str(text?).ok()?;
    parsed.value().is_object().then(|| parsed.value().clone())
}

/// What one Tab press knows about the document on its line.
struct DocumentSite {
    frontmatter: Value,
    effective: Option<EffectiveSchema>,
    earlier_paths: Vec<PropertyPath>,
}

impl DocumentSite {
    fn for_compose(line: &CommandLine) -> Self {
        Self::for_compose_in(line, completion_request(&line.magic_roots).as_ref())
    }

    fn for_compose_in(line: &CommandLine, request: Option<&MdRequest>) -> Self {
        let (input, setters) = line.compose_input_and_setters();
        let defaults = object_argument(line.defaults.as_deref());
        let overrides = object_argument(line.overrides.as_deref());
        Self::load(request, input, |document| completion_overlay(document, defaults.as_ref(), overrides.as_ref(), setters))
    }

    /// `get`, `set`, and `rm`: the first positional is the document, and the rest
    /// are the paths already given.
    fn for_path_command(line: &CommandLine) -> Self {
        let input = line.positionals.first().map(String::as_str);
        let request = completion_request(&line.magic_roots);
        let mut site = Self::load(request.as_ref(), input, |document| completion_overlay(document, None, None, []));
        site.earlier_paths = line.positionals.iter().skip(1).filter_map(|path| PropertyPath::parse(path).ok()).collect();
        site
    }

    fn load(request: Option<&MdRequest>, input: Option<&str>, overlay: impl FnOnce(&Markdown) -> CompletionOverlay) -> Self {
        let opened = request.zip(input).and_then(|(request, input)| open_document(input, request));
        let overlay = overlay(&opened.as_ref().map_or_else(|| Markdown::new(""), |(_, document)| document.clone()));
        let effective = request.zip(opened.as_ref()).and_then(|(request, (argument, _))| {
            resolve_schema(argument, &overlay.markdown, request)
        });
        Self {
            frontmatter: frontmatter_as_json(&overlay.markdown),
            effective,
            earlier_paths: overlay.paths,
        }
    }

    fn query<'a>(&'a self, command: PathCommand, position: PathPosition<'a>) -> PathQuery<'a> {
        PathQuery {
            effective: self.effective.as_ref(),
            frontmatter: &self.frontmatter,
            command,
            position,
            earlier_paths: &self.earlier_paths,
        }
    }

    fn takes_files(&self, command: PathCommand, position: PathPosition<'_>) -> bool {
        path_value_takes_files(&self.query(command, position))
    }
}

/// The document `input` names. `-` is stdin, which completion must not consume.
fn open_document(input: &str, request: &MdRequest) -> Option<(OpenedArgument, Markdown)> {
    if input == "-" {
        return None;
    }
    let opened = open_argument(Path::new(input), request).ok()?;
    let document = Markdown::try_from(opened.path()).ok()?;
    Some((opened, document))
}

/// The schema for `overlay`, assembled as `md set` assembles it. `None` both when
/// nothing applies and when anything fails, so a partly assembled schema is never
/// offered as the authority; the candidates then read the baseline.
fn resolve_schema(argument: &OpenedArgument, overlay: &Markdown, request: &MdRequest) -> Option<EffectiveSchema> {
    let context = request.document_context(Some(argument.reference()), argument.path()).ok()?;
    let schemas = document_schemas(request, context).ok()?;
    schemas.effective_for(overlay).ok().flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(words: &[&str]) -> CommandLine {
        CommandLine::read(&words.iter().map(ToString::to_string).collect::<Vec<_>>())
    }

    #[test]
    fn flag_values_are_not_positionals() {
        let read = line(&["md", "compose", "--output", "json", "doc.md", "a=1", "--frontmatter", "b=2"]);
        assert_eq!(read.positionals, ["doc.md", "a=1", "b=2"]);
    }

    #[test]
    fn an_inline_flag_value_consumes_no_word() {
        let read = line(&["md", "compose", "--output=json", "doc.md"]);
        assert_eq!(read.positionals, ["doc.md"]);
    }

    #[test]
    fn magic_roots_are_read_before_the_subcommand_only() {
        let read = line(&["md", "--magic-root", "one", "--magic-root=two", "get", "--magic-root", "three", "doc.md"]);
        assert_eq!(read.magic_roots, [PathBuf::from("one"), PathBuf::from("two")]);
    }

    #[test]
    fn state_and_set_are_read_from_the_subcommand() {
        let read = line(&["md", "compose", "--state", "{a:1}", "--set={b:2}", "doc.md"]);
        assert_eq!(read.defaults.as_deref(), Some("{a:1}"));
        assert_eq!(read.overrides.as_deref(), Some("{b:2}"));
        assert_eq!(read.positionals, ["doc.md"]);
    }

    #[test]
    fn words_after_the_escape_are_positionals() {
        let read = line(&["md", "compose", "--", "--output", "get"]);
        assert_eq!(read.positionals, ["--output", "get"]);
    }

    #[test]
    fn a_word_naming_a_subcommand_is_only_special_once() {
        let read = line(&["md", "get", "get", "compose"]);
        assert_eq!(read.positionals, ["get", "compose"]);
    }

    #[test]
    fn a_dangling_flag_is_not_an_error() {
        let read = line(&["md", "compose", "--state"]);
        assert_eq!(read, CommandLine::default());
    }

    #[test]
    fn the_compose_input_can_be_anywhere_on_the_line() {
        let read = line(&["md", "compose", "a=1", "./b=c.md", "d=2", "second.md", "=x"]);
        let (input, setters) = read.compose_input_and_setters();
        assert_eq!(input, Some("./b=c.md"));
        assert_eq!(setters.iter().map(|setter| setter.argument.as_str()).collect::<Vec<_>>(), ["a=1", "d=2"]);
    }

    #[test]
    fn stdin_is_never_opened() {
        let temp_dir = tempfile::tempdir().unwrap();
        let request = MdRequest::new(darkmatter::markdown::compose::RequestSnapshot::new(temp_dir.path()));
        assert!(open_document("-", &request).is_none());
    }

    /// Counters and the observation frame are private to this process and thread,
    /// so each measurement below is one Tab press. The first press compiles the
    /// baseline, and that cost has nothing to do with the document, so the document's
    /// own expressions are judged by comparing warmed presses.
    #[test]
    fn one_tab_prepares_once_and_reads_only_the_documents_schema_files() {
        use darkmatter::work_counters::{Work, WorkSnapshot, snapshot};
        use simplified_schema::resolve::observed::{Observed, collect};

        let temp_dir = tempfile::tempdir().unwrap();
        let root = biscuit_file::canonicalize_simplified(temp_dir.path()).unwrap();
        let write = |name: &str, text: &str| {
            let path = root.join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        };
        write(".git/HEAD", "ref: refs/heads/main\n");
        std::fs::create_dir_all(root.join(".git/objects")).unwrap();
        std::fs::create_dir_all(root.join(".git/refs")).unwrap();
        write("quiet.md", "---\n$schema: ./s.yaml\n---\n");
        write("active.md", "---\n$schema: ./s.yaml\ncmd: $(echo hi)\nshown: \"{{ 1 + 1 }}\"\n---\n::file ./other.md\n{{ 2 + 2 }}\n");
        write("s.yaml", "$schema:\n  first: name@./types.yaml\n  second: name@./types.yaml\n");
        write("types.yaml", "kind: schema\ntypes:\n  name: string\n");
        write("schemas/lang.trigger.yaml", "kind: schema-trigger\nmatch:\n  lang: enum(rust; required)\n$schema:\n  edition: string\n");
        write("unrelated.yaml", "$schema:\n  never: string\n");
        write("other.md", "---\n$schema: ./unrelated.yaml\n---\n");
        let request = MdRequest::new(darkmatter::markdown::compose::RequestSnapshot::new(&root));

        let tab = |document: &str| -> (WorkSnapshot, Observed) {
            let before = snapshot();
            let (site, observed) = collect(|| DocumentSite::for_compose_in(&line(&["md", "compose", document]), Some(&request)));
            assert!(site.effective.is_some(), "the schema must resolve, or nothing here was read");
            (snapshot().since(&before), observed)
        };

        let (first, read) = tab("quiet.md");
        assert_eq!(first.get(Work::TriggerDiscovery), 1, "{:?}", first.nonzero());
        assert_eq!(first.get(Work::EffectivePreparation), 1, "{:?}", first.nonzero());
        assert_eq!(first.get(Work::NamedTypeTableParse), 1, "two import sites, one table: {:?}", first.nonzero());
        // Trigger files are read by discovery's own source, which the first count covers.
        assert_eq!(read.read, [root.join("s.yaml"), root.join("types.yaml")].into_iter().collect());

        let (warmed, _) = tab("quiet.md");
        let (active, read) = tab("active.md");
        assert_eq!(active.get(Work::ExpressionParse), warmed.get(Work::ExpressionParse), "the document's expressions stay unparsed");
        assert_eq!(active.get(Work::LifecycleConfigParse), 0, "{:?}", active.nonzero());
        assert!(!read.read.contains(&root.join("unrelated.yaml")), "a transcluded child is not followed: {:?}", read.read);
    }

    #[test]
    fn an_object_argument_ignores_anything_else() {
        assert_eq!(object_argument(Some("{a:1}")), Some(serde_json::json!({"a": 1})));
        for text in ["[1]", "5", "{a:", ""] {
            assert_eq!(object_argument(Some(text)), None, "{text}");
        }
        assert_eq!(object_argument(None), None);
    }
}
