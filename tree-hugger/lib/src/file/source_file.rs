//! A source file read into memory but not yet parsed.

use std::path::{Path, PathBuf};

use biscuit_hash::xx_hash;
use tree_sitter::Parser;

use crate::analysis::counters::count_parser_construction;
use crate::error::TreeHuggerError;
use crate::file::query::{SymbolQuery, SymbolQueryResult};
use crate::file::tree_file::TreeFile;
use crate::shared::ProgrammingLanguage;

/// A file's text, language and content hash, read once.
///
/// [`hash`](Self::hash) describes exactly the text [`parse`](Self::parse)
/// parses: the file is never read again, so an edit made after
/// [`read`](Self::read) cannot be analyzed under the earlier hash.
#[derive(Debug, Clone)]
pub struct SourceFile {
    path: PathBuf,
    language: ProgrammingLanguage,
    hash: String,
    source: String,
}

impl SourceFile {
    /// Reads `path` and detects its language from the path.
    ///
    /// ## Errors
    ///
    /// Returns [`TreeHuggerError::Io`] when the file cannot be read or is not
    /// UTF-8, and [`TreeHuggerError::UnsupportedLanguage`] when no language
    /// matches the path.
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
        let path = path.as_ref().to_path_buf();
        let source = std::fs::read_to_string(&path).map_err(|source| TreeHuggerError::Io {
            path: path.clone(),
            source,
        })?;
        let language = match language {
            Some(language) => language,
            None => ProgrammingLanguage::from_path(&path)
                .ok_or_else(|| TreeHuggerError::UnsupportedLanguage { path: path.clone() })?,
        };
        let hash = format!("{:x}", xx_hash(&source));

        Ok(Self {
            path,
            language,
            hash,
            source,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn language(&self) -> ProgrammingLanguage {
        self.language
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

        Ok(TreeFile::from_parts(
            self.path,
            self.language,
            self.hash,
            self.source,
            tree,
            grammar,
            grammar_id,
        ))
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
}
