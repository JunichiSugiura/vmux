//! Global command-palette state (⌘T / Ctrl+T, ⌘L / Ctrl+L) shared with input/CEF suppression.

use bevy::prelude::*;

/// Fixed palette list row count; must match `ROWS_MAX` in `vmux_command` (`build_palette_rows`).
pub const VMUX_PALETTE_ROW_COUNT: usize = 42;

/// Loopback base URL for the **debug-only** vmux UI library bundle (`:debug vmux ui` in the palette).
///
/// Populated in debug builds by `vmux_ui` (UI library host); always [`None`] in release.
#[derive(Resource, Default, Clone)]
pub struct VmuxUiLibraryBaseUrl(pub Option<String>);

/// When the command palette requests “Open UI library” before [`VmuxUiLibraryBaseUrl`] is populated,
/// we queue navigation here until the loopback URL is ready.
#[derive(Resource, Default)]
pub struct VmuxPendingUiLibraryNavigation {
    pub inner: Option<VmuxPendingUiLibraryNavTarget>,
    pub wait_frames: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct VmuxPendingUiLibraryNavTarget {
    pub new_pane: bool,
}

/// Open/closed state, query string, and list selection for the centered command palette.
#[derive(Resource, Clone)]
pub struct VmuxCommandPaletteState {
    pub open: bool,
    pub input: CommandPaletteInputState,
    /// Row index in one list: open-tab slots first, then omnibox / web / history or GitHub / commands.
    pub selection: usize,
    /// Copy of [`VmuxUiLibraryBaseUrl`] for omnibox resolution without extra `SystemParam` slots.
    pub ui_library_base: Option<String>,
    /// When true, moving the mouse over list rows updates [`Self::selection`]. Set to false when
    /// using ↑/↓ (or Ctrl+N/P) so keyboard navigation is not overwritten until the mouse moves again.
    pub pointer_row_selects: bool,
    /// Primary pointer pressed a list row; consumed by the palette submit pass (same effect as Enter).
    pub pending_pointer_submit: bool,
    /// Per-row selectable flag (mirrors `PaletteRowSpec::selectable` in `vmux_command`); drives list styling.
    pub row_selectable_mask: [bool; VMUX_PALETTE_ROW_COUNT],
}

impl Default for VmuxCommandPaletteState {
    fn default() -> Self {
        Self {
            open: false,
            input: CommandPaletteInputState::default(),
            selection: 0,
            ui_library_base: None,
            pointer_row_selects: true,
            pending_pointer_submit: false,
            row_selectable_mask: [true; VMUX_PALETTE_ROW_COUNT],
        }
    }
}

/// Query text-edit model (string + caret/selection) for browser-like palette editing.
#[derive(Default, Clone)]
pub struct CommandPaletteInputState {
    pub query: String,
    /// Caret position in `query` (character index).
    pub caret: usize,
    /// Optional selection anchor (character index). Selection is active when this is `Some` and differs from `caret`.
    pub selection_anchor: Option<usize>,
    /// [`Time::elapsed_secs`] when the palette was last opened; resets caret blink phase.
    pub caret_blink_t0: f32,
}
