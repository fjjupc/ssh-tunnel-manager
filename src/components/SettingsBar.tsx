type Props = {
  autoStart: boolean;
  closeToTray: boolean;
  onAutoStartChange: (v: boolean) => void;
  onCloseToTrayChange: (v: boolean) => void;
  onQuit: () => void;
};

export function SettingsBar({
  autoStart,
  closeToTray,
  onAutoStartChange,
  onCloseToTrayChange,
  onQuit,
}: Props) {
  return (
    <footer className="settings-bar">
      <label className="switch">
        <input
          type="checkbox"
          checked={autoStart}
          onChange={(e) => onAutoStartChange(e.target.checked)}
        />
        <span>开机自启</span>
      </label>
      <label className="switch">
        <input
          type="checkbox"
          checked={closeToTray}
          onChange={(e) => onCloseToTrayChange(e.target.checked)}
        />
        <span>关闭窗口时最小化到托盘</span>
      </label>
      <div className="spacer" />
      <button type="button" className="btn btn-danger" onClick={onQuit}>
        直接退出
      </button>
    </footer>
  );
}
