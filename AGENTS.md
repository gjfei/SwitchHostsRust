# AGENTS.md

SwitchHostsRust 的 AI 协作指南。非官方社区项目，对齐 [SwitchHosts](https://github.com/oldj/SwitchHosts) v5 功能与 UI，Rust + egui 实现。

## 项目概览

| 项 | 值 |
|----|-----|
| 许可证 | Apache-2.0（见 `LICENSE`，第三方见 `NOTICE`） |
| Rust | 1.75+，edition 2021 |
| 数据目录 | `~/.SwitchHostsRust`（与官方 `~/.SwitchHosts` **独立**） |
| 远程 | https://github.com/gjfei/SwitchHostsRust |

## Workspace 结构

```
switch-hosts-rust/
├── crates/core/      # 业务核心：存储、切换、写入、导入导出
├── crates/service/   # HTTP 客户端、本地 API :50761、定时刷新
├── crates/cli/       # CLI 二进制 switch-hosts-rust（default member）
├── apps/egui/        # GUI：egui-app / switch-hosts-rust-gui
├── tests/fixtures/   # 集成测试共享数据
├── scripts/          # dev-gui、package-macos 等
└── docs/phases/      # 分阶段交付文档
```

**依赖方向：** `apps/egui` → `service` → `core`。GUI 不反向被 core 依赖。

## 常用命令

```bash
# 全 workspace
cargo build --workspace
cargo test --workspace

# CLI（默认 member，需显式 -p）
cargo run -p cli -- list
cargo run -p cli -- apply --system

# GUI
cargo run -p egui-app                              # Debug → dev test.hosts
cargo run -p egui-app -- --system                 # 写入 /etc/hosts
cargo run --release -p egui-app                   # Release → 系统 hosts
make dev-gui                                       # cargo watch 热重载

# macOS .app / DMG
cargo package-macos                                  # → dist/SwitchHostsRust.app
cargo package-dmg                                    # → dist/SwitchHostsRust.dmg
```

**注意：** `cargo run` 不带 `-p` 会跑 CLI，不是 GUI。GUI 参数需 `--` 分隔：`cargo run -p egui-app -- --system`。

## 入口与架构

| 组件 | 路径 |
|------|------|
| GUI main | `apps/egui/src/main.rs` |
| 应用状态与帧循环 | `apps/egui/src/app.rs`（`SwitchHostsApp`） |
| 面板 | `apps/egui/src/panels/` |
| 主题 / 间距 | `apps/egui/src/theme.rs` |
| 图标 | `apps/egui/src/icons.rs` + `assets/icons/` |
| Core 公共 API | `crates/core/src/lib.rs` |
| 存储路径 | `crates/core/src/storage/paths.rs` |
| Hosts 写入 | `crates/core/src/hosts_apply/` |
| 本地 API | `crates/service/src/api/mod.rs` |

**GUI 模式：** `app.rs` 持有状态，各 `panels/*.rs` 提供 `draw_*` 函数；抽屉/模态在 `drawer.rs` 及对应 panel 中实现。主题 token 统一走 `theme.rs`，勿在面板内硬编码颜色。

**Core 模式：** 存储（manifest、entries、config、trashcan）与业务（toggle、apply pipeline、import/export）分离；系统写入经 `Elevation` trait，测试用 `MockElevation`。

## 对齐 SwitchHosts（必读）

UI/交互对齐时 **只读本地原版仓库**，不要在线搜索 GitHub 源码：

```
/Users/jarven/Desktop/project/self/SwitchHosts
```

| Rust (egui) | 原版 (Tauri/React) |
|-------------|---------------------|
| `apps/egui/src/panels/` | `src/renderer/components/` |
| `panels/details.rs` | `RightPanel/` |
| `panels/edit_hosts.rs` | `EditHostsInfo.tsx` |
| `panels/tree.rs` | `Tree/`、`List/` |
| `panels/top_bar.rs` | `TopBar/` |
| `panels/editor.rs` | `Editor/HostsEditor.tsx` |
| `crates/core/` | `src-tauri/src/`、`src/common/` |

- 布局、间距、文案以原版 TSX + `*.module.scss` 为准
- 颜色对照 `src/renderer/styles/themes/light.scss`
- 新增面板前先在本机 SwitchHosts 目录找对应组件
- 文件头 `//! 对齐 SwitchHosts ...` 注释保持与原版路径一致

## 数据路径与环境变量

**目录布局**（`crates/core/src/storage/paths.rs`）：

- `manifest.json`、`entries/`、`trashcan.json`
- `internal/config.json`、`internal/state.json`、`internal/histories/`
- `internal/dev/test.hosts` — Debug 默认写入目标

**环境变量：**

| 变量 | 作用 |
|------|------|
| `SWITCH_HOSTS_RUST_DATA_DIR` | 覆盖数据根目录 |
| `SWITCH_HOSTS_RUST_HOSTS_FILE` | 覆盖 hosts 写入文件 |
| `SWITCH_HOSTS_RUST_DISABLE_TRAY` | 跳过托盘初始化（测试） |

**Hosts 写入：** Debug → dev test.hosts；Release GUI → 系统 hosts。macOS/Linux `/etc/hosts`，Windows `System32\drivers\etc\hosts`。

## 平台相关代码

| 平台 | 位置 |
|------|------|
| macOS 提权（Security.framework + osascript 回退） | `crates/core/src/hosts_apply/platform_write.rs` |
| Linux pkexec / Windows UAC | 同上 |
| macOS 标题栏 / traffic lights | `apps/egui/src/macos.rs`、`panels/top_bar.rs` |
| macOS bundle | `apps/egui/Cargo.toml` `[package.metadata.bundle]`，Bundle ID `app.switchhostsrust` |

修改提权逻辑时参考原版 `SwitchHosts/src-tauri/src/hosts_apply/elevation.rs`，但保持 Rust 独立实现，勿逐字复制。

## 测试

```bash
cargo test --workspace
```

| 类型 | 位置 |
|------|------|
| Core 集成 | `crates/core/tests/{storage,apply,toggle}_integration.rs` |
| CLI 集成 | `crates/cli/tests/cli_integration.rs` |
| Service 集成 | `crates/service/tests/{client,api}_integration.rs` |
| GUI smoke | `apps/egui/tests/smoke.rs` |
| 共享 fixture | `tests/fixtures/` |

- 集成测试使用 `MockElevation`，不调用真实 sudo
- Core 禁用 doctest（`Cargo.toml`）
- CI：`.github/workflows/test.yml`（Ubuntu test）、`build.yml`（三平台 release build）

## 编码约定

1. **最小改动** — 只改任务相关文件，不顺手重构
2. **匹配现有风格** — 命名、错误处理（`thiserror`/`anyhow`）、模块划分与周边一致
3. **注释** — 仅解释非显而易见的业务/平台细节；对齐注释保留原版路径
4. **测试** — 仅在用户要求或覆盖真实行为时添加；优先集成测试
5. **许可证** — 不从 SwitchHosts 逐字复制大段代码；复用资源需在 `NOTICE` 中署名
6. **产品命名** — 对外说明「非官方」；Bundle/产品名 SwitchHostsRust，勿暗示官方背书

## 常见任务指引

**新增 GUI 面板：** 在 `panels/` 建模块 → `mod.rs` 导出 → `app.rs` 挂状态与 `draw_*` → 对照原版 TSX → 用 `theme.rs` token。

**修改 hosts 写入：** `hosts_apply/` pipeline → `platform_write.rs` 平台分支 → 失败时 GUI 需回滚 UI 状态（见 `toggle_and_apply_hosts`）。

**导入导出：** `crates/core/src/import_export/mod.rs` + `apps/egui/src/data_transfer.rs`；格式兼容 SwitchHosts v5 backup JSON。

**HTTP API：** 端口 50761，与原版兼容；运行时封装在 `apps/egui/src/http_api_runtime.rs`。

## 勿做

- 不要在线 fetch SwitchHosts 源码替代本地参考
- 不要使用 `net.oldj.*` 等原版 Bundle ID / 命名空间
- 不要默认 `cargo run` 测 GUI（会跑 CLI）
- 不要在测试中依赖真实 `/etc/hosts` 写入
- 不要未经用户要求提交 git 或 push

## 参考文档

- `README.md` — 构建与打包
- `NOTICE` / `LICENSE` — 法律与第三方
- `docs/phases/` — 功能分期
- `.cursor/rules/switchhosts-reference.mdc` — Cursor 规则（与本文 SwitchHosts 对齐部分一致）
