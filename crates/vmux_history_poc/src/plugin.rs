use bevy::prelude::*;
use std::path::PathBuf;
use vmux_webview_app::{WebviewAppConfig, WebviewAppPlugin};

pub struct HistoryPlugin;

impl Plugin for HistoryPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(WebviewAppPlugin::new(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")),
            WebviewAppConfig::with_custom_host("history"),
        ));
    }
}
