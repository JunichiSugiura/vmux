use bevy::prelude::*;
use bevy_cef::prelude::*;

pub const HEADER_WEBVIEW_URL: &str = "vmux://header/";

#[derive(Component)]
pub struct Header;

#[derive(Bundle)]
pub struct HeaderBundle {
    pub marker: Header,
    pub source: WebviewSource,
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<WebviewExtendStandardMaterial>,
    pub webview_size: WebviewSize,
}
