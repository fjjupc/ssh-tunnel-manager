import { invoke } from "@tauri-apps/api/core";
import type {
  AppConfig,
  Device,
  DeviceStatusEvent,
  PortMapping,
} from "./types";

export async function getConfig(): Promise<AppConfig> {
  return invoke("get_config");
}

export async function saveConfig(config: AppConfig): Promise<void> {
  return invoke("save_config", { config });
}

export async function getDeviceStatuses(): Promise<
  Record<string, DeviceStatusEvent>
> {
  return invoke("get_device_statuses");
}

export async function addDevice(name: string, host: string): Promise<Device> {
  return invoke("add_device", { name, host });
}

export async function removeDevice(deviceId: string): Promise<void> {
  return invoke("remove_device", { deviceId });
}

export async function updateDevice(device: Device): Promise<void> {
  return invoke("update_device", { device });
}

export async function setDeviceEnabled(
  deviceId: string,
  enabled: boolean,
): Promise<void> {
  return invoke("set_device_enabled", { deviceId, enabled });
}

export async function addMapping(
  deviceId: string,
  localPort: number,
  remotePort: number,
  remoteHost?: string,
): Promise<PortMapping> {
  return invoke("add_mapping", {
    deviceId,
    localPort,
    remotePort,
    remoteHost: remoteHost ?? null,
  });
}

export async function removeMapping(
  deviceId: string,
  mappingId: string,
): Promise<void> {
  return invoke("remove_mapping", { deviceId, mappingId });
}

export async function setMappingEnabled(
  deviceId: string,
  mappingId: string,
  enabled: boolean,
): Promise<void> {
  return invoke("set_mapping_enabled", { deviceId, mappingId, enabled });
}

export async function updateMapping(
  deviceId: string,
  mapping: PortMapping,
): Promise<void> {
  return invoke("update_mapping", { deviceId, mapping });
}

export async function getDefaultKeyPath(): Promise<string> {
  return invoke("get_default_key_path");
}

export async function quitApp(): Promise<void> {
  return invoke("quit_app");
}
