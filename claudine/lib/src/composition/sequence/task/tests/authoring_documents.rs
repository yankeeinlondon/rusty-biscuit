//! A stack runs in the document that authored it: its `source_path` and its
//! file-resolution context are handed off together. The sequence lives in
//! `flows/`, the file it references in another directory (`groups/`,
//! `tasks/`), so a relative write that lands beside the wrong document is
//! visible. Each case runs twice: with an invocation, which derives the
//! authoring document's own context, and without one, where the caller's
//! context is judged for the authoring document.

use super::*;
use crate::invocation_context::InvocationContext;
use darkmatter::markdown::compose::RequestSnapshot;

/// A canonical temp root holding `flows/` and the authoring directory, with
/// the sequence at `flows/seq.md` whose one step is `step`.
struct Tree {
    root: PathBuf,
    fixture: Fixture,
    invocation: InvocationContext,
}

impl Tree {
    /// `write` fills the tree (beside `flows/`) before the sequence is built.
    fn build(step: Value, write: impl FnOnce(&Path)) -> Self {
        let dir = TempDir::new().unwrap();
        let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
        fs::create_dir_all(root.join("flows")).unwrap();
        write(&root);
        let source = one_step_source(&root.join("flows"), step);
        let mut fixture = Fixture::build(dir, &source).unwrap();
        // Relative targets are created beside their documents, so the
        // boundary must contain the whole tree.
        fixture.engine = EffectEngine::builder().mutation_root(&root).auto_rehash(false).build();
        let invocation = InvocationContext::capture(
            &RequestSnapshot::new(&root).with_home(Some(root.join("home"))),
        )
        .unwrap();
        Self { root, fixture, invocation }
    }

    /// Run the step, with or without the invocation, and return its outcome.
    fn run(&self, with_invocation: bool) -> TaskOutcome {
        let recorder = Recorder::default();
        let shell = FakeTaskShell::default();
        let runtime = Arc::new(RuntimeState::new());
        let mut wiring = Wiring::new(&recorder, &shell);
        wiring.runtime = Some(&runtime);
        if with_invocation {
            wiring.document = Some(&self.invocation);
        }
        self.fixture.execute(&wiring)
    }

    fn lines(&self, relative: &str) -> Vec<String> {
        fs::read_to_string(self.root.join(relative))
            .unwrap_or_else(|error| panic!("expected {relative}: {error}"))
            .lines()
            .map(str::to_string)
            .collect()
    }

    /// Remove every file the previous run wrote, keeping the documents.
    fn clear(&self, written: &[&str]) {
        for relative in written {
            let _ = fs::remove_file(self.root.join(relative));
        }
    }
}

fn append(target: &str, line: &str) -> Value {
    json!({ "append_line": [target, line] })
}

/// A group file whose four events and one-pass loop gate append to
/// `events.log`, and whose one leg appends to `member.log`.
fn group_document() -> Value {
    json!({
        "kind": "group",
        "name": "bundle",
        "initialize": [append("events.log", "initialize")],
        "start": [append("events.log", "start")],
        "success": [append("events.log", "success")],
        "finalize": [append("events.log", "finalize")],
        "loop": { "until": "loop.count >= 1", "gate": [append("events.log", "gate")] },
        "tasks": [{ "name": "leg", "side_effect": append("member.log", "member") }],
    })
}

#[test]
fn a_referenced_groups_events_gate_and_member_write_beside_the_group() {
    let tree = Tree::build(json!({ "name": "alpha", "group": "../groups/group.yaml" }), |root| {
        fs::create_dir_all(root.join("groups")).unwrap();
        write_yaml(&root.join("groups"), "group.yaml", &group_document());
    });
    for with_invocation in [true, false] {
        let outcome = tree.run(with_invocation);

        assert!(outcome.succeeded(), "invocation={with_invocation}: {}", failure_message(&outcome));
        assert_eq!(
            tree.lines("groups/events.log"),
            ["initialize", "start", "success", "finalize", "gate"],
            "invocation={with_invocation}",
        );
        assert_eq!(tree.lines("groups/member.log"), ["member"], "invocation={with_invocation}");
        assert!(!tree.root.join("flows/events.log").exists(), "invocation={with_invocation}: nothing beside the caller");
        assert!(!tree.root.join("flows/member.log").exists(), "invocation={with_invocation}");
        tree.clear(&["groups/events.log", "groups/member.log"]);
    }
}

#[test]
fn a_catalog_selected_groups_events_write_beside_the_catalog() {
    let tree = Tree::build(json!({ "name": "alpha", "group": "selected@../groups/catalog.yaml" }), |root| {
        fs::create_dir_all(root.join("groups")).unwrap();
        write_yaml(&root.join("groups"), "catalog.yaml", &json!({
            "kind": "group-catalog",
            "groups": [
                { "name": "other", "tasks": [{ "shell": "unused" }] },
                {
                    "name": "selected",
                    "start": [append("catalog.log", "start")],
                    "finalize": [append("catalog.log", "finalize")],
                    "tasks": [{ "name": "leg", "side_effect": append("catalog.log", "member") }],
                },
            ],
        }));
    });
    for with_invocation in [true, false] {
        let outcome = tree.run(with_invocation);

        assert!(outcome.succeeded(), "invocation={with_invocation}: {}", failure_message(&outcome));
        assert_eq!(tree.lines("groups/catalog.log"), ["start", "member", "finalize"], "invocation={with_invocation}");
        assert!(!tree.root.join("flows/catalog.log").exists(), "invocation={with_invocation}");
        tree.clear(&["groups/catalog.log"]);
    }
}

/// Control: an inline group's events and legs belong to the sequence that
/// contains them.
#[test]
fn an_inline_groups_events_and_member_write_beside_the_containing_sequence() {
    let mut group = group_document();
    group.as_object_mut().unwrap().remove("kind");
    let tree = Tree::build(json!({ "name": "alpha", "group": group }), |_| {});
    for with_invocation in [true, false] {
        let outcome = tree.run(with_invocation);

        assert!(outcome.succeeded(), "invocation={with_invocation}: {}", failure_message(&outcome));
        assert_eq!(
            tree.lines("flows/events.log"),
            ["initialize", "start", "success", "finalize", "gate"],
            "invocation={with_invocation}",
        );
        assert_eq!(tree.lines("flows/member.log"), ["member"], "invocation={with_invocation}");
        tree.clear(&["flows/events.log", "flows/member.log"]);
    }
}

/// Control: a referenced sequence's events, gate, and task write beside it.
#[test]
fn a_referenced_sequences_events_gate_and_task_write_beside_the_sequence() {
    let tree = Tree::build(json!({ "name": "alpha", "sequence": "../parts/nested.yaml" }), |root| {
        fs::create_dir_all(root.join("parts")).unwrap();
        write_yaml(&root.join("parts"), "nested.yaml", &json!({
            "initialize": [append("events.log", "initialize")],
            "start": [append("events.log", "start")],
            "success": [append("events.log", "success")],
            "finalize": [append("events.log", "finalize")],
            "loop": { "until": "loop.count >= 1", "gate": [append("events.log", "gate")] },
            "sequence": [{ "name": "task", "side_effect": append("member.log", "task") }],
        }));
    });
    for with_invocation in [true, false] {
        let outcome = tree.run(with_invocation);

        assert!(outcome.succeeded(), "invocation={with_invocation}: {}", failure_message(&outcome));
        assert_eq!(
            tree.lines("parts/events.log"),
            ["initialize", "start", "success", "finalize", "gate"],
            "invocation={with_invocation}",
        );
        assert_eq!(tree.lines("parts/member.log"), ["task"], "invocation={with_invocation}");
        assert!(!tree.root.join("flows/events.log").exists(), "invocation={with_invocation}");
        tree.clear(&["parts/events.log", "parts/member.log"]);
    }
}

/// The internal `kind: task` adapter: setup, the primary side effect, and
/// teardown were authored in the task file, so each writes beside it.
#[test]
fn a_task_files_setup_primary_and_teardown_write_beside_the_task_file() {
    let tree = Tree::build(json!({ "name": "alpha", "task": "../tasks/task.yaml" }), |root| {
        fs::create_dir_all(root.join("tasks")).unwrap();
        write_yaml(&root.join("tasks"), "task.yaml", &json!({
            "kind": "task",
            "setup": [{ "ensure_file": ["setup.md"] }],
            "side_effect": { "ensure_file": ["note.md"] },
            "teardown": [{ "ensure_file": ["teardown.md"] }],
        }));
    });
    for with_invocation in [true, false] {
        let outcome = tree.run(with_invocation);

        assert!(outcome.succeeded(), "invocation={with_invocation}: {}", failure_message(&outcome));
        for name in ["setup.md", "note.md", "teardown.md"] {
            assert!(tree.root.join("tasks").join(name).is_file(), "invocation={with_invocation}: {name} beside the task");
            assert!(!tree.root.join("flows").join(name).exists(), "invocation={with_invocation}: {name} not beside the sequence");
        }
        tree.clear(&["tasks/setup.md", "tasks/note.md", "tasks/teardown.md"]);
    }
}

/// A referenced group's `variables` were authored in the group file, so their
/// file reads look beside it.
#[test]
fn a_referenced_groups_variables_read_files_beside_the_group() {
    let tree = Tree::build(json!({ "name": "alpha", "group": "../groups/group.yaml" }), |root| {
        fs::create_dir_all(root.join("groups")).unwrap();
        fs::write(root.join("groups/present.txt"), "here").unwrap();
        write_yaml(&root.join("groups"), "group.yaml", &json!({
            "kind": "group",
            "name": "bundle",
            "variables": { "found": "{{ file_exists(\"present.txt\") }}" },
            "tasks": [{ "name": "leg", "side_effect": append("member.log", "found={{ group.found }}") }],
        }));
    });
    for with_invocation in [true, false] {
        let outcome = tree.run(with_invocation);

        assert!(outcome.succeeded(), "invocation={with_invocation}: {}", failure_message(&outcome));
        assert_eq!(tree.lines("groups/member.log"), ["found=true"], "invocation={with_invocation}");
        tree.clear(&["groups/member.log"]);
    }
}
