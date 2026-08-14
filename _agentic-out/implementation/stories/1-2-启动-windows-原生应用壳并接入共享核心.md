---
artifact_kind: story
status: done
delivery_profile: standard
source_story: ''
baseline_commit: NO_VCS
---

# Story 1.2：启动 Windows 原生应用壳并接入共享核心

状态：done

## Story

作为 Windows 产品验证者，  
我希望启动连接共享核心的最小 Windows 原生应用，  
以便验证桌面平台具备安全、可诊断的实现基础。

## 验收标准

### AC1：最小 Windows 壳与核心健康状态

**Given** Story 1.1 的共享契约可用  
**When** 初始化锁定版本的 Tauri/React Windows 壳与 `DesktopApi`  
**Then** 应用可构建、启动并显示共享核心健康状态  
**And** UI 不直接访问 SQLite、Tauri `invoke` 或生成的底层绑定。（FR1、ARCH-5、ARCH-17）

### AC2：Windows—核心契约往返与秘密边界

**Given** Windows 平台提交 DTO、错误、effect 回报和短生命周期凭据测试输入  
**When** 通过 `DesktopApi` 往返共享核心  
**Then** 结果与 Rust 契约一致且重复 effect 不产生第二副作用  
**And** 测试明文只存在于 Windows 安全存储和受控内存租约，不进入数据库、日志或诊断。（ARCH-2、ARCH-11、ARCH-16）

### 可执行验收证据（细化原始 AC，不改变其语义）

- **AC1.1 Release 构建：** 使用 `x86_64-pc-windows-msvc` release 配置完成 Vite production build 与 Tauri build；release capability 解析结果只包含批准的产品 command，不含测试探针、远程源、`unsafe-eval`、shell、文件、HTTP 或 SQL 权限。
- **AC1.2 启动 smoke：** 在当前受支持 Windows 10/11 验证主机上启动 release 应用；60 秒内出现唯一主窗口，并在可访问状态区域显示由共享 core 返回的 `healthy` 与 `contract_version: 1`；随后由测试宿主请求正常退出并确认无残留进程。本 Story 记录 OS build、WebView2、MSVC/SDK、Node/pnpm、Rust 与 Tauri 版本。
- **AC2.1 Health/Error：** 独立测试宿主通过固定 command 读取 Story 1.1 的 health、validation failure、internal error golden，逐字段断言 `snake_case`、显式 `null`、稳定 error code/category/retryability/message_key/correlation_id；panic 不跨越 command 边界且底层文本零泄露。
- **AC2.2 Effect：** 对同一固定 idempotency key 依次执行 first/repeat/conflict，结果必须分别为 `Applied`、`AlreadyApplied`、稳定 conflict `AppError`，并由可观察计数证明副作用只发生一次。
- **AC2.3 Secret：** 测试专用 Rust 宿主以每轮唯一 credential target 写入运行时 canary，经 Windows 安全存储读取并交给 `SecretLeaseInput`；success/error/panic/repeat 后均删除测试 credential，目标测试 namespace 残留为 0，捕获的 stdout/stderr、日志、测试诊断 sink、IPC response、snapshot、fixture 与前端 bundle 对完整 canary 的命中为 0。
- 上述 contract/effect/panic/secret 探针只存在于 `cfg(test)`、独立测试二进制或专用 test config；不得注册进 release command table 或 release capability。

## Tasks / Subtasks

- [x] 1. 建立项目隔离、版本锁定的 Windows 工具链与 workspace（AC1）
  - [x] 在根目录增加 `package.json`、`pnpm-workspace.yaml`、`pnpm-lock.yaml`；workspace 仅包含 `apps/windows`，不得承载 Apple/Android 代码。
  - [x] 将官方 Node.js 24 LTS 解压到 `.toolchains/node`，pnpm store 放在 `.toolchains/pnpm-store`，通过 `scripts/node-env.cmd`/`scripts/pnpm-env.cmd` 调用且不修改用户/系统 PATH；wrapper 必须校验精确版本。根 `packageManager` 精确锁定 pnpm，禁止 `latest` 留在持久配置或锁文件生成脚本中。
  - [x] 为 Windows 壳增加项目隔离的 Rust MSVC target/wrapper：继续复用 `.toolchains/cargo` 与 `.toolchains/rustup`，但 Tauri 命令显式使用 `x86_64-pc-windows-msvc`，不得继承根 `.cargo/config.toml` 的默认 `gnullvm` target。以最小 spike 先证明 build/start；记录 MSVC Build Tools、Windows SDK 与 WebView2 发现结果，缺失时明确阻断并请求安装授权，不静默改全局 PATH/默认 Rust。
  - [x] 初始化 `apps/windows` 的 React + TypeScript + Vite 工程和 `src-tauri`；锁定 Tauri core `2.11.5`、`@tauri-apps/cli 2.11.4`，保存 Rust/pnpm 锁文件变化（当前工作区无 Git，不声称 commit/tracked-file 证据）。
  - [x] 使用 shadcn/ui + Tailwind CSS + Radix 基础，建立语义 token 与浅/深主题入口；只安装健康壳实际消费的最小组件，不批量生成未来业务组件。
  - [x] 更新根 README、`.gitignore`、编辑器配置和复跑命令；忽略项目隔离 Node/pnpm store、Vite/Tauri 构建产物、签名材料、用户数据与真实 secret。

- [x] 2. 建立最小 Tauri 宿主和严格平台边界（AC1）
  - [x] 创建 `apps/windows/src-tauri/src/{main.rs,lib.rs,commands/}`；release command allowlist 仅注册 `health_v1`，不暴露 SQL、文件、shell、任意 URL/HTTP、反射式 command 或任何测试探针。
  - [x] Tauri command 直接调用 `radar-core`/既有安全适配逻辑，不在宿主复制业务规则；保持 Rust panic containment 和稳定 `AppError` 映射。
  - [x] 配置严格 CSP 和最小 capabilities；下限为 `default-src 'self'`，按 Vite/Tauri 实际产物仅添加必要的本地资源指令，禁止 `unsafe-eval`、远程 origin、开发服务器地址、调试 capability 或宽权限插件；增加 release 配置解析/快照测试。
  - [x] 不安装 single-instance 插件，不实现托盘、关闭隐藏、开机启动、通知、NSIS、签名或自动更新；这些属于后续生命周期/发布 Story。

- [x] 3. 建立 `DesktopApi` 单一 UI 入口（AC1、AC2）
  - [x] 在 `apps/windows/src/lib/desktop-api/` 定义窄接口和 Tauri transport；只有该 infrastructure 层可以 import/call `@tauri-apps/api/core` 的 `invoke`。
  - [x] React components、hooks、providers、stores 和 feature 代码只依赖可注入的 `DesktopApi` 接口；测试 mock `DesktopApi`，不得 mock 全局裸 `invoke` 来掩盖边界违规。
  - [x] 保留 `contract_version`、不透明 ID、RFC3339 UTC、显式 `null` 和稳定 `AppError` 字段；TypeScript 可映射 `camelCase`，但不得改变语义或重新计算错误、effect、secret 规则。
  - [x] release `DesktopApi` 只提供 `health`；validation/internal error、effect first/repeat/conflict 与 secret probe 只通过 `cfg(test)`、独立测试宿主或 test-only transport 提供。禁止通用 `execute`/`invokeAny` API，并用边界测试证明测试方法未进入 production bundle。

- [x] 4. 实现可访问的最小 Windows 健康壳（AC1）
  - [x] 建立 `src/app/{providers,shell}/` 与最小入口；启动后通过 `DesktopApi` 查询共享核心健康状态，并显式呈现 loading、success、persistent error 三态。
  - [x] 使用 `DESIGN.md` 的 `color.*`、`type.*`、`space.*`、radius/border/focus/motion 语义；不得在页面硬编码颜色、间距、圆角或阴影。
  - [x] 使用 Segoe UI Variable、可见 focus ring、非纯颜色状态、稳定 accessible name/role；异步状态使用合适的 busy/live 语义，错误不得只用短暂 Toast。
  - [x] 验证键盘可达、浅/深主题、Windows 高对比度与至少 100%/200% 缩放；尊重 Reduce Motion。浅色/100% 已取得人工截图；其余真实运行矩阵经用户于 2026-08-14 批准延期至 Story 1.7，不计为本 Story 的 PASS 证据。
  - [x] 不伪造情报列表、详情、演示数据、默认赛道、配置引导或完整导航；健康页不代表 FR1 完整产品体验已交付。

- [x] 5. 建立 Windows 契约往返和幂等验证（AC2）
  - [x] Tauri/Rust tests 验证 health DTO、显式 null、validation/internal/panic 映射和 correlation ID 脱敏。
  - [x] 通过 Windows command → core `EffectLedger` 验证首次回报 `Applied`、完全重复 `AlreadyApplied`、不同终态稳定 conflict，且状态只变化一次。
  - [x] TypeScript contract tests 通过仓库相对路径的 test-only loader 读取 `contracts/fixtures/golden` 作为唯一 fixture 来源；不得复制到 `apps/windows`、手写第二份契约真相或打入 production bundle。
  - [x] 记录本 Story 只完成 Windows 局部调用链；UniFFI 生成物漂移、Swift/Kotlin 与三端统一 CI 归 Story 1.5。

- [x] 6. 建立 Windows 安全存储—短租约测试链路（AC2；NFR15/NFR20 派生约束）
  - [x] 平台适配层使用 Windows Credential Manager；若只保存小型 blob 可使用 current-user DPAPI，明确禁止 `LOCAL_MACHINE`。生产 command 不接受或返回任意明文字节。
  - [x] 从安全存储取出的明文只在 Rust 受控、可清零的瞬时内存链路中转为 `SecretLeaseInput`；调用结束、错误或 panic 后立即销毁，React 仅看到 configured/masked 状态。
  - [x] 使用测试专用 fake/受控入口生成 runtime canary；真实 Credential Manager 用例为每轮生成唯一、明确带测试 namespace 的 target/credential ID，以 RAII/finally 在 success/error/panic 后删除，且绝不枚举、读取或修改该 namespace 外的用户凭据。
  - [x] 不得把 canary 写入 fixture、源文件、前端 store、IPC 响应、快照或普通错误信息；每轮结束枚举测试 namespace 并断言残留 credential 数为 0。
  - [x] 捕获 Windows/Rust 测试 stdout/stderr、日志和诊断结果并断言 canary 零命中；覆盖 success、operation error、panic 与重复消费。
  - [x] 不实现完整 AI 凭据设置 UI/provider；该产品能力属于 Story 5.1。

- [x] 7. 演进 Story 1.1 工程边界门禁并建立完成证据（AC1、AC2）
  - [x] 将 `xtask contracts` 从“拒绝整个 `apps/**`”演进为仅允许本 Story 明确的最小 `apps/windows` 壳；继续拒绝 `apps/apple`、`apps/android`、migrations/数据库、业务 feature、通用高权限 commands 和敏感文件。
  - [x] 增加 mutation tests，证明未批准平台目录、裸 `invoke`（除 `lib/desktop-api`）、SQLite/SQL、任意 URL/file/shell API、secret 日志与复制 fixture 都会使门禁失败。
  - [x] 运行项目隔离 Rust 门禁：fmt、Clippy `-D warnings`、workspace tests、`xtask contracts`。
  - [x] 运行项目隔离 Windows 门禁：frozen pnpm install、format/lint/typecheck、Vitest/Testing Library、Vite production build、Tauri Rust tests、MSVC release build 与符合 AC1.2 的真实启动/退出 smoke。
  - [x] 至少执行 5 轮 Windows contract/test burn-in，记录版本、命令、结果、File List 和任何条件性跳过；不得把纯 Rust 1.1 证据冒充 Windows runtime 证据。

### Review Findings

- [x] [Review][Patch] 合同 JSON 漂移门禁只压缩文本、不实际解析 JSON，无效 JSON 可与期望值比较相等 [`crates/xtask/src/contracts.rs`:52]
- [x] [Review][Patch] 生产扫描在任意 `#[cfg(test)]` 文本处截断，注释或字符串可隐藏后续危险代码 [`crates/xtask/src/contracts.rs`:332]
- [x] [Review][Patch] 可进入 production bundle 的 `.test.ts(x)`/`src/test` 模块被两层边界扫描同时排除 [`crates/xtask/src/contracts.rs`:202]
- [x] [Review][Patch] release command allowlist 依赖可伪造的源码子串/首个 handler，注释可掩护额外 IPC command [`apps/windows/src-tauri/tests/release_surface.rs`:10]
- [x] [Review][Patch] 敏感制品门禁漏检 `.pfx`、`.p12`、`.secret` 等已由仓库忽略规则认定的凭据格式 [`crates/xtask/src/contracts.rs`:222]
- [x] [Review][Patch] Windows 范围门禁不是严格最小壳 allowlist，并对路径大小写及宽泛 `gen`/`dist` 排除存在绕过面 [`crates/xtask/src/contracts.rs`:178]
- [x] [Review][Patch] 裸 invoke 检测只覆盖少量精确文本形式，空白、内部入口或转发形式可逃逸 [`crates/xtask/src/contracts.rs`:338]
- [x] [Review][Patch] AC2 host 直接调用 core/FFI，validation、internal、panic、effect 未经过固定 Windows command 与序列化边界 [`apps/windows/src-tauri/tests/contract_host.rs`:1]
- [x] [Review][Patch] DesktopApi 仅校验 health 成功值，未解析并保留稳定结构化 AppError 字段 [`apps/windows/src/lib/desktop-api/tauri-desktop-api.ts`:9]
- [x] [Review][Patch] DesktopApi health 校验接受任意 `checked_at` 字符串，未守住 RFC3339 UTC 契约 [`apps/windows/src/lib/desktop-api/desktop-api.ts`:32]
- [x] [Review][Patch] effect “一次副作用”测试只统计返回枚举，不能证明真实可观察副作用只执行一次 [`apps/windows/src-tauri/tests/contract_host.rs`:72]
- [x] [Review][Patch] error redaction 测试从未把私密文本注入待映射错误，断言天然为真 [`apps/windows/src-tauri/src/commands/mod.rs`:79]
- [x] [Review][Patch] secret canary 以普通 String 和子进程环境变量传递，绕过受控可清零租约边界 [`apps/windows/src-tauri/src/platform/windows/secrets.rs`:84]
- [x] [Review][Patch] secret 零泄露证据只扫描 stdout/stderr，未动态覆盖诊断 sink、IPC、snapshot、fixture、bundle 等声明的输出面 [`apps/windows/src-tauri/src/platform/windows/secrets.rs`:166]
- [x] [Review][Patch] credential residue 检查只探测单个 target 且把读取错误当作不存在，无法证明测试 namespace 残留为零 [`apps/windows/src-tauri/src/platform/windows/secrets.rs`:92]
- [x] [Review][Patch] secret 子进程入口未强制测试 namespace，保留环境变量可导致测试跳过或触碰非测试 credential [`apps/windows/src-tauri/src/platform/windows/secrets.rs`:101]
- [x] [Review][Patch] safe-store 写入未在持久化前执行 SecretLeaseInput 等价校验，构造失败可留下不可消费 credential [`apps/windows/src-tauri/src/platform/windows/secrets.rs`:19]
- [x] [Review][Patch] 启动 smoke 的 `healthy$` 正则可把 `unhealthy` 当作健康 [`scripts/windows-smoke.ps1`:38]
- [x] [Review][Patch] 启动 smoke 统计进程而非顶层窗口，且未验证窗口可见/非屏外，不能证明唯一可见主窗口 [`scripts/windows-smoke.ps1`:22]
- [x] [Review][Patch] smoke 从扁平可访问名称的相邻值推断 contract version，可能产生假阳性/假阴性 [`scripts/windows-smoke.ps1`:39]
- [x] [Review][Patch] AppShell IPC 无超时，永久 pending 会把 UI 困在不可恢复的 loading 状态 [`apps/windows/src/app/shell/app-shell.tsx`:18]
- [x] [Review][Patch] AppShell 重试缺少请求世代/取消保护，旧请求可覆盖新 DesktopApi 结果并在卸载后更新状态 [`apps/windows/src/app/shell/app-shell.tsx`:24]
- [x] [Review][Patch] release CSP/capability 测试只拒绝少数模式而非精确 allowlist，额外来源或权限可在门禁绿色时进入 [`apps/windows/src-tauri/tests/release_surface.rs`:32]
- [x] [Review][Patch] CSP 允许未证明必要的 `style-src 'unsafe-inline'`，与严格最小本地资源策略不一致 [`apps/windows/src-tauri/tauri.conf.json`:12]
- [x] [Review][Patch] Rust MSVC wrapper 用未编码的 `%LIB%` 拼接 RUSTFLAGS，项目路径含空格时参数会被拆分 [`scripts/rust-msvc-env.cmd`:15]
- [x] [Review][Deferred][User-approved 2026-08-14] UX 测试只搜索 CSS 文本，未实际验证深色、高对比度、Reduce Motion 与 200% 缩放行为；真实运行矩阵转 Story 1.7 [`apps/windows/src/test/ux-foundation.test.ts`:26]
- [x] [Review][Patch] Story 启动证据未完整记录所用 MSVC/SDK（或替代 sysroot）具体版本 [`_agentic-out/implementation/stories/1-2-启动-windows-原生应用壳并接入共享核心.md`:220]
- [x] [Review][Patch] 健康成功标记复用 signal-primary 青色，违反青色仅用于主操作、焦点和选中的 UX 语义 [`apps/windows/src/styles/globals.css`:172]
- [x] [Review][Patch] release 未设置 Windows GUI subsystem，实际启动产生额外控制台窗口 [`apps/windows/src-tauri/src/main.rs`:1]
- [x] [Review][Deferred][User-approved 2026-08-14] 在可提供 WebView2 UI Automation provider 的交互式 Windows 会话复跑严格 smoke；健康 UIA 证据转 Story 1.7，当前保留视觉截图与 contract version UIA 证据 [`scripts/windows-smoke.ps1`:1]

## Dev Notes

### 权威范围与依赖

- Story 1.1 已 `done`：32/32 Rust tests、12/12 trace FULL、gate PASS。复用现有 `health_check`、`HealthStatusWire`、`AppError`、`EffectLedger`、`SecretLeaseInput`、manifest/snapshots/golden fixtures 和 `scripts/rust-env.cmd`。
- 当前拆分后的 Epics/Stories 是交付范围权威。架构中“首个实现 Story 同时建立三端壳/绑定”的旧聚合文字已被 Stories 1.1–1.5 拆分取代。
- FR1 在本 Story 只获得 Windows 壳/无阻断启动的部分基础证据；可浏览演示数据和完整首次体验分别由 Stories 1.6–1.8 完成。
- `DesktopApi` 内部可以且必须封装 Tauri `invoke`；“UI 不直接 invoke”指页面、组件、hooks、providers 与 stores 不得绕过它。

### 架构与安全护栏

- 数据流固定为 `React View → Hook/Provider → DesktopApi → Tauri command → radar-core → DTO/AppError → Render State`。
- Rust/JSON 权威保持 `snake_case`；TypeScript presentation mapping 可使用 `camelCase`。不得形成另一套 DTO、错误码、effect 状态机或 secret 生命周期规则。
- 不引入本地 Web Server、REST/GraphQL/gRPC、PWA、SSR 或 Service Worker。
- WebView 不接触真实 secret；安全存储读取与 lease 消费都留在 Rust 平台层。任何为测试方便而让生产 command 接收任意 secret bytes 的实现均不合格。
- Release command/capability 只允许 `health_v1`。错误、effect、panic 与 secret 合同由测试宿主验证，不能以“测试需要”为由扩张生产攻击面。
- `radar-core` 仍不得依赖 Tauri/React/Windows SDK；平台依赖只在 `apps/windows/src-tauri`。
- 本 Story 的“诊断零泄露”只覆盖捕获日志和测试诊断 sink，不提前实现 Story 8.5 的诊断导出；安全存储只交付适配器/租约验证，不实现 Story 5.1 的凭据 CRUD UI、provider 或用户配置持久化。

### 最小文件结构

```text
package.json
pnpm-workspace.yaml
pnpm-lock.yaml
apps/windows/
  package.json
  vite.config.ts
  tsconfig*.json
  components.json
  index.html
  src/
    main.tsx
    app/providers/
    app/shell/
    components/ui/
    lib/desktop-api/
    styles/
    test/
  src-tauri/
    Cargo.toml
    build.rs
    tauri.conf.json
    capabilities/
    src/main.rs
    src/lib.rs
    src/commands/
    src/platform/windows/secrets.rs
```

- 不为了匹配完整 architecture tree 预建空 `features/`、router、stores、database、notifications、scheduler 或 external-links 模块。
- React 文件使用 `kebab-case.tsx`；Rust/TypeScript 类型使用 `PascalCase`，普通函数/变量使用 `snake_case`/`camelCase` 的语言惯例；React tests 与实现共置为 `*.test.ts(x)`。

### UX 验收下限

- 视觉权威：`ux/DESIGN.md`；行为/状态权威：`ux/EXPERIENCE.md`。
- 视觉方向为“冷静决策台 / 信号青”；青色只用于主操作、焦点、选中，红色只用于真实错误/数据风险/破坏性状态。
- 本 Story 可只落地 shell/workspace 骨架和健康状态，不能显示不可操作的虚假业务导航或用 Card 墙替代未来信息密度布局。
- 组件必须允许后续接入 Sidebar、React Router、TanStack Query 和纯 UI Zustand，但本 Story 不应为未交付业务提前创建巨型 provider/store。

### 技术版本与官方核验（2026-08-13）

- 架构锁定 Tauri core `2.11.5`、`@tauri-apps/cli 2.11.4`；官方 release 页与 changelog 确认该组合存在。
- 使用 Node.js `24.18.0` LTS（Krypton）作为本 Story 项目隔离验证基线；不要使用已 EOL 的 Node 20。
- 锁定 pnpm `11.15.1`；写入根 `packageManager` 并以 frozen lockfile 安装。
- React 使用稳定 `19.2.x` 并精确锁定实际安装 patch；Vite/TypeScript/Vitest/Testing Library 也必须由生成后的 lockfile 精确固定，不在 Story 文档猜测未核验版本。
- shadcn CLI 当前默认 primitive 已不固定为 Radix；初始化必须显式选择 `--base radix`，不得因默认值变化偏离 DESIGN 的 Radix 决策。
- TanStack Query `5.101.4`、Zustand `5.0.14`、React Router `8.3.0` 是后续 Windows 架构锁定值；本 Story 若无实际消费，不为“预留”而安装。
- Tauri single-instance 插件必须作为首插件注册，但它属于后续 FR63/NFR35 生命周期 Story；本 Story不安装。
- 根 `.cargo/config.toml` 的 `gnullvm` 仍服务 Story 1.1 的纯 Rust 门禁；Tauri Windows 构建必须由 wrapper 显式覆盖为项目隔离 `x86_64-pc-windows-msvc`，两条链路都要分别复跑，避免一个 target 的配置污染另一个。

### 测试设计与完成真实性

- Test Design 产物当前缺失；standard profile 允许继续，但 AC1/AC2 的 Windows build/start、DesktopApi boundary、contract round-trip 与 secret zero-hit 都是阻断性完成条件。
- 前端组件测试 mock `DesktopApi` 接口；transport test 单独验证 invoke command name/payload/error mapping。不要在组件测试全局 mock `invoke`。
- 建议静态边界测试扫描 imports 和 source tree，动态测试验证真正 Tauri command/core 路径。仅测试 mock 或快照不能证明 Windows 往返。
- 若环境无法完成真实 Tauri 启动、Windows Credential Manager 或 WebView2 smoke，必须明确标记 blocker/partial evidence，不得将 Story 置为 done。
- 健康壳在本 Story 只验键盘、焦点、主题、高对比和 100%/200% 缩放；列表/详情、完整 NVDA 旅程及全响应式矩阵归 Story 1.7。

### Planning Gate

- Delivery profile：`standard`；source：schema-v2 compatibility fallback；风险信号 `clear-requirement + single-component` 与 standard 一致，无有意覆盖。
- PRD、UX、UX spine、Architecture、Epics、Readiness、Sprint Status 均完整；Story 1.1 before-done gate 已 PASS 并封版为 `story-1.1-complete`。
- Test Design 缺失，属于 recommended warning，不阻止创建/开发；实现必须用本 Story 的测试任务补足可执行证据。

### Previous Story Intelligence

- Story 1.1 建立了项目隔离 Rust 1.97.1 + gnullvm/LLVM-MinGW；不得回退全局 Cargo，也不要让 Node 安装污染全局 PATH。
- 合同镜像必须由 Rust 权威生成/校验，fixture 必须执行语义，禁止同源自比较。
- `AppError` 底层文本必须脱敏且 correlation ID 唯一；effect 必须真正以 idempotency key 管理；secret 必须覆盖 success/error/panic/constructor failure 清零和真实输出 canary 扫描。
- Story 1.1 的边界 scanner 是安全资产，不是一次性障碍；新增 Windows 壳时应做最小 allowlist 演进和独立 mutation tests。
- 当前目录不是 Git 仓库，`baseline_commit=NO_VCS`；不得谎称 commit、tracked-file 或 PR 证据。

### References

- [Source: `_agentic-out/planning/epics.md`，Epic 1 与 Story 1.2，L395–433]
- [Source: `_agentic-out/planning/prd.md`，FR1、NFR15、NFR20、NFR28 与 Windows 平台边界]
- [Source: `_agentic-out/planning/architecture.md`，起始模板/版本，L114–206；安全，L468–508；平台绑定，L521–580；Windows React，L750–933；项目结构，L1298–1610]
- [Source: `_agentic-out/planning/ux/DESIGN.md`，Windows foundation、tokens、layout、focus/motion]
- [Source: `_agentic-out/planning/ux/EXPERIENCE.md`，Windows shell、状态、无障碍与平台边界]
- [Source: `_agentic-out/implementation/stories/1-1-建立共享核心与版本化契约基线.md`，完成证据、Review Findings、RCL-001–RCL-010]
- [Tauri create project](https://v2.tauri.app/start/create-project/)
- [Tauri ecosystem releases](https://tauri.app/release/)
- [Tauri single-instance](https://v2.tauri.app/plugin/single-instance/)
- [Node.js releases](https://nodejs.org/en/about/previous-releases)
- [React versions](https://react.dev/versions)
- [shadcn/ui installation](https://ui.shadcn.com/docs/installation)

## Dev Agent Record

### Agent Model Used

OpenAI Codex（GPT-5）

### Implementation Plan

- 按 Story 任务顺序执行 RED→GREEN→REFACTOR：先建立项目隔离工具链，再实现 Tauri/DesktopApi/健康壳，随后补齐合同、凭据和工程门禁。
- Rust 纯核心保持 gnullvm；Windows Tauri 显式使用项目内 MSVC sysroot 与 `x86_64-pc-windows-msvc`，两条链路独立验证。
- release 攻击面只保留 `health_v1`；effect/error/panic/secret 仅在 Rust 测试宿主验证；Windows 凭据使用当前用户会话 Credential Manager。

### Debug Log References

- 2026-08-13：`agentic-flow` reconcile 完成，drift=0、stale=0；Story 1.1 before-done gate PASS，封版 `story-1.1-complete`。
- 2026-08-13：Delivery profile 解析为 `standard`；Test Design 缺失作为 recommended warning 记录。
- 2026-08-13：Ultimate context engine analysis completed - comprehensive developer guide created。
- 2026-08-13：独立 checklist 复核完成；已修正 MSVC target 隔离、release/test command 分离、Credential Manager 测试清理和可执行验收证据。
- 2026-08-13：Task 1 项目隔离工具链完成；Node 24.18.0、pnpm 11.15.1、Rust 1.97.1、MSVC sysroot/LLVM 均位于 `.toolchains`，未使用 Python 或全局安装器。项目内 LLVM/Clang 为 22.1.8，兼容 sysroot 自带 README 标识 Windows Kits 10.0.28000.0；系统 MSVC Build Tools/SDK 未发现。MSVC release build 通过；沙箱外真实 smoke 获得唯一窗口 `AI Subscribe`、正常退出码 0、零残留进程。主机 OS build 26200，WebView2 Runtime 151.0.4129.78；未修改全局环境。
- 2026-08-13：Task 7 全门禁通过。Rust gnullvm：fmt、Clippy、32/32 tests、xtask PASS；Windows：frozen install、format/lint/typecheck、5 files/11 Vitest、Vite build、MSVC Clippy、8/8 Tauri tests、Tauri CLI release build 均 PASS。`windows-smoke.ps1` 在 60 秒内验证唯一 `AI Subscribe` 窗口、可访问 `healthy`/`contract_version: 1`、退出码 0、零残留进程，WebView2 数据仅写项目 `target`。
- 2026-08-13：Windows burn-in 5/5 PASS；每轮执行 `rust-msvc-env.cmd test -p ai-subscribe-desktop --all-targets --quiet`（8 tests）与 `pnpm-env.cmd --filter @ai-subscribe/windows test:burn-in`（11 tests），无 skip/ignore/条件性跳过。版本：Windows build 26200、WebView2 151.0.4129.78、Node 24.18.0、pnpm 11.15.1、Rust 1.97.1、Tauri CLI 2.11.4/core 2.11.5。
- 2026-08-14：一次性完整代码审查的 28 项原始 patch finding 已修复 27 项；新增严格窗口枚举实际发现 release 缺少 GUI subsystem、产生额外控制台窗口，已补 `windows_subsystem = "windows"`。项目隔离 gnullvm/MSVC fmt、Clippy、Rust tests、xtask contracts、frozen pnpm、format/lint/typecheck、14/14 Vitest、Vite/Tauri release build 均 PASS。严格 smoke 已证明单进程、单一可见顶层窗口和非屏外，但当前自动化会话没有暴露 WebView2 UI Automation 内容元素，故不能重新声称可访问健康文本已 PASS。
- 2026-08-14：获准继续补齐两项真实运行证据后，按 `computer-use` 规则初始化 Windows 交互控制；`@oai/sky` 在模块解析/应用枚举前持续因 `EPERM: lstat C:\Users\13479\AppData\Local\OpenAI\Codex` 失败。仅只读目录授权确认该目录存在，但权限未传播到独立 Windows 控制助手；重置会话并重试仍同样失败。未执行 UI 输入、未改变系统主题/缩放、未安装任何包，也未用 PowerShell UI Automation 替代真实交互证据；两项 finding 保持未完成。
- 2026-08-14：用户提供真实运行截图，确认浅色/100% 下唯一可见产品界面显示 `共享核心 healthy` 与 `contract_version: 1`，无明显裁切；只读系统检查为 DPI 96（100%）、高对比度关闭、动画开启。修复 smoke 将 Tao 内部无标题 `Tao Thread Event Target` 与产品窗口区分，并修复额外窗口误计；前端补充健康精确 accessible name，14/14 Vitest、format、lint、typecheck、Vite 与 Tauri release build 通过。最终 10 秒严格 smoke 证明唯一产品窗口、非屏外、`contract_version: 1` UIA 可见、正常失败清理后残留进程 0，但 WebView2 仍将视觉可见的健康名称节点报告为 offscreen，故健康 UIA 证据保持 FAIL；未继续重复长等待。
- 2026-08-14：用户批准将 WebView2 health UIA 与深色/高对比度/Reduce Motion/200% 真实矩阵延期至 Story 1.7。Story 1.2 traceability 决策为 `WAIVED`，NFR 为 `CONCERNS_WITH_APPROVED_WAIVER`；无 Critical blocker，Windows-first 里程碑可继续。

### Completion Notes List

- Story context engine analysis completed - comprehensive developer guide created。
- Critical review findings resolved；Story 保持 `ready-for-dev`，尚未开始依赖安装或实现。
- Task 1 完成：建立精确版本、项目隔离的 Node/pnpm/Rust Windows 构建链，修复嵌套 pnpm 命中全局版本的问题，并验证 frozen install、Vite production build、MSVC release build 和真实启动/退出 smoke。
- Task 2 完成：release command table 仅包含 `health_v1`，CSP/能力最小化测试改为解析配置字段，避免把 JSON schema 元数据误判为远程运行时 origin；MSVC Tauri 8/8 tests 通过。
- Task 3 完成：`DesktopApi` 是唯一 invoke 边界，组件只注入接口；修复 Vitest 环境中 `import.meta.url` 非 file scheme 导致的扫描失败，改用稳定 workspace 路径，3 files/6 tests 通过。
- Task 4 完成：健康壳实现 loading/success/persistent-error 三态与键盘重试；按 DESIGN 落地精确 color/type/space/radius/border/focus/motion tokens、浅深主题、forced-colors、rem 缩放和 Reduce Motion。补充 DOM cleanup 消除测试串扰；format/lint/typecheck、4 files/9 tests、production build 全通过。
- Task 5 完成：Windows Rust 宿主逐字段验证 health/error/panic 与 effect first/repeat/conflict，并证明 Applied 仅一次；TypeScript test-only loader 直接读取仓库 Rust golden fixtures。Windows Rust 8/8、前端 5 files/11 tests 通过；UniFFI/Swift/Kotlin/三端 CI 明确保留给 Story 1.5。
- Task 6 完成：Windows Credential Manager 使用当前用户 Session persistence（无 `LOCAL_MACHINE`、无 durable/global credential），运行时 PID+原子轮次生成唯一 target/canary；RAII 清理、受控 namespace 精确残留计数、stdout/stderr 零命中覆盖 success/error/panic/repeat。生产 release 未注册任何 secret command，完整凭据 UI 保留给 Story 5.1。
- Task 7 完成：xtask 最小放行 Windows 壳并新增独立边界 mutation cases；修复临时测试目录依赖系统时间的非确定性问题。项目隔离 Rust/Windows 全门禁和 5 轮 burn-in 全通过；新增可复跑 UI Automation smoke，未使用 Python、全局包或系统安装器。
- Code review 批量修复完成 27/28 个代码/测试问题；其余两项为真实运行证据缺口，并非未修复生产缺陷。用户已批准转 Story 1.7；Story 1.2 以 `WAIVED` 门禁收口为 `done`，不把延期证据声明为 PASS。

### File List

- `_agentic-out/implementation/stories/1-2-启动-windows-原生应用壳并接入共享核心.md`
- `.editorconfig`
- `.gitignore`
- `Cargo.lock`
- `Cargo.toml`
- `.npmrc`
- `README.md`
- `package.json`
- `pnpm-lock.yaml`
- `pnpm-workspace.yaml`
- `scripts/clang-cl.cmd`
- `scripts/node-env.cmd`
- `scripts/pnpm-env.cmd`
- `scripts/rust-msvc-env.cmd`
- `scripts/windows-smoke.ps1`
- `crates/xtask/src/contracts.rs`
- `crates/xtask/tests/generated_contract_gate_negative.rs`
- `apps/windows/src-tauri/Cargo.toml`
- `apps/windows/src-tauri/build.rs`
- `apps/windows/src-tauri/capabilities/main.json`
- `apps/windows/src-tauri/icons/128x128.png`
- `apps/windows/src-tauri/icons/128x128@2x.png`
- `apps/windows/src-tauri/icons/32x32.png`
- `apps/windows/src-tauri/icons/64x64.png`
- `apps/windows/src-tauri/icons/app-icon.svg`
- `apps/windows/src-tauri/icons/icon.ico`
- `apps/windows/src-tauri/icons/icon.png`
- `apps/windows/src-tauri/src/commands/mod.rs`
- `apps/windows/src-tauri/src/lib.rs`
- `apps/windows/src-tauri/src/main.rs`
- `apps/windows/src-tauri/tauri.conf.json`
- `apps/windows/src-tauri/tests/release_surface.rs`
- `apps/windows/src-tauri/tests/windows_secret_store.rs`
- `apps/windows/src/lib/desktop-api/desktop-api.ts`
- `apps/windows/src/lib/desktop-api/tauri-desktop-api.test.ts`
- `apps/windows/src/lib/desktop-api/tauri-desktop-api.ts`
- `apps/windows/src/test/source-boundaries.test.ts`
- `apps/windows/.prettierignore`
- `apps/windows/components.json`
- `apps/windows/eslint.config.js`
- `apps/windows/index.html`
- `apps/windows/package.json`
- `apps/windows/src/app/providers/desktop-api-context.ts`
- `apps/windows/src/app/providers/desktop-api-provider.tsx`
- `apps/windows/src/app/providers/use-desktop-api.ts`
- `apps/windows/src/app/shell/app-shell.test.tsx`
- `apps/windows/src/app/shell/app-shell.tsx`
- `apps/windows/src/components/ui/button.tsx`
- `apps/windows/src/lib/utils.ts`
- `apps/windows/src/main.tsx`
- `apps/windows/src/styles/globals.css`
- `apps/windows/src/test/setup.ts`
- `apps/windows/src/test/ux-foundation.test.ts`
- `apps/windows/tsconfig.app.json`
- `apps/windows/tsconfig.json`
- `apps/windows/tsconfig.node.json`
- `apps/windows/vite.config.ts`
- `apps/windows/src-tauri/tests/contract_host.rs`
- `apps/windows/src/test/contract-fixtures.test.ts`
- `apps/windows/src/test/load-golden-fixture.ts`
- `apps/windows/src-tauri/src/platform/mod.rs`
- `apps/windows/src-tauri/src/platform/windows/mod.rs`
- `apps/windows/src-tauri/src/platform/windows/secrets.rs`

### Change Log

- 2026-08-13：完成 Story 1.2 Windows Tauri/React 最小健康壳、`DesktopApi` 单一边界、Windows Credential Manager 短租约链路、严格 CSP/capability、合同/幂等/可访问性测试和 Story 1.1 边界门禁演进；全量门禁、真实启动退出 smoke 与 5/5 burn-in 通过，状态更新为 `review`。
- 2026-08-14：完成代码审查批量整改 27/28，并修复审查中实际复现的 release 控制台窗口；严格 UIA smoke 与真实 UX 环境矩阵仍缺最终证据，状态更新为 `in-progress`。
- 2026-08-14：用户批准将两项真实运行证据转 Story 1.7；完成 traceability 与 NFR 评估，确定性门禁为 `WAIVED`、无 Critical blocker，Story 1.2 状态更新为 `done`。
