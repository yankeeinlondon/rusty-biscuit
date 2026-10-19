//! The CLI's runner for lifecycle `prep` targets.
//!
//! A `prep` call runs its target as an invocation of its own, in this
//! process, through the same entry points `claudine compose`,
//! `inline-compose`, and `sequence` use: the target is read, schema-checked,
//! approved, and run with its own `initialize`, runtime layer, and output.
//! The caller's provider and approval flags carry over; the caller's
//! setters, loop, sequence, and group roots do not. The `with:` overlay
//! becomes the target's caller values, as data, anchored at the document that
//! authored the `prep`.
//!
//! The runner is installed per thread by the top-level composition commands
//! ([`install_for`]); the library refuses a `prep` on a thread without one.

use std::sync::Arc;

use claudine::composition::lifecycle::prep::{self, PrepCall, PrepFailure, PrepRunner, PrepScope};
use claudine::diagnostics::DiagnosticSnapshot;

use super::compose::{CompositionKind, SharedComposeArgs};

/// Runs `prep` targets with the flags of the command that installed it.
pub(crate) struct CliPrepRunner {
    shared: SharedComposeArgs,
    verbose: u8,
}

/// Install the `prep` target runner for this command on the current thread.
pub(crate) fn install_for(shared: &SharedComposeArgs, verbose: u8) -> PrepScope {
    prep::install(Arc::new(CliPrepRunner { shared: shared.clone(), verbose }))
}

/// How a target runs.
enum TargetKind {
    Sequence,
    Document(CompositionKind),
}

impl PrepRunner for CliPrepRunner {
    fn run(&self, call: &PrepCall<'_>) -> Result<(), PrepFailure> {
        let resolved = claudine::composition::resolve_proxy_target(call.target, call.caller, call.file_resolution_context)
            .map_err(|error| failure(&color_eyre::eyre::Report::from(error)))?;
        let target = biscuit_file::canonicalize_simplified(&resolved).unwrap_or(resolved);
        let _call = prep::enter_call(call, &target).map_err(|error| failure(&color_eyre::eyre::Report::from(error)))?;
        let kind = target_kind(&target).map_err(|error| failure(&error))?;
        let mut shared = self.shared.clone();
        // The target receives the overlay as its only caller values, authored
        // by the document whose `prep` fired, so a relative file value
        // resolves from that document rather than the launch directory.
        let overlay = call.overlay_as_data();
        shared.set_records = claudine::composition::CallerInputLayers::from_caller_overrides(
            Some(overlay.clone()),
            call.file_resolution_context.for_source(call.caller),
        )
        .caller_input_records;
        shared.set = Some(overlay.to_string());
        shared.caller_arguments = Default::default();
        let file = biscuit_file::to_portable_string(&target);
        let status = match kind {
            TargetKind::Sequence => super::sequence::run_sequence_target(shared, file, self.verbose),
            TargetKind::Document(kind) => {
                super::compose::prep::run_composition_inner(shared, vec![file], self.verbose, None, kind)
            }
        };
        match status {
            Ok(0) => Ok(()),
            Ok(code) => Err(PrepFailure { message: format!("it exited with status {code}"), snapshot: None }),
            Err(error) => Err(failure(&error)),
        }
    }
}

fn failure(error: &color_eyre::eyre::Report) -> PrepFailure {
    PrepFailure {
        message: error.to_string(),
        snapshot: DiagnosticSnapshot::select(error.as_ref()).map(Box::new),
    }
}

/// Classify a resolved target by what it declares: a sequence document (or a
/// YAML sequence file) runs as `claudine sequence` does, a document whose
/// frontmatter `prompt:` is a string as `inline-compose`, any other document
/// as `compose`.
fn target_kind(path: &std::path::Path) -> color_eyre::eyre::Result<TargetKind> {
    if claudine::composition::is_yaml_source(path) {
        return Ok(TargetKind::Sequence);
    }
    let markdown = darkmatter::markdown::Markdown::try_from(path).map_err(|error| {
        color_eyre::eyre::Report::new(error).wrap_err(format!("could not read `prep` target {}", path.display()))
    })?;
    let frontmatter = markdown.frontmatter().as_map();
    // A `kind: group` file is a container too; the sequence entry point
    // names it if it cannot run on its own.
    if frontmatter.contains_key("sequence") || frontmatter.get("kind").and_then(serde_json::Value::as_str) == Some("group") {
        return Ok(TargetKind::Sequence);
    }
    Ok(match frontmatter.get("prompt") {
        Some(serde_json::Value::String(_)) => TargetKind::Document(CompositionKind::Inline),
        _ => TargetKind::Document(CompositionKind::Direct),
    })
}
