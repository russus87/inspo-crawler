mod downloader;
mod model;
mod sources;

use model::{DownloadReq, SaveReport, SearchResponse, SourceError};
use reqwest::Client;
use sources::SourceInfo;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 \
    (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36";

/// Shared application state: a single reused HTTP client.
struct AppState {
    client: Client,
}

/// List the available inspiration sources (id + label) for the UI toggles.
#[tauri::command]
fn list_sources() -> Vec<SourceInfo> {
    sources::list()
}

/// Search the selected sources concurrently and merge their results.
#[tauri::command]
async fn search(
    state: State<'_, AppState>,
    query: String,
    sources: Vec<String>,
    page: u32,
) -> Result<SearchResponse, String> {
    let query = query.trim().to_string();
    if query.is_empty() {
        return Ok(SearchResponse {
            items: vec![],
            errors: vec![],
        });
    }

    let client = state.client.clone();
    let selected: Vec<Box<dyn sources::Source>> = sources::all()
        .into_iter()
        .filter(|s| sources.iter().any(|id| id == s.id()))
        .collect();

    let futures_iter = selected.into_iter().map(|src| {
        let client = client.clone();
        let query = query.clone();
        async move {
            let id = src.id().to_string();
            let label = src.label().to_string();
            match src.search(&client, &query, page).await {
                Ok(items) => (items, None),
                Err(e) => (
                    Vec::new(),
                    Some(SourceError {
                        source: id,
                        source_label: label,
                        message: e.to_string(),
                    }),
                ),
            }
        }
    });

    let results = futures::future::join_all(futures_iter).await;

    let mut items = Vec::new();
    let mut errors = Vec::new();
    for (mut its, err) in results {
        items.append(&mut its);
        if let Some(e) = err {
            errors.push(e);
        }
    }

    Ok(SearchResponse { items, errors })
}

/// Open the native folder picker; returns the chosen path or `None`.
#[tauri::command]
async fn pick_folder(app: AppHandle) -> Result<Option<String>, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog().file().pick_folder(move |folder| {
        let _ = tx.send(folder);
    });
    let picked = rx.await.map_err(|e| e.to_string())?;
    Ok(picked.map(|p| p.to_string()))
}

/// Download the given images into `folder` (created if missing).
#[tauri::command]
async fn save_images(
    state: State<'_, AppState>,
    items: Vec<DownloadReq>,
    folder: String,
) -> Result<SaveReport, String> {
    if folder.trim().is_empty() {
        return Err("No destination folder selected".into());
    }
    let client = state.client.clone();
    let report = downloader::save_all(&client, items, folder.into()).await;
    Ok(report)
}

/// Open a URL in the user's default browser.
#[tauri::command]
fn open_external(app: AppHandle, url: String) -> Result<(), String> {
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let client = Client::builder()
        .user_agent(USER_AGENT)
        .gzip(true)
        .brotli(true)
        .timeout(std::time::Duration::from_secs(25))
        .build()
        .expect("failed to build HTTP client");

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            app.manage(AppState { client });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_sources,
            search,
            pick_folder,
            save_images,
            open_external
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
