# Multiple Space Support — Design

Linear: [VMX-19](https://linear.app/vmux/issue/VMX-19/multiple-space-support)

## Goal

Allow users to create, switch between, close, swap, and visualize multiple spaces — analogous to tmux windows. Always-visible footer surfaces the space list and creation affordance.

## Decisions

- **Footer**: always visible (not toggle-gated, not auto-hidden).
- **Interactivity**: click pill to switch active space; trailing `+` button creates a new space.
- **Naming**: auto-named `Space N` (N = current count + 1) on creation. `Rename` command stays defined but is a no-op for this PR.
- **Closing the last space**: no-op (one space minimum).
- **Persistence**: existing `Save` derive on `Space` already persists via `moonshine_save`.

## Shortcuts (tmux-style leader chord)

Leader defaults to `Ctrl+g`; user-configurable. Already declared in `vmux_desktop/src/command.rs`; this PR adds handlers.

| Command         | Chord            |
|-----------------|------------------|
| New             | `<leader> c`     |
| Close           | `<leader> &`     |
| Next            | `<leader> n`     |
| Previous        | `<leader> p`     |
| Rename (no-op)  | `<leader> ,`     |
| SwapPrev        | (existing)       |
| SwapNext        | (existing)       |

## Architecture

```
Bevy ECS                                                CEF webview (Dioxus)
─────────                                               ────────────────────
SpaceCommand events ──► layout/space.rs handlers
                          │
                          ▼
                       spawn / despawn / activate Space entities
                          │
                          ▼
                       sync_spaces_to_footer  ──SpacesHostEvent──► vmux_footer App
                                                                    │
                                                                    ▼
                       handle_footer_command ◄──FooterCommandEvent──┘
```

### Components

- **New crate `vmux_footer`** mirrors `vmux_header`:
  - `bundle.rs`: `Footer` marker + `FooterBundle` (CEF webview, `vmux://footer/`).
  - `event.rs`: `SpacesHostEvent { spaces: Vec<SpaceRow> }`, `FooterCommandEvent { command, space_id? }`.
  - `app.rs`: Dioxus UI — pills + `+` button.
  - `plugin.rs`: registers webview-app config.
  - `system.rs`: `FOOTER_HEIGHT_PX = 32.0`.

- **`vmux_desktop::layout::space`**:
  - Implement `New`, `Close`, `Next`, `Previous` handlers.
  - Add `sync_spaces_to_footer` system — emits `SpacesHostEvent` when set of `Space` entities or active changes.
  - Add observer for `FooterCommandEvent` — dispatches to handler logic.

- **`vmux_desktop::layout::window`**:
  - Replace existing `BottomBar` placeholder with `FooterBundle` (height 32px, flex-shrink 0).

- **`vmux_desktop::lib`**: register `FooterPlugin`.

### Data model

- `Space.name: String` — set to `format!("Space {n}")` on `New`.
- Active space: `max LastActivatedAt` (existing pattern preserved).
- Footer-side ID: `Entity.to_bits().to_string()` — ephemeral, per-session. JS sends back the same string for `switch`.

### Event payloads

```rust
struct SpaceRow {
    id: String,        // Entity bits as string
    name: String,
    is_active: bool,
}
struct SpacesHostEvent { spaces: Vec<SpaceRow> }
struct FooterCommandEvent {
    command: String,        // "switch" | "new"
    space_id: Option<String>,
}
```

### Handler semantics

| Command  | Behavior |
|----------|----------|
| New      | Spawn `space_bundle` as child of `Main` with auto name + `LastActivatedAt::now()` + `CreatedAt::now()`; spawn empty pane + tab via existing `NewTabContext` flow; open command bar (mirror first-launch). |
| Close    | If `>1` Space, despawn active subtree; activate next sibling (wrap). Else no-op. |
| Next     | Cycle to next sibling Space, set `LastActivatedAt::now()`. Wraps. |
| Previous | Same as `Next`, opposite direction. |
| Rename   | No-op (logs trace). |

### Footer UI

```
┌────────────────────────────────────────────────────────────────────┐
│  [1 · Space 1]  [2 · Space 2]  [3 · Space 3 (active)]   ...   [+]  │
└────────────────────────────────────────────────────────────────────┘
```

- Always-visible row at window bottom.
- Each pill: `{index} · {name}`, click → switch.
- Active pill gets glass highlight (matches header pattern).
- Trailing `+` button → `FooterCommandEvent { command: "new" }`.
- Tailwind classes consistent with `vmux_header`.

## Files

```
+ crates/vmux_footer/Cargo.toml
+ crates/vmux_footer/index.html
+ crates/vmux_footer/src/{lib.rs, bundle.rs, event.rs, system.rs, plugin.rs, app.rs, main.rs}
M crates/vmux_desktop/src/layout/space.rs       (impl handlers + sync system + footer observer)
M crates/vmux_desktop/src/layout/window.rs      (BottomBar → FooterBundle)
M crates/vmux_desktop/src/lib.rs                (register FooterPlugin)
M Cargo.toml                                    (workspace member)
```

## Out of scope (this PR)

- Rename UI (command bar prompt) — reserved for follow-up.
- Drag-to-reorder spaces — `Swap*` commands cover keyboard reordering.
- Per-space settings / theming.
- Visual transitions between spaces.

## Verification

- Spawn 3 spaces, verify pills appear in footer in order.
- Switch via click and via `<leader> n`/`<leader> p`.
- Close active space — next becomes active; closing last is no-op.
- Restart app — spaces and active selection persist.
- All clippy + tests pass per `AGENTS.md` pre-commit checks.
