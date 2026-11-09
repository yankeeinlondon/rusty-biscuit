//! `load_data`'s options at execution, through the registered function:
//! the effective format, delimiter, and header; option and row-shape failures
//! before anything is read; and reuse across the raw, parse, and bind layers.

use std::path::Path;

use serde_json::{Value, json};

use super::{DataError, DataRuntimeStats, DataSourceIdentity, Observation, ObservationKind};
use crate::markdown::compose::expression::functions::call_function;
use crate::markdown::compose::expression::{ExpressionError, ResolutionContext};

fn context(dir: &Path) -> ResolutionContext {
    ResolutionContext::new(biscuit_file::FileResolutionContext::new(dir))
}

fn call(context: &ResolutionContext, args: &[Value]) -> Result<Value, ExpressionError> {
    call_function("load_data", args, Some(context)).expect("registered")
}

fn data_code(result: Result<Value, ExpressionError>) -> &'static str {
    match result {
        Err(ExpressionError::Data(error)) => error.code().expect("a data code"),
        other => panic!("not a data error: {other:?}"),
    }
}

fn counts(stats: DataRuntimeStats) -> (usize, usize, usize) {
    (stats.reads, stats.parses, stats.binds)
}

#[test]
fn csv_format_with_a_semicolon_reads_a_txt_file() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("rows.txt"), "name;port\nweb;8080\n").unwrap();
    let context = context(dir.path());
    let rows = call(&context, &[json!("rows.txt"), json!("{name: string, port: number}[]"), json!({"format": "csv", "delimiter": ";"})]);
    assert_eq!(rows.unwrap(), json!([{"name": "web", "port": 8080}]));
    // Without a format the extension is not a data format.
    assert_eq!(data_code(call(&context, &[json!("rows.txt"), json!("tuple([string, string])[]")])), "dm.data.unsupported_format");
}

#[test]
fn two_delimiters_read_once_and_parse_twice() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("rows.csv"), "a;b,c\n1;2,3\n").unwrap();
    let context = context(dir.path());
    let comma = call(&context, &[json!("rows.csv"), json!("tuple([string, string])[]")]).unwrap();
    let semicolon = call(&context, &[json!("rows.csv"), json!("tuple([string, string])[]"), json!({"delimiter": ";"})]).unwrap();
    assert_eq!((comma, semicolon), (json!([["1;2", "3"]]), json!([["1", "2,3"]])));
    // The default spelled out shares the default's parse and binding.
    call(&context, &[json!("rows.csv"), json!("tuple([string, string])[]"), json!({"delimiter": ",", "header": true})]).unwrap();
    let stats = context.data.stats();
    assert_eq!(counts(stats), (1, 2, 2), "{stats:?}");
    assert_eq!((stats.source_hits, stats.bind_hits), (1, 1), "{stats:?}");
}

#[test]
fn header_false_reads_the_first_record_as_a_row_from_one_parse() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("rows.csv"), "web,8080\ndb,5432\n").unwrap();
    let context = context(dir.path());
    let contract = json!("tuple([string, number])[]");
    let headerless = call(&context, &[json!("rows.csv"), contract.clone(), json!({"header": false})]).unwrap();
    assert_eq!(headerless, json!([["web", 8080], ["db", 5432]]));
    let headed = call(&context, &[json!("rows.csv"), contract]).unwrap();
    assert_eq!(headed, json!([["db", 5432]]));
    assert_eq!(counts(context.data.stats()), (1, 1, 2));
}

#[test]
fn tsv_accepts_only_a_tab_and_fails_before_reading() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("rows.tsv"), "a\tb\n1\t2\n").unwrap();
    std::fs::write(dir.path().join("rows.csv"), "a\tb\n1\t2\n").unwrap();
    let context = context(dir.path());
    for (file, options) in [("rows.tsv", json!({"delimiter": ","})), ("rows.csv", json!({"format": "tsv", "delimiter": ";"}))] {
        assert_eq!(data_code(call(&context, &[json!(file), json!("tuple([string, string])[]"), options])), "dm.data.invalid_options");
    }
    assert_eq!(context.data.stats().reads, 0);
    let tab = call(&context, &[json!("rows.tsv"), json!("tuple([string, string])[]"), json!({"delimiter": "\t"})]).unwrap();
    assert_eq!(tab, json!([["1", "2"]]));
    let csv_with_tab = call(&context, &[json!("rows.csv"), json!("tuple([string, string])[]"), json!({"format": "csv", "delimiter": "\t"})]);
    assert_eq!(csv_with_tab.unwrap(), json!([["1", "2"]]));
}

#[test]
fn tabular_options_on_a_tree_format_fail_before_reading() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("rows.json"), "[]").unwrap();
    let context = context(dir.path());
    for options in [json!({"delimiter": ";"}), json!({"header": true}), json!({"format": "yml", "header": false})] {
        let error = call(&context, &[json!("rows.json"), json!("string[]"), options.clone()]);
        let Err(ExpressionError::Data(error)) = error else { panic!("{options}: {error:?}") };
        assert!(matches!(&*error, DataError::InvalidOptions { .. }), "{options}: {error}");
        let format = if options.get("format").is_some() { "YAML" } else { "JSON" };
        assert!(error.to_string().contains(&format!("this call reads {format}")), "{error}");
    }
    let stats = context.data.stats();
    assert_eq!(stats.reads, 0, "{stats:?}");
    // The file is still a recorded dependency.
    assert_eq!(context.data.read_set(), vec![DataSourceIdentity::local(&dir.path().join("rows.json")).read_set_entry()]);
}

#[test]
fn an_explicit_format_overrides_a_supported_extension() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("rows.json"), "name,port\nweb,8080\n").unwrap();
    let context = context(dir.path());
    let rows = call(&context, &[json!("rows.json"), json!("{name: string, port: number}[]"), json!({"format": "csv"})]);
    assert_eq!(rows.unwrap(), json!([{"name": "web", "port": 8080}]));
}

#[test]
fn headerless_object_rows_fail_before_reading() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("rows.csv"), "a\n1\n").unwrap();
    std::fs::write(dir.path().join("row.yaml"), "kind: schema\ntypes:\n  Row: \"{a: string}[]\"\n").unwrap();
    let context = context(dir.path());
    for contract in ["{a: string}[]", "Row@./row.yaml"] {
        let error = call(&context, &[json!("rows.csv"), json!(contract), json!({"header": false})]);
        let Err(ExpressionError::Data(error)) = error else { panic!("{contract}: {error:?}") };
        assert!(error.to_string().contains("`header: false` reads rows by position"), "{contract}: {error}");
        assert_eq!(error.code(), Some("dm.data.row_shape_conflict"));
    }
    assert_eq!(context.data.stats().reads, 0);
}

#[test]
fn a_null_file_ignores_its_options_and_reads_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let context = context(dir.path());
    for options in [json!({"format": "csv", "header": false}), json!({"delimiter": ";;"}), json!({"format": "json", "delimiter": ";"})] {
        assert_eq!(call(&context, &[Value::Null, json!("{a: string}[]"), options]).unwrap(), Value::Null);
    }
    let stats = context.data.stats();
    assert_eq!((stats.reads, stats.contract_preparations), (0, 0));
    assert!(context.data.read_set().is_empty());
}

#[test]
fn null_options_and_null_fields_are_the_defaults() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("rows.csv"), "a,b\n1,2\n").unwrap();
    let context = context(dir.path());
    for options in [Value::Null, json!({}), json!({"format": null, "delimiter": null, "header": null})] {
        assert_eq!(call(&context, &[json!("rows.csv"), json!("tuple([string, string])[]"), options]).unwrap(), json!([["1", "2"]]));
    }
    assert_eq!(counts(context.data.stats()), (1, 1, 1));
}

#[test]
fn aliases_share_the_canonical_formats_entries() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("os.txt"), "- macOS\n").unwrap();
    std::fs::write(dir.path().join("lines.txt"), "\"a\"\n\"b\"\n").unwrap();
    let context = context(dir.path());
    for (file, alias, canonical, value) in [("os.txt", "yml", "yaml", json!(["macOS"])), ("lines.txt", "ndjson", "jsonl", json!(["a", "b"]))] {
        for format in [alias, canonical] {
            assert_eq!(call(&context, &[json!(file), json!("string[]"), json!({"format": format})]).unwrap(), value);
        }
    }
    let stats = context.data.stats();
    assert_eq!((stats.reads, stats.parses, stats.binds, stats.source_hits, stats.bind_hits), (2, 2, 2, 2, 2), "{stats:?}");
}

#[test]
fn invalid_utf8_is_read_once_and_each_format_names_itself() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("data.txt"), b"ok\xFF\n").unwrap();
    let context = context(dir.path());
    for (format, name) in [("yaml", biscuit_file::DataFormat::Yaml), ("json", biscuit_file::DataFormat::Json)] {
        let error = call(&context, &[json!("data.txt"), json!("any[]"), json!({"format": format})]);
        let Err(ExpressionError::Data(error)) = error else { panic!("{error:?}") };
        let DataError::Malformed { format, location, .. } = &*error else { panic!("{error}") };
        assert_eq!((*format, location.byte_offset), (name, Some(2)));
    }
    assert_eq!(context.data.stats().reads, 1);
}

#[test]
fn every_call_observes_the_read_whichever_layer_answers() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("rows.csv");
    std::fs::write(&file, "a\n1\n").unwrap();
    let context = context(dir.path());
    let expected = Observation {
        entry: DataSourceIdentity::local(&file).read_set_entry(),
        kind: ObservationKind::File,
        hash: Some(biscuit_hash::blake3_hash_bytes(b"a\n1\n")),
    };
    // A cold call, a raw hit (new delimiter), a parse hit (new header), and
    // a bind hit each observe the same read once.
    for options in [json!({}), json!({"delimiter": ";"}), json!({"header": false}), json!({"header": false})] {
        let mark = context.data.read_mark();
        call(&context, &[json!("rows.csv"), json!("tuple([string])[]"), options.clone()]).unwrap();
        assert_eq!(context.data.observations_since(mark), vec![expected.clone()], "{options}");
    }
    assert_eq!(counts(context.data.stats()), (1, 2, 3));
}

#[test]
fn invalidation_rereads_every_layer() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("rows.csv");
    std::fs::write(&file, "a\n1\n").unwrap();
    let context = context(dir.path());
    let args = [json!("rows.csv"), json!("tuple([string])[]"), json!({"header": false})];
    assert_eq!(call(&context, &args).unwrap(), json!([["a"], ["1"]]));
    let generation = context.data.memo_generation();
    context.data.invalidate_memo();
    assert_eq!(context.data.memo_generation(), generation + 1);
    std::fs::write(&file, "b\n2\n").unwrap();
    assert_eq!(call(&context, &args).unwrap(), json!([["b"], ["2"]]));
    let stats = context.data.stats();
    assert_eq!((counts(stats), stats.source_hits, stats.bind_hits), ((2, 2, 2), 0, 0), "{stats:?}");
}

