---
stepsCompleted:
  - step-01-load-context
  - step-02-discover-tests
  - step-03-map-criteria
  - step-04-analyze-gaps
  - step-05-gate-decision
lastStep: step-05-gate-decision
lastSaved: 2026-08-20T16:33:00+08:00
coverageBasis: acceptance_criteria
oracleConfidence: high
oracleResolutionMode: formal_requirements
oracleSources:
  - _agentic-out/implementation/stories/4-5-浏览高价值主情报流并组合筛选.md
  - _agentic-out/planning/epics.md
  - _agentic-out/planning/prd.md
externalPointerStatus: not_used
tempCoverageMatrixPath: _agentic-out/tests/reports/testing-trace-coverage-matrix-2026-08-20-story-4.5.json
---

# Story 4.3 可追溯性矩阵

## Step 1 — Coverage Oracle

- 权威 oracle：Story 4.3 的 AC1–AC6，并以 Epic/PRD 的 FR16/FR19–FR22、NFR8/12/13 作交叉核对。
- 解析模式：formal requirements；信心 high；无外部 tracker 指针。
- 交付边界：shared Rust core + SQLite 的 Windows RSS minimum-loop；无新 HTTP、auth、Tauri IPC、route 或 UI。
- 优先级：AC1–AC5 为 P0（规则正确性、数据完整性与原子性），AC6 为 P1（确定性、迁移与平台边界）。

## Step 2 — 稳定证据目录

### Unit / domain

- T43-U01–T43-U10：`crates/radar-core/tests/intelligence_value.rs:76-342`，覆盖确定 high-value、硬门控、threshold/importance 边界、freshness、source alias/track、六类 technical impact、disabled/untrusted/inactive source、Unicode include/exclude、invalid config/context。
- T43-CFG01：`crates/radar-core/tests/configuration_validation.rs:264`，RSS alias 风险语义与 canonical duplicate identity。

### SQLite integration

- T43-INT01：`crates/radar-core/src/application/sync.rs:2296`，fact update 保留 provenance 并刷新 rule projection。
- T43-INT02：`sync.rs:2333`，rule write failure 回滚整个来源事务。
- T43-INT03：`sync.rs:2386`，configuration save 原子重评且不改写 facts。
- T43-INT04：`sync.rs:2473`，configuration recompute failure 回滚新版本。
- T43-INT05：`sync.rs:2548`，单来源 rule failure 不回滚其他已提交来源。
- T43-INT06：`sync.rs:2609`，unchanged replay 在同 rule bucket 零 rule writes。

### Migration / verifier

- T43-MIG01：`crates/radar-core/src/application/demo.rs:3217`，v9 real facts 确定 backfill v10，保留 demo rows。
- T43-MIG02：`demo.rs:3290`，partial v10 extension fail-closed 且不推进 v9。
- T43-MIG03：`demo.rs:3324`，重启拒绝损坏 current-rule JSON。
- T43-MIG04：`demo.rs:3356`，重启拒绝伪造 rule semantics 与缺失 provenance。

### Contract drift

- T43-CON01：`crates/xtask/tests/generated_contract_gate_negative.rs:97`，生产 xtask 拒绝语义 contract drift；当前 gate 执行 Story 4.3 RSS rule golden 全部7个 scenario。

### Heuristics inventory

- HTTP endpoint：N/A（无 inbound HTTP/OpenAPI）。
- Tauri IPC/DesktopApi：N/A（本 Story 无新 command/transport）。
- Auth/authz：N/A（本地单设备规则投影，无 login/role/tenant 边界）。
- UI journey/state：N/A（本 Story backend-only，4.5/4.6 负责可达界面）。
- Error paths：适用；已覆盖 invalid config/context、rule/config transaction failure、partial schema、corrupt JSON、forged semantics、missing provenance，当前 gap=0。
- Happy-path-only：适用；每个 AC 均有对应边界/失败证据，当前 gap=0。

## Step 3 — AC ↔ Test Matrix

| AC | Priority | Status | Direct evidence | 说明 |
| --- | --- | --- | --- | --- |
| AC1 | P0 | FULL | T43-U01, T43-U07, T43-INT01, T43-CON01 | 版本化五维结果、AI-independent、持久化与 golden drift gate 均经生产路径执行。 |
| AC2 | P0 | FULL | T43-U03, T43-U04, T43-U05, T43-U10, T43-CON01 | 25/25/20/20/10、49/50/79/80、freshness fallback/future 和 invalid context 边界完整。 |
| AC3 | P0 | FULL | T43-U02, T43-U06–T43-U10, T43-CFG01 | RSS alias/canonical source、track/trust/include/exclude/active window/HTTPS provenance 硬门控及六类 technical impact 均有正反证据。 |
| AC4 | P0 | FULL | T43-U02, T43-U03, T43-U04 | threshold 等号边界、importance 独立性、high/ordinary 分流和具体 filter reason 直接断言。 |
| AC5 | P0 | FULL | T43-INT01–T43-INT06 | new/changed/unchanged、来源事务、配置重评、失败回滚、多来源隔离和零重复写有 SQLite 集成证据。 |
| AC6 | P1 | FULL | T43-U01, T43-INT06, T43-MIG01–T43-MIG04, T43-CON01 | 固定输入字节级确定、replay/reopen、v10 backfill/fail-closed verifier 与 contract mirror 门完整。 |

### 去重说明

- 22个稳定直接 evidence IDs，分布在5个文件；同一 evidence 可支撑多个 AC，统计时只计1次。
- Unit 负责纯规则/边界，integration 负责 SQLite 事务/迁移，contract gate 负责镜像漂移；层级间验证的风险不同，非不必要重复。
- skipped=0、pending=0、fixme=0。symlink 环境条件分支不在 T43 直接 evidence inventory 中。

## Step 4 — Gap Analysis

- Requirements：6；FULL=6，PARTIAL=0，NONE=0，UNIT-ONLY=0，INTEGRATION-ONLY=0。
- Overall fully-covered：100%。P0=5/5（100%）；P1=1/1（100%）；P2/P3 无要求（N/A，safePct=100%）。
- 稳定证据：22 cases / 5 files；active=22，skipped=0，fixme=0，pending=0。
- 层级：unit=11 cases，覆盖5项 AC；other=11 cases（SQLite integration/migration/contract），覆盖4项 AC；E2E/API/component=N/A。
- P0/P1 gap=0；endpoint/auth/UI gap=N/A；适用的 error-path 和 happy-path-only gap=0。
- 建议：无需追加测试；继续执行 deterministic gate decision。

## Step 5 — Deterministic Gate Decision

- Gate：**PASS**。
- P0：5/5 FULL（100%），满足必须 100% 的门槛。
- P1：1/1 FULL（100%），满足目标 90% 的门槛。
- Overall：6/6 FULL（100%），满足最低 80% 的门槛。
- Blockers：0；适用的 error-path / happy-path-only gaps：0。
- HTTP、auth、Tauri IPC/DesktopApi 与 UI journey/state 对本 backend-only Story 为 N/A，不作为虚假覆盖计入。
- 结论限定为 **Windows RSS minimum-loop PASS**；4.5/4.6 的可达 UI 与详情解释仍是不可豁免的后续交付。

# Story 4.5 可追溯性矩阵

## Step 1 — Coverage Oracle

- 权威 oracle：Story 4.5 的 AC1–AC7，并以 Epic 4、PRD 与 UX spine 交叉核对；解析模式 `formal_requirements`，信心 `high`。
- Oracle sources：Story 4.5、`_agentic-out/planning/epics.md`、`_agentic-out/planning/prd.md`；无外部 tracker 指针，`externalPointerStatus=not_used`。
- 交付边界：Windows RSS/Atom minimum-loop 的 real high-value/ordinary feed、四维筛选、opaque cursor、状态/键盘与 50k×30×2 P95；Deferred 能力不进入覆盖分母。
- 优先级：AC1–AC5、AC7 为 P0；AC6 为 P1。AC7 属性能完成门，必须由显式 isolated runner 而非普通 `cargo test` 证明。

## Step 2 — 稳定证据目录

### Rust core / SQLite

- T45-C01–C05：`intel_feed.rs:484-630`，覆盖 current fact/content projection、无写读取、snapshot/future/late evaluation、HMAC cursor 防伪、稳定排序与 limit=100 跨页去重。
- T45-C06：`intel_feed.rs:671`，invocation-scoped 原子 evidence reservation。
- T45-C07：`intel_feed.rs:695`，50k 固定数据集 default/combined 各 30 samples P95；源码为 ignored，但唯一合法执行入口为仓库 runner。
- T45-I01–I02：`sync_tasks.rs:1069,1142`，真实 high/ordinary 不混 demo，四维筛选与 cursor/query identity 绑定。

### Transport / release contract

- T45-T01–T05：`intel-feed-transport.test.ts:61-131`，精确 command/page contract、矛盾 projection fail-closed、非法 ID/extra keys/filter 拒绝、opaque track、as-of/order/duplicate/limit 边界。
- T45-R01：`release_surface.rs:12`，release command table 只允许 canonical fixture 中的批准命令。
- T45-X01–X03：xtask production contract、negative drift 与 test-source import gate，共用 `approved-commands-v1.txt`。

### Component / E2E

- T45-U01–U12：`intel-feed.test.tsx:96-379`，high/ordinary、四维 input、键盘、loading/empty、分页失败/成功/漂移、cursor 恢复、表单验证、Enter/Esc、刷新焦点。
- T45-E01–E03：`story-4-5-intel-feed.spec.ts:8-146`，两流可达且无 external call、四维筛选上下文、cursor 分页与导航保持。

### 性能 automation evidence

- T45-P01：`scripts/story-4-5-performance-gate.ps1` 显式调用 `--ignored --exact`，跨进程锁、唯一新文件与 evidence contract 校验。
- T45-P02：`target/story-4-5/intel-feed-performance-35400-0.json`，50,000 items、30+30 samples、default P95 62.021ms、combined P95 100.734ms、阈值 200ms，并含 dataset/candidate/source hash 与两类 query plan。

### Coverage heuristics

- Inbound HTTP/Auth：N/A；本地 Tauri 应用无 HTTP endpoint、login/session/role 边界。
- Tauri IPC/DesktopApi：适用，T45-T01–T05、T45-R01、T45-X01–X03 直接覆盖。
- UI journey/state：适用，T45-U01–U12 与 T45-E01–E03 覆盖 happy、loading、empty、validation、pagination error/recovery；permission-denied N/A。
- Error paths：适用，cursor forgery/drift/expiry、projection contradiction、malformed DTO/filter、分页失败均有直接证据。
- Execution state：普通证据 skipped=0、pending=0、fixme=0；T45-C07 源码 `ignored=1` 是有意隔离的性能门，已由 T45-P01 显式执行并通过，不计覆盖缺口。

## Step 3 — AC ↔ Test Matrix

| AC | Priority | Status | Direct evidence | Heuristic validation |
|---|---|---|---|---|
| AC1 | P0 | FULL | T45-C01, T45-I01, T45-T01–T05, T45-R01/X01–X03 | real/current projection、high/ordinary、bounded DTO 与 release IPC 均有正反证据；HTTP/Auth N/A。 |
| AC2 | P0 | FULL | T45-I02, T45-C07, T45-U03, T45-E02 | 四维 AND/OR、稳定 identity/order、fixed-as-of Last7d 与 UI narrow input 均直接断言。 |
| AC3 | P0 | FULL | T45-C02–C05, T45-I02, T45-T02/T05, T45-U06–U09, T45-E03 | HMAC 防伪、snapshot、跨 identity/drift/expiry/limit 与 UI 恢复路径完整。 |
| AC4 | P0 | FULL | T45-U01–U04, T45-U10–U12, T45-E01 | 两流可达、可见非颜色状态、无假功能、stable identity seam 与真实 journey 有证据。 |
| AC5 | P0 | FULL | T45-U05–U09, T45-U12, T45-E03 | loading/empty、局部分页失败、漂移/失效恢复、刷新相邻选择与导航上下文；permission-denied N/A。 |
| AC6 | P1 | FULL | T45-U04, T45-U10–U12, T45-E01/E03 | Arrow/J/K、Enter/Esc、roving focus、文字 disposition 与可访问原生按钮定位；当前实现无 overlay。 |
| AC7 | P0 | FULL | T45-C06/C07, T45-P01/P02 | 显式 isolated runner、原子唯一证据、50k×30×2、两类 P95<200ms、hash/query plan/环境元数据均验证。 |

去重说明：core 负责规则、SQLite 与 cursor；transport/release 负责边界漂移；Vitest 负责状态机；Playwright 只验证三条必要用户旅程；性能 runner 负责普通套件不能证明的墙钟属性。层级间风险不同，不构成无意义重复。

## Step 4 — Gap Analysis

- Requirements：7；FULL=7，PARTIAL=0，NONE=0，UNIT-ONLY=0，overall=100%。
- P0：6/6 FULL（100%）；P1：1/1 FULL（100%）；P2/P3 无要求（N/A；machine `safePct=100`）。
- Heuristic gaps：endpoint=0、auth negative=0、happy-path-only=0、UI journey=0、UI state=0；HTTP/Auth/permission-denied 的 N/A 边界未误计为覆盖或缺口。
- Deduplicated inventory：33 cases / 9 files；active=33，skipped=0，fixme=0，pending=0。AC7 源测试虽默认 ignored，但由 T45-P01 以 `--ignored --exact` 显式执行且 T45-P02 为当前新鲜证据，因此记为 active isolated gate；runner/evidence 任一缺失或过期都会使 AC7 降级并形成 blocker。
- Gap recommendations：无 P0/P1/P2/P3 覆盖补测；仅保留低优先级 post-fix test-review 刷新。
- Durable Phase 1 matrix：`_agentic-out/tests/reports/testing-trace-coverage-matrix-2026-08-20-story-4.5.json`。

## Step 5 — Deterministic Gate Decision

- Collection：`COLLECTED`，`allow_gate=true`，因此 gate eligible。
- P0：6/6 FULL = 100%（required 100%，MET）。
- P1：1/1 FULL = 100%（PASS target 90%、minimum 80%，MET）。
- Overall：7/7 FULL = 100%（minimum 80%，MET）。
- Critical/high/heuristic gaps：0；active evidence cases=33；formal oracle confidence=high。
- **Gate decision：PASS**。Rationale：P0、P1 与 overall 均达到确定性阈值；AC7 的默认 ignored 状态由已执行的仓库 runner 和当前新鲜证据闭合，不以普通 suite 通过冒充。
- Machine outputs：`_agentic-out/tests/reports/story-4-5-e2e-trace-summary.json`、`_agentic-out/tests/reports/story-4-5-gate-decision.json`。
- 此 PASS 仅代表 Story 4.5 Windows RSS minimum-loop 的 traceability gate；仍需刷新 post-fix test-review/code-review 完成记录后才能满足 standard before_done。
