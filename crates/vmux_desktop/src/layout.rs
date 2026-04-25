mod focus_ring;
pub(crate) mod glass;
mod header;
pub(crate) mod tab;

pub(crate) mod drag;
pub(crate) mod pane;
pub(crate) mod side_sheet;
pub(crate) mod space;
pub(crate) mod swap;
pub(crate) mod window;

use bevy::prelude::*;
use focus_ring::FocusRingPlugin;
use glass::GlassMaterialPlugin;
use header::HeaderLayoutPlugin;
use pane::PanePlugin;
use side_sheet::SideSheetLayoutPlugin;
use space::SpacePlugin;
use tab::TabPlugin;
use vmux_webview_app::JsEmitUiReadyPlugin;
use window::WindowPlugin;
pub(crate) use window::fit_window_to_screen;

pub struct LayoutPlugin;

impl Plugin for LayoutPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            JsEmitUiReadyPlugin,
            WindowPlugin,
            SpacePlugin,
            PanePlugin,
            TabPlugin,
            FocusRingPlugin,
            GlassMaterialPlugin,
            SideSheetLayoutPlugin,
            HeaderLayoutPlugin,
        ));
    }
}
