---
artifact_kind: sprint-change-proposal
status: approved
change_scope: moderate
delivery_scope: windows-first-rss-minimum-loop
date: 2026-08-18
approved_by: xmy
approval_basis: "用户明确要求把三来源收缩为当前已实现的 RSS/Atom 来执行"
---

# Sprint Change Proposal：Story 2.5/2.6 仅执行 RSS/Atom

## 1. Issue Summary

2026-08-17 已批准第一阶段为 Windows RSS 最小闭环，但权威 Epic 中 Story 2.5 的标题、正文和部分 UX 条款仍使用“三来源 Phase 1 门禁”。这会让后续 create-story/develop-story 误把尚未实现的 GitHub Release 和 arXiv 纳入当前交付，违背快速闭环目标。

本次变更将当前执行口径固定为 RSS/Atom；完整三来源能力不删除，只延期。

## 2. Impact Analysis

- Epic 2：Story 2.5 改为 RSS/Atom 手动同步编排与来源级就绪；Story 2.6 只验收 RSS/Atom 本轮结果。
- PRD/Architecture：现有第一阶段覆盖已是 RSS-only，无需改动。
- UX：把残留的“三来源 Phase 1”表述改成“完整产品阶段 Deferred”，第一阶段只展示 RSS/Atom。
- Sprint：Story ID 和顺序不变，仅更新 2.5 的显示键及延期 AC。
- 技术：不实现 GitHub Release/arXiv adapter，不创建假状态或占位入口；保留可扩展的 source/task/result 分组合同。

## 3. Recommended Approach

采用 Direct Adjustment + 已批准的 MVP Review：不回滚 Story 2.2，不新增 Epic，不删除完整产品需求，直接修改 2.5/2.6 的当前权威 AC。

- 投入：低。
- 风险：低。
- 主要收益：Story 2.5 可直接复用当前 RSS/Atom、TaskRef、SQLite 和 Windows UI，避免为两类延期 adapter 扩大实现与测试范围。

## 4. Detailed Changes

### Story 2.5

OLD：同步 RSS/Atom、GitHub Release、arXiv，并以三来源判断 Phase 1 就绪。

NEW：只同步当前设备已启用的 RSS/Atom 来源；`SourceDeliveryReadiness.required_source = rss_atom`。GitHub Release/arXiv 不显示为失败、未配置或第一阶段阻塞。只实现 Windows 前台手动同步。

### Story 2.6

OLD：按三类来源展示本轮结果。

NEW：按 RSS/Atom 来源实例展示新增、更新、跳过、失败和最小结果；“同步全部”只表示当前设备全部已启用 RSS/Atom 来源，不暗示三来源完成。

### Deferred

GitHub Release、arXiv、三来源聚合就绪、计划/后台同步和移动 ExecutionBudget 保留到后续完整产品阶段。

## 5. Checklist and Handoff

- [x] 触发与证据：阶段覆盖和权威 Story/UX 表述冲突。
- [x] Epic 影响：只修改 Epic 2 的混合 Story，不新增/删除 Epic。
- [x] PRD/Architecture：现有 RSS-only 覆盖继续有效。
- [x] UX/Sprint：已同步当前阶段术语和 Story 显示键。
- [x] 路径选择：直接调整，不回滚。
- [x] 用户批准：本轮明确批准 RSS/Atom-only 执行。

交接给 Developer：下一项创建并实现 Story 2.5，严格按 RSS/Atom-only AC；Reviewer/Test 不得以 GitHub Release、arXiv 或三来源门禁未完成判失败。

## 6. Success Criteria

- Story 2.5 只对 RSS/Atom 创建/查询持久任务并呈现来源级状态。
- Story 2.6 只对 RSS/Atom 提供本轮可消费结果。
- UI 无 GitHub Release/arXiv 假入口、假失败或假就绪。
- 结论仅可声明 `Windows RSS minimum-loop PASS`，不得声明三来源或完整产品 MVP 完成。
