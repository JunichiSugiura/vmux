//! Bordered surface panel (loading shells, empty states, etc.).

use super::util::merge_class;
use dioxus::prelude::*;

#[component]
pub fn UiPanel(
    /// When `true`, `class` replaces the default surface classes entirely (use for alternate empty-state shells).
    #[props(default)]
    replace_default: bool,
    #[props(default)] class: Option<String>,
    #[props(default)] aria_busy: Option<bool>,
    #[props(default)] aria_label: Option<String>,
    children: Element,
) -> Element {
    let default = "flex flex-col items-center justify-center gap-3 rounded-2xl border border-dashed border-white/10 bg-white/[0.02] px-6 py-14 text-center";
    let c = if replace_default {
        class.unwrap_or_else(|| default.to_string())
    } else {
        merge_class(default, class.as_deref())
    };
    let ab = |b: bool| if b { "true" } else { "false" };
    match (aria_label, aria_busy) {
        (None, None) => rsx! {
            div { class: "{c}", {children} }
        },
        (Some(l), None) => rsx! {
            div { class: "{c}", aria_label: "{l}", {children} }
        },
        (None, Some(b)) => {
            let s = ab(b);
            rsx! {
                div { class: "{c}", aria_busy: "{s}", {children} }
            }
        }
        (Some(l), Some(b)) => {
            let s = ab(b);
            rsx! {
                div { class: "{c}", aria_label: "{l}", aria_busy: "{s}", {children} }
            }
        }
    }
}
