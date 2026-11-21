//! Production YAML documents go through `biscuit_file::to_yaml_string`; the
//! entries below are the direct serializer calls whose output a contract
//! fixes. See `test_toolkit::serializer_guard`.

use test_toolkit::serializer_guard::{Allow, assert_package};

const ALLOWED: &[Allow] = &[Allow {
    file: "src/providers/frontmatter.rs",
    symbol: "yaml_scalar_literal",
    reason: "a completion inserts one scalar literal; its quoting must be the backend's spelling of that scalar",
}];

#[test]
fn yaml_serializer_guard() {
    assert_package(&biscuit_test_harness::manifest_dir!().join("src"), ALLOWED);
}
