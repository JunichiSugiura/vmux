use bevy::{picking::Pickable, prelude::*, render::alpha::AlphaMode};
use bevy_cef::prelude::*;
use vmux_daemon::protocol::ClientMessage;
use vmux_sessions::event::*;
use vmux_webview_app::UiReady;

use crate::{
    browser::Browser,
    layout::window::WEBVIEW_MESH_DEPTH_BIAS,
    terminal::{DaemonClient, DaemonSessionHandle, Terminal},
};

/// Marker for the sessions monitor webview entity.
#[derive(Component)]
pub(crate) struct SessionsMonitor;

impl SessionsMonitor {
    /// Create a sessions monitor webview bundle.
    pub(crate) fn new(
        meshes: &mut ResMut<Assets<Mesh>>,
        webview_mt: &mut ResMut<Assets<WebviewExtendStandardMaterial>>,
    ) -> impl Bundle {
        (
            (
                Self,
                Browser,
                WebviewSource::new(SESSIONS_WEBVIEW_URL),
                ResolvedWebviewUri(SESSIONS_WEBVIEW_URL.to_string()),
                vmux_header::PageMetadata {
                    title: "Sessions".to_string(),
                    url: SESSIONS_WEBVIEW_URL.to_string(),
                    favicon_url: String::new(),
                },
                Mesh3d(meshes.add(bevy::math::primitives::Plane3d::new(
                    Vec3::Z,
                    Vec2::splat(0.5),
                ))),
            ),
            (
                MeshMaterial3d(webview_mt.add(WebviewExtendStandardMaterial {
                    base: StandardMaterial {
                        unlit: true,
                        alpha_mode: AlphaMode::Blend,
                        depth_bias: WEBVIEW_MESH_DEPTH_BIAS,
                        ..default()
                    },
                    ..default()
                })),
                WebviewSize(Vec2::new(1280.0, 720.0)),
                Transform::default(),
                GlobalTransform::default(),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    right: Val::Px(0.0),
                    top: Val::Px(0.0),
                    bottom: Val::Px(0.0),
                    ..default()
                },
                Visibility::Inherited,
                Pickable::default(),
            ),
        )
    }
}

/// Cached session list from the daemon, updated via ListSessions responses.
/// Written by terminal.rs's poll_daemon_messages, read by this module.
#[derive(Resource, Default)]
pub(crate) struct DaemonSessionList {
    pub sessions: Vec<vmux_daemon::protocol::SessionInfo>,
}

/// Timer for periodic session list polling.
#[derive(Resource)]
struct SessionsPollTimer(Timer);

pub(crate) struct SessionsMonitorPlugin;

impl Plugin for SessionsMonitorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DaemonSessionList>()
            .insert_resource(SessionsPollTimer(Timer::from_seconds(
                1.0,
                TimerMode::Repeating,
            )))
            .add_systems(Update, (request_session_list, broadcast_to_monitors).chain());
    }
}

/// Periodically send ListSessions to the daemon.
fn request_session_list(
    time: Res<Time>,
    mut timer: ResMut<SessionsPollTimer>,
    daemon: Option<Res<DaemonClient>>,
    monitors: Query<(), With<SessionsMonitor>>,
) {
    if monitors.is_empty() {
        return;
    }
    timer.0.tick(time.delta());
    if timer.0.just_finished() {
        if let Some(daemon) = daemon {
            daemon.0.send(ClientMessage::ListSessions);
        }
    }
}

/// Broadcast the cached session list to all sessions monitor webviews.
fn broadcast_to_monitors(
    session_list: Res<DaemonSessionList>,
    daemon: Option<Res<DaemonClient>>,
    monitors: Query<Entity, (With<SessionsMonitor>, With<UiReady>)>,
    browsers: NonSend<Browsers>,
    terminal_handles: Query<&DaemonSessionHandle, With<Terminal>>,
    mut commands: Commands,
) {
    if monitors.is_empty() || !session_list.is_changed() {
        return;
    }

    let connected = daemon.is_some();

    // Build attached set from local terminal handles
    let attached_ids: std::collections::HashSet<String> = terminal_handles
        .iter()
        .map(|h| h.session_id.to_string())
        .collect();

    let sessions: Vec<SessionEntry> = session_list
        .sessions
        .iter()
        .map(|info| SessionEntry {
            id: info.id.to_string(),
            shell: info.shell.clone(),
            cwd: info.cwd.clone(),
            cols: info.cols,
            rows: info.rows,
            pid: info.pid,
            uptime_secs: info.created_at_secs,
            attached: attached_ids.contains(&info.id.to_string()),
            preview_lines: Vec::new(), // TODO: add preview from snapshot
        })
        .collect();

    let event = SessionsListEvent {
        connected,
        sessions,
    };

    for entity in &monitors {
        if browsers.has_browser(entity) && browsers.host_emit_ready(&entity) {
            commands.trigger(HostEmitEvent::new(entity, SESSIONS_LIST_EVENT, &event));
        }
    }
}
