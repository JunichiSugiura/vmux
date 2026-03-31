//! Hosted web UIs: static assets served over loopback and bound to a CEF surface (see [`VmuxWebviewSurface`]).
//!
//! ## Plugin order
//!
//! Implementations are [`ServePlugin`](vmux_server::ServePlugin) types (`*ServerPlugin` crates).
//! Register embedded HTTP against [`VmuxServerShutdownRegistry`](vmux_server::VmuxServerShutdownRegistry)
//! after [`ServerPlugin`](vmux_server::ServerPlugin) (add `ServerPlugin` before the hosted server plugin).

use vmux_server::ServePlugin;

/// Which vmux CEF surface a hosted UI targets.
///
/// Type-level label for [`VmuxHostedWebPlugin`]; entities still use [`Pane`](crate::Pane),
/// [`PaneChromeStrip`](crate::PaneChromeStrip), etc.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VmuxWebviewSurface {
    /// Primary document webview for each layout pane.
    MainPane,
    /// Bottom status / chrome strip ([`PaneChromeStrip`](crate::PaneChromeStrip)).
    PaneChrome,
    /// Tiled history pane ([`Pane`] + [`Webview`] + [`History`](crate::History)).
    HistoryPane,
}

/// A [`ServePlugin`](vmux_server::ServePlugin) (`*ServerPlugin`) that serves a web app from loopback and maps to a [`VmuxWebviewSurface`].
pub trait VmuxHostedWebPlugin: ServePlugin {
    const SURFACE: VmuxWebviewSurface;
}
