//! Global command-palette state (⌘T / Ctrl+T) shared with input/CEF suppression.

use bevy::prelude::*;

/// Open/closed state, query string, and list selection for the centered command palette.
#[derive(Resource, Default, Clone)]
pub struct VmuxCommandPaletteState {
    pub open: bool,
    pub query: String,
    /// Index into fixed result rows (see `vmux` palette UI).
    pub selection: usize,
}
