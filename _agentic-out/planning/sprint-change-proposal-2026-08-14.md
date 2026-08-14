---
title: Sprint Change Proposal - Windows-first 交付调整
project: ai-subscribe
date: 2026-08-14
status: implemented
change_scope: moderate
delivery_scope: windows-first
approved_incrementally: true
implemented_at: '2026-08-14T10:36:18+08:00'
---

# Sprint Change Proposal：Windows-first 交付调整

## 1. 问题摘要

Story 1.2（启动 Windows 原生应用壳并接入共享核心）实施期间，产品交付优先级调整为先快速落地 Windows 全部功能，Apple、Android、手机和平板能力保留但暂不实施。

这是一项交付顺序调整，不是技术方案失败，也不删除最终跨平台需求。当前 Story 1.1 已完成、Story 1.2 正在进行，移动端 Story 尚未实施，因此无需回滚现有代码。

## 2. 影响分析

### Epic 与 Story 影响

- 现有 9 个 Epic、53 个 Story 及编号保持不变。
- 当前实施范围统一为共享核心、Windows 平台层和 Windows UI。
- Story 1.3、1.4、1.5、8.4、9.5、9.6、9.7 延期至移动端阶段。
- 混合 Story 中 Apple、Android、手机和平板专属验收条款在当前 Windows 里程碑中标记为 deferred/N/A，原文保留。
- Windows 里程碑通过不代表跨平台 MVP 完成；移动端延期项不得被标记为已实施。

### 规划产物影响

- `prd.md`：不修改。最终跨平台产品目标和需求保持不变。
- `architecture.md`：不修改。目标架构及跨平台边界继续保留。
- `ux-design-specification.md` 与 UX spine：不修改。当前实施使用 Windows 规范，移动端规范留待后续。
- `epics.md`：只增加 Windows-first 交付覆盖规则，不重写或重新编号 Story。
- `sprint-status.yaml`：增加当前交付范围、Windows 里程碑和移动端延期清单。

### 技术影响

- 当前不安装、不构建 Apple/Android 工具链和依赖。
- 继续维护共享核心的跨平台可演进边界，不引入 Windows UI 领域规则分叉。
- 每个 Story 继续执行 Windows 适用的 contracts、自动化、代码审查、traceability 和 NFR 门禁。
- Rust、Node、Python 及其他安装包必须保持项目隔离，不影响全局环境。

## 3. 推荐方案

采用直接调整，不回滚已完成工作，不重排 53 个 Story，不修改最终跨平台产品范围。

### Windows 实施顺序

1. 完成 Story 1.2，包括真实 Windows 运行证据。
2. 延期 Stories 1.3–1.5，继续 Stories 1.6–1.8。
3. 依次实施 Epics 2–7 的共享核心和 Windows 功能。
4. 实施 Stories 8.1、8.2、8.3、8.5；延期 Story 8.4。
5. 实施 Stories 9.1–9.4 中适用于 Windows 的旅程、窗口适配、数据边界、安装升级和发布验证。
6. 达成 Windows 功能里程碑后停止；移动端延期项继续保留在 Backlog。

### 评估

- 调整工作量：低。
- 实施风险：低至中；主要风险是未来恢复移动端时需补做平台适配和跨平台回归。
- 时间影响：避免当前安装、构建和验证两套移动端工具链，缩短 Windows 可用版本交付路径。
- 不采用回滚：Story 1.1 的共享核心和 Story 1.2 的 Windows 工作均可直接复用。
- 不缩减最终产品目标：本次定义的是 Windows 前置里程碑，不是宣称跨平台 MVP 已完成。

## 4. 具体变更提案

### 4.1 `sprint-status.yaml`

新增顶层交付范围：

```yaml
delivery_scope: windows-first
milestone: windows-functional-complete
```

新增延期清单：

```yaml
deferred_mobile_stories:
  - 1.3
  - 1.4
  - 1.5
  - 8.4
  - 9.5
  - 9.6
  - 9.7
```

### 4.2 `epics.md`

增加以下交付覆盖规则：

> 当前迭代采用 Windows-first 交付范围：仅实施和验收共享核心、Windows 平台层及 Windows UI。Story 中 Apple、Android、手机和平板专属条款原样保留，在当前 Windows 里程碑中标记为 deferred/N/A，不阻塞 Windows 里程碑，但不得据此宣称跨平台 MVP 完成。恢复移动端实施时，重新激活延期 Story 和全部移动端验收条款。

### 4.3 Story 实施与质量报告

每个后续 Story 在创建或开发时记录 `delivery_scope: windows-first`，并把不适用的移动端条款列入 Deferred Acceptance。质量结论使用 `Windows milestone PASS/FAIL`，不得使用会暗示三端已完成的结论。

## 5. 实施交接

### 变更分类

Moderate：需要产品范围说明和 Sprint Backlog 标注，但不需要重构产品架构或回滚代码。

### 职责

- Product Owner：确认 Windows-first 范围和移动端延期边界。
- Developer：更新 `epics.md`、`sprint-status.yaml` 和后续 Story 范围记录；继续完成 Story 1.2，再按 Windows 路线实施。
- Test/Review：只对 Windows 里程碑作出质量结论，同时保留移动端未执行证据清单。

### 成功标准

- Windows 路线不再被移动端 Story 阻塞。
- 延期移动端需求、Story 和验收条款均可追踪且未被误标为完成。
- 每个 Windows Story 的适用自动化、代码审查、traceability 和 NFR 门禁通过。
- 所有依赖安装保持项目隔离，不改变全局 Python、Rust、Node 或系统包环境。
- 最终只声明 Windows 功能里程碑，不冒充跨平台 MVP 完成。

## 6. 批准记录

- 2026-08-14：用户选择增量确认模式。
- 2026-08-14：批准 Windows-first 范围定义。
- 2026-08-14：批准不重排 Story 的简化方案。
- 2026-08-14：批准最小文档改动范围。
- 2026-08-14：批准实施路线与成功标准。
- 2026-08-14：用户最终批准提案并授权实施规划调整。
