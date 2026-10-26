//! A violation path names only `&'static` text, so a submitted string
//! cannot become a path segment.

use schematic_runtime::validate::PathSegment;

fn main() {
    let submitted = String::from("a value from the request");
    let _segment = PathSegment::Name(&submitted);
}
