# Story 4.5 Blind Hunter

先完整读取并遵循 `D:/2026/TEST1/.agents/skills/agentic-adversarial-review/SKILL.md`。

只审 Story 4.5 的变更，不读取 Story/spec、planning、sprint 或既有 review 文档；只读，不运行测试，不修改文件。报告必须给出精确文件/行号、触发条件、影响和修复建议；禁止凑数或纯风格 finding。

## Group 1 — Core / SQLite / contracts / performance

- `contracts/fixtures/intel-feed/phase1-v1.json`
- `contracts/schemas/contract-manifest-v1.json` 的 intel-feed hunks
- `crates/radar-core/src/contracts/dto/intel_feed.rs`
- `crates/radar-core/src/contracts/dto/mod.rs` 的 intel-feed export
- `crates/radar-core/src/contracts/manifest.rs` 的 intel-feed hunks
- `crates/radar-core/src/application/intel_feed.rs`
- `crates/radar-core/src/application/mod.rs` 的 intel-feed export
- `crates/radar-core/src/application/sources.rs` 的相关可见性 hunk
- `crates/radar-core/src/infrastructure/database/intel_feed_repository.rs`
- `crates/radar-core/src/infrastructure/database/mod.rs` 的 intel-feed export
- `crates/radar-core/tests/sync_tasks.rs` 的 feed tests
- `target/story-4-5/intel-feed-performance.json`

## Group 2 — Tauri / DesktopApi / boundaries

- `crates/xtask/src/contracts.rs` 的 Story 4.5 hunks
- `crates/xtask/tests/generated_contract_gate_negative.rs` 的 handler hunk
- `apps/windows/src-tauri/src/commands/mod.rs` 的 intel-feed command/helper/tests
- `apps/windows/src-tauri/src/lib.rs` 的 handler hunk
- `apps/windows/src-tauri/tests/release_surface.rs` 的 allowlist hunk
- `apps/windows/src/lib/desktop-api/desktop-api.ts` 的 intel-feed DTO/guards/API
- `apps/windows/src/lib/desktop-api/tauri-desktop-api.ts` 的 query method
- `apps/windows/src/lib/desktop-api/intel-feed-transport.test.ts`
- `apps/windows/src/lib/query-client.ts` 的 intelFeedKeys

## Group 3 — React / UI / Playwright / regressions

- `apps/windows/src/app/router/app-router.tsx`
- `apps/windows/src/features/intel-feed/**`
- `apps/windows/src/styles/globals.css` 的 feed hunks
- `tests/support/factories/demo-dto.factory.ts` 的 feed hunks
- `tests/support/fixtures/demo-app.fixture.ts` 的 feed hunks
- `tests/support/helpers/tauri-command-mock.ts` 的 command hunk
- `tests/e2e/story-4-5-intel-feed.spec.ts`
- Story 1.6/1.7/1.8 E2E 中 `/demo` / real-root 回归 hunks

按 Group 1/2/3 分节输出 findings；没有发现的组明确写 `None`。
