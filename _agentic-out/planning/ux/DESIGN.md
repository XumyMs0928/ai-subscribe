---
title: DESIGN
status: complete
created: '2026-08-11'
updated: '2026-08-17'
project: ai-subscribe
source_documents:
  - '../ux-design-specification.md'
  - '../prd.md'
  - '../architecture.md'
aligned_requirements:
  - 'FR59-FR63'
  - 'FR64'
  - 'NFR33'
  - 'NFR35'
  - 'NFR52-NFR53'
  - 'NFR24'
  - 'NFR54'
visual_direction: '冷静决策台'
theme: '信号青'
platform_foundations:
  windows: 'shadcn/ui + Tailwind CSS + Radix'
  apple: 'SwiftUI'
  android: 'Jetpack Compose Material 3 Adaptive'
---

# DESIGN

本文档是 ai-subscribe 的实现级视觉契约。完整设计理由见 [`../ux-design-specification.md`](../ux-design-specification.md)；行为、流程和状态转换见 [`EXPERIENCE.md`](EXPERIENCE.md)。若两者存在歧义，以本文件的视觉令牌和 `EXPERIENCE.md` 的行为规则分别为准。

## Phase 1 Surface Policy

第一阶段只实现 Windows RSS 最小闭环。可见导航限于情报、雷达规则、同步结果/状态和设置；未实现的 GitHub/arXiv、项目发现、AI、通知、反馈、搜索、收藏、托盘和移动能力不得以禁用占位或假数据出现在发布候选中。既有 token、焦点、非颜色状态和响应式规则保持不变，后续平台视觉规范保留但不作为本阶段完成条件。

## Visual Principles

1. **判断优先：** 视觉层级首先回答“发生了什么、为什么重要、依据是什么、下一步是什么”。
2. **证据可见：** 原始事实、规则判断和 AI 内容必须通过区块标题、图标或文字标签区分，颜色仅作辅助。
3. **安静紧凑：** 使用单一主表面、细边界和稳定间距；不制造未读焦虑，不使用装饰性渐变、发光或卡片墙。
4. **异常局部化：** 离线、部分成功、AI 等待和来源错误附着在受影响区域，不以全屏错误替代仍可用内容。
5. **平台原生：** 三端共享语义和状态，不共享像素布局。Windows、Apple、Android 分别遵守本平台组件、导航、字体、焦点、输入方式与自动化语义惯例；当前产品不承诺盲人屏幕阅读器支持。
6. **状态完整：** 每个交互组件必须覆盖默认、悬停（适用时）、焦点、按下、选中、禁用、加载、错误、成功和破坏性状态。

## Brand And Voice Implications

- 品牌性格：冷静、可信、专业、克制、可核验。
- 界面文案优先陈述事实、影响范围、数据安全状态和可执行动作。
- 不使用“AI 魔法”“绝对准确”“实时必达”等承诺性或拟人化表达。
- 重要度、AI 置信度、来源可信度使用不同术语，不合并为单一神秘分数。
- 空状态和完成状态传达掌控感，不使用签到、庆祝动画或惩罚性红点。
- 错误标题应指出对象，例如“Vendor RSS 同步受限”，不得只写“发生错误”。

## Tokens

令牌名是跨平台合同。Windows 使用下列 OKLCH 值；SwiftUI 与 Compose 使用平台资产/主题映射到相同语义，不在页面中硬编码颜色。

### Color

| Token | Light | Dark | Contract |
|---|---|---|---|
| `color.background` | `oklch(0.985 0.004 240)` | `oklch(0.155 0.008 240)` | 应用主背景 |
| `color.foreground` | `oklch(0.190 0.012 240)` | `oklch(0.940 0.008 240)` | 主文本 |
| `color.surface` | `oklch(1 0 0)` | `oklch(0.190 0.010 240)` | 详情、表单和浮层内容面 |
| `color.muted` | `oklch(0.955 0.006 240)` | `oklch(0.245 0.010 240)` | 次级表面、悬停背景 |
| `color.mutedForeground` | `oklch(0.470 0.015 240)` | `oklch(0.700 0.014 240)` | 次要文本 |
| `color.primary` | `oklch(0.500 0.105 188)` | `oklch(0.700 0.115 188)` | 主操作、焦点关联和选中指示 |
| `color.onPrimary` | `oklch(0.985 0.004 188)` | `oklch(0.160 0.020 188)` | 主色上的文字与图标 |
| `color.accent` | `oklch(0.930 0.030 188)` | `oklch(0.270 0.045 188)` | 轻量强调、选中和悬停 |
| `color.border` | `oklch(0.890 0.008 240)` | `oklch(0.310 0.010 240)` | 区域、控件和分隔线 |
| `color.focusRing` | `oklch(0.560 0.120 188)` | `oklch(0.720 0.120 188)` | 键盘焦点 |
| `color.destructive` | `oklch(0.570 0.220 27)` | `oklch(0.650 0.200 27)` | 真错误、数据风险、不可逆操作 |

业务语义令牌必须分别注册浅色与深色值：

| Token | Meaning | Required non-color cue |
|---|---|---|
| `color.signalCritical` | 满足重大动态门槛 | “重大动态”文字 + 状态图标 |
| `color.signalHigh` | 高价值、非即时 | “高价值”文字 |
| `color.signalNormal` | 普通候选 | “普通候选”文字 |
| `color.sourceOfficial` | 官方或一手来源 | “官方来源”文字 |
| `color.aiGenerated` | AI 生成内容边界 | “AI 生成”文字/图标 |
| `color.statusOffline` | 离线但缓存可用 | “离线可用”文字 |
| `color.statusWarning` | 部分成功、限流、等待恢复 | 对象和恢复条件 |
| `color.statusSuccess` | 保存、同步或恢复成功 | 成功动词和对象 |

颜色对比门禁：正文与背景至少 `4.5:1`；大文本和非文本控件至少 `3:1`。红色不得用于普通未读或一般高优先级。

### Typography

| Role | Windows | Apple | Android | Usage |
|---|---|---|---|---|
| `type.display` | 28/36, 650 | `.title` | `headlineMedium` | 极少量完成/空状态标题 |
| `type.h1` | 24/32, 650 | `.title2` | `headlineSmall` | 情报详情标题 |
| `type.h2` | 20/28, 650 | `.title3` | `titleLarge` | 页面标题 |
| `type.h3` | 16/24, 650 | `.headline` | `titleMedium` | 区块标题 |
| `type.title` | 15/22, 600 | `.headline` | `titleSmall` | 情报列表标题 |
| `type.body` | 14/21, 400 | `.body` | `bodyMedium` | 摘要、说明、正文 |
| `type.label` | 13/18, 600 | `.subheadline` | `labelLarge` | 表单和操作标签 |
| `type.meta` | 12/18, 400–600 | `.caption` | `bodySmall` | 来源、时间、状态 |
| `type.code` | 12/18, 400 monospace | `.caption.monospaced()` | `bodySmall` monospace | URL、标识、错误码 |

- Windows 字体：`"Segoe UI Variable", "Segoe UI", system-ui, sans-serif`。
- Windows 技术字段：`ui-monospace, "Cascadia Code", Consolas, monospace`。
- Apple 和 Android 使用系统字体与用户缩放；不得固定字号绕过 Dynamic Type 或系统字体缩放。
- 关键元数据不得低于 12px；列表标题最多两行，提炼最多两行；详情正文建议最大阅读宽度 72 个中文字符。

### Spacing And Layout

基础网格为 `4`。只允许使用以下共享步长：

| Token | Value | Typical use |
|---|---:|---|
| `space.1` | 4 | 紧邻图标/微间距 |
| `space.2` | 8 | 图标与标签、同组元素 |
| `space.3` | 12 | 列表垂直内边距、紧凑面板 |
| `space.4` | 16 | 标准面板内边距 |
| `space.5` | 20 | 详情语义区块间距 |
| `space.6` | 24 | 页面区块与详情内边距 |
| `space.8` | 32 | 页面级分区 |

桌面布局合同：

- 主侧栏 `220–248px`，折叠宽度 `56px`。
- 情报列表 `380–520px`，允许用户调整并持久化宽度。
- 详情面板最小 `440px`，正文最大 `780px`。
- Windows 最小支持窗口 `1024×700`；`≥1280px` 使用完整三栏，`1024–1279px` 使用压缩三栏，`640–1023px` 使用导航+列表并将详情置于独立视图或 Sheet，`<640px` 使用单栏。
- 列表项内边距：水平 `14–16px`、垂直 `8–12px`；详情内边距 `20–24px`。

平台自适应合同：

- iPhone/紧凑 Android：底部主导航 + 单栈详情；筛选使用 Sheet/Bottom Sheet。
- iPad：按 Size Class 使用 NavigationSplitView；空间不足时折叠为两栏或单栈。
- Android 平板/折叠屏：按 Window Size Class 使用 Navigation Rail/Drawer 与 Adaptive 多窗格。
- 布局变化必须保留搜索、筛选、选择、未提交输入和滚动锚点，不得通过隐藏核心功能解决空间不足。

### Radius, Border, Shadow

| Token | Value | Usage |
|---|---:|---|
| `radius.badge` | 4px | Badge 与小状态标签 |
| `radius.control` | 6px | 按钮、输入和选中行 |
| `radius.panel` | 8px | 面板、Popover、Sheet |
| `radius.dialog` | 10px | Dialog、Command |
| `border.default` | 1px solid `color.border` | 固定区域和控件 |
| `focus.default` | 2px `color.focusRing`, offset 2px | 键盘焦点 |

- 固定侧栏、列表、详情、表格和 Alert 不使用阴影。
- 阴影仅用于 Dialog、Popover、Command 和拖动浮层。
- 不使用超过 12px 的通用圆角或消费型大胶囊。
- 选中列表项同时使用 `color.accent`、左侧 2px `color.primary` 指示和平台选中语义。

### Motion And Feedback

| Token | Value | Usage |
|---|---:|---|
| `motion.fast` | 120ms | 悬停、按下、焦点邻近变化 |
| `motion.standard` | 180ms | 面板状态、选择和局部内容更新 |
| `motion.emphasized` | 240ms | Sheet/Dialog 进入退出 |
| `motion.easing` | `cubic-bezier(0.2, 0, 0, 1)` | Windows 非弹性过渡 |

- 列表选择、收藏和反馈必须在本地立即更新；异步失败时原位回滚或显示可重试状态。
- 加载保留现有布局和内容，使用局部 Skeleton/Spinner，不执行整页闪白。
- 成功使用克制的 Toast/Sonner/Snackbar 或内联更新，不使用庆祝动画。
- 错误反馈必须持续到用户理解或处理，不能只依赖短暂 Toast。
- 开启 Reduce Motion / `prefers-reduced-motion` 时，将非必要动画时长降为 0；不以动画作为唯一状态提示。

## Component Appearance Rules

### Shared domain components

| Component | Required anatomy | Visual constraints |
|---|---|---|
| `IntelligenceFeedItem` | 来源/时间、标题、必要摘录、重要度、赛道、AI/离线状态、快捷操作 | 语义列表项，不包成 Card；标题优先；选中/焦点/收藏相互独立 |
| `EvidenceDetailPanel` | 标题、来源、发生了什么、为什么重要、影响、重要度、AI 置信度、依据、溯源、操作 | 从判断到证据渐进展开；AI 与事实分区 |
| `SourceProvenanceGroup` | 发布方、作者、原始标题、链接、时间、可用状态、关联依据 | 使用列表/描述列表；缺失字段写“未提供” |
| `SyncHealthSummary` | 总体状态、最后同步、来源结果、AI 队列、数据安全、恢复动作 | 部分成功分别展示成功与失败范围 |
| `SyncResultSummary` | 本轮时间、总结果、成功/失败来源、候选数量、最小规范化条目入口、后续处理状态 | 使用现有成功/警告/错误语义；零结果与部分成功必须保留来源级文字，不以 AI 状态覆盖同步结果 |
| `ProcessingStateControl` | 未查看、已判断、待研究三态及当前态标签 | 与收藏、选中、价值反馈、AI 状态使用独立位置和非颜色提示；不得用星标或 AI 图标表达处理状态 |
| `RuleImpactPreview` | 修改摘要、影响范围、风险、冲突、生效时间、历史数据说明 | 阻断性无效配置使用 `color.destructive` 且不可保存；过窄风险使用 `color.statusWarning` 且可确认；无风险不增加确认层 |
| `ProgressiveSetupGuide` | 进度、赛道、来源、刷新周期、AI 说明、跳过 | 非阻塞；每步独立保存；始终提供跳过/稍后继续；设置根页固定入口显示“配置引导”和进度状态 |
| `SourceDeliveryReadiness` | RSS/Atom、GitHub Release、arXiv 的交付/启用/同步状态与阻塞原因 | 三类逐项显示；不得用总体成功掩盖缺失；状态必须有文字 |
| `DeviceScopeNotice` | 当前设备标识、受影响数据、无云备份/跨设备同步说明 | 使用常规信息层级；破坏性后果只在实际删除时使用 destructive |
| `DistributionDisclosureStatus` | 平台、签名/安装资格、授权对象、七类隐私披露完成度 | 属于构建/关于/诊断上下文；不得复用重大动态视觉语义 |
| `GitHubDiscoverySubscriptionForm` | 名称、赛道、语言、Topic、Star/Fork 当前门槛、增长条件、观察窗口、组合逻辑摘要 | 复用表单、阻断错误和过窄风险语义；不同维度“同时满足”、同维度“满足任一”必须以文字呈现 |
| `GitHubDiscoveryResultItem` | 仓库身份、新发现/新符合条件分类、主要语言、Topic、Star/Fork、命中依据、观测时间、关注与 Release 状态 | 语义列表项而非营销卡片；分类、关注、发现运行与 Release 同步四类状态必须分区且有文字 |
| `GitHubMonitoringControl` | 忽略当前订阅、停用自动关注、转固定关注，以及其他有效关注说明 | 忽略与停用使用 warning/neutral，只有删除历史数据才使用 destructive；固定关注优先级必须可见 |

### Controls

- 每个容器原则上只有一个 Primary；常规打开、查看、重试使用 Neutral；列表内次要操作使用 Quiet；不可逆操作使用 Destructive。
- 图标按钮必须有可访问名称和 Tooltip/平台等价说明；Tooltip 不得承载关键事实或唯一入口。
- Windows 紧凑控件可为 32px，高频触控兼容控件至少 36px；Apple 目标 44×44pt；Android 不小于 48×48dp。
- 表单标签置于控件上方，说明和错误置于下方；不得使用 placeholder 代替标签。
- 情报流不得使用 Data Table；Table 仅用于来源、诊断、规则统计等结构化数据。
- Badge 只承载短状态；完整解释放在详情、Popover 或状态区。
- 不可逆操作使用 Alert Dialog，明确对象、删除内容和保留内容。

### Platform mappings

- Windows：Sidebar/Resizable/Scroll Area、Command、Popover、Dialog/Alert Dialog、Sheet、Sonner。
- Apple：TabView、NavigationStack、NavigationSplitView、List、Section、DisclosureGroup、Form、Sheet、Alert、ConfirmationDialog。
- Android：Navigation Bar/Rail/Drawer、LazyColumn、Adaptive Pane、Scaffold、Modal Bottom Sheet、Snackbar、Dialog。
- 平台手势必须存在可见等价操作；不得把 Windows DOM/CSS 结构移植到移动端。

## Asset, Image, And Icon Guidance

- MVP 使用平台原生或一致的线性图标集；同一语义在各平台保持名称与含义一致，但允许采用平台惯用图形。
- 图标只辅助文字，不单独表达重要度、AI、离线、错误或破坏性后果。
- 来源 Logo 为可选辅助资产；缺失时使用发布方文字，不生成仿冒品牌标识。
- 不使用装饰性 AI 星芒、机器人、发光渐变、无意义雷达动画或大面积品牌插画。
- 情报正文中的外部图片默认不作为主列表必需内容；加载失败不得影响标题、来源、摘要和判断。
- 截图与视觉基准位于本目录的 `design-directions-*.png` 和 `adaptation-*.png`；HTML 文件仅用于设计方向、平台和页面状态预览，不是业务交互实现。

状态语义增补：

- “三类来源就绪”必须由 RSS/Atom、GitHub Release、arXiv 三项可见状态共同构成，不新增汇总颜色或单一成功图标来替代逐项结果。
- 同步结果的“全部成功”“部分成功”“零候选”和“失败”只复用既有 `color.statusSuccess`、`color.statusWarning`、`color.destructive` 与中性文字层级；候选数量、来源范围和下一步必须有文字，AI 未配置或失败不得改变同步结果的视觉分类。
- “未查看、已判断、待研究”采用稳定文字标签和状态图标；收藏星标、列表选中、价值反馈、AI 等待/完成可与任一处理状态同时出现，视觉上不得互相替代或覆盖。
- 配置校验严格使用两类语义：字段不可解析、范围越界、上下界倒置、来源地址/标识无效或协议不受支持属于 destructive 阻断；过窄条件属于 warning 确认。无风险保存保持普通 Primary 路径，不展示成功色确认门。
- Windows 再次启动只允许对现有窗口使用既有 `focus.default` 与平台任务栏注意语义；不新增“第二实例”页面、品牌动画或独立强调色。
- “仅当前设备”是作用域说明，不是警告；仅当操作会删除且不可恢复时，才使用 `color.destructive` 和 Alert Dialog。
- 签名无效、安装资格无效和隐私披露缺失使用 `color.statusWarning` 或 `color.destructive`，并始终配套具体对象、原因和修复动作。
- 测试分发状态不得使用 `color.signalCritical`、`color.signalHigh` 或雷达/情报图标，防止与内容重要度混淆。
- GitHub 发现分类使用中性文字标签：“新发现项目”“新符合条件项目”“增长基线已建立”，不得借用 `color.signalCritical` 或 `color.signalHigh` 暗示情报重要度。
- GitHub 关注状态使用稳定文字：“自动关注”“自动关注已停用”“固定关注”“此订阅已忽略”；固定关注可使用 `color.primary` 的低强调边界，停用/忽略使用中性或 `color.statusWarning`，不得把停用表现为历史数据已删除。
- GitHub 发现运行状态与 Release 同步状态必须分列或分区：成功有结果/成功零结果使用成功与中性层级，部分完成/限流使用 `color.statusWarning`，失败使用 `color.destructive`；发现错误不得覆盖 Release 成功状态。
- 命中依据优先使用描述列表与可展开分组；数值、观察窗口和时间戳使用 `type.meta`/`type.code`，不得以图表替代可核验的实际值。
- 自动关注容量及超限视觉暂不定义；在产品给出明确上限前，不展示“无限”“无上限”或虚假的容量进度。

## Implementation Notes

- 共享领域状态和设计令牌；各平台分别实现视图及原生导航。
- 页面和业务组件不得硬编码颜色、间距、圆角或阴影；先注册令牌，再消费令牌。
- 状态模型使用可判别类型，禁止用多个互相矛盾的布尔值拼装状态。
- Windows 使用 Storybook 或等效场景；Apple 使用 Xcode Preview；Android 使用 Compose Preview 覆盖每个组件的状态、主题与尺寸类别。
- 视觉回归至少覆盖浅/深主题、默认/焦点/选中/禁用/加载/错误状态及桌面/手机/平板关键宽度。
- 组合状态回归至少覆盖三种处理状态 × 收藏/未收藏 × AI 等待/完成，以及同步零结果/部分成功、配置阻断错误/过窄警告和 Windows 现有窗口获焦。
- GitHub 发现组件回归至少覆盖新发现/新符合条件/增长基线、手动与多订阅重叠、固定/自动/停用/忽略、成功零结果/部分分页失败/限流，以及发现失败但 Release 同步成功。
- 所有变更同时核对 [`EXPERIENCE.md`](EXPERIENCE.md) 的流程、状态、键盘/触控、焦点、非颜色表达和缩放合同；不得用 HTML 视觉预览代替真实应用验收。
