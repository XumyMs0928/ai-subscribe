---
stepsCompleted: [step-01-load-context, step-02-discover-tests, step-03-map-criteria, step-04-analyze-gaps, step-05-gate-decision]
lastStep: step-05-gate-decision
lastSaved: 2026-08-19
coverageBasis: acceptance_criteria
oracleConfidence: high
oracleResolutionMode: formal_requirements
oracleSources:
  - _agentic-out/implementation/stories/4-1-将真实来源规范化为可溯源情报.md
  - _agentic-out/planning/architecture.md
externalPointerStatus: not_used
story: '4.1'
deliveryScope: windows-rss-minimum-loop
tempCoverageMatrixPath: 'C:\Users\13479\AppData\Local\Temp\testing-trace-coverage-matrix-2026-08-19T15-55-39-447Z.json'
---

# Story 4.1 可追溯性矩阵

## Oracle 与范围

- 权威 Oracle：Story 4.1 的 AC1–AC6，置信度 high。
- 当前 gate：Windows + RSS/Atom + shared core/SQLite minimum loop。
- GitHub Release、arXiv、移动端及 Epic 3 discovery integration 明确 Deferred，不计入本阶段缺口，也不得被本报告宣称完成。
- Story 4.1 没有新增页面、路由、HTTP endpoint 或 Tauri command；因此不虚构新的 API/E2E 证据。

## 测试清单

| ID | Level | Test | File:line | Priority | State |
| --- | --- | --- | --- | --- | --- |
| T41-U01 | Unit | normalization_is_stable_source_scoped_and_does_not_invent_author | `crates/radar-core/tests/intel_normalization.rs:36` | P0 | active |
| T41-U02 | Unit | invalid_optional_time_becomes_null_with_allowlisted_warning | `crates/radar-core/tests/intel_normalization.rs:71` | P0 | active |
| T41-U03 | Unit | required_and_bounded_fields_fail_closed | `crates/radar-core/tests/intel_normalization.rs:97` | P0 | active |
| T41-U04 | Unit | serde_boundaries_reject_unknown_candidate_and_warning_fields | `crates/radar-core/tests/intel_normalization.rs:142` | P1 | active |
| T41-U05 | Unit | sync_result_item_v1_accepts_legacy_missing_link_but_rejects_extra_fields | `crates/radar-core/tests/intel_normalization.rs:166` | P1 | active |
| T41-I01 | Integration | committed_result_is_paged_reopenable_and_counts_invalid_candidates | `crates/radar-core/tests/sync_tasks.rs:120` | P0 | active |
| T41-I02 | Integration | final_fact_identity_is_stable_updated_and_source_scoped | `crates/radar-core/tests/sync_tasks.rs:192` | P0 | active |
| T41-I03 | Integration | same_hash_replay_is_unchanged_even_when_display_fields_differ | `crates/radar-core/tests/sync_tasks.rs:402` | P1 | active |
| T41-A01 | Unit/DB | final_fact_update_preserves_provenance_and_has_no_derived_projection | `crates/radar-core/src/application/sync.rs:2190` | P0 | active |
| T41-A02 | Unit/DB | all_invalid_candidates_fail_without_advancing_success_checkpoint | `crates/radar-core/src/application/sync.rs:2282` | P0 | active |
| T41-A03 | Unit/DB | changed_fact_keeps_last_updated_monotonic_when_clock_moves_back | `crates/radar-core/src/application/sync.rs:2353` | P0 | active |
| T41-A04 | Unit/DB | normalized_fact_write_failures_roll_back_the_whole_source | `crates/radar-core/src/application/sync.rs:2425` | P0 | active |
| T41-A05 | Unit/DB | terminal_history_pruning_preserves_fact_provenance_and_checkpoint | `crates/radar-core/src/application/sync.rs:2556` | P0 | active |
| T41-M01 | Migration | v7_results_backfill_one_stable_fact_with_deterministic_tie_break | `crates/radar-core/src/application/demo.rs:2722` | P0 | active |
| T41-M02 | Migration | v7_result_backfill_rejects_noncanonical_or_unsafe_history | `crates/radar-core/src/application/demo.rs:2778` | P0 | active |
| T41-R01 | Integration | rss_and_atom_fixtures_preserve_optional_fields_and_stable_identity | `crates/radar-core/tests/rss_atom_sources.rs:148` | P0 | active |
| T41-R02 | Integration | malformed_or_oversized_feed_is_rejected_without_partial_candidates | `crates/radar-core/tests/rss_atom_sources.rs:162` | P0 | active |
| T41-R03 | Integration | parser_rejects_non_feed_xml_and_unsafe_identity_but_accepts_an_empty_feed | `crates/radar-core/tests/rss_atom_sources.rs:309` | P0 | active |
| T41-C01 | Component/contract | normalizes legacy missing intel IDs and rejects malformed additive shapes | `apps/windows/src/lib/desktop-api/tauri-desktop-api.test.ts:1191` | P1 | active |

清单共 19 个去重直接证据；`skipped=0`、`pending=0`、`fixme=0`。

## AC → 测试映射

| AC | Priority | Risk | Direct evidence | Levels | Status | Rationale |
| --- | --- | ---: | --- | --- | --- | --- |
| AC1 稳定事实与溯源 | P0 | 6 | T41-U01, T41-U02, T41-I01, T41-A01, T41-R01 | Unit + Integration + DB | FULL | 直接验证稳定 ID、完整最小字段、author null、HTTPS/time/hash 与落库投影。 |
| AC2 生命周期且不覆盖事实 | P0 | 6 | T41-I02, T41-I03, T41-A01, T41-A03 | Integration + DB | FULL | 覆盖 new/changed/unchanged、revision、first/last 单调以及规则/AI 零写入。 |
| AC3 第三方字段 fail closed | P0 | 9 | T41-U02, T41-U03, T41-U04, T41-A02, T41-M02, T41-R02, T41-R03 | Unit + Adapter + DB + Migration | FULL | 结构性 feed 与记录级错误分层，unsafe URL/长度/hash/extra key/时间均有负路径且不推进 checkpoint。 |
| AC4 混合批次与事务一致 | P0 | 9 | T41-I01, T41-A02, T41-A04 | Integration + DB fault injection | FULL | 混合有效/无效计数、failure projection、来源事务回滚与修复后恢复均为直接证据。 |
| AC5 迁移、重启、保留与隔离 | P0 | 9 | T41-I01, T41-A05, T41-M01, T41-M02 | Integration + Migration | FULL | v7→v8 确定性回填、矛盾历史回滚、reopen、retention 保留 fact/provenance/checkpoint。 |
| AC6 来源隔离与稳定身份 | P1 | 6 | T41-U01, T41-I02, T41-I03, T41-A04 | Unit + Integration + DB | FULL | 同来源跨 run 稳定，不同来源同 raw ID 不碰撞；写入故障限定到当前来源事务。 |

## Coverage heuristics

- HTTP/OpenAPI endpoint：N/A；产品没有 inbound HTTP API。本 Story 也没有新增 Tauri IPC。
- Auth/authz：N/A；当前设备本地单用户 slice 没有 login、role、tenant 或 permission boundary。
- Error paths：0 gap；AC3–AC5 包含 validation、unsafe input、migration contradiction、transaction failure、rollback/recovery。
- UI journeys/states：N/A；Story 明确 backend-first 且无 UI/route change，既有 Story 2.6 页面仅作为回归，不作为 SQLite 事实证明。
- Happy-path-only criteria：0；每个 P0 数据完整性 AC 都有失败、边界或重放证据。

## Phase 1 统计

- FULL：6/6（100%）；PARTIAL/NONE/UNIT-ONLY：0。
- P0：5/5（100%）；P1：1/1（100%）。
- 去重直接证据：19 cases / 6 files；skipped/fixme/pending 均为 0。
- 可执行 coverage gap：0；下一步进入确定性 gate decision。

## Gate Decision：PASS

**Rationale:** P0 coverage is 100%, P1 coverage is 100% (target: 90%), and overall coverage is 100% (minimum: 80%).

- P0：100%（required 100%）→ MET
- P1：100%（target 90%, minimum 80%）→ MET
- Overall：100%（minimum 80%）→ MET
- Critical gaps：0
- Collection：COLLECTED；Oracle：formal requirements / high confidence

本结论仅批准 Story 4.1 的 Windows RSS minimum-loop；Deferred 的 GitHub Release、arXiv、移动端和完整三来源能力不在本次 PASS 范围内。
