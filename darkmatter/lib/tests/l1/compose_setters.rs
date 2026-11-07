//! The library side of `ComposeOptions::with_setters`, for what only a host can
//! arrange: a data override beside a setter, and a preparation cache key.

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::setters::{RawSetter, SetterPlan, TokenClass, classify_token};
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions};
use serde_json::{Value, json};
use tempfile::TempDir;

fn setter(token: &str) -> RawSetter {
    match classify_token(token) {
        TokenClass::Setter(setter) => setter,
        other => panic!("`{token}` is not a setter: {other:?}"),
    }
}

fn plan(tokens: &[&str]) -> SetterPlan {
    SetterPlan::new(tokens.iter().map(|token| setter(token))).expect("the plan settles")
}

fn compose(dir: &TempDir, document: &str, options: impl FnOnce(ComposeOptions) -> ComposeOptions) -> String {
    let path = dir.path().join("doc.md");
    std::fs::write(&path, document).unwrap();
    let base = ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir.path(), "")).with_source_file(&path);
    let request = crate::request_support::request(options(base));
    let (composed, _) = Markdown::try_from(path.as_path()).unwrap().compose_with(&request).unwrap();
    composed.content().trim_end().to_string()
}

#[test]
fn a_setter_applies_after_set_overrides() {
    let dir = TempDir::new().unwrap();
    let out = compose(&dir, "---\nt: 1\n---\n{{ a.b }} {{ a.c }}\n", |options| {
        options.with_set_overrides(json!({"a": {"b": "set", "c": "set"}})).with_setters(plan(&["a.b=setter"]))
    });
    assert_eq!(out, "setter set");
}

#[test]
fn a_data_override_keeps_the_key_it_names() {
    let dir = TempDir::new().unwrap();
    let out = compose(&dir, "---\nt: 1\n---\n{{ a.b }} {{ c }} {{ d }}\n", |options| {
        options
            .with_data_overrides(json!({"a": {"b": "data"}, "c": "data"}))
            .with_setters(plan(&["a.b=setter", "c=setter", "d=setter"]))
    });
    assert_eq!(out, "data data setter");
}

#[test]
fn an_empty_plan_changes_nothing() {
    let dir = TempDir::new().unwrap();
    let out = compose(&dir, "---\nt: 1\n---\n{{ t }}\n", |options| options.with_setters(SetterPlan::default()));
    assert_eq!(out, "1");
}

/// One grammar: `PropertyPath` owns splitting a path, so a consumer that splits on
/// `.` would misread `a\.b` while ordinary keys kept working.
#[test]
fn only_property_path_splits_a_path_in_the_library() {
    for (file, source) in [
        ("compose/setters.rs", include_str!("../../src/markdown/compose/setters.rs")),
        ("hash/path_edit.rs", include_str!("../../src/markdown/hash/path_edit.rs")),
        ("schemas/completion.rs", include_str!("../../src/markdown/schemas/completion.rs")),
    ] {
        for needle in ["split('.')", "split(\".\")", "split_once('.')", "rsplit('.')", "rsplit_once('.')"] {
            assert!(!source.contains(needle), "{file} parses a path with `{needle}`");
        }
    }
}

/// A caller's file value below a top-level property, through a root union that
/// must select its arm with the value's origin in hand. The launch directory is
/// the request directory; the document lives in `sub/`, which holds no `assets/`
/// unless a test puts one there.
mod caller_file_origins {
    use super::*;
    use std::path::PathBuf;

    const UNION: &str = "\
kind: schema
types:
  node:
    avatar: file(eager; match(./assets/*.png))
    cover: file(eager; match(./assets/*.png))
    children: node[]
$schema:
  - kind: literal(image)
    avatar: file(eager; match(./assets/*.png))
    author: node
    authors: node[]
    byid:
      \"<string>\": node
  - kind: literal(other)
    title: string(required)
";

    const PARENTS: &str = "---\n$schema: ../s.yaml\nkind: image\nauthor:\n  children:\n    - children: []\nauthors:\n  - children: []\nbyid:\n  primary:\n    children: []\n---\nbody\n";

    struct Launch {
        _dir: TempDir,
        root: PathBuf,
    }

    impl Launch {
        fn new(schema_file: &str, schema: &str, document: &str) -> Self {
            let dir = TempDir::new().unwrap();
            let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
            let launch = Self { _dir: dir, root };
            launch.write(schema_file, schema);
            launch.write("sub/doc.md", document);
            launch.write("assets/avatar.png", "launch");
            launch
        }

        fn union(document: &str) -> Self {
            Self::new("s.yaml", UNION, document)
        }

        fn write(&self, relative: &str, text: &str) {
            let path = self.root.join(relative);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }

        fn avatar(&self) -> Value {
            let path = self.root.join("assets").join("avatar.png");
            json!(path.to_str().unwrap())
        }

        fn compose(&self, options: impl FnOnce(ComposeOptions) -> ComposeOptions) -> Result<Value, String> {
            let path = self.root.join("sub").join("doc.md");
            let base = ComposeOptions::new_with_context(ComposeContext::capture_for_content(&self.root, "")).with_source_file(&path);
            let request = crate::request_support::request_at(&self.root, options(base));
            let (composed, _) = Markdown::try_from(path.as_path())
                .unwrap()
                .compose_with(&request)
                .map_err(|error| error.to_string())?;
            let frontmatter = composed.frontmatter().as_map();
            Ok(Value::Object(frontmatter.iter().map(|(key, value)| (key.clone(), value.clone())).collect()))
        }

        fn with_setters(&self, tokens: &[&str]) -> Result<Value, String> {
            self.compose(|options| options.with_setters(plan(tokens)))
        }

        fn with_set(&self, set: Value) -> Result<Value, String> {
            self.compose(|options| options.with_set_overrides(set))
        }

        /// Writes a document authoring `author.cover` as the launch directory's
        /// avatar by its absolute path.
        fn author_a_copy_of_the_avatar(&self) {
            let cover = serde_json::to_string(self.avatar().as_str().unwrap()).unwrap();
            self.write("sub/doc.md", &format!("---\n$schema: ../s.yaml\nkind: image\nauthor:\n  cover: {cover}\n  children: []\n---\nbody\n"));
        }

        /// `{{ expression }}` after `setter` renders as `{{ avatar }}` does with the
        /// same file set at the top level.
        fn assert_renders_like_the_top_level(&self, setter: &str, expression: &str) {
            let path = self.root.join("sub").join("doc.md");
            let render = |token: &str, read: &str| {
                self.write("sub/doc.md", &PARENTS.replace("body", &format!("{{{{ {read} }}}}")));
                let base = ComposeOptions::new_with_context(ComposeContext::capture_for_content(&self.root, ""))
                    .with_source_file(&path)
                    .with_setters(plan(&[token]));
                let request = crate::request_support::request_at(&self.root, base);
                let (composed, _) = Markdown::try_from(path.as_path()).unwrap().compose_with(&request).unwrap();
                composed.content().trim().to_string()
            };
            assert_eq!(render(setter, expression), render("avatar=assets/avatar.png", "avatar"), "{setter}");
        }

        /// The problems composing with `tokens` is refused for.
        fn refusal(&self, tokens: &[&str]) -> Vec<simplified_schema::ValidationProblem> {
            let path = self.root.join("sub").join("doc.md");
            let base = ComposeOptions::new_with_context(ComposeContext::capture_for_content(&self.root, ""))
                .with_source_file(&path)
                .with_setters(plan(tokens));
            let request = crate::request_support::request_at(&self.root, base);
            let failure = Markdown::try_from(path.as_path()).unwrap().compose_with(&request).unwrap_err();
            let darkmatter::markdown::MarkdownError::SchemaValidationFailed { problems, .. } = failure else {
                panic!("{tokens:?}: {failure:?}");
            };
            problems
        }

        fn assert_refusal_names_the_launch_directory(&self, token: &str, raw: &str) {
            let problems = self.refusal(&[token]);
            let problem = problems
                .iter()
                .find(|problem| problem.code == simplified_schema::ValidationProblemCode::InvalidFileReference)
                .unwrap_or_else(|| panic!("{token}: {problems:?}"));
            let launch = format!("`{}`", self.root.display());
            assert!(problem.message.contains(raw) && problem.message.contains(&launch), "{token}: {}", problem.message);
        }

        /// Refused for `author.cover` being outside its `match()`, and nothing else.
        fn assert_only_the_cover_is_outside_its_glob(&self, tokens: &[&str]) {
            let problems = self.refusal(tokens);
            assert_eq!(problems.len(), 1, "{problems:?}");
            assert_eq!(problems[0].path, "/author/cover", "{problems:?}");
            assert!(problems[0].message.contains("is outside this schema arm's"), "{problems:?}");
        }
    }

    #[test]
    fn a_named_object_file() {
        let launch = Launch::union(PARENTS);
        let out = launch.with_setters(&["author.avatar=assets/avatar.png"]).unwrap();
        assert_eq!(out["author"]["avatar"], launch.avatar());
    }

    #[test]
    fn an_array_item_file() {
        let launch = Launch::union(PARENTS);
        let out = launch.with_setters(&["authors.0.avatar=assets/avatar.png"]).unwrap();
        assert_eq!(out["authors"][0]["avatar"], launch.avatar());
    }

    #[test]
    fn a_dictionary_value_file() {
        let launch = Launch::union(PARENTS);
        let out = launch.with_setters(&["byid.primary.avatar=assets/avatar.png"]).unwrap();
        assert_eq!(out["byid"]["primary"]["avatar"], launch.avatar());
    }

    #[test]
    fn a_recursive_child_file() {
        let launch = Launch::union(PARENTS);
        let out = launch.with_setters(&["author.children.0.avatar=assets/avatar.png"]).unwrap();
        assert_eq!(out["author"]["children"][0]["avatar"], launch.avatar());
    }

    #[test]
    fn a_whole_object_setter() {
        let launch = Launch::union(PARENTS);
        let out = launch.with_setters(&[r#"author={avatar:"assets/avatar.png",children:[]}"#]).unwrap();
        assert_eq!(out["author"]["avatar"], launch.avatar());
    }

    #[test]
    fn a_whole_object_set_override() {
        let launch = Launch::union(PARENTS);
        let out = launch.with_set(json!({"author": {"avatar": "assets/avatar.png", "children": []}})).unwrap();
        assert_eq!(out["author"]["avatar"], launch.avatar());
    }

    #[test]
    fn a_set_override_holding_an_array_of_objects() {
        let launch = Launch::union(PARENTS);
        let out = launch.with_set(json!({"authors": [{"avatar": "assets/avatar.png", "children": []}]})).unwrap();
        assert_eq!(out["authors"][0]["avatar"], launch.avatar());
    }

    #[test]
    fn a_set_override_holding_a_dictionary() {
        let launch = Launch::union(PARENTS);
        let out = launch.with_set(json!({"byid": {"primary": {"avatar": "assets/avatar.png", "children": []}}})).unwrap();
        assert_eq!(out["byid"]["primary"]["avatar"], launch.avatar());
    }

    #[test]
    fn a_nested_setter_into_a_set_override() {
        let launch = Launch::union(PARENTS);
        let out = launch
            .compose(|options| {
                options
                    .with_set_overrides(json!({"author": {"children": []}}))
                    .with_setters(plan(&["author.avatar=assets/avatar.png"]))
            })
            .unwrap();
        assert_eq!(out["author"]["avatar"], launch.avatar());
    }

    #[test]
    fn a_top_level_file_is_the_control() {
        let launch = Launch::union(PARENTS);
        let out = launch.with_setters(&["avatar=assets/avatar.png"]).unwrap();
        assert_eq!(out["avatar"], launch.avatar());
    }

    #[test]
    fn an_inline_single_schema_is_the_control() {
        let launch = Launch::new(
            "unused.yaml",
            "",
            "---\n$schema:\n  author:\n    avatar: file(eager)\nauthor: {}\n---\nbody\n",
        );
        let out = launch.with_setters(&["author.avatar=assets/avatar.png"]).unwrap();
        assert_eq!(out["author"]["avatar"], launch.avatar());
    }

    #[test]
    fn a_raw_pattern_key_dictionary() {
        let schema = r#"{"$schema":"https://json-schema.org/draft/2020-12/schema","type":"object","properties":{"map":{"type":"object","patternProperties":{"^p":{"type":"object","properties":{"avatar":{"type":"string","format":"darkmatter-file"}}}},"additionalProperties":false}}}"#;
        let launch = Launch::new("s.json", schema, "---\n$schema: ../s.json\nmap:\n  primary: {}\n---\nbody\n");
        let out = launch.with_setters(&["map.primary.avatar=assets/avatar.png"]).unwrap();
        assert_eq!(out["map"]["primary"]["avatar"], launch.avatar());
    }

    /// The authored value resolves from the document, the setter's from the launch
    /// directory, in the same parent and under the same glob.
    #[test]
    fn an_authored_sibling_keeps_the_document_origin() {
        let launch = Launch::union("---\n$schema: ../s.yaml\nkind: image\nauthor:\n  cover: ./assets/mine.png\n  children: []\n---\nbody\n");
        launch.write("sub/assets/mine.png", "document");
        let out = launch.with_setters(&["author.avatar=assets/avatar.png"]).unwrap();
        assert_eq!(out["author"]["avatar"], launch.avatar());
        let cover = out["author"]["cover"].as_str().unwrap();
        assert!(
            !cover.starts_with(launch.root.to_str().unwrap()) && cover.ends_with("mine.png"),
            "the authored value must stay document-relative: {cover}"
        );
    }

    /// Without the authored file in the document's folder, the launch directory's
    /// copy must not rescue it: the sibling setter lends it no origin.
    #[test]
    fn an_authored_sibling_is_not_rebased_on_the_launch_directory() {
        let launch = Launch::union("---\n$schema: ../s.yaml\nkind: image\nauthor:\n  cover: ./assets/launch-only.png\n  children: []\n---\nbody\n");
        launch.write("assets/launch-only.png", "launch");
        let out = launch.with_setters(&["author.avatar=assets/avatar.png"]);
        assert!(out.is_err(), "{out:?}");
    }

    #[test]
    fn a_file_only_in_the_documents_folder_is_refused_under_a_union() {
        let launch = Launch::union(PARENTS);
        launch.write("sub/assets/doc-only.png", "document");
        let failure = launch.with_setters(&["author.avatar=assets/doc-only.png"]);
        assert!(failure.is_err(), "{failure:?}");
    }

    #[test]
    fn a_top_level_file_only_in_the_documents_folder_is_refused_under_a_union() {
        let launch = Launch::union(PARENTS);
        launch.write("sub/assets/doc-only.png", "document");
        let failure = launch.with_setters(&["avatar=assets/doc-only.png"]);
        assert!(failure.is_err(), "{failure:?}");
    }

    #[test]
    fn a_file_only_in_the_documents_folder_is_refused_under_an_optional_file() {
        let launch = Launch::new(
            "unused.yaml",
            "",
            "---\n$schema:\n  spec: file(eager)\n  author:\n    avatar: file(eager)\nauthor: {}\n---\nbody\n",
        );
        launch.write("sub/doc-only.png", "document");
        for token in ["spec=doc-only.png", "author.avatar=doc-only.png"] {
            let failure = launch.with_setters(&[token]);
            assert!(failure.is_err(), "{token}: {failure:?}");
        }
    }

    #[test]
    fn a_launch_file_outside_the_glob_is_refused() {
        let launch = Launch::union(PARENTS);
        launch.write("other/outside.png", "launch");
        let failure = launch.with_setters(&["author.avatar=other/outside.png"]);
        assert!(failure.is_err(), "{failure:?}");
    }

    #[test]
    fn a_refused_nested_file_found_at_launch_names_the_launch_directory() {
        let launch = Launch::union(PARENTS);
        launch.write("other/outside.png", "launch");
        launch.assert_refusal_names_the_launch_directory("author.avatar=other/outside.png", "other/outside.png");
    }

    #[test]
    fn a_refused_top_level_file_found_at_launch_names_the_launch_directory() {
        let launch = Launch::union(PARENTS);
        launch.write("other/outside.png", "launch");
        launch.assert_refusal_names_the_launch_directory("avatar=other/outside.png", "other/outside.png");
    }

    #[test]
    fn a_refused_nested_file_missing_at_launch_names_the_launch_directory() {
        let launch = Launch::union(PARENTS);
        launch.assert_refusal_names_the_launch_directory("author.avatar=missing.png", "missing.png");
    }

    #[test]
    fn a_refused_top_level_file_missing_at_launch_names_the_launch_directory() {
        let launch = Launch::union(PARENTS);
        launch.assert_refusal_names_the_launch_directory("avatar=missing.png", "missing.png");
    }

    #[test]
    fn a_launch_path_the_document_authors_alone_is_refused_by_match() {
        let launch = Launch::union(PARENTS);
        launch.author_a_copy_of_the_avatar();
        launch.assert_only_the_cover_is_outside_its_glob(&[]);
    }

    /// A supplied value is recognized by its text, so without a judgment of its
    /// own a document's copy of that path would pass a `match()` it fails from
    /// the document's folder.
    #[test]
    fn a_launch_path_the_document_authors_stays_refused_beside_the_same_supplied_path() {
        let launch = Launch::union(PARENTS);
        launch.author_a_copy_of_the_avatar();
        launch.assert_only_the_cover_is_outside_its_glob(&["author.avatar=assets/avatar.png"]);
    }

    #[test]
    fn a_path_the_document_may_author_stays_accepted_beside_the_same_supplied_path() {
        let launch = Launch::new(
            "s.yaml",
            "kind: schema\ntypes:\n  node:\n    avatar: file(eager; match(./assets/*.png, ../assets/*.png))\n    cover: file(eager; match(./assets/*.png, ../assets/*.png))\n    children: node[]\n$schema:\n  - kind: literal(image)\n    author: node\n  - kind: literal(other)\n    title: string(required)\n",
            "",
        );
        launch.author_a_copy_of_the_avatar();
        let out = launch.with_setters(&["author.avatar=assets/avatar.png"]).unwrap();
        assert_eq!(out["author"]["avatar"], launch.avatar());
    }

    #[test]
    fn a_named_object_file_renders_in_the_top_level_spelling() {
        Launch::union(PARENTS).assert_renders_like_the_top_level("author.avatar=assets/avatar.png", "author.avatar");
    }

    #[test]
    fn an_array_item_file_renders_in_the_top_level_spelling() {
        Launch::union(PARENTS).assert_renders_like_the_top_level("authors.0.avatar=assets/avatar.png", "authors[0].avatar");
    }

    #[test]
    fn a_dictionary_value_file_renders_in_the_top_level_spelling() {
        Launch::union(PARENTS).assert_renders_like_the_top_level("byid.primary.avatar=assets/avatar.png", "byid.primary.avatar");
    }

    #[test]
    fn a_recursive_child_file_renders_in_the_top_level_spelling() {
        Launch::union(PARENTS).assert_renders_like_the_top_level("author.children.0.avatar=assets/avatar.png", "author.children[0].avatar");
    }

    #[test]
    fn a_whole_object_setter_file_renders_in_the_top_level_spelling() {
        Launch::union(PARENTS).assert_renders_like_the_top_level(r#"author={avatar:"assets/avatar.png",children:[]}"#, "author.avatar");
    }

    #[test]
    fn a_whole_read_of_the_parent_holds_the_top_level_spelling() {
        let launch = Launch::union("---\n$schema: ../s.yaml\nkind: image\nauthor:\n  children: []\n---\n{{ author }}\n");
        let path = launch.root.join("sub").join("doc.md");
        let base = ComposeOptions::new_with_context(ComposeContext::capture_for_content(&launch.root, ""))
            .with_source_file(&path)
            .with_setters(plan(&["author.avatar=assets/avatar.png"]));
        let request = crate::request_support::request_at(&launch.root, base);
        let (composed, _) = Markdown::try_from(path.as_path()).unwrap().compose_with(&request).unwrap();
        let rendered: Value = serde_json::from_str(composed.content().trim()).unwrap();
        let avatar = launch.root.join("assets").join("avatar.png");
        assert_eq!(rendered, json!({"avatar": biscuit_file::to_portable_string(&avatar), "children": []}));
    }

    /// The rendered text is portable, while the value later stages read stays native.
    #[test]
    fn a_nested_file_stays_native_in_the_frontmatter() {
        let launch = Launch::union(PARENTS);
        let out = launch.with_setters(&["author.avatar=assets/avatar.png"]).unwrap();
        assert_eq!(out["author"]["avatar"], launch.avatar());
    }

    #[test]
    fn a_transcluded_child_resolves_the_setter_from_the_launch_directory() {
        let launch = Launch::union("---\n$schema: ../s.yaml\nkind: image\nauthor:\n  children: []\n---\n::file ./child.md\n");
        launch.write("sub/child.md", "---\n$schema: ../s.yaml\nkind: image\nauthor:\n  children: []\n---\nCHILD {{ author.avatar }}\n");
        let path = launch.root.join("sub").join("doc.md");
        let base = ComposeOptions::new_with_context(ComposeContext::capture_for_content(&launch.root, ""))
            .with_source_file(&path)
            .with_setters(plan(&["author.avatar=assets/avatar.png"]));
        let request = crate::request_support::request_at(&launch.root, base);
        let (composed, _) = Markdown::try_from(path.as_path()).unwrap().compose_with(&request).unwrap();
        let rendered = biscuit_file::to_portable_string(std::path::Path::new(launch.avatar().as_str().unwrap()));
        assert!(composed.content().contains(&format!("CHILD {rendered}")), "{}", composed.content());
    }

    /// `--state` fills in a value as the document's default, so it is judged from
    /// the document's folder at any depth, as at the top level.
    #[test]
    fn a_state_default_keeps_the_document_origin() {
        let launch = Launch::union("---\n$schema: ../s.yaml\nkind: image\n---\nbody\n");
        let from_launch = launch.compose(|options| options.with_external_state(json!({"author": {"avatar": "assets/avatar.png", "children": []}})));
        assert!(from_launch.is_err(), "{from_launch:?}");
        let top_level = launch.compose(|options| options.with_external_state(json!({"avatar": "assets/avatar.png"})));
        assert!(top_level.is_err(), "{top_level:?}");

        launch.write("sub/assets/doc-side.png", "document");
        let from_document = launch.compose(|options| options.with_external_state(json!({"author": {"avatar": "assets/doc-side.png", "children": []}})));
        assert!(from_document.is_ok(), "{from_document:?}");
    }

    #[test]
    fn a_template_in_a_nested_setter_is_judged_like_one_at_the_top_level() {
        let launch = Launch::union(PARENTS);
        launch.write("sub/assets/doc-side.png", "document");
        for token in [r#"avatar={{ "assets/doc-side.png" }}"#, r#"author.avatar={{ "assets/doc-side.png" }}"#] {
            let out = launch.with_setters(&[token]);
            assert!(out.is_ok(), "{token}: {out:?}");
        }
    }
}
