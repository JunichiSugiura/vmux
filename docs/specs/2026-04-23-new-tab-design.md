# New Tab Design

## Summary

Replace the current new-tab behavior (spawn Browser with `startup_url`) with a new flow: spawn an empty tab with a glass background, immediately open the command bar, and let the user choose what to open. Dismissing the command bar closes the empty tab.

## Current Behavior

```
Cmd+T → TabCommand::New → spawn Tab + Browser(settings.browser.startup_url)
Cmd+Shift+T → TerminalCommand::New → spawn Tab + Terminal
```

## New Behavior

```
Cmd+T → TabCommand::New
  → spawn Tab entity (no Browser/Terminal child)
  → tab renders glass background
  → open command bar with empty input + new_tab context flag

Command bar action:
  "navigate" URL  → attach Browser(url) as child of empty tab
  "terminal"      → attach Terminal as child of empty tab
  "switch_tab"    → despawn empty tab, activate target tab
  "dismiss" (Esc) → despawn empty tab, activate previous tab

Cmd+Shift+T → unchanged, still spawns terminal directly
```

## Command Bar Changes

### Context Flag

`CommandBarOpenEvent` gains a `new_tab: bool` field. When `true`:
- Input starts empty (no prefilled URL)
- The empty tab's entity ID is passed so the action handler knows which tab to attach children to

### Result Ordering (new tab mode, empty input)

1. **Terminal** -- always shown as first result
2. **Existing tabs** from active space
3. **Commands**

### Result Ordering (new tab mode, with text input)

1. **Search/Navigate "{query}"** -- top result
2. **Terminal** -- if query matches
3. **Filtered tabs**
4. **Filtered commands**

### New Result Type: Terminal

Add `ResultItem::Terminal` to the command bar's result types. Selecting it emits `CommandBarActionEvent { action: "terminal", value: "" }`.

## Empty Tab Rendering

An empty tab has no CEF child view. The tab area is a Bevy `Node` with a semi-transparent `BackgroundColor` component to achieve the glass effect. No new webview is needed.

## Component Changes

### 1. `crates/vmux_desktop/src/layout/tab.rs`

**`TabCommand::New` handler (~line 141):**
- Stop spawning `Browser` child with `startup_url`
- Spawn tab entity with `tab_bundle()` only (no Browser/Terminal child)
- Add `BackgroundColor` with semi-transparent color for glass effect
- After spawning, trigger command bar open with `new_tab: true` context

### 2. `crates/vmux_desktop/src/command_bar.rs`

**`handle_open_command_bar` (~line 59):**
- Accept `new_tab` context (e.g., via a resource or event payload)
- When `new_tab: true`, send `CommandBarOpenEvent` with empty URL and `new_tab: true`
- Store the empty tab's entity ID in a resource for the action handler

**`on_command_bar_action` (~line 203):**
- When in new_tab mode:
  - `"navigate"`: spawn `Browser(url)` as child of the stored empty tab entity, remove `BackgroundColor`
  - `"terminal"`: spawn `Terminal` as child of the stored empty tab entity, remove `BackgroundColor`
  - `"switch_tab"`: despawn the empty tab entity, then activate the target tab
  - `"dismiss"` / default: despawn the empty tab entity, activate the previously active tab
- When not in new_tab mode: existing behavior unchanged

### 3. `crates/vmux_command_bar/src/event.rs`

**`CommandBarOpenEvent`:**
- Add `new_tab: bool` field

**`CommandBarActionEvent`:**
- Add `"terminal"` as a recognized action value

### 4. `crates/vmux_command_bar/src/app.rs`

**Result filtering:**
- Add `ResultItem::Terminal` variant
- When `new_tab: true` and input is empty, show Terminal as first result
- When `new_tab: true` and input has text, show Terminal if it fuzzy-matches the query
- Selecting Terminal emits `CommandBarActionEvent { action: "terminal", value: "" }`

**Input behavior:**
- When `new_tab: true`, start with empty input (no prefilled URL)

### 5. Tab rendering / header sync

**`push_tabs_host_emit` system:**
- Empty tabs (no Browser/Terminal child) should either be excluded from the tab list sent to the header, or sent with a placeholder indicating "empty" state. Since the command bar is immediately open, this is a transient state.

## Edge Cases

| Scenario | Behavior |
|---|---|
| Dismiss command bar (Esc) on empty tab | Despawn empty tab, activate previous tab |
| Cmd+T when empty tab already focused | Refocus command bar (no second empty tab) |
| Navigate to `vmux://terminal/...` URL | Attach Terminal instead of Browser |
| Cmd+L on empty tab | Re-open command bar in new_tab mode |
| Cmd+Shift+T | Spawns terminal directly, no command bar |
| Last tab is empty and dismissed | If it's the only tab in the pane, despawn the empty tab and let existing "last tab closed" logic handle it (same as closing any other tab) |

## Out of Scope

- New tab page with shortcuts/bookmarks/recent URLs
- Bookmark management
- Search engine configuration
- Tab preview thumbnails
