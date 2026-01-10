//! Test: API definition with documentation URLs compiles successfully.

use api_macros::RestApi;

#[derive(RestApi)]
#[api(base_url = "https://api.example.com")]
#[api(docs = "https://docs.example.com")]
#[api(openapi = "https://api.example.com/openapi.json")]
pub struct DocumentedApi;

fn main() {}
