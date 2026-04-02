/// Must match `RemotePlugin::…with_method(…, …)` on the host (`main.rs`). Only referenced from native code.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(crate) const HANDSHAKE_METHOD: &str = "history_handshake";

/// Preload calls this `window` function when WASM has registered it; the closure updates the UI signal.
pub const HANDSHAKE_APPLY_FN: &str = "__vmuxHandshakeApply";

/// Preload calls this on `cef.brp` rejection when the apply function exists.
pub const HANDSHAKE_APPLY_ERROR_FN: &str = "__vmuxHandshakeApplyError";

/// Fallback if BRP settles before WASM registers [`HANDSHAKE_APPLY_FN`].
pub const HANDSHAKE_RESULT_GLOBAL: &str = "__vmuxHistoryHandshake";

/// Fallback error property (same timing as [`HANDSHAKE_RESULT_GLOBAL`]).
pub const HANDSHAKE_ERROR_GLOBAL: &str = "__vmuxHistoryHandshakeError";
