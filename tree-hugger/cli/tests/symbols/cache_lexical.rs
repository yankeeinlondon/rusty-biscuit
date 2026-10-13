//! Hermetic shipped CLI consumers of the shared lexical import corpus.
use serde_json::json;
use crate::{Scratch, json_of, stdout_of};
#[path = "../../../lib/tests/fixtures/scoped_import_lexical.rs"]
mod corpus;
fn run(extension: &'static str, partition: usize, report_index: usize) {
    for case in corpus::cases(extension).into_iter().skip(partition).take(1) {
        let file = format!("source.{}", case.extension);
        let scratch = Scratch::new(&[(&file, &case.source)]);
        let full = json_of(&scratch.stdout(&["--json-full", "symbols", &file]));
        let index = &full["indexes"][0];
        assert_eq!(index["imports"].as_array().expect("imports").len(), usize::from(case.import.is_some()), "{extension} {}", case.label);
        if let Some((name, module)) = case.import { assert_eq!(index["imports"][0]["name"], name); assert_eq!(index["imports"][0]["source"], module); }
        let id = index["symbols"].as_array().expect("symbols").iter().find(|s| s["identity"]["name"] == "f").expect("function")["id"].as_str().expect("ID");
        let entry = crate::cache_upgrade::only_entry(scratch.temp.path());
        let saved = crate::cache_upgrade::read_json(&entry);
        let mut changed = saved.clone();
        if case.import.is_some() { changed["imports"] = json!([]); }
        else {
            let start = case.source.rfind('f').expect("token");
            let point = |byte: usize| { let before = &case.source[..byte]; json!({"line":before.matches('\n').count()+1,"column":byte-before.rfind('\n').map_or(0, |at| at+1)+1}) };
            let span = json!({"start_byte":start,"end_byte":start+1,"start":point(start),"end":point(start+1)});
            changed["imports"] = json!([{"name":"f","alias":null,"source":null,"span":span,"statement_span":span,"symbol":{"name":"f","qualified_name":null,"id":null,"source_span":null}}]);
        }
        let mut reports = vec![vec!["--json", "imports", &file], vec!["--plain", "imports", &file], vec!["--json-full", "symbols", &file], vec!["--plain", "symbols", &file]];
        for format in ["--json", "--plain"] {
            for direction in [None, Some("--callers"), Some("--callees")] {
                let mut args = vec![format, "refs", "--id", id]; if let Some(direction) = direction { args.push(direction); } args.push(&file); reports.push(args);
            }
        }
        assert_eq!(reports.len(), REPORT_COUNT, "every report needs a scheduled consumer");
        for report in reports.into_iter().skip(report_index).take(1) {
            let mut uncached = report.clone(); uncached.insert(2, "--no-cache");
            let expected = scratch.stdout(&uncached);
            if report[0] == "--json" && report[1] == "refs" && case.import.is_none() {
                let json = json_of(&expected);
                for field in ["occurrences", "name_only_suggestions"] {
                    if let Some(rows) = json["files"][0][field].as_array() { assert!(!rows.iter().any(|row| matches!(row["target"]["name"].as_str(), Some("D" | "A" | "B"))), "{extension} {}: {json}", case.label); }
                }
            }
            for edited in [false, true] {
                std::fs::write(&entry, serde_json::to_vec(if edited { &changed } else { &saved }).expect("JSON")).expect("restore entry");
                let mut args = vec!["--verbose"]; args.extend_from_slice(&report);
                for warm in [false, true] {
                    let output = scratch.hug(&args);
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let counts = if edited && !warm { "0 hits, 0 partial hits, 1 misses; parsed 1 files" } else { "1 hits, 0 partial hits, 0 misses; parsed 0 files" };
                    assert!(stderr.contains(counts), "{extension} {} {report:?} edited={edited}: {stderr}", case.label);
                    assert_eq!(stdout_of(output, &args), expected, "{extension} {} {report:?}", case.label);
                }
            }
        }
    }
}
const REPORT_COUNT: usize = 10;

macro_rules! consumer_tests {
    ($ext:literal, $source:literal; $($test:ident => $report:literal),* $(,)?) => {
        pub(super) const CONSUMERS: &[usize] = &[$($report),*];
        $(#[test] fn $test() { run($ext, $source, $report); })*
    };
}

macro_rules! lexical_tests {
    ($($name:ident => ($ext:literal, $source:literal)),* $(,)?) => {
        const REGISTERED: &[(&str, usize, &[usize])] = &[$(($ext, $source, $name::CONSUMERS)),*];
        $(mod $name {
            use super::*;
            consumer_tests! { $ext, $source;
                json_imports => 0,
                plain_imports => 1,
                json_full_symbols => 2,
                plain_symbols => 3,
                json_refs => 4,
                json_callers => 5,
                json_callees => 6,
                plain_refs => 7,
                plain_callers => 8,
                plain_callees => 9,
            }
        })*
    };
}

lexical_tests! {
    lexical_inventory_reports_ts_0 => ("ts", 0),
    lexical_inventory_reports_ts_1 => ("ts", 1),
    lexical_inventory_reports_ts_2 => ("ts", 2),
    lexical_inventory_reports_ts_3 => ("ts", 3),
    lexical_inventory_reports_ts_4 => ("ts", 4),
    lexical_inventory_reports_ts_5 => ("ts", 5),
    lexical_inventory_reports_ts_6 => ("ts", 6),
    lexical_inventory_reports_ts_7 => ("ts", 7),
    lexical_inventory_reports_ts_8 => ("ts", 8),
    lexical_inventory_reports_ts_9 => ("ts", 9),
    lexical_inventory_reports_ts_10 => ("ts", 10),
    lexical_inventory_reports_ts_11 => ("ts", 11),
    lexical_inventory_reports_ts_12 => ("ts", 12),
    lexical_inventory_reports_ts_13 => ("ts", 13),
    lexical_inventory_reports_ts_14 => ("ts", 14),
    lexical_inventory_reports_ts_15 => ("ts", 15),
    lexical_inventory_reports_ts_16 => ("ts", 16),
    lexical_inventory_reports_ts_17 => ("ts", 17),
    lexical_inventory_reports_ts_18 => ("ts", 18),
    lexical_inventory_reports_ts_19 => ("ts", 19),
    lexical_inventory_reports_ts_20 => ("ts", 20),
    lexical_inventory_reports_ts_21 => ("ts", 21),
    lexical_inventory_reports_ts_22 => ("ts", 22),
    lexical_inventory_reports_ts_23 => ("ts", 23),
    lexical_inventory_reports_ts_24 => ("ts", 24),
    lexical_inventory_reports_ts_25 => ("ts", 25),
    lexical_inventory_reports_ts_26 => ("ts", 26),
    lexical_inventory_reports_ts_27 => ("ts", 27),
    lexical_inventory_reports_ts_28 => ("ts", 28),
    lexical_inventory_reports_ts_29 => ("ts", 29),
    lexical_inventory_reports_ts_30 => ("ts", 30),
    lexical_inventory_reports_ts_31 => ("ts", 31),
    lexical_inventory_reports_ts_32 => ("ts", 32),
    lexical_inventory_reports_ts_33 => ("ts", 33),
    lexical_inventory_reports_ts_34 => ("ts", 34),
    lexical_inventory_reports_ts_35 => ("ts", 35),
    lexical_inventory_reports_ts_36 => ("ts", 36),
    lexical_inventory_reports_ts_37 => ("ts", 37),
    lexical_inventory_reports_ts_38 => ("ts", 38),
    lexical_inventory_reports_ts_39 => ("ts", 39),
    lexical_inventory_reports_ts_40 => ("ts", 40),
    lexical_inventory_reports_ts_41 => ("ts", 41),
    lexical_inventory_reports_ts_42 => ("ts", 42),
    lexical_inventory_reports_ts_43 => ("ts", 43),
    lexical_inventory_reports_ts_44 => ("ts", 44),
    lexical_inventory_reports_ts_45 => ("ts", 45),
    lexical_inventory_reports_ts_46 => ("ts", 46),
    lexical_inventory_reports_ts_47 => ("ts", 47),
    lexical_inventory_reports_ts_48 => ("ts", 48),
    lexical_inventory_reports_ts_49 => ("ts", 49),
    lexical_inventory_reports_js_0 => ("js", 0),
    lexical_inventory_reports_js_1 => ("js", 1),
    lexical_inventory_reports_js_2 => ("js", 2),
    lexical_inventory_reports_js_3 => ("js", 3),
    lexical_inventory_reports_js_4 => ("js", 4),
    lexical_inventory_reports_js_5 => ("js", 5),
    lexical_inventory_reports_js_6 => ("js", 6),
    lexical_inventory_reports_js_7 => ("js", 7),
    lexical_inventory_reports_js_8 => ("js", 8),
    lexical_inventory_reports_js_9 => ("js", 9),
    lexical_inventory_reports_js_10 => ("js", 10),
    lexical_inventory_reports_js_11 => ("js", 11),
    lexical_inventory_reports_js_12 => ("js", 12),
    lexical_inventory_reports_js_13 => ("js", 13),
    lexical_inventory_reports_js_14 => ("js", 14),
    lexical_inventory_reports_js_15 => ("js", 15),
    lexical_inventory_reports_js_16 => ("js", 16),
    lexical_inventory_reports_js_17 => ("js", 17),
    lexical_inventory_reports_js_18 => ("js", 18),
    lexical_inventory_reports_js_19 => ("js", 19),
    lexical_inventory_reports_js_20 => ("js", 20),
    lexical_inventory_reports_js_21 => ("js", 21),
    lexical_inventory_reports_js_22 => ("js", 22),
    lexical_inventory_reports_js_23 => ("js", 23),
    lexical_inventory_reports_js_24 => ("js", 24),
    lexical_inventory_reports_js_25 => ("js", 25),
    lexical_inventory_reports_js_26 => ("js", 26),
    lexical_inventory_reports_js_27 => ("js", 27),
    lexical_inventory_reports_js_28 => ("js", 28),
    lexical_inventory_reports_js_29 => ("js", 29),
    lexical_inventory_reports_js_30 => ("js", 30),
    lexical_inventory_reports_js_31 => ("js", 31),
    lexical_inventory_reports_js_32 => ("js", 32),
    lexical_inventory_reports_js_33 => ("js", 33),
    lexical_inventory_reports_js_34 => ("js", 34),
    lexical_inventory_reports_js_35 => ("js", 35),
    lexical_inventory_reports_js_36 => ("js", 36),
    lexical_inventory_reports_js_37 => ("js", 37),
    lexical_inventory_reports_js_38 => ("js", 38),
    lexical_inventory_reports_js_39 => ("js", 39),
    lexical_inventory_reports_js_40 => ("js", 40),
    lexical_inventory_reports_js_41 => ("js", 41),
    lexical_inventory_reports_js_42 => ("js", 42),
    lexical_inventory_reports_js_43 => ("js", 43),
    lexical_inventory_reports_js_44 => ("js", 44),
    lexical_inventory_reports_js_45 => ("js", 45),
    lexical_inventory_reports_js_46 => ("js", 46),
    lexical_inventory_reports_js_47 => ("js", 47),
    lexical_inventory_reports_js_48 => ("js", 48),
    lexical_inventory_reports_js_49 => ("js", 49),
    lexical_inventory_reports_swift_0 => ("swift", 0),
    lexical_inventory_reports_swift_1 => ("swift", 1),
    lexical_inventory_reports_swift_2 => ("swift", 2),
    lexical_inventory_reports_swift_3 => ("swift", 3),
    lexical_inventory_reports_swift_4 => ("swift", 4),
    lexical_inventory_reports_swift_5 => ("swift", 5),
    lexical_inventory_reports_cs_0 => ("cs", 0),
    lexical_inventory_reports_cs_1 => ("cs", 1),
    lexical_inventory_reports_cs_2 => ("cs", 2),
    lexical_inventory_reports_cs_3 => ("cs", 3),
    lexical_inventory_reports_cs_4 => ("cs", 4),
    lexical_inventory_reports_rs_0 => ("rs", 0),
    lexical_inventory_reports_rs_1 => ("rs", 1),
    lexical_inventory_reports_rs_2 => ("rs", 2),
    lexical_inventory_reports_rs_3 => ("rs", 3),
    lexical_inventory_reports_rs_4 => ("rs", 4),
    lexical_inventory_reports_rs_5 => ("rs", 5),
    lexical_inventory_reports_rs_6 => ("rs", 6),
    lexical_inventory_reports_java_0 => ("java", 0),
    lexical_inventory_reports_java_1 => ("java", 1),
    lexical_inventory_reports_go_0 => ("go", 0),
    lexical_inventory_reports_go_1 => ("go", 1),
    lexical_inventory_reports_scala_0 => ("scala", 0),
    lexical_inventory_reports_scala_1 => ("scala", 1),
    lexical_inventory_reports_py_0 => ("py", 0),
    lexical_inventory_reports_py_1 => ("py", 1),
    lexical_inventory_reports_py_2 => ("py", 2),
    lexical_inventory_reports_py_3 => ("py", 3),
    lexical_inventory_reports_php_0 => ("php", 0),
    lexical_inventory_reports_php_1 => ("php", 1),
    lexical_inventory_reports_php_2 => ("php", 2),
    lexical_inventory_reports_php_3 => ("php", 3),
    lexical_inventory_reports_c_0 => ("c", 0),
    lexical_inventory_reports_cpp_0 => ("cpp", 0),
    lexical_inventory_reports_lua_0 => ("lua", 0),
    lexical_inventory_reports_pl_0 => ("pl", 0),
    lexical_inventory_reports_sh_0 => ("sh", 0),
    lexical_inventory_reports_zsh_0 => ("zsh", 0),
}

#[test]
fn lexical_inventory_report_registration_covers_every_source_and_consumer() {
    let mut seen = std::collections::BTreeSet::new();
    for &(extension, source, consumers) in REGISTERED {
        assert!(source < corpus::cases(extension).len());
        assert_eq!(consumers, (0..REPORT_COUNT).collect::<Vec<_>>());
        for &consumer in consumers {
            assert!(seen.insert((extension, source, consumer)), "duplicate consumer");
        }
    }
    for extension in ["rs", "ts", "js", "go", "py", "java", "cs", "c", "cpp", "swift", "scala", "php", "pl", "sh", "zsh", "lua"] {
        for source in 0..corpus::cases(extension).len() {
            for consumer in 0..REPORT_COUNT { assert!(seen.contains(&(extension, source, consumer)), "{extension} {source} {consumer}"); }
        }
    }
}
