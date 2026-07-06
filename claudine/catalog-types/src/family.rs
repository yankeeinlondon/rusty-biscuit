//! `&'static` row type for the vendored model-family index.
//!
//! `claudine-gen` compiles the slice of the unchained-ai models-catalog
//! artifact's family index reachable from generated expected-offering
//! `catalog_id` joins into the claudine lib's `family_latest` resolver
//! (design/model-catalog-boundary.md "`family_latest` semantics").

/// One family from the artifact's family index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FamilyRow {
    /// Family key (`vendor/family`, the identity-key prefix).
    pub key: &'static str,
    /// Identity key of the family's newest release — a release, not an
    /// offering: consumers pick a concrete offering from the duplicate
    /// group by their own policy (Checkpoint F ruling).
    pub latest: Option<&'static str>,
    /// Member offering wire ids, release-ordered ascending.
    pub members: &'static [&'static str],
    /// Wire ids the vendor re-targets over releases (e.g.
    /// `openrouter/anthropic/claude-opus-latest`).
    pub rolling_aliases: &'static [&'static str],
}

/// The family key of an identity key: the prefix before the first `@`,
/// `+`, or `:` — exact by construction of the ratified grammar
/// `vendor/family[@version|@date_pin](+variant)*(+size)*(:tag)*`.
pub fn family_key(identity_key: &str) -> &str {
    match identity_key.find(['@', '+', ':']) {
        Some(at) => &identity_key[..at],
        None => identity_key,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pins the grammar prefix contract shared by the generator's key
    /// derivation and the lib's alias resolver.
    #[test]
    fn family_key_strips_everything_after_the_first_marker() {
        // Versioned.
        assert_eq!(family_key("anthropic/claude-opus@4.8"), "anthropic/claude-opus");
        // Date-pinned.
        assert_eq!(family_key("openai/gpt@2026-01-23"), "openai/gpt");
        // Variant after the version.
        assert_eq!(family_key("google/gemini-pro@3+preview"), "google/gemini-pro");
        // Size chains.
        assert_eq!(family_key("google/gemma@4+it+26b+a4b"), "google/gemma");
        // Serving tag.
        assert_eq!(family_key("moonshotai/kimi-k@2.7:thinking"), "moonshotai/kimi-k");
        // Variant without a version (the `@` segment is optional).
        assert_eq!(family_key("vendor/model+code"), "vendor/model");
        assert_eq!(family_key("vendor/model:thinking"), "vendor/model");
        // Bare key is its own family key.
        assert_eq!(family_key("vendor-a/shared-name"), "vendor-a/shared-name");
    }
}
