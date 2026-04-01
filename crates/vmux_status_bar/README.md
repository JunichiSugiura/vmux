# vmux status bar (Dioxus web)

Rust + **Dioxus** web app served from loopback by `vmux` (see `vmux_status_bar::StatusBarServerPlugin`) and loaded in the active pane’s CEF strip.

- **`src/main.rs`** — `dioxus::launch` entry.
- **`src/app.rs`** — root **App** component (status strip markup).
- **`src/bridge.rs`** — `document::eval` script for clock + `window.cef.listen("vmux_status", …)`.
- **`src/payload.rs`** — host payload types and `apply_payload`.
- **`assets/index.html`** — static shell for `wasm-bindgen` output (tracked).
- **`assets/input.css`** — Tailwind entry (`@tailwind` + `@layer base`).
- **`tailwind.config.js`** — theme (`tmux` colors, `text-status`, font stack).
- **`Dioxus.toml`** — Dioxus CLI metadata for `dx build`.
- **`dist/`** — **not** hand-edited: produced by `build.rs` via **`dx build --platform web`** (Tailwind + `wasm-bindgen` are bundled in **dioxus-cli**), then the CEF shell `assets/index.html` is copied over `dist/index.html`.

## Build (`dist/`)

Install [**dioxus-cli**](https://crates.io/crates/dioxus-cli) (**`dx`**) on your `PATH` (pin to the workspace Dioxus version, e.g. **0.7.4**). **Node.js is not required** for this crate.

**Styles:** Tailwind scans `tailwind.config.js` during `dx build`. The app loads CSS with `document::Stylesheet { href: asset!("/assets/input.css") }` so utilities stay in sync with the bundle.

From the repo root (`build.rs` runs `dx` when `dist/` is stale):

```bash
env -u CEF_PATH cargo build -p vmux_status_bar
```

`vmux` embeds `crates/vmux_status_bar/dist/` via Axum `ServeDir`; **`index.html`** must exist there.

Set **`VMUX_STATUS_UI_URL`** (e.g. a static dev server URL) to skip the embedded bundle for that session.
