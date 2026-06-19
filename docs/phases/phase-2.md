# 阶段 2 — service crate 与高级功能

## 目标

实现 service 的 client/scheduler/api，以及 core 的 import_export、find，和 CLI 的 import/export。

## 验收清单

- [ ] service::client（HTTP、file://、32MiB 限制）
- [ ] service::scheduler（60s 扫描、启动刷新）
- [ ] service::api :50761
- [ ] core import_export + find
- [ ] CLI import / export
