import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { enable, disable, isEnabled } from "@tauri-apps/plugin-autostart";
import {
  addDevice,
  addMapping,
  getConfig,
  getDefaultKeyPath,
  getDeviceStatuses,
  quitApp,
  removeDevice,
  removeMapping,
  saveConfig,
  setDeviceEnabled,
  setMappingEnabled,
  updateDevice,
  updateMapping,
} from "./api";
import { DeviceList } from "./components/DeviceList";
import { DevicePanel } from "./components/DevicePanel";
import { SettingsBar } from "./components/SettingsBar";
import type {
  AppConfig,
  Device,
  DeviceStatus,
  DeviceStatusEvent,
  PortMapping,
} from "./types";
import "./App.css";

const emptyConfig: AppConfig = {
  devices: [],
  autoStart: false,
  closeToTray: true,
};

function App() {
  const [config, setConfig] = useState<AppConfig>(emptyConfig);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [statuses, setStatuses] = useState<Record<string, DeviceStatus>>({});
  const [messages, setMessages] = useState<Record<string, string>>({});
  const [defaultKeyPath, setDefaultKeyPath] = useState("~/.ssh/id_ed25519");
  const [error, setError] = useState<string | null>(null);
  const saveTimer = useRef<number | null>(null);
  const draftRef = useRef<Device | null>(null);

  const selected = useMemo(
    () => config.devices.find((d) => d.id === selectedId) ?? null,
    [config.devices, selectedId],
  );

  const refresh = useCallback(async () => {
    const [cfg, st, keyHint] = await Promise.all([
      getConfig(),
      getDeviceStatuses(),
      getDefaultKeyPath(),
    ]);
    setConfig(cfg);
    setDefaultKeyPath(keyHint);
    const map: Record<string, DeviceStatus> = {};
    const msgs: Record<string, string> = {};
    for (const [id, ev] of Object.entries(st)) {
      map[id] = ev.status;
      if (ev.message) msgs[id] = ev.message;
    }
    setStatuses(map);
    setMessages(msgs);
    setSelectedId((prev) => {
      if (prev && cfg.devices.some((d) => d.id === prev)) return prev;
      return cfg.devices[0]?.id ?? null;
    });
  }, []);

  useEffect(() => {
    refresh().catch((e) => setError(String(e)));
    const unlisten = listen<DeviceStatusEvent>("device-status", (event) => {
      const { deviceId, status, message } = event.payload;
      setStatuses((prev) => ({ ...prev, [deviceId]: status }));
      setMessages((prev) => {
        const next = { ...prev };
        if (message) next[deviceId] = message;
        else delete next[deviceId];
        return next;
      });
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [refresh]);

  const persistDevice = useCallback(async (device: Device) => {
    try {
      await updateDevice(device);
      await refresh();
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }, [refresh]);

  const scheduleDeviceSave = useCallback(
    (device: Device) => {
      draftRef.current = device;
      setConfig((prev) => ({
        ...prev,
        devices: prev.devices.map((d) => (d.id === device.id ? device : d)),
      }));
      if (saveTimer.current) window.clearTimeout(saveTimer.current);
      saveTimer.current = window.setTimeout(() => {
        const d = draftRef.current;
        if (d) void persistDevice(d);
      }, 400);
    },
    [persistDevice],
  );

  const handleAddDevice = async () => {
    const host = window.prompt("主机地址（IP 或域名）", "");
    if (!host?.trim()) return;
    const name = window.prompt("设备名称", host.trim()) ?? host.trim();
    try {
      const device = await addDevice(name.trim(), host.trim());
      await refresh();
      setSelectedId(device.id);
    } catch (e) {
      setError(String(e));
    }
  };

  const handleRemoveDevice = async (id: string) => {
    if (!window.confirm("确定删除该设备及其端口映射？")) return;
    try {
      await removeDevice(id);
      await refresh();
    } catch (e) {
      setError(String(e));
    }
  };

  const handleToggleDevice = async (enabled: boolean) => {
    if (!selectedId) return;
    // Flush pending draft first
    if (draftRef.current && draftRef.current.id === selectedId) {
      if (saveTimer.current) window.clearTimeout(saveTimer.current);
      await persistDevice(draftRef.current);
    }
    try {
      await setDeviceEnabled(selectedId, enabled);
      await refresh();
    } catch (e) {
      setError(String(e));
    }
  };

  const handleAddMapping = async () => {
    if (!selectedId) return;
    const local = Number(window.prompt("本地端口", "8080"));
    const remote = Number(window.prompt("远程端口", "80"));
    if (!local || !remote) return;
    try {
      await addMapping(selectedId, local, remote);
      await refresh();
    } catch (e) {
      setError(String(e));
    }
  };

  const handleUpdateMapping = async (mapping: PortMapping) => {
    if (!selectedId) return;
    setConfig((prev) => ({
      ...prev,
      devices: prev.devices.map((d) =>
        d.id !== selectedId
          ? d
          : {
              ...d,
              mappings: d.mappings.map((m) =>
                m.id === mapping.id ? mapping : m,
              ),
            },
      ),
    }));
    try {
      await updateMapping(selectedId, mapping);
    } catch (e) {
      setError(String(e));
      await refresh();
    }
  };

  const handleToggleMapping = async (mappingId: string, enabled: boolean) => {
    if (!selectedId) return;
    try {
      await setMappingEnabled(selectedId, mappingId, enabled);
      await refresh();
    } catch (e) {
      setError(String(e));
    }
  };

  const handleRemoveMapping = async (mappingId: string) => {
    if (!selectedId) return;
    try {
      await removeMapping(selectedId, mappingId);
      await refresh();
    } catch (e) {
      setError(String(e));
    }
  };

  const handleAutoStart = async (v: boolean) => {
    const next = { ...config, autoStart: v };
    setConfig(next);
    try {
      await saveConfig(next);
      if (v) await enable();
      else await disable();
      // Keep plugin state in sync even if enable fails on unsupported env
      const actual = await isEnabled().catch(() => v);
      if (actual !== v) {
        const synced = { ...next, autoStart: actual };
        setConfig(synced);
        await saveConfig(synced);
      }
    } catch (e) {
      setError(String(e));
      await refresh();
    }
  };

  const handleCloseToTray = async (v: boolean) => {
    const next = { ...config, closeToTray: v };
    setConfig(next);
    try {
      await saveConfig(next);
    } catch (e) {
      setError(String(e));
      await refresh();
    }
  };

  const handleQuit = async () => {
    try {
      await quitApp();
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <div className="app-shell">
      <header className="app-header">
        <div className="brand">SSH Tunnel Manager</div>
        <div className="subtitle">多设备本地端口转发</div>
      </header>
      {error ? (
        <div className="error-banner" onClick={() => setError(null)}>
          {error}
        </div>
      ) : null}
      <main className="app-main">
        <DeviceList
          devices={config.devices}
          selectedId={selectedId}
          statuses={statuses}
          onSelect={setSelectedId}
          onAdd={handleAddDevice}
          onRemove={handleRemoveDevice}
        />
        <DevicePanel
          device={selected}
          status={selected ? statuses[selected.id] ?? "disconnected" : "disconnected"}
          statusMessage={selected ? messages[selected.id] : null}
          defaultKeyPath={defaultKeyPath}
          onChange={scheduleDeviceSave}
          onToggleEnabled={handleToggleDevice}
          onAddMapping={handleAddMapping}
          onUpdateMapping={handleUpdateMapping}
          onToggleMapping={handleToggleMapping}
          onRemoveMapping={handleRemoveMapping}
        />
      </main>
      <SettingsBar
        autoStart={config.autoStart}
        closeToTray={config.closeToTray}
        onAutoStartChange={handleAutoStart}
        onCloseToTrayChange={handleCloseToTray}
        onQuit={handleQuit}
      />
    </div>
  );
}

export default App;
