use bevy::prelude::*;
use bevy_cef::prelude::*;

pub const STATUS_BAR_WEBVIEW_URL: &str = "vmux://status_bar/";

#[derive(Component)]
pub struct StatusBar;

#[derive(Bundle)]
pub struct StatusBarBundle {
    pub marker: StatusBar,
    pub source: WebviewSource,
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<WebviewExtendStandardMaterial>,
    pub webview_size: WebviewSize,
}
