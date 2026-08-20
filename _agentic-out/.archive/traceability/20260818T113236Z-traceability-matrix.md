---
stepsCompleted:
  - step-01-load-context
  - step-02-discover-tests
  - step-03-map-criteria
  - step-04-analyze-gaps
  - step-05-gate-decision
lastStep: step-05-gate-decision
lastSaved: '2026-08-18T20:10:00+08:00'
coverageBasis: acceptance_criteria
oracleConfidence: high
oracleResolutionMode: formal_requirements
oracleSources:
  - '_agentic-out/implementation/stories/2-5-执行-rss-atom-同步并判断当前来源就绪.md'
  - '_agentic-out/planning/epics.md'
externalPointerStatus: not_used
collectionMode: contract_static
gateType: story
deliveryScope: windows-first-rss-minimum-loop
---

# Story 2.5 Requirements-to-Tests Traceability Matrix

## Step 1：覆盖依据与上下文

- 覆盖依据：Story 2.5 的 5 项正式验收标准（AC1–AC5），优先于任何推断旅程。
- Oracle 置信度：高。Story 已明确收缩为 Windows + shared core 的 RSS/Atom 最小闭环，并逐项给出 Given/When/Then。
- 范围：持久同步任务、恢复与幂等、RSS-only readiness、非阻塞执行、有界轮询和失败/重试一致性。
- Deferred：GitHub Release、arXiv、Apple/Android、native GUI 及最终冻结候选的 30 次启动样本。
- 外部需求指针：未使用；本地 Story 与 Epic 已足够。
- HTTP/OpenAPI/Auth：无入站 HTTP API、OpenAPI 或账号权限模型；适用边界是 core-owned RSS transport 和精确 Tauri IPC。
- 相关证据：Story 2.5 代码审查 PASS、自动化去重结论“无新增测试缺口”、测试质量 98/A。

### 验收项优先级

| ID | Priority | 验收目标 |
| --- | --- | --- |
| AC1 | P0 | 单源/全部同步创建持久、可查询、幂等任务并复用安全抓取/checkpoint |
| AC2 | P0 | queued 到各终态的任务与来源状态准确、可恢复 |
| AC3 | P1 | 第一阶段 readiness 只认 RSS/Atom 并区分来源运行状态 |
| AC4 | P0 | 网络/解析/持久化不阻塞 UI，轮询与 30 秒预算有界 |
| AC5 | P0 | 成功、部分失败、限流、格式/网络失败保持配置、checkpoint 与任务一致 |

## Step 2：测试清单与覆盖启发式

### 直接相关清单

| Level | 文件 | Story 2.5 直接测试 | 主要信号 |
| --- | --- | ---: | --- |
| Unit | `crates/radar-core/src/application/sync.rs` | 2 | 历史有界清理、事务原子回滚 |
| Integration | `crates/radar-core/tests/sync_tasks.rs` | 13 | start/claim/commit、幂等、恢复、readiness、多任务与 Retry-After |
| Unit/Integration | `apps/windows/src-tauri/src/commands/mod.rs` | 3 个同步专属 + 既有 panic seam | fetch 不持锁、budget cancel/race、失败终结 |
| Contract | `apps/windows/src-tauri/tests/release_surface.rs` | 2 | 精确 IPC handler 与本地 CSP/capability |
| Component/Transport | `apps/windows/src/lib/desktop-api/tauri-desktop-api.test.ts` | 4 个同步专属 | 三命令、identity、严格 lifecycle/health guard、timeout intent |
| Component | `apps/windows/src/features/sources/sources-page.test.tsx` | 10 | all/single、partial、deadline、revision、轮询、焦点/滚动 |
| E2E | `tests/e2e/story-2-5-rss-sync.spec.ts` | 4（2 P0、2 P1） | all/single、reload 恢复、partial、Retry-After |
| Contract mutation | `crates/xtask/tests/generated_contract_gate_negative.rs` | 相关 gate 用例 | 未批准 command/危险 surface 不能进入 release |

所有发现的 Story 2.5 测试均非 skipped/pending/fixme。Rust 临时 DB 使用 scoped path + RAII；Playwright 每测试新 context，fixture state 逐测试重建。

### Coverage heuristics

| Heuristic | 结果 | 说明 |
| --- | --- | --- |
| 入站 HTTP/OpenAPI endpoint | N/A | 产品无服务端 HTTP endpoint；RSS 是 core-owned 出站 transport |
| Tauri IPC boundary gaps | 0 | `start_sync_v1`、`task_v1`、`sync_health_v1` 均有 allowlist、Rust helper、TS guard/adapter 证据 |
| Auth/authz negative paths | N/A | 第一阶段没有账号、session、role 或权限门槛 |
| Happy-path-only criteria | 0 | AC1–AC5 均有冲突、失败、恢复、deadline 或部分成功的直接负路径 |
| UI journey gaps | 0 | all/single、reload、partial、Retry-After 四条核心旅程均有 E2E |
| UI state gaps | 0 | queued/running/retry_wait/terminal、partial、disabled reason、late revision、timeout retry 均有组件证据 |

## Step 3：AC→测试矩阵

| ID | Priority | Direct evidence | Levels | Error/alternate state | Coverage |
| --- | --- | --- | --- | --- | --- |
| AC1 | P0 | `start_is_idempotent_conflicting_payload_is_rejected_and_duplicate_active_is_blocked` (`sync_tasks.rs:118`); `all_sync_isolates_preexisting_retry_after_and_runs_other_sources` (`:430`); DesktopApi exact-command test (`tauri-desktop-api.test.ts:780`); all/single E2E (`story-2-5-rss-sync.spec.ts:21`, `:53`) | Integration, Transport, Component, E2E | 同 intent 重放、异 payload 冲突、活跃任务去重、单源隔离 | **FULL** |
| AC2 | P0 | success/readiness consistency (`sync_tasks.rs:150`); reopen running/queued recovery (`:332`, `:363`); multi-task health (`:521`); reload task recovery E2E (`story-2-5-rss-sync.spec.ts:21`) | Integration, Component, E2E | interrupted running/queued、late lower revision、partial terminal | **FULL** |
| AC3 | P1 | fresh RSS-only readiness (`sync_tasks.rs:78`); success/retry/failed/disabled 聚合路径 (`:150`, `:195`, `:430`, `:521`); component readiness/status 文案 (`sources-page.test.tsx:177`, `:244`, `:309`, `:499`) | Integration, Component | 未配置、同步中、限流/待重试、失败、停用与可用 | **FULL** |
| AC4 | P0 | fetch seam 不持 store mutex (`commands/mod.rs:769`); hard budget cancellation (`:811`); deterministic budget race (`:841`); adaptive bounded polling (`sources-page.test.tsx:472`); focus/scroll continuity (`:571`) | Unit/Integration, Component | budget 到期取消、远近 Retry-After 间隔、终态停止轮询 | **FULL** |
| AC5 | P0 | partial success + retry_wait (`sync_tasks.rs:195`); internal worker failure preserves source projection (`:294`); revision drift (`:386`); projection failure atomic rollback (`sync.rs:1338`); partial E2E (`story-2-5-rss-sync.spec.ts:77`); Retry-After E2E (`:126`) | Unit, Integration, Component, E2E | rate limit、格式/网络/internal、事务失败、部分成功与 retry deadline | **FULL** |

跨层重复是有意的：Rust 证明状态机/SQLite 原子性，Tauri/TS 证明 IPC 和 fail-closed 合同，Component/E2E 证明用户可见状态与 intent；它们验证不同故障边界，不是同源重复。

## Step 4：缺口分析与统计

- FULL：5/5（100%）；PARTIAL/NONE/UNIT-ONLY：0。
- P0：4/4 FULL（100%）；P1：1/1 FULL（100%）。
- 去重直接证据：38 cases / 8 files；skipped/fixme/pending：0。
- Critical/High requirement gaps：0；HTTP 与 Auth 明确为 N/A，不能误写为“已测试 HTTP/Auth”。
- Error-path gaps：0；UI journey gaps：0。
- UI 状态显式断言加固项：4 个 LOW（loading、not_configured、整任务 failed、cancelled）。对应 core/Tauri 状态机和生产文案已经存在，缺的是逐条 UI 文案断言，因此不把 AC 降级为 PARTIAL。
- 建议：进入 gate decision；四项 UI 状态断言可在后续集中整理测试 fixture 时补齐，不触发本阶段再次完整回归。

## Step 5：质量门决定

### Gate Decision：PASS

**Rationale:** P0 coverage 100%，P1 coverage 100%（目标 90%），overall coverage 100%（最低 80%）；无 critical/high requirement gap，collection 状态为 `COLLECTED`，正式 Story oracle 置信度为 high。

| Gate criterion | Actual | Status |
| --- | ---: | --- |
| P0 coverage（required 100%） | 4/4 = 100% | MET |
| P1 coverage（target 90%，minimum 80%） | 1/1 = 100% | MET |
| Overall coverage（minimum 80%） | 5/5 = 100% | MET |

### 证据边界

- 本 Gate 批准的是 Story 2.5 的 Windows RSS/Atom 最小闭环，不代表 GitHub Release、arXiv、移动端或三来源完成。
- native GUI 和冻结候选 30 次启动样本按既定决定延后至第一阶段 release candidate；它们不属于 AC1–AC5 当前门禁。
- 独立 MSVC `release_surface.exe` 当前哈希的真实执行仍受 Windows Application Control（OS 4551）阻止；源码级 release allowlist、xtask mutation gate、Tauri helper 与全套 Rust/TS/E2E 证据均已通过。该外部环境限制不改变本次覆盖门判定，但发布候选冻结时必须在允许执行的隔离环境补证。
- 四个 UI 状态文案断言为 LOW 加固项；底层状态与生产映射已有证据，不构成当前 requirement gap。

### Machine-readable outputs

- `_agentic-out/tests/reports/e2e-trace-summary.json`
- `_agentic-out/tests/reports/gate-decision.json`

生命周期终态确认：automation 与 Story `done` 状态登记后，AC、测试清单和 gate 输入均未变化，PASS 结论继续有效。
