//! Descriptors for the reserved expression roots.
//!
//! A reserved root is a top-level name no frontmatter key or injected global
//! may shadow. `current` and `current_env` are lazy mirrors of `ctx` and `env`
//! (spec R30–R32): same keys and value types, observed when a reference is
//! evaluated instead of at launch. The evaluator resolves them through the
//! request's refresh authority
//! ([`CurrentAuthority`](crate::markdown::compose::CurrentAuthority)); this
//! catalog is the descriptor projection those roots publish, and enumerating it
//! observes nothing.

use crate::markdown::compose::context::{ContextVariableDescriptor, context_variable_descriptors};

/// When a root's members are observed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootEvaluation {
    /// Fixed for the whole composition request.
    Eager,
    /// Observed each time an expression evaluation first reads a key, and
    /// memoized only within that one expression evaluation.
    Lazy,
}

/// The key set and value types a root exposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootMembers {
    /// The document's own frontmatter values.
    DocumentFrontmatter,
    /// Every `ctx.*` variable descriptor, by name and with the same type and
    /// optionality ([`context_variable_descriptors`]).
    ContextVariables,
    /// Environment variables: string values, open key set, `null` when unset.
    EnvironmentVariables,
}

/// Descriptor for one reserved expression root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReservedRootDescriptor {
    /// The root name as written in expressions.
    pub name: &'static str,
    /// When members are observed.
    pub evaluation: RootEvaluation,
    /// The eager root this one mirrors key for key, if any.
    pub mirrors: Option<&'static str>,
    /// The key set and value types.
    pub members: RootMembers,
    /// Short description of the root.
    pub description: &'static str,
}

impl ReservedRootDescriptor {
    /// The cataloged member descriptors for a root whose keys are the `ctx`
    /// variables; empty for open or document-defined key sets. Enumerating
    /// members reads descriptors only and never materializes a value.
    pub fn context_members(&self) -> &'static [ContextVariableDescriptor] {
        match self.members {
            RootMembers::ContextVariables => context_variable_descriptors(),
            RootMembers::DocumentFrontmatter | RootMembers::EnvironmentVariables => &[],
        }
    }
}

const RESERVED_ROOTS: &[ReservedRootDescriptor] = &[
    ReservedRootDescriptor {
        name: "doc",
        evaluation: RootEvaluation::Eager,
        mirrors: None,
        members: RootMembers::DocumentFrontmatter,
        description: "The document's frontmatter values.",
    },
    ReservedRootDescriptor {
        name: "ctx",
        evaluation: RootEvaluation::Eager,
        mirrors: None,
        members: RootMembers::ContextVariables,
        description: "Runtime context captured once at the start of execution.",
    },
    ReservedRootDescriptor {
        name: "env",
        evaluation: RootEvaluation::Eager,
        mirrors: None,
        members: RootMembers::EnvironmentVariables,
        description: "Environment variables from the snapshot frozen at the start of execution.",
    },
    ReservedRootDescriptor {
        name: "current",
        evaluation: RootEvaluation::Lazy,
        mirrors: Some("ctx"),
        members: RootMembers::ContextVariables,
        description: "The ctx keys, each observed when referenced. Request-owned identity, \
            source path, and resolution anchors stay fixed; mutable facts such as the branch refresh.",
    },
    ReservedRootDescriptor {
        name: "current_env",
        evaluation: RootEvaluation::Lazy,
        mirrors: Some("env"),
        members: RootMembers::EnvironmentVariables,
        description: "The env keys, each read from the live process environment when referenced.",
    },
];

/// All reserved expression roots, in display order.
pub fn reserved_root_descriptors() -> &'static [ReservedRootDescriptor] {
    RESERVED_ROOTS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roots_are_unique_and_include_both_lazy_mirrors() {
        let names: Vec<&str> = reserved_root_descriptors().iter().map(|root| root.name).collect();
        assert_eq!(names, ["doc", "ctx", "env", "current", "current_env"]);
    }

    /// A lazy root mirrors an eager root with the identical member set, so
    /// `current.<key>` exists exactly when `ctx.<key>` does, with no nested
    /// `current.ctx` or `current.env` shape.
    #[test]
    fn lazy_roots_mirror_an_eager_root_member_for_member() {
        for root in reserved_root_descriptors() {
            match (root.evaluation, root.mirrors) {
                (RootEvaluation::Eager, None) => {}
                (RootEvaluation::Lazy, Some(mirrored)) => {
                    let eager = reserved_root_descriptors()
                        .iter()
                        .find(|candidate| candidate.name == mirrored)
                        .unwrap_or_else(|| panic!("{} mirrors unknown root {mirrored}", root.name));
                    assert_eq!(eager.evaluation, RootEvaluation::Eager);
                    assert_eq!(eager.members, root.members, "{} members", root.name);
                    assert!(
                        std::ptr::eq(eager.context_members(), root.context_members()),
                        "{} must enumerate the same descriptor slice",
                        root.name
                    );
                }
                other => panic!("{} has an inconsistent evaluation/mirror pair: {other:?}", root.name),
            }
        }
        let current = reserved_root_descriptors().iter().find(|root| root.name == "current").unwrap();
        let keys: Vec<&str> = current.context_members().iter().map(|member| member.name).collect();
        assert!(keys.contains(&"branch") && keys.contains(&"recent_commits"));
        assert!(!keys.contains(&"ctx") && !keys.contains(&"env"));
    }
}
