# Sprint Change Proposal：移除屏幕阅读器支持并调整真实显示矩阵门禁

日期：2026-08-15  
项目：ai-subscribe  
模式：Batch  
触发 Story：1.7「浏览可访问的演示情报列表与证据详情」  
状态：已批准并实施

## 1. Issue Summary

Story 1.7 的代码、契约、Windows UI、键盘/UIA、自动化布局与候选构建证据已基本完成，剩余门禁要求在用户日常 Windows 会话中运行 NVDA，并切换 100%、125%、150%、175%、200% 系统缩放与浅/深主题。用户明确决定产品不考虑盲人用户，因此不再承诺 NVDA、VoiceOver 或 TalkBack 支持；同时不要求为开发验收修改用户日常电脑的全局主题和缩放。

这是一项主动的产品范围缩减，不是技术实现失败。现有屏幕阅读器语义代码无需回滚；它仍可服务键盘行为、Windows UI Automation、自动化定位和未来低成本恢复，但不再构成受支持能力或发布声明。

## 2. Change Navigation Checklist

### 2.1 Trigger and Context

- [x] 1.1：触发 Story 为 1.7，问题暴露于真实 NVDA 与 10 组系统矩阵验收门。
- [x] 1.2：类型为 stakeholder strategic scope decision；明确排除盲人屏幕阅读器支持。
- [x] 1.3：证据包括用户明确指令“那就不考虑盲人用户”、当前沙盒不是隔离 Windows VM，以及 Story 1.7 已有 UIA 3/3、Playwright 10/10 场景断言和 30/30 候选性能证据。

### 2.2 Epic Impact

- [x] 2.1：Epic 1 仍可完成；Story 1.7 仅需调整 AC4/AC5 和完成门禁。
- [!] 2.2：Epic 1、4、7、8、9 中明确要求屏幕阅读器、NVDA、VoiceOver 或 TalkBack 的验收文字需批量改写。
- [x] 2.3：未来移动 Story 保留 Dynamic Type、显示缩放、触控目标、键盘/焦点、非颜色表达，但不承诺 VoiceOver/TalkBack。
- [N/A] 2.4：无需新增或废弃 Epic。
- [N/A] 2.5：无需调整 Epic 顺序或 Windows-first 优先级。

### 2.3 Artifact Impact

- [!] 3.1 PRD：NFR43/NFR44 删除平台屏幕阅读器条件与播报判据；NFR30 键盘要求保留；NFR31 保留为最终显示兼容目标，但不再由 Story 1.7 在用户日常系统执行。
- [!] 3.2 Architecture：删除屏幕阅读器发布门禁和测试承诺；保留平台语义、稳定名称、角色/状态、焦点和自动化树作为工程质量要求。
- [!] 3.3 UX：删除 NVDA/VoiceOver/TalkBack 支持声明与真实运行门；保留键盘、对比度、非颜色状态、Dynamic Type/显示缩放、触控目标、Reduce Motion。
- [!] 3.4 Secondary artifacts：Story 1.7、traceability、NFR、review 和 artifacts manifest 需重生成或重新对账。现有代码和测试不删除。

### 2.4 Path Forward

- [x] 4.1 Direct Adjustment：可行；工作量 Low，风险 Low。
- [x] 4.2 Rollback：不可取；删除现有语义和键盘实现没有收益，反而降低自动化稳定性。
- [x] 4.3 MVP Review：可行；将屏幕阅读器支持从 MVP/当前产品声明中移除，属于明确范围缩减。
- [x] 4.4 推荐 Hybrid：MVP 范围缩减 + 对现有文档直接调整。保留实现，不声明支持，不保留运行门禁。

## 3. Impact Analysis

### Epic Impact

- Epic 1：Story 1.7 可在自动化、键盘/UIA和候选证据完成后关闭；NVDA 不再阻塞。
- Epic 4/7/8/9：删除具体屏幕阅读器旅程；可见焦点、键盘、触控、非颜色表达和布局连续性不变。
- 移动端延期策略不变；未来实现 Apple/Android 时不要求 VoiceOver/TalkBack 验收，但仍需平台原生布局、字体/显示缩放和触控目标。

### Technical Impact

- 不改 Rust、SQLite、Tauri、DesktopApi、React DTO 或数据合同。
- 不删除现有 ARIA、accessible name、role、heading、focus restore 或 UIA automation ID。
- 不新增依赖，不安装 NVDA，不修改全局 Python/Node/Rust、主题、缩放或辅助功能。
- Playwright 汇总进程退出挂起仍是独立测试基础设施问题，不因范围变化而伪装为已解决。

### Product and Risk Impact

- 产品将不承诺盲人可独立完成核心旅程，也不以 NVDA/VoiceOver/TalkBack 作为发布门禁。
- 仍支持普通键盘用户、色觉差异用户、放大显示用户和 Reduce Motion 用户的基础体验。
- NFR31 的真实系统缩放×主题验证移交给未来专用 Windows 发布测试机；开发阶段允许用 Playwright 等效视口、真实候选 UIA 和人工默认环境 smoke 作为证据。

## 4. Detailed Change Proposals

### 4.1 PRD

#### NFR43

OLD：iPhone/iPad 使用平台屏幕阅读器、最大动态字体、提高对比度和减弱动画完成核心任务，并验证状态播报。

NEW：iPhone/iPad 使用最大受支持动态字体、提高对比度和减弱动画完成核心任务；检查文字裁切、操作结果和非颜色状态表达。删除平台屏幕阅读器与状态播报通过判据。

#### NFR44

OLD：Android 手机/平板使用平台屏幕阅读器、系统字体与显示缩放及减弱动画，并验证名称、遍历顺序和焦点。

NEW：Android 手机/平板使用系统字体与显示缩放及减弱动画完成核心任务；验证文字裁切、操作结果、非颜色状态与 ≥48×48dp 核心触控目标。删除 TalkBack/屏幕阅读器通过判据。

#### NFR31 evidence ownership

OLD：相关 Story 可要求在当前开发电脑完成 Windows 5 档缩放 × 2 主题真实系统矩阵。

NEW：NFR31 仍为最终 Windows 显示兼容目标，但真实系统矩阵只在专用发布测试机或隔离 VM 执行；普通 Story 开发门使用等效视口自动化、候选 UIA和默认环境 smoke，不修改开发者日常系统设置。

### 4.2 Epics and Stories

#### UX-DR20

OLD：Apple 支持 VoiceOver；Android 支持 TalkBack。

NEW：Windows 保持全键盘与可见焦点；Apple 保持 Dynamic Type、平台焦点和可见等价操作；Android 保持字体/显示缩放、平台焦点及 ≥48×48dp 目标。删除 VoiceOver/TalkBack 承诺。

#### Story 1.7 AC4

OLD：用户仅使用键盘或 Windows 屏幕阅读器；真实 NVDA 是完成门禁。

NEW：用户仅使用键盘；保留稳定名称、角色、状态、可见焦点、正确顺序、快捷键和无焦点陷阱。UIA 语义作为自动化与工程合同，不声明盲人独立使用支持。删除 NVDA 门禁。

#### Story 1.7 AC5 / Task 6

OLD：Story 1.7 必须在用户系统完成 10 个真实缩放×主题组合才可 done。

NEW：Story 1.7 以等效视口自动化、主题场景、forced-colors/Reduce Motion 静态与运行检查、MSVC UIA/键盘 smoke 作为当前验收；真实 10 组系统矩阵转移到专用发布环境，不要求修改用户日常系统设置。

#### Other stories

将“键盘或屏幕阅读器”“VoiceOver 或 TalkBack”“对应屏幕阅读器”改为适用平台输入方式、稳定可见标签、焦点、非颜色反馈、动态字体/显示缩放和触控目标。删除播报结果与屏幕阅读器完成率指标；不删除可访问名称和角色，因为其仍支撑 UIA、自动化和一致交互。

### 4.3 Architecture

OLD：屏幕阅读器文本和屏幕阅读器语义分别通过平台门禁验证；候选构建包含屏幕阅读器发布门。

NEW：稳定名称、角色、状态、焦点顺序和平台自动化树作为工程质量合同验证；候选构建不再要求 NVDA/VoiceOver/TalkBack。动态字体、显示缩放、键盘、触控目标、非颜色状态和 Reduce Motion 仍是平台门禁。

### 4.4 UX Documents

- `ux-design-specification.md`：删除 NVDA/VoiceOver/TalkBack 支持与“自动检查不能替代屏幕阅读器”文字；改为自动化/UIA、键盘、字体/显示缩放、对比度、Reduce Motion 和真实设备布局验证。
- `ux/EXPERIENCE.md`：Windows 删除 NVDA，Apple 删除 VoiceOver，Android 删除 TalkBack；核心验收旅程改为键盘/触控/平台输入方式。
- `ux/DESIGN.md`：将“辅助技术惯例/无障碍合同”收敛为键盘、焦点、非颜色状态、字体/显示缩放、触控目标和自动化语义合同。

## 5. Recommended Approach

采用 Hybrid：产品范围缩减 + 文档直接调整。

- Effort：Low–Medium，主要是权威文档和追踪制品批量同步。
- Technical risk：Low，不删除已工作的代码。
- Product risk：Medium，产品明确不覆盖盲人用户；未来若恢复支持，需要重新建立真实屏幕阅读器验收。
- Timeline impact：Story 1.7 不再等待 NVDA/本机真实矩阵，可在剩余自动化基础设施问题处理并重新生成质量制品后进入 review/done。

不推荐回滚现有语义实现，也不建议让用户在日常系统上反复切换全局显示设置。

## 6. Implementation Handoff

Change scope：Moderate（跨 PRD、Epic、Architecture、UX、Story 和测试追踪，但无代码架构变更）。

批准后：

1. Product/Developer：应用 PRD、Epics、Architecture、UX 文本变更。
2. Developer：更新 Story 1.7 AC、Tasks、Review Finding、Debug Log 和状态判定；保留现有语义代码。
3. Test workflow：重新生成 Story 1.7 automation/review/traceability/NFR 结论，明确 screen-reader N/A、真实系统矩阵 deferred-to-release-environment。
4. Agentic Flow：reconcile artifacts manifest，保持 Windows-first 和移动端延期策略。

成功标准：

- 权威文档中不再声明 NVDA、VoiceOver、TalkBack 或盲人核心旅程支持。
- Story 1.7 不再要求修改用户日常 Windows 主题/缩放或安装 NVDA。
- 键盘、焦点、非颜色表达、对比度、字体/显示缩放适配、触控目标和 Reduce Motion 要求仍明确存在。
- 不删除现有 ARIA/UIA/自动化语义代码。
- Playwright 退出问题保持独立、诚实跟踪，未解决前不冒充干净测试退出。

## 7. Approval

用户于 2026-08-15 明确回复“批准”。变更按本提案实施，交由 Developer 更新权威文档与 Story，Test workflow 重新生成 Story 1.7 质量制品，Agentic Flow 负责最终对账。

## 8. Implementation Result

- [x] PRD：NFR31 evidence ownership、NFR43、NFR44 已更新。
- [x] Epics：屏幕阅读器旅程与平台承诺已改为键盘/触控/焦点/字体或显示缩放合同。
- [x] Architecture：屏幕阅读器门禁已改为稳定自动化语义和平台输入质量合同。
- [x] UX specification、DESIGN、EXPERIENCE：盲人屏幕阅读器支持声明与运行门已移除；其他可用性要求保留。
- [x] Story 1.7：屏幕阅读器和开发者本机真实系统矩阵不再阻塞；Playwright 汇总退出挂起已由项目内受控 runner 修复，Story 已进入 `review`。
- [x] Sprint status：记录当前范围和专用发布环境的 NFR31 责任边界。
- [x] Agentic Flow：已 reconcile；readiness、automation、traceability、NFR 和 code review 因上游范围变化被诚实标记为 stale。

Handoff：Developer 已解决 Playwright 退出问题；Testing workflow 按新范围重生成质量制品。无需产品经理或架构师进一步重排 backlog。
