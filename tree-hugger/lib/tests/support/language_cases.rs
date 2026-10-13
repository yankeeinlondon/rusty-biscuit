//! Named language cases shared by the library's unit and integration tests.

macro_rules! language_tests {
    ($consumer:ident => $check:ident) => {
        mod $consumer {
            use super::*;
            $crate::language_cases::language_tests!(@cases $check;
                rust => Rust,
                typescript => TypeScript,
                javascript => JavaScript,
                go => Go,
                python => Python,
                java => Java,
                csharp => CSharp,
                c => C,
                cpp => Cpp,
                swift => Swift,
                scala => Scala,
                php => Php,
                perl => Perl,
                bash => Bash,
                zsh => Zsh,
                lua => Lua,
            );
        }
    };
    (@cases $check:ident; $($name:ident => $language:ident),+ $(,)?) => {
        $(#[test] fn $name() { super::$check(ProgrammingLanguage::$language); })+
        #[test]
        fn every_language_has_a_case() {
            let mut registered = vec![$(ProgrammingLanguage::$language),+];
            let mut supported = ProgrammingLanguage::all();
            registered.sort_by_key(|language| language.query_name());
            supported.sort_by_key(|language| language.query_name());
            assert_eq!(registered, supported);
        }
    };
}
pub(crate) use language_tests;

use crate::ProgrammingLanguage;

/// Per-language corpus floors retain the aggregate non-vacuity checks after
/// the exhaustive comparisons are scheduled as independent nextest cases.
#[derive(Clone, Copy)]
pub(crate) struct CorpusMinimums {
    pub symbols: usize,
    pub metadata: usize,
    pub files: usize,
    pub imports: usize,
    pub lookups: usize,
    pub containers: usize,
    pub matched_containers: usize,
    pub exports: usize,
    pub captures: usize,
    pub parents: usize,
    pub fields: usize,
    pub signatures: usize,
    pub members: usize,
    pub metadata_fields: usize,
    pub deferred: usize,
}

const CORPUS_MINIMUMS: &[(ProgrammingLanguage, CorpusMinimums)] = &[
    (ProgrammingLanguage::Rust, CorpusMinimums { symbols: 80, metadata: 8, files: 17, imports: 31, lookups: 141, containers: 51, matched_containers: 44, exports: 27, captures: 9, parents: 12, fields: 31, signatures: 80, members: 9, metadata_fields: 9, deferred: 0 }),
    (ProgrammingLanguage::TypeScript, CorpusMinimums { symbols: 69, metadata: 5, files: 24, imports: 18, lookups: 117, containers: 36, matched_containers: 28, exports: 15, captures: 10, parents: 15, fields: 29, signatures: 69, members: 8, metadata_fields: 8, deferred: 9 }),
    (ProgrammingLanguage::JavaScript, CorpusMinimums { symbols: 48, metadata: 0, files: 23, imports: 6, lookups: 86, containers: 16, matched_containers: 14, exports: 9, captures: 5, parents: 9, fields: 11, signatures: 48, members: 3, metadata_fields: 3, deferred: 0 }),
    (ProgrammingLanguage::Go, CorpusMinimums { symbols: 34, metadata: 1, files: 30, imports: 31, lookups: 80, containers: 8, matched_containers: 6, exports: 0, captures: 6, parents: 5, fields: 9, signatures: 34, members: 2, metadata_fields: 2, deferred: 12 }),
    (ProgrammingLanguage::Python, CorpusMinimums { symbols: 45, metadata: 1, files: 38, imports: 47, lookups: 102, containers: 24, matched_containers: 19, exports: 0, captures: 5, parents: 6, fields: 15, signatures: 45, members: 6, metadata_fields: 6, deferred: 3 }),
    (ProgrammingLanguage::Java, CorpusMinimums { symbols: 48, metadata: 3, files: 4, imports: 4, lookups: 84, containers: 43, matched_containers: 39, exports: 0, captures: 9, parents: 24, fields: 24, signatures: 48, members: 6, metadata_fields: 6, deferred: 8 }),
    (ProgrammingLanguage::CSharp, CorpusMinimums { symbols: 59, metadata: 7, files: 10, imports: 18, lookups: 86, containers: 64, matched_containers: 58, exports: 0, captures: 9, parents: 24, fields: 45, signatures: 59, members: 13, metadata_fields: 13, deferred: 6 }),
    (ProgrammingLanguage::C, CorpusMinimums { symbols: 32, metadata: 6, files: 3, imports: 0, lookups: 48, containers: 7, matched_containers: 5, exports: 0, captures: 7, parents: 0, fields: 0, signatures: 30, members: 4, metadata_fields: 5, deferred: 6 }),
    (ProgrammingLanguage::Cpp, CorpusMinimums { symbols: 61, metadata: 4, files: 10, imports: 0, lookups: 88, containers: 40, matched_containers: 33, exports: 0, captures: 9, parents: 9, fields: 36, signatures: 61, members: 10, metadata_fields: 10, deferred: 18 }),
    (ProgrammingLanguage::Swift, CorpusMinimums { symbols: 55, metadata: 4, files: 13, imports: 26, lookups: 92, containers: 33, matched_containers: 30, exports: 0, captures: 7, parents: 17, fields: 33, signatures: 54, members: 10, metadata_fields: 10, deferred: 8 }),
    (ProgrammingLanguage::Scala, CorpusMinimums { symbols: 42, metadata: 2, files: 16, imports: 20, lookups: 102, containers: 24, matched_containers: 11, exports: 0, captures: 7, parents: 15, fields: 11, signatures: 42, members: 3, metadata_fields: 3, deferred: 30 }),
    (ProgrammingLanguage::Php, CorpusMinimums { symbols: 38, metadata: 4, files: 4, imports: 3, lookups: 60, containers: 22, matched_containers: 19, exports: 0, captures: 9, parents: 9, fields: 22, signatures: 38, members: 6, metadata_fields: 4, deferred: 6 }),
    (ProgrammingLanguage::Perl, CorpusMinimums { symbols: 15, metadata: 0, files: 28, imports: 0, lookups: 36, containers: 17, matched_containers: 8, exports: 0, captures: 2, parents: 0, fields: 0, signatures: 15, members: 0, metadata_fields: 0, deferred: 0 }),
    (ProgrammingLanguage::Bash, CorpusMinimums { symbols: 8, metadata: 0, files: 2, imports: 0, lookups: 24, containers: 0, matched_containers: 0, exports: 0, captures: 1, parents: 0, fields: 0, signatures: 8, members: 0, metadata_fields: 0, deferred: 0 }),
    (ProgrammingLanguage::Zsh, CorpusMinimums { symbols: 8, metadata: 0, files: 2, imports: 0, lookups: 27, containers: 0, matched_containers: 0, exports: 0, captures: 1, parents: 0, fields: 0, signatures: 8, members: 0, metadata_fields: 0, deferred: 0 }),
    (ProgrammingLanguage::Lua, CorpusMinimums { symbols: 18, metadata: 0, files: 32, imports: 0, lookups: 39, containers: 5, matched_containers: 0, exports: 0, captures: 3, parents: 2, fields: 0, signatures: 18, members: 0, metadata_fields: 0, deferred: 0 }),
];

pub(crate) fn corpus_minimums(language: ProgrammingLanguage) -> CorpusMinimums {
    CORPUS_MINIMUMS.iter().find(|row| row.0 == language).expect("registered corpus language").1
}

#[test]
fn corpus_floors_preserve_the_original_aggregate_checks() {
    let sum = |field: fn(&CorpusMinimums) -> usize| CORPUS_MINIMUMS.iter().map(|row| field(&row.1)).sum::<usize>();
    assert!(sum(|row| row.symbols) > 500);
    assert!(2 * sum(|row| row.symbols) > 800);
    assert!(sum(|row| row.metadata) > 20);
    assert!(sum(|row| row.files) > 50 && sum(|row| row.imports) > 50);
    assert!(sum(|row| row.lookups) > 1000 && sum(|row| row.containers) > 300 && sum(|row| row.exports) > 30);
    assert!(sum(|row| row.captures) > 50);
    assert!(sum(|row| row.matched_containers) > 100);
    assert!(sum(|row| row.parents) > 20 && sum(|row| row.fields) > 20);
    assert!(sum(|row| row.signatures) > 500);
    assert!(sum(|row| row.members) > 40);
    assert!(sum(|row| row.metadata_fields) > 50);
    assert!(sum(|row| row.deferred) > 50);
    let mut registered: Vec<_> = CORPUS_MINIMUMS.iter().map(|row| row.0).collect();
    let mut supported = ProgrammingLanguage::all();
    registered.sort_by_key(|language| language.query_name());
    supported.sort_by_key(|language| language.query_name());
    assert_eq!(registered, supported);
}
