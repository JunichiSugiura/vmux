use serde::{Deserialize, Serialize};

#[cfg(target_os = "macos")]
const KEYCHAIN_SERVICE: &str = "ai.vmux.mcp";

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

    fn account(server: &str) -> String {
        format!("{}:{server}", crate::active_profile_name())
    }

    fn decode(bytes: &[u8]) -> Result<Self, String> {
        serde_json::from_slice(bytes).map_err(|error| error.to_string())
    }
}

#[cfg(target_os = "macos")]
impl McpOauthCredentials {
    fn load_account(account: &str) -> Result<Option<Self>, String> {
        use security_framework::passwords::{PasswordOptions, generic_password};
        use security_framework_sys::base::errSecItemNotFound;

        let options = PasswordOptions::new_generic_password(KEYCHAIN_SERVICE, account);
        match generic_password(options) {
            Ok(bytes) => Self::decode(&bytes).map(Some),
            Err(error) if error.code() == errSecItemNotFound => Ok(None),
            Err(error) => Err(format!("failed to load MCP credentials: {error}")),
        }
    }

    fn store_account(account: &str, bytes: &[u8]) -> Result<(), String> {
        security_framework::passwords::set_generic_password(KEYCHAIN_SERVICE, account, bytes)
            .map_err(|error| format!("failed to store MCP credentials: {error}"))
    }

    fn remove_account(account: &str) -> Result<(), String> {
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
        use std::os::unix::fs::OpenOptionsExt;

        let path = Self::path(account);
        let Some(parent) = path.parent() else {
            return Err("MCP credential path has no parent".to_string());
        };
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .mode(0o600)
            .open(path)
            .map_err(|error| error.to_string())?;
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
        let name = account
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                    character
                } else {
                    '-'
                }
            })
            .collect::<String>();
        crate::profile_dir()
            .join("mcp-credentials")
            .join(format!("{name}.json"))
    }
}

#[cfg(test)]
mod tests {
    use super::McpOauthCredentials;

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
}
