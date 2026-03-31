//! Relative wrapper: leading affordance + text input (leading first in DOM; icon uses flex centering
//! in the left inset so it stays aligned with padded inputs).

use super::util::merge_class;
use dioxus::prelude::*;

#[component]
pub fn UiInputShell(
    #[props(default)] class: Option<String>,
    #[props(default)] leading_class: Option<String>,
    leading: Element,
    input: Element,
) -> Element {
    let outer = merge_class("relative", class.as_deref());
    let lc = merge_class(
        "pointer-events-none absolute inset-y-0 left-0 z-[1] flex w-9 shrink-0 items-center justify-center text-white/28",
        leading_class.as_deref(),
    );
    rsx! {
        div { class: "{outer}",
            span { class: "{lc}", aria_hidden: true, {leading} }
            {input}
        }
    }
}
