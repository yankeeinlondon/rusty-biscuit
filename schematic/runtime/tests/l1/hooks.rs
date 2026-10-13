//! The two response hooks (R29): the global JSON hook and typed mutators run
//! in order on successful JSON replies only, raw sends and declared errors
//! bypass both, a hook's refusal is a safe `Hook` failure with the original
//! bytes, and a mutator is found by API and endpoint, never by endpoint name
//! alone. The compile errors for mutators on text, binary, and empty
//! endpoints are trybuild fixtures (`compile_fail.rs`).

use std::sync::{Arc, Mutex};

use bytes::Bytes;
use http::Method;
use schematic_runtime::{
    Api, ApiDefinition, Binary, CallOptions, Client, Empty, EndpointSpec, ErrorKind, HookError,
    InvalidArgument, Json, NoDeclaredError, RequestParts, ResponseMeta, Text,
};
use serde::Deserialize;
use wiremock::matchers::path;
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::support::{Call, Item, ProviderError, SENTINEL, TestApi, builder};

/// The order in which hooks ran, with the endpoint and status each saw.
type Log = Arc<Mutex<Vec<String>>>;

fn entries(log: &Log) -> Vec<String> {
    log.lock().expect("log").clone()
}

async fn serve(routes: &[(&str, u16, &'static str, &str)]) -> MockServer {
    let server = MockServer::start().await;
    for (at, status, body, content_type) in routes {
        Mock::given(path(*at))
            .respond_with(ResponseTemplate::new(*status).set_body_raw(*body, content_type))
            .mount(&server)
            .await;
    }
    server
}

fn base(server: &MockServer) -> Client<TestApi> {
    builder(ApiDefinition::new("https://api.example.invalid"), &server.uri())
}

/// A global hook that records each call and unwraps a `{"data": …}`
/// envelope.
fn unwrapping(log: &Log) -> impl Fn(&ResponseMeta, serde_json::Value) -> Result<serde_json::Value, HookError> + use<> {
    let log = Arc::clone(log);
    move |meta, json| {
        log.lock().expect("log").push(format!("global {} {}", meta.request.endpoint_id, meta.status.as_u16()));
        Ok(json.get("data").cloned().unwrap_or(json))
    }
}

#[tokio::test]
async fn a_json_success_runs_parse_then_global_hook_then_decode_then_mutator() {
    let server = serve(&[("/wrapped", 200, r#"{"data":{"id":1}}"#, "application/json")]).await;
    let log: Log = Arc::default();
    let mutator_log = Arc::clone(&log);
    let client = base(&server)
        .pre_response_json(unwrapping(&log))
        .mutate_response::<Call<Json<Item>>, _>(move |meta, item| {
            mutator_log.lock().expect("log").push(format!("mutator {} saw id {}", meta.request.endpoint_id, item.id));
            item.id += 10;
            Ok(())
        })
        .build()
        .expect("client");

    let response = client.send(Call::<Json<Item>>::get("/wrapped"), &CallOptions::new()).await.expect("head");

    assert_eq!(response.data(), Some(&Item { id: 11 }), "the mutator ran on the value decoded from the hook's output");
    assert_eq!(entries(&log), ["global call 200", "mutator call saw id 1"]);
}

#[tokio::test]
async fn each_hook_runs_alone_when_it_is_the_only_one_registered() {
    let server = serve(&[("/wrapped", 200, r#"{"data":{"id":1}}"#, "application/json"), ("/plain", 200, r#"{"id":4}"#, "application/json")]).await;
    let log: Log = Arc::default();
    let global_only = base(&server).pre_response_json(unwrapping(&log)).build().expect("client");
    let response = global_only.send(Call::<Json<Item>>::get("/wrapped"), &CallOptions::new()).await.expect("head");
    assert_eq!(response.data(), Some(&Item { id: 1 }));
    assert_eq!(entries(&log), ["global call 200"]);

    let mutator_only = base(&server)
        .mutate_response::<Call<Json<Item>>, _>(|_, item| {
            item.id *= 2;
            Ok(())
        })
        .build()
        .expect("client");
    let response = mutator_only.send(Call::<Json<Item>>::get("/plain"), &CallOptions::new()).await.expect("head");
    assert_eq!(response.data(), Some(&Item { id: 8 }));

    let none = base(&server).build().expect("client");
    let response = none.send(Call::<Json<Item>>::get("/plain"), &CallOptions::new()).await.expect("head");
    assert_eq!(response.data(), Some(&Item { id: 4 }), "control: no hook, no change");
}

#[tokio::test]
async fn non_2xx_replies_non_json_endpoints_and_raw_sends_run_neither_hook() {
    let server = serve(&[
        ("/error", 404, r#"{"data":{"message":"wrapped"},"message":"top"}"#, "application/json"),
        ("/undeclared", 500, r#"{"data":{"id":1}}"#, "application/json"),
        ("/text", 200, r#"{"data":{"id":1}}"#, "application/json"),
        ("/json", 200, r#"{"data":{"id":1}}"#, "application/json"),
    ])
    .await;
    let log: Log = Arc::default();
    let mutator_runs = Arc::new(Mutex::new(0_u32));
    let counted = Arc::clone(&mutator_runs);
    let client = base(&server)
        .pre_response_json(unwrapping(&log))
        .mutate_response::<Call<Json<Item>, ProviderError>, _>(move |_, _| {
            *counted.lock().expect("count") += 1;
            Ok(())
        })
        .build()
        .expect("client");
    let options = CallOptions::new();

    let declared = client.send(Call::<Json<Item>, ProviderError>::get("/error"), &options).await.expect("head");
    let error = declared.error().expect("failure");
    assert_eq!(
        error.declared(),
        Some(&ProviderError { message: "top".into() }),
        "the declared error decodes from the original body, not a hook's output"
    );
    let undeclared = client.send(Call::<Json<Item>>::get("/undeclared"), &options).await.expect("head");
    assert_eq!(undeclared.error().map(|error| &error.kind), Some(&ErrorKind::Undeclared));

    let text = client.send(Call::<Text>::get("/text"), &options).await.expect("head");
    assert_eq!(text.data().map(String::as_str), Some(r#"{"data":{"id":1}}"#), "a text body is never reshaped");
    let binary = client.send(Call::<Binary>::get("/text"), &options).await.expect("head");
    assert_eq!(binary.data(), Some(&Bytes::from_static(br#"{"data":{"id":1}}"#)));
    let empty = client.send(Call::<Empty>::get("/text"), &options).await.expect("head");
    assert_eq!(empty.data(), Some(&()));

    let raw = client.send_raw(Call::<Json<Item>, ProviderError>::get("/json"), &options).await.expect("head");
    assert_eq!(raw.body, Bytes::from_static(br#"{"data":{"id":1}}"#), "a raw body is never reshaped");

    assert!(entries(&log).is_empty(), "the global hook ran: {:?}", entries(&log));
    assert_eq!(*mutator_runs.lock().expect("count"), 0);

    // Control: the same client runs both hooks on a 2xx JSON reply.
    let typed = client.send(Call::<Json<Item>, ProviderError>::get("/json"), &options).await.expect("head");
    assert_eq!(typed.data(), Some(&Item { id: 1 }));
    assert_eq!(entries(&log), ["global call 200"]);
    assert_eq!(*mutator_runs.lock().expect("count"), 1);
}

#[tokio::test]
async fn a_body_that_is_not_json_is_malformed_and_the_global_hook_never_sees_it() {
    let server = serve(&[("/html", 200, "<html>not json</html>", "application/json")]).await;
    let log: Log = Arc::default();
    let client = base(&server).pre_response_json(unwrapping(&log)).build().expect("client");

    let response = client.send(Call::<Json<Item>>::get("/html"), &CallOptions::new()).await.expect("head");

    let error = response.error().expect("failure");
    assert_eq!(error.kind, ErrorKind::MalformedSuccess);
    assert_eq!(error.body, Bytes::from_static(b"<html>not json</html>"));
    assert!(entries(&log).is_empty());
}

#[tokio::test]
async fn a_hook_output_that_does_not_decode_is_malformed_with_the_original_bytes() {
    let server = serve(&[("/item", 200, r#"{"id":1}"#, "application/json")]).await;
    let client = base(&server)
        .pre_response_json(|_, _| Ok(serde_json::json!({ "id": "not a number" })))
        .build()
        .expect("client");

    let response = client.send(Call::<Json<Item>>::get("/item"), &CallOptions::new()).await.expect("head");

    let error = response.error().expect("failure");
    assert_eq!(error.kind, ErrorKind::MalformedSuccess);
    assert_eq!(error.body, Bytes::from_static(br#"{"id":1}"#), "the bytes as received, not the hook's output");
}

#[tokio::test]
async fn a_refusing_hook_is_a_hook_failure_with_the_original_bytes_and_no_hook_text() {
    const BODY: &str = r#"{"id":3}"#;
    let server = serve(&[("/item", 200, BODY, "application/json")]).await;
    let refusing_global = base(&server)
        .pre_response_json(|_, json| {
            // A conversion from any error discards its text.
            let _: Vec<u8> = serde_json::from_value(json).map_err(|_| std::io::Error::other(SENTINEL))?;
            Ok(serde_json::Value::Null)
        })
        .build()
        .expect("client");
    let refusing_mutator = base(&server)
        .mutate_response::<Call<Json<Item>>, _>(|_, _| Err(HookError::new()))
        .build()
        .expect("client");

    for (label, client) in [("global", refusing_global), ("mutator", refusing_mutator)] {
        let response = client.send(Call::<Json<Item>>::get("/item"), &CallOptions::new()).await.expect("head");
        let error = response.error().expect("failure");
        assert_eq!(error.kind, ErrorKind::Hook, "{label}");
        assert_eq!(error.body, Bytes::from_static(BODY.as_bytes()), "{label}");
        assert_eq!(response.meta.status, 200, "{label}");
        let shown = format!("{response} {:?}", error.kind);
        assert!(shown.contains("response hook rejected the body"), "{label}: {shown}");
        assert!(!shown.contains(SENTINEL), "{label}: {shown}");
    }
    assert_eq!(HookError::from(std::io::Error::other(SENTINEL)).to_string(), "response hook rejected the body");
}

#[tokio::test]
async fn a_mutator_runs_only_for_its_own_endpoint_and_a_second_registration_replaces_the_first() {
    /// Another endpoint of the same API with the same success type.
    struct Other;

    impl EndpointSpec for Other {
        type Api = TestApi;
        type Success = Item;
        type Failure = NoDeclaredError;
        type Decoder = Json<Item>;

        const ENDPOINT_ID: &'static str = "other";
        const METHOD: Method = Method::GET;

        fn into_parts(self) -> Result<RequestParts, InvalidArgument> {
            Ok(RequestParts::new(Method::GET, "/item"))
        }
    }

    let server = serve(&[("/item", 200, r#"{"id":1}"#, "application/json")]).await;
    let client = base(&server)
        .mutate_response::<Call<Json<Item>>, _>(|_, item| {
            item.id = 100;
            Ok(())
        })
        .mutate_response::<Call<Json<Item>>, _>(|_, item| {
            item.id = 200;
            Ok(())
        })
        .build()
        .expect("client");

    let mutated = client.send(Call::<Json<Item>>::get("/item"), &CallOptions::new()).await.expect("head");
    assert_eq!(mutated.data(), Some(&Item { id: 200 }));
    let other = client.send(Other, &CallOptions::new()).await.expect("head");
    assert_eq!(other.data(), Some(&Item { id: 1 }));
}

/// Two APIs that both declare an endpoint `list` with the same success type.
struct Alpha;
struct Beta;

impl Api for Alpha {
    const NAME: &'static str = "Alpha";
}

impl Api for Beta {
    const NAME: &'static str = "Beta";
}

#[derive(Debug, PartialEq, Deserialize)]
struct Listing {
    seen_by: Vec<String>,
}

struct List<A>(std::marker::PhantomData<fn() -> A>);

impl<A: Api> EndpointSpec for List<A> {
    type Api = A;
    type Success = Listing;
    type Failure = NoDeclaredError;
    type Decoder = Json<Listing>;

    const ENDPOINT_ID: &'static str = "list";
    const METHOD: Method = Method::GET;

    fn into_parts(self) -> Result<RequestParts, InvalidArgument> {
        Ok(RequestParts::new(Method::GET, "/list"))
    }
}

fn tagging<A: Api>(server: &MockServer, transport: &schematic_runtime::SharedTransport) -> Client<A> {
    Client::<A>::new(ApiDefinition::new(server.uri()))
        .transport(transport)
        .mutate_response::<List<A>, _>(|_, listing| {
            listing.seen_by.push(A::NAME.to_owned());
            Ok(())
        })
        .build()
        .expect("client")
}

#[tokio::test]
async fn apis_sharing_an_endpoint_id_never_run_each_others_mutators() {
    let server = serve(&[("/list", 200, r#"{"seen_by":[]}"#, "application/json")]).await;
    // One shared transport, so the clients differ only in API and hooks.
    let transport = schematic_runtime::SharedTransport::builder().build().expect("transport");
    let alpha = tagging::<Alpha>(&server, &transport);
    let beta = tagging::<Beta>(&server, &transport);
    let beta_without_hooks = Client::<Beta>::new(ApiDefinition::new(server.uri())).transport(&transport).build().expect("client");

    let from_alpha = alpha.send(List::<Alpha>(std::marker::PhantomData), &CallOptions::new()).await.expect("head");
    let from_beta = beta.send(List::<Beta>(std::marker::PhantomData), &CallOptions::new()).await.expect("head");
    let unhooked = beta_without_hooks.send(List::<Beta>(std::marker::PhantomData), &CallOptions::new()).await.expect("head");

    assert_eq!(from_alpha.data().map(|listing| listing.seen_by.clone()), Some(vec!["Alpha".to_owned()]));
    assert_eq!(from_beta.data().map(|listing| listing.seen_by.clone()), Some(vec!["Beta".to_owned()]));
    assert_eq!(unhooked.data().map(|listing| listing.seen_by.clone()), Some(Vec::new()));
}
