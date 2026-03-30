//! Input action types.

use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

#[derive(Actionlike, Reflect, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppAction {
    Quit,
}
