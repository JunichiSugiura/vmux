//! Web binary entry: [`dioxus::launch`] → [`app::App`] (wasm32 only).

#[cfg(target_arch = "wasm32")]
mod app;
#[cfg(target_arch = "wasm32")]
mod gallery;

#[cfg(target_arch = "wasm32")]
fn main() {
    dioxus::launch(app::App);
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    eprintln!("vmux_ui: wasm binary is for wasm32 (see build.rs).");
}
