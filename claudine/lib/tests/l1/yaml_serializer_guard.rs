//! Production YAML documents go through `biscuit_file::to_yaml_string`; the
//! entries below are the direct serializer calls whose output a contract
//! fixes. See `test_toolkit::serializer_guard`.

use test_toolkit::serializer_guard::{Allow, assert_package};

const ALLOWED: &[Allow] = &[
    Allow {
        file: "src/composition/file_detail.rs",
        symbol: "string_from_yaml_map",
        reason: "renders one non-string frontmatter value as inline context text, not a document",
    },
    Allow {
        file: "src/composition/file_detail.rs",
        symbol: "yaml_lines",
        reason: "splits a schema value into context lines; the line text, not a written document, is the output",
    },
    Allow {
        file: "src/linking/compatibility/properties.rs",
        symbol: "yaml_value_to_string",
        reason: "renders a collection property value as a trimmed context fragment, not a document",
    },
];

#[test]
fn yaml_serializer_guard() {
    assert_package(&biscuit_test_harness::manifest_dir!().join("src"), ALLOWED);
}
