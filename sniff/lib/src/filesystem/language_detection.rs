use rust_code_analysis::{Lang, get_language};
use std::path::Path;

fn detect_primary_languages(path: &Path) -> Vec<Lang> {
    let mut language_counts: std::collections::HashMap<Lang, usize> = std::collections::HashMap::new();

    // Use walkdir to iterate over files
    for entry in walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let file_path = entry.path();
        // Attempt to detect language
        if let Some(lang) = get_language(file_path) {
            *language_counts.entry(lang).or_insert(0) += 1;
        }
    }

    // Sort languages by count (most frequent first)
    let mut sorted_languages: Vec<_> = language_counts.into_iter().collect();
    sorted_languages.sort_by(|a, b| b.1.cmp(&a.1));

    // Return just the language types
    sorted_languages.into_iter().map(|(lang, _)| lang).collect()
}

