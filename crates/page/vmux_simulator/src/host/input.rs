use super::device::{Axe, SimulatorDevice};
use super::hid::{HidBroker, HidRequest};
use crate::event::{SimulatorGesture, SimulatorKey};

pub struct DeviceGesture {
    points: (f32, f32),
    from: (f32, f32),
    to: (f32, f32),
    tap: bool,
    from_bottom_edge: bool,
}

impl DeviceGesture {
    pub fn resolve(gesture: &SimulatorGesture, points: (f32, f32)) -> Option<Self> {
        if points.0 <= 0.0 || points.1 <= 0.0 {
            return None;
        }
        let max_x = (points.0 - 1.0).max(0.0);
        let max_y = (points.1 - 1.0).max(0.0);
        let on_device = |x: f32, y: f32| (x.clamp(0.0, 1.0) * max_x, y.clamp(0.0, 1.0) * max_y);
        let dx = gesture.to_x - gesture.from_x;
        let dy = gesture.to_y - gesture.from_y;
        Some(Self {
            points,
            from: on_device(gesture.from_x, gesture.from_y),
            to: on_device(gesture.to_x, gesture.to_y),
            tap: gesture.is_tap(),
            from_bottom_edge: gesture.from_y >= 0.94 && dy < -0.1 && dy.abs() > dx.abs(),
        })
    }

    pub fn dispatch(self, hid: &HidBroker) {
        let request = if self.tap {
            HidRequest::tap(self.to)
        } else if self.from_bottom_edge {
            HidRequest::bottom_edge(self.points)
        } else {
            HidRequest::swipe(self.from, self.to)
        };
        hid.dispatch(request);
    }
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
        let gesture = SimulatorGesture {
            from_x: 0.5,
            from_y: 0.5,
            to_x: 0.5,
            to_y: 0.5,
        };

        let resolved = DeviceGesture::resolve(&gesture, POINTS).expect("resolved");

        assert!(
            (resolved.to.0 - 200.5).abs() < 0.01,
            "got {:?}",
            resolved.to
        );
        assert!(
            (resolved.to.1 - 436.5).abs() < 0.01,
            "got {:?}",
            resolved.to
        );
        assert!(resolved.tap);
    }

    #[test]
    fn a_drag_keeps_its_direction_and_is_not_a_tap() {
        let gesture = SimulatorGesture {
            from_x: 0.5,
            from_y: 0.8,
            to_x: 0.5,
            to_y: 0.2,
        };

        let resolved = DeviceGesture::resolve(&gesture, POINTS).expect("resolved");

        assert!(!resolved.tap);
        assert!(resolved.from.1 > resolved.to.1, "expected an upward swipe");
        assert!(!resolved.from_bottom_edge);
    }

    #[test]
    fn fractions_outside_the_image_are_clamped_onto_it() {
        let gesture = SimulatorGesture {
            from_x: -0.5,
            from_y: 2.0,
            to_x: -0.5,
            to_y: 2.0,
        };

        let resolved = DeviceGesture::resolve(&gesture, POINTS).expect("resolved");

        assert_eq!(resolved.to.0, 0.0);
        assert!(
            (resolved.to.1 - (POINTS.1 - 1.0)).abs() < 0.01,
            "got {:?}",
            resolved.to
        );
    }

    #[test]
    fn an_upward_drag_from_the_home_indicator_uses_the_bottom_edge_gesture() {
        let gesture = SimulatorGesture {
            from_x: 0.5,
            from_y: 0.98,
            to_x: 0.5,
            to_y: 0.65,
        };

        let resolved = DeviceGesture::resolve(&gesture, POINTS).expect("resolved");

        assert!(resolved.from_bottom_edge);
    }

    #[test]
    fn a_device_with_no_measured_point_size_has_no_gesture() {
        let gesture = SimulatorGesture::default();

        assert!(DeviceGesture::resolve(&gesture, (0.0, 0.0)).is_none());
    }
}
