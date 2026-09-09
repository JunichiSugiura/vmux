use objc2::rc::{Retained, autoreleasepool};
use objc2::runtime::{AnyClass, AnyObject};
use objc2::{msg_send, sel};
use objc2_foundation::{NSArray, NSError, NSString, NSUUID};
use std::ffi::{CStr, CString};
use std::process::Command;
use std::sync::OnceLock;

pub struct CoreSimulatorDevice;

impl CoreSimulatorDevice {
    pub fn set_hardware_keyboard_enabled(udid: &str, enabled: bool) -> Result<(), String> {
        autoreleasepool(|_| {
            Self::load_framework()?;
            let device = Self::find(udid)?;
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

    fn find(udid: &str) -> Result<Retained<AnyObject>, String> {
        let class = AnyClass::get(c"SimServiceContext")
            .ok_or_else(|| "CoreSimulator service context is unavailable".to_string())?;
        if class
            .class_method(sel!(sharedServiceContextForDeveloperDir:error:))
            .is_none()
        {
            return Err("CoreSimulator service context lookup is unavailable".to_string());
        }
        let developer_directory = NSString::from_str(&Self::developer_directory()?);
        let mut context_error: Option<Retained<NSError>> = None;
        let context: Option<Retained<AnyObject>> = unsafe {
            msg_send![
                class,
                sharedServiceContextForDeveloperDir: &*developer_directory,
                error: &mut context_error
            ]
        };
        let context = context
            .ok_or_else(|| Self::error("could not connect to CoreSimulator", context_error))?;
        if !context
            .class()
            .responds_to(sel!(defaultDeviceSetWithError:))
        {
            return Err("CoreSimulator device set lookup is unavailable".to_string());
        }
        let mut device_set_error: Option<Retained<NSError>> = None;
        let device_set: Option<Retained<AnyObject>> =
            unsafe { msg_send![&*context, defaultDeviceSetWithError: &mut device_set_error] };
        let device_set = device_set
            .ok_or_else(|| Self::error("could not load CoreSimulator devices", device_set_error))?;
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
                let handle =
                    unsafe { libc::dlopen(path.as_ptr(), libc::RTLD_NOW | libc::RTLD_GLOBAL) };
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

    fn error(prefix: &str, error: Option<Retained<NSError>>) -> String {
        match error {
            Some(error) => format!("{prefix}: {}", error.localizedDescription()),
            None => prefix.to_string(),
        }
    }
}
