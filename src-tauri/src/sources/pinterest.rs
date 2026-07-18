use super::Source;
use crate::model::InspoItem;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};

/// Pinterest via its internal `BaseSearchResource` endpoint (unauthenticated).
pub struct Pinterest;

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

    async fn search(&self, client: &Client, query: &str, _page: u32) -> anyhow::Result<Vec<InspoItem>> {
        // Pinterest paginates with opaque bookmarks rather than page numbers,
        // so we fetch the first page of results (25 pins).
        let data = json!({
            "options": { "query": query, "scope": "pins", "page_size": 25 },
            "context": {}
        })
        .to_string();

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
