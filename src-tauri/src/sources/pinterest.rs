use super::Source;
use crate::model::InspoItem;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};

/// Pinterest via its internal `BaseSearchResource` endpoint (unauthenticated).
pub struct Pinterest;

impl Pinterest {
    /// Fetch one batch of results, optionally continuing from `bookmark`.
    /// Returns the raw result array and the next-page bookmark (if any).
    async fn fetch_page(
        &self,
        client: &Client,
        query: &str,
        bookmark: Option<&str>,
    ) -> anyhow::Result<(Vec<Value>, Option<String>)> {
        let mut options = json!({ "query": query, "scope": "pins", "page_size": 25 });
        if let Some(b) = bookmark {
            options["bookmarks"] = json!([b]);
        }
        let data = json!({ "options": options, "context": {} }).to_string();

        let source_url = format!("/search/pins/?q={}", urlencoding::encode(query));
        let url = format!(
            "https://www.pinterest.com/resource/BaseSearchResource/get/?source_url={}&data={}",
            urlencoding::encode(&source_url),
            urlencoding::encode(&data)
        );

        let v: Value = client
            .get(&url)
            .header("Accept", "application/json, text/javascript, */*, q=0.01")
            .header("X-Requested-With", "XMLHttpRequest")
            .header("X-APP-VERSION", "0e2d9c1")
            .header("X-Pinterest-PWS-Handler", "www/search/[scope].js")
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let results = v
            .pointer("/resource_response/data/results")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let next = v
            .pointer("/resource_response/bookmark")
            .and_then(Value::as_str)
            .map(|s| s.to_string());
        Ok((results, next))
    }
}

/// Pick a thumbnail and a full-size url out of a Pinterest `images` object,
/// which looks like `{ "236x": {"url": ...}, "orig": {"url": ...}, ... }`.
fn pick_images(images: &Value) -> Option<(String, String)> {
    let get = |k: &str| images.pointer(&format!("/{k}/url")).and_then(Value::as_str);
    let thumb = get("474x").or_else(|| get("236x")).or_else(|| get("736x"));
    let full = get("orig").or_else(|| get("736x")).or(thumb);
    match (thumb, full) {
        (Some(t), Some(f)) => Some((t.to_string(), f.to_string())),
        _ => None,
    }
}

#[async_trait]
impl Source for Pinterest {
    fn id(&self) -> &'static str {
        "pinterest"
    }
    fn label(&self) -> &'static str {
        "Pinterest"
    }
    fn referer(&self) -> &'static str {
        "https://www.pinterest.com/"
    }

    async fn search(&self, client: &Client, query: &str, page: u32) -> anyhow::Result<Vec<InspoItem>> {
        // Pinterest paginates with opaque "bookmarks", not page numbers: each
        // response carries a bookmark that must be replayed to fetch the next
        // batch. Sources are stateless per invoke, so to reach page N we chain
        // N sequential requests, keeping only the last batch's results.
        let mut bookmark: Option<String> = None;
        let mut results = Vec::new();
        for _ in 0..page.max(1) {
            let (batch, next) = self.fetch_page(client, query, bookmark.as_deref()).await?;
            results = batch;
            match next {
                // A "-end-" bookmark (or none) means no further pages exist.
                Some(b) if b != "-end-" => bookmark = Some(b),
                _ => break,
            }
        }

        let mut out = Vec::new();
        for r in &results {
            let id = r.get("id").and_then(Value::as_str).unwrap_or_default();
            if id.is_empty() {
                continue;
            }
            let Some((thumbnail, full)) = r.get("images").and_then(pick_images) else {
                continue;
            };
            let title = r
                .get("grid_title")
                .or_else(|| r.get("title"))
                .and_then(Value::as_str)
                .unwrap_or_default();
            let author = r
                .pointer("/pinner/full_name")
                .or_else(|| r.pointer("/pinner/username"))
                .and_then(Value::as_str)
                .unwrap_or_default();

            out.push(InspoItem {
                id: id.to_string(),
                source: self.id().to_string(),
                source_label: self.label().to_string(),
                title: title.to_string(),
                author: author.to_string(),
                thumbnail,
                full,
                link: format!("https://www.pinterest.com/pin/{id}/"),
            });
        }
        Ok(out)
    }
}
