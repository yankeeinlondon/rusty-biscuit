//! Schema-aware completion for composition `name=value` setters.
//!
//! Phase 4 of the `2026-05-15-schemas` feature. When the cursor sits in a
//! setter slot of `claudine compose`, `claudine inline-compose`, or
//! `claudine sequence` AND a positional prompt-file argument is already
//! committed, this module consults the prompt's `$schema` declaration via
//! Darkmatter to produce:
//!
//! - **Property names** (before `=`) — required properties first, then
//!   optional, in declaration order. Already-supplied names are filtered out.
//!   See [`keys`].
//! - **Property values** (after `=`) — enum members for `enum` properties,
//!   filesystem paths constrained by `match(...)` globs for `file` properties.
//!   See [`candidates`].
//!
//! Names and the values of a property with no `{{ … }}` slot are pure
//! functions over schema state plus filesystem reads: no shell execution, no
//! provider launches, nothing evaluated. When the schema cannot be resolved or
//! the file_arg cannot be loaded, every function returns an empty `Vec`,
//! signalling to the caller that the shell's native fallback should take
//! over.
//!
//! A schema holding slots loads as [`PromptSchema::Pending`]: its property
//! names complete from the authored template. A value request for a property
//! whose authored definition holds a slot (a template constraint or a
//! template type) resolves the whole template through Darkmatter's resolver
//! with [`SessionMode::ReferenceCheck`]: only the automatic clean-committed
//! grant and persisted user grants apply, nothing prompts, and nothing is
//! printed. An ungranted or failed resolution offers no candidates; one slot
//! elsewhere in the template that cannot resolve leaves the property with
//! none too, because the template resolves as a whole. See
//! [`setter_value_candidates`].

use std::path::PathBuf;
use std::sync::Arc;

use biscuit_file::{FileReference, FileResolutionContext};
use darkmatter::markdown::compose::{ComposeSource, DataRuntime};
use darkmatter::markdown::schemas::completion as dm_completion;
use darkmatter::markdown::schemas::slots::{SessionMode, SlotSession};
use darkmatter::markdown::schemas::{DarkmatterSchemas, EffectiveSchema};
use darkmatter::markdown::Markdown;
use simplified_schema::simplified::{DefinitionTable, Slot};
use simplified_schema::{SchemaError, SchemaRoot};

use super::scopes::{self, ScopeContext};

mod candidates;
mod keys;
#[cfg(test)]
mod parity_tests;
#[cfg(test)]
mod tests;

pub(crate) use candidates::file_candidate_paths;
#[cfg(test)]
pub(crate) use candidates::property_value;
pub(crate) use keys::{declared_property_order, property_names_in};
#[cfg(test)]
pub(crate) use keys::property_names;

/// The schema a committed prompt declares, as completion can use it.
pub(crate) enum PromptSchema {
    /// A concrete effective schema: names and values complete from it.
    Resolved(Box<EffectiveSchema>),
    /// The schema is a template holding `{{ … }}` slots, loaded without
    /// evaluating any: the template's declarations name the properties.
    Pending(PendingPromptSchema),
}

/// A template schema completion did not resolve.
pub(crate) struct PendingPromptSchema {
    /// The expanded template (slots in place) and the stored definitions its
    /// references name, when the resolution that met it kept one.
    pub(crate) template: Option<(SchemaRoot, Arc<DefinitionTable>)>,
    /// The slots the template holds, in declaration order.
    pub(crate) slots: Vec<Slot>,
}

impl PromptSchema {
    /// The concrete schema, when there is one.
    #[cfg(test)]
    pub(crate) fn resolved(&self) -> Option<&EffectiveSchema> {
        match self {
            Self::Resolved(effective) => Some(effective),
            Self::Pending(_) => None,
        }
    }

    /// The declarations property names complete from: the effective
    /// schema's, or the pending template's.
    pub(crate) fn declarations(&self) -> Option<&SchemaRoot> {
        match self {
            Self::Resolved(effective) => effective.simplified.as_ref(),
            Self::Pending(pending) => pending.template.as_ref().map(|(root, _)| root),
        }
    }
}

/// Resolve `file_arg` to the schema its document declares.
///
/// Returns `None` when:
/// - the committed prompt reference does not resolve to a file,
/// - the file is not valid Markdown,
/// - the document has no `$schema`,
/// - the schema cannot be parsed or compiled.
///
/// Those failure modes are silent: completion is best-effort and falls back
/// to shell-native behavior. A template schema is not a failure: it is
/// [`PromptSchema::Pending`], so callers can still offer its property names.
pub(crate) fn load_prompt_schema(file_arg: &str, ctx: &ScopeContext) -> Option<PromptSchema> {
    LoadedPrompt::load(file_arg, ctx)?.schema()
}

/// Value candidates for `property=<value_partial>` from `file_arg`'s schema;
/// empty when the schema offers none, so the caller's fallback runs.
///
/// A concrete schema answers directly. In a template, a property whose
/// authored definition reaches no slot completes from that declaration with
/// nothing evaluated; a property that reaches one resolves the template
/// first with non-prompting grants only (see the module documentation) and
/// completes from the resolved schema.
pub(crate) fn setter_value_candidates(
    file_arg: &str,
    property: &str,
    value_partial: &str,
    ctx: &ScopeContext,
) -> Vec<String> {
    let Some(loaded) = LoadedPrompt::load(file_arg, ctx) else {
        return Vec::new();
    };
    match loaded.schema() {
        Some(PromptSchema::Resolved(effective)) => {
            candidates::property_value(&effective, property, value_partial, ctx)
        }
        Some(PromptSchema::Pending(pending)) => {
            let Some((root, definitions)) = &pending.template else {
                return Vec::new();
            };
            let Some(def) = simplified_schema::completion::root_property_def(root, property) else {
                return Vec::new();
            };
            if simplified_schema::slots::reachable_slots(&def, definitions).is_empty() {
                let suggestion = dm_completion::for_root_property(root, definitions.as_ref(), property);
                return candidates::suggestion_values(suggestion, property, value_partial, ctx);
            }
            match loaded.resolve_template() {
                Some(effective) => candidates::property_value(&effective, property, value_partial, ctx),
                None => Vec::new(),
            }
        }
        None => Vec::new(),
    }
}

/// A committed prompt read and parsed once, for loading its schema and, when
/// a value needs it, resolving its template.
struct LoadedPrompt {
    prompt: CommittedPrompt,
    markdown: Markdown,
}

impl LoadedPrompt {
    fn load(file_arg: &str, ctx: &ScopeContext) -> Option<Self> {
        let prompt = CommittedPrompt::resolve(file_arg, ctx)?;
        let text = std::fs::read_to_string(&prompt.path).ok()?;
        let markdown = Markdown::from(text).with_source(ComposeSource::infer_from_path(&prompt.path));
        Some(Self { prompt, markdown })
    }

    /// The schema with no slot evaluated.
    fn schema(&self) -> Option<PromptSchema> {
        match DarkmatterSchemas::new(self.prompt.context.clone()).effective_for(&self.markdown) {
            Ok(effective) => effective.map(|effective| PromptSchema::Resolved(Box::new(effective))),
            Err(SchemaError::UnresolvedSlots(unresolved)) => {
                let template = unresolved.pending.as_ref().and_then(|pending| {
                    pending
                        .template()
                        .map(|(root, definitions)| (root.clone(), Arc::clone(definitions)))
                });
                let pending = PendingPromptSchema { template, slots: unresolved.slots };
                tracing::debug!(
                    target: "claudine::completion",
                    prompt = %self.prompt.path.display(),
                    slots = pending.slots.len(),
                    "the prompt's schema is a template"
                );
                Some(PromptSchema::Pending(pending))
            }
            Err(_) => None,
        }
    }

    /// The effective schema with every slot resolved, using only grants that
    /// never prompt; `None` when a read is ungranted or resolution fails.
    ///
    /// The session has no approval handler, so `ReferenceCheck` leaves an
    /// undecided read unresolved rather than asking: a Tab must never prompt
    /// or write to the terminal.
    fn resolve_template(&self) -> Option<EffectiveSchema> {
        let context = &self.prompt.context;
        let session =
            SlotSession::for_context(DataRuntime::new(), context).with_mode(SessionMode::ReferenceCheck);
        match DarkmatterSchemas::new(context.clone())
            .with_slot_session(Arc::new(session))
            .effective_for(&self.markdown)
        {
            Ok(effective) => effective,
            Err(error) => {
                tracing::debug!(
                    target: "claudine::completion",
                    prompt = %self.prompt.path.display(),
                    %error,
                    "the prompt's schema template did not resolve; values fall back"
                );
                None
            }
        }
    }
}

/// Resolve `file_arg` to a concrete [`EffectiveSchema`]; `None` for every
/// case [`load_prompt_schema`] reports as `None` and for a template.
#[cfg(test)]
pub(crate) fn load_effective_schema(file_arg: &str, ctx: &ScopeContext) -> Option<EffectiveSchema> {
    match load_prompt_schema(file_arg, ctx)? {
        PromptSchema::Resolved(effective) => Some(*effective),
        PromptSchema::Pending(_) => None,
    }
}

/// The prompt-file argument already committed in argv, resolved the way
/// composition resolves it.
pub(super) struct CommittedPrompt {
    pub(super) path: PathBuf,
    /// The completion context derived for the prompt document, so the
    /// document's own references (`$schema` files, `file` values) resolve as
    /// they will when the prompt is composed.
    pub(super) context: FileResolutionContext,
}

impl CommittedPrompt {
    /// Resolve `file_arg` through the shared file-reference grammar and the
    /// completion request's context.
    ///
    /// The argument keeps whatever form the user committed, including the
    /// `@`/`&`/`^`/`~/` tokens the positional completer emits, so a prefixed
    /// prompt and an explicit `./` keep their composition semantics: `./`
    /// never falls back to the repository root.
    pub(super) fn resolve(file_arg: &str, ctx: &ScopeContext) -> Option<Self> {
        let trimmed = file_arg.trim_matches(['"', '\'']);
        if trimmed.is_empty() {
            return None;
        }
        let reference = FileReference::new(trimmed).ok()?;
        let request = scopes::file_resolution_context(ctx).ok()?;
        let path = reference.resolve_in_context(&request).ok()??;
        if !path.is_file() {
            return None;
        }
        let path = in_repository_spelling(&request, path);
        let ordinary = request.for_source_reference(&reference, &path);
        if ordinary.validate().is_ok() && request.repository_root().is_some() {
            return Some(Self { path, context: ordinary });
        }
        // Outside the launch repository (or launched from none) the prompt
        // gets composition's source derivation: its own repository anchors
        // `&`, `^`, and bare references, while `@` keeps the launch scope.
        let context = claudine::composition::derive_request_context_for_source(&request, &path)
            .ok()?
            .for_source_reference(&reference, &path);
        Some(Self { path, context })
    }
}

/// `path` spelled under the request's repository root when it names a file
/// inside that repository through another spelling (`/var` against
/// `/private/var`, a symlinked checkout).
///
/// Composition re-derives a source's repository by canonical directory; a
/// derived context only keeps its repository for a lexically contained source,
/// so without this an absolute prompt spelled differently would lose its
/// repository-relative `$schema` references.
fn in_repository_spelling(request: &FileResolutionContext, path: PathBuf) -> PathBuf {
    let Some(root) = request.repository_root() else {
        return path;
    };
    if path.starts_with(root) {
        return path;
    }
    let (Ok(canonical_root), Ok(canonical_path)) = (root.canonicalize(), path.canonicalize())
    else {
        return path;
    };
    match canonical_path.strip_prefix(&canonical_root) {
        Ok(relative) => root.join(relative),
        Err(_) => path,
    }
}

#[cfg(test)]
fn ordered_completable_suggestions(
    effective: &EffectiveSchema,
) -> Vec<darkmatter::markdown::schemas::CompletionSuggestion> {
    use darkmatter::markdown::schemas::completion as dm_completion;
    dm_completion::completable_properties(effective)
        .into_iter()
        .filter_map(|name| dm_completion::for_property(effective, &name))
        .collect()
}
