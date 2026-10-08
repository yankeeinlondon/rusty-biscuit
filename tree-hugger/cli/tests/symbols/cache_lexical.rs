//! Hermetic shipped CLI consumers of the shared lexical import corpus.
use serde_json::json;
use crate::{Scratch, json_of, stdout_of};
#[path = "../../../lib/tests/fixtures/scoped_import_lexical.rs"]
mod corpus;
fn run(extension: &'static str, partition: usize, report_partition: usize) {
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
        let width = if extension == "cpp" { 2 } else { 5 };
        for report in reports.into_iter().skip(if report_partition == 5 { 0 } else { report_partition * width }).take(if report_partition == 5 { 10 } else { width }) {
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
macro_rules! lexical_tests { ($($name:ident => ($ext:literal, $part:literal, $report:literal)),+ $(,)?) => { const REGISTERED: &[(&str, usize, usize)] = &[$(($ext, $part, $report)),+]; $(#[test] fn $name(){run($ext, $part, $report);})+ }; }
lexical_tests! {
    lexical_inventory_reports_ts_0_0 => ("ts", 0, 0),
    lexical_inventory_reports_ts_0_1 => ("ts", 0, 1),
    lexical_inventory_reports_ts_1_0 => ("ts", 1, 0),
    lexical_inventory_reports_ts_1_1 => ("ts", 1, 1),
    lexical_inventory_reports_ts_2_0 => ("ts", 2, 0),
    lexical_inventory_reports_ts_2_1 => ("ts", 2, 1),
    lexical_inventory_reports_ts_3_0 => ("ts", 3, 0),
    lexical_inventory_reports_ts_3_1 => ("ts", 3, 1),
    lexical_inventory_reports_ts_4_0 => ("ts", 4, 0),
    lexical_inventory_reports_ts_4_1 => ("ts", 4, 1),
    lexical_inventory_reports_ts_5_0 => ("ts", 5, 0),
    lexical_inventory_reports_ts_5_1 => ("ts", 5, 1),
    lexical_inventory_reports_ts_6_0 => ("ts", 6, 0),
    lexical_inventory_reports_ts_6_1 => ("ts", 6, 1),
    lexical_inventory_reports_ts_7_0 => ("ts", 7, 0),
    lexical_inventory_reports_ts_7_1 => ("ts", 7, 1),
    lexical_inventory_reports_ts_8_0 => ("ts", 8, 0),
    lexical_inventory_reports_ts_8_1 => ("ts", 8, 1),
    lexical_inventory_reports_ts_9_0 => ("ts", 9, 0),
    lexical_inventory_reports_ts_9_1 => ("ts", 9, 1),
    lexical_inventory_reports_ts_10_0 => ("ts", 10, 0),
    lexical_inventory_reports_ts_10_1 => ("ts", 10, 1),
    lexical_inventory_reports_ts_11_0 => ("ts", 11, 0),
    lexical_inventory_reports_ts_11_1 => ("ts", 11, 1),
    lexical_inventory_reports_ts_12_0 => ("ts", 12, 0),
    lexical_inventory_reports_ts_12_1 => ("ts", 12, 1),
    lexical_inventory_reports_ts_13_0 => ("ts", 13, 0),
    lexical_inventory_reports_ts_13_1 => ("ts", 13, 1),
    lexical_inventory_reports_ts_14_0 => ("ts", 14, 0),
    lexical_inventory_reports_ts_14_1 => ("ts", 14, 1),
    lexical_inventory_reports_ts_15_0 => ("ts", 15, 0),
    lexical_inventory_reports_ts_15_1 => ("ts", 15, 1),
    lexical_inventory_reports_ts_16_0 => ("ts", 16, 0),
    lexical_inventory_reports_ts_16_1 => ("ts", 16, 1),
    lexical_inventory_reports_ts_17_0 => ("ts", 17, 0),
    lexical_inventory_reports_ts_17_1 => ("ts", 17, 1),
    lexical_inventory_reports_ts_18_0 => ("ts", 18, 0),
    lexical_inventory_reports_ts_18_1 => ("ts", 18, 1),
    lexical_inventory_reports_ts_19_0 => ("ts", 19, 0),
    lexical_inventory_reports_ts_19_1 => ("ts", 19, 1),
    lexical_inventory_reports_ts_20_0 => ("ts", 20, 0),
    lexical_inventory_reports_ts_20_1 => ("ts", 20, 1),
    lexical_inventory_reports_ts_21_0 => ("ts", 21, 0),
    lexical_inventory_reports_ts_21_1 => ("ts", 21, 1),
    lexical_inventory_reports_ts_22_0 => ("ts", 22, 0),
    lexical_inventory_reports_ts_22_1 => ("ts", 22, 1),
    lexical_inventory_reports_ts_23_0 => ("ts", 23, 0),
    lexical_inventory_reports_ts_23_1 => ("ts", 23, 1),
    lexical_inventory_reports_ts_24_0 => ("ts", 24, 0),
    lexical_inventory_reports_ts_24_1 => ("ts", 24, 1),
    lexical_inventory_reports_ts_25_0 => ("ts", 25, 0),
    lexical_inventory_reports_ts_25_1 => ("ts", 25, 1),
    lexical_inventory_reports_ts_26_0 => ("ts", 26, 0),
    lexical_inventory_reports_ts_26_1 => ("ts", 26, 1),
    lexical_inventory_reports_ts_27_0 => ("ts", 27, 0),
    lexical_inventory_reports_ts_27_1 => ("ts", 27, 1),
    lexical_inventory_reports_ts_28_0 => ("ts", 28, 0),
    lexical_inventory_reports_ts_28_1 => ("ts", 28, 1),
    lexical_inventory_reports_ts_29_0 => ("ts", 29, 0),
    lexical_inventory_reports_ts_29_1 => ("ts", 29, 1),
    lexical_inventory_reports_ts_30_0 => ("ts", 30, 0),
    lexical_inventory_reports_ts_30_1 => ("ts", 30, 1),
    lexical_inventory_reports_ts_31_0 => ("ts", 31, 0),
    lexical_inventory_reports_ts_31_1 => ("ts", 31, 1),
    lexical_inventory_reports_ts_32_0 => ("ts", 32, 0),
    lexical_inventory_reports_ts_32_1 => ("ts", 32, 1),
    lexical_inventory_reports_ts_33_0 => ("ts", 33, 0),
    lexical_inventory_reports_ts_33_1 => ("ts", 33, 1),
    lexical_inventory_reports_ts_34_0 => ("ts", 34, 0),
    lexical_inventory_reports_ts_34_1 => ("ts", 34, 1),
    lexical_inventory_reports_ts_35_0 => ("ts", 35, 0),
    lexical_inventory_reports_ts_35_1 => ("ts", 35, 1),
    lexical_inventory_reports_ts_36_0 => ("ts", 36, 0),
    lexical_inventory_reports_ts_36_1 => ("ts", 36, 1),
    lexical_inventory_reports_ts_37_0 => ("ts", 37, 0),
    lexical_inventory_reports_ts_37_1 => ("ts", 37, 1),
    lexical_inventory_reports_ts_38_0 => ("ts", 38, 0),
    lexical_inventory_reports_ts_38_1 => ("ts", 38, 1),
    lexical_inventory_reports_ts_39_0 => ("ts", 39, 0),
    lexical_inventory_reports_ts_39_1 => ("ts", 39, 1),
    lexical_inventory_reports_ts_40_0 => ("ts", 40, 0),
    lexical_inventory_reports_ts_40_1 => ("ts", 40, 1),
    lexical_inventory_reports_ts_41_0 => ("ts", 41, 0),
    lexical_inventory_reports_ts_41_1 => ("ts", 41, 1),
    lexical_inventory_reports_ts_42_0 => ("ts", 42, 0),
    lexical_inventory_reports_ts_42_1 => ("ts", 42, 1),
    lexical_inventory_reports_ts_43_0 => ("ts", 43, 0),
    lexical_inventory_reports_ts_43_1 => ("ts", 43, 1),
    lexical_inventory_reports_ts_44_0 => ("ts", 44, 0),
    lexical_inventory_reports_ts_44_1 => ("ts", 44, 1),
    lexical_inventory_reports_ts_45_0 => ("ts", 45, 0),
    lexical_inventory_reports_ts_45_1 => ("ts", 45, 1),
    lexical_inventory_reports_ts_46_0 => ("ts", 46, 0),
    lexical_inventory_reports_ts_46_1 => ("ts", 46, 1),
    lexical_inventory_reports_ts_47_0 => ("ts", 47, 0),
    lexical_inventory_reports_ts_47_1 => ("ts", 47, 1),
    lexical_inventory_reports_ts_48_0 => ("ts", 48, 0),
    lexical_inventory_reports_ts_48_1 => ("ts", 48, 1),
    lexical_inventory_reports_ts_49_0 => ("ts", 49, 0),
    lexical_inventory_reports_ts_49_1 => ("ts", 49, 1),
    lexical_inventory_reports_js_0_5 => ("js", 0, 5),
    lexical_inventory_reports_js_1_5 => ("js", 1, 5),
    lexical_inventory_reports_js_2_5 => ("js", 2, 5),
    lexical_inventory_reports_js_3_5 => ("js", 3, 5),
    lexical_inventory_reports_js_4_5 => ("js", 4, 5),
    lexical_inventory_reports_js_5_5 => ("js", 5, 5),
    lexical_inventory_reports_js_6_5 => ("js", 6, 5),
    lexical_inventory_reports_js_7_5 => ("js", 7, 5),
    lexical_inventory_reports_js_8_5 => ("js", 8, 5),
    lexical_inventory_reports_js_9_5 => ("js", 9, 5),
    lexical_inventory_reports_js_10_5 => ("js", 10, 5),
    lexical_inventory_reports_js_11_5 => ("js", 11, 5),
    lexical_inventory_reports_js_12_5 => ("js", 12, 5),
    lexical_inventory_reports_js_13_5 => ("js", 13, 5),
    lexical_inventory_reports_js_14_5 => ("js", 14, 5),
    lexical_inventory_reports_js_15_5 => ("js", 15, 5),
    lexical_inventory_reports_js_16_5 => ("js", 16, 5),
    lexical_inventory_reports_js_17_5 => ("js", 17, 5),
    lexical_inventory_reports_js_18_5 => ("js", 18, 5),
    lexical_inventory_reports_js_19_5 => ("js", 19, 5),
    lexical_inventory_reports_js_20_5 => ("js", 20, 5),
    lexical_inventory_reports_js_21_5 => ("js", 21, 5),
    lexical_inventory_reports_js_22_5 => ("js", 22, 5),
    lexical_inventory_reports_js_23_5 => ("js", 23, 5),
    lexical_inventory_reports_js_24_5 => ("js", 24, 5),
    lexical_inventory_reports_js_25_5 => ("js", 25, 5),
    lexical_inventory_reports_js_26_5 => ("js", 26, 5),
    lexical_inventory_reports_js_27_5 => ("js", 27, 5),
    lexical_inventory_reports_js_28_5 => ("js", 28, 5),
    lexical_inventory_reports_js_29_5 => ("js", 29, 5),
    lexical_inventory_reports_js_30_5 => ("js", 30, 5),
    lexical_inventory_reports_js_31_5 => ("js", 31, 5),
    lexical_inventory_reports_js_32_5 => ("js", 32, 5),
    lexical_inventory_reports_js_33_5 => ("js", 33, 5),
    lexical_inventory_reports_js_34_5 => ("js", 34, 5),
    lexical_inventory_reports_js_35_5 => ("js", 35, 5),
    lexical_inventory_reports_js_36_5 => ("js", 36, 5),
    lexical_inventory_reports_js_37_5 => ("js", 37, 5),
    lexical_inventory_reports_js_38_5 => ("js", 38, 5),
    lexical_inventory_reports_js_39_5 => ("js", 39, 5),
    lexical_inventory_reports_js_40_5 => ("js", 40, 5),
    lexical_inventory_reports_js_41_5 => ("js", 41, 5),
    lexical_inventory_reports_js_42_5 => ("js", 42, 5),
    lexical_inventory_reports_js_43_5 => ("js", 43, 5),
    lexical_inventory_reports_js_44_5 => ("js", 44, 5),
    lexical_inventory_reports_js_45_5 => ("js", 45, 5),
    lexical_inventory_reports_js_46_5 => ("js", 46, 5),
    lexical_inventory_reports_js_47_5 => ("js", 47, 5),
    lexical_inventory_reports_js_48_5 => ("js", 48, 5),
    lexical_inventory_reports_js_49_5 => ("js", 49, 5),
    lexical_inventory_reports_swift_0_0 => ("swift", 0, 0),
    lexical_inventory_reports_swift_0_1 => ("swift", 0, 1),
    lexical_inventory_reports_swift_1_0 => ("swift", 1, 0),
    lexical_inventory_reports_swift_1_1 => ("swift", 1, 1),
    lexical_inventory_reports_swift_2_0 => ("swift", 2, 0),
    lexical_inventory_reports_swift_2_1 => ("swift", 2, 1),
    lexical_inventory_reports_swift_3_0 => ("swift", 3, 0),
    lexical_inventory_reports_swift_3_1 => ("swift", 3, 1),
    lexical_inventory_reports_swift_4_0 => ("swift", 4, 0),
    lexical_inventory_reports_swift_4_1 => ("swift", 4, 1),
    lexical_inventory_reports_swift_5_0 => ("swift", 5, 0),
    lexical_inventory_reports_swift_5_1 => ("swift", 5, 1),
    lexical_inventory_reports_cs_0_0 => ("cs", 0, 0),
    lexical_inventory_reports_cs_0_1 => ("cs", 0, 1),
    lexical_inventory_reports_cs_1_0 => ("cs", 1, 0),
    lexical_inventory_reports_cs_1_1 => ("cs", 1, 1),
    lexical_inventory_reports_cs_2_0 => ("cs", 2, 0),
    lexical_inventory_reports_cs_2_1 => ("cs", 2, 1),
    lexical_inventory_reports_cs_3_0 => ("cs", 3, 0),
    lexical_inventory_reports_cs_3_1 => ("cs", 3, 1),
    lexical_inventory_reports_cs_4_0 => ("cs", 4, 0),
    lexical_inventory_reports_cs_4_1 => ("cs", 4, 1),
    lexical_inventory_reports_rs_0_5 => ("rs", 0, 5),
    lexical_inventory_reports_rs_1_5 => ("rs", 1, 5),
    lexical_inventory_reports_rs_2_5 => ("rs", 2, 5),
    lexical_inventory_reports_rs_3_5 => ("rs", 3, 5),
    lexical_inventory_reports_rs_4_5 => ("rs", 4, 5),
    lexical_inventory_reports_rs_5_5 => ("rs", 5, 5),
    lexical_inventory_reports_rs_6_5 => ("rs", 6, 5),
    lexical_inventory_reports_java_0_5 => ("java", 0, 5),
    lexical_inventory_reports_java_1_5 => ("java", 1, 5),
    lexical_inventory_reports_go_0_5 => ("go", 0, 5),
    lexical_inventory_reports_go_1_5 => ("go", 1, 5),
    lexical_inventory_reports_scala_0_5 => ("scala", 0, 5),
    lexical_inventory_reports_scala_1_5 => ("scala", 1, 5),
    lexical_inventory_reports_py_0_5 => ("py", 0, 5),
    lexical_inventory_reports_py_1_5 => ("py", 1, 5),
    lexical_inventory_reports_py_2_5 => ("py", 2, 5),
    lexical_inventory_reports_py_3_5 => ("py", 3, 5),
    lexical_inventory_reports_php_0_5 => ("php", 0, 5),
    lexical_inventory_reports_php_1_5 => ("php", 1, 5),
    lexical_inventory_reports_php_2_5 => ("php", 2, 5),
    lexical_inventory_reports_php_3_5 => ("php", 3, 5),
    lexical_inventory_reports_c_0_5 => ("c", 0, 5),
    lexical_inventory_reports_cpp_0_0 => ("cpp", 0, 0),
    lexical_inventory_reports_cpp_0_1 => ("cpp", 0, 1),
    lexical_inventory_reports_cpp_0_2 => ("cpp", 0, 2),
    lexical_inventory_reports_cpp_0_3 => ("cpp", 0, 3),
    lexical_inventory_reports_cpp_0_4 => ("cpp", 0, 4),
    lexical_inventory_reports_lua_0_5 => ("lua", 0, 5),
    lexical_inventory_reports_pl_0_5 => ("pl", 0, 5),
    lexical_inventory_reports_sh_0_5 => ("sh", 0, 5),
    lexical_inventory_reports_zsh_0_5 => ("zsh", 0, 5),
}

#[test]
fn lexical_inventory_report_registration_covers_every_source_and_consumer() {
    let mut seen = std::collections::BTreeSet::new();
    for &(extension, source, report) in REGISTERED {
        assert!(source < corpus::cases(extension).len());
        let width = if extension == "cpp" { 2 } else { 5 };
        for consumer in if report == 5 { 0..10 } else { report * width..report * width + width } {
            assert!(seen.insert((extension, source, consumer)), "duplicate consumer");
        }
    }
    for extension in ["rs", "ts", "js", "go", "py", "java", "cs", "c", "cpp", "swift", "scala", "php", "pl", "sh", "zsh", "lua"] {
        for source in 0..corpus::cases(extension).len() {
            for consumer in 0..10 { assert!(seen.contains(&(extension, source, consumer)), "{extension} {source} {consumer}"); }
        }
    }
}
