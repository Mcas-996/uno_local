# 项目结构与运行方式

本文面向希望从源码运行、调试或参与开发 UNO Star Carnival 的开发者。游戏是一个完全离线的 Rust 终端应用，使用 Ratatui 构建界面，并通过 Crossterm 处理跨平台终端输入与输出。

## 环境要求

- Windows、macOS 或 Linux
- Rust 1.91 或更高版本（建议通过 [rustup](https://rustup.rs/) 安装）
- 一个至少为 `70 × 22` 字符的终端窗口

程序不依赖外部服务，也不需要配置数据库或环境变量。终端达到 `70 × 26` 字符且支持相应图像协议时，程序会自动显示图像牌面；否则会安全地回退到彩色文本牌面。

## 项目结构

```text
uno_laptop_client/
├── .github/workflows/       # GitHub Actions 与 cargo-dist 发布流程
├── docs/                    # 开发说明、手工测试清单和演示资源
├── external/debug/          # cargo-dist/axoupdater 的调试辅助源码
├── openspec/                # 功能变更的 OpenSpec 设计、规格与任务记录
├── src/
│   ├── main.rs              # 程序入口、事件循环及终端状态恢复
│   ├── app.rs               # 页面状态、输入处理与本地对局流程
│   ├── core.rs              # 卡牌、牌组、规则、回合状态与游戏事件
│   ├── ai.rs                # 不同难度的本地 AI 决策
│   ├── ui.rs                # Ratatui 布局、组件和覆盖层渲染
│   ├── graphics.rs          # 终端图像协议探测、降级与预览缓存
│   ├── card_art.rs          # 以代码生成语言无关的 UNO 牌面位图
│   └── i18n.rs              # 英文和简体中文界面文本
├── Cargo.toml               # Rust 包信息、依赖和构建配置
├── Cargo.lock               # 已锁定的依赖版本
├── dist-workspace.toml      # cargo-dist 安装包与发布目标配置
└── README.md                # 项目简介和已发布版本的安装方式
```

主要调用关系如下：

```text
main（终端初始化与事件循环）
  ├── app（应用状态与输入） ──> core（规则）
  │                         └──> ai（电脑玩家）
  └── ui（界面渲染） ────────> graphics ──> card_art
                            └──> i18n
```

## 从源码运行

在仓库根目录执行：

```console
cargo run
```

第一次运行时 Cargo 会下载并编译依赖。编译完成后会直接进入设置页面，可以选择玩家名称、AI 数量、难度、牌组、语言和图像模式。

如需以优化后的发布配置运行：

```console
cargo run --release
```

查看命令行帮助或当前构建的版本信息，而不进入终端界面：

```console
cargo run -- --help
cargo run -- --version
```

`--` 用于将后面的参数传递给 `uno` 程序，而不是 Cargo。`-v` 与 `--version` 等价，版本输出同时包含 Cargo 包版本和构建所对应的 12 位 Git 提交号。当前程序不接受其他位置参数，所有对局选项都在 TUI 设置页面中完成。

## 构建与运行二进制文件

调试构建：

```console
cargo build
```

生成的程序位于：

- Windows：`target\debug\uno.exe`
- macOS / Linux：`target/debug/uno`

发布构建：

```console
cargo build --release
```

对应的程序位于 `target/release/` 目录。

构建后可以从任意目录查询版本，例如：

```console
target/release/uno --version
```

版本号和提交号已在编译时写入可执行文件。通过 README 中的安装脚本获得的发布版同样可以直接运行 `uno --version`，不会在运行时读取 `Cargo.toml`、`Cargo.lock` 或 `.git`。

## 开发检查

提交修改前建议执行：

```console
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

涉及终端渲染、键盘交互或图像协议的修改，还应按照[手工测试清单](manual-test.md)在目标终端中验证。

## 运行注意事项

- 终端小于 `70 × 22` 时，程序只显示调整窗口大小的提示。
- 图像牌面要求终端至少为 `70 × 26`；不满足条件时仍可使用完整的文本界面。
- 可在设置页面将 `Graphics` 切换为 `Text`，强制禁用图像输出。
- SSH 会话会自动使用文本牌面，避免图像协议转义序列干扰远程终端。
- 按 `Ctrl+C`、正常退出或发生 panic 时，程序都会尝试恢复终端的原始模式、光标和备用屏幕状态。

如果只想安装并运行已发布版本，请使用 [README](../README.md#quick-start) 中的安装命令。
