//! Following redirects anywhere with credentials is reachable only through
//! the `danger_` name: neither a plain constructor nor a plain variant exists.

use schematic_runtime::OriginPolicy;

fn main() {
    let _constructor = OriginPolicy::allow_any();
    let _variant = OriginPolicy::AllowAny;
}
