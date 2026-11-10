//! The one selection of the file a schema reference names.
//!
//! A `$schema` value, a named-type import (`Name@file.yaml`), an
//! `example(file.yaml)` constraint, and a trigger payload all name a schema
//! file the same way, and validation, document links, and go-to-definition
//! must agree on which file that is. [`select_schema_file`] is that rule:
//!
//! - a bare name (one path component, no `./`, no magic prefix) is looked up
//!   in each schema root in order, and the first root holding it wins;
//! - with no schema roots, or for any other form, the reference resolves
//!   through `biscuit-file` from the referencing document's folder.
//!
//! While a [`buffers`] snapshot is in force, an open buffer at a location
//! that holds no file is a match there, in the search's own order: a file
//! that exists only in the editor is found where it would be once saved, and
//! wins over every later location, which is then neither admitted nor
//! probed. A recursive (`%`) reference walks only the disk.
//!
//! Besides the selected file, the selection reports every location it
//! probed, in order, and the path a missing reference would be created at,
//! so an editor can link to it without a second lookup rule.

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use biscuit_file::{FileReference, FileReferenceError, FileReferenceKind, FileResolutionContext};

#[cfg(feature = "file")]
use super::buffers;
use super::observed;
use crate::errors::SchemaError;

/// The file a schema reference selects, with what the selection probed.
#[derive(Debug)]
pub struct SchemaFileSelection {
    /// The trimmed authored reference.
    pub reference: String,
    /// What the reference selects.
    pub outcome: SelectionOutcome,
    /// Every location probed, in probe order, deduplicated by first
    /// occurrence, lexical (absent locations included), through the selected
    /// one.
    pub attempted: Vec<PathBuf>,
}

/// What a schema reference selects.
#[derive(Debug)]
pub enum SelectionOutcome {
    /// The existing file the reference names.
    Found(PathBuf),
    /// A clean miss: nothing exists at any probed location.
    Missing {
        /// True only when the reference was looked up in the schema roots.
        bare_name: bool,
        /// Where the file would be created: the first schema root for a bare
        /// name, else the reference's first planned candidate. `None` when the
        /// reference plans no candidate.
        creation: Option<PathBuf>,
    },
    /// A bare name absent from every schema root while a file of that name
    /// is beside the referencing document.
    SiblingOnly {
        /// The file beside the document, which the reference does not name.
        sibling: PathBuf,
        /// The reference that would name it (`./name`).
        suggestion: String,
        /// The path in the first schema root the bare name would be created
        /// at.
        creation: PathBuf,
    },
    /// The reference cannot be resolved at all; no path is offered.
    Failed(SelectionFailure),
}

/// Why a schema reference cannot be resolved.
#[derive(Debug)]
pub enum SelectionFailure {
    /// A URL; remote schemas are never fetched.
    Remote,
    /// A malformed reference, a missing anchor, a boundary violation, or a
    /// location that could not be checked.
    Resolution(FileReferenceError),
}

impl SchemaFileSelection {
    /// The selection as the resolver reports it.
    ///
    /// ## Errors
    ///
    /// [`SelectionOutcome::Missing`] is [`SchemaError::NoMatch`],
    /// [`SelectionOutcome::SiblingOnly`] is
    /// [`SchemaError::BareNameSiblingExists`], [`SelectionFailure::Remote`]
    /// is [`SchemaError::RemoteUnsupported`], and
    /// [`SelectionFailure::Resolution`] is [`SchemaError::Unresolved`].
    pub fn into_result(self) -> Result<PathBuf, SchemaError> {
        let reference = self.reference;
        match self.outcome {
            SelectionOutcome::Found(path) => Ok(path),
            SelectionOutcome::Missing { bare_name, .. } => Err(SchemaError::NoMatch { reference, bare_name }),
            SelectionOutcome::SiblingOnly { suggestion, .. } => {
                Err(SchemaError::BareNameSiblingExists { reference, suggestion })
            }
            SelectionOutcome::Failed(SelectionFailure::Remote) => Err(SchemaError::RemoteUnsupported { reference }),
            SelectionOutcome::Failed(SelectionFailure::Resolution(source)) => {
                Err(SchemaError::Unresolved { reference, source })
            }
        }
    }
}

/// Selects the file `reference` names, as every schema consumer must.
///
/// `base_dir` is the folder of the document or schema that wrote the
/// reference, and `schema_roots` the searched schema-root folders in search
/// order. Probes are recorded into every running
/// [`observed::collect`](super::observed::collect) frame, so a caller whose
/// resolution later fails still reports them.
#[must_use]
pub fn select_schema_file(
    reference: &str,
    base_dir: &Path,
    schema_roots: &[PathBuf],
    request_context: &FileResolutionContext,
) -> SchemaFileSelection {
    let trimmed = reference.trim();
    let mut probes = Probes::default();
    let outcome = match select(trimmed, base_dir, schema_roots, request_context, None, &mut probes) {
        Ok(outcome) => outcome,
        Err(_) => unreachable!("an ungated selection admits every location"),
    };
    observed::record_probes(&probes.attempted);
    SchemaFileSelection {
        reference: trimmed.to_string(),
        outcome,
        attempted: probes.attempted,
    }
}

/// [`select_schema_file`], with `admit` asked about each location
/// immediately before it is probed. The search order and first-match stop
/// are the ungated selection's, so a location the ordered search never
/// reaches (a later schema root, a later candidate, the sibling of a name a
/// root holds) is never asked about. Only the import engine expanding a
/// template type's returned definition selects this way.
///
/// ## Errors
///
/// The first error `admit` returns, with the locations probed before it;
/// nothing further is probed.
pub(crate) fn select_schema_file_checked(
    reference: &str,
    base_dir: &Path,
    schema_roots: &[PathBuf],
    request_context: &FileResolutionContext,
    admit: &dyn Fn(&Path) -> Result<(), SchemaError>,
) -> Result<SchemaFileSelection, Refused> {
    let trimmed = reference.trim();
    let mut probes = Probes::default();
    let outcome = select(trimmed, base_dir, schema_roots, request_context, Some(admit), &mut probes);
    observed::record_probes(&probes.attempted);
    match outcome {
        Ok(outcome) => Ok(SchemaFileSelection {
            reference: trimmed.to_string(),
            outcome,
            attempted: probes.attempted,
        }),
        Err(error) => Err(Refused { error: Box::new(error), attempted: probes.attempted }),
    }
}

/// A gated selection stopped by its admission.
#[derive(Debug)]
pub(crate) struct Refused {
    /// What the admission returned.
    pub(crate) error: Box<SchemaError>,
    /// The locations probed before the refusal, in probe order.
    pub(crate) attempted: Vec<PathBuf>,
}

/// The admission asked about a location before it is probed; `None` admits
/// everything.
type Admit<'a> = Option<&'a dyn Fn(&Path) -> Result<(), SchemaError>>;

fn select(
    trimmed: &str,
    base_dir: &Path,
    schema_roots: &[PathBuf],
    request_context: &FileResolutionContext,
    admit: Admit<'_>,
    probes: &mut Probes,
) -> Result<SelectionOutcome, SchemaError> {
    let file_ref = match FileReference::new(trimmed) {
        Ok(file_ref) => file_ref,
        Err(error) => return Ok(SelectionOutcome::Failed(SelectionFailure::Resolution(error))),
    };
    if file_ref.class().kind == FileReferenceKind::Url {
        return Ok(SelectionOutcome::Failed(SelectionFailure::Remote));
    }
    #[cfg(not(feature = "file"))]
    {
        let _ = (base_dir, schema_roots, request_context, admit, probes);
        Ok(SelectionOutcome::Failed(SelectionFailure::Resolution(FileReferenceError::ResolutionUnavailable {
            reference: file_ref.raw().to_string(),
        })))
    }
    #[cfg(feature = "file")]
    {
        if !schema_roots.is_empty() && is_bare_name(&file_ref) {
            return select_in_roots(&file_ref, trimmed, base_dir, schema_roots, request_context, admit, probes);
        }
        let detailed = resolve_admitted(&file_ref, &request_context.for_cwd(base_dir), admit, probes)?;
        Ok(match detailed.into_convenience() {
            Ok(Some(path)) => SelectionOutcome::Found(path),
            Ok(None) => SelectionOutcome::Missing {
                bare_name: false,
                creation: first_planned_candidate(&file_ref, base_dir, request_context),
            },
            Err(error) => SelectionOutcome::Failed(SelectionFailure::Resolution(error)),
        })
    }
}

/// Resolves `file_ref` in `context`, asking `admit` before each probe, and
/// records what was probed (also when a refusal stopped the search). An open
/// buffer at an admitted candidate holding no file matches there.
#[cfg(feature = "file")]
fn resolve_admitted(
    file_ref: &FileReference,
    context: &FileResolutionContext,
    admit: Admit<'_>,
    probes: &mut Probes,
) -> Result<biscuit_file::DetailedResolution, SchemaError> {
    let mut admitted = Vec::new();
    let detailed = file_ref.resolve_detailed_with_presence(
        context,
        |path| {
            if let Some(admit) = admit {
                admit(path)?;
            }
            admitted.push(path.to_path_buf());
            Ok(())
        },
        buffers::is_open,
    );
    match detailed {
        Ok(detailed) => {
            probes.extend(detailed.probed_paths());
            Ok(detailed)
        }
        Err(refusal) => {
            // A direct reference probed every location admitted before the
            // refused one; a recursive one walks nothing until all its roots
            // are admitted.
            if !file_ref.class().recursive {
                probes.extend(admitted.into_iter());
            }
            Err(refusal)
        }
    }
}

/// Tries a bare name against each schema root in order; the first root that
/// holds it wins.
///
/// Each root is tried with the name spelled as an explicit `./name`, resolved
/// in the request context derived onto that root. An explicit relative
/// reference has exactly one candidate, the root's own entry, so the root
/// order decides the winner. The derivation keeps the request's home and
/// environment and reads no ambient state; it crosses the trust boundary
/// because a schema root (configuration, `~`) may lie outside the request's
/// repository. A root that cannot be checked fails the selection rather than
/// letting a later root's file win. The sibling beside the referencing
/// document is checked (and admitted) only once every root missed.
#[cfg(feature = "file")]
fn select_in_roots(
    file_ref: &FileReference,
    trimmed: &str,
    base_dir: &Path,
    schema_roots: &[PathBuf],
    request_context: &FileResolutionContext,
    admit: Admit<'_>,
    probes: &mut Probes,
) -> Result<SelectionOutcome, SchemaError> {
    let explicit = match FileReference::new(&format!("./{}", file_ref.raw())) {
        Ok(explicit) => explicit,
        Err(error) => return Ok(SelectionOutcome::Failed(SelectionFailure::Resolution(error))),
    };
    for root in schema_roots {
        let detailed =
            resolve_admitted(&explicit, &request_context.for_trusted_external_cwd(root), admit, probes)?;
        match detailed.into_convenience() {
            Ok(Some(path)) => return Ok(SelectionOutcome::Found(path)),
            Ok(None) => {}
            Err(error) => return Ok(SelectionOutcome::Failed(SelectionFailure::Resolution(error))),
        }
    }
    let creation = schema_roots[0].join(trimmed);
    // The author likely meant a relative reference, not a schema-root lookup.
    let sibling = base_dir.join(trimmed);
    if let Some(admit) = admit {
        admit(&sibling)?;
    }
    if sibling.is_file() {
        return Ok(SelectionOutcome::SiblingOnly {
            sibling,
            suggestion: format!("./{trimmed}"),
            creation,
        });
    }
    Ok(SelectionOutcome::Missing { bare_name: true, creation: Some(creation) })
}

/// The first candidate `file_ref` plans from `base_dir`, without probing:
/// where a missing reference would be created.
#[cfg(feature = "file")]
fn first_planned_candidate(
    file_ref: &FileReference,
    base_dir: &Path,
    request_context: &FileResolutionContext,
) -> Option<PathBuf> {
    let plan = file_ref
        .candidate_plan_with_order(&request_context.for_cwd(base_dir), biscuit_file::CandidatePlanOrder::Resolution)
        .ok()?;
    let first = plan.into_iter().next()?;
    let path = first.path();
    Some(if path.is_absolute() { path.to_path_buf() } else { base_dir.join(path) })
}

/// Returns `true` when a parsed reference is a non-recursive implicit-relative
/// name with exactly one portable path component: the form
/// [`select_schema_file`] looks up in the schema roots.
pub fn is_bare_name(file_ref: &FileReference) -> bool {
    let class = file_ref.class();
    let portable = file_ref.raw().replace('\\', "/");
    let mut components = Path::new(&portable).components();
    let is_single_component =
        matches!(components.next(), Some(Component::Normal(_))) && components.next().is_none();
    class.kind == FileReferenceKind::ImplicitRelative && !class.recursive && is_single_component
}

/// The probed locations in order, each kept at its first occurrence.
#[derive(Default)]
struct Probes {
    seen: BTreeSet<PathBuf>,
    attempted: Vec<PathBuf>,
}

impl Probes {
    #[cfg_attr(not(feature = "file"), allow(dead_code))]
    fn extend(&mut self, probed: impl Iterator<Item = PathBuf>) {
        for path in probed {
            if self.seen.insert(path.clone()) {
                self.attempted.push(path);
            }
        }
    }
}
