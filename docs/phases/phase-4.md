# 阶段 4 — 系统托盘与打包

## 目标

系统托盘、应用生命周期、CI 多平台构建。

## 子步骤与 commit 对应

| 序号 | 范围 | commit message |
|------|------|----------------|
| 4.1 | 托盘菜单构建 | `feat(egui): 添加托盘菜单构建逻辑` |
| 4.2 | CI build | `chore: 添加 CI 多平台 build 工作流` |
| 4.3 | 打包文档 | `docs: 添加发布打包说明` |
| 4.4 | 原生托盘集成 | （后续）tray-icon 集成 |

## 验收清单

- [x] 托盘菜单构建逻辑（单测）
- [ ] 原生托盘快速切换
- [x] GitHub Actions 测试门禁
- [x] GitHub Actions 多平台 build
- [x] 发布打包文档

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

### 数据目录

默认 `~/.SwitchHostsRust`；Debug 构建写入 `internal/dev/test.hosts`。

### 后续

- macOS: 可使用 `cargo install cargo-bundle` 生成 `.app`
- Windows: 使用 WiX/NSIS 打包 `switch-hosts-rust-gui.exe`
- Linux: `.deb` / AppImage 待集成
