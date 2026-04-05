use bevy::asset::*;
use bevy::pbr::MaterialPlugin;
use bevy::prelude::*;
use bevy::render::alpha::AlphaMode;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;

use crate::command::AppCommand;
use vmux_history::{CreatedAt, LastActivatedAt};

pub struct LayoutPlugin;

impl Plugin for LayoutPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((MaterialPlugin::<OutlineMaterial>::default(),))
            .add_observer(spawn_window_on_space_added)
            .add_observer(spawn_pane_on_window_added)
            .add_observer(spawn_tab_on_pane_added)
            .add_systems(Startup, spawn_space_on_startup)
            .add_systems(Update, handle_new_space);
        load_internal_asset!(app, OUTLINE_SHADER, "./outline.wgsl", Shader::from_wgsl);
    }
}

#[derive(Bundle, Default)]
struct SpatialBundle {
    transform: Transform,
    global_transform: GlobalTransform,
    visibility: Visibility,
    inherited_visibility: InheritedVisibility,
    view_visibility: ViewVisibility,
}

#[derive(Bundle)]
struct SpaceBundle {
    space: Space,
    name: Name,
    spatial: SpatialBundle,
    created_at: CreatedAt,
    last_activated_at: LastActivatedAt,
}

#[derive(Component)]
struct Space;

#[derive(Bundle)]
struct WindowBundle {
    window: Window,
    name: Name,
    spatial: SpatialBundle,
    created_at: CreatedAt,
    last_activated_at: LastActivatedAt,
}

#[derive(Component)]
struct Window;

#[derive(Bundle)]
struct PaneBundle {
    pane: Pane,
    weight: Weight,
    name: Name,
    spatial: SpatialBundle,
    created_at: CreatedAt,
    last_activated_at: LastActivatedAt,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
enum Pane {
    Horizontal,
}

#[derive(Component, Clone, Copy, Debug)]
struct Weight(f32);

#[derive(Component)]
pub(crate) struct Outline;

const OUTLINE_SHADER: Handle<Shader> = uuid_handle!("c4a8e901-2b7d-4c1e-9f63-7a2d8e5b1044");

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub(crate) struct OutlineMaterial {
    #[uniform(0)]
    pub pane_inner: Vec4,
    #[uniform(1)]
    pub pane_outer: Vec4,
    #[uniform(2)]
    pub border_color: Vec4,
    #[uniform(3)]
    pub glow_params: Vec4,
    #[uniform(4)]
    pub gradient_params: Vec4,
    #[uniform(5)]
    pub border_accent: Vec4,
    pub alpha_mode: AlphaMode,
}

impl Material for OutlineMaterial {
    fn fragment_shader() -> ShaderRef {
        OUTLINE_SHADER.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        self.alpha_mode
    }
}

#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct Tab;

#[derive(Bundle)]
struct TabBundle {
    tab: Tab,
    name: Name,
    spatial: SpatialBundle,
    created_at: CreatedAt,
    last_activated_at: LastActivatedAt,
}

fn spawn_space_on_startup(mut msg: MessageWriter<AppCommand>, q: Query<&Space>) {
    if q.is_empty() {
        msg.write(AppCommand::NewSpace);
    }
}

fn handle_new_space(mut msg: MessageReader<AppCommand>, mut commands: Commands) {
    for cmd in msg.read() {
        match cmd {
            AppCommand::NewSpace => {
                commands.spawn(SpaceBundle {
                    space: Space,
                    name: Name::new("Space 1"),
                    spatial: SpatialBundle::default(),
                    created_at: CreatedAt::default(),
                    last_activated_at: LastActivatedAt::default(),
                });
            }
            _ => {}
        }
    }
}

fn spawn_window_on_space_added(
    add: On<Add, Space>,
    mut commands: Commands,
    children_q: Query<&Children>,
    window_q: Query<(), With<Window>>,
) {
    let space = add.entity;
    if let Ok(children) = children_q.get(space) {
        for child in children.iter() {
            if window_q.contains(child) {
                return;
            }
        }
    }
    commands.entity(space).insert(children![WindowBundle {
        window: Window,
        name: Name::new("Default Window"),
        spatial: SpatialBundle::default(),
        created_at: CreatedAt::default(),
        last_activated_at: LastActivatedAt::default(),
    }]);
}

fn spawn_pane_on_window_added(
    add: On<Add, Window>,
    mut commands: Commands,
    children_q: Query<&Children>,
    pane_q: Query<(), With<Pane>>,
) {
    let window = add.entity;
    if let Ok(children) = children_q.get(window) {
        for child in children.iter() {
            if pane_q.contains(child) {
                return;
            }
        }
    }
    let w0 = Weight(1.0);
    let w0_share = w0.0;
    commands.entity(window).insert(children![PaneBundle {
        pane: Pane::Horizontal,
        weight: w0,
        name: Name::new(format!("Pane {:.2}", w0_share)),
        spatial: SpatialBundle::default(),
        created_at: CreatedAt::default(),
        last_activated_at: LastActivatedAt::default(),
    }]);
}

fn spawn_tab_on_pane_added(
    add: On<Add, Pane>,
    mut commands: Commands,
    children_q: Query<&Children>,
    tab_q: Query<(), With<Tab>>,
) {
    let pane = add.entity;
    if let Ok(children) = children_q.get(pane) {
        for child in children.iter() {
            if tab_q.contains(child) {
                return;
            }
        }
    }
    commands.entity(pane).insert(children![TabBundle {
        tab: Tab,
        name: Name::new("New Tab"),
        spatial: SpatialBundle::default(),
        created_at: CreatedAt::default(),
        last_activated_at: LastActivatedAt::default(),
    }]);
}
