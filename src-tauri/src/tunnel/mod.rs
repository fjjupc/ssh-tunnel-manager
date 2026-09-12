mod forward;
mod handler;

use crate::config::{resolve_private_key_path, save_config};
use crate::models::{
    AppConfig, AuthMethod, Device, DeviceStatus, DeviceStatusEvent, PortMapping,
};
use anyhow::{anyhow, Context, Result};
use forward::{run_local_forward, SharedSession};
use handler::ClientHandler;
use russh::client;
use russh::keys::{load_secret_key, PrivateKeyWithHashAlg};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

const RECONNECT_BASE_MS: u64 = 1_000;
const RECONNECT_MAX_MS: u64 = 60_000;

#[derive(Clone)]
pub struct SharedState {
    pub inner: Arc<Mutex<AppInner>>,
}

pub struct AppInner {
    pub config: AppConfig,
    pub config_path: PathBuf,
    pub runtimes: HashMap<String, DeviceRuntime>,
    pub app: Option<AppHandle>,
}

pub struct DeviceRuntime {
    pub generation: u64,
    pub status: DeviceStatus,
    pub last_error: Option<String>,
    pub connect_task: Option<JoinHandle<()>>,
    pub reconnect_task: Option<JoinHandle<()>>,
    pub session: Option<SharedSession>,
    pub forwards: HashMap<String, JoinHandle<()>>,
}

impl DeviceRuntime {
    fn new() -> Self {
        Self {
            generation: 0,
            status: DeviceStatus::Disconnected,
            last_error: None,
            connect_task: None,
            reconnect_task: None,
            session: None,
            forwards: HashMap::new(),
        }
    }
}

impl SharedState {
    pub fn new(config: AppConfig, config_path: PathBuf) -> Self {
        Self {
            inner: Arc::new(Mutex::new(AppInner {
                config,
                config_path,
                runtimes: HashMap::new(),
                app: None,
            })),
        }
    }

    pub async fn set_app_handle(&self, app: AppHandle) {
        let mut guard = self.inner.lock().await;
        guard.app = Some(app);
    }

    pub async fn get_config(&self) -> AppConfig {
        self.inner.lock().await.config.clone()
    }

    async fn emit_status(&self, device_id: &str, status: DeviceStatus, message: Option<String>) {
        let app = {
            let mut guard = self.inner.lock().await;
            if let Some(rt) = guard.runtimes.get_mut(device_id) {
                rt.status = status;
                rt.last_error = message.clone();
            }
            guard.app.clone()
        };
        if let Some(app) = app {
            let _ = app.emit(
                "device-status",
                DeviceStatusEvent {
                    device_id: device_id.to_string(),
                    status,
                    message,
                },
            );
        }
    }

    pub async fn get_statuses(&self) -> HashMap<String, DeviceStatusEvent> {
        let guard = self.inner.lock().await;
        guard
            .runtimes
            .iter()
            .map(|(id, rt)| {
                (
                    id.clone(),
                    DeviceStatusEvent {
                        device_id: id.clone(),
                        status: rt.status,
                        message: rt.last_error.clone(),
                    },
                )
            })
            .collect()
    }

    pub async fn save_config_and_sync(&self, config: AppConfig) -> Result<()> {
        crate::set_close_to_tray_flag(config.close_to_tray);
        let previous = {
            let mut guard = self.inner.lock().await;
            let prev = guard.config.clone();
            guard.config = config.clone();
            save_config(&guard.config_path, &guard.config)?;
            prev
        };

        for old in &previous.devices {
            let still = config.devices.iter().find(|d| d.id == old.id);
            match still {
                None => self.stop_device(&old.id, false).await,
                Some(d) if !d.enabled && old.enabled => self.stop_device(&d.id, false).await,
                Some(d) if d.enabled => {
                    if connection_identity_changed(old, d) {
                        self.stop_device(&d.id, false).await;
                        self.start_device(&d.id).await?;
                    } else if mappings_changed(old, d) {
                        let _ = self.sync_forwards_for_device(&d.id).await;
                    }
                }
                _ => {}
            }
        }

        for d in &config.devices {
            if d.enabled {
                let was_enabled = previous
                    .devices
                    .iter()
                    .find(|o| o.id == d.id)
                    .map(|o| o.enabled)
                    .unwrap_or(false);
                if !was_enabled {
                    self.start_device(&d.id).await?;
                }
            }
        }

        Ok(())
    }

    pub async fn restore_enabled(&self) -> Result<()> {
        let ids: Vec<String> = {
            let guard = self.inner.lock().await;
            guard
                .config
                .devices
                .iter()
                .filter(|d| d.enabled)
                .map(|d| d.id.clone())
                .collect()
        };
        for id in ids {
            if let Err(e) = self.start_device(&id).await {
                log::error!("failed to restore device {}: {:#}", id, e);
            }
        }
        Ok(())
    }

    pub async fn shutdown_all(&self) {
        let ids: Vec<String> = {
            let guard = self.inner.lock().await;
            guard.config.devices.iter().map(|d| d.id.clone()).collect()
        };
        for id in ids {
            self.stop_device(&id, false).await;
        }
    }

    pub async fn start_device(&self, device_id: &str) -> Result<()> {
        let generation = {
            let mut guard = self.inner.lock().await;
            let device = guard
                .config
                .devices
                .iter()
                .find(|d| d.id == device_id)
                .cloned()
                .ok_or_else(|| anyhow!("device not found"))?;
            if !device.enabled {
                anyhow::bail!("device is not enabled");
            }
            let rt = guard
                .runtimes
                .entry(device_id.to_string())
                .or_insert_with(DeviceRuntime::new);
            rt.generation += 1;
            let generation = rt.generation;
            if let Some(h) = rt.reconnect_task.take() {
                h.abort();
            }
            if let Some(h) = rt.connect_task.take() {
                h.abort();
            }
            abort_forwards(rt);
            rt.session = None;
            generation
        };

        self.emit_status(device_id, DeviceStatus::Connecting, None)
            .await;
        self.spawn_connect(device_id.to_string(), generation, 0);
        Ok(())
    }

    pub async fn stop_device(&self, device_id: &str, update_config_enabled: bool) {
        {
            let mut guard = self.inner.lock().await;
            if update_config_enabled {
                if let Some(d) = guard.config.devices.iter_mut().find(|d| d.id == device_id) {
                    d.enabled = false;
                }
            }
            if let Some(rt) = guard.runtimes.get_mut(device_id) {
                rt.generation += 1;
                if let Some(h) = rt.reconnect_task.take() {
                    h.abort();
                }
                if let Some(h) = rt.connect_task.take() {
                    h.abort();
                }
                abort_forwards(rt);
                rt.session = None;
                rt.status = DeviceStatus::Disconnected;
                rt.last_error = None;
            }
        }
        self.emit_status(device_id, DeviceStatus::Disconnected, None)
            .await;
    }

    fn spawn_connect(&self, device_id: String, generation: u64, attempt: u32) {
        let state = self.clone();
        let id_for_store = device_id.clone();
        let handle = tokio::spawn(async move {
            let result = state.connect_once(&device_id, generation).await;
            match result {
                Ok(()) => {
                    state
                        .emit_status(&device_id, DeviceStatus::Connected, None)
                        .await;
                    if let Err(e) = state.sync_forwards_for_device(&device_id).await {
                        let msg = format!("{:#}", e);
                        log::error!("forward sync failed for {}: {}", device_id, msg);
                        state
                            .emit_status(&device_id, DeviceStatus::Connected, Some(msg))
                            .await;
                    }
                    state.watch_session(device_id, generation).await;
                }
                Err(e) => {
                    let msg = format!("{:#}", e);
                    log::warn!(
                        "connect failed for {} (attempt {}): {}",
                        device_id,
                        attempt,
                        msg
                    );
                    let should_retry = {
                        let guard = state.inner.lock().await;
                        let enabled = guard
                            .config
                            .devices
                            .iter()
                            .find(|d| d.id == device_id)
                            .map(|d| d.enabled)
                            .unwrap_or(false);
                        let current = guard
                            .runtimes
                            .get(&device_id)
                            .map(|r| r.generation)
                            .unwrap_or(0);
                        enabled && current == generation
                    };
                    if should_retry {
                        state
                            .emit_status(&device_id, DeviceStatus::Reconnecting, Some(msg))
                            .await;
                        state.schedule_reconnect(device_id, generation, attempt + 1);
                    } else {
                        state
                            .emit_status(&device_id, DeviceStatus::Disconnected, Some(msg))
                            .await;
                    }
                }
            }
        });

        let state = self.clone();
        tokio::spawn(async move {
            let mut guard = state.inner.lock().await;
            if let Some(rt) = guard.runtimes.get_mut(&id_for_store) {
                if rt.generation == generation {
                    rt.connect_task = Some(handle);
                } else {
                    handle.abort();
                }
            } else {
                handle.abort();
            }
        });
    }

    fn schedule_reconnect(&self, device_id: String, generation: u64, attempt: u32) {
        let delay_ms = RECONNECT_BASE_MS
            .saturating_mul(2u64.saturating_pow(attempt.saturating_sub(1)))
            .min(RECONNECT_MAX_MS);
        let state = self.clone();
        let id_for_store = device_id.clone();
        let handle = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            let still_valid = {
                let guard = state.inner.lock().await;
                let enabled = guard
                    .config
                    .devices
                    .iter()
                    .find(|d| d.id == device_id)
                    .map(|d| d.enabled)
                    .unwrap_or(false);
                let current = guard
                    .runtimes
                    .get(&device_id)
                    .map(|r| r.generation)
                    .unwrap_or(0);
                enabled && current == generation
            };
            if still_valid {
                state
                    .emit_status(&device_id, DeviceStatus::Connecting, None)
                    .await;
                state.spawn_connect(device_id, generation, attempt);
            }
        });

        let state = self.clone();
        tokio::spawn(async move {
            let mut guard = state.inner.lock().await;
            if let Some(rt) = guard.runtimes.get_mut(&id_for_store) {
                if rt.generation == generation {
                    if let Some(old) = rt.reconnect_task.take() {
                        old.abort();
                    }
                    rt.reconnect_task = Some(handle);
                } else {
                    handle.abort();
                }
            } else {
                handle.abort();
            }
        });
    }

    async fn connect_once(&self, device_id: &str, generation: u64) -> Result<()> {
        let device = {
            let guard = self.inner.lock().await;
            let current = guard
                .runtimes
                .get(device_id)
                .map(|r| r.generation)
                .unwrap_or(0);
            if current != generation {
                anyhow::bail!("stale connection attempt");
            }
            guard
                .config
                .devices
                .iter()
                .find(|d| d.id == device_id)
                .cloned()
                .ok_or_else(|| anyhow!("device not found"))?
        };

        let config = client::Config {
            inactivity_timeout: Some(Duration::from_secs(60)),
            keepalive_interval: Some(Duration::from_secs(15)),
            ..Default::default()
        };
        let config = Arc::new(config);
        let handler = ClientHandler {
            device_id: device_id.to_string(),
        };

        let addr = (device.host.as_str(), device.ssh_port_or_default());
        let mut handle = client::connect(config, addr, handler)
            .await
            .with_context(|| {
                format!(
                    "SSH connect to {}:{}",
                    device.host,
                    device.ssh_port_or_default()
                )
            })?;

        let auth_result = match &device.auth {
            AuthMethod::Password { password } => handle
                .authenticate_password(&device.username, password)
                .await
                .context("password authentication failed")?,
            AuthMethod::Key { private_key_path } => {
                let key_path = resolve_private_key_path(private_key_path.as_deref())?;
                let key = load_secret_key(&key_path, None)
                    .with_context(|| format!("load key {}", key_path.display()))?;
                handle
                    .authenticate_publickey(
                        &device.username,
                        PrivateKeyWithHashAlg::new(Arc::new(key), None),
                    )
                    .await
                    .context("public key authentication failed")?
            }
        };

        if !auth_result.success() {
            anyhow::bail!("authentication rejected by server");
        }

        {
            let mut guard = self.inner.lock().await;
            let rt = guard
                .runtimes
                .get_mut(device_id)
                .ok_or_else(|| anyhow!("runtime missing"))?;
            if rt.generation != generation {
                anyhow::bail!("stale after authenticate");
            }
            rt.session = Some(Arc::new(Mutex::new(handle)));
            rt.last_error = None;
        }

        Ok(())
    }

    async fn watch_session(&self, device_id: String, generation: u64) {
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            let probe_ok = {
                let guard = self.inner.lock().await;
                let Some(rt) = guard.runtimes.get(&device_id) else {
                    return;
                };
                if rt.generation != generation {
                    return;
                }
                let enabled = guard
                    .config
                    .devices
                    .iter()
                    .find(|d| d.id == device_id)
                    .map(|d| d.enabled)
                    .unwrap_or(false);
                if !enabled {
                    return;
                }
                match &rt.session {
                    Some(session) => {
                        // Try non-blocking-ish check via try_lock; if busy, assume alive.
                        match session.try_lock() {
                            Ok(s) => !s.is_closed(),
                            Err(_) => true,
                        }
                    }
                    None => false,
                }
            };

            if !probe_ok {
                break;
            }
        }

        {
            let mut guard = self.inner.lock().await;
            if let Some(rt) = guard.runtimes.get_mut(&device_id) {
                if rt.generation != generation {
                    return;
                }
                abort_forwards(rt);
                rt.session = None;
            }
        }

        let should_retry = {
            let guard = self.inner.lock().await;
            guard
                .config
                .devices
                .iter()
                .find(|d| d.id == device_id)
                .map(|d| d.enabled)
                .unwrap_or(false)
                && guard
                    .runtimes
                    .get(&device_id)
                    .map(|r| r.generation == generation)
                    .unwrap_or(false)
        };

        if should_retry {
            self.emit_status(
                &device_id,
                DeviceStatus::Reconnecting,
                Some("connection lost".into()),
            )
            .await;
            self.schedule_reconnect(device_id, generation, 1);
        }
    }

    pub async fn sync_forwards_for_device(&self, device_id: &str) -> Result<()> {
        let (session, mappings, generation) = {
            let guard = self.inner.lock().await;
            let device = guard
                .config
                .devices
                .iter()
                .find(|d| d.id == device_id)
                .cloned()
                .ok_or_else(|| anyhow!("device not found"))?;
            let rt = guard
                .runtimes
                .get(device_id)
                .ok_or_else(|| anyhow!("runtime not found"))?;
            let session = rt
                .session
                .clone()
                .ok_or_else(|| anyhow!("device not connected"))?;
            (session, device.mappings.clone(), rt.generation)
        };

        {
            let mut guard = self.inner.lock().await;
            if let Some(rt) = guard.runtimes.get_mut(device_id) {
                // Restart all forwards so local/remote port edits take effect.
                abort_forwards(rt);
            }
        }

        for mapping in mappings.into_iter().filter(|m| m.enabled) {
            self.spawn_forward(device_id, generation, session.clone(), mapping)
                .await?;
        }

        Ok(())
    }

    async fn spawn_forward(
        &self,
        device_id: &str,
        generation: u64,
        session: SharedSession,
        mapping: PortMapping,
    ) -> Result<()> {
        let mapping_id = mapping.id.clone();
        let mapping_id_for_map = mapping_id.clone();
        let state = self.clone();
        let device_id_owned = device_id.to_string();

        let handle = tokio::spawn(async move {
            let result = run_local_forward(session, mapping).await;
            if let Err(e) = result {
                log::warn!(
                    "forward {} on device {} ended: {:#}",
                    mapping_id,
                    device_id_owned,
                    e
                );
            }

            let mut guard = state.inner.lock().await;
            if let Some(rt) = guard.runtimes.get_mut(&device_id_owned) {
                if rt.generation == generation {
                    rt.forwards.remove(&mapping_id);
                    let closed = match &rt.session {
                        Some(s) => s.try_lock().map(|g| g.is_closed()).unwrap_or(false),
                        None => true,
                    };
                    if closed {
                        rt.session = None;
                    }
                }
            }
        });

        let mut guard = self.inner.lock().await;
        if let Some(rt) = guard.runtimes.get_mut(device_id) {
            if rt.generation == generation {
                rt.forwards.insert(mapping_id_for_map, handle);
            } else {
                handle.abort();
            }
        }
        Ok(())
    }
}

fn abort_forwards(rt: &mut DeviceRuntime) {
    for (_, h) in rt.forwards.drain() {
        h.abort();
    }
}

fn connection_identity_changed(old: &Device, new: &Device) -> bool {
    old.host != new.host
        || old.ssh_port != new.ssh_port
        || old.username != new.username
        || auth_changed(&old.auth, &new.auth)
}

fn mappings_changed(old: &Device, new: &Device) -> bool {
    old.mappings != new.mappings
}

fn auth_changed(a: &AuthMethod, b: &AuthMethod) -> bool {
    match (a, b) {
        (AuthMethod::Password { password: p1 }, AuthMethod::Password { password: p2 }) => p1 != p2,
        (
            AuthMethod::Key {
                private_key_path: p1,
            },
            AuthMethod::Key {
                private_key_path: p2,
            },
        ) => p1 != p2,
        _ => true,
    }
}
