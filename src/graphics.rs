//! 终端图像能力探测与牌面预览缓存。
//!
//! 图像后端只负责选择终端协议、把语言无关的牌面位图编码为协议数据，以及
//! 在能力不足或编码失败时降级到文字模式；具体布局由 `ui` 模块决定。

use std::collections::HashMap;
use std::env;

use ratatui::layout::{Rect, Size};
use ratatui_image::Resize;
use ratatui_image::picker::{Picker, ProtocolType};
use ratatui_image::protocol::Protocol;

use crate::card_art::generate_card_art;
use crate::core::Card;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
/// 用户对牌面显示方式的选择。
pub enum GraphicsChoice {
    /// 无条件使用文字牌面。
    #[default]
    Text,
    /// 使用启动时探测到的受支持图像协议；该功能仍处于 Beta 阶段。
    GraphicsBeta,
}

impl GraphicsChoice {
    /// 设置页按此顺序循环切换图形选项。
    pub const ALL: [Self; 2] = [Self::Text, Self::GraphicsBeta];
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// 图像模式退回文字模式的原因。
pub enum FallbackReason {
    /// 设置页当前选择文字模式；该选项可能来自环境默认值或用户操作。
    Manual,
    /// SSH 环境中禁用图像协议，以免探测或转义序列影响远端会话。
    Ssh,
    /// 终端未提供本程序支持的图像协议。
    Unsupported,
    /// 运行时生成协议数据失败。
    Encoding,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// 当前实际采用的图形后端。
pub enum GraphicsBackend {
    /// iTerm2 内联图像协议；本项目也用它适配 WezTerm。
    Iterm2,
    /// Sixel 图像协议，主要用于受支持的 Windows Terminal。
    Sixel,
    /// Kitty 终端图像协议。
    Kitty,
    /// 不发送图像控制序列，改由 UI 绘制彩色文字牌面。
    Text(FallbackReason),
}

impl GraphicsBackend {
    /// 返回该后端是否能直接绘制图像牌面。
    pub fn supports_images(self) -> bool {
        !matches!(self, Self::Text(_))
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
/// 影响图像协议选择的终端环境特征。
pub struct TerminalEnvironment {
    /// 是否存在任一常见 SSH 会话环境变量。
    pub is_ssh: bool,
    /// 是否由 WezTerm 启动或运行在 WezTerm 中。
    pub is_wezterm: bool,
    /// 是否运行在 Windows Terminal 会话中。
    pub is_windows_terminal: bool,
}

impl TerminalEnvironment {
    /// 从常见环境变量识别 SSH、WezTerm 和 Windows Terminal。
    pub fn detect() -> Self {
        let present = |name: &str| env::var(name).is_ok_and(|value| !value.is_empty());
        let term_program = env::var("TERM_PROGRAM").unwrap_or_default();
        Self {
            is_ssh: ["SSH_CONNECTION", "SSH_CLIENT", "SSH_TTY"]
                .into_iter()
                .any(present),
            is_wezterm: present("WEZTERM_EXECUTABLE") || term_program.contains("WezTerm"),
            is_windows_terminal: present("WT_SESSION"),
        }
    }

    /// 根据终端环境决定设置页的初始图形选项。
    ///
    /// Windows Terminal（包括 WSL）是唯一默认启用 Beta 图像的环境；
    /// SSH 和 WezTerm 优先使用文字，避免继承的 `WT_SESSION` 造成误判。
    pub fn default_graphics_choice(self) -> GraphicsChoice {
        if !self.is_ssh && !self.is_wezterm && self.is_windows_terminal {
            GraphicsChoice::GraphicsBeta
        } else {
            GraphicsChoice::Text
        }
    }
}

/// 结合终端类型与探测到的协议，确定实际图形后端。
///
/// SSH 的安全降级优先级最高；WezTerm 和 Windows Terminal 只接受已验证的
/// 对应协议，其他本地终端则直接采用探测器返回的受支持协议。
pub fn resolve_backend(
    environment: TerminalEnvironment,
    detected: Option<ProtocolType>,
) -> GraphicsBackend {
    // 远程会话直接使用文字模式，不向远端终端发送能力查询或图像数据。
    if environment.is_ssh {
        return GraphicsBackend::Text(FallbackReason::Ssh);
    }

    // WezTerm 在本项目中固定走 iTerm2；Halfblocks 等探测结果不能视为
    // 可接受的图像后端，否则牌面会退化为字符拼图且清理行为不同。
    if environment.is_wezterm {
        return if detected == Some(ProtocolType::Iterm2) {
            GraphicsBackend::Iterm2
        } else {
            GraphicsBackend::Text(FallbackReason::Unsupported)
        };
    }

    // Windows Terminal 仅在明确探测到 Sixel 时启用图像，避免把其他终端的
    // 协议能力误套用到当前会话。
    if environment.is_windows_terminal {
        return if detected == Some(ProtocolType::Sixel) {
            GraphicsBackend::Sixel
        } else {
            GraphicsBackend::Text(FallbackReason::Unsupported)
        };
    }

    // 其他本地终端没有额外约束，直接采用探测器确认的原生图像协议。
    match detected {
        Some(ProtocolType::Iterm2) => GraphicsBackend::Iterm2,
        Some(ProtocolType::Sixel) => GraphicsBackend::Sixel,
        Some(ProtocolType::Kitty) => GraphicsBackend::Kitty,
        Some(ProtocolType::Halfblocks) | None => GraphicsBackend::Text(FallbackReason::Unsupported),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// UI 中相互独立的图像预览位置。
pub enum PreviewSlot {
    /// 当前选中的手牌。
    Selected,
    /// 弃牌堆顶牌。
    Discard,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// 判断某个槽位的协议缓存是否仍可复用的完整条件。
///
/// 协议数据同时包含图像内容、目标单元格尺寸和 WezTerm 锚点，所以仅比较
/// 牌面或尺寸并不充分。
struct ProtocolKey {
    /// 当前槽位展示的牌；牌变化后协议数据必须重新编码。
    card: Card,
    /// UI 居中后的最终矩形；原点或尺寸变化都必须重新生成定位数据。
    rect: Rect,
}

/// 单个预览槽已经编码完成的协议数据及其缓存键。
struct CachedProtocol {
    key: ProtocolKey,
    protocol: Protocol,
}

/// 保存终端图像后端及跨帧复用的牌面资源。
///
/// `art` 按牌缓存原始位图；每个预览槽再缓存与目标尺寸绑定的协议数据。
/// 因此只有牌或区域尺寸变化时才需要重新编码。
pub struct GraphicsRuntime {
    /// 由 `ratatui-image` 创建的协议编码器；文字模式下为 `None`。
    picker: Option<Picker>,
    /// 能力探测或编码降级后得到的实际后端。
    detected_backend: GraphicsBackend,
    /// 启动时识别到的终端环境；仅用于收窄 WezTerm 的位置包装分支。
    environment: TerminalEnvironment,
    /// 按逻辑牌面缓存固定尺寸的 RGBA 原图，避免每次布局变化都重新绘制。
    art: HashMap<Card, image::DynamicImage>,
    /// 当前选中手牌的协议缓存。
    selected: Option<CachedProtocol>,
    /// 弃牌堆顶牌的协议缓存。
    discard: Option<CachedProtocol>,
    #[cfg(test)]
    encodes: usize,
}

impl GraphicsRuntime {
    /// 探测当前终端并创建图形运行时。
    ///
    /// SSH 会跳过会向终端发送查询序列的协议探测。
    pub fn detect() -> Self {
        let environment = TerminalEnvironment::detect();
        if environment.is_ssh {
            return Self::from_picker(environment, None);
        }

        // 查询失败属于正常的能力不足，不阻止游戏启动，由 from_picker 统一
        // 转换成带原因的文字后端。
        match Picker::from_query_stdio() {
            Ok(picker) => Self::from_picker(environment, Some(picker)),
            Err(_) => Self::from_picker(environment, None),
        }
    }

    /// 将环境信息和可选探测器归一化为内部状态。
    ///
    /// 所有构造路径（真实探测与测试替身）都经过这里，从而共享相同的后端
    /// 选择规则以及“文字模式不保留编码器”的不变量。
    fn from_picker(environment: TerminalEnvironment, mut picker: Option<Picker>) -> Self {
        // WezTerm 的 iTerm2 实现能保留完整分辨率，优先于探测器的半块字符结果。
        if environment.is_wezterm
            && let Some(picker) = picker.as_mut()
        {
            picker.set_protocol_type(ProtocolType::Iterm2);
        }
        let detected = picker.as_ref().map(Picker::protocol_type);
        let detected_backend = resolve_backend(environment, detected);
        // 文字后端不保留 Picker，后续代码因而无法误用图像编码路径。
        let picker = detected_backend
            .supports_images()
            .then_some(picker)
            .flatten();
        Self {
            picker,
            detected_backend,
            environment,
            art: HashMap::new(),
            selected: None,
            discard: None,
            #[cfg(test)]
            encodes: 0,
        }
    }

    #[cfg(test)]
    /// 构造固定为文字后端的测试运行时，避免测试访问真实终端。
    pub fn text_for_tests() -> Self {
        Self::from_picker(TerminalEnvironment::default(), None)
    }

    #[cfg(test)]
    /// 构造使用指定协议的测试运行时，避免发送能力查询序列。
    pub fn with_protocol_for_tests(protocol_type: ProtocolType) -> Self {
        let mut picker = Picker::halfblocks();
        picker.set_protocol_type(protocol_type);
        Self::from_picker(TerminalEnvironment::default(), Some(picker))
    }

    /// 应用用户设置后返回本帧应展示的实际后端。
    pub fn effective_backend(&self, choice: GraphicsChoice) -> GraphicsBackend {
        // 文字选择只覆盖展示结果，不改写探测结果；用户切回 Graphics Beta
        // 时仍可恢复启动时发现的图像后端。
        if choice == GraphicsChoice::Text {
            GraphicsBackend::Text(FallbackReason::Manual)
        } else {
            self.detected_backend
        }
    }

    /// 返回当前环境推荐的启动选项，不受协议探测成功与否影响。
    pub fn default_choice(&self) -> GraphicsChoice {
        self.environment.default_graphics_choice()
    }

    /// 释放所有位置相关的协议数据，但保留可复用的原始牌面位图。
    pub fn suspend(&mut self) {
        self.selected = None;
        self.discard = None;
    }

    /// 释放指定预览位置的协议数据。
    pub fn clear_slot(&mut self, slot: PreviewSlot) {
        match slot {
            PreviewSlot::Selected => self.selected = None,
            PreviewSlot::Discard => self.discard = None,
        }
    }

    #[cfg(test)]
    pub fn cached_preview_count(&self) -> usize {
        usize::from(self.selected.is_some()) + usize::from(self.discard.is_some())
    }

    /// 根据牌面、字体单元尺寸和面板可用尺寸计算保持比例的协议尺寸。
    ///
    /// 此步骤只生成或复用原始牌面，不执行 PNG/base64 等协议编码。UI 因而
    /// 可以先把结果居中为最终矩形，再把完整位置交给 [`Self::protocol`]。
    pub fn fit_size(&mut self, card: Card, available: Size) -> Option<Size> {
        if !self.detected_backend.supports_images() || available.width == 0 || available.height == 0
        {
            return None;
        }

        let font_size = self
            .picker
            .as_ref()
            .expect("image backend retains picker")
            .font_size();
        let image = self
            .art
            .entry(card)
            .or_insert_with(|| generate_card_art(card));
        Some(Resize::Fit(None).size_for(image, font_size, available))
    }

    /// 获取指定牌面和最终矩形对应的协议数据，必要时进行编码。
    ///
    /// 编码失败会永久把本次运行降级为文字模式，避免每一帧重复失败。
    pub fn protocol(&mut self, slot: PreviewSlot, card: Card, rect: Rect) -> Option<&Protocol> {
        // 零尺寸区域无法生成有效协议；提前返回也避免 Picker 内部报错。
        if !self.detected_backend.supports_images() || rect.width == 0 || rect.height == 0 {
            return None;
        }

        let key = ProtocolKey { card, rect };
        // 两个槽位分别缓存。即使展示同一张牌，也不能共用一个槽位对象，
        // 因为左右面板可能具有不同尺寸和独立生命周期。
        let needs_encode = match slot {
            PreviewSlot::Selected => self
                .selected
                .as_ref()
                .is_none_or(|cached| cached.key != key),
            PreviewSlot::Discard => self.discard.as_ref().is_none_or(|cached| cached.key != key),
        };

        if needs_encode && self.encode(slot, key).is_err() {
            // 编码错误通常表示当前终端实际不接受已探测的协议。清空 Picker
            // 和所有协议缓存，使后续帧稳定返回文字牌面而不是不断重试。
            self.detected_backend = GraphicsBackend::Text(FallbackReason::Encoding);
            self.picker = None;
            self.suspend();
            return None;
        }

        // encode 成功后缓存必定存在；无需编码时则直接借用上一帧的协议。
        match slot {
            PreviewSlot::Selected => self.selected.as_ref().map(|cached| &cached.protocol),
            PreviewSlot::Discard => self.discard.as_ref().map(|cached| &cached.protocol),
        }
    }

    /// 为一个预览槽生成协议数据并原子地替换该槽位缓存。
    ///
    /// 只有完整编码成功后才写入缓存，因此错误不会留下与 `key` 不匹配的
    /// 半成品。调用方负责把错误转换为整个运行期的稳定文字降级。
    fn encode(&mut self, slot: PreviewSlot, key: ProtocolKey) -> Result<(), ProtocolFailure> {
        // 原始位图仅与牌面有关；协议缓存还与预览区域尺寸有关。
        // 克隆 DynamicImage 可结束对 art 的可变借用，随后才能再次借用 self
        // 中的 picker 和槽位字段；原图本身仍保留在一级缓存中供后续尺寸复用。
        let image = self
            .art
            .entry(key.card)
            .or_insert_with(|| generate_card_art(key.card))
            .clone();
        // UI 已用相同的 Resize::Fit 规则算出并居中了最终矩形；这里以该矩形
        // 的尺寸编码，并在需要时把其原点加入 WezTerm 数据。
        let mut protocol = self
            .picker
            .as_ref()
            .expect("image backend retains picker")
            .new_protocol(
                image,
                Size::new(key.rect.width, key.rect.height),
                Resize::Fit(None),
            )
            .map_err(|_| ProtocolFailure::Encoding)?;
        if self.environment.is_wezterm {
            position_wezterm_protocol(&mut protocol, key.rect)?;
        }
        let cached = Some(CachedProtocol { key, protocol });
        match slot {
            PreviewSlot::Selected => self.selected = cached,
            PreviewSlot::Discard => self.discard = cached,
        }
        #[cfg(test)]
        {
            self.encodes += 1;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// 创建可安全输出的协议数据时可能遇到的内部失败。
enum ProtocolFailure {
    /// `ratatui-image` 无法编码图像。
    Encoding,
    /// WezTerm 数据的变体、尺寸或转义序列不符合可安全包装的契约。
    UnsafeWeztermData,
}

/// 为本地 WezTerm 的 iTerm2 数据增加绝对位置；tmux 保留上游行为。
fn position_wezterm_protocol(protocol: &mut Protocol, rect: Rect) -> Result<(), ProtocolFailure> {
    let Protocol::ITerm2(iterm2) = protocol else {
        return Err(ProtocolFailure::UnsafeWeztermData);
    };
    if iterm2.is_tmux {
        return Ok(());
    }
    if iterm2.size != Size::new(rect.width, rect.height) {
        return Err(ProtocolFailure::UnsafeWeztermData);
    }

    let clear_prefix = iterm2_clear_prefix(iterm2.size);
    let Some(image_data) = iterm2.data.strip_prefix(&clear_prefix) else {
        return Err(ProtocolFailure::UnsafeWeztermData);
    };
    if !image_data.starts_with("\x1b]1337;File=inline=1;") || !image_data.ends_with('\x07') {
        return Err(ProtocolFailure::UnsafeWeztermData);
    }

    let anchor = absolute_cursor_position(rect.x, rect.y);
    let next_cell = absolute_cursor_position(rect.x.saturating_add(1), rect.y);
    iterm2.data = format!("{anchor}{}{next_cell}", iterm2.data);
    Ok(())
}

/// 复制 v11.0.6 在非 tmux iTerm2 数据开头使用的透明区域清理序列。
fn iterm2_clear_prefix(size: Size) -> String {
    if size.height == 1 {
        return format!("\x1b[{}X", size.width);
    }

    let mut prefix = String::new();
    for _ in 0..size.height {
        prefix.push_str(&format!("\x1b[{}X\x1b[1B", size.width));
    }
    prefix.push_str(&format!("\x1b[{}A", size.height));
    prefix
}

/// 把 Ratatui 的零基单元格坐标转换成 CSI 的一基绝对坐标。
fn absolute_cursor_position(x: u16, y: u16) -> String {
    format!("\x1b[{};{}H", y.saturating_add(1), x.saturating_add(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_local_windows_terminal_defaults_to_graphics_beta() {
        let windows_terminal = TerminalEnvironment {
            is_windows_terminal: true,
            ..TerminalEnvironment::default()
        };
        assert_eq!(
            windows_terminal.default_graphics_choice(),
            GraphicsChoice::GraphicsBeta
        );

        // WSL exposes the same terminal marker, so it follows the same policy
        // without depending on the Rust compile target.
        let wsl_in_windows_terminal = windows_terminal;
        assert_eq!(
            wsl_in_windows_terminal.default_graphics_choice(),
            GraphicsChoice::GraphicsBeta
        );

        assert_eq!(
            TerminalEnvironment::default().default_graphics_choice(),
            GraphicsChoice::Text
        );
        assert_eq!(
            TerminalEnvironment {
                is_wezterm: true,
                ..TerminalEnvironment::default()
            }
            .default_graphics_choice(),
            GraphicsChoice::Text
        );
    }

    #[test]
    fn ssh_and_wezterm_override_inherited_windows_terminal_default() {
        for environment in [
            TerminalEnvironment {
                is_ssh: true,
                is_wezterm: false,
                is_windows_terminal: true,
            },
            TerminalEnvironment {
                is_ssh: false,
                is_wezterm: true,
                is_windows_terminal: true,
            },
        ] {
            assert_eq!(environment.default_graphics_choice(), GraphicsChoice::Text);
        }
    }

    #[test]
    fn text_default_can_opt_into_a_detected_local_backend() {
        let mut picker = Picker::halfblocks();
        picker.set_protocol_type(ProtocolType::Kitty);
        let runtime = GraphicsRuntime::from_picker(TerminalEnvironment::default(), Some(picker));

        assert_eq!(runtime.default_choice(), GraphicsChoice::Text);
        assert_eq!(
            runtime.effective_backend(GraphicsChoice::Text),
            GraphicsBackend::Text(FallbackReason::Manual)
        );
        assert_eq!(
            runtime.effective_backend(GraphicsChoice::GraphicsBeta),
            GraphicsBackend::Kitty
        );
    }

    #[test]
    fn ssh_always_wins_and_skips_agent_only_false_positive() {
        let ssh = TerminalEnvironment {
            is_ssh: true,
            is_wezterm: true,
            is_windows_terminal: true,
        };
        assert_eq!(
            resolve_backend(ssh, Some(ProtocolType::Iterm2)),
            GraphicsBackend::Text(FallbackReason::Ssh)
        );

        let local = TerminalEnvironment::default();
        assert_eq!(
            resolve_backend(local, Some(ProtocolType::Kitty)),
            GraphicsBackend::Kitty
        );
    }

    #[test]
    fn wezterm_precedes_windows_terminal_and_requires_iterm2() {
        let environment = TerminalEnvironment {
            is_ssh: false,
            is_wezterm: true,
            is_windows_terminal: true,
        };
        assert_eq!(
            resolve_backend(environment, Some(ProtocolType::Iterm2)),
            GraphicsBackend::Iterm2
        );
        assert_eq!(
            resolve_backend(environment, Some(ProtocolType::Halfblocks)),
            GraphicsBackend::Text(FallbackReason::Unsupported)
        );
    }

    #[test]
    fn wezterm_forces_iterm2_when_detection_falls_back_to_halfblocks() {
        let environment = TerminalEnvironment {
            is_ssh: false,
            is_wezterm: true,
            is_windows_terminal: false,
        };
        let runtime = GraphicsRuntime::from_picker(environment, Some(Picker::halfblocks()));

        assert_eq!(runtime.detected_backend, GraphicsBackend::Iterm2);
        assert_eq!(
            runtime.picker.as_ref().map(Picker::protocol_type),
            Some(ProtocolType::Iterm2)
        );
    }

    #[test]
    fn fitting_does_not_encode_and_stays_within_available_size() {
        use crate::core::{Color, Rank};

        let mut runtime = GraphicsRuntime::with_protocol_for_tests(ProtocolType::Iterm2);
        let fitted = runtime
            .fit_size(Card::new(Color::Green, Rank::Number(2)), Size::new(12, 7))
            .expect("image backend should calculate a fitted size");

        assert!(fitted.width <= 12);
        assert!(fitted.height <= 7);
        assert!(fitted.width > 0);
        assert!(fitted.height > 0);
        assert_eq!(runtime.encodes, 0);
    }

    #[test]
    fn wezterm_iterm2_data_anchors_before_clearing_and_restores_next_cell() {
        use crate::core::{Color, Rank};

        let environment = TerminalEnvironment {
            is_wezterm: true,
            ..TerminalEnvironment::default()
        };
        let mut runtime = GraphicsRuntime::from_picker(environment, Some(Picker::halfblocks()));
        let card = Card::new(Color::Green, Rank::Number(2));
        let size = runtime
            .fit_size(card, Size::new(12, 7))
            .expect("WezTerm should calculate an image size");
        let rect = Rect::new(4, 7, size.width, size.height);
        let protocol = runtime
            .protocol(PreviewSlot::Selected, card, rect)
            .expect("WezTerm should retain its iTerm2 protocol");
        let Protocol::ITerm2(iterm2) = protocol else {
            panic!("WezTerm should encode an iTerm2 image");
        };

        let anchor = "\x1b[8;5H";
        let next_cell = "\x1b[8;6H";
        let clear_row = format!("\x1b[{}X\x1b[1B", iterm2.size.width);
        let return_to_top = format!("\x1b[{}A\x1b]1337;", iterm2.size.height);
        assert!(iterm2.data.starts_with(&format!("{anchor}{clear_row}")));
        assert!(iterm2.data.contains(&return_to_top));
        assert!(iterm2.data.ends_with(next_cell));
    }

    #[test]
    fn windows_terminal_requires_sixel() {
        let environment = TerminalEnvironment {
            is_windows_terminal: true,
            ..TerminalEnvironment::default()
        };
        assert_eq!(
            resolve_backend(environment, Some(ProtocolType::Sixel)),
            GraphicsBackend::Sixel
        );
        assert_eq!(
            resolve_backend(environment, Some(ProtocolType::Kitty)),
            GraphicsBackend::Text(FallbackReason::Unsupported)
        );
    }

    #[test]
    fn unsupported_and_halfblocks_become_text() {
        let environment = TerminalEnvironment::default();
        assert_eq!(
            resolve_backend(environment, None),
            GraphicsBackend::Text(FallbackReason::Unsupported)
        );
        assert_eq!(
            resolve_backend(environment, Some(ProtocolType::Halfblocks)),
            GraphicsBackend::Text(FallbackReason::Unsupported)
        );
    }

    #[test]
    fn preview_slots_are_position_aware_and_reuse_only_unchanged_rectangles() {
        use crate::core::{Color, Rank};

        let mut runtime = GraphicsRuntime::with_protocol_for_tests(ProtocolType::Iterm2);
        let card = Card::new(Color::Blue, Rank::Number(7));
        let rect = Rect::new(3, 4, 12, 7);

        assert!(
            runtime
                .protocol(PreviewSlot::Selected, card, rect)
                .is_some()
        );
        assert_eq!(runtime.encodes, 1);
        assert!(
            runtime
                .protocol(PreviewSlot::Selected, card, rect)
                .is_some()
        );
        assert_eq!(runtime.encodes, 1);
        assert!(runtime.protocol(PreviewSlot::Discard, card, rect).is_some());
        assert_eq!(runtime.encodes, 2);
        assert!(
            runtime
                .protocol(PreviewSlot::Selected, card, Rect::new(4, 4, 12, 7))
                .is_some()
        );
        assert_eq!(runtime.encodes, 3);
        assert!(
            runtime
                .protocol(PreviewSlot::Selected, card, Rect::new(4, 4, 13, 7))
                .is_some()
        );
        assert_eq!(runtime.encodes, 4);

        runtime.clear_slot(PreviewSlot::Selected);
        assert_eq!(runtime.cached_preview_count(), 1);
        assert!(
            runtime
                .protocol(PreviewSlot::Selected, card, Rect::new(4, 4, 13, 7))
                .is_some()
        );
        assert_eq!(runtime.encodes, 5);
    }

    #[test]
    fn wezterm_slots_embed_their_own_positions_even_for_the_same_card() {
        use crate::core::{Color, Rank};

        let environment = TerminalEnvironment {
            is_wezterm: true,
            ..TerminalEnvironment::default()
        };
        let mut runtime = GraphicsRuntime::from_picker(environment, Some(Picker::halfblocks()));
        let card = Card::new(Color::Yellow, Rank::Number(5));
        let size = runtime
            .fit_size(card, Size::new(12, 7))
            .expect("WezTerm should calculate an image size");
        let selected_rect = Rect::new(2, 3, size.width, size.height);
        let discard_rect = Rect::new(42, 3, size.width, size.height);

        let selected_data = match runtime
            .protocol(PreviewSlot::Selected, card, selected_rect)
            .expect("selected preview should encode")
        {
            Protocol::ITerm2(iterm2) => iterm2.data.clone(),
            _ => panic!("WezTerm should use iTerm2"),
        };
        let discard_data = match runtime
            .protocol(PreviewSlot::Discard, card, discard_rect)
            .expect("discard preview should encode")
        {
            Protocol::ITerm2(iterm2) => iterm2.data.clone(),
            _ => panic!("WezTerm should use iTerm2"),
        };

        assert!(selected_data.starts_with("\x1b[4;3H"));
        assert!(selected_data.ends_with("\x1b[4;4H"));
        assert!(discard_data.starts_with("\x1b[4;43H"));
        assert!(discard_data.ends_with("\x1b[4;44H"));
        assert_eq!(runtime.cached_preview_count(), 2);
        assert_eq!(runtime.encodes, 2);
    }

    #[test]
    fn non_wezterm_protocols_and_tmux_data_are_not_wrapped() {
        use crate::core::{Color, Rank};
        use ratatui_image::protocol::iterm2::Iterm2;

        let card = Card::new(Color::Red, Rank::Number(3));
        let rect = Rect::new(6, 8, 10, 7);
        let mut ordinary_iterm2 = GraphicsRuntime::with_protocol_for_tests(ProtocolType::Iterm2);
        let protocol = ordinary_iterm2
            .protocol(PreviewSlot::Selected, card, rect)
            .expect("ordinary iTerm2 should encode");
        let Protocol::ITerm2(iterm2) = protocol else {
            panic!("expected iTerm2 data");
        };
        assert!(iterm2.data.starts_with("\x1b["));
        assert!(!iterm2.data.starts_with("\x1b[9;7H"));

        let mut sixel = GraphicsRuntime::with_protocol_for_tests(ProtocolType::Sixel);
        assert!(matches!(
            sixel.protocol(PreviewSlot::Selected, card, rect),
            Some(Protocol::Sixel(_))
        ));
        let mut kitty = GraphicsRuntime::with_protocol_for_tests(ProtocolType::Kitty);
        assert!(matches!(
            kitty.protocol(PreviewSlot::Selected, card, rect),
            Some(Protocol::Kitty(_))
        ));

        let mut tmux = Protocol::ITerm2(Iterm2 {
            data: "upstream tmux data".to_owned(),
            size: Size::new(rect.width, rect.height),
            is_tmux: true,
        });
        position_wezterm_protocol(&mut tmux, rect).expect("tmux should bypass wrapping");
        let Protocol::ITerm2(tmux) = tmux else {
            unreachable!();
        };
        assert_eq!(tmux.data, "upstream tmux data");

        let mut text = GraphicsRuntime::text_for_tests();
        assert_eq!(text.fit_size(card, rect.as_size()), None);
        assert!(text.protocol(PreviewSlot::Selected, card, rect).is_none());
    }

    #[test]
    fn malformed_wezterm_data_is_rejected_without_modification() {
        use ratatui_image::protocol::iterm2::Iterm2;

        let rect = Rect::new(2, 3, 10, 7);
        let malformed = "\x1b]1337;File=inline=1;missing-clear-prefix\x07".to_owned();
        let mut protocol = Protocol::ITerm2(Iterm2 {
            data: malformed.clone(),
            size: rect.as_size(),
            is_tmux: false,
        });

        assert_eq!(
            position_wezterm_protocol(&mut protocol, rect),
            Err(ProtocolFailure::UnsafeWeztermData)
        );
        let Protocol::ITerm2(iterm2) = protocol else {
            unreachable!();
        };
        assert_eq!(iterm2.data, malformed);
    }

    #[test]
    fn unsafe_wezterm_protocol_stably_falls_back_and_clears_both_slots() {
        use crate::core::{Color, Rank};

        let environment = TerminalEnvironment {
            is_wezterm: true,
            ..TerminalEnvironment::default()
        };
        let mut runtime = GraphicsRuntime::from_picker(environment, Some(Picker::halfblocks()));
        let card = Card::new(Color::Blue, Rank::Number(1));
        let size = runtime
            .fit_size(card, Size::new(12, 7))
            .expect("WezTerm should calculate an image size");
        assert!(
            runtime
                .protocol(
                    PreviewSlot::Selected,
                    card,
                    Rect::new(2, 3, size.width, size.height),
                )
                .is_some()
        );
        assert_eq!(runtime.cached_preview_count(), 1);

        runtime
            .picker
            .as_mut()
            .expect("image backend retains picker")
            .set_protocol_type(ProtocolType::Sixel);
        assert!(
            runtime
                .protocol(
                    PreviewSlot::Discard,
                    card,
                    Rect::new(42, 3, size.width, size.height),
                )
                .is_none()
        );
        assert_eq!(
            runtime.detected_backend,
            GraphicsBackend::Text(FallbackReason::Encoding)
        );
        assert!(runtime.picker.is_none());
        assert_eq!(runtime.cached_preview_count(), 0);

        let encodes_after_failure = runtime.encodes;
        assert!(
            runtime
                .protocol(
                    PreviewSlot::Discard,
                    card,
                    Rect::new(42, 3, size.width, size.height),
                )
                .is_none()
        );
        assert_eq!(runtime.encodes, encodes_after_failure);
    }
}
