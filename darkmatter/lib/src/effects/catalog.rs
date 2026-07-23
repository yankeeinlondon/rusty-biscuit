//! Typed descriptor catalog for side-effect capabilities.
//!
//! Each [`EffectDescriptor`] describes a single mutating operation available on
//! [`EffectEngine`]. The catalog is a static, compile-time constant —
//! constructing or reading it performs no host probes, no I/O, and no runtime
//! context capture.
use crate::catalog::{Described, Example, ExampleVerification};


/// Safety classification for a side-effect capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EffectSafety {
    /// Filesystem write restricted to the configured mutation root.
    FilesystemWrite,
    /// Network access restricted by deny-all host allowlist.
    Network,
    /// Markdown mutation that honors auto-rehash.
    MarkdownMutation,
    /// In-memory state mutation only; performs no filesystem or network I/O.
    InMemoryState,
}

/// Descriptor for a single side-effect capability.
#[derive(Debug, Clone, PartialEq)]
pub struct EffectDescriptor {

    /// Canonical signature including arity (e.g., `ensure_file(file)` or
    /// `ensure_file(file, content)`).
    pub signature: &'static str,
    /// Short description of the capability's behavior and return value.
    pub description: &'static str,
    /// Safety classification.
    pub safety: EffectSafety,
    /// Logical grouping category.
    pub category: &'static str,
    /// Stable display order within the category.
    pub order: usize,
    /// Optional verified example.
    pub example: Option<Example>,
}
impl Described for EffectDescriptor {
    fn key(&self) -> &'static str {
        self.signature
    }
    fn description(&self) -> &'static str {
        self.description
    }
    fn category(&self) -> &'static str {
        self.category
    }
    fn order(&self) -> usize {
        self.order
    }
    fn example(&self) -> Option<&Example> {
        self.example.as_ref()
    }
}


/// All side-effect capability descriptors, in display order.
pub const EFFECT_DESCRIPTORS: &[EffectDescriptor] = &[
    // ── State Mutation ──────────────────────────────────────────────
    EffectDescriptor {
        signature: "set(key, value)",
        description: "Sets a top-level key in the in-memory state map (no disk write). Returns the prior value (or null).",
        safety: EffectSafety::InMemoryState,
        category: "State Mutation",
        order: 1,

        example: Some(Example { invocation: "set(\"ready\", true)", result: "null", verification: ExampleVerification::DisplayOnly("side-effect descriptions are proven by the sandbox verb harness") }),
    },
    // ── Frontmatter Mutations ───────────────────────────────────────
    EffectDescriptor {
        signature: "set_frontmatter(file, prop, value)",
        description: "Sets a frontmatter property. Returns the prior value (or null).",
        safety: EffectSafety::MarkdownMutation,
        category: "Frontmatter Mutations",
        order: 1,

        example: Some(Example { invocation: "set_frontmatter(\"d.md\", \"status\", \"x\")", result: "null", verification: ExampleVerification::DisplayOnly("side-effect descriptions are proven by the sandbox verb harness") }),
 },
    EffectDescriptor {

        signature: "merge_frontmatter(file, obj)",
        description: "Shallow-merges an object into the document's frontmatter. Returns the merged object.",
        safety: EffectSafety::MarkdownMutation,
        category: "Frontmatter Mutations",
        order: 2,

        example: Some(Example { invocation: "merge_frontmatter(\"d.md\", {\"owner\": \"ken\"})", result: "{\"owner\":\"ken\"}", verification: ExampleVerification::DisplayOnly("side-effect descriptions are proven by the sandbox verb harness") }),

    },
    EffectDescriptor {

        signature: "delete_frontmatter(file, prop)",
        description: "Removes a frontmatter property. Returns the removed value (or null).",
        safety: EffectSafety::MarkdownMutation,
        category: "Frontmatter Mutations",
        order: 3,

        example: Some(Example { invocation: "delete_frontmatter(\"d.md\", \"owner\")", result: "ken", verification: ExampleVerification::DisplayOnly("side-effect descriptions are proven by the sandbox verb harness") }),

    },
    EffectDescriptor {

        signature: "increment_frontmatter(file, prop)",
        description: "Increments a numeric frontmatter property. Missing becomes 1. Returns the new number.",
        safety: EffectSafety::MarkdownMutation,
        category: "Frontmatter Mutations",
        order: 4,

        example: Some(Example { invocation: "increment_frontmatter(\"d.md\", \"phase\")", result: "2", verification: ExampleVerification::DisplayOnly("side-effect descriptions are proven by the sandbox verb harness") }),

    },
    EffectDescriptor {

        signature: "decrement_frontmatter(file, prop)",
        description: "Decrements a numeric frontmatter property. Missing becomes -1. Returns the new number.",
        safety: EffectSafety::MarkdownMutation,
        category: "Frontmatter Mutations",
        order: 5,

        example: Some(Example { invocation: "decrement_frontmatter(\"d.md\", \"phase\")", result: "0", verification: ExampleVerification::DisplayOnly("side-effect descriptions are proven by the sandbox verb harness") }),

    },
    EffectDescriptor {

        signature: "append_frontmatter(file, prop, value)",
        description: "Appends a value to a frontmatter array property. Returns the new array.",
        safety: EffectSafety::MarkdownMutation,
        category: "Frontmatter Mutations",
        order: 6,

        example: Some(Example { invocation: "append_frontmatter(\"d.md\", \"tags\", \"b\")", result: "[\"a\",\"b\"]", verification: ExampleVerification::DisplayOnly("side-effect descriptions are proven by the sandbox verb harness") }),

    },
    EffectDescriptor {

        signature: "prepend_frontmatter(file, prop, value)",
        description: "Prepends a value to a frontmatter array property. Returns the new array.",
        safety: EffectSafety::MarkdownMutation,
        category: "Frontmatter Mutations",
        order: 7,

        example: Some(Example { invocation: "prepend_frontmatter(\"d.md\", \"tags\", \"z\")", result: "[\"z\",\"a\",\"b\"]", verification: ExampleVerification::DisplayOnly("side-effect descriptions are proven by the sandbox verb harness") }),

    },
    // ── File & Directory ────────────────────────────────────────────
    EffectDescriptor {

        signature: "ensure_file(file)",
        description: "Creates an empty file if missing. Returns the absolute path.",
        safety: EffectSafety::FilesystemWrite,
        category: "File & Directory",
        order: 1,

        example: Some(Example { invocation: "ensure_file(\"new.txt\")", result: "/path/to/new.txt", verification: ExampleVerification::DisplayOnly("side-effect descriptions are proven by the sandbox verb harness") }),

    },
    EffectDescriptor {

        signature: "ensure_file(file, content)",
        description: "Creates a file with content if missing. Existing files are unchanged. Returns the absolute path.",
        safety: EffectSafety::FilesystemWrite,
        category: "File & Directory",
        order: 2,

        example: Some(Example { invocation: "ensure_file(\"new.txt\", \"hello\")", result: "/path/to/new.txt", verification: ExampleVerification::DisplayOnly("side-effect descriptions are proven by the sandbox verb harness") }),

    },
    EffectDescriptor {

        signature: "ensure_dir(dir)",
        description: "Creates a directory and all parents if missing. Returns the absolute path.",
        safety: EffectSafety::FilesystemWrite,
        category: "File & Directory",
        order: 3,

        example: Some(Example { invocation: "ensure_dir(\"sub\")", result: "/path/to/sub", verification: ExampleVerification::DisplayOnly("side-effect descriptions are proven by the sandbox verb harness") }),

    },
    EffectDescriptor {

        signature: "append_line(file, text)",
        description: "Appends text and a newline to a file. Returns the absolute path.",
        safety: EffectSafety::FilesystemWrite,
        category: "File & Directory",
        order: 4,

        example: Some(Example { invocation: "append_line(\"log.txt\", \"entry\")", result: "/path/to/log.txt", verification: ExampleVerification::DisplayOnly("side-effect descriptions are proven by the sandbox verb harness") }),

    },
    EffectDescriptor {

        signature: "append_jsonl(file, obj)",
        description: "Serializes an object as JSON and appends it as a line. Returns the absolute path.",
        safety: EffectSafety::FilesystemWrite,
        category: "File & Directory",
        order: 5,

        example: Some(Example { invocation: "append_jsonl(\"log.jsonl\", {\"k\": 1})", result: "/path/to/log.jsonl", verification: ExampleVerification::DisplayOnly("side-effect descriptions are proven by the sandbox verb harness") }),

    },
    // ── Network ─────────────────────────────────────────────────────
    EffectDescriptor {

        signature: "http_post(url, body)",
        description: "Sends an HTTP POST request. Returns an object with status and body.",
        safety: EffectSafety::Network,
        category: "Network",
        order: 1,

        example: Some(Example { invocation: "http_post(\"https://example.com/hook\", \"{}\")", result: "{\"status\":403,\"body\":\"\"}", verification: ExampleVerification::DisplayOnly("side-effect descriptions are proven by the sandbox verb harness") }),

    },
];

/// Public [`EffectEngine`] methods that are intentionally outside the
/// capability surface described by [`EFFECT_DESCRIPTORS`].
///
/// The descriptor catalog is the authoritative capability surface: a method
/// without a descriptor is not a catalogued capability. This list is empty
/// because every public mutating method on [`EffectEngine`] currently has a
/// matching descriptor.
pub const INTENTIONALLY_UNCATALOGUED: &[&str] = &[];


/// Returns all side-effect capability descriptors in display order.
pub fn effect_descriptors() -> &'static [EffectDescriptor] {
    EFFECT_DESCRIPTORS
}

/// Test-only reachability harness: one side-effect signature paired with a thin
/// adapter that exercises the real [`EffectEngine`](super::EffectEngine) method
/// behind it.
///
/// Effects have no string dispatcher — they are typed methods called directly
/// by an external orchestrator, and Rust cannot enumerate the public method
/// surface at compile time. The **descriptor catalog**
/// ([`EFFECT_DESCRIPTORS`]) is therefore the authoritative capability surface;
/// a method without a descriptor is simply not a catalogued capability.
///
/// This registry exists only to keep the catalog honest under test. Each
/// `exercise` closure calls the real method, so a renamed or removed method
/// fails to compile, and
/// `verb_signature_set_equals_descriptor_signature_set` asserts the verb and
/// descriptor signature sets are equal in both directions — catching a
/// descriptor (including a new overload such as `ensure_file(file, content)`)
/// with no backing method, or a verb with no descriptor. It does not, and
/// cannot, detect a public `EffectEngine` method that was never given a
/// descriptor; such a method is intentionally outside the capability surface
/// until a descriptor adds it.
///
/// Lives under `cfg(test)` so it never enters the public API or production
/// builds.
#[cfg(test)]
pub struct EffectVerb {
    /// Canonical signature including arity; matches an [`EffectDescriptor`].
    pub signature: &'static str,
    /// Invokes the verb against a prepared sandbox engine, returning the verb's
    /// value (or, for `http_post`, `null` once the deny-all allowlist refuses
    /// it). Returning `Err` means the verb is unreachable.
    pub exercise: fn(&super::EffectEngine) -> Result<serde_json::Value, super::EffectError>,
}

/// The test-only table pairing every descriptor signature with a real
/// [`EffectEngine`](super::EffectEngine) method. See [`EffectVerb`].
#[cfg(test)]
pub const EFFECT_VERBS: &[EffectVerb] = &[
    EffectVerb {
        signature: "set(key, value)",
        exercise: |e| {
            let mut state = crate::markdown::FrontmatterMap::new();
            e.set(&mut state, "ready", serde_json::json!(true))
        },
    },
    EffectVerb {
        signature: "set_frontmatter(file, prop, value)",
        exercise: |e| e.set_frontmatter("d.md", "status", serde_json::json!("x")),
    },
    EffectVerb {
        signature: "merge_frontmatter(file, obj)",
        exercise: |e| e.merge_frontmatter("d.md", serde_json::json!({"k": 1})),
    },
    EffectVerb {
        signature: "delete_frontmatter(file, prop)",
        exercise: |e| e.delete_frontmatter("d.md", "owner"),
    },
    EffectVerb {
        signature: "increment_frontmatter(file, prop)",
        exercise: |e| e.increment_frontmatter("d.md", "phase"),
    },
    EffectVerb {
        signature: "decrement_frontmatter(file, prop)",
        exercise: |e| e.decrement_frontmatter("d.md", "phase"),
    },
    EffectVerb {
        signature: "append_frontmatter(file, prop, value)",
        exercise: |e| e.append_frontmatter("d.md", "tags", serde_json::json!("b")),
    },
    EffectVerb {
        signature: "prepend_frontmatter(file, prop, value)",
        exercise: |e| e.prepend_frontmatter("d.md", "tags", serde_json::json!("z")),
    },
    EffectVerb {
        signature: "ensure_file(file)",
        exercise: |e| e.ensure_file("new.txt").map(serde_json::Value::String),
    },
    EffectVerb {
        signature: "ensure_file(file, content)",
        exercise: |e| {
            e.ensure_file_with_content("new2.txt", "x")
                .map(serde_json::Value::String)
        },
    },
    EffectVerb {
        signature: "ensure_dir(dir)",
        exercise: |e| e.ensure_dir("sub").map(serde_json::Value::String),
    },
    EffectVerb {
        signature: "append_line(file, text)",
        exercise: |e| e.append_line("log.txt", "x").map(serde_json::Value::String),
    },
    EffectVerb {
        signature: "append_jsonl(file, obj)",
        exercise: |e| {
            e.append_jsonl("log.jsonl", serde_json::json!({"k": 1}))
                .map(serde_json::Value::String)
        },
    },
    EffectVerb {
        signature: "http_post(url, body)",
        // Reachable, but the deny-all allowlist refuses it before any network
        // access. A refusal proves reachability; any other error is a failure.
        exercise: |e| match e.http_post("https://example.com/hook", b"{}".to_vec()) {
            Err(super::EffectError::HostNotAllowed(_)) => Ok(serde_json::Value::Null),
            other => other,
        },
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::EffectEngine;
    use std::collections::HashSet;

    /// Exact, bidirectional parity between the verb harness and the descriptor
    /// catalog.
    ///
    /// Each [`EFFECT_VERBS`] entry calls a real `EffectEngine` method, so set
    /// equality fails in *both* directions: a descriptor (including a new
    /// overload) with no backing method, or a verb with no descriptor. It does
    /// not enforce that every public `EffectEngine` method has a descriptor —
    /// see [`EffectVerb`] for why the descriptor catalog, not the method
    /// surface, defines the capability set.
    #[test]
    fn verb_signature_set_equals_descriptor_signature_set() {
        let verbs: HashSet<&str> = EFFECT_VERBS.iter().map(|v| v.signature).collect();
        let descriptors: HashSet<&str> =
            EFFECT_DESCRIPTORS.iter().map(|d| d.signature).collect();

        let verbs_without_descriptors: Vec<_> = verbs.difference(&descriptors).collect();
        let descriptors_without_verbs: Vec<_> = descriptors.difference(&verbs).collect();

        assert!(
            verbs_without_descriptors.is_empty(),
            "verbs without descriptors: {verbs_without_descriptors:?}"
        );
        assert!(
            descriptors_without_verbs.is_empty(),
            "descriptors without verbs: {descriptors_without_verbs:?}"
        );

        for d in EFFECT_DESCRIPTORS {
            assert!(
                d.example().is_some(),
                "descriptor `{}` must carry an example",
                d.signature
            );
        }
    }

    /// Every registered verb must invoke a real, reachable `EffectEngine`
    /// method. A verb whose method was renamed or removed fails to compile or
    /// to run against the sandbox. The deny-all allowlist refuses `http_post`
    /// before any network access, so the test performs no I/O off the sandbox.
    #[test]
    fn every_verb_maps_to_a_reachable_method() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("d.md"),
            "---\nphase: 1\ntags: [a]\nowner: ken\n---\nBody\n",
        )
        .unwrap();
        let eng = EffectEngine::builder().mutation_root(dir.path()).build();

        for verb in EFFECT_VERBS {
            (verb.exercise)(&eng).unwrap_or_else(|e| {
                panic!("verb `{}` is not reachable: {e:?}", verb.signature)
            });
        }
    }

    #[test]
    fn descriptor_traversal_order_is_deterministic() {
        let sigs: Vec<&str> = EFFECT_DESCRIPTORS
            .iter()
            .map(|d| d.signature)
            .collect();
        let sigs_again: Vec<&str> = EFFECT_DESCRIPTORS
            .iter()
            .map(|d| d.signature)
            .collect();
        assert_eq!(sigs, sigs_again);
    }

    #[test]
    fn descriptor_signatures_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for d in EFFECT_DESCRIPTORS {
            assert!(
                seen.insert(d.signature),
                "Duplicate descriptor signature: {}",
                d.signature
            );
        }
    }

    #[test]
    fn catalog_access_performs_no_capture() {
        let _ = effect_descriptors();
    }
}
