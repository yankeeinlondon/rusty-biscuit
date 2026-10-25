//! What deletion says while it waits, and who it names.
//!
//! The engine asks [`Holders`] only to explain a wait: once when it starts
//! and again when the set of blocked entries changes, never on a timer. The
//! answer only explains the wait; it never ends it. A program that no longer
//! appears is not proof that Windows has released its file, so the wait ends
//! only when the entry deletes.

use std::fmt;
use std::path::{Path, PathBuf};

use sniff::filesystem::query::{Access, Outcome};

use crate::remove::usage::UsageSource;

/// Why an entry is waited on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitCause {
    /// Another program has it open (a sharing violation), or, on Windows, is
    /// running from it.
    InUse,
    /// A directory that filled up again every time its contents were
    /// deleted.
    Refilling,
}

/// One program using a blocked entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Holder {
    pub pid: u32,
    /// The program's name, when the OS gave one.
    pub name: Option<String>,
    /// Writing was observed, not just an open handle.
    pub writing: bool,
    /// Where under the deleted tree it was seen.
    pub paths: Vec<PathBuf>,
}

/// The programs using the blocked entries, as far as the query could tell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HolderReport {
    /// Every program the query found; empty when it found none.
    Found(Vec<Holder>),
    /// The query could not finish, or saw only part of the host.
    Unknown,
}

impl HolderReport {
    /// Whether a holder was seen at `path` or below it.
    pub fn holds(&self, path: &Path) -> bool {
        match self {
            Self::Found(holders) => holders.iter().any(|holder| holder.paths.iter().any(|seen| seen.starts_with(path))),
            Self::Unknown => false,
        }
    }
}

/// Names the programs using the blocked entries under `root`.
pub trait Holders: Sync {
    fn query(&self, root: &Path, blocked: &[PathBuf]) -> HolderReport;
}

/// [`Holders`] through the usage check's [`UsageSource`]: one Sniff query of
/// the deleted tree, as it is now. This process is never named.
pub struct UsageHolders<'a>(pub &'a dyn UsageSource);

impl Holders for UsageHolders<'_> {
    fn query(&self, root: &Path, _blocked: &[PathBuf]) -> HolderReport {
        let Ok(report) = self.0.query(root) else {
            return HolderReport::Unknown;
        };
        let own = std::process::id();
        let holders: Vec<Holder> = report
            .processes
            .iter()
            .filter(|record| record.pid != own && !record.evidence.is_empty())
            .map(|record| Holder {
                pid: record.pid,
                name: record.name.as_ref().map(|name| name.display().into_owned()),
                writing: record.evidence.iter().any(|evidence| matches!(evidence.access, Some(Access { write: true, .. }))),
                paths: record
                    .evidence
                    .iter()
                    .flat_map(|evidence| evidence.matched_paths.iter().map(|path| path.as_path().to_path_buf()))
                    .collect(),
            })
            .collect();
        if holders.is_empty() && (report.outcome != Outcome::Usable || report.budget_exhausted) {
            return HolderReport::Unknown;
        }
        HolderReport::Found(holders)
    }
}

/// A wait in progress: what is blocked, why, and by whom. Deletion reports
/// one when it starts waiting and again only when it changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wait {
    pub cause: WaitCause,
    /// The blocked entries, sorted; never empty.
    pub paths: Vec<PathBuf>,
    pub holders: HolderReport,
}

impl fmt::Display for Wait {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let first = self.paths.first().map(|path| path.display().to_string()).unwrap_or_default();
        let more = match self.paths.len() {
            0 | 1 => String::new(),
            n => format!(" and {} more", n - 1),
        };
        let blocked = match self.cause {
            WaitCause::InUse => "is in use",
            WaitCause::Refilling => "keeps filling up again",
        };
        let who = match &self.holders {
            HolderReport::Found(holders) if !holders.is_empty() => {
                holders.iter().map(describe).collect::<Vec<_>>().join("; ")
            }
            _ => "the program using it could not be identified".to_string(),
        };
        write!(f, "{first}{more} {blocked}: {who}. Waiting until it is released (Ctrl-C to stop).")
    }
}

/// An open handle is not proof of writing, so only observed writing says so.
fn describe(holder: &Holder) -> String {
    let name = holder.name.as_deref().unwrap_or("a program");
    let doing = if holder.writing { "is writing to it" } else { "has a file open" };
    format!("{name} (pid {}) {doing}", holder.pid)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn holder(name: Option<&str>, writing: bool) -> Holder {
        Holder { pid: 4123, name: name.map(str::to_string), writing, paths: vec![PathBuf::from("/wt/target/x")] }
    }

    #[test]
    fn a_wait_names_the_path_and_says_has_a_file_open_unless_writing_was_seen() {
        let wait = Wait {
            cause: WaitCause::InUse,
            paths: vec![PathBuf::from("/wt/target/x")],
            holders: HolderReport::Found(vec![holder(Some("cargo"), false)]),
        };
        assert_eq!(
            wait.to_string(),
            "/wt/target/x is in use: cargo (pid 4123) has a file open. Waiting until it is released (Ctrl-C to stop)."
        );

        let writing = Wait { holders: HolderReport::Found(vec![holder(Some("cargo"), true)]), ..wait.clone() };
        assert!(writing.to_string().contains("cargo (pid 4123) is writing to it"), "{writing}");
    }

    #[test]
    fn an_unidentified_holder_is_said_plainly_and_the_wait_still_reads_as_a_wait() {
        for holders in [HolderReport::Unknown, HolderReport::Found(Vec::new())] {
            let wait = Wait { cause: WaitCause::Refilling, paths: vec![PathBuf::from("/wt/out"), PathBuf::from("/wt/log")], holders };
            assert_eq!(
                wait.to_string(),
                "/wt/out and 1 more keeps filling up again: the program using it could not be identified. Waiting until it is released (Ctrl-C to stop)."
            );
        }
        let unnamed = Wait { cause: WaitCause::InUse, paths: vec![PathBuf::from("/wt/a")], holders: HolderReport::Found(vec![holder(None, false)]) };
        assert!(unnamed.to_string().contains("a program (pid 4123) has a file open"), "{unnamed}");
    }

    #[test]
    fn a_holder_counts_for_its_own_path_and_every_directory_above_it_only() {
        let report = HolderReport::Found(vec![holder(Some("cargo"), false)]);
        assert!(report.holds(Path::new("/wt/target/x")));
        assert!(report.holds(Path::new("/wt/target")));
        assert!(!report.holds(Path::new("/wt/target/y")));
        assert!(!HolderReport::Unknown.holds(Path::new("/wt")));
    }
}
