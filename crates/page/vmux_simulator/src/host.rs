mod device;
mod hid;
mod input;
mod stream;

use crate::event::{
    HardwareButton, SIMULATOR_READY_EVENT, SimulatorKey, SimulatorReady, SimulatorTouch,
};
use crate::url::{PAGE_HOST, PAGE_URL, SimulatorRoute};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::winit::{EventLoopProxyWrapper, WinitUserEvent};
use bevy_cef::prelude::*;
use hid::{HidBroker, HidRequest};
use input::{DeviceCoordinates, DeviceKey, DeviceTouch, DeviceTouchSession};
use std::sync::{Arc, Mutex};
use stream::StreamServer;
use vmux_core::PageMetadata;
use vmux_core::host::page::{NativelyHosted, PageReady};
use vmux_wire::protocol::{SimulatorAction, SimulatorButton};

pub use device::{Axe, SimulatorDevice};

pub struct SimulatorPlugin;

impl Plugin for SimulatorPlugin {
    fn build(&self, app: &mut App) {
        app.world_mut().spawn((
            PAGE_MANIFEST,
            NativelyHosted::subtree(PAGE_URL, PAGE_MANIFEST.title),
        ));
        vmux_core::register_host_spawn(app, PAGE_HOST);
        app.init_resource::<DeviceAttachment>()
            .init_resource::<Announced>()
            .init_resource::<DeviceTouchSession>()
            .add_message::<HardwareButtonRequest>()
            .add_message::<SimulatorControlRequest>()
            .add_message::<SimulatorControlResponse>()
            .add_message::<SimulatorScreenshotRequest>()
            .add_message::<SimulatorScreenshotResponse>()
            .add_systems(Update, (Self::attach_device, Self::announce).chain())
            .add_systems(
                Update,
                (
                    Self::handle_button_requests,
                    Self::handle_control_requests,
                    Self::handle_screenshot_requests,
                )
                    .in_set(SimulatorInputSet),
            )
            .add_plugins(
                BinEventEmitterPlugin::<(SimulatorTouch, SimulatorKey)>::for_hosts(&[PAGE_HOST]),
            )
            .add_observer(Self::on_touch)
            .add_observer(Self::on_key)
            .add_observer(Self::forget_on_reload);
    }
}

#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct HardwareButtonRequest(pub HardwareButton);

#[derive(Message, Clone)]
pub struct SimulatorControlRequest {
    pub request_id: [u8; 16],
    pub action: SimulatorAction,
}

#[derive(Message, Clone)]
pub struct SimulatorControlResponse {
    pub request_id: [u8; 16],
    pub result: Result<String, String>,
}

#[derive(Message, Clone)]
pub struct SimulatorScreenshotRequest {
    pub request_id: [u8; 16],
}

#[derive(Clone)]
pub struct SimulatorScreenshot {
    pub path: String,
    pub png: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

#[derive(Message, Clone)]
pub struct SimulatorScreenshotResponse {
    pub request_id: [u8; 16],
    pub result: Result<SimulatorScreenshot, String>,
}

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SimulatorInputSet;

pub const PAGE_MANIFEST: vmux_core::page::PageManifest = vmux_core::page::PageManifest {
    host: PAGE_HOST,
    title: "Simulator",
    title_message_id: Some("simulator-title"),
    replaces_command: None,
    keywords: &["simulator", "ios", "iphone", "device"],
    icon: Some(vmux_core::BuiltinIcon::Layers),
    command_bar: true,
};

#[derive(Resource)]
struct DevicePoints(f32, f32);

#[derive(Resource)]
struct DevicePixels(u32, u32);

#[derive(Resource, Default)]
struct Announced(HashMap<Entity, SimulatorReady>);

#[derive(Resource, Default)]
struct DeviceAttachment {
    phase: AttachmentPhase,
}

#[derive(Default)]
enum AttachmentPhase {
    #[default]
    Idle,
    Starting(Arc<Mutex<Option<Result<AttachedDevice, String>>>>),
    Complete,
}

struct AttachedDevice {
    axe: Axe,
    hid: HidBroker,
    device: SimulatorDevice,
    points: Option<(f32, f32)>,
    pixels: Option<(u32, u32)>,
    server: StreamServer,
}

impl SimulatorPlugin {
    const URL_PREFIX: &'static str = "vmux://simulator/";

    fn attach_device(
        views: Query<&PageMetadata>,
        mut attachment: ResMut<DeviceAttachment>,
        wake: Option<Res<EventLoopProxyWrapper>>,
        mut commands: Commands,
    ) {
        if attachment.is_idle() {
            let Some(route) = views
                .iter()
                .find_map(|metadata| SimulatorRoute::of_url(&metadata.url))
            else {
                return;
            };
            let wake = wake.map(|wrapper| {
                let proxy = (**wrapper).clone();
                Box::new(move || {
                    let _ = proxy.send_event(WinitUserEvent::WakeUp);
                }) as Box<dyn FnOnce() + Send>
            });
            attachment.start(route, wake);
            return;
        }
        let Some(result) = attachment.take() else {
            return;
        };
        let attached = match result {
            Ok(attached) => attached,
            Err(error) => {
                error!("could not attach an iOS Simulator: {error}");
                return;
            }
        };
        info!(
            "mirroring {} on loopback port {}",
            attached.device.name,
            attached.server.port()
        );
        if let Some((width, height)) = attached.points {
            commands.insert_resource(DevicePoints(width, height));
        }
        if let Some((width, height)) = attached.pixels {
            commands.insert_resource(DevicePixels(width, height));
        }
        commands.insert_resource(attached.server);
        commands.insert_resource(attached.device);
        commands.insert_resource(attached.hid);
        commands.insert_resource(attached.axe);
        if let Some(wake) = wake {
            let _ = wake.send_event(WinitUserEvent::WakeUp);
        }
    }

    fn announce(
        browsers: NonSend<Browsers>,
        views: Query<(Entity, &PageMetadata, Option<&ChildOf>), With<PageReady>>,
        server: Option<Res<StreamServer>>,
        device: Option<Res<SimulatorDevice>>,
        mut told: ResMut<Announced>,
        mut commands: Commands,
    ) {
        let payload = match (server.as_deref(), device.as_deref()) {
            (Some(server), Some(device)) => SimulatorReady {
                port: server.port(),
                version: device
                    .version
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_default(),
                device_name: device.name.clone(),
            },
            _ => SimulatorReady::default(),
        };
        told.0.retain(|entity, _| views.contains(*entity));
        for (entity, meta, child_of) in views.iter() {
            if !meta.url.starts_with(Self::URL_PREFIX) {
                continue;
            }
            if let Some(canonical_url) = device.as_deref().and_then(SimulatorDevice::canonical_url)
                && meta.url != canonical_url
            {
                let mut canonical = meta.clone();
                canonical.url = canonical_url;
                commands.entity(entity).insert(canonical.clone());
                if let Some(child_of) = child_of {
                    commands.entity(child_of.parent()).insert(canonical);
                }
            }
            if told.0.get(&entity) == Some(&payload) {
                continue;
            }
            if !browsers.can_emit_to(&entity) {
                continue;
            }
            commands.trigger(BinHostEmitEvent::from_rkyv(
                entity,
                SIMULATOR_READY_EVENT,
                &payload,
            ));
            told.0.insert(entity, payload.clone());
        }
    }

    fn forget_on_reload(trigger: On<BinReceive<PageReady>>, mut told: ResMut<Announced>) {
        told.0.remove(&trigger.event().webview);
    }

    fn on_touch(
        trigger: On<BinReceive<SimulatorTouch>>,
        points: Option<Res<DevicePoints>>,
        hid: Option<Res<HidBroker>>,
        mut session: ResMut<DeviceTouchSession>,
    ) {
        let (Some(points), Some(hid)) = (points.as_deref(), hid.as_deref()) else {
            return;
        };
        let Some(touch) = DeviceTouch::resolve(&trigger.event().payload, (points.0, points.1))
        else {
            return;
        };
        touch.dispatch(&mut session, hid);
    }

    fn on_key(
        trigger: On<BinReceive<SimulatorKey>>,
        device: Option<Res<SimulatorDevice>>,
        axe: Option<Res<Axe>>,
    ) {
        let (Some(device), Some(axe)) = (device.as_deref(), axe.as_deref()) else {
            return;
        };
        DeviceKey::resolve(&trigger.event().payload, device).dispatch(axe);
    }

    fn handle_button_requests(
        mut requests: MessageReader<HardwareButtonRequest>,
        device: Option<Res<SimulatorDevice>>,
        axe: Option<Res<Axe>>,
    ) {
        let (Some(device), Some(axe)) = (device.as_deref(), axe.as_deref()) else {
            return;
        };
        for request in requests.read() {
            DeviceKey::resolve(&SimulatorKey::Button(request.0), device).dispatch(axe);
        }
    }

    fn handle_control_requests(
        mut requests: MessageReader<SimulatorControlRequest>,
        mut responses: MessageWriter<SimulatorControlResponse>,
        points: Option<Res<DevicePoints>>,
        pixels: Option<Res<DevicePixels>>,
        hid: Option<Res<HidBroker>>,
        device: Option<Res<SimulatorDevice>>,
        axe: Option<Res<Axe>>,
    ) {
        for request in requests.read() {
            let result = request.dispatch(
                points.as_deref(),
                pixels.as_deref(),
                hid.as_deref(),
                device.as_deref(),
                axe.as_deref(),
            );
            responses.write(SimulatorControlResponse {
                request_id: request.request_id,
                result,
            });
        }
    }

    fn handle_screenshot_requests(
        mut requests: MessageReader<SimulatorScreenshotRequest>,
        mut responses: MessageWriter<SimulatorScreenshotResponse>,
        device: Option<Res<SimulatorDevice>>,
        axe: Option<Res<Axe>>,
    ) {
        for request in requests.read() {
            let result = match (device.as_deref(), axe.as_deref()) {
                (Some(device), Some(axe)) => {
                    SimulatorScreenshot::capture(request.request_id, device, axe)
                }
                _ => Err("no iOS Simulator is attached".to_string()),
            };
            responses.write(SimulatorScreenshotResponse {
                request_id: request.request_id,
                result,
            });
        }
    }
}

impl SimulatorControlRequest {
    fn dispatch(
        &self,
        points: Option<&DevicePoints>,
        pixels: Option<&DevicePixels>,
        hid: Option<&HidBroker>,
        device: Option<&SimulatorDevice>,
        axe: Option<&Axe>,
    ) -> Result<String, String> {
        match &self.action {
            SimulatorAction::Tap { x, y } => {
                let coordinates = Self::coordinates(points, pixels)?;
                let hid = hid.ok_or("simulator input is unavailable")?;
                hid.dispatch(HidRequest::tap(coordinates.point((*x, *y))));
                Ok(format!("tapped simulator at ({x}, {y})"))
            }
            SimulatorAction::Swipe {
                start_x,
                start_y,
                end_x,
                end_y,
                duration_ms,
            } => {
                let coordinates = Self::coordinates(points, pixels)?;
                let hid = hid.ok_or("simulator input is unavailable")?;
                let from = coordinates.point((*start_x, *start_y));
                let to = coordinates.point((*end_x, *end_y));
                hid.dispatch(HidRequest::swipe(from, to, *duration_ms));
                Ok(format!(
                    "swiped simulator from ({start_x}, {start_y}) to ({end_x}, {end_y})"
                ))
            }
            SimulatorAction::TypeText(text) => {
                let device = device.ok_or("no iOS Simulator is attached")?;
                let axe = axe.ok_or("simulator input is unavailable")?;
                DeviceKey::resolve(&SimulatorKey::Text(text.clone()), device).dispatch(axe);
                Ok("typed text into simulator".to_string())
            }
            SimulatorAction::Key(keycode) => {
                let device = device.ok_or("no iOS Simulator is attached")?;
                let axe = axe.ok_or("simulator input is unavailable")?;
                DeviceKey::resolve(&SimulatorKey::Code(u16::from(*keycode)), device).dispatch(axe);
                Ok(format!("pressed simulator keycode {keycode}"))
            }
            SimulatorAction::Button(button) => {
                let device = device.ok_or("no iOS Simulator is attached")?;
                let axe = axe.ok_or("simulator input is unavailable")?;
                let button = match button {
                    SimulatorButton::Home => HardwareButton::Home,
                    SimulatorButton::Lock => HardwareButton::Lock,
                    SimulatorButton::Siri => HardwareButton::Siri,
                };
                DeviceKey::resolve(&SimulatorKey::Button(button), device).dispatch(axe);
                Ok("pressed simulator hardware button".to_string())
            }
        }
    }

    fn coordinates(
        points: Option<&DevicePoints>,
        pixels: Option<&DevicePixels>,
    ) -> Result<DeviceCoordinates, String> {
        let points = points.ok_or("simulator point dimensions are unavailable")?;
        let pixels = pixels.ok_or("simulator pixel dimensions are unavailable")?;
        DeviceCoordinates::new((points.0, points.1), (pixels.0, pixels.1))
            .ok_or_else(|| "simulator dimensions are invalid".to_string())
    }
}

impl SimulatorScreenshot {
    fn capture(request_id: [u8; 16], device: &SimulatorDevice, axe: &Axe) -> Result<Self, String> {
        let path = std::env::temp_dir().join(format!(
            "vmux-simulator-{}-{:02x}{:02x}{:02x}{:02x}.png",
            std::process::id(),
            request_id[0],
            request_id[1],
            request_id[2],
            request_id[3]
        ));
        let png = device.screenshot(axe, &path)?;
        let (width, height) = SimulatorDevice::png_size(&png)
            .ok_or_else(|| "simulator screenshot is not a valid PNG".to_string())?;
        Ok(Self {
            path: path.to_string_lossy().into_owned(),
            png,
            width,
            height,
        })
    }
}

impl DeviceAttachment {
    fn is_idle(&self) -> bool {
        matches!(self.phase, AttachmentPhase::Idle)
    }

    fn start(&mut self, route: SimulatorRoute, wake: Option<Box<dyn FnOnce() + Send>>) {
        let result = Arc::new(Mutex::new(None));
        let worker_result = result.clone();
        let spawned = std::thread::Builder::new()
            .name("vmux-simulator-attach".into())
            .spawn(move || {
                let attached = AttachedDevice::start(&route);
                if let Ok(mut result) = worker_result.lock() {
                    *result = Some(attached);
                }
                if let Some(wake) = wake {
                    wake();
                }
            });
        match spawned {
            Ok(_) => self.phase = AttachmentPhase::Starting(result),
            Err(error) => {
                error!("could not start simulator attachment: {error}");
                self.phase = AttachmentPhase::Complete;
            }
        }
    }

    fn take(&mut self) -> Option<Result<AttachedDevice, String>> {
        let ready = match &self.phase {
            AttachmentPhase::Starting(result) => result.lock().ok()?.take(),
            AttachmentPhase::Idle | AttachmentPhase::Complete => None,
        };
        if ready.is_some() {
            self.phase = AttachmentPhase::Complete;
        }
        ready
    }
}

impl AttachedDevice {
    fn start(route: &SimulatorRoute) -> Result<Self, String> {
        let axe = Axe::locate().ok_or_else(|| {
            format!(
                "`{}` not found; install it with `brew install cameroncooke/axe/axe`",
                Axe::BIN
            )
        })?;
        info!(
            "axe {} at {}",
            axe.version().unwrap_or_default(),
            axe.path().display()
        );
        let device = SimulatorDevice::booted_or_boot(route.version(), route.device_name())?;
        let points = device.point_size(&axe);
        let pixels = device.pixel_size(&axe);
        let hid = HidBroker::start(&axe, &device)
            .map_err(|error| format!("could not start simulator input: {error}"))?;
        let server = StreamServer::start(&axe, device.clone(), pixels)
            .map_err(|error| format!("could not serve the simulator stream: {error}"))?;
        Ok(Self {
            axe,
            hid,
            device,
            points,
            pixels,
            server,
        })
    }
}

impl SimulatorDevice {
    pub fn canonical_url(&self) -> Option<String> {
        self.version
            .as_ref()
            .map(|version| crate::url::SimulatorRoute::url(version, Some(&self.name)))
    }
}
