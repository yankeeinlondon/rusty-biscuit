//! Every workspace member that can reach a YAML backend runs the serializer
//! guard over its own source.
//!
//! A member reaches a backend when its normal or build dependencies (any
//! target) name `serde_yaml` or `serde_yaml_ng`, or name `biscuit-file` with
//! its `yaml` feature enabled, directly or through default features or another
//! feature. Dev-dependencies do not count: test code is outside the guard.
//!
//! [`MEMBERS`] spells every member manifest and guard test path so CI's
//! test-input index schedules this test when one changes; the test fails when
//! the table and the root `members` list disagree, so the table cannot drift.

use std::collections::BTreeSet;

use toml::{Table, Value};

/// Every workspace member's manifest, and the guard test of each that reaches
/// a YAML backend.
const MEMBERS: &[(&str, Option<&str>)] = &[
    ("biscuit-clipboard/cli/Cargo.toml", None),
    ("biscuit-clipboard/lib/Cargo.toml", None),
    ("biscuit-clipboard/service/Cargo.toml", None),
    ("biscuit-contract/lib/Cargo.toml", None),
    ("reaper/lib/Cargo.toml", None),
    ("reaper/cli/Cargo.toml", None),
    (
        "biscuit-file/cli/Cargo.toml",
        Some("biscuit-file/cli/tests/yaml_serializer_guard.rs"),
    ),
    (
        "biscuit-file/lib/Cargo.toml",
        Some("biscuit-file/lib/tests/l1/yaml_serializer_guard.rs"),
    ),
    ("biscuit-hash/cli/Cargo.toml", None),
    ("biscuit-hash/lib/Cargo.toml", None),
    ("content-policy/cli/Cargo.toml", None),
    (
        "content-policy/lib/Cargo.toml",
        Some("content-policy/lib/tests/yaml_serializer_guard.rs"),
    ),
    ("biscuit-icon/cli/Cargo.toml", None),
    ("biscuit-icon/lib/Cargo.toml", None),
    ("biscuit-location/cli/Cargo.toml", None),
    ("biscuit-location/lib/Cargo.toml", None),
    ("biscuit-speaks/cli/Cargo.toml", None),
    ("biscuit-speaks/lib/Cargo.toml", None),
    ("biscuit-terminal/cli/Cargo.toml", None),
    (
        "biscuit-terminal/lib/Cargo.toml",
        Some("biscuit-terminal/lib/tests/l1/yaml_serializer_guard.rs"),
    ),
    ("claudine/catalog-types/Cargo.toml", None),
    (
        "claudine/cli/Cargo.toml",
        Some("claudine/cli/tests/l1/yaml_serializer_guard.rs"),
    ),
    ("claudine/contract/Cargo.toml", None),
    (
        "claudine/gen/Cargo.toml",
        Some("claudine/gen/tests/l1/yaml_serializer_guard.rs"),
    ),
    (
        "claudine/lib/Cargo.toml",
        Some("claudine/lib/tests/l1/yaml_serializer_guard.rs"),
    ),
    (
        "darkmatter/cli/Cargo.toml",
        Some("darkmatter/cli/tests/l1/yaml_serializer_guard.rs"),
    ),
    (
        "darkmatter/lib/Cargo.toml",
        Some("darkmatter/lib/tests/l1/yaml_serializer_guard.rs"),
    ),
    (
        "simplified-schema/lib/Cargo.toml",
        Some("simplified-schema/lib/tests/l1/yaml_serializer_guard.rs"),
    ),
    ("homelab/arcam-amp-integration/Cargo.toml", None),
    ("homelab/cli/Cargo.toml", None),
    ("homelab/eversolo-integration/Cargo.toml", None),
    ("homelab/lib/Cargo.toml", None),
    ("homelab/server/Cargo.toml", None),
    ("homelab/sony-receiver-integration/Cargo.toml", None),
    ("homelab/unfolded-integration-helper/Cargo.toml", None),
    ("model-citizen/cli/Cargo.toml", None),
    (
        "model-citizen/lib/Cargo.toml",
        Some("model-citizen/lib/tests/yaml_serializer_guard.rs"),
    ),
    ("unchained-ai/cli/Cargo.toml", None),
    ("unchained-ai/contract/Cargo.toml", None),
    ("unchained-ai/gen/Cargo.toml", None),
    ("unchained-ai/lib/Cargo.toml", None),
    ("unchained-ai/model_id/Cargo.toml", None),
    ("playa/cli/Cargo.toml", None),
    ("messenger/cli/Cargo.toml", None),
    ("messenger/lib/Cargo.toml", None),
    ("biscuit-visualized/src/Cargo.toml", None),
    ("worktree/lib/Cargo.toml", None),
    ("worktree/cli/Cargo.toml", None),
    (
        "darkmatter/dmls/Cargo.toml",
        Some("darkmatter/dmls/tests/l1/yaml_serializer_guard.rs"),
    ),
    ("darkmatter/dmls/zed-dmls-cli/Cargo.toml", None),
    ("tools/test-toolkit/Cargo.toml", None),
    ("biscuit-browser-harness/Cargo.toml", None),
    ("claudine/rendezvous/core/Cargo.toml", None),
    ("claudine/rendezvous/daemon/Cargo.toml", None),
    ("claudine/rendezvous/client/Cargo.toml", None),
    ("visualizer/src-tauri/Cargo.toml", None),
    (
        "sniff/lib/Cargo.toml",
        Some("sniff/lib/tests/l1/yaml_serializer_guard.rs"),
    ),
    ("sniff/cli/Cargo.toml", None),
    ("renderable/Cargo.toml", None),
    (
        "biscuit-tui/lib/Cargo.toml",
        Some("biscuit-tui/lib/tests/yaml_serializer_guard.rs"),
    ),
    (
        "biscuit-tui/cli/Cargo.toml",
        Some("biscuit-tui/cli/tests/l1/yaml_serializer_guard.rs"),
    ),
    ("playa/lib/Cargo.toml", None),
    ("queue/lib/Cargo.toml", None),
    ("queue/cli/Cargo.toml", None),
    (
        "research/lib/Cargo.toml",
        Some("research/lib/tests/yaml_serializer_guard.rs"),
    ),
    ("research/cli/Cargo.toml", None),
    ("tree-hugger/lib/Cargo.toml", None),
    ("tree-hugger/cli/Cargo.toml", None),
    ("tabby/Cargo.toml", None),
    ("tabby/ui/Cargo.toml", None),
    (
        "schematic/define/Cargo.toml",
        Some("schematic/define/tests/yaml_serializer_guard.rs"),
    ),
    ("schematic/definitions/Cargo.toml", None),
    (
        "schematic/gen/Cargo.toml",
        Some("schematic/gen/tests/l1/yaml_serializer_guard.rs"),
    ),
    ("schematic/oauth/Cargo.toml", None),
    ("schematic/runtime/Cargo.toml", None),
    ("agent-sandbox/cli/Cargo.toml", None),
    ("scripts/Cargo.toml", None),
];

const BACKENDS: &[&str] = &["serde_yaml", "serde_yaml_ng"];

/// Why `manifest` reaches a YAML backend, or `None`.
fn yaml_backend(manifest: &Table, biscuit_file_features: &Table) -> Option<String> {
    let mut tables: Vec<(String, &Table)> = Vec::new();
    for kind in ["dependencies", "build-dependencies"] {
        if let Some(table) = manifest.get(kind).and_then(Value::as_table) {
            tables.push((kind.to_string(), table));
        }
        if let Some(targets) = manifest.get("target").and_then(Value::as_table) {
            for (target, spec) in targets {
                if let Some(table) = spec.get(kind).and_then(Value::as_table) {
                    tables.push((format!("target.{target}.{kind}"), table));
                }
            }
        }
    }
    for (kind, table) in tables {
        for (name, spec) in table {
            let package = spec.get("package").and_then(Value::as_str).unwrap_or(name);
            if BACKENDS.contains(&package) {
                return Some(format!("[{kind}] {name}"));
            }
            if package == "biscuit-file" && enables_yaml(spec, biscuit_file_features) {
                return Some(format!("[{kind}] {name} with the `yaml` feature"));
            }
        }
    }
    None
}

fn enables_yaml(spec: &Value, features: &Table) -> bool {
    let defaults = spec
        .get("default-features")
        .or_else(|| spec.get("default_features"))
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let mut pending: Vec<String> = spec
        .get("features")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect();
    if defaults {
        pending.push("default".to_string());
    }
    let mut enabled = BTreeSet::new();
    while let Some(feature) = pending.pop() {
        if !enabled.insert(feature.clone()) {
            continue;
        }
        let implied = features
            .get(&feature)
            .and_then(Value::as_array)
            .into_iter()
            .flatten();
        pending.extend(
            implied
                .filter_map(Value::as_str)
                .filter(|f| !f.contains(':') && !f.contains('/'))
                .map(str::to_string),
        );
    }
    enabled.contains("yaml")
}

fn read_table(path: &std::path::Path) -> Table {
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
    text.parse()
        .unwrap_or_else(|error| panic!("cannot parse {}: {error}", path.display()))
}

#[test]
fn serializer_guard_covers_every_yaml_backend_package() {
    let root = test_toolkit::workspace_root();
    let workspace = read_table(&root.join("Cargo.toml"));
    let declared: BTreeSet<String> = workspace["workspace"]["members"]
        .as_array()
        .expect("workspace members")
        .iter()
        .filter_map(Value::as_str)
        .map(|member| format!("{member}/Cargo.toml"))
        .collect();
    let listed: BTreeSet<String> = MEMBERS
        .iter()
        .map(|(manifest, _)| (*manifest).to_string())
        .collect();
    let mut problems: Vec<String> = Vec::new();
    for missing in declared.difference(&listed) {
        problems.push(format!(
            "{missing}: a workspace member missing from MEMBERS in this test; add it"
        ));
    }
    for extra in listed.difference(&declared) {
        problems.push(format!(
            "{extra}: listed in MEMBERS but not a workspace member; remove it"
        ));
    }

    let biscuit_file = read_table(&root.join("biscuit-file/lib/Cargo.toml"));
    let features = biscuit_file
        .get("features")
        .and_then(Value::as_table)
        .cloned()
        .unwrap_or_default();
    for (manifest, guard) in MEMBERS {
        let reaches = yaml_backend(&read_table(&root.join(manifest)), &features);
        match (reaches, guard) {
            (Some(reason), None) => problems.push(format!(
                "{manifest} reaches a YAML backend ({reason}) but has no serializer guard: add an L1 \
                 test calling `test_toolkit::serializer_guard::assert_package` over its `src` and \
                 name it in MEMBERS"
            )),
            (None, Some(guard)) => problems.push(format!(
                "{manifest} no longer reaches a YAML backend; remove {guard} and its MEMBERS entry"
            )),
            (Some(_), Some(guard)) => {
                let text = std::fs::read_to_string(root.join(guard)).unwrap_or_default();
                if !text.contains("serializer_guard::") || !text.contains("assert_package") {
                    problems.push(format!(
                        "{guard} does not call `test_toolkit::serializer_guard::assert_package`"
                    ));
                }
            }
            (None, None) => {}
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
    assert!(
        MEMBERS.iter().filter(|(_, guard)| guard.is_some()).count() > 1,
        "no guarded member found; the backend detection read nothing"
    );
}
