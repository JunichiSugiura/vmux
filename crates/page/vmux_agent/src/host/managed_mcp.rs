use std::collections::BTreeMap;

use serde_json::{Map, Value};
use vmux_core::profile::tools::{McpServerManifest, McpTransport};
use vmux_service::protocol::{ManagedMcpServer, ManagedMcpTransport};

#[cfg(not(test))]
pub fn load() -> BTreeMap<String, McpServerManifest> {
    match vmux_core::profile::tools::load_manifest() {
        Ok(manifest) => {
            let mut servers = manifest.mcp.servers;
            for (name, server) in &mut servers {
                if let Err(error) = McpAuthorization::apply(name, server) {
                    bevy::log::warn!("managed MCP authorization unavailable for {name}: {error}");
                }
            }
            servers
        }
        Err(error) => {
            bevy::log::warn!("managed MCP servers unavailable: {error}");
            BTreeMap::new()
        }
    }
}

#[cfg(test)]
pub fn load() -> BTreeMap<String, McpServerManifest> {
    BTreeMap::new()
}

pub fn acp_servers() -> Vec<ManagedMcpServer> {
    load()
        .into_iter()
        .map(|(name, server)| acp_server(name, server))
        .collect()
}

fn acp_server(name: String, server: McpServerManifest) -> ManagedMcpServer {
    let headers = server.resolved_headers().into_iter().collect();
    ManagedMcpServer {
        name,
        transport: match server.transport {
            McpTransport::Stdio => ManagedMcpTransport::Stdio,
            McpTransport::Http => ManagedMcpTransport::Http,
            McpTransport::Sse => ManagedMcpTransport::Sse,
        },
        command: server.command,
        args: server.args,
        env: server.env.into_iter().collect(),
        cwd: server.cwd,
        url: server.url,
        headers,
    }
}

pub fn claude_value(server: &McpServerManifest) -> Value {
    let mut value = Map::new();
    match server.transport {
        McpTransport::Stdio => {
            if let Some(command) = &server.command {
                value.insert("command".to_string(), Value::String(command.clone()));
            }
            insert_array(&mut value, "args", &server.args);
            insert_object(&mut value, "env", &server.env);
            if let Some(cwd) = &server.cwd {
                value.insert("cwd".to_string(), Value::String(cwd.clone()));
            }
        }
        McpTransport::Http | McpTransport::Sse => {
            value.insert(
                "type".to_string(),
                Value::String(
                    match server.transport {
                        McpTransport::Sse => "sse",
                        _ => "http",
                    }
                    .to_string(),
                ),
            );
            if let Some(url) = &server.url {
                value.insert("url".to_string(), Value::String(url.clone()));
            }
            insert_object(&mut value, "headers", &server.resolved_headers());
        }
    }
    Value::Object(value)
}

pub fn vibe_value(name: &str, server: &McpServerManifest) -> Value {
    let mut value = match claude_value(server) {
        Value::Object(value) => value,
        _ => Map::new(),
    };
    value.insert("name".to_string(), Value::String(name.to_string()));
    value.insert(
        "transport".to_string(),
        Value::String(
            match server.transport {
                McpTransport::Stdio => "stdio",
                McpTransport::Http => "http",
                McpTransport::Sse => "sse",
            }
            .to_string(),
        ),
    );
    value.remove("type");
    Value::Object(value)
}

#[cfg(not(test))]
struct McpAuthorization;

#[cfg(not(test))]
impl McpAuthorization {
    fn apply(name: &str, server: &mut McpServerManifest) -> Result<(), String> {
        let Some(mut credentials) =
            vmux_core::profile::mcp_credentials::McpOauthCredentials::load(name)?
        else {
            return Ok(());
        };
        if credentials.expires_soon() {
            Self::refresh(&mut credentials)?;
            credentials.store(name)?;
        }
        if credentials.access_token.is_empty() {
            return Err("stored access token is empty".to_string());
        }
        server.headers.insert(
            "Authorization".to_string(),
            format!("Bearer {}", credentials.access_token),
        );
        Ok(())
    }

    fn refresh(
        credentials: &mut vmux_core::profile::mcp_credentials::McpOauthCredentials,
    ) -> Result<(), String> {
        let refresh_token = credentials
            .refresh_token
            .as_ref()
            .ok_or_else(|| "OAuth session expired and has no refresh token".to_string())?;
        let mut form = vec![
            ("grant_type", "refresh_token".to_string()),
            ("refresh_token", refresh_token.clone()),
            ("client_id", credentials.client_id.clone()),
        ];
        if let Some(secret) = &credentials.client_secret {
            form.push(("client_secret", secret.clone()));
        }
        if !credentials.resource.is_empty() {
            form.push(("resource", credentials.resource.clone()));
        }
        if !credentials.scope.is_empty() {
            form.push(("scope", credentials.scope.clone()));
        }
        let response = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|error| error.to_string())?
            .post(&credentials.token_endpoint)
            .form(&form)
            .send()
            .map_err(|error| error.to_string())?;
        let status = response.status();
        let body = response.text().map_err(|error| error.to_string())?;
        if !status.is_success() {
            return Err(format!("token refresh failed ({status}): {body}"));
        }
        let token: RefreshTokenResponse = serde_json::from_str(&body)
            .map_err(|error| format!("invalid token refresh response: {error}"))?;
        credentials.access_token = token.access_token;
        if token.refresh_token.is_some() {
            credentials.refresh_token = token.refresh_token;
        }
        credentials.expires_at = token
            .expires_in
            .map(|seconds| chrono::Utc::now().timestamp() + seconds as i64)
            .unwrap_or_default();
        if let Some(scope) = token.scope {
            credentials.scope = scope;
        }
        Ok(())
    }
}

#[cfg(not(test))]
#[derive(serde::Deserialize)]
struct RefreshTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
    scope: Option<String>,
}

fn insert_array(value: &mut Map<String, Value>, name: &str, entries: &[String]) {
    if !entries.is_empty() {
        value.insert(
            name.to_string(),
            Value::Array(entries.iter().cloned().map(Value::String).collect()),
        );
    }
}

fn insert_object(value: &mut Map<String, Value>, name: &str, entries: &BTreeMap<String, String>) {
    if !entries.is_empty() {
        value.insert(
            name.to_string(),
            Value::Object(
                entries
                    .iter()
                    .map(|(key, value)| (key.clone(), Value::String(value.clone())))
                    .collect(),
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_projection_resolves_remote_headers() {
        let server = McpServerManifest {
            transport: McpTransport::Http,
            command: None,
            args: Vec::new(),
            env: BTreeMap::new(),
            cwd: None,
            url: Some("https://example.com/mcp".to_string()),
            headers: BTreeMap::from([("X-Key".to_string(), "value".to_string())]),
            header_env: BTreeMap::new(),
            bearer_token_env_var: None,
        };

        assert_eq!(
            claude_value(&server),
            serde_json::json!({
                "type": "http",
                "url": "https://example.com/mcp",
                "headers": {"X-Key": "value"}
            })
        );
    }

    #[test]
    fn acp_projection_preserves_stdio_launch_configuration() {
        let server = McpServerManifest {
            transport: McpTransport::Stdio,
            command: Some("npx".to_string()),
            args: vec!["-y".to_string(), "server".to_string()],
            env: BTreeMap::from([("MODE".to_string(), "local".to_string())]),
            cwd: Some("/tmp/project".to_string()),
            url: None,
            headers: BTreeMap::new(),
            header_env: BTreeMap::new(),
            bearer_token_env_var: None,
        };

        assert_eq!(
            acp_server("local".to_string(), server),
            ManagedMcpServer {
                name: "local".to_string(),
                transport: ManagedMcpTransport::Stdio,
                command: Some("npx".to_string()),
                args: vec!["-y".to_string(), "server".to_string()],
                env: vec![("MODE".to_string(), "local".to_string())],
                cwd: Some("/tmp/project".to_string()),
                url: None,
                headers: Vec::new(),
            }
        );
    }
}
