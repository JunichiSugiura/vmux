//! Moonshine save of session snapshot + app settings into one file.

use std::path::PathBuf;

use bevy::prelude::*;
use moonshine_save::prelude::*;
use vmux_settings::VmuxAppSettings;

use crate::SessionLayoutSnapshot;

/// Writes [`SessionLayoutSnapshot`] and [`VmuxAppSettings`] to `path` (same file moonshine loads in `vmux`).
pub fn save_session_snapshot_to_file(commands: &mut Commands, path: PathBuf) {
    commands.trigger_save(
        SaveWorld::default_into_file(path)
            .include_resource::<SessionLayoutSnapshot>()
            .include_resource::<VmuxAppSettings>(),
    );
}
