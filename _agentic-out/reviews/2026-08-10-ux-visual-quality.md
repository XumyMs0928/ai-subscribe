---
project: ai-subscribe
reviewDate: 2026-08-10
reviewType: cross-platform-ux-visual-quality
selectedDirection: 冷静决策台
platforms:
  - Windows 10/11
  - iOS/iPadOS
  - Android phone/tablet/foldable
status: Pass-with-implementation-gates
---

# ai-subscribe UX 视觉质量评审

## 评审结论

**结论：通过设计阶段视觉质量门禁，进入实现前必须同步修订并重新验证 PRD。**

“冷静决策台”已从 Windows 基线扩展为三端原生策略：Windows 使用 shadcn/ui，iOS/iPadOS 使用 SwiftUI，Android 使用 Jetpack Compose Material 3 Adaptive。三端共享情报、证据、置信度、溯源、规则影响和故障恢复语义，但使用平台原生导航、控件和辅助技术行为。

设计预览验证已完成；真实应用仍需在 SwiftUI、Compose 与 Windows 运行时完成设备、缩放、屏幕阅读器、权限和生命周期测试。本报告不把浏览器原型通过等同于生产实现通过。

## 评审产物

- 设计方向：`_agentic-out/planning/ux/design-directions.html`
- 跨平台适配：`_agentic-out/planning/ux/cross-platform-adaptations.html`
- Windows 截图：`_agentic-out/planning/ux/adaptation-windows.png`
- iPhone 截图：`_agentic-out/planning/ux/adaptation-iphone.png`
- iPad 截图：`_agentic-out/planning/ux/adaptation-ipad.png`
- Android 手机截图：`_agentic-out/planning/ux/adaptation-android-phone.png`
- Android 平板截图：`_agentic-out/planning/ux/adaptation-android-tablet.png`

## 选定设计方向

- [x] 使用“冷静决策台”作为跨平台产品语义与信息层级基线。
- [x] Windows 保持高密度三栏工作区。
- [x] iPhone 与 Compact Android 使用底部主导航和单栈详情。
- [x] iPad 与 Android 平板/折叠屏按可用窗口空间使用多窗格布局。
- [x] AI 结论、置信度、判断依据和原始事实保持分层。
- [x] 离线、部分成功、AI 等待和恢复状态在所有平台含义一致。

## 平台组件覆盖

### Windows / shadcn/ui

- [x] Sidebar、Resizable、Scroll Area 支持桌面工作区。
- [x] Input、Command、Popover、Sheet 支持搜索与筛选。
- [x] Tabs、Collapsible、Accordion 支持证据和溯源。
- [x] Form、Alert Dialog 支持规则配置和风险确认。
- [x] Alert、Progress、Skeleton、Sonner 支持状态反馈。

### iOS/iPadOS / SwiftUI

- [x] TabView、NavigationStack、NavigationSplitView 覆盖手机与平板导航。
- [x] List、Section、DisclosureGroup、LabeledContent 覆盖情报、证据和溯源。
- [x] Form、Picker、Toggle、Slider、Sheet 覆盖配置。
- [x] ProgressView、ContentUnavailableView、Alert、ConfirmationDialog 覆盖状态。
- [x] searchable、refreshable、ShareLink 与 openURL 覆盖平台能力。

### Android / Jetpack Compose

- [x] Material 3 Navigation Bar、Rail、Drawer 覆盖尺寸类别变化。
- [x] LazyColumn、ListItem 与 Adaptive Pane 覆盖列表—详情。
- [x] TextField、Switch、Slider、Bottom Sheet 覆盖配置和筛选。
- [x] Snackbar、Progress、Dialog 与空状态组合覆盖反馈。
- [x] Deep Link、System Back、Window Insets 与 Window Size Classes 纳入规格。

## 共享领域组件覆盖

- [x] `IntelligenceFeedItem`
- [x] `EvidenceDetailPanel`
- [x] `SourceProvenanceGroup`
- [x] `SyncHealthSummary`
- [x] `RuleImpactPreview`
- [x] `ProgressiveSetupGuide`

以上名称代表共享语义契约，不代表三端复用同一视图代码。

## 语义令牌一致性

- [x] 主色使用“信号青”，并按平台映射到原生主题。
- [x] 背景、表面、边界、前景和弱化文字使用语义令牌。
- [x] 成功、警告、错误、离线、部分成功与 AI 状态语义分离。
- [x] 不使用任意页面级颜色传达关键业务状态。
- [x] 浅色与深色主题具有明确映射策略。
- [ ] 实现阶段需分别验证 Windows 高对比度、iOS Increase Contrast 与 Android 对比度设置。

## 必需 UI 状态

- [x] 默认、悬停/触控反馈、焦点、按下、选中和禁用。
- [x] 加载、空、错误、成功和破坏性确认。
- [x] 离线、部分成功、限流、恢复中和需要用户处理。
- [x] AI 等待、AI 不可用、未授权和低置信度。
- [x] 原文失效、字段缺失、演示数据与离线快照。
- [ ] 实现阶段需用真实系统权限拒绝、后台终止和网络恢复验证状态迁移。

## 响应式与自适应检查

### 设计预览

- [x] 原设计方向预览：4 套方向 × 4 类页面 × 2 个视口，共 32 个组合。
- [x] 跨平台预览：5 类设备 × 4 类页面 × 宽/窄承载，共 40 个组合。
- [x] 最终跨平台页面复测：20 个平台/页面组合。
- [x] 无脚本错误。
- [x] 无文档级横向溢出。
- [x] 无设备内部横向溢出。
- [x] 无可见顶部操作越界。

### 实现门禁

- [ ] Windows：320、640、768、1024、1280、1440、1920px 与 100%–200% 缩放。
- [ ] iPhone：小屏、标准屏、大屏，横竖屏与 Dynamic Type。
- [ ] iPad：Mini、标准、大屏，分屏、Stage Manager 与外接键鼠。
- [ ] Android：Compact、Medium、Expanded、Large、Extra Large。
- [ ] Android：折叠、展开、铰链姿态、多窗口和外接输入。
- [ ] 所有平台在布局变化后保持搜索、筛选、选择、输入和滚动上下文。

## 无障碍检查

- [x] 目标定义为 WCAG 2.2 AA。
- [x] 正文对比度目标 4.5:1，非文本控件目标 3:1。
- [x] 状态不只依赖颜色。
- [x] iOS/iPadOS 默认触控目标以 44×44pt 为目标。
- [x] Android 触控目标不小于 48×48dp。
- [x] 手势具有可见的等价操作。
- [x] 覆盖层、深链、系统返回和焦点恢复规则已定义。
- [ ] Windows 实现需通过 NVDA、键盘和高对比度测试。
- [ ] iOS/iPadOS 实现需通过 VoiceOver、Voice Control、Switch Control 与 Full Keyboard Access。
- [ ] Android 实现需通过 TalkBack、Switch Access、Voice Access 与 Compose Accessibility Checks。

## 反模式移除

- [x] 无装饰性渐变、发光或 AI 魔法视觉。
- [x] 无无意义大标题与营销页留白。
- [x] 无多层嵌套卡片堆叠。
- [x] 无仅靠 Tooltip、Toast 或颜色传达的关键信息。
- [x] 未将桌面三栏机械压缩为手机布局。
- [x] 未将手机大卡片机械放大到平板和桌面。
- [x] 未强迫 SwiftUI、Compose 与 shadcn/ui 像素一致。

## 已修复问题

1. 手机情报流顶部操作过密：隐藏重复搜索控件，保留筛选与同步。
2. 手机详情顶部次要操作造成越界：收藏移至内容操作区，顶部保留返回与原文。
3. 预览在窄承载中错误压缩平板设备框：固定逻辑设备尺寸并由外层滚动承载。
4. 设备切换动画造成自动检查读取中间尺寸：移除非必要尺寸过渡。
5. 移动状态栏残留 Windows 同步文本：按平台隐藏桌面标题栏信息。

## 已知限制与后续门禁

- 当前跨平台产物是高保真交互式 HTML 适配预览，不是 SwiftUI 或 Compose 运行时截图。
- 推送权限、后台刷新、系统终止、深链恢复与操作系统限制需要架构和实现验证。
- PRD 当前仍以 Windows MVP 为主要平台口径，必须编辑为 Windows + iOS/iPadOS + Android MVP，并重新运行 PRD 验证。
- 技术架构必须决定三端数据、同步、AI 调用、通知与本地优先语义如何一致实现。

## 视觉质量评分

| 维度 | 分数（1–5） | 说明 |
|---|---:|---|
| 信息层级 | 5 | 扫描、判断、证据与原文层级清楚 |
| 间距节奏 | 4 | 三端稳定；真实 Dynamic Type/字体缩放仍需验证 |
| 一致性 | 5 | 共享产品语义明确，平台控件保持原生 |
| 色彩 | 5 | 信号青克制，状态语义清楚 |
| 字体 | 4 | 已采用平台系统字体策略，需真机验证极端字号 |
| 状态覆盖 | 5 | 通用、故障、AI 与生命周期状态均有规范 |
| 响应式/自适应 | 5 | 桌面、手机、平板与折叠屏策略完整 |
| 无障碍 | 4 | 规范完整，真实 NVDA/VoiceOver/TalkBack 尚未执行 |
| 实施可行性 | 4 | 平台原生方案可靠，但 MVP 三端显著增加工程量 |

**平均分：4.6 / 5。**

## 官方平台依据

- Apple Layout：https://developer.apple.com/design/human-interface-guidelines/layout
- Apple Accessibility：https://developer.apple.com/design/human-interface-guidelines/accessibility
- SwiftUI NavigationSplitView：https://developer.apple.com/documentation/swiftui/navigationsplitview
- Android Window Size Classes：https://developer.android.com/develop/adaptive-apps/guides/use-window-size-classes
- Android Adaptive Guidelines：https://developer.android.com/develop/adaptive-apps/guides/adaptive-dos-and-donts
- Compose Accessibility：https://developer.android.com/develop/ui/compose/accessibility
- Compose Accessibility Testing：https://developer.android.com/develop/ui/compose/accessibility/testing
