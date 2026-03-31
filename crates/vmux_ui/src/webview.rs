//! Dioxus / WASM: embedded webview UIs (status bar, history, …). Dioxus widgets live under [`crate::webview::components`].

#[cfg(target_arch = "wasm32")]
pub mod components;

#[cfg(target_arch = "wasm32")]
pub mod hooks;

#[cfg(target_arch = "wasm32")]
pub mod web_color;

#[cfg(target_arch = "wasm32")]
pub mod cef_bridge;
