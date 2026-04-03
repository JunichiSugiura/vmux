use std::path::PathBuf;

use bevy::asset::io::web::WebAssetPlugin;
use bevy::prelude::*;
use bevy::render::alpha::AlphaMode;
use bevy_cef::prelude::*;
use serde::Deserialize;
use vmux_history_poc::HistoryPlugin;
use vmux_history_poc::event::{HISTORY_EVENT, HistoryEvent};
use vmux_webview_app::WebviewAppEmbedSet;

fn main() {
    #[cfg(not(target_os = "macos"))]
    early_exit_if_subprocess();

    App::new()
        .add_plugins((
            DefaultPlugins.set(WebAssetPlugin {
                silence_startup_warning: true,
            }),
            HistoryPlugin,
            BrowserPlugin,
        ))
        .add_observer(on_history_ui_ready)
        .add_systems(
            Startup,
            (
                spawn_camera,
                spawn_history_webview.after(WebviewAppEmbedSet),
            ),
        )
        .add_systems(Update, push_history_via_host_emit)
        .run();
}

#[derive(Deserialize)]
struct HistoryUiReady {}

#[derive(Component)]
struct HistoryPocUiReady;

#[derive(Component)]
struct HistoryPocHistorySent;

#[derive(Component)]
struct HistoryPocEntry {
    url: String,
}

fn on_history_ui_ready(trigger: On<Receive<HistoryUiReady>>, mut commands: Commands) {
    let wv = trigger.event().webview;
    commands.entity(wv).insert(HistoryPocUiReady);
}

fn push_history_via_host_emit(
    mut commands: Commands,
    browsers: NonSend<Browsers>,
    ready: Query<
        Entity,
        (
            With<WebviewSource>,
            With<HistoryPocUiReady>,
            Without<HistoryPocHistorySent>,
        ),
    >,
    history_q: Query<&HistoryPocEntry>,
) {
    for wv in ready.iter() {
        if !browsers.has_browser(wv) || !browsers.host_emit_ready(&wv) {
            continue;
        }
        let history: Vec<String> = history_q.iter().map(|h| h.url.clone()).collect();
        let url = history.join(", ");
        let payload = HistoryEvent { url, history };
        let ron_body = ron::ser::to_string(&payload).unwrap_or_default();
        commands.trigger(HostEmitEvent::new(wv, HISTORY_EVENT, &ron_body));
        commands.entity(wv).insert(HistoryPocHistorySent);
    }
}

#[derive(Component)]
struct Browser;

#[derive(Bundle)]
struct BrowserBundle {
    browser: Browser,
    webview: WebviewBundle,
}

#[derive(Bundle)]
struct WebviewBundle {
    source: WebviewSource,
    mesh: Mesh3d,
    material: MeshMaterial3d<WebviewExtendStandardMaterial>,
}

fn spawn_history_webview(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<WebviewExtendStandardMaterial>>,
) {
    commands.spawn((BrowserBundle {
        browser: Browser,
        webview: WebviewBundle {
            source: WebviewSource::new("vmux://history"),
            mesh: Mesh3d(meshes.add(Plane3d::new(Vec3::Z, Vec2::ONE))),
            material: MeshMaterial3d(materials.add(WebviewExtendStandardMaterial {
                base: StandardMaterial {
                    unlit: true,
                    alpha_mode: AlphaMode::Blend,
                    ..default()
                },
                extension: WebviewMaterial::default(),
            })),
        },
    },));
}

struct BrowserPlugin;

impl Plugin for BrowserPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            JsEmitEventPlugin::<HistoryUiReady>::default(),
            CefPlugin {
                root_cache_path: poc_cef_root_cache_path(),
                ..default()
            },
        ));
    }
}

fn poc_cef_root_cache_path() -> Option<String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| {
            let base = if cfg!(target_os = "macos") {
                home.join("Library/Caches/vmux_history_poc")
            } else {
                home.join(".cache/vmux_history_poc")
            };
            base.join("cef").to_string_lossy().into_owned()
        })
        .or_else(|| {
            std::env::temp_dir()
                .to_str()
                .map(|p| format!("{p}/vmux_history_poc_cef"))
        })
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(Vec3::new(0., 0., 3.)).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
