//! Terminal graphics capability selection and cached card previews.

use std::collections::HashMap;
use std::env;

use ratatui::layout::{Rect, Size};
use ratatui_image::picker::{Picker, ProtocolType};
use ratatui_image::protocol::Protocol;
use ratatui_image::{Resize, errors::Errors};

use crate::card_art::generate_card_art;
use crate::core::Card;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum GraphicsChoice {
    #[default]
    Auto,
    Text,
}

impl GraphicsChoice {
    pub const ALL: [Self; 2] = [Self::Auto, Self::Text];
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FallbackReason {
    Manual,
    Ssh,
    Unsupported,
    Encoding,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GraphicsBackend {
    Iterm2,
    Sixel,
    Kitty,
    Halfblocks,
    Text(FallbackReason),
}

impl GraphicsBackend {
    pub fn supports_images(self) -> bool {
        !matches!(self, Self::Text(_))
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TerminalEnvironment {
    pub is_ssh: bool,
    pub is_wezterm: bool,
    pub is_windows_terminal: bool,
}

impl TerminalEnvironment {
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

pub fn resolve_backend(
    environment: TerminalEnvironment,
    detected: Option<ProtocolType>,
) -> GraphicsBackend {
    if environment.is_ssh {
        return GraphicsBackend::Text(FallbackReason::Ssh);
    }

    if environment.is_wezterm {
        return if detected == Some(ProtocolType::Halfblocks) {
            GraphicsBackend::Halfblocks
        } else {
            GraphicsBackend::Text(FallbackReason::Unsupported)
        };
    }

    if environment.is_windows_terminal {
        return if detected == Some(ProtocolType::Sixel) {
            GraphicsBackend::Sixel
        } else {
            GraphicsBackend::Text(FallbackReason::Unsupported)
        };
    }

    match detected {
        Some(ProtocolType::Iterm2) => GraphicsBackend::Iterm2,
        Some(ProtocolType::Sixel) => GraphicsBackend::Sixel,
        Some(ProtocolType::Kitty) => GraphicsBackend::Kitty,
        Some(ProtocolType::Halfblocks) | None => GraphicsBackend::Text(FallbackReason::Unsupported),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PreviewSlot {
    Selected,
    Discard,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ProtocolKey {
    card: Card,
    area: Rect,
}

struct CachedProtocol {
    key: ProtocolKey,
    protocol: Protocol,
}

pub struct GraphicsRuntime {
    picker: Option<Picker>,
    detected_backend: GraphicsBackend,
    is_wezterm: bool,
    art: HashMap<Card, image::DynamicImage>,
    selected: Option<CachedProtocol>,
    discard: Option<CachedProtocol>,
    #[cfg(test)]
    encodes: usize,
}

impl GraphicsRuntime {
    pub fn detect() -> Self {
        let environment = TerminalEnvironment::detect();
        if environment.is_ssh {
            return Self::from_picker(environment, None);
        }

        match Picker::from_query_stdio() {
            Ok(picker) => Self::from_picker(environment, Some(picker)),
            Err(_) => Self::from_picker(environment, None),
        }
    }

    fn from_picker(environment: TerminalEnvironment, mut picker: Option<Picker>) -> Self {
        // Halfblocks render directly into Ratatui cells, avoiding placement
        // bugs in older WezTerm external-image protocol implementations.
        if environment.is_wezterm
            && let Some(picker) = picker.as_mut()
        {
            picker.set_protocol_type(ProtocolType::Halfblocks);
        }
        let detected = picker.as_ref().map(Picker::protocol_type);
        let detected_backend = resolve_backend(environment, detected);
        let picker = detected_backend
            .supports_images()
            .then_some(picker)
            .flatten();
        Self {
            picker,
            detected_backend,
            is_wezterm: environment.is_wezterm,
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

    pub fn effective_backend(&self, choice: GraphicsChoice) -> GraphicsBackend {
        if choice == GraphicsChoice::Text {
            GraphicsBackend::Text(FallbackReason::Manual)
        } else {
            self.detected_backend
        }
    }

    pub fn suspend(&mut self) {
        self.selected = None;
        self.discard = None;
    }

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

    pub fn protocol(
        &mut self,
        slot: PreviewSlot,
        card: Card,
        area: Rect,
    ) -> Option<(&Protocol, Rect)> {
        if !self.detected_backend.supports_images() || area.width == 0 || area.height == 0 {
            return None;
        }

        let key = ProtocolKey { card, area };
        let needs_encode = match slot {
            PreviewSlot::Selected => self
                .selected
                .as_ref()
                .is_none_or(|cached| cached.key != key),
            PreviewSlot::Discard => self.discard.as_ref().is_none_or(|cached| cached.key != key),
        };

        if needs_encode && self.encode(slot, key).is_err() {
            self.detected_backend = GraphicsBackend::Text(FallbackReason::Encoding);
            self.picker = None;
            self.suspend();
            return None;
        }

        let cached = match slot {
            PreviewSlot::Selected => self.selected.as_ref(),
            PreviewSlot::Discard => self.discard.as_ref(),
        }?;
        let image_area = centered_image_area(cached.key.area, cached.protocol.size());
        Some((&cached.protocol, image_area))
    }

    fn encode(&mut self, slot: PreviewSlot, key: ProtocolKey) -> Result<(), Errors> {
        let image = self
            .art
            .entry(key.card)
            .or_insert_with(|| generate_card_art(key.card))
            .clone();
        let mut protocol = self
            .picker
            .as_ref()
            .expect("image backend retains picker")
            .new_protocol(image, key.area.as_size(), Resize::Fit(None))?;
        if self.is_wezterm {
            let image_area = centered_image_area(key.area, protocol.size());
            position_wezterm_iterm2(&mut protocol, image_area);
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

fn centered_image_area(area: Rect, image_size: Size) -> Rect {
    let width = image_size.width.min(area.width);
    let height = image_size.height.min(area.height);
    Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    )
}

fn position_wezterm_iterm2(protocol: &mut Protocol, area: Rect) {
    let Protocol::ITerm2(iterm2) = protocol else {
        return;
    };
    if iterm2.is_tmux {
        return;
    }

    // Make the OSC image independent of the backend's tracked cursor. The
    // Ratatui buffer still owns clipping and diffing, while this final CUP
    // forces WezTerm to start the image at the exact widget cell.
    if let Some(image_command) = iterm2.data.find("\x1b]1337;") {
        let absolute_position = format!("\x1b[{};{}H", area.y + 1, area.x + 1);
        iterm2.data.insert_str(image_command, &absolute_position);
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
            resolve_backend(environment, Some(ProtocolType::Sixel)),
            GraphicsBackend::Text(FallbackReason::Unsupported)
        );
    }

    #[test]
    fn wezterm_forces_iterm2_when_font_query_falls_back_to_halfblocks() {
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
        let area = Rect::new(2, 3, size.width, size.height);

        assert!(
            runtime
                .protocol(PreviewSlot::Selected, card, area)
                .is_some()
        );
        assert_eq!(runtime.encodes, 1);
        assert!(
            runtime
                .protocol(PreviewSlot::Selected, card, area)
                .is_some()
        );
        assert_eq!(runtime.encodes, 1);
        assert!(runtime.protocol(PreviewSlot::Discard, card, area).is_some());
        assert_eq!(runtime.encodes, 2);
        assert!(
            runtime
                .protocol(PreviewSlot::Selected, card, Rect::new(2, 3, 13, 7))
                .is_some()
        );
        assert_eq!(runtime.encodes, 3);

        runtime.clear_slot(PreviewSlot::Selected);
        assert_eq!(runtime.cached_preview_count(), 1);
        assert!(
            runtime
                .protocol(PreviewSlot::Selected, card, Rect::new(2, 3, 13, 7))
                .is_some()
        );
        assert_eq!(runtime.encodes, 4);
    }

    #[test]
    fn wezterm_iterm2_protocol_uses_absolute_image_position() {
        use crate::core::{Color, Rank};

        let mut picker = Picker::halfblocks();
        picker.set_protocol_type(ProtocolType::Iterm2);
        let mut runtime = GraphicsRuntime::from_picker(
            TerminalEnvironment {
                is_wezterm: true,
                ..TerminalEnvironment::default()
            },
            Some(picker),
        );
        let card = Card::new(Color::Green, Rank::Number(2));

        let (protocol, image_area) = runtime
            .protocol(PreviewSlot::Selected, card, Rect::new(10, 5, 20, 10))
            .expect("iTerm2 protocol");
        let Protocol::ITerm2(iterm2) = protocol else {
            panic!("expected iTerm2 protocol");
        };

        assert!(iterm2.data.contains("\x1b[1B"));
        assert!(iterm2.data.contains(&format!(
            "\x1b[{};{}H\x1b]1337;",
            image_area.y + 1,
            image_area.x + 1
        )));
    }

    #[test]
    fn fitted_images_are_centered_inside_their_panels() {
        assert_eq!(
            centered_image_area(Rect::new(10, 5, 20, 10), Size::new(6, 4)),
            Rect::new(17, 8, 6, 4)
        );
        assert_eq!(
            centered_image_area(Rect::new(10, 5, 20, 10), Size::new(30, 20)),
            Rect::new(10, 5, 20, 10)
        );
    }
}
