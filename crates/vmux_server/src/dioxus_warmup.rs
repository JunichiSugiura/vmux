//! Hidden CEF webviews that preload Dioxus bundles (WASM + renderer path) before a pane opens.
//!
//! Lives in `vmux_server` (not `vmux_layout`) to avoid a dependency cycle: `vmux_layout` → `vmux_ui`
//! → `vmux_server` → …

use bevy::prelude::*;
use bevy_cef::prelude::{PreloadScripts, WebviewSize, WebviewSource, ZoomLevel};

/// CEF page zoom; `0.0` matches typical desktop browsers at 100% (see `vmux_layout::CEF_PAGE_ZOOM_LEVEL`).
const CEF_PAGE_ZOOM_LEVEL: f64 = 0.0;

/// Emacs-style readline bindings for `<input>` / `<textarea>` (same source as `vmux_layout::TEXT_INPUT_EMACS_BINDINGS_PRELOAD`).
const TEXT_INPUT_EMACS_BINDINGS_PRELOAD: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../vmux_input/src/text_input_emacs_bindings.js"
));

/// Hosted Dioxus UI surface (aligns with [`vmux_layout::VmuxWebviewSurface`] for history + chrome).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EmbeddedDioxusUiSurface {
    HistoryPane,
    PaneChrome,
}

/// Marks a hidden loopback webview spawned to warm CEF for a hosted Dioxus surface.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DioxusUiWarmupWebview {
    pub surface: EmbeddedDioxusUiSurface,
}

/// Bundle matching real tiled / chrome panes so layout and Dioxus hit the same code paths.
pub fn dioxus_ui_warmup_bundle(
    surface: EmbeddedDioxusUiSurface,
    name: &'static str,
    url: String,
) -> impl Bundle {
    (
        DioxusUiWarmupWebview { surface },
        Name::new(name.to_string()),
        Visibility::Hidden,
        WebviewSize(Vec2::new(1024.0, 768.0)),
        ZoomLevel(CEF_PAGE_ZOOM_LEVEL),
        PreloadScripts::from([TEXT_INPUT_EMACS_BINDINGS_PRELOAD.to_string()]),
        WebviewSource::new(url),
    )
}
