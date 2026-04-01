//! Full UI library gallery (wasm): every widget under [`vmux_ui::webview::components`].

mod demos;
mod layout;

use dioxus::prelude::*;
use vmux_ui::webview::components::{UiStack, UiText, UiTextSize, UiTextTone};

use demos::GalleryDemos;
use layout::{HEADER, PAGE};

/// Root for the embedded `dist/` bundle: long scrollable catalog of all vendored components.
#[component]
pub fn UiLibraryGallery() -> Element {
    rsx! {
        document::Stylesheet { href: asset!("/assets/input.css") }
        div { class: "{PAGE}",
            header { class: "{HEADER}",
                UiStack { class: "gap-1",
                    UiText { size: UiTextSize::Sm, tone: UiTextTone::Accent, "vmux_ui" }
                    UiText { size: UiTextSize::Xs, tone: UiTextTone::Muted,
                        "Full component gallery (DioxusLabs + vmux chrome)"
                    }
                }
            }
            GalleryDemos {}
        }
    }
}
