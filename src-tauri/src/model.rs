use serde::{Deserialize, Serialize};

/// A single inspiration result shown in the grid.
#[derive(Debug, Clone, Serialize)]
pub struct InspoItem {
    /// Stable id inside its source (used together with `source` as a grid key).
    pub id: String,
    /// Source id, e.g. "pinterest".
    pub source: String,
    /// Human label for the source, e.g. "Pinterest".
    pub source_label: String,
    /// Short title / description (may be empty).
    pub title: String,
    /// Author / uploader name (may be empty).
    pub author: String,
    /// URL used to render the thumbnail in the grid.
    pub thumbnail: String,
    /// Best-quality URL used when the image is saved to disk.
    pub full: String,
    /// Web page for this item, opened in the system browser on click.
    pub link: String,
}

/// A request from the UI to download one image.
#[derive(Debug, Clone, Deserialize)]
pub struct DownloadReq {
    pub url: String,
    pub source: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub id: String,
}

/// Per-source failure surfaced to the UI so the user knows what broke.
#[derive(Debug, Clone, Serialize)]
pub struct SourceError {
    pub source: String,
    pub source_label: String,
    pub message: String,
}

/// Result of a `search` invocation.
#[derive(Debug, Clone, Serialize)]
pub struct SearchResponse {
    pub items: Vec<InspoItem>,
    pub errors: Vec<SourceError>,
}

/// One saved-or-failed entry from a `save_images` invocation.
#[derive(Debug, Clone, Serialize)]
pub struct SaveOutcome {
    pub id: String,
    pub ok: bool,
    /// Saved file path on success, error message on failure.
    pub detail: String,
}

/// Result of a `save_images` invocation.
#[derive(Debug, Clone, Serialize)]
pub struct SaveReport {
    pub saved: usize,
    pub failed: usize,
    pub folder: String,
    pub outcomes: Vec<SaveOutcome>,
}
