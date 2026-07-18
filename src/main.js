const invoke = window.__TAURI__.core.invoke;

// ---------- state ----------
const state = {
  items: new Map(), // key -> item
  selected: new Set(), // keys
  folder: "",
  query: "",
  page: 1,
  sources: [], // [{id,label}]
  activeSources: new Set(),
  busy: false,
};

const key = (it) => `${it.source}:${it.id}`;

// ---------- elements ----------
const $ = (id) => document.getElementById(id);
const grid = $("grid");
const statusEl = $("status");
const actionbar = $("actionbar");
const selcount = $("selcount");
const saveSelectedBtn = $("save-selected");
const selectAllBox = $("select-all");
const loadmoreBtn = $("loadmore");
const folderPathEl = $("folder-path");
const toastEl = $("toast");

// ---------- toast ----------
let toastTimer;
function toast(msg, kind = "") {
  clearTimeout(toastTimer);
  toastEl.textContent = msg;
  toastEl.className = `toast ${kind}`;
  toastTimer = setTimeout(() => toastEl.classList.add("hidden"), 4200);
}

// ---------- sources ----------
async function loadSources() {
  state.sources = await invoke("list_sources");
  const box = $("sources");
  box.innerHTML = "";
  for (const s of state.sources) {
    state.activeSources.add(s.id);
    const label = document.createElement("label");
    label.className = "chip on";
    label.innerHTML = `<span class="dot"></span>${s.label}`;
    const cb = document.createElement("input");
    cb.type = "checkbox";
    cb.checked = true;
    cb.addEventListener("change", () => {
      if (cb.checked) {
        state.activeSources.add(s.id);
        label.classList.add("on");
      } else {
        state.activeSources.delete(s.id);
        label.classList.remove("on");
      }
    });
    label.prepend(cb);
    box.appendChild(label);
  }
}

// ---------- rendering ----------
function updateActionBar() {
  const n = state.selected.size;
  selcount.textContent = `${n} selected`;
  saveSelectedBtn.disabled = n === 0 || state.busy;
  saveSelectedBtn.textContent = n > 0 ? `Save selected (${n})` : "Save selected";
  actionbar.classList.toggle("hidden", state.items.size === 0);
  const total = state.items.size;
  selectAllBox.checked = total > 0 && n === total;
}

function makeCard(it) {
  const k = key(it);
  const card = document.createElement("div");
  card.className = "card" + (state.selected.has(k) ? " sel" : "");
  card.dataset.key = k;

  const cb = document.createElement("input");
  cb.type = "checkbox";
  cb.className = "check";
  cb.checked = state.selected.has(k);
  cb.title = "Select";
  cb.addEventListener("change", () => toggleSelect(k, cb.checked));

  const saveBtn = document.createElement("button");
  saveBtn.className = "save-one";
  saveBtn.type = "button";
  saveBtn.textContent = "⤓";
  saveBtn.title = "Save this image";
  saveBtn.addEventListener("click", (e) => {
    e.stopPropagation();
    saveOne(it, saveBtn);
  });

  const img = document.createElement("img");
  img.loading = "lazy";
  img.src = it.thumbnail;
  img.alt = it.title || it.source;
  img.addEventListener("click", () => invoke("open_external", { url: it.link }));
  img.addEventListener("error", () => card.classList.add("is-broken"));

  const broken = document.createElement("div");
  broken.className = "broken";
  broken.textContent = "Preview blocked — still saveable ⤓";

  const meta = document.createElement("div");
  meta.className = "meta";
  const tag = document.createElement("span");
  tag.className = "src-tag";
  tag.textContent = it.source_label;
  const title = document.createElement("span");
  title.className = "title";
  title.textContent = it.title || it.author || "";
  meta.append(tag, title);

  card.append(cb, saveBtn, img, broken, meta);
  return card;
}

function renderAppend(items) {
  const frag = document.createDocumentFragment();
  for (const it of items) {
    const k = key(it);
    if (state.items.has(k)) continue;
    state.items.set(k, it);
    frag.appendChild(makeCard(it));
  }
  grid.appendChild(frag);
  updateActionBar();
}

function toggleSelect(k, on) {
  if (on) state.selected.add(k);
  else state.selected.delete(k);
  const card = grid.querySelector(`.card[data-key="${CSS.escape(k)}"]`);
  if (card) card.classList.toggle("sel", on);
  updateActionBar();
}

// ---------- search ----------
async function runSearch(reset = true) {
  const q = $("query").value.trim();
  if (!q) return;
  if (state.busy) return;
  if (state.activeSources.size === 0) {
    toast("Enable at least one source", "err");
    return;
  }

  if (reset) {
    state.query = q;
    state.page = 1;
    state.items.clear();
    state.selected.clear();
    grid.innerHTML = "";
  } else {
    state.page += 1;
  }

  state.busy = true;
  updateActionBar();
  statusEl.innerHTML = `<span class="spinner"></span>Searching “${state.query}” (page ${state.page})…`;

  try {
    const res = await invoke("search", {
      query: state.query,
      sources: Array.from(state.activeSources),
      page: state.page,
    });
    renderAppend(res.items);

    let msg = `${state.items.size} results for “${state.query}”.`;
    if (res.errors && res.errors.length) {
      msg += "  ⚠ " + res.errors.map((e) => `${e.source_label} failed`).join(", ") + ".";
    }
    if (res.items.length === 0 && !reset) msg = "No more results.";
    statusEl.textContent = msg;
  } catch (e) {
    statusEl.textContent = "";
    toast("Search failed: " + e, "err");
  } finally {
    state.busy = false;
    updateActionBar();
  }
}

// ---------- folder ----------
async function chooseFolder() {
  try {
    const f = await invoke("pick_folder");
    if (f) setFolder(f);
  } catch (e) {
    toast("Could not open folder picker: " + e, "err");
  }
}
function setFolder(f) {
  state.folder = f;
  folderPathEl.textContent = f;
  folderPathEl.title = f;
}
async function ensureFolder() {
  if (state.folder) return true;
  await chooseFolder();
  return !!state.folder;
}

// ---------- saving ----------
async function saveOne(it, btn) {
  if (!(await ensureFolder())) return;
  btn.disabled = true;
  try {
    const report = await invoke("save_images", {
      items: [{ url: it.full, source: it.source, title: it.title, id: it.id }],
      folder: state.folder,
    });
    if (report.saved > 0) {
      btn.classList.add("saved");
      btn.textContent = "✓";
      toast("Saved to " + report.folder, "ok");
    } else {
      const why = report.outcomes[0] ? report.outcomes[0].detail : "unknown error";
      toast("Save failed: " + why, "err");
    }
  } catch (e) {
    toast("Save failed: " + e, "err");
  } finally {
    btn.disabled = false;
  }
}

async function saveSelected() {
  if (state.selected.size === 0) return;
  if (!(await ensureFolder())) return;
  const items = Array.from(state.selected)
    .map((k) => state.items.get(k))
    .filter(Boolean)
    .map((it) => ({ url: it.full, source: it.source, title: it.title, id: it.id }));

  state.busy = true;
  updateActionBar();
  const n = items.length;
  statusEl.innerHTML = `<span class="spinner"></span>Saving ${n} image${n > 1 ? "s" : ""}…`;
  try {
    const report = await invoke("save_images", { items, folder: state.folder });
    // mark saved cards
    for (const o of report.outcomes) {
      if (!o.ok) continue;
    }
    const msg =
      `Saved ${report.saved}/${n} to ${report.folder}` +
      (report.failed ? ` (${report.failed} failed).` : ".");
    statusEl.textContent = msg;
    toast(msg, report.failed ? "err" : "ok");
  } catch (e) {
    toast("Save failed: " + e, "err");
    statusEl.textContent = "";
  } finally {
    state.busy = false;
    updateActionBar();
  }
}

// ---------- select all ----------
function selectAll(on) {
  for (const k of state.items.keys()) {
    if (on) state.selected.add(k);
    else state.selected.delete(k);
  }
  for (const card of grid.querySelectorAll(".card")) {
    card.classList.toggle("sel", on);
    const cb = card.querySelector(".check");
    if (cb) cb.checked = on;
  }
  updateActionBar();
}

// ---------- wiring ----------
$("search-form").addEventListener("submit", (e) => {
  e.preventDefault();
  runSearch(true);
});
$("folder-btn").addEventListener("click", chooseFolder);
loadmoreBtn.addEventListener("click", () => runSearch(false));
saveSelectedBtn.addEventListener("click", saveSelected);
selectAllBox.addEventListener("change", () => selectAll(selectAllBox.checked));

loadSources();
