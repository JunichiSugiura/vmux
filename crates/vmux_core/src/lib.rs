//! Shared scene markers and constants for `vmux` and `vmux_webview`.

use serde::Deserialize;

pub use vmux_layout::{
    Active, LayoutAxis, LayoutNode, LayoutPlugin, LayoutTree, LastVisitedUrl, Pane, PaneLastUrl,
    PixelRect, Root, SavedLayoutNode, SessionLayoutSnapshot, SessionSavePath, allowed_navigation_url,
    initial_webview_url, CAMERA_DISTANCE, VmuxWorldCamera, layout_node_to_saved, solve_layout,
};

/// Payload from `window.cef.emit({ url })` (single-arg form matches bevy_cef IPC).
#[derive(Debug, Clone, Deserialize)]
pub struct WebviewDocumentUrlEmit {
    pub url: String,
}
