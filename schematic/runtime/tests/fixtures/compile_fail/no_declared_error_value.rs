// An endpoint without a declared error body uses `NoDeclaredError`, which has
// no values, so a `Declared` error cannot be constructed for it.
use schematic_runtime::{ErrorKind, NoDeclaredError};

fn main() {
    let _declared: ErrorKind<NoDeclaredError> = ErrorKind::Declared(NoDeclaredError::Body);
}
