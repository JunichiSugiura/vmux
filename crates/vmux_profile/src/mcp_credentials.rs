use serde::{Deserialize, Serialize};

static MCP_CREDENTIAL_ACCESS: std::sync::OnceLock<std::sync::Mutex<()>> =
    std::sync::OnceLock::new();
static MCP_CREDENTIAL_REFRESH_ACCESS: std::sync::OnceLock<std::sync::Mutex<()>> =
    std::sync::OnceLock::new();
static MCP_CREDENTIAL_REVISION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

const DEFAULT_TOKEN_LIFETIME_SECS: u64 = 3600;

#[cfg(target_os = "macos")]
const KEYCHAIN_SERVICE: &str = "ai.vmux.mcp";

#[cfg(target_os = "macos")]
static MCP_CREDENTIAL_BROKER_ACCESS: std::sync::OnceLock<std::sync::Mutex<()>> =
    std::sync::OnceLock::new();

#[cfg(target_os = "macos")]
struct McpCredentialBroker(std::path::PathBuf);

#[cfg(target_os = "macos")]
impl McpCredentialBroker {
    fn current() -> Option<Self> {
        let executable = std::env::current_exe().ok()?;
        Self::candidate(crate::build_profile(), &executable)
            .filter(|path| path.is_file())
            .map(Self)
    }

    fn candidate(profile: &str, executable: &std::path::Path) -> Option<std::path::PathBuf> {
        if profile == "dev" {
            return None;
        }
        Some(executable.parent()?.join("vmux"))
    }

    fn load(&self, account: &str) -> Result<Option<Vec<u8>>, String> {
        let output = self.run("load", account, None)?;
        if output.status.success() {
            return Ok(Some(output.stdout));
        }
        if output.status.code() == Some(2) {
            return Ok(None);
        }
        Err(Self::error(&output))
    }

    fn store(&self, account: &str, bytes: &[u8]) -> Result<(), String> {
        let output = self.run("store", account, Some(bytes))?;
        if output.status.success() {
            return Ok(());
        }
        Err(Self::error(&output))
    }

    fn remove(&self, account: &str) -> Result<(), String> {
        let output = self.run("remove", account, None)?;
        if output.status.success() {
            return Ok(());
        }
        Err(Self::error(&output))
    }

    fn run(
        &self,
        action: &str,
        account: &str,
        input: Option<&[u8]>,
    ) -> Result<std::process::Output, String> {
        use std::io::Write;
        use std::process::{Command, Stdio};

        let _access = MCP_CREDENTIAL_BROKER_ACCESS
            .get_or_init(Default::default)
            .lock()
            .map_err(|error| error.to_string())?;
        let mut command = Command::new(&self.0);
        command
            .args(["mcp-credentials", action, "--account", account])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if input.is_none() {
            return command
                .output()
                .map_err(|error| format!("failed to run packaged MCP credential broker: {error}"));
        }
        let mut child = command
            .stdin(Stdio::piped())
            .spawn()
            .map_err(|error| format!("failed to run packaged MCP credential broker: {error}"))?;
        child
            .stdin
            .take()
            .ok_or_else(|| "failed to open MCP credential broker input".to_string())?
            .write_all(input.unwrap_or_default())
            .map_err(|error| format!("failed to send MCP credentials to broker: {error}"))?;
        child
            .wait_with_output()
            .map_err(|error| format!("failed to wait for MCP credential broker: {error}"))
    }

    fn error(output: &std::process::Output) -> String {
        let error = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if error.is_empty() {
            "MCP credential broker failed".to_string()
        } else {
            error
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpOauthCredentials {
    pub token_endpoint: String,
    pub client_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_secret: Option<String>,
    pub access_token: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub expires_at: i64,
    #[serde(default)]
    pub scope: String,
    #[serde(default)]
    pub resource: String,
}

impl McpOauthCredentials {
    pub fn read_transaction<T>(operation: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
        let _access = MCP_CREDENTIAL_ACCESS
            .get_or_init(Default::default)
            .lock()
            .map_err(|error| error.to_string())?;
        operation()
    }

    pub fn write_transaction<T>(
        operation: impl FnOnce() -> Result<T, String>,
    ) -> Result<T, String> {
        let _access = MCP_CREDENTIAL_ACCESS
            .get_or_init(Default::default)
            .lock()
            .map_err(|error| error.to_string())?;
        MCP_CREDENTIAL_REVISION.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        let result = operation();
        MCP_CREDENTIAL_REVISION.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        result
    }

    pub fn refresh_transaction<T>(
        operation: impl FnOnce() -> Result<T, String>,
    ) -> Result<T, String> {
        let _access = MCP_CREDENTIAL_REFRESH_ACCESS
            .get_or_init(Default::default)
            .lock()
            .map_err(|error| error.to_string())?;
        operation()
    }

    pub fn stable_revision() -> Result<u64, String> {
        let _access = MCP_CREDENTIAL_ACCESS
            .get_or_init(Default::default)
            .lock()
            .map_err(|error| error.to_string())?;
        Ok(Self::revision())
    }

    pub fn revision() -> u64 {
        MCP_CREDENTIAL_REVISION.load(std::sync::atomic::Ordering::Acquire)
    }

    pub fn with_revision<T>(
        revision: u64,
        operation: impl FnOnce() -> T,
    ) -> Result<Option<T>, String> {
        let _access = match MCP_CREDENTIAL_ACCESS
            .get_or_init(Default::default)
            .try_lock()
        {
            Ok(access) => access,
            Err(std::sync::TryLockError::WouldBlock) => return Ok(None),
            Err(std::sync::TryLockError::Poisoned(error)) => return Err(error.to_string()),
        };
        if Self::revision() != revision || revision % 2 != 0 {
            return Ok(None);
        }
        Ok(Some(operation()))
    }

    pub fn load(server: &str) -> Result<Option<Self>, String> {
        Self::load_account(&Self::account(server))
    }

    pub fn store(&self, server: &str) -> Result<(), String> {
        let bytes = serde_json::to_vec(self).map_err(|error| error.to_string())?;
        Self::store_account(&Self::account(server), &bytes)
    }

    pub fn remove(server: &str) -> Result<(), String> {
        Self::remove_account(&Self::account(server))
    }

    pub fn expires_soon(&self) -> bool {
        self.expires_at != 0 && self.expires_at <= chrono::Utc::now().timestamp() + 60
    }

    pub fn expires_at(expires_in: Option<u64>) -> i64 {
        let lifetime = expires_in.unwrap_or(DEFAULT_TOKEN_LIFETIME_SECS);
        let lifetime = i64::try_from(lifetime).unwrap_or(i64::MAX);
        chrono::Utc::now().timestamp().saturating_add(lifetime)
    }

    pub fn authorizes(&self, resource: &str) -> bool {
        let Ok(stored) = url::Url::parse(&self.resource) else {
            return false;
        };
        let Ok(requested) = url::Url::parse(resource) else {
            return false;
        };
        stored.scheme() == "https" && requested.scheme() == "https" && stored == requested
    }

    fn account(server: &str) -> String {
        format!("{}:{server}", crate::active_profile_name())
    }

    fn decode(bytes: &[u8]) -> Result<Self, String> {
        serde_json::from_slice(bytes).map_err(|error| error.to_string())
    }

    #[doc(hidden)]
    pub fn authorize_broker_parent() -> Result<(), String> {
        #[cfg(target_os = "macos")]
        {
            crate::vault::authorize_key_broker_parent()
        }
        #[cfg(not(target_os = "macos"))]
        {
            Err("MCP credential broker is only available on macOS".to_string())
        }
    }

    #[doc(hidden)]
    pub fn broker_load(account: &str) -> Result<Option<Vec<u8>>, String> {
        #[cfg(target_os = "macos")]
        {
            Self::load_keychain_account(account)?
                .map(|credentials| {
                    serde_json::to_vec(&credentials).map_err(|error| error.to_string())
                })
                .transpose()
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = account;
            Err("MCP credential broker is only available on macOS".to_string())
        }
    }

    #[doc(hidden)]
    pub fn broker_store(account: &str, bytes: &[u8]) -> Result<(), String> {
        #[cfg(target_os = "macos")]
        {
            Self::decode(bytes)?;
            Self::store_keychain_account(account, bytes)
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (account, bytes);
            Err("MCP credential broker is only available on macOS".to_string())
        }
    }

    #[doc(hidden)]
    pub fn broker_remove(account: &str) -> Result<(), String> {
        #[cfg(target_os = "macos")]
        {
            Self::remove_keychain_account(account)
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = account;
            Err("MCP credential broker is only available on macOS".to_string())
        }
    }
}

#[cfg(target_os = "macos")]
impl McpOauthCredentials {
    fn load_account(account: &str) -> Result<Option<Self>, String> {
        if let Some(broker) = McpCredentialBroker::current() {
            return broker
                .load(account)?
                .map(|bytes| Self::decode(&bytes))
                .transpose();
        }
        Self::load_keychain_account(account)
    }

    fn store_account(account: &str, bytes: &[u8]) -> Result<(), String> {
        if let Some(broker) = McpCredentialBroker::current() {
            return broker.store(account, bytes);
        }
        Self::store_keychain_account(account, bytes)
    }

    fn remove_account(account: &str) -> Result<(), String> {
        if let Some(broker) = McpCredentialBroker::current() {
            return broker.remove(account);
        }
        Self::remove_keychain_account(account)
    }

    fn load_keychain_account(account: &str) -> Result<Option<Self>, String> {
        use security_framework::passwords::{PasswordOptions, generic_password};
        use security_framework_sys::base::errSecItemNotFound;

        let options = PasswordOptions::new_generic_password(KEYCHAIN_SERVICE, account);
        match generic_password(options) {
            Ok(bytes) => Self::decode(&bytes).map(Some),
            Err(error) if error.code() == errSecItemNotFound => Ok(None),
            Err(error) => Err(format!("failed to load MCP credentials: {error}")),
        }
    }

    fn store_keychain_account(account: &str, bytes: &[u8]) -> Result<(), String> {
        security_framework::passwords::set_generic_password(KEYCHAIN_SERVICE, account, bytes)
            .map_err(|error| format!("failed to store MCP credentials: {error}"))
    }

    fn remove_keychain_account(account: &str) -> Result<(), String> {
        use security_framework_sys::base::errSecItemNotFound;

        match security_framework::passwords::delete_generic_password(KEYCHAIN_SERVICE, account) {
            Ok(()) => Ok(()),
            Err(error) if error.code() == errSecItemNotFound => Ok(()),
            Err(error) => Err(format!("failed to remove MCP credentials: {error}")),
        }
    }
}

#[cfg(not(target_os = "macos"))]
impl McpOauthCredentials {
    fn load_account(account: &str) -> Result<Option<Self>, String> {
        let path = Self::path(account);
        match std::fs::read(path) {
            Ok(bytes) => Self::decode(&bytes).map(Some),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }

    fn store_account(account: &str, bytes: &[u8]) -> Result<(), String> {
        use std::io::Write;

        let path = Self::path(account);
        let Some(parent) = path.parent() else {
            return Err("MCP credential path has no parent".to_string());
        };
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        let mut options = std::fs::OpenOptions::new();
        options.create(true).truncate(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(path).map_err(|error| error.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            file.set_permissions(std::fs::Permissions::from_mode(0o600))
                .map_err(|error| error.to_string())?;
        }
        file.write_all(bytes).map_err(|error| error.to_string())
    }

    fn remove_account(account: &str) -> Result<(), String> {
        match std::fs::remove_file(Self::path(account)) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.to_string()),
        }
    }

    fn path(account: &str) -> std::path::PathBuf {
        crate::profile_dir()
            .join("mcp-credentials")
            .join(format!("{}.json", encoded_account(account)))
    }
}

#[cfg(any(not(target_os = "macos"), test))]
fn encoded_account(account: &str) -> String {
    use std::fmt::Write;

    let mut encoded = String::with_capacity(account.len() * 2);
    for byte in account.as_bytes() {
        write!(&mut encoded, "{byte:02x}").unwrap();
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expiry_keeps_tokens_with_more_than_a_minute_left() {
        let credentials = McpOauthCredentials {
            expires_at: chrono::Utc::now().timestamp() + 61,
            ..McpOauthCredentials::default()
        };

        assert!(!credentials.expires_soon());
    }

    #[test]
    fn expiry_refreshes_tokens_with_a_minute_left() {
        let credentials = McpOauthCredentials {
            expires_at: chrono::Utc::now().timestamp() + 60,
            ..McpOauthCredentials::default()
        };

        assert!(credentials.expires_soon());
    }

    #[test]
    fn missing_expiry_uses_a_bounded_lifetime() {
        let before = chrono::Utc::now().timestamp() + DEFAULT_TOKEN_LIFETIME_SECS as i64;
        let expires_at = McpOauthCredentials::expires_at(None);
        let after = chrono::Utc::now().timestamp() + DEFAULT_TOKEN_LIFETIME_SECS as i64;

        assert!((before..=after).contains(&expires_at));
    }

    #[test]
    fn credential_file_names_preserve_distinct_accounts() {
        assert_ne!(encoded_account("work.dev"), encoded_account("work-dev"));
        assert_ne!(encoded_account("linear"), encoded_account("linear."));
    }

    #[test]
    fn oauth_credentials_only_authorize_the_registered_resource() {
        let credentials = McpOauthCredentials {
            resource: "https://mcp.linear.app/mcp".to_string(),
            ..McpOauthCredentials::default()
        };

        assert!(credentials.authorizes("https://mcp.linear.app/mcp"));
        assert!(credentials.authorizes("https://MCP.LINEAR.APP:443/mcp"));
        assert!(!credentials.authorizes("https://example.com/mcp"));
        assert!(!credentials.authorizes("http://mcp.linear.app/mcp"));
    }

    #[test]
    fn credential_writes_invalidate_prepared_launches() {
        let revision = McpOauthCredentials::stable_revision().unwrap();
        assert_eq!(
            McpOauthCredentials::with_revision(revision, || "current").unwrap(),
            Some("current")
        );

        McpOauthCredentials::write_transaction(|| Ok(())).unwrap();

        assert_eq!(
            McpOauthCredentials::with_revision(revision, || "stale").unwrap(),
            None
        );

        let revision = McpOauthCredentials::stable_revision().unwrap();
        let result: Result<(), String> =
            McpOauthCredentials::write_transaction(|| Err("possibly changed".to_string()));
        assert!(result.is_err());
        assert_eq!(
            McpOauthCredentials::with_revision(revision, || "current").unwrap(),
            None
        );
    }

    #[test]
    fn launch_validation_does_not_wait_for_credential_writes() {
        let revision = McpOauthCredentials::stable_revision().unwrap();
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (finish_tx, finish_rx) = std::sync::mpsc::channel();
        let writer = std::thread::spawn(move || {
            McpOauthCredentials::write_transaction(|| {
                started_tx.send(()).unwrap();
                finish_rx.recv().unwrap();
                Ok(())
            })
            .unwrap();
        });
        started_rx.recv().unwrap();

        assert_eq!(
            McpOauthCredentials::with_revision(revision, || "stale").unwrap(),
            None
        );

        finish_tx.send(()).unwrap();
        writer.join().unwrap();
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn packaged_builds_use_the_stable_mcp_credential_broker() {
        let executable =
            std::path::Path::new("/Applications/Vmux (abcdef0).app/Contents/MacOS/vmux_desktop");

        assert_eq!(
            McpCredentialBroker::candidate("local", executable),
            Some(std::path::PathBuf::from(
                "/Applications/Vmux (abcdef0).app/Contents/MacOS/vmux"
            ))
        );
        assert_eq!(
            McpCredentialBroker::candidate("release", executable),
            Some(std::path::PathBuf::from(
                "/Applications/Vmux (abcdef0).app/Contents/MacOS/vmux"
            ))
        );
        assert_eq!(McpCredentialBroker::candidate("dev", executable), None);
    }
}
