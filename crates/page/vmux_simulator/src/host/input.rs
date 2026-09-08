use super::device::{Axe, SimulatorDevice};
use super::hid::{HidBroker, HidRequest};
use crate::event::{SimulatorKey, SimulatorTouch, SimulatorTouchPhase};
use bevy::prelude::Resource;

pub struct DeviceTouch {
    point: (f32, f32),
    phase: SimulatorTouchPhase,
}

impl DeviceTouch {
    pub fn resolve(touch: &SimulatorTouch, points: (f32, f32)) -> Option<Self> {
        if points.0 <= 0.0 || points.1 <= 0.0 {
            return None;
        }
        let max_x = (points.0 - 1.0).max(0.0);
        let max_y = (points.1 - 1.0).max(0.0);
        Some(Self {
            point: (
                touch.x.clamp(0.0, 1.0) * max_x,
                touch.y.clamp(0.0, 1.0) * max_y,
            ),
            phase: touch.phase,
        })
    }

    pub fn dispatch(self, session: &mut DeviceTouchSession, hid: &HidBroker) {
        match self.phase {
            SimulatorTouchPhase::Down => {
                if let Some(previous) = session.last {
                    hid.dispatch(HidRequest::up(previous));
                }
                session.active = true;
                session.last = Some(self.point);
                hid.dispatch(HidRequest::down(self.point));
            }
            SimulatorTouchPhase::Move => {
                if !session.active {
                    return;
                }
                session.last = Some(self.point);
                hid.dispatch(HidRequest::move_to(self.point));
            }
            SimulatorTouchPhase::Up => {
                if !session.active {
                    return;
                }
                session.active = false;
                session.last = None;
                hid.dispatch(HidRequest::up(self.point));
            }
        }
    }
}

#[derive(Resource, Default)]
pub struct DeviceTouchSession {
    active: bool,
    last: Option<(f32, f32)>,
}

pub struct DeviceKey {
    udid: String,
    key: SimulatorKey,
}

impl DeviceKey {
    pub fn resolve(key: &SimulatorKey, device: &SimulatorDevice) -> Self {
        Self {
            udid: device.udid.clone(),
            key: key.clone(),
        }
    }

    pub fn dispatch(self, axe: &Axe) {
        let mut command = axe.command();
        match &self.key {
            SimulatorKey::Text(text) => {
                command.arg("type").arg(text);
            }
            SimulatorKey::Code(code) => {
                command.arg("key").arg(code.to_string());
            }
            SimulatorKey::Button(button) => {
                command.arg("button").arg(button.as_arg());
            }
        }
        command.args(["--udid", &self.udid]);
        Axe::run_detached(command);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const POINTS: (f32, f32) = (402.0, 874.0);

    #[test]
    fn the_centre_of_the_view_is_the_centre_of_the_device() {
        let touch = SimulatorTouch {
            phase: SimulatorTouchPhase::Move,
            x: 0.5,
            y: 0.5,
        };

        let resolved = DeviceTouch::resolve(&touch, POINTS).expect("resolved");

        assert!(
            (resolved.point.0 - 200.5).abs() < 0.01,
            "got {:?}",
            resolved.point
        );
        assert!(
            (resolved.point.1 - 436.5).abs() < 0.01,
            "got {:?}",
            resolved.point
        );
    }

    #[test]
    fn fractions_outside_the_image_are_clamped_onto_it() {
        let touch = SimulatorTouch {
            phase: SimulatorTouchPhase::Down,
            x: -0.5,
            y: 2.0,
        };

        let resolved = DeviceTouch::resolve(&touch, POINTS).expect("resolved");

        assert_eq!(resolved.point.0, 0.0);
        assert!(
            (resolved.point.1 - (POINTS.1 - 1.0)).abs() < 0.01,
            "got {:?}",
            resolved.point
        );
    }

    #[test]
    fn a_device_with_no_measured_point_size_has_no_gesture() {
        let touch = SimulatorTouch {
            phase: SimulatorTouchPhase::Down,
            x: 0.5,
            y: 0.5,
        };

        assert!(DeviceTouch::resolve(&touch, (0.0, 0.0)).is_none());
    }
}
