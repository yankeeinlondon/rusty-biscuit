//! Explicit option bundle for the option-driven hash model.

use std::collections::BTreeSet;

use super::kind::MdHashKind;

/// Default name of the frontmatter property that stores the hash.
pub const DEFAULT_HASH_PROPERTY: &str = "hash";

/// Frontmatter key recording the last content-change date. Always ignored
/// during hash computation so rewriting it can never alter a hash.
pub const LAST_UPDATED_KEY: &str = "last_updated";

/// Inputs for computing or saving a markdown hash.
///
/// The library reads no environment; the CLI resolves every field (from
/// `HASH_PROPERTY`, `HASH_IGNORE_PROPERTIES`, `--kind`, `--strict`, …) and
/// passes the bundle in. This keeps hashing deterministic and unit-testable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MdHashOptions {
    /// Active hash property name (default [`DEFAULT_HASH_PROPERTY`]).
    pub property: String,
    /// Extra ignored property names, beyond the always-ignored managed keys.
    pub extra_ignored: Vec<String>,
    /// Forced kind from `--kind`; `None` matches the stored kind / defaults to
    /// [`MdHashKind::Simple`].
    pub forced_kind: Option<MdHashKind>,
    /// Strict mode: no whitespace normalization or key reordering.
    pub strict: bool,
}

impl Default for MdHashOptions {
    fn default() -> Self {
        Self {
            property: DEFAULT_HASH_PROPERTY.to_string(),
            extra_ignored: Vec::new(),
            forced_kind: None,
            strict: false,
        }
    }
}

impl MdHashOptions {
    /// The full set of frontmatter keys ignored during hash computation:
    /// the active hash property, [`LAST_UPDATED_KEY`], and any extra ignored
    /// properties. Keys are matched byte-exactly.
    ///
    /// The managed keys (`property` and `last_updated`) are always present and
    /// cannot be removed; the stored `ignored` list records only the extras.
    pub fn ignore_set(&self) -> BTreeSet<String> {
        let mut set = BTreeSet::new();
        set.insert(self.property.clone());
        set.insert(LAST_UPDATED_KEY.to_string());
        for key in &self.extra_ignored {
            set.insert(key.clone());
        }
        set
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_ignore_set_holds_only_managed_keys() {
        let opts = MdHashOptions::default();
        let set = opts.ignore_set();
        assert_eq!(set.len(), 2);
        assert!(set.contains("hash"));
        assert!(set.contains("last_updated"));
    }

    #[test]
    fn ignore_set_merges_property_and_extras() {
        let opts = MdHashOptions {
            property: "fingerprint".to_string(),
            extra_ignored: vec!["reviewed".to_string(), "draft".to_string()],
            ..MdHashOptions::default()
        };
        let set = opts.ignore_set();
        assert!(set.contains("fingerprint"));
        assert!(set.contains("last_updated"));
        assert!(set.contains("reviewed"));
        assert!(set.contains("draft"));
        // The default "hash" property is not ignored once overridden.
        assert!(!set.contains("hash"));
    }
}
