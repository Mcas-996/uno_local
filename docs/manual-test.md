# Local Terminal Manual Test

自动测试覆盖环境分派、Sixel 响应解析、自定义 Canvas、布局、缓存、Termwiz PNG 类型和游戏规则。实际终端的图像显示、缩放、清理与 raw mode 恢复仍按以下矩阵手工验证。

## 基础流程

1. 在至少 `70 × 26` 的终端运行 `cargo run -p uno`。
2. 分别以 1–4 个 AI、Easy/Normal/Hard 开始对局。
3. 用方向键选牌、Enter 出牌、`D` 抽牌，并验证万能牌选色。
4. 打开 `:`，验证 `play <index>`、`draw`、`pass`、`help`、`new` 和 `quit`。
5. 完成一局并从结果页开始新局。
6. 验证 `uno --help`、`uno -v` 和 `uno --version` 不进入 raw mode。

## 前端与终端矩阵

| 环境 | 预期前端 | 默认显示 | 必查项目 |
| --- | --- | --- | --- |
| 本地 WezTerm | Termwiz | Graphics | 两张 PNG 牌面位置正确；设置页可切 Text |
| 支持 Sixel 的本地终端（如新版 Windows Terminal） | Universal | Graphics | 启动查询后显示 Sixel；无查询乱码或滚屏 |
| 不支持/未确认 Sixel 的本地终端 | Universal | Text | 不输出图像转义；游戏完整可用 |
| SSH、WezTerm SSH domain | Universal | Text | 不发送能力查询或图像；状态与输入正常 |
| tmux | Universal | Text | 不发送 Sixel；窗格切换和退出后终端正常 |

仅设置 `SSH_AUTH_SOCK` 不应被判定为 SSH。若环境同时带有 WezTerm 和 SSH/tmux 标记，SSH/tmux 必须优先并强制 Text。

## 图像、布局和降级

1. 在 WezTerm 的 `70 × 26`、普通尺寸和 `159 × 41` 下确认两张牌都在各自面板内且近似居中。
2. 在已确认 Sixel 的终端重复相同尺寸测试，确认没有滚屏，光标和后续文字位置正常。
3. 切换选中牌、出牌、抽牌并开始新局，确认 Selected/Discard 不串位，不留下旧图。
4. 反复缩放并跨越 `70 × 26` 边界；小窗口只显示调整提示，恢复后图像在新位置出现且无残影。
5. 打开/关闭帮助、退出确认、万能牌选色和结果页；覆盖层期间不显示图像，关闭后图像正确恢复。
6. 在设置页切换 Text/Graphics，确认 Text 完全停止图像输出，Graphics 只在当前前端能力可用时生效。
7. 模拟 PNG/Sixel 编码失败，确认前端稳定留在文字模式且不会每帧重复失败。
8. 模拟 Termwiz 初始化或 I/O 失败，确认自动进入 Universal Text，并保留当前设置或对局状态。

## 终端生命周期

1. 正常退出、按 `Ctrl+C`，以及在调试构建中触发 panic；每次都确认主屏幕、可见光标、输入回显和 cooked mode 恢复。
2. 在图像可见时退出，确认 shell 提示符区域没有残留图像。
3. 重复快速缩放、打开覆盖层和退出，确认清理顺序稳定。

## 本地化与卸载

1. 在 `zh-CN`/其他 `zh*` locale 检查中文；在其他或不可用 locale 检查英文回退；设置页切换语言应立即刷新。
2. 对 cargo-dist 安装执行 `uno --uninstall`、`-y` 和 `--yes`；确认只删除匹配收据中的 `uno`、`uno-update` 和收据，不修改共享 bin 目录、shell 配置或 PATH。
3. 对源码构建、Cargo 安装、包管理器版本和不匹配的收据确认卸载被拒绝。
