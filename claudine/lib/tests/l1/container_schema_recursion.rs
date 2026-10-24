//! Recursive container schemas (spec criterion 10): `task` and `group` in
//! `claudine/schemas/sequence-types.yaml` reference themselves, so every
//! container event is typed at every container depth.
//!
//! Each container path (sequence → sequence, sequence → group,
//! group → sequence, group → group) is nested to container depth 4, with all
//! seven events on the innermost container. The controls are the shipped
//! fixtures `tests/fixtures/sequence-schema/containers-depth4-*.md`, which
//! `md schema validate` and DMLS read too; every other cell is one edit to a
//! control, judged through the same trigger discovery, effective schema, and
//! validation they use. The task parser (`resolve_sequence_plan`) judges the
//! conditional-depth cells: each container event starts its own count.

use std::path::Path;

use biscuit_file::serde_yaml_ng;
use claudine::composition::{CompositionError, resolve_composition_source_in_context, resolve_sequence_plan};
use serde_json::{Map, Value, json};

use crate::claudine_schemas::{Discovered, SchemaRepo, problems, shipped_bundle};

/// The container kind at one level of a path.
#[derive(Clone, Copy)]
enum Kind {
    Sequence,
    Group,
}

/// A container path: the kinds alternate between `outer` and `inner` from
/// the first level down.
#[derive(Clone, Copy)]
struct ContainerPath {
    name: &'static str,
    outer: Kind,
    inner: Kind,
}

const SEQ_SEQ: ContainerPath = ContainerPath { name: "seq-seq", outer: Kind::Sequence, inner: Kind::Sequence };
const SEQ_GROUP: ContainerPath = ContainerPath { name: "seq-group", outer: Kind::Sequence, inner: Kind::Group };
const GROUP_SEQ: ContainerPath = ContainerPath { name: "group-seq", outer: Kind::Group, inner: Kind::Sequence };
const GROUP_GROUP: ContainerPath = ContainerPath { name: "group-group", outer: Kind::Group, inner: Kind::Group };

const CONTAINER_DEPTH: usize = 4;

/// The seven container events, each with a valid stack.
const EVENTS: [&str; 7] = ["initialize", "start", "blocked", "success", "failure", "cancelled", "finalize"];

/// The flow directives `cancelled` excludes, each a well-formed `start`
/// action (see `claudine_schemas::forbidden_cancelled_items_are_well_formed_start_actions`).
const FORBIDDEN_IN_CANCELLED: [(&str, &str); 8] = [
    ("retry", "{ retry: 2 }"),
    ("resume", "{ resume: continue }"),
    ("proxy", "{ proxy: '@prompts/next.md' }"),
    ("prep", "{ prep: '@prompts/next.md' }"),
    ("skip", "{ skip: null }"),
    ("error", "{ error: stopped }"),
    ("break", "{ break: done }"),
    ("defer", "{ defer: 5m }"),
];

fn yaml(text: &str) -> Value {
    serde_yaml_ng::from_str(text).unwrap_or_else(|error| panic!("`{text}` parses: {error}"))
}

/// The control stack for `event` on the innermost container.
fn control_stack(event: &str) -> Value {
    match event {
        "cancelled" => json!(["stop"]),
        _ => json!([{ "info": event }]),
    }
}

/// A document nesting `path` to container depth 4 with `events` on the
/// innermost container, and the JSON pointer of the object holding those
/// events.
fn document(path: ContainerPath, events: &Map<String, Value>) -> (Value, String) {
    fn container(path: ContainerPath, level: usize, pointer: String, events: &Map<String, Value>) -> (Value, String) {
        let kind = if level % 2 == 1 { path.outer } else { path.inner };
        let innermost = level == CONTAINER_DEPTH;
        let (child_pointer, holder) = match kind {
            Kind::Sequence => (format!("{pointer}/sequence/0"), pointer.clone()),
            Kind::Group => (format!("{pointer}/group/tasks/0"), format!("{pointer}/group")),
        };
        let (child, holder) = if innermost {
            (json!({ "name": "leaf", "shell": "echo leaf" }), holder)
        } else {
            container(path, level + 1, child_pointer, events)
        };
        let mut body = Map::new();
        match kind {
            Kind::Sequence => {
                body.insert("name".into(), json!(format!("l{level}")));
                body.insert("sequence".into(), json!([child]));
            }
            Kind::Group => {
                body.insert("name".into(), json!(format!("g{level}")));
                body.insert("tasks".into(), json!([child]));
            }
        }
        if innermost {
            body.extend(events.clone());
        }
        match kind {
            Kind::Sequence => (Value::Object(body), holder),
            Kind::Group => (json!({ "name": format!("l{level}"), "group": body }), holder),
        }
    }
    let (step, holder) = container(path, 1, "/sequence/0".to_string(), events);
    (json!({ "sequence": [step] }), holder)
}

/// The control document's events: every event with a valid stack.
fn control_events() -> Map<String, Value> {
    EVENTS.iter().map(|event| ((*event).to_string(), control_stack(event))).collect()
}

/// `frontmatter` as a Markdown document.
fn markdown(frontmatter: &Value) -> String {
    let yaml = serde_yaml_ng::to_string(frontmatter).expect("the frontmatter serializes");
    format!("---\n{yaml}---\nBody.\n")
}

/// The control document of `path`.
fn control(path: ContainerPath) -> String {
    markdown(&document(path, &control_events()).0)
}

/// The control with `event`'s stack replaced by `stack`, and the pointer of
/// the object holding the events.
fn with_event(path: ContainerPath, event: &str, stack: Value) -> (String, String) {
    let mut events = control_events();
    events.insert(event.to_string(), stack);
    let (frontmatter, holder) = document(path, &events);
    (markdown(&frontmatter), holder)
}

/// The shipped bundle, discovered once per test.
struct Judge {
    repo: SchemaRepo,
    discovered: Discovered,
}

impl Judge {
    fn new() -> Self {
        let repo = SchemaRepo::new(&shipped_bundle!());
        let discovered = repo.discover().expect("the shipped bundle loads");
        Self { repo, discovered }
    }

    /// The schema verdict on `text`: its validity and every problem.
    fn schema(&self, name: &str, text: &str) -> (bool, Vec<(String, String)>) {
        let path = self.repo.write_document(&format!("{name}.md"), text);
        let report = self
            .discovered
            .judge(
                &path,
                include_str!("../../../schemas/claudine.yaml"),
                include_str!("../../../schemas/inline-compose.yaml"),
            )
            .expect("judged")
            .report;
        (report.valid, problems(&report))
    }

    /// The task parser's rejection of `text`, or `None` when it accepts it.
    fn parser(&self, name: &str, text: &str) -> Option<CompositionError> {
        let path = self.repo.write_document(&format!("{name}.md"), text);
        rejection(&self.repo, &path)
    }

    /// `text` is rejected with a problem at `pointer`.
    fn assert_rejected_at(&self, label: &str, text: &str, pointer: &str) {
        let (valid, found) = self.schema("cell", text);
        assert!(!valid, "{label}: rejected");
        assert!(found.iter().any(|(path, _)| path == pointer), "{label}: reported at `{pointer}`: {found:?}");
    }

    fn assert_valid(&self, label: &str, text: &str) {
        let (valid, found) = self.schema("cell", text);
        assert!(valid, "{label}: valid: {found:?}");
    }
}

fn rejection(repo: &SchemaRepo, path: &Path) -> Option<CompositionError> {
    let context = repo.context();
    let source = resolve_composition_source_in_context(path.to_str().expect("UTF-8 path"), &context)
        .expect("the document resolves");
    resolve_sequence_plan(&source, &context).err()
}

/// The items placed in `branch` of a conditional `level` levels deep, each
/// enclosing conditional descending through `then`, and the instance path of
/// `leaf` below the stack.
fn in_conditional(level: usize, branch: &str, leaf: &str) -> (String, String) {
    let mut stack = if branch == "then" {
        format!("[{{ when: \"ctx.l{level}\", then: [{leaf}] }}]")
    } else {
        format!("[{{ when: \"ctx.l{level}\", then: [{{ info: hi }}], else: [{leaf}] }}]")
    };
    for outer in (1..level).rev() {
        stack = format!("[{{ when: \"ctx.l{outer}\", then: {stack} }}]");
    }
    (stack, format!("{}/0/{branch}/0", "/0/then".repeat(level - 1)))
}

/// A stack whose conditionals nest `depth` levels, through `then`.
fn nested(depth: usize) -> Value {
    yaml(&in_conditional(depth, "then", "{ info: leaf }").0)
}

/// The control is valid and the parser accepts it; an unknown action in each
/// of the seven events is rejected at that event's first item.
fn assert_every_event_is_typed(path: ContainerPath, control_text: &str) {
    let judge = Judge::new();
    assert_eq!(control(path), control_text, "{}: the shipped control is the generated one", path.name);
    judge.assert_valid(&format!("{} control", path.name), control_text);
    if let Some(error) = judge.parser("control", control_text) {
        panic!("{}: the task parser accepts the control: {error}", path.name);
    }
    for event in EVENTS {
        let (text, holder) = with_event(path, event, yaml("[{ inf0: hi }]"));
        judge.assert_rejected_at(&format!("{} {event}: unknown action", path.name), &text, &format!("{holder}/{event}/0"));
    }
}

/// Each forbidden flow directive is rejected in `cancelled` (as an item, and
/// in the `then` and `else` of a level-4 conditional); `[stop]` and `[]`
/// are valid.
fn assert_cancelled_is_restricted(path: ContainerPath) {
    let judge = Judge::new();
    for (verb, item) in FORBIDDEN_IN_CANCELLED {
        let (text, holder) = with_event(path, "cancelled", yaml(&format!("[{item}]")));
        judge.assert_rejected_at(&format!("{} cancelled `{verb}`", path.name), &text, &format!("{holder}/cancelled/0"));
        for branch in ["then", "else"] {
            let (stack, below) = in_conditional(4, branch, item);
            let (text, holder) = with_event(path, "cancelled", yaml(&stack));
            judge.assert_rejected_at(
                &format!("{} cancelled `{verb}` in a level-4 `{branch}`", path.name),
                &text,
                &format!("{holder}/cancelled{below}"),
            );
        }
    }
    for branch in ["then", "else"] {
        let (stack, _) = in_conditional(4, branch, "stop");
        judge.assert_valid(&format!("{} cancelled `stop` in a level-4 `{branch}`", path.name), &with_event(path, "cancelled", yaml(&stack)).0);
    }
    judge.assert_valid(&format!("{} cancelled: []", path.name), &with_event(path, "cancelled", json!([])).0);
}

/// An initialization shell is rejected in each branch; a malformed generated
/// argument in an ordinary event is rejected at its operand; each container
/// event starts its own conditional-depth count, which only the task parser
/// limits.
fn assert_event_profiles_and_depth_counts(path: ContainerPath) {
    let judge = Judge::new();
    for branch in ["then", "else"] {
        let (stack, below) = in_conditional(1, branch, "{ shell: ls }");
        let (text, holder) = with_event(path, "initialize", yaml(&stack));
        judge.assert_rejected_at(&format!("{} initialize shell in `{branch}`", path.name), &text, &format!("{holder}/initialize{below}"));
        let error = judge.parser("init-shell", &text).unwrap_or_else(|| panic!("{}: the task parser refuses an initialize shell", path.name));
        assert!(format!("{error}").contains("shell"), "{}: {error}", path.name);
    }

    let (text, holder) = with_event(path, "start", yaml("[{ pr_list: three }]"));
    judge.assert_rejected_at(&format!("{} malformed generated argument", path.name), &text, &format!("{holder}/start/0/pr_list"));

    // Five levels in every event of the innermost container: each event
    // counts its own conditionals, so the parser accepts all of them.
    let mut events = control_events();
    for event in ["start", "blocked", "success", "failure", "finalize"] {
        events.insert(event.to_string(), nested(5));
    }
    let deep = markdown(&document(path, &events).0);
    judge.assert_valid(&format!("{} five levels in each event", path.name), &deep);
    if let Some(error) = judge.parser("five-each", &deep) {
        panic!("{}: each event starts its own count: {error}", path.name);
    }

    // A sixth level is the parser's alone to refuse.
    let (six, _) = with_event(path, "success", nested(6));
    judge.assert_valid(&format!("{} six levels", path.name), &six);
    let error = judge.parser("six", &six).unwrap_or_else(|| panic!("{}: the task parser refuses a sixth level", path.name));
    assert!(format!("{error}").contains("conditionals nest at most 5 levels"), "{}: {error}", path.name);
}

macro_rules! container_path_tests {
    ($($path:ident: $events:ident, $cancelled:ident, $profiles:ident => $fixture:literal;)*) => {
        $(
            #[test]
            fn $events() {
                assert_every_event_is_typed($path, include_str!($fixture));
            }

            #[test]
            fn $cancelled() {
                assert_cancelled_is_restricted($path);
            }

            #[test]
            fn $profiles() {
                assert_event_profiles_and_depth_counts($path);
            }
        )*

        /// Every path the macro generates tests for.
        const PATHS: &[ContainerPath] = &[$($path),*];
    };
}

container_path_tests! {
    SEQ_SEQ: seq_seq_types_every_event, seq_seq_restricts_cancelled, seq_seq_event_profiles_and_depth_counts
        => "../fixtures/sequence-schema/containers-depth4-seq-seq.md";
    SEQ_GROUP: seq_group_types_every_event, seq_group_restricts_cancelled, seq_group_event_profiles_and_depth_counts
        => "../fixtures/sequence-schema/containers-depth4-seq-group.md";
    GROUP_SEQ: group_seq_types_every_event, group_seq_restricts_cancelled, group_seq_event_profiles_and_depth_counts
        => "../fixtures/sequence-schema/containers-depth4-group-seq.md";
    GROUP_GROUP: group_group_types_every_event, group_group_restricts_cancelled, group_group_event_profiles_and_depth_counts
        => "../fixtures/sequence-schema/containers-depth4-group-group.md";
}

#[test]
fn the_tests_cover_the_four_container_paths() {
    let names: Vec<&str> = PATHS.iter().map(|path| path.name).collect();
    assert_eq!(names, ["seq-seq", "seq-group", "group-seq", "group-group"]);
}

/// Scalar steps and the file-reference alternatives of `sequence:` and
/// `group:` remain valid at container depth 4.
#[test]
fn scalar_steps_and_file_references_stay_valid_at_depth_four() {
    let judge = Judge::new();
    for path in PATHS {
        let mut events = Map::new();
        events.insert("success".into(), json!([{ "info": "done" }]));
        let (mut frontmatter, holder) = document(*path, &events);
        let holder = holder.trim_end_matches("/group").to_string();
        let container = frontmatter.pointer_mut(&holder).expect("the innermost container");
        match container.get_mut("group") {
            Some(group) => {
                group["tasks"] = json!([{ "name": "ref", "sequence": "@sequences/next.md" }, { "name": "grp", "group": "@groups/legs.yaml" }]);
            }
            None => container["sequence"] = json!(["plain", 3, true, { "name": "ref", "group": "@groups/legs.yaml" }]),
        }
        judge.assert_valid(&format!("{} scalar steps and references", path.name), &markdown(&frontmatter));
    }
}
