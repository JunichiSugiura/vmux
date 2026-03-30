//! Moonshine session file path resource.

use std::path::PathBuf;

use bevy::prelude::*;

/// Moonshine session file path (shared by `vmux` and `vmux_webview`).
#[derive(Resource, Clone, Debug)]
pub struct SessionSavePath(pub PathBuf);
