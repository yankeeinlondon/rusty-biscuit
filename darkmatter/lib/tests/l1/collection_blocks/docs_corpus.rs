//! Passive shipped-artifact corpus: the directive examples in the shipped
//! docs parse through the public scanners, and the stable fixtures under
//! `tests/fixtures/collection_blocks/` execute.
//!
//! The docs are prose first. Only fenced `md` blocks with an authored
//! `::map` / `::filter` opener at the start of a line are examples; a block
//! whose opener holds a `<placeholder>` is a template and is skipped.

use darkmatter::markdown::compose::collection_blocks::{parse_collection_regions, scan_collection_regions_lenient};

/// One fenced `md` block that carries at least one collection opener.
struct Example {
    /// 1-based line of the opening fence.
    line: usize,
    text: String,
    /// The nearest prose line above the fence calls the block mismatched,
    /// invalid, or malformed, so it documents a rejection.
    negative: bool,
}

/// De-indented contents of every fenced `md`/`markdown` block that has an
/// authored collection opener line.
fn examples(doc: &str) -> Vec<Example> {
    let lines: Vec<&str> = doc.lines().collect();
    let mut found = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        let ticks = trimmed.chars().take_while(|&c| c == '`').count();
        let info = trimmed[ticks.min(trimmed.len())..].trim();
        if ticks >= 3 && matches!(info, "md" | "markdown") {
            let start = index + 1;
            let mut end = start;
            while end < lines.len() {
                let candidate = lines[end].trim_start();
                if candidate.starts_with(&"`".repeat(ticks)) && candidate.trim_start_matches('`').is_empty() {
                    break;
                }
                end += 1;
            }
            let body: Vec<&str> = lines[start..end.min(lines.len())]
                .iter()
                .map(|content| content.get(indent..).unwrap_or(content.trim_start()))
                .collect();
            let text = body.join("\n") + "\n";
            let has_opener = body.iter().any(|content| content.starts_with("::map ") || content.starts_with("::filter ") || *content == "::map" || *content == "::filter");
            if has_opener {
                let negative = lines[..index].iter().rev().find(|prose| !prose.trim().is_empty()).is_some_and(|prose| {
                    let prose = prose.to_ascii_lowercase();
                    ["mismatched", "invalid", "malformed"].iter().any(|word| prose.contains(word))
                });
                found.push(Example { line: index + 1, text, negative });
            }
            index = end;
        }
        index += 1;
    }
    found
}

fn is_template(example: &Example) -> bool {
    example
        .text
        .lines()
        .filter(|line| line.starts_with("::map") || line.starts_with("::filter"))
        .any(|line| line.contains('<') || line.contains("..."))
}

/// Parses every complete, non-template example; returns `(parsed, failures)`.
fn parse_all(name: &str, doc: &str) -> (usize, Vec<String>) {
    let mut parsed = 0;
    let mut failures = Vec::new();
    for example in examples(doc) {
        if is_template(&example) {
            continue;
        }
        let scan = scan_collection_regions_lenient(&example.text);
        let unpaired = scan.structural.is_some();
        if unpaired && !example.negative && !example.text.lines().any(|line| line.starts_with("::end-map") || line.starts_with("::end-filter")) {
            // A header-only illustration of an opener has nothing to pair.
            continue;
        }
        let outcome = parse_collection_regions(&example.text, None);
        if example.negative {
            match outcome {
                Err(error) if error.code() == "dm.collection.malformed" => parsed += 1,
                other => failures.push(format!("{name}:{}: a documented rejection parsed as {other:?}", example.line)),
            }
            continue;
        }
        match outcome {
            Ok(regions) if !regions.is_empty() => parsed += 1,
            Ok(_) => failures.push(format!("{name}:{}: no region was found", example.line)),
            Err(error) => failures.push(format!("{name}:{}: {error}", example.line)),
        }
    }
    (parsed, failures)
}

#[test]
fn the_map_doc_examples_parse() {
    let doc = include_str!("../../../../docs/inline/map.md");
    let (parsed, failures) = parse_all("map.md", doc);
    assert!(failures.is_empty(), "{failures:#?}");
    assert!(parsed >= 5, "only {parsed} example(s) parsed; the extractor may have stopped seeing the doc");
}

#[test]
fn the_filter_doc_examples_parse() {
    let doc = include_str!("../../../../docs/inline/filter.md");
    let (parsed, failures) = parse_all("filter.md", doc);
    assert!(failures.is_empty(), "{failures:#?}");
    assert!(parsed >= 5, "only {parsed} example(s) parsed; the extractor may have stopped seeing the doc");
}

#[test]
fn the_iterables_doc_examples_parse() {
    let doc = include_str!("../../../../docs/iterables.md");
    let (parsed, failures) = parse_all("iterables.md", doc);
    assert!(failures.is_empty(), "{failures:#?}");
    assert!(parsed >= 3, "only {parsed} example(s) parsed; the extractor may have stopped seeing the doc");
}

#[test]
fn the_extractor_skips_templates_and_prose_and_reads_indented_fences() {
    let doc = concat!(
        "Prose mentioning ::map data inline is not an example.\n\n",
        "```md\n::map <source> \"<pattern> => <expression>\"\nbody\n::end-map\n```\n\n",
        "- list item\n\n    ```md\n    ::filter items \"i => i\"\n    - {{i}}\n    ::end-filter\n    ```\n\n",
        "```yaml\n::map not-md\n```\n",
    );
    let found = examples(doc);
    assert!(found.iter().all(|example| !example.negative));
    assert_eq!(found.len(), 2, "{:?}", found.iter().map(|e| e.line).collect::<Vec<_>>());
    assert!(is_template(&found[0]) && !is_template(&found[1]));
    assert_eq!(found[1].text, "::filter items \"i => i\"\n- {{i}}\n::end-filter\n");
    let (parsed, failures) = parse_all("inline", doc);
    assert_eq!((parsed, failures.len()), (1, 0));

    let negative = "This has an invalid closer:\n\n```md\n::map a\n::end-filter\n```\n";
    assert!(examples(negative)[0].negative);
    assert_eq!(parse_all("negative", negative), (1, Vec::new()));
    let wrongly_negative = "This is invalid:\n\n```md\n::map a\nx\n::end-map\n```\n";
    assert_eq!(parse_all("negative", wrongly_negative).1.len(), 1, "a documented rejection that parses is a failure");
}

// ── executable fixtures ──────────────────────────────────────────────────

/// Fixture documents under `tests/fixtures/collection_blocks/`, each paired
/// with its expected output.
mod fixtures {
    use super::super::support::composed_raw;

    #[test]
    fn the_map_fixture_composes_to_its_expected_output() {
        let document = include_str!("../../fixtures/collection_blocks/map.md");
        let expected = include_str!("../../fixtures/collection_blocks/map.expected.md");
        assert_eq!(composed_raw(document), expected);
    }

    #[test]
    fn the_filter_fixture_composes_to_its_expected_output() {
        let document = include_str!("../../fixtures/collection_blocks/filter.md");
        let expected = include_str!("../../fixtures/collection_blocks/filter.expected.md");
        assert_eq!(composed_raw(document), expected);
    }
}
