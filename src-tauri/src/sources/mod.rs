use crate::model::InspoItem;
use async_trait::async_trait;
use reqwest::Client;

pub mod arena;
pub mod awwwards;
pub mod behance;
pub mod dribbble;
pub mod lexica;
pub mod pinterest;
pub mod unsplash;

/// A pluggable inspiration source. Add a new provider by implementing this
/// trait and registering it in [`all`].
#[async_trait]
pub trait Source: Send + Sync {
    /// Stable machine id, e.g. "pinterest".
    fn id(&self) -> &'static str;
    /// Human-facing label, e.g. "Pinterest".
    fn label(&self) -> &'static str;
    /// Referer sent when downloading images from this source (hotlink bypass).
    fn referer(&self) -> &'static str;
    /// Run a search. `page` is 1-based; sources that don't paginate ignore it.
    async fn search(&self, client: &Client, query: &str, page: u32) -> anyhow::Result<Vec<InspoItem>>;
}

/// Every source known to the app, in display order.
pub fn all() -> Vec<Box<dyn Source>> {
    vec![
        Box::new(pinterest::Pinterest),
        Box::new(dribbble::Dribbble),
        Box::new(behance::Behance),
        Box::new(awwwards::Awwwards),
        Box::new(unsplash::Unsplash),
        Box::new(arena::Arena),
        Box::new(lexica::Lexica),
    ]
}

/// Look up the referer for a source id (used by the downloader).
pub fn referer_for(source_id: &str) -> String {
    all()
        .into_iter()
        .find(|s| s.id() == source_id)
        .map(|s| s.referer().to_string())
        .unwrap_or_default()
}

/// Lightweight descriptor sent to the UI so it can render source toggles.
#[derive(serde::Serialize)]
pub struct SourceInfo {
    pub id: String,
    pub label: String,
}

pub fn list() -> Vec<SourceInfo> {
    all()
        .iter()
        .map(|s| SourceInfo {
            id: s.id().to_string(),
            label: s.label().to_string(),
        })
        .collect()
}
