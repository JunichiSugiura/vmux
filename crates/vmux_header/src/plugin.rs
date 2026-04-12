use std::path::PathBuf;

use bevy::prelude::*;
use vmux_webview_app::{WebviewAppConfig, WebviewAppPlugin};

use crate::system::apply_chrome_state_from_cef;

pub struct HeaderPlugin;

impl Plugin for HeaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(WebviewAppPlugin::new(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")),
            WebviewAppConfig::with_custom_host("header"),
        ))
        .add_systems(Update, apply_chrome_state_from_cef);
    }
}
