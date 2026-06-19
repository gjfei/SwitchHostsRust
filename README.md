# switch-hosts-rust

[SwitchHosts](https://github.com/oldj/SwitchHosts) v5 功能的 Rust 重写实现。

> **非官方项目（Unofficial）**  
> 本项目由社区独立开发，与 [SwitchHosts](https://github.com/oldj/SwitchHosts) 及其作者 **无隶属、无授权、无背书关系**。  
> 「SwitchHosts」名称仅用于说明功能来源；SwitchHostsRust 是本项目的产品名，不代表官方版本。

- **数据目录**：`~/.SwitchHostsRust`（Windows：`%USERPROFILE%\.SwitchHostsRust`）
- **开发 hosts 写入目标**：`~/.SwitchHostsRust/internal/dev/test.hosts`（默认；使用 `--system` 写入真实 `/etc/hosts`）

## 构建

```bash
cargo build --workspace
cargo test --workspace
```

## CLI

```bash
cargo run -p cli -- list
cargo run -p cli -- toggle <id>
cargo run -p cli -- apply
cargo run -p cli -- apply --system   # 写入系统 hosts
```

## GUI

```bash
cargo run -p egui-app                              # Debug：写入 dev test.hosts
cargo run -p egui-app -- --system                  # 写入系统 /etc/hosts
cargo run --release -p egui-app                    # Release 默认写入系统 hosts
```

### macOS 封装 `.app`

依赖 [cargo-bundle](https://github.com/burtonageo/cargo-bundle)，配置见 `apps/egui/Cargo.toml` 的 `[package.metadata.bundle]`。

```bash
./scripts/package-macos.sh
# 产物: dist/SwitchHostsRust.app
#       target/release/bundle/osx/SwitchHostsRust.app

open dist/SwitchHostsRust.app
```

可选生成 DMG：

```bash
hdiutil create -volname SwitchHostsRust \
  -srcfolder dist/SwitchHostsRust.app \
  -ov -format UDZO dist/SwitchHostsRust.dmg
```

或手动：

```bash
cargo install cargo-bundle --locked   # 首次
cargo bundle --release -p egui-app
open target/release/bundle/osx/SwitchHostsRust.app
```

## Crates

| Crate | 职责 |
|-------|------|
| `core` | 存储、切换、hosts 写入、查找、导入导出 |
| `service` | HTTP 客户端、本地 API :50761、定时刷新 |
| `cli` | 命令行工具 |
| `apps/egui` | 桌面 GUI |

## 许可证与致谢

本项目源码以 [Apache License 2.0](LICENSE) 发布。第三方组件与资源署名见 [NOTICE](NOTICE)。

| 组件 | 许可 | 说明 |
|------|------|------|
| 本项目源码 | Apache-2.0 | 见 [LICENSE](LICENSE) |
| [SwitchHosts](https://github.com/oldj/SwitchHosts) | Apache-2.0 | 功能参考；`apps/egui/icons/` 应用图标来自原版 |
| [Tabler Icons](https://tabler.io/icons) | MIT | `apps/egui/assets/icons/` UI 图标（v3.42.0） |

与原版 SwitchHosts 的数据目录（`~/.SwitchHosts`）相互独立；导入/导出格式在 v5 备份 JSON 上尽量兼容，但不保证与官方客户端完全一致。
