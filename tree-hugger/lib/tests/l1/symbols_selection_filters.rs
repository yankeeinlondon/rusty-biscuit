//! The selection-filter matrix of `2026-10-03-symbol-filters-and-reports`:
//! each `SymbolQuery` predicate, run through `SourceFile::query`, keeps
//! exactly the expected declarations of every language's primary fixture,
//! and nothing where the language cannot express the construct.
//!
//! A cell is evaluated over the fixture's "Flags and metrics" block (the
//! declarations after its marker comment, `B`) or, for a construct the block
//! does not hold (async declarations, declared return types, named types),
//! over the whole file (`F`). Fixtures are read from an isolated copy so the
//! Rust fixture's place under `tests/` does not make every declaration a
//! test. Names are in-file qualified names; the query matches a qualified
//! pattern against the displayed name, which carries the `{file}#` prefix
//! unless the name is namespace-rooted.

use std::collections::BTreeSet;

use tempfile::TempDir;
use tree_hugger::{
    NamePattern, ProgrammingLanguage, SourceFile, SymbolKindV2, SymbolQuery, SymbolRecord, SymbolScope,
    VisibilityLevel,
};

use crate::symbols_fixtures::{display_name, in_file_name, primary_fixture};

use ProgrammingLanguage as L;
use VisibilityLevel as V;

/// One `SymbolQuery` predicate.
#[derive(Debug, Clone, Copy)]
pub enum Predicate {
    Scope(SymbolScope),
    Visibility(VisibilityLevel),
    Exported,
    ExcludeTests,
    TestsOnly,
    Undocumented,
    Deprecated,
    Async,
    Returns(&'static str),
    ParamType(&'static str),
    MinParams(u32),
    In(&'static str),
    Name(&'static str),
    ExcludeName(&'static str),
}

impl Predicate {
    const VARIANTS: usize = 14;

    fn variant(self) -> usize {
        match self {
            Self::Scope(_) => 0,
            Self::Visibility(_) => 1,
            Self::Exported => 2,
            Self::ExcludeTests => 3,
            Self::TestsOnly => 4,
            Self::Undocumented => 5,
            Self::Deprecated => 6,
            Self::Async => 7,
            Self::Returns(_) => 8,
            Self::ParamType(_) => 9,
            Self::MinParams(_) => 10,
            Self::In(_) => 11,
            Self::Name(_) => 12,
            Self::ExcludeName(_) => 13,
        }
    }

    fn apply(self, query: SymbolQuery) -> SymbolQuery {
        match self {
            Self::Scope(scope) => query.scopes([scope]),
            Self::Visibility(level) => query.visibility(level),
            Self::Exported => query.exported(true),
            Self::ExcludeTests => query.exclude_tests(true),
            Self::TestsOnly => query.tests_only(true),
            Self::Undocumented => query.undocumented(true),
            Self::Deprecated => query.deprecated(true),
            Self::Async => query.is_async(true),
            Self::Returns(pattern) => query.returns(pattern),
            Self::ParamType(pattern) => query.param_type(pattern),
            Self::MinParams(minimum) => query.min_params(minimum),
            Self::In(pattern) => query.in_container(NamePattern::new(pattern)),
            Self::Name(pattern) => query.name(NamePattern::new(pattern)),
            Self::ExcludeName(pattern) => query.exclude_name(NamePattern::new(pattern)),
        }
    }
}

/// Which declarations of the fixture a cell looks at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Region {
    /// The "Flags and metrics" block.
    B,
    /// The whole file.
    F,
}

/// The region, the predicates (combined with AND) and the in-file names the
/// query keeps, or why the language cannot express the construct (the query
/// then keeps nothing). `Ok(&[])` is a construct the language has but the
/// region does not hold.
pub type Cell = (
    Region,
    &'static [Predicate],
    Result<&'static [&'static str], &'static str>,
);

pub struct SelectionRow {
    pub language: ProgrammingLanguage,
    pub cells: &'static [Cell],
}

use Predicate as P;
use Region::{B, F};
use SymbolScope as S;

const NO_ASYNC: &str = "the language has no async declarations";
const NO_EXPORT: &str = "the language has no export declaration; `is_exported` is never set";
const NO_TEST_MARKER: &str = "only test-file names or test attributes mark tests, and the language has neither in a primary fixture";
const NO_DEPRECATION: &str = "no deprecation source is recognized for the language";
const NO_DOCS: &str = "no doc-comment detection: `has_docs` is None, never undocumented";
const NO_TYPES: &str = "the language declares no type text";
const NO_PARAMETER_COUNT: &str = "parameters are not counted for the language";

pub const SELECTION_FILTER_MATRIX: &[SelectionRow] = &[
    SelectionRow {
        language: L::Rust,
        cells: &[
            (B, &[P::Scope(S::Item)], Ok(&["Receiver", "any_test_helper", "attribute_test", "cfg_test_module", "cfg_test_module::inside_cfg_test", "deprecated_api", "documented_pair", "test_helper", "undocumented_none"])),
            (B, &[P::Scope(S::Member)], Ok(&["Receiver::with_receiver"])),
            (B, &[P::Scope(S::Local)], Ok(&["Receiver::with_receiver::only", "Receiver::with_receiver::self", "cfg_test_module::inside_cfg_test::cfg_flag", "documented_pair::first", "documented_pair::second"])),
            (B, &[P::Visibility(V::Public)], Ok(&["Receiver", "Receiver::with_receiver", "deprecated_api", "documented_pair", "test_helper", "undocumented_none"])),
            // Repeated levels are alternatives.
            (B, &[P::Visibility(V::Private), P::Visibility(V::Crate)], Ok(&["any_test_helper", "attribute_test", "cfg_test_module", "cfg_test_module::inside_cfg_test"])),
            (B, &[P::Exported], Ok(&["Receiver", "deprecated_api", "documented_pair", "test_helper", "undocumented_none"])),
            (B, &[P::TestsOnly], Ok(&["attribute_test", "cfg_test_module", "cfg_test_module::inside_cfg_test", "cfg_test_module::inside_cfg_test::cfg_flag"])),
            // `cfg(any(test, …))` is not test-only code.
            (B, &[P::ExcludeTests, P::Name("test")], Ok(&["any_test_helper", "test_helper"])),
            (B, &[P::Undocumented, P::Name("documented")], Ok(&["undocumented_none"])),
            (B, &[P::Deprecated], Ok(&["deprecated_api"])),
            (F, &[P::Async], Ok(&["async_function", "schema::Record::load"])),
            (F, &[P::Async, P::Returns("Self")], Ok(&["schema::Record::load"])),
            // No function in the block declares `->`: no inferred `()`.
            (B, &[P::Returns("*")], Ok(&[])),
            (F, &[P::Returns("u32")], Ok(&["PublicStruct::instance_method", "PublicStruct::required_trait_method", "PublicTrait::required_trait_method", "boundary_multi_line_header", "function_with_local_static", "private_function"])),
            (B, &[P::ParamType("u8")], Ok(&["Receiver::with_receiver", "documented_pair"])),
            // `self` is the receiver, not a parameter.
            (B, &[P::MinParams(2)], Ok(&["documented_pair"])),
            (B, &[P::In("receiver")], Ok(&["Receiver::with_receiver", "Receiver::with_receiver::only", "Receiver::with_receiver::self"])),
            (B, &[P::Name("documented")], Ok(&["documented_pair", "undocumented_none"])),
            (B, &[P::Name("DOCUMENTED")], Ok(&[])),
            (B, &[P::Name("documented_pair!")], Ok(&["documented_pair"])),
            (B, &[P::Name("documented"), P::ExcludeName("undocumented")], Ok(&["documented_pair"])),
            // A qualified pattern is anchored at the displayed name's start,
            // which is the file prefix.
            (B, &[P::Name("Receiver::*")], Ok(&[])),
            (B, &[P::Name("*Receiver::*")], Ok(&["Receiver::with_receiver", "Receiver::with_receiver::only", "Receiver::with_receiver::self"])),
            (B, &[P::Name("rust.rs#documented_pair*")], Ok(&["documented_pair", "documented_pair::first", "documented_pair::second"])),
        ],
    },
    SelectionRow {
        language: L::TypeScript,
        cells: &[
            (B, &[P::Scope(S::Item)], Ok(&["Receiver", "deprecatedApi", "documentedPair", "testHelper", "undocumentedNone", "variadic"])),
            (B, &[P::Scope(S::Member)], Ok(&["Receiver.withReceiver"])),
            (B, &[P::Scope(S::Local)], Ok(&["Receiver.withReceiver.only", "documentedPair.first", "documentedPair.second", "variadic.first", "variadic.rest"])),
            (B, &[P::Visibility(V::Public)], Ok(&["Receiver.withReceiver", "deprecatedApi", "documentedPair"])),
            (B, &[P::Visibility(V::ModulePrivate)], Ok(&["Receiver", "testHelper", "undocumentedNone", "variadic"])),
            (B, &[P::Exported], Ok(&["deprecatedApi", "documentedPair"])),
            (B, &[P::TestsOnly], Err(NO_TEST_MARKER)),
            (B, &[P::ExcludeTests, P::Name("test")], Ok(&["testHelper"])),
            (B, &[P::Undocumented, P::Name("documented")], Ok(&["undocumentedNone"])),
            (B, &[P::Deprecated], Ok(&["Receiver.withReceiver", "deprecatedApi"])),
            (F, &[P::Async], Ok(&["FirstShape.area", "asyncArrow", "asyncFunction"])),
            (F, &[P::Async, P::Returns("Promise<*")], Ok(&["FirstShape.area", "asyncArrow", "asyncFunction"])),
            (B, &[P::Returns("void")], Ok(&["Receiver.withReceiver", "deprecatedApi", "documentedPair", "testHelper", "undocumentedNone", "variadic"])),
            (B, &[P::ParamType("number")], Ok(&["Receiver.withReceiver", "documentedPair", "variadic"])),
            (B, &[P::ParamType("number[]")], Ok(&["variadic"])),
            (B, &[P::MinParams(2)], Ok(&["documentedPair", "variadic"])),
            (B, &[P::In("receiver")], Ok(&["Receiver.withReceiver", "Receiver.withReceiver.only"])),
            (B, &[P::Name("documented")], Ok(&["documentedPair", "undocumentedNone"])),
            (B, &[P::Name("Documented")], Ok(&[])),
            (B, &[P::Name("documented"), P::ExcludeName("undocumented")], Ok(&["documentedPair"])),
            (B, &[P::Name("*#Receiver.*")], Ok(&["Receiver.withReceiver", "Receiver.withReceiver.only"])),
        ],
    },
    SelectionRow {
        language: L::JavaScript,
        cells: &[
            (B, &[P::Scope(S::Item)], Ok(&["deprecatedApi", "documentedArrow", "documentedPair", "lookAlikeTag", "testHelper", "undocumentedNone", "variadic"])),
            (B, &[P::Scope(S::Member)], Ok(&[])),
            (B, &[P::Scope(S::Local)], Ok(&["documentedArrow.only", "documentedPair.first", "documentedPair.second", "variadic.first", "variadic.rest"])),
            (B, &[P::Visibility(V::ModulePrivate)], Ok(&["deprecatedApi", "documentedArrow", "documentedPair", "lookAlikeTag", "testHelper", "undocumentedNone", "variadic"])),
            (B, &[P::Visibility(V::Unknown)], Ok(&["documentedArrow.only", "documentedPair.first", "documentedPair.second", "variadic.first", "variadic.rest"])),
            (B, &[P::Exported], Ok(&[])),
            (F, &[P::Exported, P::Async], Ok(&["asyncArrow", "asyncFunction"])),
            (B, &[P::TestsOnly], Err(NO_TEST_MARKER)),
            (B, &[P::ExcludeTests, P::Name("test")], Ok(&["testHelper"])),
            (B, &[P::Undocumented, P::Name("documented")], Ok(&["undocumentedNone"])),
            (B, &[P::Deprecated], Ok(&["deprecatedApi"])),
            (F, &[P::Async], Ok(&["FirstShape.area", "asyncArrow", "asyncFunction"])),
            (F, &[P::Returns("*")], Err(NO_TYPES)),
            (F, &[P::ParamType("*")], Err(NO_TYPES)),
            (B, &[P::MinParams(2)], Ok(&["documentedPair", "variadic"])),
            (F, &[P::In("FirstShape")], Ok(&["FirstShape.area"])),
            (B, &[P::Name("documented")], Ok(&["documentedArrow", "documentedPair", "undocumentedNone"])),
            (B, &[P::Name("documented"), P::ExcludeName("undocumented")], Ok(&["documentedArrow", "documentedPair"])),
            (B, &[P::Name("*#documentedArrow*")], Ok(&["documentedArrow", "documentedArrow.only"])),
        ],
    },
    SelectionRow {
        language: L::Go,
        cells: &[
            (B, &[P::Scope(S::Item)], Ok(&["symbols.DeprecatedAPI", "symbols.DeprecatedLater", "symbols.DocumentedPair", "symbols.TestHelper", "symbols.UndocumentedNone", "symbols.Variadic"])),
            (B, &[P::Scope(S::Member)], Ok(&["symbols.ExportedStruct.WithReceiver"])),
            (B, &[P::Scope(S::Local)], Ok(&["symbols.DocumentedPair.first", "symbols.DocumentedPair.second", "symbols.ExportedStruct.WithReceiver.only", "symbols.ExportedStruct.WithReceiver.receiver", "symbols.Variadic.first", "symbols.Variadic.rest"])),
            (B, &[P::Visibility(V::Public)], Ok(&["symbols.DeprecatedAPI", "symbols.DeprecatedLater", "symbols.DocumentedPair", "symbols.ExportedStruct.WithReceiver", "symbols.TestHelper", "symbols.UndocumentedNone", "symbols.Variadic"])),
            (B, &[P::Visibility(V::Package)], Ok(&[])),
            (B, &[P::Exported], Err(NO_EXPORT)),
            (B, &[P::TestsOnly], Err(NO_TEST_MARKER)),
            (B, &[P::ExcludeTests, P::Name("test")], Ok(&["symbols.TestHelper"])),
            (B, &[P::Undocumented, P::Name("documented")], Ok(&["symbols.UndocumentedNone"])),
            (B, &[P::Deprecated], Ok(&["symbols.DeprecatedAPI", "symbols.DeprecatedLater"])),
            (F, &[P::Async], Err(NO_ASYNC)),
            // A method's receiver and parameter lists are not its result.
            (B, &[P::Returns("*")], Ok(&[])),
            (F, &[P::Returns("int")], Ok(&["symbols.ExportedStruct.Area", "symbols.boundaryMultiLineHeader", "symbols.packageFunction", "symbols.packageStruct.Area"])),
            (B, &[P::ParamType("int")], Ok(&["symbols.DocumentedPair", "symbols.ExportedStruct.WithReceiver", "symbols.Variadic"])),
            (B, &[P::ParamType("ExportedStruct")], Ok(&[])),
            (B, &[P::MinParams(2)], Ok(&["symbols.DocumentedPair", "symbols.Variadic"])),
            (B, &[P::In("ExportedStruct")], Ok(&["symbols.ExportedStruct.WithReceiver"])),
            (B, &[P::Name("documented")], Ok(&["symbols.DocumentedPair", "symbols.UndocumentedNone"])),
            (B, &[P::Name("Documented")], Ok(&["symbols.DocumentedPair"])),
            (B, &[P::Name("documented"), P::ExcludeName("undocumented")], Ok(&["symbols.DocumentedPair"])),
            (B, &[P::Name("symbols.Variadic*")], Ok(&["symbols.Variadic", "symbols.Variadic.first", "symbols.Variadic.rest"])),
        ],
    },
    SelectionRow {
        language: L::Python,
        cells: &[
            (B, &[P::Scope(S::Item)], Ok(&["Receiver", "deprecated_api", "documented_pair", "look_alike_decorator", "qualified_deprecated", "test_helper", "undocumented_none", "variadic", "warnings_deprecated"])),
            (B, &[P::Scope(S::Member)], Ok(&["Receiver.with_receiver"])),
            (B, &[P::Scope(S::Local)], Ok(&["Receiver.with_receiver.only", "Receiver.with_receiver.self", "documented_pair.first", "documented_pair.second", "variadic.first", "variadic.options", "variadic.rest"])),
            (B, &[P::Visibility(V::Public)], Ok(&[])),
            (F, &[P::Visibility(V::Private)], Ok(&[])),
            (B, &[P::Exported], Err(NO_EXPORT)),
            (B, &[P::TestsOnly], Err(NO_TEST_MARKER)),
            (B, &[P::ExcludeTests, P::Name("test")], Ok(&["test_helper"])),
            (B, &[P::Undocumented, P::Name("documented")], Ok(&["undocumented_none"])),
            (B, &[P::Deprecated], Ok(&["deprecated_api", "qualified_deprecated", "warnings_deprecated"])),
            (F, &[P::Async], Ok(&["FirstShape.area", "async_function"])),
            (B, &[P::Returns("*")], Ok(&[])),
            (F, &[P::Returns("int")], Ok(&["FirstShape.area", "PlainWidget.instance_method", "SecondShape.area", "boundary_multi_line_header", "module_function"])),
            (B, &[P::ParamType("*")], Ok(&[])),
            (F, &[P::ParamType("int")], Ok(&["PlainWidget.__init__", "boundary_multi_line_header", "module_function"])),
            (B, &[P::MinParams(2)], Ok(&["Receiver.with_receiver", "documented_pair", "variadic"])),
            (B, &[P::In("receiver")], Ok(&["Receiver.with_receiver", "Receiver.with_receiver.only", "Receiver.with_receiver.self"])),
            (B, &[P::Name("documented")], Ok(&["documented_pair", "undocumented_none"])),
            (B, &[P::Name("documented"), P::ExcludeName("undocumented")], Ok(&["documented_pair"])),
            (B, &[P::Name("*#Receiver.*")], Ok(&["Receiver.with_receiver", "Receiver.with_receiver.only", "Receiver.with_receiver.self"])),
        ],
    },
    SelectionRow {
        language: L::Java,
        cells: &[
            (B, &[P::Scope(S::Item)], Ok(&["com.example.symbols.FlagsAndMetrics"])),
            (B, &[P::Scope(S::Member)], Ok(&["com.example.symbols.FlagsAndMetrics.annotatedTest", "com.example.symbols.FlagsAndMetrics.deprecatedByTag", "com.example.symbols.FlagsAndMetrics.deprecatedQualified", "com.example.symbols.FlagsAndMetrics.documentedPair", "com.example.symbols.FlagsAndMetrics.qualifiedTest", "com.example.symbols.FlagsAndMetrics.testHelper", "com.example.symbols.FlagsAndMetrics.undocumentedNone", "com.example.symbols.FlagsAndMetrics.variadic"])),
            (B, &[P::Scope(S::Local)], Ok(&["com.example.symbols.FlagsAndMetrics.documentedPair.first", "com.example.symbols.FlagsAndMetrics.documentedPair.second", "com.example.symbols.FlagsAndMetrics.variadic.first"])),
            (B, &[P::Visibility(V::Package)], Ok(&["com.example.symbols.FlagsAndMetrics", "com.example.symbols.FlagsAndMetrics.annotatedTest", "com.example.symbols.FlagsAndMetrics.deprecatedByTag", "com.example.symbols.FlagsAndMetrics.deprecatedQualified", "com.example.symbols.FlagsAndMetrics.documentedPair", "com.example.symbols.FlagsAndMetrics.qualifiedTest", "com.example.symbols.FlagsAndMetrics.testHelper", "com.example.symbols.FlagsAndMetrics.undocumentedNone", "com.example.symbols.FlagsAndMetrics.variadic"])),
            (B, &[P::Exported], Err(NO_EXPORT)),
            (B, &[P::TestsOnly], Ok(&["com.example.symbols.FlagsAndMetrics.annotatedTest", "com.example.symbols.FlagsAndMetrics.qualifiedTest"])),
            (B, &[P::ExcludeTests, P::Name("test")], Ok(&["com.example.symbols.FlagsAndMetrics.testHelper"])),
            (B, &[P::Undocumented, P::Name("documented")], Ok(&["com.example.symbols.FlagsAndMetrics.undocumentedNone"])),
            (B, &[P::Deprecated], Ok(&["com.example.symbols.FlagsAndMetrics.deprecatedByTag", "com.example.symbols.FlagsAndMetrics.deprecatedQualified"])),
            (F, &[P::Async], Err(NO_ASYNC)),
            (B, &[P::Returns("void"), P::Name("document")], Ok(&["com.example.symbols.FlagsAndMetrics.documentedPair", "com.example.symbols.FlagsAndMetrics.undocumentedNone"])),
            (B, &[P::ParamType("int")], Ok(&["com.example.symbols.FlagsAndMetrics.documentedPair", "com.example.symbols.FlagsAndMetrics.variadic"])),
            (B, &[P::MinParams(2)], Ok(&["com.example.symbols.FlagsAndMetrics.documentedPair", "com.example.symbols.FlagsAndMetrics.variadic"])),
            (B, &[P::In("FlagsAndMetrics"), P::Name("pair")], Ok(&["com.example.symbols.FlagsAndMetrics.documentedPair"])),
            (B, &[P::Name("documented")], Ok(&["com.example.symbols.FlagsAndMetrics.documentedPair", "com.example.symbols.FlagsAndMetrics.undocumentedNone"])),
            (B, &[P::Name("documented"), P::ExcludeName("undocumented")], Ok(&["com.example.symbols.FlagsAndMetrics.documentedPair"])),
            // A namespace-rooted name has no file prefix.
            (B, &[P::Name("com.example.symbols.FlagsAndMetrics.documentedPair*")], Ok(&["com.example.symbols.FlagsAndMetrics.documentedPair", "com.example.symbols.FlagsAndMetrics.documentedPair.first", "com.example.symbols.FlagsAndMetrics.documentedPair.second"])),
            (B, &[P::Name("FlagsAndMetrics.*")], Ok(&[])),
        ],
    },
    SelectionRow {
        language: L::CSharp,
        cells: &[
            (B, &[P::Scope(S::Item)], Ok(&["Example.Symbols.FlagsAndMetrics"])),
            (B, &[P::Scope(S::Member)], Ok(&["Example.Symbols.FlagsAndMetrics.DocumentedPair", "Example.Symbols.FlagsAndMetrics.FactTest", "Example.Symbols.FlagsAndMetrics.MethodTest", "Example.Symbols.FlagsAndMetrics.ObsoleteQualified", "Example.Symbols.FlagsAndMetrics.PlainTest", "Example.Symbols.FlagsAndMetrics.TestHelper", "Example.Symbols.FlagsAndMetrics.UndocumentedNone", "Example.Symbols.FlagsAndMetrics.Variadic"])),
            (B, &[P::Scope(S::Local)], Ok(&["Example.Symbols.FlagsAndMetrics.DocumentedPair.first", "Example.Symbols.FlagsAndMetrics.DocumentedPair.second", "Example.Symbols.FlagsAndMetrics.Variadic.first"])),
            (B, &[P::Visibility(V::Internal)], Ok(&["Example.Symbols.FlagsAndMetrics"])),
            (B, &[P::Exported], Err(NO_EXPORT)),
            (B, &[P::TestsOnly], Ok(&["Example.Symbols.FlagsAndMetrics.FactTest", "Example.Symbols.FlagsAndMetrics.MethodTest", "Example.Symbols.FlagsAndMetrics.PlainTest"])),
            (B, &[P::ExcludeTests, P::Name("test")], Ok(&["Example.Symbols.FlagsAndMetrics.TestHelper"])),
            (B, &[P::Undocumented, P::Name("documented")], Ok(&["Example.Symbols.FlagsAndMetrics.UndocumentedNone"])),
            (B, &[P::Deprecated], Ok(&["Example.Symbols.FlagsAndMetrics.ObsoleteQualified"])),
            (F, &[P::Async], Ok(&["Example.Symbols.AsyncWork.RunAsync"])),
            (F, &[P::Async, P::Returns("Task")], Ok(&["Example.Symbols.AsyncWork.RunAsync"])),
            (B, &[P::ParamType("int")], Ok(&["Example.Symbols.FlagsAndMetrics.DocumentedPair", "Example.Symbols.FlagsAndMetrics.Variadic"])),
            (B, &[P::MinParams(2)], Ok(&["Example.Symbols.FlagsAndMetrics.DocumentedPair", "Example.Symbols.FlagsAndMetrics.Variadic"])),
            (B, &[P::In("FlagsAndMetrics"), P::Name("variadic")], Ok(&["Example.Symbols.FlagsAndMetrics.Variadic"])),
            (B, &[P::Name("documented")], Ok(&["Example.Symbols.FlagsAndMetrics.DocumentedPair", "Example.Symbols.FlagsAndMetrics.UndocumentedNone"])),
            (B, &[P::Name("documented"), P::ExcludeName("undocumented")], Ok(&["Example.Symbols.FlagsAndMetrics.DocumentedPair"])),
            (B, &[P::Name("*.Variadic*")], Ok(&["Example.Symbols.FlagsAndMetrics.Variadic", "Example.Symbols.FlagsAndMetrics.Variadic.first"])),
        ],
    },
    SelectionRow {
        language: L::C,
        cells: &[
            (B, &[P::Scope(S::Item)], Ok(&["documented_pair", "test_helper", "undocumented_none", "variadic"])),
            (B, &[P::Scope(S::Member)], Ok(&[])),
            (B, &[P::Scope(S::Local)], Ok(&["documented_pair.first", "documented_pair.second", "variadic.first"])),
            (B, &[P::Visibility(V::Public)], Ok(&["documented_pair", "test_helper", "undocumented_none", "variadic"])),
            (B, &[P::Exported], Err(NO_EXPORT)),
            (B, &[P::TestsOnly], Err(NO_TEST_MARKER)),
            (B, &[P::ExcludeTests, P::Name("test")], Ok(&["test_helper"])),
            (B, &[P::Undocumented, P::Name("documented")], Ok(&["undocumented_none"])),
            (B, &[P::Deprecated], Err(NO_DEPRECATION)),
            (F, &[P::Async], Err(NO_ASYNC)),
            (B, &[P::Returns("void")], Ok(&["documented_pair", "test_helper", "undocumented_none", "variadic"])),
            (B, &[P::ParamType("int")], Ok(&["documented_pair", "variadic"])),
            // `...` counts once; `(void)` is zero.
            (B, &[P::MinParams(2)], Ok(&["documented_pair", "variadic"])),
            (F, &[P::In("NamedTag")], Ok(&["NamedTag.tag_field", "NamedTag.tag_name_field"])),
            (B, &[P::Name("documented")], Ok(&["documented_pair", "undocumented_none"])),
            (B, &[P::Name("documented"), P::ExcludeName("undocumented")], Ok(&["documented_pair"])),
            (B, &[P::Name("c.c#documented_pair*")], Ok(&["documented_pair", "documented_pair.first", "documented_pair.second"])),
        ],
    },
    SelectionRow {
        language: L::Cpp,
        cells: &[
            (B, &[P::Scope(S::Item)], Ok(&["DeprecatedClass", "deprecated_api", "documented_pair", "namespaced_attribute", "test_helper", "undocumented_none", "variadic"])),
            (B, &[P::Scope(S::Member)], Ok(&[])),
            (B, &[P::Scope(S::Local)], Ok(&["documented_pair::first", "documented_pair::second", "variadic::first"])),
            (B, &[P::Visibility(V::Unknown)], Ok(&["DeprecatedClass", "documented_pair::first", "documented_pair::second", "variadic::first"])),
            (B, &[P::Exported], Err(NO_EXPORT)),
            (B, &[P::TestsOnly], Err(NO_TEST_MARKER)),
            (B, &[P::ExcludeTests, P::Name("test")], Ok(&["test_helper"])),
            (B, &[P::Undocumented, P::Name("documented")], Ok(&["undocumented_none"])),
            // `[[gnu::deprecated]]` is not the standard attribute.
            (B, &[P::Deprecated], Ok(&["DeprecatedClass", "deprecated_api"])),
            (F, &[P::Async], Err(NO_ASYNC)),
            (B, &[P::Returns("void"), P::Deprecated], Ok(&["deprecated_api"])),
            (B, &[P::ParamType("int")], Ok(&["documented_pair", "variadic"])),
            (B, &[P::MinParams(2)], Ok(&["documented_pair", "variadic"])),
            (F, &[P::In("FirstShape")], Ok(&["outer_space::inner_space::FirstShape::area"])),
            (B, &[P::Name("documented")], Ok(&["documented_pair", "undocumented_none"])),
            (B, &[P::Name("documented"), P::ExcludeName("undocumented")], Ok(&["documented_pair"])),
            (F, &[P::Name("outer_space::*::area")], Ok(&["outer_space::inner_space::FirstShape::area"])),
            (B, &[P::Name("*documented_pair::*")], Ok(&["documented_pair::first", "documented_pair::second"])),
        ],
    },
    SelectionRow {
        language: L::Swift,
        cells: &[
            (B, &[P::Scope(S::Item)], Ok(&["documentedPair", "platformDeprecated", "testHelper", "undocumentedNone", "variadic"])),
            (B, &[P::Scope(S::Member)], Ok(&[])),
            (B, &[P::Scope(S::Local)], Ok(&["documentedPair.first", "documentedPair.second", "variadic.first", "variadic.rest"])),
            (B, &[P::Visibility(V::Internal)], Ok(&["documentedPair", "platformDeprecated", "testHelper", "undocumentedNone", "variadic"])),
            (B, &[P::Exported], Err(NO_EXPORT)),
            (B, &[P::TestsOnly], Err(NO_TEST_MARKER)),
            (B, &[P::ExcludeTests, P::Name("test")], Ok(&["testHelper"])),
            (B, &[P::Undocumented, P::Name("documented")], Ok(&["undocumentedNone"])),
            // A platform-specific `@available(iOS, deprecated:)` is not a
            // deprecation of the declaration.
            (B, &[P::Deprecated], Ok(&[])),
            (F, &[P::Async], Ok(&["FirstShape.area", "asyncFunction"])),
            (F, &[P::Async, P::Returns("Int")], Ok(&["FirstShape.area"])),
            (B, &[P::Returns("*")], Ok(&[])),
            (B, &[P::ParamType("Int")], Ok(&["documentedPair", "variadic"])),
            (B, &[P::MinParams(2)], Ok(&["documentedPair", "variadic"])),
            (F, &[P::In("SecondShape")], Ok(&["SecondShape.area"])),
            (B, &[P::Name("documented")], Ok(&["documentedPair", "undocumentedNone"])),
            (B, &[P::Name("documented"), P::ExcludeName("undocumented")], Ok(&["documentedPair"])),
            (B, &[P::Name("*#variadic.*")], Ok(&["variadic.first", "variadic.rest"])),
        ],
    },
    SelectionRow {
        language: L::Scala,
        cells: &[
            (B, &[P::Scope(S::Item)], Ok(&["example.symbols.documentedPair", "example.symbols.qualifiedDeprecated", "example.symbols.testHelper", "example.symbols.undocumentedNone", "example.symbols.variadic"])),
            (B, &[P::Scope(S::Member)], Ok(&[])),
            (B, &[P::Scope(S::Local)], Ok(&["example.symbols.documentedPair.first", "example.symbols.documentedPair.second", "example.symbols.variadic.first", "example.symbols.variadic.rest"])),
            (B, &[P::Visibility(V::Public)], Ok(&["example.symbols.documentedPair", "example.symbols.qualifiedDeprecated", "example.symbols.testHelper", "example.symbols.undocumentedNone", "example.symbols.variadic"])),
            (B, &[P::Exported], Err(NO_EXPORT)),
            (B, &[P::TestsOnly], Err(NO_TEST_MARKER)),
            (B, &[P::ExcludeTests, P::Name("test")], Ok(&["example.symbols.testHelper"])),
            (B, &[P::Undocumented, P::Name("documented")], Ok(&["example.symbols.undocumentedNone"])),
            (B, &[P::Deprecated], Ok(&["example.symbols.qualifiedDeprecated"])),
            (F, &[P::Async], Err(NO_ASYNC)),
            (B, &[P::Returns("Unit"), P::Name("variadic")], Ok(&["example.symbols.variadic"])),
            (B, &[P::ParamType("Int")], Ok(&["example.symbols.documentedPair", "example.symbols.variadic"])),
            (B, &[P::MinParams(2)], Ok(&["example.symbols.documentedPair", "example.symbols.variadic"])),
            (F, &[P::In("SecondShape")], Ok(&["example.symbols.SecondShape.area"])),
            (B, &[P::Name("documented")], Ok(&["example.symbols.documentedPair", "example.symbols.undocumentedNone"])),
            (B, &[P::Name("documented"), P::ExcludeName("undocumented")], Ok(&["example.symbols.documentedPair"])),
            (B, &[P::Name("example.symbols.variadic*")], Ok(&["example.symbols.variadic", "example.symbols.variadic.first", "example.symbols.variadic.rest"])),
        ],
    },
    SelectionRow {
        language: L::Php,
        cells: &[
            (B, &[P::Scope(S::Item)], Ok(&[r"Example\Symbols\deprecated_api", r"Example\Symbols\documented_pair", r"Example\Symbols\test_helper", r"Example\Symbols\undocumented_none", r"Example\Symbols\variadic"])),
            (B, &[P::Scope(S::Member)], Ok(&[])),
            (B, &[P::Scope(S::Local)], Ok(&[r"Example\Symbols\documented_pair::first", r"Example\Symbols\documented_pair::second", r"Example\Symbols\variadic::first"])),
            (B, &[P::Visibility(V::Public)], Ok(&[r"Example\Symbols\deprecated_api", r"Example\Symbols\documented_pair", r"Example\Symbols\test_helper", r"Example\Symbols\undocumented_none", r"Example\Symbols\variadic"])),
            (B, &[P::Exported], Err(NO_EXPORT)),
            (B, &[P::TestsOnly], Err(NO_TEST_MARKER)),
            (B, &[P::ExcludeTests, P::Name("test")], Ok(&[r"Example\Symbols\test_helper"])),
            (B, &[P::Undocumented, P::Name("documented")], Ok(&[r"Example\Symbols\undocumented_none"])),
            (B, &[P::Deprecated], Ok(&[r"Example\Symbols\deprecated_api"])),
            (F, &[P::Async], Err(NO_ASYNC)),
            (B, &[P::Returns("void"), P::Name("pair")], Ok(&[r"Example\Symbols\documented_pair"])),
            (B, &[P::ParamType("int")], Ok(&[r"Example\Symbols\documented_pair", r"Example\Symbols\variadic"])),
            (B, &[P::MinParams(2)], Ok(&[r"Example\Symbols\documented_pair", r"Example\Symbols\variadic"])),
            (F, &[P::In("ConstantForms")], Ok(&[r"Example\Symbols\ConstantForms::CLASS_CONSTANT"])),
            (B, &[P::Name("documented")], Ok(&[r"Example\Symbols\documented_pair", r"Example\Symbols\undocumented_none"])),
            (B, &[P::Name("documented"), P::ExcludeName("undocumented")], Ok(&[r"Example\Symbols\documented_pair"])),
            (B, &[P::Name(r"Example\Symbols\documented*")], Ok(&[r"Example\Symbols\documented_pair", r"Example\Symbols\documented_pair::first", r"Example\Symbols\documented_pair::second"])),
        ],
    },
    SelectionRow {
        language: L::Perl,
        cells: &[
            (B, &[P::Scope(S::Item)], Ok(&["Fixture::Other::test_helper"])),
            (B, &[P::Scope(S::Local)], Ok(&[])),
            (B, &[P::Visibility(V::Unknown)], Ok(&["Fixture::Other::test_helper"])),
            (B, &[P::Exported], Err(NO_EXPORT)),
            (B, &[P::TestsOnly], Err(NO_TEST_MARKER)),
            (B, &[P::ExcludeTests, P::Name("test")], Ok(&["Fixture::Other::test_helper"])),
            (B, &[P::Undocumented], Err(NO_DOCS)),
            (B, &[P::Deprecated], Err(NO_DEPRECATION)),
            (F, &[P::Async], Err(NO_ASYNC)),
            (F, &[P::Returns("*")], Err(NO_TYPES)),
            (F, &[P::ParamType("*")], Err(NO_TYPES)),
            (F, &[P::MinParams(0)], Err(NO_PARAMETER_COUNT)),
            (B, &[P::In("Fixture::Other")], Ok(&["Fixture::Other::test_helper"])),
            (B, &[P::Name("test_helper!")], Ok(&["Fixture::Other::test_helper"])),
            (B, &[P::Name("helper"), P::ExcludeName("test")], Ok(&[])),
            (B, &[P::Name("Fixture::Other::*")], Ok(&["Fixture::Other::test_helper"])),
        ],
    },
    SelectionRow {
        language: L::Lua,
        cells: &[
            (B, &[P::Scope(S::Item)], Ok(&["documented_pair", "test_helper", "variadic"])),
            (B, &[P::Scope(S::Member)], Ok(&[])),
            (B, &[P::Scope(S::Local)], Ok(&["documented_pair.first", "documented_pair.second", "variadic.first"])),
            (B, &[P::Visibility(V::ModulePrivate)], Ok(&["documented_pair", "test_helper", "variadic"])),
            (B, &[P::Exported], Err(NO_EXPORT)),
            (B, &[P::TestsOnly], Err(NO_TEST_MARKER)),
            (B, &[P::ExcludeTests, P::Name("test")], Ok(&["test_helper"])),
            (B, &[P::Undocumented], Err(NO_DOCS)),
            (B, &[P::Deprecated], Err(NO_DEPRECATION)),
            (F, &[P::Async], Err(NO_ASYNC)),
            (F, &[P::Returns("*")], Err(NO_TYPES)),
            (F, &[P::ParamType("*")], Err(NO_TYPES)),
            (B, &[P::MinParams(2)], Ok(&["documented_pair", "variadic"])),
            (F, &[P::In("Widget")], Ok(&["Widget.boundaryMultiLineHeader", "Widget.tableField", "Widget.tableFunction", "Widget:methodWithSelf"])),
            (B, &[P::Name("documented")], Ok(&["documented_pair"])),
            (B, &[P::Name("documented"), P::ExcludeName("pair")], Ok(&[])),
            (F, &[P::Name("*#Widget:*")], Ok(&["Widget:methodWithSelf"])),
        ],
    },
    SelectionRow {
        language: L::Bash,
        cells: &[
            (B, &[P::Scope(S::Item)], Ok(&["test_helper"])),
            (F, &[P::Scope(S::Local), P::Name("nested")], Ok(&["outer_function.nested_function"])),
            (B, &[P::Visibility(V::Unknown)], Ok(&["test_helper"])),
            (B, &[P::Exported], Err(NO_EXPORT)),
            (B, &[P::TestsOnly], Err(NO_TEST_MARKER)),
            (B, &[P::ExcludeTests, P::Name("test")], Ok(&["test_helper"])),
            (B, &[P::Undocumented], Err(NO_DOCS)),
            (B, &[P::Deprecated], Err(NO_DEPRECATION)),
            (F, &[P::Async], Err(NO_ASYNC)),
            (F, &[P::Returns("*")], Err(NO_TYPES)),
            (F, &[P::ParamType("*")], Err(NO_TYPES)),
            (F, &[P::MinParams(0)], Err(NO_PARAMETER_COUNT)),
            // A nested shell function has no container; its qualified name
            // still starts at the enclosing function.
            (F, &[P::In("outer_function")], Ok(&[])),
            (B, &[P::Name("TEST")], Ok(&[])),
            (B, &[P::Name("test"), P::ExcludeName("helper")], Ok(&[])),
            (F, &[P::Name("*#outer_function.*")], Ok(&["outer_function.nested_function"])),
        ],
    },
    SelectionRow {
        language: L::Zsh,
        cells: &[
            (B, &[P::Scope(S::Item)], Ok(&["test_helper"])),
            (F, &[P::Scope(S::Local), P::Name("nested")], Ok(&["outer_function.nested_function"])),
            (B, &[P::Visibility(V::Unknown)], Ok(&["test_helper"])),
            (B, &[P::Exported], Err(NO_EXPORT)),
            (B, &[P::TestsOnly], Err(NO_TEST_MARKER)),
            (B, &[P::ExcludeTests, P::Name("test")], Ok(&["test_helper"])),
            (B, &[P::Undocumented], Err(NO_DOCS)),
            (B, &[P::Deprecated], Err(NO_DEPRECATION)),
            (F, &[P::Async], Err(NO_ASYNC)),
            (F, &[P::Returns("*")], Err(NO_TYPES)),
            (F, &[P::ParamType("*")], Err(NO_TYPES)),
            (F, &[P::MinParams(0)], Err(NO_PARAMETER_COUNT)),
            // A nested shell function has no container; its qualified name
            // still starts at the enclosing function.
            (F, &[P::In("outer_function")], Ok(&[])),
            (B, &[P::Name("TEST")], Ok(&[])),
            (B, &[P::Name("test"), P::ExcludeName("helper")], Ok(&[])),
            (F, &[P::Name("*#outer_function.*")], Ok(&["outer_function.nested_function"])),
        ],
    },
];

/// The languages with a selection-filter row, for the coverage guard.
pub fn selection_filter_matrix_languages() -> Vec<ProgrammingLanguage> {
    SELECTION_FILTER_MATRIX.iter().map(|row| row.language).collect()
}

const MARKER: &str = "Flags and metrics";

crate::language_cases::language_tests!(selection_filters_keep_exactly_the_expected_declarations => check_selection_filters_keep_exactly_the_expected_declarations);

fn check_selection_filters_keep_exactly_the_expected_declarations(language: ProgrammingLanguage) {
    let mut failures = Vec::new();
    for row in SELECTION_FILTER_MATRIX.iter().filter(|row| row.language == language) {
        let path = primary_fixture(row.language).expect("primary fixture");
        let label = display_name(&path);
        let mut covered = [false; Predicate::VARIANTS];
        for (_, predicates, _) in row.cells {
            for predicate in *predicates {
                covered[predicate.variant()] = true;
            }
        }
        assert!(covered.iter().all(|seen| *seen), "{label}: a predicate has no cell: {covered:?}");

        let source = std::fs::read_to_string(&path).expect("read fixture");
        let marker_line = source
            .lines()
            .position(|line| line.contains(MARKER))
            .unwrap_or_else(|| panic!("{label}: no `{MARKER}` block")) as u32;
        let dir = TempDir::new().expect("temp dir");
        let copy = dir.path().join(path.file_name().expect("file name"));
        std::fs::copy(&path, &copy).expect("copy fixture");

        for (region, predicates, expected) in row.cells {
            let query = predicates.iter().fold(SymbolQuery::new(), |query, predicate| predicate.apply(query));
            let actual: BTreeSet<String> = SourceFile::read(&copy)
                .expect("read fixture copy")
                .query(&query)
                .expect("query")
                .index
                .symbols
                .iter()
                .filter(|record: &&SymbolRecord| *region == F || record.source.declaration_span.start.line > marker_line)
                .map(|record| in_file_name(record).to_string())
                .collect();
            let wanted: BTreeSet<String> = expected
                .map(|names| names.iter().map(|name| name.to_string()).collect())
                .unwrap_or_default();
            if actual != wanted {
                failures.push(format!("{label} {region:?} {predicates:?}: got {actual:?}, want {wanted:?}"));
            }
        }
    }
    assert!(failures.is_empty(), "selection filters:\n{}", failures.join("\n"));
}

/// `(extension, source, predicate)`: the predicate keeps the one function
/// `patterned`, whose parameters are bound by patterns or spread across
/// curried lists rather than named one per list.
type PatternedCell = (&'static str, &'static str, Predicate);

const RUST_PATTERNED: &str = "pub fn patterned((a, b): (u32, u32), _: String) {}\n";
const TS_PATTERNED: &str = "export function patterned({x}: {x:number}, [a,b]: number[]) {}\n";
const JS_PATTERNED: &str = "export function patterned({x}, [a,b]) {}\n";
const SCALA_PATTERNED: &str = "object O {\n  def patterned(a: Int)(b: String): Int = a\n}\n";

const PATTERNED_CELLS: &[PatternedCell] = &[
    ("rs", RUST_PATTERNED, P::MinParams(2)),
    ("rs", RUST_PATTERNED, P::ParamType("(u32, u32)")),
    ("rs", RUST_PATTERNED, P::ParamType("String")),
    ("ts", TS_PATTERNED, P::MinParams(2)),
    ("ts", TS_PATTERNED, P::ParamType("number[]")),
    ("js", JS_PATTERNED, P::MinParams(2)),
    ("scala", SCALA_PATTERNED, P::MinParams(2)),
    ("scala", SCALA_PATTERNED, P::ParamType("String")),
];

#[test]
fn signature_predicates_select_patterned_and_curried_parameters() {
    let mut failures = Vec::new();
    for (extension, source, predicate) in PATTERNED_CELLS {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join(format!("patterned.{extension}"));
        std::fs::write(&path, source).expect("write snippet");
        let query = predicate.apply(SymbolQuery::new().name(NamePattern::new("patterned")));
        let actual: Vec<String> = SourceFile::read(&path)
            .expect("read snippet")
            .query(&query)
            .expect("query")
            .index
            .symbols
            .iter()
            .map(|record| record.identity.name.clone())
            .collect();
        if actual != ["patterned"] {
            failures.push(format!(".{extension} {predicate:?}: got {actual:?}"));
        }
    }
    assert!(failures.is_empty(), "patterned selections:\n{}", failures.join("\n"));
}

/// `(extension, source, kinds, predicates, in-file names kept)`: one
/// selection over declarations the deferred kinds report. Names are sorted;
/// a repeated name is a separate declaration (an overload).
type DeferredCell = (
    &'static str,
    &'static str,
    &'static [SymbolKindV2],
    &'static [Predicate],
    &'static [&'static str],
);

const JAVA_DEFERRED: &str = "class Counter {\n    Counter(int start) {}\n    private Counter() {}\n    void tick() {}\n}\n\nenum Color { RED, BLUE }\n";
const CSHARP_DEFERRED: &str = "class C {\n    public C(int x) {}\n    private C() {}\n}\n\nenum E { Red, Blue = 2 }\n";
const CPP_DEFERRED: &str = "class W {\npublic:\n    W(int x);\nprivate:\n    W();\n};\n\nenum class E { A, B };\n";
const PYTHON_DEFERRED: &str = "class Box:\n    def __init__(self, width, height=1):\n        pass\n";
const SWIFT_DEFERRED: &str = "struct S {\n    init(v: Int) {}\n    private init() {}\n}\n\ntypealias Alias = Int\n";
const PHP_DEFERRED: &str = "<?php\nnamespace App;\n\nclass Service {\n    public function __construct(int $x) {}\n}\n\nenum Suit {\n    case Hearts;\n}\n";
const TS_DEFERRED: &str = "export enum Mode { On, Off = 2 }\nenum Hidden { H }\nnamespace Space {}\n";
const GO_DEFERRED: &str = "package p\n\ntype Alias = int\n\ntype Point struct {\n\tX, Y int\n\tlabel string\n}\n";
const SCALA_DEFERRED: &str = "object Registry {\n  val count = 0\n  type Key = String\n}\n\nclass Box {\n  private val width: Int = 1\n  var height = 2\n}\n\nenum Tone {\n  case Low, High\n}\n";
const C_DEFERRED: &str = "enum E { RED, BLUE = 2 };\n";

const CONSTRUCTOR: &[SymbolKindV2] = &[SymbolKindV2::Constructor];
const FUNCTIONS: &[SymbolKindV2] = &[SymbolKindV2::Function, SymbolKindV2::Method, SymbolKindV2::Constructor];
const VARIANT: &[SymbolKindV2] = &[SymbolKindV2::EnumVariant];
const ALIAS: &[SymbolKindV2] = &[SymbolKindV2::TypeAlias];
const FIELD: &[SymbolKindV2] = &[SymbolKindV2::Field];
const NAMESPACE: &[SymbolKindV2] = &[SymbolKindV2::Namespace];

const DEFERRED_CELLS: &[DeferredCell] = &[
    // Java: an explicit constructor without a keyword is package access.
    ("java", JAVA_DEFERRED, CONSTRUCTOR, &[], &["Counter.Counter", "Counter.Counter"]),
    ("java", JAVA_DEFERRED, &[SymbolKindV2::Method], &[], &["Counter.tick"]),
    ("java", JAVA_DEFERRED, CONSTRUCTOR, &[P::Visibility(V::Private)], &["Counter.Counter"]),
    ("java", JAVA_DEFERRED, FUNCTIONS, &[P::MinParams(1)], &["Counter.Counter"]),
    ("java", JAVA_DEFERRED, FUNCTIONS, &[P::ParamType("int")], &["Counter.Counter"]),
    ("java", JAVA_DEFERRED, FUNCTIONS, &[P::Returns("*")], &["Counter.tick"]),
    ("java", JAVA_DEFERRED, CONSTRUCTOR, &[P::Scope(S::Member)], &["Counter.Counter", "Counter.Counter"]),
    ("java", JAVA_DEFERRED, CONSTRUCTOR, &[P::Scope(S::Item)], &[]),
    ("java", JAVA_DEFERRED, VARIANT, &[P::Visibility(V::Public), P::In("Color")], &["Color.BLUE", "Color.RED"]),
    // C#
    ("cs", CSHARP_DEFERRED, CONSTRUCTOR, &[P::Visibility(V::Public)], &["C.C"]),
    ("cs", CSHARP_DEFERRED, CONSTRUCTOR, &[P::MinParams(1)], &["C.C"]),
    ("cs", CSHARP_DEFERRED, VARIANT, &[P::Visibility(V::Public)], &["E.Blue", "E.Red"]),
    // C++: access follows the class's access sections; enumerators are Unknown.
    ("cpp", CPP_DEFERRED, CONSTRUCTOR, &[P::Visibility(V::Public)], &["W::W"]),
    ("cpp", CPP_DEFERRED, CONSTRUCTOR, &[P::ParamType("int")], &["W::W"]),
    ("cpp", CPP_DEFERRED, VARIANT, &[P::In("E")], &["E::A", "E::B"]),
    ("cpp", CPP_DEFERRED, VARIANT, &[P::Visibility(V::Public)], &[]),
    // Python: `self` is a parameter.
    ("py", PYTHON_DEFERRED, CONSTRUCTOR, &[P::MinParams(3)], &["Box.__init__"]),
    ("py", PYTHON_DEFERRED, CONSTRUCTOR, &[P::MinParams(4)], &[]),
    ("py", PYTHON_DEFERRED, FUNCTIONS, &[P::Returns("*")], &[]),
    // Swift
    ("swift", SWIFT_DEFERRED, CONSTRUCTOR, &[P::Visibility(V::Private)], &["S.init"]),
    ("swift", SWIFT_DEFERRED, CONSTRUCTOR, &[P::ParamType("Int")], &["S.init"]),
    ("swift", SWIFT_DEFERRED, ALIAS, &[P::Scope(S::Item)], &["Alias"]),
    // PHP: namespace-rooted names carry no file prefix.
    ("php", PHP_DEFERRED, CONSTRUCTOR, &[P::Visibility(V::Public), P::MinParams(1)], &["App\\Service::__construct"]),
    ("php", PHP_DEFERRED, VARIANT, &[P::In("Suit")], &["App\\Suit::Hearts"]),
    ("php", PHP_DEFERRED, NAMESPACE, &[P::Scope(S::Item)], &["App"]),
    ("php", PHP_DEFERRED, NAMESPACE, &[P::Scope(S::Member)], &[]),
    // TypeScript: members take their enum's access.
    ("ts", TS_DEFERRED, VARIANT, &[P::Visibility(V::Public)], &["Mode.Off", "Mode.On"]),
    ("ts", TS_DEFERRED, VARIANT, &[P::In("Hidden")], &["Hidden.H"]),
    ("ts", TS_DEFERRED, NAMESPACE, &[P::Name("Space")], &["Space"]),
    // Go: field access follows the name's case.
    ("go", GO_DEFERRED, FIELD, &[P::Visibility(V::Public)], &["p.Point.X", "p.Point.Y"]),
    ("go", GO_DEFERRED, FIELD, &[P::In("Point")], &["p.Point.X", "p.Point.Y", "p.Point.label"]),
    ("go", GO_DEFERRED, ALIAS, &[], &["p.Alias"]),
    // Scala
    ("scala", SCALA_DEFERRED, FIELD, &[P::Visibility(V::Private)], &["Box.width"]),
    ("scala", SCALA_DEFERRED, FIELD, &[P::Visibility(V::Public)], &["Box.height", "Registry.count"]),
    ("scala", SCALA_DEFERRED, ALIAS, &[P::In("Registry")], &["Registry.Key"]),
    ("scala", SCALA_DEFERRED, VARIANT, &[P::Visibility(V::Public)], &["Tone.High", "Tone.Low"]),
    // C
    ("c", C_DEFERRED, VARIANT, &[P::In("E"), P::Scope(S::Member)], &["E.BLUE", "E.RED"]),
];

/// Every selector applies to the records the deferred kinds report: a kind
/// selection, access, scope, container and signature-shape predicates keep
/// exactly the expected declarations, and a constructor never gains a
/// return type for `returns` to match.
#[test]
fn selectors_apply_to_deferred_kind_records() {
    let mut failures = Vec::new();
    let dir = TempDir::new().expect("temp dir");
    for (ordinal, (extension, source, kinds, predicates, expected)) in DEFERRED_CELLS.iter().enumerate() {
        let path = dir.path().join(format!("deferred_{ordinal}.{extension}"));
        std::fs::write(&path, source).expect("write snippet");
        let query = predicates
            .iter()
            .fold(SymbolQuery::new().kinds(kinds.iter().copied()), |query, predicate| predicate.apply(query));
        let mut actual: Vec<String> = SourceFile::read(&path)
            .expect("read snippet")
            .query(&query)
            .expect("query")
            .index
            .symbols
            .iter()
            .map(|record| in_file_name(record).to_string())
            .collect();
        actual.sort();
        if actual != *expected {
            failures.push(format!(".{extension} {kinds:?} {predicates:?}: got {actual:?}, want {expected:?}"));
        }
    }
    assert!(failures.is_empty(), "deferred selections:\n{}", failures.join("\n"));
}
