//! One test per cell of the document-data matrix in `schema_union_matrix.rs`.
//! Each test spells its repository reads inside itself, so a change to either
//! file schedules exactly these tests.

use crate::schema_union_matrix::matrix_cell;

#[test]
fn control_document_validates() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    matrix_cell(shipped, "control_document_validates");
}

#[test]
fn content_policy_absent_is_valid() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    matrix_cell(shipped, "content_policy_absent_is_valid");
}

#[test]
fn rule_absent_is_reported_at_the_item() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    matrix_cell(shipped, "rule_absent_is_reported_at_the_item");
}

#[test]
fn action_absent_is_reported_at_the_item() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    matrix_cell(shipped, "action_absent_is_reported_at_the_item");
}

#[test]
fn content_policy_null_is_valid() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    matrix_cell(shipped, "content_policy_null_is_valid");
}

#[test]
fn rule_null_is_rejected() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    matrix_cell(shipped, "rule_null_is_rejected");
}

#[test]
fn action_null_is_rejected() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    matrix_cell(shipped, "action_null_is_rejected");
}

#[test]
fn content_policy_scalar_is_rejected() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    matrix_cell(shipped, "content_policy_scalar_is_rejected");
}

#[test]
fn rule_number_is_rejected() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    matrix_cell(shipped, "rule_number_is_rejected");
}

#[test]
fn action_number_is_rejected() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    matrix_cell(shipped, "action_number_is_rejected");
}

#[test]
fn content_policy_one_numeric_item_fails_at_its_index() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    matrix_cell(
        shipped,
        "content_policy_one_numeric_item_fails_at_its_index",
    );
}

#[test]
fn rule_mixed_sequence_is_rejected() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    matrix_cell(shipped, "rule_mixed_sequence_is_rejected");
}

#[test]
fn action_mixed_sequence_is_rejected() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    matrix_cell(shipped, "action_mixed_sequence_is_rejected");
}

#[test]
fn content_policy_numeric_items_each_fail() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    matrix_cell(shipped, "content_policy_numeric_items_each_fail");
}

#[test]
fn rule_numeric_sequence_is_rejected() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    matrix_cell(shipped, "rule_numeric_sequence_is_rejected");
}

#[test]
fn action_numeric_sequence_is_rejected() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    matrix_cell(shipped, "action_numeric_sequence_is_rejected");
}

#[test]
fn content_policy_empty_list_fails_min_items() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    matrix_cell(shipped, "content_policy_empty_list_fails_min_items");
}

#[test]
fn rule_empty_string_is_rejected() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    matrix_cell(shipped, "rule_empty_string_is_rejected");
}

#[test]
fn rule_empty_list_is_rejected() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    matrix_cell(shipped, "rule_empty_list_is_rejected");
}

#[test]
fn rule_empty_map_is_rejected() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    matrix_cell(shipped, "rule_empty_map_is_rejected");
}

#[test]
fn action_empty_string_is_rejected() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    matrix_cell(shipped, "action_empty_string_is_rejected");
}

#[test]
fn action_empty_list_is_rejected() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    matrix_cell(shipped, "action_empty_list_is_rejected");
}

#[test]
fn action_empty_map_is_rejected() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    matrix_cell(shipped, "action_empty_map_is_rejected");
}

#[test]
fn content_policy_duplicate_key_fails_loading() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    matrix_cell(shipped, "content_policy_duplicate_key_fails_loading");
}

#[test]
fn rule_duplicate_key_fails_loading() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    matrix_cell(shipped, "rule_duplicate_key_fails_loading");
}

#[test]
fn action_duplicate_key_fails_loading() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    matrix_cell(shipped, "action_duplicate_key_fails_loading");
}

#[test]
fn content_policy_unterminated_flow_sequence_fails_loading() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    matrix_cell(
        shipped,
        "content_policy_unterminated_flow_sequence_fails_loading",
    );
}

#[test]
fn trailing_frontmatter_like_body_stays_legal() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    matrix_cell(shipped, "trailing_frontmatter_like_body_stays_legal");
}

#[test]
fn rule_unterminated_quote_fails_loading() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    matrix_cell(shipped, "rule_unterminated_quote_fails_loading");
}

#[test]
fn action_nested_mapping_value_fails_loading() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let _ = include_str!("../../../lib/tests/union_fixtures/mod.rs");
    matrix_cell(shipped, "action_nested_mapping_value_fails_loading");
}
