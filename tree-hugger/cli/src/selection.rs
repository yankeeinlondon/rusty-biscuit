//! The record selectors: the flags that choose which declarations (and, for
//! `imports`, which import records) a command reports, translated into the
//! library's one filter path ([`SymbolQuery`] predicates and
//! [`NamePattern`]). The user-facing rules are in `docs/symbol-reports.md`.

use std::path::{Path, PathBuf};

use biscuit_file::{FileReference, FileResolutionContext};
use clap::ValueEnum;
use clap::ValueHint;
use clap_complete::engine::ArgValueCompleter;
use tree_hugger::{
    ImportRecord, NamePattern, ProgrammingLanguage, SymbolKindV2, SymbolQuery, SymbolScope,
    VisibilityLevel,
};

use crate::complete_source_path;

/// `--kind`: the snake_case spelling of each [`SymbolKindV2`] variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "snake_case")]
pub(crate) enum KindArg {
    Function,
    Method,
    Constructor,
    Type,
    Class,
    Interface,
    Enum,
    Trait,
    Module,
    Namespace,
    Field,
    Property,
    Variable,
    Constant,
    Parameter,
    TypeAlias,
    EnumVariant,
    Macro,
    Unknown,
}

impl From<KindArg> for SymbolKindV2 {
    fn from(kind: KindArg) -> Self {
        match kind {
            KindArg::Function => Self::Function,
            KindArg::Method => Self::Method,
            KindArg::Constructor => Self::Constructor,
            KindArg::Type => Self::Type,
            KindArg::Class => Self::Class,
            KindArg::Interface => Self::Interface,
            KindArg::Enum => Self::Enum,
            KindArg::Trait => Self::Trait,
            KindArg::Module => Self::Module,
            KindArg::Namespace => Self::Namespace,
            KindArg::Field => Self::Field,
            KindArg::Property => Self::Property,
            KindArg::Variable => Self::Variable,
            KindArg::Constant => Self::Constant,
            KindArg::Parameter => Self::Parameter,
            KindArg::TypeAlias => Self::TypeAlias,
            KindArg::EnumVariant => Self::EnumVariant,
            KindArg::Macro => Self::Macro,
            KindArg::Unknown => Self::Unknown,
        }
    }
}

/// `--scope`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum ScopeArg {
    Item,
    Member,
    Local,
}

impl From<ScopeArg> for SymbolScope {
    fn from(scope: ScopeArg) -> Self {
        match scope {
            ScopeArg::Item => Self::Item,
            ScopeArg::Member => Self::Member,
            ScopeArg::Local => Self::Local,
        }
    }
}

/// `--visibility`: the snake_case spelling of each [`VisibilityLevel`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "snake_case")]
pub(crate) enum VisibilityArg {
    Public,
    Protected,
    Private,
    Internal,
    Package,
    Crate,
    ModulePrivate,
    ProtectedInternal,
    PrivateProtected,
    Unknown,
}

impl From<VisibilityArg> for VisibilityLevel {
    fn from(level: VisibilityArg) -> Self {
        match level {
            VisibilityArg::Public => Self::Public,
            VisibilityArg::Protected => Self::Protected,
            VisibilityArg::Private => Self::Private,
            VisibilityArg::Internal => Self::Internal,
            VisibilityArg::Package => Self::Package,
            VisibilityArg::Crate => Self::Crate,
            VisibilityArg::ModulePrivate => Self::ModulePrivate,
            VisibilityArg::ProtectedInternal => Self::ProtectedInternal,
            VisibilityArg::PrivateProtected => Self::PrivateProtected,
            VisibilityArg::Unknown => Self::Unknown,
        }
    }
}

/// `--file`, `--name` and `--exact`: the explicit file and name selectors
/// every symbol command and `imports` accept.
#[derive(clap::Args, Debug, Clone, Default)]
pub(crate) struct TargetArgs {
    /// Scan this file, directory pattern or glob (repeatable). Unlike a
    /// positional FILTER it is never taken for a symbol name; a file it
    /// names is scanned even when ignored or under a fixtures directory
    #[arg(
        long = "file",
        value_name = "PATH|GLOB",
        display_order = 20,
        value_hint = ValueHint::AnyPath,
        add = ArgValueCompleter::new(complete_source_path),
    )]
    pub(crate) files: Vec<String>,

    /// Keep symbols whose name matches PATTERN (repeatable; any may match).
    /// Lowercase matches any case; `*` matches any run; a pattern with `#`
    /// or the language's separator matches the qualified name, which usually
    /// starts with the file's path (`src/lib.rs#Type::name`)
    #[arg(long = "name", short = 'n', value_name = "PATTERN", display_order = 21)]
    pub(crate) names: Vec<String>,

    /// Match names and `--in` whole and literally (`*` included); smart case
    /// still applies
    #[arg(long, display_order = 22)]
    pub(crate) exact: bool,
}

impl TargetArgs {
    /// `token` as a name pattern under `--exact`.
    pub(crate) fn pattern(&self, token: &str) -> NamePattern {
        if self.exact {
            NamePattern::exact(token)
        } else {
            NamePattern::new(token)
        }
    }
}

/// The record predicates of `functions`, `types`, `symbols` and `classes`.
#[derive(clap::Args, Debug, Clone, Default)]
pub(crate) struct SelectionArgs {
    #[clap(flatten)]
    pub(crate) targets: TargetArgs,

    /// Keep only this kind (repeatable); narrows the command's own kinds,
    /// never widens them
    #[arg(long = "kind", value_enum, value_name = "KIND", display_order = 23)]
    pub(crate) kinds: Vec<KindArg>,

    /// Keep only this scope (repeatable). `symbols` lists `item` and
    /// `member` by default; the other commands every scope
    #[arg(long = "scope", value_enum, value_name = "SCOPE", display_order = 24)]
    pub(crate) scopes: Vec<ScopeArg>,

    /// Keep every scope, locals included
    #[arg(long, conflicts_with = "scopes", display_order = 25)]
    pub(crate) all_scopes: bool,

    /// Keep only this exact visibility level (repeatable); `unknown` matches
    /// only unknown
    #[arg(long = "visibility", value_enum, value_name = "LEVEL", display_order = 26)]
    pub(crate) visibility: Vec<VisibilityArg>,

    /// Keep members of containers whose name matches PATTERN (repeatable);
    /// for `classes`, the class or type's own name
    #[arg(long = "in", value_name = "PATTERN", display_order = 27)]
    pub(crate) containers: Vec<String>,

    /// Drop test code (test files, test attributes, Rust `cfg(test)`)
    #[arg(long, conflicts_with = "tests_only", display_order = 32)]
    pub(crate) exclude_tests: bool,

    /// Keep only test code
    #[arg(long, display_order = 33)]
    pub(crate) tests_only: bool,

    /// Keep only declarations without a doc comment (never Lua, Perl, Bash
    /// or Zsh, which have no doc-comment detection)
    #[arg(long, display_order = 34)]
    pub(crate) undocumented: bool,

    /// Keep only deprecated declarations
    #[arg(long, display_order = 35)]
    pub(crate) deprecated: bool,

    /// Keep only `async` functions, methods and constructors
    #[arg(long = "async", display_order = 36)]
    pub(crate) is_async: bool,

    /// Keep only functions whose declared return type matches GLOB
    /// (whole-string, case-sensitive, `*` wildcard)
    #[arg(long, value_name = "GLOB", display_order = 37)]
    pub(crate) returns: Option<String>,

    /// Keep only functions with a parameter whose declared type matches GLOB
    #[arg(long, value_name = "GLOB", display_order = 38)]
    pub(crate) param_type: Option<String>,

    /// Keep only functions with at least N declared parameters
    #[arg(long, value_name = "N", display_order = 39)]
    pub(crate) min_params: Option<u32>,
}

impl SelectionArgs {
    /// The record-selecting flags given, for `--json-full`'s rejection
    /// (`--file` and `--exact` select no record).
    pub(crate) fn given_flags(&self) -> Vec<&'static str> {
        let flags = [
            (!self.targets.names.is_empty(), "--name"),
            (!self.kinds.is_empty(), "--kind"),
            (!self.scopes.is_empty(), "--scope"),
            (self.all_scopes, "--all-scopes"),
            (!self.visibility.is_empty(), "--visibility"),
            (!self.containers.is_empty(), "--in"),
            (self.exclude_tests, "--exclude-tests"),
            (self.tests_only, "--tests-only"),
            (self.undocumented, "--undocumented"),
            (self.deprecated, "--deprecated"),
            (self.is_async, "--async"),
            (self.returns.is_some(), "--returns"),
            (self.param_type.is_some(), "--param-type"),
            (self.min_params.is_some(), "--min-params"),
        ];
        flags
            .into_iter()
            .filter_map(|(given, flag)| given.then_some(flag))
            .collect()
    }
}

/// One run's record selection, resolved from the flags and the positional
/// name filters.
#[derive(Debug, Clone, Default)]
pub(crate) struct Selection {
    /// `--kind`; empty keeps the command's kinds.
    pub(crate) kinds: Vec<SymbolKindV2>,
    pub(crate) scopes: Scopes,
    pub(crate) visibility: Vec<VisibilityLevel>,
    /// `--exported`.
    pub(crate) exported: bool,
    /// `Some(false)`: `--exclude-tests`; `Some(true)`: `--tests-only`.
    pub(crate) tests: Option<bool>,
    pub(crate) undocumented: bool,
    pub(crate) deprecated: bool,
    pub(crate) is_async: bool,
    pub(crate) returns: Option<String>,
    pub(crate) param_type: Option<String>,
    pub(crate) min_params: Option<u32>,
    /// `--in`.
    pub(crate) containers: Vec<NamePattern>,
    /// Positional name filters and `--name`.
    pub(crate) names: Vec<NamePattern>,
    /// `--exclude-symbols`.
    pub(crate) excluded_names: Vec<NamePattern>,
}

/// Which scopes a run keeps.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) enum Scopes {
    /// The command's default.
    #[default]
    Default,
    /// `--all-scopes`.
    All,
    /// `--scope`.
    Only(Vec<SymbolScope>),
}

impl Selection {
    /// Resolves `args`, the positional name tokens and `--exclude-symbols`.
    pub(crate) fn new(
        args: &SelectionArgs,
        positional_names: &[String],
        excluded: &[String],
        exported: bool,
    ) -> Self {
        let targets = &args.targets;
        Self {
            kinds: args.kinds.iter().copied().map(SymbolKindV2::from).collect(),
            scopes: if args.all_scopes {
                Scopes::All
            } else if args.scopes.is_empty() {
                Scopes::Default
            } else {
                Scopes::Only(args.scopes.iter().copied().map(SymbolScope::from).collect())
            },
            visibility: args.visibility.iter().copied().map(VisibilityLevel::from).collect(),
            exported,
            tests: if args.exclude_tests {
                Some(false)
            } else if args.tests_only {
                Some(true)
            } else {
                None
            },
            undocumented: args.undocumented,
            deprecated: args.deprecated,
            is_async: args.is_async,
            returns: args.returns.clone(),
            param_type: args.param_type.clone(),
            min_params: args.min_params,
            containers: args.containers.iter().map(|token| targets.pattern(token)).collect(),
            names: positional_names
                .iter()
                .chain(&targets.names)
                .map(|token| targets.pattern(token))
                .collect(),
            excluded_names: excluded_patterns(excluded),
        }
    }

    /// `command_kinds` (`None`: every kind) narrowed by `--kind`; `None`
    /// keeps every kind.
    pub(crate) fn kinds_within(&self, command_kinds: Option<&[SymbolKindV2]>) -> Option<Vec<SymbolKindV2>> {
        match (command_kinds, self.kinds.is_empty()) {
            (None, true) => None,
            (None, false) => Some(self.kinds.clone()),
            (Some(command), true) => Some(command.to_vec()),
            (Some(command), false) => Some(
                command
                    .iter()
                    .copied()
                    .filter(|kind| self.kinds.contains(kind))
                    .collect(),
            ),
        }
    }

    /// The scopes kept, given the command's default (`None`: every scope).
    pub(crate) fn scopes_within(&self, default: Option<&[SymbolScope]>) -> Option<Vec<SymbolScope>> {
        match &self.scopes {
            Scopes::Default => default.map(<[SymbolScope]>::to_vec),
            Scopes::All => None,
            Scopes::Only(scopes) => Some(scopes.clone()),
        }
    }

    /// Adds the predicates that select a declaration by what it is rather
    /// than by its signature: visibility, export, test, docs and deprecation
    /// flags and the name patterns. Kinds, scopes, `--in` and the signature
    /// shape are left to the caller.
    pub(crate) fn identity_predicates(&self, mut query: SymbolQuery) -> SymbolQuery {
        for level in &self.visibility {
            query = query.visibility(*level);
        }
        query = query
            .exported(self.exported)
            .undocumented(self.undocumented)
            .deprecated(self.deprecated);
        query = match self.tests {
            Some(false) => query.exclude_tests(true),
            Some(true) => query.tests_only(true),
            None => query,
        };
        for pattern in &self.names {
            query = query.name(pattern.clone());
        }
        for pattern in &self.excluded_names {
            query = query.exclude_name(pattern.clone());
        }
        query
    }

    /// Adds the signature-shape predicates (`--async`, `--returns`,
    /// `--param-type`, `--min-params`), which keep only function-like kinds.
    pub(crate) fn shape_predicates(&self, mut query: SymbolQuery) -> SymbolQuery {
        query = query.is_async(self.is_async);
        if let Some(pattern) = &self.returns {
            query = query.returns(pattern.clone());
        }
        if let Some(pattern) = &self.param_type {
            query = query.param_type(pattern.clone());
        }
        if let Some(minimum) = self.min_params {
            query = query.min_params(minimum);
        }
        query
    }

    /// Whether a signature-shape predicate is given.
    pub(crate) fn has_shape(&self) -> bool {
        self.is_async || self.returns.is_some() || self.param_type.is_some() || self.min_params.is_some()
    }
}

/// `--exclude-symbols` patterns; an empty value excludes nothing. A trailing
/// `!` makes one exact, as for an inclusion; `--exact` does not apply.
fn excluded_patterns(excluded: &[String]) -> Vec<NamePattern> {
    excluded
        .iter()
        .filter(|token| !token.is_empty())
        .map(|token| NamePattern::new(token))
        .collect()
}

/// The name selectors of `imports`: `--name` and `--exclude-symbols`.
#[derive(Debug, Clone, Default)]
pub(crate) struct ImportSelection {
    names: Vec<NamePattern>,
    excluded_names: Vec<NamePattern>,
}

impl ImportSelection {
    pub(crate) fn new(targets: &TargetArgs, excluded: &[String]) -> Self {
        Self {
            names: targets.names.iter().map(|token| targets.pattern(token)).collect(),
            excluded_names: excluded_patterns(excluded),
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.names.is_empty() && self.excluded_names.is_empty()
    }

    /// Whether `import` (from a file in `language`) is kept: a pattern
    /// matches the local name, the alias, or the original name's last
    /// segment; a qualified pattern matches the imported path instead: the
    /// original, the source, or the source joined with the original (else
    /// the name), since languages split a path between the two differently
    /// (`std::collections` + `HashMap`, `A.B` alone).
    pub(crate) fn matches(&self, import: &ImportRecord, language: ProgrammingLanguage) -> bool {
        let original = import.symbol.qualified_name.as_deref();
        let joined = import.source.as_deref().map(|source| {
            let separator = match language {
                ProgrammingLanguage::Rust | ProgrammingLanguage::Cpp => "::",
                ProgrammingLanguage::Php => "\\",
                _ => ".",
            };
            format!("{source}{separator}{}", original.unwrap_or(&import.name))
        });
        let matches = |pattern: &NamePattern| {
            let path_matches = if pattern.is_qualified_for(language) {
                [original, import.source.as_deref(), joined.as_deref()].into_iter().flatten().any(|path| pattern.matches(path))
            } else {
                original.is_some_and(|original| pattern.matches(last_segment(original)))
            };
            pattern.matches(&import.name) || import.alias.as_deref().is_some_and(|alias| pattern.matches(alias)) || path_matches
        };
        (self.names.is_empty() || self.names.iter().any(matches))
            && !self.excluded_names.iter().any(matches)
    }
}

/// The last segment of an import path (`std::collections::HashMap` →
/// `HashMap`, `os.path` → `path`, `App\Models\User` → `User`).
fn last_segment(path: &str) -> &str {
    ["::", ".", "\\", "/"]
        .iter()
        .filter_map(|separator| path.rfind(separator).map(|found| found + separator.len()))
        .max()
        .map_or(path, |start| &path[start..])
}

/// Splits `--file` values into walker inputs (globs, directories and
/// anything that is not an existing file, used like a positional file
/// FILTER) and explicit files, resolved through [`FileReference`] from
/// `cwd`.
///
/// ## Errors
///
/// A value that is not a valid file reference, or whose resolution needs
/// state that is missing (an unset environment variable, no home directory).
pub(crate) fn resolve_file_selectors(
    values: &[String],
    cwd: &Path,
) -> Result<(Vec<String>, Vec<PathBuf>), String> {
    let context = FileResolutionContext::new(cwd);
    let mut inputs = Vec::new();
    let mut explicit = Vec::new();
    for value in values {
        if is_glob(value) {
            inputs.push(value.clone());
            continue;
        }
        let resolved = FileReference::new(value)
            .and_then(|reference| reference.resolve_in_context(&context))
            .map_err(|error| format!("invalid `--file {value}`: {error}"))?;
        match resolved {
            Some(file) => explicit.push(file),
            None => inputs.push(value.clone()),
        }
    }
    Ok((inputs, explicit))
}

/// Whether a `--file` value is a glob. The `?` of a Windows verbatim prefix
/// (`\\?\C:\…`) is not a wildcard.
fn is_glob(value: &str) -> bool {
    let body = value.strip_prefix(r"\\?\").unwrap_or(value);
    body.contains(['*', '?', '['])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every `SymbolKindV2` has a `--kind` value spelled as the snake_case
    /// of its serialized name, so a new variant cannot be left out.
    #[test]
    fn every_kind_has_its_snake_case_value() {
        for kind in KindArg::value_variants() {
            let name = kind.to_possible_value().expect("value").get_name().to_string();
            let serialized = serde_json::to_value(SymbolKindV2::from(*kind)).expect("serialize");
            assert_eq!(name, snake_case(serialized.as_str().expect("a string")));
        }
        assert_eq!(KindArg::value_variants().len(), 19, "one value per SymbolKindV2 variant");
        assert!(KindArg::from_str("type_alias", false).is_ok());
        assert!(KindArg::from_str("type-alias", false).is_err());
    }

    #[test]
    fn every_visibility_level_has_its_snake_case_value() {
        for level in VisibilityArg::value_variants() {
            let name = level.to_possible_value().expect("value").get_name().to_string();
            let serialized = serde_json::to_value(VisibilityLevel::from(*level)).expect("serialize");
            assert_eq!(name, snake_case(serialized.as_str().expect("a string")));
        }
        assert_eq!(VisibilityArg::value_variants().len(), 10);
    }

    fn snake_case(pascal: &str) -> String {
        let mut out = String::new();
        for (index, character) in pascal.chars().enumerate() {
            if character.is_uppercase() && index > 0 {
                out.push('_');
            }
            out.extend(character.to_lowercase());
        }
        out
    }

    #[test]
    fn kind_and_scope_selections_intersect_and_replace() {
        let mut selection = Selection::default();
        let functions = [SymbolKindV2::Function, SymbolKindV2::Method, SymbolKindV2::Constructor];
        assert_eq!(selection.kinds_within(None), None);
        assert_eq!(selection.kinds_within(Some(&functions)), Some(functions.to_vec()));
        selection.kinds = vec![SymbolKindV2::Method, SymbolKindV2::Trait];
        assert_eq!(selection.kinds_within(Some(&functions)), Some(vec![SymbolKindV2::Method]));
        assert_eq!(
            selection.kinds_within(None),
            Some(vec![SymbolKindV2::Method, SymbolKindV2::Trait])
        );

        let default = [SymbolScope::Item, SymbolScope::Member];
        assert_eq!(selection.scopes_within(Some(&default)), Some(default.to_vec()));
        assert_eq!(selection.scopes_within(None), None);
        selection.scopes = Scopes::All;
        assert_eq!(selection.scopes_within(Some(&default)), None);
        selection.scopes = Scopes::Only(vec![SymbolScope::Local]);
        assert_eq!(selection.scopes_within(Some(&default)), Some(vec![SymbolScope::Local]));
    }

    #[test]
    fn file_values_are_globs_only_with_a_wildcard() {
        assert!(is_glob("src/**/*.rs"));
        assert!(is_glob("src/?.rs"));
        assert!(is_glob("src/[ab].rs"));
        assert!(!is_glob("src/lib.rs"));
        assert!(!is_glob("~/notes/x.rs"));
        assert!(!is_glob(r"\\?\C:\repo\src\lib.rs"), "a verbatim prefix is not a wildcard");
        assert!(is_glob(r"\\?\C:\repo\*.rs"));
    }

    #[test]
    fn import_paths_end_in_their_last_segment() {
        assert_eq!(last_segment("std::collections::HashMap"), "HashMap");
        assert_eq!(last_segment("os.path"), "path");
        assert_eq!(last_segment("App\\Models\\User"), "User");
        assert_eq!(last_segment("join"), "join");
    }
}
