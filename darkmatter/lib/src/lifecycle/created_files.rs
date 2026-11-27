//! The files a lifecycle stack writes, found without running it.
//!
//! A dry run never runs an event, yet a real run's `initialize` can create a
//! file that a later read needs (`ensure_file: generated/notes.md` before a
//! `::file generated/notes.md`), or append to one that exists. [`created_files`]
//! lists the files a parsed stack's file-writing side effects name, literal
//! or templated, and [`CreatedFile::target`] resolves each literal one exactly
//! as the effect resolves it when it runs ([`CreatedFile::target_as`] does
//! the same for a template a host resolved, [`CreatedFile::pattern`] bounds
//! one it could not), so a host can declare them to its dry run
//! ([`PassiveSession::declare_initializer_write`](crate::markdown::compose::passive::PassiveSession::declare_initializer_write))
//! and leave a read whose verdict the write changes unchecked.

use std::path::{Path, PathBuf};

use biscuit_file::{FileReference, FileResolutionContext};

use super::actions::{LifecycleActionKind, LifecycleNode, LifecyclePath};
use super::executor::is_template_operand;
use crate::markdown::compose::expression::{Expr, ExpressionFinder};
use crate::markdown::compose::passive::{InitializerChange, WritePattern};

/// The side effects that create their file (operand 0) when it is missing.
/// The frontmatter verbs need an existing file, and `ensure_dir` creates a
/// folder, which no file read finds.
pub const FILE_CREATING_VERBS: &[&str] = &["ensure_file", "append_line", "append_jsonl"];

/// A file a lifecycle stack creates, as authored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatedFile {
    /// The side effect (`ensure_file`, `append_line`, `append_jsonl`).
    pub verb: String,
    /// The file operand as written (`generated/notes.md`, `{{ target }}`).
    pub reference: String,
    /// Whether the operand is a template, resolved only when the event
    /// runs: [`Self::target`] is `None` for one.
    pub template: bool,
    /// Whether the action sits under a `when:` branch, so the run may skip it.
    pub conditional: bool,
    /// The action's authored path (`initialize[0].ensure_file`).
    pub path: LifecyclePath,
}

impl CreatedFile {
    /// The file the effect creates when the document at `source_path` runs
    /// it in `context` under an engine whose mutation root is
    /// `mutation_root`: the file the reference resolves to, else its first
    /// candidate, as the effect selects its target. `None` where the effect
    /// creates nothing: a reference with no local target (a URL, an
    /// unmatched recursive search, a reference that does not parse or
    /// resolve) or a target the effect refuses as outside `mutation_root`.
    /// A template operand names no file before it runs, so it is `None`
    /// here; see [`Self::target_as`].
    #[must_use]
    pub fn target(&self, source_path: &Path, context: &FileResolutionContext, mutation_root: &Path) -> Option<PathBuf> {
        if self.template {
            return None;
        }
        self.target_as(&self.reference, source_path, context, mutation_root)
    }

    /// [`Self::target`] for the operand's text `resolved`: a template's
    /// value, as the event would interpolate it.
    #[must_use]
    pub fn target_as(&self, resolved: &str, source_path: &Path, context: &FileResolutionContext, mutation_root: &Path) -> Option<PathBuf> {
        if resolved.trim().is_empty() {
            return None;
        }
        let reference = FileReference::new(resolved).ok()?;
        if reference.class().kind == biscuit_file::FileReferenceKind::Url {
            return None;
        }
        let target = super::executor::select_mutation_target(&reference, &context.for_source(source_path)).ok()?;
        crate::effects::fs_write::ensure_within(mutation_root, &target).ok()
    }

    /// What the effect does to its file.
    #[must_use]
    pub fn change(&self) -> InitializerChange {
        if self.verb == "ensure_file" { InitializerChange::CreatesIfAbsent } else { InitializerChange::CreatesOrAppends }
    }

    /// The locations a template operand may name, for a host that cannot
    /// resolve it before the event runs: the operand's literal text, placed
    /// as the effect would place it, with anything where a `{{ … }}` span
    /// stands. An operand that starts with a span may name any location
    /// below `mutation_root`, the only place the effect writes. `None` for
    /// a literal operand, and where the literal text already places the
    /// file outside `mutation_root`.
    #[must_use]
    pub fn pattern(&self, source_path: &Path, context: &FileResolutionContext, mutation_root: &Path) -> Option<WritePattern> {
        if !self.template {
            return None;
        }
        // Each span is replaced by a name no author writes, the result is
        // placed as the effect places a file, and the names are cut out
        // again: the pieces between them are the fixed text of every
        // location the operand can name.
        const HOLE: &str = "dm-initializer-hole-7c1f";
        let spans = ExpressionFinder::find_all_plain(&self.reference);
        let starts_with_span = spans.first().is_some_and(|span| self.reference[..span.start].trim().is_empty());
        let mut filled = String::new();
        let mut at = 0;
        for span in &spans {
            filled.push_str(&self.reference[at..span.start]);
            filled.push_str(HOLE);
            at = span.end;
        }
        filled.push_str(&self.reference[at..]);
        let placed = self.target_as(&filled, source_path, context, mutation_root)?;
        let placed = placed.to_string_lossy();
        let mut pieces: Vec<String> = placed.split(HOLE).map(str::to_string).collect();
        if starts_with_span || pieces.len() == 1 {
            let mut root = mutation_root.to_string_lossy().to_string();
            if !root.ends_with(std::path::MAIN_SEPARATOR) {
                root.push(std::path::MAIN_SEPARATOR);
            }
            pieces[0] = root;
            if pieces.len() == 1 {
                pieces.push(String::new());
            }
        }
        WritePattern::new(pieces)
    }
}

/// Every file-creating side effect in `nodes` ([`FILE_CREATING_VERBS`]) whose
/// file operand is text, in authored order, through both branches of every
/// conditional. A `{{ … }}` operand is listed as a
/// [`CreatedFile::template`]; a whole-value expression operand is left out.
///
/// The walk is structural: no guard is evaluated, so an action under a
/// `when:` is listed, marked [`CreatedFile::conditional`].
#[must_use]
pub fn created_files(nodes: &[LifecycleNode]) -> Vec<CreatedFile> {
    let mut found = Vec::new();
    collect(nodes, false, &mut found);
    found
}

fn collect(nodes: &[LifecycleNode], conditional: bool, found: &mut Vec<CreatedFile>) {
    for node in nodes {
        match node {
            LifecycleNode::Action(action) => {
                let (verb, args) = match &action.kind {
                    LifecycleActionKind::SideEffect(effect) => (effect.verb.as_str(), &effect.args),
                    LifecycleActionKind::ExpressionFunction(function) => (function.function.as_str(), &function.args),
                    _ => continue,
                };
                if !FILE_CREATING_VERBS.contains(&verb) {
                    continue;
                }
                let (reference, template) = match args.first() {
                    Some(Expr::StringLiteral(reference)) => (reference.clone(), is_template_operand(&args[0])),
                    // A whole-value `{{ … }}` operand reaches the stack
                    // parsed; its text is that one span again.
                    Some(expr) => (format!("{{{{ {expr} }}}}"), true),
                    None => continue,
                };
                if reference.trim().is_empty() {
                    continue;
                }
                found.push(CreatedFile {
                    verb: verb.to_string(),
                    reference,
                    template,
                    conditional,
                    path: action.path.clone(),
                });
            }
            LifecycleNode::Conditional(branch) => {
                collect(&branch.then, true, found);
                collect(&branch.otherwise, true, found);
            }
        }
    }
}
