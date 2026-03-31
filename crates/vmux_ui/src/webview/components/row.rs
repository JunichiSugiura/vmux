//! Horizontal flex row helper (vmux chrome; not part of upstream gallery).

use super::util::merge_class;
use dioxus::prelude::*;

#[component]
pub fn UiRow(#[props(default)] class: Option<String>, children: Element) -> Element {
    let c = merge_class(
        "flex min-w-0 flex-row flex-nowrap items-center gap-1",
        class.as_deref(),
    );
    rsx! {
        div { class: "{c}", {children} }
    }
}
