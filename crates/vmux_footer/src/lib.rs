pub mod event;
pub mod style;

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

#[cfg(test)]
mod tests {
    #[test]
    fn chrome_body_uses_glass_class_with_solid_white_text() {
        let css = include_str!("../assets/index.css");
        let html = include_str!("../assets/index.html");

        assert!(css.contains("--foreground: oklch(1 0 0);"));
        assert!(css.contains("--glass: oklch(0.36 0 0 / 0.82);"));
        assert!(!css.contains("--chrome-background-glass"));
        assert!(!css.contains("background-color: var(--chrome-background-glass);"));
        assert!(css.contains("html.dark body.glass"));
        assert!(css.contains("background-color: transparent;"));
        assert!(css.contains("border: 0;"));
        assert!(html.contains("<body class=\"glass "));
    }

    #[test]
    fn active_space_text_uses_solid_foreground_without_opacity() {
        let style = include_str!("style.rs");

        assert!(!style.contains("text-sidebar-primary-foreground/"));
    }
}
