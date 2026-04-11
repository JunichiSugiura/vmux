use std::path::PathBuf;

use bevy::prelude::*;
use vmux_webview_app::{WebviewAppConfig, WebviewAppPlugin};

pub struct StatusBarPlugin;

impl Plugin for StatusBarPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(WebviewAppPlugin::new(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")),
            WebviewAppConfig::with_custom_host("status_bar"),
        ));
    }
}
