//! Planned, reported publication of one output root.
//!
//! Every output root (the Rust client, the OpenAPI documents, the Postman
//! collections) is published in two steps. Preparation finishes validation,
//! assembly, formatting, and every ownership check in memory and produces a
//! [`MutationPlan`]; nothing touches the filesystem until
//! [`MutationPlan::publish`] applies it. A failure during publication is
//! therefore an I/O failure, and its error lists the mutations already applied
//! and those never attempted.
//!
//! Each write is atomic for its own file ([`write_atomic`]); a plan as a whole
//! is not a transaction. Writes come first, in the order the preparer chose,
//! and deletions of obsolete paths follow them, so a replacement always exists
//! before the path it replaces is removed.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::errors::GeneratorError;
use crate::output::write::write_atomic;

/// One change a plan makes to the filesystem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mutation {
    /// Writes `content` to `path`, creating missing parent directories.
    Write { path: PathBuf, content: String },
    /// Removes a file the generator provably wrote.
    RemoveFile { path: PathBuf },
    /// Removes a directory tree the generator provably wrote, without
    /// following symbolic links.
    RemoveDir { path: PathBuf },
}

impl Mutation {
    /// The path this mutation changes.
    pub fn path(&self) -> &Path {
        match self {
            Self::Write { path, .. } | Self::RemoveFile { path } | Self::RemoveDir { path } => path,
        }
    }

    fn apply(&self) -> Result<(), GeneratorError> {
        let removal = |result: std::io::Result<()>| {
            result.map_err(|source| GeneratorError::WriteError {
                path: self.path().display().to_string(),
                source,
            })
        };
        match self {
            Self::Write { path, content } => write_atomic(path, content),
            Self::RemoveFile { path } => removal(fs::remove_file(path)),
            Self::RemoveDir { path } => removal(fs::remove_dir_all(path)),
        }
    }
}

impl fmt::Display for Mutation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Write { path, .. } => write!(f, "write {}", path.display()),
            Self::RemoveFile { path } => write!(f, "delete {}", path.display()),
            Self::RemoveDir { path } => write!(f, "delete directory {}", path.display()),
        }
    }
}

/// Every mutation one output root needs, checked and ready to apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MutationPlan {
    root: PathBuf,
    mutations: Vec<Mutation>,
}

impl MutationPlan {
    /// Joins planned writes and the deletions that follow them.
    pub(crate) fn new(root: &Path, writes: Vec<Mutation>, deletions: Vec<Mutation>) -> Self {
        let mut mutations = writes;
        mutations.extend(deletions);
        Self {
            root: root.to_path_buf(),
            mutations,
        }
    }

    /// The output root this plan publishes.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Every mutation, in the order [`publish`](Self::publish) applies them.
    pub fn mutations(&self) -> &[Mutation] {
        &self.mutations
    }

    /// Prints what a dry run would do, one line per mutation. With
    /// `contents`, each planned file's content follows its line.
    pub fn print_dry_run(&self, contents: bool) {
        for mutation in &self.mutations {
            match mutation {
                Mutation::Write { path, content } if contents => {
                    println!("=== {} ===\n{}\n", self.display_path(path), content);
                }
                other => println!("would {}", other),
            }
        }
    }

    fn display_path(&self, path: &Path) -> String {
        path.strip_prefix(&self.root)
            .unwrap_or(path)
            .display()
            .to_string()
    }

    /// Applies every mutation in order and stops at the first failure.
    ///
    /// ## Errors
    ///
    /// [`GeneratorError::PublicationFailed`] naming the failed mutation, the
    /// ones already applied, and the ones never attempted.
    pub fn publish(&self) -> Result<PublicationReport, GeneratorError> {
        for (index, mutation) in self.mutations.iter().enumerate() {
            if let Err(source) = mutation.apply() {
                return Err(GeneratorError::PublicationFailed {
                    root: self.root.display().to_string(),
                    failed: mutation.to_string(),
                    completed: self.mutations[..index].iter().map(ToString::to_string).collect(),
                    pending: self.mutations[index + 1..]
                        .iter()
                        .map(ToString::to_string)
                        .collect(),
                    source: Box::new(source),
                });
            }
        }
        Ok(PublicationReport {
            root: self.root.clone(),
            applied: self.mutations.clone(),
        })
    }
}

/// What a successful publication changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicationReport {
    /// The published output root.
    pub root: PathBuf,
    /// Every mutation applied, in order.
    pub applied: Vec<Mutation>,
}

impl PublicationReport {
    /// Number of files written.
    pub fn written(&self) -> usize {
        self.applied
            .iter()
            .filter(|m| matches!(m, Mutation::Write { .. }))
            .count()
    }

    /// Number of files and directories deleted.
    pub fn deleted(&self) -> usize {
        self.applied.len() - self.written()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_precede_deletions_and_failure_lists_completed_and_pending() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        fs::write(root.join("old.rs"), "old").unwrap();
        // A directory where a file must go makes the rename fail on every OS.
        fs::create_dir(root.join("b.rs")).unwrap();

        let plan = MutationPlan::new(
            root,
            vec![
                Mutation::Write { path: root.join("a.rs"), content: "a".into() },
                Mutation::Write { path: root.join("b.rs"), content: "b".into() },
                Mutation::Write { path: root.join("c.rs"), content: "c".into() },
            ],
            vec![Mutation::RemoveFile { path: root.join("old.rs") }],
        );
        let err = plan.publish().unwrap_err();
        let GeneratorError::PublicationFailed { failed, completed, pending, .. } = &err else {
            panic!("expected PublicationFailed, got {err:?}");
        };
        assert_eq!(failed, &format!("write {}", root.join("b.rs").display()));
        assert_eq!(completed, &[format!("write {}", root.join("a.rs").display())]);
        assert_eq!(
            pending,
            &[
                format!("write {}", root.join("c.rs").display()),
                format!("delete {}", root.join("old.rs").display()),
            ]
        );
        assert_eq!(fs::read_to_string(root.join("a.rs")).unwrap(), "a");
        assert!(!root.join("c.rs").exists());
        assert!(root.join("old.rs").exists(), "a deletion after the failure never ran");
        let rendered = err.to_string();
        assert!(rendered.contains("completed (1)") && rendered.contains("not done (3)"), "{rendered}");
    }

    #[test]
    fn successful_publication_reports_every_mutation() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("nested");
        let plan = MutationPlan::new(
            &root,
            vec![Mutation::Write { path: root.join("x/y.rs"), content: "y".into() }],
            vec![],
        );
        let report = plan.publish().unwrap();
        assert_eq!((report.written(), report.deleted()), (1, 0));
        assert_eq!(fs::read_to_string(root.join("x/y.rs")).unwrap(), "y");
    }
}
