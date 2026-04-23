pub mod event;

#[cfg(not(target_arch = "wasm32"))]
pub use vmux_core::{now_millis, CreatedAt, LastActivatedAt, Visit};

#[cfg(not(target_arch = "wasm32"))]
use bevy::prelude::*;

#[cfg(not(target_arch = "wasm32"))]
include!("plugin.rs");
