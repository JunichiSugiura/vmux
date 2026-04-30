//! Generic anti-flash reveal for newly-spawned webviews.
//!
//! When a `WebviewSource` is added to an entity, hide it via
//! `Visibility::Hidden` and start a frame counter. After a few frames
//! Bevy's UI layout has run and bevy_cef has resized the underlying CEF
//! webview, so revealing the entity avoids a 0-size flash on first paint.

use bevy::{prelude::*, ui::UiSystems};
use bevy_cef::prelude::WebviewSource;

use crate::layout::window::VmuxWindow;

pub(crate) struct WebviewRevealPlugin;

impl Plugin for WebviewRevealPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_webview_added)
            .add_systems(PostUpdate, reveal_webviews.after(UiSystems::Layout));
    }
}

/// Frame counter for a hidden webview waiting to be revealed.
#[derive(Component)]
pub(crate) struct PendingWebviewReveal(u8);

/// Number of frames to wait before revealing a freshly spawned webview.
/// 2 frames lets Bevy UI layout + bevy_cef resize the CEF surface so the
/// first visible paint is at the correct size.
const REVEAL_FRAMES: u8 = 2;

fn on_webview_added(
    trigger: On<Add, WebviewSource>,
    root: Query<(), With<VmuxWindow>>,
    mut commands: Commands,
) {
    let entity = trigger.event_target();
    // Don't hide the root window's own webview surface.
    if root.contains(entity) {
        return;
    }
    commands
        .entity(entity)
        .insert((Visibility::Hidden, PendingWebviewReveal(0)));
}

fn reveal_webviews(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Visibility, &mut PendingWebviewReveal)>,
) {
    for (entity, mut vis, mut pending) in &mut query {
        if pending.0 >= REVEAL_FRAMES {
            *vis = Visibility::Inherited;
            commands.entity(entity).remove::<PendingWebviewReveal>();
        } else {
            pending.0 += 1;
        }
    }
}
