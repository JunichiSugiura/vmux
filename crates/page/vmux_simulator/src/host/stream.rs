use super::device::{Axe, SimulatorDevice};
use bevy::prelude::*;
use image::ExtendedColorType;
use image::codecs::jpeg::JpegEncoder;
use std::io::{self, Read, Write};
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, ChildStdout, Stdio};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

#[derive(Resource)]
pub struct StreamServer {
    port: u16,
}

impl StreamServer {
    const FPS: &'static str = "30";
    const JPEG_QUALITY: u8 = 95;
    const ROW_ALIGNMENT: usize = 64;
    const SCALE: f32 = 0.5;

    pub fn start(
        axe: &Axe,
        device: SimulatorDevice,
        pixels: Option<(u32, u32)>,
    ) -> io::Result<Self> {
        let axe = axe.path().to_path_buf();
        let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))?;
        let port = listener.local_addr()?.port();
        let stream = Self::shared_stream(&axe, &device, pixels);
        std::thread::Builder::new()
            .name("vmux-simulator-stream".into())
            .spawn(move || Self::accept_loop(listener, axe, device, stream))?;
        Ok(Self { port })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    fn accept_loop(
        listener: TcpListener,
        axe: PathBuf,
        device: SimulatorDevice,
        stream: Option<SharedStream>,
    ) {
        for connection in listener.incoming() {
            let Ok(socket) = connection else {
                continue;
            };
            let device = device.clone();
            let axe = axe.clone();
            let stream = stream.clone();
            let spawned = std::thread::Builder::new()
                .name("vmux-simulator-pipe".into())
                .spawn(move || Self::pipe(socket, axe, device, stream));
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
    ) {
        if Self::read_request(&mut socket).is_err() {
            return;
        }
        let Some(stream) = stream else {
            Self::pipe_screenshots(socket, axe, device);
            return;
        };
        Self::write_mjpeg(socket, stream);
    }

    fn shared_stream(
        axe: &PathBuf,
        device: &SimulatorDevice,
        pixels: Option<(u32, u32)>,
    ) -> Option<SharedStream> {
        let (pixel_width, pixel_height) = pixels?;
        let width = ((pixel_width as f32) * Self::SCALE).floor() as u32;
        let height = ((pixel_height as f32) * Self::SCALE).floor() as u32;
        if width == 0 || height == 0 {
            return None;
        }
        let stride = (width as usize * 4).next_multiple_of(Self::ROW_ALIGNMENT);
        let (mut child, stdout) = Self::spawn_bgra(axe, device)?;
        let frames = Arc::new(FrameExchange::default());
        let captured = frames.clone();
        let reader = std::thread::Builder::new()
            .name("vmux-simulator-capture".into())
            .spawn(move || {
                Self::read_bgra(stdout, height, stride, captured);
                let _ = child.kill();
                let _ = child.wait();
            });
        if reader.is_err() {
            frames.close();
            return None;
        }
        let encoded = Arc::new(EncodedFrameExchange::default());
        let published = encoded.clone();
        let encoder = std::thread::Builder::new()
            .name("vmux-simulator-encode".into())
            .spawn(move || Self::encode_frames(frames, published, width, height));
        if encoder.is_err() {
            encoded.close();
            return None;
        }
        Some(SharedStream { frames: encoded })
    }

    fn spawn_bgra(axe: &PathBuf, device: &SimulatorDevice) -> Option<(Child, ChildStdout)> {
        let mut child = std::process::Command::new(axe)
            .args(["stream-video", "--udid", &device.udid])
            .args(["--format", "bgra"])
            .args(["--fps", Self::FPS])
            .args(["--scale", &Self::SCALE.to_string()])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        let stdout = child.stdout.take()?;
        Some((child, stdout))
    }

    fn read_bgra(mut stdout: ChildStdout, height: u32, stride: usize, frames: Arc<FrameExchange>) {
        let frame_bytes = stride * height as usize;
        let mut frame = vec![0; frame_bytes];
        loop {
            if stdout.read_exact(&mut frame).is_err() {
                frames.close();
                return;
            }
            let Some(recycled) = frames.replace(frame) else {
                return;
            };
            frame = recycled;
            frame.resize(frame_bytes, 0);
        }
    }

    fn encode_frames(
        frames: Arc<FrameExchange>,
        encoded: Arc<EncodedFrameExchange>,
        width: u32,
        height: u32,
    ) {
        while let Some(frame) = frames.take() {
            let result = Self::encode_bgra(&frame, width, height);
            frames.recycle(frame);
            let Ok(frame) = result else {
                continue;
            };
            encoded.replace(frame);
        }
        encoded.close();
    }

    fn write_mjpeg(mut socket: TcpStream, stream: SharedStream) {
        let _ = socket.set_nodelay(true);
        if socket
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: multipart/x-mixed-replace; boundary=vmuxframe\r\nCache-Control: no-store\r\n\r\n",
            )
            .is_err()
        {
            return;
        }
        let mut generation = 0;
        while let Some((next_generation, encoded)) = stream.frames.after(generation) {
            generation = next_generation;
            let header = format!(
                "--vmuxframe\r\nContent-Type: image/jpeg\r\nContent-Length: {}\r\n\r\n",
                encoded.len()
            );
            if socket.write_all(header.as_bytes()).is_err()
                || socket.write_all(encoded.as_ref()).is_err()
                || socket.write_all(b"\r\n").is_err()
            {
                return;
            }
        }
    }

    fn read_request(socket: &mut TcpStream) -> io::Result<()> {
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
        if !request.starts_with(b"GET / HTTP/1.1\r\n")
            && !request.starts_with(b"GET / HTTP/1.0\r\n")
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "simulator stream expected GET /",
            ));
        }
        Ok(())
    }

    fn encode_bgra(bytes: &[u8], width: u32, height: u32) -> io::Result<Vec<u8>> {
        let stride = (width as usize * 4).next_multiple_of(Self::ROW_ALIGNMENT);
        let rgb = Self::rgb_from_bgra(bytes, width, height, stride)?;
        let mut jpeg = Vec::new();
        JpegEncoder::new_with_quality(&mut jpeg, Self::JPEG_QUALITY)
            .encode(&rgb, width, height, ExtendedColorType::Rgb8)
            .map_err(io::Error::other)?;
        Ok(jpeg)
    }

    fn rgb_from_bgra(bytes: &[u8], width: u32, height: u32, stride: usize) -> io::Result<Vec<u8>> {
        let row_bytes = width as usize * 4;
        if stride < row_bytes || bytes.len() != stride * height as usize {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "simulator frame has an unexpected size",
            ));
        }
        let mut rgb = Vec::with_capacity(width as usize * height as usize * 3);
        for row in bytes.chunks_exact(stride) {
            for pixel in row[..row_bytes].chunks_exact(4) {
                rgb.extend_from_slice(&[pixel[2], pixel[1], pixel[0]]);
            }
        }
        Ok(rgb)
    }

    fn pipe_screenshots(mut socket: TcpStream, axe: PathBuf, device: SimulatorDevice) {
        let child = std::process::Command::new(axe)
            .args(["stream-video", "--udid", &device.udid])
            .args(["--format", "mjpeg"])
            .args(["--fps", Self::FPS])
            .args(["--quality", &Self::JPEG_QUALITY.to_string()])
            .args(["--scale", &Self::SCALE.to_string()])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn();
        let Ok(mut child) = child else {
            warn!("could not start `{} stream-video`", Axe::BIN);
            return;
        };
        if let Some(mut stdout) = child.stdout.take() {
            let _ = io::copy(&mut stdout, &mut socket);
        }
        let _ = child.kill();
        let _ = child.wait();
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

#[derive(Default)]
struct FrameExchange {
    state: Mutex<FrameState>,
    ready: Condvar,
}

impl FrameExchange {
    fn replace(&self, frame: Vec<u8>) -> Option<Vec<u8>> {
        let Ok(mut state) = self.state.lock() else {
            return None;
        };
        if state.closed {
            return None;
        }
        let recycled = state.latest.replace(frame).or_else(|| state.recycled.pop());
        self.ready.notify_one();
        Some(recycled.unwrap_or_default())
    }

    fn take(&self) -> Option<Vec<u8>> {
        let mut state = self.state.lock().ok()?;
        while state.latest.is_none() && !state.closed {
            state = self.ready.wait(state).ok()?;
        }
        state.latest.take()
    }

    fn recycle(&self, frame: Vec<u8>) {
        if let Ok(mut state) = self.state.lock() {
            state.recycled.push(frame);
        }
    }

    fn close(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.closed = true;
            self.ready.notify_all();
        }
    }
}

#[derive(Default)]
struct FrameState {
    latest: Option<Vec<u8>>,
    recycled: Vec<Vec<u8>>,
    closed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bgra_frame_encodes_as_jpeg() {
        let mut bytes = vec![0; StreamServer::ROW_ALIGNMENT];
        bytes[..8].copy_from_slice(&[0, 0, 255, 255, 0, 255, 0, 255]);

        let encoded = StreamServer::encode_bgra(&bytes, 2, 1).expect("jpeg");

        assert_eq!(encoded.get(..2), Some([0xff, 0xd8].as_slice()));
        assert_eq!(
            encoded.get(encoded.len() - 2..),
            Some([0xff, 0xd9].as_slice())
        );
    }

    #[test]
    fn a_malformed_bgra_frame_is_rejected() {
        assert!(StreamServer::encode_bgra(&[0; 7], 2, 1).is_err());
    }

    #[test]
    fn core_video_row_padding_is_not_encoded_as_pixels() {
        let bytes = [
            1, 2, 3, 255, 4, 5, 6, 255, 91, 92, 93, 94, 7, 8, 9, 255, 10, 11, 12, 255, 95, 96, 97,
            98,
        ];

        assert_eq!(
            StreamServer::rgb_from_bgra(&bytes, 2, 2, 12).expect("rgb"),
            [3, 2, 1, 6, 5, 4, 9, 8, 7, 12, 11, 10]
        );
    }

    #[test]
    fn newer_frames_replace_stale_frames() {
        let frames = FrameExchange::default();

        let first_buffer = frames.replace(vec![1]).expect("buffer");
        assert!(first_buffer.is_empty());
        let second_buffer = frames.replace(vec![2]).expect("buffer");

        assert_eq!(second_buffer, vec![1]);
        assert_eq!(frames.take(), Some(vec![2]));
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
