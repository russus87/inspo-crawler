// Ad-hoc probe: run every source for page 1 and page 2 with a real network
// and report how many NEW items page 2 adds. Not part of the app.
use inspo_crawler_lib::probe_sources;

#[tokio::main]
async fn main() {
    let query = std::env::args().nth(1).unwrap_or_else(|| "dashboard ui".into());
    probe_sources(&query).await;
}
