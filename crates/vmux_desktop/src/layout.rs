use crate::{
    command::{AppCommand, PaneCommand, ReadAppCommands, TabCommand, WriteAppCommands},
    rounded::{RoundedCorners, RoundedMaterial},
    scene::MainCamera,
    settings::{AppSettings, load_settings},
    unit::{PIXELS_PER_METER, WindowExt},
};
use bevy::{
    asset::*,
    ecs::query::Has,
    ecs::relationship::Relationship,
    pbr::MaterialPlugin,
    prelude::*,
    render::alpha::AlphaMode,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
    ui::{FlexDirection, UiGlobalTransform, UiSystems, UiTargetCamera, ZIndex},
    window::PrimaryWindow,
};
use bevy_cef::prelude::*;
use bevy_cef_core::prelude::RenderTextureMessage;
use std::{collections::HashSet, path::PathBuf};
use vmux_status_bar::{
    STATUS_BAR_WEBVIEW_URL, StatusBar, StatusBarBundle,
    event::{TABS_EVENT, TabRow, TabsHostEvent},
};
use vmux_webview_app::{JsEmitUiReadyPlugin, UiReady};

pub struct LayoutPlugin;

impl Plugin for LayoutPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            JsEmitUiReadyPlugin,
            CefPlugin {
                root_cache_path: cef_root_cache_path(),
                ..default()
            },
            MaterialPlugin::<OutlineMaterial>::default(),
        ))
        .add_systems(
            Startup,
            (setup, spawn_outline, fit_display_glass_to_window)
                .chain()
                .after(load_settings)
                .after(crate::scene::setup),
        )
        .add_systems(
            Update,
            (
                write_tab_hotkeys.in_set(WriteAppCommands),
                (on_pane_cycle, handle_pane_commands).in_set(ReadAppCommands),
                apply_chrome_state_from_cef.after(handle_pane_commands),
                push_tabs_host_emit.after(apply_chrome_state_from_cef),
                tick_outline_gradient_time,
            ),
        )
        .add_systems(
            PostUpdate,
            (
                fit_display_glass_to_window,
                sync_keyboard_target,
                sync_children_to_ui,
                sync_outline_to_active_pane,
                sync_cef_webview_resize_after_ui,
                sync_webview_pane_corner_clip,
                sync_osr_webview_focus,
                kick_tab_startup_navigation,
                flush_pending_osr_textures,
            )
                .chain()
                .after(UiSystems::Layout)
                .before(render_standard_materials),
        )
        .add_observer(on_pane_added)
        .add_observer(on_pane_hover);
        load_internal_asset!(app, OUTLINE_SHADER, "./outline.wgsl", Shader::from_wgsl);
    }
}

#[derive(Bundle)]
struct DisplayGlassBundle<M>
where
    M: Material,
{
    marker: DisplayGlass,
    mesh: Mesh3d,
    material: MeshMaterial3d<M>,
    transform: Transform,
    node: Node,
    ui_target: UiTargetCamera,
}

#[derive(Component)]
pub struct DisplayGlass;

#[derive(Component)]
pub struct Pane;

#[derive(Component)]
struct PaneSplit(FlexDirection);

#[derive(Component)]
struct Active;

#[derive(Component, Clone, Debug)]
struct PageMetadata {
    title: String,
    url: String,
    favicon_url: String,
}

#[derive(Component)]
struct Browser;

const OUTLINE_SHADER: Handle<Shader> = uuid_handle!("c4a8e901-2b7d-4c1e-9f63-7a2d8e5b1044");

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
struct OutlineMaterial {
    #[uniform(0)]
    pane_inner: Vec4,
    #[uniform(1)]
    pane_outer: Vec4,
    #[uniform(2)]
    border_color: Vec4,
    #[uniform(3)]
    glow_params: Vec4,
    #[uniform(4)]
    gradient_params: Vec4,
    #[uniform(5)]
    border_accent: Vec4,
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

#[derive(Component)]
struct NomadicOutline;

fn setup(
    window: Single<&Window, With<PrimaryWindow>>,
    primary_window: Single<Entity, With<PrimaryWindow>>,
    main_camera: Single<Entity, With<MainCamera>>,
    mut commands: Commands,
    settings: Res<AppSettings>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<RoundedMaterial>>,
    mut webview_mt: ResMut<Assets<WebviewExtendStandardMaterial>>,
) {
    let m = window.meters();
    let pw = *primary_window;

    let display = commands
        .spawn(DisplayGlassBundle {
            marker: DisplayGlass,
            mesh: Mesh3d(meshes.add(Plane3d::new(Vec3::Z, Vec2::splat(0.5)))),
            material: MeshMaterial3d(materials.add(RoundedMaterial {
                base: StandardMaterial {
                    base_color: Color::srgba(0.08, 0.08, 0.08, 0.4),
                    alpha_mode: AlphaMode::Blend,
                    cull_mode: None,
                    perceptual_roughness: 0.12,
                    metallic: 0.0,
                    specular_transmission: 0.9,
                    diffuse_transmission: 1.0,
                    thickness: 0.1,
                    ior: 1.5,
                    ..default()
                },
                extension: RoundedCorners {
                    clip: Vec4::new(settings.layout.pane.radius, m.x, m.y, PIXELS_PER_METER),
                    ..default()
                },
            })),
            transform: Transform {
                translation: Vec3::new(0.0, m.y * 0.5, 0.0),
                scale: Vec3::new(m.x, m.y, 1.0),
                ..default()
            },
            node: Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Relative,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            ui_target: UiTargetCamera(*main_camera),
        })
        .id();

    let main = commands
        .spawn((
            Pane,
            PaneSplit(FlexDirection::Column),
            ChildOf(display),
            HostWindow(pw),
            ZIndex(0),
            Transform::default(),
            GlobalTransform::default(),
            Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                min_height: Val::Px(0.0),
                padding: UiRect::all(Val::Px(settings.layout.window.padding)),
                column_gap: Val::Px(settings.layout.pane.gap),
                row_gap: Val::Px(settings.layout.pane.gap),
                ..default()
            },
        ))
        .id();

    let leaf = spawn_leaf_pane(&mut commands, main);
    commands.entity(leaf).insert(Active);

    spawn_browser(
        &mut commands,
        &mut meshes,
        &mut webview_mt,
        leaf,
        settings.browser.startup_url.as_str(),
    );

    commands.spawn((
        ChildOf(display),
        ZIndex(1),
        HostWindow(pw),
        Browser,
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(STATUS_BAR_HEIGHT_PX),
            flex_shrink: 0.0,
            ..default()
        },
        StatusBarBundle {
            marker: StatusBar,
            source: WebviewSource::new(STATUS_BAR_WEBVIEW_URL),
            mesh: Mesh3d(meshes.add(Plane3d::new(Vec3::Z, Vec2::splat(0.5)))),
            material: MeshMaterial3d(webview_mt.add(WebviewExtendStandardMaterial {
                base: StandardMaterial {
                    unlit: true,
                    alpha_mode: AlphaMode::Blend,
                    depth_bias: WEBVIEW_MESH_DEPTH_BIAS,
                    ..default()
                },
                ..default()
            })),
            webview_size: WebviewSize(Vec2::new(1280.0, STATUS_BAR_HEIGHT_PX)),
        },
    ));
}

fn spawn_leaf_pane(commands: &mut Commands, parent: Entity) -> Entity {
    commands
        .spawn((
            Pane,
            ChildOf(parent),
            Transform::default(),
            GlobalTransform::default(),
            Node {
                flex_grow: 1.0,
                flex_basis: Val::Px(0.0),
                align_items: AlignItems::Stretch,
                justify_content: JustifyContent::Stretch,
                ..default()
            },
        ))
        .id()
}

fn spawn_browser(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    webview_mt: &mut ResMut<Assets<WebviewExtendStandardMaterial>>,
    parent: Entity,
    url: &str,
) {
    commands.spawn((
        Browser,
        PageMetadata {
            title: url.to_string(),
            url: url.to_string(),
            favicon_url: String::new(),
        },
        WebviewSource::new(url),
        Mesh3d(meshes.add(Plane3d::new(Vec3::Z, Vec2::splat(0.5)))),
        MeshMaterial3d(webview_mt.add(WebviewExtendStandardMaterial {
            base: StandardMaterial {
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                depth_bias: WEBVIEW_MESH_DEPTH_BIAS,
                ..default()
            },
            ..default()
        })),
        WebviewSize(Vec2::new(1280.0, 720.0)),
        ChildOf(parent),
        Transform::default(),
        GlobalTransform::default(),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            top: Val::Px(0.0),
            bottom: Val::Px(0.0),
            ..default()
        },
        Visibility::Inherited,
    ));
}

fn first_leaf_descendant(
    entity: Entity,
    children_q: &Query<&Children, With<Pane>>,
    leaf_q: &Query<Entity, (With<Pane>, Without<PaneSplit>)>,
) -> Entity {
    if leaf_q.contains(entity) {
        return entity;
    }
    if let Ok(children) = children_q.get(entity) {
        for child in children.iter() {
            if leaf_q.contains(child) {
                return child;
            }
            let found = first_leaf_descendant(child, children_q, leaf_q);
            if found != child || leaf_q.contains(found) {
                return found;
            }
        }
    }
    entity
}

fn write_tab_hotkeys(keyboard: Res<ButtonInput<KeyCode>>, mut writer: MessageWriter<AppCommand>) {
    let ctrl = keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight);
    let meta = keyboard.pressed(KeyCode::SuperLeft) || keyboard.pressed(KeyCode::SuperRight);
    if !keyboard.just_pressed(KeyCode::Tab) || (!ctrl && !meta) {
        return;
    }
    let shift = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);
    if shift {
        writer.write(AppCommand::Tab(TabCommand::Previous));
    } else {
        writer.write(AppCommand::Tab(TabCommand::Next));
    }
}

fn handle_pane_commands(
    mut reader: MessageReader<AppCommand>,
    active_pane: Query<Entity, With<Active>>,
    pane_children: Query<&Children, With<Pane>>,
    child_of_q: Query<&ChildOf>,
    pane_q: Query<(), With<Pane>>,
    split_q: Query<(), With<PaneSplit>>,
    browser_filter: Query<Entity, With<Browser>>,
    leaf_panes: Query<Entity, (With<Pane>, Without<PaneSplit>)>,
    settings: Res<AppSettings>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut webview_mt: ResMut<Assets<WebviewExtendStandardMaterial>>,
) {
    for cmd in reader.read() {
        let AppCommand::Pane(pane_cmd) = *cmd else {
            continue;
        };
        let Ok(active) = active_pane.single() else {
            continue;
        };

        match pane_cmd {
            PaneCommand::SplitV | PaneCommand::SplitH => {
                let direction = if pane_cmd == PaneCommand::SplitV {
                    FlexDirection::Row
                } else {
                    FlexDirection::Column
                };

                let existing_browsers: Vec<Entity> = pane_children
                    .get(active)
                    .map(|c| c.iter().filter(|&e| browser_filter.contains(e)).collect())
                    .unwrap_or_default();

                let pane1 = spawn_leaf_pane(&mut commands, active);
                let pane2 = spawn_leaf_pane(&mut commands, active);

                for browser in existing_browsers {
                    commands.entity(browser).insert(ChildOf(pane1));
                }

                spawn_browser(
                    &mut commands,
                    &mut meshes,
                    &mut webview_mt,
                    pane2,
                    settings.browser.startup_url.as_str(),
                );

                commands
                    .entity(active)
                    .insert(PaneSplit(direction))
                    .remove::<Active>();
                let gap = Val::Px(settings.layout.pane.gap);
                commands
                    .entity(active)
                    .entry::<Node>()
                    .and_modify(move |mut n| {
                        n.flex_direction = direction;
                        n.column_gap = gap;
                        n.row_gap = gap;
                    });

                commands.entity(pane2).insert(Active);
            }
            PaneCommand::Close => {
                let Ok(child_of) = child_of_q.get(active) else {
                    continue;
                };
                let parent = child_of.get();
                if !split_q.contains(parent) {
                    continue;
                }

                let Ok(siblings) = pane_children.get(parent) else {
                    continue;
                };
                let sibling = siblings
                    .iter()
                    .find(|&e| e != active && pane_q.contains(e));
                let Some(sibling) = sibling else {
                    continue;
                };

                let new_active = if split_q.contains(sibling) {
                    first_leaf_descendant(sibling, &pane_children, &leaf_panes)
                } else {
                    sibling
                };

                let sibling_children: Vec<Entity> = pane_children
                    .get(sibling)
                    .map(|c| c.iter().collect())
                    .unwrap_or_default();

                for child in sibling_children {
                    commands.entity(child).insert(ChildOf(parent));
                }

                if split_q.contains(sibling) {
                    commands.entity(sibling).remove::<ChildOf>();
                    commands.queue(move |world: &mut World| {
                        world.despawn(sibling);
                    });
                } else {
                    commands.entity(parent).remove::<PaneSplit>();
                    commands.entity(sibling).insert(ChildOf(parent));
                }

                commands.entity(active).despawn();
                commands.entity(new_active).insert(Active);
            }
            PaneCommand::Toggle => {}
        }
    }
}

fn on_pane_cycle(
    mut reader: MessageReader<AppCommand>,
    leaf_panes: Query<Entity, (With<Pane>, Without<PaneSplit>)>,
    active_pane: Query<Entity, With<Active>>,
    mut commands: Commands,
) {
    for cmd in reader.read() {
        let delta: i32 = match cmd {
            AppCommand::Tab(TabCommand::Next) => 1,
            AppCommand::Tab(TabCommand::Previous) => -1,
            _ => continue,
        };
        let mut panes: Vec<Entity> = leaf_panes.iter().collect();
        if panes.len() < 2 {
            continue;
        }
        panes.sort_by_key(|e| e.to_bits());
        let Ok(current) = active_pane.single() else {
            continue;
        };
        let Some(pos) = panes.iter().position(|&e| e == current) else {
            continue;
        };
        let n = panes.len() as i32;
        let idx = (pos as i32 + delta).rem_euclid(n) as usize;
        let target = panes[idx];
        commands.entity(current).remove::<Active>();
        commands.entity(target).insert(Active);
    }
}

fn on_pane_added(trigger: On<Add, Pane>, mut commands: Commands) {
    commands
        .entity(trigger.entity)
        .observe(on_pane_hover);
}

fn on_pane_hover(
    trigger: On<Pointer<Over>>,
    pane_q: Query<(), (With<Pane>, Without<PaneSplit>)>,
    active_q: Query<Entity, With<Active>>,
    mut commands: Commands,
) {
    let entity = trigger.entity;
    if !pane_q.contains(entity) {
        return;
    }
    if let Ok(current) = active_q.single() {
        if current == entity {
            return;
        }
        commands.entity(current).remove::<Active>();
    }
    commands.entity(entity).insert(Active);
}

fn sync_keyboard_target(
    browsers: NonSend<Browsers>,
    active_pane: Query<Entity, With<Active>>,
    child_of_q: Query<&ChildOf>,
    status_q: Query<(), With<StatusBar>>,
    mut browser_q: Query<(Entity, &mut Visibility, Has<CefKeyboardTarget>), With<Browser>>,
    mut commands: Commands,
) {
    let Ok(active_entity) = active_pane.single() else {
        return;
    };
    for (browser_e, mut visibility, has_kb) in &mut browser_q {
        if status_q.contains(browser_e) {
            continue;
        }
        *visibility = Visibility::Inherited;
        browsers.set_osr_not_hidden(&browser_e);

        let in_active = child_of_q
            .get(browser_e)
            .ok()
            .map(|co| co.get() == active_entity)
            .unwrap_or(false);

        if in_active && !has_kb {
            commands.entity(browser_e).insert(CefKeyboardTarget);
        } else if !in_active && has_kb {
            commands.entity(browser_e).remove::<CefKeyboardTarget>();
        }
    }
}

pub fn fit_display_glass_to_window(
    window: Single<&Window, With<PrimaryWindow>>,
    settings: Res<AppSettings>,
    mut materials: ResMut<Assets<RoundedMaterial>>,
    mut last_size: Local<Vec2>,
    mut q: Query<(&mut Transform, &MeshMaterial3d<RoundedMaterial>), With<DisplayGlass>>,
) {
    let m = window.meters();
    if (m.x - last_size.x).abs() < 0.001 && (m.y - last_size.y).abs() < 0.001 {
        return;
    }
    *last_size = m;

    let r = settings.layout.pane.radius;

    for (mut tf, handle) in &mut q {
        tf.translation = Vec3::new(0.0, m.y * 0.5, 0.0);
        tf.scale = Vec3::new(m.x, m.y, 1.0);

        if let Some(mat) = materials.get_mut(handle) {
            mat.extension.clip = Vec4::new(r, m.x, m.y, PIXELS_PER_METER);
        }
    }
}

fn sync_children_to_ui(
    mut browser_q: Query<
        (
            &mut Transform,
            &ComputedNode,
            &UiGlobalTransform,
            &ChildOf,
            &mut WebviewSize,
            Option<&StatusBar>,
        ),
        With<Browser>,
    >,
    pane_rect: Query<(&ComputedNode, &UiGlobalTransform), With<Pane>>,
    glass: Single<(Entity, &ComputedNode, &UiGlobalTransform), With<DisplayGlass>>,
) {
    let &(glass_entity, glass_node, glass_ui_gt) = &*glass;

    for (mut tf, self_computed, self_ui_gt, child_of, mut webview_size, status) in
        browser_q.iter_mut()
    {
        let parent = child_of.get();
        let (computed, ui_gt) = match pane_rect.get(parent) {
            Ok((cn, gt)) => (cn, gt),
            Err(_) => (self_computed, self_ui_gt),
        };

        let glass_size_px = glass_node.size;
        if glass_size_px.x <= 0.0 || glass_size_px.y <= 0.0 {
            continue;
        }

        let size_px = computed.size;
        if size_px.x <= 0.0 || size_px.y <= 0.0 {
            continue;
        }

        let sx = size_px.x / glass_size_px.x;
        let sy = size_px.y / glass_size_px.y;
        tf.scale = Vec3::new(sx, sy, 1.0);

        let center_ui = ui_gt.transform_point2(Vec2::ZERO);
        let glass_center_ui = glass_ui_gt.transform_point2(Vec2::ZERO);
        let delta_px = center_ui - glass_center_ui;

        let tx = delta_px.x / glass_size_px.x;
        let ty = -delta_px.y / glass_size_px.y;
        let z = if status.is_some() {
            WEBVIEW_Z_STATUS
        } else if parent != glass_entity {
            WEBVIEW_Z_MAIN
        } else {
            0.01 + self_computed.stack_index as f32 * 0.001
        };
        tf.translation = Vec3::new(tx, ty, z);

        let dip = (size_px * computed.inverse_scale_factor).max(Vec2::splat(1.0));
        if webview_size.0 != dip {
            webview_size.0 = dip;
        }
    }
}

fn sync_cef_webview_resize_after_ui(
    browsers: NonSend<Browsers>,
    webviews: Query<(Entity, &WebviewSize), (Changed<WebviewSize>, With<Browser>)>,
    host_window: Query<&HostWindow>,
    windows: Query<&Window>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
) {
    for (entity, size) in webviews.iter() {
        if !browsers.has_browser(entity) {
            continue;
        }
        let window_entity = host_window
            .get(entity)
            .ok()
            .map(|h| h.0)
            .or_else(|| primary_window.single().ok());
        let device_scale_factor = window_entity
            .and_then(|e| windows.get(e).ok())
            .map(|w| w.resolution.scale_factor() as f32)
            .filter(|s| s.is_finite() && *s > 0.0)
            .unwrap_or(1.0);
        browsers.resize(&entity, size.0, device_scale_factor);
    }
}

fn sync_webview_pane_corner_clip(
    settings: Res<AppSettings>,
    mut materials: ResMut<Assets<WebviewExtendStandardMaterial>>,
    tabs: Query<(&WebviewSize, &MeshMaterial3d<WebviewExtendStandardMaterial>), With<Browser>>,
    status: Query<(&WebviewSize, &MeshMaterial3d<WebviewExtendStandardMaterial>), With<StatusBar>>,
) {
    let r = settings.layout.pane.radius;
    for (size, mat_h) in &tabs {
        let w = size.0.x.max(1.0e-6);
        let h = size.0.y.max(1.0e-6);
        if let Some(mat) = materials.get_mut(mat_h.id()) {
            mat.extension.pane_corner_clip = Vec4::new(r, w, h, 0.0);
        }
    }
    for (size, mat_h) in &status {
        let w = size.0.x.max(1.0e-6);
        let h = size.0.y.max(1.0e-6);
        if let Some(mat) = materials.get_mut(mat_h.id()) {
            mat.extension.pane_corner_clip = Vec4::new(r, w, h, 1.0);
        }
    }
}

const STATUS_BAR_HEIGHT_PX: f32 = 40.0;
const WEBVIEW_Z_MAIN: f32 = 0.12;
const WEBVIEW_Z_OUTLINE: f32 = 0.13;
const WEBVIEW_Z_STATUS: f32 = 0.125;
const WEBVIEW_MESH_DEPTH_BIAS: f32 = -4.0;

fn spawn_outline(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut outline_materials: ResMut<Assets<OutlineMaterial>>,
    settings: Res<AppSettings>,
    time: Res<Time>,
) {
    let mat = build_outline_material(800.0, 600.0, &settings, time.elapsed_secs());
    commands.spawn((
        NomadicOutline,
        Mesh3d(meshes.add(Plane3d::new(Vec3::Z, Vec2::splat(0.5)))),
        MeshMaterial3d(outline_materials.add(mat)),
        Transform::default(),
        GlobalTransform::default(),
        Visibility::Hidden,
        InheritedVisibility::VISIBLE,
        ViewVisibility::default(),
    ));
}

fn sync_outline_to_active_pane(
    active_pane: Query<(&ComputedNode, &UiGlobalTransform), With<Active>>,
    glass: Single<(&ComputedNode, &UiGlobalTransform, &Transform), With<DisplayGlass>>,
    settings: Res<AppSettings>,
    time: Res<Time>,
    mut outline_q: Query<
        (
            &mut Transform,
            &MeshMaterial3d<OutlineMaterial>,
            &mut Visibility,
        ),
        (With<NomadicOutline>, Without<DisplayGlass>),
    >,
    mut outline_materials: ResMut<Assets<OutlineMaterial>>,
) {
    let Ok((mut tf, mat_h, mut visibility)) = outline_q.single_mut() else {
        return;
    };
    let Ok((pane_computed, pane_ui_gt)) = active_pane.single() else {
        *visibility = Visibility::Hidden;
        return;
    };
    let &(glass_node, glass_ui_gt, glass_tf) = &*glass;

    let glass_size_px = glass_node.size;
    if glass_size_px.x <= 0.0 || glass_size_px.y <= 0.0 {
        *visibility = Visibility::Hidden;
        return;
    }

    let size_px = pane_computed.size;
    if size_px.x <= 0.0 || size_px.y <= 0.0 {
        *visibility = Visibility::Hidden;
        return;
    }

    let border_px = settings.layout.pane.outline.width.max(0.0);
    if border_px <= 0.0 {
        *visibility = Visibility::Hidden;
        return;
    }

    *visibility = Visibility::Visible;

    let outer_w = size_px.x + 2.0 * border_px;
    let outer_h = size_px.y + 2.0 * border_px;
    let world_sx = glass_tf.scale.x * outer_w / glass_size_px.x;
    let world_sy = glass_tf.scale.y * outer_h / glass_size_px.y;
    tf.scale = Vec3::new(world_sx, world_sy, 1.0);

    let center_ui = pane_ui_gt.transform_point2(Vec2::ZERO);
    let glass_center_ui = glass_ui_gt.transform_point2(Vec2::ZERO);
    let delta_px = center_ui - glass_center_ui;
    let norm_x = delta_px.x / glass_size_px.x;
    let norm_y = -delta_px.y / glass_size_px.y;
    let world_x = glass_tf.translation.x + glass_tf.scale.x * norm_x;
    let world_y = glass_tf.translation.y + glass_tf.scale.y * norm_y;
    tf.translation = Vec3::new(world_x, world_y, WEBVIEW_Z_OUTLINE);

    let inner_logical = size_px * pane_computed.inverse_scale_factor;
    let w_i = inner_logical.x.max(1.0e-6);
    let h_i = inner_logical.y.max(1.0e-6);

    if let Some(m) = outline_materials.get_mut(&mat_h.0) {
        *m = build_outline_material(w_i, h_i, &settings, time.elapsed_secs());
    }
}

fn tick_outline_gradient_time(
    time: Res<Time>,
    mut materials: ResMut<Assets<OutlineMaterial>>,
    outlines: Query<&MeshMaterial3d<OutlineMaterial>, With<NomadicOutline>>,
) {
    let t = time.elapsed_secs();
    for mesh_mat in &outlines {
        let id = mesh_mat.id();
        let Some(m) = materials.get(id) else {
            continue;
        };
        if m.gradient_params.x <= 0.5 {
            continue;
        };
        let Some(m) = materials.get_mut(id) else {
            continue;
        };
        m.gradient_params.w = t;
    }
}

fn build_outline_material(
    w_i: f32,
    h_i: f32,
    settings: &AppSettings,
    time_secs: f32,
) -> OutlineMaterial {
    let b = settings.layout.pane.outline.width.max(0.0);
    let w_o = w_i + 2.0 * b;
    let h_o = h_i + 2.0 * b;
    let m = w_i.min(h_i);
    let r_i = settings.layout.pane.radius.min(m * 0.5).max(0.0);
    let m_o = w_o.min(h_o);
    let r_o = (r_i + b).min(m_o * 0.5);
    let c = &settings.layout.pane.outline.color;
    let border_color = Color::srgb(c.r, c.g, c.b).to_linear().to_vec4();
    let g = &settings.layout.pane.outline.gradient;
    let accent = &g.accent;
    let border_accent = Color::srgb(accent.r, accent.g, accent.b)
        .to_linear()
        .to_vec4();
    let grad_on = if g.enabled { 1.0 } else { 0.0 };
    let gradient_params = Vec4::new(grad_on, g.speed, g.cycles.max(0.01), time_secs);
    let spread = settings.layout.pane.outline.glow.spread.max(0.5);
    let intensity = settings.layout.pane.outline.glow.intensity.max(0.0);
    let glow_on = if intensity > 1.0e-4 { 1.0 } else { 0.0 };
    OutlineMaterial {
        pane_inner: Vec4::new(r_i, w_i, h_i, 0.0),
        pane_outer: Vec4::new(r_o, w_o, h_o, 0.0),
        border_color,
        glow_params: Vec4::new(glow_on, intensity, spread, 0.0),
        gradient_params,
        border_accent,
        alpha_mode: AlphaMode::Blend,
    }
}

fn kick_tab_startup_navigation(
    browsers: NonSend<Browsers>,
    q: Query<(Entity, &WebviewSource), With<Browser>>,
    mut kicked: Local<HashSet<u64>>,
) {
    for (entity, source) in &q {
        let WebviewSource::Url(url) = source else {
            continue;
        };
        let key = entity.to_bits();
        if kicked.contains(&key) {
            continue;
        }
        if !browsers.has_browser(entity) || !browsers.host_emit_ready(&entity) {
            continue;
        }
        browsers.navigate(&entity, url);
        kicked.insert(key);
    }
}

fn sync_osr_webview_focus(
    browsers: NonSend<Browsers>,
    webviews: Query<Entity, With<WebviewSource>>,
    keyboard_target: Query<Entity, (With<WebviewSource>, With<CefKeyboardTarget>)>,
    status_chrome: Query<Entity, (With<StatusBar>, With<Browser>)>,
    mut ready: Local<Vec<Entity>>,
    mut auxiliary: Local<Vec<Entity>>,
) {
    ready.clear();
    ready.extend(webviews.iter().filter(|&e| browsers.has_browser(e)));
    if ready.is_empty() {
        return;
    }
    ready.sort_by_key(|e| e.to_bits());

    let active = keyboard_target
        .iter()
        .filter(|&k| ready.iter().any(|&e| e == k))
        .min_by_key(|e| e.to_bits())
        .unwrap_or(ready[0]);

    auxiliary.clear();
    auxiliary.extend(ready.iter().copied().filter(|&e| e != active));
    browsers.sync_osr_focus_to_active_pane(Some(active), auxiliary.as_slice());
    for e in status_chrome.iter() {
        browsers.set_osr_not_hidden(&e);
    }
}

fn apply_chrome_state_from_cef(
    chrome_rx: Res<WebviewChromeStateReceiver>,
    mut browser_meta: Query<&mut PageMetadata, With<Browser>>,
) {
    while let Ok(ev) = chrome_rx.0.try_recv() {
        let Ok(mut meta) = browser_meta.get_mut(ev.webview) else {
            continue;
        };
        if let Some(url) = ev.url {
            meta.url = url;
            meta.favicon_url.clear();
        }
        if let Some(title) = ev.title {
            meta.title = title;
        }
        if let Some(favicon) = ev.favicon_url {
            meta.favicon_url = favicon;
        }
    }
}

fn push_tabs_host_emit(
    mut commands: Commands,
    browsers: NonSend<Browsers>,
    status: Single<Entity, (With<StatusBar>, With<UiReady>)>,
    browser_q: Query<(&PageMetadata, &ChildOf), With<Browser>>,
    active_pane: Query<(), With<Active>>,
    mut last: Local<String>,
) {
    let status_e = *status;
    if !browsers.has_browser(status_e) || !browsers.host_emit_ready(&status_e) {
        return;
    }
    let mut rows: Vec<TabRow> = Vec::new();
    for (meta, child_of) in &browser_q {
        if !active_pane.contains(child_of.get()) {
            continue;
        }
        rows.push(TabRow {
            title: meta.title.clone(),
            url: meta.url.clone(),
            favicon_url: meta.favicon_url.clone(),
            is_active: true,
        });
    }
    let payload = TabsHostEvent { tabs: rows };
    let ron_body = ron::ser::to_string(&payload).unwrap_or_default();
    if ron_body.as_str() == last.as_str() {
        return;
    }
    commands.trigger(HostEmitEvent::new(status_e, TABS_EVENT, &ron_body));
    *last = ron_body;
}

fn flush_pending_osr_textures(
    mut ew: MessageWriter<RenderTextureMessage>,
    browsers: NonSend<Browsers>,
) {
    while let Ok(texture) = browsers.try_receive_texture() {
        ew.write(texture);
    }
}

fn cef_root_cache_path() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME").map(|home| {
            PathBuf::from(home)
                .join("Library/Application Support/vmux")
                .to_string_lossy()
                .into_owned()
        })
    }
    #[cfg(not(target_os = "macos"))]
    {
        std::env::temp_dir()
            .to_str()
            .map(|p| format!("{p}/vmux_cef"))
    }
}
