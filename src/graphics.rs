//! 终端图像能力探测与牌面预览缓存。
//!
//! 图像后端只负责选择终端协议、把语言无关的牌面位图编码为协议数据，以及
//! 在能力不足或编码失败时降级到文字模式；具体布局由 `ui` 模块决定。

use std::collections::HashMap;
use std::env;

use ratatui::layout::Size;
use ratatui_image::picker::{Picker, ProtocolType};
use ratatui_image::protocol::Protocol;
use ratatui_image::{Resize, errors::Errors};

use crate::card_art::generate_card_art;
use crate::core::Card;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
/// 用户对牌面显示方式的选择。
pub enum GraphicsChoice {
    /// 自动探测并使用受支持的终端图像协议。
    #[default]
    Auto,
    /// 无条件使用文字牌面。
    Text,
}

impl GraphicsChoice {
    /// 设置页按此顺序循环切换图形选项。
    pub const ALL: [Self; 2] = [Self::Auto, Self::Text];
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// 图像模式退回文字模式的原因。
pub enum FallbackReason {
    /// 用户在设置页主动选择了文字模式。
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
struct ProtocolKey {
    /// 当前槽位展示的牌；牌变化后协议数据必须重新编码。
    card: Card,
    /// UI 分配给预览的单元格尺寸；终端缩放后同一张牌也要重新编码。
    size: Size,
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
            art: HashMap::new(),
            selected: None,
            discard: None,
            #[cfg(test)]
            encodes: 0,
        }
    }

    #[cfg(test)]
    pub fn text_for_tests() -> Self {
        Self::from_picker(TerminalEnvironment::default(), None)
    }

    #[cfg(test)]
    pub fn with_protocol_for_tests(protocol_type: ProtocolType) -> Self {
        let mut picker = Picker::halfblocks();
        picker.set_protocol_type(protocol_type);
        Self::from_picker(TerminalEnvironment::default(), Some(picker))
    }

    /// 应用用户设置后返回本帧应展示的实际后端。
    pub fn effective_backend(&self, choice: GraphicsChoice) -> GraphicsBackend {
        // 手动选择只覆盖展示结果，不改写探测结果；用户切回 Auto 时仍可恢复
        // 启动时发现的图像后端。
        if choice == GraphicsChoice::Text {
            GraphicsBackend::Text(FallbackReason::Manual)
        } else {
            self.detected_backend
        }
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

    /// 获取指定牌面和区域对应的协议数据，必要时进行编码。
    ///
    /// 编码失败会永久把本次运行降级为文字模式，避免每一帧重复失败。
    pub fn protocol(&mut self, slot: PreviewSlot, card: Card, size: Size) -> Option<&Protocol> {
        // 零尺寸区域无法生成有效协议；提前返回也避免 Picker 内部报错。
        if !self.detected_backend.supports_images() || size.width == 0 || size.height == 0 {
            return None;
        }

        let key = ProtocolKey { card, size };
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

    fn encode(&mut self, slot: PreviewSlot, key: ProtocolKey) -> Result<(), Errors> {
        // 原始位图仅与牌面有关；协议缓存还与预览区域尺寸有关。
        let image = self
            .art
            .entry(key.card)
            .or_insert_with(|| generate_card_art(key.card))
            .clone();
        // Resize::Fit 保持牌面纵横比；协议返回的实际尺寸可能小于 UI 区域，
        // UI 会依据 protocol.size() 再做居中。
        let protocol = self
            .picker
            .as_ref()
            .expect("image backend retains picker")
            .new_protocol(image, key.size, Resize::Fit(None))?;
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn wezterm_iterm2_data_clears_the_previous_image_area() {
        use crate::core::{Color, Rank};

        let environment = TerminalEnvironment {
            is_wezterm: true,
            ..TerminalEnvironment::default()
        };
        let mut runtime = GraphicsRuntime::from_picker(environment, Some(Picker::halfblocks()));
        let protocol = runtime
            .protocol(
                PreviewSlot::Selected,
                Card::new(Color::Green, Rank::Number(2)),
                Size::new(12, 7),
            )
            .expect("WezTerm should retain its iTerm2 protocol");
        let Protocol::ITerm2(iterm2) = protocol else {
            panic!("WezTerm should encode an iTerm2 image");
        };

        let clear_row = format!("\x1b[{}X\x1b[1B", iterm2.size.width);
        let return_to_top = format!("\x1b[{}A\x1b]1337;", iterm2.size.height);
        assert!(iterm2.data.starts_with(&clear_row));
        assert!(iterm2.data.contains(&return_to_top));
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
    fn preview_slots_reuse_unchanged_encodings() {
        use crate::core::{Color, Rank};

        let mut runtime = GraphicsRuntime::with_protocol_for_tests(ProtocolType::Iterm2);
        let card = Card::new(Color::Blue, Rank::Number(7));
        let size = Size::new(12, 7);

        assert!(
            runtime
                .protocol(PreviewSlot::Selected, card, size)
                .is_some()
        );
        assert_eq!(runtime.encodes, 1);
        assert!(
            runtime
                .protocol(PreviewSlot::Selected, card, size)
                .is_some()
        );
        assert_eq!(runtime.encodes, 1);
        assert!(runtime.protocol(PreviewSlot::Discard, card, size).is_some());
        assert_eq!(runtime.encodes, 2);
        assert!(
            runtime
                .protocol(PreviewSlot::Selected, card, Size::new(13, 7))
                .is_some()
        );
        assert_eq!(runtime.encodes, 3);

        runtime.clear_slot(PreviewSlot::Selected);
        assert_eq!(runtime.cached_preview_count(), 1);
        assert!(
            runtime
                .protocol(PreviewSlot::Selected, card, Size::new(13, 7))
                .is_some()
        );
        assert_eq!(runtime.encodes, 4);
    }
}
