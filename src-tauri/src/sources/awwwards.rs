use super::Source;
use crate::model::InspoItem;
use async_trait::async_trait;
use reqwest::Client;
use scraper::{Html, Selector};

/// Awwwards via HTML scraping of the website search — award-winning web/UI
/// design, very on-theme. Best-effort: markup and bot protection may change.
pub struct Awwwards;

#[async_trait]
impl Source for Awwwards {
    fn id(&self) -> &'static str {
        "awwwards"
    }
    fn label(&self) -> &'static str {
        "Awwwards"
    }
    fn referer(&self) -> &'static str {
        "https://www.awwwards.com/"
    }

    async fn search(&self, client: &Client, query: &str, page: u32) -> anyhow::Result<Vec<InspoItem>> {
        let url = format!(
            "https://www.awwwards.com/websites/?text={}&page={}",
            urlencoding::encode(query),
            page.max(1)
        );
        let html = client
            .get(&url)
            .header("Accept", "text/html,application/xhtml+xml")
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        let doc = Html::parse_document(&html);
        // Each award card carries a figure with the preview image.
        let card_sel = Selector::parse("li.js-collectable, div.card-site, figure").unwrap();
        let img_sel = Selector::parse("img").unwrap();
        let link_sel = Selector::parse("a[href*='/sites/'], a[href*='/inspiration/']").unwrap();

        let mut out = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for card in doc.select(&card_sel) {
            let img = match card.select(&img_sel).next() {
                Some(i) => i,
                None => continue,
            };
            let thumbnail = img
                .value()
                .attr("data-src")
                .or_else(|| img.value().attr("src"))
                .map(|s| s.to_string())
                .filter(|u| u.starts_with("http"));
            let Some(thumbnail) = thumbnail else { continue };
            if !seen.insert(thumbnail.clone()) {
                continue;
            }

            let (link, id) = match card
                .select(&link_sel)
                .next()
                .and_then(|a| a.value().attr("href"))
            {
                Some(href) => {
                    let full = if href.starts_with("http") {
                        href.to_string()
                    } else {
                        format!("https://www.awwwards.com{href}")
                    };
                    let id = href.trim_end_matches('/').rsplit('/').next().unwrap_or("").to_string();
                    (full, id)
                }
                None => (url.clone(), thumbnail.clone()),
            };

            let title = img.value().attr("alt").unwrap_or_default().trim().to_string();

            out.push(InspoItem {
                id: if id.is_empty() { thumbnail.clone() } else { id },
                source: self.id().to_string(),
                source_label: self.label().to_string(),
                title,
                author: String::new(),
                full: thumbnail.clone(),
                thumbnail,
                link,
            });
        }
        Ok(out)
    }
}
