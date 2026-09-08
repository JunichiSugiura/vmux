use std::collections::{BTreeSet, VecDeque};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use bevy::prelude::*;
use bevy::tasks::{IoTaskPool, Task, futures_lite::future};
use bevy_cef::prelude::{BinEventEmitterPlugin, BinHostEmitEvent, BinReceive, Browsers};
use parking_lot::Mutex;
use reqwest::blocking::{Client, Response};
use ring::digest::{SHA256, digest};
use serde::{Deserialize, Serialize};
use url::Url;
use vmux_command::{AppCommand, BrowserCommand, open::OpenCommand};
use vmux_core::profile::mcp_credentials::McpOauthCredentials;
use vmux_core::profile::tools::{McpServerManifest, McpTransport, load_manifest, write_manifest};
use vmux_wire::mcp::{
    MCP_SERVER_ACTION_RESULT_EVENT, MCP_SERVERS_EVENT, McpServerAction, McpServerActionRequest,
    McpServerActionResult, McpServerEntry, McpServerStatus, McpServers, McpServersRequest,
};

pub struct McpConnectionPlugin;

impl Plugin for McpConnectionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<McpActionQueue>()
            .add_plugins(BinEventEmitterPlugin::<(
                McpServersRequest,
                McpServerActionRequest,
            )>::default())
            .add_observer(McpConnections::request)
            .add_observer(McpConnections::act)
            .add_systems(
                Update,
                (McpConnections::start, McpConnections::drain).chain(),
            );
    }
}

struct McpConnections;

impl McpConnections {
    fn request(
        trigger: On<BinReceive<McpServersRequest>>,
        browsers: NonSend<Browsers>,
        mut commands: Commands,
    ) {
        let target = trigger.event().webview;
        if !browsers.can_emit_to(&target) {
            return;
        }
        commands.trigger(BinHostEmitEvent::from_rkyv(
            target,
            MCP_SERVERS_EVENT,
            &McpCatalog::snapshot(),
        ));
    }

    fn act(trigger: On<BinReceive<McpServerActionRequest>>, mut queue: ResMut<McpActionQueue>) {
        queue
            .0
            .push_back((trigger.event().webview, trigger.event().payload.clone()));
    }

    fn start(
        mut queue: ResMut<McpActionQueue>,
        tasks: Query<(), With<McpActionTask>>,
        proxy: Option<Res<bevy::winit::EventLoopProxyWrapper>>,
        mut commands: Commands,
    ) {
        if !tasks.is_empty() {
            return;
        }
        let Some((target, request)) = queue.0.pop_front() else {
            return;
        };
        let task_request = request.clone();
        let completion_wake = proxy.as_deref().map(|proxy| (**proxy).clone());
        let progress_wake = completion_wake.clone();
        let (progress_sender, progress_receiver) = mpsc::channel();
        let task = IoTaskPool::get().spawn(async move {
            let result = match task_request.action {
                McpServerAction::Connect => McpConnection::connect(&task_request.id, |url| {
                    if progress_sender.send(url).is_ok()
                        && let Some(wake) = &progress_wake
                    {
                        let _ = wake.send_event(bevy::winit::WinitUserEvent::WakeUp);
                    }
                }),
                McpServerAction::Disconnect => McpConnection::disconnect(&task_request.id),
            };
            if let Some(wake) = completion_wake {
                let _ = wake.send_event(bevy::winit::WinitUserEvent::WakeUp);
            }
            result
        });
        commands.spawn(McpActionTask {
            target,
            request,
            task,
            progress: Mutex::new(progress_receiver),
        });
    }

    fn drain(
        mut tasks: Query<(Entity, &mut McpActionTask)>,
        browsers: NonSend<Browsers>,
        mut app_commands: MessageWriter<AppCommand>,
        mut commands: Commands,
    ) {
        for (entity, mut task) in &mut tasks {
            while let Ok(url) = task.progress.get_mut().try_recv() {
                if browsers.can_emit_to(&task.target) {
                    app_commands.write(AppCommand::Browser(BrowserCommand::Open(
                        OpenCommand::InNewStack { url: Some(url) },
                    )));
                }
            }
            let Some(result) = future::block_on(future::poll_once(&mut task.task)) else {
                continue;
            };
            commands.entity(entity).despawn();
            let (success, message) = match result {
                Ok(()) => (true, String::new()),
                Err(message) => (false, message),
            };
            if !browsers.can_emit_to(&task.target) {
                continue;
            }
            commands.trigger(BinHostEmitEvent::from_rkyv(
                task.target,
                MCP_SERVER_ACTION_RESULT_EVENT,
                &McpServerActionResult {
                    id: task.request.id.clone(),
                    action: task.request.action,
                    success,
                    message,
                },
            ));
            commands.trigger(BinHostEmitEvent::from_rkyv(
                task.target,
                MCP_SERVERS_EVENT,
                &McpCatalog::snapshot(),
            ));
        }
    }
}

#[derive(Resource, Default)]
struct McpActionQueue(VecDeque<(Entity, McpServerActionRequest)>);

#[derive(Component)]
struct McpActionTask {
    target: Entity,
    request: McpServerActionRequest,
    task: Task<Result<(), String>>,
    progress: Mutex<mpsc::Receiver<String>>,
}

#[derive(Clone, Copy)]
struct McpCatalogEntry {
    id: &'static str,
    name: &'static str,
    url: &'static str,
    scopes: &'static [&'static str],
}

struct McpCatalog;

impl McpCatalog {
    const ENTRIES: [McpCatalogEntry; 1] = [McpCatalogEntry {
        id: "linear",
        name: "Linear",
        url: "https://mcp.linear.app/mcp",
        scopes: &["read", "write"],
    }];

    fn get(id: &str) -> Option<McpCatalogEntry> {
        Self::ENTRIES.iter().copied().find(|entry| entry.id == id)
    }

    fn snapshot() -> McpServers {
        let manifest = load_manifest().unwrap_or_default();
        let mut servers = Vec::new();
        let mut catalog_ids = BTreeSet::new();
        for entry in Self::ENTRIES {
            catalog_ids.insert(entry.id);
            let configured = manifest.mcp.servers.contains_key(entry.id);
            let authenticated = McpOauthCredentials::load(entry.id).ok().flatten().is_some();
            let status = match (configured, authenticated) {
                (true, true) => McpServerStatus::Connected,
                (true, false) => McpServerStatus::AuthenticationRequired,
                (false, _) => McpServerStatus::Available,
            };
            servers.push(McpServerEntry {
                id: entry.id.to_string(),
                name: entry.name.to_string(),
                description: String::new(),
                status,
            });
        }
        for (id, server) in manifest.mcp.servers {
            if catalog_ids.contains(id.as_str()) || id == "vmux" {
                continue;
            }
            let description = server
                .url
                .or(server.command)
                .unwrap_or_else(|| format!("{:?}", server.transport).to_ascii_lowercase());
            servers.push(McpServerEntry {
                name: id.clone(),
                id,
                description,
                status: McpServerStatus::Configured,
            });
        }
        McpServers { servers }
    }
}

struct McpConnection;

impl McpConnection {
    fn connect(id: &str, progress: impl FnOnce(String)) -> Result<(), String> {
        let entry = McpCatalog::get(id).ok_or_else(|| format!("Unknown MCP server: {id}"))?;
        let listener = TcpListener::bind("127.0.0.1:0")
            .map_err(|error| format!("failed to open OAuth callback: {error}"))?;
        let address = listener
            .local_addr()
            .map_err(|error| format!("failed to read OAuth callback address: {error}"))?;
        let redirect_uri = format!("http://127.0.0.1:{}/callback", address.port());
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|error| error.to_string())?;
        let protected = Self::protected_metadata(&client, entry.url)?;
        let issuer = protected
            .authorization_servers
            .first()
            .ok_or_else(|| "MCP server did not advertise an authorization server".to_string())?;
        let authorization = Self::authorization_metadata(&client, issuer)?;
        let registration = Self::register(&client, &authorization, &redirect_uri)?;
        let verifier = format!(
            "{}{}",
            uuid::Uuid::new_v4().simple(),
            uuid::Uuid::new_v4().simple()
        );
        let challenge = base64_url(digest(&SHA256, verifier.as_bytes()).as_ref());
        let state = uuid::Uuid::new_v4().simple().to_string();
        let mut url = Url::parse(&authorization.authorization_endpoint)
            .map_err(|error| format!("invalid OAuth authorization URL: {error}"))?;
        url.query_pairs_mut()
            .append_pair("response_type", "code")
            .append_pair("client_id", &registration.client_id)
            .append_pair("redirect_uri", &redirect_uri)
            .append_pair("code_challenge", &challenge)
            .append_pair("code_challenge_method", "S256")
            .append_pair("state", &state)
            .append_pair("resource", entry.url)
            .append_pair("scope", &entry.scopes.join(" "));
        progress(url.to_string());
        let code = McpCallback::receive(listener, &state)?;
        let token = Self::exchange(
            &client,
            &authorization.token_endpoint,
            &registration,
            &code,
            &verifier,
            &redirect_uri,
            entry.url,
        )?;
        let credentials = McpOauthCredentials {
            token_endpoint: authorization.token_endpoint,
            client_id: registration.client_id,
            client_secret: registration.client_secret,
            access_token: token.access_token,
            refresh_token: token.refresh_token,
            expires_at: token
                .expires_in
                .map(|seconds| chrono::Utc::now().timestamp() + seconds as i64)
                .unwrap_or_default(),
            scope: token.scope.unwrap_or_else(|| entry.scopes.join(" ")),
            resource: entry.url.to_string(),
        };
        credentials.store(id)?;
        if let Err(error) = Self::write_manifest(entry) {
            let _ = McpOauthCredentials::remove(id);
            return Err(error);
        }
        Ok(())
    }

    fn disconnect(id: &str) -> Result<(), String> {
        McpOauthCredentials::remove(id)?;
        let mut manifest = load_manifest()?;
        manifest.mcp.servers.remove(id);
        write_manifest(&manifest)
    }

    fn protected_metadata(client: &Client, resource: &str) -> Result<ProtectedResource, String> {
        let resource = Url::parse(resource).map_err(|error| error.to_string())?;
        let mut metadata = resource.clone();
        metadata.set_path(&format!(
            "/.well-known/oauth-protected-resource{}",
            resource.path()
        ));
        metadata.set_query(None);
        Self::json(client.get(metadata).send())
    }

    fn authorization_metadata(
        client: &Client,
        issuer: &str,
    ) -> Result<AuthorizationServer, String> {
        let mut metadata = Url::parse(issuer).map_err(|error| error.to_string())?;
        metadata.set_path("/.well-known/oauth-authorization-server");
        metadata.set_query(None);
        Self::json(client.get(metadata).send())
    }

    fn register(
        client: &Client,
        authorization: &AuthorizationServer,
        redirect_uri: &str,
    ) -> Result<ClientRegistration, String> {
        let endpoint = authorization
            .registration_endpoint
            .as_ref()
            .ok_or_else(|| {
                "OAuth server does not support dynamic client registration".to_string()
            })?;
        Self::json(
            client
                .post(endpoint)
                .json(&RegistrationRequest {
                    client_name: "vmux",
                    redirect_uris: [redirect_uri],
                    grant_types: ["authorization_code", "refresh_token"],
                    response_types: ["code"],
                    token_endpoint_auth_method: "none",
                })
                .send(),
        )
    }

    fn exchange(
        client: &Client,
        endpoint: &str,
        registration: &ClientRegistration,
        code: &str,
        verifier: &str,
        redirect_uri: &str,
        resource: &str,
    ) -> Result<TokenResponse, String> {
        let mut form = vec![
            ("grant_type", "authorization_code".to_string()),
            ("code", code.to_string()),
            ("client_id", registration.client_id.clone()),
            ("redirect_uri", redirect_uri.to_string()),
            ("code_verifier", verifier.to_string()),
            ("resource", resource.to_string()),
        ];
        if let Some(secret) = &registration.client_secret {
            form.push(("client_secret", secret.clone()));
        }
        Self::json(client.post(endpoint).form(&form).send())
    }

    fn json<T: for<'de> Deserialize<'de>>(
        response: Result<Response, reqwest::Error>,
    ) -> Result<T, String> {
        let response = response.map_err(|error| error.to_string())?;
        let status = response.status();
        let body = response.text().map_err(|error| error.to_string())?;
        if !status.is_success() {
            return Err(format!("OAuth request failed ({status}): {body}"));
        }
        serde_json::from_str(&body).map_err(|error| format!("invalid OAuth response: {error}"))
    }

    fn write_manifest(entry: McpCatalogEntry) -> Result<(), String> {
        let mut manifest = load_manifest()?;
        manifest.mcp.servers.insert(
            entry.id.to_string(),
            McpServerManifest {
                transport: McpTransport::Http,
                command: None,
                args: Vec::new(),
                env: Default::default(),
                cwd: None,
                url: Some(entry.url.to_string()),
                headers: Default::default(),
                header_env: Default::default(),
                bearer_token_env_var: None,
            },
        );
        write_manifest(&manifest)
    }
}

struct McpCallback;

impl McpCallback {
    fn receive(listener: TcpListener, expected_state: &str) -> Result<String, String> {
        listener
            .set_nonblocking(true)
            .map_err(|error| error.to_string())?;
        let deadline = Instant::now() + Duration::from_secs(300);
        loop {
            match listener.accept() {
                Ok((mut stream, _)) => return Self::read(&mut stream, expected_state),
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if Instant::now() >= deadline {
                        return Err("OAuth authorization timed out".to_string());
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
                Err(error) => return Err(format!("OAuth callback failed: {error}")),
            }
        }
    }

    fn read(stream: &mut TcpStream, expected_state: &str) -> Result<String, String> {
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .map_err(|error| error.to_string())?;
        let mut bytes = [0_u8; 16 * 1024];
        let length = stream
            .read(&mut bytes)
            .map_err(|error| format!("failed to read OAuth callback: {error}"))?;
        let request = String::from_utf8_lossy(&bytes[..length]);
        let target = request
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .ok_or_else(|| "invalid OAuth callback".to_string())?;
        let result = Self::code(target, expected_state);
        let response = match result {
            Ok(_) => b"HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n".as_slice(),
            Err(_) => b"HTTP/1.1 400 Bad Request\r\nConnection: close\r\n\r\n".as_slice(),
        };
        stream
            .write_all(response)
            .map_err(|error| format!("failed to finish OAuth callback: {error}"))?;
        result
    }

    fn code(target: &str, expected_state: &str) -> Result<String, String> {
        let url = Url::parse(&format!("http://127.0.0.1{target}"))
            .map_err(|error| format!("invalid OAuth callback: {error}"))?;
        let values = url
            .query_pairs()
            .collect::<std::collections::BTreeMap<_, _>>();
        let state = values
            .get("state")
            .ok_or_else(|| "OAuth callback omitted state".to_string())?;
        if state.as_ref() != expected_state {
            return Err("OAuth callback state did not match".to_string());
        }
        if let Some(error) = values.get("error") {
            return Err(values
                .get("error_description")
                .map(|description| description.to_string())
                .unwrap_or_else(|| error.to_string()));
        }
        let code = values
            .get("code")
            .ok_or_else(|| "OAuth callback omitted authorization code".to_string())?
            .to_string();
        Ok(code)
    }
}

#[derive(Deserialize)]
struct ProtectedResource {
    authorization_servers: Vec<String>,
}

#[derive(Deserialize)]
struct AuthorizationServer {
    authorization_endpoint: String,
    token_endpoint: String,
    registration_endpoint: Option<String>,
}

#[derive(Serialize)]
struct RegistrationRequest<'a> {
    client_name: &'static str,
    redirect_uris: [&'a str; 1],
    grant_types: [&'static str; 2],
    response_types: [&'static str; 1],
    token_endpoint_auth_method: &'static str,
}

#[derive(Deserialize)]
struct ClientRegistration {
    client_id: String,
    client_secret: Option<String>,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
    scope: Option<String>,
}

fn base64_url(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut encoded = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let first = chunk[0];
        let second = chunk.get(1).copied().unwrap_or_default();
        let third = chunk.get(2).copied().unwrap_or_default();
        encoded.push(TABLE[(first >> 2) as usize] as char);
        encoded.push(TABLE[(((first & 0b11) << 4) | (second >> 4)) as usize] as char);
        if chunk.len() > 1 {
            encoded.push(TABLE[(((second & 0b1111) << 2) | (third >> 6)) as usize] as char);
        }
        if chunk.len() > 2 {
            encoded.push(TABLE[(third & 0b111111) as usize] as char);
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::{McpCallback, base64_url};

    #[test]
    fn pkce_uses_unpadded_url_safe_base64() {
        assert_eq!(base64_url(&[0xfb, 0xff]), "-_8");
    }

    #[test]
    fn oauth_callback_requires_the_matching_state() {
        assert_eq!(
            McpCallback::code("/callback?code=abc&state=expected", "expected").unwrap(),
            "abc"
        );
        assert!(McpCallback::code("/callback?code=abc&state=wrong", "expected").is_err());
    }
}
