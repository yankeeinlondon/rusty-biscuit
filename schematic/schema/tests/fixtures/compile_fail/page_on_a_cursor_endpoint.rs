// OpenAI `ListBatches` authors a cursor: `.page(n)` does not exist on it.
use schematic_schema::openai::OpenAI;

fn main() {
    let client = OpenAI::new();
    let _ = client.list_batches().pager().page(2);
    let _ = client.list_batches().page(2);
}
