# switch-hosts-rust

[SwitchHosts](https://github.com/oldj/SwitchHosts) v5 功能的 Rust 重写实现。

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

## 许可证

Apache-2.0
