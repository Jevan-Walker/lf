# lf — Lightweight system info + cross-platform shell environment for AI agents

`lf` 是一个单二进制 CLI，为 AI agent 提供**统一的跨平台执行环境底座**：

1. **`lf info`** — 类似 fastfetch 的系统环境识别（OS / 内核 / CPU / RAM / 磁盘 / shell / 工具版本），支持 `--json` 输出给 agent 直接解析。
2. **`lf install [nu] [brush]`** — 自动安装 nushell 与 brush（已装则跳过），无需管理员权限，所有产物放到 `~/.lf/bin`，下载后做 SHA256 校验，并自动加入用户 PATH。
3. **`lf doctor`** — 校验 nu / brush 及常用工具（ffmpeg / git / cargo / node）是否就绪，JSON 输出 + 退出码可用于 CI 判断。
4. **`lf setup`** — 一键：装齐两个 shell → 跑 doctor。

## 用法

```console
$ lf info              # 文字展示（fastfetch 风格）
$ lf info --json       # 结构化输出，AI agent 可直接用
$ lf install           # 装 nushell + brush（缺啥装啥）
$ lf install nu --force  # 强制升级 nushell
$ lf install --no-path   # 只装二进制，不改 PATH
$ lf doctor            # 环境体检，退出码 0=就绪 / 1=缺依赖
$ lf setup             # install + doctor 一步到位
```

## 跨平台安装矩阵（预研结论，2026-08 核验）

| 目标 | 平台 | 方式 |
|------|-------|------|
| nushell | Windows / macOS / Linux (x86_64, aaarch64) | 官方 GitHub Release 直链下载 + SHA256SUMS 校验 |
| brush  | Linux / macOS (x86_64, aarch64) | 官方 Release targ.z 下载 + `.sha256` 校验 |
| brush  | Windows | 无官方二进制 → `cargo install --locked brush-shell`（需 Rust 工具链，编译约几分钟） |

> ⚠️ brush 目前**不发布 Windows 预编译二进制**（官方仅 Linux + macOS）；Windows
> 侧由 `lf` 自动降级为 `cargo install`。nushell 三平台均有官方二进制，无此问题。

## AI agent 使用建议

- agent 判断宿主环境：`lf info --json`（含 `os`、`cpu`、`memory`、`disk`、`shells`、`tools`）。
- agent 统一命令入口：让 agent 固定调用 `nu -c`（数据型）与 `brush -c`（bash 兼容），
  避免在 Bash / PowerShell / cmd 间切换造成脚本不通用。
- doctor 的退出码可以接入 agent 的 pre-flight 检查，缺少的 shell 让 agent 跑 `lf install` 自愈。

MIT License.