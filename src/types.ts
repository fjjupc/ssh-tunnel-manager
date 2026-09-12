export type AuthMethod =
  | { type: "password"; password: string }
  | { type: "key"; privateKeyPath?: string | null };

export type PortMapping = {
  id: string;
  localPort: number;
  remoteHost: string;
  remotePort: number;
  enabled: boolean;
};

export type Device = {
  id: string;
  name: string;
  host: string;
  sshPort?: number | null;
  username: string;
  auth: AuthMethod;
  enabled: boolean;
  mappings: PortMapping[];
};

export type AppConfig = {
  devices: Device[];
  autoStart: boolean;
  closeToTray: boolean;
};

export type DeviceStatus =
  | "disconnected"
  | "connecting"
  | "connected"
  | "reconnecting";

export type DeviceStatusEvent = {
  deviceId: string;
  status: DeviceStatus;
  message?: string | null;
};

export const STATUS_LABEL: Record<DeviceStatus, string> = {
  disconnected: "已断开",
  connecting: "连接中",
  connected: "已连接",
  reconnecting: "重连中",
};
