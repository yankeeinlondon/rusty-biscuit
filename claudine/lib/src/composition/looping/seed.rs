//! Loop-seed construction — the single compose pass run before iteration 1
//! that lifts iteration-control variables (and, optionally, the full parsed
//! lifecycle config) out of a resolved composition source.

use serde_json::{Map, Value};

use super::super::error::CompositionError;
use super::super::lifecycle::LifecycleConfig;
use super::super::prepare::{BootstrapPreparation, PrepareOptions, prepare_direct, prepare_inline};
use super::super::types::{CompositionMode, LoopConfig, ResolvedCompositionSource};
use super::config::extract_control_variables;

/// Build the initial frontmatter for a loop from resolved control variables.
///
/// Runs one compose pass to resolve the document, then lifts only:
/// - CLI `set_overrides` keys, carried verbatim so the body sees them every
///   iteration;
/// - control variables (action targets, condition identifiers, and identifiers
///   referenced by action-value templates), resolved from
///   `effective_frontmatter`.
///
/// Derived/presentation frontmatter keys are intentionally omitted so they
/// re-resolve each iteration against current state and ambients.
///
/// `mode` selects the seed compose pass so seeding matches the iteration
/// executor:
/// - [`CompositionMode::ChainedDocument`] composes the document body (as
///   `compose` does); a doc with an empty body fails seed resolution with
///   [`CompositionError::ComposedBodyEmpty`].
/// - [`CompositionMode::InlineFrontmatterPrompt`] composes the frontmatter
///   `prompt` value as the body (as `inline-compose` does); a doc whose
///   prompt lives in frontmatter resolves even when the body is empty.
///   Without this mode split, an inline-compose doc with an empty body
///   would fail seed resolution before iteration 1 even though the
///   iteration executor composes the `prompt:` frontmatter value.
///
/// ## Errors
///
/// Returns `CompositionError` when the seed compose pass fails.
pub fn build_loop_seed(
    source: &ResolvedCompositionSource,
    config: &LoopConfig,
    prepare_options: PrepareOptions,
    mode: CompositionMode,
) -> Result<Map<String, Value>, CompositionError> {
    Ok(build_loop_seed_with_lifecycle(source, config, prepare_options, mode)?.seed)
}

/// The seed frontmatter for a loop plus the **full** parsed lifecycle config.
///
/// [`build_loop_seed`] lifts only iteration-control variables into the seed,
/// dropping every lifecycle event block (`initialize`/`start`/`success`/
/// `blocked`/`failure`/`finalize` and the `loop:` gate's concerns). The loop
/// runner needs those blocks to fire lifecycle events, so this struct carries
/// the lifecycle config parsed from the document's full composed frontmatter
/// alongside the control-variable seed.
#[derive(Debug)]
pub struct LoopSeed {
    /// Iteration-control seed frontmatter (control variables + CLI setters).
    pub seed: Map<String, Value>,
    /// Full bootstrap state for initialize and its catch handlers. Kept out of
    /// the control seed so derived values still recompose on each iteration.
    pub initialize_frontmatter: Map<String, Value>,
    /// Lifecycle config parsed from the **full** composed effective
    /// frontmatter — carries every event block, unlike [`Self::seed`].
    pub lifecycle: LifecycleConfig,
}

/// Build the loop seed and parse the lifecycle config from the full composed
/// frontmatter.
///
/// This runs the same single compose pass as [`build_loop_seed`] but returns
/// the document's complete lifecycle config (parsed from
/// `prepared.effective_frontmatter`, which contains the lifecycle event
/// blocks) in addition to the control-variable-only seed. The loop runner
/// uses the lifecycle config so loop iterations fire `initialize`/`start`/
/// terminal/`finalize` and the `loop:` gate concerns — the seed alone would
/// parse to an empty lifecycle config because the control-variable lift drops
/// every event block.
///
/// ## Errors
///
/// Returns `CompositionError` when the compose pass fails.
pub fn build_loop_seed_with_lifecycle(
    source: &ResolvedCompositionSource,
    config: &LoopConfig,
    prepare_options: PrepareOptions,
    mode: CompositionMode,
) -> Result<LoopSeed, CompositionError> {
    let prepared = match mode {
        CompositionMode::ChainedDocument => prepare_direct(source, prepare_options.clone())?,
        CompositionMode::InlineFrontmatterPrompt => {
            prepare_inline(source, prepare_options.clone())?
        }
    };
    Ok(LoopSeed {
        seed: lift_seed(
            config,
            &prepared.effective_frontmatter,
            prepare_options.set_overrides.as_ref(),
        ),
        lifecycle: prepared.lifecycle,
        initialize_frontmatter: prepared.effective_frontmatter.as_object().cloned().unwrap_or_default(),
    })
}

/// The loop seed for a document whose `initialize` has not run yet.
///
/// Takes the seed and lifecycle from the initialize-bootstrap read instead of a
/// full compose, so recognizing loop ownership never dereferences the body
/// `initialize` may still be about to create. Loop control reads frontmatter
/// only, so both constructors lift the same seed from the same effective
/// frontmatter.
#[must_use]
pub fn build_loop_seed_from_bootstrap(
    bootstrap: &BootstrapPreparation,
    config: &LoopConfig,
) -> LoopSeed {
    LoopSeed {
        seed: lift_seed(
            config,
            &bootstrap.effective_frontmatter,
            bootstrap.input_layers.set_overrides.as_ref(),
        ),
        lifecycle: bootstrap.lifecycle.clone(),
        initialize_frontmatter: bootstrap.effective_frontmatter.as_object().cloned().unwrap_or_default(),
    }
}

fn lift_seed(
    config: &LoopConfig,
    effective: &Value,
    set_overrides: Option<&Value>,
) -> Map<String, Value> {
    let mut seed = Map::new();

    if let Some(Value::Object(set_overrides)) = set_overrides {
        for (key, value) in set_overrides {
            seed.insert(key.clone(), value.clone());
        }
    }

    for name in extract_control_variables(config) {
        if let Some(value) = effective.get(&name) {
            seed.insert(name, value.clone());
        }
    }
    seed
}
