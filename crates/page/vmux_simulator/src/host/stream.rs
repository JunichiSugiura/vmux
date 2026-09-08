use super::device::{Axe, SimulatorDevice};
use bevy::prelude::*;
use image::ExtendedColorType;
use image::codecs::jpeg::JpegEncoder;
use std::io::{self, Read, Write};
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, ChildStdout, Stdio};
use std::sync::{Arc, Condvar, Mutex};

#[derive(Resource)]
pub struct StreamServer {
    port: u16,
}

impl StreamServer {
    const FPS: &'static str = "30";
    const JPEG_QUALITY: u8 = 80;
    const SCALE: f32 = 0.5;

    pub fn start(
        axe: &Axe,
        device: SimulatorDevice,
        pixels: Option<(u32, u32)>,
    ) -> io::Result<Self> {
        let axe = axe.path().to_path_buf();
        let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))?;
        let port = listener.local_addr()?.port();
        std::thread::Builder::new()
            .name("vmux-simulator-stream".into())
            .spawn(move || Self::accept_loop(listener, axe, device, pixels))?;
        Ok(Self { port })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    fn accept_loop(
        listener: TcpListener,
        axe: PathBuf,
        device: SimulatorDevice,
        pixels: Option<(u32, u32)>,
    ) {
        for connection in listener.incoming() {
            let Ok(socket) = connection else {
                continue;
            };
            let device = device.clone();
            let axe = axe.clone();
            let spawned = std::thread::Builder::new()
                .name("vmux-simulator-pipe".into())
                .spawn(move || Self::pipe(socket, axe, device, pixels));
            if spawned.is_err() {
                warn!("could not spawn a stream thread");
            }
        }
    }

    fn pipe(socket: TcpStream, axe: PathBuf, device: SimulatorDevice, pixels: Option<(u32, u32)>) {
        let Some((pixel_width, pixel_height)) = pixels else {
            Self::pipe_screenshots(socket, axe, device);
            return;
        };
        let width = ((pixel_width as f32) * Self::SCALE).round() as u32;
        let height = ((pixel_height as f32) * Self::SCALE).round() as u32;
        if width == 0 || height == 0 {
            Self::pipe_screenshots(socket, axe, device);
            return;
        }
        let Some((mut child, stdout)) = Self::spawn_bgra(&axe, &device) else {
            Self::pipe_screenshots(socket, axe, device);
            return;
        };
        let frames = Arc::new(FrameExchange::default());
        let captured = frames.clone();
        let reader = std::thread::Builder::new()
            .name("vmux-simulator-capture".into())
            .spawn(move || {
                Self::read_bgra(stdout, width, height, captured);
                let _ = child.kill();
                let _ = child.wait();
            });
        if reader.is_err() {
            frames.close();
            return;
        }
        Self::write_mjpeg(socket, width, height, frames);
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

    fn read_bgra(mut stdout: ChildStdout, width: u32, height: u32, frames: Arc<FrameExchange>) {
        let frame_bytes = width as usize * height as usize * 4;
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

    fn write_mjpeg(mut socket: TcpStream, width: u32, height: u32, frames: Arc<FrameExchange>) {
        let _ = socket.set_nodelay(true);
        if socket
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: multipart/x-mixed-replace; boundary=vmuxframe\r\nCache-Control: no-store\r\n\r\n",
            )
            .is_err()
        {
            frames.close();
            return;
        }
        while let Some(frame) = frames.take() {
            let encoded = Self::encode_bgra(&frame, width, height);
            frames.recycle(frame);
            let Ok(encoded) = encoded else {
                continue;
            };
            let header = format!(
                "--vmuxframe\r\nContent-Type: image/jpeg\r\nContent-Length: {}\r\n\r\n",
                encoded.len()
            );
            if socket.write_all(header.as_bytes()).is_err()
                || socket.write_all(&encoded).is_err()
                || socket.write_all(b"\r\n").is_err()
            {
                frames.close();
                return;
            }
        }
    }

    fn encode_bgra(bytes: &[u8], width: u32, height: u32) -> io::Result<Vec<u8>> {
        let expected = width as usize * height as usize * 4;
        if bytes.len() != expected {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "simulator frame has an unexpected size",
            ));
        }
        let mut rgb = Vec::with_capacity(width as usize * height as usize * 3);
        for pixel in bytes.chunks_exact(4) {
            rgb.extend_from_slice(&[pixel[2], pixel[1], pixel[0]]);
        }
        let mut jpeg = Vec::new();
        JpegEncoder::new_with_quality(&mut jpeg, Self::JPEG_QUALITY)
            .encode(&rgb, width, height, ExtendedColorType::Rgb8)
            .map_err(io::Error::other)?;
        Ok(jpeg)
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
        let bytes = [0, 0, 255, 255, 0, 255, 0, 255];

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
    fn newer_frames_replace_stale_frames() {
        let frames = FrameExchange::default();

        let first_buffer = frames.replace(vec![1]).expect("buffer");
        assert!(first_buffer.is_empty());
        let second_buffer = frames.replace(vec![2]).expect("buffer");

        assert_eq!(second_buffer, vec![1]);
        assert_eq!(frames.take(), Some(vec![2]));
    }
}
