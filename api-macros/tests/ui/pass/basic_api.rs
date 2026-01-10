//! Test: Basic API definition with derive macro compiles successfully.

use api_macros::RestApi;

#[derive(RestApi)]
#[api(base_url = "https://api.example.com")]
pub struct BasicApi;

fn main() {
    // Verify the generated method exists
    let _url = BasicApi::__api_base_url();
}
