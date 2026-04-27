use crate::protocol::{ClientMessage, DaemonMessage};
use crate::{read_message, socket_path, write_message};
use std::sync::Arc;
use tokio::io::BufReader;
use tokio::net::UnixStream;
use tokio::sync::Mutex;

/// Async client connection to the vmux daemon.
/// Wraps the Unix socket with framing/serialization.
pub struct DaemonConnection {
    reader: Mutex<BufReader<tokio::net::unix::OwnedReadHalf>>,
    writer: Mutex<tokio::net::unix::OwnedWriteHalf>,
}

impl DaemonConnection {
    /// Connect to the daemon socket.
    pub async fn connect() -> std::io::Result<Self> {
        let sock = socket_path();
        let stream = UnixStream::connect(&sock).await?;
        let (r, w) = stream.into_split();
        Ok(Self {
            reader: Mutex::new(BufReader::new(r)),
            writer: Mutex::new(w),
        })
    }

    /// Send a message to the daemon.
    pub async fn send(&self, msg: &ClientMessage) -> std::io::Result<()> {
        let mut w = self.writer.lock().await;
        write_message!(&mut *w, msg)
    }

    /// Receive a message from the daemon. Returns None on disconnect.
    pub async fn recv(&self) -> std::io::Result<Option<DaemonMessage>> {
        let mut r = self.reader.lock().await;
        read_message!(&mut *r, DaemonMessage)
    }
}

/// Non-async handle for Bevy systems to communicate with the daemon.
/// Uses a background tokio task and std mpsc channels.
pub struct DaemonHandle {
    cmd_tx: std::sync::mpsc::Sender<ClientMessage>,
    msg_rx: std::sync::Mutex<std::sync::mpsc::Receiver<DaemonMessage>>,
    _runtime: Arc<tokio::runtime::Runtime>,
}

impl DaemonHandle {
    /// Spawn a background connection to the daemon.
    /// Returns `None` if the daemon is not running.
    pub fn connect() -> Option<Self> {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .ok()?;
        let rt = Arc::new(rt);

        let (cmd_tx, cmd_rx) = std::sync::mpsc::channel::<ClientMessage>();
        let (msg_tx, msg_rx) = std::sync::mpsc::channel::<DaemonMessage>();

        let rt2 = Arc::clone(&rt);
        std::thread::Builder::new()
            .name("daemon-client".into())
            .spawn(move || {
                rt2.block_on(async move {
                    let conn = match DaemonConnection::connect().await {
                        Ok(c) => Arc::new(c),
                        Err(e) => {
                            eprintln!("daemon connect failed: {e}");
                            return;
                        }
                    };

                    // Reader task: daemon -> msg_tx
                    let conn_r = Arc::clone(&conn);
                    let msg_tx2 = msg_tx.clone();
                    tokio::spawn(async move {
                        loop {
                            match conn_r.recv().await {
                                Ok(Some(msg)) => {
                                    if msg_tx2.send(msg).is_err() {
                                        break;
                                    }
                                }
                                Ok(None) => break,
                                Err(_) => break,
                            }
                        }
                    });

                    // Writer task: cmd_rx -> daemon
                    loop {
                        let msg = match cmd_rx.recv() {
                            Ok(m) => m,
                            Err(_) => break,
                        };
                        if conn.send(&msg).await.is_err() {
                            break;
                        }
                    }
                });
            })
            .ok()?;

        Some(Self {
            cmd_tx,
            msg_rx: std::sync::Mutex::new(msg_rx),
            _runtime: rt,
        })
    }

    /// Send a command to the daemon (non-blocking).
    pub fn send(&self, msg: ClientMessage) {
        let _ = self.cmd_tx.send(msg);
    }

    /// Drain all available messages from the daemon (non-blocking).
    pub fn drain(&self) -> Vec<DaemonMessage> {
        let rx = self.msg_rx.lock().unwrap();
        let mut msgs = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            msgs.push(msg);
        }
        msgs
    }
}
