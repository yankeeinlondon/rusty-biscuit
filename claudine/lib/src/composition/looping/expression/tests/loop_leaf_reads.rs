//! `loop.<member>` reads the member off the ambient; only bare `loop` builds
//! the whole object.

use super::*;

/// One megabyte of text, the size at which copying the previous result
/// dominated a scalar read.
fn large_text() -> String {
    "x".repeat(1024 * 1024)
}

fn lookup_over<'a>(
    fm: &'a Map<String, Value>,
    ambient: &'a LoopAmbient,
) -> LoopExpressionLookup<'a> {
    LoopExpressionLookup::new(
        fm,
        ambient,
        crate::test_support::process_context(),
        std::path::Path::new("prompt.md"),
    )
}

/// Members, nested and indexed paths into the previous result, malformed
/// segments, and names the object lacks, including keys of the `loop:` block.
const MEMBERS: &[&str] = &[
    "count",
    "is_first",
    "is_last",
    "last_output",
    "last_exit_code",
    "last_output.status",
    "last_output.0",
    "count.value",
    "Count",
    "iteration",
    "max",
    "while",
    "missing",
    "",
    ".",
];

#[test]
fn loop_leaf_reads_equal_the_whole_object_member() {
    // Same-named frontmatter keys, top level and in the `loop:` block, that a
    // miss must not reach.
    let fm = map(json!({
        "missing": "frontmatter",
        "loop": {"max": 4, "while": "true", "missing": "configured"},
    }));
    for last_output in [
        json!(large_text()),
        json!({"status": "ok", "nothing": null, "items": [1, 2], "body": large_text()}),
        json!(["a", "b"]),
        Value::Null,
        json!(""),
    ] {
        let ambient = LoopAmbient::new(2, false, true, last_output, 3);
        let lookup = lookup_over(&fm, &ambient);
        let whole = lookup.get("loop").expect("bare loop resolves");
        for member in MEMBERS {
            assert_eq!(
                lookup.get(&format!("loop.{member}")),
                whole.get(*member).cloned(),
                "`loop.{member}` differs from the whole object's member"
            );
        }
    }
}

#[test]
fn loop_leaf_reads_see_each_pass_current_values() {
    let fm = map(json!({}));
    let condition = LoopCondition::Until("loop.count >= 2 && loop.last_output == 'second'".into());
    let first = LoopAmbient::new(1, true, false, "first", 0);
    let second = LoopAmbient::new(2, false, true, "second", 1);

    assert!(evaluate_condition(&condition, &lookup_over(&fm, &first)).unwrap());
    let lookup = lookup_over(&fm, &second);
    assert!(!evaluate_condition(&condition, &lookup).unwrap());
    assert_eq!(lookup.get("loop.count"), Some(json!(2)));
    assert_eq!(lookup.get("loop.is_last"), Some(json!(true)));
    assert_eq!(lookup.get("loop.last_exit_code"), Some(json!(1)));
}

#[cfg(feature = "work-counters")]
mod work_counts {
    use super::*;
    use crate::work_counters::{Work, snapshot};

    fn materializations(read: impl FnOnce()) -> u64 {
        let before = snapshot();
        read();
        snapshot().since(&before).get(Work::LoopAmbientMaterialize)
    }

    #[test]
    fn loop_leaf_reads_never_build_the_object_beside_a_large_result() {
        let fm = map(json!({}));
        let ambient = LoopAmbient::new(3, false, true, large_text(), 0);
        let lookup = lookup_over(&fm, &ambient);

        for path in [
            "loop.count",
            "loop.is_first",
            "loop.is_last",
            "loop.last_exit_code",
            "loop.missing",
        ] {
            assert_eq!(materializations(|| drop(lookup.get(path))), 0, "`{path}`");
        }
        let condition = LoopCondition::While("loop.count < 5 && !loop.is_last".into());
        let mut decision = None;
        assert_eq!(
            materializations(|| decision = Some(evaluate_condition(&condition, &lookup).unwrap())),
            0
        );
        assert_eq!(decision, Some(false));

        // The requested large value is copied, but the object still is not built.
        let mut output = None;
        assert_eq!(
            materializations(|| output = lookup.get("loop.last_output")),
            0
        );
        assert_eq!(output, Some(Value::String(large_text())));
    }

    #[test]
    fn a_whole_loop_read_builds_the_object_once() {
        let fm = map(json!({}));
        let ambient = LoopAmbient::new(3, false, true, large_text(), 0);
        let lookup = lookup_over(&fm, &ambient);

        let mut whole = None;
        assert_eq!(materializations(|| whole = lookup.get("loop")), 1);
        assert_eq!(
            whole.and_then(|value| value.get("count").cloned()),
            Some(json!(3))
        );
    }
}
