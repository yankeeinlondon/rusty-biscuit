use super::model::{
    ClassificationConfidence, ClassificationSource, FrameworkKind, ProgrammingLanguage,
};
use std::path::Path;

fn file_contains_any(content: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| content.contains(needle))
}

pub fn related_languages(
    framework: FrameworkKind,
    path: &Path,
) -> (Vec<ProgrammingLanguage>, ClassificationConfidence, ClassificationSource) {
    let content = std::fs::read_to_string(path).unwrap_or_default();

    let explicit_ts = file_contains_any(
        &content,
        &[
            r#"lang="ts""#,
            r#"lang='ts'"#,
            r#"lang="tsx""#,
            r#"lang='tsx'"#,
        ],
    );

    let related_languages = match framework {
        FrameworkKind::Vue | FrameworkKind::Svelte | FrameworkKind::Astro => {
            if explicit_ts {
                vec![ProgrammingLanguage::TypeScript]
            } else {
                vec![ProgrammingLanguage::JavaScript]
            }
        }
        FrameworkKind::AngularTemplate => vec![ProgrammingLanguage::TypeScript],
        FrameworkKind::RemixRouteModule | FrameworkKind::NextAppRouter => {
            vec![ProgrammingLanguage::TypeScript]
        }
        FrameworkKind::Unknown => Vec::new(),
    };

    let confidence = if explicit_ts {
        ClassificationConfidence::High
    } else {
        ClassificationConfidence::Medium
    };
    let source = if explicit_ts {
        ClassificationSource::EmbeddedLanguageHint
    } else {
        ClassificationSource::Extension
    };

    (related_languages, confidence, source)
}
