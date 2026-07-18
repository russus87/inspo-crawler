use super::Source;
use crate::model::InspoItem;
use async_trait::async_trait;
use reqwest::Client;
use scraper::{Html, Selector};

/// Dribbble via HTML scraping of the public shot search.
///
/// Best-effort: Dribbble sits behind bot protection and changes its markup,
/// so this may return nothing. The trait-based design keeps it easy to fix.
pub struct Dribbble;

/// Pick the largest candidate from a `srcset` attribute.
fn best_from_srcset(srcset: &str) -> Option<String> {
    srcset
        .split(',')
        .filter_map(|part| part.split_whitespace().next())
        .rfind(|u| u.starts_with("http"))
        .map(|s| s.to_string())
}

#[async_trait]
impl Source for Dribbble {
    fn id(&self) -> &'static str {
        "dribbble"
    }
    fn label(&self) -> &'static str {
        "Dribbble"
    }
    fn referer(&self) -> &'static str {
        "https://dribbble.com/"
    }

    async fn search(&self, client: &Client, query: &str, page: u32) -> anyhow::Result<Vec<InspoItem>> {
        let url = format!(
            "https://dribbble.com/search/shots?q={}&page={}",
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
        let li_sel = Selector::parse("li.shot-thumbnail, li[id^=screenshot]").unwrap();
        let img_sel = Selector::parse("img").unwrap();
        let link_sel = Selector::parse("a.shot-thumbnail-link, a[href*='/shots/']").unwrap();

        let mut out = Vec::new();
        for li in doc.select(&li_sel) {
            let img = match li.select(&img_sel).next() {
                Some(i) => i,
                None => continue,
            };
            let thumbnail = img
                .value()
                .attr("data-src")
                .map(|s| s.to_string())
                .or_else(|| img.value().attr("srcset").and_then(best_from_srcset))
                .or_else(|| img.value().attr("src").map(|s| s.to_string()));
            let Some(thumbnail) = thumbnail.filter(|u| u.starts_with("http")) else {
                continue;
            };

            let (link, id) = match li.select(&link_sel).next().and_then(|a| a.value().attr("href")) {
                Some(href) => {
                    let full_link = if href.starts_with("http") {
                        href.to_string()
                    } else {
                        format!("https://dribbble.com{href}")
                    };
                    let id = href.trim_end_matches('/').rsplit('/').next().unwrap_or("").to_string();
                    (full_link, id)
                }
                None => (url.clone(), thumbnail.clone()),
            };

            let title = img
                .value()
                .attr("alt")
                .unwrap_or_default()
                .trim()
                .to_string();

            out.push(InspoItem {
                id: if id.is_empty() { thumbnail.clone() } else { id },
                source: self.id().to_string(),
                source_label: self.label().to_string(),
                title,
                author: String::new(),
                // Dribbble thumbnails are already fairly large; strip resize
                // suffix to get a bigger image when possible.
                full: thumbnail.replace("_1x1", "").clone(),
                thumbnail,
                link,
            });
        }
        Ok(out)
    }
}
