---
stepsCompleted:
  - 'step-01-load-context'
  - 'step-02-discover-tests'
  - 'step-03f-aggregate-scores'
  - 'step-04-generate-report'
lastStep: 'step-04-generate-report'
lastSaved: '2026-08-14T19:10:00+08:00'
workflowType: 'testarch-test-review'
inputDocuments:
  - '_agentic/config.yaml'
  - '_agentic-out/implementation/stories/1-6-浏览安全隔离的演示情报.md'
  - '_agentic-out/tests/reports/automation-summary.md'
  - 'playwright.config.ts'
  - 'tests/'
  - 'apps/windows/src/**/*.test.ts(x)'
  - 'apps/windows/src-tauri/tests/'
  - 'crates/**/tests/'
  - 'agentic-test-review/resources/testing-index.csv'
  - 'agentic-test-review/resources/knowledge/test-quality.md'
  - 'agentic-test-review/resources/knowledge/data-factories.md'
  - 'agentic-test-review/resources/knowledge/test-levels-framework.md'
  - 'agentic-test-review/resources/knowledge/selective-testing.md'
  - 'agentic-test-review/resources/knowledge/test-healing-patterns.md'
  - 'agentic-test-review/resources/knowledge/selector-resilience.md'
  - 'agentic-test-review/resources/knowledge/timing-debugging.md'
  - 'agentic-test-review/resources/knowledge/playwright-utils-core'
  - 'agentic-test-review/resources/knowledge/playwright-cli.md'
---

# Test Quality Review: Story 1.6 suite

**Quality Score:** 100/100（A — Excellent；修复前 93/100）  
**Review Date:** 2026-08-14  
**Review Scope:** suite，22 个一方测试文件、75 个实际运行测试  
**Recommendation:** Approve（复核确认 3/3 问题全部关闭）

> 本评审只审计测试质量，不评分验收覆盖率。覆盖矩阵与 gate decision 交由 `agentic-test-traceability`。

## Executive Summary

测试体系整体质量优秀：Rust、Vitest 与 Playwright 分层清楚，E2E 使用导航前 Tauri mock、确定性 DTO factory、语义选择器和零外联断言；无硬等待、无 serial group，三轮 Playwright burn-in 21/21。性能维度 100 分。

当前需要修复两个高优先级问题：文件型 SQLite 测试使用固定路径且不清理，可能跨运行共享数据或锁；一个 xtask 变异测试单体 101 行，刚超过维护阈值。另有一处轻微 E2E 重复。固定 SQLite 路径同时影响确定性和隔离性，但属于同一个根因，修复一次即可消除两项评分扣分。

### Key Strengths

- Playwright fixture 在应用脚本前注入 Tauri IPC，并逐测试重建调用与外联记录。
- 共享 demo 数据使用固定、可 override 的合同 factory；不使用随机值破坏可复现性。
- Rust 并发测试用 Barrier + join，同步明确且断言不依赖调度顺序。
- Credential Manager 使用测试 namespace、PID 与原子轮次，带 RAII 和显式删除。
- 现有实跑：Rust 49/49、Vitest 19/19、Playwright 7/7、burn-in 21/21。

### Key Weaknesses

- 文件型 SQLite 合同测试缺少唯一 scoped path 与 teardown。
- 一个 Windows boundary mutation 测试体 101 行，可读性和扩展成本偏高。
- E2E 多处重复 `page.goto("/")` 与演示列表 locator。

## Quality Criteria Assessment

| Criterion | Status | Violations | Notes |
|---|---|---:|---|
| BDD / readable intent | PASS | 0 | Playwright 使用 Given/When/Then steps；Rust 使用行为命名 |
| Priority markers | PASS | 0 | 新生成/风险选择测试均有 P0/P1；legacy 不强制补标 |
| Hard waits | PASS | 0 | 无 sleep、waitForTimeout 或任意延迟 |
| Determinism | WARN | 1 | 固定 file-backed DB path |
| Isolation | FAIL | 1 | 固定 DB 跨进程共享且无 cleanup |
| Fixture patterns | PASS | 0 | mergeTests + auto fixture + per-page state |
| Data factories | PASS | 0 | 确定性合同 factory；faker 对 demo 合同 N/A |
| Network-first | PASS | 0 | addInitScript 在 goto 前生效；真实外联默认阻断 |
| Selectors | PASS | 0 | ARIA role/accessible name，无 CSS/nth/XPath |
| Explicit assertions | PASS | 0 | 用户结果、IPC 参数、错误脱敏、零外联均显式断言 |
| Test length | FAIL | 1 | 1 个测试体 101 行 |
| Test duration | PASS | 0 | E2E 3 轮 21 tests 约 26 秒；无慢测证据 |
| Performance / parallelism | PASS | 0 | 75 tests 可并行；0 serial |
| Duplication | WARN | 1 | E2E 导航与列表 locator 轻微重复 |

### Weighted Score

| Dimension | Weight | Score | Grade | Violations |
|---|---:|---:|---|---:|
| Determinism | 30% | 95 | A | 1 MEDIUM |
| Isolation | 30% | 90 | A- | 1 HIGH |
| Maintainability | 25% | 88 | B | 1 HIGH、1 LOW |
| Performance | 15% | 100 | A | 0 |
| **Overall** | **100%** | **93** | **A** | **2 HIGH、1 MEDIUM、1 LOW** |

## High-Priority Findings

### 1. 固定 SQLite 路径跨运行共享且不清理

**Severity:** HIGH（Isolation）+ MEDIUM（Determinism）  
**Location:** `crates/radar-core/tests/demo_catalog.rs:112`  
**Knowledge:** test-quality / fixture-architecture

当前测试始终写入 `target/story-1-6-file-backed-test/ai-subscribe.sqlite3`。并发 Cargo 进程、上次中断残留或重跑都可能继承记录和锁状态。

建议使用 PID + 原子序列形成唯一目录，并用 RAII teardown 删除：

```rust
let temp = ScopedTestDir::new("story-1-6-file-backed");
let path = temp.path().join("ai-subscribe.sqlite3");
let mut store = DemoStore::open(&path).expect("file-backed demo store");
// temp drops after store and removes the scoped directory
```

### 2. 单个 xtask 变异测试超过 100 行

**Severity:** HIGH（Maintainability）  
**Location:** `crates/xtask/tests/generated_contract_gate_negative.rs:298`  
**Knowledge:** test-quality

`xtask_rejects_windows_boundary_mutations` 的单个函数体为 101 行。建议把 case table 移到命名常量或 builder，并抽出重复的 sandbox/write/check/assert helper；bundled test-source 场景保留为独立测试。

```rust
fn assert_boundary_rejected(relative: &str, contents: &str, expected: &str) {
    let temp = TempDir::new("windows-mutation");
    write_file(temp.path(), relative, contents);
    let error = check_boundaries(temp.path()).expect_err("mutation must fail");
    assert!(error.contains(expected), "{relative}: {error}");
}
```

## Recommendation

### 3. 抽取 E2E 演示目录入口 helper

**Severity:** LOW  
**Location:** `tests/e2e/story-1-6-demo-intelligence.spec.ts:27`  
**Knowledge:** fixture-composition / selector-resilience

多条场景重复导航与同一 accessible region locator。可抽取一个小函数，命令专属行为仍留在各测试内：

```ts
async function openDemoCatalog(page: Page) {
    await page.goto("/");
    const list = page.getByRole("region", { name: "演示情报列表" });
    await expect(list).toBeVisible();
    return list;
}
```

## Best Practices Found

- `tests/support/fixtures/demo-app.fixture.ts`：导航前注入、逐测试状态、失败证据附件和默认外联阻断。
- `tests/support/factories/demo-dto.factory.ts`：完整 DTO defaults + overrides，适合稳定合同数据。
- `crates/radar-ffi/tests/generated_wire_concurrency.rs`：Barrier + join 的并发契约验证，不依赖线程顺序。
- `apps/windows/src-tauri/tests/windows_secret_store.rs`：子进程隔离、唯一 namespace、清理 guard 与残留探测。
- `tests/e2e/story-1-6-demo-intelligence.spec.ts`：P0/P1 选择标签、错误 canary 不泄漏、重试恢复与 externalCalls 零断言。

## Scope and Evidence

| Group | Files | Runtime tests |
|---|---:|---:|
| Rust core/FFI/xtask | 13 | 40 |
| Rust Tauri host | 3 | 7 |
| Vitest frontend | 5 | 19 |
| Playwright E2E | 1 | 7 |

- 总计 22 文件、3530 行、75 tests。
- Browser CLI 未安装；为遵守项目隔离约束未做全局安装，改用仓库本地 Playwright runner 与 trace/artifact 配置。
- Playwright preview 及项目浏览器进程已清理。
- Apple/Android 仍按已批准的 Windows-first 决策延期，不把 Windows 测试证据扩张为移动端证据。

## Knowledge Base References

- `test-quality.md` — 测试 DoD、长度、确定性与清理。
- `fixture-architecture.md` / `fixtures-composition.md` — scoped fixture 与 teardown。
- `data-factories.md` — defaults + overrides；合同数据的确定性例外。
- `selector-resilience.md` — role/accessible name 与 locator scope。
- `timing-debugging.md` / `test-healing-patterns.md` — 无硬等待、事件驱动同步。
- `selective-testing.md` — P0/P1 风险选择。
- Playwright Utils core 与 `playwright-cli.md` — fixture、证据和会话卫生。

## Decision and Next Steps

**Decision:** Approve。当前无 P0 blocker；原 2 个 HIGH、1 个 MEDIUM、1 个 LOW 已全部关闭。

1. 为 file-backed SQLite 测试加入唯一 scoped path 与 RAII cleanup。
2. 重构 101 行 xtask mutation 测试，并可顺手抽取 E2E catalog helper。
3. 重跑 Rust workspace、Clippy、contracts、Playwright 及 burn-in。
4. 复评通过后运行 `agentic-test-traceability`。

Coverage mapping 明确不属于本报告，未纳入 93 分评分。

## Post-fix Re-review

- 复核日期：2026-08-14。
- `demo_catalog.rs` 已使用 PID + 原子序列的 scoped 目录，并在 `Drop` 中清理，关闭隔离与确定性问题。
- `xtask_rejects_windows_boundary_mutations` 已缩短至 78 行，重复 setup/assert 已抽取，独立场景已拆分。
- `openDemoCatalog` helper 实际复用 5 次，关闭 E2E 重复问题。
- 独立静态复核结果：3/3 closed，remaining violations = 0。
- 完整验证：Rust workspace 50/50、Vitest 19/19、Playwright burn-in 21/21、Clippy、fmt、contracts、frontend format/lint/typecheck/build 全部通过。
- 复核证据：`_agentic-out/tests/reports/test-review-workers/testing-review-post-fix-2026-08-14T10-00-00-000Z.json`。

修复后质量分为 100/100；覆盖矩阵仍不属于本报告，将由 `agentic-test-traceability` 独立判定。

## Review Metadata

- Workflow: `agentic-test-review`
- Review ID: `story-1-6-suite-20260814`
- Execution: subagent with sequential fallback after two worker timeouts
- Temp outputs: 已聚合，正式证据将保存到 `_agentic-out/tests/reports/`
