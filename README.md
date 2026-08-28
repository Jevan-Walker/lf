# lf 为AI Agent提供统一的跨平台Shell执行底座
> 避免在 Bash / PowerShell / cmd 间切换造成脚本不通用。
## 功能介绍
- 类似 Fastfetch 的系统环境识别（OS / 内核 / CPU / RAM / 磁盘 / shell / 工具版本）的格式化输出。
- 一键环境补全：自动安装 nushell 与 brush（已装则跳过），无需管理员权限，所有产物放置在`~/.lf/bin`目录。
- 常用工具环境校验 `ffmpeg / git / cargo / node`等，为Agent提供内容格式化输出及退出码。

## 安装 
### 预编译版
查看[Release](https://github.com/lilyco-42/lf/releases)找到对应预编译的可执行文件。

### 从源码构建
> 需要预装Rust工具链
```Bash
git clone --depth=1 https://github.com/lilyco-42/lf.git
cd ./lf
cargo build
```

## 用法介绍
```console
$ lf info              # 文字展示（fastfetch 风格）
$ lf info --json       # 结构化输出，提供给AI agent
$ lf install           # 自动检测补全nushell + brush
$ lf install nu --force  # 强制升级 nushell
$ lf install --no-path   # portable安装
$ lf doctor            # 环境体检
$ lf setup             # install && doctor
```

## 平台安装支持参考

| 目标 | 平台 | 方式 |
|------|-------|------|
| nushell | Windows / macOS / Linux (x86_64, aaarch64) | 官方Release下载（经哈希校验） |
| brush  | Linux / macOS (x86_64, aarch64) | 官方Release下载（经哈希校验） |
| brush  | Windows | ⚠️ 无官方二进制，从源码编译（需等待编译） |

## 配合AI agent使用
- 判断宿主环境：`lf info --json`
- 统一命令入口：Agent调用 `nu -c`与 `brush -c`
- doctor退出码搭配agent的pre-flight检查，通过`lf install` 自愈。

## License
MIT License.
