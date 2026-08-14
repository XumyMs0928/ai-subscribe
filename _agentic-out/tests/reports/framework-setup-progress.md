---
stepsCompleted: ['step-01-preflight', 'step-02-select-framework']
lastStep: 'step-02-select-framework'
lastSaved: '2026-08-14T15:35:00+08:00'
inputDocuments:
  - 'package.json'
  - 'apps/windows/package.json'
  - 'Cargo.toml'
  - '_agentic-out/planning/architecture.md'
  - '_agentic-out/implementation/stories/1-6-浏览安全隔离的演示情报.md'
---

# Test Framework Setup Progress

## Step 1 — Preflight

- Detected stack: `fullstack`（React 19 + TypeScript 5.9 + Vite 8 Windows frontend；Rust 1.97.1/Tauri 2 backend）。
- Root `package.json`、Windows package manifest 和 Cargo workspace 均存在。
- Existing E2E framework configs: `0`；没有 Playwright/Cypress 冲突。
- Existing test layers: Vitest + Testing Library、Rust unit/integration/contract tests、Windows UI Automation smoke。
- Architecture context: DesktopApi 是唯一 React→Tauri 边界；MVP 没有账户/login/session/auth 服务；Story 1.6 只需要本地离线 demo 路径。
- Isolation constraint: 所有依赖、浏览器缓存和运行数据必须位于 `D:/2026/TEST1`，禁止修改全局 Python、PATH 或系统包管理器。
- Preflight result: PASS，允许进入框架选择。

## Step 2 — Framework Selection

- Browser framework: **Playwright**。
- Backend framework: Rust 原生 **cargo test**（保留既有 unit/integration/contract suites）。
- Rationale: 当前为 React/Vite + Tauri/Rust fullstack，Playwright 提供浏览器隔离、并行、trace/screenshot 和后续多浏览器扩展；与项目未来 Windows-first CI 更匹配。
- Boundary: Playwright 只验证浏览器中的 UI/DesktopApi contract seam；真实 Tauri command、MSVC release、WebView2 窗口和进程回收继续由 Rust/Tauri tests 与 `windows-demo-smoke.ps1` 验证。
- Cypress 未选择：其组件测试优势已由 Vitest/Testing Library 覆盖，且不会改善真实 Tauri runtime 证据。
