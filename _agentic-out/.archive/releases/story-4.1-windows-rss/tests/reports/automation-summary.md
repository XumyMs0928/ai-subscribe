---
stepsCompleted:
  - step-01-preflight-and-context
  - step-02-identify-targets
  - step-03a-api
  - step-03b-e2e
  - step-03b-backend
  - step-03c-aggregate
  - step-04-validate-and-summarize
lastStep: step-04-validate-and-summarize
lastSaved: 2026-08-19T14:40:00+08:00
inputDocuments:
  - _agentic/config.yaml
  - _agentic-out/implementation/stories/4-1-将真实来源规范化为可溯源情报.md
  - _agentic-out/planning/architecture.md
  - crates/radar-core/src/application/demo.rs
  - crates/radar-core/src/application/sync.rs
---

# Story 4.1 测试自动化摘要

## 范围与策略

- 模式：Integrated；Windows-first、RSS/Atom-only、backend-first。
- 本 Story 没有新增 HTTP/OpenAPI、页面、路由或 Tauri command，因此 API 与 E2E 严格去重后均无需新增测试。
- agent-team 槽位不可用，按工作流降级为顺序执行；没有伪造并行 worker 输出。
- 所有命令使用项目内 Rust/Node 工具链；未安装全局依赖，未修改 Python 或系统设置。

## 新增自动化证据

| 测试 | 层级 | 优先级 | 覆盖 |
| --- | --- | --- | --- |
| `v7_results_backfill_one_stable_fact_with_deterministic_tie_break` | migration/integration | P0 | 同身份多轮快照按稳定 tie-break 回填为一个 fact，全部历史 result 关联同一 intel ID |
| `terminal_history_pruning_preserves_fact_provenance_and_checkpoint` | persistence/unit | P0 | 终态历史裁剪仅删除 run/result，保留 fact、provenance、checkpoint |

## 验证

- API worker JSON：PASS，0 tests，HTTP/OpenAPI N/A。
- E2E worker JSON：PASS，0 tests；复用 Story 2.6 结果页 seam 回归。
- Backend worker JSON：PASS，新增 2 个 P0 Rust 测试。
- 两个新增测试均定向通过；第一次 retention 测试因测试数据违反既有 `created_at_ms >= 1` 约束失败，修正 fixture 后仅续跑该失败项并通过。
- 未在本阶段重复运行完整仓库测试；最终合并门将在 code-review/test-review 修复收敛后统一执行一次。

## 结论

Story 4.1 测试自动化阶段 PASS。没有新增 fixture、UI 测试或公网依赖；下一步进入一次性完整代码审查。
