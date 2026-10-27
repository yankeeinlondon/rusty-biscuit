//! The generated crate's documentation is honest (R38–R41, R51).
//!
//! The fast tests generate the shipped APIs into a temporary directory
//! through the public generation entry point and read the doc text of the
//! output: every code fence is a `no_run` doctest, no doc line is a stray
//! `;`, every `client.<method>(` an example writes exists on that module's
//! client, and a module shared by several APIs documents each of them. The
//! `slow_` test compiles those doctests in a fresh crate with the default
//! features and with one module selected.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use schematic_define::RestApi;
use schematic_definitions::apis_by_module;
use schematic_gen::output::generate_and_write_all;
use schematic_gen::pipeline::resolve_all_apis;

use crate::common::fresh_output::{Definitions, FreshCrate};

/// The methods `schematic_runtime::api_client!` gives every client.
const API_CLIENT_SOURCE: &str = include_str!("../../../runtime/src/api_client.rs");

/// Generates `apis` into a temporary `src` directory and returns every
/// generated file's source by its path relative to that directory.
fn generate(apis: &[&RestApi]) -> BTreeMap<PathBuf, String> {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let src = temp.path().join("src");
    generate_and_write_all(apis, &src, false).expect("generate");
    let mut files = BTreeMap::new();
    collect(&src, &src, &mut files);
    files
}

fn collect(root: &Path, dir: &Path, files: &mut BTreeMap<PathBuf, String>) {
    for entry in fs::read_dir(dir).expect("read dir") {
        let path = entry.expect("entry").path();
        if path.is_dir() {
            collect(root, &path, files);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            let source = fs::read_to_string(&path).expect("read source");
            files.insert(path.strip_prefix(root).unwrap().to_path_buf(), source);
        }
    }
}

fn generate_shipped() -> BTreeMap<PathBuf, String> {
    let apis = resolve_all_apis();
    generate(&apis.iter().collect::<Vec<_>>())
}

/// The doc text of each doc line of `source`, with its 1-based line number:
/// a `///` or `//!` comment, or the string of a `doc = "…"` attribute
/// (the crate root and prelude gate their doc lines on features).
fn doc_lines(source: &str) -> Vec<(usize, String)> {
    source
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let trimmed = line.trim_start();
            let text = if let Some(rest) = trimmed.strip_prefix("///").or_else(|| trimmed.strip_prefix("//!")) {
                rest.to_string()
            } else {
                let (_, rest) = trimmed.split_once("doc = \"")?;
                rest.strip_suffix("\")]").or_else(|| rest.strip_suffix('"'))?.replace("\\\"", "\"")
            };
            Some((index + 1, text))
        })
        .collect()
}

/// The method names `client.<name>(` calls in `docs`.
fn client_calls(docs: &str) -> BTreeSet<String> {
    docs.match_indices("client.")
        .filter_map(|(at, _)| {
            let rest = &docs[at + "client.".len()..];
            let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            rest[name.len()..].starts_with('(').then_some(name)
        })
        .filter(|name| !name.is_empty())
        .collect()
}

/// The names of the `pub fn`s in `source`.
fn pub_fns(source: &str) -> BTreeSet<String> {
    source
        .match_indices("pub fn ")
        .map(|(at, _)| {
            source[at + "pub fn ".len()..].chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect()
        })
        .collect()
}

/// The top-level module a generated file belongs to: `emqx/requests.rs`
/// and `emqx/mod.rs` are `emqx`, `anthropic.rs` is `anthropic`.
fn module_of(path: &Path) -> String {
    let first = path.components().next().unwrap().as_os_str().to_string_lossy().to_string();
    first.trim_end_matches(".rs").to_string()
}

/// One generation of the shipped APIs serves the three crate-wide checks:
/// generating them takes about two seconds.
#[test]
fn the_shipped_crate_docs_hold_only_compilable_examples_of_existing_methods() {
    let files = generate_shipped();

    // R38: every code fence is a `no_run` doctest.
    let mut fences = 0;
    for (path, source) in &files {
        let mut open = false;
        for (line, text) in doc_lines(source) {
            let Some(info) = text.trim().strip_prefix("```") else { continue };
            if !open {
                assert_eq!(info, "no_run", "{}:{line}: a generated example must be a no_run doctest", path.display());
                fences += 1;
            }
            open = !open;
        }
        assert!(!open, "{}: unclosed code fence", path.display());
    }
    // Every request type, module, crate root and prelude carries one.
    assert!(fences > 500, "only {fences} fences found; the scan is not reading the doc lines");

    // R41: no doc line is a stray `;`.
    for (path, source) in &files {
        for (line, text) in doc_lines(source) {
            assert_ne!(text.trim(), ";", "{}:{line}: a doc line holding only `;`", path.display());
        }
        assert!(!source.contains("///;"), "{}: holds `///;`", path.display());
    }

    // R39: every `client.<method>(` names a method of that module's client
    // or one `api_client!` gives every client.
    let client_methods = pub_fns(API_CLIENT_SOURCE);
    assert!(client_methods.contains("send") && client_methods.contains("base_url"), "{client_methods:?}");
    let mut module_fns: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (path, source) in &files {
        module_fns.entry(module_of(path)).or_default().extend(pub_fns(source));
    }
    let every_fn: BTreeSet<String> = module_fns.values().flatten().cloned().collect();
    let mut checked = 0;
    for (path, source) in &files {
        let module = module_of(path);
        // The crate root and prelude show one API's client.
        let available = match module.as_str() {
            "lib" | "prelude" => &every_fn,
            _ => &module_fns[&module],
        };
        let docs: String = doc_lines(source).into_iter().map(|(_, text)| text + "\n").collect();
        for method in client_calls(&docs) {
            assert!(
                available.contains(&method) || client_methods.contains(&method),
                "{}: a doc example calls `client.{method}(..)`, which the client does not have",
                path.display()
            );
            checked += 1;
        }
    }
    assert!(checked > 30, "only {checked} calls found; the scan is not reading the examples");
}

#[test]
fn the_client_call_scan_reports_a_method_the_module_lacks() {
    let docs = "let data = client.list_models().await?;\nlet data = client.send(ListModelsRequest::default()).await?;";
    assert_eq!(client_calls(docs), BTreeSet::from(["list_models".to_string(), "send".to_string()]));
    let module = pub_fns("pub fn send(&self) {}\npub fn retrieve_model(&self) {}");
    assert!(!module.contains("list_models"));
}

/// Each combined module's docs name every member API and hold its own
/// module's sections, one heading level deeper (R51; emqx is the reported
/// case, ollama the other shipped one).
#[test]
fn a_shared_module_documents_every_member_api() {
    let groups = apis_by_module();
    let combined: Vec<(&String, &Vec<RestApi>)> = groups.iter().filter(|(_, apis)| apis.len() > 1).collect();
    assert!(combined.iter().any(|(module, _)| module.as_str() == "emqx"), "emqx shares its module");

    for (module, apis) in combined {
        let files = generate(&apis.iter().collect::<Vec<_>>());
        let mod_rs = files
            .get(&PathBuf::from(module).join("mod.rs"))
            .or_else(|| files.get(&PathBuf::from(format!("{module}.rs"))))
            .unwrap_or_else(|| panic!("no module file for {module}"));
        let module_docs: String = mod_rs
            .lines()
            .filter_map(|line| line.trim_start().strip_prefix("//!"))
            .map(|text| format!("{}\n", text.strip_prefix(' ').unwrap_or(text)))
            .collect();

        // Each member's section runs from its `## Name` heading to the next.
        let mut sections = Vec::new();
        for (index, api) in apis.iter().enumerate() {
            let heading = format!("\n## {}\n", api.name);
            let start = module_docs
                .find(&heading)
                .unwrap_or_else(|| panic!("{module}: no section for {}\n{module_docs}", api.name));
            let end = apis
                .get(index + 1)
                .and_then(|next| module_docs.find(&format!("\n## {}\n", next.name)))
                .unwrap_or(module_docs.len());
            sections.push((api, &module_docs[start..end]));
        }
        assert!(module_docs.starts_with("Generated API clients for "), "{module}: summary first\n{module_docs}");
        for (api, section) in sections {
            for heading in ["### Authentication", "### Features", "### Examples"] {
                assert!(section.contains(heading), "{module}/{}: no `{heading}`\n{section}", api.name);
            }
            assert!(section.contains(&format!("let client = {}::new();", api.name)), "{module}/{}\n{section}", api.name);
            for endpoint in &api.endpoints {
                assert!(
                    section.contains(&format!("- `{}` - ", endpoint.id)),
                    "{module}/{}: endpoint `{}` not listed\n{section}",
                    api.name,
                    endpoint.id
                );
            }
        }
        if module == "emqx" {
            let basic = &module_docs[module_docs.find("\n## EmqxBasic\n").unwrap()..module_docs.find("\n## EmqxBearer\n").unwrap()];
            let bearer = &module_docs[module_docs.find("\n## EmqxBearer\n").unwrap()..];
            assert!(basic.contains("`EMQX_API_KEY`") && basic.contains("`EMQX_API_SECRET`"), "{basic}");
            assert!(bearer.contains("`EMQX_TOKEN`") && bearer.contains("- `Login` - "), "{bearer}");
        }
    }
}

#[test]
fn slow_generated_doc_examples_compile_with_default_and_selected_features() {
    let lockfile = biscuit_test_harness::manifest_dir!().join("../schema/Cargo.lock");
    let apis = resolve_all_apis();
    let api_refs: Vec<&RestApi> = apis.iter().collect();
    let fresh = FreshCrate::generate(&api_refs, Definitions::Checkout, &lockfile);

    // `emqx` is a shared module and not the crate-level example's API, so the
    // gated crate and prelude examples must drop out with its feature alone.
    for features in [&[][..], &["--no-default-features", "--features", "emqx"][..]] {
        let mut args = vec!["test", "--doc"];
        args.extend(features);
        let run = fresh.cargo_offline(&args);
        run.assert_success();
        let passed = run
            .stdout
            .lines()
            .find_map(|line| line.strip_prefix("test result: ok. ")?.split(' ').next()?.parse::<usize>().ok())
            .unwrap_or_else(|| panic!("{features:?}: no doctest summary\n{}", run.stdout));
        assert!(passed > 0, "{features:?}: no doctest ran\n{}", run.stdout);
    }
}
