# Pane Resize Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enable pane resize via cursor drag on the gap and tmux-style keyboard commands (Ctrl+B, Alt+Arrow).

**Architecture:** Store per-pane size as a `PaneSize` component with `flex_grow` ratio. Drag state lives as a `PaneDrag` component on the `PaneSplit` entity being dragged. Keyboard resize adjusts `flex_grow` on the active pane and its sibling. All resize writes to both `Node.flex_grow` (view) and `PaneSize.flex_grow` (model/persistence).

**Tech Stack:** Bevy 0.18 (ECS, UI flexbox), moonshine-save (persistence), bevy_cef (cursor icons)

**Spec:** `docs/superpowers/specs/2025-07-20-pane-resize-design.md`

---

### Task 1: Add PaneSize component and wire into existing bundles

**Files:**
- Modify: `crates/vmux_desktop/src/layout/pane.rs` (components, `leaf_pane_bundle`, split handler)
- Modify: `crates/vmux_desktop/src/lib.rs` (register_type)
- Modify: `crates/vmux_desktop/src/persistence.rs` (save allowlist, rebuild)

- [ ] **Step 1: Add PaneSize component to pane.rs**

After `PaneSplitDirection` enum (line ~67), add:

```rust
#[derive(Component, Reflect, Clone, Copy, Debug)]
#[reflect(Component)]
#[require(Save)]
pub(crate) struct PaneSize {
    pub flex_grow: f32,
}

impl Default for PaneSize {
    fn default() -> Self {
        Self { flex_grow: 1.0 }
    }
}

const MIN_PANE_PX: f32 = 60.0;
const RESIZE_STEP: f32 = 0.05;
```

- [ ] **Step 2: Add PaneSize to leaf_pane_bundle**

Change `leaf_pane_bundle` to include `PaneSize::default()`:

```rust
pub(crate) fn leaf_pane_bundle() -> impl Bundle {
    (
        Pane::default(),
        PaneSize::default(),
        Transform::default(),
        GlobalTransform::default(),
        Node {
            flex_grow: 1.0,
            flex_basis: Val::Px(0.0),
            align_items: AlignItems::Stretch,
            justify_content: JustifyContent::Stretch,
            ..default()
        },
    )
}
```

- [ ] **Step 3: Register PaneSize type in lib.rs**

After `.register_type::<layout::pane::PaneSplitDirection>()` add:

```rust
.register_type::<layout::pane::PaneSize>()
```

- [ ] **Step 4: Add PaneSize to save allowlist in persistence.rs**

In `do_save`, after `.allow::<PaneSplit>()` add:

```rust
.allow::<layout::pane::PaneSize>()
```

Also add the import — in the `use crate::layout::pane::{...}` block, add `PaneSize`.

- [ ] **Step 5: Use PaneSize in rebuild_session_views**

Add `PaneSize` to the query and imports. Change the leaf pane rebuild section:

Add a new query parameter:
```rust
pane_sizes: Query<&PaneSize>,
```

In the `"-- Leaf Pane: add stretch layout --"` section, read the persisted flex_grow:

```rust
for entity in &panes_need_view {
    let grow = pane_sizes.get(entity).map(|s| s.flex_grow).unwrap_or(1.0);
    let mut ecmds = commands.entity(entity);
    ecmds.insert((
        Transform::default(),
        GlobalTransform::default(),
        Node {
            flex_grow: grow,
            flex_basis: Val::Px(0.0),
            align_items: AlignItems::Stretch,
            justify_content: JustifyContent::Stretch,
            ..default()
        },
    ));
    if let Ok(co) = child_of_q.get(entity) {
        ecmds.insert(ChildOf(co.get()));
    }
}
```

- [ ] **Step 6: Build and verify**

Run: `cargo build 2>&1 | tail -5`
Expected: compiles with no errors.

- [ ] **Step 7: Commit**

```bash
git add crates/vmux_desktop/src/layout/pane.rs crates/vmux_desktop/src/lib.rs crates/vmux_desktop/src/persistence.rs
git commit -m "feat: add PaneSize component for pane resize ratios"
```

---

### Task 2: Implement keyboard resize (ResizeLeft/Right/Up/Down + EqualizeSize)

**Files:**
- Modify: `crates/vmux_desktop/src/layout/pane.rs` (fill stub handlers)

- [ ] **Step 1: Add a helper function for resize logic**

Add after `spawn_leaf_pane` function:

```rust
/// Resize a pane relative to a sibling in the same split.
/// `grow_delta` is positive to grow the pane, negative to shrink.
/// Returns true if resize was applied.
fn apply_pane_resize(
    pane: Entity,
    sibling: Entity,
    parent_split: Entity,
    grow_delta: f32,
    node_q: &mut Query<&mut Node>,
    size_q: &mut Query<&mut PaneSize>,
    parent_node_q: &Query<&ComputedNode>,
) -> bool {
    let parent_size = parent_node_q.get(parent_split).map(|cn| cn.size).unwrap_or(Vec2::ZERO);
    let Ok(mut pane_node) = node_q.get_mut(pane) else { return false };
    let Ok(mut sib_node) = node_q.get_mut(sibling) else { return false };

    let mut pane_grow = pane_node.flex_grow;
    let mut sib_grow = sib_node.flex_grow;
    let total = pane_grow + sib_grow;

    pane_grow += grow_delta;
    sib_grow -= grow_delta;

    // Compute minimum flex_grow from MIN_PANE_PX
    // For Row splits, use parent width; for Column, use parent height.
    // We use the larger axis as a conservative estimate here;
    // the caller knows the axis but we clamp against both.
    let parent_len = parent_size.x.max(parent_size.y).max(1.0);
    let min_grow = MIN_PANE_PX / parent_len * total;

    pane_grow = pane_grow.max(min_grow);
    sib_grow = sib_grow.max(min_grow);

    // Renormalize so total is preserved
    let new_total = pane_grow + sib_grow;
    if new_total > 0.0 {
        pane_grow = pane_grow / new_total * total;
        sib_grow = sib_grow / new_total * total;
    }

    pane_node.flex_grow = pane_grow;
    sib_node.flex_grow = sib_grow;

    if let Ok(mut ps) = size_q.get_mut(pane) { ps.flex_grow = pane_grow; }
    if let Ok(mut ps) = size_q.get_mut(sibling) { ps.flex_grow = sib_grow; }

    true
}
```

- [ ] **Step 2: Add required query parameters to handle_pane_commands**

Add these parameters to the `handle_pane_commands` function signature:

```rust
mut node_q: Query<&mut Node>,
mut size_q: Query<&mut PaneSize>,
parent_node_q: Query<&ComputedNode>,
```

- [ ] **Step 3: Fill in EqualizeSize handler**

Replace `PaneCommand::EqualizeSize => {}` with:

```rust
PaneCommand::EqualizeSize => {
    let Ok(co) = child_of_q.get(active) else { continue };
    let parent = co.get();
    if !split_q.contains(parent) { continue; }
    let Ok(children) = all_children.get(parent) else { continue };
    for &child in children.iter() {
        if let Ok(mut node) = node_q.get_mut(child) {
            node.flex_grow = 1.0;
        }
        if let Ok(mut ps) = size_q.get_mut(child) {
            ps.flex_grow = 1.0;
        }
    }
}
```

- [ ] **Step 4: Fill in Resize handlers**

Add a new query parameter to `handle_pane_commands` (the existing `split_q: Query<(), With<PaneSplit>>` doesn't give direction):

```rust
split_dir_q: Query<&PaneSplit>,
```

Replace the four resize stubs:

```rust
PaneCommand::ResizeLeft | PaneCommand::ResizeRight
| PaneCommand::ResizeUp | PaneCommand::ResizeDown => {
    let target_axis = match pane_cmd {
        PaneCommand::ResizeLeft | PaneCommand::ResizeRight => PaneSplitDirection::Row,
        _ => PaneSplitDirection::Column,
    };
    let grows = matches!(
        pane_cmd,
        PaneCommand::ResizeRight | PaneCommand::ResizeDown
    );

    // Walk up to find a split whose direction matches target_axis
    let mut child_in_split = active;
    let mut found_parent: Option<Entity> = None;
    for _ in 0..10 {
        let Ok(co) = child_of_q.get(child_in_split) else { break };
        let parent = co.get();
        if let Ok(ps) = split_dir_q.get(parent) {
            if ps.direction == target_axis {
                found_parent = Some(parent);
                break;
            }
        }
        child_in_split = parent;
    }
    let Some(parent) = found_parent else { continue };
    let Ok(siblings) = all_children.get(parent) else { continue };
    let sibs: Vec<Entity> = siblings.iter().collect();
    let Some(idx) = sibs.iter().position(|&e| e == child_in_split) else { continue };

    let total_grow: f32 = sibs.iter()
        .filter_map(|&e| node_q.get(e).ok())
        .map(|n| n.flex_grow)
        .sum();
    let step = RESIZE_STEP * total_grow;

    let (pane_entity, sibling_entity, delta) = if grows {
        if idx + 1 >= sibs.len() { continue; }
        (child_in_split, sibs[idx + 1], step)
    } else {
        if idx == 0 { continue; }
        (child_in_split, sibs[idx - 1], step)
    };

    apply_pane_resize(
        pane_entity, sibling_entity, parent,
        delta, &mut node_q, &mut size_q, &parent_node_q,
    );
}
```

- [ ] **Step 5: Build and verify**

Run: `cargo build 2>&1 | tail -5`
Expected: compiles with no errors.

- [ ] **Step 6: Manual test**

Launch the app, split a pane (Ctrl+B, V), then press Ctrl+B, Alt+Right and Ctrl+B, Alt+Left. Verify panes resize. Press Ctrl+B, = to equalize.

- [ ] **Step 7: Commit**

```bash
git add crates/vmux_desktop/src/layout/pane.rs
git commit -m "feat: implement keyboard pane resize and equalize"
```

---

### Task 3: Implement drag resize on gap

**Files:**
- Modify: `crates/vmux_desktop/src/layout/pane.rs` (PaneDrag component, drag system, focus suppression)

- [ ] **Step 1: Add PaneDrag component**

After `PaneSize` component definition, add:

```rust
#[derive(Component)]
pub(crate) struct PaneDrag {
    prev_child: Entity,
    next_child: Entity,
    start_pos: f32,
    start_prev_grow: f32,
    start_next_grow: f32,
}
```

- [ ] **Step 2: Add the drag resize system**

Add the system function:

```rust
fn pane_gap_drag_resize(
    windows: Query<&Window, With<PrimaryWindow>>,
    window_entities: Query<Entity, With<PrimaryWindow>>,
    splits: Query<(Entity, &PaneSplit, &Children), Without<PaneDrag>>,
    active_drags: Query<(Entity, &PaneDrag, &PaneSplit)>,
    child_nodes: Query<(&ComputedNode, &UiGlobalTransform)>,
    parent_nodes: Query<&ComputedNode>,
    mut node_q: Query<&mut Node>,
    mut size_q: Query<&mut PaneSize>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut commands: Commands,
) {
    let Ok(window) = windows.single() else { return };
    let Some(cursor_pos) = window.physical_cursor_position() else { return };
    let cursor = Vec2::new(cursor_pos.x as f32, cursor_pos.y as f32);

    // --- Handle active drag ---
    if let Ok((split_entity, drag, split)) = active_drags.single() {
        if mouse.pressed(MouseButton::Left) {
            let pos_along = match split.direction {
                PaneSplitDirection::Row => cursor.x,
                PaneSplitDirection::Column => cursor.y,
            };
            let parent_size = parent_nodes.get(split_entity)
                .map(|cn| cn.size).unwrap_or(Vec2::ONE);
            let parent_len = match split.direction {
                PaneSplitDirection::Row => parent_size.x,
                PaneSplitDirection::Column => parent_size.y,
            }.max(1.0);

            let total_grow = drag.start_prev_grow + drag.start_next_grow;
            let delta_grow = (pos_along - drag.start_pos) / parent_len * total_grow;

            let mut prev_grow = drag.start_prev_grow + delta_grow;
            let mut next_grow = drag.start_next_grow - delta_grow;

            let min_grow = MIN_PANE_PX / parent_len * total_grow;
            prev_grow = prev_grow.max(min_grow);
            next_grow = next_grow.max(min_grow);

            let new_total = prev_grow + next_grow;
            if new_total > 0.0 {
                prev_grow = prev_grow / new_total * total_grow;
                next_grow = next_grow / new_total * total_grow;
            }

            if let Ok(mut n) = node_q.get_mut(drag.prev_child) { n.flex_grow = prev_grow; }
            if let Ok(mut n) = node_q.get_mut(drag.next_child) { n.flex_grow = next_grow; }
            if let Ok(mut s) = size_q.get_mut(drag.prev_child) { s.flex_grow = prev_grow; }
            if let Ok(mut s) = size_q.get_mut(drag.next_child) { s.flex_grow = next_grow; }
        } else {
            commands.entity(split_entity).remove::<PaneDrag>();
        }

        // Keep resize cursor during drag
        let icon = match split.direction {
            PaneSplitDirection::Row => SystemCursorIcon::ColResize,
            PaneSplitDirection::Column => SystemCursorIcon::RowResize,
        };
        if let Ok(we) = window_entities.single() {
            commands.entity(we).insert(CursorIcon::System(icon));
        }
        return;
    }

    // --- Hover detection + drag initiation ---
    let mut hovered_dir: Option<PaneSplitDirection> = None;
    'outer: for (split_entity, split, children) in &splits {
        let sibs: Vec<Entity> = children.iter().collect();
        for i in 0..sibs.len().saturating_sub(1) {
            let Ok((node_a, gt_a)) = child_nodes.get(sibs[i]) else { continue };
            let Ok((node_b, gt_b)) = child_nodes.get(sibs[i + 1]) else { continue };

            let center_a = gt_a.transform_point2(Vec2::ZERO);
            let center_b = gt_b.transform_point2(Vec2::ZERO);
            let half_a = node_a.size * 0.5;
            let half_b = node_b.size * 0.5;

            let (gap_min, gap_max, cross_min, cross_max) = match split.direction {
                PaneSplitDirection::Row => (
                    center_a.x + half_a.x,
                    center_b.x - half_b.x,
                    (center_a.y - half_a.y).min(center_b.y - half_b.y),
                    (center_a.y + half_a.y).max(center_b.y + half_b.y),
                ),
                PaneSplitDirection::Column => (
                    center_a.y + half_a.y,
                    center_b.y - half_b.y,
                    (center_a.x - half_a.x).min(center_b.x - half_b.x),
                    (center_a.x + half_a.x).max(center_b.x + half_b.x),
                ),
            };

            let (pos_along, pos_cross) = match split.direction {
                PaneSplitDirection::Row => (cursor.x, cursor.y),
                PaneSplitDirection::Column => (cursor.y, cursor.x),
            };

            if pos_along >= gap_min && pos_along <= gap_max
                && pos_cross >= cross_min && pos_cross <= cross_max
            {
                hovered_dir = Some(split.direction);

                if mouse.just_pressed(MouseButton::Left) {
                    let prev_grow = node_q.get(sibs[i]).map(|n| n.flex_grow).unwrap_or(1.0);
                    let next_grow = node_q.get(sibs[i + 1]).map(|n| n.flex_grow).unwrap_or(1.0);
                    commands.entity(split_entity).insert(PaneDrag {
                        prev_child: sibs[i],
                        next_child: sibs[i + 1],
                        start_pos: pos_along,
                        start_prev_grow: prev_grow,
                        start_next_grow: next_grow,
                    });
                }
                break 'outer;
            }
        }
    }

    // Set cursor icon on hover
    if let Some(dir) = hovered_dir {
        let icon = match dir {
            PaneSplitDirection::Row => SystemCursorIcon::ColResize,
            PaneSplitDirection::Column => SystemCursorIcon::RowResize,
        };
        if let Ok(we) = window_entities.single() {
            commands.entity(we).insert(CursorIcon::System(icon));
        }
    }
}
```

- [ ] **Step 3: Register the system and add focus suppression**

In `PanePlugin::build`, add the system:

```rust
.add_systems(Update, pane_gap_drag_resize)
```

In `poll_cursor_pane_focus`, add a query parameter and guard:

```rust
active_drags: Query<(), With<PaneDrag>>,
```

At the top of the function, after the Ctrl key guard:

```rust
if !active_drags.is_empty() {
    return;
}
```

- [ ] **Step 4: Build and verify**

Run: `cargo build 2>&1 | tail -5`
Expected: compiles with no errors.

- [ ] **Step 5: Manual test**

Launch the app, split a pane. Hover cursor over the gap — cursor should change to resize cursor. Click and drag to resize. Verify minimum size enforcement.

- [ ] **Step 6: Commit**

```bash
git add crates/vmux_desktop/src/layout/pane.rs
git commit -m "feat: implement drag-to-resize on pane gap"
```

---

### Task 4: Mark dirty on resize for auto-save

**Files:**
- Modify: `crates/vmux_desktop/src/persistence.rs` (mark_dirty_on_change)

- [ ] **Step 1: Add PaneSize change detection to mark_dirty_on_change**

In the `mark_dirty_on_change` system, add a query parameter:

```rust
changed_size: Query<(), Changed<PaneSize>>,
```

Add to the `use crate::layout::pane::{...}` import: `PaneSize`.

Add to the `if` condition:

```rust
|| !changed_size.is_empty()
```

- [ ] **Step 2: Build and verify**

Run: `cargo build 2>&1 | tail -5`
Expected: compiles with no errors.

- [ ] **Step 3: Commit**

```bash
git add crates/vmux_desktop/src/persistence.rs
git commit -m "feat: trigger auto-save on pane resize"
```
