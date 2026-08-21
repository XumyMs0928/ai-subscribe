---
stepsCompleted: ['step-01-load-context', 'step-02-discover-tests', 'step-03-map-criteria', 'step-04-analyze-gaps', 'step-05-gate-decision']
lastStep: 'step-05-gate-decision'
lastSaved: '2026-08-15'
coverageBasis: 'acceptance_criteria'
oracleConfidence: 'high'
oracleResolutionMode: 'formal_requirements'
oracleSources:
  - '_agentic-out/implementation/stories/1-6-浏览安全隔离的演示情报.md#验收标准'
  - '_agentic-out/planning/epics.md#Story-1.6'
  - '_agentic-out/planning/prd.md#FR1-FR2,NFR1,NFR10,NFR28-NFR29'
externalPointerStatus: 'not_used'
gateType: 'story'
collectionMode: 'contract_static'
tempCoverageMatrixPath: 'C:\Users\13479\AppData\Local\Temp\testing-trace-coverage-matrix-2026-08-15T00-15-00+08-00.json'
---

# Story 1.6 测试追踪矩阵与质量门

## Step 1：覆盖权威与上下文

- 覆盖权威：Story 1.6 的正式 AC1–AC4；Story 文件记录的 Windows-first 交付覆盖和用户批准的 30 次候选构建样本调整属于本次里程碑的权威验收口径。
- 解析方式：`formal_requirements`；覆盖基础为 `acceptance_criteria`；置信度 `high`。
- 辅助上下文：Epic Story 1.6、PRD FR1–FR2、NFR1、NFR10、NFR28–NFR29，以及 Story 引用的 Architecture/UX 边界。
- 外部指针：未使用；全部权威材料均为本地项目文档。
- 范围：共享核心与 Windows 应用。Apple/iPhone/iPad/Android 仍是明确的 Deferred Acceptance，不计为 Windows milestone 缺口，也不能被本报告宣称已实施。
- 质量前置：`agentic-test-automation` 已完成；`agentic-test-review` 修复后 100/100、remaining violations = 0。

## Step 2：测试清单与覆盖启发式

完整回归清单为 76 个运行测试：Rust 50、Vitest 19、Playwright 7；另有 AC4 的 30 次真实候选构建冷启动证据。Story 1.6 的主要直接证据如下。

| ID | Level | 测试/证据 | 文件与行 | Priority | 状态 |
|---|---|---|---|---|---|
| T16-E01 | E2E | 无注册、Key、通知授权显示三条固定 demo | `tests/e2e/story-1-6-demo-intelligence.spec.ts:35` | P0 | PASS |
| T16-E02 | E2E | 列表与详情逐条文字标记 demo | `tests/e2e/story-1-6-demo-intelligence.spec.ts:55` | P1 | PASS |
| T16-E03 | E2E | 选择条目并打开对应详情 | `tests/e2e/story-1-6-demo-intelligence.spec.ts:67` | P1 | PASS |
| T16-E04 | E2E | 搜索、赛道筛选及结果 demo 标记 | `tests/e2e/story-1-6-demo-intelligence.spec.ts:90` | P1 | PASS |
| T16-E05 | E2E | 空态与清除条件 | `tests/e2e/story-1-6-demo-intelligence.spec.ts:141` | P1 | PASS |
| T16-E06 | E2E | 网络与 Notification 外调为 0 | `tests/e2e/story-1-6-demo-intelligence.spec.ts:177` | P0 | PASS |
| T16-E07 | E2E | command 错误脱敏、持久错误与重试恢复 | `tests/e2e/story-1-6-demo-intelligence.spec.ts:210` | P0 | PASS |
| T16-C01..05 | Component | ready/search/error/无权限无外联/health-detail 错误恢复 | `apps/windows/src/features/demo-intelligence/demo-intelligence.test.tsx:59` | P0/P1 | 5/5 PASS |
| T16-U01..04 | Unit/Integration | seed 幂等与查询、零副作用、origin 隔离、file-backed DB 合同 | `crates/radar-core/tests/demo_catalog.rs:62` | P0 | 4/4 PASS |
| T16-U05..06 | Unit | reseed 自愈与严格 fixture 校验 | `crates/radar-core/src/application/demo.rs:594` | P0/P1 | 2/2 PASS |
| T16-U07 | Unit/Integration | 分页边界、错误脱敏、失败后 store 可恢复 | `crates/radar-core/tests/generated_demo_pagination_edges.rs:4` | P1 | PASS |
| T16-U08 | Unit | demo command panic containment 且 store 不 poisoning | `apps/windows/src-tauri/src/commands/mod.rs:194` | P0 | PASS |
| T16-U09..10 | Unit/Contract | release command 精确 allowlist；本地 CSP/minimal capability | `apps/windows/src-tauri/tests/release_surface.rs:8` | P0 | 2/2 PASS |
| T16-U11..14 | Unit/Transport | demo commands、empty/search、detail guard、list/filter paging | `apps/windows/src/lib/desktop-api/tauri-desktop-api.test.ts:79` | P1 | 4/4 PASS |
| T16-U15..16 | Unit/Boundary | 裸 invoke 单入口；无 DB/通用 command/明文 secret surface | `apps/windows/src/test/source-boundaries.test.ts:20` | P0 | 2/2 PASS |
| T16-U17..22 | Unit/Mutation | fixture 语义变异、敏感内容、越界面、最小壳、Windows 边界、测试命名源码导入均受 gate 约束 | `crates/xtask/tests/generated_contract_gate_negative.rs:15` | P0 | 6/6 PASS |
| T16-P01 | Runtime acceptance | 30/30 MSVC release 冷启动，P95 2693.34 ms，阈值 5000 ms | `target/story-1-6-benchmark/20260814-072808-235/windows-demo-cold-start.json` | P0 | PASS |

### Coverage heuristics

- API endpoint：N/A。项目没有 HTTP/OpenAPI endpoint；Story 1.6 使用本地 Tauri IPC，相关 command/DTO 由 Rust contract、TS transport 和 Playwright 共同覆盖，不虚报 API tests。
- Authentication/authorization：N/A。AC 明确要求无需注册、账户或 Key；当前 Story 没有 login/session/role 权限模型。无门槛启动由 T16-E01 与 T16-C04 直接覆盖。
- Error paths：适用且已覆盖。包括 fixture/schema 变异、分页非法参数、SQLite 合同、panic containment、DTO guard、loading/empty/persistent error/retry、超时/旧请求代次防护以及私密 canary 不泄漏。
- UI journey：启动 → 列表 → 选择详情 → 搜索/筛选 → 空态/清除均有 E2E；无网络/无通知权限路径有执行时外调计数为 0 的 E2E 与 component 证据。
- UI states：ready、empty、persistent error、retry、detail error 均有断言；loading 由 component 测试覆盖。Story 1.7 才承担完整 NVDA/主题/高对比度/200% 真实矩阵。
- skipped/fixme/pending：测试框架标记均为 0。xtask 的 symlink 用例在当前 Windows 无创建权限时可条件提前返回，但本矩阵不依赖该分支证明 AC1–AC4。
- 性能：T16-P01 是同一 MSVC release 候选的 30 个原始样本，含 OS、设备、WebView2、工具版本、fixture 与候选 SHA；不是用 Vitest/E2E 时长替代 AC4。

## Step 3：AC → 测试追踪矩阵

| Oracle ID | 正式验收语义 | Priority | 主要测试证据 | Levels | 状态 | 判定依据 |
|---|---|---|---|---|---|---|
| AC1.1 | 全新安装从共享 fixture 可复现、幂等加载 demo | P0 | T16-U01, T16-U05, T16-U17, T16-P01 | Unit/Integration/Runtime acceptance | FULL | 编译时共享 fixture、严格语义校验、seed/reseed、投影自愈及隔离新用户目录的真实候选首次启动均有直接证据 |
| AC1.2 | 列表、详情、搜索/筛选结果逐条非颜色标记“演示数据” | P1 | T16-E02, T16-E04, T16-C01..02 | E2E/Component | FULL | 真实用户路径及组件状态均逐条断言文字标签 |
| AC2.1 | demo 不产生 notification、AI、validation metrics、network/outbox/effect | P0 | T16-U02, T16-E06, T16-C04 | Unit/Integration/E2E | FULL | 核心 fake ports 与浏览器外联/Notification 运行时记录均为 0 |
| AC2.2 | `data_origin=demo` 使用独立 namespace，不与 real 合并/重复 | P0 | T16-U03, T16-U05 | Unit/Integration | FULL | 同 external ID/URL/hash 的 demo 与 synthetic-real 保持独立；reseed 不伤 real |
| AC3.1 | 无网络、无 Key、未授权通知时无注册/账户/Key/授权墙 | P0 | T16-E01, T16-E06, T16-C04 | E2E/Component | FULL | 默认阻断网络与通知 API 后仍完整呈现固定目录，且未请求权限 |
| AC3.2 | 可浏览列表、打开详情、基础搜索/筛选与空态恢复 | P1 | T16-E03..05, T16-C01..03, T16-U07, T16-U11..14 | E2E/Component/Unit | FULL | 主路径、空态、非法分页、transport DTO、persistent error/retry 均覆盖 |
| AC3.3 | 外部网络请求与系统通知提交为 0 | P0 | T16-E06, T16-U02, T16-U09..10, T16-U15..22 | E2E/Unit/Contract | FULL | 运行时外调计数 + core policy + capability/source mutation gate 三层证据 |
| AC4.1 | Windows 候选构建至少 30 次，启动至列表可滚动且详情可打开的 P95 ≤ 5 秒 | P0 | T16-P01 | Runtime acceptance | FULL | 30/30 成功，P95 2693.34 ms，max 4636.79 ms，阈值判定为 PASS |
| AC4.2 | 验收记录保存平台、设备、构建、数据集和实际结果 | P1 | T16-P01 | Runtime acceptance | FULL | JSON 保存 OS/device/WebView2/Rust/Tauri/Node/pnpm/compiler/SDK/fixture/SHA 和 30 个原始样本 |

### 覆盖逻辑校验

- P0：6/6 FULL；P1：3/3 FULL。
- 无 `NONE`、`PARTIAL`、`UNIT-ONLY` 或 `INTEGRATION-ONLY` 项。
- AC1/AC3 的用户旅程均有 E2E，不以 unit-only 证据冒充 UI 验收。
- AC2 同时拥有核心 policy 的直接执行证据和 Windows 运行时外联记录；静态 gate 仅作为第三层防回归，不单独支撑 FULL。
- AC4 使用正式候选与原始 30 样本；测试套件执行耗时未参与性能判定。
- HTTP endpoint 与 auth/authz 对本 Story 均不适用；不存在因为缺少非适用测试而降级的情况。

## Step 4：覆盖缺口与统计

- Trace items：9；FULL 9（100%）；PARTIAL 0；NONE 0；UNIT-ONLY 0。
- P0：6/6 FULL（100%）；P1：3/3 FULL（100%）；P2/P3 为空集合，按规则为 100%。
- Critical/High/Medium/Low uncovered gaps：0/0/0/0。
- Coverage heuristics：endpoint、auth negative path、happy-path-only、UI journey、UI state gaps 均为 0；其中 HTTP endpoint 和 auth/authz 是明确 N/A，不代表存在 HTTP/auth 测试。
- 去重矩阵引用：30 项（29 个自动化测试 + 1 个 runtime benchmark）；Step 2 主要证据展开为 35 项；完整回归为 76 项。三种口径用途不同，不混算覆盖率。
- skipped/fixme/pending blockers：0；条件性 symlink 分支不支撑本 Story 的 FULL 判定。
- 独立复核：Worker A、B、C 均同意 9/9 FULL；Worker C 纠正了优先级统计为 P0 6、P1 3。

### 残余 provenance 说明（非覆盖缺口）

- 30 次样本、候选 EXE 哈希和记录内的原始统计保持一致，候选 `target/x86_64-pc-windows-msvc/release/ai-subscribe-desktop.exe` 当前哈希仍为 `2393F438...3717`。
- 采样后 `apps/windows/src-tauri/src/lib.rs` 与 `pnpm-lock.yaml` 发生了变更，导致记录的 9 个 source hashes 中 2 个与当前树不同。已核对变更分别是条件编译下的 `unused_mut` 修正和 Playwright 开发依赖锁文件更新，不改变已测候选运行时行为；因此 AC4 仍为 FULL。
- 若以当前源码生成新的发布候选，应重新构建并重新采集 30 次样本，不能把旧候选的性能数字转移到新二进制。

### 建议

- LOW：在下一次 release candidate 构建后重跑 `scripts/windows-demo-smoke.ps1`，刷新候选与 source hash 同源证据。
- LOW：继续保留 test-review、完整回归、contract mutation gate 和 Playwright P0/P1 选择执行。

## Phase 2：质量门禁结论

### GATE DECISION：PASS

**门禁资格：** eligible（`allow_gate=true`，`collection_status=COLLECTED`）  
**判定模式：** deterministic  
**判定依据：** P0 6/6（100%，要求 100%）、P1 3/3（100%，PASS 目标 90%）、总体 9/9（100%，最低 80%）全部满足，critical gaps 为 0。

| Gate criterion | 实际值 | 阈值 | 结果 |
|---|---:|---:|---|
| P0 coverage | 100% | 100% | MET |
| P1 coverage | 100% | ≥90% PASS / ≥80% minimum | MET |
| Overall coverage | 100% | ≥80% | MET |
| Critical open gaps | 0 | 0 | MET |

候选性能 provenance 的 2/9 source-hash 漂移已作为 LOW 后续建议披露，不改变对已记录候选本身的 AC4 与覆盖门判断；任何新候选仍须重新采样。

机器输出：

- `_agentic-out/tests/reports/e2e-trace-summary.json`
- `_agentic-out/tests/reports/gate-decision.json`

### Artifact-order audit

2026-08-15 reconcile 后按 Story → automation → traceability 重新登记产物；本矩阵读取的是最终 automation 50/50、Vitest 19/19、Playwright burn-in 21/21 证据。
