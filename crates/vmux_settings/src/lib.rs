//! Shared vmux defaults and [`VmuxAppSettings`] (persisted via [moonshine-save] with session).
//!
//! Bundled defaults are read from `settings.ron` next to this crate’s `Cargo.toml`.

use bevy::prelude::*;
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(Deserialize)]
struct BundledSettings {
    default_webview_url: String,
}

static BUNDLED_SETTINGS: OnceLock<VmuxAppSettings> = OnceLock::new();

fn bundled_settings() -> &'static VmuxAppSettings {
    BUNDLED_SETTINGS.get_or_init(|| {
        const EMBEDDED: &str = include_str!("../settings.ron");
        let bundled: BundledSettings = ron::de::from_str(EMBEDDED)
            .unwrap_or_else(|e| panic!("vmux_settings: invalid bundled settings.ron: {e}"));
        VmuxAppSettings {
            default_webview_url: bundled.default_webview_url,
        }
    })
}

/// Bundled default webview URL from `settings.ron` (same string as [`VmuxAppSettings::default`] until overridden at runtime).
pub fn default_webview_url() -> &'static str {
    bundled_settings().default_webview_url.as_str()
}

/// User-tunable app settings. Saved with [`SessionLayoutSnapshot`] in `last_session.ron` (moonshine).
#[derive(Resource, Clone, Debug, Reflect)]
#[reflect(Resource, Default)]
pub struct VmuxAppSettings {
    pub default_webview_url: String,
}

impl Default for VmuxAppSettings {
    fn default() -> Self {
        bundled_settings().clone()
    }
}

/// User-writable vmux cache directory (session, CEF sibling, etc.), inserted in [`PreStartup`](Schedule) by [`SettingsPlugin`].
#[derive(Resource, Clone, Debug, Default)]
pub struct VmuxCacheDir(pub Option<PathBuf>);

/// Runs before systems that read [`VmuxCacheDir`] (e.g. session save path).
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct VmuxCacheDirInitSet;

fn vmux_cache_base_dir() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(|home| {
        if cfg!(target_os = "macos") {
            PathBuf::from(home).join("Library/Caches/vmux")
        } else {
            PathBuf::from(home).join(".cache/vmux")
        }
    })
}

fn init_vmux_cache_dir(mut commands: Commands) {
    commands.insert_resource(VmuxCacheDir(vmux_cache_base_dir()));
}

/// User-writable CEF disk cache root (`<vmux cache>/cef`), with temp-dir fallback when `HOME` is unset.
///
/// Matches the layout implied by [`VmuxCacheDir`]; safe to call before [`PreStartup`](Schedule) inserts that resource (e.g. when configuring CEF at app startup).
pub fn cef_root_cache_path() -> Option<String> {
    vmux_cache_base_dir()
        .map(|base| base.join("cef").to_string_lossy().into_owned())
        .or_else(|| {
            std::env::temp_dir()
                .to_str()
                .map(|p| format!("{p}/vmux_cef"))
        })
}

/// Registers [`VmuxAppSettings`] for reflection (moonshine load/save) and [`VmuxCacheDir`] on startup.
#[derive(Default)]
pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<VmuxAppSettings>()
            .init_resource::<VmuxAppSettings>()
            .configure_sets(PreStartup, VmuxCacheDirInitSet)
            .add_systems(PreStartup, init_vmux_cache_dir.in_set(VmuxCacheDirInitSet));
    }
}
