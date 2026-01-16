
/// A **TreePackage** is the primary primitive you will want to use
/// when evaluating a full "package" or "repo" of source code files.
///
/// Unlike the `TreeFile` -- whose focus is purely on a single file --
/// this struct is
pub struct TreePackage {
    /// the root directory for the source package
    pub root_dir: String,

    /// the programming language used in this package
    ///
    /// > **Note:** a `TreePackage` can only have **one**
    /// > programming language.
    pub language: ProgrammingLanguage,

    /// if the language being evaluated then
    /// it will be _cached_ here on first call to `modules()`
    /// function.
    modules: Option<Vec<String>>,

    pub source_files: Vec<String>,
    pub doc_files: Vec<String>
}



impl TreePackage {
    /// Creates a new `TreePackage` when passed a valid directory
    /// which resides in a git repo.
    ///
    /// A successful result when:
    ///
    /// 1. the directory is valid, and is part of a git repo
    /// 2. the root of the "package" (in a monorepo) or the root
    ///    of the repo (in a non-monorepo) is identified
    /// 3. the primary programming language file is identified
    /// 4. all source files for the package are identified
    /// 5. all markdown documents in the package are found
    ///
    /// These steps will be done in large part by the `sniff` library crate
    /// in this monorepo.
    pub fn new<T: Into<String>(dir: T) -> Result<TreePackage, Error> {
      todo!()
    }

    pub fn modules() -> Option<Vec<String>> {
      todo!()
    }
}
