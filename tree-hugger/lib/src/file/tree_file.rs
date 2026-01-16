

pub struct TreeFile {
    pub file: String,
    content: Option<String>,
    hash:

    /// a cache of symbols in the file which is populated
    /// lazily
    symbols: Option<Vec<Symbol>>,

}

impl TreeFile {

    pub new<T: Into<String>>(file: T) -> Result<TreeFile, Error> {
        todo!()
    }

    /// provides the list of symbols imported by this file
    pub imported_symbols() -> Vec<ImportSymbol> {
        todo!()
    }

    /// provides the list of symbols _exported_ by this file
    pub exported_symbols() -> Vec<Symbol> {
        todo!()
    }

    /// provides the list of symbols _re-exported_ by this file
    pub reexported_symbols() -> Vec<ImportSymbol> {
        todo!()
    }

    /// local symbols defined in this file but _not_ exported
    pub local_symbols() -> Vec<Symbol> {
        todo!()
    }

    pub lint_diagnostics() -> Vec<LintDiagnostic> {
        todo!()
    }

    pub syntax_diagnostics() -> Vec<SyntaxDiagnostic> {
        todo!()
    }

    /// dead code blocks which are **unreachable**
    pub dead_code() -> Vec<CodeBlock> {
        todo!()
    }


}

