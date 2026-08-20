---
artifact_kind: story
status: done
delivery_profile: standard
delivery_scope: windows-first-rss-minimum-loop
source_story: '2.2'
blocking_condition: ''
followup_review_recommended: true
risk_override_reason: '按已批准的快速第一阶段采用 standard；网络安全、迁移和真实候选证据不可豁免'
non_waivable_gates:
  - ssrf_tls_response_budget
  - sqlite_v5_migration_atomicity
  - release_surface_and_project_isolation
---

# Story 2.2：订阅并增量同步 RSS/Atom 来源

状态：done

## Story

作为 AI 开发者，  
我希望订阅公开 RSS/Atom 信息源并在当前设备增量同步，  
以便持续获得厂商博客和技术媒体的真实更新。

## 验收标准

### AC1：只保存通过网络安全校验的公开来源

**Given** 用户输入公开 RSS/Atom URL  
**When** 保存来源  
**Then** 共享核心校验协议、规范化地址并创建当前设备来源配置  
**And** 拒绝本机、环回、内网、保留地址、禁用协议及无效 TLS，不提供忽略证书错误开关。（FR7、NFR16、NFR17）

### AC2：首次及后续同步保留真实可溯源字段

**Given** 合法 RSS 2.0 或 Atom fixture 包含缺失作者、缺失摘要、不同时间格式和内容更新  
**When** 执行首次与后续增量同步  
**Then** 可溯源的有效条目被解析并保留可获得字段，无法解析字段明确为空而不伪造  
**And** ETag、Last-Modified 或来源游标被持久化，用于避免重复下载和重复结果。（FR7、FR10、NFR26）

### AC3：请求预算和每跳 SSRF 校验不可绕过

**Given** 来源返回超大响应、慢响应或多次重定向  
**When** 达到 10MB、5 次重定向或 30 秒任一预算上限  
**Then** 请求终止并保存可诊断的来源级错误  
**And** 应用保持可操作，实际连接目标在每次重定向时重新通过安全校验。（NFR18、ARCH-13）

### AC4：来源错误准确归属并遵守退避

**Given** RSS/Atom 返回 429、5xx、解析错误或连接失败  
**When** 同步任务处理失败  
**Then** 错误准确归属该来源，并记录可重试性和下一次允许时间  
**And** 有 Retry-After 时不得提前请求；无指示时首次重试不少于 60 秒，连续间隔不缩短。（NFR23、NFR24）

### 第一阶段验收边界

- 本轮只实现共享 Rust core、Windows Tauri/DesktopApi 和 Windows 来源界面；Apple/iPhone/iPad/Android 继续 Deferred/N/A。
- 第一阶段唯一真实来源为 RSS/Atom。不得创建空的 GitHub Release、arXiv 或 GitHub discovery adapter，也不得在 UI 显示未实现入口。
- 本 Story 交付 RSS/Atom 来源的创建/查询、共享安全网络层、adapter 与可持久化增量 checkpoint；AC2 的同步通过 core 内部 application harness 验证。Story 2.5 才公开“立即同步单源/同步全部”、持久任务与统一状态；Story 2.6 才负责本轮计数和最小结果；Story 4.1 才负责最终统一情报规范化投影。
- 不实现后台计划同步、托盘、通知、AI、评分、搜索、反馈、收藏、移动 ExecutionBudget 或第二数据库。
- 完成结论只能声明 `Story 2.2 Windows RSS milestone PASS/FAIL`，不得宣称完整产品或跨平台 MVP 完成。
- 所有依赖、缓存、浏览器和测试服务必须位于项目目录；不得全局安装 Python、Node、Rust 或修改系统网络、代理、证书、主题、缩放及权限设置。

## Tasks / Subtasks

- [x] Task 1：建立 RSS/Atom 来源、adapter 候选与错误合同（AC1–AC4）
  - [x] 在 Rust 权威合同中定义版本化 `SaveSourceInputV1`、`SourceViewV1`、`SourcePageV1`；公开 DTO 均含 `contract_version`，JSON 使用 `snake_case`，ID 不透明，时间为 RFC3339 UTC，可选字段显式 `null`。`SourcePageV1` 使用 core 生成/校验的不透明 keyset cursor，limit 1–100，不暴露 offset/rowid。adapter 内部使用有界 `RawSourceCandidate`/`FetchIncrementalResult`，不把它们提前暴露为 Story 2.6 的同步结果合同。
  - [x] `SourceViewV1` 至少包含 source ID、kind=`rss_atom`、规范 URL 的安全展示值、enabled、revision、created/updated time、last success、freshness、status、retryability 和 `next_allowed_at`；默认诊断不得包含 URL query/fragment、标题、正文或响应 payload。
  - [x] 公共 `TaskRef/TaskSnapshot/TaskState`、`start_sync/get_task` 及 `partially_succeeded` 语义全部留给 Story 2.5；本 Story不得创建一个删减状态的 V1 合同。内部 harness 只返回 adapter 结果或 `AppError`。
  - [x] 固定来源错误分类：URL/协议/IP/TLS 安全拒绝、连接/总超时、响应过大、重定向超限、rate limited、5xx、source format；映射到现有 `AppError` 稳定 category/retryability/source_id，不回显底层网络错误或完整 URL，并提供稳定阶段、影响范围、数据安全状态与恢复动作 message key。
  - [x] 新增唯一权威 fixtures：RSS 2.0、Atom、304、内容更新、缺作者、缺摘要、不同合法/非法时间、429（秒与 HTTP-date）、5xx、畸形 XML、超大流、慢流、重定向链与重定向到私网。fixtures 仅放 `contracts/fixtures/rss-atom/`，平台测试不得复制业务样本。
  - [x] 同步 contract manifest、schema/error snapshot、golden fixture、radar-ffi 当前共享 DTO/error 映射、Tauri allowlist、TypeScript exact guards 和 xtask gate。共享 FFI 若已镜像这些 DTO，必须通过 drift/golden 检查；Apple/Android 的生成调用面、平台 UI 与运行证据继续 Deferred，不得半生成未使用绑定。

- [x] Task 2：实现唯一共享 HTTP 安全策略（AC1、AC3–AC4）
  - [x] 新建 `crates/radar-core/src/infrastructure/http/source_http_policy.rs`，由所有公开来源请求复用；RSS adapter 不得自行实现较弱策略，React/Tauri 不得直接联网。
  - [x] 精确锁定项目直接依赖：`reqwest = 0.13.4`、`tokio = 1.53.1`、`url = 2.5.8`、`ipnet = 2.12.1`；RSS/Atom XML parser 使用经审查的项目锁定版本（优先 `quick-xml = 0.41.0`）。只更新项目 Cargo.toml/Cargo.lock，不安装全局工具。
  - [x] 来源 client 禁用 cookie store、隐式系统/环境代理和默认自动重定向；不启用 HTTP/3，不接受无效 TLS，不提供 insecure/ignore-certificate 开关。连接、首字节和总请求均服从同一 30 秒绝对 deadline，取消后不得继续读流或写库。
  - [x] 第一阶段真实 RSS endpoint 仅接受 `https`、非空 host、无 userinfo；移除 fragment，保留对资源有意义的 path/query，规范 scheme/host/default port。公网 `http` 在探测前阻断，不能静默升级；Story 2.1 历史 HTTP preference 可保留为不可执行配置并提示修改，但不得生成可运行 `sources` 投影。该限制以 NFR16 优先于 Architecture 中“HTTP/HTTPS、推荐 HTTPS”的宽泛长期描述。
  - [x] 请求前解析 DNS 并验证全部候选 IP；拒绝 loopback、link-local、private、carrier-grade NAT、benchmark、documentation、multicast、reserved、unspecified 及 IPv4-mapped IPv6 等非公网目标。实现为共享 policy/client factory，按 authority 与 redirect hop 把同一次校验得到的地址集合 pin 到 connector/custom resolver；不得“先 resolve 检查、再让长期共享 Client 自行 resolve”，实际 peer 必须属于本次已验证集合。
  - [x] 每次 redirect 都重新执行 scheme、host、端口、DNS 和全部候选 IP 校验；最多跟随 5 次，检测循环。禁止把 Authorization/Cookie 等敏感 header 跨 origin 传播。不得仅用 `reqwest::redirect::Policy::limited(5)` 代替每跳校验。
  - [x] 固定 `MAX_RESPONSE_BYTES = 10_000_000`；原始传输体按流累计并执行精确边界。V1 明确拒绝所有非 identity 压缩表示，避免在没有双预算解压器时接受压缩炸弹。XML 实体/嵌套/文本/条目数量设置确定性上限，禁止外部实体和 DTD 网络解析。
  - [x] 429 解析合法 `Retry-After` delta-seconds 或 HTTP-date，并按不早于其时间进入 `retry_wait`；缺失/非法指示时首次退避 ≥60 秒，后续间隔非递减且第三次至少为首次 4 倍。测试注入 Clock/Resolver/Transport，不等待真实时间、不访问公网。

- [x] Task 3：实现 RSS/Atom adapter 与确定性增量语义（AC2–AC4）
  - [x] 新建 `domain/sources/` 的来源值对象/候选模型与 `infrastructure/sources/rss_atom/` adapter；adapter 只验证配置、构造条件请求、抓取/解析并返回候选及新 cursor，不直接执行 SQL、发通知或调用 AI。
  - [x] RSS 2.0 与 Atom 使用有界流式 XML 解析；保留原始标题、链接、作者、摘要/内容、published/updated、GUID/Atom ID 等可获得字段。缺失或不可解析字段为 `None` 并记录稳定 source-format warning/error，不生成假作者、假时间、假摘要或当前时间替代值。
  - [x] 条目身份优先使用合法稳定 GUID/Atom ID；缺失时使用规范化原文 URL 的确定性 hash；二者均缺失时条目不可持久化且计为失败，不能用数组位置、标题或系统时间制造身份。
  - [x] 首次请求不发送条件 header；后续请求只发送已持久化的 ETag/Last-Modified。304 必须产生零下载结果且保留原 cursor；200 成功解析后才更新条件元数据，失败/取消不得推进 cursor。
  - [x] adapter 对同一稳定 ID 使用内容 hash 判定 unchanged/changed/new，并把处置建议返回调用方；正式 inserted/updated/skipped/failed 计数及结果持久化留给 Story 2.6。Story 4.2 的跨来源确定性关联不在本 Story实现。
  - [x] 解析需保留 XML namespace 与编码正确性。使用 `quick-xml 0.41.0` 时保持 well-formedness 检查；V1 不启用额外 encoding feature，只接受 UTF-8/ASCII-compatible 且可正确解码的输入，UTF-16BE/LE、ISO-2022-JP 或声明/字节不一致时稳定拒绝，不能在错误字节解释下产生伪造字段。
  - [x] adapter 测试只使用进程内 fake Resolver/Connector，不走生产公网/DNS，也不放宽生产 SSRF allowlist。

- [x] Task 4：以 SQLite v5 保存来源运行投影与增量 checkpoint（AC1–AC4）
  - [x] 从当前 schema v4 单调迁移到 v5；继续扩展 `DemoStore::from_connection` 的唯一 migration runner 与启动 verifier，先完成原子 v5 迁移，再把新增 SQL 经同一 connection owner 委托给 source repository seam；不得创建只处理 v5 的第二 runner。fresh、v1/v2/v3/v4→v5 结果一致并逐字段保留 demo、FTS、setup 与 configuration 数据。未知未来 schema、缺表/列/约束、非法枚举或损坏 cursor fail closed。
  - [x] 在唯一 `radar-core` SQLite owner 中只新增 `sources` 与最小 `source_entry_checkpoints` 及所需索引/唯一约束；不得提前创建 `jobs`、Story 2.5/2.6 的 `sync_runs/sync_source_results/sync_result_items`、原始正文 staging 或 Story 4.1 的最终规范化投影。
  - [x] 明确单一事实源：`configuration_versions.configuration_json.source_preferences` 仍是用户配置意图的权威；`sources` 是绑定 `configuration_version + canonical preference identity` 的运行投影，只保存执行所需快照和运行状态。`save_source` 携带 `expected_configuration_revision` 与 idempotency key，并在同一事务追加配置版本、调用同一 configuration validator、reconcile 投影；`/rules` 后续修改来源偏好也必须走同一 reconcile 用例。禁止两边独立写入、last-write-wins 或让 setup 的 demo source examples 创建真实订阅。
  - [x] `sources` 至少持久化稳定 ID、kind、规范 URL、enabled、revision、ETag、Last-Modified、adapter cursor、last attempt/success、status、consecutive failures、next allowed time 与脱敏错误分类；URL 唯一性基于规范化身份。
  - [x] `source_entry_checkpoints` 只保存 `(source_id, stable_external_id, content_hash, first_seen_at, last_seen_at)`，不保存标题、摘要、正文、HTML 或平台展示 DTO；它仅用于 adapter 级增量判定，不是可消费情报事实源。Story 2.6/4.1 后续以正式来源事务消费 adapter candidates，不得把 checkpoint 当结果页面或 intel item。
  - [x] core 内部 `fetch_incremental` harness 成功时在同一短 `IMMEDIATE` 事务提交 ETag/Last-Modified/opaque cursor、entry checkpoints 与 source health；解析、取消、超时或存储失败时全部不推进。正式同步运行/计数/结果事务在 Story 2.5/2.6 接入同一 repository seam。
  - [x] 保持 WAL、`foreign_keys=ON`、有限 busy timeout、单写执行器和启动 schema 审计。网络等待不得持有 SQLite 写事务；先抓取/解析，后进入短事务提交并重新检查 source revision。

- [x] Task 5：接通精确 Windows 来源保存/查询与来源界面（AC1、AC3–AC4）
  - [x] 在 Application 层只公开窄用例 `save_source`、`query_sources`；只允许 `source_kind=rss_atom`。`fetch_incremental` 保持 core 内部 seam，公开 `start_sync/get_task` 留给 Story 2.5。禁止 `fetch_url`、generic execute、HTTP/SQL/file/shell command。
  - [x] `save_source` 在写库前通过同一安全 HTTP policy 完成有界来源探测，至少验证实际 DNS/IP、TLS 和 RSS/Atom 格式；无效 TLS、禁止目标或格式不符不得落库。瞬时连接/5xx 以来源保存失败返回稳定可重试错误，不以“先保存以后再说”绕过 AC1；探测期间不得持有数据库 mutex/事务。
  - [x] 新增 `save_source_v1`、`query_sources_v1` 异步 Tauri commands；来源探测 async future 由现有 Tauri Tokio runtime 驱动，不在 core 内另建嵌套 runtime。阻塞 SQLite 进入现有受控 executor；复用 `CommandErrorV1`、panic containment、correlation/source ID 和 release handler allowlist。
  - [x] 扩展唯一 DesktopApi 与 `tauri-desktop-api.ts`，对 exact keys、contract version、safe revision、enum、时间、URL 展示值、source identity 与状态组合 fail closed；本地 query 保持 10 秒 IPC timeout，`save_source` 的安全探测使用显式 35 秒上限以覆盖 core 的 30 秒绝对 deadline。
  - [x] 在 `apps/windows/src/features/sources/` 和 `/sources` 实现最小来源页：添加 HTTPS RSS/Atom URL、来源列表及从 `/rules` 投影的 enabled/最后验证/错误状态。立即同步按钮、任务进度与 retry_wait 控件留给 Story 2.5；不得显示 GitHub/arXiv 占位或“全部来源就绪”。
  - [x] TanStack Query 管 source DTO，集中 `sourceKeys`；保存来源不得乐观提交，成功后按领域根 key 失效。旧 revision/迟到响应不得覆盖新状态。
  - [x] 页面覆盖 `initial_loading`、`empty`、`ready`、`refreshing_with_data`、`blocking_failure`、`read_only_migration_failure` 与保存中/失败/成功；刷新错误保留旧列表，保存失败保留 URL 输入、焦点和滚动。输入、错误与状态有稳定 label、可见焦点及非颜色文字，并持续显示“仅此 Windows 设备；真实网络请求仅访问已验证的公开 HTTPS RSS/Atom 地址”。
  - [x] 不新增 Tauri HTTP capability、shell/fs capability、远程 WebView 导航、系统代理修改或证书例外；所有来源网络仍只在 Rust 共享层发生。

- [x] Task 6：形成安全、增量、恢复与真实运行证据（AC1–AC4）
  - [x] Rust 单元/集成测试覆盖 URL canonicalization、全部拒绝 IP 类别、DNS 多地址任一非法即拒绝、DNS rebinding/TOCTOU、IPv4-mapped IPv6、每跳 redirect、TLS/connector 失败、header 跨 origin、10,000,000-byte 原始响应边界、V1 非 identity 压缩拒绝、30 秒 deadline 与取消。
  - [x] adapter fixture 测试覆盖 RSS/Atom、namespace/CDATA/encoding、缺失字段、非法时间、GUID/Atom ID/URL fallback、首次/304/updated/skipped、cursor 只在成功后推进、429/5xx/解析错误及日志脱敏；连接/TLS/取消由 injected connector/clock 层覆盖。
  - [x] 数据库测试覆盖 fresh/v1/v2/v3/v4→v5、schema 损坏、同 URL 冲突、source/config revision、reopen、checkpoint 原子回滚、失败后重试和 ETag/Last-Modified/cursor/checkpoint 一致；临时 DB 使用唯一 RAII 目录。
  - [x] Tauri/DesktopApi/Vitest 覆盖 exact save/query commands、panic 后恢复、未知/矛盾 DTO、HTTPS 来源添加、阻断失败零半写、从 `/rules` 投影的 enabled 状态、页面 loading/empty/ready/refresh/error/read-only 状态、失败保留列表/输入与迟到响应；TLS/格式/超时/5xx 的网络语义由注入生产 policy 与 SQLite harness 直接覆盖，不在 UI 重复实现。
  - [x] Playwright 使用 deterministic DesktopApi seam 覆盖 HTTPS 来源保存、列表/reload、阻断错误与无 GitHub/arXiv/AI/通知外联；浏览器 mock 只能证明 UI 接缝，不能冒充 Rust HTTP/SQLite 或同步证据。
  - [x] Rust production-policy 测试使用真实生产 policy + fake Resolver/Connector，证明合法公网映射、私网拒绝、TOCTOU 和 redirect 复验；独立 core integration harness 验证 RSS/Atom 首次/后续、304、更新、checkpoint/reopen，但不冒充 release candidate 网络证据。
  - [x] 已重建无测试 transport 的项目隔离 MSVC release，并由 release-surface/xtask 静态门禁拒绝 test transport、localhost allow、开发 URL 和测试 command；按用户批准不在本 Story 再启动原生 GUI。真实启动、save/query IPC、DB、进程树及 30 次 Windows 冷启动 P95 统一延后到第一阶段最终发布候选冻结后执行一次并绑定最终 candidate/source SHA。
  - [x] 项目隔离 format/lint/typecheck、Vitest 81/81、Playwright 23/23、前端 build、Rust fmt/Clippy、GNULLVM 非桌面 117/117、MSVC desktop 9/9、xtask contracts 和 MSVC release 已通过；automation、test review 与 traceability 已刷新，AC1–AC4 为 4/4 FULL，gate=PASS。

### Review Findings

- [x] [Review][Patch][Group 1] 补齐 IANA IPv6 特殊用途/保留网段拒绝，避免 `100::/64`、`64:ff9b:1::/48` 等绕过 SSRF。 [`crates/radar-core/src/infrastructure/http/source_http_policy.rs:361`]
- [x] [Review][Patch][Group 1] 将 source idempotency 查询放入同一 `BEGIN IMMEDIATE` 事务，并让同 key 同 payload 跨连接重放权威响应。 [`crates/radar-core/src/application/sources.rs:38`]
- [x] [Review][Patch][Group 1] 重新保存失败来源时原子清除 failure count、retry deadline、last attempt 与 error，避免 `ready + next_allowed_at` 矛盾状态。 [`crates/radar-core/src/application/sources.rs:96`]
- [x] [Review][Patch][Group 1] 304 必须验证存在条件请求、更新成功/尝试时间并采用响应中的新 ETag/Last-Modified，同时保持 checkpoint 不推进。 [`crates/radar-core/src/infrastructure/http/source_http_policy.rs:219`; `crates/radar-core/src/application/sources.rs:185`]
- [x] [Review][Patch][Group 1] 永不重试的 source-format 错误应写 `error + never + null next_allowed_at`，不能进入 `retry_wait`。 [`crates/radar-core/src/application/sources.rs:231`]
- [x] [Review][Patch][Group 1] 将 429 的合法 `Retry-After` 从真实 transport 传到来源退避持久化，不能只返回无上下文错误。 [`crates/radar-core/src/infrastructure/http/source_http_policy.rs:248`]
- [x] [Review][Patch][Group 1] 为永久 4xx、超大 Retry-After 与时间加法溢出定义稳定分类和上限，避免永久错误无限重试或无法记录失败。 [`crates/radar-core/src/infrastructure/http/source_http_policy.rs:64`; `crates/radar-core/src/infrastructure/http/source_http_policy.rs:260`]
- [x] [Review][Patch][Group 1] 严格验证 RSS `rss/channel/item` 与 Atom feed namespace/root/直接父子关系，拒绝任意含 `item`/`entry` 的 XML 和扩展字段污染。 [`crates/radar-core/src/infrastructure/sources/rss_atom/mod.rs:41`]
- [x] [Review][Patch][Group 1] Feed published/updated 使用真正 RFC3339/IMF-fixdate 解析，禁止把 Retry-After delta-seconds 当发布时间。 [`crates/radar-core/src/infrastructure/sources/rss_atom/mod.rs:201`]
- [x] [Review][Patch][Group 1] 让 source golden 通过生产保存/映射行为生成或断言，不能只反序列化手写 expected。 [`contracts/fixtures/golden/source_view_v1.json:12`; `crates/radar-ffi/tests/boundary.rs:115`]
- [x] [Review][Patch][Group 1] 建立 core-owned 增量 application harness：从 repository 读取 endpoint/validators，执行生产 policy，再短事务提交并在 reopen 后复用条件元数据。 [`crates/radar-core/src/application/sources.rs:168`]
- [x] [Review][Patch][Group 1] 用类型化探测证明绑定 canonical URL，消除 `save_validated_source` 仅靠注释信任调用方或探测 A/保存 B 的绕过。 [`crates/radar-core/src/application/sources.rs:27`]
- [x] [Review][Patch][Group 1] 接受结构合法但尚无条目的空 RSS/Atom feed，同时继续拒绝非 feed XML。 [`crates/radar-core/src/infrastructure/sources/rss_atom/mod.rs:108`]
- [x] [Review][Patch][Group 1] Atom link 同时处理 Start/Empty，按 `rel=alternate` 选择规范文章地址，避免元素顺序改变身份。 [`crates/radar-core/src/infrastructure/sources/rss_atom/mod.rs:53`]
- [x] [Review][Patch][Group 1] 对 XML 累计字段长度、嵌套深度和遇到的 entry 数设总上限，不能仅限制单个 text event 或有效输出数。 [`crates/radar-core/src/infrastructure/sources/rss_atom/mod.rs:43`; `crates/radar-core/src/infrastructure/sources/rss_atom/mod.rs:114`]
- [x] [Review][Patch][Group 1] 校验并规范化 candidate 原文 URL 的 HTTPS/host/userinfo/fragment/default-port，再用于 fallback hash；拒绝危险 scheme。 [`crates/radar-core/src/infrastructure/sources/rss_atom/mod.rs:155`]
- [x] [Review][Patch][Group 1] 将 XML CPU 解析纳入可合作取消的绝对 deadline，避免最后一次网络 await 后同步解析越过 30 秒。 [`crates/radar-core/src/infrastructure/http/source_http_policy.rs:163`; `crates/radar-core/src/infrastructure/http/source_http_policy.rs:299`]
- [x] [Review][Patch][Group 1] 拒绝 `not_modified + nonempty candidates` 和单结果重复 stable ID，避免静默丢弃或顺序依赖 checkpoint。 [`crates/radar-core/src/application/sources.rs:185`]
- [x] [Review][Patch][Group 1] 来源操作错误补 source_id 与稳定阶段/恢复分类，不能把 TLS、redirect、5xx、oversize 全压成无归属的 `network.source`。 [`crates/radar-core/src/contracts/errors.rs:245`; `crates/radar-core/src/infrastructure/http/source_http_policy.rs:214`]
- [x] [Review][Patch][Group 1] `FetchIncrementalResult` 增加 opaque cursor 与 new/changed/unchanged disposition，比较既有 content hash 后再提交。 [`crates/radar-core/src/domain/sources/mod.rs:18`; `crates/radar-core/src/application/sources.rs:193`]
- [x] [Review][Patch][Group 1] 对 identity-less 与非法时间条目产生稳定失败/警告，而不是静默丢弃或与字段缺失混同。 [`crates/radar-core/src/infrastructure/sources/rss_atom/mod.rs:158`; `crates/radar-core/src/infrastructure/sources/rss_atom/mod.rs:201`]
- [x] [Review][Patch][Group 1] 从 XML declaration 读取并校验 encoding，不能仅扫描开头 160 字节；声明/字节不一致必须稳定拒绝。 [`crates/radar-core/src/infrastructure/sources/rss_atom/mod.rs:213`]
- [x] [Review][Patch][Group 1] 对 Content-Encoding token 做大小写无关处理；V1 明确安全拒绝所有非 identity 压缩响应，不在无双预算解压器时接受压缩内容。 [`crates/radar-core/src/infrastructure/http/source_http_policy.rs:272`]
- [x] [Review][Patch][Group 1] 补齐 304、非法时间、429 两种格式、5xx、慢流、重定向/私网跳转、TLS/connector、压缩拒绝与取消的权威 fixture/injected transport 测试矩阵。 [`contracts/fixtures/rss-atom`; `crates/radar-core/tests/rss_atom_sources.rs:13`; `crates/radar-core/src/infrastructure/http/source_http_policy.rs`]
- [x] [Review][Defer][Group 1] `.env.example` placeholder 豁免仍可能放过真实凭据，属于前序通用 xtask 边界问题。 [`crates/xtask/src/contracts.rs:474`] — deferred, pre-existing
- [x] [Review][Patch][Group 2] 将来源输入预检、生产探测收口到 core Application seam；无效合同、过期 revision 在联网前拒绝，探测 panic 映射为稳定错误。 [`crates/radar-core/src/application/sources.rs`; `apps/windows/src-tauri/src/commands/mod.rs`]
- [x] [Review][Patch][Group 2] 已提交的 source idempotency 同 key 同 payload 在探测前重放权威响应，避免响应丢失后因 endpoint 离线而错误失败。 [`crates/radar-core/src/application/sources.rs`; `apps/windows/src-tauri/src/commands/mod.rs`]
- [x] [Review][Patch][Group 2] 对并发生产来源探测设置有界容量并返回稳定 rate-limit 错误，避免渲染进程无限并发 DNS/TLS/下载。 [`apps/windows/src-tauri/src/commands/mod.rs`]
- [x] [Review][Defer][Group 2] Desktop mutation 超时仍不能取消已进入系统网络栈的底层 Tauri 命令；当前通过 core 30 秒 deadline、UI 45 秒上限及超时后 cache reconciliation 保持一致性。更强的 operation cancellation 合同留给 Story 2.5 持久任务生命周期。 [`apps/windows/src/lib/desktop-api/tauri-desktop-api.ts`; `apps/windows/src/features/sources/sources-page.tsx`]
- [x] [Review][Patch][Group 2] `saveSource` 关联请求与响应时比较规范化 scheme/host/effective port/path，仅忽略合同明确隐藏的 query/fragment。 [`apps/windows/src/lib/desktop-api/tauri-desktop-api.ts`]
- [x] [Review][Patch][Group 2] Source DTO guard 按 ready/error/retry_wait 明确允许状态组合，并验证时间顺序、空页 cursor 与 opaque source_id。 [`apps/windows/src/lib/desktop-api/desktop-api.ts`]
- [x] [Review][Patch][Group 2] `querySources` 在 IPC 前验证 cursor/limit，并在响应后要求 items 不超过请求 limit。 [`apps/windows/src/lib/desktop-api/tauri-desktop-api.ts`]
- [x] [Review][Dismiss][Group 2] 来源页无需实现第 101 条分页：本阶段权威配置最多 64 个 source preference，而页面一次读取 100 条；超过该约束会先被 core 拒绝。 [`apps/windows/src/features/sources/sources-page.tsx`]
- [x] [Review][Patch][Group 2] 来源保存幂等键由 32-bit FNV 改为项目运行时 Web Crypto SHA-256，保持同 revision/URL 重试稳定且不可实际构造碰撞。 [`apps/windows/src/features/sources/sources-page.tsx`]
- [x] [Review][Patch][Group 2] 配置读取失败时明确阻断来源保存并提供重试；冲突/超时失效配置与来源 cache，避免旧 revision 永久重试。 [`apps/windows/src/features/sources/sources-page.tsx`]
- [x] [Review][Patch][Group 2] 来源保存迟到成功仅在输入仍等于本次提交值时清空，保留用户随后输入的新 URL。 [`apps/windows/src/features/sources/sources-page.tsx`]
- [x] [Review][Patch][Group 2] 规则保存成功后同步失效来源投影 query，避免 `/sources` 长期展示旧 enabled 状态。 [`apps/windows/src/features/configuration-validation/configuration-editor.tsx`]
- [x] [Review][Patch][Group 2] populated source list 刷新时用 `isFetching` 暴露 `aria-busy`，而非只覆盖首次加载。 [`apps/windows/src/features/sources/sources-page.tsx`]
- [x] [Review][Patch][Group 2] 按 migration/storage/network 稳定错误分类区分只读迁移失败、阻断失败和可重试刷新。 [`apps/windows/src/features/sources/sources-page.tsx`]
- [x] [Review][Patch][Group 2] native release smoke 保持生产 SSRF 不可绕过，只证明真实 Tauri save/query IPC、localhost 拒绝、SQLite 零半写和进程树清零；合法公网成功、reload 与幂等三通道由注入生产 policy/SQLite harness 证明，测试 transport 不进入 release candidate。 [`scripts/windows-rss-smoke.ps1`]
- [x] [Review][Resolved][Group 2] Tauri source helper 覆盖 command 映射、panic 后恢复与存储一致性；完整网络失败矩阵由同一生产 Application policy 的 injected Resolver/Connector/Clock 直接测试，避免在 Tauri 层复制 transport 语义。 [`apps/windows/src-tauri/src/commands/mod.rs`; `crates/radar-core/src/infrastructure/http/source_http_policy.rs`]
- [x] [Review][Defer][Group 2] 旧 MSVC candidate/native smoke/30 次冷启动 SHA 证据已明确作废；本轮已完成新的 MSVC release 编译但不启动 GUI，真实 native smoke 与 30 次采样按用户批准统一延后到第一阶段最终发布候选。 [`_agentic-out/tests/evidence/story-2-2-cold-start-recovery.json`]
- [x] [Review][Patch][Group 3] Playwright 来源 fixture 改为初始空列表、保存后 stateful query、reload 持久化，并新增阻断失败保留输入/零外联场景。 [`tests/support/fixtures/demo-app.fixture.ts`; `tests/e2e/story-2-2-rss-atom-source.spec.ts`]
- [x] [Review][Patch][Group 3] standard profile 的 before_done 强制 automation、code review、test review、traceability，避免 Story 2.2 缺关键门禁仍置 done。 [`_agentic-out/artifacts.yaml`]
- [x] [Review][Patch][Group 3] Story 与 sprint 权威状态回退为 `in-progress`；未完成的网络 transport/增量合同不再被 `review` 状态掩盖。 [`_agentic-out/implementation/stories/2-2-订阅并增量同步-rss-atom-来源.md`; `_agentic-out/implementation/sprint-status.yaml`]
- [x] [Review][Resolved][Group 3] 已重新生成 Story 2.2 automation、test-review 与 traceability 正式质量制品；测试质量 100/100，AC1–AC4 4/4 FULL，gate=PASS。 [`_agentic-out/tests/automation-summary.md`; `_agentic-out/tests/test-review.md`; `_agentic-out/tests/traceability-matrix.md`]
- [x] [Review][Patch][Group 3] release-surface gate除 handler/CSP/capability 检查外，在两份 native candidate 脚本中扫描最终二进制并拒绝 fake resolver/connector、localhost allow 与 test transport 标记。 [`apps/windows/src-tauri/tests/release_surface.rs`; `crates/xtask/src/contracts.rs`; `scripts/windows-demo-smoke.ps1`; `scripts/windows-rss-smoke.ps1`]
- [x] [Review][Patch][Group 3] 候选 source fingerprint 补齐 DesktopApi/query/schema/snapshot/golden/capability/Cargo manifest，并要求采样前后完全一致。 [`scripts/windows-demo-smoke.ps1`]
- [x] [Review][Patch][Group 3] native RSS PASS 证据延后到环境恢复和完整 owned PID 清零后原子发布；正式 evidence 禁止 `SkipBuild`。 [`scripts/windows-rss-smoke.ps1`]
- [x] [Review][Patch][Group 3] 冷启动 durable evidence 仅在显式 `-Evidence -Samples 30` 时写入，普通单次运行不再覆盖正式证据。 [`scripts/windows-demo-smoke.ps1`]
- [x] [Review][Patch][Group 3] Playwright runner 处理 spawn error、taskkill 超时/失败和 fallback exit，并让 cleanup 失败决定非零退出。 [`scripts/playwright-run.mjs`]
- [x] [Review][Patch][Group 3] Playwright 零外联 fixture 已通过 context routing 覆盖 image/iframe/EventSource/CSS 等浏览器资源通道。 [`tests/support/fixtures/demo-app.fixture.ts`]
- [x] [Review][Patch][Group 3] xtask 对 RSS/Atom 必需 fixture 执行成功/失败语义，并将 source golden 反序列化为权威 input/expected DTO。 [`crates/xtask/src/contracts.rs`]
- [x] [Review][Defer][Group 3] xtask 的动态 import/re-export、字符串拼接 invoke/remote URL 需要 AST/依赖图级门禁，超出本 Story 的确定性小补丁并进入通用构建门禁 backlog；现有 release handler、CSP/capability 与最终二进制 marker gate 均通过。 [`crates/xtask/src/contracts.rs`]
- [x] [Review][Patch][Group 3] durable evidence 移除本机绝对路径和明文设备名，改用项目相对路径与匿名 device profile hash。 [`scripts/windows-demo-smoke.ps1`; `scripts/windows-rss-smoke.ps1`]

## Dev Notes

### 交付档位与风险取舍

- Agentic Flow 解析结果为 `standard`（compatibility fallback）；网络安全、高 NFR 和跨 artifact 风险会推荐 `assured`。
- 按用户批准的快速第一阶段策略，本 Story 有意采用 `standard`，但 Task 2 的 SSRF/TLS/预算、Task 4 的原子持久化及 Task 6 的真实候选证据均为不可豁免门禁。项目级 NFR 总评可在第一阶段发布汇总时统一刷新。

### 架构与复用边界

- 数据流固定为 `Windows UI → DesktopApi → 精确 Tauri command → radar-core Application → RSS adapter/SQLite`；UI/Tauri 不执行 SQL，不直接请求来源。
- 复用 Story 2.1 的 v4 `DemoStore` 单一连接所有权、`DemoState` 串行 mutex、`spawn_blocking`、AppError、contract manifest、DesktopApi 10 秒 timeout、query client、Router、xtask/release surface 和项目隔离脚本；不要复制第二套 store、错误、transport 或测试 runner。
- Story 2.1 的 `source_preferences` 是规则偏好，不等同于已订阅来源。Story 2.2 的 `sources` 是真实 endpoint 配置，必须有独立稳定 ID/revision/health；保存来源后可通过明确投影关联配置偏好，但不得把 setup demo `source_example_ids` 伪装为订阅。
- 网络等待不能占用现有 SQLite mutex/写事务。Application orchestration 必须把 resolver/transport/clock 作为可替换内部 ports，生产实现唯一，测试实现确定性。
- `reqwest` 默认会自动跟随最多 10 次重定向；本 Story必须使用自定义/禁用默认 policy，并在应用层逐跳校验后再发起下一请求。
- Tokio `timeout` 只有 future 主动 yield 时才可强制截止，因此解析/解压循环必须有界并让出执行；不得用单一 timeout 包裹不可中断的 CPU/同步解析后宣称满足 30 秒预算。
- `quick-xml 0.41.0` 提供流式 pull reader；保持 end-name/well-formedness 检查，不要全局 trim 导致 CDATA/分段文本语义损坏。

### 推荐文件落点

```text
contracts/fixtures/rss-atom/
crates/radar-core/src/domain/sources/
crates/radar-core/src/application/commands/save_source.rs
crates/radar-core/src/application/queries/query_sources.rs
crates/radar-core/src/application/rss_incremental_harness.rs
crates/radar-core/src/infrastructure/http/source_http_policy.rs
crates/radar-core/src/infrastructure/sources/rss_atom/
crates/radar-core/src/infrastructure/database/source_repository.rs
crates/radar-core/src/contracts/dto/source.rs
apps/windows/src/features/sources/
tests/e2e/story-2-2-rss-atom-source.spec.ts
scripts/windows-rss-smoke.ps1
```

当前仓库的 SQL 仍集中在 `application/demo.rs`。本 Story允许为真实来源首次建立 `infrastructure/database` seam，并逐步把新增 SQL 放到 repository；不要为追求目录纯度大规模搬迁既有 demo/configuration SQL，也不要留下两套 migration runner。

### 回归与反模式护栏

- 不 reset/checkout/覆盖当前脏工作树；Story 1.6–2.1 的未提交内容均视为用户资产。
- 不把公网 E2E 当作门禁，不把静态字符串扫描、mock 自报零外联或 localhost 成功冒充生产 SSRF 证据。
- 不记录完整 URL query/fragment、Feed 标题/正文、Authorization/Cookie、DNS 原始响应或 XML payload；诊断只记录 source/task/correlation ID、阶段、状态、耗时、计数和稳定错误。
- 不在失败时推进 ETag/Last-Modified/cursor，不把 304 记为错误，不把解析到零有效条目静默伪装为有结果。
- 不自动信任 Feed 内链接，不渲染远程 HTML，不在 WebView 打开来源 URL；安全原文入口留给 Story 4.6。
- 不提前实现“同步全部”、跨来源部分成功汇总、GitHub/arXiv adapter、AI/通知、后台调度或最终 intel normalization。

### Previous Story Intelligence

- Story 2.1 已完成配置 validation、SHA-256/CSPRNG receipt、SQLite v4、精确 Tauri commands、DesktopApi fail-closed、`/rules` 和真实 native/30-sample 证据；本 Story在其上增量开发。
- 前序审查证明：安全门禁不能依赖源码子串；DesktopApi 必须 exact/fail-closed；浏览器 mock 不能冒充真实 IPC/SQLite；迁移必须 fresh 与所有受支持旧版本同构；临时 DB/进程/端口必须唯一并 RAII 清理；运行证据必须绑定源码与候选 SHA。
- 当前 Git 历史仍只有基线提交 `f73aed0`，大量已完成实现位于脏工作树；以当前文件事实为准，不能用 commit 历史推断能力缺失。

### Latest Technical Information

- reqwest 0.13.4 的默认 redirect policy 最多自动跟随 10 跳；自定义 policy 不会自动替你检测循环或执行 SSRF 校验，因此这里必须显式控制每跳。
- Tokio 1.53.1 的 `timeout` 在截止时取消 future，但无法抢占不 yield 的工作；流读取和 XML 处理必须合作式取消。
- quick-xml 0.41.0 适合有界流式解析；其文档明确提醒全局 trim 对由 comment/PI/CDATA 分隔的文本可能不正确，编码处理也需要显式策略。

### References

- [第一阶段范围、Epic 2 与 Story 2.2](../../planning/epics.md)
- [PRD Windows RSS 最小闭环、FR7/FR10、NFR16–18/NFR23–26](../../planning/prd.md)
- [Architecture 数据所有权、SSRF、HTTP runtime、来源事务与 Application API](../../planning/architecture.md)
- [UX Phase 1 Journey 与来源状态语义](../../planning/ux/EXPERIENCE.md)
- [Windows 设计 token 与来源表单状态](../../planning/ux/DESIGN.md)
- [Story 2.1 前序实现与审查经验](./2-1-管理并安全校验当前设备的关注配置.md)
- [reqwest 0.13.4 redirect policy](https://docs.rs/reqwest/0.13.4/reqwest/redirect/struct.Policy.html)
- [Tokio 1.53.1 timeout](https://docs.rs/tokio/1.53.1/tokio/time/fn.timeout.html)
- [quick-xml 0.41.0](https://docs.rs/quick-xml/0.41.0/quick_xml/)

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- 2026-08-18：Agentic Flow reconcile 后确认第一阶段下一项唯一允许自动选择的 Story 为 2.2；Story 2.1 已 done 并封存。
- 2026-08-18：按 standard 快速交付档创建 Story；明确保留 assured 风险建议，但不删减 SSRF、TLS、响应预算、迁移和真实运行证据。
- 2026-08-18：开始 Story 2.2 实施；按合同/fixtures → HTTP policy → RSS adapter → SQLite v5 → Windows IPC/UI → 全量证据顺序执行。
- 2026-08-18：MSVC Release workspace 106 tests、Clippy `-D warnings`、xtask contracts、Vitest 77/77、Playwright 22/22、lint/typecheck/format/build 全绿。
- 2026-08-18：原生 RSS 查询/拒绝保存/SQLite 零半写冒烟曾通过且零候选进程残留；后续源码继续变化，因此候选需在补丁冻结后只重建一次。
- 2026-08-18：一次 30 样本采集完成全部样本后在 PowerShell 5.1 最终哈希转换处失败；旧脚本未逐样本持久化精确耗时，故不伪造 P95。已增加采样前 manifest、逐样本结果和独立离线汇总器；30 次性能门统一延后到第一阶段最终候选。
- 2026-08-18：补丁冻结后只执行一次剩余自动化门禁：Rust GNULLVM 非桌面 117/117、MSVC desktop 9/9、Clippy `-D warnings`、xtask contracts、Vitest 81/81、Playwright 23/23、format/lint/typecheck/frontend build、MSVC release、PowerShell parser 与 `git diff --check` 全部通过；未启动原生 GUI，未重跑 30 次采样。
- 2026-08-18：正式质量流程收口：automation 严格去重后新增测试 0；test-review 100/100、0 violations；traceability AC1–AC4 4/4 FULL、P0 3/3、P1 1/1、gate=PASS。Story 2.2 Windows RSS milestone 完成。

### Completion Notes List

- 2026-08-18：Ultimate context engine analysis completed - comprehensive developer guide created。
- 2026-08-18：分组三轮审查发现的生产 policy、增量 cursor/disposition、UI 状态和证据可靠性问题已修复；当前仅保留 Desktop mutation 的跨 IPC 取消/对账增强、最终候选原生运行/性能证据，以及 automation/test-review/traceability 正式质量制品，Story 继续保持 in-progress。

### File List

- `_agentic-out/implementation/stories/2-2-订阅并增量同步-rss-atom-来源.md`
- `_agentic-out/implementation/sprint-status.yaml`
- `_agentic-out/tests/evidence/story-2-2-cold-start.json`
- `_agentic-out/tests/evidence/story-2-2-native-rss-smoke.json`
- `contracts/fixtures/rss-atom/*`
- `contracts/fixtures/golden/source_view_v1.json`
- `crates/radar-core/src/application/sources.rs`
- `crates/radar-core/src/contracts/dto/source.rs`
- `crates/radar-core/src/domain/sources/mod.rs`
- `crates/radar-core/src/infrastructure/http/source_http_policy.rs`
- `crates/radar-core/src/infrastructure/sources/rss_atom/mod.rs`
- `crates/radar-core/tests/rss_atom_sources.rs`
- `scripts/lld-link-xwin.cmd`
- `scripts/windows-demo-aggregate.ps1`
- `_agentic-out/tests/evidence/story-2-2-cold-start-recovery.json`
- `apps/windows/src/features/sources/sources-page.tsx`
- `apps/windows/src/features/sources/sources-page.test.tsx`
- `tests/e2e/story-2-2-rss-atom-source.spec.ts`
- `scripts/windows-rss-smoke.ps1`
- `scripts/windows-demo-smoke.ps1`
- `scripts/windows-demo-aggregate.ps1`
- `_agentic-out/tests/evidence/story-2-2-cold-start-recovery.json`
