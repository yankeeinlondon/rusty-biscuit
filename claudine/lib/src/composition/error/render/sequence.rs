//! Terminal rendering for the sequence error family.
//!
//! Every `CompositionError::Sequence…` variant without a block of its own
//! elsewhere renders here, following the error-message standard: the header
//! names what failed, the body states the variant's message (the authored
//! construct and the proven cause) and then its site as data (file,
//! property, task), and the hint is one next step for the cause that
//! applies. [`CompositionError::sequence_hint`] is the same hint, so the
//! passive check's row carries what the block does.

use super::super::*;
use super::rows;
use crate::composition::sequence::model::ExecutableField;
use renderable::markdown::code_span;

/// Render the [`StatusBlock`] for a sequence-family [`CompositionError`].
pub(super) fn status_block(err: &CompositionError) -> StatusBlock {
    let (headline, hint) = parts(err).expect("the dispatcher routes only sequence variants here");
    let mut body = vec![rows(super::escape_prose_path(&err.to_string()))];
    body.extend(evidence(err));
    body.extend(site_rows(err.sequence_site()));
    StatusBlock::new(StatusState::Error)
        .error_header(ErrorHeader::new("CompositionError", &headline))
        .body(body)
        .hint(hint)
}

impl CompositionError {
    /// The next step for a sequence error, for its cause: what its block's
    /// hint says, and what the passive check's row for it carries. `None`
    /// for an error that is not one of the family [`render`](super) gives a
    /// sequence block.
    #[must_use]
    pub fn sequence_hint(&self) -> Option<String> {
        parts(self.presented()).map(|(_, hint)| hint)
    }
}

/// The site's rows: `at:` the file (with its line and column when proven),
/// the authored `property:`, and the `task:` it belongs to, each only when
/// known.
fn site_rows(site: Option<&SequenceSite>) -> Option<Prose> {
    let site = site?;
    let mut lines = Vec::new();
    if let Some(file) = &site.file {
        let position = site.position.map(|(line, column)| format!(":{line}:{column}")).unwrap_or_default();
        lines.push(format!("<dim>at:</dim> {}{position}", super::render_file_link(file)));
    }
    if let Some(property) = &site.property {
        lines.push(format!("<dim>property:</dim> {}", code_span(property)));
    }
    if let Some(task) = &site.task {
        lines.push(format!("<dim>task:</dim> {}", super::escape_prose_path(task)));
    }
    (!lines.is_empty()).then(|| rows(lines.join("\n")))
}

/// Proof a variant carries beyond its message: the candidates an unresolved
/// reference tried.
fn evidence(err: &CompositionError) -> Option<Prose> {
    let CompositionError::SequenceReferenceResolution { resolution: Some(resolution), .. } = err else {
        return None;
    };
    if resolution.candidates().is_empty() {
        return None;
    }
    let mut tried = String::from("<b>Tried:</b>");
    for probed in resolution.candidates() {
        let path = biscuit_file::to_forward_slash_string(probed.candidate().path());
        tried.push_str(&format!("\n- <cyan>{}</cyan>", code_span(&path)));
    }
    Some(rows(tried))
}

/// The first candidate an unresolved reference tried, which is the file the
/// author most likely meant.
fn first_candidate(resolution: Option<&ResolutionDetail>) -> Option<String> {
    let candidate = resolution?.candidates().first()?;
    Some(biscuit_file::to_forward_slash_string(candidate.candidate().path()))
}

/// The header and hint of a sequence-family variant; `None` for any other.
fn parts(err: &CompositionError) -> Option<(String, String)> {
    use CompositionError as E;
    let noun = |field: &Option<ExecutableField>| field.map_or("file", ExecutableField::noun);
    let forward = |path: &Path| biscuit_file::to_forward_slash_string(path);
    let shown = |path: &Path| code_span(&forward(path));
    let (headline, hint): (String, String) = match err {
        E::SequenceInvalid { .. } => (
            "invalid sequence definition".into(),
            "Correct the value named above: a sequence is a list of steps, a file reference, an expression, or a shell expansion.".into(),
        ),
        E::SequenceEmpty { .. } => ("empty sequence".into(), "Add at least one step to the list.".into()),
        E::SequenceReferenceResolution { field, reference, resolution, source, .. } => match source {
            SequenceLoadCause::NotFound { glob_hint } => (
                format!("{} not found", field.noun()),
                match (glob_hint, first_candidate(resolution.as_deref())) {
                    (Some(hint), _) => (*hint).to_string(),
                    (None, Some(candidate)) => format!("Create {candidate}, or correct the reference `{reference}`."),
                    (None, None) => format!("Correct the reference `{reference}`."),
                },
            ),
            SequenceLoadCause::HomeDir => (
                format!("{} reference needs a home directory", field.noun()),
                "Set `HOME`, or write the reference without a leading `~`.".into(),
            ),
            _ => (
                format!("invalid {} reference", field.noun()),
                format!("Correct the reference `{reference}`."),
            ),
        },
        E::SequenceSourceRead { field, path, source, .. } => (
            format!("{} unreadable", noun(field)),
            match source.kind() {
                std::io::ErrorKind::PermissionDenied => format!("Grant read permission on {}.", shown(path)),
                _ => format!("Make {} a readable file.", shown(path)),
            },
        ),
        E::SequenceSourceParse { field, path, stage, source, site } => (
            format!("{} does not parse", noun(field)),
            match (stage, source) {
                (_, SequenceLoadCause::DuplicateKey { key, .. }) => {
                    format!("Remove one of the repeated `{key}` keys from {}.", shown(path))
                }
                (_, SequenceLoadCause::JsonLine { line, .. }) => {
                    format!("Fix line {line} of {}: each line holds one JSON value.", shown(path))
                }
                (SourceParseStage::Frontmatter, _) => format!(
                    "Correct the declaration at `{}`.",
                    site.property.as_deref().unwrap_or("$schema")
                ),
                (SourceParseStage::YamlConversion, _) => {
                    format!("Use string keys and plain values in {}.", shown(path))
                }
                (SourceParseStage::Yaml, _) => format!("Fix the YAML syntax of {}.", shown(path)),
                _ => format!("Fix the JSON syntax of {}.", shown(path)),
            },
        ),
        E::SequenceExternalWrongType { site, .. } => (
            "wrong sequence file structure".into(),
            if site.property.as_deref() == Some("list") {
                "Rename `list:` to `sequence:`.".into()
            } else {
                "Give the file a root `sequence:` list, make its root a list, or select a list with `-> offset`.".into()
            },
        ),
        E::SequenceStepNameMissing { .. } => ("step without a name".into(), "Add a `name:` to the step.".into()),
        E::SequenceStepNameWrongType { .. } => {
            ("step name is not a string".into(), "Write the step's `name` as a string.".into())
        }
        E::SequenceReservedTemplateKey { key, .. } => (
            "reserved template key".into(),
            format!("Remove `{key}` from `template`; Claudine generates or reserves it."),
        ),
        E::SequenceExclusiveExecutable { fields, .. } => (
            "more than one executable".into(),
            format!("Keep one of {} and move the others to steps of their own.", fields.join(", ")),
        ),
        E::SequenceReservedStateKey { key, .. } => (
            "reserved state key".into(),
            format!("Rename `{key}`; Claudine generates it for every step."),
        ),
        E::SequenceInvalidTaskField { field, executable, .. } => (
            "task field not valid here".into(),
            format!("Remove `{field}`, or use an executable that accepts it instead of `{executable}`."),
        ),
        E::SequenceUnknownTaskField { key, suggestion, allowed, .. } => (
            "unknown task field".into(),
            match suggestion {
                Some(suggestion) => format!("Did you mean `{suggestion}`? Rename `{key}`."),
                None => format!("Remove `{key}`, or use one of {}.", allowed.join(", ")),
            },
        ),
        E::SequenceSourceSyntax { .. } => (
            "invalid sequence source".into(),
            "Write the source as `<file> [-> offset] [::operator(args)]`.".into(),
        ),
        E::SequenceUnknownOperator { .. } => {
            ("unknown sequence operator".into(), "Use `map`, `name`, or `template`.".into())
        }
        E::SequenceOperatorArity { operator, expected, .. } => (
            "wrong operator arguments".into(),
            format!("Give `{operator}` {expected} argument(s)."),
        ),
        E::SequenceOffsetMissing { failed_at, .. } => (
            "offset not found".into(),
            format!("Correct the `->` path; `{failed_at}` does not exist."),
        ),
        E::SequenceOffsetNotAList { .. } => ("offset is not a list".into(), "Point the `->` path at a list.".into()),
        E::SequenceOffsetUnsupported { format, .. } => (
            "offset on a line-delimited file".into(),
            format!("Remove the `->` offset; a {format} file is already a list."),
        ),
        E::SequenceOperatorItemNotObject { operator, .. } => (
            "operator needs object items".into(),
            format!("Apply `{operator}` only to a list of objects, or remove it."),
        ),
        E::SequenceOperatorMissingField { field, index, .. } => (
            "item lacks the operator's field".into(),
            format!("Add `{field}` to item {}, or read a field every item has.", index + 1),
        ),
        E::SequenceOperatorEmptyName { expression, .. } => (
            "empty generated name".into(),
            format!("Make `template({expression})` produce a non-empty name for every item."),
        ),
        E::SequenceExpressionFailed { source, .. } => (
            "sequence expression failed".into(),
            match source {
                SequenceExpressionCause::Prepare(_) => "Fix the expression's syntax.".into(),
                SequenceExpressionCause::Evaluate(_) => "Make sure every value the expression reads exists.".into(),
                SequenceExpressionCause::Compose(_) => "Correct the template text.".into(),
            },
        ),
        E::SequenceShellFailed { source, .. } => (
            "sequence shell source failed".into(),
            match source {
                SequenceShellCause::Approval(_) => "Approve the command, or add it to the shell whitelist.".into(),
                SequenceShellCause::Spawn(_) => "Check that the command's executable exists and is runnable.".into(),
                SequenceShellCause::Exited { .. } => "Fix the command; it must exit 0 and print the list.".into(),
                SequenceShellCause::Unavailable => {
                    "Run the sequence with `claudine sequence`, which can expand a `$( … )` source.".into()
                }
            },
        ),
        E::SequenceNullItem { .. } => (
            "null sequence item".into(),
            "Replace the `null` item with a name or an object, or remove it.".into(),
        ),
        E::SequenceStateSchemaViolation { property, container, .. } => (
            "step state fails its `$state` schema".into(),
            format!("Correct `{property}`, or relax the `$state` schema of `{container}`."),
        ),
        E::SequenceDeclarationMisplaced { hint, .. } => ("misplaced sequence declaration".into(), hint.clone()),
        E::SequenceDeclarationShape { property, .. } => (
            "invalid sequence declaration".into(),
            format!("Correct `{property}` as the message says."),
        ),
        E::SequenceDeclarationLoad { property, .. } => (
            "sequence declaration could not be loaded".into(),
            format!("Fix the schema `{property}` declares, or the file it references."),
        ),
        E::SequenceContainerSchemaViolation { container, property, .. } => (
            "container input fails its `$schema`".into(),
            format!("Supply a valid `{property}` for container `{container}`, or relax its `$schema`."),
        ),
        E::SequenceReferenceCycle { .. } => (
            "reference cycle".into(),
            "Remove one of the references in the chain so it no longer returns to a file it left.".into(),
        ),
        E::SequenceReferenceInvalid { context, .. } => (
            "invalid sequence reference".into(),
            format!("Correct the {context} as the message says."),
        ),
        E::SequenceNestedSequence { path, .. } => (
            "prompt document declares a sequence".into(),
            format!("Reference it with `sequence: {}` instead of `prompt:`.", forward(path)),
        ),
        E::SequenceUndefined { .. } => (
            "not a sequence document".into(),
            "Add a `sequence:` list of tasks, or run the document with `claudine compose`.".into(),
        ),
        E::SequenceUnsupportedConstruct { detail, .. } => ("unsupported sequence construct".into(), capitalized(detail)),
        E::SequenceTaskPromptLaunch { .. } => (
            "task prompt could not launch".into(),
            "Fix the launch failure in the message; the task's prompt never ran.".into(),
        ),
        E::SequenceStepNotExecutable { .. } => (
            "step has nothing to run".into(),
            "Give the step an executable such as `prompt: ./step.md`, or give the document a body.".into(),
        ),
        E::SequenceGroupCatalogLookup { name, available, problem, .. } => (
            "group not found in catalog".into(),
            if problem.contains("more than once") {
                format!("Define group `{name}` once in the catalog.")
            } else if available.is_empty() {
                format!("Add a group named `{name}` to the catalog.")
            } else {
                format!("Use one of the catalog's groups: {}.", available.join(", "))
            },
        ),
        E::SequenceGroupInvalid { .. } => ("invalid group".into(), "Correct the group as the message says.".into()),
        E::SequenceWriteBackCollision { .. } => (
            "parallel write-back collision".into(),
            "Give each task of the parallel group its own inline-compose target, or run the group sequentially.".into(),
        ),
        E::SequenceShellLateBinding { root, .. } => (
            "shell command reads a value it cannot have".into(),
            format!("Read `{root}` in a lifecycle event instead, or pass the value in through a task's `with`."),
        ),
        E::SequenceShellTargetIdentity { .. } => (
            "shell command reads the selected target".into(),
            "Move this command into the task that owns the target.".into(),
        ),
        E::SequenceShellResolution { .. } => (
            "shell command could not be prepared".into(),
            "Correct the command so it resolves before the sequence starts.".into(),
        ),
        E::SequenceShellLateValue { .. } => (
            "shell command value missing".into(),
            "Make sure an earlier step produces the value this command reads.".into(),
        ),
        E::SequenceShellApprovalChanged { .. } => (
            "approved command changed".into(),
            "Run the sequence again so preflight approves the command as it now resolves.".into(),
        ),
        E::SequenceTaskTimeoutInvalid { .. } => (
            "invalid task timeout".into(),
            "Write `timeout` as a duration with a unit, such as `30s` or `5m`.".into(),
        ),
        E::SequenceTaskShellExit { code, .. } => (
            "task command failed".into(),
            format!("Fix the command or the state it runs in; it exited with code {code}."),
        ),
        E::SequenceTaskShellTimeout { .. } => (
            "task command timed out".into(),
            "Raise the task's `timeout`, or make the command finish sooner.".into(),
        ),
        E::SequenceTaskShellRunaway { .. } => (
            "task command output runaway".into(),
            "Reduce the command's output, or redirect it to a file.".into(),
        ),
        E::SequenceTaskShellSpawn { .. } => (
            "task command could not start".into(),
            "Check that the command's executable exists and is runnable.".into(),
        ),
        E::SequenceTaskShellWait { .. } => (
            "task command status unknown".into(),
            "Check the command's effects, then run the task again.".into(),
        ),
        E::SequenceTaskShellIsolation { .. } => (
            "task command could not be isolated".into(),
            "Run on a host where Claudine can own the command's process tree.".into(),
        ),
        E::SequenceTaskPromptFailed { code, .. } => (
            "task prompt failed".into(),
            format!("Check the provider's output; the prompt exited with code {code}."),
        ),
        E::SequenceTaskInvalidSideEffect { .. } => (
            "not a side-effect action".into(),
            "Use a side-effect action in `side_effect`, or move this action to a lifecycle event.".into(),
        ),
        E::SequenceTaskWithReserved { key, .. } => (
            "reserved `with` key".into(),
            format!("Rename `with.{key}`; that key is the executor's."),
        ),
        E::SequenceTaskValueResolution { field, .. } => (
            "task value could not be evaluated".into(),
            format!("Correct `{field}`, or make sure the values it reads exist at the task's turn."),
        ),
        E::SequenceTaskUnsupported { detail, .. } => ("task not executable yet".into(), capitalized(detail)),
        _ => return None,
    };
    Some((headline, hint))
}

/// `text` with its first letter capitalized, as a hint sentence starts.
fn capitalized(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |first| first.to_uppercase().chain(chars).collect())
}
