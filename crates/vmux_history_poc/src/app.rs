#![allow(non_snake_case)]

use dioxus::prelude::*;
use vmux_history_poc::event::{HISTORY_EVENT, HistoryEvent};
use vmux_ui::hooks::use_event_listener;

#[component]
pub fn App() -> Element {
    let mut history = use_signal(Vec::<String>::new);
    let listener = use_event_listener::<HistoryEvent, _>(HISTORY_EVENT, move |data| {
        history.set(data.history);
    });

    rsx! {
        document::Stylesheet { href: asset!("/assets/input.css") }
        div { class: "p-4 font-sans text-neutral-200",
            h1 { class: "mb-2 text-xl", "History POC" }
            if (listener.is_loading)() {
                p {
                    class: "whitespace-pre text-pulse-neutral",
                    "Waiting for `window.cef`…"
                }
            } else if let Some(err) = (listener.error)() {
                p { class: "text-red-300", "{err}" }
            } else {
                for h in history() {
                    p { "{h}" }
                }
            }
        }
    }
}
