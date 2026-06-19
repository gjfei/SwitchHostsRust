# 阶段 1 — 核心库与数据层

## 目标

实现 storage、tree_format、toggle、hosts_apply，以及 CLI 的 list/toggle/apply。

## 验收清单

- [ ] AppPaths + manifest + entries + config + trashcan + state
- [ ] tree_format legacy ↔ v5 双向转换
- [ ] aggregate + 去重 + HostsTarget 写入（默认 append）
- [ ] toggle 传播（choice_mode / folder_mode）
- [ ] CLI list / toggle / apply + assert_cmd 测试
- [ ] `cargo test --workspace` 全绿
