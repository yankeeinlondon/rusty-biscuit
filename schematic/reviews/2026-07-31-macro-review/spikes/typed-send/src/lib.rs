//! Throwaway spike for D3 (typed send only), D6 (success set + error set),
//! D7/D21 (the `Response<T, E>` envelope), and D17 (GraphQL strings).
//! See `findings.md`.
//!
//! The doctests below are the D3 compile-fail criteria.
//!
//! A typed request cannot be read as another success type:
//!
//! ```compile_fail,E0308
//! # use typed_send_spike::{api::*, client::DemoClient, runtime::*};
//! # async fn f(c: DemoClient) {
//! let r: Response<Vec<Pull>, GhError> = c.send(GetRepoRequest::new("o", "r")).await.unwrap();
//! # }
//! ```
//!
//! ...nor through `data()`:
//!
//! ```compile_fail,E0308
//! # use typed_send_spike::{api::*, client::DemoClient};
//! # async fn f(c: DemoClient) {
//! let resp = c.send(GetRepoRequest::new("o", "r")).await.unwrap();
//! let s: Option<&String> = resp.data();
//! # }
//! ```
//!
//! There is no free generic `request<T>`:
//!
//! ```compile_fail,E0599
//! # use typed_send_spike::{api::*, client::DemoClient};
//! # async fn f(c: DemoClient) {
//! let r: Vec<Pull> = c.request::<Vec<Pull>>(GetRepoRequest::new("o", "r")).await.unwrap();
//! # }
//! ```
//!
//! GraphQL variables are typed:
//!
//! ```compile_fail,E0308
//! # use typed_send_spike::api::*;
//! let q = GraphQlRequest::<ViewerRepos>::new(42u32);
//! ```
//!
//! A mutator's closure type comes from the endpoint:
//!
//! ```compile_fail,E0631
//! # use typed_send_spike::{api::*, client::DemoClient};
//! DemoClient::builder("http://x")
//!     .mutate_response::<GetRepoRequest>(|_ctx, pulls: &mut Vec<Pull>| Ok(()));
//! ```
//!
//! ...and only JSON-decoded endpoints accept one:
//!
//! ```compile_fail,E0277
//! # use typed_send_spike::{api::*, client::DemoClient};
//! DemoClient::builder("http://x")
//!     .mutate_response::<GetReadmeRawRequest>(|_ctx, s| { s.push('!'); Ok(()) });
//! ```
//!
//! The positive control (compiles):
//!
//! ```
//! # use typed_send_spike::{api::*, client::DemoClient, runtime::*};
//! # async fn f(c: DemoClient) {
//! let r: Response<Repo, GhError> = c.send(GetRepoRequest::new("o", "r")).await.unwrap();
//! let _: Option<&Repo> = r.data();
//! # }
//! ```

pub mod api;
pub mod client;
pub mod runtime;
pub mod sniff_port;

#[cfg(feature = "synthetic-old")]
pub mod synthetic_old;
#[cfg(feature = "synthetic-new")]
pub mod synthetic_new;
