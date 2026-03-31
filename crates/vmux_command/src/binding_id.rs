//! [`vmux_settings::BindingCommandId`] ↔ [`KeyAction`] / [`AppCommand`] (keyboard binding layer).

use vmux_layout::PaneSwapDir;
use vmux_settings::BindingCommandId;

use crate::{AppCommand, KeyAction};

impl KeyAction {
    pub fn to_binding_id(self) -> BindingCommandId {
        match self {
            KeyAction::Quit => BindingCommandId::Quit,
            KeyAction::ToggleCommandPalette => BindingCommandId::ToggleCommandPalette,
            KeyAction::FocusCommandPaletteUrl => BindingCommandId::FocusCommandPaletteUrl,
            KeyAction::OpenHistory => BindingCommandId::OpenHistory,
            KeyAction::OpenHistoryInNewTab => BindingCommandId::OpenHistoryInNewTab,
            KeyAction::SplitHorizontal => BindingCommandId::SplitHorizontal,
            KeyAction::SplitVertical => BindingCommandId::SplitVertical,
            KeyAction::CycleNextPane => BindingCommandId::CycleNextPane,
            KeyAction::SelectPane(PaneSwapDir::Left) => BindingCommandId::SelectPaneLeft,
            KeyAction::SelectPane(PaneSwapDir::Right) => BindingCommandId::SelectPaneRight,
            KeyAction::SelectPane(PaneSwapDir::Up) => BindingCommandId::SelectPaneUp,
            KeyAction::SelectPane(PaneSwapDir::Down) => BindingCommandId::SelectPaneDown,
            KeyAction::SwapPane(PaneSwapDir::Left) => BindingCommandId::SwapPaneLeft,
            KeyAction::SwapPane(PaneSwapDir::Right) => BindingCommandId::SwapPaneRight,
            KeyAction::SwapPane(PaneSwapDir::Up) => BindingCommandId::SwapPaneUp,
            KeyAction::SwapPane(PaneSwapDir::Down) => BindingCommandId::SwapPaneDown,
            KeyAction::ToggleZoom => BindingCommandId::ToggleZoom,
            KeyAction::MirrorLayout => BindingCommandId::MirrorLayout,
            KeyAction::RotateBackward => BindingCommandId::RotateBackward,
            KeyAction::RotateForward => BindingCommandId::RotateForward,
            KeyAction::ClosePane => BindingCommandId::ClosePane,
        }
    }
}

pub fn key_action_from_binding_id(id: BindingCommandId) -> Option<KeyAction> {
    Some(match id {
        BindingCommandId::Quit => KeyAction::Quit,
        BindingCommandId::ToggleCommandPalette => KeyAction::ToggleCommandPalette,
        BindingCommandId::FocusCommandPaletteUrl => KeyAction::FocusCommandPaletteUrl,
        BindingCommandId::OpenHistory => KeyAction::OpenHistory,
        BindingCommandId::OpenHistoryInNewTab => KeyAction::OpenHistoryInNewTab,
        BindingCommandId::SplitHorizontal => KeyAction::SplitHorizontal,
        BindingCommandId::SplitVertical => KeyAction::SplitVertical,
        BindingCommandId::CycleNextPane => KeyAction::CycleNextPane,
        BindingCommandId::SelectPaneLeft => KeyAction::SelectPane(PaneSwapDir::Left),
        BindingCommandId::SelectPaneRight => KeyAction::SelectPane(PaneSwapDir::Right),
        BindingCommandId::SelectPaneUp => KeyAction::SelectPane(PaneSwapDir::Up),
        BindingCommandId::SelectPaneDown => KeyAction::SelectPane(PaneSwapDir::Down),
        BindingCommandId::SwapPaneLeft => KeyAction::SwapPane(PaneSwapDir::Left),
        BindingCommandId::SwapPaneRight => KeyAction::SwapPane(PaneSwapDir::Right),
        BindingCommandId::SwapPaneUp => KeyAction::SwapPane(PaneSwapDir::Up),
        BindingCommandId::SwapPaneDown => KeyAction::SwapPane(PaneSwapDir::Down),
        BindingCommandId::ToggleZoom => KeyAction::ToggleZoom,
        BindingCommandId::MirrorLayout => KeyAction::MirrorLayout,
        BindingCommandId::RotateBackward => KeyAction::RotateBackward,
        BindingCommandId::RotateForward => KeyAction::RotateForward,
        BindingCommandId::ClosePane => KeyAction::ClosePane,
    })
}

/// Maps a binding from settings to a user-facing action (palette / leafwing).
pub fn app_command_from_binding_id(id: BindingCommandId) -> AppCommand {
    match id {
        BindingCommandId::Quit => AppCommand::Quit,
        BindingCommandId::ToggleCommandPalette => AppCommand::ToggleCommandPalette,
        BindingCommandId::FocusCommandPaletteUrl => AppCommand::FocusCommandPaletteUrl,
        BindingCommandId::OpenHistory => AppCommand::OpenHistory,
        BindingCommandId::OpenHistoryInNewTab => AppCommand::OpenHistoryInNewTab,
        BindingCommandId::SplitHorizontal => AppCommand::SplitHorizontal,
        BindingCommandId::SplitVertical => AppCommand::SplitVertical,
        BindingCommandId::CycleNextPane => AppCommand::CycleNextPane,
        BindingCommandId::SelectPaneLeft => AppCommand::SelectPane(PaneSwapDir::Left),
        BindingCommandId::SelectPaneRight => AppCommand::SelectPane(PaneSwapDir::Right),
        BindingCommandId::SelectPaneUp => AppCommand::SelectPane(PaneSwapDir::Up),
        BindingCommandId::SelectPaneDown => AppCommand::SelectPane(PaneSwapDir::Down),
        BindingCommandId::SwapPaneLeft => AppCommand::SwapPane(PaneSwapDir::Left),
        BindingCommandId::SwapPaneRight => AppCommand::SwapPane(PaneSwapDir::Right),
        BindingCommandId::SwapPaneUp => AppCommand::SwapPane(PaneSwapDir::Up),
        BindingCommandId::SwapPaneDown => AppCommand::SwapPane(PaneSwapDir::Down),
        BindingCommandId::ToggleZoom => AppCommand::ToggleZoom,
        BindingCommandId::MirrorLayout => AppCommand::MirrorLayout,
        BindingCommandId::RotateBackward => AppCommand::RotateBackward,
        BindingCommandId::RotateForward => AppCommand::RotateForward,
        BindingCommandId::ClosePane => AppCommand::ClosePane,
    }
}
