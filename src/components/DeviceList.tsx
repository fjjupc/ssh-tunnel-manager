import type { Device, DeviceStatus } from "../types";
import { STATUS_LABEL } from "../types";

type Props = {
  devices: Device[];
  selectedId: string | null;
  statuses: Record<string, DeviceStatus>;
  onSelect: (id: string) => void;
  onAdd: () => void;
  onRemove: (id: string) => void;
};

export function DeviceList({
  devices,
  selectedId,
  statuses,
  onSelect,
  onAdd,
  onRemove,
}: Props) {
  return (
    <aside className="device-list">
      <div className="panel-header">
        <h2>目标设备</h2>
        <button type="button" className="btn btn-primary btn-sm" onClick={onAdd}>
          添加
        </button>
      </div>
      <ul className="device-items">
        {devices.length === 0 && (
          <li className="empty-hint">暂无设备，点击添加开始配置</li>
        )}
        {devices.map((d) => {
          const status = statuses[d.id] ?? "disconnected";
          return (
            <li
              key={d.id}
              className={`device-item ${selectedId === d.id ? "active" : ""}`}
              onClick={() => onSelect(d.id)}
            >
              <span className={`status-dot status-${status}`} title={STATUS_LABEL[status]} />
              <div className="device-meta">
                <div className="device-name">{d.name || d.host}</div>
                <div className="device-host">{d.host}</div>
              </div>
              <button
                type="button"
                className="btn btn-ghost btn-sm"
                title="删除设备"
                onClick={(e) => {
                  e.stopPropagation();
                  onRemove(d.id);
                }}
              >
                删
              </button>
            </li>
          );
        })}
      </ul>
    </aside>
  );
}
