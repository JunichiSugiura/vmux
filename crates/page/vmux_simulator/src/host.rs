mod device;
mod input;
mod stream;

use crate::event::{SIMULATOR_READY_EVENT, SimulatorGesture, SimulatorKey, SimulatorReady};
use crate::url::{PAGE_HOST, PAGE_URL, SimulatorRoute};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy_cef::prelude::*;
use input::{DeviceGesture, DeviceKey};
use stream::StreamServer;
use vmux_core::PageMetadata;
use vmux_core::host::page::{NativelyHosted, PageReady};

pub use device::{Axe, SimulatorDevice};

pub struct SimulatorPlugin;

impl Plugin for SimulatorPlugin {
    fn build(&self, app: &mut App) {
        app.world_mut().spawn((
            PAGE_MANIFEST,
            NativelyHosted::subtree(PAGE_URL, PAGE_MANIFEST.title),
        ));
        vmux_core::register_host_spawn(app, PAGE_HOST);
        app.init_resource::<Announced>()
            .add_systems(Startup, Self::attach_device)
            .add_systems(Update, Self::announce)
            .add_plugins(
                BinEventEmitterPlugin::<(SimulatorGesture, SimulatorKey)>::for_hosts(&[PAGE_HOST]),
            )
            .add_observer(Self::on_gesture)
            .add_observer(Self::on_key)
            .add_observer(Self::forget_on_reload);
    }
}

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

#[derive(Resource, Default)]
struct Announced(HashMap<Entity, SimulatorReady>);

impl SimulatorPlugin {
    const URL_PREFIX: &'static str = "vmux://simulator/";

    fn attach_device(mut commands: Commands) {
        let Some(axe) = Axe::locate() else {
            warn!(
                "`{}` not found — the simulator page needs it; \
                 install with `brew install cameroncooke/axe/axe`",
                Axe::BIN
            );
            return;
        };
        info!(
            "axe {} at {}",
            axe.version().unwrap_or_default(),
            axe.path().display()
        );
        let Some(device) = SimulatorDevice::booted() else {
            info!("no booted simulator; the simulator page will be empty");
            return;
        };
        if let Some((w, h)) = device.point_size(&axe) {
            commands.insert_resource(DevicePoints(w, h));
        }
        match StreamServer::start(&axe, device.clone()) {
            Ok(server) => {
                info!(
                    "mirroring {} on loopback port {}",
                    device.name,
                    server.port()
                );
                commands.insert_resource(server);
            }
            Err(error) => error!("could not serve the simulator stream: {error}"),
        }
        commands.insert_resource(device);
        commands.insert_resource(axe);
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
            if matches!(
                SimulatorRoute::of_url(&meta.url),
                Some(SimulatorRoute::Unpinned)
            ) && let Some(version) = crate::url::IosVersion::parse(&payload.version)
            {
                let mut canonical = meta.clone();
                canonical.url = SimulatorRoute::url(&version);
                commands.entity(entity).insert(canonical.clone());
                if let Some(child_of) = child_of {
                    commands.entity(child_of.parent()).insert(canonical);
                }
            }
            if told.0.get(&entity) == Some(&payload) {
                continue;
            }
            if !browsers.has_browser(entity) || !browsers.host_emit_ready(&entity) {
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

    fn on_gesture(
        trigger: On<BinReceive<SimulatorGesture>>,
        device: Option<Res<SimulatorDevice>>,
        points: Option<Res<DevicePoints>>,
        axe: Option<Res<Axe>>,
    ) {
        let (Some(device), Some(points), Some(axe)) =
            (device.as_deref(), points.as_deref(), axe.as_deref())
        else {
            return;
        };
        let Some(gesture) =
            DeviceGesture::resolve(&trigger.event().payload, device, (points.0, points.1))
        else {
            return;
        };
        gesture.dispatch(axe);
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
}

impl SimulatorDevice {
    pub fn canonical_url(&self) -> Option<String> {
        self.version.as_ref().map(crate::url::SimulatorRoute::url)
    }
}
