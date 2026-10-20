// Configuration happens on the pager: `Pages` has no knob, even an authored one.
use schematic_runtime::pager::spec::{IndexBase, Input, Kind, MoreSignal, Strategy};
use schematic_runtime::pager::{ItemSelector, NoSession, PageNumberKind, PageSizeKnob, Paginated};
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
    type Kind = PageNumberKind;
    type Session = NoSession;

    const STRATEGY: &'static Strategy = &Strategy {
        kind: Kind::PageNumber {
            page: Input::Query("page"),
            base: IndexBase::OneBased,
            more: MoreSignal::NonEmptyPage,
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
    let _ = client.send(ListItems).pager().pages().page_size(10);
    let _ = client.send(ListItems).pager().pages().page(2);
}
