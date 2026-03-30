//! Default keyboard shortcuts for vmux apps (e.g. quit) and tmux-style **Ctrl+B** chord registration.
//!
//! Add `vmux_settings::SettingsPlugin` **before** this plugin so `VmuxAppSettings` is initialized for chord systems.

mod component;
mod system;

pub use component::AppAction;
pub use vmux_layout::{
    tmux_prefix_commands, AppInputRoot, PREFIX_TIMEOUT_SECS, VmuxPrefixChordSet, VmuxPrefixState,
};

use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

#[derive(Default)]
pub struct VmuxInputPlugin;

impl Plugin for VmuxInputPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(Update, VmuxPrefixChordSet)
            .add_plugins(InputManagerPlugin::<AppAction>::default())
            .add_systems(Startup, system::spawn_app_input)
            .add_systems(
                Update,
                (
                    system::exit_on_quit_action,
                    tmux_prefix_commands.in_set(VmuxPrefixChordSet),
                ),
            );
    }
}
