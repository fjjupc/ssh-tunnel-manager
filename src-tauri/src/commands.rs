use crate::config::default_key_hint;
use crate::models::{AppConfig, Device, DeviceStatusEvent, PortMapping};
use crate::tunnel::SharedState;
use std::collections::HashMap;
use tauri::State;

#[tauri::command]
pub async fn get_config(state: State<'_, SharedState>) -> Result<AppConfig, String> {
    Ok(state.get_config().await)
}

#[tauri::command]
pub async fn save_config(state: State<'_, SharedState>, config: AppConfig) -> Result<(), String> {
    state
        .save_config_and_sync(config)
        .await
        .map_err(|e| format!("{:#}", e))
}

#[tauri::command]
pub async fn get_device_statuses(
    state: State<'_, SharedState>,
) -> Result<HashMap<String, DeviceStatusEvent>, String> {
    Ok(state.get_statuses().await)
}

#[tauri::command]
pub async fn add_device(
    state: State<'_, SharedState>,
    name: String,
    host: String,
) -> Result<Device, String> {
    let mut config = state.get_config().await;
    let device = Device::new(name, host);
    let cloned = device.clone();
    config.devices.push(device);
    state
        .save_config_and_sync(config)
        .await
        .map_err(|e| format!("{:#}", e))?;
    Ok(cloned)
}

#[tauri::command]
pub async fn remove_device(state: State<'_, SharedState>, device_id: String) -> Result<(), String> {
    state.stop_device(&device_id, false).await;
    let mut config = state.get_config().await;
    config.devices.retain(|d| d.id != device_id);
    state
        .save_config_and_sync(config)
        .await
        .map_err(|e| format!("{:#}", e))
}

#[tauri::command]
pub async fn update_device(state: State<'_, SharedState>, device: Device) -> Result<(), String> {
    let mut config = state.get_config().await;
    if let Some(slot) = config.devices.iter_mut().find(|d| d.id == device.id) {
        *slot = device;
    } else {
        return Err("device not found".into());
    }
    state
        .save_config_and_sync(config)
        .await
        .map_err(|e| format!("{:#}", e))
}

#[tauri::command]
pub async fn set_device_enabled(
    state: State<'_, SharedState>,
    device_id: String,
    enabled: bool,
) -> Result<(), String> {
    let mut config = state.get_config().await;
    let device = config
        .devices
        .iter_mut()
        .find(|d| d.id == device_id)
        .ok_or_else(|| "device not found".to_string())?;
    device.enabled = enabled;
    if !enabled {
        for m in &mut device.mappings {
            m.enabled = false;
        }
    }
    state
        .save_config_and_sync(config)
        .await
        .map_err(|e| format!("{:#}", e))
}

#[tauri::command]
pub async fn add_mapping(
    state: State<'_, SharedState>,
    device_id: String,
    local_port: u16,
    remote_port: u16,
    remote_host: Option<String>,
) -> Result<PortMapping, String> {
    let mut config = state.get_config().await;
    let device = config
        .devices
        .iter_mut()
        .find(|d| d.id == device_id)
        .ok_or_else(|| "device not found".to_string())?;
    let mut mapping = PortMapping::new(local_port, remote_port);
    if let Some(host) = remote_host {
        if !host.trim().is_empty() {
            mapping.remote_host = host;
        }
    }
    let cloned = mapping.clone();
    device.mappings.push(mapping);
    state
        .save_config_and_sync(config)
        .await
        .map_err(|e| format!("{:#}", e))?;
    Ok(cloned)
}

#[tauri::command]
pub async fn remove_mapping(
    state: State<'_, SharedState>,
    device_id: String,
    mapping_id: String,
) -> Result<(), String> {
    let mut config = state.get_config().await;
    let device = config
        .devices
        .iter_mut()
        .find(|d| d.id == device_id)
        .ok_or_else(|| "device not found".to_string())?;
    device.mappings.retain(|m| m.id != mapping_id);
    state
        .save_config_and_sync(config)
        .await
        .map_err(|e| format!("{:#}", e))
}

#[tauri::command]
pub async fn set_mapping_enabled(
    state: State<'_, SharedState>,
    device_id: String,
    mapping_id: String,
    enabled: bool,
) -> Result<(), String> {
    let mut config = state.get_config().await;
    let device = config
        .devices
        .iter_mut()
        .find(|d| d.id == device_id)
        .ok_or_else(|| "device not found".to_string())?;
    if enabled && !device.enabled {
        return Err("enable the device before enabling port mappings".into());
    }
    let mapping = device
        .mappings
        .iter_mut()
        .find(|m| m.id == mapping_id)
        .ok_or_else(|| "mapping not found".to_string())?;
    mapping.enabled = enabled;
    state
        .save_config_and_sync(config)
        .await
        .map_err(|e| format!("{:#}", e))
}

#[tauri::command]
pub async fn update_mapping(
    state: State<'_, SharedState>,
    device_id: String,
    mapping: PortMapping,
) -> Result<(), String> {
    let mut config = state.get_config().await;
    let device = config
        .devices
        .iter_mut()
        .find(|d| d.id == device_id)
        .ok_or_else(|| "device not found".to_string())?;
    if let Some(slot) = device.mappings.iter_mut().find(|m| m.id == mapping.id) {
        *slot = mapping;
        if !device.enabled {
            slot.enabled = false;
        }
    } else {
        return Err("mapping not found".into());
    }
    state
        .save_config_and_sync(config)
        .await
        .map_err(|e| format!("{:#}", e))
}

#[tauri::command]
pub fn get_default_key_path() -> String {
    default_key_hint()
}

#[tauri::command]
pub async fn quit_app(app: tauri::AppHandle, state: State<'_, SharedState>) -> Result<(), String> {
    crate::allow_exit();
    state.shutdown_all().await;
    app.exit(0);
    Ok(())
}
