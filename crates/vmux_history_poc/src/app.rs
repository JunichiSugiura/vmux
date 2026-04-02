//! Dioxus UI: preload calls `window.__vmuxHandshakeApply` (a wasm `Closure`) so the handshake
//! updates a [`Signal`] directly; globals are only a fallback if BRP finishes before registration.

use std::cell::Cell;
use std::rc::Rc;

use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;
use serde::Deserialize;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::window;

#[derive(Deserialize)]
struct HostHistoryPayload {
    url: Option<String>,
}

fn format_handshake_message(raw: &str) -> String {
    match serde_json::from_str::<HostHistoryPayload>(raw) {
        Ok(p) => p
            .url
            .filter(|s| !s.is_empty())
            .map(|u| format!("received host handshake: {u}"))
            .unwrap_or_else(|| format!("received host payload (no url): {raw}")),
        Err(_) => format!("received host payload: {raw}"),
    }
}

/// If preload fell back to globals (WASM not ready yet), read them once polling succeeds.
async fn wait_for_preload_handshake_globals() -> Result<String, String> {
    let win = window().ok_or_else(|| "no `window`".to_string())?;
    for _ in 0..400 {
        let err = js_sys::Reflect::get(
            &win,
            &JsValue::from_str(crate::bridge::HANDSHAKE_ERROR_GLOBAL),
        )
        .ok();
        if let Some(e) = err.filter(|v| !v.is_undefined() && !v.is_null()) {
            return Err(
                e.as_string()
                    .unwrap_or_else(|| format!("handshake error: {e:?}")),
            );
        }
        let ok = js_sys::Reflect::get(
            &win,
            &JsValue::from_str(crate::bridge::HANDSHAKE_RESULT_GLOBAL),
        )
        .ok();
        if let Some(v) = ok.filter(|x| !x.is_undefined() && !x.is_null()) {
            return js_sys::JSON::stringify(&v)
                .ok()
                .and_then(|s| s.as_string())
                .filter(|s| !s.is_empty())
                .ok_or_else(|| "handshake value not JSON-serializable".to_string());
        }
        TimeoutFuture::new(25).await;
    }
    Err("timeout waiting for preload BRP handshake".to_string())
}

#[component]
pub fn App() -> Element {
    let status = use_signal(|| "Waiting for preload `cef.brp` handshake…".to_string());

    use_hook(move || {
        let applied = Rc::new(Cell::new(false));
        let win = window().expect("window");

        let applied_ok = Rc::clone(&applied);
        let mut label_ok = status;
        let on_ok = Closure::new(move |v: JsValue| {
            if applied_ok.get() {
                return;
            }
            applied_ok.set(true);
            let Some(json) = js_sys::JSON::stringify(&v)
                .ok()
                .and_then(|s| s.as_string())
                .filter(|s| !s.is_empty())
            else {
                return;
            };
            label_ok.set(format_handshake_message(&json));
        });
        let _ = js_sys::Reflect::set(
            &win,
            &JsValue::from_str(crate::bridge::HANDSHAKE_APPLY_FN),
            on_ok.as_ref().unchecked_ref(),
        );
        on_ok.forget();

        let applied_err = Rc::clone(&applied);
        let mut label_err = status;
        let on_err = Closure::new(move |msg: JsValue| {
            if applied_err.get() {
                return;
            }
            applied_err.set(true);
            let s = msg
                .as_string()
                .unwrap_or_else(|| format!("{msg:?}"));
            label_err.set(format!("Handshake: {s}"));
        });
        let _ = js_sys::Reflect::set(
            &win,
            &JsValue::from_str(crate::bridge::HANDSHAKE_APPLY_ERROR_FN),
            on_err.as_ref().unchecked_ref(),
        );
        on_err.forget();

        let applied_poll = Rc::clone(&applied);
        let mut label_poll = status;
        spawn(async move {
            match wait_for_preload_handshake_globals().await {
                Ok(json) => {
                    if applied_poll.get() {
                        return;
                    }
                    applied_poll.set(true);
                    label_poll.set(format_handshake_message(&json));
                }
                Err(e) => {
                    if applied_poll.get() {
                        return;
                    }
                    applied_poll.set(true);
                    label_poll.set(format!("Handshake: {e}"));
                }
            }
        });
    });

    rsx! {
        document::Stylesheet { href: asset!("/assets/input.css") }
        div { class: "p-4 font-sans text-neutral-200",
            h1 { class: "mb-2 text-xl", "History POC" }
            p { "{status}" }
        }
    }
}
