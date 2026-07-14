# UI、终端图像与牌面生成

本文说明 `src/ui.rs`、`src/graphics.rs` 和 `src/card_art.rs` 的职责、调用关系与主要实现方法，供维护界面布局、终端图像兼容性或牌面外观时参考。

## 总体结构

这三个模块按“布局—协议—像素”分层：

```text
main.rs 事件循环
  │ 每帧调用 ui::render(frame, app, graphics)
  ▼
ui.rs
  ├─ 读取 App 和公开游戏状态，决定页面、布局、文字样式与覆盖层
  ├─ 文字模式：直接用 Ratatui 组件绘制
  └─ 图像模式：取得拟合尺寸、决定居中 Rect，再为两个 PreviewSlot 请求协议
          ▼
      graphics.rs
        ├─ 探测终端并选择 iTerm2、Sixel、Kitty 或文字后端
        ├─ 根据字体单元与面板空间计算协议尺寸
        ├─ 缓存原始牌面和按最终 Rect 编码的协议
        └─ 缓存未命中时调用 generate_card_art
                ▼
            card_art.rs
              └─ 逐像素生成固定为 180 × 270 的 RGBA 牌面
```

分层的核心约束是：`ui` 不生成像素或判断具体终端类型，`graphics` 不决定页面布局，`card_art` 不访问终端和本地化文本。这样，纯文本界面在图像协议完全不可用时仍能独立工作。

## `src/ui.rs`：布局与交互视图

### 每帧渲染流程

公开入口 `render` 接收只读的 `App` 和可变的 `GraphicsRuntime`。一帧按以下顺序构造：

1. 检查终端尺寸。小于 `70 × 22` 时只绘制调整窗口提示，并调用 `graphics.suspend()` 清除位置相关的图像协议缓存。
2. 通过 `should_render_images` 决定本帧能否展示图像。要求已有对局、处于无普通弹窗的游戏页、没有待处理的万能牌选色、终端至少为 `70 × 26`，且实际后端支持图像。
3. 根据是否已有 `Game` 绘制设置页或牌桌主体。帮助、退出确认和结算页仍以牌桌为背景。
4. 最后绘制帮助、结算、退出确认等 `Screen` 覆盖层。
5. 万能牌选色由 `pending_wild` 表示，不是独立的 `Screen`，因此最后单独叠加。

弹窗出现时禁用图像，是因为部分终端协议的图片层可能覆盖普通字符；单纯用 Ratatui 的 `Clear` 擦除字符单元不一定能遮住协议图像。

### 设置页和游戏页

设置页由标题、选项列表和操作提示三个纵向区域组成。选项数组的顺序必须与 `app::adjust_setup` 使用的字段索引保持一致。图形设置显示的是用户选择与 `effective_backend` 共同决定的实际结果，例如 Beta 图像使用的协议或带原因的文字降级。

游戏页使用六个纵向区域：

| 区域 | 高度策略 | 内容 |
| --- | --- | --- |
| 标题 | 固定 3 行 | 牌组、当前玩家和方向 |
| 对手 | 固定 3 行 | 每个 AI 的手牌数量 |
| 牌桌 | 文字 5 行；图像 9 行 | 当前选择和弃牌堆顶 |
| 手牌 | 动态，最少 5 行 | 可滚动、可高亮的文字手牌 |
| 日志 | 使用剩余空间，最少 3 行 | 最新事件置顶 |
| 页脚 | 固定 3 行 | 合法操作提示或命令输入 |

`hand_height` 在完整显示手牌和给日志保留最低高度之间取平衡。图像牌桌比文字牌桌多占四行，所以计算手牌最大高度时也会扣除这四行。

### 手牌换行、滚动与上下导航

`hand_layout` 是手牌显示和键盘纵向导航共享的布局算法：

- 每张牌被转换为多个带样式的 `Span`，宽度由 Ratatui 计算，因此中文等宽字符的显示宽度也会被计入。
- 牌条目采用贪心换行；当前行放不下完整条目时，把条目放到下一行，不拆分一张牌。
- `HandCardPosition` 保存逻辑索引、行号和横向中心的两倍值。使用整数的“两倍中心”可精确表达半个字符，无需浮点数。
- `adjacent_hand_card` 在目标相邻行中选择中心位置最接近的牌；距离相同时选索引较小者，以保证结果稳定。
- `hand_scroll` 只滚动到刚好能看见选中行的位置，减少上下移动造成的跳动。

应用层处理上下键时传入终端宽度并调用同一个 `adjacent_hand_card`，所以导航结果与实际换行保持一致。

### 文字牌面和图像牌面

文字模式使用 `render_text_table`，始终显示当前生效颜色和弃牌堆顶。`styled_card` 为普通牌使用规则颜色、为万能牌使用洋红色，并将 `WildDrawSixteen` 拆成四色片段。选中牌的所有片段共享白色背景。

图像模式使用 `render_image_table` 将牌桌横向分为两个独立槽位：

- `PreviewSlot::Selected`：人类玩家当前选中的手牌；手牌为空或索引暂时越界时清除此槽位。
- `PreviewSlot::Discard`：弃牌堆顶，同时在标题显示生效颜色。

`render_card_preview` 先把面板内部尺寸交给 `GraphicsRuntime::fit_size`。这个调用按牌面、探测到的字体单元和 `Resize::Fit` 计算协议尺寸，但不执行 PNG/base64 编码。`centered_image_area` 再把拟合结果限制并居中为最终 `Rect`，最后将完整矩形交给 `GraphicsRuntime::protocol`。这样 UI 仍拥有布局决定权，graphics 仍拥有缩放与协议细节；若任一步失败，该位置立即绘制本地化的文字牌名，因此对局仍可继续。

### 覆盖层和主题

帮助、结果和退出确认共用 `render_overlay`。该函数先用 `Clear` 擦除目标字符区域，再画有边框的段落。`centered` 会把期望尺寸限制在父区域之内，避免小区域下的坐标下溢。

`carnival_block` 和 `carnival_border` 集中定义圆角、亮黄色边框及洋红色标题；新增常规面板应复用它们，以保持主题一致。

## `src/graphics.rs`：能力探测、降级与缓存

### 用户选择与实际后端

`GraphicsChoice` 是设置页的策略，目前有：

- `Text`：使用文字牌面，也是除 Windows Terminal 外的默认选择。
- `GraphicsBeta`：使用启动时探测出的可用协议，并在设置页标记为 `Graphics (Beta)`。

`GraphicsBackend` 表示实际结果：`Iterm2`、`Sixel`、`Kitty`，或携带 `FallbackReason` 的 `Text`。文字结果的原因分为设置页选择 Text（可能是环境默认值）、SSH、安全兼容性不足和运行时编码失败。

`effective_backend` 只在读取时应用用户选择，不覆盖启动探测结果。因此从 `Text` 切到 `GraphicsBeta` 后，可以恢复已发现的图像协议，无需再次探测。

### 环境识别与后端优先级

`TerminalEnvironment::detect` 读取环境变量：

- `SSH_CONNECTION`、`SSH_CLIENT` 或 `SSH_TTY` 非空表示 SSH；
- `WEZTERM_EXECUTABLE` 或包含 `WezTerm` 的 `TERM_PROGRAM` 表示 WezTerm；
- `WT_SESSION` 表示 Windows Terminal。

`default_graphics_choice` 在 SSH 或 WezTerm 中返回 `Text`；其他环境只有检测到 Windows Terminal 时返回 `GraphicsBeta`。因此原生 Windows Terminal 和 WSL 默认启用 Beta，而继承了 `WT_SESSION` 的 WezTerm、普通 Windows 终端、Linux 和 macOS 默认使用 Text。

`GraphicsRuntime::detect` 每次启动只执行一次。SSH 会跳过可能向终端发送查询序列的 `Picker::from_query_stdio`，直接安全降级。`resolve_backend` 的判断优先级如下：

1. SSH 始终使用文字模式。
2. WezTerm 固定使用 iTerm2 协议；构造运行时时会把探测器切换到 iTerm2，即使它最初只给出 Halfblocks。
3. Windows Terminal 只有明确探测到 Sixel 时才启用图像。
4. 其他本地终端接受探测到的 iTerm2、Sixel 或 Kitty。
5. Halfblocks 和没有探测结果都视为不受支持，使用本项目自己的彩色文字牌面。

这些规则有意比“探测到任何可显示内容就启用”更严格，因为 Halfblocks 的清理和覆盖行为与真正的图像协议不同。

### WezTerm 的位置与光标契约

只有本地、非 tmux 的 WezTerm iTerm2 数据会由应用增加定位。编码完成后，graphics 先核对协议变体、尺寸、上游透明区域清理前缀和 iTerm2 图片引导序列；全部符合预期才执行以下包装：

1. 在上游清屏序列之前写入最终图像矩形左上角的一基绝对 CSI 坐标。
2. 原样保留 `ratatui-image` 的清屏序列、iTerm2 图片参数和 PNG/base64 数据。
3. 数据结尾写入同一行的下一单元格绝对坐标，与 Ratatui 对承载字符串的首个单元格所做的光标记账一致。

tmux 会话保留上游 passthrough，普通 iTerm2、Sixel 和 Kitty 也不经过此包装。若 WezTerm 数据结构无法安全核对，运行期立即降级为 `Text(Encoding)`，释放 Picker 和两个协议缓存，且不会输出未经锚定的图片或在后续帧重复尝试。

### 两级缓存

`GraphicsRuntime` 使用两级缓存降低逐帧渲染成本：

```text
HashMap<Card, DynamicImage>                一级：固定尺寸 RGBA 原图
        │
        ├─ Selected: (Card, Rect) -> Protocol  二级：选中牌槽位协议
        └─ Discard:  (Card, Rect) -> Protocol  二级：弃牌槽位协议
```

一级缓存只以 `Card` 为键，因为原图不依赖终端区域。二级缓存的 `ProtocolKey` 同时包含 `Card` 和最终 Ratatui `Rect`：牌、原点或尺寸变化都会触发重新编码；完全不变的 50 ms 重绘则复用协议。原点必须属于键，因为 WezTerm 输出嵌入绝对坐标。两个槽位不能共享二级缓存，即使展示同一张牌也必须保留各自的位置与生命周期。

`protocol` 的处理过程是：

1. 若当前为文字后端或目标矩形为零，返回 `None`。
2. 比较目标键与对应槽位缓存；完全一致时直接返回上一帧的协议。
3. 缓存未命中时，从一级缓存取得或生成原图，再按最终矩形的尺寸编码。
4. 本地非 tmux WezTerm 在核对上游数据后加入最终矩形坐标；其他后端保持原数据。
5. 编码与必要的位置包装都成功后，原子地替换对应槽位缓存。
6. 任一步失败时，把整个运行期降级为 `Text(Encoding)`，丢弃 `Picker` 和两个协议缓存，防止每帧重复失败。

`suspend` 只清空两个位置相关的协议缓存，保留较昂贵且与布局无关的原图。它用于窗口过小、切换到弹窗或退出等场景。

## `src/card_art.rs`：程序化牌面光栅化

### 为什么不用外部图片和系统字体

牌面完全由代码生成，能够避免资源文件路径、打包遗漏、字体缺失以及不同语言环境下的字形差异。所有标签只使用内置 5×7 点阵字体，因此输出在各平台上确定一致，也便于用像素断言测试。

`generate_card_art` 总是返回 `180 × 270` 的 RGBA 图像，宽高比为 2:3。画布初始透明，使圆角外区域在终端缩放后不会出现不透明方框。

### 生成顺序

一张牌按覆盖顺序绘制：

1. 黑色外层圆角矩形；
2. 白色内框；
3. 普通牌的规则色主体，或万能牌的黑色底；
4. 普通牌的白色中央椭圆，或万能牌的四色象限和黑色中央椭圆；
5. 中央标签及两个角标；禁止牌改用圆环加斜杠的几何符号。

绘制顺序本身就是图层系统：后写入的像素覆盖先前颜色，不需要额外的合成对象。

### 基础几何算法

- `rounded_rect` 遍历外接矩形。位于中间水平或垂直条带的像素直接填充，角部像素通过到圆角圆心的平方距离判断。
- `ellipse` 使用椭圆隐式方程判断像素是否在内部。坐标放大两倍并取像素中心，避免浮点运算并保持对称。
- `wild_quadrants` 在主体内绘制红、黄、绿、蓝四个小圆角矩形，随后中央黑色椭圆覆盖它们的一部分。
- `draw_block_symbol` 在同一次扫描中判断像素是否位于圆环或斜杠上。

这些函数当前采用直接逐像素扫描。牌面尺寸固定且一级缓存保证每种逻辑牌通常只生成一次，所以实现优先考虑简单、确定和易测试，而不是引入复杂的矢量渲染依赖。

### 点阵文字

`glyph` 用 7 个字节表示 7 行，每行低 5 位表示从左到右的 5 个像素。`draw_text` 将每个置位像素扩展为 `scale × scale` 色块，字符之间保留一个点阵像素的间距。

`rank_label` 将牌面映射为短标签，例如 `R`、`+2`、`+16` 和 `W`。长标签使用较小缩放比例，中央文字由 `draw_centered_text` 根据 `text_width` 居中。未知字符会显示问号；禁止牌不经过标签路径，而由 `draw_block_symbol` 绘制。

## 修改指南

### 调整布局

修改游戏页固定区域高度时，应同时检查 `FIXED_GAME_HEIGHT` 和 `hand_height` 的预算。修改手牌条目格式时，必须继续让显示和 `adjacent_hand_card` 共享 `hand_layout`，否则上下导航会与实际换行不一致。

新增覆盖层时，应确认它出现期间是否需要在 `should_render_images` 中禁用协议图像，并在需要时调用 `suspend` 或 `clear_slot`，避免旧图像残留。

### 增加终端协议或兼容规则

协议选择集中在 `resolve_backend` 和 `from_picker`，WezTerm 数据核对与坐标包装集中在 graphics 的协议创建路径。增加规则时应同时考虑：环境优先级、探测失败行为、tmux passthrough、Ratatui 光标记账、设置页本地化说明，以及编码失败后的稳定降级。不要在 `ui.rs` 中根据环境变量选择协议。

### 修改牌面

改变 `CARD_WIDTH`、`CARD_HEIGHT` 或边距后，需要复核所有固定坐标、中心点和角标位置。新增牌面标签时，应确认 `glyph` 包含所需字符，并根据标签长度调整缩放策略。若牌面外观开始依赖语言，则应重新评估模块边界；当前缓存键只有 `Card`，不包含语言。

## 验证

相关自动化测试覆盖以下不变量：

- 小窗口、文字与图像布局不会越界；
- 手牌换行、滚动和上下导航一致；
- 默认选择遵守 SSH、WezTerm 和 Windows Terminal 优先级，且 WSL 中的 Windows Terminal 默认启用 Beta；
- 后端选择继续遵守 SSH、WezTerm 和 Windows Terminal 的协议约束；
- 两个预览的拟合与居中矩形始终位于各自面板内；
- 相同槽位、牌面和矩形能够复用协议，牌面、原点或尺寸变化时会重新编码；
- WezTerm 的绝对锚点、下一单元格恢复、槽位独立性和异常降级符合契约；
- 普通 iTerm2、Sixel、Kitty、tmux 和文字路径不被 WezTerm 包装改变；
- 每种牌面保持固定 RGBA 尺寸、四色映射正确，禁止牌的中心和角标存在。

修改后至少运行：

```console
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

终端协议的显示和清理仍依赖实际终端行为；涉及这部分的变更还应按照[手工测试清单](manual-test.md)在目标终端中验证。
