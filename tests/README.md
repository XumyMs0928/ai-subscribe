# Browser test framework

Story 1.6 的浏览器验收使用 Playwright，验证 React UI 与 `DesktopApi` 的契约接缝。真实 Tauri/MSVC/WebView2 证据由 Rust、Tauri 和 Windows smoke 测试负责。

## 项目隔离安装

所有依赖必须安装在仓库内。不要执行全局 npm、pnpm、Python 或 Playwright 安装。

```powershell
.\scripts\pnpm-env.cmd install --frozen-lockfile
.\scripts\pnpm-env.cmd run test:e2e:install
```

Node、pnpm、浏览器和缓存分别位于 `.toolchains/node`、`.toolchains/pnpm`、`.toolchains/playwright-browsers` 与项目 pnpm store。

## 运行方式

```powershell
# 全部 E2E
.\scripts\pnpm-env.cmd run test:e2e

# 仅 P0，或 P0 + P1
.\scripts\pnpm-env.cmd run test:e2e:p0
.\scripts\pnpm-env.cmd run test:e2e:p1

# 列出测试、运行单文件或按标题筛选
.\scripts\pnpm-env.cmd run test:e2e -- --list
.\scripts\pnpm-env.cmd run test:e2e -- tests/e2e/story-1-6-demo-intelligence.spec.ts
.\scripts\pnpm-env.cmd run test:e2e -- --grep "搜索"

# 其他项目隔离测试入口
.\scripts\rust-env.cmd cargo test --workspace --all-targets
.\scripts\pnpm-env.cmd test
```

## 结构与约定

- `tests/e2e/`：用户可见行为与验收场景。
- `tests/support/fixtures/`：自动注入 Tauri mock，并在 teardown 附加调用证据。
- `tests/support/factories/`：确定性、支持 override 的完整 DTO factory。演示合同不能使用随机数据。
- `tests/support/helpers/`：纯函数 command behavior 与调用记录类型。
- 测试名必须包含 `[P0]`、`[P1]`、`[P2]` 或 `[P3]`。
- 优先使用 role、accessible name 或稳定 id；不要依赖 CSS 层级。
- 动作前安装 mock；禁止 `waitForTimeout`、条件静默通过和测试间共享状态。
- 通过 factory override 表达场景差异；不得记录 secret、token 或私密详情。
- 当前应用只有本地 Tauri IPC，因此 HTTP/auth/Pact 测试为 N/A，不添加虚假覆盖。

Fixture 在应用脚本前安装 `window.__TAURI_INTERNALS__.invoke`，并阻断及记录 fetch、XHR、WebSocket、sendBeacon 和 Notification。断言使用 `invokeCalls()` 与 `externalCalls()`，不依赖控制台文本。

## CI 与故障排查

CI 使用 1 worker 和 2 次 retry，本地固定 4 workers。失败时保存 trace、截图、视频、JUnit 与 HTML 报告；这些目录均被 `.gitignore` 排除。

- 找不到浏览器：确认 `PLAYWRIGHT_BROWSERS_PATH` 指向 `.toolchains/playwright-browsers`，再运行项目内 `test:e2e:install`。
- 端口 4173 被占用：确认是否有本项目 Vite preview 残留；不要复用来源不明的服务。
- `desktop_contract_mismatch`：检查失败 trace 与 `tauri-invoke-calls` 附件，不要延长超时掩盖问题。
- 外联断言失败：查看 `external-calls` 附件，Story 1.6 不允许网络或通知权限调用。

权威演示数据仍来自 `contracts/fixtures/demo`；`tests/support` 只提供契约兼容的浏览器响应，不是产品事实源。
