# Inspo Crawler

A small desktop app to **hunt UI / design inspiration** across the web and save
the images you like. Search a theme, browse a live masonry grid pulled from
several design sources at once, then download shots **one by one** or **in bulk**
with a checkbox on every image.

Built with **Rust + Tauri v2** (native, tiny binary, no Electron). The frontend
is plain HTML/CSS/JS — no bundler, no Node required to build.

> Made for my own workflow (Anvil + new projects), but free to use.

## Sources

| Source        | How it's fetched                        | Reliability |
|---------------|-----------------------------------------|-------------|
| **Pinterest** | internal search resource (no login)     | good        |
| **Unsplash**  | internal `napi` search (no API key)     | good        |
| **Are.na**    | public v2 API                           | good        |
| **Lexica**    | public search API (AI imagery)          | good        |
| **Dribbble**  | HTML scraping of shot search            | best-effort |
| **Behance**   | HTML scraping of project search         | best-effort |
| **Awwwards**  | HTML scraping of website search         | best-effort |

Each source is a self-contained module implementing a single `Source` trait
(`src-tauri/src/sources/`), so adding a new one — or fixing one that a site
breaks — is a ~80-line file. Dribbble and Behance sit behind bot protection and
change their markup often; if one stops returning results the app shows a small
`⚠ <source> failed` note and the others keep working.

## Features

- 🔎 Search any theme across all enabled sources at once (concurrent).
- 🧩 Toggle sources on/off with chips.
- 🖼️ Masonry grid; click any image to open its page in your browser.
- ☑️ Per-image checkbox + **Select all** for bulk selection.
- ⤓ Save a single image, or **Save selected (N)** in one go.
- 📁 Pick any destination folder (native dialog). Downloads run server-side in
  Rust with a proper `Referer`, so images that a browser would hotlink-block
  still save fine.
- ➕ **Load more** to pull the next page (where the source supports paging).

## Install / build from source

Requires the [Rust toolchain](https://rustup.rs) and the Tauri v2 system
dependencies for your OS
([Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)).

```bash
# one-time: the Tauri CLI
cargo install tauri-cli --version '^2.0' --locked

# run in dev
cargo tauri dev

# build a release bundle for your platform
cargo tauri build
```

On Arch Linux the runtime deps are:

```bash
sudo pacman -S --needed webkit2gtk-4.1 gtk3 libappindicator-gtk3 librsvg
```

## Prebuilt downloads

Every tagged release (`vX.Y.Z`) publishes installers via GitHub Actions:

- **Windows** — `.msi` and NSIS `.exe`
- **macOS** — universal `.dmg` / `.app` (Apple Silicon + Intel)
- **Linux** — `.deb`, `.rpm`, `.AppImage`
- **Arch Linux** — native `x86_64` binary tarball

To cut a release:

```bash
git tag v0.1.0 && git push origin v0.1.0
```

The workflow (`.github/workflows/build.yml`) also runs `clippy` on every push/PR.

## Project layout

```
src/                     # static frontend (index.html, styles.css, main.js)
src-tauri/
  src/
    lib.rs               # Tauri commands: search, save_images, pick_folder, ...
    model.rs             # shared data types
    downloader.rs        # concurrent image download + save
    sources/             # one file per inspiration source (Source trait)
  tauri.conf.json
.github/workflows/build.yml
```

## Notes & etiquette

This tool scrapes public search pages for **personal inspiration gathering**.
Respect each site's terms of service and the rights of the original creators —
always credit and link back (click any image to open its source page). It does
not bypass logins or paywalls.

## License

MIT — see [LICENSE](LICENSE).
