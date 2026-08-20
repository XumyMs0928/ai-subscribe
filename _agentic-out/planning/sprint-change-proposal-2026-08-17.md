---
artifact_kind: sprint-change-proposal
status: approved
change_scope: moderate
delivery_scope: windows-first-rss-minimum-loop
date: 2026-08-17
approved_by: xmy
approved_at: 2026-08-17T17:37:43+08:00
---

# Sprint Change Proposal：第一阶段收缩为 15 个 Story 的 Windows 最小闭环

## 1. Issue Summary

### 触发原因

Story 2.1 进入 review 后，现有计划仍有 9 个 Epic、53 个 Story。若继续按完整跨平台 MVP 顺序推进，需要同时完成三类来源、GitHub 自动发现、AI、通知、反馈、移动生命周期和全平台发布门禁，无法满足“精简开发、快速完成闭环”的当前目标。

本次变更属于 **MVP 范围战略收缩**，不是技术失败，也不删除既有需求。目标是先交付一条真实、可安装、可验证的 Windows 垂直闭环，再逐步恢复其余能力。

### 第一阶段成功定义

用户能够在 Windows 上：

1. 启动应用并理解产品价值；
2. 配置关注规则和一个公开 RSS/Atom 来源；
3. 手动执行安全增量同步；
4. 查看本轮同步结果；
5. 将真实内容规范化、确定性去重并按透明规则分流；
6. 在高价值主流中浏览、筛选、查看证据详情并用系统浏览器打开原文；
7. 安装或升级内部候选构建，且本地数据库迁移安全。

第一阶段明确不要求 GitHub Release、arXiv、GitHub 自动发现、AI、通知、反馈、收藏正文、全文搜索、托盘、移动端或公开商店发布。

## 2. Impact Analysis

### Epic Impact

- Epic 1：保留已完成的 Windows 基础 Story；1.3–1.5 继续延期。
- Epic 2：第一阶段只交付 RSS/Atom；2.5、2.6 使用 RSS-only 验收覆盖，三来源完整验收保留到第二阶段。
- Epic 3：整体延期，保留全部 Story。
- Epic 4：第一阶段只交付规范化、去重、透明规则、主流和详情；处理状态、全文搜索、收藏正文延期。
- Epic 5–7：整体延期。AI 是可选增强，不阻塞无 Key 的本地规则闭环。
- Epic 8：整体延期；第一阶段只要求已落 SQLite 内容可在无新网络请求时继续读取，不宣称完成完整离线/重试 Epic。
- Epic 9：只纳入 9.4 的当前能力子集，用于内部 Windows 安装候选和现有 schema 安全迁移；跨平台发布 Story 继续延期。

### Artifact Conflicts

- PRD 当前把三来源、AI/通知体验和多平台描述为统一 MVP，需要新增“第一阶段 Windows 最小闭环”覆盖层，并明确完整 MVP 未删除。
- Epics 当前写明三类来源均为 Phase 1 门禁，需要改为“第一阶段 RSS 垂直切片，完整三来源为第二阶段门禁”。
- Architecture 当前大量结构按最终能力设计；无需推翻，只需新增第一阶段激活组件/禁用组件清单，禁止提前创建延期模块。
- UX 当前包含完整 AI、通知、收藏、移动旅程；需新增第一阶段可见导航和状态裁剪，未实现入口不得显示假功能。
- Sprint status 需要增加唯一的 `phase_1_story_ids` 与 `deferred_after_phase_1`，避免 Agentic Flow 自动继续全量 backlog。

### Technical Impact

- 复用现有 Rust core、SQLite owner、DesktopApi、Tauri、React Query 和 Windows UI。
- 只实现一个真实网络适配器 RSS/Atom，但保留通用 source/task 接口供后续 GitHub/arXiv 接入。
- 不引入 AI provider、通知权限、移动工具链、GitHub OAuth、云服务或第二数据库。
- 网络测试使用固定本地 fixtures/受控测试服务器；正式测试不依赖公网。

## 3. Recommended Approach

采用 **Hybrid：Direct Adjustment + MVP Review**。

- 不回滚已完成工作；回滚收益低、风险高。
- 不新增 Epic；现有领域划分仍有效。
- 为现有 Story 添加第一阶段验收覆盖层，完整 AC 原文保留为后续阶段目标。
- 第一阶段固定为 15 个 Story，其中 6 个已完成或处于 review，只剩 9 个实现项。

### Effort / Risk

- 剩余工作：9 个 Story，建议按 4 个连续实现批次完成。
- 预计范围压缩：从 53 个 Story 降到第一阶段 15 个；相对原计划尚未完成的 47 个 Story，第一阶段只剩 9 个实现项。
- 实施风险：中。主要风险是 RSS 网络安全、同步幂等、数据库迁移和 2.5/2.6/4.5 的阶段性 AC 漂移。
- 时间影响：显著缩短；三来源、AI、通知和移动工具链不再阻塞首次闭环。

## 4. Detailed Change Proposals

### 4.1 第一阶段 Story 清单（固定 15 个）

| 顺序 | Story | 第一阶段作用 | 当前状态 |
|---:|---|---|---|
| 1 | 1.1 | 共享核心与契约 | done |
| 2 | 1.2 | Windows 原生壳 | done |
| 3 | 1.6 | 无门槛演示数据 | done |
| 4 | 1.7 | 列表与证据详情基础 | done |
| 5 | 1.8 | 默认赛道与可恢复引导 | done |
| 6 | 2.1 | 当前设备关注配置 | review |
| 7 | 2.2 | RSS/Atom 安全增量同步 | backlog |
| 8 | 2.5 | RSS-only 手动同步编排与状态 | backlog |
| 9 | 2.6 | RSS-only 本轮最小结果 | backlog |
| 10 | 4.1 | 真实来源规范化与溯源 | backlog |
| 11 | 4.2 | 确定性去重与事件关联 | backlog |
| 12 | 4.3 | 透明规则评分与主流分流 | backlog |
| 13 | 4.5 | Windows 高价值主流与必要筛选 | backlog |
| 14 | 4.6 | 证据详情与安全打开原文 | backlog |
| 15 | 9.4 | Windows 内部候选安装/升级与迁移保护 | backlog |

### 4.2 实施批次

1. **批次 A：真实采集** — 2.2 → 2.5 → 2.6。
2. **批次 B：情报事实** — 4.1 → 4.2。
3. **批次 C：日常价值** — 4.3 → 4.5 → 4.6。
4. **批次 D：交付闭环** — 9.4 当前 schema/内部 Windows 候选子集。

### 4.3 Story 2.5 阶段覆盖

**OLD：** RSS/Atom、GitHub Release 和 arXiv 三类来源均是 Phase 1 交付门禁。

**NEW（第一阶段覆盖）：**

- 同步编排器只要求 RSS/Atom adapter 真正接通；结构必须允许后续增加其他 adapter。
- 支持“同步全部已启用来源”和“立即同步单个 RSS 来源”；第一阶段已启用真实来源只有 RSS/Atom。
- `SourceDeliveryReadiness` 只对 `phase_1_required = rss_atom` 判定第一阶段完成。
- GitHub Release、arXiv 显示为“后续阶段未启用”或不显示，不得伪装为失败、未配置或第一阶段阻塞。
- 第一阶段只要求前台手动同步；计划调度、后台常驻和移动 ExecutionBudget 延期。

完整三来源 AC 原文保留，第二阶段恢复。

### 4.4 Story 2.6 阶段覆盖

**OLD：** 按三类来源展示单源和全部来源结果。

**NEW（第一阶段覆盖）：**

- 只要求 RSS/Atom 的新增、更新、跳过、失败计数和最小结果列表。
- “全部来源”表示当前设备所有已启用且已实现的第一阶段来源，不暗示三类来源已经完成。
- 保留 `SyncResultSummary` 的来源分组结构，后续增量加入 GitHub/arXiv 时不改 v1 语义。
- 无 AI、评分或搜索时结果仍可消费。

### 4.5 Story 4.5 阶段覆盖

**OLD：** 筛选同时覆盖赛道、来源、时间、重要度、处理状态和收藏状态，并要求完整 50,000 条全平台门禁。

**NEW（第一阶段覆盖）：**

- 必要筛选只包含赛道、来源、时间和重要度。
- 处理状态、收藏状态对应 Story 4.4/4.8，第一阶段不显示相关控件。
- Windows 固定数据集性能仍需自动门禁；正式样本使用用户已批准的 30 次口径。完整跨平台/50,000 条发布矩阵保留到后续发布阶段。

### 4.6 Story 4.6 阶段覆盖

- 必须展示原始事实、规则依据、溯源和安全原文入口。
- AI 区块只允许显示明确的“未配置/本阶段未启用”，不得生成模拟 AI 结论。
- 只验收 Windows 原位详情；移动列表栈/平板分栏继续 Deferred。

### 4.7 Story 9.4 阶段覆盖

- 只覆盖 Windows 内部候选：全新安装、当前已支持 schema 升级、迁移失败保护、本地数据说明和卸载说明。
- 不要求未来尚不存在的收藏、反馈、GitHub 发现、AI、通知数据迁移。
- 不要求公开代码签名证书、商店提交或自动更新；输出明确标识的内部测试安装包。
- 完整生产发布门禁原文保留到后续阶段。

### 4.8 延期范围

- Epic 1：1.3、1.4、1.5。
- Epic 2：2.3、2.4，以及 2.5/2.6 的完整三来源 AC。
- Epic 3：3.1–3.7。
- Epic 4：4.4、4.7、4.8，以及 4.5/4.6 的完整跨平台/高级状态 AC。
- Epic 5：5.1–5.4。
- Epic 6：6.1–6.4。
- Epic 7：7.1–7.4。
- Epic 8：8.1–8.5。
- Epic 9：9.1–9.3、9.5–9.7，以及 9.4 的完整生产发布矩阵。

延期不是取消；除第一阶段覆盖层外，原 Story 和 AC 均保留。

### 4.9 Canonical Artifact Edits after Approval

- PRD：新增“Delivery Phase 1 — Windows RSS Minimum Closed Loop”章节；原 MVP 改称完整产品 MVP。
- Epics：在 Epic 列表前新增第一阶段覆盖表；为 2.5、2.6、4.5、4.6、9.4 增加阶段覆盖说明。
- Architecture：新增激活组件图和“不得提前实现”列表。
- UX：导航仅保留情报、规则、同步结果、设置；隐藏未实现的 AI、通知、收藏、反馈、GitHub 发现入口。
- Sprint status：写入 15 个 `phase_1_story_ids`、9 个 `phase_1_remaining_story_ids` 和完整延期列表；Agentic Flow 只从 remaining 列表选择下一 Story。

## 5. Checklist Status

- [x] 1.1–1.3：触发、问题和证据已明确。
- [x] 2.1–2.5：9 个 Epic 的范围、依赖和顺序已评估；无需新增或删除 Epic。
- [x] 3.1–3.4：PRD、Architecture、UX、测试/发布/Sprint artifacts 的冲突已识别。
- [x] 4.1：Direct Adjustment 可行，投入中、风险中。
- [x] 4.2：Rollback 不可取，投入中、风险高、收益低。
- [x] 4.3：MVP Review 可行，投入低、风险中。
- [x] 4.4：选择 Hybrid（直接调整 + MVP 收缩）。
- [x] 5.1–5.5：问题、影响、路径、行动和交接计划已形成。
- [x] 6.1–6.2：提案一致性与可执行性已复核。
- [x] 6.3：用户已于 2026-08-17 明确批准。
- [x] 6.4：PRD、Epics、Architecture、UX、UX Spine、artifact manifest 与 sprint-status 已同步。
- [x] 6.5：批准后由 Product Owner/Developer 更新 backlog，Developer 按 4 个批次实施。

## 6. Implementation Handoff

### Change Classification

**Moderate** — 不改变核心架构，但需要重排 backlog、增加阶段覆盖并修改多个规划 artifact。

### Responsibilities

- Product Owner / Planning：确认 15 个 Story 和阶段性 AC。
- Developer：按 A→D 批次实现剩余 9 个 Story，不进入延期模块。
- Reviewer/Test：只按第一阶段覆盖层判定 Windows milestone；不得因移动端、AI、通知或第二/第三来源未实现而失败。

### Final Success Criteria

- 15 个第一阶段 Story 全部 done；
- Windows 内部安装候选可完成 RSS 配置 → 手动同步 → 结果 → 规范化/去重 → 规则主流 → 证据详情 → 原文核验；
- 无公网依赖的自动化、真实候选冒烟和项目隔离门禁通过；
- 延期入口不显示假成功或不可用占位功能；
- 不宣称三来源、AI、通知、移动端或完整跨平台 MVP 已完成。

## 7. Approval and Handoff Log

- 2026-08-17：用户批准将第一阶段收缩为 15 个 Story，既有 Story 与完整 AC 保留、延期但不删除。
- Canonical artifacts 已同步：PRD、Epics、Architecture、UX 设计规格、DESIGN、EXPERIENCE、Sprint Status 与 artifact manifest。
- 当前 6 个 Story 已完成或处于 review，剩余 9 个按固定顺序实施。
- Product Owner / Developer 的下一项是 Story 2.2；完成后依次执行 2.5、2.6、4.1、4.2、4.3、4.5、4.6、9.4。
- Reviewer/Test 只按阶段覆盖层判定 `Windows RSS minimum-loop PASS`，延期能力不得成为第一阶段阻塞项。
