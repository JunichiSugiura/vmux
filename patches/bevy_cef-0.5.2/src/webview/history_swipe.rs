use crate::common::HistorySwipeVisualOffset;
use bevy::input::mouse::MouseScrollUnit;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use std::time::{Duration, Instant};

const HISTORY_SWIPE_THRESHOLD_PX: f32 = 200.0;
const HISTORY_SWIPE_MAX_VERTICAL_RATIO: f32 = 0.65;
const HISTORY_SWIPE_TRACK_OFFSET_PX: f32 = 28.0;
const HISTORY_SWIPE_NAVIGATE_OFFSET_PX: f32 = 36.0;
const HISTORY_SWIPE_IDLE_RESET: Duration = Duration::from_millis(220);
const HISTORY_SWIPE_COOLDOWN: Duration = Duration::from_millis(650);
const HISTORY_SWIPE_VISUAL_RETURN_PER_SECOND: f32 = 14.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HistorySwipeAction {
    Back,
    Forward,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct HistorySwipeVisual {
    pub(crate) offset_px: f32,
    pub(crate) progress: f32,
}

impl HistorySwipeVisual {
    fn zero() -> Self {
        Self {
            offset_px: 0.0,
            progress: 0.0,
        }
    }
}

impl From<HistorySwipeVisual> for HistorySwipeVisualOffset {
    fn from(value: HistorySwipeVisual) -> Self {
        Self {
            offset_px: value.offset_px,
            progress: value.progress,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum HistorySwipeOutcome {
    PassThrough,
    Consumed {
        visual: HistorySwipeVisual,
    },
    Navigate {
        action: HistorySwipeAction,
        visual: HistorySwipeVisual,
    },
}

#[derive(Default)]
pub(crate) struct HistorySwipeState {
    gestures: HashMap<Entity, HistorySwipeGesture>,
}

impl HistorySwipeState {
    pub(crate) fn record(
        &mut self,
        webview: Entity,
        unit: MouseScrollUnit,
        delta: Vec2,
        now: Instant,
    ) -> HistorySwipeOutcome {
        if unit != MouseScrollUnit::Pixel {
            return HistorySwipeOutcome::PassThrough;
        }

        let gesture = self.gestures.entry(webview).or_default();
        if gesture.in_cooldown(now) {
            gesture.last_event = Some(now);
            return if has_horizontal_history_intent(delta) {
                HistorySwipeOutcome::Consumed {
                    visual: HistorySwipeVisual::zero(),
                }
            } else {
                HistorySwipeOutcome::PassThrough
            };
        }
        if gesture.should_reset(now, delta) {
            gesture.reset();
        }

        gesture.accumulated += delta;
        gesture.last_event = Some(now);

        if !has_horizontal_history_intent(gesture.accumulated) {
            gesture.reset();
            return HistorySwipeOutcome::PassThrough;
        }

        let visual = visual_for_accumulated(gesture.accumulated);
        if !is_horizontal_history_swipe(gesture.accumulated) {
            return HistorySwipeOutcome::Consumed { visual };
        }
        let action = if gesture.accumulated.x.is_sign_positive() {
            HistorySwipeAction::Back
        } else {
            HistorySwipeAction::Forward
        };
        gesture.reset();
        gesture.cooldown_until = Some(now + HISTORY_SWIPE_COOLDOWN);
        HistorySwipeOutcome::Navigate { action, visual }
    }
}

#[derive(Default)]
struct HistorySwipeGesture {
    accumulated: Vec2,
    last_event: Option<Instant>,
    cooldown_until: Option<Instant>,
}

impl HistorySwipeGesture {
    fn in_cooldown(&self, now: Instant) -> bool {
        self.cooldown_until.is_some_and(|until| now < until)
    }

    fn should_reset(&self, now: Instant, delta: Vec2) -> bool {
        let idle = self
            .last_event
            .is_some_and(|last| now.duration_since(last) > HISTORY_SWIPE_IDLE_RESET);
        let changed_direction = self.accumulated.x != 0.0
            && delta.x != 0.0
            && self.accumulated.x.signum() != delta.x.signum();
        let vertical_dominant = delta.y.abs() > delta.x.abs();
        idle || changed_direction || vertical_dominant
    }

    fn reset(&mut self) {
        self.accumulated = Vec2::ZERO;
    }
}

fn is_horizontal_history_swipe(delta: Vec2) -> bool {
    delta.x.abs() >= HISTORY_SWIPE_THRESHOLD_PX
        && delta.y.abs() <= delta.x.abs() * HISTORY_SWIPE_MAX_VERTICAL_RATIO
}

fn has_horizontal_history_intent(delta: Vec2) -> bool {
    delta.x.abs() > 0.0 && delta.y.abs() <= delta.x.abs() * HISTORY_SWIPE_MAX_VERTICAL_RATIO
}

fn visual_for_accumulated(delta: Vec2) -> HistorySwipeVisual {
    let direction = delta.x.signum();
    let progress = (delta.x.abs() / HISTORY_SWIPE_THRESHOLD_PX).clamp(0.0, 1.0);
    let offset_px = if progress >= 1.0 {
        HISTORY_SWIPE_NAVIGATE_OFFSET_PX
    } else {
        progress * HISTORY_SWIPE_TRACK_OFFSET_PX
    };
    HistorySwipeVisual {
        offset_px: direction * offset_px,
        progress,
    }
}

pub(crate) fn return_history_swipe_visual(
    time: Res<Time>,
    mut visuals: Query<&mut HistorySwipeVisualOffset>,
) {
    let step = (time.delta_secs() * HISTORY_SWIPE_VISUAL_RETURN_PER_SECOND).clamp(0.0, 1.0);
    for mut visual in visuals.iter_mut() {
        visual.offset_px = visual.offset_px.lerp(0.0, step);
        visual.progress = visual.progress.lerp(0.0, step);
        if visual.offset_px.abs() < 0.25 {
            visual.offset_px = 0.0;
        }
        if visual.progress < 0.01 {
            visual.progress = 0.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::input::mouse::MouseScrollUnit;
    use std::time::{Duration, Instant};

    #[test]
    fn line_wheel_passes_through_history_swipe() {
        let mut state = HistorySwipeState::default();
        let now = Instant::now();

        assert_eq!(
            state.record(
                Entity::PLACEHOLDER,
                MouseScrollUnit::Line,
                Vec2::new(999.0, 0.0),
                now
            ),
            HistorySwipeOutcome::PassThrough
        );
    }

    #[test]
    fn horizontal_pixel_swipe_is_consumed_before_threshold_to_lock_y_scroll() {
        let mut state = HistorySwipeState::default();
        let now = Instant::now();

        assert_eq!(
            state.record(
                Entity::PLACEHOLDER,
                MouseScrollUnit::Pixel,
                Vec2::new(100.0, 4.0),
                now
            ),
            HistorySwipeOutcome::Consumed {
                visual: HistorySwipeVisual {
                    offset_px: 14.0,
                    progress: 0.5,
                },
            }
        );
    }

    #[test]
    fn positive_horizontal_pixel_swipe_triggers_back() {
        let mut state = HistorySwipeState::default();
        let now = Instant::now();

        assert_eq!(
            state.record(
                Entity::PLACEHOLDER,
                MouseScrollUnit::Pixel,
                Vec2::new(100.0, 4.0),
                now
            ),
            HistorySwipeOutcome::Consumed {
                visual: HistorySwipeVisual {
                    offset_px: 14.0,
                    progress: 0.5,
                },
            }
        );
        assert_eq!(
            state.record(
                Entity::PLACEHOLDER,
                MouseScrollUnit::Pixel,
                Vec2::new(110.0, 2.0),
                now + Duration::from_millis(16)
            ),
            HistorySwipeOutcome::Navigate {
                action: HistorySwipeAction::Back,
                visual: HistorySwipeVisual {
                    offset_px: 36.0,
                    progress: 1.0,
                },
            }
        );
    }

    #[test]
    fn negative_horizontal_pixel_swipe_triggers_forward() {
        let mut state = HistorySwipeState::default();
        let now = Instant::now();

        assert_eq!(
            state.record(
                Entity::PLACEHOLDER,
                MouseScrollUnit::Pixel,
                Vec2::new(-210.0, 0.0),
                now
            ),
            HistorySwipeOutcome::Navigate {
                action: HistorySwipeAction::Forward,
                visual: HistorySwipeVisual {
                    offset_px: -36.0,
                    progress: 1.0,
                },
            }
        );
    }

    #[test]
    fn vertical_dominant_pixel_scroll_does_not_trigger_history_swipe() {
        let mut state = HistorySwipeState::default();
        let now = Instant::now();

        assert_eq!(
            state.record(
                Entity::PLACEHOLDER,
                MouseScrollUnit::Pixel,
                Vec2::new(220.0, 200.0),
                now
            ),
            HistorySwipeOutcome::PassThrough
        );
    }

    #[test]
    fn cooldown_prevents_repeat_navigation() {
        let mut state = HistorySwipeState::default();
        let now = Instant::now();

        assert_eq!(
            state.record(
                Entity::PLACEHOLDER,
                MouseScrollUnit::Pixel,
                Vec2::new(220.0, 0.0),
                now
            ),
            HistorySwipeOutcome::Navigate {
                action: HistorySwipeAction::Back,
                visual: HistorySwipeVisual {
                    offset_px: 36.0,
                    progress: 1.0,
                },
            }
        );
        assert_eq!(
            state.record(
                Entity::PLACEHOLDER,
                MouseScrollUnit::Pixel,
                Vec2::new(220.0, 0.0),
                now + Duration::from_millis(100)
            ),
            HistorySwipeOutcome::Consumed {
                visual: HistorySwipeVisual {
                    offset_px: 0.0,
                    progress: 0.0,
                },
            }
        );
    }
}
