// Every knob and accessor the shipped definitions author, used through the
// generated call types; the compile-fail cases beside it differ from this
// only in the capability they ask for.
use schematic_schema::bitbucket::Bitbucket;
use schematic_schema::github::{GitHub, ListReleasesRequest, Release};
use schematic_schema::openai::OpenAI;

async fn used(github: GitHub, openai: OpenAI, bitbucket: Bitbucket) {
    let _: Result<Vec<Release>, _> = github.list_releases("octo", "widgets").page(2).page_size(50).collect_all().await;
    let _ = github.send(ListReleasesRequest::new("octo", "widgets")).pager().page_limit(3).fetch_page().await;
    let mut runs = github.list_workflow_runs("octo", "widgets").page_size(5).pages();
    let _ = runs.next().await;
    let _: Option<u64> = runs.total();

    let _ = openai.list_batches().after("b1").page_size(10).fetch_page().await;
    let _ = openai.list_audit_logs().before("a9").page_size(5).collect_all().await;

    let mut tags = bitbucket.list_tags("team", "widgets").page_size(2).pages();
    let _ = tags.next().await;
    let _ = tags.prev().await;
    let _: Option<u64> = tags.total();
    let _ = bitbucket.list_tags("team", "widgets").pages().items();
}

fn main() {
    // Built, not run: the control proves the surface compiles.
    let _ = used;
}
