pub mod event;

#[cfg(not(target_arch = "wasm32"))]
pub mod bundle;

#[cfg(not(target_arch = "wasm32"))]
pub use bundle::SIDE_SHEET_WEBVIEW_URL;

#[cfg(not(target_arch = "wasm32"))]
include!("plugin.rs");

#[cfg(test)]
mod tests {
    #[test]
    fn chrome_body_uses_dark_glass_class_with_solid_white_text() {
        let css = include_str!("../assets/index.css");
        let html = include_str!("../assets/index.html");

        assert!(css.contains("--foreground: oklch(1 0 0);"));
        assert!(css.contains("--glass: oklch(0.36 0 0 / 0.82);"));
        assert!(css.contains("--glass-border: oklch(0.52 0 0 / 0.36);"));
        assert!(!css.contains("--glass-border: oklch(1 0 0"));
        assert!(!css.contains("--chrome-background-glass"));
        assert!(!css.contains("background-color: var(--chrome-background-glass);"));
        assert!(!css.contains("html.dark body.glass"));
        assert!(!css.contains("background-color: transparent;"));
        assert!(html.contains("<body class=\"glass "));
    }

    #[test]
    fn active_tab_title_uses_solid_foreground_without_opacity() {
        let app = include_str!("app.rs");

        assert!(app.contains("text-ui font-medium text-foreground"));
        assert!(!app.contains("text-foreground/"));
    }

    #[test]
    fn active_tab_row_uses_glass_surface() {
        let app = include_str!("app.rs");

        assert!(app.contains("glass group flex cursor-default"));
        assert!(!app.contains("bg-glass px-2 py-1.5 border"));
    }
}
