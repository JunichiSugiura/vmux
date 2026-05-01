pub mod client;
pub mod framing;
pub mod process;
pub mod protocol;
pub mod server;

use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// Directory for service runtime files (socket, pid, log).
pub fn service_dir() -> PathBuf {
    let home = std::env::var_os("HOME").expect("HOME not set");
    PathBuf::from(home).join("Library/Application Support/Vmux/services")
}

/// Path to the Unix domain socket.
pub fn socket_path() -> PathBuf {
    service_dir().join("vmux.sock")
}

/// Path to the PID file.
pub fn pid_path() -> PathBuf {
    service_dir().join("service.pid")
}

/// Path to the service executable identity file.
pub fn service_identity_path() -> PathBuf {
    service_dir().join("service.identity")
}

/// Identity for the current executable. Changes when the binary path, size,
/// or modification timestamp changes.
pub fn current_executable_identity() -> std::io::Result<String> {
    executable_identity_for_path(&std::env::current_exe()?)
}

/// Write the current executable identity for a service process.
pub fn write_service_identity() -> std::io::Result<()> {
    std::fs::write(service_identity_path(), current_executable_identity()?)
}

pub(crate) fn executable_identity_for_path(path: &Path) -> std::io::Result<String> {
    let path = std::fs::canonicalize(path)?;
    let metadata = std::fs::metadata(&path)?;
    let modified = metadata
        .modified()?
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    Ok(format!(
        "{}\n{}\n{modified}",
        path.display(),
        metadata.len()
    ))
}

pub(crate) fn service_identity_matches(recorded: &str, current: &str) -> bool {
    recorded.trim() == current.trim()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn executable_identity_changes_when_file_changes() {
        let path = std::env::temp_dir().join(format!("vmux-identity-test-{}", std::process::id()));
        {
            let mut file = std::fs::File::create(&path).expect("create identity test file");
            file.write_all(b"old").expect("write old identity bytes");
        }
        let old_identity = executable_identity_for_path(&path).expect("old identity");

        std::thread::sleep(std::time::Duration::from_millis(2));
        {
            let mut file = std::fs::File::create(&path).expect("rewrite identity test file");
            file.write_all(b"newer").expect("write new identity bytes");
        }
        let new_identity = executable_identity_for_path(&path).expect("new identity");
        let _ = std::fs::remove_file(&path);

        assert_ne!(old_identity, new_identity);
    }

    #[test]
    fn service_identity_match_requires_exact_record() {
        assert!(service_identity_matches("a\n1\n2\n", "a\n1\n2"));
        assert!(!service_identity_matches("a\n1\n2", "a\n1\n3"));
    }
}
