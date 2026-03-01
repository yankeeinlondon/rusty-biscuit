//! Search command - searches HuggingFace for GGUF models.

use crate::output::print_search_results;
use color_eyre::eyre::Result;
use model_citizen::SortOrder;
use model_citizen::huggingface::HuggingFaceClient;

pub async fn run(
    query: Option<&str>,
    limit: usize,
    sort: SortOrder,
    json_output: bool,
    verbose: bool,
) -> Result<()> {
    let client = HuggingFaceClient::new();

    match query {
        Some(_) => println!(),
        None => println!("Browsing top models by {}...", sort.display_label()),
    }

    let results = client.search_models(query, limit, sort).await?;

    print_search_results(&results, query, sort, json_output, verbose)
}
