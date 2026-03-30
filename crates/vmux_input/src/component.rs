//! Input actions and prefix-routing markers (re-exported from [`vmux_core`] for layout sharing).

use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

pub use vmux_core::input_root::{
    AppInputRoot, VmuxPrefixChordSet, VmuxPrefixState, PREFIX_TIMEOUT_SECS,
};

#[derive(Actionlike, Reflect, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppAction {
    Quit,
    /// Centered command palette (⌘T on macOS, Ctrl+T elsewhere).
    ToggleCommandPalette,
}
