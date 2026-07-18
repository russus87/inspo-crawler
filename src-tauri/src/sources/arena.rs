use super::Source;
use crate::model::InspoItem;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;

/// Are.na via its public v2 API (no auth for public content).
/// A curator-heavy design community — reliable source.
pub struct Arena;

#[async_trait]
impl Source for Arena {
    fn id(&self) -> &'static str {
        "arena"
    }
    fn label(&self) -> &'static str {
        "Are.na"
    }
    fn referer(&self) -> &'static str {
        "https://www.are.na/"
    }

    async fn search(&self, client: &Client, query: &str, page: u32) -> anyhow::Result<Vec<InspoItem>> {
        let url = format!(
            "https://api.are.na/v2/search/blocks?q={}&per=32&page={}",
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
        if let Some(blocks) = v.get("blocks").and_then(Value::as_array) {
            for b in blocks {
                // Only image blocks are useful here.
                if b.get("class").and_then(Value::as_str) != Some("Image") {
                    continue;
                }
                let id = b
                    .get("id")
                    .map(|x| x.to_string())
                    .unwrap_or_default();
                let thumbnail = b
                    .pointer("/image/display/url")
                    .or_else(|| b.pointer("/image/thumb/url"))
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let full = b
                    .pointer("/image/original/url")
                    .or_else(|| b.pointer("/image/large/url"))
                    .and_then(Value::as_str)
                    .unwrap_or(thumbnail);
                if thumbnail.is_empty() || id.is_empty() {
                    continue;
                }
                let title = b
                    .get("title")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let author = b
                    .pointer("/user/full_name")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let link = format!("https://www.are.na/block/{id}");

                out.push(InspoItem {
                    id,
                    source: self.id().to_string(),
                    source_label: self.label().to_string(),
                    title: title.to_string(),
                    author: author.to_string(),
                    thumbnail: thumbnail.to_string(),
                    full: full.to_string(),
                    link,
                });
            }
        }
        Ok(out)
    }
}
