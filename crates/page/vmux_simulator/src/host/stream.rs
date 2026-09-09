use super::device::{Axe, SimulatorDevice};
use bevy::prelude::*;
use std::io::{self, Read, Write};
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, ChildStdout, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

#[derive(Component)]
pub struct StreamServer {
    port: u16,
    capability: String,
    stop: Arc<AtomicBool>,
    frames: Option<Arc<EncodedFrameExchange>>,
    capture: Option<Arc<Mutex<Child>>>,
}

impl StreamServer {
    const FPS: &'static str = "30";
    const JPEG_QUALITY: &'static str = "95";
    const SCALE: &'static str = "0.5";

    pub fn start(axe: &Axe, device: SimulatorDevice) -> io::Result<Self> {
        let axe = axe.path().to_path_buf();
        let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))?;
        listener.set_nonblocking(true)?;
        let port = listener.local_addr()?.port();
        let capability = uuid::Uuid::new_v4().simple().to_string();
        let shared = Self::shared_stream(&axe, &device);
        let stream = shared.as_ref().map(|(stream, _)| stream.clone());
        let frames = stream.as_ref().map(|stream| stream.frames.clone());
        let capture = shared.map(|(_, capture)| capture);
        let stop = Arc::new(AtomicBool::new(false));
        let listener_stop = stop.clone();
        let listener_capability = capability.clone();
        let listener_thread = std::thread::Builder::new()
            .name("vmux-simulator-stream".into())
            .spawn(move || {
                Self::accept_loop(
                    listener,
                    axe,
                    device,
                    stream,
                    listener_capability,
                    listener_stop,
                )
            });
        if let Err(error) = listener_thread {
            stop.store(true, Ordering::Release);
            if let Some(frames) = &frames {
                frames.close();
            }
            if let Some(capture) = &capture {
                Self::stop_capture(capture);
            }
            return Err(error);
        }
        Ok(Self {
            port,
            capability,
            stop,
            frames,
            capture,
        })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn capability(&self) -> &str {
        &self.capability
    }

    fn accept_loop(
        listener: TcpListener,
        axe: PathBuf,
        device: SimulatorDevice,
        stream: Option<SharedStream>,
        capability: String,
        stop: Arc<AtomicBool>,
    ) {
        while !stop.load(Ordering::Acquire) {
            let socket = match listener.accept() {
                Ok((socket, _)) => socket,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(25));
                    continue;
                }
                Err(error) => {
                    warn!("simulator stream listener stopped: {error}");
                    return;
                }
            };
            let device = device.clone();
            let axe = axe.clone();
            let stream = stream.clone();
            let capability = capability.clone();
            let spawned = std::thread::Builder::new()
                .name("vmux-simulator-pipe".into())
                .spawn(move || Self::pipe(socket, axe, device, stream, &capability));
            if spawned.is_err() {
                warn!("could not spawn a stream thread");
            }
        }
    }

    fn pipe(
        mut socket: TcpStream,
        axe: PathBuf,
        device: SimulatorDevice,
        stream: Option<SharedStream>,
        capability: &str,
    ) {
        if Self::read_request(&mut socket, capability).is_err() {
            return;
        }
        let Some(stream) = stream else {
            Self::pipe_live_mjpeg(socket, axe, device);
            return;
        };
        Self::write_mjpeg(socket, stream);
    }

    fn shared_stream(
        axe: &PathBuf,
        device: &SimulatorDevice,
    ) -> Option<(SharedStream, Arc<Mutex<Child>>)> {
        let (child, stdout) = Self::spawn_mjpeg(axe, device)?;
        let capture = Arc::new(Mutex::new(child));
        let frames = Arc::new(EncodedFrameExchange::default());
        let published = frames.clone();
        let reader_capture = capture.clone();
        let reader = std::thread::Builder::new()
            .name("vmux-simulator-capture".into())
            .spawn(move || {
                Self::read_mjpeg(stdout, published);
                Self::stop_capture(&reader_capture);
            });
        if reader.is_err() {
            frames.close();
            Self::stop_capture(&capture);
            return None;
        }
        Some((SharedStream { frames }, capture))
    }

    fn spawn_mjpeg(axe: &PathBuf, device: &SimulatorDevice) -> Option<(Child, ChildStdout)> {
        let mut child = std::process::Command::new(axe)
            .args(["stream-video", "--udid", &device.udid])
            .args(["--format", "mjpeg"])
            .args(["--fps", Self::FPS])
            .args(["--quality", Self::JPEG_QUALITY])
            .args(["--scale", Self::SCALE])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        let stdout = child.stdout.take()?;
        Some((child, stdout))
    }

    fn read_mjpeg(mut stdout: ChildStdout, frames: Arc<EncodedFrameExchange>) {
        let mut parser = MjpegFrames::default();
        let mut chunk = [0; 64 * 1024];
        loop {
            let count = match stdout.read(&mut chunk) {
                Ok(0) | Err(_) => {
                    frames.close();
                    return;
                }
                Ok(count) => count,
            };
            let parsed = match parser.push(&chunk[..count]) {
                Ok(parsed) => parsed,
                Err(error) => {
                    warn!("simulator MJPEG stream failed: {error}");
                    frames.close();
                    return;
                }
            };
            for frame in parsed {
                frames.replace(frame);
            }
        }
    }

    fn write_mjpeg(mut socket: TcpStream, stream: SharedStream) {
        let _ = socket.set_nodelay(true);
        if Self::write_header(&mut socket).is_err() {
            return;
        }
        let mut generation = 0;
        while let Some((next_generation, encoded)) = stream.frames.after(generation) {
            generation = next_generation;
            if Self::write_frame(&mut socket, &encoded).is_err() {
                return;
            }
        }
    }

    fn read_request(socket: &mut TcpStream, capability: &str) -> io::Result<()> {
        socket.set_read_timeout(Some(Duration::from_secs(2)))?;
        let mut request = Vec::with_capacity(1024);
        let mut chunk = [0; 1024];
        while request.len() < 8192 {
            let count = socket.read(&mut chunk)?;
            if count == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "simulator stream request ended early",
                ));
            }
            request.extend_from_slice(&chunk[..count]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        socket.set_read_timeout(None)?;
        if !Self::request_has_capability(&request, capability) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "simulator stream capability is missing or invalid",
            ));
        }
        Ok(())
    }

    fn request_has_capability(request: &[u8], capability: &str) -> bool {
        let expected_11 = format!("GET /{capability} HTTP/1.1\r\n");
        let expected_10 = format!("GET /{capability} HTTP/1.0\r\n");
        request.starts_with(expected_11.as_bytes()) || request.starts_with(expected_10.as_bytes())
    }

    fn write_header(socket: &mut TcpStream) -> io::Result<()> {
        socket.write_all(
            b"HTTP/1.1 200 OK\r\nContent-Type: multipart/x-mixed-replace; boundary=vmuxframe\r\nCache-Control: no-store\r\n\r\n",
        )
    }

    fn write_frame(socket: &mut TcpStream, encoded: &[u8]) -> io::Result<()> {
        let header = format!(
            "--vmuxframe\r\nContent-Type: image/jpeg\r\nContent-Length: {}\r\n\r\n",
            encoded.len()
        );
        socket.write_all(header.as_bytes())?;
        socket.write_all(encoded)?;
        socket.write_all(b"\r\n")
    }

    fn pipe_live_mjpeg(mut socket: TcpStream, axe: PathBuf, device: SimulatorDevice) {
        let Some((mut child, mut stdout)) = Self::spawn_mjpeg(&axe, &device) else {
            return;
        };
        if Self::write_header(&mut socket).is_err() {
            let _ = child.kill();
            let _ = child.wait();
            return;
        }
        let mut parser = MjpegFrames::default();
        let mut chunk = [0; 64 * 1024];
        'stream: loop {
            let count = match stdout.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(count) => count,
            };
            let Ok(frames) = parser.push(&chunk[..count]) else {
                break;
            };
            for frame in frames {
                if Self::write_frame(&mut socket, &frame).is_err() {
                    break 'stream;
                }
            }
        }
        let _ = child.kill();
        let _ = child.wait();
    }

    fn stop_capture(capture: &Mutex<Child>) {
        let Ok(mut child) = capture.lock() else {
            return;
        };
        let _ = child.kill();
        let _ = child.wait();
    }
}

impl Drop for StreamServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(frames) = &self.frames {
            frames.close();
        }
        if let Some(capture) = &self.capture {
            Self::stop_capture(capture);
        }
    }
}

#[derive(Default)]
struct MjpegFrames {
    bytes: Vec<u8>,
}

impl MjpegFrames {
    const MAX_FRAME_BYTES: usize = 32 * 1024 * 1024;

    fn push(&mut self, chunk: &[u8]) -> io::Result<Vec<Vec<u8>>> {
        self.bytes.extend_from_slice(chunk);
        let mut frames = Vec::new();
        loop {
            let Some(start) = Self::marker(&self.bytes, [0xff, 0xd8]) else {
                if self.bytes.len() > Self::MAX_FRAME_BYTES {
                    self.bytes.clear();
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "simulator MJPEG stream has no frame start",
                    ));
                }
                if self.bytes.last() == Some(&0xff) {
                    self.bytes.drain(..self.bytes.len() - 1);
                } else {
                    self.bytes.clear();
                }
                return Ok(frames);
            };
            if start > 0 {
                self.bytes.drain(..start);
            }
            let Some(end) = Self::marker(&self.bytes[2..], [0xff, 0xd9]) else {
                if self.bytes.len() > Self::MAX_FRAME_BYTES {
                    self.bytes.clear();
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "simulator MJPEG frame is too large",
                    ));
                }
                return Ok(frames);
            };
            let end = end + 4;
            frames.push(self.bytes.drain(..end).collect());
        }
    }

    fn marker(bytes: &[u8], marker: [u8; 2]) -> Option<usize> {
        bytes.windows(2).position(|window| window == marker)
    }
}

#[derive(Clone)]
struct SharedStream {
    frames: Arc<EncodedFrameExchange>,
}

#[derive(Default)]
struct EncodedFrameExchange {
    state: Mutex<EncodedFrameState>,
    ready: Condvar,
}

impl EncodedFrameExchange {
    fn replace(&self, frame: Vec<u8>) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        if state.closed {
            return;
        }
        state.latest = Some(Arc::from(frame));
        state.generation = state.generation.wrapping_add(1).max(1);
        self.ready.notify_all();
    }

    fn after(&self, generation: u64) -> Option<(u64, Arc<[u8]>)> {
        let mut state = self.state.lock().ok()?;
        while state.generation == generation && !state.closed {
            state = self.ready.wait(state).ok()?;
        }
        if state.closed {
            return None;
        }
        Some((state.generation, state.latest.as_ref()?.clone()))
    }

    fn close(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.closed = true;
            self.ready.notify_all();
        }
    }
}

#[derive(Default)]
struct EncodedFrameState {
    latest: Option<Arc<[u8]>>,
    generation: u64,
    closed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mjpeg_frames_survive_chunk_boundaries() {
        let mut parser = MjpegFrames::default();

        assert!(parser.push(&[9, 0xff]).unwrap().is_empty());
        assert!(parser.push(&[0xd8, 1, 2, 0xff]).unwrap().is_empty());
        assert_eq!(
            parser.push(&[0xd9, 0xff, 0xd8, 3, 0xff, 0xd9]).unwrap(),
            [
                vec![0xff, 0xd8, 1, 2, 0xff, 0xd9],
                vec![0xff, 0xd8, 3, 0xff, 0xd9]
            ]
        );
    }

    #[test]
    fn stream_requests_require_the_capability() {
        assert!(StreamServer::request_has_capability(
            b"GET /secret HTTP/1.1\r\nHost: localhost\r\n\r\n",
            "secret"
        ));
        assert!(!StreamServer::request_has_capability(
            b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n",
            "secret"
        ));
        assert!(!StreamServer::request_has_capability(
            b"GET /secret-extra HTTP/1.1\r\nHost: localhost\r\n\r\n",
            "secret"
        ));
    }

    #[test]
    fn encoded_frames_are_shared_between_stream_clients() {
        let frames = EncodedFrameExchange::default();
        frames.replace(vec![1, 2, 3]);

        let (_, first) = frames.after(0).expect("first client");
        let (_, second) = frames.after(0).expect("second client");

        assert!(Arc::ptr_eq(&first, &second));
    }
}
