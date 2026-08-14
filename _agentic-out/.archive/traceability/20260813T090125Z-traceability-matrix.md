---
stepsCompleted: ['step-01-load-context', 'step-02-discover-tests', 'step-03-map-criteria', 'step-04-analyze-gaps', 'step-05-gate-decision']
lastStep: 'step-05-gate-decision'
lastSaved: '2026-08-13'
coverageBasis: 'acceptance_criteria'
oracleConfidence: 'high'
oracleResolutionMode: 'formal_requirements'
oracleSources:
  - '_agentic-out/implementation/stories/1-1-建立共享核心与版本化契约基线.md#验收标准'
  - '_agentic-out/planning/prd.md#FR1,NFR15'
  - '_agentic-out/planning/architecture.md#SecretLeaseInput,PlatformEffect,AppError'
externalPointerStatus: 'not_used'
gateType: 'story'
collectionMode: 'contract_static'
tempCoverageMatrixPath: '_agentic-out/tests/reports/testing-trace-coverage-matrix-2026-08-13T17-10-00+08-00.json'
---

# Story 1.1 测试追踪矩阵与质量门

## Step 1：覆盖权威与上下文

- 覆盖权威：Story 1.1 正式 AC1、AC2，拆为 12 个稳定 trace item；PRD FR1/NFR15 与 Architecture 合同边界作为派生上下文。
- 解析方式：formal requirements；置信度 high；未使用外部指针或 synthetic oracle。
- 范围：仅评价纯 Rust core、adapter 与 contract gate；平台壳、真实 UniFFI ABI 和三端往返属于 Stories 1.2–1.5。
- 执行证据：32/32 测试、rustfmt、Clippy `-D warnings`、`xtask contracts` 与 5/5 burn-in 全部通过。

## Step 2：测试清单与启发式

发现 32 个相关 `#[test]`，分布于 9 个物理文件；`skipped=0`、`pending=0`、`fixme=0`。统一归为 Unit 32 / API 0 / Component 0 / E2E 0；external-crate binaries、xtask 与子进程输出捕获保留 integration 子类型，但不虚报 HTTP API/E2E。

关键新增证据：

- `T-015 platform_effect_rfc3339_utc_boundaries_are_complete_and_canonical`：`crates/radar-core/tests/generated_effect_contract_edges.rs:143`。
- `T-026 xtask_rejects_each_out_of_scope_workspace_surface`：`crates/xtask/tests/generated_contract_gate_negative.rs:186`。
- 原 `T-026/T-027` 顺延为 `T-027/T-028`；AC2.10 的清零/子进程证据为 `T-029..T-032`。

### Coverage heuristics

- API endpoint：N/A；无 HTTP/OpenAPI/server surface。
- Authentication/authorization：N/A；无 login/session/token/role 模型。SecretLease 属凭据泄漏防护，不冒充 auth coverage。
- Error paths：适用且覆盖充分；validation、internal、panic、effect conflict、lease consumed/error/panic、invalid JSON、CLI misuse、drift、敏感文件、越界目录、SQL DDL/rusqlite 均有直接断言。
- UI journey/state：N/A；Story 明确禁止平台 UI。
- 条件性说明：Windows 无符号链接权限（OS 1314）时 symlink 测试提前返回，因此不声称该宿主执行了链接拒绝断言；这不是本轮 AC1.2 补证阻塞项。

## Step 3：AC → 测试追踪矩阵

| Oracle ID | 正式验收语义 | Priority | 主要测试证据 | 状态 | 判定依据 |
|---|---|---|---|---|---|
| AC1.1 | workspace、core 构建、健康与纯 Rust 合同测试 | P1 | T-001, T-002, T-017 | FULL | health/core/adapter 直接断言；32/32 suite 通过 |
| AC1.2 | 最小工程边界；无未来业务表或平台 UI | P1 | T-025..T-028 | FULL | apps、migrations、SQL DDL 空白变体、rusqlite、敏感文件和目录剪枝有独立 gate 测试 |
| AC2.1 | v1 DTO/AppError/Effect/Secret 合同与稳定枚举 | P0 | T-002, T-003, T-016, T-017, T-020, T-023, T-024 | FULL | manifest/snapshot/wire/drift/invalid JSON 可执行 |
| AC2.2 | 成功路径稳定版本与字段语义 | P1 | T-001, T-017, T-020 | FULL | core health、adapter wire、JSON 转义精确断言 |
| AC2.3 | 字段失败映射稳定 validation | P0 | T-005, T-007, T-014, T-015, T-024 | FULL | ID、secret、enum、状态、时间与 JSON 负路径完整 |
| AC2.4 | unknown/panic → internal，panic 不越过 adapter | P0 | T-018, T-019, T-021 | FULL | containment、脱敏及 correlation ID 唯一性 |
| AC2.5 | effect 首次、重复与冲突回报幂等 | P0 | T-004, T-010..T-013 | FULL | 全终态、重复注册、unknown key、identity mismatch 与状态安全 |
| AC2.6 | SecretLease 一次性，失败/panic 后不可恢复 | P0 | T-006..T-009 | FULL | 构造、成功、operation error 与 callback panic 分支 |
| AC2.7 | ID 不透明且显式校验 | P1 | T-005, T-014 | FULL | 空/非法/非 ASCII、128/129 边界均覆盖 |
| AC2.8 | 时间使用 RFC3339 UTC | P1 | T-005, T-015 | FULL | 接受 T/t、Z/z、+00:00、fraction、四位年份与实际已公告闰秒；拒绝非法日历/offset/未公告闰秒；输出 canonical uppercase T/Z |
| AC2.9 | 缺失值显式 optional/null | P1 | T-001, T-017, T-020 | FULL | core None 与 wire null 精确断言 |
| AC2.10 | 明文不进入持久化、日志或可观察错误 | P0 | T-006..T-009, T-018, T-019, T-025, T-029..T-032 | FULL | zeroize 全容量清零审计、真实子进程 stdout/stderr canary 零命中、错误/fixture/workspace 防泄漏 |

## Step 4：覆盖缺口与统计

- Trace items：12；FULL 12（100%）；PARTIAL 0；NONE 0；UNIT-ONLY 0。
- P0：6/6 FULL（100%）。
- P1：6/6 FULL（100%）。
- P2/P3：无适用项，按空集合规则记为 100%。
- Critical/High/Medium/Low uncovered gaps：0/0/0/0。
- Endpoint/auth/UI heuristic gaps：0，均明确 N/A；happy-path-only error gaps：0。
- 测试清单：9 文件、32 cases、0 skipped/fixme/pending。

建议：继续保留项目隔离工具链中的 fmt、Clippy、workspace tests、contract gate 与 burn-in；Story 1.5 再增加真实 UniFFI/三端往返门禁。

## Phase 2：质量门禁结论

### GATE DECISION：PASS

**门禁类型：** Story  
**判定模式：** deterministic  
**收集状态：** COLLECTED（具备门禁资格）

| 判据 | 阈值 | 实际 | 状态 |
|---|---:|---:|---|
| P0 FULL 覆盖率 | 100% | 100%（6/6） | MET |
| P1 FULL 覆盖率 | PASS 目标 90%，最低 80% | 100%（6/6） | MET |
| 总体 FULL 覆盖率 | 最低 80% | 100%（12/12） | MET |

判定依据：P0、P1 与总体覆盖均满足确定性 PASS 阈值；无 uncovered requirement、框架级 skip/fixme/pending blocker 或开放 critical risk。Story 1.1 的纯 Rust 基线质量门通过。

机器可读结果：`_agentic-out/tests/reports/e2e-trace-summary.json` 与 `_agentic-out/tests/reports/gate-decision.json`。
