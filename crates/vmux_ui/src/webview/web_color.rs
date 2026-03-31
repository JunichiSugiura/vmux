//! Web/Tailwind string tokens for Dioxus panes (WASM).
//!
//! Names align conceptually with [`vmux_core::color_tokens`] / `vmux_ui::native::utils::color` on host builds (e.g. primary accent uses sky tones).

/// Tailwind classes for a thin progress track (history loading bar).
pub const LOADING_TRACK: &str = "h-1 w-36 overflow-hidden rounded-full bg-white/[0.08]";

/// Tailwind classes for the shimmer fill inside the loading track (accent = primary).
pub const LOADING_PULSE: &str = "vmux-shimmer-bar h-full rounded-full bg-sky-400/35";

/// Muted animated status line under the loading bar.
pub const SHIMMER_TEXT: &str = "vmux-shimmer-text text-[11px] text-white/28";

/// Primary accent text (URLs, highlights) — pairs with Bevy `PRIMARY` visually.
pub const TEXT_ACCENT_SKY: &str = "text-sky-300/95";
