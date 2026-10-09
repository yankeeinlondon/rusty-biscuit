//! `--format markdown` and `--format prose` share the listing's layout:
//! Markdown is parsed as CommonMark and checked item by item (nesting,
//! headings, emphasis, literal source text), and Prose is rendered back
//! through biscuit-terminal and compared with `--format plain` both ways the
//! export contract states.

use biscuit_terminal::discovery::detection::ColorDepth;
use biscuit_terminal::prelude::{Prose, Terminal, TerminalRenderable, WordWrap, strip_escape_codes};
use pulldown_cmark::{Event, Parser, Tag, TagEnd};

use super::*;

const HOSTILE_DOC: &str = "Zebra docs `code` [x](u) **b** <img> &copy; | ~s~";

/// Two Cargo packages with declarations and one without, so listings get
/// package sections; `a.rs` declares `zebra` and `z.rs` declares `alpha`,
/// so name order differs from file order; the Java class has a base,
/// overloads, static and instance methods and fields.
const WORKSPACE: &[(&str, &str)] = &[
    ("Cargo.toml", "[package]\nname = \"root\"\nversion = \"0.1.0\"\n"),
    (
        "src/a.rs",
        "/// Zebra docs `code` [x](u) **b** <img> &copy; | ~s~\n/// - second\npub fn zebra(x: Vec<i32>) -> i32 { 0 }\n\
         pub struct Holder { pub value: i32 }\nimpl Holder {\n    pub fn get(&self) -> i32 { self.value }\n}\n",
    ),
    ("src/z.rs", "pub fn alpha() {}\n"),
    ("crates/b/Cargo.toml", "[package]\nname = \"b\"\nversion = \"0.1.0\"\n"),
    (
        "crates/b/src/Example.java",
        "/** Base docs. */\nclass Base {}\nclass Box extends Base {\n    /** Builds one. */\n    static void build() {}\n\
         \x20   void open() {}\n    void open(int count) {}\n    int field;\n    static int total;\n}\n",
    ),
    ("crates/empty/Cargo.toml", "[package]\nname = \"empty\"\nversion = \"0.1.0\"\n"),
    ("crates/empty/src/lib.rs", "// nothing\n"),
];

/// The Java class alone, in one file at the root of a manifest-less repo.
const EXAMPLE: &[(&str, &str)] = &[(
    "Example.java",
    "class Base {}\nclass Box extends Base {\n    static void build() {}\n    void open() {}\n    void open(int count) {}\n\
     \x20   int field;\n}\n",
)];

/// The Markdown document as one line per heading, list item or paragraph:
/// `H3 text`, `- text` indented two spaces per nesting level, `P text`.
/// Strong and emphasis show as `**`/`*`, a hard break as ` / `. Any other
/// event (link, image, HTML, inline code, code block, rule) fails: source
/// text must never produce one.
fn shape(markdown: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut depth = 0usize;
    let mut current: Option<String> = None;
    for event in Parser::new(markdown) {
        match event {
            Event::Start(Tag::Heading { level, .. }) => current = Some(format!("{level:?} ")),
            Event::End(TagEnd::Heading(_)) => lines.extend(current.take()),
            Event::Start(Tag::List(None)) => {
                lines.extend(current.take());
                depth += 1;
            }
            Event::End(TagEnd::List(false)) => depth -= 1,
            Event::Start(Tag::Item) => current = Some(format!("{}- ", "  ".repeat(depth - 1))),
            Event::End(TagEnd::Item) => lines.extend(current.take()),
            Event::Start(Tag::Paragraph) if current.is_none() => current = Some("P ".to_string()),
            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) if depth == 0 => lines.extend(current.take()),
            Event::End(TagEnd::Paragraph) => {}
            Event::Text(text) => current.as_mut().expect("text inside a block").push_str(&text),
            Event::Start(Tag::Strong) | Event::End(TagEnd::Strong) => current.as_mut().expect("block").push_str("**"),
            Event::Start(Tag::Emphasis) | Event::End(TagEnd::Emphasis) => current.as_mut().expect("block").push('*'),
            Event::HardBreak => current.as_mut().expect("block").push_str(" / "),
            other => panic!("unexpected Markdown event {other:?} in:\n{markdown}"),
        }
    }
    lines
}

fn markdown(scratch: &Scratch, args: &[&str]) -> Vec<String> {
    let mut full = args.to_vec();
    full.extend(["--format", "markdown"]);
    shape(&scratch.stdout(&full))
}

#[test]
fn markdown_functions_tree_nests_methods_under_their_owner_context() {
    let scratch = Scratch::new(EXAMPLE);
    assert_eq!(
        markdown(&scratch, &["functions", "--tree"]),
        [
            "H3 **Example.java**",
            "- [in class Box]",
            "  - *package* method **build() -> void** [3:5]",
            "  - *package* method **open() -> void** [4:5]",
            "  - *package* method **open(count: int) -> void** [5:5]",
        ]
    );
}

#[test]
fn markdown_types_tree_keeps_declared_bases_without_signatures() {
    let scratch = Scratch::new(EXAMPLE);
    assert_eq!(
        markdown(&scratch, &["types", "--tree", "--signatures", "none"]),
        [
            "H3 **Example.java**",
            "- *package* class **Base** [1:1]",
            "- *package* class **Box** extends Base [2:1]",
        ]
    );
}

#[test]
fn markdown_symbols_tree_heads_a_filtered_owner_as_context_only() {
    let scratch = Scratch::new(EXAMPLE);
    let shape = markdown(&scratch, &["symbols", "--tree", "--name", "open"]);
    assert_eq!(
        shape,
        [
            "H3 **Example.java**",
            "- [in class Box]",
            "  - *package* method **open() -> void** [4:5]",
            "  - *package* method **open(count: int) -> void** [5:5]",
        ]
    );
    let full = markdown(&scratch, &["symbols", "--tree"]);
    assert_eq!(
        full,
        [
            "H3 **Example.java**",
            "- *package* class **Base** [1:1]",
            "- *package* class **Box { field: int }** extends Base [2:1]",
            "  - *package* method **build() -> void** [3:5]",
            "  - *package* method **open() -> void** [4:5]",
            "  - *package* method **open(count: int) -> void** [5:5]",
            "  - *package* field **field** [6:9]",
        ]
    );
}

#[test]
fn markdown_classes_keep_method_and_field_sections_for_every_selection() {
    let scratch = Scratch::new(EXAMPLE);
    let header = "H3 **Example.java** (Java)";
    assert_eq!(
        markdown(&scratch, &["classes"]),
        [
            header,
            "- class **Base** [1:1]",
            "- class **Box** [2:1]",
            "  - Static Methods (1)",
            "    - *package* build() -> void [in class Box] [3:5]",
            "  - Instance Methods (2)",
            "    - *package* open() -> void [in class Box] [4:5]",
            "    - *package* open(count: int) -> void [in class Box] [5:5]",
            "  - Instance Fields (1)",
            "    - *package* field: int",
        ]
    );
    assert_eq!(
        markdown(&scratch, &["classes", "--static-only"]),
        [
            header,
            "- class **Base** [1:1]",
            "- class **Box** [2:1]",
            "  - Static Methods (1)",
            "    - *package* build() -> void [in class Box] [3:5]",
        ]
    );
    let instance = markdown(&scratch, &["classes", "--instance-only", "--signatures", "none"]);
    assert_eq!(
        instance,
        [
            header,
            "- class **Base** [1:1]",
            "- class **Box** [2:1]",
            "  - Instance Methods (2)",
            "    - *package* open [in class Box] [4:5]",
            "    - *package* open [in class Box] [5:5]",
            "  - Instance Fields (1)",
            "    - *package* field: int",
        ]
    );
    assert_eq!(markdown(&scratch, &["classes", "--name", "absent"]), ["P (no symbols)"]);
}

#[test]
fn markdown_groups_and_packages_become_headings_with_empty_sections() {
    let scratch = Scratch::new(WORKSPACE);
    assert_eq!(
        markdown(&scratch, &["functions", "--group-by-module", "--group-by-file", "--signatures", "none"]),
        [
            "H2 .",
            "H3 **src**",
            "H4 src/a.rs",
            "- *public* function **zebra** [3:1]",
            "- *public* method **get [in type Holder]** [6:5]",
            "H4 src/z.rs",
            "- *public* function **alpha** [1:1]",
            "H2 **crates/b**",
            "H3 **crates/b/src**",
            "H4 crates/b/src/Example.java",
            "- *package* method **build [in class Box]** [5:5]",
            "- *package* method **open [in class Box]** [6:5]",
            "- *package* method **open [in class Box]** [7:5]",
            "H2 **crates/empty**",
            "P (no symbols)",
        ]
    );
    let by_file = markdown(&scratch, &["functions", "--group-by-file", "--signatures", "none", "--name", "alpha"]);
    assert_eq!(
        by_file,
        [
            "H2 .",
            "H3 **src/z.rs**",
            "- *public* function **alpha** [1:1]",
            "H2 **crates/b**",
            "P (no symbols)",
            "H2 **crates/empty**",
            "P (no symbols)",
        ]
    );
    let by_module = markdown(&scratch, &["types", "--group-by-module", "--signatures", "none"]);
    assert_eq!(
        by_module,
        [
            "H2 .",
            "H3 **src**",
            "- *public* struct **Holder** src/a.rs:4:1",
            "H2 **crates/b**",
            "H3 **crates/b/src**",
            "- *package* class **Base** crates/b/src/Example.java:2:1",
            "- *package* class **Box** crates/b/src/Example.java:3:1",
            "H2 **crates/empty**",
            "P (no symbols)",
        ]
    );
    let empty = markdown(&scratch, &["symbols", "--name", "absent"]);
    assert_eq!(
        empty,
        ["H2 .", "P (no symbols)", "H2 **crates/b**", "P (no symbols)", "H2 **crates/empty**", "P (no symbols)"]
    );
    let single = Scratch::new(EXAMPLE);
    assert_eq!(markdown(&single, &["symbols", "--name", "absent"]), ["P (no symbols)"]);
    assert_eq!(markdown(&single, &["symbols", "--tree", "--name", "absent"]), ["P (no symbols)"]);
}

#[test]
fn markdown_ungrouped_name_sort_orders_the_whole_package() {
    let scratch = Scratch::new(WORKSPACE);
    let shape = markdown(&scratch, &["functions", "--sort", "name", "--signatures", "none", "--file", "src/**"]);
    assert_eq!(
        shape,
        [
            "- *public* function **alpha** src/z.rs:1:1",
            "- *public* method **get [in type Holder]** src/a.rs:6:5",
            "- *public* function **zebra** src/a.rs:3:1",
        ]
    );
}

#[test]
fn markdown_declarations_share_the_listing_text_in_every_signature_mode() {
    let scratch = Scratch::new(WORKSPACE);
    let args = |mode: &'static str| ["functions", "--file", "src/a.rs", "--name", "zebra", "--signatures", mode];
    assert_eq!(
        markdown(&scratch, &args("short")),
        ["- *public* function **zebra(x: Vec<i32>) -> i32** src/a.rs:3:1"]
    );
    // The header already names the declaration: it is shown once.
    assert_eq!(markdown(&scratch, &args("full")), ["- **pub fn zebra(x: Vec<i32>) -> i32** src/a.rs:3:1"]);
    assert_eq!(markdown(&scratch, &args("none")), ["- *public* function **zebra** src/a.rs:3:1"]);
}

#[test]
fn markdown_comments_continue_their_item_as_literal_text() {
    let scratch = Scratch::new(WORKSPACE);
    let shape = markdown(&scratch, &["functions", "--file", "src/a.rs", "--name", "zebra", "--comments"]);
    assert_eq!(
        shape,
        [format!("- *public* function **zebra(x: Vec<i32>) -> i32** src/a.rs:3:1 / {HOSTILE_DOC} / - second")]
    );
    let classes = markdown(&scratch, &["classes", "--file", "crates/b/src/Example.java", "--comments", "--static-only"]);
    assert_eq!(
        classes,
        [
            "H3 **crates/b/src/Example.java** (Java)",
            "- class **Base** [2:1] / Base docs.",
            "- class **Box** [3:1]",
            "  - Static Methods (1)",
            "    - *package* build() -> void [in class Box] [5:5] / Builds one.",
            "  - Static Fields (1)",
            "    - *package* total: int",
        ]
    );
}

// Windows rejects newlines and angle brackets in file names.
#[cfg(unix)]
#[test]
fn markdown_source_newlines_and_markup_in_paths_stay_inert() {
    let name = "line\n\n        indented # [x](u) <b>.rs";
    let scratch = Scratch::new(&[(name, "fn sample() {}\n")]);
    let output = scratch.hug(&["functions", "--format", "markdown", "--file", "line*"]);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let markdown = String::from_utf8(output.stdout).expect("utf-8");
    assert_eq!(
        shape(&markdown),
        ["- *private* function **sample()** line\\u{a}\\u{a}        indented # [x](u) <b>.rs:1:1"]
    );
}

/// Prose rendered line by line (each generated tag closes within its line)
/// on `term`, unwrapped, without escape sequences.
fn rendered(prose: &str, term: &Terminal) -> Vec<String> {
    prose
        .lines()
        .map(|line| strip_escape_codes(Prose::new(line).with_word_wrap(WordWrap::None).render(term)))
        .collect()
}

fn plain_terminal() -> Terminal {
    let mut term = Terminal::builder()
        .is_tty(false)
        .color_depth(ColorDepth::None)
        .osc_link_support(false)
        .build();
    term.fixed_width = Some(1_000_000);
    term
}

fn link_terminal() -> Terminal {
    let mut term = Terminal::new_optimistic(1_000_000);
    term.fixed_width = Some(1_000_000);
    term
}

const MODES: &[&[&str]] = &[
    &["--signatures", "short", "--comments"],
    &["--signatures", "full"],
    &["--signatures", "none", "--comments"],
];

/// Both comparisons the Prose contract states, for `command` + `view` in
/// every signature mode, with and without a filtered selection:
/// `--plain --format prose` rendered without styles or links, and styled
/// Prose rendered on an OSC-8-capable terminal with escapes stripped, each
/// equal `--format plain`.
fn assert_prose_matches_plain(command: &str, view: &[&str]) {
    let scratch = Scratch::new(WORKSPACE);
    let (plain_term, link_term) = (plain_terminal(), link_terminal());
    for selection in [&[][..], &["--name", "open"], &["--name", "absent"]] {
        for mode in MODES {
            let mut args = vec![command];
            args.extend(view);
            args.extend(selection);
            args.extend(*mode);
            let context = args.join(" ");
            let plain: Vec<String> =
                scratch.stdout(&[args.as_slice(), &["--format", "plain"]].concat()).lines().map(str::to_string).collect();
            let unstyled = scratch.stdout(&[&["--plain"], args.as_slice(), &["--format", "prose"]].concat());
            assert!(!unstyled.contains("<a ") && !unstyled.contains("<b>"), "{context}: {unstyled}");
            assert_eq!(rendered(&unstyled, &plain_term), plain, "--plain prose vs plain: {context}");
            let styled = scratch.stdout(&[args.as_slice(), &["--format", "prose"]].concat());
            let shows_a_file = plain.iter().any(|line| line.contains(".rs") || line.contains(".java"));
            assert_eq!(styled.contains("<a href="), shows_a_file, "{context}: styled Prose links each file and location");
            assert_eq!(rendered(&styled, &link_term), plain, "styled prose vs plain: {context}");
        }
    }
}

macro_rules! prose_comparisons {
    ($($name:ident: $command:literal $(, $flag:literal)*;)*) => {
        $(
            #[test]
            fn $name() {
                assert_prose_matches_plain($command, &[$($flag),*]);
            }
        )*
    };
}

prose_comparisons! {
    prose_matches_plain_functions: "functions";
    prose_matches_plain_functions_tree: "functions", "--tree";
    prose_matches_plain_functions_by_file: "functions", "--group-by-file";
    prose_matches_plain_functions_by_module: "functions", "--group-by-module";
    prose_matches_plain_functions_by_module_and_file: "functions", "--group-by-module", "--group-by-file";
    prose_matches_plain_functions_by_name: "functions", "--sort", "name";
    prose_matches_plain_types: "types";
    prose_matches_plain_types_tree: "types", "--tree";
    prose_matches_plain_types_by_kind: "types", "--sort", "kind";
    prose_matches_plain_symbols: "symbols";
    prose_matches_plain_symbols_tree: "symbols", "--tree";
    prose_matches_plain_symbols_tree_by_module: "symbols", "--tree", "--group-by-module";
    prose_matches_plain_symbols_tree_by_name: "symbols", "--tree", "--sort", "name";
    prose_matches_plain_symbols_by_file: "symbols", "--group-by-file";
    prose_matches_plain_symbols_by_kind: "symbols", "--sort", "kind";
    prose_matches_plain_classes: "classes";
    prose_matches_plain_classes_static: "classes", "--static-only";
    prose_matches_plain_classes_instance: "classes", "--instance-only";
    prose_matches_plain_classes_by_name: "classes", "--sort", "name";
}

#[test]
fn prose_keeps_source_markup_literal_in_both_presentations() {
    let scratch = Scratch::new(WORKSPACE);
    for global in [&[][..], &["--plain"]] {
        let args = [global, &["functions", "--file", "src/a.rs", "--name", "zebra", "--comments", "--format", "prose"]].concat();
        let prose = scratch.stdout(&args);
        let lines = rendered(&prose, &plain_terminal());
        assert!(lines.iter().any(|line| line.trim() == HOSTILE_DOC), "{args:?}: {lines:?}");
        assert!(lines.iter().any(|line| line.contains("zebra(x: Vec<i32>) -> i32")), "{args:?}: {lines:?}");
    }
}

/// Source text with angle brackets in every place a declaration report
/// shows it: generic headers (Rust, Java, TypeScript), class member
/// summaries, doc lines with an HTML-like tag, Prose-like emphasis and an
/// unmatched `<`, and impl owners written with type arguments.
const GENERIC: &[(&str, &str)] = &[
    (
        "x.rs",
        "/// literal <img> and **bold**\n/// a < b unmatched `tick\npub fn sample(x: Vec<i32>) {}\n\
         impl<T> Other<T> {\n    pub fn probe(&self) {}\n}\n",
    ),
    ("G.java", "class Range {\n    void open(java.util.List<Integer> start) {}\n    java.util.Map<String, Integer> index;\n}\n"),
    ("g.ts", "export class Box<T> {\n  get(): Array<T> { return []; }\n}\n"),
];

/// `--plain … --format prose` rendered through biscuit-terminal Prose with
/// no color and no links: exactly the source text, nothing deleted and no
/// escape backslash left behind.
fn plain_prose(scratch: &Scratch, args: &[&str]) -> Vec<String> {
    let prose = scratch.stdout(&[&["--plain"], args, &["--format", "prose"]].concat());
    rendered(&prose, &plain_terminal())
}

#[test]
fn plain_prose_keeps_generic_headers_literal() {
    let scratch = Scratch::new(GENERIC);
    assert_eq!(
        plain_prose(&scratch, &["functions", "--signatures", "full", "--file", "x.rs", "--file", "G.java"]),
        [
            "  - void open(java.util.List<Integer> start) [in class Range] G.java:2:5",
            "  - pub fn sample(x: Vec<i32>) x.rs:3:1",
            "  - pub fn probe(&self) [in Other] x.rs:5:5",
        ]
    );
}

#[test]
fn plain_prose_keeps_generic_short_signatures_and_class_summaries_literal() {
    let scratch = Scratch::new(GENERIC);
    assert_eq!(
        plain_prose(&scratch, &["functions", "--file", "G.java", "--file", "g.ts"]),
        [
            "  - package method open(start: java.util.List<Integer>) -> void [in class Range] G.java:2:5",
            "  - public method get(): Array<T> [in class Box] g.ts:2:3",
        ]
    );
    assert_eq!(
        plain_prose(&scratch, &["classes", "--file", "G.java", "--file", "g.ts"]),
        [
            "G.java (Java)",
            "  class Range [1:1]",
            "    Instance Methods (1)",
            "      - package open(start: java.util.List<Integer>) -> void [in class Range] [2:5]",
            "    Instance Fields (1)",
            "      - package index: java.util.Map<String, Integer>",
            "",
            "g.ts (TypeScript)",
            "  class Box [1:8]",
            "    Instance Methods (1)",
            "      - public get(): Array<T> [in class Box] [2:3]",
            "",
        ]
    );
}

#[test]
fn plain_prose_keeps_doc_tags_emphasis_and_unmatched_angles_literal() {
    let scratch = Scratch::new(GENERIC);
    assert_eq!(
        plain_prose(&scratch, &["functions", "--comments", "--file", "x.rs", "--name", "sample"]),
        [
            "  - public function sample(x: Vec<i32>) x.rs:3:1",
            "    literal <img> and **bold**",
            "    a < b unmatched `tick",
        ]
    );
}

#[test]
fn plain_prose_keeps_generic_owner_names_literal() {
    let scratch = Scratch::new(GENERIC);
    assert_eq!(
        plain_prose(&scratch, &["symbols", "--tree", "--signatures", "full", "--file", "x.rs", "--file", "g.ts"]),
        [
            "g.ts",
            "  class Box<T> [1:8]",
            "    get(): Array<T> [2:3]",
            "",
            "x.rs",
            "  pub fn sample(x: Vec<i32>) [3:1]",
            "  [in Other<T>]",
            "    pub fn probe(&self) [5:5]",
            "",
        ]
    );
}

// Windows rejects angle brackets in file names.
#[cfg(unix)]
#[test]
fn plain_prose_keeps_angle_brackets_in_paths_literal() {
    let scratch = Scratch::new(&[("x<tag>.rs", "fn sample() {}\n")]);
    assert_eq!(
        plain_prose(&scratch, &["functions", "--file", "x*"]),
        ["  - private function sample() x<tag>.rs:1:1"]
    );
}

#[test]
fn styled_prose_tsv_and_markdown_keep_generic_text() {
    let scratch = Scratch::new(GENERIC);
    let args = ["functions", "--signatures", "full", "--file", "G.java", "--file", "x.rs", "--name", "open", "--name", "sample"];
    let styled = scratch.stdout(&[&args[..], &["--format", "prose"]].concat());
    assert_eq!(
        rendered(&styled, &link_terminal()),
        [
            "  - void open(java.util.List<Integer> start) [in class Range] G.java:2:5",
            "  - pub fn sample(x: Vec<i32>) x.rs:3:1",
        ]
    );
    let tsv = scratch.stdout(&[&args[..], &["--format", "tsv"]].concat());
    let signatures: Vec<&str> = tsv.lines().skip(1).filter_map(|row| row.split('\t').nth(7)).collect();
    assert_eq!(signatures, ["void open(java.util.List<Integer> start)", "pub fn sample(x: Vec<i32>)"]);
    assert_eq!(
        markdown(&scratch, &args),
        [
            "- **void open(java.util.List<Integer> start) [in class Range]** G.java:2:5",
            "- **pub fn sample(x: Vec<i32>)** x.rs:3:1",
        ]
    );
}
