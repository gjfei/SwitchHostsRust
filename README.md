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
cargo dev egui                                     # 单次运行（默认 Debug）
cargo dev gpui
cargo dev-watch egui                               # 热重载
cargo dev egui -- --system                         # 传参给 app
cargo list-apps                                    # 列出 app/ 下所有 app
cargo dev gpui
```

macOS 开发时若需 Mission Control / Dock 正确图标（需 `Packager.toml` 已配置）：

```bash
cargo run-app-macos egui
```

### macOS 封装 `.dmg`

使用 [cargo-packager](https://github.com/crabnebula-dev/cargo-packager)，仅打包 `app/` 下的 crate，配置见 `Packager.toml` 的 `[[apps]]`。

```bash
cargo package-macos
# 产物: dist/SwitchHostsRust.dmg

# 仅打包 GUI
cargo package-macos --app egui-app

# 仅 .app（开发调试，写入 target/packager/）
cargo package-macos --app-only
```

### Windows 封装 NSIS 安装包

在 Windows 上运行（cargo-packager 会自动下载 NSIS）：

```bash
cargo package-windows
# 产物: dist/SwitchHostsRust_<version>_<arch>-setup.exe
```

### 发布

推送 tag 后由 GitHub Actions（`.github/workflows/release.yml`）自动更新版本并创建 Release（macOS `.dmg` + Windows NSIS 安装包）：

```bash
git tag v0.2.0
git push origin v0.2.0

# 仅发布单个 app
git tag egui-app-v0.2.0
git push origin egui-app-v0.2.0
```

Tag 格式：

| Tag | 作用 |
|-----|------|
| `v0.2.0` | 打包全部 `app/` 下 app |
| `egui-app-v0.2.0` | 仅打包 `egui-app` |

本地预检：

```bash
cargo run -p xtask -- release-prepare --tag v0.2.0 --dry-run
cargo run -p xtask -- release-prepare --tag v0.2.0
cargo run -p xtask -- package-macos
```

## Crates

| Crate | 职责 |
|-------|------|
| `core` | 存储、切换、hosts 写入、查找、导入导出 |
| `service` | HTTP 客户端、本地 API :50761、定时刷新 |
| `cli` | 命令行工具 |
| `app/egui` | 桌面 GUI |

## 许可证与致谢

本项目源码以 [Apache License 2.0](LICENSE) 发布。第三方组件与资源署名见 [NOTICE](NOTICE)。

| 组件 | 许可 | 说明 |
|------|------|------|
| 本项目源码 | Apache-2.0 | 见 [LICENSE](LICENSE) |
| [SwitchHosts](https://github.com/oldj/SwitchHosts) | Apache-2.0 | 功能参考；`crates/ui-assets/app-icons/` 应用图标来自原版 |
| [Tabler Icons](https://tabler.io/icons) | MIT | `crates/ui-assets/assets/icons/` UI 图标（v3.42.0） |

与原版 SwitchHosts 的数据目录（`~/.SwitchHosts`）相互独立；导入兼容 v3/v4/v5 备份 JSON，导出为 v5 格式。
