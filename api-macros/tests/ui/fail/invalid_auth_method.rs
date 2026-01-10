//! Test: API definition with invalid auth method should fail.

use api_macros::RestApi;

#[derive(RestApi)]
#[api(base_url = "https://api.example.com")]
#[api(auth = invalid_method)]
pub struct InvalidAuthApi;

fn main() {}
