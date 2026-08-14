---
stepsCompleted:
  - step-01-document-discovery
  - step-02-prd-analysis
  - step-03-epic-coverage-validation
  - step-04-ux-alignment
  - step-05-epic-quality-review
  - step-06-final-assessment
inputDocuments:
  - '_agentic-out/planning/prd.md'
  - '_agentic-out/planning/architecture.md'
  - '_agentic-out/planning/epics.md'
  - '_agentic-out/planning/ux-design-specification.md'
  - '_agentic-out/planning/ux/DESIGN.md'
  - '_agentic-out/planning/ux/EXPERIENCE.md'
status: needs-work
date: '2026-08-13'
project: 'ai-subscribe'
---

# Implementation Readiness Assessment Report

**Date:** 2026-08-13
**Project:** ai-subscribe

## Document Discovery

### Documents selected for assessment

| Type | Authoritative document | Size |
|---|---|---:|
| PRD | `_agentic-out/planning/prd.md` | 98,851 bytes |
| Architecture | `_agentic-out/planning/architecture.md` | 121,432 bytes |
| Epics & Stories | `_agentic-out/planning/epics.md` | 148,562 bytes |
| UX specification | `_agentic-out/planning/ux-design-specification.md` | 114,134 bytes |
| UX visual spine | `_agentic-out/planning/ux/DESIGN.md` | 18,170 bytes |
| UX behavior spine | `_agentic-out/planning/ux/EXPERIENCE.md` | 33,684 bytes |

### Discovery findings

- Required document types found: 4/4.
- Whole/sharded duplicate conflicts: 0.
- Missing required documents: 0.
- Historical readiness and validation reports are excluded as authority inputs; they may only be used later as historical comparison if explicitly needed.


## PRD Analysis

### Functional Requirements

- **FR1:** 用户无需注册或配置 AI 服务即可首次使用产品。
- **FR2:** 用户首次启动时可以浏览按“演示数据”口径明确标识的示例情报，且这些记录不得触发真实通知或计入验证指标。
- **FR3:** 用户首次启动时可以使用预置的 AI 技术赛道和默认来源。
- **FR4:** 用户可以添加、修改、启用或停用关注赛道。
- **FR5:** 用户可以配置关键词、排除词、关注来源和刷新周期。
- **FR6:** 用户可以配置来源可信度、提醒阈值、免打扰时段和提醒频率上限。
- **FR7:** 用户可以订阅公开的 RSS/Atom 信息源。
- **FR8:** 用户可以监控指定 GitHub 项目的 Release 更新。
- **FR9:** 用户可以监控指定主题、关键词或分类的 arXiv 论文。
- **FR10:** 系统可以在每台受支持设备上按照该设备的来源配置执行增量同步；移动端在系统不允许后台执行时，可以在下次获得执行机会或用户前台触发后继续。
- **FR11:** 用户可以在当前设备手动触发全部来源或单个来源同步。
- **FR12:** 系统可以分别显示当前设备各来源的同步进度、最后成功时间、失败状态、待处理任务、数据新鲜度及后台受限状态，并提供最近一次同步的新增、更新、跳过与失败数量及同步结果入口。
- **FR13:** 系统可以将不同来源内容转换为统一的情报记录。
- **FR14:** 系统必须为每条情报保留来源类型、发布方、原始标题、有效原文链接、发布时间、采集时间和内容标识；来源能够提供作者时还必须保留作者，不能提供时须明确记录为不可获得。
- **FR15:** 用户可以查看情报首次发现时间、最后更新时间和原文可用状态。
- **FR16:** 系统可以区分原始事实、规则判断与 AI 分析。
- **FR17:** 对于未收藏情报，系统仅可长期保存元数据、原文链接、符合“必要摘录”口径的片段和 AI 分析，不得持久化完整正文或可实质还原全文的内容。
- **FR18:** 用户收藏情报时，可以保存合法获取的完整正文快照；取消收藏后可以删除该正文及其派生索引。
- **FR19:** 系统可以依据赛道匹配、来源可信度、新鲜度、技术影响和用户配置规则形成“情报价值”判断，且用户能够查看影响该判断的因素。
- **FR20:** 系统可以为情报给出“重要程度”，并展示促成该结果的赛道、来源、新鲜度、技术影响、规则命中及适用的 AI 判断依据。
- **FR21:** 系统可以按照用户当前主情报流阈值将“高价值情报”与“普通候选”区分，阈值变化不得删除或改写原始情报记录。
- **FR22:** 用户可以查看被过滤的内容及其过滤原因。
- **FR23:** Phase 1 中，系统可以依据“确定性重复或关联”口径识别重复内容；不得仅凭语义相似自动合并记录。
- **FR24:** Phase 1 中，系统可以依据上述确定性共享标识关联至少 2 个指向同一事件的来源记录，并保留每个来源各自的标题、链接、发布时间、发布方及其他溯源记录供用户展开核验。
- **FR25:** 用户可以在当前设备配置、更新或移除兼容 OpenAI 接口规范的第三方 AI 服务凭据；配置不得自动传播到其他设备。
- **FR26:** 系统可以在每台设备首次向第三方 AI 服务发送内容前说明将发送的数据，并取得用户确认。
- **FR27:** 系统可以默认对新情报执行中文翻译、摘要、主题提取、重要度判断、置信度生成和理由生成，并在情报详情中展示置信度。
- **FR28:** 用户可以在当前设备按来源决定是否允许向第三方 AI 服务发送内容，并可随时撤销授权。
- **FR29:** 系统必须按“明确标识 AI 内容”口径区分所有 AI 生成字段与原始事实、规则判断，并显示每条情报当前的 AI 处理状态。
- **FR30:** AI 服务不可用、输出无效或用户未配置有效凭据时，系统仍可以保留并展示原始情报，供稍后重试。
- **FR31:** 用户可以浏览经过筛选和排序的主情报流。
- **FR32:** 用户可以按赛道、来源、时间、重要度、用户处理状态和收藏状态筛选情报；用户处理状态与 AI 处理状态必须使用不同名称和筛选语义。
- **FR33:** 用户可以搜索已保存的情报元数据、符合“必要摘录”口径的片段和当前可用的收藏正文；搜索结果必须遵守内容生命周期边界，不得暴露已删除正文或未获准持久化的全文内容。
- **FR34:** 用户可以查看单条情报的内容提炼、评分依据、溯源信息和关联来源。
- **FR35:** 用户可以从情报详情打开原始内容进行核验。
- **FR36:** 用户可以收藏、取消收藏并离线阅读已保存的正文快照；收藏变化及正文删除不得隐式改变用户处理状态。
- **FR37:** 当前设备只有在本地完成情报判定，且该情报同时满足“重大动态”口径、完整溯源要求、用户提醒阈值和当前设备通知授权时，才可以向操作系统提交通知；条件不满足时不得以重大动态名义通知。MVP 不依赖远程推送生成该通知。
- **FR38:** 用户点击通知后可以直接打开对应情报详情。
- **FR39:** 系统可以在当前设备内防止同一情报或内容版本产生重复通知；MVP 不承诺跨设备通知去重。
- **FR40:** 系统可以遵守用户设置的免打扰时段、通知频率上限和操作系统通知授权；权限不足时须显示提醒不可送达状态。
- **FR41:** 在 Windows 上，用户关闭主窗口后，系统可以继续在托盘运行和同步。
- **FR42:** 在 Windows 上，用户可以从托盘显示主窗口、立即同步或显式退出应用，并可以控制开机自动启动。
- **FR43:** 用户可以将情报标记为“值得立即知道”“有价值但不紧急”“无价值”“重复”或“分析错误”。
- **FR44:** 用户可以记录已确认的漏报及其原因。
- **FR45:** 系统可以汇总提醒有效率、主情报流有效率、漏报和每日使用情况，支持四周个人验证。
- **FR46:** 系统可以按赛道、来源、规则命中和筛选结果汇总用户反馈，供用户识别误报与漏报原因并手动校准“可解释”的筛选规则；Phase 1 不得据此自动修改权重或规则。
- **FR47:** 当前设备的单个来源失败时，系统可以继续处理其他来源并保留已成功结果。
- **FR48:** 用户可以在当前设备单独重试失败来源或待处理的 AI 分析任务；来源处于强制退避期间时须说明暂不可重试的原因和时间。
- **FR49:** 离线、后台挂起或应用恢复后，用户仍可以浏览、搜索、筛选、收藏、配置和反馈；系统再次获得联网与执行机会后可以恢复有效待处理任务。
- **FR50:** 用户可以查看按“可操作错误信息”口径呈现的来源级状态，并通过当前平台提供的复制或分享能力导出不含 API Key、完整正文、敏感提示内容或其他凭据的脱敏诊断信息。
- **FR51:** 在 iOS/iPadOS 与 Android 上，系统可以保存应用挂起、终止或进程被回收前已经完成的数据和可恢复任务状态；再次获得执行机会后，可以继续处理可恢复任务，或将无法继续的任务明确标记为可重试。
- **FR52:** 用户可以在所有平台查看最后成功同步时间、当前同步状态和影响时效性的后台限制，并可在前台手动触发同步。
- **FR53:** 系统只能在需要发送重大动态通知时请求通知权限；用户拒绝或撤销后，系统不得阻断采集、浏览、检索、溯源、反馈或前台同步，也不得在每次启动时重复强迫授权。
- **FR54:** 用户从系统通知进入应用时，无论应用正在前台、后台或尚未运行，系统都可以打开对应情报详情；目标已删除或不可用时必须说明原因并提供返回主情报流的入口。
- **FR55:** 用户可以在 Windows、iPhone、iPad、Android 手机和 Android 平板上完成首启、主情报流浏览、搜索、详情、收藏、溯源、设置、反馈和同步状态查看。
- **FR56:** 系统可以根据桌面、手机、平板、窗口宽度、横竖屏和输入方式调整导航与布局，同时保持功能含义、筛选状态和当前任务上下文一致。
- **FR57:** 系统必须将配置、收藏、反馈、凭据和历史情报保存在当前设备的数据边界内，并在首次使用与设置中明确说明 MVP 不提供云端备份或跨设备同步。
- **FR58:** 用户可以在卸载、清除应用数据或执行可能删除本地数据的操作前看到数据保留后果；产品不得暗示未配置的恢复或迁移能力。
- **FR59:** 用户在当前设备完成全部来源或单个来源同步后，可以分别查看 RSS/Atom、GitHub Release 和 arXiv 本次同步的新增、更新、跳过与失败数量，以及本次成功转换的最小结果列表。每项至少展示来源类型、发布方、原始标题、发布时间（来源可提供时）、采集时间和原文链接。该结果不得依赖 AI 凭据、评分、搜索或完整详情；零结果和部分失败须明确呈现，且部分失败不得隐藏已成功结果。
- **FR60:** 用户可以在当前设备将每条情报显式设置为“未查看”“已判断”或“待研究”，并在三种状态间转换；新入库情报默认为“未查看”，仅浏览列表或打开详情不得自动改变状态。状态须在应用重启、离线、平台生命周期恢复和受支持版本升级后保留。收藏是独立维度，任一用户处理状态均可收藏或不收藏，收藏变化及正文删除不得隐式改变处理状态。
- **FR61:** 系统可以在用户已能浏览主情报流后提供分步配置引导；所有可延后步骤均可跳过，跳过或退出不得阻断主情报流、同步、搜索和详情查看。用户可以从“设置”根级页面中名称为“配置引导”的固定入口继续未完成步骤；从主情报流到达该入口不得超过两次用户发起的导航操作，且已保存的配置不得丢失。
- **FR62:** 系统在保存赛道、关键词、排除词、来源或阈值配置前，必须按照验收术语中的判定口径区分阻断性无效配置与“过窄配置风险”。阻断性无效配置不得保存，并须指出受影响字段、所属无效类别和修正方式；检测到过窄配置风险时，须指出受影响条件和可能漏报的后果，并允许用户返回修改或明确确认后保存；未检测到风险时不得要求额外确认。
- **FR63:** 在 Windows 同一用户会话中，系统只允许一个可交互应用实例；已有实例运行时，包括主窗口已隐藏至托盘的状态，再次启动必须显示并聚焦现有主窗口，不得创建第二个可交互实例。
- **FR64:** 用户可以在当前设备保留通过仓库标识手动建立的 GitHub Release 固定关注，并可以创建、修改、启用或停用 GitHub 项目发现订阅。发现订阅可以按关注赛道、主要语言、GitHub Topic、Star/Fork 当前门槛及“GitHub 增长条件”发现公开仓库；系统必须按“新发现项目”和“新符合条件项目”口径展示结果，为每个项目显示“GitHub 发现命中依据”，并将符合条件的项目自动纳入 Release 监控。用户可以忽略发现项目、停用其自动关注或将其转为固定关注。系统必须按“GitHub 仓库规范身份”合并手动关注、多个发现订阅及仓库重命名或转移产生的同一项目，固定关注优先于自动关注；同一项目在当前设备只能形成一个项目对象和一个有效 Release 监控对象，不得产生重复 Release 记录或重复通知。发现订阅停用、条件失配或项目被忽略不得删除已经保存的 Release、反馈、收藏或其他合法保留的历史数据；发现任务失败或受限不得阻断既有项目的 Release 同步。

**Total FRs: 64**

### Non-Functional Requirements

- **NFR1:** 条件：在推荐硬件上首次安装并启动候选构建，且尚无用户数据。测量方法：从用户发起启动开始计时，至示例情报可见且可滚动、打开详情为止。通过判据：每个平台至少 100 次样本的 P95 ≤5 秒。保护目标：避免首次体验因等待过长而中断。
- **NFR2:** 条件：本地已加载固定 50,000 条情报数据集。测量方法：分别测量主情报流首屏、常用筛选、关键词搜索和搜索结果首屏的操作完成时间。通过判据：每类操作至少 100 次样本的 P95 均 <200 毫秒。保护目标：保证数据增长后日常浏览和检索仍然流畅。
- **NFR3:** 条件：来源同步、正文处理和 AI 分析分别单独运行及同时运行。测量方法：并发执行浏览、滚动、搜索和设置保存，各操作至少 100 次，并记录界面无响应事件。通过判据：界面无响应事件为 0，用户操作 P95 ≤500 毫秒，任何单次操作不得因后台任务持续阻塞超过 1 秒。保护目标：保证后台工作不剥夺用户对应用的控制。
- **NFR4:** 条件：当前设备已在本地完成内容采集和高优先级判定，并满足展示或提醒条件；移动端仅统计应用正在运行或系统已授予执行机会的时段。测量方法：记录本地判定完成时间以及进入主情报流或向操作系统提交通知的时间，使用至少 100 条固定样本。通过判据：每条样本均在 60 秒内进入主情报流；满足提醒条件且已获通知权限的样本也须在 60 秒内向操作系统提交通知；系统未授予后台执行机会的时段不进入产品可控时延，恢复执行后仍有效的任务须在 60 秒内处理；已失去提醒时效的任务可以进入主情报流，但补发“即时”通知的次数必须为 0。保护目标：确保设备本地重大动态及时支持决策，同时不承诺操作系统未授予的后台时段或远程推送能力。
- **NFR5:** 条件：Windows 候选构建位于后台空闲状态，当前无同步、AI 分析、用户交互或待执行任务。测量方法：按统一资源采样规则记录应用进程总 CPU 占用。通过判据：10 分钟窗口平均 CPU <2%，且不得出现持续 30 秒以上高于 5% 的占用。保护目标：减少桌面常驻对电量、散热和其他工作的影响。
- **NFR6:** 条件：Windows 候选构建已加载固定 50,000 条数据集，但当前未执行同步和 AI 分析。测量方法：依次完成主情报流、搜索、详情、收藏和设置操作，随后按统一资源采样规则记录应用进程工作集。通过判据：10 分钟窗口内存 P95 ≤250 MB，且窗口内不得持续增长超过 10%。保护目标：控制长期常驻内存压力并识别明显资源泄漏。
- **NFR7:** 条件：多个来源同时同步，并分别向每类来源注入超时、无效响应和连接失败。测量方法：核对未故障来源的完成状态、已保存记录及失败来源的错误状态。通过判据：全部固定场景中，未故障来源均正常完成，已成功保存结果的记录数和字段值不回退，失败仅归属于对应来源。保护目标：将单点外部故障限制在最小范围。
- **NFR8:** 条件：对固定数据集中的同一内容版本跨轮次重复同步，并包含应用重启和网络恢复后的重试。测量方法：比对内容标识、入库记录和通知记录。通过判据：重复入库率为 0，重复通知率为 0。保护目标：防止重复内容污染信息流和打扰用户。
- **NFR9:** 条件：分别在保存完成后及任务处理中触发应用崩溃、系统重启、网络中断、移动端挂起、终止和低资源回收。测量方法：恢复应用后核对已完成记录的数量与字段值，并观察未完成任务的恢复或重新执行结果。通过判据：已完成数据损坏或丢失为 0；每个未完成任务均安全恢复、重新执行或明确标记为可重试，且不产生重复结果。保护目标：保护用户数据并确保意外中断后可继续工作。
- **NFR10:** 条件：分别进入离线、无有效 AI Key、AI 服务超时三种状态，并预先加载固定缓存数据。测量方法：逐项执行主情报流浏览、搜索、详情查看和收藏访问。通过判据：三种状态下缓存内容可访问率均为 100%，且外部服务失败不得阻断这些操作。保护目标：维持本地优先和弱网可用性。
- **NFR11:** 条件：使用同时包含完整溯源、缺失来源、无原始链接和来源失效内容的固定样本。测量方法：核对高优先级入库结果、溯源字段和重大提醒记录。通过判据：触发重大提醒的高优先级情报有效溯源信息完整率为 100%；任一未达到溯源门槛的样本触发提醒即失败。保护目标：避免不可核验内容驱动重要决策。
- **NFR12:** 条件：从每个受支持的既有数据版本升级至候选构建，数据中包含用户配置、收藏、反馈和历史情报。测量方法：升级前后比较记录数量、关键字段值及关联关系。通过判据：上述数据的静默丢失、清空或错误改写数量均为 0；无法迁移时必须停止升级并提供明确恢复提示。保护目标：保护升级过程中的长期用户资产。
- **NFR13:** 条件：在同步、分析和通知的不同阶段触发系统休眠、网络断开、移动端挂起、进程回收及设备重启，随后恢复并执行补偿同步。测量方法：核对任务执行记录、分析结果和通知结果。通过判据：每个逻辑任务最多产生一份有效入库结果、一份有效分析和一次通知；并发重复任务数量为 0。保护目标：防止环境恢复造成重复工作和重复打扰。
- **NFR14:** 条件：运行期间注入可检测的数据库完整性错误。测量方法：尝试对受影响数据继续写入，并检查错误提示中的影响对象、数据安全状态和恢复步骤。通过判据：相关写入成功次数为 0；所有固定错误场景均显示可操作恢复提示，且未受影响的已保存数据保持可读取。保护目标：阻止错误扩散并帮助用户安全恢复。
- **NFR15:** 条件：配置、更新、使用和删除 API Key，并执行正常操作、错误诊断与诊断导出。测量方法：检查操作系统安全凭据存储状态，并搜索内容数据库、界面持久状态、日志、诊断导出和测试数据中的完整密钥及可还原片段。通过判据：密钥仅存在于操作系统提供的安全凭据存储中；其他检查位置命中数为 0。保护目标：防止凭据泄露和非预期持久化。
- **NFR16:** 条件：访问全部受支持外部来源和 AI 服务，并配置一个用户主动指定的本地服务地址。测量方法：记录每次连接的目标和传输协议。通过判据：除用户主动配置的本地服务地址外，外部连接未加密次数为 0；发生证书或加密校验失败时不得继续传输内容或凭据。保护目标：防止传输过程中的窃听和篡改。
- **NFR17:** 条件：来源内容包含指向本机地址、环回地址、内网地址、非允许协议及经重定向抵达这些目标的链接。测量方法：执行固定恶意样本并记录所有实际网络访问目标。通过判据：对上述目标和协议的实际访问次数为 0，且其他合法来源仍可继续处理。保护目标：防止不可信内容诱导应用探测或访问受保护网络资源。
- **NFR18:** 条件：外部来源分别返回超大响应、重定向链、慢速响应及其组合。测量方法：记录单次响应接收字节数、跟随重定向次数、处理持续时间和应用资源状态。通过判据：单个外部响应最多接收 10 MB、最多跟随 5 次重定向、单次处理最长 30 秒；达到任一上限后必须终止该请求并保留可诊断错误，且应用保持可操作。保护目标：防止单一恶意或异常来源耗尽本机资源。
- **NFR19:** 条件：对未收藏正文完成采集、临时处理、AI 分析、失败重试和应用重启。测量方法：处理完成后检查数据库、日志、崩溃信息及持久化临时文件中的完整正文或可复原正文。通过判据：未收藏完整正文的持久化残留数量为 0；仅允许保留需求明确要求的必要摘录和分析结果。保护目标：最小化用户未选择保留的内容及潜在敏感信息。
- **NFR20:** 条件：在包含密钥、授权头、完整正文和敏感提示内容的正常及故障场景中生成诊断信息。测量方法：导出并检查全部诊断字段。通过判据：完整密钥、可用授权头、完整正文和敏感提示内容的命中数均为 0；被移除内容以不泄露原值的标识替代。保护目标：使故障排查材料能够安全分享。
- **NFR21:** 条件：分别对来源授予或撤销第三方 AI 分析授权，并在关闭总开关时存在排队任务。测量方法：记录授权状态变化后的新外发请求及其来源。通过判据：未经用户确认或未获对应来源授权的内容外发次数为 0；关闭后不得发起新的内容请求，排队任务不得继续发送。保护目标：确保用户掌控内容是否交给第三方处理。
- **NFR22:** 条件：收藏正文已建立索引并可搜索，随后用户删除该正文及其索引。测量方法：重新启动应用后，通过关键词搜索、详情、收藏和历史入口尝试访问被删正文。通过判据：被删正文的搜索命中、展示和可访问残留均为 0；不含正文的必要收藏元数据仅在产品规则允许时保留。保护目标：落实用户删除意图并避免内容残留。
- **NFR23:** 条件：分别向 RSS/Atom、GitHub Release 和 arXiv 来源连接注入连接失败、无效响应和解析错误，同时保持其他来源连接正常。测量方法：核对错误归属、诊断记录、可重试状态及其他来源连接结果。通过判据：每项错误均被准确归属于对应来源并可独立重试；其他来源连接成功率不受该错误影响。保护目标：实现来源级故障隔离和可诊断恢复。
- **NFR24:** 条件：来源分别返回带有效重试指示、无重试指示和连续限流响应。测量方法：记录限流后的每次请求时间及来源给出的重试时间。通过判据：存在有效重试指示时，在指定时间前请求次数为 0；无明确指示时首次重试间隔 ≥60 秒，连续重试间隔不得缩短，第三次间隔至少为首次的 4 倍，且不得持续高频请求。保护目标：尊重外部服务容量并避免扩大限流。
- **NFR25:** 条件：AI 服务分别返回超时、限流、无效结构和不完整结果。测量方法：检查用户界面、保存结果和后续流程中的状态标识。通过判据：上述结果被标记为有效 AI 分析的次数为 0；系统必须保留明确的失败或待重试状态，不得以推测性兜底文本伪装成功。保护目标：维护 AI 分析的可信边界。
- **NFR26:** 条件：每类来源使用缺失作者、缺失摘要、不同发布时间格式及同一内容更新的固定样本。测量方法：同步后核对有效内容是否入库、溯源是否保留、更新是否反映。通过判据：仍可溯源的有效样本因上述字段差异被丢弃的数量为 0；无法解析的字段必须留空或明确标识，不得伪造值。保护目标：提高真实世界来源差异下的数据覆盖与可信度。
- **NFR27:** 条件：外部来源或 AI 服务在任务处理过程中不可用，形成待处理任务后恢复服务。测量方法：观察任务是否自动继续，并核对入库、分析和通知结果。通过判据：所有仍有效的待处理任务均无需用户重新创建即可继续；每个逻辑任务最多产生一份有效入库结果、一份有效分析和一次通知。保护目标：在短暂外部故障后自动恢复且不产生重复副作用。
- **NFR28:** 条件：在每类受支持设备上全新安装，用户不注册、不连接云端账户、不配置 AI Key 且不授予通知权限。测量方法：从首次启动开始执行示例情报浏览、详情查看和基础筛选。通过判据：上述首次体验任务完成率为 100%，过程中不得出现强制注册、强制云端连接、强制配置 AI Key 或强制通知授权。保护目标：降低首次使用门槛并兑现本地优先体验。
- **NFR29:** 条件：分别展示示例数据、原始事实、规则判断、AI 分析、离线状态和待分析状态，并在彩色、灰度和常见色觉差异模拟下检查。测量方法：核对每种状态是否具有可见文字、图标或形状标识。通过判据：六类状态的非颜色区分覆盖率为 100%，任意两类不得仅以颜色差异区分。保护目标：避免用户混淆事实、推断和系统状态，并支持色觉差异用户。
- **NFR30:** 条件：在 Windows 及连接外部键盘的平板上仅使用键盘操作候选构建。测量方法：依次完成主情报流浏览、搜索、打开详情、添加与移除收藏、修改并保存设置，同时检查焦点位置。通过判据：全部核心任务均可完成；每一步均有可见焦点，且不存在无法退出的焦点区域。保护目标：支持键盘用户并提高桌面与平板操作效率。
- **NFR31:** 条件：在 Windows 100%、125%、150%、175% 和 200% 缩放比例下，分别使用系统浅色和深色主题检查主情报流、搜索、详情、收藏、设置及错误提示。测量方法：逐屏核对文字可读性、内容裁切、控件重叠和键盘可达性。通过判据：全部组合中核心文字可读、核心操作可触达，阻止阅读或操作的裁切与重叠数量为 0。保护目标：保证常见显示设置下的可读性和可操作性。
- **NFR32:** 条件：触发固定故障场景中的每类可展示错误。测量方法：检查对应错误提示是否包含受影响来源或任务、当前数据安全状态及用户可采取的恢复操作。通过判据：三项信息的完整率为 100%；无法由用户恢复时必须明确说明无需操作或等待条件。保护目标：减少故障焦虑并帮助用户采取正确恢复行动。
- **NFR33:** 条件：完成首次配置后出现后续引导，并分别选择跳过和稍后继续。测量方法：尝试直接进入主情报流浏览，再从主情报流开始记录用户发起的导航操作，进入“设置”根级页面中的“配置引导”并恢复未完成步骤。通过判据：后续引导阻塞直接浏览的次数为 0；所有可延后步骤均可跳过；从主情报流到达“配置引导”入口不得超过两次用户发起的导航操作，且恢复后已保存配置丢失数为 0。保护目标：让用户掌控学习节奏，避免引导妨碍核心任务。
- **NFR34:** 条件：在基线规定的 Windows 10 x64 和 Windows 11 x64 环境中，以标准当前用户账户执行全新安装、首次启动和日常运行。测量方法：记录安装与运行期间的权限提升请求及任务完成结果。通过判据：两类系统上的安装和运行均成功，管理员凭据或权限提升请求次数为 0。保护目标：确保普通用户能够安全部署和使用应用。
- **NFR35:** 条件：在 Windows 10/11 x64 安装状态下，分别验证托盘、桌面通知、开机启动、关闭隐藏、单实例和显式退出。测量方法：按功能逐项执行正常、重复触发和重启后的操作检查。通过判据：六项能力在两个 Windows 版本上的规定行为通过率均为 100%；重复启动不得产生多个可交互实例，显式退出后不得继续后台运行。保护目标：保证桌面常驻行为符合用户预期。
- **NFR36:** 条件：用户关闭开机启动，随后分别重启应用、重启系统并从既有版本升级到候选构建。测量方法：每次操作后检查开机启动状态并实际登录系统验证。通过判据：全部场景中开机启动保持关闭，应用不得自动恢复该设置。保护目标：尊重用户对系统启动行为的明确选择。
- **NFR37:** 条件：分别在空闲、同步、分析和保存过程中关闭主窗口、执行系统关机及显式退出。测量方法：重新启动应用后核对已完成数据和进行中任务状态。通过判据：已完成数据损坏或丢失数量为 0；每个进行中任务均已安全完成、持久化为可恢复状态或明确标记为可重试；显式退出不得遗留继续运行的应用进程。保护目标：避免窗口和系统生命周期事件造成数据损坏或任务失控。
- **NFR38:** 条件：在分别缺少各项必要运行环境或其版本不满足要求的干净 Windows 10/11 x64 环境中启动安装。测量方法：执行安装并检查检测结果、缺失项说明及用户可采取的补充或修复步骤。通过判据：每个固定缺失场景均在应用无法正常运行前被检测；提示必须准确指出缺失项并提供可执行的补充或修复指引，不得仅显示通用失败信息。保护目标：降低安装失败的排查成本并避免形成不可运行的安装状态。
- **NFR39:** 条件：在 Windows、iPhone、iPad、Android 手机和 Android 平板各执行四条用户旅程及旅程汇总中的核心任务。测量方法：记录每一步完成结果和是否需要转到另一平台。通过判据：适用步骤完成率为 100%，因平台缺少能力而必须转到另一设备的阻断次数为 0；Windows 专属托盘步骤不计入移动端分母。保护目标：保证三端均为可独立验证的 MVP，而非附属查看器。
- **NFR40:** 条件：在移动端同步、分析、收藏、配置保存和通知处理中分别触发后台、挂起、终止、低资源回收、设备重启及网络恢复。测量方法：恢复后核对已完成数据、有效未完成任务及副作用。通过判据：已完成数据损坏或丢失数为 0；有效未完成任务 100% 恢复、重试或明确标记；重复入库、重复分析和重复通知均为 0。保护目标：适应不可控移动生命周期。
- **NFR41:** 条件：通知权限分别处于未请求、允许、拒绝和撤销状态，并分别允许和限制后台刷新。测量方法：执行全部核心非通知任务并检查状态说明、最后同步时间和权限请求次数。通过判据：核心非通知任务完成率为 100%；未经授权的通知提交数为 0；受限原因、最后同步时间和恢复入口展示完整率为 100%；拒绝后再次自动请求系统权限的次数为 0。保护目标：使权限降级可理解且不阻断产品价值。
- **NFR42:** 条件：覆盖手机横竖屏、iPad 分屏或多窗口、Android 窗口宽度变化和至少一种折叠姿态，并在变化前建立搜索、筛选、选中项和未提交输入。测量方法：逐状态检查内容、操作与上下文。通过判据：核心文字与操作可达率为 100%；阻断操作的裁切、重叠和应用内部横向溢出数为 0；搜索、筛选、选中项和未提交输入保留率为 100%。保护目标：保证手机、平板和可变窗口中的任务连续性。
- **NFR43:** 条件：在 iPhone 与 iPad 上分别使用平台屏幕阅读器、最大受支持动态字体、提高对比度和减弱动画完成核心任务。测量方法：检查控件名称、遍历顺序、状态播报、文字裁切及操作结果。通过判据：核心任务完成率为 100%；无名称核心控件、错误遍历顺序、阻断裁切及仅靠颜色表达状态的数量均为 0。保护目标：满足 Apple 平台的无障碍使用需求。
- **NFR44:** 条件：在 Android 手机和平板上分别使用平台屏幕阅读器、系统字体与显示缩放及减弱动画完成核心任务。测量方法：检查控件名称、遍历顺序、焦点、文字裁切和核心触控目标。通过判据：核心任务完成率为 100%；无名称核心控件、错误遍历顺序、不可退出焦点区、阻断裁切及小于 48×48 dp 的核心触控目标数量均为 0。保护目标：满足 Android 平台的无障碍与触控可用性需求。
- **NFR45:** 条件：分别在应用前台、后台和未运行状态打开固定通知样本，并包含目标已删除或不可用样本。测量方法：记录详情到达、重复副作用、错误说明和返回路径。通过判据：有效目标正确详情到达率为 100%；重复详情或重复通知数为 0；无效目标的可操作错误信息完整率为 100%；返回主情报流成功率为 100%。保护目标：保证通知入口可靠且可恢复。
- **NFR46:** 条件：在两台设备分别修改来源、规则、收藏、反馈、正文和凭据，并在其中一台执行删除。测量方法：比较两台设备的数据变化、网络传输与界面作用域说明。通过判据：另一设备未经明确同步能力自动出现变更的次数为 0；跨设备传输上述本地数据的次数为 0；“仅影响当前设备”说明覆盖率为 100%。保护目标：落实 MVP 的设备本地隔离边界。
- **NFR47:** 条件：在每类平台分别删除收藏正文、缓存、配置、反馈和凭据，随后重启并执行搜索、详情、诊断和网络请求。测量方法：检查本地数据、派生索引、安全凭据和可恢复残留。通过判据：用户明确要求删除的数据残留数为 0；被删除凭据再次用于请求的次数为 0；允许保留的数据必须在确认前逐项说明。保护目标：落实用户对当前设备数据的删除控制。
- **NFR48:** 条件：移动候选构建在无待处理任务的后台或挂起状态持续 30 分钟。测量方法：记录应用主动网络请求、周期唤醒、持续执行和平台能耗诊断。通过判据：主动网络请求数为 0，产品自行发起的周期唤醒数为 0，不得持续占用后台执行时间；系统授予的明确后台任务除外且必须在任务完成后结束。保护目标：避免不必要的电量、网络和后台资源消耗。
- **NFR49:** 条件：移动基线设备加载固定 50,000 条数据集并完成主情报流、搜索、详情、收藏和设置操作，随后重复进入后台与前台 20 次。测量方法：记录系统内存警告、非预期终止、操作时延和内存趋势。通过判据：非预期终止数为 0；核心操作时延满足 NFR2 与 NFR3；完成每轮相同操作后的内存不得连续 5 轮增长且总增幅不得超过首轮稳定值的 20%。保护目标：控制移动端内存压力与生命周期泄漏。
- **NFR50:** 条件：从每个平台全部声明支持的既有版本升级至候选构建，并分别执行保留应用数据的卸载或更新路径。测量方法：升级前后比较配置、收藏、反馈、历史情报、权限状态及用户告知。通过判据：静默丢失或错误改写记录数为 0；无法迁移时升级被安全停止；安装、升级、卸载或清除数据前的数据保留后果说明覆盖率为 100%。保护目标：保证三端版本演进和本地数据后果可预期。
- **NFR51:** 条件：分别准备签名有效、签名缺失或失效、安装资格有效和安装资格无效的移动候选构建，并检查其隐私与第三方 AI 数据处理说明。测量方法：在受控测试分发环境中验证安装结果、授权对象和全部披露字段。通过判据：签名有效且安装资格有效的候选构建安装成功率为 100%；其他候选构建进入测试分发的次数为 0；设备本地数据、系统权限、公开来源、第三方 AI 外发数据、正文保留与删除、诊断范围及无云备份或跨设备同步七类说明的完整率为 100%，声明与实际行为不一致项为 0。保护目标：保证受控移动测试分发可安装、可追责且隐私边界透明。
- **NFR52:** 条件：在每类受支持设备上，为 RSS/Atom、GitHub Release 和 arXiv 分别使用包含新增、更新、跳过、解析失败和零结果的固定响应集，在未配置 AI Key 时执行单一来源同步与全部来源同步。测量方法：比较来源执行记录、同步结果摘要、最小结果列表和错误归属。通过判据：新增、更新、跳过与失败数量和执行记录一致率为 100%；成功转换记录的来源类型、发布方、原始标题、可获得的发布时间、采集时间和原文链接展示完整率为 100%；失败来源归属正确率为 100%；零结果与部分成功状态误报数为 0；任一来源失败导致其他来源成功结果被隐藏或回滚的数量为 0。保护目标：确保三类来源在不依赖 AI、评分、搜索或完整详情时即可形成可消费、可核验的独立交付结果。
- **NFR53:** 条件：在每类受支持设备上建立“未查看”“已判断”“待研究”与收藏、未收藏构成的六种组合，执行三态互转、列表浏览、详情打开、收藏、取消收藏、正文删除、离线恢复、应用重启、移动挂起或进程回收及从每个受支持既有版本升级。测量方法：比较每次操作前后的用户处理状态、收藏状态及持久化记录。通过判据：显式状态转换成功率为 100%；仅浏览或打开详情导致用户处理状态变化的次数为 0；收藏、取消收藏或正文删除导致用户处理状态变化的次数为 0；重启、恢复和升级后的六种组合保留率为 100%；用户处理状态与 AI 处理状态相互覆盖或混用的数量为 0。保护目标：保证用户处理进度可控、可撤销、可持久化，并与收藏及 AI 生命周期保持独立。
- **NFR54:** 条件：在每类受支持设备上，使用统一测量基线中的固定 GitHub 仓库响应集和带时间戳的 Star/Fork 观测快照执行手动固定关注、多个发现订阅、自动纳入 Release 监控、忽略、停用、转固定关注和 Release 同步，并分别注入发现请求超时、限流、无效响应和部分分页失败。测量方法：比较固定预期集与发现结果、首次发现或首次跨门槛分类、命中依据、项目状态、规范仓库身份、有效 Release 监控对象、Release 记录、通知记录、错误归属和退避时间。通过判据：符合条件项目的发现结果与固定预期集一致率为 100%，不符合条件项目进入结果或自动监控的数量为 0；“新发现项目”与“新符合条件项目”分类正确率为 100%；首次观测、不足两个有效观测值或零基数比例增长命中“GitHub 增长条件”的数量为 0；每个结果的命中条件、实际观测值和观测时间展示完整率为 100%；同一“GitHub 仓库规范身份”的项目对象和有效 Release 监控对象均最多为 1，因手动与自动重叠、多个订阅重叠、仓库重命名或转移产生的重复 Release 记录和重复通知均为 0；忽略、停用和转固定关注后的状态及规定行为一致率为 100%，非用户主动删除造成的历史数据丢失数量为 0；发现失败必须仅归属于对应发现订阅并遵守 NFR24 的退避规则，既有项目 Release 同步被阻断、成功结果被隐藏或回滚的数量均为 0，部分分页失败或限流被误报为成功且零结果的数量为 0。保护目标：保证 GitHub 项目发现准确、可解释、可控、无重复，并将发现服务的限流和故障与既有 Release 监控隔离。

**Total NFRs: 54**

### Additional Requirements

- Phase 1 is a device-local, cross-platform native MVP for Windows, iPhone/iPad, Android phone/tablet; it does not provide accounts, cloud backup, cross-device synchronization, remote push, public mobile-store release, or multi-user collaboration.
- MVP source delivery is gated on real RSS/Atom, GitHub Release, and arXiv integrations; demonstration data cannot substitute for a real source or contribute to validation metrics.
- AI analysis is optional, third-party, explicitly authorized per device/source, and must remain separable from original facts and deterministic rule judgments.
- Content acquisition, retention, excerpts, full-text snapshots, deletion, diagnostics, provenance, and external-link handling must observe the PRD's copyright, privacy, SSRF, and device-local boundaries.
- GitHub project discovery covers public repositories only and is distinct from fixed GitHub Release monitoring; it supports subscription create/update/enable/disable, not subscription deletion.
- The PRD leaves automatic-monitoring capacity and overflow behavior undecided. Implementation must not invent unlimited monitoring, silent eviction, or an unapproved capacity limit.
- Windows production distribution requires production code signing; controlled mobile testing retains its platform signing, installation-eligibility, disclosure, and restricted-distribution gates.
- Validation uses the unified fixed data sets, clocks, platform matrix, lifecycle matrix, and failure injections defined in the PRD; live external-service results are not authoritative acceptance evidence.

### PRD Completeness Assessment

The PRD is structurally complete and implementation-usable. It contains 64 contiguous functional requirements and 54 contiguous non-functional requirements with explicit conditions, measurement methods, pass criteria, and protection goals. Product scope, user journeys, terminology, platform boundaries, failure modes, validation baselines, and FR/NFR traceability are present. No missing numbered requirement or unresolved critical PRD gap was found. The sole intentional product decision deferred beyond the current contract is the automatic-monitoring capacity/overflow policy, which is explicitly prohibited from being invented during implementation.


## Epic Coverage Validation

### Coverage Matrix

| FR | PRD requirement | Epic and story coverage | Status |
|---|---|---|---|
| FR1 | 用户无需注册或配置 AI 服务即可首次使用产品。 | Epic 1; Story 1.1：启动使用共享核心的三端原生应用; Story 1.2：浏览安全隔离的演示情报; Story 1.4：使用默认赛道并按自己的节奏完成配置引导 | ✓ Covered |
| FR2 | 用户首次启动时可以浏览按“演示数据”口径明确标识的示例情报，且这些记录不得触发真实通知或计入验证指标。 | Epic 1; Story 1.2：浏览安全隔离的演示情报; Story 1.3：浏览可访问的演示情报列表与证据详情; Story 7.1：记录可解释的情报价值与分析反馈 | ✓ Covered |
| FR3 | 用户首次启动时可以使用预置的 AI 技术赛道和默认来源。 | Epic 1; Story 1.4：使用默认赛道并按自己的节奏完成配置引导 | ✓ Covered |
| FR4 | 用户可以添加、修改、启用或停用关注赛道。 | Epic 2; Story 2.1：管理并安全校验当前设备的关注配置; Story 2.6：查看本轮同步的最小可消费结果 | ✓ Covered |
| FR5 | 用户可以配置关键词、排除词、关注来源和刷新周期。 | Epic 2; Story 2.1：管理并安全校验当前设备的关注配置 | ✓ Covered |
| FR6 | 用户可以配置来源可信度、提醒阈值、免打扰时段和提醒频率上限。 | Epic 2; Story 2.1：管理并安全校验当前设备的关注配置 | ✓ Covered |
| FR7 | 用户可以订阅公开的 RSS/Atom 信息源。 | Epic 2; Story 2.2：订阅并增量同步 RSS/Atom 来源 | ✓ Covered |
| FR8 | 用户可以监控指定 GitHub 项目的 Release 更新。 | Epic 2; Story 2.3：监控 GitHub Release 更新; Story 3.1：创建并管理 GitHub 项目发现订阅; Story 4.1：将真实来源规范化为可溯源情报 | ✓ Covered |
| FR9 | 用户可以监控指定主题、关键词或分类的 arXiv 论文。 | Epic 2; Story 2.4：监控 arXiv 主题与分类 | ✓ Covered |
| FR10 | 系统可以在每台受支持设备上按照该设备的来源配置执行增量同步；移动端在系统不允许后台执行时，可以在下次获得执行机会或用户前台触发后继续。 | Epic 2; Story 2.2：订阅并增量同步 RSS/Atom 来源; Story 2.3：监控 GitHub Release 更新; Story 2.4：监控 arXiv 主题与分类; Story 2.5：统一执行同步并判断三来源交付就绪 | ✓ Covered |
| FR11 | 用户可以在当前设备手动触发全部来源或单个来源同步。 | Epic 2; Story 2.5：统一执行同步并判断三来源交付就绪 | ✓ Covered |
| FR12 | 系统可以分别显示当前设备各来源的同步进度、最后成功时间、失败状态、待处理任务、数据新鲜度及后台受限状态，并提供最近一次同步的新增、更新、跳过与失败数量及同步结果入口。 | Epic 2; Story 2.3：监控 GitHub Release 更新; Story 2.4：监控 arXiv 主题与分类; Story 2.5：统一执行同步并判断三来源交付就绪; Story 2.6：查看本轮同步的最小可消费结果 | ✓ Covered |
| FR13 | 系统可以将不同来源内容转换为统一的情报记录。 | Epic 4; Story 4.1：将真实来源规范化为可溯源情报; Story 4.8：收藏正文、离线阅读并彻底删除 | ✓ Covered |
| FR14 | 系统必须为每条情报保留来源类型、发布方、原始标题、有效原文链接、发布时间、采集时间和内容标识；来源能够提供作者时还必须保留作者，不能提供时须明确记录为不可获得。 | Epic 4; Story 4.1：将真实来源规范化为可溯源情报; Story 4.6：查看证据详情并打开原文核验 | ✓ Covered |
| FR15 | 用户可以查看情报首次发现时间、最后更新时间和原文可用状态。 | Epic 4; Story 4.1：将真实来源规范化为可溯源情报; Story 4.6：查看证据详情并打开原文核验 | ✓ Covered |
| FR16 | 系统可以区分原始事实、规则判断与 AI 分析。 | Epic 4; Story 4.1：将真实来源规范化为可溯源情报; Story 4.3：用透明规则判断价值并形成主流分流; Story 4.6：查看证据详情并打开原文核验; Story 5.3：生成并验证结构化中文情报分析 | ✓ Covered |
| FR17 | 对于未收藏情报，系统仅可长期保存元数据、原文链接、符合“必要摘录”口径的片段和 AI 分析，不得持久化完整正文或可实质还原全文的内容。 | Epic 4; Story 4.8：收藏正文、离线阅读并彻底删除; Story 5.3：生成并验证结构化中文情报分析 | ✓ Covered |
| FR18 | 用户收藏情报时，可以保存合法获取的完整正文快照；取消收藏后可以删除该正文及其派生索引。 | Epic 4; Story 4.8：收藏正文、离线阅读并彻底删除 | ✓ Covered |
| FR19 | 系统可以依据赛道匹配、来源可信度、新鲜度、技术影响和用户配置规则形成“情报价值”判断，且用户能够查看影响该判断的因素。 | Epic 4; Story 4.3：用透明规则判断价值并形成主流分流 | ✓ Covered |
| FR20 | 系统可以为情报给出“重要程度”，并展示促成该结果的赛道、来源、新鲜度、技术影响、规则命中及适用的 AI 判断依据。 | Epic 4; Story 4.3：用透明规则判断价值并形成主流分流; Story 4.6：查看证据详情并打开原文核验 | ✓ Covered |
| FR21 | 系统可以按照用户当前主情报流阈值将“高价值情报”与“普通候选”区分，阈值变化不得删除或改写原始情报记录。 | Epic 4; Story 4.3：用透明规则判断价值并形成主流分流 | ✓ Covered |
| FR22 | 用户可以查看被过滤的内容及其过滤原因。 | Epic 4; Story 4.3：用透明规则判断价值并形成主流分流 | ✓ Covered |
| FR23 | Phase 1 中，系统可以依据“确定性重复或关联”口径识别重复内容；不得仅凭语义相似自动合并记录。 | Epic 4; Story 4.2：确定性识别重复并关联同一事件来源 | ✓ Covered |
| FR24 | Phase 1 中，系统可以依据上述确定性共享标识关联至少 2 个指向同一事件的来源记录，并保留每个来源各自的标题、链接、发布时间、发布方及其他溯源记录供用户展开核验。 | Epic 4; Story 4.2：确定性识别重复并关联同一事件来源; Story 4.6：查看证据详情并打开原文核验; Story 4.8：收藏正文、离线阅读并彻底删除 | ✓ Covered |
| FR25 | 用户可以在当前设备配置、更新或移除兼容 OpenAI 接口规范的第三方 AI 服务凭据；配置不得自动传播到其他设备。 | Epic 5; Story 5.1：在平台安全存储中管理 AI 凭据; Story 5.4：明确展示 AI 状态并在失败时可信降级 | ✓ Covered |
| FR26 | 系统可以在每台设备首次向第三方 AI 服务发送内容前说明将发送的数据，并取得用户确认。 | Epic 5; Story 5.2：在首次外发前确认数据范围并按来源授权 | ✓ Covered |
| FR27 | 系统可以默认对新情报执行中文翻译、摘要、主题提取、重要度判断、置信度生成和理由生成，并在情报详情中展示置信度。 | Epic 5; Story 5.3：生成并验证结构化中文情报分析; Story 5.4：明确展示 AI 状态并在失败时可信降级 | ✓ Covered |
| FR28 | 用户可以在当前设备按来源决定是否允许向第三方 AI 服务发送内容，并可随时撤销授权。 | Epic 5; Story 5.2：在首次外发前确认数据范围并按来源授权 | ✓ Covered |
| FR29 | 系统必须按“明确标识 AI 内容”口径区分所有 AI 生成字段与原始事实、规则判断，并显示每条情报当前的 AI 处理状态。 | Epic 5; Story 5.4：明确展示 AI 状态并在失败时可信降级 | ✓ Covered |
| FR30 | AI 服务不可用、输出无效或用户未配置有效凭据时，系统仍可以保留并展示原始情报，供稍后重试。 | Epic 5; Story 2.6：查看本轮同步的最小可消费结果; Story 5.4：明确展示 AI 状态并在失败时可信降级 | ✓ Covered |
| FR31 | 用户可以浏览经过筛选和排序的主情报流。 | Epic 4; Story 4.5：浏览高价值主情报流并组合筛选; Story 4.8：收藏正文、离线阅读并彻底删除 | ✓ Covered |
| FR32 | 用户可以按赛道、来源、时间、重要度、用户处理状态和收藏状态筛选情报；用户处理状态与 AI 处理状态必须使用不同名称和筛选语义。 | Epic 4; Story 4.4：显式管理每条情报的处理状态; Story 4.5：浏览高价值主情报流并组合筛选; Story 4.7：搜索允许持久化的本地情报内容 | ✓ Covered |
| FR33 | 用户可以搜索已保存的情报元数据、符合“必要摘录”口径的片段和当前可用的收藏正文；搜索结果必须遵守内容生命周期边界，不得暴露已删除正文或未获准持久化的全文内容。 | Epic 4; Story 4.7：搜索允许持久化的本地情报内容 | ✓ Covered |
| FR34 | 用户可以查看单条情报的内容提炼、评分依据、溯源信息和关联来源。 | Epic 4; Story 4.6：查看证据详情并打开原文核验; Story 5.4：明确展示 AI 状态并在失败时可信降级 | ✓ Covered |
| FR35 | 用户可以从情报详情打开原始内容进行核验。 | Epic 4; Story 4.6：查看证据详情并打开原文核验 | ✓ Covered |
| FR36 | 用户可以收藏、取消收藏并离线阅读已保存的正文快照；收藏变化及正文删除不得隐式改变用户处理状态。 | Epic 4; Story 4.4：显式管理每条情报的处理状态; Story 4.8：收藏正文、离线阅读并彻底删除 | ✓ Covered |
| FR37 | 当前设备只有在本地完成情报判定，且该情报同时满足“重大动态”口径、完整溯源要求、用户提醒阈值和当前设备通知授权时，才可以向操作系统提交通知；条件不满足时不得以重大动态名义通知。MVP 不依赖远程推送生成该通知。 | Epic 6; Story 6.1：仅为符合门槛的重大动态生成本地通知效果; Story 6.4：通过 Windows 托盘安全管理后台运行 | ✓ Covered |
| FR38 | 用户点击通知后可以直接打开对应情报详情。 | Epic 6; Story 6.3：从通知安全直达证据详情并处理无效目标 | ✓ Covered |
| FR39 | 系统可以在当前设备内防止同一情报或内容版本产生重复通知；MVP 不承诺跨设备通知去重。 | Epic 6; Story 6.1：仅为符合门槛的重大动态生成本地通知效果 | ✓ Covered |
| FR40 | 系统可以遵守用户设置的免打扰时段、通知频率上限和操作系统通知授权；权限不足时须显示提醒不可送达状态。 | Epic 6; Story 6.2：渐进请求通知权限并遵守提醒治理 | ✓ Covered |
| FR41 | 在 Windows 上，用户关闭主窗口后，系统可以继续在托盘运行和同步。 | Epic 6; Story 6.4：通过 Windows 托盘安全管理后台运行 | ✓ Covered |
| FR42 | 在 Windows 上，用户可以从托盘显示主窗口、立即同步或显式退出应用，并可以控制开机自动启动。 | Epic 6; Story 6.4：通过 Windows 托盘安全管理后台运行 | ✓ Covered |
| FR43 | 用户可以将情报标记为“值得立即知道”“有价值但不紧急”“无价值”“重复”或“分析错误”。 | Epic 7; Story 4.4：显式管理每条情报的处理状态; Story 7.1：记录可解释的情报价值与分析反馈; Story 7.4：从透明证据手动校准筛选规则 | ✓ Covered |
| FR44 | 用户可以记录已确认的漏报及其原因。 | Epic 7; Story 7.2：登记已确认漏报及其原因 | ✓ Covered |
| FR45 | 系统可以汇总提醒有效率、主情报流有效率、漏报和每日使用情况，支持四周个人验证。 | Epic 7; Story 7.3：汇总四周个人验证指标; Story 7.4：从透明证据手动校准筛选规则 | ✓ Covered |
| FR46 | 系统可以按赛道、来源、规则命中和筛选结果汇总用户反馈，供用户识别误报与漏报原因并手动校准“可解释”的筛选规则；Phase 1 不得据此自动修改权重或规则。 | Epic 7; Story 7.1：记录可解释的情报价值与分析反馈; Story 7.4：从透明证据手动校准筛选规则 | ✓ Covered |
| FR47 | 当前设备的单个来源失败时，系统可以继续处理其他来源并保留已成功结果。 | Epic 8; Story 2.3：监控 GitHub Release 更新; Story 2.5：统一执行同步并判断三来源交付就绪; Story 2.6：查看本轮同步的最小可消费结果; Story 8.1：在单源故障中保留部分成功结果; Story 8.5：查看全平台状态并导出安全诊断 | ✓ Covered |
| FR48 | 用户可以在当前设备单独重试失败来源或待处理的 AI 分析任务；来源处于强制退避期间时须说明暂不可重试的原因和时间。 | Epic 8; Story 8.2：按退避规则独立重试来源和 AI 任务 | ✓ Covered |
| FR49 | 离线、后台挂起或应用恢复后，用户仍可以浏览、搜索、筛选、收藏、配置和反馈；系统再次获得联网与执行机会后可以恢复有效待处理任务。 | Epic 8; Story 2.6：查看本轮同步的最小可消费结果; Story 8.3：在离线模式继续使用本地资料和配置 | ✓ Covered |
| FR50 | 用户可以查看按“可操作错误信息”口径呈现的来源级状态，并通过当前平台提供的复制或分享能力导出不含 API Key、完整正文、敏感提示内容或其他凭据的脱敏诊断信息。 | Epic 8; Story 8.5：查看全平台状态并导出安全诊断 | ✓ Covered |
| FR51 | 在 iOS/iPadOS 与 Android 上，系统可以保存应用挂起、终止或进程被回收前已经完成的数据和可恢复任务状态；再次获得执行机会后，可以继续处理可恢复任务，或将无法继续的任务明确标记为可重试。 | Epic 8; Story 8.4：在移动生命周期中安全恢复持久任务 | ✓ Covered |
| FR52 | 用户可以在所有平台查看最后成功同步时间、当前同步状态和影响时效性的后台限制，并可在前台手动触发同步。 | Epic 8; Story 2.5：统一执行同步并判断三来源交付就绪; Story 8.5：查看全平台状态并导出安全诊断 | ✓ Covered |
| FR53 | 系统只能在需要发送重大动态通知时请求通知权限；用户拒绝或撤销后，系统不得阻断采集、浏览、检索、溯源、反馈或前台同步，也不得在每次启动时重复强迫授权。 | Epic 6; Story 6.2：渐进请求通知权限并遵守提醒治理; Story 6.4：通过 Windows 托盘安全管理后台运行 | ✓ Covered |
| FR54 | 用户从系统通知进入应用时，无论应用正在前台、后台或尚未运行，系统都可以打开对应情报详情；目标已删除或不可用时必须说明原因并提供返回主情报流的入口。 | Epic 6; Story 6.3：从通知安全直达证据详情并处理无效目标; Story 6.4：通过 Windows 托盘安全管理后台运行 | ✓ Covered |
| FR55 | 用户可以在 Windows、iPhone、iPad、Android 手机和 Android 平板上完成首启、主情报流浏览、搜索、详情、收藏、溯源、设置、反馈和同步状态查看。 | Epic 9; Story 9.1：在五类设备使用平台原生导航完成核心旅程; Story 9.5：通过移动受控测试分发与隐私披露门禁 | ✓ Covered |
| FR56 | 系统可以根据桌面、手机、平板、窗口宽度、横竖屏和输入方式调整导航与布局，同时保持功能含义、筛选状态和当前任务上下文一致。 | Epic 9; Story 9.1：在五类设备使用平台原生导航完成核心旅程; Story 9.2：在方向、分屏和窗口变化中保持任务上下文 | ✓ Covered |
| FR57 | 系统必须将配置、收藏、反馈、凭据和历史情报保存在当前设备的数据边界内，并在首次使用与设置中明确说明 MVP 不提供云端备份或跨设备同步。 | Epic 9; Story 5.1：在平台安全存储中管理 AI 凭据; Story 7.3：汇总四周个人验证指标; Story 9.3：明确当前设备数据边界并安全执行删除 | ✓ Covered |
| FR58 | 用户可以在卸载、清除应用数据或执行可能删除本地数据的操作前看到数据保留后果；产品不得暗示未配置的恢复或迁移能力。 | Epic 9; Story 9.3：明确当前设备数据边界并安全执行删除; Story 9.4：保护安装、升级和数据迁移路径; Story 9.5：通过移动受控测试分发与隐私披露门禁 | ✓ Covered |
| FR59 | 用户在当前设备完成全部来源或单个来源同步后，可以分别查看 RSS/Atom、GitHub Release 和 arXiv 本次同步的新增、更新、跳过与失败数量，以及本次成功转换的最小结果列表。每项至少展示来源类型、发布方、原始标题、发布时间（来源可提供时）、采集时间和原文链接。该结果不得依赖 AI 凭据、评分、搜索或完整详情；零结果和部分失败须明确呈现，且部分失败不得隐藏已成功结果。 | Epic 2; Story 2.6：查看本轮同步的最小可消费结果 | ✓ Covered |
| FR60 | 用户可以在当前设备将每条情报显式设置为“未查看”“已判断”或“待研究”，并在三种状态间转换；新入库情报默认为“未查看”，仅浏览列表或打开详情不得自动改变状态。状态须在应用重启、离线、平台生命周期恢复和受支持版本升级后保留。收藏是独立维度，任一用户处理状态均可收藏或不收藏，收藏变化及正文删除不得隐式改变处理状态。 | Epic 4; Story 4.4：显式管理每条情报的处理状态; Story 4.8：收藏正文、离线阅读并彻底删除 | ✓ Covered |
| FR61 | 系统可以在用户已能浏览主情报流后提供分步配置引导；所有可延后步骤均可跳过，跳过或退出不得阻断主情报流、同步、搜索和详情查看。用户可以从“设置”根级页面中名称为“配置引导”的固定入口继续未完成步骤；从主情报流到达该入口不得超过两次用户发起的导航操作，且已保存的配置不得丢失。 | Epic 1; Story 1.4：使用默认赛道并按自己的节奏完成配置引导 | ✓ Covered |
| FR62 | 系统在保存赛道、关键词、排除词、来源或阈值配置前，必须按照验收术语中的判定口径区分阻断性无效配置与“过窄配置风险”。阻断性无效配置不得保存，并须指出受影响字段、所属无效类别和修正方式；检测到过窄配置风险时，须指出受影响条件和可能漏报的后果，并允许用户返回修改或明确确认后保存；未检测到风险时不得要求额外确认。 | Epic 2; Story 2.1：管理并安全校验当前设备的关注配置; Story 2.6：查看本轮同步的最小可消费结果; Story 3.1：创建并管理 GitHub 项目发现订阅; Story 7.4：从透明证据手动校准筛选规则 | ✓ Covered |
| FR63 | 在 Windows 同一用户会话中，系统只允许一个可交互应用实例；已有实例运行时，包括主窗口已隐藏至托盘的状态，再次启动必须显示并聚焦现有主窗口，不得创建第二个可交互实例。 | Epic 6; Story 6.4：通过 Windows 托盘安全管理后台运行 | ✓ Covered |
| FR64 | 用户可以在当前设备保留通过仓库标识手动建立的 GitHub Release 固定关注，并可以创建、修改、启用或停用 GitHub 项目发现订阅。发现订阅可以按关注赛道、主要语言、GitHub Topic、Star/Fork 当前门槛及“GitHub 增长条件”发现公开仓库；系统必须按“新发现项目”和“新符合条件项目”口径展示结果，为每个项目显示“GitHub 发现命中依据”，并将符合条件的项目自动纳入 Release 监控。用户可以忽略发现项目、停用其自动关注或将其转为固定关注。系统必须按“GitHub 仓库规范身份”合并手动关注、多个发现订阅及仓库重命名或转移产生的同一项目，固定关注优先于自动关注；同一项目在当前设备只能形成一个项目对象和一个有效 Release 监控对象，不得产生重复 Release 记录或重复通知。发现订阅停用、条件失配或项目被忽略不得删除已经保存的 Release、反馈、收藏或其他合法保留的历史数据；发现任务失败或受限不得阻断既有项目的 Release 同步。 | Epic 3; Story 3.1：创建并管理 GitHub 项目发现订阅; Story 3.2：发现符合当前条件的公开 GitHub 项目; Story 3.3：用有效观测识别增长与首次跨越门槛; Story 3.4：合并手动关注、多订阅及仓库身份变化; Story 3.5：控制自动关注并保留其他有效关注; Story 3.6：在订阅和条件变化后保留合法历史; Story 3.7：在限流和部分失败中继续发现与同步 Release; Story 4.1：将真实来源规范化为可溯源情报; Story 6.1：仅为符合门槛的重大动态生成本地通知效果; Story 7.4：从透明证据手动校准筛选规则; Story 8.1：在单源故障中保留部分成功结果; Story 8.2：按退避规则独立重试来源和 AI 任务; Story 8.4：在移动生命周期中安全恢复持久任务; Story 8.5：查看全平台状态并导出安全诊断; Story 9.1：在五类设备使用平台原生导航完成核心旅程; Story 9.3：明确当前设备数据边界并安全执行删除; Story 9.4：保护安装、升级和数据迁移路径 | ✓ Covered |

### Missing Requirements

None. Every PRD functional requirement is present in the formal FR coverage map and referenced by at least one detailed Story acceptance criterion.

### Coverage Statistics

- Total PRD FRs: 64.
- FRs mapped to an Epic: 64.
- FRs referenced by detailed Story acceptance criteria: 64.
- Missing FRs: 0.
- Coverage: 100.0%.
- Extra FR identifiers not present in PRD: 0.


## UX Alignment Assessment

### UX Document Status

Found and implementation-grade. The assessment uses the complete UX specification plus the paired visual and behavioral spines:

- `_agentic-out/planning/ux-design-specification.md`
- `_agentic-out/planning/ux/DESIGN.md`
- `_agentic-out/planning/ux/EXPERIENCE.md`

### UX ↔ PRD Alignment

- The four PRD journeys are represented by explicit UX flows for onboarding, daily intelligence review, notification-to-evidence decisions, calibration, failure recovery, synchronization consumption, processing-state management, Windows repeated launch, and GitHub project discovery.
- FR59–FR63 are represented by `SyncResultSummary`, `ProcessingStateControl`, `ProgressiveSetupGuide`, the two-channel configuration validation flow, and Windows activation states.
- FR64/NFR54 are represented by distinct fixed-follow and discovery information architecture, subscription create/update/enable/disable, condition composition, baseline/growth classifications, evidence disclosure, canonical repository identity, monitoring controls, history retention, discovery outcomes, and Release isolation.
- Device-local scope, no cloud backup/cross-device synchronization, optional third-party AI authorization, content lifecycle, notification permission timing, and controlled distribution disclosures match the PRD.
- UX does not introduce subscription deletion, private-repository access, GitHub login, or an automatic-monitoring capacity promise.

### UX ↔ Architecture Alignment

- Shared semantic state with platform-native Windows, Apple, and Android implementations matches the architecture's Rust-core authority and platform feature boundaries.
- UX subscription drafts remain platform-local transient state; saved subscriptions, discovery results, classifications, evidence, and monitoring state come from core commands and queries.
- Discovery status and GitHub Release status are orthogonal in UX and use independent tasks, projections, query invalidation, errors, retry, and diagnostics in Architecture.
- Canonical repository identity, growth observation rules, fixed-over-automatic precedence, non-optimistic monitoring controls, retained history, pagination, rate-limit handling, and fixed fixtures are supported by explicit Architecture decisions.
- Responsiveness, 50,000-record performance, lifecycle recovery, accessibility, responsive layouts, focus restoration, and non-color state semantics have corresponding architectural patterns and Story acceptance criteria.
- UX-DR1–UX-DR37 are each referenced by at least one detailed Story; no orphan implementation-facing UX requirement was found.

### Alignment Issues

None blocking or material. The previous subscription-deletion scope drift was removed from UX and Architecture before this assessment.

### Warnings and Advisories

- The automatic-monitoring capacity and overflow experience intentionally remains undefined because the PRD has not approved a capacity policy. UX and Architecture correctly prohibit implementation from inventing a limit, silent eviction, or an “unlimited” promise. This is a deferred product decision, not a current readiness blocker.
- Older narrative portions of Architecture may mention the pre-increment FR/NFR totals; the dated FR64/NFR54 increment and validation section explicitly supersede those counts. Current authority is FR1–FR64 and NFR1–NFR54.

### UX Alignment Result

**PASS.** UX, PRD, Architecture, and Epics/Stories are aligned with zero blocking UX gaps.


## Epic and Story Quality Review

### Epic Structure

| Epic | User-value focus | Independence and dependency flow | Result |
|---|---|---|---|
| Epic 1 — Immediate local-radar experience | Users can launch and evaluate the product without registration, cloud, AI, or notifications. | Standalone foundation; later Epics may build on it. | Pass |
| Epic 2 — Configure and synchronize three real sources | Users obtain real RSS/Atom, fixed GitHub Release, and arXiv results without AI. | Uses Epic 1 only; does not require discovery or later intelligence processing. | Pass |
| Epic 3 — Conditional GitHub project discovery | Users create subscriptions, review evidence, and control automatic monitoring. | Uses Epic 2 fixed Release monitoring; complete discovery/control/failure loop without later Epics. | Pass |
| Epic 4 — Explainable daily intelligence and offline library | Users normalize, filter, inspect, search, save, and track intelligence. | Uses previously acquired source records; processing state was moved before feed filtering to remove a forward dependency. | Pass |
| Epic 5 — Optional AI analysis | Users add explicitly authorized AI assistance while retaining non-AI usability. | Uses Epic 4 facts; no later Epic dependency. | Pass |
| Epic 6 — Controlled local notifications | Users receive traceable, deduplicated notifications and Windows tray lifecycle behavior. | Uses Epic 4; AI is optional. | Pass |
| Epic 7 — Feedback and calibration | Users measure and manually calibrate quality without automatic rule mutation. | Uses prior intelligence evidence; no later dependency. | Pass |
| Epic 8 — Offline, failure, and lifecycle continuity | Users continue working and diagnose failures. | Builds on existing tasks/data; provides a complete resilience outcome. | Pass |
| Epic 9 — Cross-platform, safe delivery | Users complete core journeys on every target device with safe migration and controlled distribution. | Integrates Epics 1–8; no future Epic dependency. | Pass |

No Epic is organized as a database, API, infrastructure, or other purely technical milestone.

### Story Structure and Acceptance Criteria

- Detailed Stories reviewed: 47/47.
- Stories with exactly one “As a / I want / So that” statement: 47/47.
- Stories with at least one complete Given/When/Then acceptance scenario: 47/47.
- Stories with mismatched Given/When/Then scenario counts: 0.
- Stories with missing FR traceability: 0.
- Explicit or implicit forward dependencies found after reordering Story 4.4: 0.
- Database/entity timing: Pass. Story 1.1 establishes only the minimum workspace and cross-language contract; source, discovery, processing-state, sync-result, notification, content, and migration persistence are introduced with the first Story that needs each capability rather than through an up-front all-table Story.
- Starter-template placement: Pass. Architecture requires a composed official scaffold rather than a repository clone; Story 1.1 owns Cargo/Tauri/SwiftUI/Compose initialization, locked dependencies, generated bindings, minimal platform adapters, and initial quality smoke checks.

### Findings by Severity

#### Critical violations

None.

#### Major issues

1. **Story 1.1 is oversized for the workflow's single-agent completion rule.** It combines a Cargo workspace, Tauri/React Windows shell, SwiftUI Apple shell, Compose Android shell, UniFFI generation, DTO/AppError round trips, PlatformEffect outbox behavior, temporary secret leases, dependency locking, and cross-platform CI smoke checks. The work is coherent and correctly placed first, but it is materially larger than the other Stories and carries several independently failing toolchains.
   - **Impact:** A single implementation run can finish only part of the acceptance contract while the Story still appears “in progress,” reducing review clarity and increasing integration risk.
   - **Recommendation:** Before sprint planning, split it into a thin shared-core/contract bootstrap followed by platform-shell slices and a generated-binding/CI integration slice, while preserving the rule that each slice depends only on previous slices and that Epic 1 remains usable after the platform target(s) declared for that slice.

2. **Story 9.5 is oversized and mixes delivery ownership across Apple, Android, and the full five-device NFR54 release gate.** It includes TestFlight signing/distribution, Google Play Internal Testing signing/distribution, disclosure validation, behavior-versus-declaration checks, migration/task/privacy proof, and GitHub-discovery fixed-fixture execution on five device classes.
   - **Impact:** Distinct protected environments, credentials, platform release systems, and test matrices make one-agent completion and evidence review impractical.
   - **Recommendation:** Split into Apple controlled distribution, Android controlled distribution, and cross-platform release-evidence aggregation. Keep secrets unavailable to ordinary/PR builds and make the aggregation slice depend only on the completed platform slices.

#### Minor concerns

1. Several Stories contain seven or eight acceptance scenarios (notably 2.1, 6.4, 7.4, 8.5). Their scopes remain cohesive and independently testable, but sprint planning should avoid combining them with unrelated setup work and should create explicit implementation/test tasks beneath each Story.
2. Story 9.1 is an end-to-end cross-platform equivalence Story. It is valid as an integration/acceptance Story because feature implementation is owned by earlier Epics, but its execution evidence should be partitioned by platform and then aggregated to avoid an opaque all-or-nothing test run.

### Best-Practice Compliance Result

**CONDITIONAL PASS.** Requirements coverage, user-value Epic structure, dependency direction, BDD completeness, architecture alignment, and database timing pass. Two Major sizing issues must be resolved before Sprint Planning if the project intends to enforce “one Story = one independently completable development-agent assignment.” They do not indicate missing product behavior, but they do affect implementation execution readiness.


## Summary and Recommendations

### Overall Readiness Status

**NEEDS WORK**

The product and solution definition is complete, aligned, and fully traceable, but the delivery plan does not yet meet the workflow's single-development-agent Story sizing rule. Implementation should not enter Sprint Planning with Story 1.1 and Story 9.5 in their current form.

### Evidence Summary

| Assessment area | Result |
|---|---|
| Required document discovery | Pass — 4/4 types found; no duplicate formats |
| PRD completeness | Pass — FR1–FR64 and NFR1–NFR54 contiguous and complete |
| Functional requirement coverage | Pass — 64/64 mapped and referenced by detailed Story ACs |
| UX ↔ PRD ↔ Architecture alignment | Pass — zero blocking gaps; UX-DR1–UX-DR37 owned by Stories |
| Epic user-value structure | Pass — 9/9 user-outcome Epics |
| Epic and Story dependency direction | Pass — zero forward dependencies after Story 4.4 reorder |
| Story BDD and traceability structure | Pass — 47/47 complete |
| Architecture requirement ownership | Pass — ARCH-1–ARCH-44 referenced |
| Story sizing for single-agent delivery | Needs work — 2 Major issues |

### Critical Issues Requiring Immediate Action

No Critical product, requirements, architecture, UX, coverage, or dependency issue was found.

The following **Major delivery-planning issues must be resolved before Sprint Planning**:

1. **Split Story 1.1.** Separate the shared-core/contract bootstrap, platform shell work, and generated-binding/CI integration into sequential, independently completable Stories. Preserve a thin runnable vertical slice and avoid creating all domain persistence up front.
2. **Split Story 9.5.** Separate Apple controlled distribution, Android controlled distribution, and cross-platform release-evidence aggregation. Preserve protected-secret boundaries and make aggregate evidence depend only on completed platform slices.

### Recommended Next Steps

1. Reopen the Epics workflow only for the two sizing corrections; do not change FR allocation, user outcomes, or the approved GitHub discovery scope.
2. Revalidate numbering, backward-only dependencies, and acceptance ownership after the split. All 64 FRs, 54 NFRs, 44 Architecture requirements, and 37 UX requirements must remain covered.
3. Rerun Implementation Readiness. The expected outcome is `READY` if the two oversized Stories are split without introducing coverage or dependency gaps.
4. After a `READY` result, run Sprint Planning, then create implementation-ready Story files in dependency order.
5. Before refining automatic-monitoring capacity behavior, obtain an explicit product decision; until then, Stories must continue to prohibit invented limits, silent eviction, and “unlimited” promises.

### Issue Count

- Critical: 0.
- Major: 2.
- Minor: 2.
- Non-blocking product advisories: 1.
- Categories with issues: Story sizing, execution/evidence partitioning, sprint task granularity, deferred capacity policy.

### Final Note

This assessment found no missing product behavior and no broken traceability chain. The project is technically and functionally well specified. The `NEEDS WORK` decision is driven solely by delivery-unit size: two Stories combine too many independently failing toolchains or protected release environments for reliable single-agent completion. Correct those slices before Sprint Planning; broad document redesign is neither required nor recommended.

**Assessment completed:** 2026-08-13  
**Assessor:** Codex — Implementation Readiness workflow
