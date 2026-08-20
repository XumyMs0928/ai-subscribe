---
artifact_kind: story
status: done
delivery_profile: standard
delivery_scope: windows-first-rss-minimum-loop
source_story: '2.5'
scope_override: '当前只执行 RSS/Atom；GitHub Release、arXiv 与三来源门禁 Deferred'
blocking_condition: ''
non_waivable_gates:
  - persistent_task_idempotency_and_recovery
  - rss_source_isolation_and_retry_deadline
  - release_surface_and_project_isolation
---

# Story 2.5：执行 RSS/Atom 同步并判断当前来源就绪

状态：review

## Story

作为日常使用者，  
我希望手动同步当前设备全部或单个 RSS/Atom 来源，并清楚看到每个来源的独立状态，  
以便判断数据是否新鲜以及 Windows RSS 最小闭环是否真正就绪。

## 验收标准

### AC1：单源与全部 RSS/Atom 同步创建持久任务

**Given** 当前设备已配置一个或多个 RSS/Atom 来源  
**When** 用户触发“同步全部”或某个来源的“立即同步”  
**Then** 核心创建持久、可查询且幂等的 `TaskRefV1`，使用 Story 2.2 的安全抓取与增量 checkpoint 执行同步  
**And** 单源只影响目标来源；“全部”只包含当前设备已启用的 RSS/Atom 来源，一个来源失败不回滚其他来源。

### AC2：任务和来源级状态准确、可恢复

**Given** 同步任务处于 queued、running、retry_wait、succeeded、partially_succeeded、failed 或 cancelled  
**When** 用户查看来源页或同步状态  
**Then** `SyncHealthSummaryV1` 显示任务状态、最后成功时间、来源结果、待处理任务和数据新鲜度  
**And** 部分成功同时保留成功来源与失败范围，不折叠为整体失败，不用错误覆盖可用来源。

### AC3：第一阶段就绪只认 RSS/Atom

**Given** 当前设备 RSS/Atom 来源处于不同配置或运行状态  
**When** 核心计算 `SourceDeliveryReadinessV1`  
**Then** required source 固定为 `rss_atom`，来源实例显示未配置、可用、同步中、限流、失败、停用或待重试  
**And** 只有 RSS/Atom 达到候选条件时才显示“Windows RSS 最小闭环就绪”；不得显示“三来源已就绪”，不得把 GitHub Release/arXiv 当失败或阻塞。

### AC4：同步不阻塞界面且轮询有界

**Given** 用户在同步过程中继续浏览、滚动或编辑设置  
**When** 网络、解析和持久化异步执行  
**Then** 网络等待不持有 SQLite mutex/事务，UI 操作不被同步调用阻塞  
**And** Windows 只在任务活跃时按稳定 task/revision 自适应轮询，终态立即停止；重复点击不创建重复有效任务。

### AC5：前台失败与重试遵守既有策略

**Given** RSS/Atom 同步成功、部分失败、rate limited、格式失败或网络失败  
**When** 任务提交状态  
**Then** 配置、checkpoint、任务状态和最后成功时间保持一致，失败精确归属来源  
**And** `next_allowed_at` 未到时禁止提前请求；本 Story 只提供 Windows 前台手动同步/重试，不实现计划同步、后台常驻或移动 ExecutionBudget。

## 第一阶段边界

- 只实现共享 Rust core、Windows Tauri/DesktopApi 和 Windows UI；Apple/iPhone/iPad/Android Deferred/N/A。
- 唯一真实 adapter 是已完成的 RSS/Atom。不得创建 GitHub Release、arXiv 或 discovery adapter，不得显示占位入口、假失败、假就绪。
- Story 2.6 才持久化/展示本轮 inserted、updated、skipped、failed 计数和最小结果；本 Story 的任务终态只保存来源级成功/失败摘要和状态引用，不提前建结果事实模型。
- Story 4.1 才把真实候选规范化为最终情报；本 Story 不把 checkpoint 当情报列表。
- 不实现 AI、通知、评分、搜索、收藏、后台计划、托盘、通用 HTTP/SQL/file/shell command 或第二数据库。
- 完成结论只能是 `Story 2.5 Windows RSS milestone PASS/FAIL`。
- 所有依赖、缓存和测试服务位于项目目录；禁止全局安装 Python/Node/Rust，禁止修改系统代理、证书、主题、缩放或权限。

## Tasks / Subtasks

- [x] Task 1：定义 RSS-only 持久任务与健康合同（AC1–AC3、AC5）
  - [x] 在 `crates/radar-core/src/contracts/dto/` 定义 `StartSyncInputV1`、`TaskRefV1`、`TaskSnapshotV1`、`SyncHealthSummaryV1`、`SourceSyncStatusV1`、`SourceDeliveryReadinessV1`。公开 DTO 均有 `contract_version=1`，JSON `snake_case`，revision 为安全整数，时间 RFC3339 UTC，可选字段显式 `null`。
  - [x] `StartSyncInputV1` 只接受 `target = { all_enabled_rss_atom | source_id }`、有界 idempotency key 和显式前台预算；平台不能传 adapter 名单、URL、SQL 或 generic command。
  - [x] `TaskStateV1` 固定为 queued/running/retry_wait/succeeded/partially_succeeded/failed/cancelled；终态不可逆。`partially_succeeded` 仅在同一 all-sync 中至少一个来源提交成功且至少一个来源失败/待重试时出现。
  - [x] readiness 的 required kinds 固定只含 `rss_atom`；不得返回 GitHub/arXiv 占位。无已启用来源时为 not_configured；有活跃任务时 syncing；所有已启用 RSS 来源最近一次可用且无阻塞任务时 ready；失败/rate-limit 保留来源级状态。
  - [x] 同步 contract manifest、error snapshot、radar-ffi 当前共享映射（仅现有 gate 所需）、Tauri allowlist、TypeScript exact guards、test factory/mock 和 xtask mutation gate。移动生成绑定和运行面不创建。

- [x] Task 2：SQLite v6 持久任务与确定性状态机（AC1–AC3、AC5）
  - [x] 从当前 v5 单调迁移到 v6，继续使用 `DemoStore::from_connection` 唯一 migration runner/verifier。fresh、v1/v2/v3/v4/v5→v6 一致并保留 demo/FTS/setup/configuration/sources/checkpoints；未来版本、缺表列约束、非法状态 fail closed。
  - [x] 只新增 `jobs` 和最小 `job_source_states`（或同等规范表）。`jobs` 至少包含 task_id、kind=`rss_atom_sync`、target、state、revision、idempotency fingerprint、created/started/finished/updated、error summary；来源子状态绑定 `source_id`、source revision、state、last success/error/retry time。不得提前创建 Story 2.6 的 `sync_runs/sync_source_results/sync_result_items`。
  - [x] idempotency：同 key 同 payload重放权威 `TaskRefV1`；同 key 异 payload稳定 conflict。每个来源同一时刻最多一个有效 queued/running/retry_wait 同步；重复点击不得发第二次请求。
  - [x] 原子 claim queued→running；每次状态转换使用 expected revision/compare-and-set。成功/失败提交 source checkpoint 和 job source state 时保持可恢复顺序，不让 job 宣称成功而来源事务未提交。
  - [x] 启动恢复：进程中断留下的 running 不伪装继续运行；重开时确定性转为可前台恢复/失败状态。V1 不自动后台续跑，用户可见后按同一 task 或明确重试 intent 恢复。
  - [x] idempotency/历史有界清理必须只清终态旧记录，不删除仍被查询或活跃任务引用的行。

- [x] Task 3：复用 Story 2.2 完成 RSS/Atom 编排（AC1、AC4–AC5）
  - [x] 新增 application orchestration seam（建议 `application/sync.rs`），复用 `prepare_incremental_fetch` → async `fetch_incremental` → `commit_incremental_fetch`/`commit_incremental_failure`。禁止复制 HTTP policy、parser、retry 或 checkpoint 逻辑。
  - [x] 网络阶段不持有 `DemoState` store mutex/SQLite transaction：短锁读取 request，释放后 await fetch，再短锁 CAS 提交。来源 revision 变化时稳定 conflict，不用旧响应覆盖新配置。
  - [x] “同步全部”在开始时冻结已启用 RSS/Atom source IDs 的有界快照；逐来源独立执行并提交。一个来源错误不取消其他来源，除非整个任务显式取消/预算耗尽。
  - [x] rate-limit 在 `next_allowed_at` 前不访问网络；状态为 retry_wait。source-format 永不自动重试；network 按 Story 2.2 非递减退避。V1 的手动重试也不得绕过尚未到期的服务端 Retry-After。
  - [x] 任务 panic/JoinError 映射稳定脱敏 AppError，附 task_id/source_id；不得持久化 URL query、标题、正文、payload 或底层网络错误。

- [x] Task 4：接通精确 Tauri/DesktopApi 命令（AC1–AC5）
  - [x] 新增且只新增 `start_sync_v1(input)`、`task_v1(task_id)`、`sync_health_v1()` 三个异步命令；更新 `generate_handler!`、release_surface 和 xtask allowlist。禁止 generic execute、fetch URL 或任意 task kind。
  - [x] Tauri 使用现有 runtime 驱动 async fetch，不另建 Tokio runtime；SQLite 工作继续进入受控 blocking seam。应用关闭时不让 detached worker冒充持久后台能力。
  - [x] 扩展唯一 `DesktopApi`/`tauri-desktop-api.ts`，对 exact keys、版本、状态组合、task/source identity、revision、时间和 readiness fail closed；读取命令沿用 10 秒 IPC timeout，start 只等待任务持久创建，不等待 30 秒网络完成。
  - [x] 重试 start 使用同一个 intent 的 idempotency key；timeout 后不得自动生成新 key。late task/health response 不覆盖更高 revision。

- [x] Task 5：在 Windows 来源页完成手动同步和有界轮询（AC1–AC5）
  - [x] 扩展 `apps/windows/src/features/sources/`：页面级“同步全部 RSS/Atom”，每个 enabled 来源“立即同步”；disabled、active、retry_wait-before-deadline 时按钮明确禁用并解释原因。
  - [x] 用 TanStack Query 管 task/health；集中 `syncKeys`。mutation 成功后查询 task；仅 queued/running/retry_wait 活跃且允许前台观察时轮询，终态或卸载立即停止。轮询相同 revision 不重置列表/焦点/滚动。
  - [x] 呈现 loading、not_configured、ready、syncing、retry_wait、partial、failed、stale/recoverable 状态，均有文字而非只靠颜色；来源错误局部化，保留其他来源和现有列表。
  - [x] 页面固定说明“当前仅 RSS/Atom；只影响此 Windows 设备”。不得出现 GitHub Release、arXiv、“三来源已就绪”、计划同步或后台常驻承诺。
  - [x] 同步期间添加来源/浏览/滚动保持可操作；不对来源/任务做乐观成功写入。

- [x] Task 6：一次性形成自动化与可追溯证据（AC1–AC5）
  - [x] Rust：迁移 fresh/v1–v5→v6、损坏 schema、idempotent replay/conflict、CAS/终态、重复 start、单源隔离、all-sync 部分成功、retry deadline、panic/reopen recovery、成功/失败后 checkpoint 与任务一致。
  - [x] 使用 injected Resolver/Connector/Clock 固定响应；不访问公网、不真实等待。证明 network await 时无 SQLite 写事务/全局 mutex，多个来源结果互不回滚。
  - [x] Tauri/DesktopApi：三个精确 command、panic containment、unknown/contradictory DTO、timeout/idempotency、release allowlist；React/Vitest覆盖按钮、状态、局部错误、轮询开始/停止、迟到 revision、焦点/滚动保留及 RSS-only 文案。
  - [x] Playwright 只验证 UI/DesktopApi seam：单源、全部、部分失败、retry_wait、reload 后任务状态、external calls=0；不得冒充 Rust 网络/SQLite 证据。
  - [x] 集中完成所有修复后只跑一次相关 gate：project-local rustfmt/Clippy/Rust tests/xtask、frontend format/lint/typecheck/Vitest/Playwright/build。失败后只从失败 gate 续跑。
  - [x] 本 Story 不启动 native GUI、不重复 30 次性能采样；真实 native smoke + 30-sample 只在第一阶段最终 release candidate 冻结后统一执行一次。

### Review Findings

- [x] [Review][Patch] 解除到期 `retry_wait` 对来源的永久占用，并允许到期后由明确重试 intent 接管 [`crates/radar-core/src/application/sync.rs:558`]
- [x] [Review][Patch] 修复 all-sync 的 Retry-After 预检：不得因单个受限来源阻断其他可运行来源，且必须检查全部未来 deadline [`crates/radar-core/src/application/sync.rs:72`]
- [x] [Review][Patch] 恢复孤立 queued/running 任务，并以单事务保持父任务与来源子状态一致 [`crates/radar-core/src/application/sync.rs:451`]
- [x] [Review][Patch] 真正执行 30 秒前台预算，为未执行来源留下可恢复且真实的终态 [`apps/windows/src-tauri/src/commands/mod.rs:447`]
- [x] [Review][Patch] 不再丢弃 detached worker 的 claim/commit/storage 错误，确保任务不会永久停在 running [`apps/windows/src-tauri/src/commands/mod.rs:432`]
- [x] [Review][Patch] 补偿 checkpoint 与 job projection 的分事务崩溃窗口，禁止来源已提交但任务永久不一致 [`crates/radar-core/src/application/sync.rs:276`]
- [x] [Review][Patch] claim 时校验启动时冻结的 source revision，配置变化必须显式冲突而非静默同步新版本 [`crates/radar-core/src/application/sync.rs:205`]
- [x] [Review][Patch] 健康投影和轮询覆盖全部活跃任务/来源，不得只看 latest task 或被旧 observed task 遮蔽 [`crates/radar-core/src/application/sync.rs:387`]
- [x] [Review][Patch] 同步按钮使用完整 active/pending 状态，避免 UI 发出后端必然拒绝的重复请求 [`apps/windows/src/features/sources/sources-page.tsx:249`]
- [x] [Review][Patch] DesktopApi 对任务聚合状态以及 health/latest/source_results 的矛盾载荷 fail closed [`apps/windows/src/lib/desktop-api/desktop-api.ts:953`]
- [x] [Review][Patch] 实现终态任务与幂等历史的有界清理，且绝不删除活跃或仍被引用的记录 [`crates/radar-core/src/application/sync.rs:45`]
- [x] [Review][Patch] 补齐真实编排、预算、Retry-After 混合来源、恢复、锁释放及同步期间焦点/滚动证据，修正已勾选但缺证据的 Task 6 [`crates/radar-core/tests/sync_tasks.rs:75`]

## Requirement Change Log

### RCL-001：第一阶段收缩为 Windows RSS/Atom 最小闭环

- **Trigger:** 用户要求“把三来源收缩为当前已实现的 RSS/Atom 来执行”，随后批准并继续 Story 2.5。
- **Classification:** Cross-artifact Change。
- **Previous behavior:** 规划要求 RSS/Atom、GitHub Release、arXiv 三来源共同参与同步、就绪和第一阶段门禁。
- **New behavior:** 第一阶段只实现和验收 Windows + shared core 的 RSS/Atom 单源/全部同步、任务恢复、来源状态与 RSS-only readiness；GitHub Release、arXiv、移动端和三来源门禁保留为 Deferred。
- **Acceptance Criteria affected:** AC1–AC5 及“第一阶段边界”。
- **Tasks affected:** Task 1–Task 6。
- **Upstream artifacts affected:** `epics.md`、`ux/EXPERIENCE.md`、`ux-design-specification.md`、`sprint-status.yaml`、`artifacts.yaml`、`sprint-change-proposal-2026-08-18.md`。
- **Tests required:** Rust 持久任务/迁移/幂等/恢复/Retry-After/部分成功；Tauri/DesktopApi 精确命令和 fail-closed；Vitest 状态与轮询；Playwright RSS-only UI seam；release allowlist 与 xtask contracts。
- **Approval evidence:** 用户于 2026-08-18 明确提出范围收缩，并在 Correct Course 后回复“继续”批准执行。
- **Status:** applied。

### RCL-002：代码审查要求补强同步生命周期一致性

- **Trigger:** 2026-08-18 Story 2.5 一次性完整代码审查发现 retry_wait/queued 恢复、预算执行、多任务投影、detached error、source revision 和持久化一致性缺口。
- **Classification:** Implementation Correction。
- **Previous behavior:** 部分异常路径可留下永久 active 任务、忽略 30 秒预算、让 all-sync 被单个 Retry-After 整体阻断，或让 UI/DTO 接受不完整聚合状态。
- **New behavior:** 在不改变 RSS-only AC 和范围的前提下，补齐确定性恢复、预算终止、来源隔离、多任务健康投影、严格 DTO 校验、有界历史和相应自动化证据。
- **Acceptance Criteria affected:** AC1、AC2、AC4、AC5。
- **Tasks affected:** Task 2、Task 3、Task 4、Task 5、Task 6 及 Review Findings。
- **Upstream artifacts affected:** 无；保持已批准的 RSS-only Correct Course 不变。
- **Tests required:** 每个 Review Finding 至少一个定向回归；集中修复后只运行相关合并门禁，失败仅续跑失败 gate。
- **Approval evidence:** 用户要求“一次性完整审查”并在 Step 1 checkpoint 后回复“继续”。
- **Status:** applied。

## Dev Notes

### 必须复用

- `crates/radar-core/src/application/sources.rs`：prepare/fetch/commit/failure/retry/checkpoint 唯一实现。
- `crates/radar-core/src/infrastructure/http/source_http_policy.rs`：SSRF、DNS pin、TLS、redirect、10MB、30s、Retry-After 唯一策略。
- `crates/radar-core/src/infrastructure/sources/rss_atom/` 与 `domain/sources/`：唯一 parser/candidate 模型。
- `crates/radar-core/src/application/demo.rs`：唯一 SQLite owner、migration runner、schema verifier。
- `apps/windows/src-tauri/src/commands/mod.rs::DemoState`：现有 store/executor/panic containment。
- `apps/windows/src/lib/desktop-api/`、`lib/query-client.ts`、`features/sources/`：唯一 Windows transport/query/UI seam。

### 关键设计决定

- `start_sync_v1` 只持久创建任务并立即返回，不等待同步完成。
- 网络 future 由现有 Tauri runtime 驱动；core 拥有任务语义和 SQLite 状态，不新建 runtime。
- 第一阶段 all-sync = 当前设备已启用 RSS/Atom 来源集合，不是三种 adapter。
- 任务状态不是结果事实模型；2.6 才新增来源计数和最小结果表。
- 不新增依赖；沿用锁定 Rust 1.97.1、Tauri 2.11.5、reqwest 0.13.4、Tokio 1.53.1、rusqlite 0.40.1/SQLite 3.53.4、React 19.2.8、TanStack Query 5.101.4、TypeScript 5.9.3、Vitest 4.1.10、Playwright 1.62.1。

### 防回归

- Story 2.2 的 production SSRF policy、来源保存/查询、v5 数据和 126/126 Rust + 81/81 Vitest + 23/23 Playwright 基线不得退化。
- 不把 `sources.status` 同时当 task state；task/source 是不同生命周期，通过 task-source projection关联。
- 不用源码字符串/注释伪造 release handler 检查；门禁必须解析/严格匹配批准命令。
- 不使用系统时间断言、硬等待、固定共享 DB 路径或真实公网。

### Project Structure Notes

- 新 DTO：`crates/radar-core/src/contracts/dto/sync.rs`。
- 新编排：`crates/radar-core/src/application/sync.rs`；数据库 plumbing 可沿现有 DemoStore impl，但不得把网络 future 放进 DB 模块。
- Windows UI 继续就近位于 `apps/windows/src/features/sources/`，不新建第二 sources service/store。
- 共享 fixture 放 `contracts/fixtures/`；平台测试仅引用/映射，不复制业务 fixture。

### References

- [第一阶段覆盖与 Story 2.5 AC](D:/2026/TEST1/_agentic-out/planning/epics.md#story-25执行-rssatom-同步并判断当前来源就绪)
- [RSS-only Correct Course](D:/2026/TEST1/_agentic-out/planning/sprint-change-proposal-2026-08-18.md)
- [第一阶段架构](D:/2026/TEST1/_agentic-out/planning/architecture.md#第一阶段激活架构windows-rss-最小闭环)
- [前序 Story 2.2](D:/2026/TEST1/_agentic-out/implementation/stories/2-2-订阅并增量同步-rss-atom-来源.md)

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- 2026-08-18：集中实现后执行一次自动化门禁；只从失败 gate 续跑。MSVC `release_surface.exe` 最终哈希被 Windows 应用控制策略阻止执行（OS 4551），未修改系统策略；同一门禁在本轮较早构建 1/1 通过，当前 `xtask contracts` 通过。
- 2026-08-18：一次性完整代码审查的 12 项 patch 已全部修复。只续跑失败门：Vitest 异步等待 fixture、Rust 过期时间 fixture 和 contracts 测试边界均已定向纠正。

### Completion Notes List

- Ultimate context engine analysis completed - comprehensive developer guide created.
- RSS/Atom-only 持久任务、SQLite v6 状态机、单源/全部同步、任务恢复和就绪投影已实现；未创建 Story 2.6 结果表，未增加 GitHub Release/arXiv 入口。
- 修复审查发现：worker panic 使用专用 internal-failure 路径，不再卡住 running，也不污染 source/checkpoint；retry_wait 使用有界自适应轮询。
- 代码审查后通过 rustfmt、workspace Clippy `-D warnings`、core unit 36/36（35/35 + 新增原子回滚 1/1）、sync_tasks 13/13、Tauri lib 6/6、xtask contracts、前端 format/lint/typecheck、Vitest 95/95 绿色证据、build 与 Playwright 27/27。
- 未启动原生 GUI、未进行 30 次采样、未修改全局 Python/Node/Rust 或系统设置。

### File List

- `contracts/schemas/contract-manifest-v1.json`
- `crates/radar-core/src/application/{demo,mod,sync}.rs`
- `crates/radar-core/src/contracts/dto/{mod,sync}.rs`
- `crates/radar-core/src/contracts/manifest.rs`
- `crates/radar-core/tests/sync_tasks.rs`
- `apps/windows/src-tauri/src/{commands/mod,lib}.rs`
- `apps/windows/src-tauri/tests/release_surface.rs`
- `crates/xtask/src/contracts.rs`
- `crates/xtask/tests/generated_contract_gate_negative.rs`
- `apps/windows/src/lib/desktop-api/{desktop-api,tauri-desktop-api,tauri-desktop-api.test}.ts`
- `apps/windows/src/lib/query-client.ts`
- `apps/windows/src/features/sources/{sources-page,sources-page.test,sync-queries}.tsx/ts`
- `apps/windows/src/features/{configuration-validation,demo-intelligence,setup-guide}/*test.tsx`
- `tests/support/factories/demo-dto.factory.ts`
- `tests/support/fixtures/{demo-app.fixture,index}.ts`
- `tests/support/helpers/tauri-command-mock.ts`
- `tests/e2e/story-2-{2-rss-atom-source,5-rss-sync}.spec.ts`

### Change Log

- 2026-08-18：完成 Story 2.5 Windows RSS/Atom-only 实现并进入 review。
