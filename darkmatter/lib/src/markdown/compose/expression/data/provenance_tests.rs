//! Typed failure provenance survives the request memo: a second unguarded
//! call that the memo answers fails with the same variant, and so with the
//! same verdict on whether `try` may recover it.

use std::path::Path;
use std::sync::Arc;

use serde_json::{Value, json};

use super::DataError;
use crate::markdown::compose::expression::functions::call_function;
use crate::markdown::compose::expression::{ExpressionError, FileRefFailure, ResolutionContext};

fn context(dir: &Path) -> ResolutionContext {
    ResolutionContext::new(biscuit_file::FileResolutionContext::new(dir))
}

fn fail(context: &ResolutionContext, args: &[Value]) -> ExpressionError {
    call_function("load_data", args, Some(context)).expect("registered").expect_err("the call fails")
}

fn data(error: &ExpressionError) -> &Arc<DataError> {
    error.data_failure().unwrap_or_else(|| panic!("not a data failure: {error:?}"))
}

#[test]
fn a_memoized_contract_import_failure_stays_a_contract_import() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("ports.yaml"), "- port: 1\n").unwrap();
    let context = context(dir.path());
    let args = [json!("ports.yaml"), json!("{ port: Port@./absent.yaml }[]")];
    let first = fail(&context, &args);
    let second = fail(&context, &args);
    assert_eq!(context.data.stats().contract_preparations, 1, "the second call reuses the memoized failure");
    for error in [&first, &second] {
        assert!(matches!(&**data(error), DataError::ContractImport(d) if d.kind == FileRefFailure::NotFound), "{error:?}");
        assert!(!error.is_recoverable_read_failure());
    }
    assert!(Arc::ptr_eq(data(&first), data(&second)));
}

#[test]
fn a_memoized_malformed_source_stays_malformed() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("broken.yaml"), "a: [\n").unwrap();
    let context = context(dir.path());
    let args = [json!("broken.yaml"), json!("any[]")];
    let first = fail(&context, &args);
    let second = fail(&context, &args);
    assert_eq!(context.data.stats().reads, 1, "the second call reads nothing");
    for error in [&first, &second] {
        assert!(matches!(&**data(error), DataError::Malformed { .. }), "{error:?}");
        assert!(error.is_recoverable_read_failure());
    }
}

#[test]
fn a_memoized_missing_source_stays_a_clean_miss() {
    let dir = tempfile::tempdir().unwrap();
    let context = context(dir.path());
    let args = [json!("absent.yaml"), json!("string[]")];
    for _ in 0..2 {
        let error = fail(&context, &args);
        assert!(
            matches!(&error, ExpressionError::FileReference(d) if d.kind == FileRefFailure::NotFound && d.source.is_none()),
            "{error:?}"
        );
        assert!(error.is_recoverable_read_failure());
    }
}
