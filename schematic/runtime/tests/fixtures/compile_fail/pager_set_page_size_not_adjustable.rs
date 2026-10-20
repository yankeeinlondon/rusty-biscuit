// `Pages::set_page_size(..)` needs an adjustable page size; a fixed one is not enough.
use schematic_runtime::pager::spec::{Boundary, CursorLeg, CursorPart, CursorSource, Input, Kind, Source, Strategy};
use schematic_runtime::pager::{CursorKind, ItemSelector, NoSession, PageSizeKnob, Paginated};
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

impl PageSizeKnob for ListItems {}

fn main() {
    let client = Client::<Example>::new(ApiDefinition::new("https://x.invalid"));
    let mut pages = client.send(ListItems).pager().page_size(5).pages();
    pages.set_page_size(10);
}
