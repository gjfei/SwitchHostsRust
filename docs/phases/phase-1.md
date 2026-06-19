# 阶段 1 — 核心库与数据层

## 目标

实现 storage、tree_format、toggle、hosts_apply，以及 CLI 的 list/toggle/apply。

## 子步骤与 commit 对应

| 序号 | 范围 | commit message |
|------|------|----------------|
| 1.1 | Cargo workspace、CI、`tests/fixtures/` | `chore: 搭建 Cargo workspace、CI 与测试 fixtures` |
| 1.2 | `AppPaths` | `feat(core): 实现可注入数据根的 AppPaths` |
| 1.3 | `manifest`、`tree_format` | `feat(core): 实现 manifest 与 tree_format 存储` |
| 1.4 | `entries`、`config`、`trashcan`、`state` | `feat(core): 实现 entries、config、trashcan 与 state` |
| 1.5 | `toggle`、`hosts_apply` | `feat(core): 实现 toggle 传播与 hosts 写入流水线` |
| 1.6 | CLI list/toggle/apply | `feat(cli): 实现 list、toggle 与 apply 命令` |
| 1.7 | 更新本文档 | `docs: 标记阶段 1 交付完成` |

## 验收清单

- [x] AppPaths + manifest + entries + config + trashcan + state
- [x] tree_format legacy ↔ v5 双向转换
- [x] aggregate + 去重 + HostsTarget 写入（默认 append）
- [x] toggle 传播（choice_mode / folder_mode）
- [x] CLI list / toggle / apply + assert_cmd 测试
- [x] `cargo test --workspace` 全绿
