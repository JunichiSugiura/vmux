use bevy::prelude::*;
use bevy_cef::prelude::*;

pub const FOOTER_WEBVIEW_URL: &str = "vmux://footer/";

#[derive(Component)]
pub struct Footer;

#[derive(Bundle)]
pub struct FooterBundle {
    pub marker: Footer,
    pub source: WebviewSource,
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<WebviewExtendStandardMaterial>,
    pub webview_size: WebviewSize,
}
