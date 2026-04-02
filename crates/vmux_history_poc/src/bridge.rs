//! CEF IPC channel id for [Host Emit](https://not-elm.github.io/bevy_cef/communication/) (Bevy → JS).
//! JS → Bevy ready uses a single JSON object passed to `cef.emit` (see `try_emit_ui_ready`).

use std::fmt;

/// `window.cef.listen` / host `HostEmitEvent` channel for the history JSON payload.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub const HOST_HISTORY_CHANNEL: &str = "vmux_history_poc_history";

/// Why `cef.listen` / `cef.emit` could not run (WASM UI only; unused on native host builds).
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CefBridgeError {
    NoWindow,
    NoCefGlobal,
    CefNotInjected,
    NoListenMethod,
    ListenNotCallable,
    NoEmitMethod,
    EmitNotCallable,
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
impl fmt::Display for CefBridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NoWindow => "no `window`",
            Self::NoCefGlobal => "no `window.cef` property",
            Self::CefNotInjected => "`window.cef` not ready",
            Self::NoListenMethod => "no `cef.listen`",
            Self::ListenNotCallable => "`cef.listen` is not a function",
            Self::NoEmitMethod => "no `cef.emit`",
            Self::EmitNotCallable => "`cef.emit` is not a function",
        })
    }
}

#[cfg(target_arch = "wasm32")]
use js_sys::Function;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::closure::Closure;
#[cfg(target_arch = "wasm32")]
use web_sys::window;

/// Register `cef.listen` when `window.cef` exists.
#[cfg(target_arch = "wasm32")]
pub fn try_cef_listen<F>(channel: &str, on_event: F) -> Result<(), CefBridgeError>
where
    F: FnMut(JsValue) + 'static,
{
    let Some(win) = window() else {
        return Err(CefBridgeError::NoWindow);
    };
    let Ok(cef) = js_sys::Reflect::get(&win, &JsValue::from_str("cef")) else {
        return Err(CefBridgeError::NoCefGlobal);
    };
    if cef.is_null() || cef.is_undefined() {
        return Err(CefBridgeError::CefNotInjected);
    }
    let Ok(listen) = js_sys::Reflect::get(&cef, &JsValue::from_str("listen")) else {
        return Err(CefBridgeError::NoListenMethod);
    };
    let Ok(listen_fn) = listen.dyn_into::<Function>() else {
        return Err(CefBridgeError::ListenNotCallable);
    };

    let mut on_event = on_event;
    let closure = Closure::wrap(Box::new(move |e: JsValue| {
        on_event(e);
    }) as Box<dyn FnMut(JsValue)>);

    let cb = closure.as_ref().unchecked_ref();
    let _ = listen_fn.call2(&cef, &JsValue::from_str(channel), cb);
    closure.forget();
    Ok(())
}

/// Emit `{}` for JS Emit after the host-history listener is registered (matches `HistoryUiReady` on the Bevy side).
#[cfg(target_arch = "wasm32")]
pub fn try_emit_ui_ready() -> Result<(), CefBridgeError> {
    let Some(win) = window() else {
        return Err(CefBridgeError::NoWindow);
    };
    let Ok(cef) = js_sys::Reflect::get(&win, &JsValue::from_str("cef")) else {
        return Err(CefBridgeError::NoCefGlobal);
    };
    if cef.is_null() || cef.is_undefined() {
        return Err(CefBridgeError::CefNotInjected);
    }
    let Ok(emit) = js_sys::Reflect::get(&cef, &JsValue::from_str("emit")) else {
        return Err(CefBridgeError::NoEmitMethod);
    };
    let Ok(emit_fn) = emit.dyn_into::<Function>() else {
        return Err(CefBridgeError::EmitNotCallable);
    };
    let obj = js_sys::Object::new();
    let obj: JsValue = obj.into();
    let _ = emit_fn.call1(&cef, &obj);
    Ok(())
}
