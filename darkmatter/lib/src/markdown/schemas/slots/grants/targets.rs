//! Passive pre-flight collection of the reads a template's slots will make.
//!
//! Every read-side call of every slot is visited, in both branches of `?:`
//! and every `||` operand, as the shell pre-flight counts commands. A call
//! whose target argument is a literal, or an expression of literals and
//! pure functions only (folded here, which reads nothing), yields a
//! [`ReadTarget`]; a URL is taken as written, and a local path only when it
//! is absolute or explicitly relative (`./`, `../`), joined to the slot's
//! defining folder, because any other spelling is resolved by candidate
//! probing, which pre-flight never does. Every other call is
//! [`CollectedRead::Computed`] and is judged when the slot evaluates.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use biscuit_file::FileResolutionContext;
use serde_json::Value;
use simplified_schema::expression::{SpannedExpr, SpannedExprKind, parse_spanned};
use simplified_schema::simplified::Slot;

use crate::markdown::compose::conditions::ShortcutLookup;
use crate::markdown::compose::expression::call_check::visit_call_nodes;
use crate::markdown::compose::expression::functions::{SlotClass, read_fact, slot_class};
use crate::markdown::compose::expression::read_gate::{ReadLocation, ReadTarget};
use crate::markdown::compose::expression::{ResolutionContext, evaluate};
use crate::markdown::schemas::slots::evaluator::DenyAllReads;

/// One read-side call found in a slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CollectedRead {
    /// The read's target is known before evaluation.
    Known(ReadTarget),
    /// The target is computed; the gate judges it during evaluation.
    Computed { function: String },
}

/// The reads of one slot, with where the slot was written.
#[derive(Debug, Clone)]
pub(crate) struct SlotReads {
    pub(crate) slot: Slot,
    /// The slot's defining file, or for an inline schema the folder its
    /// document context reads relative to.
    pub(crate) defining: PathBuf,
    pub(crate) reads: Vec<CollectedRead>,
}

/// The reads every slot of `pending` will make, in slot order, judged in
/// the request `context`. A slot whose text does not parse yields none
/// (admission reports it).
#[cfg(test)]
pub(crate) fn collect_targets(pending: &simplified_schema::slots::PendingSchema, context: &FileResolutionContext) -> Vec<SlotReads> {
    collect_slot_targets(pending.slots(), context)
}

/// The reads each of `slots` will make, in order; see [`collect_targets`].
pub(crate) fn collect_slot_targets(slots: &[Slot], context: &FileResolutionContext) -> Vec<SlotReads> {
    let base_dir = context.base_dir();
    slots
        .iter()
        .map(|slot| {
            let defining = slot.origin.clone().unwrap_or_else(|| base_dir.to_path_buf());
            let folder = if slot.origin.is_some() {
                defining.parent().map_or_else(|| base_dir.to_path_buf(), Path::to_path_buf)
            } else {
                base_dir.to_path_buf()
            };
            let reads = parse_spanned(&slot.expression)
                .map(|parsed| reads_of(&parsed, &folder, context))
                .unwrap_or_default();
            SlotReads { slot: slot.clone(), defining, reads }
        })
        .collect()
}

fn reads_of(parsed: &SpannedExpr, folder: &Path, context: &FileResolutionContext) -> Vec<CollectedRead> {
    let mut reads = Vec::new();
    visit_call_nodes(parsed, &mut |node| {
        let SpannedExprKind::FunctionCall { name, args } = &node.kind else {
            return;
        };
        let Some(fact) = read_fact(name) else {
            return;
        };
        let function = name.to_ascii_lowercase();
        let known = args
            .first()
            .and_then(|argument| fold_pure(argument, context))
            .and_then(|value| value.as_str().map(str::to_string))
            .and_then(|written| location_of(&written, folder, fact.remote));
        reads.push(match known {
            Some(location) => {
                CollectedRead::Known(ReadTarget { function, kind: fact.kind, location, observed_hash: None })
            }
            None => CollectedRead::Computed { function },
        });
    });
    reads
}

/// `written`'s location, when pre-flight can name it without probing.
fn location_of(written: &str, folder: &Path, remote: bool) -> Option<ReadLocation> {
    if let Ok(url) = url::Url::parse(written)
        && matches!(url.scheme(), "http" | "https")
    {
        return remote.then_some(ReadLocation::Remote(url));
    }
    let path = Path::new(written);
    if path.is_absolute() {
        return Some(ReadLocation::Local(path.to_path_buf()));
    }
    let explicit = written.starts_with("./") || written.starts_with("../") || written.starts_with(".\\") || written.starts_with("..\\");
    explicit.then(|| ReadLocation::Local(lexically_joined(folder, path)))
}

/// `folder` joined with `relative`, `.` and `..` applied by name.
fn lexically_joined(folder: &Path, relative: &Path) -> PathBuf {
    let mut joined = folder.to_path_buf();
    for component in relative.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                joined.pop();
            }
            other => joined.push(other.as_os_str()),
        }
    }
    joined
}

/// The value of `expr` when it is built only from literals and pure
/// functions; `None` otherwise. Evaluating such an expression reads nothing.
fn fold_pure(expr: &SpannedExpr, context: &FileResolutionContext) -> Option<Value> {
    if !is_pure(expr) {
        return None;
    }
    let mut resolution = ResolutionContext::new(context.clone());
    resolution.slot_gate = Some(Arc::new(DenyAllReads));
    let lookup = ShortcutLookup::in_context(&Value::Null, &resolution);
    evaluate(&expr.erase(), &lookup).ok()
}

fn is_pure(expr: &SpannedExpr) -> bool {
    match &expr.kind {
        SpannedExprKind::StringLiteral(_) | SpannedExprKind::NumberLiteral(_) | SpannedExprKind::BoolLiteral(_) => {
            true
        }
        SpannedExprKind::Variable(_) | SpannedExprKind::CallbackCall { .. } => false,
        SpannedExprKind::ArrayLiteral(items) => items.iter().all(is_pure),
        SpannedExprKind::ObjectLiteral(entries) => entries.iter().all(|(_, value)| is_pure(value)),
        SpannedExprKind::UnaryNot(inner)
        | SpannedExprKind::UnaryMinus(inner)
        | SpannedExprKind::Paren(inner)
        | SpannedExprKind::MemberAccess { base: inner, .. } => is_pure(inner),
        SpannedExprKind::Binary { left, right, .. }
        | SpannedExprKind::Comparison { left, right, .. }
        | SpannedExprKind::LogicalOr { left, right }
        | SpannedExprKind::Index { base: left, index: right } => is_pure(left) && is_pure(right),
        SpannedExprKind::Ternary { condition, then_branch, else_branch } => {
            is_pure(condition) && is_pure(then_branch) && is_pure(else_branch)
        }
        SpannedExprKind::FunctionCall { name, args } => {
            slot_class(name) == Some(SlotClass::Pure) && args.iter().all(is_pure)
        }
    }
}
