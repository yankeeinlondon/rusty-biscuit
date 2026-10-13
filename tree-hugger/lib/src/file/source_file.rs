//! A source file read into memory but not yet parsed.

use std::path::{Path, PathBuf};

use biscuit_file::canonicalize_simplified;
use biscuit_hash::xx_hash;
use tree_sitter::Parser;

use crate::analysis::counters::count_parser_construction;
use crate::error::TreeHuggerError;
use crate::file::query::{SymbolQuery, SymbolQueryResult};
use crate::file::tree_file::TreeFile;
use crate::shared::ProgrammingLanguage;

/// The one path every spelling of an existing file analyzes under: absolute,
/// with `.`/`..` and every symlink resolved, and on Windows in its legacy
/// (non-`\\?\`) spelling.
///
/// A symlink and its target therefore share one identity. A path that cannot
/// be resolved (it does not exist, or a component is unreadable) falls back
/// to its absolute spelling against the working directory, unresolved.
/// `docs/cache-design/entry-format.md#file-identity` records the policy.
pub fn file_identity(path: &Path) -> PathBuf {
    canonicalize_simplified(path)
        .or_else(|_| std::path::absolute(path))
        .unwrap_or_else(|_| path.to_path_buf())
}

/// A file's text, language and content hash, read once.
///
/// [`hash`](Self::hash) describes exactly the text [`parse`](Self::parse)
/// parses: the file is never read again, so an edit made after
/// [`read`](Self::read) cannot be analyzed under the earlier hash.
///
/// [`path`](Self::path) is the file's [`file_identity`], not the spelling it
/// was read through, so records, IDs and cache entries are the same for
/// every spelling of one file.
#[derive(Debug, Clone)]
pub struct SourceFile {
    path: PathBuf,
    language: ProgrammingLanguage,
    /// Whether `language` was given rather than detected from the path.
    forced_language: bool,
    hash: String,
    source: String,
    experimental_semantics: bool,
}

impl SourceFile {
    /// Reads `path` and detects its language from its [`file_identity`].
    ///
    /// ## Errors
    ///
    /// Returns [`TreeHuggerError::Io`] when the file cannot be read or is not
    /// UTF-8, and [`TreeHuggerError::UnsupportedLanguage`] when no language
    /// matches the path. Both name `path` as given.
    pub fn read<P: AsRef<Path>>(path: P) -> Result<Self, TreeHuggerError> {
        Self::read_with_language(path, None)
    }

    /// Reads `path`, using `language` instead of detection when given.
    ///
    /// ## Errors
    ///
    /// As [`read`](Self::read); a forced language never fails detection.
    pub fn read_with_language<P: AsRef<Path>>(
        path: P,
        language: Option<ProgrammingLanguage>,
    ) -> Result<Self, TreeHuggerError> {
        let supplied = path.as_ref();
        let source = std::fs::read_to_string(supplied).map_err(|source| TreeHuggerError::Io {
            path: supplied.to_path_buf(),
            source,
        })?;
        Self::from_read(supplied, language, source)
    }

    /// As [`read_with_language`](Self::read_with_language), for a path that
    /// is already its own [`file_identity`], such as every path
    /// [`collect_files`](crate::scanner::collect_files) returns. It becomes
    /// [`path`](Self::path) verbatim, without resolving it a second time.
    ///
    /// Any other spelling would key records, IDs and cache entries under
    /// that spelling; give arbitrary paths to `read_with_language`.
    ///
    /// ## Errors
    ///
    /// As [`read_with_language`](Self::read_with_language).
    pub fn read_identified(
        identity: &Path,
        language: Option<ProgrammingLanguage>,
    ) -> Result<Self, TreeHuggerError> {
        let source = std::fs::read_to_string(identity).map_err(|source| TreeHuggerError::Io {
            path: identity.to_path_buf(),
            source,
        })?;
        Self::from_identity(identity.to_path_buf(), identity, language, source)
    }

    /// The file [`read_with_language`](Self::read_with_language) builds from
    /// `source`, text the caller already read from `supplied`.
    pub(crate) fn from_read(
        supplied: &Path,
        language: Option<ProgrammingLanguage>,
        source: String,
    ) -> Result<Self, TreeHuggerError> {
        // Resolved after the read so a missing file keeps its I/O error; the
        // bytes already read are the ones hashed and parsed.
        Self::from_identity(file_identity(supplied), supplied, language, source)
    }

    /// The file whose identity `path` is, from `source` already read through
    /// `supplied`; errors name `supplied`.
    pub(crate) fn from_identity(
        path: PathBuf,
        supplied: &Path,
        language: Option<ProgrammingLanguage>,
        source: String,
    ) -> Result<Self, TreeHuggerError> {
        let forced_language = language.is_some();
        let language = match language {
            Some(language) => language,
            None => ProgrammingLanguage::from_path(&path).ok_or_else(|| {
                TreeHuggerError::UnsupportedLanguage {
                    path: supplied.to_path_buf(),
                }
            })?,
        };
        let hash = format!("{:x}", xx_hash(&source));

        Ok(Self {
            path,
            language,
            forced_language,
            hash,
            source,
            experimental_semantics: false,
        })
    }

    /// Enables the experimental semantic lint rules for the diagnostics this
    /// file's analysis produces (see [`TreeFile::experimental_semantics`]).
    pub fn experimental_semantics(mut self, enabled: bool) -> Self {
        self.experimental_semantics = enabled;
        self
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn language(&self) -> ProgrammingLanguage {
        self.language
    }

    /// The language when it was given to
    /// [`read_with_language`](Self::read_with_language), `None` when it was
    /// detected from the path.
    pub fn forced_language(&self) -> Option<ProgrammingLanguage> {
        self.forced_language.then_some(self.language)
    }

    pub fn uses_experimental_semantics(&self) -> bool {
        self.experimental_semantics
    }

    /// The xxHash of the text, as [`TreeFile::hash`] reports it.
    pub fn hash(&self) -> &str {
        &self.hash
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    /// Parses the text that was read.
    ///
    /// ## Errors
    ///
    /// Returns [`TreeHuggerError::UnsupportedLanguage`] when the grammar
    /// cannot be loaded and [`TreeHuggerError::ParseFailed`] when parsing
    /// does not produce a tree.
    pub fn parse(self) -> Result<TreeFile, TreeHuggerError> {
        let (grammar, grammar_id) = self.language.grammar_for_path(&self.path);

        count_parser_construction();
        let mut parser = Parser::new();
        parser
            .set_language(&grammar)
            .map_err(|_| TreeHuggerError::UnsupportedLanguage {
                path: self.path.clone(),
            })?;
        let tree = parser
            .parse(&self.source, None)
            .ok_or_else(|| TreeHuggerError::ParseFailed {
                path: self.path.clone(),
            })?;

        let mut tree_file = TreeFile::from_parts(
            self.path,
            self.language,
            self.hash,
            self.source,
            tree,
            grammar,
            grammar_id,
        );
        tree_file.experimental_semantics = self.experimental_semantics;
        Ok(tree_file)
    }

    /// Parses the text and runs `query` over it, through the same planning
    /// path as [`TreeFile::query`].
    ///
    /// ## Errors
    ///
    /// As [`parse`](Self::parse), and any error of the analysis passes.
    pub fn query(self, query: &SymbolQuery) -> Result<SymbolQueryResult, TreeHuggerError> {
        self.parse()?.query(query)
    }

    /// As [`query`](Self::query), and gives back the source, for a caller
    /// that decides from the result whether to keep these exact bytes. The
    /// text is moved through the parse, never copied.
    ///
    /// ## Errors
    ///
    /// As [`query`](Self::query); the source is dropped with the error.
    pub fn query_retaining(self, query: &SymbolQuery) -> Result<(SymbolQueryResult, SourceFile), TreeHuggerError> {
        self.run_retaining(|file| file.query(query))
    }

    /// As [`query_retaining`](Self::query_retaining), but unfinished: in-file
    /// qualified names and no predicate applied, as the cache stores it.
    pub(crate) fn analyze_retaining(
        self,
        query: &SymbolQuery,
    ) -> Result<(SymbolQueryResult, SourceFile), TreeHuggerError> {
        self.run_retaining(|file| crate::analysis::run_query(file, query))
    }

    fn run_retaining(
        self,
        run: impl FnOnce(&TreeFile) -> Result<SymbolQueryResult, TreeHuggerError>,
    ) -> Result<(SymbolQueryResult, SourceFile), TreeHuggerError> {
        let (language, forced_language, experimental_semantics) =
            (self.language, self.forced_language, self.experimental_semantics);
        let file = self.parse()?;
        let result = run(&file)?;
        let (path, hash, source) = file.into_source();
        Ok((
            result,
            SourceFile {
                path,
                language,
                forced_language,
                hash,
                source,
                experimental_semantics,
            },
        ))
    }
}
