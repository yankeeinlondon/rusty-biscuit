//! Human rendering of the library's lockfile observations: each workspace
//! layer's `lockfile` and the repository's `standalone_lockfiles`.
//!
//! This only projects what the library observed; nothing here reads or
//! reinterprets a lockfile.

use std::rc::Rc;

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::{RenderableTerminalContent, TerminalRenderable};
use biscuit_terminal::terminal::Terminal;
use sniff::filesystem::repo::{
    LockfileObservation, LockfileReason, LockfileStatus, RepoInfo, StandaloneLockfileTool,
};

use super::relative_path_between;
use super::repo::format_monorepo_label;

/// Render the "Lockfiles" section, or an empty string when the repository has
/// neither a workspace layer nor a standalone lockfile.
pub(super) fn render_lockfile_section(repo: &RepoInfo, term: &Terminal) -> String {
    let mut items: Vec<RenderableTerminalContent> = Vec::new();
    for layer in &repo.monorepo_layers {
        let label = format_monorepo_label(layer.authority, &layer.orchestrators);
        let root = relative_path_between(&repo.root, &layer.root);
        append_observation(&mut items, &label, &root, &layer.lockfile, term);
    }
    for entry in &repo.standalone_lockfiles {
        let root = if entry.root.is_empty() {
            "."
        } else {
            entry.root.as_str()
        };
        append_observation(
            &mut items,
            tool_label(entry.tool),
            root,
            &entry.observation,
            term,
        );
    }
    if items.is_empty() {
        return String::new();
    }

    let title = Prose::new("<b>Lockfiles</b>").render(term);
    let list = UnorderedList::from(items).with_indent_children(Some(4));
    format!("\n{title}\n\n{}\n", list.render(term))
}

/// One headline per observation, then its selected lockfiles and any
/// members only one side records, each on its own line: a legacy Gradle
/// group can list many files.
fn append_observation(
    items: &mut Vec<RenderableTerminalContent>,
    label: &str,
    root: &str,
    observation: &LockfileObservation,
    term: &Terminal,
) {
    let headline = format!(
        "<b>{}</b> <dim>({})</dim>: {} — {}",
        Prose::escape_text(label),
        Prose::escape_text(root),
        status_markup(observation.status),
        explanation(observation),
    );
    items.push(RenderableTerminalContent::String(
        Prose::new(headline).render(term),
    ));

    let details: Vec<String> = observation
        .paths
        .iter()
        .map(|path| format!("lockfile: {}", Prose::escape_text(path)))
        .chain(observation.missing.iter().map(|member| {
            format!(
                "<red>missing from the lockfile:</red> {}",
                Prose::escape_text(member)
            )
        }))
        .chain(observation.extra.iter().map(|member| {
            format!(
                "<red>only in the lockfile:</red> {}",
                Prose::escape_text(member)
            )
        }))
        .map(|detail| Prose::new(detail).render(term))
        .collect();
    if !details.is_empty() {
        items.push(RenderableTerminalContent::Component(Rc::new(
            UnorderedList::new(details),
        )));
    }
}

fn tool_label(tool: StandaloneLockfileTool) -> &'static str {
    match tool {
        StandaloneLockfileTool::Composer => "Composer",
        StandaloneLockfileTool::Pdm => "PDM",
        StandaloneLockfileTool::Poetry => "Poetry",
    }
}

fn status_markup(status: LockfileStatus) -> &'static str {
    match status {
        LockfileStatus::Match => "<green>match</green>",
        LockfileStatus::Mismatch => "<red>mismatch</red>",
        LockfileStatus::MembersPresent => "<green>members present</green>",
        LockfileStatus::MembersMissing => "<red>members missing</red>",
        LockfileStatus::Unverifiable => "<yellow>unverifiable</yellow>",
        LockfileStatus::Unreadable => "<red>unreadable</red>",
        LockfileStatus::Absent => "<dim>absent</dim>",
        LockfileStatus::NotApplicable => "<dim>not applicable</dim>",
        LockfileStatus::NotRequested => "<dim>not requested</dim>",
    }
}

/// A plain-language reading of the status, refined by its reason.
fn explanation(observation: &LockfileObservation) -> &'static str {
    use LockfileReason as Reason;
    use LockfileStatus as Status;

    match (observation.status, observation.reason) {
        (_, Some(Reason::RequestDisabled)) | (Status::NotRequested, _) => {
            "a lockfile exists, but this request did not compare it"
        }
        (_, Some(Reason::NoLockfileSource)) => {
            "this workspace tool writes no lockfile that records its members"
        }
        (_, Some(Reason::UnknownStandard)) => {
            "the workspace tool is not recognized, so no lockfile is known for it"
        }
        (_, Some(Reason::UnsupportedVersion)) => {
            "this lockfile format version is not supported, so members were not compared"
        }
        (_, Some(Reason::UnsupportedLayout)) => {
            "this workspace configuration is not one whose lockfile can be compared"
        }
        (_, Some(Reason::NoMembershipData)) => {
            "this lockfile does not record which packages are workspace members"
        }
        (_, Some(Reason::AmbiguousMembership)) => {
            "the lockfile's members could not each be tied to one package path"
        }
        (_, Some(Reason::IncompleteManifestDiscovery)) => {
            "the manifest's member list may be incomplete, so it was not compared"
        }
        (_, Some(Reason::InvalidMemberPath)) => {
            "a member path is absolute or cannot be represented, so it was not compared"
        }
        (_, Some(Reason::MetadataFailed)) => "the lockfile's file information could not be read",
        (_, Some(Reason::ReadFailed)) => "the lockfile could not be read",
        (_, Some(Reason::ParseFailed)) => "the lockfile is not valid for its format",
        (Status::MembersPresent, _) => {
            "every member is in the lockfile; it may list other local crates, so an exact match is not claimed"
        }
        (Status::MembersMissing, _) => "some members are not in the lockfile",
        (Status::Match, _) => "the lockfile's members match the manifest",
        (Status::Mismatch, _) => "the lockfile's members differ from the manifest",
        (Status::Absent, _) => "no lockfile was found",
        (Status::Unverifiable, _) => "the lockfile cannot establish the member list",
        (Status::Unreadable, _) => "the lockfile could not be read",
        (Status::NotApplicable, _) => "no lockfile applies to this workspace",
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::path::PathBuf;

    use biscuit_terminal::prelude::strip_escape_codes;
    use sniff::filesystem::repo::{
        MonorepoLayer, MonorepoStandard, PackageProvenance, StandaloneLockfileObservation,
    };

    use super::*;

    fn observation(
        status: LockfileStatus,
        paths: &[&str],
        reason: Option<LockfileReason>,
        extra: &[&str],
        missing: &[&str],
    ) -> LockfileObservation {
        let owned = |items: &[&str]| items.iter().map(|item| (*item).to_string()).collect();
        LockfileObservation {
            status,
            paths: owned(paths),
            reason,
            extra: owned(extra),
            missing: owned(missing),
        }
    }

    fn layer(root: &str, authority: MonorepoStandard, lockfile: LockfileObservation) -> MonorepoLayer {
        MonorepoLayer {
            root: PathBuf::from(root),
            authority,
            orchestrators: Vec::new(),
            provenance: PackageProvenance::Globbed,
            lockfile,
            root_is_package: false,
            packages: Vec::new(),
        }
    }

    /// Unstyled output with every whitespace run collapsed, so assertions do
    /// not depend on the terminal width.
    fn rendered(repo: &RepoInfo) -> String {
        strip_escape_codes(render_lockfile_section(repo, &Terminal::default()))
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn nothing_is_rendered_without_a_layer_or_standalone_lockfile() {
        assert_eq!(render_lockfile_section(&RepoInfo::default(), &Terminal::default()), "");
    }

    /// No CLI command displays a layer from a declining request, so this is
    /// the only coverage of `not_requested` in human output. The line states
    /// the fact; it names no flag, because a CLI hint belongs on stderr.
    #[test]
    fn not_requested_layers_and_standalone_lockfiles_say_the_request_skipped_them() {
        let repo = RepoInfo {
            root: PathBuf::from("/repo"),
            monorepo_layers: vec![layer(
                "/repo/web",
                MonorepoStandard::PnpmWorkspaces,
                observation(
                    LockfileStatus::NotRequested,
                    &["pnpm-lock.yaml"],
                    Some(LockfileReason::RequestDisabled),
                    &[],
                    &[],
                ),
            )],
            standalone_lockfiles: vec![StandaloneLockfileObservation {
                root: String::new(),
                tool: StandaloneLockfileTool::Poetry,
                observation: observation(
                    LockfileStatus::NotRequested,
                    &["poetry.lock"],
                    Some(LockfileReason::RequestDisabled),
                    &[],
                    &[],
                ),
            }],
            ..RepoInfo::default()
        };

        let text = rendered(&repo);
        for expected in [
            "Lockfiles - pnpm workspaces (web): not requested — a lockfile exists, but this request did not compare it - lockfile: pnpm-lock.yaml",
            "- Poetry (.): not requested — a lockfile exists, but this request did not compare it - lockfile: poetry.lock",
        ] {
            assert!(text.contains(expected), "missing `{expected}` in: {text}");
        }
        assert!(!text.contains("--"), "no flag hint on stdout: {text}");
    }

    #[test]
    fn a_mismatch_lists_each_missing_and_extra_member() {
        let repo = RepoInfo {
            root: PathBuf::from("/repo"),
            monorepo_layers: vec![layer(
                "/repo",
                MonorepoStandard::YarnWorkspaces,
                observation(
                    LockfileStatus::Mismatch,
                    &["yarn.lock"],
                    None,
                    &["packages/old"],
                    &["packages/a", "packages/b"],
                ),
            )],
            ..RepoInfo::default()
        };

        let text = rendered(&repo);
        let expected = "(.): mismatch — the lockfile's members differ from the manifest \
                        - lockfile: yarn.lock \
                        - missing from the lockfile: packages/a \
                        - missing from the lockfile: packages/b \
                        - only in the lockfile: packages/old";
        assert!(text.contains(expected), "missing `{expected}` in: {text}");
    }

    /// A legacy Gradle group selects many files; each gets its own line.
    #[test]
    fn a_multi_file_lockfile_group_lists_each_file_on_its_own_line() {
        let paths = [
            "gradle/dependency-locks/compileClasspath.lockfile",
            "gradle/dependency-locks/runtimeClasspath.lockfile",
            "gradle/dependency-locks/testCompileClasspath.lockfile",
        ];
        let repo = RepoInfo {
            root: PathBuf::from("/repo"),
            monorepo_layers: vec![layer(
                "/repo",
                MonorepoStandard::GradleMultiProject,
                observation(
                    LockfileStatus::Unverifiable,
                    &paths,
                    Some(LockfileReason::NoMembershipData),
                    &[],
                    &[],
                ),
            )],
            ..RepoInfo::default()
        };

        let plain = strip_escape_codes(render_lockfile_section(&repo, &Terminal::default()));
        let lines: Vec<&str> = plain
            .lines()
            .map(str::trim)
            .filter(|line| line.starts_with("- lockfile:"))
            .collect();
        let expected: Vec<String> = paths.iter().map(|path| format!("- lockfile: {path}")).collect();
        assert_eq!(lines, expected, "{plain}");
        assert!(
            rendered(&repo).contains("unverifiable — this lockfile does not record which packages are workspace members"),
            "{plain}"
        );
    }

    /// Every status/reason pairing ruling R5 allows reads differently, so no
    /// two reasons collapse into one explanation.
    #[test]
    fn each_status_and_reason_pairing_has_its_own_explanation() {
        use LockfileReason as Reason;
        use LockfileStatus as Status;

        let pairings = [
            (Status::Match, None),
            (Status::Mismatch, None),
            (Status::Absent, None),
            (Status::MembersPresent, Some(Reason::SubsetOnly)),
            (Status::MembersMissing, Some(Reason::SubsetOnly)),
            (Status::NotRequested, Some(Reason::RequestDisabled)),
            (Status::NotApplicable, Some(Reason::NoLockfileSource)),
            (Status::NotApplicable, Some(Reason::UnknownStandard)),
            (Status::Unverifiable, Some(Reason::UnsupportedVersion)),
            (Status::Unverifiable, Some(Reason::UnsupportedLayout)),
            (Status::Unverifiable, Some(Reason::NoMembershipData)),
            (Status::Unverifiable, Some(Reason::AmbiguousMembership)),
            (Status::Unverifiable, Some(Reason::IncompleteManifestDiscovery)),
            (Status::Unverifiable, Some(Reason::InvalidMemberPath)),
            (Status::Unreadable, Some(Reason::MetadataFailed)),
            (Status::Unreadable, Some(Reason::ReadFailed)),
            (Status::Unreadable, Some(Reason::ParseFailed)),
        ];
        let explanations: HashSet<&str> = pairings
            .iter()
            .map(|&(status, reason)| explanation(&observation(status, &[], reason, &[], &[])))
            .collect();
        assert_eq!(explanations.len(), pairings.len(), "{explanations:#?}");
    }
}
