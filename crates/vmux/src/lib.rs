//! vmux — Bevy + embedded CEF webview library.

mod command_palette;
pub mod core;
#[cfg(target_os = "macos")]
mod macos_liquid_glass;
mod system;

pub use core::{VmuxWorldCamera, CAMERA_DISTANCE};
pub use vmux_input::{AppAction, AppInputRoot, VmuxInputPlugin};
pub use vmux_layout::LastVisitedUrl;
pub use vmux_layout::{LayoutPlugin, SessionLayoutSnapshot};
pub use vmux_session::SessionPlugin;
pub use vmux_session::{SessionSavePath, SessionSaveQueue};
pub use vmux_settings::cef_root_cache_path;
pub use vmux_settings::{SettingsPlugin, VmuxAppSettings};
pub use vmux_webview::VmuxWebviewPlugin;

use bevy::prelude::*;
use bevy::window::{CompositeAlphaMode, Window, WindowPlugin};
use vmux_core::VmuxCommandPaletteState;

/// Primary window: on macOS, transparent surface + post-multiplied alpha for system compositor
/// (see [Bevy window docs](https://docs.rs/bevy/latest/bevy/window/struct.Window.html#structfield.transparent)).
#[cfg(target_os = "macos")]
fn vmux_primary_window() -> Window {
    Window {
        transparent: true,
        composite_alpha_mode: CompositeAlphaMode::PostMultiplied,
        // Match liquid-glass / NSGlassEffectView expectations (winit macOS extensions).
        titlebar_transparent: true,
        fullsize_content_view: true,
        ..default()
    }
}

#[cfg(not(target_os = "macos"))]
fn vmux_primary_window() -> Window {
    Window::default()
}

#[derive(Default)]
pub struct VmuxScenePlugin;

impl Plugin for VmuxScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Startup,
            (
                system::normalize_window_padding_from_legacy_save,
                system::configure_primary_window,
                system::sync_clear_color_from_primary_window,
                system::spawn_camera,
                command_palette::setup.after(system::spawn_camera),
                system::spawn_directional_light,
            )
                .chain(),
        );
        app.add_systems(
            Update,
            (
                command_palette::toggle_hotkey,
                command_palette::handle_keyboard,
                command_palette::submit,
                command_palette::sync_visibility,
                command_palette::refresh_labels,
                command_palette::style_rows,
            )
                .chain(),
        );
        #[cfg(target_os = "macos")]
        app.add_systems(Update, macos_liquid_glass::apply_macos_liquid_glass);
    }
}

/// Full vmux stack: Bevy defaults, CEF, input, scene, and webview.
#[derive(Default)]
pub struct VmuxPlugin;

impl Plugin for VmuxPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VmuxCommandPaletteState>();
        app.add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(vmux_primary_window()),
                ..default()
            }),
            SettingsPlugin,
            VmuxInputPlugin,
            VmuxScenePlugin,
            SessionPlugin,
            VmuxWebviewPlugin::default(),
        ));
    }
}
