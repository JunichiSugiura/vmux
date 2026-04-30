pub mod event;

#[cfg(not(target_arch = "wasm32"))]
pub mod bundle;

#[cfg(not(target_arch = "wasm32"))]
pub mod system;

#[cfg(not(target_arch = "wasm32"))]
pub use bundle::{FOOTER_WEBVIEW_URL, Footer, FooterBundle};

#[cfg(not(target_arch = "wasm32"))]
pub use system::FOOTER_HEIGHT_PX;

#[cfg(not(target_arch = "wasm32"))]
include!("plugin.rs");
