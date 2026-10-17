// OpenAI `ListBatches` authors no backward cursor leg.
use schematic_schema::openai::OpenAI;

fn main() {
    let client = OpenAI::new();
    let _ = client.list_batches().pager().before("b1");
    let _ = client.list_batches().before("b1");
}
