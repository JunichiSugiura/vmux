pub mod event;

#[cfg(not(target_arch = "wasm32"))]
pub mod bundle;

#[cfg(not(target_arch = "wasm32"))]
pub mod system;

#[cfg(not(target_arch = "wasm32"))]
pub use bundle::{HEADER_WEBVIEW_URL, Header, HeaderBundle};

#[cfg(not(target_arch = "wasm32"))]
pub use system::{HEADER_HEIGHT_PX, NavigationState, PageMetadata};

#[cfg(not(target_arch = "wasm32"))]
include!("plugin.rs");
