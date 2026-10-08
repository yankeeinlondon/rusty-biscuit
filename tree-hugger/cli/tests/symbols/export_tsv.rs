use crate::Scratch;

fn parse_rows(text: &str) -> Vec<Vec<String>> {
    text.lines().skip(1).map(|line| line.split('\t').map(str::to_owned).collect()).collect()
}

#[test]
fn tsv_header_empty_row_shapes_and_comment_error() {
    let scratch = Scratch::new(&[("src/lib.rs", "pub fn alpha(x: i32) {}\n")]);
    let empty = scratch.stdout(&["functions", "--format", "tsv", "--name", "absent"]);
    assert_eq!(empty, "file\tline\tcolumn\tkind\tname\tqualified_name\tvisibility\tsignature\n");

    let output = scratch.stdout(&["functions", "--format", "tsv", "--signatures", "none"]);
    let rows = parse_rows(&output);
    assert!(rows.iter().all(|row| row.len() == 8));
    assert_eq!(&rows[0][1..3], ["1", "1"], "declaration starts at the first source byte");
    assert!(rows.iter().all(|row| row[7].is_empty()));

    let error = scratch.hug(&["functions", "--format", "tsv", "--comments"]);
    assert_eq!(error.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&error.stderr).contains("--comments"));
}

#[test]
fn tsv_matches_jsonl_record_fields_and_signature_modes() {
    let scratch = Scratch::new(&[("src/lib.rs", "pub fn alpha(x: i32) -> i32 { x }\npub fn beta() {}\n")]);
    let tsv = scratch.stdout(&["functions", "--format", "tsv", "--sort", "name"]);
    let jsonl = scratch.stdout(&["functions", "--jsonl", "--sort", "name"]);
    let json_records: Vec<_> = jsonl.lines().map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap()).collect();
    let rows = parse_rows(&tsv);
    assert_eq!(rows.len(), json_records.len());
    for (row, record) in rows.iter().zip(&json_records) {
        for (cell, key) in row.iter().zip(["file", "line", "column", "kind", "name", "qualified_name"]) {
            let expected = match key {
                "file" => record[key].as_str().unwrap().to_owned(),
                "line" => record["record"]["source"]["declaration_span"]["start"]["line"].as_u64().unwrap().to_string(),
                "column" => record["record"]["source"]["declaration_span"]["start"]["column"].as_u64().unwrap().to_string(),
                "name" | "qualified_name" => record["record"]["identity"][key].as_str().unwrap().to_owned(),
                "kind" => match record["record"][key].as_str().unwrap() {
                    "Function" => "function".to_owned(), "Method" => "method".to_owned(),
                    "Constructor" => "constructor".to_owned(), other => other.to_owned(),
                },
                _ => record["record"][key].as_str().map(str::to_owned).unwrap_or_else(|| record["record"][key].to_string()),
            };
            assert_eq!(cell, &expected, "field {key}");
        }
    }

    let none = parse_rows(&scratch.stdout(&["functions", "--format", "tsv", "--signatures", "none"]));
    assert!(none.iter().all(|row| row[7].is_empty()));
    let short = parse_rows(&scratch.stdout(&["functions", "--format", "tsv", "--signatures", "short"]));
    let full = parse_rows(&scratch.stdout(&["functions", "--format", "tsv", "--signatures", "full"]));
    assert!(short.iter().all(|row| !row[7].is_empty()));
    assert!(full.iter().all(|row| !row[7].is_empty()));
}

#[test]
fn tsv_encodes_cells_once_and_keeps_file_order_and_grouping_silent() {
    let scratch = Scratch::new(&[
        ("z.rs", "pub fn zulu() {}\n"),
        ("a.rs", "pub fn alpha() {}\n"),
    ]);
    let ordinary = parse_rows(&scratch.stdout(&["functions", "--format", "tsv"]));
    let grouped = parse_rows(&scratch.stdout(&["functions", "--format", "tsv", "--group-by-file"]));
    assert_eq!(ordinary, grouped);
    assert_eq!(ordinary.iter().map(|row| row[0].as_str()).collect::<Vec<_>>(), ["a.rs", "z.rs"]);
    let in_file = Scratch::new(&[("one.rs", "pub fn zulu() {}\npub fn alpha() {}\n")]);
    let sorted = parse_rows(&in_file.stdout(&["functions", "--format", "tsv", "--sort", "name", "--group-by-file", "--group-by-module"]));
    assert_eq!(sorted.iter().map(|row| row[4].as_str()).collect::<Vec<_>>(), ["alpha", "zulu"]);

    let ansi_name = "src/\u{9b}31mfile.rs";
    let weird = Scratch::new(&[(ansi_name, "pub fn sample() {}\n")]);
    let encoded = weird.stdout(&["functions", "--format", "tsv"]);
    assert!(!encoded.as_bytes().contains(&0x1b));
    assert!(!encoded.contains('\u{9b}'));
    assert!(encoded.contains("\\u{9b}31mfile.rs"));
}

#[test]
fn classes_tsv_flattens_only_selected_static_or_instance_members() {
    let scratch = Scratch::new(&[("Example.java", "class Box { static void build() {} void open() {} int field; }\n")]);
    let static_rows = parse_rows(&scratch.stdout(&["classes", "--format", "tsv", "--static-only"]));
    let instance_rows = parse_rows(&scratch.stdout(&["classes", "--format", "tsv", "--instance-only"]));
    let static_names: Vec<_> = static_rows.iter().map(|row| row[4].as_str()).collect();
    let instance_names: Vec<_> = instance_rows.iter().map(|row| row[4].as_str()).collect();
    assert!(static_names.contains(&"Box"));
    assert!(static_names.contains(&"build"));
    assert!(!static_names.contains(&"open"));
    assert!(instance_names.contains(&"Box"));
    assert!(instance_names.contains(&"open"));
    assert!(!instance_names.contains(&"build"));
    assert!(!static_names.contains(&"field") && !instance_names.contains(&"field"));
}
