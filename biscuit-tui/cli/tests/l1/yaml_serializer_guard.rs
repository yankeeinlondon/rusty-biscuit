//! Production YAML documents go through `biscuit_file::to_yaml_string`; the
//! entries below are the direct serializer calls whose output a contract
//! fixes. See `test_toolkit::serializer_guard`.

use test_toolkit::serializer_guard::{Allow, assert_package};

const ALLOWED: &[Allow] = &[
    Allow {
        file: "src/option_sources.rs",
        symbol: "extract_yaml_string_array",
        reason: "encodes a non-string array item as an option label string, not a document",
    },
    Allow {
        file: "src/option_sources.rs",
        symbol: "yaml_value_to_string",
        reason: "encodes a non-scalar value as an option identifier string, not a document",
    },
];

#[test]
fn yaml_serializer_guard() {
    assert_package(&biscuit_test_harness::manifest_dir!().join("src"), ALLOWED);
}
