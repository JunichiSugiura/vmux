use bevy::prelude::*;

/// Swap two same-type siblings within a parent's Children.
/// `kind_positions` are indices into Children of entities matching the filter.
/// `a` and `b` are indices into that filtered list.
pub fn swap_siblings(
    commands: &mut Commands,
    parent: Entity,
    children: &Children,
    kind_positions: &[usize],
    a: usize,
    b: usize,
) {
    if a == b {
        return;
    }
    let Some(&pos_a) = kind_positions.get(a) else {
        return;
    };
    let Some(&pos_b) = kind_positions.get(b) else {
        return;
    };

    let mut ordered: Vec<Entity> = children.iter().collect();
    ordered.swap(pos_a, pos_b);

    for &child in &ordered {
        commands.entity(child).remove::<ChildOf>();
    }
    for &child in &ordered {
        commands.entity(child).insert(ChildOf(parent));
    }
}

/// Find the index of `entity` within the filtered positions list.
pub fn find_kind_index(
    entity: Entity,
    children: &Children,
    kind_positions: &[usize],
) -> Option<usize> {
    kind_positions
        .iter()
        .position(|&pos| children[pos] == entity)
}

pub fn resolve_prev(active_idx: usize) -> Option<(usize, usize)> {
    active_idx.checked_sub(1).map(|p| (active_idx, p))
}

pub fn resolve_next(active_idx: usize, len: usize) -> Option<(usize, usize)> {
    (active_idx + 1 < len).then(|| (active_idx, active_idx + 1))
}

/// Move `child` to `new_parent`'s `Children` at `index`.
///
/// Clamps `index` to a valid slot. Works around a Bevy 0.18 `Vec<Entity>::place`
/// panic when the child is already present and `index >= len`.
pub fn move_to_index(world: &mut World, child: Entity, new_parent: Entity, index: usize) {
    let already_child = world
        .get::<ChildOf>(child)
        .is_some_and(|c| c.parent() == new_parent);
    let current_len = world
        .get::<Children>(new_parent)
        .map(|c| c.len())
        .unwrap_or(0);
    let clamped = if already_child {
        index.min(current_len.saturating_sub(1))
    } else {
        index.min(current_len)
    };
    if already_child
        && let Some(children) = world.get::<Children>(new_parent)
        && children.get(clamped) == Some(&child)
    {
        return;
    }
    world.entity_mut(new_parent).insert_child(clamped, child);
}

/// If `split` has exactly one child, replace `split` with its child in the
/// grandparent's `Children` and despawn `split`. No-op otherwise.
pub fn collapse_if_single_child(world: &mut World, split: Entity) {
    let children = match world.get::<Children>(split) {
        Some(c) => c.iter().collect::<Vec<_>>(),
        None => return,
    };

    if children.len() != 1 {
        return;
    }
    let only_child = children[0];

    let grandparent = world.get::<ChildOf>(split).map(|p| p.parent());

    if let Some(gp) = grandparent {
        let gp_index = world
            .get::<Children>(gp)
            .and_then(|kids| kids.iter().position(|e| e == split));
        if let Some(idx) = gp_index {
            move_to_index(world, only_child, gp, idx);
        } else {
            world.entity_mut(only_child).remove::<ChildOf>();
            world.entity_mut(only_child).insert(ChildOf(gp));
        }
    } else {
        world.entity_mut(only_child).remove::<ChildOf>();
    }

    world.entity_mut(split).despawn();
}

/// Create a new `PaneSplit` of `direction` containing `target` and `dragged`.
///
/// Child order is `[dragged, target]` when `dragged_first`, else `[target, dragged]`.
/// If `target` had a parent, the new split takes `target`'s old slot in that parent.
/// If `target` had no parent, the new split is returned unparented — the caller
/// must attach it.
///
/// Returns the new split entity.
pub fn wrap_in_split(
    world: &mut World,
    target: Entity,
    direction: crate::pane::PaneSplitDirection,
    dragged: Entity,
    dragged_first: bool,
) -> Entity {
    use crate::pane::PaneSplit;

    let grandparent = world.get::<ChildOf>(target).map(|p| p.parent());
    let target_idx = grandparent.and_then(|gp| {
        world
            .get::<Children>(gp)
            .and_then(|c| c.iter().position(|e| e == target))
    });

    let split = world.spawn(PaneSplit { direction }).id();

    world.entity_mut(target).remove::<ChildOf>();
    world.entity_mut(dragged).remove::<ChildOf>();
    world.entity_mut(target).insert(ChildOf(split));
    world.entity_mut(dragged).insert(ChildOf(split));

    if dragged_first {
        move_to_index(world, dragged, split, 0);
    } else {
        move_to_index(world, target, split, 0);
    }

    if let (Some(gp), Some(idx)) = (grandparent, target_idx) {
        world.entity_mut(split).insert(ChildOf(gp));
        move_to_index(world, split, gp, idx);
    }

    split
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spawn_parent_with_children(world: &mut World, n: usize) -> (Entity, Vec<Entity>) {
        let parent = world.spawn_empty().id();
        let kids: Vec<Entity> = (0..n).map(|_| world.spawn(ChildOf(parent)).id()).collect();
        (parent, kids)
    }

    #[test]
    fn move_to_index_reorders_within_same_parent() {
        let mut world = World::new();
        let (parent, kids) = spawn_parent_with_children(&mut world, 3);
        move_to_index(&mut world, kids[2], parent, 0);
        let children = world.get::<Children>(parent).unwrap();
        assert_eq!(&**children, &[kids[2], kids[0], kids[1]]);
    }

    #[test]
    fn move_to_index_reparents_across_parents() {
        let mut world = World::new();
        let (p1, a) = spawn_parent_with_children(&mut world, 2);
        let (p2, b) = spawn_parent_with_children(&mut world, 1);
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

    #[test]
    fn move_to_index_same_position_is_noop() {
        let mut world = World::new();
        let (parent, kids) = spawn_parent_with_children(&mut world, 3);
        move_to_index(&mut world, kids[1], parent, 1);
        let children = world.get::<Children>(parent).unwrap();
        assert_eq!(&**children, &[kids[0], kids[1], kids[2]]);
    }

    #[test]
    fn move_to_index_to_last_valid_slot() {
        let mut world = World::new();
        let (parent, kids) = spawn_parent_with_children(&mut world, 3);
        move_to_index(&mut world, kids[0], parent, 2);
        let children = world.get::<Children>(parent).unwrap();
        assert_eq!(&**children, &[kids[1], kids[2], kids[0]]);
    }

    use crate::pane::{PaneSplit, PaneSplitDirection};

    fn spawn_split(world: &mut World, dir: PaneSplitDirection, parent: Entity) -> Entity {
        world
            .spawn((PaneSplit { direction: dir }, ChildOf(parent)))
            .id()
    }

    #[test]
    fn collapse_replaces_single_child_split() {
        let mut world = World::new();
        let root = world.spawn_empty().id();
        let split = spawn_split(&mut world, PaneSplitDirection::Row, root);
        let only_child = world.spawn(ChildOf(split)).id();

        collapse_if_single_child(&mut world, split);

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
            true,
        );

        let root_kids = world.get::<Children>(root).unwrap();
        assert_eq!(&**root_kids, &[split]);

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

        let split = wrap_in_split(&mut world, target, PaneSplitDirection::Row, dragged, false);

        let root_kids = world.get::<Children>(root).unwrap();
        assert_eq!(&**root_kids, &[a, split, c]);

        let split_kids = world.get::<Children>(split).unwrap();
        assert_eq!(&**split_kids, &[target, dragged]);
    }

    #[test]
    fn wrap_leaves_split_parentless_when_target_is_root() {
        let mut world = World::new();
        let target = world.spawn_empty().id();
        let dragged = world.spawn_empty().id();

        let split = wrap_in_split(&mut world, target, PaneSplitDirection::Row, dragged, true);

        assert!(world.get::<ChildOf>(split).is_none());
        let kids = world.get::<Children>(split).unwrap();
        assert_eq!(&**kids, &[dragged, target]);
    }
}
