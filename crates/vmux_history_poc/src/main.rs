//! **wasm32:** [`dioxus::launch`] → [`app::App`]. **Native:** Bevy + CEF host; `build.rs` fills **`dist/`**
//! via **`dx build`**. Handshake: `PreloadScripts` issues [BRP](https://not-elm.github.io/bevy_cef/communication/brp)
//! (`window.cef.brp`); WASM reads the result from `window` (see `bridge` module).

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
use bevy_remote::{BrpResult, RemotePlugin};

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

/// Preload script calls `cef.brp`; WASM reads `window.__vmuxHistoryHandshake` (see `bridge` module).
#[cfg(not(target_arch = "wasm32"))]
fn handle_handshake(In(_params): In<Option<serde_json::Value>>, q: Query<&History>) -> BrpResult {
    let history: Vec<String> = q.iter().map(|h| h.url.clone()).collect();
    let url = history.join(", ");
    Ok(serde_json::json!({ "url": url, "history": history }))
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
            RemotePlugin::default().with_method(crate::bridge::HANDSHAKE_METHOD, handle_handshake),
            CefPlugin {
                root_cache_path: poc_cef_root_cache_path(),
                ..default()
            },
        ))
        .add_systems(
            Startup,
            (spawn_camera, spawn_directional_light, spawn_webview),
        )
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
fn history_handshake_preload_script() -> String {
    format!(
        r#"
window.cef.brp({{
  jsonrpc: "2.0",
  method: "{method}",
  params: null
}}).then(function (v) {{
  var apply = window.{apply};
  if (typeof apply === "function") apply(v);
  else window.{result} = v;
}}).catch(function (e) {{
  var applyErr = window.{apply_err};
  if (typeof applyErr === "function") applyErr(String(e));
  else window.{error} = String(e);
}});"#,
        method = crate::bridge::HANDSHAKE_METHOD,
        apply = crate::bridge::HANDSHAKE_APPLY_FN,
        apply_err = crate::bridge::HANDSHAKE_APPLY_ERROR_FN,
        result = crate::bridge::HANDSHAKE_RESULT_GLOBAL,
        error = crate::bridge::HANDSHAKE_ERROR_GLOBAL,
    )
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
        PreloadScripts::from([history_handshake_preload_script()]),
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
