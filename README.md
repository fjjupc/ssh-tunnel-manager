# SSH Tunnel Manager

跨平台 SSH 隧道管理器：多设备、多端口本地转发、断线指数退避重连、系统托盘与开机自启。

技术栈：**Tauri 2 + React + TypeScript**，SSH 使用 Rust `russh`。

## 功能

- 左侧管理目标设备（添加 / 删除 / 选择）
- 右侧配置 SSH 端口、用户名、密码或私钥认证
- 私钥路径可留空，自动使用平台默认 `~/.ssh/id_ed25519`（其次 `id_rsa` 等）
- 每台设备可配置多个本地→远程端口映射，独立启用
- 启用设备后自动连接；启用端口后自动转发
- 断线后指数退避重连（1s → 2s → 4s … 最大 60s），generation 防止时序冲突
- 配置持久化，下次启动恢复启用状态并自动重连
- 关闭窗口可最小化到托盘；托盘或「直接退出」可完全退出并断开连接
- 开机自启（Windows / macOS / Linux）

## 开发环境

- Node.js 18+
- Rust stable（建议 1.85+）
- 系统依赖：
  - Windows：WebView2（通常已预装）
  - Linux：`webkit2gtk`、`libappindicator` 等（见 [Tauri 文档](https://v2.tauri.app/start/prerequisites/)）
  - macOS：Xcode CLT

```bash
npm install
npm run tauri dev
```

## 构建

```bash
npm run tauri build
```

产物位于 `src-tauri/target/release/bundle/`（msi / deb / dmg 等，视平台而定）。

## 配置位置

配置保存在**当前登录用户**的系统应用数据目录中的 `config.json`（运行时由 Tauri `app_data_dir` 自动解析，不写死任何用户名）：

- Windows：`%APPDATA%\app.ssh-tunnel-manager\config.json`
- macOS：`~/Library/Application Support/app.ssh-tunnel-manager/config.json`
- Linux：`~/.local/share/app.ssh-tunnel-manager/config.json`

其中 `%APPDATA%` / `~` 均为当前用户环境变量/主目录。换设备、换账号会自动落到对应用户目录。

密码以明文保存在本地配置中（个人工具首版约定）；请勿在不受信任环境共享配置文件。

## 使用提示

1. 添加设备并填写主机、用户名与认证信息
2. 打开「启用设备」等待状态变为「已连接」
3. 添加端口映射并勾选启用
4. 本机访问 `127.0.0.1:<本地端口>` 即转发到远程

## License

见 [LICENSE](LICENSE)。
