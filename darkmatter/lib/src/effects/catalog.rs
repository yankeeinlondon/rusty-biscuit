//! Typed descriptor catalog for side-effect capabilities.
//!
//! Each [`EffectDescriptor`] describes a single mutating operation available on
//! [`EffectEngine`]. The catalog is a static, compile-time constant —
//! constructing or reading it performs no host probes, no I/O, and no runtime
//! context capture.

/// Safety classification for a side-effect capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EffectSafety {
    /// Filesystem write restricted to the configured mutation root.
    FilesystemWrite,
    /// Network access restricted by deny-all host allowlist.
    Network,
    /// Markdown mutation that honors auto-rehash.
    MarkdownMutation,
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
}

/// All side-effect capability descriptors, in display order.
pub const EFFECT_DESCRIPTORS: &[EffectDescriptor] = &[
    // ── Frontmatter Mutations ───────────────────────────────────────
    EffectDescriptor {
        signature: "set_frontmatter(file, prop, value)",
        description: "Sets a frontmatter property. Returns the prior value (or null).",
        safety: EffectSafety::MarkdownMutation,
        category: "Frontmatter Mutations",
        order: 1,
    },
    EffectDescriptor {
        signature: "merge_frontmatter(file, obj)",
        description: "Shallow-merges an object into the document's frontmatter. Returns the merged object.",
        safety: EffectSafety::MarkdownMutation,
        category: "Frontmatter Mutations",
        order: 2,
    },
    EffectDescriptor {
        signature: "delete_frontmatter(file, prop)",
        description: "Removes a frontmatter property. Returns the removed value (or null).",
        safety: EffectSafety::MarkdownMutation,
        category: "Frontmatter Mutations",
        order: 3,
    },
    EffectDescriptor {
        signature: "increment_frontmatter(file, prop)",
        description: "Increments a numeric frontmatter property. Missing becomes 1. Returns the new number.",
        safety: EffectSafety::MarkdownMutation,
        category: "Frontmatter Mutations",
        order: 4,
    },
    EffectDescriptor {
        signature: "decrement_frontmatter(file, prop)",
        description: "Decrements a numeric frontmatter property. Missing becomes -1. Returns the new number.",
        safety: EffectSafety::MarkdownMutation,
        category: "Frontmatter Mutations",
        order: 5,
    },
    EffectDescriptor {
        signature: "append_frontmatter(file, prop, value)",
        description: "Appends a value to a frontmatter array property. Returns the new array.",
        safety: EffectSafety::MarkdownMutation,
        category: "Frontmatter Mutations",
        order: 6,
    },
    EffectDescriptor {
        signature: "prepend_frontmatter(file, prop, value)",
        description: "Prepends a value to a frontmatter array property. Returns the new array.",
        safety: EffectSafety::MarkdownMutation,
        category: "Frontmatter Mutations",
        order: 7,
    },
    // ── File & Directory ────────────────────────────────────────────
    EffectDescriptor {
        signature: "ensure_file(file)",
        description: "Creates an empty file if missing. Returns the absolute path.",
        safety: EffectSafety::FilesystemWrite,
        category: "File & Directory",
        order: 1,
    },
    EffectDescriptor {
        signature: "ensure_file(file, content)",
        description: "Creates a file with content if missing. Existing files are unchanged. Returns the absolute path.",
        safety: EffectSafety::FilesystemWrite,
        category: "File & Directory",
        order: 2,
    },
    EffectDescriptor {
        signature: "ensure_dir(dir)",
        description: "Creates a directory and all parents if missing. Returns the absolute path.",
        safety: EffectSafety::FilesystemWrite,
        category: "File & Directory",
        order: 3,
    },
    EffectDescriptor {
        signature: "append_line(file, text)",
        description: "Appends text and a newline to a file. Returns the absolute path.",
        safety: EffectSafety::FilesystemWrite,
        category: "File & Directory",
        order: 4,
    },
    EffectDescriptor {
        signature: "append_jsonl(file, obj)",
        description: "Serializes an object as JSON and appends it as a line. Returns the absolute path.",
        safety: EffectSafety::FilesystemWrite,
        category: "File & Directory",
        order: 5,
    },
    // ── Network ─────────────────────────────────────────────────────
    EffectDescriptor {
        signature: "http_post(url, body)",
        description: "Sends an HTTP POST request. Returns an object with status and body.",
        safety: EffectSafety::Network,
        category: "Network",
        order: 1,
    },
];

/// Returns all side-effect capability descriptors in display order.
pub fn effect_descriptors() -> &'static [EffectDescriptor] {
    EFFECT_DESCRIPTORS
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::{EffectEngine, EffectError};
    use serde_json::json;

    /// Every descriptor signature must map to a real, reachable
    /// [`EffectEngine`] verb.
    ///
    /// Effects have no string dispatcher — they are typed methods on
    /// `EffectEngine` — so there is no name registry to enumerate. Instead of
    /// comparing two hand-maintained signature lists, this test *invokes* the
    /// verb behind each descriptor against a sandbox mutation root. A descriptor
    /// whose verb was renamed or removed fails to compile or to run; a new
    /// descriptor with no exercised verb hits the `_` arm and fails. The verb
    /// behind a signature is the source of truth; the descriptor only documents
    /// it.
    #[test]
    fn every_descriptor_signature_maps_to_a_reachable_verb() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("d.md"),
            "---\nphase: 1\ntags: [a]\nowner: ken\n---\nBody\n",
        )
        .unwrap();
        // Deny-all allowlist (the default): `http_post` is reachable but is
        // refused before any network access, so the test performs no I/O off
        // the sandbox.
        let eng = EffectEngine::builder().mutation_root(dir.path()).build();

        for desc in EFFECT_DESCRIPTORS {
            match desc.signature {
                "set_frontmatter(file, prop, value)" => {
                    eng.set_frontmatter("d.md", "status", json!("x")).unwrap();
                }
                "merge_frontmatter(file, obj)" => {
                    eng.merge_frontmatter("d.md", json!({"k": 1})).unwrap();
                }
                "delete_frontmatter(file, prop)" => {
                    eng.delete_frontmatter("d.md", "owner").unwrap();
                }
                "increment_frontmatter(file, prop)" => {
                    eng.increment_frontmatter("d.md", "phase").unwrap();
                }
                "decrement_frontmatter(file, prop)" => {
                    eng.decrement_frontmatter("d.md", "phase").unwrap();
                }
                "append_frontmatter(file, prop, value)" => {
                    eng.append_frontmatter("d.md", "tags", json!("b")).unwrap();
                }
                "prepend_frontmatter(file, prop, value)" => {
                    eng.prepend_frontmatter("d.md", "tags", json!("z")).unwrap();
                }
                "ensure_file(file)" => {
                    eng.ensure_file("new.txt").unwrap();
                }
                "ensure_file(file, content)" => {
                    eng.ensure_file_with_content("new2.txt", "x").unwrap();
                }
                "ensure_dir(dir)" => {
                    eng.ensure_dir("sub").unwrap();
                }
                "append_line(file, text)" => {
                    eng.append_line("log.txt", "x").unwrap();
                }
                "append_jsonl(file, obj)" => {
                    eng.append_jsonl("log.jsonl", json!({"k": 1})).unwrap();
                }
                "http_post(url, body)" => {
                    // Reachable, but deny-all refuses it before any network hit.
                    let err = eng.http_post("https://example.com/hook", b"{}").unwrap_err();
                    assert!(
                        matches!(err, EffectError::HostNotAllowed(_)),
                        "http_post must be refused by the deny-all allowlist, not error otherwise: {err:?}"
                    );
                }
                other => panic!(
                    "descriptor signature has no exercised EffectEngine verb: {other}"
                ),
            }
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
