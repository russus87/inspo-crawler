use super::Source;
use crate::model::InspoItem;
use async_trait::async_trait;
use reqwest::Client;
use scraper::{Html, Selector};

/// Behance via HTML scraping of the public project search.
///
/// Best-effort: like Dribbble, Behance uses bot protection and dynamic markup,
/// so results are not guaranteed. Isolated behind the `Source` trait.
pub struct Behance;

fn best_from_srcset(srcset: &str) -> Option<String> {
    srcset
        .split(',')
        .filter_map(|part| part.split_whitespace().next())
        .rfind(|u| u.starts_with("http"))
        .map(|s| s.to_string())
}

/// Extract the numeric gallery id from a `/gallery/<id>/<slug>` href.
fn gallery_id(href: &str) -> Option<String> {
    let after = href.split("/gallery/").nth(1)?;
    let id = after.split('/').next()?;
    if id.chars().all(|c| c.is_ascii_digit()) && !id.is_empty() {
        Some(id.to_string())
    } else {
        None
    }
}

#[async_trait]
impl Source for Behance {
    fn id(&self) -> &'static str {
        "behance"
    }
    fn label(&self) -> &'static str {
        "Behance"
    }
    fn referer(&self) -> &'static str {
        "https://www.behance.net/"
    }

    async fn search(&self, client: &Client, query: &str, page: u32) -> anyhow::Result<Vec<InspoItem>> {
        let url = format!(
            "https://www.behance.net/search/projects?search={}&page={}",
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
        let card_sel = Selector::parse("a[href*='/gallery/']").unwrap();
        let img_sel = Selector::parse("img").unwrap();

        let mut out = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for a in doc.select(&card_sel) {
            let href = match a.value().attr("href") {
                Some(h) => h,
                None => continue,
            };
            let Some(id) = gallery_id(href) else { continue };
            if !seen.insert(id.clone()) {
                continue;
            }
            let img = match a.select(&img_sel).next() {
                Some(i) => i,
                None => continue,
            };
            let thumbnail = img
                .value()
                .attr("srcset")
                .and_then(best_from_srcset)
                .or_else(|| img.value().attr("src").map(|s| s.to_string()));
            let Some(thumbnail) = thumbnail.filter(|u| u.starts_with("http")) else {
                continue;
            };
            let title = img
                .value()
                .attr("alt")
                .or_else(|| a.value().attr("title"))
                .unwrap_or_default()
                .trim()
                .to_string();
            let link = if href.starts_with("http") {
                href.to_string()
            } else {
                format!("https://www.behance.net{href}")
            };

            out.push(InspoItem {
                id,
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
