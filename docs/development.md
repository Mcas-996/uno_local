# 项目结构与运行方式

本文面向希望从源码运行、调试或参与开发 UNO Star Carnival 的开发者。游戏是一个完全离线的 Rust 终端应用，由同一个 `uno` 二进制文件在运行时选择两套前端：通用 Crossterm 前端，以及 WezTerm 专用 Termwiz 前端。应用状态、规则、输入语义和布局模型由两套前端共享。

## 环境要求

- Windows、macOS 或 Linux
- Rust 1.91 或更高版本（建议通过 [rustup](https://rustup.rs/) 安装）
- 一个至少为 `70 × 26` 字符的终端窗口

程序不依赖外部服务，也不需要数据库。前端选择顺序固定如下：

1. SSH 或 tmux 会话强制使用通用文字模式。
2. 本地 WezTerm 会话使用 Termwiz 前端，默认显示图像，也允许在设置页切换为文字。
3. 其他本地终端使用通用前端；程序只查询一次终端能力，确认 Sixel 与单元格像素尺寸后才默认显示图像，否则使用文字。

## 项目结构

```text
uno_laptop_client/
├── .github/workflows/       # GitHub Actions 与 cargo-dist 发布流程
├── docs/                    # 开发说明与手工测试清单
├── external/debug/          # cargo-dist/axoupdater 的调试辅助源码
├── openspec/                # OpenSpec 设计、规格与任务记录
├── src/
│   ├── main.rs              # 程序入口、前端分派及异常恢复
│   ├── environment.rs       # SSH、tmux、WezTerm 环境分类
│   ├── frontend.rs          # 前端中立的输入、视口与显示类型
│   ├── app.rs               # 页面状态、输入处理与本地对局流程
│   ├── view.rs              # 两套前端共享的语义视图与导航
│   ├── screen.rs            # 自定义单元格缓冲区、布局和图像槽位
│   ├── universal.rs         # Crossterm 文字渲染和直接 Sixel 输出
│   ├── termwez.rs           # Termwiz Surface、输入和 WezTerm 图像
│   ├── core.rs              # 卡牌、牌组、规则、回合状态与游戏事件
│   ├── ai.rs                # 本地 AI 决策
│   ├── card_art.rs          # 生成语言无关的 UNO 牌面位图
│   └── i18n.rs              # 英文和简体中文界面文本
├── Cargo.toml
├── Cargo.lock
├── dist-workspace.toml
└── README.md
```

主要调用关系：

```text
main ──> environment ──> universal（Crossterm + 文字/Sixel）
  │                    └> termwez（Termwiz + 文字/PNG 图像）
  └──> app ──> core / ai
        └──> view / screen ──> card_art / i18n
```

项目不再依赖 Ratatui 或 `ratatui-image`，也没有应用层 Kitty/iTerm2 转义序列实现。通用前端通过 `icy_sixel` 直接编码 Sixel；WezTerm 前端把 PNG `EncodedFile` 交给 Termwiz 的 `Change::Image`。

## 从源码运行

```console
cargo run
cargo run --release
```

查看帮助和构建版本：

```console
cargo run -- --help
cargo run -- --version
```

`-v` 与 `--version` 等价。版本输出包含 Cargo 包版本和构建对应的 12 位 Git 提交号。所有对局选项都在设置页完成。

安装版可运行 `uno --uninstall` 查看并确认待删除路径，或用 `uno --uninstall -y` 跳过确认。只有 cargo-dist 收据与当前程序匹配时才会删除 `uno`、`uno-update` 和收据；源码、Cargo、包管理器或手动复制的版本会被拒绝。

## 构建与检查

```console
cargo build
cargo build --release
cargo fmt --check
cargo check --all-targets
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
```

调试程序位于 Windows 的 `target\debug\uno.exe` 或 macOS/Linux 的 `target/debug/uno`，发布程序位于 `target/release/`。

涉及终端渲染、键盘交互或图像的修改，还应按照[手工测试清单](manual-test.md)验证目标终端。

## 渲染与降级规则

- 通用前端启动时发送 Primary DA、`CSI 16 t` 和 DSR 查询，并在有限等待时间内解析响应。只有 Primary DA 声明 Sixel 且返回有效单元格像素尺寸时，才启用图像。
- Sixel 牌面按卡牌、目标单元格尺寸和像素尺寸缓存；移动、缩放、覆盖层或模式切换会清理旧区域。
- Termwiz 前端缓存 PNG 数据并通过 Surface 图像单元格显示；PNG 生成失败时留在 Termwiz 文字模式。
- Termwiz 初始化或终端 I/O 失败时，会保留当前 `App` 状态并切换到通用文字前端。
- 帮助、颜色选择、结果和退出确认覆盖层出现时不输出图像；小于 `70 × 26` 时只显示调整窗口提示。
- 正常退出、`Ctrl+C` 和 panic 都会尝试恢复原始模式、光标和主屏幕。

如果只想安装发布版，请使用 [README](../README.md#quick-start) 中的命令。
