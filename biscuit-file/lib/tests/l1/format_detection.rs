//! Extension detection and the format enums' metadata, through the public
//! re-exports.

use std::path::Path;

use biscuit_file::{DataFormat, FileType, detect_from_extension};

#[test]
fn new_data_extensions_are_detected() {
    for (path, expected) in [
        ("rows.jsonl", FileType::Jsonl),
        ("rows.ndjson", FileType::Jsonl),
        ("table.csv", FileType::Csv),
        ("table.tsv", FileType::Tsv),
    ] {
        assert_eq!(detect_from_extension(Path::new(path)), expected, "{path}");
    }
}

#[test]
fn extension_matching_ignores_case() {
    for (path, expected) in [
        ("DATA.CSV", FileType::Csv),
        ("Data.Tsv", FileType::Tsv),
        ("ROWS.JSONL", FileType::Jsonl),
        ("rows.NDJSON", FileType::Jsonl),
        ("CONFIG.JSON", FileType::Json),
    ] {
        assert_eq!(detect_from_extension(Path::new(path)), expected, "{path}");
    }
}

#[test]
fn ndjson_and_jsonl_are_the_same_format() {
    assert_eq!(
        detect_from_extension(Path::new("a.ndjson")),
        detect_from_extension(Path::new("a.jsonl"))
    );
}

#[test]
fn existing_extensions_are_unchanged() {
    for (path, expected) in [
        ("c.toml", FileType::Toml),
        ("c.yaml", FileType::Yaml),
        ("c.yml", FileType::Yaml),
        ("d.json", FileType::Json),
        ("d.json5", FileType::Json5),
        ("r.md", FileType::Markdown),
        ("r.markdown", FileType::Markdown),
        ("r.mdx", FileType::Markdown),
        ("doc.pdf", FileType::Pdf),
    ] {
        assert_eq!(detect_from_extension(Path::new(path)), expected, "{path}");
    }
}

#[test]
fn unlisted_or_missing_extensions_are_unknown() {
    for path in ["notes.txt", "table.xlsx", "Makefile", "archive.csv.gz", ""] {
        assert_eq!(detect_from_extension(Path::new(path)), FileType::Unknown, "{path:?}");
    }
}

#[test]
fn new_file_types_carry_extension_mime_and_data_format() {
    for (file_type, extension, mime, format) in [
        (FileType::Jsonl, "jsonl", "application/x-ndjson", DataFormat::Jsonl),
        (FileType::Csv, "csv", "text/csv", DataFormat::Csv),
        (FileType::Tsv, "tsv", "text/tab-separated-values", DataFormat::Tsv),
    ] {
        assert_eq!(file_type.extension(), Some(extension));
        assert_eq!(file_type.mime_type(), Some(mime));
        assert_eq!(file_type.as_data_format(), Some(format));
        assert_eq!(FileType::from(format), file_type);
        assert_eq!(format.extension(), extension);
        assert_eq!(format.mime_type(), mime);
    }
}

#[test]
fn new_data_formats_are_structured_utf8_text() {
    for (format, display) in [
        (DataFormat::Jsonl, "JSONL"),
        (DataFormat::Csv, "CSV"),
        (DataFormat::Tsv, "TSV"),
    ] {
        assert!(format.requires_utf8(), "{format:?}");
        assert!(format.is_structured(), "{format:?}");
        assert_eq!(format.to_string(), display);
    }
}
