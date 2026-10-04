//! DMLS consumes Darkmatter's binding classification (spec R7a, R7b;
//! acceptance criterion 10): an undeclared bare property is a valid
//! unknown-typed document property reported as one advisory, an unknown
//! function stays an error, bare names are document properties in completion
//! and hover, and DMLS keeps no root catalog of its own.

use crate::common;

use std::path::{Path, PathBuf};

use common::{LspFixture, LspWorkspace};
use darkmatter::markdown::compose::expression::reserved_root_descriptors;
use serde_json::{Value, json};

const UNDECLARED: &str = "dm.expression.undeclared_property";
const UNKNOWN_FUNCTION: &str = "dm.expression.unknown_function";

fn initialize_params(root: &Path) -> Value {
    let root_uri = url::Url::from_directory_path(root).unwrap();
    json!({
        "processId": null,
        "capabilities": {
            "general": { "positionEncodings": ["utf-8", "utf-16"] },
            "textDocument": { "foldingRange": { "lineFoldingOnly": true } }
        },
        "workspaceFolders": [ { "uri": root_uri.as_str(), "name": "scratch" } ]
    })
}

/// Opens `text` as `name` in a fresh session and returns its diagnostics.
fn open_document<'w>(workspace: &'w LspWorkspace, name: &str, text: &str) -> (LspFixture<'w>, String, Vec<Value>) {
    let path = workspace.path().join(name);
    std::fs::write(&path, text).unwrap();
    let mut fixture = LspFixture::start(workspace);
    fixture.initialize(initialize_params(workspace.path()));
    let uri = url::Url::from_file_path(&path).unwrap().as_str().to_string();
    fixture.notify(
        "textDocument/didOpen",
        json!({ "textDocument": { "uri": uri, "languageId": "markdown", "version": 1, "text": text } }),
    );
    let diagnostics = fixture.wait_for_diagnostics(&uri);
    (fixture, uri, diagnostics)
}

/// `(line, start, end)` of `needle` on the line starting with `marker`.
fn span_on(doc: &str, marker: &str, needle: &str) -> (u64, u64, u64) {
    let (line, text) = doc
        .lines()
        .enumerate()
        .find(|(_, text)| text.starts_with(marker))
        .unwrap_or_else(|| panic!("no line starting with {marker}"));
    let start = text.find(needle).unwrap_or_else(|| panic!("{needle} not on {marker}"));
    (line as u64, start as u64, (start + needle.len()) as u64)
}

/// `(span, source, severity, message)` of one published diagnostic.
type Reported = ((u64, u64, u64), String, u64, String);

/// Every diagnostic with `code`, sorted.
fn with_code(diagnostics: &[Value], code: &str) -> Vec<Reported> {
    let mut found: Vec<_> = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic["code"] == json!(code))
        .map(|diagnostic| {
            let range = &diagnostic["range"];
            (
                (
                    range["start"]["line"].as_u64().unwrap(),
                    range["start"]["character"].as_u64().unwrap(),
                    range["end"]["character"].as_u64().unwrap(),
                ),
                diagnostic["source"].as_str().unwrap().to_string(),
                diagnostic["severity"].as_u64().unwrap(),
                diagnostic["message"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    found.sort();
    found
}

fn advisory(root: &str) -> String {
    format!("`{root}` is an undeclared document property (unknown type; `null` unless supplied at runtime)")
}

/// Bare names in every representation: undeclared (body and an `expression`
/// frontmatter value), handled absence, schema-declared but unset, present,
/// and unknown functions in a live branch, an inactive branch, and
/// frontmatter.
const CONTRACT_DOC: &str = concat!(
    "---\n",
    "$schema:\n",
    "  gate: expression\n",
    "  check: expression\n",
    "  declared: string -> A caller-supplied note\n",
    "title: Review\n",
    "repo: biscuit\n",
    "gate: supplied_flag\n",
    "check: no_such_check(title)\n",
    "---\n",
    "\n",
    "U1: {{ supplied_later }}\n",
    "U2: {{ supplied_later || 'fallback' }} {{ is_null(supplied_later) }}\n",
    "U3: {{ declared }} {{ title }} {{ repo }} {{ null }} {{ ctx.repo }} {{ doc.supplied_later }}\n",
    "F1: {{ no_such_fn(title) }}\n",
    "F2: {{ title ? upper(title) : missing_fn(title) }}\n",
);

#[test]
fn an_undeclared_property_is_one_advisory_and_an_unknown_function_an_error() {
    let workspace = LspWorkspace::new();
    let (fixture, _, diagnostics) = open_document(&workspace, "contract.md", CONTRACT_DOC);
    fixture.shutdown();

    // One WARNING per undeclared read, worded as a valid unknown-typed
    // property. A runtime-supplied name is not a parser error.
    assert_eq!(
        with_code(&diagnostics, UNDECLARED),
        vec![
            (span_on(CONTRACT_DOC, "gate:", "supplied_flag"), "darkmatter.frontmatter".to_string(), 2, advisory("supplied_flag")),
            (span_on(CONTRACT_DOC, "U1:", "supplied_later"), "darkmatter.compose".to_string(), 2, advisory("supplied_later")),
        ],
        "{diagnostics:#?}"
    );
    assert!(with_code(&diagnostics, "dm.expression.malformed").is_empty(), "{diagnostics:#?}");

    // Unknown functions are distinct ERRORs, ranged on the name, in every
    // branch: the closed catalog makes compose fail on each.
    let unknown_function = |name: &str| format!("`{name}` is not a Darkmatter expression function");
    assert_eq!(
        with_code(&diagnostics, UNKNOWN_FUNCTION),
        vec![
            (span_on(CONTRACT_DOC, "check:", "no_such_check"), "darkmatter.frontmatter".to_string(), 1, unknown_function("no_such_check")),
            (span_on(CONTRACT_DOC, "F1:", "no_such_fn"), "darkmatter.compose".to_string(), 1, unknown_function("no_such_fn")),
            (span_on(CONTRACT_DOC, "F2:", "missing_fn"), "darkmatter.compose".to_string(), 1, unknown_function("missing_fn")),
        ],
        "{diagnostics:#?}"
    );

    // Advisories never escalate: the only errors are the unknown functions.
    let errors = diagnostics.iter().filter(|diagnostic| diagnostic["severity"] == json!(1)).count();
    assert_eq!(errors, 3, "{diagnostics:#?}");
}

/// Supplying the property clears its advisory: the report tracks declaration,
/// and an unknown function is reported even without frontmatter, where no
/// bare name is (it could be a `--set` value).
#[test]
fn declaring_the_property_clears_the_advisory_and_functions_need_no_frontmatter() {
    let workspace = LspWorkspace::new();
    let declared = "---\nsupplied_later: soon\n---\n\nU1: {{ supplied_later }}\n";
    let (fixture, _, diagnostics) = open_document(&workspace, "declared.md", declared);
    fixture.shutdown();
    assert!(with_code(&diagnostics, UNDECLARED).is_empty(), "{diagnostics:#?}");

    let workspace = LspWorkspace::new();
    let bare = "# No frontmatter\n\nF1: {{ no_such_fn(anything) }}\n";
    let (fixture, _, diagnostics) = open_document(&workspace, "bare.md", bare);
    fixture.shutdown();
    assert!(with_code(&diagnostics, UNDECLARED).is_empty(), "{diagnostics:#?}");
    assert_eq!(with_code(&diagnostics, UNKNOWN_FUNCTION).len(), 1, "{diagnostics:#?}");
}

const CLASSIFY_DOC: &str = "---\nrepo: biscuit\n---\n\nH1: {{ repo }}\nH2: {{ branch }}\nC1: {{ rep }}\n";

#[test]
fn completion_and_hover_classify_bare_names_as_document_properties() {
    let workspace = LspWorkspace::new();
    let (mut fixture, uri, _) = open_document(&workspace, "classify.md", CLASSIFY_DOC);
    let hover = |fixture: &mut LspFixture<'_>, marker: &str, needle: &str| {
        let (line, start, _) = span_on(CLASSIFY_DOC, marker, needle);
        let result = fixture
            .request(
                "textDocument/hover",
                json!({ "textDocument": { "uri": uri }, "position": { "line": line, "character": start + 1 } }),
            )
            .result
            .expect("hover");
        result["contents"]["value"].as_str().unwrap_or_default().to_string()
    };

    // A bare name matching a context variable reads the document, never `ctx`.
    let repo = hover(&mut fixture, "H1:", "repo");
    assert!(repo.contains("Static value: `biscuit` (from frontmatter `repo`)"), "{repo}");
    assert!(!repo.contains("**`ctx.repo`**"), "{repo}");
    let branch = hover(&mut fixture, "H2:", "branch");
    assert!(!branch.contains("**`ctx.branch`**"), "{branch}");

    let (line, _, end) = span_on(CLASSIFY_DOC, "C1:", "rep");
    let completions = fixture
        .request(
            "textDocument/completion",
            json!({ "textDocument": { "uri": uri }, "position": { "line": line, "character": end } }),
        )
        .result
        .expect("completions");
    let items = completions.as_array().cloned().unwrap_or_default();
    // FIELD (5) is a document property; no context variable is offered bare.
    assert!(
        items.iter().any(|item| item["label"] == json!("repo") && item["kind"] == json!(5)),
        "{items:#?}"
    );
    assert!(!items.iter().any(|item| item["kind"] == json!(6)), "{items:#?}");
    fixture.shutdown();
}

/// Every non-test `src/` line, as `(path relative to src, line number, text)`.
/// A file stops at its trailing `#[cfg(test)] mod … {`, and `*tests.rs`
/// files are skipped.
fn production_lines(src: &Path) -> Vec<(String, usize, String)> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|ext| ext == "rs")
                && !path.file_name().unwrap().to_string_lossy().ends_with("tests.rs")
            {
                out.push(path);
            }
        }
    }
    let mut files = Vec::new();
    walk(src, &mut files);
    files.sort();
    let mut lines = Vec::new();
    for file in files {
        let relative = file.strip_prefix(src).unwrap().to_string_lossy().replace('\\', "/");
        let text = std::fs::read_to_string(&file).unwrap();
        let all: Vec<&str> = text.lines().collect();
        for (index, line) in all.iter().enumerate() {
            if line.trim() == "#[cfg(test)]"
                && all.get(index + 1).is_some_and(|next| next.trim_start().starts_with("mod ") && next.trim_end().ends_with('{'))
            {
                break;
            }
            lines.push((relative.clone(), index + 1, line.to_string()));
        }
    }
    lines
}

/// R7b: DMLS takes reserved roots and globals from Darkmatter's binding model,
/// so no source line spells a root name as a string literal, keeps a host
/// global list, or opens an evaluation session (which could run a lazy
/// provider).
#[test]
fn dmls_has_no_separate_root_catalog_or_evaluation_session() {
    // `corpus.rs` draws synthetic file names from a word list that includes
    // the English word "current"; it names no expression root.
    const ALLOWED: &[(&str, &str)] = &[("corpus.rs", "\"current\"")];

    let src = biscuit_test_harness::manifest_dir!().join("src");
    let lines = production_lines(&src);
    assert!(lines.len() > 10_000, "scanned only {} lines", lines.len());

    let mut forbidden: Vec<String> = reserved_root_descriptors()
        .iter()
        .map(|root| format!("\"{}\"", root.name))
        .collect();
    // `null` and the Claudine lifecycle globals are not Darkmatter roots, so
    // a literal for one would be exactly the drifting catalog R7b forbids.
    forbidden.extend(["null", "err", "timing", "group", "outputs"].map(|name| format!("\"{name}\"")));
    forbidden.extend(
        ["LATE_BINDING_ROOTS", "EvaluationSession", "RuntimeBinding", "LazyProvider", "evaluate_prepared", "SubtreeCompose"]
            .map(str::to_string),
    );

    let violations: Vec<String> = lines
        .iter()
        .filter(|(_, _, text)| !text.trim_start().starts_with("//"))
        .flat_map(|(file, number, text)| {
            forbidden
                .iter()
                .filter(|needle| text.contains(needle.as_str()))
                .filter(|needle| !ALLOWED.contains(&(file.as_str(), needle.as_str())))
                .map(move |needle| format!("src/{file}:{number}: {needle}"))
        })
        .collect();
    assert!(violations.is_empty(), "DMLS spells its own root classification:\n{}", violations.join("\n"));

    // Each allowance still matches a live line, so a stale one fails.
    for (file, needle) in ALLOWED {
        assert!(
            lines.iter().any(|(path, _, text)| path == file && text.contains(needle)),
            "stale allowance {file} {needle}"
        );
    }
}
