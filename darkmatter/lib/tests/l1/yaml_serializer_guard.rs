//! Production YAML documents go through `biscuit_file::to_yaml_string`; the
//! entries below are the direct serializer calls whose output a contract
//! fixes. See `test_toolkit::serializer_guard`.

use test_toolkit::serializer_guard::{Allow, assert_package};

const ALLOWED: &[Allow] = &[
    Allow {
        file: "src/markdown/hash/mod.rs",
        symbol: "hash_frontmatter_map",
        reason: "strict frontmatter hash input: the backend bytes are hashed, so changing the writer would change every stored strict hash",
    },
    Allow {
        file: "src/markdown/delta/report.rs",
        symbol: "render_delta_report",
        reason: "both frontmatter sides of a visual diff fragment in the delta report; they are compared, not written",
    },
    Allow {
        file: "src/markdown/toc/mod.rs",
        symbol: "MarkdownToc::from",
        reason: "raw frontmatter hash input for the TOC; the backend bytes are hashed, so the hash must not follow writer settings",
    },
    Allow {
        file: "src/markdown/schemas/mod.rs",
        symbol: "positions_for",
        reason: "materializes frontmatter to project schema positions onto; the text is reparsed for offsets and never written",
    },
];

#[test]
fn yaml_serializer_guard() {
    assert_package(&biscuit_test_harness::manifest_dir!().join("src"), ALLOWED);
}
