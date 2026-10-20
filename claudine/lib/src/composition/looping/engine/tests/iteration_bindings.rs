//! A loop iteration's preparation declares the `loop` global to Darkmatter:
//! the body, compose-time frontmatter values, and lifecycle shell bytes read
//! the iteration's values, `fm.loop` stays the `loop:` configuration, and a
//! retired `_loop_*` read a later render introduces is removed grammar.

use super::*;
use crate::composition::lifecycle::actions::LifecycleActionKind;
use crate::composition::prepare::prepare_direct;
use darkmatter::markdown::compose::expression::Expr;

/// A document `loop.md` (plus any `extra` files) in a directory that lives as
/// long as the returned guard.
fn source_in_dir(content: &str, extra: &[(&str, &str)]) -> (TempDir, ResolvedCompositionSource) {
    let dir = TempDir::new().unwrap();
    for (name, text) in extra {
        std::fs::write(dir.path().join(name), text).unwrap();
    }
    let file = dir.path().join("loop.md");
    std::fs::write(&file, content).unwrap();
    let source = ResolvedCompositionSource {
        original_ref: file.to_string_lossy().to_string(),
        resolved_path: file,
        original_text: content.to_string(),
        markdown: content.to_string().into(),
    };
    (dir, source)
}

fn options(values: Option<LoopAmbient>) -> PrepareOptions {
    let mut options = PrepareOptions::new(crate::test_support::context());
    options.loop_values = values;
    options
}

fn second_pass() -> LoopAmbient {
    LoopAmbient::new(2, false, true, "prior answer", 3)
}

const LOOP_DOCUMENT: &str = "---\n\
    icon: \"{{ loop.is_last ? 'last' : 'more' }}\"\n\
    label: \"pass {{ loop['count'] }}\"\n\
    whole: \"{{ loop }}\"\n\
    loop:\n  max: 5\n  until: \"loop.count >= 2\"\n\
    ---\n\
    count={{ loop.count }} first={{ loop['is_first'] }} last={{ loop.is_last }} \
    out=[{{ loop.last_output }}] code={{ loop.last_exit_code }} \
    icon={{ icon }} label={{ label }} max={{ fm.loop.max }} missing=[{{ loop.max }}]\n";

#[test]
fn an_iterations_body_and_compose_time_frontmatter_read_its_loop_values() {
    let (_dir, source) = source_in_dir(LOOP_DOCUMENT, &[]);

    let prepared = prepare_direct(&source, options(Some(second_pass()))).unwrap();

    assert_eq!(
        prepared.prompt.trim(),
        "count=2 first=false last=true out=[prior answer] code=3 icon=last label=pass 2 \
         max=5 missing=[]"
    );
    let frontmatter = prepared.effective_frontmatter.as_object().unwrap();
    assert_eq!(frontmatter.get("icon"), Some(&json!("last")));
    assert_eq!(frontmatter.get("label"), Some(&json!("pass 2")));
    assert_eq!(frontmatter.get("whole"), Some(&second_pass().to_value()));
    // `fm.loop` and the composed document keep the configuration block.
    assert_eq!(frontmatter.get("loop").and_then(|block| block.get("max")), Some(&json!(5)));
}

/// Before its first iteration (the loop seed, the bootstrap read) a loop
/// document cannot know the values: `loop` reads `null`, never the
/// configuration.
#[test]
fn a_loop_document_prepared_before_any_iteration_reads_null() {
    let (_dir, source) = source_in_dir(LOOP_DOCUMENT, &[]);

    let prepared = prepare_direct(&source, options(None)).unwrap();

    assert_eq!(
        prepared.prompt.trim(),
        "count= first= last= out=[] code= icon=more label=pass  max=5 missing=[]"
    );
}

/// Outside any loop `loop` is unavailable with a typed scope diagnostic.
#[test]
fn a_document_outside_any_loop_cannot_read_loop() {
    let (_dir, source) = source_in_dir("---\ntitle: plain\n---\npass {{ loop.count }}\n", &[]);

    let error = prepare_direct(&source, options(None)).unwrap_err();

    let CompositionError::ComposeFailed(markdown) = &error else {
        panic!("expected a composition failure, got {error:?}");
    };
    let read = markdown.unavailable_binding().expect("an unavailable-binding read");
    assert_eq!(read.root, "loop");
    assert_eq!(read.reason.code(), crate::composition::lifecycle::bindings::OUTSIDE_LOOP);
}

/// A lifecycle shell's bytes are fixed per iteration, so they read the
/// iteration's values.
#[test]
fn an_iterations_lifecycle_shell_bytes_read_its_loop_values() {
    let (_dir, source) = source_in_dir(
        "---\nloop:\n  until: \"loop.count >= 2\"\nstart:\n  - shell: \"echo pass {{ loop.count }} of {{ fm.loop.until }}\"\n---\nbody\n",
        &[],
    );

    let prepared = prepare_direct(&source, options(Some(second_pass()))).unwrap();

    let mut commands = Vec::new();
    crate::composition::lifecycle::actions::for_each_action(
        prepared.lifecycle.stack(crate::composition::LifecycleSignal::Start),
        &mut |action| {
            if let LifecycleActionKind::Shell(shell) = &action.kind
                && let Expr::StringLiteral(command) = &shell.command
            {
                commands.push(command.clone());
            }
        },
    );
    assert_eq!(commands, ["echo pass 2 of loop.count >= 2"]);
}

/// A retired name a transcluded document reads is found when that document
/// composes, as removed grammar naming the replacement.
#[test]
fn a_retired_read_a_transcluded_document_introduces_is_removed_grammar() {
    let (_dir, source) = source_in_dir(
        "---\nloop:\n  max: 2\n---\n::file ./part.md\n",
        &[("part.md", "---\ntitle: part\n---\nlast {{ _loop_last_output }}\n")],
    );

    let error = prepare_direct(&source, options(Some(second_pass()))).unwrap_err();

    match error {
        CompositionError::RemovedGrammar {
            removed,
            replacement,
            path,
            ..
        } => {
            assert_eq!(removed, "_loop_last_output");
            assert_eq!(replacement, "use `loop.last_output`");
            assert_eq!(path, "render");
        }
        other => panic!("expected RemovedGrammar, got {other:?}"),
    }
}
