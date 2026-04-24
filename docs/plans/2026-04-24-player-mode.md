# Player Mode Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the free camera toggle with a two-mode system (User Mode / Player Mode) featuring smooth camera transitions, bloom/light fading, click-to-focus panes, and double-click to exit.

**Architecture:** A new `InteractionMode` resource replaces `FreeCameraActive`. A `ModeTransition` resource drives animated transitions. Bevy's `AnimationPlayer` handles camera Transform easing on exit; a manual lerp system handles Bloom intensity and DirectionalLight illuminance fading. Double-click detection is added to the pane click handler.

**Tech Stack:** Bevy 0.18 (AnimationPlayer, EasingCurve, EaseFunction), existing FreeCameraPlugin from bevy_camera_controller.

**Spec:** `docs/specs/2026-04-24-player-mode-design.md`

---

### Task 1: Rename ToggleFreeCamera to TogglePlayerMode

**Files:**
- Modify: `crates/vmux_desktop/src/command.rs:242-248`
- Modify: `crates/vmux_desktop/src/scene.rs` (all `ToggleFreeCamera` references)
- Modify: `docs/specs/2025-07-13-commands-and-keybindings.md:122`

- [ ] **Step 1: Rename the command enum variant**

In `crates/vmux_desktop/src/command.rs`, change:

```rust
#[derive(OsSubMenu, DefaultShortcuts, CommandBar, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SceneCommand {
    #[default]
    #[menu(id = "toggle_player_mode", label = "Toggle Player Mode")]
    #[shortcut(chord = "Ctrl+g, Enter")]
    TogglePlayerMode,
}
```

- [ ] **Step 2: Update the match in scene.rs**

In `crates/vmux_desktop/src/scene.rs`, in function `on_toggle_free_camera`, change:

```rust
let AppCommand::Scene(SceneCommand::ToggleFreeCamera) = *cmd else {
```

to:

```rust
let AppCommand::Scene(SceneCommand::TogglePlayerMode) = *cmd else {
```

- [ ] **Step 3: Update keybindings spec**

In `docs/specs/2025-07-13-commands-and-keybindings.md`, change line 122:

```
| TogglePlayerMode | toggle_player_mode | Toggle Player Mode | | Toggle Player Mode | | ✅ |
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check 2>&1 | tail -5`
Expected: Compiles with no new errors.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "refactor: rename ToggleFreeCamera to TogglePlayerMode"
```

---

### Task 2: Replace FreeCameraActive with InteractionMode

**Files:**
- Modify: `crates/vmux_desktop/src/scene.rs`
- Modify: `crates/vmux_desktop/src/browser.rs:166,180`
- Modify: `crates/vmux_desktop/src/layout/pane.rs:503,512,578-579,588,623`

- [ ] **Step 1: Define InteractionMode and CameraHome in scene.rs**

Replace the `FreeCameraActive` struct with:

```rust
#[derive(Resource, Default, PartialEq, Eq, Clone, Copy)]
pub(crate) enum InteractionMode {
    #[default]
    User,
    Player,
}
```

Add below it:

```rust
#[derive(Resource)]
pub(crate) struct CameraHome(pub Transform);
```

- [ ] **Step 2: Update ScenePlugin registration**

Change `.init_resource::<FreeCameraActive>()` to `.init_resource::<InteractionMode>()`.

- [ ] **Step 3: Update on_toggle_free_camera to use InteractionMode**

Change the parameter `mut free_cam_active: ResMut<FreeCameraActive>` to `mut mode: ResMut<InteractionMode>`.

Change the toggle logic:

```rust
// Before:
state.enabled = !state.enabled;
free_cam_active.0 = state.enabled;
if state.enabled {
// After:
let entering = *mode == InteractionMode::User;
if entering {
    *mode = InteractionMode::Player;
    state.enabled = true;
```

And change the `else` to:

```rust
} else {
    *mode = InteractionMode::User;
    state.enabled = false;
```

- [ ] **Step 4: Update suppress_free_camera_when_pane_active**

Change `free_cam: Res<FreeCameraActive>` to `mode: Res<InteractionMode>`.

Replace `if !free_cam.0 {` with `if *mode != InteractionMode::Player {`.

- [ ] **Step 5: Update browser.rs sync_keyboard_target**

In `crates/vmux_desktop/src/browser.rs:164-180`, change:

```rust
free_cam: Res<crate::scene::FreeCameraActive>,
```
to:
```rust
mode: Res<crate::scene::InteractionMode>,
```

Change:
```rust
if crate::command_bar::is_command_bar_open(&modal_q) || free_cam.0 {
```
to:
```rust
if crate::command_bar::is_command_bar_open(&modal_q) || *mode != crate::scene::InteractionMode::User {
```

- [ ] **Step 6: Update pane.rs poll_cursor_pane_focus**

In `crates/vmux_desktop/src/layout/pane.rs`, change:

```rust
free_cam: Res<crate::scene::FreeCameraActive>,
```
to:
```rust
mode: Res<crate::scene::InteractionMode>,
```

Change `if free_cam.0 {` to `if *mode != crate::scene::InteractionMode::User {`.

- [ ] **Step 7: Update pane.rs click_pane_in_free_camera**

Rename function to `click_pane_in_player_mode`.

Change parameter:
```rust
mut free_cam: ResMut<crate::scene::FreeCameraActive>,
```
to:
```rust
mut mode: ResMut<crate::scene::InteractionMode>,
```

Change `if !free_cam.0 {` to `if *mode != crate::scene::InteractionMode::Player {`.

Change `free_cam.0 = false;` to `*mode = crate::scene::InteractionMode::User;`.

- [ ] **Step 8: Update system registration in PanePlugin**

Change `.add_systems(Update, click_pane_in_free_camera)` to `.add_systems(Update, click_pane_in_player_mode)`.

- [ ] **Step 9: Verify compilation**

Run: `cargo check 2>&1 | tail -5`
Expected: Compiles with no new errors.

- [ ] **Step 10: Commit**

```bash
git add -A && git commit -m "refactor: replace FreeCameraActive with InteractionMode enum"
```

---

### Task 3: Add ModeTransition resource and tick system

**Files:**
- Modify: `crates/vmux_desktop/src/scene.rs`

- [ ] **Step 1: Define ModeTransition types**

Add to `crates/vmux_desktop/src/scene.rs`, after the `CameraHome` definition:

```rust
const TRANSITION_DURATION: f32 = 0.3;

#[derive(Resource)]
pub(crate) struct ModeTransition {
    pub direction: TransitionDirection,
    pub timer: Timer,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum TransitionDirection {
    EnterPlayer,
    ExitPlayer,
}

impl ModeTransition {
    fn new(direction: TransitionDirection) -> Self {
        Self {
            direction,
            timer: Timer::from_seconds(TRANSITION_DURATION, TimerMode::Once),
        }
    }

    pub fn progress(&self) -> f32 {
        self.timer.fraction()
    }
}
```

- [ ] **Step 2: Add tick_mode_transition system**

```rust
fn tick_mode_transition(
    time: Res<Time>,
    transition: Option<ResMut<ModeTransition>>,
) {
    if let Some(mut t) = transition {
        t.timer.tick(time.delta());
    }
}
```

- [ ] **Step 3: Register the system**

In `ScenePlugin::build`, add to the Update systems tuple:

```rust
tick_mode_transition,
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check 2>&1 | tail -5`
Expected: Compiles with no new errors.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat: add ModeTransition resource with timer"
```

---

### Task 4: Add bloom and light fade system

**Files:**
- Modify: `crates/vmux_desktop/src/scene.rs`

- [ ] **Step 1: Add the bloom/light interpolation constants**

Add near the top of `scene.rs`:

```rust
const BLOOM_INTENSITY: f32 = 0.15; // Bloom::NATURAL intensity
const SUNLIGHT_ILLUMINANCE: f32 = 8000.0;
```

- [ ] **Step 2: Add fade_bloom_and_light system**

```rust
fn fade_bloom_and_light(
    transition: Option<Res<ModeTransition>>,
    mut bloom_q: Query<&mut Bloom, With<MainCamera>>,
    mut light_q: Query<&mut DirectionalLight, With<SceneSunlight>>,
) {
    let Some(transition) = transition else { return };
    let t = EaseFunction::CubicInOut.sample(transition.progress());

    let (bloom_target, light_target) = match transition.direction {
        TransitionDirection::EnterPlayer => (t, t),
        TransitionDirection::ExitPlayer => (1.0 - t, 1.0 - t),
    };

    if let Ok(mut bloom) = bloom_q.single_mut() {
        bloom.intensity = BLOOM_INTENSITY * bloom_target;
    }
    if let Ok(mut light) = light_q.single_mut() {
        light.illuminance = SUNLIGHT_ILLUMINANCE * light_target;
    }
}
```

- [ ] **Step 3: Add necessary import**

Add to the bevy imports block:

```rust
use bevy::math::curve::easing::EaseFunction;
```

- [ ] **Step 4: Register the system**

Add `fade_bloom_and_light` to the Update systems tuple in `ScenePlugin::build`.

- [ ] **Step 5: Verify compilation**

Run: `cargo check 2>&1 | tail -5`
Expected: Compiles with no new errors.

- [ ] **Step 6: Commit**

```bash
git add -A && git commit -m "feat: add bloom and sunlight fade system"
```

---

### Task 5: Rewrite entry transition (User -> Player)

**Files:**
- Modify: `crates/vmux_desktop/src/scene.rs`

- [ ] **Step 1: Rewrite the entering branch of on_toggle_free_camera (now on_toggle_player_mode)**

Rename the function to `on_toggle_player_mode`. Replace the entire function body:

```rust
fn on_toggle_player_mode(
    mut reader: MessageReader<AppCommand>,
    window: Single<&Window, With<PrimaryWindow>>,
    settings: Res<AppSettings>,
    mut state: Single<&mut FreeCameraState, With<MainCamera>>,
    camera: Single<Entity, With<MainCamera>>,
    kb_targets: Query<Entity, With<CefKeyboardTarget>>,
    mut mode: ResMut<InteractionMode>,
    transition: Option<Res<ModeTransition>>,
    mut commands: Commands,
) {
    for cmd in reader.read() {
        let AppCommand::Scene(SceneCommand::TogglePlayerMode) = *cmd else {
            continue;
        };

        // Ignore command during active transition
        if transition.is_some() {
            continue;
        }

        match *mode {
            InteractionMode::User => {
                // Store home transform
                let home = frame_main_camera_transform(
                    &window,
                    window.aspect(),
                    camera_margin_px(&settings),
                );
                commands.insert_resource(CameraHome(home));

                *mode = InteractionMode::Player;

                // Remove keyboard targets so free camera keys work
                for e in &kb_targets {
                    commands.entity(e).remove::<CefKeyboardTarget>();
                }

                // Spawn bloom with 0 intensity (fade system will animate it)
                let mut bloom = Bloom::NATURAL;
                bloom.intensity = 0.0;
                commands.entity(*camera).insert(bloom);

                // Spawn sunlight with 0 illuminance
                commands.spawn((
                    SceneSunlight,
                    DirectionalLight {
                        illuminance: 0.0,
                        shadows_enabled: false,
                        color: Color::srgb(1.0, 0.98, 0.95),
                        ..default()
                    },
                    Transform::from_rotation(Quat::from_euler(
                        EulerRot::XYZ,
                        -0.6,
                        0.4,
                        0.0,
                    )),
                ));

                // Start transition timer (fade system reads this)
                commands.insert_resource(ModeTransition::new(TransitionDirection::EnterPlayer));

                // NOTE: FreeCameraState.enabled is NOT set here.
                // It will be enabled by complete_mode_transition once the fade finishes.
            }
            InteractionMode::Player => {
                // Start exit transition
                start_exit_transition(
                    &mut state,
                    &mut commands,
                    *camera,
                );
            }
        }
    }
}
```

- [ ] **Step 2: Add start_exit_transition helper**

```rust
fn start_exit_transition(
    state: &mut FreeCameraState,
    commands: &mut Commands,
    camera_entity: Entity,
) {
    // Disable free camera immediately so WASD stops during transition
    state.enabled = false;

    // Start the exit transition timer
    commands.insert_resource(ModeTransition::new(TransitionDirection::ExitPlayer));

    // Camera animation is set up by setup_exit_camera_animation system
    // which reads ModeTransition and creates the AnimationPlayer
}
```

- [ ] **Step 3: Update system registration**

Change `on_toggle_free_camera.in_set(ReadAppCommands)` to `on_toggle_player_mode.in_set(ReadAppCommands)`.

- [ ] **Step 4: Verify compilation**

Run: `cargo check 2>&1 | tail -5`
Expected: Compiles with no new errors (unused parameters from old function removed).

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat: rewrite entry transition for player mode"
```

---

### Task 6: Add camera exit animation with AnimationPlayer

**Files:**
- Modify: `crates/vmux_desktop/src/scene.rs`

- [ ] **Step 1: Add animation imports**

Add to the bevy imports block in `scene.rs`:

```rust
use bevy::animation::prelude::*;
use bevy::math::curve::easing::EasingCurve;
```

- [ ] **Step 2: Add setup_exit_camera_animation system**

This system runs once when `ModeTransition(ExitPlayer)` is inserted. It creates the AnimationClip and starts playback.

```rust
fn setup_exit_camera_animation(
    transition: Option<Res<ModeTransition>>,
    home: Option<Res<CameraHome>>,
    camera_transform: Single<&Transform, With<MainCamera>>,
    camera_entity: Single<Entity, With<MainCamera>>,
    mut clips: ResMut<Assets<AnimationClip>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    mut commands: Commands,
    mut setup_done: Local<bool>,
) {
    let Some(ref transition) = transition else {
        *setup_done = false;
        return;
    };
    if transition.direction != TransitionDirection::ExitPlayer {
        *setup_done = false;
        return;
    }
    if *setup_done {
        return;
    }
    *setup_done = true;

    let Some(ref home) = home else { return };

    let target_id = AnimationTargetId::from_name(&Name::new("main_camera"));

    let mut clip = AnimationClip::default();

    // Translation curve: current -> home, eased
    let translation_curve = EasingCurve::new(
        camera_transform.translation,
        home.0.translation,
        EaseFunction::CubicInOut,
    );
    clip.add_curve_to_target(
        target_id,
        AnimatableCurve::new(
            animated_field!(Transform::translation),
            translation_curve,
        ),
    );

    // Rotation curve: current -> home, eased
    let rotation_curve = EasingCurve::new(
        camera_transform.rotation,
        home.0.rotation,
        EaseFunction::CubicInOut,
    );
    clip.add_curve_to_target(
        target_id,
        AnimatableCurve::new(
            animated_field!(Transform::rotation),
            rotation_curve,
        ),
    );

    let clip_handle = clips.add(clip);
    let (graph, node_index) = AnimationGraph::from_clip(clip_handle);
    let graph_handle = graphs.add(graph);

    // Add animation components to camera
    commands.entity(*camera_entity).insert((
        Name::new("main_camera"),
        AnimationGraphHandle(graph_handle),
        AnimationPlayer::default(),
    ));

    // We need to start playback in the next frame after components are added
    // Store the node index for the playback starter system
    commands.insert_resource(PendingAnimationStart(node_index));
}

#[derive(Resource)]
struct PendingAnimationStart(AnimationNodeIndex);
```

- [ ] **Step 3: Add start_pending_animation system**

This runs the frame after the AnimationPlayer is added:

```rust
fn start_pending_animation(
    pending: Option<Res<PendingAnimationStart>>,
    mut player_q: Query<&mut AnimationPlayer, With<MainCamera>>,
    mut commands: Commands,
) {
    let Some(pending) = pending else { return };
    let Ok(mut player) = player_q.single_mut() else { return };

    player
        .start(pending.0)
        .set_speed(1.0 / TRANSITION_DURATION);

    commands.remove_resource::<PendingAnimationStart>();
}
```

- [ ] **Step 4: Register both systems**

Add to the Update systems in `ScenePlugin::build`:

```rust
setup_exit_camera_animation,
start_pending_animation,
```

- [ ] **Step 5: Verify compilation**

Run: `cargo check 2>&1 | tail -5`
Expected: Compiles with no new errors.

- [ ] **Step 6: Commit**

```bash
git add -A && git commit -m "feat: add camera exit animation with AnimationPlayer"
```

---

### Task 7: Add transition completion system

**Files:**
- Modify: `crates/vmux_desktop/src/scene.rs`

- [ ] **Step 1: Add complete_mode_transition system**

```rust
fn complete_mode_transition(
    transition: Option<Res<ModeTransition>>,
    mut state: Single<&mut FreeCameraState, With<MainCamera>>,
    camera: Single<Entity, With<MainCamera>>,
    sunlight_q: Query<Entity, With<SceneSunlight>>,
    mut mode: ResMut<InteractionMode>,
    home: Option<Res<CameraHome>>,
    mut transform: Single<&mut Transform, With<MainCamera>>,
    mut commands: Commands,
) {
    let Some(ref transition) = transition else { return };
    if !transition.timer.finished() {
        return;
    }

    match transition.direction {
        TransitionDirection::EnterPlayer => {
            // Fade-in done, enable free camera movement
            state.enabled = true;
        }
        TransitionDirection::ExitPlayer => {
            // Animation done, clean up
            *mode = InteractionMode::User;

            // Remove bloom
            commands.entity(*camera).remove::<Bloom>();

            // Despawn sunlight
            for e in &sunlight_q {
                commands.entity(e).despawn();
            }

            // Snap to exact home transform
            if let Some(ref home) = home {
                **transform = home.0;
            }

            // Remove animation components
            commands.entity(*camera)
                .remove::<AnimationPlayer>()
                .remove::<AnimationGraphHandle>()
                .remove::<Name>();

            commands.remove_resource::<CameraHome>();
        }
    }

    commands.remove_resource::<ModeTransition>();
}
```

- [ ] **Step 2: Register the system**

Add `complete_mode_transition` to the Update systems tuple in `ScenePlugin::build`.

- [ ] **Step 3: Verify compilation**

Run: `cargo check 2>&1 | tail -5`
Expected: Compiles with no new errors.

- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "feat: add transition completion system"
```

---

### Task 8: Update suppress system and fit_main_camera guards

**Files:**
- Modify: `crates/vmux_desktop/src/scene.rs`

- [ ] **Step 1: Simplify suppress_free_camera_when_pane_active**

The suppress system no longer needs to do full cleanup on exit (the transition system handles that). Simplify it to only toggle `FreeCameraState.enabled` and `CefSuppressKeyboardInput` based on keyboard targets:

```rust
fn suppress_free_camera_when_pane_active(
    mode: Res<InteractionMode>,
    transition: Option<Res<ModeTransition>>,
    kb_targets: Query<(), With<CefKeyboardTarget>>,
    mut state: Single<&mut FreeCameraState, With<MainCamera>>,
    mut suppress: ResMut<bevy_cef::prelude::CefSuppressKeyboardInput>,
) {
    // Only applies in Player mode with no active transition
    if *mode != InteractionMode::Player || transition.is_some() {
        return;
    }

    let no_target = kb_targets.is_empty();
    state.enabled = no_target;
    suppress.0 = no_target;
}
```

- [ ] **Step 2: Add transition guard to fit_main_camera**

Add `transition: Option<Res<ModeTransition>>` parameter and skip when transitioning:

```rust
fn fit_main_camera(
    window: Single<&Window, With<PrimaryWindow>>,
    settings: Res<AppSettings>,
    mut camera_q: Query<(&mut Transform, &mut Projection), With<MainCamera>>,
    camera_state: Single<&FreeCameraState, With<MainCamera>>,
    mode: Res<InteractionMode>,
    transition: Option<Res<ModeTransition>>,
) {
    let Ok((mut tf, mut proj)) = camera_q.single_mut() else {
        return;
    };
    let aspect = window.aspect();

    if let Projection::Perspective(ref mut p) = *proj {
        if (p.aspect_ratio - aspect).abs() > f32::EPSILON {
            p.aspect_ratio = aspect;
        }
    }

    // Skip transform update during transitions or when camera is user-controlled
    if transition.is_some() || camera_state.enabled {
        return;
    }

    // Only reset transform in User mode
    if *mode == InteractionMode::User {
        *tf = frame_main_camera_transform(&window, aspect, camera_margin_px(&settings));
    }
}
```

- [ ] **Step 3: Update CameraHome on window resize in User mode**

Add a system to keep `CameraHome` up-to-date (only relevant if we want to preserve it across resize while in Player mode):

```rust
fn update_camera_home(
    window: Single<&Window, With<PrimaryWindow>>,
    settings: Res<AppSettings>,
    mode: Res<InteractionMode>,
    home: Option<ResMut<CameraHome>>,
) {
    if *mode != InteractionMode::Player {
        return;
    }
    let Some(mut home) = home else { return };
    home.0 = frame_main_camera_transform(&window, window.aspect(), camera_margin_px(&settings));
}
```

Register in PostUpdate:
```rust
.add_systems(PostUpdate, update_camera_home.after(fit_window_to_screen))
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check 2>&1 | tail -5`
Expected: Compiles with no new errors.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat: update suppress system and fit_main_camera guards for transitions"
```

---

### Task 9: Add double-click detection to pane click handler

**Files:**
- Modify: `crates/vmux_desktop/src/layout/pane.rs`

- [ ] **Step 1: Rewrite click_pane_in_player_mode with double-click detection**

Replace the entire function:

```rust
fn click_pane_in_player_mode(
    mut mode: ResMut<crate::scene::InteractionMode>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    leaf_panes: Query<(Entity, &ComputedNode, &UiGlobalTransform), (With<Pane>, Without<PaneSplit>)>,
    kb_targets: Query<Entity, With<CefKeyboardTarget>>,
    mut commands: Commands,
    accumulated_motion: Res<AccumulatedMouseMotion>,
    mut press_motion: Local<Option<f32>>,
    mut last_click: Local<Option<(Entity, Instant)>>,
    transition: Option<Res<crate::scene::ModeTransition>>,
) {
    if *mode != crate::scene::InteractionMode::Player {
        *press_motion = None;
        *last_click = None;
        return;
    }

    // Don't handle clicks during transition
    if transition.is_some() {
        *press_motion = None;
        return;
    }

    let Ok(window) = windows.single() else { return };
    let Some(cursor_pos) = window.physical_cursor_position() else { return };
    let cursor = Vec2::new(cursor_pos.x, cursor_pos.y);

    if mouse.just_pressed(MouseButton::Left) {
        *press_motion = Some(0.0);
        return;
    }

    if let Some(ref mut total) = *press_motion {
        *total += accumulated_motion.delta.length();
    }

    if !mouse.just_released(MouseButton::Left) {
        return;
    }
    let Some(total_motion) = press_motion.take() else { return };
    const DRAG_THRESHOLD: f32 = 2.0;
    if total_motion > DRAG_THRESHOLD {
        return;
    }

    // Hit-test panes
    let mut hit_pane: Option<Entity> = None;
    for (entity, node, ui_gt) in &leaf_panes {
        let center = ui_gt.transform_point2(Vec2::ZERO);
        let half = node.size * 0.5;
        if cursor.x >= center.x - half.x
            && cursor.x <= center.x + half.x
            && cursor.y >= center.y - half.y
            && cursor.y <= center.y + half.y
        {
            hit_pane = Some(entity);
            break;
        }
    }

    if let Some(pane) = hit_pane {
        // Check for double-click
        const DOUBLE_CLICK_MS: u128 = 400;
        if let Some((prev_entity, prev_time)) = *last_click {
            if prev_entity == pane && prev_time.elapsed().as_millis() < DOUBLE_CLICK_MS {
                // Double-click: exit player mode
                *last_click = None;
                *mode = crate::scene::InteractionMode::User;
                // The suppress system and complete_mode_transition handle cleanup
                // But we need to trigger exit transition via a different path.
                // Set mode to User so suppress system deactivates,
                // but the animated exit is handled by ModeTransition.
                // Actually, we should NOT set mode here -- we need to start
                // the exit transition instead.
                // Revert mode, and dispatch the exit:
                *mode = crate::scene::InteractionMode::Player;
                commands.insert_resource(
                    crate::scene::ModeTransition::new(crate::scene::TransitionDirection::ExitPlayer),
                );
                return;
            }
        }

        // Single click: activate pane
        *last_click = Some((pane, Instant::now()));
        commands.entity(pane).insert(LastActivatedAt::now());
        // CefKeyboardTarget will be assigned by sync_keyboard_target in browser.rs
        // But sync_keyboard_target skips when mode != User.
        // We need to manually assign keyboard target to the pane's browser.
        // However, we don't have access to the browser child here.
        // Instead, remove the guard in sync_keyboard_target for Player/Focused.
        // This is handled in Task 10.
    } else {
        // Clicked empty space: remove all keyboard targets (return to roaming)
        *last_click = None;
        for e in &kb_targets {
            commands.entity(e).remove::<CefKeyboardTarget>();
        }
    }
}
```

- [ ] **Step 2: Make ModeTransition::new and TransitionDirection pub(crate)**

Verify that `ModeTransition::new` and `TransitionDirection` are `pub(crate)` in `scene.rs` (they should be from Task 3).

- [ ] **Step 3: Verify compilation**

Run: `cargo check 2>&1 | tail -5`
Expected: Compiles with no new errors.

- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "feat: add double-click detection for player mode exit"
```

---

### Task 10: Update sync_keyboard_target for Player/Focused

**Files:**
- Modify: `crates/vmux_desktop/src/browser.rs`

- [ ] **Step 1: Allow sync_keyboard_target to run in Player mode**

The current guard blocks all keyboard target sync when not in User mode. But in Player/Focused, we need `sync_keyboard_target` to assign `CefKeyboardTarget` to the active pane's browser.

Change the guard in `crates/vmux_desktop/src/browser.rs`:

```rust
// Before:
if crate::command_bar::is_command_bar_open(&modal_q) || *mode != crate::scene::InteractionMode::User {
    return;
}

// After:
if crate::command_bar::is_command_bar_open(&modal_q) {
    return;
}
// In Player mode, only sync when there's an active pane (Focused sub-state).
// In Roaming (no LastActivatedAt recently set), skip sync.
if *mode == crate::scene::InteractionMode::Player {
    // Check if any pane was just activated (has a recent LastActivatedAt)
    // sync_keyboard_target already finds the focused tab via focused_tab(),
    // and the existing logic will assign CefKeyboardTarget correctly.
    // We just need to let it run.
}
```

Actually, the simpler approach: remove the `InteractionMode` guard entirely from `sync_keyboard_target`. The suppress system already handles the interplay -- when there are no keyboard targets (Roaming), `suppress_free_camera_when_pane_active` enables the free camera. When a pane gets activated and `sync_keyboard_target` assigns `CefKeyboardTarget`, the suppress system disables free camera movement.

```rust
fn sync_keyboard_target(
    mode: Res<crate::scene::InteractionMode>,
    // ...rest unchanged...
) {
    if crate::command_bar::is_command_bar_open(&modal_q) {
        return;
    }
    // In Player/Roaming, no pane has LastActivatedAt set recently after entry,
    // so sync won't assign any CefKeyboardTarget. That's correct.
    // When user clicks a pane, LastActivatedAt is set, and sync will assign
    // CefKeyboardTarget to that pane's browser. The suppress system then
    // disables FreeCameraState. This is the Focused sub-state.
```

Wait -- this won't work because `sync_keyboard_target` always finds a focused tab (based on `LastActivatedAt` across all panes). On entering player mode, the previously active pane still has the most recent `LastActivatedAt`, so sync would immediately re-assign `CefKeyboardTarget`.

The fix: on entering Player mode (`on_toggle_player_mode`), we already remove `CefKeyboardTarget`. But sync runs every frame and would re-add it. We need to either:
(a) Clear `LastActivatedAt` on all panes when entering player mode, or
(b) Keep the guard but make it smarter.

Option (b): Keep the guard but only block during Roaming:

```rust
if crate::command_bar::is_command_bar_open(&modal_q) {
    return;
}

// In Player mode during Roaming (no CefKeyboardTarget), don't sync
// This prevents re-assigning targets to the previously active pane
if *mode == crate::scene::InteractionMode::Player {
    let transition = transition.as_ref();
    if transition.is_some() {
        return; // Don't sync during transitions
    }
    // In Focused state (some pane was clicked), let sync run
    // But we need to know if we're in Focused... check if any non-header,
    // non-side-sheet browser has CefKeyboardTarget
    let has_pane_target = content_q.iter().any(|(e, has_kb)| {
        has_kb && !status_q.contains(e) && !side_sheet_q.contains(e)
    });
    if !has_pane_target {
        return; // Roaming: don't assign targets
    }
}
```

Add `transition: Option<Res<crate::scene::ModeTransition>>` parameter.

- [ ] **Step 2: Verify compilation**

Run: `cargo check 2>&1 | tail -5`
Expected: Compiles with no new errors.

- [ ] **Step 3: Commit**

```bash
git add -A && git commit -m "feat: allow sync_keyboard_target in Player/Focused state"
```

---

### Task 11: Handle FreeCameraState disable on double-click exit

**Files:**
- Modify: `crates/vmux_desktop/src/scene.rs`
- Modify: `crates/vmux_desktop/src/layout/pane.rs`

- [ ] **Step 1: Disable FreeCameraState when exit transition starts from pane double-click**

In `click_pane_in_player_mode`, the double-click path inserts `ModeTransition(ExitPlayer)` but doesn't disable `FreeCameraState`. Add a query for it:

Add parameter to `click_pane_in_player_mode`:
```rust
mut camera_state: Single<&mut FreeCameraState, With<crate::scene::MainCamera>>,
```

Add import at top of pane.rs:
```rust
use bevy::camera_controller::free_camera::FreeCameraState;
```

In the double-click branch, before inserting `ModeTransition`:
```rust
camera_state.enabled = false;
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check 2>&1 | tail -5`
Expected: Compiles with no new errors.

- [ ] **Step 3: Commit**

```bash
git add -A && git commit -m "feat: disable FreeCameraState on double-click exit"
```

---

### Task 12: Wire up CefSuppressKeyboardInput on mode transitions

**Files:**
- Modify: `crates/vmux_desktop/src/scene.rs`

- [ ] **Step 1: Set suppress on entry**

In `on_toggle_player_mode`, in the `InteractionMode::User` (entering) branch, add after removing CefKeyboardTargets:

```rust
// Suppress keyboard input during transition and Roaming
// (will be managed by suppress_free_camera_when_pane_active after transition)
```

The suppress system already handles this: when `mode == Player` and `kb_targets.is_empty()`, it sets `suppress.0 = true`. But during the entry transition, `transition.is_some()` causes the suppress system to return early. We need to set suppress immediately.

Add parameter to `on_toggle_player_mode`:
```rust
mut suppress: ResMut<bevy_cef::prelude::CefSuppressKeyboardInput>,
```

In the entering branch:
```rust
suppress.0 = true;
```

In the exiting branch (inside `start_exit_transition` or after calling it):
```rust
suppress.0 = false;
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check 2>&1 | tail -5`
Expected: Compiles with no new errors.

- [ ] **Step 3: Commit**

```bash
git add -A && git commit -m "feat: wire up CefSuppressKeyboardInput during mode transitions"
```

---

### Task 13: Manual verification

- [ ] **Step 1: Build and run**

Run: `cargo run 2>&1`

- [ ] **Step 2: Test entry transition**

1. Press `Ctrl+G, Enter` to enter Player Mode
2. Verify bloom and sunlight fade in smoothly over ~300ms
3. Verify camera stays at framing position
4. After fade completes, verify WASD moves the camera
5. Verify mouse left-click-drag rotates the camera

- [ ] **Step 3: Test pane click focus**

1. While in Player Mode, single-click on a pane
2. Verify camera stays in place
3. Verify you can type into the pane (keyboard input reaches the browser/terminal)
4. Verify WASD no longer moves the camera

- [ ] **Step 4: Test return to roaming**

1. Click on empty space (outside any pane)
2. Verify keyboard target is removed (no pane has focus)
3. Verify WASD works again

- [ ] **Step 5: Test drag does not activate pane**

1. In Roaming mode, left-click-drag to rotate camera
2. Release on top of a pane
3. Verify the pane did NOT get activated

- [ ] **Step 6: Test double-click exit**

1. In Player Mode (Roaming or Focused), double-click on a pane
2. Verify camera smoothly transitions back to the framing position over ~300ms
3. Verify bloom and sunlight fade out during the transition
4. After transition completes, verify hover-to-activate works
5. Verify you're back in normal User Mode

- [ ] **Step 7: Test TogglePlayerMode exit**

1. Enter Player Mode, move camera around
2. Press `Ctrl+G, Enter` again
3. Verify smooth camera return + bloom fade out
4. Verify return to User Mode

- [ ] **Step 8: Test transition interruption**

1. Enter Player Mode
2. During the 300ms fade-in, press `Ctrl+G, Enter` again
3. Verify the command is ignored (no crash, no double-transition)

- [ ] **Step 9: Commit final state**

```bash
git add -A && git commit -m "feat: player mode with animated transitions, click-to-focus, double-click exit"
```
