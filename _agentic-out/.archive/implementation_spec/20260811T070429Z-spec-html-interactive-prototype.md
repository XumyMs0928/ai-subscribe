---
title: 'HTML 可交互体验原型'
artifact_kind: implementation_spec
type: 'feature'
created: '2026-08-11'
status: 'draft'
delivery_profile: ''
source_story: ''
context:
  - '_agentic-out/planning/ux/DESIGN.md'
  - '_agentic-out/planning/ux/EXPERIENCE.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** 现有 HTML 只能切换视觉方案、平台外壳和静态页面，无法验证浏览、判断、规则校准与故障恢复等核心体验。

**Approach:** 新建独立、零依赖、可直接在浏览器运行的响应式 HTML 原型，使用模拟数据完成 UX spine 的关键交互闭环；保留现有视觉基准不变。原型只用于体验验证，不代表原生应用实现。

## Boundaries & Constraints

**Always:** 遵守 `DESIGN.md` 与 `EXPERIENCE.md`；支持桌面和约 390px 手机视口；保持搜索、筛选、选择和规则输入上下文；状态不只依赖颜色；主要路径支持键盘。

**Ask First:** 引入框架或外部依赖、访问网络、保存真实数据，或修改 PRD、UX、架构和现有视觉预览。

**Never:** 调用真实来源、AI、通知、托盘或原生 API；保存密钥；把原型宣称为原生 MVP、真实同步或运行时无障碍验收。

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Behavior | Error Handling |
|---|---|---|---|
| 浏览判断 | 选择情报 | 详情、证据、来源和操作同步更新 | 目标缺失时说明并安全返回 |
| 搜索筛选 | 查询和组合条件 | 即时过滤，可移除条件、恢复默认 | 无结果显示条件和清除入口 |
| 收藏反馈 | 收藏、反馈、待研究 | 状态独立、即时、可撤销，不跳离当前项 | 失败保留上下文并可重试 |
| 规则校准 | 修改规则或阈值 | 影响预览、风险提示、保存后说明下一轮生效 | 阻断错误禁止保存；失败保留输入 |
| 故障恢复 | 离线、限流、AI 等待、部分成功 | 缓存可用，显示范围、数据安全和恢复动作 | 重试恢复且不产生重复状态 |
| 手机路径 | 列表进入详情 | 单栈返回并保持筛选与选中项 | 无效深链进入安全回退 |

</frozen-after-approval>

## Code Map

- `_agentic-out/implementation/html-prototype/index.html` -- 语义结构、页面区域与覆盖层。
- `_agentic-out/implementation/html-prototype/styles.css` -- 设计令牌、响应式布局和组件状态。
- `_agentic-out/implementation/html-prototype/app.js` -- 模拟数据、状态模型与交互事件。
- `_agentic-out/implementation/html-prototype/prototype-smoke.mjs` -- 无依赖静态契约检查。
- `_agentic-out/implementation/html-prototype/README.md` -- 启动、验收路径与边界。

## Tasks & Acceptance

**Execution:**
- [ ] `index.html`, `styles.css` -- 实现情报流、详情、规则、状态区及桌面/手机布局，覆盖焦点、空、加载、错误和成功状态。
- [ ] `app.js` -- 实现筛选、选择、收藏/反馈、撤销、规则校验保存、故障恢复和手机返回。
- [ ] `prototype-smoke.mjs`, `README.md` -- 添加静态检查和人工验收说明。

**Acceptance Criteria:**
- Given 桌面或手机视口，when 完成“列表 → 详情 → 证据 → 收藏/反馈 → 返回”，then 上下文保持且操作结果可见。
- Given 修改搜索、筛选或规则，when 清除、校验或保存，then 结果、风险、生效边界与恢复动作清楚呈现。
- Given 离线、部分成功或限流，when 继续浏览并重试，then 缓存保持可用且恢复无重复副作用。
- Given 仅使用键盘，when 执行主要路径并关闭覆盖层，then 焦点可见、顺序合理并返回触发控件。
- Given 加载原型，when 完成自动与双视口人工检查，then 控制台无错误、无真实网络请求、无阻断性横向溢出。

## Spec Change Log

## Requirement Change Log

- **Trigger:** 用户要求暂缓架构并新增 HTML 可交互原型。 **Classification:** Story Amendment（独立 spec）。 **Previous behavior:** 视觉 HTML 无业务交互。 **New behavior:** 独立浏览器原型模拟核心体验。 **Acceptance Criteria affected:** 本规格全部 AC。 **Tasks affected:** 本规格全部任务。 **Upstream artifacts affected:** 无，仅引用。 **Tests required:** 语法、静态契约、桌面/手机人工检查。 **Approval evidence:** 用户于 2026-08-11 明确要求。 **Status:** proposed。

## Design Notes

采用独立目录和原生 HTML/CSS/JavaScript，避免污染视觉基准或过早引入框架。所有服务结果均为确定性模拟，不产生真实副作用。

## Verification

**Commands:**
- `node --check _agentic-out/implementation/html-prototype/app.js` -- JavaScript 语法通过。
- `node _agentic-out/implementation/html-prototype/prototype-smoke.mjs` -- 静态契约通过。

**Manual checks:**
- 桌面和约 390px 视口逐条执行矩阵路径，检查键盘、焦点、上下文、控制台和横向溢出。
