//! Shared scene markers and IPC payloads for vmux crates.

mod active;
pub mod command_palette;
pub mod input_root;
pub mod pane_corner_clip;
mod session;

pub use active::Active;
pub use command_palette::VmuxCommandPaletteState;
pub use input_root::{AppInputRoot, VmuxPrefixChordSet, VmuxPrefixState, PREFIX_TIMEOUT_SECS};
pub use session::{SessionSavePath, SessionSaveQueue};

use serde::Deserialize;

/// Payload from `window.cef.emit({ url })` (single-arg form matches bevy_cef IPC).
#[derive(Debug, Clone, Deserialize)]
pub struct WebviewDocumentUrlEmit {
    pub url: String,
}
