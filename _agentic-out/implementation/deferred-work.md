# Deferred Work

## Deferred from: code review of Story 1.8 (2026-08-17)

- `scripts/windows-demo-smoke.ps1:237`：WebView2 版本探测/清理按全局进程枚举，可能关联或终止不属于候选应用的 WebView2；既有 Story 1.6/1.7 脚本问题，后续应改为候选进程树或 Job Object 精确归属。
- `scripts/windows-demo-smoke.ps1:337`：`AccessibilityEvidence` 模式未独立强制 5000ms 性能阈值；既有证据脚本问题，后续应拆分 performance/accessibility 结论。
- `scripts/windows-demo-smoke.ps1:281`：候选强杀后的 `WaitForExit()` 无总时限；既有脚本问题，后续应有界等待并保证 finally 恢复环境。
- `tests/support/factories/demo-dto.factory.ts:19`：默认 `original_url` 与 provenance URL 不一致；属于 Story 1.7 既有 factory 默认值问题，后续应让默认对象满足生产全部不变量。

## Deferred from: code review of Story 2.2 (2026-08-18)

- `crates/xtask/src/contracts.rs:474`：`.env.example` placeholder 豁免仍可能放过真实凭据；属于前序通用 xtask 边界问题，后续应校验示例值仅为占位符。

## Deferred from: code review of Story 4.1 (2026-08-19)

- `apps/windows/src-tauri/src/commands/mod.rs:458`：Story 2.5 的 `retry_wait` 任务没有 deadline 后的生产重调度，可能长期保持 pending。
- `crates/radar-core/src/application/sync.rs:1491`：Story 2.5 的 mixed failed+retry_wait 聚合与 Windows TS 守卫不一致。
- `crates/radar-core/src/application/sync.rs:1848`：Story 2.5 的 retry_wait readiness 被聚合为 syncing，但 TS 要求至少一个 syncing source。
- `apps/windows/src-tauri/src/commands/mod.rs:515`：Story 2.5 的 30 秒预算路径 drop JoinHandle 后，detached fetch 可能继续占用网络资源。
- `apps/windows/src-tauri/src/commands/mod.rs:471`：Story 2.5 使用 blocking sleep 作为预算 timer，快速完成后线程仍睡满 30 秒。
- `apps/windows/src/lib/desktop-api/desktop-api.ts:995`：Story 2.5 startSync idempotency key 的 TS/Rust 字符集规则不一致。
- `crates/radar-core/src/application/sync.rs:541`：Story 2.5 正常预算耗尽被记录成 `internal.unexpected` 与 failed count，混淆控制流和内部故障。
