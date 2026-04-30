use crate::{
    command::{AppCommand, ReadAppCommands, SpaceCommand},
    command_bar::NewTabContext,
    layout::pane::{Pane, PaneSplit, PaneSplitDirection, leaf_pane_bundle},
    layout::swap::{find_kind_index, resolve_next, resolve_prev, swap_siblings},
    layout::tab::tab_bundle,
    layout::window::Main,
    settings::AppSettings,
};
use bevy::{
    ecs::{message::Messages, relationship::Relationship},
    prelude::*,
    window::PrimaryWindow,
};
use bevy_cef::prelude::*;
use moonshine_save::prelude::*;
use vmux_footer::{
    Footer,
    event::{FooterCommandEvent, SPACES_EVENT, SpaceRow, SpacesHostEvent},
};
use vmux_history::{CreatedAt, LastActivatedAt};
use vmux_webview_app::UiReady;

pub(crate) struct SpacePlugin;

impl Plugin for SpacePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Space>()
            .add_plugins(JsEmitEventPlugin::<FooterCommandEvent>::default())
            .add_observer(on_footer_command_emit)
            .add_systems(Update, handle_space_commands.in_set(ReadAppCommands))
            .add_systems(Update, push_spaces_host_emit)
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
        Visibility::default(),
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

/// Spawn a new Space (with a leaf pane + empty tab) under `Main` and
/// activate it. The empty tab triggers the command bar via `NewTabContext`.
fn spawn_new_space(
    main: Entity,
    pw: Entity,
    name: String,
    settings: &AppSettings,
    new_tab_ctx: &mut NewTabContext,
    commands: &mut Commands,
) -> Entity {
    let space = commands
        .spawn((
            Space { name },
            Transform::default(),
            GlobalTransform::default(),
            Visibility::default(),
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
            LastActivatedAt::now(),
            CreatedAt::now(),
            ChildOf(main),
        ))
        .id();

    let split_root = commands
        .spawn((
            Pane,
            PaneSplit {
                direction: PaneSplitDirection::Row,
            },
            HostWindow(pw),
            ZIndex(0),
            Transform::default(),
            GlobalTransform::default(),
            Node {
                flex_grow: 1.0,
                min_height: Val::Px(0.0),
                column_gap: Val::Px(settings.layout.pane.gap),
                row_gap: Val::Px(settings.layout.pane.gap),
                ..default()
            },
            ChildOf(space),
        ))
        .id();

    let leaf = commands
        .spawn((
            leaf_pane_bundle(),
            LastActivatedAt::now(),
            ChildOf(split_root),
        ))
        .id();

    let tab = commands
        .spawn((
            tab_bundle(),
            LastActivatedAt::now(),
            CreatedAt::now(),
            ChildOf(leaf),
        ))
        .id();

    new_tab_ctx.tab = Some(tab);
    new_tab_ctx.previous_tab = None;
    new_tab_ctx.needs_open = true;

    space
}

#[allow(clippy::too_many_arguments)]
fn handle_space_commands(
    mut reader: MessageReader<AppCommand>,
    spaces: Query<(Entity, &LastActivatedAt), With<Space>>,
    space_q: Query<Entity, With<Space>>,
    main_q: Query<Entity, With<Main>>,
    primary_window: Single<Entity, With<PrimaryWindow>>,
    child_of_q: Query<&ChildOf>,
    all_children: Query<&Children>,
    settings: Res<AppSettings>,
    mut new_tab_ctx: ResMut<NewTabContext>,
    mut commands: Commands,
) {
    for cmd in reader.read() {
        let AppCommand::Space(space_cmd) = *cmd else {
            continue;
        };

        let active_space = spaces.iter().max_by_key(|(_, ts)| ts.0).map(|(e, _)| e);

        match space_cmd {
            SpaceCommand::New => {
                let Ok(main) = main_q.single() else { continue };
                let count = spaces.iter().count();
                let name = format!("Space {}", count + 1);
                spawn_new_space(
                    main,
                    *primary_window,
                    name,
                    &settings,
                    &mut new_tab_ctx,
                    &mut commands,
                );
            }
            SpaceCommand::Close => {
                let Some(active) = active_space else { continue };
                let siblings = active_space_siblings(active, &child_of_q, &all_children, &space_q);
                if siblings.len() <= 1 {
                    // Closing the only remaining space: spawn a fresh empty
                    // space first so the user is never left without one.
                    let Ok(main) = main_q.single() else { continue };
                    let count = spaces.iter().count();
                    let name = format!("Space {}", count + 1);
                    spawn_new_space(
                        main,
                        *primary_window,
                        name,
                        &settings,
                        &mut new_tab_ctx,
                        &mut commands,
                    );
                } else if let Some(next) = pick_after_close(active, &siblings) {
                    commands.entity(next).insert(LastActivatedAt::now());
                }
                commands.entity(active).despawn();
            }
            SpaceCommand::Next | SpaceCommand::Previous => {
                let Some(active) = active_space else { continue };
                let siblings = active_space_siblings(active, &child_of_q, &all_children, &space_q);
                if siblings.len() <= 1 {
                    continue;
                }
                let Some(idx) = siblings.iter().position(|e| *e == active) else {
                    continue;
                };
                let target_idx = if space_cmd == SpaceCommand::Next {
                    (idx + 1) % siblings.len()
                } else {
                    (idx + siblings.len() - 1) % siblings.len()
                };
                let target = siblings[target_idx];
                if target != active {
                    commands.entity(target).insert(LastActivatedAt::now());
                }
            }
            SpaceCommand::Rename => {
                // Reserved: command bar prompt for rename will land in a follow-up.
            }
            SpaceCommand::SwapPrev | SpaceCommand::SwapNext => {
                let Some(active) = active_space else { continue };
                let Ok(co) = child_of_q.get(active) else {
                    continue;
                };
                let parent = co.get();
                let Ok(children) = all_children.get(parent) else {
                    continue;
                };
                let kind_positions: Vec<usize> = children
                    .iter()
                    .enumerate()
                    .filter(|(_, e)| space_q.contains(*e))
                    .map(|(i, _)| i)
                    .collect();
                let Some(active_idx) = find_kind_index(active, children, &kind_positions) else {
                    continue;
                };
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

/// Returns sibling Space entities under the same parent in child order.
fn active_space_siblings(
    active: Entity,
    child_of_q: &Query<&ChildOf>,
    all_children: &Query<&Children>,
    space_q: &Query<Entity, With<Space>>,
) -> Vec<Entity> {
    let Ok(co) = child_of_q.get(active) else {
        return vec![active];
    };
    let parent = co.get();
    let Ok(children) = all_children.get(parent) else {
        return vec![active];
    };
    children
        .iter()
        .filter(|e| space_q.contains(*e))
        .collect::<Vec<_>>()
}

/// When closing `active`, return the entity that should become active.
fn pick_after_close(active: Entity, siblings: &[Entity]) -> Option<Entity> {
    if siblings.len() <= 1 {
        return None;
    }
    let idx = siblings.iter().position(|e| *e == active)?;
    let next_idx = if idx + 1 < siblings.len() { idx + 1 } else { 0 };
    let target = siblings[next_idx];
    if target == active { None } else { Some(target) }
}

fn sync_space_visibility(
    mut spaces: Query<(Entity, &LastActivatedAt, &mut Node, &mut Visibility), With<Space>>,
) {
    let active = spaces
        .iter()
        .max_by_key(|(_, ts, _, _)| ts.0)
        .map(|(e, _, _, _)| e);
    for (entity, _, mut node, mut vis) in &mut spaces {
        let is_active = Some(entity) == active;
        let target_display = if is_active {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != target_display {
            node.display = target_display;
        }
        let target_vis = if is_active {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != target_vis {
            *vis = target_vis;
        }
    }
}

/// Push the current set of spaces (and the active marker) to the footer
/// webview whenever the payload changes.
#[allow(clippy::too_many_arguments)]
fn push_spaces_host_emit(
    mut commands: Commands,
    browsers: NonSend<Browsers>,
    footer: Option<Single<Entity, (With<Footer>, With<UiReady>)>>,
    spaces: Query<(Entity, &Space, &LastActivatedAt)>,
    child_of_q: Query<&ChildOf>,
    all_children: Query<&Children>,
    space_q: Query<Entity, With<Space>>,
    mut last: Local<String>,
) {
    let Some(footer) = footer else { return };
    let footer_e = *footer;
    if !browsers.has_browser(footer_e) || !browsers.host_emit_ready(&footer_e) {
        return;
    }

    let active_space = spaces.iter().max_by_key(|(_, _, ts)| ts.0).map(|t| t.0);

    // Stable sibling order: pick any space, walk its parent's children.
    let ordered = if let Some(any) = spaces.iter().next() {
        active_space_siblings(any.0, &child_of_q, &all_children, &space_q)
    } else {
        Vec::new()
    };

    let rows: Vec<SpaceRow> = ordered
        .iter()
        .filter_map(|e| spaces.get(*e).ok())
        .map(|(entity, space, _)| SpaceRow {
            id: entity.to_bits().to_string(),
            name: if space.name.is_empty() {
                "Space".to_string()
            } else {
                space.name.clone()
            },
            is_active: Some(entity) == active_space,
        })
        .collect();

    let payload = SpacesHostEvent { spaces: rows };
    let body = ron::ser::to_string(&payload).unwrap_or_default();
    if body == *last {
        return;
    }
    commands.trigger(HostEmitEvent::new(footer_e, SPACES_EVENT, &body));
    *last = body;
}

fn on_footer_command_emit(
    trigger: On<Receive<FooterCommandEvent>>,
    spaces: Query<Entity, With<Space>>,
    mut messages: ResMut<Messages<AppCommand>>,
    mut commands: Commands,
) {
    let evt = &trigger.event().payload;
    match evt.command.as_str() {
        "new" => {
            messages.write(AppCommand::Space(SpaceCommand::New));
        }
        "close" => {
            messages.write(AppCommand::Space(SpaceCommand::Close));
        }
        "switch" => {
            let Some(id_str) = evt.space_id.as_deref() else {
                return;
            };
            let Ok(bits) = id_str.parse::<u64>() else {
                return;
            };
            let Some(target) = spaces.iter().find(|e| e.to_bits() == bits) else {
                return;
            };
            commands.entity(target).insert(LastActivatedAt::now());
        }
        _ => {}
    }
}
