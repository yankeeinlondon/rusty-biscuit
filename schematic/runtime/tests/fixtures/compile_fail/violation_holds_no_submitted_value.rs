//! Only the runtime's checks construct a violation, so no route writes a
//! submitted value into its reason.

use schematic_runtime::validate::{Keyword, Violation};

fn main() {
    let submitted = String::from("a value from the request");
    let _violation = Violation::new(Vec::new(), Keyword::Enum, submitted);
}
