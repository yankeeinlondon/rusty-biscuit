// `.page(n)` exists only for page-number strategies.
use schematic_runtime::pager::spec::{Boundary, CursorLeg, CursorPart, CursorSource, Input, Kind, Source, Strategy};
use schematic_runtime::pager::{CursorKind, ItemSelector, NoSession, Paginated};
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
    type Session = NoSession;

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
        session: None,
        partition: None,
    };
    const ITEMS: &'static ItemSelector = &ItemSelector::ROOT;
    const PATH: &'static str = "/items";
}

fn main() {
    let client = Client::<Example>::new(ApiDefinition::new("https://x.invalid"));
    let _ = client.send(ListItems).pager().page(2);
}
