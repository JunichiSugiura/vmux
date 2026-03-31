use bevy::prelude::*;

/// Z distance of the world camera from the webview plane at z = 0 (used for frustum sizing).
pub const CAMERA_DISTANCE: f32 = 3.0;

/// Marker for the vmux world-facing camera used to size the webview plane (spawned by scene bootstrap).
#[derive(Component)]
pub struct VmuxWorldCamera;
