//! The provider-argument tail a caller forwards to the underlying agent CLI.
//!
//! One typed descriptor serves composition (`compose`, `inline-compose`,
//! `sequence`) and the direct provider wrappers. It keeps the forwarded tokens
//! in their original order together with the position of an authored `--`, so
//! an implicit prefix (`-c x=y`) and an opaque suffix (`--native z`) stay
//! distinguishable after the CLI has consumed the separator.
//!
//! Tokens are caller input and may hold secrets. Neither type here derives
//! `Debug`: both print counts only, so a `?request` trace cannot leak a value.
//! The raw tokens are reachable through [`ProviderTail::launch_args`], which
//! exists for building the child argv.

use std::collections::HashSet;
use std::fmt;
use std::ops::Range;
use std::sync::{Arc, Mutex};

use crate::provider::Provider;

/// The ordered provider-argument tail, with the authored `--` boundary.
///
/// `boundary` is the index in [`launch_args`](Self::launch_args) where the
/// opaque suffix starts:
///
/// | `boundary()` | Meaning |
/// | --- | --- |
/// | `None` | no authored `--`; every token is implicit |
/// | `Some(0)` | fully explicit: everything came after `--` |
/// | `Some(len)` | an implicit prefix followed by an authored, empty suffix |
///
/// ## Examples
///
/// ```
/// use claudine::composition::ProviderTail;
///
/// // `-c x=y -- --native z`
/// let tail = ProviderTail::new(
///     vec!["-c".into(), "x=y".into()],
///     Some(vec!["--native".into(), "z".into()]),
/// );
/// assert_eq!(tail.implicit_args(), ["-c", "x=y"]);
/// assert_eq!(tail.opaque_args(), Some(&["--native".to_string(), "z".to_string()][..]));
/// assert_eq!(tail.boundary(), Some(2));
/// ```
#[derive(Clone, Default, PartialEq, Eq)]
pub struct ProviderTail {
    args: Vec<String>,
    boundary: Option<usize>,
    assignments: Vec<SwitchAssignment>,
}

/// Which implicit-prefix tokens one provider switch took as its values.
///
/// Indices refer to [`ProviderTail::launch_args`]. Composition's type-aware
/// ownership ([`super::ownership::own_arguments`]) records one per implicit
/// switch, so [`super::ownership::check_launch_tail`] can name the assignment
/// a resolved provider disagrees with. A direct wrapper's tail records none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwitchAssignment {
    /// Index of the switch token.
    pub switch: usize,
    /// Indices of the tokens the switch took as values (empty for none).
    pub values: Range<usize>,
}

impl ProviderTail {
    /// Build a tail from the implicit prefix and, when the caller wrote `--`,
    /// the opaque suffix that followed it (possibly empty).
    pub fn new(implicit: Vec<String>, opaque: Option<Vec<String>>) -> Self {
        let mut args = implicit;
        let boundary = opaque.map(|suffix| {
            let at = args.len();
            args.extend(suffix);
            at
        });
        Self {
            args,
            boundary,
            assignments: Vec::new(),
        }
    }

    /// The unredacted tokens, in order, for building the child argv.
    ///
    /// Every display surface must redact these first; this accessor is for
    /// launching only.
    pub fn launch_args(&self) -> &[String] {
        &self.args
    }

    /// Index where the opaque suffix starts; `None` without an authored `--`.
    pub fn boundary(&self) -> Option<usize> {
        self.boundary
    }

    /// The tokens before the authored `--` (all of them when there is none).
    pub fn implicit_args(&self) -> &[String] {
        &self.args[..self.boundary.unwrap_or(self.args.len())]
    }

    /// The tokens after the authored `--`, or `None` when there was no `--`.
    pub fn opaque_args(&self) -> Option<&[String]> {
        self.boundary.map(|at| &self.args[at..])
    }

    /// Switch/value assignments recorded for the implicit prefix.
    pub fn assignments(&self) -> &[SwitchAssignment] {
        &self.assignments
    }

    /// The same tail with the switch/value assignments ownership decided.
    pub(crate) fn with_assignments(mut self, assignments: Vec<SwitchAssignment>) -> Self {
        self.assignments = assignments;
        self
    }

    /// `true` when no token is forwarded, even if an empty `--` suffix was
    /// authored.
    pub fn is_empty(&self) -> bool {
        self.args.is_empty()
    }
}

impl fmt::Debug for ProviderTail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProviderTail")
            .field("len", &self.args.len())
            .field("boundary", &self.boundary)
            .field("assignments", &self.assignments.len())
            .finish_non_exhaustive()
    }
}

/// Command-scoped record of which forwarding notices were already shown.
///
/// The top-level command creates one; clones share it, so every composition
/// attempt, `sequence` task (parallel ones included), and retry of that
/// command announces a distinct `(provider, tail, boundary)` once. Separate
/// commands, and separate tests in one process, hold separate registries.
#[derive(Clone, Default)]
pub struct ProviderTailNotices {
    claimed: Arc<Mutex<HashSet<NoticeKey>>>,
}

#[derive(PartialEq, Eq, Hash)]
struct NoticeKey {
    provider: Provider,
    args: Vec<String>,
    boundary: Option<usize>,
}

impl ProviderTailNotices {
    /// Claim the notice for `provider` and `tail`.
    ///
    /// Returns `true` exactly once per distinct pair: the caller that gets
    /// `true` renders the notice. The lock is released before returning, so
    /// rendering never happens under it.
    pub fn claim(&self, provider: Provider, tail: &ProviderTail) -> bool {
        let key = NoticeKey {
            provider,
            args: tail.args.clone(),
            boundary: tail.boundary,
        };
        self.claimed
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(key)
    }
}

impl fmt::Debug for ProviderTailNotices {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let claimed = self
            .claimed
            .lock()
            .map_or(0, |claimed| claimed.len());
        f.debug_struct("ProviderTailNotices")
            .field("claimed", &claimed)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(tokens: &[&str]) -> Vec<String> {
        tokens.iter().map(|token| token.to_string()).collect()
    }

    #[test]
    fn boundary_records_every_shape() {
        let implicit = ProviderTail::new(strings(&["-c", "x=y"]), None);
        assert_eq!(implicit.boundary(), None);
        assert_eq!(implicit.implicit_args(), ["-c", "x=y"]);
        assert_eq!(implicit.opaque_args(), None);

        let explicit = ProviderTail::new(Vec::new(), Some(strings(&["-c", "value"])));
        assert_eq!(explicit.boundary(), Some(0));
        assert!(explicit.implicit_args().is_empty());
        assert_eq!(explicit.opaque_args().unwrap(), ["-c", "value"]);

        let empty_suffix = ProviderTail::new(strings(&["-c", "x"]), Some(Vec::new()));
        assert_eq!(empty_suffix.boundary(), Some(2));
        assert_eq!(empty_suffix.opaque_args().unwrap(), [] as [String; 0]);

        let mixed = ProviderTail::new(strings(&["-c", "x=y"]), Some(strings(&["--native", "z"])));
        assert_eq!(mixed.launch_args(), ["-c", "x=y", "--native", "z"]);
        assert_eq!(mixed.boundary(), Some(2));
        assert!(mixed.assignments().is_empty());
    }

    #[test]
    fn default_is_empty_with_no_boundary() {
        let tail = ProviderTail::default();
        assert!(tail.is_empty());
        assert_eq!(tail.boundary(), None);
        assert!(ProviderTail::new(Vec::new(), Some(Vec::new())).is_empty());
    }

    #[test]
    fn debug_never_prints_tokens() {
        let tail = ProviderTail::new(
            strings(&["--api-key", "sk-secret", "-csecret"]),
            Some(strings(&["--token=sk-other"])),
        );
        let rendered = format!("{tail:?} {tail:#?}");
        for token in ["sk-secret", "-csecret", "sk-other", "api-key", "token"] {
            assert!(!rendered.contains(token), "{token} leaked into {rendered}");
        }
        assert!(rendered.contains("len: 4"));

        let notices = ProviderTailNotices::default();
        assert!(notices.claim(Provider::Codex, &tail));
        let rendered = format!("{notices:?}");
        assert!(!rendered.contains("sk-secret"), "{rendered}");
        assert!(rendered.contains("claimed: 1"));
    }

    #[test]
    fn notices_claim_each_distinct_pair_once() {
        let notices = ProviderTailNotices::default();
        let tail = ProviderTail::new(strings(&["-c", "x"]), None);
        assert!(notices.claim(Provider::Codex, &tail));
        assert!(!notices.claim(Provider::Codex, &tail));
        // Same tokens, other provider: distinct.
        assert!(notices.claim(Provider::Claude, &tail));
        // Same provider, other tokens: distinct.
        assert!(notices.claim(Provider::Codex, &ProviderTail::new(strings(&["-c", "y"]), None)));
        // A clone shares the record.
        assert!(!notices.clone().claim(Provider::Codex, &tail));
    }

    #[test]
    fn boundary_is_part_of_the_notice_key() {
        let notices = ProviderTailNotices::default();
        let implicit = ProviderTail::new(strings(&["-c", "x"]), None);
        let mixed = ProviderTail::new(strings(&["-c"]), Some(strings(&["x"])));
        let explicit = ProviderTail::new(Vec::new(), Some(strings(&["-c", "x"])));
        assert_eq!(implicit.launch_args(), mixed.launch_args());
        assert!(notices.claim(Provider::Codex, &implicit));
        assert!(notices.claim(Provider::Codex, &mixed));
        assert!(notices.claim(Provider::Codex, &explicit));
    }

    #[test]
    fn separate_registries_do_not_share_claims() {
        let tail = ProviderTail::new(strings(&["-c", "x"]), None);
        let first_command = ProviderTailNotices::default();
        let second_command = ProviderTailNotices::default();
        assert!(first_command.claim(Provider::Codex, &tail));
        assert!(second_command.claim(Provider::Codex, &tail));
    }

    #[test]
    fn concurrent_claims_admit_exactly_one_winner() {
        let notices = ProviderTailNotices::default();
        let tail = ProviderTail::new(strings(&["-c", "x"]), None);
        let winners: usize = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..16)
                .map(|_| {
                    let notices = notices.clone();
                    let tail = tail.clone();
                    scope.spawn(move || usize::from(notices.claim(Provider::Codex, &tail)))
                })
                .collect();
            handles.into_iter().map(|handle| handle.join().unwrap()).sum()
        });
        assert_eq!(winners, 1);
    }
}
