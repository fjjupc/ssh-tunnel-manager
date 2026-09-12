import type { PortMapping } from "../types";

type Props = {
  mappings: PortMapping[];
  deviceEnabled: boolean;
  onAdd: () => void;
  onUpdate: (mapping: PortMapping) => void;
  onToggle: (mappingId: string, enabled: boolean) => void;
  onRemove: (mappingId: string) => void;
};

export function PortMappingTable({
  mappings,
  deviceEnabled,
  onAdd,
  onUpdate,
  onToggle,
  onRemove,
}: Props) {
  return (
    <div className="mapping-block">
      <div className="panel-header">
        <h3>端口映射</h3>
        <button type="button" className="btn btn-primary btn-sm" onClick={onAdd}>
          添加映射
        </button>
      </div>
      <div className="mapping-table">
        <div className="mapping-head">
          <span>启用</span>
          <span>本地端口</span>
          <span>远程主机</span>
          <span>远程端口</span>
          <span />
        </div>
        {mappings.length === 0 && (
          <div className="empty-hint">暂无端口映射</div>
        )}
        {mappings.map((m) => (
          <div className="mapping-row" key={m.id}>
            <input
              type="checkbox"
              checked={m.enabled}
              disabled={!deviceEnabled}
              title={deviceEnabled ? "启用映射" : "请先启用设备"}
              onChange={(e) => onToggle(m.id, e.target.checked)}
            />
            <input
              type="number"
              min={1}
              max={65535}
              value={m.localPort}
              onChange={(e) =>
                onUpdate({ ...m, localPort: Number(e.target.value) || 0 })
              }
            />
            <input
              value={m.remoteHost}
              onChange={(e) => onUpdate({ ...m, remoteHost: e.target.value })}
            />
            <input
              type="number"
              min={1}
              max={65535}
              value={m.remotePort}
              onChange={(e) =>
                onUpdate({ ...m, remotePort: Number(e.target.value) || 0 })
              }
            />
            <button
              type="button"
              className="btn btn-ghost btn-sm"
              onClick={() => onRemove(m.id)}
            >
              删
            </button>
          </div>
        ))}
      </div>
    </div>
  );
}
