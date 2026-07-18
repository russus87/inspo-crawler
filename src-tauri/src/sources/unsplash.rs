use super::Source;
use crate::model::InspoItem;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;

/// Unsplash via its internal `napi` endpoint (no API key required).
/// This is one of the more reliable sources.
pub struct Unsplash;

#[async_trait]
impl Source for Unsplash {
    fn id(&self) -> &'static str {
        "unsplash"
    }
    fn label(&self) -> &'static str {
        "Unsplash"
    }
    fn referer(&self) -> &'static str {
        "https://unsplash.com/"
    }

    async fn search(&self, client: &Client, query: &str, page: u32) -> anyhow::Result<Vec<InspoItem>> {
        let url = format!(
            "https://unsplash.com/napi/search/photos?query={}&per_page=30&page={}",
            urlencoding::encode(query),
            page.max(1)
        );
        let v: Value = client
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let mut out = Vec::new();
        if let Some(results) = v.get("results").and_then(Value::as_array) {
            for r in results {
                let id = r.get("id").and_then(Value::as_str).unwrap_or_default();
                if id.is_empty() {
                    continue;
                }
                let thumbnail = r
                    .pointer("/urls/small")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let full = r
                    .pointer("/urls/full")
                    .or_else(|| r.pointer("/urls/raw"))
                    .and_then(Value::as_str)
                    .unwrap_or(thumbnail);
                if thumbnail.is_empty() {
                    continue;
                }
                let title = r
                    .get("alt_description")
                    .or_else(|| r.get("description"))
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let author = r
                    .pointer("/user/name")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let link = r
                    .pointer("/links/html")
                    .and_then(Value::as_str)
                    .unwrap_or("https://unsplash.com/");

                out.push(InspoItem {
                    id: id.to_string(),
                    source: self.id().to_string(),
                    source_label: self.label().to_string(),
                    title: title.to_string(),
                    author: author.to_string(),
                    thumbnail: thumbnail.to_string(),
                    full: full.to_string(),
                    link: link.to_string(),
                });
            }
        }
        Ok(out)
    }
}
