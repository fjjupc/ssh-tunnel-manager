use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AuthMethod {
    #[serde(rename = "password")]
    Password { password: String },
    #[serde(rename = "key")]
    Key {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        private_key_path: Option<String>,
    },
}

impl Default for AuthMethod {
    fn default() -> Self {
        AuthMethod::Key {
            private_key_path: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PortMapping {
    pub id: String,
    pub local_port: u16,
    #[serde(default = "default_remote_host")]
    pub remote_host: String,
    pub remote_port: u16,
    #[serde(default)]
    pub enabled: bool,
}

fn default_remote_host() -> String {
    "127.0.0.1".into()
}

impl PortMapping {
    pub fn new(local_port: u16, remote_port: u16) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            local_port,
            remote_host: default_remote_host(),
            remote_port,
            enabled: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub id: String,
    pub name: String,
    pub host: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ssh_port: Option<u16>,
    pub username: String,
    #[serde(default)]
    pub auth: AuthMethod,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub mappings: Vec<PortMapping>,
}

impl Device {
    pub fn new(name: String, host: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            host,
            ssh_port: None,
            username: "root".into(),
            auth: AuthMethod::default(),
            enabled: false,
            mappings: vec![],
        }
    }

    pub fn ssh_port_or_default(&self) -> u16 {
        self.ssh_port.unwrap_or(22)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    #[serde(default)]
    pub devices: Vec<Device>,
    #[serde(default)]
    pub auto_start: bool,
    #[serde(default = "default_true")]
    pub close_to_tray: bool,
}

fn default_true() -> bool {
    true
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            devices: vec![],
            auto_start: false,
            close_to_tray: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DeviceStatus {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceStatusEvent {
    pub device_id: String,
    pub status: DeviceStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}
