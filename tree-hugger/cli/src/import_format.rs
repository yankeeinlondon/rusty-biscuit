//! Import grouping and per-language import-statement formatting.

use std::collections::{BTreeMap, HashSet};

use tree_hugger::{ImportSymbol, ProgrammingLanguage};


pub(crate) fn group_imports(imports: &[ImportSymbol]) -> Vec<Vec<&ImportSymbol>> {
    let mut groups: BTreeMap<(usize, usize), Vec<&ImportSymbol>> = BTreeMap::new();
    for import in imports {
        let key = import
            .statement_range
            .as_ref()
            .map(|range| (range.start_line, range.start_column))
            .unwrap_or((import.range.start_line, import.range.start_column));
        groups.entry(key).or_default().push(import);
    }

    let mut result = Vec::new();
    for (_, mut group) in groups {
        group.sort_by_key(|import| (import.range.start_line, import.range.start_column));
        result.push(group);
    }

    result
}

pub(crate) fn dedupe_import_group<'a>(imports: &'a [&'a ImportSymbol]) -> Vec<&'a ImportSymbol> {
    let mut alias_originals = HashSet::new();
    for import in imports {
        if import.alias.is_some()
            && let Some(original) = import.original_name.as_deref()
        {
            alias_originals.insert((import.source.as_deref(), original));
        }
    }

    let mut result = Vec::new();
    for import in imports {
        let is_alias_shadow = import.alias.is_none()
            && import.original_name.is_none()
            && alias_originals.contains(&(import.source.as_deref(), import.name.as_str()));
        if is_alias_shadow {
            continue;
        }
        result.push(*import);
    }

    result
}

pub(crate) fn format_import_locations(imports: &[&ImportSymbol]) -> (String, usize) {
    let mut positions: Vec<(usize, usize)> = imports
        .iter()
        .map(|import| (import.range.start_line, import.range.start_column))
        .collect();
    positions.sort();

    let (first_line, _) = positions.first().copied().unwrap_or((1, 1));
    let location = if positions.iter().all(|(line, _)| *line == first_line) {
        let columns = positions
            .iter()
            .map(|(_, column)| column.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        format!("[{}:{}]", first_line, columns)
    } else {
        let entries = positions
            .iter()
            .map(|(line, column)| format!("{}:{}", line, column))
            .collect::<Vec<_>>()
            .join(", ");
        format!("[{}]", entries)
    };

    (location, first_line)
}

pub(crate) fn format_import_group_display(imports: &[&ImportSymbol]) -> String {
    let language = imports
        .first()
        .map(|import| import.language)
        .unwrap_or(ProgrammingLanguage::Rust);
    match language {
        ProgrammingLanguage::JavaScript | ProgrammingLanguage::TypeScript => {
            format_ecma_import_group(imports)
        }
        ProgrammingLanguage::Python => format_python_import_group(imports),
        ProgrammingLanguage::Rust => format_rust_import_group(imports),
        ProgrammingLanguage::Go => format_go_import_group(imports),
        ProgrammingLanguage::Java => format_java_import_group(imports),
        ProgrammingLanguage::CSharp => format_csharp_import_group(imports),
        ProgrammingLanguage::Php => format_php_import_group(imports),
        ProgrammingLanguage::Scala => format_scala_import_group(imports),
        ProgrammingLanguage::Swift => format_swift_import_group(imports),
        _ => format_generic_import_group(imports),
    }
}

pub(crate) fn format_ecma_import_group(imports: &[&ImportSymbol]) -> String {
    let source = imports.first().and_then(|import| import.source.as_deref());
    let is_namespace = imports.len() == 1
        && imports[0]
            .original_name
            .as_deref()
            .is_some_and(|name| name == "*");

    if is_namespace {
        let alias = &imports[0].name;
        if let Some(source) = source {
            return format!("import * as {} from \"{}\"", alias, source);
        }
        return format!("import * as {}", alias);
    }

    let specs = imports
        .iter()
        .map(|import| {
            if let Some(alias) = import.alias.as_deref() {
                let original = import.original_name.as_deref().unwrap_or(&import.name);
                format!("{} as {}", original, alias)
            } else {
                import.name.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(", ");

    if let Some(source) = source {
        format!("import {{ {} }} from \"{}\"", specs, source)
    } else {
        format!("import {{ {} }}", specs)
    }
}

pub(crate) fn format_python_import_group(imports: &[&ImportSymbol]) -> String {
    let sources: HashSet<&str> = imports
        .iter()
        .filter_map(|import| import.source.as_deref())
        .collect();

    let specs = |import: &ImportSymbol| {
        if let Some(alias) = import.alias.as_deref() {
            let original = import.original_name.as_deref().unwrap_or(&import.name);
            format!("{} as {}", original, alias)
        } else {
            import.name.clone()
        }
    };

    if sources.len() == 1 {
        let source = *sources.iter().next().unwrap();
        let is_import_stmt = imports.iter().all(|import| {
            import.source.as_deref() == Some(source)
                && (import.name == source || import.original_name.as_deref() == Some(source))
        });
        let spec_list = imports
            .iter()
            .map(|import| specs(import))
            .collect::<Vec<_>>()
            .join(", ");
        if is_import_stmt {
            format!("import {}", spec_list)
        } else {
            format!("from {} import {}", source, spec_list)
        }
    } else {
        let spec_list = imports
            .iter()
            .map(|import| specs(import))
            .collect::<Vec<_>>()
            .join(", ");
        format!("import {}", spec_list)
    }
}

pub(crate) fn format_rust_import_group(imports: &[&ImportSymbol]) -> String {
    let source = imports.first().and_then(|import| import.source.as_deref());
    let specs = imports
        .iter()
        .map(|import| {
            if let Some(alias) = import.alias.as_deref() {
                let original = import.original_name.as_deref().unwrap_or(&import.name);
                let stripped = source
                    .and_then(|src| original.strip_prefix(&format!("{}::", src)))
                    .unwrap_or(original);
                format!("{} as {}", stripped, alias)
            } else {
                import.name.clone()
            }
        })
        .collect::<Vec<_>>();

    if let Some(source) = source {
        if specs.len() == 1 {
            let spec = &specs[0];
            if spec.contains("::") {
                format!("use {}", spec)
            } else {
                format!("use {}::{}", source, spec)
            }
        } else {
            format!("use {}::{{{}}}", source, specs.join(", "))
        }
    } else {
        format!("use {}", specs.join(", "))
    }
}

pub(crate) fn format_go_import_group(imports: &[&ImportSymbol]) -> String {
    let specs = imports
        .iter()
        .map(|import| {
            let path = import.source.as_deref().unwrap_or(&import.name);
            let quoted = format!("\"{}\"", path);
            if let Some(alias) = import.alias.as_deref() {
                format!("{} {}", alias, quoted)
            } else {
                quoted
            }
        })
        .collect::<Vec<_>>()
        .join(", ");

    format!("import {}", specs)
}

pub(crate) fn format_java_import_group(imports: &[&ImportSymbol]) -> String {
    let specs = imports
        .iter()
        .map(|import| {
            if import.original_name.as_deref() == Some("*") {
                if let Some(source) = import.source.as_deref() {
                    format!("{}.*", source)
                } else {
                    "*".to_string()
                }
            } else if let Some(source) = import.source.as_deref() {
                format!("{}.{}", source, import.name)
            } else {
                import.name.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(", ");

    format!("import {}", specs)
}

pub(crate) fn format_csharp_import_group(imports: &[&ImportSymbol]) -> String {
    let specs = imports
        .iter()
        .map(|import| import.source.as_deref().unwrap_or(&import.name).to_string())
        .collect::<Vec<_>>()
        .join(", ");

    format!("using {}", specs)
}

pub(crate) fn format_php_import_group(imports: &[&ImportSymbol]) -> String {
    let specs = imports
        .iter()
        .map(|import| {
            let base = import.source.as_deref().unwrap_or(&import.name);
            if let Some(alias) = import.alias.as_deref() {
                format!("{} as {}", base, alias)
            } else {
                base.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(", ");

    format!("use {}", specs)
}

pub(crate) fn format_scala_import_group(imports: &[&ImportSymbol]) -> String {
    if let Some(source) = imports.first().and_then(|import| import.source.as_deref()) {
        return format!("import {}", source);
    }

    let specs = imports
        .iter()
        .map(|import| {
            if let Some(alias) = import.alias.as_deref() {
                let original = import.original_name.as_deref().unwrap_or(&import.name);
                format!("{} => {}", original, alias)
            } else {
                import.name.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(", ");

    format!("import {}", specs)
}

pub(crate) fn format_swift_import_group(imports: &[&ImportSymbol]) -> String {
    let specs = imports
        .iter()
        .map(|import| import.source.as_deref().unwrap_or(&import.name).to_string())
        .collect::<Vec<_>>()
        .join(", ");

    format!("import {}", specs)
}

pub(crate) fn format_generic_import_group(imports: &[&ImportSymbol]) -> String {
    let specs = imports
        .iter()
        .map(|import| import.name.clone())
        .collect::<Vec<_>>()
        .join(", ");
    format!("import {}", specs)
}
