use super::*;

// ==================== Table Layout ====================
//
// `table-layout` (default on) pads cells so each column has one display
// width; cell contents keep their authored spelling.

#[test]
fn table_columns_are_aligned() {
    assert_eq!(
        clean("|Short|VeryLongHeader|\n|---|---|\n|A|B|"),
        "| Short | VeryLongHeader |\n| ----- | -------------- |\n| A     | B              |"
    );
}

#[test]
fn table_cells_gain_spacing() {
    assert_eq!(clean("|A|B|\n|---|---|\n|1|2|"), "| A   | B   |\n| --- | --- |\n| 1   | 2   |");
}

#[test]
fn table_layout_off_leaves_table_unchanged() {
    let content = "|A|B|\n|---|---|\n|1|2|";
    assert_eq!(clean_with(content, |overrides| overrides.table_layout = Leaf::Value(false)), content);
}

#[test]
fn table_code_spans_are_not_escaped() {
    assert_eq!(
        clean("| Name | Value |\n|---|---|\n| `foo` | bar |"),
        "| Name  | Value |\n| ----- | ----- |\n| `foo` | bar   |"
    );
}

/// Width is measured on the authored cell source, link syntax included.
#[test]
fn table_alignment_counts_link_syntax() {
    assert_eq!(
        clean(
            "| Variable | Description |\n|----------|-------------|\n| `THEME` | Default theme |\n| `NO_COLOR` | Disable colors ([no-color.org](https://no-color.org)) |"
        ),
        concat!(
            "| Variable   | Description                                           |\n",
            "| ---------- | ----------------------------------------------------- |\n",
            "| `THEME`    | Default theme                                         |\n",
            "| `NO_COLOR` | Disable colors ([no-color.org](https://no-color.org)) |",
        )
    );
}

#[test]
fn table_alignment_counts_emphasis_markers() {
    assert_eq!(
        clean(
            "| Option | Description |\n|--------|-------------|\n| `--mermaid` | *Terminal only*. Renders diagrams |\n| `--html` | Output HTML |"
        ),
        concat!(
            "| Option      | Description                       |\n",
            "| ----------- | --------------------------------- |\n",
            "| `--mermaid` | *Terminal only*. Renders diagrams |\n",
            "| `--html`    | Output HTML                       |",
        )
    );
}

#[test]
fn table_alignment_counts_strong_markers() {
    assert_eq!(
        clean("| Name | Description |\n|------|-------------|\n| Test | **Bold text** here |\n| A | Short |"),
        concat!(
            "| Name | Description        |\n",
            "| ---- | ------------------ |\n",
            "| Test | **Bold text** here |\n",
            "| A    | Short              |",
        )
    );
}

#[test]
fn table_alignment_with_mixed_formatting() {
    assert_eq!(
        clean(
            "| Column | Content |\n|--------|-------------|\n| A | `code` with *emphasis* and [link](url) |\n| B | Plain text |"
        ),
        concat!(
            "| Column | Content                                |\n",
            "| ------ | -------------------------------------- |\n",
            "| A      | `code` with *emphasis* and [link](url) |\n",
            "| B      | Plain text                             |",
        )
    );
}

/// East Asian characters are two display columns wide.
#[test]
fn table_alignment_uses_display_width() {
    assert_eq!(
        clean("| Name | Greeting |\n|------|----------|\n| 你好 | Hello |\n| A | World |"),
        concat!(
            "| Name | Greeting |\n",
            "| ---- | -------- |\n",
            "| 你好 | Hello    |\n",
            "| A    | World    |",
        )
    );
}

#[test]
fn table_after_cleanup_still_parses_as_table() {
    let cleaned = clean("| A | B |\n|---|---|\n| 1 | 2 |");
    let has_table = Parser::new_ext(&cleaned, Options::all())
        .any(|event| matches!(event, pulldown_cmark::Event::Start(pulldown_cmark::Tag::Table(_))));
    assert!(has_table, "cleaned table should still parse as a table, got:\n{cleaned}");
}
