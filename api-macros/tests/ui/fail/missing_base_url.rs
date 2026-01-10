//! Test: API definition without base_url should fail with clear error.

use api_macros::RestApi;

#[derive(RestApi)]
pub struct NoBaseUrlApi;

fn main() {}
