//! Embedded HTTP for the history UI (`dist/` / `web_dist` / embedded / `VMUX_HISTORY_UI_URL`).
//! Host wiring lives in [`vmux_ui::hosted::history`]; this crate only resolves the bundle path and registers the serve plugin.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use bevy::prelude::*;
use rust_embed::RustEmbed;
use vmux_layout::{VmuxHostedWebPlugin, VmuxWebviewSurface};
use vmux_server::{
    DioxusWarmupDescriptor, EmbeddedDioxusUiSurface, EmbeddedServeDirStartup, PendingEmbeddedServeDir,
    ServePlugin, push_pending_embedded_serve_dir, register_serve_plugin_dioxus_warmup,
};
use vmux_ui::extract_embedded_dist_to_temp;
use vmux_ui::hosted::history::{
    HistoryUiBaseUrl, HistoryUiChromeUnavailable, HistoryUiUrlReceiver, history_dioxus_warmup_should_spawn,
};

/// Embedded `dist/` when not on disk (release / missing checkout); see [`startup_history_server`].
#[derive(RustEmbed)]
#[folder = "dist"]
struct HistoryWebDist;

/// On-disk UI roots (same order as [`startup_history_server`]).
///
/// Native `cargo build -p vmux_history` runs **`build.rs`**, which writes **`dist/`**
/// (`wasm-bindgen` + Tailwind). Keep `web_dist/` as a compatibility fallback for older local trees.
fn history_dist_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(crate::DIST_DIR_NAME)
}

fn history_web_dist_fallback_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(crate::DIST_WEB_DIR_NAME)
}

/// True when `index.html` was post-processed by `scripts/inline-history-css.mjs` (single-document load for CEF).
fn history_index_has_inlined_css(index_html: &Path) -> bool {
    fs::read_to_string(index_html)
        .ok()
        .is_some_and(|s| s.contains("vmux-history-inline"))
}

fn history_ui_filesystem_root() -> Option<PathBuf> {
    let dist = history_dist_dir();
    let dist_index = dist.join("index.html");
    if dist_index.is_file() {
        if history_index_has_inlined_css(&dist_index) {
            return Some(dist);
        }
        bevy::log::warn!(
            "vmux history: {} is missing inlined CSS (expected `<style id=\"vmux-history-inline\">` from `npm run build:css`); not using this folder so the embedded bundle can load instead. Run `cargo build -p vmux_history` to refresh dist.",
            dist_index.display()
        );
    }
    let web = history_web_dist_fallback_dir();
    let web_index = web.join("index.html");
    if web_index.is_file() {
        if history_index_has_inlined_css(&web_index) {
            bevy::log::warn!(
                "vmux history: using legacy web_dist fallback at {}; rebuild vmux_history to refresh dist/.",
                web.display()
            );
            return Some(web);
        }
        bevy::log::warn!(
            "vmux history: {} is missing inlined CSS; ignoring this legacy web_dist folder.",
            web_index.display()
        );
    }
    None
}

/// Filesystem `dist/` / `web_dist/` or embedded `dist/` extract.
pub fn history_bundle_root() -> Option<PathBuf> {
    history_ui_filesystem_root()
        .or_else(|| extract_embedded_dist_to_temp::<HistoryWebDist>("vmux-history-ui"))
}

fn startup_history_server(mut commands: Commands, mut pending: ResMut<PendingEmbeddedServeDir>) {
    if let Ok(u) = std::env::var("VMUX_HISTORY_UI_URL") {
        let u = u.trim();
        if !u.is_empty() {
            commands.insert_resource(HistoryUiBaseUrl(Some(u.to_string())));
            return;
        }
    }

    let t_resolve = Instant::now();
    let root = history_bundle_root();
    let Some(root) = root else {
        bevy::log::warn!(
            "vmux history: no UI bundle (run `cargo build -p vmux_history` to populate dist via build.rs, or set VMUX_HISTORY_UI_URL; need dist/ or embedded dist)"
        );
        commands.insert_resource(HistoryUiChromeUnavailable(true));
        return;
    };
    bevy::log::info!(
        "vmux history: serving UI from {} (resolved in {:?})",
        root.display(),
        t_resolve.elapsed()
    );

    let rx = push_pending_embedded_serve_dir(&mut pending, root);
    commands.insert_resource(HistoryUiUrlReceiver(Some(rx)));
}

/// Embedded HTTP for the history UI (`dist/` if present, else legacy `web_dist/`, else embedded
/// `dist/`, or `VMUX_HISTORY_UI_URL`).
#[derive(Default)]
pub struct HistoryServerPlugin;

impl Plugin for HistoryServerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HistoryUiBaseUrl>()
            .init_resource::<HistoryUiUrlReceiver>()
            .init_resource::<HistoryUiChromeUnavailable>()
            .add_systems(
                Startup,
                startup_history_server.in_set(EmbeddedServeDirStartup::FillPending),
            );
        register_serve_plugin_dioxus_warmup::<Self>(app);
    }
}

impl ServePlugin for HistoryServerPlugin {
    fn dioxus_warmup_descriptor() -> Option<DioxusWarmupDescriptor> {
        Some(DioxusWarmupDescriptor {
            surface: EmbeddedDioxusUiSurface::HistoryPane,
            name: "vmux_history_ui_warmup",
            should_spawn: history_dioxus_warmup_should_spawn,
        })
    }
}

impl VmuxHostedWebPlugin for HistoryServerPlugin {
    const SURFACE: VmuxWebviewSurface = VmuxWebviewSurface::HistoryPane;
}

#[cfg(test)]
mod tests {
    use super::*;
    use vmux_server::ServerPlugin;
    use vmux_ui::hosted::history::HistoryUiPlugin;

    #[test]
    fn history_server_and_ui_plugins_register_in_app() {
        let mut app = App::new();
        app.add_plugins((ServerPlugin, HistoryServerPlugin, HistoryUiPlugin));
    }
}
