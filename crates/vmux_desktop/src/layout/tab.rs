use crate::command::{AppCommand, TabCommand, WriteAppCommands};
use bevy::prelude::*;

pub(crate) struct TabPlugin;

impl Plugin for TabPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, write_tab_hotkeys.in_set(WriteAppCommands));
    }
}

fn write_tab_hotkeys(keyboard: Res<ButtonInput<KeyCode>>, mut writer: MessageWriter<AppCommand>) {
    let ctrl = keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight);
    let meta = keyboard.pressed(KeyCode::SuperLeft) || keyboard.pressed(KeyCode::SuperRight);
    if !keyboard.just_pressed(KeyCode::Tab) || (!ctrl && !meta) {
        return;
    }
    let shift = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);
    if shift {
        writer.write(AppCommand::Tab(TabCommand::Previous));
    } else {
        writer.write(AppCommand::Tab(TabCommand::Next));
    }
}
