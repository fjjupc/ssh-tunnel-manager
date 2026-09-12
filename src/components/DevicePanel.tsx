import type { Device, DeviceStatus, PortMapping } from "../types";
import { STATUS_LABEL } from "../types";
import { PortMappingTable } from "./PortMappingTable";

type Props = {
  device: Device | null;
  status: DeviceStatus;
  statusMessage?: string | null;
  defaultKeyPath: string;
  onChange: (device: Device) => void;
  onToggleEnabled: (enabled: boolean) => void;
  onAddMapping: () => void;
  onUpdateMapping: (mapping: PortMapping) => void;
  onToggleMapping: (mappingId: string, enabled: boolean) => void;
  onRemoveMapping: (mappingId: string) => void;
};

export function DevicePanel({
  device,
  status,
  statusMessage,
  defaultKeyPath,
  onChange,
  onToggleEnabled,
  onAddMapping,
  onUpdateMapping,
  onToggleMapping,
  onRemoveMapping,
}: Props) {
  if (!device) {
    return (
      <section className="device-panel empty">
        <p>从左侧选择或添加一台设备</p>
      </section>
    );
  }

  const authType = device.auth.type;
  const sshPortDisplay =
    device.sshPort === null || device.sshPort === undefined || device.sshPort === 22
      ? ""
      : String(device.sshPort);

  return (
    <section className="device-panel">
      <div className="panel-header">
        <h2>设备详情</h2>
        <div className="status-line">
          <span className={`status-dot status-${status}`} />
          <span>{STATUS_LABEL[status]}</span>
          {statusMessage ? <span className="status-msg">{statusMessage}</span> : null}
        </div>
      </div>

      <div className="form-grid">
        <label>
          名称
          <input
            value={device.name}
            onChange={(e) => onChange({ ...device, name: e.target.value })}
            onBlur={() => onChange({ ...device })}
          />
        </label>
        <label>
          主机地址
          <input
            value={device.host}
            onChange={(e) => onChange({ ...device, host: e.target.value })}
          />
        </label>
        <label>
          SSH 端口
          <input
            placeholder="22"
            value={sshPortDisplay}
            onChange={(e) => {
              const v = e.target.value.trim();
              onChange({
                ...device,
                sshPort: v === "" ? null : Number(v) || null,
              });
            }}
          />
        </label>
        <label>
          用户名
          <input
            value={device.username}
            onChange={(e) => onChange({ ...device, username: e.target.value })}
          />
        </label>
      </div>

      <div className="auth-block">
        <div className="auth-tabs">
          <button
            type="button"
            className={authType === "key" ? "active" : ""}
            onClick={() =>
              onChange({
                ...device,
                auth: { type: "key", privateKeyPath: null },
              })
            }
          >
            证书
          </button>
          <button
            type="button"
            className={authType === "password" ? "active" : ""}
            onClick={() =>
              onChange({
                ...device,
                auth: { type: "password", password: "" },
              })
            }
          >
            密码
          </button>
        </div>
        {authType === "password" ? (
          <label>
            密码
            <input
              type="password"
              value={device.auth.password}
              onChange={(e) =>
                onChange({
                  ...device,
                  auth: { type: "password", password: e.target.value },
                })
              }
            />
          </label>
        ) : (
          <label>
            私钥路径
            <input
              placeholder={`默认: ${defaultKeyPath}`}
              value={device.auth.privateKeyPath ?? ""}
              onChange={(e) =>
                onChange({
                  ...device,
                  auth: {
                    type: "key",
                    privateKeyPath: e.target.value.trim() || null,
                  },
                })
              }
            />
          </label>
        )}
      </div>

      <div className="enable-row">
        <label className="switch">
          <input
            type="checkbox"
            checked={device.enabled}
            onChange={(e) => onToggleEnabled(e.target.checked)}
          />
          <span>启用设备（启用后自动连接）</span>
        </label>
      </div>

      <PortMappingTable
        mappings={device.mappings}
        deviceEnabled={device.enabled}
        onAdd={onAddMapping}
        onUpdate={onUpdateMapping}
        onToggle={onToggleMapping}
        onRemove={onRemoveMapping}
      />
    </section>
  );
}
