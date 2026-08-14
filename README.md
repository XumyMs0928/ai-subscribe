# ai-subscribe

当前仓库包含共享 Rust 核心、版本化合同，以及 Story 1.2 的最小 Windows Tauri/React 健康壳。所有开发工具、包缓存与构建依赖均隔离在项目目录；脚本不会修改用户或系统 PATH，也不会安装或调用 Python 包。

## 项目隔离工具链

项目内固定版本：

- Rust 1.97.1、rustfmt、Clippy：`.toolchains/cargo`、`.toolchains/rustup`
- Node.js 24.18.0：`.toolchains/node`
- pnpm 11.15.1：`.toolchains/pnpm`，store/cache 位于 `.toolchains/`
- LLVM-MinGW 与 MSVC 兼容 sysroot：`.toolchains/llvm-mingw`、`.toolchains/windows-msvc-sysroot-cache`

所有 wrapper 仅为其子进程设置环境变量，找不到项目内工具或版本不匹配时直接失败，不会回退到全局安装。

## 复跑命令

纯 Rust 合同链继续使用隔离的 `x86_64-pc-windows-gnullvm`：

```powershell
.\scripts\rust-env.cmd fmt --all --check
.\scripts\rust-env.cmd clippy --workspace --exclude ai-subscribe-desktop --all-targets -- -D warnings
.\scripts\rust-env.cmd test --workspace --exclude ai-subscribe-desktop --all-targets
.\scripts\rust-env.cmd run -p xtask -- contracts
```

Windows 前端链使用项目内 Node/pnpm：

```powershell
.\scripts\pnpm-env.cmd install --frozen-lockfile
.\scripts\pnpm-env.cmd format:check
.\scripts\pnpm-env.cmd lint
.\scripts\pnpm-env.cmd typecheck
.\scripts\pnpm-env.cmd test
.\scripts\pnpm-env.cmd build
```

Tauri 的 Windows Rust 链显式覆盖为 `x86_64-pc-windows-msvc`，不会继承根 `.cargo/config.toml` 的 gnullvm 默认目标：

```powershell
.\scripts\rust-msvc-env.cmd test -p ai-subscribe-desktop --all-targets
.\scripts\rust-msvc-env.cmd clippy -p ai-subscribe-desktop --all-targets -- -D warnings
.\scripts\pnpm-env.cmd --filter @ai-subscribe/windows tauri build --runner D:\2026\TEST1\scripts\rust-msvc-env.cmd --target x86_64-pc-windows-msvc --no-bundle --ci
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\windows-smoke.ps1
```

Tauri build 中的绝对 runner 路径请在仓库移动后替换为当前项目路径。`windows-smoke.ps1` 仅启动项目 `target` 中的 release 可执行文件，把 WebView2 数据重定向到项目内，并验证可访问健康文本、正常关闭和零残留进程。

`apps/windows` 只交付健康壳与 `DesktopApi` 边界，不包含业务数据库、Apple/Android 壳、凭据设置 UI、单实例、托盘、签名或自动更新。

## 干净环境准备原则

下载的官方 Node/Rust 便携包、pnpm、Windows MSVC sysroot 和缓存只能写入 `.toolchains/`。Rust 安装必须把 `CARGO_HOME`/`RUSTUP_HOME` 指向项目目录并使用 `rustup-init --no-modify-path`。不得运行系统级安装器，不得修改全局 Python、Node、pnpm、Cargo 或 PATH。
