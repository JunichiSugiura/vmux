//! Dioxus UI: register `cef.listen` for host history, then `cef.emit({})` so Bevy can mark the webview
//! ready and push history via Host Emit ([communication patterns](https://not-elm.github.io/bevy_cef/communication/)).

use dioxus::prelude::*;
use serde::Deserialize;
use wasm_bindgen::JsValue;

#[derive(Deserialize)]
struct HostHistoryPayload {
    url: Option<String>,
    history: Option<Vec<String>>,
}

fn format_history_status(raw: &str) -> String {
    match serde_json::from_str::<HostHistoryPayload>(raw) {
        Ok(p) => {
            let n = p.history.as_ref().map(Vec::len).unwrap_or(0);
            let url = p.url.as_deref().unwrap_or("");
            if n > 0 {
                format!("Host history: {n} entries (url summary: {url})")
            } else if !url.is_empty() {
                format!("Host history: {url}")
            } else {
                format!("Host payload: {raw}")
            }
        }
        Err(_) => format!("Host payload: {raw}"),
    }
}

#[component]
pub fn App() -> Element {
    let status = use_signal(|| "Waiting for `window.cef`…".to_string());

    use_hook(move || {
        let mut label = status;
        let mut line = label;
        match crate::bridge::try_cef_listen(
            crate::bridge::HOST_HISTORY_CHANNEL,
            move |e: JsValue| {
                let json = js_sys::JSON::stringify(&e)
                    .ok()
                    .and_then(|s| s.as_string())
                    .unwrap_or_else(|| e.as_string().unwrap_or_default());
                line.set(format_history_status(&json));
            },
        ) {
            Ok(()) => {
                label.set("Listener registered; notifying host…".to_string());
                match crate::bridge::try_emit_ui_ready() {
                    Ok(()) => label.set("Waiting for host history…".to_string()),
                    Err(e) => label.set(format!("cef.emit failed: {e}")),
                }
            }
            Err(e) => label.set(format!("cef.listen failed: {e}")),
        }
    });

    rsx! {
        document::Stylesheet { href: asset!("/assets/input.css") }
        div { class: "p-4 font-sans text-neutral-200",
            h1 { class: "mb-2 text-xl", "History POC" }
            p { "{status}" }
        }
    }
}
