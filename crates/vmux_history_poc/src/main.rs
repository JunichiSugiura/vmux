//! **wasm32:** [`dioxus::launch`] → [`app::App`]. **Native:** Bevy + CEF; `build.rs` fills **`dist/`** via **`dx build`**.
//! UI readiness: JS registers [`cef.listen`](https://not-elm.github.io/bevy_cef/communication/) then `cef.emit` (`{}`); Bevy marks the webview and pushes history with **Host Emit**.

mod bridge;

#[cfg(target_arch = "wasm32")]
mod app;

#[cfg(target_arch = "wasm32")]
fn main() {
    dioxus::launch(app::App);
}

#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};

#[cfg(not(target_arch = "wasm32"))]
use bevy::asset::io::embedded::EmbeddedAssetRegistry;
#[cfg(not(target_arch = "wasm32"))]
use bevy::asset::io::web::WebAssetPlugin;
#[cfg(not(target_arch = "wasm32"))]
use bevy::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use bevy_cef::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use bevy_cef_core::prelude::Browsers;
#[cfg(not(target_arch = "wasm32"))]
use serde::Deserialize;
#[cfg(not(target_arch = "wasm32"))]
use serde_json::json;

#[cfg(not(target_arch = "wasm32"))]
struct HistoryPocPlugin;

#[cfg(not(target_arch = "wasm32"))]
impl Plugin for HistoryPocPlugin {
    fn build(&self, app: &mut App) {
        let manifest_dist = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("dist");
        let mut reg = app.world_mut().resource_mut::<EmbeddedAssetRegistry>();
        if let Err(e) = embed_dist_recursive(&mut reg, &manifest_dist, &manifest_dist) {
            bevy::log::error!(
                "vmux_history_poc: failed to embed dist/ (run `cargo build -p vmux_history_poc` so build.rs runs `dx`): {e}"
            );
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn embed_dist_recursive(
    reg: &mut EmbeddedAssetRegistry,
    manifest_dist: &Path,
    cur: &Path,
) -> std::io::Result<()> {
    let read_dir = match std::fs::read_dir(cur) {
        Ok(rd) => rd,
        Err(e) if cur == manifest_dist => return Err(e),
        Err(_) => return Ok(()),
    };
    for e in read_dir.flatten() {
        let p = e.path();
        if p.is_dir() {
            embed_dist_recursive(reg, manifest_dist, &p)?;
        } else {
            let Ok(rel) = p.strip_prefix(manifest_dist) else {
                continue;
            };
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            let embedded_path: &Path = if rel_str == "index.html" {
                Path::new(VMUX_HISTORY_DEFAULT_DOCUMENT)
            } else {
                Path::new(&rel_str)
            };
            let bytes = std::fs::read(&p)?;
            reg.insert_asset(p, embedded_path, bytes);
        }
    }
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
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

/// Payload from `cef.emit` after the UI registered `cef.listen` and emitted this object (`{}`).
#[cfg(not(target_arch = "wasm32"))]
#[derive(Deserialize)]
struct HistoryUiReady {}

/// Marker on the [`WebviewSource`] entity once the Dioxus side has emitted ready.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Component)]
struct HistoryPocUiReady;

/// Host emit has been sent at least once for this webview (POC: single snapshot).
#[cfg(not(target_arch = "wasm32"))]
#[derive(Component)]
struct HistoryPocHistorySent;

#[cfg(not(target_arch = "wasm32"))]
fn on_history_ui_ready(trigger: On<Receive<HistoryUiReady>>, mut commands: Commands) {
    let wv = trigger.event().webview;
    commands.entity(wv).insert(HistoryPocUiReady);
}

#[cfg(not(target_arch = "wasm32"))]
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
    history_q: Query<&History>,
) {
    for wv in ready.iter() {
        if !browsers.has_browser(wv) || !browsers.host_emit_ready(&wv) {
            continue;
        }
        let history: Vec<String> = history_q.iter().map(|h| h.url.clone()).collect();
        let url = history.join(", ");
        let payload = json!({ "url": url, "history": history });
        commands.trigger(HostEmitEvent::new(
            wv,
            crate::bridge::HOST_HISTORY_CHANNEL,
            &payload,
        ));
        commands.entity(wv).insert(HistoryPocHistorySent);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    #[cfg(not(target_os = "macos"))]
    early_exit_if_subprocess();

    App::new()
        .add_plugins((
            DefaultPlugins.set(WebAssetPlugin {
                silence_startup_warning: true,
            }),
            HistoryPocPlugin,
            JsEmitEventPlugin::<HistoryUiReady>::default(),
            CefPlugin {
                root_cache_path: poc_cef_root_cache_path(),
                ..default()
            },
        ))
        .add_systems(
            Startup,
            (spawn_camera, spawn_directional_light, spawn_webview),
        )
        .add_systems(Update, push_history_via_host_emit)
        .add_observer(on_history_ui_ready)
        .run();
}

#[cfg(not(target_arch = "wasm32"))]
fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(Vec3::new(0., 0., 3.)).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

#[cfg(not(target_arch = "wasm32"))]
fn spawn_directional_light(mut commands: Commands) {
    commands.spawn((
        DirectionalLight::default(),
        Transform::from_translation(Vec3::new(1., 1., 1.)).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

#[cfg(not(target_arch = "wasm32"))]
fn spawn_webview(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<WebviewExtendStandardMaterial>>,
) {
    commands.spawn((
        WebviewSource::vmux_service_root("history"),
        Mesh3d(meshes.add(Plane3d::new(Vec3::Z, Vec2::ONE))),
        MeshMaterial3d(materials.add(WebviewExtendStandardMaterial::default())),
    ));
    commands.spawn(History {
        url: "history1".to_string(),
    });
    commands.spawn(History {
        url: "history2".to_string(),
    });
    commands.spawn(History {
        url: "history3".to_string(),
    });
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Component)]
struct History {
    url: String,
}
