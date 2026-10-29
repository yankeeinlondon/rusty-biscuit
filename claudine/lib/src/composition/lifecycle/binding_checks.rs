//! Lifecycle checks that read Claudine's binding catalog.
//!
//! Darkmatter's lifecycle validators walk every expression surface
//! ([`walk_stack`]); the checks here judge those surfaces against the
//! lifecycle globals Claudine declares in [`super::bindings`], which is
//! Claudine policy and stays here.

use std::path::Path;

use darkmatter::lifecycle::{
    AuthoredValue, LifecycleConfig, LifecycleSignal, LifecycleSourceMap, event_surfaces, walk_stack,
};
use darkmatter::lifecycle::actions::LifecyclePath;
use darkmatter::markdown::compose::expression::{
    AuthoredMode, Expr, ParseMode, prepare_value, validate_prepared,
};

use super::bindings;
use crate::composition::error::CompositionError;

/// Validates that no lifecycle expression reads a global its event declares
/// unavailable, in any branch.
///
/// A thin adapter over Darkmatter's passive validation: each event's
/// [`bindings::binding_view`] declares `err` unavailable in `initialize`,
/// `start`, `success`, and `loop` (`claudine.event-has-no-error`), and every
/// authored surface [`walk_stack`] yields, in both branches, is prepared from
/// its authored text and checked against that view. Group membership is not known yet, so
/// `group` is deferred to runtime. `fm.err` reads the document everywhere.
/// Nothing is evaluated and no provider runs.
///
/// Expression syntax is not this check's concern: the parser has already
/// rejected every guard, whole-value operand, and mixed-text span that does
/// not parse, in both branches. An unknown function is reported when the event
/// evaluates it.
///
/// ## Errors
///
/// [`CompositionError::LifecycleErrNotAvailable`] for the first surface, in
/// [`LifecycleSignal::ALL`] order, that reads an unavailable global, carrying
/// Darkmatter's typed unavailable-binding error as its source.
pub fn validate_no_err_in_no_error_events(
    frontmatter: &serde_json::Value,
    lifecycle: &LifecycleConfig,
    source_path: &Path,
) -> Result<(), CompositionError> {
    let sources = LifecycleSourceMap::from_frontmatter(frontmatter);
    let surfaces = event_surfaces(lifecycle);
    for signal in LifecycleSignal::ALL {
        let view = bindings::binding_view(
            bindings::LifecycleScope::Event(signal),
            bindings::GroupMembership::Unknown,
        );
        let mut authored: Vec<(String, &str, AuthoredMode)> = Vec::new();
        for (_, path, surface) in surfaces.iter().filter(|(owner, ..)| *owner == signal) {
            if let Some(AuthoredValue::Text(text)) = sources.get(path) {
                let mode = if surface.predicate {
                    AuthoredMode::Expression(ParseMode::Condition)
                } else {
                    AuthoredMode::InterpolatedValue
                };
                authored.push((path.to_string(), text, mode));
            }
        }
        for (property, text, mode) in authored {
            // Unreachable for a parsed config (see above); a syntax failure is
            // the parser's to report, not a missing-`err` diagnostic.
            let Ok(prepared) = prepare_value(&serde_json::Value::String(text.to_string()), mode)
            else {
                continue;
            };
            let unavailable = validate_prepared(&prepared, &view)
                .into_iter()
                .find(|diagnostic| bindings::unavailable_root(&diagnostic.error).is_some());
            if let Some(diagnostic) = unavailable {
                return Err(CompositionError::LifecycleErrNotAvailable {
                    source_path: source_path.to_path_buf(),
                    property,
                    event: signal.property_name().to_string(),
                    source: Some(Box::new(diagnostic.error)),
                });
            }
        }
    }
    Ok(())
}

/// The first read of a retired `_loop_*` name in any lifecycle stack, with its
/// authored path and the retired name and `loop` member it names.
///
/// Stacks are walked in [`LifecycleSignal::ALL`] order through both branches.
/// A guard is read as the condition it is; an operand that stayed literal text
/// (`"pass {{ _loop_count }}"`) is read through its `{{ … }}` spans, so a quoted
/// literal or plain text naming `_loop_count` is data, never a read.
pub(crate) fn find_removed_loop_read(
    lifecycle: &LifecycleConfig,
) -> Option<(LifecyclePath, (&'static str, &'static str))> {
    for signal in LifecycleSignal::ALL {
        let mut found = None;
        walk_stack(lifecycle.stack(signal), &mut |path, surface| {
            if found.is_some() {
                return;
            }
            let retired = match surface.expr {
                Expr::StringLiteral(text) if !surface.predicate => {
                    bindings::removed_loop_read_in_text(text)
                }
                expr => bindings::removed_loop_read(expr),
            };
            found = retired.map(|retired| (path.clone(), retired));
        });
        if found.is_some() {
            return found;
        }
    }
    None
}
