use super::{
    ActiveSimulatorView, SimulatorDevice, SimulatorInputSet, SimulatorSoftwareKeyboardRequest,
};
use bevy::prelude::*;
use bevy::tasks::{IoTaskPool, Task, futures_lite::future};
use bevy::winit::{EventLoopProxyWrapper, WinitUserEvent};
use objc2::rc::{Retained, autoreleasepool};
use objc2::runtime::{AnyClass, AnyObject};
use objc2::{msg_send, sel};
use objc2_foundation::{NSArray, NSError, NSString, NSUUID};
use std::collections::{HashMap, VecDeque};
use std::ffi::{CStr, CString};
use std::io::Write;
use std::process::Command;
use std::sync::OnceLock;

pub(super) struct CoreSimulatorPlugin;

impl Plugin for CoreSimulatorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HardwareKeyboardState>()
            .add_message::<HardwareKeyboardSetRequest>()
            .add_systems(Update, update_hardware_keyboard.in_set(SimulatorInputSet));
    }
}

#[derive(Message)]
pub(super) struct HardwareKeyboardSetRequest {
    pub udid: String,
    pub enabled: bool,
}

#[derive(Resource, Default)]
struct HardwareKeyboardState {
    enabled: HashMap<String, bool>,
    requested: VecDeque<(String, bool)>,
    pending: Option<HardwareKeyboardTask>,
}

struct HardwareKeyboardTask {
    udid: String,
    enabled: bool,
    task: Task<Result<(), String>>,
}

fn update_hardware_keyboard(
    mut sets: MessageReader<HardwareKeyboardSetRequest>,
    mut toggles: MessageReader<SimulatorSoftwareKeyboardRequest>,
    active: Res<ActiveSimulatorView>,
    devices: Query<(Entity, &SimulatorDevice)>,
    wake: Option<Res<EventLoopProxyWrapper>>,
    mut state: ResMut<HardwareKeyboardState>,
) {
    let completed = state
        .pending
        .as_mut()
        .and_then(|pending| future::block_on(future::poll_once(&mut pending.task)));
    if let Some(result) = completed {
        let pending = state.pending.take().unwrap();
        match result {
            Ok(()) => {
                state.enabled.insert(pending.udid, pending.enabled);
            }
            Err(error) => error!("could not change simulator keyboard mode: {error}"),
        }
    }

    for request in sets.read() {
        state
            .requested
            .push_back((request.udid.clone(), request.enabled));
    }
    for request in toggles.read() {
        let target = request
            .view
            .filter(|entity| devices.contains(*entity))
            .or_else(|| active.select(devices.iter().map(|(entity, _)| entity)));
        let Some(target) = target else {
            continue;
        };
        let Ok((_, device)) = devices.get(target) else {
            continue;
        };
        let enabled = state
            .requested
            .iter()
            .rev()
            .find(|(udid, _)| udid == &device.udid)
            .map(|(_, enabled)| *enabled)
            .or_else(|| {
                state
                    .pending
                    .as_ref()
                    .filter(|pending| pending.udid == device.udid)
                    .map(|pending| pending.enabled)
            })
            .or_else(|| state.enabled.get(&device.udid).copied())
            .unwrap_or(true);
        state.requested.push_back((device.udid.clone(), !enabled));
    }

    if state.pending.is_some() {
        return;
    }
    let Some((udid, enabled)) = state.requested.pop_front() else {
        return;
    };
    let wake = wake.map(|wrapper| (**wrapper).clone());
    let requested_udid = udid.clone();
    let task = IoTaskPool::get().spawn(async move {
        let result = request_hardware_keyboard(&requested_udid, enabled);
        if let Some(wake) = wake {
            let _ = wake.send_event(WinitUserEvent::WakeUp);
        }
        result
    });
    state.pending = Some(HardwareKeyboardTask {
        udid,
        enabled,
        task,
    });
}

pub(super) fn exit_if_requested() {
    let Some(request) = helper_request() else {
        return;
    };
    let result = request.and_then(|(udid, enabled)| set_hardware_keyboard(&udid, enabled));
    match result {
        Ok(()) => std::process::exit(0),
        Err(error) => {
            let _ = writeln!(std::io::stderr(), "{error}");
            std::process::exit(1);
        }
    }
}

fn request_hardware_keyboard(udid: &str, enabled: bool) -> Result<(), String> {
    let executable =
        std::env::current_exe().map_err(|error| format!("could not locate Vmux: {error}"))?;
    let output = Command::new(executable)
        .arg("--vmux-core-simulator-keyboard")
        .arg(udid)
        .arg(if enabled { "enabled" } else { "disabled" })
        .output()
        .map_err(|error| format!("could not start CoreSimulator keyboard helper: {error}"))?;
    if output.status.success() {
        return Ok(());
    }
    let error = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if error.is_empty() {
        return Err(format!(
            "CoreSimulator keyboard helper exited with {}",
            output.status
        ));
    }
    Err(error)
}

fn helper_request() -> Option<Result<(String, bool), String>> {
    let mut arguments = std::env::args().skip(1);
    if arguments.next().as_deref() != Some("--vmux-core-simulator-keyboard") {
        return None;
    }
    let Some(udid) = arguments.next() else {
        return Some(Err(
            "CoreSimulator keyboard helper requires a device".to_string()
        ));
    };
    let enabled = match arguments.next().as_deref() {
        Some("enabled") => true,
        Some("disabled") => false,
        _ => {
            return Some(Err(
                "CoreSimulator keyboard helper requires a keyboard state".to_string(),
            ));
        }
    };
    if arguments.next().is_some() {
        return Some(Err(
            "CoreSimulator keyboard helper received unexpected arguments".to_string(),
        ));
    }
    Some(Ok((udid, enabled)))
}

fn set_hardware_keyboard(udid: &str, enabled: bool) -> Result<(), String> {
    autoreleasepool(|_| {
        load_framework()?;
        let device = find_device(udid)?;
        let selector = sel!(setHardwareKeyboardEnabled:keyboardType:error:);
        if !device.class().responds_to(selector) {
            return Err("CoreSimulator hardware keyboard control is unavailable".to_string());
        }
        let result: Result<(), Retained<NSError>> = unsafe {
            msg_send![
                &*device,
                setHardwareKeyboardEnabled: enabled,
                keyboardType: 0u8,
                error: _
            ]
        };
        result.map_err(|error| error.localizedDescription().to_string())
    })
}

fn find_device(udid: &str) -> Result<Retained<AnyObject>, String> {
    let class = AnyClass::get(c"SimServiceContext")
        .ok_or_else(|| "CoreSimulator service context is unavailable".to_string())?;
    if class
        .class_method(sel!(sharedServiceContextForDeveloperDir:error:))
        .is_none()
    {
        return Err("CoreSimulator service context lookup is unavailable".to_string());
    }
    let developer_directory = NSString::from_str(&developer_directory()?);
    let mut context_error: Option<Retained<NSError>> = None;
    let context: Option<Retained<AnyObject>> = unsafe {
        msg_send![
            class,
            sharedServiceContextForDeveloperDir: &*developer_directory,
            error: &mut context_error
        ]
    };
    let context = context
        .ok_or_else(|| core_simulator_error("could not connect to CoreSimulator", context_error))?;
    if !context
        .class()
        .responds_to(sel!(defaultDeviceSetWithError:))
    {
        return Err("CoreSimulator device set lookup is unavailable".to_string());
    }
    let mut device_set_error: Option<Retained<NSError>> = None;
    let device_set: Option<Retained<AnyObject>> =
        unsafe { msg_send![&*context, defaultDeviceSetWithError: &mut device_set_error] };
    let device_set = device_set.ok_or_else(|| {
        core_simulator_error("could not load CoreSimulator devices", device_set_error)
    })?;
    if !device_set.class().responds_to(sel!(devices)) {
        return Err("CoreSimulator device listing is unavailable".to_string());
    }
    let devices: Retained<NSArray<AnyObject>> = unsafe { msg_send![&*device_set, devices] };
    for device in devices.iter() {
        if !device.class().responds_to(sel!(UDID)) {
            continue;
        }
        let identifier: Option<Retained<NSUUID>> = unsafe { msg_send![&*device, UDID] };
        if identifier.is_some_and(|identifier| identifier.UUIDString().to_string() == udid) {
            return Ok(device.clone());
        }
    }
    Err(format!("iOS Simulator {udid} is unavailable"))
}

fn developer_directory() -> Result<String, String> {
    if let Ok(path) = std::env::var("DEVELOPER_DIR")
        && !path.trim().is_empty()
    {
        return Ok(path);
    }
    let output = Command::new("xcode-select")
        .arg("--print-path")
        .output()
        .map_err(|error| format!("could not locate Xcode: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "could not locate Xcode: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path.is_empty() {
        return Err("xcode-select returned an empty developer directory".to_string());
    }
    Ok(path)
}

fn load_framework() -> Result<(), String> {
    static LOADED: OnceLock<Result<(), String>> = OnceLock::new();
    LOADED
        .get_or_init(|| {
            let path = CString::new(
                "/Library/Developer/PrivateFrameworks/CoreSimulator.framework/CoreSimulator",
            )
            .expect("CoreSimulator path has no null bytes");
            let handle = unsafe { libc::dlopen(path.as_ptr(), libc::RTLD_NOW | libc::RTLD_GLOBAL) };
            if !handle.is_null() {
                return Ok(());
            }
            let error = unsafe { libc::dlerror() };
            if error.is_null() {
                return Err("could not load CoreSimulator".to_string());
            }
            Err(unsafe { CStr::from_ptr(error) }
                .to_string_lossy()
                .into_owned())
        })
        .clone()
}

fn core_simulator_error(prefix: &str, error: Option<Retained<NSError>>) -> String {
    match error {
        Some(error) => format!("{prefix}: {}", error.localizedDescription()),
        None => prefix.to_string(),
    }
}
