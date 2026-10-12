//! The document-data Input Robustness Matrix for a `content_policy` property
//! typed `policy[](min(1))@./content-policy.yaml`, through `md schema
//! validate --format json`.
//!
//! Columns are `content_policy` itself and the object item's `rule` and
//! `action`. Each cell is one textual edit of the control document, and each
//! has its own test in `schema_union_matrix_cells.rs`, because a spawn costs
//! too much to loop every cell in one test. Every cell also asserts that validation left the file's bytes
//! unchanged, so an absent key stays absent and a null stays null. Duplicate
//! keys and malformed YAML go through `md`'s real frontmatter loader.

use crate::schema_union_pipeline::{
    Expected, assert_validates_unchanged, content_policy_document, content_policy_workspace,
};

use Expected::{Invalid, LoadError, Valid};

/// The rows of the matrix, in table order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Row {
    Control,
    Absent,
    ExplicitNull,
    WrongWholeType,
    OneWrongElement,
    EveryElementWrong,
    Empty,
    DuplicateKey,
    TrailingOrInvalid,
}

const ROWS: [Row; 8] = [
    Row::Absent,
    Row::ExplicitNull,
    Row::WrongWholeType,
    Row::OneWrongElement,
    Row::EveryElementWrong,
    Row::Empty,
    Row::DuplicateKey,
    Row::TrailingOrInvalid,
];

/// The columns of the matrix. The control row covers all three at once.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Column {
    All,
    ContentPolicy,
    Rule,
    Action,
}

const COLUMNS: [Column; 3] = [Column::ContentPolicy, Column::Rule, Column::Action];

/// The control frontmatter: a scalar item and an object item.
const CONTROL: &str = "content_policy:\n  - \"ValidFor(3mo)\"\n  - rule: \"ValidUntil(2027-01-01)\"\n    action: archive\n";
const BODY: &str = "Body with a [link](./notes.md).\n";
const RULE: &str = "rule: \"ValidUntil(2027-01-01)\"";
const ACTION: &str = "action: archive";

struct Cell {
    id: &'static str,
    row: Row,
    column: Column,
    /// Replaced exactly once in the control document by `to`.
    from: &'static str,
    to: &'static str,
    expected: Expected,
}

const fn cell(
    id: &'static str,
    (row, column): (Row, Column),
    from: &'static str,
    to: &'static str,
    expected: Expected,
) -> Cell {
    Cell {
        id,
        row,
        column,
        from,
        to,
        expected,
    }
}

use Column::{Action as A, ContentPolicy as CP, Rule as R};

const CELLS: &[Cell] = &[
    cell(
        "control_document_validates",
        (Row::Control, Column::All),
        CONTROL,
        CONTROL,
        Valid,
    ),
    // Absent: the optional property stays valid; a required field is missing,
    // which fails every arm and is reported at the item.
    cell(
        "content_policy_absent_is_valid",
        (Row::Absent, CP),
        CONTROL,
        "",
        Valid,
    ),
    cell(
        "rule_absent_is_reported_at_the_item",
        (Row::Absent, R),
        "  - rule: \"ValidUntil(2027-01-01)\"\n    action: archive\n",
        "  - action: archive\n",
        Invalid(&["/content_policy/1"]),
    ),
    cell(
        "action_absent_is_reported_at_the_item",
        (Row::Absent, A),
        "\n    action: archive",
        "",
        Invalid(&["/content_policy/1"]),
    ),
    // Explicit null.
    cell(
        "content_policy_null_is_valid",
        (Row::ExplicitNull, CP),
        CONTROL,
        "content_policy: null\n",
        Valid,
    ),
    cell(
        "rule_null_is_rejected",
        (Row::ExplicitNull, R),
        RULE,
        "rule: null",
        Invalid(&["/content_policy/1/rule"]),
    ),
    cell(
        "action_null_is_rejected",
        (Row::ExplicitNull, A),
        ACTION,
        "action: null",
        Invalid(&["/content_policy/1/action"]),
    ),
    // Wrong whole type.
    cell(
        "content_policy_scalar_is_rejected",
        (Row::WrongWholeType, CP),
        CONTROL,
        "content_policy: \"ValidFor(3mo)\"\n",
        Invalid(&["/content_policy"]),
    ),
    cell(
        "rule_number_is_rejected",
        (Row::WrongWholeType, R),
        RULE,
        "rule: 1",
        Invalid(&["/content_policy/1/rule"]),
    ),
    cell(
        "action_number_is_rejected",
        (Row::WrongWholeType, A),
        ACTION,
        "action: 1",
        Invalid(&["/content_policy/1/action"]),
    ),
    // One wrong element: the valid items are kept, not filtered.
    cell(
        "content_policy_one_numeric_item_fails_at_its_index",
        (Row::OneWrongElement, CP),
        "    action: archive\n",
        "    action: archive\n  - 1\n",
        Invalid(&["/content_policy/2"]),
    ),
    cell(
        "rule_mixed_sequence_is_rejected",
        (Row::OneWrongElement, R),
        RULE,
        "rule: [\"ValidUntil(2027-01-01)\", 1]",
        Invalid(&["/content_policy/1/rule"]),
    ),
    cell(
        "action_mixed_sequence_is_rejected",
        (Row::OneWrongElement, A),
        ACTION,
        "action: [archive, 1]",
        Invalid(&["/content_policy/1/action"]),
    ),
    // Every element wrong: each item fails, never an empty-list substitution.
    cell(
        "content_policy_numeric_items_each_fail",
        (Row::EveryElementWrong, CP),
        CONTROL,
        "content_policy: [1, 2]\n",
        Invalid(&["/content_policy/0", "/content_policy/1"]),
    ),
    cell(
        "rule_numeric_sequence_is_rejected",
        (Row::EveryElementWrong, R),
        RULE,
        "rule: [1, 2]",
        Invalid(&["/content_policy/1/rule"]),
    ),
    cell(
        "action_numeric_sequence_is_rejected",
        (Row::EveryElementWrong, A),
        ACTION,
        "action: [1, 2]",
        Invalid(&["/content_policy/1/action"]),
    ),
    // Empty.
    cell(
        "content_policy_empty_list_fails_min_items",
        (Row::Empty, CP),
        CONTROL,
        "content_policy: []\n",
        Invalid(&["/content_policy"]),
    ),
    cell(
        "rule_empty_string_is_rejected",
        (Row::Empty, R),
        RULE,
        "rule: \"\"",
        Invalid(&["/content_policy/1/rule"]),
    ),
    cell(
        "rule_empty_list_is_rejected",
        (Row::Empty, R),
        RULE,
        "rule: []",
        Invalid(&["/content_policy/1/rule"]),
    ),
    cell(
        "rule_empty_map_is_rejected",
        (Row::Empty, R),
        RULE,
        "rule: {}",
        Invalid(&["/content_policy/1/rule"]),
    ),
    cell(
        "action_empty_string_is_rejected",
        (Row::Empty, A),
        ACTION,
        "action: \"\"",
        Invalid(&["/content_policy/1/action"]),
    ),
    cell(
        "action_empty_list_is_rejected",
        (Row::Empty, A),
        ACTION,
        "action: []",
        Invalid(&["/content_policy/1/action"]),
    ),
    cell(
        "action_empty_map_is_rejected",
        (Row::Empty, A),
        ACTION,
        "action: {}",
        Invalid(&["/content_policy/1/action"]),
    ),
    // Duplicate key: the loader refuses it rather than keeping the last value.
    cell(
        "content_policy_duplicate_key_fails_loading",
        (Row::DuplicateKey, CP),
        "    action: archive\n",
        "    action: archive\ncontent_policy: null\n",
        LoadError,
    ),
    cell(
        "rule_duplicate_key_fails_loading",
        (Row::DuplicateKey, R),
        ACTION,
        "rule: \"ValidFor(3mo)\"\n    action: archive",
        LoadError,
    ),
    cell(
        "action_duplicate_key_fails_loading",
        (Row::DuplicateKey, A),
        ACTION,
        "action: archive\n    action: refresh",
        LoadError,
    ),
    // Trailing / invalid content: malformed frontmatter fails loading, while
    // frontmatter-like text after the closing fence is ordinary body.
    cell(
        "content_policy_unterminated_flow_sequence_fails_loading",
        (Row::TrailingOrInvalid, CP),
        CONTROL,
        "content_policy: [\"ValidFor(3mo)\"\n",
        LoadError,
    ),
    cell(
        "trailing_frontmatter_like_body_stays_legal",
        (Row::TrailingOrInvalid, CP),
        BODY,
        "Body with a [link](./notes.md).\n\n---\ncontent_policy: 1\n---\n",
        Valid,
    ),
    cell(
        "rule_unterminated_quote_fails_loading",
        (Row::TrailingOrInvalid, R),
        RULE,
        "rule: \"ValidUntil(2027-01-01)",
        LoadError,
    ),
    cell(
        "action_nested_mapping_value_fails_loading",
        (Row::TrailingOrInvalid, A),
        ACTION,
        "action: archive: extra",
        LoadError,
    ),
];

/// Applies `id`'s edit to the control document and asserts its outcome.
pub(crate) fn matrix_cell(shipped: &str, id: &str) {
    let cell = CELLS
        .iter()
        .find(|cell| cell.id == id)
        .unwrap_or_else(|| panic!("no matrix cell `{id}`"));
    let control = content_policy_document(CONTROL);
    assert_eq!(
        control.matches(cell.from).count(),
        1,
        "`{id}` must edit text that occurs exactly once in the control document",
    );
    let document = control.replacen(cell.from, cell.to, 1);
    if cell.row != Row::Control {
        assert_ne!(document, control, "`{id}` must change the control document");
    }
    let (process, doc) = content_policy_workspace(shipped, &document);
    assert_validates_unchanged(&process, &doc, cell.expected);
}

#[test]
fn matrix_covers_every_row_and_column() {
    let controls = CELLS.iter().filter(|cell| cell.row == Row::Control).count();
    assert_eq!(controls, 1, "exactly one control cell");
    for row in ROWS {
        for column in COLUMNS {
            assert!(
                CELLS
                    .iter()
                    .any(|cell| cell.row == row && cell.column == column),
                "no cell for {row:?} × {column:?}",
            );
        }
    }
    for (index, cell) in CELLS.iter().enumerate() {
        assert!(
            CELLS[index + 1..].iter().all(|other| other.id != cell.id),
            "duplicate cell id `{}`",
            cell.id,
        );
    }
}

#[test]
fn every_matrix_cell_has_its_own_test() {
    // Whitespace is dropped so a call the formatter wraps still matches.
    let source: String = include_str!("schema_union_matrix_cells.rs")
        .split_whitespace()
        .collect();
    assert_eq!(
        source.matches("matrix_cell(shipped,\"").count(),
        CELLS.len(),
        "every cell test must name a cell in CELLS"
    );
    for cell in CELLS {
        let id = cell.id;
        let header = format!("fn{id}(){{");
        let call = format!("matrix_cell(shipped,\"{id}\"");
        assert_eq!(
            source.matches(&header).count(),
            1,
            "`{id}` needs exactly one test"
        );
        assert_eq!(
            source.matches(&call).count(),
            1,
            "`{id}`'s test must run it"
        );
    }
}
