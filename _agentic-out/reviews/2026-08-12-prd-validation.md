---
validationTarget: '_agentic-out/planning/prd.md'
validationDate: '2026-08-12'
inputDocuments:
  - '_agentic-out/planning/research/technical-ai-intelligence-radar-research-2026-08-06.md'
  - '_agentic-out/planning/ux-design-specification.md'
  - '_agentic-out/reviews/2026-08-10-prd-validation-post-edit.md'
  - '_agentic-out/reviews/2026-08-11-prd-validation.md'
  - '_agentic-out/reviews/2026-08-12-readiness.md'
validationStepsCompleted:
  - 'step-v-01-discovery'
  - 'step-v-02-format-detection'
  - 'step-v-03-density-validation'
  - 'step-v-04-brief-coverage-validation'
  - 'step-v-05-measurability-validation'
  - 'step-v-06-traceability-validation'
  - 'step-v-07-implementation-leakage-validation'
  - 'step-v-08-domain-compliance-validation'
  - 'step-v-09-project-type-validation'
  - 'step-v-10-smart-validation'
  - 'step-v-11-holistic-quality-validation'
  - 'step-v-12-completeness-validation'
validationStatus: COMPLETE
holisticQualityRating: '4/5 - Good'
overallStatus: 'Warning'
---

# PRD Validation Report

**PRD Being Validated:** `_agentic-out/planning/prd.md`
**Validation Date:** 2026-08-12

## Input Documents

- PRD：`_agentic-out/planning/prd.md`
- 技术研究：`_agentic-out/planning/research/technical-ai-intelligence-radar-research-2026-08-06.md`
- UX 设计规格：`_agentic-out/planning/ux-design-specification.md`
- 历史验证：`_agentic-out/reviews/2026-08-10-prd-validation-post-edit.md`
- 历史验证：`_agentic-out/reviews/2026-08-11-prd-validation.md`
- 实现就绪评估：`_agentic-out/reviews/2026-08-12-readiness.md`

## Validation Findings

完整验证结论见以下各节。

## Format Detection

**PRD Structure:**

1. 执行摘要
2. 项目分类
3. 成功标准
4. 用户旅程
5. 领域专项要求
6. 创新与新颖模式
7. 跨平台原生应用专项要求
8. 产品范围与分阶段开发
9. 功能需求
10. 非功能需求

**workflow Core Sections Present:**

- Executive Summary: Present（执行摘要）
- Success Criteria: Present（成功标准）
- Product Scope: Present（产品范围与分阶段开发）
- User Journeys: Present（用户旅程）
- Functional Requirements: Present（功能需求）
- Non-Functional Requirements: Present（非功能需求）

**Format Classification:** workflow Standard
**Core Sections Present:** 6/6

PRD frontmatter 声明领域为 `AI/ML 技术研究与开发者工具`，项目类型为 `cross_platform_native_app`，复杂度为 `high`，项目上下文为 `greenfield`。

## Information Density Validation

**Anti-Pattern Violations:**

**Conversational Filler:** 0 occurrences

**Wordy Phrases:** 0 occurrences

**Redundant Phrases:** 0 occurrences

**Total Violations:** 0

**Severity Assessment:** Pass

**Recommendation:**
PRD demonstrates good information density with minimal violations. 中英文等价反模式扫描均未发现目标短语。

## Product Brief Coverage

**Status:** N/A - No Product Brief was provided as input

## Measurability Validation

### Functional Requirements

**Total FRs Analyzed:** 60

**Format Violations:** 0

**Subjective Adjectives Found:** 0

**Vague Quantifiers Found:** 0

**Implementation Leakage:** 0

RSS/Atom、GitHub Release、arXiv、兼容 OpenAI 接口规范、Windows 托盘及平台通知均用于定义产品接入或平台能力边界，不计为实现泄漏。

**FR Violations Total:** 0

FR51 原有“安全续作”措辞已改为可直接验收的结果；新增 FR59 与 FR60 分别明确同步结果最小消费闭环和用户处理状态合同。

### Non-Functional Requirements

**Total NFRs Analyzed:** 53

**Missing Metrics:** 0

**Incomplete Template:** 0

**Missing Context:** 0

53/53 条 NFR 均包含条件、测量方法、通过判据和保护目标；新增 NFR52、NFR53 具备固定操作/响应矩阵与 100% 或零次门禁。

**NFR Violations Total:** 0

### Overall Assessment

**Total Requirements:** 113
**Total Violations:** 0

**Severity:** Pass

**Recommendation:**
Requirements demonstrate good measurability with minimal issues.

## Traceability Validation

### Chain Validation

**Executive Summary → Success Criteria:** Intact

愿景中的每日低时长、高价值情报、完整溯源、本地优先、无 Key/离线降级、跨平台、可靠性与安全均有对应成功标准。

**Success Criteria → User Journeys:** Intact

日常效率与主情报流质量由旅程一、三承接；重大提醒和溯源由旅程二承接；离线、单源与 AI 故障恢复由旅程一、四承接；跨平台与四周验证由旅程汇总及对应功能需求支撑。

**User Journeys → Functional Requirements:** Gaps Identified

- 行 137、143：旅程一承诺“非阻塞、渐进式配置引导”，但 FR1–FR5 只定义首启与配置能力，未明确引导必须可跳过、可恢复且不阻塞主情报流。
- 行 163：旅程三承诺保存前校验并提示“过窄条件可能漏报”，但现有 FR 未明确规定风险预览、阻断错误或警告后的保存行为。

四条用户旅程均有大量 FR 支撑，不存在完全无 FR 的旅程。

**Scope → FR Alignment:** Misaligned

- 行 312、392 将 Windows 单实例列为平台要求和 MVP 必备能力；FR41–FR42 未包含单实例能力。NFR35 虽提供验收门禁，但范围缺少对应功能合同。
- 正式“旅程与验收追踪矩阵”未显式列入 FR7–FR9、FR13。它们分别由旅程中的三类来源同步、成功转换语义和 Phase 1 来源范围支撑，因此不是语义孤儿，但矩阵存在维护性遗漏。

### Orphan Elements

**Orphan Functional Requirements:** 0

FR59 追溯至旅程一、旅程四及 Phase 1 同步最小消费闭环；FR60 追溯至旅程一、旅程三及日常处理状态目标，均非孤儿需求。

**Unsupported Success Criteria:** 0

**User Journeys Without FRs:** 0

### Traceability Matrix

| FR 范围 | 主要来源 |
|---|---|
| FR1–FR6 | 旅程一、旅程三；首次体验与个性化范围 |
| FR7–FR13 | 旅程一、旅程四及 Phase 1 三类来源与同步范围；FR7–FR9、FR13 缺少正式矩阵显式列入 |
| FR14–FR18 | 旅程二；溯源与内容生命周期范围 |
| FR19–FR24 | 旅程二、旅程三；筛选解释与确定性关联范围 |
| FR25–FR30 | 旅程一、旅程二、旅程四；AI 授权、分析与降级范围 |
| FR31–FR36 | 旅程一至三；浏览、检索、详情与收藏范围 |
| FR37–FR42 | 旅程一、旅程二；通知与 Windows 常驻范围 |
| FR43–FR50 | 旅程一至四；反馈验证、故障隔离与诊断范围 |
| FR51–FR58 | 四条旅程及跨平台、本地数据与生命周期范围 |
| FR59 | 旅程一、旅程四；同步结果最小消费闭环 |
| FR60 | 旅程一、旅程三；用户处理状态闭环 |

**Total Traceability Issues:** 4

**Severity:** Warning

**Recommendation:**
补齐正式矩阵中的 FR7–FR9、FR13，并将渐进配置、过窄规则风险提示和 Windows 单实例提升为明确 FR，或从旅程/范围中移除对应承诺。所有现有 FR 均可追溯到用户需要或业务目标，不存在孤立功能需求。

## Implementation Leakage Validation

### Leakage by Category

**Frontend Frameworks:** 0 violations

**Backend Frameworks:** 0 violations

**Databases:** 0 violations

**Cloud Platforms:** 0 violations

**Infrastructure:** 0 violations

**Libraries:** 0 violations

**Other Implementation Details:** 0 violations

RSS/Atom、GitHub Release、arXiv 与兼容 OpenAI 接口规范用于定义产品接入和互操作能力；Windows、iOS/iPadOS、Android、HTTP/HTTPS、操作系统安全凭据存储及 48×48 dp 用于定义平台、安全或验收边界，均不属于内部实现方案泄漏。

### Summary

**Total Implementation Leakage Violations:** 0

**Severity:** Pass

**Recommendation:**
No significant implementation leakage found. Requirements properly specify WHAT without prescribing HOW.

## Domain Compliance Validation

**Domain:** AI/ML 技术研究与开发者工具（匹配 `scientific`）
**Complexity:** Medium（非受监管行业）

### Scientific-Domain Considerations

| Requirement | Status | Notes |
|---|---|---|
| Validation methodology | Met | 连续四周个人验证、固定故障矩阵、候选构建和人工反馈复盘均有明确方法。 |
| Accuracy metrics | Met | 通知有效率 ≥80%、主情报流有效率 ≥70%、已确认关键漏报为 0、重大提醒溯源完整率 100%。 |
| Reproducibility plan | Met | 统一验收基线固定平台、设备、50,000 条数据集、样本数、服务响应及资源采样方法。 |
| Computational requirements | Met | Windows 与移动基线设备、P95、CPU、内存、生命周期和后台能耗指标均已定义。 |

### Summary

**Required Considerations Present:** 4/4
**Compliance Gaps:** 0

**Severity:** Pass

**Recommendation:**
该产品不属于医疗、金融、政府、法律等受监管高复杂度领域；科学类产品所需的验证方法、准确性、可复现性和计算资源要求均已充分记录，无需额外监管合规章节。

## Project-Type Compliance Validation

**Project Type:** `cross_platform_native_app`（按 `mobile_app` + `desktop_app` 复合类型核验）

`project-types.csv` 未提供该复合枚举，因此合并移动端与桌面端必需项；两类模板互相列出的 `desktop_features` 与 `mobile_features` 排除项不适用于明确要求 Windows、iOS/iPadOS 和 Android 的产品。

### Required Sections

| Required capability | Status | Evidence |
|---|---|---|
| Mobile platform requirements | Present | 定义 iOS/iPadOS 17+、Android 10+、手机/平板形态、窗口变化、输入与辅助技术。 |
| Device permissions | Present | 定义通知权限按需申请、拒绝/撤销降级、后台限制、隐私披露及可测验收。 |
| Offline mode | Present | 定义设备本地数据、离线可用操作、暂停的外部能力及联网恢复。 |
| Push/notification strategy | Present | 明确 MVP 无远程推送，仅设备本地判定后提交系统通知，并覆盖时效、深链、免打扰与权限边界。 |
| Store/distribution compliance | Present | 明确受控测试分发、有效移动测试签名/安装资格、七类隐私披露及公开商店后置。 |
| Desktop platform support | Present | 定义 Windows 10/11 x64、当前用户安装及不支持的平台。 |
| Desktop system integration | Present | 覆盖窗口、托盘、通知、开机启动、单实例和显式退出；单实例已有 NFR 门禁但缺 FR 合同的问题计入追踪 Warning。 |
| Update strategy | Present | 明确 MVP 手动安装、自动更新后置、升级迁移保护与失败停止。 |
| Desktop offline capabilities | Present | 与跨平台离线、缓存访问、任务恢复和前台同步要求一致。 |

### Excluded Sections (Should Not Be Present)

- CLI 命令结构、shell 交互、SEO、SPA/PWA 等不相关能力：Absent ✓
- Web、macOS、Linux 等只作为明确不支持范围出现，不构成功能残留。

### Compliance Summary

**Required Sections:** 9/9 present
**Excluded Sections Present:** 0
**Compliance Score:** 100%

**Severity:** Pass

**Recommendation:**
All required sections for the composite cross-platform native application are present. No applicable excluded sections were found.

## SMART Requirements Validation

**Total Functional Requirements:** 60

### Scoring Summary

**All scores ≥ 3:** 100% (60/60)
**All scores ≥ 4:** 88.3% (53/60)
**Overall Average Score:** 4.77/5.0

### Scoring Table

| FR # | Specific | Measurable | Attainable | Relevant | Traceable | Average | Flag |
|---|---:|---:|---:|---:|---:|---:|:---:|
| FR1 | 5 | 5 | 5 | 5 | 5 | 5.0 | |
| FR2 | 5 | 5 | 5 | 5 | 5 | 5.0 | |
| FR3 | 4 | 4 | 5 | 5 | 5 | 4.6 | |
| FR4 | 4 | 4 | 5 | 5 | 5 | 4.6 | |
| FR5 | 5 | 4 | 5 | 5 | 5 | 4.8 | |
| FR6 | 5 | 4 | 5 | 5 | 5 | 4.8 | |
| FR7 | 4 | 4 | 5 | 5 | 3 | 4.2 | |
| FR8 | 4 | 4 | 5 | 5 | 3 | 4.2 | |
| FR9 | 4 | 4 | 5 | 5 | 3 | 4.2 | |
| FR10 | 5 | 4 | 4 | 5 | 5 | 4.6 | |
| FR11 | 5 | 5 | 5 | 5 | 5 | 5.0 | |
| FR12 | 5 | 5 | 4 | 5 | 5 | 4.8 | |
| FR13 | 4 | 3 | 5 | 5 | 3 | 4.0 | |
| FR14 | 5 | 5 | 5 | 5 | 5 | 5.0 | |
| FR15 | 5 | 5 | 5 | 5 | 5 | 5.0 | |
| FR16 | 5 | 4 | 5 | 5 | 5 | 4.8 | |
| FR17 | 5 | 4 | 4 | 5 | 5 | 4.6 | |
| FR18 | 5 | 5 | 4 | 5 | 5 | 4.8 | |
| FR19 | 4 | 3 | 4 | 5 | 5 | 4.2 | |
| FR20 | 5 | 4 | 4 | 5 | 5 | 4.6 | |
| FR21 | 5 | 5 | 5 | 5 | 5 | 5.0 | |
| FR22 | 5 | 5 | 5 | 5 | 5 | 5.0 | |
| FR23 | 5 | 4 | 4 | 5 | 5 | 4.6 | |
| FR24 | 5 | 5 | 4 | 5 | 5 | 4.8 | |
| FR25 | 5 | 5 | 5 | 5 | 5 | 5.0 | |
| FR26 | 5 | 5 | 5 | 5 | 5 | 5.0 | |
| FR27 | 5 | 4 | 4 | 5 | 5 | 4.6 | |
| FR28 | 5 | 5 | 5 | 5 | 5 | 5.0 | |
| FR29 | 5 | 5 | 5 | 5 | 5 | 5.0 | |
| FR30 | 5 | 5 | 5 | 5 | 5 | 5.0 | |
| FR31 | 4 | 4 | 5 | 5 | 5 | 4.6 | |
| FR32 | 5 | 5 | 5 | 5 | 5 | 5.0 | |
| FR33 | 5 | 5 | 4 | 5 | 5 | 4.8 | |
| FR34 | 5 | 4 | 5 | 5 | 5 | 4.8 | |
| FR35 | 5 | 5 | 5 | 5 | 5 | 5.0 | |
| FR36 | 5 | 5 | 4 | 5 | 5 | 4.8 | |
| FR37 | 5 | 5 | 4 | 5 | 5 | 4.8 | |
| FR38 | 5 | 5 | 5 | 5 | 5 | 5.0 | |
| FR39 | 5 | 5 | 5 | 5 | 5 | 5.0 | |
| FR40 | 5 | 5 | 5 | 5 | 5 | 5.0 | |
| FR41 | 5 | 5 | 4 | 5 | 5 | 4.8 | |
| FR42 | 5 | 5 | 4 | 5 | 5 | 4.8 | |
| FR43 | 5 | 5 | 5 | 5 | 5 | 5.0 | |
| FR44 | 4 | 4 | 5 | 5 | 5 | 4.6 | |
| FR45 | 5 | 4 | 4 | 5 | 5 | 4.6 | |
| FR46 | 5 | 4 | 4 | 5 | 5 | 4.6 | |
| FR47 | 5 | 5 | 5 | 5 | 5 | 5.0 | |
| FR48 | 5 | 5 | 4 | 5 | 5 | 4.8 | |
| FR49 | 5 | 4 | 4 | 5 | 5 | 4.6 | |
| FR50 | 5 | 5 | 4 | 5 | 5 | 4.8 | |
| FR51 | 5 | 5 | 4 | 5 | 5 | 4.8 | |
| FR52 | 5 | 5 | 5 | 5 | 5 | 5.0 | |
| FR53 | 5 | 5 | 5 | 5 | 5 | 5.0 | |
| FR54 | 5 | 5 | 4 | 5 | 5 | 4.8 | |
| FR55 | 5 | 5 | 3 | 5 | 5 | 4.6 | |
| FR56 | 5 | 4 | 3 | 5 | 5 | 4.4 | |
| FR57 | 5 | 5 | 5 | 5 | 5 | 5.0 | |
| FR58 | 5 | 5 | 5 | 5 | 5 | 5.0 | |
| FR59 | 5 | 5 | 4 | 5 | 5 | 4.8 | |
| FR60 | 5 | 5 | 4 | 5 | 5 | 4.8 | |

**Legend:** 1=Poor, 3=Acceptable, 5=Excellent  
**Flag:** X = Score < 3 in one or more categories

### Improvement Suggestions

**Low-Scoring FRs:** None

FR7–FR9、FR13 的 Traceable 分数受正式追踪矩阵遗漏影响，但它们仍有旅程语义和 Phase 1 范围来源，并非孤儿需求。FR55–FR56 的跨平台范围可行但交付体量较大。FR59、FR60 的追踪关系完整。

### Overall Assessment

**Severity:** Pass

**Recommendation:**
Functional Requirements demonstrate good SMART quality overall.

## Holistic Quality Assessment

### Document Flow & Coherence

**Assessment:** Good

**Strengths:**

- 文档形成“问题与价值 → 成功标准 → 用户旅程 → 领域与平台边界 → 分阶段范围 → FR/NFR”的清晰主线。
- 本地优先、无 Key 可用、完整溯源、设备本地通知、跨平台生命周期和受控分发等核心原则前后一致。
- 新增同步结果最小消费闭环和用户处理状态合同，已消除实现就绪评估中的两个主要产品缺口。

**Areas for Improvement:**

- 文档较长，核心决策分散；缺少便于高管和交付负责人快速定位的一页式 MVP 决策摘要。
- 非阻塞渐进配置、过窄规则风险提示与 Windows 单实例仍未形成明确 FR。
- 正式追踪矩阵遗漏 FR7–FR9、FR13，影响自动追踪和变更审计。

### Dual Audience Effectiveness

**For Humans:**

- Executive-friendly: Strong；愿景、用户、指标和范围清晰，但关键决策可进一步集中。
- Developer clarity: Strong；需求可测且无实现泄漏，少数功能合同需补齐。
- Designer clarity: Strong；旅程、状态、平台差异和交互边界充分，渐进配置仍需需求级约束。
- Stakeholder decision-making: Strong；签名、数据、通知、来源和发布边界可用于决策。

**For LLMs:**

- Machine-readable structure: Excellent；标准标题、连续编号和统一 NFR 模板。
- UX readiness: Excellent；旅程、状态与跨平台约束可直接生成设计。
- Architecture readiness: Strong；核心边界完整，待同步 Windows 签名和领域组件映射。
- Epic/Story readiness: Strong；FR/NFR 可直接拆解，但追踪矩阵和三个缺失功能合同应先修订。

**Dual Audience Score:** 4.5/5

### workflow PRD Principles Compliance

| Principle | Status | Notes |
|---|---|---|
| Information Density | Met | 目标反模式扫描为 0；虽然篇幅较长，但内容大多承担约束或验收作用。 |
| Measurability | Met | 60 条 FR 与 53 条 NFR 均可测试，0 项可测量性违规。 |
| Traceability | Partial | 无孤儿 FR，但正式矩阵遗漏 4 条 FR，且 3 项旅程/范围承诺缺少 FR 合同。 |
| Domain Awareness | Met | 科学验证、准确性、可复现性、计算资源、内容合规和 AI 边界完整。 |
| Zero Anti-Patterns | Met | 未发现填充、冗长、冗余或实现泄漏。 |
| Dual Audience | Met | 同时支持人类决策、UX/架构生成和 Epic/Story 拆解。 |
| Markdown Format | Met | 6/6 核心章节、frontmatter、需求编号和表格结构规范。 |

**Principles Met:** 6/7 fully met; 1 partial

### Overall Quality Rating

**Rating:** 4/5 - Good

**Scale:**

- 5/5 - Excellent: Exemplary, ready for production use
- 4/5 - Good: Strong with minor improvements needed
- 3/5 - Adequate: Acceptable but needs refinement
- 2/5 - Needs Work: Significant gaps or issues
- 1/5 - Problematic: Major flaws, needs substantial revision

### Top 3 Improvements

1. **补齐三个功能合同**
   为非阻塞渐进配置、过窄规则风险提示和 Windows 单实例新增或扩展明确 FR，使旅程、范围与验收完全闭合。

2. **修复正式追踪矩阵**
   将 FR7–FR9、FR13 显式加入对应旅程，避免自动追踪工具误判并提高变更审计可靠性。

3. **增加一页式 MVP 决策摘要**
   集中展示平台范围、三类来源门禁、Windows 生产签名边界、核心指标、发布方式与非目标，降低长文档的决策定位成本。

### Summary

**This PRD is:** 一份结构强健、可测、跨平台边界清晰的 PRD，已具备下游规划基础，但在成为完全闭环的权威需求合同前仍需一次小范围追踪与功能补丁。

**To make it great:** Focus on the top 3 improvements above.

## Completeness Validation

### Template Completeness

**Template Variables Found:** 0

No template variables remaining ✓

### Content Completeness by Section

**Executive Summary:** Complete

**Success Criteria:** Complete

**Product Scope:** Complete

**User Journeys:** Complete

**Functional Requirements:** Complete（FR1–FR60 连续且唯一）

**Non-Functional Requirements:** Complete（NFR1–NFR53 连续且唯一）

### Section-Specific Completeness

**Success Criteria Measurability:** All measurable

**User Journeys Coverage:** Yes - covers all declared user types and normal, calibration, notification, offline, and failure flows

**FRs Cover MVP Scope:** Partial

MVP 核心能力已有 FR 覆盖，但非阻塞渐进配置、过窄规则漏报警告和 Windows 单实例三项已承诺行为缺少明确 FR 合同。

**NFRs Have Specific Criteria:** All

### Frontmatter Completeness

**stepsCompleted:** Present
**classification:** Present
**inputDocuments:** Present（5/5 路径存在）
**date:** Present

**Frontmatter Completeness:** 4/4

### Completeness Summary

**Overall Completeness:** 96%

**Critical Gaps:** 0

**Minor Gaps:** 4

1. 非阻塞、可跳过且可恢复的渐进配置引导缺少明确 FR。
2. 保存前识别过窄规则并提示漏报风险的行为缺少明确 FR。
3. Windows 单实例虽有范围声明和 NFR35 门禁，但缺少明确 FR。
4. 正式旅程追踪矩阵遗漏已有的 FR7–FR9、FR13。

**Severity:** Warning

**Recommendation:**
PRD is structurally complete and implementation-usable. No critical completeness defects were found. Address the four minor gaps for a fully closed requirements contract.

## Final Validation Summary

| Check | Result |
|---|---|
| Format | workflow Standard（6/6 core sections） |
| Information Density | Pass（0 violations） |
| Product Brief Coverage | N/A（no Product Brief） |
| Measurability | Pass（0/113 violations） |
| Traceability | Warning（4 issues; 0 orphan FRs） |
| Implementation Leakage | Pass（0 violations） |
| Domain Compliance | Pass（4/4 scientific considerations） |
| Project-Type Compliance | Pass（9/9; 100%） |
| SMART Quality | Pass（60/60 acceptable; average 4.77/5） |
| Holistic Quality | 4/5 - Good |
| Completeness | Warning（96%） |

**Overall Status:** Warning

**Critical Issues:** 0

**Warnings:**

1. 非阻塞、可跳过且可恢复的渐进配置引导缺少明确 FR。
2. 保存前识别过窄规则并提示漏报风险的行为缺少明确 FR。
3. Windows 单实例已有范围声明与 NFR35 门禁，但缺少明确 FR。
4. 正式旅程追踪矩阵遗漏 FR7–FR9、FR13；这些需求具有语义来源，并非孤儿 FR。

**Strengths:**

- 60 条 FR 与 53 条 NFR 连续、唯一且全部可测，未发现实现泄漏。
- 三端原生应用的 9 项项目类型要求完整，通知、离线、分发、权限、数据与升级边界清晰。
- FR59/FR60 与 NFR52/NFR53 已关闭同步结果最小消费和用户处理状态两个主要缺口。
- 0 个孤儿 FR、0 个不受支持成功标准、0 条无 FR 支撑的完整旅程。
- 文档同时适合人类决策和下游 UX、架构、Epic/Story 生成。

**Recommendation:** PRD 已可用于下游同步修订，但应先用一次小范围编辑补齐三个功能合同和追踪矩阵，再同步 Architecture 与 Epics，并重跑实现就绪评估。
