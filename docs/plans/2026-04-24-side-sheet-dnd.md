# Side Sheet Drag-and-Drop Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add drag-and-drop editing of pane/tab layout in the side sheet, with a mini-map on top and a linear list of panes (with tabs inline) below.

**Architecture:** Webview emits `SideSheetDragCommand` events via the existing `JsEmitEventPlugin` bridge. Bevy handler systems call pure tree-mutation helpers (`move_to_index`, `wrap_in_split`, `collapse_if_single_child`) operating on `&mut World` for trivial unit-testing. The `PaneTreeEvent` payload becomes a recursive `LayoutNode` tree so the mini-map can render split structure faithfully. Auto-save piggy-backs on existing `Changed<Children>` trigger — no persistence work.

**Tech Stack:** Rust / Bevy 0.18 / Dioxus 0.7 / moonshine_save / serde+RON for webview payloads.

**Design spec:** [docs/specs/2026-04-24-side-sheet-dnd-design.md](../specs/2026-04-24-side-sheet-dnd-design.md)

---

## File Structure

**New files:**
- `crates/vmux_desktop/src/layout/drag.rs` — command handler systems + auto-collapse

**Modified files:**
- `crates/vmux_side_sheet/src/event.rs` — new types + reshape `PaneTreeEvent`
- `crates/vmux_desktop/src/layout/swap.rs` — new helpers (`move_to_index`, `wrap_in_split`, `collapse_if_single_child`)
- `crates/vmux_desktop/src/browser.rs` — rewrite `push_pane_tree_emit`, register drag event plugin + observer
- `crates/vmux_desktop/src/layout.rs` — register `drag` submodule
- `crates/vmux_side_sheet/src/app.rs` — add mini-map component, drag handlers, drop-zone overlays

---

## Task 1: Add new event types (non-breaking)

Adds `LayoutNode`, `SplitDirection`, `Edge`, `SideSheetDragCommand`, and the new event-name constant alongside existing types. Nothing consumes them yet, so this can't break anything.

**Files:**
- Modify: `crates/vmux_side_sheet/src/event.rs`

- [ ] **Step 1: Append new types to event.rs**

Add after the existing `SideSheetCommandEvent` struct:

```rust
pub const SIDE_SHEET_DRAG_EVENT: &str = "side-sheet-drag";

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SplitDirection {
    Row,
    Column,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Edge {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum LayoutNode {
    Split {
        id: u64,
        direction: SplitDirection,
        children: Vec<LayoutNode>,
        flex_weights: Vec<f32>,
    },
    Pane {
        id: u64,
        is_active: bool,
        tabs: Vec<TabNode>,
    },
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind")]
pub enum SideSheetDragCommand {
    MoveTab {
        from_pane: u64,
        from_index: usize,
        to_pane: u64,
        to_index: usize,
    },
    SwapPane {
        pane: u64,
        target: u64,
    },
    SplitPane {
        dragged: u64,
        target: u64,
        edge: Edge,
    },
}
```

- [ ] **Step 2: Build to confirm types compile**

Run: `cargo build -p vmux_side_sheet`
Expected: success, no warnings about unused types (they're `pub`).

- [ ] **Step 3: Commit**

```bash
git add crates/vmux_side_sheet/src/event.rs
git commit -m "feat(side_sheet): add LayoutNode and SideSheetDragCommand types"
```

---

## Task 2: Add `move_to_index` helper + tests

Pure `&mut World` helper that removes a child from its current parent's `Children` and inserts it into another parent at a specific index. Uses Bevy's `ChildOf` relationship, so the framework keeps `Children` in sync.

**Files:**
- Modify: `crates/vmux_desktop/src/layout/swap.rs`

- [ ] **Step 1: Add failing test at bottom of swap.rs**

Append to `crates/vmux_desktop/src/layout/swap.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;

    fn spawn_parent_with_children(world: &mut World, n: usize) -> (Entity, Vec<Entity>) {
        let parent = world.spawn_empty().id();
        let kids: Vec<Entity> = (0..n)
            .map(|_| world.spawn(ChildOf(parent)).id())
            .collect();
        (parent, kids)
    }

    #[test]
    fn move_to_index_reorders_within_same_parent() {
        let mut world = World::new();
        let (parent, kids) = spawn_parent_with_children(&mut world, 3);
        // Move kids[2] to position 0.
        move_to_index(&mut world, kids[2], parent, 0);
        let children = world.get::<Children>(parent).unwrap();
        assert_eq!(&**children, &[kids[2], kids[0], kids[1]]);
    }

    #[test]
    fn move_to_index_reparents_across_parents() {
        let mut world = World::new();
        let (p1, a) = spawn_parent_with_children(&mut world, 2);
        let (p2, b) = spawn_parent_with_children(&mut world, 1);
        // Move a[0] from p1 into p2 at index 0.
        move_to_index(&mut world, a[0], p2, 0);

        let p1_kids = world.get::<Children>(p1).unwrap();
        assert_eq!(&**p1_kids, &[a[1]]);

        let p2_kids = world.get::<Children>(p2).unwrap();
        assert_eq!(&**p2_kids, &[a[0], b[0]]);
    }

    #[test]
    fn move_to_index_clamps_out_of_range() {
        let mut world = World::new();
        let (parent, kids) = spawn_parent_with_children(&mut world, 2);
        move_to_index(&mut world, kids[0], parent, 999);
        let children = world.get::<Children>(parent).unwrap();
        assert_eq!(&**children, &[kids[1], kids[0]]);
    }
}
```

- [ ] **Step 2: Run tests — expect failure**

Run: `cargo test -p vmux_desktop --lib layout::swap::tests`
Expected: compile error, `move_to_index` not found.

- [ ] **Step 3: Implement `move_to_index`**

Add to `crates/vmux_desktop/src/layout/swap.rs` above the tests module:

```rust
pub(crate) fn move_to_index(
    world: &mut bevy::prelude::World,
    child: bevy::prelude::Entity,
    new_parent: bevy::prelude::Entity,
    index: usize,
) {
    use bevy::prelude::*;

    // Detach from current parent (if any) so ChildOf's on_insert hook
    // doesn't append and leave us with two entries when we re-parent.
    world.entity_mut(child).remove::<ChildOf>();

    // Re-parent. The hook will append `child` to new_parent's Children.
    world.entity_mut(child).insert(ChildOf(new_parent));

    // Re-order new_parent's Children so child sits at `index`.
    let Some(mut children) = world.get_mut::<Children>(new_parent) else {
        return;
    };
    let Some(current) = children.iter().position(|e| e == child) else {
        return;
    };
    let target = index.min(children.len() - 1);
    if current != target {
        let list = children.as_mut();
        let entity = list.remove(current);
        list.insert(target, entity);
    }
}
```

Note: `Children` exposes `as_mut()` returning `&mut Vec<Entity>` in Bevy 0.18 (check via `cargo doc --open -p bevy` if this API moved — if it's `swap()` only, swap at the end and bubble; but the straightforward `remove`+`insert` keeps order intent obvious).

- [ ] **Step 4: Run tests — expect pass**

Run: `cargo test -p vmux_desktop --lib layout::swap::tests`
Expected: 3 tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/vmux_desktop/src/layout/swap.rs
git commit -m "feat(layout): add move_to_index helper with tests"
```

---

## Task 3: Add `collapse_if_single_child` helper + tests

After any pane move the parent split may end up with a single remaining child. This helper replaces the split with its sole child in the grandparent and despawns the split entity.

**Files:**
- Modify: `crates/vmux_desktop/src/layout/swap.rs`

- [ ] **Step 1: Add failing tests**

Append inside the existing `tests` module in `swap.rs`:

```rust
    use crate::layout::pane::{PaneSplit, PaneSplitDirection};

    fn spawn_split(world: &mut World, dir: PaneSplitDirection, parent: Entity) -> Entity {
        world.spawn((PaneSplit { direction: dir }, ChildOf(parent))).id()
    }

    #[test]
    fn collapse_replaces_single_child_split() {
        let mut world = World::new();
        let root = world.spawn_empty().id();
        let split = spawn_split(&mut world, PaneSplitDirection::Row, root);
        let only_child = world.spawn(ChildOf(split)).id();

        collapse_if_single_child(&mut world, split);

        // split entity is despawned, only_child is now a direct child of root.
        assert!(world.get_entity(split).is_err());
        let root_kids = world.get::<Children>(root).unwrap();
        assert_eq!(&**root_kids, &[only_child]);
    }

    #[test]
    fn collapse_is_noop_when_two_children() {
        let mut world = World::new();
        let root = world.spawn_empty().id();
        let split = spawn_split(&mut world, PaneSplitDirection::Row, root);
        let a = world.spawn(ChildOf(split)).id();
        let b = world.spawn(ChildOf(split)).id();

        collapse_if_single_child(&mut world, split);

        assert!(world.get_entity(split).is_ok());
        let split_kids = world.get::<Children>(split).unwrap();
        assert_eq!(&**split_kids, &[a, b]);
    }

    #[test]
    fn collapse_cascades_through_two_levels() {
        let mut world = World::new();
        let root = world.spawn_empty().id();
        let outer = spawn_split(&mut world, PaneSplitDirection::Row, root);
        let inner = spawn_split(&mut world, PaneSplitDirection::Column, outer);
        let leaf = world.spawn(ChildOf(inner)).id();

        collapse_if_single_child(&mut world, inner);
        collapse_if_single_child(&mut world, outer);

        assert!(world.get_entity(outer).is_err());
        assert!(world.get_entity(inner).is_err());
        let root_kids = world.get::<Children>(root).unwrap();
        assert_eq!(&**root_kids, &[leaf]);
    }
```

- [ ] **Step 2: Run tests — expect failure**

Run: `cargo test -p vmux_desktop --lib layout::swap::tests`
Expected: compile error, `collapse_if_single_child` not found.

- [ ] **Step 3: Implement `collapse_if_single_child`**

Add to `swap.rs` above the tests module:

```rust
pub(crate) fn collapse_if_single_child(
    world: &mut bevy::prelude::World,
    split: bevy::prelude::Entity,
) {
    use bevy::prelude::*;

    let Ok(children) = world
        .get::<Children>(split)
        .map(|c| c.iter().collect::<Vec<_>>())
        .ok_or(())
    else {
        return;
    };

    if children.len() != 1 {
        return;
    }
    let only_child = children[0];

    // Find split's parent (if any) and the index of split within it.
    let grandparent = world.get::<ChildOf>(split).map(|p| p.0);

    if let Some(gp) = grandparent {
        let gp_index = world
            .get::<Children>(gp)
            .and_then(|kids| kids.iter().position(|e| e == split));

        // Re-parent only_child onto grandparent; move_to_index handles order.
        if let Some(idx) = gp_index {
            super::swap::move_to_index(world, only_child, gp, idx);
        } else {
            world.entity_mut(only_child).remove::<ChildOf>();
            world.entity_mut(only_child).insert(ChildOf(gp));
        }
    } else {
        // Root-level split: detach the child, caller must reparent.
        world.entity_mut(only_child).remove::<ChildOf>();
    }

    world.entity_mut(split).despawn();
}
```

Note the `super::swap::move_to_index` reference is redundant because we're inside `swap`; use plain `move_to_index` instead. Leave the explicit path if it compiles; otherwise drop `super::swap::`.

- [ ] **Step 4: Run tests — expect pass**

Run: `cargo test -p vmux_desktop --lib layout::swap::tests`
Expected: all tests pass (3 new + 3 from Task 2).

- [ ] **Step 5: Commit**

```bash
git add crates/vmux_desktop/src/layout/swap.rs
git commit -m "feat(layout): add collapse_if_single_child helper"
```

---

## Task 4: Add `wrap_in_split` helper + tests

Creates a new `PaneSplit` entity, places it in the target's slot within the grandparent, and parents both target and dragged under the new split.

**Files:**
- Modify: `crates/vmux_desktop/src/layout/swap.rs`

- [ ] **Step 1: Add failing tests**

Append to the tests module:

```rust
    #[test]
    fn wrap_replaces_target_with_split_containing_both() {
        let mut world = World::new();
        let root = world.spawn_empty().id();
        let target = world.spawn(ChildOf(root)).id();
        let dragged = world.spawn_empty().id();

        let split = wrap_in_split(
            &mut world,
            target,
            PaneSplitDirection::Column,
            dragged,
            /* dragged_first */ true,
        );

        // Root now contains the split in target's old slot.
        let root_kids = world.get::<Children>(root).unwrap();
        assert_eq!(&**root_kids, &[split]);

        // Split has direction Column and children [dragged, target].
        let dir = world.get::<PaneSplit>(split).unwrap().direction;
        assert_eq!(dir, PaneSplitDirection::Column);

        let split_kids = world.get::<Children>(split).unwrap();
        assert_eq!(&**split_kids, &[dragged, target]);
    }

    #[test]
    fn wrap_preserves_target_position_in_grandparent() {
        let mut world = World::new();
        let root = world.spawn_empty().id();
        let a = world.spawn(ChildOf(root)).id();
        let target = world.spawn(ChildOf(root)).id();
        let c = world.spawn(ChildOf(root)).id();
        let dragged = world.spawn_empty().id();

        let split = wrap_in_split(
            &mut world,
            target,
            PaneSplitDirection::Row,
            dragged,
            /* dragged_first */ false,
        );

        let root_kids = world.get::<Children>(root).unwrap();
        assert_eq!(&**root_kids, &[a, split, c]);

        let split_kids = world.get::<Children>(split).unwrap();
        assert_eq!(&**split_kids, &[target, dragged]);
    }
```

- [ ] **Step 2: Run tests — expect failure**

Run: `cargo test -p vmux_desktop --lib layout::swap::tests`
Expected: `wrap_in_split` not found.

- [ ] **Step 3: Implement `wrap_in_split`**

Add to `swap.rs` above the tests module:

```rust
pub(crate) fn wrap_in_split(
    world: &mut bevy::prelude::World,
    target: bevy::prelude::Entity,
    direction: crate::layout::pane::PaneSplitDirection,
    dragged: bevy::prelude::Entity,
    dragged_first: bool,
) -> bevy::prelude::Entity {
    use bevy::prelude::*;
    use crate::layout::pane::PaneSplit;

    let grandparent = world.get::<ChildOf>(target).map(|p| p.0);
    let target_idx = grandparent
        .and_then(|gp| world.get::<Children>(gp).and_then(|c| c.iter().position(|e| e == target)));

    // Spawn the new split entity. Parent is resolved below.
    let split = world.spawn(PaneSplit { direction }).id();

    // Detach target and dragged from their current parents, park them under split.
    world.entity_mut(target).remove::<ChildOf>();
    world.entity_mut(dragged).remove::<ChildOf>();
    world.entity_mut(target).insert(ChildOf(split));
    world.entity_mut(dragged).insert(ChildOf(split));

    // Order children: [dragged, target] or [target, dragged].
    if dragged_first {
        move_to_index(world, dragged, split, 0);
    } else {
        move_to_index(world, target, split, 0);
    }

    // Place split into grandparent at target's original slot (if any).
    if let (Some(gp), Some(idx)) = (grandparent, target_idx) {
        world.entity_mut(split).insert(ChildOf(gp));
        move_to_index(world, split, gp, idx);
    }

    split
}
```

- [ ] **Step 4: Run tests — expect pass**

Run: `cargo test -p vmux_desktop --lib layout::swap::tests`
Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/vmux_desktop/src/layout/swap.rs
git commit -m "feat(layout): add wrap_in_split helper"
```

---

## Task 5: Create `drag.rs` with `handle_move_tab` system + tests

Introduces the new `drag` module and the first command handler. Uses `Commands::queue` with a closure so we get exclusive world access from a normal system.

**Files:**
- Create: `crates/vmux_desktop/src/layout/drag.rs`
- Modify: `crates/vmux_desktop/src/layout.rs` (register submodule)

- [ ] **Step 1: Declare the submodule**

Open `crates/vmux_desktop/src/layout.rs` and add alongside the other module declarations:

```rust
pub(crate) mod drag;
```

- [ ] **Step 2: Create `drag.rs` with failing test**

Create `crates/vmux_desktop/src/layout/drag.rs`:

```rust
use bevy::prelude::*;
use vmux_side_sheet::event::SideSheetDragCommand;

pub(crate) fn handle_drag_commands(
    mut commands: Commands,
    mut events: EventReader<SideSheetDragCommand>,
    pane_lookup: Query<Entity, With<crate::layout::pane::Pane>>,
) {
    for event in events.read() {
        match event.clone() {
            SideSheetDragCommand::MoveTab {
                from_pane, from_index, to_pane, to_index,
            } => {
                let _ = (from_pane, from_index, to_pane, to_index);
                // Implemented below.
            }
            SideSheetDragCommand::SwapPane { .. } => {}
            SideSheetDragCommand::SplitPane { .. } => {}
        }
        let _ = (&mut commands, &pane_lookup);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::tab::Tab;
    use crate::layout::pane::Pane;

    fn spawn_pane_with_tabs(world: &mut World, n: usize) -> (Entity, Vec<Entity>) {
        let pane = world.spawn(Pane).id();
        let tabs: Vec<Entity> = (0..n)
            .map(|_| world.spawn((Tab::default(), ChildOf(pane))).id())
            .collect();
        (pane, tabs)
    }

    #[test]
    fn move_tab_within_pane_reorders() {
        let mut world = World::new();
        let (pane, tabs) = spawn_pane_with_tabs(&mut world, 3);
        let pane_id = pane.to_bits();

        move_tab_impl(&mut world, pane_id, 2, pane_id, 0);

        let kids = world.get::<Children>(pane).unwrap();
        assert_eq!(&**kids, &[tabs[2], tabs[0], tabs[1]]);
    }

    #[test]
    fn move_tab_across_panes() {
        let mut world = World::new();
        let (p1, t1) = spawn_pane_with_tabs(&mut world, 2);
        let (p2, t2) = spawn_pane_with_tabs(&mut world, 1);

        move_tab_impl(&mut world, p1.to_bits(), 0, p2.to_bits(), 0);

        let p1_kids = world.get::<Children>(p1).unwrap();
        assert_eq!(&**p1_kids, &[t1[1]]);

        let p2_kids = world.get::<Children>(p2).unwrap();
        assert_eq!(&**p2_kids, &[t1[0], t2[0]]);
    }
}
```

- [ ] **Step 3: Run tests — expect failure**

Run: `cargo test -p vmux_desktop --lib layout::drag::tests`
Expected: compile error, `move_tab_impl` not found.

- [ ] **Step 4: Implement `move_tab_impl` and wire into system**

Replace the body of `handle_drag_commands` and add `move_tab_impl`:

```rust
pub(crate) fn move_tab_impl(
    world: &mut World,
    from_pane_id: u64,
    from_index: usize,
    to_pane_id: u64,
    to_index: usize,
) {
    let from_pane = Entity::from_bits(from_pane_id);
    let to_pane = Entity::from_bits(to_pane_id);

    let Some(tab) = world
        .get::<Children>(from_pane)
        .and_then(|c| c.get(from_index).copied())
    else {
        return;
    };

    crate::layout::swap::move_to_index(world, tab, to_pane, to_index);
}

pub(crate) fn handle_drag_commands(
    mut commands: Commands,
    mut events: EventReader<SideSheetDragCommand>,
) {
    for event in events.read() {
        let event = event.clone();
        commands.queue(move |world: &mut World| match event {
            SideSheetDragCommand::MoveTab {
                from_pane, from_index, to_pane, to_index,
            } => {
                move_tab_impl(world, from_pane, from_index, to_pane, to_index);
            }
            SideSheetDragCommand::SwapPane { .. } => {
                // Implemented in Task 6.
            }
            SideSheetDragCommand::SplitPane { .. } => {
                // Implemented in Task 7-8.
            }
        });
    }
}
```

- [ ] **Step 5: Run tests — expect pass**

Run: `cargo test -p vmux_desktop --lib layout::drag::tests`
Expected: 2 tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/vmux_desktop/src/layout.rs crates/vmux_desktop/src/layout/drag.rs
git commit -m "feat(drag): add handle_drag_commands system and MoveTab handler"
```

---

## Task 6: Add `SwapPane` handler + tests

Swaps two panes' positions, works across different parent splits.

**Files:**
- Modify: `crates/vmux_desktop/src/layout/drag.rs`

- [ ] **Step 1: Add failing tests**

Inside the `tests` module, append:

```rust
    use crate::layout::pane::{PaneSplit, PaneSplitDirection};

    fn spawn_split(world: &mut World, dir: PaneSplitDirection) -> Entity {
        world.spawn(PaneSplit { direction: dir }).id()
    }

    #[test]
    fn swap_pane_same_parent_swaps_positions() {
        let mut world = World::new();
        let split = spawn_split(&mut world, PaneSplitDirection::Row);
        let a = world.spawn((Pane, ChildOf(split))).id();
        let b = world.spawn((Pane, ChildOf(split))).id();

        swap_pane_impl(&mut world, a.to_bits(), b.to_bits());

        let kids = world.get::<Children>(split).unwrap();
        assert_eq!(&**kids, &[b, a]);
    }

    #[test]
    fn swap_pane_cross_parent_exchanges_slots() {
        let mut world = World::new();
        let root = spawn_split(&mut world, PaneSplitDirection::Row);
        let outer_a = world.spawn((Pane, ChildOf(root))).id();
        let col = world.spawn((PaneSplit { direction: PaneSplitDirection::Column }, ChildOf(root))).id();
        let inner_a = world.spawn((Pane, ChildOf(col))).id();
        let inner_b = world.spawn((Pane, ChildOf(col))).id();

        // Swap outer_a with inner_b.
        swap_pane_impl(&mut world, outer_a.to_bits(), inner_b.to_bits());

        let root_kids = world.get::<Children>(root).unwrap();
        assert_eq!(&**root_kids, &[inner_b, col]);

        let col_kids = world.get::<Children>(col).unwrap();
        assert_eq!(&**col_kids, &[inner_a, outer_a]);
    }
```

- [ ] **Step 2: Run tests — expect failure**

Run: `cargo test -p vmux_desktop --lib layout::drag::tests`
Expected: `swap_pane_impl` not found.

- [ ] **Step 3: Implement `swap_pane_impl` and wire into match**

Above `handle_drag_commands`, add:

```rust
pub(crate) fn swap_pane_impl(world: &mut World, a_id: u64, b_id: u64) {
    let a = Entity::from_bits(a_id);
    let b = Entity::from_bits(b_id);
    if a == b {
        return;
    }

    let a_parent = world.get::<ChildOf>(a).map(|p| p.0);
    let b_parent = world.get::<ChildOf>(b).map(|p| p.0);
    let a_idx = a_parent.and_then(|p| world.get::<Children>(p).and_then(|c| c.iter().position(|e| e == a)));
    let b_idx = b_parent.and_then(|p| world.get::<Children>(p).and_then(|c| c.iter().position(|e| e == b)));

    match (a_parent, b_parent, a_idx, b_idx) {
        (Some(ap), Some(bp), Some(ai), Some(bi)) if ap == bp => {
            // Same parent: swap positions in-place.
            let mut kids = world.get_mut::<Children>(ap).unwrap();
            kids.as_mut().swap(ai, bi);
        }
        (Some(ap), Some(bp), Some(ai), Some(bi)) => {
            // Different parents: detach both, re-parent into the other's slot.
            crate::layout::swap::move_to_index(world, a, bp, bi);
            crate::layout::swap::move_to_index(world, b, ap, ai);
        }
        _ => {}
    }
}
```

Replace the `SwapPane` arm in `handle_drag_commands`:

```rust
            SideSheetDragCommand::SwapPane { pane, target } => {
                swap_pane_impl(world, pane, target);
            }
```

- [ ] **Step 4: Run tests — expect pass**

Run: `cargo test -p vmux_desktop --lib layout::drag::tests`
Expected: 4 tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/vmux_desktop/src/layout/drag.rs
git commit -m "feat(drag): add SwapPane handler"
```

---

## Task 7: Add `SplitPane` handler — aligned-edge insert cases + tests

Handles the 4 cases where the drop edge matches the target's parent split direction: insert dragged as a sibling of target at the appropriate position in the existing split.

**Files:**
- Modify: `crates/vmux_desktop/src/layout/drag.rs`

- [ ] **Step 1: Add failing tests**

Append to the `tests` module:

```rust
    use vmux_side_sheet::event::Edge;

    #[test]
    fn split_row_left_inserts_before_target_in_same_row() {
        let mut world = World::new();
        let row = spawn_split(&mut world, PaneSplitDirection::Row);
        let a = world.spawn((Pane, ChildOf(row))).id();
        let target = world.spawn((Pane, ChildOf(row))).id();
        let dragged = world.spawn(Pane).id();

        split_pane_impl(&mut world, dragged.to_bits(), target.to_bits(), Edge::Left);

        let kids = world.get::<Children>(row).unwrap();
        assert_eq!(&**kids, &[a, dragged, target]);
    }

    #[test]
    fn split_row_right_inserts_after_target() {
        let mut world = World::new();
        let row = spawn_split(&mut world, PaneSplitDirection::Row);
        let target = world.spawn((Pane, ChildOf(row))).id();
        let b = world.spawn((Pane, ChildOf(row))).id();
        let dragged = world.spawn(Pane).id();

        split_pane_impl(&mut world, dragged.to_bits(), target.to_bits(), Edge::Right);

        let kids = world.get::<Children>(row).unwrap();
        assert_eq!(&**kids, &[target, dragged, b]);
    }

    #[test]
    fn split_column_top_inserts_before_target() {
        let mut world = World::new();
        let col = spawn_split(&mut world, PaneSplitDirection::Column);
        let a = world.spawn((Pane, ChildOf(col))).id();
        let target = world.spawn((Pane, ChildOf(col))).id();
        let dragged = world.spawn(Pane).id();

        split_pane_impl(&mut world, dragged.to_bits(), target.to_bits(), Edge::Top);

        let kids = world.get::<Children>(col).unwrap();
        assert_eq!(&**kids, &[a, dragged, target]);
    }

    #[test]
    fn split_column_bottom_inserts_after_target() {
        let mut world = World::new();
        let col = spawn_split(&mut world, PaneSplitDirection::Column);
        let target = world.spawn((Pane, ChildOf(col))).id();
        let b = world.spawn((Pane, ChildOf(col))).id();
        let dragged = world.spawn(Pane).id();

        split_pane_impl(&mut world, dragged.to_bits(), target.to_bits(), Edge::Bottom);

        let kids = world.get::<Children>(col).unwrap();
        assert_eq!(&**kids, &[target, dragged, b]);
    }
```

- [ ] **Step 2: Run tests — expect failure**

Run: `cargo test -p vmux_desktop --lib layout::drag::tests`
Expected: `split_pane_impl` not found.

- [ ] **Step 3: Implement `split_pane_impl` for aligned cases**

Add above `handle_drag_commands`:

```rust
pub(crate) fn split_pane_impl(world: &mut World, dragged_id: u64, target_id: u64, edge: Edge) {
    let dragged = Entity::from_bits(dragged_id);
    let target = Entity::from_bits(target_id);
    if dragged == target {
        return;
    }

    let target_parent = world.get::<ChildOf>(target).map(|p| p.0);
    let parent_dir = target_parent
        .and_then(|p| world.get::<PaneSplit>(p).map(|s| s.direction));

    use PaneSplitDirection::{Row, Column};
    use Edge::{Left, Right, Top, Bottom};

    let aligned_insert = match (parent_dir, edge) {
        (Some(Row), Left) | (Some(Row), Right)
        | (Some(Column), Top) | (Some(Column), Bottom) => true,
        _ => false,
    };

    if aligned_insert {
        let parent = target_parent.unwrap();
        let target_idx = world
            .get::<Children>(parent)
            .and_then(|kids| kids.iter().position(|e| e == target))
            .unwrap_or(0);
        let insert_idx = match edge {
            Left | Top => target_idx,
            Right | Bottom => target_idx + 1,
        };
        // Detach the old parent link of dragged first.
        world.entity_mut(dragged).remove::<ChildOf>();
        crate::layout::swap::move_to_index(world, dragged, parent, insert_idx);

        // Collapse dragged's old parent if it's now a single-child split.
        // (Implemented fully in Task 8; same-parent case is fine because the
        // old parent === new parent, and at >=2 children there's nothing to
        // collapse.)
        return;
    }

    // Perpendicular + root cases: implemented in Task 8.
    let _ = (parent_dir, edge);
}
```

Replace the `SplitPane` arm in `handle_drag_commands`:

```rust
            SideSheetDragCommand::SplitPane { dragged, target, edge } => {
                split_pane_impl(world, dragged, target, edge);
            }
```

Add the required imports near the top of `drag.rs`:

```rust
use crate::layout::pane::{Pane, PaneSplit, PaneSplitDirection};
use vmux_side_sheet::event::Edge;
```

Keep `use bevy::prelude::*;` and the existing `SideSheetDragCommand` import.

- [ ] **Step 4: Run tests — expect pass**

Run: `cargo test -p vmux_desktop --lib layout::drag::tests`
Expected: 8 tests pass (4 new + 4 prior).

- [ ] **Step 5: Commit**

```bash
git add crates/vmux_desktop/src/layout/drag.rs
git commit -m "feat(drag): add SplitPane aligned-edge insert cases"
```

---

## Task 8: Extend `SplitPane` — perpendicular wrap + no-parent cases + auto-collapse

Handles remaining 5 decision-table rows: perpendicular drops wrap target in a new split; drops onto a root pane wrap it directly. Also adds auto-collapse of the dragged pane's old split parent after a cross-parent move.

**Files:**
- Modify: `crates/vmux_desktop/src/layout/drag.rs`

- [ ] **Step 1: Add failing tests**

Append to the `tests` module:

```rust
    #[test]
    fn split_row_top_wraps_target_in_new_column_split_dragged_first() {
        let mut world = World::new();
        let row = spawn_split(&mut world, PaneSplitDirection::Row);
        let target = world.spawn((Pane, ChildOf(row))).id();
        let dragged = world.spawn(Pane).id();

        split_pane_impl(&mut world, dragged.to_bits(), target.to_bits(), Edge::Top);

        // Target was at index 0; now the new column split replaces it.
        let row_kids = world.get::<Children>(row).unwrap();
        let replacement = row_kids[0];
        let split = world.get::<PaneSplit>(replacement).unwrap();
        assert_eq!(split.direction, PaneSplitDirection::Column);

        let split_kids = world.get::<Children>(replacement).unwrap();
        assert_eq!(&**split_kids, &[dragged, target]);
    }

    #[test]
    fn split_row_bottom_wraps_target_dragged_second() {
        let mut world = World::new();
        let row = spawn_split(&mut world, PaneSplitDirection::Row);
        let target = world.spawn((Pane, ChildOf(row))).id();
        let dragged = world.spawn(Pane).id();

        split_pane_impl(&mut world, dragged.to_bits(), target.to_bits(), Edge::Bottom);

        let row_kids = world.get::<Children>(row).unwrap();
        let replacement = row_kids[0];
        let split_kids = world.get::<Children>(replacement).unwrap();
        assert_eq!(&**split_kids, &[target, dragged]);
    }

    #[test]
    fn split_column_left_wraps_in_row_split() {
        let mut world = World::new();
        let col = spawn_split(&mut world, PaneSplitDirection::Column);
        let target = world.spawn((Pane, ChildOf(col))).id();
        let dragged = world.spawn(Pane).id();

        split_pane_impl(&mut world, dragged.to_bits(), target.to_bits(), Edge::Left);

        let col_kids = world.get::<Children>(col).unwrap();
        let replacement = col_kids[0];
        assert_eq!(world.get::<PaneSplit>(replacement).unwrap().direction, PaneSplitDirection::Row);

        let split_kids = world.get::<Children>(replacement).unwrap();
        assert_eq!(&**split_kids, &[dragged, target]);
    }

    #[test]
    fn split_root_pane_wraps_in_matching_direction() {
        let mut world = World::new();
        // target has no parent split (simulates root pane of a Space).
        let space = world.spawn_empty().id();
        let target = world.spawn((Pane, ChildOf(space))).id();
        let dragged = world.spawn(Pane).id();

        split_pane_impl(&mut world, dragged.to_bits(), target.to_bits(), Edge::Right);

        // Root now holds a Row split.
        let space_kids = world.get::<Children>(space).unwrap();
        let replacement = space_kids[0];
        assert_eq!(world.get::<PaneSplit>(replacement).unwrap().direction, PaneSplitDirection::Row);

        let split_kids = world.get::<Children>(replacement).unwrap();
        assert_eq!(&**split_kids, &[target, dragged]);
    }

    #[test]
    fn cross_parent_move_collapses_old_single_child_split() {
        let mut world = World::new();
        let space = world.spawn_empty().id();
        let outer = world.spawn((PaneSplit { direction: PaneSplitDirection::Row }, ChildOf(space))).id();
        let p_outer = world.spawn((Pane, ChildOf(outer))).id();
        let old_split = world.spawn((PaneSplit { direction: PaneSplitDirection::Column }, ChildOf(outer))).id();
        let dragged = world.spawn((Pane, ChildOf(old_split))).id();
        let sibling = world.spawn((Pane, ChildOf(old_split))).id();
        // old_split has two children: dragged + sibling.

        split_pane_impl(&mut world, dragged.to_bits(), p_outer.to_bits(), Edge::Top);

        // old_split now has one child (sibling). It should be collapsed
        // so sibling becomes a direct child of outer.
        assert!(world.get_entity(old_split).is_err());
        let outer_kids = world.get::<Children>(outer).unwrap();
        assert!(outer_kids.iter().any(|e| e == sibling));
    }
```

- [ ] **Step 2: Run tests — expect failure**

Run: `cargo test -p vmux_desktop --lib layout::drag::tests`
Expected: 5 new tests fail (behaviour not yet implemented), 8 prior pass.

- [ ] **Step 3: Extend `split_pane_impl`**

Replace the early-return block after `aligned_insert` with:

```rust
    // Remember dragged's original parent so we can collapse a leftover single-child split.
    let old_parent = world.get::<ChildOf>(dragged).map(|p| p.0);

    if aligned_insert {
        let parent = target_parent.unwrap();
        let target_idx = world
            .get::<Children>(parent)
            .and_then(|kids| kids.iter().position(|e| e == target))
            .unwrap_or(0);
        let insert_idx = match edge {
            Left | Top => target_idx,
            Right | Bottom => target_idx + 1,
        };
        world.entity_mut(dragged).remove::<ChildOf>();
        crate::layout::swap::move_to_index(world, dragged, parent, insert_idx);
    } else {
        // Perpendicular or root: wrap target in a new split.
        let new_direction = match edge {
            Left | Right => PaneSplitDirection::Row,
            Top | Bottom => PaneSplitDirection::Column,
        };
        let dragged_first = matches!(edge, Left | Top);
        crate::layout::swap::wrap_in_split(world, target, new_direction, dragged, dragged_first);
    }

    // If dragged came from a different split that now has a single child, collapse it.
    if let Some(old) = old_parent {
        if world.get::<PaneSplit>(old).is_some() {
            crate::layout::swap::collapse_if_single_child(world, old);
        }
    }
```

Replace the whole function body below the `aligned_insert` computation — i.e. keep the `aligned_insert` variable and the guard but delete the previous early-return block.

- [ ] **Step 4: Run tests — expect pass**

Run: `cargo test -p vmux_desktop --lib layout::drag::tests`
Expected: 13 tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/vmux_desktop/src/layout/drag.rs
git commit -m "feat(drag): add SplitPane perpendicular-wrap and auto-collapse"
```

---

## Task 9: Reshape `PaneTreeEvent` to a recursive tree

Atomic change across producer + consumer so the app stays functional. After this task: webview still renders a flat list of leaf panes (no mini-map yet — that's Task 11), but the payload can carry split structure.

**Files:**
- Modify: `crates/vmux_side_sheet/src/event.rs`
- Modify: `crates/vmux_desktop/src/browser.rs:565-653`
- Modify: `crates/vmux_side_sheet/src/app.rs`

- [ ] **Step 1: Reshape `PaneTreeEvent` in event.rs**

In `crates/vmux_side_sheet/src/event.rs`, replace the existing `PaneTreeEvent` struct and `PaneNode` struct with:

```rust
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PaneTreeEvent {
    pub root: Option<LayoutNode>,
}

impl Default for PaneTreeEvent {
    fn default() -> Self {
        Self { root: None }
    }
}
```

Leave `TabNode` as-is. Remove the standalone `PaneNode` struct — the same fields now live inside `LayoutNode::Pane`.

- [ ] **Step 2: Rewrite `push_pane_tree_emit` in browser.rs**

Open `crates/vmux_desktop/src/browser.rs` at lines 565–653 and replace `push_pane_tree_emit` with a recursive traversal. Paste this function, adjusting queries/imports only if needed to match surrounding code:

```rust
fn push_pane_tree_emit(
    spaces: Query<(Entity, &LastActivatedAt), With<Space>>,
    all_children: Query<&Children>,
    pane_split_q: Query<&PaneSplit>,
    pane_q: Query<Entity, With<Pane>>,
    pane_size_q: Query<&PaneSize>,
    pane_ts: Query<(Entity, &LastActivatedAt), With<Pane>>,
    tab_ts: Query<(Entity, &LastActivatedAt), With<Tab>>,
    tab_q: Query<Entity, With<Tab>>,
    browser_meta: Query<(&PageMetadata, Has<Loading>), With<Browser>>,
    mut emits: MessageWriter<HostEmitEvent>,
    side_sheet_q: Query<Entity, With<SideSheet>>,
) {
    let Some(space) = active_among(spaces.iter()) else { return };
    let Ok(side_sheet_e) = side_sheet_q.single() else { return };

    let root = build_layout_node(
        space,
        &all_children,
        &pane_split_q,
        &pane_q,
        &pane_size_q,
        &pane_ts,
        &tab_ts,
        &tab_q,
        &browser_meta,
    );

    let event = PaneTreeEvent { root };
    let Ok(body) = ron::to_string(&event) else { return };
    emits.write(HostEmitEvent::new(side_sheet_e, PANE_TREE_EVENT, &body));
}

fn build_layout_node(
    entity: Entity,
    all_children: &Query<&Children>,
    pane_split_q: &Query<&PaneSplit>,
    pane_q: &Query<Entity, With<Pane>>,
    pane_size_q: &Query<&PaneSize>,
    pane_ts: &Query<(Entity, &LastActivatedAt), With<Pane>>,
    tab_ts: &Query<(Entity, &LastActivatedAt), With<Tab>>,
    tab_q: &Query<Entity, With<Tab>>,
    browser_meta: &Query<(&PageMetadata, Has<Loading>), With<Browser>>,
) -> Option<LayoutNode> {
    // PaneSplit → recurse into children.
    if let Ok(split) = pane_split_q.get(entity) {
        let kids = all_children.get(entity).ok()?;
        let mut children = Vec::with_capacity(kids.len());
        let mut weights = Vec::with_capacity(kids.len());
        for k in kids.iter() {
            if let Some(node) = build_layout_node(
                k, all_children, pane_split_q, pane_q, pane_size_q,
                pane_ts, tab_ts, tab_q, browser_meta,
            ) {
                let w = pane_size_q.get(k).map(|s| s.flex_grow).unwrap_or(1.0);
                children.push(node);
                weights.push(w);
            }
        }
        if children.is_empty() { return None; }
        return Some(LayoutNode::Split {
            id: entity.to_bits(),
            direction: match split.direction {
                crate::layout::pane::PaneSplitDirection::Row => SplitDirection::Row,
                crate::layout::pane::PaneSplitDirection::Column => SplitDirection::Column,
            },
            children,
            flex_weights: weights,
        });
    }

    // Pane leaf → collect tabs.
    if pane_q.get(entity).is_ok() {
        let active_pane = active_among(pane_ts.iter()).map(|e| e == entity).unwrap_or(false);
        let kids = all_children.get(entity).ok();
        let tabs: Vec<TabNode> = kids
            .map(|c| c.iter().filter(|e| tab_q.get(*e).is_ok()).collect::<Vec<_>>())
            .unwrap_or_default()
            .into_iter()
            .enumerate()
            .map(|(i, t)| {
                let (meta, loading) = browser_meta.get(t).ok().map_or(
                    (PageMetadata::default(), false),
                    |(m, l)| (m.clone(), l),
                );
                let active_tab = active_among(tab_ts.iter()).map(|e| e == t).unwrap_or(false);
                TabNode {
                    title: meta.title,
                    url: meta.url,
                    favicon_url: meta.favicon_url,
                    is_active: active_tab,
                    tab_index: i,
                    is_loading: loading,
                }
            })
            .collect();
        return Some(LayoutNode::Pane {
            id: entity.to_bits(),
            is_active: active_pane,
            tabs,
        });
    }

    // Space: descend into its root pane/split child.
    let kids = all_children.get(entity).ok()?;
    for k in kids.iter() {
        if pane_split_q.get(k).is_ok() || pane_q.get(k).is_ok() {
            return build_layout_node(
                k, all_children, pane_split_q, pane_q, pane_size_q,
                pane_ts, tab_ts, tab_q, browser_meta,
            );
        }
    }
    None
}
```

Add the necessary imports at the top of `browser.rs`: `SplitDirection` and `LayoutNode` from `vmux_side_sheet::event`; `PaneSize` from the layout module. Keep existing imports for `PaneSplit`, `Pane`, `Tab`, `TabNode`, `PaneTreeEvent`, `PANE_TREE_EVENT`.

- [ ] **Step 3: Update `app.rs` to consume the new tree**

Open `crates/vmux_side_sheet/src/app.rs`. Replace the `PaneTreeEvent { panes } = tree_state()` destructuring and the `for (i, pane) in panes.iter().enumerate()` loop with a traversal that flattens leaf panes:

Near the top of the file, update imports:

```rust
use vmux_side_sheet::event::{
    PANE_TREE_EVENT, LayoutNode, PaneTreeEvent, SideSheetCommandEvent, TabNode,
};
```

Remove the now-absent `PaneNode` import.

In the `App` component, replace the rendering block with:

```rust
        let panes = collect_leaf_panes(tree_state().root.as_ref());

        rsx! {
            div { class: "flex h-full flex-col overflow-y-auto px-2 py-3 text-foreground",
                if (listener.is_loading)() {
                    div { class: "flex items-center px-2 py-1",
                        span { class: "text-ui text-muted-foreground", "Connecting…" }
                    }
                } else if let Some(err) = (listener.error)() {
                    div { class: "flex items-center px-2 py-1",
                        span { class: "text-ui text-destructive", "{err}" }
                    }
                } else if panes.is_empty() {
                    div { class: "flex items-center px-2 py-1",
                        span { class: "text-ui text-muted-foreground", "No panes" }
                    }
                } else {
                    for (i, pane) in panes.iter().enumerate() {
                        PaneSection { key: "{pane.id}", id: pane.id, is_active: pane.is_active, tabs: pane.tabs.clone(), index: i }
                    }
                }
            }
        }
```

Add at the bottom of `app.rs` (outside components):

```rust
struct LeafPane {
    id: u64,
    is_active: bool,
    tabs: Vec<TabNode>,
}

fn collect_leaf_panes(root: Option<&LayoutNode>) -> Vec<LeafPane> {
    fn walk(node: &LayoutNode, out: &mut Vec<LeafPane>) {
        match node {
            LayoutNode::Split { children, .. } => {
                for c in children { walk(c, out); }
            }
            LayoutNode::Pane { id, is_active, tabs } => {
                out.push(LeafPane { id: *id, is_active: *is_active, tabs: tabs.clone() });
            }
        }
    }
    let mut out = Vec::new();
    if let Some(n) = root { walk(n, &mut out); }
    out
}
```

Update the `PaneSection` component signature to take separate fields (since `PaneNode` is gone):

```rust
#[component]
fn PaneSection(id: u64, is_active: bool, tabs: Vec<TabNode>, index: usize) -> Element {
    let label = format!("Pane {}", index + 1);
    let pane_id = id;
    let any_loading = tabs.iter().any(|t| t.is_loading);

    // ... rest of the existing render body, substituting `pane.is_active`
    //     with `is_active`, `pane.tabs` with `tabs`, and `pane.id` with `id`.
}
```

Adjust the `for tab in pane.tabs.iter()` inside `PaneSection` to `for tab in tabs.iter()`, etc. Everything else in the original render body transfers unchanged.

- [ ] **Step 4: Build + smoke-run**

Run: `cargo build -p vmux_desktop`
Expected: build succeeds.

Run: `make run-mac`
Expected: app launches, side sheet still shows a flat list of panes with tabs (same as before), because we're just carrying split structure through — no mini-map yet.

- [ ] **Step 5: Commit**

```bash
git add crates/vmux_side_sheet/src/event.rs crates/vmux_desktop/src/browser.rs crates/vmux_side_sheet/src/app.rs
git commit -m "feat(side_sheet): carry split tree in PaneTreeEvent"
```

---

## Task 10: Register `SideSheetDragCommand` observer

Wires the webview → Bevy bridge for the new event type. After this, the system compiles to receive drag commands, but no UI emits them yet.

**Files:**
- Modify: `crates/vmux_desktop/src/browser.rs`

- [ ] **Step 1: Register the plugin and add observer**

In `crates/vmux_desktop/src/browser.rs`, near the existing `.add_plugins(JsEmitEventPlugin::<SideSheetCommandEvent>::default())` line (~line 60):

```rust
            .add_plugins(JsEmitEventPlugin::<SideSheetDragCommand>::default())
```

And near the existing `.add_observer(on_side_sheet_command_emit)`:

```rust
            .add_observer(on_side_sheet_drag_emit)
```

Then add the observer function, next to `on_side_sheet_command_emit` (~line 779):

```rust
fn on_side_sheet_drag_emit(
    trigger: On<Receive<SideSheetDragCommand>>,
    mut writer: EventWriter<SideSheetDragCommand>,
) {
    writer.write(trigger.event().payload.clone());
}
```

And register the event type + handler system. Near where events are registered (look for `app.add_event::<SideSheetCommandEvent>()` or similar), add:

```rust
        .add_event::<SideSheetDragCommand>()
        .add_systems(Update, crate::layout::drag::handle_drag_commands)
```

Add the import near the top:

```rust
use vmux_side_sheet::event::{..., SideSheetDragCommand, SIDE_SHEET_DRAG_EVENT};
```

(The `SIDE_SHEET_DRAG_EVENT` constant isn't needed by `JsEmitEventPlugin` if it uses type-based event naming — check the existing `SideSheetCommandEvent` registration. If the pattern uses the constant explicitly, mirror it.)

- [ ] **Step 2: Build**

Run: `cargo build -p vmux_desktop`
Expected: success.

- [ ] **Step 3: Commit**

```bash
git add crates/vmux_desktop/src/browser.rs
git commit -m "feat(drag): register SideSheetDragCommand bridge and handler system"
```

---

## Task 11: Add mini-map component (no DnD yet)

Adds the recursive mini-map rendering to the top of the side sheet. Pure visual — no interactions yet.

**Files:**
- Modify: `crates/vmux_side_sheet/src/app.rs`

- [ ] **Step 1: Add `MiniMap` and `MiniMapNode` components**

At the bottom of `app.rs`, add:

```rust
#[component]
fn MiniMap(root: Option<LayoutNode>) -> Element {
    rsx! {
        div { class: "mb-3",
            div { class: "mb-1 px-1 text-[10px] uppercase tracking-wider text-muted-foreground/70",
                "Layout"
            }
            div { class: "rounded-lg bg-white/[0.03] p-1.5",
                div { class: "h-24 w-full",
                    if let Some(node) = root.as_ref() {
                        MiniMapNode { node: node.clone() }
                    } else {
                        div { class: "flex h-full w-full items-center justify-center text-xs text-muted-foreground", "empty" }
                    }
                }
            }
        }
    }
}

#[component]
fn MiniMapNode(node: LayoutNode) -> Element {
    match node {
        LayoutNode::Split { direction, children, flex_weights, .. } => {
            let flex_class = match direction {
                vmux_side_sheet::event::SplitDirection::Row => "flex h-full w-full flex-row gap-1",
                vmux_side_sheet::event::SplitDirection::Column => "flex h-full w-full flex-col gap-1",
            };
            rsx! {
                div { class: "{flex_class}",
                    for (i, child) in children.into_iter().enumerate() {
                        {
                            let weight = flex_weights.get(i).copied().unwrap_or(1.0);
                            rsx! {
                                div { key: "{i}", style: "flex-grow: {weight};",
                                    MiniMapNode { node: child }
                                }
                            }
                        }
                    }
                }
            }
        }
        LayoutNode::Pane { id, is_active, tabs } => {
            let count = tabs.len();
            rsx! {
                div {
                    "data-pane-id": "{id}",
                    class: if is_active {
                        "flex h-full w-full items-center justify-center rounded border-2 border-ring bg-ring/20 text-[10px] font-semibold text-foreground"
                    } else {
                        "flex h-full w-full items-center justify-center rounded border border-border bg-foreground/10 text-[10px] text-muted-foreground"
                    },
                    "P{id % 1000} · {count}"
                }
            }
        }
    }
}
```

- [ ] **Step 2: Render `MiniMap` in `App`**

In the `App` component, add the `MiniMap` call at the top of the outer `div` (before the listener/error/panes block):

```rust
            MiniMap { root: tree_state().root.clone() }
```

- [ ] **Step 3: Build + smoke-run**

Run: `make run-mac`
Expected: the side sheet now shows a small layout preview at the top matching the current pane geometry, plus the flat list below. Open two or three panes and confirm the preview updates.

- [ ] **Step 4: Commit**

```bash
git add crates/vmux_side_sheet/src/app.rs
git commit -m "feat(side_sheet): add layout mini-map"
```

---

## Task 12: Wire tab drag (HTML5 DnD) → `MoveTab`

Native HTML5 drag events on each `TabRow`, drop zones between rows and on pane-section bodies. Emits `SideSheetDragCommand::MoveTab`.

**Files:**
- Modify: `crates/vmux_side_sheet/src/app.rs`

- [ ] **Step 1: Add drag state signal + helpers**

Near the top of `App`, add a shared drag state (module-level via `GlobalSignal` or passed through props):

```rust
#[derive(Clone, Copy, Default, PartialEq, Debug)]
struct TabDragSource {
    pane_id: u64,
    tab_index: usize,
}

static TAB_DRAG: GlobalSignal<Option<TabDragSource>> = Signal::global(|| None);
```

Add `use dioxus::prelude::GlobalSignal;` (or confirm the existing `dioxus::prelude::*` covers it).

- [ ] **Step 2: Mark `TabRow` draggable and emit dragstart/dragend**

In `TabRow`, change the outer `div` attributes to include:

```rust
            draggable: "true",
            ondragstart: move |_| {
                *TAB_DRAG.write() = Some(TabDragSource { pane_id, tab_index });
            },
            ondragend: move |_| {
                *TAB_DRAG.write() = None;
            },
```

- [ ] **Step 3: Add drop handler on `PaneSection`**

In `PaneSection`, wrap the tab list `div` to handle `ondragover` and `ondrop`:

```rust
            div { class: "flex flex-col gap-px",
                ondragover: move |evt| {
                    evt.prevent_default();
                },
                ondrop: move |evt| {
                    evt.prevent_default();
                    if let Some(src) = *TAB_DRAG.read() {
                        let to_index = tabs.len(); // drop on empty area = append
                        let _ = try_cef_emit_serde(&SideSheetDragCommand::MoveTab {
                            from_pane: src.pane_id,
                            from_index: src.tab_index,
                            to_pane: pane_id,
                            to_index,
                        });
                    }
                    *TAB_DRAG.write() = None;
                },
                for (i, tab) in tabs.iter().enumerate() {
                    TabRow { key: "{i}", tab: tab.clone(), pane_id, tab_index: i }
                }
            }
```

Add the matching import at the top of `app.rs`:

```rust
use vmux_side_sheet::event::SideSheetDragCommand;
```

- [ ] **Step 4: Handle drop between rows (gap)**

Alongside each `TabRow`, insert a 4px-tall drop gap that accepts drops with a specific `to_index`. Inside the `for` loop in `PaneSection`:

```rust
                for (i, tab) in tabs.iter().enumerate() {
                    TabDropGap { to_pane: pane_id, to_index: i }
                    TabRow { key: "{i}", tab: tab.clone(), pane_id, tab_index: i }
                }
                TabDropGap { to_pane: pane_id, to_index: tabs.len() }
```

And define `TabDropGap`:

```rust
#[component]
fn TabDropGap(to_pane: u64, to_index: usize) -> Element {
    let mut is_over = use_signal(|| false);
    rsx! {
        div {
            class: if is_over() {
                "my-0.5 h-0.5 rounded bg-ring transition-colors"
            } else {
                "my-0.5 h-0.5 rounded bg-transparent"
            },
            ondragover: move |evt| {
                evt.prevent_default();
                is_over.set(true);
            },
            ondragleave: move |_| is_over.set(false),
            ondrop: move |evt| {
                evt.prevent_default();
                is_over.set(false);
                if let Some(src) = *TAB_DRAG.read() {
                    let _ = try_cef_emit_serde(&SideSheetDragCommand::MoveTab {
                        from_pane: src.pane_id,
                        from_index: src.tab_index,
                        to_pane,
                        to_index,
                    });
                }
                *TAB_DRAG.write() = None;
            },
        }
    }
}
```

- [ ] **Step 5: Build + smoke-run**

Run: `make run-mac`
Expected: you can drag a tab and drop it in the gap between two tabs or onto a different pane's section. Order updates; restart verifies persistence.

- [ ] **Step 6: Commit**

```bash
git add crates/vmux_side_sheet/src/app.rs
git commit -m "feat(side_sheet): add tab drag-and-drop"
```

---

## Task 13: Wire pane drag (pointer events) → `SwapPane` / `SplitPane`

Pointer-driven drag on mini-map pane rectangles with drop-zone overlays (4 edges + center).

**Files:**
- Modify: `crates/vmux_side_sheet/src/app.rs`

- [ ] **Step 1: Add pane drag signal and zone detection helper**

At the bottom of `app.rs`, next to `TAB_DRAG`:

```rust
#[derive(Clone, Copy, Default, PartialEq, Debug)]
struct PaneDragSource {
    id: u64,
}

static PANE_DRAG: GlobalSignal<Option<PaneDragSource>> = Signal::global(|| None);

fn classify_drop_zone(px: f64, py: f64, w: f64, h: f64) -> DropZone {
    let rx = px / w;
    let ry = py / h;
    let left = rx;
    let right = 1.0 - rx;
    let top = ry;
    let bottom = 1.0 - ry;
    let min = [left, right, top, bottom].into_iter().fold(f64::INFINITY, f64::min);

    if (0.3..=0.7).contains(&rx) && (0.3..=0.7).contains(&ry) {
        DropZone::Center
    } else if min == left {
        DropZone::Edge(vmux_side_sheet::event::Edge::Left)
    } else if min == right {
        DropZone::Edge(vmux_side_sheet::event::Edge::Right)
    } else if min == top {
        DropZone::Edge(vmux_side_sheet::event::Edge::Top)
    } else {
        DropZone::Edge(vmux_side_sheet::event::Edge::Bottom)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum DropZone {
    Edge(vmux_side_sheet::event::Edge),
    Center,
}
```

- [ ] **Step 2: Turn `MiniMapNode` Pane leaves into drag sources + drop targets**

Replace the `LayoutNode::Pane` arm of `MiniMapNode` with:

```rust
        LayoutNode::Pane { id, is_active, tabs } => {
            let count = tabs.len();
            let mut hover_zone = use_signal(|| None::<DropZone>);

            rsx! {
                div {
                    "data-pane-id": "{id}",
                    class: if is_active {
                        "relative flex h-full w-full items-center justify-center rounded border-2 border-ring bg-ring/20 text-[10px] font-semibold text-foreground select-none"
                    } else {
                        "relative flex h-full w-full items-center justify-center rounded border border-border bg-foreground/10 text-[10px] text-muted-foreground select-none"
                    },
                    onpointerdown: move |evt| {
                        evt.stop_propagation();
                        *PANE_DRAG.write() = Some(PaneDragSource { id });
                    },
                    onpointermove: move |evt| {
                        if PANE_DRAG.read().is_none() { return; }
                        let rect = evt.data().element_coordinates();
                        let page = evt.data().page_coordinates();
                        let px = rect.x;
                        let py = rect.y;
                        // We don't have the element size directly; approximate via
                        // page delta. Dioxus 0.7 doesn't expose offsetWidth here;
                        // fall back to using the bounding rect via a small helper.
                        // For MVP, derive width/height from a data-attribute set
                        // during render (requires passing width/height via ref-like
                        // mechanism). As a simpler placeholder, use `element_coordinates`
                        // relative to element and clamp.
                        let _ = (px, py, page);
                        hover_zone.set(Some(DropZone::Center));
                    },
                    onpointerup: move |evt| {
                        if let Some(src) = *PANE_DRAG.read() {
                            if src.id != id {
                                if let Some(zone) = hover_zone() {
                                    match zone {
                                        DropZone::Center => {
                                            let _ = try_cef_emit_serde(&SideSheetDragCommand::SwapPane {
                                                pane: src.id,
                                                target: id,
                                            });
                                        }
                                        DropZone::Edge(edge) => {
                                            let _ = try_cef_emit_serde(&SideSheetDragCommand::SplitPane {
                                                dragged: src.id,
                                                target: id,
                                                edge,
                                            });
                                        }
                                    }
                                }
                            }
                        }
                        *PANE_DRAG.write() = None;
                        hover_zone.set(None);
                        let _ = evt;
                    },
                    "P{id % 1000} · {count}",
                    // Drop zone overlay (visible only while dragging)
                    if PANE_DRAG.read().is_some() {
                        div { class: "pointer-events-none absolute inset-0 grid grid-cols-5 grid-rows-5",
                            div { class: "col-span-1 row-span-5 bg-ring/30" }
                            div { class: "col-span-3 row-span-1 bg-ring/30" }
                            div { class: "col-span-1 row-span-5 bg-ring/30 col-start-5" }
                            div { class: "col-span-3 row-span-1 bg-ring/30 row-start-5 col-start-2" }
                        }
                    }
                }
            }
        }
```

Note: `onpointermove` zone classification needs element dimensions. In Dioxus 0.7 you can read them via an `on_mounted` ref that stores the element. If `element_coordinates` returns the client-relative position only, add a `use_signal` for `(width, height)` set on `onresize` / `onmounted`. For the MVP, if you cannot get dimensions reliably, fall back to reading `evt.data().client_coordinates()` plus a `use_memo` that reads `evt.target()` dims via `web_sys` (the Dioxus CEF backend should expose `web_sys`).

If the pointer-move logic proves fiddly, simplify to: always treat the drop as `Center` (swap) — the feature is still useful. The full edge-zone detection can be a follow-up task.

- [ ] **Step 3: Build + smoke-run**

Run: `make run-mac`
Expected: you can grab a pane rectangle in the mini-map and drop it on another; swap or split behaviour fires based on the zone.

- [ ] **Step 4: Commit**

```bash
git add crates/vmux_side_sheet/src/app.rs
git commit -m "feat(side_sheet): add pane drag-and-drop in mini-map"
```

---

## Task 14: Manual verification pass

Run through the five scenarios from the spec to confirm end-to-end behaviour and persistence.

**Files:** none (verification only).

- [ ] **Step 1: Scenario 1 — within-pane tab reorder**

Open a pane with tabs A/B/C. Drag C between A and B. Order becomes A/C/B. Quit the app completely. Reopen. Order still A/C/B.

- [ ] **Step 2: Scenario 2 — cross-pane tab move**

Two panes side-by-side. Drag a tab from the right pane into the left pane's section in the linear list. Tab moves. Restart, same.

- [ ] **Step 3: Scenario 3 — cross-split pane swap**

Three panes (Row with P1 left, Column P2/P3 right). In the mini-map, drag P1 onto P3's center. P1 and P3 swap positions. Restart, same.

- [ ] **Step 4: Scenario 4 — edge split wrap**

Drag P1 onto P2's top edge. A new Column split replaces P2, containing `[P1, P2]`. Restart, same.

- [ ] **Step 5: Scenario 5 — auto-collapse**

Starting from a two-pane Row, drag P1 onto another pane elsewhere. The Row collapses and P2 becomes a direct child of the Space. Restart, same.

- [ ] **Step 6: Commit verification notes (optional)**

If anything failed, file a follow-up issue; otherwise no commit.

---

## Self-Review Notes

**Spec coverage:**
- Mini-map + linear list (spec § Representation) → Tasks 11, 9
- Recursive `LayoutNode` (spec § Data Changes) → Tasks 1, 9
- `SideSheetDragCommand` (spec § New command event) → Tasks 1, 10
- Tab DnD (spec § Tab DnD) → Task 12
- Pane DnD + drop zones (spec § Pane DnD) → Task 13
- Tree mutations (spec § Tree Mutations) → Tasks 2–8
- Auto-collapse (spec § Auto-collapse) → Tasks 3, 8
- Persistence via existing `Changed<Children>` → no work needed, confirmed in Task 9 smoke test
- Out-of-scope rename hook: `u64` IDs preserved in `LayoutNode`, `PaneNode`/`TabNode` untouched → future `label: Option<String>` fits without schema churn

**Known MVP compromise:** Task 13's zone classifier needs element dimensions from Dioxus pointer events. Implementer should check Dioxus 0.7 docs; if the API is awkward, ship center-only (swap only) first and follow up with edge-zone detection as a separate task. The Rust-side `SplitPane` handler is fully implemented and tested regardless.

**Persistence corner:** New splits spawned by `wrap_in_split` inherit `PaneSplit` with direction but no `PaneSize`. Confirm during Task 14 that the visible layout doesn't collapse to 0-width — if it does, the fix is to give the new split a `PaneSize { flex_grow: 1.0 }` inside `wrap_in_split`. Keep as a watch item rather than a preemptive change.
