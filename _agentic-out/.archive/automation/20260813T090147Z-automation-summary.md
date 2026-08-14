---
stepsCompleted: ['step-01-preflight-and-context', 'step-02-identify-targets', 'step-03c-aggregate', 'step-04-validate-and-summarize']
lastStep: 'step-04-validate-and-summarize'
lastSaved: '2026-08-13'
lastValidated: '2026-08-13T17:12:00+08:00'
inputDocuments:
  - '_agentic/config.yaml'
  - '_agentic-out/implementation/stories/1-1-建立共享核心与版本化契约基线.md'
  - '_agentic-out/planning/prd.md#FR1,NFR15'
  - '_agentic-out/planning/architecture.md#contract-boundaries'
  - 'Cargo.toml'
  - 'crates/radar-core/tests/'
  - 'crates/radar-ffi/tests/'
  - 'crates/xtask/tests/'
---

# Story 1.1 测试自动化总结

## 本轮目标

- 模式：Integrated；技术栈：backend-only Rust Cargo workspace。
- 补齐 AC1.2：为 `apps/**`、`migrations/**`、SQL DDL 与 `rusqlite` 增加独立变异测试。
- 补齐 AC2.8：验证 RFC3339 UTC 合法变体、日历边界、实际已公告闰秒与 canonical 输出。
- 避免 HTTP、浏览器、数据库或平台 UI 测试；这些均不属于 Story 1.1。

## 新增与强化证据

- `xtask_rejects_each_out_of_scope_workspace_surface`：独立验证越界目录、SQL 文件、SQL DDL 空格/换行/制表符/多空格变体及 `rusqlite`。
- `xtask_rejects_sensitive_files_and_contents`：补齐 `.env.*`、`id_ed25519`、`.db`、`.sqlite`、`.key` 与 `private_key`。
- `platform_effect_rfc3339_utc_boundaries_are_complete_and_canonical`：接受 `T/t`、`Z/z`、`+00:00`、fraction、四位年份与实际已公告闰秒；拒绝非法日期、未公告闰秒、非零/未知偏移及超限 canonical 值；输出统一为 uppercase `T...Z`。
- 真实 RED/GREEN：首次新增 `.sql` 变异测试暴露 scanner 未读取 `.sql`；修复后通过。审查又发现 SQL 关键字可用空白变化绕过，现已归一化空白并补回归用例。

## 验证结果

- `scripts/rust-env.cmd fmt --all --check`：PASS。
- `scripts/rust-env.cmd clippy --workspace --all-targets --all-features -- -D warnings`：PASS。
- `scripts/rust-env.cmd test --workspace --all-targets`：32 passed，0 failed，0 ignored。
- `scripts/rust-env.cmd run -p xtask -- contracts`：PASS。
- 完整 suite burn-in：5/5 PASS，无不稳定失败。

## 覆盖与质量

- 当前测试：32 个，分布于 9 个含测试文件。
- Story trace items：12 个；补证后 AC1.2、AC2.8、AC2.10 均具备完整直接证据。
- Endpoint/Auth/UI：N/A；Error paths：适用且覆盖充分。
- 框架级 skipped/fixme/pending：0/0/0。
- Windows 当前宿主无符号链接创建特权时，symlink 用例会条件性提前返回；这不影响本轮 AC1.2 的 apps/migrations/DDL/rusqlite 补证，但不应把该宿主描述为执行了链接拒绝断言。

## 结论

测试自动化增量完成并验证稳定。下一步由 `agentic-test-traceability` 重新生成 12/12 FULL 的矩阵与质量门结论。
