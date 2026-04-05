use bevy::prelude::*;

pub struct CommandPlugin;

impl Plugin for CommandPlugin {
    fn build(&self, _app: &mut App) {}
}

#[derive(Event)]
pub struct NewSpaceCommand;

#[derive(Event)]
pub struct SplitVerticallyCommand;

#[derive(Event)]
pub struct SplitHorizontallyCommand;
