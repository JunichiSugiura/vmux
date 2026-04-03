use {
    bevy::asset::io::embedded::EmbeddedAssetRegistry,
    bevy::prelude::*,
    bevy_cef::prelude::*,
    bevy_cef_core::prelude::Browsers,
    event::{HISTORY_EVENT, HistoryEvent},
    serde::Deserialize,
    std::path::{Path, PathBuf},
};

#[derive(Deserialize)]
pub struct HistoryUiReady {}

#[derive(Component)]
pub struct HistoryPocUiReady;

#[derive(Component)]
pub struct HistoryPocHistorySent;

#[derive(Component)]
pub struct HistoryPocEntry {
    pub url: String,
}

pub fn on_history_ui_ready(trigger: On<Receive<HistoryUiReady>>, mut commands: Commands) {
    let wv = trigger.event().webview;
    commands.entity(wv).insert(HistoryPocUiReady);
}

#[derive(Default)]
pub struct HistoryPlugin;

impl Plugin for HistoryPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_history_ui_ready);
        let manifest_dist = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("dist");
        let mut reg = app.world_mut().resource_mut::<EmbeddedAssetRegistry>();
        if let Err(e) = embed_dist_recursive(&mut reg, &manifest_dist, &manifest_dist) {
            bevy::log::error!(
                "vmux_history_poc: failed to embed dist/ (run `cargo build -p vmux_history_poc` so build.rs runs `dx`): {e}"
            );
        }
    }
}

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

pub fn push_history_via_host_emit(
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
