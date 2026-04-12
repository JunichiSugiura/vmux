use crate::{
    layout::{
        display::{DisplayGlass, WEBVIEW_Z_OUTLINE},
        pane::Active,
    },
    settings::{AppSettings, load_settings},
};
use bevy::{
    asset::*,
    pbr::MaterialPlugin,
    prelude::*,
    render::alpha::AlphaMode,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
    ui::UiGlobalTransform,
};

const OUTLINE_SHADER: Handle<Shader> = uuid_handle!("c4a8e901-2b7d-4c1e-9f63-7a2d8e5b1044");

pub(crate) struct OutlinePlugin;

impl Plugin for OutlinePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<OutlineMaterial>::default())
            .add_systems(
                Startup,
                spawn_outline
                    .after(load_settings)
                    .after(crate::scene::setup),
            )
            .add_systems(Update, tick_outline_gradient_time)
            .add_systems(
                PostUpdate,
                sync_outline_to_active_pane.after(bevy::ui::UiSystems::Layout),
            );
        load_internal_asset!(app, OUTLINE_SHADER, "outline.wgsl", Shader::from_wgsl);
    }
}

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

    let pad = glass_node.padding;
    let glass_size_px = glass_node.size + pad.min_inset + pad.max_inset;
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
