# UI、终端图像与牌面生成

界面由共享语义层和两套终端适配层组成。`App` 不依赖 Crossterm 或 Termwiz，因此前端切换和故障降级不会重建或丢失对局。

## 分层

```text
App / core / ai
      │
      ├── view.rs：语义页面、手牌换行和导航
      └── screen.rs：布局、自定义 Cell 缓冲区、图像槽位
              │
              ├── universal.rs：Crossterm 差量文字 + 直接 Sixel
              └── termwez.rs：Termwiz Surface + PNG ImageData
                                      │
                                  card_art.rs
```

`frontend.rs` 定义前端中立的按键、修饰键、视口和显示状态。两套事件循环只负责把各自的终端事件转换成这些类型，再调用同一个 `App::handle_key`。

## 共享屏幕模型

`screen.rs` 的 `Canvas` 保存字符、前景色、背景色和样式，并以 Unicode 显示宽度推进光标。设置、游戏、帮助、结果、退出确认和万能牌选色都通过同一套布局绘制。

终端小于 `70 × 26` 时只画调整尺寸提示。游戏页在无覆盖层时产生 Selected 和 Discard 两个语义图像槽位；帮助、颜色选择、结果和退出确认会抑制图像，使弹窗不会被终端图像层遮挡。文字模式仍显示完整、带规则颜色的牌面名称。

手牌显示和上下导航共享 `view.rs` 的贪心换行结果。上下键会在相邻行选择横向中心最近的牌，避免显示与导航采用不同宽度算法。

## 通用前端：文字与 Sixel

通用前端基于 Crossterm，但不使用 Ratatui。它维护上一帧 `Canvas`，只把发生变化的单元格写入终端。

启动时只进行一次能力查询：

- Primary Device Attributes，用参数 `4` 确认 Sixel；
- `CSI 16 t`，获得单元格像素高宽；
- DSR，标记响应边界。

解析器有有限等待时间。响应缺失、格式无效或没有同时满足两项能力时，前端使用文字，不会猜测支持情况。SSH 和 tmux 会在查询前被环境分派器强制为文字。

图像由 `card_art.rs` 生成 RGBA 位图，按目标槽位的字符尺寸和单元格像素尺寸缩放，再由 `icy_sixel` 直接编码。缓存键包含 `Card`、目标字符尺寸和单元格像素尺寸。写入时保存光标、绝对定位到图像槽位、输出 Sixel，再恢复光标。牌面移动、窗口缩放、覆盖层或模式改变时会清理旧矩形；编码失败后本次运行稳定降级为文字。

## WezTerm 前端：Termwiz

本地 WezTerm 使用 Termwiz 创建终端和 `BufferedTerminal`。共享 `Canvas` 被复制到 `Surface`，图像槽位则追加 `Change::Image`。

牌面以 PNG 编码并缓存为 `Arc<ImageData>`，数据类型保持 `ImageDataType::EncodedFile`。具体终端协议由 Termwiz/WezTerm 处理，应用不拼接 Kitty 或 iTerm2 转义序列。PNG 生成失败只把本前端切到文字；Termwiz 初始化、绘制或输入 I/O 失败则由 `main` 捕获，保留原 `App` 并进入通用文字前端。

## 牌面生成

`card_art.rs` 以代码生成固定 `180 × 270` RGBA 位图，不依赖资源路径或系统字体。圆角、边框、中央椭圆、万能牌四色区域、禁止符号和 5×7 点阵标签均由确定性的像素算法产生。

原始牌面只依赖逻辑 `Card`，因此两个前端可以在各自协议缓存之前复用相同生成函数。修改牌面尺寸或固定坐标时，应继续运行像素和尺寸测试。

## 修改与验证

- 新页面或覆盖层应先加入共享语义/Canvas 层，再确认是否应抑制图像。
- 输入行为应加入前端中立按键，不应让 `App` 引用 Crossterm 或 Termwiz 类型。
- 新增图像能力时必须保持“明确确认才启用”和稳定文字降级。
- 不要重新引入 Ratatui、`ratatui-image` 或应用自有的 Kitty/iTerm2 协议路径。

```console
cargo fmt --check
cargo check --all-targets
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
```

实际协议显示、清理和终端恢复还需按照[手工测试清单](manual-test.md)验证。
