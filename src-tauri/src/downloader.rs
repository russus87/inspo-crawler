use crate::model::{DownloadReq, SaveOutcome, SaveReport};
use crate::sources;
use futures::stream::{self, StreamExt};
use reqwest::Client;
use std::path::{Path, PathBuf};

const MAX_CONCURRENT: usize = 6;

/// Turn arbitrary text into a filesystem-safe fragment.
fn sanitize(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => c,
            ' ' | '/' | '\\' | ':' | '.' => '-',
            _ => '-',
        })
        .collect();
    let trimmed = cleaned.trim_matches('-');
    let collapsed = trimmed
        .split('-')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    collapsed.chars().take(60).collect()
}

/// Guess a file extension from a content-type, falling back to the URL, then jpg.
fn guess_ext(content_type: Option<&str>, url: &str) -> &'static str {
    if let Some(ct) = content_type {
        let ct = ct.to_ascii_lowercase();
        if ct.contains("png") {
            return "png";
        } else if ct.contains("webp") {
            return "webp";
        } else if ct.contains("gif") {
            return "gif";
        } else if ct.contains("svg") {
            return "svg";
        } else if ct.contains("jpeg") || ct.contains("jpg") {
            return "jpg";
        }
    }
    let lower = url.split('?').next().unwrap_or(url).to_ascii_lowercase();
    for ext in ["png", "webp", "gif", "svg", "jpeg", "jpg"] {
        if lower.ends_with(&format!(".{ext}")) {
            return if ext == "jpeg" { "jpg" } else { ext_static(ext) };
        }
    }
    "jpg"
}

fn ext_static(ext: &str) -> &'static str {
    match ext {
        "png" => "png",
        "webp" => "webp",
        "gif" => "gif",
        "svg" => "svg",
        _ => "jpg",
    }
}

async fn download_one(client: &Client, req: &DownloadReq, folder: &Path) -> SaveOutcome {
    let referer = sources::referer_for(&req.source);
    let resp = client
        .get(&req.url)
        .header("Referer", referer)
        .header("Accept", "image/avif,image/webp,image/png,image/*,*/*")
        .send()
        .await
        .and_then(|r| r.error_for_status());

    let resp = match resp {
        Ok(r) => r,
        Err(e) => {
            return SaveOutcome {
                id: req.id.clone(),
                ok: false,
                detail: format!("download failed: {e}"),
            }
        }
    };

    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let ext = guess_ext(content_type.as_deref(), &req.url);

    let bytes = match resp.bytes().await {
        Ok(b) => b,
        Err(e) => {
            return SaveOutcome {
                id: req.id.clone(),
                ok: false,
                detail: format!("read failed: {e}"),
            }
        }
    };

    let base = {
        let t = sanitize(&req.title);
        if t.is_empty() {
            req.source.clone()
        } else {
            t
        }
    };
    let id_part = sanitize(&req.id);
    let file_name = if id_part.is_empty() {
        format!("{base}-{}.{ext}", &req.source)
    } else {
        format!("{base}-{}-{id_part}.{ext}", &req.source)
    };
    let path: PathBuf = folder.join(file_name);

    match tokio::fs::write(&path, &bytes).await {
        Ok(_) => SaveOutcome {
            id: req.id.clone(),
            ok: true,
            detail: path.to_string_lossy().to_string(),
        },
        Err(e) => SaveOutcome {
            id: req.id.clone(),
            ok: false,
            detail: format!("write failed: {e}"),
        },
    }
}

/// Download every requested image into `folder`, limited concurrency.
pub async fn save_all(client: &Client, reqs: Vec<DownloadReq>, folder: PathBuf) -> SaveReport {
    if let Err(e) = tokio::fs::create_dir_all(&folder).await {
        return SaveReport {
            saved: 0,
            failed: reqs.len(),
            folder: folder.to_string_lossy().to_string(),
            outcomes: vec![SaveOutcome {
                id: String::new(),
                ok: false,
                detail: format!("cannot create folder: {e}"),
            }],
        };
    }

    let folder_ref = folder.as_path();
    let outcomes: Vec<SaveOutcome> = stream::iter(reqs)
        .map(|req| async move { download_one(client, &req, folder_ref).await })
        .buffer_unordered(MAX_CONCURRENT)
        .collect()
        .await;

    let saved = outcomes.iter().filter(|o| o.ok).count();
    let failed = outcomes.len() - saved;
    SaveReport {
        saved,
        failed,
        folder: folder.to_string_lossy().to_string(),
        outcomes,
    }
}
