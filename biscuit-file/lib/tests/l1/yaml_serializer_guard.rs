//! Production YAML documents go through `biscuit_file::to_yaml_string`; the
//! entries below are the direct serializer calls whose output a contract
//! fixes. See `test_toolkit::serializer_guard`.

use test_toolkit::serializer_guard::{Allow, assert_package};

const ALLOWED: &[Allow] = &[
    Allow {
        file: "src/yaml/write.rs",
        symbol: "to_yaml_string",
        reason: "the shared writer itself: the backend output it lays out and checks the result against",
    },
    Allow {
        file: "src/yaml/write.rs",
        symbol: "spell",
        reason: "the shared writer itself: a scalar's backend spelling, which the layout keeps",
    },
    Allow {
        file: "src/toml_impl/types.rs",
        symbol: "Toml::as_yaml_value",
        reason: "builds a YAML value, never text: the serialized string is reparsed at once and discarded",
    },
];

#[test]
fn yaml_serializer_guard() {
    assert_package(&biscuit_test_harness::manifest_dir!().join("src"), ALLOWED);
}
