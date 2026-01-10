//! Test: API definition with authentication method compiles successfully.

use api_macros::RestApi;

#[derive(RestApi)]
#[api(base_url = "https://api.example.com")]
#[api(auth = bearer)]
pub struct AuthenticatedApi;

#[derive(RestApi)]
#[api(base_url = "https://api.example.com")]
#[api(auth = header_key)]
pub struct HeaderKeyApi;

#[derive(RestApi)]
#[api(base_url = "https://api.example.com")]
#[api(auth = query_param)]
pub struct QueryParamApi;

fn main() {}
