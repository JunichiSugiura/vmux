//! Minimal Bevy + CEF + Dioxus POC: **JS Emit** (UI ready) and **Host Emit** (history snapshot).
//!
//! Native `cargo build -p vmux_history_poc` runs **`build.rs`** (`dx build` → **`dist/`**). The Bevy
//! host embeds that tree so `vmux://history/` resolves HTML, WASM, and hashed assets.

/// Relative directory name for the Dioxus web bundle (`build.rs` writes here).
pub const DIST_DIR_NAME: &str = "dist";
