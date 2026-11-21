//! Production YAML documents go through `biscuit_file::to_yaml_string`; the
//! entries below are the direct serializer calls whose output a contract
//! fixes. See `test_toolkit::serializer_guard`.

use test_toolkit::serializer_guard::{Allow, assert_package};

const ALLOWED: &[Allow] = &[
    Allow {
        file: "src/yaml_documents/mod.rs",
        symbol: "LoadedYamlDocument::declaration_key",
        reason: "declaration fingerprint: the backend bytes are hashed and must stay stable",
    },
    Allow {
        file: "src/coercion/engine.rs",
        symbol: "Engine::content",
        reason: "a coerced string value: the trimmed backend text is the coerced content, not a document",
    },
    Allow {
        file: "src/triggers/envelope.rs",
        symbol: "describe_key",
        reason: "one key's compact text inside an unknown-key message",
    },
];

#[test]
fn yaml_serializer_guard() {
    assert_package(&biscuit_test_harness::manifest_dir!().join("src"), ALLOWED);
}
