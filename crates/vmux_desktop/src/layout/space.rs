use crate::{
    command::{AppCommand, ReadAppCommands, SpaceCommand},
    layout::swap::{find_kind_index, resolve_prev, resolve_next, swap_siblings},
};
use bevy::{ecs::relationship::Relationship, prelude::*};
use moonshine_save::prelude::*;
use vmux_history::LastActivatedAt;

pub(crate) struct SpacePlugin;

impl Plugin for SpacePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Space>()
            .add_systems(Update, handle_space_commands.in_set(ReadAppCommands))
            .add_systems(PostUpdate, sync_space_visibility);
    }
}

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
#[require(Save)]
pub(crate) struct Space {
    pub name: String,
}

pub(crate) fn space_bundle() -> impl Bundle {
    (
        Space::default(),
        Transform::default(),
        GlobalTransform::default(),
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            top: Val::Px(0.0),
            bottom: Val::Px(0.0),
            ..default()
        },
    )
}

fn handle_space_commands(
    mut reader: MessageReader<AppCommand>,
    spaces: Query<(Entity, &LastActivatedAt), With<Space>>,
    space_q: Query<Entity, With<Space>>,
    child_of_q: Query<&ChildOf>,
    all_children: Query<&Children>,
    mut commands: Commands,
) {
    for cmd in reader.read() {
        let AppCommand::Space(space_cmd) = *cmd else {
            continue;
        };

        let active_space = spaces.iter().max_by_key(|(_, ts)| ts.0).map(|(e, _)| e);

        match space_cmd {
            SpaceCommand::New => {}
            SpaceCommand::Close => {}
            SpaceCommand::Next => {}
            SpaceCommand::Previous => {}
            SpaceCommand::Rename => {}
            SpaceCommand::SwapPrev | SpaceCommand::SwapNext => {
                let Some(active) = active_space else { continue };
                let Ok(co) = child_of_q.get(active) else { continue };
                let parent = co.get();
                let Ok(children) = all_children.get(parent) else { continue };
                let kind_positions: Vec<usize> = children.iter()
                    .enumerate()
                    .filter(|(_, e)| space_q.contains(*e))
                    .map(|(i, _)| i)
                    .collect();
                let Some(active_idx) = find_kind_index(active, children, &kind_positions) else { continue };
                let pair = if space_cmd == SpaceCommand::SwapPrev {
                    resolve_prev(active_idx)
                } else {
                    resolve_next(active_idx, kind_positions.len())
                };
                if let Some((a, b)) = pair {
                    swap_siblings(&mut commands, parent, children, &kind_positions, a, b);
                }
            }
        }
    }
}

fn sync_space_visibility(
    mut spaces: Query<(Entity, &LastActivatedAt, &mut Node), With<Space>>,
) {
    let active = spaces.iter().max_by_key(|(_, ts, _)| ts.0).map(|(e, _, _)| e);
    for (entity, _, mut node) in &mut spaces {
        let target = if Some(entity) == active { Display::Flex } else { Display::None };
        if node.display != target {
            node.display = target;
        }
    }
}
