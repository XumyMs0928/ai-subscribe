---
title: 'AI 大模型开发行业情报雷达 MVP'
type: 'feature'
created: '2026-08-06'
status: 'draft'
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** 大模型开发者需要持续跟踪模型、工具、论文和厂商动态，但信息分散、重复，难以快速判断价值。

**Approach:** 构建 Windows 优先的 Tauri 常驻工具，以 React 聚合 RSS、GitHub、Hugging Face、arXiv 和厂商博客；用 SQLite 去重保存，并通过 OpenAI-compatible API 生成中文摘要、标签和重要度评分。

## Boundaries & Constraints

**Always:** 本地优先；API Key 仅存本机且不入日志；无 Key 或离线时仍能查看缓存；单源失败不阻断同步；来源、关键词、刷新周期和模型可配置；托盘支持显示、隐藏、退出。

**Ask First:** 付费数据源、云同步、遥测、绕过反爬、扩展非 Windows 平台，以及安装系统级 Rust/Tauri 依赖。

**Never:** 抓取需登录的私域内容；提交真实密钥；自动操作第三方账户；把 AI 结论冒充原文事实。

## I/O & Edge-Case Matrix

| 场景 | 输入/状态 | 预期行为 | 异常处理 |
|---|---|---|---|
| 首启 | 空库、无 Key | 初始化默认来源与演示情报 | 提示模型未配置但不阻断 |
| 同步 | 新旧内容混合 | 标准化并按链接/外部 ID 去重 | 隔离单源错误 |
| AI 分析 | 有兼容 API | 生成摘要、标签、评分及理由 | 超时后保留原文并待重试 |
| 离线 | 有缓存 | 展示缓存与最后同步时间 | 非阻塞提示 |
| 配置错误 | 地址或周期无效 | 拒绝保存并定位字段 | 保留旧配置 |

</frozen-after-approval>

## Code Map

- `src/` -- React 仪表盘、筛选、详情、来源和模型设置。
- `src/lib/` -- 类型、状态、桌面能力接口和演示数据。
- `src-tauri/` -- 托盘、SQLite、定时同步、来源与 AI 适配器。
- `tests/` -- 去重、过滤、配置和失败隔离测试。

## Tasks & Acceptance

**Execution:**
- [ ] `package.json`, `vite.config.ts`, `tsconfig*.json` -- 初始化 React/TypeScript/Vite/Tauri 与质量命令。
- [ ] `src/App.tsx`, `src/styles.css`, `src/components/**` -- 实现深色紧凑仪表盘、情报流、搜索筛选、详情和设置状态。
- [ ] `src/lib/**` -- 定义领域模型、演示数据、状态及浏览器/Tauri 同形接口。
- [ ] `src-tauri/**` -- 实现桌面壳、托盘、数据库 schema、同步和 OpenAI-compatible 分析边界。
- [ ] `tests/**` -- 覆盖矩阵中的去重、筛选、无 Key、无效配置和故障隔离。
- [ ] `README.md`, `.env.example` -- 记录启动、配置、隐私边界和 Windows 打包前置条件。

**Acceptance Criteria:**
- Given 首次启动无配置, when 初始化完成, then 可浏览示例情报和默认来源并看到模型提示。
- Given 用户改变关键词、来源或评分条件, when 筛选执行, then 列表和统计同步更新并可清除。
- Given 用户打开情报, when 查看详情, then 显示摘要、评分理由、标签、时间和原文链接。
- Given 用户关闭窗口, when 托盘模式启用, then 进程常驻并可恢复或退出。
- Given 来源或 AI 服务失败, when 同步结束, then 其他结果保留且界面给出可操作状态。

## Spec Change Log

## Design Notes

前端只调用窄桌面接口，浏览器模式使用同形 mock，使未安装 Rust 时也能开发测试。采集统一输出 `IntelItem`；URL 与外部 ID 唯一。AI 分析是可重试增强步骤，不阻塞采集。

## Verification

**Commands:**
- `npm.cmd run typecheck` -- TypeScript 无错误。
- `npm.cmd test` -- 单元与组件测试通过。
- `npm.cmd run build` -- Web 构建成功。
- `npm.cmd run tauri build` -- 安装 Rust/Cargo/WebView2 后生成 Windows 安装包。
