//! The crate-private scenarios; see the parent module.

mod prewarm {
    //! C6: pre-warm classification rebuilds evaluation state per phrase.

    use darkmatter::markdown::compose::ComposeContext;
    use darkmatter::markdown::compose::expression::Expr;
    use serde_json::Value;

    use super::super::{frontmatter_of, measure, report};
    use crate::composition::lifecycle::LifecycleSignal;
    use crate::composition::lifecycle::bindings::LifecycleScope;
    use crate::composition::lifecycle::prewarm::{Classification, PrewarmInputs, classify};

    /// A submitted batch is classified phrase by phrase, as
    /// `PrewarmHandle::queue` does before de-duplication.
    #[test]
    #[ignore = "measurement spike; run explicitly"]
    fn pipeline_spike_c6_prewarm_batch() {
        let context = ComposeContext::capture_for_content(&std::path::PathBuf::from("."), "");
        for frontmatter_bytes in [10 * 1024, 200 * 1024] {
            let mut frontmatter = frontmatter_of(frontmatter_bytes);
            frontmatter.insert("title".into(), Value::String("Release notes".into()));
            let inputs = PrewarmInputs::captured(&Value::Object(frontmatter), context.clone());
            for phrases in [2usize, 20, 200] {
                for (shape, template) in [
                    ("literal", "Step {index} finished"),
                    ("fm-read", "Step {index} of {{ title }} finished"),
                ] {
                    let texts: Vec<Expr> = (0..phrases)
                        .map(|index| {
                            Expr::StringLiteral(template.replace("{index}", &index.to_string()))
                        })
                        .collect();
                    let measured = measure(5, || {
                        texts
                            .iter()
                            .filter(|text| {
                                matches!(
                                    classify(
                                        LifecycleScope::Event(LifecycleSignal::Success),
                                        text,
                                        &inputs
                                    ),
                                    Classification::Early(_)
                                )
                            })
                            .count()
                    });
                    report(
                        "C6",
                        &format!(
                            "{shape}/fm={}KB/phrases={phrases}",
                            frontmatter_bytes / 1024
                        ),
                        &measured,
                    );
                }
            }
        }
    }
}

mod prediction {
    //! C7: provider prediction copies the prompt frontmatter for a literal hint.

    use std::collections::HashMap;
    use std::path::Path;

    use serde_json::{Map, Value};

    use super::super::{frontmatter_of, measure, report};
    use crate::composition::sequence::preflight::providers::{HintLayers, HintState, predict};

    #[test]
    #[ignore = "measurement spike; run explicitly"]
    fn pipeline_spike_c7_literal_hint_prediction() {
        for (shape, hint) in [("literal-agent", Some("codex")), ("hintless", None)] {
            let mut document = frontmatter_of(100 * 1024);
            if let Some(hint) = hint {
                document.insert("agent".into(), Value::String(hint.into()));
            }
            let empty = Map::new();
            let step = darkmatter::markdown::compose::SharedFrontmatter::default();
            let failed = HashMap::new();
            let layers = HintLayers {
                document: &document,
                document_path: Path::new("/spike/prompt.md"),
                with: &empty,
                with_evaluated: &empty,
                with_deferred: &[],
                with_failed: &failed,
                with_origin: Path::new("/spike/seq.md"),
                task_property: "sequence[0]",
                step_frontmatter: &step,
                caller_setters: &empty,
                sequence_path: Path::new("/spike/seq.md"),
            };
            let measured = measure(9, || {
                let mut evaluate =
                    |_: &Value,
                     _: &serde_json::Map<String, Value>|
                     -> Result<_, crate::composition::CompositionError> {
                        unreachable!("a literal hint is never evaluated")
                    };
                let prediction = predict(&layers, &mut evaluate).expect("prediction");
                matches!(prediction.agent, HintState::Static { .. })
            });
            report("C7", &format!("{shape}/fm=100KB/per-task"), &measured);
            // The caller copies the document's frontmatter before predicting.
            let caller_copy = measure(9, || {
                document
                    .iter()
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect::<Map<_, _>>()
            });
            report("C7", &format!("{shape}/fm=100KB/caller-copy"), &caller_copy);
        }
    }
}
