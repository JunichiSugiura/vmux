//! Web bundle root: [`UiLibraryGallery`] — full storybook for [`vmux_ui::webview::components`].

use dioxus::prelude::*;

pub use crate::gallery::UiLibraryGallery;

#[component]
pub fn App() -> Element {
    rsx! {
        UiLibraryGallery {}
    }
}
