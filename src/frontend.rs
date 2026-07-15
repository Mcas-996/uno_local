//! Frontend-neutral terminal input and display policy.

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Viewport {
    pub columns: u16,
    pub rows: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyCode {
    Backspace,
    Enter,
    Left,
    Right,
    Up,
    Down,
    Esc,
    Char(char),
    Unknown,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct KeyModifiers(u8);

impl KeyModifiers {
    pub const NONE: Self = Self(0);
    #[cfg(test)]
    pub const SHIFT: Self = Self(1);
    pub const CONTROL: Self = Self(2);

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    pub const fn from_flags(shift: bool, control: bool, alt: bool) -> Self {
        Self((shift as u8) | ((control as u8) << 1) | ((alt as u8) << 2))
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum KeyEventKind {
    #[default]
    Press,
    Repeat,
    Release,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KeyEvent {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
    pub kind: KeyEventKind,
}

impl KeyEvent {
    pub const fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self {
            code,
            modifiers,
            kind: KeyEventKind::Press,
        }
    }

    pub const fn new_with_kind(code: KeyCode, modifiers: KeyModifiers, kind: KeyEventKind) -> Self {
        Self {
            code,
            modifiers,
            kind,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum GraphicsChoice {
    #[default]
    Text,
    GraphicsBeta,
}

impl GraphicsChoice {
    pub const ALL: [Self; 2] = [Self::Text, Self::GraphicsBeta];
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FallbackReason {
    Manual,
    Ssh,
    Tmux,
    Unsupported,
    Encoding,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GraphicsBackend {
    Sixel,
    Termwiz,
    Text(FallbackReason),
}

impl GraphicsBackend {
    pub const fn supports_images(self) -> bool {
        matches!(self, Self::Sixel | Self::Termwiz)
    }
}
