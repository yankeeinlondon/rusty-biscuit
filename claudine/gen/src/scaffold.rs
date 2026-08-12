//! Never-overwrite scaffolding for a newly wired provider.
//!
//! Onboarding a provider is a two-pass loop driven by `generate <slug>
//! --scaffold`:
//!
//! 1. Wire the enum variant + [`emit::PROVIDER_VARIANTS`] entry by hand.
//! 2. First `--scaffold` run writes a TODO facts skeleton
//!    ([`scaffold_facts`]) and stops — the required fields are unfilled, so
//!    `data.rs` generation cannot run yet.
//! 3. Fill the facts (and land the research topics), then rerun
//!    `--scaffold`: the facts file now exists, so this pass writes the
//!    hand-owned `mod.rs`/`behavior.rs` stubs ([`scaffold_mod`] /
//!    [`scaffold_behavior`]) and proceeds to `data.rs` generation.
//!
//! Every writer here refuses to overwrite an existing file: the facts
//! skeleton and the behavior half are hand-owned surfaces, and a rerun must
//! never clobber authored work.

use std::path::Path;

use crate::emit;
use crate::errors::GenError;
use crate::registry::{Coercion, DeclaredSource, REGISTRY};

/// Writes `docs/providers/facts/<slug>.yaml` IF ABSENT, seeded with the
/// facts-sourced registry fields (each `null`, prefixed with a
/// `# TODO(<required|optional>): <description>` line), in registry order,
/// plus the facts-fed `acp:` sub-record (see below).
///
/// Returns `true` when it wrote the skeleton, `false` when the file already
/// existed (never overwrites). A still-`null` required field fails `data.rs`
/// generation loudly — that is the intended "fill it" signal, so no typed
/// placeholder is invented here.
///
/// ## Notes
///
/// The `acp` field is `Research`-sourced for `server_mode`, but its coercion
/// is mixed: `client_supported`/`events_via_acp` are read from this facts
/// file's `acp:` sub-record. The registry loop only walks
/// [`DeclaredSource::Facts`] entries, so it skips `acp` — the sub-record is
/// emitted explicitly here, or a fully-wired provider fails generation with
/// "facts file has no `acp` key" (a confirmed M-Kilo/M-Pi onboarding papercut).
pub fn scaffold_facts(area: &Path, slug: &str) -> Result<bool, GenError> {
    let path = area.join(format!("docs/providers/facts/{slug}.yaml"));
    if path.exists() {
        return Ok(false);
    }
    let mut out = String::new();
    out.push_str(&format!(
        "# TODO facts skeleton for the `{slug}` provider — scaffolded by claudine-gen.\n\
         #\n\
         # Fill every TODO(required) field before generating data.rs; optional fields may\n\
         # stay `null`. Keys are the facts-sourced mapping-registry fields, registry order.\n\n"
    ));
    for entry in REGISTRY {
        let DeclaredSource::Facts { key } = entry.source else {
            continue;
        };
        let kind = if entry.optional { "optional" } else { "required" };
        out.push_str(&format!("# TODO({kind}): {}\n", entry.description));
        out.push_str(&format!("{key}: null\n"));
    }
    // `acp` is Research-sourced (`server_mode`) so the loop above skips it, but
    // its coercion also reads `client_supported`/`events_via_acp` from this facts
    // file — emit that sub-record, or generation fails "facts file has no `acp`
    // key". Gated on the coercion so it self-removes if `acp` ever goes
    // single-source. `server_mode` is research-owned: seeding it here is a
    // generation error, so it is deliberately absent.
    if let Some(acp) = REGISTRY.iter().find(|entry| entry.coercion == Coercion::AcpRecord) {
        out.push_str(&format!(
            "\n# TODO(required): ACP facts sub-record — client_supported (bool) and\n\
             # events_via_acp (list) are facts-fed; never add server_mode (research-owned).\n\
             {}:\n\
             \x20 client_supported: null\n\
             \x20 events_via_acp: []\n",
            acp.field,
        ));
    }
    write_if_absent(&path, &out)
}

/// Writes `lib/src/provider/<slug>/mod.rs` IF ABSENT (modeled on
/// `opencode/mod.rs`). Returns whether it wrote (`true`) or the file already
/// existed (`false`).
pub fn scaffold_mod(area: &Path, slug: &str, variant: &str) -> Result<bool, GenError> {
    let path = area.join(format!("lib/src/provider/{slug}/mod.rs"));
    let upper = slug.to_uppercase();
    let contents = format!(
        "//! {variant} provider definition.\n\
         \n\
         mod behavior;\n\
         mod data;\n\
         \n\
         pub(super) use data::{upper}_INFO;\n"
    );
    write_if_absent(&path, &contents)
}

/// Writes `lib/src/provider/<slug>/behavior.rs` IF ABSENT: a compiling stub
/// modeled on `opencode/behavior.rs`. Returns whether it wrote (`true`) or
/// the file already existed (`false`).
///
/// The stub implements the one required method (`detect_from_payload`) and
/// takes the default implementations for the other three traits — the
/// adapter/configurator defaults panic, so the TODO comments flag the two
/// overrides an onboarding author must supply before wiring adapters/config.
pub fn scaffold_behavior(area: &Path, slug: &str, variant: &str) -> Result<bool, GenError> {
    let path = area.join(format!("lib/src/provider/{slug}/behavior.rs"));
    let upper = slug.to_uppercase();
    let contents = format!(
        "//! Hand-written behavior half for {variant} — scaffolded stub. Fill the TODOs.\n\
         \n\
         use serde_json::Value;\n\
         \n\
         use crate::provider::behavior::{{\n\
         \x20   AdapterBehavior, ConfiguratorBehavior, McpBehavior, ProviderBehavior,\n\
         }};\n\
         \n\
         #[derive(Debug)]\n\
         pub(super) struct {variant}Provider;\n\
         \n\
         pub(super) static {upper}_PROVIDER: {variant}Provider = {variant}Provider;\n\
         \n\
         impl ProviderBehavior for {variant}Provider {{\n\
         \x20   fn detect_from_payload(&self, _raw: &Value) -> bool {{\n\
         \x20       // TODO: recognize this provider's stream payloads.\n\
         \x20       false\n\
         \x20   }}\n\
         }}\n\
         impl McpBehavior for {variant}Provider {{}}\n\
         // TODO: provider_adapter() default panics — override before wiring adapters.\n\
         impl AdapterBehavior for {variant}Provider {{}}\n\
         // TODO: agent_configurator() default panics — override before configuring agents.\n\
         impl ConfiguratorBehavior for {variant}Provider {{}}\n"
    );
    write_if_absent(&path, &contents)
}

/// Resolves the wired `Provider` variant name for a slug (a thin re-export
/// of [`emit::provider_variant`]), so scaffolding can fail with the "wire the
/// enum variant first" message before touching any file.
pub fn provider_variant(slug: &str) -> Result<&'static str, GenError> {
    emit::provider_variant(slug)
}

/// Creates parent dirs and writes `contents` only when `path` is absent.
fn write_if_absent(path: &Path, contents: &str) -> Result<bool, GenError> {
    if path.exists() {
        return Ok(false);
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| GenError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    std::fs::write(path, contents).map_err(|source| GenError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The facts-sourced registry fields, registry order — the exact key set
    /// the skeleton must emit.
    fn facts_keys() -> Vec<&'static str> {
        REGISTRY
            .iter()
            .filter_map(|entry| match entry.source {
                DeclaredSource::Facts { key } => Some(key),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn facts_skeleton_lists_every_facts_field_in_registry_order() {
        let dir = tempfile::tempdir().unwrap();
        let area = dir.path();
        assert!(scaffold_facts(area, "kilo").unwrap(), "absent file is written");

        let text = std::fs::read_to_string(area.join("docs/providers/facts/kilo.yaml")).unwrap();
        // Every facts key appears, `null`-valued, in registry order. Indented
        // lines (the nested `acp:` sub-record) are not top-level facts keys.
        let emitted: Vec<String> = text
            .lines()
            .filter(|line| !line.starts_with(char::is_whitespace))
            .filter_map(|line| line.strip_suffix(": null"))
            .map(str::to_string)
            .collect();
        let expected: Vec<String> = facts_keys().iter().map(|k| k.to_string()).collect();
        assert_eq!(emitted, expected);
        // Required vs optional is reflected in the preceding TODO tag.
        assert!(text.contains("# TODO(optional): "), "{text}");
        assert!(text.contains("# TODO(required): "), "{text}");
    }

    #[test]
    fn facts_skeleton_emits_the_mixed_source_acp_subrecord() {
        let dir = tempfile::tempdir().unwrap();
        let area = dir.path();
        assert!(scaffold_facts(area, "kilo").unwrap());

        let text = std::fs::read_to_string(area.join("docs/providers/facts/kilo.yaml")).unwrap();
        // The nested facts-fed sub-record the AcpRecord coercion reads (missing
        // it fails generation with "facts file has no `acp` key").
        assert!(text.contains("\nacp:\n"), "acp sub-record present:\n{text}");
        assert!(text.contains("client_supported: null"), "{text}");
        assert!(text.contains("events_via_acp: []"), "{text}");
        // `server_mode` is research-owned; seeding the key here is a generation
        // error, so it must never appear as a facts sub-key.
        assert!(!text.contains("server_mode:"), "{text}");
    }

    #[test]
    fn facts_skeleton_never_overwrites() {
        let dir = tempfile::tempdir().unwrap();
        let area = dir.path();
        let path = area.join("docs/providers/facts/kilo.yaml");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "stream_protocol: ndjson\n").unwrap();

        assert!(!scaffold_facts(area, "kilo").unwrap(), "existing file is preserved");
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "stream_protocol: ndjson\n"
        );
    }

    #[test]
    fn mod_and_behavior_stubs_carry_the_expected_identifiers() {
        let dir = tempfile::tempdir().unwrap();
        let area = dir.path();
        assert!(scaffold_mod(area, "kilo", "Kilo").unwrap());
        assert!(scaffold_behavior(area, "kilo", "Kilo").unwrap());

        let mod_rs = std::fs::read_to_string(area.join("lib/src/provider/kilo/mod.rs")).unwrap();
        assert!(mod_rs.contains("pub(super) use data::KILO_INFO;"), "{mod_rs}");
        assert!(mod_rs.contains("mod behavior;") && mod_rs.contains("mod data;"), "{mod_rs}");

        let behavior_rs =
            std::fs::read_to_string(area.join("lib/src/provider/kilo/behavior.rs")).unwrap();
        assert!(behavior_rs.contains("pub(super) struct KiloProvider;"), "{behavior_rs}");
        assert!(
            behavior_rs.contains("pub(super) static KILO_PROVIDER: KiloProvider = KiloProvider;"),
            "{behavior_rs}"
        );
        assert!(
            behavior_rs.contains("impl ProviderBehavior for KiloProvider"),
            "{behavior_rs}"
        );
    }

    #[test]
    fn stub_writers_never_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let area = dir.path();
        for rel in ["lib/src/provider/kilo/mod.rs", "lib/src/provider/kilo/behavior.rs"] {
            let path = area.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, "// authored\n").unwrap();
        }
        assert!(!scaffold_mod(area, "kilo", "Kilo").unwrap());
        assert!(!scaffold_behavior(area, "kilo", "Kilo").unwrap());
        assert_eq!(
            std::fs::read_to_string(area.join("lib/src/provider/kilo/mod.rs")).unwrap(),
            "// authored\n"
        );
    }
}
