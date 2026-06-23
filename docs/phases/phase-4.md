# 阶段 4 — 系统托盘与打包

## 目标

系统托盘、应用生命周期、CI 多平台构建。

## 子步骤与 commit 对应

| 序号 | 范围 | commit message |
|------|------|----------------|
| 4.1 | 托盘菜单构建 | `feat(egui): 添加托盘菜单构建逻辑` |
| 4.2 | CI build | `chore: 添加 CI 多平台 build 工作流` |
| 4.3 | 打包文档 | `docs: 添加发布打包说明` |
| 4.4 | 原生托盘 | `feat(egui): 集成原生系统托盘与快捷切换` |
| 4.5 | 生命周期 | `feat(egui): 实现应用生命周期与窗口行为` |

## 验收清单

- [x] 托盘菜单构建逻辑（单测）
- [x] 原生托盘快速切换（tray-icon + muda）
- [x] 关闭窗口最小化到托盘
- [x] 启动时隐藏主窗口（`hide_at_launch`）
- [x] 登录时启动（`auto-launch`）
- [x] GitHub Actions 测试门禁
- [x] GitHub Actions 多平台 build
- [x] 发布打包文档与 macOS 构建脚本

## 托盘行为

- 托盘菜单：显示主窗口、方案快捷切换（勾选状态）、退出
- 双击托盘图标显示主窗口
- 树面板切换方案后同步刷新托盘菜单
- CI/无 GUI 环境可设 `SWITCH_HOSTS_RUST_DISABLE_TRAY=1` 跳过托盘创建

## 打包说明

### CLI

```bash
cargo build --release -p cli
# 产物: target/release/switch-hosts-rust
```

### GUI

```bash
cargo build --release -p egui-app
# 产物: target/release/switch-hosts-rust-gui
```

或使用 xtask：

```bash
cargo package-macos
cargo package-macos --app egui-app
```

### 数据目录

默认 `~/.SwitchHostsRust`；Debug 构建写入 `internal/dev/test.hosts`。

### 后续

- macOS: `cargo package-macos` 默认生成 `.app` + `.dmg`；多 app 见 `Packager.toml` `[[apps]]`
- Windows: 使用 WiX/NSIS 打包 `switch-hosts-rust-gui.exe`
- Linux: egui 需独立 gtk 线程初始化托盘（见 tray-icon 官方 egui 示例）；`.deb` / AppImage 待集成
- macOS `hide_dock_icon`：需额外 Objective-C 集成，暂未实现
