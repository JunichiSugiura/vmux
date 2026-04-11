#![allow(non_snake_case)]

use dioxus::prelude::*;
use vmux_status_bar::event::{TABS_EVENT, TabsHostEvent};
use vmux_ui::hooks::use_event_listener;

#[component]
pub fn App() -> Element {
    let mut tabs_state = use_signal(TabsHostEvent::default);
    let listener = use_event_listener::<TabsHostEvent, _>(TABS_EVENT, move |data| {
        tabs_state.set(data);
    });

    rsx! {
        document::Stylesheet { href: asset!("/assets/input.css") }
        div { class: "flex min-h-0 min-w-0 flex-1 flex-row items-stretch border-t border-border bg-card",
            if (listener.is_loading)() {
                div { class: "flex flex-1 items-center px-3",
                    span { class: "text-ui-xs text-muted-foreground", "Connecting…" }
                }
            } else if let Some(err) = (listener.error)() {
                div { class: "flex flex-1 items-center px-3",
                    span { class: "text-ui-xs text-destructive", "{err}" }
                }
            } else {
                div { class: "flex min-w-0 flex-1 flex-row items-stretch gap-1 overflow-x-auto px-2 py-1",
                    for row in tabs_state().tabs {
                        div {
                            class: if row.is_active {
                                "flex min-w-0 max-w-40 shrink-0 items-center rounded-md border border-border bg-muted px-2 py-1"
                            } else {
                                "flex min-w-0 max-w-40 shrink-0 items-center rounded-md border border-transparent px-2 py-1 opacity-80"
                            },
                            span { class: "truncate text-ui-xs text-foreground", "{row.title}" }
                        }
                    }
                }
            }
        }
    }
}
