# 阶段 2 — service crate 与高级功能

## 目标

实现 `service` 的 client/scheduler/api，以及 core 的 import_export、find、cmd_runner/history，和 CLI 的 import/export。

## 子步骤与 commit 对应

| 序号 | 范围 | commit message |
|------|------|----------------|
| 2.1 | `crates/service` 骨架 | `chore: 添加 service crate 脚手架` |
| 2.2 | `service::client` | `feat(service): 实现远程 hosts HTTP 客户端` |
| 2.3 | `service::scheduler` | `feat(service): 实现定时刷新调度器` |
| 2.4 | `core::import_export`、`find` | `feat(core): 实现 import_export、find 与 hosts_edit` |
| 2.5 | `service::api` :50761 | `feat(service): 实现本地 HTTP API（端口 50761）` |
| 2.6 | `cmd_runner`、`history` | （含于阶段 1 hosts_apply commit） |
| 2.7 | CLI import/export | （含于阶段 1 CLI commit） |
| 2.8 | 更新本文档 | `docs: 标记阶段 2 交付完成` |

## 验收清单

- [x] service::client（HTTP、file://、32MiB 限制）
- [x] service::scheduler（60s 扫描、启动刷新）
- [x] service::api :50761
- [x] core import_export + find
- [x] CLI import / export
