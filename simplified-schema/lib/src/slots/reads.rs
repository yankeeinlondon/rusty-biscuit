//! The read boundary of a template type: every schema file, example, and
//! path probe a returned definition causes is admitted by the host before it
//! happens and reported to it afterward.
//!
//! A template type's value is type syntax, so it can name imports
//! (`Person@./types.yaml`) and examples the schema author never wrote. Those
//! reads belong to the slot that returned the definition, not to the schema:
//! a static import's ability to read a file never authorizes them. While a
//! returned definition expands, the import engine asks
//! [`TemplateReads::admit`] for each location immediately before the ordered
//! file selection probes it (so a location the first-match search never
//! reaches is never asked about) and for each file before its text is used
//! (from the disk, an open buffer, or a parse memo), and calls
//! [`TemplateReads::observe`] after each read or probe, also when it failed.
//!
//! The first refusal is sticky: it is recorded for the running
//! [`with_template_reads`] scope and the resolution reports it as
//! [`SchemaError::SlotReadRefused`] even where the engine's discovery
//! swallowed the error, so a denied read can never become a schema.

use std::cell::RefCell;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use super::SlotKey;
use super::host::SlotAdmissionError;
use crate::errors::SchemaError;
use crate::simplified::Slot;

/// What a host decides and learns about the reads a template type's returned
/// definition causes ([`super::SlotHost::template_reads`]).
pub trait TemplateReads {
    /// Whether `read`, caused by the definition `slot` returned, may happen.
    /// Asked before the path is probed or its text used; must not read it.
    ///
    /// ## Errors
    ///
    /// Why the read is refused; resolution fails with
    /// [`SchemaError::SlotReadRefused`] carrying it unchanged.
    fn admit(&self, slot: &Slot, read: &SchemaRead<'_>) -> Result<(), SlotAdmissionError>;

    /// Reports an admitted read or probe after it happened, whether or not
    /// it succeeded, so the host can record it as a dependency of the
    /// resolution.
    fn observe(&self, slot: &Slot, outcome: &SchemaReadOutcome<'_>);
}

/// One read a returned definition asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchemaRead<'a> {
    /// The file or probed location.
    pub path: &'a Path,
    /// Whether the text is used or only the location checked.
    pub kind: SchemaReadKind,
}

/// What a read does with its path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SchemaReadKind {
    /// The file's text is used: an imported schema file or an example.
    Content,
    /// Only whether the location holds a file is checked, while selecting
    /// the file a reference names.
    Probe,
}

/// What one admitted read or probe found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchemaReadOutcome<'a> {
    /// The file or probed location.
    pub path: &'a Path,
    /// The read's kind.
    pub kind: SchemaReadKind,
    /// The bytes read, for a [`SchemaReadKind::Content`] read that
    /// succeeded; `None` for a probe or a failed read.
    pub bytes: Option<&'a [u8]>,
    /// Whether a file (or an open buffer) is at `path`.
    pub exists: bool,
}

/// A [`TemplateReads`] that refuses every read: for a host that never lets a
/// returned definition import or reference a file. A returned definition
/// that names none still resolves.
#[derive(Debug, Clone, Copy, Default)]
pub struct RefuseTemplateReads;

impl TemplateReads for RefuseTemplateReads {
    fn admit(&self, _: &Slot, read: &SchemaRead<'_>) -> Result<(), SlotAdmissionError> {
        Err(SlotAdmissionError {
            code: None,
            message: format!("a template type's definition may not read `{}`", read.path.display()),
            span: None,
        })
    }

    fn observe(&self, _: &Slot, _: &SchemaReadOutcome<'_>) {}
}

thread_local! {
    /// The host's read hooks for the running [`with_template_reads`] scope.
    static READS: RefCell<Option<Rc<dyn TemplateReads>>> = RefCell::new(None);
    /// The template slot whose returned definition is expanding.
    static EXPANDING: RefCell<Option<Slot>> = const { RefCell::new(None) };
    /// The first refusal of the running scope.
    static REFUSAL: RefCell<Option<(Slot, SlotAdmissionError)>> = const { RefCell::new(None) };
    /// The reads the running scope admitted, so each is asked once per
    /// resolution however many expansions reach it.
    static ADMITTED: RefCell<HashSet<(SlotKey, PathBuf, SchemaReadKind)>> = RefCell::new(HashSet::new());
}

/// Runs `resolve` with `reads` judging every read a returned definition
/// causes, restoring what was in force before, even when `resolve` panics.
/// Returns `resolve`'s result with the first refusal the scope recorded.
pub fn with_template_reads<T>(
    reads: Rc<dyn TemplateReads>,
    resolve: impl FnOnce() -> T,
) -> (T, Option<(Slot, SlotAdmissionError)>) {
    type Saved = (
        Option<Rc<dyn TemplateReads>>,
        Option<(Slot, SlotAdmissionError)>,
        HashSet<(SlotKey, PathBuf, SchemaReadKind)>,
    );
    struct Restore(Option<Saved>);
    impl Drop for Restore {
        fn drop(&mut self) {
            if let Some((reads, refusal, admitted)) = self.0.take() {
                READS.with_borrow_mut(|active| *active = reads);
                REFUSAL.with_borrow_mut(|active| *active = refusal);
                ADMITTED.with_borrow_mut(|active| *active = admitted);
            }
        }
    }
    let previous = (
        READS.with_borrow_mut(|active| active.replace(reads)),
        REFUSAL.with_borrow_mut(Option::take),
        ADMITTED.with_borrow_mut(std::mem::take),
    );
    let restore = Restore(Some(previous));
    let result = resolve();
    let refusal = REFUSAL.with_borrow_mut(Option::take);
    drop(restore);
    (result, refusal)
}

/// Runs `expand` as the expansion of `slot`'s returned definition: every
/// import engine built inside asks the scope's hooks on `slot`'s behalf.
pub(crate) fn expanding<T>(slot: &Slot, expand: impl FnOnce() -> T) -> T {
    struct Restore(Option<Slot>);
    impl Drop for Restore {
        fn drop(&mut self) {
            let previous = self.0.take();
            EXPANDING.with_borrow_mut(|active| *active = previous);
        }
    }
    let previous = EXPANDING.with_borrow_mut(|active| active.replace(slot.clone()));
    let _restore = Restore(previous);
    expand()
}

/// The first refusal recorded in the running scope, left in place.
pub(crate) fn recorded_refusal() -> Option<(Slot, SlotAdmissionError)> {
    REFUSAL.with_borrow(Clone::clone)
}

/// The gate of the expansion running on this thread: `Some` only inside an
/// [`expanding`] scope with hooks installed. Outside one, reads are not a
/// returned definition's and the engine behaves as it always has.
pub(crate) fn current_gate() -> Option<ReadGate> {
    let slot = EXPANDING.with_borrow(Clone::clone)?;
    let reads = READS.with_borrow(Clone::clone)?;
    Some(ReadGate { slot, reads })
}

/// The hooks of one returned definition's expansion, bound to its slot.
#[derive(Clone)]
pub(crate) struct ReadGate {
    slot: Slot,
    reads: Rc<dyn TemplateReads>,
}

impl ReadGate {
    /// Asks the host to admit `path`; a refusal is recorded (the first one
    /// sticks) and returned as [`SchemaError::SlotReadRefused`].
    pub(crate) fn admit(&self, path: &Path, kind: SchemaReadKind) -> Result<(), SchemaError> {
        if let Some((slot, refusal)) = recorded_refusal() {
            // Nothing more is read once a read was refused.
            return Err(SchemaError::SlotReadRefused { slot: Box::new(slot), refusal });
        }
        let asked = (SlotKey::of(&self.slot), path.to_path_buf(), kind);
        if ADMITTED.with_borrow(|admitted| admitted.contains(&asked)) {
            return Ok(());
        }
        self.reads.admit(&self.slot, &SchemaRead { path, kind }).map_err(|refusal| {
            REFUSAL.with_borrow_mut(|recorded| {
                recorded.get_or_insert_with(|| (self.slot.clone(), refusal.clone()));
            });
            SchemaError::SlotReadRefused { slot: Box::new(self.slot.clone()), refusal }
        })?;
        ADMITTED.with_borrow_mut(|admitted| admitted.insert(asked));
        Ok(())
    }

    /// Reports one read or probe to the host.
    pub(crate) fn observe(&self, path: &Path, kind: SchemaReadKind, bytes: Option<&[u8]>, exists: bool) {
        self.reads.observe(&self.slot, &SchemaReadOutcome { path, kind, bytes, exists });
    }

    /// Reads `path`'s bytes as the resolver reads a schema source, after
    /// admitting it, and reports the read whether or not it succeeded.
    pub(crate) fn read(&self, path: &Path) -> Result<Vec<u8>, SchemaError> {
        self.admit(path, SchemaReadKind::Content)?;
        match crate::resolve::buffers::read(path) {
            Ok(bytes) => {
                self.observe(path, SchemaReadKind::Content, Some(&bytes), true);
                Ok(bytes)
            }
            Err(source) => {
                self.observe(path, SchemaReadKind::Content, None, false);
                Err(SchemaError::Io { path: path.to_path_buf(), source })
            }
        }
    }
}
