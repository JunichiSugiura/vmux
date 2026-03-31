//! Vertical flex stack helper (vmux chrome; not part of upstream gallery).

use super::util::merge_class;
use dioxus::prelude::*;

#[component]
pub fn UiStack(#[props(default)] class: Option<String>, children: Element) -> Element {
    let c = merge_class(
        "flex min-w-0 flex-col items-stretch gap-2",
        class.as_deref(),
    );
    rsx! {
        div { class: "{c}", {children} }
    }
}
