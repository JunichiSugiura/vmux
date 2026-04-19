# Commands and Keybindings Spec

## Goal

Define the complete command inventory for vmux with Chromium-style keybindings for tab/browser commands and tmux-style chord bindings for pane/space management. Add all command variants to the enum definitions with OS menu accelerators and keybindings. Unimplemented commands are no-op stubs.

## Keybinding System

Two binding mechanisms:

- `accel = "super+t"` on `#[menu(...)]` — OS-level menu accelerator via muda. Shown in native menu bar.
- `#[bind(direct = "super+t")]` or `#[bind(chord = "ctrl+b, v")]` — Custom keybinding system. `direct` for single-combo shortcuts. `chord` for tmux-style leader+key sequences.

Both can coexist on the same variant. `accel` requires `id` and `label` in `#[menu(...)]`.

For commands with chord bindings but no `accel`, the chord is embedded in the menu label text itself, e.g. `"Split Vertically\tCtrl+B, V"`. This displays the chord right-aligned in the menu item since native accelerators don't support two-step sequences.

## Handler Status Legend

- ✅ Implemented
- 🔲 Stub (no-op `{}` match arm)

## Command Definitions

### TabCommand

| Variant | menu id | label | accel | Menu Display | bind | Handler |
|---------|---------|-------|-------|--------------|------|---------|
| New | tab_new | New Tab | super+t | New Tab  Cmd+T | | ✅ |
| Close | tab_close | Close Tab | super+w | Close Tab  Cmd+W | | ✅ |
| Next | tab_next | Next Tab | super+shift+] | Next Tab  Cmd+Shift+] | | ✅ |
| Previous | tab_previous | Previous Tab | super+shift+[ | Previous Tab  Cmd+Shift+[ | | ✅ |
| SelectIndex1 | tab_select_1 | Select Tab 1 | super+1 | Select Tab 1  Cmd+1 | | ✅ |
| SelectIndex2 | tab_select_2 | Select Tab 2 | super+2 | Select Tab 2  Cmd+2 | | ✅ |
| SelectIndex3 | tab_select_3 | Select Tab 3 | super+3 | Select Tab 3  Cmd+3 | | ✅ |
| SelectIndex4 | tab_select_4 | Select Tab 4 | super+4 | Select Tab 4  Cmd+4 | | ✅ |
| SelectIndex5 | tab_select_5 | Select Tab 5 | super+5 | Select Tab 5  Cmd+5 | | ✅ |
| SelectIndex6 | tab_select_6 | Select Tab 6 | super+6 | Select Tab 6  Cmd+6 | | ✅ |
| SelectIndex7 | tab_select_7 | Select Tab 7 | super+7 | Select Tab 7  Cmd+7 | | ✅ |
| SelectIndex8 | tab_select_8 | Select Tab 8 | super+8 | Select Tab 8  Cmd+8 | | ✅ |
| SelectLast | tab_select_last | Select Last Tab | super+9 | Select Last Tab  Cmd+9 | | ✅ |
| Reopen | tab_reopen | Reopen Closed Tab | super+shift+t | Reopen Closed Tab  Cmd+Shift+T | | 🔲 |
| Duplicate | tab_duplicate | Duplicate Tab | | Duplicate Tab | | 🔲 |
| Pin | tab_pin | Pin Tab | | Pin Tab | | 🔲 |
| Mute | tab_mute | Mute Tab | | Mute Tab | | 🔲 |
| MoveToPane | tab_move_to_pane | Move Tab to Pane | | Move Tab to Pane | | 🔲 |

### BrowserCommand

| Variant | menu id | label | accel | Menu Display | bind | Handler |
|---------|---------|-------|-------|--------------|------|---------|
| PrevPage | browser_prev_page | Back | super+[ | Back  Cmd+[ | | ✅ |
| NextPage | browser_next_page | Forward | super+] | Forward  Cmd+] | | ✅ |
| Reload | browser_reload | Reload | super+r | Reload  Cmd+R | | ✅ |
| HardReload | browser_hard_reload | Hard Reload | super+shift+r | Hard Reload  Cmd+Shift+R | | ✅ |
| Stop | browser_stop | Stop Loading | | Stop Loading | | 🔲 |
| FocusAddressBar | browser_focus_address_bar | Open Location | super+l | Open Location  Cmd+L | | 🔲 |
| Find | browser_find | Find | super+f | Find  Cmd+F | | 🔲 |
| ZoomIn | browser_zoom_in | Zoom In | super+= | Zoom In  Cmd+= | | ✅ |
| ZoomOut | browser_zoom_out | Zoom Out | super+- | Zoom Out  Cmd+- | | ✅ |
| ZoomReset | browser_zoom_reset | Actual Size | super+0 | Actual Size  Cmd+0 | | ✅ |
| DevTools | browser_dev_tools | Developer Tools | super+alt+i | Developer Tools  Cmd+Opt+I | | ✅ |
| ViewSource | browser_view_source | View Source | super+alt+u | View Source  Cmd+Opt+U | | 🔲 |
| Print | browser_print | Print | super+p | Print  Cmd+P | | 🔲 |

### PaneCommand

| Variant | menu id | label | accel | Menu Display | bind | Handler |
|---------|---------|-------|-------|--------------|------|---------|
| SplitV | split_v | Split Vertically\tCtrl+B, V | | Split Vertically  Ctrl+B, V | ctrl+b, v | ✅ |
| SplitH | split_h | Split Horizontally\tCtrl+B, H | | Split Horizontally  Ctrl+B, H | ctrl+b, h | ✅ |
| Close | close_pane | Close Pane\tCtrl+B, X | | Close Pane  Ctrl+B, X | ctrl+b, x | ✅ |
| Toggle | toggle_pane | Toggle Pane\tCtrl+B, T | | Toggle Pane  Ctrl+B, T | ctrl+b, t | 🔲 |
| Zoom | zoom_pane | Zoom Pane\tCtrl+B, Z | | Zoom Pane  Ctrl+B, Z | ctrl+b, z | 🔲 |
| SelectLeft | select_pane_left | Select Left Pane\tCtrl+B, Left | | Select Left Pane  Ctrl+B, Left | ctrl+b, left | ✅ |
| SelectRight | select_pane_right | Select Right Pane\tCtrl+B, Right | | Select Right Pane  Ctrl+B, Right | ctrl+b, right | ✅ |
| SelectUp | select_pane_up | Select Up Pane\tCtrl+B, Up | | Select Up Pane  Ctrl+B, Up | ctrl+b, up | ✅ |
| SelectDown | select_pane_down | Select Down Pane\tCtrl+B, Down | | Select Down Pane  Ctrl+B, Down | ctrl+b, down | ✅ |
| SwapPrev | swap_pane_prev | Swap Pane Previous\tCtrl+B, { | | Swap Pane Previous  Ctrl+B, { | ctrl+b, { | 🔲 |
| SwapNext | swap_pane_next | Swap Pane Next\tCtrl+B, } | | Swap Pane Next  Ctrl+B, } | ctrl+b, } | 🔲 |
| RotateForward | rotate_forward | Rotate Forward\tCtrl+B, Ctrl+O | | Rotate Forward  Ctrl+B, Ctrl+O | ctrl+b, ctrl+o | 🔲 |
| RotateBackward | rotate_backward | Rotate Backward\tCtrl+B, Opt+O | | Rotate Backward  Ctrl+B, Opt+O | ctrl+b, alt+o | 🔲 |
| EqualizeSize | equalize_pane_size | Equalize Pane Size\tCtrl+B, = | | Equalize Pane Size  Ctrl+B, = | ctrl+b, = | 🔲 |
| ResizeLeft | resize_pane_left | Resize Pane Left\tCtrl+B, Opt+Left | | Resize Pane Left  Ctrl+B, Opt+Left | ctrl+b, alt+left | 🔲 |
| ResizeRight | resize_pane_right | Resize Pane Right\tCtrl+B, Opt+Right | | Resize Pane Right  Ctrl+B, Opt+Right | ctrl+b, alt+right | 🔲 |
| ResizeUp | resize_pane_up | Resize Pane Up\tCtrl+B, Opt+Up | | Resize Pane Up  Ctrl+B, Opt+Up | ctrl+b, alt+up | 🔲 |
| ResizeDown | resize_pane_down | Resize Pane Down\tCtrl+B, Opt+Down | | Resize Pane Down  Ctrl+B, Opt+Down | ctrl+b, alt+down | 🔲 |

### SpaceCommand

| Variant | menu id | label | accel | Menu Display | bind | Handler |
|---------|---------|-------|-------|--------------|------|---------|
| New | new_space | New Space\tCtrl+B, C | | New Space  Ctrl+B, C | ctrl+b, c | 🔲 |
| Close | close_space | Close Space\tCtrl+B, & | | Close Space  Ctrl+B, & | ctrl+b, & | 🔲 |
| Next | next_space | Next Space | ctrl+tab | Next Space  Ctrl+Tab | | 🔲 |
| Previous | prev_space | Previous Space | ctrl+shift+tab | Previous Space  Ctrl+Shift+Tab | | 🔲 |
| Rename | rename_space | Rename Space | | Rename Space | | 🔲 |

### SideSheetCommand

| Variant | menu id | label | accel | Menu Display | bind | Handler |
|---------|---------|-------|-------|--------------|------|---------|
| Toggle | toggle_side_sheet | Toggle Side Sheet\tCtrl+B, S | | Toggle Side Sheet  Ctrl+B, S | ctrl+b, s | ✅ |
| ToggleRight | toggle_side_sheet_right | Toggle Right Sheet | | Toggle Right Sheet | | 🔲 |
| ToggleBottom | toggle_side_sheet_bottom | Toggle Bottom Sheet | | Toggle Bottom Sheet | | 🔲 |

### WindowCommand (new)

| Variant | menu id | label | accel | Menu Display | bind | Handler |
|---------|---------|-------|-------|--------------|------|---------|
| NewWindow | new_window | New Window | super+n | New Window  Cmd+N | | 🔲 |
| CloseWindow | close_window | Close Window | super+shift+w | Close Window  Cmd+Shift+W | | 🔲 |
| Minimize | minimize_window | Minimize | super+m | Minimize  Cmd+M | | 🔲 |
| ToggleFullscreen | toggle_fullscreen | Toggle Fullscreen | ctrl+super+f | Toggle Fullscreen  Ctrl+Cmd+F | | 🔲 |
| Settings | open_settings | Settings | super+, | Settings  Cmd+, | | 🔲 |

### CameraCommand (unchanged)

| Variant | menu id | label | accel | Menu Display | bind | Handler |
|---------|---------|-------|-------|--------------|------|---------|
| Reset | reset_camera | Reset Camera | | Reset Camera | | ✅ |
| ToggleFreeCamera | toggle_free_camera | Toggle Free Camera | | Toggle Free Camera | | ✅ |

## Summary

| Category | ✅ | 🔲 | Total |
|----------|:--:|:--:|:-----:|
| Tab | 13 | 5 | 18 |
| Browser | 8 | 5 | 13 |
| Pane | 7 | 10 | 17 |
| Space | 0 | 5 | 5 |
| SideSheet | 1 | 2 | 3 |
| Window | 0 | 5 | 5 |
| Camera | 2 | 0 | 2 |
| **Total** | **31** | **32** | **63** |

## Behavioral Changes

### Tab Next/Previous Fix

Currently `on_pane_cycle` in `pane.rs` intercepts `TabCommand::Next` and `TabCommand::Previous` to cycle between panes. This must change:

- `TabCommand::Next/Previous` cycles tabs within the active pane
- Pane cycling moves to `PaneCommand` variants (e.g. SelectLeft/Right or a dedicated cycle command)

The `on_pane_cycle` system should stop listening for `TabCommand::Next/Previous`. Instead, `handle_tab_commands` in `tab.rs` should handle Next/Previous by moving Active between Tab siblings within the active pane.

### Tab SelectIndex

`SelectIndex1..8` activates the Nth tab (0-indexed: index 0..7) in the active pane. If the index exceeds tab count, no-op. `SelectLast` activates the last tab regardless of count.

Implementation in `handle_tab_commands`: query children of active pane, filter to Tab entities, sort by entity bits (stable ordering), pick by index, swap Active.

## Scope

### Implemented in this change

- All command enum variants added with `#[menu(...)]` attributes
- All `accel` values on variants that have them
- All `#[bind(...)]` values on variants that have them
- `TabCommand::Next/Previous` handler: cycle tabs in active pane
- `TabCommand::SelectIndex1..8/SelectLast` handler: select tab by index
- Remove `on_pane_cycle` interception of `TabCommand::Next/Previous`
- `BrowserCommand` accel values on existing PrevPage/NextPage/Reload

### Stub only (no-op match arm)

Everything marked 🔲 in the tables above.

## Files Changed

- `crates/vmux_desktop/src/command.rs` — all enum definitions, new WindowCommand
- `crates/vmux_desktop/src/layout/tab.rs` — Next/Previous/SelectIndex handlers
- `crates/vmux_desktop/src/layout/pane.rs` — remove on_pane_cycle TabCommand listener
- `crates/vmux_desktop/src/layout/side_sheet.rs` — new SideSheetCommand variants
- `crates/vmux_desktop/src/layout/space.rs` — new SpaceCommand variant (Rename)
