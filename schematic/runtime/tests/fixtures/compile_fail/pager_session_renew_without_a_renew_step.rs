// `renew()` exists only where the definition authors a renew step.
use schematic_runtime::pager::spec::{
    Boundary, CursorLeg, CursorPart, CursorSource, Input, Kind, Session, SessionOpen, Source, Strategy,
};
use schematic_runtime::pager::{CursorKind, ItemSelector, Paginated, WithSession};
use schematic_runtime::{Api, ApiDefinition, Client, EndpointSpec, InvalidArgument, Json, NoDeclaredError, RequestParts};

struct Example;

impl Api for Example {
    const NAME: &'static str = "Example";
}

struct ListItems;

impl EndpointSpec for ListItems {
    type Api = Example;
    type Success = Vec<String>;
    type Failure = NoDeclaredError;
    type Decoder = Json<Vec<String>>;

    const ENDPOINT_ID: &'static str = "list_items";
    const METHOD: http::Method = http::Method::GET;

    fn into_parts(self) -> Result<RequestParts, InvalidArgument> {
        Ok(RequestParts::new(http::Method::GET, "/items"))
    }
}

impl Paginated for ListItems {
    type Item = String;
    type Kind = CursorKind;
    type Session = WithSession;

    const STRATEGY: &'static Strategy = &Strategy {
        kind: Kind::Cursor {
            forward: CursorLeg {
                parts: &[CursorPart {
                    param: Input::Query("after"),
                    source: CursorSource::Field(Source::Body(&["next"])),
                }],
                boundary: Boundary::Exclusive,
            },
            backward: None,
            more: None,
            resumable: false,
        },
        size: None,
        constants: &[],
        total: None,
        terminal_values: &[],
        follow_up: None,
        session: Some(Session {
            open: SessionOpen::FirstPage,
            pages: None,
            handle_from_page: Some(Source::Body(&["next"])),
            send_handle_as: None,
            keep_alive: None,
            renew: None,
            close: None,
        }),
        partition: None,
    };
    const ITEMS: &'static ItemSelector = &ItemSelector::ROOT;
    const PATH: &'static str = "/items";
}

fn main() {
    let client = Client::<Example>::new(ApiDefinition::new("https://x.invalid"));
    let _ = client.send(ListItems).pager().with_session(|scope| {
        Box::pin(async move {
            let _ = scope.renew().await;
        })
    });
}
