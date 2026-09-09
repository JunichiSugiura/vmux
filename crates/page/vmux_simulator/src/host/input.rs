use super::device::{Axe, SimulatorDevice};
use super::hid::{HidBroker, HidRequest};
use crate::event::{SimulatorClipboardAction, SimulatorKey, SimulatorTouch, SimulatorTouchPhase};
use bevy::prelude::Resource;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{LazyLock, mpsc};

pub struct DeviceTouch {
    point: (f32, f32),
    phase: SimulatorTouchPhase,
}

pub struct DeviceCoordinates {
    points: (f32, f32),
    pixels: (u32, u32),
}

impl DeviceCoordinates {
    pub fn new(points: (f32, f32), pixels: (u32, u32)) -> Option<Self> {
        if points.0 <= 0.0 || points.1 <= 0.0 || pixels.0 == 0 || pixels.1 == 0 {
            return None;
        }
        Some(Self { points, pixels })
    }

    pub fn point(&self, pixel: (u32, u32)) -> (f32, f32) {
        let max_pixel_x = self.pixels.0.saturating_sub(1).max(1) as f32;
        let max_pixel_y = self.pixels.1.saturating_sub(1).max(1) as f32;
        let max_point_x = (self.points.0 - 1.0).max(0.0);
        let max_point_y = (self.points.1 - 1.0).max(0.0);
        (
            pixel.0.min(self.pixels.0.saturating_sub(1)) as f32 / max_pixel_x * max_point_x,
            pixel.1.min(self.pixels.1.saturating_sub(1)) as f32 / max_pixel_y * max_point_y,
        )
    }
}

impl DeviceTouch {
    const DRAG_THRESHOLD: f32 = 6.0;

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
                if session.dragging
                    && let Some(previous) = session.last
                {
                    hid.dispatch(HidRequest::up(previous));
                }
                session.start = Some(self.point);
                session.last = Some(self.point);
                session.dragging = false;
            }
            SimulatorTouchPhase::Move => {
                let Some(start) = session.start else {
                    return;
                };
                if !session.dragging && Self::distance(start, self.point) >= Self::DRAG_THRESHOLD {
                    hid.dispatch(HidRequest::down(start));
                    session.dragging = true;
                }
                session.last = Some(self.point);
                if session.dragging {
                    hid.dispatch(HidRequest::move_to(self.point));
                }
            }
            SimulatorTouchPhase::Up => {
                let Some(start) = session.start.take() else {
                    return;
                };
                session.last = None;
                if session.dragging {
                    hid.dispatch(HidRequest::up(self.point));
                } else {
                    hid.dispatch(HidRequest::tap(start));
                }
                session.dragging = false;
            }
            SimulatorTouchPhase::Cancel => {
                session.start = None;
                if session.dragging
                    && let Some(last) = session.last.take()
                {
                    hid.dispatch(HidRequest::up(last));
                }
                session.last = None;
                session.dragging = false;
            }
        }
    }

    fn distance(from: (f32, f32), to: (f32, f32)) -> f32 {
        ((to.0 - from.0).powi(2) + (to.1 - from.1).powi(2)).sqrt()
    }
}

#[derive(Resource, Default)]
pub struct DeviceTouchSession {
    start: Option<(f32, f32)>,
    last: Option<(f32, f32)>,
    dragging: bool,
}

pub struct DeviceKey {
    udid: String,
    key: SimulatorKey,
}

pub struct DeviceClipboard {
    axe: PathBuf,
    udid: String,
    action: SimulatorClipboardAction,
}

impl DeviceClipboard {
    pub fn resolve(action: SimulatorClipboardAction, device: &SimulatorDevice, axe: &Axe) -> Self {
        Self {
            axe: axe.path().to_path_buf(),
            udid: device.udid.clone(),
            action,
        }
    }

    pub fn dispatch(self) {
        if Self::sender().send(self).is_err() {
            bevy::log::error!("simulator clipboard worker stopped");
        }
    }

    fn sender() -> &'static mpsc::Sender<Self> {
        static SENDER: LazyLock<mpsc::Sender<DeviceClipboard>> = LazyLock::new(|| {
            let (sender, receiver) = mpsc::channel::<DeviceClipboard>();
            let spawned = std::thread::Builder::new()
                .name("vmux-simulator-clipboard".into())
                .spawn(move || {
                    for request in receiver {
                        if let Err(error) = request.run() {
                            bevy::log::error!("simulator clipboard failed: {error}");
                        }
                    }
                });
            if let Err(error) = spawned {
                bevy::log::error!("could not start simulator clipboard worker: {error}");
            }
            sender
        });
        &SENDER
    }

    fn run(&self) -> Result<(), String> {
        match self.action {
            SimulatorClipboardAction::Copy => {
                self.key_combo(6)?;
                self.sync(&self.udid, "host")
            }
            SimulatorClipboardAction::Paste => {
                self.sync("host", &self.udid)?;
                self.key_combo(25)
            }
        }
    }

    fn key_combo(&self, key: u8) -> Result<(), String> {
        let output = Command::new(&self.axe)
            .args([
                "key-combo",
                "--modifiers",
                "227",
                "--key",
                &key.to_string(),
                "--udid",
                &self.udid,
            ])
            .stdin(Stdio::null())
            .output()
            .map_err(|error| format!("could not send simulator clipboard shortcut: {error}"))?;
        if output.status.success() {
            return Ok(());
        }
        Err(format!(
            "could not send simulator clipboard shortcut: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }

    fn sync(&self, source: &str, destination: &str) -> Result<(), String> {
        let output = Command::new("xcrun")
            .args(["simctl", "pbsync", source, destination])
            .stdin(Stdio::null())
            .output()
            .map_err(|error| format!("could not sync simulator clipboard: {error}"))?;
        if output.status.success() {
            return Ok(());
        }
        Err(format!(
            "could not sync simulator clipboard: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
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

    #[test]
    fn screenshot_pixels_map_to_simulator_points() {
        let coordinates = DeviceCoordinates::new((402.0, 874.0), (1206, 2622)).expect("mapping");

        let point = coordinates.point((603, 1311));

        assert!((point.0 - 200.67).abs() < 0.01);
        assert!((point.1 - 436.67).abs() < 0.01);
    }
}
