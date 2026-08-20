---
stepsCompleted: ['step-01-load-context', 'step-02-discover-tests', 'step-03-build-matrix', 'step-04-analyze-gaps', 'step-05-gate-decision']
lastStep: 'step-05-gate-decision'
lastSaved: '2026-08-14T12:10:00+08:00'
story: '1.2'
deliveryScope: 'windows-first'
coverageBasis: 'acceptance_criteria'
oracleConfidence: 'high'
gateDecision: 'WAIVED'
---

# Story 1.2 Windows 测试追踪矩阵

## 结论

Story 1.2 的核心功能、契约、安全边界和构建证据完整；7 个验收追踪项中 5 个 FULL、2 个 PARTIAL。两项 PARTIAL 已由用户于 2026-08-14 明确批准延期至 Story 1.7，因此本 Story 的确定性门禁结论为 **WAIVED**，不是 PASS。

## 追踪矩阵

| ID | 优先级 | 验收行为 | 主要证据 | 覆盖 | 处理 |
|---|---:|---|---|---|---|
| AC1.1 | P0 | MSVC release 构建与最小 release surface | Tauri release build；`release_surface.rs`；CSP/capability 与 xtask boundary tests | FULL | — |
| AC1.2 | P1 | 唯一可见窗口、共享 core health/version、退出零残留 | 用户真实运行截图；10 秒 strict smoke：唯一产品窗口、非屏外、`contract_version: 1`、失败清理后零残留 | PARTIAL | WebView2 将视觉可见 health UIA 节点报告为 offscreen；延期至 Story 1.7 |
| AC2.1 | P0 | DTO、AppError、panic containment 往返 | `contract_host.rs`、DesktopApi/fixture tests、command unit tests | FULL | — |
| AC2.2 | P0 | effect first/repeat/conflict 与单次副作用 | Windows command host tests、core ledger tests | FULL | — |
| AC2.3 | P0 | Credential Manager → SecretLease；零泄漏与零残留 | `windows_secret_store.rs`、secret diagnostics/bundle scans、canary child process | FULL | — |
| AC2.4 | P0 | UI 只能经 DesktopApi 调用 | `source-boundaries.test.ts`、xtask mutation tests、production bundle inspection | FULL | — |
| UX1.2 | P2 | 键盘、深浅主题、高对比度、Reduce Motion、100%/200% | 组件/样式静态测试；浅色 100% 人工截图 | PARTIAL | 深色/高对比度/Reduce Motion/200% 真实矩阵延期至 Story 1.7 |

## 统计与质量启发式

- FULL：5/7（71%）；PARTIAL：2/7；NONE：0。
- P0：5/5 FULL（100%）。
- HTTP endpoint、认证和移动端路径：本 Story 不适用。
- 错误路径：validation、internal、panic、timeout/retry、effect conflict、secret error/panic 均有直接测试。
- UI 状态：loading/success/error/timeout/retry 有 14 个 Vitest 用例支撑；真实环境可访问性矩阵仍受上述 waiver 约束。
- 没有 `#[ignore]`、框架 skipped、FIXME 或 pending 测试被计入成功覆盖。

## Gate Decision

**WAIVED — Windows-first milestone 可继续。** Waiver owner：用户/xmy；批准日期：2026-08-14；接收 Story：1.7。关闭条件：在可提供 WebView2 UI Automation provider 的交互式 Windows 会话验证 health/version，并完成深浅主题、高对比度、Reduce Motion 与 100%/200% 缩放矩阵。

