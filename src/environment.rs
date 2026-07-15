//! Deterministic runtime frontend selection.

use std::env;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TerminalEnvironment {
    pub is_ssh: bool,
    pub is_tmux: bool,
    pub is_wezterm: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrontendKind {
    UniversalText,
    Universal,
    Termwiz,
}

impl TerminalEnvironment {
    pub fn detect() -> Self {
        let present = |name: &str| env::var(name).is_ok_and(|value| !value.is_empty());
        let term = env::var("TERM").unwrap_or_default();
        let term_program = env::var("TERM_PROGRAM").unwrap_or_default();
        Self {
            is_ssh: ["SSH_CONNECTION", "SSH_CLIENT", "SSH_TTY"]
                .into_iter()
                .any(present),
            is_tmux: present("TMUX") || term.starts_with("tmux") || term_program == "tmux",
            is_wezterm: present("WEZTERM_EXECUTABLE") || term_program.contains("WezTerm"),
        }
    }

    pub const fn frontend(self) -> FrontendKind {
        if self.is_ssh || self.is_tmux {
            FrontendKind::UniversalText
        } else if self.is_wezterm {
            FrontendKind::Termwiz
        } else {
            FrontendKind::Universal
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssh_and_tmux_precede_wezterm() {
        for environment in [
            TerminalEnvironment {
                is_ssh: true,
                is_wezterm: true,
                ..TerminalEnvironment::default()
            },
            TerminalEnvironment {
                is_tmux: true,
                is_wezterm: true,
                ..TerminalEnvironment::default()
            },
        ] {
            assert_eq!(environment.frontend(), FrontendKind::UniversalText);
        }
        assert_eq!(
            TerminalEnvironment {
                is_wezterm: true,
                ..TerminalEnvironment::default()
            }
            .frontend(),
            FrontendKind::Termwiz
        );
        assert_eq!(
            TerminalEnvironment::default().frontend(),
            FrontendKind::Universal
        );
    }
}
