use darkmatter::markdown::Markdown;

#[test]
fn test_ternary_content_interpolation() {
    let content = "---\nactive: true\n---\nStatus: {{ active ? 'Online' : 'Offline' }}";
    let md: Markdown = content.into();
    let (composed, _) = md.compose().unwrap();
    assert_eq!(composed.content().trim(), "Status: Online");

    let content_false = "---\nactive: false\n---\nStatus: {{ active ? 'Online' : 'Offline' }}";
    let md_false: Markdown = content_false.into();
    let (composed_false, _) = md_false.compose().unwrap();
    assert_eq!(composed_false.content().trim(), "Status: Offline");
}

#[test]
fn test_recursive_ternary_content_interpolation() {
    let content =
        "---\na: true\nb: false\n---\nResult: {{ a ? b ? 'both' : 'only a' : 'neither' }}";
    let md: Markdown = content.into();
    let (composed, _) = md.compose().unwrap();
    assert_eq!(composed.content().trim(), "Result: only a");
}

#[test]
fn test_frontmatter_ternary_interpolation() {
    let content = "---\nmode: production\nurl: \"{{ mode == 'production' ? 'https://api.com' : 'http://localhost' }}\"\n---\nURL: {{url}}";
    let md: Markdown = content.into();
    let (composed, _) = md.compose().unwrap();
    assert_eq!(composed.content().trim(), "URL: https://api.com");
}

#[test]
fn test_page_block_with_ternary_condition() {
    // Ternary in 'when' condition of page block using boolean literals
    let content =
        "---\nenabled: true\n---\n::block when=\"enabled ? true : false\"\nVISIBLE\n::end-block";
    let md: Markdown = content.into();
    let (composed, _) = md.compose().unwrap();
    assert!(composed.content().contains("VISIBLE"));

    let content_false =
        "---\nenabled: false\n---\n::block when=\"enabled ? true : false\"\nVISIBLE\n::end-block";
    let md_false: Markdown = content_false.into();
    let (composed_false, _) = md_false.compose().unwrap();
    assert!(!composed_false.content().contains("VISIBLE"));
}

#[test]
fn test_ternary_with_parentheses_grouping() {
    let content = "---\na: true\nb: true\n---\n{{ a ? (b ? 'YES' : 'NO') : 'OUTER' }}";
    let md: Markdown = content.into();
    let (composed, _) = md.compose().unwrap();
    assert_eq!(composed.content().trim(), "YES");
}

#[test]
fn test_ternary_with_complex_comparison() {
    let content = "---\ncount: 5\n---\n{{ count > 0 ? 'HAS' : 'EMPTY' }}";
    let md: Markdown = content.into();
    let (composed, _) = md.compose().unwrap();
    assert_eq!(composed.content().trim(), "HAS");
}

#[test]
fn test_ternary_branch_with_nested_interpolation() {
    // Spec example: a selected branch that itself contains interpolation.
    let content = "---\npkg: darkmatter\n---\n{{ pkg ? 'in a package directory: {{pkg}}' : 'not in a package directory' }}";
    let md: Markdown = content.into();
    let (composed, _) = md.compose().unwrap();
    assert_eq!(
        composed.content().trim(),
        "in a package directory: darkmatter"
    );
}

#[test]
fn test_frontmatter_ternary_with_nested_interpolation() {
    // Frontmatter ternary where the selected branch contains interpolation.
    let content = "---\npkg: darkmatter\nmessage: \"{{ pkg ? 'Package: {{pkg}}' : 'No package' }}\"\n---\n{{message}}";
    let md: Markdown = content.into();
    let (composed, _) = md.compose().unwrap();
    assert_eq!(composed.content().trim(), "Package: darkmatter");
}

#[test]
fn test_false_branch_with_nested_interpolation() {
    let content =
        "---\npkg: null\nfallback: none\n---\n{{ pkg ? 'has: {{pkg}}' : 'missing: {{fallback}}' }}";
    let md: Markdown = content.into();
    let (composed, _) = md.compose().unwrap();
    assert_eq!(composed.content().trim(), "missing: none");
}

#[test]
fn test_frontmatter_ternary_dependency_ordering() {
    // The ternary in `message` references `spec`, which is itself templated and
    // declared after `message`. The frontmatter dependency scanner must look
    // through the ternary AST to discover `spec` as a dependency, otherwise
    // `message` would resolve before `spec` and produce an empty branch value.
    let content = "---\nuse: true\nmessage: \"{{ use ? spec : 'none' }}\"\nbase: \"prefix\"\nspec: \"{{ base }}/spec.md\"\n---\n{{message}}";
    let md: Markdown = content.into();
    let (composed, _) = md.compose().unwrap();
    assert_eq!(composed.content().trim(), "prefix/spec.md");
}

#[test]
fn test_frontmatter_fallback_dependency_ordering() {
    // A fallback expression must declare `area` as a dependency even though it
    // appears alongside a literal default.
    let content = "---\nlabel: \"{{ area || 'unknown' }}\"\nbase: \"core\"\narea: \"{{ base }}-area\"\n---\n{{label}}";
    let md: Markdown = content.into();
    let (composed, _) = md.compose().unwrap();
    assert_eq!(composed.content().trim(), "core-area");
}
