use super::device::{Axe, SimulatorDevice};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io::{self, Read, Write};
use std::net::Shutdown;
use std::os::fd::AsRawFd;
use std::os::unix::fs::{DirBuilderExt, FileTypeExt, MetadataExt, OpenOptionsExt};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

#[derive(Resource)]
pub struct HidBroker {
    sender: Sender<HidRequest>,
}

impl HidBroker {
    pub fn start(axe: &Axe, device: &SimulatorDevice) -> io::Result<Self> {
        let client = HidBrokerClient::new(axe.path().to_path_buf(), device.udid.clone())?;
        let (sender, receiver) = mpsc::channel();
        std::thread::Builder::new()
            .name("vmux-simulator-input".into())
            .spawn(move || client.run(receiver))?;
        Ok(Self { sender })
    }

    pub fn dispatch(&self, request: HidRequest) {
        if self.sender.send(request).is_err() {
            error!("simulator input worker stopped");
        }
    }
}

pub struct HidRequest {
    primitives: Vec<HidPrimitive>,
    fallback: Vec<String>,
}

impl HidRequest {
    pub fn tap(point: (f32, f32)) -> Self {
        Self {
            primitives: vec![
                HidPrimitive::touch(HidKind::Down, point),
                HidPrimitive::delay(0.1),
                HidPrimitive::touch(HidKind::Up, point),
            ],
            fallback: vec![
                "tap".into(),
                "-x".into(),
                format!("{:.0}", point.0),
                "-y".into(),
                format!("{:.0}", point.1),
            ],
        }
    }

    pub fn swipe(from: (f32, f32), to: (f32, f32)) -> Self {
        Self {
            primitives: Self::swipe_primitives(from, to),
            fallback: vec![
                "swipe".into(),
                "--start-x".into(),
                format!("{:.0}", from.0),
                "--start-y".into(),
                format!("{:.0}", from.1),
                "--end-x".into(),
                format!("{:.0}", to.0),
                "--end-y".into(),
                format!("{:.0}", to.1),
                "--duration".into(),
                "0.25".into(),
            ],
        }
    }

    pub fn bottom_edge(points: (f32, f32)) -> Self {
        let from = (points.0 * 0.5, (points.1 - 2.0).max(0.0));
        let to = (points.0 * 0.5, points.1 * 0.55);
        Self {
            primitives: Self::swipe_primitives(from, to),
            fallback: vec![
                "gesture".into(),
                "swipe-from-bottom-edge".into(),
                "--screen-width".into(),
                format!("{:.0}", points.0),
                "--screen-height".into(),
                format!("{:.0}", points.1),
                "--duration".into(),
                "0.25".into(),
            ],
        }
    }

    fn swipe_primitives(from: (f32, f32), to: (f32, f32)) -> Vec<HidPrimitive> {
        const STEPS: usize = 6;
        let mut primitives = Vec::with_capacity(STEPS * 2 + 2);
        primitives.push(HidPrimitive::touch(HidKind::Down, from));
        for step in 1..=STEPS {
            let progress = step as f32 / STEPS as f32;
            let point = (
                from.0 + (to.0 - from.0) * progress,
                from.1 + (to.1 - from.1) * progress,
            );
            primitives.push(HidPrimitive::delay(0.012));
            primitives.push(HidPrimitive::touch(HidKind::Down, point));
        }
        primitives.push(HidPrimitive::touch(HidKind::Up, to));
        primitives
    }
}

#[derive(Serialize)]
struct HidBrokerRequest {
    primitives: Vec<HidPrimitive>,
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
enum HidKind {
    Down,
    Up,
    Delay,
}

#[derive(Serialize)]
struct HidPrimitive {
    kind: HidKind,
    x: Option<f64>,
    y: Option<f64>,
    duration: Option<f64>,
}

impl HidPrimitive {
    fn touch(kind: HidKind, point: (f32, f32)) -> Self {
        Self {
            kind,
            x: Some(point.0 as f64),
            y: Some(point.1 as f64),
            duration: None,
        }
    }

    fn delay(duration: f64) -> Self {
        Self {
            kind: HidKind::Delay,
            x: None,
            y: None,
            duration: Some(duration),
        }
    }
}

#[derive(Deserialize)]
struct HidBrokerHandshake {
    ready: bool,
}

#[derive(Deserialize)]
struct HidBrokerResponse {
    error: Option<String>,
}

struct HidBrokerClient {
    axe: PathBuf,
    udid: String,
    endpoint: PathBuf,
}

impl HidBrokerClient {
    const MAX_MESSAGE_BYTES: usize = 64 * 1024;
    const STARTUP_TIMEOUT: Duration = Duration::from_secs(30);

    fn new(axe: PathBuf, udid: String) -> io::Result<Self> {
        let developer = Self::developer_directory()?;
        let temporary = std::env::temp_dir();
        let uid = unsafe { libc::getuid() };
        let endpoint = Self::endpoint_path(&udid, &developer, &temporary, uid);
        Self::ensure_private_directory(
            endpoint
                .parent()
                .ok_or_else(|| io::Error::other("AXe HID endpoint has no directory"))?,
            uid,
        )?;
        Ok(Self {
            axe,
            udid,
            endpoint,
        })
    }

    fn run(self, receiver: Receiver<HidRequest>) {
        if let Err(error) = self.exchange(Vec::new()) {
            warn!("could not warm simulator input: {error}");
        }
        while let Ok(request) = receiver.recv() {
            if let Err(error) = self.exchange(request.primitives) {
                warn!("fast simulator input failed: {error}");
                self.fallback(request.fallback);
            }
        }
    }

    fn exchange(&self, primitives: Vec<HidPrimitive>) -> io::Result<()> {
        let mut stream = self.ready_stream()?;
        let mut data = serde_json::to_vec(&HidBrokerRequest { primitives })?;
        data.push(b'\n');
        stream.write_all(&data)?;
        let response: HidBrokerResponse =
            serde_json::from_slice(&Self::read_message(&mut stream)?)?;
        if let Some(error) = response.error {
            return Err(io::Error::other(error));
        }
        Ok(())
    }

    fn ready_stream(&self) -> io::Result<UnixStream> {
        if let Ok(stream) = self.connect() {
            return Ok(stream);
        }
        let deadline = Instant::now() + Self::STARTUP_TIMEOUT;
        let _startup_lock = self.acquire_startup_lock(deadline)?;
        if let Ok(stream) = self.connect() {
            return Ok(stream);
        }
        let mut spawned = false;
        while Instant::now() < deadline {
            if !spawned && !self.broker_is_alive()? {
                self.remove_stale_endpoint()?;
                self.spawn();
                spawned = true;
            }
            std::thread::sleep(Duration::from_millis(50));
            if let Ok(stream) = self.connect() {
                return Ok(stream);
            }
        }
        Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "AXe HID broker did not become ready",
        ))
    }

    fn connect(&self) -> io::Result<UnixStream> {
        let mut stream = UnixStream::connect(&self.endpoint)?;
        stream.set_read_timeout(Some(Duration::from_secs(2)))?;
        stream.set_write_timeout(Some(Duration::from_secs(2)))?;
        let handshake: HidBrokerHandshake =
            serde_json::from_slice(&Self::read_message(&mut stream)?)?;
        if !handshake.ready {
            return Err(io::Error::new(
                io::ErrorKind::ConnectionAborted,
                "AXe HID broker session is stale",
            ));
        }
        Ok(stream)
    }

    fn spawn(&self) {
        let mut command = std::process::Command::new(&self.axe);
        command
            .args(["hid-broker", "--udid", &self.udid])
            .env("AXE_HID_STABILIZATION_MS", "0")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        Axe::run_detached(command);
    }

    fn fallback(&self, arguments: Vec<String>) {
        let mut command = std::process::Command::new(&self.axe);
        command.args(arguments).args(["--udid", &self.udid]);
        Axe::run_detached(command);
    }

    fn read_message(stream: &mut UnixStream) -> io::Result<Vec<u8>> {
        let mut data = Vec::new();
        let mut byte = [0u8; 1];
        while data.len() <= Self::MAX_MESSAGE_BYTES {
            let count = stream.read(&mut byte)?;
            if count == 0 {
                break;
            }
            if byte[0] == b'\n' {
                return Ok(data);
            }
            data.push(byte[0]);
        }
        let _ = stream.shutdown(Shutdown::Both);
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "AXe HID broker message is missing or too large",
        ))
    }

    fn acquire_startup_lock(&self, deadline: Instant) -> io::Result<File> {
        let lock = Self::open_private_lock(&self.suffixed_endpoint(".lock"))?;
        loop {
            if Self::try_lock(&lock)? {
                return Ok(lock);
            }
            if Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "AXe HID broker startup lock timed out",
                ));
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    fn broker_is_alive(&self) -> io::Result<bool> {
        let lock = Self::open_private_lock(&self.suffixed_endpoint(".lifetime.lock"))?;
        Ok(!Self::try_lock(&lock)?)
    }

    fn remove_stale_endpoint(&self) -> io::Result<()> {
        let metadata = match fs::symlink_metadata(&self.endpoint) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error),
        };
        let uid = unsafe { libc::getuid() };
        if !metadata.file_type().is_socket() || metadata.uid() != uid {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "refusing to remove an unowned AXe HID endpoint",
            ));
        }
        fs::remove_file(&self.endpoint)
    }

    fn suffixed_endpoint(&self, suffix: &str) -> PathBuf {
        let mut path = self.endpoint.as_os_str().to_os_string();
        path.push(suffix);
        path.into()
    }

    fn open_private_lock(path: &Path) -> io::Result<File> {
        let lock = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .mode(0o600)
            .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW)
            .open(path)?;
        let metadata = lock.metadata()?;
        let uid = unsafe { libc::getuid() };
        if !metadata.file_type().is_file() || metadata.uid() != uid || metadata.mode() & 0o077 != 0
        {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "AXe HID lock is not a private owned file",
            ));
        }
        Ok(lock)
    }

    fn try_lock(lock: &File) -> io::Result<bool> {
        loop {
            let result = unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
            if result == 0 {
                return Ok(true);
            }
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            if error.kind() == io::ErrorKind::WouldBlock {
                return Ok(false);
            }
            return Err(error);
        }
    }

    fn ensure_private_directory(path: &Path, uid: u32) -> io::Result<()> {
        match DirBuilder::new().mode(0o700).create(path) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
        let metadata = fs::symlink_metadata(path)?;
        if !metadata.file_type().is_dir() || metadata.uid() != uid || metadata.mode() & 0o077 != 0 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "AXe HID directory is not private and owned",
            ));
        }
        Ok(())
    }

    fn developer_directory() -> io::Result<PathBuf> {
        if let Some(path) = std::env::var_os("DEVELOPER_DIR") {
            return fs::canonicalize(path);
        }
        let output = std::process::Command::new("xcode-select")
            .arg("-p")
            .output()?;
        if !output.status.success() {
            return Err(io::Error::other("xcode-select could not resolve Xcode"));
        }
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        fs::canonicalize(path)
    }

    fn endpoint_path(udid: &str, developer: &Path, temporary: &Path, uid: u32) -> PathBuf {
        let directory = temporary.join(format!("axe-hid-{uid}"));
        let simulator = Self::fnv1a64(udid);
        let developer = Self::fnv1a64(&developer.to_string_lossy());
        directory.join(format!("{simulator:x}-{developer:x}-v2.sock"))
    }

    fn fnv1a64(value: &str) -> u64 {
        let mut hash = 14_695_981_039_346_656_037u64;
        for byte in value.bytes() {
            hash = (hash ^ u64::from(byte)).wrapping_mul(1_099_511_628_211);
        }
        hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_matches_the_axe_protocol() {
        let endpoint = HidBrokerClient::endpoint_path(
            "174D774A-1F21-455C-AB54-AF19D513988A",
            Path::new("/Applications/Xcode.app/Contents/Developer"),
            Path::new("/private/var/folders/example/T"),
            501,
        );

        assert_eq!(
            endpoint.file_name().and_then(|name| name.to_str()),
            Some("8924da6ea82aea1b-8987718aa6246ff8-v2.sock")
        );
    }

    #[test]
    fn tap_uses_one_connection_with_a_real_hold() {
        let request = HidRequest::tap((120.0, 240.0));
        let json = serde_json::to_value(HidBrokerRequest {
            primitives: request.primitives,
        })
        .expect("request");
        let primitives = json["primitives"].as_array().expect("primitives");

        assert_eq!(primitives.len(), 3);
        assert_eq!(primitives[0]["kind"], "down");
        assert_eq!(primitives[1]["kind"], "delay");
        assert_eq!(primitives[1]["duration"], 0.1);
        assert_eq!(primitives[2]["kind"], "up");
    }
}
