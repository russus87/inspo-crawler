use super::Source;
use crate::model::InspoItem;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;

/// Lexica via its public search API. AI-generated imagery — broad visual
/// inspiration rather than production UI, but reliable and keyword-searchable.
pub struct Lexica;

#[async_trait]
impl Source for Lexica {
    fn id(&self) -> &'static str {
        "lexica"
    }
    fn label(&self) -> &'static str {
        "Lexica"
    }
    fn referer(&self) -> &'static str {
        "https://lexica.art/"
    }

    async fn search(&self, client: &Client, query: &str, page: u32) -> anyhow::Result<Vec<InspoItem>> {
        // The endpoint returns a single (large) result set with no paging.
        if page > 1 {
            return Ok(Vec::new());
        }
        let url = format!("https://lexica.art/api/v1/search?q={}", urlencoding::encode(query));
        let v: Value = client
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let mut out = Vec::new();
        if let Some(images) = v.get("images").and_then(Value::as_array) {
            for img in images {
                let id = img.get("id").and_then(Value::as_str).unwrap_or_default();
                let full = img.get("src").and_then(Value::as_str).unwrap_or_default();
                if id.is_empty() || full.is_empty() {
                    continue;
                }
                let thumbnail = img
                    .get("srcSmall")
                    .and_then(Value::as_str)
                    .unwrap_or(full);
                let title = img
                    .get("prompt")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .chars()
                    .take(80)
                    .collect::<String>();
                let link = match img.get("promptid").and_then(Value::as_str) {
                    Some(pid) if !pid.is_empty() => format!("https://lexica.art/prompt/{pid}"),
                    _ => full.to_string(),
                };

                out.push(InspoItem {
                    id: id.to_string(),
                    source: self.id().to_string(),
                    source_label: self.label().to_string(),
                    title,
                    author: String::new(),
                    thumbnail: thumbnail.to_string(),
                    full: full.to_string(),
                    link,
                });
            }
        }
        Ok(out)
    }
}
