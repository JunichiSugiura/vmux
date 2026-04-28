use serde::{Deserialize, Serialize};

/// Event name for session list updates (host → webview).
pub const SESSIONS_LIST_EVENT: &str = "sessions_list";

/// URL for the sessions monitor webview.
pub const SESSIONS_WEBVIEW_URL: &str = "vmux://sessions/";

/// Daemon connection status + session list, sent periodically to the webview.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionsListEvent {
    pub connected: bool,
    pub sessions: Vec<SessionEntry>,
}

/// A single daemon session's metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionEntry {
    pub id: String,
    pub shell: String,
    pub cwd: String,
    pub cols: u16,
    pub rows: u16,
    pub pid: u32,
    pub uptime_secs: u64,
    /// Whether a GUI terminal is attached to this session.
    pub attached: bool,
    /// Last few lines of terminal output for preview.
    pub preview_lines: Vec<PreviewLine>,
}

/// A simplified terminal line for the preview.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewLine {
    pub text: String,
}
