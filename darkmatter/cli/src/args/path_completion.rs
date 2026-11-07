//! Tab completion for property paths and their values, on `md compose`, `get`,
//! `set`, and `rm`.
//!
//! A value completer receives only the current word, so this reads the rest of the
//! line through [`recover_words`], opens the input document with the same opener
//! the command would use, and asks
//! [`path_candidates`](darkmatter::markdown::schemas::completion::path_candidates).
//!
//! Completion reads the schema the command on the line would use: the same baseline
//! selection and the same schema triggers, so a disabled layer offers nothing.
//!
//! Completion is advisory and passive. Every failure means fewer candidates and
//! no output: a document that cannot be opened or parsed, or a schema that cannot
//! be resolved, leaves the document's keys and the selected baseline. The compiled
//! baseline answers only when the selection itself cannot be read. It never reads
//! stdin, evaluates nothing, and makes no network request, since a remote `$schema`
//! fails to resolve without one.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use clap::CommandFactory;
use clap_complete::engine::CompletionCandidate;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::setters::{CompletionOverlay, RawSetter, TokenClass, classify_token, completion_overlay};
use darkmatter::markdown::schemas::completion::{
    PathCandidate, PathCommand, PathPosition, PathQuery, PathSchema, path_candidates, path_value_takes_files,
};
use darkmatter::markdown::schemas::{DarkmatterSchemas, EffectiveSchema, frontmatter_as_json};
use serde_json::Value;
use simplified_schema::PropertyPath;

use crate::args::policy::{self, PolicyCommand, PolicyVerb, ResolvedPolicy};
use crate::args::{Cli, complete_any_files, complete_markdown_files};
use crate::commands::compose::compose_baseline_schemas;
use crate::commands::schema::validate::{effective_document_schema, request_schemas};
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
    /// `--baseline-schema`, as typed.
    baseline_schema: Option<PathBuf>,
    /// Each `--disable`, as typed.
    disabled: Vec<String>,
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
            ("baseline-schema", true) => self.baseline_schema = Some(PathBuf::from(value)),
            ("disable", true) => self.disabled.push(value),
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

    /// The `md compose` policy on this line. A `--disable` it would refuse selects
    /// nothing, as an incomplete word on the line does.
    fn compose_policy(&self) -> SchemaPolicy<'_> {
        let selectors: Vec<_> = self
            .disabled
            .iter()
            .filter_map(|id| policy::parse_selector(PolicyCommand::Compose, PolicyVerb::Disable, id).ok())
            .collect();
        let resolved = ResolvedPolicy::resolve(&[], &[], &selectors).unwrap_or_default();
        SchemaPolicy::Compose {
            baseline_schema: self.baseline_schema.as_ref(),
            no_baseline_schema: resolved.disables("baseline-schema"),
            no_schema_triggers: resolved.disables("schema-triggers"),
        }
    }
}

/// How the command being completed selects its schema layers.
#[derive(Debug, Clone, Copy)]
enum SchemaPolicy<'a> {
    /// `md compose`, from its flags and the environment.
    Compose {
        baseline_schema: Option<&'a PathBuf>,
        no_baseline_schema: bool,
        no_schema_triggers: bool,
    },
    /// `md get`, `set`, and `rm`, which take no schema flags and read what
    /// `md schema validate` reads from the environment.
    Edit,
}

impl SchemaPolicy<'_> {
    fn baseline_schemas(self, request: &MdRequest) -> Option<DarkmatterSchemas> {
        match self {
            Self::Compose { baseline_schema, no_baseline_schema, .. } => compose_baseline_schemas(baseline_schema, no_baseline_schema, request).ok(),
            Self::Edit => request_schemas(request).ok(),
        }
    }

    fn discovers_triggers(self) -> bool {
        !matches!(self, Self::Compose { no_schema_triggers: true, .. })
    }
}

/// The schema one Tab press settled on.
enum SelectedSchema {
    Effective(Box<EffectiveSchema>),
    CompiledBaseline,
    Nothing,
}

impl SelectedSchema {
    fn from_effective(effective: Option<EffectiveSchema>) -> Self {
        effective.map_or(Self::Nothing, |effective| Self::Effective(Box::new(effective)))
    }

    fn as_path_schema(&self) -> PathSchema<'_> {
        match self {
            Self::Effective(effective) => PathSchema::Effective(effective),
            Self::CompiledBaseline => PathSchema::CompiledBaseline,
            Self::Nothing => PathSchema::Nothing,
        }
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
    schema: SelectedSchema,
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
        Self::load(request, line.compose_policy(), input, |document| {
            completion_overlay(document, defaults.as_ref(), overrides.as_ref(), setters)
        })
    }

    /// `get`, `set`, and `rm`: the first positional is the document, and the rest
    /// are the paths already given.
    fn for_path_command(line: &CommandLine) -> Self {
        let input = line.positionals.first().map(String::as_str);
        let request = completion_request(&line.magic_roots);
        let mut site = Self::load(request.as_ref(), SchemaPolicy::Edit, input, |document| completion_overlay(document, None, None, []));
        site.earlier_paths = line.positionals.iter().skip(1).filter_map(|path| PropertyPath::parse(path).ok()).collect();
        site
    }

    fn load(
        request: Option<&MdRequest>,
        policy: SchemaPolicy<'_>,
        input: Option<&str>,
        overlay: impl FnOnce(&Markdown) -> CompletionOverlay,
    ) -> Self {
        let opened = request.zip(input).and_then(|(request, input)| open_document(input, request));
        let overlay = overlay(&opened.as_ref().map_or_else(|| Markdown::new(""), |(_, document)| document.clone()));
        let document = opened.as_ref().map(|(argument, _)| (argument, &overlay.markdown));
        // One Tab press reads each schema file once, across the baseline, trigger,
        // and document layers that each resolve on their own.
        let schema = request.map_or(SelectedSchema::CompiledBaseline, |request| {
            simplified_schema::resolve::buffers::with_reads_retained(|| select_schema(policy, document, request))
        });
        Self {
            frontmatter: frontmatter_as_json(&overlay.markdown),
            schema,
            earlier_paths: overlay.paths,
        }
    }

    fn query<'a>(&'a self, command: PathCommand, position: PathPosition<'a>) -> PathQuery<'a> {
        PathQuery {
            schema: self.schema.as_path_schema(),
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

/// The schema the command would use for `document`, its opened argument and
/// overlay.
///
/// With no document, or one whose schema fails, a partly assembled schema is never
/// offered as the authority: the selected baseline answers alone. A baseline that
/// cannot be loaded leaves the compiled one.
fn select_schema(policy: SchemaPolicy<'_>, document: Option<(&OpenedArgument, &Markdown)>, request: &MdRequest) -> SelectedSchema {
    let Some(schemas) = policy.baseline_schemas(request) else {
        return SelectedSchema::CompiledBaseline;
    };
    if let Some((argument, overlay)) = document
        && let Some(effective) = document_effective(policy, schemas.clone(), argument, overlay, request)
    {
        return SelectedSchema::from_effective(effective);
    }
    if schemas.uses_darkmatter_baseline() {
        // The same declarations, without compiling validators for them.
        return SelectedSchema::CompiledBaseline;
    }
    schemas
        .effective_for(&Markdown::new(""))
        .map_or(SelectedSchema::CompiledBaseline, SelectedSchema::from_effective)
}

/// `None` when any part of the document's schema fails; `Some(None)` when no layer
/// applies to it.
fn document_effective(
    policy: SchemaPolicy<'_>,
    schemas: DarkmatterSchemas,
    argument: &OpenedArgument,
    overlay: &Markdown,
    request: &MdRequest,
) -> Option<Option<EffectiveSchema>> {
    let context = argument.document_context(request).ok()?;
    effective_document_schema(schemas, overlay, context, policy.discovers_triggers()).ok().map(|selected| selected.schema)
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
    fn baseline_flags_are_read_from_the_subcommand() {
        let read = line(&["md", "compose", "--baseline-schema", "b.yaml", "--disable", "schema-triggers", "--disable=baseline-schema", "doc.md"]);
        assert_eq!(read.baseline_schema, Some(PathBuf::from("b.yaml")));
        assert_eq!(read.disabled, ["schema-triggers", "baseline-schema"]);
        assert_eq!(read.positionals, ["doc.md"]);
    }

    #[test]
    fn a_disable_compose_would_refuse_selects_nothing() {
        let read = line(&["md", "compose", "--disable", "nonsense", "--disable", "schema-triggers", "doc.md"]);
        assert!(matches!(
            read.compose_policy(),
            SchemaPolicy::Compose { no_baseline_schema: false, no_schema_triggers: true, .. }
        ));
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
            assert!(matches!(site.schema, SelectedSchema::Effective(_)), "the schema must resolve, or nothing here was read");
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

    /// `md get`, `set`, and `rm` under a custom catalog baseline: one Tab opens the
    /// catalog, the document's schema, and the file both of them import once each.
    #[test]
    fn one_edit_tab_opens_each_schema_file_once_across_the_baseline_and_document() {
        use std::collections::BTreeMap;

        let temp_dir = tempfile::tempdir().unwrap();
        let root = biscuit_file::canonicalize_simplified(temp_dir.path()).unwrap();
        let write = |name: &str, text: &str| std::fs::write(root.join(name), text).unwrap();
        write("types.yaml", "kind: schema\ntypes:\n  shared: {name: string}\n");
        write(
            "base.yaml",
            "kind: schema\n$schema:\n  fm:\n    status: enum(custom)\n  ctx: shared@./types.yaml\n  current: shared@./types.yaml\n  env: object\n  err: {message: string}\n  timing: {elapsed: number}\n",
        );
        write("s.yaml", "$schema:\n  first: shared@./types.yaml\n  second: shared@./types.yaml\n");
        write("doc.md", "---\n$schema: ./s.yaml\nstatus: custom\n---\n");
        let env = [("BASELINE_SCHEMA".to_string(), "./base.yaml".to_string())].into_iter().collect();
        let request = MdRequest::new(darkmatter::markdown::compose::RequestSnapshot::new(&root).with_env(env));
        let status = PropertyPath::parse("status").unwrap();

        for (command, position, expected) in [
            (PathCommand::Get, PathPosition::Path("sta"), "status"),
            (PathCommand::Set, PathPosition::Value { path: &status, typed: "cu" }, "custom"),
            (PathCommand::Rm, PathPosition::Path("sta"), "status"),
        ] {
            let (site, observed) = simplified_schema::resolve::observed::collect(|| {
                DocumentSite::load(Some(&request), SchemaPolicy::Edit, Some("doc.md"), |document| completion_overlay(document, None, None, []))
            });
            let words: Vec<String> = path_candidates(&site.query(command, position)).into_iter().map(|candidate| candidate.insert).collect();
            assert!(words.iter().any(|word| word == expected), "{command:?}: {words:?}");
            let mut opened = BTreeMap::new();
            for path in observed.opened {
                *opened.entry(path.file_name().unwrap().to_string_lossy().into_owned()).or_insert(0) += 1;
            }
            let once: BTreeMap<String, usize> = ["base.yaml", "s.yaml", "types.yaml"].map(|name| (name.to_string(), 1)).into_iter().collect();
            assert_eq!(opened, once, "{command:?}");
        }
    }

    /// `md compose --baseline-schema`: one Tab opens the flag's file once.
    #[test]
    fn one_compose_tab_opens_the_baseline_flags_file_once() {
        let temp_dir = tempfile::tempdir().unwrap();
        let root = biscuit_file::canonicalize_simplified(temp_dir.path()).unwrap();
        std::fs::write(root.join("base.yaml"), "$schema:\n  status: enum(custom)\n").unwrap();
        std::fs::write(root.join("doc.md"), "---\ntitle: T\n---\n").unwrap();
        let request = MdRequest::new(darkmatter::markdown::compose::RequestSnapshot::new(&root));

        let (site, observed) = simplified_schema::resolve::observed::collect(|| {
            DocumentSite::for_compose_in(&line(&["md", "compose", "--baseline-schema", "base.yaml", "doc.md"]), Some(&request))
        });
        let words: Vec<String> =
            path_candidates(&site.query(PathCommand::Compose, PathPosition::Path("status="))).into_iter().map(|candidate| candidate.insert).collect();
        assert_eq!(words, ["status=custom"]);
        assert_eq!(observed.opened, [root.join("base.yaml")]);
    }

    /// One Tab with the schemas in the discovered `SCHEMAS_DIR` root: trigger
    /// discovery reads every root file, and the resolution that follows reuses those
    /// bytes. `lang.trigger.yaml` is read by discovery alone, so its single open
    /// shows discovery's reads are counted.
    mod discovered_root {
        use super::*;
        use std::collections::BTreeMap;

        const TYPES: &str = "kind: schema\ntypes:\n  shared: {name: string}\n";
        const TRIGGER: &str = "kind: schema-trigger\nmatch:\n  lang: enum(rust; required)\n$schema:\n  owner: shared@./types.yaml\n";

        struct Fixture {
            _temp_dir: tempfile::TempDir,
            root: PathBuf,
            request: MdRequest,
        }

        /// A repository whose `schemas/` holds `s.yaml`, `types.yaml`, the trigger,
        /// and `extra`, with `doc.md` naming `schema` and matching the trigger. The
        /// home is the fixture's own empty folder, and `env` is the whole environment
        /// beside `SCHEMAS_DIR`.
        fn fixture(schema: &str, extra: &[(&str, &str)], env: &[(&str, &str)]) -> Fixture {
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
            std::fs::create_dir_all(root.join("home")).unwrap();
            write("schemas/s.yaml", "$schema:\n  status: enum(custom)\n  owner: shared@./types.yaml\n");
            write("schemas/types.yaml", TYPES);
            write("schemas/lang.trigger.yaml", TRIGGER);
            for (name, text) in extra {
                write(name, text);
            }
            write("doc.md", &format!("---\n$schema: {schema}\nstatus: custom\nlang: rust\n---\n"));
            let env = env
                .iter()
                .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
                .chain([("SCHEMAS_DIR".to_string(), root.join("schemas").display().to_string())])
                .collect();
            let snapshot = darkmatter::markdown::compose::RequestSnapshot::new(&root).with_home(Some(root.join("home"))).with_env(env);
            Fixture { _temp_dir: temp_dir, request: MdRequest::new(snapshot), root }
        }

        impl Fixture {
            /// The candidates `command` offers at `position` after one Tab, and how
            /// many times that Tab opened each file, by path under the fixture root.
            fn tab(&self, compose: Option<&[&str]>, command: PathCommand, position: PathPosition<'_>) -> (Vec<String>, BTreeMap<String, usize>) {
                let (site, observed) = simplified_schema::resolve::observed::collect(|| match compose {
                    Some(words) => DocumentSite::for_compose_in(&line(words), Some(&self.request)),
                    None => DocumentSite::load(Some(&self.request), SchemaPolicy::Edit, Some("doc.md"), |document| completion_overlay(document, None, None, [])),
                });
                let words = path_candidates(&site.query(command, position)).into_iter().map(|candidate| candidate.insert).collect();
                let mut opened = BTreeMap::new();
                for path in observed.opened {
                    let name = path.strip_prefix(&self.root).unwrap_or(&path).to_string_lossy().replace('\\', "/");
                    *opened.entry(name).or_insert(0) += 1;
                }
                (words, opened)
            }

            fn edit(&self, command: PathCommand, position: PathPosition<'_>) -> (Vec<String>, BTreeMap<String, usize>) {
                self.tab(None, command, position)
            }
        }

        /// Each of `names` opened exactly once, with the files every fixture reads.
        fn once(names: &[&str]) -> BTreeMap<String, usize> {
            ["schemas/s.yaml", "schemas/types.yaml", "schemas/lang.trigger.yaml"].iter().chain(names).map(|name| ((*name).to_string(), 1)).collect()
        }

        fn has(words: &[String], expected: &str) -> bool {
            words.iter().any(|word| word == expected)
        }

        #[test]
        fn get_reads_each_root_file_once() {
            let (words, opened) = fixture("./schemas/s.yaml", &[], &[]).edit(PathCommand::Get, PathPosition::Path("sta"));
            assert!(has(&words, "status"), "{words:?}");
            assert_eq!(opened, once(&[]));
        }

        #[test]
        fn set_reads_each_root_file_once() {
            let status = PropertyPath::parse("status").unwrap();
            let (words, opened) = fixture("./schemas/s.yaml", &[], &[]).edit(PathCommand::Set, PathPosition::Value { path: &status, typed: "cu" });
            assert_eq!(words, ["custom"]);
            assert_eq!(opened, once(&[]));
        }

        #[test]
        fn rm_reads_each_root_file_once() {
            let (words, opened) = fixture("./schemas/s.yaml", &[], &[]).edit(PathCommand::Rm, PathPosition::Path("sta"));
            assert!(has(&words, "status"), "{words:?}");
            assert_eq!(opened, once(&[]));
        }

        #[test]
        fn compose_reads_each_root_file_once() {
            let fixture = fixture("./schemas/s.yaml", &[], &[]);
            let (words, opened) = fixture.tab(Some(&["md", "compose", "doc.md"]), PathCommand::Compose, PathPosition::Path("status=cu"));
            assert_eq!(words, ["status=custom"]);
            assert_eq!(opened, once(&[]));
        }

        /// `BASELINE_SCHEMA` naming a catalog inside the root: baseline selection,
        /// discovery, and resolution share one read of each file.
        #[test]
        fn an_edit_baseline_inside_the_root_is_read_once() {
            const CATALOG: &str = "kind: schema\n$schema:\n  fm:\n    status: enum(custom)\n  ctx: shared@./types.yaml\n  current: shared@./types.yaml\n  env: object\n  err: {message: string}\n  timing: {elapsed: number}\n";
            let fixture = fixture("./schemas/s.yaml", &[("schemas/base.yaml", CATALOG)], &[("BASELINE_SCHEMA", "./schemas/base.yaml")]);
            let status = PropertyPath::parse("status").unwrap();
            for (command, position, expected) in [
                (PathCommand::Get, PathPosition::Path("sta"), "status"),
                (PathCommand::Set, PathPosition::Value { path: &status, typed: "cu" }, "custom"),
                (PathCommand::Rm, PathPosition::Path("sta"), "status"),
            ] {
                let (words, opened) = fixture.edit(command, position);
                assert!(has(&words, expected), "{command:?}: {words:?}");
                assert_eq!(opened, once(&["schemas/base.yaml"]), "{command:?}");
            }
        }

        #[test]
        fn a_compose_baseline_flag_inside_the_root_is_read_once() {
            let fixture = fixture("./schemas/s.yaml", &[("schemas/base.yaml", "$schema:\n  status: enum(custom)\n")], &[]);
            let (words, opened) = fixture.tab(
                Some(&["md", "compose", "--baseline-schema", "schemas/base.yaml", "doc.md"]),
                PathCommand::Compose,
                PathPosition::Path("status=cu"),
            );
            assert_eq!(words, ["status=custom"]);
            assert_eq!(opened, once(&["schemas/base.yaml"]));
        }

        /// With trigger discovery disabled the trigger file is never read.
        #[test]
        fn compose_without_trigger_discovery_reads_no_trigger_file() {
            let fixture = fixture("./schemas/s.yaml", &[], &[]);
            let (words, opened) = fixture.tab(
                Some(&["md", "compose", "--disable", "schema-triggers", "doc.md"]),
                PathCommand::Compose,
                PathPosition::Path("status=cu"),
            );
            assert_eq!(words, ["status=custom"]);
            let expected: BTreeMap<String, usize> = [("schemas/s.yaml".to_string(), 1), ("schemas/types.yaml".to_string(), 1)].into_iter().collect();
            assert_eq!(opened, expected);
        }

        /// A document schema outside the root whose `example(...)` names a root file.
        #[test]
        fn an_example_file_inside_the_root_is_read_once() {
            let fixture = fixture(
                "./outside.yaml",
                &[
                    ("outside.yaml", "$schema:\n  status: enum(custom)\n  greeting: string(example(./schemas/ex.yaml))\n"),
                    ("schemas/ex.yaml", "kind: example\ninvocation: demo\nreturns: hi\ndescription: text\n"),
                ],
                &[],
            );
            let (words, opened) = fixture.edit(PathCommand::Get, PathPosition::Path("gre"));
            assert!(has(&words, "greeting"), "{words:?}");
            assert_eq!(opened, once(&["outside.yaml", "schemas/ex.yaml"]));
        }
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
